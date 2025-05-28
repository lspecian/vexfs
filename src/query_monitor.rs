//! Query Performance Monitoring for VexFS Vector Search
//!
//! This module implements comprehensive telemetry and monitoring for vector search operations
//! to identify optimization opportunities. It provides real-time performance metrics collection,
//! query analytics, bottleneck identification, and optimization recommendations.

extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap, string::String, format, sync::Arc};
use core::{mem, time::Duration};

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::fs_core::operations::OperationContext;
use crate::query_planner::{QueryPlanner, QueryCharacteristics, IndexRecommendation, QueryExecutionPlan, ExecutionStage, StageType, QueryComplexity};
use crate::search_cache::{SearchResultCache, CacheStatistics, CacheConfig};
use crate::vector_search_integration::{VectorSearchSubsystem, SearchStatistics, SystemHealthStatus};
use crate::vector_search::{SearchQuery, VectorSearchEngine};
use crate::anns::{IndexStrategy, DistanceMetric};
use crate::result_scoring::ScoredResult;

/// Performance monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Enable real-time metrics collection
    pub enable_realtime_metrics: bool,
    /// Enable historical trend analysis
    pub enable_historical_analysis: bool,
    /// Enable performance regression detection
    pub enable_regression_detection: bool,
    /// Enable optimization recommendations
    pub enable_optimization_recommendations: bool,
    /// Metrics collection interval (microseconds)
    pub metrics_collection_interval_us: u64,
    /// Historical data retention period (microseconds)
    pub historical_retention_period_us: u64,
    /// Performance regression threshold (percentage)
    pub regression_threshold_percent: f32,
    /// Maximum number of performance samples to retain
    pub max_performance_samples: usize,
    /// Alert threshold for query latency (microseconds)
    pub latency_alert_threshold_us: u64,
    /// Alert threshold for memory usage (bytes)
    pub memory_alert_threshold_bytes: usize,
    /// Enable detailed query profiling
    pub enable_detailed_profiling: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_realtime_metrics: true,
            enable_historical_analysis: true,
            enable_regression_detection: true,
            enable_optimization_recommendations: true,
            metrics_collection_interval_us: 1000, // 1ms
            historical_retention_period_us: 24 * 60 * 60 * 1_000_000, // 24 hours
            regression_threshold_percent: 20.0, // 20% performance degradation
            max_performance_samples: 10000,
            latency_alert_threshold_us: 100_000, // 100ms
            memory_alert_threshold_bytes: 128 * 1024 * 1024, // 128MB
            enable_detailed_profiling: true,
        }
    }
}

/// Real-time performance metrics
#[derive(Debug, Clone, Default)]
pub struct RealTimeMetrics {
    /// Current query execution time (microseconds)
    pub current_query_time_us: u64,
    /// Current memory usage (bytes)
    pub current_memory_usage_bytes: usize,
    /// Current CPU utilization (0.0 - 1.0)
    pub current_cpu_utilization: f32,
    /// Current I/O operations per second
    pub current_io_ops_per_second: u64,
    /// Current cache hit rate (0.0 - 1.0)
    pub current_cache_hit_rate: f32,
    /// Current index utilization (0.0 - 1.0)
    pub current_index_utilization: f32,
    /// Active query count
    pub active_query_count: usize,
    /// Queries per second
    pub queries_per_second: f32,
    /// Average query latency (microseconds)
    pub avg_query_latency_us: u64,
    /// 95th percentile query latency (microseconds)
    pub p95_query_latency_us: u64,
    /// 99th percentile query latency (microseconds)
    pub p99_query_latency_us: u64,
}

/// Historical performance trend data
#[derive(Debug, Clone)]
pub struct HistoricalTrend {
    /// Timestamp (microseconds)
    pub timestamp_us: u64,
    /// Query latency (microseconds)
    pub query_latency_us: u64,
    /// Memory usage (bytes)
    pub memory_usage_bytes: usize,
    /// Cache hit rate (0.0 - 1.0)
    pub cache_hit_rate: f32,
    /// Index strategy used
    pub index_strategy: IndexStrategy,
    /// Query complexity
    pub query_complexity: QueryComplexity,
    /// Result count
    pub result_count: usize,
    /// User ID
    pub user_id: u32,
}

/// Performance bottleneck identification
#[derive(Debug, Clone)]
pub struct PerformanceBottleneck {
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,
    /// Severity level (0.0 - 1.0)
    pub severity: f32,
    /// Description of the bottleneck
    pub description: String,
    /// Affected operations count
    pub affected_operations: u64,
    /// Performance impact (percentage)
    pub performance_impact_percent: f32,
    /// Recommended actions
    pub recommendations: Vec<OptimizationRecommendation>,
    /// First detected timestamp
    pub first_detected_us: u64,
    /// Last detected timestamp
    pub last_detected_us: u64,
}

/// Types of performance bottlenecks
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BottleneckType {
    /// High query latency
    HighLatency,
    /// Memory pressure
    MemoryPressure,
    /// Cache inefficiency
    CacheInefficiency,
    /// Index suboptimal performance
    IndexInefficiency,
    /// I/O bottleneck
    IOBottleneck,
    /// CPU bottleneck
    CPUBottleneck,
    /// Query planning overhead
    PlanningOverhead,
    /// Result processing bottleneck
    ResultProcessingBottleneck,
}

/// Optimization recommendations
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Priority level (0.0 - 1.0)
    pub priority: f32,
    /// Expected performance improvement (percentage)
    pub expected_improvement_percent: f32,
    /// Implementation complexity (0.0 - 1.0)
    pub implementation_complexity: f32,
    /// Detailed description
    pub description: String,
    /// Specific actions to take
    pub actions: Vec<String>,
    /// Estimated implementation time (microseconds)
    pub estimated_implementation_time_us: u64,
}

