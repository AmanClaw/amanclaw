use crate::learning::detection::detect_corrections;
use amanclaw_llm::client::LlmClient;
use amanclaw_memory::knowledge_store::{CorrectionEvent, KnowledgeStore};
use amanclaw_traits::memory::MemoryBackend;

/// Run correction detection as a background task.
/// Called from PersistMiddleware after the exchange is complete.
#[allow(clippy::too_many_arguments)]
pub async fn detect_and_store_corrections(
    store: &KnowledgeStore,
    llm: &LlmClient,
    memory: &dyn MemoryBackend,
    user_id: &str,
    platform: &str,
    ns: &str,
    user_message: &str,
    bot_response: &str,
) {
    // 1. Get recent history for context (last 4 messages = 2 exchanges)
    let history = match memory.get_history(ns, user_id, 4).await {
        Ok(h) => h,
        Err(_) => return,
    };

    // 2. Convert to (&str, &str) pairs
    let history_pairs: Vec<(&str, &str)> = history
        .iter()
        .map(|m| (m.role.as_str(), m.content.as_str()))
        .collect();

    // 3. Call detection
    let corrections =
        match detect_corrections(llm, user_message, bot_response, &history_pairs).await {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(error = %e, "Correction detection failed");
                return;
            }
        };

    // 4. Store each correction with confidence >= 0.4
    for correction in &corrections {
        if correction.confidence < 0.4 {
            continue;
        }

        let layer = "user"; // user-level for now

        match store
            .upsert_rule(correction, Some(user_id), None, layer)
            .await
        {
            Ok(rule_id) => {
                let event = CorrectionEvent {
                    rule_id: Some(rule_id),
                    user_id: user_id.to_string(),
                    platform: Some(platform.to_string()),
                    signal_type: correction.signal_type.clone(),
                    signal_confidence: correction.confidence,
                    source_user_msg: Some(user_message.to_string()),
                    source_bot_msg: Some(bot_response.to_string()),
                };
                let _ = store.log_event(&event).await;
                tracing::info!(rule_id, trigger = %correction.trigger, confidence = correction.confidence, "Learned correction");
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to store correction");
            }
        }
    }

    // 5. Promote any candidates that crossed threshold
    let _ = store.promote_candidates(0.7).await;
}
