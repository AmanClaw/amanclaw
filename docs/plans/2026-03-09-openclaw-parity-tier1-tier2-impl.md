# OpenClaw Parity (Tier 1 + Tier 2) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 7 features to reach OpenClaw parity — FTS5 hybrid search, SOUL.md agent files, cron scheduler, webhook triggers, WebSocket gateway, sub-agent spawning, and skill marketplace.

**Architecture:** Bottom-up build order where each feature builds on the previous. All features integrate into the existing trait-based architecture. Two new workspace crates (`amanclaw-gateway`, `amanclaw-registry`). The engine's `run()` loop gains `tokio::select!` to multiplex chat messages, cron events, and webhook events.

**Tech Stack:** Rust, Tokio, Axum (WebSocket), SQLite FTS5, `cron` crate, `chrono-tz`, Handlebars templates, HMAC-SHA256, JSON-RPC 2.0, `semver`, `tar`/`flate2`.

**Design Doc:** `docs/plans/2026-03-09-openclaw-parity-tier1-tier2-design.md`

---

## Task 1: FTS5 Hybrid Search — Schema

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/schema.rs`

**Step 1: Add FTS5 virtual table and sync triggers to INIT_SQL**

In `rust/crates/amanclaw-memory/src/schema.rs`, append to the end of the `INIT_SQL` string (before the closing `"#;`):

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS vector_documents_fts
USING fts5(content, tokenize='unicode61 remove_diacritics 2', content='vector_documents', content_rowid='rowid');

CREATE TRIGGER IF NOT EXISTS vector_documents_ai AFTER INSERT ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS vector_documents_ad AFTER DELETE ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(vector_documents_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS vector_documents_au AFTER UPDATE ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(vector_documents_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO vector_documents_fts(rowid, content) VALUES (new.rowid, new.content);
END;
```

**Step 2: Run existing tests to verify schema is valid**

Run: `cd rust && cargo test -p amanclaw-memory -- --nocapture`
Expected: All existing tests pass (FTS5 virtual table is additive).

**Step 3: Commit**

```bash
git add rust/crates/amanclaw-memory/src/schema.rs
git commit -m "feat(memory): add FTS5 virtual table with sync triggers for hybrid search"
```

---

## Task 2: FTS5 Hybrid Search — Vector Store Implementation

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/vector.rs`
- Test: `rust/crates/amanclaw-memory/src/vector.rs` (inline tests)

**Step 1: Write failing test for FTS5 text search**

Add to the `#[cfg(test)] mod tests` block in `rust/crates/amanclaw-memory/src/vector.rs`:

```rust
#[tokio::test]
async fn test_fts5_text_search_with_bm25() {
    let pool = make_pool().await;
    let store = SqliteVectorStore::new(pool);

    let docs = vec![
        Document { id: "q1".into(), content: "Bismillah ar-Rahman ar-Rahim".into(), metadata: HashMap::new() },
        Document { id: "q2".into(), content: "Alhamdulillah Rabbil Alamin".into(), metadata: HashMap::new() },
        Document { id: "q3".into(), content: "The most merciful and compassionate".into(), metadata: HashMap::new() },
    ];
    store.upsert("quran", &docs).await.unwrap();

    // FTS5 MATCH should find "Rahman" but not random words
    let results = store.search("quran", "Rahman", 5).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "q1");
    assert!(results[0].score < 0.0); // BM25 returns negative scores (lower = better match)
}
```

**Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-memory test_fts5_text_search_with_bm25 -- --nocapture`
Expected: FAIL — current `search()` uses LIKE which doesn't produce BM25 scores.

**Step 3: Replace LIKE search with FTS5 BM25 in `search()`**

In `rust/crates/amanclaw-memory/src/vector.rs`, replace the `search` method in the `impl VectorStore for SqliteVectorStore` block (the one using `LIKE`):

```rust
async fn search(&self, collection: &str, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    // Use FTS5 MATCH with BM25 ranking instead of LIKE
    let rows = sqlx::query(
        "SELECT vd.id, vd.content, vd.metadata, bm25(vector_documents_fts) as rank
         FROM vector_documents vd
         JOIN vector_documents_fts fts ON vd.rowid = fts.rowid
         WHERE vd.collection = ? AND vector_documents_fts MATCH ?
         ORDER BY rank
         LIMIT ?"
    )
        .bind(collection)
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&self.pool).await?;

    let results = rows.iter().map(|row| {
        let metadata_str: String = row.get("metadata");
        SearchResult {
            id: row.get("id"),
            content: row.get("content"),
            score: row.get::<f64, _>("rank"),
            metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
        }
    }).collect();

    Ok(results)
}
```

**Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p amanclaw-memory test_fts5_text_search_with_bm25 -- --nocapture`
Expected: PASS

**Step 5: Write failing test for hybrid RRF search**

Add to tests:

```rust
#[tokio::test]
async fn test_hybrid_rrf_search() {
    let pool = make_pool().await;
    let store = SqliteVectorStore::new(pool);

    let docs = vec![
        Document { id: "d1".into(), content: "Prayer times for Kuala Lumpur".into(), metadata: HashMap::new() },
        Document { id: "d2".into(), content: "Fasting rules during Ramadan".into(), metadata: HashMap::new() },
        Document { id: "d3".into(), content: "Solat prayer schedule Malaysia".into(), metadata: HashMap::new() },
    ];
    let embeddings = vec![
        vec![1.0, 0.0, 0.0],  // d1: "prayer" direction
        vec![0.0, 1.0, 0.0],  // d2: "fasting" direction
        vec![0.8, 0.2, 0.0],  // d3: close to d1
    ];
    store.upsert_with_embeddings("test", &docs, &embeddings).await.unwrap();

    // Query embedding close to d1, text query "prayer" matches d1 and d3
    // RRF should rank d1 highest (top in both vector AND FTS)
    let results = store.search_by_embedding("test", &[0.9, 0.1, 0.0], "prayer", 3).await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "d1"); // Best in both ranking lists
}
```

**Step 6: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-memory test_hybrid_rrf_search -- --nocapture`
Expected: FAIL — current `search_by_embedding` doesn't use FTS5.

**Step 7: Add `hybrid_rrf` function and update `search_by_embedding`**

Add the `hybrid_rrf` helper function in `rust/crates/amanclaw-memory/src/vector.rs` (above the `impl VectorStore` block):

```rust
/// Reciprocal Rank Fusion: combines two ranked lists without score normalization.
/// k=60 is the standard constant from Cormack et al. 2009.
fn hybrid_rrf(
    vector_ranked: &[(String, f64, String, String)], // (id, score, content, metadata)
    fts_ranked: &[(String, f64, String, String)],
    k: f64,
) -> Vec<(String, f64, String, String)> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut data: HashMap<String, (String, String)> = HashMap::new();

    for (rank, (id, _, content, metadata)) in vector_ranked.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        data.entry(id.clone()).or_insert_with(|| (content.clone(), metadata.clone()));
    }
    for (rank, (id, _, content, metadata)) in fts_ranked.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        data.entry(id.clone()).or_insert_with(|| (content.clone(), metadata.clone()));
    }

    let mut merged: Vec<_> = scores.into_iter()
        .map(|(id, score)| {
            let (content, metadata) = data.remove(&id).unwrap_or_default();
            (id, score, content, metadata)
        })
        .collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged
}
```

Then update `search_by_embedding` in the `impl VectorStore for SqliteVectorStore` block to use hybrid RRF:

```rust
async fn search_by_embedding(
    &self, collection: &str, query_embedding: &[f32], query_text: &str, limit: usize,
) -> Result<Vec<SearchResult>> {
    // 1. Vector search (cosine similarity) — existing logic
    let vector_results = SqliteVectorStore::search_by_embedding(self, collection, query_embedding, limit * 2).await?;
    let vector_ranked: Vec<_> = vector_results.iter()
        .map(|r| (r.id.clone(), r.score, r.content.clone(), serde_json::to_string(&r.metadata).unwrap_or_default()))
        .collect();

    // 2. FTS5 BM25 search
    let fts_rows = sqlx::query(
        "SELECT vd.id, vd.content, vd.metadata, bm25(vector_documents_fts) as rank
         FROM vector_documents vd
         JOIN vector_documents_fts fts ON vd.rowid = fts.rowid
         WHERE vd.collection = ? AND vector_documents_fts MATCH ?
         ORDER BY rank
         LIMIT ?"
    )
        .bind(collection)
        .bind(query_text)
        .bind((limit * 2) as i64)
        .fetch_all(&self.pool).await
        .unwrap_or_default(); // FTS match may fail on invalid syntax — degrade gracefully

    let fts_ranked: Vec<_> = fts_rows.iter()
        .map(|row| {
            let id: String = row.get("id");
            let content: String = row.get("content");
            let metadata: String = row.get("metadata");
            let rank: f64 = row.get("rank");
            (id, rank, content, metadata)
        })
        .collect();

    // 3. Merge with RRF (k=60)
    if fts_ranked.is_empty() {
        // No FTS matches — fall back to vector-only results
        let mut results = vector_results;
        results.truncate(limit);
        return Ok(results);
    }

    let merged = hybrid_rrf(&vector_ranked, &fts_ranked, 60.0);

    let results: Vec<SearchResult> = merged.into_iter()
        .take(limit)
        .map(|(id, score, content, metadata_str)| {
            let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            SearchResult { id, content, score, metadata }
        })
        .collect();

    Ok(results)
}
```

**Step 8: Run all vector tests**

Run: `cd rust && cargo test -p amanclaw-memory -- --nocapture`
Expected: All tests pass including the two new ones.

**Step 9: Commit**

```bash
git add rust/crates/amanclaw-memory/src/vector.rs
git commit -m "feat(memory): implement FTS5 hybrid search with BM25 + cosine RRF fusion"
```

---

## Task 3: SOUL.md — Add soul_file to AgentProfile

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/agent.rs`
- Modify: `rust/crates/amanclaw-traits/src/config.rs`
- Test: inline in `agent.rs`

