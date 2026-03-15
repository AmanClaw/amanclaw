# User Management Dashboard Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist user auth state to SQLite, add rich user management API endpoints, and enhance the existing Svelte dashboard with user detail views, filters, and conversation history.

**Architecture:** New `users` table in SQLite with write-through cache in `Auth`. New API endpoints for user detail/history/stats. Enhanced Svelte dashboard pages for user management.

**Tech Stack:** Rust (sqlx, axum), Svelte 5 + Tailwind CSS 4 + Vite (existing dashboard at `dashboard/`)

**Key discovery:** A Svelte dashboard already exists at `dashboard/` (NOT React). It has Login, Users list, Sidebar, and is served at `/admin` via `include_dir`. We extend it instead of building from scratch.

---

## Chunk 1: SQLite User Persistence + Auth Rewrite

### Task 1: Add `users` table to SQLite schema

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/schema.rs`

- [ ] **Step 1: Add `users` table to `INIT_SQL`**

In `rust/crates/amanclaw-memory/src/schema.rs`, add the `users` table and index after the `community_admins` table (before `vector_documents`):

```sql
CREATE TABLE IF NOT EXISTS users (
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    username TEXT,
    first_name TEXT,
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, platform)
);

CREATE INDEX IF NOT EXISTS idx_users_state ON users(state);
CREATE INDEX IF NOT EXISTS idx_users_platform ON users(platform);
```

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo check -p amanclaw-memory`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-memory/src/schema.rs
git commit -m "feat: add users table to SQLite schema for persistent auth state"
```

### Task 2: Add user CRUD methods to SqliteMemory

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/sqlite.rs`

- [ ] **Step 1: Write tests for user CRUD**

Add to the `tests` module in `rust/crates/amanclaw-memory/src/sqlite.rs`:

```rust
#[tokio::test]
async fn test_upsert_user() {
    let mem = make_memory().await;
    mem.upsert_user("123", "telegram", "pending", Some("aman"), Some("Aman"))
        .await
        .unwrap();
    let user = mem.get_user("123", "telegram").await.unwrap().unwrap();
    assert_eq!(user.state, "pending");
    assert_eq!(user.username.as_deref(), Some("aman"));
}

#[tokio::test]
async fn test_update_user_state() {
    let mem = make_memory().await;
    mem.upsert_user("123", "telegram", "pending", None, None)
        .await
        .unwrap();
    mem.update_user_state("123", "telegram", "approved")
        .await
        .unwrap();
    let user = mem.get_user("123", "telegram").await.unwrap().unwrap();
    assert_eq!(user.state, "approved");
}

#[tokio::test]
async fn test_list_users_with_filters() {
    let mem = make_memory().await;
    mem.upsert_user("1", "telegram", "pending", None, None).await.unwrap();
    mem.upsert_user("2", "discord", "approved", None, None).await.unwrap();
    mem.upsert_user("3", "telegram", "approved", None, None).await.unwrap();

    let all = mem.list_users(None, None, None).await.unwrap();
    assert_eq!(all.len(), 3);

    let telegram = mem.list_users(Some("telegram"), None, None).await.unwrap();
    assert_eq!(telegram.len(), 2);

    let approved = mem.list_users(None, Some("approved"), None).await.unwrap();
    assert_eq!(approved.len(), 2);
}

#[tokio::test]
async fn test_touch_user_last_seen() {
    let mem = make_memory().await;
    mem.upsert_user("123", "telegram", "approved", None, None)
        .await
        .unwrap();
    mem.touch_user_last_seen("123", "telegram").await.unwrap();
    let user = mem.get_user("123", "telegram").await.unwrap().unwrap();
    assert!(user.last_seen.is_some());
}

#[tokio::test]
async fn test_user_stats() {
    let mem = make_memory().await;
    mem.upsert_user("1", "telegram", "pending", None, None).await.unwrap();
    mem.upsert_user("2", "telegram", "approved", None, None).await.unwrap();
    mem.upsert_user("3", "discord", "approved", None, None).await.unwrap();
    mem.upsert_user("4", "slack", "blocked", None, None).await.unwrap();

    let stats = mem.get_user_stats().await.unwrap();
    assert_eq!(stats.total, 4);
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.approved, 2);
    assert_eq!(stats.blocked, 1);
    assert_eq!(stats.by_platform.get("telegram"), Some(&2));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && cargo test -p amanclaw-memory -- test_upsert_user test_update_user_state test_list_users test_touch_user test_user_stats 2>&1 | tail -5`
