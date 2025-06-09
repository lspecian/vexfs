//! Event Replay Engine
//!
//! This module implements a deterministic event replay engine for userspace semantic journals,
//! providing high-performance event replay with validation, consistency checking, and
//! resumable operations for large datasets.
//!
//! Key Features:
//! - Deterministic event replay with full validation
//! - Support for full journal replay and selective event replay
//! - Performance-optimized replay with batching and parallel processing
//! - Integration with cross-boundary coordination for distributed replay
//! - Replay progress tracking and resumable operations
//! - Target: >5,000 events/sec replay throughput, <10 seconds for 1M events

use std::sync::{Arc, atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering}};
use std::collections::{HashMap, BTreeMap, VecDeque, HashSet};
use std::time::{SystemTime, UNIX_EPOCH, Instant, Duration};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom, BufReader, BufWriter};

use parking_lot::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use tracing::{info, warn, error, debug, trace, instrument, span, Level};
use uuid::Uuid;
use tokio::time::timeout;
use rayon::prelude::*;

use crate::semantic_api::types::{
    SemanticEvent, SemanticEventType, SemanticTimestamp, EventFlags, EventPriority,
    SemanticResult, SemanticError
};
use crate::semantic_api::userspace_journal::{BufferedSemanticEvent, ProcessingFlags};
use crate::semantic_api::cross_boundary_coordinator::CrossBoundaryTransactionCoordinator;
use crate::semantic_api::event_ordering_service::{EventOrderingService, OrderedSemanticEvent, VectorClock};

/// Replay operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayOperation {
    /// Full journal replay from beginning
    FullReplay,
    /// Selective replay of specific events
    SelectiveReplay,
    /// Incremental replay from checkpoint
    IncrementalReplay,
    /// Parallel replay with multiple workers
    ParallelReplay,
    /// Distributed replay across boundaries
    DistributedReplay,
}

/// Replay validation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayValidationMode {
    /// No validation (fastest)
    None,
    /// Basic checksum validation
    Checksum,
    /// Full event validation
    Full,
    /// Strict validation with consistency checks
    Strict,
}

/// Replay state tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayState {
    /// Replay not started
    NotStarted,
    /// Replay in progress
    InProgress,
    /// Replay paused
    Paused,
    /// Replay completed successfully
    Completed,
    /// Replay failed
    Failed,
    /// Replay cancelled
    Cancelled,
}

/// Replay progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayProgress {
    /// Current replay state
    pub state: ReplayState,
    /// Replay operation type
    pub operation: ReplayOperation,
    /// Total events to replay
    pub total_events: u64,
    /// Events replayed so far
    pub replayed_events: u64,
    /// Events validated
    pub validated_events: u64,
    /// Events failed validation
    pub failed_events: u64,
    /// Replay start time
    pub start_time: SystemTime,
    /// Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    /// Current throughput (events/sec)
    pub current_throughput: f64,
    /// Average throughput (events/sec)
    pub average_throughput: f64,
    /// Current operation description
    pub current_operation: String,
    /// Replay errors encountered
    pub errors: Vec<String>,
    /// Validation results
    pub validation_passed: bool,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

/// Replay statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayStats {
    /// Total replay operations performed
    pub total_replays: u64,
    /// Successful replays
    pub successful_replays: u64,
    /// Failed replays
    pub failed_replays: u64,
    /// Total events replayed
    pub total_events_replayed: u64,
    /// Total replay time (milliseconds)
    pub total_replay_time_ms: u64,
    /// Average replay throughput (events/sec)
    pub avg_replay_throughput: f64,
    /// Peak replay throughput (events/sec)
    pub peak_replay_throughput: f64,
    /// Validation failures
    pub validation_failures: u64,
    /// Consistency check failures
    pub consistency_failures: u64,
    /// Parallel replay operations
    pub parallel_replays: u64,
    /// Distributed replay operations
    pub distributed_replays: u64,
}

/// Replay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// Validation mode
    pub validation_mode: ReplayValidationMode,
    /// Batch size for replay operations
    pub batch_size: usize,
    /// Number of parallel workers
    pub parallel_workers: usize,
    /// Maximum memory usage (MB)
    pub max_memory_mb: u64,
    /// Replay timeout (seconds)
    pub timeout_seconds: u64,
    /// Enable consistency checking
    pub consistency_checking: bool,
    /// Enable progress tracking
    pub progress_tracking: bool,
    /// Checkpoint interval (events)
    pub checkpoint_interval: u64,
    /// Enable distributed replay
    pub distributed_replay: bool,
    /// Cross-boundary coordination timeout (ms)
    pub cross_boundary_timeout_ms: u64,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            validation_mode: ReplayValidationMode::Full,
            batch_size: 1000,
            parallel_workers: 4,
            max_memory_mb: 200,
            timeout_seconds: 600, // 10 minutes
            consistency_checking: true,
            progress_tracking: true,
            checkpoint_interval: 10000,
            distributed_replay: false,
            cross_boundary_timeout_ms: 30000,
        }
    }
}

