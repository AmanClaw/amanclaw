use anyhow::Result;
use reqwest::Client;

use crate::models::SkillIndex;

const DEFAULT_INDEX_URL: &str =
    "https://raw.githubusercontent.com/AmanClaw/skill-index/main/index.json";

pub struct IndexClient {
    http: Client,
    index_url: String,
}

impl IndexClient {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
            index_url: DEFAULT_INDEX_URL.to_string(),
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            index_url: url.into(),
        }
    }

    pub async fn fetch_index(&self) -> Result<SkillIndex> {
        let resp = self.http.get(&self.index_url).send().await?;
        let text = resp.text().await?;
        Self::parse_index(&text)
    }

    pub fn parse_index(json: &str) -> Result<SkillIndex> {
        let index: SkillIndex = serde_json::from_str(json)?;
        Ok(index)
    }
}

impl Default for IndexClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_index() {
        let json = r#"{"skills": [], "packs": {}}"#;
        let index = IndexClient::parse_index(json).unwrap();
        assert!(index.skills.is_empty());
        assert!(index.packs.is_empty());
    }
}
