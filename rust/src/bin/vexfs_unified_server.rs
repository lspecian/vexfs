//! VexFS Unified Server
//! 
//! A multi-dialect vector database server that provides ChromaDB, Qdrant, and Native VexFS
//! APIs using the same high-performance VexFS backend engine.

use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use vexfs::dialects::router::create_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vexfs_unified_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse configuration from environment variables
    let config = ServerConfig::from_env();
    
    info!("🚀 Starting VexFS Unified Server");
    info!("📍 Listening on: {}", config.bind_address);
    info!("🔧 Configuration: {:#?}", config);
    
    // Print API information
    print_api_info(&config);
    
    // Create the router with all dialects
    let app = create_router();
    
    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    info!("✅ Server bound to {}", config.bind_address);
    
    // Start the server with graceful shutdown
    info!("🌟 VexFS Unified Server is ready!");
    info!("📖 Visit http://{} for server information", config.bind_address);
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    info!("🛑 VexFS Unified Server shutdown complete");
    Ok(())
}

/// Server configuration
#[derive(Debug, Clone)]
struct ServerConfig {
    bind_address: SocketAddr,
    log_level: String,
}

impl ServerConfig {
    fn from_env() -> Self {
        let host = std::env::var("VEXFS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("VEXFS_PORT")
            .or_else(|_| std::env::var("PORT"))  // Fallback to PORT for compatibility
            .unwrap_or_else(|_| "7680".to_string())  // Default to 7680
            .parse::<u16>()
            .unwrap_or(7680);
        let log_level = std::env::var("VEXFS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        
        Self {
            bind_address: SocketAddr::new(host.parse().unwrap_or([0, 0, 0, 0].into()), port),
            log_level,
        }
    }
}

/// Print API information and usage examples
fn print_api_info(config: &ServerConfig) {
    let base_url = format!("http://{}", config.bind_address);
    
    info!("📚 API Documentation:");
    info!("");
    info!("🔵 ChromaDB API (Compatible with ChromaDB clients):");
    info!("   Base URL: {}/api/v1", base_url);
    info!("   Collections: GET {}/api/v1/collections", base_url);
    info!("   Create Collection: POST {}/api/v1/collections", base_url);
    info!("   Add Documents: POST {}/api/v1/collections/{{collection}}/add", base_url);
    info!("   Query: POST {}/api/v1/collections/{{collection}}/query", base_url);
    info!("");
    info!("🟠 Qdrant API (Compatible with Qdrant clients):");
    info!("   Base URL: {}", base_url);
    info!("   Collections: GET {}/collections", base_url);
    info!("   Create Collection: PUT {}/collections/{{collection}}", base_url);
    info!("   Upsert Points: PUT {}/collections/{{collection}}/points", base_url);
    info!("   Search: POST {}/collections/{{collection}}/points/search", base_url);
    info!("");
    info!("🟢 Native VexFS API (Advanced VexFS features):");
    info!("   Base URL: {}/vexfs/v1", base_url);
    info!("   Collections: GET {}/vexfs/v1/collections", base_url);
    info!("   Create Collection: POST {}/vexfs/v1/collections", base_url);
    info!("   Add Documents: POST {}/vexfs/v1/collections/{{collection}}/documents", base_url);
    info!("   Search: POST {}/vexfs/v1/collections/{{collection}}/search", base_url);
    info!("   Health: GET {}/vexfs/v1/health", base_url);
    info!("");
    info!("🔧 Server Endpoints:");
    info!("   Server Info: GET {}/", base_url);
    info!("   Health Check: GET {}/health", base_url);
    info!("   Metrics: GET {}/metrics", base_url);
    info!("");
    info!("💡 Example Usage:");
    info!("   curl {}/", base_url);
    info!("   curl {}/health", base_url);
    info!("   curl {}/api/v1/collections", base_url);
    info!("   curl {}/collections", base_url);
    info!("   curl {}/vexfs/v1/collections", base_url);
    info!("");
    info!("🚀 Performance Target: 361,000+ operations/second");
    info!("⚡ Engine: VexFS High-Performance Vector Database");
    info!("");
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🛑 Received Ctrl+C, initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("🛑 Received SIGTERM, initiating graceful shutdown...");
        },
    }
}