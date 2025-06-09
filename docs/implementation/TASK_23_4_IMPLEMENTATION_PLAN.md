# VexFS Task 23.4 Implementation Plan
## Userspace Semantic Journal System Compatible with Kernel Implementation

### Overview

This document provides a detailed implementation plan for Task 23.4, breaking down the comprehensive architecture into specific, actionable implementation tasks with clear dependencies, deliverables, and success criteria.

### Implementation Strategy

The implementation follows a **bottom-up approach** with **incremental integration**, ensuring each component is thoroughly tested before building dependent components. This strategy minimizes integration risks and allows for early performance validation.

## Phase 1: Core Infrastructure Foundation (Weeks 1-2)

### Task 1.1: Userspace Journal Core Structure
**File**: `rust/src/semantic_api/userspace_journal.rs`
**Dependencies**: Existing event emission framework
**Estimated Effort**: 3 days

#### Implementation Details:
```rust
// Core structure implementation
pub struct UserspaceSemanticJournal {
    config: Arc<RwLock<UserspaceJournalConfig>>,
    file_manager: Arc<JournalFileManager>,
    event_buffer: Arc<Mutex<VecDeque<BufferedSemanticEvent>>>,
    index_manager: Arc<SemanticIndexManager>,
    metrics: Arc<RwLock<UserspaceJournalMetrics>>,
    global_sequence: AtomicU64,
    local_sequence: AtomicU64,
    state: AtomicU32,
    shutdown_signal: Arc<AtomicBool>,
}

// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserspaceJournalConfig {
    pub journal_path: PathBuf,
    pub max_journal_size: u64,
    pub buffer_size: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub target_emission_latency_ns: u64,
    pub target_throughput_events_per_sec: u32,
}
```

#### Success Criteria:
- [ ] Core structure compiles without errors
- [ ] Configuration loading and validation works
- [ ] Basic initialization and shutdown functionality
- [ ] Memory usage <10MB for empty journal
- [ ] Unit tests achieve 100% coverage

### Task 1.2: Basic Event Buffering System
**File**: `rust/src/semantic_api/userspace_journal.rs` (continued)
**Dependencies**: Task 1.1
**Estimated Effort**: 2 days

#### Implementation Details:
```rust
#[derive(Debug, Clone)]
struct BufferedSemanticEvent {
    event: SemanticEvent,
    buffer_timestamp: SystemTime,
    emission_latency_ns: u64,
    sequence_number: u64,
    priority: EventPriority,
}

impl UserspaceSemanticJournal {
    pub async fn emit_event(&self, event: SemanticEvent) -> Result<u64, SemanticJournalError> {
        let start_time = Instant::now();
        
        // Generate sequence number
        let sequence = self.global_sequence.fetch_add(1, Ordering::Relaxed);
        
        // Create buffered event
        let buffered_event = BufferedSemanticEvent {
            event,
            buffer_timestamp: SystemTime::now(),
            emission_latency_ns: start_time.elapsed().as_nanos() as u64,
            sequence_number: sequence,
            priority: EventPriority::Normal,
        };
        
        // Add to buffer
        {
            let mut buffer = self.event_buffer.lock().unwrap();
            buffer.push_back(buffered_event);
            
            // Check buffer overflow
            if buffer.len() > self.config.read().unwrap().buffer_size {
                buffer.pop_front(); // Drop oldest event
                self.metrics.write().unwrap().events_dropped += 1;
            }
        }
        
        // Update metrics
        self.update_emission_metrics(start_time.elapsed().as_nanos() as u64);
        
        Ok(sequence)
    }
}
```

#### Success Criteria:
- [ ] Event buffering with configurable size limits
- [ ] Overflow handling (drop oldest events)
- [ ] Emission latency <1μs (target: 500ns)
- [ ] Thread-safe concurrent access
- [ ] Buffer utilization metrics

