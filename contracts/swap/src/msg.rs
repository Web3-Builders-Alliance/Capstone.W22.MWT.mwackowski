use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, CustomMsg, Uint128, Coin, Decimal, CustomQuery};

use crate::types::{Step, Swap, SwapAmountWithLimit, SwapAmount};

#[cw_serde]
pub struct InstantiateMsg {}


/// A number of Custom messages that can call into the Osmosis bindings
#[cw_serde]
pub enum ExecuteMsg {
    /// CreateDenom creates a new factory denom, of denomination:
    /// factory/{creating contract bech32 address}/{Subdenom}
    /// Subdenom can be of length at most 44 characters, in [0-9a-zA-Z./]
    /// Empty subdenoms are valid.
    /// The (creating contract address, subdenom) pair must be unique.
    /// The created denom's admin is the creating contract address,
    /// but this admin can be changed using the UpdateAdmin binding.
    // CreateDenom { subdenom: String },
    // /// ChangeAdmin changes the admin for a factory denom.
    // /// Can only be called by the current contract admin.
    // /// If the NewAdminAddress is empty, the denom will have no admin.
    // ChangeAdmin {
    //     denom: String,
    //     new_admin_address: String,
    // },
    // /// Contracts can mint native tokens for an existing factory denom
    // /// that they are the admin of.
    // MintTokens {
    //     denom: String,
    //     amount: Uint128,
    //     mint_to_address: String,
    // },
    // /// Contracts can burn native tokens for an existing factory denom
    // /// that they are the admin of.
    // /// Currently, the burn from address must be the admin contract.
    // BurnTokens {
    //     denom: String,
    //     amount: Uint128,
    //     burn_from_address: String,
    // },
    /// Swap over one or more pools
    /// Returns SwapResponse in the data field of the Response
    Swap {
        first: Swap,
        route: Vec<Step>,
        amount: SwapAmountWithLimit,
    },
}

impl ExecuteMsg {
    /// Basic helper to define a swap with one pool
    pub fn simple_swap(
        pool_id: u64,
        denom_in: impl Into<String>,
        denom_out: impl Into<String>,
        amount: SwapAmountWithLimit,
    ) -> Self {
        ExecuteMsg::Swap {
            first: Swap::new(pool_id, denom_in, denom_out),
            amount,
            route: vec![],
        }
    }
}

impl From<ExecuteMsg> for CosmosMsg<ExecuteMsg> {
    fn from(msg: ExecuteMsg) -> CosmosMsg<ExecuteMsg> {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for ExecuteMsg {}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // #[returns(FullDenomResponse)]
    // FullDenom {
    //     creator_addr: String,
    //     subdenom: String,
    // },
    /// For a given pool ID, list all tokens traded on it with current liquidity (spot).
    /// As well as the total number of LP shares and their denom
    #[returns(PoolStateResponse)]
    PoolState { id: u64 },
    // / Return current spot price swapping In for Out on given pool ID.
    // / Warning: this can easily be manipulated via sandwich attacks, do not use as price oracle.
    // / We will add TWAP for more robust price feed.
    // #[returns(SpotPriceResponse)]
    // SpotPrice { swap: Swap, with_swap_fee: bool },
    // / Return current spot price swapping In for Out on given pool ID.
    // / You can call `EstimateSwap { contract: env.contract.address, ... }` to set sender to the
    // / current contract.
    // / Warning: this can easily be manipulated via sandwich attacks, do not use as price oracle.
    // / We will add TWAP for more robust price feed.
    // #[returns(QueryMsg)]
    // EstimateSwap {
    //     sender: String,
    //     first: Swap,
    //     route: Vec<Step>,
    //     amount: SwapAmount,
    // },
    // Returns the Arithmetic TWAP given base asset and quote asset.
    // CONTRACT: start_time and end_time should be based on Unix time millisecond.
    // #[returns(ArithmeticTwapResponse)]
    // ArithmeticTwap {
    //     id: u64,
    //     quote_asset_denom: String,
    //     base_asset_denom: String,
    //     start_time: i64,
    //     end_time: i64,
    // },
    // Returns the accumulated historical TWAP of the given base asset and quote asset.
    // CONTRACT: start_time should be based on Unix time millisecond.
    // #[returns(ArithmeticTwapToNowResponse)]
    // ArithmeticTwapToNow {
    //     id: u64,
    //     quote_asset_denom: String,
    //     base_asset_denom: String,
    //     start_time: i64,
    // },
}


impl CustomQuery for QueryMsg {}

impl QueryMsg {
    // / Calculate spot price without swap fee
    // pub fn spot_price(pool_id: u64, denom_in: &str, denom_out: &str) -> Self {
    //     QueryMsg::SpotPrice {
    //         swap: Swap::new(pool_id, denom_in, denom_out),
    //         with_swap_fee: false,
    //     }
    // }

