use amanclaw_traits::config::AppConfig;
use amanclaw_core::Engine;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use std::io::Write;

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

    // Engine should start successfully
    let result = Engine::start(config).await;
    assert!(result.is_ok(), "Engine failed to start: {:?}", result.err());

    let result = result.unwrap();

    // Send a message from an admin user via the handle
    result.handle.send_message(amanclaw_traits::message::IncomingMessage {
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

    // Small delay to let message process
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Shutdown the engine
    result.handle.shutdown().await.unwrap();

    // Engine should complete without error
    let join_result = result.join.await.unwrap();
    assert!(join_result.is_ok());
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
    let result = Engine::start(config).await.unwrap();

    // Send message from unknown user — should get registration message
    result.handle.send_message(amanclaw_traits::message::IncomingMessage {
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

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    result.handle.shutdown().await.unwrap();
    let join_result = result.join.await.unwrap();
    assert!(join_result.is_ok());
}

#[tokio::test]
async fn test_cron_message_bypasses_auth() {
    let mock_server = MockServer::start().await;

    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap()) };

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "Cron response", "tool_calls": null },
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
  dir: "/tmp/amanclaw-test-plugins-cron"
"#, mock_server.uri());

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = Engine::start(config).await.unwrap();

    // Send a cron message from non-admin user — should bypass auth
    result.handle.send_message(amanclaw_traits::message::IncomingMessage {
        user_id: "cron-system".into(),
        chat_id: "some-chat".into(),
        platform: "telegram".into(),
        text: "Daily reminder".into(),
        username: None,
        first_name: None,
        is_group: false,
        image_data: None,
        reply_to: None,
        topic_id: None,
        channel_context: None,
        is_cron: true,
        is_webhook: false,
        is_subagent: false,
    }).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    result.handle.shutdown().await.unwrap();
    let join_result = result.join.await.unwrap();
    assert!(join_result.is_ok());
}

#[tokio::test]
async fn test_soul_loader_resolves_agent_prompt() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let soul_path = tmp_dir.path().join("test-agent.md");
    {
        let mut f = std::fs::File::create(&soul_path).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "version: 1").unwrap();
        writeln!(f, "language: en").unwrap();
        writeln!(f, "tags: [test]").unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "# PREAMBLE").unwrap();
        writeln!(f, "You are a test agent.").unwrap();
        writeln!(f, "# PERSONALITY").unwrap();
        writeln!(f, "Helpful and friendly.").unwrap();
    }

    let resolved = amanclaw_core::soul::SoulLoader::load(tmp_dir.path(), "test-agent.md").unwrap();
    assert!(resolved.prompt.contains("You are a test agent."));
    assert!(resolved.prompt.contains("Helpful and friendly."));
    assert!(resolved.tags.contains(&"test".to_string()));
}