### Task 1.3: Semantic Persistence Layer Foundation
**File**: `rust/src/semantic_api/semantic_persistence.rs`
**Dependencies**: Task 1.1, existing storage infrastructure
**Estimated Effort**: 4 days

#### Implementation Details:
```rust
pub struct SemanticPersistenceLayer {
    config: Arc<PersistenceConfig>,
    io_manager: Arc<AsyncFileManager>,
    compression: Arc<CompressionEngine>,
    checksum_calculator: Arc<ChecksumCalculator>,
    buffer_pool: Arc<WriteBufferPool>,
    durability_manager: Arc<DurabilityManager>,
}

impl SemanticPersistenceLayer {
    pub async fn write_events(&self, events: &[SemanticEvent]) -> Result<WriteResult, PersistenceError> {
        let start_time = Instant::now();
        
        // Serialize events
        let serialized = self.serialize_events(events)?;
        
        // Compress if enabled
        let data = if self.config.compression.enabled {
            self.compression.compress(&serialized)?
        } else {
            serialized
        };
        
        // Calculate checksum
        let checksum = self.checksum_calculator.calculate(&data);
        
        // Write to storage
        let write_result = self.io_manager.write_with_checksum(&data, checksum).await?;
        
        // Update durability
        self.durability_manager.sync_if_required(&write_result).await?;
        
        Ok(WriteResult {
            bytes_written: data.len(),
            events_written: events.len(),
            write_latency_ns: start_time.elapsed().as_nanos() as u64,
            checksum,
        })
    }
}
```

#### Success Criteria:
- [ ] Asynchronous file I/O with proper error handling
- [ ] Configurable compression (LZ4, Zstd)
- [ ] SHA-256 checksum calculation and verification
- [ ] Write latency <100μs for 100 events
- [ ] Integration with existing durability manager

### Task 1.4: Basic Index Management
**File**: `rust/src/semantic_api/index_manager.rs`
**Dependencies**: Task 1.3
**Estimated Effort**: 3 days

#### Implementation Details:
```rust
pub struct SemanticIndexManager {
    config: IndexConfig,
    primary_index: Arc<RwLock<BTreeMap<u64, IndexEntry>>>, // sequence -> offset
    type_index: Arc<RwLock<HashMap<SemanticEventType, Vec<u64>>>>, // type -> sequences
    time_index: Arc<RwLock<BTreeMap<SystemTime, Vec<u64>>>>, // time -> sequences
    index_file: Arc<Mutex<File>>,
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub sequence_number: u64,
    pub file_offset: u64,
    pub event_size: u32,
    pub event_type: SemanticEventType,
    pub timestamp: SystemTime,
    pub checksum: u32,
}

impl SemanticIndexManager {
    pub async fn add_event_index(&self, entry: IndexEntry) -> Result<(), IndexError> {
        // Add to primary index
        self.primary_index.write().unwrap().insert(entry.sequence_number, entry.clone());
        
        // Add to type index
        self.type_index.write().unwrap()
            .entry(entry.event_type)
            .or_insert_with(Vec::new)
            .push(entry.sequence_number);
        
        // Add to time index
        self.time_index.write().unwrap()
            .entry(entry.timestamp)
            .or_insert_with(Vec::new)
            .push(entry.sequence_number);
        
        // Persist index entry
        self.persist_index_entry(&entry).await?;
        
        Ok(())
    }
    
    pub fn query_by_sequence_range(&self, start: u64, end: u64) -> Vec<IndexEntry> {
        self.primary_index.read().unwrap()
            .range(start..=end)
            .map(|(_, entry)| entry.clone())
            .collect()
    }
    
    pub fn query_by_event_type(&self, event_type: SemanticEventType) -> Vec<u64> {
        self.type_index.read().unwrap()
            .get(&event_type)
            .cloned()
            .unwrap_or_default()
    }
}
```

#### Success Criteria:
- [ ] Fast lookups by sequence number (<1μs)
- [ ] Event type and timestamp indexing
- [ ] Persistent index storage
- [ ] Index recovery on startup
- [ ] Memory usage <50MB for 1M events

