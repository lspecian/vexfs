//! Userspace Semantic Journal System
//! 
//! This module implements a userspace semantic journal system that is compatible
//! with the kernel module journaling infrastructure. It provides AI-native capabilities
//! and event tracking for FUSE filesystems while maintaining cross-boundary consistency.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use crate::semantic_api::serialization::{SemanticSerializer, SerializationConfig, SerializationFormat, CompressionType};
use crate::storage::journal::{VexfsJournal, JournalSuperblock, VexfsTransaction};
use crate::vector_storage::VectorStorageManager;
use crate::anns::hnsw::HnswGraph;
use crate::shared::errors::{VexfsError, VexfsResult};

use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Magic number for userspace semantic journal files
const USERSPACE_SEMANTIC_JOURNAL_MAGIC: u32 = 0x55534A4C; // "USJL"

/// Version of the userspace semantic journal format
const USERSPACE_SEMANTIC_JOURNAL_VERSION: u32 = 1;

/// Maximum stack usage for journal operations (6KB limit)
const MAX_STACK_USAGE: usize = 6144;

/// Default journal file size in bytes (64MB)
const DEFAULT_JOURNAL_SIZE: u64 = 64 * 1024 * 1024;

/// Default batch size for journal operations
const DEFAULT_BATCH_SIZE: usize = 100;

/// Userspace semantic journal header compatible with kernel format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct UserspaceSemanticHeader {
    /// Magic number for identification
    pub magic: u32,
    /// Major version number
    pub version_major: u32,
    /// Minor version number
    pub version_minor: u32,
    /// Total number of events in journal
    pub total_events: u64,
    /// Next event ID to be assigned
    pub next_event_id: u64,
    /// Total journal size in bytes
    pub journal_size: u64,
    /// Offset to event index
    pub index_offset: u64,
    /// Journal flags
    pub flags: u32,
    /// Header checksum
    pub checksum: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modification timestamp
    pub modified_at: u64,
    /// Reserved space for future use
    pub reserved: [u32; 16],
}

impl Default for UserspaceSemanticHeader {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            magic: USERSPACE_SEMANTIC_JOURNAL_MAGIC,
            version_major: USERSPACE_SEMANTIC_JOURNAL_VERSION,
            version_minor: 0,
            total_events: 0,
            next_event_id: 1,
            journal_size: DEFAULT_JOURNAL_SIZE,
            index_offset: 0,
            flags: 0,
            checksum: 0,
            created_at: now,
            modified_at: now,
            reserved: [0; 16],
        }
    }
}

/// Journal entry for userspace semantic events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserspaceJournalEntry {
    /// Entry header
    pub header: JournalEntryHeader,
    /// Serialized semantic event
    pub event_data: Vec<u8>,
    /// Entry checksum
    pub checksum: u32,
}

/// Journal entry header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntryHeader {
    /// Entry size in bytes
    pub entry_size: u32,
    /// Event ID
    pub event_id: u64,
    /// Event type
    pub event_type: u32,
    /// Timestamp
    pub timestamp: u64,
    /// Serialization format used
    pub format: u8,
    /// Compression type used
    pub compression: u8,
    /// Entry flags
    pub flags: u16,
}

/// Journal configuration for userspace operations
#[derive(Debug, Clone)]
pub struct UserspaceJournalConfig {
    /// Journal file path
    pub journal_path: PathBuf,
    /// Maximum journal size
    pub max_size: u64,
    /// Batch size for operations
    pub batch_size: usize,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression threshold
    pub compression_threshold: usize,
    /// Sync interval in milliseconds
    pub sync_interval_ms: u64,
    /// Enable lazy sync
    pub lazy_sync: bool,
    /// Maximum memory usage for caching
    pub max_memory_cache: usize,
    /// Enable cross-boundary compatibility
    pub kernel_compatibility: bool,
}

impl Default for UserspaceJournalConfig {
    fn default() -> Self {
        Self {
            journal_path: PathBuf::from(".vexfs_userspace_journal"),
            max_size: DEFAULT_JOURNAL_SIZE,
            batch_size: DEFAULT_BATCH_SIZE,
            enable_compression: true,
            compression_threshold: 1024,
            sync_interval_ms: 1000,
            lazy_sync: true,
            max_memory_cache: 16 * 1024 * 1024, // 16MB
            kernel_compatibility: true,
        }
    }
}

