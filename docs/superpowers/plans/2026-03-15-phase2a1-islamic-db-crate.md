# Plan 2A-1: Islamic DB Crate + Schema + Sync

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `amanclaw-islamic-db` crate with SQLite schema for Quran, Hadith, and Fiqh data, plus an API-based sync engine and CLI commands for data management.

**Architecture:** New crate follows `amanclaw-memory` patterns — sqlx 0.8, async pool, raw SQL schema with FTS5. Sync engine downloads from Quran.com and Sunnah.com APIs, stores in local SQLite. CLI commands `amanclaw islamic sync/status` manage the data. Progress events emitted via tracing for WebSocket subscribers.

**Tech Stack:** Rust, sqlx 0.8 (SQLite), reqwest (HTTP client), serde_json, FTS5, clap 4

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rust/Cargo.toml` | MODIFY | Add `amanclaw-islamic-db` to workspace members |
| `rust/crates/amanclaw-islamic-db/Cargo.toml` | CREATE | Crate manifest with sqlx, reqwest, serde deps |
| `rust/crates/amanclaw-islamic-db/src/lib.rs` | CREATE | `IslamicDb` struct — pool init, empty check, pool accessor |
| `rust/crates/amanclaw-islamic-db/src/schema.rs` | CREATE | INIT_SQL constant with all CREATE TABLE/INDEX/FTS5 |
| `rust/crates/amanclaw-islamic-db/src/quran.rs` | CREATE | Quran query methods (verse, search, tafsir) |
| `rust/crates/amanclaw-islamic-db/src/hadith.rs` | CREATE | Hadith query methods (search, lookup, browse) |
| `rust/crates/amanclaw-islamic-db/src/fiqh.rs` | CREATE | Fiqh query methods (ask, browse, topics) |
| `rust/crates/amanclaw-islamic-db/src/sync.rs` | CREATE | API sync engine (Quran.com, Sunnah.com importers) |
| `rust/crates/amanclaw-islamic-db/src/seed.rs` | CREATE | Fiqh seed data loader from JSON |
| `rust/crates/amanclaw-islamic-db/data/fiqh_seed.json` | CREATE | Initial curated fiqh rulings (~50 entries for MVP) |
| `rust/crates/amanclaw-cli/src/cli.rs` | MODIFY | Add `Islamic` subcommand with `IslamicAction` enum |
| `rust/crates/amanclaw-cli/src/main.rs` | MODIFY | Add `cmd_islamic()` handler |
| `rust/crates/amanclaw-cli/Cargo.toml` | MODIFY | Add `amanclaw-islamic-db` dependency |

---

## Chunk 1: Crate Scaffold + Schema

### Task 1: Create crate skeleton

**Files:**
- Modify: `rust/Cargo.toml`
- Create: `rust/crates/amanclaw-islamic-db/Cargo.toml`
- Create: `rust/crates/amanclaw-islamic-db/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

Create `rust/crates/amanclaw-islamic-db/Cargo.toml`:

```toml
[package]
name = "amanclaw-islamic-db"
version.workspace = true
edition.workspace = true
license = "MIT"
description = "Local SQLite knowledge store for Islamic texts — Quran, Hadith, Fiqh"

[dependencies]
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
anyhow = "1"
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json"] }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

- [ ] **Step 2: Add to workspace**

In `rust/Cargo.toml`, add `"crates/amanclaw-islamic-db"` to the `members` array.

- [ ] **Step 3: Create lib.rs stub**

Create `rust/crates/amanclaw-islamic-db/src/lib.rs`:

```rust
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
```

- [ ] **Step 4: Create empty module stubs**

Create these files with placeholder content:

`rust/crates/amanclaw-islamic-db/src/quran.rs`:
```rust
//! Quran query methods.
```

`rust/crates/amanclaw-islamic-db/src/hadith.rs`:
```rust
//! Hadith query methods.
```

`rust/crates/amanclaw-islamic-db/src/fiqh.rs`:
```rust
//! Fiqh query methods.
```

`rust/crates/amanclaw-islamic-db/src/sync.rs`:
```rust
//! API sync engine for importing Islamic data.
```

`rust/crates/amanclaw-islamic-db/src/seed.rs`:
```rust
//! Seed data loader for fiqh rulings.
```

- [ ] **Step 5: Verify compilation**

Run: `cd rust && cargo check --package amanclaw-islamic-db`
Expected: Compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add rust/Cargo.toml rust/crates/amanclaw-islamic-db/
git commit -m "feat(islamic-db): scaffold amanclaw-islamic-db crate"
```