Expected: compilation errors (methods don't exist yet)

- [ ] **Step 3: Add `UserRow` and `UserStats` structs and implement CRUD methods**

Add above the `impl SqliteMemory` block in `rust/crates/amanclaw-memory/src/sqlite.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserRow {
    pub user_id: String,
    pub platform: String,
    pub state: String,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UserStats {
    pub total: i64,
    pub pending: i64,
    pub approved: i64,
    pub blocked: i64,
    pub by_platform: std::collections::HashMap<String, i64>,
}
```

Add these methods inside the `impl SqliteMemory` block, after the `delete_fact` method:

```rust
// --- User management ---

pub async fn upsert_user(
    &self,
    user_id: &str,
    platform: &str,
    state: &str,
    username: Option<&str>,
    first_name: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO users (user_id, platform, state, username, first_name)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id, platform) DO UPDATE SET
           username = COALESCE(excluded.username, users.username),
           first_name = COALESCE(excluded.first_name, users.first_name),
           last_seen = CURRENT_TIMESTAMP"
    )
    .bind(user_id).bind(platform).bind(state)
    .bind(username).bind(first_name)
    .execute(&self.pool).await?;
    Ok(())
}

pub async fn get_user(&self, user_id: &str, platform: &str) -> Result<Option<UserRow>> {
    let row = sqlx::query(
        "SELECT user_id, platform, state, username, first_name, first_seen, last_seen
         FROM users WHERE user_id = ? AND platform = ?"
    )
    .bind(user_id).bind(platform)
    .fetch_optional(&self.pool).await?;

    Ok(row.map(|r| UserRow {
        user_id: r.get("user_id"),
        platform: r.get("platform"),
        state: r.get("state"),
        username: r.get("username"),
        first_name: r.get("first_name"),
        first_seen: r.get("first_seen"),
        last_seen: r.get("last_seen"),
    }))
}

pub async fn update_user_state(&self, user_id: &str, platform: &str, state: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE users SET state = ?, last_seen = CURRENT_TIMESTAMP WHERE user_id = ? AND platform = ?"
    )
    .bind(state).bind(user_id).bind(platform)
    .execute(&self.pool).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_users(
    &self,
    platform: Option<&str>,
    state: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<UserRow>> {
    let mut sql = "SELECT user_id, platform, state, username, first_name, first_seen, last_seen FROM users WHERE 1=1".to_string();
    let mut binds: Vec<String> = Vec::new();

    if let Some(p) = platform {
        sql.push_str(" AND platform = ?");
        binds.push(p.to_string());
    }
    if let Some(s) = state {
        sql.push_str(" AND state = ?");
        binds.push(s.to_string());
    }
    if let Some(q) = search {
        sql.push_str(" AND (user_id LIKE ? OR username LIKE ? OR first_name LIKE ?)");
        let pattern = format!("%{q}%");
        binds.push(pattern.clone());
        binds.push(pattern.clone());
        binds.push(pattern);
    }
    sql.push_str(" ORDER BY last_seen DESC");

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    let rows = query.fetch_all(&self.pool).await?;
    Ok(rows.iter().map(|r| UserRow {
        user_id: r.get("user_id"),
        platform: r.get("platform"),
        state: r.get("state"),
        username: r.get("username"),
        first_name: r.get("first_name"),
        first_seen: r.get("first_seen"),
        last_seen: r.get("last_seen"),
    }).collect())
}

pub async fn touch_user_last_seen(&self, user_id: &str, platform: &str) -> Result<()> {
    sqlx::query("UPDATE users SET last_seen = CURRENT_TIMESTAMP WHERE user_id = ? AND platform = ?")
        .bind(user_id).bind(platform)
        .execute(&self.pool).await?;
    Ok(())
}

pub async fn get_user_stats(&self) -> Result<UserStats> {
    let total: i64 = sqlx::query("SELECT COUNT(*) as c FROM users")
        .fetch_one(&self.pool).await?.get("c");
    let pending: i64 = sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'pending'")
        .fetch_one(&self.pool).await?.get("c");
    let approved: i64 = sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'approved'")
        .fetch_one(&self.pool).await?.get("c");
    let blocked: i64 = sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'blocked'")
        .fetch_one(&self.pool).await?.get("c");

    let platform_rows = sqlx::query("SELECT platform, COUNT(*) as c FROM users GROUP BY platform")
        .fetch_all(&self.pool).await?;
    let by_platform: std::collections::HashMap<String, i64> = platform_rows.iter()
        .map(|r| (r.get::<String, _>("platform"), r.get::<i64, _>("c")))
        .collect();

    Ok(UserStats { total, pending, approved, blocked, by_platform })
}

pub async fn get_history_paginated(
    &self,
    ns: &str,
    user_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryMessage>> {
    let rows = sqlx::query(
        "SELECT role, content FROM messages WHERE namespace = ? AND user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
    )
    .bind(ns).bind(user_id).bind(limit).bind(offset)
    .fetch_all(&self.pool).await?;

    let mut messages: Vec<HistoryMessage> = rows.iter().map(|row| HistoryMessage {
        role: row.get("role"),
        content: row.get("content"),
    }).collect();
    messages.reverse();
    Ok(messages)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && cargo test -p amanclaw-memory -- test_upsert_user test_update_user_state test_list_users test_touch_user test_user_stats 2>&1 | tail -10`
Expected: all 5 tests pass

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-memory/src/sqlite.rs
git commit -m "feat: add user CRUD methods to SqliteMemory"
```

### Task 3: Rewrite Auth to use SqlitePool

**Files:**
- Modify: `rust/crates/amanclaw-security/Cargo.toml`
- Modify: `rust/crates/amanclaw-security/src/auth.rs`

- [ ] **Step 1: Add sqlx dependency to amanclaw-security**

Add to `[dependencies]` in `rust/crates/amanclaw-security/Cargo.toml`:

```toml
sqlx = { workspace = true }
```

Also add to `[dev-dependencies]`:

```toml
sqlx = { workspace = true }
```

- [ ] **Step 2: Write tests for SQLite-backed Auth**

Replace the entire test module in `rust/crates/amanclaw-security/src/auth.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth() -> Auth {
        let mut admin_users = HashMap::new();
        admin_users.insert("telegram".into(), vec!["12345".into()]);
        Auth::new(admin_users)
    }

    #[test]
    fn test_admin_user_is_authorized() {
        let auth = make_auth();
        assert_eq!(auth.get_user_state("12345", "telegram"), UserState::Admin);
    }

    #[test]
    fn test_unknown_user_is_new() {
        let auth = make_auth();
        assert_eq!(auth.get_user_state("99999", "telegram"), UserState::New);
    }

    #[test]
    fn test_register_and_approve() {
        let mut auth = make_auth();
        auth.register_user("55555", "telegram", None, None);
        assert_eq!(auth.get_user_state("55555", "telegram"), UserState::Pending);

        auth.approve_user("55555", "telegram");
        assert_eq!(auth.get_user_state("55555", "telegram"), UserState::Approved);
    }

    #[test]
    fn test_block_user() {
        let mut auth = make_auth();
        auth.register_user("66666", "telegram", None, None);
        auth.block_user("66666", "telegram");
        assert_eq!(auth.get_user_state("66666", "telegram"), UserState::Blocked);
    }

    #[test]
    fn test_list_users() {
        let mut auth = make_auth();
        auth.register_user("111", "telegram", Some("user1"), Some("User"));
        auth.register_user("222", "discord", None, None);
        let users = auth.list_users();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_unblock_resets_to_pending() {
        let mut auth = make_auth();
        auth.register_user("77777", "telegram", None, None);
        auth.block_user("77777", "telegram");
        auth.unblock_user("77777", "telegram");
        assert_eq!(auth.get_user_state("77777", "telegram"), UserState::Pending);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd rust && cargo test -p amanclaw-security -- 2>&1 | tail -10`
Expected: compilation errors (signature changes)

- [ ] **Step 4: Rewrite Auth struct**

Replace the entire `Auth` impl in `rust/crates/amanclaw-security/src/auth.rs` (keep `UserState` enum and its `Display` impl as-is):

```rust
pub struct Auth {
    admin_users: HashMap<String, Vec<String>>,
    registered: HashMap<(String, String), UserState>,
    pool: Option<sqlx::SqlitePool>,
}

impl Auth {
    /// Create Auth without SQLite (for tests or in-memory use).
    pub fn new(admin_users: HashMap<String, Vec<String>>) -> Self {
        Self {
            admin_users,
            registered: HashMap::new(),
            pool: None,
        }
    }

    /// Create Auth backed by SQLite. Loads existing users on startup.
    pub async fn with_pool(
        admin_users: HashMap<String, Vec<String>>,
        pool: sqlx::SqlitePool,
    ) -> Self {
        let mut registered = HashMap::new();

        // Load all users from SQLite into the in-memory cache
        if let Ok(rows) = sqlx::query("SELECT user_id, platform, state FROM users")
            .fetch_all(&pool)
            .await
        {
            for row in rows {
                let uid: String = sqlx::Row::get(&row, "user_id");
                let plat: String = sqlx::Row::get(&row, "platform");
                let state_str: String = sqlx::Row::get(&row, "state");
                let state = match state_str.as_str() {
                    "approved" => UserState::Approved,
                    "blocked" => UserState::Blocked,
                    _ => UserState::Pending,
                };
                registered.insert((uid, plat), state);
            }
            tracing::info!(count = registered.len(), "Loaded users from SQLite");
        }

        Self {
            admin_users,
            registered,
            pool: Some(pool),
        }
    }

    pub fn get_user_state(&self, user_id: &str, platform: &str) -> UserState {
        if let Some(admins) = self.admin_users.get(platform)
            && admins.iter().any(|id| id == user_id)
        {
            return UserState::Admin;
        }
        let key = (user_id.to_string(), platform.to_string());
        self.registered.get(&key).cloned().unwrap_or(UserState::New)
    }

    pub fn register_user(
        &mut self,
        user_id: &str,
        platform: &str,
        username: Option<&str>,
        first_name: Option<&str>,
    ) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.entry(key).or_insert(UserState::Pending);

        // Write-through to SQLite
        if let Some(pool) = &self.pool {
            let pool = pool.clone();
            let uid = user_id.to_string();
            let plat = platform.to_string();
            let uname = username.map(|s| s.to_string());
            let fname = first_name.map(|s| s.to_string());
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "INSERT INTO users (user_id, platform, state, username, first_name)
                     VALUES (?, ?, 'pending', ?, ?)
                     ON CONFLICT(user_id, platform) DO UPDATE SET
                       username = COALESCE(excluded.username, users.username),
                       first_name = COALESCE(excluded.first_name, users.first_name),
                       last_seen = CURRENT_TIMESTAMP"
                )
                .bind(&uid).bind(&plat).bind(&uname).bind(&fname)
                .execute(&pool).await;
            });
        }
    }

    pub fn approve_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Approved);
        self.persist_state(user_id, platform, "approved");
    }

    pub fn block_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Blocked);
        self.persist_state(user_id, platform, "blocked");
    }

    pub fn unblock_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Pending);
        self.persist_state(user_id, platform, "pending");
    }

    pub fn touch_last_seen(&self, user_id: &str, platform: &str) {
        if let Some(pool) = &self.pool {
            let pool = pool.clone();
            let uid = user_id.to_string();
            let plat = platform.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE users SET last_seen = CURRENT_TIMESTAMP WHERE user_id = ? AND platform = ?"
                )
                .bind(&uid).bind(&plat)
                .execute(&pool).await;
            });
        }
    }

    pub fn list_users(&self) -> Vec<(String, String, UserState)> {
        self.registered
            .iter()
            .map(|((uid, plat), state)| (uid.clone(), plat.clone(), state.clone()))
            .collect()
    }

    fn persist_state(&self, user_id: &str, platform: &str, state: &str) {
        if let Some(pool) = &self.pool {
            let pool = pool.clone();
            let uid = user_id.to_string();
            let plat = platform.to_string();
            let st = state.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE users SET state = ? WHERE user_id = ? AND platform = ?"
                )
                .bind(&st).bind(&uid).bind(&plat)
                .execute(&pool).await;
            });
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd rust && cargo test -p amanclaw-security -- 2>&1 | tail -10`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add rust/crates/amanclaw-security/Cargo.toml rust/crates/amanclaw-security/src/auth.rs
git commit -m "feat: rewrite Auth with SQLite write-through cache and unblock support"
```

