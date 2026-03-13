use crate::state::ApiState;
use amanclaw_memory::sqlite::SqliteMemory;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserListQuery {
    pub platform: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_users(
    State(state): State<ApiState>,
    Query(query): Query<UserListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let users = mem
        .list_users(
            query.platform.as_deref(),
            query.status.as_deref(),
            query.search.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let count = users.len();
    Ok(Json(serde_json::json!({ "users": users, "count": count })))
}

pub async fn get_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let user = mem
        .get_user(&user_id, &platform)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let message_count = mem
        .get_message_count_ns("default", &user_id)
        .await
        .unwrap_or(0);
    let facts = mem.get_facts(&user_id).await.unwrap_or_default();

    Ok(Json(serde_json::json!({
        "user_id": user.user_id,
        "platform": user.platform,
        "state": user.state,
        "username": user.username,
        "first_name": user.first_name,
        "first_seen": user.first_seen,
        "last_seen": user.last_seen,
        "message_count": message_count,
        "facts": facts,
    })))
}

pub async fn get_user_history(
    State(state): State<ApiState>,
    Path((_platform, user_id)): Path<(String, String)>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let messages = mem
        .get_history_paginated("default", &user_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total = mem
        .get_message_count_ns("default", &user_id)
        .await
        .unwrap_or(0);
    Ok(Json(serde_json::json!({
        "messages": messages.iter().map(|m| serde_json::json!({
            "role": m.role,
            "content": m.content,
        })).collect::<Vec<_>>(),
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

pub async fn approve_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.approve_user(&user_id, &platform);
    Ok(Json(
        serde_json::json!({ "ok": true, "user_id": user_id, "state": "approved" }),
    ))
}

pub async fn block_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.block_user(&user_id, &platform);
    Ok(Json(
        serde_json::json!({ "ok": true, "user_id": user_id, "state": "blocked" }),
    ))
}

pub async fn unblock_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.unblock_user(&user_id, &platform);
    Ok(Json(
        serde_json::json!({ "ok": true, "user_id": user_id, "state": "pending" }),
    ))
}
