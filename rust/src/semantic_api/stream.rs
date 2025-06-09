//! Real-time Event Streaming for Semantic API
//! 
//! This module implements real-time event stream subscription mechanism
//! for live agent monitoring of semantic events.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use crate::semantic_api::kernel_interface;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio_stream::{Stream, StreamExt};
use futures::stream::BoxStream;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde_json;

/// Stream manager for handling event subscriptions
#[derive(Debug)]
pub struct StreamManager {
    /// Active subscriptions
    subscriptions: Arc<RwLock<HashMap<Uuid, ActiveSubscription>>>,
    /// Event broadcaster
    event_broadcaster: broadcast::Sender<SemanticEvent>,
    /// Stream statistics
    stats: Arc<RwLock<StreamStats>>,
    /// Configuration
    config: StreamConfig,
}

/// Stream configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Maximum subscriptions per agent
    pub max_subscriptions_per_agent: usize,
    /// Maximum buffer size per subscription
    pub max_buffer_size: usize,
    /// Default buffer size
    pub default_buffer_size: usize,
    /// Subscription timeout in seconds
    pub subscription_timeout_seconds: u64,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_seconds: u64,
    /// Maximum historical events to send
    pub max_historical_events: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_subscriptions_per_agent: 10,
            max_buffer_size: 10000,
            default_buffer_size: 1000,
            subscription_timeout_seconds: 300, // 5 minutes
            heartbeat_interval_seconds: 30,
            max_historical_events: 1000,
        }
    }
}

/// Active subscription tracking
#[derive(Debug)]
struct ActiveSubscription {
    /// Subscription details
    subscription: StreamSubscription,
    /// Event sender
    sender: mpsc::UnboundedSender<StreamMessage>,
    /// Creation time
    created_at: DateTime<Utc>,
    /// Last activity time
    last_activity: DateTime<Utc>,
    /// Events sent count
    events_sent: u64,
    /// Agent ID
    agent_id: String,
}

/// Stream statistics
#[derive(Debug, Default, Clone)]
pub struct StreamStats {
    /// Total subscriptions created
    pub total_subscriptions: u64,
    /// Active subscriptions
    pub active_subscriptions: u32,
    /// Total events streamed
    pub total_events_streamed: u64,
    /// Events per second (approximate)
    pub events_per_second: f64,
    /// Subscriptions by agent
    pub subscriptions_by_agent: HashMap<String, u32>,
    /// Last stats update
    pub last_update: DateTime<Utc>,
}