**Step 1: Write failing test for soul_file deserialization**

Add to `rust/crates/amanclaw-traits/src/agent.rs` tests:

```rust
#[test]
fn test_agent_profile_with_soul_file() {
    let yaml = r#"
id: ustazbot
name: UstazBot
system_prompt: ""
soul_file: "ustazbot.md"
allowed_skills:
  - solat
memory_namespace: ustaz
"#;
    let profile: AgentProfile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(profile.soul_file.as_deref(), Some("ustazbot.md"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-traits test_agent_profile_with_soul_file -- --nocapture`
Expected: FAIL — `soul_file` field doesn't exist.

**Step 3: Add `soul_file` to AgentProfile**

In `rust/crates/amanclaw-traits/src/agent.rs`, add to `AgentProfile`:

```rust
/// Path to a SOUL.md file (relative to soul_dir). Overrides system_prompt if set.
#[serde(default)]
pub soul_file: Option<String>,
```

Add to `AgentProfile::default_agent()`:

```rust
soul_file: None,
```

**Step 4: Add `soul_dir` to SkillsConfig**

In `rust/crates/amanclaw-traits/src/config.rs`, add to `SkillsConfig`:

```rust
/// Directory containing SOUL.md agent personality files.
#[serde(default = "default_soul_dir")]
pub soul_dir: String,
```

Add the default function:

```rust
fn default_soul_dir() -> String { "./souls".into() }
```

**Step 5: Run tests**

Run: `cd rust && cargo test -p amanclaw-traits -- --nocapture`
Expected: All tests pass.

**Step 6: Fix any existing tests that construct AgentProfile or IncomingMessage**

Check `rust/crates/amanclaw-core/src/router.rs` `make_profile` helper and add `soul_file: None` to it. Check any other places constructing `AgentProfile` and add the new field.

Run: `cd rust && cargo test -p amanclaw-core -- --nocapture`
Expected: All pass.

**Step 7: Commit**

```bash
git add rust/crates/amanclaw-traits/src/agent.rs rust/crates/amanclaw-traits/src/config.rs rust/crates/amanclaw-core/src/router.rs
git commit -m "feat(traits): add soul_file to AgentProfile and soul_dir to config"
```

---

## Task 4: SOUL.md — SoulLoader Implementation

**Files:**
- Create: `rust/crates/amanclaw-core/src/soul.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs` (add `pub mod soul;`)

**Step 1: Write failing test for basic soul loading**

Create `rust/crates/amanclaw-core/src/soul.rs`:

```rust
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub struct ResolvedSoul {
    pub prompt: String,
    pub variables: HashMap<String, String>,
    pub tags: Vec<String>,
}

pub struct SoulLoader;

impl SoulLoader {
    pub fn load(_soul_dir: &Path, _filename: &str) -> Result<ResolvedSoul> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_simple_soul() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.md"), "# TestBot\n\nYou are a test bot.").unwrap();

        let soul = SoulLoader::load(dir.path(), "test.md").unwrap();
        assert!(soul.prompt.contains("You are a test bot"));
    }

    #[test]
    fn test_load_soul_with_frontmatter() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.md"), r#"---
version: 1
tags: [islamic, test]
variables:
  region: malaysia
---
# TestBot

Expert for {{region}}.
"#).unwrap();

        let soul = SoulLoader::load(dir.path(), "test.md").unwrap();
        assert!(soul.prompt.contains("Expert for malaysia"));
        assert_eq!(soul.tags, vec!["islamic", "test"]);
        assert_eq!(soul.variables.get("region").unwrap(), "malaysia");
    }

    #[test]
    fn test_load_soul_with_inheritance() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("base.md"), r#"---
version: 1
variables:
  greeting: Hello
---
# Base

{{greeting}} world.

## Rules
- Be helpful
"#).unwrap();

        fs::write(dir.path().join("child.md"), r#"---
version: 1
extends: base.md
variables:
  greeting: Assalamualaikum
---
# Child

Islamic expert.

## Rules
- Follow Islamic guidelines
"#).unwrap();

        let soul = SoulLoader::load(dir.path(), "child.md").unwrap();
        // Child overrides greeting variable
        assert!(soul.prompt.contains("Assalamualaikum"));
        // Child's "Rules" section overrides base's "Rules" section
        assert!(soul.prompt.contains("Follow Islamic guidelines"));
        assert!(!soul.prompt.contains("Be helpful"));
    }

    #[test]
    fn test_max_inheritance_depth() {
        let dir = TempDir::new().unwrap();
        for i in 0..6 {
            let extends = if i > 0 { format!("extends: level{}.md", i - 1) } else { String::new() };
            let content = format!("---\n{}\n---\n# Level {}", extends, i);
            fs::write(dir.path().join(format!("level{}.md", i)), content).unwrap();
        }
        let result = SoulLoader::load(dir.path(), "level5.md");
        assert!(result.is_err()); // Max depth 5 exceeded
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-core test_load_simple_soul -- --nocapture`
Expected: FAIL — `todo!()` panics.

