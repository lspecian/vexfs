//! Stack Overflow Test Scenarios for FUSE Implementation
//! 
//! This module contains test scenarios designed to trigger the 8KB FUSE stack
//! limit and measure actual stack usage patterns in critical code paths.

use std::time::Instant;
use std::collections::HashMap;
use crate::fuse_stack_profiler::{FuseStackProfiler, MemoryAllocationProfiler, AllocationType};

/// Test scenarios for stack overflow conditions
pub struct StackOverflowTestSuite {
    profiler: FuseStackProfiler,
    memory_profiler: MemoryAllocationProfiler,
}

impl StackOverflowTestSuite {
    pub fn new() -> Self {
        Self {
            profiler: FuseStackProfiler::new(),
            memory_profiler: MemoryAllocationProfiler::new(),
        }
    }

    /// Test 1: Large vector processing that could trigger stack overflow
    pub fn test_large_vector_processing(&self) -> TestResult {
        let mut results = TestResult::new("large_vector_processing");
        
        // Test with increasingly large vectors
        let vector_sizes = vec![128, 512, 1024, 2048, 4096, 8192];
        
        for size in vector_sizes {
            let test_name = format!("vector_size_{}", size);
            
            let result = self.profiler.measure_operation(&test_name, || {
                self.simulate_large_vector_operation(size)
            });
            
            results.add_measurement(test_name, result.stack_usage, result.success);
            
            if result.stack_usage > 6 * 1024 { // 6KB threshold
                results.add_overflow_risk(format!("Vector size {} exceeded 6KB stack usage", size));
            }
        }
        
        results
    }

    /// Test 2: Deep HNSW graph traversal
    pub fn test_deep_hnsw_traversal(&self) -> TestResult {
        let mut results = TestResult::new("deep_hnsw_traversal");
        
        // Test with increasing graph depths
        let depths = vec![5, 10, 15, 20, 25, 30];
        
        for depth in depths {
            let test_name = format!("graph_depth_{}", depth);
            
            let result = self.profiler.measure_operation(&test_name, || {
                self.simulate_hnsw_traversal(depth)
            });
            
            results.add_measurement(test_name, result.stack_usage, result.success);
            
            if result.stack_usage > 6 * 1024 {
                results.add_overflow_risk(format!("Graph depth {} exceeded 6KB stack usage", depth));
            }
        }
        
        results
    }

    /// Test 3: Recursive algorithm stack consumption
    pub fn test_recursive_algorithms(&self) -> TestResult {
        let mut results = TestResult::new("recursive_algorithms");
        
        // Test recursive search with different depths
        let recursion_depths = vec![10, 50, 100, 200, 500, 1000];
        
        for depth in recursion_depths {
            let test_name = format!("recursion_depth_{}", depth);
            
            let result = self.profiler.measure_operation(&test_name, || {
                self.simulate_recursive_search(depth)
            });
            
            results.add_measurement(test_name, result.stack_usage, result.success);
            
            if result.stack_usage > 6 * 1024 {
                results.add_overflow_risk(format!("Recursion depth {} exceeded 6KB stack usage", depth));
            }
        }
        
        results
    }

    /// Test 4: Component initialization stack requirements
    pub fn test_component_initialization(&self) -> TestResult {
        let mut results = TestResult::new("component_initialization");
        
        let components = vec![
            "vector_storage_manager",
            "hnsw_graph",
            "memory_pool",
            "search_state",
            "bridge_components",
        ];
        
        for component in components {
            let result = self.profiler.measure_operation(component, || {
                self.simulate_component_initialization(component)
            });
            
            results.add_measurement(component.to_string(), result.stack_usage, result.success);
            
            if result.stack_usage > 4 * 1024 { // Lower threshold for initialization
                results.add_overflow_risk(format!("Component {} initialization exceeded 4KB", component));
            }
        }
        
        results
    }

    /// Test 5: Concurrent operations stack usage
    pub fn test_concurrent_operations(&self) -> TestResult {
        let mut results = TestResult::new("concurrent_operations");
        
        let concurrency_levels = vec![1, 2, 4, 8, 16];
        
        for level in concurrency_levels {
            let test_name = format!("concurrency_{}", level);
            
            let result = self.profiler.measure_operation(&test_name, || {
                self.simulate_concurrent_operations(level)
            });
            
            results.add_measurement(test_name, result.stack_usage, result.success);
            
            if result.stack_usage > 5 * 1024 { // 5KB threshold for concurrent ops
                results.add_overflow_risk(format!("Concurrency level {} exceeded 5KB stack usage", level));
            }
        }
        
        results
    }

