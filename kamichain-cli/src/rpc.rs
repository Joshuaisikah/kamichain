use anyhow::{bail, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub async fn call(addr: &str, method: &str, params: Value) -> Result<Value> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = stream.into_split();

    let mut line = serde_json::to_string(&serde_json::json!({
        "method": method,
        "params": params,
    }))?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    let mut reader   = BufReader::new(reader);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    let v: Value = serde_json::from_str(response.trim())?;

    if v["ok"].as_bool() != Some(true) {
        let msg = v["error"].as_str().unwrap_or("node returned an error");
        bail!("{}", msg);
    }

    Ok(v["result"].clone())
}
