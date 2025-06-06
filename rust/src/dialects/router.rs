//! Multi-dialect router for VexFS unified server
//! 
//! This module provides a unified HTTP server that routes requests to appropriate
//! API dialects based on URL patterns.

use super::{ApiDialect, VexFSEngine};
use super::chromadb::ChromaDBDialect;
use super::qdrant::QdrantDialect;
use super::native::NativeDialect;
use crate::shared::errors::*;

use axum::{
    extract::{Path, State},
    http::{Method, StatusCode, Uri},
    response::{Json, Redirect},
    routing::{get, post, put},
    Router,
};
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

/// Multi-dialect router state
#[derive(Clone)]
pub struct RouterState {
    pub engine: VexFSEngine,
    pub chromadb: Arc<ChromaDBDialect>,
    pub qdrant: Arc<QdrantDialect>,
    pub native: Arc<NativeDialect>,
}

impl RouterState {
    pub fn new() -> Self {
        let engine = VexFSEngine::new();
        Self {
            chromadb: Arc::new(ChromaDBDialect::new(engine.clone())),
            qdrant: Arc::new(QdrantDialect::new(engine.clone())),
            native: Arc::new(NativeDialect::new(engine.clone())),
            engine,
        }
    }
}

/// Create the unified router with all dialects
pub fn create_router() -> Router {
    let state = RouterState::new();
    
    // Get dashboard path from environment variable
    let dashboard_path = std::env::var("DASHBOARD_PATH").unwrap_or_else(|_| "/app/dashboard".to_string());
    
    Router::new()
        // ChromaDB API routes (/api/v1/*)
        .route("/api/v1/version", get(api_version))
        .route("/api/v1/collections", get(chromadb_handler).post(chromadb_handler))
        .route("/api/v1/collections/:collection/add", post(chromadb_handler))
        .route("/api/v1/collections/:collection/query", post(chromadb_handler))
        .route("/api/v1/collections/:collection/vectors", get(chromadb_handler).post(chromadb_handler))
        
        // Qdrant API routes (/collections/*)
        .route("/collections", get(qdrant_handler))
        .route("/collections/:collection", put(qdrant_handler))
        .route("/collections/:collection/points", put(qdrant_handler))
        .route("/collections/:collection/points/search", post(qdrant_handler))
        
        // Native VexFS API routes (/vexfs/v1/*)
        .route("/vexfs/v1/collections", get(native_handler).post(native_handler))
        .route("/vexfs/v1/collections/:collection/documents", post(native_handler))
        .route("/vexfs/v1/collections/:collection/search", post(native_handler))
        .route("/vexfs/v1/health", get(native_handler))
        
        // Health and metrics endpoints
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        
        // Serve dashboard static files from /ui/* path
        .nest_service("/ui", ServeDir::new(dashboard_path.clone()).append_index_html_on_directories(true))
        
        // Redirect root to dashboard
        .route("/", get(redirect_to_dashboard))
        
        // Fallback for any unmatched routes (404 handler)
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not Found") })
        
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

/// API version endpoint for dashboard compatibility
async fn api_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": "1.0.0",
        "name": "VexFS Unified Server",
        "status": "healthy"
    }))
}

/// Server information endpoint
async fn server_info() -> Json<ServerInfo> {
    Json(ServerInfo {
        name: "VexFS Unified Server".to_string(),
        version: "1.0.0".to_string(),
        description: "Multi-dialect vector database server powered by VexFS".to_string(),
        supported_apis: vec![
            ApiInfo {
                name: "ChromaDB".to_string(),
                version: "0.4.x".to_string(),
                base_path: "/api/v1".to_string(),
                description: "ChromaDB-compatible API".to_string(),
            },
            ApiInfo {
                name: "Qdrant".to_string(),
                version: "1.x".to_string(),
                base_path: "/".to_string(),
                description: "Qdrant-compatible API".to_string(),
            },
            ApiInfo {
                name: "Native VexFS".to_string(),
                version: "1.0.0".to_string(),
                base_path: "/vexfs/v1".to_string(),
                description: "Native VexFS API with advanced features".to_string(),
            },
        ],
        performance: PerformanceInfo {
            target_ops_per_second: 361000,
            engine: "VexFS High-Performance Vector Engine".to_string(),
        },
    })
}

/// ChromaDB API handler
async fn chromadb_handler(
    State(state): State<RouterState>,
    method: Method,
    uri: axum::http::Uri,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    let path = uri.path();
    let method_str = method.as_str();
    
    match state.chromadb.handle_request(path, method_str, body.as_bytes()) {
        Ok(response_body) => {
            Ok(axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(response_body))
                .unwrap())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Qdrant API handler
async fn qdrant_handler(
    State(state): State<RouterState>,
    method: Method,
    uri: axum::http::Uri,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    let path = uri.path();
    let method_str = method.as_str();
    
    match state.qdrant.handle_request(path, method_str, body.as_bytes()) {
        Ok(response_body) => {
            Ok(axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(response_body))
                .unwrap())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Native VexFS API handler
async fn native_handler(
    State(state): State<RouterState>,
    method: Method,
    uri: axum::http::Uri,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    let path = uri.path();
    let method_str = method.as_str();
    
    match state.native.handle_request(path, method_str, body.as_bytes()) {
        Ok(response_body) => {
            Ok(axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(response_body))
                .unwrap())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Health check endpoint
async fn health_check(State(state): State<RouterState>) -> Json<HealthStatus> {
    let collections_count = state.engine.list_collections().unwrap_or_default().len();
    
    Json(HealthStatus {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        collections_count,
        dialects: vec![
            DialectStatus {
                name: "ChromaDB".to_string(),
                status: "active".to_string(),
                endpoint: "/api/v1".to_string(),
            },
            DialectStatus {
                name: "Qdrant".to_string(),
                status: "active".to_string(),
                endpoint: "/collections".to_string(),
            },
            DialectStatus {
                name: "Native VexFS".to_string(),
                status: "active".to_string(),
                endpoint: "/vexfs/v1".to_string(),
            },
        ],
    })
}

/// Metrics endpoint
async fn metrics(State(state): State<RouterState>) -> Json<MetricsInfo> {
    let collections_count = state.engine.list_collections().unwrap_or_default().len();
    
    Json(MetricsInfo {
        collections_count,
        total_documents: 0, // Would need to iterate through collections
        memory_usage_mb: 256, // Placeholder
        uptime_seconds: 3600, // Placeholder
        requests_per_second: 1000.0, // Placeholder
        average_response_time_ms: 1.0, // Placeholder
    })
}

/// Redirect root to dashboard
async fn redirect_to_dashboard() -> Redirect {
    Redirect::permanent("/ui/")
}

// Response types
#[derive(Debug, Serialize, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
    description: String,
    supported_apis: Vec<ApiInfo>,
    performance: PerformanceInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiInfo {
    name: String,
    version: String,
    base_path: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerformanceInfo {
    target_ops_per_second: u64,
    engine: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthStatus {
    status: String,
    timestamp: i64,
    collections_count: usize,
    dialects: Vec<DialectStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DialectStatus {
    name: String,
    status: String,
    endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetricsInfo {
    collections_count: usize,
    total_documents: u64,
    memory_usage_mb: u64,
    uptime_seconds: u64,
    requests_per_second: f64,
    average_response_time_ms: f64,
}