## Phase 2: Kernel Compatibility Layer (Weeks 3-4)

### Task 2.1: Kernel Format Compatibility Bridge
**File**: `rust/src/semantic_api/journal_compatibility.rs`
**Dependencies**: Phase 1, kernel interface analysis
**Estimated Effort**: 5 days

#### Implementation Details:
```rust
pub struct KernelCompatibilityBridge {
    kernel_interface: Arc<KernelInterface>,
    format_converter: Arc<SemanticEventConverter>,
    sync_state: Arc<RwLock<KernelSyncState>>,
    metrics: Arc<RwLock<CompatibilityMetrics>>,
}

/// Userspace semantic journal header (kernel-compatible)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct UserspaceSemanticHeader {
    magic: u32,           // 0x53454D4A ("SEMJ")
    version_major: u32,   // 1
    version_minor: u32,   // 0
    total_events: u64,
    next_event_id: u64,
    journal_size: u64,
    index_offset: u64,
    flags: u32,
    checksum: u32,        // SHA-256 checksum
    userspace_marker: u32, // 0x55535253 ("USRS")
    reserved: [u32; 6],
}

impl KernelCompatibilityBridge {
    pub fn convert_to_kernel_format(&self, event: &SemanticEvent) -> Result<Vec<u8>, ConversionError> {
        let mut buffer = Vec::with_capacity(1024);
        
        // Create kernel-compatible header
        let header = KernelEventHeader {
            event_id: event.event_id,
            event_type: event.event_type as u32,
            event_subtype: event.event_subtype.unwrap_or(0),
            timestamp_ns: event.timestamp.timestamp.timestamp_nanos() as u64,
            sequence: event.global_sequence,
            cpu_id: 0, // Will be filled by kernel
            process_id: event.timestamp.process_id,
            global_sequence: event.global_sequence,
            local_sequence: event.local_sequence,
            event_flags: self.convert_flags(&event.flags),
            event_priority: event.priority as u32,
            event_size: 0, // Will be calculated
            context_size: 0, // Will be calculated
            payload_size: 0, // Will be calculated
            metadata_size: 0, // Will be calculated
            event_version: event.event_version,
            checksum: 0, // Will be calculated
            compression_type: event.compression_type.unwrap_or(0),
            encryption_type: event.encryption_type.unwrap_or(0),
            causality_link_count: event.causality_links.len() as u32,
            parent_event_id: event.parent_event_id.unwrap_or(0),
            root_cause_event_id: event.root_cause_event_id.unwrap_or(0),
            agent_visibility_mask: event.agent_visibility_mask,
            agent_relevance_score: event.agent_relevance_score,
            replay_priority: event.replay_priority,
        };
        
        // Serialize header
        buffer.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                &header as *const _ as *const u8,
                std::mem::size_of::<KernelEventHeader>()
            )
        });
        
        // Serialize context
        let context_data = self.serialize_context(&event.context)?;
        buffer.extend_from_slice(&context_data);
        
        // Serialize payload
        if let Some(ref payload) = event.payload {
            let payload_data = serde_json::to_vec(payload)?;
            buffer.extend_from_slice(&payload_data);
        }
        
        // Serialize metadata
        if let Some(ref metadata) = event.metadata {
            let metadata_data = serde_json::to_vec(metadata)?;
            buffer.extend_from_slice(&metadata_data);
        }
        
        // Serialize causality links
        for link in &event.causality_links {
            buffer.extend_from_slice(&link.to_le_bytes());
        }
        
        // Update sizes in header
        let header_mut = unsafe {
            &mut *(buffer.as_mut_ptr() as *mut KernelEventHeader)
        };
        header_mut.event_size = buffer.len() as u32;
        header_mut.context_size = context_data.len() as u32;
        // ... update other sizes
        
        // Calculate and update checksum
        let checksum = crc32(&buffer[std::mem::size_of::<u32>()..]);
        header_mut.checksum = checksum;
        
        Ok(buffer)
    }
}
```

