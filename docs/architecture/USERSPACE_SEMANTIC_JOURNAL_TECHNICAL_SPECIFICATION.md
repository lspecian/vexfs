# VexFS Userspace Semantic Journal Technical Specification
## Task 23.4 - Interface Definitions and Data Structures

### Overview

This document provides detailed technical specifications for the Userspace Semantic Journal System, including precise interface definitions, data structures, error handling, and performance requirements.

## 1. Core Data Structures

### 1.1 Userspace Semantic Journal Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserspaceJournalConfig {
    /// Journal file path
    pub journal_path: PathBuf,
    
    /// Maximum journal file size before rotation (default: 1GB)
    pub max_journal_size: u64,
    
    /// Event buffer size for batching (default: 10,000)
    pub buffer_size: usize,
    
    /// Batch size for writes (default: 100)
    pub batch_size: usize,
    
    /// Flush interval in milliseconds (default: 10ms)
    pub flush_interval_ms: u64,
    
    /// Checkpoint interval in seconds (default: 60s)
    pub checkpoint_interval_s: u64,
    
    /// Enable compression (default: true)
    pub compression_enabled: bool,
    
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
    
    /// Enable kernel compatibility mode (default: true)
    pub kernel_compatibility: bool,
    
    /// Enable cross-boundary synchronization (default: true)
    pub cross_boundary_sync: bool,
    
    /// Performance targets
    pub target_emission_latency_ns: u64, // <1000ns
    pub target_throughput_events_per_sec: u32, // >10,000
    
    /// Recovery configuration
    pub enable_recovery: bool,
    pub recovery_batch_size: usize,
    pub max_recovery_time_s: u64,
    
    /// Memory management
    pub memory_pool_size: usize,
    pub max_memory_usage_mb: u64,
    
    /// Security settings
    pub enable_encryption: bool,
    pub encryption_key_path: Option<PathBuf>,
    pub enable_access_control: bool,
}

impl Default for UserspaceJournalConfig {
    fn default() -> Self {
        Self {
            journal_path: PathBuf::from("/tmp/vexfs_userspace_journal"),
            max_journal_size: 1024 * 1024 * 1024, // 1GB
            buffer_size: 10000,
            batch_size: 100,
            flush_interval_ms: 10,
            checkpoint_interval_s: 60,
            compression_enabled: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
            kernel_compatibility: true,
            cross_boundary_sync: true,
            target_emission_latency_ns: 1000,
            target_throughput_events_per_sec: 10000,
            enable_recovery: true,
            recovery_batch_size: 1000,
            max_recovery_time_s: 300,
            memory_pool_size: 1000,
            max_memory_usage_mb: 512,
            enable_encryption: false,
            encryption_key_path: None,
            enable_access_control: false,
        }
    }
}
```

### 1.2 Buffered Semantic Event

```rust
#[derive(Debug, Clone)]
pub struct BufferedSemanticEvent {
    /// The semantic event
    pub event: SemanticEvent,
    
    /// Timestamp when event was buffered
    pub buffer_timestamp: SystemTime,
    
    /// Emission latency in nanoseconds
    pub emission_latency_ns: u64,
    
    /// Sequence number in buffer
    pub sequence_number: u64,
    
    /// Event priority
    pub priority: EventPriority,
    
    /// Buffer position for tracking
    pub buffer_position: usize,
    
    /// Cross-boundary transaction ID if applicable
    pub cross_boundary_tx_id: Option<Uuid>,
    
    /// Retry count for failed operations
    pub retry_count: u32,
    
    /// Event flags for processing
    pub processing_flags: ProcessingFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessingFlags {
    pub requires_kernel_sync: bool,
    pub requires_persistence: bool,
    pub requires_indexing: bool,
    pub requires_compression: bool,
    pub requires_encryption: bool,
    pub is_high_priority: bool,
    pub is_transactional: bool,
    pub requires_ordering: bool,
}
```

### 1.3 Kernel-Compatible Header Structures

```rust
/// Userspace semantic journal header (kernel-compatible)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UserspaceSemanticHeader {
    /// Magic number: 0x53454D4A ("SEMJ")
    pub magic: u32,
    
