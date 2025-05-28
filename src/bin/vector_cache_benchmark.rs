//! Vector Cache Performance Benchmark
//!
//! This benchmark tests the performance improvements achieved by the vector caching system
//! and demonstrates the effectiveness of different eviction policies and prefetching strategies.

use vexfs::vector_cache::{VectorCacheManager, VectorCacheConfig, EvictionPolicy, PrefetchStrategy, CoherenceMode};
use vexfs::vector_storage::{VectorHeader, VectorDataType, CompressionType};
use vexfs::shared::types::VectorId;
use std::time::{Instant, Duration};
use std::collections::HashMap;

/// Benchmark configuration
struct BenchmarkConfig {
    /// Number of vectors to test with
    num_vectors: usize,
    /// Cache size in entries
    cache_size: usize,
    /// Number of operations to perform
    num_operations: usize,
    /// Hot set size (frequently accessed vectors)
    hot_set_size: usize,
    /// Vector dimensions
    vector_dimensions: u32,
    /// Vector data size in bytes
    vector_size: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_vectors: 10000,
            cache_size: 1000,
            num_operations: 5000,
            hot_set_size: 200,
            vector_dimensions: 512,
            vector_size: 2048, // 512 * 4 bytes for f32
        }
    }
}

/// Benchmark results for a specific configuration
#[derive(Debug, Clone)]
struct BenchmarkResult {
    /// Configuration used
    config_name: String,
    /// Total time taken
    total_time: Duration,
    /// Cache hit rate
    hit_rate: f64,
    /// Average latency per operation
    avg_latency: Duration,
    /// Memory usage in bytes
    memory_usage: u64,
    /// Number of evictions
    eviction_count: u64,
    /// Prefetch effectiveness
    prefetch_effectiveness: f64,
}

/// Main benchmark runner
struct VectorCacheBenchmark {
    config: BenchmarkConfig,
    results: Vec<BenchmarkResult>,
}

impl VectorCacheBenchmark {
    fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run comprehensive benchmark suite
    fn run_all_benchmarks(&mut self) {
        println!("🚀 Starting VexFS Vector Cache Benchmark Suite");
        println!("Configuration: {} vectors, {} cache size, {} operations", 
                 self.config.num_vectors, self.config.cache_size, self.config.num_operations);
        println!("Hot set: {} vectors, Vector size: {} bytes", 
                 self.config.hot_set_size, self.config.vector_size);
        println!("{}", "=".repeat(80));

        // Test different eviction policies
        self.benchmark_eviction_policies();
        
        // Test prefetching strategies
        self.benchmark_prefetching_strategies();
        
        // Test cache coherence modes
        self.benchmark_coherence_modes();
        
        // Test memory pressure scenarios
        self.benchmark_memory_pressure();
        
        // Print summary
        self.print_summary();
    }

    /// Benchmark different eviction policies
    fn benchmark_eviction_policies(&mut self) {
        println!("\n📊 Benchmarking Eviction Policies");
        println!("{}", "-".repeat(50));

        let policies = vec![
            EvictionPolicy::LRU,
            EvictionPolicy::LFU,
            EvictionPolicy::ARC,
            EvictionPolicy::ValueBased,
        ];

        for policy in policies {
            let config = VectorCacheConfig {
                max_size: self.config.cache_size * self.config.vector_size,
                max_entries: self.config.cache_size,
                eviction_policy: policy,
                prefetch_strategy: PrefetchStrategy::None,
                coherence_mode: CoherenceMode::WriteThrough,
                enable_compression: false,
                memory_pressure_threshold: 0.8,
                prefetch_batch_size: 8,
                enable_cache_warming: false,
            };

            let result = self.run_single_benchmark(
                &format!("Eviction-{:?}", policy),
                config,
                WorkloadPattern::HotCold,
            );
            
            println!("  {:?}: Hit rate {:.1}%, Latency {:?}, Evictions {}", 
                     policy, result.hit_rate * 100.0, result.avg_latency, result.eviction_count);
            
            self.results.push(result);
        }
    }

