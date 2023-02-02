use std::fmt::Debug;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, to_vec, Binary, ContractResult, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SystemResult, CosmosMsg, BankMsg, Coin as CoinStd,
};
use cw2::set_contract_version;
use osmosis_std::shim::Timestamp;
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::epochs::v1beta1::{
    QueryEpochsInfoRequest, QueryEpochsInfoResponse,
};
use osmosis_std::types::osmosis::gamm::v1beta1::{
    QueryNumPoolsRequest, QueryNumPoolsResponse, QueryPoolParamsRequest, QueryPoolParamsResponse,
    QueryPoolRequest, QueryPoolResponse, SwapAmountInRoute, MsgSwapExactAmountIn,
};
use osmosis_std::types::osmosis::twap::v1beta1::{ArithmeticTwapToNowResponse, TwapQuerier, ArithmeticTwapResponse};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMapResponse, QueryMsg};
use crate::state::{DEBUG, MAP, OWNER};
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:osmosis-std-cosmwasm-test";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DEBUG.save(deps.storage, &msg.debug)?;
    OWNER.save(deps.storage, &info.sender)?;

    // With `Response` type, it is possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

/// Handling contract migration
/// To make a contract migratable, you need
/// - this entry_point implemented
/// - only contract admin can migrate, so admin has to be set at contract initiation time
/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        // Find matched incoming message variant and execute them with your custom logic.
        //
        // With `Response` type, it is possible to dispatch message to invoke external logic.
        // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    }
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExecuteSwapExactAmountIn { 
            routes, 
            token_in, 
            token_out_min_amount,
          } => execute_swap_exact_amount_in(env, routes, token_in, token_out_min_amount),
        ExecuteMsg::SendTokensBack {
            tokens,
            recipient
        } => execute_send_tokens_back(deps, env, info, tokens, recipient)
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryNumPools {} => {
            query_and_debug::<QueryNumPoolsResponse>(&deps, QueryNumPoolsRequest {})
        }
        QueryMsg::QueryEpochsInfo {} => {
            query_and_debug::<QueryEpochsInfoResponse>(&deps, QueryEpochsInfoRequest {})
        }
        QueryMsg::QueryPool { pool_id } => {
            query_and_debug::<QueryPoolResponse>(&deps, QueryPoolRequest { pool_id })
        }
        QueryMsg::QueryPoolParams { pool_id } => {
            query_and_debug::<QueryPoolParamsResponse>(&deps, QueryPoolParamsRequest { pool_id })
        }
        QueryMsg::QueryArithmeticTwap {pool_id, base_asset, quote_asset, start_time, end_time} 
        => query_arithmetic_twap (
            deps,
            _env,
            pool_id,
            base_asset,
            quote_asset,
            start_time,
            end_time
    ),
        QueryMsg::QueryMap { key } => to_binary(&QueryMapResponse {
            value: MAP.load(deps.storage, key)?,
        }),
    }
}


pub fn query_arithmetic_twap(
    deps: Deps,
    _env: Env,
    pool_id: u64,
    base_asset: String,
    quote_asset: String,
    start_time: Option<Timestamp>,
    end_time: Option<Timestamp>
) -> StdResult<Binary> {
    let res = TwapQuerier::new(&deps.querier).arithmetic_twap(
        pool_id, 
        base_asset,
        quote_asset,
        start_time,
        end_time
    )?;

    to_binary(&ArithmeticTwapResponse {
        arithmetic_twap: res.arithmetic_twap,
    })
}

fn query_and_debug<T>(
    deps: &Deps,
    q: impl Into<cosmwasm_std::QueryRequest<Empty>>,
) -> StdResult<Binary>
where
    T: Serialize + DeserializeOwned + Debug,
{
    to_binary(&{
        let request: cosmwasm_std::QueryRequest<Empty> = q.into();
        let raw = to_vec(&request).map_err(|serialize_err| {
            StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
        })?;
        let res: T = match deps.querier.raw_query(&raw) {
            SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
                "Querier system error: {}",
                system_err
            ))),
            SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(
                format!("Querier contract error: {}", contract_err),
            )),
            SystemResult::Ok(ContractResult::Ok(value)) => {
                if DEBUG.load(deps.storage)? {
                    let json_str = std::str::from_utf8(value.as_slice()).unwrap();
                    let json_str = jsonformat::format(json_str, jsonformat::Indentation::TwoSpace);

                    deps.api.debug("========================");
                    match request {
                        cosmwasm_std::QueryRequest::Stargate { path, data: _ } => {
                            deps.api
                                .debug(format!("Stargate Query :: {}", path).as_str());
                        }
                        request => {
                            deps.api.debug(format!("{:?}", request).as_str());
                        }
                    };

                    deps.api.debug("");
                    deps.api.debug("```");
                    deps.api.debug(&json_str);
                    deps.api.debug("```");
                    deps.api.debug("========================");
                }
                cosmwasm_std::from_binary(&value)
            }
        }?;
        res
    })
}

/// Handling submessage reply.
/// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    // With `Response` type, it is still possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages

    todo!()
}


pub fn execute_swap_exact_amount_in(
    env: Env, 
    routes: Vec<SwapAmountInRoute>,
    token_in: Option<Coin>,
    token_out_min_amount: String
) -> Result<Response, ContractError> {
    
    let sender = env.contract.address.into();
    let msg_create_swap: CosmosMsg = MsgSwapExactAmountIn {
        sender, 
        routes, 
        token_in,
        token_out_min_amount 
    }.into();
    
    Ok(Response::new()
        .add_message(msg_create_swap)
        .add_attribute("method", "execute_swap_exact_amount_in"))
}

pub fn execute_send_tokens_back(
    deps: DepsMut,
    _env: Env, 
    info: MessageInfo, 
    tokens: Vec<CoinStd>,
    recipient: String) -> Result<Response, ContractError> {
    if info.sender != OWNER.load(deps.storage)? {
        return Err(ContractError::Unauthorized{})
    }

    let bank_msg = BankMsg::Send { to_address: recipient.to_owned(), amount: tokens.to_owned() };

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attribute("method", "execute_send_tokens_back")
        .add_attribute("recipient", recipient)
        .add_attribute("amount_sent_back", tokens[0].amount))
}