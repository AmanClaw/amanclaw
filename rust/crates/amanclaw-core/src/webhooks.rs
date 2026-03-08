use amanclaw_traits::config::{WebhookConfig, WebhookEndpointConfig, WebhookAuthConfig, WebhookTransformConfig};
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use crate::scheduler::SchedulerEvent;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct WebhookRouter {
    endpoints: HashMap<String, WebhookEndpointConfig>,
    default_secret: Option<String>,
    tx: mpsc::Sender<SchedulerEvent>,
}

impl WebhookRouter {
    pub fn new(config: &WebhookConfig, tx: mpsc::Sender<SchedulerEvent>) -> Self {
        Self {
            endpoints: config.endpoints.clone(),
            default_secret: config.default_secret.clone(),
            tx,
        }
    }

    pub fn list_endpoints(&self) -> Vec<(&str, &str, bool)> {
        self.endpoints.iter()
            .map(|(id, ep)| (id.as_str(), ep.name.as_str(), ep.enabled))
            .collect()
    }

    pub async fn handle(
        &self,
        webhook_id: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<WebhookResult> {
        let endpoint = self.endpoints.get(webhook_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown webhook: {}", webhook_id))?;

        if !endpoint.enabled {
            return Ok(WebhookResult::Rejected("Webhook disabled".into()));
        }

        // Auth validation
        let secret = endpoint.auth.secret.as_ref()
            .or(self.default_secret.as_ref());
        if !validate_auth(&endpoint.auth, headers, body, secret)? {
            return Ok(WebhookResult::Rejected("Auth failed".into()));
        }

        // Parse body as JSON
        let payload: serde_json::Value = serde_json::from_slice(body)
            .unwrap_or_else(|_| {
                serde_json::json!({ "raw": String::from_utf8_lossy(body).to_string() })
            });

        // Transform
        let message = transform(&endpoint.transform, &payload)?;

        // Send to targets
        for target in &endpoint.targets {
            match endpoint.transform.transform_type.as_str() {
                "agent_prompt" | "skill_invocation" => {
                    self.tx.send(SchedulerEvent::InjectMessage(IncomingMessage {
                        user_id: format!("webhook:{}", webhook_id),
                        chat_id: target.chat_id.clone(),
                        platform: target.platform.clone(),
                        text: message.clone(),
                        username: None,
                        first_name: None,
                        is_group: false,
                        image_data: None,
                        reply_to: None,
                        topic_id: target.topic_id.clone(),
                        channel_context: None,
                        is_cron: false,
                        is_webhook: true,
                        is_subagent: false,
                    })).await?;
                }
                _ => {
                    self.tx.send(SchedulerEvent::SendMessage(OutgoingMessage {
                        chat_id: target.chat_id.clone(),
                        text: message.clone(),
                        parse_mode: None,
                        reply_to: None,
                        platform: Some(target.platform.clone()),
                        topic_id: target.topic_id.clone(),
                    })).await?;
                }
            }
        }

        Ok(WebhookResult::Accepted)
    }
}

#[derive(Debug)]
pub enum WebhookResult {
    Accepted,
    Rejected(String),
}

/// Validate webhook auth based on config.
fn validate_auth(
    auth: &WebhookAuthConfig,
    headers: &HashMap<String, String>,
    body: &[u8],
    secret: Option<&String>,
) -> Result<bool> {
    match auth.auth_type.as_str() {
        "none" => Ok(true),
        "hmac_sha256" => {
            let secret = secret
                .ok_or_else(|| anyhow::anyhow!("HMAC auth requires a secret"))?;
            let header_name = auth.header.as_deref().unwrap_or("x-hub-signature-256");
            let signature = headers.get(header_name)
                .ok_or_else(|| anyhow::anyhow!("Missing signature header: {}", header_name))?;
            Ok(verify_hmac_sha256(secret.as_bytes(), body, signature))
        }
        "bearer" => {
            let expected = auth.token.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Bearer auth requires a token"))?;
            let auth_header = headers.get("authorization")
                .ok_or_else(|| anyhow::anyhow!("Missing Authorization header"))?;
            Ok(auth_header == &format!("Bearer {}", expected))
        }
        "header_match" => {
            let header_name = auth.header.as_ref()
                .ok_or_else(|| anyhow::anyhow!("header_match requires header name"))?;
            let expected = auth.value.as_ref()
                .ok_or_else(|| anyhow::anyhow!("header_match requires value"))?;
            let actual = headers.get(header_name.to_lowercase().as_str());
            Ok(actual.map_or(false, |v| v == expected))
        }
        other => {
            tracing::warn!(auth_type = %other, "Unknown webhook auth type, rejecting");
            Ok(false)
        }
    }
}

fn verify_hmac_sha256(secret: &[u8], body: &[u8], signature: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .expect("HMAC can take key of any size");
    mac.update(body);
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());

    // Support "sha256=..." prefix (GitHub style)
    let sig = signature.strip_prefix("sha256=").unwrap_or(signature);
    sig == expected
}

/// Transform webhook payload into a message string.
fn transform(config: &WebhookTransformConfig, payload: &serde_json::Value) -> Result<String> {
    match config.transform_type.as_str() {
        "raw_json" => Ok(serde_json::to_string_pretty(payload)?),
        "json_path" => {
            let path = config.message_path.as_deref().unwrap_or("$.message");
            let value = json_path_extract(payload, path);
            Ok(value.unwrap_or_else(|| format!("{}", payload)))
        }
        "template" => {
            let template = config.template.as_deref().unwrap_or("{{json}}");
            let hbs = handlebars::Handlebars::new();
            let mut data = HashMap::new();
            data.insert("json".to_string(), serde_json::to_string_pretty(payload)?);
            // Add top-level keys from payload
            if let Some(obj) = payload.as_object() {
                for (k, v) in obj {
                    data.insert(k.clone(), match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    });
                }
            }
            Ok(hbs.render_template(template, &data)?)
        }
        "agent_prompt" => {
            let prompt_template = config.prompt_template.as_deref()
                .unwrap_or("Process this webhook payload: {{json}}");
            let hbs = handlebars::Handlebars::new();
            let mut data = HashMap::new();
            data.insert("json".to_string(), serde_json::to_string(payload)?);
            if let Some(obj) = payload.as_object() {
                for (k, v) in obj {
                    data.insert(k.clone(), match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    });
                }
            }
            Ok(hbs.render_template(prompt_template, &data)?)
        }
        "skill_invocation" => {
            let skill = config.skill.as_deref().unwrap_or("echo");
            let input_template = config.input_template.as_deref().unwrap_or("{{json}}");
            let hbs = handlebars::Handlebars::new();
            let mut data = HashMap::new();
            data.insert("json".to_string(), serde_json::to_string(payload)?);
            let input = hbs.render_template(input_template, &data)?;
            Ok(format!("/{} {}", skill, input))
        }
        other => {
            tracing::warn!(transform_type = %other, "Unknown transform type, using raw JSON");
            Ok(serde_json::to_string_pretty(payload)?)
        }
    }
}

