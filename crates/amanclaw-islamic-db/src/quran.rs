//! Quran query methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QuranVerse {
    pub surah: i64,
    pub ayat: i64,
    pub text_uthmani: String,
    pub text_simple: String,
    pub translation_ms: String,
    pub translation_en: String,
    pub juz: i64,
    pub hizb: i64,
    pub page: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TafsirEntry {
    pub surah: i64,
    pub ayat: i64,
    pub tafsir_name: String,
    pub language: String,
    pub text: String,
}

/// Get a specific verse by surah and ayat number.
pub async fn get_verse(pool: &SqlitePool, surah: i64, ayat: i64) -> Result<Option<QuranVerse>> {
    let verse = sqlx::query_as::<_, QuranVerse>(
        "SELECT surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page FROM quran_ayat WHERE surah = ? AND ayat = ?"
    )
    .bind(surah)
    .bind(ayat)
    .fetch_optional(pool)
    .await?;
    Ok(verse)
}

/// Full-text search across Quran translations.
pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<QuranVerse>> {
    let results = sqlx::query_as::<_, QuranVerse>(
        r#"SELECT q.surah, q.ayat, q.text_uthmani, q.text_simple, q.translation_ms, q.translation_en, q.juz, q.hizb, q.page
        FROM quran_ayat q
        JOIN quran_fts f ON q.rowid = f.rowid
        WHERE quran_fts MATCH ?
        LIMIT ?"#,
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(results)
}

/// Get tafsir for a specific verse.
pub async fn get_tafsir(pool: &SqlitePool, surah: i64, ayat: i64, tafsir_name: &str) -> Result<Vec<TafsirEntry>> {
    let entries = sqlx::query_as::<_, TafsirEntry>(
        "SELECT surah, ayat, tafsir_name, language, text FROM quran_tafsir WHERE surah = ? AND ayat = ? AND tafsir_name = ?"
    )
    .bind(surah)
    .bind(ayat)
    .bind(tafsir_name)
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

/// Get all tafsir for a verse (all available tafsir).
pub async fn get_all_tafsir(pool: &SqlitePool, surah: i64, ayat: i64) -> Result<Vec<TafsirEntry>> {
    let entries = sqlx::query_as::<_, TafsirEntry>(
        "SELECT surah, ayat, tafsir_name, language, text FROM quran_tafsir WHERE surah = ? AND ayat = ?"
    )
    .bind(surah)
    .bind(ayat)
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

/// Count total ayat in the database.
pub async fn count(pool: &SqlitePool) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quran_ayat")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> crate::IslamicDb {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        // Insert test data
        sqlx::query(
            "INSERT INTO quran_ayat (surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page) VALUES (1, 1, 'بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ', 'bismillah', 'Dengan nama Allah Yang Maha Pemurah lagi Maha Penyayang', 'In the name of Allah the Most Gracious the Most Merciful', 1, 1, 1)"
        )
        .execute(db.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO quran_ayat (surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page) VALUES (2, 255, 'ٱللَّهُ لَآ إِلَـٰهَ إِلَّا هُوَ', 'ayat al-kursi', 'Allah tiada tuhan selain Dia', 'Allah there is no deity except Him', 3, 5, 42)"
        )
        .execute(db.pool())
        .await
        .unwrap();

        // Insert FTS data
        sqlx::query("INSERT INTO quran_fts(rowid, surah, ayat, text, translation_ms, translation_en) SELECT rowid, surah, ayat, text_uthmani, translation_ms, translation_en FROM quran_ayat")
            .execute(db.pool())
            .await
            .unwrap();

        // Insert tafsir
        sqlx::query(
            "INSERT INTO quran_tafsir (surah, ayat, tafsir_name, language, text) VALUES (1, 1, 'ibn_kathir', 'en', 'This is the opening verse of the Quran, known as the Basmalah.')"
        )
        .execute(db.pool())
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn test_get_verse() {
        let db = setup_db().await;
        let verse = get_verse(db.pool(), 1, 1).await.unwrap();
        assert!(verse.is_some());
        let v = verse.unwrap();
        assert_eq!(v.surah, 1);
        assert_eq!(v.ayat, 1);
        assert!(v.translation_en.contains("Gracious"));
    }

    #[tokio::test]
    async fn test_get_verse_not_found() {
        let db = setup_db().await;
        let verse = get_verse(db.pool(), 999, 999).await.unwrap();
        assert!(verse.is_none());
    }

    #[tokio::test]
    async fn test_search() {
        let db = setup_db().await;
        let results = search(db.pool(), "Gracious", 5).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].surah, 1);
    }

    #[tokio::test]
    async fn test_get_tafsir() {
        let db = setup_db().await;
        let tafsir = get_tafsir(db.pool(), 1, 1, "ibn_kathir").await.unwrap();
        assert_eq!(tafsir.len(), 1);
        assert!(tafsir[0].text.contains("Basmalah"));
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        let count = count(db.pool()).await.unwrap();
        assert_eq!(count, 2);
    }
}
