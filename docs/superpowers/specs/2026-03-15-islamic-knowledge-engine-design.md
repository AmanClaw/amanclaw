# Phase 2A: Islamic Knowledge Engine — Design Spec

**Date:** 2026-03-15
**Scope:** Quran engine with tafsir, Hadith engine with isnad grading, Fiqh resolver with multi-madhab support, local SQLite knowledge store with user-triggered sync
**Approach:** Local database (SQLite) with API-based import/sync. Offline-first, sovereign.
**Part of:** Phase 2 (Islamic Sovereign Core) from the [sovereign AI agent design](2026-03-14-sovereign-islamic-ai-agent-design.md)

---

## Overview

A new crate `amanclaw-islamic-db` provides a local SQLite knowledge store containing the full Quran (with tafsir), 6 major hadith collections (with grading), and curated fiqh rulings (multi-madhab). Three Rust skills — `QuranSkill`, `HadithSkill`, `FiqhSkill` — query this store for instant, offline Islamic knowledge. Data is populated via API import and kept up to date with user-triggered sync (CLI command, dashboard button, or API call).

---

## Data Layer

### New Crate: `amanclaw-islamic-db`

```
rust/crates/amanclaw-islamic-db/
├── src/
│   ├── lib.rs          — IslamicDb struct (owns SQLite pool)
│   ├── quran.rs        — Quran queries (verse, search, tafsir, thematic)
│   ├── hadith.rs       — Hadith queries (search, lookup, browse, related)
│   ├── fiqh.rs         — Fiqh queries (ask, browse, topics)
│   ├── sync.rs         — Data import/sync from APIs
│   ├── schema.rs       — Table creation + migrations
│   └── seed.rs         — Initial seed data loading
├── data/
│   └── fiqh_seed.json  — Curated fiqh rulings seed file
└── Cargo.toml
```

### Database Schema

```sql
-- Quran: 6,236 ayat
quran_ayat (
  surah INT, ayat INT, text_uthmani TEXT, text_simple TEXT,
  translation_ms TEXT, translation_en TEXT,
  juz INT, hizb INT, page INT,
  PRIMARY KEY (surah, ayat)
)

-- Tafsir: multiple tafsir per ayat
quran_tafsir (
  id INTEGER PRIMARY KEY,
  surah INT, ayat INT,
  tafsir_name TEXT,  -- 'ibn_kathir', 'jalalayn'
  language TEXT,     -- 'ar', 'en', 'ms'
  text TEXT,
  UNIQUE(surah, ayat, tafsir_name, language)
)

-- Hadith: ~40,000 across 6 collections
hadith (
  id INTEGER PRIMARY KEY,
  collection TEXT,  -- 'bukhari', 'muslim', 'abudawud', 'tirmidhi', 'nasai', 'ibnmajah'
  book_number INT, hadith_number INT,
  text_ar TEXT, text_en TEXT,
  grade TEXT,       -- 'sahih', 'hasan', 'daif', 'mawdu'
  graded_by TEXT,   -- 'al-albani', 'darussalam', etc.
  chapter TEXT,
  UNIQUE(collection, hadith_number)
)

-- Fiqh: scholarly rulings by topic + madhab
fiqh_rulings (
  id INTEGER PRIMARY KEY,
  topic TEXT,        -- 'prayer', 'fasting', 'zakat', 'marriage', etc.
  subtopic TEXT,
  madhab TEXT,       -- 'shafii', 'hanafi', 'maliki', 'hanbali', 'consensus'
  ruling TEXT,
  evidence TEXT,     -- Quran/Hadith references
  source TEXT,       -- book/scholar citation
  language TEXT
)

-- FTS5 indexes for full-text search
quran_fts (surah, ayat, text, translation_ms, translation_en)
hadith_fts (collection, hadith_number, text_ar, text_en, chapter)
fiqh_fts (topic, subtopic, ruling, evidence)

-- Sync tracking
sync_metadata (
  dataset TEXT PRIMARY KEY,  -- 'quran', 'hadith_bukhari', 'tafsir_ibn_kathir', etc.
  last_synced TEXT,          -- ISO8601 timestamp
  version TEXT,
  record_count INT
)
```

### Data Sources

