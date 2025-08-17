// VexFS Monitoring and Health Check System
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration, Instant};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Unknown,
}

/// Component health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub component: String,
    pub status: HealthStatus,
    pub message: String,
    pub last_check: SystemTime,
    pub response_time_ms: u64,
    pub metadata: HashMap<String, String>,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    // Operation counts
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    
    // Performance metrics
    pub avg_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub max_latency_ms: u64,
    
    // Resource usage
    pub memory_usage_bytes: u64,
    pub open_file_handles: u64,
    pub active_connections: u64,
    
    // FUSE specific
    pub fuse_operations: u64,
    pub fuse_errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    
    // Timestamps
    pub start_time: SystemTime,
    pub last_update: SystemTime,
}

/// Operation metrics for tracking individual operations
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub operation_type: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Monitoring system for VexFS
pub struct MonitoringSystem {
    metrics: Arc<RwLock<SystemMetrics>>,
    health_checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    operation_latencies: Arc<RwLock<Vec<u64>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
}

/// Alert for monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub component: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            avg_latency_ms: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            max_latency_ms: 0,
            memory_usage_bytes: 0,
            open_file_handles: 0,
            active_connections: 0,
            fuse_operations: 0,
            fuse_errors: 0,
            cache_hits: 0,
            cache_misses: 0,
            start_time: SystemTime::now(),
            last_update: SystemTime::now(),
        }
    }
}

impl MonitoringSystem {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            operation_latencies: Arc::new(RwLock::new(Vec::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Record an operation
    pub fn record_operation(&self, operation: OperationMetrics) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.total_operations += 1;
            
            if operation.success {
                metrics.successful_operations += 1;
            } else {
                metrics.failed_operations += 1;
            }
            
            // Calculate latency
            if let Some(end_time) = operation.end_time {
                let latency_ms = end_time.duration_since(operation.start_time).as_millis() as u64;
                
                // Update latency metrics
                if let Ok(mut latencies) = self.operation_latencies.write() {
                    latencies.push(latency_ms);
                    
                    // Keep only last 1000 latencies to avoid memory growth
                    if latencies.len() > 1000 {
                        latencies.remove(0);
                    }
                    
                    // Update percentiles
                    let mut sorted = latencies.clone();
                    sorted.sort_unstable();
                    
                    if !sorted.is_empty() {
                        metrics.p50_latency_ms = sorted[sorted.len() / 2];
                        metrics.p95_latency_ms = sorted[sorted.len() * 95 / 100];
                        metrics.p99_latency_ms = sorted[sorted.len() * 99 / 100];
                        metrics.max_latency_ms = *sorted.last().unwrap_or(&0);
                        
                        let sum: u64 = sorted.iter().sum();
                        metrics.avg_latency_ms = sum as f64 / sorted.len() as f64;
                    }
                }
            }
            
            metrics.last_update = SystemTime::now();
        }
    }
    
    /// Update a health check
    pub fn update_health_check(&self, check: HealthCheck) {
        // Check for status changes and create alerts
        if let Ok(health_checks) = self.health_checks.read() {
            if let Some(old_check) = health_checks.get(&check.component) {
                if old_check.status != check.status {
                    let alert_level = match &check.status {
                        HealthStatus::Healthy => AlertLevel::Info,
                        HealthStatus::Degraded(_) => AlertLevel::Warning,
                        HealthStatus::Unhealthy(_) => AlertLevel::Error,
                        HealthStatus::Unknown => AlertLevel::Warning,
                    };
                    
                    self.create_alert(
                        alert_level,
                        check.component.clone(),
                        format!("Health status changed: {:?} -> {:?}", old_check.status, check.status),
                    );
                }
            }
        }
        
        if let Ok(mut health_checks) = self.health_checks.write() {
            health_checks.insert(check.component.clone(), check);
        }
    }
    
    /// Create an alert
    pub fn create_alert(&self, level: AlertLevel, component: String, message: String) {
        if let Ok(mut alerts) = self.alerts.write() {
            alerts.push(Alert {
                level,
                component,
                message,
                timestamp: SystemTime::now(),
                resolved: false,
            });
            
            // Keep only last 100 alerts
            if alerts.len() > 100 {
                alerts.remove(0);
            }
        }
    }
    
