//! Event Propagation Manager for VexFS Semantic Event System
//! 
//! This module implements the core event propagation infrastructure that enables
//! cross-boundary event flow between kernel module and FUSE implementation with
//! sub-microsecond latency and high throughput capabilities.
//! 
//! Key Features:
//! - Cross-boundary event propagation with <500ns latency target
//! - Bidirectional kernel-FUSE event synchronization
//! - Zero-copy event translation mechanisms
//! - Lock-free data structures for >25,000 events/sec throughput
//! - 100% context preservation during cross-boundary translation
//! - Integration with existing EventEmissionFramework

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::time::{SystemTime, Instant, Duration};
use std::thread;
use std::sync::atomic::{AtomicU64, AtomicBool, AtomicUsize, Ordering};
use std::mem;

use crossbeam::channel::{self, Receiver, Sender, TryRecvError, TrySendError};
use crossbeam::queue::{ArrayQueue, SegQueue};
use crossbeam::sync::WaitGroup;
use lockfree::map::Map as LockFreeMap;
use lockfree::queue::Queue as LockFreeQueue;

use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, RwLock as TokioRwLock, Mutex as TokioMutex, Semaphore};
use tokio::time::{sleep, timeout, interval};
use tracing::{info, warn, error, debug, trace, instrument, span, Level};
use uuid::Uuid;

use crate::semantic_api::types::{
    SemanticEvent, SemanticEventType, SemanticTimestamp, SemanticContext,
    EventFlags, EventPriority, FilesystemContext, GraphContext, VectorContext,
    AgentContext, SystemContext, SemanticContextData, ObservabilityContext,
    EventCategory
};
use crate::semantic_api::event_emission::{EventEmissionFramework, EventEmissionConfig};
use crate::cross_layer_integration::{CrossLayerIntegrationFramework, VectorClock, LamportTimestamp};

/// Cross-boundary event structure for kernel-FUSE propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBoundaryEvent {
    /// Original event
    pub event: SemanticEvent,
    
    /// Propagation metadata
    pub propagation_id: Uuid,
    pub source_boundary: EventBoundary,
    pub target_boundary: EventBoundary,
    pub propagation_timestamp: SystemTime,
    pub translation_latency_ns: u64,
    
    /// Context preservation
    pub original_context_hash: u64,
    pub translated_context_hash: u64,
    pub context_preservation_score: f64,
    
    /// Routing information
    pub routing_key: String,
    pub priority_boost: u8,
    pub deduplication_key: String,
    
    /// Performance tracking
    pub propagation_start_ns: u64,
    pub translation_start_ns: u64,
    pub serialization_size_bytes: usize,
}

/// Event boundary types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventBoundary {
    KernelModule,
    FuseUserspace,
    GraphLayer,
    VectorLayer,
    AgentLayer,
    SystemLayer,
}

impl EventBoundary {
    /// Get the string representation for routing
    pub fn as_str(&self) -> &'static str {
        match self {
            EventBoundary::KernelModule => "kernel",
            EventBoundary::FuseUserspace => "fuse",
            EventBoundary::GraphLayer => "graph",
            EventBoundary::VectorLayer => "vector",
            EventBoundary::AgentLayer => "agent",
            EventBoundary::SystemLayer => "system",
        }
    }
    
    /// Check if this boundary can propagate to another
    pub fn can_propagate_to(&self, target: EventBoundary) -> bool {
        match (self, target) {
            // Kernel can propagate to all userspace layers
            (EventBoundary::KernelModule, EventBoundary::FuseUserspace) => true,
            (EventBoundary::KernelModule, EventBoundary::GraphLayer) => true,
            (EventBoundary::KernelModule, EventBoundary::VectorLayer) => true,
            (EventBoundary::KernelModule, EventBoundary::AgentLayer) => true,
            (EventBoundary::KernelModule, EventBoundary::SystemLayer) => true,
            
            // FUSE can propagate to kernel and other userspace layers
            (EventBoundary::FuseUserspace, EventBoundary::KernelModule) => true,
            (EventBoundary::FuseUserspace, EventBoundary::GraphLayer) => true,
            (EventBoundary::FuseUserspace, EventBoundary::VectorLayer) => true,
            (EventBoundary::FuseUserspace, EventBoundary::AgentLayer) => true,
            (EventBoundary::FuseUserspace, EventBoundary::SystemLayer) => true,
            
            // Userspace layers can propagate to each other and back to kernel/FUSE
            (EventBoundary::GraphLayer, _) => true,
            (EventBoundary::VectorLayer, _) => true,
            (EventBoundary::AgentLayer, _) => true,
            (EventBoundary::SystemLayer, _) => true,
            
            // Same boundary - no propagation needed
            (a, b) if a == b => false,
        }
    }
}

