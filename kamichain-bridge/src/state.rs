use crate::node_supervisor::LogHub;
use kamichain_wallet::Wallet;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

pub struct AppState {
    pub wallet: Wallet,
    pub rpc_addr: String,
    pub log: Arc<LogHub>,
    pub reset_tx: mpsc::Sender<()>,
    pub admin_token: String,
    pub demo_recipient: String,
    pub last_tx_by_ip: Mutex<HashMap<IpAddr, Instant>>,
    pub rate_limit_secs: u64,
}