| Dataset | Source | API | Records |
|---------|--------|-----|---------|
| Quran text + translations | Quran.com API v4 | `api.quran.com/api/v4` | 6,236 ayat |
| Tafsir Ibn Kathir | Quran.com API (resource 169) | Same | ~6,236 |
| Tafsir Al-Jalalayn | Quran.com API (resource 74) | Same | ~6,236 |
| Hadith Bukhari | Sunnah.com API | `api.sunnah.com/v1` | ~7,563 |
| Hadith Muslim | Sunnah.com API | Same | ~7,453 |
| Hadith Abu Dawud | Sunnah.com API | Same | ~5,274 |
| Hadith Tirmidhi | Sunnah.com API | Same | ~3,956 |
| Hadith Nasai | Sunnah.com API | Same | ~5,761 |
| Hadith Ibn Majah | Sunnah.com API | Same | ~4,341 |
| Fiqh rulings | Curated JSON seed file | Local | ~500 initial |

### Sync Mechanism

- **CLI:** `amanclaw islamic sync [dataset]`
- **API:** `POST /api/islamic/sync` with optional `{ "dataset": "quran" }` body
- **Dashboard/Desktop:** "Update Islamic Data" button in Settings page
- **Progress:** Emits WebSocket events `islamic.sync.progress { dataset, progress, total }`
- **First run:** Auto-detects empty database on engine start, prints instructions
- **Incremental:** Uses `sync_metadata.last_synced` to avoid re-downloading unchanged data

---

## Quran Engine

Replaces the existing API-dependent `QuranSkill` with a local database-backed engine.

### Capabilities

| Action | Description |
|--------|-------------|
| `verse` | Lookup by surah:ayat — instant, offline |
| `search` | FTS5 full-text search across translations |
| `tafsir` | Retrieve tafsir (Ibn Kathir or Al-Jalalayn) for a verse |
| `thematic` | Semantic search: "What does the Quran say about patience?" |
| `related` | Find related ayat by topic/theme (vector similarity) |
| `surah_list` | List all 114 surahs (unchanged from Phase 1) |

### Parameters

```json
{
  "action": "verse|search|tafsir|thematic|related|surah_list",
  "surah": 2, "ayat": 255,
  "query": "patience in hardship",
  "tafsir": "ibn_kathir|jalalayn",
  "language": "en|ms|ar",
  "limit": 5
}
```

### Thematic Search Flow

```
User: "What does the Quran say about patience?"
  → FTS5 search "patience" in translations → top matches
  → Vector similarity search on embeddings → semantic matches
  → Reciprocal Rank Fusion (existing RRF logic from amanclaw-memory) → merged results
  → Return top 5 ayat with translations + tafsir excerpts
```

### Migration from Phase 1

- Existing `QuranSkill` refactored to accept `IslamicDb` in constructor
- API fallback retained if database is empty (graceful degradation)
- Static surah list unchanged
- All existing parameters remain backward-compatible

---

## Hadith Engine

New Rust skill replacing the Python `skill_hadith.py` with full local database support.

### Capabilities

| Action | Description |
|--------|-------------|
| `search` | FTS5 + semantic search across all 6 collections |
| `lookup` | Fetch specific hadith by collection + number |
| `browse` | Browse by book/chapter structure |
| `related` | Find similar hadith by topic (vector similarity) |

### Parameters

```json
{
  "action": "search|lookup|browse|related",
  "query": "prayer at night",
  "collection": "bukhari|muslim|abudawud|tirmidhi|nasai|ibnmajah|all",
  "grade": "sahih|hasan|daif|all",
  "book": 2,
  "hadith_number": 1234,
  "limit": 5
}
```

### Grade Filtering

Users can filter by authenticity grade:
- `sahih` — only authenticated hadith
- `hasan` — good/fair hadith
- `daif` — weak hadith (shown with warning)
- `all` — show everything with grade labels

### Output Format

```
📖 Sahih al-Bukhari #1234
Grade: Sahih (Al-Albani)
Chapter: Book of Prayer

[Arabic text]

[English translation]

Related: Muslim #567, Tirmidhi #890
```

### Key Principle

AmanClaw does **not** independently grade hadith. It displays existing scholarly classifications from Sunnah.com metadata. The `grade` and `graded_by` fields reflect established scholarly work (Al-Albani, Darussalam, etc.).

### Python Backward Compatibility

The existing `plugins/skill_hadith.py` remains as a backward-compatible wrapper. If the Rust hadith skill is registered, the Python one is not loaded (name collision prevention in registry). Users who haven't synced the database fall back to the Python API-based version.

---

## Fiqh Resolver

New Rust skill for Islamic jurisprudence with multi-madhab support and source citations.

### How It Works

Three-source synthesis:

1. **Local fiqh_rulings table** — curated rulings by topic + madhab
2. **RAG retrieval** — searches Quran + Hadith database for relevant evidence
3. **LLM reasoning** — synthesizes retrieved sources into a structured answer