/// Types of optimization recommendations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecommendationType {
    /// Index strategy optimization
    IndexOptimization,
    /// Cache configuration tuning
    CacheOptimization,
    /// Query parameter tuning
    QueryOptimization,
    /// Memory allocation optimization
    MemoryOptimization,
    /// SIMD optimization
    SimdOptimization,
    /// Batch processing optimization
    BatchOptimization,
    /// Resource allocation optimization
    ResourceOptimization,
}
/// Query analytics and profiling data
#[derive(Debug, Clone)]
pub struct QueryAnalytics {
    /// Query pattern classification
    pub query_pattern: QueryPattern,
    /// Execution stage breakdown
    pub stage_breakdown: Vec<StagePerformance>,
    /// Resource utilization breakdown
    pub resource_utilization: ResourceUtilization,
    /// Index effectiveness metrics
    pub index_effectiveness: IndexEffectiveness,
    /// Cache effectiveness metrics
    pub cache_effectiveness: CacheEffectiveness,
    /// Query optimization opportunities
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
}

/// Query pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryPattern {
    /// Simple exact search
    SimpleExact,
    /// Simple approximate search
    SimpleApproximate,
    /// Complex filtered search
    ComplexFiltered,
    /// High-dimensional search
    HighDimensional,
    /// Sparse vector search
    SparseVector,
    /// Batch search pattern
    BatchSearch,
    /// Frequent similar queries
    FrequentSimilar,
    /// Rare complex queries
    RareComplex,
}

/// Performance data for individual execution stages
#[derive(Debug, Clone)]
pub struct StagePerformance {
    /// Stage type
    pub stage_type: StageType,
    /// Actual execution time (microseconds)
    pub actual_time_us: u64,
    /// Estimated time (microseconds)
    pub estimated_time_us: u64,
    /// Performance ratio (actual/estimated)
    pub performance_ratio: f32,
    /// Memory usage (bytes)
    pub memory_usage_bytes: usize,
    /// CPU utilization (0.0 - 1.0)
    pub cpu_utilization: f32,
    /// I/O operations count
    pub io_operations: u64,
}

/// Resource utilization breakdown
#[derive(Debug, Clone)]
pub struct ResourceUtilization {
    /// Memory utilization breakdown
    pub memory: MemoryUtilization,
    /// CPU utilization breakdown
    pub cpu: CpuUtilization,
    /// I/O utilization breakdown
    pub io: IoUtilization,
}

/// Memory utilization details
#[derive(Debug, Clone)]
pub struct MemoryUtilization {
    /// Total memory allocated (bytes)
    pub total_allocated_bytes: usize,
    /// Peak memory usage (bytes)
    pub peak_usage_bytes: usize,
    /// Memory efficiency (useful data / total allocated)
    pub efficiency: f32,
    /// Memory fragmentation level (0.0 - 1.0)
    pub fragmentation: f32,
    /// Cache memory usage (bytes)
    pub cache_usage_bytes: usize,
    /// Index memory usage (bytes)
    pub index_usage_bytes: usize,
}

/// CPU utilization details
#[derive(Debug, Clone)]
pub struct CpuUtilization {
    /// Total CPU time (microseconds)
    pub total_cpu_time_us: u64,
    /// SIMD utilization (0.0 - 1.0)
    pub simd_utilization: f32,
    /// Vectorization efficiency (0.0 - 1.0)
    pub vectorization_efficiency: f32,
    /// Parallelization efficiency (0.0 - 1.0)
    pub parallelization_efficiency: f32,
}

/// I/O utilization details
#[derive(Debug, Clone)]
pub struct IoUtilization {
    /// Total I/O operations
    pub total_io_operations: u64,
    /// Read operations
    pub read_operations: u64,
    /// Write operations
    pub write_operations: u64,
    /// I/O throughput (bytes per second)
    pub throughput_bytes_per_second: u64,
    /// I/O latency (microseconds)
    pub avg_io_latency_us: u64,
}

/// Index effectiveness metrics
#[derive(Debug, Clone)]
pub struct IndexEffectiveness {
    /// Index strategy used
    pub strategy: IndexStrategy,
    /// Index hit rate (0.0 - 1.0)
    pub hit_rate: f32,
    /// Index traversal efficiency (0.0 - 1.0)
    pub traversal_efficiency: f32,
    /// Index memory efficiency (0.0 - 1.0)
    pub memory_efficiency: f32,
    /// Index build time (microseconds)
    pub build_time_us: u64,
    /// Index update frequency
    pub update_frequency: f32,
}

/// Cache effectiveness metrics
#[derive(Debug, Clone)]
pub struct CacheEffectiveness {
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f32,
    /// Cache memory efficiency (0.0 - 1.0)
    pub memory_efficiency: f32,
    /// Cache eviction rate
    pub eviction_rate: f32,
    /// Cache warming effectiveness (0.0 - 1.0)
    pub warming_effectiveness: f32,
    /// Average cache lookup time (microseconds)
    pub avg_lookup_time_us: u64,
}

/// Optimization opportunity identification
#[derive(Debug, Clone)]
pub struct OptimizationOpportunity {
    /// Opportunity type
    pub opportunity_type: RecommendationType,
    /// Potential improvement (percentage)
    pub potential_improvement_percent: f32,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Description
    pub description: String,
    /// Required changes
    pub required_changes: Vec<String>,
}

