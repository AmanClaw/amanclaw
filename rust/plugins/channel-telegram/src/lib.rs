use amanclaw_traits::channel::Channel;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use teloxide::prelude::*;
use tokio::sync::mpsc;

pub struct TelegramChannel {
    token: String,
    bot: Option<Bot>,
}

impl TelegramChannel {
    pub fn new(token: String) -> Self {
        Self { token, bot: None }
    }
}

#[async_trait::async_trait]
impl Channel for TelegramChannel {
    fn platform(&self) -> &str {
        "telegram"
    }

    async fn start(&mut self, tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()> {
        let bot = Bot::new(&self.token);
        self.bot = Some(bot.clone());

        tracing::info!("Telegram channel starting...");

        let handler = Update::filter_message().endpoint(move |msg: Message, _bot: Bot| {
            let tx = tx.clone();
            async move {
                if let Some(text) = msg.text() {
                    let user = msg.from.as_ref();
                    let incoming = IncomingMessage {
                        user_id: user.map(|u| u.id.0.to_string()).unwrap_or_default(),
                        chat_id: msg.chat.id.0.to_string(),
                        platform: "telegram".into(),
                        text: text.to_string(),
                        username: user.and_then(|u| u.username.clone()),
                        first_name: user.map(|u| u.first_name.clone()),
                        is_group: msg.chat.is_group() || msg.chat.is_supergroup(),
                        image_data: None,
                        reply_to: None,
                        topic_id: None,
                        channel_context: None,
                        is_cron: false,
                        is_webhook: false,
                        is_subagent: false,
                    };
                    match tx.try_send(incoming) {
                        Ok(()) => {}
                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                            tracing::warn!(
                                platform = "telegram",
                                "Engine buffer full (backpressure)"
                            );
                        }
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            tracing::error!(platform = "telegram", "Engine channel closed");
                        }
                    }
                }
                respond(())
            }
        });

        tokio::spawn(async move {
            Dispatcher::builder(bot, handler).build().dispatch().await;
        });

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Telegram channel stopping...");
        Ok(())
    }

    async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()> {
        if let Some(bot) = &self.bot {
            let chat_id = ChatId(msg.chat_id.parse::<i64>()?);
            bot.send_message(chat_id, &msg.text).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let channel = TelegramChannel::new("fake-token".into());
        assert_eq!(channel.platform(), "telegram");
    }
}
