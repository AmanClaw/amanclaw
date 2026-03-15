use crate::state::ApiState;
use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

/// GET /api/islamic/status — returns sync status for all datasets.
pub async fn get_status(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    let db = state
        .islamic_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let statuses = amanclaw_islamic_db::sync::get_all_status(db.pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "datasets": statuses })))
}

/// POST /api/islamic/sync — trigger sync for a dataset.
/// Body: `{ "dataset": "quran" }` or `{ "dataset": "all" }`.
pub async fn trigger_sync(
    State(state): State<ApiState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let db = state
        .islamic_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let dataset = body["dataset"].as_str().unwrap_or("all");
    let pool = db.pool().clone();
    let api_key = std::env::var("SUNNAH_API_KEY").ok();
    let dataset_owned = dataset.to_string();

    // Spawn sync in background (don't block the request)
    tokio::spawn(async move {
        match dataset_owned.as_str() {
            "all" => {
                let _ = amanclaw_islamic_db::sync::sync_all(&pool, api_key.as_deref()).await;
                let _ = amanclaw_islamic_db::seed::load_fiqh_seed(&pool).await;
            }
            "quran" => {
                let _ = amanclaw_islamic_db::sync::sync_quran(&pool).await;
            }
            "hadith" => {
                let _ =
                    amanclaw_islamic_db::sync::sync_all_hadith(&pool, api_key.as_deref()).await;
            }
            "tafsir" => {
                let _ =
                    amanclaw_islamic_db::sync::sync_tafsir(&pool, "ibn_kathir", 169).await;
                let _ = amanclaw_islamic_db::sync::sync_tafsir(&pool, "jalalayn", 74).await;
            }
            "fiqh" => {
                let _ = amanclaw_islamic_db::seed::load_fiqh_seed(&pool).await;
            }
            other => tracing::warn!("Unknown dataset: {}", other),
        }
    });

    Ok(Json(
        json!({ "status": "sync_started", "dataset": dataset }),
    ))
}
