use crate::pipeline::Pipeline;
use amanclaw_traits::message::IncomingMessage;
#[cfg(test)]
use amanclaw_traits::message::OutgoingMessage;
use tokio::sync::mpsc;

/// Routes incoming messages from channels to the pipeline,
/// and dispatches outgoing responses back to channels.
pub struct Router {
    rx: mpsc::Receiver<IncomingMessage>,
    pipeline: Pipeline,
}

impl Router {
    pub fn new(rx: mpsc::Receiver<IncomingMessage>) -> Self {
        Self {
            rx,
            pipeline: Pipeline::new(),
        }
    }

    /// Main loop: receive messages, process, collect responses.
    /// In production, responses are sent back to the originating channel.
    pub async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            let platform = msg.platform.clone();
            let chat_id = msg.chat_id.clone();
            match self.pipeline.process(msg).await {
                Ok(Some(response)) => {
                    tracing::info!(platform, chat_id, "Response ready");
                    // TODO: dispatch response back to the correct channel
                    drop(response);
                }
                Ok(None) => {
                    tracing::debug!(platform, chat_id, "Message dropped (auth/rate limit)");
                }
                Err(e) => {
                    tracing::error!(platform, chat_id, error = %e, "Pipeline error");
                }
            }
        }
    }

    /// Test helper: process all messages in the channel and return responses.
    #[cfg(test)]
    pub async fn run_until_empty(mut self) -> Vec<OutgoingMessage> {
        let mut responses = Vec::new();
        while let Some(msg) = self.rx.recv().await {
            match self.pipeline.process(msg).await {
                Ok(Some(response)) => responses.push(response),
                _ => {}
            }
        }
        responses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_processes_incoming_message() {
        let (tx, rx) = mpsc::channel(32);
        let router = Router::new(rx);

        let msg = IncomingMessage {
            user_id: "u1".into(),
            chat_id: "c1".into(),
            platform: "test".into(),
            text: "hello".into(),
            username: None,
            first_name: None,
            is_group: false,
            image_data: None,
            reply_to: None,
        };

        tx.send(msg).await.unwrap();
        drop(tx); // close channel so router loop exits

        let responses = router.run_until_empty().await;
        assert_eq!(responses.len(), 1);
        assert!(responses[0].text.contains("hello"));
    }
}
