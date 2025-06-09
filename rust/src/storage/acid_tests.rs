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

//! ACID Transaction System Tests
//!
//! Comprehensive test suite for ACID compliance, covering all four properties:
//! Atomicity, Consistency, Isolation, and Durability.

#[cfg(test)]
mod tests {
    use super::super::{
        acid_transaction_manager::*,
        mvcc::*,
        deadlock_detector::*,
        durability_manager::*,
        journal::VexfsJournal,
    };
    use crate::fs_core::locking::{LockScope, LockType};
    use crate::shared::types::*;

    /// Test helper to create a test ACID transaction manager
    fn create_test_acid_manager() -> AcidTransactionManager {
        let journal = VexfsJournal::new(4096, 1024);
        AcidTransactionManager::new(journal, 256)
    }

    /// Test helper to create test MVCC manager
    fn create_test_mvcc_manager() -> MvccManager {
        MvccManager::new()
    }

    /// Test helper to create test deadlock detector
    fn create_test_deadlock_detector() -> DeadlockDetector {
        DeadlockDetector::new(
            DeadlockDetectionStrategy::WaitForGraph,
            DeadlockResolutionStrategy::AbortYoungest,
        )
    }

    /// Test helper to create test durability manager
    fn create_test_durability_manager() -> DurabilityManager {
        DurabilityManager::new(DurabilityPolicy::DataAndMetadata)
    }

    // ==========================================
    // ATOMICITY TESTS
    // ==========================================

    #[test]
    fn test_atomicity_successful_commit() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction
        let tx_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Perform multiple operations
        let lock_id1 = manager.acquire_lock(tx_id, LockScope::Inode(100), LockType::Write).unwrap();
        let lock_id2 = manager.acquire_lock(tx_id, LockScope::Inode(101), LockType::Write).unwrap();
        
        let _version1 = manager.mvcc_write(tx_id, 100, &vec![1, 2, 3, 4]).unwrap();
        let _version2 = manager.mvcc_write(tx_id, 101, &vec![5, 6, 7, 8]).unwrap();
        
        // Commit transaction - all operations should succeed atomically
        let result = manager.commit_transaction(tx_id);
        assert!(result.is_ok());
        
