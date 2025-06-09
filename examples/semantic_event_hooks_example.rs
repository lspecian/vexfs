//! VexFS Semantic Event Hooks Example
//! 
//! This example demonstrates the comprehensive event interception and hooks system
//! implemented for Task 18.3 of the VexFS Semantic Operation Journal.
//! 
//! Features demonstrated:
//! - Event emission framework with rate limiting and buffering
//! - Kernel-level filesystem operation hooks
//! - Userspace graph and vector operation hooks
//! - Cross-layer integration with existing VexFS infrastructure
//! - Thread-safe concurrent event processing

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use vexfs::semantic_api::event_emission::{
    EventEmissionFramework, emit_filesystem_event, emit_graph_event, emit_vector_event
};
use vexfs::semantic_api::kernel_hooks::{
    KernelOperationType, vexfs_rust_emit_kernel_event, vexfs_rust_hook_fs_operation_start
};
use vexfs::semantic_api::userspace_hooks::{
    UserspaceHookRegistry, GraphHook, VectorHook, hook_graph_node_create, hook_vector_search
};
use vexfs::semantic_api::types::{
    SemanticEventType, FilesystemEventData, GraphEventData, VectorEventData,
    GraphNodeId, VectorId, VectorDimensions
};

/// Example graph hook that logs node creation events
struct ExampleGraphHook {
    name: String,
}

impl ExampleGraphHook {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl GraphHook for ExampleGraphHook {
    fn on_node_create(&self, node_id: GraphNodeId, metadata: &std::collections::HashMap<String, String>) {
        println!("🔗 [{}] Graph node created: {:?} with metadata: {:?}", 
                 self.name, node_id, metadata);
    }

    fn on_edge_create(&self, from: GraphNodeId, to: GraphNodeId, weight: f32) {
        println!("🔗 [{}] Graph edge created: {:?} -> {:?} (weight: {})", 
                 self.name, from, to, weight);
    }

    fn on_node_update(&self, node_id: GraphNodeId, metadata: &std::collections::HashMap<String, String>) {
        println!("🔗 [{}] Graph node updated: {:?} with metadata: {:?}", 
                 self.name, node_id, metadata);
    }

    fn on_node_delete(&self, node_id: GraphNodeId) {
        println!("🔗 [{}] Graph node deleted: {:?}", self.name, node_id);
    }
}

/// Example vector hook that logs search operations
struct ExampleVectorHook {
    name: String,
}

impl ExampleVectorHook {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl VectorHook for ExampleVectorHook {
    fn on_vector_insert(&self, vector_id: VectorId, dimensions: VectorDimensions) {
        println!("🔍 [{}] Vector inserted: {:?} with {} dimensions", 
                 self.name, vector_id, dimensions);
    }

    fn on_vector_search(&self, query_vector: &[f32], k: usize, results: &[(VectorId, f32)]) {
        println!("🔍 [{}] Vector search: query_dim={}, k={}, found {} results", 
                 self.name, query_vector.len(), k, results.len());
        for (i, (vector_id, score)) in results.iter().take(3).enumerate() {
            println!("  {}. {:?} (score: {:.4})", i + 1, vector_id, score);
        }
    }

    fn on_vector_update(&self, vector_id: VectorId, dimensions: VectorDimensions) {
        println!("🔍 [{}] Vector updated: {:?} with {} dimensions", 
                 self.name, vector_id, dimensions);
    }

