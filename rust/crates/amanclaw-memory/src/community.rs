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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn make_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(crate::schema::INIT_SQL)
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn sample_community(id: &str, name: &str, group_id: &str) -> Community {
        Community {
            id: id.to_string(),
            name: name.to_string(),
            zone: "WLY01".to_string(),
            language: "ms".to_string(),
            platform: "telegram".to_string(),
            platform_group_id: group_id.to_string(),
            enabled_skills: vec!["solat".to_string(), "doa".to_string()],
        }
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        let community = sample_community("c1", "Masjid KL", "group_123");

        repo.upsert(&community).await.unwrap();

        let fetched = repo.get("c1").await.unwrap().unwrap();
        assert_eq!(fetched.name, "Masjid KL");
        assert_eq!(fetched.zone, "WLY01");
        assert_eq!(fetched.enabled_skills, vec!["solat", "doa"]);
    }

    #[tokio::test]
    async fn test_get_by_group() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        let community = sample_community("c1", "Masjid KL", "group_123");
        repo.upsert(&community).await.unwrap();

        let fetched = repo
            .get_by_group("telegram", "group_123")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, "c1");
    }

    #[tokio::test]
    async fn test_get_by_group_not_found() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);

        let result = repo.get_by_group("telegram", "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_all() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);

        repo.upsert(&sample_community("c1", "Community 1", "g1"))
            .await
            .unwrap();
        repo.upsert(&sample_community("c2", "Community 2", "g2"))
            .await
            .unwrap();

        let all = repo.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        let deleted = repo.delete("c1").await.unwrap();
        assert!(deleted);

        let deleted_again = repo.delete("c1").await.unwrap();
        assert!(!deleted_again);

        let result = repo.get("c1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_upsert_updates_existing() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);

        let mut community = sample_community("c1", "Old Name", "g1");
        repo.upsert(&community).await.unwrap();

        community.name = "New Name".to_string();
        community.zone = "JHR01".to_string();
        repo.upsert(&community).await.unwrap();

        let fetched = repo.get("c1").await.unwrap().unwrap();
        assert_eq!(fetched.name, "New Name");
        assert_eq!(fetched.zone, "JHR01");
    }

    #[tokio::test]
    async fn test_notifications() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        repo.set_notification("c1", "azan", true).await.unwrap();
        repo.set_notification("c1", "khutbah", false).await.unwrap();

        let notifications = repo.get_notifications("c1").await.unwrap();
        assert_eq!(notifications.len(), 2);

        let azan = notifications
            .iter()
            .find(|n| n.notification_type == "azan")
            .unwrap();
        assert!(azan.enabled);

        let khutbah = notifications
            .iter()
            .find(|n| n.notification_type == "khutbah")
            .unwrap();
        assert!(!khutbah.enabled);
    }

    #[tokio::test]
    async fn test_notification_upsert() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        repo.set_notification("c1", "azan", true).await.unwrap();
        repo.set_notification("c1", "azan", false).await.unwrap();

        let notifications = repo.get_notifications("c1").await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(!notifications[0].enabled);
    }

    #[tokio::test]
    async fn test_admins() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        repo.add_admin("c1", "user1", "telegram").await.unwrap();
        repo.add_admin("c1", "user2", "telegram").await.unwrap();

        let admins = repo.list_admins("c1").await.unwrap();
        assert_eq!(admins.len(), 2);
        assert!(admins.contains(&"user1".to_string()));
        assert!(admins.contains(&"user2".to_string()));
    }

    #[tokio::test]
    async fn test_remove_admin() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        repo.add_admin("c1", "user1", "telegram").await.unwrap();
        let removed = repo.remove_admin("c1", "user1").await.unwrap();
        assert!(removed);

        let removed_again = repo.remove_admin("c1", "user1").await.unwrap();
        assert!(!removed_again);

        let admins = repo.list_admins("c1").await.unwrap();
        assert!(admins.is_empty());
    }

    #[tokio::test]
    async fn test_add_admin_idempotent() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        repo.add_admin("c1", "user1", "telegram").await.unwrap();
        repo.add_admin("c1", "user1", "telegram").await.unwrap();

        let admins = repo.list_admins("c1").await.unwrap();
        assert_eq!(admins.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_cascades_notifications_and_admins() {
        let pool = make_pool().await;
        let repo = CommunityRepo::new(&pool);
        repo.upsert(&sample_community("c1", "Test", "g1"))
            .await
            .unwrap();

        repo.add_admin("c1", "user1", "telegram").await.unwrap();
        repo.set_notification("c1", "azan", true).await.unwrap();

        repo.delete("c1").await.unwrap();

        // Related data should be cleaned up
        let admins = repo.list_admins("c1").await.unwrap();
        assert!(admins.is_empty());
        let notifications = repo.get_notifications("c1").await.unwrap();
        assert!(notifications.is_empty());
    }
}
