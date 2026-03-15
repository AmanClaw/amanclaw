use amanclaw_traits::channel_config::ChannelsConfig;
use amanclaw_traits::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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

/// Convert core engine status to serializable desktop status.
impl From<amanclaw_core::handle::EngineStatus> for EngineStatus {
    fn from(status: amanclaw_core::handle::EngineStatus) -> Self {
        match status {
            amanclaw_core::handle::EngineStatus::Stopped => EngineStatus::Stopped,
            amanclaw_core::handle::EngineStatus::Starting => EngineStatus::Starting,
            amanclaw_core::handle::EngineStatus::Running { .. } => EngineStatus::Running,
            amanclaw_core::handle::EngineStatus::Error(e) => EngineStatus::Error(e),
        }
    }
}

/// Holds references to the running engine's subsystems.
pub struct EngineHandle {
    /// Core engine handle for sending commands (shutdown, status, skills).
    pub engine_handle: amanclaw_core::handle::EngineHandle,
    /// Join handle for the monitoring wrapper task.
    pub join_handle: tokio::task::JoinHandle<()>,
    /// Auth for user management.
    pub auth: Arc<tokio::sync::RwLock<amanclaw_security::auth::Auth>>,
    /// SQLite pool for queries.
    pub pool: sqlx::SqlitePool,
    /// Plugin registry for skill listing.
    pub registry: Arc<amanclaw_core::registry::PluginRegistry>,
    /// Sub-agent manager (None if sub-agents disabled).
    pub subagent_manager: Option<Arc<amanclaw_core::subagent::SubAgentManager>>,
    /// Channel manager for dynamic channel lifecycle.
    pub channel_manager: Option<Arc<amanclaw_core::channel_manager::ChannelManager>>,
    /// Shared channels config.
    pub channels_config: Option<Arc<tokio::sync::RwLock<ChannelsConfig>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

const MAX_LOG_ENTRIES: usize = 500;

pub struct AppState {
    pub mode: AppMode,
    pub engine_status: EngineStatus,
    pub engine_handle: Option<EngineHandle>,
    pub config: Option<AppConfig>,
    pub started_at: Option<std::time::Instant>,
    pub logs: VecDeque<LogEntry>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Local,
            engine_status: EngineStatus::Stopped,
            engine_handle: None,
            config: None,
            started_at: None,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
        }
    }

    pub fn push_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
    }
}
