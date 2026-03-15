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
    if let Some(response) = handler.handle(request).await {
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
