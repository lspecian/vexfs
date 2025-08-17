// Health check HTTP endpoint for VexFS
use std::sync::Arc;
use serde_json::json;
use crate::monitoring::{MonitoringSystem, HealthStatus, HealthCheckRunner};
use std::time::Duration;

/// Health endpoint handler for HTTP servers
pub struct HealthEndpoint {
    monitoring: Arc<MonitoringSystem>,
    health_runner: Arc<HealthCheckRunner>,
}

impl HealthEndpoint {
    pub fn new(monitoring: Arc<MonitoringSystem>) -> Self {
        let health_runner = Arc::new(HealthCheckRunner::new(
            monitoring.clone(),
            Duration::from_secs(30),
        ));
        
        Self {
            monitoring,
            health_runner,
        }
    }
    
    /// Handle /health endpoint
    pub fn handle_health(&self) -> (u16, String) {
        // Run health checks
        self.health_runner.run_all_checks();
        
        // Get system health
        let system_health = self.monitoring.get_system_health();
        let health_report = self.monitoring.generate_health_report();
        
        // Determine HTTP status code
        let status_code = match system_health {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded(_) => 200, // Still return 200 for degraded
            HealthStatus::Unhealthy(_) => 503,
            HealthStatus::Unknown => 503,
        };
        
        // Generate response
        let response = json!({
            "status": match system_health {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Degraded(_) => "degraded",
                HealthStatus::Unhealthy(_) => "unhealthy",
                HealthStatus::Unknown => "unknown",
            },
            "timestamp": health_report.generated_at.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            "checks": health_report.health_checks.iter().map(|check| {
                json!({
                    "component": check.component,
                    "status": match &check.status {
                        HealthStatus::Healthy => "healthy",
                        HealthStatus::Degraded(_) => "degraded",
                        HealthStatus::Unhealthy(_) => "unhealthy",
                        HealthStatus::Unknown => "unknown",
                    },
                    "message": check.message,
                    "response_time_ms": check.response_time_ms,
                    "metadata": check.metadata,
                })
            }).collect::<Vec<_>>(),
            "metrics": {
                "total_operations": health_report.metrics.total_operations,
                "success_rate": if health_report.metrics.total_operations > 0 {
                    (health_report.metrics.successful_operations as f64 / 
                     health_report.metrics.total_operations as f64) * 100.0
                } else {
                    0.0
                },
                "avg_latency_ms": health_report.metrics.avg_latency_ms,
                "p99_latency_ms": health_report.metrics.p99_latency_ms,
                "memory_mb": health_report.metrics.memory_usage_bytes / 1_048_576,
                "uptime_seconds": health_report.metrics.last_update
                    .duration_since(health_report.metrics.start_time)
                    .unwrap_or_default().as_secs(),
            },
            "alerts": health_report.active_alerts.iter().map(|alert| {
                json!({
                    "level": format!("{:?}", alert.level).to_lowercase(),
                    "component": alert.component,
                    "message": alert.message,
                    "timestamp": alert.timestamp.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().as_secs(),
                })
            }).collect::<Vec<_>>(),
        });
        
        (status_code, response.to_string())
    }
    
    /// Handle /health/live endpoint (liveness probe)
    pub fn handle_liveness(&self) -> (u16, String) {
        // Simple liveness check - just return OK if the process is running
        (200, json!({
            "status": "alive",
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
        }).to_string())
    }
    
    /// Handle /health/ready endpoint (readiness probe)
    pub fn handle_readiness(&self) -> (u16, String) {
        // Check if system is ready to serve traffic
        let system_health = self.monitoring.get_system_health();
        
        let (status_code, ready) = match system_health {
            HealthStatus::Healthy => (200, true),
            HealthStatus::Degraded(_) => (200, true), // Still ready even if degraded
            HealthStatus::Unhealthy(_) => (503, false),
            HealthStatus::Unknown => (503, false),
        };
        
        (status_code, json!({
            "ready": ready,
            "status": match system_health {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Degraded(_) => "degraded",
                HealthStatus::Unhealthy(_) => "unhealthy",
                HealthStatus::Unknown => "unknown",
            },
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
        }).to_string())
    }
    
