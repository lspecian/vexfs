use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

// Note: VexfsResult and VexfsError not currently used in this simple server implementation

// ChromaDB-compatible API structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Collection {
    id: String,
    name: String,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    id: String,
    embedding: Option<Vec<f32>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    document: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddRequest {
    ids: Vec<String>,
    embeddings: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<HashMap<String, serde_json::Value>>>,
    documents: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    query_embeddings: Vec<Vec<f32>>,
    n_results: Option<usize>,
    #[serde(rename = "where")]
    where_filter: Option<HashMap<String, serde_json::Value>>,
    include: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct QueryResponse {
    ids: Vec<Vec<String>>,
    distances: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<Vec<HashMap<String, serde_json::Value>>>>,
    documents: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

// VexFS Server State
#[derive(Clone)]
struct VexFSServerState {
    collections: Arc<Mutex<HashMap<String, Collection>>>,
    documents: Arc<Mutex<HashMap<String, HashMap<String, Document>>>>, // collection_id -> doc_id -> document
}

impl VexFSServerState {
    fn new() -> Self {
        Self {
            collections: Arc::new(Mutex::new(HashMap::new())),
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// ChromaDB-compatible API handlers
async fn get_version() -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        success: true,
        data: Some("VexFS 1.0.0 (ChromaDB-compatible)".to_string()),
        error: None,
    })
}

async fn list_collections(State(state): State<VexFSServerState>) -> Json<ApiResponse<Vec<Collection>>> {
    let collections = state.collections.lock().unwrap();
    let collection_list: Vec<Collection> = collections.values().cloned().collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(collection_list),
        error: None,
    })
}

async fn create_collection(
    State(state): State<VexFSServerState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<Collection>>, StatusCode> {
    let name = payload.get("name")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let metadata = payload.get("metadata")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    
    let collection = Collection {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        metadata,
    };
    
    // Store collection
    {
        let mut collections = state.collections.lock().unwrap();
        collections.insert(collection.id.clone(), collection.clone());
    }
    
    // Initialize document storage for this collection
    {
        let mut documents = state.documents.lock().unwrap();
        documents.insert(collection.id.clone(), HashMap::new());
    }
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(collection),
        error: None,
    }))
}

async fn get_collection(
    State(state): State<VexFSServerState>,
    Path(collection_name): Path<String>,
) -> Result<Json<ApiResponse<Collection>>, StatusCode> {
    let collections = state.collections.lock().unwrap();
    
    let collection = collections.values()
        .find(|c| c.name == collection_name)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(collection),
        error: None,
    }))
}

