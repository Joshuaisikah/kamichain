use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use kamichain_core::Transaction;
use serde_json::{json, Value};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::rpc_client;
use crate::state::AppState;

fn client_ip(headers: &HeaderMap, connect_info: &SocketAddr) -> IpAddr {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(connect_info.ip())
}

fn err_response(status: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": msg.into() })))
}

pub async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let chain_info = match rpc_client::call(&state.rpc_addr, "chain_info", None).await {
        Ok(v) => v,
        Err(e) => return err_response(StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };
    let balance = rpc_client::call(
        &state.rpc_addr,
        "wallet_balance",
        Some(json!({ "address": state.wallet.address() })),
    )
    .await
    .unwrap_or(json!({ "balance": 0 }));

    Json(json!({
        "chain": chain_info,
        "demoWallet": {
            "address": state.wallet.address(),
            "balance": balance["balance"],
        },
    }))
    .into_response()
}

pub async fn get_block(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u64>,
) -> impl IntoResponse {
    match rpc_client::call(
        &state.rpc_addr,
        "chain_block",
        Some(json!({ "index": index })),
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match rpc_client::call(
        &state.rpc_addr,
        "wallet_balance",
        Some(json!({ "address": address })),
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_response(StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

pub async fn get_tx(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match rpc_client::call(&state.rpc_addr, "tx_get", Some(json!({ "id": id }))).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_response(StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn recent_log(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "lines": state.log.recent_lines().await }))
}

pub async fn log_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.log.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(line) => Some(Ok(Event::default().data(line))),
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Signs and submits a small real transfer from the bridge's demo wallet
/// (funded by real mined coinbase rewards) to a fixed sink address.
/// Rate-limited per IP so one visitor can't flood the mempool.
pub async fn submit_demo_tx(
    State(state): State<Arc<AppState>>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let ip = client_ip(&headers, &connect_info);

    {
        let mut last_by_ip = state.last_tx_by_ip.lock().await;
        let now = Instant::now();
        if let Some(last) = last_by_ip.get(&ip) {
            let elapsed = now.duration_since(*last);
            let limit = Duration::from_secs(state.rate_limit_secs);
            if elapsed < limit {
                let retry_after = (limit - elapsed).as_secs();
                return err_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("rate limited — try again in {}s", retry_after),
                )
                .into_response();
            }
        }
        last_by_ip.insert(ip, now);
    }

    let balance = match rpc_client::call(
        &state.rpc_addr,
        "wallet_balance",
        Some(json!({ "address": state.wallet.address() })),
    )
    .await
    {
        Ok(v) => v["balance"].as_u64().unwrap_or(0),
        Err(e) => return err_response(StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };

    const AMOUNT: u64 = 1;
    const FEE: u64 = 1;
    if balance < AMOUNT + FEE {
        return err_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "the demo wallet hasn't mined enough coins yet — try again shortly",
        )
        .into_response();
    }

    let mut tx = Transaction::new(
        state.wallet.address(),
        state.demo_recipient.clone(),
        AMOUNT,
        FEE,
    );
    if state.wallet.sign_transaction(&mut tx).is_err() {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to sign transaction",
        )
        .into_response();
    }

    let tx_id = tx.id.clone();
    match rpc_client::call(&state.rpc_addr, "tx_submit", Some(json!({ "tx": tx }))).await {
        Ok(_) => Json(json!({
            "submitted": true,
            "txId": tx_id,
            "from": state.wallet.address(),
            "to": state.demo_recipient,
            "amount": AMOUNT,
            "fee": FEE,
        }))
        .into_response(),
        Err(e) => err_response(StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

pub async fn admin_reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if provided != Some(state.admin_token.as_str()) {
        return err_response(StatusCode::UNAUTHORIZED, "invalid admin token").into_response();
    }

    let _ = state.reset_tx.send(()).await;
    Json(json!({ "reset": true })).into_response()
}
