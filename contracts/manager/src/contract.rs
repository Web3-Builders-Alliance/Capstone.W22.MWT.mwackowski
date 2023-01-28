#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg, WasmMsg, Reply, StdError, Empty, Coin, coin, Uint128, WasmQuery, BankMsg, attr, CosmosMsg,
};

use cw2::{set_contract_version, CONTRACT};
use cw20::MinterResponse;
use cw_utils::{parse_reply_instantiate_data};

use osmo_swap;
use cw20_base;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetTokensResponse, InstantiateMsg, QueryMsg, EtfSwapRoutes, GetInitialSwapResponse};
use crate::state::{INITIAL_DEPOSIT, INITIAL_DEPOSIT_CACHE, LEDGER, ETF_CACHE, Cache, SWAP_CONTRACT, INITIAL_SWAP, MINT_CONTRACTS, MINT_CACHE, MintCache, MANAGER_CONTRACT};
use osmosis_std::types::osmosis::gamm::v1beta1::{SwapAmountInRoute, QueryPoolResponse, Pool, GammQuerier};
use prost::DecodeError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:counter_manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAP_KEY:&str = "0";

const INSTANTIATE_SWAP_REPLY_ID:u64 = 1;
const INSTANTIATE_CW20_REPLY_ID:u64 = 2;
const EXECUTE_SWAP_REPLY_ID:u64 = 3;
const EXECUTE_SWAPS_REPLY_ID:u64 = 4;
const EXECUTE_MINT_TOKENS_REPLY_ID:u64 = 5;


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    MANAGER_CONTRACT.save(deps.storage, &env.contract.address)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InstantiateSwap { code_id, debug} 
            => instantiate_swap(deps, info, code_id, debug),
        ExecuteMsg::InstantiateCw20 { code_id, etf_name, etf_symbol} => instantiate_cw20(deps, info, env, code_id, etf_name, etf_symbol),
        ExecuteMsg::SwapTokens { initial_balance, etf_swap_routes} => try_execute_swap_exact_amount_in(
            deps, 
            env,
            info.sender.to_string(), 
            etf_swap_routes,
            initial_balance
        ),
        ExecuteMsg::MintTokens {recipient: _, amount_to_mint, mint_contract_address} => execute_mint_tokens(
            info.sender.to_string(), 
            amount_to_mint, 
            mint_contract_address
        )
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_SWAP_REPLY_ID => handle_instantiate_swap_reply(deps, msg),
        INSTANTIATE_CW20_REPLY_ID => handle_instantiate_cw20_reply(deps, msg),
        EXECUTE_SWAP_REPLY_ID => handle_swap_reply(deps, msg),
        EXECUTE_SWAPS_REPLY_ID => handle_swaps_reply(deps, msg),
        // EXECUTE_MINT_TOKENS_REPLY_ID => handle_mint_tokens_reply(deps, msg),
        id => Err(ContractError::Std(StdError::generic_err(format!("Unknown reply id: {}", id)))),
    }
}

// TODO
// should I save owner as well?
fn handle_instantiate_swap_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let res = parse_reply_instantiate_data(msg).unwrap();
    SWAP_CONTRACT.save(deps.storage,  &MAP_KEY, &res.contract_address)?;
    Ok(Response::default())
}

fn handle_instantiate_cw20_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let mint_cache = MINT_CACHE.load(deps.storage)?;
    let res = parse_reply_instantiate_data(msg).unwrap();
    MINT_CONTRACTS.save(deps.storage,  &mint_cache.etf_name, &res.contract_address)?;
    Ok(Response::default().add_attributes(vec![
        attr("mint_contract_name", mint_cache.etf_name),
        attr("mint_contract_address", res.contract_address)
    ])) 
}

