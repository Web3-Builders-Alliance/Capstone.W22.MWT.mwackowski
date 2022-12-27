use osmo_bindings::PoolStateResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};
use cosmwasm_std::{
    coins, to_binary, Addr, Api, BankMsg, Binary, BlockInfo, Coin, CustomQuery, Decimal, Empty,
    Fraction, Isqrt, Querier, QuerierResult, StdError, StdResult, Storage, Uint128,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema, Debug)]
pub struct Pool {
    pub assets: Vec<Coin>,
    pub shares: Uint128,
    pub fee: Decimal,
}

impl Pool {
    // make an equal-weighted uniswap-like pool with 0.3% fees
    pub fn new(a: Coin, b: Coin) -> Self {
        let shares = (a.amount * b.amount).isqrt();
        Pool {
            assets: vec![a, b],
            shares,
            fee: Decimal::permille(3),
        }
    }
    pub fn into_response(self, pool_id: u64) -> PoolStateResponse {
        let denom = self.gamm_denom(pool_id);
        PoolStateResponse {
            assets: self.assets,
            shares: Coin {
                denom,
                amount: self.shares,
            },
        }
    }
    pub fn gamm_denom(&self, pool_id: u64) -> String {
        // see https://github.com/osmosis-labs/osmosis/blob/e13cddc698a121dce2f8919b2a0f6a743f4082d6/x/gamm/types/key.go#L52-L54
        format!("gamm/pool/{}", pool_id)
    }
}


pub const STATE: Item<State> = Item::new("state");
pub const POOLS: Map<u64, Pool> = Map::new("pools");
