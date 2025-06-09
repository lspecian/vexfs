//! Operational Monitoring Dashboard with Predictive Analytics

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::semantic_api::{
    types::{SemanticEvent, SemanticEventType, EventCategory},
    event_analytics_engine::{
        EventAnalyticsEngine, AnalyticsConfig, StreamStatistics, EventPattern, 
        Anomaly, Prediction, AnalyticsPerformanceMetrics, SystemHealth
    },
    SemanticResult, SemanticError,
};

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub refresh_interval_ms: u64,
    pub max_chart_points: usize,
    pub alert_thresholds: AlertThresholds,
    pub enable_predictive_charts: bool,
    pub enable_real_time_updates: bool,
    pub retention_hours: u64,
    pub max_alerts: usize,
    pub dashboard_port: u16,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 1000, // 1 second
            max_chart_points: 1000,
            alert_thresholds: AlertThresholds::default(),
            enable_predictive_charts: true,
            enable_real_time_updates: true,
            retention_hours: 24,
            max_alerts: 100,
            dashboard_port: 8080,
        }
    }
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub latency_warning_ms: u64,
    pub latency_critical_ms: u64,
    pub throughput_warning_eps: f64,
    pub throughput_critical_eps: f64,
    pub error_rate_warning_percent: f64,
    pub error_rate_critical_percent: f64,
    pub memory_warning_mb: f64,
    pub memory_critical_mb: f64,
    pub cpu_warning_percent: f64,
    pub cpu_critical_percent: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            latency_warning_ms: 10,
            latency_critical_ms: 50,
            throughput_warning_eps: 100.0,
            throughput_critical_eps: 10.0,
            error_rate_warning_percent: 1.0,
            error_rate_critical_percent: 5.0,
            memory_warning_mb: 1000.0,
            memory_critical_mb: 2000.0,
            cpu_warning_percent: 80.0,
            cpu_critical_percent: 95.0,
        }
    }
}

/// Dashboard alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub alert_id: Uuid,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub triggered_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
    pub metric_value: f64,
    pub threshold_value: f64,
    pub component: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    Latency,
    Throughput,
    ErrorRate,
    Memory,
    CPU,
    Anomaly,
    Prediction,
    SystemHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Time-series chart data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_id: String,
    pub title: String,
    pub chart_type: ChartType,
    pub data_points: VecDeque<ChartPoint>,
    pub prediction_points: VecDeque<ChartPoint>,
    pub y_axis_label: String,
    pub unit: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Line,
    Area,
    Bar,
    Scatter,
    Heatmap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub metadata: HashMap<String, String>,
}

