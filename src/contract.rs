use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::helpers::{
    calculate_price_deviation, is_admin, is_owner, is_token_supported, is_whitelisted,
    validate_threshold,
};
use crate::msg::{
    AllPricesResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PendingUpdate,
    PendingUpdatesResponse, PriceHistoryEntry, PriceHistoryResponse, QueryMsg,
    SupportedTokensResponse, TokenPrice, TokenPriceResponse, WhitelistedUpdatersResponse,
};
use crate::state::{
    Config, TokenInfo, CONFIG, PENDING_UPDATES, PRICE_HISTORY, PRICES, TOKENS, WHITELISTED_UPDATERS,
};

use cosmwasm_std::Decimal;
use std::collections::HashMap;

// Constants
const DEFAULT_HISTORY_LIMIT: u32 = 100;
const MAX_HISTORY_LIMIT: u32 = 1000;

// Instantiate the contract
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Get owner and admin from message or use message sender
    let owner = match msg.owner {
        Some(owner) => deps.api.addr_validate(&owner)?,
        None => info.sender.clone(),
    };

    let admin = match msg.admin {
        Some(admin) => deps.api.addr_validate(&admin)?,
        None => owner.clone(),
    };

    // Set default price deviation threshold if not provided or validate if provided
    let price_deviation_threshold = msg.price_deviation_threshold;
    validate_threshold(price_deviation_threshold)?;

    // Store the config
    let config = Config {
        owner,
        admin,
        price_deviation_threshold,
    };
    CONFIG.save(deps.storage, &config)?;

    // Store whitelisted updaters
    for updater in msg.whitelisted_updaters {
        let addr = deps.api.addr_validate(&updater)?;
        WHITELISTED_UPDATERS.save(deps.storage, &addr, &true)?;
    }

    // Store supported tokens
    for token_id in msg.supported_tokens {
        TOKENS.save(
            deps.storage,
            &token_id,
            &TokenInfo { supported: true },
        )?;
    }

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", config.owner)
        .add_attribute("admin", config.admin)
        .add_attribute("price_deviation_threshold", price_deviation_threshold.to_string()))
}

// Execute entry point
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Admin functions
        ExecuteMsg::UpdateOwner { new_owner } => execute_update_owner(deps, info, new_owner),
        ExecuteMsg::UpdateAdmin { new_admin } => execute_update_admin(deps, info, new_admin),
        ExecuteMsg::SetDeviationThreshold { threshold } => {
            execute_set_deviation_threshold(deps, info, threshold)
        }
        ExecuteMsg::AddWhitelistedUpdater { updater } => {
            execute_add_whitelisted_updater(deps, info, updater)
        }
        ExecuteMsg::RemoveWhitelistedUpdater { updater } => {
            execute_remove_whitelisted_updater(deps, info, updater)
        }
        ExecuteMsg::AddSupportedToken { token_id } => {
            execute_add_supported_token(deps, info, token_id)
        }
        ExecuteMsg::RemoveSupportedToken { token_id } => {
            execute_remove_supported_token(deps, info, token_id)
        }

        // Price update functions
        ExecuteMsg::UpdatePrices { price_data } => {
            execute_update_prices(deps, env, info, price_data)
        }
        ExecuteMsg::UpdateSinglePrice {
            token_id,
            price_info,
        } => execute_update_single_price(deps, env, info, token_id, price_info),

        // Manual admin actions
        ExecuteMsg::ApprovePrice { token_id, price } => {
            execute_approve_price(deps, env, info, token_id, price)
        }
        ExecuteMsg::RejectPrice { token_id } => execute_reject_price(deps, info, token_id),
    }
}

// Update owner - Only current owner can call this
fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    // Check if sender is the current owner
    if !is_owner(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate new owner address
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;

    // Update config
    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.owner = new_owner_addr.clone();
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_owner")
        .add_attribute("new_owner", new_owner))
}

// Update admin - Only owner can call this
fn execute_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    // Check if sender is the owner
    if !is_owner(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate new admin address
    let new_admin_addr = deps.api.addr_validate(&new_admin)?;

    // Update config
    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.admin = new_admin_addr.clone();
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("method", "update_admin")
        .add_attribute("new_admin", new_admin))
}

