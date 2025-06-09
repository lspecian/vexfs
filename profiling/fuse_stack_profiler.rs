//! FUSE Stack Profiling Framework
//! 
//! This module provides comprehensive stack usage measurement and profiling
//! for VexFS FUSE operations to identify stack overflow triggers and optimize
//! stack usage patterns.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::thread;

/// Stack usage measurement point
#[derive(Debug, Clone)]
pub struct StackMeasurement {
    pub operation: String,
    pub stack_usage: usize,
    pub timestamp: Instant,
    pub thread_id: String,
    pub call_depth: usize,
}

/// Stack profiling results
#[derive(Debug, Clone)]
pub struct StackProfilingResults {
    pub measurements: Vec<StackMeasurement>,
    pub peak_usage: usize,
    pub average_usage: f64,
    pub operation_stats: HashMap<String, OperationStats>,
    pub overflow_triggers: Vec<StackOverflowTrigger>,
}

/// Statistics for a specific operation type
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub count: usize,
    pub min_stack: usize,
    pub max_stack: usize,
    pub avg_stack: f64,
    pub total_duration: Duration,
    pub avg_duration: Duration,
}

/// Stack overflow trigger information
#[derive(Debug, Clone)]
pub struct StackOverflowTrigger {
    pub operation: String,
    pub stack_usage: usize,
    pub trigger_point: String,
    pub call_stack_depth: usize,
    pub timestamp: Instant,
}

/// Stack profiler for FUSE operations
pub struct FuseStackProfiler {
    measurements: Arc<Mutex<Vec<StackMeasurement>>>,
    overflow_triggers: Arc<Mutex<Vec<StackOverflowTrigger>>>,
    stack_limit: usize,
    profiling_enabled: bool,
}

impl FuseStackProfiler {
    /// Create new stack profiler with 8KB FUSE limit
    pub fn new() -> Self {
        Self {
            measurements: Arc::new(Mutex::new(Vec::new())),
            overflow_triggers: Arc::new(Mutex::new(Vec::new())),
            stack_limit: 8 * 1024, // 8KB FUSE stack limit
            profiling_enabled: true,
        }
    }

    /// Create profiler with custom stack limit
    pub fn with_limit(stack_limit: usize) -> Self {
        Self {
            measurements: Arc::new(Mutex::new(Vec::new())),
            overflow_triggers: Arc::new(Mutex::new(Vec::new())),
            stack_limit,
            profiling_enabled: true,
        }
    }

    /// Measure stack usage for an operation
    pub fn measure_operation<F, R>(&self, operation: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        if !self.profiling_enabled {
            return f();
        }

        let start_time = Instant::now();
        let stack_usage = self.estimate_stack_usage();
        
        // Record measurement before operation
        self.record_measurement(StackMeasurement {
            operation: format!("{}_start", operation),
            stack_usage,
            timestamp: start_time,
            thread_id: format!("{:?}", thread::current().id()),
            call_depth: self.estimate_call_depth(),
        });

        // Check for potential overflow
        if stack_usage > self.stack_limit * 3 / 4 { // 75% threshold
            self.record_overflow_trigger(StackOverflowTrigger {
                operation: operation.to_string(),
                stack_usage,
                trigger_point: "operation_start".to_string(),
                call_stack_depth: self.estimate_call_depth(),
                timestamp: start_time,
            });
        }

        // Execute operation
        let result = f();

        // Record measurement after operation
        let end_time = Instant::now();
        let end_stack_usage = self.estimate_stack_usage();
        
        self.record_measurement(StackMeasurement {
            operation: format!("{}_end", operation),
            stack_usage: end_stack_usage,
            timestamp: end_time,
            thread_id: format!("{:?}", thread::current().id()),
            call_depth: self.estimate_call_depth(),
        });

        result
    }

    /// Estimate current stack usage (simplified approach)
    fn estimate_stack_usage(&self) -> usize {
        // This is a simplified estimation - in a real implementation,
        // we would use platform-specific methods to get actual stack usage
        let dummy_array = [0u8; 64]; // Small stack allocation to measure
        let stack_ptr = dummy_array.as_ptr() as usize;
        
        // Estimate based on stack pointer position
        // This is a rough approximation for demonstration
        let estimated_usage = 2048 + (stack_ptr % 4096); // Base + variation
        estimated_usage.min(self.stack_limit)
    }

    /// Estimate call depth (simplified)
    fn estimate_call_depth(&self) -> usize {
        // In a real implementation, this would analyze the call stack
        // For now, return a reasonable estimate
        10
    }

