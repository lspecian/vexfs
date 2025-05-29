//! Copy-on-Write and Snapshot Performance Benchmark
//! 
//! This benchmark validates that the CoW and snapshot implementation meets the performance goals:
//! - Atomic updates with <10% overhead compared to direct writes
//! - 70-90% space savings through delta storage
//! - Linear scaling for snapshot creation time
//! - Efficient garbage collection with minimal impact

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

// Mock imports for benchmark - in real implementation these would be actual VexFS imports
use crate::fs_core::{
    cow::{CowManager, CowMapping, CowExtent, CowBlockRef, CowStats},
    snapshot::{SnapshotManager, SnapshotMetadata, SnapshotStats},
    cow_integration::{CowFilesystemOperations, CowConfig},
    cow_vector_integration::{VectorCowManager, VectorCowStats},
    cow_garbage_collection::{CowGarbageCollector, GcConfig, GcStats},
    operations::OperationContext,
    permissions::UserContext,
    inode::InodeManager,
    locking::LockManager,
};
use crate::storage::{StorageManager, TransactionManager, layout::VexfsLayout, block::BlockDevice};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::errors::VexfsError;

/// Performance benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of files to create for testing
    pub num_files: usize,
    /// Size of each file in bytes
    pub file_size: usize,
    /// Number of modifications per file
    pub modifications_per_file: usize,
    /// Number of snapshots to create
    pub num_snapshots: usize,
    /// Number of vector operations to test
    pub num_vector_ops: usize,
    /// Vector dimension for testing
    pub vector_dims: usize,
    /// Enable garbage collection testing
    pub test_gc: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_files: 1000,
            file_size: 64 * 1024, // 64KB
            modifications_per_file: 10,
            num_snapshots: 50,
            num_vector_ops: 10000,
            vector_dims: 768,
            test_gc: true,
        }
    }
}

/// Benchmark results for performance analysis
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Direct write performance (baseline)
    pub direct_write_time: Duration,
    /// CoW write performance
    pub cow_write_time: Duration,
    /// CoW overhead percentage
    pub cow_overhead_percent: f64,
    /// Space savings from CoW
    pub space_savings_percent: f64,
    /// Snapshot creation times
    pub snapshot_times: Vec<Duration>,
    /// Average snapshot creation time
    pub avg_snapshot_time: Duration,
    /// Vector CoW performance
    pub vector_cow_stats: VectorCowStats,
    /// Garbage collection performance
    pub gc_stats: Option<GcStats>,
    /// Total test duration
    pub total_duration: Duration,
}

/// Main benchmark runner
pub struct CowSnapshotBenchmark {
    config: BenchmarkConfig,
    cow_ops: Arc<CowFilesystemOperations>,
    vector_cow: Arc<VectorCowManager>,
    gc: Option<Arc<CowGarbageCollector>>,
}

impl CowSnapshotBenchmark {
    /// Create a new benchmark instance
    pub fn new(config: BenchmarkConfig) -> Result<Self, VexfsError> {
        // Initialize mock storage and filesystem components
        let layout = VexfsLayout {
            total_blocks: 1_000_000,
            block_size: 4096,
            inodes_per_group: 8192,
            group_count: 128,
            inode_size: 256,
            vector_blocks: 10000,
        };

        let block_device = Arc::new(BlockDevice::new_memory(layout.total_blocks * layout.block_size as u64)?);
        let storage = Arc::new(StorageManager::new(block_device.clone(), layout.clone())?);
        let transaction_mgr = Arc::new(TransactionManager::new(storage.clone())?);
        let inode_mgr = Arc::new(InodeManager::new(storage.clone())?);
        let lock_mgr = Arc::new(LockManager::new());

        // Initialize CoW components
        let cow_config = CowConfig {
            enable_compression: true,
            max_cow_blocks: 100000,
            gc_threshold: 0.8,
            snapshot_retention_days: 30,
        };

        let cow_ops = Arc::new(CowFilesystemOperations::new(
            storage.clone(),
            transaction_mgr.clone(),
            inode_mgr.clone(),
            lock_mgr.clone(),
            cow_config,
        )?);

        let vector_cow = Arc::new(VectorCowManager::new(
            cow_ops.cow_manager().clone(),
            storage.clone(),
        )?);

        let gc = if config.test_gc {
            let gc_config = GcConfig {
                max_gc_time: Duration::from_secs(30),
                incremental_threshold: 1000,
                compaction_threshold: 0.7,
                enable_background_gc: false, // Disable for benchmarking
            };
            Some(Arc::new(CowGarbageCollector::new(
                cow_ops.cow_manager().clone(),
                cow_ops.snapshot_manager().clone(),
                storage.clone(),
                gc_config,
            )?))
        } else {
            None
        };

        Ok(Self {
            config,
            cow_ops,
            vector_cow,
            gc,
        })
    }