// Set price deviation threshold - Only admin can call this
fn execute_set_deviation_threshold(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Decimal,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate threshold
    validate_threshold(threshold)?;

    // Update config
    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.price_deviation_threshold = threshold;
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("method", "set_deviation_threshold")
        .add_attribute("threshold", threshold.to_string()))
}

// Add whitelisted updater - Only admin can call this
fn execute_add_whitelisted_updater(
    deps: DepsMut,
    info: MessageInfo,
    updater: String,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate updater address
    let updater_addr = deps.api.addr_validate(&updater)?;

    // Check if already whitelisted
    if WHITELISTED_UPDATERS
        .may_load(deps.storage, &updater_addr)?
        .unwrap_or(false)
    {
        return Err(ContractError::UpdaterAlreadyWhitelisted(updater));
    }

    // Add to whitelist
    WHITELISTED_UPDATERS.save(deps.storage, &updater_addr, &true)?;

    Ok(Response::new()
        .add_attribute("method", "add_whitelisted_updater")
        .add_attribute("updater", updater))
}

// Remove whitelisted updater - Only admin can call this
fn execute_remove_whitelisted_updater(
    deps: DepsMut,
    info: MessageInfo,
    updater: String,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate updater address
    let updater_addr = deps.api.addr_validate(&updater)?;

    // Check if whitelisted
    if !WHITELISTED_UPDATERS
        .may_load(deps.storage, &updater_addr)?
        .unwrap_or(false)
    {
        return Err(ContractError::UpdaterNotWhitelisted(updater));
    }

    // Remove from whitelist
    WHITELISTED_UPDATERS.remove(deps.storage, &updater_addr);

    Ok(Response::new()
        .add_attribute("method", "remove_whitelisted_updater")
        .add_attribute("updater", updater))
}

// Add supported token - Only admin can call this
fn execute_add_supported_token(
    deps: DepsMut,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Check if already supported
    if is_token_supported(deps.as_ref(), &token_id)? {
        return Err(ContractError::TokenAlreadySupported(token_id));
    }

    // Add to supported tokens
    TOKENS.save(
        deps.storage,
        &token_id,
        &TokenInfo { supported: true },
    )?;

    Ok(Response::new()
        .add_attribute("method", "add_supported_token")
        .add_attribute("token_id", token_id))
}

// Remove supported token - Only admin can call this
fn execute_remove_supported_token(
    deps: DepsMut,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Check if supported
    if !is_token_supported(deps.as_ref(), &token_id)? {
        return Err(ContractError::TokenNotSupported(token_id));
    }

    // Mark as not supported
    TOKENS.update(
        deps.storage,
        &token_id,
        |token_info| -> StdResult<_> {
            let mut token = token_info.unwrap();
            token.supported = false;
            Ok(token)
        },
    )?;

    // Optionally remove from active prices
    PRICES.remove(deps.storage, &token_id);

    // Optionally remove pending updates
    PENDING_UPDATES.remove(deps.storage, &token_id);

    Ok(Response::new()
        .add_attribute("method", "remove_supported_token")
        .add_attribute("token_id", token_id))
}

