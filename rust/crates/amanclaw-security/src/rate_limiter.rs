use dashmap::DashMap;
use std::time::Instant;

pub struct RateLimiter {
    limit_per_minute: u32,
    windows: DashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(limit_per_minute: u32) -> Self {
        Self {
            limit_per_minute,
            windows: DashMap::new(),
        }
    }

    /// Check if user is within rate limit. Thread-safe, no external lock needed.
    pub fn check(&self, user_id: &str) -> bool {
        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(60);

        let mut entry = self.windows.entry(user_id.to_string()).or_default();
        let timestamps = entry.value_mut();
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.limit_per_minute as usize {
            return false;
        }

        timestamps.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_under_limit() {
        let limiter = RateLimiter::new(5); // 5 per minute
        for _ in 0..5 {
            assert!(limiter.check("user1"));
        }
    }

    #[test]
    fn test_blocks_over_limit() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(!limiter.check("user1")); // 4th should fail
    }

    #[test]
    fn test_separate_users() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(!limiter.check("user1"));
        // user2 is independent
        assert!(limiter.check("user2"));
    }
}
