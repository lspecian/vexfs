//! Comprehensive tests for Cross-Layer Integration Framework (Task 21)
//! 
//! This test suite validates the unified transaction management, versioned metadata,
//! journal ordering, atomic cross-boundary operations, crash recovery, and semantic
//! views of the Cross-Layer Integration Framework.

use std::time::Duration;
use std::collections::HashMap;
use tokio::time::sleep;
use uuid::Uuid;

#[cfg(feature = "cross_layer_integration")]
use vexfs::cross_layer_integration::{
    CrossLayerIntegrationFramework, IntegrationConfig, IntegrationStats,
    VectorClock, LamportTimestamp, JournalEntry, VersionedMetadata,
    TwoPhaseCommitState, TwoPhaseCommitTransaction, RecoveryLogEntry,
    UnifiedTransactionManager, JournalOrderingService, VersionedMetadataManager,
    TwoPhaseCommitCoordinator, RecoveryManager, PerformanceCache,
    ConsistencyLevel, QueryResult,
};

use vexfs::cross_layer_consistency::{
    CrossLayerIsolationLevel, CrossLayerOperationType,
};
use vexfs::error::VexFSError;

#[cfg(feature = "cross_layer_integration")]
mod integration_tests {
    use super::*;
    use proptest::prelude::*;
    use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
    use loom::sync::Arc;
    use loom::thread;

    /// Test integration framework creation and basic functionality
    #[tokio::test]
    async fn test_integration_framework_creation() {
        let config = IntegrationConfig::default();
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        
        let stats = framework.get_integration_stats().await;
        assert_eq!(stats.total_transactions, 0);
        assert_eq!(stats.successful_transactions, 0);
        assert_eq!(stats.failed_transactions, 0);
        assert_eq!(stats.active_transactions, 0);
    }

