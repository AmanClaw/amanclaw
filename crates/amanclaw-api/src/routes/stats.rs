use crate::state::ApiState;
use amanclaw_memory::sqlite::SqliteMemory;
use axum::{Json, extract::State, http::StatusCode};

pub async fn get_stats(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let stats = mem
        .get_user_stats()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let auth = state.auth.read().await;
    let admin_count: usize = auth.admin_users().values().map(|v| v.len()).sum();

    Ok(Json(serde_json::json!({
        "total": stats.total,
        "pending": stats.pending,
        "approved": stats.approved,
        "blocked": stats.blocked,
        "admin": admin_count,
    })))
}
