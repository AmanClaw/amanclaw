//! Cloud management database schema.

pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS tenants (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT UNIQUE NOT NULL,
    owner_email TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    plan        TEXT NOT NULL DEFAULT 'beta',
    created_at  TEXT NOT NULL DEFAULT '',
    last_active TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS cloud_users (
    id            TEXT PRIMARY KEY,
    email         TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id     TEXT REFERENCES tenants(id),
    role          TEXT NOT NULL DEFAULT 'owner',
    created_at    TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS invites (
    code       TEXT PRIMARY KEY,
    email      TEXT NOT NULL DEFAULT '',
    used_by    TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT '',
    expires_at TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants(status);
CREATE INDEX IF NOT EXISTS idx_cloud_users_email ON cloud_users(email);
CREATE INDEX IF NOT EXISTS idx_cloud_users_tenant ON cloud_users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_invites_email ON invites(email);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_schema_creates_all_tables() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(INIT_SQL).execute(&pool).await.unwrap();

        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        assert!(names.contains(&"tenants"));
        assert!(names.contains(&"cloud_users"));
        assert!(names.contains(&"invites"));
    }
}
