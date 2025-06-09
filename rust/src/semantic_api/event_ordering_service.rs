//! Event Ordering Service
//!
//! This module implements the EventOrderingService for consistent event ordering
//! across kernel and userspace boundaries using vector clocks and conflict resolution.

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, BTreeMap, VecDeque, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::cmp::Ordering;
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock as TokioRwLock, Mutex as TokioMutex};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error, debug, trace};
use uuid::Uuid;

use crate::semantic_api::types::*;
use crate::shared::errors::{VexfsError, VexfsResult};

/// Vector clock for distributed event ordering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    /// Clock values for each process/node
    pub clocks: BTreeMap<String, u64>,
    /// Last update timestamp
    pub last_update: SystemTime,
}

impl VectorClock {
    /// Create new vector clock
    pub fn new() -> Self {
        Self {
            clocks: BTreeMap::new(),
            last_update: SystemTime::now(),
        }
    }

    /// Create vector clock with initial process
    pub fn with_process(process_id: String) -> Self {
        let mut clocks = BTreeMap::new();
        clocks.insert(process_id, 0);
        
        Self {
            clocks,
            last_update: SystemTime::now(),
        }
    }

    /// Increment clock for a process
    pub fn increment(&mut self, process_id: &str) {
        let current = self.clocks.get(process_id).unwrap_or(&0);
        self.clocks.insert(process_id.to_string(), current + 1);
        self.last_update = SystemTime::now();
    }

    /// Update clock with another vector clock (for message reception)
    pub fn update(&mut self, other: &VectorClock, process_id: &str) {
        // Update all clocks to maximum values
        for (pid, &other_time) in &other.clocks {
            let current = self.clocks.get(pid).unwrap_or(&0);
            self.clocks.insert(pid.clone(), (*current).max(other_time));
        }
        
        // Increment our own clock
        self.increment(process_id);
    }

    /// Compare two vector clocks for ordering
    pub fn compare(&self, other: &VectorClock) -> VectorClockOrdering {
        let mut less_than = false;
        let mut greater_than = false;
        
        // Get all process IDs from both clocks
        let all_processes: HashSet<_> = self.clocks.keys()
            .chain(other.clocks.keys())
            .collect();
        
        for process_id in all_processes {
            let self_time = self.clocks.get(process_id).unwrap_or(&0);
            let other_time = other.clocks.get(process_id).unwrap_or(&0);
            
            match self_time.cmp(other_time) {
                Ordering::Less => less_than = true,
                Ordering::Greater => greater_than = true,
                Ordering::Equal => {}
            }
        }
        
        match (less_than, greater_than) {
            (true, false) => VectorClockOrdering::Before,
            (false, true) => VectorClockOrdering::After,
            (false, false) => VectorClockOrdering::Equal,
            (true, true) => VectorClockOrdering::Concurrent,
        }
    }

    /// Check if this clock happens before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        matches!(self.compare(other), VectorClockOrdering::Before)
    }

    /// Check if this clock is concurrent with another
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        matches!(self.compare(other), VectorClockOrdering::Concurrent)
    }

    /// Get clock value for a specific process
    pub fn get_clock(&self, process_id: &str) -> u64 {
        self.clocks.get(process_id).unwrap_or(&0).clone()
    }

    /// Get all process IDs in this clock
    pub fn get_processes(&self) -> Vec<String> {
        self.clocks.keys().cloned().collect()
    }
}

/// Vector clock ordering relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorClockOrdering {
    /// This clock happens before the other
    Before,
    /// This clock happens after the other
    After,
    /// Clocks are equal
    Equal,
    /// Clocks are concurrent (no causal relationship)
    Concurrent,
}

/// Ordered semantic event with vector clock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedSemanticEvent {
    /// Base semantic event
    pub event: SemanticEvent,
    /// Vector clock for ordering
    pub vector_clock: VectorClock,
    /// Process/node that generated the event
    pub process_id: String,
    /// Sequence number within process
    pub sequence_number: u64,
    /// Event ordering priority
    pub priority: EventOrderingPriority,
    /// Causal dependencies (events that must happen before this one)
    pub causal_dependencies: Vec<EventId>,
    /// Conflict resolution metadata
    pub conflict_metadata: ConflictMetadata,
}

impl OrderedSemanticEvent {
    /// Create new ordered event
    pub fn new(
        event: SemanticEvent,
        process_id: String,
        sequence_number: u64,
        vector_clock: VectorClock,
    ) -> Self {
        Self {
            event,
            vector_clock,
            process_id,
            sequence_number,
            priority: EventOrderingPriority::Normal,
            causal_dependencies: Vec::new(),
            conflict_metadata: ConflictMetadata::default(),
        }
    }

    /// Get event ID
    pub fn event_id(&self) -> EventId {
        self.event.event_id
    }

    /// Check if this event causally depends on another
    pub fn depends_on(&self, other_event_id: EventId) -> bool {
        self.causal_dependencies.contains(&other_event_id)
    }

