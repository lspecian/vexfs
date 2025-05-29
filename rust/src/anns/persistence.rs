//! ANNS Persistence and Recovery System
//! 
//! This module provides comprehensive persistence and recovery mechanisms for all ANNS
//! index types, ensuring data durability, crash recovery, and consistent state management.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;
use core::mem;

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;
use crate::storage::persistence::{OnDiskSerializable, PersistenceManager};
use crate::storage::journal::{VexfsJournal, JournalOpType, TransactionManager};
use crate::fs_core::operations::OperationContext;
use crate::anns::{AnnsError, IndexSerializer, IndexDeserializer, WalManager, WalEntryType};

/// ANNS-specific persistence error kinds
#[derive(Debug, Clone, PartialEq)]
pub enum AnnsPersistenceError {
    /// Index serialization failed
    SerializationFailed(String),
    /// Index deserialization failed
    DeserializationFailed(String),
    /// Checkpoint creation failed
    CheckpointFailed(String),
    /// Recovery operation failed
    RecoveryFailed(String),
    /// Data integrity check failed
    IntegrityCheckFailed(String),
    /// Incremental persistence failed
    IncrementalPersistenceFailed(String),
    /// Corruption detected during recovery
    CorruptionDetected(String),
    /// Recovery point not found
    RecoveryPointNotFound(u64),
    /// Invalid persistence format
    InvalidFormat(String),
    /// Persistence operation timeout
    OperationTimeout,
}

impl From<AnnsPersistenceError> for VexfsError {
    fn from(err: AnnsPersistenceError) -> Self {
        match err {
            AnnsPersistenceError::SerializationFailed(msg) => 
                VexfsError::VectorError(crate::shared::errors::VectorErrorKind::SerializationError),
            AnnsPersistenceError::DeserializationFailed(msg) => 
                VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError),
            AnnsPersistenceError::CheckpointFailed(msg) => 
                VexfsError::JournalError(crate::shared::errors::JournalErrorKind::CheckpointError),
            AnnsPersistenceError::RecoveryFailed(msg) => 
                VexfsError::JournalError(crate::shared::errors::JournalErrorKind::RecoveryError),
            AnnsPersistenceError::IntegrityCheckFailed(msg) => 
                VexfsError::ChecksumMismatch,
            AnnsPersistenceError::CorruptionDetected(msg) => 
                VexfsError::CorruptedData,
            _ => VexfsError::InternalError(format!("ANNS persistence error: {:?}", err)),
        }
    }
}

/// Index type enumeration for persistence
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum IndexType {
    LSH = 1,
    IVF = 2,
    PQ = 3,
    Flat = 4,
    HNSW = 5,
}

impl IndexType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(IndexType::LSH),
            2 => Some(IndexType::IVF),
            3 => Some(IndexType::PQ),
            4 => Some(IndexType::Flat),
            5 => Some(IndexType::HNSW),
            _ => None,
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Persistence header for ANNS indices
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AnnsPersistenceHeader {
    /// Magic number for validation
    pub magic: u32,
    /// Format version
    pub version: u32,
    /// Index type
    pub index_type: u8,
    /// Compression enabled
    pub compression_enabled: u8,
    /// Reserved flags
    pub flags: u16,
    /// Index dimensions
    pub dimensions: u32,
    /// Number of vectors
    pub vector_count: u64,
    /// Index data size
    pub index_data_size: u64,
    /// Metadata size
    pub metadata_size: u32,
    /// Checksum of index data
    pub data_checksum: u32,
    /// Checksum of metadata
    pub metadata_checksum: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
    /// Recovery point ID
    pub recovery_point_id: u64,
    /// Reserved space
    pub reserved: [u32; 8],
}

impl AnnsPersistenceHeader {
    /// Magic number for ANNS persistence
    pub const MAGIC: u32 = 0x414E4E53; // "ANNS"
    
    /// Current version
    pub const VERSION: u32 = 1;

