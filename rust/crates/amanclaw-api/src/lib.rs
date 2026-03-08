pub mod auth;
pub mod routes;
pub mod state;

use axum::{middleware, routing::{get, post, put, delete}, Router};
use state::ApiState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn api_router(state: ApiState) -> Router {
    let authed = Router::new()
        .route("/api/status", get(routes::bot::get_status))
        .route("/api/communities", get(routes::communities::list_communities))
        .route("/api/communities", post(routes::communities::create_community))
        .route("/api/communities/{id}", get(routes::communities::get_community))
        .route("/api/communities/{id}", delete(routes::communities::delete_community))
        .route("/api/communities/{id}/skills", put(routes::communities::update_community_skills))
        .route("/api/skills", get(routes::skills::list_skills))
        .route("/api/users", get(routes::users::list_users))
        .route("/api/users/{platform}/{user_id}/approve", post(routes::users::approve_user))
        .route("/api/users/{platform}/{user_id}/block", post(routes::users::block_user))
        .route("/api/webhooks", get(routes::webhooks::list_webhooks))
        .layer(middleware::from_fn_with_state(state.clone(), auth::require_auth))
        .with_state(state.clone());

    // Webhook receiver — no auth middleware (uses its own auth)
    let webhook_routes = Router::new()
        .route("/hooks/{webhook_id}", post(routes::webhooks::receive_webhook))
        .with_state(state);

    Router::new()
        .merge(authed)
        .merge(webhook_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

pub async fn run_api_server(state: ApiState, port: u16) -> anyhow::Result<()> {
    let app = api_router(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Management API listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
