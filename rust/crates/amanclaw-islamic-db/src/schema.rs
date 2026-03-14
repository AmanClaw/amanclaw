//! Database schema for Islamic knowledge store.

pub const INIT_SQL: &str = r#"
-- Quran: 6,236 ayat with translations
CREATE TABLE IF NOT EXISTS quran_ayat (
    surah       INTEGER NOT NULL,
    ayat        INTEGER NOT NULL,
    text_uthmani TEXT NOT NULL DEFAULT '',
    text_simple  TEXT NOT NULL DEFAULT '',
    translation_ms TEXT NOT NULL DEFAULT '',
    translation_en TEXT NOT NULL DEFAULT '',
    juz         INTEGER NOT NULL DEFAULT 0,
    hizb        INTEGER NOT NULL DEFAULT 0,
    page        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (surah, ayat)
);

-- Tafsir: multiple tafsir per ayat
CREATE TABLE IF NOT EXISTS quran_tafsir (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    surah       INTEGER NOT NULL,
    ayat        INTEGER NOT NULL,
    tafsir_name TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'en',
    text        TEXT NOT NULL DEFAULT '',
    UNIQUE(surah, ayat, tafsir_name, language)
);
CREATE INDEX IF NOT EXISTS idx_tafsir_surah_ayat ON quran_tafsir(surah, ayat);

-- Hadith: ~40,000 across 6 collections
CREATE TABLE IF NOT EXISTS hadith (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    collection      TEXT NOT NULL,
    book_number     INTEGER NOT NULL DEFAULT 0,
    hadith_number   INTEGER NOT NULL DEFAULT 0,
    text_ar         TEXT NOT NULL DEFAULT '',
    text_en         TEXT NOT NULL DEFAULT '',
    grade           TEXT NOT NULL DEFAULT '',
    graded_by       TEXT NOT NULL DEFAULT '',
    chapter         TEXT NOT NULL DEFAULT '',
    UNIQUE(collection, hadith_number)
);
CREATE INDEX IF NOT EXISTS idx_hadith_collection ON hadith(collection);
CREATE INDEX IF NOT EXISTS idx_hadith_grade ON hadith(grade);

-- Fiqh: scholarly rulings by topic + madhab
CREATE TABLE IF NOT EXISTS fiqh_rulings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    topic       TEXT NOT NULL,
    subtopic    TEXT NOT NULL DEFAULT '',
    madhab      TEXT NOT NULL,
    ruling      TEXT NOT NULL,
    evidence    TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL DEFAULT '',
    language    TEXT NOT NULL DEFAULT 'en'
);
CREATE INDEX IF NOT EXISTS idx_fiqh_topic ON fiqh_rulings(topic);
CREATE INDEX IF NOT EXISTS idx_fiqh_madhab ON fiqh_rulings(madhab);

-- FTS5 full-text search indexes
CREATE VIRTUAL TABLE IF NOT EXISTS quran_fts USING fts5(
    surah, ayat, text, translation_ms, translation_en,
    content='quran_ayat',
    content_rowid='rowid'
);

CREATE VIRTUAL TABLE IF NOT EXISTS hadith_fts USING fts5(
    collection, hadith_number, text_ar, text_en, chapter,
    content='hadith',
    content_rowid='rowid'
);

CREATE VIRTUAL TABLE IF NOT EXISTS fiqh_fts USING fts5(
    topic, subtopic, ruling, evidence,
    content='fiqh_rulings',
    content_rowid='rowid'
);

-- Sync metadata tracking
CREATE TABLE IF NOT EXISTS sync_metadata (
    dataset      TEXT PRIMARY KEY,
    last_synced  TEXT NOT NULL DEFAULT '',
    version      TEXT NOT NULL DEFAULT '',
    record_count INTEGER NOT NULL DEFAULT 0
);
"#;

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_schema_creates_all_tables() {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        let pool = db.pool();

        // Verify all tables exist
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )
        .fetch_all(pool)
        .await
        .unwrap();

        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        assert!(names.contains(&"quran_ayat"), "quran_ayat table missing");
        assert!(names.contains(&"quran_tafsir"), "quran_tafsir table missing");
        assert!(names.contains(&"hadith"), "hadith table missing");
        assert!(names.contains(&"fiqh_rulings"), "fiqh_rulings table missing");
        assert!(names.contains(&"sync_metadata"), "sync_metadata table missing");
    }

    #[tokio::test]
    async fn test_empty_db_is_empty() {
        let db = crate::IslamicDb::new(":memory:").await.unwrap();
        assert!(db.is_empty().await.unwrap());
    }
}
