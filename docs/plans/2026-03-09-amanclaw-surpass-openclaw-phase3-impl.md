# Phase 3: Ecosystem Launch — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable external developers to discover, install, and publish AmanClaw skills through a GitHub-powered ecosystem with quality tiers and curated packs.

**Architecture:** New `amanclaw-skill-index` crate provides data models and client for a JSON-based skill index hosted on GitHub. CLI extended with `search`, `install`, `install-pack`, and `publish` subcommands. Enhanced scaffolding generates full GitHub template repo structure. SOUL.md persona system documented with examples.

**Tech Stack:** Rust, reqwest, serde, clap, tokio, tempfile (tests)

---

### Task 1: Skill Index Data Model Crate

Create `amanclaw-skill-index` crate with data models for the skill ecosystem.

**Files:**
- Create: `rust/crates/amanclaw-skill-index/Cargo.toml`
- Create: `rust/crates/amanclaw-skill-index/src/lib.rs`
- Create: `rust/crates/amanclaw-skill-index/src/models.rs`
- Create: `rust/crates/amanclaw-skill-index/src/client.rs`
- Modify: `rust/Cargo.toml` (add to workspace members)

**Data models (`models.rs`):**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillTier {
    Community,
    Verified,
    Official,
}

impl SkillTier {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Community => "[community]",
            Self::Verified => "[verified]",
            Self::Official => "[official]",
        }
    }
}

impl std::fmt::Display for SkillTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Community => write!(f, "community"),
            Self::Verified => write!(f, "verified"),
            Self::Official => write!(f, "official"),
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
    pub packs: std::collections::HashMap<String, Vec<String>>,
}

impl SkillIndex {
    /// Search skills by query (matches name, description, tags).
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

    /// Find a skill by exact name.
    pub fn find(&self, name: &str) -> Option<&SkillEntry> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Get all skills in a pack.
    pub fn pack_skills(&self, pack_name: &str) -> Option<Vec<&SkillEntry>> {
        let names = self.packs.get(pack_name)?;
        let skills: Vec<&SkillEntry> = names
            .iter()
            .filter_map(|n| self.find(n))
            .collect();
        Some(skills)
    }

    /// List available pack names.
    pub fn pack_names(&self) -> Vec<&str> {
        self.packs.keys().map(|s| s.as_str()).collect()
    }
}
```

**Client (`client.rs`):**

```rust
use crate::models::SkillIndex;
use anyhow::{Context, Result};

const DEFAULT_INDEX_URL: &str =
    "https://raw.githubusercontent.com/AmanClaw/skill-index/main/index.json";

pub struct IndexClient {
    http: reqwest::Client,
    index_url: String,
}

impl IndexClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            index_url: DEFAULT_INDEX_URL.to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            index_url: url,
        }
    }

    /// Fetch the skill index from the remote URL.
    pub async fn fetch_index(&self) -> Result<SkillIndex> {
        let resp = self
            .http
            .get(&self.index_url)
            .send()
            .await
            .context("Failed to fetch skill index")?;
        let index: SkillIndex = resp
            .json()
            .await
            .context("Failed to parse skill index JSON")?;
        Ok(index)
    }

    /// Parse a skill index from a JSON string (for testing/offline use).
    pub fn parse_index(json: &str) -> Result<SkillIndex> {
        serde_json::from_str(json).context("Failed to parse skill index JSON")
    }
}

impl Default for IndexClient {
    fn default() -> Self {
        Self::new()
    }
}
```

**`lib.rs`:**

```rust
mod client;
mod models;

