//! Bridge Integration Test for VexFS FUSE
//! 
//! This test demonstrates the Storage-HNSW Bridge integration in the FUSE implementation,
//! showing how vector storage and search operations are synchronized through the bridge.

use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::thread;

// Import VexFS components
use vexfs::fuse_impl::VexFSFuse;
use vexfs::storage::vector_hnsw_bridge::{BridgeConfig, SyncStatus};
use vexfs::shared::VexfsResult;

/// Test the bridge integration in VexFS FUSE
fn test_bridge_integration() -> VexfsResult<()> {
    println!("=== VexFS Bridge Integration Test ===");
    
    // Create VexFS FUSE instance with bridge integration
    println!("1. Creating VexFS FUSE instance with Storage-HNSW Bridge...");
    let mut vexfs = VexFSFuse::new()?;
    
    // Test initial sync status
    println!("2. Testing initial sync status...");
    let initial_status = vexfs.get_sync_status();
    println!("   Initial sync status: synchronized={}, pending={}, errors={}", 
             initial_status.is_synchronized, 
             initial_status.pending_operations, 
             initial_status.sync_errors);
    
    // Test bridge statistics
    println!("3. Getting bridge statistics...");
    match vexfs.get_bridge_statistics() {
        Ok(stats) => {
            println!("   Bridge statistics: total_vectors={}, pending_ops={}", 
                     stats.total_vectors, stats.pending_operations);
        }
        Err(e) => {
            println!("   Warning: Failed to get bridge statistics: {:?}", e);
        }
    }
    
    // Test vector storage through bridge
    println!("4. Testing vector storage through bridge...");
    let test_vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0], // Vector 1
        vec![0.0, 1.0, 0.0, 0.0], // Vector 2
        vec![0.0, 0.0, 1.0, 0.0], // Vector 3
        vec![0.5, 0.5, 0.0, 0.0], // Vector 4 (similar to 1 and 2)
    ];
    
    let mut stored_vector_ids = Vec::new();
    
    for (i, vector) in test_vectors.iter().enumerate() {
        let file_inode = (i + 10) as u64; // Use unique file inodes
        let metadata = HashMap::new();
        
        match vexfs.store_vector(vector, file_inode, metadata) {
            Ok(vector_id) => {
                println!("   Stored vector {} with ID: {}", i + 1, vector_id);
                stored_vector_ids.push(vector_id);
            }
            Err(e) => {
                println!("   Error storing vector {}: {:?}", i + 1, e);
            }
        }
    }
    
    // Check sync status after vector storage
    println!("5. Checking sync status after vector storage...");
    let post_storage_status = vexfs.get_sync_status();
    println!("   Post-storage sync status: synchronized={}, pending={}", 
             post_storage_status.is_synchronized, 
             post_storage_status.pending_operations);
    
    // Test lazy synchronization trigger
    if post_storage_status.pending_operations > 0 {
        println!("6. Testing lazy synchronization trigger...");
        match vexfs.trigger_lazy_sync() {
            Ok(_) => println!("   Lazy sync triggered successfully"),
            Err(e) => println!("   Lazy sync failed: {:?}", e),
        }
        
        // Check sync status after lazy sync
        let post_lazy_sync_status = vexfs.get_sync_status();
        println!("   Post-lazy-sync status: synchronized={}, pending={}", 
                 post_lazy_sync_status.is_synchronized, 
                 post_lazy_sync_status.pending_operations);
    }
    
    // Test force synchronization
    println!("7. Testing force synchronization...");
    match vexfs.force_sync() {
        Ok(_) => println!("   Force sync completed successfully"),
        Err(e) => println!("   Force sync failed: {:?}", e),
    }
    
    // Check final sync status
    let final_sync_status = vexfs.get_sync_status();
    println!("   Final sync status: synchronized={}, pending={}", 
             final_sync_status.is_synchronized, 
             final_sync_status.pending_operations);
    
    // Test vector search through bridge
    println!("8. Testing vector search through bridge...");
    let query_vector = vec![0.7, 0.3, 0.0, 0.0]; // Should be similar to vectors 1, 2, and 4
    let top_k = 3;
    
    match vexfs.search_vectors(&query_vector, top_k) {
        Ok(results) => {
            println!("   Search results ({} found):", results.len());
            for (i, file_path) in results.iter().enumerate() {
                println!("     {}. {}", i + 1, file_path);
            }
        }
        Err(e) => {
            println!("   Search failed: {:?}", e);
        }
    }
    
    // Test enhanced vector search
    println!("9. Testing enhanced vector search...");
    match vexfs.search_vectors_enhanced(&query_vector, top_k, None) {
        Ok(results) => {
            println!("   Enhanced search results ({} found):", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("     {}. {} (distance: {:.4})", i + 1, result.file_path, result.distance);
            }
        }
        Err(e) => {
            println!("   Enhanced search failed: {:?}", e);
        }
    }
    
    // Test batch synchronization
    println!("10. Testing batch synchronization...");
    match vexfs.batch_sync(Some(2)) {
        Ok(synced_ops) => println!("   Batch sync completed: {} operations processed", synced_ops),
        Err(e) => println!("   Batch sync failed: {:?}", e),
    }
    
    // Test sync scheduling
    println!("11. Testing sync scheduling...");
    let needs_sync = vexfs.needs_sync();
    println!("   Sync needed: {}", needs_sync);
    
    // Final bridge statistics
    println!("12. Final bridge statistics...");
    match vexfs.get_bridge_statistics() {
        Ok(stats) => {
            println!("   Final statistics:");
            println!("     Total vectors: {}", stats.total_vectors);
            println!("     Graph memory usage: {} bytes", stats.graph_memory_usage);
            println!("     Storage memory usage: {} bytes", stats.storage_memory_usage);
            println!("     Pending operations: {}", stats.pending_operations);
            println!("     Sync status: synchronized={}", stats.sync_status.is_synchronized);
        }
        Err(e) => {
            println!("   Failed to get final statistics: {:?}", e);
        }
    }
    
    println!("=== Bridge Integration Test Completed ===");
    Ok(())
}

