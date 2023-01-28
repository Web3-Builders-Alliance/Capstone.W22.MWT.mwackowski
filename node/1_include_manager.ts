import { SigningCosmWasmClient, Secp256k1HdWallet, GasPrice, Coin } from "cosmwasm";

import * as fs from 'fs';
import axios from 'axios';
import { ClientRequest } from "http";
import assert from "assert";


const rpcEndpoint= "https://rpc-test.osmosis.zone";
// const rpcEndpoint = "https://rpc.testnet.osmosis.zone";
// const rpcEndpoint = "https://testnet-rest.osmosis.zone/"

const swap_wasm = fs.readFileSync("../artifacts/osmo_swap.wasm");
const manager_wasm = fs.readFileSync("../artifacts/osmo_swap_manager.wasm");

const mnemonic =
"steak indicate rice motor change pond clarify sign fade call umbrella fork";

const swap_code_id = 5322; // 5102;
const manager_code_id = 5474;

const swap_addr = "osmo15h7qt607apjdfr4elgkp9rj3zplg4k839q5yd8mqzu3mhgddsc0qk59tzg";
// #"osmo1fjwcwk70ztzz48fahev0qxhlnpwmsy60jdm83v2vnc7kyhh9ph7srytxfq";
const manager_addr = "osmo165etpernqff33tpzesrplhln4j7j0kr4nt3rxpky42rj0prtklysl3rtac";



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
        let receiver = manager_addr;
        let res = await client.sendTokens(await getAddress(mnemonic), receiver, [{denom:"uosmo", amount:"3000"}], "auto");
        console.log(res);
    }).timeout(100000);

    xit("Balance Wallet Tokens", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let res = await client.getBalance(await getAddress(mnemonic), "uosmo");
        console.log(res);  
        let res2 = await client.getBalance(await getAddress(mnemonic), 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2');
        console.log(res2);   
    }).timeout(100000);


    //same as
    //junod tx wasm store artifacts/messages.wasm --from wallet --node https://rpc.uni.juno.deuslabs.fi --chain_id=uni-3 --gas-price=0.025uosmo --gas auto
    xit("1. Upload swap code to testnet", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let res = await client.upload(await getAddress(mnemonic), swap_wasm, "auto");
        console.log("Osmo contract: %s",JSON.stringify(res.logs[0].events));
    }).timeout(100000);

    it("2 Upload manager code to testnet", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let res = await client.upload(await getAddress(mnemonic), manager_wasm, "auto");
        console.log("Manager contract: %s",JSON.stringify(res.logs[0].events));
    }).timeout(100000);

    xit("3. Instantiate manager code on testnet", async () => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
            let res = await client.instantiate(await getAddress(mnemonic), manager_code_id, 
            {}, "messages", "auto");

        for (let i = 0; i<res.logs[0].events.length; i++) {
            console.log("------------EVENTS[%s]-----------------",i);
            console.log(res.logs[0].events[i]);          
        };
    }).timeout(60000);

    xit("Instantiate new swap contract", async() => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let res = await client.execute(await getAddress(mnemonic), 
        manager_addr, {  instantiate_swap: {code_id: swap_code_id, debug: false}},
        "auto", "", 
        [{amount: "1000", denom: "uosmo"}]);
        console.log(res);

        for (let i = 0; i<res.logs[0].events.length; i++) {
            console.log("------------EVENTS[%s]-----------------",i);
            console.log(res.logs[0].events[i]);          
        }
    }).timeout(20000);

    it("1. Execute swap", async() => {
        let client = await setupClient(mnemonic, rpcEndpoint, "0.025uosmo");
        let res = await client.execute(await getAddress(mnemonic), 
        manager_addr, {  swap_tokens: { 
            // contract: swap_addr,
            initial_balance: {amount: "3000", denom: "uosmo"}, 
            etf_swap_routes: 
            {
                name: "first_swap",
                routes: 
                [
                    // {pool_id: 1, token_out_denom: "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2"},
                    {pool_id: 12, token_out_denom: "ibc/A0CC0CF735BFB30E730C70019D4218A1244FF383503FF7579C9201AB93CA9293"},
                    {pool_id: 10, token_out_denom: "ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1"}, 
                    
                    {pool_id: 6, token_out_denom: "ibc/9712DBB13B9631EDFA9BF61B55F1B2D290B2ADB67E3A4EB3A875F3B6081B3B84"}
                ],
                ratios: 
                ["30", "40", "30"]
            }
        }},
        "auto", "", 
        // [{amount: "1000", denom: "uosmo"}]
        []
        );
        console.log(res);
        for (let i = 0; i<res.logs[0].events.length; i++) {
            console.log("------------EVENTS[%s]-----------------",i);
            console.log(res.logs[0].events[i]);          
        }
    }).timeout(20000);

});