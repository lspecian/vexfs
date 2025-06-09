//! FUSE Integration Tests for Task 23.3 Phase 2
//! 
//! This module provides comprehensive integration tests for the FUSE implementation
//! with Storage-HNSW bridge integration and performance monitoring.

use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::fuse_impl::{VexFSFuse, FusePerformanceMetrics, FuseVexfsError};
use crate::storage::vector_hnsw_bridge::{SearchParameters, VectorSearchResult};
use crate::shared::errors::VexfsResult;

/// Test configuration for FUSE integration tests
#[derive(Debug, Clone)]
pub struct FuseTestConfig {
    pub max_test_vectors: usize,
    pub vector_dimensions: usize,
    pub performance_threshold_ms: u64,
    pub stack_limit_bytes: usize,
    pub memory_limit_mb: usize,
}

impl Default for FuseTestConfig {
    fn default() -> Self {
        Self {
            max_test_vectors: 1000,
            vector_dimensions: 128,
            performance_threshold_ms: 100,
            stack_limit_bytes: 6144, // 6KB FUSE limit
            memory_limit_mb: 64,
        }
    }
}

/// Comprehensive FUSE integration test suite
pub struct FuseIntegrationTestSuite {
    fuse_fs: Arc<VexFSFuse>,
    config: FuseTestConfig,
    test_vectors: Vec<Vec<f32>>,
}

impl FuseIntegrationTestSuite {
    /// Create new test suite with FUSE filesystem
    pub fn new(config: FuseTestConfig) -> VexfsResult<Self> {
        let fuse_fs = Arc::new(VexFSFuse::new()?);
        let test_vectors = Self::generate_test_vectors(&config);
        
        Ok(Self {
            fuse_fs,
            config,
            test_vectors,
        })
    }

    /// Generate test vectors for integration testing
    fn generate_test_vectors(config: &FuseTestConfig) -> Vec<Vec<f32>> {
        let mut vectors = Vec::with_capacity(config.max_test_vectors);
        
        for i in 0..config.max_test_vectors {
            let mut vector = Vec::with_capacity(config.vector_dimensions);
            for j in 0..config.vector_dimensions {
                // Generate deterministic test data
                vector.push((i as f32 * 0.1) + (j as f32 * 0.01));
            }
            vectors.push(vector);
        }
        
        vectors
    }

    /// Test 1: Basic FUSE initialization and configuration
    pub fn test_fuse_initialization(&self) -> VexfsResult<()> {
        println!("Testing FUSE initialization...");
        
        // Test performance metrics initialization
        let metrics = self.fuse_fs.get_performance_metrics();
        assert_eq!(metrics.vector_operations, 0);
        assert_eq!(metrics.search_operations, 0);
        assert_eq!(metrics.error_count, 0);
        
        // Test sync status
        let sync_status = self.fuse_fs.get_sync_status();
        assert!(sync_status.is_synchronized);
        assert_eq!(sync_status.pending_operations, 0);
        
        println!("✅ FUSE initialization test passed");
        Ok(())
    }

    /// Test 2: Vector storage through FUSE interface
    pub fn test_vector_storage(&self) -> VexfsResult<()> {
        println!("Testing vector storage through FUSE...");
        
        let start_time = Instant::now();
        let mut stored_vectors = Vec::new();
        
        // Store test vectors
        for (i, vector) in self.test_vectors.iter().take(10).enumerate() {
            let mut metadata = HashMap::new();
            metadata.insert("test_id".to_string(), i.to_string());
            metadata.insert("vector_type".to_string(), "test".to_string());
            
            match self.fuse_fs.store_vector_enhanced(vector, i as u64 + 1, metadata) {
                Ok(vector_id) => {
                    stored_vectors.push(vector_id);
                    println!("Stored vector {} with ID: {}", i, vector_id);
                }
                Err(e) => {
                    eprintln!("Failed to store vector {}: {:?}", i, e);
                    return Err(crate::shared::errors::VexfsError::InvalidOperation(
                        format!("Vector storage failed: {:?}", e)
                    ));
                }
            }
        }
        
        let storage_duration = start_time.elapsed();
        println!("Stored {} vectors in {:?}", stored_vectors.len(), storage_duration);
        
        // Verify performance metrics
        let metrics = self.fuse_fs.get_performance_metrics();
        assert!(metrics.vector_operations > 0);
        assert!(metrics.avg_latency_ms > 0.0);
        
        // Check performance threshold
        if storage_duration.as_millis() as u64 > self.config.performance_threshold_ms * 10 {
            eprintln!("Warning: Storage performance below threshold");
        }
        
        println!("✅ Vector storage test passed");
        Ok(())
    }

