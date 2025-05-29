/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! VexFS Main API
//!
//! This module provides the main VexFS API that includes both native VexFS operations
//! and ChromaDB-compatible methods for easy migration from ChromaDB.

use crate::shared::errors::{VexfsResult, VexfsError};
use crate::chromadb_api::*;
use std::collections::HashMap;
use std::path::Path;

/// Main VexFS API client that provides both native and ChromaDB-compatible operations
pub struct VexFS {
    /// ChromaDB-compatible API instance
    chroma_api: ChromaDBApi,
    /// Mount point for the VexFS filesystem
    mount_point: Option<String>,
    /// Whether the filesystem is initialized
    initialized: bool,
}

impl VexFS {
    /// Create a new VexFS instance
    pub fn new() -> Self {
        Self {
            chroma_api: ChromaDBApi::new(),
            mount_point: None,
            initialized: false,
        }
    }

    /// Initialize VexFS with a mount point
    pub fn init<P: AsRef<Path>>(mount_point: P) -> VexfsResult<Self> {
        let mount_str = mount_point.as_ref().to_string_lossy().to_string();
        
        // TODO: Initialize actual VexFS filesystem at mount point
        // For now, we'll just track the mount point
        
        Ok(Self {
            chroma_api: ChromaDBApi::new(),
            mount_point: Some(mount_str),
            initialized: true,
        })
    }

    /// Get VexFS version information
    pub fn version(&self) -> HashMap<String, String> {
        self.chroma_api.version()
    }

    // ========================================
    // ChromaDB-Compatible API Methods
    // ========================================

    /// Create a new collection (ChromaDB-compatible)
    pub fn create_collection(
        &mut self,
        name: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
        distance_function: Option<DistanceFunction>,
    ) -> VexfsResult<Collection> {
        self.chroma_api.create_collection(name, metadata, distance_function)
    }

    /// List all collections (ChromaDB-compatible)
    pub fn list_collections(&self) -> Vec<Collection> {
        self.chroma_api.list_collections()
    }

    /// Get a specific collection by name (ChromaDB-compatible)
    pub fn get_collection(&self, name: &str) -> VexfsResult<Collection> {
        self.chroma_api.get_collection(name)
    }

    /// Delete a collection (ChromaDB-compatible)
    pub fn delete_collection(&mut self, name: &str) -> VexfsResult<()> {
        self.chroma_api.delete_collection(name)
    }

    /// Add documents to a collection (ChromaDB-compatible)
    pub fn add_documents(
        &mut self,
        collection_name: &str,
        ids: Vec<String>,
        embeddings: Vec<Vec<f32>>,
        documents: Option<Vec<String>>,
        metadatas: Option<Vec<HashMap<String, serde_json::Value>>>,
    ) -> VexfsResult<String> {
        self.chroma_api.add_documents(collection_name, ids, embeddings, documents, metadatas)
    }

    /// Query a collection for similar vectors (ChromaDB-compatible)
    pub fn query_collection(
        &self,
        collection_name: &str,
        query_embeddings: Vec<Vec<f32>>,
        n_results: Option<usize>,
        where_clause: Option<HashMap<String, serde_json::Value>>,
        include: Option<Vec<String>>,
    ) -> VexfsResult<QueryResult> {
        self.chroma_api.query_collection(collection_name, query_embeddings, n_results, where_clause, include)
    }

    /// Update documents in a collection (ChromaDB-compatible)
    pub fn update_documents(
        &mut self,
        collection_name: &str,
        ids: Vec<String>,
        embeddings: Option<Vec<Vec<f32>>>,
        documents: Option<Vec<String>>,
        metadatas: Option<Vec<HashMap<String, serde_json::Value>>>,
    ) -> VexfsResult<String> {
        self.chroma_api.update_documents(collection_name, ids, embeddings, documents, metadatas)
    }

    /// Delete documents from a collection (ChromaDB-compatible)
    pub fn delete_documents(
        &mut self,
        collection_name: &str,
        ids: Vec<String>,
    ) -> VexfsResult<String> {
        self.chroma_api.delete_documents(collection_name, ids)
    }

    /// Get collection statistics (ChromaDB-compatible)
    pub fn get_collection_stats(&self, collection_name: &str) -> VexfsResult<HashMap<String, serde_json::Value>> {
        self.chroma_api.get_collection_stats(collection_name)
    }

    // ========================================
    // Native VexFS API Methods
    // ========================================

    /// Add a document with automatic embedding generation (VexFS native)
    pub fn add(&mut self, document: &str, metadata: Option<HashMap<String, serde_json::Value>>) -> VexfsResult<String> {
        // TODO: Implement automatic embedding generation using VexFS's native capabilities
        // For now, we'll create a simple embedding and use the ChromaDB API
        
        let doc_id = uuid::Uuid::new_v4().to_string();
        let embedding = self.generate_simple_embedding(document);
        
        // Use default collection if none specified
        let collection_name = "default";
        
        // Ensure default collection exists
        if self.chroma_api.get_collection(collection_name).is_err() {
            self.chroma_api.create_collection(
                collection_name.to_string(),
                None,
                Some(DistanceFunction::Cosine),
            )?;
        }

        self.chroma_api.add_documents(
            collection_name,
            vec![doc_id.clone()],
            vec![embedding],
            Some(vec![document.to_string()]),
            metadata.map(|m| vec![m]),
        )?;

        Ok(doc_id)
    }

