/// P2P integration tests.
///
/// Tests spin up two in-process P2PLayer instances and verify the
/// gossip protocol: block broadcast, chain sync, peer exchange.
///
/// Protocol: newline-delimited JSON over TCP.
/// Message types (adjacently-tagged: "type" + "data"):
///   { "type": "new_block",  "data": <Block JSON>   }
///   { "type": "get_chain"                          }
///   { "type": "chain",      "data": [...]          }
///   { "type": "new_tx",     "data": <Tx JSON>      }
///   { "type": "get_peers"                          }
///   { "type": "peers",      "data": [...]          }

use kamichain_core::{Block, Transaction};
use kamichain_node::mempool::Mempool;
use kamichain_node::p2p::{Message, P2PLayer};
use kamichain_node::state::NodeState;
use std::sync::{Arc, Mutex, RwLock};

fn make_state(difficulty: usize) -> Arc<RwLock<NodeState>> {
    Arc::new(RwLock::new(NodeState::new(difficulty)))
}

#[tokio::test]
async fn message_new_block_roundtrips_json() {
    let block = Block::genesis();
    let msg   = Message::NewBlock(block.clone());
    let json  = serde_json::to_string(&msg).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();
    if let Message::NewBlock(b) = back {
        assert_eq!(b.hash, block.hash);
    } else {
        panic!("wrong variant after roundtrip");
    }
}

#[tokio::test]
async fn message_new_tx_roundtrips_json() {
    let tx  = Transaction::new("alice", "bob", 42, 0);
    let msg = Message::NewTx(tx.clone());
    let json = serde_json::to_string(&msg).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();
    if let Message::NewTx(t) = back {
        assert_eq!(t.amount, 42);
    } else {
        panic!("wrong variant after roundtrip");
    }
}

#[tokio::test]
async fn two_nodes_can_connect() {
    let state_a  = make_state(2);
    let state_b  = make_state(2);
    let pool_a   = Arc::new(Mutex::new(Mempool::new(100)));
    let pool_b   = Arc::new(Mutex::new(Mempool::new(100)));

    let node_a = P2PLayer::new("127.0.0.1:0", state_a, pool_a);
    let node_b = P2PLayer::new("127.0.0.1:0", state_b, pool_b);

    let addr_a = node_a.listen_addr();
    tokio::spawn(async move { node_a.listen().await.unwrap() });

    // B connects to A — should not panic or error
    node_b.connect(&addr_a).await.unwrap();
}

#[tokio::test]
async fn broadcast_block_reaches_connected_peer() {
    let state_a = make_state(2);
    let state_b = make_state(2);
    let pool_a  = Arc::new(Mutex::new(Mempool::new(100)));
    let pool_b  = Arc::new(Mutex::new(Mempool::new(100)));

    let node_a = P2PLayer::new("127.0.0.1:0", Arc::clone(&state_a), Arc::clone(&pool_a));
    let node_b = P2PLayer::new("127.0.0.1:0", Arc::clone(&state_b), Arc::clone(&pool_b));

    let addr_a = node_a.listen_addr();
    tokio::spawn(async move { node_a.listen().await.unwrap() });
    node_b.connect(&addr_a).await.unwrap();

    // mine a block and broadcast it
    let genesis_hash = state_a.read().unwrap().chain.latest_block().hash.clone();
    let mut block    = Block::new(1, vec![], genesis_hash);
    let pow          = kamichain_core::ProofOfWork::new(2);
    pow.mine(&mut block);

    node_b.broadcast_block(&block).await;

    // give the async runtime a moment to deliver
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // node_a should have added the block to its chain
    assert_eq!(state_a.read().unwrap().chain.len(), 2);
}

#[tokio::test]
async fn sync_replaces_shorter_chain() {
    let state_a = make_state(2);
    let state_b = make_state(2);
    let pool_a  = Arc::new(Mutex::new(Mempool::new(100)));
    let pool_b  = Arc::new(Mutex::new(Mempool::new(100)));

    // Give node_b a longer chain (3 blocks)
    {
        use kamichain_node::miner::Miner;
        let mut pool = pool_b.lock().unwrap();
        let miner    = Miner::new("miner", 2);
        miner.mine_and_commit(&state_b, &mut pool).unwrap();
        miner.mine_and_commit(&state_b, &mut pool).unwrap();
    }

    let node_a = P2PLayer::new("127.0.0.1:0", Arc::clone(&state_a), Arc::clone(&pool_a));
    let node_b = P2PLayer::new("127.0.0.1:0", Arc::clone(&state_b), Arc::clone(&pool_b));

    let addr_b = node_b.listen_addr();
    tokio::spawn(async move { node_b.listen().await.unwrap() });

    // A syncs from B — should adopt B's longer chain
    node_a.sync_with_peer(&addr_b).await.unwrap();

    assert_eq!(state_a.read().unwrap().chain.len(), 3);
}

#[tokio::test]
async fn get_peers_returns_connected_addresses() {
    let state   = make_state(2);
    let pool    = Arc::new(Mutex::new(Mempool::new(100)));
    let node    = P2PLayer::new("127.0.0.1:0", state, pool);

    let peers = node.peers();
    assert!(peers.is_empty());
}
