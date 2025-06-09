//! Task 23.7: Comprehensive Testing and Validation Framework
//!
//! This module implements a unified testing framework that consolidates all VexFS testing
//! capabilities and provides comprehensive validation for the complete AI-native semantic
//! computing platform achieved through Tasks 23.2-23.6.
//!
//! Key Features:
//! - Behavior parity validation between kernel module and FUSE implementations
//! - Real implementation testing (replacing placeholder tests)
//! - Performance benchmarking and regression testing
//! - Stress testing and reliability validation
//! - Security testing and vulnerability assessment
//! - Automated CI/CD pipeline integration
//! - Multi-environment deployment testing

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};
use std::path::{Path, PathBuf};
use std::fmt;
use std::thread;
use std::process::{Command, Stdio};
use std::fs;

use vexfs::shared::errors::{VexfsError, VexfsResult};
use vexfs::fuse_impl::VexFSFuse;
use vexfs::storage::StorageManager;
use vexfs::vector_search::VectorSearchEngine;
use vexfs::semantic_api::types::*;

/// Comprehensive test framework configuration
#[derive(Debug, Clone)]
pub struct Task23_7TestConfig {
    /// Enable behavior parity testing between kernel and FUSE
    pub enable_behavior_parity: bool,
    /// Enable real implementation testing (vs placeholder)
    pub enable_real_implementation: bool,
    /// Enable performance benchmarking
    pub enable_performance_testing: bool,
    /// Enable stress testing
    pub enable_stress_testing: bool,
    /// Enable security testing
    pub enable_security_testing: bool,
    /// Enable multi-environment testing
    pub enable_multi_environment: bool,
    /// Maximum test execution time
    pub max_test_duration: Duration,
    /// Number of parallel test threads
    pub max_parallel_threads: usize,
    /// Test data directory
    pub test_data_dir: PathBuf,
    /// Temporary test directory
    pub temp_dir: PathBuf,
    /// Enable verbose logging
    pub verbose: bool,
    /// Enable CI/CD mode (automated reporting)
    pub ci_cd_mode: bool,
}

impl Default for Task23_7TestConfig {
    fn default() -> Self {
        Self {
            enable_behavior_parity: true,
            enable_real_implementation: true,
            enable_performance_testing: true,
            enable_stress_testing: true,
            enable_security_testing: true,
            enable_multi_environment: true,
            max_test_duration: Duration::from_secs(3600), // 1 hour
            max_parallel_threads: 8,
            test_data_dir: PathBuf::from("tests/data"),
            temp_dir: PathBuf::from("/tmp/vexfs_test_23_7"),
            verbose: false,
            ci_cd_mode: false,
        }
    }
}

/// Test execution environment
#[derive(Debug, Clone, PartialEq)]
pub enum TestEnvironment {
    /// FUSE userspace implementation
    Fuse,
    /// Kernel module implementation
    Kernel,
    /// Both implementations (for parity testing)
    Both,
    /// Docker containerized environment
    Docker,
    /// QEMU virtual machine environment
    Qemu,
    /// Bare metal environment
    BareMetal,
}

/// Test category classification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TestCategory {
    // Core VexFS testing
    BehaviorParity,
    RealImplementation,
    PlatformTransformation,
    
    // Component-specific testing
    VectorStorage,
    GraphOperations,
    SemanticJournal,
    EventPropagation,
    
    // Implementation-specific testing
    KernelModule,
    FuseImplementation,
    CrossLayerIntegration,
    
    // Quality assurance testing
    Performance,
    Stress,
    Security,
    Reliability,
    
    // Compliance testing
    PosixCompliance,
    DataIntegrity,
    CrashRecovery,
    
    // Deployment testing
    MultiEnvironment,
    CiCdIntegration,
    ProductionReadiness,
}

impl fmt::Display for TestCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestCategory::BehaviorParity => write!(f, "Behavior Parity"),
            TestCategory::RealImplementation => write!(f, "Real Implementation"),
            TestCategory::PlatformTransformation => write!(f, "Platform Transformation"),
            TestCategory::VectorStorage => write!(f, "Vector Storage"),
            TestCategory::GraphOperations => write!(f, "Graph Operations"),
            TestCategory::SemanticJournal => write!(f, "Semantic Journal"),
            TestCategory::EventPropagation => write!(f, "Event Propagation"),
            TestCategory::KernelModule => write!(f, "Kernel Module"),
            TestCategory::FuseImplementation => write!(f, "FUSE Implementation"),
            TestCategory::CrossLayerIntegration => write!(f, "Cross-Layer Integration"),
            TestCategory::Performance => write!(f, "Performance"),
            TestCategory::Stress => write!(f, "Stress"),
            TestCategory::Security => write!(f, "Security"),
            TestCategory::Reliability => write!(f, "Reliability"),
            TestCategory::PosixCompliance => write!(f, "POSIX Compliance"),
            TestCategory::DataIntegrity => write!(f, "Data Integrity"),
            TestCategory::CrashRecovery => write!(f, "Crash Recovery"),
            TestCategory::MultiEnvironment => write!(f, "Multi-Environment"),
            TestCategory::CiCdIntegration => write!(f, "CI/CD Integration"),
            TestCategory::ProductionReadiness => write!(f, "Production Readiness"),
        }
    }
}

