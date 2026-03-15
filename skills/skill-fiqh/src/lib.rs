use std::sync::Arc;

use amanclaw_islamic_db::fiqh::FiqhRuling;
use amanclaw_islamic_db::IslamicDb;
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};

pub struct FiqhSkill {
    db: Arc<IslamicDb>,
}

impl FiqhSkill {
    pub fn new(db: Arc<IslamicDb>) -> Self {
        Self { db }
    }
}

const DISCLAIMER: &str =
    "\u{26a0}\u{fe0f} This is a summary of scholarly positions. For personal rulings, consult a qualified scholar.";

/// Human-readable madhab name.
fn madhab_display_name(slug: &str) -> &str {
    match slug {
        "shafii" => "Shafi'i",
        "hanafi" => "Hanafi",
        "maliki" => "Maliki",
        "hanbali" => "Hanbali",
        _ => slug,
    }
}

/// Validate the madhab parameter. Returns `None` for "all" or missing.
fn parse_madhab(args: &serde_json::Value) -> Result<Option<&str>, String> {
    match args.get("madhab").and_then(|v| v.as_str()) {
        Some("all") | None => Ok(None),
        Some(m) => {
            const VALID: &[&str] = &["shafii", "hanafi", "maliki", "hanbali"];
            if VALID.contains(&m) {
                Ok(Some(m))
            } else {
                Err(format!(
                    "Invalid madhab '{}'. Valid: shafii, hanafi, maliki, hanbali, all",
                    m
                ))
            }
        }
    }
}

/// Format a single fiqh ruling for display.
fn format_ruling(r: &FiqhRuling) -> String {
    format!(
        "\u{1f539} {}: {} [Source: {}]",
        madhab_display_name(&r.madhab),
        r.ruling,
        r.source,
    )
}

/// Format the evidence section from Quran and Hadith search results.
fn format_evidence(
    quran_results: &[amanclaw_islamic_db::quran::QuranVerse],
    hadith_results: &[amanclaw_islamic_db::hadith::HadithEntry],
) -> String {
    if quran_results.is_empty() && hadith_results.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\u{1f4d6} Evidence:\n");
    for v in quran_results {
        let translation = if !v.translation_en.is_empty() {
            &v.translation_en
        } else {
            &v.translation_ms
        };
        out.push_str(&format!(
            "- Quran {}:{} \u{2014} \"{}\"\n",
            v.surah, v.ayat, translation,
        ));
    }
    for h in hadith_results {
        let collection_name = match h.collection.as_str() {
            "bukhari" => "Sahih Bukhari",
            "muslim" => "Sahih Muslim",
            "abudawud" => "Sunan Abu Dawud",
            "tirmidhi" => "Jami` at-Tirmidhi",
            "nasai" => "Sunan an-Nasa'i",
            "ibnmajah" => "Sunan Ibn Majah",
            other => other,
        };
        let text = if !h.text_en.is_empty() {
            &h.text_en
        } else {
            &h.text_ar
        };
        out.push_str(&format!(
            "- {} #{} \u{2014} \"{}\"\n",
            collection_name, h.hadith_number, text,
        ));
    }
    out
}

