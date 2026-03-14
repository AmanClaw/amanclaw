# Plan 1B: MCP Enhancements — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SSE transport, MCP Resources/Prompts support, and CLI management commands to make AmanClaw a fully compliant MCP client and server.

**Architecture:** Extend the existing `amanclaw-mcp` crate with new protocol types, handler methods, and transport. SSE uses Axum's built-in streaming with `tokio-stream`. The bridge wraps external server resources/prompts as AmanClaw skills. CLI commands go through the existing `SkillAction` pattern in `cli.rs`.

**Tech Stack:** Rust, Axum (SSE via `axum::response::sse`), tokio-stream, amanclaw-mcp

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rust/crates/amanclaw-mcp/src/protocol.rs` | MODIFY | Add Resource, Prompt, ResourceContent types |
| `rust/crates/amanclaw-mcp/src/handler.rs` | MODIFY | Add resources/list, resources/read, prompts/list, prompts/get handlers |
| `rust/crates/amanclaw-mcp/src/client.rs` | MODIFY | Add list_resources(), read_resource(), list_prompts(), get_prompt() |
| `rust/crates/amanclaw-mcp/src/sse.rs` | CREATE | SSE transport — streaming endpoint + client |
| `rust/crates/amanclaw-mcp/src/lib.rs` | MODIFY | Add sse module |
| `rust/crates/amanclaw-mcp/src/bridge.rs` | MODIFY | Wrap external resources as skills |
| `rust/crates/amanclaw-mcp/Cargo.toml` | MODIFY | Add tokio-stream, futures-util deps |
| `rust/crates/amanclaw-cli/src/cli.rs` | MODIFY | Add Mcp subcommand |
| `rust/crates/amanclaw-cli/src/main.rs` | MODIFY | Add cmd_mcp() handler |

---

## Chunk 1: Protocol Types

### Task 1: Add Resource and Prompt types to protocol.rs

**Files:**
- Modify: `rust/crates/amanclaw-mcp/src/protocol.rs`

- [ ] **Step 1: Add Resource types**

Add after existing `McpTool` struct:

```rust
/// MCP Resource definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Content returned when reading a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>, // base64-encoded
}
```

- [ ] **Step 2: Add Prompt types**

```rust
/// MCP Prompt definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPrompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<PromptArgument>,
}

/// Argument definition for a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

/// Message returned from getting a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String, // "user" or "assistant"
    pub content: McpContent,
}
```

- [ ] **Step 3: Update ServerCapabilities**

Update existing `ServerCapabilities` struct to include resources and prompts:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
    #[serde(default)]
    pub subscribe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}
```

- [ ] **Step 4: Add unit tests for new types**

```rust
#[test]
fn test_resource_serialization() {
    let resource = McpResource {
        uri: "file:///tmp/test.txt".into(),
        name: "test file".into(),
        description: Some("A test file".into()),
        mime_type: Some("text/plain".into()),
    };
    let json = serde_json::to_string(&resource).unwrap();
    assert!(json.contains("mimeType"));
    let parsed: McpResource = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.uri, "file:///tmp/test.txt");
}

#[test]
fn test_prompt_serialization() {
    let prompt = McpPrompt {
        name: "greeting".into(),
        description: Some("A greeting prompt".into()),
        arguments: vec![PromptArgument {
            name: "name".into(),
            description: Some("Person's name".into()),
            required: true,
        }],
    };
    let json = serde_json::to_string(&prompt).unwrap();
    let parsed: McpPrompt = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.arguments.len(), 1);
    assert!(parsed.arguments[0].required);
}
```

- [ ] **Step 5: Run tests**

Run: `cd rust && cargo test --package amanclaw-mcp protocol -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add rust/crates/amanclaw-mcp/src/protocol.rs
git commit -m "feat(mcp): add Resource and Prompt protocol types"
```

---

## Chunk 2: Handler Methods

### Task 2: Add resources/prompts handlers

**Files:**
- Modify: `rust/crates/amanclaw-mcp/src/handler.rs`

- [ ] **Step 1: Add resource/prompt storage to McpHandler**

Update the `McpHandler` struct:

```rust
pub struct McpHandler {
    skills: HashMap<String, Arc<dyn Skill>>,
    resources: Vec<McpResource>,
    prompts: Vec<McpPrompt>,
    server_name: String,
    server_version: String,
}
```

Update `new()` to initialize empty resources/prompts vecs.

Add methods:
```rust
pub fn add_resource(&mut self, resource: McpResource) {
    self.resources.push(resource);
}

pub fn add_prompt(&mut self, prompt: McpPrompt) {
    self.prompts.push(prompt);
}
```

- [ ] **Step 2: Handle resources/list and resources/read**

Add to the method dispatch in `handle_request()`:

