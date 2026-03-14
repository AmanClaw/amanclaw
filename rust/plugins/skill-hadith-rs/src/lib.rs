use std::sync::Arc;

use amanclaw_islamic_db::hadith::HadithEntry;
use amanclaw_islamic_db::IslamicDb;
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};

pub struct HadithSkill {
    db: Arc<IslamicDb>,
}

impl HadithSkill {
    pub fn new(db: Arc<IslamicDb>) -> Self {
        Self { db }
    }
}

/// Human-readable collection name.
fn collection_display_name(slug: &str) -> &str {
    match slug {
        "bukhari" => "Sahih al-Bukhari",
        "muslim" => "Sahih Muslim",
        "abudawud" => "Sunan Abu Dawud",
        "tirmidhi" => "Jami` at-Tirmidhi",
        "nasai" => "Sunan an-Nasa'i",
        "ibnmajah" => "Sunan Ibn Majah",
        other => other,
    }
}

/// Format a single hadith entry for display.
fn format_hadith(h: &HadithEntry) -> String {
    let collection_name = collection_display_name(&h.collection);
    let grade_display = if h.graded_by.is_empty() {
        h.grade.clone()
    } else {
        format!("{} ({})", h.grade, h.graded_by)
    };

    let mut out = format!(
        "\u{1f4d6} {} #{}\nGrade: {}\nChapter: {}\n",
        collection_name, h.hadith_number, grade_display, h.chapter,
    );

    if !h.text_ar.is_empty() {
        out.push('\n');
        out.push_str(&h.text_ar);
        out.push('\n');
    }

    if !h.text_en.is_empty() {
        out.push('\n');
        out.push_str(&h.text_en);
        out.push('\n');
    }

    out
}

/// Validate the collection parameter. Returns `None` for "all" or missing.
fn parse_collection(args: &serde_json::Value) -> Result<Option<&str>, String> {
    match args.get("collection").and_then(|v| v.as_str()) {
        Some("all") | None => Ok(None),
        Some(c) => {
            const VALID: &[&str] = &[
                "bukhari", "muslim", "abudawud", "tirmidhi", "nasai", "ibnmajah",
            ];
            if VALID.contains(&c) {
                Ok(Some(c))
            } else {
                Err(format!(
                    "Invalid collection '{}'. Valid: bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah, all",
                    c
                ))
            }
        }
    }
}

/// Validate the grade parameter. Returns `None` for "all" or missing.
fn parse_grade(args: &serde_json::Value) -> Result<Option<&str>, String> {
    match args.get("grade").and_then(|v| v.as_str()) {
        Some("all") | None => Ok(None),
        Some(g) => {
            const VALID: &[&str] = &["sahih", "hasan", "daif"];
            if VALID.contains(&g) {
                Ok(Some(g))
            } else {
                Err(format!(
                    "Invalid grade '{}'. Valid: sahih, hasan, daif, all",
                    g
                ))
            }
        }
    }
}

