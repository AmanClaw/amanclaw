//! Hadith query methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HadithEntry {
    pub id: i64,
    pub collection: String,
    pub book_number: i64,
    pub hadith_number: i64,
    pub text_ar: String,
    pub text_en: String,
    pub grade: String,
    pub graded_by: String,
    pub chapter: String,
}

/// Lookup a specific hadith by collection and number.
pub async fn lookup(pool: &SqlitePool, collection: &str, hadith_number: i64) -> Result<Option<HadithEntry>> {
    let entry = sqlx::query_as::<_, HadithEntry>(
        "SELECT id, collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter FROM hadith WHERE collection = ? AND hadith_number = ?"
    )
    .bind(collection)
    .bind(hadith_number)
    .fetch_optional(pool)
    .await?;
    Ok(entry)
}

/// Full-text search across hadith collections with optional grade filter.
pub async fn search(pool: &SqlitePool, query: &str, collection: Option<&str>, grade: Option<&str>, limit: i64) -> Result<Vec<HadithEntry>> {
    let mut sql = String::from(
        "SELECT h.id, h.collection, h.book_number, h.hadith_number, h.text_ar, h.text_en, h.grade, h.graded_by, h.chapter FROM hadith h JOIN hadith_fts f ON h.rowid = f.rowid WHERE hadith_fts MATCH ?"
    );
    if collection.is_some() {
        sql.push_str(" AND h.collection = ?");
    }
    if grade.is_some() {
        sql.push_str(" AND h.grade = ?");
    }
    sql.push_str(" LIMIT ?");

    let mut q = sqlx::query_as::<_, HadithEntry>(&sql).bind(query);
    if let Some(c) = collection {
        q = q.bind(c);
    }
    if let Some(g) = grade {
        q = q.bind(g);
    }
    q = q.bind(limit);

    let results = q.fetch_all(pool).await?;
    Ok(results)
}

/// Browse hadith by collection and book number.
pub async fn browse(pool: &SqlitePool, collection: &str, book_number: Option<i64>, limit: i64) -> Result<Vec<HadithEntry>> {
    let entries = if let Some(book) = book_number {
        sqlx::query_as::<_, HadithEntry>(
            "SELECT id, collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter FROM hadith WHERE collection = ? AND book_number = ? ORDER BY hadith_number LIMIT ?"
        )
        .bind(collection)
        .bind(book)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, HadithEntry>(
            "SELECT id, collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter FROM hadith WHERE collection = ? ORDER BY hadith_number LIMIT ?"
        )
        .bind(collection)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(entries)
}

/// Count hadith by collection (or all).
pub async fn count(pool: &SqlitePool, collection: Option<&str>) -> Result<i64> {
    let (count,): (i64,) = if let Some(c) = collection {
        sqlx::query_as("SELECT COUNT(*) FROM hadith WHERE collection = ?")
            .bind(c)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_as("SELECT COUNT(*) FROM hadith")
            .fetch_one(pool)
            .await?
    };
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> crate::IslamicDb {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        let pool = db.pool();

        sqlx::query("INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('bukhari', 1, 1, 'إنما الأعمال بالنيات', 'Actions are judged by intentions', 'sahih', 'al-albani', 'Revelation')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('muslim', 1, 1, 'بني الإسلام على خمس', 'Islam is built on five pillars', 'sahih', 'darussalam', 'Faith')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('tirmidhi', 1, 100, 'من حسن إسلام المرء', 'Part of good Islam is leaving what does not concern', 'hasan', 'al-albani', 'Faith')")
            .execute(pool).await.unwrap();

        sqlx::query("INSERT INTO hadith_fts(rowid, collection, hadith_number, text_ar, text_en, chapter) SELECT rowid, collection, hadith_number, text_ar, text_en, chapter FROM hadith")
            .execute(pool).await.unwrap();

        db
    }

    #[tokio::test]
    async fn test_lookup() {
        let db = setup_db().await;
        let h = lookup(db.pool(), "bukhari", 1).await.unwrap();
        assert!(h.is_some());
        assert!(h.unwrap().text_en.contains("intentions"));
    }

    #[tokio::test]
    async fn test_lookup_not_found() {
        let db = setup_db().await;
        let h = lookup(db.pool(), "bukhari", 9999).await.unwrap();
        assert!(h.is_none());
    }

    #[tokio::test]
    async fn test_search_all() {
        let db = setup_db().await;
        let results = search(db.pool(), "Islam", None, None, 10).await.unwrap();
        assert!(results.len() >= 1);
    }

    #[tokio::test]
    async fn test_search_by_collection() {
        let db = setup_db().await;
        let results = search(db.pool(), "Islam", Some("muslim"), None, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].collection, "muslim");
    }

    #[tokio::test]
    async fn test_browse() {
        let db = setup_db().await;
        let results = browse(db.pool(), "bukhari", None, 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        assert_eq!(count(db.pool(), None).await.unwrap(), 3);
        assert_eq!(count(db.pool(), Some("bukhari")).await.unwrap(), 1);
    }
}
