/*
 * VexFS v2.0 - VexGraph Phase 2 VexServer Integration
 * 
 * Integration layer with VexServer's VectorDB API for seamless vector operations
 * between standalone vectors and graph-embedded vectors.
 */

use crate::vexgraph::{
    error_handling::{VexGraphError, VexGraphResult},
    semantic_search::{VectorEmbedding, EmbeddingType, HybridIndex},
    semantic_query_language::DistanceMetric,
    semantic_search_manager::SemanticSearchManager,
    semantic_plugin_system::{PluginRegistry, EmbeddingRequest, EmbeddingResponse},
    core::GraphNode,
    NodeId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// VexServer vector collection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCollection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dimensions: usize,
    pub distance_metric: DistanceMetric,
    pub embedding_type: EmbeddingType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub vector_count: usize,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Vector document in VexServer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub collection_id: String,
    pub embedding: VectorEmbedding,
    pub content: Option<Vec<u8>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub graph_node_id: Option<NodeId>,
}

/// Vector search query for VexServer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchQuery {
    pub collection_id: String,
    pub query_vector: Vec<f32>,
    pub top_k: usize,
    pub distance_metric: Option<DistanceMetric>,
    pub filter: Option<VectorFilter>,
    pub include_metadata: bool,
    pub include_content: bool,
}

/// Vector filter for search queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilter {
    pub metadata_filters: HashMap<String, serde_json::Value>,
    pub embedding_type: Option<EmbeddingType>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub graph_node_ids: Option<Vec<NodeId>>,
}

/// Vector search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub document: VectorDocument,
    pub score: f32,
    pub distance: f32,
}

/// Batch vector operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVectorRequest {
    pub collection_id: String,
    pub operations: Vec<VectorOperation>,
}

/// Vector operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorOperation {
    Insert {
        id: String,
        embedding: VectorEmbedding,
        content: Option<Vec<u8>>,
        metadata: HashMap<String, serde_json::Value>,
        graph_node_id: Option<NodeId>,
    },
    Update {
        id: String,
        embedding: Option<VectorEmbedding>,
        content: Option<Vec<u8>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        graph_node_id: Option<NodeId>,
    },
    Delete {
        id: String,
    },
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub operation_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub document_id: Option<String>,
}

/// VexServer integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VexServerConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub max_batch_size: usize,
    pub retry_attempts: u32,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub enable_compression: bool,
    pub connection_pool_size: usize,
}

impl Default for VexServerConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8081".to_string(),
            api_key: None,
            timeout_ms: 30000,
            max_batch_size: 100,
            retry_attempts: 3,
            enable_caching: true,
            cache_ttl_seconds: 300,
            enable_compression: true,
            connection_pool_size: 10,
        }
    }
}

/// VexServer integration client
pub struct VexServerIntegration {
    config: VexServerConfig,
    #[cfg(feature = "reqwest")]
    http_client: reqwest::Client,
    #[cfg(not(feature = "reqwest"))]
    http_client: (),
    collections_cache: Arc<RwLock<HashMap<String, VectorCollection>>>,
    plugin_registry: Arc<PluginRegistry>,
    semantic_manager: Arc<SemanticSearchManager>,
}

impl VexServerIntegration {
    /// Create a new VexServer integration client
    pub fn new(
        config: VexServerConfig,
        plugin_registry: Arc<PluginRegistry>,
        semantic_manager: Arc<SemanticSearchManager>,
    ) -> VexGraphResult<Self> {
        #[cfg(feature = "reqwest")]
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .pool_max_idle_per_host(config.connection_pool_size)
            .build()
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;
        
        #[cfg(not(feature = "reqwest"))]
        let http_client = ();

        Ok(Self {
            config,
            http_client,
            collections_cache: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry,
            semantic_manager,
        })
    }

    /// List all vector collections
    pub async fn list_collections(&self) -> VexGraphResult<Vec<VectorCollection>> {
        let url = format!("{}/api/v1/collections", self.config.endpoint);
        let mut request = self.http_client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to list collections: {}", e)))?;

        if !response.status().is_success() {
            return Err(VexGraphError::NetworkError(format!(
                "VexServer returned error: {}",
                response.status()
            )));
        }

        let collections: Vec<VectorCollection> = response
            .json()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to parse response: {}", e)))?;

        // Update cache
        {
            let mut cache = self.collections_cache.write().await;
            for collection in &collections {
                cache.insert(collection.id.clone(), collection.clone());
            }
        }

        Ok(collections)
    }