    // /// Basic helper to estimate price of a swap on one pool
    // pub fn estimate_swap(
    //     contract: impl Into<String>,
    //     pool_id: u64,
    //     denom_in: impl Into<String>,
    //     denom_out: impl Into<String>,
    //     amount: SwapAmount,
    // ) -> Self {
    //     QueryMsg::EstimateSwap {
    //         sender: contract.into(),
    //         first: Swap::new(pool_id, denom_in, denom_out),
    //         amount,
    //         route: vec![],
    //     }
    // }

    // pub fn arithmetic_twap(
    //     pool_id: u64,
    //     quote_asset_denom: impl Into<String>,
    //     base_asset_denom: impl Into<String>,
    //     start_time: i64,
    //     end_time: i64,
    // ) -> Self {
    //     QueryMsg::ArithmeticTwap {
    //         id: pool_id,
    //         quote_asset_denom: quote_asset_denom.into(),
    //         base_asset_denom: base_asset_denom.into(),
    //         start_time,
    //         end_time,
    //     }
    // }

    // pub fn arithmetic_twap_to_now(
    //     pool_id: u64,
    //     quote_asset_denom: impl Into<String>,
    //     base_asset_denom: impl Into<String>,
    //     start_time: i64,
    // ) -> Self {
    //     QueryMsg::ArithmeticTwapToNow {
    //         id: pool_id,
    //         quote_asset_denom: quote_asset_denom.into(),
    //         base_asset_denom: base_asset_denom.into(),
    //         start_time,
    //     }
    // }
}

#[cw_serde]
pub struct FullDenomResponse {
    pub denom: String,
}

#[cw_serde]
pub struct PoolStateResponse {
    /// The various assets that be swapped. Including current liquidity.
    pub assets: Vec<Coin>,
    /// The number of lp shares and their amount
    pub shares: Coin,
}

impl PoolStateResponse {
    pub fn has_denom(&self, denom: &str) -> bool {
        self.assets.iter().any(|c| c.denom == denom)
    }

    pub fn lp_denom(&self) -> &str {
        &self.shares.denom
    }

    /// If I hold num_shares of the lp_denom, how many assets does that equate to?
    pub fn shares_value(&self, num_shares: impl Into<Uint128>) -> Vec<Coin> {
        let num_shares = num_shares.into();
        self.assets
            .iter()
            .map(|c| Coin {
                denom: c.denom.clone(),
                amount: c.amount * num_shares / self.shares.amount,
            })
            .collect()
    }
}

#[cw_serde]
pub struct SpotPriceResponse {
    /// How many output we would get for 1 input
    pub price: Decimal,
}

#[cw_serde]
pub struct SwapResponse {
    // If you query with SwapAmount::Input, this is SwapAmount::Output
    // If you query with SwapAmount::Output, this is SwapAmount::Input
    pub amount: SwapAmount,
}

#[cw_serde]
pub struct ArithmeticTwapResponse {
    pub twap: Decimal,
}

#[cw_serde]
pub struct ArithmeticTwapToNowResponse {
    pub twap: Decimal,
}