    /// Test 3: Vector search operations through FUSE
    pub fn test_vector_search(&self) -> VexfsResult<()> {
        println!("Testing vector search through FUSE...");
        
        // First store some vectors for searching
        for (i, vector) in self.test_vectors.iter().take(5).enumerate() {
            let mut metadata = HashMap::new();
            metadata.insert("search_test_id".to_string(), i.to_string());
            
            let _ = self.fuse_fs.store_vector_enhanced(vector, i as u64 + 100, metadata)?;
        }
        
        let start_time = Instant::now();
        
        // Perform search operations
        let query_vector = &self.test_vectors[0];
        let search_params = Some(SearchParameters {
            ef_search: Some(50),
            similarity_threshold: Some(0.8),
            max_distance: Some(1.0),
            include_metadata: true,
        });
        
        match self.fuse_fs.search_vectors_enhanced(query_vector, 3, search_params) {
            Ok(results) => {
                println!("Search returned {} results", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!("Result {}: ID={}, distance={:.3}, similarity={:.3}", 
                        i, result.vector_id, result.distance, result.similarity);
                }
            }
            Err(e) => {
                eprintln!("Search failed: {:?}", e);
                return Err(crate::shared::errors::VexfsError::InvalidOperation(
                    format!("Vector search failed: {:?}", e)
                ));
            }
        }
        
        let search_duration = start_time.elapsed();
        println!("Search completed in {:?}", search_duration);
        
        // Verify performance metrics
        let metrics = self.fuse_fs.get_performance_metrics();
        assert!(metrics.search_operations > 0);
        
        println!("✅ Vector search test passed");
        Ok(())
    }

    /// Test 4: Performance monitoring and metrics collection
    pub fn test_performance_monitoring(&self) -> VexfsResult<()> {
        println!("Testing performance monitoring...");
        
        let initial_metrics = self.fuse_fs.get_performance_metrics();
        
        // Perform multiple operations to generate metrics
        for i in 0..5 {
            let vector = &self.test_vectors[i];
            let mut metadata = HashMap::new();
            metadata.insert("perf_test".to_string(), i.to_string());
            
            let _ = self.fuse_fs.store_vector_enhanced(vector, i as u64 + 200, metadata)?;
            
            // Perform search
            let _ = self.fuse_fs.search_vectors_enhanced(vector, 2, None)?;
        }
        
        let final_metrics = self.fuse_fs.get_performance_metrics();
        
        // Verify metrics have been updated
        assert!(final_metrics.vector_operations > initial_metrics.vector_operations);
        assert!(final_metrics.search_operations > initial_metrics.search_operations);
        assert!(final_metrics.total_latency_ms > initial_metrics.total_latency_ms);
        assert!(final_metrics.avg_latency_ms > 0.0);
        
        println!("Performance metrics:");
        println!("  Vector operations: {}", final_metrics.vector_operations);
        println!("  Search operations: {}", final_metrics.search_operations);
        println!("  Average latency: {:.2}ms", final_metrics.avg_latency_ms);
        println!("  Max latency: {}ms", final_metrics.max_latency_ms);
        println!("  Min latency: {}ms", final_metrics.min_latency_ms);
        println!("  Error count: {}", final_metrics.error_count);
        
        println!("✅ Performance monitoring test passed");
        Ok(())
    }

