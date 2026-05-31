use crate::mempool::Mempool;
use crate::state::NodeState;
use kamichain_core::{Block, Transaction};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    NewBlock(Block),
    GetChain,
    Chain(Vec<Block>),
    NewTx(Transaction),
    GetPeers,
    Peers(Vec<String>),
}

pub struct P2PLayer {
    listener: TcpListener,
    peers: Arc<Mutex<Vec<String>>>,
    state: Arc<RwLock<NodeState>>,
    mempool: Arc<Mutex<Mempool>>,
}

impl P2PLayer {
    pub fn new(addr: &str, state: Arc<RwLock<NodeState>>, mempool: Arc<Mutex<Mempool>>) -> Self {
        let listener = TcpListener::bind(addr).unwrap();
        listener.set_nonblocking(true).unwrap();
        P2PLayer {
            listener,
            peers: Arc::new(Mutex::new(vec![])),
            state,
            mempool,
        }
    }

    pub fn listen_addr(&self) -> String {
        self.listener.local_addr().unwrap().to_string()
    }

    pub fn peers(&self) -> Vec<String> {
        self.peers.lock().unwrap().clone()
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::from_std(self.listener.try_clone()?)?;

        loop {
            let (stream, _) = listener.accept().await?;
            let state = Arc::clone(&self.state);
            let mempool = Arc::clone(&self.mempool);
            let peers = Arc::clone(&self.peers);
            tokio::spawn(async move {
                if let Err(e) = handle_peer(stream, state, mempool, peers).await {
                    eprintln!("p2p error: {}", e);
                }
            });
        }
    }

    pub async fn connect(&self, peer_addr: &str) -> anyhow::Result<()> {
        let stream = tokio::net::TcpStream::connect(peer_addr).await?;
        self.peers.lock().unwrap().push(peer_addr.to_string());

        let state = Arc::clone(&self.state);
        let mempool = Arc::clone(&self.mempool);
        let peers = Arc::clone(&self.peers);

        tokio::spawn(async move {
            if let Err(e) = handle_peer(stream, state, mempool, peers).await {
                eprintln!("p2p connect error: {}", e);
            }
        });

        Ok(())
    }

    pub async fn broadcast_block(&self, block: &Block) {
        let msg = Message::NewBlock(block.clone());
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(&peer).await {
                let mut line = serde_json::to_string(&msg).unwrap();
                line.push('\n');
                let _ = stream.write_all(line.as_bytes()).await;
            }
        }
    }

    pub async fn broadcast_tx(&self, tx: &Transaction) {
        let msg = Message::NewTx(tx.clone());
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(&peer).await {
                let mut line = serde_json::to_string(&msg).unwrap();
                line.push('\n');
                let _ = stream.write_all(line.as_bytes()).await;
            }
        }
    }

    pub async fn sync_with_peer(&self, addr: &str) -> anyhow::Result<()> {
        let mut stream = tokio::net::TcpStream::connect(addr).await?;

        let mut line = serde_json::to_string(&Message::GetChain)?;
        line.push('\n');
        stream.write_all(line.as_bytes()).await?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        let msg: Message = serde_json::from_str(&response)?;
        if let Message::Chain(blocks) = msg {
            let mut state_w = self.state.write().unwrap();
            state_w.chain.replace(blocks);
        }

        Ok(())
    }
}

async fn handle_peer(
    stream: tokio::net::TcpStream,
    state: Arc<RwLock<NodeState>>,
    mempool: Arc<Mutex<Mempool>>,
    _peers: Arc<Mutex<Vec<String>>>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line).await?;
    if line.is_empty() {
        return Ok(());
    }

    let msg: Message = serde_json::from_str(&line)?;

    match msg {
        Message::NewBlock(block) => {
            let mut state_w = state.write().unwrap();
            if let Ok(()) = state_w.chain.add_block(block.clone()) {
                state_w.apply_block(&block);
            } else {
                // peer might be ahead — could send GetChain here
            }
        }

        Message::GetChain => {
            let blocks = state.read().unwrap().chain.blocks.clone();
            let response = Message::Chain(blocks);
            let mut resp_line = serde_json::to_string(&response)?;
            resp_line.push('\n');
            writer.write_all(resp_line.as_bytes()).await?;
        }

        Message::NewTx(tx) => {
            let sender_balance = state.read().unwrap().balance_of(&tx.sender);
            let _ = mempool.lock().unwrap().add(tx, sender_balance);
        }

        Message::GetPeers => {
            let response = Message::Peers(vec![]);
            let mut resp_line = serde_json::to_string(&response)?;
            resp_line.push('\n');
            writer.write_all(resp_line.as_bytes()).await?;
        }

        _ => {}
    }

    Ok(())
}
