mod collection;

use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use collection::{by_category, get_categories, random_doa, search_doa, Doa};

pub struct DoaSkill;

fn format_doa(doa: &Doa) -> String {
    format!(
        "📿 {} / {}\n\n{}\n\n🔤 {}\n\n🇲🇾 {}\n🇬🇧 {}\n\n📖 {}",
        doa.title_ms, doa.title_en, doa.arabic, doa.transliteration,
        doa.translation_ms, doa.translation_en, doa.source
    )
}

fn format_doa_list(doas: &[&Doa]) -> String {
    if doas.is_empty() {
        return "Tiada doa dijumpai. / No doa found.".to_string();
    }
    doas.iter()
        .map(|d| format_doa(d))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

#[async_trait::async_trait]
impl Skill for DoaSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "doa".into(),
            description: "Koleksi doa dan zikir harian. Collection of daily supplications and remembrance (doa & zikir).".into(),
            timeout_ms: 5000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list_categories", "by_category", "search", "random"],
                    "description": "Action to perform. list_categories: show all categories, by_category: get doas in a category, search: search doas by keyword, random: get a random doa"
                },
                "category": {
                    "type": "string",
                    "description": "Category name for by_category action (e.g. harian, pagi, petang, solat, musafir, makan, tidur, wudhu, masjid)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query for search action"
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
                    error: Some(format!("Invalid args: {}", e)),
                };
            }
        };

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list_categories");

        match action {
            "list_categories" => {
                let cats = get_categories();
                let output = cats
                    .iter()
                    .map(|(code, label)| format!("• {} — {}", code, label))
                    .collect::<Vec<_>>()
                    .join("\n");
                SkillResult {
                    success: true,
                    output: format!("Kategori Doa & Zikir:\n\n{}", output),
                    error: None,
                }
            }
            "by_category" => {
                let category = args
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("harian");
                let doas = by_category(category);
                if doas.is_empty() {
                    SkillResult {
                        success: true,
                        output: format!(
                            "Tiada doa dalam kategori '{}'. Sila guna action 'list_categories' untuk senarai kategori.",
                            category
                        ),
                        error: None,
                    }
                } else {
                    SkillResult {
                        success: true,
                        output: format!(
                            "Doa Kategori '{}':\n\n{}",
                            category,
                            format_doa_list(&doas)
                        ),
                        error: None,
                    }
                }
            }
            "search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if query.is_empty() {
                    return SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some("Parameter 'query' diperlukan untuk action 'search'.".into()),
                    };
                }
                let results = search_doa(query);
                SkillResult {
                    success: true,
                    output: format!(
                        "Hasil carian '{}' ({} dijumpai):\n\n{}",
                        query,
                        results.len(),
                        format_doa_list(&results)
                    ),
                    error: None,
                }
            }
            "random" => {
                let doa = random_doa();
                SkillResult {
                    success: true,
                    output: format!("Doa Rawak:\n\n{}", format_doa(doa)),
                    error: None,
                }
            }
            _ => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Unknown action '{}'. Use: list_categories, by_category, search, random",
                    action
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let skill = DoaSkill;
        let meta = skill.metadata();
        assert_eq!(meta.name, "doa");
        assert_eq!(meta.timeout_ms, 5000);
    }

    #[test]
    fn test_parameters_schema() {
        let skill = DoaSkill;
        let schema = skill.parameters_schema();
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("action").is_some());
    }

    #[tokio::test]
    async fn test_list_categories() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "list_categories"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("harian"));
        assert!(result.output.contains("pagi"));
        assert!(result.output.contains("masjid"));
    }

    #[tokio::test]
    async fn test_by_category() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "by_category", "category": "harian"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("harian"));
        assert!(result.output.contains("Doa"));
    }

    #[tokio::test]
    async fn test_by_category_empty() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "by_category", "category": "nonexistent"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Tiada doa"));
    }

    #[tokio::test]
    async fn test_search() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "search", "query": "makan"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("makan"));
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "search", "query": ""}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_random() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "random"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Doa Rawak"));
    }

    #[tokio::test]
    async fn test_invalid_action() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: r#"{"action": "invalid"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown action"));
    }

    #[tokio::test]
    async fn test_invalid_args() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: "not json".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_default_action() {
        let skill = DoaSkill;
        let input = SkillInput {
            name: "doa".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Kategori"));
    }

    #[test]
    fn test_collection_has_enough_doas() {
        assert!(collection::ALL_DOA.len() >= 15, "Should have at least 15 doas");
    }

    #[test]
    fn test_collection_categories_covered() {
        let categories: std::collections::HashSet<&str> =
            collection::ALL_DOA.iter().map(|d| d.category).collect();
        for cat in &["harian", "pagi", "petang", "solat", "musafir", "tidur", "wudhu", "masjid", "makan"] {
            assert!(categories.contains(cat), "Category '{}' should have at least one doa", cat);
        }
    }

    #[test]
    fn test_search_doa_finds_results() {
        let results = collection::search_doa("tidur");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_doa_no_results() {
        let results = collection::search_doa("xyznonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_by_id() {
        let doa = collection::get_by_id(1);
        assert!(doa.is_some());
        assert_eq!(doa.unwrap().id, 1);
    }

    #[test]
    fn test_by_category_returns_correct() {
        let doas = collection::by_category("wudhu");
        assert!(!doas.is_empty());
        for d in &doas {
            assert_eq!(d.category, "wudhu");
        }
    }
}
