/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

//! Journaling and Transaction System
//!
//! This module implements crash-consistent journaling for filesystem integrity,
//! providing atomic operations and recovery mechanisms for VexFS.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::constants::VEXFS_JOURNAL_BUFFER_SIZE;
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;
use core::mem;

/// Transaction states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionState {
    /// Transaction is being built
    Building,
    /// Transaction is committed to journal
    Committed,
    /// Transaction is being written to main filesystem
    Checkpointing,
    /// Transaction is complete
    Complete,
    /// Transaction was aborted
    Aborted,
}

/// Journal operation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JournalOpType {
    /// Metadata write operation
    MetadataWrite,
    /// Data write operation
    DataWrite,
    /// Block allocation
    BlockAlloc,
    /// Block deallocation
    BlockFree,
    /// Inode creation
    InodeCreate,
    /// Inode deletion
    InodeDelete,
    /// Directory entry add
    DirEntryAdd,
    /// Directory entry remove
    DirEntryRemove,
}

impl From<JournalOpType> for u32 {
    fn from(op_type: JournalOpType) -> u32 {
        match op_type {
            JournalOpType::MetadataWrite => 1,
            JournalOpType::DataWrite => 2,
            JournalOpType::BlockAlloc => 3,
            JournalOpType::BlockFree => 4,
            JournalOpType::InodeCreate => 5,
            JournalOpType::InodeDelete => 6,
            JournalOpType::DirEntryAdd => 7,
            JournalOpType::DirEntryRemove => 8,
        }
    }
}

/// Journal record header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JournalRecordHeader {
    /// Magic number for validation
    pub magic: u32,
    /// Record type
    pub record_type: u32,
    /// Transaction ID
    pub transaction_id: u64,
    /// Sequence number within transaction
    pub sequence: u32,
    /// Record length including header
    pub length: u32,
    /// Checksum of record data
    pub checksum: u32,
    /// Flags
    pub flags: u32,
    /// Timestamp
    pub timestamp: u64,
}

impl JournalRecordHeader {
    /// Create new journal record header
    pub fn new(record_type: u32, transaction_id: u64, sequence: u32, length: u32) -> Self {
        Self {
            magic: VEXFS_JOURNAL_RECORD_MAGIC,
            record_type,
            transaction_id,
            sequence,
            length,
            checksum: 0,
            flags: 0,
            timestamp: 0,
        }
    }

    /// Update checksum for this header
    pub fn update_checksum(&mut self) {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>() - core::mem::size_of::<u32>() // Exclude checksum field
            )
        };
        self.checksum = crc32(data);
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>() - core::mem::size_of::<u32>()
            )
        };
        verify_checksum(data, self.checksum)
    }
}

/// Journal commit record
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JournalCommitRecord {
    /// Standard header
    pub header: JournalRecordHeader,
    /// Number of operations in transaction
    pub num_operations: u32,
    /// Transaction checksum
    pub transaction_checksum: u32,
    /// Reserved fields
    pub reserved: [u32; 6],
}

impl JournalCommitRecord {
    /// Create new commit record
    pub fn new(transaction_id: u64, num_operations: u32, transaction_checksum: u32) -> Self {
        let header = JournalRecordHeader::new(
            JOURNAL_RECORD_COMMIT,
            transaction_id,
            num_operations,
            mem::size_of::<Self>() as u32,
        );

        Self {
            header,
            num_operations,
            transaction_checksum,
            reserved: [0; 6],
        }
    }
}

/// Journal operation record
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JournalOperationRecord {
    /// Standard header
    pub header: JournalRecordHeader,
    /// Operation type
    pub op_type: u32,
    /// Target block number
    pub block_number: u64,
    /// Data offset within block
    pub offset: u32,
    /// Data length
    pub data_length: u32,
    /// Old data checksum (for undo)
    pub old_checksum: u32,
    /// New data checksum (for redo)
    pub new_checksum: u32,
}

