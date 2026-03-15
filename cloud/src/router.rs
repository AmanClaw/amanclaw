//! Tenant router — maps slugs to engine instances with lazy start/stop.

use crate::db::{CloudDb, Tenant};
use amanclaw_core::handle::EngineHandle;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;

pub struct TenantState {
    pub tenant: Tenant,
    pub engine: Option<EngineHandle>,
    pub last_active: Instant,
    join: Option<tokio::task::JoinHandle<Result<()>>>,
}

pub struct TenantRouter {
    tenants: HashMap<String, TenantState>,
    db: CloudDb,
}

impl TenantRouter {
    pub fn new(db: CloudDb) -> Self {
        Self {
            tenants: HashMap::new(),
            db,
        }
    }

    /// Get or start a tenant's engine. Returns the EngineHandle.
    pub async fn get_engine(&mut self, slug: &str) -> Result<EngineHandle> {
        // Update last active
        if let Some(state) = self.tenants.get_mut(slug) {
            state.last_active = Instant::now();
            if let Some(ref handle) = state.engine {
                return Ok(handle.clone());
            }
        }

        // Load tenant from DB if not cached
        if !self.tenants.contains_key(slug) {
            let tenant = self
                .db
                .get_tenant_by_slug(slug)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Tenant not found: {slug}"))?;

            if tenant.status != "active" {
                anyhow::bail!("Tenant is {}: {slug}", tenant.status);
            }

            self.tenants.insert(
                slug.to_string(),
                TenantState {
                    tenant,
                    engine: None,
                    last_active: Instant::now(),
                    join: None,
                },
            );
        }

        // Start engine
        let state = self.tenants.get_mut(slug).unwrap();
        let tenant_id = &state.tenant.id;

        let config_path = crate::tenant::tenant_config_path(tenant_id);
        if !config_path.exists() {
            crate::tenant::provision_tenant(tenant_id, &state.tenant.name)?;
        }

        let config_str = std::fs::read_to_string(&config_path)?;
        let config: amanclaw_traits::config::AppConfig = serde_yaml::from_str(&config_str)?;

        // Override DB paths to tenant-specific locations
        let mem_db = crate::tenant::tenant_memory_db(tenant_id);
        unsafe { std::env::set_var("MEMORY_DB_PATH", mem_db.to_str().unwrap()) };

        let islamic_db = crate::tenant::tenant_islamic_db(tenant_id);
        unsafe { std::env::set_var("ISLAMIC_DB_PATH", islamic_db.to_str().unwrap()) };

        tracing::info!(slug, tenant_id, "Starting engine for tenant");
        let result = amanclaw_core::Engine::start(config).await?;

        state.engine = Some(result.handle.clone());
        state.join = Some(result.join);

        self.db.touch_tenant(slug).await.ok();

        Ok(result.handle)
    }

    /// Stop a tenant's engine.
    pub async fn stop_engine(&mut self, slug: &str) -> Result<()> {
        if let Some(state) = self.tenants.get_mut(slug) {
            if let Some(ref handle) = state.engine {
                tracing::info!(slug, "Stopping engine for tenant");
                handle.shutdown().await.ok();
            }
            state.engine = None;
            state.join = None;
        }
        Ok(())
    }

    /// Check if a tenant has a running engine.
    pub fn is_running(&self, slug: &str) -> bool {
        self.tenants
            .get(slug)
            .map(|s| s.engine.is_some())
            .unwrap_or(false)
    }

    /// Stop engines that have been idle for more than `max_idle_secs`.
    pub async fn cleanup_idle(&mut self, max_idle_secs: u64) {
        let idle_slugs: Vec<String> = self
            .tenants
            .iter()
            .filter(|(_, state)| {
                state.engine.is_some()
                    && state.last_active.elapsed().as_secs() > max_idle_secs
            })
            .map(|(slug, _)| slug.clone())
            .collect();

        for slug in idle_slugs {
            tracing::info!(slug, "Stopping idle tenant engine");
            self.stop_engine(&slug).await.ok();
        }
    }

    /// Get tenant info without starting engine.
    pub fn get_tenant_info(&self, slug: &str) -> Option<&Tenant> {
        self.tenants.get(slug).map(|s| &s.tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_tenant_not_found() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let mut router = TenantRouter::new(db);
        let result = router.get_engine("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_router_is_running_false_initially() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let router = TenantRouter::new(db);
        assert!(!router.is_running("any-slug"));
    }
}
