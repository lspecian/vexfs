//! Multi-dialect router for VexFS unified server
//! 
//! This module provides a unified HTTP server that routes requests to appropriate
//! API dialects based on URL patterns.

use super::{ApiDialect, VexFSEngine};
use super::chromadb::ChromaDBDialect;
use super::qdrant::QdrantDialect;
use super::native::NativeDialect;
use crate::shared::errors::*;
use crate::auth::{AuthConfig, AuthUser, OptionalAuth, Claims, UserRole, check_collection_access, generate_api_key};

use axum::{
    extract::{Path, State},
    http::{Method, StatusCode, Uri},
    response::{Json, Redirect},
    routing::{delete, get, post, put},
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
    pub auth_config: AuthConfig,
}

impl RouterState {
    pub fn new() -> Self {
        let engine = VexFSEngine::new();
        let auth_config = AuthConfig::default();
        Self {
            chromadb: Arc::new(ChromaDBDialect::new(engine.clone())),
            qdrant: Arc::new(QdrantDialect::new(engine.clone())),
            native: Arc::new(NativeDialect::new(engine.clone())),
            engine,
            auth_config,
        }
    }
}

/// Create the unified router with all dialects
pub fn create_router() -> Router {
    let state = RouterState::new();
    
    // Get dashboard path from environment variable
    let dashboard_path = std::env::var("DASHBOARD_PATH").unwrap_or_else(|_| "/app/dashboard".to_string());
    
    Router::new()
        // Authentication endpoints
        .route("/auth/login", post(auth_login))
        .route("/auth/api-key", post(create_api_key))
        .route("/auth/verify", get(verify_token))
        
        // ChromaDB API routes (/api/v1/*) - Protected
        .route("/api/v1/version", get(api_version))
        .route("/api/v1/collections", get(chromadb_list_collections).post(chromadb_create_collection))
        .route("/api/v1/collections/:collection", delete(chromadb_delete_collection))
        .route("/api/v1/collections/:collection/add", post(chromadb_add_documents))
        .route("/api/v1/collections/:collection/query", post(chromadb_query))
        .route("/api/v1/collections/:collection/vectors", get(chromadb_get_vectors).post(chromadb_update_vectors))
        
        // Qdrant API routes (/collections/*) - Protected
        .route("/collections", get(qdrant_list_collections))
        .route("/collections/:collection", put(qdrant_create_collection))
        .route("/collections/:collection/points", put(qdrant_upsert_points))
        .route("/collections/:collection/points/search", post(qdrant_search))
        
        // Native VexFS API routes (/vexfs/v1/*) - Protected
        .route("/vexfs/v1/collections", get(native_list_collections).post(native_create_collection))
        .route("/vexfs/v1/collections/:collection/documents", post(native_add_documents))
        .route("/vexfs/v1/collections/:collection/search", post(native_search))
        .route("/vexfs/v1/health", get(native_health))
        
        // Health and metrics endpoints (public)
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

/// Authentication endpoints

/// Login endpoint - exchanges credentials for JWT token
async fn auth_login(
    State(state): State<RouterState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // For now, authenticate with API key
    let claims = state.auth_config.authenticate_api_key(&payload.api_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let token = state.auth_config.generate_token(&claims)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(LoginResponse {
        token,
        expires_at: claims.exp,
        role: claims.role,
    }))
}

/// Create new API key (admin only)
async fn create_api_key(
    State(state): State<RouterState>,
    auth: AuthUser,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, StatusCode> {
    // Only admins can create API keys
    if !auth.claims.role.can_admin() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let api_key = generate_api_key();
    
    Ok(Json(CreateApiKeyResponse {
        api_key,
        role: payload.role,
        collections: payload.collections,
    }))
}

/// Verify token validity
async fn verify_token(
    auth: AuthUser,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "valid": true,
        "sub": auth.claims.sub,
        "role": auth.claims.role,
        "exp": auth.claims.exp,
    }))
}

/// ChromaDB API handlers with authentication