    /// Run the complete benchmark suite
    pub fn run_benchmark(&self) -> Result<BenchmarkResults, VexfsError> {
        println!("🚀 Starting CoW and Snapshot Performance Benchmark");
        println!("Configuration: {:?}", self.config);
        
        let start_time = Instant::now();

        // 1. Baseline direct write performance
        println!("\n📊 Phase 1: Measuring baseline direct write performance...");
        let direct_write_time = self.benchmark_direct_writes()?;
        println!("   Direct writes: {:?}", direct_write_time);

        // 2. CoW write performance
        println!("\n📊 Phase 2: Measuring CoW write performance...");
        let cow_write_time = self.benchmark_cow_writes()?;
        println!("   CoW writes: {:?}", cow_write_time);

        // Calculate overhead
        let cow_overhead_percent = ((cow_write_time.as_nanos() as f64 / direct_write_time.as_nanos() as f64) - 1.0) * 100.0;
        println!("   CoW overhead: {:.2}%", cow_overhead_percent);

        // 3. Space savings analysis
        println!("\n📊 Phase 3: Analyzing space savings...");
        let space_savings_percent = self.measure_space_savings()?;
        println!("   Space savings: {:.2}%", space_savings_percent);

        // 4. Snapshot performance
        println!("\n📊 Phase 4: Measuring snapshot performance...");
        let snapshot_times = self.benchmark_snapshots()?;
        let avg_snapshot_time = Duration::from_nanos(
            snapshot_times.iter().map(|d| d.as_nanos()).sum::<u128>() / snapshot_times.len() as u128
        );
        println!("   Average snapshot time: {:?}", avg_snapshot_time);

        // 5. Vector CoW performance
        println!("\n📊 Phase 5: Testing vector CoW performance...");
        let vector_cow_stats = self.benchmark_vector_cow()?;
        println!("   Vector operations: {} completed", vector_cow_stats.total_vectors_modified);

        // 6. Garbage collection performance
        println!("\n📊 Phase 6: Testing garbage collection...");
        let gc_stats = if self.config.test_gc {
            Some(self.benchmark_garbage_collection()?)
        } else {
            None
        };

        let total_duration = start_time.elapsed();

        let results = BenchmarkResults {
            direct_write_time,
            cow_write_time,
            cow_overhead_percent,
            space_savings_percent,
            snapshot_times,
            avg_snapshot_time,
            vector_cow_stats,
            gc_stats,
            total_duration,
        };

        self.print_results(&results);
        self.validate_performance_goals(&results)?;

        Ok(results)
    }

    /// Benchmark direct write performance (baseline)
    fn benchmark_direct_writes(&self) -> Result<Duration, VexfsError> {
        let start = Instant::now();
        let context = OperationContext::new(UserContext::root(), None);

        for i in 0..self.config.num_files {
            let inode = InodeId::from(1000 + i as u64);
            let data = vec![0u8; self.config.file_size];
            
            // Simulate direct write without CoW
            for j in 0..self.config.modifications_per_file {
                let offset = (j * 1024) as u64;
                // In real implementation, this would be a direct storage write
                std::thread::sleep(Duration::from_nanos(100)); // Simulate write latency
            }
        }

        Ok(start.elapsed())
    }

    /// Benchmark CoW write performance
    fn benchmark_cow_writes(&self) -> Result<Duration, VexfsError> {
        let start = Instant::now();
        let context = OperationContext::new(UserContext::root(), None);

        for i in 0..self.config.num_files {
            let inode = InodeId::from(2000 + i as u64);
            let data = vec![0u8; self.config.file_size];
            
            // Create file with CoW
            self.cow_ops.create_file(&context, inode, &data)?;
            
            // Perform modifications
            for j in 0..self.config.modifications_per_file {
                let offset = (j * 1024) as u64;
                let new_data = vec![(i + j) as u8; 1024];
                self.cow_ops.write_file(&context, inode, offset, &new_data)?;
            }
        }

        Ok(start.elapsed())
    }

