//! Event Emission Framework for VexFS Semantic Operation Journal
//! 
//! This module implements the unified event emission API that can be called
//! from different layers (kernel, userspace, graph, vector) to capture
//! semantic events for the operation journal.

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, RwLock as TokioRwLock, Mutex as TokioMutex};
use tracing::{info, warn, error, debug, trace, instrument};
use uuid::Uuid;

use crate::semantic_api::types::{
    SemanticEvent, SemanticEventType, SemanticTimestamp, SemanticContext,
    EventFlags, EventPriority, FilesystemContext, GraphContext, VectorContext,
    AgentContext, SystemContext, SemanticContextData, ObservabilityContext
};
use crate::cross_layer_integration::CrossLayerIntegrationFramework;

/// Event emission configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEmissionConfig {
    pub enabled: bool,
    pub buffer_size: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub max_events_per_second: u32,
    pub enable_kernel_events: bool,
    pub enable_userspace_events: bool,
    pub enable_graph_events: bool,
    pub enable_vector_events: bool,
    pub enable_agent_events: bool,
    pub enable_system_events: bool,
    pub enable_semantic_events: bool,
    pub enable_observability_events: bool,
    pub thread_safe: bool,
    pub compression_enabled: bool,
}

impl Default for EventEmissionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 10000,
            batch_size: 100,
            flush_interval_ms: 100,
            max_events_per_second: 10000,
            enable_kernel_events: true,
            enable_userspace_events: true,
            enable_graph_events: true,
            enable_vector_events: true,
            enable_agent_events: true,
            enable_system_events: true,
            enable_semantic_events: true,
            enable_observability_events: true,
            thread_safe: true,
            compression_enabled: false,
        }
    }
}

/// Event emission statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventEmissionStats {
    pub total_events_emitted: u64,
    pub events_by_type: HashMap<String, u64>,
    pub events_by_category: HashMap<String, u64>,
    pub events_dropped: u64,
    pub events_buffered: u64,
    pub events_flushed: u64,
    pub avg_emission_time_ns: u64,
    pub max_emission_time_ns: u64,
    pub buffer_overflows: u64,
    pub rate_limit_hits: u64,
}

/// Event buffer entry
#[derive(Debug, Clone)]
struct BufferedEvent {
    event: SemanticEvent,
    emitted_at: SystemTime,
    source_layer: String,
}

/// Rate limiter for event emission
#[derive(Debug)]
struct RateLimiter {
    max_events_per_second: u32,
    current_count: AtomicU64,
    window_start: Mutex<SystemTime>,
}

impl RateLimiter {
    fn new(max_events_per_second: u32) -> Self {
        Self {
            max_events_per_second,
            current_count: AtomicU64::new(0),
            window_start: Mutex::new(SystemTime::now()),
        }
    }

    fn check_rate_limit(&self) -> bool {
        let now = SystemTime::now();
        let mut window_start = self.window_start.lock().unwrap();
        
        // Reset window if more than 1 second has passed
        if now.duration_since(*window_start).unwrap_or_default().as_secs() >= 1 {
            *window_start = now;
            self.current_count.store(0, Ordering::Relaxed);
        }
        
        let current = self.current_count.fetch_add(1, Ordering::Relaxed);
        current < self.max_events_per_second as u64
    }
}

/// Main event emission framework
pub struct EventEmissionFramework {
    config: Arc<RwLock<EventEmissionConfig>>,
    stats: Arc<RwLock<EventEmissionStats>>,
    event_buffer: Arc<Mutex<VecDeque<BufferedEvent>>>,
    rate_limiter: Arc<RateLimiter>,
    sequence_counter: AtomicU64,
    global_sequence: AtomicU64,
    running: AtomicBool,
    
    // Background task handles
    flush_handle: Option<thread::JoinHandle<()>>,
    
    // Event sinks
    event_sender: Option<mpsc::UnboundedSender<SemanticEvent>>,
    
    // Integration with cross-layer framework
    integration_framework: Option<Arc<CrossLayerIntegrationFramework>>,
}

