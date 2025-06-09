//! Event Persistence Layer for Userspace Semantic Journal
//! 
//! This module implements efficient event storage with adaptive batching,
//! priority-based ordering, and integration with existing durability manager patterns.
//! 
//! Key Features:
//! - Adaptive batching with priority-based ordering
//! - Integration with existing durability manager patterns
//! - Configurable persistence strategies (immediate, batched, async)
//! - Efficient storage with compression and indexing

use std::sync::{Arc, atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering}};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH, Instant, Duration};
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom, BufWriter, BufReader};

use crossbeam::queue::SegQueue;
use parking_lot::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use tracing::{info, warn, error, debug, trace, instrument};
use uuid::Uuid;
use tokio::sync::{mpsc, RwLock as TokioRwLock, Mutex as TokioMutex};
use tokio::time::{interval, sleep};

use crate::semantic_api::types::{
    SemanticEvent, SemanticEventType, SemanticTimestamp, EventFlags, EventPriority,
    SemanticResult, SemanticError
};
use crate::semantic_api::userspace_journal::{
    BufferedSemanticEvent, ProcessingFlags, CompressionAlgorithm
};
use crate::storage::durability_manager::{DurabilityPolicy, SyncOperation, SyncPriority, SyncRequest};

/// Persistence strategy configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistenceStrategy {
    /// Immediate persistence (highest durability, lowest performance)
    Immediate,
    
    /// Batched persistence (balanced durability and performance)
    Batched,
    
    /// Asynchronous persistence (lowest durability, highest performance)
    Async,
    
    /// Adaptive persistence (adjusts based on load and priority)
    Adaptive,
}

/// Batch configuration for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    
    /// Minimum batch size before forced flush
    pub min_batch_size: usize,
    
    /// Maximum time to wait before flushing batch
    pub max_batch_time_ms: u64,
    
    /// Priority threshold for immediate persistence
    pub priority_threshold: EventPriority,
    
    /// Enable priority-based ordering within batches
    pub enable_priority_ordering: bool,
    
    /// Enable compression for batches
    pub enable_batch_compression: bool,
    
    /// Compression threshold (bytes)
    pub compression_threshold: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            min_batch_size: 10,
            max_batch_time_ms: 100,
            priority_threshold: EventPriority::High,
            enable_priority_ordering: true,
            enable_batch_compression: true,
            compression_threshold: 1024,
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base storage directory
    pub storage_dir: PathBuf,
    
    /// Event log file name
    pub event_log_file: String,
    
    /// Index file name
    pub index_file: String,
    
    /// Metadata file name
    pub metadata_file: String,
    
    /// Maximum file size before rotation
    pub max_file_size: u64,
    
    /// Number of backup files to keep
    pub backup_count: u32,
    
    /// Enable file rotation
    pub enable_rotation: bool,
    
    /// Enable checksums for files
    pub enable_checksums: bool,
    
    /// Buffer size for file I/O
    pub io_buffer_size: usize,
    
    /// Sync policy for file operations
    pub sync_policy: DurabilityPolicy,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("/tmp/vexfs_semantic_storage"),
            event_log_file: "events.log".to_string(),
            index_file: "events.idx".to_string(),
            metadata_file: "metadata.json".to_string(),
            max_file_size: 100 * 1024 * 1024, // 100MB
            backup_count: 5,
            enable_rotation: true,
            enable_checksums: true,
            io_buffer_size: 64 * 1024, // 64KB
            sync_policy: DurabilityPolicy::DataAndMetadata,
        }
    }
}

