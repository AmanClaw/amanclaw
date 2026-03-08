use crate::state::ApiState;
use amanclaw_memory::community::{Community, CommunityRepo};
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateCommunity {
    pub name: String,
    pub zone: String,
    pub language: String,
    pub platform: String,
    pub platform_group_id: String,
    pub enabled_skills: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateSkills {
    pub enabled_skills: Vec<String>,
}

fn community_to_json(c: &Community) -> serde_json::Value {
    serde_json::json!({
        "id": c.id,
        "name": c.name,
        "zone": c.zone,
        "language": c.language,
        "platform": c.platform,
        "platform_group_id": c.platform_group_id,
        "enabled_skills": c.enabled_skills,
    })
}

pub async fn list_communities(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = CommunityRepo::new(&state.pool);
    let communities = repo.list_all().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items: Vec<serde_json::Value> = communities.iter().map(community_to_json).collect();
    Ok(Json(serde_json::json!({ "communities": items, "count": items.len() })))
}

pub async fn get_community(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = CommunityRepo::new(&state.pool);
    let community = repo.get(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match community {
        Some(c) => Ok(Json(community_to_json(&c))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn create_community(
    State(state): State<ApiState>,
    Json(body): Json<CreateCommunity>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = uuid::Uuid::new_v4().to_string();
    let community = Community {
        id: id.clone(),
        name: body.name,
        zone: body.zone,
        language: body.language,
        platform: body.platform,
        platform_group_id: body.platform_group_id,
        enabled_skills: body.enabled_skills.unwrap_or_default(),
    };
    let repo = CommunityRepo::new(&state.pool);
    repo.upsert(&community).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(community_to_json(&community)))
}

pub async fn delete_community(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = CommunityRepo::new(&state.pool);
    let deleted = repo.delete(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if deleted {
        Ok(Json(serde_json::json!({ "ok": true })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn update_community_skills(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSkills>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo = CommunityRepo::new(&state.pool);
    let community = repo.get(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match community {
        Some(mut c) => {
            c.enabled_skills = body.enabled_skills;
            repo.upsert(&c).await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(serde_json::json!({ "ok": true })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