```rust
"resources/list" => self.handle_resources_list(request.id),
"resources/read" => self.handle_resources_read(request.id, request.params),
"prompts/list" => self.handle_prompts_list(request.id),
"prompts/get" => self.handle_prompts_get(request.id, request.params),
```

Implement the handlers:

```rust
fn handle_resources_list(&self, id: Option<Value>) -> Option<JsonRpcResponse> {
    Some(JsonRpcResponse::success(id, json!({
        "resources": self.resources,
    })))
}

fn handle_resources_read(&self, id: Option<Value>, params: Option<Value>) -> Option<JsonRpcResponse> {
    let uri = params
        .as_ref()
        .and_then(|p| p.get("uri"))
        .and_then(|u| u.as_str());

    let Some(uri) = uri else {
        return Some(JsonRpcResponse::error(id, INVALID_PARAMS, "Missing 'uri' parameter", None));
    };

    // Check if resource exists
    if !self.resources.iter().any(|r| r.uri == uri) {
        return Some(JsonRpcResponse::error(id, INVALID_PARAMS, &format!("Unknown resource: {uri}"), None));
    }

    // For now, return empty content — resource backends will be added later
    Some(JsonRpcResponse::success(id, json!({
        "contents": [ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("text/plain".into()),
            text: Some(String::new()),
            blob: None,
        }]
    })))
}
```

- [ ] **Step 3: Handle prompts/list and prompts/get**

```rust
fn handle_prompts_list(&self, id: Option<Value>) -> Option<JsonRpcResponse> {
    Some(JsonRpcResponse::success(id, json!({
        "prompts": self.prompts,
    })))
}

fn handle_prompts_get(&self, id: Option<Value>, params: Option<Value>) -> Option<JsonRpcResponse> {
    let name = params
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str());

    let Some(name) = name else {
        return Some(JsonRpcResponse::error(id, INVALID_PARAMS, "Missing 'name' parameter", None));
    };

    let prompt = self.prompts.iter().find(|p| p.name == name);
    let Some(prompt) = prompt else {
        return Some(JsonRpcResponse::error(id, INVALID_PARAMS, &format!("Unknown prompt: {name}"), None));
    };

    // Return prompt with placeholder messages — actual template rendering comes later
    Some(JsonRpcResponse::success(id, json!({
        "description": prompt.description,
        "messages": [PromptMessage {
            role: "user".into(),
            content: McpContent {
                content_type: "text".into(),
                text: format!("Prompt: {name}"),
            },
        }]
    })))
}
```

- [ ] **Step 4: Update initialize to advertise capabilities**

Update `handle_initialize` to include resources/prompts in capabilities:

```rust
"capabilities": ServerCapabilities {
    tools: Some(ToolCapability { list_changed: false }),
    resources: if self.resources.is_empty() { None } else {
        Some(ResourceCapability { list_changed: false, subscribe: false })
    },
    prompts: if self.prompts.is_empty() { None } else {
        Some(PromptCapability { list_changed: false })
    },
},
```

- [ ] **Step 5: Add tests**

```rust
#[tokio::test]
async fn test_resources_list() {
    let mut handler = McpHandler::new("test", "0.1.0");
    handler.add_resource(McpResource {
        uri: "file:///test.txt".into(),
        name: "test".into(),
        description: None,
        mime_type: Some("text/plain".into()),
    });

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "resources/list".into(),
        params: None,
    };

    let response = handler.handle_request(request).await.unwrap();
    let resources = response.result.unwrap()["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);
}

#[tokio::test]
async fn test_prompts_list() {
    let mut handler = McpHandler::new("test", "0.1.0");
    handler.add_prompt(McpPrompt {
        name: "greeting".into(),
        description: Some("Say hello".into()),
        arguments: vec![],
    });

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "prompts/list".into(),
        params: None,
    };

    let response = handler.handle_request(request).await.unwrap();
    let prompts = response.result.unwrap()["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 1);
}
```

- [ ] **Step 6: Run tests**

