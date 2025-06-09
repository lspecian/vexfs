//! Test Discovery Engine - Automatic Test Registration and Discovery
//!
//! This module implements automatic discovery and registration of tests from existing
//! testing frameworks within VexFS. It scans the codebase for tests and integrates
//! them into the unified testing framework.
//!
//! ## Key Features
//!
//! - **Automatic Discovery**: Scans codebase for existing tests
//! - **Framework Integration**: Integrates tests from multiple testing frameworks
//! - **Metadata Extraction**: Extracts test metadata and categorization
//! - **Dependency Analysis**: Analyzes test dependencies and ordering
//! - **Dynamic Registration**: Registers discovered tests with the unified framework

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

use crate::framework::{FrameworkError, FrameworkResult};
use crate::framework::unified_test_framework::{TestCase, TestCategory, TestExecutionMode};

/// Test discovery configuration
#[derive(Debug, Clone)]
pub struct TestDiscoveryConfig {
    pub search_paths: Vec<PathBuf>,
    pub file_patterns: Vec<String>,
    pub framework_types: Vec<TestFrameworkType>,
    pub exclude_patterns: Vec<String>,
    pub max_depth: usize,
    pub follow_symlinks: bool,
}

impl Default for TestDiscoveryConfig {
    fn default() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("tests"),
                PathBuf::from("src"),
                PathBuf::from("examples"),
                PathBuf::from("benches"),
            ],
            file_patterns: vec![
                "*.rs".to_string(),
                "*test*.rs".to_string(),
                "*_test.rs".to_string(),
                "test_*.rs".to_string(),
            ],
            framework_types: vec![
                TestFrameworkType::RustTest,
                TestFrameworkType::Tokio,
                TestFrameworkType::Criterion,
                TestFrameworkType::Custom,
            ],
            exclude_patterns: vec![
                "target/*".to_string(),
                ".git/*".to_string(),
                "*.bak".to_string(),
            ],
            max_depth: 10,
            follow_symlinks: false,
        }
    }
}

/// Types of testing frameworks to discover
#[derive(Debug, Clone, PartialEq)]
pub enum TestFrameworkType {
    RustTest,      // Standard Rust #[test] functions
    Tokio,         // #[tokio::test] async tests
    Criterion,     // Criterion benchmarks
    Custom,        // Custom test frameworks
}

/// Discovered test information
#[derive(Debug, Clone)]
pub struct DiscoveredTest {
    pub id: String,
    pub name: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub framework_type: TestFrameworkType,
    pub test_function: String,
    pub attributes: Vec<TestAttribute>,
    pub dependencies: Vec<String>,
    pub category: TestCategory,
    pub estimated_duration: Option<std::time::Duration>,
    pub resource_requirements: ResourceRequirements,
}

/// Test attributes extracted from source code
#[derive(Debug, Clone)]
pub enum TestAttribute {
    Ignore,
    ShouldPanic { expected: Option<String> },
    Timeout { duration: std::time::Duration },
    Category { name: String },
    Tags { tags: Vec<String> },
    Requires { resources: Vec<String> },
    Custom { name: String, value: String },
}

/// Resource requirements for tests
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub memory_mb: Option<usize>,
    pub disk_space_mb: Option<usize>,
    pub network_access: bool,
    pub filesystem_access: bool,
    pub kernel_module_access: bool,
    pub root_privileges: bool,
    pub external_dependencies: Vec<String>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            memory_mb: None,
            disk_space_mb: None,
            network_access: false,
            filesystem_access: false,
            kernel_module_access: false,
            root_privileges: false,
            external_dependencies: Vec::new(),
        }
    }
}

/// Test discovery statistics
#[derive(Debug, Clone)]
pub struct DiscoveryStatistics {
    pub files_scanned: usize,
    pub tests_discovered: usize,
    pub tests_by_framework: HashMap<TestFrameworkType, usize>,
    pub tests_by_category: HashMap<TestCategory, usize>,
    pub discovery_duration: std::time::Duration,
    pub errors_encountered: Vec<DiscoveryError>,
}

