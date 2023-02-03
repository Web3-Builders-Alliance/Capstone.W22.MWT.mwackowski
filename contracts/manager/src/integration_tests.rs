#[cfg(test)]
mod tests {
    use crate::helpers::ManagerContract;
    use crate::msg::{ExecuteMsg, GetTokensResponse, QueryMsg, EtfSwapRoutes, InstantiateMsg, Route, GetInitialSwapResponse, GetBalanceResponse};
    use cosmwasm_std::{Coin, Uint128};
    use cw_multi_test::{App};
    use cosmrs::proto::cosmos::bank::v1beta1::QueryAllBalancesRequest;
    use osmosis_testing::cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContractResponse;
    use osmosis_testing::{Gamm, Module, OsmosisTestApp, SigningAccount, Wasm, ExecuteResponse, Account, Bank, cosmrs};
    use cw20_base;
    use std::path::PathBuf;


    fn get_wasm_byte_code(filename: &str) -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("artifacts")
                .join(filename),
        )
        .unwrap()
    }
 
    fn parse_init_response(swap_response: ExecuteResponse<MsgExecuteContractResponse>) -> String {
        let result:String = swap_response
        .events
        .iter()
        .filter(|event| event.ty == "instantiate" && event.attributes[0].key == "_contract_address").map(|p| p.attributes[0].value.clone())
        .collect();
        result
    }

    // basic environment setup that will be used throughout tests
    fn with_env_setup(
        run: impl Fn(&OsmosisTestApp, Wasm<OsmosisTestApp>, SigningAccount, SigningAccount, String, String, String)
    ) {
        let app = OsmosisTestApp::default();
        let wasm = Wasm::new(&app);
        let signer = app
            .init_account(&[
                Coin::new(100_000_000_000, "uosmo"),
                Coin::new(100_000_000_000, "uion"),
                Coin::new(100_000_000_000, "usdc"),
                Coin::new(100_000_000_000, "uiou"),
                Coin::new(100_000_000_000, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"),
                Coin::new(100_000_000_000, "atom")
            ])
            .unwrap();
        let signer2 = app
            .init_account(&[
                Coin::new(100_000_000_000, "uosmo"),
                Coin::new(100_000_000_000, "uion"),
                Coin::new(100_000_000_000, "usdc"),
                Coin::new(100_000_000_000, "uiou"),
                Coin::new(100_000_000_000, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"),
                Coin::new(100_000_000_000, "atom")
            ])
            .unwrap();
        let manage_code_id = wasm
            .store_code(&get_wasm_byte_code("osmo_swap_manager.wasm"), None, &signer)
            .unwrap()
            .data
            .code_id;
        let manager_contract_addr = wasm
            .instantiate(manage_code_id, &InstantiateMsg { }, None, None, &[], &signer)
            .unwrap()
            .data
            .address;
        let mint_code_id = wasm
            .store_code(&get_wasm_byte_code("cw20_base.wasm"), None, &signer)
            .unwrap()
            .data
            .code_id;
        let mint_init_resp = wasm
            .execute(&manager_contract_addr, &ExecuteMsg::InstantiateCw20 { 
                etf_name: "WladziooEtf_First".to_string(), 
                etf_symbol: "wetfone".to_string(), 
                code_id: mint_code_id
            }, &[], &signer)
            .unwrap();
        let swap_code_id = wasm
            .store_code(&get_wasm_byte_code("osmo_swap.wasm"), None, &signer)
            .unwrap()
            .data
            .code_id;
        let swap_resp = wasm
            .execute(&manager_contract_addr, &ExecuteMsg::InstantiateSwap { code_id: swap_code_id, debug: false }, 
                &[], &signer)
            .unwrap();
        let swap_contract_addr = parse_init_response(swap_resp);
        let mint_contract_addr = parse_init_response(mint_init_resp);
        println!("manager addr: {:?},\ncontract addr: {:?},\nsigner addr: {:?},\nmint addr: {:?}", 
            manager_contract_addr, swap_contract_addr, signer.address(), mint_contract_addr);
        run(&app, wasm, signer, signer2, manager_contract_addr, swap_contract_addr, mint_contract_addr )
    }


    fn setup_pool(app: &OsmosisTestApp, signer: &SigningAccount, first_token_denom: &str, second_token_denom: &str) -> u64 {
        let gamm = Gamm::new(app);
    
        // resulted in `mock_balancer_pool`
        let balancer_pool_id = gamm
            .create_basic_pool(
                &[
                    Coin::new(10_000, first_token_denom), 
                    Coin::new(10_000, second_token_denom)],
                signer,
            )
            .unwrap()
            .data
            .pool_id;
    
        balancer_pool_id
    }

    fn execute_swap(wasm: &Wasm<OsmosisTestApp>, contract_address: String, signer: &SigningAccount, init_balance: Coin, etf_name: &String,
    routes: Vec<Route>, ratios: Vec<Uint128>) -> ExecuteResponse<MsgExecuteContractResponse> {
        let swap_resp = wasm

        .execute(&contract_address, &ExecuteMsg::SwapTokens { 
            initial_balance: init_balance.clone(), 
            etf_swap_routes: EtfSwapRoutes { 
                name: etf_name.to_owned(),
                routes: routes,
                ratios: ratios
            } }, 
            &vec![init_balance], &signer)
        .unwrap();
        swap_resp
    }

    #[test]
    fn test_init() {
        with_env_setup(
            |_app, _wasm, _signer, _signer2, _manager_contract_addr, 
                _swap_contract_addr, _mint_contract_addr| {
            }
        );
    }

    #[test]
    fn test_2_initial_swaps_with_2_signers() {
        with_env_setup(
            |app, wasm, signer, signer2, manager_contract_addr, swap_contract_addr, mint_contract_addr| {
            let pools = setup_pool(app, &signer, "uosmo", "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2");
            let pool_id = pools;
            let etf_name = "WladziooEtf_First".to_string();
            let initial_coin = Coin::new(11, "uosmo");
            let swap_resp = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer, initial_coin, &etf_name,
                vec![Route{
                    pool_id: pool_id,
                    token_out_denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string()
                    }],
                vec![Uint128::from(100u128)] 
            );

            let inital_swap_received_amount: String = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key == "initial_swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();
                
            let res_query_get_initial_swap: GetInitialSwapResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetInitialSwap {  sender: signer.address() })
                .unwrap();
            
            // assert that the initial swap amount has been properly saved
            assert_eq!(inital_swap_received_amount.parse::<u128>().unwrap(), res_query_get_initial_swap.initial_swap.amount.u128());
            let minted_tokens_after_first_swap: u128 = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].value == "mint")
                .map(|p| p.attributes[2].value.clone().parse::<u128>().unwrap())
                .sum();
            println!(">>> tokens minted: {:?}", minted_tokens_after_first_swap);

            // add second swap
            let initial_coin = Coin::new(19, "uosmo");
            let swap_resp = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer2, initial_coin, &etf_name,
                vec![Route{
                    pool_id: pool_id,
                    token_out_denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string()
                    }],
                vec![Uint128::from(100u128)] 
            );

            let minted_tokens_after_second_swap: u128 = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].value == "mint")
                .map(|p| p.attributes[2].value.clone().parse::<u128>().unwrap())
                .sum();

            let managers_minted_balance: cw20::BalanceResponse = wasm
                .query(
                    &mint_contract_addr,
                    &cw20_base::msg::QueryMsg::Balance { address: manager_contract_addr.to_owned() }
                )
                .unwrap();

            // assert that the amount of minted tokens is equal to signer's balance
            assert_eq!(managers_minted_balance.balance.u128(), minted_tokens_after_second_swap + minted_tokens_after_first_swap);
            println!(">>> tokens minted: {:?}", minted_tokens_after_second_swap);

            // query balances of particular signers
            let first_signers_balance: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer.address(), etf_type: etf_name.to_owned() }
                ).unwrap();
            assert_eq!(first_signers_balance.balance.amount.u128(), minted_tokens_after_first_swap);
            let second_signers_balance: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer2.address(), etf_type: etf_name.to_owned() }
                ).unwrap();
            assert_eq!(second_signers_balance.balance.amount.u128(), minted_tokens_after_second_swap);

        });
    }

    #[test]
    fn test_2_full_swaps_one_signer() {
        with_env_setup(
            |app, wasm, signer, _signer2, manager_contract_addr, _swap_contract_addr, mint_contract_addr| {
            let pool_id_1 = setup_pool(app, &signer, "uosmo", "atom");
            let pool_id_2 = setup_pool(app, &signer, "uosmo", "usdc");
            let pool_id_3 = setup_pool(app, &signer, "uosmo", "uion");
            let pool_id_4 = setup_pool(app, &signer, "uosmo", "uiou");

            let etf_name = "WladziooEtf_First".to_string();
            let initial_coin = Coin::new(30, "usdc");
            let swap_resp = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer, initial_coin.to_owned(), &etf_name,
                vec![
                    Route{pool_id: pool_id_1,
                    token_out_denom: "atom".to_string()},
                    Route{pool_id: pool_id_3,
                        token_out_denom: "uion".to_string()}],
                        vec![Uint128::from(33u128), Uint128::from(67u128)] 
                );
            let initial_coin2 = Coin::new(51, "usdc");

            let swap_resp2 = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer, initial_coin2.to_owned(), &etf_name,
                vec![
                    Route{pool_id: pool_id_3,
                    token_out_denom: "uion".to_string()},
                    Route{pool_id: pool_id_4,
                        token_out_denom: "uiou".to_string()}],
                        vec![Uint128::from(50u128), Uint128::from(50u128)] 
                );

            let query_tokens_res: GetTokensResponse = wasm
            .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer.address(), 
                etf_type: etf_name.to_owned() })
                .unwrap();

            let swap_received_amounts: Vec<String> = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key =="swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();
            let swap_received_amounts2: Vec<String> = swap_resp2.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key =="swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();
            let minted_tokens: u128 = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].value == "mint")
                .map(|p| p.attributes[2].value.clone().parse::<u128>().unwrap())
                .sum();
            let minted_tokens2: u128 = swap_resp2.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].value == "mint")
                .map(|p| p.attributes[2].value.clone().parse::<u128>().unwrap())
                .sum();
            let manager_minted_balance: cw20::BalanceResponse = wasm
                .query(
                    &mint_contract_addr,
                    &cw20_base::msg::QueryMsg::Balance { address: manager_contract_addr.to_owned() }
                )
                .unwrap();
            let users_depo_balance: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer.address(), etf_type: etf_name }
                ).unwrap();

            for ev in swap_resp.events.iter() {
                println!("{:?}", ev);
            }

            let sum_received: u128 = swap_received_amounts.iter().map(|amnt| amnt.parse::<u128>().unwrap()).sum();
            let sum_received2: u128 = swap_received_amounts2.iter().map(|amnt| amnt.parse::<u128>().unwrap()).sum();
            let sum_query_ledger: u128 = query_tokens_res.tokens_per_etf.iter().map(|c| c.amount.u128()).sum();

            // assert that total received amount has been properly saved into storage (ledger)
            assert_eq!(sum_received + sum_received2, sum_query_ledger);

            // assert that the amount of minted tokens is equal to manager's balance in cw20 storage
            assert_eq!(manager_minted_balance.balance.u128(), minted_tokens + minted_tokens2);

            // assert that the user have balance properly stored
            assert_eq!(users_depo_balance.balance.amount, initial_coin.amount + initial_coin2.amount);
            
            });
    }
    
    #[test]
    fn test_swaps_and_redeem() {
        with_env_setup(
            |app, wasm, signer, signer2, manager_contract_addr, _swap_contract_addr, mint_contract_addr| {
            let pool_id_1 = setup_pool(app, &signer, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", "uosmo");
            let pool_id_2 = setup_pool(app, &signer, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", "usdc");
            let pool_id_3 = setup_pool(app, &signer, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", "uion");
            let pool_id_4 = setup_pool(app, &signer, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", "uiou");
            let pool_id_5 = setup_pool(app, &signer, "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2", "atom");
            // instantiate additional mint contract
            let mint_code_id = wasm
                .store_code(&get_wasm_byte_code("cw20_base.wasm"), None, &signer)
                .unwrap()
                .data
                .code_id;
            let etf_name2 = "Come_Buidl_With_Us".to_string();
            let mint_init_resp = wasm
                .execute(&manager_contract_addr, &ExecuteMsg::InstantiateCw20 { 
                    etf_name: etf_name2.to_string(),
                    etf_symbol: "wba".to_string(), 
                    code_id: mint_code_id
                }, &[], &signer)
                .unwrap();
            let mint_contract_addr2 = parse_init_response(mint_init_resp);

            // FIRST SIGNER - TWO SWAPS WITH DIFFERENT CONTRACTS

            let etf_name = "WladziooEtf_First".to_string();
            let initial_coin = Coin::new(22000, "uosmo");
            let swap_resp = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer, initial_coin.clone(), &etf_name,
                vec![
                    Route{pool_id: pool_id_4,
                    token_out_denom: "uiou".to_string()},
                    Route{pool_id: pool_id_3,
                        token_out_denom: "uion".to_string()}
                    ],
                        vec![Uint128::from(33u128), Uint128::from(67u128)] 
                );

            let initial_coin2 = Coin::new(33000, "uosmo");
            let swap_resp2 = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer, initial_coin2.clone(), &etf_name2,
                vec![
                    Route{pool_id: pool_id_1,
                    token_out_denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string()},
                    Route{pool_id: pool_id_5,
                        token_out_denom: "atom".to_string()}
                    ],
                        vec![Uint128::from(33u128), Uint128::from(67u128)] 
                );
            
            // SECOND SIGNER - ONE SWAP 
            let initial_coin3 = Coin::new(44000, "uosmo");
            let swap_resp3 = execute_swap(
                &wasm, manager_contract_addr.to_owned(), &signer2, initial_coin3.clone(), &etf_name,
                vec![
                    Route{pool_id: pool_id_5,
                    token_out_denom: "atom".to_string()},
                    Route{pool_id: pool_id_1,
                        token_out_denom: "uosmo".to_string()}
                    ],
                        vec![Uint128::from(33u128), Uint128::from(67u128)] 
                );
            let tokens_res_first_signer_first_swap: GetTokensResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer.address(), 
                etf_type: etf_name.to_owned() })
                .unwrap();

            let tokens_res_first_signer_second_swap: GetTokensResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer.address(), 
                etf_type: etf_name2.to_owned() })
                .unwrap();

            let tokens_res_second_signer: GetTokensResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer2.address(), 
                etf_type: etf_name.to_owned() })
                .unwrap();

            let swap_received_amounts: Vec<String> = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key =="swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();
            let swap_received_amounts2: Vec<String> = swap_resp2.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key =="swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();    
            let swap_received_amounts3: Vec<String> = swap_resp3.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key =="swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();             
            let bob_balance = Bank::new(app)
                .query_all_balances(&QueryAllBalancesRequest {
                    address: signer.address(),
                    pagination: None,
                })
                .unwrap().balances.into_iter().find(|c| c.denom == "uosmo")
                .unwrap().amount.parse::<u128>().unwrap();
            let alice_balance = Bank::new(app)
                .query_all_balances(&QueryAllBalancesRequest {
                    address: signer2.address(),
                    pagination: None,
                })
                .unwrap().balances.into_iter().find(|c| c.denom == "uosmo")
                .unwrap().amount.parse::<u128>().unwrap();
            println!(">>> Bob balance: {:?}", bob_balance);
            println!(">>> Alice balance: {:?}", alice_balance);
            let sum_received: u128 = swap_received_amounts.iter().map(|amnt| amnt.parse::<u128>().unwrap()).sum();
            let sum_received2: u128 = swap_received_amounts2.iter().map(|amnt| amnt.parse::<u128>().unwrap()).sum();
            let sum_received3: u128 = swap_received_amounts3.iter().map(|amnt| amnt.parse::<u128>().unwrap()).sum();
            let sum_query_ledger: u128 = tokens_res_first_signer_first_swap.tokens_per_etf.iter().map(|c| c.amount.u128()).sum();
            let sum_query_ledger2: u128 = tokens_res_first_signer_second_swap.tokens_per_etf.iter().map(|c| c.amount.u128()).sum();
            let sum_query_ledger3: u128 = tokens_res_second_signer.tokens_per_etf.iter().map(|c| c.amount.u128()).sum();
            assert_eq!(sum_received, sum_query_ledger);
            // assert_ne - swapping of ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2 has
            // already been done in initial swap, therefore simple sum of attributes won't work here as it is not visible 
            assert_ne!(sum_received2, sum_query_ledger2); 
            assert_eq!(sum_received3, sum_query_ledger3);


            let users_balance: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer.address(), etf_type: etf_name.to_owned() }
            ).unwrap();
            let users_balance2: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer.address(), etf_type: etf_name2.to_owned() }
            ).unwrap();
            let users_balance3: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer2.address(), etf_type: etf_name.to_owned() }
            ).unwrap();
            println!("balance sender1, etf1: {:?}", users_balance);
            println!("balance sender1, etf2: {:?}", users_balance2);
            println!("balance sender2, etf1: {:?}", users_balance3);
            assert_eq!(users_balance.balance.amount, initial_coin.amount);
            assert_eq!(users_balance2.balance.amount, initial_coin2.amount);
            assert_eq!(users_balance3.balance.amount, initial_coin3.amount);
            
            let manager_minted_balance: cw20::BalanceResponse = wasm
                .query(
                &mint_contract_addr,
                &cw20_base::msg::QueryMsg::Balance { address: manager_contract_addr.to_owned() }
            ).unwrap();
            let manager_minted_balance2: cw20::BalanceResponse = wasm
                .query(
                &mint_contract_addr2,
                &cw20_base::msg::QueryMsg::Balance { address: manager_contract_addr.to_owned() }
            ).unwrap();

            // assert that the amount of minted tokens is equal to users' balances
            assert_eq!(manager_minted_balance.balance + manager_minted_balance2.balance, 
                users_balance.balance.amount + users_balance2.balance.amount + users_balance3.balance.amount);

            // REDEEM tokens
            let redeem_resp = wasm
                .execute(&manager_contract_addr, &ExecuteMsg::RedeemTokens { etf_name: etf_name.to_owned() }, 
                &vec![Coin::new(1000, "uosmo")], &signer)
                .unwrap();
            println!("{:?}", redeem_resp);

            let query_tokens_first_signer_first_swap_after_redeeming: Result<GetTokensResponse, osmosis_testing::RunnerError> = wasm
                .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer.address(), 
                etf_type: etf_name.to_owned() });
            let query_tokens_first_signer_second_swap_after_redeeming: GetTokensResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer.address(), 
                etf_type: etf_name2.to_owned() })
                .unwrap();
            let query_tokens_second_signer_after_redeeming: GetTokensResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer2.address(), 
                etf_type: etf_name.to_owned() })
                .unwrap();
            // after the tokens have been redeemed for particular swap, ledger of redeeming user should be empty (returns error: Not Found when queried)
            // the others should remain unchanged
            assert!(query_tokens_first_signer_first_swap_after_redeeming.is_err());
            assert_eq!(query_tokens_first_signer_second_swap_after_redeeming.tokens_per_etf, tokens_res_first_signer_second_swap.tokens_per_etf);
            assert_eq!(query_tokens_second_signer_after_redeeming.tokens_per_etf, tokens_res_second_signer.tokens_per_etf);

            let manager_minted_balance: cw20::BalanceResponse = wasm
                .query(
                &mint_contract_addr,
                &cw20_base::msg::QueryMsg::Balance { address: manager_contract_addr.to_owned() }
                ).unwrap();
            let manager_minted_balance2: cw20::BalanceResponse = wasm
                .query(
                &mint_contract_addr2,
                &cw20_base::msg::QueryMsg::Balance { address: manager_contract_addr.to_owned() }
                ).unwrap();

            // now minted balances should be equal to 2 coins out of initial 3
            assert_eq!(manager_minted_balance.balance + manager_minted_balance2.balance, initial_coin3.amount + initial_coin2.amount);
            // REDEEM tokens for remaining users
            let redeem_resp = wasm
                .execute(&manager_contract_addr, &ExecuteMsg::RedeemTokens { etf_name: etf_name2.to_owned() }, 
                &vec![Coin::new(1000, "uosmo")], &signer)
                .unwrap();

            let redeem_resp = wasm
                .execute(&manager_contract_addr, &ExecuteMsg::RedeemTokens { etf_name: etf_name.to_owned() }, 
                &vec![Coin::new(1000, "uosmo")], &signer2)
                .unwrap();
            println!("{:?}", redeem_resp);           
            let users_balance_after_redeeming: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer.address(), etf_type: etf_name.to_owned() }
            ).unwrap();
            let users_balance_after_redeeming2: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer.address(), etf_type: etf_name2.to_owned() }
            ).unwrap();
            let users_balance_after_redeeming3: GetBalanceResponse = wasm
                .query(
                &manager_contract_addr,
                &QueryMsg::GetBalance { sender: signer2.address(), etf_type: etf_name.to_owned() }
            ).unwrap();

            // first users balance should be now = 0, as it has been redeemed, but the rest should have the same value as before
            assert_eq!(users_balance_after_redeeming.balance.amount, Uint128::zero());
            assert_eq!(users_balance_after_redeeming2.balance.amount, Uint128::zero());
            assert_eq!(users_balance_after_redeeming3.balance.amount, Uint128::zero());
            print!("users balance in storage {:?}", users_balance_after_redeeming);
            print!("users balance in storage2 {:?}", users_balance_after_redeeming2);
            print!("users balance in storage3 {:?}", users_balance_after_redeeming3);

            let bob_balance = Bank::new(app)
                .query_all_balances(&QueryAllBalancesRequest {
                    address: signer.address(),
                    pagination: None,
                })
                .unwrap().balances.into_iter().find(|c| c.denom == "uosmo")
                .unwrap().amount.parse::<u128>().unwrap();
            let alice_balance = Bank::new(app)
                .query_all_balances(&QueryAllBalancesRequest {
                    address: signer2.address(),
                    pagination: None,
                })
                .unwrap().balances.into_iter().find(|c| c.denom == "uosmo")
                .unwrap().amount.parse::<u128>().unwrap();
            println!(">>> Bob balance: {:?}", bob_balance);
            println!(">>> Alice balance: {:?}", alice_balance);
            });

    }

}