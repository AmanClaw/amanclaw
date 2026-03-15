use amanclaw_llm::client::{LlmClient, LlmResponse};
use amanclaw_memory::knowledge_store::DetectedCorrection;
use anyhow::Result;

/// System prompt instructing the LLM to extract corrections from a user-bot exchange.
const DETECTION_PROMPT: &str = r#"You are a correction-detection engine for a conversational AI assistant.

Analyze the provided user-bot exchange and identify any corrections the user is making to the bot's responses.

Output a JSON array of correction objects. Each object must have:
- "trigger": string — the key phrase or topic that would trigger this correction in future
- "wrong_response": string or null — what the bot said that was wrong (null if unknown)
- "correct_response": string — what the correct answer should be
- "topic": string or null — the subject area (e.g. "solat", "fiqh", "quran"), or null if unclear
- "confidence": number between 0.0 and 1.0 — how confident you are this is a correction signal
- "signal_type": string — one of: "explicit_correction", "implicit_correction", "negation", "clarification", "affirmation_of_mistake"

If there are no corrections in the exchange, output an empty array: []

Output ONLY valid JSON. No markdown fences, no explanation, no commentary."#;

/// Detect corrections in a user-bot exchange using an LLM.
///
/// Returns a list of detected corrections extracted from the conversation.
/// The `history_context` is a slice of (user, bot) message pairs preceding the current exchange.
pub async fn detect_corrections(
    llm: &LlmClient,
    user_message: &str,
    bot_response: &str,
    history_context: &[(&str, &str)],
) -> Result<Vec<DetectedCorrection>> {
    let mut exchange = String::new();

    for (user, bot) in history_context {
        exchange.push_str(&format!("User: {user}\nBot: {bot}\n\n"));
    }
    exchange.push_str(&format!("User: {user_message}\nBot: {bot_response}\n"));

    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": DETECTION_PROMPT
        }),
        serde_json::json!({
            "role": "user",
            "content": exchange
        }),
    ];

    let response = llm.call(&messages, &[]).await?;

    let text = match response {
        LlmResponse::Text(t) => t,
        LlmResponse::ToolCalls(_) => {
            tracing::debug!("Detection LLM returned tool calls instead of text; returning empty");
            return Ok(vec![]);
        }
    };

    parse_corrections(&text)
}

/// Parse a JSON array of corrections from LLM output text.
///
/// Strips markdown fences if present and gracefully returns an empty vec on parse failure.
fn parse_corrections(text: &str) -> Result<Vec<DetectedCorrection>> {
    let trimmed = text.trim();

    // Strip markdown fences if present (```json ... ``` or ``` ... ```)
    let json_str = if trimmed.starts_with("```") {
        let after_fence = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```");
        // Find the closing fence
        if let Some(end) = after_fence.rfind("```") {
            after_fence[..end].trim()
        } else {
            after_fence.trim()
        }
    } else {
        trimmed
    };

    match serde_json::from_str::<Vec<DetectedCorrection>>(json_str) {
        Ok(corrections) => Ok(corrections),
        Err(e) => {
            tracing::debug!(error = %e, raw = %json_str, "Failed to parse corrections JSON; returning empty vec");
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_corrections_valid_json() {
        let json = r#"[{"trigger":"subuh time","wrong_response":"5:00 AM","correct_response":"5:30 AM","topic":"solat","confidence":0.9,"signal_type":"explicit_correction"}]"#;
        let result = parse_corrections(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].trigger, "subuh time");
        assert_eq!(result[0].wrong_response.as_deref(), Some("5:00 AM"));
        assert_eq!(result[0].correct_response, "5:30 AM");
        assert_eq!(result[0].topic.as_deref(), Some("solat"));
        assert!((result[0].confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(result[0].signal_type, "explicit_correction");
    }

    #[test]
    fn test_parse_corrections_empty_array() {
        let result = parse_corrections("[]").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_corrections_markdown_fenced() {
        let json = "```json\n[{\"trigger\":\"qiblat direction\",\"wrong_response\":null,\"correct_response\":\"270 degrees\",\"topic\":\"qiblat\",\"confidence\":0.8,\"signal_type\":\"explicit_correction\"}]\n```";
        let result = parse_corrections(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].trigger, "qiblat direction");
        assert!(result[0].wrong_response.is_none());
        assert_eq!(result[0].correct_response, "270 degrees");
    }

    #[test]
    fn test_parse_corrections_invalid_json_returns_empty() {
        let garbage = "This is not JSON at all! The LLM went off script.";
        let result = parse_corrections(garbage).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_corrections_multiple() {
        let json = r#"[
            {
                "trigger": "zakat fitrah amount",
                "wrong_response": "RM5",
                "correct_response": "RM7",
                "topic": "zakat",
                "confidence": 0.95,
                "signal_type": "explicit_correction"
            },
            {
                "trigger": "asr prayer time",
                "wrong_response": null,
                "correct_response": "Asr starts at 4:15 PM",
                "topic": "solat",
                "confidence": 0.75,
                "signal_type": "implicit_correction"
            }
        ]"#;
        let result = parse_corrections(json).unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].trigger, "zakat fitrah amount");
        assert_eq!(result[0].wrong_response.as_deref(), Some("RM5"));
        assert_eq!(result[0].correct_response, "RM7");
        assert_eq!(result[0].topic.as_deref(), Some("zakat"));
        assert!((result[0].confidence - 0.95).abs() < f64::EPSILON);
        assert_eq!(result[0].signal_type, "explicit_correction");

        assert_eq!(result[1].trigger, "asr prayer time");
        assert!(result[1].wrong_response.is_none());
        assert_eq!(result[1].correct_response, "Asr starts at 4:15 PM");
        assert_eq!(result[1].topic.as_deref(), Some("solat"));
        assert!((result[1].confidence - 0.75).abs() < f64::EPSILON);
        assert_eq!(result[1].signal_type, "implicit_correction");
    }
}
