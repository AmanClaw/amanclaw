/// Islamic content guardrails — 3-layer approach:
/// 1. System prompt guardrails (set Islamic-aware tone)
/// 2. Post-processing filter (check output against sensitive topics)
/// 3. Scholarly attribution enforcement (for Islamic rulings)

/// Topics that require scholarly attribution (never present opinion as fact)
const SENSITIVE_ISLAMIC_TOPICS: &[&str] = &[
    "fatwa", "haram", "halal", "permissible", "forbidden",
    "ruling", "fiqh", "madhab", "divorce", "talaq",
    "apostasy", "jihad", "marriage", "nikah", "murtad",
];

/// Check if a response contains Islamic rulings that need attribution.
pub fn needs_scholarly_attribution(text: &str) -> bool {
    let lower = text.to_lowercase();
    SENSITIVE_ISLAMIC_TOPICS.iter().any(|topic| lower.contains(topic))
}

/// Check if a response about Islamic rulings has proper attribution.
/// Returns true if the response contains source citations.
pub fn has_attribution(text: &str) -> bool {
    let indicators = [
        "source:", "according to", "scholars", "madhab",
        "imam", "quran", "hadith", "bukhari", "muslim",
        "al-", "ibn ", "surah", "ayat",
    ];
    let lower = text.to_lowercase();
    indicators.iter().any(|ind| lower.contains(ind))
}

/// Suggest adding a disclaimer if response discusses Islamic rulings
/// without proper scholarly context.
pub fn suggest_disclaimer(text: &str) -> Option<String> {
    if needs_scholarly_attribution(text) && !has_attribution(text) {
        Some(
            "\n\n\u{26a0}\u{fe0f} For specific Islamic rulings, please consult a qualified scholar."
                .to_string(),
        )
    } else {
        None
    }
}

/// Content sensitivity categories for Islamic topics.
#[derive(Debug, PartialEq)]
pub enum SensitivityLevel {
    /// Safe — general knowledge, history, language
    Low,
    /// Moderate — requires scholarly attribution
    Moderate,
    /// High — sectarian, controversial, requires extreme care
    High,
}

/// Assess sensitivity level of a topic.
pub fn assess_sensitivity(text: &str) -> SensitivityLevel {
    let lower = text.to_lowercase();

    let high_sensitivity = [
        "sectarian",
        "shia",
        "sunni conflict",
        "takfir",
        "apostasy",
        "murtad",
    ];
    if high_sensitivity.iter().any(|t| lower.contains(t)) {
        return SensitivityLevel::High;
    }

    if needs_scholarly_attribution(&lower) {
        return SensitivityLevel::Moderate;
    }

    SensitivityLevel::Low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_attribution_for_fatwa_content() {
        assert!(needs_scholarly_attribution("This fatwa says it is haram"));
        assert!(needs_scholarly_attribution("The ruling on talaq is clear"));
        assert!(needs_scholarly_attribution("According to fiqh principles"));
    }

    #[test]
    fn test_no_attribution_for_general_content() {
        assert!(!needs_scholarly_attribution("What is the weather today?"));
        assert!(!needs_scholarly_attribution("Tell me about Malaysian food"));
        assert!(!needs_scholarly_attribution("How do I learn programming?"));
    }

    #[test]
    fn test_has_attribution_with_scholar_reference() {
        assert!(has_attribution("According to Imam Nawawi"));
        assert!(has_attribution("Source: Sahih Bukhari"));
        assert!(has_attribution("As mentioned in Surah Al-Baqarah"));
        assert!(has_attribution("Ibn Taymiyyah stated that"));
    }

    #[test]
    fn test_suggest_disclaimer_when_no_attribution() {
        let text = "This is definitely haram and forbidden in all cases";
        let disclaimer = suggest_disclaimer(text);
        assert!(disclaimer.is_some());
        assert!(disclaimer.unwrap().contains("consult a qualified scholar"));
    }

    #[test]
    fn test_no_disclaimer_when_attributed() {
        let text = "According to scholars, this fatwa indicates it is haram";
        let disclaimer = suggest_disclaimer(text);
        assert!(disclaimer.is_none());
    }

    #[test]
    fn test_sensitivity_high_for_sectarian() {
        assert_eq!(
            assess_sensitivity("The sectarian divide is deep"),
            SensitivityLevel::High
        );
        assert_eq!(
            assess_sensitivity("Shia practices differ"),
            SensitivityLevel::High
        );
        assert_eq!(
            assess_sensitivity("Takfir is a serious accusation"),
            SensitivityLevel::High
        );
        assert_eq!(
            assess_sensitivity("Discussion about apostasy and murtad"),
            SensitivityLevel::High
        );
    }

    #[test]
    fn test_sensitivity_moderate_for_fiqh() {
        assert_eq!(
            assess_sensitivity("The fiqh position on this matter"),
            SensitivityLevel::Moderate
        );
        assert_eq!(
            assess_sensitivity("Is this halal or haram?"),
            SensitivityLevel::Moderate
        );
        assert_eq!(
            assess_sensitivity("The ruling on nikah mut'ah"),
            SensitivityLevel::Moderate
        );
    }

    #[test]
    fn test_sensitivity_low_for_general() {
        assert_eq!(
            assess_sensitivity("What time is solat today?"),
            SensitivityLevel::Low
        );
        assert_eq!(
            assess_sensitivity("Tell me a dua for morning"),
            SensitivityLevel::Low
        );
        assert_eq!(
            assess_sensitivity("What is the hijri date?"),
            SensitivityLevel::Low
        );
    }
}