/// Dashboard widget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub widget_id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub size: WidgetSize,
    pub data: WidgetData,
    pub refresh_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetType {
    Metric,
    Chart,
    Alert,
    Table,
    Gauge,
    Progress,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetData {
    Metric(MetricData),
    Chart(ChartData),
    Alert(Vec<DashboardAlert>),
    Table(TableData),
    Gauge(GaugeData),
    Progress(ProgressData),
    Status(StatusData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricData {
    pub current_value: f64,
    pub previous_value: f64,
    pub change_percent: f64,
    pub trend: TrendDirection,
    pub unit: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub sortable: bool,
    pub filterable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaugeData {
    pub current_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub current: f64,
    pub total: f64,
    pub percentage: f64,
    pub status: ProgressStatus,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressStatus {
    InProgress,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    pub status: ComponentStatus,
    pub message: String,
    pub last_updated: SystemTime,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentStatus {
    Healthy,
    Warning,
    Critical,
    Offline,
}

/// Main monitoring dashboard
pub struct MonitoringDashboard {
    config: DashboardConfig,
    analytics_engine: Arc<EventAnalyticsEngine>,
    widgets: Arc<RwLock<HashMap<String, DashboardWidget>>>,
    charts: Arc<RwLock<HashMap<String, ChartData>>>,
    alerts: Arc<RwLock<HashMap<Uuid, DashboardAlert>>>,
    alert_manager: Arc<AlertManager>,
    chart_manager: Arc<ChartManager>,
    prediction_engine: Arc<PredictionEngine>,
    dashboard_state: Arc<RwLock<DashboardState>>,
    performance_metrics: Arc<Mutex<DashboardPerformanceMetrics>>,
    shutdown_signal: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

/// Alert management system
pub struct AlertManager {
    config: DashboardConfig,
    active_alerts: Arc<RwLock<HashMap<Uuid, DashboardAlert>>>,
    alert_history: Arc<RwLock<VecDeque<DashboardAlert>>>,
    notification_channels: Vec<Box<dyn NotificationChannel>>,
}

/// Chart management system
pub struct ChartManager {
    config: DashboardConfig,
    chart_data: Arc<RwLock<HashMap<String, ChartData>>>,
    chart_templates: Vec<ChartTemplate>,
}

/// Predictive analytics for dashboard
pub struct PredictionEngine {
    config: DashboardConfig,
    prediction_models: HashMap<String, PredictionModel>,
    forecast_cache: Arc<RwLock<HashMap<String, Vec<ChartPoint>>>>,
}

/// Dashboard state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardState {
    pub is_running: bool,
    pub last_updated: SystemTime,
    pub connected_clients: u32,
    pub total_events_processed: u64,
    pub system_health: SystemHealth,
    pub active_alerts_count: u32,
    pub performance_score: f64,
}

/// Dashboard performance metrics
#[derive(Debug, Clone)]
pub struct DashboardPerformanceMetrics {
    pub update_latency_ms: u64,
    pub render_time_ms: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub network_throughput_mbps: f64,
    pub client_connections: u32,
    pub updates_per_second: f64,
}

/// Notification channel trait
pub trait NotificationChannel: Send + Sync {
    fn send_alert(&self, alert: &DashboardAlert) -> Result<(), Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
}

/// Chart template for automatic chart generation
#[derive(Debug, Clone)]
pub struct ChartTemplate {
    pub template_id: String,
    pub title: String,
    pub chart_type: ChartType,
    pub metric_source: MetricSource,
    pub aggregation: AggregationType,
    pub time_window: Duration,
}

#[derive(Debug, Clone)]
pub enum MetricSource {
    Analytics,
    System,
    Custom(String),
}

#[derive(Debug, Clone)]
pub enum AggregationType {
    Average,
    Sum,
    Count,
    Min,
    Max,
    Percentile(f64),
}

/// Prediction model for forecasting
#[derive(Debug, Clone)]
pub struct PredictionModel {
    pub model_id: String,
    pub model_type: PredictionModelType,
    pub accuracy: f64,
    pub last_trained: SystemTime,
    pub parameters: Vec<f64>,
}

#[derive(Debug, Clone)]
pub enum PredictionModelType {
    LinearRegression,
    MovingAverage,
    ExponentialSmoothing,
    ARIMA,
    NeuralNetwork,
}

impl MonitoringDashboard {
    /// Create new monitoring dashboard
    pub fn new(
        config: DashboardConfig,
        analytics_engine: Arc<EventAnalyticsEngine>,
    ) -> SemanticResult<Self> {
        let alert_manager = Arc::new(AlertManager::new(config.clone())?);
        let chart_manager = Arc::new(ChartManager::new(config.clone())?);
        let prediction_engine = Arc::new(PredictionEngine::new(config.clone())?);

        Ok(Self {
            config: config.clone(),
            analytics_engine,
            widgets: Arc::new(RwLock::new(HashMap::new())),
            charts: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_manager,
            chart_manager,
            prediction_engine,
            dashboard_state: Arc::new(RwLock::new(DashboardState {
                is_running: false,
                last_updated: SystemTime::now(),
                connected_clients: 0,
                total_events_processed: 0,
                system_health: SystemHealth {
                    overall_score: 1.0,
                    latency_health: 1.0,
                    throughput_health: 1.0,
                    memory_health: 1.0,
                    error_rate_health: 1.0,
                },
                active_alerts_count: 0,
                performance_score: 1.0,
            })),
            performance_metrics: Arc::new(Mutex::new(DashboardPerformanceMetrics {
                update_latency_ms: 0,
                render_time_ms: 0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                network_throughput_mbps: 0.0,
                client_connections: 0,
                updates_per_second: 0.0,
            })),
            shutdown_signal: Arc::new(Mutex::new(None)),
        })
    }

    /// Start the monitoring dashboard
    pub async fn start(&self) -> SemanticResult<()> {
        let (shutdown_tx, _) = broadcast::channel(1);
        *self.shutdown_signal.lock().unwrap() = Some(shutdown_tx);

        // Initialize default widgets
        self.initialize_default_widgets().await?;

        // Start dashboard workers
        self.start_data_collection().await?;
        self.start_alert_monitoring().await?;
        self.start_chart_updates().await?;
        self.start_prediction_updates().await?;
        self.start_performance_monitoring().await?;

        // Update dashboard state
        {
            let mut state = self.dashboard_state.write().await;
            state.is_running = true;
            state.last_updated = SystemTime::now();
        }

        Ok(())
    }

    /// Get complete dashboard data
    pub async fn get_dashboard_data(&self) -> SemanticResult<DashboardData> {
        let widgets = self.widgets.read().await.clone();
        let charts = self.charts.read().await.clone();
        let alerts = self.alerts.read().await.clone();
        let state = self.dashboard_state.read().await.clone();
        let performance = self.performance_metrics.lock().unwrap().clone();

        // Get analytics data
        let analytics_dashboard = self.analytics_engine.get_analytics_dashboard().await?;

        Ok(DashboardData {
            state,
            widgets: widgets.into_values().collect(),
            charts: charts.into_values().collect(),
            alerts: alerts.into_values().collect(),
            analytics: analytics_dashboard,
            performance_metrics: performance,
            predictions: self.prediction_engine.get_predictions().await?,
        })
    }

    /// Add custom widget to dashboard
    pub async fn add_widget(&self, widget: DashboardWidget) -> SemanticResult<()> {
        let mut widgets = self.widgets.write().await;
        widgets.insert(widget.widget_id.clone(), widget);
        Ok(())
    }

    /// Update widget data
    pub async fn update_widget_data(&self, widget_id: &str, data: WidgetData) -> SemanticResult<()> {
        let mut widgets = self.widgets.write().await;
        if let Some(widget) = widgets.get_mut(widget_id) {
            widget.data = data;
            Ok(())
        } else {
            Err(SemanticError::NotFound(format!("Widget not found: {}", widget_id)))
        }
    }

    /// Get real-time metrics for dashboard
    pub async fn get_real_time_metrics(&self) -> SemanticResult<RealTimeMetrics> {
        let analytics_metrics = self.analytics_engine.get_performance_metrics().await;
        let dashboard_metrics = self.performance_metrics.lock().unwrap().clone();
        let state = self.dashboard_state.read().await.clone();

        Ok(RealTimeMetrics {
            timestamp: SystemTime::now(),
            analytics_metrics,
            dashboard_metrics,
            system_health: state.system_health,
            active_alerts: state.active_alerts_count,
            connected_clients: state.connected_clients,
        })
    }

    async fn initialize_default_widgets(&self) -> SemanticResult<()> {
        let default_widgets = vec![
            self.create_latency_widget(),
            self.create_throughput_widget(),
            self.create_error_rate_widget(),
            self.create_system_health_widget(),
            self.create_alerts_widget(),
            self.create_patterns_widget(),
            self.create_anomalies_widget(),
            self.create_predictions_widget(),
        ];

        let mut widgets = self.widgets.write().await;
        for widget in default_widgets {
            widgets.insert(widget.widget_id.clone(), widget);
        }

        Ok(())
    }

    fn create_latency_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "latency_metric".to_string(),
            widget_type: WidgetType::Metric,
            title: "Processing Latency".to_string(),
            position: WidgetPosition { x: 0, y: 0 },
            size: WidgetSize { width: 300, height: 150 },
            data: WidgetData::Metric(MetricData {
                current_value: 0.0,
                previous_value: 0.0,
                change_percent: 0.0,
                trend: TrendDirection::Stable,
                unit: "ms".to_string(),
                format: "{:.2}".to_string(),
            }),
            refresh_interval_ms: 1000,
        }
    }

    fn create_throughput_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "throughput_metric".to_string(),
            widget_type: WidgetType::Metric,
            title: "Event Throughput".to_string(),
            position: WidgetPosition { x: 300, y: 0 },
            size: WidgetSize { width: 300, height: 150 },
            data: WidgetData::Metric(MetricData {
                current_value: 0.0,
                previous_value: 0.0,
                change_percent: 0.0,
                trend: TrendDirection::Stable,
                unit: "events/sec".to_string(),
                format: "{:.0}".to_string(),
            }),
            refresh_interval_ms: 1000,
        }
    }

    fn create_error_rate_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "error_rate_metric".to_string(),
            widget_type: WidgetType::Gauge,
            title: "Error Rate".to_string(),
            position: WidgetPosition { x: 600, y: 0 },
            size: WidgetSize { width: 300, height: 150 },
            data: WidgetData::Gauge(GaugeData {
                current_value: 0.0,
                min_value: 0.0,
                max_value: 100.0,
                warning_threshold: self.config.alert_thresholds.error_rate_warning_percent,
                critical_threshold: self.config.alert_thresholds.error_rate_critical_percent,
                unit: "%".to_string(),
            }),
            refresh_interval_ms: 1000,
        }
    }

    fn create_system_health_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "system_health".to_string(),
            widget_type: WidgetType::Status,
            title: "System Health".to_string(),
            position: WidgetPosition { x: 900, y: 0 },
            size: WidgetSize { width: 300, height: 150 },
            data: WidgetData::Status(StatusData {
                status: ComponentStatus::Healthy,
                message: "All systems operational".to_string(),
                last_updated: SystemTime::now(),
                uptime_seconds: 0,
            }),
            refresh_interval_ms: 5000,
        }
    }

    fn create_alerts_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "active_alerts".to_string(),
            widget_type: WidgetType::Alert,
            title: "Active Alerts".to_string(),
            position: WidgetPosition { x: 0, y: 200 },
            size: WidgetSize { width: 600, height: 300 },
            data: WidgetData::Alert(Vec::new()),
            refresh_interval_ms: 2000,
        }
    }

    fn create_patterns_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "discovered_patterns".to_string(),
            widget_type: WidgetType::Table,
            title: "Discovered Patterns".to_string(),
            position: WidgetPosition { x: 600, y: 200 },
            size: WidgetSize { width: 600, height: 300 },
            data: WidgetData::Table(TableData {
                headers: vec![
                    "Pattern ID".to_string(),
                    "Type".to_string(),
                    "Confidence".to_string(),
                    "Frequency".to_string(),
                    "Last Seen".to_string(),
                ],
                rows: Vec::new(),
                sortable: true,
                filterable: true,
            }),
            refresh_interval_ms: 5000,
        }
    }

    fn create_anomalies_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "detected_anomalies".to_string(),
            widget_type: WidgetType::Table,
            title: "Detected Anomalies".to_string(),
            position: WidgetPosition { x: 0, y: 500 },
            size: WidgetSize { width: 600, height: 300 },
            data: WidgetData::Table(TableData {
                headers: vec![
                    "Anomaly ID".to_string(),
                    "Type".to_string(),
                    "Severity".to_string(),
                    "Confidence".to_string(),
                    "Detected At".to_string(),
                ],
                rows: Vec::new(),
                sortable: true,
                filterable: true,
            }),
            refresh_interval_ms: 3000,
        }
    }

    fn create_predictions_widget(&self) -> DashboardWidget {
        DashboardWidget {
            widget_id: "predictions_chart".to_string(),
            widget_type: WidgetType::Chart,
            title: "Predictive Analytics".to_string(),
            position: WidgetPosition { x: 600, y: 500 },
            size: WidgetSize { width: 600, height: 300 },
            data: WidgetData::Chart(ChartData {
                chart_id: "predictions".to_string(),
                title: "Predicted Metrics".to_string(),
                chart_type: ChartType::Line,
                data_points: VecDeque::new(),
                prediction_points: VecDeque::new(),
                y_axis_label: "Value".to_string(),
                unit: "".to_string(),
                color: "#007bff".to_string(),
            }),
            refresh_interval_ms: 10000,
        }
    }

    async fn start_data_collection(&self) -> SemanticResult<()> {
        // Implementation for data collection worker
        Ok(())
    }

    async fn start_alert_monitoring(&self) -> SemanticResult<()> {
        // Implementation for alert monitoring worker
        Ok(())
    }

    async fn start_chart_updates(&self) -> SemanticResult<()> {
        // Implementation for chart updates worker
        Ok(())
    }

    async fn start_prediction_updates(&self) -> SemanticResult<()> {
        // Implementation for prediction updates worker
        Ok(())
    }

    async fn start_performance_monitoring(&self) -> SemanticResult<()> {
        // Implementation for performance monitoring worker
        Ok(())
    }
}

