/*
 * VexFS v2.0 - VexGraph Semantic Search Integration (Task 11)
 * 
 * This module implements semantic search integration for VexGraph, combining
 * graph traversal with vector similarity search using FAISS for efficient
 * nearest neighbor operations.
 */

use crate::vexgraph::{
    NodeId, EdgeId, PropertyType, VexGraphConfig,
    core::{VexGraphCore, GraphNode, GraphEdge},
    error_handling::{VexGraphError, VexGraphResult},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use uuid::Uuid;

#[cfg(feature = "semantic_search")]
use faiss::{Index, IndexFlat, MetricType};

/// Vector embedding types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EmbeddingType {
    Text = 0x01,
    Image = 0x02,
    Audio = 0x03,
    Custom = 0x04,
    Multimodal = 0x05,
}

/// Vector embedding representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbedding {
    pub id: Uuid,
    pub node_id: NodeId,
    pub embedding_type: EmbeddingType,
    pub dimensions: usize,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl VectorEmbedding {
    pub fn new(
        node_id: NodeId,
        embedding_type: EmbeddingType,
        vector: Vec<f32>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            node_id,
            embedding_type,
            dimensions: vector.len(),
            vector,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn update_vector(&mut self, vector: Vec<f32>) -> VexGraphResult<()> {
        if vector.len() != self.dimensions {
            return Err(VexGraphError::InvalidArgument(
                format!("Vector dimension mismatch: expected {}, got {}", 
                       self.dimensions, vector.len())
            ));
        }
        self.vector = vector;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}

/// Semantic search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub query_vector: Vec<f32>,
    pub k: usize,
    pub embedding_type: Option<EmbeddingType>,
    pub graph_constraints: Option<GraphConstraints>,
    pub similarity_threshold: Option<f32>,
    pub include_metadata: bool,
}

/// Graph constraints for semantic queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConstraints {
    pub start_nodes: Option<Vec<NodeId>>,
    pub max_hops: Option<u32>,
    pub edge_types: Option<Vec<crate::vexgraph::EdgeType>>,
    pub node_types: Option<Vec<crate::vexgraph::NodeType>>,
    pub property_filters: Option<HashMap<String, PropertyType>>,
}

/// Semantic search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub node_id: NodeId,
    pub embedding_id: Uuid,
    pub similarity_score: f32,
    pub distance: f32,
    pub graph_distance: Option<u32>,
    pub metadata: HashMap<String, String>,
    pub node_properties: Option<HashMap<String, PropertyType>>,
}

/// Hybrid index combining FAISS with graph topology
#[derive(Debug)]
pub struct HybridIndex {
    #[cfg(feature = "semantic_search")]
    faiss_index: Option<Box<dyn Index>>,
    embedding_map: DashMap<i64, VectorEmbedding>,
    node_embeddings: DashMap<NodeId, Vec<Uuid>>,
    type_indices: DashMap<EmbeddingType, Vec<i64>>,
    dimensions: usize,
    next_faiss_id: parking_lot::Mutex<i64>,
}

impl HybridIndex {
    pub fn new(dimensions: usize) -> VexGraphResult<Self> {
        #[cfg(feature = "semantic_search")]
        let faiss_index = {
            let mut index = IndexFlat::new(dimensions, MetricType::L2)
                .map_err(|e| VexGraphError::SemanticSearchError(format!("FAISS index creation failed: {}", e)))?;
            Some(Box::new(index) as Box<dyn Index>)
        };

        #[cfg(not(feature = "semantic_search"))]
        let faiss_index = None;

        Ok(Self {
            faiss_index,
            embedding_map: DashMap::new(),
            node_embeddings: DashMap::new(),
            type_indices: DashMap::new(),
            dimensions,
            next_faiss_id: parking_lot::Mutex::new(0),
        })
    }