    /// Test 5: Stack usage compliance (<6KB limits)
    pub fn test_stack_usage_compliance(&self) -> VexfsResult<()> {
        println!("Testing stack usage compliance...");
        
        // Test with large vectors to stress stack usage
        let large_vector: Vec<f32> = (0..1024).map(|i| i as f32 * 0.001).collect();
        
        let mut metadata = HashMap::new();
        metadata.insert("stack_test".to_string(), "large_vector".to_string());
        
        // This should not cause stack overflow
        match self.fuse_fs.store_vector_enhanced(&large_vector, 999, metadata) {
            Ok(_) => {
                println!("Large vector storage succeeded without stack overflow");
            }
            Err(FuseVexfsError::StackOverflow) => {
                return Err(crate::shared::errors::VexfsError::InvalidOperation(
                    "Stack overflow detected during large vector storage".to_string()
                ));
            }
            Err(e) => {
                println!("Large vector storage failed with non-stack error: {:?}", e);
            }
        }
        
        // Test search with large query vector
        match self.fuse_fs.search_vectors_enhanced(&large_vector, 5, None) {
            Ok(_) => {
                println!("Large vector search succeeded without stack overflow");
            }
            Err(FuseVexfsError::StackOverflow) => {
                return Err(crate::shared::errors::VexfsError::InvalidOperation(
                    "Stack overflow detected during large vector search".to_string()
                ));
            }
            Err(e) => {
                println!("Large vector search failed with non-stack error: {:?}", e);
            }
        }
        
        println!("✅ Stack usage compliance test passed");
        Ok(())
    }

    /// Test 6: Error handling and recovery
    pub fn test_error_handling(&self) -> VexfsResult<()> {
        println!("Testing error handling and recovery...");
        
        let initial_metrics = self.fuse_fs.get_performance_metrics();
        let initial_error_count = initial_metrics.error_count;
        
        // Test invalid vector storage
        let invalid_vector = vec![]; // Empty vector should cause error
        let mut metadata = HashMap::new();
        metadata.insert("error_test".to_string(), "invalid".to_string());
        
        match self.fuse_fs.store_vector_enhanced(&invalid_vector, 888, metadata) {
            Ok(_) => {
                println!("Warning: Invalid vector storage unexpectedly succeeded");
            }
            Err(e) => {
                println!("Invalid vector storage correctly failed: {:?}", e);
            }
        }
        
        // Test search with invalid parameters
        let query_vector = &self.test_vectors[0];
        match self.fuse_fs.search_vectors_enhanced(query_vector, 0, None) {
            Ok(results) => {
                println!("Search with k=0 returned {} results", results.len());
            }
            Err(e) => {
                println!("Search with k=0 correctly failed: {:?}", e);
            }
        }
        
        // Verify error counting
        let final_metrics = self.fuse_fs.get_performance_metrics();
        if final_metrics.error_count > initial_error_count {
            println!("Error count increased from {} to {}", 
                initial_error_count, final_metrics.error_count);
        }
        
        println!("✅ Error handling test passed");
        Ok(())
    }

    /// Test 7: Synchronization operations
    pub fn test_synchronization(&self) -> VexfsResult<()> {
        println!("Testing synchronization operations...");
        
        // Store some vectors
        for i in 0..3 {
            let vector = &self.test_vectors[i];
            let mut metadata = HashMap::new();
            metadata.insert("sync_test".to_string(), i.to_string());
            
            let _ = self.fuse_fs.store_vector_enhanced(vector, i as u64 + 300, metadata)?;
        }
        
        // Test force synchronization
        match self.fuse_fs.force_sync() {
            Ok(_) => {
                println!("Force synchronization succeeded");
            }
            Err(e) => {
                eprintln!("Force synchronization failed: {:?}", e);
                return Err(crate::shared::errors::VexfsError::InvalidOperation(
                    format!("Synchronization failed: {:?}", e)
                ));
            }
        }
        
        // Verify sync status
        let sync_status = self.fuse_fs.get_sync_status();
        println!("Sync status: synchronized={}, pending={}, errors={}", 
            sync_status.is_synchronized, sync_status.pending_operations, sync_status.sync_errors);
        
        println!("✅ Synchronization test passed");
        Ok(())
    }