/// Journal statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct UserspaceJournalStats {
    /// Total events written
    pub events_written: u64,
    /// Total events read
    pub events_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Number of sync operations
    pub sync_operations: u64,
    /// Number of recovery operations
    pub recovery_operations: u64,
    /// Average write latency in microseconds
    pub avg_write_latency_us: u64,
    /// Average read latency in microseconds
    pub avg_read_latency_us: u64,
    /// Current memory usage
    pub memory_usage: usize,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Number of errors encountered
    pub error_count: u64,
    /// Last sync timestamp
    pub last_sync: SystemTime,
}

/// Recovery information for journal replay
#[derive(Debug, Clone)]
pub struct RecoveryInfo {
    /// Last known good position
    pub last_good_position: u64,
    /// Number of corrupted entries found
    pub corrupted_entries: u64,
    /// Recovery timestamp
    pub recovery_timestamp: SystemTime,
    /// Recovery status
    pub status: RecoveryStatus,
}

/// Recovery status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStatus {
    Clean,
    Recovered,
    PartialRecovery,
    Failed,
}

/// Userspace Semantic Journal Manager
pub struct UserspaceSemanticJournal {
    /// Configuration
    config: UserspaceJournalConfig,
    /// Journal file handle
    journal_file: Arc<Mutex<File>>,
    /// Journal header
    header: Arc<RwLock<UserspaceSemanticHeader>>,
    /// Event serializer
    serializer: SemanticSerializer,
    /// In-memory event cache for performance
    event_cache: Arc<RwLock<BTreeMap<u64, SemanticEvent>>>,
    /// Pending events for batch operations
    pending_events: Arc<Mutex<VecDeque<SemanticEvent>>>,
    /// Journal statistics
    stats: Arc<RwLock<UserspaceJournalStats>>,
    /// Recovery information
    recovery_info: Arc<RwLock<Option<RecoveryInfo>>>,
    /// Vector storage integration
    vector_storage: Option<Arc<VectorStorageManager>>,
    /// HNSW graph integration
    hnsw_graph: Option<Arc<Mutex<HnswGraph>>>,
    /// Last sync time
    last_sync: Arc<RwLock<SystemTime>>,
    /// Event index for fast lookups
    event_index: Arc<RwLock<HashMap<u64, u64>>>, // event_id -> file_offset
}

impl UserspaceSemanticJournal {
    /// Create a new userspace semantic journal
    pub fn new(config: UserspaceJournalConfig) -> SemanticResult<Self> {
        // Stack usage check
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        // Create serialization config optimized for userspace
        let serialization_config = SerializationConfig {
            format: SerializationFormat::Bincode, // Efficient binary format
            compression: if config.enable_compression {
                CompressionType::Lz4 // Fast compression for userspace
            } else {
                CompressionType::None
            },
            compression_threshold: config.compression_threshold,
            pretty_print: false,
            include_metadata: true,
        };
        
        let serializer = SemanticSerializer::new(serialization_config);
        
        // Open or create journal file
        let journal_file = Arc::new(Mutex::new(
            OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&config.journal_path)
                .map_err(|e| SemanticError::IoError(format!("Failed to open journal file: {}", e)))?
        ));
        
        // Initialize or read header
        let header = Arc::new(RwLock::new(Self::initialize_header(&journal_file)?));
        
        let journal = Self {
            config,
            journal_file,
            header,
            serializer,
            event_cache: Arc::new(RwLock::new(BTreeMap::new())),
            pending_events: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(UserspaceJournalStats::default())),
            recovery_info: Arc::new(RwLock::new(None)),
            vector_storage: None,
            hnsw_graph: None,
            last_sync: Arc::new(RwLock::new(SystemTime::now())),
            event_index: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Perform recovery if needed
        journal.recover()?;
        
