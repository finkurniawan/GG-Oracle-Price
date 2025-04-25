use cosmwasm_std::{Addr, Decimal, Deps, StdResult};
use std::ops::Div;

use crate::error::ContractError;
use crate::state::{CONFIG, TOKENS, WHITELISTED_UPDATERS};

pub fn is_owner(deps: Deps, address: &Addr) -> StdResult<bool> {
    let config = CONFIG.load(deps.storage)?;
    Ok(address == &config.owner)
}

pub fn is_admin(deps: Deps, address: &Addr) -> StdResult<bool> {
    let config = CONFIG.load(deps.storage)?;
    Ok(address == &config.admin || address == &config.owner)
}

pub fn is_whitelisted(deps: Deps, address: &Addr) -> StdResult<bool> {
    // Check if the address is whitelisted, owner, or admin
    if is_admin(deps, address)? {
        return Ok(true);
    }
    Ok(WHITELISTED_UPDATERS.may_load(deps.storage, address)?.unwrap_or(false))
}

pub fn is_token_supported(deps: Deps, token_id: &str) -> StdResult<bool> {
    match TOKENS.may_load(deps.storage, token_id)? {
        Some(token_info) => Ok(token_info.supported),
        None => Ok(false),
    }
}

pub fn calculate_price_deviation(old_price: Decimal, new_price: Decimal) -> StdResult<Decimal> {
    if old_price.is_zero() {
        return Ok(Decimal::percent(100));
    }

    let difference = if new_price > old_price {
        new_price - old_price
    } else {
        old_price - new_price
    };

    Ok(difference.div(old_price) * Decimal::percent(100))
}

pub fn validate_threshold(threshold: Decimal) -> Result<(), ContractError> {
    if threshold.is_zero() || threshold > Decimal::percent(100) {
        return Err(ContractError::InvalidThreshold(
            "Threshold must be between 0 and 100 percent".to_string(),
        ));
    }
    Ok(())
}