    /// Handle /metrics endpoint (Prometheus format)
    pub fn handle_metrics(&self) -> (u16, String) {
        let metrics = self.monitoring.get_metrics().unwrap_or_default();
        
        let mut output = String::new();
        
        // Add standard Prometheus metrics
        output.push_str("# HELP vexfs_operations_total Total number of operations\n");
        output.push_str("# TYPE vexfs_operations_total counter\n");
        output.push_str(&format!("vexfs_operations_total {}\n", metrics.total_operations));
        
        output.push_str("# HELP vexfs_operations_success_total Total number of successful operations\n");
        output.push_str("# TYPE vexfs_operations_success_total counter\n");
        output.push_str(&format!("vexfs_operations_success_total {}\n", metrics.successful_operations));
        
        output.push_str("# HELP vexfs_operations_failed_total Total number of failed operations\n");
        output.push_str("# TYPE vexfs_operations_failed_total counter\n");
        output.push_str(&format!("vexfs_operations_failed_total {}\n", metrics.failed_operations));
        
        output.push_str("# HELP vexfs_latency_milliseconds Operation latency in milliseconds\n");
        output.push_str("# TYPE vexfs_latency_milliseconds summary\n");
        output.push_str(&format!("vexfs_latency_milliseconds{{quantile=\"0.5\"}} {}\n", metrics.p50_latency_ms));
        output.push_str(&format!("vexfs_latency_milliseconds{{quantile=\"0.95\"}} {}\n", metrics.p95_latency_ms));
        output.push_str(&format!("vexfs_latency_milliseconds{{quantile=\"0.99\"}} {}\n", metrics.p99_latency_ms));
        output.push_str(&format!("vexfs_latency_milliseconds_sum {}\n", 
            metrics.avg_latency_ms * metrics.total_operations as f64));
        output.push_str(&format!("vexfs_latency_milliseconds_count {}\n", metrics.total_operations));
        
        output.push_str("# HELP vexfs_memory_bytes Memory usage in bytes\n");
        output.push_str("# TYPE vexfs_memory_bytes gauge\n");
        output.push_str(&format!("vexfs_memory_bytes {}\n", metrics.memory_usage_bytes));
        
        output.push_str("# HELP vexfs_open_file_handles Number of open file handles\n");
        output.push_str("# TYPE vexfs_open_file_handles gauge\n");
        output.push_str(&format!("vexfs_open_file_handles {}\n", metrics.open_file_handles));
        
        output.push_str("# HELP vexfs_active_connections Number of active connections\n");
        output.push_str("# TYPE vexfs_active_connections gauge\n");
        output.push_str(&format!("vexfs_active_connections {}\n", metrics.active_connections));
        
        output.push_str("# HELP vexfs_cache_hits_total Total number of cache hits\n");
        output.push_str("# TYPE vexfs_cache_hits_total counter\n");
        output.push_str(&format!("vexfs_cache_hits_total {}\n", metrics.cache_hits));
        
        output.push_str("# HELP vexfs_cache_misses_total Total number of cache misses\n");
        output.push_str("# TYPE vexfs_cache_misses_total counter\n");
        output.push_str(&format!("vexfs_cache_misses_total {}\n", metrics.cache_misses));
        
        // Add health status as metrics
        let health_checks = self.monitoring.get_health_checks();
        output.push_str("# HELP vexfs_health_status Health status of components (1=healthy, 0=unhealthy)\n");
        output.push_str("# TYPE vexfs_health_status gauge\n");
        
        for check in health_checks {
            let value = match check.status {
                HealthStatus::Healthy => 1,
                _ => 0,
            };
            output.push_str(&format!("vexfs_health_status{{component=\"{}\"}} {}\n", 
                check.component, value));
        }
        
        (200, output)
    }
}

/// Standalone health check server
pub mod server {
    use super::*;
    use std::io::prelude::*;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    
    pub struct HealthServer {
        endpoint: Arc<HealthEndpoint>,
        port: u16,
    }
    
    impl HealthServer {
        pub fn new(monitoring: Arc<MonitoringSystem>, port: u16) -> Self {
            Self {
                endpoint: Arc::new(HealthEndpoint::new(monitoring)),
                port,
            }
        }
        
        pub fn start(&self) -> std::io::Result<()> {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
            println!("Health check server listening on port {}", self.port);
            
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let endpoint = self.endpoint.clone();
                    thread::spawn(move || {
                        handle_connection(stream, endpoint);
                    });
                }
            }
            
            Ok(())
        }
    }
    
    fn handle_connection(mut stream: TcpStream, endpoint: Arc<HealthEndpoint>) {
        let mut buffer = [0; 1024];
        
        if let Ok(_) = stream.read(&mut buffer) {
            let request = String::from_utf8_lossy(&buffer[..]);
            
            let (status_code, body) = if request.contains("GET /health/live") {
                endpoint.handle_liveness()
            } else if request.contains("GET /health/ready") {
                endpoint.handle_readiness()
            } else if request.contains("GET /health") {
                endpoint.handle_health()
            } else if request.contains("GET /metrics") {
                endpoint.handle_metrics()
            } else {
                (404, json!({"error": "Not found"}).to_string())
            };
            
            let status_text = match status_code {
                200 => "OK",
                503 => "Service Unavailable",
                _ => "Not Found",
            };
            
            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                status_code, status_text, body.len(), body
            );
            
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    }
}