//! Graph Performance Metrics
//! 
//! This module provides comprehensive performance monitoring and metrics collection
//! for FUSE graph operations, including real-time analytics, health monitoring,
//! and performance optimization insights.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::shared::errors::{VexfsError, VexfsResult};

use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::collections::{HashMap, BTreeMap, VecDeque};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Maximum stack usage for metrics operations (6KB limit)
const METRICS_MAX_STACK_USAGE: usize = 6144;

/// Default metrics history size
const DEFAULT_METRICS_HISTORY_SIZE: usize = 1000;

/// Default percentile calculations
const DEFAULT_PERCENTILES: &[f64] = &[0.5, 0.75, 0.90, 0.95, 0.99];

/// Comprehensive graph performance metrics collector
#[derive(Debug)]
pub struct GraphPerformanceMetrics {
    /// Core operation metrics
    operation_metrics: Arc<RwLock<OperationMetrics>>,
    /// Latency distribution metrics
    latency_metrics: Arc<RwLock<LatencyMetrics>>,
    /// Throughput metrics
    throughput_metrics: Arc<RwLock<ThroughputMetrics>>,
    /// Memory usage metrics
    memory_metrics: Arc<RwLock<MemoryMetrics>>,
    /// Error tracking metrics
    error_metrics: Arc<RwLock<ErrorMetrics>>,
    /// Cache performance metrics
    cache_metrics: Arc<RwLock<CacheMetrics>>,
    /// Graph health metrics
    health_metrics: Arc<RwLock<GraphHealthMetrics>>,
    /// Resource utilization metrics
    resource_metrics: Arc<RwLock<ResourceMetrics>>,
    /// Historical metrics storage
    metrics_history: Arc<RwLock<MetricsHistory>>,
    /// Metrics configuration
    config: MetricsConfig,
}

/// Configuration for metrics collection
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Enable detailed metrics collection
    pub enable_detailed_metrics: bool,
    /// Metrics collection interval in milliseconds
    pub collection_interval_ms: u64,
    /// History retention size
    pub history_retention_size: usize,
    /// Enable percentile calculations
    pub enable_percentile_calculations: bool,
    /// Percentiles to calculate
    pub percentiles: Vec<f64>,
    /// Enable real-time alerts
    pub enable_real_time_alerts: bool,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_detailed_metrics: true,
            collection_interval_ms: 1000, // 1 second
            history_retention_size: DEFAULT_METRICS_HISTORY_SIZE,
            enable_percentile_calculations: true,
            percentiles: DEFAULT_PERCENTILES.to_vec(),
            enable_real_time_alerts: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

/// Alert thresholds for performance monitoring
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// Maximum acceptable latency in milliseconds
    pub max_latency_ms: f64,
    /// Minimum acceptable throughput (ops/sec)
    pub min_throughput_ops_per_sec: f64,
    /// Maximum acceptable error rate (percentage)
    pub max_error_rate_percentage: f64,
    /// Maximum acceptable memory usage in bytes
    pub max_memory_usage_bytes: u64,
    /// Minimum acceptable cache hit rate (percentage)
    pub min_cache_hit_rate_percentage: f64,
    /// Maximum acceptable CPU usage (percentage)
    pub max_cpu_usage_percentage: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_latency_ms: 1000.0, // 1 second
            min_throughput_ops_per_sec: 100.0,
            max_error_rate_percentage: 5.0, // 5%
            max_memory_usage_bytes: 512 * 1024 * 1024, // 512MB
            min_cache_hit_rate_percentage: 80.0, // 80%
            max_cpu_usage_percentage: 80.0, // 80%
        }
    }
}

/// Core operation metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationMetrics {
    /// Total operations processed
    pub total_operations: u64,
    /// Operations by type
    pub operations_by_type: HashMap<String, u64>,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Operations in progress
    pub operations_in_progress: u64,
    /// Average operations per second
    pub avg_ops_per_second: f64,
    /// Peak operations per second
    pub peak_ops_per_second: f64,
    /// Last operation timestamp
    pub last_operation_time: SystemTime,
    /// Operation start times (for tracking in-progress operations)
    pub operation_start_times: HashMap<Uuid, Instant>,
}