#### Success Criteria:
- [ ] Byte-perfect compatibility with kernel format
- [ ] Bidirectional conversion (userspace ↔ kernel)
- [ ] Checksum validation matches kernel implementation
- [ ] Version compatibility checking
- [ ] Conversion latency <10μs per event

### Task 2.2: Sequence Synchronization
**File**: `rust/src/semantic_api/journal_compatibility.rs` (continued)
**Dependencies**: Task 2.1
**Estimated Effort**: 3 days

#### Implementation Details:
```rust
#[derive(Debug, Clone)]
pub struct KernelSyncState {
    last_kernel_sequence: u64,
    last_userspace_sequence: u64,
    sync_offset: i64,
    last_sync_time: SystemTime,
    sync_status: SyncStatus,
}

impl KernelCompatibilityBridge {
    pub async fn sync_sequences(&self) -> Result<SequenceSyncResult, SyncError> {
        // Read kernel journal header
        let kernel_header = self.kernel_interface.read_journal_header().await?;
        
        // Get current userspace sequence
        let userspace_sequence = self.get_current_userspace_sequence();
        
        // Calculate synchronization offset
        let sync_offset = kernel_header.next_event_id as i64 - userspace_sequence as i64;
        
        // Update sync state
        {
            let mut sync_state = self.sync_state.write().unwrap();
            sync_state.last_kernel_sequence = kernel_header.next_event_id;
            sync_state.last_userspace_sequence = userspace_sequence;
            sync_state.sync_offset = sync_offset;
            sync_state.last_sync_time = SystemTime::now();
            sync_state.sync_status = SyncStatus::Synchronized;
        }
        
        Ok(SequenceSyncResult {
            kernel_sequence: kernel_header.next_event_id,
            userspace_sequence,
            offset: sync_offset,
            drift_ms: self.calculate_time_drift().as_millis() as u64,
        })
    }
    
    pub fn adjust_sequence_for_kernel(&self, userspace_sequence: u64) -> u64 {
        let sync_state = self.sync_state.read().unwrap();
        (userspace_sequence as i64 + sync_state.sync_offset) as u64
    }
}
```

#### Success Criteria:
- [ ] Accurate sequence synchronization with kernel
- [ ] Drift detection and correction
- [ ] Sync latency <1ms
- [ ] Automatic resync on drift detection
- [ ] Sync status monitoring and alerting

### Task 2.3: Format Validation and Testing
**File**: `tests/kernel_compatibility_tests.rs`
**Dependencies**: Task 2.1, 2.2
**Estimated Effort**: 2 days

#### Implementation Details:
```rust
#[cfg(test)]
mod kernel_compatibility_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_format_compatibility() {
        // Create test event
        let event = create_test_semantic_event();
        
        // Convert to kernel format
        let bridge = KernelCompatibilityBridge::new().await.unwrap();
        let kernel_data = bridge.convert_to_kernel_format(&event).unwrap();
        
        // Verify header magic and structure
        let header = unsafe {
            &*(kernel_data.as_ptr() as *const KernelEventHeader)
        };
        assert_eq!(header.magic, KERNEL_EVENT_MAGIC);
        assert_eq!(header.event_type, event.event_type as u32);
        
        // Convert back to userspace format
        let converted_event = bridge.convert_from_kernel_format(&kernel_data).unwrap();
        
        // Verify round-trip consistency
        assert_eq!(event.event_id, converted_event.event_id);
        assert_eq!(event.event_type, converted_event.event_type);
        // ... verify all fields
    }
    
    #[tokio::test]
    async fn test_sequence_synchronization() {
        let bridge = KernelCompatibilityBridge::new().await.unwrap();
        
        // Perform initial sync
        let sync_result = bridge.sync_sequences().await.unwrap();
        assert!(sync_result.offset.abs() < 1000); // Reasonable drift
        
        // Test sequence adjustment
        let userspace_seq = 12345;
        let kernel_seq = bridge.adjust_sequence_for_kernel(userspace_seq);
        assert_eq!(kernel_seq, (userspace_seq as i64 + sync_result.offset) as u64);
    }
}
```

