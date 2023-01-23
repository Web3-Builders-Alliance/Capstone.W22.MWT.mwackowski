#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg, WasmMsg, Reply, StdError, Empty, Coin, coin, Uint128, WasmQuery,
};

use cw2::set_contract_version;

// use cw_multi_test::Executor;
use cw_utils::{parse_reply_instantiate_data};

use osmo_swap;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetTokensResponse, InstantiateMsg, QueryMsg, EtfSwapRoutes};
use crate::state::{INITIAL_DEPOSIT, INITIAL_DEPOSIT_CACHE, LEDGER, ETF_CACHE, Cache, SWAP_CONTRACT};
use osmosis_std::types::osmosis::gamm::v1beta1::{SwapAmountInRoute, QueryPoolResponse, Pool, GammQuerier};
use prost::DecodeError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:counter_manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAP_KEY:&str = "0";

const INSTANTIATE_REPLY_ID:u64 = 1;
const EXECUTE_SWAP_REPLY_ID:u64 = 2;
const EXECUTE_SWAPS_REPLY_ID:u64 = 3;



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InstantiateSwap { code_id } 
            => instantiate_swap(deps, info, code_id),
        ExecuteMsg::SwapTokens { initial_balance, etf_swap_routes} => try_execute_swap_exact_amount_in(
                deps, 
                info.sender.to_string(), 
                etf_swap_routes,
                initial_balance),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        INSTANTIATE_REPLY_ID => handle_instantiate_reply(deps, msg),
        EXECUTE_SWAP_REPLY_ID => handle_swap_reply(deps, msg),
        EXECUTE_SWAPS_REPLY_ID => handle_swaps_reply(deps, msg),
        id => Err(StdError::generic_err(format!("Unknown reply id: {}", id))),
    }
}

// TODO
// should I save owner as well?
fn handle_instantiate_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    let res = parse_reply_instantiate_data(msg).unwrap();
    SWAP_CONTRACT.save(deps.storage,  &MAP_KEY, &res.contract_address)?;
    Ok(Response::default())
}


