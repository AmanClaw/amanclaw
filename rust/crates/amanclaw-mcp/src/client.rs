//! MCP Client — connects to external MCP servers and discovers their tools.
//!
//! Supports two transports:
//! - **Stdio**: Spawns a child process, communicates via stdin/stdout JSON-RPC
//! - **HTTP**: Sends JSON-RPC POST requests to a remote URL

use crate::protocol::JsonRpcResponse;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Discovered tool from an external MCP server.
#[derive(Debug, Clone)]
pub struct McpRemoteTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Transport-agnostic MCP client connection.
#[allow(clippy::large_enum_variant)]
enum Transport {
    Stdio {
        child: Child,
        stdin: Mutex<tokio::process::ChildStdin>,
        stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    },
    Http {
        url: String,
        client: reqwest::Client,
    },
}

/// MCP client that connects to a single external MCP server.
pub struct McpClient {
    server_name: String,
    transport: Transport,
    request_id: AtomicU64,
}

impl McpClient {
    /// Connect to an MCP server via stdio (spawn a child process).
    pub async fn connect_stdio(
        server_name: &str,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // On macOS, GUI apps don't inherit the full shell PATH.
        // Ensure common tool directories are in PATH so npx/uvx/node can be found.
        {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let mut extra: Vec<String> = Vec::new();

            let standard = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"];
            for p in &standard {
                if !current_path.contains(p) {
                    extra.push(p.to_string());
                }
            }

            // Check for nvm/volta/fnm node installations
            if let Ok(home) = std::env::var("HOME") {
                let home_dirs = [format!("{home}/.local/bin"), format!("{home}/.volta/bin")];
                for dir in &home_dirs {
                    if std::path::Path::new(dir).is_dir() && !current_path.contains(dir.as_str()) {
                        extra.push(dir.clone());
                    }
                }
                // nvm: find latest node version
                let nvm_dir = format!("{home}/.nvm/versions/node");
                if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
                    let mut versions: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .collect();
                    versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
                    if let Some(latest) = versions.first() {
                        let bin = latest.path().join("bin").to_string_lossy().to_string();
                        if !current_path.contains(&bin) {
                            extra.push(bin);
                        }
                    }
                }
            }

            if !extra.is_empty() {
                let new_path = format!("{}:{}", current_path, extra.join(":"));
                cmd.env("PATH", new_path);
                tracing::debug!(server = %server_name, path = %extra.join(":"), "Extended PATH for MCP server");
            }
        }

        // Set environment variables, resolving ${VAR} from process env
        for (key, value) in env {
            let resolved = if value.starts_with("${") && value.ends_with('}') {
                let env_key = &value[2..value.len() - 1];
                std::env::var(env_key).unwrap_or_default()
            } else {
                value.clone()
            };
            cmd.env(key, resolved);
        }

        let mut child = cmd.spawn().with_context(|| {
            format!("Failed to spawn MCP server '{server_name}': {command} {args:?}")
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin for MCP server '{server_name}'"))?;
        let stdout = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!("Failed to get stdout for MCP server '{server_name}'")
        })?;

        tracing::info!(server = %server_name, command = %command, "Spawned MCP server process");

