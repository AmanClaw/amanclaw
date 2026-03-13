use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum UserState {
    Admin,
    Approved,
    Pending,
    Blocked,
    New,
}

impl std::fmt::Display for UserState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserState::Admin => write!(f, "Admin"),
            UserState::Approved => write!(f, "Approved"),
            UserState::Pending => write!(f, "Pending"),
            UserState::Blocked => write!(f, "Blocked"),
            UserState::New => write!(f, "New"),
        }
    }
}

pub struct Auth {
    admin_users: HashMap<String, Vec<String>>,
    registered: HashMap<(String, String), UserState>,
    pool: Option<sqlx::SqlitePool>,
}

impl Auth {
    /// Create Auth without SQLite (for tests or in-memory use).
    pub fn new(admin_users: HashMap<String, Vec<String>>) -> Self {
        Self {
            admin_users,
            registered: HashMap::new(),
            pool: None,
        }
    }

    /// Create Auth backed by SQLite. Loads existing users on startup.
    pub async fn with_pool(
        admin_users: HashMap<String, Vec<String>>,
        pool: sqlx::SqlitePool,
    ) -> Self {
        let mut registered = HashMap::new();

        if let Ok(rows) = sqlx::query("SELECT user_id, platform, state FROM users")
            .fetch_all(&pool)
            .await
        {
            for row in rows {
                let uid: String = sqlx::Row::get(&row, "user_id");
                let plat: String = sqlx::Row::get(&row, "platform");
                let state_str: String = sqlx::Row::get(&row, "state");
                let state = match state_str.as_str() {
                    "approved" => UserState::Approved,
                    "blocked" => UserState::Blocked,
                    _ => UserState::Pending,
                };
                registered.insert((uid, plat), state);
            }
            tracing::info!(count = registered.len(), "Loaded users from SQLite");
        }

        Self {
            admin_users,
            registered,
            pool: Some(pool),
        }
    }

    pub fn get_user_state(&self, user_id: &str, platform: &str) -> UserState {
        if let Some(admins) = self.admin_users.get(platform)
            && admins.iter().any(|id| id == user_id)
        {
            return UserState::Admin;
        }
        let key = (user_id.to_string(), platform.to_string());
        self.registered.get(&key).cloned().unwrap_or(UserState::New)
    }

    pub fn register_user(
        &mut self,
        user_id: &str,
        platform: &str,
        username: Option<&str>,
        first_name: Option<&str>,
    ) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.entry(key).or_insert(UserState::Pending);

        if let Some(pool) = &self.pool {
            let pool = pool.clone();
            let uid = user_id.to_string();
            let plat = platform.to_string();
            let uname = username.map(|s| s.to_string());
            let fname = first_name.map(|s| s.to_string());
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "INSERT INTO users (user_id, platform, state, username, first_name)
                     VALUES (?, ?, 'pending', ?, ?)
                     ON CONFLICT(user_id, platform) DO UPDATE SET
                       username = COALESCE(excluded.username, users.username),
                       first_name = COALESCE(excluded.first_name, users.first_name),
                       last_seen = CURRENT_TIMESTAMP",
                )
                .bind(&uid)
                .bind(&plat)
                .bind(&uname)
                .bind(&fname)
                .execute(&pool)
                .await;
            });
        }
    }

    pub fn approve_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Approved);
        self.persist_state(user_id, platform, "approved");
    }

    pub fn block_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Blocked);
        self.persist_state(user_id, platform, "blocked");
    }

    pub fn unblock_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Pending);
        self.persist_state(user_id, platform, "pending");
    }

    pub fn touch_last_seen(&self, user_id: &str, platform: &str) {
        if let Some(pool) = &self.pool {
            let pool = pool.clone();
            let uid = user_id.to_string();
            let plat = platform.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE users SET last_seen = CURRENT_TIMESTAMP WHERE user_id = ? AND platform = ?",
                )
                .bind(&uid)
                .bind(&plat)
                .execute(&pool)
                .await;
            });
        }
    }

    pub fn list_users(&self) -> Vec<(String, String, UserState)> {
        self.registered
            .iter()
            .map(|((uid, plat), state)| (uid.clone(), plat.clone(), state.clone()))
            .collect()
    }

    /// Return the admin_users map (platform -> Vec<user_id>).
    pub fn admin_users(&self) -> &HashMap<String, Vec<String>> {
        &self.admin_users
    }

    /// Promote a user to admin (adds to in-memory admin list).
    pub fn make_admin(&mut self, user_id: &str, platform: &str) {
        let users = self.admin_users.entry(platform.to_string()).or_default();
        if !users.iter().any(|id| id == user_id) {
            users.push(user_id.to_string());
        }
    }

    /// Demote a user from admin (removes from in-memory admin list).
    pub fn remove_admin(&mut self, user_id: &str, platform: &str) {
        if let Some(users) = self.admin_users.get_mut(platform) {
            users.retain(|id| id != user_id);
        }
        // Set them to Approved so they still have access
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Approved);
        self.persist_state(user_id, platform, "approved");
    }

    fn persist_state(&self, user_id: &str, platform: &str, state: &str) {
        if let Some(pool) = &self.pool {
            let pool = pool.clone();
            let uid = user_id.to_string();
            let plat = platform.to_string();
            let st = state.to_string();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE users SET state = ? WHERE user_id = ? AND platform = ?",
                )
                .bind(&st)
                .bind(&uid)
                .bind(&plat)
                .execute(&pool)
                .await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth() -> Auth {
        let mut admin_users = HashMap::new();
        admin_users.insert("telegram".into(), vec!["12345".into()]);
        Auth::new(admin_users)
    }

    #[test]
    fn test_admin_user_is_authorized() {
        let auth = make_auth();
        assert_eq!(auth.get_user_state("12345", "telegram"), UserState::Admin);
    }

    #[test]
    fn test_unknown_user_is_new() {
        let auth = make_auth();
        assert_eq!(auth.get_user_state("99999", "telegram"), UserState::New);
    }

    #[test]
    fn test_register_and_approve() {
        let mut auth = make_auth();
        auth.register_user("55555", "telegram", None, None);
        assert_eq!(auth.get_user_state("55555", "telegram"), UserState::Pending);

        auth.approve_user("55555", "telegram");
        assert_eq!(
            auth.get_user_state("55555", "telegram"),
            UserState::Approved
        );
    }

    #[test]
    fn test_block_user() {
        let mut auth = make_auth();
        auth.register_user("66666", "telegram", None, None);
        auth.block_user("66666", "telegram");
        assert_eq!(auth.get_user_state("66666", "telegram"), UserState::Blocked);
    }

    #[test]
    fn test_list_users() {
        let mut auth = make_auth();
        auth.register_user("111", "telegram", Some("user1"), Some("User"));
        auth.register_user("222", "discord", None, None);
        let users = auth.list_users();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_unblock_resets_to_pending() {
        let mut auth = make_auth();
        auth.register_user("77777", "telegram", None, None);
        auth.block_user("77777", "telegram");
        auth.unblock_user("77777", "telegram");
        assert_eq!(
            auth.get_user_state("77777", "telegram"),
            UserState::Pending
        );
    }
}
