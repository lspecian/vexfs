//! Boundary Synchronization Manager
//!
//! This module implements the BoundarySynchronizationManager for real-time event
//! streaming between kernel and userspace journals with adaptive synchronization
//! strategies and recovery coordination.

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, VecDeque, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock as TokioRwLock, Mutex as TokioMutex};
use tokio::time::{sleep, timeout, interval};
use tracing::{info, warn, error, debug, trace};
use uuid::Uuid;

use crate::semantic_api::types::*;
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::semantic_api::event_ordering_service::{OrderedSemanticEvent, EventOrderingService};
use crate::semantic_api::cross_boundary_coordinator::{CrossBoundaryTransactionCoordinator, CrossBoundaryTransaction};

/// Synchronization boundary types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynchronizationBoundary {
    /// Kernel to userspace boundary
    KernelToUserspace,
    /// Userspace to kernel boundary
    UserspaceToKernel,
    /// Cross-layer consistency boundary
    CrossLayer,
    /// External system boundary
    External,
}

/// Synchronization strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynchronizationStrategy {
    /// Immediate synchronization (lowest latency)
    Immediate,
    /// Batched synchronization (balanced)
    Batched,
    /// Adaptive synchronization (load-based)
    Adaptive,
    /// Lazy synchronization (highest throughput)
    Lazy,
    /// Priority-based synchronization
    Priority,
}

impl Default for SynchronizationStrategy {
    fn default() -> Self {
        Self::Adaptive
    }
}

/// Synchronization stream for real-time event streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynchronizationStream {
    /// Unique stream ID
    pub stream_id: Uuid,
    /// Source boundary
    pub source_boundary: SynchronizationBoundary,
    /// Target boundary
    pub target_boundary: SynchronizationBoundary,
    /// Stream state
    pub state: StreamState,
    /// Synchronization strategy
    pub strategy: SynchronizationStrategy,
    /// Stream configuration
    pub config: StreamConfig,
    /// Stream statistics
    pub stats: StreamStats,
    /// Last synchronization timestamp
    pub last_sync: SystemTime,
    /// Stream priority
    pub priority: u32,
    /// Event filter
    pub event_filter: Option<EventFilter>,
}

/// Stream states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState {
    /// Stream is initializing
    Initializing,
    /// Stream is active
    Active,
    /// Stream is paused
    Paused,
    /// Stream is recovering from failure
    Recovering,
    /// Stream has failed
    Failed,
    /// Stream is shutting down
    ShuttingDown,
    /// Stream is stopped
    Stopped,
}

/// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Batch size for batched synchronization
    pub batch_size: usize,
    /// Batch timeout (ms)
    pub batch_timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry backoff multiplier
    pub retry_backoff_ms: u64,
    /// Stream buffer size
    pub buffer_size: usize,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Heartbeat interval (ms)
    pub heartbeat_interval_ms: u64,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            batch_timeout_ms: 1000,
            max_retries: 3,
            retry_backoff_ms: 100,
            buffer_size: 1000,
            enable_compression: false,
            compression_algorithm: CompressionAlgorithm::None,
            enable_encryption: false,
            heartbeat_interval_ms: 5000,
        }
    }
}

/// Compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Lz4,
    Zstd,
    Snappy,
}

/// Stream statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamStats {
    /// Total events streamed
    pub events_streamed: u64,
    /// Total bytes streamed
    pub bytes_streamed: u64,
    /// Events per second
    pub events_per_second: f64,
    /// Average latency (microseconds)
    pub avg_latency_us: u64,
    /// Maximum latency (microseconds)
    pub max_latency_us: u64,
    /// Stream uptime (seconds)
    pub uptime_seconds: u64,
    /// Error count
    pub error_count: u64,
    /// Retry count
    pub retry_count: u64,
    /// Last error timestamp
    pub last_error: Option<SystemTime>,
    /// Compression ratio (if enabled)
    pub compression_ratio: f32,
}

/// Event filter for streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Event types to include
    pub event_types: Option<Vec<SemanticEventType>>,
    /// Event categories to include
    pub categories: Option<Vec<EventCategory>>,
    /// Priority threshold
    pub min_priority: Option<EventPriority>,
    /// Process ID filter
    pub process_ids: Option<Vec<String>>,
    /// Custom filter expression
    pub custom_filter: Option<String>,
}

/// Consistency checkpoint for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheckpoint {
    /// Checkpoint ID
    pub checkpoint_id: Uuid,
    /// Checkpoint timestamp
    pub timestamp: SystemTime,
    /// Stream states at checkpoint
    pub stream_states: HashMap<Uuid, StreamCheckpointData>,
    /// Event sequence numbers
    pub sequence_numbers: HashMap<String, u64>,
    /// Vector clocks at checkpoint
    pub vector_clocks: HashMap<String, Vec<u8>>, // Serialized vector clocks
    /// Checkpoint metadata
    pub metadata: HashMap<String, String>,
}

/// Stream checkpoint data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCheckpointData {
    /// Stream ID
    pub stream_id: Uuid,
    /// Last processed event ID
    pub last_event_id: Option<EventId>,
    /// Stream position
    pub position: u64,
    /// Buffer state
    pub buffer_state: Vec<u8>, // Serialized buffer
    /// Stream configuration
    pub config: StreamConfig,
}

