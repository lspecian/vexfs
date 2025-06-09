//! Advanced Monitoring and Observability System for VexFS Semantic Event Propagation

use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{interval, timeout};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::semantic_api::types::{
    SemanticEvent, SemanticEventType, EventCategory, EventPriority,
    SemanticContext, SemanticResult, SemanticError
};

/// Configuration for the monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSystemConfig {
    /// Metrics collection settings
    pub metrics_collection_interval_ms: u64,
    pub metrics_retention_hours: u64,
    pub enable_prometheus_export: bool,
    pub enable_influxdb_export: bool,
    pub enable_custom_metrics: bool,
    
    /// Distributed tracing settings
    pub enable_distributed_tracing: bool,
    pub trace_sampling_rate: f64,
    pub max_trace_duration_ms: u64,
    pub trace_buffer_size: usize,
    
    /// Health monitoring settings
    pub health_check_interval_ms: u64,
    pub component_timeout_ms: u64,
    pub enable_auto_recovery: bool,
    pub max_recovery_attempts: u32,
    
    /// Alerting settings
    pub enable_alerting: bool,
    pub alert_cooldown_ms: u64,
    pub alert_channels: Vec<AlertChannel>,
    
    /// Performance settings
    pub max_concurrent_checks: usize,
    pub monitoring_overhead_limit_percent: f64,
    pub enable_adaptive_monitoring: bool,
}

impl Default for MonitoringSystemConfig {
    fn default() -> Self {
        Self {
            metrics_collection_interval_ms: 1000,
            metrics_retention_hours: 24,
            enable_prometheus_export: true,
            enable_influxdb_export: false,
            enable_custom_metrics: true,
            enable_distributed_tracing: true,
            trace_sampling_rate: 0.1,
            max_trace_duration_ms: 30000,
            trace_buffer_size: 10000,
            health_check_interval_ms: 5000,
            component_timeout_ms: 10000,
            enable_auto_recovery: true,
            max_recovery_attempts: 3,
            enable_alerting: true,
            alert_cooldown_ms: 300000,
            alert_channels: vec![AlertChannel::Log],
            max_concurrent_checks: 100,
            monitoring_overhead_limit_percent: 5.0,
            enable_adaptive_monitoring: true,
        }
    }
}

/// Alert channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannel {
    Log,
    Email { recipients: Vec<String> },
    Webhook { url: String, headers: HashMap<String, String> },
    Slack { webhook_url: String, channel: String },
    PagerDuty { integration_key: String },
    Custom { name: String, config: HashMap<String, String> },
}

/// Metric types for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
    Timer,
}

/// Metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub labels: Vec<String>,
    pub unit: Option<String>,
}

/// Metric value with timestamp and labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: SystemTime,
    pub labels: HashMap<String, String>,
}

/// Distributed trace span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub span_id: Uuid,
    pub trace_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub operation_name: String,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub duration_ms: Option<f64>,
    pub tags: HashMap<String, String>,
    pub logs: Vec<SpanLog>,
    pub status: SpanStatus,
}

/// Span log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLog {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub message: String,
    pub fields: HashMap<String, String>,
}

/// Span status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error { message: String },
    Timeout,
    Cancelled,
}

/// Log levels for span logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub component_name: String,
    pub status: HealthStatus,
    pub timestamp: SystemTime,
    pub response_time_ms: f64,
    pub details: HashMap<String, String>,
    pub dependencies: Vec<DependencyHealth>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}

/// Dependency health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealth {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: SystemTime,
}

/// Alert definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: Uuid,
    pub name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub condition: AlertCondition,
    pub timestamp: SystemTime,
    pub resolved: bool,
    pub resolution_time: Option<SystemTime>,
    pub metadata: HashMap<String, String>,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    MetricThreshold {
        metric_name: String,
        operator: ComparisonOperator,
        threshold: f64,
        duration_ms: u64,
    },
    HealthCheck {
        component_name: String,
        status: HealthStatus,
    },
    ErrorRate {
        threshold_percent: f64,
        window_ms: u64,
    },
    Custom {
        name: String,
        expression: String,
    },
}

/// Comparison operators for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Monitoring system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringMetrics {
    pub metrics_collected: u64,
    pub traces_recorded: u64,
    pub health_checks_performed: u64,
    pub alerts_triggered: u64,
    pub monitoring_overhead_percent: f64,
    pub average_collection_latency_ms: f64,
    pub active_traces: u64,
    pub active_alerts: u64,
    pub last_updated: SystemTime,
}