/// Test result status
#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    Passed,
    Failed { reason: String, details: Option<String> },
    Skipped { reason: String },
    Timeout,
    Error { error: String },
    ParityMismatch { kernel_result: String, fuse_result: String },
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestResult::Passed => write!(f, "PASSED"),
            TestResult::Failed { reason, details } => {
                if let Some(details) = details {
                    write!(f, "FAILED: {} ({})", reason, details)
                } else {
                    write!(f, "FAILED: {}", reason)
                }
            }
            TestResult::Skipped { reason } => write!(f, "SKIPPED: {}", reason),
            TestResult::Timeout => write!(f, "TIMEOUT"),
            TestResult::Error { error } => write!(f, "ERROR: {}", error),
            TestResult::ParityMismatch { kernel_result, fuse_result } => {
                write!(f, "PARITY_MISMATCH: Kernel={}, FUSE={}", kernel_result, fuse_result)
            }
        }
    }
}

/// Individual test case definition
#[derive(Debug, Clone)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: TestCategory,
    pub environment: TestEnvironment,
    pub timeout: Duration,
    pub dependencies: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub result: Option<TestResult>,
    pub execution_time: Option<Duration>,
    pub start_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
}

impl TestCase {
    pub fn new(id: &str, name: &str, description: &str, category: TestCategory, environment: TestEnvironment) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category,
            environment,
            timeout: Duration::from_secs(300), // 5 minutes default
            dependencies: Vec::new(),
            metadata: HashMap::new(),
            result: None,
            execution_time: None,
            start_time: None,
            end_time: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Test execution statistics
#[derive(Debug, Clone, Default)]
pub struct TestStatistics {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timeout: usize,
    pub errors: usize,
    pub parity_mismatches: usize,
    pub total_execution_time: Duration,
    pub average_execution_time: Duration,
    pub success_rate: f64,
    pub parity_success_rate: f64,
    pub performance_metrics: HashMap<String, f64>,
}

impl TestStatistics {
    pub fn calculate(&mut self, test_results: &[TestCase]) {
        self.total_tests = test_results.len();
        self.passed = 0;
        self.failed = 0;
        self.skipped = 0;
        self.timeout = 0;
        self.errors = 0;
        self.parity_mismatches = 0;

        let mut total_time = Duration::new(0, 0);
        let mut executed_tests = 0;

        for test in test_results {
            if let Some(ref result) = test.result {
                match result {
                    TestResult::Passed => self.passed += 1,
                    TestResult::Failed { .. } => self.failed += 1,
                    TestResult::Skipped { .. } => self.skipped += 1,
                    TestResult::Timeout => self.timeout += 1,
                    TestResult::Error { .. } => self.errors += 1,
                    TestResult::ParityMismatch { .. } => self.parity_mismatches += 1,
                }

                if let Some(exec_time) = test.execution_time {
                    total_time += exec_time;
                    executed_tests += 1;
                }
            }
        }

        self.total_execution_time = total_time;
        self.average_execution_time = if executed_tests > 0 {
            total_time / executed_tests as u32
        } else {
            Duration::new(0, 0)
        };

        self.success_rate = if self.total_tests > 0 {
            (self.passed as f64 / self.total_tests as f64) * 100.0
        } else {
            0.0
        };

        let parity_tests = test_results.iter()
            .filter(|t| t.category == TestCategory::BehaviorParity)
            .count();
        
        self.parity_success_rate = if parity_tests > 0 {
            let parity_passed = test_results.iter()
                .filter(|t| t.category == TestCategory::BehaviorParity && 
                         matches!(t.result, Some(TestResult::Passed)))
                .count();
            (parity_passed as f64 / parity_tests as f64) * 100.0
        } else {
            100.0 // No parity tests means 100% success
        };
    }
}

/// Main comprehensive testing framework
pub struct Task23_7TestFramework {
    config: Task23_7TestConfig,
    test_cases: Vec<TestCase>,
    test_registry: HashMap<String, TestCase>,
    execution_order: Vec<String>,
    statistics: TestStatistics,
    fuse_instance: Option<Arc<VexFSFuse>>,
    kernel_available: bool,
    test_data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl Task23_7TestFramework {
    /// Create a new comprehensive testing framework
    pub fn new(config: Task23_7TestConfig) -> VexfsResult<Self> {
        let mut framework = Self {
            config,
            test_cases: Vec::new(),
            test_registry: HashMap::new(),
            execution_order: Vec::new(),
            statistics: TestStatistics::default(),
            fuse_instance: None,
            kernel_available: false,
            test_data: Arc::new(RwLock::new(HashMap::new())),
        };

        framework.initialize()?;
        Ok(framework)
    }

    /// Initialize the testing framework
    fn initialize(&mut self) -> VexfsResult<()> {
        // Create test directories
        fs::create_dir_all(&self.config.test_data_dir)
            .map_err(|e| VexfsError::Other(format!("Failed to create test data dir: {}", e)))?;
        
        fs::create_dir_all(&self.config.temp_dir)
            .map_err(|e| VexfsError::Other(format!("Failed to create temp dir: {}", e)))?;

        // Initialize FUSE instance if enabled
        if self.config.enable_behavior_parity || self.config.enable_real_implementation {
            match VexFSFuse::new() {
                Ok(fuse) => {
                    self.fuse_instance = Some(Arc::new(fuse));
                    if self.config.verbose {
                        println!("✅ FUSE instance initialized successfully");
                    }
                }
                Err(e) => {
                    if self.config.verbose {
                        println!("⚠️  FUSE initialization failed: {:?}", e);
                    }
                }
            }
        }

        // Check kernel module availability
        self.kernel_available = self.check_kernel_module_availability();
        if self.config.verbose {
            println!("🔍 Kernel module available: {}", self.kernel_available);
        }

        // Generate test data
        self.generate_test_data()?;

        // Register all test cases
        self.register_all_tests();

        // Calculate execution order based on dependencies
        self.calculate_execution_order()?;

        Ok(())
    }

    /// Check if kernel module is available for testing
    fn check_kernel_module_availability(&self) -> bool {
        // Check if VexFS kernel module is loaded
        if let Ok(output) = Command::new("lsmod")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("vexfs");
        }

        // Check if module file exists
        if Path::new("/lib/modules").exists() {
            // Try to find vexfs.ko in current kernel modules
            if let Ok(output) = Command::new("find")
                .args(&["/lib/modules", "-name", "vexfs.ko"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return !stdout.trim().is_empty();
            }
        }

        false
    }

    /// Generate test data for various test scenarios
    fn generate_test_data(&mut self) -> VexfsResult<()> {
        let mut test_data = self.test_data.write().unwrap();

        // Generate vector test data
        let vector_data = (0..1000).map(|i| (i as f32).to_le_bytes()).flatten().collect::<Vec<u8>>();
        test_data.insert("test_vectors_1000_f32".to_string(), vector_data);

        // Generate small file data
        let small_file = b"Hello, VexFS! This is a small test file.".to_vec();
        test_data.insert("small_file".to_string(), small_file);

        // Generate medium file data (1MB)
        let medium_file = vec![0xAB; 1024 * 1024];
        test_data.insert("medium_file_1mb".to_string(), medium_file);

        // Generate large file data (10MB)
        let large_file = vec![0xCD; 10 * 1024 * 1024];
        test_data.insert("large_file_10mb".to_string(), large_file);

        // Generate metadata test data
        let metadata = serde_json::json!({
            "test_type": "comprehensive",
            "version": "23.7",
            "timestamp": SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        }).to_string().into_bytes();
        test_data.insert("metadata_json".to_string(), metadata);

        if self.config.verbose {
            println!("✅ Generated {} test data sets", test_data.len());
        }

        Ok(())
    }

    /// Register all test cases
    fn register_all_tests(&mut self) {
        // Register behavior parity tests
        if self.config.enable_behavior_parity {
            self.register_behavior_parity_tests();
        }

        // Register real implementation tests
        if self.config.enable_real_implementation {
            self.register_real_implementation_tests();
        }

        // Register platform transformation tests (Tasks 23.2-23.6)
        self.register_platform_transformation_tests();

        // Register performance tests
        if self.config.enable_performance_testing {
            self.register_performance_tests();
        }

        // Register stress tests
        if self.config.enable_stress_testing {
            self.register_stress_tests();
        }

        // Register security tests
        if self.config.enable_security_testing {
            self.register_security_tests();
        }

        // Register multi-environment tests
        if self.config.enable_multi_environment {
            self.register_multi_environment_tests();
        }

        if self.config.verbose {
            println!("✅ Registered {} test cases across {} categories", 
                self.test_cases.len(),
                self.test_cases.iter().map(|t| &t.category).collect::<std::collections::HashSet<_>>().len()
            );
        }
    }

    /// Register behavior parity tests between kernel and FUSE implementations
    fn register_behavior_parity_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "parity_001",
                "Basic File Operations Parity",
                "Verify identical behavior for basic file operations (create, read, write, delete)",
                TestCategory::BehaviorParity,
                TestEnvironment::Both
            ).with_timeout(Duration::from_secs(60)),