fn handle_swap_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {

    // Filter the result so that it returns single event value
    let result: String = msg.result.clone()
        .unwrap()
        .events.iter()
        .filter(|event| event.ty == "token_swapped" && event.attributes[4].key == "tokens_out")
        .map(|p| p.attributes[4].value.clone())
        .collect();

    // The result will be in a format: 9999ibc/sometxhash, so we need to split te initial numbers and the rest 
    let (initial_amount_swapped, initial_denom_swapped) = split_result_no_regex(result.to_owned());


    let cache = ETF_CACHE.load(deps.storage, 0u64).unwrap();
    // ETF_CACHE.remove(deps.storage, 0u64);

    // let depo_key = (cache.sender.as_str(), cache.etf_swap_routes.name.as_str());
    // let deposit = INITIAL_DEPOSIT.load(deps.storage, depo_key)?;
    let swap_addr = SWAP_CONTRACT.load(deps.storage, &MAP_KEY).unwrap();
    let initial_deposit =  INITIAL_DEPOSIT_CACHE.load(deps.storage)?;
    let (initial_deposit_token_out_denom, _initial_pool_id) = get_initial_route_params(initial_deposit.to_owned().denom).unwrap();

    // validate length of provided routes and ratios vectors
    let etf_swap_routes = cache.etf_swap_routes;
    if &etf_swap_routes.ratios.len() != &etf_swap_routes.routes.len() {
        panic!("Ratios and routes params should have the same length")
    }

    { // validate ratios sum
        // let mut ratios_sum = Uint128::zero();
        // for ratio in etf_swap_routes.clone().ratios.into_iter() {
        //     ratios_sum = ratios_sum.checked_add(ratio).unwrap();
        //     }
        // if ratios_sum != Uint128::from(100u128) {
        //     panic!("Sum of ratios needs to be equal to 100")
        // }
        let ratios_sum: Uint128 = etf_swap_routes.clone().ratios.iter().sum();
        if ratios_sum != Uint128::from(100u128) {
            panic!("Sum of ratios needs to be equal to 100")
        }
    }
    
    
    for route in etf_swap_routes.clone().routes.into_iter() {
        // validate if routes are passed properly before moving into execution
        let res: QueryPoolResponse = deps.querier.query_wasm_smart(swap_addr.to_owned(),     
            &osmo_swap::msg::QueryMsg::QueryPool{ pool_id: route.pool_id }).unwrap();

        let pool: Pool = res.pool
            .ok_or_else(|| StdError::NotFound {
                kind: "pool".to_string(),
            })?
            .try_into() // convert `Any` to `osmosis_std::types::osmosis::gamm::v1beta1::Pool`
            .map_err(|e: DecodeError| StdError::ParseErr {
                target_type: "osmosis_std::types::osmosis::gamm::v1beta1::Pool".to_string(),
                msg: e.to_string(),
            })?;
        
        if !&pool.pool_assets.iter().any(|i| i.token.as_ref().unwrap().denom == route.token_out_denom) {
            panic!("{:?} not found in allowed pool's token_out_denoms", route.token_out_denom) 
            //how to return this as err?
            //Err(cosmwasm_std::StdError::not_found(deposit.denom.clone()) );
        }
    }


    // let depo_key = (cache.sender.as_str(), cache.etf_swap_routes.name.as_str());
    // let deposit = INITIAL_DEPOSIT.load(deps.storage, depo_key)?;
    let mut submessages: std::vec::Vec<SubMsg<Empty>> = vec![];
    for (_i, (route, ratio)) in etf_swap_routes.clone().routes.into_iter().zip(etf_swap_routes.ratios.into_iter()).enumerate() {

        // no need to swap for deposit denoms or denoms that have been received through initial swap
        if route.token_out_denom == initial_deposit_token_out_denom.to_owned() { // || route.token_out_denom == initial_deposit.denom {
            continue 
        } else {
            // u128 division floors number automatically which is exactly what we need
            let token_in_amount = (ratio * Uint128::from(initial_amount_swapped.parse::<u128>().unwrap())).u128() / 100;
            let execute_message = create_msg_execute_swap(
                swap_addr.to_string(), route.pool_id, route.token_out_denom.to_owned(), 
                coin(token_in_amount, initial_deposit_token_out_denom.to_owned())
            );
            submessages.push(SubMsg::reply_on_success(execute_message, EXECUTE_SWAPS_REPLY_ID));
        }
    }


    // if LEDGER.has(deps.storage, depo_key.clone()){
    //     let curr_ledger = LEDGER.load(deps.storage, depo_key).unwrap();  
    //     for coin in curr_ledger.into_iter() {

    //     }      
    //     new_ledger = coin(curr_ledger.amount.checked_add(deposit.amount).unwrap().u128(),
    //         curr_deposit.denom);
    // } else {
    //     new_deposit = deposit.clone();
    // }

    // LEDGER.save(deps.storage,  depo_key,  &new_deposit)?;

    //sender, type
    // LEDGER.save(deps.storage, &cache.sender, &Ledger{etf_type: cache.etf_name,
    //     tokens: vec![coin(amount_swapped.parse::<u128>().unwrap(), denom_swapped.clone())]
    //     }
    // )?;

    // HERE THE LOGIC FOR VERIFYING IF EVERYTHING WENT THROUGH PROPERLY
    // IF SO, SAVE TO STATE
    // IF NOT, REVERT 
    // where to keep this logic?

    return Ok(Response::default()
        .add_attribute("swap_received_amount", initial_amount_swapped)
        .add_attribute("swap_received_denom", initial_denom_swapped)
        .add_submessages(submessages)
        // .add_attribute("deposit_denom", deposit.denom)
        // .add_attribute("total_deposit_amount", deposit.amount)
    );
 }

 fn handle_swaps_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {

        // Filter the result so that it returns single event value
    let result: String = msg.result.clone()
        .unwrap()
        .events.iter()
        .filter(|event| event.ty == "token_swapped" && event.attributes[4].key == "tokens_out")
        .map(|p| p.attributes[4].value.clone())
        .collect();
    let (amount_swapped, denom_swapped) = split_result_no_regex(result.to_owned());

    let cache = ETF_CACHE.load(deps.storage, 0u64).unwrap();
    let depo_key = (cache.sender.as_str(), cache.etf_swap_routes.name.as_str());

    let mut new_ledger: Vec<Coin> = vec![];
    if LEDGER.has(deps.storage, depo_key.clone()){
        let curr_ledger = LEDGER.load(deps.storage, depo_key).unwrap();  
        for c in curr_ledger.into_iter() {
            if &c.denom == &denom_swapped {
                let new_val = Uint128::from(amount_swapped.parse::<u128>().unwrap());
                new_ledger.push(
                    coin(c.amount.checked_add(new_val)?.u128(), c.denom)
                )
            } else {
                new_ledger.push(c);
            }
        }      
    }
    LEDGER.save(deps.storage, depo_key, &new_ledger)?;

    Ok(Response::default()
        .add_attribute("swap_received_amount", amount_swapped)
        .add_attribute("swap_received_denom", denom_swapped))

 }
    
    // match CONTRACTS.load(deps.storage, (&MAP_KEY, &contract_address)) {
    //     Ok(mut state) => {
    //         state.count = state.count.checked_add(1).unwrap();
    //         CONTRACTS.save(deps.storage, (&MAP_KEY, &contract_address), &state)?;
    //     }
    //     Err(_) => {
    //      let state = State {
    //      address: contract_address.clone(),
    //      count: 99,
    //      };
    //         CONTRACTS.save(deps.storage, (&MAP_KEY, &contract_address), &state)?;
    //     }
    // }
    // CONTRACTS.update(deps.storage, (&MAP_KEY, &contract_address), |state| -> Result<_, ContractError> {
    //     let mut i_state = state.unwrap();
    //     i_state.count += 1;
    //     Ok(i_state)
    // }).unwrap();