---

### Task 2: Database schema

**Files:**
- Create: `rust/crates/amanclaw-islamic-db/src/schema.rs`

- [ ] **Step 1: Write schema**

```rust
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
```

- [ ] **Step 2: Write schema test**

Add to `rust/crates/amanclaw-islamic-db/src/schema.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

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
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test --package amanclaw-islamic-db -- --nocapture`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/schema.rs
git commit -m "feat(islamic-db): add SQLite schema for Quran, Hadith, Fiqh, and sync metadata"
```

---

## Chunk 2: Query Modules

### Task 3: Quran query module

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/quran.rs`

- [ ] **Step 1: Write tests first**

```rust
//! Quran query methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TafsirEntry {
    pub surah: i64,
    pub ayat: i64,
    pub tafsir_name: String,
    pub language: String,
    pub text: String,
}

/// Get a specific verse by surah and ayat number.
pub async fn get_verse(pool: &SqlitePool, surah: i64, ayat: i64) -> Result<Option<QuranVerse>> {
    let verse = sqlx::query_as!(
        QuranVerse,
        "SELECT surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page FROM quran_ayat WHERE surah = ? AND ayat = ?",
        surah, ayat
    )
    .fetch_optional(pool)
    .await?;
    Ok(verse)
}

/// Full-text search across Quran translations.
pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<QuranVerse>> {
    let results = sqlx::query_as!(
        QuranVerse,
        r#"SELECT q.surah, q.ayat, q.text_uthmani, q.text_simple, q.translation_ms, q.translation_en, q.juz, q.hizb, q.page
        FROM quran_ayat q
        JOIN quran_fts f ON q.rowid = f.rowid
        WHERE quran_fts MATCH ?
        LIMIT ?"#,
        query, limit
    )
    .fetch_all(pool)
    .await?;
    Ok(results)
}

/// Get tafsir for a specific verse.
pub async fn get_tafsir(pool: &SqlitePool, surah: i64, ayat: i64, tafsir_name: &str) -> Result<Vec<TafsirEntry>> {
    let entries = sqlx::query_as!(
        TafsirEntry,
        "SELECT surah, ayat, tafsir_name, language, text FROM quran_tafsir WHERE surah = ? AND ayat = ? AND tafsir_name = ?",
        surah, ayat, tafsir_name
    )
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

/// Get all tafsir for a verse (all available tafsir).
pub async fn get_all_tafsir(pool: &SqlitePool, surah: i64, ayat: i64) -> Result<Vec<TafsirEntry>> {
    let entries = sqlx::query_as!(
        TafsirEntry,
        "SELECT surah, ayat, tafsir_name, language, text FROM quran_tafsir WHERE surah = ? AND ayat = ?",
        surah, ayat
    )
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
```

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-islamic-db quran -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/quran.rs
git commit -m "feat(islamic-db): add Quran query module with verse lookup, FTS5 search, and tafsir"
```

---

### Task 4: Hadith query module

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/hadith.rs`

- [ ] **Step 1: Implement with tests**