impl Default for MonitoringMetrics {
    fn default() -> Self {
        Self {
            metrics_collected: 0,
            traces_recorded: 0,
            health_checks_performed: 0,
            alerts_triggered: 0,
            monitoring_overhead_percent: 0.0,
            average_collection_latency_ms: 0.0,
            active_traces: 0,
            active_alerts: 0,
            last_updated: SystemTime::now(),
        }
    }
}

/// Main monitoring system
pub struct MonitoringSystem {
    config: MonitoringSystemConfig,
    
    // Metrics collection
    metrics_registry: Arc<RwLock<HashMap<String, MetricDefinition>>>,
    metrics_storage: Arc<RwLock<VecDeque<MetricValue>>>,
    metrics_aggregator: Arc<MetricsAggregator>,
    
    // Distributed tracing
    trace_collector: Arc<TraceCollector>,
    active_traces: Arc<RwLock<HashMap<Uuid, TraceSpan>>>,
    completed_traces: Arc<RwLock<VecDeque<TraceSpan>>>,
    
    // Health monitoring
    health_checker: Arc<HealthChecker>,
    component_registry: Arc<RwLock<HashMap<String, Box<dyn HealthCheckProvider + Send + Sync>>>>,
    health_results: Arc<RwLock<HashMap<String, HealthCheckResult>>>,
    
    // Alerting
    alert_manager: Arc<AlertManager>,
    active_alerts: Arc<RwLock<HashMap<Uuid, Alert>>>,
    alert_history: Arc<RwLock<VecDeque<Alert>>>,
    
    // Exporters
    prometheus_exporter: Option<Arc<PrometheusExporter>>,
    influxdb_exporter: Option<Arc<InfluxDBExporter>>,
    
    // Performance monitoring
    monitoring_metrics: Arc<RwLock<MonitoringMetrics>>,
    performance_tracker: Arc<PerformanceTracker>,
    
    // Control channels
    shutdown_sender: Option<broadcast::Sender<()>>,
    metric_sender: mpsc::UnboundedSender<MetricValue>,
    trace_sender: mpsc::UnboundedSender<TraceSpan>,
    alert_sender: broadcast::Sender<Alert>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(config: MonitoringSystemConfig) -> SemanticResult<Self> {
        let (metric_sender, _) = mpsc::unbounded_channel();
        let (trace_sender, _) = mpsc::unbounded_channel();
        let (alert_sender, _) = broadcast::channel(1000);
        
        let prometheus_exporter = if config.enable_prometheus_export {
            Some(Arc::new(PrometheusExporter::new()?))
        } else {
            None
        };
        
        let influxdb_exporter = if config.enable_influxdb_export {
            Some(Arc::new(InfluxDBExporter::new()?))
        } else {
            None
        };
        
        Ok(Self {
            metrics_aggregator: Arc::new(MetricsAggregator::new(config.clone())),
            trace_collector: Arc::new(TraceCollector::new(config.clone())),
            health_checker: Arc::new(HealthChecker::new(config.clone())),
            alert_manager: Arc::new(AlertManager::new(config.clone())),
            metrics_registry: Arc::new(RwLock::new(HashMap::new())),
            metrics_storage: Arc::new(RwLock::new(VecDeque::new())),
            active_traces: Arc::new(RwLock::new(HashMap::new())),
            completed_traces: Arc::new(RwLock::new(VecDeque::new())),
            component_registry: Arc::new(RwLock::new(HashMap::new())),
            health_results: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            prometheus_exporter,
            influxdb_exporter,
            monitoring_metrics: Arc::new(RwLock::new(MonitoringMetrics::default())),
            performance_tracker: Arc::new(PerformanceTracker::new()),
            shutdown_sender: None,
            metric_sender,
            trace_sender,
            alert_sender,
            config,
        })
    }
    
    /// Start the monitoring system
    pub async fn start(&mut self) -> SemanticResult<()> {
        let (shutdown_sender, _) = broadcast::channel(1);
        self.shutdown_sender = Some(shutdown_sender.clone());
        
        // Start monitoring workers
        self.start_metrics_collection_worker(shutdown_sender.subscribe()).await?;
        self.start_trace_collection_worker(shutdown_sender.subscribe()).await?;
        self.start_health_monitoring_worker(shutdown_sender.subscribe()).await?;
        self.start_alert_processing_worker(shutdown_sender.subscribe()).await?;
        self.start_export_workers(shutdown_sender.subscribe()).await?;
        self.start_performance_monitoring_worker(shutdown_sender.subscribe()).await?;
        self.start_cleanup_worker(shutdown_sender.subscribe()).await?;
        
        Ok(())
    }
    