/// Simple dot-notation JSON path extractor (e.g., "$.title" or "message.text").
fn json_path_extract(value: &serde_json::Value, path: &str) -> Option<String> {
    let path = path.strip_prefix("$.").unwrap_or(path);
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    match current {
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::config::CronTargetConfig;

    #[test]
    fn test_hmac_sha256_verification() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = b"test-secret";
        let body = b"hello world";

        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_hmac_sha256(secret, body, &sig));
        assert!(verify_hmac_sha256(secret, body, &format!("sha256={}", sig)));
        assert!(!verify_hmac_sha256(secret, body, "invalid"));
    }

    #[test]
    fn test_bearer_auth() {
        let auth = WebhookAuthConfig {
            auth_type: "bearer".into(),
            token: Some("my-token".into()),
            ..Default::default()
        };
        let mut headers = HashMap::new();
        headers.insert("authorization".into(), "Bearer my-token".into());
        assert!(validate_auth(&auth, &headers, b"", None).unwrap());

        headers.insert("authorization".into(), "Bearer wrong".into());
        assert!(!validate_auth(&auth, &headers, b"", None).unwrap());
    }

    #[test]
    fn test_header_match_auth() {
        let auth = WebhookAuthConfig {
            auth_type: "header_match".into(),
            header: Some("x-api-key".into()),
            value: Some("secret123".into()),
            ..Default::default()
        };
        let mut headers = HashMap::new();
        headers.insert("x-api-key".into(), "secret123".into());
        assert!(validate_auth(&auth, &headers, b"", None).unwrap());
    }

    #[test]
    fn test_json_path_extraction() {
        let payload = serde_json::json!({
            "alert": {
                "title": "Server down",
                "message": "CPU at 100%"
            }
        });
        assert_eq!(json_path_extract(&payload, "$.alert.title"), Some("Server down".into()));
        assert_eq!(json_path_extract(&payload, "alert.message"), Some("CPU at 100%".into()));
        assert_eq!(json_path_extract(&payload, "$.missing"), None);
    }

    #[test]
    fn test_template_transform() {
        let config = WebhookTransformConfig {
            transform_type: "template".into(),
            template: Some("Alert: {{title}} - {{body}}".into()),
            ..Default::default()
        };
        let payload = serde_json::json!({
            "title": "Deploy",
            "body": "v2.0 released"
        });
        let result = transform(&config, &payload).unwrap();
        assert!(result.contains("Alert: Deploy - v2.0 released"));
    }

    #[test]
    fn test_raw_json_transform() {
        let config = WebhookTransformConfig {
            transform_type: "raw_json".into(),
            ..Default::default()
        };
        let payload = serde_json::json!({"key": "value"});
        let result = transform(&config, &payload).unwrap();
        assert!(result.contains("\"key\""));
    }

    #[tokio::test]
    async fn test_full_webhook_handling() {
        let (tx, mut rx) = mpsc::channel(16);
        let config = WebhookConfig {
            base_path: "/hooks".into(),
            default_secret: None,
            endpoints: HashMap::from([(
                "test".into(),
                WebhookEndpointConfig {
                    name: "Test Hook".into(),
                    path: "/hooks/test".into(),
                    auth: WebhookAuthConfig::default(),
                    transform: WebhookTransformConfig {
                        transform_type: "template".into(),
                        template: Some("Got: {{message}}".into()),
                        ..Default::default()
                    },
                    targets: vec![CronTargetConfig {
                        platform: "telegram".into(),
                        chat_id: "12345".into(),
                        topic_id: None,
                    }],
                    agent: None,
                    rate_limit: None,
                    enabled: true,
                },
            )]),
        };

        let router = WebhookRouter::new(&config, tx);
        let headers = HashMap::new();
        let body = br#"{"message": "hello"}"#;

        let result = router.handle("test", &headers, body).await.unwrap();
        assert!(matches!(result, WebhookResult::Accepted));

        let event = rx.recv().await.unwrap();
        match event {
            SchedulerEvent::SendMessage(msg) => {
                assert!(msg.text.contains("Got: hello"));
                assert_eq!(msg.chat_id, "12345");
            }
            _ => panic!("Expected SendMessage"),
        }
    }

    #[tokio::test]
    async fn test_disabled_webhook() {
        let (tx, _rx) = mpsc::channel(16);
        let config = WebhookConfig {
            base_path: "/hooks".into(),
            default_secret: None,
            endpoints: HashMap::from([(
                "disabled".into(),
                WebhookEndpointConfig {
                    name: "Disabled".into(),
                    path: "/hooks/disabled".into(),
                    auth: WebhookAuthConfig::default(),
                    transform: WebhookTransformConfig::default(),
                    targets: vec![],
                    agent: None,
                    rate_limit: None,
                    enabled: false,
                },
            )]),
        };

        let router = WebhookRouter::new(&config, tx);
        let result = router.handle("disabled", &HashMap::new(), b"{}").await.unwrap();
        assert!(matches!(result, WebhookResult::Rejected(_)));
    }
}
