//! Write-ahead Logging for Index Updates
//! 
//! This module implements crash recovery system with atomic commits
//! and log replay for maintaining index consistency.

#![no_std]

use core::{mem, ptr, slice};
use crate::anns::{AnnsError, SearchResult};
use crate::vector_storage::VectorDataType;

/// WAL file format version
pub const WAL_FORMAT_VERSION: u32 = 1;

/// WAL file magic number
pub const WAL_FILE_MAGIC: u32 = 0x56455741; // "VEWA"

/// Maximum WAL file size (64MB for kernel space)
pub const MAX_WAL_SIZE: u64 = 64 * 1024 * 1024;

/// WAL checkpoint interval (number of operations)
pub const CHECKPOINT_INTERVAL: u32 = 10000;

/// Maximum transaction size
pub const MAX_TRANSACTION_SIZE: u32 = 1000;

/// WAL file header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WalFileHeader {
    /// File magic number
    pub magic: u32,
    /// File format version
    pub version: u32,
    /// Total file size
    pub file_size: u64,
    /// Number of entries in WAL
    pub entry_count: u64,
    /// Last committed transaction ID
    pub last_committed_tx: u64,
    /// Last checkpoint transaction ID
    pub last_checkpoint_tx: u64,
    /// WAL creation timestamp
    pub created_timestamp: u64,
    /// Last modification timestamp
    pub modified_timestamp: u64,
    /// File checksum
    pub checksum: u32,
    /// WAL flags
    pub flags: WalFlags,
    /// Reserved for future use
    pub reserved: [u8; 20],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WalFlags {
    /// WAL is currently being written
    pub active: bool,
    /// WAL needs recovery
    pub needs_recovery: bool,
    /// WAL is in checkpoint mode
    pub checkpointing: bool,
    /// WAL has been validated
    pub validated: bool,
}

impl WalFileHeader {
    pub const SIZE: usize = mem::size_of::<WalFileHeader>();

    pub fn new() -> Self {
        Self {
            magic: WAL_FILE_MAGIC,
            version: WAL_FORMAT_VERSION,
            file_size: Self::SIZE as u64,
            entry_count: 0,
            last_committed_tx: 0,
            last_checkpoint_tx: 0,
            created_timestamp: 0, // TODO: get kernel time
            modified_timestamp: 0,
            checksum: 0,
            flags: WalFlags {
                active: true,
                needs_recovery: false,
                checkpointing: false,
                validated: false,
            },
            reserved: [0; 20],
        }
    }
}

/// WAL entry header for individual operations
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WalEntryHeader {
    /// Entry magic for validation
    pub magic: u32,
    /// Transaction ID this entry belongs to
    pub transaction_id: u64,
    /// Sequence number within transaction
    pub sequence: u32,
    /// Operation type
    pub operation: WalOperation,
    /// Entry payload size
    pub payload_size: u32,
    /// Entry timestamp
    pub timestamp: u64,
    /// Entry checksum
    pub checksum: u32,
    /// Entry flags
    pub flags: WalEntryFlags,
}

