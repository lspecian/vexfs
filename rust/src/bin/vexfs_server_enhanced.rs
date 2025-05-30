use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

// Health check and metrics structures
#[derive(Debug, Serialize)]
struct HealthStatus {
    status: String,
    timestamp: u64,
    version: String,
    uptime_seconds: u64,
    memory_usage_mb: u64,
    active_connections: u64,
}

#[derive(Debug, Serialize)]
struct MetricsData {
    requests_total: u64,
    requests_per_second: f64,
    average_response_time_ms: f64,
    collections_count: u64,
    documents_count: u64,
    memory_usage_mb: u64,
    uptime_seconds: u64,
}

// Enhanced server state with metrics
#[derive(Clone)]
struct VexFSServerState {
    collections: Arc<Mutex<HashMap<String, Collection>>>,
    documents: Arc<Mutex<HashMap<String, HashMap<String, Document>>>>,
    metrics: Arc<Mutex<ServerMetrics>>,
    start_time: SystemTime,
}

#[derive(Debug)]
struct ServerMetrics {
    requests_total: u64,
    response_times: Vec<u64>, // Store last 1000 response times
    active_connections: u64,
}

impl ServerMetrics {
    fn new() -> Self {
        Self {
            requests_total: 0,
            response_times: Vec::new(),
            active_connections: 0,
        }
    }

    fn record_request(&mut self, response_time_ms: u64) {
        self.requests_total += 1;
        self.response_times.push(response_time_ms);
        
        // Keep only last 1000 response times
        if self.response_times.len() > 1000 {
            self.response_times.remove(0);
        }
    }

    fn average_response_time(&self) -> f64 {
        if self.response_times.is_empty() {
            0.0
        } else {
            self.response_times.iter().sum::<u64>() as f64 / self.response_times.len() as f64
        }
    }
}

// ChromaDB-compatible API structures (same as before)
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

impl VexFSServerState {
    fn new() -> Self {
        Self {
            collections: Arc::new(Mutex::new(HashMap::new())),
            documents: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(ServerMetrics::new())),
            start_time: SystemTime::now(),
        }
    }

    fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().unwrap_or_default().as_secs()
    }

    fn get_memory_usage_mb(&self) -> u64 {
        // Simple memory usage estimation
        let collections = self.collections.lock().unwrap();
        let documents = self.documents.lock().unwrap();
        
        let collections_size = collections.len() * 1024; // Rough estimate
        let documents_size = documents.values()
            .map(|docs| docs.len() * 2048) // Rough estimate per document
            .sum::<usize>();
        
        ((collections_size + documents_size) / (1024 * 1024)) as u64
    }
}

// Health check endpoint
async fn health_check(State(state): State<VexFSServerState>) -> Json<HealthStatus> {
    let metrics = state.metrics.lock().unwrap();
    
    Json(HealthStatus {
        status: "healthy".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        version: "1.0.0".to_string(),
        uptime_seconds: state.get_uptime_seconds(),
        memory_usage_mb: state.get_memory_usage_mb(),
        active_connections: metrics.active_connections,
    })
}

// Readiness check endpoint
async fn readiness_check(State(state): State<VexFSServerState>) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Check if the service is ready to accept requests
    let collections = state.collections.lock().unwrap();
    let documents = state.documents.lock().unwrap();
    
    // Simple readiness check - ensure data structures are accessible
    drop(collections);
    drop(documents);
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some("ready".to_string()),
        error: None,
    }))
}