impl AlertManager {
    pub fn new(config: DashboardConfig) -> SemanticResult<Self> {
        Ok(Self {
            config,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            notification_channels: Vec::new(),
        })
    }

    pub async fn check_thresholds(&self, metrics: &AnalyticsPerformanceMetrics) -> SemanticResult<Vec<DashboardAlert>> {
        let mut alerts = Vec::new();

        // Check latency threshold
        if metrics.processing_latency_ns > self.config.alert_thresholds.latency_critical_ms * 1_000_000 {
            alerts.push(self.create_latency_alert(metrics.processing_latency_ns, AlertSeverity::Critical));
        } else if metrics.processing_latency_ns > self.config.alert_thresholds.latency_warning_ms * 1_000_000 {
            alerts.push(self.create_latency_alert(metrics.processing_latency_ns, AlertSeverity::Warning));
        }

        // Check throughput threshold
        if metrics.throughput_events_per_second < self.config.alert_thresholds.throughput_critical_eps {
            alerts.push(self.create_throughput_alert(metrics.throughput_events_per_second, AlertSeverity::Critical));
        } else if metrics.throughput_events_per_second < self.config.alert_thresholds.throughput_warning_eps {
            alerts.push(self.create_throughput_alert(metrics.throughput_events_per_second, AlertSeverity::Warning));
        }

        Ok(alerts)
    }