    /// Version major: 1
    pub version_major: u32,
    
    /// Version minor: 0
    pub version_minor: u32,
    
    /// Total events in journal
    pub total_events: u64,
    
    /// Next event ID to be assigned
    pub next_event_id: u64,
    
    /// Total journal size in bytes
    pub journal_size: u64,
    
    /// Offset to index section
    pub index_offset: u64,
    
    /// Journal flags
    pub flags: u32,
    
    /// SHA-256 checksum of header
    pub checksum: u32,
    
    /// Userspace marker: 0x55535253 ("USRS")
    pub userspace_marker: u32,
    
    /// Compression algorithm used
    pub compression_algorithm: u32,
    
    /// Encryption algorithm used
    pub encryption_algorithm: u32,
    
    /// Last checkpoint sequence
    pub last_checkpoint_sequence: u64,
    
    /// Last kernel sync sequence
    pub last_kernel_sync_sequence: u64,
    
    /// Journal creation timestamp
    pub creation_timestamp: u64,
    
    /// Last modification timestamp
    pub modification_timestamp: u64,
    
    /// Reserved for future use
    pub reserved: [u32; 8],
}

impl UserspaceSemanticHeader {
    pub const MAGIC: u32 = 0x53454D4A; // "SEMJ"
    pub const USERSPACE_MARKER: u32 = 0x55535253; // "USRS"
    pub const VERSION_MAJOR: u32 = 1;
    pub const VERSION_MINOR: u32 = 0;
    
    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version_major: Self::VERSION_MAJOR,
            version_minor: Self::VERSION_MINOR,
            total_events: 0,
            next_event_id: 1,
            journal_size: std::mem::size_of::<Self>() as u64,
            index_offset: 0,
            flags: 0,
            checksum: 0,
            userspace_marker: Self::USERSPACE_MARKER,
            compression_algorithm: 0,
            encryption_algorithm: 0,
            last_checkpoint_sequence: 0,
            last_kernel_sync_sequence: 0,
            creation_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            modification_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            reserved: [0; 8],
        }
    }
    
    pub fn update_checksum(&mut self) {
        // Calculate SHA-256 checksum of header excluding checksum field
        let data = unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>() - std::mem::size_of::<u32>() * 9 // Exclude checksum and reserved
            )
        };
        self.checksum = crc32(data);
    }
    
    pub fn verify_checksum(&self) -> bool {
        let data = unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>() - std::mem::size_of::<u32>() * 9
            )
        };
        verify_checksum(data, self.checksum)
    }
}
```

### 1.4 Event Record Structure

```rust
/// Userspace semantic event record (kernel-compatible)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UserspaceSemanticEventRecord {
    /// Standard kernel-compatible event header
    pub header: KernelEventHeader,
    
    /// Userspace-specific extensions
    pub userspace_flags: u32,
    
    /// Emission latency in nanoseconds
    pub emission_latency_ns: u64,
    
    /// Buffer position when event was processed
    pub buffer_position: u64,
    
    /// Cross-boundary transaction ID
    pub cross_boundary_id: u64,
    
    /// Userspace sequence number
    pub userspace_sequence: u64,
    
    /// Processing timestamp
    pub processing_timestamp: u64,
    
    /// Persistence timestamp
    pub persistence_timestamp: u64,
    
    /// Retry count
    pub retry_count: u32,
    
    /// Reserved for future use
    pub reserved: [u32; 4],
}

impl UserspaceSemanticEventRecord {
    pub fn from_semantic_event(event: &SemanticEvent, buffer_info: &BufferInfo) -> Self {
        Self {
            header: KernelEventHeader::from_semantic_event(event),
            userspace_flags: buffer_info.processing_flags.to_u32(),
            emission_latency_ns: buffer_info.emission_latency_ns,
            buffer_position: buffer_info.buffer_position as u64,
            cross_boundary_id: buffer_info.cross_boundary_tx_id
                .map(|id| id.as_u128() as u64)
                .unwrap_or(0),
            userspace_sequence: buffer_info.sequence_number,
            processing_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            persistence_timestamp: 0, // Set when persisted
            retry_count: buffer_info.retry_count,
            reserved: [0; 4],
        }
    }
}
```

## 2. Core Interfaces

### 2.1 Userspace Semantic Journal Interface

```rust
#[async_trait]
pub trait UserspaceSemanticJournalTrait: Send + Sync {
    /// Initialize the journal
    async fn initialize(&mut self) -> Result<(), SemanticJournalError>;
    
