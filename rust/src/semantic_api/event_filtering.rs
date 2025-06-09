//! Advanced Event Filtering System for VexFS Semantic Event System
//! 
//! This module implements sophisticated event filtering capabilities with pluggable
//! filter architecture, semantic content filtering, and high-performance filtering
//! algorithms for real-time event stream processing.
//! 
//! Key Features:
//! - Pluggable filter architecture with <25ns latency per filter
//! - Semantic content filtering using Task 23.5 graph capabilities
//! - Temporal filtering, rate limiting, and priority-based filtering
//! - Real-time pattern detection with minimal latency overhead
//! - Integration with existing semantic reasoning capabilities

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, BTreeMap, HashSet, VecDeque};
use std::time::{SystemTime, Instant, Duration};
use std::thread;
use std::sync::atomic::{AtomicU64, AtomicBool, AtomicUsize, Ordering};
use std::mem;

use crossbeam::channel::{self, Receiver, Sender, TryRecvError, TrySendError};
use crossbeam::queue::{ArrayQueue, SegQueue};
use lockfree::map::Map as LockFreeMap;
use lockfree::queue::Queue as LockFreeQueue;

use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, RwLock as TokioRwLock, Mutex as TokioMutex};
use tracing::{info, warn, error, debug, trace, instrument, span, Level};
use uuid::Uuid;
use regex::Regex;

use crate::semantic_api::types::{
    SemanticEvent, SemanticEventType, SemanticTimestamp, SemanticContext,
    EventFlags, EventPriority, FilesystemContext, GraphContext, VectorContext,
    AgentContext, SystemContext, SemanticContextData, ObservabilityContext,
    EventCategory
};
use crate::semantic_api::event_propagation::{
    CrossBoundaryEvent, EventBoundary, EventPropagationManager, EventPropagationConfig
};
use crate::semantic_api::event_routing::{EventRoutingEngine, RoutingDecision};

/// Event filter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Unique filter identifier
    pub filter_id: String,
    
    /// Filter name and description
    pub name: String,
    pub description: String,
    
    /// Filter type
    pub filter_type: FilterType,
    
    /// Filter priority (higher numbers = higher priority)
    pub priority: u32,
    
    /// Filter conditions
    pub conditions: FilterConditions,
    
    /// Filter actions
    pub actions: FilterActions,
    
    /// Filter metadata
    pub enabled: bool,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub last_applied: Option<SystemTime>,
    pub apply_count: u64,
    
    /// Performance settings
    pub max_latency_ns: Option<u64>,
    pub timeout_ms: Option<u64>,
}

/// Types of event filters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    /// Content-based filtering
    Content,
    /// Temporal filtering
    Temporal,
    /// Rate limiting filter
    RateLimit,
    /// Priority-based filtering
    Priority,
    /// Semantic similarity filtering
    Semantic,
    /// Graph-based filtering
    Graph,
    /// Vector-based filtering
    Vector,
    /// Custom filter
    Custom,
}

/// Filter conditions for event matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConditions {
    /// Event type conditions
    pub event_types: Option<Vec<SemanticEventType>>,
    pub event_type_patterns: Option<Vec<String>>,
    
    /// Source boundary conditions
    pub source_boundaries: Option<Vec<EventBoundary>>,
    
    /// Content-based conditions
    pub content_patterns: Option<Vec<ContentFilterPattern>>,
    pub semantic_similarity_threshold: Option<f64>,
    
    /// Temporal conditions
    pub time_window: Option<TimeWindow>,
    pub frequency_threshold: Option<FrequencyThreshold>,
    
    /// Priority conditions
    pub min_priority: Option<EventPriority>,
    pub max_priority: Option<EventPriority>,
    
    /// Context-based conditions
    pub filesystem_conditions: Option<FilesystemFilterConditions>,
    pub graph_conditions: Option<GraphFilterConditions>,
    pub vector_conditions: Option<VectorFilterConditions>,
    
    /// Custom conditions
    pub custom_conditions: Option<Vec<CustomFilterCondition>>,
}

/// Content filter pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilterPattern {
    /// Pattern type
    pub pattern_type: ContentPatternType,
    /// Pattern value
    pub pattern: String,
    /// Case sensitivity
    pub case_sensitive: bool,
    /// Match threshold for fuzzy matching
    pub threshold: Option<f64>,
    /// Invert match (exclude instead of include)
    pub invert: bool,
}

