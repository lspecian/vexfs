pub mod openai;
pub mod ollama;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub usage: Option<Usage>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetrics {
    pub provider: String,
    pub model: String,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub tokens_used: Option<u32>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is available/healthy
    async fn health_check(&self) -> Result<bool>;

    /// Generate embeddings for the given text
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// Get the embedding dimension for the configured model
    async fn embedding_dimension(&self) -> Result<usize>;

    /// Get supported models
    async fn supported_models(&self) -> Result<Vec<String>>;
}

pub struct ProviderManager {
    providers: std::collections::HashMap<String, Box<dyn EmbeddingProvider>>,
    default_provider: String,
    metrics: Vec<EmbeddingMetrics>,
}

impl ProviderManager {
    pub fn new(default_provider: String) -> Self {
        Self {
            providers: std::collections::HashMap::new(),
            default_provider,
            metrics: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, name: String, provider: Box<dyn EmbeddingProvider>) {
        self.providers.insert(name, provider);
    }

    pub async fn embed(&mut self, text: String) -> Result<EmbeddingResponse> {
        self.embed_with_provider(&self.default_provider.clone(), text).await
    }

    pub async fn embed_with_provider(&mut self, provider_name: &str, text: String) -> Result<EmbeddingResponse> {
        let start_time = std::time::Instant::now();
        
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;

        let request = EmbeddingRequest {
            text,
            model: None,
            metadata: None,
        };

        let result = provider.embed(request).await;
        let latency = start_time.elapsed();

        // Record metrics
        let metrics = EmbeddingMetrics {
            provider: provider_name.to_string(),
            model: match &result {
                Ok(response) => response.model.clone(),
                Err(_) => "unknown".to_string(),
            },
            latency_ms: latency.as_millis() as u64,
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
            tokens_used: result.as_ref().ok().and_then(|r| r.usage.as_ref().map(|u| u.total_tokens.unwrap_or(0))),
            timestamp: chrono::Utc::now(),
        };

        self.metrics.push(metrics);

        // Keep only last 1000 metrics
        if self.metrics.len() > 1000 {
            self.metrics.drain(0..self.metrics.len() - 1000);
        }

        result
    }

    pub async fn health_check(&self, provider_name: &str) -> Result<bool> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;
        
        provider.health_check().await
    }

    pub async fn health_check_all(&self) -> std::collections::HashMap<String, bool> {
        let mut results = std::collections::HashMap::new();
        
        for (name, provider) in &self.providers {
            let health = provider.health_check().await.unwrap_or(false);
            results.insert(name.clone(), health);
        }
        
        results
    }

    pub fn get_metrics(&self) -> &[EmbeddingMetrics] {
        &self.metrics
    }

    pub fn get_provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub async fn get_embedding_dimension(&self, provider_name: &str) -> Result<usize> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;
        
        provider.embedding_dimension().await
    }

    pub async fn get_supported_models(&self, provider_name: &str) -> Result<Vec<String>> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;
        
        provider.supported_models().await
    }

    pub fn set_default_provider(&mut self, provider_name: String) -> Result<()> {
        if !self.providers.contains_key(&provider_name) {
            anyhow::bail!("Provider not found: {}", provider_name);
        }
        self.default_provider = provider_name;
        Ok(())
    }

    pub fn get_default_provider(&self) -> &str {
        &self.default_provider
    }

    /// Get metrics summary for the last N requests
    pub fn get_metrics_summary(&self, last_n: usize) -> MetricsSummary {
        let recent_metrics: Vec<_> = self.metrics
            .iter()
            .rev()
            .take(last_n)
            .collect();

        if recent_metrics.is_empty() {
            return MetricsSummary::default();
        }

        let total_requests = recent_metrics.len();
        let successful_requests = recent_metrics.iter().filter(|m| m.success).count();
        let failed_requests = total_requests - successful_requests;

        let avg_latency = if total_requests > 0 {
            recent_metrics.iter().map(|m| m.latency_ms).sum::<u64>() / total_requests as u64
        } else {
            0
        };

        let total_tokens = recent_metrics
            .iter()
            .filter_map(|m| m.tokens_used)
            .sum::<u32>();

        MetricsSummary {
            total_requests,
            successful_requests,
            failed_requests,
            success_rate: if total_requests > 0 {
                successful_requests as f64 / total_requests as f64
            } else {
                0.0
            },
            avg_latency_ms: avg_latency,
            total_tokens,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub success_rate: f64,
    pub avg_latency_ms: u64,
    pub total_tokens: u32,
}

impl Default for MetricsSummary {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            success_rate: 0.0,
            avg_latency_ms: 0,
            total_tokens: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        name: String,
        should_fail: bool,
    }

    #[async_trait]
    impl EmbeddingProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(!self.should_fail)
        }

        async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
            if self.should_fail {
                anyhow::bail!("Mock provider failure");
            }

            Ok(EmbeddingResponse {
                embedding: vec![0.1, 0.2, 0.3],
                model: "mock-model".to_string(),
                usage: Some(Usage {
                    prompt_tokens: Some(10),
                    total_tokens: Some(10),
                }),
                metadata: None,
            })
        }

        async fn embedding_dimension(&self) -> Result<usize> {
            Ok(3)
        }

        async fn supported_models(&self) -> Result<Vec<String>> {
            Ok(vec!["mock-model".to_string()])
        }
    }

    #[tokio::test]
    async fn test_provider_manager() {
        let mut manager = ProviderManager::new("mock".to_string());
        
        let provider = MockProvider {
            name: "mock".to_string(),
            should_fail: false,
        };
        
        manager.add_provider("mock".to_string(), Box::new(provider));
        
        let result = manager.embed("test text".to_string()).await;
        assert!(result.is_ok());
        
        let metrics = manager.get_metrics_summary(10);
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
    }

    #[tokio::test]
    async fn test_provider_failure() {
        let mut manager = ProviderManager::new("mock".to_string());
        
        let provider = MockProvider {
            name: "mock".to_string(),
            should_fail: true,
        };
        
        manager.add_provider("mock".to_string(), Box::new(provider));
        
        let result = manager.embed("test text".to_string()).await;
        assert!(result.is_err());
        
        let metrics = manager.get_metrics_summary(10);
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
    }
}