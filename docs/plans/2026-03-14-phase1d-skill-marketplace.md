# Plan 1D: Skill Marketplace CLI — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve the skill marketplace CLI with proper `amanclaw skill list`, `amanclaw skill update`, `amanclaw skill remove` commands, checksum verification, and version pinning.

**Architecture:** Extends existing `SkillAction` enum in `cli.rs` and `skill_installer.rs`. The local registry (`amanclaw-registry`) already has SQLite tracking — we connect the installer to it so installed skills are properly tracked. The remote index (`amanclaw-skill-index`) already has search and packs — we add version awareness.

**Tech Stack:** Rust, clap, reqwest, sha2 (for checksums), amanclaw-registry, amanclaw-skill-index

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rust/crates/amanclaw-cli/src/cli.rs` | MODIFY | Add ListInstalled, Update, Remove, Info to SkillAction |
| `rust/crates/amanclaw-cli/src/main.rs` | MODIFY | Handle new skill subcommands |
| `rust/crates/amanclaw-cli/src/skill_installer.rs` | MODIFY | Register installs in local registry, checksum verification |
| `rust/crates/amanclaw-cli/Cargo.toml` | MODIFY | Add sha2 dependency |
| `rust/crates/amanclaw-registry/src/local.rs` | MODIFY | Add update_version(), search improvements |

---

## Chunk 1: New CLI Commands

### Task 1: Add new SkillAction variants

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs`

- [ ] **Step 1: Add new commands to SkillAction enum**

```rust
/// List installed skills
ListInstalled,
/// Show details about an installed skill
Info {
    /// Skill name
    name: String,
},
/// Update an installed skill to latest version
Update {
    /// Skill name (or "all" to update everything)
    name: String,
    /// Custom plugins directory
    #[arg(long, default_value = "plugins")]
    plugins_dir: String,
},
/// Remove an installed skill
Remove {
    /// Skill name
    name: String,
    /// Custom plugins directory
    #[arg(long, default_value = "plugins")]
    plugins_dir: String,
},
```

- [ ] **Step 2: Add clap tests**

