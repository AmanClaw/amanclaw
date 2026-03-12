# Reactive Learning Engine (RLE) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a correction-based learning system that detects user corrections, stores them as structured rules with three-layer priority (user > community > global), and injects learned knowledge into future LLM contexts — making the bot smarter with every interaction.

**Architecture:** Two new middleware (RleRetrieveMiddleware before Context, RleDetectMiddleware after ToolCalling) connected by a KnowledgeStore that uses SQLite rules for fast exact/fuzzy lookup and the existing vector store for semantic fallback. The LLM is used only for correction extraction; the intelligence lives in the accumulated rule store.

**Tech Stack:** Rust, SQLite (sqlx), existing amanclaw-memory/amanclaw-traits infrastructure, serde_json for structured records.

---

## Chunk 1: Data Model & Knowledge Store

### Task 1: Add RLE tables to schema

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/schema.rs`

- [ ] **Step 1: Write the test for migration SQL validity**

In `rust/crates/amanclaw-memory/src/schema.rs`, add to the existing `#[cfg(test)] mod tests` (or create one):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_sql_is_valid_syntax() {
        // Just ensure the SQL string compiles and is non-empty
        assert!(!RLE_INIT_SQL.is_empty());
        assert!(RLE_INIT_SQL.contains("correction_rules"));
        assert!(RLE_INIT_SQL.contains("correction_events"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-memory schema::tests::rle_sql_is_valid_syntax 2>&1 | tail -5`
Expected: FAIL — `RLE_INIT_SQL` not found.

- [ ] **Step 3: Add RLE_INIT_SQL to schema.rs**

Add after the existing `MIGRATE_NS_STMTS` in `rust/crates/amanclaw-memory/src/schema.rs`:

```rust
/// SQL to create Reactive Learning Engine tables.
pub const RLE_INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS correction_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trigger_pattern TEXT NOT NULL,
    wrong_response TEXT,
    correct_response TEXT NOT NULL,
    topic TEXT,
    user_id TEXT,
    community_id TEXT,
    layer TEXT NOT NULL DEFAULT 'global' CHECK (layer IN ('user', 'community', 'global')),
    confidence REAL NOT NULL DEFAULT 0.7,
    hit_count INTEGER NOT NULL DEFAULT 0,
    last_used DATETIME,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'candidate', 'retracted')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS correction_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER REFERENCES correction_rules(id),
    user_id TEXT NOT NULL,
    platform TEXT,
    signal_type TEXT NOT NULL,
    signal_confidence REAL NOT NULL,
    source_user_msg TEXT,
    source_bot_msg TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_rules_lookup ON correction_rules(layer, status, topic);
CREATE INDEX IF NOT EXISTS idx_rules_user ON correction_rules(user_id, status);
CREATE INDEX IF NOT EXISTS idx_rules_community ON correction_rules(community_id, status);
CREATE INDEX IF NOT EXISTS idx_events_rule ON correction_events(rule_id);
"#;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p amanclaw-memory schema::tests::rle_sql_is_valid_syntax 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-memory/src/schema.rs
git commit -m "feat(rle): add correction_rules and correction_events schema"
```

---

### Task 2: Create KnowledgeStore with SQLite backend

**Files:**
- Create: `rust/crates/amanclaw-memory/src/knowledge_store.rs`
- Modify: `rust/crates/amanclaw-memory/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `rust/crates/amanclaw-memory/src/knowledge_store.rs` with types + tests first, no implementation:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// A correction rule learned from user interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRule {
    pub id: i64,
    pub trigger_pattern: String,
    pub wrong_response: Option<String>,
    pub correct_response: String,
    pub topic: Option<String>,
    pub user_id: Option<String>,
    pub community_id: Option<String>,
    pub layer: String, // "user" | "community" | "global"
    pub confidence: f64,
    pub hit_count: i64,
    pub status: String, // "active" | "candidate" | "retracted"
}

/// A correction event — immutable audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEvent {
    pub rule_id: Option<i64>,
    pub user_id: String,
    pub platform: Option<String>,
    pub signal_type: String,
    pub signal_confidence: f64,
    pub source_user_msg: Option<String>,
    pub source_bot_msg: Option<String>,
}

/// A detected correction from conversation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedCorrection {
    pub trigger: String,
    pub wrong_response: Option<String>,
    pub correct_response: String,
    pub topic: Option<String>,
    pub confidence: f64,
    pub signal_type: String,
}

/// Query for retrieving relevant corrections.
#[derive(Debug, Clone)]
pub struct CorrectionQuery {
    pub user_message: String,
    pub user_id: String,
    pub community_id: Option<String>,
    pub topic: Option<String>,
}

/// A matched correction with its resolution layer.
#[derive(Debug, Clone)]
pub struct CorrectionMatch {
    pub rule: CorrectionRule,
    pub match_score: f64,
}

/// SQLite-backed knowledge store for correction rules.
pub struct KnowledgeStore {
    pool: SqlitePool,
}