        Ok(Self {
            server_name: server_name.to_string(),
            transport: Transport::Stdio {
                child,
                stdin: Mutex::new(stdin),
                stdout: Mutex::new(BufReader::new(stdout)),
            },
            request_id: AtomicU64::new(1),
        })
    }

    /// Connect to an MCP server via HTTP.
    pub fn connect_http(server_name: &str, url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        tracing::info!(server = %server_name, url = %url, "Connected to remote MCP server");

        Ok(Self {
            server_name: server_name.to_string(),
            transport: Transport::Http {
                url: url.to_string(),
                client,
            },
            request_id: AtomicU64::new(1),
        })
    }

    fn next_id(&self) -> Value {
        Value::Number(self.request_id.fetch_add(1, Ordering::Relaxed).into())
    }

    /// Send a JSON-RPC request and get the response.
    async fn call(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(Value::Object(Default::default())),
        });

        match &self.transport {
            Transport::Stdio { stdin, stdout, .. } => {
                self.call_stdio(stdin, stdout, &request).await
            }
            Transport::Http { url, client } => self.call_http(client, url, &request).await,
        }
    }

    async fn call_stdio(
        &self,
        stdin: &Mutex<tokio::process::ChildStdin>,
        stdout: &Mutex<BufReader<tokio::process::ChildStdout>>,
        request: &Value,
    ) -> Result<Value> {
        let mut json = serde_json::to_string(request)?;
        json.push('\n');

        {
            let mut writer = stdin.lock().await;
            writer.write_all(json.as_bytes()).await?;
            writer.flush().await?;
        }

        let mut reader = stdout.lock().await;
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

        if let Some(error) = resp.error {
            anyhow::bail!(
                "MCP server '{}' error: {} (code {})",
                self.server_name,
                error.message,
                error.code
            );
        }

        resp.result
            .ok_or_else(|| anyhow::anyhow!("Empty result from MCP server '{}'", self.server_name))
    }

    async fn call_http(
        &self,
        client: &reqwest::Client,
        url: &str,
        request: &Value,
    ) -> Result<Value> {
        let resp = client
            .post(url)
            .json(request)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to connect to MCP server '{}' at {}",
                    self.server_name, url
                )
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "MCP server '{}' HTTP error {}: {}",
                self.server_name,
                status,
                body
            );
        }

        let rpc_resp: JsonRpcResponse = resp.json().await?;

        if let Some(error) = rpc_resp.error {
            anyhow::bail!(
                "MCP server '{}' error: {} (code {})",
                self.server_name,
                error.message,
                error.code
            );
        }

        rpc_resp
            .result
            .ok_or_else(|| anyhow::anyhow!("Empty result from MCP server '{}'", self.server_name))
    }

    /// Send initialize handshake.
    pub async fn initialize(&self) -> Result<Value> {
        self.call(
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "amanclaw",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        )
        .await
    }

    /// Send initialized notification (no response expected for stdio).
    pub async fn send_initialized(&self) -> Result<()> {
        if let Transport::Stdio { stdin, .. } = &self.transport {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "initialized"
            });
            let mut json = serde_json::to_string(&notification)?;
            json.push('\n');
            let mut writer = stdin.lock().await;
            writer.write_all(json.as_bytes()).await?;
            writer.flush().await?;
        }
        Ok(())
    }

    /// Discover available tools from the MCP server.
    pub async fn list_tools(&self) -> Result<Vec<McpRemoteTool>> {
        let result = self.call("tools/list", None).await?;

        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        let mut discovered = Vec::new();
        for tool in tools {
            let name = tool
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let description = tool
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let input_schema = tool
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

            if !name.is_empty() {
                discovered.push(McpRemoteTool {
                    name,
                    description,
                    input_schema,
                });
            }
        }

        tracing::info!(
            server = %self.server_name,
            count = discovered.len(),
            "Discovered MCP tools"
        );

        Ok(discovered)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<String> {
        let result = self
            .call(
                "tools/call",
                Some(serde_json::json!({
                    "name": tool_name,
                    "arguments": arguments,
                })),
            )
            .await?;

        // Extract text from content array
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        let text: Vec<String> = content
            .iter()
            .filter_map(|c| c.get("text").and_then(|t| t.as_str()).map(String::from))
            .collect();

        let is_error = result
            .get("isError")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);

        if is_error {
            anyhow::bail!("Tool '{}' error: {}", tool_name, text.join("\n"));
        }

        Ok(text.join("\n"))
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Transport::Stdio { ref mut child, .. } = self.transport {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_http() {
        let client = McpClient::connect_http("test", "http://localhost:9999/mcp");
        assert!(client.is_ok());
        assert_eq!(client.unwrap().server_name(), "test");
    }

    #[test]
    fn test_env_var_resolution() {
        // SAFETY: test-only, single-threaded context
        unsafe { std::env::set_var("TEST_MCP_VAR", "resolved_value") };
        let value = "${TEST_MCP_VAR}";
        let resolved = if value.starts_with("${") && value.ends_with('}') {
            let env_key = &value[2..value.len() - 1];
            std::env::var(env_key).unwrap_or_default()
        } else {
            value.to_string()
        };
        assert_eq!(resolved, "resolved_value");
        unsafe { std::env::remove_var("TEST_MCP_VAR") };
    }
}