**Step 3: Implement SoulLoader**

Replace the `todo!()` in `SoulLoader` with the full implementation:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
struct SoulFrontmatter {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    variables: HashMap<String, String>,
}

impl SoulLoader {
    pub fn load(soul_dir: &Path, filename: &str) -> Result<ResolvedSoul> {
        let mut chain = Vec::new();
        let mut current = Some(filename.to_string());

        while let Some(ref name) = current {
            if chain.len() >= 5 {
                anyhow::bail!("Soul inheritance depth exceeds maximum of 5: {:?}",
                    chain.iter().map(|(_, _, n): &(SoulFrontmatter, String, String)| n.clone()).collect::<Vec<_>>());
            }
            let path = soul_dir.join(name);
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("Failed to read soul file '{}': {}", path.display(), e))?;
            let (frontmatter, body) = Self::parse_frontmatter(&raw)?;
            current = frontmatter.extends.clone();
            chain.push((frontmatter, body, name.clone()));
        }

        chain.reverse();
        Self::merge_chain(chain)
    }

    fn parse_frontmatter(raw: &str) -> Result<(SoulFrontmatter, String)> {
        if raw.starts_with("---") {
            let rest = &raw[3..];
            if let Some(end) = rest.find("---") {
                let fm_str = &rest[..end];
                let body = rest[end + 3..].trim().to_string();
                let fm: SoulFrontmatter = serde_yaml::from_str(fm_str.trim())
                    .unwrap_or_default();
                return Ok((fm, body));
            }
        }
        Ok((SoulFrontmatter::default(), raw.to_string()))
    }

    fn parse_sections(body: &str) -> Vec<(String, String)> {
        let mut sections = Vec::new();
        let mut current_heading = "_preamble".to_string();
        let mut current_content = String::new();

        for line in body.lines() {
            if line.starts_with("## ") {
                if !current_content.trim().is_empty() || current_heading == "_preamble" {
                    sections.push((current_heading.clone(), current_content.trim().to_string()));
                }
                current_heading = line[3..].trim().to_string();
                current_content = String::new();
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
        if !current_content.trim().is_empty() || sections.is_empty() {
            sections.push((current_heading, current_content.trim().to_string()));
        }
        sections
    }

    fn merge_chain(chain: Vec<(SoulFrontmatter, String, String)>) -> Result<ResolvedSoul> {
        let mut merged_vars: HashMap<String, String> = HashMap::new();
        let mut merged_sections: Vec<(String, String)> = Vec::new();
        let mut tags = Vec::new();

        for (fm, body, _name) in chain {
            merged_vars.extend(fm.variables);
            tags.extend(fm.tags);

            let sections = Self::parse_sections(&body);
            for (heading, content) in sections {
                if let Some(existing) = merged_sections.iter_mut().find(|(h, _)| h == &heading) {
                    existing.1 = content;
                } else {
                    merged_sections.push((heading, content));
                }
            }
        }

        let mut prompt = String::new();
        for (heading, content) in &merged_sections {
            if heading == "_preamble" {
                prompt.push_str(content);
            } else {
                prompt.push_str(&format!("\n\n## {}\n{}", heading, content));
            }
        }

        for (key, value) in &merged_vars {
            prompt = prompt.replace(&format!("{{{{{}}}}}", key), value);
        }

        Ok(ResolvedSoul {
            prompt: prompt.trim().to_string(),
            variables: merged_vars,
            tags,
        })
    }
}
```

**Step 4: Add `serde_yaml` as a regular dependency to amanclaw-core**

In `rust/crates/amanclaw-core/Cargo.toml`, add under `[dependencies]`:

```toml
serde_yaml = { workspace = true }
```

**Step 5: Add `pub mod soul;` to `rust/crates/amanclaw-core/src/lib.rs`**

Add after the existing module declarations at the top.

**Step 6: Run all soul tests**

Run: `cd rust && cargo test -p amanclaw-core soul -- --nocapture`
Expected: All 4 tests pass.

**Step 7: Commit**

```bash
git add rust/crates/amanclaw-core/src/soul.rs rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-core/Cargo.toml
git commit -m "feat(core): implement SoulLoader with frontmatter, inheritance, and variable interpolation"
```

---

## Task 5: SOUL.md — Engine Integration

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

**Step 1: Add soul loading logic to Engine::new()**

In `rust/crates/amanclaw-core/src/lib.rs`, after building `agent_router` (around line 112) and before `let registry = Arc::new(registry);` (line 114), add:

```rust
// Load SOUL.md files for agents that have them configured
let soul_dir = std::path::Path::new(&config.skills.soul_dir);
for (_id, profile) in config.agents.iter_mut() {
    if let Some(ref filename) = profile.soul_file {
        match crate::soul::SoulLoader::load(soul_dir, filename) {
            Ok(resolved) => {
                profile.system_prompt = resolved.prompt;
                tracing::info!(agent = %profile.id, file = %filename, "Loaded SOUL.md");
            }
            Err(e) => {
                tracing::warn!(agent = %profile.id, error = %e, "Failed to load SOUL.md, using inline prompt");
            }
        }
    }
}
```

Note: `config.agents` needs to be mutable. The `agent_router` is built from `config.agents.clone()` already, so move the soul loading to BEFORE the router build, or re-build the router after soul loading.

Actually, the cleanest approach: do soul loading BEFORE `AgentRouter::new()`. Move the soul loading block to just before line 108.

**Step 2: Create example soul files**

Create `rust/souls/default.md`:

```markdown
---
version: 1
tags: [general]
---

# AmanClaw

You are AmanClaw, a helpful AI assistant for Malaysian Muslim communities.

## Personality
- Friendly and respectful
- Uses mixed BM/EN (Rojak) when appropriate
- Knowledgeable about Malaysian culture and Islamic practices

## Response Format
- Keep answers concise
- Use bullet points for lists
- Provide sources when citing Islamic references
```

**Step 3: Run full build**

Run: `cd rust && cargo build`
Expected: Compiles successfully.

**Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/lib.rs rust/souls/
git commit -m "feat(core): integrate SoulLoader into Engine startup"
```

---

## Task 6: Cron — Add message flags and config types

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/message.rs`
- Modify: `rust/crates/amanclaw-traits/src/config.rs`

**Step 1: Add `is_cron`, `is_webhook`, `is_subagent` flags to IncomingMessage**

In `rust/crates/amanclaw-traits/src/message.rs`, add to `IncomingMessage`:

```rust
/// True if this message was generated by the cron scheduler.
#[serde(default)]
pub is_cron: bool,

/// True if this message was generated by a webhook.
#[serde(default)]
pub is_webhook: bool,

/// True if this message is from a sub-agent.
#[serde(default)]
pub is_subagent: bool,
```

Add `platform` field to `OutgoingMessage`:

```rust
/// Target platform for routing the response.
#[serde(default)]
pub platform: Option<String>,

/// Topic/thread ID for Telegram topics or Discord threads.
#[serde(default)]
pub topic_id: Option<String>,
```

**Step 2: Add CronConfig to AppConfig**

In `rust/crates/amanclaw-traits/src/config.rs`, add the cron config types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronConfig {
    #[serde(default = "default_cron_timezone")]
    pub timezone: String,

    #[serde(default)]
    pub jobs: HashMap<String, CronJobConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobConfig {
    pub name: String,
    pub schedule: String,

    #[serde(default)]
    pub timezone: Option<String>,

    #[serde(rename = "type")]
    pub job_type: String,

    #[serde(default)]
    pub skill: Option<String>,

    #[serde(default)]
    pub input: Option<String>,

    #[serde(default)]
    pub prompt: Option<String>,

    #[serde(default)]
    pub template: Option<String>,

    #[serde(default)]
    pub targets: Vec<CronTargetConfig>,

    #[serde(default)]
    pub agent: Option<String>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTargetConfig {
    pub platform: String,
    pub chat_id: String,
    #[serde(default)]
    pub topic_id: Option<String>,
}

fn default_cron_timezone() -> String { "Asia/Kuala_Lumpur".into() }
```

Add to `AppConfig`:

```rust
#[serde(default)]
pub cron: CronConfig,
```

**Step 3: Fix all test compilation errors from new IncomingMessage fields**

Search for all places constructing `IncomingMessage` and add the three new fields with defaults. Key locations:
- `rust/crates/amanclaw-traits/src/message.rs` tests
- `rust/crates/amanclaw-core/src/router.rs` `make_msg` helper
- `rust/crates/amanclaw-core/tests/integration.rs`

Add to each: `is_cron: false, is_webhook: false, is_subagent: false,`

Fix all places constructing `OutgoingMessage` to add: `platform: None, topic_id: None,`

**Step 4: Run full workspace tests**

Run: `cd rust && cargo test --workspace -- --nocapture 2>&1 | head -100`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-traits/
git commit -m "feat(traits): add cron/webhook/subagent message flags and CronConfig"
```

---

## Task 7: Cron — Scheduler Implementation

**Files:**
- Create: `rust/crates/amanclaw-core/src/scheduler.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Add dependencies**

In `rust/crates/amanclaw-core/Cargo.toml` under `[dependencies]`:

```toml
cron = "0.13"
chrono-tz = "0.10"
```

**Step 2: Write failing test**

Create `rust/crates/amanclaw-core/src/scheduler.rs`:

```rust
use amanclaw_traits::config::{CronJobConfig, CronTargetConfig};
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Events produced by the scheduler (shared with webhooks).
#[derive(Debug)]
pub enum SchedulerEvent {
    /// Direct message ready to send (no LLM processing).
    SendMessage(OutgoingMessage),
    /// Inject into pipeline for agent processing.
    InjectMessage(IncomingMessage),
}

pub struct Scheduler {
    tx: mpsc::Sender<SchedulerEvent>,
    handles: HashMap<String, tokio::task::JoinHandle<()>>,
}

impl Scheduler {
    pub fn new(tx: mpsc::Sender<SchedulerEvent>) -> Self {
        Self { tx, handles: HashMap::new() }
    }

    pub fn start_jobs(&mut self, _jobs: &HashMap<String, CronJobConfig>, _default_tz: &str) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_message_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let target = CronTargetConfig {
            platform: "telegram".into(),
            chat_id: "12345".into(),
            topic_id: None,
        };
        let job = CronJobConfig {
            name: "Test".into(),
            schedule: "* * * * * *".into(), // Every second
            timezone: None,
            job_type: "direct_message".into(),
            skill: None,
            input: None,
            prompt: None,
            template: Some("Hello test".into()),
            targets: vec![target],
            agent: None,
            enabled: true,
        };

        let mut scheduler = Scheduler::new(tx);
        let mut jobs = HashMap::new();
        jobs.insert("test".into(), job);
        scheduler.start_jobs(&jobs, "UTC");

        // Wait up to 3 seconds for an event
        let event = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv()
        ).await;

        assert!(event.is_ok(), "Should receive a scheduler event within 3 seconds");
        match event.unwrap().unwrap() {
            SchedulerEvent::SendMessage(msg) => {
                assert_eq!(msg.text, "Hello test");
                assert_eq!(msg.chat_id, "12345");
            }
            _ => panic!("Expected SendMessage"),
        }
    }

    #[tokio::test]
    async fn test_agent_prompt_event() {
        let (tx, mut rx) = mpsc::channel(16);
        let target = CronTargetConfig {
            platform: "telegram".into(),
            chat_id: "12345".into(),
            topic_id: None,
        };
        let job = CronJobConfig {
            name: "Quran Daily".into(),
            schedule: "* * * * * *".into(),
            timezone: None,
            job_type: "agent_prompt".into(),
            skill: None,
            input: None,
            prompt: Some("Share a verse".into()),
            template: None,
            targets: vec![target],
            agent: Some("ustazbot".into()),
            enabled: true,
        };

        let mut scheduler = Scheduler::new(tx);
        let mut jobs = HashMap::new();
        jobs.insert("quran".into(), job);
        scheduler.start_jobs(&jobs, "UTC");

        let event = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv()
        ).await;

        assert!(event.is_ok());
        match event.unwrap().unwrap() {
            SchedulerEvent::InjectMessage(msg) => {
                assert_eq!(msg.text, "Share a verse");
                assert!(msg.is_cron);
                assert!(msg.user_id.starts_with("cron:"));
            }
            _ => panic!("Expected InjectMessage"),
        }
    }
}
```

**Step 3: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-core test_direct_message_event -- --nocapture`
Expected: FAIL — `todo!()`.

**Step 4: Implement Scheduler**

Replace the `todo!()` in `start_jobs`:

```rust
pub fn start_jobs(&mut self, jobs: &HashMap<String, CronJobConfig>, default_tz: &str) {
    for (id, job) in jobs {
        if !job.enabled {
            tracing::info!(job = %id, "Cron job disabled, skipping");
            continue;
        }

        let schedule = match cron::Schedule::from_str(&job.schedule) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(job = %id, error = %e, "Invalid cron expression");
                continue;
            }
        };

        let tz_str = job.timezone.as_deref().unwrap_or(default_tz);
        let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        let tx = self.tx.clone();
        let job_id = id.clone();
        let job_clone = job.clone();

        let handle = tokio::spawn(async move {
            loop {
                let now = chrono::Utc::now().with_timezone(&tz);
                let next = schedule.upcoming(tz).next();
                if let Some(next_time) = next {
                    let wait = (next_time - now).to_std().unwrap_or(std::time::Duration::from_secs(1));
                    tokio::time::sleep(wait).await;

                    if let Err(e) = Self::fire_job(&job_id, &job_clone, &tx).await {
                        tracing::error!(job = %job_id, error = %e, "Cron job failed");
                    }
                } else {
                    break;
                }
            }
        });

        self.handles.insert(id.clone(), handle);
        tracing::info!(job = %id, schedule = %job.schedule, "Cron job scheduled");
    }
}