### Task 4: Update AuthMiddleware to capture user info and update last_seen

**Files:**
- Modify: `rust/crates/amanclaw-core/src/middleware/auth.rs`

- [ ] **Step 1: Update AuthMiddleware to pass username/first_name and touch last_seen**

Replace the `UserState::New` arm and add `touch_last_seen` for known users in `rust/crates/amanclaw-core/src/middleware/auth.rs`:

```rust
UserState::New => {
    self.auth.write().await.register_user(
        user_id,
        platform,
        ctx.msg.username.as_deref(),
        ctx.msg.first_name.as_deref(),
    );
    return Ok(Some(OutgoingMessage {
        chat_id: ctx.msg.chat_id,
        text: "Welcome! You've been registered. An admin needs to approve your access."
            .into(),
        parse_mode: None,
        reply_to: None,
        platform: None,
        topic_id: None,
        interactive: None,
    }));
}
```

After the match block (before `ctx.extensions.insert(state)`), add:

```rust
// Update last_seen for active users
self.auth.read().await.touch_last_seen(user_id, platform);
```

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo check -p amanclaw-core`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/src/middleware/auth.rs
git commit -m "feat: capture username/first_name on registration and update last_seen"
```

### Task 5: Wire Auth::with_pool in Engine::start

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

- [ ] **Step 1: Replace `Auth::new` with `Auth::with_pool`**