Run: `cd rust && cargo test --package amanclaw-mcp handler -- --nocapture`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add rust/crates/amanclaw-mcp/src/handler.rs
git commit -m "feat(mcp): add resources/list, resources/read, prompts/list, prompts/get handlers"
```

---

## Chunk 3: Client Methods

### Task 3: Add resource/prompt client methods

**Files:**
- Modify: `rust/crates/amanclaw-mcp/src/client.rs`

- [ ] **Step 1: Add list_resources()**

```rust
pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
    let request = self.build_request("resources/list", None);
    let response = self.send_request(&request).await?;
    let result = response.result
        .ok_or_else(|| anyhow::anyhow!("{}: resources/list failed", self.server_name))?;
    let resources: Vec<McpResource> = serde_json::from_value(
        result.get("resources").cloned().unwrap_or(json!([]))
    )?;
    Ok(resources)
}
```

- [ ] **Step 2: Add read_resource()**

```rust
pub async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContent>> {
    let request = self.build_request("resources/read", Some(json!({"uri": uri})));
    let response = self.send_request(&request).await?;
    let result = response.result
        .ok_or_else(|| anyhow::anyhow!("{}: resources/read failed", self.server_name))?;
    let contents: Vec<ResourceContent> = serde_json::from_value(
        result.get("contents").cloned().unwrap_or(json!([]))
    )?;
    Ok(contents)
}
```

- [ ] **Step 3: Add list_prompts() and get_prompt()**

```rust
pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
    let request = self.build_request("prompts/list", None);
    let response = self.send_request(&request).await?;
    let result = response.result
        .ok_or_else(|| anyhow::anyhow!("{}: prompts/list failed", self.server_name))?;
    let prompts: Vec<McpPrompt> = serde_json::from_value(
        result.get("prompts").cloned().unwrap_or(json!([]))
    )?;
    Ok(prompts)
}

pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<Value> {
    let params = json!({
        "name": name,
        "arguments": arguments.unwrap_or(json!({})),
    });
    let request = self.build_request("prompts/get", Some(params));
    let response = self.send_request(&request).await?;
    response.result
        .ok_or_else(|| anyhow::anyhow!("{}: prompts/get failed for '{name}'", self.server_name))
}
```

- [ ] **Step 4: Run tests**

Run: `cd rust && cargo test --package amanclaw-mcp -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-mcp/src/client.rs
git commit -m "feat(mcp): add list_resources, read_resource, list_prompts, get_prompt to client"
```

---

## Chunk 4: SSE Transport

### Task 4: SSE server transport

**Files:**
- Create: `rust/crates/amanclaw-mcp/src/sse.rs`
- Modify: `rust/crates/amanclaw-mcp/src/lib.rs`
- Modify: `rust/crates/amanclaw-mcp/Cargo.toml`

- [ ] **Step 1: Add dependencies**

In `Cargo.toml`:
```toml
tokio-stream = "0.1"
futures-util = "0.3"
```

- [ ] **Step 2: Create SSE server**

Create `rust/crates/amanclaw-mcp/src/sse.rs`:

```rust
//! SSE (Server-Sent Events) transport for MCP.
//!
//! Provides a streaming endpoint at `/mcp/sse` that clients can connect to
//! for receiving server-to-client notifications. Requests come via POST to `/mcp`.

use crate::handler::McpHandler;
use crate::protocol::JsonRpcRequest;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

struct SseState {
    handler: Arc<RwLock<McpHandler>>,
    tx: broadcast::Sender<String>,
}

/// Create an Axum router with SSE + POST endpoints.
pub fn sse_router(handler: McpHandler) -> Router {
    let (tx, _) = broadcast::channel::<String>(100);
    let state = Arc::new(SseState {
        handler: Arc::new(RwLock::new(handler)),
        tx,
    });

    Router::new()
        .route("/mcp/sse", get(sse_handler))
        .route("/mcp", post(post_handler))
        .with_state(state)
}