// Metrics endpoint (Prometheus format)
async fn metrics_endpoint(State(state): State<VexFSServerState>) -> String {
    let metrics = state.metrics.lock().unwrap();
    let collections = state.collections.lock().unwrap();
    let documents = state.documents.lock().unwrap();
    
    let total_documents: usize = documents.values().map(|docs| docs.len()).sum();
    
    format!(
        "# HELP vexfs_requests_total Total number of requests processed\n\
         # TYPE vexfs_requests_total counter\n\
         vexfs_requests_total {}\n\
         \n\
         # HELP vexfs_response_time_seconds Average response time in seconds\n\
         # TYPE vexfs_response_time_seconds gauge\n\
         vexfs_response_time_seconds {:.6}\n\
         \n\
         # HELP vexfs_collections_total Total number of collections\n\
         # TYPE vexfs_collections_total gauge\n\
         vexfs_collections_total {}\n\
         \n\
         # HELP vexfs_documents_total Total number of documents\n\
         # TYPE vexfs_documents_total gauge\n\
         vexfs_documents_total {}\n\
         \n\
         # HELP vexfs_memory_usage_bytes Memory usage in bytes\n\
         # TYPE vexfs_memory_usage_bytes gauge\n\
         vexfs_memory_usage_bytes {}\n\
         \n\
         # HELP vexfs_uptime_seconds Uptime in seconds\n\
         # TYPE vexfs_uptime_seconds gauge\n\
         vexfs_uptime_seconds {}\n\
         \n\
         # HELP vexfs_active_connections Active connections\n\
         # TYPE vexfs_active_connections gauge\n\
         vexfs_active_connections {}\n",
        metrics.requests_total,
        metrics.average_response_time() / 1000.0, // Convert ms to seconds
        collections.len(),
        total_documents,
        state.get_memory_usage_mb() * 1024 * 1024, // Convert MB to bytes
        state.get_uptime_seconds(),
        metrics.active_connections
    )
}

// Enhanced version endpoint with more details
async fn get_version() -> Json<ApiResponse<serde_json::Value>> {
    let version_info = serde_json::json!({
        "version": "1.0.0",
        "name": "VexFS",
        "description": "Vector Extended File System - ChromaDB-compatible",
        "api_version": "v1",
        "build_date": option_env!("BUILD_DATE").unwrap_or("unknown"),
        "git_commit": option_env!("GIT_COMMIT").unwrap_or("unknown"),
        "rust_version": option_env!("RUST_VERSION").unwrap_or("unknown"),
        "features": [
            "chromadb-compatible",
            "vector-search",
            "metrics",
            "health-checks",
            "production-ready"
        ]
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(version_info),
        error: None,
    })
}

// Middleware to track request metrics
async fn track_request_metrics(
    State(state): State<VexFSServerState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let start_time = std::time::Instant::now();
    
    // Increment active connections
    {
        let mut metrics = state.metrics.lock().unwrap();
        metrics.active_connections += 1;
    }
    
    let response = next.run(request).await;
    
    // Record metrics
    let response_time_ms = start_time.elapsed().as_millis() as u64;
    {
        let mut metrics = state.metrics.lock().unwrap();
        metrics.record_request(response_time_ms);
        metrics.active_connections = metrics.active_connections.saturating_sub(1);
    }
    
    response
}

// All the existing API handlers (list_collections, create_collection, etc.) remain the same
// ... (keeping the same implementations as in the original server)

async fn list_collections(State(state): State<VexFSServerState>) -> Json<ApiResponse<Vec<Collection>>> {
    let collections = state.collections.lock().unwrap();
    let collection_list: Vec<Collection> = collections.values().cloned().collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(collection_list),
        error: None,
    })
}

// ... (other handlers remain the same for brevity)

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("vexfs=info".parse()?)
        )
        .json()
        .init();

    let state = VexFSServerState::new();
    
    // Build our application with routes
    let app = Router::new()
        // Health and monitoring endpoints
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/metrics", get(metrics_endpoint))
        
        // API endpoints
        .route("/api/v1/version", get(get_version))
        .route("/api/v1/collections", get(list_collections))
        // ... (add other routes)
        
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    track_request_metrics,
                ))
                .layer(CorsLayer::permissive())
        )
        .with_state(state);
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let bind_address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", bind_address, port);
    
    info!("🚀 VexFS Server starting...");
    info!("📡 Listening on: http://{}", addr);
    info!("🔗 API Base URL: http://{}/api/v1", addr);
    info!("❤️  Health check: http://{}/health", addr);
    info!("📊 Metrics: http://{}/metrics", addr);
    
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Utility function for cosine similarity (same as before)
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