/// Performance alert
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    /// Alert type
    pub alert_type: AlertType,
    /// Severity level
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Affected metric
    pub affected_metric: String,
    /// Current value
    pub current_value: f64,
    /// Threshold value
    pub threshold_value: f64,
    /// First triggered timestamp
    pub first_triggered_us: u64,
    /// Last triggered timestamp
    pub last_triggered_us: u64,
    /// Trigger count
    pub trigger_count: u64,
}

/// Alert types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertType {
    /// High latency alert
    HighLatency,
    /// Memory pressure alert
    MemoryPressure,
    /// Cache inefficiency alert
    CacheInefficiency,
    /// Performance regression alert
    PerformanceRegression,
    /// Resource exhaustion alert
    ResourceExhaustion,
    /// Index degradation alert
    IndexDegradation,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Critical alert
    Critical,
    /// Emergency alert
    Emergency,
}

/// Performance report structure
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// Report generation timestamp
    pub report_timestamp_us: u64,
    /// Summary statistics
    pub summary_stats: PerformanceSummary,
    /// Current real-time metrics
    pub realtime_metrics: RealTimeMetrics,
    /// Recent performance trends
    pub recent_trends: Vec<HistoricalTrend>,
    /// Active bottlenecks
    pub active_bottlenecks: Vec<PerformanceBottleneck>,
    /// Top optimization recommendations
    pub top_recommendations: Vec<OptimizationRecommendation>,
    /// Critical alerts
    pub critical_alerts: Vec<PerformanceAlert>,
}

/// Performance summary statistics
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    /// Total queries processed
    pub total_queries: u64,
    /// Average latency (microseconds)
    pub avg_latency_us: u64,
    /// 50th percentile latency (microseconds)
    pub p50_latency_us: u64,
    /// 95th percentile latency (microseconds)
    pub p95_latency_us: u64,
    /// 99th percentile latency (microseconds)
    pub p99_latency_us: u64,
    /// Average memory usage (bytes)
    pub avg_memory_usage_bytes: usize,
    /// Total alerts generated
    pub total_alerts: u64,
    /// Total bottlenecks identified
    pub total_bottlenecks: u64,
    /// Total recommendations generated
    pub total_recommendations: u64,
}
/// Main query performance monitor
pub struct QueryPerformanceMonitor {
    /// Monitoring configuration
    config: MonitoringConfig,
    /// Real-time metrics
    realtime_metrics: RealTimeMetrics,
    /// Historical performance data
    historical_data: Vec<HistoricalTrend>,
    /// Identified bottlenecks
    bottlenecks: Vec<PerformanceBottleneck>,
    /// Optimization recommendations
    recommendations: Vec<OptimizationRecommendation>,
    /// Active performance alerts
    active_alerts: Vec<PerformanceAlert>,
    /// Query analytics data
    query_analytics: BTreeMap<u64, QueryAnalytics>,
    /// Performance baselines for regression detection
    performance_baselines: BTreeMap<QueryPattern, PerformanceBaseline>,
    /// Active monitoring operations
    active_operations: BTreeMap<u64, MonitoringOperation>,
    /// Operation counter
    operation_counter: u64,
    /// Last metrics collection timestamp
    last_collection_timestamp_us: u64,
}

/// Performance baseline for regression detection
#[derive(Debug, Clone)]
struct PerformanceBaseline {
    /// Query pattern
    pattern: QueryPattern,
    /// Baseline latency (microseconds)
    baseline_latency_us: u64,
    /// Baseline memory usage (bytes)
    baseline_memory_bytes: usize,
    /// Baseline cache hit rate
    baseline_cache_hit_rate: f32,
    /// Sample count used for baseline
    sample_count: u64,
    /// Last updated timestamp
    last_updated_us: u64,
}

/// Monitoring operation metadata
#[derive(Debug, Clone)]
struct MonitoringOperation {
    /// Operation ID
    operation_id: u64,
    /// Start timestamp
    start_timestamp_us: u64,
    /// Query being monitored
    query_characteristics: QueryCharacteristics,
    /// Execution plan being monitored
    execution_plan: QueryExecutionPlan,
    /// Stage performance data
    stage_performances: Vec<StagePerformance>,
    /// Resource utilization tracking
    resource_tracking: ResourceUtilization,
    /// User context
    user_id: u32,
}

