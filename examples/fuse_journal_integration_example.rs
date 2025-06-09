//! FUSE Journal Integration Example
//!
//! This example demonstrates the complete FUSE journal integration functionality
//! implemented in Task 23.4.4, showing how FUSE filesystem operations are
//! automatically captured and journaled as semantic events.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tempfile::tempdir;
use uuid::Uuid;

// Import VexFS semantic API components
use vexfs::semantic_api::{
    // Core types
    SemanticEvent, SemanticEventType, SemanticTimestamp, EventFlags, EventPriority,
    SemanticResult, SemanticError,
    
    // FUSE journal integration components
    FuseJournalIntegration, FuseJournalConfig, FuseOperationType, FuseOperationContext,
    FuseEventMapper, FuseEventMappingConfig, FuseMappingContext,
    FuseJournalManager, FuseJournalManagerConfig, FuseMountInfo, FusePerformanceMode,
    
    // Userspace journal components
    UserspaceSemanticJournal, UserspaceJournalConfig, CompressionAlgorithm,
};

/// Example demonstrating basic FUSE journal integration setup
async fn demonstrate_basic_setup() -> SemanticResult<()> {
    println!("=== FUSE Journal Integration Basic Setup ===");
    
    let temp_dir = tempdir().unwrap();
    println!("Created temporary directory: {:?}", temp_dir.path());
    
    // Create FUSE journal manager
    let manager_config = FuseJournalManagerConfig::default();
    let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
    println!("✓ Created FUSE journal manager");
    
    // Create FUSE event mapper
    let mapper_config = FuseEventMappingConfig::default();
    let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
    println!("✓ Created FUSE event mapper");
    
    // Create FUSE journal integration
    let integration_config = FuseJournalConfig::default();
    let integration = FuseJournalIntegration::new(
        integration_config,
        journal_manager.clone(),
        event_mapper.clone(),
    )?;
    println!("✓ Created FUSE journal integration");
    
    // Register a FUSE mount
    let mount_id = journal_manager.register_mount(
        temp_dir.path(),
        None,
        Some(FusePerformanceMode::Balanced),
        None,
    )?;
    println!("✓ Registered FUSE mount with ID: {}", mount_id);
    
    // Display configuration
    println!("\nConfiguration:");
    println!("  - Integration enabled: {}", integration.is_enabled());
    println!("  - Active operations: {}", integration.get_active_operation_count());
    println!("  - Active mounts: {}", journal_manager.list_active_mounts().len());
    
    Ok(())
}

/// Example demonstrating FUSE operation tracking and event generation
async fn demonstrate_operation_tracking() -> SemanticResult<()> {
    println!("\n=== FUSE Operation Tracking and Event Generation ===");
    
    let temp_dir = tempdir().unwrap();
    
    // Setup components
    let manager_config = FuseJournalManagerConfig::default();
    let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
    
    let mapper_config = FuseEventMappingConfig::default();
    let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
    
    let integration_config = FuseJournalConfig::default();
    let integration = FuseJournalIntegration::new(
        integration_config,
        journal_manager.clone(),
        event_mapper.clone(),
    )?;
    
    // Register mount
    let mount_id = journal_manager.register_mount(
        temp_dir.path(),
        None,
        Some(FusePerformanceMode::Balanced),
        None,
    )?;
    
    // Demonstrate different operation types
    let operations = vec![
        ("File Creation", FuseOperationType::Create, "document.txt"),
        ("File Write", FuseOperationType::Write, "document.txt"),
        ("File Read", FuseOperationType::Read, "document.txt"),
        ("Directory Creation", FuseOperationType::Mkdir, "subdirectory"),
        ("Vector Search", FuseOperationType::VectorSearch, "vectors.idx"),
        ("Graph Node Creation", FuseOperationType::NodeCreate, "graph.db"),
        ("System Sync", FuseOperationType::Sync, ""),
    ];
    
    for (description, operation_type, filename) in operations {
        println!("\n--- {} ---", description);
        
        // Start operation tracking
        let mut metadata = HashMap::new();
        metadata.insert("operation_description".to_string(), description.to_string());
        metadata.insert("example_run".to_string(), "true".to_string());
        
        let file_path = if !filename.is_empty() {
            temp_dir.path().join(filename)
        } else {
            temp_dir.path().to_path_buf()
        };
        
        let operation_id = integration.start_operation(
            operation_type,
            12345, // inode
            &file_path,
            1000,  // user_id
            1000,  // group_id
            54321, // process_id
            metadata,
        )?;
        
        println!("  Started operation tracking (ID: {})", operation_id);
        println!("  Operation type: {:?}", operation_type);
        println!("  File path: {:?}", file_path);
        
        // Simulate operation execution time
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Add operation-specific data
        let vector_data = if matches!(operation_type, FuseOperationType::VectorSearch) {
            Some(vec![0.1, 0.2, 0.3, 0.4, 0.5])
        } else {
            None
        };
        
        let graph_data = if matches!(operation_type, FuseOperationType::NodeCreate) {
            let mut data = HashMap::new();
            data.insert("node_type".to_string(), "document".to_string());
            data.insert("node_id".to_string(), "node_12345".to_string());
            Some(data)
        } else {
            None
        };
        
        // Complete operation tracking
        integration.complete_operation(
            operation_id,
            Ok(()), // Success result
            Some(1024), // File size
            vector_data,
            graph_data,
        )?;
        
        println!("  ✓ Completed operation tracking");
        
        // Show current metrics
        let metrics = integration.get_metrics();
        println!("  Total operations: {}", metrics.total_operations.load(std::sync::atomic::Ordering::Relaxed));
        println!("  Events generated: {}", metrics.events_generated.load(std::sync::atomic::Ordering::Relaxed));
        println!("  Average latency: {:.2}μs", metrics.get_average_latency_ns() / 1000.0);
    }
    
    Ok(())
}