    /// Get a specific vector collection
    pub async fn get_collection(&self, collection_id: &str) -> VexGraphResult<VectorCollection> {
        // Check cache first
        {
            let cache = self.collections_cache.read().await;
            if let Some(collection) = cache.get(collection_id) {
                return Ok(collection.clone());
            }
        }

        let url = format!("{}/api/v1/collections/{}", self.config.endpoint, collection_id);
        let mut request = self.http_client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to get collection: {}", e)))?;

        if !response.status().is_success() {
            return Err(VexGraphError::NetworkError(format!(
                "VexServer returned error: {}",
                response.status()
            )));
        }

        let collection: VectorCollection = response
            .json()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to parse response: {}", e)))?;

        // Update cache
        {
            let mut cache = self.collections_cache.write().await;
            cache.insert(collection.id.clone(), collection.clone());
        }

        Ok(collection)
    }

    /// Create a new vector collection
    pub async fn create_collection(
        &self,
        name: &str,
        description: &str,
        dimensions: usize,
        distance_metric: DistanceMetric,
        embedding_type: EmbeddingType,
        metadata: HashMap<String, serde_json::Value>,
    ) -> VexGraphResult<VectorCollection> {
        let url = format!("{}/api/v1/collections", self.config.endpoint);
        let mut request = self.http_client.post(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let create_request = serde_json::json!({
            "name": name,
            "description": description,
            "dimensions": dimensions,
            "distance_metric": distance_metric,
            "embedding_type": embedding_type,
            "metadata": metadata
        });

        let response = request
            .json(&create_request)
            .send()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to create collection: {}", e)))?;

        if !response.status().is_success() {
            return Err(VexGraphError::NetworkError(format!(
                "VexServer returned error: {}",
                response.status()
            )));
        }

        let collection: VectorCollection = response
            .json()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to parse response: {}", e)))?;

        // Update cache
        {
            let mut cache = self.collections_cache.write().await;
            cache.insert(collection.id.clone(), collection.clone());
        }

        Ok(collection)
    }

    /// Insert a vector document
    pub async fn insert_vector(
        &self,
        collection_id: &str,
        id: &str,
        embedding: VectorEmbedding,
        content: Option<Vec<u8>>,
        metadata: HashMap<String, serde_json::Value>,
        graph_node_id: Option<NodeId>,
    ) -> VexGraphResult<VectorDocument> {
        let url = format!("{}/api/v1/collections/{}/vectors", self.config.endpoint, collection_id);
        let mut request = self.http_client.post(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let insert_request = serde_json::json!({
            "id": id,
            "embedding": embedding,
            "content": content,
            "metadata": metadata,
            "graph_node_id": graph_node_id
        });

        let response = request
            .json(&insert_request)
            .send()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to insert vector: {}", e)))?;

        if !response.status().is_success() {
            return Err(VexGraphError::NetworkError(format!(
                "VexServer returned error: {}",
                response.status()
            )));
        }

        let document: VectorDocument = response
            .json()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to parse response: {}", e)))?;

        Ok(document)
    }

    /// Search vectors in a collection
    pub async fn search_vectors(
        &self,
        query: VectorSearchQuery,
    ) -> VexGraphResult<Vec<VectorSearchResult>> {
        let url = format!("{}/api/v1/collections/{}/search", self.config.endpoint, query.collection_id);
        let mut request = self.http_client.post(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .json(&query)
            .send()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to search vectors: {}", e)))?;

        if !response.status().is_success() {
            return Err(VexGraphError::NetworkError(format!(
                "VexServer returned error: {}",
                response.status()
            )));
        }

        let results: Vec<VectorSearchResult> = response
            .json()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to parse response: {}", e)))?;

        Ok(results)
    }

    /// Perform batch vector operations
    pub async fn batch_operations(
        &self,
        request: BatchVectorRequest,
    ) -> VexGraphResult<Vec<BatchOperationResult>> {
        let url = format!("{}/api/v1/collections/{}/batch", self.config.endpoint, request.collection_id);
        let mut http_request = self.http_client.post(&url);

        if let Some(api_key) = &self.config.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = http_request
            .json(&request)
            .send()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to perform batch operations: {}", e)))?;

        if !response.status().is_success() {
            return Err(VexGraphError::NetworkError(format!(
                "VexServer returned error: {}",
                response.status()
            )));
        }

        let results: Vec<BatchOperationResult> = response
            .json()
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to parse response: {}", e)))?;

        Ok(results)
    }

    /// Sync graph node with VexServer vector
    pub async fn sync_graph_node_to_vector(
        &self,
        collection_id: &str,
        node: &GraphNode,
    ) -> VexGraphResult<VectorDocument> {
        // Generate embedding for the node if it doesn't have one
        let embedding = if let Some(existing_embedding) = &node.vector_embedding {
            existing_embedding.clone()
        } else {
            // Use plugin system to generate embedding
            let content = self.extract_node_content(node)?;
            let embedding_request = EmbeddingRequest {
                content,
                content_type: EmbeddingType::Text, // Default to text, could be configurable
                target_dimensions: None,
                preprocessing_options: HashMap::new(),
                metadata: HashMap::new(),
            };

            let embedding_response = self.plugin_registry.generate_embedding(embedding_request).await?;
            embedding_response.embedding
        };

        // Prepare metadata from node properties
        let mut metadata = HashMap::new();
        for (key, value) in &node.properties {
            metadata.insert(key.clone(), serde_json::to_value(value).unwrap_or(serde_json::Value::Null));
        }
        metadata.insert("node_type".to_string(), serde_json::json!(node.node_type));
        metadata.insert("created_at".to_string(), serde_json::json!(node.created_at));

        // Insert or update vector in VexServer
        self.insert_vector(
            collection_id,
            &node.id.to_string(),
            embedding,
            None, // Content is stored in the graph node
            metadata,
            Some(node.id),
        ).await
    }

    /// Sync VexServer vector to graph node
    pub async fn sync_vector_to_graph_node(
        &self,
        document: &VectorDocument,
    ) -> VexGraphResult<GraphNode> {
        if let Some(node_id) = document.graph_node_id {
            // Update existing graph node
            let mut node = self.semantic_manager.get_node(node_id).await?;
            node.vector_embedding = Some(document.embedding.clone());
            
            // Update properties from metadata
            for (key, value) in &document.metadata {
                if key != "node_type" && key != "created_at" {
                    // Convert JSON value to PropertyType
                    if let Ok(property_value) = self.json_to_property_type(value) {
                        node.properties.insert(key.clone(), property_value);
                    }
                }
            }
            
            self.semantic_manager.update_node(node.clone()).await?;
            Ok(node)
        } else {
            // Create new graph node
            let node_type = document.metadata.get("node_type")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or(crate::vexgraph::NodeType::Vector);

            let mut properties = HashMap::new();
            for (key, value) in &document.metadata {
                if key != "node_type" && key != "created_at" {
                    if let Ok(property_value) = self.json_to_property_type(value) {
                        properties.insert(key.clone(), property_value);
                    }
                }
            }

            let node = GraphNode {
                id: self.semantic_manager.generate_node_id().await?,
                node_type,
                properties,
                vector_embedding: Some(document.embedding.clone()),
                created_at: document.created_at,
                updated_at: document.updated_at,
            };

            self.semantic_manager.create_node(node.clone()).await?;
            Ok(node)
        }
    }

    /// Perform hybrid search combining VexServer and graph traversal
    pub async fn hybrid_search(
        &self,
        collection_id: &str,
        query_vector: Vec<f32>,
        top_k: usize,
        graph_constraints: Option<crate::vexgraph::semantic_search::GraphConstraints>,
    ) -> VexGraphResult<Vec<VectorSearchResult>> {
        // First, perform vector search in VexServer
        let vector_query = VectorSearchQuery {
            collection_id: collection_id.to_string(),
            query_vector,
            top_k: top_k * 2, // Get more results to allow for graph filtering
            distance_metric: None,
            filter: None,
            include_metadata: true,
            include_content: false,
        };

        let mut vector_results = self.search_vectors(vector_query).await?;

        // Apply graph constraints if provided
        if let Some(constraints) = graph_constraints {
            let mut filtered_results = Vec::new();
            
            for result in vector_results {
                if let Some(node_id) = result.document.graph_node_id {
                    // Check if node satisfies graph constraints
                    if self.semantic_manager.check_graph_constraints(node_id, &constraints).await? {
                        filtered_results.push(result);
                    }
                }
            }
            
            vector_results = filtered_results;
        }

        // Limit to requested top_k
        vector_results.truncate(top_k);
        Ok(vector_results)
    }

    /// Extract content from graph node for embedding generation
    fn extract_node_content(&self, node: &GraphNode) -> VexGraphResult<Vec<u8>> {
        // Extract meaningful content from node properties
        let mut content_parts = Vec::new();
        
        // Add node type
        content_parts.push(format!("Type: {:?}", node.node_type));
        
        // Add properties
        for (key, value) in &node.properties {
            content_parts.push(format!("{}: {}", key, self.property_to_string(value)));
        }
        
        let content = content_parts.join("\n");
        Ok(content.into_bytes())
    }

    /// Convert PropertyType to string representation
    fn property_to_string(&self, property: &crate::vexgraph::PropertyType) -> String {
        match property {
            crate::vexgraph::PropertyType::String(s) => s.clone(),
            crate::vexgraph::PropertyType::Integer(i) => i.to_string(),
            crate::vexgraph::PropertyType::Float(f) => f.to_string(),
            crate::vexgraph::PropertyType::Boolean(b) => b.to_string(),
            crate::vexgraph::PropertyType::Vector(v) => format!("Vector[{}]", v.len()),
            crate::vexgraph::PropertyType::Timestamp(t) => t.to_rfc3339(),
            crate::vexgraph::PropertyType::Json(j) => j.to_string(),
            crate::vexgraph::PropertyType::Binary(b) => format!("Binary[{}]", b.len()),
        }
    }

    /// Convert JSON value to PropertyType
    fn json_to_property_type(&self, value: &serde_json::Value) -> VexGraphResult<crate::vexgraph::PropertyType> {
        match value {
            serde_json::Value::String(s) => Ok(crate::vexgraph::PropertyType::String(s.clone())),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(crate::vexgraph::PropertyType::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(crate::vexgraph::PropertyType::Float(f))
                } else {
                    Err(VexGraphError::ValidationError("Invalid number format".to_string()))
                }
            }
            serde_json::Value::Bool(b) => Ok(crate::vexgraph::PropertyType::Boolean(*b)),
            serde_json::Value::Array(arr) => {
                // Try to convert to vector of floats
                let mut float_vec = Vec::new();
                for item in arr {
                    if let Some(f) = item.as_f64() {
                        float_vec.push(f as f32);
                    } else {
                        // If not all floats, store as JSON
                        return Ok(crate::vexgraph::PropertyType::Json(value.clone()));
                    }
                }
                Ok(crate::vexgraph::PropertyType::Vector(float_vec))
            }
            _ => Ok(crate::vexgraph::PropertyType::Json(value.clone())),
        }
    }

    /// Get integration statistics
    pub async fn get_statistics(&self) -> VexGraphResult<VexServerIntegrationStats> {
        let collections_count = {
            let cache = self.collections_cache.read().await;
            cache.len()
        };

        Ok(VexServerIntegrationStats {
            endpoint: self.config.endpoint.clone(),
            collections_cached: collections_count,
            last_sync: chrono::Utc::now(),
            total_requests: 0, // TODO: Implement request counting
            successful_requests: 0,
            failed_requests: 0,
        })
    }
}

/// VexServer integration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VexServerIntegrationStats {
    pub endpoint: String,
    pub collections_cached: usize,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vexgraph::semantic_plugin_system::PluginRegistry;
    use crate::vexgraph::semantic_search_manager::SemanticSearchManager;

    #[tokio::test]
    async fn test_vexserver_integration_creation() {
        let config = VexServerConfig::default();
        let plugin_registry = Arc::new(PluginRegistry::new());
        let semantic_manager = Arc::new(SemanticSearchManager::new(
            crate::vexgraph::semantic_search_manager::SemanticSearchConfig::default()
        ).await.unwrap());

        let integration = VexServerIntegration::new(config, plugin_registry, semantic_manager);
        assert!(integration.is_ok());
    }

    #[test]
    fn test_vector_collection_serialization() {
        let collection = VectorCollection {
            id: "test_collection".to_string(),
            name: "Test Collection".to_string(),
            description: "A test collection".to_string(),
            dimensions: 128,
            distance_metric: DistanceMetric::Cosine,
            embedding_type: EmbeddingType::Text,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            vector_count: 0,
            metadata: HashMap::new(),
        };

        let serialized = serde_json::to_string(&collection).unwrap();
        let deserialized: VectorCollection = serde_json::from_str(&serialized).unwrap();
        assert_eq!(collection.id, deserialized.id);
    }

    #[test]
    fn test_vector_search_query_serialization() {
        let query = VectorSearchQuery {
            collection_id: "test_collection".to_string(),
            query_vector: vec![0.1, 0.2, 0.3],
            top_k: 10,
            distance_metric: Some(DistanceMetric::Cosine),
            filter: None,
            include_metadata: true,
            include_content: false,
        };

        let serialized = serde_json::to_string(&query).unwrap();
        let deserialized: VectorSearchQuery = serde_json::from_str(&serialized).unwrap();
        assert_eq!(query.collection_id, deserialized.collection_id);
        assert_eq!(query.top_k, deserialized.top_k);
    }
}