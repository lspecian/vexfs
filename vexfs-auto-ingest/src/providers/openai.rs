use super::{EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    input: String,
    model: String,
    encoding_format: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    object: String,
}

impl OpenAIProvider {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Ok(Self {
            client,
            api_key,
            model,
            base_url,
            timeout: Duration::from_secs(timeout_seconds),
            max_retries,
        })
    }

    async fn make_request_with_retry<T>(&self, request_fn: impl Fn() -> reqwest::RequestBuilder) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(1000 * (1 << (attempt - 1))); // Exponential backoff
                debug!("Retrying OpenAI request in {:?} (attempt {})", delay, attempt);
                tokio::time::sleep(delay).await;
            }

            let response = match request_fn().send().await {
                Ok(response) => response,
                Err(e) => {
                    last_error = Some(anyhow::Error::from(e));
                    continue;
                }
            };

            if response.status().is_success() {
                match response.json::<T>().await {
                    Ok(data) => return Ok(data),
                    Err(e) => {
                        last_error = Some(anyhow::Error::from(e));
                        continue;
                    }
                }
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                
                // Don't retry on client errors (4xx)
                if status.is_client_error() {
                    return Err(anyhow::anyhow!(
                        "OpenAI API client error {}: {}",
                        status,
                        error_text
                    ));
                }

                last_error = Some(anyhow::anyhow!(
                    "OpenAI API error {}: {}",
                    status,
                    error_text
                ));
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn health_check(&self) -> Result<bool> {
        debug!("Performing OpenAI health check");

        let request_fn = || {
            self.client
                .get(&format!("{}/models", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
        };

        match self.make_request_with_retry::<OpenAIModelsResponse>(request_fn).await {
            Ok(_) => {
                info!("OpenAI health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!("OpenAI health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        debug!("Generating OpenAI embedding for text length: {}", request.text.len());

        let model = request.model.unwrap_or_else(|| self.model.clone());
        
        let openai_request = OpenAIEmbeddingRequest {
            input: request.text,
            model: model.clone(),
            encoding_format: "float".to_string(),
        };

        let request_fn = || {
            self.client
                .post(&format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&openai_request)
        };

        let response: OpenAIEmbeddingResponse = self
            .make_request_with_retry(request_fn)
            .await
            .context("Failed to get embedding from OpenAI")?;

        if response.data.is_empty() {
            return Err(anyhow::anyhow!("OpenAI returned empty embedding data"));
        }

        let embedding_data = &response.data[0];
        
        info!(
            "Generated OpenAI embedding: model={}, dimension={}, tokens={}",
            response.model,
            embedding_data.embedding.len(),
            response.usage.total_tokens
        );

        Ok(EmbeddingResponse {
            embedding: embedding_data.embedding.clone(),
            model: response.model,
            usage: Some(Usage {
                prompt_tokens: Some(response.usage.prompt_tokens),
                total_tokens: Some(response.usage.total_tokens),
            }),
            metadata: request.metadata,
        })
    }

    async fn embedding_dimension(&self) -> Result<usize> {
        // Known dimensions for OpenAI models
        match self.model.as_str() {
            "text-embedding-3-small" => Ok(1536),
            "text-embedding-3-large" => Ok(3072),
            "text-embedding-ada-002" => Ok(1536),
            _ => {
                // Try to get dimension by making a test embedding
                warn!("Unknown model dimension for {}, making test request", self.model);
                
                let test_request = EmbeddingRequest {
                    text: "test".to_string(),
                    model: Some(self.model.clone()),
                    metadata: None,
                };

                let response = self.embed(test_request).await?;
                Ok(response.embedding.len())
            }
        }
    }

    async fn supported_models(&self) -> Result<Vec<String>> {
        debug!("Fetching supported OpenAI models");

        let request_fn = || {
            self.client
                .get(&format!("{}/models", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
        };

        let response: OpenAIModelsResponse = self
            .make_request_with_retry(request_fn)
            .await
            .context("Failed to fetch OpenAI models")?;

        // Filter for embedding models
        let embedding_models: Vec<String> = response
            .data
            .into_iter()
            .filter(|model| {
                model.id.contains("embedding") || 
                model.id.contains("ada") ||
                model.id == "text-embedding-3-small" ||
                model.id == "text-embedding-3-large"
            })
            .map(|model| model.id)
            .collect();

        info!("Found {} OpenAI embedding models", embedding_models.len());
        Ok(embedding_models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-small".to_string(),
            None,
            30,
            3,
        );
        
        assert!(provider.is_ok());
        let provider = provider.unwrap();
        assert_eq!(provider.name(), "openai");
    }

    #[tokio::test]
    async fn test_embedding_dimension() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-small".to_string(),
            None,
            30,
            3,
        ).unwrap();
        
        let dimension = provider.embedding_dimension().await.unwrap();
        assert_eq!(dimension, 1536);
    }

    #[tokio::test]
    async fn test_embedding_dimension_large() {
        let provider = OpenAIProvider::new(
            "test-key".to_string(),
            "text-embedding-3-large".to_string(),
            None,
            30,
            3,
        ).unwrap();
        
        let dimension = provider.embedding_dimension().await.unwrap();
        assert_eq!(dimension, 3072);
    }

    // Note: Integration tests with real API calls would require valid API keys
    // and should be run separately from unit tests
}