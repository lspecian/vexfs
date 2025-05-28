//! Comprehensive Integration Test Suite for Enhanced IOCTL Interface
//!
//! This module provides extensive testing for the enhanced IOCTL interface implementation
//! including security validation, performance optimization, error recovery, logging,
//! and integration with the fs_core architecture components.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ioctl_integration::{
        EnhancedIoctlHandler, SecurityLevel, SecurityConfig, PerformanceConfig, 
        ErrorRecoveryConfig, LoggingConfig, IoctlOperation, IoctlRequest, IoctlResponse, 
        SecurityContext, PerformanceMetrics, RecoveryResult, LogEntry, BatchOperation,
        SearchResultCache, VectorSearchSubsystem, QueryPlanner
    };
    use crate::fs_core::{
        operations::OperationContext,
        enhanced_operation_context::{EnhancedOperationContext, OperationType},
        permissions::UserContext,
        inode::InodeManager,
        locking::LockManager,
    };
    use crate::storage::{
        journal::{VexfsJournal, TransactionManager},
        persistence::StorageManager,
    };
    use crate::shared::{
        errors::{VexfsError, VexfsResult},
        types::{Vector, SearchResult, CacheStats},
        config::VexfsConfig,
    };
    use std::sync::{Arc, Mutex, RwLock};
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant};
    use std::thread;

    /// Test fixture for IOCTL integration tests
    struct IoctlTestFixture {
        handler: EnhancedIoctlHandler,
        enhanced_context: EnhancedOperationContext<'static>,
        user_context: UserContext,
    }

    impl IoctlTestFixture {
        /// Create new test fixture with all required components
        fn new() -> Self {
            let user_context = UserContext {
                uid: 1000,
                gid: 1000,
                pid: 12345,
                capabilities: 0,
            };

            let inode_manager = Arc::new(Mutex::new(InodeManager::new()));
            let lock_manager = Arc::new(RwLock::new(LockManager::new()));
            
            let journal = VexfsJournal::new(4096, 1000);
            let transaction_manager = Arc::new(Mutex::new(TransactionManager::new(journal)));
            let storage_manager = Arc::new(Mutex::new(StorageManager::new(
                transaction_manager.clone(),
                4096,
            )));

            let base_context = OperationContext::new(1, 0);
            let enhanced_context = EnhancedOperationContext::new(
                base_context,
                OperationType::AdminOperation,
                "IOCTL Integration Test".to_string(),
            );

            let vector_search = Arc::new(Mutex::new(VectorSearchSubsystem::new()));
            let query_planner = Arc::new(Mutex::new(QueryPlanner::new()));
            let cache = Arc::new(Mutex::new(SearchResultCache::new(1024 * 1024)));

            let handler = EnhancedIoctlHandler::new(
                SecurityConfig::test_config(),
                PerformanceConfig::test_config(),
                ErrorRecoveryConfig::test_config(),
                LoggingConfig::test_config(),
                vector_search,
                query_planner,
                cache,
                storage_manager,
            );

            Self {
                handler,
                enhanced_context,
                user_context,
            }
        }

        /// Create test IOCTL request
        fn create_test_request(&self, operation: IoctlOperation, data: Vec<u8>) -> IoctlRequest {
            IoctlRequest {
                operation,
                data,
                user_context: self.user_context.clone(),
                security_context: SecurityContext::new(SecurityLevel::Standard),
                transaction_id: Some(1),
                timeout_ms: Some(5000),
                metadata: BTreeMap::new(),
            }
        }

        /// Create test vector for operations
        fn create_test_vector(&self, dimensions: usize) -> Vector {
            Vector::new((0..dimensions).map(|i| i as f32 / dimensions as f32).collect())
        }
    }

    // ============================================================================
    // Core Integration Tests
    // ============================================================================

    #[test]
    fn test_enhanced_ioctl_handler_creation() {
        let fixture = IoctlTestFixture::new();
        
        // Verify handler was created successfully
        assert!(fixture.handler.is_initialized());
        
        // Verify all subsystems are properly connected
        assert!(fixture.handler.has_vector_search_subsystem());
        assert!(fixture.handler.has_query_planner());
        assert!(fixture.handler.has_cache());
        assert!(fixture.handler.has_storage_manager());
    }

    #[test]
    fn test_operation_context_integration() {
        let fixture = IoctlTestFixture::new();
        
        let request = fixture.create_test_request(
            IoctlOperation::VectorSearch,
            vec![1, 2, 3, 4],
        );

        // Test that enhanced operation context is properly integrated
        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.operation_id.is_some());
        assert!(response.telemetry.is_some());
        assert!(response.resource_usage.is_some());
    }

    #[test]
    fn test_vector_search_subsystem_integration() {
        let fixture = IoctlTestFixture::new();
        
        // Add test vectors to the subsystem
        let vector1 = fixture.create_test_vector(128);
        let vector2 = fixture.create_test_vector(128);
        
        let add_request1 = fixture.create_test_request(
            IoctlOperation::AddVector,
            vector1.to_bytes(),
        );
        
        let add_request2 = fixture.create_test_request(
            IoctlOperation::AddVector,
            vector2.to_bytes(),
        );

        // Add vectors
        let result1 = fixture.handler.handle_ioctl(&fixture.enhanced_context, add_request1);
        assert!(result1.is_ok());
        
        let result2 = fixture.handler.handle_ioctl(&fixture.enhanced_context, add_request2);
        assert!(result2.is_ok());

        // Perform search
        let search_request = fixture.create_test_request(
            IoctlOperation::VectorSearch,
            vector1.to_bytes(),
        );

        let search_result = fixture.handler.handle_ioctl(&fixture.enhanced_context, search_request);
        assert!(search_result.is_ok());

        let response = search_result.unwrap();
        assert!(!response.data.is_empty());
        
        // Verify search results contain expected data
        let search_results: Vec<SearchResult> = SearchResult::from_bytes(&response.data).unwrap();
        assert!(!search_results.is_empty());
    }

    #[test]
    fn test_query_planner_integration() {
        let fixture = IoctlTestFixture::new();
        
        let query_request = fixture.create_test_request(
            IoctlOperation::OptimizeQuery,
            b"SELECT * FROM vectors WHERE similarity > 0.8".to_vec(),
        );

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, query_request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.performance_metrics.is_some());
        
        let metrics = response.performance_metrics.unwrap();
        assert!(metrics.query_optimization_time_us > 0);
        assert!(metrics.execution_plan_complexity > 0.0);
    }

    #[test]
    fn test_search_result_cache_integration() {
        let fixture = IoctlTestFixture::new();
        
        let vector = fixture.create_test_vector(64);
        let search_request = fixture.create_test_request(
            IoctlOperation::VectorSearch,
            vector.to_bytes(),
        );

        // First search - should miss cache
        let result1 = fixture.handler.handle_ioctl(&fixture.enhanced_context, search_request.clone());
        assert!(result1.is_ok());
        
        let response1 = result1.unwrap();
        assert_eq!(response1.cache_status, Some("miss".to_string()));

        // Second search - should hit cache
        let result2 = fixture.handler.handle_ioctl(&fixture.enhanced_context, search_request);
        assert!(result2.is_ok());
        
        let response2 = result2.unwrap();
        assert_eq!(response2.cache_status, Some("hit".to_string()));
        
        // Verify cache performance improvement
        assert!(response2.performance_metrics.unwrap().total_time_us < 
                response1.performance_metrics.unwrap().total_time_us);
    }

    // ============================================================================
    // Security Testing
    // ============================================================================

    #[test]
    fn test_security_validation() {
        let fixture = IoctlTestFixture::new();
        
        // Test with invalid security context
        let mut request = fixture.create_test_request(
            IoctlOperation::AdminCommand,
            b"DANGEROUS_COMMAND".to_vec(),
        );
        request.security_context = SecurityContext::new(SecurityLevel::Low);

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            VexfsError::SecurityViolation(_) => {}, // Expected
            _ => panic!("Expected SecurityViolation error"),
        }
    }

    #[test]
    fn test_privilege_escalation_prevention() {
        let fixture = IoctlTestFixture::new();
        
        // Create request that attempts privilege escalation
        let mut request = fixture.create_test_request(
            IoctlOperation::SetPermissions,
            b"uid=0,gid=0".to_vec(),
        );
        request.user_context.uid = 1000; // Non-root user

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            VexfsError::PermissionDenied(_) => {}, // Expected
            _ => panic!("Expected PermissionDenied error"),
        }
    }

    #[test]
    fn test_buffer_validation() {
        let fixture = IoctlTestFixture::new();
        
        // Test with oversized buffer
        let large_buffer = vec![0u8; 10 * 1024 * 1024]; // 10MB
        let request = fixture.create_test_request(
            IoctlOperation::VectorSearch,
            large_buffer,
        );

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            VexfsError::InvalidInput(_) => {}, // Expected
            _ => panic!("Expected InvalidInput error for oversized buffer"),
        }
    }

    #[test]
    fn test_rate_limiting() {
        let fixture = IoctlTestFixture::new();
        
        let request = fixture.create_test_request(
            IoctlOperation::VectorSearch,
            fixture.create_test_vector(32).to_bytes(),
        );

        // Send multiple requests rapidly to trigger rate limiting
        let mut results = Vec::new();
        for _ in 0..100 {
            let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request.clone());
            results.push(result);
        }

        // At least some requests should be rate limited
        let rate_limited_count = results.iter()
            .filter(|r| matches!(r, Err(VexfsError::RateLimited(_))))
            .count();
        
        assert!(rate_limited_count > 0, "Expected some requests to be rate limited");
    }

    // ============================================================================
    // Performance Testing
    // ============================================================================

    #[test]
    fn test_batch_operations() {
        let fixture = IoctlTestFixture::new();
        
        // Create batch of vectors
        let vectors: Vec<Vector> = (0..10)
            .map(|i| fixture.create_test_vector(64 + i))
            .collect();
        
        let batch_data = BatchOperation::new(vectors).to_bytes();
        let batch_request = fixture.create_test_request(
            IoctlOperation::BatchAddVectors,
            batch_data,
        );

        let start_time = Instant::now();
        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, batch_request);
        let batch_duration = start_time.elapsed();
        
        assert!(result.is_ok());
        
        // Compare with individual operations
        let individual_start = Instant::now();
        for vector in &vectors {
            let individual_request = fixture.create_test_request(
                IoctlOperation::AddVector,
                vector.to_bytes(),
            );
            let _ = fixture.handler.handle_ioctl(&fixture.enhanced_context, individual_request);
        }
        let individual_duration = individual_start.elapsed();
        
        // Batch should be significantly faster
        assert!(batch_duration < individual_duration / 2);
    }

    #[test]
    fn test_parallelization() {
        let fixture = IoctlTestFixture::new();
        
        let request = fixture.create_test_request(
            IoctlOperation::ParallelVectorSearch,
            fixture.create_test_vector(128).to_bytes(),
        );

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
        assert!(result.is_ok());

        let response = result.unwrap();
        let metrics = response.performance_metrics.unwrap();
        
        // Verify parallelization was used
        assert!(metrics.parallel_threads_used > 1);
        assert!(metrics.simd_instructions_executed > 0);
    }

    #[test]
    fn test_memory_optimization() {
        let fixture = IoctlTestFixture::new();
        
        let request = fixture.create_test_request(
            IoctlOperation::OptimizeMemory,
            vec![],
        );

        let initial_memory = fixture.enhanced_context.get_resource_usage_summary().memory_current_bytes;
        
        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
        assert!(result.is_ok());

        let final_memory = fixture.enhanced_context.get_resource_usage_summary().memory_current_bytes;
        
        // Memory usage should be optimized (reduced or at least not increased significantly)
        assert!(final_memory <= initial_memory + 1024); // Allow small overhead
    }

    #[test]
    fn test_cache_optimization() {
        let fixture = IoctlTestFixture::new();
        
        // Warm up cache with some searches
        for i in 0..5 {
            let vector = fixture.create_test_vector(64 + i);
            let search_request = fixture.create_test_request(
                IoctlOperation::VectorSearch,
                vector.to_bytes(),
            );
            let _ = fixture.handler.handle_ioctl(&fixture.enhanced_context, search_request);
        }

        // Request cache optimization
        let optimize_request = fixture.create_test_request(
            IoctlOperation::OptimizeCache,
            vec![],
        );

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, optimize_request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.cache_stats.is_some());
        
        let cache_stats = response.cache_stats.unwrap();
        assert!(cache_stats.hit_rate > 0.0);
        assert!(cache_stats.optimization_applied);
    }

    // ============================================================================
    // Error Recovery Testing
    // ============================================================================

    #[test]
    fn test_transaction_based_recovery() {
        let fixture = IoctlTestFixture::new();
        
        // Start a transaction
        let begin_request = fixture.create_test_request(
            IoctlOperation::BeginTransaction,
            vec![],
        );
        
        let begin_result = fixture.handler.handle_ioctl(&fixture.enhanced_context, begin_request);
        assert!(begin_result.is_ok());
        
        let transaction_id = begin_result.unwrap().transaction_id.unwrap();

        // Perform operations within transaction
        let vector = fixture.create_test_vector(64);
        let mut add_request = fixture.create_test_request(
            IoctlOperation::AddVector,
            vector.to_bytes(),
        );
        add_request.transaction_id = Some(transaction_id);

        let add_result = fixture.handler.handle_ioctl(&fixture.enhanced_context, add_request);
        assert!(add_result.is_ok());

        // Simulate error and rollback
        let rollback_request = fixture.create_test_request(
            IoctlOperation::RollbackTransaction,
            transaction_id.to_le_bytes().to_vec(),
        );

        let rollback_result = fixture.handler.handle_ioctl(&fixture.enhanced_context, rollback_request);
        assert!(rollback_result.is_ok());

        // Verify rollback was successful
        let recovery_result = rollback_result.unwrap().recovery_result.unwrap();
        assert_eq!(recovery_result.status, "success");
        assert!(recovery_result.operations_rolled_back > 0);
    }

    #[test]
    fn test_acid_compliance() {
        let fixture = IoctlTestFixture::new();
        
        // Test Atomicity: All operations in transaction succeed or all fail
        let transaction_request = fixture.create_test_request(
            IoctlOperation::AtomicBatchOperation,
            vec![1, 2, 3, 4, 5], // Simulated batch data
        );

        let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, transaction_request);
        
        // Either all operations succeed or transaction is rolled back
        match result {
            Ok(response) => {
                assert_eq!(response.status, "success");
                assert!(response.operations_completed > 0);
            }
            Err(VexfsError::TransactionFailed(_)) => {
                // Acceptable - transaction was properly rolled back
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_error_recovery_strategies() {
        let fixture = IoctlTestFixture::new();
        
        // Test different recovery strategies
        let strategies = vec![
            "checkpoint_rollback",
            "partial_repair", 
            "full_reconstruction",
            "integrity_verification",
        ];

        for strategy in strategies {
            let mut recovery_request = fixture.create_test_request(
                IoctlOperation::RecoverFromError,
                strategy.as_bytes().to_vec(),
            );
            recovery_request.metadata.insert("error_type".to_string(), "corruption".to_string());

            let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, recovery_request);
            assert!(result.is_ok());

            let response = result.unwrap();
            let recovery_result = response.recovery_result.unwrap();
            assert!(recovery_result.recovery_time_ms > 0);
            assert!(!recovery_result.data_loss_occurred || strategy == "partial_repair");
        }
    }

    // ============================================================================
    // Comprehensive Integration Validation
    // ============================================================================

    #[test]
    fn test_end_to_end_workflow() {
        let fixture = IoctlTestFixture::new();
        
        // Complete workflow: Add vectors, search, optimize, recover
        let vectors: Vec<Vector> = (0..5)
            .map(|i| fixture.create_test_vector(64 + i * 8))
            .collect();

        // Step 1: Add vectors
        for (i, vector) in vectors.iter().enumerate() {
            let add_request = fixture.create_test_request(
                IoctlOperation::AddVector,
                vector.to_bytes(),
            );
            
            let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, add_request);
            assert!(result.is_ok(), "Failed to add vector {}", i);
        }

        // Step 2: Perform searches
        for vector in &vectors {
            let search_request = fixture.create_test_request(
                IoctlOperation::VectorSearch,
                vector.to_bytes(),
            );
            
            let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, search_request);
            assert!(result.is_ok());
            
            let response = result.unwrap();
            assert!(!response.data.is_empty());
        }

        // Step 3: Optimize performance
        let optimize_request = fixture.create_test_request(
            IoctlOperation::OptimizeCache,
            vec![],
        );
        
        let optimize_result = fixture.handler.handle_ioctl(&fixture.enhanced_context, optimize_request);
        assert!(optimize_result.is_ok());

        // Step 4: Verify system health
        let health_request = fixture.create_test_request(
            IoctlOperation::SystemHealth,
            vec![],
        );
        
        let health_result = fixture.handler.handle_ioctl(&fixture.enhanced_context, health_request);
        assert!(health_result.is_ok());
        
        let health_response = health_result.unwrap();
        assert_eq!(health_response.status, "healthy");
    }

    #[test]
    fn test_comprehensive_integration_validation() {
        let fixture = IoctlTestFixture::new();
        
        // Test all major subsystem integrations in sequence
        let test_operations = vec![
            (IoctlOperation::SystemInit, vec![]),
            (IoctlOperation::AddVector, fixture.create_test_vector(128).to_bytes()),
            (IoctlOperation::VectorSearch, fixture.create_test_vector(128).to_bytes()),
            (IoctlOperation::OptimizeQuery, b"test query".to_vec()),
            (IoctlOperation::OptimizeCache, vec![]),
            (IoctlOperation::OptimizeMemory, vec![]),
            (IoctlOperation::SystemHealth, vec![]),
        ];

        for (operation, data) in test_operations {
            let request = fixture.create_test_request(operation, data);
            let result = fixture.handler.handle_ioctl(&fixture.enhanced_context, request);
            
            // All operations should complete successfully or with expected errors
            match result {
                Ok(response) => {
                    // Verify response contains expected fields
                    assert!(response.operation_id.is_some());
                    assert!(!response.status.is_empty());
                }
                Err(e) => {
                    // Only allow specific expected errors
                    match e {
                        VexfsError::NotImplemented(_) => {}, // Acceptable for some operations
                        VexfsError::InvalidInput(_) => {}, // Acceptable for malformed requests
                        _ => panic!("Unexpected error for {:?}: {:?}", operation, e),
                    }
                }
            }
        }
    }
}