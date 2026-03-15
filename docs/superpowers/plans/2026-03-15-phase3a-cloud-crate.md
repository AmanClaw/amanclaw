# Plan 3A: Cloud Crate + Tenant Management — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `amanclaw-cloud` crate with multi-tenant architecture, tenant routing with lazy engine start/stop, cloud management API (signup, login, tenant CRUD), and invite system — the foundation for AmanClaw Cloud.

**Architecture:** New workspace member `cloud/` contains the cloud binary and crate. Cloud has its own SQLite database (`cloud.db`) for tenant/user/invite management, separate from tenant data. Each tenant gets an isolated directory with their own config, memory DB, and islamic DB. The cloud server wraps Axum with a tenant-aware router that lazily starts/stops Engine instances.

**Tech Stack:** Rust, Axum 0.8, sqlx 0.8 (SQLite), jsonwebtoken 9, tokio, clap 4

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `cloud/Cargo.toml` | CREATE | Cloud binary + library crate |
| `cloud/src/main.rs` | CREATE | Binary entry point, clap CLI |
| `cloud/src/lib.rs` | CREATE | Library root, module declarations |
| `cloud/src/schema.rs` | CREATE | Cloud DB schema (tenants, users, invites) |
| `cloud/src/db.rs` | CREATE | CloudDb struct, pool init, CRUD operations |
| `cloud/src/tenant.rs` | CREATE | Tenant model, directory management, config generation |
| `cloud/src/router.rs` | CREATE | Tenant router: slug → engine, lazy start/stop, idle cleanup |
| `cloud/src/api.rs` | CREATE | Cloud management API routes (signup, login, tenant CRUD) |
| `cloud/src/invite.rs` | CREATE | Invite code generation, validation, CRUD |
| `cloud/src/state.rs` | CREATE | CloudState struct for Axum |
| `rust/Cargo.toml` | MODIFY | Add `../cloud` to workspace members |

---

## Chunk 1: Crate Scaffold + Cloud DB

### Task 1: Create cloud crate skeleton

**Files:**
- Create: `cloud/Cargo.toml`
- Create: `cloud/src/main.rs`
- Create: `cloud/src/lib.rs`
- Modify: `rust/Cargo.toml`

- [ ] **Step 1: Create Cargo.toml**

Create `cloud/Cargo.toml`:

```toml
[package]
name = "amanclaw-cloud"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "AmanClaw Cloud — managed hosting for AmanClaw AI agent"

[[bin]]
name = "amanclaw-cloud"
path = "src/main.rs"

[lib]
name = "amanclaw_cloud"
path = "src/lib.rs"

[dependencies]
amanclaw-core = { path = "../rust/crates/amanclaw-core" }
amanclaw-traits = { path = "../rust/crates/amanclaw-traits" }
amanclaw-api = { path = "../rust/crates/amanclaw-api" }
amanclaw-islamic-db = { path = "../rust/crates/amanclaw-islamic-db" }
amanclaw-security = { path = "../rust/crates/amanclaw-security" }

axum = { version = "0.8", features = ["json"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
jsonwebtoken = "9"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
rand = "0.9"
tower-http = { version = "0.6", features = ["cors", "trace"] }

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

- [ ] **Step 2: Create lib.rs**

Create `cloud/src/lib.rs`:

```rust
pub mod schema;
pub mod db;
pub mod tenant;
pub mod router;
pub mod api;
pub mod invite;
pub mod state;
```

- [ ] **Step 3: Create main.rs stub**

Create `cloud/src/main.rs`:

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "amanclaw-cloud", version, about = "AmanClaw Cloud — managed hosting")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the cloud server
    Serve {
        #[arg(short, long, default_value = "8443")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("amanclaw=info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port } => {
            tracing::info!(port, "Starting AmanClaw Cloud");
            println!("AmanClaw Cloud — not yet implemented");
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Add to workspace**

In `rust/Cargo.toml`, add `"../cloud"` to the workspace members array.

- [ ] **Step 5: Verify compilation**

Run: `cd cloud && cargo check`
Expected: Compiles

- [ ] **Step 6: Commit**

```bash
git add cloud/ rust/Cargo.toml
git commit -m "feat(cloud): scaffold amanclaw-cloud crate"
```

---

### Task 2: Cloud database schema

**Files:**
- Create: `cloud/src/schema.rs`

- [ ] **Step 1: Write schema**

```rust
//! Cloud management database schema.

pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS tenants (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT UNIQUE NOT NULL,
    owner_email TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    plan        TEXT NOT NULL DEFAULT 'beta',
    created_at  TEXT NOT NULL DEFAULT '',
    last_active TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS cloud_users (
    id            TEXT PRIMARY KEY,
    email         TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id     TEXT REFERENCES tenants(id),
    role          TEXT NOT NULL DEFAULT 'owner',
    created_at    TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS invites (
    code       TEXT PRIMARY KEY,
    email      TEXT NOT NULL DEFAULT '',
    used_by    TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT '',
    expires_at TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants(status);
CREATE INDEX IF NOT EXISTS idx_cloud_users_email ON cloud_users(email);
CREATE INDEX IF NOT EXISTS idx_cloud_users_tenant ON cloud_users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_invites_email ON invites(email);
"#;
```

- [ ] **Step 2: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_schema_creates_all_tables() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(INIT_SQL).execute(&pool).await.unwrap();

        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        assert!(names.contains(&"tenants"));
        assert!(names.contains(&"cloud_users"));
        assert!(names.contains(&"invites"));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd cloud && cargo test schema -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add cloud/src/schema.rs
git commit -m "feat(cloud): add cloud database schema for tenants, users, invites"
```

---

### Task 3: CloudDb struct with CRUD operations

**Files:**
- Create: `cloud/src/db.rs`

- [ ] **Step 1: Implement CloudDb**

```rust
//! Cloud database operations.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::FromRow;

#[derive(Clone)]
pub struct CloudDb {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub owner_email: String,
    pub status: String,
    pub plan: String,
    pub created_at: String,
    pub last_active: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CloudUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub tenant_id: Option<String>,
    pub role: String,
    pub created_at: String,
}

impl CloudDb {
    pub async fn new(db_path: &str) -> Result<Self> {
        let url = if db_path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{db_path}?mode=rwc")
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        sqlx::raw_sql(crate::schema::INIT_SQL)
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // --- Tenant CRUD ---

    pub async fn create_tenant(&self, name: &str, slug: &str, owner_email: &str) -> Result<Tenant> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO tenants (id, name, slug, owner_email, status, plan, created_at, last_active) VALUES (?, ?, ?, ?, 'active', 'beta', ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(slug)
        .bind(owner_email)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(Tenant {
            id,
            name: name.to_string(),
            slug: slug.to_string(),
            owner_email: owner_email.to_string(),
            status: "active".to_string(),
            plan: "beta".to_string(),
            created_at: now.clone(),
            last_active: now,
        })
    }

    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, owner_email, status, plan, created_at, last_active FROM tenants WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tenant)
    }

    pub async fn get_tenant(&self, id: &str) -> Result<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, owner_email, status, plan, created_at, last_active FROM tenants WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tenant)
    }

    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let tenants = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, owner_email, status, plan, created_at, last_active FROM tenants ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tenants)
    }

    pub async fn update_tenant_status(&self, id: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE tenants SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn touch_tenant(&self, slug: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE tenants SET last_active = ? WHERE slug = ?")
            .bind(&now)
            .bind(slug)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- User CRUD ---

    pub async fn create_user(&self, email: &str, password_hash: &str, tenant_id: &str) -> Result<CloudUser> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO cloud_users (id, email, password_hash, tenant_id, role, created_at) VALUES (?, ?, ?, ?, 'owner', ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(password_hash)
        .bind(tenant_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(CloudUser {
            id,
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            tenant_id: Some(tenant_id.to_string()),
            role: "owner".to_string(),
            created_at: now,
        })
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<CloudUser>> {
        let user = sqlx::query_as::<_, CloudUser>(
            "SELECT id, email, password_hash, tenant_id, role, created_at FROM cloud_users WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_tenant() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let tenant = db.create_tenant("My Bot", "my-bot", "user@example.com").await.unwrap();
        assert_eq!(tenant.slug, "my-bot");
        assert_eq!(tenant.status, "active");

        let found = db.get_tenant_by_slug("my-bot").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "My Bot");
    }

    #[tokio::test]
    async fn test_tenant_not_found() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let found = db.get_tenant_by_slug("nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_tenants() {
        let db = CloudDb::new(":memory:").await.unwrap();
        db.create_tenant("Bot 1", "bot-1", "a@example.com").await.unwrap();
        db.create_tenant("Bot 2", "bot-2", "b@example.com").await.unwrap();
        let list = db.list_tenants().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_update_tenant_status() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let tenant = db.create_tenant("Bot", "bot", "a@example.com").await.unwrap();
        db.update_tenant_status(&tenant.id, "suspended").await.unwrap();
        let found = db.get_tenant(&tenant.id).await.unwrap().unwrap();
        assert_eq!(found.status, "suspended");
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let tenant = db.create_tenant("Bot", "bot", "user@example.com").await.unwrap();
        let user = db.create_user("user@example.com", "hashed_pw", &tenant.id).await.unwrap();
        assert_eq!(user.role, "owner");

        let found = db.get_user_by_email("user@example.com").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_slug_uniqueness() {
        let db = CloudDb::new(":memory:").await.unwrap();
        db.create_tenant("Bot 1", "same-slug", "a@example.com").await.unwrap();
        let result = db.create_tenant("Bot 2", "same-slug", "b@example.com").await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd cloud && cargo test db -- --nocapture`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add cloud/src/db.rs
git commit -m "feat(cloud): add CloudDb with tenant and user CRUD operations"
```

---

## Chunk 2: Invite System + Tenant Directory

### Task 4: Invite code management

**Files:**
- Create: `cloud/src/invite.rs`

- [ ] **Step 1: Implement invite system**

```rust
//! Invite code generation and validation.

use anyhow::Result;
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invite {
    pub code: String,
    pub email: String,
    pub used_by: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Generate a random 8-character invite code.
fn generate_code() -> String {
    let mut rng = rand::rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..8).map(|_| chars[rng.random_range(0..chars.len())]).collect()
}

/// Create a new invite code.
pub async fn create_invite(pool: &SqlitePool, email: &str, days_valid: i64) -> Result<Invite> {
    let code = generate_code();
    let now = Utc::now();
    let expires = now + Duration::days(days_valid);

    sqlx::query(
        "INSERT INTO invites (code, email, used_by, created_at, expires_at) VALUES (?, ?, '', ?, ?)",
    )
    .bind(&code)
    .bind(email)
    .bind(now.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(Invite {
        code,
        email: email.to_string(),
        used_by: String::new(),
        created_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    })
}

/// Validate an invite code. Returns the invite if valid and unused.
pub async fn validate_invite(pool: &SqlitePool, code: &str) -> Result<Option<Invite>> {
    let invite = sqlx::query_as::<_, Invite>(
        "SELECT code, email, used_by, created_at, expires_at FROM invites WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    match invite {
        Some(inv) if !inv.used_by.is_empty() => Ok(None), // Already used
        Some(inv) => {
            // Check expiration
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&inv.expires_at) {
                if Utc::now() > expires {
                    return Ok(None); // Expired
                }
            }
            Ok(Some(inv))
        }
        None => Ok(None),
    }
}

/// Mark an invite as used.
pub async fn use_invite(pool: &SqlitePool, code: &str, user_id: &str) -> Result<()> {
    sqlx::query("UPDATE invites SET used_by = ? WHERE code = ?")
        .bind(user_id)
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all invites.
pub async fn list_invites(pool: &SqlitePool) -> Result<Vec<Invite>> {
    let invites = sqlx::query_as::<_, Invite>(
        "SELECT code, email, used_by, created_at, expires_at FROM invites ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(invites)
}

/// Revoke (delete) an invite.
pub async fn revoke_invite(pool: &SqlitePool, code: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM invites WHERE code = ?")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(crate::schema::INIT_SQL)
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        assert_eq!(invite.code.len(), 8);
        assert_eq!(invite.email, "user@example.com");
        assert!(invite.used_by.is_empty());
    }

    #[tokio::test]
    async fn test_validate_unused_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        let valid = validate_invite(&pool, &invite.code).await.unwrap();
        assert!(valid.is_some());
    }

    #[tokio::test]
    async fn test_validate_used_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        use_invite(&pool, &invite.code, "user-123").await.unwrap();
        let valid = validate_invite(&pool, &invite.code).await.unwrap();
        assert!(valid.is_none());
    }

    #[tokio::test]
    async fn test_validate_nonexistent_invite() {
        let pool = setup_pool().await;
        let valid = validate_invite(&pool, "INVALID").await.unwrap();
        assert!(valid.is_none());
    }

    #[tokio::test]
    async fn test_list_invites() {
        let pool = setup_pool().await;
        create_invite(&pool, "a@example.com", 7).await.unwrap();
        create_invite(&pool, "b@example.com", 7).await.unwrap();
        let list = list_invites(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_revoke_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        assert!(revoke_invite(&pool, &invite.code).await.unwrap());
        let valid = validate_invite(&pool, &invite.code).await.unwrap();
        assert!(valid.is_none());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd cloud && cargo test invite -- --nocapture`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add cloud/src/invite.rs
git commit -m "feat(cloud): add invite code generation, validation, and management"
```

---

### Task 5: Tenant directory management

**Files:**
- Create: `cloud/src/tenant.rs`

- [ ] **Step 1: Implement tenant directory setup**

```rust
//! Tenant directory and config management.

use anyhow::Result;
use std::path::{Path, PathBuf};

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
        std::env::set_var("CLOUD_TENANTS_DIR", dir.path().to_str().unwrap());

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

        std::env::remove_var("CLOUD_TENANTS_DIR");
    }

    #[test]
    fn test_tenant_db_paths() {
        let mem = tenant_memory_db("abc");
        assert!(mem.to_str().unwrap().contains("tenant-abc"));
        assert!(mem.to_str().unwrap().contains("memory.db"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd cloud && cargo test tenant -- --nocapture`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add cloud/src/tenant.rs
git commit -m "feat(cloud): add tenant directory provisioning and config generation"
```

---

## Chunk 3: Tenant Router + Cloud State

### Task 6: Cloud state

**Files:**
- Create: `cloud/src/state.rs`

- [ ] **Step 1: Implement CloudState**

```rust
//! Cloud server state shared across Axum handlers.

use crate::db::CloudDb;
use crate::router::TenantRouter;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CloudState {
    pub db: CloudDb,
    pub router: Arc<RwLock<TenantRouter>>,
    pub jwt_secret: String,
}

impl CloudState {
    pub fn new(db: CloudDb, router: TenantRouter, jwt_secret: String) -> Self {
        Self {
            db,
            router: Arc::new(RwLock::new(router)),
            jwt_secret,
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add cloud/src/state.rs
git commit -m "feat(cloud): add CloudState for Axum handlers"
```

---

### Task 7: Tenant router with lazy engine start/stop

**Files:**
- Create: `cloud/src/router.rs`

- [ ] **Step 1: Implement TenantRouter**

```rust
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
        let mut config: amanclaw_traits::config::AppConfig = serde_yaml::from_str(&config_str)?;

        // Override DB paths to tenant-specific locations
        let mem_db = crate::tenant::tenant_memory_db(tenant_id);
        std::env::set_var("MEMORY_DB_PATH", mem_db.to_str().unwrap());

        let islamic_db = crate::tenant::tenant_islamic_db(tenant_id);
        std::env::set_var("ISLAMIC_DB_PATH", islamic_db.to_str().unwrap());

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
```

- [ ] **Step 2: Run tests**

Run: `cd cloud && cargo test router -- --nocapture`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add cloud/src/router.rs
git commit -m "feat(cloud): add TenantRouter with lazy engine start/stop and idle cleanup"
```

---

## Chunk 4: Cloud API

### Task 8: Cloud management API

**Files:**
- Create: `cloud/src/api.rs`

- [ ] **Step 1: Implement cloud API routes**

```rust
//! Cloud management API routes.

use crate::state::CloudState;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,      // user ID
    tenant: String,   // tenant slug
    exp: usize,
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub bot_name: String,
    pub invite_code: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Build the cloud API router.
pub fn cloud_router(state: CloudState) -> Router {
    Router::new()
        .route("/api/cloud/signup", post(signup))
        .route("/api/cloud/login", post(login))
        .route("/api/cloud/tenant", get(get_tenant))
        .route("/api/cloud/tenant/status", get(tenant_status))
        .with_state(state)
}

async fn signup(
    State(state): State<CloudState>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validate invite
    let invite = crate::invite::validate_invite(state.db.pool(), &req.invite_code)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Check email not taken
    if state.db.get_user_by_email(&req.email).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    // Generate slug from bot name
    let slug: String = req.bot_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if slug.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check slug not taken
    if state.db.get_tenant_by_slug(&slug).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    // Create tenant
    let tenant = state
        .db
        .create_tenant(&req.bot_name, &slug, &req.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create user (plain password for MVP — add hashing later)
    let user = state
        .db
        .create_user(&req.email, &req.password, &tenant.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Use invite
    crate::invite::use_invite(state.db.pool(), &req.invite_code, &user.id)
        .await
        .ok();

    // Provision tenant directory
    crate::tenant::provision_tenant(&tenant.id, &tenant.name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate JWT
    let token = create_jwt(&state.jwt_secret, &user.id, &slug)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tenant": {
            "slug": slug,
            "name": tenant.name,
        },
        "token": token,
    })))
}

async fn login(
    State(state): State<CloudState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = state
        .db
        .get_user_by_email(&req.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.password_hash != req.password {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let tenant_id = user.tenant_id.as_deref().ok_or(StatusCode::UNAUTHORIZED)?;
    let tenant = state
        .db
        .get_tenant(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = create_jwt(&state.jwt_secret, &user.id, &tenant.slug)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tenant": {
            "slug": tenant.slug,
            "name": tenant.name,
        },
        "token": token,
    })))
}

async fn get_tenant(
    State(state): State<CloudState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // For MVP: return first tenant (proper auth extraction comes later)
    let tenants = state.db.list_tenants().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tenant = tenants.first().ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "tenant": tenant,
    })))
}

async fn tenant_status(
    State(state): State<CloudState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tenants = state.db.list_tenants().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tenant = tenants.first().ok_or(StatusCode::NOT_FOUND)?;
    let running = state.router.read().await.is_running(&tenant.slug);

    Ok(Json(serde_json::json!({
        "slug": tenant.slug,
        "status": tenant.status,
        "engine_running": running,
    })))
}

fn create_jwt(secret: &str, user_id: &str, slug: &str) -> Result<String, StatusCode> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tenant: slug.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_jwt() {
        let token = create_jwt("test-secret", "user-1", "my-bot").unwrap();
        assert!(!token.is_empty());

        // Validate it
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("test-secret".as_bytes()),
            &Validation::default(),
        )
        .unwrap();
        assert_eq!(decoded.claims.sub, "user-1");
        assert_eq!(decoded.claims.tenant, "my-bot");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd cloud && cargo test api -- --nocapture`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add cloud/src/api.rs
git commit -m "feat(cloud): add cloud management API (signup, login, tenant status)"
```

---

## Chunk 5: Cloud Binary + CLI

### Task 9: Wire up cloud server binary

**Files:**
- Modify: `cloud/src/main.rs`

- [ ] **Step 1: Implement serve and invite commands**

```rust
use amanclaw_cloud::{api, db::CloudDb, invite, router::TenantRouter, state::CloudState};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "amanclaw-cloud", version, about = "AmanClaw Cloud — managed hosting")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the cloud server
    Serve {
        #[arg(short, long, default_value = "8443")]
        port: u16,
        #[arg(long, default_value = "cloud/cloud.db")]
        db_path: String,
    },
    /// Manage invite codes
    Invite {
        #[command(subcommand)]
        action: InviteAction,
    },
    /// Manage tenants
    Tenant {
        #[command(subcommand)]
        action: TenantAction,
    },
}

#[derive(Subcommand)]
enum InviteAction {
    /// Create a new invite code
    Create {
        #[arg(long)]
        email: String,
        #[arg(long, default_value = "30")]
        days: i64,
    },
    /// List all invite codes
    List,
    /// Revoke an invite code
    Revoke { code: String },
}

#[derive(Subcommand)]
enum TenantAction {
    /// List all tenants
    List,
    /// Show tenant info
    Info { slug: String },
    /// Suspend a tenant
    Suspend { slug: String },
    /// Delete a tenant
    Delete { slug: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("amanclaw=info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, db_path } => {
            // Ensure parent directory exists
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let db = CloudDb::new(&db_path).await?;
            let router = TenantRouter::new(db.clone());
            let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                use rand::Rng;
                rand::rng()
                    .sample_iter(&rand::distr::Alphanumeric)
                    .take(64)
                    .map(char::from)
                    .collect()
            });

            let state = CloudState::new(db, router, jwt_secret);

            // Spawn idle cleanup task
            let cleanup_state = state.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                    cleanup_state.router.write().await.cleanup_idle(1800).await;
                }
            });

            let app = api::cloud_router(state);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            tracing::info!(port, "AmanClaw Cloud server listening");
            axum::serve(listener, app).await?;
            Ok(())
        }
        Commands::Invite { action } => {
            let db = CloudDb::new("cloud/cloud.db").await?;
            match action {
                InviteAction::Create { email, days } => {
                    let inv = invite::create_invite(db.pool(), &email, days).await?;
                    println!("Invite created:");
                    println!("  Code:    {}", inv.code);
                    println!("  Email:   {}", inv.email);
                    println!("  Expires: {}", inv.expires_at);
                }
                InviteAction::List => {
                    let invites = invite::list_invites(db.pool()).await?;
                    if invites.is_empty() {
                        println!("No invites.");
                        return Ok(());
                    }
                    println!("{:<10} {:<30} {:<10} {:<25}", "Code", "Email", "Used", "Expires");
                    println!("{}", "-".repeat(77));
                    for inv in &invites {
                        let used = if inv.used_by.is_empty() { "no" } else { "yes" };
                        let expires = inv.expires_at.chars().take(19).collect::<String>();
                        println!("{:<10} {:<30} {:<10} {:<25}", inv.code, inv.email, used, expires);
                    }
                }
                InviteAction::Revoke { code } => {
                    if invite::revoke_invite(db.pool(), &code).await? {
                        println!("Invite {code} revoked.");
                    } else {
                        println!("Invite {code} not found.");
                    }
                }
            }
            Ok(())
        }
        Commands::Tenant { action } => {
            let db = CloudDb::new("cloud/cloud.db").await?;
            match action {
                TenantAction::List => {
                    let tenants = db.list_tenants().await?;
                    if tenants.is_empty() {
                        println!("No tenants.");
                        return Ok(());
                    }
                    println!("{:<20} {:<20} {:<10} {:<10}", "Slug", "Name", "Status", "Plan");
                    println!("{}", "-".repeat(62));
                    for t in &tenants {
                        println!("{:<20} {:<20} {:<10} {:<10}", t.slug, t.name, t.status, t.plan);
                    }
                }
                TenantAction::Info { slug } => {
                    match db.get_tenant_by_slug(&slug).await? {
                        Some(t) => {
                            println!("Tenant: {}", t.name);
                            println!("  Slug:    {}", t.slug);
                            println!("  Email:   {}", t.owner_email);
                            println!("  Status:  {}", t.status);
                            println!("  Plan:    {}", t.plan);
                            println!("  Created: {}", t.created_at);
                            println!("  Active:  {}", t.last_active);
                        }
                        None => println!("Tenant '{slug}' not found."),
                    }
                }
                TenantAction::Suspend { slug } => {
                    match db.get_tenant_by_slug(&slug).await? {
                        Some(t) => {
                            db.update_tenant_status(&t.id, "suspended").await?;
                            println!("Tenant '{slug}' suspended.");
                        }
                        None => println!("Tenant '{slug}' not found."),
                    }
                }
                TenantAction::Delete { slug } => {
                    match db.get_tenant_by_slug(&slug).await? {
                        Some(t) => {
                            db.update_tenant_status(&t.id, "deleted").await?;
                            crate::amanclaw_cloud::tenant::deprovision_tenant(&t.id)?;
                            println!("Tenant '{slug}' deleted.");
                        }
                        None => println!("Tenant '{slug}' not found."),
                    }
                }
            }
            Ok(())
        }
    }
}
```

Note: The `Delete` command uses `amanclaw_cloud::tenant::deprovision_tenant` — adjust the import path to match how the crate is structured. The binary is in the same crate so use `amanclaw_cloud::tenant::deprovision_tenant`.

- [ ] **Step 2: Verify compilation**

Run: `cd cloud && cargo check`
Expected: Compiles

- [ ] **Step 3: Test help output**

Run: `cd cloud && cargo run -- --help`
Expected: Shows serve, invite, tenant commands

- [ ] **Step 4: Commit**

```bash
git add cloud/src/main.rs
git commit -m "feat(cloud): wire up cloud server binary with serve, invite, and tenant commands"
```

---

## Summary

| Task | Description | Steps |
|------|-------------|-------|
| 1 | Crate skeleton | 6 |
| 2 | Cloud DB schema | 4 |
| 3 | CloudDb CRUD operations | 3 |
| 4 | Invite code management | 3 |
| 5 | Tenant directory provisioning | 3 |
| 6 | CloudState struct | 2 |
| 7 | TenantRouter with lazy start/stop | 3 |
| 8 | Cloud management API | 3 |
| 9 | Cloud binary with CLI | 4 |

**Total: 9 tasks, 31 steps**

After completing this plan:
```bash
# Start cloud server
amanclaw-cloud serve --port 8443

# Manage invites
amanclaw-cloud invite create --email user@example.com
amanclaw-cloud invite list
amanclaw-cloud invite revoke CODE

# Manage tenants
amanclaw-cloud tenant list
amanclaw-cloud tenant info my-bot
amanclaw-cloud tenant suspend my-bot
amanclaw-cloud tenant delete my-bot

# API
POST /api/cloud/signup  { email, password, bot_name, invite_code }
POST /api/cloud/login   { email, password }
GET  /api/cloud/tenant
GET  /api/cloud/tenant/status
```