impl EventEmissionFramework {
    /// Create a new event emission framework
    pub fn new(config: EventEmissionConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(config.max_events_per_second));
        
        Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(EventEmissionStats::default())),
            event_buffer: Arc::new(Mutex::new(VecDeque::new())),
            rate_limiter,
            sequence_counter: AtomicU64::new(0),
            global_sequence: AtomicU64::new(0),
            running: AtomicBool::new(false),
            flush_handle: None,
            event_sender: None,
            integration_framework: None,
        }
    }

    /// Start the event emission framework
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.running.store(true, Ordering::Relaxed);
        
        // Create event channel
        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.event_sender = Some(sender);
        
        // Start background flush task
        let config = Arc::clone(&self.config);
        let stats = Arc::clone(&self.stats);
        let event_buffer = Arc::clone(&self.event_buffer);
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        
        self.flush_handle = Some(thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut interval = tokio::time::interval(
                    std::time::Duration::from_millis(
                        config.read().unwrap().flush_interval_ms
                    )
                );
                
                while running_clone.load(Ordering::Relaxed) {
                    tokio::select! {
                        _ = interval.tick() => {
                            Self::flush_events_batch(&event_buffer, &stats).await;
                        }
                        event = receiver.recv() => {
                            if let Some(event) = event {
                                Self::process_event(event, &event_buffer, &stats).await;
                            }
                        }
                    }
                }
            });
        }));
        
        info!("Event emission framework started");
        Ok(())
    }

    /// Stop the event emission framework
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.running.store(false, Ordering::Relaxed);
        
        // Close event sender
        self.event_sender = None;
        
        // Wait for flush task to complete
        if let Some(handle) = self.flush_handle.take() {
            handle.join().map_err(|_| "Failed to join flush thread")?;
        }
        
        // Final flush
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            Self::flush_events_batch(&self.event_buffer, &self.stats).await;
        });
        
        info!("Event emission framework stopped");
        Ok(())
    }

    /// Set integration framework for cross-layer coordination
    pub fn set_integration_framework(&mut self, framework: Arc<CrossLayerIntegrationFramework>) {
        self.integration_framework = Some(framework);
    }

    /// Emit a semantic event
    #[instrument(skip(self, event_type, context))]
    pub fn emit_event(
        &self,
        event_type: SemanticEventType,
        context: SemanticContext,
        flags: EventFlags,
        priority: EventPriority,
        payload: Option<serde_json::Value>,
        metadata: Option<serde_json::Value>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        
        // Check if emission is enabled
        if !config.enabled {
            return Ok(0);
        }
        
        // Check category-specific enablement
        if !self.is_category_enabled(&event_type, &config) {
            return Ok(0);
        }
        
        // Check rate limit
        if !self.rate_limiter.check_rate_limit() {
            let mut stats = self.stats.write().unwrap();
            stats.rate_limit_hits += 1;
            return Err("Rate limit exceeded".into());
        }
        
        drop(config);
        
        // Generate event ID and timestamps
        let event_id = self.global_sequence.fetch_add(1, Ordering::Relaxed);
        let local_sequence = self.sequence_counter.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now();
        
        let timestamp = SemanticTimestamp {
            timestamp: chrono::DateTime::from(now),
            sequence: local_sequence,
            cpu_id: 0, // TODO: Get actual CPU ID
            process_id: std::process::id(),
        };
        
        // Create semantic event
        let event = SemanticEvent {
            event_id,
            event_type,
            event_subtype: None,
            timestamp,
            global_sequence: event_id,
            local_sequence,
            flags,
            priority,
            event_size: 0, // Will be calculated during serialization
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFFFFFFFFFFFFFFFF, // Visible to all agents by default
            agent_relevance_score: 100,
            replay_priority: priority as u32,
            context,
            payload,
            metadata,
        };
        
        // Send event for processing
        if let Some(sender) = &self.event_sender {
            if let Err(_) = sender.send(event) {
                warn!("Failed to send event to processing queue");
                let mut stats = self.stats.write().unwrap();
                stats.events_dropped += 1;
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_events_emitted += 1;
            
            let type_name = format!("{:?}", event_type);
            *stats.events_by_type.entry(type_name).or_insert(0) += 1;
            
            let category_name = format!("{:?}", event_type.category());
            *stats.events_by_category.entry(category_name).or_insert(0) += 1;
        }
        
        trace!("Emitted event {} of type {:?}", event_id, event_type);
        Ok(event_id)
    }

    /// Emit a filesystem event
    pub fn emit_filesystem_event(
        &self,
        event_type: SemanticEventType,
        path: String,
        inode_number: Option<u64>,
        file_type: Option<String>,
        flags: EventFlags,
        priority: EventPriority,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let context = SemanticContext {
            transaction_id: None,
            session_id: None,
            causality_chain_id: None,
            filesystem: Some(FilesystemContext {
                path,
                inode_number,
                file_type,
            }),
            graph: None,
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: None,
        };
        
        self.emit_event(event_type, context, flags, priority, None, None)
    }

    /// Emit a graph event
    pub fn emit_graph_event(
        &self,
        event_type: SemanticEventType,
        node_id: Option<u64>,
        edge_id: Option<u64>,
        operation_type: Option<u32>,
        flags: EventFlags,
        priority: EventPriority,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let context = SemanticContext {
            transaction_id: None,
            session_id: None,
            causality_chain_id: None,
            filesystem: None,
            graph: Some(GraphContext {
                node_id,
                edge_id,
                operation_type,
            }),
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: None,
        };
        
        self.emit_event(event_type, context, flags, priority, None, None)
    }

    /// Emit a vector event
    pub fn emit_vector_event(
        &self,
        event_type: SemanticEventType,
        vector_id: Option<u64>,
        dimensions: Option<u32>,
        element_type: Option<u32>,
        flags: EventFlags,
        priority: EventPriority,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let context = SemanticContext {
            transaction_id: None,
            session_id: None,
            causality_chain_id: None,
            filesystem: None,
            graph: None,
            vector: Some(VectorContext {
                vector_id,
                dimensions,
                element_type,
            }),
            agent: None,
            system: None,
            semantic: None,
            observability: None,
        };
        
        self.emit_event(event_type, context, flags, priority, None, None)
    }

    /// Get emission statistics
    pub fn get_stats(&self) -> EventEmissionStats {
        self.stats.read().unwrap().clone()
    }

    /// Reset emission statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        *stats = EventEmissionStats::default();
    }

    // Helper methods
    fn is_category_enabled(&self, event_type: &SemanticEventType, config: &EventEmissionConfig) -> bool {
        use crate::semantic_api::types::EventCategory;
        
        match event_type.category() {
            EventCategory::Filesystem => config.enable_kernel_events,
            EventCategory::Graph => config.enable_graph_events,
            EventCategory::Vector => config.enable_vector_events,
            EventCategory::Agent => config.enable_agent_events,
            EventCategory::System => config.enable_system_events,
            EventCategory::Semantic => config.enable_semantic_events,
            EventCategory::Observability => config.enable_observability_events,
            EventCategory::Unknown => false,
        }
    }

    async fn process_event(
        event: SemanticEvent,
        event_buffer: &Arc<Mutex<VecDeque<BufferedEvent>>>,
        stats: &Arc<RwLock<EventEmissionStats>>,
    ) {
        let buffered_event = BufferedEvent {
            event,
            emitted_at: SystemTime::now(),
            source_layer: "unknown".to_string(), // TODO: Determine source layer
        };
        
        // Add to buffer
        {
            let mut buffer = event_buffer.lock().unwrap();
            buffer.push_back(buffered_event);
            
            let mut stats_guard = stats.write().unwrap();
            stats_guard.events_buffered += 1;
            
            // Check for buffer overflow
            let config_buffer_size = 10000; // TODO: Get from config
            if buffer.len() > config_buffer_size {
                buffer.pop_front();
                stats_guard.buffer_overflows += 1;
                stats_guard.events_dropped += 1;
            }
        }
    }

    async fn flush_events_batch(
        event_buffer: &Arc<Mutex<VecDeque<BufferedEvent>>>,
        stats: &Arc<RwLock<EventEmissionStats>>,
    ) {
        let events_to_flush = {
            let mut buffer = event_buffer.lock().unwrap();
            let batch_size = std::cmp::min(buffer.len(), 100); // TODO: Get from config
            buffer.drain(..batch_size).collect::<Vec<_>>()
        };
        
        if events_to_flush.is_empty() {
            return;
        }
        
        // TODO: Serialize and store events to journal
        // For now, just log them
        debug!("Flushing {} events to journal", events_to_flush.len());
        
        // Update statistics
        {
            let mut stats_guard = stats.write().unwrap();
            stats_guard.events_flushed += events_to_flush.len() as u64;
        }
    }
}

