//! FUSE Graph Configuration
//! 
//! This module provides configuration management for FUSE graph operations,
//! including performance tuning, analytics settings, and integration parameters.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::shared::errors::{VexfsError, VexfsResult};

use std::time::Duration;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Maximum stack usage for FUSE graph operations (6KB limit)
const FUSE_GRAPH_MAX_STACK_USAGE: usize = 6144;

/// Default configuration values
const DEFAULT_GRAPH_CACHE_SIZE: usize = 1024 * 1024; // 1MB
const DEFAULT_ANALYTICS_INTERVAL: u64 = 60; // 60 seconds
const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_MAX_CONCURRENT_OPERATIONS: usize = 16;

/// FUSE Graph Configuration
/// 
/// Comprehensive configuration for FUSE graph operations, analytics, and performance tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuseGraphConfig {
    /// Graph operation settings
    pub graph_settings: GraphOperationSettings,
    /// Performance tuning parameters
    pub performance_settings: PerformanceSettings,
    /// Analytics configuration
    pub analytics_settings: AnalyticsSettings,
    /// Monitoring configuration
    pub monitoring_settings: MonitoringSettings,
    /// Cache configuration
    pub cache_settings: CacheSettings,
    /// Integration settings
    pub integration_settings: IntegrationSettings,
    /// Security settings
    pub security_settings: SecuritySettings,
}

impl Default for FuseGraphConfig {
    fn default() -> Self {
        Self {
            graph_settings: GraphOperationSettings::default(),
            performance_settings: PerformanceSettings::default(),
            analytics_settings: AnalyticsSettings::default(),
            monitoring_settings: MonitoringSettings::default(),
            cache_settings: CacheSettings::default(),
            integration_settings: IntegrationSettings::default(),
            security_settings: SecuritySettings::default(),
        }
    }
}

/// Graph operation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphOperationSettings {
    /// Enable graph operations in FUSE context
    pub enable_graph_operations: bool,
    /// Maximum graph size (number of nodes)
    pub max_graph_size: u64,
    /// Maximum edges per node
    pub max_edges_per_node: usize,
    /// Default search parameters
    pub default_search_params: SearchParameters,
    /// Default insertion parameters
    pub default_insertion_params: InsertionParameters,
    /// Enable automatic graph optimization
    pub enable_auto_optimization: bool,
    /// Optimization interval in seconds
    pub optimization_interval_seconds: u64,
    /// Enable graph validation
    pub enable_graph_validation: bool,
    /// Validation interval in seconds
    pub validation_interval_seconds: u64,
}

impl Default for GraphOperationSettings {
    fn default() -> Self {
        Self {
            enable_graph_operations: true,
            max_graph_size: 1_000_000, // 1M nodes
            max_edges_per_node: 64,
            default_search_params: SearchParameters::default(),
            default_insertion_params: InsertionParameters::default(),
            enable_auto_optimization: true,
            optimization_interval_seconds: 300, // 5 minutes
            enable_graph_validation: true,
            validation_interval_seconds: 600, // 10 minutes
        }
    }
}

/// Search parameters for graph operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParameters {
    /// Default k value for k-NN search
    pub default_k: usize,
    /// Default ef_search parameter
    pub default_ef_search: usize,
    /// Maximum k value allowed
    pub max_k: usize,
    /// Maximum ef_search value allowed
    pub max_ef_search: usize,
    /// Enable approximate search
    pub enable_approximate_search: bool,
    /// Search timeout in milliseconds
    pub search_timeout_ms: u64,
}

impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            default_k: 10,
            default_ef_search: 50,
            max_k: 1000,
            max_ef_search: 500,
            enable_approximate_search: true,
            search_timeout_ms: 5000, // 5 seconds
        }
    }
}

/// Insertion parameters for graph operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertionParameters {
    /// Default M parameter for HNSW
    pub default_m: usize,
    /// Default ef_construction parameter
    pub default_ef_construction: usize,
    /// Maximum M value allowed
    pub max_m: usize,
    /// Maximum ef_construction value allowed
    pub max_ef_construction: usize,
    /// Enable batch insertion
    pub enable_batch_insertion: bool,
    /// Batch size for insertions
    pub batch_size: usize,
    /// Insertion timeout in milliseconds
    pub insertion_timeout_ms: u64,
}

impl Default for InsertionParameters {
    fn default() -> Self {
        Self {
            default_m: 16,
            default_ef_construction: 200,
            max_m: 64,
            max_ef_construction: 1000,
            enable_batch_insertion: true,
            batch_size: DEFAULT_BATCH_SIZE,
            insertion_timeout_ms: 10000, // 10 seconds
        }
    }
}

