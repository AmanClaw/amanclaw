use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

pub struct ResolvedSoul {
    pub prompt: String,
    pub variables: HashMap<String, String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SoulFrontmatter {
    #[serde(default)]
    _version: u32,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    _language: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    variables: HashMap<String, String>,
}

pub struct SoulLoader;

impl SoulLoader {
    pub fn load(soul_dir: &Path, filename: &str) -> Result<ResolvedSoul> {
        let mut chain = Vec::new();
        let mut current = Some(filename.to_string());

        while let Some(ref n) = current {
            let name = n.clone();
            if chain.len() >= 5 {
                anyhow::bail!(
                    "Soul inheritance depth exceeds maximum of 5: {:?}",
                    chain
                        .iter()
                        .map(|(_, _, n): &(SoulFrontmatter, String, String)| n.clone())
                        .collect::<Vec<_>>()
                );
            }
            let path = soul_dir.join(&name);
            let raw = std::fs::read_to_string(&path).map_err(|e| {
                anyhow::anyhow!("Failed to read soul file '{}': {}", path.display(), e)
            })?;
            let (frontmatter, body) = Self::parse_frontmatter(&raw)?;
            current = frontmatter.extends.clone();
            chain.push((frontmatter, body, name));
        }

        chain.reverse();
        Self::merge_chain(chain)
    }

    fn parse_frontmatter(raw: &str) -> Result<(SoulFrontmatter, String)> {
        if let Some(rest) = raw.strip_prefix("---")
            && let Some(end) = rest.find("---")
        {
            let fm_str = &rest[..end];
            let body = rest[end + 3..].trim().to_string();
            let fm: SoulFrontmatter = serde_yaml::from_str(fm_str.trim()).unwrap_or_default();
            return Ok((fm, body));
        }
        Ok((SoulFrontmatter::default(), raw.to_string()))
    }

    fn parse_sections(body: &str) -> Vec<(String, String)> {
        let mut sections = Vec::new();
        let mut current_heading = "_preamble".to_string();
        let mut current_content = String::new();

        for line in body.lines() {
            if let Some(heading) = line.strip_prefix("## ") {
                if !current_content.trim().is_empty() || current_heading == "_preamble" {
                    sections.push((current_heading.clone(), current_content.trim().to_string()));
                }
                current_heading = heading.trim().to_string();
                current_content = String::new();
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
        if !current_content.trim().is_empty() || sections.is_empty() {
            sections.push((current_heading, current_content.trim().to_string()));
        }
        sections
    }

    fn merge_chain(chain: Vec<(SoulFrontmatter, String, String)>) -> Result<ResolvedSoul> {
        let mut merged_vars: HashMap<String, String> = HashMap::new();
        let mut merged_sections: Vec<(String, String)> = Vec::new();
        let mut tags = Vec::new();

        for (fm, body, _name) in chain {
            merged_vars.extend(fm.variables);
            tags.extend(fm.tags);

            let sections = Self::parse_sections(&body);
            for (heading, content) in sections {
                if let Some(existing) = merged_sections.iter_mut().find(|(h, _)| h == &heading) {
                    existing.1 = content;
                } else {
                    merged_sections.push((heading, content));
                }
            }
        }

        let mut prompt = String::new();
        for (heading, content) in &merged_sections {
            if heading == "_preamble" {
                prompt.push_str(content);
            } else {
                prompt.push_str(&format!("\n\n## {heading}\n{content}"));
            }
        }

        for (key, value) in &merged_vars {
            prompt = prompt.replace(&format!("{{{{{key}}}}}"), value);
        }

        Ok(ResolvedSoul {
            prompt: prompt.trim().to_string(),
            variables: merged_vars,
            tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_simple_soul() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.md"),
            "# TestBot\n\nYou are a test bot.",
        )
        .unwrap();

        let soul = SoulLoader::load(dir.path(), "test.md").unwrap();
        assert!(soul.prompt.contains("You are a test bot"));
    }

    #[test]
    fn test_load_soul_with_frontmatter() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.md"),
            r#"---
version: 1
tags: [islamic, test]
variables:
  region: malaysia
---
# TestBot

Expert for {{region}}.
"#,
        )
        .unwrap();

        let soul = SoulLoader::load(dir.path(), "test.md").unwrap();
        assert!(soul.prompt.contains("Expert for malaysia"));
        assert_eq!(soul.tags, vec!["islamic", "test"]);
        assert_eq!(soul.variables.get("region").unwrap(), "malaysia");
    }

    #[test]
    fn test_load_soul_with_inheritance() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("base.md"),
            r#"---
version: 1
variables:
  greeting: Hello
---
# Base

{{greeting}} world.

## Rules
- Be helpful
"#,
        )
        .unwrap();

        fs::write(
            dir.path().join("child.md"),
            r#"---
version: 1
extends: base.md
variables:
  greeting: Assalamualaikum
---
# Child

{{greeting}}, Islamic expert here.

## Rules
- Follow Islamic guidelines
"#,
        )
        .unwrap();

        let soul = SoulLoader::load(dir.path(), "child.md").unwrap();
        // Child overrides greeting variable and uses it in preamble
        assert!(soul.prompt.contains("Assalamualaikum, Islamic expert here"));
        // Child's "Rules" section overrides base's "Rules" section
        assert!(soul.prompt.contains("Follow Islamic guidelines"));
        assert!(!soul.prompt.contains("Be helpful"));
    }

    #[test]
    fn test_max_inheritance_depth() {
        let dir = TempDir::new().unwrap();
        for i in 0..6 {
            let extends = if i > 0 {
                format!("extends: level{}.md", i - 1)
            } else {
                String::new()
            };
            let content = format!("---\n{extends}\n---\n# Level {i}");
            fs::write(dir.path().join(format!("level{i}.md")), content).unwrap();
        }
        let result = SoulLoader::load(dir.path(), "level5.md");
        assert!(result.is_err()); // Max depth 5 exceeded
    }
}
