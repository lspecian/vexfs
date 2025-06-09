//! RESTful API Server for VexFS Semantic Operation Journal
//! 
//! This module implements the RESTful API endpoints for querying and streaming
//! semantic events from the journal with efficient indexing and real-time capabilities.
//! 
//! Task 18.4: RESTful API for Journal Queries

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock as TokioRwLock};
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, error, debug, instrument};
use uuid::Uuid;

use crate::semantic_api::{
    types::*,
    SemanticResult, SemanticError,
    event_emission::get_global_emission_framework,
};

/// API Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub request_timeout_secs: u64,
    pub enable_cors: bool,
    pub enable_compression: bool,
    pub max_events_per_query: usize,
    pub max_concurrent_streams: usize,
    pub rate_limit_per_minute: u32,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            request_timeout_secs: 30,
            enable_cors: true,
            enable_compression: true,
            max_events_per_query: 10000,
            max_concurrent_streams: 100,
            rate_limit_per_minute: 1000,
        }
    }
}

/// Query parameters for event listing
#[derive(Debug, Deserialize)]
pub struct EventListParams {
    pub event_type: Option<String>,
    pub category: Option<String>,
    pub actor: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub include_payload: Option<bool>,
    pub include_metadata: Option<bool>,
    pub include_causality: Option<bool>,
}

/// Query parameters for event search
#[derive(Debug, Deserialize)]
pub struct EventSearchParams {
    pub q: Option<String>,
    pub event_types: Option<String>,
    pub categories: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub agent_id: Option<String>,
    pub transaction_id: Option<u64>,
    pub causality_chain_id: Option<u64>,
    pub path_pattern: Option<String>,
    pub min_priority: Option<String>,
    pub tags: Option<String>,
    pub min_relevance_score: Option<u32>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub include_payload: Option<bool>,
    pub include_metadata: Option<bool>,
    pub include_causality: Option<bool>,
    pub aggregation: Option<String>,
}

/// WebSocket subscription parameters
#[derive(Debug, Deserialize)]
pub struct StreamParams {
    pub event_types: Option<String>,
    pub categories: Option<String>,
    pub agent_id: Option<String>,
    pub buffer_size: Option<usize>,
    pub include_historical: Option<bool>,
    pub historical_limit: Option<usize>,
}

/// Journal statistics response
#[derive(Debug, Serialize, Clone)]
pub struct JournalStats {
    pub total_events: u64,
    pub events_by_type: HashMap<String, u64>,
    pub events_by_category: HashMap<String, u64>,
    pub events_by_priority: HashMap<String, u64>,
    pub oldest_event_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub newest_event_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub storage_size_bytes: u64,
    pub index_size_bytes: u64,
    pub active_streams: u64,
    pub query_performance: QueryPerformanceStats,
}

/// Query performance statistics
#[derive(Debug, Serialize, Clone)]
pub struct QueryPerformanceStats {
    pub avg_query_time_ms: f64,
    pub p95_query_time_ms: f64,
    pub p99_query_time_ms: f64,
    pub total_queries: u64,
    pub cache_hit_rate: f64,
}

/// Event storage and indexing interface
pub trait EventStorage: Send + Sync {
    /// Query events with filtering and pagination
    fn query_events<'a>(&'a self, query: &'a EventQuery) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<EventQueryResponse>> + Send + 'a>>;
    
    /// Get a specific event by ID
    fn get_event<'a>(&'a self, event_id: u64) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<Option<SemanticEvent>>> + Send + 'a>>;
    
    /// Get journal statistics
    fn get_stats<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<JournalStats>> + Send + 'a>>;
    
    /// Subscribe to real-time events
    fn subscribe_events<'a>(&'a self, filter: &'a EventFilter) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<broadcast::Receiver<SemanticEvent>>> + Send + 'a>>;
}

/// In-memory event storage implementation (for demonstration)
#[derive(Debug)]
pub struct InMemoryEventStorage {
    events: Arc<TokioRwLock<Vec<SemanticEvent>>>,
    event_broadcaster: broadcast::Sender<SemanticEvent>,
    stats: Arc<RwLock<JournalStats>>,
    query_times: Arc<RwLock<Vec<Duration>>>,
}

