use anyhow::Result;
use reqwest::Client;

/// Client for generating text embeddings via OpenAI-compatible /v1/embeddings endpoint.
pub struct EmbeddingClient {
    client: Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl EmbeddingClient {
    pub fn new(base_url: String, model: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        tracing::info!(model = %model, base_url = %base_url, "Embedding client initialized");

        Self { client, base_url, model, api_key }
    }

    /// Generate embeddings for a batch of texts.
    pub async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let payload = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let api_key = self.api_key.as_deref().unwrap_or("no-key");
        let url = format!("{}/embeddings", self.base_url);

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Embedding API error {}: {}", status, body);
        }

        let data: serde_json::Value = resp.json().await?;
        let embeddings = data["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'data' array in embedding response"))?
            .iter()
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
            .collect();

        Ok(embeddings)
    }

    /// Generate a single embedding.
    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed(&[text]).await?;
        results.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("Empty embedding response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_embed_batch() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "data": [
                {"embedding": [0.1, 0.2, 0.3], "index": 0},
                {"embedding": [0.4, 0.5, 0.6], "index": 1},
            ],
            "model": "test-model",
            "usage": {"prompt_tokens": 10, "total_tokens": 10}
        });

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = EmbeddingClient::new(
            format!("{}/v1", mock_server.uri()),
            "test-model".into(),
            Some("test-key".into()),
        );

        let results = client.embed(&["hello", "world"]).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].len(), 3);
        assert!((results[0][0] - 0.1).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_embed_one() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "data": [{"embedding": [1.0, 2.0, 3.0], "index": 0}],
            "model": "test-model",
            "usage": {"prompt_tokens": 5, "total_tokens": 5}
        });

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = EmbeddingClient::new(
            format!("{}/v1", mock_server.uri()),
            "test-model".into(),
            None,
        );

        let embedding = client.embed_one("test text").await.unwrap();
        assert_eq!(embedding.len(), 3);
    }
}