    /// Record a stack measurement
    fn record_measurement(&self, measurement: StackMeasurement) {
        if let Ok(mut measurements) = self.measurements.lock() {
            measurements.push(measurement);
        }
    }

    /// Record a stack overflow trigger
    fn record_overflow_trigger(&self, trigger: StackOverflowTrigger) {
        if let Ok(mut triggers) = self.overflow_triggers.lock() {
            triggers.push(trigger);
        }
    }

    /// Generate profiling results
    pub fn generate_results(&self) -> StackProfilingResults {
        let measurements = self.measurements.lock().unwrap().clone();
        let overflow_triggers = self.overflow_triggers.lock().unwrap().clone();

        let peak_usage = measurements.iter()
            .map(|m| m.stack_usage)
            .max()
            .unwrap_or(0);

        let average_usage = if measurements.is_empty() {
            0.0
        } else {
            measurements.iter().map(|m| m.stack_usage as f64).sum::<f64>() / measurements.len() as f64
        };

        let operation_stats = self.calculate_operation_stats(&measurements);

        StackProfilingResults {
            measurements,
            peak_usage,
            average_usage,
            operation_stats,
            overflow_triggers,
        }
    }

    /// Calculate statistics per operation type
    fn calculate_operation_stats(&self, measurements: &[StackMeasurement]) -> HashMap<String, OperationStats> {
        let mut stats = HashMap::new();
        let mut operation_groups: HashMap<String, Vec<&StackMeasurement>> = HashMap::new();

        // Group measurements by operation
        for measurement in measurements {
            let base_op = measurement.operation.split('_').next().unwrap_or(&measurement.operation);
            operation_groups.entry(base_op.to_string())
                .or_insert_with(Vec::new)
                .push(measurement);
        }

        // Calculate stats for each operation
        for (operation, group) in operation_groups {
            let stack_usages: Vec<usize> = group.iter().map(|m| m.stack_usage).collect();
            let min_stack = stack_usages.iter().copied().min().unwrap_or(0);
            let max_stack = stack_usages.iter().copied().max().unwrap_or(0);
            let avg_stack = if stack_usages.is_empty() {
                0.0
            } else {
                stack_usages.iter().sum::<usize>() as f64 / stack_usages.len() as f64
            };

            stats.insert(operation, OperationStats {
                count: group.len(),
                min_stack,
                max_stack,
                avg_stack,
                total_duration: Duration::from_millis(0), // Would calculate from timestamps
                avg_duration: Duration::from_millis(0),
            });
        }

        stats
    }

    /// Clear all measurements
    pub fn clear(&self) {
        if let Ok(mut measurements) = self.measurements.lock() {
            measurements.clear();
        }
        if let Ok(mut triggers) = self.overflow_triggers.lock() {
            triggers.clear();
        }
    }

    /// Enable/disable profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.profiling_enabled = enabled;
    }
}

/// Memory allocation profiler for heap usage patterns
pub struct MemoryAllocationProfiler {
    allocations: Arc<Mutex<Vec<AllocationEvent>>>,
    peak_heap_usage: Arc<Mutex<usize>>,
    current_heap_usage: Arc<Mutex<usize>>,
}

/// Memory allocation event
#[derive(Debug, Clone)]
pub struct AllocationEvent {
    pub size: usize,
    pub operation: String,
    pub timestamp: Instant,
    pub allocation_type: AllocationType,
}

/// Type of memory allocation
#[derive(Debug, Clone)]
pub enum AllocationType {
    VectorStorage,
    HnswGraph,
    SearchState,
    MemoryPool,
    Other(String),
}

impl MemoryAllocationProfiler {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(Mutex::new(Vec::new())),
            peak_heap_usage: Arc::new(Mutex::new(0)),
            current_heap_usage: Arc::new(Mutex::new(0)),
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, size: usize, operation: &str, alloc_type: AllocationType) {
        let event = AllocationEvent {
            size,
            operation: operation.to_string(),
            timestamp: Instant::now(),
            allocation_type: alloc_type,
        };

        if let Ok(mut allocations) = self.allocations.lock() {
            allocations.push(event);
        }

        // Update current usage
        if let Ok(mut current) = self.current_heap_usage.lock() {
            *current += size;
            
            // Update peak if necessary
            if let Ok(mut peak) = self.peak_heap_usage.lock() {
                if *current > *peak {
                    *peak = *current;
                }
            }
        }
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, size: usize) {
        if let Ok(mut current) = self.current_heap_usage.lock() {
            *current = current.saturating_sub(size);
        }
    }

    /// Get current heap usage
    pub fn get_current_usage(&self) -> usize {
        self.current_heap_usage.lock().unwrap_or_else(|_| std::sync::MutexGuard::map(
            self.current_heap_usage.lock().unwrap(),
            |usage| usage
        )).clone()
    }

    /// Get peak heap usage
    pub fn get_peak_usage(&self) -> usize {
        self.peak_heap_usage.lock().unwrap_or_else(|_| std::sync::MutexGuard::map(
            self.peak_heap_usage.lock().unwrap(),
            |usage| usage
        )).clone()
    }
}

