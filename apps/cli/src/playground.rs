use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

const PLAYGROUND_HTML: &str = include_str!("../static/playground.html");

pub async fn run_playground(port: u16) -> anyhow::Result<()> {
    let app = build_router();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Playground running at http://localhost:{port}");
    println!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/skills", get(list_skills))
        .route("/api/send", post(send_message))
        .route("/api/health", get(health))
}

async fn index() -> Html<&'static str> {
    Html(PLAYGROUND_HTML)
}

#[derive(Serialize)]
struct SkillInfo {
    name: String,
    description: String,
}

async fn list_skills() -> Json<Vec<SkillInfo>> {
    Json(vec![
        SkillInfo {
            name: "solat".into(),
            description: "Prayer times by zone or global calculation".into(),
        },
        SkillInfo {
            name: "qiblat".into(),
            description: "Qiblat direction from any location".into(),
        },
        SkillInfo {
            name: "hijri".into(),
            description: "Hijri calendar conversion".into(),
        },
        SkillInfo {
            name: "doa".into(),
            description: "Islamic prayers and supplications".into(),
        },
        SkillInfo {
            name: "sysinfo".into(),
            description: "System information".into(),
        },
    ])
}

#[derive(Deserialize)]
struct SendRequest {
    message: String,
}

#[derive(Serialize)]
struct SendResponse {
    reply: String,
}

async fn send_message(Json(req): Json<SendRequest>) -> Json<SendResponse> {
    // Echo mode for now — will connect to real engine later
    Json(SendResponse {
        reply: format!("Echo: {}", req.message),
    })
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_playground_index() {
        let resp = build_router()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_playground_skills_api() {
        let resp = build_router()
            .oneshot(
                Request::builder()
                    .uri("/api/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_playground_send() {
        let resp = build_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message":"hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_playground_health() {
        let resp = build_router()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }
}