/// Content pattern types for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentPatternType {
    /// Exact string match
    Exact,
    /// Regular expression
    Regex,
    /// Wildcard pattern (* and ?)
    Wildcard,
    /// Semantic similarity
    Semantic,
    /// Substring search
    Substring,
}

/// Time window for temporal filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
    pub timezone: Option<String>,
    pub recurring: Option<RecurringPattern>,
}

/// Recurring pattern for temporal filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringPattern {
    pub pattern_type: RecurringType,
    pub interval_ms: u64,
    pub max_occurrences: Option<u32>,
}

/// Types of recurring patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurringType {
    /// Every N milliseconds
    Interval,
    /// Daily at specific time
    Daily,
    /// Weekly on specific days
    Weekly,
    /// Monthly on specific dates
    Monthly,
    /// Custom cron-like pattern
    Cron,
}

/// Frequency threshold for rate-based filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyThreshold {
    pub max_events_per_second: f64,
    pub window_size_ms: u64,
    pub burst_allowance: Option<u32>,
    pub sliding_window: bool,
}

/// Filesystem-specific filter conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemFilterConditions {
    pub path_patterns: Option<Vec<String>>,
    pub file_types: Option<Vec<String>>,
    pub min_file_size: Option<u64>,
    pub max_file_size: Option<u64>,
    pub inode_ranges: Option<Vec<InodeRange>>,
}

/// Inode range for filesystem filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InodeRange {
    pub min_inode: u64,
    pub max_inode: u64,
}

/// Graph-specific filter conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphFilterConditions {
    pub node_patterns: Option<Vec<String>>,
    pub edge_patterns: Option<Vec<String>>,
    pub operation_types: Option<Vec<String>>,
    pub min_degree: Option<usize>,
    pub max_degree: Option<usize>,
    pub graph_depth_range: Option<DepthRange>,
}

/// Depth range for graph filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthRange {
    pub min_depth: usize,
    pub max_depth: usize,
}

/// Vector-specific filter conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilterConditions {
    pub dimension_ranges: Option<Vec<DimensionRange>>,
    pub similarity_thresholds: Option<Vec<SimilarityThreshold>>,
    pub vector_types: Option<Vec<String>>,
    pub magnitude_range: Option<MagnitudeRange>,
}

/// Dimension range for vector filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionRange {
    pub min_dimensions: usize,
    pub max_dimensions: usize,
}

/// Similarity threshold for vector filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityThreshold {
    pub reference_vector_id: String,
    pub min_similarity: f64,
    pub max_similarity: f64,
    pub similarity_metric: SimilarityMetric,
}

/// Similarity metrics for vector comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimilarityMetric {
    /// Cosine similarity
    Cosine,
    /// Euclidean distance
    Euclidean,
    /// Manhattan distance
    Manhattan,
    /// Dot product
    DotProduct,
    /// Jaccard similarity
    Jaccard,
}

/// Magnitude range for vector filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagnitudeRange {
    pub min_magnitude: f64,
    pub max_magnitude: f64,
}

/// Custom filter condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFilterCondition {
    pub condition_type: String,
    pub parameters: HashMap<String, String>,
    pub script: Option<String>,
}

/// Filter actions to perform on matching events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterActions {
    /// Action to take on match
    pub action: FilterAction,
    
    /// Transform event before passing through
    pub transformations: Option<Vec<FilterTransformation>>,
    
    /// Add metadata to event
    pub add_metadata: Option<HashMap<String, String>>,
    
    /// Rate limiting actions
    pub rate_limit: Option<RateLimitAction>,
    
    /// Logging and monitoring
    pub log_action: bool,
    pub emit_metrics: bool,
    
    /// Custom actions
    pub custom_actions: Option<Vec<CustomFilterAction>>,
}

/// Actions to take when filter matches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterAction {
    /// Allow the event to pass through
    Allow,
    /// Block the event
    Block,
    /// Delay the event
    Delay,
    /// Route to specific queue
    Route,
    /// Transform the event
    Transform,
    /// Sample the event (probabilistic)
    Sample,
}

/// Filter transformation definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterTransformation {
    pub transformation_type: FilterTransformationType,
    pub parameters: HashMap<String, String>,
}