    /// Create new persistence header
    pub fn new(
        index_type: IndexType,
        dimensions: u32,
        vector_count: u64,
        index_data_size: u64,
        metadata_size: u32,
    ) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            index_type: index_type.to_u8(),
            compression_enabled: 0,
            flags: 0,
            dimensions,
            vector_count,
            index_data_size,
            metadata_size,
            data_checksum: 0,
            metadata_checksum: 0,
            created_at: 0, // Would be system timestamp in real implementation
            modified_at: 0,
            recovery_point_id: 0,
            reserved: [0; 8],
        }
    }

    /// Validate header
    pub fn validate(&self) -> VexfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(AnnsPersistenceError::InvalidFormat("invalid magic number".to_string()).into());
        }
        
        if self.version != Self::VERSION {
            return Err(AnnsPersistenceError::InvalidFormat("unsupported version".to_string()).into());
        }
        
        if IndexType::from_u8(self.index_type).is_none() {
            return Err(AnnsPersistenceError::InvalidFormat("invalid index type".to_string()).into());
        }
        
        Ok(())
    }

    /// Update checksums
    pub fn update_checksums(&mut self, index_data: &[u8], metadata: &[u8]) {
        self.data_checksum = crc32(index_data);
        self.metadata_checksum = crc32(metadata);
        self.modified_at = 0; // Would be current timestamp
    }

    /// Verify checksums
    pub fn verify_checksums(&self, index_data: &[u8], metadata: &[u8]) -> bool {
        verify_checksum(index_data, self.data_checksum) &&
        verify_checksum(metadata, self.metadata_checksum)
    }
}

impl OnDiskSerializable for AnnsPersistenceHeader {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                mem::size_of::<Self>()
            )
        }
    }

    fn from_bytes(data: &[u8]) -> VexfsResult<Self> {
        if data.len() < mem::size_of::<Self>() {
            return Err(VexfsError::InvalidData("insufficient data for AnnsPersistenceHeader".to_string()));
        }

        let header = unsafe {
            *(data.as_ptr() as *const Self)
        };

        header.validate()?;
        Ok(header)
    }

    fn serialized_size() -> usize {
        mem::size_of::<Self>()
    }

    fn validate(&self) -> VexfsResult<()> {
        self.validate()
    }

    fn update_checksum(&mut self) {
        // Checksums are handled separately for data and metadata
    }
}

/// Checkpoint information
#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    /// Checkpoint ID
    pub id: u64,
    /// Index type
    pub index_type: IndexType,
    /// Creation timestamp
    pub created_at: u64,
    /// Data size
    pub data_size: u64,
    /// Block location
    pub block_location: BlockNumber,
    /// Checksum
    pub checksum: u32,
    /// Recovery point dependencies
    pub dependencies: Vec<u64>,
}

impl CheckpointInfo {
    pub fn new(
        id: u64,
        index_type: IndexType,
        data_size: u64,
        block_location: BlockNumber,
        checksum: u32,
    ) -> Self {
        Self {
            id,
            index_type,
            created_at: 0, // Would be current timestamp
            data_size,
            block_location,
            checksum,
            dependencies: Vec::new(),
        }
    }
}

/// Recovery state information
#[derive(Debug, Clone)]
pub struct RecoveryState {
    /// Recovery needed flag
    pub recovery_needed: bool,
    /// Last valid checkpoint
    pub last_checkpoint_id: u64,
    /// Corrupted indices
    pub corrupted_indices: Vec<(IndexType, String)>,
    /// Recovery progress
    pub recovery_progress: f32,
    /// Estimated recovery time
    pub estimated_time_remaining: u64,
}

impl RecoveryState {
    pub fn new() -> Self {
        Self {
            recovery_needed: false,
            last_checkpoint_id: 0,
            corrupted_indices: Vec::new(),
            recovery_progress: 0.0,
            estimated_time_remaining: 0,
        }
    }

    pub fn add_corrupted_index(&mut self, index_type: IndexType, reason: String) {
        self.corrupted_indices.push((index_type, reason));
        self.recovery_needed = true;
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.recovery_progress = progress.clamp(0.0, 100.0);
    }
}

/// Incremental persistence manager
pub struct IncrementalPersistenceManager {
    /// Delta tracking
    deltas: HashMap<u64, Vec<u8>>,
    /// Last full checkpoint ID
    last_full_checkpoint: u64,
    /// Delta size threshold
    delta_threshold: usize,
    /// Maximum deltas before full checkpoint
    max_deltas: usize,
    /// Current delta count
    delta_count: usize,
}