/// Discovery error information
#[derive(Debug, Clone)]
pub struct DiscoveryError {
    pub file_path: PathBuf,
    pub line_number: Option<usize>,
    pub error_type: DiscoveryErrorType,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum DiscoveryErrorType {
    ParseError,
    FileAccessError,
    InvalidTestFunction,
    UnsupportedFramework,
    MissingDependency,
}

/// Test discovery engine
pub struct TestDiscoveryEngine {
    config: TestDiscoveryConfig,
    discovered_tests: Vec<DiscoveredTest>,
    statistics: DiscoveryStatistics,
}

impl TestDiscoveryEngine {
    /// Create a new test discovery engine
    pub fn new(config: TestDiscoveryConfig) -> Self {
        Self {
            config,
            discovered_tests: Vec::new(),
            statistics: DiscoveryStatistics {
                files_scanned: 0,
                tests_discovered: 0,
                tests_by_framework: HashMap::new(),
                tests_by_category: HashMap::new(),
                discovery_duration: std::time::Duration::from_secs(0),
                errors_encountered: Vec::new(),
            },
        }
    }

    /// Discover all tests in the configured search paths
    pub fn discover_tests(&mut self) -> FrameworkResult<Vec<DiscoveredTest>> {
        let start_time = std::time::Instant::now();
        
        println!("🔍 Starting test discovery");
        println!("Scanning {} search paths", self.config.search_paths.len());
        
        self.discovered_tests.clear();
        self.statistics = DiscoveryStatistics {
            files_scanned: 0,
            tests_discovered: 0,
            tests_by_framework: HashMap::new(),
            tests_by_category: HashMap::new(),
            discovery_duration: std::time::Duration::from_secs(0),
            errors_encountered: Vec::new(),
        };
        
        for search_path in &self.config.search_paths {
            if search_path.exists() {
                self.scan_directory(search_path, 0)?;
            } else {
                println!("⚠️  Search path does not exist: {}", search_path.display());
            }
        }
        
        self.statistics.discovery_duration = start_time.elapsed();
        self.statistics.tests_discovered = self.discovered_tests.len();
        
        // Update statistics
        for test in &self.discovered_tests {
            *self.statistics.tests_by_framework.entry(test.framework_type.clone()).or_insert(0) += 1;
            *self.statistics.tests_by_category.entry(test.category.clone()).or_insert(0) += 1;
        }
        
        println!("✅ Test discovery completed");
        println!("  Files scanned: {}", self.statistics.files_scanned);
        println!("  Tests discovered: {}", self.statistics.tests_discovered);
        println!("  Discovery time: {:.2}s", self.statistics.discovery_duration.as_secs_f64());
        
        Ok(self.discovered_tests.clone())
    }

    /// Scan a directory for test files
    fn scan_directory(&mut self, dir_path: &Path, depth: usize) -> FrameworkResult<()> {
        if depth > self.config.max_depth {
            return Ok(());
        }
        
        let entries = fs::read_dir(dir_path)
            .map_err(|e| FrameworkError::DiscoveryError(format!("Failed to read directory {}: {}", dir_path.display(), e)))?;
        
        for entry in entries {
            let entry = entry
                .map_err(|e| FrameworkError::DiscoveryError(format!("Failed to read directory entry: {}", e)))?;
            
            let path = entry.path();
            
            // Skip excluded patterns
            if self.should_exclude(&path) {
                continue;
            }
            
            if path.is_dir() {
                if self.config.follow_symlinks || !path.is_symlink() {
                    self.scan_directory(&path, depth + 1)?;
                }
            } else if path.is_file() && self.matches_file_pattern(&path) {
                self.scan_file(&path)?;
            }
        }
        
        Ok(())
    }

    /// Scan a single file for tests
    fn scan_file(&mut self, file_path: &Path) -> FrameworkResult<()> {
        self.statistics.files_scanned += 1;
        
        let content = fs::read_to_string(file_path)
            .map_err(|e| {
                let error = DiscoveryError {
                    file_path: file_path.to_path_buf(),
                    line_number: None,
                    error_type: DiscoveryErrorType::FileAccessError,
                    message: format!("Failed to read file: {}", e),
                };
                self.statistics.errors_encountered.push(error);
                FrameworkError::DiscoveryError(format!("Failed to read file {}: {}", file_path.display(), e))
            })?;
        
        // Parse the file content for tests
        self.parse_rust_tests(file_path, &content)?;
        
        Ok(())
    }

