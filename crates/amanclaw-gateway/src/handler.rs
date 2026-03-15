use std::sync::Arc;

use serde_json::json;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::session::SessionManager;

/// Dispatches incoming JSON-RPC requests to the appropriate handler.
pub struct GatewayHandler {
    pub session_manager: Arc<SessionManager>,
    pub api_token: String,
}

impl GatewayHandler {
    pub fn new(session_manager: Arc<SessionManager>, api_token: impl Into<String>) -> Self {
        Self {
            session_manager,
            api_token: api_token.into(),
        }
    }

    /// Dispatch a JSON-RPC request and return a response.
    pub async fn dispatch(&self, request: &JsonRpcRequest, session_id: &str) -> JsonRpcResponse {
        let id = request.id.clone().unwrap_or(serde_json::Value::Null);

        match request.method.as_str() {
            "gateway.auth" => self.handle_auth(&id, request, session_id).await,
            "gateway.ping" => self.handle_ping(&id, session_id).await,
            "subscribe" => self.handle_subscribe(&id, request, session_id).await,
            "unsubscribe" => self.handle_unsubscribe(&id, request, session_id).await,
            "engine.status" => self.handle_engine_status(&id).await,
            _ => JsonRpcResponse::error(id, -32601, "Method not found"),
        }
    }

    async fn handle_auth(
        &self,
        id: &serde_json::Value,
        request: &JsonRpcRequest,
        session_id: &str,
    ) -> JsonRpcResponse {
        let token = request
            .params
            .as_ref()
            .and_then(|p| p.get("token"))
            .and_then(|t| t.as_str());

        match token {
            Some(t) if t == self.api_token => {
                self.session_manager.authenticate(session_id).await;
                JsonRpcResponse::success(id.clone(), json!({"authenticated": true}))
            }
            _ => JsonRpcResponse::error(id.clone(), -32000, "Invalid or missing token"),
        }
    }

    async fn handle_ping(&self, id: &serde_json::Value, session_id: &str) -> JsonRpcResponse {
        self.session_manager.touch(session_id).await;
        JsonRpcResponse::success(id.clone(), json!({"pong": true}))
    }

    async fn handle_subscribe(
        &self,
        id: &serde_json::Value,
        request: &JsonRpcRequest,
        session_id: &str,
    ) -> JsonRpcResponse {
        let topic = request
            .params
            .as_ref()
            .and_then(|p| p.get("topic"))
            .and_then(|t| t.as_str());

        match topic {
            Some(t) => {
                self.session_manager.subscribe(session_id, t).await;
                JsonRpcResponse::success(id.clone(), json!({"subscribed": t}))
            }
            None => JsonRpcResponse::error(id.clone(), -32602, "Missing 'topic' parameter"),
        }
    }

    async fn handle_unsubscribe(
        &self,
        id: &serde_json::Value,
        request: &JsonRpcRequest,
        session_id: &str,
    ) -> JsonRpcResponse {
        let topic = request
            .params
            .as_ref()
            .and_then(|p| p.get("topic"))
            .and_then(|t| t.as_str());

        match topic {
            Some(t) => {
                self.session_manager.unsubscribe(session_id, t).await;
                JsonRpcResponse::success(id.clone(), json!({"unsubscribed": t}))
            }
            None => JsonRpcResponse::error(id.clone(), -32602, "Missing 'topic' parameter"),
        }
    }

    async fn handle_engine_status(&self, id: &serde_json::Value) -> JsonRpcResponse {
        let count = self.session_manager.session_count().await;
        JsonRpcResponse::success(
            id.clone(),
            json!({
                "status": "running",
                "active_sessions": count,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_handler() -> (Arc<SessionManager>, GatewayHandler) {
        let mgr = Arc::new(SessionManager::new());
        let handler = GatewayHandler::new(mgr.clone(), "secret-token");
        (mgr, handler)
    }

    fn make_request(method: &str, params: Option<serde_json::Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: Some(json!(1)),
        }
    }

    #[tokio::test]
    async fn auth_success() {
        let (mgr, handler) = make_handler();
        let sid = mgr.connect().await;
        let req = make_request("gateway.auth", Some(json!({"token": "secret-token"})));
        let resp = handler.dispatch(&req, &sid).await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["authenticated"], true);
    }

    #[tokio::test]
    async fn auth_failure() {
        let (mgr, handler) = make_handler();
        let sid = mgr.connect().await;
        let req = make_request("gateway.auth", Some(json!({"token": "wrong"})));
        let resp = handler.dispatch(&req, &sid).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32000);
    }

    #[tokio::test]
    async fn ping() {
        let (mgr, handler) = make_handler();
        let sid = mgr.connect().await;
        let req = make_request("gateway.ping", None);
        let resp = handler.dispatch(&req, &sid).await;
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["pong"], true);
    }

    #[tokio::test]
    async fn subscribe_and_unsubscribe() {
        let (mgr, handler) = make_handler();
        let sid = mgr.connect().await;

        let req = make_request("subscribe", Some(json!({"topic": "agent.*"})));
        let resp = handler.dispatch(&req, &sid).await;
        assert!(resp.error.is_none());

        let subs = mgr.get_subscribers("agent.tool_call").await;
        assert!(subs.contains(&sid));

        let req = make_request("unsubscribe", Some(json!({"topic": "agent.*"})));
        let resp = handler.dispatch(&req, &sid).await;
        assert!(resp.error.is_none());

        let subs = mgr.get_subscribers("agent.tool_call").await;
        assert!(!subs.contains(&sid));
    }

    #[tokio::test]
    async fn engine_status() {
        let (mgr, handler) = make_handler();
        let _sid = mgr.connect().await;
        let req = make_request("engine.status", None);
        let resp = handler.dispatch(&req, &_sid).await;
        let result = resp.result.unwrap();
        assert_eq!(result["status"], "running");
        assert_eq!(result["active_sessions"], 1);
    }

    #[tokio::test]
    async fn unknown_method() {
        let (mgr, handler) = make_handler();
        let sid = mgr.connect().await;
        let req = make_request("nonexistent.method", None);
        let resp = handler.dispatch(&req, &sid).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }
}