impl InMemoryEventStorage {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(10000);
        
        Self {
            events: Arc::new(TokioRwLock::new(Vec::new())),
            event_broadcaster: sender,
            stats: Arc::new(RwLock::new(JournalStats {
                total_events: 0,
                events_by_type: HashMap::new(),
                events_by_category: HashMap::new(),
                events_by_priority: HashMap::new(),
                oldest_event_timestamp: None,
                newest_event_timestamp: None,
                storage_size_bytes: 0,
                index_size_bytes: 0,
                active_streams: 0,
                query_performance: QueryPerformanceStats {
                    avg_query_time_ms: 0.0,
                    p95_query_time_ms: 0.0,
                    p99_query_time_ms: 0.0,
                    total_queries: 0,
                    cache_hit_rate: 0.0,
                },
            })),
            query_times: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add an event to storage (for testing)
    pub async fn add_event(&self, event: SemanticEvent) -> SemanticResult<()> {
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
        }
        
        // Broadcast to subscribers
        let _ = self.event_broadcaster.send(event.clone());
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_events += 1;
            
            let type_name = format!("{:?}", event.event_type);
            *stats.events_by_type.entry(type_name).or_insert(0) += 1;
            
            let category_name = format!("{:?}", event.event_type.category());
            *stats.events_by_category.entry(category_name).or_insert(0) += 1;
            
            let priority_name = format!("{:?}", event.priority);
            *stats.events_by_priority.entry(priority_name).or_insert(0) += 1;
            
            if stats.oldest_event_timestamp.is_none() || 
               Some(event.timestamp.timestamp) < stats.oldest_event_timestamp {
                stats.oldest_event_timestamp = Some(event.timestamp.timestamp);
            }
            
            if stats.newest_event_timestamp.is_none() || 
               Some(event.timestamp.timestamp) > stats.newest_event_timestamp {
                stats.newest_event_timestamp = Some(event.timestamp.timestamp);
            }
        }
        
        Ok(())
    }
    
    fn record_query_time(&self, duration: Duration) {
        let mut times = self.query_times.write().unwrap();
        times.push(duration);
        
        // Keep only last 1000 query times
        if times.len() > 1000 {
            let excess = times.len() - 1000;
            times.drain(0..excess);
        }
        
        // Update performance stats
        let mut stats = self.stats.write().unwrap();
        stats.query_performance.total_queries += 1;
        
        if !times.is_empty() {
            let total_ms: f64 = times.iter().map(|d| d.as_millis() as f64).sum();
            stats.query_performance.avg_query_time_ms = total_ms / times.len() as f64;
            
            let mut sorted_times = times.clone();
            sorted_times.sort();
            
            if sorted_times.len() >= 20 {
                let p95_idx = (sorted_times.len() as f64 * 0.95) as usize;
                let p99_idx = (sorted_times.len() as f64 * 0.99) as usize;
                
                stats.query_performance.p95_query_time_ms = sorted_times[p95_idx].as_millis() as f64;
                stats.query_performance.p99_query_time_ms = sorted_times[p99_idx].as_millis() as f64;
            }
        }
    }
}

