mod mcp;

use anyhow::Result;
use sentinel_core::client::SentinelClient;
use sentinel_core::config::FleetConfig;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    let config = FleetConfig::from_env()?;
    let client = Arc::new(SentinelClient::new(config)?);

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let error_resp = mcp::jsonrpc_error(
                    serde_json::Value::Null,
                    -32700,
                    &format!("Parse error: {e}"),
                );
                println!("{}", serde_json::to_string(&error_resp)?);
                continue;
            }
        };

        let response = mcp::handle_request(&client, &request).await;
        println!("{}", serde_json::to_string(&response)?);
    }

    Ok(())
}
