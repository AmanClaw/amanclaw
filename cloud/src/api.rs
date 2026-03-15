//! Cloud management API routes.

use crate::state::CloudState;
use amanclaw_traits::message::IncomingMessage;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

/// The chat widget HTML template, embedded at compile time.
const CHAT_HTML: &str = include_str!("chat.html");

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,      // user ID
    tenant: String,   // tenant slug
    exp: usize,
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub bot_name: String,
    pub invite_code: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Build the cloud API router.
pub fn cloud_router(state: CloudState) -> Router {
    Router::new()
        .route("/api/cloud/signup", post(signup))
        .route("/api/cloud/login", post(login))
        .route("/api/cloud/tenant", get(get_tenant))
        .route("/api/cloud/tenant/status", get(tenant_status))
        // Chat widget routes
        .route("/t/{slug}/chat", get(serve_chat_widget))
        .route("/t/{slug}/api/chat", post(tenant_chat))
        .with_state(state)
}

async fn signup(
    State(state): State<CloudState>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validate invite
    let _invite = crate::invite::validate_invite(state.db.pool(), &req.invite_code)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Check email not taken
    if state.db.get_user_by_email(&req.email).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    // Generate slug from bot name
    let slug: String = req.bot_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if slug.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check slug not taken
    if state.db.get_tenant_by_slug(&slug).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    // Create tenant
    let tenant = state
        .db
        .create_tenant(&req.bot_name, &slug, &req.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create user (plain password for MVP — add hashing later)
    let user = state
        .db
        .create_user(&req.email, &req.password, &tenant.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Use invite
    crate::invite::use_invite(state.db.pool(), &req.invite_code, &user.id)
        .await
        .ok();

    // Provision tenant directory
    crate::tenant::provision_tenant(&tenant.id, &tenant.name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate JWT
    let token = create_jwt(&state.jwt_secret, &user.id, &slug)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tenant": {
            "slug": slug,
            "name": tenant.name,
        },
        "token": token,
    })))
}

async fn login(
    State(state): State<CloudState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = state
        .db
        .get_user_by_email(&req.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.password_hash != req.password {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let tenant_id = user.tenant_id.as_deref().ok_or(StatusCode::UNAUTHORIZED)?;
    let tenant = state
        .db
        .get_tenant(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = create_jwt(&state.jwt_secret, &user.id, &tenant.slug)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tenant": {
            "slug": tenant.slug,
            "name": tenant.name,
        },
        "token": token,
    })))
}

async fn get_tenant(
    State(state): State<CloudState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // For MVP: return first tenant (proper auth extraction comes later)
    let tenants = state.db.list_tenants().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tenant = tenants.first().ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "tenant": tenant,
    })))
}

async fn tenant_status(
    State(state): State<CloudState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tenants = state.db.list_tenants().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tenant = tenants.first().ok_or(StatusCode::NOT_FOUND)?;
    let running = state.router.read().await.is_running(&tenant.slug);

    Ok(Json(serde_json::json!({
        "slug": tenant.slug,
        "status": tenant.status,
        "engine_running": running,
    })))
}

// --- Chat widget routes ---

/// Serve the chat widget HTML with tenant placeholders filled in.
async fn serve_chat_widget(
    State(state): State<CloudState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let tenant = state
        .db
        .get_tenant_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if tenant.status != "active" {
        return Err(StatusCode::FORBIDDEN);
    }

    let html = CHAT_HTML
        .replace("{{TENANT_NAME}}", &tenant.name)
        .replace("{{TENANT_SLUG}}", &tenant.slug);

    Ok(Html(html))
}

#[derive(Deserialize)]
struct ChatRequest {
    text: String,
}

#[derive(Serialize)]
struct ChatResponse {
    text: String,
}

/// HTTP chat endpoint — accepts a user message and returns the bot response.
async fn tenant_chat(
    State(state): State<CloudState>,
    Path(slug): Path<String>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    if req.text.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get or start the tenant engine
    let engine = state
        .router
        .write()
        .await
        .get_engine(&slug)
        .await
        .map_err(|e| {
            tracing::error!(slug, error = %e, "Failed to get engine for tenant");
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    // Build an IncomingMessage for the web chat
    let msg = IncomingMessage {
        user_id: format!("web-{slug}"),
        chat_id: format!("web-{slug}"),
        platform: "web".to_string(),
        text: req.text,
        username: Some("Web User".to_string()),
        first_name: None,
        is_group: false,
        image_data: None,
        reply_to: None,
        topic_id: None,
        channel_context: None,
        is_cron: false,
        is_webhook: false,
        is_subagent: false,
    };

    // Ask the engine and wait for the response
    let response = engine.ask(msg).await.map_err(|e| {
        tracing::error!(slug, error = %e, "Engine ask failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let text = response
        .map(|r| r.text)
        .unwrap_or_else(|| "I'm not sure how to respond to that.".to_string());

    Ok(Json(ChatResponse { text }))
}

fn create_jwt(secret: &str, user_id: &str, slug: &str) -> Result<String, StatusCode> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tenant: slug.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, DecodingKey, Validation};

    #[test]
    fn test_create_jwt() {
        let token = create_jwt("test-secret", "user-1", "my-bot").unwrap();
        assert!(!token.is_empty());

        // Validate it
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("test-secret".as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(decoded.claims.sub, "user-1");
        assert_eq!(decoded.claims.tenant, "my-bot");
    }
}
