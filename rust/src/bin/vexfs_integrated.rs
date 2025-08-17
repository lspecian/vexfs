// VexFS Integrated - FUSE with Monitoring and VexGraph
// This is the main production-ready binary that combines all features

use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use fuse;

// Import all modules
#[path = "../fuse_vexgraph_integrated.rs"]
mod fuse_vexgraph_integrated;

#[path = "../fuse_error_handling.rs"]
mod fuse_error_handling;

#[path = "../monitoring.rs"]
mod monitoring;

#[path = "../health_endpoint.rs"]
mod health_endpoint;

#[path = "../fuse_vexgraph_bridge.rs"]
mod fuse_vexgraph_bridge;

use monitoring::{MonitoringSystem, HealthCheckRunner};
use health_endpoint::server::HealthServer;
use fuse_vexgraph_bridge::{IntegratedFilesystemBuilder, StorageConfig};

fn main() {
    println!("VexFS Integrated v0.0.4-alpha");
    println!("================================");
    println!("Features: FUSE + Monitoring + VexGraph");
    println!();
    
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    
    let mountpoint = &args[1];
    let path = Path::new(mountpoint);
    
    if !path.exists() {
        eprintln!("Error: Mount point {} does not exist", mountpoint);
        eprintln!("Please create it first: sudo mkdir -p {}", mountpoint);
        std::process::exit(1);
    }
    
    // Parse command line options
    let config = parse_args(&args);
    
    println!("Configuration:");
    println!("  Mount point: {}", mountpoint);
    println!("  Storage backend: {:?}", config.storage_backend);
    println!("  Health check port: {}", config.health_port);
    println!("  VexGraph enabled: {}", config.enable_vexgraph);
    println!("  Monitoring enabled: {}", config.enable_monitoring);
    println!();
    
    // Create monitoring system
    let monitoring = Arc::new(MonitoringSystem::new());
    
    // Start health check server if monitoring is enabled
    if config.enable_monitoring {
        let monitoring_clone = monitoring.clone();
        let health_port = config.health_port;
        thread::spawn(move || {
            let health_server = HealthServer::new(monitoring_clone, health_port);
            if let Err(e) = health_server.start() {
                eprintln!("Failed to start health server: {}", e);
            }
        });
        
        println!("Health check endpoints available:");
        println!("  http://localhost:{}/health       - Full health check", config.health_port);
        println!("  http://localhost:{}/health/live  - Liveness probe", config.health_port);
        println!("  http://localhost:{}/health/ready - Readiness probe", config.health_port);
        println!("  http://localhost:{}/metrics      - Prometheus metrics", config.health_port);
        println!();
        
        // Start background health checks
        let monitoring_clone = monitoring.clone();
        thread::spawn(move || {
            let runner = HealthCheckRunner::new(monitoring_clone, Duration::from_secs(30));
            loop {
                runner.run_all_checks();
                thread::sleep(Duration::from_secs(30));
            }
        });
    }
    
    // Create the integrated filesystem
    let builder = IntegratedFilesystemBuilder::new()
        .with_storage(config.storage_backend.clone())
        .with_monitoring(config.enable_monitoring)
        .with_semantic_search(config.enable_semantic_search);
    
    let integrated_fs = match builder.build() {
        Ok(fs) => fs,
        Err(e) => {
            eprintln!("Failed to create integrated filesystem: {}", e);
            std::process::exit(1);
        }
    };
    
    // Create FUSE filesystem
    let vexfs = fuse_vexgraph_integrated::IntegratedVexFS::new(
        integrated_fs.get_bridge(),
        integrated_fs.get_monitoring(),
    );
    
    let mut fuse_options = vec!["-o", "fsname=vexfs", "-o", "auto_unmount"];
    
    if config.foreground {
        println!("Running in foreground mode...");
    } else {
        println!("Running in background mode...");
    }
    
    if config.debug {
        println!("Debug output enabled");
        fuse_options.push("-d");
    }
    
    println!("\nMounting VexFS...");
    
    // Mount the filesystem
    match fuse::mount(vexfs, path, &fuse_options) {
        Ok(_) => {
            println!("VexFS unmounted successfully");
            
            // Print final statistics
            if config.enable_monitoring {
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
            
            if config.enable_vexgraph {
                let bridge = integrated_fs.get_bridge();
                if let Ok(bridge) = bridge.lock() {
                    let stats = bridge.get_stats();
                    println!("\nGraph Statistics:");
                    println!("  Total nodes: {}", stats.total_nodes);
                    println!("  Total edges: {}", stats.total_edges);
                    println!("  File mappings: {}", stats.total_mappings);
                }
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

#[derive(Debug, Clone)]
struct Config {
    storage_backend: StorageConfig,
    health_port: u16,
    enable_vexgraph: bool,
    enable_monitoring: bool,
    enable_semantic_search: bool,
    foreground: bool,
    debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage_backend: StorageConfig::Memory,
            health_port: 8080,
            enable_vexgraph: true,
            enable_monitoring: true,
            enable_semantic_search: false,
            foreground: false,
            debug: false,
        }
    }
}

fn parse_args(args: &[String]) -> Config {
    let mut config = Config::default();
    
    for i in 0..args.len() {
        match args[i].as_str() {
            "-f" | "--foreground" => config.foreground = true,
            "-d" | "--debug" => config.debug = true,
            "--no-monitoring" => config.enable_monitoring = false,
            "--no-vexgraph" => config.enable_vexgraph = false,
            "--semantic-search" => config.enable_semantic_search = true,
            "--health-port" => {
                if i + 1 < args.len() {
                    if let Ok(port) = args[i + 1].parse() {
                        config.health_port = port;
                    }
                }
            }
            "--storage" => {
                if i + 1 < args.len() {
                    config.storage_backend = match args[i + 1].as_str() {
                        "memory" => StorageConfig::Memory,
                        "json" => {
                            let path = if i + 2 < args.len() {
                                std::path::PathBuf::from(&args[i + 2])
                            } else {
                                std::path::PathBuf::from("/tmp/vexgraph")
                            };
                            StorageConfig::JsonFile { path }
                        }
                        _ => StorageConfig::Memory,
                    };
                }
            }
            _ => {}
        }
    }
    
    config
}

fn print_usage(program: &str) {
    eprintln!("Usage: {} <mountpoint> [options]", program);
    eprintln!("\nOptions:");
    eprintln!("  -f, --foreground         Run in foreground");
    eprintln!("  -d, --debug             Enable debug output");
    eprintln!("  --no-monitoring         Disable monitoring");
    eprintln!("  --no-vexgraph          Disable VexGraph integration");
    eprintln!("  --semantic-search      Enable semantic search");
    eprintln!("  --health-port <port>   Health check port (default: 8080)");
    eprintln!("  --storage <type>       Storage backend: memory|json");
    eprintln!("  --storage json <path>  Use JSON storage at path");
    eprintln!("\nExamples:");
    eprintln!("  {} /mnt/vexfs -f", program);
    eprintln!("  {} /mnt/vexfs --storage json /var/lib/vexgraph", program);
    eprintln!("  {} /mnt/vexfs --health-port 9090 --semantic-search", program);
}