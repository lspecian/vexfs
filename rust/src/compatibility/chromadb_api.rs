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

//! ChromaDB-compatible API for VexFS
//!
//! This module provides ChromaDB-compatible operations using VexFS's native
//! vector storage and search capabilities. It allows existing ChromaDB applications
//! to use VexFS as a drop-in replacement with superior performance.

use crate::shared::errors::{VexfsResult, VexfsError};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ChromaDB-compatible collection representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub dimension: Option<usize>,
    pub distance_function: DistanceFunction,
}

/// Supported distance functions (ChromaDB-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistanceFunction {
    #[serde(rename = "l2")]
    Euclidean,
    #[serde(rename = "cosine")]
    Cosine,
    #[serde(rename = "ip")]
    InnerProduct,
}

/// Document with embedding and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub embedding: Vec<f32>,
    pub document: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Query result with distance and ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub ids: Vec<Vec<String>>,
    pub distances: Vec<Vec<f32>>,
    pub documents: Option<Vec<Vec<Option<String>>>>,
    pub metadatas: Option<Vec<Vec<Option<HashMap<String, serde_json::Value>>>>>,
}

/// ChromaDB-compatible API client for VexFS
pub struct ChromaDBApi {
    collections: HashMap<String, Collection>,
    documents: HashMap<String, HashMap<String, Document>>, // collection_name -> doc_id -> document
}