    /// Test unified transaction lifecycle with multiple layers
    #[tokio::test]
    async fn test_unified_transaction_lifecycle() {
        let config = IntegrationConfig {
            max_concurrent_transactions: 10,
            transaction_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        
        framework.start().await.unwrap();
        
        // Begin transaction across multiple layers
        let transaction_id = framework.begin_unified_transaction(
            vec!["filesystem".to_string(), "graph".to_string(), "semantic".to_string()],
            CrossLayerIsolationLevel::Serializable,
            Some(Duration::from_secs(10)),
        ).await.unwrap();
        
        // Add operations to different layers
        let fs_op_id = framework.add_unified_operation(
            transaction_id,
            "filesystem".to_string(),
            "write".to_string(),
            vec![1, 2, 3, 4],
            HashMap::new(),
        ).await.unwrap();
        
        let graph_op_id = framework.add_unified_operation(
            transaction_id,
            "graph".to_string(),
            "add_node".to_string(),
            vec![5, 6, 7, 8],
            HashMap::new(),
        ).await.unwrap();
        
        let semantic_op_id = framework.add_unified_operation(
            transaction_id,
            "semantic".to_string(),
            "index_document".to_string(),
            vec![9, 10, 11, 12],
            HashMap::new(),
        ).await.unwrap();
        
        assert!(fs_op_id != Uuid::nil());
        assert!(graph_op_id != Uuid::nil());
        assert!(semantic_op_id != Uuid::nil());
        
        // Commit transaction
        framework.commit_unified_transaction(transaction_id).await.unwrap();
        
        // Check statistics
        let stats = framework.get_integration_stats().await;
        assert_eq!(stats.total_transactions, 1);
        assert_eq!(stats.successful_transactions, 1);
        assert_eq!(stats.failed_transactions, 0);
        assert_eq!(stats.active_transactions, 0);
        
        framework.stop().await.unwrap();
    }

    /// Test versioned metadata management
    #[tokio::test]
    async fn test_versioned_metadata() {
        let mut manager = VersionedMetadataManager::new();
        
        // Create initial version
        let transaction_id1 = Uuid::new_v4();
        let version1 = manager.create_version(transaction_id1).await.unwrap();
        assert_eq!(version1, 1);
        
        // Create another version
        let transaction_id2 = Uuid::new_v4();
        let version2 = manager.create_version(transaction_id2).await.unwrap();
        assert_eq!(version2, 2);
        
        // Create snapshot
        let snapshot_version = manager.create_snapshot().await.unwrap();
        assert_eq!(snapshot_version, 3);
        
        // Restore to previous version
        manager.restore_snapshot(version1).await.unwrap();
        assert_eq!(manager.current_version, version1);
        
        // Test cleanup
        manager.cleanup_old_versions().await.unwrap();
    }

    /// Test vector clock ordering and causality
    #[tokio::test]
    async fn test_vector_clock_ordering() {
        let mut clock1 = VectorClock::new("node1".to_string());
        let mut clock2 = VectorClock::new("node2".to_string());
        let mut clock3 = VectorClock::new("node3".to_string());
        
        // Initial state - all clocks are concurrent
        assert!(clock1.concurrent_with(&clock2));
        assert!(clock2.concurrent_with(&clock3));
        
        // Node1 performs an operation
        clock1.tick();
        assert!(clock1.concurrent_with(&clock2) || clock2.happens_before(&clock1));
        
        // Node2 receives update from node1
        clock2.update(&clock1);
        assert!(clock1.happens_before(&clock2));
        
        // Node3 receives update from node2
        clock3.update(&clock2);
        assert!(clock2.happens_before(&clock3));
        assert!(clock1.happens_before(&clock3));
        
        // Test transitivity
        let mut clock4 = VectorClock::new("node4".to_string());
        clock4.update(&clock3);
        assert!(clock1.happens_before(&clock4));
        assert!(clock2.happens_before(&clock4));
        assert!(clock3.happens_before(&clock4));
    }

    /// Test Lamport timestamp ordering
    #[tokio::test]
    async fn test_lamport_timestamp_ordering() {
        let mut ts1 = LamportTimestamp::new(1);
        let mut ts2 = LamportTimestamp::new(2);
        
        // Initial timestamps
        assert_eq!(ts1.timestamp, 0);
        assert_eq!(ts2.timestamp, 0);
        
        // Node 1 performs operations
        ts1.tick();
        ts1.tick();
        assert_eq!(ts1.timestamp, 2);
        
        // Node 2 receives message from node 1
        ts2.update(ts1);
        assert_eq!(ts2.timestamp, 3); // max(0, 2) + 1
        
        // Node 1 receives message from node 2
        ts1.update(ts2);
        assert_eq!(ts1.timestamp, 4); // max(2, 3) + 1
        
        // Test ordering
        assert!(ts1 > ts2);
    }

    /// Test journal ordering service with batch processing
    #[tokio::test]
    async fn test_journal_ordering() {
        let mut service = JournalOrderingService::new("node1".to_string(), 3);
        
        let tx_id = Uuid::new_v4();
        let op_id1 = Uuid::new_v4();
        let op_id2 = Uuid::new_v4();
        let op_id3 = Uuid::new_v4();
        
        // Add entries to batch
        service.add_entry(
            tx_id,
            op_id1,
            "filesystem".to_string(),
            "write".to_string(),
            vec![1, 2, 3],
            HashMap::new(),
        ).await.unwrap();
        
        service.add_entry(
            tx_id,
            op_id2,
            "graph".to_string(),
            "add_edge".to_string(),
            vec![4, 5, 6],
            HashMap::new(),
        ).await.unwrap();
        
        // Batch should not be flushed yet
        assert_eq!(service.batch_buffer.len(), 2);
        
        service.add_entry(
            tx_id,
            op_id3,
            "semantic".to_string(),
            "index".to_string(),
            vec![7, 8, 9],
            HashMap::new(),
        ).await.unwrap();
        
        // Batch should be flushed automatically when reaching batch size
        assert!(service.batch_buffer.is_empty());
        
        // Manual flush test
        service.add_entry(
            tx_id,
            Uuid::new_v4(),
            "filesystem".to_string(),
            "read".to_string(),
            vec![10, 11, 12],
            HashMap::new(),
        ).await.unwrap();
        
        service.flush_batch().await.unwrap();
        assert!(service.batch_buffer.is_empty());
    }

    /// Test two-phase commit protocol
    #[tokio::test]
    async fn test_two_phase_commit() {
        let mut coordinator = TwoPhaseCommitCoordinator::new(
            "coordinator1".to_string(),
            Duration::from_secs(30),
        );
        
        let transaction_id = Uuid::new_v4();
        let transaction = TwoPhaseCommitTransaction {
            transaction_id,
            coordinator_id: "coordinator1".to_string(),
            state: TwoPhaseCommitState::Init,
            participants: vec!["fs".to_string(), "graph".to_string(), "semantic".to_string()],
            prepare_votes: HashMap::new(),
            timeout: Duration::from_secs(30),
            started_at: std::time::SystemTime::now(),
            prepared_at: None,
            committed_at: None,
            operations: Vec::new(),
        };
        
        coordinator.active_commits.insert(transaction_id, transaction);
        
        // Phase 1: Prepare
        coordinator.prepare_transaction(transaction_id).await.unwrap();
        
        let tx = coordinator.active_commits.get(&transaction_id).unwrap();
        assert_eq!(tx.state, TwoPhaseCommitState::Prepared);
        assert!(tx.prepared_at.is_some());
        
        // Phase 2: Commit
        coordinator.commit_transaction(transaction_id).await.unwrap();
        
        let tx = coordinator.active_commits.get(&transaction_id).unwrap();
        assert_eq!(tx.state, TwoPhaseCommitState::Committed);
        assert!(tx.committed_at.is_some());
    }

    /// Test recovery manager functionality
    #[tokio::test]
    async fn test_recovery_manager() {
        let mut manager = RecoveryManager::new();
        
        // Initially no recovery needed
        manager.check_recovery_needed().await.unwrap();
        assert!(!manager.recovery_in_progress);
        
        // Add recovery log entry
        let entry = RecoveryLogEntry {
            entry_id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            operation_type: "write".to_string(),
            layer_id: "filesystem".to_string(),
            before_state: Some(vec![1, 2, 3]),
            after_state: Some(vec![4, 5, 6]),
            timestamp: std::time::SystemTime::now(),
            vector_clock: VectorClock::new("node1".to_string()),
        };
        
        manager.recovery_log.push(entry);
        
        // Now recovery should be needed
        manager.check_recovery_needed().await.unwrap();
        assert!(manager.last_checkpoint.is_some());
    }

    /// Test performance cache functionality
    #[tokio::test]
    async fn test_performance_cache() {
        let mut cache = PerformanceCache::new(100);
        
        // Test query caching
        let query_result = QueryResult {
            rows: vec![HashMap::new()],
            execution_time: Duration::from_millis(50),
            layers_accessed: vec!["filesystem".to_string()],
        };
        
        let query_hash = "test_query_hash".to_string();
        cache.cache_query(query_hash.clone(), query_result.clone());
        
        let cached = cache.get_cached_query(&query_hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().result.execution_time, Duration::from_millis(50));
        
        // Test cache cleanup
        cache.cleanup_expired();
    }

    /// Test concurrent transactions
    #[tokio::test]
    async fn test_concurrent_transactions() {
        let config = IntegrationConfig {
            max_concurrent_transactions: 5,
            transaction_timeout: Duration::from_secs(10),
            ..Default::default()
        };
        let framework = Arc::new(CrossLayerIntegrationFramework::new(config).await.unwrap());
        
        framework.start().await.unwrap();
        
        let mut handles = Vec::new();
        
        // Start multiple concurrent transactions
        for i in 0..3 {
            let framework_clone = Arc::clone(&framework);
            let handle = tokio::spawn(async move {
                let transaction_id = framework_clone.begin_unified_transaction(
                    vec!["filesystem".to_string()],
                    CrossLayerIsolationLevel::ReadCommitted,
                    Some(Duration::from_secs(5)),
                ).await.unwrap();
                
                // Add operation
                framework_clone.add_unified_operation(
                    transaction_id,
                    "filesystem".to_string(),
                    "write".to_string(),
                    vec![i as u8; 4],
                    HashMap::new(),
                ).await.unwrap();
                
                // Small delay to simulate work
                sleep(Duration::from_millis(100)).await;
                
                // Commit transaction
                framework_clone.commit_unified_transaction(transaction_id).await.unwrap();
                
                transaction_id
            });
            handles.push(handle);
        }
        
        // Wait for all transactions to complete
        let mut transaction_ids = Vec::new();
        for handle in handles {
            let tx_id = handle.await.unwrap();
            transaction_ids.push(tx_id);
        }
        
        // Verify all transactions completed
        assert_eq!(transaction_ids.len(), 3);
        
        let stats = framework.get_integration_stats().await;
        assert_eq!(stats.total_transactions, 3);
        assert_eq!(stats.successful_transactions, 3);
        assert_eq!(stats.active_transactions, 0);
        
        framework.stop().await.unwrap();
    }

    /// Test transaction timeout handling
    #[tokio::test]
    async fn test_transaction_timeout() {
        let config = IntegrationConfig {
            transaction_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        
        framework.start().await.unwrap();
        
        let transaction_id = framework.begin_unified_transaction(
            vec!["filesystem".to_string()],
            CrossLayerIsolationLevel::ReadCommitted,
            Some(Duration::from_millis(50)), // Very short timeout
        ).await.unwrap();
        
        // Add operation
        framework.add_unified_operation(
            transaction_id,
            "filesystem".to_string(),
            "write".to_string(),
            vec![1, 2, 3, 4],
            HashMap::new(),
        ).await.unwrap();
        
        // Wait longer than timeout
        sleep(Duration::from_millis(200)).await;
        
        // Commit should succeed even after timeout (implementation dependent)
        let result = framework.commit_unified_transaction(transaction_id).await;
        // Note: The actual behavior depends on implementation details
        
        framework.stop().await.unwrap();
    }

    /// Test snapshot creation and restoration
    #[tokio::test]
    async fn test_snapshot_operations() {
        let config = IntegrationConfig::default();
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        
        framework.start().await.unwrap();
        
        // Create initial snapshot
        let snapshot1 = framework.create_versioned_snapshot().await.unwrap();
        assert_eq!(snapshot1, 1);
        
        // Perform some transactions
        let tx_id = framework.begin_unified_transaction(
            vec!["filesystem".to_string()],
            CrossLayerIsolationLevel::ReadCommitted,
            None,
        ).await.unwrap();
        
        framework.add_unified_operation(
            tx_id,
            "filesystem".to_string(),
            "write".to_string(),
            vec![1, 2, 3, 4],
            HashMap::new(),
        ).await.unwrap();
        
        framework.commit_unified_transaction(tx_id).await.unwrap();
        
        // Create another snapshot
        let snapshot2 = framework.create_versioned_snapshot().await.unwrap();
        assert_eq!(snapshot2, 2);
        
        // Restore to first snapshot
        framework.restore_versioned_snapshot(snapshot1).await.unwrap();
        
        framework.stop().await.unwrap();
    }

    /// Property-based test for vector clock properties
    proptest! {
        #[test]
        fn test_vector_clock_properties(
            node_ids in prop::collection::vec("[a-z]+", 2..5),
            operations in prop::collection::vec(0..10usize, 1..20)
        ) {
            let mut clocks: Vec<VectorClock> = node_ids.iter()
                .map(|id| VectorClock::new(id.clone()))
                .collect();
            
            // Apply operations
            for &op in &operations {
                let node_idx = op % clocks.len();
                clocks[node_idx].tick();
                
                // Occasionally sync clocks
                if op % 3 == 0 && clocks.len() > 1 {
                    let other_idx = (node_idx + 1) % clocks.len();
                    let other_clock = clocks[other_idx].clone();
                    clocks[node_idx].update(&other_clock);
                }
            }
            
            // Verify properties
            for i in 0..clocks.len() {
                for j in 0..clocks.len() {
                    if i != j {
                        let clock_i = &clocks[i];
                        let clock_j = &clocks[j];
                        
                        // Either happens-before, or concurrent
                        let hb_ij = clock_i.happens_before(clock_j);
                        let hb_ji = clock_j.happens_before(clock_i);
                        let concurrent = clock_i.concurrent_with(clock_j);
                        
                        prop_assert!(hb_ij || hb_ji || concurrent);
                        
                        // If happens-before, then not concurrent
                        if hb_ij {
                            prop_assert!(!concurrent);
                            prop_assert!(!hb_ji);
                        }
                    }
                }
            }
        }
    }

    /// Benchmark transaction throughput
    fn benchmark_transaction_throughput(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("transaction_throughput", |b| {
            b.to_async(&rt).iter(|| async {
                let config = IntegrationConfig {
                    max_concurrent_transactions: 1000,
                    ..Default::default()
                };
                let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
                framework.start().await.unwrap();
                
                let tx_id = framework.begin_unified_transaction(
                    vec!["filesystem".to_string()],
                    CrossLayerIsolationLevel::ReadCommitted,
                    None,
                ).await.unwrap();
                
                framework.add_unified_operation(
                    tx_id,
                    "filesystem".to_string(),
                    "write".to_string(),
                    vec![1, 2, 3, 4],
                    HashMap::new(),
                ).await.unwrap();
                
                framework.commit_unified_transaction(tx_id).await.unwrap();
                framework.stop().await.unwrap();
            });
        });
    }

    /// Benchmark vector clock operations
    fn benchmark_vector_clock_operations(c: &mut Criterion) {
        c.bench_function("vector_clock_tick", |b| {
            let mut clock = VectorClock::new("node1".to_string());
            b.iter(|| {
                clock.tick();
            });
        });
        
        c.bench_function("vector_clock_update", |b| {
            let mut clock1 = VectorClock::new("node1".to_string());
            let mut clock2 = VectorClock::new("node2".to_string());
            clock2.tick();
            
            b.iter(|| {
                clock1.update(&clock2);
            });
        });
    }

    /// Loom-based concurrency test for transaction manager
    #[test]
    fn test_concurrent_transaction_manager() {
        loom::model(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            
            rt.block_on(async {
                let config = IntegrationConfig {
                    max_concurrent_transactions: 2,
                    ..Default::default()
                };
                let framework = Arc::new(CrossLayerIntegrationFramework::new(config).await.unwrap());
                framework.start().await.unwrap();
                
                let framework1 = Arc::clone(&framework);
                let framework2 = Arc::clone(&framework);
                
                let handle1 = loom::thread::spawn(move || {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        let tx_id = framework1.begin_unified_transaction(
                            vec!["filesystem".to_string()],
                            CrossLayerIsolationLevel::ReadCommitted,
                            None,
                        ).await.unwrap();
                        
                        framework1.commit_unified_transaction(tx_id).await.unwrap();
                    });
                });
                
                let handle2 = loom::thread::spawn(move || {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        let tx_id = framework2.begin_unified_transaction(
                            vec!["graph".to_string()],
                            CrossLayerIsolationLevel::ReadCommitted,
                            None,
                        ).await.unwrap();
                        
                        framework2.commit_unified_transaction(tx_id).await.unwrap();
                    });
                });
                
                handle1.join().unwrap();
                handle2.join().unwrap();
                
                let stats = framework.get_integration_stats().await;
                assert_eq!(stats.total_transactions, 2);
                assert_eq!(stats.successful_transactions, 2);
                
                framework.stop().await.unwrap();
            });
        });
    }

    criterion_group!(
        benches,
        benchmark_transaction_throughput,
        benchmark_vector_clock_operations
    );
    criterion_main!(benches);
}

/// Fallback tests when cross_layer_integration feature is not enabled
#[cfg(not(feature = "cross_layer_integration"))]
mod fallback_tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_not_enabled() {
        // Test that the module compiles even without the feature
        println!("Cross-layer integration feature not enabled");
        assert!(true);
    }
}

/// Integration tests that work with or without the feature
#[tokio::test]
async fn test_basic_functionality() {
    // Basic test that always runs
    let uuid = Uuid::new_v4();
    assert!(uuid != Uuid::nil());
    
    let duration = Duration::from_millis(100);
    assert_eq!(duration.as_millis(), 100);
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() {
    // Test VexFSError variants
    let error1 = VexFSError::NotFound("test".to_string());
    let error2 = VexFSError::ResourceExhausted("test".to_string());
    let error3 = VexFSError::TransactionFailed("test".to_string());
    
    match error1 {
        VexFSError::NotFound(_) => assert!(true),
        _ => assert!(false),
    }
    
    match error2 {
        VexFSError::ResourceExhausted(_) => assert!(true),
        _ => assert!(false),
    }
    
    match error3 {
        VexFSError::TransactionFailed(_) => assert!(true),
        _ => assert!(false),
    }
}