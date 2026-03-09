use crate::state::ApiState;
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};

/// POST /hooks/{webhook_id} — Receive a webhook (no auth middleware).
pub async fn receive_webhook(
    State(state): State<ApiState>,
    Path(webhook_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    let router = state.webhook_router.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Convert headers to HashMap
    let header_map: std::collections::HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|val| (k.as_str().to_lowercase(), val.to_string()))
        })
        .collect();

    match router.handle(&webhook_id, &header_map, &body).await {
        Ok(amanclaw_core::webhooks::WebhookResult::Accepted) => Ok(StatusCode::OK),
        Ok(amanclaw_core::webhooks::WebhookResult::Rejected(reason)) => {
            tracing::warn!(webhook_id, reason, "Webhook rejected");
            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            tracing::error!(webhook_id, error = %e, "Webhook error");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/webhooks — List configured webhooks (requires auth).
pub async fn list_webhooks(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let endpoints = state
        .webhook_router
        .as_ref()
        .map(|r| {
            r.list_endpoints()
                .into_iter()
                .map(|(id, name, enabled)| {
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "enabled": enabled,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "webhooks": endpoints }))
}
