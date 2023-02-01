use cosmwasm_schema::{cw_serde, QueryResponses};

use osmosis_std::{shim::Timestamp, types::{cosmos::base::v1beta1::Coin, osmosis::gamm::v1beta1::Pool}};
pub use osmosis_std::types::osmosis::epochs::v1beta1::QueryEpochsInfoResponse;
use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;
pub use osmosis_std::types::osmosis::gamm::v1beta1::{QueryNumPoolsResponse, QueryPoolParamsResponse, QueryPoolResponse,};
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapResponse;
pub use osmosis_std::types::osmosis::twap::v1beta1::{ArithmeticTwapToNowRequest, ArithmeticTwapToNowResponse,};
use cosmwasm_std::{Coin as CoinStd,};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub debug: bool,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    ExecuteSwapExactAmountIn {
        routes: Vec<SwapAmountInRoute>,
        token_in: Option<Coin>,
        token_out_min_amount: String
    },
    SendTokensBack {
        tokens: Vec<CoinStd>,
        recipient: String
    }
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QueryNumPoolsResponse)]
    QueryNumPools {},

    #[returns(QueryEpochsInfoResponse)]
    QueryEpochsInfo {},

    #[returns(QueryPoolResponse)]
    QueryPool { pool_id: u64 },

    #[returns(QueryPoolParamsResponse)]
    QueryPoolParams { pool_id: u64 },

    #[returns(ArithmeticTwapResponse)]
    QueryArithmeticTwap{            
        pool_id: u64,
        base_asset: String,
        quote_asset: String,
        start_time: Option<Timestamp>,
        end_time: Option<Timestamp>},

    #[returns(QueryMapResponse)]
    QueryMap { key: String },
}
// #[cw_serde]
// #[derive(QueryResponses)]
// pub enum ResponseTypes{
//     #[returns(QueryEpochsInfoResponse)]
//     QueryEpochsInfoResponse,
//     #[returns(QueryPoolParamsResponse)]
//     QueryPoolParamsResponse,
//     #[returns(QueryPoolResponse)]
//     QueryPoolResponse
// }

#[cw_serde]
pub struct QueryMapResponse {
    pub value: String,
}

#[cw_serde]
pub struct OtherQueryPoolResponse{
    pub pool: Pool
}