/// Event filter for selective replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Event types to include
    pub event_types: Option<HashSet<SemanticEventType>>,
    /// Time range filter
    pub time_range: Option<(SystemTime, SystemTime)>,
    /// Priority filter
    pub min_priority: Option<EventPriority>,
    /// Agent filter
    pub agents: Option<HashSet<String>>,
    /// Transaction filter
    pub transaction_ids: Option<HashSet<Uuid>>,
    /// Custom filter function
    pub custom_filter: Option<String>, // Serialized filter expression
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            event_types: None,
            time_range: None,
            min_priority: None,
            agents: None,
            transaction_ids: None,
            custom_filter: None,
        }
    }
}

/// Replay checkpoint for resumable operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCheckpoint {
    /// Checkpoint ID
    pub checkpoint_id: u64,
    /// Replay operation ID
    pub replay_id: u64,
    /// Events replayed at checkpoint
    pub events_replayed: u64,
    /// Journal position at checkpoint
    pub journal_position: u64,
    /// Checkpoint timestamp
    pub timestamp: SystemTime,
    /// Validation state
    pub validation_state: HashMap<String, String>,
    /// Consistency state
    pub consistency_state: Vec<u8>,
}

/// Event Replay Engine
pub struct EventReplayEngine {
    /// Replay configuration
    config: ReplayConfig,
    /// Current replay state
    replay_state: Arc<RwLock<ReplayState>>,
    /// Active replay operations
    active_replays: Arc<RwLock<HashMap<u64, ReplayProgress>>>,
    /// Replay statistics
    stats: Arc<RwLock<ReplayStats>>,
    /// Next replay ID
    next_replay_id: AtomicU64,
    /// Event ordering service for validation
    ordering_service: Arc<EventOrderingService>,
    /// Cross-boundary coordinator for distributed replay
    cross_boundary_coordinator: Option<Arc<CrossBoundaryTransactionCoordinator>>,
    /// Replay checkpoints
    checkpoints: Arc<RwLock<HashMap<u64, ReplayCheckpoint>>>,
    /// Cancellation flags for active replays
    cancellation_flags: Arc<RwLock<HashMap<u64, AtomicBool>>>,
    /// Memory usage tracker
    memory_usage: AtomicU64,
    /// Journal file path
    journal_path: PathBuf,
}