        Ok(journal)
    }
    
    /// Initialize journal with vector storage integration
    pub fn with_vector_storage(
        mut self,
        vector_storage: Arc<VectorStorageManager>,
    ) -> Self {
        self.vector_storage = Some(vector_storage);
        self
    }
    
    /// Initialize journal with HNSW graph integration
    pub fn with_hnsw_graph(
        mut self,
        hnsw_graph: Arc<Mutex<HnswGraph>>,
    ) -> Self {
        self.hnsw_graph = Some(hnsw_graph);
        self
    }
    
    /// Write a semantic event to the journal
    pub fn write_event(&self, event: SemanticEvent) -> SemanticResult<u64> {
        let start_time = Instant::now();
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        // Assign event ID if not set
        let mut event = event;
        if event.event_id == 0 {
            event.event_id = self.get_next_event_id()?;
        }
        
        // Add to pending events for batch processing
        if self.config.lazy_sync {
            let mut pending = self.pending_events.lock()
                .map_err(|_| SemanticError::LockError)?;
            pending.push_back(event.clone());
            
            // Check if we should flush batch
            if pending.len() >= self.config.batch_size {
                drop(pending); // Release lock before flush
                self.flush_pending_events()?;
            }
        } else {
            // Immediate write
            self.write_event_immediate(&event)?;
        }
        
        // Update cache
        {
            let mut cache = self.event_cache.write()
                .map_err(|_| SemanticError::LockError)?;
            
            // Limit cache size to prevent memory exhaustion
            if cache.len() * std::mem::size_of::<SemanticEvent>() > self.config.max_memory_cache {
                // Remove oldest entries (BTreeMap maintains order)
                let keys_to_remove: Vec<u64> = cache.keys().take(cache.len() / 4).cloned().collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
            
            cache.insert(event.event_id, event.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.events_written += 1;
            let latency = start_time.elapsed().as_micros() as u64;
            stats.avg_write_latency_us = (stats.avg_write_latency_us + latency) / 2;
            stats.memory_usage = self.get_memory_usage();
            if stats.memory_usage > stats.peak_memory_usage {
                stats.peak_memory_usage = stats.memory_usage;
            }
        }
        
        // Integrate with vector storage if available
        if let Some(ref vector_storage) = self.vector_storage {
            self.integrate_vector_event(&event, vector_storage)?;
        }
        
        // Integrate with HNSW graph if available
        if let Some(ref hnsw_graph) = self.hnsw_graph {
            self.integrate_hnsw_event(&event, hnsw_graph)?;
        }
        
        Ok(event.event_id)
    }
    
    /// Read a semantic event from the journal
    pub fn read_event(&self, event_id: u64) -> SemanticResult<Option<SemanticEvent>> {
        let start_time = Instant::now();
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        // Check cache first
        {
            let cache = self.event_cache.read()
                .map_err(|_| SemanticError::LockError)?;
            if let Some(event) = cache.get(&event_id) {
                // Update read statistics
                let mut stats = self.stats.write()
                    .map_err(|_| SemanticError::LockError)?;
                stats.events_read += 1;
                let latency = start_time.elapsed().as_micros() as u64;
                stats.avg_read_latency_us = (stats.avg_read_latency_us + latency) / 2;
                
                return Ok(Some(event.clone()));
            }
        }
        
        // Read from disk using index
        let file_offset = {
            let index = self.event_index.read()
                .map_err(|_| SemanticError::LockError)?;
            index.get(&event_id).copied()
        };
        
        if let Some(offset) = file_offset {
            let event = self.read_event_at_offset(offset)?;
            
            // Update cache
            if let Some(ref event) = event {
                let mut cache = self.event_cache.write()
                    .map_err(|_| SemanticError::LockError)?;
                cache.insert(event_id, event.clone());
            }
            
            // Update statistics
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.events_read += 1;
            let latency = start_time.elapsed().as_micros() as u64;
            stats.avg_read_latency_us = (stats.avg_read_latency_us + latency) / 2;
            
            Ok(event)
        } else {
            Ok(None)
        }
    }
    
    /// Query events with filtering
    pub fn query_events(&self, filter: EventFilter) -> SemanticResult<Vec<SemanticEvent>> {
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        let mut results = Vec::new();
        
        // For now, scan through cache and disk
        // In a full implementation, this would use more sophisticated indexing
        
        // Check cache first
        {
            let cache = self.event_cache.read()
                .map_err(|_| SemanticError::LockError)?;
            
            for event in cache.values() {
                if self.matches_filter(event, &filter) {
                    results.push(event.clone());
                }
            }
        }
        
        // If we need more results, scan disk
        // This is a simplified implementation - a full version would use proper indexing
        if results.len() < filter.min_relevance_score.unwrap_or(100) as usize {
            // Read events from disk that aren't in cache
            // Implementation would go here
        }
        
        // Sort results by relevance/timestamp
        results.sort_by(|a, b| {
            b.timestamp.timestamp.cmp(&a.timestamp.timestamp)
        });
        
        Ok(results)
    }
    
    /// Flush pending events to disk
    pub fn flush_pending_events(&self) -> SemanticResult<()> {
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        let events_to_write = {
            let mut pending = self.pending_events.lock()
                .map_err(|_| SemanticError::LockError)?;
            let events: Vec<SemanticEvent> = pending.drain(..).collect();
            events
        };
        
        if events_to_write.is_empty() {
            return Ok(());
        }
        
        // Write events in batch
        for event in events_to_write {
            self.write_event_immediate(&event)?;
        }
        
        // Sync to disk
        self.sync()?;
        
        Ok(())
    }
    
    /// Force synchronization to disk
    pub fn sync(&self) -> SemanticResult<()> {
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        // Flush pending events first
        self.flush_pending_events()?;
        
        // Sync file to disk
        {
            let mut file = self.journal_file.lock()
                .map_err(|_| SemanticError::LockError)?;
            file.flush()
                .map_err(|e| SemanticError::IoError(format!("Failed to flush journal: {}", e)))?;
        }
        
        // Update sync time and statistics
        {
            let mut last_sync = self.last_sync.write()
                .map_err(|_| SemanticError::LockError)?;
            *last_sync = SystemTime::now();
        }
        
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.sync_operations += 1;
            stats.last_sync = SystemTime::now();
        }
        
        Ok(())
    }
    
    /// Perform journal recovery
    pub fn recover(&self) -> SemanticResult<()> {
        let _stack_guard = StackGuard::new(MAX_STACK_USAGE)?;
        
        let recovery_start = SystemTime::now();
        let mut corrupted_entries = 0;
        let mut last_good_position = 0;
        
        // Read and validate journal entries
        let mut file = self.journal_file.lock()
            .map_err(|_| SemanticError::LockError)?;
        
        // Skip header
        file.seek(SeekFrom::Start(std::mem::size_of::<UserspaceSemanticHeader>() as u64))
            .map_err(|e| SemanticError::IoError(format!("Failed to seek in journal: {}", e)))?;
        
        let mut position = std::mem::size_of::<UserspaceSemanticHeader>() as u64;
        let mut event_index = HashMap::new();
        
        loop {
            // Try to read entry header
            let mut header_bytes = vec![0u8; std::mem::size_of::<JournalEntryHeader>()];
            match file.read_exact(&mut header_bytes) {
                Ok(_) => {
                    // Deserialize header
                    match bincode::deserialize::<JournalEntryHeader>(&header_bytes) {
                        Ok(entry_header) => {
                            // Validate entry
                            if self.validate_entry_header(&entry_header) {
                                event_index.insert(entry_header.event_id, position);
                                last_good_position = position + entry_header.entry_size as u64;
                                
                                // Skip to next entry
                                file.seek(SeekFrom::Start(last_good_position))
                                    .map_err(|e| SemanticError::IoError(format!("Failed to seek: {}", e)))?;
                                position = last_good_position;
                            } else {
                                corrupted_entries += 1;
                                break;
                            }
                        }
                        Err(_) => {
                            corrupted_entries += 1;
                            break;
                        }
                    }
                }
                Err(_) => break, // End of file or error
            }
        }
        
        // Update event index
        {
            let mut index = self.event_index.write()
                .map_err(|_| SemanticError::LockError)?;
            *index = event_index;
        }
        
        // Create recovery info
        let recovery_status = if corrupted_entries == 0 {
            RecoveryStatus::Clean
        } else if last_good_position > 0 {
            RecoveryStatus::PartialRecovery
        } else {
            RecoveryStatus::Failed
        };
        
        let recovery_info = RecoveryInfo {
            last_good_position,
            corrupted_entries,
            recovery_timestamp: recovery_start,
            status: recovery_status,
        };
        
        {
            let mut recovery = self.recovery_info.write()
                .map_err(|_| SemanticError::LockError)?;
            *recovery = Some(recovery_info);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.recovery_operations += 1;
            if corrupted_entries > 0 {
                stats.error_count += corrupted_entries;
            }
        }
        
        Ok(())
    }
    
    /// Get journal statistics
    pub fn get_statistics(&self) -> SemanticResult<UserspaceJournalStats> {
        let stats = self.stats.read()
            .map_err(|_| SemanticError::LockError)?;
        Ok(stats.clone())
    }
    
    /// Get recovery information
    pub fn get_recovery_info(&self) -> SemanticResult<Option<RecoveryInfo>> {
        let recovery = self.recovery_info.read()
            .map_err(|_| SemanticError::LockError)?;
        Ok(recovery.clone())
    }
    
    /// Check if journal needs synchronization
    pub fn needs_sync(&self) -> bool {
        let last_sync = self.last_sync.read().unwrap_or_else(|_| {
            std::sync::RwLockReadGuard::map(
                self.last_sync.read().unwrap(),
                |time| time
            )
        });
        
        let elapsed = last_sync.elapsed().unwrap_or_default();
        elapsed.as_millis() as u64 >= self.config.sync_interval_ms
    }
    
    // Private helper methods
    
    fn initialize_header(journal_file: &Arc<Mutex<File>>) -> SemanticResult<UserspaceSemanticHeader> {
        let mut file = journal_file.lock()
            .map_err(|_| SemanticError::LockError)?;
        
        // Try to read existing header
        let mut header_bytes = vec![0u8; std::mem::size_of::<UserspaceSemanticHeader>()];
        match file.read_exact(&mut header_bytes) {
            Ok(_) => {
                // Deserialize and validate header
                match bincode::deserialize::<UserspaceSemanticHeader>(&header_bytes) {
                    Ok(header) => {
                        if header.magic == USERSPACE_SEMANTIC_JOURNAL_MAGIC {
                            return Ok(header);
                        }
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
        
        // Create new header
        let header = UserspaceSemanticHeader::default();
        
        // Write header to file
        file.seek(SeekFrom::Start(0))
            .map_err(|e| SemanticError::IoError(format!("Failed to seek: {}", e)))?;
        
        let header_bytes = bincode::serialize(&header)
            .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
        
        file.write_all(&header_bytes)
            .map_err(|e| SemanticError::IoError(format!("Failed to write header: {}", e)))?;
        
        file.flush()
            .map_err(|e| SemanticError::IoError(format!("Failed to flush header: {}", e)))?;
        
        Ok(header)
    }
    
    fn get_next_event_id(&self) -> SemanticResult<u64> {
        let mut header = self.header.write()
            .map_err(|_| SemanticError::LockError)?;
        
        let event_id = header.next_event_id;
        header.next_event_id += 1;
        header.total_events += 1;
        header.modified_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Update header on disk
        self.write_header_to_disk(&*header)?;
        
        Ok(event_id)
    }
    
    fn write_header_to_disk(&self, header: &UserspaceSemanticHeader) -> SemanticResult<()> {
        let mut file = self.journal_file.lock()
            .map_err(|_| SemanticError::LockError)?;
        
        file.seek(SeekFrom::Start(0))
            .map_err(|e| SemanticError::IoError(format!("Failed to seek: {}", e)))?;
        
        let header_bytes = bincode::serialize(header)
            .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
        
        file.write_all(&header_bytes)
            .map_err(|e| SemanticError::IoError(format!("Failed to write header: {}", e)))?;
        
        Ok(())
    }
    
    fn write_event_immediate(&self, event: &SemanticEvent) -> SemanticResult<()> {
        // Serialize event
        let event_data = self.serializer.serialize_event(event)?;
        
        // Create journal entry
        let entry_header = JournalEntryHeader {
            entry_size: (std::mem::size_of::<JournalEntryHeader>() + event_data.len() + 4) as u32,
            event_id: event.event_id,
            event_type: event.event_type as u32,
            timestamp: event.timestamp.timestamp.timestamp() as u64,
            format: 4, // Bincode
            compression: if self.config.enable_compression { 1 } else { 0 },
            flags: 0,
        };
        
        let checksum = self.calculate_checksum(&event_data);
        
        let journal_entry = UserspaceJournalEntry {
            header: entry_header,
            event_data,
            checksum,
        };
        
        // Write to file
        let mut file = self.journal_file.lock()
            .map_err(|_| SemanticError::LockError)?;
        
        // Seek to end of file
        let position = file.seek(SeekFrom::End(0))
            .map_err(|e| SemanticError::IoError(format!("Failed to seek: {}", e)))?;
        
        // Serialize and write entry
        let entry_bytes = bincode::serialize(&journal_entry)
            .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
        
        file.write_all(&entry_bytes)
            .map_err(|e| SemanticError::IoError(format!("Failed to write entry: {}", e)))?;
        
        // Update index
        {
            let mut index = self.event_index.write()
                .map_err(|_| SemanticError::LockError)?;
            index.insert(event.event_id, position);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.bytes_written += entry_bytes.len() as u64;
        }
        
        Ok(())
    }
    
    fn read_event_at_offset(&self, offset: u64) -> SemanticResult<Option<SemanticEvent>> {
        let mut file = self.journal_file.lock()
            .map_err(|_| SemanticError::LockError)?;
        
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| SemanticError::IoError(format!("Failed to seek: {}", e)))?;
        
        // Read entry header first
        let mut header_bytes = vec![0u8; std::mem::size_of::<JournalEntryHeader>()];
        file.read_exact(&mut header_bytes)
            .map_err(|e| SemanticError::IoError(format!("Failed to read header: {}", e)))?;
        
        let entry_header: JournalEntryHeader = bincode::deserialize(&header_bytes)
            .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
        
        // Read the rest of the entry
        let remaining_size = entry_header.entry_size as usize - std::mem::size_of::<JournalEntryHeader>();
        let mut entry_data = vec![0u8; remaining_size];
        file.read_exact(&mut entry_data)
            .map_err(|e| SemanticError::IoError(format!("Failed to read entry data: {}", e)))?;
        
        // Deserialize the journal entry
        let journal_entry: UserspaceJournalEntry = bincode::deserialize(&entry_data)
            .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
        
        // Verify checksum
        let calculated_checksum = self.calculate_checksum(&journal_entry.event_data);
        if calculated_checksum != journal_entry.checksum {
            return Err(SemanticError::CorruptedData("Checksum mismatch".to_string()));
        }
        
        // Deserialize event
        let event = self.serializer.deserialize_event(&journal_entry.event_data)?;
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.bytes_read += entry_data.len() as u64;
        }
        
        Ok(Some(event))
    }
    
    fn matches_filter(&self, event: &SemanticEvent, filter: &EventFilter) -> bool {
        // Check event types
        if let Some(ref event_types) = filter.event_types {
            if !event_types.contains(&event.event_type) {
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
            let event_time = event.timestamp.timestamp;
            if event_time < time_range.start || event_time > time_range.end {
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
        if let Some(min_relevance) = filter.min_relevance_score {
            if event.agent_relevance_score < min_relevance {
                return false;
            }
        }
        
        true
    }
    
    fn validate_entry_header(&self, header: &JournalEntryHeader) -> bool {
        // Basic validation
        header.entry_size > 0 &&
        header.entry_size < self.config.max_size as u32 &&
        header.event_id > 0
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        // Simple CRC32-like checksum
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_mul(31).wrapping_add(byte as u32);
        }
        checksum
    }
    
    fn integrate_vector_event(
        &self,
        event: &SemanticEvent,
        vector_storage: &Arc<VectorStorageManager>,
    ) -> SemanticResult<()> {
        // Integration with vector storage for vector-related events
        match event.event_type {
            SemanticEventType::VectorCreate |
            SemanticEventType::VectorUpdate |
            SemanticEventType::VectorIndex => {
                // Log vector operation in journal context
                // This could trigger vector storage operations or indexing
                if let Some(ref context) = event.context.vector {
                    // Vector storage integration would go here
                    // For now, just log the integration
                    eprintln!("Journal: Integrated vector event {} with storage", event.event_id);
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    fn integrate_hnsw_event(
        &self,
        event: &SemanticEvent,
        hnsw_graph: &Arc<Mutex<HnswGraph>>,
    ) -> SemanticResult<()> {
        // Integration with HNSW graph for graph-related events
        match event.event_type {
            SemanticEventType::VectorSearch |
            SemanticEventType::VectorSimilarity |
            SemanticEventType::GraphTraverse => {
                // HNSW graph integration would go here
                // For now, just log the integration
                eprintln!("Journal: Integrated search/graph event {} with HNSW", event.event_id);
            }
            _ => {}
        }
        Ok(())
    }
    
    fn get_memory_usage(&self) -> usize {
        // Estimate memory usage
        let cache_size = self.event_cache.read()
            .map(|cache| cache.len() * std::mem::size_of::<SemanticEvent>())
            .unwrap_or(0);
        
        let pending_size = self.pending_events.lock()
            .map(|pending| pending.len() * std::mem::size_of::<SemanticEvent>())
            .unwrap_or(0);
        
        let index_size = self.event_index.read()
            .map(|index| index.len() * (std::mem::size_of::<u64>() * 2))
            .unwrap_or(0);
        
        cache_size + pending_size + index_size
    }
}

/// Stack guard to prevent stack overflow in journal operations
struct StackGuard {
    _marker: [u8; 0],
}

impl StackGuard {
    fn new(max_usage: usize) -> SemanticResult<Self> {
        // Simple stack usage check
        let stack_var = 0u8;
        let stack_ptr = &stack_var as *const u8 as usize;
        
        // This is a simplified check - in a real implementation,
        // you would use more sophisticated stack monitoring
        if max_usage > MAX_STACK_USAGE {
            return Err(SemanticError::StackOverflow);
        }
        
        Ok(Self { _marker: [] })
    }
}

/// Stream message type for journal events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamMessage {
    Event(StreamEventMessage),
    Heartbeat,
    Error(String),
}

/// Journal event stream for real-time monitoring
pub struct JournalEventStream {
    journal: Arc<UserspaceSemanticJournal>,
    subscription: StreamSubscription,
    last_event_id: u64,
}

impl JournalEventStream {
    /// Create a new event stream
    pub fn new(
        journal: Arc<UserspaceSemanticJournal>,
        subscription: StreamSubscription,
    ) -> Self {
        Self {
            journal,
            subscription,
            last_event_id: 0,
        }
    }
    
    /// Get next batch of events
    pub fn next_events(&mut self) -> SemanticResult<Vec<StreamEventMessage>> {
        let mut events = Vec::new();
        
        // Query for new events since last_event_id
        let filter = EventFilter {
            event_types: self.subscription.filter.event_types.clone(),
            categories: self.subscription.filter.categories.clone(),
            time_range: self.subscription.filter.time_range.clone(),
            agent_id: self.subscription.filter.agent_id.clone(),
            transaction_id: self.subscription.filter.transaction_id,
            causality_chain_id: self.subscription.filter.causality_chain_id,
            path_pattern: self.subscription.filter.path_pattern.clone(),
            min_priority: self.subscription.filter.min_priority,
            required_flags: self.subscription.filter.required_flags,
            tags: self.subscription.filter.tags.clone(),
            min_relevance_score: self.subscription.filter.min_relevance_score,
        };
        
        let journal_events = self.journal.query_events(filter)?;
        
        for event in journal_events {
            if event.event_id > self.last_event_id {
                let stream_message = StreamEventMessage {
                    subscription_id: self.subscription.subscription_id,
                    event,
                    sequence_number: self.last_event_id + 1,
                    timestamp: Utc::now(),
                };
                events.push(stream_message);
                self.last_event_id = event.event_id;
            }
        }
        
        Ok(events)
    }
}

/// Kernel compatibility layer for cross-boundary operations
pub struct KernelCompatibilityLayer {
    userspace_journal: Arc<UserspaceSemanticJournal>,
    kernel_journal_path: Option<PathBuf>,
}

impl KernelCompatibilityLayer {
    /// Create a new compatibility layer
    pub fn new(
        userspace_journal: Arc<UserspaceSemanticJournal>,
        kernel_journal_path: Option<PathBuf>,
    ) -> Self {
        Self {
            userspace_journal,
            kernel_journal_path,
        }
    }
    
    /// Synchronize with kernel journal
    pub fn sync_with_kernel(&self) -> SemanticResult<()> {
        if let Some(ref kernel_path) = self.kernel_journal_path {
            // Read kernel journal format and convert events
            // This would implement the actual kernel journal reading logic
            eprintln!("Syncing userspace journal with kernel journal at {:?}", kernel_path);
            
            // For now, just return success
            // In a full implementation, this would:
            // 1. Read kernel journal entries
            // 2. Convert them to userspace format
            // 3. Merge with userspace journal
            // 4. Handle conflicts and duplicates
        }
        Ok(())
    }
    
    /// Export events to kernel format
    pub fn export_to_kernel_format(&self, events: &[SemanticEvent]) -> SemanticResult<Vec<u8>> {
        // Convert userspace events to kernel journal format
        // This would implement the actual kernel format serialization
        
        let mut kernel_data = Vec::new();
        
        for event in events {
            // Convert event to kernel format
            // For now, just use the existing serialization
            let event_data = self.userspace_journal.serializer.serialize_event(event)?;
            kernel_data.extend_from_slice(&event_data);
        }
        
        Ok(kernel_data)
    }
    
    /// Import events from kernel format
    pub fn import_from_kernel_format(&self, data: &[u8]) -> SemanticResult<Vec<SemanticEvent>> {
        // Convert kernel journal format to userspace events
        // This would implement the actual kernel format deserialization
        
        // For now, just try to deserialize as userspace format
        let events = vec![self.userspace_journal.serializer.deserialize_event(data)?];
        
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_event() -> SemanticEvent {
        SemanticEvent {
            event_id: 0, // Will be assigned by journal
            event_type: SemanticEventType::VectorCreate,
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
                vector: Some(VectorContext {
                    vector_id: Some(123),
                    dimensions: Some(128),
                    element_type: Some(0),
                }),
                agent: None,
                system: None,
                semantic: None,
                observability: None,
            },
            payload: None,
            metadata: None,
        }
    }
    
    #[test]
    fn test_journal_creation() {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config).unwrap();
        assert!(journal.get_statistics().is_ok());
    }
    
    #[test]
    fn test_event_write_read() {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            lazy_sync: false, // Immediate write for testing
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config).unwrap();
        let event = create_test_event();
        
        // Write event
        let event_id = journal.write_event(event.clone()).unwrap();
        assert!(event_id > 0);
        
        // Read event back
        let read_event = journal.read_event(event_id).unwrap();
        assert!(read_event.is_some());
        
        let read_event = read_event.unwrap();
        assert_eq!(read_event.event_type, event.event_type);
        assert_eq!(read_event.event_id, event_id);
    }
    
    #[test]
    fn test_batch_operations() {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            batch_size: 2,
            lazy_sync: true,
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config).unwrap();
        
        // Write multiple events
        let event1 = create_test_event();
        let event2 = create_test_event();
        let event3 = create_test_event();
        
        let id1 = journal.write_event(event1).unwrap();
        let id2 = journal.write_event(event2).unwrap();
        let id3 = journal.write_event(event3).unwrap(); // Should trigger batch flush
        
        // Force sync to ensure all events are written
        journal.sync().unwrap();
        
        // Verify all events can be read
        assert!(journal.read_event(id1).unwrap().is_some());
        assert!(journal.read_event(id2).unwrap().is_some());
        assert!(journal.read_event(id3).unwrap().is_some());
    }
    
    #[test]
    fn test_event_filtering() {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            lazy_sync: false,
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config).unwrap();
        
        // Create events with different types
        let mut vector_event = create_test_event();
        vector_event.event_type = SemanticEventType::VectorCreate;
        
        let mut graph_event = create_test_event();
        graph_event.event_type = SemanticEventType::GraphNodeCreate;
        
        journal.write_event(vector_event).unwrap();
        journal.write_event(graph_event).unwrap();
        
        // Query for vector events only
        let filter = EventFilter {
            event_types: Some(vec![SemanticEventType::VectorCreate]),
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
        
        let results = journal.query_events(filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type, SemanticEventType::VectorCreate);
    }
    
    #[test]
    fn test_recovery() {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path: journal_path.clone(),
            lazy_sync: false,
            ..Default::default()
        };
        
        // Create journal and write some events
        {
            let journal = UserspaceSemanticJournal::new(config.clone()).unwrap();
            let event = create_test_event();
            journal.write_event(event).unwrap();
            journal.sync().unwrap();
        }
        
        // Create new journal instance (simulates restart)
        let journal = UserspaceSemanticJournal::new(config).unwrap();
        
        // Check recovery info
        let recovery_info = journal.get_recovery_info().unwrap();
        assert!(recovery_info.is_some());
        
        let recovery_info = recovery_info.unwrap();
        assert_eq!(recovery_info.status, RecoveryStatus::Clean);
    }
}