impl EventStorage for InMemoryEventStorage {
    fn query_events<'a>(&'a self, query: &'a EventQuery) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<EventQueryResponse>> + Send + 'a>> {
        Box::pin(async move {
            let start_time = Instant::now();
            
            let events = self.events.read().await;
            let mut filtered_events: Vec<SemanticEvent> = events.iter()
                .filter(|event| self.matches_filter(event, &query.filter))
                .cloned()
                .collect();
            
            // Apply sorting
            if let Some(sort_by) = &query.sort_by {
                match sort_by {
                    SortBy::Timestamp => {
                        filtered_events.sort_by(|a, b| a.timestamp.timestamp.cmp(&b.timestamp.timestamp));
                    }
                    SortBy::EventId => {
                        filtered_events.sort_by(|a, b| a.event_id.cmp(&b.event_id));
                    }
                    SortBy::Priority => {
                        filtered_events.sort_by(|a, b| a.priority.cmp(&b.priority));
                    }
                    SortBy::RelevanceScore => {
                        filtered_events.sort_by(|a, b| b.agent_relevance_score.cmp(&a.agent_relevance_score));
                    }
                    SortBy::GlobalSequence => {
                        filtered_events.sort_by(|a, b| a.global_sequence.cmp(&b.global_sequence));
                    }
                }
            }
            
            let total_count = filtered_events.len();
            
            // Apply pagination
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100).min(10000); // Cap at 10k events
            
            let paginated_events: Vec<SemanticEvent> = filtered_events
                .into_iter()
                .skip(offset)
                .take(limit)
                .collect();
            
            let has_more = offset + paginated_events.len() < total_count;
            
            let query_time = start_time.elapsed();
            self.record_query_time(query_time);
            
            Ok(EventQueryResponse {
                events: paginated_events,
                total_count,
                has_more,
                aggregation_results: None, // TODO: Implement aggregation
                query_time_ms: query_time.as_millis() as u64,
            })
        })
    }
    
    fn get_event<'a>(&'a self, event_id: u64) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<Option<SemanticEvent>>> + Send + 'a>> {
        Box::pin(async move {
            let events = self.events.read().await;
            Ok(events.iter().find(|e| e.event_id == event_id).cloned())
        })
    }
    
    fn get_stats<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<JournalStats>> + Send + 'a>> {
        Box::pin(async move {
            Ok(self.stats.read().unwrap().clone())
        })
    }
    
    fn subscribe_events<'a>(&'a self, _filter: &'a EventFilter) -> std::pin::Pin<Box<dyn std::future::Future<Output = SemanticResult<broadcast::Receiver<SemanticEvent>>> + Send + 'a>> {
        Box::pin(async move {
            Ok(self.event_broadcaster.subscribe())
        })
    }
}

