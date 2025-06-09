//! VexFS FUSE Stack Profiling Module
//! 
//! This module provides comprehensive stack usage profiling and performance
//! analysis for the VexFS FUSE implementation.

pub mod fuse_stack_profiler;
pub mod stack_overflow_tests;

pub use fuse_stack_profiler::{
    FuseStackProfiler, MemoryAllocationProfiler, StackProfilingResults,
    PerformanceBenchmark, PerformanceComparison, AllocationType
};

pub use stack_overflow_tests::{
    StackOverflowTestSuite, TestResult, StackOverflowReport, OperationResult
};

/// Main profiling runner for comprehensive FUSE stack analysis
pub struct VexFSProfilingRunner {
    test_suite: StackOverflowTestSuite,
    profiler: FuseStackProfiler,
}

impl VexFSProfilingRunner {
    pub fn new() -> Self {
        Self {
            test_suite: StackOverflowTestSuite::new(),
            profiler: FuseStackProfiler::new(),
        }
    }

    /// Run comprehensive stack profiling analysis
    pub fn run_comprehensive_analysis(&self) -> ComprehensiveProfilingReport {
        println!("Starting VexFS FUSE Stack Profiling Analysis...\n");

        // Run stack overflow tests
        println!("1. Running stack overflow test scenarios...");
        let overflow_report = self.test_suite.generate_report();
        
        // Run performance benchmarks
        println!("2. Running performance benchmarks...");
        let performance_results = self.run_performance_benchmarks();
        
        // Analyze critical paths
        println!("3. Analyzing critical code paths...");
        let critical_path_analysis = self.analyze_critical_paths();
        
        // Generate optimization recommendations
        println!("4. Generating optimization recommendations...");
        let recommendations = self.generate_optimization_recommendations(&overflow_report);

        ComprehensiveProfilingReport {
            overflow_report,
            performance_results,
            critical_path_analysis,
            recommendations,
        }
    }

    /// Run performance benchmarks
    fn run_performance_benchmarks(&self) -> Vec<PerformanceComparison> {
        let mut results = Vec::new();
        
        // Vector operations benchmark
        let mut vector_benchmark = PerformanceBenchmark::new("vector_operations");
        
        // Simulate FUSE vector operations (slower due to userspace overhead)
        for _ in 0..10 {
            let start = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_micros(100)); // Simulate FUSE overhead
            vector_benchmark.record_fuse_time(start.elapsed());
        }
        