impl QueryPerformanceMonitor {
    /// Create new query performance monitor
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            realtime_metrics: RealTimeMetrics::default(),
            historical_data: Vec::new(),
            bottlenecks: Vec::new(),
            recommendations: Vec::new(),
            active_alerts: Vec::new(),
            query_analytics: BTreeMap::new(),
            performance_baselines: BTreeMap::new(),
            active_operations: BTreeMap::new(),
            operation_counter: 0,
            last_collection_timestamp_us: 0,
        }
    }

    /// Start monitoring a query operation
    pub fn start_query_monitoring(
        &mut self,
        context: &OperationContext,
        query_characteristics: QueryCharacteristics,
        execution_plan: QueryExecutionPlan,
    ) -> VexfsResult<u64> {
        if !self.config.enable_realtime_metrics {
            return Ok(0); // Monitoring disabled
        }

        self.operation_counter += 1;
        let operation_id = self.operation_counter;
        let start_timestamp = self.get_current_time_us();

        let monitoring_operation = MonitoringOperation {
            operation_id,
            start_timestamp_us: start_timestamp,
            query_characteristics,
            execution_plan,
            stage_performances: Vec::new(),
            resource_tracking: ResourceUtilization {
                memory: MemoryUtilization {
                    total_allocated_bytes: 0,
                    peak_usage_bytes: 0,
                    efficiency: 0.0,
                    fragmentation: 0.0,
                    cache_usage_bytes: 0,
                    index_usage_bytes: 0,
                },
                cpu: CpuUtilization {
                    total_cpu_time_us: 0,
                    simd_utilization: 0.0,
                    vectorization_efficiency: 0.0,
                    parallelization_efficiency: 0.0,
                },
                io: IoUtilization {
                    total_io_operations: 0,
                    read_operations: 0,
                    write_operations: 0,
                    throughput_bytes_per_second: 0,
                    avg_io_latency_us: 0,
                },
            },
            user_id: context.user.uid,
        };

        self.active_operations.insert(operation_id, monitoring_operation);
        self.realtime_metrics.active_query_count = self.active_operations.len();

        Ok(operation_id)
    }

    /// Record stage performance during query execution
    pub fn record_stage_performance(
        &mut self,
        operation_id: u64,
        stage_type: StageType,
        actual_time_us: u64,
        estimated_time_us: u64,
        memory_usage_bytes: usize,
    ) -> VexfsResult<()> {
        // Calculate CPU and I/O estimates before borrowing
        let cpu_utilization = (actual_time_us as f32 / 100000.0).min(1.0);
        let io_operations = match stage_type {
            StageType::CandidateGeneration => (memory_usage_bytes / 4096) as u64,
            StageType::DistanceComputation => 0,
            StageType::ResultFiltering => (memory_usage_bytes / 8192) as u64,
            _ => (memory_usage_bytes / 16384) as u64,
        };

        if let Some(operation) = self.active_operations.get_mut(&operation_id) {
            let performance_ratio = if estimated_time_us > 0 {
                actual_time_us as f32 / estimated_time_us as f32
            } else {
                1.0
            };

            let stage_performance = StagePerformance {
                stage_type,
                actual_time_us,
                estimated_time_us,
                performance_ratio,
                memory_usage_bytes,
                cpu_utilization,
                io_operations,
            };

            operation.stage_performances.push(stage_performance);
            
            // Update resource tracking
            operation.resource_tracking.memory.total_allocated_bytes += memory_usage_bytes;
            operation.resource_tracking.memory.peak_usage_bytes = 
                operation.resource_tracking.memory.peak_usage_bytes.max(memory_usage_bytes);
            operation.resource_tracking.cpu.total_cpu_time_us += actual_time_us;

            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Monitoring operation not found".to_string()))
        }
    }

    /// Complete query monitoring and generate analytics
    pub fn complete_query_monitoring(
        &mut self,
        operation_id: u64,
        results: &[ScoredResult],
        cache_stats: &CacheStatistics,
        search_stats: &SearchStatistics,
    ) -> VexfsResult<QueryAnalytics> {
        let operation = self.active_operations.remove(&operation_id)
            .ok_or_else(|| VexfsError::InvalidOperation("Monitoring operation not found".to_string()))?;

        let end_timestamp = self.get_current_time_us();
        let total_time_us = end_timestamp - operation.start_timestamp_us;

        // Update real-time metrics
        self.update_realtime_metrics(total_time_us, &operation, results.len(), cache_stats);

        // Generate query analytics
        let analytics = self.generate_query_analytics(&operation, results, cache_stats, search_stats)?;

        // Store historical data
        if self.config.enable_historical_analysis {
            self.store_historical_data(&operation, total_time_us, results.len());
        }

        // Detect performance regressions
        if self.config.enable_regression_detection {
            self.detect_performance_regression(&analytics, total_time_us)?;
        }

        // Generate optimization recommendations
        if self.config.enable_optimization_recommendations {
            self.generate_optimization_recommendations(&analytics)?;
        }

        // Check for performance alerts
        self.check_performance_alerts(total_time_us, &operation)?;

        // Store analytics
        self.query_analytics.insert(operation_id, analytics.clone());

        // Update active query count
        self.realtime_metrics.active_query_count = self.active_operations.len();

        Ok(analytics)
    }

    /// Get current real-time metrics
    pub fn get_realtime_metrics(&self) -> &RealTimeMetrics {
        &self.realtime_metrics
    }

    /// Get historical performance trends
    pub fn get_historical_trends(&self, time_range_us: u64) -> Vec<&HistoricalTrend> {
        let current_time = self.get_current_time_us();
        let cutoff_time = current_time.saturating_sub(time_range_us);

        self.historical_data
            .iter()
            .filter(|trend| trend.timestamp_us >= cutoff_time)
            .collect()
    }

    /// Get identified performance bottlenecks
    pub fn get_performance_bottlenecks(&self) -> &[PerformanceBottleneck] {
        &self.bottlenecks
    }

    /// Get optimization recommendations
    pub fn get_optimization_recommendations(&self) -> &[OptimizationRecommendation] {
        &self.recommendations
    }

    /// Get active performance alerts
    pub fn get_active_alerts(&self) -> &[PerformanceAlert] {
        &self.active_alerts
    }

    /// Get query analytics for a specific operation
    pub fn get_query_analytics(&self, operation_id: u64) -> Option<&QueryAnalytics> {
        self.query_analytics.get(&operation_id)
    }

    /// Generate comprehensive performance report
    pub fn generate_performance_report(&self) -> PerformanceReport {
        let current_time = self.get_current_time_us();
        
        // Calculate summary statistics
        let total_queries = self.historical_data.len() as u64;
        let avg_latency = if total_queries > 0 {
            self.historical_data.iter().map(|h| h.query_latency_us).sum::<u64>() / total_queries
        } else {
            0
        };

        let avg_memory_usage = if total_queries > 0 {
            self.historical_data.iter().map(|h| h.memory_usage_bytes).sum::<usize>() / total_queries as usize
        } else {
            0
        };

        // Calculate percentiles
        let mut latencies: Vec<u64> = self.historical_data.iter().map(|h| h.query_latency_us).collect();
        latencies.sort_unstable();
        
        let p50_latency = self.calculate_percentile(&latencies, 50.0);
        let p95_latency = self.calculate_percentile(&latencies, 95.0);
        let p99_latency = self.calculate_percentile(&latencies, 99.0);

        PerformanceReport {
            report_timestamp_us: current_time,
            summary_stats: PerformanceSummary {
                total_queries,
                avg_latency_us: avg_latency,
                p50_latency_us: p50_latency,
                p95_latency_us: p95_latency,
                p99_latency_us: p99_latency,
                avg_memory_usage_bytes: avg_memory_usage,
                total_alerts: self.active_alerts.len() as u64,
                total_bottlenecks: self.bottlenecks.len() as u64,
                total_recommendations: self.recommendations.len() as u64,
            },
            realtime_metrics: self.realtime_metrics.clone(),
            recent_trends: self.get_historical_trends(3600 * 1_000_000).into_iter().cloned().collect(), // Last hour
            active_bottlenecks: self.bottlenecks.clone(),
            top_recommendations: self.get_top_recommendations(5),
            critical_alerts: self.get_critical_alerts(),
        }
    }

    /// Clear historical data older than retention period
    pub fn cleanup_historical_data(&mut self) -> usize {
        let current_time = self.get_current_time_us();
        let cutoff_time = current_time.saturating_sub(self.config.historical_retention_period_us);
        
        let initial_count = self.historical_data.len();
        self.historical_data.retain(|trend| trend.timestamp_us >= cutoff_time);
        
        // Also cleanup old analytics data
        let analytics_to_remove: Vec<u64> = self.query_analytics
            .iter()
            .filter(|(_, analytics)| {
                // Remove analytics older than retention period
                // This is a simplified check - in practice, you'd store timestamps with analytics
                self.historical_data.iter().any(|trend| trend.timestamp_us < cutoff_time)
            })
            .map(|(&id, _)| id)
            .collect();
        
        for id in analytics_to_remove {
            self.query_analytics.remove(&id);
        }

        initial_count - self.historical_data.len()
    }