    fn create_latency_alert(&self, latency_ns: u64, severity: AlertSeverity) -> DashboardAlert {
        DashboardAlert {
            alert_id: Uuid::new_v4(),
            alert_type: AlertType::Latency,
            severity,
            title: "High Processing Latency".to_string(),
            description: format!("Processing latency is {}ms", latency_ns / 1_000_000),
            triggered_at: SystemTime::now(),
            resolved_at: None,
            metric_value: (latency_ns / 1_000_000) as f64,
            threshold_value: self.config.alert_thresholds.latency_warning_ms as f64,
            component: "Analytics Engine".to_string(),
        }
    }

    fn create_throughput_alert(&self, throughput: f64, severity: AlertSeverity) -> DashboardAlert {
        DashboardAlert {
            alert_id: Uuid::new_v4(),
            alert_type: AlertType::Throughput,
            severity,
            title: "Low Event Throughput".to_string(),
            description: format!("Event throughput is {:.1} events/sec", throughput),
            triggered_at: SystemTime::now(),
            resolved_at: None,
            metric_value: throughput,
            threshold_value: self.config.alert_thresholds.throughput_warning_eps,
            component: "Analytics Engine".to_string(),
        }
    }
}

impl ChartManager {
    pub fn new(config: DashboardConfig) -> SemanticResult<Self> {
        Ok(Self {
            config,
            chart_data: Arc::new(RwLock::new(HashMap::new())),
            chart_templates: Vec::new(),
        })
    }

