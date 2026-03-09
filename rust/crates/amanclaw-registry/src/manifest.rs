use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed `amanclaw-skill.toml` manifest for a skill package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,

    #[serde(rename = "type", default = "default_skill_type")]
    pub skill_type: String,

    #[serde(default)]
    pub entry: Option<String>,

    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(default)]
    pub min_engine_version: Option<String>,

    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Version(String),
    Detailed {
        version: String,
        #[serde(default)]
        optional: bool,
    },
}

fn default_skill_type() -> String {
    "wasm".into()
}

impl SkillManifest {
    pub fn from_toml(content: &str) -> anyhow::Result<Self> {
        let manifest: Self = toml::from_str(content)?;
        semver::Version::parse(&manifest.version)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse() {
        let toml = r#"
name = "weather"
version = "1.0.0"
description = "Get weather forecasts"
authors = ["Alice <alice@example.com>"]
license = "MIT"
type = "wasm"
entry = "weather.wasm"
tags = ["utility", "weather"]

[dependencies]
http-client = "0.2"
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.name, "weather");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.skill_type, "wasm");
        assert_eq!(manifest.entry.as_deref(), Some("weather.wasm"));
        assert_eq!(manifest.tags.len(), 2);
        assert!(manifest.dependencies.contains_key("http-client"));
    }

    #[test]
    fn test_manifest_minimal() {
        let toml = r#"
name = "hello"
version = "0.1.0"
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.name, "hello");
        assert_eq!(manifest.skill_type, "wasm"); // default
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_manifest_invalid_version() {
        let toml = r#"
name = "bad"
version = "not-semver"
"#;
        assert!(SkillManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_manifest_detailed_dependency() {
        let toml = r#"
name = "complex"
version = "1.0.0"

[dependencies]
core = { version = "2.0", optional = true }
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        match &manifest.dependencies["core"] {
            DependencySpec::Detailed { version, optional } => {
                assert_eq!(version, "2.0");
                assert!(optional);
            }
            _ => panic!("Expected detailed dependency"),
        }
    }
}