async fn chromadb_list_collections(
    State(state): State<RouterState>,
    auth: OptionalAuth,
) -> Result<axum::response::Response, StatusCode> {
    // Check if user has read permission
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    
    match state.chromadb.handle_request("/api/v1/collections", "GET", b"") {
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

async fn chromadb_create_collection(
    State(state): State<RouterState>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    // Check if user has write permission
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    match state.chromadb.handle_request("/api/v1/collections", "POST", body.as_bytes()) {
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

async fn chromadb_delete_collection(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: AuthUser,
) -> Result<axum::response::Response, StatusCode> {
    // Check if user has write permission and collection access
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    check_collection_access(&auth, &collection)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    
    let path = format!("/api/v1/collections/{}", collection);
    match state.chromadb.handle_request(&path, "DELETE", b"") {
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

async fn chromadb_add_documents(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    // Check if user has write permission and collection access
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    check_collection_access(&auth, &collection)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    
    let path = format!("/api/v1/collections/{}/add", collection);
    match state.chromadb.handle_request(&path, "POST", body.as_bytes()) {
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

async fn chromadb_query(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: OptionalAuth,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    // Check if user has read permission and collection access
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
        check_collection_access(&user, &collection)
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }
    
    let path = format!("/api/v1/collections/{}/query", collection);
    match state.chromadb.handle_request(&path, "POST", body.as_bytes()) {
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

async fn chromadb_get_vectors(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: OptionalAuth,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
        check_collection_access(&user, &collection)
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }
    
    let path = format!("/api/v1/collections/{}/vectors", collection);
    match state.chromadb.handle_request(&path, "GET", b"") {
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

async fn chromadb_update_vectors(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    check_collection_access(&auth, &collection)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    
    let path = format!("/api/v1/collections/{}/vectors", collection);
    match state.chromadb.handle_request(&path, "POST", body.as_bytes()) {
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

/// Qdrant API handlers with authentication

async fn qdrant_list_collections(
    State(state): State<RouterState>,
    auth: OptionalAuth,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    
    match state.qdrant.handle_request("/collections", "GET", b"") {
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

async fn qdrant_create_collection(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let path = format!("/collections/{}", collection);
    match state.qdrant.handle_request(&path, "PUT", body.as_bytes()) {
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

async fn qdrant_upsert_points(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    check_collection_access(&auth, &collection)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    
    let path = format!("/collections/{}/points", collection);
    match state.qdrant.handle_request(&path, "PUT", body.as_bytes()) {
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

async fn qdrant_search(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: OptionalAuth,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
        check_collection_access(&user, &collection)
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }
    
    let path = format!("/collections/{}/points/search", collection);
    match state.qdrant.handle_request(&path, "POST", body.as_bytes()) {
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

/// Native VexFS API handlers with authentication

async fn native_list_collections(
    State(state): State<RouterState>,
    auth: OptionalAuth,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    
    match state.native.handle_request("/vexfs/v1/collections", "GET", b"") {
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

async fn native_create_collection(
    State(state): State<RouterState>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    match state.native.handle_request("/vexfs/v1/collections", "POST", body.as_bytes()) {
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

async fn native_add_documents(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: AuthUser,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if !auth.claims.role.can_write() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    check_collection_access(&auth, &collection)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    
    let path = format!("/vexfs/v1/collections/{}/documents", collection);
    match state.native.handle_request(&path, "POST", body.as_bytes()) {
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

async fn native_search(
    State(state): State<RouterState>,
    Path(collection): Path<String>,
    auth: OptionalAuth,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(user) = auth.0 {
        if !user.claims.role.can_read() {
            return Err(StatusCode::FORBIDDEN);
        }
        check_collection_access(&user, &collection)
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }
    
    let path = format!("/vexfs/v1/collections/{}/search", collection);
    match state.native.handle_request(&path, "POST", body.as_bytes()) {
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

async fn native_health(
    State(state): State<RouterState>,
) -> Result<axum::response::Response, StatusCode> {
    match state.native.handle_request("/vexfs/v1/health", "GET", b"") {
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

// Request/Response types for authentication
#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    token: String,
    expires_at: i64,
    role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateApiKeyRequest {
    role: UserRole,
    collections: Option<Vec<String>>,
    rate_limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateApiKeyResponse {
    api_key: String,
    role: UserRole,
    collections: Option<Vec<String>>,
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