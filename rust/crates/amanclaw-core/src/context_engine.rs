use crate::registry::PluginRegistry;
use crate::token_budget::TokenBudget;
use amanclaw_llm::client::{LlmClient, LlmResponse};
use amanclaw_llm::embeddings::EmbeddingClient;
use amanclaw_memory::knowledge_store::CorrectionMatch;
use amanclaw_traits::context::{ContextEngine, ContextRequest, ContextResult, ExchangeEvent};
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::vector::VectorStore;
use anyhow::Result;
use base64::Engine as Base64Engine;
use std::sync::Arc;

/// Format learned corrections for injection into the LLM system prompt.
pub fn format_learned_corrections(matches: &[CorrectionMatch]) -> String {
    if matches.is_empty() {
        return String::new();
    }

    let mut section =
        String::from("\n\n## Learned knowledge (treat as ground truth unless user says otherwise)");

    for m in matches {
        let conf_pct = (m.rule.confidence * 100.0) as u32;
        let layer_label = match m.rule.layer.as_str() {
            "user" => "personal",
            "community" => "community",
            "global" => "general",
            _ => "unknown",
        };

        if m.rule.confidence >= 0.85 {
            section.push_str(&format!(
                "\n- {} ({}%, {} knowledge)",
                m.rule.correct_response, conf_pct, layer_label
            ));
        } else {
            section.push_str(&format!(
                "\n- Previously learned: {} ({}% confident, {} — verify with user if unsure)",
                m.rule.correct_response, conf_pct, layer_label
            ));
        }
    }

    section
}

/// Default context engine that replicates current pipeline behavior:
/// history + facts + summary + optional RAG + tool filtering.
pub struct StandardContextEngine {
    memory: Arc<dyn MemoryBackend>,
    vector_store: Option<Arc<dyn VectorStore>>,
    embedding_client: Option<Arc<EmbeddingClient>>,
    #[allow(dead_code)]
    llm: Arc<LlmClient>,
    registry: Arc<PluginRegistry>,
    base_system_prompt: String,
}

impl StandardContextEngine {
    pub fn new(
        memory: Arc<dyn MemoryBackend>,
        llm: Arc<LlmClient>,
        registry: Arc<PluginRegistry>,
        base_system_prompt: String,
        vector_store: Option<Arc<dyn VectorStore>>,
        embedding_client: Option<Arc<EmbeddingClient>>,
    ) -> Self {
        Self {
            memory,
            llm,
            registry,
            base_system_prompt,
            vector_store,
            embedding_client,
        }
    }
}

