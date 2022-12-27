

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, StdError, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Coin, QueryRequest};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, PoolStateResponse};
use crate::state::{State, STATE, POOLS};
use crate::types::{Step, Swap, SwapAmountWithLimit};
use osmo_bindings::{OsmosisQuerier, OsmosisQuery};
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:osmosis-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use prost::DecodeError;

use osmosis_std::types::osmosis::gamm::v1beta1::GammQuerier;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Swap { first, route, amount} 
            => execute_simple_swap( 
                first.pool_id, 
                first.denom_in, 
                first.denom_out,
                amount,
                route),
    }
}

pub fn execute_simple_swap(
    pool_id: u64,
    denom_in: impl Into<String>,
    denom_out: impl Into<String>,
    amount: SwapAmountWithLimit,
    route: Vec<Step>,
) -> Result<Response, ContractError> {

    let msg = ExecuteMsg::Swap {
        first: Swap::new(pool_id, denom_in, denom_out),
        amount,
        route,
    };
    Ok(Response::new()
    .add_attribute("execute", "simple swap")
    .add_attribute("pool id", pool_id.to_string())
    // .add_message(msg)
    )
}

pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PoolState { id } => to_binary(&query_pool(&deps, id)?)
    }
}

fn get_pool_state(deps: &Deps<OsmosisQuery>, id: u64) -> StdResult<PoolStateResponse> {
    let pool_query = OsmosisQuery::PoolState { id: id };
    let query = QueryRequest::from(pool_query);
    Ok(deps.querier.query(&query)?)
}

fn query_pool(
    deps: &Deps<OsmosisQuery>,
    pool_id: u64,
) -> StdResult<osmosis_std::types::osmosis::gamm::v1beta1::Pool> {
    let res = GammQuerier::new(&deps.querier).pool(pool_id)?;
    res.pool
        .ok_or_else(|| StdError::NotFound {
            kind: "pool".to_string(),
        })?
        .try_into() // convert `Any` to `osmosis_std::types::osmosis::gamm::v1beta1::Pool`
        .map_err(|e: DecodeError| StdError::ParseErr {
            target_type: "osmosis_std::types::osmosis::gamm::v1beta1::Pool".to_string(),
            msg: e.to_string(),
        })
}