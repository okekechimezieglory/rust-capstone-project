#![allow(unused)]
use bitcoincore_rpc::bitcoin::{Amount, Network, SignedAmount, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

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
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Create wallets if they do not exist
    rpc.create_wallet("Miner", None, None, None, Some(false))?;
    rpc.create_wallet("Trader", None, None, None, Some(false))?;

    // Generate an address for the Miner wallet
    let miner_address = rpc.get_new_address(Some("Mining Reward"), None)?;

    // Convert address to checked network address for mining
    let miner_checked = match miner_address.clone().require_network(Network::Regtest) {
        Ok(addr) => addr,
        Err(_) => {
            // If require_network fails, assume it's already correct and use assume_checked
            miner_address.assume_checked()
        }
    };

    // Mine blocks to the Miner wallet address
    let blocks_mined = rpc.generate_to_address(1, &miner_checked)?;
    println!(
        "Mined {} blocks to address: {}",
        blocks_mined.len(),
        miner_checked
    );

    // Check the balance of the Miner wallet
    let miner_balance = rpc.get_balance(None, None)?;
    println!("Miner wallet balance: {miner_balance}");

    // Generate a receiving address for the Trader wallet
    let trader_address = rpc.get_new_address(Some("Received"), None)?;

    // Send 20 BTC from Miner to Trader (clone to avoid move)
    let txid_str = send(&rpc, &trader_address.clone().assume_checked().to_string())?;

    // Parse transaction ID
    let txid = match Txid::from_str(&txid_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Failed to parse transaction ID: {e}");
            return Err(bitcoincore_rpc::Error::JsonRpc(
                bitcoincore_rpc::jsonrpc::Error::Transport(
                    format!("Invalid transaction ID: {e}").into(),
                ),
            ));
        }
    };

    println!("Transaction ID: {txid}");

    // Fetch the unconfirmed transaction from the mempool
    let mempool_entry = rpc.get_mempool_entry(&txid)?;
    println!("Mempool Entry: {mempool_entry:?}");

    // Mine 1 block to confirm the transaction
    rpc.generate_to_address(1, &miner_checked)?;

    // Extract transaction details
    let transaction_details = rpc.get_transaction(&txid, None)?;

    // Write the output to out.txt
    let mut output_file = File::create("out.txt")?;
    writeln!(output_file, "{}", transaction_details.info.txid)?;
    writeln!(output_file, "{miner_checked}")?;
    writeln!(output_file, "{miner_balance}")?;
    writeln!(output_file, "{}", trader_address.assume_checked())?;
    writeln!(output_file, "20")?; // Amount sent to Trader

    // Handle transaction details
    if let Some(first_detail) = transaction_details.details.first() {
        // Handle address - it might be None for some transaction types
        let address_str = match &first_detail.address {
            Some(addr) => addr.clone().assume_checked().to_string(),
            None => "N/A".to_string(),
        };
        writeln!(output_file, "{address_str}")?;
        writeln!(output_file, "{}", first_detail.amount.abs())?; // Use abs() to get positive amount
    } else {
        writeln!(output_file, "N/A")?;
        writeln!(output_file, "0")?;
    }

    // Transaction fee - handle SignedAmount properly
    let fee = transaction_details.fee.unwrap_or(SignedAmount::from_sat(0));
    writeln!(output_file, "{}", fee.abs().to_btc())?; // Transaction fee

    writeln!(
        output_file,
        "{}",
        transaction_details.info.blockheight.unwrap_or(u32::MAX)
    )?; // Block height

    writeln!(
        output_file,
        "{}",
        transaction_details
            .info
            .blockhash
            .map(|h| h.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    )?; // Block hash

    Ok(())
}

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