    /// Parse Rust test functions from file content
    fn parse_rust_tests(&mut self, file_path: &Path, content: &str) -> FrameworkResult<()> {
        let lines: Vec<&str> = content.lines().collect();
        
        for (line_number, line) in lines.iter().enumerate() {
            let line_num = line_number + 1;
            
            // Look for test attributes
            if line.trim().starts_with("#[") {
                if let Some(test) = self.parse_test_function(file_path, &lines, line_num)? {
                    self.discovered_tests.push(test);
                }
            }
        }
        
        Ok(())
    }

    /// Parse a test function starting at the given line
    fn parse_test_function(&self, file_path: &Path, lines: &[&str], start_line: usize) -> FrameworkResult<Option<DiscoveredTest>> {
        let mut attributes = Vec::new();
        let mut current_line = start_line - 1;
        
        // Parse attributes
        while current_line < lines.len() && lines[current_line].trim().starts_with("#[") {
            if let Some(attr) = self.parse_test_attribute(lines[current_line])? {
                attributes.push(attr);
            }
            current_line += 1;
        }
        
        // Check if this is a test function
        if !self.is_test_function(&attributes) {
            return Ok(None);
        }
        
        // Find the function declaration
        while current_line < lines.len() && !lines[current_line].trim().starts_with("fn ") {
            current_line += 1;
        }
        
        if current_line >= lines.len() {
            return Ok(None);
        }
        
        let function_line = lines[current_line];
        let function_name = self.extract_function_name(function_line)?;
        
        // Determine framework type
        let framework_type = self.determine_framework_type(&attributes);
        
        // Determine test category
        let category = self.determine_test_category(file_path, &function_name, &attributes);
        
        // Extract dependencies
        let dependencies = self.extract_dependencies(&attributes);
        
        // Estimate duration
        let estimated_duration = self.estimate_test_duration(&attributes, &category);
        
        // Determine resource requirements
        let resource_requirements = self.determine_resource_requirements(file_path, &function_name, &attributes);
        
        let test = DiscoveredTest {
            id: format!("{}::{}", file_path.display(), function_name),
            name: function_name.clone(),
            file_path: file_path.to_path_buf(),
            line_number: current_line + 1,
            framework_type,
            test_function: function_name,
            attributes,
            dependencies,
            category,
            estimated_duration,
            resource_requirements,
        };
        
        Ok(Some(test))
    }

    /// Parse a test attribute from a line
    fn parse_test_attribute(&self, line: &str) -> FrameworkResult<Option<TestAttribute>> {
        let line = line.trim();
        
        if line == "#[test]" {
            return Ok(None); // Standard test marker, not a special attribute
        }
        
        if line == "#[ignore]" {
            return Ok(Some(TestAttribute::Ignore));
        }
        
        if line.starts_with("#[should_panic") {
            let expected = if line.contains("expected") {
                // Extract expected message
                Some("panic message".to_string()) // Simplified extraction
            } else {
                None
            };
            return Ok(Some(TestAttribute::ShouldPanic { expected }));
        }
        
        if line.starts_with("#[timeout") {
            // Extract timeout duration
            let duration = std::time::Duration::from_secs(30); // Default timeout
            return Ok(Some(TestAttribute::Timeout { duration }));
        }
        
        // Custom attributes
        if line.starts_with("#[category") {
            return Ok(Some(TestAttribute::Category { name: "custom".to_string() }));
        }
        
        Ok(None)
    }

    /// Check if the attributes indicate this is a test function
    fn is_test_function(&self, attributes: &[TestAttribute]) -> bool {
        // Check for standard test frameworks
        for framework_type in &self.config.framework_types {
            match framework_type {
                TestFrameworkType::RustTest => {
                    // Look for #[test] in the source (simplified check)
                    return true; // Simplified for now
                }
                TestFrameworkType::Tokio => {
                    // Look for #[tokio::test]
                    return true; // Simplified for now
                }
                TestFrameworkType::Criterion => {
                    // Look for criterion benchmarks
                    return false; // Not implemented yet
                }
                TestFrameworkType::Custom => {
                    // Look for custom test markers
                    return false; // Not implemented yet
                }
            }
        }
        
        false
    }