/// Global event emission framework instance
static mut GLOBAL_EMISSION_FRAMEWORK: Option<Arc<Mutex<EventEmissionFramework>>> = None;
static INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Initialize the global event emission framework
pub fn initialize_event_emission(config: EventEmissionConfig) -> Result<(), Box<dyn std::error::Error>> {
    INIT_ONCE.call_once(|| {
        let framework = EventEmissionFramework::new(config);
        unsafe {
            GLOBAL_EMISSION_FRAMEWORK = Some(Arc::new(Mutex::new(framework)));
        }
    });
    
    // Start the framework
    unsafe {
        if let Some(framework) = &GLOBAL_EMISSION_FRAMEWORK {
            framework.lock().unwrap().start()?;
        }
    }
    
    Ok(())
}

/// Get the global event emission framework
pub fn get_global_emission_framework() -> Option<Arc<Mutex<EventEmissionFramework>>> {
    unsafe { GLOBAL_EMISSION_FRAMEWORK.clone() }
}

/// Shutdown the global event emission framework
pub fn shutdown_event_emission() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if let Some(framework) = &GLOBAL_EMISSION_FRAMEWORK {
            framework.lock().unwrap().stop()?;
        }
    }
    Ok(())
}

/// Convenience function to emit a filesystem event
pub fn emit_filesystem_event(
    event_type: SemanticEventType,
    path: String,
    inode_number: Option<u64>,
    file_type: Option<String>,
) -> Result<u64, Box<dyn std::error::Error>> {
    if let Some(framework) = get_global_emission_framework() {
        let flags = EventFlags {
            atomic: false,
            transactional: false,
            causal: true,
            agent_visible: true,
            deterministic: true,
            compressed: false,
            indexed: true,
            replicated: false,
        };
        
        framework.lock().unwrap().emit_filesystem_event(
            event_type,
            path,
            inode_number,
            file_type,
            flags,
            EventPriority::Normal,
        )
    } else {
        Err("Event emission framework not initialized".into())
    }
}