/// Persistence manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Persistence strategy
    pub strategy: PersistenceStrategy,
    
    /// Batch configuration
    pub batch_config: BatchConfig,
    
    /// Storage configuration
    pub storage_config: StorageConfig,
    
    /// Enable indexing
    pub enable_indexing: bool,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
    
    /// Enable encryption
    pub enable_encryption: bool,
    
    /// Worker thread count
    pub worker_threads: usize,
    
    /// Queue capacity
    pub queue_capacity: usize,
    
    /// Enable metrics collection
    pub enable_metrics: bool,
    
    /// Adaptive thresholds
    pub adaptive_high_load_threshold: f64,
    pub adaptive_low_load_threshold: f64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            strategy: PersistenceStrategy::Adaptive,
            batch_config: BatchConfig::default(),
            storage_config: StorageConfig::default(),
            enable_indexing: true,
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
            enable_encryption: false,
            worker_threads: 2,
            queue_capacity: 10000,
            enable_metrics: true,
            adaptive_high_load_threshold: 0.8,
            adaptive_low_load_threshold: 0.2,
        }
    }
}

/// Event batch for persistence
#[derive(Debug, Clone)]
pub struct EventBatch {
    /// Events in the batch
    pub events: Vec<BufferedSemanticEvent>,
    
    /// Batch ID
    pub batch_id: Uuid,
    
    /// Batch creation time
    pub created_at: SystemTime,
    
    /// Batch priority (highest priority event in batch)
    pub priority: EventPriority,
    
    /// Batch size in bytes
    pub size_bytes: usize,
    
    /// Compression applied
    pub compressed: bool,
    
    /// Checksum
    pub checksum: Option<u32>,
}

impl EventBatch {
    /// Create a new event batch
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            batch_id: Uuid::new_v4(),
            created_at: SystemTime::now(),
            priority: EventPriority::Normal,
            size_bytes: 0,
            compressed: false,
            checksum: None,
        }
    }
    
    /// Add event to batch
    pub fn add_event(&mut self, event: BufferedSemanticEvent) {
        // Update batch priority to highest priority event
        if event.priority > self.priority {
            self.priority = event.priority;
        }
        
        // Estimate size (simplified)
        self.size_bytes += std::mem::size_of::<BufferedSemanticEvent>();
        
        self.events.push(event);
    }
    
    /// Check if batch should be flushed based on config
    pub fn should_flush(&self, config: &BatchConfig) -> bool {
        // Check size limits
        if self.events.len() >= config.max_batch_size {
            return true;
        }
        
        // Check time limits
        if let Ok(elapsed) = self.created_at.elapsed() {
            if elapsed.as_millis() >= config.max_batch_time_ms as u128 {
                return true;
            }
        }
        
        // Check priority threshold
        if self.priority >= config.priority_threshold {
            return true;
        }
        
        false
    }
    
    /// Sort events by priority if enabled
    pub fn sort_by_priority(&mut self) {
        self.events.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
    
    /// Calculate batch checksum
    pub fn calculate_checksum(&mut self) -> SemanticResult<()> {
        let batch_data = bincode::serialize(&self.events)
            .map_err(|e| SemanticError::SerializationError(
                format!("Failed to serialize batch for checksum: {}", e)
            ))?;
        
        let mut hasher = Sha256::new();
        hasher.update(&batch_data);
        let hash = hasher.finalize();
        
        self.checksum = Some(u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]));
        Ok(())
    }
}

/// Event index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Event ID
    pub event_id: u64,
    
    /// Event type
    pub event_type: SemanticEventType,
    
    /// Timestamp
    pub timestamp: SemanticTimestamp,
    
    /// File offset
    pub file_offset: u64,
    
    /// Event size
    pub event_size: u32,
    
    /// Batch ID
    pub batch_id: Uuid,
    
    /// Priority
    pub priority: EventPriority,
}

/// Persistence metrics
#[derive(Debug, Default)]
pub struct PersistenceMetrics {
    /// Total events persisted
    pub total_events_persisted: AtomicU64,
    
    /// Total batches persisted
    pub total_batches_persisted: AtomicU64,
    
    /// Bytes written
    pub bytes_written: AtomicU64,
    
    /// Bytes read
    pub bytes_read: AtomicU64,
    
    /// Average batch size
    pub avg_batch_size: AtomicU64,
    
    /// Average persistence latency (nanoseconds)
    pub avg_persistence_latency_ns: AtomicU64,
    
    /// Persistence errors
    pub persistence_errors: AtomicU64,
    