/// Performance tuning settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Maximum concurrent graph operations
    pub max_concurrent_operations: usize,
    /// Thread pool size for graph operations
    pub thread_pool_size: usize,
    /// Enable parallel processing
    pub enable_parallel_processing: bool,
    /// Memory limit for graph operations in bytes
    pub memory_limit_bytes: u64,
    /// Stack usage limit in bytes
    pub stack_usage_limit_bytes: usize,
    /// Enable memory pooling
    pub enable_memory_pooling: bool,
    /// Memory pool size in bytes
    pub memory_pool_size_bytes: u64,
    /// Enable operation prioritization
    pub enable_operation_prioritization: bool,
    /// Priority levels configuration
    pub priority_levels: PriorityLevels,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            max_concurrent_operations: DEFAULT_MAX_CONCURRENT_OPERATIONS,
            thread_pool_size: 4,
            enable_parallel_processing: true,
            memory_limit_bytes: 512 * 1024 * 1024, // 512MB
            stack_usage_limit_bytes: FUSE_GRAPH_MAX_STACK_USAGE,
            enable_memory_pooling: true,
            memory_pool_size_bytes: 64 * 1024 * 1024, // 64MB
            enable_operation_prioritization: true,
            priority_levels: PriorityLevels::default(),
        }
    }
}

/// Priority levels for operation scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityLevels {
    /// High priority operations (e.g., real-time search)
    pub high_priority_weight: f32,
    /// Medium priority operations (e.g., batch operations)
    pub medium_priority_weight: f32,
    /// Low priority operations (e.g., background optimization)
    pub low_priority_weight: f32,
    /// Maximum queue size per priority level
    pub max_queue_size_per_level: usize,
}

impl Default for PriorityLevels {
    fn default() -> Self {
        Self {
            high_priority_weight: 1.0,
            medium_priority_weight: 0.5,
            low_priority_weight: 0.1,
            max_queue_size_per_level: 1000,
        }
    }
}

/// Analytics configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSettings {
    /// Enable real-time analytics
    pub enable_real_time_analytics: bool,
    /// Analytics update interval in seconds
    pub analytics_interval_seconds: u64,
    /// Enable centrality measures
    pub enable_centrality_measures: bool,
    /// Centrality calculation interval in seconds
    pub centrality_interval_seconds: u64,
    /// Enable clustering analysis
    pub enable_clustering_analysis: bool,
    /// Clustering analysis interval in seconds
    pub clustering_interval_seconds: u64,
    /// Enable pathfinding analytics
    pub enable_pathfinding_analytics: bool,
    /// Pathfinding analysis interval in seconds
    pub pathfinding_interval_seconds: u64,
    /// Enable graph health monitoring
    pub enable_health_monitoring: bool,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    /// Analytics history size
    pub analytics_history_size: usize,
    /// Enable analytics persistence
    pub enable_analytics_persistence: bool,
    /// Analytics persistence path
    pub analytics_persistence_path: Option<PathBuf>,
}

impl Default for AnalyticsSettings {
    fn default() -> Self {
        Self {
            enable_real_time_analytics: true,
            analytics_interval_seconds: DEFAULT_ANALYTICS_INTERVAL,
            enable_centrality_measures: true,
            centrality_interval_seconds: 120, // 2 minutes
            enable_clustering_analysis: true,
            clustering_interval_seconds: 300, // 5 minutes
            enable_pathfinding_analytics: true,
            pathfinding_interval_seconds: 180, // 3 minutes
            enable_health_monitoring: true,
            health_check_interval_seconds: 60, // 1 minute
            analytics_history_size: 1000,
            enable_analytics_persistence: false,
            analytics_persistence_path: None,
        }
    }
}

/// Monitoring configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Performance metrics collection interval in seconds
    pub metrics_collection_interval_seconds: u64,
    /// Enable error monitoring
    pub enable_error_monitoring: bool,
    /// Error threshold for alerting
    pub error_threshold_percentage: f64,
    /// Enable latency monitoring
    pub enable_latency_monitoring: bool,
    /// Latency threshold for alerting in milliseconds
    pub latency_threshold_ms: u64,
    /// Enable memory monitoring
    pub enable_memory_monitoring: bool,
    /// Memory threshold for alerting in bytes
    pub memory_threshold_bytes: u64,
    /// Enable throughput monitoring
    pub enable_throughput_monitoring: bool,
    /// Throughput threshold for alerting (ops/sec)
    pub throughput_threshold_ops_per_sec: f64,
    /// Monitoring history size
    pub monitoring_history_size: usize,
    /// Enable monitoring alerts
    pub enable_monitoring_alerts: bool,
    /// Alert configuration
    pub alert_config: AlertConfig,
}

