use amanclaw_traits::memory::{HistoryMessage, MemoryBackend};
use anyhow::Result;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::collections::HashMap;

use crate::schema::INIT_SQL;

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
    pub by_platform: HashMap<String, i64>,
}

pub struct SqliteMemory {
    pool: SqlitePool,
}

impl SqliteMemory {
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

        // Run migrations first — adds columns to existing tables before INIT_SQL creates indexes on them
        for stmt in crate::schema::MIGRATE_NS_STMTS {
            let _ = sqlx::raw_sql(stmt).execute(&pool).await;
        }

        sqlx::raw_sql(INIT_SQL).execute(&pool).await?;

        tracing::info!("Memory initialized at {}", db_path);
        Ok(Self { pool })
    }

    /// Get a reference to the underlying SQLite pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // --- Backward-compatible methods (delegate to namespaced with "default") ---

    pub async fn save_exchange(
        &self,
        user_id: &str,
        platform: &str,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Result<()> {
        self.save_exchange_ns("default", user_id, platform, user_msg, assistant_msg)
            .await
    }

    pub async fn get_history(&self, user_id: &str, limit: i64) -> Result<Vec<HistoryMessage>> {
        self.get_history_ns("default", user_id, limit).await
    }

    pub async fn clear_history(&self, user_id: &str) -> Result<()> {
        self.clear_history_ns("default", user_id).await
    }

    pub async fn get_message_count(&self, user_id: &str) -> Result<i64> {
        self.get_message_count_ns("default", user_id).await
    }

    pub async fn get_summary(&self, user_id: &str) -> Result<Option<String>> {
        self.get_summary_ns("default", user_id).await
    }

    pub async fn save_summary_and_prune(
        &self,
        user_id: &str,
        summary: &str,
        keep_recent: i64,
    ) -> Result<()> {
        self.save_summary_and_prune_ns("default", user_id, summary, keep_recent)
            .await
    }

    pub async fn needs_summarization(&self, user_id: &str, threshold: i64) -> Result<bool> {
        self.needs_summarization_ns("default", user_id, threshold)
            .await
    }

    // --- Namespaced methods ---

    pub async fn save_exchange_ns(
        &self,
        ns: &str,
        user_id: &str,
        platform: &str,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO messages (namespace, user_id, platform, role, content) VALUES (?, ?, ?, 'user', ?)"
        )
            .bind(ns).bind(user_id).bind(platform).bind(user_msg)
            .execute(&self.pool).await?;
        sqlx::query(
            "INSERT INTO messages (namespace, user_id, platform, role, content) VALUES (?, ?, ?, 'assistant', ?)"
        )
            .bind(ns).bind(user_id).bind(platform).bind(assistant_msg)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_history_ns(
        &self,
        ns: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<HistoryMessage>> {
        let rows = sqlx::query(
            "SELECT role, content FROM messages WHERE namespace = ? AND user_id = ? ORDER BY id DESC LIMIT ?"
        )
            .bind(ns).bind(user_id).bind(limit)
            .fetch_all(&self.pool).await?;

        let mut messages: Vec<HistoryMessage> = rows
            .iter()
            .map(|row| HistoryMessage {
                role: row.get("role"),
                content: row.get("content"),
            })
            .collect();
        messages.reverse();
        Ok(messages)
    }

    pub async fn clear_history_ns(&self, ns: &str, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE namespace = ? AND user_id = ?")
            .bind(ns)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_message_count_ns(&self, ns: &str, user_id: &str) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM messages WHERE namespace = ? AND user_id = ?",
        )
        .bind(ns)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get("count"))
    }

    pub async fn get_summary_ns(&self, ns: &str, user_id: &str) -> Result<Option<String>> {
        let row = sqlx::query(
            "SELECT summary FROM summaries WHERE namespace = ? AND user_id = ? ORDER BY id DESC LIMIT 1"
        )
            .bind(ns).bind(user_id)
            .fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get("summary")))
    }

    pub async fn save_summary_and_prune_ns(
        &self,
        ns: &str,
        user_id: &str,
        summary: &str,
        keep_recent: i64,
    ) -> Result<()> {
        let count = self.get_message_count_ns(ns, user_id).await?;
        sqlx::query(
            "INSERT INTO summaries (namespace, user_id, summary, message_count) VALUES (?, ?, ?, ?)"
        )
            .bind(ns).bind(user_id).bind(summary).bind(count)
            .execute(&self.pool).await?;

        sqlx::query(
            "DELETE FROM messages WHERE namespace = ? AND user_id = ? AND id NOT IN (
                SELECT id FROM messages WHERE namespace = ? AND user_id = ? ORDER BY id DESC LIMIT ?
            )",
        )
        .bind(ns)
        .bind(user_id)
        .bind(ns)
        .bind(user_id)
        .bind(keep_recent)
        .execute(&self.pool)
        .await?;

        tracing::info!(ns, user_id, "Summarized and pruned conversation");
        Ok(())
    }

    pub async fn needs_summarization_ns(
        &self,
        ns: &str,
        user_id: &str,
        threshold: i64,
    ) -> Result<bool> {
        let count = self.get_message_count_ns(ns, user_id).await?;
        Ok(count > threshold)
    }

    // --- Facts (not namespaced — per-user across all agents) ---

    pub async fn save_fact(&self, user_id: &str, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO facts (user_id, key, value) VALUES (?, ?, ?)
             ON CONFLICT(user_id, key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP"
        )
            .bind(user_id).bind(key).bind(value)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_facts(&self, user_id: &str) -> Result<HashMap<String, String>> {
        let rows = sqlx::query("SELECT key, value FROM facts WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .iter()
            .map(|row| (row.get::<String, _>("key"), row.get::<String, _>("value")))
            .collect())
    }

    pub async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM facts WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Create a SqliteMemory from an existing pool (no schema init).
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

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
               last_seen = CURRENT_TIMESTAMP",
        )
        .bind(user_id)
        .bind(platform)
        .bind(state)
        .bind(username)
        .bind(first_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user(&self, user_id: &str, platform: &str) -> Result<Option<UserRow>> {
        let row = sqlx::query(
            "SELECT user_id, platform, state, username, first_name, first_seen, last_seen
             FROM users WHERE user_id = ? AND platform = ?",
        )
        .bind(user_id)
        .bind(platform)
        .fetch_optional(&self.pool)
        .await?;

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

    pub async fn update_user_state(
        &self,
        user_id: &str,
        platform: &str,
        state: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE users SET state = ?, last_seen = CURRENT_TIMESTAMP WHERE user_id = ? AND platform = ?",
        )
        .bind(state)
        .bind(user_id)
        .bind(platform)
        .execute(&self.pool)
        .await?;
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
        Ok(rows
            .iter()
            .map(|r| UserRow {
                user_id: r.get("user_id"),
                platform: r.get("platform"),
                state: r.get("state"),
                username: r.get("username"),
                first_name: r.get("first_name"),
                first_seen: r.get("first_seen"),
                last_seen: r.get("last_seen"),
            })
            .collect())
    }

    pub async fn touch_user_last_seen(&self, user_id: &str, platform: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET last_seen = CURRENT_TIMESTAMP WHERE user_id = ? AND platform = ?",
        )
        .bind(user_id)
        .bind(platform)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user_stats(&self) -> Result<UserStats> {
        let total: i64 = sqlx::query("SELECT COUNT(*) as c FROM users")
            .fetch_one(&self.pool)
            .await?
            .get("c");
        let pending: i64 =
            sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'pending'")
                .fetch_one(&self.pool)
                .await?
                .get("c");
        let approved: i64 =
            sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'approved'")
                .fetch_one(&self.pool)
                .await?
                .get("c");
        let blocked: i64 =
            sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'blocked'")
                .fetch_one(&self.pool)
                .await?
                .get("c");

        let platform_rows =
            sqlx::query("SELECT platform, COUNT(*) as c FROM users GROUP BY platform")
                .fetch_all(&self.pool)
                .await?;
        let by_platform: HashMap<String, i64> = platform_rows
            .iter()
            .map(|r| (r.get::<String, _>("platform"), r.get::<i64, _>("c")))
            .collect();

        Ok(UserStats {
            total,
            pending,
            approved,
            blocked,
            by_platform,
        })
    }

    pub async fn get_history_paginated(
        &self,
        ns: &str,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<HistoryMessage>> {
        let rows = sqlx::query(
            "SELECT role, content FROM messages WHERE namespace = ? AND user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(ns)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut messages: Vec<HistoryMessage> = rows
            .iter()
            .map(|row| HistoryMessage {
                role: row.get("role"),
                content: row.get("content"),
            })
            .collect();
        messages.reverse();
        Ok(messages)
    }
}

#[async_trait::async_trait]
impl MemoryBackend for SqliteMemory {
    async fn save_exchange(
        &self,
        ns: &str,
        user_id: &str,
        platform: &str,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Result<()> {
        self.save_exchange_ns(ns, user_id, platform, user_msg, assistant_msg)
            .await
    }

    async fn get_history(
        &self,
        ns: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<HistoryMessage>> {
        self.get_history_ns(ns, user_id, limit).await
    }

    async fn clear_history(&self, ns: &str, user_id: &str) -> Result<()> {
        self.clear_history_ns(ns, user_id).await
    }

    async fn get_message_count(&self, ns: &str, user_id: &str) -> Result<i64> {
        self.get_message_count_ns(ns, user_id).await
    }

    async fn save_fact(&self, user_id: &str, key: &str, value: &str) -> Result<()> {
        SqliteMemory::save_fact(self, user_id, key, value).await
    }

    async fn get_facts(&self, user_id: &str) -> Result<HashMap<String, String>> {
        SqliteMemory::get_facts(self, user_id).await
    }

    async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool> {
        SqliteMemory::delete_fact(self, user_id, key).await
    }

    async fn get_summary(&self, ns: &str, user_id: &str) -> Result<Option<String>> {
        self.get_summary_ns(ns, user_id).await
    }

    async fn save_summary_and_prune(
        &self,
        ns: &str,
        user_id: &str,
        summary: &str,
        keep_recent: i64,
    ) -> Result<()> {
        self.save_summary_and_prune_ns(ns, user_id, summary, keep_recent)
            .await
    }

    async fn needs_summarization(&self, ns: &str, user_id: &str, threshold: i64) -> Result<bool> {
        self.needs_summarization_ns(ns, user_id, threshold).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_memory() -> SqliteMemory {
        SqliteMemory::new(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_save_and_get_history() {
        let mem = make_memory().await;
        mem.save_exchange("u1", "telegram", "Hello", "Hi there!")
            .await
            .unwrap();
        mem.save_exchange("u1", "telegram", "How are you?", "I'm good!")
            .await
            .unwrap();

        let history = mem.get_history("u1", 10).await.unwrap();
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_save_and_get_facts() {
        let mem = make_memory().await;
        mem.save_fact("u1", "name", "Aman").await.unwrap();
        mem.save_fact("u1", "city", "KL").await.unwrap();

        let facts = mem.get_facts("u1").await.unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts.get("name").unwrap(), "Aman");
    }

    #[tokio::test]
    async fn test_fact_upsert() {
        let mem = make_memory().await;
        mem.save_fact("u1", "city", "KL").await.unwrap();
        mem.save_fact("u1", "city", "Puncak Alam").await.unwrap();

        let facts = mem.get_facts("u1").await.unwrap();
        assert_eq!(facts.get("city").unwrap(), "Puncak Alam");
    }

    #[tokio::test]
    async fn test_message_count() {
        let mem = make_memory().await;
        mem.save_exchange("u1", "telegram", "a", "b").await.unwrap();
        mem.save_exchange("u1", "telegram", "c", "d").await.unwrap();

        let count = mem.get_message_count("u1").await.unwrap();
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn test_summary_and_prune() {
        let mem = make_memory().await;
        for i in 0..20 {
            mem.save_exchange("u1", "telegram", &format!("msg{i}"), &format!("reply{i}"))
                .await
                .unwrap();
        }
        assert_eq!(mem.get_message_count("u1").await.unwrap(), 40);

        mem.save_summary_and_prune("u1", "User discussed topics 0-19", 10)
            .await
            .unwrap();

        let summary = mem.get_summary("u1").await.unwrap();
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("topics 0-19"));

        let count = mem.get_message_count("u1").await.unwrap();
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_needs_summarization() {
        let mem = make_memory().await;
        assert!(!mem.needs_summarization("u1", 30).await.unwrap());

        for i in 0..20 {
            mem.save_exchange("u1", "telegram", &format!("m{i}"), &format!("r{i}"))
                .await
                .unwrap();
        }
        assert!(mem.needs_summarization("u1", 30).await.unwrap());
        assert!(!mem.needs_summarization("u1", 50).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_fact() {
        let mem = make_memory().await;
        mem.save_fact("u1", "name", "Aman").await.unwrap();
        assert!(mem.delete_fact("u1", "name").await.unwrap());
        assert!(!mem.delete_fact("u1", "name").await.unwrap());
        let facts = mem.get_facts("u1").await.unwrap();
        assert!(facts.is_empty());
    }

    // --- New namespace tests ---

    #[tokio::test]
    async fn test_namespaced_history_isolation() {
        let mem = make_memory().await;
        mem.save_exchange_ns("agent_a", "u1", "telegram", "Hello A", "Hi from A")
            .await
            .unwrap();
        mem.save_exchange_ns("agent_b", "u1", "telegram", "Hello B", "Hi from B")
            .await
            .unwrap();

        let history_a = mem.get_history_ns("agent_a", "u1", 10).await.unwrap();
        let history_b = mem.get_history_ns("agent_b", "u1", 10).await.unwrap();

        assert_eq!(history_a.len(), 2);
        assert_eq!(history_b.len(), 2);
        assert!(history_a[0].content.contains("Hello A"));
        assert!(history_b[0].content.contains("Hello B"));
    }

    #[tokio::test]
    async fn test_namespaced_summary_isolation() {
        let mem = make_memory().await;
        for i in 0..10 {
            mem.save_exchange_ns(
                "ns_a",
                "u1",
                "telegram",
                &format!("a{i}"),
                &format!("ra{i}"),
            )
            .await
            .unwrap();
            mem.save_exchange_ns(
                "ns_b",
                "u1",
                "telegram",
                &format!("b{i}"),
                &format!("rb{i}"),
            )
            .await
            .unwrap();
        }

        mem.save_summary_and_prune_ns("ns_a", "u1", "Summary A", 4)
            .await
            .unwrap();

        let summary_a = mem.get_summary_ns("ns_a", "u1").await.unwrap();
        let summary_b = mem.get_summary_ns("ns_b", "u1").await.unwrap();

        assert!(summary_a.is_some());
        assert!(summary_b.is_none());

        // ns_b should still have all 20 messages (10 exchanges)
        let count_b = mem.get_message_count_ns("ns_b", "u1").await.unwrap();
        assert_eq!(count_b, 20);
    }

    // --- User management tests ---

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
        mem.upsert_user("1", "telegram", "pending", None, None)
            .await
            .unwrap();
        mem.upsert_user("2", "discord", "approved", None, None)
            .await
            .unwrap();
        mem.upsert_user("3", "telegram", "approved", None, None)
            .await
            .unwrap();

        let all = mem.list_users(None, None, None).await.unwrap();
        assert_eq!(all.len(), 3);

        let telegram = mem
            .list_users(Some("telegram"), None, None)
            .await
            .unwrap();
        assert_eq!(telegram.len(), 2);

        let approved = mem
            .list_users(None, Some("approved"), None)
            .await
            .unwrap();
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
        mem.upsert_user("1", "telegram", "pending", None, None)
            .await
            .unwrap();
        mem.upsert_user("2", "telegram", "approved", None, None)
            .await
            .unwrap();
        mem.upsert_user("3", "discord", "approved", None, None)
            .await
            .unwrap();
        mem.upsert_user("4", "slack", "blocked", None, None)
            .await
            .unwrap();

        let stats = mem.get_user_stats().await.unwrap();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.approved, 2);
        assert_eq!(stats.blocked, 1);
        assert_eq!(stats.by_platform.get("telegram"), Some(&2));
    }

    #[tokio::test]
    async fn test_memory_backend_trait_works() {
        let mem = make_memory().await;
        let backend: &dyn MemoryBackend = &mem;

        backend
            .save_exchange("test_ns", "u1", "telegram", "hello", "hi")
            .await
            .unwrap();
        let history = backend.get_history("test_ns", "u1", 10).await.unwrap();
        assert_eq!(history.len(), 2);

        backend.save_fact("u1", "name", "Aman").await.unwrap();
        let facts = backend.get_facts("u1").await.unwrap();
        assert_eq!(facts.get("name").unwrap(), "Aman");
    }

    #[tokio::test]
    async fn test_clear_history() {
        let mem = make_memory().await;
        mem.save_exchange("u1", "telegram", "hello", "hi")
            .await
            .unwrap();
        assert_eq!(mem.get_message_count("u1").await.unwrap(), 2);

        mem.clear_history("u1").await.unwrap();
        assert_eq!(mem.get_message_count("u1").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_history_ordering() {
        let mem = make_memory().await;
        mem.save_exchange("u1", "telegram", "first", "reply1")
            .await
            .unwrap();
        mem.save_exchange("u1", "telegram", "second", "reply2")
            .await
            .unwrap();

        let history = mem.get_history("u1", 10).await.unwrap();
        assert_eq!(history[0].content, "first");
        assert_eq!(history[1].content, "reply1");
        assert_eq!(history[2].content, "second");
        assert_eq!(history[3].content, "reply2");
    }

    #[tokio::test]
    async fn test_history_limit() {
        let mem = make_memory().await;
        for i in 0..10 {
            mem.save_exchange("u1", "telegram", &format!("msg{i}"), &format!("reply{i}"))
                .await
                .unwrap();
        }
        // Total messages: 20 (10 pairs). Limit 4 should return the last 4.
        let history = mem.get_history("u1", 4).await.unwrap();
        assert_eq!(history.len(), 4);
        // Should be the most recent messages in order
        assert_eq!(history[0].content, "msg8");
        assert_eq!(history[3].content, "reply9");
    }

    #[tokio::test]
    async fn test_get_nonexistent_user() {
        let mem = make_memory().await;
        let user = mem.get_user("nonexistent", "telegram").await.unwrap();
        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_update_nonexistent_user_returns_false() {
        let mem = make_memory().await;
        let updated = mem
            .update_user_state("nonexistent", "telegram", "approved")
            .await
            .unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn test_upsert_user_preserves_existing_fields() {
        let mem = make_memory().await;
        mem.upsert_user("123", "telegram", "pending", Some("aman"), Some("Aman"))
            .await
            .unwrap();
        // Re-upsert without username/first_name
        mem.upsert_user("123", "telegram", "pending", None, None)
            .await
            .unwrap();
        let user = mem.get_user("123", "telegram").await.unwrap().unwrap();
        // COALESCE should preserve the existing values
        assert_eq!(user.username.as_deref(), Some("aman"));
        assert_eq!(user.first_name.as_deref(), Some("Aman"));
    }

    #[tokio::test]
    async fn test_list_users_search() {
        let mem = make_memory().await;
        mem.upsert_user("1", "telegram", "pending", Some("ali"), Some("Ali"))
            .await
            .unwrap();
        mem.upsert_user("2", "telegram", "approved", Some("abu"), Some("Abu"))
            .await
            .unwrap();

        let results = mem
            .list_users(None, None, Some("ali"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].username.as_deref(), Some("ali"));
    }

    #[tokio::test]
    async fn test_empty_history_for_nonexistent_user() {
        let mem = make_memory().await;
        let history = mem.get_history("nonexistent", 10).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_empty_facts_for_nonexistent_user() {
        let mem = make_memory().await;
        let facts = mem.get_facts("nonexistent").await.unwrap();
        assert!(facts.is_empty());
    }

    #[tokio::test]
    async fn test_get_history_paginated() {
        let mem = make_memory().await;
        for i in 0..5 {
            mem.save_exchange_ns("ns", "u1", "telegram", &format!("msg{i}"), &format!("reply{i}"))
                .await
                .unwrap();
        }
        // 10 messages total. Page: limit=4, offset=2
        let page = mem.get_history_paginated("ns", "u1", 4, 2).await.unwrap();
        assert_eq!(page.len(), 4);
    }
}
