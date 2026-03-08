//! Unofficial WhatsApp channel adapter via WAHA (WhatsApp HTTP API).
//!
//! WAHA is a self-hosted WhatsApp Web bridge that exposes a REST API.
//! See: https://waha.devlike.pro
//!
//! Environment variables:
//! - `WAHA_API_URL` — Base URL of the WAHA instance (e.g. http://localhost:3000)
//! - `WAHA_API_KEY` — API key for WAHA (optional)
//! - `WAHA_SESSION` — Session name (default: "default")
//! - `WAHA_WEBHOOK_PORT` — Port for incoming webhook (default: 8081)

use amanclaw_traits::channel::Channel;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use axum::{extract::State, routing::post, Json, Router};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

// --- Incoming webhook types (WAHA webhook format) ---

#[derive(Debug, Deserialize)]
struct WahaWebhook {
    event: String,
    payload: Option<serde_json::Value>,
    #[allow(dead_code)]
    session: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WahaMessage {
    id: Option<String>,
    from: Option<String>,
    #[allow(dead_code)]
    to: Option<String>,
    body: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    from_me: Option<bool>,
    has_media: Option<bool>,
    // Chat ID for group messages
    chat_id: Option<String>,
    // Contact info
    #[serde(rename = "_data")]
    data: Option<WahaMessageData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WahaMessageData {
    notify_name: Option<String>,
}

// --- Outgoing message types ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WahaSendMessage {
    chat_id: String,
    text: String,
    session: String,
}

// --- Channel implementation ---

struct AppState {
    tx: mpsc::Sender<IncomingMessage>,
}

pub struct WhatsAppWebChannel {
    api_url: String,
    api_key: Option<String>,
    session: String,
    webhook_port: u16,
    http: Client,
}

impl WhatsAppWebChannel {
    pub fn new(api_url: String, api_key: Option<String>, session: String, webhook_port: u16) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_url,
            api_key,
            session,
            webhook_port,
            http,
        }
    }

    pub fn from_env() -> Option<Self> {
        let api_url = std::env::var("WAHA_API_URL").ok()?;
        let api_key = std::env::var("WAHA_API_KEY").ok();
        let session = std::env::var("WAHA_SESSION").unwrap_or_else(|_| "default".into());
        let webhook_port: u16 = std::env::var("WAHA_WEBHOOK_PORT")
            .unwrap_or_else(|_| "8081".into())
            .parse()
            .unwrap_or(8081);

        Some(Self::new(api_url, api_key, session, webhook_port))
    }
}

#[async_trait::async_trait]
impl Channel for WhatsAppWebChannel {
    fn platform(&self) -> &str {
        "whatsapp-web"
    }

    async fn start(&mut self, tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()> {
        let state = Arc::new(AppState { tx });

        let app = Router::new()
            .route("/webhook", post(handle_webhook))
            .with_state(state);

        let port = self.webhook_port;
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                .await
                .expect("Failed to bind WAHA webhook port");
            tracing::info!(port, "WAHA webhook server listening");
            axum::serve(listener, app).await.ok();
        });

        tracing::info!(api_url = %self.api_url, session = %self.session, "WhatsApp Web channel started (WAHA)");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("WhatsApp Web channel stopping...");
        Ok(())
    }

    async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()> {
        let url = format!("{}/api/sendText", self.api_url);

        let payload = WahaSendMessage {
            chat_id: msg.chat_id,
            text: msg.text,
            session: self.session.clone(),
        };

        let mut req = self.http.post(&url).json(&payload);

        if let Some(ref key) = self.api_key {
            req = req.header("X-Api-Key", key);
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(body, "WAHA API error");
        }

        Ok(())
    }
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    Json(webhook): Json<WahaWebhook>,
) -> &'static str {
    // Only handle incoming message events
    if webhook.event != "message" && webhook.event != "message.any" {
        return "OK";
    }

    let Some(payload) = webhook.payload else {
        return "OK";
    };

    let msg: WahaMessage = match serde_json::from_value(payload) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse WAHA message");
            return "OK";
        }
    };

    // Skip messages sent by us
    if msg.from_me.unwrap_or(false) {
        return "OK";
    }

    let text = msg.body.unwrap_or_default();
    if text.is_empty() && !msg.has_media.unwrap_or(false) {
        return "OK";
    }

    let from = msg.from.unwrap_or_default();
    // WAHA phone numbers: "601234567890@c.us" for direct, "...@g.us" for groups
    let user_id = from.split('@').next().unwrap_or(&from).to_string();
    let chat_id = msg.chat_id.unwrap_or_else(|| from.clone());
    let is_group = chat_id.contains("@g.us");

    let display_text = if msg.has_media.unwrap_or(false) && text.is_empty() {
        format!("[Media: {}]", msg.msg_type.unwrap_or_else(|| "unknown".into()))
    } else {
        text
    };

    let first_name = msg.data
        .and_then(|d| d.notify_name);

    let incoming = IncomingMessage {
        user_id,
        chat_id,
        platform: "whatsapp-web".into(),
        text: display_text,
        username: None,
        first_name,
        is_group,
        image_data: None,
        reply_to: None,
    };

    let _ = state.tx.send(incoming).await;

    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let channel = WhatsAppWebChannel::new(
            "http://localhost:3000".into(),
            None,
            "default".into(),
            8081,
        );
        assert_eq!(channel.platform(), "whatsapp-web");
    }

    #[test]
    fn test_deserialize_waha_webhook() {
        let json = r#"{
            "event": "message",
            "session": "default",
            "payload": {
                "id": "msg_123",
                "from": "601234567890@c.us",
                "to": "60987654321@c.us",
                "body": "Hello from WhatsApp!",
                "type": "chat",
                "fromMe": false,
                "hasMedia": false,
                "chatId": "601234567890@c.us",
                "_data": {
                    "notifyName": "Aman"
                }
            }
        }"#;

        let webhook: WahaWebhook = serde_json::from_str(json).unwrap();
        assert_eq!(webhook.event, "message");

        let msg: WahaMessage = serde_json::from_value(webhook.payload.unwrap()).unwrap();
        assert_eq!(msg.body.unwrap(), "Hello from WhatsApp!");
        assert_eq!(msg.from.unwrap(), "601234567890@c.us");
        assert!(!msg.from_me.unwrap());
        assert_eq!(msg.data.unwrap().notify_name.unwrap(), "Aman");
    }

    #[test]
    fn test_group_detection() {
        let chat_id = "120363123456789@g.us";
        assert!(chat_id.contains("@g.us"));

        let dm_id = "601234567890@c.us";
        assert!(!dm_id.contains("@g.us"));
    }

    #[test]
    fn test_user_id_extraction() {
        let from = "601234567890@c.us";
        let user_id = from.split('@').next().unwrap();
        assert_eq!(user_id, "601234567890");
    }
}
