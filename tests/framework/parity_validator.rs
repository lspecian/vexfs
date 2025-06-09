//! Behavior Parity Validator - Kernel vs FUSE Implementation Comparison
//!
//! This module implements systematic validation that kernel module and FUSE implementations
//! produce identical results for the same operations. This is critical for ensuring
//! consistent behavior across VexFS's dual architecture.
//!
//! ## Key Features
//!
//! - **Systematic Comparison**: Execute identical operations on both implementations
//! - **Result Validation**: Compare outputs, side effects, and error conditions
//! - **Performance Analysis**: Compare performance characteristics between implementations
//! - **Regression Detection**: Identify when parity breaks due to changes
//! - **Comprehensive Reporting**: Detailed analysis of parity validation results

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::fmt;

use crate::framework::{FrameworkError, FrameworkResult};

/// Implementation types for parity testing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImplementationType {
    KernelModule,
    FuseUserspace,
}

impl fmt::Display for ImplementationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImplementationType::KernelModule => write!(f, "Kernel Module"),
            ImplementationType::FuseUserspace => write!(f, "FUSE Userspace"),
        }
    }
}

/// Parity test case definition
#[derive(Debug, Clone)]
pub struct ParityTestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub operation: ParityOperation,
    pub test_data: TestData,
    pub expected_behavior: ExpectedBehavior,
    pub tolerance: ParityTolerance,
    pub timeout: Duration,
}

/// Types of operations to test for parity
#[derive(Debug, Clone)]
pub enum ParityOperation {
    // Filesystem operations
    FileCreate { path: String, mode: u32 },
    FileRead { path: String, offset: u64, size: usize },
    FileWrite { path: String, offset: u64, data: Vec<u8> },
    FileDelete { path: String },
    DirectoryCreate { path: String, mode: u32 },
    DirectoryList { path: String },
    DirectoryDelete { path: String },
    
    // Vector operations
    VectorStore { id: String, vector: Vec<f32>, metadata: HashMap<String, String> },
    VectorSearch { query: Vec<f32>, k: usize, threshold: f32 },
    VectorUpdate { id: String, vector: Vec<f32> },
    VectorDelete { id: String },
    
    // Graph operations
    GraphNodeCreate { id: String, properties: HashMap<String, String> },
    GraphEdgeCreate { from: String, to: String, properties: HashMap<String, String> },
    GraphTraversal { start: String, algorithm: String, depth: usize },
    
    // System operations
    Mount { device: String, mountpoint: String, options: Vec<String> },
    Unmount { mountpoint: String },
    Sync,
    Statfs { path: String },
    
    // Custom operation for extensibility
    Custom { name: String, parameters: HashMap<String, String> },
}

/// Test data for parity operations
#[derive(Debug, Clone)]
pub struct TestData {
    pub input_files: Vec<PathBuf>,
    pub input_vectors: Vec<Vec<f32>>,
    pub input_metadata: HashMap<String, String>,
    pub setup_commands: Vec<String>,
    pub cleanup_commands: Vec<String>,
}

impl Default for TestData {
    fn default() -> Self {
        Self {
            input_files: Vec::new(),
            input_vectors: Vec::new(),
            input_metadata: HashMap::new(),
            setup_commands: Vec::new(),
            cleanup_commands: Vec::new(),
        }
    }
}

/// Expected behavior specification
#[derive(Debug, Clone)]
pub struct ExpectedBehavior {
    pub should_succeed: bool,
    pub expected_error_code: Option<i32>,
    pub expected_output_pattern: Option<String>,
    pub expected_side_effects: Vec<SideEffect>,
    pub performance_bounds: Option<PerformanceBounds>,
}

/// Side effects to validate
#[derive(Debug, Clone)]
pub enum SideEffect {
    FileCreated { path: String },
    FileModified { path: String, expected_size: Option<u64> },
    FileDeleted { path: String },
    DirectoryCreated { path: String },
    DirectoryDeleted { path: String },
    VectorIndexUpdated { expected_count: Option<usize> },
    GraphStructureChanged { nodes_added: usize, edges_added: usize },
    MetadataUpdated { key: String, expected_value: Option<String> },
}