    /// Stop the monitoring system
    pub async fn stop(&self) -> SemanticResult<()> {
        if let Some(sender) = &self.shutdown_sender {
            let _ = sender.send(());
        }
        Ok(())
    }
    
    /// Register a metric definition
    pub async fn register_metric(&self, metric: MetricDefinition) -> SemanticResult<()> {
        let mut registry = self.metrics_registry.write().unwrap();
        registry.insert(metric.name.clone(), metric);
        Ok(())
    }
    
    /// Record a metric value
    pub async fn record_metric(&self, metric_value: MetricValue) -> SemanticResult<()> {
        // Validate metric exists
        {
            let registry = self.metrics_registry.read().unwrap();
            if !registry.contains_key(&metric_value.metric_name) {
                return Err(SemanticError::InvalidInput(
                    format!("Metric '{}' not registered", metric_value.metric_name)
                ));
            }
        }
        
        // Send metric for processing
        self.metric_sender.send(metric_value)
            .map_err(|e| SemanticError::ProcessingError(format!("Failed to send metric: {}", e)))?;
        
        // Update monitoring metrics
        {
            let mut metrics = self.monitoring_metrics.write().unwrap();
            metrics.metrics_collected += 1;
            metrics.last_updated = SystemTime::now();
        }
        
        Ok(())
    }
    
    /// Start a new trace span
    pub async fn start_span(&self, operation_name: String, parent_span_id: Option<Uuid>) -> SemanticResult<Uuid> {
        let span_id = Uuid::new_v4();
        let trace_id = parent_span_id
            .and_then(|parent_id| {
                let traces = self.active_traces.read().unwrap();
                traces.get(&parent_id).map(|span| span.trace_id)
            })
            .unwrap_or_else(Uuid::new_v4);
        
        let span = TraceSpan {
            span_id,
            trace_id,
            parent_span_id,
            operation_name,
            start_time: SystemTime::now(),
            end_time: None,
            duration_ms: None,
            tags: HashMap::new(),
            logs: Vec::new(),
            status: SpanStatus::Ok,
        };
        
        // Store active span
        {
            let mut active_traces = self.active_traces.write().unwrap();
            active_traces.insert(span_id, span);
        }
        
        // Update monitoring metrics
        {
            let mut metrics = self.monitoring_metrics.write().unwrap();
            metrics.active_traces += 1;
            metrics.last_updated = SystemTime::now();
        }
        
        Ok(span_id)
    }
    
    /// Finish a trace span
    pub async fn finish_span(&self, span_id: Uuid) -> SemanticResult<()> {
        let span = {
            let mut active_traces = self.active_traces.write().unwrap();
            active_traces.remove(&span_id)
        };
        
        if let Some(mut span) = span {
            span.end_time = Some(SystemTime::now());
            span.duration_ms = span.end_time
                .and_then(|end| end.duration_since(span.start_time).ok())
                .map(|duration| duration.as_millis() as f64);
            
            // Send span for processing
            self.trace_sender.send(span)
                .map_err(|e| SemanticError::ProcessingError(format!("Failed to send trace: {}", e)))?;
            
            // Update monitoring metrics
            {
                let mut metrics = self.monitoring_metrics.write().unwrap();
                metrics.active_traces = metrics.active_traces.saturating_sub(1);
                metrics.traces_recorded += 1;
                metrics.last_updated = SystemTime::now();
            }
        }
        
        Ok(())
    }
    
    /// Add a tag to a trace span
    pub async fn add_span_tag(&self, span_id: Uuid, key: String, value: String) -> SemanticResult<()> {
        let mut active_traces = self.active_traces.write().unwrap();
        if let Some(span) = active_traces.get_mut(&span_id) {
            span.tags.insert(key, value);
        }
        Ok(())
    }
    
    /// Add a log to a trace span
    pub async fn add_span_log(&self, span_id: Uuid, level: LogLevel, message: String, fields: HashMap<String, String>) -> SemanticResult<()> {
        let mut active_traces = self.active_traces.write().unwrap();
        if let Some(span) = active_traces.get_mut(&span_id) {
            span.logs.push(SpanLog {
                timestamp: SystemTime::now(),
                level,
                message,
                fields,
            });
        }
        Ok(())
    }
    
    /// Register a health check provider
    pub async fn register_health_check(&self, name: String, provider: Box<dyn HealthCheckProvider + Send + Sync>) -> SemanticResult<()> {
        let mut registry = self.component_registry.write().unwrap();
        registry.insert(name, provider);
        Ok(())
    }
    