// fn handle_reset_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
//     //println!("{:?}", msg.clone());
//     let contract_address = get_contract_address(&msg);
//     let count = get_reset_count(&msg);

//     CONTRACTS.update(deps.storage, (&MAP_KEY, &contract_address), |state| -> Result<_, ContractError> {
//         let mut i_state = state.unwrap();
//         i_state.count = count;
//         Ok(i_state)
//     }).unwrap();

//     Ok(Response::default())
// }

pub fn instantiate_swap(
    _deps: DepsMut,
    _info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    let instantiate_message = WasmMsg::Instantiate {
        admin: None,
        code_id,
        msg: to_binary(&osmo_swap::msg::InstantiateMsg { debug: true })?,
        funds: vec![],
        label: "osmo_swap".to_string(),
    };

    let submessage:SubMsg<Empty> = SubMsg::reply_on_success(instantiate_message, INSTANTIATE_REPLY_ID);
    Ok(Response::new().add_submessage(submessage))
}


pub fn try_execute_swap_exact_amount_in(
    deps: DepsMut, 
    sender: String,
    etf_swap_routes: EtfSwapRoutes,
    deposit: Coin,
    // tokens_to_swap: Vec<Coin>
) 
-> Result<Response, ContractError> {
    
    let swap_contract_addr = SWAP_CONTRACT.load(deps.storage, &MAP_KEY)?;

    // let's keep track of user's deposited USDC
    let depo_key = (sender.as_str(), etf_swap_routes.name.as_str());
    let new_deposit;

    
    if INITIAL_DEPOSIT.has(deps.storage, depo_key.clone()){
        let curr_deposit = INITIAL_DEPOSIT.load(deps.storage, depo_key).unwrap();        
        new_deposit = coin(curr_deposit.amount.checked_add(deposit.amount).unwrap().u128(),
            curr_deposit.denom);
    } else {
        new_deposit = deposit.clone();
    }

    INITIAL_DEPOSIT.save(deps.storage,  depo_key,  &new_deposit)?;
    INITIAL_DEPOSIT_CACHE.save(deps.storage, &coin(deposit.amount.into(), deposit.denom.to_string()))?;

    let (deposit_token_out_denom, pool_id) = get_initial_route_params(deposit.denom.to_string())?;
    
    if !vec!["uosmo", "usdc"].iter().any(|&i| i == deposit.denom) {
        return Err(ContractError::InvalidDepositDenom {val: deposit.denom.clone()});
    }

    // firstly, as most of the pools on Osmosis are based on OSMO, it is better to swap all USDC to 
    // OSMO as one transaction in the first place

    let execute_message = create_msg_execute_swap(
        swap_contract_addr.to_string(), pool_id, deposit_token_out_denom.to_owned(), deposit.clone()
    );

    let submessage:SubMsg<Empty> = SubMsg::reply_on_success(execute_message, EXECUTE_SWAP_REPLY_ID);
    ETF_CACHE.save(deps.storage, 0u64, &Cache { sender: sender.to_string(), etf_swap_routes: etf_swap_routes.clone()})?;

    // Ok(Response::new().add_submessage(submessage))
    Ok(Response::new()
        // .add_attribute("token_from_pool_asset", token_in)
        .add_submessage(submessage))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTokens {sender, etf_type} => to_binary(&query_get_tokens(deps, sender, etf_type)?),
    }
}

