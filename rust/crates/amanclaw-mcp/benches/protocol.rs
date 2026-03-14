use criterion::{criterion_group, criterion_main, Criterion};

use amanclaw_mcp::protocol::*;

fn bench_json_rpc_serialization(c: &mut Criterion) {
    let response = JsonRpcResponse::success(
        Some(serde_json::json!(1)),
        serde_json::json!({"tools": []}),
    );

    c.bench_function("json_rpc_serialize_success", |b| {
        b.iter(|| serde_json::to_string(&response).unwrap())
    });
}

fn bench_json_rpc_error_serialization(c: &mut Criterion) {
    let response = JsonRpcResponse::error(
        Some(serde_json::json!(1)),
        METHOD_NOT_FOUND,
        "Method not found",
    );

    c.bench_function("json_rpc_serialize_error", |b| {
        b.iter(|| serde_json::to_string(&response).unwrap())
    });
}

fn bench_json_rpc_deserialization(c: &mut Criterion) {
    let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"test","arguments":{"query":"hello"}}}"#;

    c.bench_function("json_rpc_deserialize_request", |b| {
        b.iter(|| serde_json::from_str::<JsonRpcRequest>(json).unwrap())
    });
}

fn bench_mcp_tool_serialization(c: &mut Criterion) {
    let tools: Vec<McpTool> = (0..10)
        .map(|i| McpTool {
            name: format!("tool_{i}"),
            description: format!("Tool number {i} for testing"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"]
            }),
        })
        .collect();

    c.bench_function("serialize_10_mcp_tools", |b| {
        b.iter(|| serde_json::to_string(&tools).unwrap())
    });
}

fn bench_resource_serialization(c: &mut Criterion) {
    let resource = McpResource {
        uri: "file:///home/user/document.txt".into(),
        name: "document.txt".into(),
        description: Some("A text document".into()),
        mime_type: Some("text/plain".into()),
    };

    c.bench_function("serialize_mcp_resource", |b| {
        b.iter(|| serde_json::to_string(&resource).unwrap())
    });
}

fn bench_prompt_serialization(c: &mut Criterion) {
    let prompt = McpPrompt {
        name: "greeting".into(),
        description: Some("Generate a greeting".into()),
        arguments: vec![
            PromptArgument {
                name: "name".into(),
                description: Some("Person's name".into()),
                required: true,
            },
            PromptArgument {
                name: "language".into(),
                description: Some("Language code".into()),
                required: false,
            },
        ],
    };

    c.bench_function("serialize_mcp_prompt", |b| {
        b.iter(|| serde_json::to_string(&prompt).unwrap())
    });
}

criterion_group!(
    benches,
    bench_json_rpc_serialization,
    bench_json_rpc_error_serialization,
    bench_json_rpc_deserialization,
    bench_mcp_tool_serialization,
    bench_resource_serialization,
    bench_prompt_serialization,
);
criterion_main!(benches);