fn handle_swap_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {

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
        return Err(ContractError::InvalidEntryParams{})
    }

    { 
        let ratios_sum: Uint128 = etf_swap_routes.clone().ratios.iter().sum();
        if ratios_sum != Uint128::from(100u128) {
            panic!("Sum of ratios needs to be equal to 100")
        }
    }
    
    // validate if routes are passed properly before moving into execution
    for route in etf_swap_routes.clone().routes.into_iter() {
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
            return Err(ContractError::PoolTokenNotFound { val: route.token_out_denom })
            // panic!("{:?} not found in allowed pool's token_out_denoms", route.token_out_denom) 
        }
    }


    // let depo_key = (cache.sender.as_str(), cache.etf_swap_routes.name.as_str());
    // let deposit = INITIAL_DEPOSIT.load(deps.storage, depo_key)?;
    let mut submessages: std::vec::Vec<SubMsg<Empty>> = vec![];
    let mut token_in_amnt_adder = Uint128::zero();
    let init_amnt = Uint128::from(initial_amount_swapped.parse::<u128>().unwrap());
    for (i, (route, ratio)) in etf_swap_routes.clone().routes.into_iter().zip(etf_swap_routes.ratios.into_iter()).enumerate() {

        // no need to swap for deposit denoms or denoms that have been received through initial swap
        if route.token_out_denom == initial_deposit_token_out_denom.to_owned() { // || route.token_out_denom == initial_deposit.denom {
            continue 
        } else {
            let token_in_amount: Uint128;
            if &i == &(etf_swap_routes.routes.len() - 1) {
                token_in_amount = init_amnt.checked_sub(token_in_amnt_adder).unwrap();
            } else {
                token_in_amount = init_amnt.checked_multiply_ratio(ratio, 100u128).unwrap();
                token_in_amnt_adder = token_in_amnt_adder.checked_add(token_in_amount).unwrap();
            }
            
            let execute_message = create_msg_execute_swap(
                swap_addr.to_string(), route.pool_id, route.token_out_denom.to_owned(), 
                coin(token_in_amount.into(), initial_deposit_token_out_denom.to_owned())
            );
            submessages.push(SubMsg::reply_on_success(execute_message, EXECUTE_SWAPS_REPLY_ID));
        }
    }

    INITIAL_SWAP.save(deps.storage, &cache.sender, &coin(initial_amount_swapped.parse::<u128>().unwrap(), 
                    initial_denom_swapped.to_owned()))?;


    let mint_contract_addr = MINT_CONTRACTS.load(deps.storage, etf_swap_routes.name.as_str())?;
    let msg_execute_mint_tokens = create_msg_execute_mint_tokens(
        cache.sender.to_owned(),
        initial_deposit.amount, 
        mint_contract_addr);

    return Ok(Response::default()
        .add_attributes(vec![
            attr("initial_swap_received_amount", initial_amount_swapped),
            attr("initial_swap_received_denom", initial_denom_swapped),
            attr("initial_swap_sender", &cache.sender),
            attr("execute_mint_tokens_amount", initial_deposit.amount),
            attr("execute_mint_tokens_recipient", cache.sender),
            ])
        .add_submessages(submessages)
        .add_message(msg_execute_mint_tokens)
        );            
        
        // .add_attribute("deposit_denom", deposit.denom)
        // .add_attribute("total_deposit_amount", deposit.amount)

 }

 fn handle_swaps_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {

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
    let mut found = false;
    if LEDGER.has(deps.storage, depo_key.clone()) {
        let curr_ledger = LEDGER.load(deps.storage, depo_key).unwrap();  
        new_ledger = curr_ledger.clone();
        for (i, c) in new_ledger.clone().into_iter().enumerate() {
            if &c.denom == &denom_swapped {
                let new_val = Uint128::from(amount_swapped.parse::<u128>().unwrap());
                new_ledger[i] = coin(c.amount.checked_add(new_val).unwrap().u128(), c.denom);
                found = true;
                break
            } 
        }
    }

    if !found {
        new_ledger.push(coin(amount_swapped.parse::<u128>().unwrap(), denom_swapped.to_owned()));
    }
    
    LEDGER.save(deps.storage, depo_key, &new_ledger)?;

    // let initial_depo_cache = INITIAL_DEPOSIT_CACHE.load(deps.storage)?;
    // let mint_contract_addr = MINT_CONTRACTS.load(deps.storage, cache.etf_swap_routes.name.as_str())?;
    // let msg_execute_mint_tokens = create_msg_execute_mint_tokens(
    //     cache.sender,
    //     initial_depo_cache.amount, 
    //     mint_contract_addr);
    
    Ok(Response::default()
        .add_attribute("swap_received_amount", amount_swapped)
        .add_attribute("swap_received_denom", denom_swapped)
 
        // .add_attribute("new_ledger_key", depo_key.0.to_owned())
        // .add_attribute("new_ledger_key2", depo_key.1.to_owned())
        // .add_attribute("new_ledger_val_amnt", new_ledger[0].to_owned().amount)
        // .add_attribute("new_ledger_val_denom", new_ledger[0].to_owned().denom)
    
    )

 }
   
 
 fn handle_mint_tokens_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError>  {
    unimplemented!()
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
    debug: bool
) -> Result<Response, ContractError> {
    let instantiate_message = WasmMsg::Instantiate {
        admin: None,
        code_id,
        msg: to_binary(&osmo_swap::msg::InstantiateMsg { debug: debug })?,
        funds: vec![],
        label: "osmo_swap".to_string(),
    };

    let submessage:SubMsg<Empty> = SubMsg::reply_on_success(instantiate_message, INSTANTIATE_SWAP_REPLY_ID);
    Ok(Response::new().add_submessage(submessage)
        .add_attribute("method", "instantiate_from_manager"))
}

