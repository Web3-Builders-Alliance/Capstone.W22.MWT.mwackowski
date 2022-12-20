import { SigningCosmWasmClient, Secp256k1HdWallet, GasPrice, Coin } from "cosmwasm";

import * as fs from 'fs';
import axios from 'axios';
import { ClientRequest } from "http";
import assert from "assert";


const rpcEndpoint= "https://rpc-test.osmosis.zone";

const swap_wasm = fs.readFileSync("../artifacts/osmo_swap.wasm");

const mnemonic =
"steak indicate rice motor change pond clarify sign fade call umbrella fork";

const swap_code_id = 4880;

const swap_addr = "osmo1qydna99vxzx2fyrcsn8psugwctxtr5sxnxcajcy0300nd7fr3d3qa8pgzm";


async function setupClient(mnemonic: string, rpc: string, gas: string | undefined): Promise<SigningCosmWasmClient> {
    if (gas === undefined) {
        let wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'osmo'});
        let client = await SigningCosmWasmClient.connectWithSigner(rpc, wallet);
        return client;
    } else {
        let gas_price = GasPrice.fromString(gas);
        let wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'osmo' });
        let client = await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice: gas_price });
        return client;
    }
}

async function getAddress(mnemonic: string, prefix: string = 'osmo') {
    let wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic, { prefix });
    let accounts = await wallet.getAccounts();
    return accounts[0].address;
}

async function delay(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

describe("swap Fullstack Test", () => {
    xit("Generate Wallet", async () => {
        let wallet = await Secp256k1HdWallet.generate(12);
        console.log(wallet.mnemonic);
    });

    xit("Get Address", async() => {
        console.log(await getAddress(mnemonic));
    }).timeout(200000);

    xit("Get Testnet Tokens", async () => {
        console.log(await getAddress(mnemonic));
        try {
            let res = await axios.post("https://faucet.osmosis.zone", { "denom": "uosmo", "address": await getAddress(mnemonic) });
            console.log(res);
        } catch (e) {
            console.log(e);
        }
    }).timeout(100000);

    xit("Send Testnet Tokens", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let receiver = "";
        let res = await client.sendTokens(await getAddress(mnemonic), receiver, [{denom:"uosmo", amount:"1000000"}], "auto");
        console.log(res);
    }).timeout(100000);

    xit("Balance Testnet Tokens", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let searchDenom: string = 'uosmo';
        let res = await client.getBalance(await getAddress(mnemonic), "uosmo");
        console.log(res);        
    }).timeout(100000);


    //same as
    //junod tx wasm store artifacts/messages.wasm --from wallet --node https://rpc.uni.juno.deuslabs.fi --chain_id=uni-3 --gas-price=0.025uosmo --gas auto
    xit("1. Upload code to testnet", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let res = await client.upload(await getAddress(mnemonic), swap_wasm, "auto");
        console.log("Osmo contract: %s",JSON.stringify(res.logs[0].events));
    }).timeout(100000);


    xit("2. Instantiate contract code on testnet", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
            let res = await client.instantiate(await getAddress(mnemonic), swap_code_id, 
            {}, "messages", "auto");

        //InstantiateMsg {name: String, symbol: String, decimals: u8, initial_balances: Vec<Cw20Coin>, mint: Option<MinterResponse>, marketing: Option<InstantiateMarketingInfo>,}
        //  Cw20Coin {address: String, amount: Uint128,}
 
        //console.log(res);
        for (let i = 0; i<res.logs[0].events.length; i++) {
            console.log("------------EVENTS[%s]-----------------",i);
            console.log(res.logs[0].events[i]);          
        };
    }).timeout(100000);

    it("3. Query pool", async () => {
            let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
            let res = await client.queryContractSmart(swap_addr, { pool_state : {id: 1}});

            console.log(res);
        }).timeout(100000);

    // xit("4. Mint some on cw20 contract code on testnet", async () => {
    //     let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
    //     let res = await client.execute(minter_addr
    //     , base_contract_address
    //     , {mint : { recipient: user_addr, amount: "1000"}}
    //     , "auto"
    //     , "memo"
    //     , []
    //     ); 

    //     for (let i = 0; i<res.logs[0].events.length; i++) {
    //         console.log("------------EVENTS[%s]-----------------",i);
    //         console.log(res.logs[0].events[i]);          
    //     };
    // }).timeout(100000);
    
    // xit("6. Create allowance for a user on a cw20 contract on testnet", async () => {
    //     // Increase allowance for the user that is going to create the stream. That is how the ExecuteMsg::SendFrom {owner,contract,amount,msg,}  is coded.
    //     // Msg:        ExecuteMsg::IncreaseAllowance { spender: String,   amount: Uint128,   expires: Option<Expiration>,  }
    //     let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
    //     let res = await client.execute(minter_addr
    //     , base_contract_address
    //     , {increase_allowance : { spender: user_addr, amount: "100"}}
    //     , "auto"
    //     , "memo"
    //     , []
    //     ); 

    //     for (let i = 0; i<res.logs[0].events.length; i++) {
    //         console.log("------------EVENTS[%s]-----------------",i);
    //         console.log(res.logs[0].events[i]);
    //     };
    //     console.log(res);
    // }).timeout(100000);
});