/// Latency distribution metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// Total latency samples
    pub total_samples: u64,
    /// Average latency in microseconds
    pub avg_latency_us: f64,
    /// Minimum latency in microseconds
    pub min_latency_us: f64,
    /// Maximum latency in microseconds
    pub max_latency_us: f64,
    /// Latency percentiles
    pub percentiles: BTreeMap<String, f64>,
    /// Latency by operation type
    pub latency_by_operation: HashMap<String, LatencyStats>,
    /// Recent latency samples (for percentile calculation)
    pub recent_samples: VecDeque<f64>,
    /// Latency histogram buckets
    pub latency_histogram: HashMap<String, u64>,
}

/// Latency statistics for a specific operation type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Sample count
    pub sample_count: u64,
    /// Average latency
    pub avg_latency_us: f64,
    /// Minimum latency
    pub min_latency_us: f64,
    /// Maximum latency
    pub max_latency_us: f64,
    /// Standard deviation
    pub std_dev_us: f64,
}

/// Throughput metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Current throughput (ops/sec)
    pub current_throughput: f64,
    /// Average throughput (ops/sec)
    pub avg_throughput: f64,
    /// Peak throughput (ops/sec)
    pub peak_throughput: f64,
    /// Throughput by operation type
    pub throughput_by_operation: HashMap<String, f64>,
    /// Throughput history (timestamp, ops/sec)
    pub throughput_history: VecDeque<(SystemTime, f64)>,
    /// Throughput trend (increasing/decreasing/stable)
    pub throughput_trend: ThroughputTrend,
    /// Last throughput calculation time
    pub last_calculation_time: SystemTime,
}

/// Throughput trend indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThroughputTrend {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

impl Default for ThroughputTrend {
    fn default() -> Self {
        ThroughputTrend::Unknown
    }
}

/// Memory usage metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Current memory usage in bytes
    pub current_memory_usage: u64,
    /// Peak memory usage in bytes
    pub peak_memory_usage: u64,
    /// Average memory usage in bytes
    pub avg_memory_usage: u64,
    /// Memory usage by component
    pub memory_by_component: HashMap<String, u64>,
    /// Memory allocation rate (bytes/sec)
    pub allocation_rate: f64,
    /// Memory deallocation rate (bytes/sec)
    pub deallocation_rate: f64,
    /// Memory fragmentation percentage
    pub fragmentation_percentage: f64,
    /// Memory pool utilization
    pub pool_utilization: HashMap<String, f64>,
    /// Memory pressure indicators
    pub memory_pressure: MemoryPressure,
}

/// Memory pressure levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryPressure {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for MemoryPressure {
    fn default() -> Self {
        MemoryPressure::Low
    }
}

/// Error tracking metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,
    /// Errors by type
    pub errors_by_type: HashMap<String, u64>,
    /// Errors by operation
    pub errors_by_operation: HashMap<String, u64>,
    /// Error rate (errors/total operations)
    pub error_rate: f64,
    /// Recent error rate (last N operations)
    pub recent_error_rate: f64,
    /// Error recovery rate
    pub recovery_rate: f64,
    /// Mean time to recovery (MTTR) in seconds
    pub mean_time_to_recovery_seconds: f64,
    /// Error patterns
    pub error_patterns: Vec<ErrorPattern>,
    /// Last error timestamp
    pub last_error_time: SystemTime,
}

/// Error pattern for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Pattern description
    pub pattern: String,
    /// Frequency of occurrence
    pub frequency: u64,
    /// First occurrence
    pub first_seen: SystemTime,
    /// Last occurrence
    pub last_seen: SystemTime,
    /// Associated operation types
    pub operation_types: Vec<String>,
}

/// Cache performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Cache hit rate
    pub hit_rate: f64,
    /// Cache miss rate
    pub miss_rate: f64,
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Cache size in bytes
    pub cache_size_bytes: u64,
    /// Cache utilization percentage
    pub utilization_percentage: f64,
    /// Cache eviction rate
    pub eviction_rate: f64,
    /// Cache by type
    pub cache_by_type: HashMap<String, CacheTypeMetrics>,
    /// Cache efficiency score
    pub efficiency_score: f64,
}

/// Cache metrics for a specific cache type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheTypeMetrics {
    /// Hit rate for this cache type
    pub hit_rate: f64,
    /// Size in bytes
    pub size_bytes: u64,
    /// Entry count
    pub entry_count: u64,
    /// Average entry size
    pub avg_entry_size_bytes: f64,
    /// Eviction count
    pub eviction_count: u64,
}

