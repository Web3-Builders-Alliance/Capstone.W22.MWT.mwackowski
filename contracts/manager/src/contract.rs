#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg, WasmMsg, Reply, StdError, Empty, Coin, coin, Uint128, BankMsg, attr, Addr, Event,
};

use cw2::{set_contract_version, CONTRACT};
use cw20::MinterResponse;
use cw_utils::{parse_reply_instantiate_data};

use osmo_swap;
use cw20_base;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetTokensResponse, InstantiateMsg, QueryMsg, EtfSwapRoutes, GetInitialSwapResponse};
use crate::state::{INITIAL_DEPOSIT, INITIAL_DEPOSIT_CACHE, LEDGER, ETF_CACHE, EtfCache, SWAP_CONTRACT, INITIAL_SWAP, MINT_CONTRACTS, MINT_CACHE, MintCache, MANAGER_CONTRACT, ETF_POOLS, REVERT_SWAP_CACHE, SwapCache, ETF_NAME_CACHE, EtfNameCache};
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
const EXECUTE_REVERT_SWAPS_REPLY_ID: u64 = 5;
const EXECUTE_CONJUNCTION_SWAPS_REPLY_ID: u64 = 6;
// const EXECUTE_MINT_TOKENS_REPLY_ID:u64 = 5;

const OSMO_ATOM_POOL_ID: u64 = 1;
const OSMO_USDC_POOL_ID: u64 = 2;

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
            => execute_instantiate_swap(deps, info, code_id, debug
        ),
        ExecuteMsg::InstantiateCw20 { code_id, etf_name, etf_symbol} 
            => execute_instantiate_cw20(deps, info, env, code_id, etf_name, etf_symbol
        ),
        ExecuteMsg::SwapTokens { initial_balance, etf_swap_routes} 
            => try_execute_swap_exact_amount_in(deps, env,info.sender.to_string(), etf_swap_routes,initial_balance
        ),
        ExecuteMsg::MintTokens {amount_to_mint, mint_contract_address} 
            => execute_mint_tokens(info.sender.to_string(), amount_to_mint, mint_contract_address
        ),
        ExecuteMsg::QueryMintTokens {sender, mint_contract} 
            => get_token_balance(deps, info, mint_contract, sender
        ),
        ExecuteMsg::RedeemTokens {etf_name} 
            => redeem_tokens(deps, info, env, etf_name
        ),
        ExecuteMsg::Callback { operands } => Ok(Response::default().add_submessages(operands)),

    }
}

