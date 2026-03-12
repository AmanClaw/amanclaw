use amanclaw_traits::channel::Channel;
use amanclaw_traits::channel_config::{ChannelStatusInfo, ChannelsConfig, WhatsAppWebConfig};
use amanclaw_traits::message::IncomingMessage;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Manages channel lifecycle: start, stop, status tracking, hot-reload.
pub struct ChannelManager {
    channels: RwLock<HashMap<String, ChannelEntry>>,
    msg_tx: mpsc::Sender<IncomingMessage>,
}

struct ChannelEntry {
    channel: Arc<dyn Channel>,
    running: bool,
    error: Option<String>,
}

impl ChannelManager {
    pub fn new(msg_tx: mpsc::Sender<IncomingMessage>) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            msg_tx,
        }
    }

    /// Register a channel that was started externally (for backwards compat with env var init).
    pub async fn register_running(&self, id: &str, channel: Arc<dyn Channel>) {
        let mut channels = self.channels.write().await;
        channels.insert(
            id.to_string(),
            ChannelEntry {
                channel,
                running: true,
                error: None,
            },
        );
    }

    /// Get status of all known channels.
    pub async fn get_all_status(&self, config: &ChannelsConfig) -> Vec<ChannelStatusInfo> {
        let channels = self.channels.read().await;

        let all_ids = vec![
            (
                "telegram",
                config
                    .telegram
                    .as_ref()
                    .map(|c| c.enabled)
                    .unwrap_or(false),
                config.telegram.is_some(),
            ),
            (
                "discord",
                config
                    .discord
                    .as_ref()
                    .map(|c| c.enabled)
                    .unwrap_or(false),
                config.discord.is_some(),
            ),
            (
                "slack",
                config.slack.as_ref().map(|c| c.enabled).unwrap_or(false),
                config.slack.is_some(),
            ),
            (
                "whatsapp-cloud",
                config
                    .whatsapp_cloud
                    .as_ref()
                    .map(|c| c.enabled)
                    .unwrap_or(false),
                config.whatsapp_cloud.is_some(),
            ),
            (
                "whatsapp-web",
                config
                    .whatsapp_web
                    .as_ref()
                    .map(|c| c.enabled)
                    .unwrap_or(false),
                config.whatsapp_web.is_some(),
            ),
        ];

        all_ids
            .iter()
            .map(|(id, enabled, configured)| {
                let entry = channels.get(*id);
                ChannelStatusInfo {
                    id: id.to_string(),
                    platform: id.to_string(),
                    configured: *configured,
                    enabled: *enabled,
                    running: entry.map(|e| e.running).unwrap_or(false),
                    error: entry.and_then(|e| e.error.clone()),
                }
            })
            .collect()
    }

    /// Get status of a single channel.
    pub async fn get_status(
        &self,
        id: &str,
        config: &ChannelsConfig,
    ) -> Option<ChannelStatusInfo> {
        self.get_all_status(config)
            .await
            .into_iter()
            .find(|s| s.id == id)
    }

    /// Get a reference to a running channel for sending messages.
    pub async fn get_channel(&self, platform: &str) -> Option<Arc<dyn Channel>> {
        let channels = self.channels.read().await;
        channels
            .get(platform)
            .filter(|e| e.running)
            .map(|e| e.channel.clone())
    }

    /// Get all running channels (for Engine to use in send_to_channel).
    pub async fn get_running_channels(&self) -> Vec<Arc<dyn Channel>> {
        let channels = self.channels.read().await;
        channels
            .values()
            .filter(|e| e.running)
            .map(|e| e.channel.clone())
            .collect()
    }

    /// Start a WhatsApp Web channel from config.
    pub async fn start_whatsapp_web(&self, config: &WhatsAppWebConfig) -> Result<()> {
        // Stop existing if running
        self.stop_channel("whatsapp-web").await.ok();

        let mut channel = amanclaw_channel_whatsapp_web::WhatsAppWebChannel::new(
            config.waha_url.clone(),
            config.waha_api_key.clone(),
            config.session.clone(),
            config.webhook_port,
        );

        match channel.start(self.msg_tx.clone()).await {
            Ok(()) => {
                let mut channels = self.channels.write().await;
                channels.insert(
                    "whatsapp-web".to_string(),
                    ChannelEntry {
                        channel: Arc::new(channel),
                        running: true,
                        error: None,
                    },
                );
                tracing::info!("WhatsApp Web channel started via ChannelManager");
                Ok(())
            }
            Err(e) => {
                let mut channels = self.channels.write().await;
                channels.insert(
                    "whatsapp-web".to_string(),
                    ChannelEntry {
                        channel: Arc::new(
                            amanclaw_channel_whatsapp_web::WhatsAppWebChannel::new(
                                config.waha_url.clone(),
                                config.waha_api_key.clone(),
                                config.session.clone(),
                                config.webhook_port,
                            ),
                        ),
                        running: false,
                        error: Some(e.to_string()),
                    },
                );
                Err(e)
            }
        }
    }

    /// Stop a channel by ID.
    pub async fn stop_channel(&self, id: &str) -> Result<()> {
        let mut channels = self.channels.write().await;
        if let Some(entry) = channels.get_mut(id) {
            entry.running = false;
            tracing::info!(channel = id, "Channel stopped");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_manager_empty() {
        let (tx, _rx) = mpsc::channel(1);
        let mgr = ChannelManager::new(tx);
        let config = ChannelsConfig::default();
        let statuses = mgr.get_all_status(&config).await;
        assert_eq!(statuses.len(), 5);
        assert!(statuses.iter().all(|s| !s.running));
        assert!(statuses.iter().all(|s| !s.configured));
    }

    #[tokio::test]
    async fn test_get_running_channels_empty() {
        let (tx, _rx) = mpsc::channel(1);
        let mgr = ChannelManager::new(tx);
        let running = mgr.get_running_channels().await;
        assert!(running.is_empty());
    }
}