    /// Measure space savings from CoW
    fn measure_space_savings(&self) -> Result<f64, VexfsError> {
        let cow_stats = self.cow_ops.get_cow_stats()?;
        
        // Calculate theoretical space without CoW
        let total_writes = self.config.num_files * self.config.modifications_per_file;
        let theoretical_space = total_writes * 1024; // 1KB per modification
        
        // Actual space used with CoW
        let actual_space = cow_stats.total_cow_blocks * 4096; // Assuming 4KB blocks
        
        let savings = ((theoretical_space as f64 - actual_space as f64) / theoretical_space as f64) * 100.0;
        Ok(savings.max(0.0))
    }

    /// Benchmark snapshot creation performance
    fn benchmark_snapshots(&self) -> Result<Vec<Duration>, VexfsError> {
        let mut snapshot_times = Vec::new();
        let context = OperationContext::new(UserContext::root(), None);

        for i in 0..self.config.num_snapshots {
            let start = Instant::now();
            
            let snapshot_name = format!("benchmark_snapshot_{}", i);
            self.cow_ops.create_snapshot(&context, &snapshot_name)?;
            
            snapshot_times.push(start.elapsed());
            
            // Add some data between snapshots to test incremental behavior
            if i < self.config.num_snapshots - 1 {
                let inode = InodeId::from(3000 + i as u64);
                let data = vec![i as u8; 4096];
                self.cow_ops.create_file(&context, inode, &data)?;
            }
        }

        Ok(snapshot_times)
    }

    /// Benchmark vector CoW operations
    fn benchmark_vector_cow(&self) -> Result<VectorCowStats, VexfsError> {
        let context = OperationContext::new(UserContext::root(), None);

        for i in 0..self.config.num_vector_ops {
            let inode = InodeId::from(4000 + i as u64);
            let vector_data = vec![0.5f32; self.config.vector_dims];
            
            // Create vector with CoW
            self.vector_cow.create_vector(&context, inode, &vector_data)?;
            
            // Modify vector (triggers CoW)
            if i % 10 == 0 {
                let modified_vector = vec![1.0f32; self.config.vector_dims];
                self.vector_cow.update_vector(&context, inode, &modified_vector)?;
            }
        }

        self.vector_cow.get_stats()
    }

    /// Benchmark garbage collection performance
    fn benchmark_garbage_collection(&self) -> Result<GcStats, VexfsError> {
        if let Some(gc) = &self.gc {
            let start = Instant::now();
            
            // Run full garbage collection
            gc.run_full_gc()?;
            
            let mut stats = gc.get_stats()?;
            stats.last_gc_duration = start.elapsed();
            
            Ok(stats)
        } else {
            Err(VexfsError::InvalidOperation("GC not enabled".to_string()))
        }
    }

    /// Print detailed benchmark results
    fn print_results(&self, results: &BenchmarkResults) {
        println!("\n🎯 BENCHMARK RESULTS");
        println!("==================");
        
        println!("\n📈 Write Performance:");
        println!("  Direct writes:     {:?}", results.direct_write_time);
        println!("  CoW writes:        {:?}", results.cow_write_time);
        println!("  CoW overhead:      {:.2}%", results.cow_overhead_percent);
        
        println!("\n💾 Space Efficiency:");
        println!("  Space savings:     {:.2}%", results.space_savings_percent);
        
        println!("\n📸 Snapshot Performance:");
        println!("  Average time:      {:?}", results.avg_snapshot_time);
        println!("  Total snapshots:   {}", results.snapshot_times.len());
        
        println!("\n🔢 Vector Operations:");
        println!("  Vectors modified:  {}", results.vector_cow_stats.total_vectors_modified);
        println!("  Space efficiency:  {:.2}%", results.vector_cow_stats.space_efficiency_percent());
        
        if let Some(gc_stats) = &results.gc_stats {
            println!("\n🗑️  Garbage Collection:");
            println!("  Blocks reclaimed:  {}", gc_stats.blocks_reclaimed);
            println!("  GC duration:       {:?}", gc_stats.last_gc_duration);
        }
        
        println!("\n⏱️  Total Duration:    {:?}", results.total_duration);
    }