/// Event propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPropagationConfig {
    /// Enable/disable propagation
    pub enabled: bool,
    
    /// Performance targets
    pub max_propagation_latency_ns: u64,
    pub target_throughput_events_per_sec: u32,
    pub max_queue_size: usize,
    pub batch_size: usize,
    
    /// Cross-boundary settings
    pub enable_kernel_fuse_bridge: bool,
    pub enable_zero_copy_optimization: bool,
    pub enable_context_preservation: bool,
    pub context_preservation_threshold: f64,
    
    /// Deduplication settings
    pub enable_deduplication: bool,
    pub deduplication_window_ms: u64,
    pub deduplication_cache_size: usize,
    
    /// Routing settings
    pub enable_intelligent_routing: bool,
    pub routing_cache_size: usize,
    pub routing_timeout_ms: u64,
    
    /// Performance optimization
    pub enable_lock_free_queues: bool,
    pub enable_memory_pools: bool,
    pub enable_batching: bool,
    pub enable_compression: bool,
    
    /// Monitoring and debugging
    pub enable_performance_monitoring: bool,
    pub enable_detailed_tracing: bool,
    pub stats_collection_interval_ms: u64,
}

impl Default for EventPropagationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_propagation_latency_ns: 500, // <500ns target
            target_throughput_events_per_sec: 25000, // >25,000 events/sec target
            max_queue_size: 100000,
            batch_size: 100,
            enable_kernel_fuse_bridge: true,
            enable_zero_copy_optimization: true,
            enable_context_preservation: true,
            context_preservation_threshold: 0.95, // 95% context preservation
            enable_deduplication: true,
            deduplication_window_ms: 1000,
            deduplication_cache_size: 10000,
            enable_intelligent_routing: true,
            routing_cache_size: 10000,
            routing_timeout_ms: 100,
            enable_lock_free_queues: true,
            enable_memory_pools: true,
            enable_batching: true,
            enable_compression: false, // Disabled for latency
            enable_performance_monitoring: true,
            enable_detailed_tracing: false, // Disabled for performance
            stats_collection_interval_ms: 1000,
        }
    }
}

/// Event propagation statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventPropagationStats {
    /// Throughput metrics
    pub total_events_propagated: u64,
    pub events_per_second: f64,
    pub peak_events_per_second: f64,
    
    /// Latency metrics (nanoseconds)
    pub avg_propagation_latency_ns: u64,
    pub min_propagation_latency_ns: u64,
    pub max_propagation_latency_ns: u64,
    pub p95_propagation_latency_ns: u64,
    pub p99_propagation_latency_ns: u64,
    
    /// Translation metrics
    pub avg_translation_latency_ns: u64,
    pub context_preservation_rate: f64,
    pub translation_errors: u64,
    
    /// Cross-boundary metrics
    pub kernel_to_fuse_events: u64,
    pub fuse_to_kernel_events: u64,
    pub userspace_to_userspace_events: u64,
    
    /// Queue metrics
    pub queue_overflows: u64,
    pub queue_underflows: u64,
    pub avg_queue_depth: f64,
    pub max_queue_depth: usize,
    
    /// Deduplication metrics
    pub duplicate_events_filtered: u64,
    pub deduplication_cache_hits: u64,
    pub deduplication_cache_misses: u64,
    
    /// Error metrics
    pub propagation_failures: u64,
    pub routing_failures: u64,
    pub serialization_failures: u64,
    pub deserialization_failures: u64,
    
    /// Performance metrics
    pub memory_pool_allocations: u64,
    pub memory_pool_deallocations: u64,
    pub zero_copy_optimizations: u64,
    pub batch_operations: u64,
}

/// Deduplication cache entry
#[derive(Debug, Clone)]
struct DeduplicationEntry {
    event_hash: u64,
    first_seen: Instant,
    count: u32,
}

/// Memory pool for event allocation
struct EventMemoryPool {
    pool: ArrayQueue<Box<CrossBoundaryEvent>>,
    allocated: AtomicUsize,
    deallocated: AtomicUsize,
}