/// Example demonstrating event mapping functionality
fn demonstrate_event_mapping() -> SemanticResult<()> {
    println!("\n=== FUSE Event Mapping Demonstration ===");
    
    let mapper = FuseEventMapper::new_default();
    
    // Demonstrate mapping for different operation categories
    let test_operations = vec![
        // Filesystem operations
        (FuseOperationType::Create, "Filesystem"),
        (FuseOperationType::Write, "Filesystem"),
        (FuseOperationType::Read, "Filesystem"),
        (FuseOperationType::Delete, "Filesystem"),
        
        // Vector operations
        (FuseOperationType::VectorSearch, "Vector"),
        (FuseOperationType::VectorInsert, "Vector"),
        (FuseOperationType::VectorUpdate, "Vector"),
        (FuseOperationType::VectorDelete, "Vector"),
        
        // Graph operations
        (FuseOperationType::NodeCreate, "Graph"),
        (FuseOperationType::EdgeCreate, "Graph"),
        (FuseOperationType::NodeUpdate, "Graph"),
        (FuseOperationType::EdgeDelete, "Graph"),
        
        // System operations
        (FuseOperationType::Mount, "System"),
        (FuseOperationType::Sync, "System"),
        (FuseOperationType::Statfs, "System"),
    ];
    
    println!("\nOperation to Semantic Event Mapping:");
    println!("{:<20} {:<25} {:<15} {:<10}", "FUSE Operation", "Semantic Event", "Category", "Priority");
    println!("{}", "-".repeat(75));
    
    for (operation, expected_category) in test_operations {
        let semantic_event = mapper.map_operation_to_completion_event(operation, true)?;
        let category = mapper.map_operation_to_category(operation);
        let priority = mapper.determine_event_priority(operation);
        
        println!("{:<20} {:<25} {:<15} {:<10}", 
                format!("{:?}", operation),
                format!("{:?}", semantic_event),
                format!("{:?}", category),
                format!("{:?}", priority));
    }
    
    // Demonstrate metadata extraction
    println!("\n--- Metadata Extraction Example ---");
    
    let context = FuseMappingContext {
        operation_type: FuseOperationType::Write,
        path: "/documents/research/paper.pdf".to_string(),
        inode: 98765,
        user_id: 1001,
        group_id: 1001,
        process_id: 12345,
        file_size: Some(2048576), // 2MB
        file_type: Some("regular".to_string()),
        permissions: Some(0o644),
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("document_type".to_string(), "research_paper".to_string());
            meta.insert("author".to_string(), "researcher".to_string());
            meta
        },
    };
    
    let extracted_metadata = mapper.extract_operation_metadata(&context);
    
    println!("Extracted metadata for Write operation:");
    for (key, value) in extracted_metadata.iter() {
        println!("  {}: {}", key, value);
    }
    
    Ok(())
}

