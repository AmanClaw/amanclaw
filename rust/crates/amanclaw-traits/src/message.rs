use serde::{Deserialize, Serialize};

/// Normalized incoming message from any platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub user_id: String,
    pub chat_id: String,
    pub platform: String,
    pub text: String,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub is_group: bool,
    pub image_data: Option<Vec<u8>>,
    pub reply_to: Option<String>,
    /// Platform-specific topic/thread ID for agent routing.
    #[serde(default)]
    pub topic_id: Option<String>,
    /// Additional routing context (e.g., Discord channel name).
    #[serde(default)]
    pub channel_context: Option<String>,
}

/// Normalized outgoing message to any platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub chat_id: String,
    pub text: String,
    pub parse_mode: Option<String>,
    pub reply_to: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incoming_message_creation() {
        let msg = IncomingMessage {
            user_id: "12345".into(),
            chat_id: "12345".into(),
            platform: "telegram".into(),
            text: "Hello bot".into(),
            username: Some("aman".into()),
            first_name: Some("Aman".into()),
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
        };
        assert_eq!(msg.platform, "telegram");
        assert_eq!(msg.text, "Hello bot");
    }

    #[test]
    fn test_outgoing_message_creation() {
        let msg = OutgoingMessage {
            chat_id: "12345".into(),
            text: "Hi there!".into(),
            parse_mode: None,
            reply_to: None,
        };
        assert_eq!(msg.text, "Hi there!");
    }

    #[test]
    fn test_incoming_message_serialization() {
        let msg = IncomingMessage {
            user_id: "12345".into(),
            chat_id: "12345".into(),
            platform: "telegram".into(),
            text: "test".into(),
            username: None,
            first_name: None,
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: IncomingMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, "12345");
    }
}