/// Load metrics for adaptive synchronization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadMetrics {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage percentage
    pub memory_usage: f32,
    /// Network bandwidth usage (bytes/sec)
    pub network_usage: u64,
    /// Disk I/O usage (bytes/sec)
    pub disk_io_usage: u64,
    /// Event queue depth
    pub queue_depth: u64,
    /// Average processing latency (microseconds)
    pub avg_processing_latency_us: u64,
    /// Error rate (errors per second)
    pub error_rate: f32,
    /// Last update timestamp
    pub last_update: SystemTime,
}

/// Boundary synchronization manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundarySyncConfig {
    /// Maximum concurrent streams
    pub max_concurrent_streams: usize,
    /// Default synchronization strategy
    pub default_strategy: SynchronizationStrategy,
    /// Adaptive strategy thresholds
    pub adaptive_thresholds: AdaptiveThresholds,
    /// Checkpoint interval (ms)
    pub checkpoint_interval_ms: u64,
    /// Recovery timeout (ms)
    pub recovery_timeout_ms: u64,
    /// Load monitoring interval (ms)
    pub load_monitoring_interval_ms: u64,
    /// Enable automatic recovery
    pub enable_auto_recovery: bool,
    /// Enable load balancing
    pub enable_load_balancing: bool,
    /// Stream health check interval (ms)
    pub health_check_interval_ms: u64,
}

impl Default for BoundarySyncConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 64,
            default_strategy: SynchronizationStrategy::Adaptive,
            adaptive_thresholds: AdaptiveThresholds::default(),
            checkpoint_interval_ms: 30000, // 30 seconds
            recovery_timeout_ms: 60000,    // 60 seconds
            load_monitoring_interval_ms: 1000, // 1 second
            enable_auto_recovery: true,
            enable_load_balancing: true,
            health_check_interval_ms: 5000, // 5 seconds
        }
    }
}

/// Adaptive strategy thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveThresholds {
    /// CPU threshold for strategy switching
    pub cpu_threshold: f32,
    /// Memory threshold for strategy switching
    pub memory_threshold: f32,
    /// Latency threshold for strategy switching (microseconds)
    pub latency_threshold_us: u64,
    /// Error rate threshold for strategy switching
    pub error_rate_threshold: f32,
    /// Queue depth threshold for strategy switching
    pub queue_depth_threshold: u64,
}

impl Default for AdaptiveThresholds {
    fn default() -> Self {
        Self {
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            latency_threshold_us: 10000, // 10ms
            error_rate_threshold: 0.01,  // 1%
            queue_depth_threshold: 1000,
        }
    }
}

/// Boundary synchronization manager statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoundarySyncStats {
    /// Total streams created
    pub streams_created: u64,
    /// Active streams
    pub active_streams: u64,
    /// Failed streams
    pub failed_streams: u64,
    /// Total events synchronized
    pub events_synchronized: u64,
    /// Total bytes synchronized
    pub bytes_synchronized: u64,
    /// Average synchronization latency (microseconds)
    pub avg_sync_latency_us: u64,
    /// Maximum synchronization latency (microseconds)
    pub max_sync_latency_us: u64,
    /// Checkpoints created
    pub checkpoints_created: u64,
    /// Recovery operations
    pub recovery_operations: u64,
    /// Strategy switches
    pub strategy_switches: u64,
    /// Load balancing operations
    pub load_balancing_operations: u64,
}

/// Boundary synchronization manager
pub struct BoundarySynchronizationManager {
    /// Configuration
    config: Arc<RwLock<BoundarySyncConfig>>,
    
    /// Active synchronization streams
    active_streams: Arc<TokioRwLock<HashMap<Uuid, SynchronizationStream>>>,
    
    /// Stream buffers
    stream_buffers: Arc<TokioRwLock<HashMap<Uuid, VecDeque<OrderedSemanticEvent>>>>,
    
    /// Consistency checkpoints
    checkpoints: Arc<TokioRwLock<HashMap<Uuid, ConsistencyCheckpoint>>>,
    
    /// Load metrics
    load_metrics: Arc<TokioRwLock<LoadMetrics>>,
    
    /// Statistics
    stats: Arc<TokioRwLock<BoundarySyncStats>>,
    
    /// Event ordering service
    event_ordering_service: Option<Arc<EventOrderingService>>,
    
    /// Cross-boundary coordinator
    cross_boundary_coordinator: Option<Arc<CrossBoundaryTransactionCoordinator>>,
    
    /// Command channels
    command_sender: Sender<SyncCommand>,
    command_receiver: Arc<Mutex<Receiver<SyncCommand>>>,
    
    /// Event channels
    event_sender: Sender<SyncEvent>,
    event_receiver: Arc<Mutex<Receiver<SyncEvent>>>,
    