/// Example demonstrating performance modes and configuration
async fn demonstrate_performance_modes() -> SemanticResult<()> {
    println!("\n=== Performance Modes Demonstration ===");
    
    let temp_dir = tempdir().unwrap();
    let manager = FuseJournalManager::new(FuseJournalManagerConfig::default())?;
    
    let performance_modes = vec![
        ("High Performance", FusePerformanceMode::HighPerformance),
        ("Balanced", FusePerformanceMode::Balanced),
        ("High Reliability", FusePerformanceMode::HighReliability),
    ];
    
    for (mode_name, performance_mode) in performance_modes {
        println!("\n--- {} Mode ---", mode_name);
        
        // Create mount info for this performance mode
        let mount_info = FuseMountInfo {
            mount_id: Uuid::new_v4(),
            mount_path: temp_dir.path().to_path_buf(),
            device_path: None,
            mount_time: std::time::SystemTime::now(),
            journal_enabled: true,
            journal_path: temp_dir.path().join(".vexfs_journal"),
            performance_mode,
            metadata: HashMap::new(),
        };
        
        // Generate journal configuration for this mode
        let journal_config = manager.create_journal_config(&mount_info)?;
        
        println!("Configuration for {} mode:", mode_name);
        println!("  Target emission latency: {}ns", journal_config.target_emission_latency_ns);
        println!("  Buffer size: {}", journal_config.buffer_size);
        println!("  Compression enabled: {}", journal_config.enable_compression);
        println!("  Sync frequency: {}ms", journal_config.sync_frequency_ms);
        println!("  Max memory usage: {}MB", journal_config.max_memory_usage_mb);
        
        // Show performance characteristics
        match performance_mode {
            FusePerformanceMode::HighPerformance => {
                println!("  Optimized for: Maximum throughput and minimal latency");
                println!("  Trade-offs: Reduced durability guarantees");
            },
            FusePerformanceMode::Balanced => {
                println!("  Optimized for: Balance of performance and reliability");
                println!("  Trade-offs: Moderate performance with good durability");
            },
            FusePerformanceMode::HighReliability => {
                println!("  Optimized for: Maximum data integrity and durability");
                println!("  Trade-offs: Higher latency for stronger guarantees");
            },
        }
    }
    
    Ok(())
}

