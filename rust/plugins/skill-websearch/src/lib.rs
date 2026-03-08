use amanclaw_traits::skill::{Skill, SkillMetadata, SkillInput, SkillResult};
use serde::Deserialize;

pub struct WebSearchSkill;

#[async_trait::async_trait]
impl Skill for WebSearchSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "web_search".into(),
            description: "Search the web using DuckDuckGo for current information".into(),
            timeout_ms: 15000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        // Run blocking HTTP in a thread to avoid blocking tokio
        let args_str = input.args.clone();
        tokio::task::spawn_blocking(move || execute_search(&args_str))
            .await
            .unwrap_or_else(|e| SkillResult { success: false, output: String::new(), error: Some(format!("Task failed: {}", e)) })
    }
}

#[derive(Deserialize)]
struct DdgResult {
    #[serde(rename = "Text")]
    text: String,
    #[allow(dead_code)]
    #[serde(rename = "FirstURL")]
    first_url: String,
}

#[derive(Deserialize)]
struct DdgResponse {
    #[serde(rename = "RelatedTopics")]
    related_topics: Vec<DdgResult>,
    #[serde(rename = "Abstract")]
    abstract_text: String,
}

fn execute_search(args_str: &str) -> SkillResult {
    let args: serde_json::Value = match serde_json::from_str(args_str) {
        Ok(v) => v,
        Err(e) => return SkillResult { success: false, output: String::new(), error: Some(format!("Invalid args: {}", e)) },
    };

    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return SkillResult { success: false, output: String::new(), error: Some("Missing required parameter: query".into()) },
    };

    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1", query);

    let resp = match reqwest::blocking::get(&url) {
        Ok(r) => r,
        Err(e) => return SkillResult { success: false, output: String::new(), error: Some(format!("Search failed: {}", e)) },
    };

    let data: DdgResponse = match resp.json() {
        Ok(d) => d,
        Err(e) => return SkillResult { success: false, output: String::new(), error: Some(format!("Parse failed: {}", e)) },
    };

    let mut results = Vec::new();

    if !data.abstract_text.is_empty() {
        results.push(format!("Summary: {}", data.abstract_text));
    }

    for topic in data.related_topics.iter().take(5) {
        if !topic.text.is_empty() {
            results.push(format!("- {}", topic.text));
        }
    }

    if results.is_empty() {
        SkillResult { success: true, output: format!("No results found for '{}'", query), error: None }
    } else {
        SkillResult { success: true, output: results.join("\n"), error: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let skill = WebSearchSkill;
        assert_eq!(skill.metadata().name, "web_search");
    }

    #[test]
    fn test_missing_query() {
        let result = execute_search("{}");
        assert!(!result.success);
        assert!(result.error.unwrap().contains("query"));
    }
}