pub use client::IndexClient;
pub use models::{SkillEntry, SkillIndex, SkillTier};
```

**`Cargo.toml`:**

```toml
[package]
name = "amanclaw-skill-index"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
anyhow = { workspace = true }
```

**Tests (in `models.rs`):**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_index() -> SkillIndex {
        serde_json::from_str(r#"{
            "skills": [
                {
                    "name": "skill-solat",
                    "version": "1.0.0",
                    "description": "Malaysian prayer times by JAKIM zone",
                    "author": "amanclaw",
                    "repo": "amanclaw/skill-solat",
                    "tier": "official",
                    "lang": "rust",
                    "tags": ["islamic", "prayer", "malaysia"]
                },
                {
                    "name": "skill-weather",
                    "version": "0.3.0",
                    "description": "Weather forecast via OpenWeatherMap",
                    "author": "community-dev",
                    "repo": "community-dev/skill-weather",
                    "tier": "verified",
                    "lang": "python",
                    "tags": ["weather", "utility"]
                },
                {
                    "name": "skill-joke",
                    "version": "0.1.0",
                    "description": "Random jokes",
                    "author": "fun-dev",
                    "repo": "fun-dev/skill-joke",
                    "tier": "community",
                    "lang": "python",
                    "tags": ["fun", "entertainment"]
                }
            ],
            "packs": {
                "islamic": ["skill-solat"],
                "fun": ["skill-joke"]
            }
        }"#).unwrap()
    }

    #[test]
    fn test_search_by_name() {
        let idx = sample_index();
        let results = idx.search("solat");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "skill-solat");
    }

    #[test]
    fn test_search_by_tag() {
        let idx = sample_index();
        let results = idx.search("islamic");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_by_description() {
        let idx = sample_index();
        let results = idx.search("weather");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "skill-weather");
    }

    #[test]
    fn test_search_case_insensitive() {
        let idx = sample_index();
        let results = idx.search("PRAYER");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_exact() {
        let idx = sample_index();
        assert!(idx.find("skill-solat").is_some());
        assert!(idx.find("nonexistent").is_none());
    }

    #[test]
    fn test_pack_skills() {
        let idx = sample_index();
        let islamic = idx.pack_skills("islamic").unwrap();
        assert_eq!(islamic.len(), 1);
        assert_eq!(islamic[0].name, "skill-solat");
    }

    #[test]
    fn test_pack_names() {
        let idx = sample_index();
        let names = idx.pack_names();
        assert!(names.contains(&"islamic"));
        assert!(names.contains(&"fun"));
    }

    #[test]
    fn test_tier_badge() {
        assert_eq!(SkillTier::Official.badge(), "[official]");
        assert_eq!(SkillTier::Verified.badge(), "[verified]");
        assert_eq!(SkillTier::Community.badge(), "[community]");
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", SkillTier::Official), "official");
    }

    #[test]
    fn test_parse_index() {
        let json = r#"{"skills":[],"packs":{}}"#;
        let idx = IndexClient::parse_index(json).unwrap();
        assert!(idx.skills.is_empty());
    }
}
```

**Run:** `cargo test -p amanclaw-skill-index`
**Expected:** All 10 tests pass.

**Commit:** `feat: add amanclaw-skill-index crate with data models and client`

---

### Task 2: CLI Skill Search Command

Add `amanclaw skill search <query>` that fetches the skill index and displays matching skills with tier badges.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` (add `Search` to `SkillAction`)
- Modify: `rust/crates/amanclaw-cli/src/main.rs` (handle search action)
- Modify: `rust/crates/amanclaw-cli/Cargo.toml` (add `amanclaw-skill-index` dep)

**Changes to `cli.rs`:**

Add to `SkillAction` enum:
```rust
/// Search for skills in the index
Search {
    /// Search query (matches name, description, tags)
    query: String,
},
/// List available skill packs
Packs,
```

**Changes to `main.rs`:**

Add handler in `cmd_skill`:
```rust
SkillAction::Search { query } => {
    let client = amanclaw_skill_index::IndexClient::new();
    let index = client.fetch_index().await?;
    let results = index.search(&query);
    if results.is_empty() {
        println!("No skills found matching '{query}'.");
    } else {
        println!("Found {} skill(s) matching '{query}':\n", results.len());
        for s in &results {
            println!(
                "{} {} v{} — {}",
                s.tier.badge(),
                s.name,
                s.version,
                s.description
            );
            println!("  repo: {} | lang: {} | tags: {}", s.repo, s.lang, s.tags.join(", "));
            println!();
        }
    }
    Ok(())
}
SkillAction::Packs => {
    let client = amanclaw_skill_index::IndexClient::new();
    let index = client.fetch_index().await?;
    let names = index.pack_names();
    if names.is_empty() {
        println!("No skill packs available.");
    } else {
        println!("Available skill packs:\n");
        for name in &names {
            let count = index.packs.get(*name).map(|v| v.len()).unwrap_or(0);
            println!("  {name} ({count} skills)");
        }
        println!("\nInstall a pack: amanclaw skill install-pack <name>");
    }
    Ok(())
}
```

Note: `cmd_skill` must become `async fn` since search needs network.

**CLI tests to add:**
```rust
#[test]
fn test_cli_skill_search() {
    let cli = Cli::parse_from(["amanclaw", "skill", "search", "prayer"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::Search { query } }) => {
            assert_eq!(query, "prayer");
        }
        _ => panic!("expected Skill Search command"),
    }
}