    pub async fn update_chart_data(&self, chart_id: &str, point: ChartPoint) -> SemanticResult<()> {
        let mut charts = self.chart_data.write().await;
        if let Some(chart) = charts.get_mut(chart_id) {
            chart.data_points.push_back(point);
            
            // Maintain max points
            if chart.data_points.len() > self.config.max_chart_points {
                chart.data_points.pop_front();
            }
        }
        Ok(())
    }
}

impl PredictionEngine {
    pub fn new(config: DashboardConfig) -> SemanticResult<Self> {
        Ok(Self {
            config,
            prediction_models: HashMap::new(),
            forecast_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn get_predictions(&self) -> SemanticResult<Vec<Prediction>> {
        // Simplified prediction implementation
        Ok(Vec::new())
    }
}

/// Complete dashboard data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub state: DashboardState,
    pub widgets: Vec<DashboardWidget>,
    pub charts: Vec<ChartData>,
    pub alerts: Vec<DashboardAlert>,
    pub analytics: crate::semantic_api::event_analytics_engine::AnalyticsDashboard,
    pub performance_metrics: DashboardPerformanceMetrics,
    pub predictions: Vec<Prediction>,
}

/// Real-time metrics for dashboard updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeMetrics {
    pub timestamp: SystemTime,
    pub analytics_metrics: AnalyticsPerformanceMetrics,
    pub dashboard_metrics: DashboardPerformanceMetrics,
    pub system_health: SystemHealth,
    pub active_alerts: u32,
    pub connected_clients: u32,
}

impl Default for DashboardPerformanceMetrics {
    fn default() -> Self {
        Self {
            update_latency_ms: 0,
            render_time_ms: 0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            network_throughput_mbps: 0.0,
            client_connections: 0,
            updates_per_second: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_api::event_analytics_engine::AnalyticsConfig;

    #[tokio::test]
    async fn test_dashboard_creation() {
        let config = DashboardConfig::default();
        let analytics_config = AnalyticsConfig::default();
        let analytics_engine = Arc::new(EventAnalyticsEngine::new(analytics_config).unwrap());
        
        let dashboard = MonitoringDashboard::new(config, analytics_engine).unwrap();
        assert!(!dashboard.dashboard_state.read().await.is_running);
    }

    #[tokio::test]
    async fn test_widget_management() {
        let config = DashboardConfig::default();
        let analytics_config = AnalyticsConfig::default();
        let analytics_engine = Arc::new(EventAnalyticsEngine::new(analytics_config).unwrap());
        
        let dashboard = MonitoringDashboard::new(config, analytics_engine).unwrap();
        
        // Test adding a widget
        let test_widget = DashboardWidget {
            widget_id: "test_widget".to_string(),
            widget_type: WidgetType::Metric,
            title: "Test Widget".to_string(),
            position: WidgetPosition { x: 0, y: 0 },
            size: WidgetSize { width: 200, height: 100 },
            data: WidgetData::Metric(MetricData {
                current_value: 42.0,
                previous_value: 40.0,
                change_percent: 5.0,
                trend: TrendDirection::Up,
                unit: "test".to_string(),
                format: "{:.1}".to_string(),
            }),
            refresh_interval_ms: 1000,
        };
        
        dashboard.add_widget(test_widget).await.unwrap();
        
        let widgets = dashboard.widgets.read().await;
        assert!(widgets.contains_key("test_widget"));
    }
}