/// Convenience function to emit a graph event
pub fn emit_graph_event(
    event_type: SemanticEventType,
    node_id: Option<u64>,
    edge_id: Option<u64>,
    operation_type: Option<u32>,
) -> Result<u64, Box<dyn std::error::Error>> {
    if let Some(framework) = get_global_emission_framework() {
        let flags = EventFlags {
            atomic: false,
            transactional: false,
            causal: true,
            agent_visible: true,
            deterministic: true,
            compressed: false,
            indexed: true,
            replicated: false,
        };
        
        framework.lock().unwrap().emit_graph_event(
            event_type,
            node_id,
            edge_id,
            operation_type,
            flags,
            EventPriority::Normal,
        )
    } else {
        Err("Event emission framework not initialized".into())
    }
}

/// Convenience function to emit a vector event
pub fn emit_vector_event(
    event_type: SemanticEventType,
    vector_id: Option<u64>,
    dimensions: Option<u32>,
    element_type: Option<u32>,
) -> Result<u64, Box<dyn std::error::Error>> {
    if let Some(framework) = get_global_emission_framework() {
        let flags = EventFlags {
            atomic: false,
            transactional: false,
            causal: true,
            agent_visible: true,
            deterministic: true,
            compressed: false,
            indexed: true,
            replicated: false,
        };
        
        framework.lock().unwrap().emit_vector_event(
            event_type,
            vector_id,
            dimensions,
            element_type,
            flags,
            EventPriority::Normal,
        )
    } else {
        Err("Event emission framework not initialized".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_api::types::SemanticEventType;

    #[tokio::test]
    async fn test_event_emission_framework() {
        let config = EventEmissionConfig::default();
        let mut framework = EventEmissionFramework::new(config);
        
        framework.start().unwrap();
        
        // Test emitting a filesystem event
        let event_id = framework.emit_filesystem_event(
            SemanticEventType::FilesystemCreate,
            "/test/file.txt".to_string(),
            Some(12345),
            Some("regular".to_string()),
            EventFlags {
                atomic: false,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            EventPriority::Normal,
        ).unwrap();
        
        assert!(event_id > 0);
        
        // Wait a bit for processing
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        
        let stats = framework.get_stats();
        assert_eq!(stats.total_events_emitted, 1);
        
        framework.stop().unwrap();
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10);
        
        // Should allow first 10 events
        for _ in 0..10 {
            assert!(limiter.check_rate_limit());
        }
        
        // Should deny 11th event
        assert!(!limiter.check_rate_limit());
    }
}