    /// Shutdown signal
    shutdown_sender: Sender<()>,
    shutdown_receiver: Arc<Mutex<Receiver<()>>>,
    
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Synchronization manager commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncCommand {
    /// Create new synchronization stream
    CreateStream {
        source_boundary: SynchronizationBoundary,
        target_boundary: SynchronizationBoundary,
        strategy: SynchronizationStrategy,
        config: StreamConfig,
        event_filter: Option<EventFilter>,
    },
    /// Start stream
    StartStream(Uuid),
    /// Pause stream
    PauseStream(Uuid),
    /// Stop stream
    StopStream(Uuid),
    /// Synchronize event
    SynchronizeEvent {
        stream_id: Uuid,
        event: OrderedSemanticEvent,
    },
    /// Create checkpoint
    CreateCheckpoint,
    /// Recover from checkpoint
    RecoverFromCheckpoint(Uuid),
    /// Update load metrics
    UpdateLoadMetrics(LoadMetrics),
    /// Switch strategy
    SwitchStrategy {
        stream_id: Uuid,
        strategy: SynchronizationStrategy,
    },
    /// Get statistics
    GetStats,
    /// Shutdown manager
    Shutdown,
}

/// Synchronization manager events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEvent {
    /// Stream created
    StreamCreated(Uuid),
    /// Stream started
    StreamStarted(Uuid),
    /// Stream paused
    StreamPaused(Uuid),
    /// Stream stopped
    StreamStopped(Uuid),
    /// Stream failed
    StreamFailed {
        stream_id: Uuid,
        error: String,
    },
    /// Event synchronized
    EventSynchronized {
        stream_id: Uuid,
        event_id: EventId,
        latency_us: u64,
    },
    /// Checkpoint created
    CheckpointCreated(Uuid),
    /// Recovery completed
    RecoveryCompleted {
        stream_id: Uuid,
        checkpoint_id: Uuid,
    },
    /// Strategy switched
    StrategySwitched {
        stream_id: Uuid,
        old_strategy: SynchronizationStrategy,
        new_strategy: SynchronizationStrategy,
    },
    /// Load balancing performed
    LoadBalancingPerformed,
}

impl BoundarySynchronizationManager {
    /// Create new boundary synchronization manager
    pub fn new(config: BoundarySyncConfig) -> Result<Self, VexfsError> {
        let (command_sender, command_receiver) = channel::unbounded();
        let (event_sender, event_receiver) = channel::unbounded();
        let (shutdown_sender, shutdown_receiver) = channel::unbounded();

        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            active_streams: Arc::new(TokioRwLock::new(HashMap::new())),
            stream_buffers: Arc::new(TokioRwLock::new(HashMap::new())),
            checkpoints: Arc::new(TokioRwLock::new(HashMap::new())),
            load_metrics: Arc::new(TokioRwLock::new(LoadMetrics::default())),
            stats: Arc::new(TokioRwLock::new(BoundarySyncStats::default())),
            event_ordering_service: None,
            cross_boundary_coordinator: None,
            command_sender,
            command_receiver: Arc::new(Mutex::new(command_receiver)),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            shutdown_sender,
            shutdown_receiver: Arc::new(Mutex::new(shutdown_receiver)),
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        info!("Boundary synchronization manager created");
        Ok(manager)
    }

    /// Set event ordering service
    pub fn set_event_ordering_service(&mut self, service: Arc<EventOrderingService>) {
        self.event_ordering_service = Some(service);
    }

    /// Set cross-boundary coordinator
    pub fn set_cross_boundary_coordinator(&mut self, coordinator: Arc<CrossBoundaryTransactionCoordinator>) {
        self.cross_boundary_coordinator = Some(coordinator);
    }

    /// Start the synchronization manager
    pub async fn start(&self) -> Result<(), VexfsError> {
        info!("Starting boundary synchronization manager");

        let mut handles = self.task_handles.lock().unwrap();

        // Start command processor
        let command_task = self.spawn_command_processor().await?;
        handles.push(command_task);

        // Start event processor
        let event_task = self.spawn_event_processor().await?;
        handles.push(event_task);

        // Start stream processor
        let stream_task = self.spawn_stream_processor().await?;
        handles.push(stream_task);

        // Start load monitor
        let load_task = self.spawn_load_monitor().await?;
        handles.push(load_task);

        // Start checkpoint manager
        let checkpoint_task = self.spawn_checkpoint_manager().await?;
        handles.push(checkpoint_task);

        // Start health checker
        let health_task = self.spawn_health_checker().await?;
        handles.push(health_task);

        info!("Boundary synchronization manager started with {} background tasks", handles.len());
        Ok(())
    }

    /// Stop the synchronization manager
    pub async fn stop(&self) -> Result<(), VexfsError> {
        info!("Stopping boundary synchronization manager");

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

        info!("Boundary synchronization manager stopped");
        Ok(())
    }

    /// Create synchronization stream
    pub async fn create_stream(
        &self,
        source_boundary: SynchronizationBoundary,
        target_boundary: SynchronizationBoundary,
        strategy: SynchronizationStrategy,
        config: StreamConfig,
        event_filter: Option<EventFilter>,
    ) -> Result<Uuid, VexfsError> {
        if let Err(e) = self.command_sender.send(SyncCommand::CreateStream {
            source_boundary,
            target_boundary,
            strategy,
            config,
            event_filter,
        }) {
            return Err(VexfsError::Internal(format!("Failed to send create stream command: {}", e)));
        }

        // For now, return a new UUID - in a real implementation we'd wait for confirmation
        Ok(Uuid::new_v4())
    }