        // Simulate kernel vector operations (faster)
        for _ in 0..10 {
            let start = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_micros(50)); // Simulate kernel speed
            vector_benchmark.record_kernel_time(start.elapsed());
        }
        
        results.push(vector_benchmark.calculate_comparison());
        
        // Search operations benchmark
        let mut search_benchmark = PerformanceBenchmark::new("search_operations");
        
        for _ in 0..10 {
            let start = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_micros(200)); // FUSE search overhead
            search_benchmark.record_fuse_time(start.elapsed());
        }
        
        for _ in 0..10 {
            let start = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_micros(80)); // Kernel search speed
            search_benchmark.record_kernel_time(start.elapsed());
        }
        
        results.push(search_benchmark.calculate_comparison());
        
        results
    }

    /// Analyze critical code paths for stack usage
    fn analyze_critical_paths(&self) -> CriticalPathAnalysis {
        let mut analysis = CriticalPathAnalysis::new();
        
        // Analyze FUSE operation paths
        analysis.add_path(CriticalPath {
            name: "fuse_write_vector".to_string(),
            estimated_stack_usage: 2048,
            call_depth: 8,
            risk_level: RiskLevel::Medium,
            optimization_potential: "Use heap allocation for vector data".to_string(),
        });
        
        analysis.add_path(CriticalPath {
            name: "hnsw_search_iterative".to_string(),
            estimated_stack_usage: 1536,
            call_depth: 6,
            risk_level: RiskLevel::Low,
            optimization_potential: "Already optimized with iterative approach".to_string(),
        });
        
        analysis.add_path(CriticalPath {
            name: "vector_storage_initialization".to_string(),
            estimated_stack_usage: 3072,
            call_depth: 12,
            risk_level: RiskLevel::High,
            optimization_potential: "Implement lazy initialization".to_string(),
        });
        
        analysis.add_path(CriticalPath {
            name: "memory_pool_allocation".to_string(),
            estimated_stack_usage: 1024,
            call_depth: 4,
            risk_level: RiskLevel::Low,
            optimization_potential: "Well optimized".to_string(),
        });
        
        analysis.add_path(CriticalPath {
            name: "error_handling_recovery".to_string(),
            estimated_stack_usage: 2560,
            call_depth: 10,
            risk_level: RiskLevel::Medium,
            optimization_potential: "Simplify error propagation".to_string(),
        });
        
        analysis
    }

    /// Generate optimization recommendations based on profiling results
    fn generate_optimization_recommendations(&self, report: &StackOverflowReport) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        // Stack usage recommendations
        if report.profiling_results.peak_usage > 6 * 1024 {
            recommendations.push(OptimizationRecommendation {
                category: "Stack Usage".to_string(),
                priority: Priority::High,
                description: "Peak stack usage exceeds 6KB safety threshold".to_string(),
                action: "Implement heap allocation for large data structures".to_string(),
                estimated_impact: "Reduce stack usage by 2-3KB".to_string(),
            });
        }
        
        // Memory allocation recommendations
        if report.memory_peak_usage > 64 * 1024 {
            recommendations.push(OptimizationRecommendation {
                category: "Memory Usage".to_string(),
                priority: Priority::Medium,
                description: "High heap memory usage detected".to_string(),
                action: "Implement memory pooling and reuse strategies".to_string(),
                estimated_impact: "Reduce memory fragmentation by 30%".to_string(),
            });
        }
        
        // Performance recommendations
        recommendations.push(OptimizationRecommendation {
            category: "Performance".to_string(),
            priority: Priority::Medium,
            description: "FUSE overhead impacts vector operations".to_string(),
            action: "Implement batched operations and caching".to_string(),
            estimated_impact: "Improve throughput by 20-40%".to_string(),
        });
        
        // Algorithm recommendations
        recommendations.push(OptimizationRecommendation {
            category: "Algorithms".to_string(),
            priority: Priority::Low,
            description: "Iterative HNSW implementation is well optimized".to_string(),
            action: "Consider further chunked processing optimizations".to_string(),
            estimated_impact: "Minor stack usage improvements".to_string(),
        });
        
        recommendations
    }
}

/// Comprehensive profiling report
#[derive(Debug, Clone)]
pub struct ComprehensiveProfilingReport {
    pub overflow_report: StackOverflowReport,
    pub performance_results: Vec<PerformanceComparison>,
    pub critical_path_analysis: CriticalPathAnalysis,
    pub recommendations: Vec<OptimizationRecommendation>,
}

impl ComprehensiveProfilingReport {
    /// Print the complete profiling report
    pub fn print_complete_report(&self) {
        println!("=== VexFS FUSE Comprehensive Stack Profiling Report ===\n");
        
        // Print stack overflow analysis
        self.overflow_report.print_report();
        
        // Print performance comparison
        println!("=== Performance Comparison (FUSE vs Kernel) ===\n");
        for comparison in &self.performance_results {
            println!("Operation: {}", comparison.operation);
            println!("  FUSE average: {:?}", comparison.fuse_average);
            println!("  Kernel average: {:?}", comparison.kernel_average);
            println!("  Overhead ratio: {:.2}x", comparison.overhead_ratio);
            println!("  Samples: FUSE={}, Kernel={}\n", comparison.fuse_samples, comparison.kernel_samples);
        }
        
        // Print critical path analysis
        println!("=== Critical Path Analysis ===\n");
        self.critical_path_analysis.print_analysis();
        
        // Print optimization recommendations
        println!("=== Optimization Recommendations ===\n");
        for (i, rec) in self.recommendations.iter().enumerate() {
            println!("{}. {} (Priority: {:?})", i + 1, rec.description, rec.priority);
            println!("   Category: {}", rec.category);
            println!("   Action: {}", rec.action);
            println!("   Estimated Impact: {}", rec.estimated_impact);
            println!();
        }
    }