/// Example demonstrating multi-mount support
async fn demonstrate_multi_mount_support() -> SemanticResult<()> {
    println!("\n=== Multi-Mount Support Demonstration ===");
    
    let manager = FuseJournalManager::new(FuseJournalManagerConfig::default())?;
    
    // Create multiple temporary directories for different mounts
    let temp_dirs: Vec<_> = (0..3).map(|_| tempdir().unwrap()).collect();
    let mut mount_ids = Vec::new();
    
    // Register multiple mounts with different configurations
    for (i, temp_dir) in temp_dirs.iter().enumerate() {
        let performance_mode = match i {
            0 => FusePerformanceMode::HighPerformance,
            1 => FusePerformanceMode::Balanced,
            2 => FusePerformanceMode::HighReliability,
            _ => FusePerformanceMode::Balanced,
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("mount_purpose".to_string(), format!("example_mount_{}", i + 1));
        metadata.insert("performance_mode".to_string(), format!("{:?}", performance_mode));
        
        let mount_id = manager.register_mount(
            temp_dir.path(),
            None,
            Some(performance_mode),
            Some(metadata),
        )?;
        
        mount_ids.push(mount_id);
        
        println!("Registered mount {} (ID: {})", i + 1, mount_id);
        println!("  Path: {:?}", temp_dir.path());
        println!("  Performance mode: {:?}", performance_mode);
    }
    
    // Display active mounts
    println!("\nActive mounts summary:");
    let active_mounts = manager.list_active_mounts();
    println!("Total active mounts: {}", active_mounts.len());
    
    for mount_info in active_mounts {
        println!("  Mount ID: {}", mount_info.mount_id);
        println!("    Path: {:?}", mount_info.mount_path);
        println!("    Performance mode: {:?}", mount_info.performance_mode);
        println!("    Journal enabled: {}", mount_info.journal_enabled);
        if let Some(purpose) = mount_info.metadata.get("mount_purpose") {
            println!("    Purpose: {}", purpose);
        }
        println!();
    }
    
    // Show manager metrics
    let metrics = manager.get_metrics();
    println!("Manager metrics:");
    println!("  Active mounts: {}", metrics.active_mounts.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Total mounts created: {}", metrics.total_mounts_created.load(std::sync::atomic::Ordering::Relaxed));
    
    // Cleanup - unregister mounts
    for mount_id in mount_ids {
        manager.unregister_mount(mount_id)?;
        println!("Unregistered mount: {}", mount_id);
    }
    
    println!("All mounts unregistered successfully");
    
    Ok(())
}

/// Example demonstrating error handling and recovery
async fn demonstrate_error_handling() -> SemanticResult<()> {
    println!("\n=== Error Handling and Recovery Demonstration ===");
    
    let temp_dir = tempdir().unwrap();
    
    // Setup components
    let manager_config = FuseJournalManagerConfig::default();
    let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
    
    let mapper_config = FuseEventMappingConfig::default();
    let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
    
    let integration_config = FuseJournalConfig::default();
    let integration = FuseJournalIntegration::new(
        integration_config,
        journal_manager.clone(),
        event_mapper.clone(),
    )?;
    
    // Register mount
    let mount_id = journal_manager.register_mount(
        temp_dir.path(),
        None,
        Some(FusePerformanceMode::Balanced),
        None,
    )?;
    
    // Test 1: Operation when integration is disabled
    println!("--- Test 1: Operation when integration disabled ---");
    integration.set_enabled(false);
    
    let operation_id = integration.start_operation(
        FuseOperationType::Create,
        12345,
        temp_dir.path().join("test_file.txt").as_path(),
        1000,
        1000,
        54321,
        HashMap::new(),
    )?;
    
    println!("Operation ID when disabled: {} (should be 0)", operation_id);
    assert_eq!(operation_id, 0);
    
    // Re-enable for further tests
    integration.set_enabled(true);
    println!("✓ Integration re-enabled");
    
    // Test 2: Error operation handling
    println!("\n--- Test 2: Error operation handling ---");
    
    let operation_id = integration.start_operation(
        FuseOperationType::Create,
        12345,
        temp_dir.path().join("error_file.txt").as_path(),
        1000,
        1000,
        54321,
        HashMap::new(),
    )?;
    
    println!("Started operation for error test (ID: {})", operation_id);
    
    // Complete with error
    integration.complete_operation(
        operation_id,
        Err(libc::EACCES), // Permission denied error
        None,
        None,
        None,
    )?;
    
    println!("✓ Completed operation with error (EACCES)");
    
    // Test 3: Graceful shutdown
    println!("\n--- Test 3: Graceful shutdown ---");
    
    // Start an operation
    let operation_id = integration.start_operation(
        FuseOperationType::Write,
        12345,
        temp_dir.path().join("shutdown_test.txt").as_path(),
        1000,
        1000,
        54321,
        HashMap::new(),
    )?;
    
    println!("Started operation before shutdown (ID: {})", operation_id);
    println!("Active operations before shutdown: {}", integration.get_active_operation_count());
    
    // Shutdown integration
    integration.shutdown()?;
    println!("✓ Integration shutdown completed");
    println!("Integration enabled after shutdown: {}", integration.is_enabled());
    println!("Active operations after shutdown: {}", integration.get_active_operation_count());
    
    // Shutdown journal manager
    journal_manager.shutdown()?;
    println!("✓ Journal manager shutdown completed");
    println!("Manager enabled after shutdown: {}", journal_manager.is_enabled());
    println!("Active mounts after shutdown: {}", journal_manager.list_active_mounts().len());
    
    Ok(())
}

/// Main example function demonstrating all FUSE journal integration features
#[tokio::main]
async fn main() -> SemanticResult<()> {
    println!("VexFS FUSE Journal Integration Example");
    println!("=====================================");
    println!();
    println!("This example demonstrates the complete FUSE journal integration");
    println!("functionality implemented in Task 23.4.4, showing how FUSE");
    println!("filesystem operations are automatically captured and journaled");
    println!("as semantic events with high performance and reliability.");
    println!();
    
    // Run all demonstration functions
    demonstrate_basic_setup().await?;
    demonstrate_operation_tracking().await?;
    demonstrate_event_mapping()?;
    demonstrate_performance_modes().await?;
    demonstrate_multi_mount_support().await?;
    demonstrate_error_handling().await?;
    
    println!("\n=== Example Completed Successfully ===");
    println!();
    println!("Key features demonstrated:");
    println!("✓ Basic FUSE journal integration setup");
    println!("✓ Automatic operation tracking and event generation");
    println!("✓ Comprehensive event mapping for all operation types");
    println!("✓ Performance mode configuration and optimization");
    println!("✓ Multi-mount support with independent journaling");
    println!("✓ Error handling and graceful recovery mechanisms");
    println!();
    println!("The FUSE journal integration provides seamless semantic event");
    println!("capture for all filesystem operations while maintaining high");
    println!("performance and reliability standards.");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_setup_example() {
        assert!(demonstrate_basic_setup().await.is_ok());
    }
    
    #[test]
    fn test_event_mapping_example() {
        assert!(demonstrate_event_mapping().is_ok());
    }
    
    #[tokio::test]
    async fn test_performance_modes_example() {
        assert!(demonstrate_performance_modes().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_multi_mount_example() {
        assert!(demonstrate_multi_mount_support().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_error_handling_example() {
        assert!(demonstrate_error_handling().await.is_ok());
    }
}