#[async_trait::async_trait]
impl Skill for HadithSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "hadith".into(),
            description: "Search and browse hadith collections with isnad grading. Supports Bukhari, Muslim, Abu Dawud, Tirmidhi, Nasa'i, and Ibn Majah.".into(),
            timeout_ms: 10000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "lookup", "browse"],
                    "description": "Action to perform. search = full-text search, lookup = specific hadith by collection+number, browse = list hadith by collection/book."
                },
                "query": {
                    "type": "string",
                    "description": "Search keyword(s) for action=search"
                },
                "collection": {
                    "type": "string",
                    "enum": ["bukhari", "muslim", "abudawud", "tirmidhi", "nasai", "ibnmajah", "all"],
                    "description": "Hadith collection to query. Default: all"
                },
                "grade": {
                    "type": "string",
                    "enum": ["sahih", "hasan", "daif", "all"],
                    "description": "Filter by grade. Default: all"
                },
                "hadith_number": {
                    "type": "integer",
                    "description": "Hadith number for action=lookup"
                },
                "book": {
                    "type": "integer",
                    "description": "Book number for action=browse"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results to return (default 5, max 20)"
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
            .unwrap_or("search");

        let limit = args
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .clamp(1, 20);

        let pool = self.db.pool();

        match action {
            "lookup" => {
                let collection = match args.get("collection").and_then(|v| v.as_str()) {
                    Some("all") | None => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Collection is required for lookup. Specify one of: bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah".into(),
                            ),
                        };
                    }
                    Some(c) => c,
                };

                let hadith_number = match args.get("hadith_number").and_then(|v| v.as_i64()) {
                    Some(n) if n >= 1 => n,
                    Some(_) => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some("hadith_number must be at least 1.".into()),
                        };
                    }
                    None => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some("hadith_number is required for lookup.".into()),
                        };
                    }
                };

                match amanclaw_islamic_db::hadith::lookup(pool, collection, hadith_number).await {
                    Ok(Some(h)) => SkillResult {
                        success: true,
                        output: format_hadith(&h),
                        error: None,
                    },
                    Ok(None) => SkillResult {
                        success: true,
                        output: format!(
                            "No hadith found: {} #{}. The hadith database may need syncing — run the sync command to populate it.",
                            collection_display_name(collection),
                            hadith_number
                        ),
                        error: None,
                    },
                    Err(e) => SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Lookup error: {e}")),
                    },
                }
            }

            "search" => {
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(q) if !q.is_empty() => q,
                    _ => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Search query is required. Provide 'query' parameter.".into(),
                            ),
                        };
                    }
                };

                let collection = match parse_collection(&args) {
                    Ok(c) => c,
                    Err(e) => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(e),
                        };
                    }
                };

                let grade = match parse_grade(&args) {
                    Ok(g) => g,
                    Err(e) => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(e),
                        };
                    }
                };

                match amanclaw_islamic_db::hadith::search(pool, query, collection, grade, limit)
                    .await
                {
                    Ok(results) if results.is_empty() => SkillResult {
                        success: true,
                        output: format!(
                            "No hadith found for '{}'. The hadith database may need syncing — run the sync command to populate it.",
                            query
                        ),
                        error: None,
                    },
                    Ok(results) => {
                        let mut output = format!(
                            "Found {} hadith for '{}':\n\n",
                            results.len(),
                            query
                        );
                        for h in &results {
                            output.push_str(&format_hadith(h));
                            output.push_str("---\n");
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
                        error: Some(format!("Search error: {e}")),
                    },
                }
            }

            "browse" => {
                let collection = match args.get("collection").and_then(|v| v.as_str()) {
                    Some("all") | None => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Collection is required for browse. Specify one of: bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah".into(),
                            ),
                        };
                    }
                    Some(c) => c,
                };

                let book = args.get("book").and_then(|v| v.as_i64());

                match amanclaw_islamic_db::hadith::browse(pool, collection, book, limit).await {
                    Ok(results) if results.is_empty() => {
                        let msg = if let Some(b) = book {
                            format!(
                                "No hadith found in {} book {}. The hadith database may need syncing — run the sync command to populate it.",
                                collection_display_name(collection),
                                b
                            )
                        } else {
                            format!(
                                "No hadith found in {}. The hadith database may need syncing — run the sync command to populate it.",
                                collection_display_name(collection)
                            )
                        };
                        SkillResult {
                            success: true,
                            output: msg,
                            error: None,
                        }
                    }
                    Ok(results) => {
                        let header = if let Some(b) = book {
                            format!(
                                "{} - Book {} ({} hadith):\n\n",
                                collection_display_name(collection),
                                b,
                                results.len()
                            )
                        } else {
                            format!(
                                "{} ({} hadith):\n\n",
                                collection_display_name(collection),
                                results.len()
                            )
                        };
                        let mut output = header;
                        for h in &results {
                            output.push_str(&format_hadith(h));
                            output.push_str("---\n");
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

            other => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Unknown action '{}'. Valid actions: search, lookup, browse",
                    other
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

        sqlx::query("INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('bukhari', 1, 1, '\u{0625}\u{0646}\u{0645}\u{0627} \u{0627}\u{0644}\u{0623}\u{0639}\u{0645}\u{0627}\u{0644} \u{0628}\u{0627}\u{0644}\u{0646}\u{064a}\u{0627}\u{062a}', 'Actions are judged by intentions', 'sahih', 'al-albani', 'Revelation')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('muslim', 1, 1, '\u{0628}\u{0646}\u{064a} \u{0627}\u{0644}\u{0625}\u{0633}\u{0644}\u{0627}\u{0645} \u{0639}\u{0644}\u{0649} \u{062e}\u{0645}\u{0633}', 'Islam is built on five pillars', 'sahih', 'darussalam', 'Faith')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO hadith (collection, book_number, hadith_number, text_ar, text_en, grade, graded_by, chapter) VALUES ('tirmidhi', 1, 100, '\u{0645}\u{0646} \u{062d}\u{0633}\u{0646} \u{0625}\u{0633}\u{0644}\u{0627}\u{0645} \u{0627}\u{0644}\u{0645}\u{0631}\u{0621}', 'Part of good Islam is leaving what does not concern', 'hasan', 'al-albani', 'Faith')")
            .execute(pool).await.unwrap();

        sqlx::query("INSERT INTO hadith_fts(rowid, collection, hadith_number, text_ar, text_en, chapter) SELECT rowid, collection, hadith_number, text_ar, text_en, chapter FROM hadith")
            .execute(pool).await.unwrap();

        Arc::new(db)
    }

    fn make_input(args: &str) -> SkillInput {
        SkillInput {
            name: "hadith".into(),
            args: args.into(),
            user_id: "test".into(),
            platform: "test".into(),
        }
    }

    #[test]
    fn test_metadata() {
        let db = tokio::runtime::Runtime::new().unwrap().block_on(async {
            IslamicDb::new(":memory:").await.unwrap()
        });
        let skill = HadithSkill::new(Arc::new(db));
        let meta = skill.metadata();
        assert_eq!(meta.name, "hadith");
        assert_eq!(meta.version, "0.1.0");
    }

    #[test]
    fn test_parameters_schema() {
        let db = tokio::runtime::Runtime::new().unwrap().block_on(async {
            IslamicDb::new(":memory:").await.unwrap()
        });
        let skill = HadithSkill::new(Arc::new(db));
        let schema = skill.parameters_schema();
        let actions = &schema["properties"]["action"]["enum"];
        assert!(actions.as_array().unwrap().iter().any(|v| v == "search"));
        assert!(actions.as_array().unwrap().iter().any(|v| v == "lookup"));
        assert!(actions.as_array().unwrap().iter().any(|v| v == "browse"));
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["collection"].is_object());
        assert!(schema["properties"]["grade"].is_object());
        assert!(schema["properties"]["hadith_number"].is_object());
        assert!(schema["properties"]["book"].is_object());
        assert!(schema["properties"]["limit"].is_object());
    }

    #[tokio::test]
    async fn test_lookup() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "lookup", "collection": "bukhari", "hadith_number": 1}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Sahih al-Bukhari"));
        assert!(result.output.contains("#1"));
        assert!(result.output.contains("intentions"));
        assert!(result.output.contains("sahih"));
    }

    #[tokio::test]
    async fn test_lookup_not_found() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "lookup", "collection": "bukhari", "hadith_number": 9999}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("No hadith found"));
        assert!(result.output.contains("sync"));
    }

    #[tokio::test]
    async fn test_lookup_missing_collection() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "lookup", "hadith_number": 1}"#,
            ))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Collection is required"));
    }

    #[tokio::test]
    async fn test_lookup_missing_number() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "lookup", "collection": "bukhari"}"#,
            ))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("hadith_number is required"));
    }

    #[tokio::test]
    async fn test_search() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "search", "query": "Islam"}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Found"));
        assert!(result.output.contains("Islam"));
    }

    #[tokio::test]
    async fn test_search_with_collection_filter() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "search", "query": "Islam", "collection": "muslim"}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Sahih Muslim"));
    }

    #[tokio::test]
    async fn test_search_with_grade_filter() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "search", "query": "Islam", "grade": "hasan"}"#,
            ))
            .await;
        assert!(result.success);
        // Should find the tirmidhi hadith (hasan) but not bukhari/muslim (sahih)
        assert!(result.output.contains("Tirmidhi"));
    }

    #[tokio::test]
    async fn test_search_missing_query() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "search"}"#))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Search query is required"));
    }

    #[tokio::test]
    async fn test_browse() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "browse", "collection": "bukhari"}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Sahih al-Bukhari"));
        assert!(result.output.contains("intentions"));
    }

    #[tokio::test]
    async fn test_browse_with_book() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "browse", "collection": "bukhari", "book": 1}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Book 1"));
    }

    #[tokio::test]
    async fn test_browse_missing_collection() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "browse"}"#))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Collection is required"));
    }

    #[tokio::test]
    async fn test_browse_empty_result() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "browse", "collection": "nasai"}"#,
            ))
            .await;
        assert!(result.success);
        assert!(result.output.contains("No hadith found"));
        assert!(result.output.contains("sync"));
    }

    #[tokio::test]
    async fn test_invalid_action() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(r#"{"action": "delete"}"#))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown action"));
    }

    #[tokio::test]
    async fn test_invalid_args() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill.execute(make_input("not json")).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid args"));
    }

    #[tokio::test]
    async fn test_invalid_collection() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "search", "query": "test", "collection": "fake"}"#,
            ))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid collection"));
    }

    #[tokio::test]
    async fn test_invalid_grade() {
        let db = setup_db().await;
        let skill = HadithSkill::new(db);
        let result = skill
            .execute(make_input(
                r#"{"action": "search", "query": "test", "grade": "mawdu"}"#,
            ))
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid grade"));
    }
}
