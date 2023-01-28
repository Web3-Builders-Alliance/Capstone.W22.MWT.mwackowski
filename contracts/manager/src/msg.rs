use cosmwasm_std::{Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    InstantiateSwap { 
        code_id: u64, 
        debug: bool,
    }, // should I use it within manager contract or maybe instantiate both indepenedntly?
    InstantiateCw20 { 
        code_id: u64, 
        etf_name: String, 
        etf_symbol: String,
    },
    SwapTokens { 
        initial_balance: Coin,
        etf_swap_routes: EtfSwapRoutes,
    },
    MintTokens {
        amount_to_mint: Uint128,
        mint_contract_address: String,
    },
    QueryMintTokens {
        sender: String,
        mint_contract: String
    },
    RedeemTokens {
        etf_name: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTokens {
        sender: String,
        etf_type: String
    },
    GetInitialSwap {
        sender: String
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetTokensResponse {
    pub tokens_per_etf: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetInitialSwapResponse {
    pub initial_swap: Coin
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EtfSwapRoutes {
    pub name: String,
    pub routes: Vec<Route>, // Route per each token that etf consists of
    pub ratios: Vec<Uint128>    // ratio per each token that etf consists of -> consider merging into Vec<(Route, u64)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Route {
    pub pool_id: u64, 
    pub token_out_denom: String
}