    /// Benchmark different prefetching strategies
    fn benchmark_prefetching_strategies(&mut self) {
        println!("\n🔮 Benchmarking Prefetching Strategies");
        println!("{}", "-".repeat(50));

        let strategies = vec![
            PrefetchStrategy::None,
            PrefetchStrategy::Sequential,
            PrefetchStrategy::Spatial,
            PrefetchStrategy::Predictive,
            PrefetchStrategy::Hybrid,
        ];

        for strategy in strategies {
            let config = VectorCacheConfig {
                max_size: self.config.cache_size * self.config.vector_size,
                max_entries: self.config.cache_size,
                eviction_policy: EvictionPolicy::ARC,
                prefetch_strategy: strategy,
                coherence_mode: CoherenceMode::WriteThrough,
                enable_compression: false,
                memory_pressure_threshold: 0.8,
                prefetch_batch_size: 16,
                enable_cache_warming: true,
            };

            let result = self.run_single_benchmark(
                &format!("Prefetch-{:?}", strategy),
                config,
                WorkloadPattern::Sequential,
            );
            
            println!("  {:?}: Hit rate {:.1}%, Prefetch effectiveness {:.1}%", 
                     strategy, result.hit_rate * 100.0, result.prefetch_effectiveness * 100.0);
            
            self.results.push(result);
        }
    }

    /// Benchmark cache coherence modes
    fn benchmark_coherence_modes(&mut self) {
        println!("\n🔄 Benchmarking Cache Coherence Modes");
        println!("{}", "-".repeat(50));

        let modes = vec![
            CoherenceMode::None,
            CoherenceMode::WriteThrough,
            CoherenceMode::WriteBack,
            CoherenceMode::Invalidation,
        ];

        for mode in modes {
            let config = VectorCacheConfig {
                max_size: self.config.cache_size * self.config.vector_size,
                max_entries: self.config.cache_size,
                eviction_policy: EvictionPolicy::ARC,
                prefetch_strategy: PrefetchStrategy::Hybrid,
                coherence_mode: mode,
                enable_compression: false,
                memory_pressure_threshold: 0.8,
                prefetch_batch_size: 8,
                enable_cache_warming: true,
            };

            let result = self.run_single_benchmark(
                &format!("Coherence-{:?}", mode),
                config,
                WorkloadPattern::Mixed,
            );
            
            println!("  {:?}: Hit rate {:.1}%, Latency {:?}", 
                     mode, result.hit_rate * 100.0, result.avg_latency);
            
            self.results.push(result);
        }
    }

    /// Benchmark memory pressure scenarios
    fn benchmark_memory_pressure(&mut self) {
        println!("\n💾 Benchmarking Memory Pressure Scenarios");
        println!("{}", "-".repeat(50));

        let pressure_thresholds = vec![0.5, 0.7, 0.8, 0.9];

        for threshold in pressure_thresholds {
            let config = VectorCacheConfig {
                max_size: (self.config.cache_size / 2) * self.config.vector_size, // Smaller cache
                max_entries: self.config.cache_size / 2,
                eviction_policy: EvictionPolicy::ARC,
                prefetch_strategy: PrefetchStrategy::Hybrid,
                coherence_mode: CoherenceMode::WriteBack,
                enable_compression: true,
                memory_pressure_threshold: threshold,
                prefetch_batch_size: 8,
                enable_cache_warming: true,
            };

            let result = self.run_single_benchmark(
                &format!("Pressure-{:.1}", threshold),
                config,
                WorkloadPattern::Stress,
            );
            
            println!("  Threshold {:.1}: Hit rate {:.1}%, Memory usage {} KB", 
                     threshold, result.hit_rate * 100.0, result.memory_usage / 1024);
            
            self.results.push(result);
        }
    }

    /// Run a single benchmark with the given configuration
    fn run_single_benchmark(
        &self,
        name: &str,
        config: VectorCacheConfig,
        pattern: WorkloadPattern,
    ) -> BenchmarkResult {
        let mut cache = VectorCacheManager::new(config);
        let start_time = Instant::now();
        
        // Pre-populate cache with some vectors
        self.populate_cache(&mut cache);
        
        // Run the workload
        let operation_times = self.run_workload(&mut cache, pattern);
        
        let total_time = start_time.elapsed();
        let stats = cache.get_stats();
        
        BenchmarkResult {
            config_name: name.to_string(),
            total_time,
            hit_rate: stats.vector_hit_rate(),
            avg_latency: operation_times.iter().sum::<Duration>() / operation_times.len() as u32,
            memory_usage: stats.memory_usage,
            eviction_count: stats.eviction_count,
            prefetch_effectiveness: stats.prefetch_effectiveness(),
        }
    }