    /// Add causal dependency
    pub fn add_dependency(&mut self, event_id: EventId) {
        if !self.causal_dependencies.contains(&event_id) {
            self.causal_dependencies.push(event_id);
        }
    }

    /// Check if this event conflicts with another
    pub fn conflicts_with(&self, other: &OrderedSemanticEvent) -> bool {
        // Events conflict if they are concurrent and operate on the same resource
        if !self.vector_clock.is_concurrent(&other.vector_clock) {
            return false;
        }

        // Check for resource conflicts based on event context
        self.has_resource_conflict(other)
    }

    /// Check for resource conflicts
    fn has_resource_conflict(&self, other: &OrderedSemanticEvent) -> bool {
        // Check filesystem conflicts
        if let (Some(fs_ctx1), Some(fs_ctx2)) = (
            &self.event.context.filesystem,
            &other.event.context.filesystem,
        ) {
            if fs_ctx1.path == fs_ctx2.path {
                // Same file path - check for write conflicts
                return matches!(
                    (self.event.event_type, other.event.event_type),
                    (SemanticEventType::FilesystemWrite, _) |
                    (_, SemanticEventType::FilesystemWrite) |
                    (SemanticEventType::FilesystemDelete, _) |
                    (_, SemanticEventType::FilesystemDelete)
                );
            }
        }

        // Check graph conflicts
        if let (Some(graph_ctx1), Some(graph_ctx2)) = (
            &self.event.context.graph,
            &other.event.context.graph,
        ) {
            if graph_ctx1.node_id == graph_ctx2.node_id || 
               graph_ctx1.edge_id == graph_ctx2.edge_id {
                return true;
            }
        }

        // Check vector conflicts
        if let (Some(vec_ctx1), Some(vec_ctx2)) = (
            &self.event.context.vector,
            &other.event.context.vector,
        ) {
            if vec_ctx1.vector_id == vec_ctx2.vector_id {
                return true;
            }
        }

        false
    }
}

/// Event ordering priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventOrderingPriority {
    /// Critical system events (highest priority)
    Critical = 0,
    /// High priority events
    High = 1,
    /// Normal priority events
    Normal = 2,
    /// Low priority events
    Low = 3,
    /// Background events (lowest priority)
    Background = 4,
}

impl Default for EventOrderingPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Conflict resolution metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConflictMetadata {
    /// Conflict resolution strategy
    pub resolution_strategy: ConflictResolutionStrategy,
    /// Conflict resolution timestamp
    pub resolution_timestamp: Option<SystemTime>,
    /// Conflict resolution reason
    pub resolution_reason: Option<String>,
    /// Original conflicting event IDs
    pub conflicting_events: Vec<EventId>,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Last writer wins
    LastWriterWins,
    /// First writer wins
    FirstWriterWins,
    /// Merge conflicts if possible
    Merge,
    /// Abort conflicting transaction
    Abort,
    /// Manual resolution required
    Manual,
    /// Priority-based resolution
    Priority,
}

impl Default for ConflictResolutionStrategy {
    fn default() -> Self {
        Self::LastWriterWins
    }
}

/// Event sequence gap information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceGap {
    /// Process ID with the gap
    pub process_id: String,
    /// Expected sequence number
    pub expected_sequence: u64,
    /// Actual sequence number received
    pub actual_sequence: u64,
    /// Gap detection timestamp
    pub detected_at: SystemTime,
    /// Gap size
    pub gap_size: u64,
}

impl SequenceGap {
    /// Create new sequence gap
    pub fn new(process_id: String, expected: u64, actual: u64) -> Self {
        Self {
            process_id,
            expected_sequence: expected,
            actual_sequence: actual,
            detected_at: SystemTime::now(),
            gap_size: actual.saturating_sub(expected),
        }
    }
}

/// Event ordering statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventOrderingStats {
    /// Total events processed
    pub events_processed: u64,
    /// Events reordered
    pub events_reordered: u64,
    /// Conflicts detected
    pub conflicts_detected: u64,
    /// Conflicts resolved
    pub conflicts_resolved: u64,
    /// Sequence gaps detected
    pub sequence_gaps_detected: u64,
    /// Sequence gaps resolved
    pub sequence_gaps_resolved: u64,
    /// Average ordering latency (microseconds)
    pub avg_ordering_latency_us: u64,
    /// Maximum ordering latency (microseconds)
    pub max_ordering_latency_us: u64,
    /// Current pending events
    pub pending_events: u64,
    /// Vector clock updates
    pub vector_clock_updates: u64,
}

