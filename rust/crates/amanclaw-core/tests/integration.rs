use amanclaw_core::Engine;
use amanclaw_traits::config::AppConfig;
use std::io::Write;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["12345"]
plugins:
  dir: "/tmp/amanclaw-test-plugins"
"#,
        mock_server.uri()
    );

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();

    // Engine should start successfully
    let result = Engine::start(config).await;
    assert!(result.is_ok(), "Engine failed to start: {:?}", result.err());

    let result = result.unwrap();

    // Send a message from an admin user via the handle
    result
        .handle
        .send_message(amanclaw_traits::message::IncomingMessage {
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
        })
        .await
        .unwrap();

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

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["admin1"]
plugins:
  dir: "/tmp/amanclaw-test-plugins-2"
"#,
        mock_server.uri()
    );

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = Engine::start(config).await.unwrap();

    // Send message from unknown user — should get registration message
    result
        .handle
        .send_message(amanclaw_traits::message::IncomingMessage {
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
        })
        .await
        .unwrap();

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

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["admin1"]
plugins:
  dir: "/tmp/amanclaw-test-plugins-cron"
"#,
        mock_server.uri()
    );

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = Engine::start(config).await.unwrap();

    // Send a cron message from non-admin user — should bypass auth
    result
        .handle
        .send_message(amanclaw_traits::message::IncomingMessage {
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
        })
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    result.handle.shutdown().await.unwrap();
    let join_result = result.join.await.unwrap();
    assert!(join_result.is_ok());
}

#[tokio::test]
async fn test_pipeline_processes_message_end_to_end() {
    let mock_server = MockServer::start().await;

    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap()) };

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "I am responding to your message.", "tool_calls": null },
                "finish_reason": "stop"
            }]
        })))
        .expect(1..)
        .mount(&mock_server)
        .await;

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["admin42"]
plugins:
  dir: "/tmp/amanclaw-test-plugins-e2e"
"#,
        mock_server.uri()
    );

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = Engine::start(config).await.unwrap();

    // Send a message from an admin user
    result
        .handle
        .send_message(amanclaw_traits::message::IncomingMessage {
            user_id: "admin42".into(),
            chat_id: "admin42".into(),
            platform: "telegram".into(),
            text: "Tell me about the weather".into(),
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
        })
        .await
        .unwrap();

    // Wait for the pipeline to process the message
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify the LLM was called at least once
    let requests = mock_server.received_requests().await.unwrap();
    assert!(
        !requests.is_empty(),
        "Expected the mock LLM server to receive at least one request"
    );

    result.handle.shutdown().await.unwrap();
    let join_result = result.join.await.unwrap();
    assert!(join_result.is_ok());
}

#[tokio::test]
async fn test_check_command_returns_stats() {
    let mock_server = MockServer::start().await;

    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap()) };

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "Stats response", "tool_calls": null },
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["statsadmin"]
plugins:
  dir: "/tmp/amanclaw-test-plugins-stats"
"#,
        mock_server.uri()
    );

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = Engine::start(config).await.unwrap();

    // Send a /stats command from an admin user
    result
        .handle
        .send_message(amanclaw_traits::message::IncomingMessage {
            user_id: "statsadmin".into(),
            chat_id: "statsadmin".into(),
            platform: "telegram".into(),
            text: "/stats".into(),
            username: Some("statsadmin".into()),
            first_name: Some("Stats".into()),
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
            is_cron: false,
            is_webhook: false,
            is_subagent: false,
        })
        .await
        .unwrap();

    // Wait for command processing
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    result.handle.shutdown().await.unwrap();
    let join_result = result.join.await.unwrap();
    assert!(join_result.is_ok());
}

#[tokio::test]
async fn test_engine_ask_returns_response() {
    let mock_server = MockServer::start().await;

    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap()) };

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "Hello from Ask!", "tool_calls": null },
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
admin_users:
  telegram: ["askuser"]
plugins:
  dir: "/tmp/amanclaw-test-plugins-ask"
"#,
        mock_server.uri()
    );

    let config: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = Engine::start(config).await.unwrap();
    let handle = result.handle.clone();

    let msg = amanclaw_traits::message::IncomingMessage {
        user_id: "askuser".into(),
        chat_id: "cli-test".into(),
        platform: "cli".into(),
        text: "hello".into(),
        username: Some("tester".into()),
        first_name: None,
        is_group: false,
        image_data: None,
        reply_to: None,
        topic_id: None,
        channel_context: None,
        is_cron: false,
        is_webhook: false,
        is_subagent: false,
    };

    let response = handle.ask(msg).await.unwrap();
    assert!(response.is_some(), "Ask should return a response");
    let response = response.unwrap();
    assert!(!response.text.is_empty(), "Response text should not be empty");

    handle.shutdown().await.unwrap();
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
