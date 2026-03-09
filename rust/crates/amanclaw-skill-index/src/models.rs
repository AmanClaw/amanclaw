use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillTier {
    Community,
    Verified,
    Official,
}

impl SkillTier {
    pub fn badge(&self) -> &'static str {
        match self {
            SkillTier::Community => "🌱",
            SkillTier::Verified => "✅",
            SkillTier::Official => "⭐",
        }
    }
}

impl fmt::Display for SkillTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillTier::Community => write!(f, "Community"),
            SkillTier::Verified => write!(f, "Verified"),
            SkillTier::Official => write!(f, "Official"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repo: String,
    pub tier: SkillTier,
    pub lang: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillIndex {
    pub skills: Vec<SkillEntry>,
    #[serde(default)]
    pub packs: HashMap<String, Vec<String>>,
}

impl SkillIndex {
    pub fn search(&self, query: &str) -> Vec<&SkillEntry> {
        let q = query.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    pub fn find(&self, name: &str) -> Option<&SkillEntry> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn pack_skills(&self, pack_name: &str) -> Option<&Vec<String>> {
        self.packs.get(pack_name)
    }

    pub fn pack_names(&self) -> Vec<&String> {
        self.packs.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_index() -> SkillIndex {
        SkillIndex {
            skills: vec![
                SkillEntry {
                    name: "skill-solat".into(),
                    version: "0.1.0".into(),
                    description: "Prayer times for Malaysia".into(),
                    author: "amanclaw".into(),
                    repo: "https://github.com/AmanClaw/skill-solat".into(),
                    tier: SkillTier::Official,
                    lang: "rust".into(),
                    tags: vec!["islamic".into(), "prayer".into()],
                },
                SkillEntry {
                    name: "skill-weather".into(),
                    version: "0.2.0".into(),
                    description: "Weather forecast plugin".into(),
                    author: "community-dev".into(),
                    repo: "https://github.com/example/skill-weather".into(),
                    tier: SkillTier::Community,
                    lang: "python".into(),
                    tags: vec!["weather".into(), "utility".into()],
                },
                SkillEntry {
                    name: "skill-hadith".into(),
                    version: "0.1.0".into(),
                    description: "Hadith lookup service".into(),
                    author: "verified-dev".into(),
                    repo: "https://github.com/example/skill-hadith".into(),
                    tier: SkillTier::Verified,
                    lang: "python".into(),
                    tags: vec!["islamic".into(), "hadith".into()],
                },
            ],
            packs: HashMap::from([
                (
                    "islamic".into(),
                    vec!["skill-solat".into(), "skill-hadith".into()],
                ),
                ("starter".into(), vec!["skill-weather".into()]),
            ]),
        }
    }

    #[test]
    fn test_search_by_name() {
        let index = sample_index();
        let results = index.search("solat");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "skill-solat");
    }

    #[test]
    fn test_search_by_tag() {
        let index = sample_index();
        let results = index.search("islamic");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_by_description() {
        let index = sample_index();
        let results = index.search("forecast");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "skill-weather");
    }

    #[test]
    fn test_search_case_insensitive() {
        let index = sample_index();
        let results = index.search("PRAYER");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "skill-solat");
    }

    #[test]
    fn test_find_exact() {
        let index = sample_index();
        let entry = index.find("skill-hadith");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().author, "verified-dev");

        let missing = index.find("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_pack_skills() {
        let index = sample_index();
        let skills = index.pack_skills("islamic").unwrap();
        assert_eq!(skills.len(), 2);
        assert!(skills.contains(&"skill-solat".to_string()));
        assert!(skills.contains(&"skill-hadith".to_string()));

        assert!(index.pack_skills("nonexistent").is_none());
    }

    #[test]
    fn test_pack_names() {
        let index = sample_index();
        let names = index.pack_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_tier_badge() {
        assert_eq!(SkillTier::Community.badge(), "🌱");
        assert_eq!(SkillTier::Verified.badge(), "✅");
        assert_eq!(SkillTier::Official.badge(), "⭐");
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", SkillTier::Community), "Community");
        assert_eq!(format!("{}", SkillTier::Verified), "Verified");
        assert_eq!(format!("{}", SkillTier::Official), "Official");
    }
}