/// Event ordering service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOrderingConfig {
    /// Maximum pending events before backpressure
    pub max_pending_events: usize,
    /// Sequence gap timeout (ms)
    pub sequence_gap_timeout_ms: u64,
    /// Conflict resolution timeout (ms)
    pub conflict_resolution_timeout_ms: u64,
    /// Enable vector clock validation
    pub enable_vector_clock_validation: bool,
    /// Enable conflict detection
    pub enable_conflict_detection: bool,
    /// Enable sequence gap detection
    pub enable_sequence_gap_detection: bool,
    /// Default conflict resolution strategy
    pub default_conflict_resolution: ConflictResolutionStrategy,
    /// Ordering buffer size
    pub ordering_buffer_size: usize,
    /// Process heartbeat interval (ms)
    pub process_heartbeat_interval_ms: u64,
}

impl Default for EventOrderingConfig {
    fn default() -> Self {
        Self {
            max_pending_events: 10000,
            sequence_gap_timeout_ms: 5000,
            conflict_resolution_timeout_ms: 1000,
            enable_vector_clock_validation: true,
            enable_conflict_detection: true,
            enable_sequence_gap_detection: true,
            default_conflict_resolution: ConflictResolutionStrategy::LastWriterWins,
            ordering_buffer_size: 1000,
            process_heartbeat_interval_ms: 1000,
        }
    }
}

/// Event ordering service
pub struct EventOrderingService {
    /// Configuration
    config: Arc<RwLock<EventOrderingConfig>>,
    
    /// Process vector clocks
    process_clocks: Arc<TokioRwLock<HashMap<String, VectorClock>>>,
    
    /// Process sequence numbers
    process_sequences: Arc<TokioRwLock<HashMap<String, u64>>>,
    
    /// Pending events waiting for ordering
    pending_events: Arc<TokioRwLock<VecDeque<OrderedSemanticEvent>>>,
    
    /// Ordered events ready for delivery
    ordered_events: Arc<TokioRwLock<VecDeque<OrderedSemanticEvent>>>,
    
    /// Detected sequence gaps
    sequence_gaps: Arc<TokioRwLock<HashMap<String, Vec<SequenceGap>>>>,
    
    /// Conflict resolution queue
    conflict_queue: Arc<TokioRwLock<VecDeque<ConflictResolutionTask>>>,
    
    /// Statistics
    stats: Arc<TokioRwLock<EventOrderingStats>>,
    
    /// Command channels
    command_sender: Sender<OrderingCommand>,
    command_receiver: Arc<Mutex<Receiver<OrderingCommand>>>,
    
    /// Event channels
    event_sender: Sender<OrderingEvent>,
    event_receiver: Arc<Mutex<Receiver<OrderingEvent>>>,
    
    /// Shutdown signal
    shutdown_sender: Sender<()>,
    shutdown_receiver: Arc<Mutex<Receiver<()>>>,
    
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Conflict resolution task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionTask {
    /// Task ID
    pub task_id: Uuid,
    /// Conflicting events
    pub conflicting_events: Vec<OrderedSemanticEvent>,
    /// Resolution strategy
    pub strategy: ConflictResolutionStrategy,
    /// Task creation time
    pub created_at: SystemTime,
    /// Task timeout
    pub timeout_ms: u64,
}

/// Ordering service commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderingCommand {
    /// Submit event for ordering
    SubmitEvent(OrderedSemanticEvent),
    /// Update process vector clock
    UpdateVectorClock {
        process_id: String,
        vector_clock: VectorClock,
    },
    /// Resolve conflict
    ResolveConflict {
        task_id: Uuid,
        resolution: ConflictResolution,
    },
    /// Detect sequence gaps
    DetectSequenceGaps,
    /// Process pending events
    ProcessPendingEvents,
    /// Get statistics
    GetStats,
    /// Shutdown service
    Shutdown,
}

/// Ordering service events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderingEvent {
    /// Event ordered and ready
    EventOrdered(OrderedSemanticEvent),
    /// Conflict detected
    ConflictDetected {
        task_id: Uuid,
        conflicting_events: Vec<EventId>,
    },
    /// Conflict resolved
    ConflictResolved {
        task_id: Uuid,
        resolution: ConflictResolution,
    },
    /// Sequence gap detected
    SequenceGapDetected(SequenceGap),
    /// Sequence gap resolved
    SequenceGapResolved {
        process_id: String,
        gap_size: u64,
    },
}

/// Conflict resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Resolution strategy used
    pub strategy: ConflictResolutionStrategy,
    /// Winning event (if any)
    pub winning_event: Option<EventId>,
    /// Merged event (if applicable)
    pub merged_event: Option<OrderedSemanticEvent>,
    /// Aborted events
    pub aborted_events: Vec<EventId>,
    /// Resolution timestamp
    pub resolved_at: SystemTime,
    /// Resolution reason
    pub reason: String,
}

