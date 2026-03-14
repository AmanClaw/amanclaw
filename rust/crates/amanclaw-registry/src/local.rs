use crate::manifest::SkillManifest;
use anyhow::Result;
use sqlx::SqlitePool;

pub const REGISTRY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS installed_skills (
    name        TEXT PRIMARY KEY,
    version     TEXT NOT NULL,
    skill_type  TEXT NOT NULL DEFAULT 'wasm',
    description TEXT,
    entry       TEXT,
    install_dir TEXT NOT NULL,
    installed_at TEXT NOT NULL DEFAULT (datetime('now')),
    checksum    TEXT
);

CREATE TABLE IF NOT EXISTS skill_dependencies (
    skill_name  TEXT NOT NULL,
    dep_name    TEXT NOT NULL,
    dep_version TEXT NOT NULL,
    optional    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (skill_name, dep_name),
    FOREIGN KEY (skill_name) REFERENCES installed_skills(name) ON DELETE CASCADE
);
"#;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct InstalledSkill {
    pub name: String,
    pub version: String,
    pub skill_type: String,
    pub description: Option<String>,
    pub entry: Option<String>,
    pub install_dir: String,
    pub installed_at: String,
    pub checksum: Option<String>,
}

pub struct SkillRegistry {
    pool: SqlitePool,
    skills_dir: String,
}

impl SkillRegistry {
    pub async fn new(pool: SqlitePool, skills_dir: String) -> Result<Self> {
        sqlx::raw_sql(REGISTRY_SCHEMA).execute(&pool).await?;
        Ok(Self { pool, skills_dir })
    }

