use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Map, Item};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ledger {
    pub etf_type: String,
    // pub tokens: Vec<Coin>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Deposit {
    pub etf_type: String,
    pub tokens: Coin // change into Coin later!
}

pub const CONTRACTS: Map<(&str, &str), State> = Map::new("contracts");

pub const LEDGER: Map<&str, Ledger> = Map::new("ledger");

pub const DEPOSIT: Map<&str, Deposit> = Map::new("usdcbalance");

pub const ETF_CACHE: Item<String> = Item::new("cache");
