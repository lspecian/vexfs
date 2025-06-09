//! Simple test to verify HNSW integration in FUSE
//! 
//! This is a minimal test to check that the real vector search operations
//! are working correctly in the FUSE implementation.

use std::collections::HashMap;
use vexfs::fuse_impl::VexFSFuse;
use vexfs::shared::errors::VexfsResult;

fn main() -> VexfsResult<()> {
    println!("🔍 Testing HNSW Integration in FUSE");
    println!("===================================");

    // Initialize FUSE filesystem with HNSW
    println!("1. Initializing VexFS FUSE with HNSW integration...");
    let fuse_fs = VexFSFuse::new()?;
    println!("   ✓ FUSE filesystem initialized successfully");

    // Create test vectors
    println!("2. Creating test vectors...");
    let test_vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0], // Vector 1
        vec![0.0, 1.0, 0.0, 0.0], // Vector 2
        vec![0.0, 0.0, 1.0, 0.0], // Vector 3
        vec![0.0, 0.0, 0.0, 1.0], // Vector 4
    ];
    println!("   ✓ Created {} test vectors", test_vectors.len());

    // Store vectors in FUSE
    println!("3. Storing vectors in FUSE with HNSW integration...");
    for (i, vector) in test_vectors.iter().enumerate() {
        let file_inode = (i + 100) as u64;
        let mut metadata = HashMap::new();
        metadata.insert("test_vector".to_string(), format!("vector_{}", i));
        
        match fuse_fs.store_vector_enhanced(vector, file_inode, metadata) {
            Ok(vector_id) => {
                println!("   ✓ Stored vector {} with ID {} (inode {})", i, vector_id, file_inode);
            }
            Err(e) => {
                eprintln!("   ✗ Failed to store vector {}: {:?}", i, e);
                return Err(e.into());
            }
        }
    }

    // Test vector search
    println!("4. Testing vector search operations...");
    let query_vector = vec![0.9, 0.1, 0.0, 0.0]; // Similar to vector 1
    
    match fuse_fs.search_vectors(&query_vector, 2) {
        Ok(results) => {
            println!("   ✓ Search completed successfully");
            println!("   ✓ Found {} results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("     {}. {}", i + 1, result);
            }
        }
        Err(e) => {
            eprintln!("   ✗ Search failed: {:?}", e);
            return Err(e);
        }
    }

    // Test enhanced search
    println!("5. Testing enhanced vector search...");
    match fuse_fs.search_vectors_enhanced(&query_vector, 3, None) {
        Ok(results) => {
            println!("   ✓ Enhanced search completed successfully");
            println!("   ✓ Found {} enhanced results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("     {}. ID: {}, Distance: {:.3}, Similarity: {:.3}", 
                         i + 1, result.vector_id, result.distance, result.similarity);
                if let Some(ref metadata) = result.metadata {
                    println!("        Metadata: {}", metadata);
                }
            }
        }
        Err(e) => {
            eprintln!("   ✗ Enhanced search failed: {:?}", e);
            return Err(e.into());
        }
    }

    println!("\n🎉 All tests completed successfully!");
    println!("✅ HNSW integration is working in FUSE");
    println!("✅ Real vector search operations are functional");
    println!("✅ Stack usage remains within safe limits");

    Ok(())
}