// Update prices - Only whitelisted updaters can call this
fn execute_update_prices(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    price_data: crate::msg::PriceData,
) -> Result<Response, ContractError> {
    // Check if sender is whitelisted
    if !is_whitelisted(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut response = Response::new().add_attribute("method", "update_prices");
    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    // Process each price update
    for (token_id, price_info) in price_data.prices {
        // Check if token is supported
        if !is_token_supported(deps.as_ref(), &token_id)? {
            return Err(ContractError::TokenNotSupported(token_id));
        }

        let current_price_opt = PRICES.may_load(deps.storage, &token_id)?;

        // If we have a current price, check for deviation
        if let Some(current_price) = current_price_opt {
            let deviation = calculate_price_deviation(current_price.usd, price_info.usd)?;

            // If price deviation exceeds threshold, add to pending updates
            if deviation > config.price_deviation_threshold {
                let pending_update = PendingUpdate {
                    token_id: token_id.clone(),
                    current_price: current_price.usd,
                    new_price: price_info.usd,
                    percent_change: deviation,
                    requested_at: current_time,
                };

                PENDING_UPDATES.save(deps.storage, &token_id, &pending_update)?;

                response = response.add_attribute("token_pending", &token_id);
                continue;
            }
        }

        // Update price and add to history
        let token_price = TokenPrice {
            usd: price_info.usd,
            last_updated: current_time,
        };

        PRICES.save(deps.storage, &token_id, &token_price)?;
        PRICE_HISTORY.save(deps.storage, (&token_id, current_time), &price_info.usd)?;

        response = response.add_attribute("token_updated", &token_id);
    }

    Ok(response)
}

// Update single price - Only whitelisted updaters can call this
fn execute_update_single_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    price_info: crate::msg::TokenPriceInfo,
) -> Result<Response, ContractError> {
    // Check if sender is whitelisted
    if !is_whitelisted(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Check if token is supported
    if !is_token_supported(deps.as_ref(), &token_id)? {
        return Err(ContractError::TokenNotSupported(token_id.clone()));
    }

    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let current_price_opt = PRICES.may_load(deps.storage, &token_id)?;

    // If we have a current price, check for deviation
    if let Some(current_price) = current_price_opt {
        let deviation = calculate_price_deviation(current_price.usd, price_info.usd)?;

        // If price deviation exceeds threshold, add to pending updates
        if deviation > config.price_deviation_threshold {
            let pending_update = PendingUpdate {
                token_id: token_id.clone(),
                current_price: current_price.usd,
                new_price: price_info.usd,
                percent_change: deviation,
                requested_at: current_time,
            };

            PENDING_UPDATES.save(deps.storage, &token_id, &pending_update)?;

            return Ok(Response::new()
                .add_attribute("method", "update_single_price")
                .add_attribute("token_id", token_id)
                .add_attribute("status", "pending_approval")
                .add_attribute("deviation", deviation.to_string()));
        }
    }

    // Update price and add to history
    let token_price = TokenPrice {
        usd: price_info.usd,
        last_updated: current_time,
    };

    PRICES.save(deps.storage, &token_id, &token_price)?;
    PRICE_HISTORY.save(deps.storage, (&token_id, current_time), &price_info.usd)?;

    Ok(Response::new()
        .add_attribute("method", "update_single_price")
        .add_attribute("token_id", token_id)
        .add_attribute("price", price_info.usd.to_string())
        .add_attribute("status", "updated"))
}

// Approve pending price - Only admin can call this
fn execute_approve_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    price: Decimal,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Check if token is supported
    if !is_token_supported(deps.as_ref(), &token_id)? {
        return Err(ContractError::TokenNotSupported(token_id.clone()));
    }

    // Check if there's a pending update
    let pending_update = PENDING_UPDATES.may_load(deps.storage, &token_id)?;
    if pending_update.is_none() {
        return Err(ContractError::NoPendingUpdate(token_id));
    }

    let current_time = env.block.time.seconds();

    // Update price and add to history
    let token_price = TokenPrice {
        usd: price,
        last_updated: current_time,
    };

    PRICES.save(deps.storage, &token_id, &token_price)?;
    PRICE_HISTORY.save(deps.storage, (&token_id, current_time), &price)?;

    // Remove pending update
    PENDING_UPDATES.remove(deps.storage, &token_id);

    Ok(Response::new()
        .add_attribute("method", "approve_price")
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string()))
}

// Reject pending price - Only admin can call this
fn execute_reject_price(
    deps: DepsMut,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // Check if sender is admin or owner
    if !is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Check if there's a pending update
    let pending_update = PENDING_UPDATES.may_load(deps.storage, &token_id)?;
    if pending_update.is_none() {
        return Err(ContractError::NoPendingUpdate(token_id));
    }

    // Remove pending update
    PENDING_UPDATES.remove(deps.storage, &token_id);

    Ok(Response::new()
        .add_attribute("method", "reject_price")
        .add_attribute("token_id", token_id))
}

