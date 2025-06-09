//! WebSocket Streaming for VexFS Semantic Operation Journal
//! 
//! This module implements WebSocket-based real-time streaming of semantic events
//! with filtering, backpressure handling, and connection management.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock as TokioRwLock};
use tokio::time::{interval, timeout};
use tracing::{info, warn, error, debug, instrument};
use uuid::Uuid;

use crate::semantic_api::{
    types::*,
    SemanticResult, SemanticError,
};

/// WebSocket connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub max_message_size: usize,
    pub ping_interval_secs: u64,
    pub connection_timeout_secs: u64,
    pub max_buffer_size: usize,
    pub enable_compression: bool,
    pub heartbeat_interval_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: 1024 * 1024, // 1MB
            ping_interval_secs: 30,
            connection_timeout_secs: 300, // 5 minutes
            max_buffer_size: 1000,
            enable_compression: true,
            heartbeat_interval_secs: 10,
        }
    }
}

/// WebSocket connection state
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connection_id: Uuid,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub subscription: Option<StreamSubscription>,
    pub events_sent: u64,
    pub bytes_sent: u64,
    pub is_active: bool,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Subscribe to event stream
    Subscribe {
        subscription: StreamSubscription,
    },
    /// Unsubscribe from event stream
    Unsubscribe {
        subscription_id: Uuid,
    },
    /// Event data message
    Event {
        event: StreamEventMessage,
    },
    /// Heartbeat/ping message
    Ping {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Pong response
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Error message
    Error {
        error: String,
        code: u32,
    },
    /// Connection status
    Status {
        connection_id: Uuid,
        events_sent: u64,
        uptime_secs: u64,
    },
}

/// WebSocket connection manager
pub struct WebSocketConnectionManager {
    config: WebSocketConfig,
    connections: Arc<TokioRwLock<HashMap<Uuid, ConnectionState>>>,
    event_broadcaster: broadcast::Sender<SemanticEvent>,
}

impl WebSocketConnectionManager {
    pub fn new(config: WebSocketConfig) -> Self {
        let (sender, _) = broadcast::channel(10000);
        
        Self {
            config,
            connections: Arc::new(TokioRwLock::new(HashMap::new())),
            event_broadcaster: sender,
        }
    }
    
    /// Handle a new WebSocket connection
    #[instrument(skip(self, socket))]
    pub async fn handle_connection(&self, socket: WebSocket) -> SemanticResult<()> {
        let connection_id = Uuid::new_v4();
        let connection_state = ConnectionState {
            connection_id,
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            subscription: None,
            events_sent: 0,
            bytes_sent: 0,
            is_active: true,
        };
        
        // Register connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, connection_state);
        }
        
        info!("New WebSocket connection established: {}", connection_id);
        
        // Handle the connection
        let result = self.handle_connection_lifecycle(socket, connection_id).await;
        
        // Cleanup connection
        {
            let mut connections = self.connections.write().await;
            connections.remove(&connection_id);
        }
        