    /// Index operations
    pub index_operations: AtomicU64,
    
    /// Compression ratio (percentage)
    pub compression_ratio: AtomicU32,
    
    /// Current queue size
    pub current_queue_size: AtomicU64,
    
    /// Strategy switches (for adaptive mode)
    pub strategy_switches: AtomicU64,
}

/// Semantic persistence manager implementation
#[derive(Debug)]
pub struct SemanticPersistenceManager {
    /// Configuration
    config: PersistenceConfig,
    
    /// Current persistence strategy
    current_strategy: RwLock<PersistenceStrategy>,
    
    /// Event queue for persistence
    event_queue: SegQueue<BufferedSemanticEvent>,
    
    /// Current batch being built
    current_batch: Mutex<EventBatch>,
    
    /// Persistence metrics
    metrics: PersistenceMetrics,
    
    /// Event log file
    event_log: Mutex<Option<BufWriter<File>>>,
    
    /// Index file
    index_file: Mutex<Option<BufWriter<File>>>,
    
    /// In-memory index
    index: RwLock<BTreeMap<u64, IndexEntry>>,
    
    /// Running state
    is_running: AtomicBool,
    
    /// Shutdown signal
    shutdown_requested: AtomicBool,
    
    /// Load tracking for adaptive strategy
    load_tracker: RwLock<LoadTracker>,
}

/// Load tracking for adaptive persistence
#[derive(Debug)]
struct LoadTracker {
    /// Recent queue sizes
    queue_sizes: VecDeque<usize>,
    
    /// Recent latencies
    latencies: VecDeque<u64>,
    
    /// Current load factor (0.0 - 1.0)
    current_load: f64,
    
    /// Last update time
    last_update: SystemTime,
}

impl LoadTracker {
    fn new() -> Self {
        Self {
            queue_sizes: VecDeque::with_capacity(100),
            latencies: VecDeque::with_capacity(100),
            current_load: 0.0,
            last_update: SystemTime::now(),
        }
    }
    
    fn update(&mut self, queue_size: usize, latency_ns: u64) {
        // Add new measurements
        self.queue_sizes.push_back(queue_size);
        self.latencies.push_back(latency_ns);
        
        // Keep only recent measurements
        if self.queue_sizes.len() > 100 {
            self.queue_sizes.pop_front();
        }
        if self.latencies.len() > 100 {
            self.latencies.pop_front();
        }
        
        // Calculate current load factor
        let avg_queue_size = self.queue_sizes.iter().sum::<usize>() as f64 / self.queue_sizes.len() as f64;
        let avg_latency = self.latencies.iter().sum::<u64>() as f64 / self.latencies.len() as f64;
        
        // Normalize to 0.0 - 1.0 range (simplified)
        let queue_factor = (avg_queue_size / 1000.0).min(1.0);
        let latency_factor = (avg_latency / 1_000_000.0).min(1.0); // 1ms max
        
        self.current_load = (queue_factor + latency_factor) / 2.0;
        self.last_update = SystemTime::now();
    }
    
    fn get_load(&self) -> f64 {
        self.current_load
    }
}

impl SemanticPersistenceManager {
    /// Create a new semantic persistence manager
    pub fn new(config: PersistenceConfig) -> SemanticResult<Self> {
        info!("Initializing semantic persistence manager with config: {:?}", config);
        
        // Create storage directory if it doesn't exist
        std::fs::create_dir_all(&config.storage_config.storage_dir)
            .map_err(|e| SemanticError::PersistenceError(
                format!("Failed to create storage directory: {}", e)
            ))?;
        
        let manager = Self {
            current_strategy: RwLock::new(config.strategy),
            config,
            event_queue: SegQueue::new(),
            current_batch: Mutex::new(EventBatch::new()),
            metrics: PersistenceMetrics::default(),
            event_log: Mutex::new(None),
            index_file: Mutex::new(None),
            index: RwLock::new(BTreeMap::new()),
            is_running: AtomicBool::new(false),
            shutdown_requested: AtomicBool::new(false),
            load_tracker: RwLock::new(LoadTracker::new()),
        };
        
        info!("Semantic persistence manager initialized successfully");
        Ok(manager)
    }
    
