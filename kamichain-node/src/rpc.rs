
// kamichain-node/src/rpc.rs
use std::net::TcpListener;
use std::sync::{Arc, Mutex, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use kamichain_core::Transaction;
use crate::mempool::Mempool;
use crate::state::NodeState;

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl RpcResponse {
    fn ok(result: Value) -> Self {
        RpcResponse { ok: true, result: Some(result), error: None }
    }

    fn err(msg: impl Into<String>) -> Self {
        RpcResponse { ok: false, result: None, error: Some(msg.into()) }
    }
}

pub struct RpcServer {
    listener: TcpListener,
    state: Arc<RwLock<NodeState>>,
    mempool: Arc<Mutex<Mempool>>,
}

impl RpcServer {
    pub fn new(
        addr: &str,
        state: Arc<RwLock<NodeState>>,
        mempool: Arc<Mutex<Mempool>>,
    ) -> Self {
        let listener = TcpListener::bind(addr).unwrap();
        listener.set_nonblocking(true).unwrap();
        RpcServer { listener, state, mempool }
    }

    pub fn local_port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::from_std(self.listener)?;
        let state = self.state;
        let mempool = self.mempool;

        loop {
            let (stream, _) = listener.accept().await?;
            let state = Arc::clone(&state);
            let mempool = Arc::clone(&mempool);
            tokio::spawn(async move {
                if let Err(e) = handle(stream, state, mempool).await {
                    eprintln!("rpc error: {}", e);
                }
            });
        }
    }
}

async fn handle(
    stream: tokio::net::TcpStream,
    state: Arc<RwLock<NodeState>>,
    mempool: Arc<Mutex<Mempool>>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response = match serde_json::from_str::<RpcRequest>(&line) {
        Err(_) => RpcResponse::err("invalid request"),
        Ok(req) => dispatch(req, &state, &mempool),
    };

    let mut resp_line = serde_json::to_string(&response)?;
    resp_line.push('\n');
    writer.write_all(resp_line.as_bytes()).await?;

    Ok(())
}

fn dispatch(
    req: RpcRequest,
    state: &Arc<RwLock<NodeState>>,
    mempool: &Arc<Mutex<Mempool>>,
) -> RpcResponse {
    match req.method.as_str() {
        "chain_info" => {
            let s = state.read().unwrap();
            RpcResponse::ok(serde_json::json!({
                "height": s.chain.len(),
                "latest_hash": s.chain.latest_block().hash,
                "difficulty": s.chain.difficulty,
            }))
        }

        "chain_block" => {
            let index = req.params
                .as_ref()
                .and_then(|p| p["index"].as_u64())
                .unwrap_or(0);
            let s = state.read().unwrap();
            match s.chain.get_block(index as usize) {
                Some(block) => RpcResponse::ok(serde_json::to_value(block).unwrap()),
                None => RpcResponse::err(format!("block {} not found", index)),
            }
        }

        "tx_submit" => {
            let tx: Transaction = match req.params
                .as_ref()
                .and_then(|p| p.get("tx"))
                .and_then(|t| serde_json::from_value(t.clone()).ok())
            {
                Some(tx) => tx,
                None => return RpcResponse::err("invalid transaction"),
            };
            let sender_balance = state.read().unwrap().balance_of(&tx.sender);
            match mempool.lock().unwrap().add(tx, sender_balance) {
                Ok(_) => RpcResponse::ok(serde_json::json!({ "submitted": true })),
                Err(e) => RpcResponse::err(e.to_string()),
            }
        }

        "wallet_balance" => {
            let address = req.params
                .as_ref()
                .and_then(|p| p["address"].as_str())
                .unwrap_or("");
            let s = state.read().unwrap();
            RpcResponse::ok(serde_json::json!({
                "balance": s.balance_of(address)
            }))
        }

        "chain_validate" => {
            let s = state.read().unwrap();
            match s.chain.is_valid() {
                Ok(()) => RpcResponse::ok(serde_json::json!({
                    "valid": true,
                    "message": "chain is valid"
                })),
                Err(e) => RpcResponse::ok(serde_json::json!({
                    "valid": false,
                    "message": e.to_string()
                })),
            }
        }

        "tx_get" => {
            let id = req.params
                .as_ref()
                .and_then(|p| p["id"].as_str())
                .unwrap_or("");
            let s = state.read().unwrap();
            let found = s.chain.blocks.iter()
                .flat_map(|b| b.transactions.iter())
                .find(|tx| tx.id == id);
            match found {
                Some(tx) => RpcResponse::ok(serde_json::to_value(tx).unwrap()),
                None => RpcResponse::err(format!("transaction {} not found", id)),
            }
        }

        "node_peers" => {
            RpcResponse::ok(serde_json::json!({ "peers": [] }))
        }

        _ => RpcResponse::err("unknown method"),
    }
}