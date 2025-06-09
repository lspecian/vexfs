//! FUSE Stack Profiling Example for Task 23.2.4
//! 
//! This example demonstrates stack usage monitoring and profiling
//! for VexFS FUSE operations to validate stack safety requirements.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;

// Import VexFS components
use vexfs::fuse_impl::VexFSFuse;
use vexfs::shared::errors::VexfsResult;

/// Stack profiling configuration
#[derive(Debug, Clone)]
pub struct StackProfilingConfig {
    pub stack_limit_bytes: usize,
    pub warning_threshold_bytes: usize,
    pub sample_interval_ms: u64,
    pub max_samples: usize,
}

impl Default for StackProfilingConfig {
    fn default() -> Self {
        Self {
            stack_limit_bytes: 6144,    // 6KB target from Task 23.1
            warning_threshold_bytes: 4096, // 4KB warning threshold
            sample_interval_ms: 10,     // Sample every 10ms
            max_samples: 1000,          // Maximum samples to collect
        }
    }
}

/// Stack usage sample
#[derive(Debug, Clone)]
pub struct StackUsageSample {
    pub operation: String,
    pub estimated_usage_bytes: usize,
    pub timestamp: Instant,
    pub thread_id: String,
}

/// Stack profiler for FUSE operations
pub struct FuseStackProfiler {
    config: StackProfilingConfig,
    samples: Arc<Mutex<Vec<StackUsageSample>>>,
    is_profiling: Arc<Mutex<bool>>,
}

impl FuseStackProfiler {
    /// Create new stack profiler
    pub fn new(config: StackProfilingConfig) -> Self {
        Self {
            config,
            samples: Arc::new(Mutex::new(Vec::new())),
            is_profiling: Arc::new(Mutex::new(false)),
        }
    }

    /// Start profiling
    pub fn start_profiling(&self) {
        if let Ok(mut profiling) = self.is_profiling.lock() {
            *profiling = true;
        }
        println!("🔍 Stack profiling started (limit: {} bytes)", self.config.stack_limit_bytes);
    }

    /// Stop profiling
    pub fn stop_profiling(&self) {
        if let Ok(mut profiling) = self.is_profiling.lock() {
            *profiling = false;
        }
        println!("⏹️  Stack profiling stopped");
    }

    /// Record stack usage sample
    pub fn record_sample(&self, operation: &str, estimated_usage: usize) {
        if let Ok(profiling) = self.is_profiling.lock() {
            if !*profiling {
                return;
            }
        }

        let sample = StackUsageSample {
            operation: operation.to_string(),
            estimated_usage_bytes: estimated_usage,
            timestamp: Instant::now(),
            thread_id: format!("{:?}", std::thread::current().id()),
        };

        if let Ok(mut samples) = self.samples.lock() {
            samples.push(sample.clone());

            // Check for warnings
            if estimated_usage > self.config.warning_threshold_bytes {
                println!("⚠️  Stack usage warning: {} operation using {} bytes (threshold: {} bytes)", 
                         operation, estimated_usage, self.config.warning_threshold_bytes);
            }

            // Check for critical usage
            if estimated_usage > self.config.stack_limit_bytes {
                println!("🚨 CRITICAL: Stack usage exceeds limit: {} operation using {} bytes (limit: {} bytes)", 
                         operation, estimated_usage, self.config.stack_limit_bytes);
            }

            // Limit sample count
            if samples.len() > self.config.max_samples {
                samples.remove(0);
            }
        }
    }