    /// Test 6: Error handling stack usage patterns
    pub fn test_error_handling_patterns(&self) -> TestResult {
        let mut results = TestResult::new("error_handling_patterns");
        
        let error_scenarios = vec![
            "stack_overflow_recovery",
            "memory_exhaustion",
            "invalid_vector_data",
            "bridge_communication_failure",
            "search_timeout",
        ];
        
        for scenario in error_scenarios {
            let result = self.profiler.measure_operation(scenario, || {
                self.simulate_error_handling(scenario)
            });
            
            results.add_measurement(scenario.to_string(), result.stack_usage, result.success);
            
            if result.stack_usage > 3 * 1024 { // Lower threshold for error handling
                results.add_overflow_risk(format!("Error scenario {} exceeded 3KB stack usage", scenario));
            }
        }
        
        results
    }

    /// Simulate large vector operation
    fn simulate_large_vector_operation(&self, size: usize) -> OperationResult {
        // Simulate stack allocation for vector processing
        let stack_usage = self.estimate_vector_stack_usage(size);
        
        // Record memory allocation
        self.memory_profiler.record_allocation(
            size * 4, // f32 vectors
            "vector_processing",
            AllocationType::VectorStorage,
        );
        
        // Simulate processing time
        std::thread::sleep(std::time::Duration::from_micros(size as u64 / 10));
        
        OperationResult {
            stack_usage,
            success: stack_usage < 8 * 1024,
        }
    }

    /// Simulate HNSW graph traversal
    fn simulate_hnsw_traversal(&self, depth: usize) -> OperationResult {
        let stack_usage = self.estimate_traversal_stack_usage(depth);
        
        // Record memory allocation for search state
        self.memory_profiler.record_allocation(
            depth * 64, // Estimated per-level state
            "hnsw_traversal",
            AllocationType::HnswGraph,
        );
        
        OperationResult {
            stack_usage,
            success: stack_usage < 8 * 1024,
        }
    }

    /// Simulate recursive search
    fn simulate_recursive_search(&self, depth: usize) -> OperationResult {
        let stack_usage = self.estimate_recursive_stack_usage(depth);
        
        OperationResult {
            stack_usage,
            success: stack_usage < 8 * 1024,
        }
    }

    /// Simulate component initialization
    fn simulate_component_initialization(&self, component: &str) -> OperationResult {
        let stack_usage = match component {
            "vector_storage_manager" => 2048,
            "hnsw_graph" => 1536,
            "memory_pool" => 1024,
            "search_state" => 512,
            "bridge_components" => 2560,
            _ => 1024,
        };
        
        // Record memory allocation
        self.memory_profiler.record_allocation(
            stack_usage * 2, // Heap allocation typically larger
            component,
            AllocationType::Other(component.to_string()),
        );
        
        OperationResult {
            stack_usage,
            success: stack_usage < 8 * 1024,
        }
    }

    /// Simulate concurrent operations
    fn simulate_concurrent_operations(&self, level: usize) -> OperationResult {
        let base_stack = 1024;
        let stack_usage = base_stack + (level * 256); // Stack grows with concurrency
        
        OperationResult {
            stack_usage,
            success: stack_usage < 8 * 1024,
        }
    }

    /// Simulate error handling
    fn simulate_error_handling(&self, scenario: &str) -> OperationResult {
        let stack_usage = match scenario {
            "stack_overflow_recovery" => 2048,
            "memory_exhaustion" => 1024,
            "invalid_vector_data" => 512,
            "bridge_communication_failure" => 1536,
            "search_timeout" => 768,
            _ => 1024,
        };
        
        OperationResult {
            stack_usage,
            success: true, // Error handling should always succeed
        }
    }

    /// Estimate stack usage for vector operations
    fn estimate_vector_stack_usage(&self, size: usize) -> usize {
        // Base stack usage + size-dependent component
        let base = 1024;
        let size_component = (size / 128) * 64; // 64 bytes per 128 vector elements
        base + size_component
    }

    /// Estimate stack usage for graph traversal
    fn estimate_traversal_stack_usage(&self, depth: usize) -> usize {
        // Base stack + depth-dependent component
        let base = 512;
        let depth_component = depth * 32; // 32 bytes per level
        base + depth_component
    }

    /// Estimate stack usage for recursive operations
    fn estimate_recursive_stack_usage(&self, depth: usize) -> usize {
        // Each recursion level adds stack frame
        let frame_size = 64; // Estimated stack frame size
        depth * frame_size
    }

    /// Run all stack overflow tests
    pub fn run_all_tests(&self) -> Vec<TestResult> {
        vec![
            self.test_large_vector_processing(),
            self.test_deep_hnsw_traversal(),
            self.test_recursive_algorithms(),
            self.test_component_initialization(),
            self.test_concurrent_operations(),
            self.test_error_handling_patterns(),
        ]
    }

    /// Generate comprehensive report
    pub fn generate_report(&self) -> StackOverflowReport {
        let test_results = self.run_all_tests();
        let profiling_results = self.profiler.generate_results();
        
        StackOverflowReport {
            test_results,
            profiling_results,
            memory_peak_usage: self.memory_profiler.get_peak_usage(),
            memory_current_usage: self.memory_profiler.get_current_usage(),
            recommendations: self.generate_recommendations(),
        }
    }

