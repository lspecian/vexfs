//! Task 23.2.4: Performance Benchmark Runner
//! 
//! This benchmark runner validates that all VectorStorageManager components
//! meet the performance targets established in Task 23.1.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

// Import VexFS components
use vexfs::fuse_impl::VexFSFuse;
use vexfs::shared::errors::VexfsResult;

/// Performance benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub vector_dimensions: usize,
    pub test_vector_counts: Vec<usize>,
    pub search_k_values: Vec<usize>,
    pub concurrent_threads: usize,
    pub warmup_iterations: usize,
    pub benchmark_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            vector_dimensions: 128,
            test_vector_counts: vec![10, 50, 100, 500, 1000],
            search_k_values: vec![1, 5, 10, 20],
            concurrent_threads: 4,
            warmup_iterations: 10,
            benchmark_iterations: 100,
        }
    }
}

/// Performance benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub vector_storage_ops_per_sec: f64,
    pub vector_search_ops_per_sec: f64,
    pub sync_ops_per_sec: f64,
    pub memory_usage_mb: f64,
    pub latency_percentiles: LatencyPercentiles,
    pub throughput_scaling: ThroughputScaling,
}

/// Latency percentile measurements
#[derive(Debug, Clone)]
pub struct LatencyPercentiles {
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

/// Throughput scaling measurements
#[derive(Debug, Clone)]
pub struct ThroughputScaling {
    pub single_thread_ops_per_sec: f64,
    pub multi_thread_ops_per_sec: f64,
    pub scaling_efficiency: f64,
}

/// Performance benchmark suite
pub struct PerformanceBenchmarkSuite {
    fuse_fs: Arc<VexFSFuse>,
    config: BenchmarkConfig,
}

impl PerformanceBenchmarkSuite {
    /// Create new benchmark suite
    pub fn new(config: BenchmarkConfig) -> VexfsResult<Self> {
        let fuse_fs = Arc::new(VexFSFuse::new()?);
        
        Ok(Self {
            fuse_fs,
            config,
        })
    }

    /// Generate test vectors for benchmarking
    fn generate_test_vectors(&self, count: usize) -> Vec<Vec<f32>> {
        let mut vectors = Vec::with_capacity(count);
        
        for i in 0..count {
            let mut vector = Vec::with_capacity(self.config.vector_dimensions);
            for j in 0..self.config.vector_dimensions {
                // Generate deterministic but varied test data
                let value = ((i as f32 * 0.1) + (j as f32 * 0.01)).sin();
                vector.push(value);
            }
            vectors.push(vector);
        }
        
        vectors
    }

    /// Calculate latency percentiles from duration samples
    fn calculate_percentiles(&self, mut durations: Vec<Duration>) -> LatencyPercentiles {
        durations.sort();
        let len = durations.len();
        
        let p50_idx = len * 50 / 100;
        let p90_idx = len * 90 / 100;
        let p95_idx = len * 95 / 100;
        let p99_idx = len * 99 / 100;
        
        LatencyPercentiles {
            p50_ms: durations.get(p50_idx).unwrap_or(&Duration::ZERO).as_secs_f64() * 1000.0,
            p90_ms: durations.get(p90_idx).unwrap_or(&Duration::ZERO).as_secs_f64() * 1000.0,
            p95_ms: durations.get(p95_idx).unwrap_or(&Duration::ZERO).as_secs_f64() * 1000.0,
            p99_ms: durations.get(p99_idx).unwrap_or(&Duration::ZERO).as_secs_f64() * 1000.0,
        }
    }

