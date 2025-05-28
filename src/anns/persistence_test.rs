//! Comprehensive tests for ANNS persistence and recovery functionality
//! 
//! This module provides extensive testing for all persistence and recovery
//! mechanisms to ensure data durability and crash consistency.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anns::{
        AnnsPersistenceManager, AnnsRecoveryManager, IndexType, PersistenceConfig,
        RecoveryStrategy, RecoveryOperation, AnnsPersistenceHeader
    };
    use crate::storage::journal::{VexfsJournal, TransactionManager};
    use crate::fs_core::operations::OperationContext;
    use std::sync::{Arc, Mutex};

    /// Create test persistence manager
    fn create_test_persistence_manager() -> AnnsPersistenceManager {
        let journal = VexfsJournal::new(4096, 1000);
        let transaction_manager = Arc::new(Mutex::new(TransactionManager::new(journal)));
        let config = PersistenceConfig::default();
        
        AnnsPersistenceManager::new(4096, transaction_manager, config)
    }

    /// Create test recovery manager
    fn create_test_recovery_manager() -> AnnsRecoveryManager {
        let persistence_manager = Arc::new(Mutex::new(create_test_persistence_manager()));
        let recovery_strategy = RecoveryStrategy::default();
        
        AnnsRecoveryManager::new(persistence_manager, recovery_strategy)
    }

    /// Create test operation context
    fn create_test_context() -> OperationContext {
        OperationContext::new(1, 0) // tid=1, flags=0
    }

    #[test]
    fn test_persistence_header_serialization() {
        let header = AnnsPersistenceHeader::new(
            IndexType::HNSW,
            128,
            1000,
            4096,
            256,
        );
        
        // Test serialization
        let serialized = header.to_bytes();
        assert_eq!(serialized.len(), AnnsPersistenceHeader::serialized_size());
        
        // Test deserialization
        let deserialized = AnnsPersistenceHeader::from_bytes(serialized).unwrap();
        assert_eq!(deserialized.magic, AnnsPersistenceHeader::MAGIC);
        assert_eq!(deserialized.version, AnnsPersistenceHeader::VERSION);
        assert_eq!(deserialized.index_type, IndexType::HNSW.to_u8());
        assert_eq!(deserialized.dimensions, 128);
        assert_eq!(deserialized.vector_count, 1000);
        assert_eq!(deserialized.index_data_size, 4096);
        assert_eq!(deserialized.metadata_size, 256);
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
        
        // Valid header should pass validation
        assert!(header.validate().is_ok());
        
        // Invalid magic should fail
        header.magic = 0x12345678;
        assert!(header.validate().is_err());
        
        // Reset magic and test invalid version
        header.magic = AnnsPersistenceHeader::MAGIC;
        header.version = 999;
        assert!(header.validate().is_err());
        
        // Reset version and test invalid index type
        header.version = AnnsPersistenceHeader::VERSION;
        header.index_type = 99;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_persistence_header_checksums() {
        let mut header = AnnsPersistenceHeader::new(
            IndexType::IVF,
            256,
            2000,
            8192,
            512,
        );
        
        let index_data = vec![1u8; 8192];
        let metadata = vec![2u8; 512];
        
        // Update checksums
        header.update_checksums(&index_data, &metadata);
        
        // Verify checksums
        assert!(header.verify_checksums(&index_data, &metadata));
        
        // Modify data and verify checksums fail
        let mut modified_data = index_data.clone();
        modified_data[0] = 99;
        assert!(!header.verify_checksums(&modified_data, &metadata));
        
        let mut modified_metadata = metadata.clone();
        modified_metadata[0] = 99;
        assert!(!header.verify_checksums(&index_data, &modified_metadata));
    }

    #[test]
    fn test_index_type_conversion() {
        // Test all index types
        let types = vec![
            IndexType::LSH,
            IndexType::IVF,
            IndexType::PQ,
            IndexType::Flat,
            IndexType::HNSW,
        ];
        
        for index_type in types {
            let u8_val = index_type.to_u8();
            let converted_back = IndexType::from_u8(u8_val).unwrap();
            assert_eq!(index_type, converted_back);
        }
        
        // Test invalid conversion
        assert!(IndexType::from_u8(99).is_none());
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

    #[test]
    fn test_persistence_manager_creation() {
        let manager = create_test_persistence_manager();
        let stats = manager.get_persistence_stats().unwrap();
        
        assert_eq!(stats.total_checkpoints, 0);
        assert!(!stats.recovery_needed);
        assert_eq!(stats.recovery_progress, 0.0);
        assert_eq!(stats.delta_count, 0);
        assert_eq!(stats.delta_size, 0);
        assert_eq!(stats.last_checkpoint_id, 0);
    }

    #[test]
    fn test_checkpoint_creation() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        let index_data = vec![1u8; 4096];
        let metadata = vec![2u8; 256];
        
        // Create checkpoint
        let checkpoint_id = manager.persist_index(
            &ctx,
            IndexType::HNSW,
            &index_data,
            &metadata,
        ).unwrap();
        
        assert!(checkpoint_id > 0);
        
        // Verify stats updated
        let stats = manager.get_persistence_stats().unwrap();
        assert_eq!(stats.total_checkpoints, 1);
    }

    #[test]
    fn test_index_recovery() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        let original_index_data = vec![1u8; 4096];
        let original_metadata = vec![2u8; 256];
        
        // Persist index
        let checkpoint_id = manager.persist_index(
            &ctx,
            IndexType::HNSW,
            &original_index_data,
            &original_metadata,
        ).unwrap();
        
        // Recover index
        let (recovered_index_data, recovered_metadata) = manager.recover_index(
            &ctx,
            IndexType::HNSW,
            checkpoint_id,
        ).unwrap();
        
        // Verify recovered data matches original
        assert_eq!(recovered_index_data, original_index_data);
        assert_eq!(recovered_metadata, original_metadata);
    }

    #[test]
    fn test_recovery_manager_creation() {
        let recovery_manager = create_test_recovery_manager();
        let stats = recovery_manager.get_recovery_stats();
        
        assert_eq!(stats.total_recoveries, 0);
        assert_eq!(stats.successful_recoveries, 0);
        assert_eq!(stats.failed_recoveries, 0);
        assert_eq!(stats.average_recovery_time_ms, 0);
        assert_eq!(stats.total_operations_recovered, 0);
        assert_eq!(stats.data_loss_incidents, 0);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_recovery_strategy_configuration() {
        let mut recovery_manager = create_test_recovery_manager();
        
        let custom_strategy = RecoveryStrategy {
            primary_operation: RecoveryOperation::CheckpointRollback,
            fallback_operations: vec![RecoveryOperation::FullReconstruction],
            max_recovery_time: 600,
            allow_data_loss: true,
            automatic_recovery: false,
        };
        
        recovery_manager.set_recovery_strategy(custom_strategy.clone());
        
        // Verify strategy was set (we can't directly access it, but we can test behavior)
        recovery_manager.set_crash_detection(false);
        
        let ctx = create_test_context();
        assert!(!recovery_manager.detect_crash_recovery_needed(&ctx).unwrap());
    }

    #[test]
    fn test_incremental_persistence() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        // Test incremental persistence
        let delta_data = vec![3u8; 1024];
        let result = manager.incremental_persist(&ctx, 1, delta_data);
        assert!(result.is_ok());
        
        // Verify stats
        let stats = manager.get_persistence_stats().unwrap();
        assert_eq!(stats.delta_count, 1);
        assert_eq!(stats.delta_size, 1024);
    }

    #[test]
    fn test_background_checkpoint_creation() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        let index_data = vec![4u8; 2048];
        let metadata = vec![5u8; 128];
        
        // Create background checkpoint
        let checkpoint_id = manager.create_background_checkpoint(
            &ctx,
            IndexType::LSH,
            index_data,
            metadata,
        ).unwrap();
        
        assert!(checkpoint_id > 0);
        
        // Verify checkpoint was created
        let stats = manager.get_persistence_stats().unwrap();
        assert_eq!(stats.total_checkpoints, 1);
    }

    #[test]
    fn test_index_validation() {
        let manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        // Validate all indices (should be empty initially)
        let validation_results = manager.validate_all_indices(&ctx).unwrap();
        assert_eq!(validation_results.len(), 0);
    }

    #[test]
    fn test_wal_compaction() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        // Test WAL compaction
        let result = manager.compact_wal(&ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_index_types_persistence() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        let index_types = vec![
            IndexType::LSH,
            IndexType::IVF,
            IndexType::PQ,
            IndexType::Flat,
            IndexType::HNSW,
        ];
        
        let mut checkpoint_ids = Vec::new();
        
        // Persist different index types
        for (i, index_type) in index_types.iter().enumerate() {
            let index_data = vec![(i + 1) as u8; 1024 * (i + 1)];
            let metadata = vec![(i + 10) as u8; 128 * (i + 1)];
            
            let checkpoint_id = manager.persist_index(
                &ctx,
                *index_type,
                &index_data,
                &metadata,
            ).unwrap();
            
            checkpoint_ids.push(checkpoint_id);
        }
        
        // Verify all checkpoints were created
        let stats = manager.get_persistence_stats().unwrap();
        assert_eq!(stats.total_checkpoints, index_types.len());
        
        // Recover each index and verify
        for (i, (index_type, checkpoint_id)) in index_types.iter().zip(checkpoint_ids.iter()).enumerate() {
            let (recovered_data, recovered_metadata) = manager.recover_index(
                &ctx,
                *index_type,
                *checkpoint_id,
            ).unwrap();
            
            let expected_data = vec![(i + 1) as u8; 1024 * (i + 1)];
            let expected_metadata = vec![(i + 10) as u8; 128 * (i + 1)];
            
            assert_eq!(recovered_data, expected_data);
            assert_eq!(recovered_metadata, expected_metadata);
        }
    }

    #[test]
    fn test_persistence_error_handling() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        // Test recovery of non-existent checkpoint
        let result = manager.recover_index(&ctx, IndexType::HNSW, 999);
        assert!(result.is_err());
        
        // Test recovery with wrong index type
        let index_data = vec![1u8; 1024];
        let metadata = vec![2u8; 128];
        
        let checkpoint_id = manager.persist_index(
            &ctx,
            IndexType::HNSW,
            &index_data,
            &metadata,
        ).unwrap();
        
        // Try to recover with different index type
        let result = manager.recover_index(&ctx, IndexType::LSH, checkpoint_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_operations() {
        let mut recovery_manager = create_test_recovery_manager();
        let ctx = create_test_context();
        
        // Test different recovery operations
        let operations = vec![
            RecoveryOperation::WalReplay,
            RecoveryOperation::CheckpointRollback,
            RecoveryOperation::PartialRepair,
            RecoveryOperation::FullReconstruction,
            RecoveryOperation::IntegrityVerification,
        ];
        
        for operation in operations {
            let result = recovery_manager.execute_recovery_operation(&ctx, operation, 0);
            assert!(result.is_ok());
            
            let recovery_result = result.unwrap();
            assert_eq!(recovery_result.operation_performed, operation);
        }
    }

    #[test]
    fn test_recovery_checkpoint_creation() {
        let mut recovery_manager = create_test_recovery_manager();
        let ctx = create_test_context();
        
        let index_data = vec![6u8; 2048];
        let metadata = vec![7u8; 256];
        
        // Create recovery checkpoint
        let checkpoint_id = recovery_manager.create_recovery_checkpoint(
            &ctx,
            IndexType::PQ,
            &index_data,
            &metadata,
        ).unwrap();
        
        assert!(checkpoint_id > 0);
    }

    #[test]
    fn test_persistence_performance_characteristics() {
        let mut manager = create_test_persistence_manager();
        let ctx = create_test_context();
        
        // Test with various data sizes to ensure performance scales appropriately
        let test_sizes = vec![1024, 4096, 16384, 65536];
        
        for size in test_sizes {
            let index_data = vec![1u8; size];
            let metadata = vec![2u8; size / 8];
            
            let start_time = std::time::Instant::now();
            
            let checkpoint_id = manager.persist_index(
                &ctx,
                IndexType::HNSW,
                &index_data,
                &metadata,
            ).unwrap();
            
            let persist_duration = start_time.elapsed();
            
            let start_time = std::time::Instant::now();
            
            let (recovered_data, recovered_metadata) = manager.recover_index(
                &ctx,
                IndexType::HNSW,
                checkpoint_id,
            ).unwrap();
            
            let recover_duration = start_time.elapsed();
            
            // Verify data integrity
            assert_eq!(recovered_data, index_data);
            assert_eq!(recovered_metadata, metadata);
            
            // Performance should be reasonable (these are very loose bounds for testing)
            assert!(persist_duration.as_millis() < 1000); // Less than 1 second
            assert!(recover_duration.as_millis() < 1000);  // Less than 1 second
        }
    }

    #[test]
    fn test_concurrent_persistence_operations() {
        use std::thread;
        use std::sync::Arc;
        
        let manager = Arc::new(Mutex::new(create_test_persistence_manager()));
        let mut handles = Vec::new();
        
        // Spawn multiple threads to test concurrent operations
        for i in 0..4 {
            let manager_clone = Arc::clone(&manager);
            
            let handle = thread::spawn(move || {
                let ctx = create_test_context();
                let index_data = vec![(i + 1) as u8; 1024];
                let metadata = vec![(i + 10) as u8; 128];
                
                let mut mgr = manager_clone.lock().unwrap();
                mgr.persist_index(
                    &ctx,
                    IndexType::HNSW,
                    &index_data,
                    &metadata,
                ).unwrap()
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        let mut checkpoint_ids = Vec::new();
        for handle in handles {
            let checkpoint_id = handle.join().unwrap();
            checkpoint_ids.push(checkpoint_id);
        }
        
        // Verify all checkpoints were created successfully
        assert_eq!(checkpoint_ids.len(), 4);
        
        // Verify all checkpoint IDs are unique
        let mut sorted_ids = checkpoint_ids.clone();
        sorted_ids.sort();
        sorted_ids.dedup();
        assert_eq!(sorted_ids.len(), checkpoint_ids.len());
    }
}