impl InMemoryEventStorage {
    fn matches_filter(&self, event: &SemanticEvent, filter: &EventFilter) -> bool {
        // Check event types
        if let Some(event_types) = &filter.event_types {
            if !event_types.contains(&event.event_type) {
                return false;
            }
        }
        
        // Check categories
        if let Some(categories) = &filter.categories {
            if !categories.contains(&event.event_type.category()) {
                return false;
            }
        }
        
        // Check time range
        if let Some(time_range) = &filter.time_range {
            if event.timestamp.timestamp < time_range.start || 
               event.timestamp.timestamp > time_range.end {
                return false;
            }
        }
        
        // Check agent ID
        if let Some(agent_id) = &filter.agent_id {
            if let Some(agent_context) = &event.context.agent {
                if &agent_context.agent_id != agent_id {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check transaction ID
        if let Some(transaction_id) = filter.transaction_id {
            if event.context.transaction_id != Some(transaction_id) {
                return false;
            }
        }
        
        // Check causality chain
        if let Some(causality_chain_id) = filter.causality_chain_id {
            if event.context.causality_chain_id != Some(causality_chain_id) {
                return false;
            }
        }
        
        // Check path pattern
        if let Some(path_pattern) = &filter.path_pattern {
            if let Some(fs_context) = &event.context.filesystem {
                if !fs_context.path.contains(path_pattern) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check minimum priority
        if let Some(min_priority) = filter.min_priority {
            if event.priority > min_priority {
                return false;
            }
        }
        
        // Check minimum relevance score
        if let Some(min_relevance) = filter.min_relevance_score {
            if event.agent_relevance_score < min_relevance {
                return false;
            }
        }
        
        true
    }
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn EventStorage>,
    pub config: Arc<ApiServerConfig>,
    pub active_connections: Arc<RwLock<u64>>,
}

/// Main API server
pub struct ApiServer {
    config: ApiServerConfig,
    storage: Arc<dyn EventStorage>,
}

impl ApiServer {
    pub fn new(config: ApiServerConfig, storage: Arc<dyn EventStorage>) -> Self {
        Self { config, storage }
    }
    
    pub async fn start(&self) -> SemanticResult<()> {
        let app_state = AppState {
            storage: Arc::clone(&self.storage),
            config: Arc::new(self.config.clone()),
            active_connections: Arc::new(RwLock::new(0)),
        };
        
        let app = self.create_router(app_state);
        
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("Starting VexFS Semantic API server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| SemanticError::network(format!("Failed to bind to {}: {}", addr, e)))?;
        
        axum::serve(listener, app).await
            .map_err(|e| SemanticError::network(format!("Server error: {}", e)))?;
        
        Ok(())
    }
    
    fn create_router(&self, state: AppState) -> Router {
        let mut router = Router::new()
            // Event query endpoints
            .route("/api/v1/semantic/events", get(list_events))
            .route("/api/v1/semantic/events/:id", get(get_event))
            .route("/api/v1/semantic/events/search", get(search_events))
            .route("/api/v1/semantic/events/stats", get(get_stats))
            
            // WebSocket streaming endpoint
            .route("/api/v1/semantic/events/stream", get(stream_events_ws))
            
            // Health check
            .route("/health", get(health_check))
            
            .with_state(state);
        
        // Add middleware
        let service_builder = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http());
        
        if self.config.enable_cors {
            router = router.layer(CorsLayer::permissive());
        }
        
        router.layer(service_builder)
    }
}

// Handler functions

/// List events with filtering and pagination
#[instrument(skip(state))]
async fn list_events(
    State(state): State<AppState>,
    Query(params): Query<EventListParams>,
) -> Result<Json<ApiResponse<EventQueryResponse>>, StatusCode> {
    let filter = build_event_filter_from_list_params(&params)?;
    let sort_by = parse_sort_by(&params.sort_by);
    
    let query = EventQuery {
        filter,
        limit: params.limit,
        offset: params.offset,
        sort_by,
        include_payload: params.include_payload.unwrap_or(false),
        include_metadata: params.include_metadata.unwrap_or(false),
        include_causality: params.include_causality.unwrap_or(false),
        aggregation: None,
    };
    
    match state.storage.query_events(&query).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            error!("Failed to query events: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific event by ID
#[instrument(skip(state))]
async fn get_event(
    State(state): State<AppState>,
    Path(event_id): Path<u64>,
) -> Result<Json<ApiResponse<Option<SemanticEvent>>>, StatusCode> {
    match state.storage.get_event(event_id).await {
        Ok(event) => Ok(Json(ApiResponse::success(event))),
        Err(e) => {
            error!("Failed to get event {}: {}", event_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Advanced search with query parameters
#[instrument(skip(state))]
async fn search_events(
    State(state): State<AppState>,
    Query(params): Query<EventSearchParams>,
) -> Result<Json<ApiResponse<EventQueryResponse>>, StatusCode> {
    let filter = build_event_filter_from_search_params(&params)?;
    let sort_by = parse_sort_by(&params.sort_by);
    
    let query = EventQuery {
        filter,
        limit: params.limit,
        offset: params.offset,
        sort_by,
        include_payload: params.include_payload.unwrap_or(false),
        include_metadata: params.include_metadata.unwrap_or(false),
        include_causality: params.include_causality.unwrap_or(false),
        aggregation: None, // TODO: Parse aggregation params
    };
    
    match state.storage.query_events(&query).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            error!("Failed to search events: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get journal statistics
#[instrument(skip(state))]
async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JournalStats>>, StatusCode> {
    match state.storage.get_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            error!("Failed to get stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// WebSocket streaming endpoint
#[instrument(skip(state))]
async fn stream_events_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<StreamParams>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state, params))
}

/// Handle WebSocket connections for real-time streaming
async fn handle_websocket(
    mut socket: axum::extract::ws::WebSocket,
    state: AppState,
    params: StreamParams,
) {
    info!("New WebSocket connection established");
    
    // Increment active connections
    {
        let mut connections = state.active_connections.write().unwrap();
        *connections += 1;
    }
    
    let filter = match build_event_filter_from_stream_params(&params) {
        Ok(filter) => filter,
        Err(_) => {
            let _ = socket.close().await;
            return;
        }
    };
    
    let mut receiver = match state.storage.subscribe_events(&filter).await {
        Ok(receiver) => receiver,
        Err(e) => {
            error!("Failed to subscribe to events: {}", e);
            let _ = socket.close().await;
            return;
        }
    };
    
    // Send historical events if requested
    if params.include_historical.unwrap_or(false) {
        let historical_limit = params.historical_limit.unwrap_or(100);
        let query = EventQuery {
            filter: filter.clone(),
            limit: Some(historical_limit),
            offset: None,
            sort_by: Some(SortBy::Timestamp),
            include_payload: true,
            include_metadata: true,
            include_causality: true,
            aggregation: None,
        };
        
        if let Ok(response) = state.storage.query_events(&query).await {
            for event in response.events {
                let message = StreamEventMessage {
                    subscription_id: Uuid::new_v4(),
                    event,
                    sequence_number: 0,
                    timestamp: chrono::Utc::now(),
                };
                
                if let Ok(json) = serde_json::to_string(&message) {
                    if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
    
    // Stream real-time events
    while let Ok(event) = receiver.recv().await {
        let message = StreamEventMessage {
            subscription_id: Uuid::new_v4(),
            event,
            sequence_number: 0,
            timestamp: chrono::Utc::now(),
        };
        
        if let Ok(json) = serde_json::to_string(&message) {
            if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
    
    // Decrement active connections
    {
        let mut connections = state.active_connections.write().unwrap();
        *connections = connections.saturating_sub(1);
    }
    
    info!("WebSocket connection closed");
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "service": "vexfs-semantic-api"
    }))
}

// Helper functions for parameter parsing

fn build_event_filter_from_list_params(params: &EventListParams) -> Result<EventFilter, StatusCode> {
    let mut filter = EventFilter {
        event_types: None,
        categories: None,
        time_range: None,
        agent_id: params.actor.clone(),
        transaction_id: None,
        causality_chain_id: None,
        path_pattern: None,
        min_priority: None,
        required_flags: None,
        tags: None,
        min_relevance_score: None,
    };
    
    // Parse event type
    if let Some(event_type_str) = &params.event_type {
        if let Ok(event_type) = parse_event_type(event_type_str) {
            filter.event_types = Some(vec![event_type]);
        }
    }
    
    // Parse category
    if let Some(category_str) = &params.category {
        if let Ok(category) = parse_event_category(category_str) {
            filter.categories = Some(vec![category]);
        }
    }
    
    // Parse time range
    if let (Some(start_str), Some(end_str)) = (&params.start_time, &params.end_time) {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(start_str),
            chrono::DateTime::parse_from_rfc3339(end_str)
        ) {
            filter.time_range = Some(TimeRange {
                start: start.with_timezone(&chrono::Utc),
                end: end.with_timezone(&chrono::Utc),
            });
        }
    }
    
    Ok(filter)
}

fn build_event_filter_from_search_params(params: &EventSearchParams) -> Result<EventFilter, StatusCode> {
    let mut filter = EventFilter {
        event_types: None,
        categories: None,
        time_range: None,
        agent_id: params.agent_id.clone(),
        transaction_id: params.transaction_id,
        causality_chain_id: params.causality_chain_id,
        path_pattern: params.path_pattern.clone(),
        min_priority: None,
        required_flags: None,
        tags: None,
        min_relevance_score: params.min_relevance_score,
    };
    
    // Parse event types (comma-separated)
    if let Some(types_str) = &params.event_types {
        let types: Result<Vec<_>, _> = types_str
            .split(',')
            .map(|s| parse_event_type(s.trim()))
            .collect();
        if let Ok(types) = types {
            filter.event_types = Some(types);
        }
    }
    
    // Parse categories (comma-separated)
    if let Some(categories_str) = &params.categories {
        let categories: Result<Vec<_>, _> = categories_str
            .split(',')
            .map(|s| parse_event_category(s.trim()))
            .collect();
        if let Ok(categories) = categories {
            filter.categories = Some(categories);
        }
    }
    
    // Parse time range
    if let (Some(start_str), Some(end_str)) = (&params.start_time, &params.end_time) {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(start_str),
            chrono::DateTime::parse_from_rfc3339(end_str)
        ) {
            filter.time_range = Some(TimeRange {
                start: start.with_timezone(&chrono::Utc),
                end: end.with_timezone(&chrono::Utc),
            });
        }
    }
    
    // Parse minimum priority
    if let Some(priority_str) = &params.min_priority {
        if let Ok(priority) = parse_event_priority(priority_str) {
            filter.min_priority = Some(priority);
        }
    }
    
    Ok(filter)
}

fn build_event_filter_from_stream_params(params: &StreamParams) -> Result<EventFilter, StatusCode> {
    let mut filter = EventFilter {
        event_types: None,
        categories: None,
        time_range: None,
        agent_id: params.agent_id.clone(),
        transaction_id: None,
        causality_chain_id: None,
        path_pattern: None,
        min_priority: None,
        required_flags: None,
        tags: None,
        min_relevance_score: None,
    };
    
    // Parse event types (comma-separated)
    if let Some(types_str) = &params.event_types {
        let types: Result<Vec<_>, _> = types_str
            .split(',')
            .map(|s| parse_event_type(s.trim()))
            .collect();
        if let Ok(types) = types {
            filter.event_types = Some(types);
        }
    }
    
    // Parse categories (comma-separated)
    if let Some(categories_str) = &params.categories {
        let categories: Result<Vec<_>, _> = categories_str
            .split(',')
            .map(|s| parse_event_category(s.trim()))
            .collect();
        if let Ok(categories) = categories {
            filter.categories = Some(categories);
        }
    }
    
    Ok(filter)
}

fn parse_event_type(s: &str) -> Result<SemanticEventType, StatusCode> {
    match s.to_lowercase().as_str() {
        "filesystem_create" => Ok(SemanticEventType::FilesystemCreate),
        "filesystem_delete" => Ok(SemanticEventType::FilesystemDelete),
        "filesystem_read" => Ok(SemanticEventType::FilesystemRead),
        "filesystem_write" => Ok(SemanticEventType::FilesystemWrite),
        "graph_node_create" => Ok(SemanticEventType::GraphNodeCreate),
        "graph_node_delete" => Ok(SemanticEventType::GraphNodeDelete),
        "graph_node_update" => Ok(SemanticEventType::GraphNodeUpdate),
        "graph_edge_create" => Ok(SemanticEventType::GraphEdgeCreate),
        "graph_edge_delete" => Ok(SemanticEventType::GraphEdgeDelete),
        "vector_create" => Ok(SemanticEventType::VectorCreate),
        "vector_delete" => Ok(SemanticEventType::VectorDelete),
        "vector_search" => Ok(SemanticEventType::VectorSearch),
        "agent_query" => Ok(SemanticEventType::AgentQuery),
        "system_mount" => Ok(SemanticEventType::SystemMount),
        "semantic_transaction_begin" => Ok(SemanticEventType::SemanticTransactionBegin),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn parse_event_category(s: &str) -> Result<EventCategory, StatusCode> {
    match s.to_lowercase().as_str() {
        "filesystem" => Ok(EventCategory::Filesystem),
        "graph" => Ok(EventCategory::Graph),
        "vector" => Ok(EventCategory::Vector),
        "agent" => Ok(EventCategory::Agent),
        "system" => Ok(EventCategory::System),
        "semantic" => Ok(EventCategory::Semantic),
        "observability" => Ok(EventCategory::Observability),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn parse_event_priority(s: &str) -> Result<EventPriority, StatusCode> {
    match s.to_lowercase().as_str() {
        "critical" => Ok(EventPriority::Critical),
        "high" => Ok(EventPriority::High),
        "normal" => Ok(EventPriority::Normal),
        "low" => Ok(EventPriority::Low),
        "background" => Ok(EventPriority::Background),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

fn parse_sort_by(s: &Option<String>) -> Option<SortBy> {
    s.as_ref().and_then(|sort_str| {
        match sort_str.to_lowercase().as_str() {
            "timestamp" => Some(SortBy::Timestamp),
            "event_id" => Some(SortBy::EventId),
            "priority" => Some(SortBy::Priority),
            "relevance_score" => Some(SortBy::RelevanceScore),
            "global_sequence" => Some(SortBy::GlobalSequence),
            _ => None,
        }
    })
}