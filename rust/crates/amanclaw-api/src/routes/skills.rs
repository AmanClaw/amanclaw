use crate::state::ApiState;
use axum::{extract::State, Json};

pub async fn list_skills(
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    let tools = state.registry.get_tool_definitions();
    let skills: Vec<serde_json::Value> = tools.iter().map(|t| {
        serde_json::json!({
            "name": t.name,
            "description": t.description,
        })
    }).collect();
    let count = skills.len();
    Json(serde_json::json!({ "skills": skills, "count": count }))
}
