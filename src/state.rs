use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};


use crate::msg::{PendingUpdate, TokenPrice};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub admin: Addr,
    pub price_deviation_threshold: Decimal,
}

#[cw_serde]
pub struct TokenInfo {
    pub supported: bool,
}

// Store the main config
pub const CONFIG: Item<Config> = Item::new("config");

// Store token prices
pub const PRICES: Map<&str, TokenPrice> = Map::new("prices");

// Store price history
pub const PRICE_HISTORY: Map<(&str, u64), Decimal> = Map::new("price_history");

// Store pending updates that need approval
pub const PENDING_UPDATES: Map<&str, PendingUpdate> = Map::new("pending_updates");

// Store supported tokens
pub const TOKENS: Map<&str, TokenInfo> = Map::new("tokens");

// Store whitelisted updaters
pub const WHITELISTED_UPDATERS: Map<&Addr, bool> = Map::new("whitelisted_updaters");
