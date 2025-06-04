use super::{EmbeddingProvider, EmbeddingRequest, EmbeddingResponse, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
    timeout: Duration,
    max_retries: u32,
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    modified_at: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct OllamaShowResponse {
    modelfile: Option<String>,
    parameters: Option<String>,
    template: Option<String>,
    details: Option<OllamaModelDetails>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelDetails {
    format: Option<String>,
    family: Option<String>,
    families: Option<Vec<String>>,
    parameter_size: Option<String>,
    quantization_level: Option<String>,
}

impl OllamaProvider {
    pub fn new(
        base_url: String,
        model: String,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url,
            model,
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
                debug!("Retrying Ollama request in {:?} (attempt {})", delay, attempt);
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
                        "Ollama API client error {}: {}",
                        status,
                        error_text
                    ));
                }

                last_error = Some(anyhow::anyhow!(
                    "Ollama API error {}: {}",
                    status,
                    error_text
                ));
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }

    async fn ensure_model_available(&self, model: &str) -> Result<()> {
        debug!("Checking if Ollama model {} is available", model);

        // First check if model is already available
        let models = self.get_available_models().await?;
        if models.iter().any(|m| m.starts_with(model)) {
            debug!("Model {} is already available", model);
            return Ok(());
        }

        // Try to pull the model
        info!("Model {} not found, attempting to pull it", model);
        
        let pull_request = serde_json::json!({
            "name": model,
            "stream": false
        });

        let request_fn = || {
            self.client
                .post(&format!("{}/api/pull", self.base_url))
                .header("Content-Type", "application/json")
                .json(&pull_request)
        };

        match self.make_request_with_retry::<serde_json::Value>(request_fn).await {
            Ok(_) => {
                info!("Successfully pulled model {}", model);
                Ok(())
            }
            Err(e) => {
                error!("Failed to pull model {}: {}", model, e);
                Err(e)
            }
        }
    }

    async fn get_available_models(&self) -> Result<Vec<String>> {
        let request_fn = || {
            self.client
                .get(&format!("{}/api/tags", self.base_url))
        };

        let response: OllamaTagsResponse = self
            .make_request_with_retry(request_fn)
            .await
            .context("Failed to fetch Ollama models")?;

        Ok(response.models.into_iter().map(|m| m.name).collect())
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn health_check(&self) -> Result<bool> {
        debug!("Performing Ollama health check");

        let request_fn = || {
            self.client
                .get(&format!("{}/api/tags", self.base_url))
        };

        match self.make_request_with_retry::<OllamaTagsResponse>(request_fn).await {
            Ok(_) => {
                info!("Ollama health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!("Ollama health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        debug!("Generating Ollama embedding for text length: {}", request.text.len());

        let model = request.model.unwrap_or_else(|| self.model.clone());
        
        // Ensure model is available
        self.ensure_model_available(&model).await?;

        let ollama_request = OllamaEmbeddingRequest {
            model: model.clone(),
            prompt: request.text,
        };

        let request_fn = || {
            self.client
                .post(&format!("{}/api/embeddings", self.base_url))
                .header("Content-Type", "application/json")
                .json(&ollama_request)
        };

        let response: OllamaEmbeddingResponse = self
            .make_request_with_retry(request_fn)
            .await
            .context("Failed to get embedding from Ollama")?;

        info!(
            "Generated Ollama embedding: model={}, dimension={}",
            model,
            response.embedding.len()
        );

        Ok(EmbeddingResponse {
            embedding: response.embedding,
            model,
            usage: None, // Ollama doesn't provide token usage information
            metadata: request.metadata,
        })
    }

    async fn embedding_dimension(&self) -> Result<usize> {
        // Known dimensions for common Ollama embedding models
        match self.model.as_str() {
            "nomic-embed-text" => Ok(768),
            "mxbai-embed-large" => Ok(1024),
            "all-minilm" => Ok(384),
            "sentence-transformers/all-MiniLM-L6-v2" => Ok(384),
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
        debug!("Fetching supported Ollama models");

        let models = self.get_available_models().await?;
        
        // Filter for embedding models (models that typically support embeddings)
        let embedding_models: Vec<String> = models
            .into_iter()
            .filter(|model| {
                model.contains("embed") ||
                model.contains("nomic") ||
                model.contains("minilm") ||
                model.contains("sentence") ||
                model.contains("bge") ||
                model.contains("e5")
            })
            .collect();

        info!("Found {} Ollama embedding models", embedding_models.len());
        Ok(embedding_models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "nomic-embed-text".to_string(),
            60,
            3,
        );
        
        assert!(provider.is_ok());
        let provider = provider.unwrap();
        assert_eq!(provider.name(), "ollama");
    }

    #[tokio::test]
    async fn test_embedding_dimension() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "nomic-embed-text".to_string(),
            60,
            3,
        ).unwrap();
        
        let dimension = provider.embedding_dimension().await.unwrap();
        assert_eq!(dimension, 768);
    }

    #[tokio::test]
    async fn test_embedding_dimension_minilm() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "all-minilm".to_string(),
            60,
            3,
        ).unwrap();
        
        let dimension = provider.embedding_dimension().await.unwrap();
        assert_eq!(dimension, 384);
    }

    // Note: Integration tests with real Ollama instance would require
    // a running Ollama server and should be run separately from unit tests
}