pub fn execute_instantiate_swap(
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

pub fn execute_instantiate_cw20(
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

    // validate length of provided routes and ratios vectors
    if &etf_swap_routes.ratios.len() != &etf_swap_routes.routes.len() {
        return Err(ContractError::InvalidEntryParams{});
    }
    // validate sum of ratios
    let ratios_sum: Uint128 = etf_swap_routes.clone().ratios.iter().sum();
    if ratios_sum != Uint128::from(100u128) {
        return Err(ContractError::InvalidRatio{});
    }

    let swap_contract_addr = SWAP_CONTRACT.load(deps.storage)?;
    let bank_msg = BankMsg::Send { to_address: swap_contract_addr.to_string(), amount: vec![deposit.clone()] };

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

    let (deposit_token_out_denom, pool_id) = get_initial_route_params(&deposit.denom)?;
    
    if !vec!["uosmo", "usdc"].iter().any(|&i| i == deposit.denom) {
        return Err(ContractError::InvalidDepositDenom {val: deposit.denom.clone()});
    }

    // firstly, as most of the pools on Osmosis are based on OSMO, it is better to swap all USDC to 
    // OSMO as one transaction in the first place

    let execute_message = create_msg_execute_swap(
        swap_contract_addr.to_string(), pool_id, deposit_token_out_denom.to_owned(), deposit.clone()
    );
    let submessage:SubMsg<Empty> = SubMsg::reply_on_success(execute_message, EXECUTE_SWAP_REPLY_ID);

    ETF_CACHE.save(deps.storage, &EtfCache { sender: sender.to_string(), etf_swap_routes: etf_swap_routes.clone()})?;

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

fn redeem_tokens(deps: DepsMut, info: MessageInfo, env: Env, etf_name: String) -> Result<Response, ContractError> { 
    if !LEDGER.has(deps.storage, (&info.sender.as_str(), &etf_name.as_str())) {
        return Err(ContractError::Unauthorized{});
    } 
    
    let swap_addr = SWAP_CONTRACT.load(deps.storage)?;
    let ledger = LEDGER.load(deps.storage, (&info.sender.as_str(), &etf_name.as_str()))?;

    // find pool for reverting transactions
    let depo_coin = INITIAL_DEPOSIT.load(deps.storage, (&info.sender.to_string(), &etf_name))?;
    let (token_out_denom, _) = get_initial_route_params(&depo_coin.denom)?;

    let mut submessages: Vec<SubMsg<Empty>> = vec![];
    for c in ledger.clone().into_iter() {
        let pool_id = ETF_POOLS.load(deps.storage, &c.denom)?;
        let execute_message = create_msg_execute_swap(
            swap_addr.to_string(), pool_id, token_out_denom.to_string(), 
            c);
        // concuntion_messages.push(execute_message.clone());
        submessages.push(SubMsg::reply_on_success(execute_message, EXECUTE_REVERT_SWAPS_REPLY_ID));
    }

    let callback_message = WasmMsg::Execute {
        contract_addr: env.contract.address.into_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Callback { operands: submessages }).unwrap()
    };
    ETF_NAME_CACHE.save(deps.storage, &(EtfNameCache{
        sender: info.sender.to_string(),
        etf_name: etf_name
    }))?;
 
    Ok(Response::new()
    // .add_submessages(submessages)
    .add_submessage(SubMsg::reply_on_success(callback_message, EXECUTE_CONJUNCTION_SWAPS_REPLY_ID))
    // .add_event(Event::new("redeem")
    //     .add_attribute("etf_name", &etf_name)
    // )
)
}

// ----------------------------------- REPLY HANDLING
// ##############################################################################

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_SWAP_REPLY_ID => handle_instantiate_swap_reply(deps, msg),
        INSTANTIATE_CW20_REPLY_ID => handle_instantiate_cw20_reply(deps, msg),
        EXECUTE_SWAP_REPLY_ID => handle_swap_reply(deps, msg),
        EXECUTE_SWAPS_REPLY_ID => handle_swaps_reply(deps, msg),
        // EXECUTE_MINT_TOKENS_REPLY_ID => handle_mint_tokens_reply(deps, msg),
        EXECUTE_REVERT_SWAPS_REPLY_ID => handle_revert_swaps(deps, msg),
        EXECUTE_CONJUNCTION_SWAPS_REPLY_ID => handle_conjunction_swaps(deps, msg),
        id => Err(ContractError::Std(StdError::generic_err(format!("Unknown reply id: {}", id)))),
    }
}