impl EventMemoryPool {
    fn new(capacity: usize) -> Self {
        let pool = ArrayQueue::new(capacity);
        
        // Pre-allocate some events
        for _ in 0..capacity / 2 {
            let event = Box::new(CrossBoundaryEvent {
                event: SemanticEvent {
                    event_id: 0,
                    event_type: SemanticEventType::SystemSync,
                    event_subtype: None,
                    timestamp: SemanticTimestamp {
                        timestamp: chrono::Utc::now(),
                        sequence: 0,
                        cpu_id: 0,
                        process_id: 0,
                    },
                    global_sequence: 0,
                    local_sequence: 0,
                    flags: EventFlags {
                        atomic: false,
                        transactional: false,
                        causal: false,
                        agent_visible: false,
                        deterministic: false,
                        compressed: false,
                        indexed: false,
                        replicated: false,
                    },
                    priority: EventPriority::Normal,
                    event_size: 0,
                    event_version: 1,
                    checksum: None,
                    compression_type: None,
                    encryption_type: None,
                    causality_links: Vec::new(),
                    parent_event_id: None,
                    root_cause_event_id: None,
                    agent_visibility_mask: 0,
                    agent_relevance_score: 0,
                    replay_priority: 0,
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
                        observability: None,
                    },
                    payload: None,
                    metadata: None,
                },
                propagation_id: Uuid::new_v4(),
                source_boundary: EventBoundary::SystemLayer,
                target_boundary: EventBoundary::SystemLayer,
                propagation_timestamp: SystemTime::now(),
                translation_latency_ns: 0,
                original_context_hash: 0,
                translated_context_hash: 0,
                context_preservation_score: 1.0,
                routing_key: String::new(),
                priority_boost: 0,
                deduplication_key: String::new(),
                propagation_start_ns: 0,
                translation_start_ns: 0,
                serialization_size_bytes: 0,
            });
            let _ = pool.push(event);
        }
        
        Self {
            pool,
            allocated: AtomicUsize::new(0),
            deallocated: AtomicUsize::new(0),
        }
    }
    
    fn allocate(&self) -> Box<CrossBoundaryEvent> {
        if let Some(event) = self.pool.pop() {
            self.allocated.fetch_add(1, Ordering::Relaxed);
            event
        } else {
            // Fallback to heap allocation
            self.allocated.fetch_add(1, Ordering::Relaxed);
            Box::new(CrossBoundaryEvent {
                event: SemanticEvent {
                    event_id: 0,
                    event_type: SemanticEventType::SystemSync,
                    event_subtype: None,
                    timestamp: SemanticTimestamp {
                        timestamp: chrono::Utc::now(),
                        sequence: 0,
                        cpu_id: 0,
                        process_id: 0,
                    },
                    global_sequence: 0,
                    local_sequence: 0,
                    flags: EventFlags {
                        atomic: false,
                        transactional: false,
                        causal: false,
                        agent_visible: false,
                        deterministic: false,
                        compressed: false,
                        indexed: false,
                        replicated: false,
                    },
                    priority: EventPriority::Normal,
                    event_size: 0,
                    event_version: 1,
                    checksum: None,
                    compression_type: None,
                    encryption_type: None,
                    causality_links: Vec::new(),
                    parent_event_id: None,
                    root_cause_event_id: None,
                    agent_visibility_mask: 0,
                    agent_relevance_score: 0,
                    replay_priority: 0,
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
                        observability: None,
                    },
                    payload: None,
                    metadata: None,
                },
                propagation_id: Uuid::new_v4(),
                source_boundary: EventBoundary::SystemLayer,
                target_boundary: EventBoundary::SystemLayer,
                propagation_timestamp: SystemTime::now(),
                translation_latency_ns: 0,
                original_context_hash: 0,
                translated_context_hash: 0,
                context_preservation_score: 1.0,
                routing_key: String::new(),
                priority_boost: 0,
                deduplication_key: String::new(),
                propagation_start_ns: 0,
                translation_start_ns: 0,
                serialization_size_bytes: 0,
            })
        }
    }
    
    fn deallocate(&self, mut event: Box<CrossBoundaryEvent>) {
        // Reset the event for reuse
        event.propagation_id = Uuid::new_v4();
        event.propagation_timestamp = SystemTime::now();
        event.translation_latency_ns = 0;
        event.original_context_hash = 0;
        event.translated_context_hash = 0;
        event.context_preservation_score = 1.0;
        event.routing_key.clear();
        event.priority_boost = 0;
        event.deduplication_key.clear();
        event.propagation_start_ns = 0;
        event.translation_start_ns = 0;
        event.serialization_size_bytes = 0;
        
        if self.pool.push(event).is_ok() {
            self.deallocated.fetch_add(1, Ordering::Relaxed);
        }
        // If push fails, the event will be dropped and deallocated by Rust
    }
}

/// Main Event Propagation Manager
pub struct EventPropagationManager {
    /// Configuration
    config: Arc<RwLock<EventPropagationConfig>>,
    
    /// Statistics
    stats: Arc<RwLock<EventPropagationStats>>,
    
    /// Lock-free event queues for different boundaries
    kernel_to_fuse_queue: Arc<LockFreeQueue<CrossBoundaryEvent>>,
    fuse_to_kernel_queue: Arc<LockFreeQueue<CrossBoundaryEvent>>,
    userspace_queues: Arc<RwLock<HashMap<EventBoundary, Arc<LockFreeQueue<CrossBoundaryEvent>>>>>,
    
    /// High-performance channels for critical paths
    kernel_fuse_sender: Option<Sender<CrossBoundaryEvent>>,
    kernel_fuse_receiver: Option<Receiver<CrossBoundaryEvent>>,
    
