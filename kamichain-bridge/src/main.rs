mod node_supervisor;
mod routes;
mod rpc_client;
mod state;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use kamichain_wallet::Wallet;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{AllowOrigin, CorsLayer};

use node_supervisor::{LogHub, NodeConfig};
use state::AppState;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// A stable, arbitrary sink address for demo transfers. Doesn't need a real
/// keypair — it only ever appears as a recipient, never signs anything.
fn demo_recipient_address() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"kamichain-demo-faucet-sink");
    format!("{:x}", hasher.finalize())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bridge_port: u16 = env_or("BRIDGE_PORT", "8080").parse()?;
    let rpc_addr = env_or("NODE_RPC_ADDR", "127.0.0.1:8332");
    let bind_addr = env_or("NODE_BIND_ADDR", "127.0.0.1:8333");
    // Calibrated against 1 CPU core (the Docker deploy caps the container to
    // one core): difficulty 5 averages ~1-3s per block — fast enough to feel
    // alive in a live log, slow enough not to peg the whole box mining PoW.
    let difficulty = env_or("NODE_DIFFICULTY", "5");
    let data_dir = env_or("NODE_DATA_DIR", "/data/kamichain");
    let node_bin_path = env_or("NODE_BIN_PATH", "kamichain-node");
    let reset_interval_hours: u64 = env_or("RESET_INTERVAL_HOURS", "6").parse()?;
    let rate_limit_secs: u64 = env_or("DEMO_TX_COOLDOWN_SECS", "20").parse()?;
    let frontend_origin = env_or("FRONTEND_ORIGIN", "https://joshuaisikah.github.io");

    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_else(|_| {
        let generated = uuid_like_token();
        eprintln!(
            "[bridge] ADMIN_TOKEN not set — generated one for this run: {}",
            generated
        );
        generated
    });

    // The bridge's own wallet doubles as the node's configured miner address,
    // so it accumulates real mined coinbase rewards to send demo transfers from.
    let wallet = Wallet::new();
    let miner_address = wallet.address();
    println!("[bridge] demo wallet address (miner): {}", miner_address);

    let log = LogHub::new();
    let (reset_tx, reset_rx) = mpsc::channel(4);

    let node_cfg = NodeConfig {
        node_bin_path,
        rpc_addr: rpc_addr.clone(),
        bind_addr,
        difficulty,
        data_dir,
        miner_address,
    };

    tokio::spawn(node_supervisor::run(node_cfg, Arc::clone(&log), reset_rx));

    {
        let reset_tx = reset_tx.clone();
        let interval = std::time::Duration::from_secs(reset_interval_hours.max(1) * 3600);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let _ = reset_tx.send(()).await;
            }
        });
    }

    let app_state = Arc::new(AppState {
        wallet,
        rpc_addr,
        log,
        reset_tx,
        admin_token,
        demo_recipient: demo_recipient_address(),
        last_tx_by_ip: Mutex::new(HashMap::new()),
        rate_limit_secs,
    });

    // Vite auto-increments its port when the default is taken, so hardcoding
    // one dev port keeps breaking. Allow the exact production origin, plus
    // any localhost/127.0.0.1 port for local dev.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _| {
            let Ok(origin_str) = origin.to_str() else {
                return false;
            };
            origin_str == frontend_origin
                || origin_str.starts_with("http://localhost:")
                || origin_str.starts_with("http://127.0.0.1:")
        }))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    let app = Router::new()
        .route("/api/status", get(routes::status))
        .route("/api/block/{index}", get(routes::get_block))
        .route("/api/balance/{address}", get(routes::get_balance))
        .route("/api/tx/{id}", get(routes::get_tx))
        .route("/api/validate", get(routes::validate_chain))
        .route("/api/log/recent", get(routes::recent_log))
        .route("/api/log/stream", get(routes::log_stream))
        .route("/api/demo-tx", post(routes::submit_demo_tx))
        .route("/api/admin/reset", post(routes::admin_reset))
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], bridge_port));
    println!("[bridge] listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn uuid_like_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect()
}