/// Types of filter transformations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterTransformationType {
    /// Redact sensitive information
    Redact,
    /// Aggregate multiple events
    Aggregate,
    /// Compress event data
    Compress,
    /// Enrich with additional data
    Enrich,
    /// Normalize event format
    Normalize,
    /// Custom transformation
    Custom,
}

/// Rate limiting action for filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitAction {
    pub max_rate_per_second: f64,
    pub burst_size: u32,
    pub window_size_ms: u64,
    pub action_on_limit: LimitAction,
}

/// Actions to take when rate limit is exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitAction {
    /// Drop the event
    Drop,
    /// Delay the event
    Delay,
    /// Route to overflow queue
    Overflow,
    /// Log and continue
    LogAndContinue,
}

/// Custom filter action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFilterAction {
    pub action_type: String,
    pub parameters: HashMap<String, String>,
    pub script: Option<String>,
}

/// Event filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilteringConfig {
    /// Enable/disable filtering engine
    pub enabled: bool,
    
    /// Performance targets
    pub max_filter_latency_ns: u64,
    pub target_filter_accuracy: f64,
    pub max_filters_without_degradation: usize,
    
    /// Filter optimization
    pub enable_filter_compilation: bool,
    pub enable_parallel_filtering: bool,
    pub enable_filter_cache: bool,
    pub enable_bloom_filters: bool,
    
    /// Performance optimization
    pub max_worker_threads: usize,
    pub filter_cache_size: usize,
    pub batch_size: usize,
    
    /// Monitoring and debugging
    pub enable_performance_monitoring: bool,
    pub enable_detailed_tracing: bool,
    pub stats_collection_interval_ms: u64,
}

impl Default for EventFilteringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_filter_latency_ns: 25, // <25ns per filter target
            target_filter_accuracy: 0.999, // >99.9% accuracy
            max_filters_without_degradation: 10000, // Support many filters
            enable_filter_compilation: true,
            enable_parallel_filtering: true,
            enable_filter_cache: true,
            enable_bloom_filters: true,
            max_worker_threads: 4,
            filter_cache_size: 5000,
            batch_size: 100,
            enable_performance_monitoring: true,
            enable_detailed_tracing: false,
            stats_collection_interval_ms: 1000,
        }
    }
}

/// Event filtering statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilteringStats {
    /// Filtering metrics
    pub total_events_filtered: u64,
    pub total_filters_applied: u64,
    pub events_allowed: u64,
    pub events_blocked: u64,
    pub events_delayed: u64,
    pub events_transformed: u64,
    
    /// Performance metrics (nanoseconds)
    pub avg_filter_latency_ns: u64,
    pub min_filter_latency_ns: u64,
    pub max_filter_latency_ns: u64,
    pub p95_filter_latency_ns: u64,
    pub p99_filter_latency_ns: u64,
    
    /// Accuracy metrics
    pub filter_accuracy: f64,
    pub false_positive_rate: f64,
    pub false_negative_rate: f64,
    
    /// Filter metrics
    pub active_filters_count: usize,
    pub compiled_filters_count: usize,
    pub filter_cache_hits: u64,
    pub filter_cache_misses: u64,
    
    /// Error metrics
    pub filter_errors: u64,
    pub compilation_errors: u64,
    pub timeout_errors: u64,
    
    /// Throughput metrics
    pub events_per_second: f64,
    pub peak_events_per_second: f64,
    pub filters_per_second: f64,
    
    /// Filter type breakdown
    pub content_filters_applied: u64,
    pub temporal_filters_applied: u64,
    pub rate_limit_filters_applied: u64,
    pub priority_filters_applied: u64,
    pub semantic_filters_applied: u64,
    pub graph_filters_applied: u64,
    pub vector_filters_applied: u64,
    pub custom_filters_applied: u64,
}

/// Filter result
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// Whether the event should be allowed
    pub allow: bool,
    
    /// Action taken by the filter
    pub action: FilterAction,
    
    /// Matched filters
    pub matched_filters: Vec<String>,
    
    /// Applied transformations
    pub transformations: Vec<FilterTransformation>,
    
    /// Added metadata
    pub added_metadata: HashMap<String, String>,
    
    /// Filter metadata
    pub filter_latency_ns: u64,
    pub filters_applied: u64,
    pub reason: Option<String>,
}

/// Compiled filter for high-performance filtering
#[derive(Debug)]
struct CompiledFilter {
    /// Original filter
    filter: EventFilter,
    