async fn fire_job(
    job_id: &str,
    job: &CronJobConfig,
    tx: &mpsc::Sender<SchedulerEvent>,
) -> Result<()> {
    for target in &job.targets {
        match job.job_type.as_str() {
            "direct_message" => {
                let text = job.template.clone().unwrap_or_default();
                tx.send(SchedulerEvent::SendMessage(OutgoingMessage {
                    chat_id: target.chat_id.clone(),
                    text,
                    parse_mode: None,
                    reply_to: None,
                    platform: Some(target.platform.clone()),
                    topic_id: target.topic_id.clone(),
                })).await?;
            }
            "skill_invocation" => {
                let skill = job.skill.clone().unwrap_or_default();
                let input = job.input.clone().unwrap_or_default();
                let synthetic = format!("/{} {}", skill, input);
                tx.send(SchedulerEvent::InjectMessage(IncomingMessage {
                    user_id: format!("cron:{}", job_id),
                    chat_id: target.chat_id.clone(),
                    platform: target.platform.clone(),
                    text: synthetic,
                    username: None,
                    first_name: None,
                    is_group: false,
                    image_data: None,
                    reply_to: None,
                    topic_id: target.topic_id.clone(),
                    channel_context: None,
                    is_cron: true,
                    is_webhook: false,
                    is_subagent: false,
                })).await?;
            }
            "agent_prompt" => {
                let prompt = job.prompt.clone().unwrap_or_default();
                tx.send(SchedulerEvent::InjectMessage(IncomingMessage {
                    user_id: format!("cron:{}", job_id),
                    chat_id: target.chat_id.clone(),
                    platform: target.platform.clone(),
                    text: prompt,
                    username: None,
                    first_name: None,
                    is_group: false,
                    image_data: None,
                    reply_to: None,
                    topic_id: target.topic_id.clone(),
                    channel_context: None,
                    is_cron: true,
                    is_webhook: false,
                    is_subagent: false,
                })).await?;
            }
            other => {
                tracing::warn!(job = %job_id, job_type = %other, "Unknown cron job type");
            }
        }
    }
    tracing::info!(job = %job_id, "Cron job fired");
    Ok(())
}
```

Add required imports at top of the file:

```rust
use std::str::FromStr;
use cron::Schedule;
```

**Step 5: Add `pub mod scheduler;` to lib.rs**

**Step 6: Run tests**

Run: `cd rust && cargo test -p amanclaw-core scheduler -- --nocapture`
Expected: Both scheduler tests pass.

**Step 7: Commit**

```bash
git add rust/crates/amanclaw-core/src/scheduler.rs rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-core/Cargo.toml
git commit -m "feat(core): implement cron scheduler with direct_message, skill_invocation, agent_prompt job types"
```

---

## Task 8: Cron — Engine Integration with tokio::select!

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-core/src/pipeline.rs`

