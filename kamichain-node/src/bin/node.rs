use kamichain_node::config::NodeConfig;
use kamichain_node::mempool::Mempool;
use kamichain_node::miner::Miner;
use kamichain_node::p2p::P2PLayer;
use kamichain_node::rpc::RpcServer;
use kamichain_node::state::NodeState;
use kamichain_node::storage::Storage;
use std::sync::{Arc, Mutex, RwLock};

#[tokio::main]
async fn main() {
    let cfg = NodeConfig::from_args().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    println!("KamiChain Node");
    println!("  bind:       {}", cfg.bind_addr);
    println!("  rpc:        {}", cfg.rpc_addr);
    println!("  difficulty: {}", cfg.difficulty);
    println!("  data dir:   {}", cfg.data_dir);
    println!("  miner:      {}", cfg.miner_addr);

    std::fs::create_dir_all(&cfg.data_dir).unwrap();

    let storage = Storage::new(cfg.chain_path());
    let chain = storage.load_chain().unwrap_or_else(|_| {
        println!("No existing chain found — starting fresh");
        kamichain_core::Chain::new(cfg.difficulty)
    });

    let state = Arc::new(RwLock::new(NodeState {
        chain,
        balances: std::collections::HashMap::new(),
    }));

    let mempool = Arc::new(Mutex::new(Mempool::new(10_000)));

    let rpc = RpcServer::new(&cfg.rpc_addr, Arc::clone(&state), Arc::clone(&mempool));
    println!("RPC listening on {}", cfg.rpc_addr);
    tokio::spawn(async move {
        rpc.run().await.unwrap();
    });

    let p2p = P2PLayer::new(&cfg.bind_addr, Arc::clone(&state), Arc::clone(&mempool));
    println!("P2P listening on {}", cfg.bind_addr);

    if let Some(ref peer_addr) = cfg.peer {
        println!("Connecting to peer {}", peer_addr);
        p2p.connect(peer_addr).await.unwrap();
        p2p.sync_with_peer(peer_addr).await.unwrap_or_else(|e| {
            eprintln!("Sync failed: {}", e);
        });
    }

    let p2p = Arc::new(p2p);
    let p2p_listener = Arc::clone(&p2p);
    tokio::spawn(async move {
        p2p_listener.listen().await.unwrap();
    });

    let miner = Miner::new(&cfg.miner_addr, cfg.difficulty);
    println!("Mining started...");

    loop {
        let mut mempool_guard = mempool.lock().unwrap();
        match miner.mine_and_commit(&state, &mut mempool_guard) {
            Ok(block) => {
                drop(mempool_guard);
                println!("Mined block {} — hash: {}", block.index, &block.hash[..8]);

                let chain = state.read().unwrap().chain.clone();
                if let Err(e) = storage.save_chain(&chain) {
                    eprintln!("Failed to save chain: {}", e);
                }

                p2p.broadcast_block(&block).await;
            }
            Err(e) => {
                drop(mempool_guard);
                eprintln!("Mining error: {}", e);
            }
        }
    }
}
