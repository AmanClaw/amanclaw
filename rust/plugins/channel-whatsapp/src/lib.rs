use amanclaw_traits::channel::Channel;
use amanclaw_traits::message::{IncomingMessage, InteractiveMessage, OutgoingMessage};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;

/// WhatsApp Cloud API webhook payload.
#[derive(Debug, Deserialize)]
struct WebhookPayload {
    entry: Option<Vec<Entry>>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    changes: Option<Vec<Change>>,
}

#[derive(Debug, Deserialize)]
struct Change {
    value: Option<ChangeValue>,
}

#[derive(Debug, Deserialize)]
struct ChangeValue {
    messages: Option<Vec<WaMessage>>,
    contacts: Option<Vec<WaContact>>,
    #[allow(dead_code)]
    metadata: Option<WaMetadata>,
}

#[derive(Debug, Deserialize)]
struct WaMessage {
    from: String,
    #[serde(rename = "type")]
    msg_type: String,
    text: Option<WaText>,
    image: Option<WaMedia>,
    interactive: Option<WaInteractive>,
    #[allow(dead_code)]
    id: String,
}

#[derive(Debug, Deserialize)]
struct WaInteractive {
    button_reply: Option<WaReply>,
    list_reply: Option<WaReply>,
}

#[derive(Debug, Deserialize)]
struct WaReply {
    #[allow(dead_code)]
    id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct WaText {
    body: String,
}

#[derive(Debug, Deserialize)]
struct WaMedia {
    id: String,
    #[allow(dead_code)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WaContact {
    profile: Option<WaProfile>,
    wa_id: String,
}

#[derive(Debug, Deserialize)]
struct WaProfile {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WaMetadata {
    #[allow(dead_code)]
    phone_number_id: Option<String>,
}

struct AppState {
    tx: mpsc::Sender<IncomingMessage>,
    #[allow(dead_code)]
    access_token: String,
}

pub struct WhatsAppChannel {
    access_token: String,
    phone_number_id: String,
    verify_token: String,
    webhook_port: u16,
    http: Client,
}

impl WhatsAppChannel {
    pub fn new(
        access_token: String,
        phone_number_id: String,
        verify_token: String,
        webhook_port: u16,
    ) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            access_token,
            phone_number_id,
            verify_token,
            webhook_port,
            http,
        }
    }

    pub fn from_env() -> Option<Self> {
        let access_token = std::env::var("WHATSAPP_ACCESS_TOKEN").ok()?;
        let phone_number_id = std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok()?;
        let verify_token =
            std::env::var("WHATSAPP_VERIFY_TOKEN").unwrap_or_else(|_| "amanclaw_verify".into());
        let webhook_port: u16 = std::env::var("WHATSAPP_WEBHOOK_PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse()
            .unwrap_or(8080);

        Some(Self::new(
            access_token,
            phone_number_id,
            verify_token,
            webhook_port,
        ))
    }
}

#[async_trait::async_trait]
impl Channel for WhatsAppChannel {
    fn platform(&self) -> &str {
        "whatsapp"
    }

    async fn start(&mut self, tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()> {
        let verify_token = self.verify_token.clone();
        let state = Arc::new(AppState {
            tx,
            access_token: self.access_token.clone(),
        });

        let app =
            Router::new()
                .route(
                    "/webhook",
                    get({
                        let vt = verify_token.clone();
                        move |query: axum::extract::Query<
                            std::collections::HashMap<String, String>,
                        >| async move {
                            let mode = query.get("hub.mode").cloned().unwrap_or_default();
                            let token = query.get("hub.verify_token").cloned().unwrap_or_default();
                            let challenge = query.get("hub.challenge").cloned().unwrap_or_default();

                            if mode == "subscribe" && token == vt {
                                challenge
                            } else {
                                "Forbidden".to_string()
                            }
                        }
                    }),
                )
                .route("/webhook", post(handle_webhook))
                .with_state(state);

        let port = self.webhook_port;
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
                .await
                .expect("Failed to bind WhatsApp webhook port");
            tracing::info!(port, "WhatsApp webhook server listening");
            axum::serve(listener, app).await.ok();
        });

        tracing::info!("WhatsApp channel started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("WhatsApp channel stopping...");
        Ok(())
    }

    async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()> {
        let url = format!(
            "https://graph.facebook.com/v21.0/{}/messages",
            self.phone_number_id
        );

        let payload = if let Some(interactive) = &msg.interactive {
            match interactive {
                InteractiveMessage::Buttons { body, buttons } => {
                    let wa_buttons: Vec<serde_json::Value> = buttons
                        .iter()
                        .take(3) // WhatsApp allows max 3 buttons
                        .map(|b| {
                            json!({
                                "type": "reply",
                                "reply": {
                                    "id": b.id,
                                    "title": b.title
                                }
                            })
                        })
                        .collect();

                    json!({
                        "messaging_product": "whatsapp",
                        "to": msg.chat_id,
                        "type": "interactive",
                        "interactive": {
                            "type": "button",
                            "body": { "text": body },
                            "action": { "buttons": wa_buttons }
                        }
                    })
                }
                InteractiveMessage::List {
                    body,
                    button_text,
                    sections,
                } => {
                    let wa_sections: Vec<serde_json::Value> = sections
                        .iter()
                        .map(|s| {
                            let rows: Vec<serde_json::Value> = s
                                .rows
                                .iter()
                                .map(|r| {
                                    let mut row = json!({
                                        "id": r.id,
                                        "title": r.title
                                    });
                                    if let Some(desc) = &r.description {
                                        row["description"] = json!(desc);
                                    }
                                    row
                                })
                                .collect();
                            json!({
                                "title": s.title,
                                "rows": rows
                            })
                        })
                        .collect();

                    json!({
                        "messaging_product": "whatsapp",
                        "to": msg.chat_id,
                        "type": "interactive",
                        "interactive": {
                            "type": "list",
                            "body": { "text": body },
                            "action": {
                                "button": button_text,
                                "sections": wa_sections
                            }
                        }
                    })
                }
            }
        } else {
            json!({
                "messaging_product": "whatsapp",
                "to": msg.chat_id,
                "type": "text",
                "text": { "body": msg.text }
            })
        };

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(body, "WhatsApp API error");
        }