impl Default for MonitoringSettings {
    fn default() -> Self {
        Self {
            enable_performance_monitoring: true,
            metrics_collection_interval_seconds: 30,
            enable_error_monitoring: true,
            error_threshold_percentage: 5.0, // 5% error rate
            enable_latency_monitoring: true,
            latency_threshold_ms: 1000, // 1 second
            enable_memory_monitoring: true,
            memory_threshold_bytes: 256 * 1024 * 1024, // 256MB
            enable_throughput_monitoring: true,
            throughput_threshold_ops_per_sec: 100.0,
            monitoring_history_size: 2000,
            enable_monitoring_alerts: true,
            alert_config: AlertConfig::default(),
        }
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable email alerts
    pub enable_email_alerts: bool,
    /// Email recipients
    pub email_recipients: Vec<String>,
    /// Enable webhook alerts
    pub enable_webhook_alerts: bool,
    /// Webhook URLs
    pub webhook_urls: Vec<String>,
    /// Enable log alerts
    pub enable_log_alerts: bool,
    /// Log level for alerts
    pub alert_log_level: String,
    /// Alert cooldown period in seconds
    pub alert_cooldown_seconds: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enable_email_alerts: false,
            email_recipients: Vec::new(),
            enable_webhook_alerts: false,
            webhook_urls: Vec::new(),
            enable_log_alerts: true,
            alert_log_level: "WARN".to_string(),
            alert_cooldown_seconds: 300, // 5 minutes
        }
    }
}

/// Cache configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    /// Enable graph caching
    pub enable_graph_caching: bool,
    /// Graph cache size in bytes
    pub graph_cache_size_bytes: usize,
    /// Enable search result caching
    pub enable_search_result_caching: bool,
    /// Search result cache size
    pub search_result_cache_size: usize,
    /// Search result cache TTL in seconds
    pub search_result_cache_ttl_seconds: u64,
    /// Enable analytics caching
    pub enable_analytics_caching: bool,
    /// Analytics cache size
    pub analytics_cache_size: usize,
    /// Analytics cache TTL in seconds
    pub analytics_cache_ttl_seconds: u64,
    /// Cache eviction policy
    pub cache_eviction_policy: CacheEvictionPolicy,
    /// Enable cache compression
    pub enable_cache_compression: bool,
    /// Cache compression algorithm
    pub cache_compression_algorithm: CompressionAlgorithm,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            enable_graph_caching: true,
            graph_cache_size_bytes: DEFAULT_GRAPH_CACHE_SIZE,
            enable_search_result_caching: true,
            search_result_cache_size: 1000,
            search_result_cache_ttl_seconds: 300, // 5 minutes
            enable_analytics_caching: true,
            analytics_cache_size: 500,
            analytics_cache_ttl_seconds: 600, // 10 minutes
            cache_eviction_policy: CacheEvictionPolicy::LRU,
            enable_cache_compression: false,
            cache_compression_algorithm: CompressionAlgorithm::LZ4,
        }
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheEvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// First In, First Out
    FIFO,
    /// Random eviction
    Random,
}

/// Compression algorithms for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// LZ4 compression
    LZ4,
    /// Zstd compression
    Zstd,
    /// Gzip compression
    Gzip,
}

/// Integration settings with other systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationSettings {
    /// Enable vector storage integration
    pub enable_vector_storage_integration: bool,
    /// Vector storage sync interval in seconds
    pub vector_storage_sync_interval_seconds: u64,
    /// Enable journal integration
    pub enable_journal_integration: bool,
    /// Journal sync mode
    pub journal_sync_mode: JournalSyncMode,
    /// Enable FUSE filesystem integration
    pub enable_fuse_integration: bool,
    /// FUSE operation timeout in milliseconds
    pub fuse_operation_timeout_ms: u64,
    /// Enable cross-layer consistency
    pub enable_cross_layer_consistency: bool,
    /// Consistency check interval in seconds
    pub consistency_check_interval_seconds: u64,
    /// Enable external API integration
    pub enable_external_api_integration: bool,
    /// External API endpoints
    pub external_api_endpoints: HashMap<String, String>,
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self {
            enable_vector_storage_integration: true,
            vector_storage_sync_interval_seconds: 60,
            enable_journal_integration: true,
            journal_sync_mode: JournalSyncMode::Immediate,
            enable_fuse_integration: true,
            fuse_operation_timeout_ms: 30000, // 30 seconds
            enable_cross_layer_consistency: true,
            consistency_check_interval_seconds: 120, // 2 minutes
            enable_external_api_integration: false,
            external_api_endpoints: HashMap::new(),
        }
    }
}