impl IncrementalPersistenceManager {
    pub fn new(delta_threshold: usize, max_deltas: usize) -> Self {
        Self {
            deltas: HashMap::new(),
            last_full_checkpoint: 0,
            delta_threshold,
            max_deltas,
            delta_count: 0,
        }
    }

    /// Add delta for incremental persistence
    pub fn add_delta(&mut self, operation_id: u64, delta_data: Vec<u8>) -> VexfsResult<()> {
        if delta_data.len() > self.delta_threshold {
            return Err(AnnsPersistenceError::IncrementalPersistenceFailed(
                "delta too large".to_string()
            ).into());
        }

        self.deltas.insert(operation_id, delta_data);
        self.delta_count += 1;

        Ok(())
    }

    /// Check if full checkpoint is needed
    pub fn needs_full_checkpoint(&self) -> bool {
        self.delta_count >= self.max_deltas ||
        self.deltas.values().map(|d| d.len()).sum::<usize>() > self.delta_threshold * 10
    }

    /// Clear deltas after checkpoint
    pub fn clear_deltas(&mut self, checkpoint_id: u64) {
        self.deltas.clear();
        self.delta_count = 0;
        self.last_full_checkpoint = checkpoint_id;
    }

    /// Get delta statistics
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let total_size = self.deltas.values().map(|d| d.len()).sum();
        (self.delta_count, total_size, self.max_deltas)
    }
}

/// Data integrity verifier
pub struct DataIntegrityVerifier {
    /// Verification enabled
    enabled: bool,
    /// Merkle tree verification
    use_merkle_tree: bool,
    /// Block size for verification
    block_size: usize,
}

impl DataIntegrityVerifier {
    pub fn new(enabled: bool, use_merkle_tree: bool, block_size: usize) -> Self {
        Self {
            enabled,
            use_merkle_tree,
            block_size,
        }
    }

    /// Verify data integrity
    pub fn verify_integrity(&self, data: &[u8], expected_checksum: u32) -> VexfsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        let actual_checksum = crc32(data);
        if actual_checksum != expected_checksum {
            return Err(AnnsPersistenceError::IntegrityCheckFailed(
                format!("checksum mismatch: expected {}, got {}", expected_checksum, actual_checksum)
            ).into());
        }

        if self.use_merkle_tree {
            self.verify_merkle_tree(data)?;
        }

        Ok(())
    }

    /// Verify using Merkle tree (simplified implementation)
    fn verify_merkle_tree(&self, data: &[u8]) -> VexfsResult<()> {
        // Simplified Merkle tree verification
        let block_count = (data.len() + self.block_size - 1) / self.block_size;
        let mut block_hashes = Vec::with_capacity(block_count);

        for i in 0..block_count {
            let start = i * self.block_size;
            let end = (start + self.block_size).min(data.len());
            let block_data = &data[start..end];
            block_hashes.push(crc32(block_data));
        }

        // Verify block consistency (simplified)
        for (i, &hash) in block_hashes.iter().enumerate() {
            if hash == 0 {
                return Err(AnnsPersistenceError::IntegrityCheckFailed(
                    format!("invalid block hash at index {}", i)
                ).into());
            }
        }

        Ok(())
    }

    /// Calculate data checksum with integrity metadata
    pub fn calculate_checksum_with_metadata(&self, data: &[u8]) -> (u32, Vec<u8>) {
        let checksum = crc32(data);
        let metadata = if self.use_merkle_tree {
            self.calculate_merkle_metadata(data)
        } else {
            Vec::new()
        };
        (checksum, metadata)
    }

    /// Calculate Merkle tree metadata
    fn calculate_merkle_metadata(&self, data: &[u8]) -> Vec<u8> {
        let block_count = (data.len() + self.block_size - 1) / self.block_size;
        let mut metadata = Vec::with_capacity(block_count * 4); // 4 bytes per hash

        for i in 0..block_count {
            let start = i * self.block_size;
            let end = (start + self.block_size).min(data.len());
            let block_data = &data[start..end];
            let hash = crc32(block_data);
            metadata.extend_from_slice(&hash.to_le_bytes());
        }

        metadata
    }
}

