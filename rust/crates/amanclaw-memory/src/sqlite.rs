use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, Row};
use std::collections::HashMap;

use crate::schema::INIT_SQL;

#[derive(Debug, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

pub struct SqliteMemory {
    pool: SqlitePool,
}

impl SqliteMemory {
    pub async fn new(db_path: &str) -> Result<Self> {
        let url = if db_path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{}?mode=rwc", db_path)
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        sqlx::raw_sql(INIT_SQL).execute(&pool).await?;

        tracing::info!("Memory initialized at {}", db_path);
        Ok(Self { pool })
    }

    pub async fn save_exchange(
        &self, user_id: &str, platform: &str, user_msg: &str, assistant_msg: &str,
    ) -> Result<()> {
        sqlx::query("INSERT INTO messages (user_id, platform, role, content) VALUES (?, ?, 'user', ?)")
            .bind(user_id).bind(platform).bind(user_msg)
            .execute(&self.pool).await?;
        sqlx::query("INSERT INTO messages (user_id, platform, role, content) VALUES (?, ?, 'assistant', ?)")
            .bind(user_id).bind(platform).bind(assistant_msg)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_history(&self, user_id: &str, limit: i64) -> Result<Vec<HistoryMessage>> {
        let rows = sqlx::query(
            "SELECT role, content FROM messages WHERE user_id = ? ORDER BY id DESC LIMIT ?"
        )
            .bind(user_id).bind(limit)
            .fetch_all(&self.pool).await?;

        let mut messages: Vec<HistoryMessage> = rows.iter().map(|row| HistoryMessage {
            role: row.get("role"),
            content: row.get("content"),
        }).collect();
        messages.reverse();
        Ok(messages)
    }

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
            .fetch_all(&self.pool).await?;

        Ok(rows.iter().map(|row| {
            (row.get::<String, _>("key"), row.get::<String, _>("value"))
        }).collect())
    }

    pub async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM facts WHERE user_id = ? AND key = ?")
            .bind(user_id).bind(key)
            .execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn clear_history(&self, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_message_count(&self, user_id: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM messages WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool).await?;
        Ok(row.get("count"))
    }

    // --- Summarization ---

    /// Get the latest summary for a user.
    pub async fn get_summary(&self, user_id: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT summary FROM summaries WHERE user_id = ? ORDER BY id DESC LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get("summary")))
    }

    /// Save a conversation summary and delete the old messages that were summarized.
    pub async fn save_summary_and_prune(
        &self, user_id: &str, summary: &str, keep_recent: i64,
    ) -> Result<()> {
        // Save summary
        let count = self.get_message_count(user_id).await?;
        sqlx::query("INSERT INTO summaries (user_id, summary, message_count) VALUES (?, ?, ?)")
            .bind(user_id).bind(summary).bind(count)
            .execute(&self.pool).await?;

        // Delete all but the most recent N messages
        sqlx::query(
            "DELETE FROM messages WHERE user_id = ? AND id NOT IN (
                SELECT id FROM messages WHERE user_id = ? ORDER BY id DESC LIMIT ?
            )"
        )
            .bind(user_id).bind(user_id).bind(keep_recent)
            .execute(&self.pool).await?;

        tracing::info!(user_id, "Summarized and pruned conversation");
        Ok(())
    }

    /// Check if a user's history needs summarization (over threshold).
    pub async fn needs_summarization(&self, user_id: &str, threshold: i64) -> Result<bool> {
        let count = self.get_message_count(user_id).await?;
        Ok(count > threshold)
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
        mem.save_exchange("u1", "telegram", "Hello", "Hi there!").await.unwrap();
        mem.save_exchange("u1", "telegram", "How are you?", "I'm good!").await.unwrap();

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
            mem.save_exchange("u1", "telegram", &format!("msg{}", i), &format!("reply{}", i)).await.unwrap();
        }
        assert_eq!(mem.get_message_count("u1").await.unwrap(), 40);

        mem.save_summary_and_prune("u1", "User discussed topics 0-19", 10).await.unwrap();

        let summary = mem.get_summary("u1").await.unwrap();
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("topics 0-19"));

        // Only 10 most recent messages should remain
        let count = mem.get_message_count("u1").await.unwrap();
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_needs_summarization() {
        let mem = make_memory().await;
        assert!(!mem.needs_summarization("u1", 30).await.unwrap());

        for i in 0..20 {
            mem.save_exchange("u1", "telegram", &format!("m{}", i), &format!("r{}", i)).await.unwrap();
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
}