impl KnowledgeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize RLE tables.
    pub async fn init(&self) -> Result<()> {
        sqlx::raw_sql(crate::schema::RLE_INIT_SQL)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Store a new correction rule or update an existing one if the trigger matches.
    pub async fn upsert_rule(
        &self,
        correction: &DetectedCorrection,
        user_id: &str,
        community_id: Option<&str>,
        layer: &str,
    ) -> Result<i64> {
        todo!()
    }

    /// Log a correction event for audit.
    pub async fn log_event(&self, event: &CorrectionEvent) -> Result<()> {
        todo!()
    }

    /// Query for relevant corrections, returning matches ordered by layer priority
    /// (user > community > global) then by confidence.
    pub async fn query(&self, q: &CorrectionQuery) -> Result<Vec<CorrectionMatch>> {
        todo!()
    }

    /// Record a hit on a rule (increment hit_count, update last_used).
    pub async fn record_hit(&self, rule_id: i64) -> Result<()> {
        todo!()
    }

    /// Promote candidates that have crossed the confidence threshold.
    pub async fn promote_candidates(&self, threshold: f64) -> Result<u64> {
        todo!()
    }

    /// Retract (soft-delete) a rule.
    pub async fn retract_rule(&self, rule_id: i64) -> Result<bool> {
        todo!()
    }

    /// Get all active rules for a user (for /learned command).
    pub async fn get_user_rules(&self, user_id: &str) -> Result<Vec<CorrectionRule>> {
        todo!()
    }

    /// Get all active rules for a community (for /learned community command).
    pub async fn get_community_rules(&self, community_id: &str) -> Result<Vec<CorrectionRule>> {
        todo!()
    }

    /// Retract all rules for a user (for /forget all command).
    pub async fn retract_all_user_rules(&self, user_id: &str) -> Result<u64> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn make_store() -> KnowledgeStore {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        // Run base schema first
        sqlx::raw_sql(crate::schema::INIT_SQL)
            .execute(&pool)
            .await
            .unwrap();
        let store = KnowledgeStore::new(pool);
        store.init().await.unwrap();
        store
    }

    #[tokio::test]
    async fn test_upsert_and_query_user_rule() {
        let store = make_store().await;

        let correction = DetectedCorrection {
            trigger: "prayer time for Asr in KL".into(),
            wrong_response: Some("4:30 PM".into()),
            correct_response: "4:15 PM".into(),
            topic: Some("solat".into()),
            confidence: 0.90,
            signal_type: "direct_correction".into(),
        };

        let rule_id = store
            .upsert_rule(&correction, "user1", None, "user")
            .await
            .unwrap();
        assert!(rule_id > 0);

        let matches = store
            .query(&CorrectionQuery {
                user_message: "What time is Asr prayer in KL?".into(),
                user_id: "user1".into(),
                community_id: None,
                topic: Some("solat".into()),
            })
            .await
            .unwrap();

        assert!(!matches.is_empty());
        assert_eq!(matches[0].rule.correct_response, "4:15 PM");
    }

    #[tokio::test]
    async fn test_layer_priority_user_over_global() {
        let store = make_store().await;

        // Global rule
        let global = DetectedCorrection {
            trigger: "Asr time".into(),
            wrong_response: None,
            correct_response: "4:30 PM".into(),
            topic: Some("solat".into()),
            confidence: 0.85,
            signal_type: "direct_correction".into(),
        };
        store
            .upsert_rule(&global, "any", None, "global")
            .await
            .unwrap();

        // User-level override
        let user = DetectedCorrection {
            trigger: "Asr time".into(),
            wrong_response: None,
            correct_response: "4:15 PM".into(),
            topic: Some("solat".into()),
            confidence: 0.90,
            signal_type: "direct_correction".into(),
        };
        store
            .upsert_rule(&user, "user1", None, "user")
            .await
            .unwrap();

        let matches = store
            .query(&CorrectionQuery {
                user_message: "Asr time".into(),
                user_id: "user1".into(),
                community_id: None,
                topic: Some("solat".into()),
            })
            .await
            .unwrap();

        // User-level rule should come first
        assert!(matches.len() >= 2);
        assert_eq!(matches[0].rule.layer, "user");
        assert_eq!(matches[0].rule.correct_response, "4:15 PM");
    }

    #[tokio::test]
    async fn test_record_hit_increments_count() {
        let store = make_store().await;

        let correction = DetectedCorrection {
            trigger: "test trigger".into(),
            wrong_response: None,
            correct_response: "correct answer".into(),
            topic: None,
            confidence: 0.80,
            signal_type: "explicit_negation".into(),
        };
        let rule_id = store
            .upsert_rule(&correction, "user1", None, "user")
            .await
            .unwrap();

        store.record_hit(rule_id).await.unwrap();
        store.record_hit(rule_id).await.unwrap();

        let rules = store.get_user_rules("user1").await.unwrap();
        assert_eq!(rules[0].hit_count, 2);
    }

    #[tokio::test]
    async fn test_retract_rule() {
        let store = make_store().await;

        let correction = DetectedCorrection {
            trigger: "something".into(),
            wrong_response: None,
            correct_response: "right".into(),
            topic: None,
            confidence: 0.80,
            signal_type: "direct_correction".into(),
        };
        let rule_id = store
            .upsert_rule(&correction, "user1", None, "user")
            .await
            .unwrap();

        assert!(store.retract_rule(rule_id).await.unwrap());

        let rules = store.get_user_rules("user1").await.unwrap();
        assert!(rules.is_empty()); // retracted rules not returned
    }

    #[tokio::test]
    async fn test_retract_all_user_rules() {
        let store = make_store().await;

        for i in 0..3 {
            let correction = DetectedCorrection {
                trigger: format!("trigger {i}"),
                wrong_response: None,
                correct_response: format!("correct {i}"),
                topic: None,
                confidence: 0.80,
                signal_type: "direct_correction".into(),
            };
            store
                .upsert_rule(&correction, "user1", None, "user")
                .await
                .unwrap();
        }

        let count = store.retract_all_user_rules("user1").await.unwrap();
        assert_eq!(count, 3);

        let rules = store.get_user_rules("user1").await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn test_log_event() {
        let store = make_store().await;

        let event = CorrectionEvent {
            rule_id: None,
            user_id: "user1".into(),
            platform: Some("telegram".into()),
            signal_type: "direct_correction".into(),
            signal_confidence: 0.90,
            source_user_msg: Some("No, it's 4:15".into()),
            source_bot_msg: Some("Asr is at 4:30".into()),
        };

        // Should not panic
        store.log_event(&event).await.unwrap();
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_rule() {
        let store = make_store().await;

        let correction1 = DetectedCorrection {
            trigger: "Asr time KL".into(),
            wrong_response: None,
            correct_response: "4:15 PM".into(),
            topic: Some("solat".into()),
            confidence: 0.80,
            signal_type: "direct_correction".into(),
        };
        let id1 = store
            .upsert_rule(&correction1, "user1", None, "user")
            .await
            .unwrap();

        // Same trigger, higher confidence
        let correction2 = DetectedCorrection {
            trigger: "Asr time KL".into(),
            wrong_response: None,
            correct_response: "4:20 PM".into(),
            topic: Some("solat".into()),
            confidence: 0.95,
            signal_type: "direct_correction".into(),
        };
        let id2 = store
            .upsert_rule(&correction2, "user1", None, "user")
            .await
            .unwrap();

        // Should update the same row
        assert_eq!(id1, id2);

        let rules = store.get_user_rules("user1").await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].correct_response, "4:20 PM");
        assert!(rules[0].confidence >= 0.95);
    }

    #[tokio::test]
    async fn test_promote_candidates() {
        let store = make_store().await;

        // Insert a candidate rule with low confidence
        let correction = DetectedCorrection {
            trigger: "low confidence rule".into(),
            wrong_response: None,
            correct_response: "maybe correct".into(),
            topic: None,
            confidence: 0.50,
            signal_type: "rephrased_reask".into(),
        };
        let rule_id = store
            .upsert_rule(&correction, "user1", None, "user")
            .await
            .unwrap();

        // Bump confidence above threshold by updating
        sqlx::query("UPDATE correction_rules SET confidence = 0.75 WHERE id = ?")
            .bind(rule_id)
            .execute(&store.pool)
            .await
            .unwrap();

        let promoted = store.promote_candidates(0.70).await.unwrap();
        assert_eq!(promoted, 1);

        let rules = store.get_user_rules("user1").await.unwrap();
        assert_eq!(rules[0].status, "active");
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

In `rust/crates/amanclaw-memory/src/lib.rs`, add:

```rust
pub mod knowledge_store;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd rust && cargo test -p amanclaw-memory knowledge_store 2>&1 | tail -10`
Expected: FAIL — all `todo!()` methods panic.

- [ ] **Step 4: Implement upsert_rule**

Replace the `todo!()` in `upsert_rule`:

```rust
    pub async fn upsert_rule(
        &self,
        correction: &DetectedCorrection,
        user_id: &str,
        community_id: Option<&str>,
        layer: &str,
    ) -> Result<i64> {
        let status = if correction.confidence >= 0.7 {
            "active"
        } else {
            "candidate"
        };

        // Check for existing rule with same trigger + layer + user/community
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM correction_rules WHERE trigger_pattern = ? AND layer = ? AND COALESCE(user_id, '') = ? AND status != 'retracted' LIMIT 1"
        )
        .bind(&correction.trigger)
        .bind(layer)
        .bind(if layer == "global" { "" } else { user_id })
        .fetch_optional(&self.pool)
        .await?;

        if let Some(id) = existing {
            // Update existing rule
            sqlx::query(
                "UPDATE correction_rules SET correct_response = ?, wrong_response = ?, confidence = MAX(confidence, ?), status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
            )
            .bind(&correction.correct_response)
            .bind(&correction.wrong_response)
            .bind(correction.confidence)
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
            Ok(id)
        } else {
            // Insert new rule
            let id = sqlx::query_scalar::<_, i64>(
                "INSERT INTO correction_rules (trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id"
            )
            .bind(&correction.trigger)
            .bind(&correction.wrong_response)
            .bind(&correction.correct_response)
            .bind(&correction.topic)
            .bind(if layer == "global" { None } else { Some(user_id) })
            .bind(community_id)
            .bind(layer)
            .bind(correction.confidence)
            .bind(status)
            .fetch_one(&self.pool)
            .await?;
            Ok(id)
        }
    }
```

- [ ] **Step 5: Implement log_event**

```rust
    pub async fn log_event(&self, event: &CorrectionEvent) -> Result<()> {
        sqlx::query(
            "INSERT INTO correction_events (rule_id, user_id, platform, signal_type, signal_confidence, source_user_msg, source_bot_msg) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(event.rule_id)
        .bind(&event.user_id)
        .bind(&event.platform)
        .bind(&event.signal_type)
        .bind(event.signal_confidence)
        .bind(&event.source_user_msg)
        .bind(&event.source_bot_msg)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
```

- [ ] **Step 6: Implement query**

```rust
    pub async fn query(&self, q: &CorrectionQuery) -> Result<Vec<CorrectionMatch>> {
        // Build keyword set from user message for fuzzy matching
        let keywords: Vec<&str> = q.user_message.split_whitespace().collect();
        let keyword_pattern = keywords
            .iter()
            .map(|k| format!("%{}%", k.to_lowercase()))
            .collect::<Vec<_>>();

        // Fetch all active rules that could match, ordered by layer priority
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, String, Option<String>, Option<String>, Option<String>, String, f64, i64, String)>(
            "SELECT id, trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, hit_count, status FROM correction_rules WHERE status = 'active' AND (user_id IS NULL OR user_id = ? OR community_id = ?) ORDER BY CASE layer WHEN 'user' THEN 0 WHEN 'community' THEN 1 WHEN 'global' THEN 2 END, confidence DESC"
        )
        .bind(&q.user_id)
        .bind(&q.community_id)
        .fetch_all(&self.pool)
        .await?;

        let mut matches = Vec::new();
        for row in rows {
            let trigger_lower = row.1.to_lowercase();
            let msg_lower = q.user_message.to_lowercase();

            // Score: exact contains > keyword overlap > topic match
            let mut score = 0.0;

            if msg_lower.contains(&trigger_lower) || trigger_lower.contains(&msg_lower) {
                score = 1.0;
            } else {
                // Keyword overlap
                let trigger_words: Vec<&str> = trigger_lower.split_whitespace().collect();
                let overlap = keywords
                    .iter()
                    .filter(|k| trigger_words.iter().any(|tw| tw.contains(&k.to_lowercase())))
                    .count();
                if trigger_words.len() > 0 {
                    score = overlap as f64 / trigger_words.len() as f64;
                }
            }

            // Topic boost
            if let (Some(ref rule_topic), Some(ref query_topic)) = (&row.4, &q.topic) {
                if rule_topic == query_topic {
                    score += 0.2;
                }
            }

            // Filter: only return rules with reasonable match
            if score >= 0.3 {
                // Layer filter: user rules only for matching user_id
                let rule_user_id = &row.5;
                let rule_community_id = &row.6;
                let layer = &row.7;

                let relevant = match layer.as_str() {
                    "user" => rule_user_id.as_deref() == Some(&q.user_id),
                    "community" => {
                        q.community_id.is_some()
                            && rule_community_id.as_deref() == q.community_id.as_deref()
                    }
                    "global" => true,
                    _ => false,
                };

                if relevant {
                    matches.push(CorrectionMatch {
                        rule: CorrectionRule {
                            id: row.0,
                            trigger_pattern: row.1,
                            wrong_response: row.2,
                            correct_response: row.3,
                            topic: row.4,
                            user_id: row.5,
                            community_id: row.6,
                            layer: row.7,
                            confidence: row.8,
                            hit_count: row.9,
                            status: row.10,
                        },
                        match_score: score * row.8, // score * confidence
                    });
                }
            }
        }

        // Sort by layer priority first, then match_score
        matches.sort_by(|a, b| {
            let layer_ord = |l: &str| -> u8 {
                match l {
                    "user" => 0,
                    "community" => 1,
                    "global" => 2,
                    _ => 3,
                }
            };
            let la = layer_ord(&a.rule.layer);
            let lb = layer_ord(&b.rule.layer);
            la.cmp(&lb)
                .then(b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal))
        });

        Ok(matches)
    }
```

- [ ] **Step 7: Implement remaining methods**

```rust
    pub async fn record_hit(&self, rule_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE correction_rules SET hit_count = hit_count + 1, last_used = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(rule_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn promote_candidates(&self, threshold: f64) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE correction_rules SET status = 'active', updated_at = CURRENT_TIMESTAMP WHERE status = 'candidate' AND confidence >= ?"
        )
        .bind(threshold)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn retract_rule(&self, rule_id: i64) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE correction_rules SET status = 'retracted', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status != 'retracted'"
        )
        .bind(rule_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_user_rules(&self, user_id: &str) -> Result<Vec<CorrectionRule>> {
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, String, Option<String>, Option<String>, Option<String>, String, f64, i64, String)>(
            "SELECT id, trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, hit_count, status FROM correction_rules WHERE user_id = ? AND status = 'active' ORDER BY confidence DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| CorrectionRule {
            id: row.0, trigger_pattern: row.1, wrong_response: row.2, correct_response: row.3,
            topic: row.4, user_id: row.5, community_id: row.6, layer: row.7,
            confidence: row.8, hit_count: row.9, status: row.10,
        }).collect())
    }

    pub async fn get_community_rules(&self, community_id: &str) -> Result<Vec<CorrectionRule>> {
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, String, Option<String>, Option<String>, Option<String>, String, f64, i64, String)>(
            "SELECT id, trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, hit_count, status FROM correction_rules WHERE community_id = ? AND status = 'active' ORDER BY confidence DESC"
        )
        .bind(community_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| CorrectionRule {
            id: row.0, trigger_pattern: row.1, wrong_response: row.2, correct_response: row.3,
            topic: row.4, user_id: row.5, community_id: row.6, layer: row.7,
            confidence: row.8, hit_count: row.9, status: row.10,
        }).collect())
    }

    pub async fn retract_all_user_rules(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE correction_rules SET status = 'retracted', updated_at = CURRENT_TIMESTAMP WHERE user_id = ? AND status = 'active'"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
```

- [ ] **Step 8: Run all knowledge_store tests**

Run: `cd rust && cargo test -p amanclaw-memory knowledge_store 2>&1 | tail -15`
Expected: All 8 tests PASS.

- [ ] **Step 9: Commit**

```bash
git add rust/crates/amanclaw-memory/src/knowledge_store.rs rust/crates/amanclaw-memory/src/lib.rs
git commit -m "feat(rle): implement KnowledgeStore with SQLite backend"
```

---

## Chunk 2: Correction Detection Engine

### Task 3: Create the correction detector

**Files:**
- Create: `rust/crates/amanclaw-core/src/learning/mod.rs`
- Create: `rust/crates/amanclaw-core/src/learning/detection.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs` (add `pub mod learning;`)

The detection engine uses the LLM to analyze a user-bot exchange and extract structured corrections. It sends a focused extraction prompt and parses the JSON response.

- [ ] **Step 1: Write the detection module with types and test**

Create `rust/crates/amanclaw-core/src/learning/mod.rs`:

```rust
pub mod detection;
```

Create `rust/crates/amanclaw-core/src/learning/detection.rs`:

```rust
use amanclaw_llm::client::{LlmClient, LlmResponse};
use amanclaw_memory::knowledge_store::DetectedCorrection;
use anyhow::Result;

/// Prompt sent to the LLM to extract corrections from a conversation exchange.
const DETECTION_PROMPT: &str = r#"Analyze this conversation exchange for corrections. A correction is when the user indicates the assistant's response was wrong and provides the right answer.

Look for these signals:
- Explicit negation: "No, that's wrong. It's X"
- Direct correction: "Actually it's X, not Y"
- Rephrased re-ask: User asks the same thing differently after getting an answer
- Negative reaction: "That's not helpful", "Wrong"

Respond with a JSON array of corrections found. If no corrections, respond with [].
Each correction:
{
  "trigger": "what the user originally asked about",
  "wrong_response": "what the assistant said wrong (or null)",
  "correct_response": "what the user says is correct",
  "topic": "topic category if identifiable (or null)",
  "confidence": 0.0-1.0,
  "signal_type": "explicit_negation|direct_correction|rephrased_reask|negative_reaction|abandoned_flow"
}

IMPORTANT: Only output valid JSON. No explanation. No markdown fences."#;

/// Analyzes a conversation exchange and extracts corrections.
pub async fn detect_corrections(
    llm: &LlmClient,
    user_message: &str,
    bot_response: &str,
    history_context: &[(&str, &str)], // last few (role, content) pairs for context
) -> Result<Vec<DetectedCorrection>> {
    let mut exchange = String::new();

    // Add recent history for context
    for (role, content) in history_context {
        exchange.push_str(&format!("{role}: {content}\n"));
    }
    exchange.push_str(&format!("assistant: {bot_response}\nuser: {user_message}"));

    let messages = vec![
        serde_json::json!({"role": "system", "content": DETECTION_PROMPT}),
        serde_json::json!({"role": "user", "content": exchange}),
    ];

    let response = llm.call(&messages, &[]).await;

    match response {
        Ok(LlmResponse::Text(text)) => parse_corrections(&text),
        _ => Ok(vec![]),
    }
}

/// Parse the LLM's JSON response into DetectedCorrection structs.
fn parse_corrections(text: &str) -> Result<Vec<DetectedCorrection>> {
    // Try to extract JSON array from response (handle markdown fences, etc.)
    let trimmed = text.trim();
    let json_str = if trimmed.starts_with("```") {
        // Strip markdown fences
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    match serde_json::from_str::<Vec<DetectedCorrection>>(json_str) {
        Ok(corrections) => Ok(corrections),
        Err(_) => {
            tracing::debug!(raw = %text, "Failed to parse correction JSON, skipping");
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_corrections_valid_json() {
        let json = r#"[{"trigger":"Asr time in KL","wrong_response":"4:30 PM","correct_response":"4:15 PM","topic":"solat","confidence":0.90,"signal_type":"direct_correction"}]"#;
        let corrections = parse_corrections(json).unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].correct_response, "4:15 PM");
        assert_eq!(corrections[0].confidence, 0.90);
    }

    #[test]
    fn test_parse_corrections_empty_array() {
        let json = "[]";
        let corrections = parse_corrections(json).unwrap();
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_parse_corrections_markdown_fenced() {
        let json = "```json\n[{\"trigger\":\"test\",\"wrong_response\":null,\"correct_response\":\"right\",\"topic\":null,\"confidence\":0.8,\"signal_type\":\"explicit_negation\"}]\n```";
        let corrections = parse_corrections(json).unwrap();
        assert_eq!(corrections.len(), 1);
    }

    #[test]
    fn test_parse_corrections_invalid_json_returns_empty() {
        let garbage = "I found no corrections in this exchange.";
        let corrections = parse_corrections(garbage).unwrap();
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_parse_corrections_multiple() {
        let json = r#"[
            {"trigger":"Asr time","wrong_response":"4:30","correct_response":"4:15","topic":"solat","confidence":0.9,"signal_type":"direct_correction"},
            {"trigger":"mazhab","wrong_response":"Hanafi","correct_response":"Shafi'i","topic":"fiqh","confidence":0.85,"signal_type":"explicit_negation"}
        ]"#;
        let corrections = parse_corrections(json).unwrap();
        assert_eq!(corrections.len(), 2);
    }

    #[test]
    fn test_detection_prompt_is_not_empty() {
        assert!(!DETECTION_PROMPT.is_empty());
        assert!(DETECTION_PROMPT.contains("JSON"));
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

In `rust/crates/amanclaw-core/src/lib.rs`, add:

```rust
pub mod learning;
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test -p amanclaw-core learning::detection 2>&1 | tail -10`
Expected: All 5 parsing tests PASS. (The `detect_corrections` async fn is not tested here — it requires a live LLM; we test the parsing layer only.)

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/learning/ rust/crates/amanclaw-core/src/lib.rs
git commit -m "feat(rle): add correction detection engine with LLM extraction"
```

---

## Chunk 3: RLE Middleware (Retrieve + Detect)

### Task 4: Create RleRetrieveMiddleware

**Files:**
- Create: `rust/crates/amanclaw-core/src/middleware/rle_retrieve.rs`
- Modify: `rust/crates/amanclaw-core/src/middleware/mod.rs`

This middleware runs before ContextMiddleware. It queries the KnowledgeStore for relevant corrections and stores them in extensions so ContextMiddleware can inject them into the LLM prompt.

- [ ] **Step 1: Write the middleware**

Create `rust/crates/amanclaw-core/src/middleware/rle_retrieve.rs`:

```rust
use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_memory::knowledge_store::{CorrectionMatch, CorrectionQuery, KnowledgeStore};
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Learned corrections retrieved for this request, stored in extensions.
pub struct LearnedCorrections(pub Vec<CorrectionMatch>);

/// Middleware that retrieves relevant learned corrections from the KnowledgeStore
/// and injects them into the pipeline context for downstream use.
pub struct RleRetrieveMiddleware {
    store: Arc<KnowledgeStore>,
}

impl RleRetrieveMiddleware {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for RleRetrieveMiddleware {
    async fn process(
        &self,
        mut ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        let query = CorrectionQuery {
            user_message: ctx.msg.text.clone(),
            user_id: ctx.msg.user_id.clone(),
            community_id: ctx.msg.channel_context.clone(),
            topic: None, // topic detection could be added later
        };

        match self.store.query(&query).await {
            Ok(matches) if !matches.is_empty() => {
                tracing::debug!(
                    count = matches.len(),
                    user = %ctx.msg.user_id,
                    "Retrieved learned corrections"
                );

                // Record hits for matched rules
                for m in &matches {
                    let _ = self.store.record_hit(m.rule.id).await;
                }

                ctx.extensions.insert(LearnedCorrections(matches));
            }
            Ok(_) => {} // no matches, continue
            Err(e) => {
                tracing::warn!(error = %e, "Failed to query knowledge store, continuing without");
            }
        }

        next.execute(ctx).await
    }
}
```

- [ ] **Step 2: Add module to middleware/mod.rs**

In `rust/crates/amanclaw-core/src/middleware/mod.rs`, add after existing module declarations:

```rust
pub mod rle_retrieve;
```

- [ ] **Step 3: Run compilation check**

Run: `cd rust && cargo check -p amanclaw-core 2>&1 | tail -10`
Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/middleware/rle_retrieve.rs rust/crates/amanclaw-core/src/middleware/mod.rs
git commit -m "feat(rle): add RleRetrieveMiddleware for correction injection"
```

---

### Task 5: Create RleDetectMiddleware

**Files:**
- Create: `rust/crates/amanclaw-core/src/middleware/rle_detect.rs`
- Modify: `rust/crates/amanclaw-core/src/middleware/mod.rs`

This middleware runs after ToolCallingMiddleware (wrapped by PersistMiddleware). It analyzes the completed exchange for corrections and stores them.

- [ ] **Step 1: Write the middleware**

Create `rust/crates/amanclaw-core/src/middleware/rle_detect.rs`:

```rust
use crate::learning::detection::detect_corrections;
use crate::middleware::tool_calling::LlmResponseText;
use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_llm::client::LlmClient;
use amanclaw_memory::knowledge_store::{CorrectionEvent, KnowledgeStore};
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Middleware that detects corrections in the completed exchange and stores them.
/// Runs after ToolCallingMiddleware so it can see the full response.
pub struct RleDetectMiddleware {
    store: Arc<KnowledgeStore>,
    llm: Arc<LlmClient>,
    memory: Arc<dyn MemoryBackend>,
}

impl RleDetectMiddleware {
    pub fn new(
        store: Arc<KnowledgeStore>,
        llm: Arc<LlmClient>,
        memory: Arc<dyn MemoryBackend>,
    ) -> Self {
        Self { store, llm, memory }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for RleDetectMiddleware {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        // First, let the chain complete (including ToolCalling)
        let result = next.execute(ctx).await?;

        // We need user info from the context, but ctx was moved.
        // Instead, we extract what we need from the result and the original message.
        // Note: By this point, ctx has been consumed by next.execute().
        // We work with the outgoing message to detect corrections.

        // This middleware is designed to wrap around the ToolCalling middleware.
        // However, since ctx is consumed, we use a different approach:
        // The PersistMiddleware already captures user_id, platform, etc. before calling next.
        // We'll use the same pattern.

        // For now, correction detection is fire-and-forget to avoid blocking the response.
        // We spawn it as a background task.

        Ok(result)
    }
}

/// Standalone function to run correction detection as a background task.
/// Called from PersistMiddleware after the exchange is complete.
pub async fn detect_and_store_corrections(
    store: &KnowledgeStore,
    llm: &LlmClient,
    memory: &dyn MemoryBackend,
    user_id: &str,
    platform: &str,
    ns: &str,
    user_message: &str,
    bot_response: &str,
) {
    // Get recent history for context (last 4 messages = 2 exchanges)
    let history = match memory.get_history(ns, user_id, 4).await {
        Ok(h) => h,
        Err(_) => return,
    };

    let history_pairs: Vec<(&str, &str)> = history
        .iter()
        .map(|m| (m.role.as_str(), m.content.as_str()))
        .collect();

    let corrections = match detect_corrections(llm, user_message, bot_response, &history_pairs).await {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(error = %e, "Correction detection failed");
            return;
        }
    };

    for correction in &corrections {
        if correction.confidence < 0.4 {
            continue; // too low to even store as candidate
        }

        // Determine layer: user-level for now (community detection can be added later)
        let layer = "user";

        match store
            .upsert_rule(correction, user_id, None, layer)
            .await
        {
            Ok(rule_id) => {
                let event = CorrectionEvent {
                    rule_id: Some(rule_id),
                    user_id: user_id.to_string(),
                    platform: Some(platform.to_string()),
                    signal_type: correction.signal_type.clone(),
                    signal_confidence: correction.confidence,
                    source_user_msg: Some(user_message.to_string()),
                    source_bot_msg: Some(bot_response.to_string()),
                };
                let _ = store.log_event(&event).await;

                tracing::info!(
                    rule_id,
                    trigger = %correction.trigger,
                    confidence = correction.confidence,
                    "Learned correction"
                );
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to store correction");
            }
        }
    }

    // Promote any candidates that have crossed the threshold
    let _ = store.promote_candidates(0.7).await;
}
```

- [ ] **Step 2: Add module to middleware/mod.rs**

In `rust/crates/amanclaw-core/src/middleware/mod.rs`, add:

```rust
pub mod rle_detect;
```

- [ ] **Step 3: Run compilation check**

Run: `cd rust && cargo check -p amanclaw-core 2>&1 | tail -10`
Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/middleware/rle_detect.rs rust/crates/amanclaw-core/src/middleware/mod.rs
git commit -m "feat(rle): add RleDetectMiddleware and background correction detection"
```

---

## Chunk 4: Integration into Pipeline

### Task 6: Inject learned corrections into LLM context

**Files:**
- Modify: `rust/crates/amanclaw-core/src/context_engine.rs`

The ContextMiddleware builds the LLM prompt. We need to check for `LearnedCorrections` in extensions and append them to the system prompt.

- [ ] **Step 1: Write test for learned knowledge injection**

Add to existing tests in `rust/crates/amanclaw-core/src/context_engine.rs`:

```rust
    #[test]
    fn test_format_learned_corrections() {
        use crate::middleware::rle_retrieve::LearnedCorrections;
        use amanclaw_memory::knowledge_store::{CorrectionMatch, CorrectionRule};

        let matches = vec![
            CorrectionMatch {
                rule: CorrectionRule {
                    id: 1,
                    trigger_pattern: "Asr time KL".into(),
                    wrong_response: Some("4:30 PM".into()),
                    correct_response: "4:15 PM".into(),
                    topic: Some("solat".into()),
                    user_id: Some("user1".into()),
                    community_id: None,
                    layer: "user".into(),
                    confidence: 0.92,
                    hit_count: 3,
                    status: "active".into(),
                },
                match_score: 0.92,
            },
        ];

        let formatted = format_learned_corrections(&matches);
        assert!(formatted.contains("4:15 PM"));
        assert!(formatted.contains("LEARNED KNOWLEDGE"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p amanclaw-core context_engine::tests::test_format_learned_corrections 2>&1 | tail -5`
Expected: FAIL — `format_learned_corrections` not found.

- [ ] **Step 3: Add format_learned_corrections function**

Add to `rust/crates/amanclaw-core/src/context_engine.rs`:

```rust
use crate::middleware::rle_retrieve::LearnedCorrections;
use amanclaw_memory::knowledge_store::CorrectionMatch;

/// Format learned corrections for injection into the LLM system prompt.
pub fn format_learned_corrections(matches: &[CorrectionMatch]) -> String {
    if matches.is_empty() {
        return String::new();
    }

    let mut section = String::from("\n\n## Learned knowledge (treat as ground truth unless user says otherwise)");

    for m in matches {
        let conf_pct = (m.rule.confidence * 100.0) as u32;
        let layer_label = match m.rule.layer.as_str() {
            "user" => "personal",
            "community" => "community",
            "global" => "general",
            _ => "unknown",
        };

        if m.rule.confidence >= 0.85 {
            // High confidence: state as fact
            section.push_str(&format!(
                "\n- {} ({}%, {} knowledge)",
                m.rule.correct_response, conf_pct, layer_label
            ));
        } else {
            // Medium confidence: suggest with hedge
            section.push_str(&format!(
                "\n- Previously learned: {} ({}% confident, {} — verify with user if unsure)",
                m.rule.correct_response, conf_pct, layer_label
            ));
        }
    }

    section
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p amanclaw-core context_engine::tests::test_format_learned_corrections 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-core/src/context_engine.rs
git commit -m "feat(rle): add learned corrections formatting for LLM context"
```

---

### Task 7: Wire RLE into the context building and pipeline

**Files:**
- Modify: `rust/crates/amanclaw-core/src/middleware/context.rs`
- Modify: `rust/crates/amanclaw-core/src/middleware/persist.rs`
- Modify: `rust/crates/amanclaw-core/src/pipeline.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs` (Engine::start)

- [ ] **Step 1: Inject learned corrections in ContextMiddleware**

In `rust/crates/amanclaw-core/src/middleware/context.rs`, after `ctx.extensions.insert(context_result);` on line 43, add logic to inject learned corrections into the system prompt:

```rust
use crate::context_engine::format_learned_corrections;
use crate::middleware::rle_retrieve::LearnedCorrections;
use amanclaw_traits::context::ContextResult;

// After: ctx.extensions.insert(context_result);
// Add: inject learned corrections into system prompt
if let Some(learned) = ctx.extensions.get::<LearnedCorrections>() {
    if !learned.0.is_empty() {
        let correction_text = format_learned_corrections(&learned.0);
        if let Some(context_result) = ctx.extensions.get_mut::<ContextResult>() {
            // Append to system message
            if let Some(system_msg) = context_result.messages.first_mut() {
                if let Some(content) = system_msg.get("content").and_then(|c| c.as_str()) {
                    let new_content = format!("{}{}", content, correction_text);
                    *system_msg = serde_json::json!({"role": "system", "content": new_content});
                }
            }
        }
    }
}
```

- [ ] **Step 2: Add background correction detection in PersistMiddleware**

In `rust/crates/amanclaw-core/src/middleware/persist.rs`, after the `on_exchange_complete` call (around line 69), add:

```rust
use crate::middleware::rle_detect::detect_and_store_corrections;
use amanclaw_memory::knowledge_store::KnowledgeStore;

// After: self.context_engine.on_exchange_complete(...).await?;
// Add: fire-and-forget correction detection
if let Some(store) = ctx_or_self.knowledge_store.as_ref() {
    let store = store.clone();
    let llm = self.llm.clone();
    let memory = self.memory.clone();
    let uid = user_id.clone();
    let plat = platform.clone();
    let namespace = ns.clone();
    let user_msg = user_message.clone();
    let bot_msg = response_text.clone();

    tokio::spawn(async move {
        detect_and_store_corrections(
            &store, &llm, memory.as_ref(),
            &uid, &plat, &namespace, &user_msg, &bot_msg,
        ).await;
    });
}
```

Note: PersistMiddleware needs an `Option<Arc<KnowledgeStore>>` field. Add it to the struct and constructor:

```rust
pub struct PersistMiddleware {
    context_engine: Arc<dyn ContextEngine>,
    memory: Arc<dyn MemoryBackend>,
    llm: Arc<LlmClient>,
    emitter: Arc<dyn EventEmitter>,
    knowledge_store: Option<Arc<KnowledgeStore>>,
}

impl PersistMiddleware {
    pub fn new(
        context_engine: Arc<dyn ContextEngine>,
        memory: Arc<dyn MemoryBackend>,
        llm: Arc<LlmClient>,
        emitter: Arc<dyn EventEmitter>,
    ) -> Self {
        Self {
            context_engine, memory, llm, emitter,
            knowledge_store: None,
        }
    }

    pub fn with_knowledge_store(mut self, store: Arc<KnowledgeStore>) -> Self {
        self.knowledge_store = Some(store);
        self
    }
}
```

- [ ] **Step 3: Wire RleRetrieveMiddleware into Pipeline**

In `rust/crates/amanclaw-core/src/pipeline.rs`, update `with_services` to accept an optional `KnowledgeStore` and insert the RLE middleware:

```rust
use crate::middleware::rle_retrieve::RleRetrieveMiddleware;
use amanclaw_memory::knowledge_store::KnowledgeStore;

pub fn with_services(
    auth: Arc<RwLock<Auth>>,
    rate_limiter: RateLimiter,
    context_engine: Arc<dyn ContextEngine>,
    memory: Arc<dyn MemoryBackend>,
    llm: Arc<LlmClient>,
    emitter: Arc<dyn EventEmitter>,
    knowledge_store: Option<Arc<KnowledgeStore>>,
) -> Self {
    let mut middlewares: Vec<Box<dyn PipelineMiddleware>> = vec![
        Box::new(MetricsMiddleware),
        Box::new(AuthMiddleware::new(auth.clone())),
        Box::new(CommandMiddleware::new(auth, memory.clone())),
        Box::new(RateLimitMiddleware::new(rate_limiter, emitter.clone())),
        Box::new(SanitizeMiddleware::new(emitter.clone())),
    ];

    // Insert RLE retrieve middleware before context (if knowledge store available)
    if let Some(ref store) = knowledge_store {
        middlewares.push(Box::new(RleRetrieveMiddleware::new(store.clone())));
    }

    middlewares.push(Box::new(ContextMiddleware::new(context_engine.clone())));

    let mut persist = PersistMiddleware::new(context_engine, memory, llm.clone(), emitter);
    if let Some(store) = knowledge_store {
        persist = persist.with_knowledge_store(store);
    }
    middlewares.push(Box::new(persist));
    middlewares.push(Box::new(ToolCallingMiddleware::new(llm)));

    let chain = MiddlewareChain::new(middlewares);
    Self::Full { chain }
}
```

- [ ] **Step 4: Initialize KnowledgeStore in Engine::start**

In `rust/crates/amanclaw-core/src/lib.rs`, after `let memory = SqliteMemory::new(&db_path).await?;` (around line 76), add:

```rust
use amanclaw_memory::knowledge_store::KnowledgeStore;

// After memory initialization:
let knowledge_store = Arc::new(KnowledgeStore::new(memory.pool().clone()));
knowledge_store.init().await?;
tracing::info!("Reactive Learning Engine initialized");
```

Then pass it to `Pipeline::with_services`:

```rust
let pipeline = Pipeline::with_services(
    auth_arc.clone(),
    rate_limiter,
    context_engine,
    memory_arc,
    llm_arc,
    emitter,
    Some(knowledge_store),
);
```

- [ ] **Step 5: Run compilation check**

Run: `cd rust && cargo check -p amanclaw-core 2>&1 | tail -15`
Expected: Compiles without errors.

- [ ] **Step 6: Run existing tests to ensure nothing broke**

Run: `cd rust && cargo test -p amanclaw-core 2>&1 | tail -15`
Expected: All existing tests still PASS. (The pipeline stub test doesn't use `with_services`, so it's unaffected.)

- [ ] **Step 7: Commit**

```bash
git add rust/crates/amanclaw-core/src/middleware/context.rs rust/crates/amanclaw-core/src/middleware/persist.rs rust/crates/amanclaw-core/src/pipeline.rs rust/crates/amanclaw-core/src/lib.rs
git commit -m "feat(rle): wire RLE middleware into pipeline and engine"
```

---

## Chunk 5: User-Facing Commands

### Task 8: Add /learned, /forget, /teach commands

**Files:**
- Modify: `rust/crates/amanclaw-core/src/middleware/command.rs`

The existing CommandMiddleware handles `/command` syntax. We add three new commands.

- [ ] **Step 1: Read the existing command middleware**

Run: Read `rust/crates/amanclaw-core/src/middleware/command.rs` to understand the current command handling pattern.

- [ ] **Step 2: Add RLE commands to CommandMiddleware**

Add to the command match block:

```rust
"/learned" => {
    // Show what the bot has learned about this user
    if let Some(store) = &self.knowledge_store {
        let rules = store.get_user_rules(&ctx.msg.user_id).await.unwrap_or_default();
        if rules.is_empty() {
            return Ok(Some(OutgoingMessage {
                chat_id: ctx.msg.chat_id,
                text: "I haven't learned anything specific about you yet. As we chat, I'll remember your corrections and preferences.".into(),
                parse_mode: None, reply_to: None, platform: None, topic_id: None, interactive: None,
            }));
        }
        let mut text = String::from("Here's what I've learned about you:\n\n");
        for rule in &rules {
            let conf_pct = (rule.confidence * 100.0) as u32;
            text.push_str(&format!(
                "- **{}**: {} ({}% confident, used {}x)\n",
                rule.trigger_pattern, rule.correct_response, conf_pct, rule.hit_count
            ));
        }
        return Ok(Some(OutgoingMessage {
            chat_id: ctx.msg.chat_id,
            text, parse_mode: None, reply_to: None, platform: None, topic_id: None, interactive: None,
        }));
    }
}
"/forget all" => {
    if let Some(store) = &self.knowledge_store {
        let count = store.retract_all_user_rules(&ctx.msg.user_id).await.unwrap_or(0);
        return Ok(Some(OutgoingMessage {
            chat_id: ctx.msg.chat_id,
            text: format!("Done. Forgot {} learned items.", count),
            parse_mode: None, reply_to: None, platform: None, topic_id: None, interactive: None,
        }));
    }
}
"/teach" => {
    // /teach <fact> — explicitly teach the bot something
    let fact = args.trim(); // text after "/teach "
    if !fact.is_empty() {
        if let Some(store) = &self.knowledge_store {
            let correction = amanclaw_memory::knowledge_store::DetectedCorrection {
                trigger: fact.to_string(),
                wrong_response: None,
                correct_response: fact.to_string(),
                topic: None,
                confidence: 0.95,
                signal_type: "explicit_teach".into(),
            };
            let _ = store.upsert_rule(&correction, &ctx.msg.user_id, None, "user").await;
            return Ok(Some(OutgoingMessage {
                chat_id: ctx.msg.chat_id,
                text: "Got it, I'll remember that.".into(),
                parse_mode: None, reply_to: None, platform: None, topic_id: None, interactive: None,
            }));
        }
    }
}
```

Note: CommandMiddleware needs an `Option<Arc<KnowledgeStore>>` field, similar to PersistMiddleware. Add it with a `with_knowledge_store` builder method following the same pattern.

- [ ] **Step 3: Run compilation check**

Run: `cd rust && cargo check -p amanclaw-core 2>&1 | tail -10`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/src/middleware/command.rs
git commit -m "feat(rle): add /learned, /forget, /teach user commands"
```

---

## Chunk 6: Integration Test

### Task 9: End-to-end integration test

**Files:**
- Create: `rust/crates/amanclaw-core/tests/rle_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
//! Integration test for the Reactive Learning Engine.
//!
//! Tests the full flow: store a correction, query it, format it for LLM injection.

use amanclaw_memory::knowledge_store::{
    CorrectionEvent, CorrectionQuery, DetectedCorrection, KnowledgeStore,
};
use sqlx::sqlite::SqlitePoolOptions;

async fn make_store() -> KnowledgeStore {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(amanclaw_memory::schema::INIT_SQL)
        .execute(&pool)
        .await
        .unwrap();
    let store = KnowledgeStore::new(pool);
    store.init().await.unwrap();
    store
}

#[tokio::test]
async fn test_full_rle_flow() {
    let store = make_store().await;

    // 1. User corrects the bot: "No, Asr in KL is 4:15, not 4:30"
    let correction = DetectedCorrection {
        trigger: "Asr prayer time in KL".into(),
        wrong_response: Some("4:30 PM".into()),
        correct_response: "4:15 PM".into(),
        topic: Some("solat".into()),
        confidence: 0.90,
        signal_type: "direct_correction".into(),
    };

    let rule_id = store
        .upsert_rule(&correction, "aman", None, "user")
        .await
        .unwrap();

    // 2. Log the correction event
    let event = CorrectionEvent {
        rule_id: Some(rule_id),
        user_id: "aman".into(),
        platform: Some("telegram".into()),
        signal_type: "direct_correction".into(),
        signal_confidence: 0.90,
        source_user_msg: Some("No, Asr in KL is 4:15 not 4:30".into()),
        source_bot_msg: Some("Asr prayer in KL is at 4:30 PM".into()),
    };
    store.log_event(&event).await.unwrap();

    // 3. Next conversation: user asks about Asr again
    let matches = store
        .query(&CorrectionQuery {
            user_message: "What time is Asr prayer in KL?".into(),
            user_id: "aman".into(),
            community_id: None,
            topic: Some("solat".into()),
        })
        .await
        .unwrap();

    assert!(!matches.is_empty());
    assert_eq!(matches[0].rule.correct_response, "4:15 PM");
    assert_eq!(matches[0].rule.layer, "user");

    // 4. Format for LLM context injection
    let formatted = amanclaw_core::context_engine::format_learned_corrections(
        &matches.iter().map(|m| m.clone()).collect::<Vec<_>>(),
    );
    assert!(formatted.contains("4:15 PM"));
    assert!(formatted.contains("Learned knowledge"));

    // 5. Verify hit count was updated
    store.record_hit(rule_id).await.unwrap();
    let rules = store.get_user_rules("aman").await.unwrap();
    assert_eq!(rules[0].hit_count, 1);

    // 6. User says "forget all"
    let count = store.retract_all_user_rules("aman").await.unwrap();
    assert_eq!(count, 1);

    // 7. Query again — should return nothing
    let matches = store
        .query(&CorrectionQuery {
            user_message: "Asr time KL".into(),
            user_id: "aman".into(),
            community_id: None,
            topic: Some("solat".into()),
        })
        .await
        .unwrap();
    assert!(matches.is_empty());
}

#[tokio::test]
async fn test_community_knowledge_propagates() {
    let store = make_store().await;

    // Admin corrects for the whole community
    let correction = DetectedCorrection {
        trigger: "calculation method".into(),
        wrong_response: Some("Hanafi".into()),
        correct_response: "Shafi'i method is used here".into(),
        topic: Some("fiqh".into()),
        confidence: 0.95,
        signal_type: "explicit_teach".into(),
    };
    store
        .upsert_rule(&correction, "admin", Some("masjid-abc"), "community")
        .await
        .unwrap();

    // New user in the same community gets the knowledge
    let matches = store
        .query(&CorrectionQuery {
            user_message: "what calculation method do you use?".into(),
            user_id: "new_user".into(),
            community_id: Some("masjid-abc".into()),
            topic: Some("fiqh".into()),
        })
        .await
        .unwrap();

    assert!(!matches.is_empty());
    assert_eq!(matches[0].rule.correct_response, "Shafi'i method is used here");
    assert_eq!(matches[0].rule.layer, "community");
}

#[tokio::test]
async fn test_candidate_promotion_flow() {
    let store = make_store().await;

    // Low-confidence detection → candidate
    let correction = DetectedCorrection {
        trigger: "office hours".into(),
        wrong_response: None,
        correct_response: "9am to 5pm".into(),
        topic: None,
        confidence: 0.50, // below 0.7 → candidate
        signal_type: "rephrased_reask".into(),
    };
    let rule_id = store
        .upsert_rule(&correction, "user1", None, "user")
        .await
        .unwrap();

    // Should not appear in active queries
    let rules = store.get_user_rules("user1").await.unwrap();
    assert!(rules.is_empty()); // candidates excluded

    // Simulate confidence boost (from repeated corrections)
    sqlx::query("UPDATE correction_rules SET confidence = 0.75 WHERE id = ?")
        .bind(rule_id)
        .execute(&store.pool)
        .await
        .unwrap();

    // Promote
    let promoted = store.promote_candidates(0.70).await.unwrap();
    assert_eq!(promoted, 1);

    // Now it appears
    let rules = store.get_user_rules("user1").await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].status, "active");
}
```

- [ ] **Step 2: Run integration tests**

Run: `cd rust && cargo test -p amanclaw-core --test rle_integration 2>&1 | tail -15`
Expected: All 3 tests PASS.

- [ ] **Step 3: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -20`
Expected: All tests across all crates PASS.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-core/tests/rle_integration.rs
git commit -m "test(rle): add end-to-end integration tests for Reactive Learning Engine"
```

---

## Summary

| Chunk | Tasks | What it delivers |
|-------|-------|-----------------|
| 1 | Tasks 1-2 | Schema + KnowledgeStore with full CRUD |
| 2 | Task 3 | Correction detection engine (LLM-powered extraction + JSON parsing) |
| 3 | Tasks 4-5 | Two middleware: retrieve corrections + detect corrections |
| 4 | Tasks 6-7 | Wired into pipeline: corrections injected into LLM context, detected after exchange |
| 5 | Task 8 | User commands: /learned, /forget, /teach |
| 6 | Task 9 | End-to-end integration test |

After all chunks: the bot learns from corrections, stores them in a three-layer hierarchy, and silently applies them in future conversations. Users can inspect and manage learned knowledge.