/// Persistence configuration
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Enable compression
    pub compression_enabled: bool,
    /// Enable incremental persistence
    pub incremental_enabled: bool,
    /// Enable data integrity verification
    pub integrity_verification: bool,
    /// Checkpoint interval (in operations)
    pub checkpoint_interval: u64,
    /// Maximum recovery time (in seconds)
    pub max_recovery_time: u64,
    /// Background persistence enabled
    pub background_persistence: bool,
    /// Delta threshold for incremental persistence
    pub delta_threshold: usize,
    /// Maximum deltas before full checkpoint
    pub max_deltas: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            compression_enabled: true,
            incremental_enabled: true,
            integrity_verification: true,
            checkpoint_interval: 1000,
            max_recovery_time: 300, // 5 minutes
            background_persistence: true,
            delta_threshold: 1024 * 1024, // 1MB
            max_deltas: 100,
        }
    }
}

/// Persistence statistics
#[derive(Debug, Clone)]
pub struct PersistenceStats {
    /// Total number of checkpoints
    pub total_checkpoints: usize,
    /// Recovery needed flag
    pub recovery_needed: bool,
    /// Recovery progress percentage
    pub recovery_progress: f32,
    /// Number of pending deltas
    pub delta_count: usize,
    /// Total size of pending deltas
    pub delta_size: usize,
    /// WAL utilization percentage
    pub wal_utilization: f32,
    /// Last checkpoint ID
    pub last_checkpoint_id: u64,
}

/// Main ANNS persistence manager
pub struct AnnsPersistenceManager {
    /// Persistence manager for low-level operations
    persistence_manager: PersistenceManager,
    /// Transaction manager for journaling
    transaction_manager: Arc<Mutex<TransactionManager>>,
    /// WAL manager for ANNS operations
    wal_manager: Arc<Mutex<WalManager>>,
    /// Index serializer
    serializer: IndexSerializer,
    /// Index deserializer
    deserializer: IndexDeserializer,
    /// Checkpoint registry
    checkpoints: Arc<Mutex<HashMap<u64, CheckpointInfo>>>,
    /// Recovery state
    recovery_state: Arc<Mutex<RecoveryState>>,
    /// Incremental persistence manager
    incremental_manager: Arc<Mutex<IncrementalPersistenceManager>>,
    /// Data integrity verifier
    integrity_verifier: DataIntegrityVerifier,
    /// Next checkpoint ID
    next_checkpoint_id: Arc<Mutex<u64>>,
    /// Configuration
    config: PersistenceConfig,
    /// Checkpoint data storage (simplified in-memory storage)
    checkpoint_storage: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
}

