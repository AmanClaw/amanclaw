use anyhow::{Context, Result, bail};
use sha2::{Sha256, Digest};
use std::path::Path;

fn calculate_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Download a skill's latest release artifact from GitHub.
pub async fn install_skill(
    repo: &str,
    skill_name: &str,
    lang: &str,
    plugins_dir: &Path,
) -> Result<()> {
    install_skill_version(repo, skill_name, lang, plugins_dir, None).await
}

/// Download a specific version of a skill from GitHub.
pub async fn install_skill_version(
    repo: &str,
    skill_name: &str,
    lang: &str,
    plugins_dir: &Path,
    version: Option<&str>,
) -> Result<()> {
    std::fs::create_dir_all(plugins_dir)
        .with_context(|| format!("Failed to create {}", plugins_dir.display()))?;

    let http = reqwest::Client::builder()
        .user_agent("amanclaw-cli")
        .build()?;

    let api_url = if let Some(ver) = version {
        // Try tag formats: v1.2.3 and 1.2.3
        format!("https://api.github.com/repos/{repo}/releases/tags/v{ver}")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    };

    let resp = http
        .get(&api_url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch release info for {repo}"))?;

    // If v-prefixed tag failed, try without v prefix
    let resp = if !resp.status().is_success() && version.is_some() {
        let ver = version.unwrap();
        let fallback_url = format!("https://api.github.com/repos/{repo}/releases/tags/{ver}");
        let fallback_resp = http
            .get(&fallback_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch release info for {repo}"))?;
        if fallback_resp.status().is_success() {
            fallback_resp
        } else {
            let status = fallback_resp.status();
            let body = fallback_resp.text().await.unwrap_or_default();
            bail!("GitHub API returned {status} for {repo} version {ver}: {body}");
        }
    } else if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("GitHub API returned {status} for {repo}: {body}");
    } else {
        resp
    };

    let release: serde_json::Value = resp.json().await?;
    let release_version = release["tag_name"]
        .as_str()
        .unwrap_or("0.0.0")
        .trim_start_matches('v')
        .to_string();
    let assets = release["assets"]
        .as_array()
        .context("No assets in release")?;

    let target_ext = match lang {
        "rust" => ".wasm",
        "python" => ".py",
        _ => bail!("Unsupported skill language: {lang}"),
    };

    let mut downloaded = 0;
    let mut primary_checksum: Option<String> = None;
    let mut primary_entry: Option<String> = None;
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        let download_url = asset["browser_download_url"].as_str().unwrap_or("");

        if name.ends_with(target_ext) || name.ends_with(".toml") {
            let dest = plugins_dir.join(name);
            let bytes = http.get(download_url).send().await?.bytes().await?;

            // Calculate checksum for primary artifact (not .toml files)
            if name.ends_with(target_ext) {
                let checksum = calculate_checksum(&bytes);
                println!("  SHA256: {checksum}");
                primary_checksum = Some(checksum);
                primary_entry = Some(name.to_string());
            }

            std::fs::write(&dest, &bytes)
                .with_context(|| format!("Failed to write {}", dest.display()))?;
            println!("  Downloaded: {name}");
            downloaded += 1;
        }
    }

    if downloaded == 0 {
        bail!("No {target_ext} assets found in latest release of {repo}");
    }

    // Register in local registry
    let registry_dir = format!("{}/registry", plugins_dir.display());
    if let Ok(registry) = crate::open_skill_registry(&registry_dir).await {
        let skill_type = if target_ext == ".wasm" { "wasm" } else { "script" };
        let installed = amanclaw_registry::local::InstalledSkill {
            name: skill_name.to_string(),
            version: release_version.clone(),
            skill_type: skill_type.into(),
            description: None,
            entry: primary_entry,
            install_dir: plugins_dir.to_string_lossy().into(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            checksum: primary_checksum,
        };
        // Use raw SQL insert to register
        sqlx::query(
            "INSERT OR REPLACE INTO installed_skills (name, version, skill_type, description, entry, install_dir, installed_at, checksum) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&installed.name)
        .bind(&installed.version)
        .bind(&installed.skill_type)
        .bind(&installed.description)
        .bind(&installed.entry)
        .bind(&installed.install_dir)
        .bind(&installed.installed_at)
        .bind(&installed.checksum)
        .execute(registry.pool())
        .await
        .ok();
    }

    println!(
        "Installed {skill_name} v{release_version} ({downloaded} file(s)) to {}",
        plugins_dir.display()
    );
    Ok(())
}

/// Parse a skill name with optional version specifier (e.g., "skill-solat@1.2.3").
pub fn parse_name_version(input: &str) -> (&str, Option<&str>) {
    if let Some((name, version)) = input.rsplit_once('@') {
        (name, Some(version))
    } else {
        (input, None)
    }
}

/// Resolve a skill name to a repo path.
pub fn resolve_repo(name: &str) -> String {
    if name.contains('/') {
        name.to_string()
    } else {
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

    #[test]
    fn test_parse_name_version() {
        assert_eq!(parse_name_version("skill-solat"), ("skill-solat", None));
        assert_eq!(parse_name_version("skill-solat@1.2.3"), ("skill-solat", Some("1.2.3")));
    }

    #[test]
    fn test_calculate_checksum() {
        let data = b"hello world";
        let checksum = calculate_checksum(data);
        // SHA256 of "hello world"
        assert_eq!(
            checksum,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