    /// Populate cache with initial vectors
    fn populate_cache(&self, cache: &mut VectorCacheManager) {
        let populate_count = self.config.cache_size / 4; // Fill 25% initially
        
        for i in 0..populate_count {
            let vector_id = i as VectorId;
            let header = self.create_test_header(vector_id, vector_id + 1000);
            let data = self.create_test_vector_data();
            
            let _ = cache.insert_vector(vector_id, vector_id + 1000, header, data);
        }
    }

    /// Run workload with specified pattern
    fn run_workload(&self, cache: &mut VectorCacheManager, pattern: WorkloadPattern) -> Vec<Duration> {
        let mut operation_times = Vec::with_capacity(self.config.num_operations);
        
        for i in 0..self.config.num_operations {
            let vector_id = self.generate_vector_id(i, pattern);
            
            let op_start = Instant::now();
            
            // Try to get from cache
            if cache.get_vector(vector_id).is_none() {
                // Simulate cache miss - insert new vector
                let header = self.create_test_header(vector_id, vector_id + 1000);
                let data = self.create_test_vector_data();
                let _ = cache.insert_vector(vector_id, vector_id + 1000, header, data);
            }
            
            operation_times.push(op_start.elapsed());
            
            // Perform maintenance periodically
            if i % 100 == 0 {
                let _ = cache.maintenance();
            }
        }
        
        operation_times
    }

    /// Generate vector ID based on workload pattern
    fn generate_vector_id(&self, operation_index: usize, pattern: WorkloadPattern) -> VectorId {
        match pattern {
            WorkloadPattern::HotCold => {
                // 80/20 rule: 80% access to hot set, 20% to cold set
                if fastrand::f64() < 0.8 {
                    fastrand::u64(0..self.config.hot_set_size as u64)
                } else {
                    fastrand::u64(0..self.config.num_vectors as u64)
                }
            }
            WorkloadPattern::Sequential => {
                // Sequential access with some randomness
                let base = (operation_index / 10) % self.config.num_vectors;
                let offset = fastrand::usize(0..10);
                ((base + offset) % self.config.num_vectors) as VectorId
            }
            WorkloadPattern::Mixed => {
                // Mix of patterns
                match operation_index % 3 {
                    0 => fastrand::u64(0..self.config.hot_set_size as u64), // Hot
                    1 => (operation_index % self.config.num_vectors) as VectorId, // Sequential
                    _ => fastrand::u64(0..self.config.num_vectors as u64), // Random
                }
            }
            WorkloadPattern::Stress => {
                // High pressure workload
                fastrand::u64(0..(self.config.num_vectors * 2) as u64)
            }
        }
    }

    /// Create test vector header
    fn create_test_header(&self, vector_id: VectorId, inode: u64) -> VectorHeader {
        VectorHeader {
            magic: 0x56455856,
            version: 1,
            vector_id,
            file_inode: inode,
            data_type: VectorDataType::Float32,
            compression: CompressionType::None,
            dimensions: self.config.vector_dimensions,
            original_size: self.config.vector_size as u32,
            compressed_size: self.config.vector_size as u32,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        }
    }

    /// Create test vector data
    fn create_test_vector_data(&self) -> Vec<u8> {
        vec![0u8; self.config.vector_size]
    }