#[test]
fn test_cli_skill_packs() {
    let cli = Cli::parse_from(["amanclaw", "skill", "packs"]);
    assert!(matches!(
        cli.command,
        Some(Command::Skill { action: SkillAction::Packs })
    ));
}
```

**Run:** `cargo test -p amanclaw-cli`
**Expected:** All CLI parsing tests pass.

**Commit:** `feat(cli): add skill search and packs commands`

---

### Task 3: CLI Skill Install Command

Add `amanclaw skill install <name>` that downloads a skill release from GitHub and places it in the plugins directory.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` (add `Install` to `SkillAction`)
- Create: `rust/crates/amanclaw-cli/src/skill_installer.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs` (add mod + handler)

**Add to `SkillAction`:**
```rust
/// Install a skill from the index
Install {
    /// Skill name (e.g. "skill-solat") or repo (e.g. "amanclaw/skill-solat")
    name: String,

    /// Custom plugins directory
    #[arg(long, default_value = "plugins")]
    plugins_dir: String,
},
/// Install all skills from a pack
InstallPack {
    /// Pack name (e.g. "islamic", "productivity")
    pack: String,

    /// Custom plugins directory
    #[arg(long, default_value = "plugins")]
    plugins_dir: String,
},
```

**`skill_installer.rs`:**

```rust
use anyhow::{Context, Result, bail};
use std::path::Path;

/// Download a skill's latest release artifact from GitHub.
/// For Rust skills: downloads .wasm from release assets.
/// For Python skills: downloads .py + .toml from release assets.
pub async fn install_skill(
    repo: &str,
    skill_name: &str,
    lang: &str,
    plugins_dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(plugins_dir)
        .with_context(|| format!("Failed to create {}", plugins_dir.display()))?;

    let http = reqwest::Client::builder()
        .user_agent("amanclaw-cli")
        .build()?;

    // Fetch latest release from GitHub API
    let api_url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = http
        .get(&api_url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch release info for {repo}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("GitHub API returned {status} for {repo}: {body}");
    }

    let release: serde_json::Value = resp.json().await?;
    let assets = release["assets"]
        .as_array()
        .context("No assets in release")?;

    let target_ext = match lang {
        "rust" => ".wasm",
        "python" => ".py",
        _ => bail!("Unsupported skill language: {lang}"),
    };

    // Download matching assets
    let mut downloaded = 0;
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        let download_url = asset["browser_download_url"]
            .as_str()
            .unwrap_or("");

        if name.ends_with(target_ext) || name.ends_with(".toml") {
            let dest = plugins_dir.join(name);
            let bytes = http
                .get(download_url)
                .send()
                .await?
                .bytes()
                .await?;
            std::fs::write(&dest, &bytes)
                .with_context(|| format!("Failed to write {}", dest.display()))?;
            println!("  Downloaded: {name}");
            downloaded += 1;
        }
    }

    if downloaded == 0 {
        bail!("No {target_ext} assets found in latest release of {repo}");
    }

    println!("Installed {skill_name} ({downloaded} file(s)) to {}", plugins_dir.display());
    Ok(())
}

/// Resolve a skill name to a repo path.
/// If input contains '/', treat as repo. Otherwise, look up in index.
pub fn resolve_repo(name: &str) -> String {
    if name.contains('/') {
        name.to_string()
    } else {
        // Default to amanclaw org
        format!("amanclaw/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_repo_with_slash() {
        assert_eq!(resolve_repo("user/skill-foo"), "user/skill-foo");
    }

    #[test]
    fn test_resolve_repo_without_slash() {
        assert_eq!(resolve_repo("skill-solat"), "amanclaw/skill-solat");
    }
}
```