/// Graph health metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphHealthMetrics {
    /// Overall health score (0.0 to 1.0)
    pub overall_health_score: f64,
    /// Graph connectivity score
    pub connectivity_score: f64,
    /// Graph consistency score
    pub consistency_score: f64,
    /// Graph performance score
    pub performance_score: f64,
    /// Number of nodes
    pub node_count: u64,
    /// Number of edges
    pub edge_count: u64,
    /// Average degree
    pub avg_degree: f64,
    /// Graph density
    pub graph_density: f64,
    /// Clustering coefficient
    pub clustering_coefficient: f64,
    /// Average path length
    pub avg_path_length: f64,
    /// Disconnected components
    pub disconnected_components: u64,
    /// Graph quality indicators
    pub quality_indicators: GraphQualityIndicators,
}

/// Graph quality indicators
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphQualityIndicators {
    /// Index quality score
    pub index_quality_score: f64,
    /// Search accuracy score
    pub search_accuracy_score: f64,
    /// Update efficiency score
    pub update_efficiency_score: f64,
    /// Memory efficiency score
    pub memory_efficiency_score: f64,
    /// Scalability score
    pub scalability_score: f64,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// CPU utilization percentage
    pub cpu_utilization: f64,
    /// Memory utilization percentage
    pub memory_utilization: f64,
    /// I/O utilization percentage
    pub io_utilization: f64,
    /// Network utilization percentage
    pub network_utilization: f64,
    /// Thread pool utilization
    pub thread_pool_utilization: f64,
    /// File descriptor usage
    pub file_descriptor_usage: u64,
    /// Stack usage in bytes
    pub stack_usage_bytes: usize,
    /// Resource contention indicators
    pub resource_contention: ResourceContention,
}

/// Resource contention indicators
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceContention {
    /// Lock contention count
    pub lock_contention_count: u64,
    /// Queue wait time in microseconds
    pub queue_wait_time_us: f64,
    /// Resource starvation events
    pub starvation_events: u64,
    /// Backpressure indicators
    pub backpressure_level: BackpressureLevel,
}

/// Backpressure levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackpressureLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl Default for BackpressureLevel {
    fn default() -> Self {
        BackpressureLevel::None
    }
}

/// Historical metrics storage
#[derive(Debug)]
pub struct MetricsHistory {
    /// Snapshots of metrics over time
    snapshots: VecDeque<MetricsSnapshot>,
    /// Maximum history size
    max_size: usize,
    /// Aggregated statistics
    aggregated_stats: AggregatedStats,
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Snapshot timestamp
    pub timestamp: SystemTime,
    /// Operation metrics snapshot
    pub operation_metrics: OperationMetrics,
    /// Latency metrics snapshot
    pub latency_metrics: LatencyMetrics,
    /// Throughput metrics snapshot
    pub throughput_metrics: ThroughputMetrics,
    /// Memory metrics snapshot
    pub memory_metrics: MemoryMetrics,
    /// Error metrics snapshot
    pub error_metrics: ErrorMetrics,
    /// Cache metrics snapshot
    pub cache_metrics: CacheMetrics,
    /// Health metrics snapshot
    pub health_metrics: GraphHealthMetrics,
    /// Resource metrics snapshot
    pub resource_metrics: ResourceMetrics,
}

/// Aggregated statistics over time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedStats {
    /// Average metrics over time
    pub avg_metrics: MetricsAverages,
    /// Trend analysis
    pub trends: TrendAnalysis,
    /// Performance baselines
    pub baselines: PerformanceBaselines,
    /// Anomaly detection results
    pub anomalies: Vec<AnomalyDetection>,
}

/// Average metrics over time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsAverages {
    /// Average throughput
    pub avg_throughput: f64,
    /// Average latency
    pub avg_latency_us: f64,
    /// Average error rate
    pub avg_error_rate: f64,
    /// Average memory usage
    pub avg_memory_usage: u64,
    /// Average CPU utilization
    pub avg_cpu_utilization: f64,
}

/// Trend analysis results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Throughput trend
    pub throughput_trend: TrendDirection,
    /// Latency trend
    pub latency_trend: TrendDirection,
    /// Error rate trend
    pub error_rate_trend: TrendDirection,
    /// Memory usage trend
    pub memory_usage_trend: TrendDirection,
    /// Overall performance trend
    pub overall_trend: TrendDirection,
}

/// Trend directions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Degrading,
    Stable,
    Volatile,
    Unknown,
}

impl Default for TrendDirection {
    fn default() -> Self {
        TrendDirection::Unknown
    }
}

