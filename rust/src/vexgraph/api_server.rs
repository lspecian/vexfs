/*
 * VexFS v2.0 - VexGraph Phase 2 API Server
 * 
 * This module implements the RESTful API server for VexGraph Phase 2,
 * providing comprehensive CRUD operations for nodes and edges, traversal
 * endpoints, and query capabilities.
 */

use crate::vexgraph::{
    NodeId, EdgeId, NodeType, EdgeType, PropertyType, TraversalAlgorithm, VexGraphConfig,
    core::{VexGraphCore, GraphNode, GraphEdge},
    traversal::{TraversalEngine, TraversalQuery, TraversalResult},
    property_graph::PropertyGraphManager,
    semantic_integration::SemanticIntegration,
    error_handling::{VexGraphError, VexGraphResult},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

/// API request for creating a node
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub inode_number: u64,
    pub node_type: NodeType,
    pub properties: Option<HashMap<String, PropertyType>>,
}

/// API request for creating an edge
#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub source_id: NodeId,
    pub target_id: NodeId,
    pub edge_type: EdgeType,
    pub weight: Option<f64>,
    pub properties: Option<HashMap<String, PropertyType>>,
}

/// API request for updating node properties
#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub properties: HashMap<String, PropertyType>,
}

/// API request for updating edge properties
#[derive(Debug, Deserialize)]
pub struct UpdateEdgeRequest {
    pub weight: Option<f64>,
    pub properties: Option<HashMap<String, PropertyType>>,
}

/// API response for node operations
#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: NodeId,
    pub inode_number: u64,
    pub node_type: NodeType,
    pub properties: HashMap<String, PropertyType>,
    pub outgoing_edges: Vec<EdgeId>,
    pub incoming_edges: Vec<EdgeId>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<GraphNode> for NodeResponse {
    fn from(node: GraphNode) -> Self {
        Self {
            id: node.id,
            inode_number: node.inode_number,
            node_type: node.node_type,
            properties: node.properties,
            outgoing_edges: node.outgoing_edges,
            incoming_edges: node.incoming_edges,
            created_at: node.created_at,
            updated_at: node.updated_at,
        }
    }
}

/// API response for edge operations
#[derive(Debug, Serialize)]
pub struct EdgeResponse {
    pub id: EdgeId,
    pub source_id: NodeId,
    pub target_id: NodeId,
    pub edge_type: EdgeType,
    pub weight: f64,
    pub properties: HashMap<String, PropertyType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<GraphEdge> for EdgeResponse {
    fn from(edge: GraphEdge) -> Self {
        Self {
            id: edge.id,
            source_id: edge.source_id,
            target_id: edge.target_id,
            edge_type: edge.edge_type,
            weight: edge.weight,
            properties: edge.properties,
            created_at: edge.created_at,
            updated_at: edge.updated_at,
        }
    }
}

/// API query parameters for traversal
#[derive(Debug, Deserialize)]
pub struct TraversalQueryParams {
    pub algorithm: TraversalAlgorithm,
    pub start_node: NodeId,
    pub end_node: Option<NodeId>,
    pub max_depth: Option<u32>,
    pub max_results: Option<usize>,
    pub node_filter: Option<NodeType>,
    pub edge_filter: Option<EdgeType>,
    pub weight_threshold: Option<f64>,
    pub timeout_ms: Option<u64>,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl From<VexGraphError> for ErrorResponse {
    fn from(error: VexGraphError) -> Self {
        Self {
            error: format!("{:?}", error),
            message: error.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub core: Arc<VexGraphCore>,
    pub traversal: Arc<TraversalEngine>,
    pub property_graph: Arc<PropertyGraphManager>,
    pub semantic: Arc<SemanticIntegration>,
    pub config: VexGraphConfig,
}

/// VexGraph API server
#[derive(Debug)]
pub struct VexGraphApiServer {
    app_state: AppState,
    config: VexGraphConfig,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl VexGraphApiServer {
    /// Create a new API server
    pub async fn new(
        core: Arc<VexGraphCore>,
        traversal: Arc<TraversalEngine>,
        property_graph: Arc<PropertyGraphManager>,
        semantic: Arc<SemanticIntegration>,
        config: &VexGraphConfig,
    ) -> VexGraphResult<Self> {
        let app_state = AppState {
            core,
            traversal,
            property_graph,
            semantic,
            config: config.clone(),
        };
        
        Ok(Self {
            app_state,
            config: config.clone(),
            server_handle: None,
        })
    }
    
    /// Start the API server
    pub async fn start(&mut self) -> VexGraphResult<()> {
        let app = self.create_router();
        
        let bind_address = format!("{}:{}", self.config.api_bind_address, self.config.api_port);
        let listener = TcpListener::bind(&bind_address)
            .await
            .map_err(|e| VexGraphError::NetworkError(format!("Failed to bind to {}: {}", bind_address, e)))?;
        
        tracing::info!("VexGraph API server starting on {}", bind_address);
        
        let app_state = self.app_state.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("API server error: {}", e);
            }
        });
        
        self.server_handle = Some(handle);
        tracing::info!("VexGraph API server started successfully");
        Ok(())
    }
    
    /// Stop the API server
    pub async fn stop(&mut self) -> VexGraphResult<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            tracing::info!("VexGraph API server stopped");
        }
        Ok(())
    }
    