    /// Initialize the persistence manager
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> SemanticResult<()> {
        info!("Initializing semantic persistence manager");
        
        // Open event log file
        let event_log_path = self.config.storage_config.storage_dir
            .join(&self.config.storage_config.event_log_file);
        
        let event_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&event_log_path)
            .map_err(|e| SemanticError::PersistenceError(
                format!("Failed to open event log: {}", e)
            ))?;
        
        *self.event_log.lock() = Some(BufWriter::with_capacity(
            self.config.storage_config.io_buffer_size,
            event_log_file
        ));
        
        // Open index file
        let index_path = self.config.storage_config.storage_dir
            .join(&self.config.storage_config.index_file);
        
        let index_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&index_path)
            .map_err(|e| SemanticError::PersistenceError(
                format!("Failed to open index file: {}", e)
            ))?;
        
        *self.index_file.lock() = Some(BufWriter::with_capacity(
            self.config.storage_config.io_buffer_size,
            index_file
        ));
        
        // Load existing index
        self.load_index().await?;
        
        // Start background workers
        self.start_workers().await?;
        
        self.is_running.store(true, Ordering::Relaxed);
        
        info!("Semantic persistence manager initialized successfully");
        Ok(())
    }
    
    /// Persist a semantic event
    #[instrument(skip(self, event))]
    pub async fn persist_event(&self, event: BufferedSemanticEvent) -> SemanticResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(SemanticError::PersistenceError(
                "Persistence manager is not running".to_string()
            ));
        }
        
        let start_time = Instant::now();
        
        // Update load tracking
        let queue_size = self.get_queue_size();
        
        // Determine persistence strategy
        let strategy = self.determine_strategy(&event, queue_size).await;
        
        match strategy {
            PersistenceStrategy::Immediate => {
                self.persist_immediate(event).await?;
            },
            PersistenceStrategy::Batched | PersistenceStrategy::Adaptive => {
                self.add_to_batch(event).await?;
            },
            PersistenceStrategy::Async => {
                self.event_queue.push(event);
            },
        }
        
        // Update metrics
        let latency = start_time.elapsed().as_nanos() as u64;
        self.metrics.avg_persistence_latency_ns.store(latency, Ordering::Relaxed);
        
        // Update load tracker
        self.load_tracker.write().update(queue_size, latency);
        
        Ok(())
    }
    
    /// Determine the appropriate persistence strategy
    async fn determine_strategy(&self, event: &BufferedSemanticEvent, queue_size: usize) -> PersistenceStrategy {
        let base_strategy = *self.current_strategy.read();
        
        match base_strategy {
            PersistenceStrategy::Adaptive => {
                let load = self.load_tracker.read().get_load();
                
                // High priority events always get immediate persistence
                if event.priority >= EventPriority::High {
                    return PersistenceStrategy::Immediate;
                }
                
                // Adapt based on load
                if load > self.config.adaptive_high_load_threshold {
                    PersistenceStrategy::Async
                } else if load < self.config.adaptive_low_load_threshold {
                    PersistenceStrategy::Immediate
                } else {
                    PersistenceStrategy::Batched
                }
            },
            other => other,
        }
    }
    
    /// Persist event immediately
    async fn persist_immediate(&self, event: BufferedSemanticEvent) -> SemanticResult<()> {
        let mut batch = EventBatch::new();
        batch.add_event(event);
        self.write_batch(batch).await?;
        Ok(())
    }
    
    /// Add event to current batch
    async fn add_to_batch(&self, event: BufferedSemanticEvent) -> SemanticResult<()> {
        let should_flush = {
            let mut batch = self.current_batch.lock();
            batch.add_event(event);
            batch.should_flush(&self.config.batch_config)
        };
        
        if should_flush {
            self.flush_current_batch().await?;
        }
        
        Ok(())
    }
    
    /// Flush the current batch
    async fn flush_current_batch(&self) -> SemanticResult<()> {
        let batch = {
            let mut current_batch = self.current_batch.lock();
            let batch = std::mem::replace(&mut *current_batch, EventBatch::new());
            batch
        };
        
        if !batch.events.is_empty() {
            self.write_batch(batch).await?;
        }
        
        Ok(())
    }
    
    /// Write batch to storage
    async fn write_batch(&self, mut batch: EventBatch) -> SemanticResult<()> {
        let start_time = Instant::now();
        
        // Sort by priority if enabled
        if self.config.batch_config.enable_priority_ordering {
            batch.sort_by_priority();
        }
        
        // Calculate checksum
        batch.calculate_checksum()?;
        
        // Serialize batch
        let batch_data = bincode::serialize(&batch.events)
            .map_err(|e| SemanticError::SerializationError(
                format!("Failed to serialize batch: {}", e)
            ))?;
        
        // Compress if enabled and above threshold
        let final_data = if self.config.enable_compression && 
                            batch_data.len() > self.config.batch_config.compression_threshold {
            self.compress_data(&batch_data)?
        } else {
            batch_data
        };
        
        // Write to event log
        let file_offset = self.write_to_event_log(&final_data).await?;
        
        // Update index
        if self.config.enable_indexing {
            self.update_index(&batch, file_offset).await?;
        }
        
        // Update metrics
        let latency = start_time.elapsed().as_nanos() as u64;
        self.metrics.total_batches_persisted.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_events_persisted.fetch_add(batch.events.len() as u64, Ordering::Relaxed);
        self.metrics.bytes_written.fetch_add(final_data.len() as u64, Ordering::Relaxed);
        self.metrics.avg_persistence_latency_ns.store(latency, Ordering::Relaxed);
        
        if self.config.enable_compression && final_data.len() < batch_data.len() {
            let ratio = ((batch_data.len() - final_data.len()) * 100 / batch_data.len()) as u32;
            self.metrics.compression_ratio.store(ratio, Ordering::Relaxed);
        }
        
        trace!("Wrote batch {} with {} events in {}μs", 
               batch.batch_id, batch.events.len(), latency / 1000);
        
        Ok(())
    }
    
    /// Write data to event log
    async fn write_to_event_log(&self, data: &[u8]) -> SemanticResult<u64> {
        let mut log_guard = self.event_log.lock();
        let log = log_guard.as_mut()
            .ok_or_else(|| SemanticError::PersistenceError(
                "Event log not initialized".to_string()
            ))?;
        
        // Get current position
        let offset = log.stream_position()
            .map_err(|e| SemanticError::PersistenceError(
                format!("Failed to get log position: {}", e)
            ))?;
        
        // Write data
        log.write_all(data)
            .map_err(|e| SemanticError::PersistenceError(
                format!("Failed to write to event log: {}", e)
            ))?;
        
        // Flush based on sync policy
        match self.config.storage_config.sync_policy {
            DurabilityPolicy::Strict | DurabilityPolicy::DataAndMetadata => {
                log.flush()
                    .map_err(|e| SemanticError::PersistenceError(
                        format!("Failed to flush event log: {}", e)
                    ))?;
            },
            _ => {
                // Defer flushing for better performance
            }
        }
        
        Ok(offset)
    }
    
    /// Update the event index
    async fn update_index(&self, batch: &EventBatch, file_offset: u64) -> SemanticResult<()> {
        let mut index = self.index.write();
        let mut current_offset = file_offset;
        
        for event in &batch.events {
            let entry = IndexEntry {
                event_id: event.event.event_id,
                event_type: event.event.event_type,
                timestamp: event.event.timestamp,
                file_offset: current_offset,
                event_size: event.event.event_size,
                batch_id: batch.batch_id,
                priority: event.priority,
            };
            
            index.insert(event.event.event_id, entry);
            current_offset += event.event.event_size as u64;
        }
        
        self.metrics.index_operations.fetch_add(batch.events.len() as u64, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Compress data using configured algorithm
    fn compress_data(&self, data: &[u8]) -> SemanticResult<Vec<u8>> {
        match self.config.compression_algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Lz4 => {
                // In a real implementation, we would use lz4 compression
                // For now, just return the original data
                Ok(data.to_vec())
            },
            CompressionAlgorithm::Zstd => {
                // In a real implementation, we would use zstd compression
                Ok(data.to_vec())
            },
            CompressionAlgorithm::Snappy => {
                // In a real implementation, we would use snappy compression
                Ok(data.to_vec())
            },
        }
    }
    
    /// Load existing index from file
    async fn load_index(&self) -> SemanticResult<()> {
        let index_path = self.config.storage_config.storage_dir
            .join(&self.config.storage_config.index_file);
        
        if !index_path.exists() {
            return Ok(()); // No existing index
        }
        
        // In a real implementation, we would load the index from file
        // For now, just return success
        Ok(())
    }
    
    /// Start background worker tasks
    async fn start_workers(&self) -> SemanticResult<()> {
        // In a real implementation, we would start background tasks for:
        // - Async event processing
        // - Periodic batch flushing
        // - Index maintenance
        // - File rotation
        // - Metrics collection
        
        Ok(())
    }
    
    /// Get current queue size
    pub fn get_queue_size(&self) -> usize {
        // Note: SegQueue doesn't provide direct len(), this is an approximation
        let current_size = self.metrics.current_queue_size.load(Ordering::Relaxed) as usize;
        current_size
    }
    
    /// Get persistence metrics
    pub fn get_metrics(&self) -> &PersistenceMetrics {
        &self.metrics
    }
    
    /// Check if manager is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }
    
    /// Shutdown the persistence manager
    pub async fn shutdown(&self) -> SemanticResult<()> {
        info!("Shutting down semantic persistence manager");
        
        self.shutdown_requested.store(true, Ordering::Relaxed);
        
        // Flush any remaining batches
        self.flush_current_batch().await?;
        
        // Process any remaining events in queue
        while let Some(event) = self.event_queue.pop() {
            self.persist_immediate(event).await?;
        }
        
        // Close files
        if let Some(mut log) = self.event_log.lock().take() {
            log.flush().ok();
        }
        
        if let Some(mut index) = self.index_file.lock().take() {
            index.flush().ok();
        }
        
        self.is_running.store(false, Ordering::Relaxed);
        
        info!("Semantic persistence manager shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::semantic_api::types::SemanticEvent;
    
    #[tokio::test]
    async fn test_persistence_manager_creation() {
        let config = PersistenceConfig::default();
        let manager = SemanticPersistenceManager::new(config).unwrap();
        assert!(!manager.is_running());
    }
    
    #[tokio::test]
    async fn test_event_batch() {
        let mut batch = EventBatch::new();
        assert_eq!(batch.events.len(), 0);
        
        let event = BufferedSemanticEvent {
            event: SemanticEvent::default(),
            buffer_timestamp: SystemTime::now(),
            emission_latency_ns: 0,
            sequence_number: 1,
            priority: EventPriority::High,
            buffer_position: 0,
            cross_boundary_tx_id: None,
            retry_count: 0,
            processing_flags: ProcessingFlags::default(),
        };
        
        batch.add_event(event);
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.priority, EventPriority::High);
    }
    
    #[tokio::test]
    async fn test_batch_config() {
        let config = BatchConfig::default();
        let mut batch = EventBatch::new();
        
        // Should not flush empty batch
        assert!(!batch.should_flush(&config));
        
        // Add events up to max batch size
        for i in 0..config.max_batch_size {
            let event = BufferedSemanticEvent {
                event: SemanticEvent::default(),
                buffer_timestamp: SystemTime::now(),
                emission_latency_ns: 0,
                sequence_number: i as u64,
                priority: EventPriority::Normal,
                buffer_position: 0,
                cross_boundary_tx_id: None,
                retry_count: 0,
                processing_flags: ProcessingFlags::default(),
            };
            batch.add_event(event);
        }
        
        // Should flush when max size reached
        assert!(batch.should_flush(&config));
    }
}