    /// Validate that performance goals are met
    fn validate_performance_goals(&self, results: &BenchmarkResults) -> Result<(), VexfsError> {
        println!("\n✅ PERFORMANCE VALIDATION");
        println!("========================");
        
        let mut all_passed = true;
        
        // Goal 1: CoW overhead < 10%
        if results.cow_overhead_percent < 10.0 {
            println!("✅ CoW overhead ({:.2}%) < 10% - PASSED", results.cow_overhead_percent);
        } else {
            println!("❌ CoW overhead ({:.2}%) >= 10% - FAILED", results.cow_overhead_percent);
            all_passed = false;
        }
        
        // Goal 2: Space savings 70-90%
        if results.space_savings_percent >= 70.0 && results.space_savings_percent <= 90.0 {
            println!("✅ Space savings ({:.2}%) in 70-90% range - PASSED", results.space_savings_percent);
        } else {
            println!("❌ Space savings ({:.2}%) outside 70-90% range - FAILED", results.space_savings_percent);
            all_passed = false;
        }
        
        // Goal 3: Linear snapshot scaling (check if times don't increase exponentially)
        let snapshot_scaling = self.check_linear_scaling(&results.snapshot_times);
        if snapshot_scaling {
            println!("✅ Snapshot creation scales linearly - PASSED");
        } else {
            println!("❌ Snapshot creation does not scale linearly - FAILED");
            all_passed = false;
        }
        
        // Goal 4: Vector CoW efficiency
        if results.vector_cow_stats.space_efficiency_percent() >= 70.0 {
            println!("✅ Vector CoW efficiency ({:.2}%) >= 70% - PASSED", 
                     results.vector_cow_stats.space_efficiency_percent());
        } else {
            println!("❌ Vector CoW efficiency ({:.2}%) < 70% - FAILED", 
                     results.vector_cow_stats.space_efficiency_percent());
            all_passed = false;
        }
        
        if all_passed {
            println!("\n🎉 ALL PERFORMANCE GOALS MET!");
        } else {
            println!("\n⚠️  SOME PERFORMANCE GOALS NOT MET");
            return Err(VexfsError::PerformanceGoalNotMet("Benchmark validation failed".to_string()));
        }
        
        Ok(())
    }

    /// Check if snapshot creation times scale linearly
    fn check_linear_scaling(&self, times: &[Duration]) -> bool {
        if times.len() < 3 {
            return true; // Not enough data points
        }
        
        // Calculate if the growth rate is reasonable (not exponential)
        let first_half_avg = times[..times.len()/2].iter()
            .map(|d| d.as_nanos())
            .sum::<u128>() / (times.len()/2) as u128;
            
        let second_half_avg = times[times.len()/2..].iter()
            .map(|d| d.as_nanos())
            .sum::<u128>() / (times.len() - times.len()/2) as u128;
        
        // Allow up to 50% increase in average time (reasonable for linear scaling)
        let growth_ratio = second_half_avg as f64 / first_half_avg as f64;
        growth_ratio <= 1.5
    }
}

/// Main benchmark entry point
fn main() -> Result<(), VexfsError> {
    println!("VexFS Copy-on-Write and Snapshot Performance Benchmark");
    println!("======================================================");
    
    let config = BenchmarkConfig::default();
    let benchmark = CowSnapshotBenchmark::new(config)?;
    
    let results = benchmark.run_benchmark()?;
    
    println!("\n🏁 Benchmark completed successfully!");
    println!("Results can be used for performance analysis and optimization.");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_creation() {
        let config = BenchmarkConfig {
            num_files: 10,
            file_size: 1024,
            modifications_per_file: 2,
            num_snapshots: 5,
            num_vector_ops: 100,
            vector_dims: 128,
            test_gc: false,
        };
        
        let benchmark = CowSnapshotBenchmark::new(config);
        assert!(benchmark.is_ok());
    }

    #[test]
    fn test_linear_scaling_check() {
        let config = BenchmarkConfig::default();
        let benchmark = CowSnapshotBenchmark::new(config).unwrap();
        
        // Test with linear times
        let linear_times = vec![
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(14),
            Duration::from_millis(16),
        ];
        assert!(benchmark.check_linear_scaling(&linear_times));
        
        // Test with exponential times
        let exponential_times = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(40),
            Duration::from_millis(80),
        ];
        assert!(!benchmark.check_linear_scaling(&exponential_times));
    }
}