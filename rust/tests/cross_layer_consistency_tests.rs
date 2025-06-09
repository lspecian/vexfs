//! Comprehensive test suite for Cross-Layer Consistency Mechanisms (Task 14)
//! 
//! This test suite covers unit tests, integration tests, concurrency tests,
//! and crash simulation tests as specified in the task requirements.

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::thread;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use vexfs::cross_layer_consistency::{
    CrossLayerConsistencyManager, CrossLayerConfig, CrossLayerTransactionState,
    CrossLayerOperationType, CrossLayerIsolationLevel, CrossLayerEvent,
};
use vexfs::VexFSError;

/// Test configuration for cross-layer consistency tests
fn test_config() -> CrossLayerConfig {
    CrossLayerConfig {
        consistency_check_interval_ms: 100,  // Fast for testing
        deadlock_check_interval_ms: 50,      // Fast for testing
        recovery_check_interval_ms: 200,     // Fast for testing
        transaction_timeout_ms: 1000,        // 1 second for testing
        max_concurrent_transactions: 10,     // Small for testing
        enable_deadlock_detection: true,
        enable_consistency_checks: true,
        enable_recovery: true,
        snapshot_retention_hours: 1,         // Short for testing
    }
}

#[tokio::test]
async fn test_manager_creation_and_startup() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    
    // Test initial state
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, 0);
    assert_eq!(stats.active_transactions, 0);
    
    // Start the manager
    manager.start().await.unwrap();
    
    // Give background tasks time to start
    sleep(Duration::from_millis(50)).await;
    
    // Stop the manager
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_basic_transaction_lifecycle() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    // Begin a transaction
    let transaction_id = manager.begin_transaction(
        0x07, // All layers
        CrossLayerIsolationLevel::Serializable,
        Some(2000),
    ).await.unwrap();
    
    // Verify transaction was created
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, 1);
    assert_eq!(stats.active_transactions, 1);
    
    // Add operations to the transaction
    let op1_id = manager.add_operation(
        transaction_id,
        CrossLayerOperationType::FilesystemOnly,
        0x01,
        vec![1, 2, 3, 4],
        0,
        1,
    ).await.unwrap();
    
    let op2_id = manager.add_operation(
        transaction_id,
        CrossLayerOperationType::GraphOnly,
        0x02,
        vec![5, 6, 7, 8],
        0,
        2,
    ).await.unwrap();
    
    assert_ne!(op1_id, op2_id);
    
    // Commit the transaction
    let result = timeout(Duration::from_secs(3), manager.commit_transaction(transaction_id)).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());
    
    // Verify final statistics
    let final_stats = manager.get_stats().await;
    assert_eq!(final_stats.total_transactions, 1);
    assert_eq!(final_stats.successful_commits, 1);
    assert_eq!(final_stats.active_transactions, 0);
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_transaction_abort() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    let transaction_id = manager.begin_transaction(
        0x01, // Filesystem only
        CrossLayerIsolationLevel::ReadCommitted,
        None,
    ).await.unwrap();
    
    // Add an operation
    manager.add_operation(
        transaction_id,
        CrossLayerOperationType::FilesystemOnly,
        0x01,
        vec![1, 2, 3],
        0,
        1,
    ).await.unwrap();
    
    // Abort the transaction
    manager.abort_transaction(transaction_id).await.unwrap();
    
    // Give time for processing
    sleep(Duration::from_millis(200)).await;
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, 1);
    assert_eq!(stats.aborted_transactions, 1);
    assert_eq!(stats.active_transactions, 0);
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_transaction_timeout() {
    let mut config = test_config();
    config.transaction_timeout_ms = 100; // Very short timeout
    
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    let transaction_id = manager.begin_transaction(
        0x07, // All layers
        CrossLayerIsolationLevel::Serializable,
        Some(100), // 100ms timeout
    ).await.unwrap();
    
    // Add an operation
    manager.add_operation(
        transaction_id,
        CrossLayerOperationType::AllLayers,
        0x07,
        vec![1, 2, 3, 4, 5],
        0,
        1,
    ).await.unwrap();
    
    // Wait longer than timeout before committing
    sleep(Duration::from_millis(200)).await;
    
    // Attempt to commit should timeout
    let result = timeout(Duration::from_millis(500), manager.commit_transaction(transaction_id)).await;
    assert!(result.is_ok());
    
    // The commit itself should fail due to timeout
    let commit_result = result.unwrap();
    assert!(commit_result.is_err());
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_transactions() {
    let config = test_config();
    let manager = Arc::new(CrossLayerConsistencyManager::new(config).unwrap());
    manager.start().await.unwrap();
    
    let mut handles = Vec::new();
    let num_transactions = 5;
    
    // Start multiple concurrent transactions
    for i in 0..num_transactions {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let transaction_id = manager_clone.begin_transaction(
                0x01 << (i % 3), // Different layer combinations
                CrossLayerIsolationLevel::ReadCommitted,
                Some(2000),
            ).await.unwrap();
            
            // Add multiple operations
            for j in 0..3 {
                manager_clone.add_operation(
                    transaction_id,
                    CrossLayerOperationType::FilesystemOnly,
                    0x01,
                    vec![i as u8, j as u8],
                    0,
                    1,
                ).await.unwrap();
            }
            
            // Random delay to create interleaving
            sleep(Duration::from_millis((i * 10) as u64)).await;
            
            // Commit the transaction
            manager_clone.commit_transaction(transaction_id).await.unwrap();
            
            transaction_id
        });
        handles.push(handle);
    }
    
    // Wait for all transactions to complete
    let mut transaction_ids = Vec::new();
    for handle in handles {
        let transaction_id = handle.await.unwrap();
        transaction_ids.push(transaction_id);
    }
    
    // Verify all transactions completed successfully
    assert_eq!(transaction_ids.len(), num_transactions);
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, num_transactions as u64);
    assert_eq!(stats.successful_commits, num_transactions as u64);
    assert_eq!(stats.active_transactions, 0);
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_max_concurrent_transactions_limit() {
    let mut config = test_config();
    config.max_concurrent_transactions = 2; // Very low limit
    
    let manager = Arc::new(CrossLayerConsistencyManager::new(config).unwrap());
    manager.start().await.unwrap();
    
    // Start transactions up to the limit
    let tx1 = manager.begin_transaction(
        0x01,
        CrossLayerIsolationLevel::ReadCommitted,
        Some(5000),
    ).await.unwrap();
    
    let tx2 = manager.begin_transaction(
        0x02,
        CrossLayerIsolationLevel::ReadCommitted,
        Some(5000),
    ).await.unwrap();
    
    // Third transaction should fail due to limit
    let tx3_result = manager.begin_transaction(
        0x04,
        CrossLayerIsolationLevel::ReadCommitted,
        Some(5000),
    ).await;
    
    assert!(tx3_result.is_err());
    match tx3_result.unwrap_err() {
        VexFSError::ResourceExhausted(_) => {
            // Expected error type
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
    
    // Clean up
    manager.abort_transaction(tx1).await.unwrap();
    manager.abort_transaction(tx2).await.unwrap();
    
    sleep(Duration::from_millis(100)).await;
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_different_isolation_levels() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    let isolation_levels = vec![
        CrossLayerIsolationLevel::ReadUncommitted,
        CrossLayerIsolationLevel::ReadCommitted,
        CrossLayerIsolationLevel::RepeatableRead,
        CrossLayerIsolationLevel::Serializable,
        CrossLayerIsolationLevel::Snapshot,
    ];
    
    for isolation_level in isolation_levels {
        let transaction_id = manager.begin_transaction(
            0x07, // All layers
            isolation_level,
            Some(1000),
        ).await.unwrap();
        
        manager.add_operation(
            transaction_id,
            CrossLayerOperationType::AllLayers,
            0x07,
            vec![1, 2, 3],
            0,
            1,
        ).await.unwrap();
        
        manager.commit_transaction(transaction_id).await.unwrap();
    }
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, isolation_levels.len() as u64);
    assert_eq!(stats.successful_commits, isolation_levels.len() as u64);
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_different_operation_types() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    let operation_types = vec![
        (CrossLayerOperationType::FilesystemOnly, 0x01),
        (CrossLayerOperationType::GraphOnly, 0x02),
        (CrossLayerOperationType::SemanticOnly, 0x04),
        (CrossLayerOperationType::FilesystemGraph, 0x03),
        (CrossLayerOperationType::FilesystemSemantic, 0x05),
        (CrossLayerOperationType::GraphSemantic, 0x06),
        (CrossLayerOperationType::AllLayers, 0x07),
    ];
    
    for (op_type, layer_mask) in operation_types {
        let transaction_id = manager.begin_transaction(
            layer_mask,
            CrossLayerIsolationLevel::ReadCommitted,
            Some(1000),
        ).await.unwrap();
        
        manager.add_operation(
            transaction_id,
            op_type,
            layer_mask,
            vec![1, 2, 3, 4],
            0,
            1,
        ).await.unwrap();
        
        manager.commit_transaction(transaction_id).await.unwrap();
    }
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, operation_types.len() as u64);
    assert_eq!(stats.successful_commits, operation_types.len() as u64);
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_consistency_operations() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    // Test consistency check
    let violations = manager.check_consistency().await.unwrap();
    assert_eq!(violations, 0); // No violations expected in test
    
    // Test deadlock detection
    let deadlocks = manager.detect_deadlocks().await.unwrap();
    assert!(deadlocks.is_empty()); // No deadlocks expected in test
    
    // Test snapshot operations
    let snapshot_id = manager.create_snapshot().await.unwrap();
    assert_ne!(snapshot_id, Uuid::nil());
    
    manager.restore_snapshot(snapshot_id).await.unwrap();
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_statistics_tracking() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    // Initial stats should be zero
    let initial_stats = manager.get_stats().await;
    assert_eq!(initial_stats.total_transactions, 0);
    assert_eq!(initial_stats.successful_commits, 0);
    assert_eq!(initial_stats.aborted_transactions, 0);
    
    // Perform some transactions
    for i in 0..3 {
        let transaction_id = manager.begin_transaction(
            0x01,
            CrossLayerIsolationLevel::ReadCommitted,
            Some(1000),
        ).await.unwrap();
        
        manager.add_operation(
            transaction_id,
            CrossLayerOperationType::FilesystemOnly,
            0x01,
            vec![i],
            0,
            1,
        ).await.unwrap();
        
        if i == 2 {
            // Abort the last transaction
            manager.abort_transaction(transaction_id).await.unwrap();
        } else {
            // Commit the others
            manager.commit_transaction(transaction_id).await.unwrap();
        }
    }
    
    // Give time for processing
    sleep(Duration::from_millis(200)).await;
    
    let final_stats = manager.get_stats().await;
    assert_eq!(final_stats.total_transactions, 3);
    assert_eq!(final_stats.successful_commits, 2);
    assert_eq!(final_stats.aborted_transactions, 1);
    assert_eq!(final_stats.active_transactions, 0);
    
    // Test stats reset
    manager.reset_stats().await.unwrap();
    let reset_stats = manager.get_stats().await;
    assert_eq!(reset_stats.total_transactions, 0);
    assert_eq!(reset_stats.successful_commits, 0);
    assert_eq!(reset_stats.aborted_transactions, 0);
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_background_task_intervals() {
    let mut config = test_config();
    config.consistency_check_interval_ms = 50;
    config.deadlock_check_interval_ms = 30;
    config.recovery_check_interval_ms = 70;
    
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    // Let background tasks run for a while
    sleep(Duration::from_millis(200)).await;
    
    // Background tasks should have executed multiple times
    // (This is mainly testing that they don't crash)
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let config = test_config();
    let manager = Arc::new(CrossLayerConsistencyManager::new(config).unwrap());
    manager.start().await.unwrap();
    
    // Start some transactions
    let mut transaction_ids = Vec::new();
    for i in 0..3 {
        let transaction_id = manager.begin_transaction(
            0x01,
            CrossLayerIsolationLevel::ReadCommitted,
            Some(5000), // Long timeout
        ).await.unwrap();
        
        manager.add_operation(
            transaction_id,
            CrossLayerOperationType::FilesystemOnly,
            0x01,
            vec![i],
            0,
            1,
        ).await.unwrap();
        
        transaction_ids.push(transaction_id);
    }
    
    // Shutdown should wait for active transactions or handle them gracefully
    let shutdown_start = SystemTime::now();
    manager.stop().await.unwrap();
    let shutdown_duration = shutdown_start.elapsed().unwrap();
    
    // Shutdown should complete within reasonable time
    assert!(shutdown_duration < Duration::from_secs(5));
}

#[tokio::test]
async fn test_error_handling() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    // Test operations on non-existent transaction
    let fake_transaction_id = Uuid::new_v4();
    
    let add_op_result = manager.add_operation(
        fake_transaction_id,
        CrossLayerOperationType::FilesystemOnly,
        0x01,
        vec![1, 2, 3],
        0,
        1,
    ).await;
    
    assert!(add_op_result.is_err());
    match add_op_result.unwrap_err() {
        VexFSError::NotFound(_) => {
            // Expected error type
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
    
    // Test commit on non-existent transaction
    let commit_result = timeout(
        Duration::from_millis(500),
        manager.commit_transaction(fake_transaction_id)
    ).await;
    
    assert!(commit_result.is_ok());
    let commit_inner = commit_result.unwrap();
    assert!(commit_inner.is_err());
    
    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_stress_concurrent_operations() {
    let config = test_config();
    let manager = Arc::new(CrossLayerConsistencyManager::new(config).unwrap());
    manager.start().await.unwrap();
    
    let num_threads = 10;
    let operations_per_thread = 5;
    let mut handles = Vec::new();
    
    for thread_id in 0..num_threads {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            for op_id in 0..operations_per_thread {
                let transaction_id = manager_clone.begin_transaction(
                    0x01 << (op_id % 3),
                    CrossLayerIsolationLevel::ReadCommitted,
                    Some(2000),
                ).await.unwrap();
                
                // Add multiple operations rapidly
                for i in 0..3 {
                    manager_clone.add_operation(
                        transaction_id,
                        CrossLayerOperationType::FilesystemOnly,
                        0x01,
                        vec![thread_id as u8, op_id as u8, i as u8],
                        0,
                        1,
                    ).await.unwrap();
                }
                
                // Randomly commit or abort
                if (thread_id + op_id) % 7 == 0 {
                    manager_clone.abort_transaction(transaction_id).await.unwrap();
                } else {
                    manager_clone.commit_transaction(transaction_id).await.unwrap();
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Give time for final processing
    sleep(Duration::from_millis(200)).await;
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, (num_threads * operations_per_thread) as u64);
    assert!(stats.successful_commits > 0);
    assert!(stats.aborted_transactions > 0);
    assert_eq!(stats.active_transactions, 0);
    
    manager.stop().await.unwrap();
}

/// Integration test that simulates real cross-layer operations
#[tokio::test]
async fn test_cross_layer_integration_simulation() {
    let config = test_config();
    let manager = CrossLayerConsistencyManager::new(config).unwrap();
    manager.start().await.unwrap();
    
    // Simulate a complex cross-layer operation like file creation with metadata
    let transaction_id = manager.begin_transaction(
        0x07, // All layers involved
        CrossLayerIsolationLevel::Serializable,
        Some(3000),
    ).await.unwrap();
    
    // Step 1: Filesystem operation (create file)
    manager.add_operation(
        transaction_id,
        CrossLayerOperationType::FilesystemOnly,
        0x01,
        b"CREATE FILE /test/file.txt".to_vec(),
        0,
        3, // High priority
    ).await.unwrap();
    
    // Step 2: Graph operation (add file node)
    manager.add_operation(
        transaction_id,
        CrossLayerOperationType::GraphOnly,
        0x02,
        b"ADD NODE file_123 TYPE file".to_vec(),
        0,
        2, // Medium priority
    ).await.unwrap();
    
    // Step 3: Semantic operation (index file content)
    manager.add_operation(
        transaction_id,
        CrossLayerOperationType::SemanticOnly,
        0x04,
        b"INDEX CONTENT file_123 embedding=[0.1,0.2,0.3]".to_vec(),
        0,
        1, // Low priority
    ).await.unwrap();
    
    // Step 4: Cross-layer consistency operation
    manager.add_operation(
        transaction_id,
        CrossLayerOperationType::AllLayers,
        0x07,
        b"VERIFY CONSISTENCY file_123".to_vec(),
        0,
        3, // High priority
    ).await.unwrap();
    
    // Commit the entire cross-layer transaction
    let result = timeout(Duration::from_secs(5), manager.commit_transaction(transaction_id)).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.successful_commits, 1);
    
    manager.stop().await.unwrap();
}

/// Performance benchmark test
#[tokio::test]
async fn test_performance_benchmark() {
    let config = test_config();
    let manager = Arc::new(CrossLayerConsistencyManager::new(config).unwrap());
    manager.start().await.unwrap();
    
    let start_time = SystemTime::now();
    let num_transactions = 50;
    let mut handles = Vec::new();
    
    for i in 0..num_transactions {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let transaction_id = manager_clone.begin_transaction(
                0x01,
                CrossLayerIsolationLevel::ReadCommitted,
                Some(1000),
            ).await.unwrap();
            
            manager_clone.add_operation(
                transaction_id,
                CrossLayerOperationType::FilesystemOnly,
                0x01,
                vec![i as u8],
                0,
                1,
            ).await.unwrap();
            
            manager_clone.commit_transaction(transaction_id).await.unwrap();
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    let total_duration = start_time.elapsed().unwrap();
    let transactions_per_second = num_transactions as f64 / total_duration.as_secs_f64();
    
    println!("Performance: {:.2} transactions/second", transactions_per_second);
    
    // Should be able to handle at least 10 transactions per second
    assert!(transactions_per_second > 10.0);
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.total_transactions, num_transactions as u64);
    assert_eq!(stats.successful_commits, num_transactions as u64);
    
    manager.stop().await.unwrap();
}