#### Success Criteria:
- [ ] 100% format compatibility validation
- [ ] Round-trip conversion tests pass
- [ ] Sequence synchronization accuracy tests
- [ ] Performance benchmarks meet targets
- [ ] Integration tests with actual kernel module

## Phase 3: Performance Optimization (Weeks 5-6)

### Task 3.1: Lock-Free Event Queue
**File**: `rust/src/semantic_api/lockfree_queue.rs`
**Dependencies**: Phase 1, crossbeam crate
**Estimated Effort**: 4 days

#### Implementation Details:
```rust
use crossbeam::queue::SegQueue;
use crossbeam::epoch::{self, Atomic, Owned, Shared};

pub struct LockFreeEventQueue {
    queue: SegQueue<SemanticEvent>,
    enqueue_count: AtomicU64,
    dequeue_count: AtomicU64,
    epoch_manager: epoch::Collector,
    stats: Arc<RwLock<QueueStats>>,
}

impl LockFreeEventQueue {
    pub fn new() -> Self {
        Self {
            queue: SegQueue::new(),
            enqueue_count: AtomicU64::new(0),
            dequeue_count: AtomicU64::new(0),
            epoch_manager: epoch::Collector::new(),
            stats: Arc::new(RwLock::new(QueueStats::default())),
        }
    }
    
    pub fn enqueue(&self, event: SemanticEvent) -> Result<(), QueueError> {
        let start_time = Instant::now();
        
        // Enqueue event
        self.queue.push(event);
        
        // Update counters
        self.enqueue_count.fetch_add(1, Ordering::Relaxed);
        
        // Update statistics
        let latency_ns = start_time.elapsed().as_nanos() as u64;
        self.update_enqueue_stats(latency_ns);
        
        Ok(())
    }
    
    pub fn dequeue_batch(&self, max_size: usize) -> Vec<SemanticEvent> {
        let mut events = Vec::with_capacity(max_size);
        
        for _ in 0..max_size {
            if let Some(event) = self.queue.pop() {
                events.push(event);
                self.dequeue_count.fetch_add(1, Ordering::Relaxed);
            } else {
                break;
            }
        }
        
        events
    }
    
    pub fn len(&self) -> usize {
        let enqueued = self.enqueue_count.load(Ordering::Relaxed);
        let dequeued = self.dequeue_count.load(Ordering::Relaxed);
        (enqueued - dequeued) as usize
    }
}
```

#### Success Criteria:
- [ ] Enqueue latency <100ns
- [ ] Dequeue batch latency <1μs for 100 events
- [ ] Thread-safe concurrent access
- [ ] Memory-efficient (no memory leaks)
- [ ] Benchmark comparison with mutex-based queue

### Task 3.2: Memory Pool Management
**File**: `rust/src/semantic_api/memory_pool.rs`
**Dependencies**: Task 3.1
**Estimated Effort**: 3 days

