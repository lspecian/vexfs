/*
 * VexFS v2.0 - VexGraph Semantic Search Manager (Task 11)
 * 
 * This module implements the semantic search manager that integrates
 * vector similarity search with graph traversal operations.
 */

use crate::vexgraph::{
    NodeId, EdgeId, PropertyType, VexGraphConfig,
    core::{VexGraphCore, GraphNode, GraphEdge},
    error_handling::{VexGraphError, VexGraphResult},
    semantic_search::{
        VectorEmbedding, EmbeddingType, SemanticQuery, SemanticSearchResult,
        GraphConstraints, HybridIndex, HybridIndexStatistics,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use uuid::Uuid;

/// Semantic search manager that combines vector search with graph operations
#[derive(Debug)]
pub struct SemanticSearchManager {
    /// Reference to the graph core
    core: Arc<VexGraphCore>,
    
    /// Hybrid indices by embedding type and dimensions
    indices: DashMap<(EmbeddingType, usize), Arc<RwLock<HybridIndex>>>,
    
    /// Cache for frequently accessed embeddings
    embedding_cache: DashMap<Uuid, VectorEmbedding>,
    
    /// Cache for search results
    result_cache: DashMap<String, (Vec<SemanticSearchResult>, chrono::DateTime<chrono::Utc>)>,
    
    /// Configuration
    config: SemanticSearchConfig,
    
    /// Statistics
    stats: RwLock<SemanticSearchStatistics>,
}

/// Configuration for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchConfig {
    /// Maximum cache size for embeddings
    pub max_embedding_cache_size: usize,
    
    /// Maximum cache size for search results
    pub max_result_cache_size: usize,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Default similarity threshold
    pub default_similarity_threshold: f32,
    
    /// Maximum graph traversal depth
    pub max_graph_depth: u32,
    
    /// Enable parallel processing
    pub enable_parallel_processing: bool,
    
    /// Batch size for bulk operations
    pub batch_size: usize,
}

impl Default for SemanticSearchConfig {
    fn default() -> Self {
        Self {
            max_embedding_cache_size: 10000,
            max_result_cache_size: 1000,
            cache_ttl_seconds: 3600, // 1 hour
            default_similarity_threshold: 0.7,
            max_graph_depth: 10,
            enable_parallel_processing: true,
            batch_size: 100,
        }
    }
}

/// Statistics for semantic search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchStatistics {
    pub total_embeddings: usize,
    pub total_searches: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_search_time_ms: f64,
    pub total_indices: usize,
    pub memory_usage_bytes: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl SemanticSearchManager {
    /// Create a new semantic search manager
    pub async fn new(
        core: Arc<VexGraphCore>,
        config: SemanticSearchConfig,
    ) -> VexGraphResult<Self> {
        let stats = SemanticSearchStatistics {
            total_embeddings: 0,
            total_searches: 0,
            cache_hits: 0,
            cache_misses: 0,
            average_search_time_ms: 0.0,
            total_indices: 0,
            memory_usage_bytes: 0,
            last_updated: chrono::Utc::now(),
        };

        Ok(Self {
            core,
            indices: DashMap::new(),
            embedding_cache: DashMap::new(),
            result_cache: DashMap::new(),
            config,
            stats: RwLock::new(stats),
        })
    }

    /// Add a vector embedding to a graph node
    pub async fn add_embedding(
        &self,
        node_id: NodeId,
        embedding_type: EmbeddingType,
        vector: Vec<f32>,
        metadata: Option<HashMap<String, String>>,
    ) -> VexGraphResult<Uuid> {
        // Verify the node exists
        let _node = self.core.get_node(node_id).await?;

        // Create the embedding
        let mut embedding = VectorEmbedding::new(node_id, embedding_type, vector);
        if let Some(meta) = metadata {
            for (key, value) in meta {
                embedding.add_metadata(key, value);
            }
        }

        let embedding_id = embedding.id;
        let dimensions = embedding.dimensions;

        // Get or create the appropriate index
        let index_key = (embedding_type, dimensions);
        let index = if let Some(existing_index) = self.indices.get(&index_key) {
            existing_index.clone()
        } else {
            let new_index = Arc::new(RwLock::new(HybridIndex::new(dimensions)?));
            self.indices.insert(index_key, new_index.clone());
            new_index
        };

        // Add to index
        {
            let index_guard = index.write();
            index_guard.add_embedding(embedding.clone())?;
        }

        // Add to cache
        self.embedding_cache.insert(embedding_id, embedding);

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_embeddings += 1;
            stats.total_indices = self.indices.len();
            stats.last_updated = chrono::Utc::now();
        }

        // Clear result cache as it may be invalidated
        self.clear_result_cache();

        tracing::debug!("Added embedding {} for node {}", embedding_id, node_id);
        Ok(embedding_id)
    }

    /// Remove a vector embedding
    pub async fn remove_embedding(&self, embedding_id: Uuid) -> VexGraphResult<()> {
        // Find the embedding in cache first
        let embedding = self.embedding_cache.get(&embedding_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| VexGraphError::NotFound(format!("Embedding {} not found", embedding_id)))?;

        let index_key = (embedding.embedding_type, embedding.dimensions);

        // Remove from index
        if let Some(index) = self.indices.get(&index_key) {
            let index_guard = index.write();
            index_guard.remove_embedding(embedding_id)?;
        }

        // Remove from cache
        self.embedding_cache.remove(&embedding_id);

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_embeddings = stats.total_embeddings.saturating_sub(1);
            stats.last_updated = chrono::Utc::now();
        }

        // Clear result cache
        self.clear_result_cache();

        tracing::debug!("Removed embedding {}", embedding_id);
        Ok(())
    }

    /// Update a vector embedding
    pub async fn update_embedding(
        &self,
        embedding_id: Uuid,
        vector: Option<Vec<f32>>,
        metadata: Option<HashMap<String, String>>,
    ) -> VexGraphResult<()> {
        // Get the current embedding
        let mut embedding = self.embedding_cache.get(&embedding_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| VexGraphError::NotFound(format!("Embedding {} not found", embedding_id)))?;

        // Update vector if provided
        if let Some(new_vector) = vector {
            embedding.update_vector(new_vector)?;
        }

        // Update metadata if provided
        if let Some(meta) = metadata {
            for (key, value) in meta {
                embedding.add_metadata(key, value);
            }
        }

        let index_key = (embedding.embedding_type, embedding.dimensions);

        // Update in index (remove and re-add)
        if let Some(index) = self.indices.get(&index_key) {
            let index_guard = index.write();
            index_guard.remove_embedding(embedding_id)?;
            index_guard.add_embedding(embedding.clone())?;
        }

        // Update cache
        self.embedding_cache.insert(embedding_id, embedding);

        // Clear result cache
        self.clear_result_cache();

        tracing::debug!("Updated embedding {}", embedding_id);
        Ok(())
    }

    /// Perform semantic search with optional graph constraints
    pub async fn semantic_search(&self, query: SemanticQuery) -> VexGraphResult<Vec<SemanticSearchResult>> {
        let start_time = std::time::Instant::now();

        // Generate cache key
        let cache_key = self.generate_cache_key(&query);

        // Check result cache
        if let Some((cached_results, timestamp)) = self.result_cache.get(&cache_key) {
            let age = chrono::Utc::now().signed_duration_since(*timestamp);
            if age.num_seconds() < self.config.cache_ttl_seconds as i64 {
                let mut stats = self.stats.write();
                stats.cache_hits += 1;
                stats.total_searches += 1;
                return Ok(cached_results.clone());
            }
        }

        // Determine which indices to search
        let indices_to_search = if let Some(embedding_type) = query.embedding_type {
            // Search specific type
            let dimensions = query.query_vector.len();
            let index_key = (embedding_type, dimensions);
            if let Some(index) = self.indices.get(&index_key) {
                vec![(index_key, index.clone())]
            } else {
                vec![]
            }
        } else {
            // Search all indices with matching dimensions
            let dimensions = query.query_vector.len();
            self.indices.iter()
                .filter(|entry| entry.key().1 == dimensions)
                .map(|entry| (*entry.key(), entry.value().clone()))
                .collect()
        };

        let mut all_results = Vec::new();

        // Search each relevant index
        for ((_embedding_type, _dimensions), index) in indices_to_search {
            let index_guard = index.read();
            let mut index_results = index_guard.search(&query)?;
            all_results.append(&mut index_results);
        }

        // Sort by similarity score (descending)
        all_results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        // Apply graph constraints if specified
        if let Some(constraints) = &query.graph_constraints {
            all_results = self.apply_graph_constraints(all_results, constraints).await?;
        }

        // Limit results
        all_results.truncate(query.k);

        // Enrich results with node properties if requested
        if query.include_metadata {
            for result in &mut all_results {
                if let Ok(node) = self.core.get_node(result.node_id).await {
                    result.node_properties = Some(node.properties);
                }
            }
        }

        // Cache results
        self.result_cache.insert(cache_key, (all_results.clone(), chrono::Utc::now()));

        // Update statistics
        let search_time = start_time.elapsed().as_millis() as f64;
        {
            let mut stats = self.stats.write();
            stats.cache_misses += 1;
            stats.total_searches += 1;
            
            // Update average search time
            let total_time = stats.average_search_time_ms * (stats.total_searches - 1) as f64 + search_time;
            stats.average_search_time_ms = total_time / stats.total_searches as f64;
            
            stats.last_updated = chrono::Utc::now();
        }

        // Clean up caches if they're too large
        self.cleanup_caches();

        tracing::debug!("Semantic search completed in {:.2}ms, found {} results", search_time, all_results.len());
        Ok(all_results)
    }

    /// Find similar nodes within a graph neighborhood
    pub async fn find_similar_in_neighborhood(
        &self,
        center_node: NodeId,
        query_vector: Vec<f32>,
        max_hops: u32,
        k: usize,
        embedding_type: Option<EmbeddingType>,
    ) -> VexGraphResult<Vec<SemanticSearchResult>> {
        // First, get all nodes within the specified neighborhood
        let neighborhood_nodes = self.get_neighborhood_nodes(center_node, max_hops).await?;

        // Create a semantic query with graph constraints
        let constraints = GraphConstraints {
            start_nodes: Some(neighborhood_nodes),
            max_hops: Some(max_hops),
            edge_types: None,
            node_types: None,
            property_filters: None,
        };

        let query = SemanticQuery {
            query_vector,
            k,
            embedding_type,
            graph_constraints: Some(constraints),
            similarity_threshold: Some(self.config.default_similarity_threshold),
            include_metadata: true,
        };

        self.semantic_search(query).await
    }

    /// Get embeddings for a specific node
    pub async fn get_node_embeddings(&self, node_id: NodeId) -> VexGraphResult<Vec<VectorEmbedding>> {
        let mut embeddings = Vec::new();

        // Search through all indices
        for index_entry in self.indices.iter() {
            let index = index_entry.value().read();
            let node_embeddings = index.get_embeddings_for_node(node_id);
            embeddings.extend(node_embeddings);
        }

        Ok(embeddings)
    }

    /// Get statistics for all indices
    pub async fn get_statistics(&self) -> VexGraphResult<SemanticSearchStatistics> {
        let mut stats = self.stats.write();
        
        // Update memory usage
        stats.memory_usage_bytes = self.estimate_memory_usage();
        stats.total_indices = self.indices.len();
        
        // Count total embeddings across all indices
        let mut total_embeddings = 0;
        for index_entry in self.indices.iter() {
            let index = index_entry.value().read();
            let index_stats = index.get_statistics();
            total_embeddings += index_stats.total_embeddings;
        }
        stats.total_embeddings = total_embeddings;
        
        stats.last_updated = chrono::Utc::now();
        Ok(stats.clone())
    }

    /// Apply graph constraints to filter search results
    async fn apply_graph_constraints(
        &self,
        mut results: Vec<SemanticSearchResult>,
        constraints: &GraphConstraints,
    ) -> VexGraphResult<Vec<SemanticSearchResult>> {
        // Filter by start nodes if specified
        if let Some(start_nodes) = &constraints.start_nodes {
            let start_node_set: HashSet<NodeId> = start_nodes.iter().cloned().collect();
            results.retain(|result| start_node_set.contains(&result.node_id));
        }

        // Calculate graph distances if max_hops is specified
        if let Some(max_hops) = constraints.max_hops {
            for result in &mut results {
                if let Some(start_nodes) = &constraints.start_nodes {
                    let mut min_distance = None;
                    for &start_node in start_nodes {
                        if let Ok(distance) = self.calculate_graph_distance(start_node, result.node_id, max_hops).await {
                            min_distance = Some(min_distance.map_or(distance, |d| d.min(distance)));
                        }
                    }
                    result.graph_distance = min_distance;
                } else {
                    result.graph_distance = Some(0); // No constraint means distance is 0
                }
            }

            // Filter out results that exceed max_hops
            results.retain(|result| {
                result.graph_distance.map_or(false, |d| d <= max_hops)
            });
        }

        // Filter by node types if specified
        if let Some(node_types) = &constraints.node_types {
            let node_type_set: HashSet<_> = node_types.iter().cloned().collect();
            let mut filtered_results = Vec::new();
            
            for result in results {
                if let Ok(node) = self.core.get_node(result.node_id).await {
                    if node_type_set.contains(&node.node_type) {
                        filtered_results.push(result);
                    }
                }
            }
            results = filtered_results;
        }

        // Filter by property filters if specified
        if let Some(property_filters) = &constraints.property_filters {
            let mut filtered_results = Vec::new();
            
            for result in results {
                if let Ok(node) = self.core.get_node(result.node_id).await {
                    let mut matches = true;
                    for (key, expected_value) in property_filters {
                        if let Some(actual_value) = node.properties.get(key) {
                            if actual_value != expected_value {
                                matches = false;
                                break;
                            }
                        } else {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        filtered_results.push(result);
                    }
                }
            }
            results = filtered_results;
        }

        Ok(results)
    }

    /// Get all nodes within a specified number of hops from a center node
    async fn get_neighborhood_nodes(&self, center_node: NodeId, max_hops: u32) -> VexGraphResult<Vec<NodeId>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((center_node, 0));
        visited.insert(center_node);

        while let Some((current_node, hops)) = queue.pop_front() {
            result.push(current_node);

            if hops < max_hops {
                // Get outgoing edges
                if let Ok(outgoing_edges) = self.core.get_outgoing_edges(current_node).await {
                    for edge_id in outgoing_edges {
                        if let Ok(edge) = self.core.get_edge(edge_id).await {
                            if !visited.contains(&edge.target_id) {
                                visited.insert(edge.target_id);
                                queue.push_back((edge.target_id, hops + 1));
                            }
                        }
                    }
                }

                // Get incoming edges
                if let Ok(incoming_edges) = self.core.get_incoming_edges(current_node).await {
                    for edge_id in incoming_edges {
                        if let Ok(edge) = self.core.get_edge(edge_id).await {
                            if !visited.contains(&edge.source_id) {
                                visited.insert(edge.source_id);
                                queue.push_back((edge.source_id, hops + 1));
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Calculate the shortest graph distance between two nodes
    async fn calculate_graph_distance(&self, start: NodeId, end: NodeId, max_hops: u32) -> VexGraphResult<u32> {
        if start == end {
            return Ok(0);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((start, 0));
        visited.insert(start);

        while let Some((current_node, distance)) = queue.pop_front() {
            if distance >= max_hops {
                continue;
            }

            // Check outgoing edges
            if let Ok(outgoing_edges) = self.core.get_outgoing_edges(current_node).await {
                for edge_id in outgoing_edges {
                    if let Ok(edge) = self.core.get_edge(edge_id).await {
                        if edge.target_id == end {
                            return Ok(distance + 1);
                        }
                        if !visited.contains(&edge.target_id) {
                            visited.insert(edge.target_id);
                            queue.push_back((edge.target_id, distance + 1));
                        }
                    }
                }
            }

            // Check incoming edges
            if let Ok(incoming_edges) = self.core.get_incoming_edges(current_node).await {
                for edge_id in incoming_edges {
                    if let Ok(edge) = self.core.get_edge(edge_id).await {
                        if edge.source_id == end {
                            return Ok(distance + 1);
                        }
                        if !visited.contains(&edge.source_id) {
                            visited.insert(edge.source_id);
                            queue.push_back((edge.source_id, distance + 1));
                        }
                    }
                }
            }
        }

        Err(VexGraphError::NotFound(format!("No path found between nodes {} and {}", start, end)))
    }

    /// Generate a cache key for a semantic query
    fn generate_cache_key(&self, query: &SemanticQuery) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // Hash the query vector (simplified)
        for &value in &query.query_vector {
            value.to_bits().hash(&mut hasher);
        }
        
        query.k.hash(&mut hasher);
        query.embedding_type.hash(&mut hasher);
        query.similarity_threshold.map(|f| f.to_bits()).hash(&mut hasher);
        query.include_metadata.hash(&mut hasher);
        
        // Hash graph constraints (simplified)
        if let Some(constraints) = &query.graph_constraints {
            constraints.start_nodes.hash(&mut hasher);
            constraints.max_hops.hash(&mut hasher);
            constraints.edge_types.hash(&mut hasher);
            constraints.node_types.hash(&mut hasher);
        }

        format!("semantic_query_{:x}", hasher.finish())
    }

    /// Clear the result cache
    fn clear_result_cache(&self) {
        self.result_cache.clear();
    }

    /// Clean up caches if they exceed size limits
    fn cleanup_caches(&self) {
        // Clean up embedding cache
        if self.embedding_cache.len() > self.config.max_embedding_cache_size {
            let excess = self.embedding_cache.len() - self.config.max_embedding_cache_size;
            let mut removed = 0;
            
            // Remove oldest entries (simplified LRU)
            let keys_to_remove: Vec<_> = self.embedding_cache.iter()
                .take(excess)
                .map(|entry| *entry.key())
                .collect();
            
            for key in keys_to_remove {
                self.embedding_cache.remove(&key);
                removed += 1;
                if removed >= excess {
                    break;
                }
            }
        }

        // Clean up result cache
        if self.result_cache.len() > self.config.max_result_cache_size {
            let excess = self.result_cache.len() - self.config.max_result_cache_size;
            let mut removed = 0;
            
            let keys_to_remove: Vec<_> = self.result_cache.iter()
                .take(excess)
                .map(|entry| entry.key().clone())
                .collect();
            
            for key in keys_to_remove {
                self.result_cache.remove(&key);
                removed += 1;
                if removed >= excess {
                    break;
                }
            }
        }
    }

    /// Estimate total memory usage
    fn estimate_memory_usage(&self) -> usize {
        let embedding_cache_size = self.embedding_cache.len() * 1000; // Rough estimate
        let result_cache_size = self.result_cache.len() * 500; // Rough estimate
        
        let mut index_memory = 0;
        for index_entry in self.indices.iter() {
            let index = index_entry.value().read();
            let stats = index.get_statistics();
            index_memory += stats.memory_usage;
        }
        
        embedding_cache_size + result_cache_size + index_memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vexgraph::{NodeType, VexGraphConfig};

    #[tokio::test]
    async fn test_semantic_search_manager_creation() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let search_config = SemanticSearchConfig::default();
        
        let manager = SemanticSearchManager::new(core, search_config).await.unwrap();
        assert_eq!(manager.indices.len(), 0);
    }

    #[tokio::test]
    async fn test_embedding_operations() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let search_config = SemanticSearchConfig::default();
        let manager = SemanticSearchManager::new(core.clone(), search_config).await.unwrap();

        // Create a test node
        let node_id = core.create_node(123, NodeType::File).await.unwrap();

        // Add an embedding
        let vector = vec![1.0, 2.0, 3.0, 4.0];
        let embedding_id = manager.add_embedding(
            node_id,
            EmbeddingType::Text,
            vector.clone(),
            None,
        ).await.unwrap();

        // Get embeddings for the node
        let embeddings = manager.get_node_embeddings(node_id).await.unwrap();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].id, embedding_id);
        assert_eq!(embeddings[0].vector, vector);

        // Remove the embedding
        assert!(manager.remove_embedding(embedding_id).await.is_ok());

        // Verify it's removed
        let embeddings = manager.get_node_embeddings(node_id).await.unwrap();
        assert_eq!(embeddings.len(), 0);
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let search_config = SemanticSearchConfig::default();
        let manager = SemanticSearchManager::new(core.clone(), search_config).await.unwrap();

        // Create test nodes and embeddings
        let node1 = core.create_node(123, NodeType::File).await.unwrap();
        let node2 = core.create_node(456, NodeType::File).await.unwrap();

        let vector1 = vec![1.0, 0.0, 0.0, 0.0];
        let vector2 = vec![0.0, 1.0, 0.0, 0.0];

        manager.add_embedding(node1, EmbeddingType::Text, vector1.clone(), None).await.unwrap();
        manager.add_embedding(node2, EmbeddingType::Text, vector2.clone(), None).await.unwrap();

        // Search for similar vectors
        let query = SemanticQuery {
            query_vector: vec![1.0, 0.1, 0.0, 0.0], // More similar to vector1
            k: 2,
            embedding_type: Some(EmbeddingType::Text),
            graph_constraints: None,
            similarity_threshold: None,
            include_metadata: false,
        };

        let results = manager.semantic_search(query).await.unwrap();
        assert_eq!(results.len(), 2);
        
        // First result should be more similar (node1)
        assert_eq!(results[0].node_id, node1);
        assert!(results[0].similarity_score > results[1].similarity_score);
    }
}