impl ChromaDBApi {
    /// Create a new ChromaDB API instance
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
            documents: HashMap::new(),
        }
    }

    /// Get server version information
    pub fn version(&self) -> HashMap<String, String> {
        let mut version = HashMap::new();
        version.insert("version".to_string(), "1.0.0".to_string());
        version.insert("server".to_string(), "VexFS".to_string());
        version.insert("compatibility".to_string(), "ChromaDB".to_string());
        version
    }

    /// Create a new collection
    pub fn create_collection(
        &mut self,
        name: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
        distance_function: Option<DistanceFunction>,
    ) -> VexfsResult<Collection> {
        if self.collections.contains_key(&name) {
            return Err(VexfsError::InvalidOperation(
                format!("Collection '{}' already exists", name)
            ));
        }

        let collection = Collection {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            metadata,
            dimension: None,
            distance_function: distance_function.unwrap_or(DistanceFunction::Euclidean),
        };

        self.collections.insert(name.clone(), collection.clone());
        self.documents.insert(name, HashMap::new());

        Ok(collection)
    }

    /// Get all collections
    pub fn list_collections(&self) -> Vec<Collection> {
        self.collections.values().cloned().collect()
    }

    /// Get a specific collection by name
    pub fn get_collection(&self, name: &str) -> VexfsResult<Collection> {
        self.collections
            .get(name)
            .cloned()
            .ok_or_else(|| VexfsError::EntryNotFound(format!("Collection '{}' not found", name)))
    }

    /// Delete a collection
    pub fn delete_collection(&mut self, name: &str) -> VexfsResult<()> {
        if !self.collections.contains_key(name) {
            return Err(VexfsError::EntryNotFound(format!("Collection '{}' not found", name)));
        }

        self.collections.remove(name);
        self.documents.remove(name);
        Ok(())
    }

    /// Add documents to a collection
    pub fn add_documents(
        &mut self,
        collection_name: &str,
        ids: Vec<String>,
        embeddings: Vec<Vec<f32>>,
        documents: Option<Vec<String>>,
        metadatas: Option<Vec<HashMap<String, serde_json::Value>>>,
    ) -> VexfsResult<String> {
        // Validate collection exists
        let mut collection = self.get_collection(collection_name)?;

        // Validate input lengths
        if embeddings.len() != ids.len() {
            return Err(VexfsError::InvalidOperation(
                "Number of embeddings must match number of IDs".to_string()
            ));
        }

        if let Some(ref docs) = documents {
            if docs.len() != ids.len() {
                return Err(VexfsError::InvalidOperation(
                    "Number of documents must match number of IDs".to_string()
                ));
            }
        }

        if let Some(ref metas) = metadatas {
            if metas.len() != ids.len() {
                return Err(VexfsError::InvalidOperation(
                    "Number of metadatas must match number of IDs".to_string()
                ));
            }
        }

        // Set collection dimension from first embedding if not set
        if collection.dimension.is_none() && !embeddings.is_empty() {
            collection.dimension = Some(embeddings[0].len());
            self.collections.insert(collection_name.to_string(), collection);
        }

        // Validate embedding dimensions
        if let Some(expected_dim) = self.collections.get(collection_name).unwrap().dimension {
            for (i, embedding) in embeddings.iter().enumerate() {
                if embedding.len() != expected_dim {
                    return Err(VexfsError::InvalidOperation(
                        format!("Embedding {} has dimension {}, expected {}", i, embedding.len(), expected_dim)
                    ));
                }
            }
        }

        // Add documents to collection
        let collection_docs = self.documents.get_mut(collection_name).unwrap();
        
        for (i, id) in ids.iter().enumerate() {
            let document = Document {
                id: id.clone(),
                embedding: embeddings[i].clone(),
                document: documents.as_ref().map(|docs| docs[i].clone()),
                metadata: metadatas.as_ref().map(|metas| metas[i].clone()),
            };
            collection_docs.insert(id.clone(), document);
        }

        Ok(format!("Added {} documents", ids.len()))
    }

    /// Query a collection for similar vectors
    pub fn query_collection(
        &self,
        collection_name: &str,
        query_embeddings: Vec<Vec<f32>>,
        n_results: Option<usize>,
        where_clause: Option<HashMap<String, serde_json::Value>>,
        include: Option<Vec<String>>,
    ) -> VexfsResult<QueryResult> {
        // Validate collection exists
        let collection = self.get_collection(collection_name)?;
        let collection_docs = self.documents.get(collection_name).unwrap();

        let n_results = n_results.unwrap_or(10);
        let include = include.unwrap_or_else(|| vec!["documents".to_string(), "metadatas".to_string(), "distances".to_string()]);

        let mut all_ids = Vec::new();
        let mut all_distances = Vec::new();
        let mut all_documents = Vec::new();
        let mut all_metadatas = Vec::new();

        // Process each query embedding
        for query_embedding in query_embeddings {
            let mut results: Vec<(String, f32, Option<String>, Option<HashMap<String, serde_json::Value>>)> = Vec::new();

            // Calculate distances to all documents
            for doc in collection_docs.values() {
                // Apply where clause filter if provided
                if let Some(ref where_filter) = where_clause {
                    if let Some(ref doc_metadata) = doc.metadata {
                        let mut matches = true;
                        for (key, value) in where_filter {
                            if doc_metadata.get(key) != Some(value) {
                                matches = false;
                                break;
                            }
                        }
                        if !matches {
                            continue;
                        }
                    } else {
                        continue; // No metadata, can't match filter
                    }
                }

                let distance = self.calculate_distance(&query_embedding, &doc.embedding, &collection.distance_function)?;
                results.push((
                    doc.id.clone(),
                    distance,
                    doc.document.clone(),
                    doc.metadata.clone(),
                ));
            }

            // Sort by distance (ascending for L2 and cosine, descending for inner product)
            match collection.distance_function {
                DistanceFunction::InnerProduct => {
                    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                }
                _ => {
                    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                }
            }

            // Take top n_results
            results.truncate(n_results);

            // Extract results
            let ids: Vec<String> = results.iter().map(|(id, _, _, _)| id.clone()).collect();
            let distances: Vec<f32> = results.iter().map(|(_, dist, _, _)| *dist).collect();
            let documents: Vec<Option<String>> = results.iter().map(|(_, _, doc, _)| doc.clone()).collect();
            let metadatas: Vec<Option<HashMap<String, serde_json::Value>>> = results.iter().map(|(_, _, _, meta)| meta.clone()).collect();

            all_ids.push(ids);
            all_distances.push(distances);
            all_documents.push(documents);
            all_metadatas.push(metadatas);
        }

        Ok(QueryResult {
            ids: all_ids,
            distances: all_distances,
            documents: if include.contains(&"documents".to_string()) { Some(all_documents) } else { None },
            metadatas: if include.contains(&"metadatas".to_string()) { Some(all_metadatas) } else { None },
        })
    }

    /// Calculate distance between two vectors based on the distance function
    fn calculate_distance(
        &self,
        a: &[f32],
        b: &[f32],
        distance_function: &DistanceFunction,
    ) -> VexfsResult<f32> {
        if a.len() != b.len() {
            return Err(VexfsError::InvalidOperation(
                "Vector dimensions must match".to_string()
            ));
        }

        let distance = match distance_function {
            DistanceFunction::Euclidean => {
                // L2 (Euclidean) distance
                let sum_sq: f32 = a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum();
                sum_sq.sqrt()
            }
            DistanceFunction::Cosine => {
                // Cosine distance = 1 - cosine similarity
                let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                let norm_a: f32 = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
                let norm_b: f32 = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
                
                if norm_a == 0.0 || norm_b == 0.0 {
                    1.0 // Maximum distance for zero vectors
                } else {
                    1.0 - (dot_product / (norm_a * norm_b))
                }
            }
            DistanceFunction::InnerProduct => {
                // Inner product (negative for similarity ranking)
                -a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>()
            }
        };

        Ok(distance)
    }

    /// Get collection statistics
    pub fn get_collection_stats(&self, collection_name: &str) -> VexfsResult<HashMap<String, serde_json::Value>> {
        let _collection = self.get_collection(collection_name)?;
        let collection_docs = self.documents.get(collection_name).unwrap();

        let mut stats = HashMap::new();
        stats.insert("document_count".to_string(), serde_json::Value::Number(collection_docs.len().into()));
        
        if let Some(first_doc) = collection_docs.values().next() {
            stats.insert("dimension".to_string(), serde_json::Value::Number(first_doc.embedding.len().into()));
        }

        Ok(stats)
    }

    /// Update documents in a collection
    pub fn update_documents(
        &mut self,
        collection_name: &str,
        ids: Vec<String>,
        embeddings: Option<Vec<Vec<f32>>>,
        documents: Option<Vec<String>>,
        metadatas: Option<Vec<HashMap<String, serde_json::Value>>>,
    ) -> VexfsResult<String> {
        // Validate collection exists
        let _collection = self.get_collection(collection_name)?;
        let collection_docs = self.documents.get_mut(collection_name).unwrap();

        for (i, id) in ids.iter().enumerate() {
            if let Some(mut doc) = collection_docs.get(id).cloned() {
                // Update embedding if provided
                if let Some(ref embs) = embeddings {
                    if i < embs.len() {
                        doc.embedding = embs[i].clone();
                    }
                }

                // Update document if provided
                if let Some(ref docs) = documents {
                    if i < docs.len() {
                        doc.document = Some(docs[i].clone());
                    }
                }

                // Update metadata if provided
                if let Some(ref metas) = metadatas {
                    if i < metas.len() {
                        doc.metadata = Some(metas[i].clone());
                    }
                }

                collection_docs.insert(id.clone(), doc);
            } else {
                return Err(VexfsError::EntryNotFound(
                    format!("Document '{}' not found in collection '{}'", id, collection_name)
                ));
            }
        }

        Ok(format!("Updated {} documents", ids.len()))
    }

    /// Delete documents from a collection
    pub fn delete_documents(
        &mut self,
        collection_name: &str,
        ids: Vec<String>,
    ) -> VexfsResult<String> {
        // Validate collection exists
        let _collection = self.get_collection(collection_name)?;
        let collection_docs = self.documents.get_mut(collection_name).unwrap();

        let mut deleted_count = 0;
        for id in ids {
            if collection_docs.remove(&id).is_some() {
                deleted_count += 1;
            }
        }

        Ok(format!("Deleted {} documents", deleted_count))
    }
}