async fn add_documents(
    State(state): State<VexFSServerState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<AddRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Find collection
    let collection_id = {
        let collections = state.collections.lock().unwrap();
        collections.values()
            .find(|c| c.name == collection_name)
            .map(|c| c.id.clone())
            .ok_or(StatusCode::NOT_FOUND)?
    };
    
    // Validate request
    let num_docs = payload.ids.len();
    if let Some(ref embeddings) = payload.embeddings {
        if embeddings.len() != num_docs {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(ref metadatas) = payload.metadatas {
        if metadatas.len() != num_docs {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(ref documents) = payload.documents {
        if documents.len() != num_docs {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    
    // Add documents
    {
        let mut documents = state.documents.lock().unwrap();
        let collection_docs = documents.get_mut(&collection_id).unwrap();
        
        for (i, id) in payload.ids.iter().enumerate() {
            let document = Document {
                id: id.clone(),
                embedding: payload.embeddings.as_ref().map(|e| e[i].clone()),
                metadata: payload.metadatas.as_ref().map(|m| m[i].clone()),
                document: payload.documents.as_ref().map(|d| d[i].clone()),
            };
            
            collection_docs.insert(id.clone(), document);
        }
    }
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(format!("Added {} documents", num_docs)),
        error: None,
    }))
}

async fn query_collection(
    State(state): State<VexFSServerState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<QueryRequest>,
) -> Result<Json<ApiResponse<QueryResponse>>, StatusCode> {
    // Find collection
    let collection_id = {
        let collections = state.collections.lock().unwrap();
        collections.values()
            .find(|c| c.name == collection_name)
            .map(|c| c.id.clone())
            .ok_or(StatusCode::NOT_FOUND)?
    };
    
    let n_results = payload.n_results.unwrap_or(10);
    let include = payload.include.unwrap_or_else(|| vec!["documents".to_string(), "metadatas".to_string(), "distances".to_string()]);
    
    // Simple similarity search (cosine similarity)
    let documents = state.documents.lock().unwrap();
    let collection_docs = documents.get(&collection_id).unwrap();
    
    let mut results = QueryResponse {
        ids: Vec::new(),
        distances: if include.contains(&"distances".to_string()) { Some(Vec::new()) } else { None },
        metadatas: if include.contains(&"metadatas".to_string()) { Some(Vec::new()) } else { None },
        documents: if include.contains(&"documents".to_string()) { Some(Vec::new()) } else { None },
    };
    
    for query_embedding in payload.query_embeddings {
        let mut similarities: Vec<(String, f32, &Document)> = Vec::new();
        
        for (doc_id, document) in collection_docs.iter() {
            if let Some(ref doc_embedding) = document.embedding {
                let similarity = cosine_similarity(&query_embedding, doc_embedding);
                similarities.push((doc_id.clone(), similarity, document));
            }
        }
        
        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        similarities.truncate(n_results);
        
        // Collect results
        let query_ids: Vec<String> = similarities.iter().map(|(id, _, _)| id.clone()).collect();
        results.ids.push(query_ids);
        
        if let Some(ref mut distances) = results.distances {
            let query_distances: Vec<f32> = similarities.iter().map(|(_, dist, _)| 1.0 - dist).collect(); // Convert similarity to distance
            distances.push(query_distances);
        }
        
        if let Some(ref mut metadatas) = results.metadatas {
            let query_metadatas: Vec<HashMap<String, serde_json::Value>> = similarities.iter()
                .map(|(_, _, doc)| doc.metadata.clone().unwrap_or_default())
                .collect();
            metadatas.push(query_metadatas);
        }
        
        if let Some(ref mut documents) = results.documents {
            let query_documents: Vec<String> = similarities.iter()
                .map(|(_, _, doc)| doc.document.clone().unwrap_or_default())
                .collect();
            documents.push(query_documents);
        }
    }
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(results),
        error: None,
    }))
}

async fn delete_collection(
    State(state): State<VexFSServerState>,
    Path(collection_name): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let collection_id = {
        let mut collections = state.collections.lock().unwrap();
        let collection_id = collections.values()
            .find(|c| c.name == collection_name)
            .map(|c| c.id.clone())
            .ok_or(StatusCode::NOT_FOUND)?;
        
        collections.retain(|_, c| c.name != collection_name);
        collection_id
    };
    
    // Remove documents
    {
        let mut documents = state.documents.lock().unwrap();
        documents.remove(&collection_id);
    }
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(format!("Deleted collection: {}", collection_name)),
        error: None,
    }))
}

// Utility function for cosine similarity
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let state = VexFSServerState::new();
    
    // Build our application with routes
    let app = Router::new()
        .route("/api/v1/version", get(get_version))
        .route("/api/v1/collections", get(list_collections))
        .route("/api/v1/collections", post(create_collection))
        .route("/api/v1/collections/:name", get(get_collection))
        .route("/api/v1/collections/:name", delete(delete_collection))
        .route("/api/v1/collections/:name/add", post(add_documents))
        .route("/api/v1/collections/:name/query", post(query_collection))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        )
        .with_state(state);
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    println!("🚀 VexFS Server starting...");
    println!("📡 Listening on: http://{}", addr);
    println!("🔗 API Base URL: http://{}/api/v1", addr);
    println!("📚 ChromaDB-compatible API endpoints:");
    println!("   GET    /api/v1/version");
    println!("   GET    /api/v1/collections");
    println!("   POST   /api/v1/collections");
    println!("   GET    /api/v1/collections/:name");
    println!("   DELETE /api/v1/collections/:name");
    println!("   POST   /api/v1/collections/:name/add");
    println!("   POST   /api/v1/collections/:name/query");
    println!("");
    println!("💡 Example usage:");
    println!("   curl http://{}/api/v1/version", addr);
    println!("");
    
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}