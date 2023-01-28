use cosmwasm_std::{Coin, Addr};
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
pub struct EtfCache {
    pub sender: String,
    pub etf_swap_routes: EtfSwapRoutes
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintCache {
    pub etf_name: String,
    pub etf_symbol: String
}

// contracts
pub const MANAGER_CONTRACT: Item<Addr> = Item::new("manager_contract");
pub const SWAP_CONTRACT: Item<Addr> = Item::new("swap_contract");
pub const MINT_CONTRACTS: Map<&str, String> = Map::new("mint_contracts");

//sender, type
pub const LEDGER: Map<(&str, &str), Vec<Coin>> = Map::new("ledger");
//sender, type
pub const INITIAL_DEPOSIT: Map<(&str, &str), Coin> = Map::new("usdcbalance");
pub const INITIAL_SWAP: Map<&str, Coin> = Map::new("initial_swap");

pub const ETF_POOLS: Map<&str, u64> = Map::new("etf_pools");

pub const ETF_CACHE: Item<EtfCache> = Item::new("cache");
pub const MINT_CACHE: Item<MintCache> = Item::new("mint_cache");
pub const INITIAL_DEPOSIT_CACHE: Item<Coin> = Item::new("initial_deposit_cache");
pub const REVERT_SWAP_CACHE: Item<Coin> = Item::new("revert_swap_cache");