impl JournalOperationRecord {
    /// Create new operation record
    pub const fn new() -> Self {
        Self {
            header: JournalRecordHeader {
                magic: 0,
                record_type: 0,
                transaction_id: 0,
                sequence: 0,
                length: 0,
                checksum: 0,
                flags: 0,
                timestamp: 0,
            },
            op_type: 0,
            block_number: 0,
            offset: 0,
            data_length: 0,
            old_checksum: 0,
            new_checksum: 0,
        }
    }

    /// Initialize operation record
    pub fn initialize(&mut self, 
                      transaction_id: u64, 
                      sequence: u32, 
                      op_type: JournalOpType,
                      block_number: BlockNumber,
                      offset: u32,
                      data_length: u32) {
        self.header = JournalRecordHeader::new(
            JOURNAL_RECORD_OPERATION,
            transaction_id,
            sequence,
            mem::size_of::<Self>() as u32 + data_length,
        );
        self.op_type = u32::from(op_type);
        self.block_number = block_number;
        self.offset = offset;
        self.data_length = data_length;
    }

    /// Set checksums for undo/redo data
    pub fn set_checksums(&mut self, old_data: &[u8], new_data: &[u8]) {
        self.old_checksum = crc32(old_data);
        self.new_checksum = crc32(new_data);
        self.header.update_checksum();
    }
}

/// Journal superblock
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JournalSuperblock {
    /// Magic number
    pub j_magic: u32,
    /// Journal version
    pub j_version: u32,
    /// Journal block size
    pub j_blocksize: u32,
    /// Total journal blocks
    pub j_total_blocks: u32,
    /// First log block
    pub j_first_block: u32,
    /// First commit ID expected
    pub j_first_commit_id: u64,
    /// Sequence number of first commit ID
    pub j_commit_sequence: u32,
    /// Head of journal (next block to write)
    pub j_head: u32,
    /// Tail of journal (oldest valid data)
    pub j_tail: u32,
    /// Journal features
    pub j_features: u32,
    /// UUID of filesystem
    pub j_uuid: [u8; 16],
    /// Last mount time
    pub j_mount_time: u64,
    /// Journal state
    pub j_state: u32,
    /// Error number
    pub j_errno: u32,
    /// Checksum
    pub j_checksum: u32,
    /// Reserved space
    pub j_reserved: [u32; 32],
}

impl JournalSuperblock {
    /// Create new journal superblock
    pub fn new(block_size: u32, journal_blocks: u32) -> Self {
        Self {
            j_magic: VEXFS_JOURNAL_MAGIC,
            j_version: VEXFS_JOURNAL_VERSION,
            j_blocksize: block_size,
            j_total_blocks: journal_blocks,
            j_first_block: 1,
            j_first_commit_id: 1,
            j_commit_sequence: 1,
            j_head: 1,
            j_tail: 1,
            j_features: 0,
            j_uuid: [0; 16],
            j_mount_time: 0,
            j_state: JOURNAL_STATE_CLEAN,
            j_errno: 0,
            j_checksum: 0,
            j_reserved: [0; 32],
        }
    }

    /// Update checksum
    pub fn update_checksum(&mut self) {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>() - core::mem::size_of::<u32>()
            )
        };
        self.j_checksum = crc32(data);
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>() - core::mem::size_of::<u32>()
            )
        };
        verify_checksum(data, self.j_checksum)
    }
}

/// Transaction handle
#[derive(Debug)]
pub struct VexfsTransaction {
    /// Transaction ID
    pub tid: u64,
    /// Current state
    pub state: TransactionState,
    /// Operations in this transaction
    pub operations: [JournalOperationRecord; VEXFS_MAX_TRANSACTION_OPS],
    /// Number of operations
    pub num_ops: u32,
    /// Buffer for operation data
    pub data_buffer: [u8; VEXFS_TRANSACTION_BUFFER_SIZE],
    /// Used space in data buffer
    pub data_used: u32,
    /// Transaction flags
    pub flags: u32,
    /// Reference count
    pub ref_count: u32,
    /// Start time
    pub start_time: u64,
}