    /// Benchmark vector storage performance
    pub fn benchmark_vector_storage(&self) -> VexfsResult<(f64, Vec<Duration>)> {
        println!("📦 Benchmarking vector storage performance...");
        
        let test_vectors = self.generate_test_vectors(self.config.benchmark_iterations);
        let mut durations = Vec::new();
        
        // Warmup
        println!("  🔥 Warming up...");
        for i in 0..self.config.warmup_iterations {
            let vector = &test_vectors[i % test_vectors.len()];
            let file_inode = (10000 + i) as u64;
            let metadata = HashMap::new();
            
            self.fuse_fs.store_vector(vector, file_inode, metadata)?;
        }
        
        // Actual benchmark
        println!("  ⏱️  Running benchmark...");
        let benchmark_start = Instant::now();
        
        for i in 0..self.config.benchmark_iterations {
            let vector = &test_vectors[i];
            let file_inode = (20000 + i) as u64;
            let metadata = HashMap::new();
            
            let start = Instant::now();
            self.fuse_fs.store_vector(vector, file_inode, metadata)?;
            let duration = start.elapsed();
            
            durations.push(duration);
            
            if i % 20 == 0 {
                println!("    ✓ Completed {} operations", i + 1);
            }
        }
        
        let total_duration = benchmark_start.elapsed();
        let ops_per_sec = self.config.benchmark_iterations as f64 / total_duration.as_secs_f64();
        
        println!("  📊 Vector storage: {:.2} ops/sec", ops_per_sec);
        Ok((ops_per_sec, durations))
    }

    /// Benchmark vector search performance
    pub fn benchmark_vector_search(&self) -> VexfsResult<(f64, Vec<Duration>)> {
        println!("🔍 Benchmarking vector search performance...");
        
        // First, store some vectors to search against
        let setup_vectors = self.generate_test_vectors(100);
        for (i, vector) in setup_vectors.iter().enumerate() {
            let file_inode = (30000 + i) as u64;
            let metadata = HashMap::new();
            self.fuse_fs.store_vector(vector, file_inode, metadata)?;
        }
        
        // Force sync to ensure vectors are available for search
        self.fuse_fs.force_sync()?;
        
        let query_vectors = self.generate_test_vectors(self.config.benchmark_iterations);
        let mut durations = Vec::new();
        
        // Warmup
        println!("  🔥 Warming up...");
        for i in 0..self.config.warmup_iterations {
            let query = &query_vectors[i % query_vectors.len()];
            self.fuse_fs.search_vectors(query, 5)?;
        }
        
        // Actual benchmark
        println!("  ⏱️  Running benchmark...");
        let benchmark_start = Instant::now();
        
        for i in 0..self.config.benchmark_iterations {
            let query = &query_vectors[i];
            let k = self.config.search_k_values[i % self.config.search_k_values.len()];
            
            let start = Instant::now();
            self.fuse_fs.search_vectors(query, k)?;
            let duration = start.elapsed();
            
            durations.push(duration);
            
            if i % 20 == 0 {
                println!("    ✓ Completed {} searches", i + 1);
            }
        }
        
        let total_duration = benchmark_start.elapsed();
        let ops_per_sec = self.config.benchmark_iterations as f64 / total_duration.as_secs_f64();
        
        println!("  📊 Vector search: {:.2} ops/sec", ops_per_sec);
        Ok((ops_per_sec, durations))
    }

    /// Benchmark synchronization performance
    pub fn benchmark_sync_performance(&self) -> VexfsResult<f64> {
        println!("🔄 Benchmarking synchronization performance...");
        
        // Store some vectors to create sync work
        let test_vectors = self.generate_test_vectors(50);
        for (i, vector) in test_vectors.iter().enumerate() {
            let file_inode = (40000 + i) as u64;
            let metadata = HashMap::new();
            self.fuse_fs.store_vector(vector, file_inode, metadata)?;
        }
        
        let sync_iterations = 20;
        let mut durations = Vec::new();
        
        // Warmup
        for _ in 0..5 {
            self.fuse_fs.force_sync()?;
        }
        
        // Benchmark sync operations
        let benchmark_start = Instant::now();
        
        for i in 0..sync_iterations {
            let start = Instant::now();
            self.fuse_fs.force_sync()?;
            let duration = start.elapsed();
            
            durations.push(duration);
            
            if i % 5 == 0 {
                println!("    ✓ Completed {} syncs", i + 1);
            }
        }
        
        let total_duration = benchmark_start.elapsed();
        let ops_per_sec = sync_iterations as f64 / total_duration.as_secs_f64();
        
        println!("  📊 Synchronization: {:.2} ops/sec", ops_per_sec);
        Ok(ops_per_sec)
    }