**Handler in `main.rs`:**
```rust
SkillAction::Install { name, plugins_dir } => {
    let client = amanclaw_skill_index::IndexClient::new();
    let index = client.fetch_index().await?;

    let entry = index.find(&name);
    let (repo, lang) = if let Some(e) = entry {
        (e.repo.clone(), e.lang.clone())
    } else {
        let repo = skill_installer::resolve_repo(&name);
        (repo, "rust".into()) // default to rust
    };

    println!("Installing {name} from {repo}...");
    skill_installer::install_skill(&repo, &name, &lang, Path::new(&plugins_dir)).await?;
    Ok(())
}
SkillAction::InstallPack { pack, plugins_dir } => {
    let client = amanclaw_skill_index::IndexClient::new();
    let index = client.fetch_index().await?;

    let skills = index.pack_skills(&pack)
        .with_context(|| format!("Pack '{pack}' not found. Use 'amanclaw skill packs' to see available packs."))?;

    println!("Installing pack '{pack}' ({} skills)...\n", skills.len());
    let dir = Path::new(&plugins_dir);
    for s in &skills {
        println!("Installing {}...", s.name);
        if let Err(e) = skill_installer::install_skill(&s.repo, &s.name, &s.lang, dir).await {
            eprintln!("  Warning: Failed to install {}: {e}", s.name);
        }
    }
    println!("\nPack '{pack}' installation complete.");
    Ok(())
}
```

**CLI tests:**
```rust
#[test]
fn test_cli_skill_install() {
    let cli = Cli::parse_from(["amanclaw", "skill", "install", "skill-solat"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::Install { name, plugins_dir } }) => {
            assert_eq!(name, "skill-solat");
            assert_eq!(plugins_dir, "plugins");
        }
        _ => panic!("expected Skill Install command"),
    }
}

#[test]
fn test_cli_skill_install_pack() {
    let cli = Cli::parse_from(["amanclaw", "skill", "install-pack", "islamic"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::InstallPack { pack, .. } }) => {
            assert_eq!(pack, "islamic");
        }
        _ => panic!("expected Skill InstallPack command"),
    }
}
```

**Run:** `cargo test -p amanclaw-cli`
**Expected:** All tests pass.

**Commit:** `feat(cli): add skill install and install-pack commands`

---

### Task 4: CLI Skill Publish Command

Add `amanclaw skill publish` that validates a skill manifest and outputs instructions for publishing to the index.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` (add `Publish` to `SkillAction`)
- Create: `rust/crates/amanclaw-cli/src/skill_publisher.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs` (add mod + handler)

**Add to `SkillAction`:**
```rust
/// Validate and prepare a skill for publishing
Publish {
    /// Path to skill directory (default: current dir)
    #[arg(default_value = ".")]
    path: String,
},
```

**`skill_publisher.rs`:**

```rust
use anyhow::{Context, Result, bail};
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

/// Validate a skill directory for publishing.
pub fn validate_skill(path: &Path) -> Result<ValidationResult> {
    // Find manifest
    let manifest_path = find_manifest(path)?;
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    let manifest: SkillManifest = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;

    let mut warnings = Vec::new();

    // Check description is not default
    if manifest.skill.description.starts_with("TODO:") {
        warnings.push("Description still has TODO placeholder".into());
    }

    // Check for README
    if !path.join("README.md").exists() {
        warnings.push("No README.md found (recommended for published skills)".into());
    }

    // Check for tests
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
    // Look for amanclaw-skill.toml or any skill_*.toml
    let direct = path.join("amanclaw-skill.toml");
    if direct.exists() {
        return Ok(direct);
    }

    // Scan for skill_*.toml
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
        // Should warn about no README
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
```

**Handler in `main.rs`:**
```rust
SkillAction::Publish { path } => {
    let dir = Path::new(&path);
    let result = skill_publisher::validate_skill(dir)?;

    println!("Skill: {} v{}", result.name, result.version);
    println!("Language: {}", result.language);
    println!("Description: {}", result.description);
    println!();

    if result.warnings.is_empty() {
        println!("Validation: PASSED (eligible for 'verified' tier)");
    } else {
        println!("Warnings:");
        for w in &result.warnings {
            println!("  - {w}");
        }
        println!("\nValidation: PASSED with warnings (eligible for 'community' tier)");
    }

    println!("\nTo publish:");
    println!("  1. Push your skill to GitHub");
    println!("  2. Create a release with .wasm or .py artifacts");
    println!("  3. Submit a PR to https://github.com/AmanClaw/skill-index");
    println!("     adding your skill entry to index.json");
    Ok(())
}
```

**CLI test:**
```rust
#[test]
fn test_cli_skill_publish() {
    let cli = Cli::parse_from(["amanclaw", "skill", "publish", "/tmp/my-skill"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::Publish { path } }) => {
            assert_eq!(path, "/tmp/my-skill");
        }
        _ => panic!("expected Skill Publish command"),
    }
}
```

**Run:** `cargo test -p amanclaw-cli`
**Expected:** All tests pass.

**Commit:** `feat(cli): add skill publish validation command`

---

### Task 5: Enhanced Skill Templates

Upgrade scaffold to generate full GitHub template repo structure with CI workflow, README with badges, LICENSE, and test harness.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/scaffold.rs`

