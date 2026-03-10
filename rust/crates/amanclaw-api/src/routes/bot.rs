use crate::state::ApiState;
use amanclaw_memory::community::CommunityRepo;
use axum::{Json, extract::State};

pub async fn get_status(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let uptime = state.started_at.elapsed().as_secs();
    let skills_count = state.registry.skill_count();
    let users_count = state.auth.read().await.list_users().len();
    let communities_count = CommunityRepo::new(&state.pool)
        .list_all()
        .await
        .map(|c| c.len())
        .unwrap_or(0);

    Json(serde_json::json!({
        "running": true,
        "uptime_seconds": uptime,
        "communities_count": communities_count,
        "users_count": users_count,
        "skills_count": skills_count,
    }))
}