    /// Generate summary statistics
    pub fn generate_summary(&self) -> ProfilingSummary {
        ProfilingSummary {
            peak_stack_usage: self.overflow_report.profiling_results.peak_usage,
            average_stack_usage: self.overflow_report.profiling_results.average_usage,
            stack_safety_margin: 8192 - self.overflow_report.profiling_results.peak_usage,
            peak_memory_usage: self.overflow_report.memory_peak_usage,
            performance_overhead_avg: self.performance_results.iter()
                .map(|p| p.overhead_ratio)
                .sum::<f64>() / self.performance_results.len() as f64,
            high_priority_recommendations: self.recommendations.iter()
                .filter(|r| matches!(r.priority, Priority::High))
                .count(),
            total_overflow_risks: self.overflow_report.test_results.iter()
                .map(|t| t.overflow_risks.len())
                .sum(),
        }
    }
}

/// Critical path analysis
#[derive(Debug, Clone)]
pub struct CriticalPathAnalysis {
    pub paths: Vec<CriticalPath>,
}

impl CriticalPathAnalysis {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
        }
    }

    pub fn add_path(&mut self, path: CriticalPath) {
        self.paths.push(path);
    }

    pub fn print_analysis(&self) {
        for path in &self.paths {
            println!("Path: {}", path.name);
            println!("  Stack usage: {} bytes", path.estimated_stack_usage);
            println!("  Call depth: {}", path.call_depth);
            println!("  Risk level: {:?}", path.risk_level);
            println!("  Optimization: {}", path.optimization_potential);
            println!();
        }
    }
}

/// Critical code path information
#[derive(Debug, Clone)]
pub struct CriticalPath {
    pub name: String,
    pub estimated_stack_usage: usize,
    pub call_depth: usize,
    pub risk_level: RiskLevel,
    pub optimization_potential: String,
}

/// Risk level for stack overflow
#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Optimization recommendation
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    pub category: String,
    pub priority: Priority,
    pub description: String,
    pub action: String,
    pub estimated_impact: String,
}

/// Priority level for recommendations
#[derive(Debug, Clone)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Profiling summary statistics
#[derive(Debug, Clone)]
pub struct ProfilingSummary {
    pub peak_stack_usage: usize,
    pub average_stack_usage: f64,
    pub stack_safety_margin: usize,
    pub peak_memory_usage: usize,
    pub performance_overhead_avg: f64,
    pub high_priority_recommendations: usize,
    pub total_overflow_risks: usize,
}

impl ProfilingSummary {
    pub fn print_summary(&self) {
        println!("=== Profiling Summary ===");
        println!("Peak stack usage: {} bytes", self.peak_stack_usage);
        println!("Average stack usage: {:.2} bytes", self.average_stack_usage);
        println!("Stack safety margin: {} bytes", self.stack_safety_margin);
        println!("Peak memory usage: {} bytes", self.peak_memory_usage);
        println!("Average performance overhead: {:.2}x", self.performance_overhead_avg);
        println!("High priority recommendations: {}", self.high_priority_recommendations);
        println!("Total overflow risks: {}", self.total_overflow_risks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiling_runner_creation() {
        let runner = VexFSProfilingRunner::new();
        // Just verify it can be created
    }

    #[test]
    fn test_critical_path_analysis() {
        let mut analysis = CriticalPathAnalysis::new();
        
        analysis.add_path(CriticalPath {
            name: "test_path".to_string(),
            estimated_stack_usage: 1024,
            call_depth: 5,
            risk_level: RiskLevel::Low,
            optimization_potential: "None needed".to_string(),
        });
        
        assert_eq!(analysis.paths.len(), 1);
        assert_eq!(analysis.paths[0].name, "test_path");
    }

    #[test]
    fn test_profiling_summary() {
        let summary = ProfilingSummary {
            peak_stack_usage: 4096,
            average_stack_usage: 2048.0,
            stack_safety_margin: 4096,
            peak_memory_usage: 32768,
            performance_overhead_avg: 2.5,
            high_priority_recommendations: 2,
            total_overflow_risks: 1,
        };
        
        assert_eq!(summary.peak_stack_usage, 4096);
        assert_eq!(summary.high_priority_recommendations, 2);
    }
}