    /// Extract function name from function declaration
    fn extract_function_name(&self, function_line: &str) -> FrameworkResult<String> {
        let line = function_line.trim();
        if let Some(start) = line.find("fn ") {
            let after_fn = &line[start + 3..];
            if let Some(end) = after_fn.find('(') {
                let name = after_fn[..end].trim();
                return Ok(name.to_string());
            }
        }
        
        Err(FrameworkError::DiscoveryError("Failed to extract function name".to_string()))
    }

    /// Determine the framework type from attributes
    fn determine_framework_type(&self, _attributes: &[TestAttribute]) -> TestFrameworkType {
        // Simplified determination
        TestFrameworkType::RustTest
    }

    /// Determine test category based on file path and function name
    fn determine_test_category(&self, file_path: &Path, function_name: &str, _attributes: &[TestAttribute]) -> TestCategory {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let name_lower = function_name.to_lowercase();
        
        // Categorize based on path
        if path_str.contains("integration") {
            return TestCategory::Integration;
        }
        
        if path_str.contains("performance") || path_str.contains("bench") {
            return TestCategory::Performance;
        }
        
        if path_str.contains("stress") {
            return TestCategory::StressTest;
        }
        
        // Categorize based on function name
        if name_lower.contains("integration") {
            return TestCategory::Integration;
        }
        
        if name_lower.contains("performance") || name_lower.contains("bench") {
            return TestCategory::Performance;
        }
        
        if name_lower.contains("parity") {
            return TestCategory::BehaviorParity;
        }
        
        if name_lower.contains("real") || name_lower.contains("actual") {
            return TestCategory::RealImplementation;
        }
        
        if name_lower.contains("transform") {
            return TestCategory::PlatformTransformation;
        }
        
        // Default to unit test
        TestCategory::Unit
    }

    /// Extract test dependencies from attributes
    fn extract_dependencies(&self, _attributes: &[TestAttribute]) -> Vec<String> {
        // Simplified dependency extraction
        Vec::new()
    }

    /// Estimate test duration based on category and attributes
    fn estimate_test_duration(&self, _attributes: &[TestAttribute], category: &TestCategory) -> Option<std::time::Duration> {
        match category {
            TestCategory::Unit => Some(std::time::Duration::from_millis(100)),
            TestCategory::Integration => Some(std::time::Duration::from_secs(5)),
            TestCategory::Performance => Some(std::time::Duration::from_secs(30)),
            TestCategory::StressTest => Some(std::time::Duration::from_secs(60)),
            TestCategory::BehaviorParity => Some(std::time::Duration::from_secs(10)),
            TestCategory::RealImplementation => Some(std::time::Duration::from_secs(15)),
            TestCategory::PlatformTransformation => Some(std::time::Duration::from_secs(20)),
        }
    }

    /// Determine resource requirements for a test
    fn determine_resource_requirements(&self, file_path: &Path, function_name: &str, _attributes: &[TestAttribute]) -> ResourceRequirements {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let name_lower = function_name.to_lowercase();
        
        let mut requirements = ResourceRequirements::default();
        
        // Check for filesystem access
        if path_str.contains("fuse") || name_lower.contains("filesystem") || name_lower.contains("mount") {
            requirements.filesystem_access = true;
        }
        
        // Check for kernel module access
        if path_str.contains("kernel") || name_lower.contains("kernel") || name_lower.contains("module") {
            requirements.kernel_module_access = true;
            requirements.root_privileges = true;
        }
        
        // Check for network access
        if name_lower.contains("network") || name_lower.contains("socket") || name_lower.contains("http") {
            requirements.network_access = true;
        }
        
        // Estimate memory requirements based on test type
        if name_lower.contains("stress") || name_lower.contains("large") {
            requirements.memory_mb = Some(1024); // 1GB for stress tests
        } else if name_lower.contains("performance") {
            requirements.memory_mb = Some(512); // 512MB for performance tests
        }
        
        requirements
    }

    /// Check if a path should be excluded
    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.config.exclude_patterns {
            if path_str.contains(pattern.trim_end_matches("*")) {
                return true;
            }
        }
        