impl EventOrderingService {
    /// Create new event ordering service
    pub fn new(config: EventOrderingConfig) -> Result<Self, VexfsError> {
        let (command_sender, command_receiver) = channel::unbounded();
        let (event_sender, event_receiver) = channel::unbounded();
        let (shutdown_sender, shutdown_receiver) = channel::unbounded();

        let service = Self {
            config: Arc::new(RwLock::new(config)),
            process_clocks: Arc::new(TokioRwLock::new(HashMap::new())),
            process_sequences: Arc::new(TokioRwLock::new(HashMap::new())),
            pending_events: Arc::new(TokioRwLock::new(VecDeque::new())),
            ordered_events: Arc::new(TokioRwLock::new(VecDeque::new())),
            sequence_gaps: Arc::new(TokioRwLock::new(HashMap::new())),
            conflict_queue: Arc::new(TokioRwLock::new(VecDeque::new())),
            stats: Arc::new(TokioRwLock::new(EventOrderingStats::default())),
            command_sender,
            command_receiver: Arc::new(Mutex::new(command_receiver)),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            shutdown_sender,
            shutdown_receiver: Arc::new(Mutex::new(shutdown_receiver)),
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        info!("Event ordering service created");
        Ok(service)
    }

    /// Start the ordering service
    pub async fn start(&self) -> Result<(), VexfsError> {
        info!("Starting event ordering service");

        let mut handles = self.task_handles.lock().unwrap();

        // Start command processor
        let command_task = self.spawn_command_processor().await?;
        handles.push(command_task);

        // Start event processor
        let event_task = self.spawn_event_processor().await?;
        handles.push(event_task);

        // Start ordering processor
        let ordering_task = self.spawn_ordering_processor().await?;
        handles.push(ordering_task);

        // Start conflict resolver
        let conflict_task = self.spawn_conflict_resolver().await?;
        handles.push(conflict_task);

        // Start sequence gap detector
        if self.config.read().unwrap().enable_sequence_gap_detection {
            let gap_task = self.spawn_sequence_gap_detector().await?;
            handles.push(gap_task);
        }

        info!("Event ordering service started with {} background tasks", handles.len());
        Ok(())
    }

    /// Stop the ordering service
    pub async fn stop(&self) -> Result<(), VexfsError> {
        info!("Stopping event ordering service");

        // Send shutdown signal
        if let Err(e) = self.shutdown_sender.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }

        // Wait for all background tasks to complete
        let handles = {
            let mut handles_guard = self.task_handles.lock().unwrap();
            std::mem::take(&mut *handles_guard)
        };

        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Background task failed to complete: {}", e);
            }
        }

        info!("Event ordering service stopped");
        Ok(())
    }

    /// Submit event for ordering
    pub async fn submit_event(&self, mut event: OrderedSemanticEvent) -> Result<(), VexfsError> {
        let start_time = Instant::now();

        // Update process vector clock
        let mut clocks = self.process_clocks.write().await;
        let process_clock = clocks.entry(event.process_id.clone())
            .or_insert_with(|| VectorClock::with_process(event.process_id.clone()));
        
        process_clock.increment(&event.process_id);
        event.vector_clock = process_clock.clone();

        // Update process sequence
        let mut sequences = self.process_sequences.write().await;
        let sequence = sequences.entry(event.process_id.clone()).or_insert(0);
        *sequence += 1;
        event.sequence_number = *sequence;

        drop(clocks);
        drop(sequences);

        // Send command to process the event
        if let Err(e) = self.command_sender.send(OrderingCommand::SubmitEvent(event)) {
            return Err(VexfsError::Internal(format!("Failed to submit event: {}", e)));
        }

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.events_processed += 1;
        
        let latency_us = start_time.elapsed().as_micros() as u64;
        stats.avg_ordering_latency_us = (stats.avg_ordering_latency_us + latency_us) / 2;
        stats.max_ordering_latency_us = stats.max_ordering_latency_us.max(latency_us);

        Ok(())
    }

    /// Get next ordered event
    pub async fn get_next_event(&self) -> Option<OrderedSemanticEvent> {
        let mut ordered = self.ordered_events.write().await;
        ordered.pop_front()
    }

    /// Get ordering statistics
    pub async fn get_stats(&self) -> EventOrderingStats {
        self.stats.read().await.clone()
    }

    /// Register new process
    pub async fn register_process(&self, process_id: String) -> Result<(), VexfsError> {
        let mut clocks = self.process_clocks.write().await;
        clocks.insert(process_id.clone(), VectorClock::with_process(process_id.clone()));
        
        let mut sequences = self.process_sequences.write().await;
        sequences.insert(process_id, 0);

        Ok(())
    }

    /// Spawn command processor task
    async fn spawn_command_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let command_receiver = Arc::clone(&self.command_receiver);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let pending_events = Arc::clone(&self.pending_events);
        let ordered_events = Arc::clone(&self.ordered_events);
        let conflict_queue = Arc::clone(&self.conflict_queue);
        let stats = Arc::clone(&self.stats);
        let event_sender = self.event_sender.clone();
        let config = Arc::clone(&self.config);

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Command processor received shutdown signal");
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            // No shutdown signal, continue
                        }
                        Err(TryRecvError::Disconnected) => {
                            warn!("Shutdown channel disconnected");
                            break;
                        }
                    }
                }

                // Process commands
                let command_to_process = if let Ok(command_receiver) = command_receiver.try_lock() {
                    match command_receiver.try_recv() {
                        Ok(command) => Some(command),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => {
                            warn!("Command channel disconnected");
                            break;
                        }
                    }
                } else {
                    None
                };

                if let Some(command) = command_to_process {
                    Self::process_command(
                        command,
                        &pending_events,
                        &ordered_events,
                        &conflict_queue,
                        &stats,
                        &event_sender,
                        &config,
                    ).await;
                } else {
                    sleep(Duration::from_millis(1)).await;
                }
            }

            debug!("Command processor task completed");
        });

        Ok(handle)
    }

    /// Spawn event processor task
    async fn spawn_event_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let event_receiver = Arc::clone(&self.event_receiver);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Event processor received shutdown signal");
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            // No shutdown signal, continue
                        }
                        Err(TryRecvError::Disconnected) => {
                            warn!("Shutdown channel disconnected");
                            break;
                        }
                    }
                }

                // Process events
                let event_to_process = if let Ok(event_receiver) = event_receiver.try_lock() {
                    match event_receiver.try_recv() {
                        Ok(event) => Some(event),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => {
                            warn!("Event channel disconnected");
                            break;
                        }
                    }
                } else {
                    None
                };

                if let Some(event) = event_to_process {
                    Self::process_event(event).await;
                } else {
                    sleep(Duration::from_millis(1)).await;
                }
            }

            debug!("Event processor task completed");
        });

        Ok(handle)
    }

    /// Spawn ordering processor task
    async fn spawn_ordering_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let pending_events = Arc::clone(&self.pending_events);
        let ordered_events = Arc::clone(&self.ordered_events);
        let conflict_queue = Arc::clone(&self.conflict_queue);
        let stats = Arc::clone(&self.stats);
        let event_sender = self.event_sender.clone();
        let config = Arc::clone(&self.config);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Ordering processor received shutdown signal");
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            // No shutdown signal, continue
                        }
                        Err(TryRecvError::Disconnected) => {
                            warn!("Shutdown channel disconnected");
                            break;
                        }
                    }
                }

                // Process pending events for ordering
                Self::process_pending_events(
                    &pending_events,
                    &ordered_events,
                    &conflict_queue,
                    &stats,
                    &event_sender,
                    &config,
                ).await;

                sleep(Duration::from_millis(1)).await;
            }

            debug!("Ordering processor task completed");
        });

        Ok(handle)
    }

    /// Spawn conflict resolver task
    async fn spawn_conflict_resolver(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let conflict_queue = Arc::clone(&self.conflict_queue);
        let stats = Arc::clone(&self.stats);
        let event_sender = self.event_sender.clone();
        let config = Arc::clone(&self.config);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Conflict resolver received shutdown signal");
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            // No shutdown signal, continue
                        }
                        Err(TryRecvError::Disconnected) => {
                            warn!("Shutdown channel disconnected");
                            break;
                        }
                    }
                }

                // Process conflict resolution tasks
                Self::process_conflict_resolution(
                    &conflict_queue,
                    &stats,
                    &event_sender,
                    &config,
                ).await;

                sleep(Duration::from_millis(10)).await;
            }

            debug!("Conflict resolver task completed");
        });

        Ok(handle)
    }

    /// Spawn sequence gap detector task
    async fn spawn_sequence_gap_detector(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let process_sequences = Arc::clone(&self.process_sequences);
        let sequence_gaps = Arc::clone(&self.sequence_gaps);
        let stats = Arc::clone(&self.stats);
        let event_sender = self.event_sender.clone();
        let config = Arc::clone(&self.config);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            loop {
                let interval = {
                    let config = config.read().unwrap();
                    Duration::from_millis(config.sequence_gap_timeout_ms)
                };
match timeout(interval, async {
    if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
        shutdown_receiver.recv()
    } else {
        Err(channel::RecvError)
    }
}).await {
    Ok(Ok(_)) => {
        debug!("Sequence gap detector received shutdown signal");
        break;
    }
    Ok(Err(_)) => {
        warn!("Shutdown channel disconnected");
        break;
    }
    Err(_) => {
        // Timeout, check for sequence gaps
        Self::detect_sequence_gaps(
            &process_sequences,
            &sequence_gaps,
            &stats,
            &event_sender,
        ).await;
    }
}
}