        Ok(())
    }
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WebhookPayload>,
) -> &'static str {
    if let Some(entries) = payload.entry {
        for entry in entries {
            if let Some(changes) = entry.changes {
                for change in changes {
                    if let Some(value) = change.value {
                        let contacts = value.contacts.unwrap_or_default();
                        if let Some(messages) = value.messages {
                            for wa_msg in messages {
                                let text = match wa_msg.msg_type.as_str() {
                                    "text" => wa_msg.text.map(|t| t.body).unwrap_or_default(),
                                    "image" => {
                                        // For image messages, note the media ID
                                        format!(
                                            "[Image: media_id={}]",
                                            wa_msg.image.map(|i| i.id).unwrap_or_default()
                                        )
                                    }
                                    "interactive" => {
                                        if let Some(interactive) = wa_msg.interactive {
                                            if let Some(reply) = interactive.button_reply {
                                                reply.title
                                            } else if let Some(reply) = interactive.list_reply {
                                                reply.title
                                            } else {
                                                "[Interactive: no reply data]".into()
                                            }
                                        } else {
                                            "[Interactive: missing payload]".into()
                                        }
                                    }
                                    _ => format!("[Unsupported message type: {}]", wa_msg.msg_type),
                                };

                                let contact = contacts.iter().find(|c| c.wa_id == wa_msg.from);
                                let first_name = contact
                                    .and_then(|c| c.profile.as_ref())
                                    .and_then(|p| p.name.clone());

                                let incoming = IncomingMessage {
                                    user_id: wa_msg.from.clone(),
                                    chat_id: wa_msg.from,
                                    platform: "whatsapp".into(),
                                    text,
                                    username: None,
                                    first_name,
                                    is_group: false,
                                    image_data: None,
                                    reply_to: None,
                                    topic_id: None,
                                    channel_context: None,
                                    is_cron: false,
                                    is_webhook: false,
                                    is_subagent: false,
                                };

                                match state.tx.try_send(incoming) {
                                    Ok(()) => {}
                                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                        tracing::warn!(
                                            platform = "whatsapp",
                                            "Engine buffer full (backpressure)"
                                        );
                                    }
                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                        tracing::error!(
                                            platform = "whatsapp",
                                            "Engine channel closed"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let channel = WhatsAppChannel::new("token".into(), "12345".into(), "verify".into(), 8080);
        assert_eq!(channel.platform(), "whatsapp");
    }

    #[test]
    fn test_deserialize_interactive_button_reply() {
        let json = r#"{
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "601234567890",
                            "type": "interactive",
                            "interactive": {
                                "button_reply": {
                                    "id": "btn_1",
                                    "title": "Option 1"
                                }
                            },
                            "id": "msg_456"
                        }],
                        "contacts": [{
                            "profile": {"name": "Aman"},
                            "wa_id": "601234567890"
                        }]
                    }
                }]
            }]
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        let entry = &payload.entry.unwrap()[0];
        let change = &entry.changes.as_ref().unwrap()[0];
        let value = change.value.as_ref().unwrap();
        let msg = &value.messages.as_ref().unwrap()[0];
        assert_eq!(msg.msg_type, "interactive");
        let interactive = msg.interactive.as_ref().unwrap();
        assert_eq!(interactive.button_reply.as_ref().unwrap().title, "Option 1");
        assert_eq!(interactive.button_reply.as_ref().unwrap().id, "btn_1");
    }

    #[test]
    fn test_deserialize_interactive_list_reply() {
        let json = r#"{
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "601234567890",
                            "type": "interactive",
                            "interactive": {
                                "list_reply": {
                                    "id": "row_2",
                                    "title": "Row 2"
                                }
                            },
                            "id": "msg_789"
                        }],
                        "contacts": [{
                            "profile": {"name": "Aman"},
                            "wa_id": "601234567890"
                        }]
                    }
                }]
            }]
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        let entry = &payload.entry.unwrap()[0];
        let change = &entry.changes.as_ref().unwrap()[0];
        let value = change.value.as_ref().unwrap();
        let msg = &value.messages.as_ref().unwrap()[0];
        assert_eq!(msg.msg_type, "interactive");
        let interactive = msg.interactive.as_ref().unwrap();
        assert!(interactive.button_reply.is_none());
        assert_eq!(interactive.list_reply.as_ref().unwrap().title, "Row 2");
    }

    #[test]
    fn test_deserialize_webhook_payload() {
        let json = r#"{
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "601234567890",
                            "type": "text",
                            "text": {"body": "Hello"},
                            "id": "msg_123"
                        }],
                        "contacts": [{
                            "profile": {"name": "Aman"},
                            "wa_id": "601234567890"
                        }]
                    }
                }]
            }]
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        let entry = &payload.entry.unwrap()[0];
        let change = &entry.changes.as_ref().unwrap()[0];
        let value = change.value.as_ref().unwrap();
        let msg = &value.messages.as_ref().unwrap()[0];
        assert_eq!(msg.from, "601234567890");
        assert_eq!(msg.text.as_ref().unwrap().body, "Hello");
    }
}