/// Journal synchronization modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalSyncMode {
    /// Immediate synchronization
    Immediate,
    /// Batch synchronization
    Batch,
    /// Lazy synchronization
    Lazy,
    /// Disabled synchronization
    Disabled,
}

/// Security settings for graph operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Enable access control
    pub enable_access_control: bool,
    /// Access control policy
    pub access_control_policy: AccessControlPolicy,
    /// Enable operation auditing
    pub enable_operation_auditing: bool,
    /// Audit log path
    pub audit_log_path: Option<PathBuf>,
    /// Enable data encryption
    pub enable_data_encryption: bool,
    /// Encryption algorithm
    pub encryption_algorithm: EncryptionAlgorithm,
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Input validation rules
    pub input_validation_rules: InputValidationRules,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limiting configuration
    pub rate_limiting_config: RateLimitingConfig,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            enable_access_control: true,
            access_control_policy: AccessControlPolicy::default(),
            enable_operation_auditing: true,
            audit_log_path: None,
            enable_data_encryption: false,
            encryption_algorithm: EncryptionAlgorithm::AES256,
            enable_input_validation: true,
            input_validation_rules: InputValidationRules::default(),
            enable_rate_limiting: true,
            rate_limiting_config: RateLimitingConfig::default(),
        }
    }
}

/// Access control policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlPolicy {
    /// Default access level
    pub default_access_level: AccessLevel,
    /// User-specific access levels
    pub user_access_levels: HashMap<u32, AccessLevel>,
    /// Group-specific access levels
    pub group_access_levels: HashMap<u32, AccessLevel>,
    /// Operation-specific permissions
    pub operation_permissions: HashMap<String, Vec<AccessLevel>>,
}

impl Default for AccessControlPolicy {
    fn default() -> Self {
        Self {
            default_access_level: AccessLevel::Read,
            user_access_levels: HashMap::new(),
            group_access_levels: HashMap::new(),
            operation_permissions: HashMap::new(),
        }
    }
}

/// Access levels for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    /// No access
    None,
    /// Read-only access
    Read,
    /// Read and write access
    ReadWrite,
    /// Full administrative access
    Admin,
}

/// Encryption algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// No encryption
    None,
    /// AES-256 encryption
    AES256,
    /// ChaCha20 encryption
    ChaCha20,
}

/// Input validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidationRules {
    /// Maximum vector dimension
    pub max_vector_dimension: usize,
    /// Maximum node ID value
    pub max_node_id: u64,
    /// Maximum metadata size in bytes
    pub max_metadata_size_bytes: usize,
    /// Allowed metadata keys
    pub allowed_metadata_keys: Vec<String>,
    /// Maximum query string length
    pub max_query_string_length: usize,
    /// Enable SQL injection protection
    pub enable_sql_injection_protection: bool,
    /// Enable XSS protection
    pub enable_xss_protection: bool,
}

impl Default for InputValidationRules {
    fn default() -> Self {
        Self {
            max_vector_dimension: 2048,
            max_node_id: u64::MAX,
            max_metadata_size_bytes: 1024 * 1024, // 1MB
            allowed_metadata_keys: Vec::new(),
            max_query_string_length: 10000,
            enable_sql_injection_protection: true,
            enable_xss_protection: true,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Maximum operations per second per user
    pub max_ops_per_second_per_user: f64,
    /// Maximum operations per second per IP
    pub max_ops_per_second_per_ip: f64,
    /// Maximum concurrent operations per user
    pub max_concurrent_ops_per_user: usize,
    /// Rate limiting window size in seconds
    pub rate_limiting_window_seconds: u64,
    /// Enable burst allowance
    pub enable_burst_allowance: bool,
    /// Burst allowance size
    pub burst_allowance_size: usize,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            max_ops_per_second_per_user: 100.0,
            max_ops_per_second_per_ip: 1000.0,
            max_concurrent_ops_per_user: 10,
            rate_limiting_window_seconds: 60,
            enable_burst_allowance: true,
            burst_allowance_size: 50,
        }
    }
}