#### Implementation Details:
```rust
pub struct EventMemoryPool {
    event_buffers: Arc<MemoryPool<SemanticEvent>>,
    serialization_buffers: Arc<MemoryPool<Vec<u8>>>,
    index_buffers: Arc<MemoryPool<IndexEntry>>,
    stats: Arc<RwLock<PoolStats>>,
}

pub struct MemoryPool<T> {
    pool: SegQueue<Box<T>>,
    allocator: Arc<dyn PoolAllocator<T>>,
    max_size: usize,
    current_size: AtomicUsize,
    allocation_count: AtomicU64,
    deallocation_count: AtomicU64,
}

impl<T> MemoryPool<T> {
    pub fn new(max_size: usize, allocator: Arc<dyn PoolAllocator<T>>) -> Self {
        Self {
            pool: SegQueue::new(),
            allocator,
            max_size,
            current_size: AtomicUsize::new(0),
            allocation_count: AtomicU64::new(0),
            deallocation_count: AtomicU64::new(0),
        }
    }
    
    pub fn allocate(&self) -> PooledObject<T> {
        // Try to get from pool first
        if let Some(object) = self.pool.pop() {
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
            return PooledObject::new(object, Arc::downgrade(&Arc::new(self.clone())));
        }
        
        // Allocate new object if pool is empty
        let object = self.allocator.allocate();
        self.current_size.fetch_add(1, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        
        PooledObject::new(object, Arc::downgrade(&Arc::new(self.clone())))
    }
    
    pub fn return_object(&self, object: Box<T>) {
        if self.current_size.load(Ordering::Relaxed) < self.max_size {
            self.pool.push(object);
        } else {
            // Pool is full, drop the object
            self.current_size.fetch_sub(1, Ordering::Relaxed);
        }
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct PooledObject<T> {
    object: Option<Box<T>>,
    pool: Weak<MemoryPool<T>>,
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let (Some(object), Some(pool)) = (self.object.take(), self.pool.upgrade()) {
            pool.return_object(object);
        }
    }
}
```

#### Success Criteria:
- [ ] Object allocation latency <50ns
- [ ] Memory reuse rate >90%
- [ ] No memory leaks under stress testing
- [ ] Configurable pool sizes
- [ ] Pool utilization monitoring

### Task 3.3: Batching and Streaming Optimization
**File**: `rust/src/semantic_api/batch_processor.rs`
**Dependencies**: Task 3.1, 3.2
**Estimated Effort**: 4 days

#### Implementation Details:
```rust
pub struct EventBatchProcessor {
    config: BatchConfig,
    input_queue: Arc<LockFreeEventQueue>,
    output_sender: mpsc::UnboundedSender<EventBatch>,
    memory_pool: Arc<EventMemoryPool>,
    metrics: Arc<RwLock<BatchMetrics>>,
    adaptive_controller: Arc<AdaptiveBatchController>,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub target_batch_size: usize,
    pub max_wait_time_ms: u64,
    pub adaptive_batching: bool,
    pub priority_batching: bool,
    pub compression_threshold: usize,
    pub max_batch_size: usize,
}

pub struct EventBatch {
    pub events: Vec<SemanticEvent>,
    pub batch_id: u64,
    pub created_at: SystemTime,
    pub priority: BatchPriority,
    pub estimated_size: usize,
}

impl EventBatchProcessor {
    pub async fn start_processing(&self) -> Result<(), BatchError> {
        let mut batch_timer = tokio::time::interval(
            Duration::from_millis(self.config.max_wait_time_ms)
        );
        
        let mut current_batch = Vec::with_capacity(self.config.target_batch_size);
        let mut batch_start_time = Instant::now();
        
        loop {
            tokio::select! {
                _ = batch_timer.tick() => {
                    // Time-based flush
                    if !current_batch.is_empty() {
                        self.flush_batch(current_batch, batch_start_time).await?;
                        current_batch = Vec::with_capacity(self.config.target_batch_size);
                        batch_start_time = Instant::now();
                    }
                }
                
                _ = self.process_events(&mut current_batch, &mut batch_start_time) => {
                    // Size-based flush handled in process_events
                }
            }
        }
    }
    
    async fn process_events(
        &self,
        current_batch: &mut Vec<SemanticEvent>,
        batch_start_time: &mut Instant,
    ) -> Result<(), BatchError> {
        // Dequeue events from input queue
        let events = self.input_queue.dequeue_batch(
            self.config.target_batch_size - current_batch.len()
        );
        
        if events.is_empty() {
            tokio::time::sleep(Duration::from_micros(100)).await;
            return Ok(());
        }
        
        // Add events to current batch
        current_batch.extend(events);
        
        // Check if batch is ready for flush
        let should_flush = if self.config.adaptive_batching {
            self.adaptive_controller.should_flush_batch(current_batch)
        } else {
            current_batch.len() >= self.config.target_batch_size
        };
        
        if should_flush {
            let batch = std::mem::replace(
                current_batch,
                Vec::with_capacity(self.config.target_batch_size)
            );
            self.flush_batch(batch, *batch_start_time).await?;
            *batch_start_time = Instant::now();
        }
        
        Ok(())
    }
    
    async fn flush_batch(&self, events: Vec<SemanticEvent>, start_time: Instant) -> Result<(), BatchError> {
        let batch = EventBatch {
            events,
            batch_id: self.generate_batch_id(),
            created_at: SystemTime::now(),
            priority: self.calculate_batch_priority(&events),
            estimated_size: self.estimate_batch_size(&events),
        };
        
        // Send batch for processing
        self.output_sender.send(batch).map_err(|_| BatchError::SendFailed)?;
        
        // Update metrics
        self.update_batch_metrics(start_time.elapsed());
        
        Ok(())
    }
}
```