        info!("WebSocket connection closed: {}", connection_id);
        result
    }
    
    /// Handle the full lifecycle of a WebSocket connection
    async fn handle_connection_lifecycle(
        &self,
        socket: WebSocket,
        connection_id: Uuid,
    ) -> SemanticResult<()> {
        let (mut sender, mut receiver) = socket.split();
        
        // Create channels for internal communication
        let (tx, mut rx) = mpsc::unbounded_channel::<WebSocketMessage>();
        
        // Spawn heartbeat task
        let heartbeat_tx = tx.clone();
        let heartbeat_interval = self.config.heartbeat_interval_secs;
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(heartbeat_interval));
            loop {
                interval.tick().await;
                if heartbeat_tx.send(WebSocketMessage::Ping {
                    timestamp: chrono::Utc::now(),
                }).is_err() {
                    break;
                }
            }
        });
        
        // Spawn message sender task
        let sender_task = {
            let config = self.config.clone();
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    let json = match serde_json::to_string(&message) {
                        Ok(json) => json,
                        Err(e) => {
                            error!("Failed to serialize WebSocket message: {}", e);
                            continue;
                        }
                    };
                    
                    if json.len() > config.max_message_size {
                        warn!("WebSocket message too large: {} bytes", json.len());
                        continue;
                    }
                    
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            })
        };
        
        // Handle incoming messages
        let message_handler = {
            let tx = tx.clone();
            let connections = Arc::clone(&self.connections);
            let event_broadcaster = self.event_broadcaster.clone();
            
            tokio::spawn(async move {
                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Err(e) = Self::handle_text_message(
                                &text,
                                connection_id,
                                &tx,
                                &connections,
                                &event_broadcaster,
                            ).await {
                                error!("Error handling WebSocket message: {}", e);
                                let _ = tx.send(WebSocketMessage::Error {
                                    error: e.to_string(),
                                    code: 500,
                                });
                            }
                        }
                        Ok(Message::Binary(_)) => {
                            warn!("Binary messages not supported");
                        }
                        Ok(Message::Ping(data)) => {
                            // Echo pong
                            let _ = tx.send(WebSocketMessage::Pong {
                                timestamp: chrono::Utc::now(),
                            });
                        }
                        Ok(Message::Pong(_)) => {
                            // Update last activity
                            Self::update_last_activity(connection_id, &connections).await;
                        }
                        Ok(Message::Close(_)) => {
                            info!("WebSocket close message received");
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
            })
        };
        
        // Wait for either task to complete
        tokio::select! {
            _ = sender_task => {
                debug!("WebSocket sender task completed");
            }
            _ = message_handler => {
                debug!("WebSocket message handler completed");
            }
            _ = tokio::time::sleep(Duration::from_secs(self.config.connection_timeout_secs)) => {
                warn!("WebSocket connection timed out");
            }
        }
        
        Ok(())
    }
    
    /// Handle incoming text messages
    async fn handle_text_message(
        text: &str,
        connection_id: Uuid,
        tx: &mpsc::UnboundedSender<WebSocketMessage>,
        connections: &Arc<TokioRwLock<HashMap<Uuid, ConnectionState>>>,
        event_broadcaster: &broadcast::Sender<SemanticEvent>,
    ) -> SemanticResult<()> {
        let message: WebSocketMessage = serde_json::from_str(text)
            .map_err(|e| SemanticError::validation(format!("Invalid JSON: {}", e)))?;
        
        match message {
            WebSocketMessage::Subscribe { subscription } => {
                Self::handle_subscribe(connection_id, subscription, connections, event_broadcaster, tx).await?;
            }
            WebSocketMessage::Unsubscribe { subscription_id } => {
                Self::handle_unsubscribe(connection_id, subscription_id, connections).await?;
            }
            WebSocketMessage::Ping { .. } => {
                let _ = tx.send(WebSocketMessage::Pong {
                    timestamp: chrono::Utc::now(),
                });
            }
            WebSocketMessage::Pong { .. } => {
                Self::update_last_activity(connection_id, connections).await;
            }
            _ => {
                return Err(SemanticError::validation("Unsupported message type".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Handle subscription request
    async fn handle_subscribe(
        connection_id: Uuid,
        subscription: StreamSubscription,
        connections: &Arc<TokioRwLock<HashMap<Uuid, ConnectionState>>>,
        event_broadcaster: &broadcast::Sender<SemanticEvent>,
        tx: &mpsc::UnboundedSender<WebSocketMessage>,
    ) -> SemanticResult<()> {
        // Update connection state
        {
            let mut connections_guard = connections.write().await;
            if let Some(conn_state) = connections_guard.get_mut(&connection_id) {
                conn_state.subscription = Some(subscription.clone());
                conn_state.last_activity = chrono::Utc::now();
            }
        }
        
        // Subscribe to event stream
        let mut receiver = event_broadcaster.subscribe();
        let tx_clone = tx.clone();
        let subscription_clone = subscription.clone();
        let connections_clone = Arc::clone(connections);
        
        tokio::spawn(async move {
            let mut sequence_number = 0u64;
            
            while let Ok(event) = receiver.recv().await {
                // Check if connection is still active
                let is_active = {
                    let connections_guard = connections_clone.read().await;
                    connections_guard.get(&connection_id)
                        .map(|conn| conn.is_active)
                        .unwrap_or(false)
                };
                
                if !is_active {
                    break;
                }
                
                // Apply filter
                if Self::event_matches_filter(&event, &subscription_clone.filter) {
                    sequence_number += 1;
                    
                    let stream_message = StreamEventMessage {
                        subscription_id: subscription_clone.subscription_id,
                        event,
                        sequence_number,
                        timestamp: chrono::Utc::now(),
                    };
                    
                    let ws_message = WebSocketMessage::Event {
                        event: stream_message,
                    };
                    
                    if tx_clone.send(ws_message).is_err() {
                        break;
                    }
                    
                    // Update connection statistics
                    {
                        let mut connections_guard = connections_clone.write().await;
                        if let Some(conn_state) = connections_guard.get_mut(&connection_id) {
                            conn_state.events_sent += 1;
                            conn_state.last_activity = chrono::Utc::now();
                        }
                    }
                }
            }
        });
        
        info!("Subscription created for connection {}: {:?}", connection_id, subscription.subscription_id);
        Ok(())
    }
    
    /// Handle unsubscribe request
    async fn handle_unsubscribe(
        connection_id: Uuid,
        _subscription_id: Uuid,
        connections: &Arc<TokioRwLock<HashMap<Uuid, ConnectionState>>>,
    ) -> SemanticResult<()> {
        let mut connections_guard = connections.write().await;
        if let Some(conn_state) = connections_guard.get_mut(&connection_id) {
            conn_state.subscription = None;
            conn_state.last_activity = chrono::Utc::now();
        }
        
        info!("Unsubscribed connection {}", connection_id);
        Ok(())
    }
    
    /// Update last activity timestamp
    async fn update_last_activity(
        connection_id: Uuid,
        connections: &Arc<TokioRwLock<HashMap<Uuid, ConnectionState>>>,
    ) {
        let mut connections_guard = connections.write().await;
        if let Some(conn_state) = connections_guard.get_mut(&connection_id) {
            conn_state.last_activity = chrono::Utc::now();
        }
    }
    
    /// Check if an event matches the subscription filter
    fn event_matches_filter(event: &SemanticEvent, filter: &EventFilter) -> bool {
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
        
        // Check minimum priority
        if let Some(min_priority) = filter.min_priority {
            if event.priority > min_priority {
                return false;
            }
        }
        
        true
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> HashMap<Uuid, ConnectionState> {
        self.connections.read().await.clone()
    }
    
    /// Broadcast an event to all subscribers
    pub async fn broadcast_event(&self, event: SemanticEvent) -> SemanticResult<()> {
        let _ = self.event_broadcaster.send(event);
        Ok(())
    }
    
    /// Get active connection count
    pub async fn get_active_connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// Cleanup inactive connections
    pub async fn cleanup_inactive_connections(&self) -> usize {
        let timeout_duration = Duration::from_secs(self.config.connection_timeout_secs);
        let now = chrono::Utc::now();
        let mut removed_count = 0;
        
        let mut connections = self.connections.write().await;
        connections.retain(|_, conn_state| {
            let inactive_duration = now.signed_duration_since(conn_state.last_activity);
            let is_active = inactive_duration.num_seconds() < timeout_duration.as_secs() as i64;
            
            if !is_active {
                removed_count += 1;
            }
            
            is_active
        });
        
        if removed_count > 0 {
            info!("Cleaned up {} inactive WebSocket connections", removed_count);
        }
        
        removed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_api::types::*;
    
    #[tokio::test]
    async fn test_websocket_connection_manager() {
        let config = WebSocketConfig::default();
        let manager = WebSocketConnectionManager::new(config);
        
        // Test initial state
        assert_eq!(manager.get_active_connection_count().await, 0);
        
        // Test event broadcasting
        let event = SemanticEvent {
            event_id: 1,
            event_type: SemanticEventType::FilesystemCreate,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: chrono::Utc::now(),
                sequence: 1,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: 1,
            local_sequence: 1,
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::Normal,
            event_size: 256,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFFFFFFFFFFFFFFFF,
            agent_relevance_score: 100,
            replay_priority: 3,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: Some(FilesystemContext {
                    path: "/test/file.txt".to_string(),
                    inode_number: Some(12345),
                    file_type: Some("regular".to_string()),
                }),
                graph: None,
                vector: None,
                agent: None,
                system: None,
                semantic: None,
                observability: None,
            },
            payload: None,
            metadata: None,
        };
        
        manager.broadcast_event(event).await.unwrap();
    }
    
    #[test]
    fn test_event_filter_matching() {
        let event = SemanticEvent {
            event_id: 1,
            event_type: SemanticEventType::FilesystemCreate,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: chrono::Utc::now(),
                sequence: 1,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: 1,
            local_sequence: 1,
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::Normal,
            event_size: 256,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFFFFFFFFFFFFFFFF,
            agent_relevance_score: 100,
            replay_priority: 3,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: Some(FilesystemContext {
                    path: "/test/file.txt".to_string(),
                    inode_number: Some(12345),
                    file_type: Some("regular".to_string()),
                }),
                graph: None,
                vector: None,
                agent: None,
                system: None,
                semantic: None,
                observability: None,
            },
            payload: None,
            metadata: None,
        };
        
        // Test matching event type
        let filter = EventFilter {
            event_types: Some(vec![SemanticEventType::FilesystemCreate]),
            categories: None,
            time_range: None,
            agent_id: None,
            transaction_id: None,
            causality_chain_id: None,
            path_pattern: None,
            min_priority: None,
            required_flags: None,
            tags: None,
            min_relevance_score: None,
        };
        
        assert!(WebSocketConnectionManager::event_matches_filter(&event, &filter));
        
        // Test non-matching event type
        let filter = EventFilter {
            event_types: Some(vec![SemanticEventType::FilesystemDelete]),
            categories: None,
            time_range: None,
            agent_id: None,
            transaction_id: None,
            causality_chain_id: None,
            path_pattern: None,
            min_priority: None,
            required_flags: None,
            tags: None,
            min_relevance_score: None,
        };
        
        assert!(!WebSocketConnectionManager::event_matches_filter(&event, &filter));
    }
}