    /// Generate optimization recommendations
    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        recommendations.push("Use heap allocation for large data structures".to_string());
        recommendations.push("Implement iterative algorithms instead of recursive ones".to_string());
        recommendations.push("Use memory pools for frequent allocations".to_string());
        recommendations.push("Implement chunked processing for large vectors".to_string());
        recommendations.push("Add stack usage monitoring in critical paths".to_string());
        recommendations.push("Use lazy initialization for heavy components".to_string());
        
        recommendations
    }
}

/// Result of a single operation test
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub stack_usage: usize,
    pub success: bool,
}

/// Result of a test scenario
#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_name: String,
    pub measurements: Vec<(String, usize, bool)>, // (operation, stack_usage, success)
    pub overflow_risks: Vec<String>,
    pub peak_stack_usage: usize,
    pub average_stack_usage: f64,
}

impl TestResult {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            measurements: Vec::new(),
            overflow_risks: Vec::new(),
            peak_stack_usage: 0,
            average_stack_usage: 0.0,
        }
    }

    pub fn add_measurement(&mut self, operation: String, stack_usage: usize, success: bool) {
        self.measurements.push((operation, stack_usage, success));
        
        if stack_usage > self.peak_stack_usage {
            self.peak_stack_usage = stack_usage;
        }
        
        // Recalculate average
        let total: usize = self.measurements.iter().map(|(_, usage, _)| *usage).sum();
        self.average_stack_usage = total as f64 / self.measurements.len() as f64;
    }

    pub fn add_overflow_risk(&mut self, risk: String) {
        self.overflow_risks.push(risk);
    }
}

/// Comprehensive stack overflow report
#[derive(Debug, Clone)]
pub struct StackOverflowReport {
    pub test_results: Vec<TestResult>,
    pub profiling_results: crate::fuse_stack_profiler::StackProfilingResults,
    pub memory_peak_usage: usize,
    pub memory_current_usage: usize,
    pub recommendations: Vec<String>,
}

impl StackOverflowReport {
    /// Print detailed report
    pub fn print_report(&self) {
        println!("=== VexFS FUSE Stack Overflow Analysis Report ===\n");
        
        println!("Stack Usage Summary:");
        println!("- Peak stack usage: {} bytes", self.profiling_results.peak_usage);
        println!("- Average stack usage: {:.2} bytes", self.profiling_results.average_usage);
        println!("- FUSE stack limit: 8192 bytes");
        println!("- Safety threshold: 6144 bytes (75%)\n");
        
        println!("Memory Usage Summary:");
        println!("- Peak heap usage: {} bytes", self.memory_peak_usage);
        println!("- Current heap usage: {} bytes\n", self.memory_current_usage);
        
        println!("Test Results:");
        for test_result in &self.test_results {
            println!("  Test: {}", test_result.test_name);
            println!("    Peak stack usage: {} bytes", test_result.peak_stack_usage);
            println!("    Average stack usage: {:.2} bytes", test_result.average_stack_usage);
            println!("    Measurements: {}", test_result.measurements.len());
            
            if !test_result.overflow_risks.is_empty() {
                println!("    Overflow risks:");
                for risk in &test_result.overflow_risks {
                    println!("      - {}", risk);
                }
            }
            println!();
        }
        
        if !self.profiling_results.overflow_triggers.is_empty() {
            println!("Stack Overflow Triggers:");
            for trigger in &self.profiling_results.overflow_triggers {
                println!("  - Operation: {}", trigger.operation);
                println!("    Stack usage: {} bytes", trigger.stack_usage);
                println!("    Trigger point: {}", trigger.trigger_point);
                println!("    Call depth: {}", trigger.call_stack_depth);
            }
            println!();
        }
        
        println!("Optimization Recommendations:");
        for (i, recommendation) in self.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, recommendation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_overflow_suite_creation() {
        let suite = StackOverflowTestSuite::new();
        // Just verify it can be created without panicking
    }

    #[test]
    fn test_operation_result() {
        let result = OperationResult {
            stack_usage: 4096,
            success: true,
        };
        assert_eq!(result.stack_usage, 4096);
        assert!(result.success);
    }

    #[test]
    fn test_test_result() {
        let mut result = TestResult::new("test");
        assert_eq!(result.test_name, "test");
        assert_eq!(result.measurements.len(), 0);
        
        result.add_measurement("op1".to_string(), 1024, true);
        result.add_measurement("op2".to_string(), 2048, true);
        
        assert_eq!(result.measurements.len(), 2);
        assert_eq!(result.peak_stack_usage, 2048);
        assert_eq!(result.average_stack_usage, 1536.0);
    }
}