    /// Get current metrics
    pub fn get_metrics(&self) -> Option<SystemMetrics> {
        self.metrics.read().ok().map(|m| m.clone())
    }
    
    /// Get all health checks
    pub fn get_health_checks(&self) -> Vec<HealthCheck> {
        self.health_checks.read()
            .ok()
            .map(|checks| checks.values().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get overall system health
    pub fn get_system_health(&self) -> HealthStatus {
        let checks = self.get_health_checks();
        
        if checks.is_empty() {
            return HealthStatus::Unknown;
        }
        
        let mut unhealthy_count = 0;
        let mut degraded_count = 0;
        let mut messages = Vec::new();
        
        for check in checks {
            match check.status {
                HealthStatus::Unhealthy(msg) => {
                    unhealthy_count += 1;
                    messages.push(format!("{}: {}", check.component, msg));
                }
                HealthStatus::Degraded(msg) => {
                    degraded_count += 1;
                    messages.push(format!("{}: {}", check.component, msg));
                }
                _ => {}
            }
        }
        
        if unhealthy_count > 0 {
            HealthStatus::Unhealthy(messages.join("; "))
        } else if degraded_count > 0 {
            HealthStatus::Degraded(messages.join("; "))
        } else {
            HealthStatus::Healthy
        }
    }
    
    /// Get active alerts
    pub fn get_alerts(&self, include_resolved: bool) -> Vec<Alert> {
        self.alerts.read()
            .ok()
            .map(|alerts| {
                alerts.iter()
                    .filter(|a| include_resolved || !a.resolved)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Update resource metrics
    pub fn update_resource_metrics(&self, memory_bytes: u64, file_handles: u64, connections: u64) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.memory_usage_bytes = memory_bytes;
            metrics.open_file_handles = file_handles;
            metrics.active_connections = connections;
            metrics.last_update = SystemTime::now();
        }
    }
    
    /// Generate a health report
    pub fn generate_health_report(&self) -> HealthReport {
        HealthReport {
            system_health: self.get_system_health(),
            metrics: self.get_metrics().unwrap_or_default(),
            health_checks: self.get_health_checks(),
            active_alerts: self.get_alerts(false),
            generated_at: SystemTime::now(),
        }
    }
}

/// Complete health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub system_health: HealthStatus,
    pub metrics: SystemMetrics,
    pub health_checks: Vec<HealthCheck>,
    pub active_alerts: Vec<Alert>,
    pub generated_at: SystemTime,
}

/// Health check runner
pub struct HealthCheckRunner {
    monitoring: Arc<MonitoringSystem>,
    check_interval: Duration,
}

impl HealthCheckRunner {
    pub fn new(monitoring: Arc<MonitoringSystem>, check_interval: Duration) -> Self {
        Self {
            monitoring,
            check_interval,
        }
    }
    
    /// Run FUSE health check
    pub fn check_fuse_health(&self) -> HealthCheck {
        let start = Instant::now();
        let mut status = HealthStatus::Healthy;
        let mut message = "FUSE filesystem is operational".to_string();
        let mut metadata = HashMap::new();
        
        // Check if FUSE is mounted
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            if mounts.contains("vexfs") {
                metadata.insert("mounted".to_string(), "true".to_string());
            } else {
                status = HealthStatus::Degraded("FUSE not mounted".to_string());
                message = "FUSE filesystem is not mounted".to_string();
                metadata.insert("mounted".to_string(), "false".to_string());
            }
        }
        
        // Check error rate
        if let Some(metrics) = self.monitoring.get_metrics() {
            let error_rate = if metrics.total_operations > 0 {
                (metrics.failed_operations as f64 / metrics.total_operations as f64) * 100.0
            } else {
                0.0
            };
            
            metadata.insert("error_rate".to_string(), format!("{:.2}%", error_rate));
            
            if error_rate > 10.0 {
                status = HealthStatus::Unhealthy(format!("High error rate: {:.2}%", error_rate));
                message = format!("FUSE error rate is too high: {:.2}%", error_rate);
            } else if error_rate > 5.0 {
                status = HealthStatus::Degraded(format!("Elevated error rate: {:.2}%", error_rate));
                message = format!("FUSE error rate is elevated: {:.2}%", error_rate);
            }
        }
        
        HealthCheck {
            component: "fuse".to_string(),
            status,
            message,
            last_check: SystemTime::now(),
            response_time_ms: start.elapsed().as_millis() as u64,
            metadata,
        }
    }
    
    /// Check memory health
    pub fn check_memory_health(&self) -> HealthCheck {
        let start = Instant::now();
        let mut status = HealthStatus::Healthy;
        let mut message = "Memory usage is normal".to_string();
        let mut metadata = HashMap::new();
        
        if let Some(metrics) = self.monitoring.get_metrics() {
            let memory_mb = metrics.memory_usage_bytes / 1_048_576;
            metadata.insert("memory_mb".to_string(), memory_mb.to_string());
            
            if memory_mb > 1024 {
                status = HealthStatus::Unhealthy(format!("High memory usage: {}MB", memory_mb));
                message = format!("Memory usage is very high: {}MB", memory_mb);
            } else if memory_mb > 512 {
                status = HealthStatus::Degraded(format!("Elevated memory usage: {}MB", memory_mb));
                message = format!("Memory usage is elevated: {}MB", memory_mb);
            }
        }
        
        HealthCheck {
            component: "memory".to_string(),
            status,
            message,
            last_check: SystemTime::now(),
            response_time_ms: start.elapsed().as_millis() as u64,
            metadata,
        }
    }
    
    /// Check performance health
    pub fn check_performance_health(&self) -> HealthCheck {
        let start = Instant::now();
        let mut status = HealthStatus::Healthy;
        let mut message = "Performance is within normal parameters".to_string();
        let mut metadata = HashMap::new();
        
        if let Some(metrics) = self.monitoring.get_metrics() {
            metadata.insert("p99_latency_ms".to_string(), metrics.p99_latency_ms.to_string());
            metadata.insert("avg_latency_ms".to_string(), format!("{:.2}", metrics.avg_latency_ms));
            
            if metrics.p99_latency_ms > 1000 {
                status = HealthStatus::Unhealthy(format!("Very high latency: {}ms", metrics.p99_latency_ms));
                message = format!("P99 latency is very high: {}ms", metrics.p99_latency_ms);
            } else if metrics.p99_latency_ms > 500 {
                status = HealthStatus::Degraded(format!("High latency: {}ms", metrics.p99_latency_ms));
                message = format!("P99 latency is elevated: {}ms", metrics.p99_latency_ms);
            }
        }
        
        HealthCheck {
            component: "performance".to_string(),
            status,
            message,
            last_check: SystemTime::now(),
            response_time_ms: start.elapsed().as_millis() as u64,
            metadata,
        }
    }
    
    /// Run all health checks
    pub fn run_all_checks(&self) {
        self.monitoring.update_health_check(self.check_fuse_health());
        self.monitoring.update_health_check(self.check_memory_health());
        self.monitoring.update_health_check(self.check_performance_health());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_monitoring_system() {
        let monitoring = MonitoringSystem::new();
        
        // Record some operations
        for i in 0..10 {
            let mut op = OperationMetrics {
                operation_type: "test".to_string(),
                start_time: Instant::now(),
                end_time: None,
                success: i % 2 == 0,
                error_message: if i % 2 == 1 { Some("test error".to_string()) } else { None },
            };
            
            std::thread::sleep(Duration::from_millis(10));
            op.end_time = Some(Instant::now());
            
            monitoring.record_operation(op);
        }
        
        // Check metrics
        let metrics = monitoring.get_metrics().unwrap();
        assert_eq!(metrics.total_operations, 10);
        assert_eq!(metrics.successful_operations, 5);
        assert_eq!(metrics.failed_operations, 5);
        assert!(metrics.avg_latency_ms > 0.0);
    }
    
    #[test]
    fn test_health_checks() {
        let monitoring = Arc::new(MonitoringSystem::new());
        let runner = HealthCheckRunner::new(monitoring.clone(), Duration::from_secs(60));
        
        runner.run_all_checks();
        
        let health_checks = monitoring.get_health_checks();
        assert!(health_checks.len() >= 3);
        
        let system_health = monitoring.get_system_health();
        assert!(!matches!(system_health, HealthStatus::Unknown));
    }
}