/// RPC integration tests.
///
/// Each test starts an RpcServer bound to 127.0.0.1:0 (OS picks a free port),
/// connects via TCP, sends a newline-terminated JSON request, and asserts
/// on the JSON response.
///
/// Request format:  { "method": "<name>", "params": { ... } }
/// Response format: { "ok": true,  "result": { ... } }
///               or { "ok": false, "error": "<message>" }

use kamichain_node::rpc::RpcServer;
use kamichain_node::state::NodeState;
use kamichain_node::mempool::Mempool;
use std::sync::{Arc, Mutex, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

async fn start_server() -> (RpcServer, u16) {
    let state   = Arc::new(RwLock::new(NodeState::new(2)));
    let mempool = Arc::new(Mutex::new(Mempool::new(1000)));
    let server  = RpcServer::new("127.0.0.1:0", state, mempool);
    let port    = server.local_port();
    (server, port)
}

async fn send(port: u16, request: serde_json::Value) -> serde_json::Value {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let mut line   = serde_json::to_string(&request).unwrap();
    line.push('\n');
    stream.write_all(line.as_bytes()).await.unwrap();

    let mut reader   = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await.unwrap();
    serde_json::from_str(&response).unwrap()
}

#[tokio::test]
async fn chain_info_returns_height_and_hash() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let resp = send(port, serde_json::json!({ "method": "chain_info" })).await;
    assert_eq!(resp["ok"], true);
    assert!(resp["result"]["height"].as_u64().is_some());
    assert!(resp["result"]["latest_hash"].as_str().is_some());
    assert!(resp["result"]["difficulty"].as_u64().is_some());
}

#[tokio::test]
async fn chain_block_returns_genesis_at_index_zero() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let resp = send(port, serde_json::json!({ "method": "chain_block", "params": { "index": 0 } })).await;
    assert_eq!(resp["ok"], true);
    assert_eq!(resp["result"]["index"], 0);
}

#[tokio::test]
async fn chain_block_returns_error_for_missing_index() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let resp = send(port, serde_json::json!({ "method": "chain_block", "params": { "index": 999 } })).await;
    assert_eq!(resp["ok"], false);
    assert!(resp["error"].as_str().is_some());
}

#[tokio::test]
async fn wallet_balance_returns_zero_for_unknown_address() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let resp = send(port, serde_json::json!({ "method": "wallet_balance", "params": { "address": "unknown" } })).await;
    assert_eq!(resp["ok"], true);
    assert_eq!(resp["result"]["balance"], 0);
}

#[tokio::test]
async fn tx_submit_adds_to_mempool() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let wallet = kamichain_wallet::Wallet::new();
    let mut tx = kamichain_core::Transaction::new(wallet.address(), "bob", 10);
    wallet.sign_transaction(&mut tx).unwrap();

    let resp = send(port, serde_json::json!({ "method": "tx_submit", "params": { "tx": tx } })).await;
    assert_eq!(resp["ok"], true);
}

#[tokio::test]
async fn node_peers_returns_empty_list_initially() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let resp = send(port, serde_json::json!({ "method": "node_peers" })).await;
    assert_eq!(resp["ok"], true);
    assert_eq!(resp["result"]["peers"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn unknown_method_returns_error() {
    let (server, port) = start_server().await;
    tokio::spawn(async move { server.run().await.unwrap() });

    let resp = send(port, serde_json::json!({ "method": "not_a_real_method" })).await;
    assert_eq!(resp["ok"], false);
}
