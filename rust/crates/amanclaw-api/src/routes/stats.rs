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
    Ok(Json(serde_json::json!(stats)))
}