debug!("Sequence gap detector task completed");
});

Ok(handle)
}

/// Process command
async fn process_command(
command: OrderingCommand,
pending_events: &Arc<TokioRwLock<VecDeque<OrderedSemanticEvent>>>,
ordered_events: &Arc<TokioRwLock<VecDeque<OrderedSemanticEvent>>>,
conflict_queue: &Arc<TokioRwLock<VecDeque<ConflictResolutionTask>>>,
stats: &Arc<TokioRwLock<EventOrderingStats>>,
event_sender: &Sender<OrderingEvent>,
config: &Arc<RwLock<EventOrderingConfig>>,
) {
match command {
OrderingCommand::SubmitEvent(event) => {
debug!("Processing submit event command: {}", event.event_id());

// Add to pending events
let mut pending = pending_events.write().await;
pending.push_back(event);

// Update statistics
let mut stats_guard = stats.write().await;
stats_guard.pending_events += 1;
}
OrderingCommand::UpdateVectorClock { process_id, vector_clock } => {
debug!("Processing update vector clock command for process: {}", process_id);
// Vector clock updates are handled in submit_event
}
OrderingCommand::ResolveConflict { task_id, resolution } => {
debug!("Processing resolve conflict command: {}", task_id);

// Update statistics
let mut stats_guard = stats.write().await;
stats_guard.conflicts_resolved += 1;

if let Err(e) = event_sender.send(OrderingEvent::ConflictResolved { task_id, resolution }) {
    warn!("Failed to send conflict resolved event: {}", e);
}
}
OrderingCommand::DetectSequenceGaps => {
debug!("Processing detect sequence gaps command");
// Sequence gap detection is handled by background task
}
OrderingCommand::ProcessPendingEvents => {
debug!("Processing pending events command");
// Pending event processing is handled by background task
}
OrderingCommand::GetStats => {
debug!("Processing get stats command");
// Stats are accessible via shared reference
}
OrderingCommand::Shutdown => {
debug!("Processing shutdown command");
// Shutdown is handled by the main loop
}
}
}