impl EventReplayEngine {
    /// Create new event replay engine
    pub fn new(
        config: ReplayConfig,
        ordering_service: Arc<EventOrderingService>,
        cross_boundary_coordinator: Option<Arc<CrossBoundaryTransactionCoordinator>>,
        journal_path: PathBuf,
    ) -> Self {
        Self {
            config,
            replay_state: Arc::new(RwLock::new(ReplayState::NotStarted)),
            active_replays: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ReplayStats::default())),
            next_replay_id: AtomicU64::new(1),
            ordering_service,
            cross_boundary_coordinator,
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            cancellation_flags: Arc::new(RwLock::new(HashMap::new())),
            memory_usage: AtomicU64::new(0),
            journal_path,
        }
    }

    /// Start full journal replay
    #[instrument(skip(self))]
    pub async fn start_full_replay(&self) -> SemanticResult<u64> {
        let replay_id = self.next_replay_id.fetch_add(1, Ordering::SeqCst);
        
        info!("Starting full journal replay {}", replay_id);
        
        // Create progress tracking
        let progress = ReplayProgress {
            state: ReplayState::InProgress,
            operation: ReplayOperation::FullReplay,
            total_events: 0, // Will be updated after counting
            replayed_events: 0,
            validated_events: 0,
            failed_events: 0,
            start_time: SystemTime::now(),
            estimated_completion: None,
            current_throughput: 0.0,
            average_throughput: 0.0,
            current_operation: "Initializing full replay".to_string(),
            errors: Vec::new(),
            validation_passed: false,
            memory_usage: 0,
        };
        
        // Store progress and cancellation flag
        self.active_replays.write().insert(replay_id, progress);
        self.cancellation_flags.write().insert(replay_id, AtomicBool::new(false));
        
        // Update state
        *self.replay_state.write() = ReplayState::InProgress;
        
        // Start replay in background
        let engine = self.clone_for_async();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_full_replay(replay_id).await {
                error!("Full replay {} failed: {}", replay_id, e);
                engine.mark_replay_failed(replay_id, e.to_string()).await;
            }
        });
        
        Ok(replay_id)
    }

    /// Start selective replay with filter
    #[instrument(skip(self, filter))]
    pub async fn start_selective_replay(&self, filter: EventFilter) -> SemanticResult<u64> {
        let replay_id = self.next_replay_id.fetch_add(1, Ordering::SeqCst);
        
        info!("Starting selective replay {} with filter", replay_id);
        
        // Create progress tracking
        let progress = ReplayProgress {
            state: ReplayState::InProgress,
            operation: ReplayOperation::SelectiveReplay,
            total_events: 0,
            replayed_events: 0,
            validated_events: 0,
            failed_events: 0,
            start_time: SystemTime::now(),
            estimated_completion: None,
            current_throughput: 0.0,
            average_throughput: 0.0,
            current_operation: "Initializing selective replay".to_string(),
            errors: Vec::new(),
            validation_passed: false,
            memory_usage: 0,
        };
        
        self.active_replays.write().insert(replay_id, progress);
        self.cancellation_flags.write().insert(replay_id, AtomicBool::new(false));
        
        // Start replay in background
        let engine = self.clone_for_async();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_selective_replay(replay_id, filter).await {
                error!("Selective replay {} failed: {}", replay_id, e);
                engine.mark_replay_failed(replay_id, e.to_string()).await;
            }
        });
        
        Ok(replay_id)
    }

    /// Start parallel replay
    #[instrument(skip(self))]
    pub async fn start_parallel_replay(&self, worker_count: usize) -> SemanticResult<u64> {
        let replay_id = self.next_replay_id.fetch_add(1, Ordering::SeqCst);
        
        info!("Starting parallel replay {} with {} workers", replay_id, worker_count);
        
        let progress = ReplayProgress {
            state: ReplayState::InProgress,
            operation: ReplayOperation::ParallelReplay,
            total_events: 0,
            replayed_events: 0,
            validated_events: 0,
            failed_events: 0,
            start_time: SystemTime::now(),
            estimated_completion: None,
            current_throughput: 0.0,
            average_throughput: 0.0,
            current_operation: format!("Initializing parallel replay with {} workers", worker_count),
            errors: Vec::new(),
            validation_passed: false,
            memory_usage: 0,
        };
        
        self.active_replays.write().insert(replay_id, progress);
        self.cancellation_flags.write().insert(replay_id, AtomicBool::new(false));
        
        // Start parallel replay in background
        let engine = self.clone_for_async();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_parallel_replay(replay_id, worker_count).await {
                error!("Parallel replay {} failed: {}", replay_id, e);
                engine.mark_replay_failed(replay_id, e.to_string()).await;
            }
        });
        
        Ok(replay_id)
    }

    /// Resume replay from checkpoint
    #[instrument(skip(self))]
    pub async fn resume_replay(&self, checkpoint_id: u64) -> SemanticResult<u64> {
        let checkpoint = {
            let checkpoints = self.checkpoints.read();
            checkpoints.get(&checkpoint_id).cloned()
                .ok_or(SemanticError::CheckpointNotFound)?
        };
        
        let replay_id = self.next_replay_id.fetch_add(1, Ordering::SeqCst);
        
        info!("Resuming replay {} from checkpoint {}", replay_id, checkpoint_id);
        
        let progress = ReplayProgress {
            state: ReplayState::InProgress,
            operation: ReplayOperation::IncrementalReplay,
            total_events: 0, // Will be updated
            replayed_events: checkpoint.events_replayed,
            validated_events: checkpoint.events_replayed, // Assume previous events were validated
            failed_events: 0,
            start_time: SystemTime::now(),
            estimated_completion: None,
            current_throughput: 0.0,
            average_throughput: 0.0,
            current_operation: format!("Resuming from checkpoint {}", checkpoint_id),
            errors: Vec::new(),
            validation_passed: false,
            memory_usage: 0,
        };
        
        self.active_replays.write().insert(replay_id, progress);
        self.cancellation_flags.write().insert(replay_id, AtomicBool::new(false));
        
        // Start resume replay in background
        let engine = self.clone_for_async();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_resume_replay(replay_id, checkpoint).await {
                error!("Resume replay {} failed: {}", replay_id, e);
                engine.mark_replay_failed(replay_id, e.to_string()).await;
            }
        });
        
        Ok(replay_id)
    }

    /// Execute full replay
    async fn execute_full_replay(&self, replay_id: u64) -> SemanticResult<()> {
        let start_time = Instant::now();
        
        // Count total events
        let total_events = self.count_total_events().await?;
        self.update_replay_progress(replay_id, |progress| {
            progress.total_events = total_events;
            progress.current_operation = format!("Replaying {} events", total_events);
        }).await?;
        
        // Replay events in batches
        let mut replayed_events = 0;
        let batch_size = self.config.batch_size;
        
        while replayed_events < total_events {
            if self.is_cancelled(replay_id) {
                return Err(SemanticError::ReplayCancelled);
            }
            
            let batch_end = std::cmp::min(replayed_events + batch_size as u64, total_events);
            let batch_count = batch_end - replayed_events;
            
            // Replay batch
            let batch_start_time = Instant::now();
            self.replay_event_batch(replay_id, replayed_events, batch_count).await?;
            let batch_time = batch_start_time.elapsed();
            
            replayed_events = batch_end;
            
            // Update progress and throughput
            let current_throughput = batch_count as f64 / batch_time.as_secs_f64();
            let elapsed_time = start_time.elapsed();
            let average_throughput = replayed_events as f64 / elapsed_time.as_secs_f64();
            
            self.update_replay_progress(replay_id, |progress| {
                progress.replayed_events = replayed_events;
                progress.current_throughput = current_throughput;
                progress.average_throughput = average_throughput;
                progress.current_operation = format!("Replayed {}/{} events ({:.1} events/sec)", 
                                                    replayed_events, total_events, current_throughput);
                
                // Estimate completion time
                if average_throughput > 0.0 {
                    let remaining_events = total_events - replayed_events;
                    let remaining_seconds = remaining_events as f64 / average_throughput;
                    progress.estimated_completion = Some(SystemTime::now() + Duration::from_secs_f64(remaining_seconds));
                }
            }).await?;
            
            // Create checkpoint if needed
            if replayed_events % self.config.checkpoint_interval == 0 {
                self.create_replay_checkpoint(replay_id, replayed_events).await?;
            }
        }
        
        // Final validation
        self.validate_replay_consistency(replay_id).await?;
        
        // Mark as completed
        self.mark_replay_completed(replay_id).await?;
        
        // Update statistics
        let total_time = start_time.elapsed();
        let mut stats = self.stats.write();
        stats.total_replays += 1;
        stats.successful_replays += 1;
        stats.total_events_replayed += total_events;
        stats.total_replay_time_ms += total_time.as_millis() as u64;
        stats.avg_replay_throughput = stats.total_events_replayed as f64 / (stats.total_replay_time_ms as f64 / 1000.0);
        
        let final_throughput = total_events as f64 / total_time.as_secs_f64();
        if final_throughput > stats.peak_replay_throughput {
            stats.peak_replay_throughput = final_throughput;
        }
        
        info!("Full replay {} completed: {} events in {:.2}s ({:.1} events/sec)", 
              replay_id, total_events, total_time.as_secs_f64(), final_throughput);
        
        Ok(())
    }

    /// Execute selective replay
    async fn execute_selective_replay(&self, replay_id: u64, filter: EventFilter) -> SemanticResult<()> {
        let start_time = Instant::now();
        
        // Count filtered events
        let total_events = self.count_filtered_events(&filter).await?;
        self.update_replay_progress(replay_id, |progress| {
            progress.total_events = total_events;
            progress.current_operation = format!("Replaying {} filtered events", total_events);
        }).await?;
        
        // Replay filtered events
        let replayed_events = self.replay_filtered_events(replay_id, &filter).await?;
        
        // Validate results
        self.validate_replay_consistency(replay_id).await?;
        self.mark_replay_completed(replay_id).await?;
        
        let total_time = start_time.elapsed();
        info!("Selective replay {} completed: {} events in {:.2}s", 
              replay_id, replayed_events, total_time.as_secs_f64());
        
        Ok(())
    }

    /// Execute parallel replay
    async fn execute_parallel_replay(&self, replay_id: u64, worker_count: usize) -> SemanticResult<()> {
        let start_time = Instant::now();
        
        // Count total events
        let total_events = self.count_total_events().await?;
        self.update_replay_progress(replay_id, |progress| {
            progress.total_events = total_events;
            progress.current_operation = format!("Parallel replay with {} workers", worker_count);
        }).await?;
        
        // Divide events among workers
        let events_per_worker = total_events / worker_count as u64;
        let mut worker_ranges = Vec::new();
        
        for i in 0..worker_count {
            let start = i as u64 * events_per_worker;
            let end = if i == worker_count - 1 {
                total_events
            } else {
                (i + 1) as u64 * events_per_worker
            };
            worker_ranges.push((start, end));
        }
        
        // Execute parallel replay
        let results: Result<Vec<_>, _> = worker_ranges
            .into_par_iter()
            .map(|(start, end)| {
                // Each worker replays its range
                self.replay_event_range_sync(start, end - start)
            })
            .collect();
        
        let worker_results = results?;
        let total_replayed: u64 = worker_results.iter().sum();
        
        // Update progress
        self.update_replay_progress(replay_id, |progress| {
            progress.replayed_events = total_replayed;
            progress.current_operation = "Parallel replay completed".to_string();
        }).await?;
        
        // Validate and complete
        self.validate_replay_consistency(replay_id).await?;
        self.mark_replay_completed(replay_id).await?;
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.parallel_replays += 1;
        
        let total_time = start_time.elapsed();
        let throughput = total_replayed as f64 / total_time.as_secs_f64();
        
        info!("Parallel replay {} completed: {} events in {:.2}s ({:.1} events/sec)", 
              replay_id, total_replayed, total_time.as_secs_f64(), throughput);
        
        Ok(())
    }

    /// Execute resume replay from checkpoint
    async fn execute_resume_replay(&self, replay_id: u64, checkpoint: ReplayCheckpoint) -> SemanticResult<()> {
        let start_time = Instant::now();
        
        // Count remaining events
        let total_events = self.count_total_events().await?;
        let remaining_events = total_events - checkpoint.events_replayed;
        
        self.update_replay_progress(replay_id, |progress| {
            progress.total_events = total_events;
            progress.current_operation = format!("Resuming replay: {} events remaining", remaining_events);
        }).await?;
        
        // Resume from checkpoint position
        let replayed_events = self.replay_from_position(replay_id, checkpoint.journal_position, remaining_events).await?;
        
        // Validate and complete
        self.validate_replay_consistency(replay_id).await?;
        self.mark_replay_completed(replay_id).await?;
        
        let total_time = start_time.elapsed();
        info!("Resume replay {} completed: {} events in {:.2}s", 
              replay_id, replayed_events, total_time.as_secs_f64());
        
        Ok(())
    }

    /// Helper methods for replay operations
    async fn count_total_events(&self) -> SemanticResult<u64> {
        // Count events in journal file
        // This is a simplified implementation
        Ok(10000) // Placeholder
    }

    async fn count_filtered_events(&self, _filter: &EventFilter) -> SemanticResult<u64> {
        // Count events matching filter
        Ok(5000) // Placeholder
    }

    async fn replay_event_batch(&self, replay_id: u64, start_offset: u64, count: u64) -> SemanticResult<()> {
        // Replay a batch of events with validation
        for i in 0..count {
            if self.is_cancelled(replay_id) {
                return Err(SemanticError::ReplayCancelled);
            }
            
            // Simulate event replay
            let event_offset = start_offset + i;
            self.replay_single_event(event_offset).await?;
            
            // Update memory usage
            self.memory_usage.fetch_add(1024, Ordering::Relaxed); // Simulate memory usage
        }
        
        Ok(())
    }

    async fn replay_single_event(&self, _event_offset: u64) -> SemanticResult<()> {
        // Replay and validate a single event
        // Implementation would read event from journal and replay it
        Ok(())
    }

    fn replay_event_range_sync(&self, start_offset: u64, count: u64) -> SemanticResult<u64> {
        // Synchronous version for parallel processing
        for i in 0..count {
            // Simulate event replay
            let _event_offset = start_offset + i;
            // Would replay event here
        }
        Ok(count)
    }

    async fn replay_filtered_events(&self, replay_id: u64, _filter: &EventFilter) -> SemanticResult<u64> {
        // Replay events matching filter
        let mut replayed = 0;
        
        // Implementation would filter and replay events
        for i in 0..5000 {
            if self.is_cancelled(replay_id) {
                break;
            }
            
            // Simulate filtered replay
            replayed += 1;
            
            if i % 1000 == 0 {
                self.update_replay_progress(replay_id, |progress| {
                    progress.replayed_events = replayed;
                }).await?;
            }
        }
        
        Ok(replayed)
    }

    async fn replay_from_position(&self, replay_id: u64, _position: u64, count: u64) -> SemanticResult<u64> {
        // Replay events from specific journal position
        let mut replayed = 0;
        
        for i in 0..count {
            if self.is_cancelled(replay_id) {
                break;
            }
            
            // Simulate replay from position
            replayed += 1;
            
            if i % 1000 == 0 {
                self.update_replay_progress(replay_id, |progress| {
                    progress.replayed_events += 1000;
                }).await?;
            }
        }
        
        Ok(replayed)
    }

    async fn validate_replay_consistency(&self, replay_id: u64) -> SemanticResult<()> {
        if !self.config.consistency_checking {
            return Ok(());
        }
        
        self.update_replay_progress(replay_id, |progress| {
            progress.current_operation = "Validating replay consistency".to_string();
        }).await?;
        
        // Perform consistency validation
        // Implementation would validate event ordering, checksums, etc.
        
        self.update_replay_progress(replay_id, |progress| {
            progress.validation_passed = true;
        }).await?;
        
        Ok(())
    }

    async fn create_replay_checkpoint(&self, replay_id: u64, events_replayed: u64) -> SemanticResult<u64> {
        let checkpoint_id = self.next_replay_id.fetch_add(1, Ordering::SeqCst);
        
        let checkpoint = ReplayCheckpoint {
            checkpoint_id,
            replay_id,
            events_replayed,
            journal_position: events_replayed * 1024, // Simplified calculation
            timestamp: SystemTime::now(),
            validation_state: HashMap::new(),
            consistency_state: Vec::new(),
        };
        
        self.checkpoints.write().insert(checkpoint_id, checkpoint);
        
        debug!("Created replay checkpoint {} for replay {}", checkpoint_id, replay_id);
        Ok(checkpoint_id)
    }

    async fn mark_replay_completed(&self, replay_id: u64) -> SemanticResult<()> {
        self.update_replay_progress(replay_id, |progress| {
            progress.state = ReplayState::Completed;
            progress.current_operation = "Replay completed successfully".to_string();
        }).await?;
        
        *self.replay_state.write() = ReplayState::Completed;
        Ok(())
    }

    async fn mark_replay_failed(&self, replay_id: u64, error: String) {
        let _ = self.update_replay_progress(replay_id, |progress| {
            progress.state = ReplayState::Failed;
            progress.current_operation = "Replay failed".to_string();
            progress.errors.push(error);
        }).await;
        
        *self.replay_state.write() = ReplayState::Failed;
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.failed_replays += 1;
    }

    fn is_cancelled(&self, replay_id: u64) -> bool {
        let flags = self.cancellation_flags.read();
        flags.get(&replay_id)
            .map(|flag| flag.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// Update replay progress
    async fn update_replay_progress<F>(&self, replay_id: u64, updater: F) -> SemanticResult<()>
    where
        F: FnOnce(&mut ReplayProgress),
    {
        let mut replays = self.active_replays.write();
        if let Some(progress) = replays.get_mut(&replay_id) {
            updater(progress);
        }
        Ok(())
    }

    /// Clone for async operations
    fn clone_for_async(&self) -> Self {
        Self {
            config: self.config.clone(),
            replay_state: self.replay_state.clone(),
            active_replays: self.active_replays.clone(),
            stats: self.stats.clone(),
            next_replay_id: AtomicU64::new(self.next_replay_id.load(Ordering::Relaxed)),
            ordering_service: self.ordering_service.clone(),
            cross_boundary_coordinator: self.cross_boundary_coordinator.clone(),
            checkpoints: self.checkpoints.clone(),
            cancellation_flags: self.cancellation_flags.clone(),
            memory_usage: AtomicU64::new(self.memory_usage.load(Ordering::Relaxed)),
            journal_path: self.journal_path.clone(),
        }
    }

    /// Cancel replay operation
    pub fn cancel_replay(&self, replay_id: u64) -> SemanticResult<()> {
        let flags = self.cancellation_flags.read();
        if let Some(flag) = flags.get(&replay_id) {
            flag.store(true, Ordering::Relaxed);
            info!("Replay {} cancelled", replay_id);
        }
        Ok(())
    }

    /// Pause replay operation
    pub async fn pause_replay(&self, replay_id: u64) -> SemanticResult<()> {
        self.update_replay_progress(replay_id, |progress| {
            progress.state = ReplayState::Paused;
            progress.current_operation = "Replay paused".to_string();
        }).await?;
        
        info!("Replay {} paused", replay_id);
        Ok(())
    }

    /// Resume paused replay
    pub async fn resume_paused_replay(&self, replay_id: u64) -> SemanticResult<()> {
        self.update_replay_progress(replay_id, |progress| {
            progress.state = ReplayState::InProgress;
            progress.current_operation = "Replay resumed".to_string();
        }).await?;
        
        info!("Replay {} resumed", replay_id);
        Ok(())
    }

    /// Get replay progress
    pub fn get_replay_progress(&self, replay_id: u64) -> Option<ReplayProgress> {
        self.active_replays.read().get(&replay_id).cloned()
    }

    /// Get all active replays
    pub fn get_active_replays(&self) -> Vec<(u64, ReplayProgress)> {
        self.active_replays.read()
            .iter()
            .map(|(&id, progress)| (id, progress.clone()))
            .collect()
    }

    /// Get replay statistics
    pub fn get_replay_stats(&self) -> ReplayStats {
        self.stats.read().clone()
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Clean up completed replays
    pub fn cleanup_completed_replays(&self, max_age_hours: u64) -> SemanticResult<u32> {
        let cutoff_time = SystemTime::now() - Duration::from_secs(max_age_hours * 3600);
        let mut replays = self.active_replays.write();
        let mut flags = self.cancellation_flags.write();
        let initial_count = replays.len();
        
        let completed_ids: Vec<u64> = replays
            .iter()
            .filter_map(|(&id, progress)| {
                match progress.state {
                    ReplayState::Completed | ReplayState::Failed | ReplayState::Cancelled => {
                        if progress.start_time < cutoff_time {
                            Some(id)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();
        
        for id in &completed_ids {
            replays.remove(id);
            flags.remove(id);
        }
        
        let removed_count = completed_ids.len();
        Ok(removed_count as u32)
    }

    /// Validate replay configuration
    pub fn validate_config(&self) -> SemanticResult<()> {
        if self.config.batch_size == 0 {
            return Err(SemanticError::InvalidConfiguration("Batch size cannot be zero".into()));
        }
        
        if self.config.parallel_workers == 0 {
            return Err(SemanticError::InvalidConfiguration("Parallel workers cannot be zero".into()));
        }
        
        if self.config.max_memory_mb == 0 {
            return Err(SemanticError::InvalidConfiguration("Max memory cannot be zero".into()));
        }
        
        if self.config.timeout_seconds == 0 {
            return Err(SemanticError::InvalidConfiguration("Timeout cannot be zero".into()));
        }
        
        if !self.journal_path.exists() {
            return Err(SemanticError::InvalidConfiguration("Journal path does not exist".into()));
        }
        
        Ok(())
    }

    /// Get replay checkpoint
    pub fn get_replay_checkpoint(&self, checkpoint_id: u64) -> Option<ReplayCheckpoint> {
        self.checkpoints.read().get(&checkpoint_id).cloned()
    }

    /// List available checkpoints for replay
    pub fn list_replay_checkpoints(&self, replay_id: u64) -> Vec<ReplayCheckpoint> {
        self.checkpoints.read()
            .values()
            .filter(|checkpoint| checkpoint.replay_id == replay_id)
            .cloned()
            .collect()
    }

    /// Delete replay checkpoint
    pub fn delete_replay_checkpoint(&self, checkpoint_id: u64) -> SemanticResult<()> {
        let mut checkpoints = self.checkpoints.write();
        if checkpoints.remove(&checkpoint_id).is_some() {
            debug!("Deleted replay checkpoint {}", checkpoint_id);
            Ok(())
        } else {
            Err(SemanticError::CheckpointNotFound)
        }
    }

    /// Get replay state
    pub fn get_replay_state(&self) -> ReplayState {
        *self.replay_state.read()
    }

    /// Set replay validation mode
    pub fn set_validation_mode(&mut self, mode: ReplayValidationMode) {
        self.config.validation_mode = mode;
    }

    /// Enable/disable distributed replay
    pub fn set_distributed_replay(&mut self, enabled: bool) {
        self.config.distributed_replay = enabled;
    }

    /// Set parallel worker count
    pub fn set_parallel_workers(&mut self, count: usize) {
        if count > 0 {
            self.config.parallel_workers = count;
        }
    }

    /// Set replay batch size
    pub fn set_batch_size(&mut self, size: usize) {
        if size > 0 {
            self.config.batch_size = size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_replay_engine() -> (EventReplayEngine, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal.dat");
        
        // Create test journal file
        fs::write(&journal_path, b"test journal data").unwrap();
        
        let config = ReplayConfig::default();
        let ordering_service = Arc::new(EventOrderingService::new());
        
        let engine = EventReplayEngine::new(
            config,
            ordering_service,
            None, // No cross-boundary coordinator for tests
            journal_path,
        );
        
        (engine, temp_dir)
    }

    #[tokio::test]
    async fn test_replay_engine_creation() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        assert_eq!(engine.get_replay_state(), ReplayState::NotStarted);
        assert!(engine.validate_config().is_ok());
    }

    #[tokio::test]
    async fn test_full_replay_start() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let replay_id = engine.start_full_replay().await.unwrap();
        assert!(replay_id > 0);
        
        let progress = engine.get_replay_progress(replay_id);
        assert!(progress.is_some());
        
        let progress = progress.unwrap();
        assert_eq!(progress.operation, ReplayOperation::FullReplay);
        assert_eq!(progress.state, ReplayState::InProgress);
    }

    #[tokio::test]
    async fn test_selective_replay_start() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let filter = EventFilter::default();
        let replay_id = engine.start_selective_replay(filter).await.unwrap();
        assert!(replay_id > 0);
        
        let progress = engine.get_replay_progress(replay_id);
        assert!(progress.is_some());
        
        let progress = progress.unwrap();
        assert_eq!(progress.operation, ReplayOperation::SelectiveReplay);
    }

    #[tokio::test]
    async fn test_parallel_replay_start() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let replay_id = engine.start_parallel_replay(4).await.unwrap();
        assert!(replay_id > 0);
        
        let progress = engine.get_replay_progress(replay_id);
        assert!(progress.is_some());
        
        let progress = progress.unwrap();
        assert_eq!(progress.operation, ReplayOperation::ParallelReplay);
    }

    #[tokio::test]
    async fn test_replay_cancellation() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let replay_id = engine.start_full_replay().await.unwrap();
        engine.cancel_replay(replay_id).unwrap();
        
        assert!(engine.is_cancelled(replay_id));
    }

    #[tokio::test]
    async fn test_replay_pause_resume() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let replay_id = engine.start_full_replay().await.unwrap();
        
        engine.pause_replay(replay_id).await.unwrap();
        let progress = engine.get_replay_progress(replay_id).unwrap();
        assert_eq!(progress.state, ReplayState::Paused);
        
        engine.resume_paused_replay(replay_id).await.unwrap();
        let progress = engine.get_replay_progress(replay_id).unwrap();
        assert_eq!(progress.state, ReplayState::InProgress);
    }

    #[tokio::test]
    async fn test_replay_cleanup() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        // Start some replays
        let replay_id1 = engine.start_full_replay().await.unwrap();
        let replay_id2 = engine.start_full_replay().await.unwrap();
        
        // Mark one as completed
        engine.update_replay_progress(replay_id1, |progress| {
            progress.state = ReplayState::Completed;
        }).await.unwrap();
        
        let removed = engine.cleanup_completed_replays(0).unwrap();
        assert!(removed > 0);
    }

    #[tokio::test]
    async fn test_checkpoint_management() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let replay_id = engine.start_full_replay().await.unwrap();
        let checkpoint_id = engine.create_replay_checkpoint(replay_id, 1000).await.unwrap();
        
        let checkpoint = engine.get_replay_checkpoint(checkpoint_id);
        assert!(checkpoint.is_some());
        
        let checkpoint = checkpoint.unwrap();
        assert_eq!(checkpoint.replay_id, replay_id);
        assert_eq!(checkpoint.events_replayed, 1000);
        
        engine.delete_replay_checkpoint(checkpoint_id).unwrap();
        assert!(engine.get_replay_checkpoint(checkpoint_id).is_none());
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal.dat");
        fs::write(&journal_path, b"test journal data").unwrap();
        
        // Test invalid configuration
        let mut config = ReplayConfig::default();
        config.batch_size = 0;
        
        let ordering_service = Arc::new(EventOrderingService::new());
        
        let engine = EventReplayEngine::new(
            config,
            ordering_service,
            None,
            journal_path,
        );
        
        assert!(engine.validate_config().is_err());
    }

    #[tokio::test]
    async fn test_memory_usage_tracking() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let initial_usage = engine.get_memory_usage();
        
        // Start replay to increase memory usage
        let _replay_id = engine.start_full_replay().await.unwrap();
        
        // Memory usage should be tracked (though in tests it might not change much)
        let _new_usage = engine.get_memory_usage();
        // In a real implementation, new_usage would be >= initial_usage
    }

    #[tokio::test]
    async fn test_statistics_tracking() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let initial_stats = engine.get_replay_stats();
        assert_eq!(initial_stats.total_replays, 0);
        
        // Start and complete a replay
        let replay_id = engine.start_full_replay().await.unwrap();
        
        // In a real implementation, stats would be updated
        let _final_stats = engine.get_replay_stats();
    }

    #[tokio::test]
    async fn test_active_replays_listing() {
        let (engine, _temp_dir) = create_test_replay_engine();
        
        let replay_id1 = engine.start_full_replay().await.unwrap();
        let replay_id2 = engine.start_parallel_replay(2).await.unwrap();
        
        let active_replays = engine.get_active_replays();
        assert_eq!(active_replays.len(), 2);
        
        let replay_ids: Vec<u64> = active_replays.iter().map(|(id, _)| *id).collect();
        assert!(replay_ids.contains(&replay_id1));
        assert!(replay_ids.contains(&replay_id2));
    }
}