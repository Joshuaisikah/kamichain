use crate::rpc;
use anyhow::Result;
use clap::{Args, Subcommand};
use kamichain_wallet::Wallet;

#[derive(Args)]
pub struct WalletArgs {
    #[command(subcommand)]
    pub command: WalletCmd,
}

#[derive(Subcommand)]
pub enum WalletCmd {
    /// Generate a new keypair and save it to a keyfile
    New {
        /// Path to write the private key
        #[arg(long, default_value = "wallet.key")]
        keyfile: String,
    },
    /// Print the address stored in a keyfile
    Address {
        #[arg(long, default_value = "wallet.key")]
        keyfile: String,
    },
    /// Query confirmed on-chain balance for an address
    Balance {
        /// The address to check
        address: String,
        #[arg(long, default_value = "127.0.0.1:8332")]
        node: String,
    },
}

pub async fn run(args: WalletArgs) -> Result<()> {
    match args.command {
        WalletCmd::New { keyfile } => {
            let wallet = Wallet::new();
            wallet.save_to_file(&keyfile)?;
            println!("address : {}", wallet.address());
            println!("keyfile : {}", keyfile);
        }

        WalletCmd::Address { keyfile } => {
            let wallet = Wallet::load_from_file(&keyfile)?;
            println!("{}", wallet.address());
        }

        WalletCmd::Balance { address, node } => {
            let result = rpc::call(
                &node,
                "wallet_balance",
                serde_json::json!({ "address": address }),
            )
            .await?;
            println!("balance : {}", result["balance"]);
        }
    }
    Ok(())
}
