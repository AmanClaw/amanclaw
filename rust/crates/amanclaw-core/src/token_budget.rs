/// Simple token estimation. 1 token ~ 4 characters (conservative).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Manages a token budget for context building.
pub struct TokenBudget {
    max_tokens: usize,
    used: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens, used: 0 }
    }

    /// Try to reserve tokens. Returns true if fits.
    pub fn reserve(&mut self, text: &str) -> bool {
        let cost = estimate_tokens(text);
        if self.used + cost <= self.max_tokens {
            self.used += cost;
            true
        } else {
            false
        }
    }

    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.used)
    }

    /// Force-reserve (for must-include content like system prompt).
    pub fn force_reserve(&mut self, text: &str) {
        self.used += estimate_tokens(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        // Empty string
        assert_eq!(estimate_tokens(""), 0);
        // 4 chars = 1 token
        assert_eq!(estimate_tokens("abcd"), 1);
        // 5 chars = 2 tokens
        assert_eq!(estimate_tokens("abcde"), 2);
        // 8 chars = 2 tokens
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        // 1 char = 1 token (rounds up)
        assert_eq!(estimate_tokens("a"), 1);
    }

    #[test]
    fn test_reserve_within_budget() {
        let mut budget = TokenBudget::new(10);
        // "hello world" = 11 chars => 3 tokens
        assert!(budget.reserve("hello world"));
        assert_eq!(budget.remaining(), 7);
    }

    #[test]
    fn test_reserve_overflow() {
        let mut budget = TokenBudget::new(2);
        // 12 chars => 3 tokens, exceeds budget of 2
        assert!(!budget.reserve("twelve chars"));
        // Nothing was consumed
        assert_eq!(budget.remaining(), 2);
    }

    #[test]
    fn test_reserve_exact_fit() {
        let mut budget = TokenBudget::new(2);
        // 8 chars => 2 tokens, exactly fits
        assert!(budget.reserve("abcdefgh"));
        assert_eq!(budget.remaining(), 0);
        // No more room
        assert!(!budget.reserve("x"));
    }

    #[test]
    fn test_force_reserve() {
        let mut budget = TokenBudget::new(2);
        // Force-reserve beyond budget
        budget.force_reserve("abcdefghijkl"); // 12 chars => 3 tokens
        assert_eq!(budget.remaining(), 0); // saturating_sub
    }

    #[test]
    fn test_multiple_reserves() {
        let mut budget = TokenBudget::new(10);
        assert!(budget.reserve("abcd")); // 1 token
        assert!(budget.reserve("abcdefgh")); // 2 tokens
        assert_eq!(budget.remaining(), 7);
        // Try to add 30 tokens worth
        assert!(!budget.reserve(&"x".repeat(120)));
        assert_eq!(budget.remaining(), 7); // unchanged
    }
}