/// WAL entry magic number
pub const WAL_ENTRY_MAGIC: u32 = 0x454E5459; // "ENTY"

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WalOperation {
    /// Insert new vector into index
    VectorInsert = 1,
    /// Delete vector from index
    VectorDelete = 2,
    /// Update existing vector
    VectorUpdate = 3,
    /// Create new layer
    LayerCreate = 4,
    /// Update layer metadata
    LayerUpdate = 5,
    /// Add connection between nodes
    ConnectionAdd = 6,
    /// Remove connection between nodes
    ConnectionRemove = 7,
    /// Begin transaction
    TransactionBegin = 8,
    /// Commit transaction
    TransactionCommit = 9,
    /// Rollback transaction
    TransactionRollback = 10,
    /// Checkpoint marker
    Checkpoint = 11,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WalEntryFlags {
    /// Entry is part of a transaction
    pub transactional: bool,
    /// Entry has been applied to index
    pub applied: bool,
    /// Entry is a compensating operation
    pub compensating: bool,
    /// Entry is critical for recovery
    pub critical: bool,
}

impl WalEntryHeader {
    pub const SIZE: usize = mem::size_of::<WalEntryHeader>();

    pub fn new(operation: WalOperation, transaction_id: u64, sequence: u32, payload_size: u32) -> Self {
        Self {
            magic: WAL_ENTRY_MAGIC,
            transaction_id,
            sequence,
            operation,
            payload_size,
            timestamp: 0, // TODO: get kernel time
            checksum: 0,
            flags: WalEntryFlags {
                transactional: transaction_id > 0,
                applied: false,
                compensating: false,
                critical: false,
            },
        }
    }
}

/// Vector insert operation payload
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VectorInsertPayload {
    /// Vector ID being inserted
    pub vector_id: u64,
    /// Vector dimensions
    pub dimensions: u32,
    /// Vector data type
    pub data_type: VectorDataType,
    /// Layer level for insertion
    pub level: u8,
    /// File offset of vector data
    pub data_offset: u64,
    /// Size of vector data
    pub data_size: u32,
    /// Reserved fields
    pub reserved: [u8; 7],
}

/// Vector delete operation payload
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VectorDeletePayload {
    /// Vector ID being deleted
    pub vector_id: u64,
    /// Layer level being deleted from
    pub level: u8,
    /// Reserved fields
    pub reserved: [u8; 7],
}

/// Connection operation payload
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ConnectionPayload {
    /// Source node ID
    pub source_id: u64,
    /// Target node ID
    pub target_id: u64,
    /// Layer level
    pub level: u8,
    /// Connection weight (distance)
    pub weight: f32,
    /// Reserved fields
    pub reserved: [u8; 3],
}

/// Transaction context for batched operations
#[derive(Debug)]
pub struct Transaction {
    /// Transaction ID
    pub id: u64,
    /// Operations in this transaction
    pub operations: [WalEntryHeader; 1000], // Fixed size for kernel
    /// Operation payloads
    pub payloads: [[u8; 128]; 1000], // Fixed payload size
    /// Number of operations
    pub operation_count: u32,
    /// Transaction status
    pub status: TransactionStatus,
    /// Total payload size
    pub total_payload_size: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
    Rolledback,
}

impl Transaction {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            operations: [WalEntryHeader::new(WalOperation::TransactionBegin, 0, 0, 0); 1000],
            payloads: [[0; 128]; 1000],
            operation_count: 0,
            status: TransactionStatus::Active,
            total_payload_size: 0,
        }
    }

    /// Add operation to transaction
    pub fn add_operation(
        &mut self,
        operation: WalOperation,
        payload: &[u8],
    ) -> Result<(), AnnsError> {
        if self.operation_count >= self.operations.len() as u32 {
            return Err(AnnsError::OutOfMemory);
        }

        if payload.len() > 128 {
            return Err(AnnsError::InvalidParameters);
        }

        let idx = self.operation_count as usize;
        self.operations[idx] = WalEntryHeader::new(
            operation,
            self.id,
            self.operation_count,
            payload.len() as u32,
        );

        // Copy payload
        self.payloads[idx][..payload.len()].copy_from_slice(payload);

        self.operation_count += 1;
        self.total_payload_size += payload.len() as u32;

        Ok(())
    }

    pub fn is_full(&self) -> bool {
        self.operation_count >= self.operations.len() as u32 ||
        self.total_payload_size >= (128 * 1000 - 1000) as u32 // Leave some margin
    }
}

/// WAL writer for logging operations
pub struct WalWriter {
    /// WAL file header
    pub header: WalFileHeader,
    /// Current write buffer
    pub buffer: [u8; 65536], // 64KB buffer
    /// Current buffer position
    pub buffer_pos: usize,
    /// Next transaction ID
    pub next_tx_id: u64,
    /// Current active transaction
    pub current_tx: Option<Transaction>,
    /// Writer statistics
    pub stats: WalStats,
}