/// Stream message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum StreamMessage {
    /// Event message
    Event {
        subscription_id: Uuid,
        event: SemanticEvent,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    /// Heartbeat message
    Heartbeat {
        subscription_id: Uuid,
        timestamp: DateTime<Utc>,
        events_sent: u64,
    },
    /// Error message
    Error {
        subscription_id: Uuid,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Subscription confirmation
    Subscribed {
        subscription_id: Uuid,
        timestamp: DateTime<Utc>,
        historical_events_count: usize,
    },
    /// Subscription ended
    Unsubscribed {
        subscription_id: Uuid,
        timestamp: DateTime<Utc>,
        reason: String,
    },
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new(config: StreamConfig) -> Self {
        let (event_broadcaster, _) = broadcast::channel(10000);
        
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            stats: Arc::new(RwLock::new(StreamStats::default())),
            config,
        }
    }
    
    /// Create a new event stream subscription
    pub async fn subscribe(
        &self,
        agent_id: String,
        filter: EventFilter,
        buffer_size: Option<usize>,
        include_historical: bool,
        historical_limit: Option<usize>,
    ) -> SemanticResult<(Uuid, BoxStream<'static, StreamMessage>)> {
        // Check agent subscription limits
        self.check_agent_subscription_limit(&agent_id).await?;
        
        let subscription_id = Uuid::new_v4();
        let buffer_size = buffer_size
            .unwrap_or(self.config.default_buffer_size)
            .min(self.config.max_buffer_size);
        
        // Create subscription
        let subscription = StreamSubscription {
            subscription_id,
            agent_id: agent_id.clone(),
            filter: filter.clone(),
            buffer_size,
            include_historical,
            historical_limit,
        };
        
        // Create message channel
        let (sender, mut receiver) = mpsc::unbounded_channel();
        
        // Create active subscription
        let active_subscription = ActiveSubscription {
            subscription: subscription.clone(),
            sender: sender.clone(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            events_sent: 0,
            agent_id: agent_id.clone(),
        };
        
        // Store subscription
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(subscription_id, active_subscription);
        }
        
        // Update statistics
        self.update_stats(|stats| {
            stats.total_subscriptions += 1;
            stats.active_subscriptions += 1;
            *stats.subscriptions_by_agent.entry(agent_id.clone()).or_insert(0) += 1;
        }).await;
        
        // Send historical events if requested
        if include_historical {
            self.send_historical_events(&subscription, &sender).await?;
        }
        
        // Send subscription confirmation
        let confirmation = StreamMessage::Subscribed {
            subscription_id,
            timestamp: Utc::now(),
            historical_events_count: if include_historical { 
                historical_limit.unwrap_or(self.config.max_historical_events) 
            } else { 
                0 
            },
        };
        
        if sender.send(confirmation).is_err() {
            return Err(SemanticError::StreamError(
                "Failed to send subscription confirmation".to_string()
            ));
        }
        
        // Subscribe to live events
        let mut event_receiver = self.event_broadcaster.subscribe();
        let subscriptions_ref = self.subscriptions.clone();
        let stats_ref = self.stats.clone();
        
        // Spawn task to handle live events
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                // Check if subscription still exists
                let should_continue = {
                    let subscriptions = subscriptions_ref.read().await;
                    subscriptions.contains_key(&subscription_id)
                };
                
                if !should_continue {
                    break;
                }
                
                // Check if event matches filter
                if Self::event_matches_filter(&event, &filter) {
                    let message = StreamMessage::Event {
                        subscription_id,
                        event,
                        sequence_number: 0, // TODO: Implement proper sequence numbering
                        timestamp: Utc::now(),
                    };
                    
                    if sender.send(message).is_err() {
                        // Receiver dropped, clean up subscription
                        let mut subscriptions = subscriptions_ref.write().await;
                        subscriptions.remove(&subscription_id);
                        break;
                    }
                    
                    // Update statistics
                    let mut stats = stats_ref.write().await;
                    stats.total_events_streamed += 1;
                }
            }
        });
        
        // Create stream from receiver
        let stream = async_stream::stream! {
            while let Some(message) = receiver.recv().await {
                yield message;
            }
        };
        
        tracing::info!("Created subscription {} for agent {}", subscription_id, agent_id);
        Ok((subscription_id, Box::pin(stream)))
    }
    
    /// Unsubscribe from event stream
    pub async fn unsubscribe(&self, subscription_id: Uuid, reason: String) -> SemanticResult<()> {
        let mut subscriptions = self.subscriptions.write().await;
        
        if let Some(subscription) = subscriptions.remove(&subscription_id) {
            // Send unsubscribe message
            let message = StreamMessage::Unsubscribed {
                subscription_id,
                timestamp: Utc::now(),
                reason: reason.clone(),
            };
            
            let _ = subscription.sender.send(message); // Ignore if receiver is gone
            
            // Update statistics
            self.update_stats(|stats| {
                stats.active_subscriptions = stats.active_subscriptions.saturating_sub(1);
                if let Some(count) = stats.subscriptions_by_agent.get_mut(&subscription.agent_id) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        stats.subscriptions_by_agent.remove(&subscription.agent_id);
                    }
                }
            }).await;
            
            tracing::info!("Unsubscribed {} (reason: {})", subscription_id, reason);
            Ok(())
        } else {
            Err(SemanticError::InvalidRequest(
                format!("Subscription {} not found", subscription_id)
            ))
        }
    }
    
    /// Broadcast an event to all matching subscriptions
    pub async fn broadcast_event(&self, event: SemanticEvent) -> SemanticResult<usize> {
        let sent_count = self.event_broadcaster.send(event)
            .map_err(|_| SemanticError::StreamError(
                "Failed to broadcast event".to_string()
            ))?;
        
        Ok(sent_count)
    }
    
    /// Send historical events for a new subscription
    async fn send_historical_events(
        &self,
        subscription: &StreamSubscription,
        sender: &mpsc::UnboundedSender<StreamMessage>,
    ) -> SemanticResult<()> {
        if !subscription.include_historical {
            return Ok(());
        }
        
        let limit = subscription.historical_limit
            .unwrap_or(self.config.max_historical_events)
            .min(self.config.max_historical_events);
        
        // Query historical events from kernel interface
        let query = EventQuery {
            filter: subscription.filter.clone(),
            limit: Some(limit),
            offset: None,
            sort_by: Some(SortBy::Timestamp),
            include_payload: true,
            include_metadata: true,
            include_causality: true,
            aggregation: None,
        };
        
        let kernel_interface = kernel_interface::get_interface()?;
        let response = kernel_interface.query_events(&query).await?;
        
        // Send historical events
        for event in response.events {
            let message = StreamMessage::Event {
                subscription_id: subscription.subscription_id,
                event,
                sequence_number: 0, // TODO: Implement proper sequence numbering
                timestamp: Utc::now(),
            };
            
            if sender.send(message).is_err() {
                return Err(SemanticError::StreamError(
                    "Failed to send historical event".to_string()
                ));
            }
        }
        
        tracing::debug!("Sent {} historical events for subscription {}", 
                       response.events.len(), subscription.subscription_id);
        Ok(())
    }
    
    /// Check if an agent has reached subscription limits
    async fn check_agent_subscription_limit(&self, agent_id: &str) -> SemanticResult<()> {
        let subscriptions = self.subscriptions.read().await;
        let agent_subscription_count = subscriptions.values()
            .filter(|sub| sub.agent_id == agent_id)
            .count();
        
        if agent_subscription_count >= self.config.max_subscriptions_per_agent {
            return Err(SemanticError::RateLimitExceeded(
                format!("Agent {} has reached maximum subscription limit of {}", 
                       agent_id, self.config.max_subscriptions_per_agent)
            ));
        }
        
        Ok(())
    }
    
    /// Check if an event matches a filter
    fn event_matches_filter(event: &SemanticEvent, filter: &EventFilter) -> bool {
        // Check event types
        if let Some(ref types) = filter.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }
        
        // Check categories
        if let Some(ref categories) = filter.categories {
            if !categories.contains(&event.event_type.category()) {
                return false;
            }
        }
        
        // Check time range
        if let Some(ref time_range) = filter.time_range {
            if event.timestamp.timestamp < time_range.start || 
               event.timestamp.timestamp > time_range.end {
                return false;
            }
        }
        
        // Check priority
        if let Some(min_priority) = filter.min_priority {
            if event.priority > min_priority {
                return false;
            }
        }
        
        // Check relevance score
        if let Some(min_score) = filter.min_relevance_score {
            if event.agent_relevance_score < min_score {
                return false;
            }
        }
        
        true
    }
    
    /// Get stream statistics
    pub async fn get_stats(&self) -> StreamStats {
        let mut stats = self.stats.write().await;
        stats.last_update = Utc::now();
        stats.clone()
    }
    
    /// Get subscription information
    pub async fn get_subscription_info(&self, subscription_id: Uuid) -> Option<SubscriptionInfo> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(&subscription_id).map(|sub| SubscriptionInfo {
            subscription_id: sub.subscription.subscription_id,
            agent_id: sub.agent_id.clone(),
            created_at: sub.created_at,
            last_activity: sub.last_activity,
            events_sent: sub.events_sent,
            filter: sub.subscription.filter.clone(),
            buffer_size: sub.subscription.buffer_size,
        })
    }
    
    /// List subscriptions for an agent
    pub async fn list_agent_subscriptions(&self, agent_id: &str) -> Vec<SubscriptionInfo> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.values()
            .filter(|sub| sub.agent_id == agent_id)
            .map(|sub| SubscriptionInfo {
                subscription_id: sub.subscription.subscription_id,
                agent_id: sub.agent_id.clone(),
                created_at: sub.created_at,
                last_activity: sub.last_activity,
                events_sent: sub.events_sent,
                filter: sub.subscription.filter.clone(),
                buffer_size: sub.subscription.buffer_size,
            })
            .collect()
    }
    
    /// Clean up inactive subscriptions
    pub async fn cleanup_inactive_subscriptions(&self) -> SemanticResult<usize> {
        let timeout = chrono::Duration::seconds(self.config.subscription_timeout_seconds as i64);
        let cutoff_time = Utc::now() - timeout;
        
        let mut subscriptions = self.subscriptions.write().await;
        let initial_count = subscriptions.len();
        
        subscriptions.retain(|_, sub| sub.last_activity > cutoff_time);
        
        let cleaned_count = initial_count - subscriptions.len();
        
        if cleaned_count > 0 {
            self.update_stats(|stats| {
                stats.active_subscriptions = subscriptions.len() as u32;
            }).await;
            
            tracing::info!("Cleaned up {} inactive subscriptions", cleaned_count);
        }
        
        Ok(cleaned_count)
    }
    
    /// Start heartbeat task
    pub async fn start_heartbeat_task(&self) -> SemanticResult<()> {
        let subscriptions_ref = self.subscriptions.clone();
        let heartbeat_interval = chrono::Duration::seconds(self.config.heartbeat_interval_seconds as i64);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(heartbeat_interval.to_std().unwrap());
            
            loop {
                interval.tick().await;
                
                let subscriptions = subscriptions_ref.read().await;
                for (subscription_id, subscription) in subscriptions.iter() {
                    let heartbeat = StreamMessage::Heartbeat {
                        subscription_id: *subscription_id,
                        timestamp: Utc::now(),
                        events_sent: subscription.events_sent,
                    };
                    
                    let _ = subscription.sender.send(heartbeat); // Ignore if receiver is gone
                }
            }
        });
        
        tracing::info!("Started stream heartbeat task");
        Ok(())
    }
    
    /// Update statistics
    async fn update_stats<F>(&self, update_fn: F) 
    where
        F: FnOnce(&mut StreamStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut *stats);
    }
}