    pub fn add_embedding(&self, embedding: VectorEmbedding) -> VexGraphResult<()> {
        if embedding.dimensions != self.dimensions {
            return Err(VexGraphError::InvalidArgument(
                format!("Embedding dimension mismatch: expected {}, got {}", 
                       self.dimensions, embedding.dimensions)
            ));
        }

        let faiss_id = {
            let mut next_id = self.next_faiss_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        #[cfg(feature = "semantic_search")]
        if let Some(ref index) = self.faiss_index {
            index.add(&[embedding.vector.as_slice()])
                .map_err(|e| VexGraphError::SemanticSearchError(format!("FAISS add failed: {}", e)))?;
        }

        // Update indices
        self.node_embeddings
            .entry(embedding.node_id)
            .or_insert_with(Vec::new)
            .push(embedding.id);

        self.type_indices
            .entry(embedding.embedding_type)
            .or_insert_with(Vec::new)
            .push(faiss_id);

        self.embedding_map.insert(faiss_id, embedding);

        Ok(())
    }

    pub fn search(&self, query: &SemanticQuery) -> VexGraphResult<Vec<SemanticSearchResult>> {
        if query.query_vector.len() != self.dimensions {
            return Err(VexGraphError::InvalidArgument(
                format!("Query vector dimension mismatch: expected {}, got {}", 
                       self.dimensions, query.query_vector.len())
            ));
        }

        #[cfg(feature = "semantic_search")]
        if let Some(ref index) = self.faiss_index {
            let (distances, indices) = index.search(&[query.query_vector.as_slice()], query.k)
                .map_err(|e| VexGraphError::SemanticSearchError(format!("FAISS search failed: {}", e)))?;

            let mut results = Vec::new();
            for (i, &faiss_id) in indices[0].iter().enumerate() {
                if faiss_id < 0 {
                    continue; // Invalid result
                }

                if let Some(embedding) = self.embedding_map.get(&faiss_id) {
                    // Apply embedding type filter
                    if let Some(filter_type) = query.embedding_type {
                        if embedding.embedding_type != filter_type {
                            continue;
                        }
                    }

                    let distance = distances[0][i];
                    let similarity_score = 1.0 / (1.0 + distance);

                    // Apply similarity threshold
                    if let Some(threshold) = query.similarity_threshold {
                        if similarity_score < threshold {
                            continue;
                        }
                    }

                    let result = SemanticSearchResult {
                        node_id: embedding.node_id,
                        embedding_id: embedding.id,
                        similarity_score,
                        distance,
                        graph_distance: None, // Will be filled by graph traversal
                        metadata: if query.include_metadata {
                            embedding.metadata.clone()
                        } else {
                            HashMap::new()
                        },
                        node_properties: None, // Will be filled if requested
                    };

                    results.push(result);
                }
            }

            Ok(results)
        } else {
            #[cfg(not(feature = "semantic_search"))]
            {
                // Fallback implementation without FAISS
                let mut results = Vec::new();
                for embedding_entry in self.embedding_map.iter().take(query.k) {
                    let embedding = embedding_entry.value();
                    
                    // Apply embedding type filter
                    if let Some(filter_type) = query.embedding_type {
                        if embedding.embedding_type != filter_type {
                            continue;
                        }
                    }

                    // Calculate cosine similarity
                    let similarity_score = cosine_similarity(&query.query_vector, &embedding.vector);
                    
                    // Apply similarity threshold
                    if let Some(threshold) = query.similarity_threshold {
                        if similarity_score < threshold {
                            continue;
                        }
                    }

                    let result = SemanticSearchResult {
                        node_id: embedding.node_id,
                        embedding_id: embedding.id,
                        similarity_score,
                        distance: 1.0 - similarity_score,
                        graph_distance: None,
                        metadata: if query.include_metadata {
                            embedding.metadata.clone()
                        } else {
                            HashMap::new()
                        },
                        node_properties: None,
                    };

                    results.push(result);
                }

                // Sort by similarity score (descending)
                results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
                results.truncate(query.k);

                Ok(results)
            }
        }
    }

    pub fn remove_embedding(&self, embedding_id: Uuid) -> VexGraphResult<()> {
        // Find and remove the embedding
        let mut faiss_id_to_remove = None;
        let mut node_id_to_update = None;
        let mut embedding_type_to_update = None;

        for entry in self.embedding_map.iter() {
            if entry.value().id == embedding_id {
                faiss_id_to_remove = Some(*entry.key());
                node_id_to_update = Some(entry.value().node_id);
                embedding_type_to_update = Some(entry.value().embedding_type);
                break;
            }
        }

        if let Some(faiss_id) = faiss_id_to_remove {
            self.embedding_map.remove(&faiss_id);

            // Update node embeddings index
            if let Some(node_id) = node_id_to_update {
                if let Some(mut embeddings) = self.node_embeddings.get_mut(&node_id) {
                    embeddings.retain(|&id| id != embedding_id);
                    if embeddings.is_empty() {
                        drop(embeddings);
                        self.node_embeddings.remove(&node_id);
                    }
                }
            }

            // Update type index
            if let Some(embedding_type) = embedding_type_to_update {
                if let Some(mut type_embeddings) = self.type_indices.get_mut(&embedding_type) {
                    type_embeddings.retain(|&id| id != faiss_id);
                    if type_embeddings.is_empty() {
                        drop(type_embeddings);
                        self.type_indices.remove(&embedding_type);
                    }
                }
            }

            // Note: FAISS doesn't support efficient removal, so we keep the index as-is
            // In a production system, you might want to rebuild the index periodically
        }

        Ok(())
    }

    pub fn get_embeddings_for_node(&self, node_id: NodeId) -> Vec<VectorEmbedding> {
        if let Some(embedding_ids) = self.node_embeddings.get(&node_id) {
            let mut embeddings = Vec::new();
            for embedding_id in embedding_ids.iter() {
                for entry in self.embedding_map.iter() {
                    if entry.value().id == *embedding_id {
                        embeddings.push(entry.value().clone());
                        break;
                    }
                }
            }
            embeddings
        } else {
            Vec::new()
        }
    }

    pub fn get_statistics(&self) -> HybridIndexStatistics {
        HybridIndexStatistics {
            total_embeddings: self.embedding_map.len(),
            nodes_with_embeddings: self.node_embeddings.len(),
            embedding_types: self.type_indices.len(),
            dimensions: self.dimensions,
            memory_usage: self.estimate_memory_usage(),
        }
    }

    fn estimate_memory_usage(&self) -> usize {
        // Rough estimation
        let embedding_memory = self.embedding_map.len() * (self.dimensions * 4 + 200); // 4 bytes per f32 + overhead
        let index_memory = self.node_embeddings.len() * 50 + self.type_indices.len() * 50;
        embedding_memory + index_memory
    }
}

/// Statistics for hybrid index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridIndexStatistics {
    pub total_embeddings: usize,
    pub nodes_with_embeddings: usize,
    pub embedding_types: usize,
    pub dimensions: usize,
    pub memory_usage: usize,
}

/// Cosine similarity calculation (fallback when FAISS is not available)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_embedding_creation() {
        let vector = vec![1.0, 2.0, 3.0, 4.0];
        let embedding = VectorEmbedding::new(123, EmbeddingType::Text, vector.clone());
        
        assert_eq!(embedding.node_id, 123);
        assert_eq!(embedding.embedding_type, EmbeddingType::Text);
        assert_eq!(embedding.dimensions, 4);
        assert_eq!(embedding.vector, vector);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_index_creation() {
        let index = HybridIndex::new(128).unwrap();
        assert_eq!(index.dimensions, 128);
        assert_eq!(index.embedding_map.len(), 0);
    }

    #[test]
    fn test_embedding_operations() {
        let index = HybridIndex::new(4).unwrap();
        let embedding = VectorEmbedding::new(123, EmbeddingType::Text, vec![1.0, 2.0, 3.0, 4.0]);
        
        assert!(index.add_embedding(embedding.clone()).is_ok());
        
        let embeddings = index.get_embeddings_for_node(123);
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].id, embedding.id);
    }
}