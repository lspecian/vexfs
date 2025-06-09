/*
 * VexFS v2.0 - VexGraph Phase 2 Semantic Search Plugin System
 * 
 * Plugin system for custom embedding generators with hot-swapping capabilities
 * and dynamic loading of embedding models and processors.
 */

use crate::vexgraph::{
    error_handling::{VexGraphError, VexGraphResult},
    semantic_search::{VectorEmbedding, EmbeddingType},
    semantic_query_language::DistanceMetric,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
#[cfg(feature = "async-trait")]
use async_trait::async_trait;
use tokio::sync::Mutex;

/// Plugin metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub supported_types: Vec<EmbeddingType>,
    pub supported_metrics: Vec<DistanceMetric>,
    pub config_schema: serde_json::Value,
    pub dependencies: Vec<String>,
    pub capabilities: PluginCapabilities,
}

/// Plugin capabilities and features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilities {
    pub batch_processing: bool,
    pub streaming: bool,
    pub gpu_acceleration: bool,
    pub model_fine_tuning: bool,
    pub custom_preprocessing: bool,
    pub dimension_reduction: bool,
    pub multi_modal: bool,
    pub real_time: bool,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub priority: u32,
    pub max_batch_size: usize,
    pub timeout_ms: u64,
    pub memory_limit_mb: usize,
    pub gpu_device_id: Option<u32>,
    pub model_path: Option<String>,
    pub custom_params: HashMap<String, serde_json::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 100,
            max_batch_size: 32,
            timeout_ms: 30000,
            memory_limit_mb: 1024,
            gpu_device_id: None,
            model_path: None,
            custom_params: HashMap::new(),
        }
    }
}

/// Plugin status and health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatus {
    pub loaded: bool,
    pub active: bool,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub memory_usage_mb: usize,
    pub error_rate: f64,
}

impl Default for PluginStatus {
    fn default() -> Self {
        Self {
            loaded: false,
            active: false,
            last_used: None,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_latency_ms: 0.0,
            memory_usage_mb: 0,
            error_rate: 0.0,
        }
    }
}

/// Embedding generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub content: Vec<u8>,
    pub content_type: EmbeddingType,
    pub target_dimensions: Option<usize>,
    pub preprocessing_options: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, String>,
}

/// Embedding generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: VectorEmbedding,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub model_version: String,
    pub metadata: HashMap<String, String>,
}

/// Batch embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbeddingRequest {
    pub requests: Vec<EmbeddingRequest>,
    pub batch_options: HashMap<String, serde_json::Value>,
}

/// Batch embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbeddingResponse {
    pub responses: Vec<VexGraphResult<EmbeddingResponse>>,
    pub batch_processing_time_ms: u64,
    pub successful_count: usize,
    pub failed_count: usize,
}

/// Plugin trait for embedding generators
#[cfg(feature = "async-trait")]
#[async_trait]
pub trait EmbeddingPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Initialize the plugin with configuration
    async fn initialize(&mut self, config: PluginConfig) -> VexGraphResult<()>;
    
    /// Shutdown the plugin and cleanup resources
    async fn shutdown(&mut self) -> VexGraphResult<()>;
    
    /// Check if plugin supports the given embedding type
    fn supports_type(&self, embedding_type: &EmbeddingType) -> bool;
    
    /// Generate a single embedding
    async fn generate_embedding(
        &self,
        request: EmbeddingRequest,
    ) -> VexGraphResult<EmbeddingResponse>;
    
    /// Generate embeddings in batch
    async fn generate_batch_embeddings(
        &self,
        request: BatchEmbeddingRequest,
    ) -> VexGraphResult<BatchEmbeddingResponse> {
        let start_time = std::time::Instant::now();
        let mut responses = Vec::new();
        let mut successful_count = 0;
        let mut failed_count = 0;
        
        for req in request.requests {
            match self.generate_embedding(req).await {
                Ok(response) => {
                    responses.push(Ok(response));
                    successful_count += 1;
                }
                Err(error) => {
                    responses.push(Err(error));
                    failed_count += 1;
                }
            }
        }
        
        Ok(BatchEmbeddingResponse {
            responses,
            batch_processing_time_ms: start_time.elapsed().as_millis() as u64,
            successful_count,
            failed_count,
        })
    }
    
    /// Get current plugin status
    fn status(&self) -> PluginStatus;
    
    /// Update plugin configuration
    async fn update_config(&mut self, config: PluginConfig) -> VexGraphResult<()>;
    
    /// Perform health check
    async fn health_check(&self) -> VexGraphResult<()>;
    
    /// Get plugin metrics
    fn metrics(&self) -> HashMap<String, serde_json::Value>;
}

