mod commands;
mod rpc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{chain, node, tx, wallet};

#[derive(Parser)]
#[command(
    name = "kami",
    about = "KamiChain CLI — talk to a running node or start one",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Wallet — generate keys, print address, check balance
    Wallet(wallet::WalletArgs),
    /// Transactions — send funds, look up a tx by ID
    Tx(tx::TxArgs),
    /// Chain — info, block details, validate
    Chain(chain::ChainArgs),
    /// Node — start a kamichain-node
    Node(node::NodeArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Wallet(args) => wallet::run(args).await,
        Cmd::Tx(args) => tx::run(args).await,
        Cmd::Chain(args) => chain::run(args).await,
        Cmd::Node(args) => node::run(args).await,
    }
}
