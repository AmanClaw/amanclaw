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

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub user_id: String,
    pub platform: String,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub state: Option<String>,
}

pub async fn list_users(
    State(state): State<ApiState>,
    Query(query): Query<UserListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let mut users = mem
        .list_users(
            query.platform.as_deref(),
            query.status.as_deref(),
            query.search.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Merge admin status from in-memory Auth
    let auth = state.auth.read().await;
    let admin_map = auth.admin_users();

    // Mark existing users as admin if they appear in the admin list
    for user in &mut users {
        if let Some(admin_ids) = admin_map.get(&user.platform) {
            if admin_ids.iter().any(|id| id == &user.user_id) {
                user.state = "admin".to_string();
            }
        }
    }

    // Add admin-only users (in admin list but not in DB)
    for (platform, admin_ids) in admin_map {
        // Skip if filtering by platform and this isn't a match
        if let Some(ref filter_platform) = query.platform {
            if filter_platform != platform {
                continue;
            }
        }
        for admin_id in admin_ids {
            let already_listed = users
                .iter()
                .any(|u| u.user_id == *admin_id && u.platform == *platform);
            if !already_listed {
                // Skip if filtering by status and it's not "admin"
                if let Some(ref filter_status) = query.status {
                    if filter_status != "admin" {
                        continue;
                    }
                }
                users.push(amanclaw_memory::sqlite::UserRow {
                    user_id: admin_id.clone(),
                    platform: platform.clone(),
                    state: "admin".to_string(),
                    username: None,
                    first_name: None,
                    first_seen: None,
                    last_seen: None,
                });
            }
        }
    }

    let count = users.len();
    Ok(Json(serde_json::json!({ "users": users, "count": count })))
}

pub async fn add_user(
    State(state): State<ApiState>,
    Json(body): Json<AddUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_state = body.state.as_deref().unwrap_or("approved");

    // Insert/upsert into SQLite
    let mem = SqliteMemory::from_pool(state.pool.clone());
    mem.upsert_user(
        &body.user_id,
        &body.platform,
        user_state,
        body.username.as_deref(),
        body.first_name.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update in-memory Auth cache
    let mut auth = state.auth.write().await;
    auth.register_user(
        &body.user_id,
        &body.platform,
        body.username.as_deref(),
        body.first_name.as_deref(),
    );
    match user_state {
        "approved" => auth.approve_user(&body.user_id, &body.platform),
        "blocked" => auth.block_user(&body.user_id, &body.platform),
        _ => {}
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "user_id": body.user_id,
        "platform": body.platform,
        "state": user_state,
    })))
}

pub async fn make_admin(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.make_admin(&user_id, &platform);
    Ok(Json(
        serde_json::json!({ "ok": true, "user_id": user_id, "platform": platform, "state": "admin" }),
    ))
}

pub async fn remove_admin(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.remove_admin(&user_id, &platform);
    Ok(Json(
        serde_json::json!({ "ok": true, "user_id": user_id, "platform": platform, "state": "approved" }),
    ))
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
