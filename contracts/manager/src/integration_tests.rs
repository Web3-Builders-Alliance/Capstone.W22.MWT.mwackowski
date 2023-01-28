#[cfg(test)]
mod tests {
    use crate::helpers::ManagerContract;
    use crate::msg::{ExecuteMsg, GetTokensResponse, QueryMsg, EtfSwapRoutes, InstantiateMsg, Route, GetInitialSwapResponse};
    use cosmwasm_std::{Addr, Coin, Empty, Uint128, BankQuery};
    use cw_multi_test::{App};

    use cosmrs::proto::cosmos::bank::v1beta1::QueryAllBalancesRequest;

    use osmosis_testing::cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContractResponse;
    use osmosis_testing::{Gamm, Module, OsmosisTestApp, SigningAccount, Wasm, ExecuteResponse, Account, Runner, cosmrs, Bank};

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


    fn get_tokens(app: &App, manager_contract: &ManagerContract, sender: String, etf_type: String ) -> GetTokensResponse {
        app.wrap()
            .query_wasm_smart(manager_contract.addr(), &QueryMsg::GetTokens { sender: sender, etf_type: etf_type })
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

    fn with_env_setup(
        run: impl Fn(&OsmosisTestApp, Wasm<OsmosisTestApp>, SigningAccount, String, String, String)
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
        run(&app, wasm, signer, manager_contract_addr, swap_contract_addr, mint_contract_addr )
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

    #[test]
    fn test_init() {
        with_env_setup(
            |_app, _wasm, _signer, _manager_contract_addr, 
                _swap_contract_addr, _mint_contract_addr| {
            }
        );
    }

    #[test]
    fn test_initial_swap() {
        with_env_setup(
            |app, wasm, signer, manager_contract_addr, swap_contract_addr, mint_contract_addr| {
                let pools = setup_pool(app, &signer, "uosmo", "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2");
                let pool_id = pools;
                let etf_name = "WladziooEtf_First".to_string();
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
                let inital_swap_received_amount: String = swap_resp.events.iter()
                    .filter(|event| event.ty == "wasm" && event.attributes[1].key == "initial_swap_received_amount")
                    .map(|p| p.attributes[1].value.clone())
                    .collect();
                
            let res_query_get_initial_swap: GetInitialSwapResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetInitialSwap {  sender: signer.address() })
                .unwrap();
            
            // assert that the initial swap amount has been properly saved
            assert_eq!(inital_swap_received_amount.parse::<u128>().unwrap(), res_query_get_initial_swap.initial_swap.amount.u128());
            for ev in swap_resp.events.iter() {
                println!("{:?}", ev);
            };

            let signers_minted_balance: cw20::BalanceResponse = wasm
                    .query(
                        &mint_contract_addr,
                        &cw20_base::msg::QueryMsg::Balance { address: signer.address() }
                    )
                    .unwrap();
            let minted_tokens: u128 = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].value == "mint")
                .map(|p| p.attributes[2].value.clone().parse::<u128>().unwrap())
                .sum();
            // assert that the amount of minted tokens is equal to signer's balance
            assert_eq!(signers_minted_balance.balance.u128(), minted_tokens);
            

        });
    }

    #[test]
    fn test_2_swaps() {
        with_env_setup(
            |app, wasm, signer, manager_contract_addr, swap_contract_addr, mint_contract_addr| {
                let pool_id_1 = setup_pool(app, &signer, "uosmo", "atom");
                let pool_id_2 = setup_pool(app, &signer, "uosmo", "usdc");
                let pool_id_3 = setup_pool(app, &signer, "uosmo", "uion");
                let pool_id_4 = setup_pool(app, &signer, "uosmo", "uiou");

                let etf_name = "WladziooEtf_First".to_string();
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
                 
            let result: String = swap_resp.clone().events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key == "initial_swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();

            let res: GetInitialSwapResponse = wasm
                .query(&manager_contract_addr, &QueryMsg::GetInitialSwap {  sender: signer.address().to_owned() })
                .unwrap();
            
            // assert that the initial swap amount has been properly saved in storage
            assert_eq!(result.parse::<u128>().unwrap(), res.initial_swap.amount.u128());

            let query_tokens_res: GetTokensResponse = wasm
            .query(&manager_contract_addr, &QueryMsg::GetTokens { 
                sender: signer.address(), 
                etf_type: etf_name.to_owned() })
                .unwrap();

            let swap_received_amounts: Vec<String> = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].key =="swap_received_amount")
                .map(|p| p.attributes[1].value.clone())
                .collect();
            
            for ev in swap_resp.events.iter() {
                println!("{:?}", ev);
            }

            let sum_received: u128 = swap_received_amounts.iter().map(|amnt| amnt.parse::<u128>().unwrap()).sum();
            let sum_query_ledger: u128 = query_tokens_res.tokens_per_etf.iter().map(|c| c.amount.u128()).sum();
            // assert that total received amount has been properly saved into storage (ledger)
            assert_eq!(sum_received, sum_query_ledger);

            let signers_minted_balance: cw20::BalanceResponse = wasm
                    .query(
                        &mint_contract_addr,
                        &cw20_base::msg::QueryMsg::Balance { address: signer.address() }
                    )
                    .unwrap();

            let minted_tokens: u128 = swap_resp.events.iter()
                .filter(|event| event.ty == "wasm" && event.attributes[1].value == "mint")
                .map(|p| p.attributes[2].value.clone().parse::<u128>().unwrap())
                .sum();
            // assert that the amount of minted tokens is equal to signer's balance
            assert_eq!(signers_minted_balance.balance.u128(), minted_tokens);


            });
    }
    
}