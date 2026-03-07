use amanclaw_plugin_sdk::*;
use serde::Deserialize;

pub fn metadata() -> SkillMetadata {
    SkillMetadata {
        name: "web_search".into(),
        description: "Search the web using DuckDuckGo for current information".into(),
        timeout_ms: 15000,
        version: "0.1.0".into(),
    }
}

pub fn parameters() -> serde_json::Value {
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

pub fn execute(input: SkillInput) -> SkillResult {
    let args: serde_json::Value = match serde_json::from_str(&input.args) {
        Ok(v) => v,
        Err(e) => return SkillResult::err(format!("Invalid args: {}", e)),
    };

    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return SkillResult::err("Missing required parameter: query"),
    };

    // Use DuckDuckGo instant answer API (no API key needed)
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1", query);

    let resp = match reqwest::blocking::get(&url) {
        Ok(r) => r,
        Err(e) => return SkillResult::err(format!("Search failed: {}", e)),
    };

    let data: DdgResponse = match resp.json() {
        Ok(d) => d,
        Err(e) => return SkillResult::err(format!("Parse failed: {}", e)),
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
        SkillResult::ok(format!("No results found for '{}'", query))
    } else {
        SkillResult::ok(results.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let meta = metadata();
        assert_eq!(meta.name, "web_search");
        assert_eq!(meta.timeout_ms, 15000);
    }

    #[test]
    fn test_missing_query() {
        let input = SkillInput {
            name: "web_search".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = execute(input);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("query"));
    }
}