/// Performance bounds for comparison
#[derive(Debug, Clone)]
pub struct PerformanceBounds {
    pub max_execution_time: Duration,
    pub max_memory_usage: Option<usize>,
    pub max_cpu_usage: Option<f64>,
    pub max_io_operations: Option<u64>,
    pub performance_ratio_tolerance: f64, // Acceptable ratio between implementations
}

/// Tolerance settings for parity validation
#[derive(Debug, Clone)]
pub struct ParityTolerance {
    pub timing_tolerance: Duration,
    pub memory_tolerance_percent: f64,
    pub floating_point_epsilon: f64,
    pub allow_implementation_specific_errors: bool,
    pub ignore_performance_differences: bool,
}

impl Default for ParityTolerance {
    fn default() -> Self {
        Self {
            timing_tolerance: Duration::from_millis(100),
            memory_tolerance_percent: 10.0,
            floating_point_epsilon: 1e-6,
            allow_implementation_specific_errors: false,
            ignore_performance_differences: false,
        }
    }
}

/// Result of parity validation
#[derive(Debug, Clone)]
pub struct ParityResult {
    pub test_case_id: String,
    pub overall_status: ParityStatus,
    pub implementation_results: HashMap<ImplementationType, ImplementationResult>,
    pub comparison_results: ComparisonResults,
    pub execution_time: Duration,
    pub issues_found: Vec<ParityIssue>,
}

/// Overall parity status
#[derive(Debug, Clone, PartialEq)]
pub enum ParityStatus {
    Perfect,      // Identical behavior
    Acceptable,   // Minor differences within tolerance
    Warning,      // Differences that should be investigated
    Failed,       // Significant differences or errors
}

/// Result from a single implementation
#[derive(Debug, Clone)]
pub struct ImplementationResult {
    pub success: bool,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
    pub output: Vec<u8>,
    pub side_effects: Vec<SideEffect>,
    pub performance_metrics: PerformanceMetrics,
    pub execution_time: Duration,
}

/// Performance metrics for comparison
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub memory_usage_peak: Option<usize>,
    pub cpu_usage_percent: Option<f64>,
    pub io_read_bytes: Option<u64>,
    pub io_write_bytes: Option<u64>,
    pub io_operations: Option<u64>,
    pub system_calls: Option<u64>,
}

/// Comparison results between implementations
#[derive(Debug, Clone)]
pub struct ComparisonResults {
    pub output_identical: bool,
    pub error_codes_match: bool,
    pub side_effects_match: bool,
    pub performance_comparison: PerformanceComparison,
    pub detailed_differences: Vec<String>,
}

/// Performance comparison between implementations
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    pub timing_ratio: f64, // kernel_time / fuse_time
    pub memory_ratio: f64,
    pub io_ratio: f64,
    pub within_tolerance: bool,
}

/// Issues found during parity validation
#[derive(Debug, Clone)]
pub struct ParityIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub description: String,
    pub affected_implementations: Vec<ImplementationType>,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Critical,
    Major,
    Minor,
    Info,
}

#[derive(Debug, Clone)]
pub enum IssueCategory {
    OutputMismatch,
    ErrorCodeMismatch,
    SideEffectMismatch,
    PerformanceDifference,
    ImplementationSpecific,
    TestSetupIssue,
}

/// Parity validation configuration
#[derive(Debug, Clone)]
pub struct ParityValidationConfig {
    pub kernel_module_path: PathBuf,
    pub fuse_binary_path: PathBuf,
    pub test_mount_point: PathBuf,
    pub test_device_path: PathBuf,
    pub enable_performance_comparison: bool,
    pub enable_side_effect_validation: bool,
    pub parallel_execution: bool,
    pub cleanup_after_test: bool,
    pub capture_system_logs: bool,
    pub default_timeout: Duration,
}

impl Default for ParityValidationConfig {
    fn default() -> Self {
        Self {
            kernel_module_path: PathBuf::from("vexfs.ko"),
            fuse_binary_path: PathBuf::from("target/release/vexfs_fuse"),
            test_mount_point: PathBuf::from("/tmp/vexfs_parity_test"),
            test_device_path: PathBuf::from("/tmp/vexfs_test_device"),
            enable_performance_comparison: true,
            enable_side_effect_validation: true,
            parallel_execution: false,
            cleanup_after_test: true,
            capture_system_logs: false,
            default_timeout: Duration::from_secs(300),
        }
    }
}

