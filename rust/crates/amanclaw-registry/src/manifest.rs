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

    #[test]
    fn test_manifest_all_fields() {
        let toml = r#"
name = "full"
version = "2.1.0"
description = "A fully specified skill"
authors = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
license = "Apache-2.0"
homepage = "https://example.com"
repository = "https://github.com/example/skill"
type = "script"
entry = "main.py"
tags = ["utility", "api"]
min_engine_version = "0.2.0"

[dependencies]
http = "1.0"
parser = { version = "0.5", optional = false }
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.name, "full");
        assert_eq!(manifest.version, "2.1.0");
        assert_eq!(manifest.description.as_deref(), Some("A fully specified skill"));
        assert_eq!(manifest.authors.len(), 2);
        assert_eq!(manifest.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(manifest.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(manifest.repository.as_deref(), Some("https://github.com/example/skill"));
        assert_eq!(manifest.skill_type, "script");
        assert_eq!(manifest.entry.as_deref(), Some("main.py"));
        assert_eq!(manifest.tags.len(), 2);
        assert_eq!(manifest.min_engine_version.as_deref(), Some("0.2.0"));
        assert_eq!(manifest.dependencies.len(), 2);
    }

    #[test]
    fn test_manifest_missing_name() {
        let toml = r#"
version = "1.0.0"
"#;
        assert!(SkillManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_manifest_missing_version() {
        let toml = r#"
name = "test"
"#;
        assert!(SkillManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_manifest_invalid_toml() {
        let toml = "this is not valid toml {{{";
        assert!(SkillManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_manifest_prerelease_version() {
        let toml = r#"
name = "beta"
version = "1.0.0-beta.1"
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.version, "1.0.0-beta.1");
    }

    #[test]
    fn test_manifest_mixed_dependencies() {
        let toml = r#"
name = "mixed"
version = "1.0.0"

[dependencies]
simple = "1.0"
detailed = { version = "2.0", optional = true }
required = { version = "3.0", optional = false }
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.dependencies.len(), 3);

        match &manifest.dependencies["simple"] {
            DependencySpec::Version(v) => assert_eq!(v, "1.0"),
            _ => panic!("Expected simple version"),
        }
        match &manifest.dependencies["detailed"] {
            DependencySpec::Detailed { version, optional } => {
                assert_eq!(version, "2.0");
                assert!(optional);
            }
            _ => panic!("Expected detailed dependency"),
        }
        match &manifest.dependencies["required"] {
            DependencySpec::Detailed { version, optional } => {
                assert_eq!(version, "3.0");
                assert!(!optional);
            }
            _ => panic!("Expected detailed dependency"),
        }
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let toml_str = r#"
name = "roundtrip"
version = "1.0.0"
description = "Test roundtrip"
type = "wasm"
tags = ["test"]
"#;
        let manifest = SkillManifest::from_toml(toml_str).unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: SkillManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "roundtrip");
        assert_eq!(parsed.tags, vec!["test"]);
    }

    #[test]
    fn test_manifest_empty_tags_and_deps() {
        let toml = r#"
name = "empty"
version = "0.0.1"
"#;
        let manifest = SkillManifest::from_toml(toml).unwrap();
        assert!(manifest.tags.is_empty());
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.authors.is_empty());
    }
}
