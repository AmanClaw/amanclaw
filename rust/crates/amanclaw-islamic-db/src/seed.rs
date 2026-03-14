//! Seed data loader for fiqh rulings.

use anyhow::Result;
use sqlx::SqlitePool;

/// Load fiqh seed data from the embedded JSON file.
pub async fn load_fiqh_seed(pool: &SqlitePool) -> Result<i64> {
    let seed_json = include_str!("../data/fiqh_seed.json");
    let rulings: Vec<crate::fiqh::FiqhRuling> = serde_json::from_str(seed_json)?;

    let mut count = 0i64;
    for ruling in &rulings {
        crate::fiqh::insert(pool, ruling).await?;
        count += 1;
    }

    // Rebuild FTS index
    sqlx::query("DELETE FROM fiqh_fts").execute(pool).await.ok();
    sqlx::query("INSERT INTO fiqh_fts(rowid, topic, subtopic, ruling, evidence) SELECT rowid, topic, subtopic, ruling, evidence FROM fiqh_rulings")
        .execute(pool)
        .await?;

    crate::sync::update_metadata(pool, "fiqh", count).await?;
    tracing::info!(count, "Fiqh seed data loaded");
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_fiqh_seed() {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        let count = load_fiqh_seed(db.pool()).await.unwrap();
        assert!(count > 0, "Should load at least 1 ruling");

        // Verify data is searchable
        let results = crate::fiqh::search(db.pool(), "combining prayers", None, 10).await.unwrap();
        assert!(!results.is_empty(), "Should find combining prayers rulings");
    }
}