fn query_get_tokens(deps: Deps, sender: String, etf_type: String) -> StdResult<GetTokensResponse> {
    let res = LEDGER.load(deps.storage, (&sender, &etf_type))?;
    Ok(GetTokensResponse { tokens_per_etf: res })
}

fn create_msg_execute_swap(contract: String, 
    pool_id: u64, 
    token_out_denom: String, 
    token_in: Coin,
    ) -> cosmwasm_std::WasmMsg {
    let execute_message = WasmMsg::Execute {
        contract_addr: contract.clone(),
        funds: vec![],
        msg: to_binary(&osmo_swap::msg::ExecuteMsg::ExecuteSwapExactAmountIn {
            routes: vec![SwapAmountInRoute {
                pool_id: pool_id, 
                token_out_denom: token_out_denom
            }],
            token_in: Some(coin(
        token_in.amount.into(), 
        token_in.denom.clone()).into()
        ),
    // TODO add twap query in order to estimate token_out_min_amount
            token_out_min_amount: "1".to_string()
          }).unwrap()
    };
    execute_message   
}

// fn create_msg_query_pool(contract: String, 
//     pool_id: u64, 
//     ) -> cosmwasm_std::WasmQuery {
//     let query_message = WasmQuery::Smart {
//         contract_addr: contract.clone(),
//         msg: to_binary(&osmo_swap::msg::QueryMsg::QueryPool{ pool_id: pool_id }).unwrap()
//     };
//     query_message
// }

////////////////////////////
//helper functions for parsing reply data
// fn get_contract_address(msg: &Reply) -> String {
//     let result:String = msg.result.clone()
//         .unwrap()
//         .events.iter()
//         .filter(|event| event.ty == "execute" && event.attributes[0].key == "_contract_address").map(|p| p.attributes[0].value.clone()).collect();
//     // println!("get_contract_address {:?}", &result);
//     result
// }


// fn split_result(text: String) -> (String, String) {
//     let re = Regex::new(r"(\d*)").unwrap();
//     let mut found = "";
//     match re.captures(&text){
//         Some(caps) => found = caps.get(0).unwrap().as_str(),
//         None => println!("Denomparsingerror") //Err(ContractError::DenomParsingError{val: text})
//     }
//     let remaining = &String::from(text.clone())[found.len()..];
//     (found.to_string(), remaining.to_string())
// }

fn split_result_no_regex(coin_str: String) -> (String, String) {
    let position = coin_str.find(|c: char| !c.is_ascii_digit()).expect("did not find a split position");
    let (amount, denom) = coin_str.split_at(position);
    (amount.to_string(), denom.to_string())
}

fn get_initial_route_params(deposit_denom: String) -> Result<(String, u64), ContractError> {
    let deposit_token_out_denom: String;
    let pool_id: u64;
    if deposit_denom == "uosmo".to_string() {
        deposit_token_out_denom = "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string();
        pool_id = 1;
    } else if deposit_denom == "usdc".to_string() {
        deposit_token_out_denom = "uosmo".to_string();
        pool_id = 678;
    } else {
        return Err(ContractError::InvalidDepositDenom {val: deposit_denom.clone()});
    };
    Ok((deposit_token_out_denom, pool_id))
}