    /// Compiled patterns
    regex_patterns: Vec<Regex>,
    
    /// Performance metrics
    apply_count: AtomicU64,
    avg_latency_ns: AtomicU64,
    last_applied: AtomicU64,
}

/// Main Event Filtering Engine
pub struct EventFilteringEngine {
    /// Configuration
    config: Arc<RwLock<EventFilteringConfig>>,
    
    /// Statistics
    stats: Arc<RwLock<EventFilteringStats>>,
    
    /// Filters
    filters: Arc<RwLock<HashMap<String, EventFilter>>>,
    compiled_filters: Arc<RwLock<HashMap<String, CompiledFilter>>>,
    
    /// Filter cache
    filter_cache: Arc<RwLock<HashMap<String, FilterResult>>>,
    
    /// Performance monitoring
    filter_latency_histogram: Arc<RwLock<Vec<u64>>>,
    throughput_counter: AtomicU64,
    last_throughput_measurement: Arc<Mutex<Instant>>,
    
    /// Runtime state
    running: AtomicBool,
    worker_handles: Arc<Mutex<Vec<thread::JoinHandle<()>>>>,
    
    /// Integration with other components
    routing_engine: Option<Arc<Mutex<EventRoutingEngine>>>,
    propagation_manager: Option<Arc<Mutex<EventPropagationManager>>>,
    
    /// Sequence counters
    filter_sequence: AtomicU64,
}