In `rust/crates/amanclaw-core/src/lib.rs`, change line 85:

```rust
// Before:
let auth = Auth::new(config.admin_users.clone());

// After:
let auth = Auth::with_pool(config.admin_users.clone(), memory.pool().clone()).await;
```

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo check -p amanclaw-core`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/src/lib.rs
git commit -m "feat: initialize Auth with SQLite pool for persistent user state"
```

---

## Chunk 2: API Endpoints

### Task 6: Add new user management API routes

**Files:**
- Modify: `rust/crates/amanclaw-api/src/routes/users.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`
- Modify: `rust/crates/amanclaw-api/src/routes/mod.rs`

- [ ] **Step 1: Create stats route module**

Add a new file `rust/crates/amanclaw-api/src/routes/stats.rs`:

```rust
use crate::state::ApiState;
use amanclaw_memory::sqlite::SqliteMemory;
use axum::{Json, extract::State, http::StatusCode};

pub async fn get_stats(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let stats = mem.get_user_stats().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(stats)))
}
```

- [ ] **Step 2: Add `from_pool` convenience constructor to SqliteMemory**

In `rust/crates/amanclaw-memory/src/sqlite.rs`, add this method to `impl SqliteMemory`:

```rust
/// Create a SqliteMemory from an existing pool (no schema init).
pub fn from_pool(pool: SqlitePool) -> Self {
    Self { pool }
}
```