impl VexfsTransaction {
    /// Create new transaction
    pub fn new(tid: u64, flags: u32) -> Self {
        Self {
            tid,
            state: TransactionState::Building,
            operations: [JournalOperationRecord::new(); VEXFS_MAX_TRANSACTION_OPS],
            num_ops: 0,
            data_buffer: [0; VEXFS_TRANSACTION_BUFFER_SIZE],
            data_used: 0,
            flags,
            ref_count: 1,
            start_time: 0,
        }
    }

    /// Add operation to transaction
    pub fn add_operation(&mut self, 
                        op_type: JournalOpType,
                        block_number: BlockNumber,
                        offset: u32,
                        old_data: &[u8],
                        new_data: &[u8]) -> VexfsResult<()> {
        if self.num_ops >= VEXFS_MAX_TRANSACTION_OPS as u32 {
            return Err(VexfsError::NoSpace);
        }

        if self.data_used + old_data.len() + new_data.len() > VEXFS_TRANSACTION_BUFFER_SIZE {
            return Err(VexfsError::NoSpace);
        }

        let op_idx = self.num_ops as usize;
        let op = &mut self.operations[op_idx];
        
        op.initialize(self.tid, self.num_ops, op_type, block_number, offset, new_data.len() as u32);
        
        // Store data in buffer
        let old_data_offset = self.data_used;
        self.data_buffer[old_data_offset..old_data_offset + old_data.len()].copy_from_slice(old_data);
        
        let new_data_offset = old_data_offset + old_data.len();
        self.data_buffer[new_data_offset..new_data_offset + new_data.len()].copy_from_slice(new_data);
        
        self.data_used += old_data.len() + new_data.len();
        
        // Set checksums
        op.set_checksums(old_data, new_data);
        
        self.num_ops += 1;
        Ok(())
    }

    /// Calculate transaction checksum
    pub fn calculate_checksum(&self) -> u32 {
        let mut checksum = 0u32;
        checksum ^= self.tid as u32;
        checksum ^= self.num_ops;

        for i in 0..self.num_ops as usize {
            checksum ^= self.operations[i].header.checksum;
        }

        checksum
    }

    /// Check if transaction is ready for commit
    pub fn is_ready_for_commit(&self) -> bool {
        self.state == TransactionState::Building && self.num_ops > 0
    }
}

/// Recovery information
#[derive(Debug)]
pub struct RecoveryInfo {
    /// Recovery needed flag
    pub recovery_needed: bool,
    /// Last committed transaction
    pub last_committed_tid: u64,
    /// Transactions to replay
    pub replay_list: [u64; VEXFS_MAX_ACTIVE_TRANSACTIONS],
    /// Number of transactions to replay
    pub replay_count: u32,
    /// Recovery start block
    pub recovery_start: u32,
    /// Recovery end block
    pub recovery_end: u32,
}

impl RecoveryInfo {
    /// Create new recovery info
    pub fn new() -> Self {
        Self {
            recovery_needed: false,
            last_committed_tid: 0,
            replay_list: [0; VEXFS_MAX_ACTIVE_TRANSACTIONS],
            replay_count: 0,
            recovery_start: 0,
            recovery_end: 0,
        }
    }

    /// Add transaction to replay list
    pub fn add_replay_transaction(&mut self, tid: u64) -> VexfsResult<()> {
        if self.replay_count >= VEXFS_MAX_ACTIVE_TRANSACTIONS as u32 {
            return Err(VexfsError::NoSpace);
        }

        self.replay_list[self.replay_count as usize] = tid;
        self.replay_count += 1;
        Ok(())
    }

    /// Clear replay list
    pub fn clear_replay_list(&mut self) {
        self.replay_count = 0;
        self.replay_list = [0; VEXFS_MAX_ACTIVE_TRANSACTIONS];
    }
}

/// Journal statistics
#[derive(Debug, Clone, Copy)]
pub struct JournalStats {
    /// Total journal space in blocks
    pub total_space: u32,
    /// Free journal space in blocks
    pub free_space: u32,
    /// Number of active transactions
    pub active_transactions: u32,
    /// Number of committed but not checkpointed transactions
    pub committed_transactions: u32,
    /// Current transaction ID
    pub current_tid: u64,
}