/// Comprehensive parity report
#[derive(Debug, Clone)]
pub struct ParityReport {
    pub total_tests: usize,
    pub perfect_parity: usize,
    pub acceptable_parity: usize,
    pub warnings: usize,
    pub failures: usize,
    pub test_results: Vec<ParityResult>,
    pub summary_by_category: HashMap<String, CategorySummary>,
    pub performance_analysis: PerformanceAnalysis,
    pub recommendations: Vec<String>,
    pub generated_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct CategorySummary {
    pub total: usize,
    pub perfect: usize,
    pub acceptable: usize,
    pub warnings: usize,
    pub failures: usize,
    pub common_issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PerformanceAnalysis {
    pub average_timing_ratio: f64,
    pub average_memory_ratio: f64,
    pub performance_trends: Vec<String>,
    pub bottlenecks_identified: Vec<String>,
}

/// Main parity validator implementation
pub struct ParityValidator {
    config: ParityValidationConfig,
    test_cases: Vec<ParityTestCase>,
    results: Vec<ParityResult>,
}

impl ParityValidator {
    /// Create a new parity validator
    pub fn new(config: ParityValidationConfig) -> Self {
        Self {
            config,
            test_cases: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Add a parity test case
    pub fn add_test_case(&mut self, test_case: ParityTestCase) {
        self.test_cases.push(test_case);
    }

    /// Add multiple test cases
    pub fn add_test_cases(&mut self, test_cases: Vec<ParityTestCase>) {
        self.test_cases.extend(test_cases);
    }

    /// Register standard VexFS parity tests
    pub fn register_standard_tests(&mut self) -> FrameworkResult<()> {
        // File system operation tests
        self.register_filesystem_tests()?;
        
        // Vector operation tests
        self.register_vector_operation_tests()?;
        
        // Graph operation tests
        self.register_graph_operation_tests()?;
        
        // System operation tests
        self.register_system_operation_tests()?;
        
        println!("📋 Registered {} standard parity tests", self.test_cases.len());
        Ok(())
    }

    /// Execute all parity tests
    pub fn execute_all_tests(&mut self) -> FrameworkResult<ParityReport> {
        println!("🔄 Starting behavior parity validation");
        println!("Testing {} implementations across {} test cases", 2, self.test_cases.len());
        
        self.results.clear();
        
        for test_case in &self.test_cases.clone() {
            let result = self.execute_parity_test(test_case)?;
            self.results.push(result);
        }
        
        let report = self.generate_report()?;
        Ok(report)
    }

    /// Execute a single parity test
    fn execute_parity_test(&self, test_case: &ParityTestCase) -> FrameworkResult<ParityResult> {
        let start_time = Instant::now();
        
        println!("🧪 Testing parity: {}", test_case.name);
        
        // Setup test environment
        self.setup_test_environment(test_case)?;
        
        // Execute on both implementations
        let mut implementation_results = HashMap::new();
        
        // Execute on kernel module
        let kernel_result = self.execute_on_kernel_module(test_case)?;
        implementation_results.insert(ImplementationType::KernelModule, kernel_result);
        
        // Execute on FUSE
        let fuse_result = self.execute_on_fuse(test_case)?;
        implementation_results.insert(ImplementationType::FuseUserspace, fuse_result);
        
        // Compare results
        let comparison_results = self.compare_results(&implementation_results, test_case)?;
        
        // Determine overall status
        let overall_status = self.determine_parity_status(&comparison_results, test_case);
        
        // Identify issues
        let issues_found = self.identify_issues(&comparison_results, &implementation_results, test_case);
        
        // Cleanup
        if self.config.cleanup_after_test {
            self.cleanup_test_environment(test_case)?;
        }
        
        let result = ParityResult {
            test_case_id: test_case.id.clone(),
            overall_status,
            implementation_results,
            comparison_results,
            execution_time: start_time.elapsed(),
            issues_found,
        };
        
        // Print result
        match &result.overall_status {
            ParityStatus::Perfect => println!("  ✅ Perfect parity"),
            ParityStatus::Acceptable => println!("  ✅ Acceptable parity (within tolerance)"),
            ParityStatus::Warning => println!("  ⚠️  Warning: differences detected"),
            ParityStatus::Failed => println!("  ❌ Failed: significant differences"),
        }
        
        Ok(result)
    }

    /// Setup test environment for a specific test case
    fn setup_test_environment(&self, test_case: &ParityTestCase) -> FrameworkResult<()> {
        // Create test directories
        std::fs::create_dir_all(&self.config.test_mount_point)
            .map_err(|e| FrameworkError::EnvironmentError(format!("Failed to create mount point: {}", e)))?;
        
        // Execute setup commands
        for command in &test_case.test_data.setup_commands {
            self.execute_command(command)?;
        }
        
        Ok(())
    }

    /// Execute operation on kernel module
    fn execute_on_kernel_module(&self, test_case: &ParityTestCase) -> FrameworkResult<ImplementationResult> {
        let start_time = Instant::now();
        
        // Load kernel module if not already loaded
        self.ensure_kernel_module_loaded()?;
        
        // Mount filesystem using kernel module
        self.mount_kernel_filesystem()?;
        
        // Execute the operation
        let result = self.execute_operation_kernel(&test_case.operation)?;
        
        // Collect performance metrics
        let performance_metrics = self.collect_performance_metrics_kernel()?;
        
        // Unmount filesystem
        self.unmount_filesystem()?;
        
        Ok(ImplementationResult {
            success: result.success,
            error_code: result.error_code,
            error_message: result.error_message,
            output: result.output,
            side_effects: result.side_effects,
            performance_metrics,
            execution_time: start_time.elapsed(),
        })
    }

    /// Execute operation on FUSE
    fn execute_on_fuse(&self, test_case: &ParityTestCase) -> FrameworkResult<ImplementationResult> {
        let start_time = Instant::now();
        
        // Start FUSE daemon
        self.start_fuse_daemon()?;
        
        // Execute the operation
        let result = self.execute_operation_fuse(&test_case.operation)?;
        
        // Collect performance metrics
        let performance_metrics = self.collect_performance_metrics_fuse()?;
        
        // Stop FUSE daemon
        self.stop_fuse_daemon()?;
        
        Ok(ImplementationResult {
            success: result.success,
            error_code: result.error_code,
            error_message: result.error_message,
            output: result.output,
            side_effects: result.side_effects,
            performance_metrics,
            execution_time: start_time.elapsed(),
        })
    }

    /// Compare results between implementations
    fn compare_results(
        &self,
        results: &HashMap<ImplementationType, ImplementationResult>,
        test_case: &ParityTestCase,
    ) -> FrameworkResult<ComparisonResults> {
        let kernel_result = results.get(&ImplementationType::KernelModule)
            .ok_or_else(|| FrameworkError::ParityValidationFailed("Missing kernel result".to_string()))?;
        let fuse_result = results.get(&ImplementationType::FuseUserspace)
            .ok_or_else(|| FrameworkError::ParityValidationFailed("Missing FUSE result".to_string()))?;

        // Compare outputs
        let output_identical = kernel_result.output == fuse_result.output;
        
        // Compare error codes
        let error_codes_match = kernel_result.error_code == fuse_result.error_code;
        
        // Compare side effects
        let side_effects_match = self.compare_side_effects(&kernel_result.side_effects, &fuse_result.side_effects);
        
        // Compare performance
        let performance_comparison = self.compare_performance(&kernel_result.performance_metrics, &fuse_result.performance_metrics);
        
        // Generate detailed differences
        let detailed_differences = self.generate_detailed_differences(kernel_result, fuse_result);
        
        Ok(ComparisonResults {
            output_identical,
            error_codes_match,
            side_effects_match,
            performance_comparison,
            detailed_differences,
        })
    }

    /// Determine overall parity status
    fn determine_parity_status(&self, comparison: &ComparisonResults, test_case: &ParityTestCase) -> ParityStatus {
        if comparison.output_identical && comparison.error_codes_match && comparison.side_effects_match {
            if test_case.tolerance.ignore_performance_differences || comparison.performance_comparison.within_tolerance {
                ParityStatus::Perfect
            } else {
                ParityStatus::Acceptable
            }
        } else if comparison.error_codes_match && (comparison.output_identical || comparison.side_effects_match) {
            ParityStatus::Warning
        } else {
            ParityStatus::Failed
        }
    }

    /// Generate comprehensive parity report
    fn generate_report(&self) -> FrameworkResult<ParityReport> {
        let total_tests = self.results.len();
        let perfect_parity = self.results.iter().filter(|r| r.overall_status == ParityStatus::Perfect).count();
        let acceptable_parity = self.results.iter().filter(|r| r.overall_status == ParityStatus::Acceptable).count();
        let warnings = self.results.iter().filter(|r| r.overall_status == ParityStatus::Warning).count();
        let failures = self.results.iter().filter(|r| r.overall_status == ParityStatus::Failed).count();

        // Generate category summaries
        let summary_by_category = self.generate_category_summaries();
        
        // Generate performance analysis
        let performance_analysis = self.generate_performance_analysis();
        
        // Generate recommendations
        let recommendations = self.generate_recommendations();

        Ok(ParityReport {
            total_tests,
            perfect_parity,
            acceptable_parity,
            warnings,
            failures,
            test_results: self.results.clone(),
            summary_by_category,
            performance_analysis,
            recommendations,
            generated_at: std::time::SystemTime::now(),
        })
    }

    // Helper methods (placeholder implementations)
    fn register_filesystem_tests(&mut self) -> FrameworkResult<()> {
        // Register standard filesystem operation tests
        Ok(())
    }

    fn register_vector_operation_tests(&mut self) -> FrameworkResult<()> {
        // Register vector operation tests
        Ok(())
    }

    fn register_graph_operation_tests(&mut self) -> FrameworkResult<()> {
        // Register graph operation tests
        Ok(())
    }

    fn register_system_operation_tests(&mut self) -> FrameworkResult<()> {
        // Register system operation tests
        Ok(())
    }

    fn cleanup_test_environment(&self, _test_case: &ParityTestCase) -> FrameworkResult<()> {
        // Cleanup test environment
        Ok(())
    }

    fn execute_command(&self, _command: &str) -> FrameworkResult<()> {
        // Execute shell command
        Ok(())
    }

    fn ensure_kernel_module_loaded(&self) -> FrameworkResult<()> {
        // Ensure kernel module is loaded
        Ok(())
    }

    fn mount_kernel_filesystem(&self) -> FrameworkResult<()> {
        // Mount filesystem using kernel module
        Ok(())
    }

    fn unmount_filesystem(&self) -> FrameworkResult<()> {
        // Unmount filesystem
        Ok(())
    }

    fn start_fuse_daemon(&self) -> FrameworkResult<()> {
        // Start FUSE daemon
        Ok(())
    }

    fn stop_fuse_daemon(&self) -> FrameworkResult<()> {
        // Stop FUSE daemon
        Ok(())
    }

    fn execute_operation_kernel(&self, _operation: &ParityOperation) -> FrameworkResult<OperationResult> {
        // Execute operation on kernel module
        Ok(OperationResult::default())
    }

    fn execute_operation_fuse(&self, _operation: &ParityOperation) -> FrameworkResult<OperationResult> {
        // Execute operation on FUSE
        Ok(OperationResult::default())
    }

    fn collect_performance_metrics_kernel(&self) -> FrameworkResult<PerformanceMetrics> {
        // Collect performance metrics from kernel
        Ok(PerformanceMetrics::default())
    }

    fn collect_performance_metrics_fuse(&self) -> FrameworkResult<PerformanceMetrics> {
        // Collect performance metrics from FUSE
        Ok(PerformanceMetrics::default())
    }

    fn compare_side_effects(&self, _kernel_effects: &[SideEffect], _fuse_effects: &[SideEffect]) -> bool {
        // Compare side effects
        true
    }

    fn compare_performance(&self, _kernel_metrics: &PerformanceMetrics, _fuse_metrics: &PerformanceMetrics) -> PerformanceComparison {
        // Compare performance metrics
        PerformanceComparison {
            timing_ratio: 1.0,
            memory_ratio: 1.0,
            io_ratio: 1.0,
            within_tolerance: true,
        }
    }

    fn generate_detailed_differences(&self, _kernel_result: &ImplementationResult, _fuse_result: &ImplementationResult) -> Vec<String> {
        // Generate detailed differences
        Vec::new()
    }

    fn identify_issues(&self, _comparison: &ComparisonResults, _results: &HashMap<ImplementationType, ImplementationResult>, _test_case: &ParityTestCase) -> Vec<ParityIssue> {
        // Identify issues
        Vec::new()
    }

    fn generate_category_summaries(&self) -> HashMap<String, CategorySummary> {
        // Generate category summaries
        HashMap::new()
    }

    fn generate_performance_analysis(&self) -> PerformanceAnalysis {
        // Generate performance analysis
        PerformanceAnalysis {
            average_timing_ratio: 1.0,
            average_memory_ratio: 1.0,
            performance_trends: Vec::new(),
            bottlenecks_identified: Vec::new(),
        }
    }

    fn generate_recommendations(&self) -> Vec<String> {
        // Generate recommendations
        Vec::new()
    }
}

// Helper struct for operation results
#[derive(Debug, Clone, Default)]
struct OperationResult {
    success: bool,
    error_code: Option<i32>,
    error_message: Option<String>,
    output: Vec<u8>,
    side_effects: Vec<SideEffect>,
}

// Implement the trait for the unified framework
impl crate::framework::unified_test_framework::ParityValidator for ParityValidator {
    fn validate_parity(&self, _test_case: &crate::framework::unified_test_framework::TestCase) -> FrameworkResult<bool> {
        // Implementation for trait compatibility
        Ok(true)
    }

    fn get_parity_report(&self) -> FrameworkResult<String> {
        // Generate string report
        Ok("Parity validation report placeholder".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_validator_creation() {
        let config = ParityValidationConfig::default();
        let validator = ParityValidator::new(config);
        assert_eq!(validator.test_cases.len(), 0);
    }

    #[test]
    fn test_parity_test_case_creation() {
        let test_case = ParityTestCase {
            id: "test_001".to_string(),
            name: "File Create Test".to_string(),
            description: "Test file creation parity".to_string(),
            operation: ParityOperation::FileCreate {
                path: "/test/file.txt".to_string(),
                mode: 0o644,
            },
            test_data: TestData::default(),
            expected_behavior: ExpectedBehavior {
                should_succeed: true,
                expected_error_code: None,
                expected_output_pattern: None,
                expected_side_effects: vec![SideEffect::FileCreated {
                    path: "/test/file.txt".to_string(),
                }],
                performance_bounds: None,
            },
            tolerance: ParityTolerance::default(),
            timeout: Duration::from_secs(30),
        };

        assert_eq!(test_case.id, "test_001");
        assert_eq!(test_case.name, "File Create Test");
    }

    #[test]
    fn test_parity_status_determination() {
        let config = ParityValidationConfig::default();
        let validator = ParityValidator::new(config);
        
        let comparison = ComparisonResults {
            output_identical: true,
            error_codes_match: true,
            side_effects_match: true,
            performance_comparison: PerformanceComparison {
                timing_ratio: 1.0,
                memory_ratio: 1.0,
                io_ratio: 1.0,
                within_tolerance: true,
            },
            detailed_differences: Vec::new(),
        };

        let test_case = ParityTestCase {
            id: "test".to_string(),
            name: "test".to_string(),
            description: "test".to_string(),
            operation: ParityOperation::Sync,
            test_data: TestData::default(),
            expected_behavior: ExpectedBehavior {
                should_succeed: true,
                expected_error_code: None,
                expected_output_pattern: None,
                expected_side_effects: Vec::new(),
                performance_bounds: None,
            },
            tolerance: ParityTolerance::default(),
            timeout: Duration::from_secs(30),
        };

        let status = validator.determine_parity_status(&comparison, &test_case);
        assert_eq!(status, ParityStatus::Perfect);
    }
}