// TODO
// should I save owner as well?
fn handle_instantiate_swap_reply(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let res = parse_reply_instantiate_data(msg).unwrap();
    let checked: Addr = deps.api.addr_validate(&res.contract_address)?;
    SWAP_CONTRACT.save(deps.storage,&checked)?;
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

    let result: String = parse_swap_reply(&msg);
    // Filter the result so that it returns single event value
    // let result: String = msg.result.clone()
    //     .unwrap()
    //     .events.iter()
    //     .filter(|event| event.ty == "token_swapped" && event.attributes[4].key == "tokens_out")
    //     .map(|p| p.attributes[4].value.clone())
    //     .collect();

    // The result will be in a format: 9999ibc/sometxhash, so we need to split te initial numbers and the rest 
    let (initial_amount_swapped, initial_denom_swapped) = split_result_no_regex(result.to_owned());

    let cache = ETF_CACHE.load(deps.storage).unwrap();

    let swap_addr = SWAP_CONTRACT.load(deps.storage).unwrap();
    let initial_deposit =  INITIAL_DEPOSIT_CACHE.load(deps.storage)?;
    let (initial_deposit_token_out_denom, _) = get_initial_route_params(&initial_deposit.denom).unwrap();
    let etf_swap_routes = cache.etf_swap_routes;
    
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
        
        if !&pool.pool_assets.iter().any(|i| i.token.to_owned().unwrap().denom == route.token_out_denom) {
            return Err(ContractError::PoolTokenNotFound { val: route.token_out_denom })
        }
    }

    let mut submessages: std::vec::Vec<SubMsg<Empty>> = vec![];
    let mut token_in_amnt_adder = Uint128::zero();
    let init_amnt = Uint128::from(initial_amount_swapped.parse::<u128>().unwrap());
    for (i, (route, ratio)) in etf_swap_routes.clone().routes.into_iter().zip(etf_swap_routes.ratios.into_iter()).enumerate() {

        // no need to swap for denoms that have been received through initial swap
        if route.token_out_denom == initial_deposit_token_out_denom.to_owned() {
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
            if !ETF_POOLS.has(deps.storage, &route.token_out_denom) {
                ETF_POOLS.save(deps.storage, &route.token_out_denom, &route.pool_id)?;
            }
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
            // attr("execute_mint_tokens_amount", initial_deposit.amount),
            // attr("execute_mint_tokens_recipient", cache.sender),
            ])
        .add_submessages(submessages)
        .add_message(msg_execute_mint_tokens)
        );            

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

    let cache = ETF_CACHE.load(deps.storage).unwrap();
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

 fn handle_revert_swaps(deps: DepsMut, msg: Reply) -> Result<Response, ContractError> {
    let (amount_swapped_string, denom_swapped) = split_result_no_regex(parse_swap_reply(&msg));
    let amount_swapped = amount_swapped_string.parse::<u128>().unwrap();
    let mut updated: Uint128 = Uint128::zero();
    // let swapping_finished: String = msg.result.clone()
    //     .unwrap()
    //     .events.iter()
    //     .filter(|event| event.ty == "wasm-redeem" && event.attributes[4].key == "tokens_out")
    //     .map(|p| p.attributes[4].value.clone())
    //     .collect();
    if  REVERT_SWAP_CACHE.may_load(deps.storage).unwrap() != None {
        REVERT_SWAP_CACHE.update(deps.storage, |mut rev_coin| -> Result<_, ContractError> {
            updated = rev_coin.coin_to_revert.amount.checked_add(amount_swapped.into()).unwrap();
            rev_coin.coin_to_revert.amount = updated;
            Ok(rev_coin)
        })?;        
    } else {
        REVERT_SWAP_CACHE.save(deps.storage, &SwapCache{
         coin_to_revert: coin(amount_swapped, denom_swapped.to_owned()),
        }
        )?
    }

    Ok(Response::default()
    .add_attributes([
        attr("revert_swap_received_amount", amount_swapped_string),
        attr("revert_swap_received_denom", denom_swapped),
        attr("revert_swap_amount_total", updated),]
        )
    )
    

 }
 
 fn handle_conjunction_swaps(deps: DepsMut, msg: Reply) -> Result<Response, ContractError>  {
    let swap_addr = SWAP_CONTRACT.load(deps.storage)?;
    let etf_name_cache = ETF_NAME_CACHE.load(deps.storage)?;
    // find pool for reverting transactions
    let depo_coin = INITIAL_DEPOSIT.load(deps.storage, (&etf_name_cache.sender, &etf_name_cache.etf_name))?;
    let (_, exit_pool_id) = get_initial_route_params(&depo_coin.denom)?;

    // use cache that stores all uosmo swapped back through messages created above
    let revert_swap_cache = REVERT_SWAP_CACHE.load(deps.storage)?;
    LEDGER.remove(deps.storage, (&etf_name_cache.sender, &etf_name_cache.etf_name));
    let execute_message = create_msg_execute_swap(
        swap_addr.to_string(), exit_pool_id, depo_coin.denom, 
        revert_swap_cache.coin_to_revert);

        Ok(Response::default().add_message(execute_message))
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

// ----------------------------------- QUERIES
// ##############################################################################

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

fn get_token_balance(_deps: DepsMut, info: MessageInfo, mint_contract: String, addr_to_query: String) -> Result<Response, ContractError> {
    if &addr_to_query != &info.sender {
        return Err(ContractError::Unauthorized {  })
    }

    let execute_query_balance = WasmMsg::Execute {
        contract_addr: mint_contract.to_string(),
        funds: vec![],
        msg: to_binary(&cw20_base::msg::QueryMsg::Balance {
            address: addr_to_query.to_string(),
        }).unwrap()
    };
    Ok(Response::new()
    .add_message(execute_query_balance))

}

// ----------------------------------- HELPER FUNCTIONS
// ##############################################################################

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
}

fn split_result_no_regex(coin_str: String) -> (String, String) {
    let position = coin_str.find(|c: char| !c.is_ascii_digit()).expect("did not find a split position");
    let (amount, denom) = coin_str.split_at(position);
    (amount.to_string(), denom.to_string())
}

fn get_initial_route_params(deposit_denom: &String) -> Result<(String, u64), ContractError> {
    let deposit_token_out_denom: String;
    let pool_id: u64;
    if deposit_denom == &"uosmo".to_string() {
        deposit_token_out_denom = "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string();
        pool_id = OSMO_ATOM_POOL_ID;
    } else if deposit_denom == &"usdc".to_string() {
        deposit_token_out_denom = "uosmo".to_string();
        pool_id = OSMO_USDC_POOL_ID; //678;
    } else {
        return Err(ContractError::InvalidDepositDenom {val: deposit_denom.clone()});
    };
    Ok((deposit_token_out_denom, pool_id))
}

fn parse_swap_reply(msg: &Reply) -> String {
    msg.result.clone()
    .unwrap()
    .events.iter()
    .filter(|event| event.ty == "token_swapped" && event.attributes[4].key == "tokens_out")
    .map(|p| p.attributes[4].value.clone())
    .collect()
}