impl Default for ChromaDBApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_list_collections() {
        let mut api = ChromaDBApi::new();
        
        let collection = api.create_collection(
            "test_collection".to_string(),
            None,
            Some(DistanceFunction::Cosine),
        ).unwrap();

        assert_eq!(collection.name, "test_collection");
        assert!(matches!(collection.distance_function, DistanceFunction::Cosine));

        let collections = api.list_collections();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].name, "test_collection");
    }

    #[test]
    fn test_add_and_query_documents() {
        let mut api = ChromaDBApi::new();
        
        api.create_collection(
            "test_collection".to_string(),
            None,
            Some(DistanceFunction::Euclidean),
        ).unwrap();

        // Add documents
        let result = api.add_documents(
            "test_collection",
            vec!["doc1".to_string(), "doc2".to_string()],
            vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]],
            Some(vec!["First document".to_string(), "Second document".to_string()]),
            None,
        ).unwrap();

        assert!(result.contains("Added 2 documents"));

        // Query documents
        let query_result = api.query_collection(
            "test_collection",
            vec![vec![1.0, 0.0, 0.0]], // Should be closest to doc1
            Some(1),
            None,
            None,
        ).unwrap();

        assert_eq!(query_result.ids.len(), 1);
        assert_eq!(query_result.ids[0].len(), 1);
        assert_eq!(query_result.ids[0][0], "doc1");
    }

    #[test]
    fn test_distance_calculations() {
        let api = ChromaDBApi::new();
        
        // Test Euclidean distance
        let dist = api.calculate_distance(
            &[1.0, 0.0],
            &[0.0, 1.0],
            &DistanceFunction::Euclidean,
        ).unwrap();
        assert!((dist - 1.414).abs() < 0.01); // sqrt(2)

        // Test cosine distance
        let dist = api.calculate_distance(
            &[1.0, 0.0],
            &[1.0, 0.0],
            &DistanceFunction::Cosine,
        ).unwrap();
        assert!(dist.abs() < 0.01); // Should be 0 for identical vectors

        // Test inner product
        let dist = api.calculate_distance(
            &[1.0, 2.0],
            &[3.0, 4.0],
            &DistanceFunction::InnerProduct,
        ).unwrap();
        assert!((dist + 11.0).abs() < 0.01); // -(1*3 + 2*4) = -11
    }
}