/// Performance baselines for comparison
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceBaselines {
    /// Baseline throughput
    pub baseline_throughput: f64,
    /// Baseline latency
    pub baseline_latency_us: f64,
    /// Baseline error rate
    pub baseline_error_rate: f64,
    /// Baseline memory usage
    pub baseline_memory_usage: u64,
    /// Baseline established timestamp
    pub baseline_established: SystemTime,
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetection {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Severity level
    pub severity: AnomalySeverity,
    /// Detection timestamp
    pub detected_at: SystemTime,
    /// Anomaly description
    pub description: String,
    /// Affected metrics
    pub affected_metrics: Vec<String>,
    /// Confidence score
    pub confidence: f64,
}

/// Types of anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    LatencySpike,
    ThroughputDrop,
    ErrorRateIncrease,
    MemoryLeak,
    ResourceExhaustion,
    PerformanceDegradation,
    UnusualPattern,
}

/// Anomaly severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl GraphPerformanceMetrics {
    /// Create a new performance metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            operation_metrics: Arc::new(RwLock::new(OperationMetrics::default())),
            latency_metrics: Arc::new(RwLock::new(LatencyMetrics::default())),
            throughput_metrics: Arc::new(RwLock::new(ThroughputMetrics::default())),
            memory_metrics: Arc::new(RwLock::new(MemoryMetrics::default())),
            error_metrics: Arc::new(RwLock::new(ErrorMetrics::default())),
            cache_metrics: Arc::new(RwLock::new(CacheMetrics::default())),
            health_metrics: Arc::new(RwLock::new(GraphHealthMetrics::default())),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            metrics_history: Arc::new(RwLock::new(MetricsHistory::new(config.history_retention_size))),
            config,
        }
    }

    /// Record the start of an operation
    pub fn record_operation_start(&self, operation_id: Uuid, operation_type: &str) -> SemanticResult<()> {
        let mut metrics = self.operation_metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire operation metrics write lock".to_string())
        })?;

        metrics.operations_in_progress += 1;
        metrics.operation_start_times.insert(operation_id, Instant::now());
        
        let count = metrics.operations_by_type.entry(operation_type.to_string()).or_insert(0);
        *count += 1;

        Ok(())
    }

    /// Record the completion of an operation
    pub fn record_operation_completion(
        &self,
        operation_id: Uuid,
        operation_type: &str,
        success: bool,
    ) -> SemanticResult<()> {
        let start_time = {
            let mut metrics = self.operation_metrics.write().map_err(|_| {
                SemanticError::LockError("Failed to acquire operation metrics write lock".to_string())
            })?;

            metrics.operations_in_progress = metrics.operations_in_progress.saturating_sub(1);
            metrics.total_operations += 1;
            
            if success {
                metrics.successful_operations += 1;
            } else {
                metrics.failed_operations += 1;
            }

            metrics.last_operation_time = SystemTime::now();
            metrics.operation_start_times.remove(&operation_id)
        };

        // Record latency if we have start time
        if let Some(start_time) = start_time {
            let latency_us = start_time.elapsed().as_micros() as f64;
            self.record_latency(operation_type, latency_us)?;
        }

        // Record error if operation failed
        if !success {
            self.record_error(operation_type, "Operation failed")?;
        }

        Ok(())
    }

    /// Record latency for an operation
    pub fn record_latency(&self, operation_type: &str, latency_us: f64) -> SemanticResult<()> {
        let mut metrics = self.latency_metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire latency metrics write lock".to_string())
        })?;

        metrics.total_samples += 1;
        
        // Update overall latency stats
        if metrics.total_samples == 1 {
            metrics.avg_latency_us = latency_us;
            metrics.min_latency_us = latency_us;
            metrics.max_latency_us = latency_us;
        } else {
            metrics.avg_latency_us = (metrics.avg_latency_us * (metrics.total_samples - 1) as f64 + latency_us) / metrics.total_samples as f64;
            metrics.min_latency_us = metrics.min_latency_us.min(latency_us);
            metrics.max_latency_us = metrics.max_latency_us.max(latency_us);
        }

        // Update operation-specific latency stats
        let op_stats = metrics.latency_by_operation.entry(operation_type.to_string()).or_insert(LatencyStats::default());
        op_stats.sample_count += 1;
        
        if op_stats.sample_count == 1 {
            op_stats.avg_latency_us = latency_us;
            op_stats.min_latency_us = latency_us;
            op_stats.max_latency_us = latency_us;
        } else {
            op_stats.avg_latency_us = (op_stats.avg_latency_us * (op_stats.sample_count - 1) as f64 + latency_us) / op_stats.sample_count as f64;
            op_stats.min_latency_us = op_stats.min_latency_us.min(latency_us);
            op_stats.max_latency_us = op_stats.max_latency_us.max(latency_us);
        }

        // Add to recent samples for percentile calculation
        metrics.recent_samples.push_back(latency_us);
        if metrics.recent_samples.len() > 1000 {
            metrics.recent_samples.pop_front();
        }

        // Update latency histogram
        let bucket = self.get_latency_bucket(latency_us);
        let count = metrics.latency_histogram.entry(bucket).or_insert(0);
        *count += 1;

        Ok(())
    }

    /// Record an error
    pub fn record_error(&self, operation_type: &str, error_description: &str) -> SemanticResult<()> {
        let mut metrics = self.error_metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire error metrics write lock".to_string())
        })?;

        metrics.total_errors += 1;
        metrics.last_error_time = SystemTime::now();

        let error_type = self.classify_error(error_description);
        let count = metrics.errors_by_type.entry(error_type).or_insert(0);
        *count += 1;

        let op_count = metrics.errors_by_operation.entry(operation_type.to_string()).or_insert(0);
        *op_count += 1;

        // Update error rates
        let total_ops = {
            let op_metrics = self.operation_metrics.read().map_err(|_| {
                SemanticError::LockError("Failed to acquire operation metrics read lock".to_string())
            })?;
            op_metrics.total_operations
        };

        if total_ops > 0 {
            metrics.error_rate = metrics.total_errors as f64 / total_ops as f64;
        }

        Ok(())
    }

    /// Update memory metrics
    pub fn update_memory_metrics(&self, current_usage: u64, component: &str) -> SemanticResult<()> {
        let mut metrics = self.memory_metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire memory metrics write lock".to_string())
        })?;

        metrics.current_memory_usage = current_usage;
        metrics.peak_memory_usage = metrics.peak_memory_usage.max(current_usage);
        
        // Update component-specific usage
        metrics.memory_by_component.insert(component.to_string(), current_usage);

        // Calculate memory pressure
        let pressure_threshold = self.config.alert_thresholds.max_memory_usage_bytes;
        metrics.memory_pressure = if current_usage > pressure_threshold {
            MemoryPressure::Critical
        } else if current_usage > pressure_threshold * 3 / 4 {
            MemoryPressure::High
        } else if current_usage > pressure_threshold / 2 {
            MemoryPressure::Medium
        } else {
            MemoryPressure::Low
        };

        Ok(())
    }

    /// Update cache metrics
    pub fn update_cache_metrics(&self, cache_type: &str, hit: bool) -> SemanticResult<()> {
        let mut metrics = self.cache_metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire cache metrics write lock".to_string())
        })?;

        if hit {
            metrics.total_hits += 1;
        } else {
            metrics.total_misses += 1;
        }

        let total_requests = metrics.total_hits + metrics.total_misses;
        if total_requests > 0 {
            metrics.hit_rate = metrics.total_hits as f64 / total_requests as f64;
            metrics.miss_rate = metrics.total_misses as f64 / total_requests as f64;
        }

        // Update cache type specific metrics
        let cache_stats = metrics.cache_by_type.entry(cache_type.to_string()).or_insert(CacheTypeMetrics::default());
        if hit {
            cache_stats.hit_rate = (cache_stats.hit_rate * cache_stats.entry_count as f64 + 1.0) / (cache_stats.entry_count + 1) as f64;
        }

        Ok(())
    }

    /// Get current metrics snapshot
    pub fn get_current_snapshot(&self) -> SemanticResult<MetricsSnapshot> {
        let operation_metrics = self.operation_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire operation metrics read lock".to_string())
        })?.clone();

        let latency_metrics = self.latency_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire latency metrics read lock".to_string())
        })?.clone();

        let throughput_metrics = self.throughput_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire throughput metrics read lock".to_string())
        })?.clone();

        let memory_metrics = self.memory_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire memory metrics read lock".to_string())
        })?.clone();

        let error_metrics = self.error_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire error metrics read lock".to_string())
        })?.clone();

        let cache_metrics = self.cache_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire cache metrics read lock".to_string())
        })?.clone();

        let health_metrics = self.health_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire health metrics read lock".to_string())
        })?.clone();

        let resource_metrics = self.resource_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire resource metrics read lock".to_string())
        })?.clone();

        Ok(MetricsSnapshot {
            timestamp: SystemTime::now(),
            operation_metrics,
            latency_metrics,
            throughput_metrics,
            memory_metrics,
            error_metrics,
            cache_metrics,
            health_metrics,
            resource_metrics,
        })
    }

    /// Calculate percentiles for latency metrics
    pub fn calculate_percentiles(&self) -> SemanticResult<()> {
        if !self.config.enable_percentile_calculations {
            return Ok(());
        }

        let mut metrics = self.latency_metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire latency metrics write lock".to_string())
        })?;

        if metrics.recent_samples.is_empty() {
            return Ok(());
        }

        let mut samples: Vec<f64> = metrics.recent_samples.iter().cloned().collect();
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        for &percentile in &self.config.percentiles {
            let index = ((samples.len() as f64 - 1.0) * percentile) as usize;
            let value = samples.get(index).copied().unwrap_or(0.0);
);
        }

        Ok(())
    }

    /// Check for performance alerts
    pub fn check_alerts(&self) -> SemanticResult<Vec<PerformanceAlert>> {
        if !self.config.enable_real_time_alerts {
            return Ok(Vec::new());
        }

        let mut alerts = Vec::new();

        // Check latency alerts
        let latency_metrics = self.latency_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire latency metrics read lock".to_string())
        })?;

        if latency_metrics.avg_latency_us > self.config.alert_thresholds.max_latency_ms * 1000.0 {
            alerts.push(PerformanceAlert {
                alert_type: AlertType::HighLatency,
                severity: AlertSeverity::High,
                message: format!("Average latency ({:.2}ms) exceeds threshold ({:.2}ms)", 
                    latency_metrics.avg_latency_us / 1000.0, 
                    self.config.alert_thresholds.max_latency_ms),
                timestamp: SystemTime::now(),
                metric_value: latency_metrics.avg_latency_us,
                threshold_value: self.config.alert_thresholds.max_latency_ms * 1000.0,
            });
        }

        // Check error rate alerts
        let error_metrics = self.error_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire error metrics read lock".to_string())
        })?;

        if error_metrics.error_rate * 100.0 > self.config.alert_thresholds.max_error_rate_percentage {
            alerts.push(PerformanceAlert {
                alert_type: AlertType::HighErrorRate,
                severity: AlertSeverity::Critical,
                message: format!("Error rate ({:.2}%) exceeds threshold ({:.2}%)", 
                    error_metrics.error_rate * 100.0, 
                    self.config.alert_thresholds.max_error_rate_percentage),
                timestamp: SystemTime::now(),
                metric_value: error_metrics.error_rate * 100.0,
                threshold_value: self.config.alert_thresholds.max_error_rate_percentage,
            });
        }

        // Check memory alerts
        let memory_metrics = self.memory_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire memory metrics read lock".to_string())
        })?;

        if memory_metrics.current_memory_usage > self.config.alert_thresholds.max_memory_usage_bytes {
            alerts.push(PerformanceAlert {
                alert_type: AlertType::HighMemoryUsage,
                severity: AlertSeverity::High,
                message: format!("Memory usage ({} MB) exceeds threshold ({} MB)", 
                    memory_metrics.current_memory_usage / 1024 / 1024, 
                    self.config.alert_thresholds.max_memory_usage_bytes / 1024 / 1024),
                timestamp: SystemTime::now(),
                metric_value: memory_metrics.current_memory_usage as f64,
                threshold_value: self.config.alert_thresholds.max_memory_usage_bytes as f64,
            });
        }

        // Check cache hit rate alerts
        let cache_metrics = self.cache_metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire cache metrics read lock".to_string())
        })?;

        if cache_metrics.hit_rate * 100.0 < self.config.alert_thresholds.min_cache_hit_rate_percentage {
            alerts.push(PerformanceAlert {
                alert_type: AlertType::LowCacheHitRate,
                severity: AlertSeverity::Medium,
                message: format!("Cache hit rate ({:.2}%) below threshold ({:.2}%)", 
                    cache_metrics.hit_rate * 100.0, 
                    self.config.alert_thresholds.min_cache_hit_rate_percentage),
                timestamp: SystemTime::now(),
                metric_value: cache_metrics.hit_rate * 100.0,
                threshold_value: self.config.alert_thresholds.min_cache_hit_rate_percentage,
            });
        }

        Ok(alerts)
    }

    /// Get performance summary
    pub fn get_performance_summary(&self) -> SemanticResult<PerformanceSummary> {
        let snapshot = self.get_current_snapshot()?;
        
        Ok(PerformanceSummary {
            timestamp: SystemTime::now(),
            overall_health_score: self.calculate_overall_health_score(&snapshot)?,
            key_metrics: KeyMetrics {
                avg_latency_ms: snapshot.latency_metrics.avg_latency_us / 1000.0,
                throughput_ops_per_sec: snapshot.throughput_metrics.current_throughput,
                error_rate_percentage: snapshot.error_metrics.error_rate * 100.0,
                memory_usage_mb: snapshot.memory_metrics.current_memory_usage / 1024 / 1024,
                cache_hit_rate_percentage: snapshot.cache_metrics.hit_rate * 100.0,
            },
            performance_trends: self.get_performance_trends()?,
            recommendations: self.generate_recommendations(&snapshot)?,
        })
    }

    // Private helper methods

    fn get_latency_bucket(&self, latency_us: f64) -> String {
        if latency_us < 1000.0 {
            "0-1ms".to_string()
        } else if latency_us < 10000.0 {
            "1-10ms".to_string()
        } else if latency_us < 100000.0 {
            "10-100ms".to_string()
        } else if latency_us < 1000000.0 {
            "100ms-1s".to_string()
        } else {
            ">1s".to_string()
        }
    }

    fn classify_error(&self, error_description: &str) -> String {
        if error_description.contains("timeout") {
            "timeout".to_string()
        } else if error_description.contains("memory") {
            "memory".to_string()
        } else if error_description.contains("network") {
            "network".to_string()
        } else if error_description.contains("validation") {
            "validation".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn calculate_overall_health_score(&self, snapshot: &MetricsSnapshot) -> SemanticResult<f64> {
        let mut score = 1.0;

        // Factor in error rate (0.3 weight)
        let error_penalty = (snapshot.error_metrics.error_rate * 100.0 / self.config.alert_thresholds.max_error_rate_percentage).min(1.0);
        score -= error_penalty * 0.3;

        // Factor in latency (0.25 weight)
        let latency_penalty = (snapshot.latency_metrics.avg_latency_us / 1000.0 / self.config.alert_thresholds.max_latency_ms).min(1.0);
        score -= latency_penalty * 0.25;

        // Factor in memory usage (0.2 weight)
        let memory_penalty = (snapshot.memory_metrics.current_memory_usage as f64 / self.config.alert_thresholds.max_memory_usage_bytes as f64).min(1.0);
        score -= memory_penalty * 0.2;

        // Factor in cache performance (0.15 weight)
        let cache_penalty = (1.0 - snapshot.cache_metrics.hit_rate).min(1.0);
        score -= cache_penalty * 0.15;

        // Factor in resource utilization (0.1 weight)
        let resource_penalty = (snapshot.resource_metrics.cpu_utilization / 100.0).min(1.0);
        score -= resource_penalty * 0.1;

        Ok(score.max(0.0))
    }

    fn get_performance_trends(&self) -> SemanticResult<PerformanceTrends> {
        // This would analyze historical data to determine trends
        // For now, return default trends
        Ok(PerformanceTrends {
            latency_trend: TrendDirection::Stable,
            throughput_trend: TrendDirection::Stable,
            error_rate_trend: TrendDirection::Stable,
            memory_usage_trend: TrendDirection::Stable,
            overall_trend: TrendDirection::Stable,
        })
    }

    fn generate_recommendations(&self, snapshot: &MetricsSnapshot) -> SemanticResult<Vec<PerformanceRecommendation>> {
        let mut recommendations = Vec::new();

        // High latency recommendation
        if snapshot.latency_metrics.avg_latency_us > self.config.alert_thresholds.max_latency_ms * 1000.0 {
            recommendations.push(PerformanceRecommendation {
                category: RecommendationCategory::Latency,
                priority: RecommendationPriority::High,
                title: "Reduce Operation Latency".to_string(),
                description: "Average latency is above acceptable threshold".to_string(),
                actions: vec![
                    "Consider increasing cache size".to_string(),
                    "Optimize graph traversal algorithms".to_string(),
                    "Review concurrent operation limits".to_string(),
                ],
                expected_impact: "20-40% latency reduction".to_string(),
            });
        }

        // High error rate recommendation
        if snapshot.error_metrics.error_rate > self.config.alert_thresholds.max_error_rate_percentage / 100.0 {
            recommendations.push(PerformanceRecommendation {
                category: RecommendationCategory::Reliability,
                priority: RecommendationPriority::Critical,
                title: "Reduce Error Rate".to_string(),
                description: "Error rate is above acceptable threshold".to_string(),
                actions: vec![
                    "Review error patterns and root causes".to_string(),
                    "Implement better error handling".to_string(),
                    "Add input validation".to_string(),
                ],
                expected_impact: "50-80% error reduction".to_string(),
            });
        }

        // Low cache hit rate recommendation
        if snapshot.cache_metrics.hit_rate < self.config.alert_thresholds.min_cache_hit_rate_percentage / 100.0 {
            recommendations.push(PerformanceRecommendation {
                category: RecommendationCategory::Caching,
                priority: RecommendationPriority::Medium,
                title: "Improve Cache Performance".to_string(),
                description: "Cache hit rate is below optimal threshold".to_string(),
                actions: vec![
                    "Increase cache size".to_string(),
                    "Review cache eviction policy".to_string(),
                    "Implement cache warming strategies".to_string(),
                ],
                expected_impact: "15-30% performance improvement".to_string(),
            });
        }

        Ok(recommendations)
    }
}

impl MetricsHistory {
    fn new(max_size: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_size,
            aggregated_stats: AggregatedStats::default(),
        }
    }

    fn add_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.snapshots.push_back(snapshot);
        if self.snapshots.len() > self.max_size {
            self.snapshots.pop_front();
        }
        self.update_aggregated_stats();
    }

    fn update_aggregated_stats(&mut self) {
        if self.snapshots.is_empty() {
            return;
        }

        let count = self.snapshots.len() as f64;
        let mut total_throughput = 0.0;
        let mut total_latency = 0.0;
        let mut total_error_rate = 0.0;
        let mut total_memory = 0u64;
        let mut total_cpu = 0.0;

        for snapshot in &self.snapshots {
            total_throughput += snapshot.throughput_metrics.current_throughput;
            total_latency += snapshot.latency_metrics.avg_latency_us;
            total_error_rate += snapshot.error_metrics.error_rate;
            total_memory += snapshot.memory_metrics.current_memory_usage;
            total_cpu += snapshot.resource_metrics.cpu_utilization;
        }

        self.aggregated_stats.avg_metrics = MetricsAverages {
            avg_throughput: total_throughput / count,
            avg_latency_us: total_latency / count,
            avg_error_rate: total_error_rate / count,
            avg_memory_usage: (total_memory as f64 / count) as u64,
            avg_cpu_utilization: total_cpu / count,
        };
    }
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert type
    pub alert_type: AlertType,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Alert timestamp
    pub timestamp: SystemTime,
    /// Current metric value
    pub metric_value: f64,
    /// Threshold value
    pub threshold_value: f64,
}

