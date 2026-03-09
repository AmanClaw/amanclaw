use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// A single WebSocket session.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub topics: HashSet<String>,
    pub connected_at: DateTime<Utc>,
    pub last_ping: Instant,
    pub authenticated: bool,
}

/// Thread-safe session manager.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new session and return its id.
    pub async fn connect(&self) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let session = Session {
            id: id.clone(),
            topics: HashSet::new(),
            connected_at: Utc::now(),
            last_ping: Instant::now(),
            authenticated: false,
        };
        self.sessions.lock().await.insert(id.clone(), session);
        id
    }

    /// Remove a session.
    pub async fn disconnect(&self, id: &str) {
        self.sessions.lock().await.remove(id);
    }

    /// Mark a session as authenticated.
    pub async fn authenticate(&self, id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(id) {
            session.authenticated = true;
            true
        } else {
            false
        }
    }

    /// Subscribe a session to a topic.
    pub async fn subscribe(&self, id: &str, topic: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(id) {
            session.topics.insert(topic.to_string());
            true
        } else {
            false
        }
    }

    /// Unsubscribe a session from a topic.
    pub async fn unsubscribe(&self, id: &str, topic: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(id) {
            session.topics.remove(topic);
            true
        } else {
            false
        }
    }

    /// Get all session ids subscribed to a topic, supporting simple glob
    /// patterns where `*` matches any sequence of characters within a
    /// dot-separated segment.
    pub async fn get_subscribers(&self, topic: &str) -> Vec<String> {
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .filter(|s| s.topics.iter().any(|pattern| glob_match(pattern, topic)))
            .map(|s| s.id.clone())
            .collect()
    }

    /// Remove sessions whose last_ping is older than `timeout`.
    pub async fn cleanup_stale(&self, timeout: Duration) {
        let mut sessions = self.sessions.lock().await;
        let now = Instant::now();
        sessions.retain(|_, s| now.duration_since(s.last_ping) < timeout);
    }

    /// Update the last_ping timestamp for a session.
    pub async fn touch(&self, id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(id) {
            session.last_ping = Instant::now();
        }
    }

    /// Return the number of active sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

/// Simple glob matching: `*` matches any characters except nothing prevents
/// cross-segment matching for a single `*`.  We use a straightforward approach
/// where each `*` in the pattern matches zero or more arbitrary characters.
fn glob_match(pattern: &str, value: &str) -> bool {
    // Split pattern by '*' and try to match segments in order.
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        // No wildcard — exact match.
        return pattern == value;
    }

    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match value[pos..].find(part) {
            Some(idx) => {
                // First segment must anchor at start.
                if i == 0 && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }
    // If pattern ends with a literal (last part is non-empty), value must end there.
    if let Some(last) = parts.last() {
        if !last.is_empty() {
            return value.ends_with(last) && pos == value.len();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_and_disconnect() {
        let mgr = SessionManager::new();
        let id = mgr.connect().await;
        assert_eq!(mgr.session_count().await, 1);
        mgr.disconnect(&id).await;
        assert_eq!(mgr.session_count().await, 0);
    }

    #[tokio::test]
    async fn subscribe_and_get_subscribers() {
        let mgr = SessionManager::new();
        let id = mgr.connect().await;
        mgr.subscribe(&id, "agent.*").await;

        let subs = mgr.get_subscribers("agent.tool_call").await;
        assert!(subs.contains(&id));

        let subs = mgr.get_subscribers("engine.status").await;
        assert!(!subs.contains(&id));
    }

    #[tokio::test]
    async fn unsubscribe() {
        let mgr = SessionManager::new();
        let id = mgr.connect().await;
        mgr.subscribe(&id, "events").await;
        assert!(!mgr.get_subscribers("events").await.is_empty());

        mgr.unsubscribe(&id, "events").await;
        assert!(mgr.get_subscribers("events").await.is_empty());
    }

    #[tokio::test]
    async fn glob_exact_match() {
        let mgr = SessionManager::new();
        let id = mgr.connect().await;
        mgr.subscribe(&id, "engine.status").await;

        let subs = mgr.get_subscribers("engine.status").await;
        assert!(subs.contains(&id));

        let subs = mgr.get_subscribers("engine.other").await;
        assert!(!subs.contains(&id));
    }

    #[tokio::test]
    async fn stale_cleanup() {
        let mgr = SessionManager::new();
        let _id = mgr.connect().await;
        assert_eq!(mgr.session_count().await, 1);

        // With a zero timeout, everything is stale.
        mgr.cleanup_stale(Duration::from_secs(0)).await;
        assert_eq!(mgr.session_count().await, 0);
    }

    #[test]
    fn glob_match_cases() {
        assert!(glob_match("agent.*", "agent.tool_call"));
        assert!(glob_match("agent.*", "agent.status"));
        assert!(!glob_match("agent.*", "engine.status"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("a*b", "aXYZb"));
        assert!(!glob_match("a*b", "aXYZc"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "other"));
    }
}