/// Update real-time metrics
    fn update_realtime_metrics(
        &mut self,
        query_time_us: u64,
        operation: &MonitoringOperation,
        result_count: usize,
        cache_stats: &CacheStatistics,
    ) {
        self.realtime_metrics.current_query_time_us = query_time_us;
        self.realtime_metrics.current_memory_usage_bytes = operation.resource_tracking.memory.peak_usage_bytes;
        self.realtime_metrics.current_cache_hit_rate = cache_stats.hit_rate;
        
        // Update running averages (simplified)
        let total_queries = self.historical_data.len() as u64 + 1;
        let total_time = self.historical_data.iter().map(|h| h.query_latency_us).sum::<u64>() + query_time_us;
        self.realtime_metrics.avg_query_latency_us = total_time / total_queries;
        
        // Update percentiles (simplified - would use proper streaming percentile calculation in production)
        let mut recent_latencies: Vec<u64> = self.historical_data
            .iter()
            .rev()
            .take(1000) // Last 1000 queries
            .map(|h| h.query_latency_us)
            .collect();
        recent_latencies.push(query_time_us);
        recent_latencies.sort_unstable();
        
        self.realtime_metrics.p95_query_latency_us = self.calculate_percentile(&recent_latencies, 95.0);
        self.realtime_metrics.p99_query_latency_us = self.calculate_percentile(&recent_latencies, 99.0);
    }

    /// Generate comprehensive query analytics
    fn generate_query_analytics(
        &self,
        operation: &MonitoringOperation,
        results: &[ScoredResult],
        cache_stats: &CacheStatistics,
        search_stats: &SearchStatistics,
    ) -> VexfsResult<QueryAnalytics> {
        // Classify query pattern
        let query_pattern = self.classify_query_pattern(&operation.query_characteristics);

        // Analyze index effectiveness
        let index_effectiveness = self.analyze_index_effectiveness(&operation.execution_plan, search_stats);

        // Analyze cache effectiveness
        let cache_effectiveness = self.analyze_cache_effectiveness(cache_stats);

        // Identify optimization opportunities
        let optimization_opportunities = self.identify_optimization_opportunities(
            &operation.query_characteristics,
            &operation.stage_performances,
            &index_effectiveness,
            &cache_effectiveness,
        );

        Ok(QueryAnalytics {
            query_pattern,
            stage_breakdown: operation.stage_performances.clone(),
            resource_utilization: operation.resource_tracking.clone(),
            index_effectiveness,
            cache_effectiveness,
            optimization_opportunities,
        })
    }

    /// Store historical performance data
    fn store_historical_data(&mut self, operation: &MonitoringOperation, total_time_us: u64, result_count: usize) {
        let trend = HistoricalTrend {
            timestamp_us: self.get_current_time_us(),
            query_latency_us: total_time_us,
            memory_usage_bytes: operation.resource_tracking.memory.peak_usage_bytes,
            cache_hit_rate: 0.0, // Would be updated with actual cache stats
            index_strategy: operation.execution_plan.index_recommendation.primary_strategy,
            query_complexity: operation.query_characteristics.complexity,
            result_count,
            user_id: operation.user_id,
        };

        self.historical_data.push(trend);

        // Limit historical data size
        if self.historical_data.len() > self.config.max_performance_samples {
            self.historical_data.remove(0);
        }
    }

    /// Detect performance regression
    fn detect_performance_regression(&mut self, analytics: &QueryAnalytics, total_time_us: u64) -> VexfsResult<()> {
        let pattern = analytics.query_pattern;
        let current_time = self.get_current_time_us();
        
        // Check if baseline exists
        let baseline_exists = self.performance_baselines.contains_key(&pattern);
        
        if !baseline_exists {
            // Create new baseline
            let new_baseline = PerformanceBaseline {
                pattern,
                baseline_latency_us: total_time_us,
                baseline_memory_bytes: analytics.resource_utilization.memory.peak_usage_bytes,
                baseline_cache_hit_rate: analytics.cache_effectiveness.hit_rate,
                sample_count: 1,
                last_updated_us: current_time,
            };
            self.performance_baselines.insert(pattern, new_baseline);
            return Ok(());
        }

        // Get baseline for comparison
        let baseline_latency = self.performance_baselines.get(&pattern)
            .map(|b| b.baseline_latency_us)
            .unwrap_or(total_time_us);

        // Check for regression
        let latency_increase_percent = if baseline_latency > 0 {
            ((total_time_us as f32 - baseline_latency as f32) / baseline_latency as f32) * 100.0
        } else {
            0.0
        };

        if latency_increase_percent > self.config.regression_threshold_percent {
            // Performance regression detected
            let alert = PerformanceAlert {
                alert_type: AlertType::PerformanceRegression,
                severity: if latency_increase_percent > 50.0 { AlertSeverity::Critical } else { AlertSeverity::Warning },
                message: format!("Performance regression detected for {:?} queries: {:.1}% increase in latency", pattern, latency_increase_percent),
                affected_metric: "query_latency".to_string(),
                current_value: total_time_us as f64,
                threshold_value: baseline_latency as f64,
                first_triggered_us: current_time,
                last_triggered_us: current_time,
                trigger_count: 1,
            };

            self.active_alerts.push(alert);
        } else {
            // Update baseline with exponential moving average
            if let Some(baseline) = self.performance_baselines.get_mut(&pattern) {
                let alpha = 0.1; // Smoothing factor
                baseline.baseline_latency_us = ((1.0 - alpha) * baseline.baseline_latency_us as f32 + alpha * total_time_us as f32) as u64;
                baseline.sample_count += 1;
                baseline.last_updated_us = current_time;
            }
        }

        Ok(())
    }

    /// Generate optimization recommendations
    fn generate_optimization_recommendations(&mut self, analytics: &QueryAnalytics) -> VexfsResult<()> {
        let mut new_recommendations = Vec::new();

        // Index optimization recommendations
        if analytics.index_effectiveness.hit_rate < 0.8 {
            new_recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::IndexOptimization,
                priority: 0.8,
                expected_improvement_percent: 25.0,
                implementation_complexity: 0.6,
                description: "Index hit rate is low. Consider index strategy optimization.".to_string(),
                actions: vec![
                    "Analyze query patterns for better index selection".to_string(),
                    "Consider hybrid indexing strategies".to_string(),
                    "Evaluate index parameters tuning".to_string(),
                ],
                estimated_implementation_time_us: 3600 * 1_000_000, // 1 hour
            });
        }

        // Cache optimization recommendations
        if analytics.cache_effectiveness.hit_rate < 0.6 {
            new_recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::CacheOptimization,
                priority: 0.7,
                expected_improvement_percent: 30.0,
                implementation_complexity: 0.4,
                description: "Cache hit rate is low. Consider cache configuration tuning.".to_string(),
                actions: vec![
                    "Increase cache size if memory allows".to_string(),
                    "Adjust cache TTL settings".to_string(),
                    "Implement cache warming strategies".to_string(),
                ],
                estimated_implementation_time_us: 1800 * 1_000_000, // 30 minutes
            });
        }

        // Memory optimization recommendations
        if analytics.resource_utilization.memory.efficiency < 0.7 {
            new_recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::MemoryOptimization,
                priority: 0.6,
                expected_improvement_percent: 15.0,
                implementation_complexity: 0.5,
                description: "Memory efficiency is low. Consider memory allocation optimization.".to_string(),
                actions: vec![
                    "Implement memory pooling".to_string(),
                    "Optimize data structures".to_string(),
                    "Reduce memory fragmentation".to_string(),
                ],
                estimated_implementation_time_us: 7200 * 1_000_000, // 2 hours
            });
        }

        // SIMD optimization recommendations
        if analytics.resource_utilization.cpu.simd_utilization < 0.5 {
            new_recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::SimdOptimization,
                priority: 0.5,
                expected_improvement_percent: 20.0,
                implementation_complexity: 0.7,
                description: "SIMD utilization is low. Consider vectorization optimization.".to_string(),
                actions: vec![
                    "Enable SIMD instructions for vector operations".to_string(),
                    "Optimize memory layout for SIMD".to_string(),
                    "Use vectorized distance calculations".to_string(),
                ],
                estimated_implementation_time_us: 5400 * 1_000_000, // 1.5 hours
            });
        }

        // Add new recommendations, avoiding duplicates
        for new_rec in new_recommendations {
            if !self.recommendations.iter().any(|existing| {
                existing.recommendation_type == new_rec.recommendation_type &&
                existing.description == new_rec.description
            }) {
                self.recommendations.push(new_rec);
            }
        }

        // Limit recommendations count
        if self.recommendations.len() > 20 {
            self.recommendations.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(core::cmp::Ordering::Equal));
            self.recommendations.truncate(20);
        }

        Ok(())
    }

    /// Check for performance alerts
    fn check_performance_alerts(&mut self, total_time_us: u64, operation: &MonitoringOperation) -> VexfsResult<()> {
        let current_time = self.get_current_time_us();
        
        // High latency alert
        if total_time_us > self.config.latency_alert_threshold_us {
            let alert = PerformanceAlert {
                alert_type: AlertType::HighLatency,
                severity: if total_time_us > self.config.latency_alert_threshold_us * 2 {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Warning
                },
                message: format!("High query latency detected: {}μs", total_time_us),
                affected_metric: "query_latency".to_string(),
                current_value: total_time_us as f64,
                threshold_value: self.config.latency_alert_threshold_us as f64,
                first_triggered_us: current_time,
                last_triggered_us: current_time,
                trigger_count: 1,
            };

            self.active_alerts.push(alert);
        }

        // Memory pressure alert
        if operation.resource_tracking.memory.peak_usage_bytes > self.config.memory_alert_threshold_bytes {
            let alert = PerformanceAlert {
                alert_type: AlertType::MemoryPressure,
                severity: AlertSeverity::Warning,
                message: format!("High memory usage detected: {} bytes", operation.resource_tracking.memory.peak_usage_bytes),
                affected_metric: "memory_usage".to_string(),
                current_value: operation.resource_tracking.memory.peak_usage_bytes as f64,
                threshold_value: self.config.memory_alert_threshold_bytes as f64,
                first_triggered_us: current_time,
                last_triggered_us: current_time,
                trigger_count: 1,
            };

            self.active_alerts.push(alert);
        }

        // Limit alerts count
        if self.active_alerts.len() > 50 {
            self.active_alerts.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap_or(core::cmp::Ordering::Equal));
            self.active_alerts.truncate(50);
        }

        Ok(())
    }

    /// Classify query pattern based on characteristics
    fn classify_query_pattern(&self, characteristics: &QueryCharacteristics) -> QueryPattern {
        match characteristics.complexity {
            QueryComplexity::Simple => {
                if characteristics.approximate_acceptable {
                    QueryPattern::SimpleApproximate
                } else {
                    QueryPattern::SimpleExact
                }
            }
            QueryComplexity::Moderate => {
                if characteristics.has_filters {
                    QueryPattern::ComplexFiltered
                } else if characteristics.sparsity > 0.7 {
                    QueryPattern::SparseVector
                } else {
                    QueryPattern::SimpleApproximate
                }
            }
            QueryComplexity::Complex | QueryComplexity::HighlyComplex => {
                if characteristics.dimensions > 1024 {
                    QueryPattern::HighDimensional
                } else if characteristics.k > 100 {
                    QueryPattern::BatchSearch
                } else {
                    QueryPattern::RareComplex
                }
            }
        }
    }

    /// Analyze index effectiveness
    fn analyze_index_effectiveness(&self, execution_plan: &QueryExecutionPlan, search_stats: &SearchStatistics) -> IndexEffectiveness {
        let strategy = execution_plan.index_recommendation.primary_strategy;
        
        // Calculate effectiveness metrics based on execution plan and statistics
        let hit_rate = execution_plan.index_recommendation.confidence;
        let traversal_efficiency = 1.0 - (execution_plan.estimated_time_us as f32 / 100000.0).min(1.0);
        let memory_efficiency = if execution_plan.memory_estimate > 0 {
            (execution_plan.memory_estimate as f32 / (execution_plan.memory_estimate as f32 * 1.5)).min(1.0)
        } else {
            1.0
        };

        IndexEffectiveness {
            strategy,
            hit_rate,
            traversal_efficiency,
            memory_efficiency,
            build_time_us: 1000, // Placeholder
            update_frequency: 0.1, // Placeholder
        }
    }

    /// Analyze cache effectiveness
    fn analyze_cache_effectiveness(&self, cache_stats: &CacheStatistics) -> CacheEffectiveness {
        CacheEffectiveness {
            hit_rate: cache_stats.hit_rate,
            memory_efficiency: cache_stats.memory_efficiency,
            eviction_rate: if cache_stats.insertions > 0 {
                cache_stats.evictions as f32 / cache_stats.insertions as f32
            } else {
                0.0
            },
            warming_effectiveness: 0.8, // Placeholder
            avg_lookup_time_us: 10, // Placeholder
        }
    }

    /// Identify optimization opportunities
    fn identify_optimization_opportunities(
        &self,
        characteristics: &QueryCharacteristics,
        stage_performances: &[StagePerformance],
        index_effectiveness: &IndexEffectiveness,
        cache_effectiveness: &CacheEffectiveness,
    ) -> Vec<OptimizationOpportunity> {
        let mut opportunities = Vec::new();

        // Index optimization opportunity
        if index_effectiveness.hit_rate < 0.8 {
            opportunities.push(OptimizationOpportunity {
                opportunity_type: RecommendationType::IndexOptimization,
                potential_improvement_percent: 25.0,
                confidence: 0.8,
                description: "Index strategy could be optimized for better performance".to_string(),
                required_changes: vec![
                    "Analyze query patterns".to_string(),
                    "Tune index parameters".to_string(),
                ],
            });
        }

        // Cache optimization opportunity
        if cache_effectiveness.hit_rate < 0.6 {
            opportunities.push(OptimizationOpportunity {
                opportunity_type: RecommendationType::CacheOptimization,
                potential_improvement_percent: 30.0,
                confidence: 0.7,
                description: "Cache configuration could be improved".to_string(),
                required_changes: vec![
                    "Increase cache size".to_string(),
                    "Adjust TTL settings".to_string(),
                ],
            });
        }

        // Query optimization opportunity
        if characteristics.complexity == QueryComplexity::HighlyComplex {
            opportunities.push(OptimizationOpportunity {
                opportunity_type: RecommendationType::QueryOptimization,
                potential_improvement_percent: 20.0,
                confidence: 0.6,
                description: "Query parameters could be optimized".to_string(),
                required_changes: vec![
                    "Adjust expansion factor".to_string(),
                    "Enable approximation".to_string(),
                ],
            });
        }

        opportunities
    }

    /// Get top recommendations by priority
    fn get_top_recommendations(&self, count: usize) -> Vec<OptimizationRecommendation> {
        let mut sorted_recommendations = self.recommendations.clone();
        sorted_recommendations.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(core::cmp::Ordering::Equal));
        sorted_recommendations.into_iter().take(count).collect()
    }

    /// Get critical alerts
    fn get_critical_alerts(&self) -> Vec<PerformanceAlert> {
        self.active_alerts
            .iter()
            .filter(|alert| alert.severity >= AlertSeverity::Critical)
            .cloned()
            .collect()
    }

    /// Calculate percentile from sorted data
    fn calculate_percentile(&self, sorted_data: &[u64], percentile: f32) -> u64 {
        if sorted_data.is_empty() {
            return 0;
        }

        let index = (percentile / 100.0 * (sorted_data.len() - 1) as f32) as usize;
        sorted_data.get(index).copied().unwrap_or(0)
    }

    /// Estimate CPU utilization based on execution time
    fn estimate_cpu_utilization(&self, execution_time_us: u64) -> f32 {
        // Simplified estimation - in practice, this would use actual CPU metrics
        (execution_time_us as f32 / 100000.0).min(1.0)
    }

    /// Estimate I/O operations based on stage type and memory usage
    fn estimate_io_operations(&self, stage_type: StageType, memory_usage_bytes: usize) -> u64 {
        match stage_type {
            StageType::CandidateGeneration => (memory_usage_bytes / 4096) as u64, // Assume 4KB pages
            StageType::DistanceComputation => 0, // Mostly CPU-bound
            StageType::ResultFiltering => (memory_usage_bytes / 8192) as u64,
            _ => (memory_usage_bytes / 16384) as u64,
        }
    }

    /// Get current time in microseconds (placeholder)
    fn get_current_time_us(&self) -> u64 {
        1640995200_000_000 // Placeholder timestamp
    }
}