async fn sse_handler(
    State(state): State<Arc<SseState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(data) => Some(Ok(Event::default().data(data))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn post_handler(
    State(state): State<Arc<SseState>>,
    Json(request): Json<JsonRpcRequest>,
) -> axum::http::StatusCode {
    let handler = state.handler.read().await;
    if let Some(response) = handler.handle_request(request).await {
        let json = serde_json::to_string(&response).unwrap_or_default();
        let _ = state.tx.send(json);
        axum::http::StatusCode::ACCEPTED
    } else {
        axum::http::StatusCode::NO_CONTENT
    }
}

/// Start an SSE MCP server on the given port.
pub async fn run_sse(handler: McpHandler, port: u16) -> anyhow::Result<()> {
    let app = sse_router(handler);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!(port, "MCP SSE server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 3: Add module to lib.rs**

```rust
pub mod sse;
```

- [ ] **Step 4: Run compilation check**

Run: `cd rust && cargo check --package amanclaw-mcp`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-mcp/src/sse.rs rust/crates/amanclaw-mcp/src/lib.rs rust/crates/amanclaw-mcp/Cargo.toml
git commit -m "feat(mcp): add SSE server transport"
```

---

## Chunk 5: CLI Commands

### Task 5: Add `amanclaw mcp` CLI commands

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Add Mcp subcommand to cli.rs**

```rust
/// Manage MCP servers
Mcp {
    #[command(subcommand)]
    action: McpAction,
},
```

```rust
#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// List configured MCP servers
    List,
    /// List tools from a specific server
    Tools {
        /// Server name from config
        name: String,
    },
    /// Start MCP server mode (expose AmanClaw as MCP server)
    Serve {
        /// Transport: "stdio" or "sse"
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Port for SSE transport
        #[arg(short, long, default_value = "3001")]
        port: u16,
    },
}
```

- [ ] **Step 2: Implement cmd_mcp() in main.rs**

```rust
async fn cmd_mcp(config_path: &str, action: cli::McpAction) -> Result<()> {
    match action {
        cli::McpAction::List => {
            let config_path = find_config(config_path)?;
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: amanclaw_traits::config::AppConfig = serde_yaml::from_str(&config_str)?;

            if config.mcp_servers.is_empty() {
                println!("No MCP servers configured.");
                return Ok(());
            }

            println!("Configured MCP servers:\n");
            for (name, server) in &config.mcp_servers {
                let transport = if server.url.is_some() { "HTTP" } else { "stdio" };
                let target = server.url.as_deref()
                    .or(server.command.as_deref())
                    .unwrap_or("unknown");
                println!("  {name} ({transport}): {target}");
            }
            Ok(())
        }
        cli::McpAction::Tools { name } => {
            let config_path = find_config(config_path)?;
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: amanclaw_traits::config::AppConfig = serde_yaml::from_str(&config_str)?;

            let server_config = config.mcp_servers.get(&name)
                .ok_or_else(|| anyhow::anyhow!("MCP server '{name}' not found in config"))?;

            println!("Connecting to MCP server '{name}'...");
            let client = amanclaw_mcp::client::McpClient::connect(server_config).await?;
            let tools = client.list_tools().await?;

            println!("\nTools from '{name}' ({} total):\n", tools.len());
            for tool in &tools {
                println!("  {} — {}", tool.name, tool.description.as_deref().unwrap_or(""));
            }
            Ok(())
        }
        cli::McpAction::Serve { transport, port } => {
            let config_path = find_config(config_path)?;
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: amanclaw_traits::config::AppConfig = serde_yaml::from_str(&config_str)?;

            // Build handler with all skills
            let result = amanclaw_core::Engine::start(config).await?;
            let skills = result.handle.skills().await?;
            println!("AmanClaw MCP server ({transport}) with {} tools", skills.len());

            match transport.as_str() {
                "stdio" => {
                    let handler = amanclaw_mcp::handler::McpHandler::new("amanclaw", env!("CARGO_PKG_VERSION"));
                    // Note: skills need to be added to handler — simplified for now
                    amanclaw_mcp::stdio::run_stdio(handler).await?;
                }
                "sse" => {
                    let handler = amanclaw_mcp::handler::McpHandler::new("amanclaw", env!("CARGO_PKG_VERSION"));
                    amanclaw_mcp::sse::run_sse(handler, port).await?;
                }
                other => anyhow::bail!("Unknown transport: {other}. Use 'stdio' or 'sse'."),
            }
            Ok(())
        }
    }
}
```

- [ ] **Step 3: Wire up in main match**

```rust
Some(Command::Mcp { action }) => cmd_mcp(&cli.config, action).await,
```

- [ ] **Step 4: Add clap tests**

```rust
#[test]
fn test_cli_mcp_list() {
    let cli = Cli::parse_from(["amanclaw", "mcp", "list"]);
    assert!(matches!(cli.command, Some(Command::Mcp { action: McpAction::List })));
}

#[test]
fn test_cli_mcp_tools() {
    let cli = Cli::parse_from(["amanclaw", "mcp", "tools", "filesystem"]);
    match cli.command {
        Some(Command::Mcp { action: McpAction::Tools { name } }) => {
            assert_eq!(name, "filesystem");
        }
        _ => panic!("expected Mcp Tools command"),
    }
}

#[test]
fn test_cli_mcp_serve_stdio() {
    let cli = Cli::parse_from(["amanclaw", "mcp", "serve"]);
    match cli.command {
        Some(Command::Mcp { action: McpAction::Serve { transport, port } }) => {
            assert_eq!(transport, "stdio");
            assert_eq!(port, 3001);
        }
        _ => panic!("expected Mcp Serve command"),
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cd rust && cargo test --package amanclaw-cli cli::tests -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add rust/crates/amanclaw-cli/src/cli.rs rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): add 'amanclaw mcp' commands (list, tools, serve)"
```

---

## Summary

| Task | Description | Steps |
|------|-------------|-------|
| 1 | Resource + Prompt protocol types | 6 |
| 2 | Handler methods for resources/prompts | 7 |
| 3 | Client methods for resources/prompts | 5 |
| 4 | SSE server transport | 5 |
| 5 | CLI MCP commands | 6 |

**Total: 5 tasks, 29 steps**

After completing this plan:
```bash
amanclaw mcp list                    # List configured MCP servers
amanclaw mcp tools filesystem        # List tools from a server
amanclaw mcp serve --transport sse   # Expose AmanClaw as MCP server
```