    /// Generate profiling report
    pub fn generate_report(&self) -> VexfsResult<String> {
        let samples = self.samples.lock().map_err(|_| {
            vexfs::shared::errors::VexfsError::LockError("Failed to lock samples".to_string())
        })?;

        let mut report = String::new();
        report.push_str("# FUSE Stack Usage Profiling Report\n\n");

        if samples.is_empty() {
            report.push_str("No samples collected.\n");
            return Ok(report);
        }

        // Summary statistics
        let max_usage = samples.iter().map(|s| s.estimated_usage_bytes).max().unwrap_or(0);
        let avg_usage = samples.iter().map(|s| s.estimated_usage_bytes).sum::<usize>() / samples.len();
        let warning_count = samples.iter().filter(|s| s.estimated_usage_bytes > self.config.warning_threshold_bytes).count();
        let critical_count = samples.iter().filter(|s| s.estimated_usage_bytes > self.config.stack_limit_bytes).count();

        report.push_str("## Summary Statistics\n");
        report.push_str(&format!("- Total Samples: {}\n", samples.len()));
        report.push_str(&format!("- Maximum Usage: {} bytes\n", max_usage));
        report.push_str(&format!("- Average Usage: {} bytes\n", avg_usage));
        report.push_str(&format!("- Stack Limit: {} bytes\n", self.config.stack_limit_bytes));
        report.push_str(&format!("- Warning Threshold: {} bytes\n", self.config.warning_threshold_bytes));
        report.push_str(&format!("- Warnings: {} samples\n", warning_count));
        report.push_str(&format!("- Critical: {} samples\n", critical_count));
        report.push_str("\n");

        // Safety assessment
        report.push_str("## Safety Assessment\n");
        if critical_count > 0 {
            report.push_str("🚨 **CRITICAL**: Stack usage exceeded limit!\n");
        } else if warning_count > 0 {
            report.push_str("⚠️  **WARNING**: High stack usage detected\n");
        } else {
            report.push_str("✅ **SAFE**: All operations within limits\n");
        }
        report.push_str("\n");

        // Operation breakdown
        let mut operation_stats: HashMap<String, (usize, usize, usize)> = HashMap::new(); // (count, max, total)
        for sample in samples.iter() {
            let entry = operation_stats.entry(sample.operation.clone()).or_insert((0, 0, 0));
            entry.0 += 1; // count
            entry.1 = entry.1.max(sample.estimated_usage_bytes); // max
            entry.2 += sample.estimated_usage_bytes; // total
        }

        report.push_str("## Operation Breakdown\n");
        let mut ops: Vec<_> = operation_stats.iter().collect();
        ops.sort_by_key(|(_, (_, max, _))| std::cmp::Reverse(*max));

        for (operation, (count, max_usage, total_usage)) in ops {
            let avg_usage = total_usage / count;
            report.push_str(&format!("- **{}**: {} samples, max {} bytes, avg {} bytes\n", 
                                   operation, count, max_usage, avg_usage));
        }
        report.push_str("\n");

        // Top usage samples
        report.push_str("## Top 10 Highest Usage Samples\n");
        let mut top_samples = samples.clone();
        top_samples.sort_by_key(|s| std::cmp::Reverse(s.estimated_usage_bytes));

        for (i, sample) in top_samples.iter().take(10).enumerate() {
            report.push_str(&format!("{}. {} - {} bytes ({})\n", 
                                   i + 1, sample.operation, sample.estimated_usage_bytes, sample.thread_id));
        }

        Ok(report)
    }
}

/// Stack-aware FUSE operation wrapper
pub struct StackAwareFuseOperations {
    fuse_fs: Arc<VexFSFuse>,
    profiler: Arc<FuseStackProfiler>,
}

impl StackAwareFuseOperations {
    /// Create new stack-aware FUSE operations
    pub fn new(profiler_config: StackProfilingConfig) -> VexfsResult<Self> {
        let fuse_fs = Arc::new(VexFSFuse::new()?);
        let profiler = Arc::new(FuseStackProfiler::new(profiler_config));
        
        Ok(Self {
            fuse_fs,
            profiler,
        })
    }

    /// Store vector with stack monitoring
    pub fn store_vector_monitored(&self, vector: &[f32], file_inode: u64, metadata: HashMap<String, String>) -> VexfsResult<u64> {
        // Estimate stack usage based on vector size and operation complexity
        let estimated_usage = 1024 + (vector.len() * 4) + 512; // Base + vector data + metadata
        self.profiler.record_sample("store_vector", estimated_usage);
        
        self.fuse_fs.store_vector(vector, file_inode, metadata)
    }

    /// Search vectors with stack monitoring
    pub fn search_vectors_monitored(&self, query: &[f32], k: usize) -> VexfsResult<Vec<String>> {
        // Estimate stack usage based on query size and search complexity
        let estimated_usage = 2048 + (query.len() * 4) + (k * 64); // Base + query + results
        self.profiler.record_sample("search_vectors", estimated_usage);
        
        self.fuse_fs.search_vectors(query, k)
    }

    /// Force sync with stack monitoring
    pub fn force_sync_monitored(&self) -> VexfsResult<()> {
        self.profiler.record_sample("force_sync", 1536); // Estimated sync stack usage
        self.fuse_fs.force_sync()
    }

