use amanclaw_traits::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppMode {
    Local,
    Remote { url: String, token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

/// Holds references to the running engine's subsystems.
pub struct EngineHandle {
    /// Dropping this aborts the engine task.
    pub abort_handle: tokio::task::AbortHandle,
    /// Auth for user management.
    pub auth: Arc<std::sync::Mutex<amanclaw_security::auth::Auth>>,
    /// SQLite pool for queries.
    pub pool: sqlx::SqlitePool,
    /// Plugin registry for skill listing.
    pub registry: Arc<amanclaw_core::registry::PluginRegistry>,
}

pub struct AppState {
    pub mode: AppMode,
    pub engine_status: EngineStatus,
    pub engine_handle: Option<EngineHandle>,
    pub config: Option<AppConfig>,
    pub started_at: Option<std::time::Instant>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Local,
            engine_status: EngineStatus::Stopped,
            engine_handle: None,
            config: None,
            started_at: None,
        }
    }
}
