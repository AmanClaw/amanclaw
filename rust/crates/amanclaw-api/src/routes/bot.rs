use crate::state::ApiState;
use axum::{Json, extract::State};

pub async fn get_status(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let status = state.bot_status.read().await;
    Json(serde_json::json!({
        "running": status.running,
        "started_at": status.started_at,
        "uptime_seconds": status.uptime_seconds,
        "communities_count": status.communities_count,
        "users_count": status.users_count,
        "skills_count": status.skills_count,
    }))
}
