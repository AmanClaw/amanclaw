use amanclaw_core::registry::PluginRegistry;
use amanclaw_core::webhooks::WebhookRouter;
use amanclaw_security::auth::Auth;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ApiState {
    pub registry: Arc<PluginRegistry>,
    pub pool: SqlitePool,
    pub api_token: String,
    pub bot_status: Arc<RwLock<BotStatus>>,
    pub auth: Arc<RwLock<Auth>>,
    pub webhook_router: Option<Arc<WebhookRouter>>,
    pub gateway: Option<Arc<amanclaw_gateway::GatewayState>>,
    pub metrics_handle: Option<metrics_exporter_prometheus::PrometheusHandle>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BotStatus {
    pub running: bool,
    pub started_at: Option<String>,
    pub uptime_seconds: u64,
    pub communities_count: u64,
    pub users_count: u64,
    pub skills_count: usize,
}

impl Default for BotStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl BotStatus {
    pub fn new() -> Self {
        Self {
            running: false,
            started_at: None,
            uptime_seconds: 0,
            communities_count: 0,
            users_count: 0,
            skills_count: 0,
        }
    }
}