**Step 1: Add cron bypass in pipeline**

In `rust/crates/amanclaw-core/src/pipeline.rs`, at the beginning of `process_full()` (after `let text = msg.text.trim();` on line 80), add:

```rust
// Internal messages (cron, webhook, subagent) skip auth, rate limit, and sanitization
let is_internal = msg.is_cron || msg.is_webhook || msg.is_subagent;
```

Then wrap the auth check (lines 83-123), rate limit (lines 125-133), and sanitize (lines 135-139) in `if !is_internal { ... }`. Change `let (clean_text, was_flagged) = check_injection(...)` to:

```rust
let (clean_text, was_flagged) = if is_internal {
    (msg.text.clone(), false)
} else {
    let (ct, wf) = check_injection(&msg.text);
    (ct.to_string(), wf)
};
```

**Step 2: Modify Engine::run() to use tokio::select!**

In `rust/crates/amanclaw-core/src/lib.rs`, modify `Engine::new()` to create the scheduler and event channel. Add fields to `Engine`:

```rust
pub struct Engine {
    // ... existing fields ...
    sched_rx: mpsc::Receiver<crate::scheduler::SchedulerEvent>,
}
```

In `Engine::new()`, before the `Ok(Self { ... })`:

```rust
// Initialize scheduler
let (sched_tx, sched_rx) = mpsc::channel(64);
let mut scheduler = crate::scheduler::Scheduler::new(sched_tx);
scheduler.start_jobs(&config.cron.jobs, &config.cron.timezone);
```

Add `sched_rx` to the `Ok(Self { ... })`.

Modify `Engine::run()`:

```rust
pub async fn run(mut self) -> Result<()> {
    drop(self.tx);
    tracing::info!("Engine running");

    loop {
        tokio::select! {
            Some(msg) = self.rx.recv() => {
                let platform = msg.platform.clone();
                let profile = self.agent_router.resolve(&msg);
                tracing::debug!(agent = %profile.id, "Routed to agent");
                match self.pipeline.process(msg, &self.registry, &profile).await {
                    Ok(Some(response)) => {
                        self.send_to_channel(&platform, response).await;
                    }
                    Ok(None) => {}
                    Err(e) => tracing::error!(error = %e, "Pipeline error"),
                }
            }
            Some(event) = self.sched_rx.recv() => {
                match event {
                    crate::scheduler::SchedulerEvent::SendMessage(response) => {
                        let platform = response.platform.clone().unwrap_or_default();
                        self.send_to_channel(&platform, response).await;
                    }
                    crate::scheduler::SchedulerEvent::InjectMessage(msg) => {
                        let platform = msg.platform.clone();
                        let profile = self.agent_router.resolve(&msg);
                        match self.pipeline.process(msg, &self.registry, &profile).await {
                            Ok(Some(response)) => {
                                self.send_to_channel(&platform, response).await;
                            }
                            Ok(None) => {}
                            Err(e) => tracing::error!(error = %e, "Cron pipeline error"),
                        }
                    }
                }
            }
            else => break,
        }
    }
    Ok(())
}

async fn send_to_channel(&self, platform: &str, response: OutgoingMessage) {
    tracing::info!(chat_id = %response.chat_id, "Sending response");
    for ch in &self.channels {
        if ch.platform() == platform {
            if let Err(e) = ch.send_message(response.clone()).await {
                tracing::error!(error = %e, "Failed to send response");
            }
            break;
        }
    }
}
```

