//! API sync engine for importing Islamic data from Quran.com and Sunnah.com.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncStatus {
    pub dataset: String,
    pub last_synced: String,
    pub version: String,
    pub record_count: i64,
}

/// Get sync status for all datasets.
pub async fn get_all_status(pool: &SqlitePool) -> Result<Vec<SyncStatus>> {
    let statuses = sqlx::query_as::<_, SyncStatus>(
        "SELECT dataset, last_synced, version, record_count FROM sync_metadata ORDER BY dataset"
    )
    .fetch_all(pool)
    .await?;
    Ok(statuses)
}

/// Get sync status for a specific dataset.
pub async fn get_status(pool: &SqlitePool, dataset: &str) -> Result<Option<SyncStatus>> {
    let status = sqlx::query_as::<_, SyncStatus>(
        "SELECT dataset, last_synced, version, record_count FROM sync_metadata WHERE dataset = ?"
    )
    .bind(dataset)
    .fetch_optional(pool)
    .await?;
    Ok(status)
}

/// Update sync metadata after a successful import.
pub async fn update_metadata(pool: &SqlitePool, dataset: &str, record_count: i64) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR REPLACE INTO sync_metadata (dataset, last_synced, version, record_count) VALUES (?, ?, '1.0', ?)"
    )
    .bind(dataset)
    .bind(&now)
    .bind(record_count)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_metadata_roundtrip() {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        let pool = db.pool();

        // Initially empty
        let all = get_all_status(pool).await.unwrap();
        assert!(all.is_empty());

        // Update
        update_metadata(pool, "quran", 6236).await.unwrap();
        update_metadata(pool, "hadith_bukhari", 7563).await.unwrap();

        // Check
        let all = get_all_status(pool).await.unwrap();
        assert_eq!(all.len(), 2);

        let quran = get_status(pool, "quran").await.unwrap().unwrap();
        assert_eq!(quran.record_count, 6236);
        assert!(!quran.last_synced.is_empty());
    }
}