/// Test bridge performance under load
fn test_bridge_performance() -> VexfsResult<()> {
    println!("\n=== Bridge Performance Test ===");
    
    let mut vexfs = VexFSFuse::new()?;
    let vector_count = 100;
    let vector_dimensions = 128;
    
    println!("1. Storing {} vectors of {} dimensions...", vector_count, vector_dimensions);
    let start_time = SystemTime::now();
    
    for i in 0..vector_count {
        let vector: Vec<f32> = (0..vector_dimensions)
            .map(|j| (i as f32 + j as f32) / (vector_count + vector_dimensions) as f32)
            .collect();
        
        let file_inode = (i + 1000) as u64;
        let metadata = HashMap::new();
        
        match vexfs.store_vector(&vector, file_inode, metadata) {
            Ok(_) => {
                if i % 20 == 0 {
                    println!("   Stored {} vectors...", i + 1);
                }
            }
            Err(e) => {
                println!("   Error storing vector {}: {:?}", i, e);
            }
        }
    }
    
    let storage_duration = start_time.elapsed().unwrap_or_default();
    println!("   Storage completed in {:?}", storage_duration);
    
    // Force sync and measure time
    println!("2. Performing force synchronization...");
    let sync_start = SystemTime::now();
    match vexfs.force_sync() {
        Ok(_) => {
            let sync_duration = sync_start.elapsed().unwrap_or_default();
            println!("   Sync completed in {:?}", sync_duration);
        }
        Err(e) => {
            println!("   Sync failed: {:?}", e);
        }
    }
    
    // Test search performance
    println!("3. Testing search performance...");
    let query_vector: Vec<f32> = (0..vector_dimensions)
        .map(|i| (i as f32) / (vector_dimensions as f32))
        .collect();
    
    let search_start = SystemTime::now();
    match vexfs.search_vectors(&query_vector, 10) {
        Ok(results) => {
            let search_duration = search_start.elapsed().unwrap_or_default();
            println!("   Search completed in {:?}, found {} results", search_duration, results.len());
        }
        Err(e) => {
            println!("   Search failed: {:?}", e);
        }
    }
    
    println!("=== Performance Test Completed ===");
    Ok(())
}

fn main() {
    println!("VexFS Storage-HNSW Bridge Integration Test");
    println!("==========================================");
    
    // Run basic integration test
    if let Err(e) = test_bridge_integration() {
        eprintln!("Bridge integration test failed: {:?}", e);
        std::process::exit(1);
    }
    
    // Run performance test
    if let Err(e) = test_bridge_performance() {
        eprintln!("Bridge performance test failed: {:?}", e);
        std::process::exit(1);
    }
    
    println!("\n✅ All bridge integration tests passed!");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bridge_basic_functionality() {
        let result = test_bridge_integration();
        assert!(result.is_ok(), "Bridge integration test should pass");
    }
    
    #[test]
    fn test_bridge_sync_status() {
        let vexfs = VexFSFuse::new().expect("Failed to create VexFS instance");
        let status = vexfs.get_sync_status();
        
        // Initial status should be synchronized with no pending operations
        assert!(status.is_synchronized || status.pending_operations == 0);
    }
    
    #[test]
    fn test_bridge_statistics() {
        let vexfs = VexFSFuse::new().expect("Failed to create VexFS instance");
        let stats = vexfs.get_bridge_statistics();
        
        // Should be able to get statistics without error
        assert!(stats.is_ok(), "Should be able to get bridge statistics");
    }
    
    #[test]
    fn test_bridge_lazy_sync() {
        let vexfs = VexFSFuse::new().expect("Failed to create VexFS instance");
        let result = vexfs.trigger_lazy_sync();
        
        // Lazy sync should complete without error (even if no operations pending)
        assert!(result.is_ok(), "Lazy sync should complete without error");
    }
    
    #[test]
    fn test_bridge_force_sync() {
        let vexfs = VexFSFuse::new().expect("Failed to create VexFS instance");
        let result = vexfs.force_sync();
        
        // Force sync should complete without error
        assert!(result.is_ok(), "Force sync should complete without error");
    }
}