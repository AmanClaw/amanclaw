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

/// Sync Quran data from Quran.com API v4.
/// Downloads all 6,236 verses with Malay and English translations.
pub async fn sync_quran(pool: &SqlitePool) -> Result<i64> {
    let client = reqwest::Client::new();
    let mut total_inserted: i64 = 0;

    for surah in 1..=114 {
        tracing::info!(surah, "Syncing Quran surah {}/114", surah);

        // Fetch verses with translations
        let url = format!(
            "https://api.quran.com/api/v4/verses/by_chapter/{}?language=en&translations=131,39&fields=text_uthmani,text_imlaei&per_page=300",
            surah
        );

        let resp = client.get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!(surah, status = %resp.status(), "Failed to fetch surah");
            continue;
        }

        let data: serde_json::Value = resp.json().await?;
        let empty = vec![];
        let verses = data["verses"].as_array().unwrap_or(&empty);

        for verse in verses {
            let ayat = verse["verse_number"].as_i64().unwrap_or(0);
            let text_uthmani = verse["text_uthmani"].as_str().unwrap_or("");
            let text_simple = verse["text_imlaei"].as_str().unwrap_or("");

            // Extract translations
            let translations = verse["translations"].as_array();
            let mut translation_en = String::new();
            let mut translation_ms = String::new();

            if let Some(trans) = translations {
                for t in trans {
                    let resource_id = t["resource_id"].as_i64().unwrap_or(0);
                    let text = t["text"].as_str().unwrap_or("");
                    // Strip HTML tags
                    let clean = text.replace("<sup", " <sup")
                        .split('<').map(|s| {
                            if let Some(idx) = s.find('>') { &s[idx + 1..] } else { s }
                        }).collect::<Vec<_>>().join("").trim().to_string();

                    match resource_id {
                        131 => translation_en = clean,
                        39 => translation_ms = clean,
                        _ => {}
                    }
                }
            }

            sqlx::query(
                "INSERT OR REPLACE INTO quran_ayat (surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page) VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0)"
            )
            .bind(surah as i64)
            .bind(ayat)
            .bind(text_uthmani)
            .bind(text_simple)
            .bind(&translation_ms)
            .bind(&translation_en)
            .execute(pool)
            .await?;

            total_inserted += 1;
        }
    }

    // Rebuild FTS index
    sqlx::query("DELETE FROM quran_fts").execute(pool).await?;
    sqlx::query("INSERT INTO quran_fts(rowid, surah, ayat, text, translation_ms, translation_en) SELECT rowid, surah, ayat, text_uthmani, translation_ms, translation_en FROM quran_ayat")
        .execute(pool)
        .await?;

    update_metadata(pool, "quran", total_inserted).await?;
    tracing::info!(total = total_inserted, "Quran sync complete");
    Ok(total_inserted)
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