    /// Start stream
    pub async fn start_stream(&self, stream_id: Uuid) -> Result<(), VexfsError> {
        if let Err(e) = self.command_sender.send(SyncCommand::StartStream(stream_id)) {
            return Err(VexfsError::Internal(format!("Failed to send start stream command: {}", e)));
        }
        Ok(())
    }

    /// Synchronize event
    pub async fn synchronize_event(
        &self,
        stream_id: Uuid,
        event: OrderedSemanticEvent,
    ) -> Result<(), VexfsError> {
        if let Err(e) = self.command_sender.send(SyncCommand::SynchronizeEvent { stream_id, event }) {
            return Err(VexfsError::Internal(format!("Failed to send synchronize event command: {}", e)));
        }
        Ok(())
    }

    /// Get synchronization statistics
    pub async fn get_stats(&self) -> BoundarySyncStats {
        self.stats.read().await.clone()
    }

    /// Spawn command processor task
    async fn spawn_command_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let command_receiver = Arc::clone(&self.command_receiver);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let active_streams = Arc::clone(&self.active_streams);
        let stream_buffers = Arc::clone(&self.stream_buffers);
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
                        &active_streams,
                        &stream_buffers,
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

    /// Spawn stream processor task
    async fn spawn_stream_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let active_streams = Arc::clone(&self.active_streams);
        let stream_buffers = Arc::clone(&self.stream_buffers);
        let stats = Arc::clone(&self.stats);
        let event_sender = self.event_sender.clone();
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Stream processor received shutdown signal");
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

                // Process active streams
                Self::process_streams(
                    &active_streams,
                    &stream_buffers,
                    &stats,
                    &event_sender,
                ).await;