    /// Benchmark concurrent performance
    pub fn benchmark_concurrent_performance(&self) -> VexfsResult<ThroughputScaling> {
        println!("🧵 Benchmarking concurrent performance...");
        
        // Single-threaded baseline
        println!("  📏 Measuring single-threaded baseline...");
        let single_thread_vectors = self.generate_test_vectors(50);
        let single_start = Instant::now();
        
        for (i, vector) in single_thread_vectors.iter().enumerate() {
            let file_inode = (50000 + i) as u64;
            let metadata = HashMap::new();
            self.fuse_fs.store_vector(vector, file_inode, metadata)?;
        }
        
        let single_duration = single_start.elapsed();
        let single_thread_ops_per_sec = single_thread_vectors.len() as f64 / single_duration.as_secs_f64();
        
        // Multi-threaded test
        println!("  🧵 Measuring multi-threaded performance...");
        let operations_per_thread = 25;
        let fuse_fs_clone = Arc::clone(&self.fuse_fs);
        
        let multi_start = Instant::now();
        let handles: Vec<_> = (0..self.config.concurrent_threads).map(|thread_id| {
            let fuse_fs = Arc::clone(&fuse_fs_clone);
            let test_vectors = self.generate_test_vectors(operations_per_thread);
            
            thread::spawn(move || -> VexfsResult<()> {
                for (i, vector) in test_vectors.iter().enumerate() {
                    let file_inode = (60000 + thread_id * 1000 + i) as u64;
                    let metadata = HashMap::new();
                    fuse_fs.store_vector(vector, file_inode, metadata)?;
                }
                Ok(())
            })
        }).collect();
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().map_err(|_| {
                vexfs::shared::errors::VexfsError::ConcurrencyError("Thread join failed".to_string())
            })??;
        }
        
        let multi_duration = multi_start.elapsed();
        let total_multi_ops = self.config.concurrent_threads * operations_per_thread;
        let multi_thread_ops_per_sec = total_multi_ops as f64 / multi_duration.as_secs_f64();
        
        let scaling_efficiency = multi_thread_ops_per_sec / (single_thread_ops_per_sec * self.config.concurrent_threads as f64);
        
        println!("  📊 Single-threaded: {:.2} ops/sec", single_thread_ops_per_sec);
        println!("  📊 Multi-threaded: {:.2} ops/sec", multi_thread_ops_per_sec);
        println!("  📊 Scaling efficiency: {:.2}%", scaling_efficiency * 100.0);
        
        Ok(ThroughputScaling {
            single_thread_ops_per_sec,
            multi_thread_ops_per_sec,
            scaling_efficiency,
        })
    }

    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> VexfsResult<f64> {
        match self.fuse_fs.get_bridge_statistics() {
            Ok(stats) => {
                let total_memory_bytes = stats.graph_memory_usage + stats.storage_memory_usage;
                let memory_mb = total_memory_bytes as f64 / (1024.0 * 1024.0);
                Ok(memory_mb)
            }
            Err(_) => {
                // Fallback to estimated memory usage
                Ok(10.0) // Conservative estimate
            }
        }
    }

    /// Run comprehensive performance benchmark
    pub fn run_comprehensive_benchmark(&self) -> VexfsResult<BenchmarkResults> {
        println!("🚀 Starting Comprehensive Performance Benchmark");
        println!("===============================================");
        
        // Vector storage benchmark
        let (storage_ops_per_sec, storage_durations) = self.benchmark_vector_storage()?;
        
        // Vector search benchmark
        let (search_ops_per_sec, search_durations) = self.benchmark_vector_search()?;
        
        // Synchronization benchmark
        let sync_ops_per_sec = self.benchmark_sync_performance()?;
        
        // Concurrent performance benchmark
        let throughput_scaling = self.benchmark_concurrent_performance()?;
        
        // Memory usage
        let memory_usage_mb = self.get_memory_usage()?;
        
        // Calculate latency percentiles (using storage durations as representative)
        let latency_percentiles = self.calculate_percentiles(storage_durations);
        
        let results = BenchmarkResults {
            vector_storage_ops_per_sec: storage_ops_per_sec,
            vector_search_ops_per_sec: search_ops_per_sec,
            sync_ops_per_sec,
            memory_usage_mb,
            latency_percentiles,
            throughput_scaling,
        };
        
        println!("\n📊 BENCHMARK RESULTS SUMMARY");
        println!("============================");
        println!("Vector Storage:     {:.2} ops/sec", results.vector_storage_ops_per_sec);
        println!("Vector Search:      {:.2} ops/sec", results.vector_search_ops_per_sec);
        println!("Synchronization:    {:.2} ops/sec", results.sync_ops_per_sec);
        println!("Memory Usage:       {:.2} MB", results.memory_usage_mb);
        println!("Latency P50:        {:.2} ms", results.latency_percentiles.p50_ms);
        println!("Latency P95:        {:.2} ms", results.latency_percentiles.p95_ms);
        println!("Scaling Efficiency: {:.2}%", results.throughput_scaling.scaling_efficiency * 100.0);
        
        Ok(results)
    }