    /// Create the router with all endpoints
    fn create_router(&self) -> Router {
        Router::new()
            // Node endpoints
            .route("/api/v1/nodes", post(create_node))
            .route("/api/v1/nodes/:id", get(get_node))
            .route("/api/v1/nodes/:id", put(update_node))
            .route("/api/v1/nodes/:id", delete(delete_node))
            .route("/api/v1/nodes", get(list_nodes))
            .route("/api/v1/nodes/by-inode/:inode", get(get_node_by_inode))
            
            // Edge endpoints
            .route("/api/v1/edges", post(create_edge))
            .route("/api/v1/edges/:id", get(get_edge))
            .route("/api/v1/edges/:id", put(update_edge))
            .route("/api/v1/edges/:id", delete(delete_edge))
            .route("/api/v1/edges", get(list_edges))
            
            // Traversal endpoints
            .route("/api/v1/traversal", get(execute_traversal))
            .route("/api/v1/traversal/bfs", get(breadth_first_search))
            .route("/api/v1/traversal/dfs", get(depth_first_search))
            .route("/api/v1/traversal/dijkstra", get(dijkstra_shortest_path))
            .route("/api/v1/traversal/topological", get(topological_sort))
            
            // Graph query endpoints
            .route("/api/v1/query/nodes", get(query_nodes))
            .route("/api/v1/query/edges", get(query_edges))
            .route("/api/v1/query/neighbors", get(get_neighbors))
            
            // Statistics and health endpoints
            .route("/api/v1/stats", get(get_statistics))
            .route("/api/v1/health", get(health_check))
            
            // Add middleware
            .layer(
                ServiceBuilder::new()
                    .layer(CorsLayer::permissive())
                    .layer(tower_http::trace::TraceLayer::new_for_http())
            )
            .with_state(self.app_state.clone())
    }
}

