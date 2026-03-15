//! Invite code generation and validation.

use anyhow::Result;
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invite {
    pub code: String,
    pub email: String,
    pub used_by: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Generate a random 8-character invite code.
fn generate_code() -> String {
    let mut rng = rand::rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..8).map(|_| chars[rng.random_range(0..chars.len())]).collect()
}

/// Create a new invite code.
pub async fn create_invite(pool: &SqlitePool, email: &str, days_valid: i64) -> Result<Invite> {
    let code = generate_code();
    let now = Utc::now();
    let expires = now + Duration::days(days_valid);

    sqlx::query(
        "INSERT INTO invites (code, email, used_by, created_at, expires_at) VALUES (?, ?, '', ?, ?)",
    )
    .bind(&code)
    .bind(email)
    .bind(now.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(Invite {
        code,
        email: email.to_string(),
        used_by: String::new(),
        created_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    })
}

/// Validate an invite code. Returns the invite if valid and unused.
pub async fn validate_invite(pool: &SqlitePool, code: &str) -> Result<Option<Invite>> {
    let invite = sqlx::query_as::<_, Invite>(
        "SELECT code, email, used_by, created_at, expires_at FROM invites WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    match invite {
        Some(inv) if !inv.used_by.is_empty() => Ok(None), // Already used
        Some(inv) => {
            // Check expiration
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&inv.expires_at) {
                if Utc::now() > expires {
                    return Ok(None); // Expired
                }
            }
            Ok(Some(inv))
        }
        None => Ok(None),
    }
}

/// Mark an invite as used.
pub async fn use_invite(pool: &SqlitePool, code: &str, user_id: &str) -> Result<()> {
    sqlx::query("UPDATE invites SET used_by = ? WHERE code = ?")
        .bind(user_id)
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all invites.
pub async fn list_invites(pool: &SqlitePool) -> Result<Vec<Invite>> {
    let invites = sqlx::query_as::<_, Invite>(
        "SELECT code, email, used_by, created_at, expires_at FROM invites ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(invites)
}

/// Revoke (delete) an invite.
pub async fn revoke_invite(pool: &SqlitePool, code: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM invites WHERE code = ?")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(crate::schema::INIT_SQL)
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        assert_eq!(invite.code.len(), 8);
        assert_eq!(invite.email, "user@example.com");
        assert!(invite.used_by.is_empty());
    }

    #[tokio::test]
    async fn test_validate_unused_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        let valid = validate_invite(&pool, &invite.code).await.unwrap();
        assert!(valid.is_some());
    }

    #[tokio::test]
    async fn test_validate_used_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        use_invite(&pool, &invite.code, "user-123").await.unwrap();
        let valid = validate_invite(&pool, &invite.code).await.unwrap();
        assert!(valid.is_none());
    }

    #[tokio::test]
    async fn test_validate_nonexistent_invite() {
        let pool = setup_pool().await;
        let valid = validate_invite(&pool, "INVALID").await.unwrap();
        assert!(valid.is_none());
    }

    #[tokio::test]
    async fn test_list_invites() {
        let pool = setup_pool().await;
        create_invite(&pool, "a@example.com", 7).await.unwrap();
        create_invite(&pool, "b@example.com", 7).await.unwrap();
        let list = list_invites(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_revoke_invite() {
        let pool = setup_pool().await;
        let invite = create_invite(&pool, "user@example.com", 7).await.unwrap();
        assert!(revoke_invite(&pool, &invite.code).await.unwrap());
        let valid = validate_invite(&pool, &invite.code).await.unwrap();
        assert!(valid.is_none());
    }
}
