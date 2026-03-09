use anyhow::Result;
use sqlx::{Row, SqlitePool};

/// Represents a community (e.g. a WhatsApp/Telegram group).
#[derive(Debug, Clone)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub language: String,
    pub platform: String,
    pub platform_group_id: String,
    pub enabled_skills: Vec<String>,
}

/// Notification preference for a community.
#[derive(Debug, Clone)]
pub struct CommunityNotification {
    pub community_id: String,
    pub notification_type: String,
    pub enabled: bool,
}

/// Community data-access layer.
pub struct CommunityRepo<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CommunityRepo<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Create or update a community.
    pub async fn upsert(&self, community: &Community) -> Result<()> {
        let skills_json = serde_json::to_string(&community.enabled_skills)?;
        sqlx::query(
            "INSERT INTO communities (id, name, zone, language, platform, platform_group_id, enabled_skills)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                zone = excluded.zone,
                language = excluded.language,
                enabled_skills = excluded.enabled_skills"
        )
        .bind(&community.id)
        .bind(&community.name)
        .bind(&community.zone)
        .bind(&community.language)
        .bind(&community.platform)
        .bind(&community.platform_group_id)
        .bind(&skills_json)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get a community by its ID.
    pub async fn get(&self, id: &str) -> Result<Option<Community>> {
        let row = sqlx::query("SELECT * FROM communities WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(|r| self.row_to_community(&r)))
    }

    /// Find a community by platform group ID.
    pub async fn get_by_group(&self, platform: &str, group_id: &str) -> Result<Option<Community>> {
        let row =
            sqlx::query("SELECT * FROM communities WHERE platform = ? AND platform_group_id = ?")
                .bind(platform)
                .bind(group_id)
                .fetch_optional(self.pool)
                .await?;

        Ok(row.map(|r| self.row_to_community(&r)))
    }

    /// List all communities.
    pub async fn list_all(&self) -> Result<Vec<Community>> {
        let rows = sqlx::query("SELECT * FROM communities ORDER BY created_at")
            .fetch_all(self.pool)
            .await?;

        Ok(rows.iter().map(|r| self.row_to_community(r)).collect())
    }

    /// Delete a community by ID.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        sqlx::query("DELETE FROM community_notifications WHERE community_id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM community_admins WHERE community_id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        let result = sqlx::query("DELETE FROM communities WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Set a notification preference for a community.
    pub async fn set_notification(
        &self,
        community_id: &str,
        notification_type: &str,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO community_notifications (community_id, notification_type, enabled)
             VALUES (?, ?, ?)
             ON CONFLICT(community_id, notification_type) DO UPDATE SET enabled = excluded.enabled",
        )
        .bind(community_id)
        .bind(notification_type)
        .bind(enabled as i32)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get all notification preferences for a community.
    pub async fn get_notifications(
        &self,
        community_id: &str,
    ) -> Result<Vec<CommunityNotification>> {
        let rows = sqlx::query("SELECT * FROM community_notifications WHERE community_id = ?")
            .bind(community_id)
            .fetch_all(self.pool)
            .await?;

        Ok(rows
            .iter()
            .map(|r| CommunityNotification {
                community_id: r.get("community_id"),
                notification_type: r.get("notification_type"),
                enabled: r.get::<i32, _>("enabled") != 0,
            })
            .collect())
    }

    /// Add an admin to a community.
    pub async fn add_admin(&self, community_id: &str, user_id: &str, platform: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO community_admins (community_id, user_id, platform) VALUES (?, ?, ?)"
        )
        .bind(community_id)
        .bind(user_id)
        .bind(platform)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Remove an admin from a community.
    pub async fn remove_admin(&self, community_id: &str, user_id: &str) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM community_admins WHERE community_id = ? AND user_id = ?")
                .bind(community_id)
                .bind(user_id)
                .execute(self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// List admin user IDs for a community.
    pub async fn list_admins(&self, community_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT user_id FROM community_admins WHERE community_id = ?")
            .bind(community_id)
            .fetch_all(self.pool)
            .await?;

        Ok(rows.iter().map(|r| r.get("user_id")).collect())
    }

    fn row_to_community(&self, row: &sqlx::sqlite::SqliteRow) -> Community {
        let skills_json: String = row.get("enabled_skills");
        let enabled_skills: Vec<String> = serde_json::from_str(&skills_json).unwrap_or_default();

        Community {
            id: row.get("id"),
            name: row.get("name"),
            zone: row.get("zone"),
            language: row.get("language"),
            platform: row.get("platform"),
            platform_group_id: row.get("platform_group_id"),
            enabled_skills,
        }
    }
}