#[derive(Debug, Clone, Copy)]
pub struct WalStats {
    pub entries_written: u64,
    pub transactions_committed: u64,
    pub transactions_aborted: u64,
    pub bytes_written: u64,
    pub checkpoints_performed: u32,
    pub flush_count: u32,
    pub write_errors: u32,
}

impl WalStats {
    pub fn new() -> Self {
        Self {
            entries_written: 0,
            transactions_committed: 0,
            transactions_aborted: 0,
            bytes_written: 0,
            checkpoints_performed: 0,
            flush_count: 0,
            write_errors: 0,
        }
    }
}

impl WalWriter {
    pub fn new() -> Self {
        Self {
            header: WalFileHeader::new(),
            buffer: [0; 65536],
            buffer_pos: WalFileHeader::SIZE, // Skip header space
            next_tx_id: 1,
            current_tx: None,
            stats: WalStats::new(),
        }
    }

    /// Begin a new transaction
    pub fn begin_transaction(&mut self) -> Result<u64, AnnsError> {
        // Commit any pending transaction
        if let Some(tx) = self.current_tx.take() {
            if matches!(tx.status, TransactionStatus::Active) {
                return Err(AnnsError::TransactionInProgress);
            }
        }

        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;

        self.current_tx = Some(Transaction::new(tx_id));

        // Write transaction begin marker
        let begin_entry = WalEntryHeader::new(
            WalOperation::TransactionBegin,
            tx_id,
            0,
            0,
        );

        self.write_entry_header(&begin_entry)?;

        Ok(tx_id)
    }

    /// Log vector insert operation
    pub fn log_vector_insert(
        &mut self,
        vector_id: u64,
        dimensions: u32,
        data_type: VectorDataType,
        level: u8,
        data_offset: u64,
        data_size: u32,
    ) -> Result<(), AnnsError> {
        let payload = VectorInsertPayload {
            vector_id,
            dimensions,
            data_type,
            level,
            data_offset,
            data_size,
            reserved: [0; 7],
        };

        let payload_bytes = unsafe {
            slice::from_raw_parts(
                &payload as *const _ as *const u8,
                mem::size_of::<VectorInsertPayload>(),
            )
        };

        self.log_operation(WalOperation::VectorInsert, payload_bytes)
    }

    /// Log vector delete operation
    pub fn log_vector_delete(&mut self, vector_id: u64, level: u8) -> Result<(), AnnsError> {
        let payload = VectorDeletePayload {
            vector_id,
            level,
            reserved: [0; 7],
        };

        let payload_bytes = unsafe {
            slice::from_raw_parts(
                &payload as *const _ as *const u8,
                mem::size_of::<VectorDeletePayload>(),
            )
        };

        self.log_operation(WalOperation::VectorDelete, payload_bytes)
    }

    /// Log connection add operation
    pub fn log_connection_add(
        &mut self,
        source_id: u64,
        target_id: u64,
        level: u8,
        weight: f32,
    ) -> Result<(), AnnsError> {
        let payload = ConnectionPayload {
            source_id,
            target_id,
            level,
            weight,
            reserved: [0; 3],
        };

        let payload_bytes = unsafe {
            slice::from_raw_parts(
                &payload as *const _ as *const u8,
                mem::size_of::<ConnectionPayload>(),
            )
        };

        self.log_operation(WalOperation::ConnectionAdd, payload_bytes)
    }

    /// Log generic operation
    fn log_operation(&mut self, operation: WalOperation, payload: &[u8]) -> Result<(), AnnsError> {
        // Add to current transaction if active
        if let Some(ref mut tx) = self.current_tx {
            tx.add_operation(operation, payload)?;
            
            // Auto-commit if transaction is full
            if tx.is_full() {
                let tx_id = tx.id;
                self.commit_transaction(tx_id)?;
            }
        } else {
            // Write directly as single operation
            let entry = WalEntryHeader::new(operation, 0, 0, payload.len() as u32);
            self.write_entry_header(&entry)?;
            self.write_payload(payload)?;
        }

        Ok(())
    }