            TestCase::new(
                "parity_002", 
                "Vector Storage Operations Parity",
                "Verify identical behavior for vector storage and retrieval operations",
                TestCategory::BehaviorParity,
                TestEnvironment::Both
            ).with_timeout(Duration::from_secs(120)),

            TestCase::new(
                "parity_003",
                "Directory Operations Parity", 
                "Verify identical behavior for directory creation, listing, and deletion",
                TestCategory::BehaviorParity,
                TestEnvironment::Both
            ).with_timeout(Duration::from_secs(60)),

            TestCase::new(
                "parity_004",
                "Metadata Operations Parity",
                "Verify identical behavior for file metadata operations (stat, chmod, etc.)",
                TestCategory::BehaviorParity,
                TestEnvironment::Both
            ).with_timeout(Duration::from_secs(60)),

            TestCase::new(
                "parity_005",
                "Vector Search Parity",
                "Verify identical search results between kernel and FUSE implementations",
                TestCategory::BehaviorParity,
                TestEnvironment::Both
            ).with_timeout(Duration::from_secs(180)),
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Register real implementation tests (replacing placeholders)
    fn register_real_implementation_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "real_001",
                "Real Vector Storage Manager",
                "Test actual VectorStorageManager with real data persistence",
                TestCategory::RealImplementation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(120)),

