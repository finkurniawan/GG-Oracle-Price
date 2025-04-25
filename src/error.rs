use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Token {0} not supported")]
    TokenNotSupported(String),

    #[error("Token {0} already supported")]
    TokenAlreadySupported(String),

    #[error("Updater {0} not whitelisted")]
    UpdaterNotWhitelisted(String),

    #[error("Updater {0} already whitelisted")]
    UpdaterAlreadyWhitelisted(String),

    #[error("Price deviation exceeds threshold")]
    PriceDeviationExceedsThreshold {},

    #[error("No pending update for token {0}")]
    NoPendingUpdate(String),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),
}