impl FuseGraphConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from file
    pub fn load_from_file(path: &PathBuf) -> SemanticResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SemanticError::IoError(format!("Failed to read config file: {}", e)))?;
        
        let config: FuseGraphConfig = serde_json::from_str(&content)
            .map_err(|e| SemanticError::SerializationError(format!("Failed to parse config: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> SemanticResult<()> {
        self.validate()?;
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| SemanticError::SerializationError(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| SemanticError::IoError(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }

    /// Validate configuration settings
    pub fn validate(&self) -> SemanticResult<()> {
        // Validate graph settings
        if self.graph_settings.max_graph_size == 0 {
            return Err(SemanticError::ConfigurationError(
                "max_graph_size must be greater than 0".to_string()
            ));
        }

        if self.graph_settings.max_edges_per_node == 0 {
            return Err(SemanticError::ConfigurationError(
                "max_edges_per_node must be greater than 0".to_string()
            ));
        }

        // Validate performance settings
        if self.performance_settings.max_concurrent_operations == 0 {
            return Err(SemanticError::ConfigurationError(
                "max_concurrent_operations must be greater than 0".to_string()
            ));
        }

        if self.performance_settings.stack_usage_limit_bytes > FUSE_GRAPH_MAX_STACK_USAGE {
            return Err(SemanticError::ConfigurationError(
                format!("stack_usage_limit_bytes cannot exceed {}", FUSE_GRAPH_MAX_STACK_USAGE)
            ));
        }

        // Validate cache settings
        if self.cache_settings.graph_cache_size_bytes == 0 {
            return Err(SemanticError::ConfigurationError(
                "graph_cache_size_bytes must be greater than 0".to_string()
            ));
        }

        Ok(())
    }

    /// Get optimized configuration for performance
    pub fn get_performance_optimized() -> Self {
        let mut config = Self::default();
        
        // Optimize for performance
        config.performance_settings.max_concurrent_operations = 32;
        config.performance_settings.thread_pool_size = 8;
        config.performance_settings.enable_parallel_processing = true;
        config.performance_settings.enable_memory_pooling = true;
        config.performance_settings.memory_pool_size_bytes = 128 * 1024 * 1024; // 128MB
        
        // Optimize cache settings
        config.cache_settings.graph_cache_size_bytes = 4 * 1024 * 1024; // 4MB
        config.cache_settings.search_result_cache_size = 2000;
        config.cache_settings.analytics_cache_size = 1000;
        
        // Optimize analytics settings
        config.analytics_settings.analytics_interval_seconds = 30;
        config.analytics_settings.centrality_interval_seconds = 60;
        config.analytics_settings.clustering_interval_seconds = 120;
        
        config
    }

    /// Get memory-optimized configuration
    pub fn get_memory_optimized() -> Self {
        let mut config = Self::default();
        
        // Optimize for memory usage
        config.performance_settings.memory_limit_bytes = 128 * 1024 * 1024; // 128MB
        config.performance_settings.memory_pool_size_bytes = 32 * 1024 * 1024; // 32MB
        config.performance_settings.max_concurrent_operations = 8;
        
        // Reduce cache sizes
        config.cache_settings.graph_cache_size_bytes = 512 * 1024; // 512KB
        config.cache_settings.search_result_cache_size = 500;
        config.cache_settings.analytics_cache_size = 250;
        
        // Reduce analytics frequency
        config.analytics_settings.analytics_interval_seconds = 120;
        config.analytics_settings.centrality_interval_seconds = 300;
        config.analytics_settings.clustering_interval_seconds = 600;
        
        config
    }

    /// Get security-hardened configuration
    pub fn get_security_hardened() -> Self {
        let mut config = Self::default();
        
        // Enable all security features
        config.security_settings.enable_access_control = true;
        config.security_settings.enable_operation_auditing = true;
        config.security_settings.enable_data_encryption = true;
        config.security_settings.enable_input_validation = true;
        config.security_settings.enable_rate_limiting = true;
        
        // Stricter rate limiting
        config.security_settings.rate_limiting_config.max_ops_per_second_per_user = 50.0;
        config.security_settings.rate_limiting_config.max_concurrent_ops_per_user = 5;
        
        // Stricter input validation
        config.security_settings.input_validation_rules.max_vector_dimension = 1024;
        config.security_settings.input_validation_rules.max_metadata_size_bytes = 512 * 1024; // 512KB
        
        config
    }
}