    pub async fn install_from_path(&self, path: &std::path::Path) -> Result<InstalledSkill> {
        let manifest_path = path.join("amanclaw-skill.toml");
        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest = SkillManifest::from_toml(&content)?;

        let install_dir = std::path::Path::new(&self.skills_dir).join(&manifest.name);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir)?;
        }
        copy_dir_recursive(path, &install_dir)?;

        let installed = InstalledSkill {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            skill_type: manifest.skill_type.clone(),
            description: manifest.description.clone(),
            entry: manifest.entry.clone(),
            install_dir: install_dir.to_string_lossy().into(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            checksum: None,
        };

        sqlx::query(
            "INSERT OR REPLACE INTO installed_skills (name, version, skill_type, description, entry, install_dir, installed_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&installed.name)
        .bind(&installed.version)
        .bind(&installed.skill_type)
        .bind(&installed.description)
        .bind(&installed.entry)
        .bind(&installed.install_dir)
        .bind(&installed.installed_at)
        .execute(&self.pool)
        .await?;

        // Insert dependencies
        for (dep_name, dep_spec) in &manifest.dependencies {
            let (version, optional) = match dep_spec {
                crate::manifest::DependencySpec::Version(v) => (v.clone(), false),
                crate::manifest::DependencySpec::Detailed { version, optional } => {
                    (version.clone(), *optional)
                }
            };
            sqlx::query(
                "INSERT OR REPLACE INTO skill_dependencies (skill_name, dep_name, dep_version, optional) VALUES (?, ?, ?, ?)"
            )
            .bind(&manifest.name)
            .bind(dep_name)
            .bind(&version)
            .bind(optional)
            .execute(&self.pool)
            .await?;
        }

        tracing::info!(name = %manifest.name, version = %manifest.version, "Skill installed");
        Ok(installed)
    }

    pub async fn uninstall(&self, name: &str) -> Result<bool> {
        let result =
            sqlx::query_as::<_, InstalledSkill>("SELECT * FROM installed_skills WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        if let Some(skill) = result {
            let dir = std::path::Path::new(&skill.install_dir);
            if dir.exists() {
                std::fs::remove_dir_all(dir)?;
            }
            sqlx::query("DELETE FROM installed_skills WHERE name = ?")
                .bind(name)
                .execute(&self.pool)
                .await?;
            tracing::info!(name, "Skill uninstalled");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn list_installed(&self) -> Result<Vec<InstalledSkill>> {
        let skills =
            sqlx::query_as::<_, InstalledSkill>("SELECT * FROM installed_skills ORDER BY name")
                .fetch_all(&self.pool)
                .await?;
        Ok(skills)
    }

    pub async fn search_installed(&self, query: &str) -> Result<Vec<InstalledSkill>> {
        let pattern = format!("%{query}%");
        let skills = sqlx::query_as::<_, InstalledSkill>(
            "SELECT * FROM installed_skills WHERE name LIKE ? OR description LIKE ? ORDER BY name",
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(skills)
    }

    pub async fn get(&self, name: &str) -> Result<Option<InstalledSkill>> {
        let skill =
            sqlx::query_as::<_, InstalledSkill>("SELECT * FROM installed_skills WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;
        Ok(skill)
    }
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::raw_sql(REGISTRY_SCHEMA).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_install_and_list() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        // Create a skill package
        let pkg_dir = tmp.path().join("weather-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            r#"
name = "weather"
version = "1.0.0"
description = "Weather forecasts"
type = "script"
entry = "main.py"
"#,
        )
        .unwrap();
        std::fs::write(pkg_dir.join("main.py"), "print('hello')").unwrap();

        let installed = registry.install_from_path(&pkg_dir).await.unwrap();
        assert_eq!(installed.name, "weather");
        assert_eq!(installed.version, "1.0.0");

        let all = registry.list_installed().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "weather");

        // Verify files were copied
        let installed_dir = skills_dir.join("weather");
        assert!(installed_dir.join("amanclaw-skill.toml").exists());
        assert!(installed_dir.join("main.py").exists());
    }

    #[tokio::test]
    async fn test_uninstall() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let pkg_dir = tmp.path().join("test-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            r#"
name = "test-skill"
version = "0.1.0"
"#,
        )
        .unwrap();

        registry.install_from_path(&pkg_dir).await.unwrap();
        assert!(registry.uninstall("test-skill").await.unwrap());
        assert!(!registry.uninstall("test-skill").await.unwrap()); // already removed

        let all = registry.list_installed().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_search() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        for (name, desc) in [("weather", "Weather data"), ("calendar", "Calendar events")] {
            let pkg_dir = tmp.path().join(format!("{name}-pkg"));
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("amanclaw-skill.toml"),
                format!("name = \"{name}\"\nversion = \"1.0.0\"\ndescription = \"{desc}\""),
            )
            .unwrap();
            registry.install_from_path(&pkg_dir).await.unwrap();
        }

        let results = registry.search_installed("weather").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "weather");

        let results = registry.search_installed("events").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "calendar");
    }

    #[tokio::test]
    async fn test_get_existing() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let pkg_dir = tmp.path().join("test-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            "name = \"test-skill\"\nversion = \"0.1.0\"\ndescription = \"A test\"",
        )
        .unwrap();
        registry.install_from_path(&pkg_dir).await.unwrap();

        let skill = registry.get("test-skill").await.unwrap().unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.version, "0.1.0");
        assert_eq!(skill.description.as_deref(), Some("A test"));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let result = registry.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_reinstall_overwrites() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let pkg_dir = tmp.path().join("test-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            "name = \"test-skill\"\nversion = \"1.0.0\"",
        )
        .unwrap();
        registry.install_from_path(&pkg_dir).await.unwrap();

        // Reinstall with new version
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            "name = \"test-skill\"\nversion = \"2.0.0\"",
        )
        .unwrap();
        let installed = registry.install_from_path(&pkg_dir).await.unwrap();
        assert_eq!(installed.version, "2.0.0");

        let all = registry.list_installed().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].version, "2.0.0");
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let results = registry.search_installed("nonexistent").await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_install_with_dependencies() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool.clone(), skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let pkg_dir = tmp.path().join("dep-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            r#"
name = "with-deps"
version = "1.0.0"

[dependencies]
http-client = "0.2"
parser = { version = "1.0", optional = true }
"#,
        )
        .unwrap();

        let installed = registry.install_from_path(&pkg_dir).await.unwrap();
        assert_eq!(installed.name, "with-deps");

        // Verify dependencies were stored
        let deps = sqlx::query("SELECT dep_name, dep_version, optional FROM skill_dependencies WHERE skill_name = 'with-deps' ORDER BY dep_name")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(deps.len(), 2);

        let dep1_name: String = sqlx::Row::get(&deps[0], "dep_name");
        let dep2_name: String = sqlx::Row::get(&deps[1], "dep_name");
        assert_eq!(dep1_name, "http-client");
        assert_eq!(dep2_name, "parser");

        let dep2_optional: bool = sqlx::Row::get(&deps[1], "optional");
        assert!(dep2_optional);
    }

    #[tokio::test]
    async fn test_list_installed_empty() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let all = registry.list_installed().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_install_copies_subdirectories() {
        let pool = test_pool().await;
        let tmp = tempfile::tempdir().unwrap();
        let skills_dir = tmp.path().join("installed");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let registry = SkillRegistry::new(pool, skills_dir.to_string_lossy().into())
            .await
            .unwrap();

        let pkg_dir = tmp.path().join("complex-pkg");
        std::fs::create_dir_all(pkg_dir.join("src")).unwrap();
        std::fs::write(
            pkg_dir.join("amanclaw-skill.toml"),
            "name = \"complex\"\nversion = \"1.0.0\"",
        )
        .unwrap();
        std::fs::write(pkg_dir.join("src/main.py"), "print('hello')").unwrap();

        registry.install_from_path(&pkg_dir).await.unwrap();

        let installed_main = skills_dir.join("complex/src/main.py");
        assert!(installed_main.exists());
    }
}