```rust
#[test]
fn test_cli_skill_list_installed() {
    let cli = Cli::parse_from(["amanclaw", "skill", "list-installed"]);
    assert!(matches!(
        cli.command,
        Some(Command::Skill { action: SkillAction::ListInstalled })
    ));
}

#[test]
fn test_cli_skill_info() {
    let cli = Cli::parse_from(["amanclaw", "skill", "info", "web_search"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::Info { name } }) => {
            assert_eq!(name, "web_search");
        }
        _ => panic!("expected Skill Info"),
    }
}

#[test]
fn test_cli_skill_remove() {
    let cli = Cli::parse_from(["amanclaw", "skill", "remove", "web_search"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::Remove { name, plugins_dir } }) => {
            assert_eq!(name, "web_search");
            assert_eq!(plugins_dir, "plugins");
        }
        _ => panic!("expected Skill Remove"),
    }
}

#[test]
fn test_cli_skill_update() {
    let cli = Cli::parse_from(["amanclaw", "skill", "update", "all"]);
    match cli.command {
        Some(Command::Skill { action: SkillAction::Update { name, .. } }) => {
            assert_eq!(name, "all");
        }
        _ => panic!("expected Skill Update"),
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test --package amanclaw-cli cli::tests -- --nocapture`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-cli/src/cli.rs
git commit -m "feat(cli): add list-installed, info, update, remove skill commands"
```

---

### Task 2: Implement list-installed and info handlers

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Implement handlers**

In `cmd_skill()`, add match arms:

```rust
SkillAction::ListInstalled => {
    let registry = amanclaw_registry::local::SkillRegistry::new("plugins/registry").await?;
    let skills = registry.list_installed().await?;
    if skills.is_empty() {
        println!("No skills installed via marketplace.");
        println!("(Built-in skills and config-registered plugins are not tracked here)");
        return Ok(());
    }
    println!("Installed skills:\n");
    for s in &skills {
        println!("  {} v{} ({}) — {}", s.name, s.version, s.skill_type,
            s.description.as_deref().unwrap_or(""));
    }
    println!("\n{} skill(s) installed.", skills.len());
    Ok(())
}
SkillAction::Info { name } => {
    let registry = amanclaw_registry::local::SkillRegistry::new("plugins/registry").await?;
    match registry.get(&name).await? {
        Some(skill) => {
            println!("Skill: {}", skill.name);
            println!("Version: {}", skill.version);
            println!("Type: {}", skill.skill_type);
            println!("Description: {}", skill.description.as_deref().unwrap_or("-"));
            println!("Install dir: {}", skill.install_dir);
            println!("Installed at: {}", skill.installed_at);
            if let Some(cs) = &skill.checksum {
                println!("Checksum: {cs}");
            }
        }
        None => println!("Skill '{name}' not found. Use 'amanclaw skill list-installed' to see installed skills."),
    }
    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd rust && cargo check --package amanclaw-cli`

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): implement skill list-installed and info commands"
```

---

### Task 3: Implement remove handler

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Implement remove**

```rust
SkillAction::Remove { name, plugins_dir } => {
    let registry = amanclaw_registry::local::SkillRegistry::new(&format!("{plugins_dir}/registry")).await?;
    match registry.get(&name).await? {
        Some(skill) => {
            // Remove files
            let skill_path = std::path::Path::new(&skill.install_dir);
            if skill_path.exists() {
                std::fs::remove_dir_all(skill_path)
                    .or_else(|_| std::fs::remove_file(skill_path))
                    .ok();
            }
            // Remove from registry
            registry.uninstall(&name).await?;
            println!("Removed skill: {name}");
        }
        None => {
            // Try removing the file directly from plugins dir
            let py_path = format!("{plugins_dir}/skill_{name}.py");
            let wasm_path = format!("{plugins_dir}/{name}.wasm");
            if std::path::Path::new(&py_path).exists() {
                std::fs::remove_file(&py_path)?;
                println!("Removed plugin file: {py_path}");
            } else if std::path::Path::new(&wasm_path).exists() {
                std::fs::remove_file(&wasm_path)?;
                println!("Removed plugin file: {wasm_path}");
            } else {
                println!("Skill '{name}' not found.");
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): implement skill remove command"
```

---

### Task 4: Implement update handler

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Implement update**

```rust
SkillAction::Update { name, plugins_dir } => {
    let registry = amanclaw_registry::local::SkillRegistry::new(&format!("{plugins_dir}/registry")).await?;
    let client = amanclaw_skill_index::client::IndexClient::default();
    let index = client.fetch().await?;

    let skills_to_update = if name == "all" {
        registry.list_installed().await?
    } else {
        match registry.get(&name).await? {
            Some(s) => vec![s],
            None => {
                println!("Skill '{name}' not installed.");
                return Ok(());
            }
        }
    };

    let mut updated = 0;
    for installed in &skills_to_update {
        if let Some(remote) = index.find(&installed.name) {
            if remote.version != installed.version {
                println!("Updating {} {} → {}...", installed.name, installed.version, remote.version);
                // Re-install (download latest)
                skill_installer::install_skill(&remote.repo, &plugins_dir).await?;
                updated += 1;
            }
        }
    }

    if updated == 0 {
        println!("All skills are up to date.");
    } else {
        println!("\n{updated} skill(s) updated.");
    }
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): implement skill update command"
```

---

## Chunk 2: Checksum Verification

### Task 5: Add SHA256 verification to installer

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/skill_installer.rs`
- Modify: `rust/crates/amanclaw-cli/Cargo.toml`

- [ ] **Step 1: Add sha2 dependency**

In `Cargo.toml`:
```toml
sha2 = "0.10"
```

- [ ] **Step 2: Add checksum calculation after download**

In `skill_installer.rs`, after downloading a file:

```rust
use sha2::{Sha256, Digest};

fn calculate_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
```

Call this after downloading and store the checksum when registering in the local registry.

- [ ] **Step 3: Register installed skills in local registry**

After writing the file to disk, add:

```rust
// Register in local registry if available
if let Ok(registry) = amanclaw_registry::local::SkillRegistry::new(&format!("{plugins_dir}/registry")).await {
    let installed = amanclaw_registry::local::InstalledSkill {
        name: skill_name.clone(),
        version: version.clone(),
        skill_type: if file_name.ends_with(".wasm") { "wasm" } else { "script" }.into(),
        description: Some(description.clone()),
        entry: Some(file_name.clone()),
        install_dir: plugins_dir.to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        checksum: Some(calculate_checksum(&file_bytes)),
    };
    registry.install(installed).await.ok();
}
```

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-cli/src/skill_installer.rs rust/crates/amanclaw-cli/Cargo.toml
git commit -m "feat(cli): add SHA256 checksum verification and registry tracking for installed skills"
```

---

## Chunk 3: Version Pinning

### Task 6: Support version specifier in install command

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs`
- Modify: `rust/crates/amanclaw-cli/src/skill_installer.rs`

- [ ] **Step 1: Update Install command to accept version**

In `cli.rs`, update the Install variant:

```rust
Install {
    /// Skill name, optionally with version (e.g., "skill-solat@1.2.3")
    name: String,
    /// Custom plugins directory
    #[arg(long, default_value = "plugins")]
    plugins_dir: String,
},
```

- [ ] **Step 2: Parse version from name in installer**

In `skill_installer.rs`, add:

```rust
fn parse_name_version(input: &str) -> (&str, Option<&str>) {
    if let Some((name, version)) = input.rsplit_once('@') {
        (name, Some(version))
    } else {
        (input, None)
    }
}
```

Use this when resolving which release to download — if a version is specified, fetch that specific tag instead of latest.

- [ ] **Step 3: Add test**

```rust
#[test]
fn test_parse_name_version() {
    assert_eq!(parse_name_version("skill-solat"), ("skill-solat", None));
    assert_eq!(parse_name_version("skill-solat@1.2.3"), ("skill-solat", Some("1.2.3")));
}
```

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-cli/src/cli.rs rust/crates/amanclaw-cli/src/skill_installer.rs
git commit -m "feat(cli): support version pinning in skill install (name@version)"
```

---

## Summary

| Task | Description | Steps |
|------|-------------|-------|
| 1 | Add new SkillAction variants + tests | 4 |
| 2 | Implement list-installed and info | 3 |
| 3 | Implement remove | 2 |
| 4 | Implement update | 2 |
| 5 | SHA256 checksum verification | 4 |
| 6 | Version pinning (name@version) | 4 |

**Total: 6 tasks, 19 steps**

After completing this plan:
```bash
amanclaw skill list-installed           # Show installed marketplace skills
amanclaw skill info web_search          # Details about a skill
amanclaw skill install web_search@1.2.3 # Install specific version
amanclaw skill update all               # Update all skills
amanclaw skill remove web_search        # Uninstall a skill
```
