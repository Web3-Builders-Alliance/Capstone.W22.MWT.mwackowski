#[cfg(test)]
mod tests {
    use crate::helpers::ManagerContract;
    use crate::msg::{ExecuteMsg, GetTokensResponse, QueryMsg, EtfSwapRoutes, InstantiateMsg};
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};


    use osmo_swap::msg::InstantiateMsg as OsmoInstantiateMsg;
    use osmosis_testing::{Gamm, Module, OsmosisTestApp, SigningAccount, Wasm};
    use std::path::PathBuf;

    pub fn contract_manager() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    pub fn contract_swap() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            osmo_swap::contract::execute,
            osmo_swap::contract::instantiate,
            osmo_swap::contract::query,
        );
        Box::new(contract)
    }

    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "denom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1),
                    }],
                )
                .unwrap();
        })
    }

    fn store_code() -> (App, OsmosisTestApp, u64, u64, String) {
        let mut manager_app = mock_app();
        let manager_id = manager_app.store_code(contract_manager());
        let osmo_app = OsmosisTestApp::new();
        let wasm = Wasm::new(&osmo_app);
        let signer = osmo_app
            .init_account(&[
                Coin::new(100_000_000_000, "uosmo"),
                Coin::new(100_000_000_000, "uion"),
                Coin::new(100_000_000_000, "uusdc"),
                Coin::new(100_000_000_000, "uiou"),
            ])
            .unwrap();
    
        let swap_id = wasm
            .store_code(&get_wasm_byte_code(), None, &signer)
            .unwrap()
            .data
            .code_id;
        let swap_addr = wasm
            .instantiate(swap_id, &OsmoInstantiateMsg { debug: true }, None, None, &[], &signer)
            .unwrap()
            .data
            .address;
        println!("swap id: {}, manager id: {}", swap_id, manager_id);
        (manager_app, osmo_app, manager_id, swap_id, swap_addr)
    }

   

    fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("artifacts")
                .join("osmo_swap.wasm"),
        )
        .unwrap()
    }
    

    fn manager_instantiate(app: &mut App, manager_id: u64) -> ManagerContract {
        let msg = InstantiateMsg {};
        let manager_contract_address = app
            .instantiate_contract(
                manager_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "swap manager",
                None,
            )
            .unwrap();
        ManagerContract(manager_contract_address)
    }

    //call InstantiateNew on the manager contract to create a new counter instance.
    fn instantiate_new(app: &mut App, manager_contract: &ManagerContract, swap_id: u64) {
        let msg = ExecuteMsg::InstantiateSwap {
            code_id: swap_id,
        };
        println!("swap id from instantate_new(): {}", swap_id);
        let cosmos_msg = manager_contract.call(msg).unwrap();
        app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
        
    }

    //increment the counter from the manager contract
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

    #[test]
    fn create_one_counter() {
        let (mut manager_app, mut osmo_app, manager_id, swap_id, swap_addr) = store_code();
        let manager_contract = manager_instantiate(&mut manager_app, manager_id);

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