            TestCase::new(
                "real_002",
                "Real HNSW Graph Construction",
                "Test actual HNSW graph construction and traversal with real vectors",
                TestCategory::RealImplementation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(300)),

            TestCase::new(
                "real_003",
                "Real Semantic Journal Operations",
                "Test actual semantic journal with real event logging and replay",
                TestCategory::RealImplementation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(180)),

            TestCase::new(
                "real_004",
                "Real Event Propagation System",
                "Test actual event propagation with real cross-layer communication",
                TestCategory::RealImplementation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(240)),

            TestCase::new(
                "real_005",
                "Real Storage Layer Integration",
                "Test actual storage layer with real block device operations",
                TestCategory::RealImplementation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(180)),
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Register platform transformation validation tests (Tasks 23.2-23.6)
    fn register_platform_transformation_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "transform_001",
                "Task 23.2 Vector Storage Validation",
                "Validate VectorStorageManager restoration and performance targets (110-185% above targets)",
                TestCategory::PlatformTransformation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(300))
            .with_metadata("task", "23.2")
            .with_metadata("target_performance", "110-185%"),

            TestCase::new(
                "transform_002",
                "Task 23.3 HNSW Graph Validation",
                "Validate HNSW graph traversal and advanced analytics (97.8% reliability)",
                TestCategory::PlatformTransformation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(400))
            .with_metadata("task", "23.3")
            .with_metadata("target_reliability", "97.8%"),

            TestCase::new(
                "transform_003",
                "Task 23.4 Semantic Journal Validation",
                "Validate userspace semantic journal (>1000 events/sec, kernel compatibility)",
                TestCategory::PlatformTransformation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(300))
            .with_metadata("task", "23.4")
            .with_metadata("target_throughput", "1000 events/sec"),

            TestCase::new(
                "transform_004",
                "Task 23.5 Graph Capabilities Validation",
                "Validate advanced graph capabilities (AI-native reasoning, 96.4% kernel parity)",
                TestCategory::PlatformTransformation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(500))
            .with_metadata("task", "23.5")
            .with_metadata("target_parity", "96.4%"),

            TestCase::new(
                "transform_005",
                "Task 23.6 Event Propagation Validation",
                "Validate event propagation system (387ns latency, 1.2M events/sec analytics)",
                TestCategory::PlatformTransformation,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(400))
            .with_metadata("task", "23.6")
            .with_metadata("target_latency", "387ns")
            .with_metadata("target_analytics", "1.2M events/sec"),
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Register performance benchmarking tests
    fn register_performance_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "perf_001",
                "Vector Storage Performance Benchmark",
                "Benchmark vector storage operations throughput and latency",
                TestCategory::Performance,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(600)),

            TestCase::new(
                "perf_002",
                "Graph Traversal Performance Benchmark",
                "Benchmark HNSW graph traversal performance",
                TestCategory::Performance,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(600)),

            TestCase::new(
                "perf_003",
                "Event Processing Performance Benchmark",
                "Benchmark event processing and propagation performance",
                TestCategory::Performance,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(600)),

            TestCase::new(
                "perf_004",
                "Cross-Layer Integration Performance",
                "Benchmark end-to-end cross-layer operation performance",
                TestCategory::Performance,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(900)),
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Register stress testing scenarios
    fn register_stress_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "stress_001",
                "High-Load Vector Operations",
                "Stress test with high-volume vector storage and search operations",
                TestCategory::Stress,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(1800)), // 30 minutes

            TestCase::new(
                "stress_002",
                "Concurrent Multi-User Access",
                "Stress test with multiple concurrent users accessing the filesystem",
                TestCategory::Stress,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(1200)), // 20 minutes

            TestCase::new(
                "stress_003",
                "Memory Pressure Testing",
                "Stress test under memory pressure conditions",
                TestCategory::Stress,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(900)), // 15 minutes

            TestCase::new(
                "stress_004",
                "Long-Running Stability Test",
                "Long-running stability test for extended operation periods",
                TestCategory::Stress,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(3600)), // 1 hour
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Register security testing scenarios
    fn register_security_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "security_001",
                "Access Control Validation",
                "Validate file and directory access control mechanisms",
                TestCategory::Security,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(300)),

            TestCase::new(
                "security_002",
                "Vector Data Security",
                "Validate security of vector data storage and access",
                TestCategory::Security,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(300)),

            TestCase::new(
                "security_003",
                "Privilege Escalation Testing",
                "Test for potential privilege escalation vulnerabilities",
                TestCategory::Security,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(600)),

            TestCase::new(
                "security_004",
                "Data Integrity Validation",
                "Validate data integrity under various attack scenarios",
                TestCategory::Security,
                TestEnvironment::Fuse
            ).with_timeout(Duration::from_secs(600)),
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Register multi-environment deployment tests
    fn register_multi_environment_tests(&mut self) {
        let tests = vec![
            TestCase::new(
                "env_001",
                "Docker Container Deployment",
                "Test VexFS deployment and operation in Docker containers",
                TestCategory::MultiEnvironment,
                TestEnvironment::Docker
            ).with_timeout(Duration::from_secs(600)),

            TestCase::new(
                "env_002",
                "QEMU Virtual Machine Testing",
                "Test VexFS kernel module in QEMU virtual machine environment",
                TestCategory::MultiEnvironment,
                TestEnvironment::Qemu
            ).with_timeout(Duration::from_secs(900)),

            TestCase::new(
                "env_003",
                "Cross-Platform Compatibility",
                "Test VexFS compatibility across different Linux distributions",
                TestCategory::MultiEnvironment,
                TestEnvironment::Both
            ).with_timeout(Duration::from_secs(1200)),
        ];

        for test in tests {
            self.add_test_case(test);
        }
    }

    /// Add a test case to the framework
    fn add_test_case(&mut self, test_case: TestCase) {
        let id = test_case.id.clone();
        self.test_registry.insert(id.clone(), test_case.clone());
        self.test_cases.push(test_case);
    }

    /// Calculate test execution order based on dependencies
    fn calculate_execution_order(&mut self) -> VexfsResult<()> {
        // Simple topological sort for dependency resolution
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        for test in &self.test_cases {
            if !visited.contains(&test.id) {
                self.visit_test(&test.id, &mut visited, &mut temp_visited, &mut order)?;
            }
        }

        self.execution_order = order;
        Ok(())
    }

    /// Recursive helper for topological sort
    fn visit_test(
        &self,
        test_id: &str,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) -> VexfsResult<()> {
        if temp_visited.contains(test_id) {
            return Err(VexfsError::Other(format!("Circular dependency detected involving test {}", test_id)));
        }

        if visited.contains(test_id) {
            return Ok(());
        }

        temp_visited.insert(test_id.to_string());

        if let Some(test) = self.test_registry.get(test_id) {
            for dep in &test.dependencies {
                self.visit_test(dep, visited, temp_visited, order)?;
            }
        }

        temp_visited.remove(test_id);
        visited.insert(test_id.to_string());
        order.push(test_id.to_string());

        Ok(())
    }