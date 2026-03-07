use crate::message::{IncomingMessage, OutgoingMessage};
use tokio::sync::mpsc;

/// Trait for messaging platform adapters.
///
/// Each channel receives messages from a platform and pushes them
/// to the engine via an mpsc sender. The engine replies via send_message.
#[async_trait::async_trait]
pub trait Channel: Send + Sync {
    /// Platform identifier (e.g., "telegram", "discord").
    fn platform(&self) -> &str;

    /// Start receiving messages. Push them into `tx`.
    async fn start(&mut self, tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()>;

    /// Stop the channel and clean up resources.
    async fn stop(&mut self) -> anyhow::Result<()>;

    /// Send a reply message to the platform.
    async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockChannel {
        name: String,
    }

    #[async_trait::async_trait]
    impl Channel for MockChannel {
        fn platform(&self) -> &str {
            &self.name
        }

        async fn start(&mut self, _tx: mpsc::Sender<IncomingMessage>) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn send_message(&self, _msg: OutgoingMessage) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_channel_platform() {
        let ch = MockChannel { name: "test".into() };
        assert_eq!(ch.platform(), "test");
    }
}