- [ ] **Step 3: Rewrite users.rs with all endpoints**

Replace `rust/crates/amanclaw-api/src/routes/users.rs`:

```rust
use crate::state::ApiState;
use amanclaw_memory::sqlite::SqliteMemory;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserListQuery {
    pub platform: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_users(
    State(state): State<ApiState>,
    Query(query): Query<UserListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let users = mem
        .list_users(
            query.platform.as_deref(),
            query.status.as_deref(),
            query.search.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let count = users.len();
    Ok(Json(serde_json::json!({ "users": users, "count": count })))
}

pub async fn get_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let user = mem
        .get_user(&user_id, &platform)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let message_count = mem
        .get_message_count_ns("default", &user_id)
        .await
        .unwrap_or(0);
    let facts = mem.get_facts(&user_id).await.unwrap_or_default();

    Ok(Json(serde_json::json!({
        "user_id": user.user_id,
        "platform": user.platform,
        "state": user.state,
        "username": user.username,
        "first_name": user.first_name,
        "first_seen": user.first_seen,
        "last_seen": user.last_seen,
        "message_count": message_count,
        "facts": facts,
    })))
}

pub async fn get_user_history(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _ = platform; // user_id is unique enough for history lookup
    let mem = SqliteMemory::from_pool(state.pool.clone());
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    let messages = mem
        .get_history_paginated("default", &user_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total = mem
        .get_message_count_ns("default", &user_id)
        .await
        .unwrap_or(0);
    Ok(Json(serde_json::json!({
        "messages": messages.iter().map(|m| serde_json::json!({
            "role": m.role,
            "content": m.content,
        })).collect::<Vec<_>>(),
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

pub async fn approve_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.approve_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "approved" })))
}

pub async fn block_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.block_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "blocked" })))
}

pub async fn unblock_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    auth.unblock_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "pending" })))
}
```

- [ ] **Step 4: Add stats module to routes/mod.rs**

