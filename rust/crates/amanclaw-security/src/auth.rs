use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum UserState {
    Admin,
    Approved,
    Pending,
    Blocked,
    New,
}

pub struct Auth {
    admin_users: HashMap<String, Vec<String>>,
    registered: HashMap<(String, String), UserState>, // (user_id, platform) -> state
}

impl Auth {
    pub fn new(admin_users: HashMap<String, Vec<String>>) -> Self {
        Self {
            admin_users,
            registered: HashMap::new(),
        }
    }

    pub fn get_user_state(&self, user_id: &str, platform: &str) -> UserState {
        // Check admin list first
        if let Some(admins) = self.admin_users.get(platform) {
            if admins.iter().any(|id| id == user_id) {
                return UserState::Admin;
            }
        }

        // Check registered users
        let key = (user_id.to_string(), platform.to_string());
        self.registered.get(&key).cloned().unwrap_or(UserState::New)
    }

    pub fn register_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.entry(key).or_insert(UserState::Pending);
    }

    pub fn approve_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Approved);
    }

    pub fn block_user(&mut self, user_id: &str, platform: &str) {
        let key = (user_id.to_string(), platform.to_string());
        self.registered.insert(key, UserState::Blocked);
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
    fn test_approve_user() {
        let mut auth = make_auth();
        assert_eq!(auth.get_user_state("55555", "telegram"), UserState::New);

        auth.register_user("55555", "telegram");
        assert_eq!(auth.get_user_state("55555", "telegram"), UserState::Pending);

        auth.approve_user("55555", "telegram");
        assert_eq!(auth.get_user_state("55555", "telegram"), UserState::Approved);
    }

    #[test]
    fn test_block_user() {
        let mut auth = make_auth();
        auth.register_user("66666", "telegram");
        auth.block_user("66666", "telegram");
        assert_eq!(auth.get_user_state("66666", "telegram"), UserState::Blocked);
    }
}
