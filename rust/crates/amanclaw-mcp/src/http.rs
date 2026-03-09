//! MCP HTTP transport — JSON-RPC over HTTP POST.
//!
//! Provides an HTTP endpoint at `/mcp` that accepts JSON-RPC requests.
//! This enables remote MCP clients to connect to AmanClaw.

use crate::handler::McpHandler;
use crate::protocol::JsonRpcRequest;
use axum::{Json, Router, extract::State, routing::post};
use std::sync::Arc;

/// Create an axum Router for the MCP HTTP server.
pub fn mcp_router(handler: Arc<McpHandler>) -> Router {
    Router::new()
        .route("/mcp", post(handle_mcp_request))
        .with_state(handler)
}

async fn handle_mcp_request(
    State(handler): State<Arc<McpHandler>>,
    Json(req): Json<JsonRpcRequest>,
) -> axum::response::Response {
    tracing::debug!(method = %req.method, "MCP HTTP request");

    match handler.handle(req).await {
        Some(resp) => {
            let json = serde_json::to_string(&resp).unwrap_or_default();
            axum::response::Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(json))
                .unwrap()
        }
        None => {
            // Notification — no response needed
            axum::response::Response::builder()
                .status(204)
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }
}

/// Start the MCP HTTP server on the given port.
pub async fn run_http(handler: Arc<McpHandler>, port: u16) -> anyhow::Result<()> {
    let app = mcp_router(handler);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!(port, "MCP HTTP server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    struct EchoSkill;

    #[async_trait::async_trait]
    impl Skill for EchoSkill {
        fn metadata(&self) -> SkillMetadata {
            SkillMetadata {
                name: "echo".into(),
                description: "Echoes input".into(),
                timeout_ms: 5000,
                version: "0.1.0".into(),
            }
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            })
        }

        async fn execute(&self, input: SkillInput) -> SkillResult {
            let args: serde_json::Value = serde_json::from_str(&input.args).unwrap_or_default();
            SkillResult {
                success: true,
                output: format!("Echo: {}", args["text"].as_str().unwrap_or("")),
                error: None,
            }
        }
    }

    fn make_app() -> Router {
        let mut handler = McpHandler::new("test", "0.1.0");
        handler.register_skill(Arc::new(EchoSkill));
        mcp_router(Arc::new(handler))
    }

    #[tokio::test]
    async fn test_http_tools_list() {
        let app = make_app();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tools = json["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");
    }

    #[tokio::test]
    async fn test_http_tools_call() {
        let app = make_app();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "text": "hello MCP" }
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["result"]["content"][0]["text"], "Echo: hello MCP");
    }
}