/// Types of performance alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HighLatency,
    LowThroughput,
    HighErrorRate,
    HighMemoryUsage,
    LowCacheHitRate,
    HighCpuUsage,
    ResourceExhaustion,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Summary timestamp
    pub timestamp: SystemTime,
    /// Overall health score (0.0 to 1.0)
    pub overall_health_score: f64,
    /// Key performance metrics
    pub key_metrics: KeyMetrics,
    /// Performance trends
    pub performance_trends: PerformanceTrends,
    /// Performance recommendations
    pub recommendations: Vec<PerformanceRecommendation>,
}

/// Key performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetrics {
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Throughput in operations per second
    pub throughput_ops_per_sec: f64,
    /// Error rate percentage
    pub error_rate_percentage: f64,
    /// Memory usage in megabytes
    pub memory_usage_mb: u64,
    /// Cache hit rate percentage
    pub cache_hit_rate_percentage: f64,
}

/// Performance trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    /// Latency trend
    pub latency_trend: TrendDirection,
    /// Throughput trend
    pub throughput_trend: TrendDirection,
    /// Error rate trend
    pub error_rate_trend: TrendDirection,
    /// Memory usage trend
    pub memory_usage_trend: TrendDirection,
    /// Overall performance trend
    pub overall_trend: TrendDirection,
}

/// Performance recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecommendation {
    /// Recommendation category
    pub category: RecommendationCategory,
    /// Recommendation priority
    pub priority: RecommendationPriority,
    /// Recommendation title
    pub title: String,
    /// Recommendation description
    pub description: String,
    /// Recommended actions
    pub actions: Vec<String>,
    /// Expected impact
    pub expected_impact: String,
}

/// Recommendation categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationCategory {
    Latency,
    Throughput,
    Memory,
    Caching,
    Reliability,
    Scalability,
    Security,
}

/// Recommendation priorities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}
            metrics.percentiles.insert(format!("p{}", (percentile * 100.0) as u32), value