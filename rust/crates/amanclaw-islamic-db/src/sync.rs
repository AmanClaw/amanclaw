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

/// Sync a single hadith collection from Sunnah.com API.
pub async fn sync_hadith_collection(pool: &SqlitePool, collection: &str, api_key: Option<&str>) -> Result<i64> {
    let client = reqwest::Client::new();
    let mut total_inserted: i64 = 0;
    let mut page = 1;
    let per_page = 50;

    loop {
        tracing::info!(collection, page, "Syncing hadith page {}", page);

        let url = format!(
            "https://api.sunnah.com/v1/collections/{}/hadiths?page={}&limit={}",
            collection, page, per_page
        );

        let mut req = client.get(&url)
            .header("Accept", "application/json");
        if let Some(key) = api_key {
            req = req.header("x-api-key", key);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            tracing::warn!(collection, status = %resp.status(), "Failed to fetch hadith page");
            break;
        }

        let data: serde_json::Value = resp.json().await?;
        let empty = vec![];
        let hadiths = data["data"].as_array().unwrap_or(&empty);

        if hadiths.is_empty() {
            break;
        }

        for h in hadiths {
            let hadith_number = h["hadithNumber"].as_i64()
                .or_else(|| h["hadithNumber"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            let book_number = h["bookNumber"].as_i64()
                .or_else(|| h["bookNumber"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0);

            let text_ar = h["hadith"].as_array()
                .and_then(|arr| arr.iter().find(|t| t["lang"].as_str() == Some("ar")))
                .and_then(|t| t["body"].as_str())
                .unwrap_or("");

            let text_en = h["hadith"].as_array()
                .and_then(|arr| arr.iter().find(|t| t["lang"].as_str() == Some("en")))
                .and_then(|t| t["body"].as_str())
                .unwrap_or("");

            let grade = h["grade"].as_str()
                .or_else(|| h["grades"].as_array().and_then(|g| g.first()).and_then(|g| g["grade"].as_str()))
                .unwrap_or("");

            let graded_by = h["grades"].as_array()
                .and_then(|g| g.first())
                .and_then(|g| g["name"].as_str())
                .unwrap_or("");

            let chapter = h["chapterTitle"].as_str()
                .or_else(|| h["chapter"].as_object().and_then(|c| c.get("english")).and_then(|e| e.as_str()))
                .unwrap_or("");

            sqlx::query(
                "INSERT OR REPLACE INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(collection)
            .bind(book_number)
            .bind(hadith_number)
            .bind(text_ar)
            .bind(text_en)
            .bind(grade)
            .bind(graded_by)
            .bind(chapter)
            .execute(pool)
            .await?;

            total_inserted += 1;
        }

        if hadiths.len() < per_page as usize {
            break;
        }
        page += 1;
    }

    // Rebuild FTS index for this collection
    sqlx::query("DELETE FROM hadith_fts WHERE collection = ?")
        .bind(collection)
        .execute(pool)
        .await
        .ok(); // FTS5 content tables don't support WHERE — rebuild all instead

    let dataset = format!("hadith_{collection}");
    update_metadata(pool, &dataset, total_inserted).await?;
    tracing::info!(collection, total = total_inserted, "Hadith sync complete");
    Ok(total_inserted)
}

/// Sync all 6 hadith collections.
pub async fn sync_all_hadith(pool: &SqlitePool, api_key: Option<&str>) -> Result<i64> {
    let collections = ["bukhari", "muslim", "abudawud", "tirmidhi", "nasai", "ibnmajah"];
    let mut total = 0i64;

    for collection in &collections {
        let count = sync_hadith_collection(pool, collection, api_key).await?;
        total += count;
    }

    // Rebuild full FTS index
    sqlx::query("DELETE FROM hadith_fts").execute(pool).await.ok();
    sqlx::query("INSERT INTO hadith_fts(rowid, collection, hadith_number, text_ar, text_en, chapter) SELECT rowid, collection, hadith_number, text_ar, text_en, chapter FROM hadith")
        .execute(pool)
        .await?;

    Ok(total)
}

/// Sync tafsir from Quran.com API for a specific tafsir resource.
pub async fn sync_tafsir(pool: &SqlitePool, tafsir_name: &str, resource_id: i64) -> Result<i64> {
    let client = reqwest::Client::new();
    let mut total_inserted: i64 = 0;

    for surah in 1..=114 {
        tracing::info!(surah, tafsir = tafsir_name, "Syncing tafsir surah {}/114", surah);

        let url = format!(
            "https://api.quran.com/api/v4/tafsirs/{}?chapter_number={}",
            resource_id, surah
        );

        let resp = client.get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!(surah, tafsir = tafsir_name, "Failed to fetch tafsir");
            continue;
        }

        let data: serde_json::Value = resp.json().await?;
        let empty = vec![];
        let tafsirs = data["tafsirs"].as_array().unwrap_or(&empty);

        for t in tafsirs {
            let verse_key = t["verse_key"].as_str().unwrap_or("");
            let parts: Vec<&str> = verse_key.split(':').collect();
            if parts.len() != 2 { continue; }
            let s: i64 = parts[0].parse().unwrap_or(0);
            let a: i64 = parts[1].parse().unwrap_or(0);

            let text = t["text"].as_str().unwrap_or("");
            let language = t["language_name"].as_str().unwrap_or("en");
            let lang_code = match language {
                "english" | "English" => "en",
                "arabic" | "Arabic" => "ar",
                _ => "en",
            };

            // Strip HTML
            let clean_text: String = text.split('<').map(|s| {
                if let Some(idx) = s.find('>') { &s[idx + 1..] } else { s }
            }).collect::<Vec<_>>().join("").trim().to_string();

            sqlx::query(
                "INSERT OR REPLACE INTO quran_tafsir (surah, ayat, tafsir_name, language, text) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(s)
            .bind(a)
            .bind(tafsir_name)
            .bind(lang_code)
            .bind(&clean_text)
            .execute(pool)
            .await?;

            total_inserted += 1;
        }
    }

    let dataset = format!("tafsir_{tafsir_name}");
    update_metadata(pool, &dataset, total_inserted).await?;
    tracing::info!(tafsir = tafsir_name, total = total_inserted, "Tafsir sync complete");
    Ok(total_inserted)
}

/// Master sync: import all Islamic data.
pub async fn sync_all(pool: &SqlitePool, sunnah_api_key: Option<&str>) -> Result<()> {
    tracing::info!("Starting full Islamic data sync...");

    tracing::info!("Syncing Quran...");
    sync_quran(pool).await?;

    tracing::info!("Syncing Tafsir Ibn Kathir...");
    sync_tafsir(pool, "ibn_kathir", 169).await?;

    tracing::info!("Syncing Tafsir Al-Jalalayn...");
    sync_tafsir(pool, "jalalayn", 74).await?;

    tracing::info!("Syncing Hadith collections...");
    sync_all_hadith(pool, sunnah_api_key).await?;

    tracing::info!("Full Islamic data sync complete!");
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
