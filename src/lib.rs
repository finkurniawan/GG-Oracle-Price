use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

mod contract;
pub mod error;
pub mod helpers;
pub mod msg;
pub mod state;

// Export these for anyone using this contract as a dependency
pub use crate::error::ContractError;
pub use crate::msg::{
    AllPricesResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PendingUpdatesResponse,
    PriceHistoryResponse, QueryMsg, TokenPriceResponse, WhitelistedUpdatersResponse,
};

#[cfg(test)]
mod integration_tests;

// Entry points

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    contract::instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}