/// Subscription information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionInfo {
    pub subscription_id: Uuid,
    pub agent_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub events_sent: u64,
    pub filter: EventFilter,
    pub buffer_size: usize,
}

/// Global stream manager instance
static mut STREAM_MANAGER: Option<Arc<StreamManager>> = None;
static INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Initialize the global stream manager
pub async fn initialize(config: StreamConfig) -> SemanticResult<()> {
    let stream_manager = StreamManager::new(config);
    
    // Start background tasks
    stream_manager.start_heartbeat_task().await?;
    
    unsafe {
        INIT_ONCE.call_once(|| {
            STREAM_MANAGER = Some(Arc::new(stream_manager));
        });
    }
    
    tracing::info!("Stream manager initialized");
    Ok(())
}

/// Get the global stream manager
pub fn get_stream_manager() -> SemanticResult<Arc<StreamManager>> {
    unsafe {
        STREAM_MANAGER.as_ref()
            .cloned()
            .ok_or_else(|| SemanticError::InternalError(
                "Stream manager not initialized".to_string()
            ))
    }
}

/// Shutdown the stream manager
pub async fn shutdown() -> SemanticResult<()> {
    unsafe {
        STREAM_MANAGER = None;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_manager_creation() {
        let config = StreamConfig::default();
        let stream_manager = StreamManager::new(config);
        
        let stats = stream_manager.get_stats().await;
        assert_eq!(stats.total_subscriptions, 0);
        assert_eq!(stats.active_subscriptions, 0);
    }
    
    #[tokio::test]
    async fn test_subscription_limits() {
        let config = StreamConfig {
            max_subscriptions_per_agent: 2,
            ..Default::default()
        };
        let stream_manager = StreamManager::new(config);
        
        let agent_id = "test_agent".to_string();
        let filter = EventFilter {
            event_types: None,
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
        
        // First two subscriptions should succeed
        assert!(stream_manager.subscribe(agent_id.clone(), filter.clone(), None, false, None).await.is_ok());
        assert!(stream_manager.subscribe(agent_id.clone(), filter.clone(), None, false, None).await.is_ok());
        
        // Third subscription should fail
        assert!(stream_manager.subscribe(agent_id.clone(), filter.clone(), None, false, None).await.is_err());
    }
    
    #[tokio::test]
    async fn test_event_filtering() {
        let event = SemanticEvent {
            event_id: 1,
            event_type: SemanticEventType::FilesystemCreate,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: 1,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: 1,
            local_sequence: 1,
            flags: EventFlags::from_kernel_flags(0),
            priority: EventPriority::Normal,
            event_size: 100,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 50,
            replay_priority: 1,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: None,
                graph: None,
                vector: None,
                agent: None,
                system: None,
                semantic: None,
            },
            payload: None,
            metadata: None,
        };
        
        // Test event type filter
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
        
        assert!(StreamManager::event_matches_filter(&event, &filter));
        
        // Test category filter
        let filter = EventFilter {
            event_types: None,
            categories: Some(vec![EventCategory::Filesystem]),
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
        
        assert!(StreamManager::event_matches_filter(&event, &filter));
    }
}