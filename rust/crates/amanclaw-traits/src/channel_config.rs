use serde::{Deserialize, Serialize};

/// Per-channel configuration variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    Telegram(TelegramConfig),
    Discord(DiscordConfig),
    Slack(SlackConfig),
    WhatsappCloud(WhatsAppCloudConfig),
    WhatsappWeb(WhatsAppWebConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub app_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppCloudConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub access_token: String,
    pub phone_number_id: String,
    #[serde(default = "default_verify_token")]
    pub verify_token: String,
    #[serde(default = "default_whatsapp_port")]
    pub webhook_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppWebConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub waha_url: String,
    #[serde(default)]
    pub waha_api_key: Option<String>,
    #[serde(default = "default_session")]
    pub session: String,
    #[serde(default = "default_waha_port")]
    pub webhook_port: u16,
}

/// All channels config — the `channels:` section in config.yaml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,
    #[serde(default)]
    pub discord: Option<DiscordConfig>,
    #[serde(default)]
    pub slack: Option<SlackConfig>,
    #[serde(default)]
    pub whatsapp_cloud: Option<WhatsAppCloudConfig>,
    #[serde(default)]
    pub whatsapp_web: Option<WhatsAppWebConfig>,
}

impl ChannelsConfig {
    /// Build channels config from environment variables (backwards compatibility).
    /// Only populates channels whose env vars are set.
    pub fn from_env() -> Self {
        let telegram = std::env::var("TELEGRAM_BOT_TOKEN").ok().map(|token| {
            TelegramConfig { enabled: true, token }
        });

        let discord = std::env::var("DISCORD_BOT_TOKEN").ok().map(|token| {
            DiscordConfig { enabled: true, token }
        });

        let slack = std::env::var("SLACK_BOT_TOKEN").ok().map(|bot_token| {
            SlackConfig { enabled: true, bot_token, app_token: std::env::var("SLACK_APP_TOKEN").ok() }
        });

        let whatsapp_cloud = std::env::var("WHATSAPP_ACCESS_TOKEN").ok().map(|access_token| {
            WhatsAppCloudConfig {
                enabled: true,
                access_token,
                phone_number_id: std::env::var("WHATSAPP_PHONE_NUMBER_ID").unwrap_or_default(),
                verify_token: std::env::var("WHATSAPP_VERIFY_TOKEN").unwrap_or_else(|_| "amanclaw_verify".into()),
                webhook_port: std::env::var("WHATSAPP_WEBHOOK_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080),
            }
        });

        let whatsapp_web = std::env::var("WAHA_API_URL").ok().map(|waha_url| {
            WhatsAppWebConfig {
                enabled: true,
                waha_url,
                waha_api_key: std::env::var("WAHA_API_KEY").ok(),
                session: std::env::var("WAHA_SESSION").unwrap_or_else(|_| "default".into()),
                webhook_port: std::env::var("WAHA_WEBHOOK_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8081),
            }
        });

        Self { telegram, discord, slack, whatsapp_cloud, whatsapp_web }
    }

    /// Merge: prefer yaml config, fill gaps from env vars.
    pub fn merge_with_env(self) -> Self {
        let env = Self::from_env();
        Self {
            telegram: self.telegram.or(env.telegram),
            discord: self.discord.or(env.discord),
            slack: self.slack.or(env.slack),
            whatsapp_cloud: self.whatsapp_cloud.or(env.whatsapp_cloud),
            whatsapp_web: self.whatsapp_web.or(env.whatsapp_web),
        }
    }
}

/// Status info returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatusInfo {
    pub id: String,
    pub platform: String,
    pub configured: bool,
    pub enabled: bool,
    pub running: bool,
    pub error: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_verify_token() -> String {
    "amanclaw_verify".into()
}
fn default_whatsapp_port() -> u16 {
    8080
}
fn default_waha_port() -> u16 {
    8081
}
fn default_session() -> String {
    "default".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channels_config_deserialize_empty() {
        let yaml = "{}";
        let config: ChannelsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.telegram.is_none());
        assert!(config.whatsapp_web.is_none());
    }

    #[test]
    fn test_channels_config_deserialize_with_channels() {
        let yaml = r#"
telegram:
  token: "bot123:ABC"
whatsapp_web:
  waha_url: "http://localhost:3000"
  waha_api_key: "secret"
"#;
        let config: ChannelsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.telegram.is_some());
        assert_eq!(config.telegram.unwrap().token, "bot123:ABC");
        let wa = config.whatsapp_web.unwrap();
        assert_eq!(wa.waha_url, "http://localhost:3000");
        assert_eq!(wa.session, "default");
        assert_eq!(wa.webhook_port, 8081);
    }

    #[test]
    fn test_channel_status_info() {
        let status = ChannelStatusInfo {
            id: "telegram".into(),
            platform: "telegram".into(),
            configured: true,
            enabled: true,
            running: true,
            error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("telegram"));
    }

    #[test]
    fn test_merge_with_env_prefers_yaml() {
        let yaml_config = ChannelsConfig {
            telegram: Some(TelegramConfig { enabled: true, token: "yaml_token".into() }),
            ..Default::default()
        };
        let merged = yaml_config.merge_with_env();
        assert_eq!(merged.telegram.unwrap().token, "yaml_token");
    }
}