/// Journal manager
pub struct VexfsJournal {
    /// Journal superblock
    pub superblock: JournalSuperblock,
    /// Current transaction ID counter
    pub current_tid: u64,
    /// Active transactions
    pub active_transactions: [Option<VexfsTransaction>; VEXFS_MAX_ACTIVE_TRANSACTIONS],
    /// Journal head position
    pub head: u32,
    /// Journal tail position
    pub tail: u32,
    /// Available journal space
    pub free_space: u32,
    /// Journal device block size
    pub block_size: u32,
    /// Recovery state
    pub recovery_info: RecoveryInfo,
    /// Journal write buffer
    pub write_buffer: [u8; VEXFS_JOURNAL_BUFFER_SIZE],
    /// Buffer position
    pub buffer_pos: u32,
}

impl VexfsJournal {
    /// Create new journal manager
    pub fn new(block_size: u32, journal_blocks: u32) -> Self {
        let superblock = JournalSuperblock::new(block_size, journal_blocks);
        
        Self {
            superblock,
            current_tid: 1,
            active_transactions: [const { None }; VEXFS_MAX_ACTIVE_TRANSACTIONS],
            head: 1,
            tail: 1,
            free_space: journal_blocks - 1, // Minus superblock
            block_size,
            recovery_info: RecoveryInfo::new(),
            write_buffer: [0; VEXFS_JOURNAL_BUFFER_SIZE],
            buffer_pos: 0,
        }
    }

    /// Initialize the journal
    pub fn initialize(&mut self) -> VexfsResult<()> {
        // Update checksum
        self.superblock.update_checksum();
        
        // Check if recovery is needed
        if self.superblock.j_state != JOURNAL_STATE_CLEAN {
            self.recovery_info.recovery_needed = true;
            self.prepare_recovery()?;
        }
        
        Ok(())
    }

    /// Start a new transaction
    pub fn start_transaction(&mut self, flags: u32) -> VexfsResult<u64> {
        let slot = self.find_free_transaction_slot()?;
        
        let tid = self.current_tid;
        self.current_tid += 1;
        
        let transaction = VexfsTransaction::new(tid, flags);
        self.active_transactions[slot] = Some(transaction);
        
        Ok(tid)
    }

    /// Commit a transaction
    pub fn commit_transaction(&mut self, tid: u64) -> VexfsResult<()> {
        let transaction = self.find_transaction_mut(tid)?;
        
        if transaction.state != TransactionState::Building {
            return Err(VexfsError::InvalidArgument("transaction not in building state".to_string()));
        }

        if !transaction.is_ready_for_commit() {
            return Err(VexfsError::InvalidArgument("transaction not ready for commit".to_string()));
        }
        
        // Check available space
        let space_needed = self.calculate_transaction_space(transaction);
        if space_needed > self.free_space {
            return Err(VexfsError::NoSpace);
        }
        
        // Write transaction to journal
        self.write_transaction_to_journal(transaction)?;
        
        // Mark as committed
        transaction.state = TransactionState::Committed;
        
        // Update journal pointers
        self.free_space -= space_needed;
        
        Ok(())
    }

    /// Abort a transaction
    pub fn abort_transaction(&mut self, tid: u64) -> VexfsResult<()> {
        let slot = self.find_transaction_slot(tid)?;
        
        if let Some(ref mut transaction) = self.active_transactions[slot] {
            transaction.state = TransactionState::Aborted;
        }
        
        // Remove from active transactions
        self.active_transactions[slot] = None;
        Ok(())
    }

    /// Get journal statistics
    pub fn get_stats(&self) -> JournalStats {
        let mut active_transactions = 0;
        let mut committed_transactions = 0;
        
        for transaction_opt in &self.active_transactions {
            if let Some(ref transaction) = transaction_opt {
                match transaction.state {
                    TransactionState::Building => active_transactions += 1,
                    TransactionState::Committed => committed_transactions += 1,
                    _ => {}
                }
            }
        }
        
        JournalStats {
            total_space: self.superblock.j_total_blocks,
            free_space: self.free_space,
            active_transactions,
            committed_transactions,
            current_tid: self.current_tid,
        }
    }