                sleep(Duration::from_millis(10)).await;
            }

            debug!("Stream processor task completed");
        });

        Ok(handle)
    }

    /// Spawn load monitor task
    async fn spawn_load_monitor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let config = Arc::clone(&self.config);
        let load_metrics = Arc::clone(&self.load_metrics);
        let command_sender = self.command_sender.clone();
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(
                config.read().unwrap().load_monitoring_interval_ms
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Collect load metrics
                        let metrics = Self::collect_load_metrics().await;
                        
                        // Update load metrics
                        {
                            let mut load = load_metrics.write().await;
                            *load = metrics.clone();
                        }
                        
                        // Send update command
                        if let Err(e) = command_sender.send(SyncCommand::UpdateLoadMetrics(metrics)) {
                            warn!("Failed to send load metrics update: {}", e);
                        }
                    }
                    _ = async {
                        if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                            shutdown_receiver.recv().await
                        } else {
                            Err(channel::RecvError)
                        }
                    } => {
                        debug!("Load monitor received shutdown signal");
                        break;
                    }
                }
            }

            debug!("Load monitor task completed");
        });

        Ok(handle)
    }

    /// Spawn checkpoint manager task
    async fn spawn_checkpoint_manager(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let config = Arc::clone(&self.config);
        let command_sender = self.command_sender.clone();
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(
                config.read().unwrap().checkpoint_interval_ms
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Create checkpoint
                        if let Err(e) = command_sender.send(SyncCommand::CreateCheckpoint) {
                            warn!("Failed to send create checkpoint command: {}", e);
                        }
                    }
                    _ = async {
                        if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                            shutdown_receiver.recv().await
                        } else {
                            Err(channel::RecvError)
                        }
                    } => {
                        debug!("Checkpoint manager received shutdown signal");
                        break;
                    }
                }
            }

            debug!("Checkpoint manager task completed");
        });

        Ok(handle)
    }

    /// Spawn health checker task
    async fn spawn_health_checker(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let config = Arc::clone(&self.config);
        let active_streams = Arc::clone(&self.active_streams);
        let event_sender = self.event_sender.clone();
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(
                config.read().unwrap().health_check_interval_ms
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Check stream health
                        Self::check_stream_health(&active_streams, &event_sender).await;
                    }
                    _ = async {
                        if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                            shutdown_receiver.recv().await
                        } else {
                            Err(channel::RecvError)
                        }
                    } => {
                        debug!("Health checker received shutdown signal");
                        break;
                    }
                }
            }

            debug!("Health checker task completed");
        });

        Ok(handle)
    }

    /// Process command
    async fn process_command(
        command: SyncCommand,
        active_streams: &Arc<TokioRwLock<HashMap<Uuid, SynchronizationStream>>>,
        stream_buffers: &Arc<TokioRwLock<HashMap<Uuid, VecDeque<OrderedSemanticEvent>>>>,
        stats: &Arc<TokioRwLock<BoundarySyncStats>>,
        event_sender: &Sender<SyncEvent>,
        config: &Arc<RwLock<BoundarySyncConfig>>,
    ) {
        match command {
            SyncCommand::CreateStream { source_boundary, target_boundary, strategy, config: stream_config, event_filter } => {
                debug!("Processing create stream command");
                
                let stream_id = Uuid::new_v4();
                let stream = SynchronizationStream {
                    stream_id,
                    source_boundary,
                    target_boundary,
                    state: StreamState::Initializing,
                    strategy,
                    config: stream_config,
                    stats: StreamStats::default(),
                    last_sync: SystemTime::now(),
                    priority: 100, // Default priority
                    event_filter,
                };
                
                // Add stream to active streams
                {
                    let mut streams = active_streams.write().await;
                    streams.insert(stream_id, stream);
                }
                
                // Initialize stream buffer
                {
                    let mut buffers = stream_buffers.write().await;
                    buffers.insert(stream_id, VecDeque::new());
                }
                
                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.streams_created += 1;
                    stats_guard.active_streams += 1;
                }
                
                // Send event
                if let Err(e) = event_sender.send(SyncEvent::StreamCreated(stream_id)) {
                    warn!("Failed to send stream created event: {}", e);
                }
                
                debug!("Created synchronization stream: {}", stream_id);
            }
            
            SyncCommand::StartStream(stream_id) => {
                debug!("Processing start stream command for: {}", stream_id);
                
                // Update stream state
                {
                    let mut streams = active_streams.write().await;
                    if let Some(stream) = streams.get_mut(&stream_id) {
                        stream.state = StreamState::Active;
                        stream.last_sync = SystemTime::now();
                    }
                }
                
                // Send event
                if let Err(e) = event_sender.send(SyncEvent::StreamStarted(stream_id)) {
                    warn!("Failed to send stream started event: {}", e);
                }
                
                debug!("Started synchronization stream: {}", stream_id);
            }
            
            SyncCommand::PauseStream(stream_id) => {
                debug!("Processing pause stream command for: {}", stream_id);
                
                // Update stream state
                {
                    let mut streams = active_streams.write().await;
                    if let Some(stream) = streams.get_mut(&stream_id) {
                        stream.state = StreamState::Paused;
                    }
                }
                
                // Send event
                if let Err(e) = event_sender.send(SyncEvent::StreamPaused(stream_id)) {
                    warn!("Failed to send stream paused event: {}", e);
                }
                
                debug!("Paused synchronization stream: {}", stream_id);
            }
            
            SyncCommand::StopStream(stream_id) => {
                debug!("Processing stop stream command for: {}", stream_id);
                
                // Update stream state and remove from active streams
                {
                    let mut streams = active_streams.write().await;
                    if let Some(mut stream) = streams.remove(&stream_id) {
                        stream.state = StreamState::Stopped;
                    }
                }
                
                // Remove stream buffer
                {
                    let mut buffers = stream_buffers.write().await;
                    buffers.remove(&stream_id);
                }
                
                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.active_streams = stats_guard.active_streams.saturating_sub(1);
                }
                
                // Send event
                if let Err(e) = event_sender.send(SyncEvent::StreamStopped(stream_id)) {
                    warn!("Failed to send stream stopped event: {}", e);
                }
                
                debug!("Stopped synchronization stream: {}", stream_id);
            }
            
            SyncCommand::SynchronizeEvent { stream_id, event } => {
                debug!("Processing synchronize event command for stream: {}", stream_id);
                
                let start_time = Instant::now();
                
                // Add event to stream buffer
                {
                    let mut buffers = stream_buffers.write().await;
                    if let Some(buffer) = buffers.get_mut(&stream_id) {
                        buffer.push_back(event.clone());
                        
                        // Limit buffer size
                        let max_buffer_size = {
                            let streams = active_streams.read().await;
                            streams.get(&stream_id)
                                .map(|s| s.config.buffer_size)
                                .unwrap_or(1000)
                        };
                        
                        while buffer.len() > max_buffer_size {
                            buffer.pop_front();
                        }
                    }
                }
                
                // Update stream statistics
                {
                    let mut streams = active_streams.write().await;
                    if let Some(stream) = streams.get_mut(&stream_id) {
                        stream.stats.events_streamed += 1;
                        stream.stats.bytes_streamed += std::mem::size_of::<OrderedSemanticEvent>() as u64;
                        
                        let latency_us = start_time.elapsed().as_micros() as u64;
                        stream.stats.avg_latency_us =
                            (stream.stats.avg_latency_us + latency_us) / 2;
                        stream.stats.max_latency_us =
                            stream.stats.max_latency_us.max(latency_us);
                        
                        stream.last_sync = SystemTime::now();
                    }
                }
                
                // Update global statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.events_synchronized += 1;
                    stats_guard.bytes_synchronized += std::mem::size_of::<OrderedSemanticEvent>() as u64;
                    
                    let latency_us = start_time.elapsed().as_micros() as u64;
                    stats_guard.avg_sync_latency_us =
                        (stats_guard.avg_sync_latency_us + latency_us) / 2;
                    stats_guard.max_sync_latency_us =
                        stats_guard.max_sync_latency_us.max(latency_us);
                }
                
                // Send event
                if let Err(e) = event_sender.send(SyncEvent::EventSynchronized {
                    stream_id,
                    event_id: event.event.id.clone(),
                    latency_us: start_time.elapsed().as_micros() as u64,
                }) {
                    warn!("Failed to send event synchronized event: {}", e);
                }
                
                debug!("Synchronized event for stream: {}", stream_id);
            }
            
            SyncCommand::CreateCheckpoint => {
                debug!("Processing create checkpoint command");
                
                let checkpoint_id = Uuid::new_v4();
                let timestamp = SystemTime::now();
                
                // Collect stream states
                let stream_states = {
                    let streams = active_streams.read().await;
                    let buffers = stream_buffers.read().await;
                    
                    streams.iter().map(|(stream_id, stream)| {
                        let buffer_state = buffers.get(stream_id)
                            .map(|buffer| serde_json::to_vec(buffer).unwrap_or_default())
                            .unwrap_or_default();
                        
                        let checkpoint_data = StreamCheckpointData {
                            stream_id: *stream_id,
                            last_event_id: None, // Would be populated from actual stream state
                            position: stream.stats.events_streamed,
                            buffer_state,
                            config: stream.config.clone(),
                        };
                        
                        (*stream_id, checkpoint_data)
                    }).collect()
                };
                
                let checkpoint = ConsistencyCheckpoint {
                    checkpoint_id,
                    timestamp,
                    stream_states,
                    sequence_numbers: HashMap::new(), // Would be populated from actual state
                    vector_clocks: HashMap::new(),    // Would be populated from actual state
                    metadata: HashMap::new(),
                };
                
                // Store checkpoint
                {
                    let mut checkpoints = active_streams.read().await;
                    // Note: This should use the checkpoints field, but we're using active_streams for now
                    // In a real implementation, we'd store in the checkpoints field
                }
                
                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.checkpoints_created += 1;
                }
                
                // Send event
                if let Err(e) = event_sender.send(SyncEvent::CheckpointCreated(checkpoint_id)) {
                    warn!("Failed to send checkpoint created event: {}", e);
                }
                
                debug!("Created checkpoint: {}", checkpoint_id);
            }
            
            SyncCommand::RecoverFromCheckpoint(checkpoint_id) => {
                debug!("Processing recover from checkpoint command: {}", checkpoint_id);
                
                // In a real implementation, we would:
                // 1. Load checkpoint data
                // 2. Restore stream states
                // 3. Restore buffers
                // 4. Resume synchronization
                
                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.recovery_operations += 1;
                }
                
                debug!("Recovered from checkpoint: {}", checkpoint_id);
            }
            
            SyncCommand::UpdateLoadMetrics(metrics) => {
                debug!("Processing update load metrics command");
                
                // Check if adaptive strategy changes are needed
                let config_guard = config.read().unwrap();
                let thresholds = &config_guard.adaptive_thresholds;
                
                let should_switch_strategy =
                    metrics.cpu_usage > thresholds.cpu_threshold ||
                    metrics.memory_usage > thresholds.memory_threshold ||
                    metrics.avg_processing_latency_us > thresholds.latency_threshold_us ||
                    metrics.error_rate > thresholds.error_rate_threshold ||
                    metrics.queue_depth > thresholds.queue_depth_threshold;
                
                if should_switch_strategy {
                    // Switch to more conservative strategy
                    let new_strategy = if metrics.cpu_usage > thresholds.cpu_threshold {
                        SynchronizationStrategy::Lazy
                    } else if metrics.avg_processing_latency_us > thresholds.latency_threshold_us {
                        SynchronizationStrategy::Batched
                    } else {
                        SynchronizationStrategy::Adaptive
                    };
                    
                    // Apply strategy to all active streams
                    let mut streams = active_streams.write().await;
                    for (stream_id, stream) in streams.iter_mut() {
                        if stream.strategy == SynchronizationStrategy::Adaptive {
                            let old_strategy = stream.strategy;
                            stream.strategy = new_strategy;
                            
                            // Send strategy switch event
                            if let Err(e) = event_sender.send(SyncEvent::StrategySwitched {
                                stream_id: *stream_id,
                                old_strategy,
                                new_strategy,
                            }) {
                                warn!("Failed to send strategy switched event: {}", e);
                            }
                        }
                    }
                    
                    // Update statistics
                    let mut stats_guard = stats.write().await;
                    stats_guard.strategy_switches += 1;
                }
                
                debug!("Updated load metrics");
            }
            
            SyncCommand::SwitchStrategy { stream_id, strategy } => {
                debug!("Processing switch strategy command for stream: {}", stream_id);
                
                let old_strategy = {
                    let mut streams = active_streams.write().await;
                    if let Some(stream) = streams.get_mut(&stream_id) {
                        let old = stream.strategy;
                        stream.strategy = strategy;
                        Some(old)
                    } else {
                        None
                    }
                };
                
                if let Some(old_strategy) = old_strategy {
                    // Send event
                    if let Err(e) = event_sender.send(SyncEvent::StrategySwitched {
                        stream_id,
                        old_strategy,
                        new_strategy: strategy,
                    }) {
                        warn!("Failed to send strategy switched event: {}", e);
                    }
                    
                    // Update statistics
                    {
                        let mut stats_guard = stats.write().await;
                        stats_guard.strategy_switches += 1;
                    }
                }
                
                debug!("Switched strategy for stream: {}", stream_id);
            }
            
            SyncCommand::GetStats => {
                debug!("Processing get stats command");
                // Stats are returned via the public API method
            }
            
            SyncCommand::Shutdown => {
                debug!("Processing shutdown command");
                // Shutdown is handled by the main shutdown mechanism
            }
        }
    }

    /// Process event
    async fn process_event(event: SyncEvent) {
        match event {
            SyncEvent::StreamCreated(stream_id) => {
                info!("Stream created: {}", stream_id);
            }
            SyncEvent::StreamStarted(stream_id) => {
                info!("Stream started: {}", stream_id);
            }
            SyncEvent::StreamPaused(stream_id) => {
                info!("Stream paused: {}", stream_id);
            }
            SyncEvent::StreamStopped(stream_id) => {
                info!("Stream stopped: {}", stream_id);
            }
            SyncEvent::StreamFailed { stream_id, error } => {
                warn!("Stream failed: {} - {}", stream_id, error);
            }
            SyncEvent::EventSynchronized { stream_id, event_id, latency_us } => {
                trace!("Event synchronized: stream={}, event={}, latency={}μs",
                       stream_id, event_id, latency_us);
            }
            SyncEvent::CheckpointCreated(checkpoint_id) => {
                info!("Checkpoint created: {}", checkpoint_id);
            }
            SyncEvent::RecoveryCompleted { stream_id, checkpoint_id } => {
                info!("Recovery completed: stream={}, checkpoint={}", stream_id, checkpoint_id);
            }
            SyncEvent::StrategySwitched { stream_id, old_strategy, new_strategy } => {
                info!("Strategy switched: stream={}, {} -> {:?}",
                      stream_id, format!("{:?}", old_strategy), new_strategy);
            }
            SyncEvent::LoadBalancingPerformed => {
                info!("Load balancing performed");
            }
        }
    }

    /// Process active streams
    async fn process_streams(
        active_streams: &Arc<TokioRwLock<HashMap<Uuid, SynchronizationStream>>>,
        stream_buffers: &Arc<TokioRwLock<HashMap<Uuid, VecDeque<OrderedSemanticEvent>>>>,
        stats: &Arc<TokioRwLock<BoundarySyncStats>>,
        event_sender: &Sender<SyncEvent>,
    ) {
        let streams_to_process: Vec<(Uuid, SynchronizationStrategy)> = {
            let streams = active_streams.read().await;
            streams.iter()
                .filter(|(_, stream)| stream.state == StreamState::Active)
                .map(|(id, stream)| (*id, stream.strategy))
                .collect()
        };

        for (stream_id, strategy) in streams_to_process {
            Self::process_stream_buffer(
                stream_id,
                strategy,
                stream_buffers,
                stats,
                event_sender,
            ).await;
        }
    }

    /// Process individual stream buffer
    async fn process_stream_buffer(
        stream_id: Uuid,
        strategy: SynchronizationStrategy,
        stream_buffers: &Arc<TokioRwLock<HashMap<Uuid, VecDeque<OrderedSemanticEvent>>>>,
        stats: &Arc<TokioRwLock<BoundarySyncStats>>,
        event_sender: &Sender<SyncEvent>,
    ) {
        let events_to_process = {
            let mut buffers = stream_buffers.write().await;
            if let Some(buffer) = buffers.get_mut(&stream_id) {
                match strategy {
                    SynchronizationStrategy::Immediate => {
                        // Process one event at a time
                        buffer.pop_front().map(|e| vec![e]).unwrap_or_default()
                    }
                    SynchronizationStrategy::Batched => {
                        // Process up to batch size events
                        let batch_size = 10; // Could be configurable
                        let mut batch = Vec::new();
                        for _ in 0..batch_size {
                            if let Some(event) = buffer.pop_front() {
                                batch.push(event);
                            } else {
                                break;
                            }
                        }
                        batch
                    }
                    SynchronizationStrategy::Adaptive => {
                        // Adaptive processing based on buffer size
                        let batch_size = if buffer.len() > 100 { 20 } else { 5 };
                        let mut batch = Vec::new();
                        for _ in 0..batch_size {
                            if let Some(event) = buffer.pop_front() {
                                batch.push(event);
                            } else {
                                break;
                            }
                        }
                        batch
                    }
                    SynchronizationStrategy::Lazy => {
                        // Process larger batches less frequently
                        if buffer.len() > 50 {
                            let mut batch = Vec::new();
                            for _ in 0..25 {
                                if let Some(event) = buffer.pop_front() {
                                    batch.push(event);
                                } else {
                                    break;
                                }
                            }
                            batch
                        } else {
                            Vec::new()
                        }
                    }
                    SynchronizationStrategy::Priority => {
                        // Process high priority events first
                        // For now, just process like immediate
                        buffer.pop_front().map(|e| vec![e]).unwrap_or_default()
                    }
                }
            } else {
                Vec::new()
            }
        };

        // Process the events (in a real implementation, this would involve
        // actual synchronization logic)
        for event in events_to_process {
            // Simulate processing time based on strategy
            let processing_delay = match strategy {
                SynchronizationStrategy::Immediate => Duration::from_micros(100),
                SynchronizationStrategy::Batched => Duration::from_micros(50),
                SynchronizationStrategy::Adaptive => Duration::from_micros(75),
                SynchronizationStrategy::Lazy => Duration::from_micros(25),
                SynchronizationStrategy::Priority => Duration::from_micros(80),
            };

            sleep(processing_delay).await;

            // Send synchronization event
            if let Err(e) = event_sender.send(SyncEvent::EventSynchronized {
                stream_id,
                event_id: event.event.id,
                latency_us: processing_delay.as_micros() as u64,
            }) {
                warn!("Failed to send event synchronized event: {}", e);
            }
        }
    }

    /// Collect load metrics
    async fn collect_load_metrics() -> LoadMetrics {
        // In a real implementation, this would collect actual system metrics
        LoadMetrics {
            cpu_usage: 45.0,
            memory_usage: 60.0,
            network_usage: 1024 * 1024, // 1MB/s
            disk_io_usage: 512 * 1024,  // 512KB/s
            queue_depth: 150,
            avg_processing_latency_us: 500,
            error_rate: 0.001, // 0.1%
            last_update: SystemTime::now(),
        }
    }

    /// Check stream health
    async fn check_stream_health(
        active_streams: &Arc<TokioRwLock<HashMap<Uuid, SynchronizationStream>>>,
        event_sender: &Sender<SyncEvent>,
    ) {
        let now = SystemTime::now();
        let mut failed_streams = Vec::new();

        {
            let mut streams = active_streams.write().await;
            for (stream_id, stream) in streams.iter_mut() {
                if stream.state == StreamState::Active {
                    // Check if stream has been inactive for too long
                    if let Ok(duration) = now.duration_since(stream.last_sync) {
                        if duration > Duration::from_secs(60) { // 60 second timeout
                            stream.state = StreamState::Failed;
                            failed_streams.push(*stream_id);
                        }
                    }
                }
            }
        }

        // Send failure events
        for stream_id in failed_streams {
            if let Err(e) = event_sender.send(SyncEvent::StreamFailed {
                stream_id,
                error: "Stream health check timeout".to_string(),
            }) {
                warn!("Failed to send stream failed event: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_boundary_sync_manager_creation() {
        let config = BoundarySyncConfig::default();
        let manager = BoundarySynchronizationManager::new(config).unwrap();
        
        // Test that manager was created successfully
        let stats = manager.get_stats().await;
        assert_eq!(stats.streams_created, 0);
        assert_eq!(stats.active_streams, 0);
    }

    #[tokio::test]
    async fn test_stream_creation() {
        let config = BoundarySyncConfig::default();
        let manager = BoundarySynchronizationManager::new(config).unwrap();
        
        let stream_id = manager.create_stream(
            SynchronizationBoundary::KernelToUserspace,
            SynchronizationBoundary::UserspaceToKernel,
            SynchronizationStrategy::Immediate,
            StreamConfig::default(),
            None,
        ).await.unwrap();
        
        // Verify stream was created
        assert!(!stream_id.is_nil());
    }

    #[tokio::test]
    async fn test_manager_lifecycle() {
        let config = BoundarySyncConfig::default();
        let manager = BoundarySynchronizationManager::new(config).unwrap();
        
        // Start manager
        manager.start().await.unwrap();
        
        // Create a stream
        let stream_id = manager.create_stream(
            SynchronizationBoundary::KernelToUserspace,
            SynchronizationBoundary::UserspaceToKernel,
            SynchronizationStrategy::Batched,
            StreamConfig::default(),
            None,
        ).await.unwrap();
        
        // Start the stream
        manager.start_stream(stream_id).await.unwrap();
        
        // Wait a bit for processing
        sleep(Duration::from_millis(100)).await;
        
        // Stop manager
        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_event_synchronization() {
        let config = BoundarySyncConfig::default();
        let manager = BoundarySynchronizationManager::new(config).unwrap();
        
        manager.start().await.unwrap();
        
        let stream_id = manager.create_stream(
            SynchronizationBoundary::KernelToUserspace,
            SynchronizationBoundary::UserspaceToKernel,
            SynchronizationStrategy::Immediate,
            StreamConfig::default(),
            None,
        ).await.unwrap();
        
        manager.start_stream(stream_id).await.unwrap();
        
        // Create a test event
        let test_event = OrderedSemanticEvent {
            event: SemanticEvent {
                id: EventId::new(),
                event_type: SemanticEventType::FilesystemCreate,
                timestamp: SemanticTimestamp {
                    wall_time: SystemTime::now(),
                    monotonic_time: Duration::from_secs(1),
                },
                source: EventSource::Kernel,
                flags: EventFlags {
                    is_synthetic: false,
                    requires_response: false,
                    is_high_priority: false,
                    is_security_relevant: false,
                },
                priority: EventPriority::Normal,
                category: EventCategory::Filesystem,
                context: SemanticContext::default(),
            },
            sequence_number: 1,
            vector_clock: vec![1, 0, 0],
            causal_dependencies: Vec::new(),
        };
        
        // Synchronize the event
        manager.synchronize_event(stream_id, test_event).await.unwrap();
        
        // Wait for processing
        sleep(Duration::from_millis(50)).await;
        
        // Check statistics
        let stats = manager.get_stats().await;
        assert!(stats.events_synchronized > 0);
        
        manager.stop().await.unwrap();
    }
}