**Enhance `scaffold_rust_skill` to also generate:**

1. `README.md` with skill name, badges (build status, crate version), usage instructions
2. `LICENSE` (MIT)
3. `.github/workflows/ci.yml` — build + test + release artifact workflow
4. `tests/integration.rs` — basic integration test template

**Enhance `scaffold_python_skill` to also generate:**

1. `README.md` with skill name, usage instructions
2. `LICENSE` (MIT)
3. `.github/workflows/ci.yml` — lint + test workflow
4. `test_skill_<name>.py` — basic test template

**Example CI for Rust skill:**
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo test
      - run: cargo build --target wasm32-unknown-unknown --release
  release:
    if: startsWith(github.ref, 'refs/tags/')
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo build --target wasm32-unknown-unknown --release
      - uses: softprops/action-gh-release@v2
        with:
          files: target/wasm32-unknown-unknown/release/*.wasm
```

**Tests:** Extend existing scaffold tests to verify new files are created.

**Run:** `cargo test -p amanclaw-cli -- scaffold`
**Expected:** All scaffold tests pass.

**Commit:** `feat(cli): enhance skill templates with CI, README, LICENSE, tests`

---

### Task 6: Sample SOUL.md Personas

Create sample persona files that demonstrate the SOUL.md system for community bots.

**Files:**
- Create: `souls/ustaz.md` — Islamic knowledge assistant persona
- Create: `souls/masjid-admin.md` — Mosque administrator bot persona
- Create: `souls/community.md` — General community assistant persona

**`souls/ustaz.md`:**
```markdown
# UstazBot

You are UstazBot, a knowledgeable and respectful Islamic knowledge assistant.

## Personality
- Warm, patient, and scholarly
- Always cite sources (Quran surah:ayat, Hadith collection)
- Respectful of different madhab opinions
- Answers in the user's language (Malay or English)

## Capabilities
- Answer questions about Islamic jurisprudence (fiqh)
- Provide Quran verses with translation
- Share authentic hadith
- Explain prayer times and procedures
- Guide on zakat calculations

## Guidelines
- Never issue fatwa — always recommend consulting a qualified scholar for personal rulings
- Present mainstream Sunni positions (Shafi'i default for Malaysian context)
- If unsure, say so honestly
- Use skills: solat, quran, hadith, doa, zakat, hijri
```

**`souls/masjid-admin.md`:**
```markdown
# MasjidBot

You are MasjidBot, an administrative assistant for mosque management.

## Personality
- Professional and efficient
- Bilingual (Malay and English)
- Proactive about reminders and schedules

## Capabilities
- Announce prayer times for the mosque's zone
- Manage event announcements (Friday khutbah topics, classes, etc.)
- Send reminders for upcoming events
- Track community member queries

## Guidelines
- Keep announcements concise and clear
- Include relevant prayer times in daily broadcasts
- Use skills: solat, hijri, khutbah
- Format messages appropriately for the chat platform
```

**`souls/community.md`:**
```markdown
# CommunityBot

You are CommunityBot, a friendly general-purpose assistant for community groups.

## Personality
- Friendly, helpful, and inclusive
- Responds in the language the user writes in
- Keeps responses concise for group chats

## Capabilities
- Answer general questions
- Help with community coordination
- Provide useful information (weather, reminders, etc.)
- Welcome new members

## Guidelines
- Be brief in group chats, detailed in private messages
- Don't dominate group conversations
- Use available skills when relevant
- Escalate sensitive topics to human admins
```

**No tests needed** — these are documentation/content files.

**Commit:** `feat: add sample SOUL.md personas (ustaz, masjid-admin, community)`

---

### Task 7: Seed Index JSON

Create a seed `index.json` file that serves as the reference format and includes all existing AmanClaw skills.

**Files:**
- Create: `docs/skill-index.json` — seed/reference index with all current skills and packs

**Content:**
```json
{
  "skills": [
    {
      "name": "skill-solat",
      "version": "1.0.0",
      "description": "Malaysian prayer times by JAKIM zone + global calculation (6 methods)",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "rust",
      "tags": ["islamic", "prayer", "malaysia", "solat"]
    },
    {
      "name": "skill-qiblat",
      "version": "1.0.0",
      "description": "Qibla direction calculator from any location",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "rust",
      "tags": ["islamic", "qibla", "navigation"]
    },
    {
      "name": "skill-hijri",
      "version": "1.0.0",
      "description": "Hijri-Gregorian date converter",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "rust",
      "tags": ["islamic", "calendar", "hijri"]
    },
    {
      "name": "skill-doa",
      "version": "1.0.0",
      "description": "Daily duas (supplications) with Arabic text and translation",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "rust",
      "tags": ["islamic", "dua", "supplication"]
    },
    {
      "name": "skill-quran",
      "version": "1.0.0",
      "description": "Quran verse lookup with translation",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "rust",
      "tags": ["islamic", "quran"]
    },
    {
      "name": "plugin-hadith",
      "version": "1.0.0",
      "description": "Hadith search across major collections",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "python",
      "tags": ["islamic", "hadith"]
    },
    {
      "name": "plugin-halal",
      "version": "1.0.0",
      "description": "JAKIM halal product verification",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "python",
      "tags": ["islamic", "halal", "malaysia"]
    },
    {
      "name": "plugin-zakat",
      "version": "1.0.0",
      "description": "Zakat calculator (income, savings, gold, silver)",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "python",
      "tags": ["islamic", "zakat", "finance"]
    },
    {
      "name": "plugin-masjid",
      "version": "1.0.0",
      "description": "Nearby mosque finder",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "python",
      "tags": ["islamic", "mosque", "location"]
    },
    {
      "name": "plugin-khutbah",
      "version": "1.0.0",
      "description": "Friday khutbah summaries and schedules",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "python",
      "tags": ["islamic", "khutbah", "friday"]
    },
    {
      "name": "plugin-jakim",
      "version": "1.0.0",
      "description": "JAKIM e-services integration",
      "author": "amanclaw",
      "repo": "AmanClaw/amanclaw",
      "tier": "official",
      "lang": "python",
      "tags": ["islamic", "malaysia", "jakim"]
    }
  ],
  "packs": {
    "islamic": [
      "skill-solat", "skill-qiblat", "skill-hijri", "skill-doa", "skill-quran",
      "plugin-hadith", "plugin-halal", "plugin-zakat", "plugin-masjid",
      "plugin-khutbah", "plugin-jakim"
    ],
    "islamic-core": [
      "skill-solat", "skill-qiblat", "skill-hijri", "skill-doa", "skill-quran"
    ],
    "malaysian": [
      "skill-solat", "plugin-halal", "plugin-jakim", "plugin-masjid"
    ]
  }
}
```

**Test:** Add a test in `amanclaw-skill-index` that parses this file.

```rust
#[test]
fn test_parse_seed_index() {
    let json = include_str!("../../../docs/skill-index.json"); // adjusted path
    let idx: SkillIndex = serde_json::from_str(json).unwrap();
    assert_eq!(idx.skills.len(), 11);
    assert_eq!(idx.packs.len(), 3);
    assert!(idx.find("skill-solat").is_some());
    assert_eq!(idx.find("skill-solat").unwrap().tier, SkillTier::Official);
}
```

Note: The exact path from `amanclaw-skill-index/src/` to `docs/skill-index.json` will be `../../../../docs/skill-index.json` (up from `src/` → crate root → `crates/` → `rust/` → project root → `docs/`).

**Run:** `cargo test -p amanclaw-skill-index`
**Expected:** All tests pass including seed index parsing.

**Commit:** `feat: add seed skill index with 11 official skills and 3 packs`

---

### Task 8: Update README

Update the project README to document Phase 3 ecosystem features.

**Files:**
- Modify: `README.md`

**Add sections for:**
- Skill ecosystem (search, install, publish workflow)
- Skill packs with example commands
- Quality tiers explanation
- SOUL.md persona system
- Template repos (link to scaffold command)

**Commit:** `docs: update README with Phase 3 ecosystem features`

---

## Parallelization

Tasks that can run in parallel:
- **Tasks 1 + 6**: Skill index crate + SOUL.md personas (independent)
- **Tasks 2 + 3 + 4**: All depend on Task 1 (sequential after Task 1)
- **Task 5**: Independent of Tasks 2-4 (can parallel with them)
- **Task 7**: Depends on Task 1
- **Task 8**: Last (depends on all others)

Recommended execution order:
1. Tasks 1 + 6 (parallel)
2. Tasks 2 + 3 + 4 + 5 + 7 (parallel where possible, 2/3/4 sequential)
3. Task 8 (last)
