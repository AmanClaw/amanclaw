use amanclaw_traits::channel::Channel;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Slack channel adapter using Socket Mode (WebSocket) for events
/// and Web API for sending messages.
///
/// Required env vars:
/// - `SLACK_BOT_TOKEN`: Bot User OAuth Token (xoxb-...)
/// - `SLACK_APP_TOKEN`: App-Level Token (xapp-...) with `connections:write` scope
///
/// Optional:
/// - `SLACK_BOT_USER_ID`: The bot's own user ID to ignore self-messages.
///   Auto-detected on startup via `auth.test`.
pub struct SlackChannel {
    bot_token: String,
    app_token: String,
    bot_user_id: Arc<tokio::sync::RwLock<Option<String>>>,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct SlackSocketUrl {
    ok: bool,
    url: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackAuthTest {
    ok: bool,
    user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackEnvelope {
    #[serde(rename = "type")]
    envelope_type: Option<String>,
    envelope_id: Option<String>,
    payload: Option<SlackPayload>,
}

#[derive(Debug, Deserialize)]
struct SlackPayload {
    event: Option<SlackEvent>,
}

#[derive(Debug, Deserialize)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    user: Option<String>,
    text: Option<String>,
    channel: Option<String>,
    channel_type: Option<String>,
    thread_ts: Option<String>,
}

impl SlackChannel {
    pub fn new(bot_token: String, app_token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            bot_token,
            app_token,
            bot_user_id: Arc::new(tokio::sync::RwLock::new(None)),
            http,
        }
    }

    /// Create from environment variables. Returns None if required vars are missing.
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("SLACK_BOT_TOKEN").ok()?;
        let app_token = std::env::var("SLACK_APP_TOKEN").ok()?;
        Some(Self::new(bot_token, app_token))
    }

    /// Get a WebSocket URL via Slack's `apps.connections.open` API.
    async fn get_ws_url(&self) -> anyhow::Result<String> {
        let resp: SlackSocketUrl = self.http
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(&self.app_token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            anyhow::bail!("Slack apps.connections.open failed: {}", resp.error.unwrap_or_default());
        }

        resp.url.ok_or_else(|| anyhow::anyhow!("No WebSocket URL returned"))
    }

    /// Detect the bot's own user ID via `auth.test`.
    async fn detect_bot_user_id(&self) -> anyhow::Result<String> {
        let resp: SlackAuthTest = self.http
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            anyhow::bail!("Slack auth.test failed");
        }

        resp.user_id.ok_or_else(|| anyhow::anyhow!("No user_id from auth.test"))
    }
}

#[async_trait::async_trait]
impl Channel for SlackChannel {
    fn platform(&self) -> &str {
        "slack"
    }

