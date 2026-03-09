use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct SkillManifest {
    skill: SkillSection,
    #[allow(dead_code)]
    permissions: Option<PermissionsSection>,
}

#[derive(Debug, Deserialize)]
struct SkillSection {
    name: String,
    version: String,
    description: String,
    language: String,
    #[allow(dead_code)]
    entry: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PermissionsSection {
    #[allow(dead_code)]
    network: Option<bool>,
    #[allow(dead_code)]
    filesystem: Option<bool>,
}

pub struct ValidationResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub language: String,
    pub warnings: Vec<String>,
}

pub fn validate_skill(path: &Path) -> Result<ValidationResult> {
    let manifest_path = find_manifest(path)?;
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    let manifest: SkillManifest = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;

    let mut warnings = Vec::new();

    if manifest.skill.description.starts_with("TODO:") {
        warnings.push("Description still has TODO placeholder".into());
    }

    if !path.join("README.md").exists() {
        warnings.push("No README.md found (recommended for published skills)".into());
    }

    let has_tests = match manifest.skill.language.as_str() {
        "rust" => {
            let lib_rs = path.join("src/lib.rs");
            if lib_rs.exists() {
                let content = std::fs::read_to_string(&lib_rs).unwrap_or_default();
                content.contains("#[test]") || content.contains("#[cfg(test)]")
            } else {
                false
            }
        }
        "python" => {
            let entry = manifest.skill.entry.as_deref().unwrap_or("");
            let test_file = format!("test_{entry}");
            path.join(&test_file).exists() || path.join("tests").exists()
        }
        _ => true,
    };
    if !has_tests {
        warnings.push("No tests detected (required for 'verified' tier)".into());
    }

    Ok(ValidationResult {
        name: manifest.skill.name,
        version: manifest.skill.version,
        description: manifest.skill.description,
        language: manifest.skill.language,
        warnings,
    })
}

fn find_manifest(path: &Path) -> Result<std::path::PathBuf> {
    let direct = path.join("amanclaw-skill.toml");
    if direct.exists() {
        return Ok(direct);
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("skill_") && name_str.ends_with(".toml") {
                return Ok(entry.path());
            }
        }
    }

    bail!(
        "No skill manifest found in {}. Expected amanclaw-skill.toml or skill_*.toml",
        path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_skill_with_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let manifest = "[skill]\nname = \"test\"\nversion = \"0.1.0\"\ndescription = \"A test skill\"\nlanguage = \"rust\"\n\n[permissions]\nnetwork = false\n";
        std::fs::write(tmp.path().join("amanclaw-skill.toml"), manifest).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "#[test] fn t() {}").unwrap();

        let result = validate_skill(tmp.path()).unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.language, "rust");
        assert!(result.warnings.iter().any(|w| w.contains("README")));
    }

    #[test]
    fn test_validate_skill_todo_description() {
        let tmp = tempfile::TempDir::new().unwrap();
        let manifest = "[skill]\nname = \"test\"\nversion = \"0.1.0\"\ndescription = \"TODO: describe\"\nlanguage = \"python\"\n";
        std::fs::write(tmp.path().join("amanclaw-skill.toml"), manifest).unwrap();

        let result = validate_skill(tmp.path()).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("TODO")));
    }

    #[test]
    fn test_find_manifest_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(find_manifest(tmp.path()).is_err());
    }

    #[test]
    fn test_find_manifest_skill_prefix() {
        let tmp = tempfile::TempDir::new().unwrap();
        let manifest = "[skill]\nname = \"foo\"\nversion = \"0.1.0\"\ndescription = \"Foo\"\nlanguage = \"python\"\nentry = \"skill_foo.py\"\n";
        std::fs::write(tmp.path().join("skill_foo.toml"), manifest).unwrap();

        let found = find_manifest(tmp.path()).unwrap();
        assert!(found.to_string_lossy().contains("skill_foo.toml"));
    }
}