#### Success Criteria:
- [ ] Batch processing latency <10μs for 100 events
- [ ] Adaptive batching based on load
- [ ] Priority-based event ordering
- [ ] Throughput >10,000 events/second
- [ ] Memory-efficient batch management

## Phase 4: Cross-Boundary Coordination (Weeks 7-8)

### Task 4.1: Cross-Boundary Transaction Manager
**File**: `rust/src/semantic_api/cross_boundary_coordinator.rs`
**Dependencies**: Phase 1-3, cross-layer framework
**Estimated Effort**: 5 days

#### Implementation Details:
```rust
pub struct CrossBoundaryCoordinator {
    integration_framework: Arc<CrossLayerIntegrationFramework>,
    transaction_manager: Arc<CrossBoundaryTransactionManager>,
    kernel_sync: Arc<KernelSyncInterface>,
    ordering_service: Arc<EventOrderingService>,
    consistency_checker: Arc<ConsistencyChecker>,
    metrics: Arc<RwLock<CoordinationMetrics>>,
}

#[derive(Debug, Clone)]
pub struct CrossBoundaryTransaction {
    pub transaction_id: Uuid,
    pub layer_mask: u32,
    pub userspace_events: Vec<SemanticEvent>,
    pub kernel_event_refs: Vec<KernelEventRef>,
    pub state: CrossBoundaryTransactionState,
    pub start_time: SystemTime,
    pub prepare_time: Option<SystemTime>,
    pub commit_time: Option<SystemTime>,
    pub isolation_level: IsolationLevel,
    pub consistency_requirements: ConsistencyRequirements,
}

impl CrossBoundaryCoordinator {
    pub async fn begin_transaction(&self, layer_mask: u32) -> Result<TransactionId, CoordinationError> {
        let transaction_id = Uuid::new_v4();
        
        // Create transaction
        let transaction = CrossBoundaryTransaction {
            transaction_id,
            layer_mask,
            userspace_events: Vec::new(),
            kernel_event_refs: Vec::new(),
            state: CrossBoundaryTransactionState::Init,
            start_time: SystemTime::now(),
            prepare_time: None,
            commit_time: None,
            isolation_level: IsolationLevel::ReadCommitted,
            consistency_requirements: ConsistencyRequirements::default(),
        };
        
        // Register with transaction manager
        self.transaction_manager.register_transaction(transaction).await?;
        
        // Notify integration framework
        self.integration_framework.begin_cross_layer_transaction(transaction_id, layer_mask).await?;
        
        Ok(transaction_id)
    }
    
    pub async fn add_userspace_events(
        &self,
        tx_id: TransactionId,
        events: Vec<SemanticEvent>
    ) -> Result<(), CoordinationError>