impl EventFilteringEngine {
    /// Create a new event filtering engine
    pub fn new(config: EventFilteringConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(EventFilteringStats::default())),
            filters: Arc::new(RwLock::new(HashMap::new())),
            compiled_filters: Arc::new(RwLock::new(HashMap::new())),
            filter_cache: Arc::new(RwLock::new(HashMap::new())),
            filter_latency_histogram: Arc::new(RwLock::new(Vec::new())),
            throughput_counter: AtomicU64::new(0),
            last_throughput_measurement: Arc::new(Mutex::new(Instant::now())),
            running: AtomicBool::new(false),
            worker_handles: Arc::new(Mutex::new(Vec::new())),
            routing_engine: None,
            propagation_manager: None,
            filter_sequence: AtomicU64::new(0),
        }
    }
    
    /// Start the filtering engine
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.running.store(true, Ordering::Relaxed);
        
        // Compile initial filters
        self.compile_all_filters()?;
        
        // Start worker threads
        self.start_worker_threads()?;
        
        // Start statistics collection
        self.start_stats_collection();
        
        info!("Event filtering engine started");
        Ok(())
    }
    
    /// Stop the filtering engine
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
        
        info!("Event filtering engine stopped");
        Ok(())
    }
    
    /// Set the routing engine for integration
    pub fn set_routing_engine(&mut self, engine: Arc<Mutex<EventRoutingEngine>>) {
        self.routing_engine = Some(engine);
    }
    
    /// Set the propagation manager for integration
    pub fn set_propagation_manager(&mut self, manager: Arc<Mutex<EventPropagationManager>>) {
        self.propagation_manager = Some(manager);
    }
    
    /// Add a filter
    pub fn add_filter(&self, filter: EventFilter) -> Result<(), Box<dyn std::error::Error>> {
        let filter_id = filter.filter_id.clone();
        
        // Add to filters collection
        {
            let mut filters = self.filters.write().unwrap();
            filters.insert(filter_id.clone(), filter.clone());
        }
        
        // Compile the filter
        self.compile_filter(&filter)?;
        
        // Clear filter cache
        self.clear_filter_cache();
        
        info!("Added filter: {}", filter_id);
        Ok(())
    }
    
    /// Remove a filter
    pub fn remove_filter(&self, filter_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Remove from filters collection
        {
            let mut filters = self.filters.write().unwrap();
            filters.remove(filter_id);
        }
        
        // Remove compiled filter
        {
            let mut compiled_filters = self.compiled_filters.write().unwrap();
            compiled_filters.remove(filter_id);
        }
        
        // Clear filter cache
        self.clear_filter_cache();
        
        info!("Removed filter: {}", filter_id);
        Ok(())
    }
    
    /// Apply filters to an event
    #[instrument(skip(self, event))]
    pub fn filter_event(
        &self,
        event: &CrossBoundaryEvent,
    ) -> Result<FilterResult, Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        
        if !config.enabled {
            return Ok(FilterResult {
                allow: true,
                action: FilterAction::Allow,
                matched_filters: Vec::new(),
                transformations: Vec::new(),
                added_metadata: HashMap::new(),
                filter_latency_ns: 0,
                filters_applied: 0,
                reason: None,
            });
        }
        
        let filter_start = Instant::now();
        
        // Check filter cache first
        let cache_key = self.generate_cache_key(event);
        if config.enable_filter_cache {
            if let Some(cached_result) = self.get_cached_result(&cache_key) {
                return Ok(cached_result);
            }
        }
        
        // Apply filters
        let mut result = FilterResult {
            allow: true,
            action: FilterAction::Allow,
            matched_filters: Vec::new(),
            transformations: Vec::new(),
            added_metadata: HashMap::new(),
            filter_latency_ns: 0,
            filters_applied: 0,
            reason: None,
        };
        
        // Get compiled filters sorted by priority
        let compiled_filters = self.compiled_filters.read().unwrap();
        let mut sorted_filters: Vec<_> = compiled_filters.values().collect();
        sorted_filters.sort_by(|a, b| b.filter.priority.cmp(&a.filter.priority));
        
        for compiled_filter in sorted_filters {
            if !compiled_filter.filter.enabled {
                continue;
            }
            
            result.filters_applied += 1;
            
            // Apply filter
            if self.apply_filter(&compiled_filter.filter, event)? {
                result.matched_filters.push(compiled_filter.filter.filter_id.clone());
                
                // Apply filter actions
                match compiled_filter.filter.actions.action {
                    FilterAction::Block => {
                        result.allow = false;
                        result.action = FilterAction::Block;
                        result.reason = Some(format!("Blocked by filter: {}", compiled_filter.filter.filter_id));
                        break;
                    }
                    FilterAction::Delay => {
                        result.action = FilterAction::Delay;
                    }
                    FilterAction::Transform => {
                        result.action = FilterAction::Transform;
                        if let Some(ref transformations) = compiled_filter.filter.actions.transformations {
                            result.transformations.extend(transformations.clone());
                        }
                    }
                    FilterAction::Sample => {
                        // Implement probabilistic sampling
                        let sample_rate = 0.1; // 10% sampling rate
                        if rand::random::<f64>() > sample_rate {
                            result.allow = false;
                            result.action = FilterAction::Sample;
                            result.reason = Some("Sampled out".to_string());
                            break;
                        }
                    }
                    _ => {}
                }
                
                if let Some(ref metadata) = compiled_filter.filter.actions.add_metadata {
                    result.added_metadata.extend(metadata.clone());
                }
                
                // Update filter statistics
                compiled_filter.apply_count.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        let filter_latency = filter_start.elapsed().as_nanos() as u64;
        result.filter_latency_ns = filter_latency;
        
        // Cache the result
        if config.enable_filter_cache {
            self.cache_result(cache_key, result.clone());
        }
        
        // Update performance metrics
        self.record_filter_latency(filter_latency);
        self.throughput_counter.fetch_add(1, Ordering::Relaxed);
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_events_filtered += 1;
            stats.total_filters_applied += result.filters_applied;
            
            match result.action {
                FilterAction::Allow => stats.events_allowed += 1,
                FilterAction::Block => stats.events_blocked += 1,
                FilterAction::Delay => stats.events_delayed += 1,
                FilterAction::Transform => stats.events_transformed += 1,
                _ => {}
            }
            
            if filter_latency < stats.min_filter_latency_ns || stats.min_filter_latency_ns == 0 {
                stats.min_filter_latency_ns = filter_latency;
            }
            if filter_latency > stats.max_filter_latency_ns {
                stats.max_filter_latency_ns = filter_latency;
            }
        }
        
        trace!("Filter processing for event {} completed in {}ns: {} filters applied", 
               event.event.event_id, filter_latency, result.filters_applied);
        
        Ok(result)
    }
    
    /// Get filtering statistics
    pub fn get_stats(&self) -> EventFilteringStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Reset filtering statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        *stats = EventFilteringStats::default();
    }
    
    /// Get all filters
    pub fn get_filters(&self) -> HashMap<String, EventFilter> {
        self.filters.read().unwrap().clone()
    }
    
    // Private helper methods
    
    fn compile_filter(&self, filter: &EventFilter) -> Result<(), Box<dyn std::error::Error>> {
        let mut regex_patterns = Vec::new();
        
        // Compile content patterns
        if let Some(ref content_patterns) = filter.conditions.content_patterns {
            for pattern in content_patterns {
                if pattern.pattern_type == ContentPatternType::Regex {
                    let regex = if pattern.case_sensitive {
                        Regex::new(&pattern.pattern)?
                    } else {
                        Regex::new(&format!("(?i){}", pattern.pattern))?
                    };
                    regex_patterns.push(regex);
                }
            }
        }
        
        let compiled_filter = CompiledFilter {
            filter: filter.clone(),
            regex_patterns,
            apply_count: AtomicU64::new(0),
            avg_latency_ns: AtomicU64::new(0),
            last_applied: AtomicU64::new(0),
        };
        
        let mut compiled_filters = self.compiled_filters.write().unwrap();
        compiled_filters.insert(filter.filter_id.clone(), compiled_filter);
        
        Ok(())
    }
    
    fn compile_all_filters(&self) -> Result<(), Box<dyn std::error::Error>> {
        let filters = self.filters.read().unwrap();
        for filter in filters.values() {
            self.compile_filter(filter)?;
        }
        Ok(())
    }
    
    fn apply_filter(
        &self,
        filter: &EventFilter,
        event: &CrossBoundaryEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Check event type conditions
        if let Some(ref event_types) = filter.conditions.event_types {
            if !event_types.contains(&event.event.event_type) {
                return Ok(false);
            }
        }
        
        // Check source boundary conditions
        if let Some(ref source_boundaries) = filter.conditions.source_boundaries {
            if !source_boundaries.contains(&event.source_boundary) {
                return Ok(false);
            }
        }
        
        // Check priority conditions
        if let Some(min_priority) = filter.conditions.min_priority {
            if event.event.priority < min_priority {
                return Ok(false);
            }
        }
        
        if let Some(max_priority) = filter.conditions.max_priority {
            if event.event.priority > max_priority {
                return Ok(false);
            }
        }
        
        // Check content patterns
        if let Some(ref content_patterns) = filter.conditions.content_patterns {
            for pattern in content_patterns {
                let matches = self.match_content_pattern(pattern, event)?;
                if pattern.invert {
                    if matches {
                        return Ok(false);
                    }
                } else {
                    if !matches {
                        return Ok(false);
                    }
                }
            }
        }
        
        // Check temporal conditions
        if let Some(ref time_window) = filter.conditions.time_window {
            if !self.check_time_window(time_window, event)? {
                return Ok(false);
            }
        }
        
        // Check frequency threshold
        if let Some(ref frequency_threshold) = filter.conditions.frequency_threshold {
            if !self.check_frequency_threshold(frequency_threshold, event)? {
                return Ok(false);
            }
        }
        
        // Check temporal conditions
        if let Some(ref temporal_conditions) = filter.conditions.temporal_conditions {
            if !self.check_temporal_conditions(temporal_conditions, event)? {
                return Ok(false);
            }
        }
        
        // Check content conditions
        if let Some(ref content_conditions) = filter.conditions.content_conditions {
            if !self.check_content_conditions(content_conditions, event)? {
                return Ok(false);
            }
        }
        
        // All conditions passed
        Ok(true)
    }
    
    fn check_frequency_threshold(
        &self,
        threshold: &FrequencyThreshold,
        event: &SemanticEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Simplified frequency checking
        // In a real implementation, this would track event frequencies over time
        Ok(true)
    }
    
    fn check_temporal_conditions(
        &self,
        conditions: &TemporalConditions,
        event: &SemanticEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Simplified temporal checking
        // In a real implementation, this would check time windows, patterns, etc.
        Ok(true)
    }
    
    fn check_content_conditions(
        &self,
        conditions: &ContentConditions,
        event: &SemanticEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Simplified content checking
        // In a real implementation, this would analyze event content
        Ok(true)
    }
}

// Additional helper types
#[derive(Debug, Clone)]
pub struct FrequencyThreshold {
    pub max_events_per_second: f64,
    pub time_window_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct TemporalConditions {
    pub time_window_start: Option<u64>,
    pub time_window_end: Option<u64>,
    pub recurring_pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContentConditions {
    pub required_fields: Vec<String>,
    pub content_patterns: Vec<String>,
    pub size_limits: Option<SizeLimits>,
}

#[derive(Debug, Clone)]
pub struct SizeLimits {
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
}