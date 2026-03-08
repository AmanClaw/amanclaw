//! MCP stdio transport — reads JSON-RPC from stdin, writes to stdout.

use crate::handler::McpHandler;
use crate::protocol::JsonRpcRequest;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Run the MCP server over stdio (stdin/stdout).
/// This is the standard transport for local MCP servers (e.g. Claude Code).
pub async fn run_stdio(handler: Arc<McpHandler>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    tracing::info!("MCP stdio server started");

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "Invalid JSON-RPC request");
                continue;
            }
        };

        tracing::debug!(method = %req.method, "MCP request");

        if let Some(resp) = handler.handle(req).await {
            let mut json = serde_json::to_string(&resp)?;
            json.push('\n');
            stdout.write_all(json.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
