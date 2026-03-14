//! Fiqh query methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FiqhRuling {
    pub id: i64,
    pub topic: String,
    pub subtopic: String,
    pub madhab: String,
    pub ruling: String,
    pub evidence: String,
    pub source: String,
    pub language: String,
}

/// Search fiqh rulings by keyword with optional madhab filter.
pub async fn search(pool: &SqlitePool, query: &str, madhab: Option<&str>, limit: i64) -> Result<Vec<FiqhRuling>> {
    let mut sql = String::from(
        "SELECT r.id, r.topic, r.subtopic, r.madhab, r.ruling, r.evidence, r.source, r.language FROM fiqh_rulings r JOIN fiqh_fts f ON r.rowid = f.rowid WHERE fiqh_fts MATCH ?"
    );
    if madhab.is_some() {
        sql.push_str(" AND r.madhab = ?");
    }
    sql.push_str(" LIMIT ?");

    let mut q = sqlx::query_as::<_, FiqhRuling>(&sql).bind(query);
    if let Some(m) = madhab {
        q = q.bind(m);
    }
    q = q.bind(limit);

    let results = q.fetch_all(pool).await?;
    Ok(results)
}

/// Get all rulings for a specific topic across all madhab.
pub async fn by_topic(pool: &SqlitePool, topic: &str) -> Result<Vec<FiqhRuling>> {
    let results = sqlx::query_as::<_, FiqhRuling>(
        "SELECT id, topic, subtopic, madhab, ruling, evidence, source, language FROM fiqh_rulings WHERE topic = ? ORDER BY madhab"
    )
    .bind(topic)
    .fetch_all(pool)
    .await?;
    Ok(results)
}

/// List all unique topics.
pub async fn list_topics(pool: &SqlitePool) -> Result<Vec<String>> {
    let topics: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT topic FROM fiqh_rulings ORDER BY topic"
    )
    .fetch_all(pool)
    .await?;
    Ok(topics.into_iter().map(|t| t.0).collect())
}

/// Count total rulings.
pub async fn count(pool: &SqlitePool) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM fiqh_rulings")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Insert a fiqh ruling.
pub async fn insert(pool: &SqlitePool, ruling: &FiqhRuling) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO fiqh_rulings (topic, subtopic, madhab, ruling, evidence, source, language) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&ruling.topic)
    .bind(&ruling.subtopic)
    .bind(&ruling.madhab)
    .bind(&ruling.ruling)
    .bind(&ruling.evidence)
    .bind(&ruling.source)
    .bind(&ruling.language)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> crate::IslamicDb {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        let pool = db.pool();

        for (madhab, ruling) in [
            ("shafii", "Permissible during travel of approximately 82km or more"),
            ("hanafi", "Only permitted at Arafah and Muzdalifah during Hajj"),
            ("maliki", "Permissible during travel, also for rain and illness"),
            ("hanbali", "Permissible during travel, illness, rain, and genuine hardship"),
        ] {
            sqlx::query("INSERT INTO fiqh_rulings (topic, subtopic, madhab, ruling, evidence, source, language) VALUES ('prayer', 'combining prayers', ?, ?, 'Quran 4:101; Muslim 705', 'Classical fiqh texts', 'en')")
                .bind(madhab).bind(ruling)
                .execute(pool).await.unwrap();
        }

        sqlx::query("INSERT INTO fiqh_fts(rowid, topic, subtopic, ruling, evidence) SELECT rowid, topic, subtopic, ruling, evidence FROM fiqh_rulings")
            .execute(pool).await.unwrap();

        db
    }

    #[tokio::test]
    async fn test_search_fiqh() {
        let db = setup_db().await;
        let results = search(db.pool(), "combining prayers", None, 10).await.unwrap();
        assert_eq!(results.len(), 4);
    }

    #[tokio::test]
    async fn test_search_by_madhab() {
        let db = setup_db().await;
        let results = search(db.pool(), "combining prayers", Some("shafii"), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ruling.contains("82km"));
    }

    #[tokio::test]
    async fn test_by_topic() {
        let db = setup_db().await;
        let results = by_topic(db.pool(), "prayer").await.unwrap();
        assert_eq!(results.len(), 4);
    }

    #[tokio::test]
    async fn test_list_topics() {
        let db = setup_db().await;
        let topics = list_topics(db.pool()).await.unwrap();
        assert_eq!(topics, vec!["prayer"]);
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        assert_eq!(count(db.pool()).await.unwrap(), 4);
    }
}