/// Process event
async fn process_event(event: OrderingEvent) {
match event {
OrderingEvent::EventOrdered(event) => {
debug!("Processing event ordered: {}", event.event_id());
}
OrderingEvent::ConflictDetected { task_id, conflicting_events } => {
warn!("Processing conflict detected: {} events {:?}", task_id, conflicting_events);
}
OrderingEvent::ConflictResolved { task_id, resolution } => {
info!("Processing conflict resolved: {} strategy {:?}", task_id, resolution.strategy);
}
OrderingEvent::SequenceGapDetected(gap) => {
warn!("Processing sequence gap detected: process {} gap {}", gap.process_id, gap.gap_size);
}
OrderingEvent::SequenceGapResolved { process_id, gap_size } => {
info!("Processing sequence gap resolved: process {} gap {}", process_id, gap_size);
}
}
}

/// Process pending events for ordering
async fn process_pending_events(
pending_events: &Arc<TokioRwLock<VecDeque<OrderedSemanticEvent>>>,
ordered_events: &Arc<TokioRwLock<VecDeque<OrderedSemanticEvent>>>,
conflict_queue: &Arc<TokioRwLock<VecDeque<ConflictResolutionTask>>>,
stats: &Arc<TokioRwLock<EventOrderingStats>>,
event_sender: &Sender<OrderingEvent>,
config: &Arc<RwLock<EventOrderingConfig>>,
) {
let mut pending = pending_events.write().await;
let mut ordered = ordered_events.write().await;
let mut conflicts = conflict_queue.write().await;

if pending.is_empty() {
return;
}

let enable_conflict_detection = config.read().unwrap().enable_conflict_detection;
let mut events_to_order = Vec::new();
let mut conflicting_events = Vec::new();

// Collect events that can be ordered
while let Some(event) = pending.pop_front() {
let mut has_conflict = false;

if enable_conflict_detection {
// Check for conflicts with other pending events
for other_event in &events_to_order {
    if event.conflicts_with(other_event) {
        conflicting_events.push(event.clone());
        conflicting_events.push(other_event.clone());
        has_conflict = true;
        break;
    }
}
}

if !has_conflict {
events_to_order.push(event);
}
}

// Handle conflicts
if !conflicting_events.is_empty() {
let task_id = Uuid::new_v4();
let conflict_task = ConflictResolutionTask {
task_id,
conflicting_events: conflicting_events.clone(),
strategy: config.read().unwrap().default_conflict_resolution,
created_at: SystemTime::now(),
timeout_ms: config.read().unwrap().conflict_resolution_timeout_ms,
};

conflicts.push_back(conflict_task);

let mut stats_guard = stats.write().await;
stats_guard.conflicts_detected += 1;

let event_ids: Vec<EventId> = conflicting_events.iter().map(|e| e.event_id()).collect();
if let Err(e) = event_sender.send(OrderingEvent::ConflictDetected {
task_id,
conflicting_events: event_ids,
}) {
warn!("Failed to send conflict detected event: {}", e);
}
}

// Sort events by vector clock ordering
events_to_order.sort_by(|a, b| {
match a.vector_clock.compare(&b.vector_clock) {
VectorClockOrdering::Before => Ordering::Less,
VectorClockOrdering::After => Ordering::Greater,
VectorClockOrdering::Equal => a.sequence_number.cmp(&b.sequence_number),
VectorClockOrdering::Concurrent => {
    // For concurrent events, order by priority then sequence number
    match a.priority.cmp(&b.priority) {
        Ordering::Equal => a.sequence_number.cmp(&b.sequence_number),
        other => other,
    }
}
}
});

// Add ordered events to output queue
for event in events_to_order {
if let Err(e) = event_sender.send(OrderingEvent::EventOrdered(event.clone())) {
warn!("Failed to send event ordered: {}", e);
}
ordered.push_back(event);
}

// Update statistics
let mut stats_guard = stats.write().await;
stats_guard.pending_events = pending.len() as u64;
if !ordered.is_empty() {
stats_guard.events_reordered += ordered.len() as u64;
}
}