#[async_trait::async_trait]
impl Skill for FiqhSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "fiqh".into(),
            description: "Islamic jurisprudence resolver with multi-madhab support. Ask questions about Islamic rulings and get answers from Shafi'i, Hanafi, Maliki, and Hanbali perspectives with Quran and Hadith citations.".into(),
            timeout_ms: 15000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["ask", "browse", "topics"],
                    "description": "Action to perform. ask = search fiqh rulings with RAG evidence, browse = list rulings by topic, topics = list available topics."
                },
                "question": {
                    "type": "string",
                    "description": "The fiqh question to search for (action=ask)"
                },
                "madhab": {
                    "type": "string",
                    "enum": ["shafii", "hanafi", "maliki", "hanbali", "all"],
                    "description": "Filter by madhab. Default: all"
                },
                "topic": {
                    "type": "string",
                    "description": "Topic name for action=browse (e.g. 'prayer', 'fasting')"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        let args: serde_json::Value = match serde_json::from_str(&input.args) {
            Ok(v) => v,
            Err(e) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid args: {e}")),
                };
            }
        };

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("ask");

        let pool = self.db.pool();

        match action {
            "ask" => {
                let question = match args.get("question").and_then(|v| v.as_str()) {
                    Some(q) if !q.is_empty() => q,
                    _ => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Question is required for ask action. Provide 'question' parameter."
                                    .into(),
                            ),
                        };
                    }
                };

                let madhab = match parse_madhab(&args) {
                    Ok(m) => m,
                    Err(e) => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(e),
                        };
                    }
                };

                // 1. Search fiqh rulings
                let fiqh_results =
                    match amanclaw_islamic_db::fiqh::search(pool, question, madhab, 20).await {
                        Ok(r) => r,
                        Err(e) => {
                            return SkillResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Fiqh search error: {e}")),
                            };
                        }
                    };

                if fiqh_results.is_empty() {
                    return SkillResult {
                        success: true,
                        output: format!(
                            "No fiqh rulings found for '{}'. This topic may not be covered in the database yet. Try browsing available topics with action=topics.\n\n{}",
                            question, DISCLAIMER,
                        ),
                        error: None,
                    };
                }

                // 2. Search Quran for evidence (use question as search term)
                let quran_results =
                    amanclaw_islamic_db::quran::search(pool, question, 3)
                        .await
                        .unwrap_or_default();

                // 3. Search Hadith for evidence
                let hadith_results =
                    amanclaw_islamic_db::hadith::search(pool, question, None, None, 3)
                        .await
                        .unwrap_or_default();

                // 4. Format output
                let topic_summary = fiqh_results
                    .first()
                    .map(|r| {
                        if r.subtopic.is_empty() {
                            r.topic.clone()
                        } else {
                            format!("{} - {}", r.topic, r.subtopic)
                        }
                    })
                    .unwrap_or_else(|| question.to_string());

                let mut output = format!("\u{1f4da} {}\n\n", topic_summary);

                for r in &fiqh_results {
                    output.push_str(&format_ruling(r));
                    output.push('\n');
                }

                let evidence = format_evidence(&quran_results, &hadith_results);
                if !evidence.is_empty() {
                    output.push_str(&evidence);
                }

                output.push('\n');
                output.push_str(DISCLAIMER);

                SkillResult {
                    success: true,
                    output,
                    error: None,
                }
            }

            "browse" => {
                let topic = match args.get("topic").and_then(|v| v.as_str()) {
                    Some(t) if !t.is_empty() => t,
                    _ => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Topic is required for browse action. Provide 'topic' parameter."
                                    .into(),
                            ),
                        };
                    }
                };

                match amanclaw_islamic_db::fiqh::by_topic(pool, topic).await {
                    Ok(results) if results.is_empty() => SkillResult {
                        success: true,
                        output: format!(
                            "No rulings found for topic '{}'. Try listing available topics with action=topics.",
                            topic,
                        ),
                        error: None,
                    },
                    Ok(results) => {
                        let mut output = format!(
                            "\u{1f4da} {} ({} rulings):\n\n",
                            topic,
                            results.len(),
                        );
                        for r in &results {
                            output.push_str(&format_ruling(r));
                            output.push('\n');
                        }
                        SkillResult {
                            success: true,
                            output,
                            error: None,
                        }
                    }
                    Err(e) => SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Browse error: {e}")),
                    },
                }
            }

            "topics" => match amanclaw_islamic_db::fiqh::list_topics(pool).await {
                Ok(topics) if topics.is_empty() => SkillResult {
                    success: true,
                    output:
                        "No fiqh topics available yet. The database may need syncing \u{2014} run the sync command to populate it."
                            .into(),
                    error: None,
                },
                Ok(topics) => {
                    let mut output = format!(
                        "\u{1f4da} Available fiqh topics ({}):\n\n",
                        topics.len(),
                    );
                    for t in &topics {
                        output.push_str(&format!("- {}\n", t));
                    }
                    SkillResult {
                        success: true,
                        output,
                        error: None,
                    }
                }
                Err(e) => SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Topics error: {e}")),
                },
            },

            other => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Unknown action '{}'. Valid actions: ask, browse, topics",
                    other,
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Arc<IslamicDb> {
        let db = IslamicDb::new(":memory:").await.unwrap();
        let pool = db.pool();

        // Insert test fiqh rulings
        for (madhab, ruling) in [
            (
                "shafii",
                "Permissible during travel of approximately 82km or more",
            ),
            (
                "hanafi",
                "Only permitted at Arafah and Muzdalifah during Hajj",
            ),
            (
                "maliki",
                "Permissible during travel, also for rain and illness",
            ),
            (
                "hanbali",
                "Permissible during travel, illness, rain, and genuine hardship",
            ),
        ] {
            sqlx::query(
                "INSERT INTO fiqh_rulings (topic, subtopic, madhab, ruling, evidence, source, language) VALUES ('prayer', 'combining prayers', ?, ?, 'Quran 4:101; Muslim 705', 'Classical fiqh texts', 'en')",
            )
            .bind(madhab)
            .bind(ruling)
            .execute(pool)
            .await
            .unwrap();
        }

        // Populate fiqh FTS
        sqlx::query(
            "INSERT INTO fiqh_fts(rowid, topic, subtopic, ruling, evidence) SELECT rowid, topic, subtopic, ruling, evidence FROM fiqh_rulings",
        )
        .execute(pool)
        .await
        .unwrap();

        // Insert test Quran ayat (text includes "combining prayers" for FTS matching)
        sqlx::query(
            "INSERT INTO quran_ayat (surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page) VALUES (4, 101, '', '', '', 'Evidence for combining prayers when you travel throughout the land there is no blame upon you', 5, 10, 94)",
        )
        .execute(pool)
        .await
        .unwrap();

        // Populate quran FTS
        sqlx::query(
            "INSERT INTO quran_fts(rowid, surah, ayat, text, translation_ms, translation_en) SELECT rowid, surah, ayat, text_uthmani, translation_ms, translation_en FROM quran_ayat",
        )
        .execute(pool)
        .await
        .unwrap();

        // Insert test hadith (text includes "combining prayers" for FTS matching)
        sqlx::query(
            "INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('muslim', 1, 705, '', 'Hadith on combining prayers during travel from the Prophet', 'sahih', 'darussalam', 'Prayer')",
        )
        .execute(pool)
        .await
        .unwrap();

        // Populate hadith FTS
        sqlx::query(
            "INSERT INTO hadith_fts(rowid, collection, hadith_number, text_ar, text_en, chapter) SELECT rowid, collection, hadith_number, text_ar, text_en, chapter FROM hadith",
        )
        .execute(pool)
        .await
        .unwrap();

        Arc::new(db)
    }

    fn make_input(args: &str) -> SkillInput {
        SkillInput {
            name: "fiqh".into(),
            args: args.into(),
            user_id: "test".into(),
            platform: "test".into(),
        }
    }

    #[test]
    fn test_metadata() {
        let db = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { IslamicDb::new(":memory:").await.unwrap() });
        let skill = FiqhSkill::new(Arc::new(db));
        let meta = skill.metadata();
        assert_eq!(meta.name, "fiqh");
        assert_eq!(meta.version, "0.1.0");
        assert_eq!(meta.timeout_ms, 15000);
    }

    #[test]
    fn test_parameters_schema() {
        let db = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { IslamicDb::new(":memory:").await.unwrap() });
        let skill = FiqhSkill::new(Arc::new(db));
        let schema = skill.parameters_schema();
        let actions = &schema["properties"]["action"]["enum"];
        assert!(actions.as_array().unwrap().iter().any(|v| v == "ask"));
        assert!(actions.as_array().unwrap().iter().any(|v| v == "browse"));
        assert!(actions.as_array().unwrap().iter().any(|v| v == "topics"));
        assert!(schema["properties"]["question"].is_object());
        assert!(schema["properties"]["madhab"].is_object());
        assert!(schema["properties"]["topic"].is_object());
    }

    #[tokio::test]
    async fn test_topics() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "topics"}"#))
            .await;
        assert!(result.success);
        assert!(result.output.contains("prayer"));
        assert!(result.output.contains("Available fiqh topics"));
    }

    #[tokio::test]
    async fn test_browse_prayer() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "browse", "topic": "prayer"}"#))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Shafi'i"));
        assert!(result.output.contains("Hanafi"));
        assert!(result.output.contains("Maliki"));
        assert!(result.output.contains("Hanbali"));
        assert!(result.output.contains("4 rulings"));
    }

    #[tokio::test]
    async fn test_ask_combining_prayers() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "ask", "question": "combining prayers"}"#,
            ))
            .await;
        assert!(result.success);
        // Multi-madhab labels
        assert!(result.output.contains("Shafi'i"));
        assert!(result.output.contains("Hanafi"));
        assert!(result.output.contains("Maliki"));
        assert!(result.output.contains("Hanbali"));
        // Evidence section
        assert!(result.output.contains("Evidence"));
        // Disclaimer
        assert!(result.output.contains(DISCLAIMER));
    }

    #[tokio::test]
    async fn test_ask_with_madhab_filter() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "ask", "question": "combining prayers", "madhab": "shafii"}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Shafi'i"));
        assert!(result.output.contains("82km"));
        // Should NOT contain other madhab rulings
        assert!(!result.output.contains("Hanafi:"));
        assert!(result.output.contains(DISCLAIMER));
    }

    #[tokio::test]
    async fn test_ask_empty_db() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        let skill = FiqhSkill::new(Arc::new(db));
        let result = skill
            .execute(make_input(
                r#"{"action": "ask", "question": "something unknown"}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("No fiqh rulings found"));
        assert!(result.output.contains("topics"));
        assert!(result.output.contains(DISCLAIMER));
    }

    #[tokio::test]
    async fn test_topics_empty_db() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        let skill = FiqhSkill::new(Arc::new(db));
        let result = skill
            .execute(make_input(r#"{"action": "topics"}"#))
            .await;
        assert!(result.success);
        assert!(result.output.contains("No fiqh topics available"));
    }

    #[tokio::test]
    async fn test_browse_missing_topic() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "browse"}"#))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Topic is required"));
    }

    #[tokio::test]
    async fn test_ask_missing_question() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill.execute(make_input(r#"{"action": "ask"}"#)).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Question is required"));
    }

    #[tokio::test]
    async fn test_invalid_action() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "delete"}"#))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown action"));
    }

    #[tokio::test]
    async fn test_invalid_args() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill.execute(make_input("not json")).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid args"));
    }

    #[tokio::test]
    async fn test_invalid_madhab() {
        let db = setup_db().await;
        let skill = FiqhSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "ask", "question": "prayer", "madhab": "zahiri"}"#,
            ))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid madhab"));
    }
}