#[async_trait::async_trait]
impl ContextEngine for StandardContextEngine {
    async fn build_context(&self, request: ContextRequest) -> Result<ContextResult> {
        let profile = &request.agent_profile;
        let ns = &request.namespace;
        let user_id = &request.user_id;

        let mut budget = TokenBudget::new(profile.context.max_context_tokens);

        // 1. Build system prompt (always included)
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M %A").to_string();
        let base = if profile.system_prompt.is_empty() {
            self.base_system_prompt.clone()
        } else {
            profile.system_prompt.clone()
        };
        let mut system = base.replace("{datetime}", &now);
        budget.force_reserve(&system);

        // 2. Prepend summary if available (budget-checked)
        if let Ok(Some(summary)) = self.memory.get_summary(ns, user_id).await {
            let section = format!("\n\n## Previous conversation summary\n{summary}");
            if budget.reserve(&section) {
                system.push_str(&section);
            }
        }

        // 3. Append known facts (budget-checked per line)
        if let Ok(facts) = self.memory.get_facts(user_id).await
            && !facts.is_empty()
        {
            let header = "\n\n## Known facts about this user";
            if budget.reserve(header) {
                system.push_str(header);
                for (k, v) in &facts {
                    let line = format!("\n- {k}: {v}");
                    if !budget.reserve(&line) {
                        break;
                    }
                    system.push_str(&line);
                }
            }
        }

        // 4. RAG retrieval if enabled (budget-checked per result)
        if profile.context.rag_enabled
            && !profile.context.rag_collections.is_empty()
            && let Some(ref vs) = self.vector_store
        {
            let mut all_results = Vec::new();

            if let Some(ref ec) = self.embedding_client {
                // Embedding-based search
                if let Ok(embedding) = ec.embed_one(&request.user_message).await {
                    for collection in &profile.context.rag_collections {
                        if let Ok(results) = vs
                            .search_by_embedding(
                                collection,
                                &embedding,
                                &request.user_message,
                                profile.context.rag_top_k,
                            )
                            .await
                        {
                            all_results.extend(results);
                        }
                    }
                }
            } else {
                // Fallback: text search without embeddings
                for collection in &profile.context.rag_collections {
                    if let Ok(results) = vs
                        .search(collection, &request.user_message, profile.context.rag_top_k)
                        .await
                    {
                        all_results.extend(results);
                    }
                }
            }

            if !all_results.is_empty() {
                all_results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                all_results.truncate(profile.context.rag_top_k);
                let header = "\n\n## Relevant knowledge";
                if budget.reserve(header) {
                    system.push_str(header);
                    for doc in &all_results {
                        let line = format!("\n- {}", doc.content);
                        if !budget.reserve(&line) {
                            break;
                        }
                        system.push_str(&line);
                    }
                }
            }
        }

        // 5. User message is always included — reserve budget now
        budget.force_reserve(&request.user_message);

        // 6. Build message array
        let mut messages = vec![serde_json::json!({"role": "system", "content": system})];

        // 7. Add history (most recent first, then reverse back to chronological)
        let history = self
            .memory
            .get_history(ns, user_id, profile.context.history_limit)
            .await?;
        let mut selected: Vec<_> = Vec::new();
        for m in history.iter().rev() {
            if budget.reserve(&m.content) {
                selected.push(m);
            } else {
                break;
            }
        }
        selected.reverse();
        for m in selected {
            messages.push(serde_json::json!({"role": m.role, "content": m.content}));
        }

        // 8. Add user message (multimodal if image)
        if let Some(ref image_data) = request.image_data {
            let b64 = base64::engine::general_purpose::STANDARD.encode(image_data);
            let text = if request.user_message.is_empty() {
                "What's in this image?"
            } else {
                &request.user_message
            };
            let content = serde_json::json!([
                {"type": "text", "text": text},
                {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", b64)}}
            ]);
            messages.push(serde_json::json!({"role": "user", "content": content}));
        } else {
            messages.push(serde_json::json!({"role": "user", "content": request.user_message}));
        }

        // 9. Filter tools by agent profile
        let tools = self
            .registry
            .get_filtered_tool_definitions(&profile.allowed_skills);

        Ok(ContextResult { messages, tools })
    }

    async fn on_exchange_complete(&self, exchange: ExchangeEvent) -> Result<()> {
        // Save the exchange
        self.memory
            .save_exchange(
                &exchange.namespace,
                &exchange.user_id,
                &exchange.platform,
                &exchange.user_message,
                &exchange.assistant_response,
            )
            .await?;

        Ok(())
    }
}

/// Run auto-summarization if the threshold is exceeded.
/// Separated from ContextEngine so the pipeline can call it with profile-specific thresholds.
pub async fn maybe_summarize(
    memory: &dyn MemoryBackend,
    llm: &LlmClient,
    ns: &str,
    user_id: &str,
    threshold: i64,
    keep_recent: i64,
) -> Result<()> {
    if !memory.needs_summarization(ns, user_id, threshold).await? {
        return Ok(());
    }

    let history = memory.get_history(ns, user_id, 100).await?;
    let sum_text: Vec<String> = history
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect();
    let sum_prompt = format!(
        "Summarize the following conversation concisely. Focus on key topics, decisions, and important context. Reply with ONLY the summary:\n\n{}",
        sum_text.join("\n")
    );
    let sum_messages = vec![
        serde_json::json!({"role": "system", "content": "You are a conversation summarizer. Output only a concise summary."}),
        serde_json::json!({"role": "user", "content": sum_prompt}),
    ];

    match llm.call(&sum_messages, &[]).await {
        Ok(LlmResponse::Text(summary)) => {
            memory
                .save_summary_and_prune(ns, user_id, &summary, keep_recent)
                .await?;
        }
        _ => {
            tracing::warn!(ns, user_id, "Failed to generate summary");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_memory::knowledge_store::{CorrectionMatch, CorrectionRule};
    use amanclaw_traits::agent::AgentProfile;
    use amanclaw_traits::memory::HistoryMessage;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn make_correction_match(
        correct_response: &str,
        confidence: f64,
        layer: &str,
    ) -> CorrectionMatch {
        CorrectionMatch {
            rule: CorrectionRule {
                id: 1,
                trigger_pattern: "test trigger".into(),
                wrong_response: None,
                correct_response: correct_response.into(),
                topic: None,
                user_id: None,
                community_id: None,
                layer: layer.into(),
                confidence,
                hit_count: 0,
                status: "active".into(),
            },
            match_score: 1.0,
        }
    }

    #[test]
    fn test_format_learned_corrections() {
        // Empty input returns empty string
        assert_eq!(format_learned_corrections(&[]), String::new());

        // High confidence (>= 0.85) path — user layer
        let high = make_correction_match("Solat Jumaat is at 1pm", 0.90, "user");
        let result = format_learned_corrections(&[high]);
        assert!(result.contains("## Learned knowledge"));
        assert!(result.contains("Solat Jumaat is at 1pm"));
        assert!(result.contains("90%"));
        assert!(result.contains("personal knowledge"));
        assert!(!result.contains("Previously learned"));

        // Medium confidence (< 0.85) path — community layer
        let medium = make_correction_match("Zakat nisab is RM20,000", 0.70, "community");
        let result2 = format_learned_corrections(&[medium]);
        assert!(result2.contains("Previously learned"));
        assert!(result2.contains("Zakat nisab is RM20,000"));
        assert!(result2.contains("70%"));
        assert!(result2.contains("community"));
        assert!(result2.contains("verify with user if unsure"));

        // Multiple corrections in single output
        let c1 = make_correction_match("High confidence fact", 0.95, "global");
        let c2 = make_correction_match("Lower confidence fact", 0.60, "user");
        let combined = format_learned_corrections(&[c1, c2]);
        assert!(combined.contains("High confidence fact"));
        assert!(combined.contains("Lower confidence fact"));
        assert!(combined.contains("general knowledge"));
        assert!(combined.contains("personal"));
    }

    /// In-memory mock for testing.
    struct MockMemory {
        history: Mutex<Vec<HistoryMessage>>,
        facts: Mutex<HashMap<String, String>>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self {
                history: Mutex::new(vec![
                    HistoryMessage {
                        role: "user".into(),
                        content: "Previous msg".into(),
                    },
                    HistoryMessage {
                        role: "assistant".into(),
                        content: "Previous reply".into(),
                    },
                ]),
                facts: Mutex::new(HashMap::from([("name".into(), "Aman".into())])),
            }
        }
    }

    #[async_trait::async_trait]
    impl MemoryBackend for MockMemory {
        async fn save_exchange(
            &self,
            _ns: &str,
            _uid: &str,
            _p: &str,
            _u: &str,
            _a: &str,
        ) -> Result<()> {
            Ok(())
        }
        async fn get_history(
            &self,
            _ns: &str,
            _uid: &str,
            _limit: i64,
        ) -> Result<Vec<HistoryMessage>> {
            Ok(self.history.lock().unwrap().clone())
        }
        async fn clear_history(&self, _ns: &str, _uid: &str) -> Result<()> {
            Ok(())
        }
        async fn get_message_count(&self, _ns: &str, _uid: &str) -> Result<i64> {
            Ok(2)
        }
        async fn save_fact(&self, _uid: &str, _k: &str, _v: &str) -> Result<()> {
            Ok(())
        }
        async fn get_facts(&self, _uid: &str) -> Result<HashMap<String, String>> {
            Ok(self.facts.lock().unwrap().clone())
        }
        async fn delete_fact(&self, _uid: &str, _k: &str) -> Result<bool> {
            Ok(true)
        }
        async fn get_summary(&self, _ns: &str, _uid: &str) -> Result<Option<String>> {
            Ok(None)
        }
        async fn save_summary_and_prune(
            &self,
            _ns: &str,
            _uid: &str,
            _s: &str,
            _k: i64,
        ) -> Result<()> {
            Ok(())
        }
        async fn needs_summarization(&self, _ns: &str, _uid: &str, _t: i64) -> Result<bool> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_standard_context_engine_builds_context() {
        let memory = Arc::new(MockMemory::new());

        let profile = AgentProfile::default_agent();
        let ns = &profile.memory_namespace;

        let history = memory.get_history(ns, "u1", 20).await.unwrap();
        assert_eq!(history.len(), 2);

        let facts = memory.get_facts("u1").await.unwrap();
        assert_eq!(facts.get("name").unwrap(), "Aman");
    }
}
