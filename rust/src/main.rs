#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;

// Node access params
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

#[derive(Deserialize)]
struct SendResult {
    complete: bool,
    txid: String,
}

fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to Bitcoin Core RPC
    let rpc = Client::new(
        RPC_URL,
        Auth::User,
        Pass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Create wallets if they do not exist
    rpc.create_wallet("Miner", None, None, None)?;
    rpc.create_wallet("Trader", None, None, None)?;

    // Generate an address for the Miner wallet
    let miner_address = rpc.get_new_address(Some("Mining Reward"), None)?;

    // Mine blocks to the Miner wallet address
    let blocks_mined = rpc.generate_to_address(1, &miner_address)?;
    println!(
        "Mined {} blocks to address: {}",
        blocks_mined.len(),
        miner_address
    );

    // Check the balance of the Miner wallet
    let miner_balance = rpc.get_balance(Some("Miner"), None)?;
    println!("Miner wallet balance: {}", miner_balance);

    // Generate a receiving address for the Trader wallet
    let trader_address = rpc.get_new_address(Some("Received"), None)?;

    // Send 20 BTC from Miner to Trader
    let txid = send(&rpc, &trader_address.to_string())?;
    println!("Transaction ID: {}", txid);

    // Fetch the unconfirmed transaction from the mempool
    let mempool_entry = rpc.get_mempool_entry(&txid)?;
    println!("Mempool Entry: {:?}", mempool_entry);

    // Mine 1 block to confirm the transaction
    rpc.generate_to_address(1, &miner_address)?;

    // Extract transaction details
    let transaction_details = rpc.get_transaction(&txid, None, true)?;

    // Write the output to out.txt
    let mut output_file = File::create("out.txt")?;
    writeln!(output_file, "{}", transaction_details.txid)?;
    writeln!(output_file, "{}", miner_address)?;
    writeln!(output_file, "{}", miner_balance)?;
    writeln!(output_file, "{}", trader_address)?;
    writeln!(output_file, "20")?; // Amount sent to Trader
    writeln!(
        output_file,
        "{}",
        transaction_details.vout[0].script_pub_key.addresses[0]
    )?; // Change address
    writeln!(output_file, "{}", transaction_details.vout[1].value)?; // Change amount
    writeln!(output_file, "{}", transaction_details.fee.unwrap_or(0.0))?; // Transaction fee
    writeln!(
        output_file,
        "{}",
        transaction_details.blockheight.unwrap_or(-1)
    )?; // Block height
    writeln!(
        output_file,
        "{}",
        transaction_details.blockhash.unwrap_or_default()
    )?; // Block hash

    Ok(())
}
// You can use calls not provided in RPC lib API using the generic `call` function.
// An example of using the `send` RPC call, which doesn't have exposed API.
// You can also use serde_json `Deserialize` derivation to capture the returned json result.
// Function to send BTC to a specified address
fn send(rpc: &Client, addr: &str) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{ addr: 20 }]), // recipient address with amount
        json!(null),           // conf target
        json!(null),           // estimate mode
        json!(null),           // fee rate in sats/vb
        json!(null),           // Empty option object
    ];

    let send_result = rpc.call::<SendResult>("sendtoaddress", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}