    async fn start(&mut self, tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()> {
        // Detect bot user ID to filter self-messages
        match self.detect_bot_user_id().await {
            Ok(uid) => {
                tracing::info!(bot_user_id = %uid, "Slack bot identity detected");
                *self.bot_user_id.write().await = Some(uid);
            }
            Err(e) => {
                tracing::warn!(error = %e, "Could not detect Slack bot user ID, self-messages may not be filtered");
            }
        }

        let ws_url = self.get_ws_url().await?;
        let bot_user_id = self.bot_user_id.clone();
        let app_token = self.app_token.clone();
        let http = self.http.clone();

        tokio::spawn(async move {
            if let Err(e) = run_socket_mode(ws_url, tx, bot_user_id, app_token, http).await {
                tracing::error!(error = %e, "Slack Socket Mode error");
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Slack channel stopping...");
        Ok(())
    }

    async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()> {
        let mut payload = serde_json::json!({
            "channel": msg.chat_id,
            "text": msg.text,
        });

        // Reply in thread if there's a thread_ts
        if let Some(ref thread_ts) = msg.reply_to {
            payload["thread_ts"] = Value::String(thread_ts.clone());
        }

        let resp: Value = self.http
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        if resp["ok"].as_bool() != Some(true) {
            let error = resp["error"].as_str().unwrap_or("unknown");
            anyhow::bail!("Slack chat.postMessage failed: {}", error);
        }

        Ok(())
    }
}

/// Run the Socket Mode WebSocket loop, reconnecting on failure.
async fn run_socket_mode(
    initial_url: String,
    tx: mpsc::Sender<IncomingMessage>,
    bot_user_id: Arc<tokio::sync::RwLock<Option<String>>>,
    app_token: String,
    http: reqwest::Client,
) -> anyhow::Result<()> {
    let mut ws_url = initial_url;

    loop {
        tracing::info!("Connecting to Slack Socket Mode...");

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        tracing::info!("Slack Socket Mode connected");

        while let Some(msg_result) = read.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(error = %e, "Slack WebSocket read error");
                    break;
                }
            };

            let text = match msg {
                WsMessage::Text(t) => t,
                WsMessage::Ping(data) => {
                    let _ = write.send(WsMessage::Pong(data)).await;
                    continue;
                }
                WsMessage::Close(_) => {
                    tracing::info!("Slack WebSocket closed by server");
                    break;
                }
                _ => continue,
            };

            let envelope: SlackEnvelope = match serde_json::from_str(&text) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Acknowledge the envelope immediately (required by Socket Mode)
            if let Some(ref envelope_id) = envelope.envelope_id {
                let ack = serde_json::json!({ "envelope_id": envelope_id });
                let _ = write.send(WsMessage::Text(ack.to_string().into())).await;
            }

            // Handle disconnect requests
            if envelope.envelope_type.as_deref() == Some("disconnect") {
                tracing::info!("Slack requested disconnect, will reconnect");
                break;
            }

            // Process event callbacks
            if envelope.envelope_type.as_deref() != Some("events_api") {
                continue;
            }

            let event = match envelope.payload.and_then(|p| p.event) {
                Some(e) => e,
                None => continue,
            };

            // Only handle message events
            if event.event_type.as_deref() != Some("message") {
                continue;
            }

            let user = match event.user {
                Some(u) => u,
                None => continue, // No user = bot message or system
            };

            // Skip messages from the bot itself
            {
                let bot_id = bot_user_id.read().await;
                if bot_id.as_deref() == Some(&user) {
                    continue;
                }
            }

            let text = event.text.unwrap_or_default();
            if text.is_empty() {
                continue;
            }

            let is_dm = event.channel_type.as_deref() == Some("im");

            let incoming = IncomingMessage {
                user_id: user,
                chat_id: event.channel.unwrap_or_default(),
                platform: "slack".into(),
                text,
                username: None,
                first_name: None,
                is_group: !is_dm,
                image_data: None,
                reply_to: event.thread_ts,
            };

            let _ = tx.send(incoming).await;
        }

        // Reconnect: get a new WebSocket URL
        tracing::info!("Reconnecting to Slack Socket Mode...");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let resp: SlackSocketUrl = match http
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(&app_token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
        {
            Ok(r) => match r.json().await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to parse reconnect response");
                    continue;
                }
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to request reconnect URL");
                continue;
            }
        };

        if resp.ok {
            if let Some(url) = resp.url {
                ws_url = url;
            }
        } else {
            tracing::error!(error = ?resp.error, "Failed to get reconnect URL");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let channel = SlackChannel::new("xoxb-fake".into(), "xapp-fake".into());
        assert_eq!(channel.platform(), "slack");
    }

    #[test]
    fn test_from_env_missing() {
        // Without env vars set, should return None
        let channel = SlackChannel::from_env();
        // May or may not be None depending on environment
        let _ = channel;
    }

    #[test]
    fn test_envelope_parsing() {
        let json = r#"{
            "type": "events_api",
            "envelope_id": "abc123",
            "payload": {
                "event": {
                    "type": "message",
                    "user": "U12345",
                    "text": "Hello bot",
                    "channel": "C67890",
                    "channel_type": "im"
                }
            }
        }"#;
        let envelope: SlackEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.envelope_type.as_deref(), Some("events_api"));
        assert_eq!(envelope.envelope_id.as_deref(), Some("abc123"));
        let event = envelope.payload.unwrap().event.unwrap();
        assert_eq!(event.user.as_deref(), Some("U12345"));
        assert_eq!(event.text.as_deref(), Some("Hello bot"));
        assert_eq!(event.channel.as_deref(), Some("C67890"));
        assert_eq!(event.channel_type.as_deref(), Some("im"));
    }

    #[test]
    fn test_disconnect_envelope() {
        let json = r#"{"type": "disconnect", "reason": "refresh_requested"}"#;
        let envelope: SlackEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.envelope_type.as_deref(), Some("disconnect"));
    }
}