/// Process conflict resolution
async fn process_conflict_resolution(
conflict_queue: &Arc<TokioRwLock<VecDeque<ConflictResolutionTask>>>,
stats: &Arc<TokioRwLock<EventOrderingStats>>,
event_sender: &Sender<OrderingEvent>,
config: &Arc<RwLock<EventOrderingConfig>>,
) {
let mut conflicts = conflict_queue.write().await;

if conflicts.is_empty() {
return;
}

let current_time = SystemTime::now();
let mut resolved_tasks = Vec::new();

// Process each conflict resolution task
while let Some(task) = conflicts.pop_front() {
// Check if task has timed out
if let Ok(elapsed) = task.created_at.elapsed() {
if elapsed.as_millis() > task.timeout_ms as u128 {
    // Task timed out, use default resolution
    let resolution = Self::resolve_conflict_by_strategy(
        &task.conflicting_events,
        task.strategy,
        "Timeout - using default strategy".to_string(),
    );
    
    resolved_tasks.push((task.task_id, resolution));
    continue;
}
}

// Resolve conflict based on strategy
let resolution = Self::resolve_conflict_by_strategy(
&task.conflicting_events,
task.strategy,
"Automatic resolution".to_string(),
);

resolved_tasks.push((task.task_id, resolution));
}

// Send resolution events
for (task_id, resolution) in resolved_tasks {
if let Err(e) = event_sender.send(OrderingEvent::ConflictResolved { task_id, resolution }) {
warn!("Failed to send conflict resolved event: {}", e);
}
}
}

/// Resolve conflict by strategy
fn resolve_conflict_by_strategy(
conflicting_events: &[OrderedSemanticEvent],
strategy: ConflictResolutionStrategy,
reason: String,
) -> ConflictResolution {
match strategy {
ConflictResolutionStrategy::LastWriterWins => {
// Select event with latest timestamp
let winner = conflicting_events.iter()
    .max_by_key(|e| e.event.timestamp.seconds)
    .map(|e| e.event_id());

let aborted: Vec<EventId> = conflicting_events.iter()
    .filter(|e| Some(e.event_id()) != winner)
    .map(|e| e.event_id())
    .collect();

ConflictResolution {
    strategy,
    winning_event: winner,
    merged_event: None,
    aborted_events: aborted,
    resolved_at: SystemTime::now(),
    reason,
}
}
ConflictResolutionStrategy::FirstWriterWins => {
// Select event with earliest timestamp
let winner = conflicting_events.iter()
    .min_by_key(|e| e.event.timestamp.seconds)
    .map(|e| e.event_id());

let aborted: Vec<EventId> = conflicting_events.iter()
    .filter(|e| Some(e.event_id()) != winner)
    .map(|e| e.event_id())
    .collect();

ConflictResolution {
    strategy,
    winning_event: winner,
    merged_event: None,
    aborted_events: aborted,
    resolved_at: SystemTime::now(),
    reason,
}
}
ConflictResolutionStrategy::Priority => {
// Select event with highest priority
let winner = conflicting_events.iter()
    .min_by_key(|e| e.priority as u8)
    .map(|e| e.event_id());

let aborted: Vec<EventId> = conflicting_events.iter()
    .filter(|e| Some(e.event_id()) != winner)
    .map(|e| e.event_id())
    .collect();

ConflictResolution {
    strategy,
    winning_event: winner,
    merged_event: None,
    aborted_events: aborted,
    resolved_at: SystemTime::now(),
    reason,
}
}
ConflictResolutionStrategy::Abort => {
// Abort all conflicting events
let aborted: Vec<EventId> = conflicting_events.iter()
    .map(|e| e.event_id())
    .collect();

ConflictResolution {
    strategy,
    winning_event: None,
    merged_event: None,
    aborted_events: aborted,
    resolved_at: SystemTime::now(),
    reason,
}
}
ConflictResolutionStrategy::Merge => {
// TODO: Implement event merging logic
// For now, fall back to last writer wins
Self::resolve_conflict_by_strategy(
    conflicting_events,
    ConflictResolutionStrategy::LastWriterWins,
    format!("{} (merge not implemented, using last writer wins)", reason),
)
}
ConflictResolutionStrategy::Manual => {
// Manual resolution required - abort all for now
let aborted: Vec<EventId> = conflicting_events.iter()
    .map(|e| e.event_id())
    .collect();

ConflictResolution {
    strategy,
    winning_event: None,
    merged_event: None,
    aborted_events: aborted,
    resolved_at: SystemTime::now(),
    reason: format!("{} (manual resolution required)", reason),
}
}
}
}