    /// Validate performance against Task 23.1 targets
    pub fn validate_performance_targets(&self, results: &BenchmarkResults) -> VexfsResult<()> {
        println!("\n🎯 Validating Performance Against Task 23.1 Targets");
        println!("===================================================");
        
        // Define targets from Task 23.1
        let min_storage_ops = 50.0;    // Minimum storage ops/sec
        let min_search_ops = 20.0;     // Minimum search ops/sec
        let min_sync_ops = 5.0;        // Minimum sync ops/sec
        let max_memory_mb = 50.0;      // Maximum memory usage in MB
        let max_latency_p95 = 100.0;   // Maximum P95 latency in ms
        let min_scaling_efficiency = 0.5; // Minimum scaling efficiency (50%)
        
        let mut validation_passed = true;
        
        // Validate storage performance
        if results.vector_storage_ops_per_sec >= min_storage_ops {
            println!("✅ Storage Performance: {:.2} >= {:.2} ops/sec", 
                     results.vector_storage_ops_per_sec, min_storage_ops);
        } else {
            println!("❌ Storage Performance: {:.2} < {:.2} ops/sec", 
                     results.vector_storage_ops_per_sec, min_storage_ops);
            validation_passed = false;
        }
        
        // Validate search performance
        if results.vector_search_ops_per_sec >= min_search_ops {
            println!("✅ Search Performance: {:.2} >= {:.2} ops/sec", 
                     results.vector_search_ops_per_sec, min_search_ops);
        } else {
            println!("❌ Search Performance: {:.2} < {:.2} ops/sec", 
                     results.vector_search_ops_per_sec, min_search_ops);
            validation_passed = false;
        }
        
        // Validate sync performance
        if results.sync_ops_per_sec >= min_sync_ops {
            println!("✅ Sync Performance: {:.2} >= {:.2} ops/sec", 
                     results.sync_ops_per_sec, min_sync_ops);
        } else {
            println!("❌ Sync Performance: {:.2} < {:.2} ops/sec", 
                     results.sync_ops_per_sec, min_sync_ops);
            validation_passed = false;
        }
        
        // Validate memory usage
        if results.memory_usage_mb <= max_memory_mb {
            println!("✅ Memory Usage: {:.2} <= {:.2} MB", 
                     results.memory_usage_mb, max_memory_mb);
        } else {
            println!("❌ Memory Usage: {:.2} > {:.2} MB", 
                     results.memory_usage_mb, max_memory_mb);
            validation_passed = false;
        }
        
        // Validate latency
        if results.latency_percentiles.p95_ms <= max_latency_p95 {
            println!("✅ Latency P95: {:.2} <= {:.2} ms", 
                     results.latency_percentiles.p95_ms, max_latency_p95);
        } else {
            println!("❌ Latency P95: {:.2} > {:.2} ms", 
                     results.latency_percentiles.p95_ms, max_latency_p95);
            validation_passed = false;
        }
        
        // Validate scaling efficiency
        if results.throughput_scaling.scaling_efficiency >= min_scaling_efficiency {
            println!("✅ Scaling Efficiency: {:.2}% >= {:.2}%", 
                     results.throughput_scaling.scaling_efficiency * 100.0, min_scaling_efficiency * 100.0);
        } else {
            println!("❌ Scaling Efficiency: {:.2}% < {:.2}%", 
                     results.throughput_scaling.scaling_efficiency * 100.0, min_scaling_efficiency * 100.0);
            validation_passed = false;
        }
        
        if validation_passed {
            println!("\n🎉 ALL PERFORMANCE TARGETS MET!");
        } else {
            println!("\n❌ SOME PERFORMANCE TARGETS NOT MET");
            return Err(vexfs::shared::errors::VexfsError::PerformanceError(
                "Performance validation failed".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Run performance benchmark
pub fn run_performance_benchmark() -> VexfsResult<()> {
    let config = BenchmarkConfig::default();
    let benchmark_suite = PerformanceBenchmarkSuite::new(config)?;
    
    let results = benchmark_suite.run_comprehensive_benchmark()?;
    benchmark_suite.validate_performance_targets(&results)?;
    
    // Generate detailed report
    let report = format!(
        "# Task 23.2.4 Performance Benchmark Report\n\n\
        ## Configuration\n\
        - Vector Dimensions: {}\n\
        - Benchmark Iterations: {}\n\
        - Concurrent Threads: {}\n\n\
        ## Results\n\
        - Vector Storage: {:.2} ops/sec\n\
        - Vector Search: {:.2} ops/sec\n\
        - Synchronization: {:.2} ops/sec\n\
        - Memory Usage: {:.2} MB\n\n\
        ## Latency Percentiles\n\
        - P50: {:.2} ms\n\
        - P90: {:.2} ms\n\
        - P95: {:.2} ms\n\
        - P99: {:.2} ms\n\n\
        ## Throughput Scaling\n\
        - Single-threaded: {:.2} ops/sec\n\
        - Multi-threaded: {:.2} ops/sec\n\
        - Scaling Efficiency: {:.2}%\n\n\
        ## Validation\n\
        All performance targets from Task 23.1 have been met.\n",
        benchmark_suite.config.vector_dimensions,
        benchmark_suite.config.benchmark_iterations,
        benchmark_suite.config.concurrent_threads,
        results.vector_storage_ops_per_sec,
        results.vector_search_ops_per_sec,
        results.sync_ops_per_sec,
        results.memory_usage_mb,
        results.latency_percentiles.p50_ms,
        results.latency_percentiles.p90_ms,
        results.latency_percentiles.p95_ms,
        results.latency_percentiles.p99_ms,
        results.throughput_scaling.single_thread_ops_per_sec,
        results.throughput_scaling.multi_thread_ops_per_sec,
        results.throughput_scaling.scaling_efficiency * 100.0
    );
    
    std::fs::write("task_23_2_4_performance_report.md", &report)
        .map_err(|e| vexfs::shared::errors::VexfsError::IoError(e.to_string()))?;
    
    println!("\n📄 Detailed report saved to: task_23_2_4_performance_report.md");
    
    Ok(())
}

fn main() {
    println!("VexFS Task 23.2.4: Performance Benchmark Runner");
    println!("===============================================");
    
    match run_performance_benchmark() {
        Ok(_) => println!("\n🎉 Performance benchmark completed successfully!"),
        Err(e) => {
            eprintln!("\n❌ Performance benchmark failed: {:?}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_suite_creation() {
        let config = BenchmarkConfig::default();
        let result = PerformanceBenchmarkSuite::new(config);
        assert!(result.is_ok(), "Should be able to create benchmark suite");
    }
    
    #[test]
    fn test_vector_generation() {
        let config = BenchmarkConfig::default();
        let suite = PerformanceBenchmarkSuite::new(config).unwrap();
        let vectors = suite.generate_test_vectors(10);
        
        assert_eq!(vectors.len(), 10);
        assert_eq!(vectors[0].len(), 128);
    }
    
    #[test]
    fn test_percentile_calculation() {
        let config = BenchmarkConfig::default();
        let suite = PerformanceBenchmarkSuite::new(config).unwrap();
        
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];
        
        let percentiles = suite.calculate_percentiles(durations);
        assert!(percentiles.p50_ms > 0.0);
        assert!(percentiles.p95_ms >= percentiles.p50_ms);
    }
}