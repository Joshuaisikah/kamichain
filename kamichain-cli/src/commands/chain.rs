use crate::rpc;
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ChainArgs {
    #[command(subcommand)]
    pub command: ChainCmd,
}

#[derive(Subcommand)]
pub enum ChainCmd {
    /// Print chain height, latest hash, and difficulty
    Info {
        #[arg(long, default_value = "127.0.0.1:8332")]
        node: String,
    },
    /// Print full block details by index
    Block {
        /// Block index (0 = genesis)
        index: u64,
        #[arg(long, default_value = "127.0.0.1:8332")]
        node: String,
    },
    /// Ask the node to validate its full chain
    Validate {
        #[arg(long, default_value = "127.0.0.1:8332")]
        node: String,
    },
}

pub async fn run(args: ChainArgs) -> Result<()> {
    match args.command {
        ChainCmd::Info { node } => {
            let result = rpc::call(&node, "chain_info", serde_json::Value::Null).await?;
            println!("height      : {}", result["height"]);
            println!("latest hash : {}", result["latest_hash"]);
            println!("difficulty  : {}", result["difficulty"]);
        }

        ChainCmd::Block { index, node } => {
            let result =
                rpc::call(&node, "chain_block", serde_json::json!({ "index": index })).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        ChainCmd::Validate { node } => {
            let result = rpc::call(&node, "chain_validate", serde_json::Value::Null).await?;
            let valid = result["valid"].as_bool().unwrap_or(false);
            let msg = result["message"].as_str().unwrap_or("");
            if valid {
                println!("✓ {}", msg);
            } else {
                println!("✗ {}", msg);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
