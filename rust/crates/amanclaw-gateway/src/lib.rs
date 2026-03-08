pub mod handler;
pub mod protocol;
pub mod session;

use std::sync::Arc;

/// Shared state for the WebSocket gateway.
pub struct GatewayState {
    pub session_manager: Arc<session::SessionManager>,
    pub handler: Arc<handler::GatewayHandler>,
}