```rust
//! Hadith query methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let entry = sqlx::query_as!(
        HadithEntry,
        "SELECT id, collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter FROM hadith WHERE collection = ? AND hadith_number = ?",
        collection, hadith_number
    )
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
        sqlx::query_as!(
            HadithEntry,
            "SELECT id, collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter FROM hadith WHERE collection = ? AND book_number = ? ORDER BY hadith_number LIMIT ?",
            collection, book, limit
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            HadithEntry,
            "SELECT id, collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter FROM hadith WHERE collection = ? ORDER BY hadith_number LIMIT ?",
            collection, limit
        )
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
```

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-islamic-db hadith -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/hadith.rs
git commit -m "feat(islamic-db): add Hadith query module with lookup, FTS5 search, browse, and grade filtering"
```

---

### Task 5: Fiqh query module

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/fiqh.rs`

- [ ] **Step 1: Implement with tests**

```rust
//! Fiqh query methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let results = sqlx::query_as!(
        FiqhRuling,
        "SELECT id, topic, subtopic, madhab, ruling, evidence, source, language FROM fiqh_rulings WHERE topic = ? ORDER BY madhab",
        topic
    )
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
```

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-islamic-db fiqh -- --nocapture`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/fiqh.rs
git commit -m "feat(islamic-db): add Fiqh query module with multi-madhab search and topic browsing"
```

---

## Chunk 3: Sync Engine

### Task 6: Sync metadata helpers

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/sync.rs`

- [ ] **Step 1: Create sync metadata types and helpers**

```rust
//! API sync engine for importing Islamic data from Quran.com and Sunnah.com.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub dataset: String,
    pub last_synced: String,
    pub version: String,
    pub record_count: i64,
}

/// Get sync status for all datasets.
pub async fn get_all_status(pool: &SqlitePool) -> Result<Vec<SyncStatus>> {
    let statuses = sqlx::query_as!(
        SyncStatus,
        "SELECT dataset, last_synced, version, record_count FROM sync_metadata ORDER BY dataset"
    )
    .fetch_all(pool)
    .await?;
    Ok(statuses)
}

