use std::time::Duration;

/// Resource limits for WASM plugin execution.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum execution time before the plugin is killed.
    pub timeout: Duration,
    /// Maximum memory in bytes the plugin can use.
    pub max_memory_bytes: usize,
    /// Fuel budget for WASM execution (limits CPU usage). 0 = unlimited.
    pub fuel_limit: u64,
    /// Allowed host domains for http_fetch (empty = all allowed).
    pub allowed_domains: Vec<String>,
    /// Config keys this plugin can read.
    pub allowed_config_keys: Vec<String>,
    /// Secret keys this plugin can read.
    pub allowed_secret_keys: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_memory_bytes: 64 * 1024 * 1024, // 64 MB
            fuel_limit: 1_000_000,
            allowed_domains: vec![],
            allowed_config_keys: vec![],
            allowed_secret_keys: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sandbox_config() {
        let config = SandboxConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_memory_bytes, 64 * 1024 * 1024);
        assert_eq!(config.fuel_limit, 1_000_000);
        assert!(config.allowed_domains.is_empty());
    }
}
