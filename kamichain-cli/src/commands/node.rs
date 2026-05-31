use anyhow::Result;
use clap::{Args, Subcommand};
use kamichain_node::mempool::Mempool;
use kamichain_node::miner::Miner;
use kamichain_node::p2p::P2PLayer;
use kamichain_node::rpc::RpcServer;
use kamichain_node::state::NodeState;
use kamichain_node::storage::Storage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Args)]
pub struct NodeArgs {
    #[command(subcommand)]
    pub command: NodeCmd,
}

#[derive(Subcommand)]
pub enum NodeCmd {
    /// Start a kamichain node (runs until killed)
    Start {
        /// P2P listen address
        #[arg(long, default_value = "127.0.0.1:8333")]
        bind: String,
        /// RPC listen address
        #[arg(long, default_value = "127.0.0.1:8332")]
        rpc: String,
        /// Proof-of-work difficulty
        #[arg(long, default_value_t = 2)]
        difficulty: usize,
        /// Directory to persist chain.json
        #[arg(long, default_value = "./data")]
        data_dir: String,
        /// Coinbase reward address
        #[arg(long, default_value = "default_miner")]
        miner: String,
        /// Bootstrap peer to sync from on startup
        #[arg(long)]
        peer: Option<String>,
    },
}

pub async fn run(args: NodeArgs) -> Result<()> {
    let NodeCmd::Start {
        bind,
        rpc,
        difficulty,
        data_dir,
        miner: miner_addr,
        peer,
    } = args.command;

    println!("KamiChain Node");
    println!("  bind:       {}", bind);
    println!("  rpc:        {}", rpc);
    println!("  difficulty: {}", difficulty);
    println!("  data dir:   {}", data_dir);
    println!("  miner:      {}", miner_addr);

    std::fs::create_dir_all(&data_dir)?;

    let storage = Storage::new(format!("{}/chain.json", data_dir));
    let chain = storage.load_chain().unwrap_or_else(|_| {
        println!("No existing chain — starting fresh");
        kamichain_core::Chain::new(difficulty)
    });

    let state = Arc::new(RwLock::new(NodeState {
        chain,
        balances: HashMap::new(),
    }));
    let mempool = Arc::new(Mutex::new(Mempool::new(10_000)));

    let rpc_server = RpcServer::new(&rpc, Arc::clone(&state), Arc::clone(&mempool));
    println!("RPC listening on {}", rpc);
    tokio::spawn(async move { rpc_server.run().await.unwrap() });

    let p2p = P2PLayer::new(&bind, Arc::clone(&state), Arc::clone(&mempool));
    println!("P2P listening on {}", bind);

    if let Some(ref peer_addr) = peer {
        println!("Connecting to peer {}", peer_addr);
        p2p.connect(peer_addr).await?;
        p2p.sync_with_peer(peer_addr).await.unwrap_or_else(|e| {
            eprintln!("Sync failed: {}", e);
        });
    }

    let p2p = Arc::new(p2p);
    let p2p_listener = Arc::clone(&p2p);
    tokio::spawn(async move { p2p_listener.listen().await.unwrap() });

    let miner = Miner::new(&miner_addr, difficulty);
    println!("Mining started...");

    loop {
        let result = {
            let mut pool = mempool.lock().unwrap();
            miner.mine_and_commit(&state, &mut pool)
        };
        match result {
            Ok(block) => {
                println!("Mined block {} — hash: {}", block.index, &block.hash[..8]);
                let chain = state.read().unwrap().chain.clone();
                if let Err(e) = storage.save_chain(&chain) {
                    eprintln!("Save failed: {}", e);
                }
                p2p.broadcast_block(&block).await;
            }
            Err(e) => {
                eprintln!("Mining error: {}", e);
            }
        }
    }
}
