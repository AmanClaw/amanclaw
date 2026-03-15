//! Cloud database operations.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::FromRow;

#[derive(Clone)]
pub struct CloudDb {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub owner_email: String,
    pub status: String,
    pub plan: String,
    pub created_at: String,
    pub last_active: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CloudUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub tenant_id: Option<String>,
    pub role: String,
    pub created_at: String,
}

impl CloudDb {
    pub async fn new(db_path: &str) -> Result<Self> {
        let url = if db_path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{db_path}?mode=rwc")
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        sqlx::raw_sql(crate::schema::INIT_SQL)
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // --- Tenant CRUD ---

    pub async fn create_tenant(&self, name: &str, slug: &str, owner_email: &str) -> Result<Tenant> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO tenants (id, name, slug, owner_email, status, plan, created_at, last_active) VALUES (?, ?, ?, ?, 'active', 'beta', ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(slug)
        .bind(owner_email)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(Tenant {
            id,
            name: name.to_string(),
            slug: slug.to_string(),
            owner_email: owner_email.to_string(),
            status: "active".to_string(),
            plan: "beta".to_string(),
            created_at: now.clone(),
            last_active: now,
        })
    }

    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, owner_email, status, plan, created_at, last_active FROM tenants WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tenant)
    }

    pub async fn get_tenant(&self, id: &str) -> Result<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, owner_email, status, plan, created_at, last_active FROM tenants WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tenant)
    }

    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let tenants = sqlx::query_as::<_, Tenant>(
            "SELECT id, name, slug, owner_email, status, plan, created_at, last_active FROM tenants ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tenants)
    }

    pub async fn update_tenant_status(&self, id: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE tenants SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn touch_tenant(&self, slug: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE tenants SET last_active = ? WHERE slug = ?")
            .bind(&now)
            .bind(slug)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- User CRUD ---

    pub async fn create_user(&self, email: &str, password_hash: &str, tenant_id: &str) -> Result<CloudUser> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO cloud_users (id, email, password_hash, tenant_id, role, created_at) VALUES (?, ?, ?, ?, 'owner', ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(password_hash)
        .bind(tenant_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(CloudUser {
            id,
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            tenant_id: Some(tenant_id.to_string()),
            role: "owner".to_string(),
            created_at: now,
        })
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<CloudUser>> {
        let user = sqlx::query_as::<_, CloudUser>(
            "SELECT id, email, password_hash, tenant_id, role, created_at FROM cloud_users WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_tenant() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let tenant = db.create_tenant("My Bot", "my-bot", "user@example.com").await.unwrap();
        assert_eq!(tenant.slug, "my-bot");
        assert_eq!(tenant.status, "active");

        let found = db.get_tenant_by_slug("my-bot").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "My Bot");
    }

    #[tokio::test]
    async fn test_tenant_not_found() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let found = db.get_tenant_by_slug("nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_tenants() {
        let db = CloudDb::new(":memory:").await.unwrap();
        db.create_tenant("Bot 1", "bot-1", "a@example.com").await.unwrap();
        db.create_tenant("Bot 2", "bot-2", "b@example.com").await.unwrap();
        let list = db.list_tenants().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_update_tenant_status() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let tenant = db.create_tenant("Bot", "bot", "a@example.com").await.unwrap();
        db.update_tenant_status(&tenant.id, "suspended").await.unwrap();
        let found = db.get_tenant(&tenant.id).await.unwrap().unwrap();
        assert_eq!(found.status, "suspended");
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let db = CloudDb::new(":memory:").await.unwrap();
        let tenant = db.create_tenant("Bot", "bot", "user@example.com").await.unwrap();
        let user = db.create_user("user@example.com", "hashed_pw", &tenant.id).await.unwrap();
        assert_eq!(user.role, "owner");

        let found = db.get_user_by_email("user@example.com").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_slug_uniqueness() {
        let db = CloudDb::new(":memory:").await.unwrap();
        db.create_tenant("Bot 1", "same-slug", "a@example.com").await.unwrap();
        let result = db.create_tenant("Bot 2", "same-slug", "b@example.com").await;
        assert!(result.is_err());
    }
}