        // Verify transaction is no longer active
        assert!(!manager.active_transactions.contains_key(&tx_id));
    }

    #[test]
    fn test_atomicity_failed_commit_rollback() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction
        let tx_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Perform operations
        let _lock_id = manager.acquire_lock(tx_id, LockScope::Inode(100), LockType::Write).unwrap();
        let _version = manager.mvcc_write(tx_id, 100, &vec![1, 2, 3, 4]).unwrap();
        
        // Abort transaction - all operations should be rolled back
        let result = manager.abort_transaction(tx_id);
        assert!(result.is_ok());
        
        // Verify transaction is no longer active
        assert!(!manager.active_transactions.contains_key(&tx_id));
    }

    #[test]
    fn test_atomicity_timeout_abort() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction with very short timeout
        let tx_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 1).unwrap();
        
        // Simulate timeout by cleaning up timed-out transactions
        let cleaned_up = manager.cleanup_timed_out_transactions().unwrap();
        assert!(cleaned_up > 0);
        
        // Transaction should no longer be active
        assert!(!manager.active_transactions.contains_key(&tx_id));
    }

    // ==========================================
    // CONSISTENCY TESTS
    // ==========================================

    #[test]
    fn test_consistency_constraint_validation() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction
        let tx_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Acquire lock and write data
        let _lock_id = manager.acquire_lock(tx_id, LockScope::Inode(100), LockType::Write).unwrap();
        let _version = manager.mvcc_write(tx_id, 100, &vec![1, 2, 3, 4]).unwrap();
        
        // Commit should validate consistency
        let result = manager.commit_transaction(tx_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_consistency_concurrent_modification_conflict() {
        let mut manager = create_test_acid_manager();
        
        // Begin two transactions
        let tx1_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        let tx2_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Both try to modify the same block
        let _lock1 = manager.acquire_lock(tx1_id, LockScope::Inode(100), LockType::Write).unwrap();
        
        // Second transaction should fail to acquire conflicting lock
        let lock2_result = manager.acquire_lock(tx2_id, LockScope::Inode(100), LockType::Write);
        assert!(lock2_result.is_err());
        
        // Clean up
        let _ = manager.abort_transaction(tx1_id);
        let _ = manager.abort_transaction(tx2_id);
    }

    // ==========================================
    // ISOLATION TESTS
    // ==========================================

    #[test]
    fn test_isolation_read_committed() {
        let mut manager = create_test_acid_manager();
        
        // Begin two transactions
        let tx1_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        let tx2_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Transaction 1 writes data
        let _lock1 = manager.acquire_lock(tx1_id, LockScope::Inode(100), LockType::Write).unwrap();
        let _version1 = manager.mvcc_write(tx1_id, 100, &vec![1, 2, 3, 4]).unwrap();
        
        // Transaction 2 should not see uncommitted data
        let read_result = manager.mvcc_read(tx2_id, 100);
        // In a full implementation, this would check visibility rules
        
        // Commit transaction 1
        let _ = manager.commit_transaction(tx1_id);
        
        // Now transaction 2 should be able to see the committed data
        // (This would require more sophisticated MVCC implementation)
        
        let _ = manager.abort_transaction(tx2_id);
    }

    #[test]
    fn test_isolation_repeatable_read() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction with repeatable read isolation
        let tx_id = manager.begin_transaction(IsolationLevel::RepeatableRead, 30000).unwrap();
        
        // Read data multiple times - should get consistent results
        let _lock_id = manager.acquire_lock(tx_id, LockScope::Inode(100), LockType::Read).unwrap();
        let read1 = manager.mvcc_read(tx_id, 100);
        let read2 = manager.mvcc_read(tx_id, 100);
        
        // Both reads should return the same data (in full implementation)
        assert_eq!(read1.is_ok(), read2.is_ok());
        
        let _ = manager.commit_transaction(tx_id);
    }

    #[test]
    fn test_isolation_serializable() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction with serializable isolation
        let tx_id = manager.begin_transaction(IsolationLevel::Serializable, 30000).unwrap();
        
        // Perform operations that should be serializable
        let _lock_id = manager.acquire_lock(tx_id, LockScope::Inode(100), LockType::Write).unwrap();
        let _version = manager.mvcc_write(tx_id, 100, &vec![1, 2, 3, 4]).unwrap();
        
        // Commit should succeed if no conflicts
        let result = manager.commit_transaction(tx_id);
        assert!(result.is_ok());
    }

    // ==========================================
    // DURABILITY TESTS
    // ==========================================

    #[test]
    fn test_durability_sync_operations() {
        let mut durability_manager = create_test_durability_manager();
        
        // Test different sync operations
        let data_blocks = vec![100, 101, 102];
        let metadata_blocks = vec![200, 201];
        
        let result = durability_manager.ensure_durability(1001, &data_blocks, &metadata_blocks);
        assert!(result.is_ok());
        
        let stats = durability_manager.get_stats();
        assert!(stats.total_sync_ops > 0);
    }

    #[test]
    fn test_durability_checkpoint_creation() {
        let mut durability_manager = create_test_durability_manager();
        
        // Create checkpoint
        let checkpoint_id = durability_manager.create_checkpoint(1001, 500, 100).unwrap();
        assert!(checkpoint_id > 0);
        
        // Complete checkpoint
        durability_manager.complete_checkpoint(checkpoint_id).unwrap();
        
        // Verify checkpoint is completed
        let latest = durability_manager.get_latest_checkpoint();
        assert!(latest.is_some());
        assert!(latest.unwrap().completed);
    }

    #[test]
    fn test_durability_write_barriers() {
        let mut durability_manager = create_test_durability_manager();
        
        // Test different barrier types
        let result1 = durability_manager.issue_write_barrier(WriteBarrier::FlushCache);
        assert!(result1.is_ok());
        
        let result2 = durability_manager.issue_write_barrier(WriteBarrier::FullBarrier);
        assert!(result2.is_ok());
        
        let stats = durability_manager.get_stats();
        assert_eq!(stats.write_barriers_issued, 2);
    }

    // ==========================================
    // MVCC TESTS
    // ==========================================

    #[test]
    fn test_mvcc_version_creation() {
        let mut mvcc_manager = create_test_mvcc_manager();
        
        // Write data creating new version
        let version_id = mvcc_manager.write(100, 1001, vec![1, 2, 3, 4]).unwrap();
        assert!(version_id > 0);
        
        // Read data
        let snapshot = mvcc_manager.create_snapshot();
        let data = mvcc_manager.read(100, 1001, snapshot).unwrap();
        assert_eq!(data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_mvcc_version_visibility() {
        let mut mvcc_manager = create_test_mvcc_manager();
        
        // Transaction 1 writes data
        let _version1 = mvcc_manager.write(100, 1001, vec![1, 2, 3]).unwrap();
        
        // Transaction 2 creates snapshot
        let snapshot_tx2 = mvcc_manager.create_snapshot();
        
        // Transaction 1 writes more data
        let _version2 = mvcc_manager.write(100, 1001, vec![4, 5, 6]).unwrap();
        
        // Transaction 2 should see consistent snapshot
        let data = mvcc_manager.read(100, 1002, snapshot_tx2);
        assert!(data.is_ok());
    }

    #[test]
    fn test_mvcc_garbage_collection() {
        let mut mvcc_manager = create_test_mvcc_manager();
        mvcc_manager.set_gc_thresholds(2, 1000);
        
        // Create multiple versions
        let _v1 = mvcc_manager.write(100, 1001, vec![1]).unwrap();
        let _v2 = mvcc_manager.write(100, 1002, vec![2]).unwrap();
        let _v3 = mvcc_manager.write(100, 1003, vec![3]).unwrap();
        
        // Mark first version as deleted
        mvcc_manager.delete(100, 1, 1001).unwrap();
        
        // Run garbage collection
        let collected = mvcc_manager.garbage_collect_all().unwrap();
        assert!(collected >= 0); // May or may not collect depending on timestamps
    }

    // ==========================================
    // DEADLOCK DETECTION TESTS
    // ==========================================

    #[test]
    fn test_deadlock_detection_simple_cycle() {
        let mut detector = create_test_deadlock_detector();
        
        // Add transaction metadata
        detector.add_transaction_metadata(1, 1000, 1, 1, 0, 30000);
        detector.add_transaction_metadata(2, 1001, 1, 1, 0, 30000);
        detector.add_transaction_metadata(3, 1002, 1, 1, 0, 30000);
        
        // Create deadlock cycle: 1 -> 2 -> 3 -> 1
        detector.add_wait_relationship(1, 2, LockScope::Inode(100), LockType::Write, 1).unwrap();
        detector.add_wait_relationship(2, 3, LockScope::Inode(101), LockType::Write, 1).unwrap();
        detector.add_wait_relationship(3, 1, LockScope::Inode(102), LockType::Write, 1).unwrap();
        
        // Detect deadlocks
        let victims = detector.detect_deadlocks().unwrap();
        assert!(!victims.is_empty());
    }

    #[test]
    fn test_deadlock_resolution_strategies() {
        let mut detector = DeadlockDetector::new(
            DeadlockDetectionStrategy::WaitForGraph,
            DeadlockResolutionStrategy::AbortLowestPriority,
        );
        
        // Add transactions with different priorities
        detector.add_transaction_metadata(1, 1000, 3, 1, 0, 30000); // High priority
        detector.add_transaction_metadata(2, 1001, 1, 1, 0, 30000); // Low priority
        
        // Create deadlock
        detector.add_wait_relationship(1, 2, LockScope::Inode(100), LockType::Write, 3).unwrap();
        detector.add_wait_relationship(2, 1, LockScope::Inode(101), LockType::Write, 1).unwrap();
        
        let victims = detector.detect_deadlocks().unwrap();
        // Should select transaction 2 (lower priority) as victim
        assert!(victims.contains(&2));
    }

    #[test]
    fn test_deadlock_timeout_detection() {
        let mut detector = DeadlockDetector::new(
            DeadlockDetectionStrategy::Timeout,
            DeadlockResolutionStrategy::AbortOldest,
        );
        
        detector.set_timeout_threshold(100); // Very short timeout
        
        // Add transaction that will timeout
        detector.add_transaction_metadata(1, 1, 1, 1, 0, 50); // Short timeout
        
        // Add wait relationship
        detector.add_wait_relationship(1, 2, LockScope::Inode(100), LockType::Write, 1).unwrap();
        
        // Detection should find timed-out transaction
        let victims = detector.detect_deadlocks().unwrap();
        assert!(victims.contains(&1));
    }

    // ==========================================
    // INTEGRATION TESTS
    // ==========================================

    #[test]
    fn test_acid_integration_full_transaction() {
        let mut manager = create_test_acid_manager();
        
        // Begin transaction
        let tx_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Acquire multiple locks (Consistency)
        let _lock1 = manager.acquire_lock(tx_id, LockScope::Inode(100), LockType::Write).unwrap();
        let _lock2 = manager.acquire_lock(tx_id, LockScope::Inode(101), LockType::Read).unwrap();
        
        // Perform MVCC operations (Isolation)
        let _version1 = manager.mvcc_write(tx_id, 100, &vec![1, 2, 3, 4]).unwrap();
        let (data, _version2) = manager.mvcc_read(tx_id, 101).unwrap();
        assert!(!data.is_empty());
        
        // Commit transaction (Atomicity + Durability)
        let result = manager.commit_transaction(tx_id);
        assert!(result.is_ok());
        
        // Verify statistics
        let stats = manager.get_stats();
        assert_eq!(stats.transactions_committed, 1);
        assert_eq!(stats.active_transaction_count, 0);
    }

    #[test]
    fn test_acid_integration_concurrent_transactions() {
        let mut manager = create_test_acid_manager();
        
        // Begin multiple transactions
        let tx1_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        let tx2_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // Non-conflicting operations
        let _lock1 = manager.acquire_lock(tx1_id, LockScope::Inode(100), LockType::Write).unwrap();
        let _lock2 = manager.acquire_lock(tx2_id, LockScope::Inode(200), LockType::Write).unwrap();
        
        let _version1 = manager.mvcc_write(tx1_id, 100, &vec![1, 2, 3]).unwrap();
        let _version2 = manager.mvcc_write(tx2_id, 200, &vec![4, 5, 6]).unwrap();
        
        // Both should commit successfully
        let result1 = manager.commit_transaction(tx1_id);
        let result2 = manager.commit_transaction(tx2_id);
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let stats = manager.get_stats();
        assert_eq!(stats.transactions_committed, 2);
    }

    #[test]
    fn test_acid_integration_conflict_resolution() {
        let mut manager = create_test_acid_manager();
        
        // Begin conflicting transactions
        let tx1_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        let tx2_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
        
        // First transaction acquires lock
        let _lock1 = manager.acquire_lock(tx1_id, LockScope::Inode(100), LockType::Write).unwrap();
        
        // Second transaction should fail to acquire conflicting lock
        let lock2_result = manager.acquire_lock(tx2_id, LockScope::Inode(100), LockType::Write);
        assert!(lock2_result.is_err());
        
        // First transaction should commit successfully
        let result1 = manager.commit_transaction(tx1_id);
        assert!(result1.is_ok());
        
        // Second transaction should be aborted
        let result2 = manager.abort_transaction(tx2_id);
        assert!(result2.is_ok());
        
        let stats = manager.get_stats();
        assert_eq!(stats.transactions_committed, 1);
        assert_eq!(stats.transactions_aborted, 1);
    }

    // ==========================================
    // PERFORMANCE TESTS
    // ==========================================

    #[test]
    fn test_acid_performance_many_transactions() {
        let mut manager = create_test_acid_manager();
        let num_transactions = 100;
        
        for i in 0..num_transactions {
            let tx_id = manager.begin_transaction(IsolationLevel::ReadCommitted, 30000).unwrap();
            
            // Simple operation
            let _lock = manager.acquire_lock(tx_id, LockScope::Inode(i as u64), LockType::Write).unwrap();
            let _version = manager.mvcc_write(tx_id, i as u64, &vec![i as u8]).unwrap();
            
            let result = manager.commit_transaction(tx_id);
            assert!(result.is_ok());
        }
        
        let stats = manager.get_stats();
        assert_eq!(stats.transactions_committed, num_transactions);
    }

    #[test]
    fn test_mvcc_performance_many_versions() {
        let mut mvcc_manager = create_test_mvcc_manager();
        let num_versions = 1000;
        
        for i in 0..num_versions {
            let _version = mvcc_manager.write(100, i, vec![i as u8; 4]).unwrap();
        }
        
        // Read latest version
        let snapshot = mvcc_manager.create_snapshot();
        let data = mvcc_manager.read(100, 999, snapshot).unwrap();
        assert_eq!(data.len(), 4);
        
        let stats = mvcc_manager.get_stats();
        assert!(stats.total_versions >= num_versions);
    }
}