use anyhow::{anyhow, Context};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// One-shot call against kamichain-node's line-delimited JSON-over-TCP RPC:
/// connect, write one request line, read one response line, disconnect.
/// Mirrors exactly how `RpcServer::handle` in kamichain-node is written —
/// it does not keep the connection open for more than one request.
pub async fn call(rpc_addr: &str, method: &str, params: Option<Value>) -> anyhow::Result<Value> {
    let stream = TcpStream::connect(rpc_addr)
        .await
        .with_context(|| format!("connecting to node rpc at {}", rpc_addr))?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let request = serde_json::json!({ "method": method, "params": params });
    let mut line = serde_json::to_string(&request)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: Value = serde_json::from_str(&response_line)
        .with_context(|| format!("parsing rpc response: {}", response_line))?;

    if response["ok"].as_bool() == Some(true) {
        Ok(response["result"].clone())
    } else {
        let msg = response["error"].as_str().unwrap_or("unknown rpc error");
        Err(anyhow!("{}", msg))
    }
}