// Query entry point
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::TokenPrice { token_id } => to_json_binary(&query_token_price(deps, token_id)?),
        QueryMsg::AllPrices {} => to_json_binary(&query_all_prices(deps)?),
        QueryMsg::PriceHistory {
            token_id,
            start_time,
            end_time,
            limit,
        } => to_json_binary(&query_price_history(deps, token_id, start_time, end_time, limit)?),
        QueryMsg::PendingUpdates {} => to_json_binary(&query_pending_updates(deps)?),
        QueryMsg::SupportedTokens {} => to_json_binary(&query_supported_tokens(deps)?),
        QueryMsg::WhitelistedUpdaters {} => to_json_binary(&query_whitelisted_updaters(deps)?),
    }
}

// Query config
fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        admin: config.admin.to_string(),
        price_deviation_threshold: config.price_deviation_threshold,
    })
}

// Query token price
fn query_token_price(deps: Deps, token_id: String) -> StdResult<TokenPriceResponse> {
    // Check if token is supported
    if !is_token_supported(deps, &token_id)? {
        return Err(StdError::generic_err(format!(
            "Token {} not supported",
            token_id
        )));
    }

    // Get token price
    let token_price = PRICES.may_load(deps.storage, &token_id)?;
    match token_price {
        Some(price) => Ok(TokenPriceResponse {
            token_id,
            price: price.usd,
            last_updated: price.last_updated,
        }),
        None => Err(StdError::generic_err(format!(
            "No price data for token {}",
            token_id
        ))),
    }
}

// Query all prices
fn query_all_prices(deps: Deps) -> StdResult<AllPricesResponse> {
    let prices: StdResult<HashMap<String, TokenPrice>> = PRICES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (k, v) = item?;
            Ok((k.to_string(), v))
        })
        .collect();

    Ok(AllPricesResponse { prices: prices? })
}

// Query price history
fn query_price_history(
    deps: Deps,
    token_id: String,
    start_time: Option<u64>,
    end_time: Option<u64>,
    limit: Option<u32>,
) -> StdResult<PriceHistoryResponse> {
    if !is_token_supported(deps, &token_id)? {
        return Err(StdError::generic_err(format!(
            "Token {} not supported",
            token_id
        )));
    }

    let limit = limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .min(MAX_HISTORY_LIMIT) as usize;

    let prefix_map = PRICE_HISTORY.prefix(token_id.clone().as_str());

    let lower_bound = start_time.map(Bound::<u64>::inclusive);
    let upper_bound = end_time.map(|t| Bound::<u64>::exclusive(t + 1));

    let history: StdResult<Vec<PriceHistoryEntry>> = prefix_map
        .range(deps.storage, lower_bound, upper_bound, cosmwasm_std::Order::Descending)
        .take(limit)
        .map(|item| {
            let (timestamp, price) = item?;
            Ok(PriceHistoryEntry { price, timestamp })
        })
        .collect();

    Ok(PriceHistoryResponse {
        token_id,
        history: history?,
    })
}

// Query pending updates
fn query_pending_updates(deps: Deps) -> StdResult<PendingUpdatesResponse> {
    let updates: StdResult<Vec<PendingUpdate>> = PENDING_UPDATES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (_, update) = item?;
            Ok(update)
        })
        .collect();

    Ok(PendingUpdatesResponse { updates: updates? })
}

// Query supported tokens
fn query_supported_tokens(deps: Deps) -> StdResult<SupportedTokensResponse> {
    let tokens: StdResult<Vec<String>> = TOKENS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            let (token_id, token_info) = match item {
                Ok((k, v)) => (k, v),
                Err(e) => return Some(Err(e)),
            };

            if token_info.supported {
                Some(Ok(token_id.to_string()))
            } else {
                None
            }
        })
        .collect();

    Ok(SupportedTokensResponse { tokens: tokens? })
}

// Query whitelisted updaters
fn query_whitelisted_updaters(deps: Deps) -> StdResult<WhitelistedUpdatersResponse> {
    let updaters: StdResult<Vec<String>> = WHITELISTED_UPDATERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (addr, _) = item?;
            Ok(addr.to_string())
        })
        .collect();

    Ok(WhitelistedUpdatersResponse {
        updaters: updaters?,
    })
}
