//! ChromaDB-compatible API dialect for VexFS
//! 
//! This module provides ChromaDB API compatibility, allowing existing ChromaDB
//! clients to work with VexFS without modification.

use super::{ApiDialect, VexFSEngine, Collection, Document, CollectionMetadata, DistanceFunction};
use crate::shared::errors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ChromaDB API dialect implementation
pub struct ChromaDBDialect {
    engine: VexFSEngine,
}

impl ChromaDBDialect {
    pub fn new(engine: VexFSEngine) -> Self {
        Self { engine }
    }
}

impl ApiDialect for ChromaDBDialect {
    fn handle_request(&self, path: &str, method: &str, body: &[u8]) -> VexfsResult<Vec<u8>> {
        match (method, path) {
            ("GET", "/api/v1/collections") => {
                let collections = self.engine.list_collections()?;
                let response = ChromaCollectionsResponse { collections };
                serde_json::to_vec(&response).map_err(|_| VexfsError::InvalidData("Serialization failed".to_string()))
            }
            ("POST", "/api/v1/collections") => {
                let request: ChromaCreateCollectionRequest = 
                    serde_json::from_slice(body).map_err(|_| VexfsError::InvalidData("Deserialization failed".to_string()))?;
                
                let metadata = CollectionMetadata {
                    dimension: None,
                    distance_function: match request.metadata.distance.as_deref() {
                        Some("cosine") => DistanceFunction::Cosine,
                        Some("euclidean") => DistanceFunction::Euclidean,
                        Some("ip") => DistanceFunction::DotProduct,
                        _ => DistanceFunction::Cosine,
                    },
                    description: None,
                };
                
                self.engine.create_collection(request.name.clone(), Some(metadata))?;
                
                let response = ChromaCollectionResponse {
                    name: request.name,
                    id: uuid::Uuid::new_v4().to_string(),
                    metadata: request.metadata,
                };
                serde_json::to_vec(&response).map_err(|_| VexfsError::InvalidData("Serialization failed".to_string()))
            }
            ("POST", path) if path.starts_with("/api/v1/collections/") && path.ends_with("/add") => {
                let collection_name = path
                    .strip_prefix("/api/v1/collections/")
                    .and_then(|s| s.strip_suffix("/add"))
                    .ok_or(VexfsError::InvalidArgument("Invalid input".to_string()))?;
                
                let request: ChromaAddRequest = 
                    serde_json::from_slice(body).map_err(|_| VexfsError::InvalidData("Deserialization failed".to_string()))?;
                
                let documents: Vec<Document> = request.ids.into_iter()
                    .enumerate()
                    .map(|(i, id)| Document {
                        id,
                        embedding: request.embeddings.as_ref().and_then(|emb| emb.get(i).cloned()),
                        metadata: request.metadatas.as_ref().and_then(|meta| meta.get(i).and_then(|m| m.clone())),
                        content: request.documents.as_ref().and_then(|docs| docs.get(i).cloned()),
                    })
                    .collect();
                
                self.engine.add_documents(collection_name, documents)?;
                
                let response = ChromaAddResponse { success: true };
                serde_json::to_vec(&response).map_err(|_| VexfsError::InvalidData("Serialization failed".to_string()))
            }
            ("POST", path) if path.starts_with("/api/v1/collections/") && path.ends_with("/query") => {
                let collection_name = path
                    .strip_prefix("/api/v1/collections/")
                    .and_then(|s| s.strip_suffix("/query"))
                    .ok_or(VexfsError::InvalidArgument("Invalid input".to_string()))?;
                
                let request: ChromaQueryRequest = 
                    serde_json::from_slice(body).map_err(|_| VexfsError::InvalidData("Deserialization failed".to_string()))?;
                
                if let Some(query_embeddings) = request.query_embeddings {
                    if let Some(query_embedding) = query_embeddings.first() {
                        let n_results = request.n_results.unwrap_or(10);
                        let results = self.engine.query_collection(collection_name, query_embedding.clone(), n_results)?;
                        
                        let response = ChromaQueryResponse {
                            ids: vec![results.iter().map(|r| r.id.clone()).collect()],
                            distances: vec![results.iter().map(|r| r.distance).collect()],
                            metadatas: vec![results.iter().map(|r| r.metadata.clone()).collect()],
                            documents: vec![vec![None; results.len()]], // ChromaDB format
                        };
                        
                        return serde_json::to_vec(&response).map_err(|_| VexfsError::InvalidData("Serialization failed".to_string()));
                    }
                }
                
                Err(VexfsError::InvalidArgument("Invalid input".to_string()))
            }
            ("DELETE", path) if path.starts_with("/api/v1/collections/") => {
                let collection_name = path.strip_prefix("/api/v1/collections/")
                    .ok_or(VexfsError::InvalidArgument("Invalid collection path".to_string()))?;
                
                self.engine.delete_collection(collection_name)?;
                
                // ChromaDB returns empty response on successful deletion
                Ok(Vec::new())
            }
            _ => Err(VexfsError::NotFound),
        }
    }
    
    fn url_prefix(&self) -> &str {
        "/api/v1"
    }
    
    fn name(&self) -> &str {
        "ChromaDB"
    }
}

// ChromaDB API request/response types
#[derive(Debug, Serialize, Deserialize)]
struct ChromaCollectionsResponse {
    collections: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaCreateCollectionRequest {
    name: String,
    metadata: ChromaCollectionMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaCollectionMetadata {
    distance: Option<String>,
    description: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaCollectionResponse {
    name: String,
    id: String,
    metadata: ChromaCollectionMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaAddRequest {
    ids: Vec<String>,
    embeddings: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<Option<HashMap<String, serde_json::Value>>>>,
    documents: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaAddResponse {
    success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaQueryRequest {
    query_embeddings: Option<Vec<Vec<f32>>>,
    n_results: Option<usize>,
    #[serde(rename = "where")]
    where_clause: Option<HashMap<String, serde_json::Value>>,
    include: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    distances: Vec<Vec<f32>>,
    metadatas: Vec<Vec<Option<HashMap<String, serde_json::Value>>>>,
    documents: Vec<Vec<Option<String>>>,
}