In `rust/crates/amanclaw-api/src/routes/mod.rs`, add:

```rust
pub mod stats;
```

- [ ] **Step 5: Register new routes in lib.rs**

In `rust/crates/amanclaw-api/src/lib.rs`, add the new routes to the `authed` Router (after the existing user routes):

```rust
.route("/api/users/{platform}/{user_id}", get(routes::users::get_user))
.route("/api/users/{platform}/{user_id}/history", get(routes::users::get_user_history))
.route("/api/users/{platform}/{user_id}/unblock", put(routes::users::unblock_user))
.route("/api/stats", get(routes::stats::get_stats))
```

Also change the existing approve/block routes from `post` to `put` to match the spec:

```rust
// Change these from post to put:
.route("/api/users/{platform}/{user_id}/approve", put(routes::users::approve_user))
.route("/api/users/{platform}/{user_id}/block", put(routes::users::block_user))
```

- [ ] **Step 6: Verify it compiles**

Run: `cd rust && cargo check -p amanclaw-api`
Expected: success

- [ ] **Step 7: Commit**

```bash
git add rust/crates/amanclaw-api/src/routes/users.rs rust/crates/amanclaw-api/src/routes/stats.rs rust/crates/amanclaw-api/src/routes/mod.rs rust/crates/amanclaw-api/src/lib.rs rust/crates/amanclaw-memory/src/sqlite.rs
git commit -m "feat: add user detail, history, stats, and unblock API endpoints"
```

---

## Chunk 3: Dashboard Enhancement

### Task 7: Enhance Users page with filters and platform icons

**Files:**
- Modify: `dashboard/src/lib/pages/Users.svelte`

- [ ] **Step 1: Update Users.svelte with filters, richer table, and user detail modal**