```
User: "Is it permissible to combine prayers when traveling?"

  → Step 1: FTS5 search fiqh_rulings for "combine prayers traveling"
  → Step 2: Retrieve relevant Quran ayat + Hadith on travel prayer
  → Step 3: LLM receives: rulings + evidence + user question
  → Step 4: Output structured multi-madhab response with citations
```

### Parameters

```json
{
  "action": "ask|browse|topics",
  "question": "Is music permissible in Islam?",
  "madhab": "shafii|hanafi|maliki|hanbali|all",
  "topic": "prayer|fasting|zakat|marriage|food|business"
}
```

### Output Format

```
📚 Combining Prayers During Travel (Jam' al-Salatayn)

🔹 Shafi'i: Permissible during travel of ~82km+. Can combine
   Zuhr+Asr or Maghrib+Isha. [Source: Al-Umm]

🔹 Hanafi: Only permitted at Arafah and Muzdalifah during Hajj.
   [Source: Al-Hidayah]

🔹 Maliki: Permissible during travel. Also allowed for rain
   and illness. [Source: Al-Mudawwanah]

🔹 Hanbali: Permissible during travel, illness, rain, and
   genuine hardship. [Source: Al-Mughni]

📖 Evidence:
- Quran 4:101 — "When you travel... no blame to shorten prayer"
- Sahih Muslim #705 — Ibn Abbas: "The Prophet combined prayers
  in Madinah without fear or rain"

⚠️ This is a summary of scholarly positions. For personal rulings,
consult a qualified scholar.
```

### Design Rules

- **Always multi-madhab** — Never present a single opinion on khilafiyyah (disputed) matters
- **Always cite sources** — Every ruling links to scholarly book + Quran/Hadith evidence
- **Never issue fatwas** — Disclaimer always present
- **User's preferred madhab** — If user has set preference (via `/remember madhab shafii`), lead with that madhab but show others
- **Consensus** — If all four madhab agree (ijma'), state clearly as consensus

### Seed Data

Initial `fiqh_seed.json` covers common topics (~500 entries):
- Prayer (salah): combining, shortening, witr, tahajjud
- Fasting (sawm): breaking fast, make-up days, exemptions
- Zakat: nisab, recipients, timing
- Food & drink: halal/haram, doubtful matters
- Business: riba, contracts, insurance
- Marriage & family: nikah, mahr, divorce
- Dress & modesty: awrah, hijab

Expandable via `amanclaw islamic sync fiqh` or community contributions.

---

## Engine Integration

### Skills Registration

In `Engine::start()`:

```rust
// Initialize Islamic knowledge database (separate from conversation memory)
let islamic_db_path = std::env::var("ISLAMIC_DB_PATH")
    .unwrap_or_else(|_| "data/islamic.db".into());
let islamic_db = IslamicDb::new(&islamic_db_path).await?;

// Register Islamic skills with shared db reference
let quran_skill = QuranSkill::new(islamic_db.clone());
let hadith_skill = HadithSkill::new(islamic_db.clone());
let fiqh_skill = FiqhSkill::new(islamic_db.clone());
```

`IslamicDb` is `Arc`-wrapped, sharing one connection pool across all three skills.

### CLI Commands

```bash
amanclaw islamic sync              # Sync all datasets
amanclaw islamic sync quran        # Sync Quran + tafsir only
amanclaw islamic sync hadith       # Sync all 6 hadith collections
amanclaw islamic sync fiqh         # Sync fiqh rulings
amanclaw islamic status            # Show sync status + record counts
```

### API Endpoints

```
GET  /api/islamic/status           — sync status for all datasets
POST /api/islamic/sync             — trigger sync (body: { "dataset": "quran" })
GET  /api/islamic/sync/progress    — SSE stream of sync progress events
```

### Dashboard/Desktop

New "Islamic Data" section in Settings page:
- Table: dataset name, last synced, record count, status badge
- "Sync" button per dataset + "Sync All" button
- Progress bar during sync (WebSocket events `islamic.sync.progress`)

### User Madhab Preference

Stored in existing user facts system (`/remember madhab shafii`). The Fiqh skill reads this from the knowledge store to prioritize the user's preferred school of thought.

---

## What This Delivers

When complete, users can:

```
"What does the Quran say about patience?"
→ 5 relevant ayat with translations + tafsir excerpts (offline, instant)

"Find sahih hadith about night prayer"
→ Filtered results from Bukhari + Muslim with grades and sources

"Is it permissible to pray in a moving vehicle?"
→ Multi-madhab answer with Quran/Hadith citations + disclaimer

amanclaw islamic sync
→ Downloads full Quran + 6 hadith collections + tafsir to local SQLite

Dashboard → Settings → Islamic Data → [Sync All]
→ One-click update with progress bar
```

All queries work offline after initial sync. No API dependency at runtime.