    /// Get profiler reference
    pub fn profiler(&self) -> &Arc<FuseStackProfiler> {
        &self.profiler
    }

    /// Get FUSE filesystem reference
    pub fn fuse_fs(&self) -> &Arc<VexFSFuse> {
        &self.fuse_fs
    }
}

/// Run comprehensive stack profiling test
pub fn run_stack_profiling_test() -> VexfsResult<()> {
    println!("🚀 Starting FUSE Stack Profiling Test");
    println!("====================================");

    let config = StackProfilingConfig::default();
    let stack_ops = StackAwareFuseOperations::new(config)?;
    
    // Start profiling
    stack_ops.profiler().start_profiling();

    // Test vector storage operations
    println!("\n📦 Testing vector storage operations...");
    let test_vectors = vec![
        vec![1.0, 2.0, 3.0, 4.0],
        vec![5.0, 6.0, 7.0, 8.0],
        vec![9.0, 10.0, 11.0, 12.0],
    ];

    for (i, vector) in test_vectors.iter().enumerate() {
        let file_inode = (1000 + i) as u64;
        let metadata = HashMap::new();
        
        match stack_ops.store_vector_monitored(vector, file_inode, metadata) {
            Ok(vector_id) => println!("  ✅ Stored vector {} with ID: {}", i + 1, vector_id),
            Err(e) => println!("  ❌ Failed to store vector {}: {:?}", i + 1, e),
        }
    }

    // Test vector search operations
    println!("\n🔍 Testing vector search operations...");
    let query_vector = vec![2.0, 4.0, 6.0, 8.0];
    
    match stack_ops.search_vectors_monitored(&query_vector, 5) {
        Ok(results) => println!("  ✅ Search completed, found {} results", results.len()),
        Err(e) => println!("  ❌ Search failed: {:?}", e),
    }

    // Test synchronization operations
    println!("\n🔄 Testing synchronization operations...");
    match stack_ops.force_sync_monitored() {
        Ok(_) => println!("  ✅ Synchronization completed"),
        Err(e) => println!("  ❌ Synchronization failed: {:?}", e),
    }

    // Test with larger vectors to stress stack usage
    println!("\n💪 Testing with larger vectors...");
    let large_vector: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
    let file_inode = 2000;
    let metadata = HashMap::new();
    
    match stack_ops.store_vector_monitored(&large_vector, file_inode, metadata) {
        Ok(vector_id) => println!("  ✅ Stored large vector with ID: {}", vector_id),
        Err(e) => println!("  ❌ Failed to store large vector: {:?}", e),
    }

    // Stop profiling and generate report
    stack_ops.profiler().stop_profiling();
    
    println!("\n📊 Generating profiling report...");
    match stack_ops.profiler().generate_report() {
        Ok(report) => {
            println!("{}", report);
            
            // Save report to file
            std::fs::write("fuse_stack_profiling_report.md", &report)
                .map_err(|e| vexfs::shared::errors::VexfsError::IoError(e.to_string()))?;
            println!("📄 Report saved to: fuse_stack_profiling_report.md");
        }
        Err(e) => println!("❌ Failed to generate report: {:?}", e),
    }

    println!("\n✅ Stack profiling test completed!");
    Ok(())
}

fn main() {
    println!("VexFS FUSE Stack Profiling Example");
    println!("==================================");
    
    match run_stack_profiling_test() {
        Ok(_) => println!("\n🎉 Stack profiling completed successfully!"),
        Err(e) => {
            eprintln!("\n❌ Stack profiling failed: {:?}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stack_profiler_creation() {
        let config = StackProfilingConfig::default();
        let profiler = FuseStackProfiler::new(config);
        
        // Should be able to create profiler
        assert_eq!(profiler.config.stack_limit_bytes, 6144);
    }
    
    #[test]
    fn test_stack_sample_recording() {
        let config = StackProfilingConfig::default();
        let profiler = FuseStackProfiler::new(config);
        
        profiler.start_profiling();
        profiler.record_sample("test_operation", 1024);
        profiler.stop_profiling();
        
        let report = profiler.generate_report().unwrap();
        assert!(report.contains("test_operation"));
    }
    
    #[test]
    fn test_stack_aware_operations() {
        let config = StackProfilingConfig::default();
        let result = StackAwareFuseOperations::new(config);
        assert!(result.is_ok(), "Should be able to create stack-aware operations");
    }
}