pub fn instantiate_cw20(
    deps: DepsMut, 
    _info: MessageInfo, 
    env: Env, 
    code_id: u64,
    etf_name: String,
    etf_symbol: String
) -> Result<Response, ContractError> {

    let instantiate_mint_contract = WasmMsg::Instantiate {
        code_id: code_id,
        funds: vec![],
        admin: None,
        label: "lp_token".to_string(),
        msg: to_binary(&cw20_base::msg::InstantiateMsg {
            name: etf_name.to_owned(),
            symbol: etf_symbol.to_owned(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: env.contract.address.into(),
                cap: None,
            }),
            marketing: None,
        })?,
    };

    let reply_msg = SubMsg::reply_on_success(instantiate_mint_contract, INSTANTIATE_CW20_REPLY_ID);
    MINT_CACHE.save(deps.storage, &MintCache{etf_name: etf_name, etf_symbol: etf_symbol})?;

    Ok(Response::new().add_submessage(reply_msg))
}

pub fn try_execute_swap_exact_amount_in(
    deps: DepsMut, 
    _env: Env,
    sender: String,
    etf_swap_routes: EtfSwapRoutes,
    deposit: Coin,
    // tokens_to_swap: Vec<Coin>
) 
-> Result<Response, ContractError> { 

    if !MINT_CONTRACTS.has(deps.storage, &etf_swap_routes.name) {
        return Err(ContractError::MintContractNotFound{val: etf_swap_routes.name});
    }

    let swap_contract_addr = SWAP_CONTRACT.load(deps.storage, &MAP_KEY)?;
    let bank_msg = BankMsg::Send { to_address: swap_contract_addr.to_owned(), amount: vec![deposit.clone()] };

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


    Ok(Response::new()
        .add_message(bank_msg)
        .add_submessage(submessage))
}


fn execute_mint_tokens(        
    recipient: String,
    amount_to_mint: Uint128,
    mint_contract_address: String,
) -> Result<Response, ContractError> { 

    let execute_message = WasmMsg::Execute {
        contract_addr: mint_contract_address.to_string(),
        funds: vec![],
        msg: to_binary(&cw20_base::msg::ExecuteMsg::Mint {
            recipient: recipient.to_owned(),
            amount: amount_to_mint,
        }).unwrap()
    };
    Ok(Response::new()
        .add_attributes(vec![
            attr("execute_mint_tokens_amount", amount_to_mint),
            attr("execute_mint_tokens_recipient", &recipient),
        ])
        .add_message(execute_message))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTokens {sender, etf_type} => to_binary(&query_get_tokens(deps, sender, etf_type)?),
        QueryMsg::GetInitialSwap {sender} => to_binary(&query_get_initial_swap(deps, sender)?),
    }
}

fn query_get_tokens(deps: Deps, sender: String, etf_type: String) -> StdResult<GetTokensResponse> {
    let res = LEDGER.load(deps.storage, (&sender, &etf_type))?;
    Ok(GetTokensResponse { tokens_per_etf: res })
}

fn query_get_initial_swap(deps: Deps, sender: String) -> StdResult<GetInitialSwapResponse> {
    let res = INITIAL_SWAP.load(deps.storage, &sender)?;
    Ok(GetInitialSwapResponse {initial_swap: res})
}

fn get_token_balance(deps: Deps, contract: String, addr: String) -> StdResult<Uint128> {
    let resp: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        contract,
        &cw20_base::msg::QueryMsg::Balance {
            address: addr.to_string(),
        },
    )?;
    Ok(resp.balance)
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
        token_out_min_amount: "1".to_string()
          }).unwrap()
              // TODO add twap query in order to estimate token_out_min_amount

    };
    execute_message   
}


fn create_msg_execute_mint_tokens(        
    recipient: String,
    amount_to_mint: Uint128,
    mint_contract_address: String,
) -> cosmwasm_std::WasmMsg { 

    WasmMsg::Execute {
        contract_addr: mint_contract_address.to_string(),
        funds: vec![],
        msg: to_binary(&cw20_base::msg::ExecuteMsg::Mint {
            recipient: recipient.to_owned(),
            amount: amount_to_mint,
        }).unwrap()
    }
    // Ok(Response::new()
    //     .add_attributes(vec![
    //         attr("execute_mint_tokens_amount", amount_to_mint),
    //         attr("execute_mint_tokens_recipient", &recipient),
    //     ])
    //     .add_message(execute_message))

}

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
        pool_id = 2; //678;
    } else {
        return Err(ContractError::InvalidDepositDenom {val: deposit_denom.clone()});
    };
    Ok((deposit_token_out_denom, pool_id))
}
