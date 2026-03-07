use std::collections::HashMap;
use std::time::Instant;

/// Sliding window rate limiter per user.
pub struct RateLimiter {
    limit_per_minute: u32,
    windows: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(limit_per_minute: u32) -> Self {
        Self {
            limit_per_minute,
            windows: HashMap::new(),
        }
    }

    /// Check if user is within rate limit. Returns true if allowed.
    pub fn check(&mut self, user_id: &str) -> bool {
        let now = Instant::now();
        let window = self.windows.entry(user_id.to_string()).or_default();

        // Remove entries older than 60 seconds
        window.retain(|t| now.duration_since(*t).as_secs() < 60);

        if window.len() >= self.limit_per_minute as usize {
            return false;
        }

        window.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_under_limit() {
        let mut limiter = RateLimiter::new(5); // 5 per minute
        for _ in 0..5 {
            assert!(limiter.check("user1"));
        }
    }

    #[test]
    fn test_blocks_over_limit() {
        let mut limiter = RateLimiter::new(3);
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(!limiter.check("user1")); // 4th should fail
    }

    #[test]
    fn test_separate_users() {
        let mut limiter = RateLimiter::new(2);
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(!limiter.check("user1"));
        // user2 is independent
        assert!(limiter.check("user2"));
    }
}
