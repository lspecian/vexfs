//! Multi-dialect API support for VexFS
//! 
//! This module provides a unified server that can serve multiple vector database APIs
//! (ChromaDB, Qdrant, Native VexFS) using the same high-performance VexFS backend.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

// Re-export VexFS core types
pub use crate::shared::types::*;
pub use crate::shared::errors::*;

/// Common API dialect trait
pub trait ApiDialect {
    /// Handle a request for this dialect
    fn handle_request(&self, path: &str, method: &str, body: &[u8]) -> VexfsResult<Vec<u8>>;
    
    /// Get the URL prefix for this dialect
    fn url_prefix(&self) -> &str;
    
    /// Get dialect name
    fn name(&self) -> &str;
}

/// Shared VexFS engine for all dialects
#[derive(Clone)]
pub struct VexFSEngine {
    collections: Arc<std::sync::Mutex<HashMap<String, Collection>>>,
}

impl VexFSEngine {
    pub fn new() -> Self {
        Self {
            collections: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
    
    pub fn create_collection(&self, name: String, metadata: Option<CollectionMetadata>) -> VexfsResult<()> {
        let mut collections = self.collections.lock().map_err(|_| VexfsError::LockError)?;
        let collection = Collection {
            name: name.clone(),
            metadata: metadata.unwrap_or_default(),
            documents: HashMap::new(),
        };
        collections.insert(name, collection);
        Ok(())
    }
    
    pub fn get_collection(&self, name: &str) -> VexfsResult<Option<Collection>> {
        let collections = self.collections.lock().map_err(|_| VexfsError::LockError)?;
        Ok(collections.get(name).cloned())
    }
    
    pub fn list_collections(&self) -> VexfsResult<Vec<String>> {
        let collections = self.collections.lock().map_err(|_| VexfsError::LockError)?;
        Ok(collections.keys().cloned().collect())
    }
    
    pub fn add_documents(&self, collection_name: &str, documents: Vec<Document>) -> VexfsResult<()> {
        let mut collections = self.collections.lock().map_err(|_| VexfsError::LockError)?;
        if let Some(collection) = collections.get_mut(collection_name) {
            for doc in documents {
                collection.documents.insert(doc.id.clone(), doc);
            }
            Ok(())
        } else {
            Err(VexfsError::NotFound)
        }
    }
    
    pub fn query_collection(&self, collection_name: &str, query_vector: Vec<f32>, n_results: usize) -> VexfsResult<Vec<QueryResult>> {
        let collections = self.collections.lock().map_err(|_| VexfsError::LockError)?;
        if let Some(collection) = collections.get(collection_name) {
            // Simple similarity search - in real implementation this would use VexFS ANNS
            let mut results: Vec<QueryResult> = collection.documents.values()
                .filter_map(|doc| {
                    if let Some(embedding) = &doc.embedding {
                        let distance = cosine_distance(&query_vector, embedding);
                        Some(QueryResult {
                            id: doc.id.clone(),
                            distance,
                            metadata: doc.metadata.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            
            results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(n_results);
            Ok(results)
        } else {
            Err(VexfsError::NotFound)
        }
    }
}

/// Collection representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub metadata: CollectionMetadata,
    pub documents: HashMap<String, Document>,
}

/// Collection metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionMetadata {
    pub dimension: Option<usize>,
    pub distance_function: DistanceFunction,
    pub description: Option<String>,
}

/// Document representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub content: Option<String>,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub id: String,
    pub distance: f32,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Distance function types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DistanceFunction {
    #[default]
    Cosine,
    Euclidean,
    DotProduct,
}

/// Simple cosine distance calculation
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return f32::INFINITY;
    }
    
    1.0 - (dot_product / (norm_a * norm_b))
}

// Sub-modules for specific dialects
pub mod chromadb;
pub mod qdrant;
pub mod qdrant_optimized;
pub mod native;
pub mod router;