        false
    }

    /// Check if a file matches the configured patterns
    fn matches_file_pattern(&self, path: &Path) -> bool {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        for pattern in &self.config.file_patterns {
            if pattern == "*.rs" && file_name.ends_with(".rs") {
                return true;
            }
            
            if pattern.starts_with("*") && pattern.ends_with("*") {
                let middle = &pattern[1..pattern.len()-1];
                if file_name.contains(middle) {
                    return true;
                }
            }
            
            if pattern.starts_with("*") {
                let suffix = &pattern[1..];
                if file_name.ends_with(suffix) {
                    return true;
                }
            }
            
            if pattern.ends_with("*") {
                let prefix = &pattern[..pattern.len()-1];
                if file_name.starts_with(prefix) {
                    return true;
                }
            }
            
            if file_name == pattern {
                return true;
            }
        }
        
        false
    }

    /// Get discovery statistics
    pub fn get_statistics(&self) -> &DiscoveryStatistics {
        &self.statistics
    }

    /// Convert discovered tests to unified test cases
    pub fn convert_to_test_cases(&self) -> Vec<TestCase> {
        self.discovered_tests.iter().map(|discovered| {
            TestCase {
                id: discovered.id.clone(),
                name: discovered.name.clone(),
                description: format!("Discovered test from {}", discovered.file_path.display()),
                category: discovered.category.clone(),
                execution_mode: TestExecutionMode::Sequential, // Default mode
                timeout: discovered.estimated_duration.unwrap_or(std::time::Duration::from_secs(30)),
                dependencies: discovered.dependencies.clone(),
                setup_required: !discovered.resource_requirements.external_dependencies.is_empty(),
                cleanup_required: discovered.resource_requirements.filesystem_access || discovered.resource_requirements.kernel_module_access,
                expected_outcome: crate::framework::unified_test_framework::TestOutcome::Pass,
                metadata: {
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("file_path".to_string(), discovered.file_path.to_string_lossy().to_string());
                    metadata.insert("line_number".to_string(), discovered.line_number.to_string());
                    metadata.insert("framework_type".to_string(), format!("{:?}", discovered.framework_type));
                    metadata.insert("function_name".to_string(), discovered.test_function.clone());
                    metadata
                },
            }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_engine_creation() {
        let config = TestDiscoveryConfig::default();
        let engine = TestDiscoveryEngine::new(config);
        assert_eq!(engine.discovered_tests.len(), 0);
    }

    #[test]
    fn test_file_pattern_matching() {
        let config = TestDiscoveryConfig::default();
        let engine = TestDiscoveryEngine::new(config);
        
        assert!(engine.matches_file_pattern(&PathBuf::from("test.rs")));
        assert!(engine.matches_file_pattern(&PathBuf::from("my_test.rs")));
        assert!(engine.matches_file_pattern(&PathBuf::from("test_something.rs")));
        assert!(!engine.matches_file_pattern(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_exclusion_patterns() {
        let config = TestDiscoveryConfig::default();
        let engine = TestDiscoveryEngine::new(config);
        
        assert!(engine.should_exclude(&PathBuf::from("target/debug/test")));
        assert!(engine.should_exclude(&PathBuf::from(".git/config")));
        assert!(!engine.should_exclude(&PathBuf::from("src/test.rs")));
    }

    #[test]
    fn test_function_name_extraction() {
        let config = TestDiscoveryConfig::default();
        let engine = TestDiscoveryEngine::new(config);
        
        let result = engine.extract_function_name("fn test_something() {");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_something");
        
        let result = engine.extract_function_name("    fn another_test() -> Result<(), Error> {");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "another_test");
    }

    #[test]
    fn test_category_determination() {
        let config = TestDiscoveryConfig::default();
        let engine = TestDiscoveryEngine::new(config);
        
        let category = engine.determine_test_category(
            &PathBuf::from("tests/integration/test.rs"),
            "test_function",
            &[]
        );
        assert_eq!(category, TestCategory::Integration);
        
        let category = engine.determine_test_category(
            &PathBuf::from("src/lib.rs"),
            "test_parity_check",
            &[]
        );
        assert_eq!(category, TestCategory::BehaviorParity);
    }
}