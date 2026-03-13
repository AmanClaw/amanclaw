use crate::state::ApiState;
use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub role: String,
    pub exp: usize,
}

pub async fn login(
    State(state): State<ApiState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let admin_pw = state.admin_password.as_ref().ok_or(StatusCode::FORBIDDEN)?;
    if body.password != *admin_pw {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;

    let claims = Claims {
        role: "admin".into(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cookie = format!(
        "amanclaw_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400",
        token
    );

    Ok((
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie)],
        Json(serde_json::json!({ "ok": true })),
    ))
}

pub async fn require_auth(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check Bearer token first (existing behavior)
    let bearer_ok = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t == state.api_token)
        .unwrap_or(false);

    if bearer_ok {
        return Ok(next.run(request).await);
    }

    // Check JWT cookie
    let cookie_ok = request
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find_map(|c| c.trim().strip_prefix("amanclaw_token="))
        })
        .map(|token| {
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .is_ok()
        })
        .unwrap_or(false);

    if cookie_ok {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
