use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Map, Item};

use crate::msg::EtfSwapRoutes;


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
    pub etf_swap_routes: EtfSwapRoutes
}

pub const SWAP_CONTRACT: Map<&str, String> = Map::new("swap_contract");

//sender, type
pub const LEDGER: Map<(&str, &str), Vec<Coin>> = Map::new("ledger");
//sender, type
pub const INITIAL_DEPOSIT: Map<(&str, &str), Coin> = Map::new("usdcbalance");

pub const ETF_CACHE: Map<u64, Cache> = Map::new("cache");
pub const INITIAL_DEPOSIT_CACHE: Item<Coin> = Item::new("initial_deposit_cache");
pub const INITIAL_SWAP: Map<&str, Coin> = Map::new("initial_swap");