use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use std::collections::HashMap;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub admin: Option<String>,
    pub price_deviation_threshold: Decimal, // Default 5%
    pub whitelisted_updaters: Vec<String>, // Backend addresses allowed to update prices
    pub supported_tokens: Vec<String>,     // List of supported token IDs
}

#[cw_serde]
pub struct TokenPrice {
    pub usd: Decimal,
    pub last_updated: u64,
}

// Format serupa dengan yang Anda berikan
#[cw_serde]
pub struct PriceData {
    pub prices: HashMap<String, TokenPriceInfo>,
}

#[cw_serde]
pub struct TokenPriceInfo {
    pub usd: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    // Admin functions
    UpdateOwner { new_owner: String },
    UpdateAdmin { new_admin: String },
    SetDeviationThreshold { threshold: Decimal },
    AddWhitelistedUpdater { updater: String },
    RemoveWhitelistedUpdater { updater: String },
    AddSupportedToken { token_id: String },
    RemoveSupportedToken { token_id: String },

    // Price update functions from backend
    UpdatePrices { price_data: PriceData },
    UpdateSinglePrice { token_id: String, price_info: TokenPriceInfo },

    // Manual admin actions
    ApprovePrice { token_id: String, price: Decimal },
    RejectPrice { token_id: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(TokenPriceResponse)]
    TokenPrice { token_id: String },

    #[returns(AllPricesResponse)]
    AllPrices {},

    #[returns(PriceHistoryResponse)]
    PriceHistory {
        token_id: String,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(PendingUpdatesResponse)]
    PendingUpdates {},

    #[returns(SupportedTokensResponse)]
    SupportedTokens {},

    #[returns(WhitelistedUpdatersResponse)]
    WhitelistedUpdaters {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub admin: String,
    pub price_deviation_threshold: Decimal,
}

#[cw_serde]
pub struct TokenPriceResponse {
    pub token_id: String,
    pub price: Decimal,
    pub last_updated: u64,
}

#[cw_serde]
pub struct AllPricesResponse {
    pub prices: HashMap<String, TokenPrice>,
}

#[cw_serde]
pub struct PriceHistoryEntry {
    pub price: Decimal,
    pub timestamp: u64,
}

#[cw_serde]
pub struct PriceHistoryResponse {
    pub token_id: String,
    pub history: Vec<PriceHistoryEntry>,
}

#[cw_serde]
pub struct PendingUpdate {
    pub token_id: String,
    pub current_price: Decimal,
    pub new_price: Decimal,
    pub percent_change: Decimal,
    pub requested_at: u64,
}

#[cw_serde]
pub struct PendingUpdatesResponse {
    pub updates: Vec<PendingUpdate>,
}

#[cw_serde]
pub struct WhitelistedUpdatersResponse {
    pub updaters: Vec<String>,
}

#[cw_serde]
pub struct SupportedTokensResponse {
    pub tokens: Vec<String>,
}
