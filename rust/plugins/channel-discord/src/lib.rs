use amanclaw_traits::channel::Channel;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use serenity::all::{
    Client, Context, EventHandler, GatewayIntents, Message, Ready, ChannelId,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

struct Handler {
    tx: mpsc::Sender<IncomingMessage>,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let incoming = IncomingMessage {
            user_id: msg.author.id.to_string(),
            chat_id: msg.channel_id.to_string(),
            platform: "discord".into(),
            text: msg.content.clone(),
            username: Some(msg.author.name.clone()),
            first_name: msg.author.global_name.clone(),
            is_group: true, // Discord channels are group-like
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
            is_cron: false,
            is_webhook: false,
            is_subagent: false,
        };
        let _ = self.tx.send(incoming).await;
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!(bot = %ready.user.name, "Discord bot connected");
    }
}

pub struct DiscordChannel {
    token: String,
    http: Arc<RwLock<Option<Arc<serenity::http::Http>>>>,
}

impl DiscordChannel {
    pub fn new(token: String) -> Self {
        Self {
            token,
            http: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Channel for DiscordChannel {
    fn platform(&self) -> &str {
        "discord"
    }

    async fn start(&mut self, tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let handler = Handler { tx };
        let mut client = Client::builder(&self.token, intents)
            .event_handler(handler)
            .await?;

        let http = client.http.clone();
        *self.http.write().await = Some(http);

        tokio::spawn(async move {
            if let Err(e) = client.start().await {
                tracing::error!(error = %e, "Discord client error");
            }
        });

        tracing::info!("Discord channel started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Discord channel stopping...");
        Ok(())
    }

    async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()> {
        let http_guard = self.http.read().await;
        if let Some(http) = http_guard.as_ref() {
            let channel_id: u64 = msg.chat_id.parse()?;
            ChannelId::new(channel_id)
                .say(http.as_ref(), &msg.text)
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let channel = DiscordChannel::new("fake-token".into());
        assert_eq!(channel.platform(), "discord");
    }
}
