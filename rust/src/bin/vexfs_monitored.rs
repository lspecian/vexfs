// VexFS FUSE with Monitoring and Health Checks
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use fuse;

#[path = "../fuse_with_monitoring.rs"]
mod fuse_with_monitoring;

#[path = "../fuse_error_handling.rs"]
mod fuse_error_handling;

#[path = "../monitoring.rs"]
mod monitoring;

#[path = "../health_endpoint.rs"]
mod health_endpoint;

use monitoring::{MonitoringSystem, HealthCheckRunner};
use health_endpoint::server::HealthServer;

fn main() {
    println!("VexFS Monitored FUSE Implementation v0.0.4-alpha");
    println!("================================================");
    
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mountpoint> [options]", args[0]);
        eprintln!("\nOptions:");
        eprintln!("  -f               Run in foreground");
        eprintln!("  -d               Enable debug output");
        eprintln!("  --health-port    Port for health checks (default: 8080)");
        eprintln!("\nExample:");
        eprintln!("  {} /mnt/vexfs -f --health-port 8080", args[0]);
        std::process::exit(1);
    }
    
    let mountpoint = &args[1];
    let path = Path::new(mountpoint);
    
    if !path.exists() {
        eprintln!("Error: Mount point {} does not exist", mountpoint);
        eprintln!("Please create it first: sudo mkdir -p {}", mountpoint);
        std::process::exit(1);
    }
    
    // Parse health check port
    let health_port = args.iter()
        .position(|arg| arg == "--health-port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    
    println!("Configuration:");
    println!("  Mount point: {}", mountpoint);
    println!("  Health check port: {}", health_port);
    
    // Create monitoring system
    let monitoring = Arc::new(MonitoringSystem::new());
    
    // Start health check server in a separate thread
    let monitoring_clone = monitoring.clone();
    thread::spawn(move || {
        let health_server = HealthServer::new(monitoring_clone, health_port);
        if let Err(e) = health_server.start() {
            eprintln!("Failed to start health server: {}", e);
        }
    });
    
    println!("\nHealth check endpoints available:");
    println!("  http://localhost:{}/health       - Full health check", health_port);
    println!("  http://localhost:{}/health/live  - Liveness probe", health_port);
    println!("  http://localhost:{}/health/ready - Readiness probe", health_port);
    println!("  http://localhost:{}/metrics      - Prometheus metrics", health_port);
    
    // Start background health checks
    let monitoring_clone = monitoring.clone();
    thread::spawn(move || {
        let runner = HealthCheckRunner::new(monitoring_clone, Duration::from_secs(30));
        loop {
            runner.run_all_checks();
            thread::sleep(Duration::from_secs(30));
        }
    });
    
    // Create FUSE filesystem with monitoring
    let vexfs = fuse_with_monitoring::MonitoredVexFS::new(monitoring.clone());
    
    let mut options = vec!["-o", "fsname=vexfs", "-o", "auto_unmount"];
    
    // Add foreground option if specified
    if args.contains(&"-f".to_string()) {
        println!("\nRunning in foreground mode...");
    } else {
        println!("\nRunning in background mode...");
    }
    
    // Add debug option if specified
    if args.contains(&"-d".to_string()) {
        println!("Debug output enabled");
        options.push("-d");
    }
    
    println!("\nMounting VexFS...");
    
    // Mount the filesystem
    match fuse::mount(vexfs, path, &options) {
        Ok(_) => {
            println!("VexFS unmounted successfully");
            
            // Print final metrics
            if let Some(metrics) = monitoring.get_metrics() {
                println!("\nFinal Statistics:");
                println!("  Total operations: {}", metrics.total_operations);
                println!("  Success rate: {:.2}%", 
                    if metrics.total_operations > 0 {
                        (metrics.successful_operations as f64 / metrics.total_operations as f64) * 100.0
                    } else {
                        0.0
                    }
                );
                println!("  Average latency: {:.2}ms", metrics.avg_latency_ms);
                println!("  P99 latency: {}ms", metrics.p99_latency_ms);
            }
        }
        Err(e) => {
            eprintln!("Failed to mount VexFS: {}", e);
            eprintln!("\nTroubleshooting tips:");
            eprintln!("1. Make sure you have FUSE installed: sudo apt-get install fuse3");
            eprintln!("2. Check if another filesystem is mounted: mount | grep {}", mountpoint);
            eprintln!("3. Try unmounting first: sudo umount {}", mountpoint);
            eprintln!("4. Check permissions: you may need to run with sudo");
            std::process::exit(1);
        }
    }
}