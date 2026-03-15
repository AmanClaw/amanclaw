pub mod schema;
pub mod quran;
pub mod hadith;
pub mod fiqh;
pub mod sync;
pub mod seed;

use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

/// Local SQLite knowledge store for Islamic texts.
#[derive(Clone)]
pub struct IslamicDb {
    pool: SqlitePool,
}

impl IslamicDb {
    /// Create or open the Islamic knowledge database.
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

        // Apply schema
        sqlx::raw_sql(schema::INIT_SQL).execute(&pool).await?;

        Ok(Self { pool })
    }

    /// Check if the database has any data.
    pub async fn is_empty(&self) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quran_ayat")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0 == 0)
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
