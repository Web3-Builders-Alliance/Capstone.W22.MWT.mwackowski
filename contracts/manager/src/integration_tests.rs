#[cfg(test)]
mod tests {
    use crate::helpers::ManagerContract;
    use crate::msg::{ExecuteMsg, GetTokensResponse, QueryMsg, EtfSwapRoutes, InstantiateMsg, Route, GetInitialSwapResponse};
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};


    use osmo_swap::msg::InstantiateMsg as OsmoInstantiateMsg;

    use osmosis_testing::cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContractResponse;
    use osmosis_testing::{Gamm, Module, OsmosisTestApp, SigningAccount, Wasm, ExecuteResponse, Account};
    use std::path::PathBuf;


    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "denom";
   

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

    fn swap(app: &mut App, manager_contract: &ManagerContract, contract_addr: String,
    usdc_balance: Coin, etf_type: EtfSwapRoutes) {
        let msg = ExecuteMsg::SwapTokens { 
            initial_balance: usdc_balance, 
            etf_swap_routes: etf_type };

        let cosmos_msg = manager_contract.call(msg).unwrap();
        app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
    }

    fn get_tokens(app: &App, manager_contract: &ManagerContract, sender: String, etf_type: String ) -> GetTokensResponse {
        app.wrap()
            .query_wasm_smart(manager_contract.addr(), &QueryMsg::GetTokens { sender: sender, etf_type: etf_type })
            .unwrap()
    }

    // fn get_count(app: &App, contract_addr: &str) -> GetCountResponse {
    //     app.wrap()
    //         .query_wasm_smart(contract_addr, &counter::QueryMsg::GetCount {})
    //         .unwrap()
    // }
    fn parse_swap_init_resp(swap_response: ExecuteResponse<MsgExecuteContractResponse>) -> String {
        let result:String = swap_response
        .events
        .iter()
        .filter(|event| event.ty == "instantiate" && event.attributes[0].key == "_contract_address").map(|p| p.attributes[0].value.clone())
        .collect();
        result
    }

    fn with_env_setup(
        run: impl Fn(&OsmosisTestApp, Wasm<OsmosisTestApp>, SigningAccount, u64, String, u64, String)
    ) {
        let app = OsmosisTestApp::new();
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
        let swap_code_id = wasm
            .store_code(&get_wasm_byte_code("osmo_swap.wasm"), None, &signer)
            .unwrap()
            .data
            .code_id;
        let swap_resp = wasm
            .execute(&manager_contract_addr, &ExecuteMsg::InstantiateSwap { code_id: swap_code_id, debug: false }, 
                &[], &signer)
            .unwrap();
        let swap_contract_addr = parse_swap_init_resp(swap_resp);
        run(&app, wasm, signer, manage_code_id, manager_contract_addr, swap_code_id, swap_contract_addr)
    }


    fn setup_pool(app: &OsmosisTestApp, signer: &SigningAccount, first_token_denom: &str, second_token_denom: &str) -> u64 {
        let gamm = Gamm::new(app);
    
        // resulted in `mock_balancer_pool`
        let balancer_pool_id = gamm
            .create_basic_pool(
                &[
                    Coin::new(1_000, first_token_denom), 
                    Coin::new(1_000, second_token_denom)],
                signer,
            )
            .unwrap()
            .data
            .pool_id;
    
        balancer_pool_id
    }


    #[test]
    fn test_init() {
        with_env_setup(
            |_app, _wasm, signer, _code_id, 
                manager_contract_addr, _swap_code_id, swap_contract_addr| {
                    println!("manager addr: {:?}, swap addr: {:?}", manager_contract_addr, swap_contract_addr);
                    println!("signer: {:?}", signer.address());
            }
        );
    }
    
    // manager addr: "osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9", 
    // contract addr: "osmo1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqvlx82r"
    #[test]
    fn test_initial_swap() {
        with_env_setup(
            |app, wasm, signer, _code_id, 
                manager_contract_addr, swap_code_id, swap_contract_addr| {
                println!("manager addr: {:?}, contract addr: {:?}, signer addr: {:?}", manager_contract_addr, swap_contract_addr, signer.address());
                let pools = setup_pool(app, &signer, "uosmo", "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2");
                let pool_id = pools;
                let etf_name = "osmo_atom".to_string();
                let swap_resp = wasm

                    .execute(&manager_contract_addr, &ExecuteMsg::SwapTokens { 
                        initial_balance: Coin::new(11, "uosmo"), 
                        etf_swap_routes: EtfSwapRoutes { 
                            name: etf_name.to_owned(),
                            routes: vec![Route{
                                            pool_id: pool_id,
                                            token_out_denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string()
                                    }],
                            ratios: vec![Uint128::from(100u128)] 
                        } }, 
                        &vec![Coin::new(11, "uosmo")], &signer)
                    .unwrap();
                // println!("{:?}", swap_resp);
                let result: String = swap_resp.events.iter()
                    .filter(|event| event.ty == "wasm" && event.attributes[1].key == "initial_swap_received_amount")
                    .map(|p| p.attributes[1].value.clone())
                    .collect();
                
                // for ev in swap_resp.events.iter() {
                //     println!("{:?}", ev);
                // }

            // let res: GetTokensResponse = wasm
            // .query(&manager_contract_addr, &QueryMsg::GetTokens { 
            //     sender: manager_contract_addr.to_owned(), 
            //     etf_type: etf_name })
            //     .unwrap();
            // println!("{:?}", res);
            let res: GetInitialSwapResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetInitialSwap {  sender: signer.address() })
                .unwrap();
            
            // assert that the initial swap amount has been properly saved
            assert_eq!(result.parse::<u128>().unwrap(), res.initial_swap.amount.u128());

            }
        );
    }

    #[test]
    fn test_2_swaps() {
        with_env_setup(
            |app, wasm, signer, _code_id, 
                manager_contract_addr, swap_code_id, swap_contract_addr| {
                println!("manager addr: {:?}, contract addr: {:?}, signer addr: {:?}", manager_contract_addr, swap_contract_addr, signer.address());
                let pool_id_1 = setup_pool(app, &signer, "uosmo", "atom");
                let pool_id_2 = setup_pool(app, &signer, "usdc", "uosmo");
                let pool_id_3 = setup_pool(app, &signer, "uosmo", "uion");
                println!("{:?} {:?} {:?}", pool_id_1, pool_id_2, pool_id_3);
                let etf_name = "osmo_atom".to_string();
                let swap_resp = wasm

                    .execute(&manager_contract_addr, &ExecuteMsg::SwapTokens { 
                        initial_balance: Coin::new(30, "usdc"), 
                        etf_swap_routes: EtfSwapRoutes { 
                            name: etf_name.to_owned(),
                            routes: vec![Route{
                                            pool_id: pool_id_1,
                                            token_out_denom: "atom".to_string()},
                                        Route{
                                            pool_id: pool_id_3,
                                            token_out_denom: "uion".to_string()
                                        },
                                        ],
                            ratios: vec![Uint128::from(33u128), Uint128::from(67u128)] 
                        } }, 
                        &vec![Coin::new(30, "usdc")], &signer)
                    .unwrap();
                // println!("{:?}", swap_resp);
                let result: String = swap_resp.events.iter()
                    .filter(|event| event.ty == "wasm" && event.attributes[1].key == "initial_swap_received_amount")
                    .map(|p| p.attributes[1].value.clone())
                    .collect();
                
                for ev in swap_resp.events.iter() {
                    println!("{:?}", ev);
                }

            // let res: GetTokensResponse = wasm
            // .query(&manager_contract_addr, &QueryMsg::GetTokens { 
            //     sender: manager_contract_addr.to_owned(), 
            //     etf_type: etf_name })
            //     .unwrap();
            // println!("{:?}", res);
            let res: GetInitialSwapResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetInitialSwap {  sender: signer.address() })
                .unwrap();
            
            // assert that the initial swap amount has been properly saved
            assert_eq!(result.parse::<u128>().unwrap(), res.initial_swap.amount.u128());

            }
        );
    }
    
    #[test]
    fn create_one_counter() {
        // let (mut manager_app, mut osmo_app, manager_id, swap_id, swap_addr) = store_code();
        // let manager_contract = manager_instantiate(&mut manager_app, manager_id);

        // instantiate_new(&mut app, &manager_contract, counter_id);
        // let res = get_tokens(&app, &manager_contract, "blabla".to_string(), "first".to_string());
        // println!("{:?}", res);
        // assert_eq!(res.contracts.len(), 1);
        // assert_eq!(res.contracts[0].1.address, "contract1");
    }

    // #[test]
    // fn create_two_counters() {
    //     let (mut app, manager_id, counter_id) = store_code();
    //     let manager_contract = manager_instantiate(&mut app, manager_id);

    //     instantiate_new(&mut app, &manager_contract, counter_id);
    //     instantiate_new(&mut app, &manager_contract, counter_id);

    //     let res = get_tokens(&app, &manager_contract, "blabla".to_string(), "first".to_string());

    //     // assert_eq!(res.contracts.len(), 2);
    //     // assert_eq!(res.contracts[0].1.address, "contract1");
    //     // assert_eq!(res.contracts[1].1.address, "contract2");
    // }

    // #[test]
    // fn create_counter_and_increment() {
    //     let (mut app, manager_id, counter_id) = store_code();
    //     let manager_contract = manager_instantiate(&mut app, manager_id);

    //     instantiate_new(&mut app, &manager_contract, counter_id);
    //     increment(&mut app, &manager_contract, "contract1".to_string());

    //     let res = get_contracts(&app, &manager_contract);
    //     let res = get_count(&app, res.contracts[0].1.address.as_str());
    //     assert_eq!(res.count, 1);
    // }

    // #[test]
    // fn create_counter_and_increment_twice() {
    //     let (mut app, manager_id, counter_id) = store_code();

    //     let manager_contract = manager_instantiate(&mut app, manager_id);

    //     instantiate_new(&mut app, &manager_contract, counter_id);
    //     increment(&mut app, &manager_contract, "contract1".to_string());
    //     increment(&mut app, &manager_contract, "contract1".to_string());

    //     let res = get_contracts(&app, &manager_contract); // query contracts from manager
    //     let res = get_count(&app, res.contracts[0].1.address.as_str());
    //     assert_eq!(res.count, 2);
    // }

    // #[test]
    // fn create_counter_and_increment_and_reset() {
    //     let (mut app, manager_id, counter_id) = store_code();
    //     let manager_contract = manager_instantiate(&mut app, manager_id);
    //     instantiate_new(&mut app, &manager_contract, counter_id);

    //     increment(&mut app, &manager_contract, "contract1".to_string());

    //     let res = get_contracts(&app, &manager_contract);
    //     let res = get_count(&app, res.contracts[0].1.address.as_str());
    //     assert_eq!(res.count, 1);  

    //     reset(&mut app, &manager_contract, "contract1".to_string(), 0);
        
    //     let res = get_contracts(&app, &manager_contract); // query contracts from manager
    //     let res = get_count(&app, res.contracts[0].1.address.as_str()); 
    //     assert_eq!(res.count, 0);
    // }

    // #[test]
    // fn create_two_counters_and_increment_each() {
    //     let (mut app, manager_id, counter_id) = store_code();

    //     let manager_contract1 = manager_instantiate(&mut app, manager_id);
    //     let manager_contract2 = manager_instantiate(&mut app, manager_id);
    //     println!("counter id: {}", counter_id);
    //     instantiate_new(&mut app, &manager_contract1, counter_id); 
    //     instantiate_new(&mut app, &manager_contract2, counter_id);
        
    //     let res = get_contracts(&app, &manager_contract1);
    //     assert_eq!(res.contracts.len(), 1);
        
    //     let res = get_contracts(&app, &manager_contract2);
    //     assert_eq!(res.contracts.len(), 1);
        
    //     increment(&mut app, &manager_contract1, "contract2".to_string()); // adds 1 to counter (1)
    //     increment(&mut app, &manager_contract2, "contract3".to_string()); // adds 1 to counter (2)
    //     println!(">> manager_contract1 addr: {}", manager_contract1.addr());
    //     println!(">> manager_contract2 addr: {}", manager_contract2.addr());
    //     let res = get_contracts(&app, &manager_contract1); // query contracts from manager
    //     let res = get_count(&app, res.contracts[0].1.address.as_str()); 
    //     assert_eq!(res.count, 1);

    //     let res = get_contracts(&app, &manager_contract2); // query contracts from manager
    //     let res = get_count(&app, res.contracts[0].1.address.as_str()); 
    //     assert_eq!(res.count, 1);   
    // }
}