**Step 3: Run full build and tests**

Run: `cd rust && cargo build && cargo test --workspace 2>&1 | tail -20`
Expected: Compiles and all tests pass.

**Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-core/src/pipeline.rs
git commit -m "feat(core): integrate cron scheduler into Engine with tokio::select! and pipeline bypass"
```

---

## Task 9: Webhooks — Config and Router

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/config.rs`
- Create: `rust/crates/amanclaw-core/src/webhooks.rs`
- Modify: `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Add webhook config types**

In `rust/crates/amanclaw-traits/src/config.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookConfig {
    #[serde(default = "default_webhook_base_path")]
    pub base_path: String,

    #[serde(default)]
    pub default_secret: Option<String>,

    #[serde(default)]
    pub endpoints: HashMap<String, WebhookEndpointConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpointConfig {
    pub name: String,
    pub path: String,

    #[serde(default)]
    pub auth: WebhookAuthConfig,

    #[serde(default)]
    pub transform: WebhookTransformConfig,

    #[serde(default)]
    pub targets: Vec<CronTargetConfig>,

    #[serde(default)]
    pub agent: Option<String>,

    #[serde(default)]
    pub rate_limit: Option<u32>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookAuthConfig {
    #[serde(rename = "type", default = "default_webhook_auth_type")]
    pub auth_type: String,

    #[serde(default)]
    pub secret: Option<String>,

    #[serde(default)]
    pub header: Option<String>,

    #[serde(default)]
    pub token: Option<String>,

    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookTransformConfig {
    #[serde(rename = "type", default = "default_webhook_transform_type")]
    pub transform_type: String,

    #[serde(default)]
    pub template: Option<String>,

    #[serde(default)]
    pub message_path: Option<String>,

    #[serde(default)]
    pub title_path: Option<String>,

    #[serde(default)]
    pub prompt_template: Option<String>,

    #[serde(default)]
    pub skill: Option<String>,

    #[serde(default)]
    pub input_template: Option<String>,
}

fn default_webhook_base_path() -> String { "/hooks".into() }
fn default_webhook_auth_type() -> String { "none".into() }
fn default_webhook_transform_type() -> String { "raw_json".into() }
```

Add to `AppConfig`:

```rust
#[serde(default)]
pub webhooks: WebhookConfig,
```

**Step 2: Add dependencies to amanclaw-core**

In `rust/crates/amanclaw-core/Cargo.toml`:

```toml
handlebars = "6"
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
```

**Step 3: Create webhook router**

Create `rust/crates/amanclaw-core/src/webhooks.rs` with the `WebhookRouter` struct, auth validation, payload transformation, and tests. Follow the design doc — implement `validate_auth` (HMAC-SHA256, Bearer, HeaderMatch), `transform` (RawJson, JsonPath, Template, AgentPrompt), and `handle` method.

This is a large file. Include tests for:
- HMAC-SHA256 signature verification
- Bearer token validation
- JSON path extraction
- Template rendering
- Full webhook handling flow

**Step 4: Add `pub mod webhooks;` to lib.rs**

**Step 5: Run tests**

Run: `cd rust && cargo test -p amanclaw-core webhooks -- --nocapture`
Expected: All webhook tests pass.

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-traits/src/config.rs rust/crates/amanclaw-core/src/webhooks.rs rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-core/Cargo.toml
git commit -m "feat(core): implement WebhookRouter with HMAC auth, transforms, and template rendering"
```

---

## Task 10: Webhooks — API Routes

**Files:**
- Create: `rust/crates/amanclaw-api/src/routes/webhooks.rs`
- Modify: `rust/crates/amanclaw-api/src/routes/mod.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`
- Modify: `rust/crates/amanclaw-api/src/state.rs`
- Modify: `rust/crates/amanclaw-api/Cargo.toml`

**Step 1: Extend ApiState with webhook router**

In `rust/crates/amanclaw-api/src/state.rs`, add:

```rust
pub webhook_router: Option<Arc<amanclaw_core::webhooks::WebhookRouter>>,
```

**Step 2: Create webhook routes**

Create `rust/crates/amanclaw-api/src/routes/webhooks.rs` with:
- `receive_webhook` — POST handler at `/hooks/{webhook_id}`, no auth middleware
- `list_webhooks` — GET handler at `/api/webhooks`, requires auth

**Step 3: Mount routes in lib.rs**

Add the webhook receiver route OUTSIDE the authed router (no auth middleware). Add the management routes INSIDE the authed router.

**Step 4: Add `pub mod webhooks;` to routes/mod.rs**

**Step 5: Run build**

Run: `cd rust && cargo build -p amanclaw-api`
Expected: Compiles.

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-api/
git commit -m "feat(api): add webhook receiver and management routes"
```

---

## Task 11: EventEmitter Trait

**Files:**
- Create: `rust/crates/amanclaw-traits/src/event.rs`
- Modify: `rust/crates/amanclaw-traits/src/lib.rs`

**Step 1: Create EventEmitter trait**

Create `rust/crates/amanclaw-traits/src/event.rs`:

```rust
/// Trait for emitting events from the engine to external observers (gateway, monitoring).
pub trait EventEmitter: Send + Sync {
    fn emit(&self, topic: &str, data: serde_json::Value);
}

/// No-op emitter for CLI mode — zero overhead.
pub struct NoopEmitter;

impl EventEmitter for NoopEmitter {
    fn emit(&self, _topic: &str, _data: serde_json::Value) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_emitter() {
        let emitter = NoopEmitter;
        // Should not panic
        emitter.emit("test.event", serde_json::json!({"key": "value"}));
    }
}
```

**Step 2: Add `pub mod event;` to traits lib.rs**

**Step 3: Run tests**

Run: `cd rust && cargo test -p amanclaw-traits event -- --nocapture`
Expected: Pass.

**Step 4: Commit**

```bash
git add rust/crates/amanclaw-traits/src/event.rs rust/crates/amanclaw-traits/src/lib.rs
git commit -m "feat(traits): add EventEmitter trait with NoopEmitter"
```

---

## Task 12: WebSocket Gateway — New Crate

**Files:**
- Create: `rust/crates/amanclaw-gateway/Cargo.toml`
- Create: `rust/crates/amanclaw-gateway/src/lib.rs`
- Create: `rust/crates/amanclaw-gateway/src/session.rs`
- Create: `rust/crates/amanclaw-gateway/src/handler.rs`
- Create: `rust/crates/amanclaw-gateway/src/protocol.rs`
- Modify: `rust/Cargo.toml` (add to workspace members)

**Step 1: Create Cargo.toml**

Create `rust/crates/amanclaw-gateway/Cargo.toml`:

```toml
[package]
name = "amanclaw-gateway"
version.workspace = true
edition.workspace = true

[dependencies]
amanclaw-traits = { path = "../amanclaw-traits" }
axum = { version = "0.8", features = ["json", "ws"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
futures = "0.3"
```

**Step 2: Create protocol types (`protocol.rs`)**

JSON-RPC 2.0 request/response/event structs. See design doc Section 5.

**Step 3: Create SessionManager (`session.rs`)**

Session registry, topic-based pub/sub, glob matching, stale cleanup. Include tests for:
- Session connect/disconnect
- Topic subscription matching
- Stale session cleanup

**Step 4: Create GatewayHandler (`handler.rs`)**

Method dispatch for `gateway.auth`, `gateway.ping`, `subscribe`, `unsubscribe`, `engine.status`, etc. Start with auth + ping + subscribe. Other methods can be stubs returning `method_not_found`.

**Step 5: Create lib.rs with WebSocket handler**

```rust
pub mod protocol;
pub mod session;
pub mod handler;
```

Plus the Axum WebSocket upgrade handler and `GatewayState` struct.

**Step 6: Add to workspace**

In `rust/Cargo.toml`, add `"crates/amanclaw-gateway"` to `members`.

**Step 7: Run tests**

Run: `cd rust && cargo test -p amanclaw-gateway -- --nocapture`
Expected: All gateway tests pass.

**Step 8: Commit**

```bash
git add rust/crates/amanclaw-gateway/ rust/Cargo.toml
git commit -m "feat(gateway): implement WebSocket gateway with session manager and JSON-RPC 2.0 protocol"
```

---

## Task 13: Gateway — Pipeline Event Emission

**Files:**
- Modify: `rust/crates/amanclaw-core/src/pipeline.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

**Step 1: Add emitter to Pipeline::Full**

Add `emitter: Arc<dyn amanclaw_traits::event::EventEmitter>` to the `Full` variant and `with_services()`.

**Step 2: Emit events at key pipeline points**

- After auth check: `emitter.emit("message.received", ...)`
- After tool call: `emitter.emit("agent.tool_call", ...)`
- After response: `emitter.emit("message.sent", ...)`
- On rate limit: `emitter.emit("security.rate_limited", ...)`
- On injection: `emitter.emit("security.injection", ...)`

**Step 3: Update Engine::new() to pass NoopEmitter by default**

```rust
let emitter: Arc<dyn EventEmitter> = Arc::new(NoopEmitter);
let pipeline = Pipeline::with_services(auth_arc.clone(), rate_limiter, context_engine, memory_arc, llm_arc, emitter);
```

**Step 4: Run tests**

Run: `cd rust && cargo test --workspace 2>&1 | tail -20`
Expected: All pass.

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-core/src/pipeline.rs rust/crates/amanclaw-core/src/lib.rs
git commit -m "feat(core): integrate EventEmitter into pipeline for gateway event broadcasting"
```

---

## Task 14: Gateway — Mount on API Server

**Files:**
- Modify: `rust/crates/amanclaw-api/src/lib.rs`
- Modify: `rust/crates/amanclaw-api/Cargo.toml`
- Modify: `rust/crates/amanclaw-traits/src/config.rs`

**Step 1: Add GatewayConfig**

In config.rs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_heartbeat")]
    pub heartbeat_interval_secs: u64,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_stale_timeout")]
    pub stale_session_timeout_secs: u64,
}

fn default_heartbeat() -> u64 { 30 }
fn default_max_connections() -> usize { 50 }
fn default_stale_timeout() -> u64 { 60 }
```

Add to `AppConfig`:

```rust
#[serde(default)]
pub gateway: GatewayConfig,
```

**Step 2: Add amanclaw-gateway dep to amanclaw-api**

**Step 3: Mount `/ws` route**

In `rust/crates/amanclaw-api/src/lib.rs`, conditionally add the WS route.

**Step 4: Run build**

Run: `cd rust && cargo build`
Expected: Compiles.

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-api/ rust/crates/amanclaw-traits/src/config.rs
git commit -m "feat(api): mount WebSocket gateway on /ws endpoint"
```

---

## Task 15: Sub-Agent Spawning

**Files:**
- Create: `rust/crates/amanclaw-core/src/subagent.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-traits/src/config.rs`

**Step 1: Add SubAgentConfig**

In config.rs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_per_session")]
    pub max_per_session: usize,
    #[serde(default = "default_max_global")]
    pub max_global: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_subagent_timeout")]
    pub default_timeout_secs: u64,
}

impl Default for SubAgentConfig { /* sensible defaults */ }

fn default_max_per_session() -> usize { 5 }
fn default_max_global() -> usize { 20 }
fn default_max_depth() -> usize { 2 }
fn default_subagent_timeout() -> u64 { 120 }
```

Add to `AppConfig`:

```rust
#[serde(default)]
pub subagents: SubAgentConfig,
```

**Step 2: Create SubAgentManager**

Create `rust/crates/amanclaw-core/src/subagent.rs` with:
- `SubAgent` struct, `SubAgentStatus` enum, `SpawnRequest`
- `SubAgentManager` with `spawn`, `cancel`, `cancel_all`, `get`, `list`, `collect_results`
- Tests for spawn limits, cancellation, depth checking

**Step 3: Add `pub mod subagent;` to lib.rs**

**Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-core subagent -- --nocapture`
Expected: All pass.

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-core/src/subagent.rs rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-traits/src/config.rs
git commit -m "feat(core): implement SubAgentManager with spawn limits and cancellation"
```

---

## Task 16: Sub-Agent Skill (LLM Tool)

**Files:**
- Create: `rust/crates/amanclaw-core/src/skills/mod.rs`
- Create: `rust/crates/amanclaw-core/src/skills/subagent_skill.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

**Step 1: Create the SubAgentSkill**

Implement the `Skill` trait for `SubAgentSkill` with three tools:
- `spawn_subagent` — spawns a sub-agent
- `check_subagents` — checks status/results
- `cancel_subagent` — cancels a running sub-agent

**Step 2: Register in Engine::new()**

Add `SubAgentSkill` to the built-in skills list if `config.subagents.enabled`.

**Step 3: Run build**

Run: `cd rust && cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/skills/
git commit -m "feat(core): add SubAgentSkill as built-in LLM tool for parallel task execution"
```

---

## Task 17: Skill Registry — New Crate

**Files:**
- Create: `rust/crates/amanclaw-registry/Cargo.toml`
- Create: `rust/crates/amanclaw-registry/src/lib.rs`
- Create: `rust/crates/amanclaw-registry/src/manifest.rs`
- Create: `rust/crates/amanclaw-registry/src/local.rs`
- Create: `rust/crates/amanclaw-registry/src/remote.rs`
- Modify: `rust/Cargo.toml` (workspace members)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "amanclaw-registry"
version.workspace = true
edition.workspace = true

[dependencies]
amanclaw-traits = { path = "../amanclaw-traits" }
serde = { workspace = true }
serde_json = { workspace = true }
toml = "0.8"
semver = "1"
reqwest = { version = "0.12", features = ["rustls-tls", "json"], default-features = false }
sha2 = "0.10"
hex = "0.4"
flate2 = "1"
tar = "0.4"
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
chrono = "0.4"

[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
```

**Step 2: Implement manifest.rs**

Parse `amanclaw-skill.toml` format. Include tests for manifest deserialization.

**Step 3: Implement local.rs**

`SkillRegistry` with `install_from_path`, `uninstall`, `list_installed`, `search_installed`. Include SQLite schema for `installed_skills` and `skill_dependencies`.

**Step 4: Implement remote.rs**

`RemoteRegistry` with `refresh_index`, `search`, `resolve`, `download`. Include checksum verification.

**Step 5: Create lib.rs**

```rust
pub mod manifest;
pub mod local;
pub mod remote;
```

**Step 6: Add to workspace**

In `rust/Cargo.toml`, add `"crates/amanclaw-registry"` to `members`.

**Step 7: Run tests**

Run: `cd rust && cargo test -p amanclaw-registry -- --nocapture`
Expected: All pass.

**Step 8: Commit**

```bash
git add rust/crates/amanclaw-registry/ rust/Cargo.toml
git commit -m "feat(registry): implement skill marketplace with manifest parsing, local registry, and remote index"
```

---

## Task 18: Registry — Engine Integration

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-core/Cargo.toml`
- Modify: `rust/crates/amanclaw-traits/src/config.rs`

**Step 1: Add RegistryConfig**

In config.rs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_registry_skills_dir")]
    pub skills_dir: String,
    #[serde(default)]
    pub remote_url: Option<String>,
    #[serde(default)]
    pub auto_update_check: bool,
    #[serde(default)]
    pub allow_unverified: bool,
}

fn default_registry_skills_dir() -> String { "./plugins/registry".into() }
```

Add to `AppConfig`:

```rust
#[serde(default)]
pub registry: RegistryConfig,
```

**Step 2: Add amanclaw-registry dep to amanclaw-core**

**Step 3: Auto-load installed skills in Engine::new()**

After loading WASM and script plugins, load registry-installed skills.

**Step 4: Run build**

Run: `cd rust && cargo build`
Expected: Compiles.

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-core/ rust/crates/amanclaw-traits/src/config.rs
git commit -m "feat(core): auto-load registry-installed skills at engine startup"
```

---

## Task 19: Schema Migrations for New Tables

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/schema.rs`

**Step 1: Add cron_history, webhook_history, installed_skills tables**

Append to `INIT_SQL`:

```sql
CREATE TABLE IF NOT EXISTS cron_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    output TEXT,
    duration_ms INTEGER,
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cron_history_job ON cron_history(job_id, executed_at);

CREATE TABLE IF NOT EXISTS webhook_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    webhook_id TEXT NOT NULL,
    status TEXT NOT NULL,
    source_ip TEXT,
    payload_preview TEXT,
    error TEXT,
    duration_ms INTEGER,
    received_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_webhook_history ON webhook_history(webhook_id, received_at);

CREATE TABLE IF NOT EXISTS installed_skills (
    name TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL DEFAULT '',
    runtime_type TEXT NOT NULL,
    keywords TEXT NOT NULL DEFAULT '[]',
    categories TEXT NOT NULL DEFAULT '[]',
    entrypoint TEXT NOT NULL,
    permissions TEXT NOT NULL DEFAULT '{}',
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    source TEXT NOT NULL DEFAULT 'local',
    checksum TEXT
);

CREATE TABLE IF NOT EXISTS skill_dependencies (
    skill_name TEXT NOT NULL REFERENCES installed_skills(name),
    depends_on TEXT NOT NULL,
    version_req TEXT NOT NULL,
    optional INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (skill_name, depends_on)
);
```

**Step 2: Run tests**

Run: `cd rust && cargo test -p amanclaw-memory -- --nocapture`
Expected: All pass.

**Step 3: Commit**

```bash
git add rust/crates/amanclaw-memory/src/schema.rs
git commit -m "feat(memory): add schema for cron_history, webhook_history, and installed_skills tables"
```

---

## Task 20: Final Integration Test

**Files:**
- Modify: `rust/crates/amanclaw-core/tests/integration.rs`

**Step 1: Add integration test verifying cron + pipeline bypass**

Add a test that creates an engine with a cron job configured and verifies that the scheduler event channel produces messages.

**Step 2: Add test verifying soul loading**

Create a temp soul file, configure an agent profile with `soul_file`, and verify the system prompt is resolved correctly.

**Step 3: Run all workspace tests**

Run: `cd rust && cargo test --workspace -- --nocapture 2>&1 | tail -30`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/tests/
git commit -m "test: add integration tests for cron pipeline bypass and soul loading"
```

---

## Summary

| Task | Feature | Commit Message |
|------|---------|---------------|
| 1 | FTS5 Schema | `feat(memory): add FTS5 virtual table with sync triggers` |
| 2 | FTS5 Hybrid Search | `feat(memory): implement FTS5 hybrid search with BM25 + cosine RRF` |
| 3 | SOUL.md Traits | `feat(traits): add soul_file to AgentProfile` |
| 4 | SoulLoader | `feat(core): implement SoulLoader with inheritance and variables` |
| 5 | Soul Engine Integration | `feat(core): integrate SoulLoader into Engine startup` |
| 6 | Cron Traits | `feat(traits): add cron/webhook/subagent message flags` |
| 7 | Cron Scheduler | `feat(core): implement cron scheduler` |
| 8 | Cron Engine Integration | `feat(core): integrate cron into Engine with tokio::select!` |
| 9 | Webhook Config + Router | `feat(core): implement WebhookRouter` |
| 10 | Webhook API Routes | `feat(api): add webhook routes` |
| 11 | EventEmitter Trait | `feat(traits): add EventEmitter trait` |
| 12 | Gateway Crate | `feat(gateway): WebSocket gateway with sessions and JSON-RPC` |
| 13 | Pipeline Events | `feat(core): integrate EventEmitter into pipeline` |
| 14 | Gateway Mount | `feat(api): mount WebSocket gateway` |
| 15 | SubAgent Manager | `feat(core): implement SubAgentManager` |
| 16 | SubAgent Skill | `feat(core): add SubAgentSkill as LLM tool` |
| 17 | Registry Crate | `feat(registry): skill marketplace` |
| 18 | Registry Engine Integration | `feat(core): auto-load registry skills` |
| 19 | Schema Migrations | `feat(memory): add history and registry tables` |
| 20 | Integration Tests | `test: integration tests for new features` |