Replace `dashboard/src/lib/pages/Users.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { apiFetch } from '../stores/api'
  import StatusBadge from '../components/StatusBadge.svelte'

  let users: any[] = []
  let loading = true
  let search = ''
  let platformFilter = ''
  let statusFilter = ''
  let selectedUser: any = null
  let userHistory: any[] = []
  let historyLoading = false
  let historyTotal = 0

  const platforms = ['', 'telegram', 'discord', 'whatsapp', 'whatsapp-web', 'slack']
  const statuses = ['', 'pending', 'approved', 'blocked']
  const platformIcons: Record<string, string> = {
    telegram: '\u2708\uFE0F',
    discord: '\uD83C\uDFAE',
    whatsapp: '\uD83D\uDCAC',
    'whatsapp-web': '\uD83D\uDCAC',
    slack: '\uD83D\uDCBC',
  }

  onMount(loadUsers)

  async function loadUsers() {
    loading = true
    try {
      const params = new URLSearchParams()
      if (platformFilter) params.set('platform', platformFilter)
      if (statusFilter) params.set('status', statusFilter)
      if (search) params.set('search', search)
      const qs = params.toString()
      const data = await apiFetch(`/users${qs ? '?' + qs : ''}`)
      users = data.users
    } catch (e) {
      console.error(e)
    } finally {
      loading = false
    }
  }

  async function showUser(platform: string, userId: string) {
    try {
      selectedUser = await apiFetch(`/users/${platform}/${userId}`)
      await loadHistory(platform, userId)
    } catch (e) {
      console.error(e)
    }
  }

  async function loadHistory(platform: string, userId: string, offset = 0) {
    historyLoading = true
    try {
      const data = await apiFetch(`/users/${platform}/${userId}/history?limit=20&offset=${offset}`)
      userHistory = data.messages
      historyTotal = data.total
    } catch (e) {
      console.error(e)
    } finally {
      historyLoading = false
    }
  }

  async function approveUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/approve`, { method: 'PUT' })
    await loadUsers()
    if (selectedUser?.user_id === userId) selectedUser.state = 'approved'
  }

  async function blockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/block`, { method: 'PUT' })
    await loadUsers()
    if (selectedUser?.user_id === userId) selectedUser.state = 'blocked'
  }

  async function unblockUser(platform: string, userId: string) {
    await apiFetch(`/users/${platform}/${userId}/unblock`, { method: 'PUT' })
    await loadUsers()
    if (selectedUser?.user_id === userId) selectedUser.state = 'pending'
  }

  function closeDetail() {
    selectedUser = null
    userHistory = []
  }

  function formatDate(d: string | null) {
    if (!d) return '-'
    return new Date(d).toLocaleString()
  }
</script>

<div class="p-6 md:p-8">
  <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">Users</h2>

  <!-- Filters -->
  <div class="flex flex-wrap gap-3 mb-6">
    <input
      type="text"
      bind:value={search}
      on:input={loadUsers}
      placeholder="Search users..."
      class="flex-1 min-w-48 px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white outline-none focus:ring-2 focus:ring-blue-500"
    />
    <select
      bind:value={platformFilter}
      on:change={loadUsers}
      class="px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
    >
      <option value="">All platforms</option>
      {#each platforms.slice(1) as p}
        <option value={p}>{p}</option>
      {/each}
    </select>
    <select
      bind:value={statusFilter}
      on:change={loadUsers}
      class="px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
    >
      <option value="">All statuses</option>
      {#each statuses.slice(1) as s}
        <option value={s}>{s}</option>
      {/each}
    </select>
  </div>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else}
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="border-b border-gray-200 dark:border-gray-700">
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Platform</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">User</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Status</th>
              <th class="text-left px-4 py-3 text-xs font-medium text-gray-500 uppercase">Last Seen</th>
              <th class="text-right px-4 py-3 text-xs font-medium text-gray-500 uppercase">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each users as user}
              <tr class="border-b border-gray-100 dark:border-gray-700/50 hover:bg-gray-50 dark:hover:bg-gray-700/30 cursor-pointer"
                  on:click={() => showUser(user.platform, user.user_id)}>
                <td class="px-4 py-3 text-sm">
                  <span class="mr-1">{platformIcons[user.platform] || ''}</span>
                  <span class="text-gray-600 dark:text-gray-300">{user.platform}</span>
                </td>
                <td class="px-4 py-3 text-sm">
                  <div class="text-gray-900 dark:text-white font-mono">{user.user_id}</div>
                  {#if user.username || user.first_name}
                    <div class="text-xs text-gray-500">{user.first_name || ''} {user.username ? `@${user.username}` : ''}</div>
                  {/if}
                </td>
                <td class="px-4 py-3">
                  <StatusBadge
                    status={user.state === 'approved' ? 'online' : user.state === 'blocked' ? 'offline' : 'warning'}
                    label={user.state}
                  />
                </td>
                <td class="px-4 py-3 text-sm text-gray-500">{formatDate(user.last_seen)}</td>
                <td class="px-4 py-3 text-right space-x-2" on:click|stopPropagation>
                  {#if user.state !== 'approved'}
                    <button on:click={() => approveUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-lg">
                      Approve
                    </button>
                  {/if}
                  {#if user.state === 'blocked'}
                    <button on:click={() => unblockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg">
                      Unblock
                    </button>
                  {:else if user.state !== 'blocked'}
                    <button on:click={() => blockUser(user.platform, user.user_id)}
                      class="text-xs px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded-lg">
                      Block
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
    <p class="text-sm text-gray-500 mt-3">{users.length} users</p>
  {/if}
</div>

<!-- User Detail Modal -->
{#if selectedUser}
  <div class="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-16 px-4" on:click={closeDetail}>
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] overflow-auto"
         on:click|stopPropagation>
      <div class="p-6">
        <!-- Header -->
        <div class="flex justify-between items-start mb-6">
          <div>
            <h3 class="text-xl font-bold text-gray-900 dark:text-white">
              {platformIcons[selectedUser.platform] || ''} {selectedUser.first_name || selectedUser.user_id}
            </h3>
            <p class="text-sm text-gray-500">
              {selectedUser.platform} &middot; <span class="font-mono">{selectedUser.user_id}</span>
              {#if selectedUser.username} &middot; @{selectedUser.username}{/if}
            </p>
          </div>
          <button on:click={closeDetail} class="text-gray-400 hover:text-gray-600 text-xl">&times;</button>
        </div>

        <!-- Info Grid -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">Status</p>
            <p class="font-semibold text-gray-900 dark:text-white capitalize">{selectedUser.state}</p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">Messages</p>
            <p class="font-semibold text-gray-900 dark:text-white">{selectedUser.message_count}</p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">First Seen</p>
            <p class="font-semibold text-gray-900 dark:text-white text-xs">{formatDate(selectedUser.first_seen)}</p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase">Last Seen</p>
            <p class="font-semibold text-gray-900 dark:text-white text-xs">{formatDate(selectedUser.last_seen)}</p>
          </div>
        </div>

        <!-- Facts -->
        {#if selectedUser.facts && Object.keys(selectedUser.facts).length > 0}
          <div class="mb-6">
            <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Learned Facts</h4>
            <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-3">
              {#each Object.entries(selectedUser.facts) as [key, value]}
                <div class="flex justify-between py-1 text-sm">
                  <span class="text-gray-500">{key}</span>
                  <span class="text-gray-900 dark:text-white">{value}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Conversation History -->
        <div>
          <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
            Recent Conversations ({historyTotal} total)
          </h4>
          {#if historyLoading}
            <p class="text-gray-500 text-sm">Loading...</p>
          {:else}
            <div class="space-y-2 max-h-60 overflow-auto">
              {#each userHistory as msg}
                <div class="text-sm p-2 rounded-lg {msg.role === 'user' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-900 dark:text-blue-100' : 'bg-gray-50 dark:bg-gray-700/50 text-gray-900 dark:text-white'}">
                  <span class="text-xs font-medium text-gray-500 uppercase">{msg.role}</span>
                  <p class="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
                </div>
              {/each}
              {#if userHistory.length === 0}
                <p class="text-gray-500 text-sm">No conversation history</p>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Actions -->
        <div class="flex gap-2 mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
          {#if selectedUser.state !== 'approved'}
            <button on:click={() => approveUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg text-sm">
              Approve
            </button>
          {/if}
          {#if selectedUser.state === 'blocked'}
            <button on:click={() => unblockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg text-sm">
              Unblock
            </button>
          {:else if selectedUser.state !== 'blocked'}
            <button on:click={() => blockUser(selectedUser.platform, selectedUser.user_id)}
              class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm">
              Block
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
```