    /// Commit current transaction
    pub fn commit_transaction(&mut self, tx_id: u64) -> Result<(), AnnsError> {
        let tx = match self.current_tx.take() {
            Some(tx) if tx.id == tx_id => tx,
            Some(tx) => {
                self.current_tx = Some(tx);
                return Err(AnnsError::InvalidParameters);
            }
            None => return Err(AnnsError::TransactionNotFound),
        };

        // Write all transaction operations
        for i in 0..tx.operation_count as usize {
            self.write_entry_header(&tx.operations[i])?;
            let payload_size = tx.operations[i].payload_size as usize;
            if payload_size > 0 {
                self.write_payload(&tx.payloads[i][..payload_size])?;
            }
        }

        // Write commit marker
        let commit_entry = WalEntryHeader::new(
            WalOperation::TransactionCommit,
            tx_id,
            tx.operation_count,
            0,
        );

        self.write_entry_header(&commit_entry)?;

        self.header.last_committed_tx = tx_id;
        self.stats.transactions_committed += 1;

        // Check if checkpoint is needed
        if self.stats.entries_written % CHECKPOINT_INTERVAL as u64 == 0 {
            self.checkpoint()?;
        }

        Ok(())
    }

    /// Abort current transaction
    pub fn abort_transaction(&mut self, tx_id: u64) -> Result<(), AnnsError> {
        if let Some(tx) = self.current_tx.take() {
            if tx.id != tx_id {
                self.current_tx = Some(tx);
                return Err(AnnsError::InvalidParameters);
            }
        }

        // Write rollback marker
        let rollback_entry = WalEntryHeader::new(
            WalOperation::TransactionRollback,
            tx_id,
            0,
            0,
        );

        self.write_entry_header(&rollback_entry)?;
        self.stats.transactions_aborted += 1;

        Ok(())
    }

    /// Write entry header to buffer
    fn write_entry_header(&mut self, entry: &WalEntryHeader) -> Result<(), AnnsError> {
        if self.buffer_pos + WalEntryHeader::SIZE > self.buffer.len() {
            self.flush_buffer()?;
        }

        unsafe {
            let entry_bytes = slice::from_raw_parts(
                entry as *const _ as *const u8,
                WalEntryHeader::SIZE,
            );
            self.buffer[self.buffer_pos..self.buffer_pos + WalEntryHeader::SIZE]
                .copy_from_slice(entry_bytes);
        }

        self.buffer_pos += WalEntryHeader::SIZE;
        self.stats.entries_written += 1;

        Ok(())
    }

    /// Write payload to buffer
    fn write_payload(&mut self, payload: &[u8]) -> Result<(), AnnsError> {
        if self.buffer_pos + payload.len() > self.buffer.len() {
            self.flush_buffer()?;
        }

        self.buffer[self.buffer_pos..self.buffer_pos + payload.len()]
            .copy_from_slice(payload);

        self.buffer_pos += payload.len();

        Ok(())
    }

    /// Flush buffer to storage
    fn flush_buffer(&mut self) -> Result<(), AnnsError> {
        if self.buffer_pos <= WalFileHeader::SIZE {
            return Ok(());
        }

        // TODO: Actual write to storage would happen here
        // For now, just reset buffer
        
        self.stats.bytes_written += self.buffer_pos as u64;
        self.stats.flush_count += 1;
        
        // Update file header
        self.header.file_size += (self.buffer_pos - WalFileHeader::SIZE) as u64;
        self.header.modified_timestamp = 0; // TODO: get kernel time

        // Write header at beginning of buffer
        unsafe {
            let header_bytes = slice::from_raw_parts(
                &self.header as *const _ as *const u8,
                WalFileHeader::SIZE,
            );
            self.buffer[..WalFileHeader::SIZE].copy_from_slice(header_bytes);
        }

        // Reset buffer position
        self.buffer_pos = WalFileHeader::SIZE;

        Ok(())
    }

