use amanclaw_core::Engine;
use amanclaw_core::handle::EngineHandle;
use amanclaw_traits::config::AppConfig;
use amanclaw_traits::message::IncomingMessage;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct CliRunner {
    handle: EngineHandle,
    user_id: String,
    _join: tokio::task::JoinHandle<Result<()>>,
}

impl CliRunner {
    /// Create a new CLI runner from a config file path.
    pub async fn from_config(config_path: PathBuf) -> Result<Self> {
        let config_str = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let config: AppConfig = serde_yaml::from_str(&config_str)
            .with_context(|| "Failed to parse config file")?;

        let result = Engine::start(config).await?;
        let user_id = whoami::username();

        Ok(Self {
            handle: result.handle,
            user_id,
            _join: result.join,
        })
    }

    /// One-shot: send a query, return the response text.
    pub async fn ask(&self, query: &str) -> Result<String> {
        let msg = self.build_message(query);
        let response = self.handle.ask(msg).await?;
        match response {
            Some(r) => Ok(r.text),
            None => Ok("(no response)".into()),
        }
    }

    fn build_message(&self, text: &str) -> IncomingMessage {
        IncomingMessage {
            user_id: self.user_id.clone(),
            chat_id: format!("cli-{}", self.user_id),
            platform: "cli".into(),
            text: text.to_string(),
            username: Some(self.user_id.clone()),
            first_name: None,
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
            is_cron: false,
            is_webhook: false,
            is_subagent: false,
        }
    }

    /// Shutdown the engine gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        self.handle.shutdown().await
    }
}
