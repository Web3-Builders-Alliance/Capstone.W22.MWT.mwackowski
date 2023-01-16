use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ledger {
    pub tokens: Vec<Coin>
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct Deposit {
//     pub etf_type: String,
//     pub tokens: Coin
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cache {
    pub sender: String,
    pub etf_name: String
}

pub const CONTRACTS: Map<(&str, &str), State> = Map::new("contracts");

//sender, type
pub const LEDGER: Map<(&str, &str), Vec<Coin>> = Map::new("ledger");
//sender, type
pub const DEPOSIT: Map<(&str, &str), Coin> = Map::new("usdcbalance");

pub const ETF_CACHE: Map<u64, Cache> = Map::new("cache");