    /// Start background processing
    async fn start(&mut self) -> Result<(), SemanticJournalError>;
    
    /// Emit a single semantic event
    async fn emit_event(&self, event: SemanticEvent) -> Result<u64, SemanticJournalError>;
    
    /// Emit multiple events atomically
    async fn emit_events_atomic(&self, events: Vec<SemanticEvent>) -> Result<Vec<u64>, SemanticJournalError>;
    
    /// Emit event with custom priority
    async fn emit_event_with_priority(
        &self,
        event: SemanticEvent,
        priority: EventPriority,
    ) -> Result<u64, SemanticJournalError>;
    
    /// Query events from the journal
    async fn query_events(&self, query: &EventQuery) -> Result<EventQueryResponse, SemanticJournalError>;
    
    /// Query events by sequence range
    async fn query_events_by_sequence(
        &self,
        start_sequence: u64,
        end_sequence: u64,
    ) -> Result<Vec<SemanticEvent>, SemanticJournalError>;
    
    /// Query events by time range
    async fn query_events_by_time(
        &self,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Result<Vec<SemanticEvent>, SemanticJournalError>;
    
    /// Query events by type
    async fn query_events_by_type(
        &self,
        event_type: SemanticEventType,
        limit: Option<usize>,
    ) -> Result<Vec<SemanticEvent>, SemanticJournalError>;
    
    /// Create a checkpoint
    async fn create_checkpoint(&self) -> Result<CheckpointId, SemanticJournalError>;
    
    /// List available checkpoints
    async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>, SemanticJournalError>;
    
    /// Recover from a checkpoint
    async fn recover_from_checkpoint(&self, checkpoint_id: CheckpointId) -> Result<(), SemanticJournalError>;
    
    /// Synchronize with kernel journal
    async fn sync_with_kernel(&self) -> Result<SyncResult, SemanticJournalError>;
    
    /// Force flush of buffered events
    async fn flush(&self) -> Result<FlushResult, SemanticJournalError>;
    
    /// Compact the journal
    async fn compact(&self) -> Result<CompactionResult, SemanticJournalError>;
    
    /// Verify journal integrity
    async fn verify_integrity(&self) -> Result<IntegrityReport, SemanticJournalError>;
    
    /// Get journal statistics
    fn get_metrics(&self) -> UserspaceJournalMetrics;
    
    /// Reset statistics
    fn reset_metrics(&self);
    
    /// Get current journal status
    fn get_status(&self) -> JournalStatus;
    
    /// Shutdown the journal
    async fn shutdown(&mut self) -> Result<(), SemanticJournalError>;
}
```

### 2.2 Kernel Compatibility Bridge Interface

```rust
#[async_trait]
pub trait KernelCompatibilityBridgeTrait: Send + Sync {
    /// Convert userspace event to kernel-compatible format
    fn convert_to_kernel_format(&self, event: &SemanticEvent) -> Result<Vec<u8>, ConversionError>;
    
    /// Convert kernel event to userspace format
    fn convert_from_kernel_format(&self, data: &[u8]) -> Result<SemanticEvent, ConversionError>;
    
    /// Synchronize event sequences with kernel
    async fn sync_sequences(&self) -> Result<SequenceSyncResult, SyncError>;
    
    /// Verify format compatibility
    fn verify_compatibility(&self, kernel_version: (u32, u32)) -> Result<(), CompatibilityError>;
    
    /// Get kernel journal status
    async fn get_kernel_status(&self) -> Result<KernelJournalStatus, SyncError>;
    
