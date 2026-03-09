use crate::state::ApiState;
use axum::{extract::{Path, State}, http::StatusCode, Json};

pub async fn list_users(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth = state.auth.read().await;
    let users: Vec<serde_json::Value> = auth.list_users().iter().map(|(id, platform, user_state)| {
        serde_json::json!({
            "user_id": id,
            "platform": platform,
            "state": user_state.to_string(),
        })
    }).collect();
    let count = users.len();
    Ok(Json(serde_json::json!({ "users": users, "count": count })))
}

pub async fn approve_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.approve_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "Approved" })))
}

pub async fn block_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.block_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "Blocked" })))
}
