use crate::rpc;
use anyhow::Result;
use clap::{Args, Subcommand};
use kamichain_core::Transaction;
use kamichain_wallet::Wallet;

#[derive(Args)]
pub struct TxArgs {
    #[command(subcommand)]
    pub command: TxCmd,
}

#[derive(Subcommand)]
pub enum TxCmd {
    /// Sign and submit a transfer transaction
    Send {
        /// Keyfile holding the sender's private key
        #[arg(long, default_value = "wallet.key")]
        keyfile: String,
        /// Recipient address
        #[arg(long)]
        to: String,
        /// Amount to send (in base units)
        #[arg(long)]
        amount: u64,
        /// Optional fee for priority ordering in the mempool
        #[arg(long, default_value_t = 0)]
        fee: u64,
        #[arg(long, default_value = "127.0.0.1:8332")]
        node: String,
    },
    /// Look up a confirmed transaction by ID
    Get {
        /// Transaction ID (64-char hex)
        id: String,
        #[arg(long, default_value = "127.0.0.1:8332")]
        node: String,
    },
}

pub async fn run(args: TxArgs) -> Result<()> {
    match args.command {
        TxCmd::Send {
            keyfile,
            to,
            amount,
            fee,
            node,
        } => {
            let wallet = Wallet::load_from_file(&keyfile)?;
            let mut tx = Transaction::new(wallet.address(), &to, amount, fee);
            wallet.sign_transaction(&mut tx)?;

            let tx_id = tx.id.clone();
            rpc::call(&node, "tx_submit", serde_json::json!({ "tx": tx })).await?;
            println!("submitted : {}", tx_id);
        }

        TxCmd::Get { id, node } => {
            let result = rpc::call(&node, "tx_get", serde_json::json!({ "id": id })).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    Ok(())
}