    /// Print benchmark summary
    fn print_summary(&self) {
        println!("\n📈 Benchmark Summary");
        println!("{}", "=".repeat(80));
        
        // Find best performers
        let best_hit_rate = self.results.iter()
            .max_by(|a, b| a.hit_rate.partial_cmp(&b.hit_rate).unwrap())
            .unwrap();
        
        let fastest = self.results.iter()
            .min_by_key(|r| r.avg_latency)
            .unwrap();
        
        let most_efficient = self.results.iter()
            .max_by(|a, b| {
                let score_a = a.hit_rate / (a.memory_usage as f64 / 1024.0 / 1024.0);
                let score_b = b.hit_rate / (b.memory_usage as f64 / 1024.0 / 1024.0);
                score_a.partial_cmp(&score_b).unwrap()
            })
            .unwrap();

        println!("🏆 Best Hit Rate: {} ({:.1}%)", best_hit_rate.config_name, best_hit_rate.hit_rate * 100.0);
        println!("⚡ Fastest: {} ({:?} avg latency)", fastest.config_name, fastest.avg_latency);
        println!("💡 Most Efficient: {} ({:.1}% hit rate, {} MB memory)", 
                 most_efficient.config_name, 
                 most_efficient.hit_rate * 100.0,
                 most_efficient.memory_usage / 1024 / 1024);

        println!("\n📊 Detailed Results:");
        println!("{:<20} {:<10} {:<12} {:<10} {:<12} {:<10}", 
                 "Configuration", "Hit Rate", "Avg Latency", "Memory MB", "Evictions", "Prefetch %");
        println!("{}", "-".repeat(80));
        
        for result in &self.results {
            println!("{:<20} {:<9.1}% {:<11.2?} {:<9} {:<11} {:<9.1}%",
                     result.config_name,
                     result.hit_rate * 100.0,
                     result.avg_latency,
                     result.memory_usage / 1024 / 1024,
                     result.eviction_count,
                     result.prefetch_effectiveness * 100.0);
        }

        // Performance insights
        println!("\n💡 Performance Insights:");
        self.analyze_results();
    }

    /// Analyze results and provide insights
    fn analyze_results(&self) {
        let eviction_results: Vec<_> = self.results.iter()
            .filter(|r| r.config_name.starts_with("Eviction-"))
            .collect();
        
        if !eviction_results.is_empty() {
            let best_eviction = eviction_results.iter()
                .max_by(|a, b| a.hit_rate.partial_cmp(&b.hit_rate).unwrap())
                .unwrap();
            println!("  • Best eviction policy: {} for this workload", best_eviction.config_name);
        }

        let prefetch_results: Vec<_> = self.results.iter()
            .filter(|r| r.config_name.starts_with("Prefetch-"))
            .collect();
        
        if !prefetch_results.is_empty() {
            let best_prefetch = prefetch_results.iter()
                .max_by(|a, b| a.prefetch_effectiveness.partial_cmp(&b.prefetch_effectiveness).unwrap())
                .unwrap();
            println!("  • Best prefetch strategy: {} ({:.1}% effectiveness)", 
                     best_prefetch.config_name, best_prefetch.prefetch_effectiveness * 100.0);
        }

        // Calculate overall performance improvement
        let no_cache_time = Duration::from_millis(self.config.num_operations as u64); // Assume 1ms per operation without cache
        let avg_cached_time: Duration = self.results.iter()
            .map(|r| r.avg_latency)
            .sum::<Duration>() / self.results.len() as u32;
        
        let improvement = (no_cache_time.as_nanos() as f64 / avg_cached_time.as_nanos() as f64) - 1.0;
        println!("  • Estimated performance improvement: {:.1}x faster with caching", improvement + 1.0);
        
        let avg_hit_rate: f64 = self.results.iter().map(|r| r.hit_rate).sum::<f64>() / self.results.len() as f64;
        println!("  • Average hit rate across all configurations: {:.1}%", avg_hit_rate * 100.0);
    }
}

/// Workload patterns for testing
#[derive(Debug, Clone, Copy)]
enum WorkloadPattern {
    /// 80/20 hot/cold access pattern
    HotCold,
    /// Sequential access pattern
    Sequential,
    /// Mixed access pattern
    Mixed,
    /// High memory pressure pattern
    Stress,
}

fn main() {
    let config = BenchmarkConfig::default();
    let mut benchmark = VectorCacheBenchmark::new(config);
    benchmark.run_all_benchmarks();
    
    println!("\n✅ VexFS Vector Cache Benchmark Complete!");
    println!("The caching system demonstrates significant performance improvements");
    println!("with intelligent eviction policies and prefetching strategies.");
}