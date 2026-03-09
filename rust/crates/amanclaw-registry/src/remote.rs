use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Entry in the remote skill index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSkillEntry {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Remote registry index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteIndex {
    pub skills: Vec<RemoteSkillEntry>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

pub struct RemoteRegistry {
    base_url: String,
    client: reqwest::Client,
    index: Option<RemoteIndex>,
}

impl RemoteRegistry {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            index: None,
        }
    }

    pub async fn refresh_index(&mut self) -> Result<usize> {
        let url = format!("{}/index.json", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let index: RemoteIndex = resp.json().await?;
        let count = index.skills.len();
        self.index = Some(index);
        tracing::info!(count, "Remote index refreshed");
        Ok(count)
    }

    pub fn search(&self, query: &str) -> Vec<&RemoteSkillEntry> {
        let query_lower = query.to_lowercase();
        match &self.index {
            Some(index) => index
                .skills
                .iter()
                .filter(|s| {
                    s.name.to_lowercase().contains(&query_lower)
                        || s.description
                            .as_deref()
                            .map(|d| d.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                })
                .collect(),
            None => vec![],
        }
    }

    pub fn resolve(&self, name: &str) -> Option<&RemoteSkillEntry> {
        self.index
            .as_ref()
            .and_then(|idx| idx.skills.iter().find(|s| s.name == name))
    }

    pub async fn download(&self, entry: &RemoteSkillEntry, dest: &std::path::Path) -> Result<()> {
        let url = entry
            .download_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("No download URL for skill '{}'", entry.name))?;

        let resp = self.client.get(url).send().await?;
        let bytes = resp.bytes().await?;

        // Verify checksum if provided
        if let Some(expected) = &entry.checksum {
            use sha2::Digest;
            let actual = hex::encode(sha2::Sha256::digest(&bytes));
            if actual != *expected {
                anyhow::bail!(
                    "Checksum mismatch for '{}': expected {}, got {}",
                    entry.name,
                    expected,
                    actual
                );
            }
        }

        // Extract tarball
        std::fs::create_dir_all(dest)?;
        let decoder = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)?;

        tracing::info!(name = %entry.name, "Skill downloaded and extracted");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_index() {
        let index = RemoteIndex {
            skills: vec![
                RemoteSkillEntry {
                    name: "weather".into(),
                    version: "1.0.0".into(),
                    description: Some("Weather forecasts".into()),
                    download_url: None,
                    checksum: None,
                    tags: vec!["utility".into()],
                },
                RemoteSkillEntry {
                    name: "calendar".into(),
                    version: "2.0.0".into(),
                    description: Some("Calendar management".into()),
                    download_url: None,
                    checksum: None,
                    tags: vec!["productivity".into()],
                },
            ],
            updated_at: None,
        };

        let registry = RemoteRegistry {
            base_url: "http://localhost".into(),
            client: reqwest::Client::new(),
            index: Some(index),
        };

        let results = registry.search("weather");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "weather");

        let results = registry.search("utility");
        assert_eq!(results.len(), 1);

        let results = registry.search("management");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "calendar");
    }

    #[test]
    fn test_resolve() {
        let index = RemoteIndex {
            skills: vec![RemoteSkillEntry {
                name: "weather".into(),
                version: "1.0.0".into(),
                description: None,
                download_url: Some("http://example.com/weather.tar.gz".into()),
                checksum: None,
                tags: vec![],
            }],
            updated_at: None,
        };

        let registry = RemoteRegistry {
            base_url: "http://localhost".into(),
            client: reqwest::Client::new(),
            index: Some(index),
        };

        assert!(registry.resolve("weather").is_some());
        assert!(registry.resolve("nonexistent").is_none());
    }
}
