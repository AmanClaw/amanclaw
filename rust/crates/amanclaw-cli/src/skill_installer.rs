use anyhow::{Context, Result, bail};
use std::path::Path;

/// Download a skill's latest release artifact from GitHub.
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

    let mut downloaded = 0;
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        let download_url = asset["browser_download_url"].as_str().unwrap_or("");

        if name.ends_with(target_ext) || name.ends_with(".toml") {
            let dest = plugins_dir.join(name);
            let bytes = http.get(download_url).send().await?.bytes().await?;
            std::fs::write(&dest, &bytes)
                .with_context(|| format!("Failed to write {}", dest.display()))?;
            println!("  Downloaded: {name}");
            downloaded += 1;
        }
    }

    if downloaded == 0 {
        bail!("No {target_ext} assets found in latest release of {repo}");
    }

    println!(
        "Installed {skill_name} ({downloaded} file(s)) to {}",
        plugins_dir.display()
    );
    Ok(())
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
}