    /// Get current health status
    pub async fn get_health_status(&self) -> HashMap<String, HealthCheckResult> {
        self.health_results.read().unwrap().clone()
    }
    
    /// Get monitoring metrics
    pub async fn get_monitoring_metrics(&self) -> MonitoringMetrics {
        self.monitoring_metrics.read().unwrap().clone()
    }
    
    /// Subscribe to alerts
    pub fn subscribe_to_alerts(&self) -> broadcast::Receiver<Alert> {
        self.alert_sender.subscribe()
    }
    
    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().unwrap().values().cloned().collect()
    }
    
    async fn start_metrics_collection_worker(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        let metrics_storage = Arc::clone(&self.metrics_storage);
        let metrics_aggregator = Arc::clone(&self.metrics_aggregator);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut metric_receiver = {
                let (_, receiver) = mpsc::unbounded_channel();
                receiver
            };
            
            let mut collection_interval = interval(Duration::from_millis(config.metrics_collection_interval_ms));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,
                    metric = metric_receiver.recv() => {
                        if let Some(metric) = metric {
                            Self::process_metric(metric, &metrics_storage, &metrics_aggregator).await;
                        }
                    }
                    _ = collection_interval.tick() => {
                        Self::collect_system_metrics(&metrics_storage).await;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn process_metric(
        metric: MetricValue,
        storage: &Arc<RwLock<VecDeque<MetricValue>>>,
        _aggregator: &Arc<MetricsAggregator>,
    ) {
        let mut metrics = storage.write().unwrap();
        metrics.push_back(metric);
        
        // Maintain storage size
        if metrics.len() > 100000 {
            metrics.pop_front();
        }
    }
    
    async fn collect_system_metrics(storage: &Arc<RwLock<VecDeque<MetricValue>>>) {
        let timestamp = SystemTime::now();
        
        // Collect system metrics
        let cpu_usage = Self::get_cpu_usage();
        let memory_usage = Self::get_memory_usage();
        let disk_usage = Self::get_disk_usage();
        
        let system_metrics = vec![
            MetricValue {
                metric_name: "system_cpu_usage_percent".to_string(),
                value: cpu_usage,
                timestamp,
                labels: HashMap::new(),
            },
            MetricValue {
                metric_name: "system_memory_usage_mb".to_string(),
                value: memory_usage,
                timestamp,
                labels: HashMap::new(),
            },
            MetricValue {
                metric_name: "system_disk_usage_mb".to_string(),
                value: disk_usage,
                timestamp,
                labels: HashMap::new(),
            },
        ];
        
        let mut storage = storage.write().unwrap();
        for metric in system_metrics {
            storage.push_back(metric);
        }
    }
    
    async fn start_trace_collection_worker(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        let completed_traces = Arc::clone(&self.completed_traces);
        let trace_collector = Arc::clone(&self.trace_collector);
        
        tokio::spawn(async move {
            let mut trace_receiver = {
                let (_, receiver) = mpsc::unbounded_channel();
                receiver
            };
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,
                    trace = trace_receiver.recv() => {
                        if let Some(trace) = trace {
                            Self::process_trace(trace, &completed_traces, &trace_collector).await;
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn process_trace(
        trace: TraceSpan,
        storage: &Arc<RwLock<VecDeque<TraceSpan>>>,
        _collector: &Arc<TraceCollector>,
    ) {
        let mut traces = storage.write().unwrap();
        traces.push_back(trace);
        
        // Maintain storage size
        if traces.len() > 10000 {
            traces.pop_front();
        }
    }
    
    async fn start_health_monitoring_worker(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        let component_registry = Arc::clone(&self.component_registry);
        let health_results = Arc::clone(&self.health_results);
        let health_checker = Arc::clone(&self.health_checker);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut health_interval = interval(Duration::from_millis(config.health_check_interval_ms));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,
                    _ = health_interval.tick() => {
                        Self::perform_health_checks(&component_registry, &health_results, &health_checker).await;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn perform_health_checks(
        registry: &Arc<RwLock<HashMap<String, Box<dyn HealthCheckProvider + Send + Sync>>>>,
        results: &Arc<RwLock<HashMap<String, HealthCheckResult>>>,
        _checker: &Arc<HealthChecker>,
    ) {
        let components = {
            let registry = registry.read().unwrap();
            registry.keys().cloned().collect::<Vec<_>>()
        };
        
        for component_name in components {
            let start_time = Instant::now();
            
            // Perform health check (simplified)
            let status = HealthStatus::Healthy;
            let response_time = start_time.elapsed().as_millis() as f64;
            
            let result = HealthCheckResult {
                component_name: component_name.clone(),
                status,
                timestamp: SystemTime::now(),
                response_time_ms: response_time,
                details: HashMap::new(),
                dependencies: Vec::new(),
            };
            
            let mut results = results.write().unwrap();
            results.insert(component_name, result);
        }
    }
    
    async fn start_alert_processing_worker(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        let active_alerts = Arc::clone(&self.active_alerts);
        let alert_history = Arc::clone(&self.alert_history);
        let alert_manager = Arc::clone(&self.alert_manager);
        let alert_sender = self.alert_sender.clone();
        
        tokio::spawn(async move {
            let mut alert_interval = interval(Duration::from_secs(10));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,
                    _ = alert_interval.tick() => {
                        Self::process_alerts(&active_alerts, &alert_history, &alert_manager, &alert_sender).await;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn process_alerts(
        _active_alerts: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        _alert_history: &Arc<RwLock<VecDeque<Alert>>>,
        _alert_manager: &Arc<AlertManager>,
        _alert_sender: &broadcast::Sender<Alert>,
    ) {
        // Alert processing logic would go here
        // This would evaluate alert conditions and trigger alerts
    }
    
    async fn start_export_workers(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        if let Some(prometheus_exporter) = &self.prometheus_exporter {
            let exporter = Arc::clone(prometheus_exporter);
            let metrics_storage = Arc::clone(&self.metrics_storage);
            
            tokio::spawn(async move {
                let mut export_interval = interval(Duration::from_secs(15));
                
                loop {
                    tokio::select! {
                        _ = shutdown.recv() => break,
                        _ = export_interval.tick() => {
                            exporter.export_metrics(&metrics_storage).await;
                        }
                    }
                }
            });
        }
        
        if let Some(influxdb_exporter) = &self.influxdb_exporter {
            let exporter = Arc::clone(influxdb_exporter);
            let metrics_storage = Arc::clone(&self.metrics_storage);
            
            tokio::spawn(async move {
                let mut export_interval = interval(Duration::from_secs(30));
                
                loop {
                    tokio::select! {
                        _ = shutdown.recv() => break,
                        _ = export_interval.tick() => {
                            exporter.export_metrics(&metrics_storage).await;
                        }
                    }
                }
            });
        }
        
        Ok(())
    }
    
    async fn start_performance_monitoring_worker(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        let monitoring_metrics = Arc::clone(&self.monitoring_metrics);
        let performance_tracker = Arc::clone(&self.performance_tracker);
        
        tokio::spawn(async move {
            let mut perf_interval = interval(Duration::from_secs(5));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,
                    _ = perf_interval.tick() => {
                        let overhead = Self::calculate_monitoring_overhead();
                        performance_tracker.record_overhead(overhead);
                        
                        {
                            let mut metrics = monitoring_metrics.write().unwrap();
                            metrics.monitoring_overhead_percent = overhead;
                            metrics.last_updated = SystemTime::now();
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn start_cleanup_worker(&self, mut shutdown: broadcast::Receiver<()>) -> SemanticResult<()> {
        let metrics_storage = Arc::clone(&self.metrics_storage);
        let completed_traces = Arc::clone(&self.completed_traces);
        let alert_history = Arc::clone(&self.alert_history);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(3600)); // Cleanup every hour
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,
                    _ = cleanup_interval.tick() => {
                        Self::cleanup_old_data(&metrics_storage, &completed_traces, &alert_history, &config).await;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn cleanup_old_data(
        metrics_storage: &Arc<RwLock<VecDeque<MetricValue>>>,
        completed_traces: &Arc<RwLock<VecDeque<TraceSpan>>>,
        alert_history: &Arc<RwLock<VecDeque<Alert>>>,
        config: &MonitoringSystemConfig,
    ) {
        let cutoff_time = SystemTime::now() - Duration::from_secs(config.metrics_retention_hours * 3600);
        
        // Cleanup old metrics
        {
            let mut metrics = metrics_storage.write().unwrap();
            while let Some(metric) = metrics.front() {
                if metric.timestamp < cutoff_time {
                    metrics.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Cleanup old traces
        {
            let mut traces = completed_traces.write().unwrap();
            while let Some(trace) = traces.front() {
                if trace.start_time < cutoff_time {
                    traces.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Cleanup old alerts
        {
            let mut alerts = alert_history.write().unwrap();
            while let Some(alert) = alerts.front() {
                if alert.timestamp < cutoff_time {
                    alerts.pop_front();
                } else {
                    break;
                }
            }
        }
    }
    
    fn get_