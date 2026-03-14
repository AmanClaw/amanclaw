//! Database schema for Islamic knowledge store.

pub const INIT_SQL: &str = r#"
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
CREATE TABLE IF NOT EXISTS quran_tafsir (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    surah       INTEGER NOT NULL,
    ayat        INTEGER NOT NULL,
    tafsir_name TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'en',
    text        TEXT NOT NULL DEFAULT '',
    UNIQUE(surah, ayat, tafsir_name, language)
);
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
CREATE TABLE IF NOT EXISTS sync_metadata (
    dataset      TEXT PRIMARY KEY,
    last_synced  TEXT NOT NULL DEFAULT '',
    version      TEXT NOT NULL DEFAULT '',
    record_count INTEGER NOT NULL DEFAULT 0
);
"#;
