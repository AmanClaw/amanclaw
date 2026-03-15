# Phase 2A: Islamic Knowledge Engine — Overview

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local SQLite-backed Islamic knowledge engine with Quran tafsir, Hadith isnad grading, and Fiqh multi-madhab resolver — all working offline after initial sync.

**Architecture:** New `amanclaw-islamic-db` crate owns the SQLite knowledge store. Three Rust skills query it. Data is populated via API sync and kept up to date with user-triggered sync (CLI, dashboard, API).

**Tech Stack:** Rust, sqlx 0.8 (SQLite), reqwest (API sync), FTS5 (search), Axum (API endpoints)

---

## Sub-Plans

Execute in this order (dependencies noted):

### Plan 2A-1: Islamic DB Crate + Schema + Sync (no dependencies)
**File:** `2026-03-15-phase2a1-islamic-db-crate.md`
**Scope:** New crate with SQLite schema, data import from Quran.com + Sunnah.com APIs, CLI `amanclaw islamic sync/status` commands
**Impact:** Foundation — everything else depends on this

### Plan 2A-2: Quran Engine (depends on 2A-1)
**File:** `2026-03-15-phase2a2-quran-engine.md`
**Scope:** Refactor QuranSkill to query IslamicDb, add tafsir + thematic search
**Impact:** Offline Quran with tafsir — core differentiator

### Plan 2A-3: Hadith Engine (depends on 2A-1)
**File:** `2026-03-15-phase2a3-hadith-engine.md`
**Scope:** New Rust HadithSkill with grade filtering, cross-collection search
**Impact:** Full hadith corpus with isnad grading — offline

### Plan 2A-4: Fiqh Resolver (depends on 2A-1, 2A-2, 2A-3)
**File:** `2026-03-15-phase2a4-fiqh-resolver.md`
**Scope:** New FiqhSkill with RAG retrieval from Quran + Hadith, multi-madhab output
**Impact:** Islamic jurisprudence with scholarly citations

### Plan 2A-5: API + Dashboard (depends on 2A-1)
**File:** `2026-03-15-phase2a5-api-dashboard.md`
**Scope:** REST endpoints for sync status/trigger, dashboard sync UI
**Impact:** User-facing sync button — "click to update"