    // Private helper methods
    fn find_free_transaction_slot(&self) -> VexfsResult<usize> {
        for i in 0..VEXFS_MAX_ACTIVE_TRANSACTIONS {
            if self.active_transactions[i].is_none() {
                return Ok(i);
            }
        }
        Err(VexfsError::NoSpace)
    }

    fn find_transaction_mut(&mut self, tid: u64) -> VexfsResult<&mut VexfsTransaction> {
        for transaction_opt in &mut self.active_transactions {
            if let Some(ref mut transaction) = transaction_opt {
                if transaction.tid == tid {
                    return Ok(transaction);
                }
            }
        }
        Err(VexfsError::InvalidArgument("transaction not found".to_string()))
    }

    fn find_transaction_slot(&self, tid: u64) -> VexfsResult<usize> {
        for i in 0..VEXFS_MAX_ACTIVE_TRANSACTIONS {
            if let Some(ref transaction) = self.active_transactions[i] {
                if transaction.tid == tid {
                    return Ok(i);
                }
            }
        }
        Err(VexfsError::InvalidArgument("transaction not found".to_string()))
    }

    fn calculate_transaction_space(&self, transaction: &VexfsTransaction) -> u32 {
        let mut space = 0u32;
        
        // Space for operation records
        for i in 0..transaction.num_ops as usize {
            space += transaction.operations[i].header.length;
        }
        
        // Space for commit record
        space += mem::size_of::<JournalCommitRecord>() as u32;
        
        // Round up to block boundary
        ((space + self.block_size - 1) / self.block_size) * self.block_size
    }

    fn write_transaction_to_journal(&mut self, transaction: &VexfsTransaction) -> VexfsResult<()> {
        // Write all operation records
        for i in 0..transaction.num_ops as usize {
            let op_record = &transaction.operations[i];
            self.write_journal_record(&op_record.header)?;
        }
        
        // Write commit record
        let commit_record = JournalCommitRecord::new(
            transaction.tid,
            transaction.num_ops,
            transaction.calculate_checksum(),
        );
        
        self.write_journal_record(&commit_record.header)?;
        Ok(())
    }

    fn write_journal_record(&mut self, _header: &JournalRecordHeader) -> VexfsResult<()> {
        // In a real implementation, this would:
        // 1. Write header to journal
        // 2. Write data if provided
        // 3. Update head pointer
        // 4. Handle wrap-around
        Ok(())
    }

    fn prepare_recovery(&mut self) -> VexfsResult<()> {
        self.recovery_info.recovery_start = self.superblock.j_tail;
        self.recovery_info.recovery_end = self.superblock.j_head;
        Ok(())
    }
}

/// Transaction manager for high-level transaction operations
pub struct TransactionManager {
    /// Journal instance
    journal: VexfsJournal,
}

impl TransactionManager {
    /// Create new transaction manager
    pub fn new(journal: VexfsJournal) -> Self {
        Self { journal }
    }

    /// Execute a transaction with automatic commit/abort
    pub fn execute_transaction<F>(&mut self, flags: u32, f: F) -> VexfsResult<()>
    where
        F: FnOnce(u64, &mut VexfsJournal) -> VexfsResult<()>,
    {
        let tid = self.journal.start_transaction(flags)?;
        
        match f(tid, &mut self.journal) {
            Ok(()) => {
                self.journal.commit_transaction(tid)?;
                Ok(())
            }
            Err(e) => {
                let _ = self.journal.abort_transaction(tid);
                Err(e)
            }
        }
    }

    /// Get journal statistics
    pub fn get_stats(&self) -> JournalStats {
        self.journal.get_stats()
    }

    /// Get mutable reference to journal
    pub fn journal_mut(&mut self) -> &mut VexfsJournal {
        &mut self.journal
    }

    /// Get reference to journal
    pub fn journal(&self) -> &VexfsJournal {
        &self.journal
    }
}

// Re-export for compatibility
pub type JournalTransaction = VexfsTransaction;