/// Get sync status for a specific dataset.
pub async fn get_status(pool: &SqlitePool, dataset: &str) -> Result<Option<SyncStatus>> {
    let status = sqlx::query_as!(
        SyncStatus,
        "SELECT dataset, last_synced, version, record_count FROM sync_metadata WHERE dataset = ?",
        dataset
    )
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
```

- [ ] **Step 2: Run tests**

Run: `cd rust && cargo test --package amanclaw-islamic-db sync -- --nocapture`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/sync.rs
git commit -m "feat(islamic-db): add sync metadata tracking"
```

---

### Task 7: Quran sync from Quran.com API

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/sync.rs`

- [ ] **Step 1: Add Quran sync function**

Append to `sync.rs`:

```rust
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
        let verses = data["verses"].as_array().unwrap_or(&vec![]);

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
```

- [ ] **Step 2: Commit** (network-dependent — no automated test)

```bash
git add rust/crates/amanclaw-islamic-db/src/sync.rs
git commit -m "feat(islamic-db): add Quran sync from Quran.com API"
```

---

### Task 8: Hadith sync from Sunnah.com API

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/sync.rs`

- [ ] **Step 1: Add Hadith sync function**

Append to `sync.rs`:

```rust
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
        let hadiths = data["data"].as_array().unwrap_or(&vec![]);

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
```

- [ ] **Step 2: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/sync.rs
git commit -m "feat(islamic-db): add Hadith sync from Sunnah.com API (all 6 collections)"
```

---

### Task 9: Tafsir sync + master sync function

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/sync.rs`

- [ ] **Step 1: Add tafsir sync and master sync_all**

Append to `sync.rs`:

```rust
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
        let tafsirs = data["tafsirs"].as_array().unwrap_or(&vec![]);

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
```

- [ ] **Step 2: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/sync.rs
git commit -m "feat(islamic-db): add tafsir sync and master sync_all function"
```

---

### Task 10: Fiqh seed data

**Files:**
- Modify: `rust/crates/amanclaw-islamic-db/src/seed.rs`
- Create: `rust/crates/amanclaw-islamic-db/data/fiqh_seed.json`

- [ ] **Step 1: Create seed JSON (MVP — 12 entries, 3 topics)**

Create `rust/crates/amanclaw-islamic-db/data/fiqh_seed.json`:

```json
[
  {
    "topic": "prayer", "subtopic": "combining prayers during travel",
    "madhab": "shafii", "ruling": "Permissible during travel of approximately 82km or more. Can combine Zuhr+Asr or Maghrib+Isha (jam' taqdim or ta'khir).",
    "evidence": "Quran 4:101; Sahih Muslim 705", "source": "Al-Umm, Imam al-Shafi'i", "language": "en"
  },
  {
    "topic": "prayer", "subtopic": "combining prayers during travel",
    "madhab": "hanafi", "ruling": "Only permitted at Arafah (Zuhr+Asr) and Muzdalifah (Maghrib+Isha) during Hajj.",
    "evidence": "Sahih Bukhari 1662", "source": "Al-Hidayah, al-Marghinani", "language": "en"
  },
  {
    "topic": "prayer", "subtopic": "combining prayers during travel",
    "madhab": "maliki", "ruling": "Permissible during travel. Also permitted for rain, mud, illness, and fear.",
    "evidence": "Sahih Muslim 705; Muwatta Malik", "source": "Al-Mudawwanah, Sahnun", "language": "en"
  },
  {
    "topic": "prayer", "subtopic": "combining prayers during travel",
    "madhab": "hanbali", "ruling": "Permissible during travel, illness, rain, extreme cold, and genuine hardship.",
    "evidence": "Sahih Muslim 705; Sahih Bukhari 1174", "source": "Al-Mughni, Ibn Qudamah", "language": "en"
  },
  {
    "topic": "fasting", "subtopic": "breaking fast accidentally",
    "madhab": "shafii", "ruling": "If one eats or drinks forgetfully, the fast is valid and should be completed. No makeup required.",
    "evidence": "Sahih Bukhari 1933; Sahih Muslim 1155", "source": "Al-Majmu', Imam al-Nawawi", "language": "en"
  },
  {
    "topic": "fasting", "subtopic": "breaking fast accidentally",
    "madhab": "hanafi", "ruling": "Eating or drinking forgetfully does not break the fast. Continue fasting.",
    "evidence": "Sahih Bukhari 1933", "source": "Al-Hidayah, al-Marghinani", "language": "en"
  },
  {
    "topic": "fasting", "subtopic": "breaking fast accidentally",
    "madhab": "maliki", "ruling": "The fast is valid if broken forgetfully. If one remembers mid-eating, must stop immediately.",
    "evidence": "Sahih Muslim 1155", "source": "Al-Mudawwanah, Sahnun", "language": "en"
  },
  {
    "topic": "fasting", "subtopic": "breaking fast accidentally",
    "madhab": "hanbali", "ruling": "Eating or drinking forgetfully does not invalidate the fast. No expiation required.",
    "evidence": "Sahih Bukhari 1933; Sahih Muslim 1155", "source": "Al-Mughni, Ibn Qudamah", "language": "en"
  },
  {
    "topic": "zakat", "subtopic": "nisab threshold",
    "madhab": "consensus", "ruling": "Zakat is obligatory when wealth reaches the nisab: 85 grams of gold or 595 grams of silver (or equivalent currency), held for one lunar year.",
    "evidence": "Sahih Bukhari 1459; Sunan Abu Dawud 1573", "source": "Agreed upon by all four schools", "language": "en"
  },
  {
    "topic": "zakat", "subtopic": "zakat on salary/income",
    "madhab": "shafii", "ruling": "Zakat on income is calculated at 2.5% of net savings that exceed nisab at the end of the haul (lunar year).",
    "evidence": "Quran 2:267", "source": "Contemporary Shafi'i scholars", "language": "en"
  },
  {
    "topic": "zakat", "subtopic": "zakat on salary/income",
    "madhab": "hanafi", "ruling": "Zakat is due on savings exceeding nisab at year-end. Income spent before year-end is not zakatable.",
    "evidence": "Quran 9:103", "source": "Al-Hidayah, al-Marghinani", "language": "en"
  },
  {
    "topic": "zakat", "subtopic": "zakat on salary/income",
    "madhab": "consensus", "ruling": "Modern scholars in Malaysia (JAKIM) recommend 2.5% of gross annual income minus essential expenses, payable monthly or annually.",
    "evidence": "Fatwa JAKIM; Quran 2:267", "source": "JAKIM Fatwa Committee", "language": "en"
  }
]
```

- [ ] **Step 2: Implement seed loader**

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test --package amanclaw-islamic-db seed -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-islamic-db/src/seed.rs rust/crates/amanclaw-islamic-db/data/
git commit -m "feat(islamic-db): add fiqh seed data with 12 multi-madhab rulings"
```

---

## Chunk 4: CLI Commands

### Task 11: Add Islamic subcommand to CLI

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs`
- Modify: `rust/crates/amanclaw-cli/Cargo.toml`

- [ ] **Step 1: Add IslamicAction enum to cli.rs**

Add to `cli.rs`:

```rust
/// Manage Islamic knowledge database
Islamic {
    #[command(subcommand)]
    action: IslamicAction,
},
```

```rust
#[derive(Subcommand, Debug)]
pub enum IslamicAction {
    /// Sync Islamic data from remote APIs
    Sync {
        /// Dataset to sync: quran, hadith, tafsir, fiqh, or all
        #[arg(default_value = "all")]
        dataset: String,
    },
    /// Show sync status and record counts
    Status,
}
```

- [ ] **Step 2: Add amanclaw-islamic-db dependency to CLI**

In `rust/crates/amanclaw-cli/Cargo.toml`:
```toml
amanclaw-islamic-db = { path = "../amanclaw-islamic-db" }
```

- [ ] **Step 3: Implement cmd_islamic() in main.rs**

```rust
async fn cmd_islamic(config_path: &str, action: cli::IslamicAction) -> Result<()> {
    match action {
        cli::IslamicAction::Status => {
            let db_path = std::env::var("ISLAMIC_DB_PATH")
                .unwrap_or_else(|_| "data/islamic.db".into());
            let db = amanclaw_islamic_db::IslamicDb::new(&db_path).await?;
            let statuses = amanclaw_islamic_db::sync::get_all_status(db.pool()).await?;

            if statuses.is_empty() {
                println!("Islamic database is empty. Run 'amanclaw islamic sync' to populate.");
                return Ok(());
            }

            println!("Islamic Knowledge Database:\n");
            println!("{:<25} {:<25} {:>10}", "Dataset", "Last Synced", "Records");
            println!("{}", "-".repeat(62));
            for s in &statuses {
                let synced = if s.last_synced.is_empty() { "never".to_string() } else {
                    s.last_synced.chars().take(19).collect()
                };
                println!("{:<25} {:<25} {:>10}", s.dataset, synced, s.record_count);
            }
            Ok(())
        }
        cli::IslamicAction::Sync { dataset } => {
            let db_path = std::env::var("ISLAMIC_DB_PATH")
                .unwrap_or_else(|_| "data/islamic.db".into());

            // Ensure data directory exists
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let db = amanclaw_islamic_db::IslamicDb::new(&db_path).await?;
            let api_key = std::env::var("SUNNAH_API_KEY").ok();

            match dataset.as_str() {
                "all" => {
                    println!("Syncing all Islamic data (this may take several minutes)...\n");
                    amanclaw_islamic_db::sync::sync_all(db.pool(), api_key.as_deref()).await?;
                    // Also load fiqh seed
                    amanclaw_islamic_db::seed::load_fiqh_seed(db.pool()).await?;
                    println!("\nSync complete!");
                }
                "quran" => {
                    println!("Syncing Quran...");
                    let count = amanclaw_islamic_db::sync::sync_quran(db.pool()).await?;
                    println!("Synced {count} ayat.");
                }
                "hadith" => {
                    println!("Syncing all hadith collections...");
                    let count = amanclaw_islamic_db::sync::sync_all_hadith(db.pool(), api_key.as_deref()).await?;
                    println!("Synced {count} hadith.");
                }
                "tafsir" => {
                    println!("Syncing tafsir...");
                    amanclaw_islamic_db::sync::sync_tafsir(db.pool(), "ibn_kathir", 169).await?;
                    amanclaw_islamic_db::sync::sync_tafsir(db.pool(), "jalalayn", 74).await?;
                    println!("Tafsir sync complete.");
                }
                "fiqh" => {
                    println!("Loading fiqh seed data...");
                    let count = amanclaw_islamic_db::seed::load_fiqh_seed(db.pool()).await?;
                    println!("Loaded {count} fiqh rulings.");
                }
                other => {
                    anyhow::bail!("Unknown dataset: {other}. Use: all, quran, hadith, tafsir, fiqh");
                }
            }
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Wire up in main match**

```rust
Some(Command::Islamic { action }) => cmd_islamic(&cli.config, action).await,
```

- [ ] **Step 5: Add clap tests**

```rust
#[test]
fn test_cli_islamic_sync_all() {
    let cli = Cli::parse_from(["amanclaw", "islamic", "sync"]);
    match cli.command {
        Some(Command::Islamic { action: IslamicAction::Sync { dataset } }) => {
            assert_eq!(dataset, "all");
        }
        _ => panic!("expected Islamic Sync"),
    }
}

#[test]
fn test_cli_islamic_sync_quran() {
    let cli = Cli::parse_from(["amanclaw", "islamic", "sync", "quran"]);
    match cli.command {
        Some(Command::Islamic { action: IslamicAction::Sync { dataset } }) => {
            assert_eq!(dataset, "quran");
        }
        _ => panic!("expected Islamic Sync quran"),
    }
}

#[test]
fn test_cli_islamic_status() {
    let cli = Cli::parse_from(["amanclaw", "islamic", "status"]);
    assert!(matches!(
        cli.command,
        Some(Command::Islamic { action: IslamicAction::Status })
    ));
}
```

- [ ] **Step 6: Verify compilation**

Run: `cd rust && cargo check --package amanclaw-cli`
Expected: Compiles

- [ ] **Step 7: Run CLI tests**

Run: `cd rust && cargo test --package amanclaw-cli cli::tests -- --nocapture`
Expected: All tests PASS

- [ ] **Step 8: Commit**

```bash
git add rust/crates/amanclaw-cli/src/cli.rs rust/crates/amanclaw-cli/src/main.rs rust/crates/amanclaw-cli/Cargo.toml
git commit -m "feat(cli): add 'amanclaw islamic sync/status' commands"
```

---

## Summary

| Task | Description | Steps |
|------|-------------|-------|
| 1 | Crate skeleton + lib.rs | 6 |
| 2 | Database schema + tests | 4 |
| 3 | Quran query module + tests | 3 |
| 4 | Hadith query module + tests | 3 |
| 5 | Fiqh query module + tests | 3 |
| 6 | Sync metadata helpers + tests | 3 |
| 7 | Quran sync from API | 2 |
| 8 | Hadith sync from API | 2 |
| 9 | Tafsir sync + master sync | 2 |
| 10 | Fiqh seed data + loader | 4 |
| 11 | CLI commands (sync/status) | 8 |

**Total: 11 tasks, 40 steps**

After completing this plan:
```bash
amanclaw islamic sync              # Download all Islamic data
amanclaw islamic sync quran        # Sync Quran + translations
amanclaw islamic sync hadith       # Sync 6 hadith collections
amanclaw islamic sync tafsir       # Sync Ibn Kathir + Al-Jalalayn
amanclaw islamic sync fiqh         # Load curated fiqh rulings
amanclaw islamic status            # Show dataset counts + last synced
```