    /// Create checkpoint
    pub fn checkpoint(&mut self) -> Result<(), AnnsError> {
        // Flush any pending data
        self.flush_buffer()?;

        // Write checkpoint marker
        let checkpoint_entry = WalEntryHeader::new(
            WalOperation::Checkpoint,
            0,
            0,
            0,
        );

        self.write_entry_header(&checkpoint_entry)?;
        self.flush_buffer()?;

        self.header.last_checkpoint_tx = self.header.last_committed_tx;
        self.stats.checkpoints_performed += 1;

        Ok(())
    }

    /// Get WAL statistics
    pub fn get_stats(&self) -> WalStats {
        self.stats
    }
}

/// WAL reader for recovery operations
pub struct WalReader {
    /// WAL data buffer
    pub data: *const u8,
    /// Data size
    pub size: usize,
    /// Current read position
    pub position: usize,
    /// File header
    pub header: WalFileHeader,
    /// Reader statistics
    pub stats: ReaderStats,
}

#[derive(Debug, Clone, Copy)]
pub struct ReaderStats {
    pub entries_read: u64,
    pub transactions_replayed: u64,
    pub operations_applied: u64,
    pub errors_encountered: u32,
    pub recovery_time_ms: u64,
}

impl ReaderStats {
    pub fn new() -> Self {
        Self {
            entries_read: 0,
            transactions_replayed: 0,
            operations_applied: 0,
            errors_encountered: 0,
            recovery_time_ms: 0,
        }
    }
}

impl WalReader {
    pub fn new(data: *const u8, size: usize) -> Result<Self, AnnsError> {
        if size < WalFileHeader::SIZE {
            return Err(AnnsError::InvalidFormat);
        }

        // Read and validate header
        let header = unsafe {
            ptr::read_unaligned(data as *const WalFileHeader)
        };

        if header.magic != WAL_FILE_MAGIC {
            return Err(AnnsError::InvalidFormat);
        }

        if header.version != WAL_FORMAT_VERSION {
            return Err(AnnsError::InvalidFormat);
        }

        Ok(Self {
            data,
            size,
            position: WalFileHeader::SIZE,
            header,
            stats: ReaderStats::new(),
        })
    }

    /// Read next WAL entry
    pub fn read_next_entry(&mut self) -> Result<Option<(WalEntryHeader, &[u8])>, AnnsError> {
        if self.position + WalEntryHeader::SIZE > self.size {
            return Ok(None);
        }

        // Read entry header
        let entry_header = unsafe {
            ptr::read_unaligned(
                self.data.add(self.position) as *const WalEntryHeader
            )
        };

        if entry_header.magic != WAL_ENTRY_MAGIC {
            return Err(AnnsError::CorruptedIndex);
        }

        self.position += WalEntryHeader::SIZE;
        self.stats.entries_read += 1;

        // Read payload if present
        let payload = if entry_header.payload_size > 0 {
            if self.position + entry_header.payload_size as usize > self.size {
                return Err(AnnsError::CorruptedIndex);
            }

            let payload_slice = unsafe {
                slice::from_raw_parts(
                    self.data.add(self.position),
                    entry_header.payload_size as usize,
                )
            };

            self.position += entry_header.payload_size as usize;
            payload_slice
        } else {
            &[]
        };

        Ok(Some((entry_header, payload)))
    }

    /// Replay WAL entries for recovery
    pub fn replay_log(&mut self) -> Result<ReplayResult, AnnsError> {
        let mut result = ReplayResult::new();
        let mut current_tx: Option<u64> = None;

        while let Some((entry, payload)) = self.read_next_entry()? {
            match entry.operation {
                WalOperation::TransactionBegin => {
                    current_tx = Some(entry.transaction_id);
                }
                WalOperation::TransactionCommit => {
                    if current_tx == Some(entry.transaction_id) {
                        result.committed_transactions += 1;
                        self.stats.transactions_replayed += 1;
                        current_tx = None;
                    }
                }
                WalOperation::TransactionRollback => {
                    if current_tx == Some(entry.transaction_id) {
                        result.aborted_transactions += 1;
                        current_tx = None;
                    }
                }
                WalOperation::VectorInsert |
                WalOperation::VectorDelete |
                WalOperation::VectorUpdate |
                WalOperation::ConnectionAdd |
                WalOperation::ConnectionRemove => {
                    // Apply operation to index
                    if let Err(_) = self.apply_operation(&entry, payload) {
                        self.stats.errors_encountered += 1;
                        result.failed_operations += 1;
                    } else {
                        self.stats.operations_applied += 1;
                        result.applied_operations += 1;
                    }
                }
                WalOperation::Checkpoint => {
                    result.checkpoints_found += 1;
                }
                _ => {
                    // Unknown operation type
                    self.stats.errors_encountered += 1;
                }
            }
        }

        Ok(result)
    }

