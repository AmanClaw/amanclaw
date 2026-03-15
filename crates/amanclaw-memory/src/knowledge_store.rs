use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

use crate::schema::RLE_INIT_SQL;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRule {
    pub id: i64,
    pub trigger_pattern: String,
    pub wrong_response: Option<String>,
    pub correct_response: String,
    pub topic: Option<String>,
    pub user_id: Option<String>,
    pub community_id: Option<String>,
    pub layer: String,
    pub confidence: f64,
    pub hit_count: i64,
    pub status: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedCorrection {
    pub trigger: String,
    pub wrong_response: Option<String>,
    pub correct_response: String,
    pub topic: Option<String>,
    pub confidence: f64,
    pub signal_type: String,
}

#[derive(Debug, Clone)]
pub struct CorrectionQuery {
    pub user_message: String,
    pub user_id: String,
    pub community_id: Option<String>,
    pub topic: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CorrectionMatch {
    pub rule: CorrectionRule,
    pub match_score: f64,
}

pub struct KnowledgeStore {
    pub pool: SqlitePool,
}

fn tokenize(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

fn layer_priority(layer: &str) -> u8 {
    match layer {
        "user" => 0,
        "community" => 1,
        _ => 2,
    }
}

impl KnowledgeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn init(&self) -> Result<()> {
        sqlx::raw_sql(RLE_INIT_SQL).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn upsert_rule(
        &self,
        correction: &DetectedCorrection,
        user_id: Option<&str>,
        community_id: Option<&str>,
        layer: &str,
    ) -> Result<i64> {
        // Check for existing rule with same trigger + layer + user
        let existing = if let Some(uid) = user_id {
            sqlx::query(
                "SELECT id, confidence FROM correction_rules WHERE trigger_pattern = ? AND layer = ? AND user_id = ? AND status != 'retracted'"
            )
            .bind(&correction.trigger)
            .bind(layer)
            .bind(uid)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, confidence FROM correction_rules WHERE trigger_pattern = ? AND layer = ? AND user_id IS NULL AND status != 'retracted'"
            )
            .bind(&correction.trigger)
            .bind(layer)
            .fetch_optional(&self.pool)
            .await?
        };

        if let Some(row) = existing {
            let id: i64 = row.get("id");
            let old_confidence: f64 = row.get("confidence");
            let new_confidence = if correction.confidence > old_confidence {
                correction.confidence
            } else {
                old_confidence
            };

            sqlx::query(
                "UPDATE correction_rules SET correct_response = ?, wrong_response = ?, topic = ?, confidence = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
            )
            .bind(&correction.correct_response)
            .bind(&correction.wrong_response)
            .bind(&correction.topic)
            .bind(new_confidence)
            .bind(id)
            .execute(&self.pool)
            .await?;

            Ok(id)
        } else {
            let row = sqlx::query(
                "INSERT INTO correction_rules (trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'candidate') RETURNING id"
            )
            .bind(&correction.trigger)
            .bind(&correction.wrong_response)
            .bind(&correction.correct_response)
            .bind(&correction.topic)
            .bind(user_id)
            .bind(community_id)
            .bind(layer)
            .bind(correction.confidence)
            .fetch_one(&self.pool)
            .await?;

            Ok(row.get("id"))
        }
    }

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

    pub async fn query(&self, q: &CorrectionQuery) -> Result<Vec<CorrectionMatch>> {
        let rows = sqlx::query(
            "SELECT id, trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, hit_count, status FROM correction_rules WHERE status = 'active' AND (user_id IS NULL OR user_id = ? OR community_id = ?)"
        )
        .bind(&q.user_id)
        .bind(q.community_id.as_deref().unwrap_or(""))
        .fetch_all(&self.pool)
        .await?;

        let msg_tokens = tokenize(&q.user_message);
        let mut matches = Vec::new();

        for row in &rows {
            let rule_layer: String = row.get("layer");
            let rule_user_id: Option<String> = row.get("user_id");
            let rule_community_id: Option<String> = row.get("community_id");

            // Filter by layer relevance
            match rule_layer.as_str() {
                "user" => {
                    if rule_user_id.as_deref() != Some(&q.user_id) {
                        continue;
                    }
                }
                "community" => {
                    if rule_community_id.as_deref() != q.community_id.as_deref() {
                        continue;
                    }
                }
                _ => {} // global applies to everyone
            }

            let trigger: String = row.get("trigger_pattern");
            let trigger_tokens = tokenize(&trigger);

            if trigger_tokens.is_empty() {
                continue;
            }

            let overlap = msg_tokens.intersection(&trigger_tokens).count() as f64;
            let overlap_score = overlap / trigger_tokens.len() as f64;

            if overlap_score < 0.3 {
                continue;
            }

            // Bonus for topic match
            let topic_bonus = if let (Some(ref rule_topic), Some(q_topic)) =
                (row.get::<Option<String>, _>("topic"), &q.topic)
            {
                if rule_topic == q_topic { 0.1 } else { 0.0 }
            } else {
                0.0
            };

            let confidence: f64 = row.get("confidence");
            let match_score = (overlap_score + topic_bonus) * confidence;

            let rule = CorrectionRule {
                id: row.get("id"),
                trigger_pattern: trigger,
                wrong_response: row.get("wrong_response"),
                correct_response: row.get("correct_response"),
                topic: row.get("topic"),
                user_id: rule_user_id,
                community_id: rule_community_id,
                layer: rule_layer.clone(),
                confidence,
                hit_count: row.get("hit_count"),
                status: row.get("status"),
            };

            matches.push(CorrectionMatch { rule, match_score });
        }

        // Sort by layer priority (user=0, community=1, global=2) then by match_score desc
        matches.sort_by(|a, b| {
            let pa = layer_priority(&a.rule.layer);
            let pb = layer_priority(&b.rule.layer);
            pa.cmp(&pb)
                .then_with(|| b.match_score.partial_cmp(&a.match_score).unwrap())
        });

        Ok(matches)
    }

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
            "UPDATE correction_rules SET status = 'retracted', updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(rule_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_user_rules(&self, user_id: &str) -> Result<Vec<CorrectionRule>> {
        let rows = sqlx::query(
            "SELECT id, trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, hit_count, status FROM correction_rules WHERE user_id = ? AND status = 'active'"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| CorrectionRule {
                id: row.get("id"),
                trigger_pattern: row.get("trigger_pattern"),
                wrong_response: row.get("wrong_response"),
                correct_response: row.get("correct_response"),
                topic: row.get("topic"),
                user_id: row.get("user_id"),
                community_id: row.get("community_id"),
                layer: row.get("layer"),
                confidence: row.get("confidence"),
                hit_count: row.get("hit_count"),
                status: row.get("status"),
            })
            .collect())
    }

    pub async fn get_community_rules(&self, community_id: &str) -> Result<Vec<CorrectionRule>> {
        let rows = sqlx::query(
            "SELECT id, trigger_pattern, wrong_response, correct_response, topic, user_id, community_id, layer, confidence, hit_count, status FROM correction_rules WHERE community_id = ? AND status = 'active'"
        )
        .bind(community_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| CorrectionRule {
                id: row.get("id"),
                trigger_pattern: row.get("trigger_pattern"),
                wrong_response: row.get("wrong_response"),
                correct_response: row.get("correct_response"),
                topic: row.get("topic"),
                user_id: row.get("user_id"),
                community_id: row.get("community_id"),
                layer: row.get("layer"),
                confidence: row.get("confidence"),
                hit_count: row.get("hit_count"),
                status: row.get("status"),
            })
            .collect())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn make_store() -> KnowledgeStore {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let store = KnowledgeStore::new(pool);
        store.init().await.unwrap();
        store
    }

    fn sample_correction(trigger: &str, correct: &str) -> DetectedCorrection {
        DetectedCorrection {
            trigger: trigger.to_string(),
            wrong_response: Some("wrong answer".to_string()),
            correct_response: correct.to_string(),
            topic: Some("solat".to_string()),
            confidence: 0.8,
            signal_type: "explicit_correction".to_string(),
        }
    }

    #[tokio::test]
    async fn upsert_and_query_user_rule() {
        let store = make_store().await;
        let correction = sample_correction("what time is subuh", "Subuh is at 5:45 AM");

        let id = store
            .upsert_rule(&correction, Some("u1"), None, "user")
            .await
            .unwrap();
        assert!(id > 0);

        // Promote so it's active
        store.promote_candidates(0.5).await.unwrap();

        let q = CorrectionQuery {
            user_message: "what time is subuh prayer".to_string(),
            user_id: "u1".to_string(),
            community_id: None,
            topic: Some("solat".to_string()),
        };

        let matches = store.query(&q).await.unwrap();
        assert!(!matches.is_empty());
        assert_eq!(matches[0].rule.correct_response, "Subuh is at 5:45 AM");
    }

    #[tokio::test]
    async fn layer_priority_user_over_global() {
        let store = make_store().await;

        let global = DetectedCorrection {
            trigger: "subuh time".to_string(),
            wrong_response: None,
            correct_response: "Global: 5:30 AM".to_string(),
            topic: None,
            confidence: 0.9,
            signal_type: "explicit".to_string(),
        };
        let user = DetectedCorrection {
            trigger: "subuh time".to_string(),
            wrong_response: None,
            correct_response: "User: 5:45 AM".to_string(),
            topic: None,
            confidence: 0.9,
            signal_type: "explicit".to_string(),
        };

        store
            .upsert_rule(&global, None, None, "global")
            .await
            .unwrap();
        store
            .upsert_rule(&user, Some("u1"), None, "user")
            .await
            .unwrap();
        store.promote_candidates(0.5).await.unwrap();

        let q = CorrectionQuery {
            user_message: "subuh time".to_string(),
            user_id: "u1".to_string(),
            community_id: None,
            topic: None,
        };

        let matches = store.query(&q).await.unwrap();
        assert!(matches.len() >= 2);
        assert_eq!(matches[0].rule.layer, "user");
        assert_eq!(matches[1].rule.layer, "global");
    }

    #[tokio::test]
    async fn record_hit_increments_count() {
        let store = make_store().await;
        let correction = sample_correction("test trigger", "correct answer");
        let id = store
            .upsert_rule(&correction, Some("u1"), None, "user")
            .await
            .unwrap();

        store.promote_candidates(0.5).await.unwrap();
        store.record_hit(id).await.unwrap();
        store.record_hit(id).await.unwrap();

        let rules = store.get_user_rules("u1").await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].hit_count, 2);
    }

    #[tokio::test]
    async fn retract_rule() {
        let store = make_store().await;
        let correction = sample_correction("test trigger", "correct answer");
        let id = store
            .upsert_rule(&correction, Some("u1"), None, "user")
            .await
            .unwrap();
        store.promote_candidates(0.5).await.unwrap();

        let retracted = store.retract_rule(id).await.unwrap();
        assert!(retracted);

        let rules = store.get_user_rules("u1").await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn retract_all_user_rules() {
        let store = make_store().await;

        for i in 0..3 {
            let c = sample_correction(&format!("trigger {i}"), &format!("answer {i}"));
            store
                .upsert_rule(&c, Some("u1"), None, "user")
                .await
                .unwrap();
        }
        store.promote_candidates(0.5).await.unwrap();

        let count = store.retract_all_user_rules("u1").await.unwrap();
        assert_eq!(count, 3);

        let rules = store.get_user_rules("u1").await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn log_event() {
        let store = make_store().await;
        let event = CorrectionEvent {
            rule_id: None,
            user_id: "u1".to_string(),
            platform: Some("telegram".to_string()),
            signal_type: "explicit_correction".to_string(),
            signal_confidence: 0.9,
            source_user_msg: Some("No, the answer is X".to_string()),
            source_bot_msg: Some("The answer is Y".to_string()),
        };

        // Should not error
        store.log_event(&event).await.unwrap();

        // Verify the event was stored
        let row = sqlx::query("SELECT COUNT(*) as count FROM correction_events")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let count: i64 = row.get("count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn upsert_updates_existing_rule() {
        let store = make_store().await;
        let c1 = DetectedCorrection {
            trigger: "solat time".to_string(),
            wrong_response: Some("old wrong".to_string()),
            correct_response: "old correct".to_string(),
            topic: Some("solat".to_string()),
            confidence: 0.7,
            signal_type: "explicit".to_string(),
        };

        let id1 = store
            .upsert_rule(&c1, Some("u1"), None, "user")
            .await
            .unwrap();

        let c2 = DetectedCorrection {
            trigger: "solat time".to_string(),
            wrong_response: Some("new wrong".to_string()),
            correct_response: "new correct".to_string(),
            topic: Some("solat".to_string()),
            confidence: 0.9,
            signal_type: "explicit".to_string(),
        };

        let id2 = store
            .upsert_rule(&c2, Some("u1"), None, "user")
            .await
            .unwrap();

        // Should be the same row
        assert_eq!(id1, id2);

        // Promote and verify updated values
        store.promote_candidates(0.5).await.unwrap();
        let rules = store.get_user_rules("u1").await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].correct_response, "new correct");
        assert!((rules[0].confidence - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn promote_candidates() {
        let store = make_store().await;

        let low = DetectedCorrection {
            trigger: "low confidence".to_string(),
            wrong_response: None,
            correct_response: "low".to_string(),
            topic: None,
            confidence: 0.3,
            signal_type: "implicit".to_string(),
        };
        let high = DetectedCorrection {
            trigger: "high confidence".to_string(),
            wrong_response: None,
            correct_response: "high".to_string(),
            topic: None,
            confidence: 0.9,
            signal_type: "explicit".to_string(),
        };

        store
            .upsert_rule(&low, Some("u1"), None, "user")
            .await
            .unwrap();
        store
            .upsert_rule(&high, Some("u1"), None, "user")
            .await
            .unwrap();

        // Promote only >= 0.7
        let promoted = store.promote_candidates(0.7).await.unwrap();
        assert_eq!(promoted, 1);

        let rules = store.get_user_rules("u1").await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].correct_response, "high");
    }
}
