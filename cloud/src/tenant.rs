//! Tenant directory and config management.

use anyhow::Result;
use std::path::PathBuf;

/// Base directory for all tenant data.
const DEFAULT_TENANTS_DIR: &str = "cloud/tenants";

/// Get the tenants base directory.
pub fn tenants_dir() -> PathBuf {
    std::env::var("CLOUD_TENANTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_TENANTS_DIR))
}

/// Get a specific tenant's directory.
pub fn tenant_dir(tenant_id: &str) -> PathBuf {
    tenants_dir().join(format!("tenant-{tenant_id}"))
}

/// Create the tenant directory structure with default config.
pub fn provision_tenant(tenant_id: &str, tenant_name: &str) -> Result<PathBuf> {
    let dir = tenant_dir(tenant_id);

    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("plugins"))?;
    std::fs::create_dir_all(dir.join("souls"))?;
    std::fs::create_dir_all(dir.join("data"))?;

    // Write default config
    let config = default_tenant_config(tenant_name);
    std::fs::write(dir.join("config.yaml"), config)?;

    // Write default soul
    let soul = default_soul(tenant_name);
    std::fs::write(dir.join("souls").join("default.md"), soul)?;

    tracing::info!(tenant_id, path = %dir.display(), "Tenant directory provisioned");
    Ok(dir)
}

/// Check if a tenant directory exists.
pub fn tenant_exists(tenant_id: &str) -> bool {
    tenant_dir(tenant_id).exists()
}

/// Get paths to tenant databases.
pub fn tenant_memory_db(tenant_id: &str) -> PathBuf {
    tenant_dir(tenant_id).join("data").join("memory.db")
}

pub fn tenant_islamic_db(tenant_id: &str) -> PathBuf {
    tenant_dir(tenant_id).join("data").join("islamic.db")
}

pub fn tenant_config_path(tenant_id: &str) -> PathBuf {
    tenant_dir(tenant_id).join("config.yaml")
}

/// Remove a tenant's directory entirely.
pub fn deprovision_tenant(tenant_id: &str) -> Result<()> {
    let dir = tenant_dir(tenant_id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        tracing::info!(tenant_id, "Tenant directory removed");
    }
    Ok(())
}

fn default_tenant_config(name: &str) -> String {
    format!(
        r#"# AmanClaw — {name}
# Configure your LLM and channels below.

llm:
  base_url: "http://localhost:11434/v1"
  model: "qwen3:8b"
  max_tokens: 4096
  temperature: 0.7

admin_users: {{}}

rate_limit_per_minute: 30

skills:
  skill_timeout_seconds: 30
"#
    )
}

fn default_soul(name: &str) -> String {
    format!(
        r#"---
id: default
name: {name}
---

You are {name}, a helpful AI assistant powered by AmanClaw.
Be friendly, helpful, and concise.
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provision_and_check() {
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CLOUD_TENANTS_DIR", dir.path().to_str().unwrap()) };

        let path = provision_tenant("test-123", "Test Bot").unwrap();
        assert!(path.exists());
        assert!(path.join("config.yaml").exists());
        assert!(path.join("plugins").exists());
        assert!(path.join("souls").exists());
        assert!(path.join("souls/default.md").exists());
        assert!(path.join("data").exists());
        assert!(tenant_exists("test-123"));

        // Config contains bot name
        let config = std::fs::read_to_string(path.join("config.yaml")).unwrap();
        assert!(config.contains("Test Bot"));

        // Cleanup
        deprovision_tenant("test-123").unwrap();
        assert!(!tenant_exists("test-123"));

        unsafe { std::env::remove_var("CLOUD_TENANTS_DIR") };
    }

    #[test]
    fn test_tenant_db_paths() {
        let mem = tenant_memory_db("abc");
        assert!(mem.to_str().unwrap().contains("tenant-abc"));
        assert!(mem.to_str().unwrap().contains("memory.db"));
    }
}