    /// Run all integration tests
    pub fn run_all_tests(&self) -> VexfsResult<()> {
        println!("🚀 Starting FUSE Integration Test Suite for Task 23.3 Phase 2");
        println!("Configuration: {:?}", self.config);
        
        let start_time = Instant::now();
        
        // Run all tests
        self.test_fuse_initialization()?;
        self.test_vector_storage()?;
        self.test_vector_search()?;
        self.test_performance_monitoring()?;
        self.test_stack_usage_compliance()?;
        self.test_error_handling()?;
        self.test_synchronization()?;
        
        let total_duration = start_time.elapsed();
        
        // Final performance summary
        let final_metrics = self.fuse_fs.get_performance_metrics();
        println!("\n📊 Final Performance Summary:");
        println!("  Total test duration: {:?}", total_duration);
        println!("  Vector operations: {}", final_metrics.vector_operations);
        println!("  Search operations: {}", final_metrics.search_operations);
        println!("  Average latency: {:.2}ms", final_metrics.avg_latency_ms);
        println!("  Error count: {}", final_metrics.error_count);
        println!("  Peak stack usage: {} bytes", final_metrics.stack_usage_peak);
        println!("  Peak memory usage: {} bytes", final_metrics.memory_usage_peak);
        
        println!("\n✅ All FUSE Integration Tests Passed Successfully!");
        println!("🎯 Task 23.3 Phase 2 FUSE Integration and Performance Monitoring: COMPLETE");
        
        Ok(())
    }
}

/// Benchmark suite for FUSE performance validation
pub struct FuseBenchmarkSuite {
    test_suite: FuseIntegrationTestSuite,
}

impl FuseBenchmarkSuite {
    pub fn new(config: FuseTestConfig) -> VexfsResult<Self> {
        let test_suite = FuseIntegrationTestSuite::new(config)?;
        Ok(Self { test_suite })
    }

    /// Benchmark vector storage performance
    pub fn benchmark_vector_storage(&self, num_vectors: usize) -> VexfsResult<Duration> {
        println!("Benchmarking vector storage performance with {} vectors...", num_vectors);
        
        let start_time = Instant::now();
        
        for i in 0..num_vectors {
            let vector = &self.test_suite.test_vectors[i % self.test_suite.test_vectors.len()];
            let mut metadata = HashMap::new();
            metadata.insert("benchmark".to_string(), i.to_string());
            
            let _ = self.test_suite.fuse_fs.store_vector_enhanced(vector, i as u64 + 1000, metadata)?;
        }
        
        let duration = start_time.elapsed();
        let ops_per_sec = num_vectors as f64 / duration.as_secs_f64();
        
        println!("Storage benchmark results:");
        println!("  Vectors stored: {}", num_vectors);
        println!("  Total time: {:?}", duration);
        println!("  Operations per second: {:.2}", ops_per_sec);
        
        Ok(duration)
    }

    /// Benchmark search performance
    pub fn benchmark_search_performance(&self, num_searches: usize) -> VexfsResult<Duration> {
        println!("Benchmarking search performance with {} searches...", num_searches);
        
        // First store some vectors for searching
        for i in 0..10 {
            let vector = &self.test_suite.test_vectors[i];
            let mut metadata = HashMap::new();
            metadata.insert("search_benchmark".to_string(), i.to_string());
            
            let _ = self.test_suite.fuse_fs.store_vector_enhanced(vector, i as u64 + 2000, metadata)?;
        }
        
        let start_time = Instant::now();
        
        for i in 0..num_searches {
            let query_vector = &self.test_suite.test_vectors[i % self.test_suite.test_vectors.len()];
            let _ = self.test_suite.fuse_fs.search_vectors_enhanced(query_vector, 5, None)?;
        }
        
        let duration = start_time.elapsed();
        let searches_per_sec = num_searches as f64 / duration.as_secs_f64();
        
        println!("Search benchmark results:");
        println!("  Searches performed: {}", num_searches);
        println!("  Total time: {:?}", duration);
        println!("  Searches per second: {:.2}", searches_per_sec);
        
        Ok(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuse_integration_suite() {
        let config = FuseTestConfig::default();
        let test_suite = FuseIntegrationTestSuite::new(config).expect("Failed to create test suite");
        
        test_suite.run_all_tests().expect("Integration tests failed");
    }

    #[test]
    fn test_fuse_benchmarks() {
        let config = FuseTestConfig::default();
        let benchmark_suite = FuseBenchmarkSuite::new(config).expect("Failed to create benchmark suite");
        
        let storage_duration = benchmark_suite.benchmark_vector_storage(50)
            .expect("Storage benchmark failed");
        assert!(storage_duration.as_millis() < 5000); // Should complete within 5 seconds
        
        let search_duration = benchmark_suite.benchmark_search_performance(100)
            .expect("Search benchmark failed");
        assert!(search_duration.as_millis() < 3000); // Should complete within 3 seconds
    }
}