    /// Adjust userspace sequence for kernel compatibility
    fn adjust_sequence_for_kernel(&self, userspace_sequence: u64) -> u64;
    
    /// Adjust kernel sequence for userspace compatibility
    fn adjust_sequence_from_kernel(&self, kernel_sequence: u64) -> u64;
    
    /// Check if kernel journal is available
    async fn is_kernel_available(&self) -> bool;
    
    /// Get synchronization metrics
    fn get_sync_metrics(&self) -> SyncMetrics;
}
```

### 2.3 Semantic Persistence Layer Interface

```rust
#[async_trait]
pub trait SemanticPersistenceLayerTrait: Send + Sync {
    /// Write events to storage
    async fn write_events(&self, events: &[SemanticEvent]) -> Result<WriteResult, PersistenceError>;
    
    /// Write single event to storage
    async fn write_event(&self, event: &SemanticEvent) -> Result<WriteResult, PersistenceError>;
    
    /// Read events from storage by sequence range
    async fn read_events_by_sequence(
        &self,
        start_sequence: u64,
        end_sequence: u64,
    ) -> Result<Vec<SemanticEvent>, PersistenceError>;
    
    /// Read events from storage by offset range
    async fn read_events_by_offset(
        &self,
        start_offset: u64,
        length: u64,
    ) -> Result<Vec<SemanticEvent>, PersistenceError>;
    
    /// Create storage checkpoint
    async fn create_checkpoint(&self) -> Result<CheckpointInfo, PersistenceError>;
    
    /// Restore from checkpoint
    async fn restore_checkpoint(&self, checkpoint_id: CheckpointId) -> Result<(), PersistenceError>;
    
    /// Compact storage files
    async fn compact_storage(&self) -> Result<CompactionResult, PersistenceError>;
    
    /// Verify storage integrity
    async fn verify_integrity(&self) -> Result<IntegrityReport, PersistenceError>;
    
    /// Get storage statistics
    fn get_storage_metrics(&self) -> StorageMetrics;
    
    /// Sync storage to disk
    async fn sync_to_disk(&self) -> Result<(), PersistenceError>;
    
    /// Get storage status
    fn get_storage_status(&self) -> StorageStatus;
}
```

### 2.4 Cross-Boundary Coordinator Interface

```rust
#[async_trait]
pub trait CrossBoundaryCoordinatorTrait: Send + Sync {
    /// Begin cross-boundary transaction
    async fn begin_transaction(&self, layer_mask: u32) -> Result<TransactionId, CoordinationError>;
    
    /// Add userspace events to transaction
    async fn add_userspace_events(
        &self,
        tx_id: TransactionId,
        events: Vec<SemanticEvent>,
    ) -> Result<(), CoordinationError>;
    
    /// Add kernel event references to transaction
    async fn add_kernel_event_refs(
        &self,
        tx_id: TransactionId,
        event_refs: Vec<KernelEventRef>,
    ) -> Result<(), CoordinationError>;
    
    /// Prepare transaction for commit
    async fn prepare_transaction(&self, tx_id: TransactionId) -> Result<PrepareResult, CoordinationError>;
    
    /// Commit cross-boundary transaction
    async fn commit_transaction(&self, tx_id: TransactionId) -> Result<CommitResult, CoordinationError>;
    
    /// Abort cross-boundary transaction
    async fn abort_transaction(&self, tx_id: TransactionId) -> Result<(), CoordinationError>;
    
    /// Get transaction status
    async fn get_transaction_status(&self, tx_id: TransactionId) -> Result<TransactionStatus, CoordinationError>;
    
    /// List active transactions
    async fn list_active_transactions(&self) -> Result<Vec<TransactionInfo>, CoordinationError>;
    
    /// Synchronize with kernel events
    async fn sync_kernel_events(&self, tx_id: TransactionId) -> Result<SyncResult, CoordinationError>;
    
    /// Check consistency across boundaries
    async fn check_consistency(&self) -> Result<ConsistencyReport, CoordinationError>;
    