- [ ] **Step 2: Verify it builds**

Run: `cd dashboard && npm run build`
Expected: success, output in `dashboard/dist/`

- [ ] **Step 3: Commit**

```bash
git add dashboard/src/lib/pages/Users.svelte
git commit -m "feat: enhance Users page with filters, detail modal, and conversation history"
```

### Task 8: Add stats to Dashboard page

**Files:**
- Modify: `dashboard/src/lib/pages/Dashboard.svelte`

- [ ] **Step 1: Read current Dashboard.svelte**

Read `dashboard/src/lib/pages/Dashboard.svelte` to understand existing structure before modifying.

- [ ] **Step 2: Add user stats cards to Dashboard page**

Add a user stats section to the Dashboard page that fetches from `/api/stats` and displays total, pending, approved, blocked counts, and per-platform breakdown using the existing `StatCard` component.

- [ ] **Step 3: Verify it builds**

Run: `cd dashboard && npm run build`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add dashboard/src/lib/pages/Dashboard.svelte
git commit -m "feat: add user stats cards to Dashboard page"
```

### Task 9: Rebuild dashboard dist and verify full build

**Files:**
- Modify: `dashboard/dist/` (rebuild)

- [ ] **Step 1: Rebuild the dashboard**

Run: `cd dashboard && npm run build`
Expected: success

- [ ] **Step 2: Verify the full Rust project compiles with the new dashboard**

Run: `cd rust && cargo check`
Expected: success (Axum will embed the new `dashboard/dist/` via `include_dir`)

- [ ] **Step 3: Run all Rust tests**

Run: `cd rust && cargo test`
Expected: all tests pass

- [ ] **Step 4: Commit everything**

```bash
git add dashboard/dist/
git commit -m "chore: rebuild dashboard dist with user management enhancements"
```

---

## Summary

| Task | What | Estimated Scope |
|------|------|----------------|
| 1 | Add `users` table to schema | 1 file, ~10 lines |
| 2 | User CRUD methods in SqliteMemory | 1 file, ~150 lines + tests |
| 3 | Rewrite Auth with SQLite write-through | 2 files, ~120 lines |
| 4 | Update AuthMiddleware for user capture | 1 file, ~5 lines changed |
| 5 | Wire Auth::with_pool in Engine | 1 file, 1 line changed |
| 6 | New API routes (detail, history, stats, unblock) | 4 files, ~120 lines |
| 7 | Enhanced Users page (filters, detail modal, history) | 1 file, ~200 lines |
| 8 | Dashboard stats cards | 1 file, ~30 lines |
| 9 | Rebuild and verify | build commands |
