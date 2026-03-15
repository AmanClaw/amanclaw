pub mod auth;
pub mod routes;
pub mod state;

use axum::{
    Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode as HttpStatus, Uri, header},
    middleware,
    response::{Html, IntoResponse},
    routing::{delete, get, post, put},
};
use include_dir::{Dir, include_dir};
use state::ApiState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

static DASHBOARD_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../apps/dashboard/dist");

pub fn api_router(state: ApiState) -> Router {
    let authed = Router::new()
        .route("/api/status", get(routes::bot::get_status))
        .route(
            "/api/communities",
            get(routes::communities::list_communities),
        )
        .route(
            "/api/communities",
            post(routes::communities::create_community),
        )
        .route(
            "/api/communities/{id}",
            get(routes::communities::get_community),
        )
        .route(
            "/api/communities/{id}",
            delete(routes::communities::delete_community),
        )
        .route(
            "/api/communities/{id}/skills",
            put(routes::communities::update_community_skills),
        )
        .route("/api/skills", get(routes::skills::list_skills))
        .route(
            "/api/users",
            get(routes::users::list_users).post(routes::users::add_user),
        )
        .route(
            "/api/users/{platform}/{user_id}/make-admin",
            put(routes::users::make_admin),
        )
        .route(
            "/api/users/{platform}/{user_id}/remove-admin",
            put(routes::users::remove_admin),
        )
        .route(
            "/api/users/{platform}/{user_id}",
            get(routes::users::get_user),
        )
        .route(
            "/api/users/{platform}/{user_id}/history",
            get(routes::users::get_user_history),
        )
        .route(
            "/api/users/{platform}/{user_id}/approve",
            put(routes::users::approve_user),
        )
        .route(
            "/api/users/{platform}/{user_id}/block",
            put(routes::users::block_user),
        )
        .route(
            "/api/users/{platform}/{user_id}/unblock",
            put(routes::users::unblock_user),
        )
        .route("/api/stats", get(routes::stats::get_stats))
        .route("/api/webhooks", get(routes::webhooks::list_webhooks))
        .route("/api/channels", get(routes::channels::list_channels))
        .route(
            "/api/channels/whatsapp-web/config",
            put(routes::channels::update_whatsapp_web),
        )
        .route(
            "/api/channels/whatsapp-web/qr",
            get(routes::channels::get_whatsapp_qr),
        )
        .route(
            "/api/channels/whatsapp-web/session",
            get(routes::channels::get_whatsapp_session),
        )
        .route("/api/islamic/status", get(routes::islamic::get_status))
        .route("/api/islamic/sync", post(routes::islamic::trigger_sync))
        .route(
            "/api/mcp-servers",
            get(routes::mcp::list_mcp_servers),
        )
        .route(
            "/api/mcp-servers/{name}",
            put(routes::mcp::save_mcp_server).delete(routes::mcp::delete_mcp_server),
        )
        .route("/api/channels/{id}", get(routes::channels::get_channel))
        .route(
            "/api/channels/{id}/start",
            post(routes::channels::start_channel),
        )
        .route(
            "/api/channels/{id}/stop",
            post(routes::channels::stop_channel),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ))
        .with_state(state.clone());

    // Webhook receiver — no auth middleware (uses its own auth)
    let webhook_routes = Router::new()
        .route(
            "/hooks/{webhook_id}",
            post(routes::webhooks::receive_webhook),
        )
        .with_state(state.clone());

    // Metrics endpoint — no auth (for Prometheus scraping)
    let metrics_routes = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state.clone());

    // Login endpoint — no auth middleware (used to obtain JWT)
    let login_route = Router::new()
        .route("/api/login", post(auth::login))
        .with_state(state.clone());

    // WebSocket gateway — no auth middleware (uses JSON-RPC auth)
    let ws_routes = Router::new()
        .route("/ws", get(ws_upgrade))
        .with_state(state);

    let dashboard_routes = Router::new()
        .route("/admin/{*path}", get(serve_dashboard))
        .route("/admin", get(serve_dashboard));

    Router::new()
        .merge(authed)
        .merge(login_route)
        .merge(webhook_routes)
        .merge(metrics_routes)
        .merge(ws_routes)
        .merge(dashboard_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn serve_dashboard(uri: Uri) -> impl IntoResponse {
    let path = uri.path().strip_prefix("/admin/").unwrap_or("");
    let path = if path.is_empty() { "index.html" } else { path };

    match DASHBOARD_DIR.get_file(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                HttpStatus::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                file.contents(),
            )
                .into_response()
        }
        None => {
            // SPA fallback — serve index.html for client-side routing
            match DASHBOARD_DIR.get_file("index.html") {
                Some(index) => {
                    Html(std::str::from_utf8(index.contents()).unwrap_or("")).into_response()
                }
                None => (HttpStatus::NOT_FOUND, "Dashboard not found").into_response(),
            }
        }
    }
}

async fn metrics_handler(State(state): State<ApiState>) -> String {
    match &state.metrics_handle {
        Some(handle) => handle.render(),
        None => "# metrics not enabled\n".to_string(),
    }
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<ApiState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: ApiState) {
    use futures::StreamExt;

    let gateway = match &state.gateway {
        Some(g) => g.clone(),
        None => {
            tracing::warn!("WebSocket connection rejected: gateway not enabled");
            return;
        }
    };

    let session_id = gateway.session_manager.connect().await;
    tracing::info!(session_id = %session_id, "WebSocket connected");

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                let request: amanclaw_gateway::protocol::JsonRpcRequest =
                    match serde_json::from_str(&text) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                let response = gateway.handler.dispatch(&request, &session_id).await;
                if let Ok(json) = serde_json::to_string(&response)
                    && socket
                        .send(axum::extract::ws::Message::Text(json.into()))
                        .await
                        .is_err()
                {
                    break;
                }
            }
            axum::extract::ws::Message::Close(_) => break,
            _ => {}
        }
    }

    gateway.session_manager.disconnect(&session_id).await;
    tracing::info!(session_id = %session_id, "WebSocket disconnected");
}

pub async fn run_api_server(state: ApiState, port: u16) -> anyhow::Result<()> {
    let app = api_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("Management API listening on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