    /// Get coordination metrics
    fn get_coordination_metrics(&self) -> CoordinationMetrics;
}
```

## 3. Error Types and Handling

### 3.1 Error Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum SemanticJournalError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Compatibility error: {0}")]
    Compatibility(#[from] CompatibilityError),
    
    #[error("Synchronization error: {0}")]
    Synchronization(#[from] SyncError),
    
    #[error("Recovery error: {0}")]
    Recovery(#[from] RecoveryError),
    
    #[error("Performance degradation: {0}")]
    Performance(String),
    
    #[error("Consistency violation: {0}")]
    Consistency(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),
    
    #[error("Security error: {0}")]
    Security(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Compression error: {0}")]
    Compression(String),
    
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    
    #[error("Corruption detected: {0}")]
    Corruption(String),
    
    #[error("Insufficient space")]
    InsufficientSpace,
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CompatibilityError {
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
    
    #[error("Format conversion failed: {0}")]
    ConversionFailed(String),
    
    #[error("Kernel interface unavailable")]
    KernelUnavailable,
    
    #[error("Magic number mismatch")]
    MagicMismatch,
    
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Sequence drift detected: {drift}")]
    SequenceDrift { drift: i64 },
    
    #[error("Sync timeout")]
    Timeout,
    
    #[error("Kernel communication failed: {0}")]
    KernelCommunication(String),
    
    #[error("Consistency check failed: {0}")]
    ConsistencyFailed(String),
    
    #[error("Lock acquisition failed")]
    LockFailed,
}
```

### 3.2 Error Recovery Strategies

```rust
#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    
    /// Exponential backoff factor
    pub backoff_factor: f64,
    
    /// Maximum retry delay
    pub max_retry_delay_ms: u64,
    
    /// Enable circuit breaker
    pub enable_circuit_breaker: bool,
    
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    
    /// Circuit breaker reset timeout
    pub circuit_breaker_reset_timeout_ms: u64,
}

pub struct ErrorRecoveryManager {
    config: ErrorRecoveryConfig,
    retry_counts: Arc<RwLock<HashMap<String, u32>>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    metrics: Arc<RwLock<ErrorMetrics>>,
}

impl ErrorRecoveryManager {
    pub async fn execute_with_retry<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Result<T, E> + Send + Sync,
        E: std::error::Error + Send + Sync + 'static,
    {
        let mut attempts = 0;
        let mut delay = self.config.retry_delay_ms;
        
        loop {
            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    attempts += 1;
                    
                    if attempts >= self.config.max_retries {
                        return Err(error);
                    }
                    
                    // Check circuit breaker
                    if self.config.enable_circuit_breaker && self.is_circuit_open(&error).await {
                        return Err(error);
                    }
                    
                    // Wait before retry
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    
                    // Exponential backoff
                    delay = (delay as f64 * self.config.backoff_factor) as u64;
                    delay = delay.min(self.config.max_retry_delay_ms);
                }
            }
        }
    }
}
```

## 4. Performance Specifications

### 4.1 Latency Requirements

```rust
#[derive(Debug, Clone)]
pub struct LatencyRequirements {
    /// Event emission latency (target: <1μs)
    pub emission_latency_ns: u64,
    
    /// Batch processing latency (target: <10μs for 100 events)
    pub batch_processing_latency_ns: u64,
    
    /// Persistence latency (target: <100μs for batch write)
    pub persistence_latency_ns: u64,
    
    /// Query latency (target: <1ms for simple queries)
    pub query_latency_ns: u64,
    
    /// Sync latency (target: <1ms for kernel sync)
    pub sync_latency_ns: u64,
    
    /// Recovery latency (target: <1s for 10,000 events)
    pub recovery_latency_ns: u64,
}

impl Default for LatencyRequirements {
    fn default() -> Self {
        Self {
            emission_latency_ns: 1_000,        // 1μs
            batch_processing_latency_ns: 10_000, // 10μs
            persistence_latency_ns: 100_000,   // 100μs
            query_latency_ns: 1_000_000,       // 1ms
            sync_latency_ns: 1_000_000,        // 1ms
            recovery_latency_ns: 1_000_000_000, // 1s
        }
    }
}
```

### 4.2 Throughput Requirements

```rust
#[derive(Debug, Clone)]
pub struct ThroughputRequirements {
    /// Target events per second (target: >10,000)
    pub target_events_per_sec: u32,
    
    /// Sustained events per second (target: >5,000 for 1 hour)
    pub sustained_events_per_sec: u32,
    
    /// Burst events per second (target: >50,000 for 10 seconds)
    pub burst_events_per_sec: u32,
    
    /// Recovery events per second (target: >1,000)
    pub recovery_events_per_sec: u32,
    
    /// Query throughput (target: >100 queries/sec)
    pub query_throughput_per_sec: u32,
    
    /// Sync operations per second (target: >10)
    pub sync_ops_per_sec: u32,
}

impl Default for ThroughputRequirements {
    fn default() -> Self {
        Self {
            target_events_per_sec: 10_000,
            sustained_events_per_sec: 5_000,
            burst_events_per_sec: 50_000,
            recovery_events_per_sec: 1_000,
            query_throughput_per_sec: 100,
            sync_ops_per_sec: 10,
        }
    }
}
```

### 4.3 Resource Utilization Limits

```rust
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB (target: <512MB under load)
    pub max_memory_mb: u64,
    
    /// Baseline memory usage in MB (target: <100MB)
    pub baseline_memory_mb: u64,
    
    /// Maximum CPU usage percentage (target: <20% under load)
    pub max_cpu_percent: u32,
    
    /// Baseline CPU usage percentage (target: <5%)
    pub baseline_cpu_percent: u32,
    
    /// Maximum storage write rate in MB/s (target: <10MB/s)
    pub max_storage_write_rate_mb_per_sec: u64,
    
    /// Maximum network bandwidth in KB/s (target: <1MB/s)
    pub max_network_bandwidth_kb_per_sec: u64,
    
    /// Maximum file descriptors
    pub max_file_descriptors: u32,
    
    /// Maximum thread count
    pub max_threads: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            baseline_memory_mb: 100,
            max_cpu_percent: 20,
            baseline_cpu_percent: 5,
            max_storage_write_rate_mb_per_sec: 10,
            max_network_bandwidth_kb_per_sec: 1024,
            max_file_descriptors: 1000,
            max_threads: 10,
        }
    }
}
```

## 5. Metrics and Monitoring

### 5.1 Journal Metrics

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserspaceJournalMetrics {
    /// Event metrics
    pub events_emitted: u64,
    pub events_persisted: u64,
    pub events_recovered: u64,
    pub events_dropped: u64,
    pub events_retried: u64,
    pub events_failed: u64,
    
    /// Performance metrics
    pub avg_emission_latency_ns: u64,
    pub max_emission_latency_ns: u64,
    pub min_emission_latency_ns: u64,
    pub p95_emission_latency_ns: u64,
    pub p99_emission_latency_ns: u64,
    
    /// Throughput metrics
    pub current_events_per_sec: u64,
    pub peak_events_per_sec: u64,
    pub avg_events_per_sec: u64,
    pub total_events_processed: u64,
    
    /// Batch metrics
    pub batches_processed: u64,
    pub avg_batch_size: f64,
    pub max_batch_size: usize,
    pub avg_batch_processing_time_ns: u64,
    
    /// Storage metrics
    pub journal_size_bytes: u64,
    pub storage_utilization_percent: u32,
    pub compression_ratio: f32,
    pub checksum_verification_rate: f32,
    pub storage_write_rate_mb_per_sec: f64,
    
    /// Synchronization metrics
    pub kernel_sync_operations: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub avg_sync_latency_ms: u64,
    pub max_sync_latency_ms: u64,
    pub sequence_drift_events: u64,
    
    /// Consistency metrics
    pub consistency_checks: u64,
    pub consistency_violations: u64,
    pub cross_boundary_transactions: u64,
    pub successful_cross_boundary_commits: u64,
    pub failed_cross_boundary_commits: u64,
    
    /// Error metrics
    pub storage_errors: u64,
    pub compatibility_errors: u64,
    pub recovery