    /// Query for similar documents (VexFS native)
    pub fn query(&self, query_vector: Vec<f32>, top_k: usize) -> VexfsResult<Vec<(String, f32, Option<String>)>> {
        let collection_name = "default";
        
        let result = self.chroma_api.query_collection(
            collection_name,
            vec![query_vector],
            Some(top_k),
            None,
            None,
        )?;

        let mut results = Vec::new();
        if let Some(ids) = result.ids.get(0) {
            for (i, id) in ids.iter().enumerate() {
                let distance = result.distances[0][i];
                let document = result.documents.as_ref()
                    .and_then(|docs| docs[0].get(i))
                    .and_then(|doc| doc.as_ref())
                    .cloned();
                
                results.push((id.clone(), distance, document));
            }
        }

        Ok(results)
    }

    /// Delete a document by ID (VexFS native)
    pub fn delete(&mut self, doc_id: &str) -> VexfsResult<()> {
        let collection_name = "default";
        self.chroma_api.delete_documents(collection_name, vec![doc_id.to_string()])?;
        Ok(())
    }

    /// Get filesystem statistics (VexFS native)
    pub fn stats(&self) -> VexfsResult<HashMap<String, serde_json::Value>> {
        let mut stats = HashMap::new();
        
        // Add VexFS-specific stats
        stats.insert("mount_point".to_string(), 
            serde_json::Value::String(self.mount_point.clone().unwrap_or_else(|| "not_mounted".to_string())));
        stats.insert("initialized".to_string(), serde_json::Value::Bool(self.initialized));
        stats.insert("api_version".to_string(), serde_json::Value::String("1.0.0".to_string()));
        
        // Add collection stats
        let collections = self.list_collections();
        stats.insert("collection_count".to_string(), serde_json::Value::Number(collections.len().into()));
        
        let mut total_documents = 0;
        for collection in &collections {
            if let Ok(collection_stats) = self.get_collection_stats(&collection.name) {
                if let Some(doc_count) = collection_stats.get("document_count") {
                    if let Some(count) = doc_count.as_u64() {
                        total_documents += count;
                    }
                }
            }
        }
        stats.insert("total_documents".to_string(), serde_json::Value::Number(total_documents.into()));

        Ok(stats)
    }

    // ========================================
    // Helper Methods
    // ========================================

    /// Generate a simple embedding for a document (placeholder implementation)
    fn generate_simple_embedding(&self, document: &str) -> Vec<f32> {
        // This is a very simple embedding generation for demonstration
        // In a real implementation, this would use a proper embedding model
        
        let words: Vec<&str> = document.split_whitespace().collect();
        let mut embedding = vec![0.0; 384]; // Standard embedding dimension
        
        // Simple hash-based embedding
        for (i, word) in words.iter().enumerate() {
            let hash = self.simple_hash(word) as usize;
            let idx = hash % embedding.len();
            embedding[idx] += 1.0 / (i + 1) as f32;
        }
        
        // Normalize the embedding
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }
        
        embedding
    }

    /// Simple hash function for words
    fn simple_hash(&self, s: &str) -> u32 {
        let mut hash = 0u32;
        for byte in s.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        hash
    }
}

impl Default for VexFS {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vexfs_creation() {
        let vexfs = VexFS::new();
        assert!(!vexfs.initialized);
        assert!(vexfs.mount_point.is_none());
    }

    #[test]
    fn test_vexfs_init() {
        let vexfs = VexFS::init("/tmp/test_vexfs").unwrap();
        assert!(vexfs.initialized);
        assert_eq!(vexfs.mount_point, Some("/tmp/test_vexfs".to_string()));
    }

    #[test]
    fn test_chromadb_compatibility() {
        let mut vexfs = VexFS::new();
        
        // Test collection creation
        let collection = vexfs.create_collection(
            "test_collection".to_string(),
            None,
            Some(DistanceFunction::Cosine),
        ).unwrap();
        
        assert_eq!(collection.name, "test_collection");
        
        // Test document addition
        let result = vexfs.add_documents(
            "test_collection",
            vec!["doc1".to_string()],
            vec![vec![1.0, 0.0, 0.0]],
            Some(vec!["Test document".to_string()]),
            None,
        ).unwrap();
        
        assert!(result.contains("Added 1 documents"));
    }

    #[test]
    fn test_native_api() {
        let mut vexfs = VexFS::new();
        
        // Test native add method
        let doc_id = vexfs.add("Hello world", None).unwrap();
        assert!(!doc_id.is_empty());
        
        // Test stats
        let stats = vexfs.stats().unwrap();
        assert!(stats.contains_key("api_version"));
        assert!(stats.contains_key("collection_count"));
    }

    #[test]
    fn test_embedding_generation() {
        let vexfs = VexFS::new();
        let embedding = vexfs.generate_simple_embedding("hello world test");
        
        assert_eq!(embedding.len(), 384);
        
        // Check that embedding is normalized
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }
}