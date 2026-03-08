use amanclaw_traits::config::AppConfig;
use amanclaw_core::Engine;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_engine_initializes_with_mock_llm() {
    let mock_server = MockServer::start().await;

    // Use a fresh temp DB so schema is created from scratch
    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap()) };

    // Mock LLM endpoint
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "Hello!", "tool_calls": null },
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let yaml = format!(r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["12345"]
plugins:
  dir: "/tmp/amanclaw-test-plugins"
"#, mock_server.uri());

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();

    // Engine should initialize successfully
    let engine = Engine::new(config).await;
    assert!(engine.is_ok(), "Engine failed to initialize: {:?}", engine.err());

    let engine = engine.unwrap();

    // Get a sender and send a test message
    let tx = engine.sender();

    // Spawn engine in background
    let handle = tokio::spawn(engine.run());

    // Send a message from an admin user
    tx.send(amanclaw_traits::message::IncomingMessage {
        user_id: "12345".into(),
        chat_id: "12345".into(),
        platform: "telegram".into(),
        text: "Hello bot".into(),
        username: Some("admin".into()),
        first_name: Some("Admin".into()),
        is_group: false,
        image_data: None,
        reply_to: None,
        topic_id: None,
        channel_context: None,
        is_cron: false,
        is_webhook: false,
        is_subagent: false,
    }).await.unwrap();

    // Drop sender to close the channel and let engine exit
    drop(tx);

    // Engine should complete without error
    let result = handle.await.unwrap();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_engine_handles_new_user_registration() {
    let mock_server = MockServer::start().await;

    // Use a fresh temp DB so schema is created from scratch
    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap()) };

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "Hi!", "tool_calls": null },
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let yaml = format!(r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["admin1"]
plugins:
  dir: "/tmp/amanclaw-test-plugins-2"
"#, mock_server.uri());

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let engine = Engine::new(config).await.unwrap();
    let tx = engine.sender();

    let handle = tokio::spawn(engine.run());

    // Send message from unknown user — should get registration message
    tx.send(amanclaw_traits::message::IncomingMessage {
        user_id: "unknown_user".into(),
        chat_id: "unknown_user".into(),
        platform: "telegram".into(),
        text: "Hello".into(),
        username: None,
        first_name: None,
        is_group: false,
        image_data: None,
        reply_to: None,
        topic_id: None,
        channel_context: None,
        is_cron: false,
        is_webhook: false,
        is_subagent: false,
    }).await.unwrap();

    drop(tx);
    let result = handle.await.unwrap();
    assert!(result.is_ok());
}