/// Integration trait for connecting monitoring with existing components
pub trait MonitoringIntegration {
    /// Integrate with query planner
    fn integrate_with_query_planner(&mut self, planner: &mut QueryPlanner) -> VexfsResult<()>;
    
    /// Integrate with search cache
    fn integrate_with_search_cache(&mut self, cache: &mut SearchResultCache) -> VexfsResult<()>;
    
    /// Integrate with vector search subsystem
    fn integrate_with_search_subsystem(&mut self, subsystem: &mut VectorSearchSubsystem) -> VexfsResult<()>;
}

impl MonitoringIntegration for QueryPerformanceMonitor {
    fn integrate_with_query_planner(&mut self, _planner: &mut QueryPlanner) -> VexfsResult<()> {
        // Integration logic would be implemented here
        // This would involve setting up callbacks or hooks in the query planner
        // to automatically start/stop monitoring operations
        Ok(())
    }
    
    fn integrate_with_search_cache(&mut self, _cache: &mut SearchResultCache) -> VexfsResult<()> {
        // Integration logic would be implemented here
        // This would involve monitoring cache operations and collecting metrics
        Ok(())
    }
    
    fn integrate_with_search_subsystem(&mut self, _subsystem: &mut VectorSearchSubsystem) -> VexfsResult<()> {
        // Integration logic would be implemented here
        // This would involve monitoring ioctl operations and system health
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anns::DistanceMetric;

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enable_realtime_metrics);
        assert!(config.enable_historical_analysis);
        assert!(config.enable_regression_detection);
        assert_eq!(config.regression_threshold_percent, 20.0);
    }

    #[test]
    fn test_query_performance_monitor_creation() {
        let config = MonitoringConfig::default();
        let monitor = QueryPerformanceMonitor::new(config);
        assert_eq!(monitor.operation_counter, 0);
        assert!(monitor.active_operations.is_empty());
        assert!(monitor.historical_data.is_empty());
    }

    #[test]
    fn test_query_pattern_classification() {
        let config = MonitoringConfig::default();
        let monitor = QueryPerformanceMonitor::new(config);
        
        let simple_characteristics = QueryCharacteristics {
            dimensions: 128,
            sparsity: 0.1,
            magnitude: 1.0,
            entropy: 2.0,
            k: 10,
            metric: DistanceMetric::Euclidean,
            has_filters: false,
            filter_selectivity: 1.0,
            complexity: QueryComplexity::Simple,
            approximate_acceptable: true,
        };
        
        let pattern = monitor.classify_query_pattern(&simple_characteristics);
        assert_eq!(pattern, QueryPattern::SimpleApproximate);
    }

    #[test]
    fn test_percentile_calculation() {
        let config = MonitoringConfig::default();
        let monitor = QueryPerformanceMonitor::new(config);
        
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(monitor.calculate_percentile(&data, 50.0), 5);
        assert_eq!(monitor.calculate_percentile(&data, 90.0), 9);
        assert_eq!(monitor.calculate_percentile(&data, 100.0), 10);
    }

    #[test]
    fn test_bottleneck_type_variants() {
        // Test that all bottleneck types are properly defined
        let _types = [
            BottleneckType::HighLatency,
            BottleneckType::MemoryPressure,
            BottleneckType::CacheInefficiency,
            BottleneckType::IndexInefficiency,
            BottleneckType::IOBottleneck,
            BottleneckType::CPUBottleneck,
            BottleneckType::PlanningOverhead,
            BottleneckType::ResultProcessingBottleneck,
        ];
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Emergency > AlertSeverity::Critical);
        assert!(AlertSeverity::Critical > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }
}