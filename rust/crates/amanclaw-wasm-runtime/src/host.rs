use std::collections::HashMap;

/// Host state passed to WASM plugins.
/// Provides http_fetch, logging, config, and secrets.
pub struct HostState {
    pub http_client: reqwest::Client,
    pub config: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
    pub logs: Vec<(String, String)>, // (level, message) for testing
}

impl HostState {
    pub fn new(config: HashMap<String, String>, secrets: HashMap<String, String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            config,
            secrets,
            logs: Vec::new(),
        }
    }

    pub fn log(&mut self, level: &str, message: &str) {
        match level {
            "error" => tracing::error!(target: "wasm_plugin", "{}", message),
            "warn" => tracing::warn!(target: "wasm_plugin", "{}", message),
            "debug" => tracing::debug!(target: "wasm_plugin", "{}", message),
            _ => tracing::info!(target: "wasm_plugin", "{}", message),
        }
        self.logs.push((level.to_string(), message.to_string()));
    }

    pub fn get_config(&self, key: &str) -> Option<String> {
        self.config.get(key).cloned()
    }

    pub fn get_secret(&self, key: &str) -> Option<String> {
        self.secrets.get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_state_config() {
        let mut config = HashMap::new();
        config.insert("api_url".into(), "https://example.com".into());
        let host = HostState::new(config, HashMap::new());

        assert_eq!(
            host.get_config("api_url"),
            Some("https://example.com".into())
        );
        assert_eq!(host.get_config("missing"), None);
    }

    #[test]
    fn test_host_state_logging() {
        let mut host = HostState::new(HashMap::new(), HashMap::new());
        host.log("info", "Plugin started");
        host.log("error", "Something went wrong");

        assert_eq!(host.logs.len(), 2);
        assert_eq!(host.logs[0], ("info".into(), "Plugin started".into()));
    }

    #[test]
    fn test_host_state_secrets_scoped() {
        let mut secrets = HashMap::new();
        secrets.insert("BRAVE_API_KEY".into(), "secret123".into());
        let host = HostState::new(HashMap::new(), secrets);

        assert_eq!(host.get_secret("BRAVE_API_KEY"), Some("secret123".into()));
        assert_eq!(host.get_secret("DATABASE_PASSWORD"), None);
    }
}
