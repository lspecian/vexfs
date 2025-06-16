//! Integration tests for FUSE vector operations
//! 
//! This test file verifies that the FUSE implementation correctly handles
//! vector storage, retrieval, and search operations with real implementations
//! instead of mocks.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use vexfs::fuse_impl::{VexFSFuse, FuseVexfsError};
    use vexfs::storage::vector_hnsw_bridge::{SearchParameters, VectorSearchResult};
    
    /// Create a test instance of VexFSFuse
    fn create_test_fuse() -> VexFSFuse {
        VexFSFuse::new().expect("Failed to create VexFSFuse instance")
    }
    
    /// Generate a test vector with given dimensions
    fn generate_test_vector(dimensions: usize, seed: f32) -> Vec<f32> {
        (0..dimensions)
            .map(|i| seed + (i as f32) * 0.1)
            .collect()
    }
    
    #[test]
    fn test_store_and_retrieve_vector() {
        let fuse = create_test_fuse();
        let test_vector = generate_test_vector(128, 1.0);
        let file_inode = 1000;
        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());
        
        // Store the vector
        let vector_id = fuse.store_vector_enhanced(&test_vector, file_inode, metadata.clone())
            .expect("Failed to store vector");
        
        assert!(vector_id > 0, "Vector ID should be greater than 0");
        
        // Retrieve the vector
        let (retrieved_vector, retrieved_metadata) = fuse.get_vector_enhanced(vector_id)
            .expect("Failed to retrieve vector");
        
        // Verify the vector data
        assert_eq!(retrieved_vector.len(), test_vector.len());
        for (i, &value) in retrieved_vector.iter().enumerate() {
            assert!((value - test_vector[i]).abs() < 0.0001, 
                "Vector values should match at index {}", i);
        }
        
        // Verify metadata contains expected fields
        assert!(retrieved_metadata.contains_key("vector_id"));
        assert_eq!(retrieved_metadata.get("vector_id").unwrap(), &vector_id.to_string());
    }
    
    #[test]
    fn test_vector_search() {
        let fuse = create_test_fuse();
        
        // Store multiple vectors
        let vectors = vec![
            generate_test_vector(128, 1.0),
            generate_test_vector(128, 2.0),
            generate_test_vector(128, 3.0),
            generate_test_vector(128, 4.0),
            generate_test_vector(128, 5.0),
        ];
        
        let mut vector_ids = Vec::new();
        for (i, vector) in vectors.iter().enumerate() {
            let file_inode = 1000 + i as u64;
            let metadata = HashMap::new();
            let vector_id = fuse.store_vector_enhanced(vector, file_inode, metadata)
                .expect("Failed to store vector");
            vector_ids.push(vector_id);
        }
        
        // Search for similar vectors
        let query_vector = generate_test_vector(128, 2.5); // Should be close to the second vector
        let search_params = SearchParameters::default();
        
        let results = fuse.search_vectors_enhanced(&query_vector, 3, Some(search_params))
            .expect("Failed to search vectors");
        
        // Verify search results
        assert!(!results.is_empty(), "Search should return results");
        assert!(results.len() <= 3, "Should return at most top_k results");
        
        // Verify results have proper structure
        for result in &results {
            assert!(result.distance >= 0.0, "Distance should be non-negative");
            assert!(result.similarity >= 0.0 && result.similarity <= 1.0, 
                "Similarity should be between 0 and 1");
            assert!(result.metadata.is_some(), "Metadata should be present");
        }
        
        // Results should be sorted by distance (ascending)
        for i in 1..results.len() {
            assert!(results[i].distance >= results[i-1].distance, 
                "Results should be sorted by distance");
        }
    }
    
    #[test]
    fn test_error_handling_empty_vector() {
        let fuse = create_test_fuse();
        let empty_vector: Vec<f32> = vec![];
        let metadata = HashMap::new();
        
        let result = fuse.store_vector_enhanced(&empty_vector, 1000, metadata);
        assert!(matches!(result, Err(FuseVexfsError::InvalidVector(_))));
    }
    
    #[test]
    fn test_error_handling_invalid_vector() {
        let fuse = create_test_fuse();
        let invalid_vector = vec![1.0, 2.0, f32::NAN, 4.0];
        let metadata = HashMap::new();
        
        let result = fuse.store_vector_enhanced(&invalid_vector, 1000, metadata);
        assert!(matches!(result, Err(FuseVexfsError::InvalidVector(_))));
    }
    
    #[test]
    fn test_error_handling_nonexistent_vector() {
        let fuse = create_test_fuse();
        let result = fuse.get_vector_enhanced(999999);
        assert!(matches!(result, Err(FuseVexfsError::VectorNotFound(_))));
    }
    
    #[test]
    fn test_search_with_invalid_parameters() {
        let fuse = create_test_fuse();
        
        // Test with empty query vector
        let empty_query: Vec<f32> = vec![];
        let result = fuse.search_vectors_enhanced(&empty_query, 5, None);
        assert!(matches!(result, Err(FuseVexfsError::InvalidVector(_))));
        
        // Test with zero top_k
        let valid_query = generate_test_vector(128, 1.0);
        let result = fuse.search_vectors_enhanced(&valid_query, 0, None);
        assert!(matches!(result, Err(FuseVexfsError::InvalidVector(_))));
        
        // Test with excessive top_k
        let result = fuse.search_vectors_enhanced(&valid_query, 10000, None);
        assert!(matches!(result, Err(FuseVexfsError::InvalidVector(_))));
    }
    
    #[test]
    fn test_unique_vector_ids() {
        let fuse = create_test_fuse();
        let test_vector = generate_test_vector(128, 1.0);
        let metadata = HashMap::new();
        
        let mut vector_ids = Vec::new();
        for i in 0..5 {
            let file_inode = 1000 + i;
            let vector_id = fuse.store_vector_enhanced(&test_vector, file_inode, metadata.clone())
                .expect("Failed to store vector");
            vector_ids.push(vector_id);
        }
        
        // Verify all vector IDs are unique
        let mut unique_ids = vector_ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(unique_ids.len(), vector_ids.len(), "All vector IDs should be unique");
    }
}