// Node handlers
async fn create_node(
    State(state): State<AppState>,
    Json(request): Json<CreateNodeRequest>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let node_id = state.core
        .create_node(request.inode_number, request.node_type)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    // Add properties if provided
    if let Some(properties) = request.properties {
        state.core
            .update_node_properties(node_id, properties)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;
    }
    
    let node = state.core
        .get_node(node_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;
    
    Ok(Json(node.into()))
}

async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<NodeId>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let node = state.core
        .get_node(id)
        .await
        .map_err(|e| match e {
            VexGraphError::NodeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    Ok(Json(node.into()))
}

async fn get_node_by_inode(
    State(state): State<AppState>,
    Path(inode): Path<u64>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let node = state.core
        .get_node_by_inode(inode)
        .await
        .map_err(|e| match e {
            VexGraphError::NodeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    Ok(Json(node.into()))
}

async fn update_node(
    State(state): State<AppState>,
    Path(id): Path<NodeId>,
    Json(request): Json<UpdateNodeRequest>,
) -> Result<Json<NodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.core
        .update_node_properties(id, request.properties)
        .await
        .map_err(|e| match e {
            VexGraphError::NodeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    let node = state.core
        .get_node(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;
    
    Ok(Json(node.into()))
}

async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<NodeId>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.core
        .delete_node(id)
        .await
        .map_err(|e| match e {
            VexGraphError::NodeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    Ok(StatusCode::NO_CONTENT)
}

async fn list_nodes(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<NodeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement node listing with filtering
    // For now, return empty list
    Ok(Json(Vec::new()))
}

// Edge handlers
async fn create_edge(
    State(state): State<AppState>,
    Json(request): Json<CreateEdgeRequest>,
) -> Result<Json<EdgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let weight = request.weight.unwrap_or(1.0);
    let edge_id = state.core
        .create_edge(request.source_id, request.target_id, request.edge_type, weight)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    let edge = state.core
        .get_edge(edge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;
    
    Ok(Json(edge.into()))
}

async fn get_edge(
    State(state): State<AppState>,
    Path(id): Path<EdgeId>,
) -> Result<Json<EdgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let edge = state.core
        .get_edge(id)
        .await
        .map_err(|e| match e {
            VexGraphError::EdgeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    Ok(Json(edge.into()))
}

async fn update_edge(
    State(state): State<AppState>,
    Path(id): Path<EdgeId>,
    Json(request): Json<UpdateEdgeRequest>,
) -> Result<Json<EdgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement edge property updates
    let edge = state.core
        .get_edge(id)
        .await
        .map_err(|e| match e {
            VexGraphError::EdgeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    Ok(Json(edge.into()))
}

async fn delete_edge(
    State(state): State<AppState>,
    Path(id): Path<EdgeId>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.core
        .delete_edge(id)
        .await
        .map_err(|e| match e {
            VexGraphError::EdgeNotFound(_) => (StatusCode::NOT_FOUND, Json(e.into())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())),
        })?;
    
    Ok(StatusCode::NO_CONTENT)
}

async fn list_edges(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<EdgeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement edge listing with filtering
    // For now, return empty list
    Ok(Json(Vec::new()))
}

// Traversal handlers
async fn execute_traversal(
    State(state): State<AppState>,
    Query(params): Query<TraversalQueryParams>,
) -> Result<Json<TraversalResult>, (StatusCode, Json<ErrorResponse>)> {
    let query = TraversalQuery {
        algorithm: params.algorithm,
        start_node: params.start_node,
        end_node: params.end_node,
        max_depth: params.max_depth,
        max_results: params.max_results,
        node_filter: params.node_filter,
        edge_filter: params.edge_filter,
        weight_threshold: params.weight_threshold,
        timeout_ms: params.timeout_ms,
    };
    
    let result = state.traversal
        .execute_traversal(query)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    Ok(Json(result))
}

async fn breadth_first_search(
    State(state): State<AppState>,
    Query(params): Query<TraversalQueryParams>,
) -> Result<Json<TraversalResult>, (StatusCode, Json<ErrorResponse>)> {
    let mut query = TraversalQuery {
        algorithm: TraversalAlgorithm::BreadthFirstSearch,
        start_node: params.start_node,
        end_node: params.end_node,
        max_depth: params.max_depth,
        max_results: params.max_results,
        node_filter: params.node_filter,
        edge_filter: params.edge_filter,
        weight_threshold: params.weight_threshold,
        timeout_ms: params.timeout_ms,
    };
    
    let result = state.traversal
        .execute_traversal(query)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    Ok(Json(result))
}

async fn depth_first_search(
    State(state): State<AppState>,
    Query(params): Query<TraversalQueryParams>,
) -> Result<Json<TraversalResult>, (StatusCode, Json<ErrorResponse>)> {
    let query = TraversalQuery {
        algorithm: TraversalAlgorithm::DepthFirstSearch,
        start_node: params.start_node,
        end_node: params.end_node,
        max_depth: params.max_depth,
        max_results: params.max_results,
        node_filter: params.node_filter,
        edge_filter: params.edge_filter,
        weight_threshold: params.weight_threshold,
        timeout_ms: params.timeout_ms,
    };
    
    let result = state.traversal
        .execute_traversal(query)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    Ok(Json(result))
}

async fn dijkstra_shortest_path(
    State(state): State<AppState>,
    Query(params): Query<TraversalQueryParams>,
) -> Result<Json<TraversalResult>, (StatusCode, Json<ErrorResponse>)> {
    let query = TraversalQuery {
        algorithm: TraversalAlgorithm::Dijkstra,
        start_node: params.start_node,
        end_node: params.end_node,
        max_depth: params.max_depth,
        max_results: params.max_results,
        node_filter: params.node_filter,
        edge_filter: params.edge_filter,
        weight_threshold: params.weight_threshold,
        timeout_ms: params.timeout_ms,
    };
    
    let result = state.traversal
        .execute_traversal(query)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    Ok(Json(result))
}

async fn topological_sort(
    State(state): State<AppState>,
    Query(params): Query<TraversalQueryParams>,
) -> Result<Json<TraversalResult>, (StatusCode, Json<ErrorResponse>)> {
    let query = TraversalQuery {
        algorithm: TraversalAlgorithm::TopologicalSort,
        start_node: params.start_node,
        end_node: params.end_node,
        max_depth: params.max_depth,
        max_results: params.max_results,
        node_filter: params.node_filter,
        edge_filter: params.edge_filter,
        weight_threshold: params.weight_threshold,
        timeout_ms: params.timeout_ms,
    };
    
    let result = state.traversal
        .execute_traversal(query)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(e.into())))?;
    
    Ok(Json(result))
}

// Query handlers
async fn query_nodes(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<NodeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement advanced node queries
    Ok(Json(Vec::new()))
}

async fn query_edges(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<EdgeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement advanced edge queries
    Ok(Json(Vec::new()))
}

async fn get_neighbors(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<NodeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement neighbor queries
    Ok(Json(Vec::new()))
}

// Utility handlers
async fn get_statistics(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let core_stats = state.core
        .get_statistics()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;
    
    let traversal_stats = state.traversal
        .get_statistics()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e.into())))?;
    
    let stats = serde_json::json!({
        "core": core_stats,
        "traversal": traversal_stats,
        "timestamp": chrono::Utc::now()
    });
    
    Ok(Json(stats))
}

async fn health_check() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": "2.0.0"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_api_server_creation() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let traversal = Arc::new(TraversalEngine::new(core.clone()).await.unwrap());
        let property_graph = Arc::new(PropertyGraphManager::new(core.clone()).await.unwrap());
        let semantic = Arc::new(SemanticIntegration::new(core.clone()).await.unwrap());
        
        let server = VexGraphApiServer::new(
            core,
            traversal,
            property_graph,
            semantic,
            &config,
        ).await.unwrap();
        
        // Test that server was created successfully
        assert!(server.server_handle.is_none());
    }
}