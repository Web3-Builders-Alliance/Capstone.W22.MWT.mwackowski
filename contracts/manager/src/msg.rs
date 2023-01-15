use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Ledger;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    InstantiateSwap { code_id: u64 }, // should I use it within manager contract or maybe instantiate both indepenedntly?
    SwapTokens { 
        contract: String,
        initial_balance: Coin,
        etf_swap_routes: EtfSwapRoutes }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTokens {sender: String},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetTokensResponse {
    pub tokens_per_etf: Vec<Ledger>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EtfSwapRoutes {
    pub name: String,
    pub routes: Vec<Route>, // for now there will be only one item, I should consider nesting into another Vec <>
    pub ratios: Vec<u64>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Route {
    pub pool_id: String, 
    pub token_out_denom: String
}