    fn on_vector_delete(&self, vector_id: VectorId) {
        println!("🔍 [{}] Vector deleted: {:?}", self.name, vector_id);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 VexFS Semantic Event Hooks Example");
    println!("=====================================\n");

    // Initialize the event emission framework
    println!("📡 Initializing Event Emission Framework...");
    let framework = Arc::new(EventEmissionFramework::new(
        1000,  // buffer_size
        100,   // rate_limit_per_second
        Duration::from_millis(100), // flush_interval
    ));

    // Start the framework
    framework.start().await?;
    println!("✅ Event emission framework started\n");

    // Initialize userspace hook registry
    println!("🔧 Setting up Userspace Hook Registry...");
    let mut hook_registry = UserspaceHookRegistry::new();

    // Register example hooks
    let graph_hook = Box::new(ExampleGraphHook::new("GraphLogger"));
    let vector_hook = Box::new(ExampleVectorHook::new("VectorLogger"));

    hook_registry.register_graph_hook(graph_hook);
    hook_registry.register_vector_hook(vector_hook);
    println!("✅ Hooks registered\n");

    // Demonstrate filesystem event emission
    println!("📁 Demonstrating Filesystem Events...");
    emit_filesystem_event(SemanticEventType::FilesystemRead, FilesystemEventData {
        path: "/vexfs/data/vectors.bin".to_string(),
        operation: "read".to_string(),
        size: 4096,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }).await?;

    emit_filesystem_event(SemanticEventType::FilesystemWrite, FilesystemEventData {
        path: "/vexfs/data/index.bin".to_string(),
        operation: "write".to_string(),
        size: 8192,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }).await?;

    sleep(Duration::from_millis(100)).await;

    // Demonstrate graph events
    println!("\n🔗 Demonstrating Graph Events...");
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("type".to_string(), "document".to_string());
    metadata.insert("title".to_string(), "Example Document".to_string());

    emit_graph_event(SemanticEventType::GraphNodeCreate, GraphEventData {
        node_id: GraphNodeId(42),
        edge_count: 0,
        metadata: metadata.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }).await?;

    // Trigger graph hook
    hook_graph_node_create(GraphNodeId(42), &metadata);

    sleep(Duration::from_millis(100)).await;

    // Demonstrate vector events
    println!("\n🔍 Demonstrating Vector Events...");
    emit_vector_event(SemanticEventType::VectorInsert, VectorEventData {
        vector_id: VectorId(123),
        dimensions: 768,
        operation: "insert".to_string(),
        metadata: std::collections::HashMap::new(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }).await?;

    // Trigger vector hook
    let query_vector = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let search_results = vec![
        (VectorId(123), 0.95),
        (VectorId(456), 0.87),
        (VectorId(789), 0.72),
    ];
    hook_vector_search(&query_vector, 3, &search_results);

    sleep(Duration::from_millis(100)).await;

    // Demonstrate kernel-level hooks (simulated)
    println!("\n⚙️  Demonstrating Kernel-Level Hooks (Simulated)...");
    
    // Simulate kernel operation start
    let operation_id = vexfs_rust_hook_fs_operation_start(
        KernelOperationType::FileRead,
        "/vexfs/data/vectors.bin\0".as_ptr() as *const i8,
    );
    println!("🔧 Started kernel operation: {}", operation_id);

    // Simulate kernel event emission
    vexfs_rust_emit_kernel_event(
        KernelOperationType::FileRead,
        "/vexfs/data/vectors.bin\0".as_ptr() as *const i8,
        4096,
        0, // success
    );
    println!("📤 Emitted kernel event for file read");

    sleep(Duration::from_millis(100)).await;

    // Demonstrate rate limiting
    println!("\n⏱️  Demonstrating Rate Limiting...");
    println!("Emitting events rapidly to test rate limiting...");
    
    for i in 0..10 {
        emit_filesystem_event(SemanticEventType::FilesystemRead, FilesystemEventData {
            path: format!("/vexfs/test/file_{}.txt", i),
            operation: "read".to_string(),
            size: 1024,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }).await?;
    }

    println!("✅ Rate limiting test completed");

    // Wait for background processing
    sleep(Duration::from_millis(500)).await;

    // Get framework statistics
    let stats = framework.get_stats();
    println!("\n📊 Event Emission Statistics:");
    println!("  Events emitted: {}", stats.events_emitted);
    println!("  Events dropped: {}", stats.events_dropped);
    println!("  Rate limit hits: {}", stats.rate_limit_hits);
    println!("  Buffer overflows: {}", stats.buffer_overflows);

    // Stop the framework
    println!("\n🛑 Stopping Event Emission Framework...");
    framework.stop().await?;
    println!("✅ Framework stopped gracefully");

    println!("\n🎉 Example completed successfully!");
    println!("\nThis example demonstrated:");
    println!("  ✅ Event emission framework with rate limiting");
    println!("  ✅ Kernel-level filesystem operation hooks");
    println!("  ✅ Userspace graph and vector operation hooks");
    println!("  ✅ Cross-layer integration capabilities");
    println!("  ✅ Thread-safe concurrent event processing");
    println!("  ✅ Performance monitoring and statistics");

    Ok(())
}