    /// Apply individual operation during replay
    fn apply_operation(&self, entry: &WalEntryHeader, payload: &[u8]) -> Result<(), AnnsError> {
        match entry.operation {
            WalOperation::VectorInsert => {
                // TODO: Apply vector insertion to index
                Ok(())
            }
            WalOperation::VectorDelete => {
                // TODO: Apply vector deletion to index
                Ok(())
            }
            WalOperation::ConnectionAdd => {
                // TODO: Apply connection addition to index
                Ok(())
            }
            WalOperation::ConnectionRemove => {
                // TODO: Apply connection removal to index
                Ok(())
            }
            _ => Err(AnnsError::InvalidParameters),
        }
    }

    /// Get reader statistics
    pub fn get_stats(&self) -> ReaderStats {
        self.stats
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReplayResult {
    pub applied_operations: u64,
    pub failed_operations: u64,
    pub committed_transactions: u64,
    pub aborted_transactions: u64,
    pub checkpoints_found: u32,
}

impl ReplayResult {
    pub fn new() -> Self {
        Self {
            applied_operations: 0,
            failed_operations: 0,
            committed_transactions: 0,
            aborted_transactions: 0,
            checkpoints_found: 0,
        }
    }
}

/// Tests for WAL functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_file_header() {
        let header = WalFileHeader::new();
        assert_eq!(header.magic, WAL_FILE_MAGIC);
        assert_eq!(header.version, WAL_FORMAT_VERSION);
        assert_eq!(header.entry_count, 0);
        assert!(header.flags.active);
    }

    #[test]
    fn test_wal_entry_header() {
        let entry = WalEntryHeader::new(
            WalOperation::VectorInsert,
            123,
            0,
            64,
        );
        assert_eq!(entry.magic, WAL_ENTRY_MAGIC);
        assert_eq!(entry.transaction_id, 123);
        assert_eq!(entry.payload_size, 64);
        assert!(entry.flags.transactional);
    }

    #[test]
    fn test_transaction() {
        let mut tx = Transaction::new(42);
        assert_eq!(tx.id, 42);
        assert_eq!(tx.operation_count, 0);
        assert!(matches!(tx.status, TransactionStatus::Active));

        let payload = [1, 2, 3, 4];
        tx.add_operation(WalOperation::VectorInsert, &payload).unwrap();
        assert_eq!(tx.operation_count, 1);
        assert_eq!(tx.total_payload_size, 4);
    }

    #[test]
    fn test_wal_writer() {
        let mut writer = WalWriter::new();
        assert_eq!(writer.next_tx_id, 1);
        assert_eq!(writer.stats.entries_written, 0);

        let tx_id = writer.begin_transaction().unwrap();
        assert_eq!(tx_id, 1);
        assert_eq!(writer.next_tx_id, 2);
    }

    #[test]
    fn test_vector_payloads() {
        let insert_payload = VectorInsertPayload {
            vector_id: 123,
            dimensions: 128,
            data_type: VectorDataType::Float32,
            level: 2,
            data_offset: 1024,
            data_size: 512,
            reserved: [0; 7],
        };

        assert_eq!(insert_payload.vector_id, 123);
        assert_eq!(insert_payload.dimensions, 128);

        let delete_payload = VectorDeletePayload {
            vector_id: 456,
            level: 1,
            reserved: [0; 7],
        };

        assert_eq!(delete_payload.vector_id, 456);
        assert_eq!(delete_payload.level, 1);
    }
}