/// Performance benchmark for FUSE vs kernel operations
pub struct PerformanceBenchmark {
    fuse_times: Vec<Duration>,
    kernel_times: Vec<Duration>,
    operation_name: String,
}

impl PerformanceBenchmark {
    pub fn new(operation_name: &str) -> Self {
        Self {
            fuse_times: Vec::new(),
            kernel_times: Vec::new(),
            operation_name: operation_name.to_string(),
        }
    }

    /// Record FUSE operation time
    pub fn record_fuse_time(&mut self, duration: Duration) {
        self.fuse_times.push(duration);
    }

    /// Record kernel operation time
    pub fn record_kernel_time(&mut self, duration: Duration) {
        self.kernel_times.push(duration);
    }

    /// Calculate performance comparison
    pub fn calculate_comparison(&self) -> PerformanceComparison {
        let fuse_avg = self.calculate_average(&self.fuse_times);
        let kernel_avg = self.calculate_average(&self.kernel_times);
        
        let overhead_ratio = if kernel_avg.as_nanos() > 0 {
            fuse_avg.as_nanos() as f64 / kernel_avg.as_nanos() as f64
        } else {
            1.0
        };

        PerformanceComparison {
            operation: self.operation_name.clone(),
            fuse_average: fuse_avg,
            kernel_average: kernel_avg,
            overhead_ratio,
            fuse_samples: self.fuse_times.len(),
            kernel_samples: self.kernel_times.len(),
        }
    }

    fn calculate_average(&self, times: &[Duration]) -> Duration {
        if times.is_empty() {
            Duration::from_nanos(0)
        } else {
            let total_nanos: u64 = times.iter().map(|d| d.as_nanos() as u64).sum();
            Duration::from_nanos(total_nanos / times.len() as u64)
        }
    }
}

/// Performance comparison results
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    pub operation: String,
    pub fuse_average: Duration,
    pub kernel_average: Duration,
    pub overhead_ratio: f64,
    pub fuse_samples: usize,
    pub kernel_samples: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_profiler_creation() {
        let profiler = FuseStackProfiler::new();
        assert_eq!(profiler.stack_limit, 8 * 1024);
    }

    #[test]
    fn test_stack_measurement() {
        let profiler = FuseStackProfiler::new();
        
        let result = profiler.measure_operation("test_op", || {
            42
        });
        
        assert_eq!(result, 42);
        
        let results = profiler.generate_results();
        assert!(!results.measurements.is_empty());
    }

    #[test]
    fn test_memory_profiler() {
        let profiler = MemoryAllocationProfiler::new();
        
        profiler.record_allocation(1024, "test", AllocationType::VectorStorage);
        assert_eq!(profiler.get_current_usage(), 1024);
        assert_eq!(profiler.get_peak_usage(), 1024);
        
        profiler.record_allocation(512, "test2", AllocationType::HnswGraph);
        assert_eq!(profiler.get_current_usage(), 1536);
        assert_eq!(profiler.get_peak_usage(), 1536);
        
        profiler.record_deallocation(512);
        assert_eq!(profiler.get_current_usage(), 1024);
        assert_eq!(profiler.get_peak_usage(), 1536); // Peak remains
    }

    #[test]
    fn test_performance_benchmark() {
        let mut benchmark = PerformanceBenchmark::new("vector_search");
        
        benchmark.record_fuse_time(Duration::from_millis(10));
        benchmark.record_fuse_time(Duration::from_millis(12));
        benchmark.record_kernel_time(Duration::from_millis(5));
        benchmark.record_kernel_time(Duration::from_millis(7));
        
        let comparison = benchmark.calculate_comparison();
        assert_eq!(comparison.operation, "vector_search");
        assert!(comparison.overhead_ratio > 1.0); // FUSE should be slower
    }
}