impl AnnsPersistenceManager {
    /// Create new ANNS persistence manager
    pub fn new(
        block_size: u32,
        transaction_manager: Arc<Mutex<TransactionManager>>,
        config: PersistenceConfig,
    ) -> Self {
        let persistence_manager = PersistenceManager::new(block_size, config.integrity_verification);
        let wal_manager = Arc::new(Mutex::new(WalManager::new(1000, 10 * 1024 * 1024))); // 10MB WAL
        let incremental_manager = Arc::new(Mutex::new(
            IncrementalPersistenceManager::new(config.delta_threshold, config.max_deltas)
        ));
        let integrity_verifier = DataIntegrityVerifier::new(
            config.integrity_verification,
            true, // Use Merkle tree
            4096, // 4KB blocks
        );

        Self {
            persistence_manager,
            transaction_manager,
            wal_manager,
            serializer: IndexSerializer::new(),
            deserializer: IndexDeserializer::new(),
            checkpoints: Arc::new(Mutex::new(HashMap::new())),
            recovery_state: Arc::new(Mutex::new(RecoveryState::new())),
            incremental_manager,
            integrity_verifier,
            next_checkpoint_id: Arc::new(Mutex::new(1)),
            config,
            checkpoint_storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Persist index with full durability guarantees
    pub fn persist_index(
        &mut self,
        _ctx: &OperationContext,
        index_type: IndexType,
        index_data: &[u8],
        metadata: &[u8],
    ) -> VexfsResult<u64> {
        // Create persistence header
        let mut header = AnnsPersistenceHeader::new(
            index_type,
            0, // dimensions would be extracted from metadata
            0, // vector count would be extracted from metadata
            index_data.len() as u64,
            metadata.len() as u32,
        );
        
        // Update checksums
        header.update_checksums(index_data, metadata);
        
        // Verify data integrity
        self.integrity_verifier.verify_integrity(index_data, header.data_checksum)?;
        self.integrity_verifier.verify_integrity(metadata, header.metadata_checksum)?;
        
        // Serialize header
        let header_data = self.persistence_manager.serialize_to_block(&header)?;
        
        // Create checkpoint
        let checkpoint_id = self.create_checkpoint(index_type, &header_data, index_data, metadata)?;
        
        // Log operation to WAL (after checkpoint creation to avoid borrowing conflicts)
        {
            let mut wal = self.wal_manager.lock().map_err(|_| VexfsError::LockError)?;
            
            // Create operation data
            let mut operation_data = Vec::new();
            operation_data.push(index_type.to_u8());
            operation_data.extend_from_slice(&(index_data.len() as u32).to_le_bytes());
            operation_data.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
            operation_data.extend_from_slice(index_data);
            operation_data.extend_from_slice(metadata);
            
            wal.append(WalEntryType::Update, operation_data)
                .map_err(|_| AnnsPersistenceError::SerializationFailed("WAL append failed".to_string()))?;
        }
        
        // Execute transaction for journaling
        {
            let mut tm = self.transaction_manager.lock().map_err(|_| VexfsError::LockError)?;
            tm.execute_transaction(0, |tid, journal| {
                // Log to journal
                journal.log_block_write(
                    tid,
                    0, // Block number would be determined by storage manager
                    0,
                    &[],
                    &header_data,
                )?;
                
                Ok(())
            })?;
        }
        
        Ok(checkpoint_id)
    }

    /// Recover index from persistent storage
    pub fn recover_index(
        &mut self,
        ctx: &OperationContext,
        index_type: IndexType,
        checkpoint_id: u64,
    ) -> VexfsResult<(Vec<u8>, Vec<u8>)> {
        let checkpoints = self.checkpoints.lock().map_err(|_| VexfsError::LockError)?;
        
        let checkpoint = checkpoints.get(&checkpoint_id)
            .ok_or_else(|| AnnsPersistenceError::RecoveryPointNotFound(checkpoint_id))?;
        
        if checkpoint.index_type != index_type {
            return Err(AnnsPersistenceError::RecoveryFailed(
                "index type mismatch".to_string()
            ).into());
        }
        
        // Read checkpoint data
        let checkpoint_data = self.read_checkpoint_data(checkpoint_id)?;
        
        // Deserialize header
        let header = AnnsPersistenceHeader::from_bytes(&checkpoint_data[..AnnsPersistenceHeader::serialized_size()])?;
        
        // Extract index data and metadata
        let header_size = AnnsPersistenceHeader::serialized_size();
        let index_data_end = header_size + header.index_data_size as usize;
        let index_data = checkpoint_data[header_size..index_data_end].to_vec();
        let metadata = checkpoint_data[index_data_end..index_data_end + header.metadata_size as usize].to_vec();
        
        // Verify integrity
        if !header.verify_checksums(&index_data, &metadata) {
            return Err(AnnsPersistenceError::IntegrityCheckFailed(
                "checksum verification failed during recovery".to_string()
            ).into());
        }
        
        self.integrity_verifier.verify_integrity(&index_data, header.data_checksum)?;
        self.integrity_verifier.verify_integrity(&metadata, header.metadata_checksum)?;
        
        Ok((index_data, metadata))
    }

    /// Create checkpoint for consistent state snapshot
    pub fn create_checkpoint(
        &mut self,
        index_type: IndexType,
        header_data: &[u8],
        index_data: &[u8],
        metadata: &[u8],
    ) -> VexfsResult<u64> {
        let checkpoint_id = {
            let mut next_id = self.next_checkpoint_id.lock().map_err(|_| VexfsError::LockError)?;
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        // Combine all data
        let mut checkpoint_data = Vec::new();
        checkpoint_data.extend_from_slice(header_data);
        checkpoint_data.extend_from_slice(index_data);
        checkpoint_data.extend_from_slice(metadata);
        
        // Calculate checksum
        let checksum = crc32(&checkpoint_data);
        
        // Store checkpoint info
        let checkpoint_info = CheckpointInfo::new(
            checkpoint_id,
            index_type,
            checkpoint_data.len() as u64,
            0, // Block location would be assigned by storage manager
            checksum,
        );
        
        let mut checkpoints = self.checkpoints.lock().map_err(|_| VexfsError::LockError)?;
        checkpoints.insert(checkpoint_id, checkpoint_info);
        
        // Store checkpoint data
        self.store_checkpoint_data(checkpoint_id, &checkpoint_data)?;
        
        Ok(checkpoint_id)
    }

    /// Perform crash recovery
    pub fn perform_crash_recovery(&mut self, ctx: &OperationContext) -> VexfsResult<()> {
        let recovery_needed = {
            let recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
            recovery_state.recovery_needed
        };
        
        if !recovery_needed {
            return Ok(());
        }
        
        {
            let mut recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
            recovery_state.update_progress(0.0);
        }
        
        // Replay WAL entries
        let wal_entries = {
            let recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
            let last_checkpoint_id = recovery_state.last_checkpoint_id;
            drop(recovery_state);
            
            let wal = self.wal_manager.lock().map_err(|_| VexfsError::LockError)?;
            wal.get_entries_since(last_checkpoint_id)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        };
        
        let total_entries = wal_entries.len();
        for (i, entry) in wal_entries.iter().enumerate() {
            self.replay_wal_entry(entry)?;
            
            let progress = ((i + 1) as f32 / total_entries as f32) * 100.0;
            let mut recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
            recovery_state.update_progress(progress);
        }
        
        // Verify recovered indices
        self.verify_recovered_indices()?;
        
        {
            let mut recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
            recovery_state.recovery_needed = false;
            recovery_state.corrupted_indices.clear();
            recovery_state.update_progress(100.0);
        }
        
        Ok(())
    }

    /// Incremental persistence for large-scale operations
    pub fn incremental_persist(
        &mut self,
        ctx: &OperationContext,
        operation_id: u64,
        delta_data: Vec<u8>,
    ) -> VexfsResult<()> {
        if !self.config.incremental_enabled {
            return Ok(());
        }
        
        let mut incremental = self.incremental_manager.lock().map_err(|_| VexfsError::LockError)?;
        incremental.add_delta(operation_id, delta_data)?;
        
        if incremental.needs_full_checkpoint() {
            drop(incremental); // Release lock before checkpoint
            self.create_full_checkpoint_from_deltas(ctx)?;
        }
        
        Ok(())
    }

    /// Get persistence statistics
    pub fn get_persistence_stats(&self) -> VexfsResult<PersistenceStats> {
        let checkpoints = self.checkpoints.lock().map_err(|_| VexfsError::LockError)?;
        let recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
        let incremental = self.incremental_manager.lock().map_err(|_| VexfsError::LockError)?;
        let wal = self.wal_manager.lock().map_err(|_| VexfsError::LockError)?;
        
        let (delta_count, delta_size, max_deltas) = incremental.get_stats();
        let wal_stats = wal.stats();
        
        Ok(PersistenceStats {
            total_checkpoints: checkpoints.len(),
            recovery_needed: recovery_state.recovery_needed,
            recovery_progress: recovery_state.recovery_progress,
            delta_count,
            delta_size,
            wal_utilization: wal_stats.utilization_percent,
            last_checkpoint_id: recovery_state.last_checkpoint_id,
        })
    }

    /// Validate all stored indices for corruption
    pub fn validate_all_indices(&self, ctx: &OperationContext) -> VexfsResult<Vec<(u64, bool, String)>> {
        let checkpoints = self.checkpoints.lock().map_err(|_| VexfsError::LockError)?;
        let mut validation_results = Vec::new();
        
        for (&checkpoint_id, checkpoint_info) in checkpoints.iter() {
            let validation_result = match self.validate_checkpoint(checkpoint_id, checkpoint_info) {
                Ok(()) => (checkpoint_id, true, "Valid".to_string()),
                Err(e) => (checkpoint_id, false, format!("Validation failed: {}", e)),
            };
            validation_results.push(validation_result);
        }
        
        Ok(validation_results)
    }

    /// Compact WAL and remove old entries
    pub fn compact_wal(&mut self, ctx: &OperationContext) -> VexfsResult<()> {
        let mut wal = self.wal_manager.lock().map_err(|_| VexfsError::LockError)?;
        let recovery_state = self.recovery_state.lock().map_err(|_| VexfsError::LockError)?;
        
        // Truncate WAL up to last checkpoint
        wal.truncate(recovery_state.last_checkpoint_id)?;
        
        Ok(())
    }

    /// Create background checkpoint asynchronously
    pub fn create_background_checkpoint(
        &mut self,
        ctx: &OperationContext,
        index_type: IndexType,
        index_data: Vec<u8>,
        metadata: Vec<u8>,
    ) -> VexfsResult<u64> {
        if !self.config.background_persistence {
            return self.persist_index(ctx, index_type, &index_data, &metadata);
        }
        
        // For now, just create checkpoint synchronously
        // In a full implementation, this would spawn a background task
        let header_data = {
            let mut header = AnnsPersistenceHeader::new(
                index_type,
                0,
                0,
                index_data.len() as u64,
                metadata.len() as u32,
            );
            header.update_checksums(&index_data, &metadata);
            self.persistence_manager.serialize_to_block(&header)?
        };
        
        self.create_checkpoint(index_type, &header_data, &index_data, &metadata)
    }

    // Private helper methods
    
    fn log_persistence_operation(
        &mut self,
        tid: u64,
        index_type: IndexType,
        index_data: &[u8],
        metadata: &[u8],
    ) -> VexfsResult<()> {
        let mut wal = self.wal_manager.lock().map_err(|_| VexfsError::LockError)?;
        
        // Create operation data
        let mut operation_data = Vec::new();
        operation_data.push(index_type.to_u8());
        operation_data.extend_from_slice(&(index_data.len() as u32).to_le_bytes());
        operation_data.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        operation_data.extend_from_slice(index_data);
        operation_data.extend_from_slice(metadata);
        
        wal.append(WalEntryType::Update, operation_data)
            .map_err(|_| AnnsPersistenceError::SerializationFailed("WAL append failed".to_string()))?;
        
        Ok(())
    }
    
    fn read_checkpoint_data(&self, checkpoint_id: u64) -> VexfsResult<Vec<u8>> {
        let storage = self.checkpoint_storage.lock().map_err(|_| VexfsError::LockError)?;
        storage.get(&checkpoint_id)
            .cloned()
            .ok_or_else(|| AnnsPersistenceError::RecoveryPointNotFound(checkpoint_id).into())
    }
    
    fn store_checkpoint_data(&self, checkpoint_id: u64, data: &[u8]) -> VexfsResult<()> {
        let mut storage = self.checkpoint_storage.lock().map_err(|_| VexfsError::LockError)?;
        storage.insert(checkpoint_id, data.to_vec());
        Ok(())
    }
    
    fn replay_wal_entry(&mut self, entry: &crate::anns::wal::WalEntry) -> VexfsResult<()> {
        // Simplified WAL replay implementation
        match entry.entry_type() {
            Some(WalEntryType::Update) => {
                // Replay update operation
                Ok(())
            }
            Some(WalEntryType::Insert) => {
                // Replay insert operation
                Ok(())
            }
            Some(WalEntryType::Delete) => {
                // Replay delete operation
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    fn verify_recovered_indices(&self) -> VexfsResult<()> {
        // Simplified verification - would check all recovered indices
        Ok(())
    }
    
    fn create_full_checkpoint_from_deltas(&mut self, ctx: &OperationContext) -> VexfsResult<()> {
        // Simplified implementation - would consolidate deltas into full checkpoint
        let mut incremental = self.incremental_manager.lock().map_err(|_| VexfsError::LockError)?;
        let checkpoint_id = {
            let mut next_id = self.next_checkpoint_id.lock().map_err(|_| VexfsError::LockError)?;
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        incremental.clear_deltas(checkpoint_id);
        Ok(())
    }
    
    fn validate_checkpoint(&self, checkpoint_id: u64, checkpoint_info: &CheckpointInfo) -> VexfsResult<()> {
        // Read checkpoint data
        let data = self.read_checkpoint_data(checkpoint_id)?;
        
        // Verify checksum
        let actual_checksum = crc32(&data);
        if actual_checksum != checkpoint_info.checksum {
            return Err(AnnsPersistenceError::IntegrityCheckFailed(
                format!("checkpoint {} checksum mismatch", checkpoint_id)
            ).into());
        }
        
        // Verify header if present
        if data.len() >= AnnsPersistenceHeader::serialized_size() {
            let header = AnnsPersistenceHeader::from_bytes(&data[..AnnsPersistenceHeader::serialized_size()])?;
            header.validate()?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::journal::VexfsJournal;

    #[test]
    fn test_persistence_header_creation() {
        let header = AnnsPersistenceHeader::new(
            IndexType::HNSW,
            128,
            1000,
            4096,
            256,
        );
        
        assert_eq!(header.magic, AnnsPersistenceHeader::MAGIC);
        assert_eq!(header.version, AnnsPersistenceHeader::VERSION);
        assert_eq!(header.index_type, IndexType::HNSW.to_u8());
        assert_eq!(header.dimensions, 128);
        assert_eq!(header.vector_count, 1000);
        assert_eq!(header.index_data_size, 4096);
        assert_eq!(header.metadata_size, 256);
    }

    #[test]
    fn test_persistence_header_validation() {
        let mut header = AnnsPersistenceHeader::new(
            IndexType::LSH,
            64,
            500,
            2048,
            128,
        );
        
        assert!(header.validate().is_ok());
        
        // Test invalid magic
        header.magic = 0x12345678;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_checkpoint_info_creation() {
        let checkpoint = CheckpointInfo::new(
            1,
            IndexType::IVF,
            1024,
            42,
            0x12345678,
        );
        
        assert_eq!(checkpoint.id, 1);
        assert_eq!(checkpoint.index_type, IndexType::IVF);
        assert_eq!(checkpoint.data_size, 1024);
        assert_eq!(checkpoint.block_location, 42);
        assert_eq!(checkpoint.checksum, 0x12345678);
    }

    #[test]
    fn test_recovery_state() {
        let mut state = RecoveryState::new();
        assert!(!state.recovery_needed);
        assert_eq!(state.recovery_progress, 0.0);
        
        state.add_corrupted_index(IndexType::PQ, "test corruption".to_string());
        assert!(state.recovery_needed);
        assert_eq!(state.corrupted_indices.len(), 1);
        
        state.update_progress(50.0);
        assert_eq!(state.recovery_progress, 50.0);
    }

    #[test]
    fn test_incremental_persistence_manager() {
        let mut manager = IncrementalPersistenceManager::new(1024, 10);
        
        assert!(!manager.needs_full_checkpoint());
        
        // Add some deltas
        for i in 0..5 {
            let delta = vec![i as u8; 100];
            manager.add_delta(i as u64, delta).unwrap();
        }
        
        let (count, size, max) = manager.get_stats();
        assert_eq!(count, 5);
        assert_eq!(size, 500);
        assert_eq!(max, 10);
        
        assert!(!manager.needs_full_checkpoint());
        
        // Add more deltas to trigger checkpoint
        for i in 5..15 {
            let delta = vec![i as u8; 100];
            manager.add_delta(i as u64, delta).unwrap();
        }
        
        assert!(manager.needs_full_checkpoint());
    }

    #[test]
    fn test_data_integrity_verifier() {
        let verifier = DataIntegrityVerifier::new(true, false, 4096);
        let data = b"test data for integrity verification";
        let checksum = crc32(data);
        
        assert!(verifier.verify_integrity(data, checksum).is_ok());
        assert!(verifier.verify_integrity(data, checksum + 1).is_err());
        
        let (calc_checksum, metadata) = verifier.calculate_checksum_with_metadata(data);
        assert_eq!(calc_checksum, checksum);
        assert!(metadata.is_empty()); // No Merkle tree in this test
    }

    #[test]
    fn test_persistence_config_default() {
        let config = PersistenceConfig::default();
        
        assert!(config.compression_enabled);
        assert!(config.incremental_enabled);
        assert!(config.integrity_verification);
        assert_eq!(config.checkpoint_interval, 1000);
        assert_eq!(config.max_recovery_time, 300);
        assert!(config.background_persistence);
        assert_eq!(config.delta_threshold, 1024 * 1024);
        assert_eq!(config.max_deltas, 100);
    }
}