/// Detect sequence gaps
async fn detect_sequence_gaps(
process_sequences: &Arc<TokioRwLock<HashMap<String, u64>>>,
sequence_gaps: &Arc<TokioRwLock<HashMap<String, Vec<SequenceGap>>>>,
stats: &Arc<TokioRwLock<EventOrderingStats>>,
event_sender: &Sender<OrderingEvent>,
) {
let sequences = process_sequences.read().await;
let mut gaps = sequence_gaps.write().await;

// For now, this is a placeholder implementation
// In a real system, we would track expected vs actual sequence numbers
// and detect gaps in the sequence

for (process_id, &current_sequence) in sequences.iter() {
// Simple gap detection logic - check for large jumps in sequence numbers
if let Some(process_gaps) = gaps.get_mut(process_id) {
// Check if we have any unresolved gaps that might now be resolved
process_gaps.retain(|gap| {
    let gap_age = gap.detected_at.elapsed().unwrap_or(Duration::from_secs(0));
    gap_age < Duration::from_secs(300) // Keep gaps for 5 minutes
});
}
}
}
}

#[cfg(test)]
mod tests {
use super::*;
use tokio::time::timeout;

#[tokio::test]
async fn test_vector_clock_ordering() {
let mut clock1 = VectorClock::with_process("process1".to_string());
let mut clock2 = VectorClock::with_process("process2".to_string());

// Initial state - concurrent
assert_eq!(clock1.compare(&clock2), VectorClockOrdering::Concurrent);

// Process 1 increments
clock1.increment("process1");
assert_eq!(clock1.compare(&clock2), VectorClockOrdering::After);
assert_eq!(clock2.compare(&clock1), VectorClockOrdering::Before);

// Process 2 updates with process 1's clock
clock2.update(&clock1, "process2");
assert_eq!(clock2.compare(&clock1), VectorClockOrdering::After);
}

#[tokio::test]
async fn test_event_ordering_service() {
let config = EventOrderingConfig::default();
let service = EventOrderingService::new(config).unwrap();

// Start the service
service.start().await.unwrap();

// Register processes
service.register_process("kernel".to_string()).await.unwrap();
service.register_process("userspace".to_string()).await.unwrap();

// Create test events
let event1 = create_test_event("kernel", 1);
let event2 = create_test_event("userspace", 1);

// Submit events
service.submit_event(event1).await.unwrap();
service.submit_event(event2).await.unwrap();

// Wait for processing
tokio::time::sleep(Duration::from_millis(100)).await;

// Check that events are ordered
let ordered_event = service.get_next_event().await;
assert!(ordered_event.is_some());

// Check statistics
let stats = service.get_stats().await;
assert_eq!(stats.events_processed, 2);

// Stop the service
service.stop().await.unwrap();
}

#[tokio::test]
async fn test_conflict_detection() {
let config = EventOrderingConfig::default();
let service = EventOrderingService::new(config).unwrap();

service.start().await.unwrap();
service.register_process("process1".to_string()).await.unwrap();

// Create conflicting events (same file path)
let mut event1 = create_test_event("process1", 1);
let mut event2 = create_test_event("process1", 2);

// Make them conflict by setting same file path and write operations
event1.event.event_type = SemanticEventType::FilesystemWrite;
event2.event.event_type = SemanticEventType::FilesystemWrite;

if let Some(ref mut fs_ctx) = event1.event.context.filesystem {
fs_ctx.path = "/test/file.txt".to_string();
}
if let Some(ref mut fs_ctx) = event2.event.context.filesystem {
fs_ctx.path = "/test/file.txt".to_string();
}

// Submit conflicting events
service.submit_event(event1).await.unwrap();
service.submit_event(event2).await.unwrap();

// Wait for processing
tokio::time::sleep(Duration::from_millis(100)).await;

// Check statistics for conflict detection
let stats = service.get_stats().await;
assert!(stats.conflicts_detected > 0 || stats.conflicts_resolved > 0);

service.stop().await.unwrap();
}

fn create_test_event(process_id: &str, sequence: u64) -> OrderedSemanticEvent {
let event = SemanticEvent::default();
let vector_clock = VectorClock::with_process(process_id.to_string());

OrderedSemanticEvent::new(
event,
process_id.to_string(),
sequence,
vector_clock,
)
}
}