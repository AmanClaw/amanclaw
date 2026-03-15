use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Server-initiated event (JSON-RPC notification, no id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEvent {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcResponse {
    /// Build a successful response.
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Build an error response.
    pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

impl ServerEvent {
    /// Create a new server-initiated event.
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_roundtrip() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "gateway.ping".into(),
            params: Some(json!({"ts": 123})),
            id: Some(json!(1)),
        };
        let serialized = serde_json::to_string(&req).unwrap();
        let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.method, "gateway.ping");
        assert_eq!(deserialized.id, Some(json!(1)));
    }

    #[test]
    fn request_without_params_or_id() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "notify".into(),
            params: None,
            id: None,
        };
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(!serialized.contains("params"));
        assert!(!serialized.contains("\"id\""));
    }

    #[test]
    fn success_response_roundtrip() {
        let resp = JsonRpcResponse::success(json!(42), json!({"status": "ok"}));
        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.jsonrpc, "2.0");
        assert_eq!(deserialized.id, json!(42));
        assert!(deserialized.result.is_some());
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn error_response_roundtrip() {
        let resp = JsonRpcResponse::error(json!("abc"), -32600, "Invalid request");
        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.result.is_none());
        let err = deserialized.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid request");
    }

    #[test]
    fn server_event_roundtrip() {
        let evt = ServerEvent::new("agent.tool_call", json!({"tool": "search"}));
        let serialized = serde_json::to_string(&evt).unwrap();
        let deserialized: ServerEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.jsonrpc, "2.0");
        assert_eq!(deserialized.method, "agent.tool_call");
        assert_eq!(deserialized.params["tool"], "search");
    }
}