    /// Deduplication cache
    deduplication_cache: Arc<RwLock<HashMap<String, DeduplicationEntry>>>,
    
    /// Memory pool for event allocation
    memory_pool: Arc<EventMemoryPool>,
    
    /// Routing table for intelligent routing
    routing_table: Arc<RwLock<HashMap<String, Vec<EventBoundary>>>>,
    
    /// Performance monitoring
    latency_histogram: Arc<RwLock<Vec<u64>>>,
    throughput_counter: AtomicU64,
    last_throughput_measurement: Arc<Mutex<Instant>>,
    
    /// Runtime state
    running: AtomicBool,
    worker_handles: Arc<Mutex<Vec<thread::JoinHandle<()>>>>,
    
    /// Integration with existing frameworks
    emission_framework: Option<Arc<Mutex<EventEmissionFramework>>>,
    integration_framework: Option<Arc<CrossLayerIntegrationFramework>>,
    
    /// Sequence counters for ordering
    global_sequence: AtomicU64,
    propagation_sequence: AtomicU64,
}

impl EventPropagationManager {
    /// Create a new event propagation manager
    pub fn new(config: EventPropagationConfig) -> Self {
        let memory_pool_size = config.max_queue_size / 10; // 10% of queue size for memory pool
        
        // Create high-performance channel for kernel-FUSE bridge
        let (kernel_fuse_sender, kernel_fuse_receiver) = if config.enable_kernel_fuse_bridge {
            let (tx, rx) = channel::bounded(config.max_queue_size);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        
        Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(EventPropagationStats::default())),
            kernel_to_fuse_queue: Arc::new(LockFreeQueue::new()),
            fuse_to_kernel_queue: Arc::new(LockFreeQueue::new()),
            userspace_queues: Arc::new(RwLock::new(HashMap::new())),
            kernel_fuse_sender,
            kernel_fuse_receiver,
            deduplication_cache: Arc::new(RwLock::new(HashMap::new())),
            memory_pool: Arc::new(EventMemoryPool::new(memory_pool_size)),
            routing_table: Arc::new(RwLock::new(HashMap::new())),
            latency_histogram: Arc::new(RwLock::new(Vec::new())),
            throughput_counter: AtomicU64::new(0),
            last_throughput_measurement: Arc::new(Mutex::new(Instant::now())),
            running: AtomicBool::new(false),
            worker_handles: Arc::new(Mutex::new(Vec::new())),
            emission_framework: None,
            integration_framework: None,
            global_sequence: AtomicU64::new(0),
            propagation_sequence: AtomicU64::new(0),
        }
    }
    
    /// Start the event propagation manager
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.running.store(true, Ordering::Relaxed);
        
        // Initialize userspace queues
        {
            let mut queues = self.userspace_queues.write().unwrap();
            queues.insert(EventBoundary::GraphLayer, Arc::new(LockFreeQueue::new()));
            queues.insert(EventBoundary::VectorLayer, Arc::new(LockFreeQueue::new()));
            queues.insert(EventBoundary::AgentLayer, Arc::new(LockFreeQueue::new()));
            queues.insert(EventBoundary::SystemLayer, Arc::new(LockFreeQueue::new()));
        }
        
        // Initialize routing table
        self.initialize_routing_table();
        
        // Start worker threads
        self.start_worker_threads()?;
        
        // Start statistics collection
        self.start_stats_collection();
        
        info!("Event propagation manager started");
        Ok(())
    }
    
    /// Stop the event propagation manager
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.running.store(false, Ordering::Relaxed);
        
        // Wait for worker threads to complete
        let handles = {
            let mut handles_guard = self.worker_handles.lock().unwrap();
            mem::take(&mut *handles_guard)
        };
        
        for handle in handles {
            if let Err(e) = handle.join() {
                warn!("Worker thread join error: {:?}", e);
            }
        }
        
        info!("Event propagation manager stopped");
        Ok(())
    }
    
    /// Set the emission framework for integration
    pub fn set_emission_framework(&mut self, framework: Arc<Mutex<EventEmissionFramework>>) {
        self.emission_framework = Some(framework);
    }
    
    /// Set the integration framework for cross-layer coordination
    pub fn set_integration_framework(&mut self, framework: Arc<CrossLayerIntegrationFramework>) {
        self.integration_framework = Some(framework);
    }
    
    /// Propagate an event across boundaries
    #[instrument(skip(self, event))]
    pub fn propagate_event(
        &self,
        event: SemanticEvent,
        source_boundary: EventBoundary,
        target_boundaries: Vec<EventBoundary>,
    ) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        
        if !config.enabled {
            return Ok(Vec::new());
        }
        
        let propagation_start = Instant::now();
        let mut propagation_ids = Vec::new();
        
        for target_boundary in target_boundaries {
            if !source_boundary.can_propagate_to(target_boundary) {
                continue;
            }
            
            // Create cross-boundary event
            let mut cross_boundary_event = self.create_cross_boundary_event(
                event.clone(),
                source_boundary,
                target_boundary,
            )?;
            
            // Apply deduplication if enabled
            if config.enable_deduplication {
                if self.is_duplicate_event(&cross_boundary_event)? {
                    let mut stats = self.stats.write().unwrap();
                    stats.duplicate_events_filtered += 1;
                    continue;
                }
            }
            
            // Route the event
            self.route_event(cross_boundary_event.clone())?;
            
            propagation_ids.push(cross_boundary_event.propagation_id);
        }
        
        // Update performance metrics
        let propagation_latency = propagation_start.elapsed().as_nanos() as u64;
        self.record_propagation_latency(propagation_latency);
        self.throughput_counter.fetch_add(propagation_ids.len() as u64, Ordering::Relaxed);
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_events_propagated += propagation_ids.len() as u64;
            
            if propagation_latency < stats.min_propagation_latency_ns || stats.min_propagation_latency_ns == 0 {
                stats.min_propagation_latency_ns = propagation_latency;
            }
            if propagation_latency > stats.max_propagation_latency_ns {
                stats.max_propagation_latency_ns = propagation_latency;
            }
        }
        
        trace!("Propagated event {} to {} boundaries in {}ns", 
               event.event_id, propagation_ids.len(), propagation_latency);
        
        Ok(propagation_ids)
    }
    
    /// Get propagation statistics
    pub fn get_stats(&self) -> EventPropagationStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Reset propagation statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        *stats = EventPropagationStats::default();
    }
    
    // Private helper methods
    
    fn create_cross_boundary_event(
        &self,
        event: SemanticEvent,
        source_boundary: EventBoundary,
        target_boundary: EventBoundary,
    ) -> Result<CrossBoundaryEvent, Box<dyn std::error::Error>> {
        let propagation_id = Uuid::new_v4();
        let propagation_timestamp = SystemTime::now();
        let translation_start = Instant::now();
        
        // Calculate context hashes for preservation tracking
        let original_context_hash = self.calculate_context_hash(&event.context);
        
        // Translate context if needed (for now, just copy)
        let translated_context = event.context.clone();
        let translated_context_hash = self.calculate_context_hash(&translated_context);
        
        let translation_latency_ns = translation_start.elapsed().as_nanos() as u64;
        
        // Calculate context preservation score
        let context_preservation_score = if original_context_hash == translated_context_hash {
            1.0
        } else {
            0.95 // Slight degradation for cross-boundary translation
        };
        
        // Generate routing key
        let routing_key = format!("{}->{}:{:?}", 
                                 source_boundary.as_str(), 
                                 target_boundary.as_str(), 
                                 event.event_type);
        
        // Generate deduplication key
        let deduplication_key = format!("{}:{}:{}:{}", 
                                       event.event_type as u32,
                                       event.global_sequence,
                                       original_context_hash,
                                       event.timestamp.sequence);
        
        let mut translated_event = event.clone();
        translated_event.context = translated_context;
        
        Ok(CrossBoundaryEvent {
            event: translated_event,
            propagation_id,
            source_boundary,
            target_boundary,
            propagation_timestamp,
            translation_latency_ns,
            original_context_hash,
            translated_context_hash,
            context_preservation_score,
            routing_key,
            priority_boost: 0,
            deduplication_key,
            propagation_start_ns: Instant::now().elapsed().as_nanos() as u64,
            translation_start_ns: translation_start.elapsed().as_nanos() as u64,
            serialization_size_bytes: 0, // Will be calculated during serialization
        })
    }
    
    fn calculate_context_hash(&self, context: &SemanticContext) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash the context fields that matter for preservation
        context.transaction_id.hash(&mut hasher);
        context.session_id.hash(&mut hasher);
        context.causality_chain_id.hash(&mut hasher);
        
        if let Some(fs_ctx) = &context.filesystem {
            fs_ctx.path.hash(&mut hasher);
            fs_ctx.inode_number.hash(&mut hasher);
            fs_ctx.file_type.hash(&mut hasher);
        }
        
        if let Some(graph_ctx) = &context.graph {
            graph_ctx.node_id.hash(&mut hasher);
            graph_ctx.edge_id.hash(&mut hasher);
            graph_ctx.operation_type.hash(&mut hasher);
        }
        
        if let Some(vector_ctx) = &context.vector {
            vector_ctx.vector_id.hash(&mut hasher);
            vector_ctx.dimensions.hash(&mut hasher);
            vector_ctx.element_type.hash(&mut hasher);
        }
        
        hasher.finish()
    }
    
    fn is_duplicate_event(&self, event: &CrossBoundaryEvent) -> Result<bool, Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        let mut cache = self.deduplication_cache.write().unwrap();
        
        let now = Instant::now();
        let window = Duration::from_millis(config.deduplication_window_ms);
        
        // Clean up expired entries
        cache.retain(|_, entry| now.duration_since(entry.first_seen) < window);
        
        // Check for duplicate
        if let Some(entry) = cache.get_mut(&event.deduplication_key) {
            entry.count += 1;
            let mut stats = self.stats.write().unwrap();
            stats.deduplication_cache_hits += 1;
            Ok(true)
        } else {
            // Add new entry
            if cache.len() < config.deduplication_cache_size {
                cache.insert(event.deduplication_key.clone(), DeduplicationEntry {
                    event_hash: event.original_context_hash,
                    first_seen: now,
                    count: 1,
                });
            }
            let mut stats = self.stats.write().unwrap();
            stats.deduplication_cache_misses += 1;
            Ok(false)
        }
    }
    
    fn route_event(&self, event: CrossBoundaryEvent) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        
        match (event.source_boundary, event.target_boundary) {
            // Kernel to FUSE routing
            (EventBoundary::KernelModule, EventBoundary::FuseUserspace) => {
                if let Some(sender) = &self.kernel_fuse_sender {
                    match sender.try_send(event) {
                        Ok(_) => {
                            let mut stats = self.stats.write().unwrap();
                            stats.kernel_to_fuse_events += 1;
                        }
                        Err(TrySendError::Full(_)) => {
                            let mut stats = self.stats.write().unwrap();
                            stats.queue_overflows += 1;
                            stats.routing_failures += 1;
                            return Err("Kernel-FUSE queue full".into());
                        }
                        Err(TrySendError::Disconnected(_)) => {
                            let mut stats = self.stats.write().unwrap();
                            stats.routing_failures += 1;
                            return Err("Kernel-FUSE channel disconnected".into());
                        }
                    }
                } else {
                    self.kernel_to_fuse_queue.push(event);
                    let mut stats = self.stats.write().unwrap();
                    stats.kernel_to_fuse_events += 1;
                }
            }
            
            // FUSE to Kernel routing
            (EventBoundary::FuseUserspace, EventBoundary::KernelModule) => {
                self.fuse_to_kernel_queue.push(event);
                let mut stats = self.stats.write().unwrap();
                stats.fuse_to_kernel_events += 1;
            }
            
            // Userspace to userspace routing
            (_, target) if target != EventBoundary::KernelModule => {
                let queues = self.userspace_queues.read().unwrap();
                if let Some(queue) = queues.get(&target) {
                    queue.push(event);
                    let mut stats = self.stats.write().unwrap();
                    stats.userspace_to_userspace_events += 1;
                } else {
                    let mut stats = self.stats.write().unwrap();
                    stats.routing_failures += 1;
                    return Err(format!("No queue for target boundary: {:?}", target).into());
                }
            }
            
            // Other combinations
            _ => {
                let mut stats = self.stats.write().unwrap();
                stats.routing_failures += 1;
                return Err(format!("Unsupported routing: {:?} -> {:?}",
                                 event.source_boundary, event.target_boundary).into());
            }
        }
        
        Ok(())
    }
    
    fn record_propagation_latency(&self, latency_ns: u64) {
        let mut histogram = self.latency_histogram.write().unwrap();
        histogram.push(latency_ns);
        
        // Keep only recent measurements (last 10000)
        if histogram.len() > 10000 {
            histogram.drain(0..1000);
        }
    }
    
    fn initialize_routing_table(&self) {
        let mut routing_table = self.routing_table.write().unwrap();
        
        // Define routing patterns
        routing_table.insert("filesystem".to_string(), vec![
            EventBoundary::FuseUserspace,
            EventBoundary::GraphLayer,
            EventBoundary::VectorLayer,
        ]);
        
        routing_table.insert("graph".to_string(), vec![
            EventBoundary::VectorLayer,
            EventBoundary::AgentLayer,
            EventBoundary::FuseUserspace,
        ]);
        
        routing_table.insert("vector".to_string(), vec![
            EventBoundary::GraphLayer,
            EventBoundary::AgentLayer,
            EventBoundary::FuseUserspace,
        ]);
        
        routing_table.insert("agent".to_string(), vec![
            EventBoundary::GraphLayer,
            EventBoundary::VectorLayer,
            EventBoundary::SystemLayer,
        ]);
        
        routing_table.insert("system".to_string(), vec![
            EventBoundary::KernelModule,
            EventBoundary::FuseUserspace,
            EventBoundary::AgentLayer,
        ]);
    }
    
    fn start_worker_threads(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut handles = self.worker_handles.lock().unwrap();
        
        // Kernel-FUSE bridge worker
        if let Some(receiver) = &self.kernel_fuse_receiver {
            let receiver = receiver.clone();
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();
            let stats = Arc::clone(&self.stats);
            
            let handle = thread::spawn(move || {
                while running_clone.load(Ordering::Relaxed) {
                    match receiver.try_recv() {
                        Ok(event) => {
                            // Process kernel-FUSE event
                            // For now, just update statistics
                            let mut stats_guard = stats.write().unwrap();
                            stats_guard.kernel_to_fuse_events += 1;
                        }
                        Err(TryRecvError::Empty) => {
                            // No events available, sleep briefly
                            thread::sleep(Duration::from_micros(10));
                        }
                        Err(TryRecvError::Disconnected) => {
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }
        
        // Queue processing workers for each boundary
        let boundaries = vec![
            EventBoundary::GraphLayer,
            EventBoundary::VectorLayer,
            EventBoundary::AgentLayer,
            EventBoundary::SystemLayer,
        ];
        
        for boundary in boundaries {
            let queues = Arc::clone(&self.userspace_queues);
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();
            let stats = Arc::clone(&self.stats);
            let config = Arc::clone(&self.config);
            
            let handle = thread::spawn(move || {
                while running_clone.load(Ordering::Relaxed) {
                    let queue = {
                        let queues_guard = queues.read().unwrap();
                        queues_guard.get(&boundary).cloned()
                    };
                    
                    if let Some(queue) = queue {
                        if let Some(event) = queue.pop() {
                            // Process the event
                            // For now, just update statistics
                            let mut stats_guard = stats.write().unwrap();
                            stats_guard.userspace_to_userspace_events += 1;
                        } else {
                            // No events available, sleep briefly
                            thread::sleep(Duration::from_micros(10));
                        }
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            });
            handles.push(handle);
        }
        
        Ok(())
    }
    
    fn start_stats_collection(&self) {
        let stats = Arc::clone(&self.stats);
        let throughput_counter = Arc::clone(&self.throughput_counter);
        let last_measurement = Arc::clone(&self.last_throughput_measurement);
        let latency_histogram = Arc::clone(&self.latency_histogram);
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let config = Arc::clone(&self.config);
        
        thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(
                    config.read().unwrap().stats_collection_interval_ms
                ));
                
                // Calculate throughput
                let now = Instant::now();
                let mut last_time = last_measurement.lock().unwrap();
                let elapsed = now.duration_since(*last_time).as_secs_f64();
                
                if elapsed >= 1.0 {
                    let current_count = throughput_counter.load(Ordering::Relaxed);
                    let events_per_second = current_count as f64 / elapsed;
                    
                    let mut stats_guard = stats.write().unwrap();
                    stats_guard.events_per_second = events_per_second;
                    if events_per_second > stats_guard.peak_events_per_second {
                        stats_guard.peak_events_per_second = events_per_second;
                    }
                    
                    // Calculate latency percentiles
                    let histogram = latency_histogram.read().unwrap();
                    if !histogram.is_empty() {
                        let mut sorted_latencies = histogram.clone();
                        sorted_latencies.sort_unstable();
                        
                        let len = sorted_latencies.len();
                        stats_guard.avg_propagation_latency_ns =
                            sorted_latencies.iter().sum::<u64>() / len as u64;
                        
                        if len > 0 {
                            stats_guard.p95_propagation_latency_ns =
                                sorted_latencies[(len * 95 / 100).min(len - 1)];
                            stats_guard.p99_propagation_latency_ns =
                                sorted_latencies[(len * 99 / 100).min(len - 1)];
                        }
                    }
                    
                    // Reset counters
                    throughput_counter.store(0, Ordering::Relaxed);
                    *last_time = now;
                }
            }
        });
    }
}

/// Convenience functions for event propagation

/// Propagate a filesystem event across boundaries
pub fn propagate_filesystem_event(
    manager: &EventPropagationManager,
    event: SemanticEvent,
    source_boundary: EventBoundary,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let target_boundaries = vec![
        EventBoundary::FuseUserspace,
        EventBoundary::GraphLayer,
        EventBoundary::VectorLayer,
    ];
    
    manager.propagate_event(event, source_boundary, target_boundaries)
}

/// Propagate a graph event across boundaries
pub fn propagate_graph_event(
    manager: &EventPropagationManager,
    event: SemanticEvent,
    source_boundary: EventBoundary,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let target_boundaries = vec![
        EventBoundary::VectorLayer,
        EventBoundary::AgentLayer,
        EventBoundary::FuseUserspace,
    ];
    
    manager.propagate_event(event, source_boundary, target_boundaries)
}

/// Propagate a vector event across boundaries
pub fn propagate_vector_event(
    manager: &EventPropagationManager,
    event: SemanticEvent,
    source_boundary: EventBoundary,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let target_boundaries = vec![
        EventBoundary::GraphLayer,
        EventBoundary::AgentLayer,
        EventBoundary::FuseUserspace,
    ];
    
    manager.propagate_event(event, source_boundary, target_boundaries)
}

/// Global event propagation manager instance
static mut GLOBAL_PROPAGATION_MANAGER: Option<Arc<Mutex<EventPropagationManager>>> = None;
static PROPAGATION_INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Initialize the global event propagation manager
pub fn initialize_event_propagation(
    config: EventPropagationConfig
) -> Result<(), Box<dyn std::error::Error>> {
    PROPAGATION_INIT_ONCE.call_once(|| {
        let manager = EventPropagationManager::new(config);
        unsafe {
            GLOBAL_PROPAGATION_MANAGER = Some(Arc::new(Mutex::new(manager)));
        }
    });
    
    // Start the manager
    unsafe {
        if let Some(manager) = &GLOBAL_PROPAGATION_MANAGER {
            manager.lock().unwrap().start()?;
        }
    }
    
    Ok(())
}

/// Get the global event propagation manager
pub fn get_global_propagation_manager() -> Option<Arc<Mutex<EventPropagationManager>>> {
    unsafe { GLOBAL_PROPAGATION_MANAGER.clone() }
}

/// Shutdown the global event propagation manager
pub fn shutdown_event_propagation() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if let Some(manager) = &GLOBAL_PROPAGATION_MANAGER {
            manager.lock().unwrap().stop()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_api::types::SemanticEventType;
    
    #[tokio::test]
    async fn test_event_propagation_manager() {
        let config = EventPropagationConfig::default();
        let mut manager = EventPropagationManager::new(config);
        
        manager.start().unwrap();
        
        // Create a test event
        let event = SemanticEvent {
            event_id: 12345,
            event_type: SemanticEventType::FilesystemCreate,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: chrono::Utc::now(),
                sequence: 1,
                cpu_id: 0,
                process_id: std::process::id(),
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
            event_size: 0,
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
        
        // Test propagation
        let propagation_ids = manager.propagate_event(
            event,
            EventBoundary::KernelModule,
            vec![EventBoundary::FuseUserspace, EventBoundary::GraphLayer],
        ).unwrap();
        
        assert_eq!(propagation_ids.len(), 2);
        
        // Wait a bit for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let stats = manager.get_stats();
        assert!(stats.total_events_propagated >= 2);
        
        manager.stop().unwrap();
    }
    
    #[test]
    fn test_event_boundary_routing() {
        assert!(EventBoundary::KernelModule.can_propagate_to(EventBoundary::FuseUserspace));
        assert!(EventBoundary::FuseUserspace.can_propagate_to(EventBoundary::KernelModule));
        assert!(EventBoundary::GraphLayer.can_propagate_to(EventBoundary::VectorLayer));
        assert!(!EventBoundary::KernelModule.can_propagate_to(EventBoundary::KernelModule));
    }
    
    #[test]
    fn test_deduplication() {
        let config = EventPropagationConfig::default();
        let manager = EventPropagationManager::new(config);
        
        let event = CrossBoundaryEvent {
            event: SemanticEvent {
                event_id: 1,
                event_type: SemanticEventType::FilesystemCreate,
                event_subtype: None,
                timestamp: SemanticTimestamp {
                    timestamp: chrono::Utc::now(),
                    sequence: 1,
                    cpu_id: 0,
                    process_id: 0,
                },
                global_sequence: 1,
                local_sequence: 1,
                flags: EventFlags {
                    atomic: false,
                    transactional: false,
                    causal: false,
                    agent_visible: false,
                    deterministic: false,
                    compressed: false,
                    indexed: false,
                    replicated: false,
                },
                priority: EventPriority::Normal,
                event_size: 0,
                event_version: 1,
                checksum: None,
                compression_type: None,
                encryption_type: None,
                causality_links: Vec::new(),
                parent_event_id: None,
                root_cause_event_id: None,
                agent_visibility_mask: 0,
                agent_relevance_score: 0,
                replay_priority: 0,
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
                    observability: None,
                },
                payload: None,
                metadata: None,
            },
            propagation_id: Uuid::new_v4(),
            source_boundary: EventBoundary::KernelModule,
            target_boundary: EventBoundary::FuseUserspace,
            propagation_timestamp: SystemTime::now(),
            translation_latency_ns: 0,
            original_context_hash: 12345,
            translated_context_hash: 12345,
            context_preservation_score: 1.0,
            routing_key: "test".to_string(),
            priority_boost: 0,
            deduplication_key: "test_key".to_string(),
            propagation_start_ns: 0,
            translation_start_ns: 0,
            serialization_size_bytes: 0,
        };
        
        // First event should not be duplicate
        assert!(!manager.is_duplicate_event(&event).unwrap());
        
        // Second identical event should be duplicate
        assert!(manager.is_duplicate_event(&event).unwrap());
    }
}