/// Plugin trait for embedding generators (stub version when async-trait is not available)
#[cfg(not(feature = "async-trait"))]
pub trait EmbeddingPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Check if plugin supports the given embedding type
    fn supports_type(&self, embedding_type: &EmbeddingType) -> bool;
    
    /// Get plugin status
    fn status(&self) -> PluginStatus;
    
    /// Get plugin metrics
    fn metrics(&self) -> HashMap<String, serde_json::Value>;
}

/// Plugin registry for managing embedding plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Box<dyn EmbeddingPlugin>>>>,
    configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
    status_map: Arc<RwLock<HashMap<String, PluginStatus>>>,
    type_mappings: Arc<RwLock<HashMap<EmbeddingType, Vec<String>>>>,
    priority_order: Arc<RwLock<Vec<String>>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            status_map: Arc::new(RwLock::new(HashMap::new())),
            type_mappings: Arc::new(RwLock::new(HashMap::new())),
            priority_order: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Register a new plugin
    pub async fn register_plugin(
        &self,
        plugin: Box<dyn EmbeddingPlugin>,
        config: PluginConfig,
    ) -> VexGraphResult<()> {
        let plugin_name = plugin.metadata().name.clone();
        
        // Initialize the plugin
        let mut plugin = plugin;
        plugin.initialize(config.clone()).await?;
        
        // Update registry
        {
            let mut plugins = self.plugins.write().unwrap();
            let mut configs = self.configs.write().unwrap();
            let mut status_map = self.status_map.write().unwrap();
            let mut type_mappings = self.type_mappings.write().unwrap();
            let mut priority_order = self.priority_order.write().unwrap();
            
            // Store plugin and config
            plugins.insert(plugin_name.clone(), plugin);
            configs.insert(plugin_name.clone(), config);
            status_map.insert(plugin_name.clone(), PluginStatus::default());
            
            // Update type mappings
            let metadata = plugins.get(&plugin_name).unwrap().metadata();
            for embedding_type in &metadata.supported_types {
                type_mappings
                    .entry(embedding_type.clone())
                    .or_insert_with(Vec::new)
                    .push(plugin_name.clone());
            }
            
            // Update priority order
            priority_order.push(plugin_name.clone());
            priority_order.sort_by(|a, b| {
                let config_a = configs.get(a).unwrap();
                let config_b = configs.get(b).unwrap();
                config_a.priority.cmp(&config_b.priority)
            });
        }
        
        tracing::info!("Registered plugin: {}", plugin_name);
        Ok(())
    }
    
    /// Unregister a plugin
    pub async fn unregister_plugin(&self, plugin_name: &str) -> VexGraphResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        let mut configs = self.configs.write().unwrap();
        let mut status_map = self.status_map.write().unwrap();
        let mut type_mappings = self.type_mappings.write().unwrap();
        let mut priority_order = self.priority_order.write().unwrap();
        
        // Shutdown plugin
        if let Some(mut plugin) = plugins.remove(plugin_name) {
            plugin.shutdown().await?;
        }
        
        // Remove from all mappings
        configs.remove(plugin_name);
        status_map.remove(plugin_name);
        priority_order.retain(|name| name != plugin_name);
        
        // Remove from type mappings
        for (_, plugin_list) in type_mappings.iter_mut() {
            plugin_list.retain(|name| name != plugin_name);
        }
        
        tracing::info!("Unregistered plugin: {}", plugin_name);
        Ok(())
    }
    
    /// Get the best plugin for a given embedding type
    pub fn get_best_plugin(&self, embedding_type: &EmbeddingType) -> Option<String> {
        let type_mappings = self.type_mappings.read().unwrap();
        let priority_order = self.priority_order.read().unwrap();
        let configs = self.configs.read().unwrap();
        
        if let Some(available_plugins) = type_mappings.get(embedding_type) {
            for plugin_name in &*priority_order {
                if available_plugins.contains(plugin_name) {
                    if let Some(config) = configs.get(plugin_name) {
                        if config.enabled {
                            return Some(plugin_name.clone());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Generate embedding using the best available plugin
    pub async fn generate_embedding(
        &self,
        request: EmbeddingRequest,
    ) -> VexGraphResult<EmbeddingResponse> {
        let plugin_name = self.get_best_plugin(&request.content_type)
            .ok_or_else(|| VexGraphError::SemanticSearchError(
                format!("No plugin available for embedding type: {:?}", request.content_type)
            ))?;
        
        self.generate_embedding_with_plugin(&plugin_name, request).await
    }
    
    /// Generate embedding using a specific plugin
    pub async fn generate_embedding_with_plugin(
        &self,
        plugin_name: &str,
        request: EmbeddingRequest,
    ) -> VexGraphResult<EmbeddingResponse> {
        let plugins = self.plugins.read().unwrap();
        let plugin = plugins.get(plugin_name)
            .ok_or_else(|| VexGraphError::SemanticSearchError(
                format!("Plugin not found: {}", plugin_name)
            ))?;
        
        let start_time = std::time::Instant::now();
        let result = plugin.generate_embedding(request).await;
        let duration = start_time.elapsed();
        
        // Update plugin statistics
        self.update_plugin_stats(plugin_name, &result, duration).await;
        
        result
    }
    
    /// Generate batch embeddings
    pub async fn generate_batch_embeddings(
        &self,
        request: BatchEmbeddingRequest,
    ) -> VexGraphResult<BatchEmbeddingResponse> {
        // Group requests by embedding type
        let mut type_groups: HashMap<EmbeddingType, Vec<EmbeddingRequest>> = HashMap::new();
        for req in request.requests {
            type_groups.entry(req.content_type.clone()).or_default().push(req);
        }
        
        let mut all_responses = Vec::new();
        let mut total_successful = 0;
        let mut total_failed = 0;
        let start_time = std::time::Instant::now();
        
        // Process each type group with its best plugin
        for (embedding_type, requests) in type_groups {
            if let Some(plugin_name) = self.get_best_plugin(&embedding_type) {
                let batch_request = BatchEmbeddingRequest {
                    requests,
                    batch_options: request.batch_options.clone(),
                };
                
                match self.generate_batch_with_plugin(&plugin_name, batch_request).await {
                    Ok(batch_response) => {
                        all_responses.extend(batch_response.responses);
                        total_successful += batch_response.successful_count;
                        total_failed += batch_response.failed_count;
                    }
                    Err(error) => {
                        // If batch processing fails, mark all requests as failed
                        let failed_count = requests.len();
                        for _ in 0..failed_count {
                            all_responses.push(Err(error.clone()));
                        }
                        total_failed += failed_count;
                    }
                }
            } else {
                // No plugin available for this type
                let error = VexGraphError::SemanticSearchError(
                    format!("No plugin available for embedding type: {:?}", embedding_type)
                );
                let failed_count = requests.len();
                for _ in 0..failed_count {
                    all_responses.push(Err(error.clone()));
                }
                total_failed += failed_count;
            }
        }
        
        Ok(BatchEmbeddingResponse {
            responses: all_responses,
            batch_processing_time_ms: start_time.elapsed().as_millis() as u64,
            successful_count: total_successful,
            failed_count: total_failed,
        })
    }
    
    /// Generate batch embeddings with a specific plugin
    pub async fn generate_batch_with_plugin(
        &self,
        plugin_name: &str,
        request: BatchEmbeddingRequest,
    ) -> VexGraphResult<BatchEmbeddingResponse> {
        let plugins = self.plugins.read().unwrap();
        let plugin = plugins.get(plugin_name)
            .ok_or_else(|| VexGraphError::SemanticSearchError(
                format!("Plugin not found: {}", plugin_name)
            ))?;
        
        let start_time = std::time::Instant::now();
        let result = plugin.generate_batch_embeddings(request).await;
        let duration = start_time.elapsed();
        
        // Update plugin statistics for batch operation
        self.update_plugin_batch_stats(plugin_name, &result, duration).await;
        
        result
    }
    
    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().unwrap();
        plugins.keys().cloned().collect()
    }
    
    /// Get plugin metadata
    pub fn get_plugin_metadata(&self, plugin_name: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(plugin_name).map(|plugin| plugin.metadata().clone())
    }
    
    /// Get plugin status
    pub fn get_plugin_status(&self, plugin_name: &str) -> Option<PluginStatus> {
        let status_map = self.status_map.read().unwrap();
        status_map.get(plugin_name).cloned()
    }
    
    /// Update plugin configuration
    pub async fn update_plugin_config(
        &self,
        plugin_name: &str,
        config: PluginConfig,
    ) -> VexGraphResult<()> {
        let plugins = self.plugins.read().unwrap();
        let mut configs = self.configs.write().unwrap();
        
        if let Some(plugin) = plugins.get(plugin_name) {
            plugin.update_config(config.clone()).await?;
            configs.insert(plugin_name.to_string(), config);
            Ok(())
        } else {
            Err(VexGraphError::SemanticSearchError(
                format!("Plugin not found: {}", plugin_name)
            ))
        }
    }
    
    /// Perform health check on all plugins
    pub async fn health_check_all(&self) -> HashMap<String, VexGraphResult<()>> {
        let plugins = self.plugins.read().unwrap();
        let mut results = HashMap::new();
        
        for (name, plugin) in plugins.iter() {
            let result = plugin.health_check().await;
            results.insert(name.clone(), result);
        }
        
        results
    }
    
    /// Get metrics from all plugins
    pub fn get_all_metrics(&self) -> HashMap<String, HashMap<String, serde_json::Value>> {
        let plugins = self.plugins.read().unwrap();
        let mut all_metrics = HashMap::new();
        
        for (name, plugin) in plugins.iter() {
            all_metrics.insert(name.clone(), plugin.metrics());
        }
        
        all_metrics
    }
    
    /// Update plugin statistics after a request
    async fn update_plugin_stats(
        &self,
        plugin_name: &str,
        result: &VexGraphResult<EmbeddingResponse>,
        duration: std::time::Duration,
    ) {
        let mut status_map = self.status_map.write().unwrap();
        if let Some(status) = status_map.get_mut(plugin_name) {
            status.last_used = Some(chrono::Utc::now());
            status.total_requests += 1;
            
            match result {
                Ok(_) => {
                    status.successful_requests += 1;
                }
                Err(_) => {
                    status.failed_requests += 1;
                }
            }
            
            // Update average latency (simple moving average)
            let new_latency = duration.as_millis() as f64;
            if status.total_requests == 1 {
                status.average_latency_ms = new_latency;
            } else {
                status.average_latency_ms = 
                    (status.average_latency_ms * (status.total_requests - 1) as f64 + new_latency) 
                    / status.total_requests as f64;
            }
            
            // Update error rate
            status.error_rate = status.failed_requests as f64 / status.total_requests as f64;
        }
    }
    
    /// Update plugin statistics after a batch request
    async fn update_plugin_batch_stats(
        &self,
        plugin_name: &str,
        result: &VexGraphResult<BatchEmbeddingResponse>,
        duration: std::time::Duration,
    ) {
        let mut status_map = self.status_map.write().unwrap();
        if let Some(status) = status_map.get_mut(plugin_name) {
            status.last_used = Some(chrono::Utc::now());
            
            match result {
                Ok(batch_response) => {
                    status.total_requests += batch_response.responses.len() as u64;
                    status.successful_requests += batch_response.successful_count as u64;
                    status.failed_requests += batch_response.failed_count as u64;
                }
                Err(_) => {
                    // If the entire batch failed, we don't know how many individual requests there were
                    status.total_requests += 1;
                    status.failed_requests += 1;
                }
            }
            
            // Update average latency
            let new_latency = duration.as_millis() as f64;
            if status.total_requests == 1 {
                status.average_latency_ms = new_latency;
            } else {
                status.average_latency_ms = 
                    (status.average_latency_ms * (status.total_requests - 1) as f64 + new_latency) 
                    / status.total_requests as f64;
            }
            
            // Update error rate
            status.error_rate = status.failed_requests as f64 / status.total_requests as f64;
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin manager for coordinating plugin lifecycle
pub struct PluginManager {
    registry: PluginRegistry,
    hot_swap_enabled: bool,
    auto_reload: bool,
    plugin_directory: Option<String>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            hot_swap_enabled: true,
            auto_reload: false,
            plugin_directory: None,
        }
    }
    
    /// Enable hot-swapping of plugins
    pub fn enable_hot_swap(&mut self, enabled: bool) {
        self.hot_swap_enabled = enabled;
    }
    
    /// Enable automatic plugin reloading
    pub fn enable_auto_reload(&mut self, enabled: bool, plugin_directory: Option<String>) {
        self.auto_reload = enabled;
        self.plugin_directory = plugin_directory;
    }
    
    /// Get reference to the plugin registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
    
    /// Hot-swap a plugin with a new version
    pub async fn hot_swap_plugin(
        &self,
        plugin_name: &str,
        new_plugin: Box<dyn EmbeddingPlugin>,
        new_config: PluginConfig,
    ) -> VexGraphResult<()> {
        if !self.hot_swap_enabled {
            return Err(VexGraphError::SemanticSearchError(
                "Hot-swapping is disabled".to_string()
            ));
        }
        
        // Unregister old plugin
        self.registry.unregister_plugin(plugin_name).await?;
        
        // Register new plugin
        self.registry.register_plugin(new_plugin, new_config).await?;
        
        tracing::info!("Hot-swapped plugin: {}", plugin_name);
        Ok(())
    }
    
    /// Start the plugin manager (initialize auto-reload if enabled)
    pub async fn start(&self) -> VexGraphResult<()> {
        if self.auto_reload && self.plugin_directory.is_some() {
            // TODO: Implement file system watching for plugin directory
            tracing::info!("Plugin auto-reload enabled");
        }
        
        tracing::info!("Plugin manager started");
        Ok(())
    }
    
    /// Stop the plugin manager
    pub async fn stop(&self) -> VexGraphResult<()> {
        // Unregister all plugins
        let plugin_names = self.registry.list_plugins();
        for plugin_name in plugin_names {
            self.registry.unregister_plugin(&plugin_name).await?;
        }
        
        tracing::info!("Plugin manager stopped");
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock plugin for testing
    struct MockEmbeddingPlugin {
        metadata: PluginMetadata,
        status: PluginStatus,
    }
    
    impl MockEmbeddingPlugin {
        fn new() -> Self {
            Self {
                metadata: PluginMetadata {
                    name: "mock_plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Mock plugin for testing".to_string(),
                    author: "Test Author".to_string(),
                    supported_types: vec![EmbeddingType::Text],
                    supported_metrics: vec![DistanceMetric::Cosine],
                    config_schema: serde_json::json!({}),
                    dependencies: vec![],
                    capabilities: PluginCapabilities {
                        batch_processing: true,
                        streaming: false,
                        gpu_acceleration: false,
                        model_fine_tuning: false,
                        custom_preprocessing: false,
                        dimension_reduction: false,
                        multi_modal: false,
                        real_time: true,
                    },
                },
                status: PluginStatus::default(),
            }
        }
    }
    
    #[cfg(feature = "async-trait")]
    #[async_trait]
    impl EmbeddingPlugin for MockEmbeddingPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }
        
        async fn initialize(&mut self, _config: PluginConfig) -> VexGraphResult<()> {
            self.status.loaded = true;
            self.status.active = true;
            Ok(())
        }
        
        async fn shutdown(&mut self) -> VexGraphResult<()> {
            self.status.loaded = false;
            self.status.active = false;
            Ok(())
        }
        
        fn supports_type(&self, embedding_type: &EmbeddingType) -> bool {
            self.metadata.supported_types.contains(embedding_type)
        }
        
        async fn generate_embedding(
            &self,
            request: EmbeddingRequest,
        ) -> VexGraphResult<EmbeddingResponse> {
            // Mock embedding generation
            let embedding = VectorEmbedding {
                embedding_type: request.content_type,
                dimensions: 128,
                values: vec![0.1; 128],
                metadata: HashMap::new(),
            };
            
            Ok(EmbeddingResponse {
                embedding,
                confidence: 0.95,
                processing_time_ms: 10,
                model_version: "mock-1.0".to_string(),
                metadata: HashMap::new(),
            })
        }
        
        fn status(&self) -> PluginStatus {
            self.status.clone()
        }
        
        async fn update_config(&mut self, _config: PluginConfig) -> VexGraphResult<()> {
            Ok(())
        }
        
        async fn health_check(&self) -> VexGraphResult<()> {
            if self.status.active {
                Ok(())
            } else {
                Err(VexGraphError::SemanticSearchError("Plugin not active".to_string()))
            }
        }
        
        fn metrics(&self) -> HashMap<String, serde_json::Value> {
            let mut metrics = HashMap::new();
            metrics.insert("requests".to_string(), serde_json::json!(self.status.total_requests));
            metrics.insert("success_rate".to_string(), serde_json::json!(1.0 - self.status.error_rate));
            metrics
        }
    }
    
    #[cfg(not(feature = "async-trait"))]
    impl EmbeddingPlugin for MockEmbeddingPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }
        
        fn supports_type(&self, embedding_type: &EmbeddingType) -> bool {
            self.metadata.supported_types.contains(embedding_type)
        }
        
        fn status(&self) -> PluginStatus {
            self.status.clone()
        }
        
        fn metrics(&self) -> HashMap<String, serde_json::Value> {
            let mut metrics = HashMap::new();
            metrics.insert("requests".to_string(), serde_json::json!(self.status.total_requests));
            metrics.insert("success_rate".to_string(), serde_json::json!(1.0 - self.status.error_rate));
            metrics
        }
    }
    
    #[tokio::test]
    async fn test_plugin_registration() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(MockEmbeddingPlugin::new());
        let config = PluginConfig::default();
        
        let result = registry.register_plugin(plugin, config).await;
        assert!(result.is_ok());
        
        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert!(plugins.contains(&"mock_plugin".to_string()));
    }
    
    #[tokio::test]
    async fn test_embedding_generation() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(MockEmbeddingPlugin::new());
        let config = PluginConfig::default();
        
        registry.register_plugin(plugin, config).await.unwrap();
        
        let request = EmbeddingRequest {
            content: b"test content".to_vec(),
            content_type: EmbeddingType::Text,
            target_dimensions: Some(128),
            preprocessing_options: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        let result = registry.generate_embedding(request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.embedding.dimensions, 128);
        assert_eq!(response.confidence, 0.95);
    }
    
    #[tokio::test]
    async fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        manager.enable_hot_swap(true);
        
        let result = manager.start().await;
        assert!(result.is_ok());
        
        let result = manager.stop().await;
        assert!(result.is_ok());
    }
}