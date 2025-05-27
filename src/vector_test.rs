// Userspace test for VexFS vector operations
//
// This module provides standalone testing of vector search functionality
// without requiring kernel module compilation.

use std::collections::HashMap;
use std::time::Instant;

/// Userspace vector for testing
#[derive(Debug, Clone)]
pub struct TestVector {
    pub id: u64,
    pub data: Vec<f32>,
    pub metadata: TestMetadata,
}

/// Test metadata
#[derive(Debug, Clone)]
pub struct TestMetadata {
    pub file_path: String,
    pub file_size: u64,
    pub timestamp: u64,
    pub checksum: u32,
}

/// Simple distance metrics for testing
#[derive(Debug, Clone, Copy)]
pub enum TestDistanceMetric {
    Euclidean,
    Cosine,
    InnerProduct,
}

/// Test search result
#[derive(Debug, Clone)]
pub struct TestSearchResult {
    pub vector_id: u64,
    pub distance: f32,
    pub score: f32,
    pub metadata: TestMetadata,
}

/// Userspace vector search engine for testing
pub struct TestVectorSearchEngine {
    vectors: HashMap<u64, TestVector>,
    next_id: u64,
}

impl TestVectorSearchEngine {
    /// Create new test search engine
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
            next_id: 1,
        }
    }
    
    /// Add vector to the test engine
    pub fn add_vector(&mut self, data: Vec<f32>, metadata: TestMetadata) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let vector = TestVector {
            id,
            data,
            metadata,
        };
        
        self.vectors.insert(id, vector);
        id
    }
    
    /// Search for similar vectors
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        metric: TestDistanceMetric,
    ) -> Vec<TestSearchResult> {
        let mut results = Vec::new();
        
        for vector in self.vectors.values() {
            if vector.data.len() != query.len() {
                continue; // Skip vectors with different dimensions
            }
            
            let distance = self.calculate_distance(query, &vector.data, metric);
            let score = self.distance_to_score(distance, metric);
            
            results.push(TestSearchResult {
                vector_id: vector.id,
                distance,
                score,
                metadata: vector.metadata.clone(),
            });
        }
        
        // Sort by distance (ascending for most metrics)
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        
        // Return top k results
        results.truncate(k);
        results
    }
    
    /// Calculate distance between vectors
    fn calculate_distance(&self, a: &[f32], b: &[f32], metric: TestDistanceMetric) -> f32 {
        match metric {
            TestDistanceMetric::Euclidean => {
                let sum_sq: f32 = a.iter().zip(b.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum();
                sum_sq.sqrt()
            }
            TestDistanceMetric::Cosine => {
                let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                let norm_a: f32 = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
                let norm_b: f32 = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
                
                if norm_a == 0.0 || norm_b == 0.0 {
                    1.0 // Maximum distance for zero vectors
                } else {
                    1.0 - (dot_product / (norm_a * norm_b))
                }
            }
            TestDistanceMetric::InnerProduct => {
                -a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>() // Negative for ascending sort
            }
        }
    }
    
    /// Convert distance to score (0-1 range)
    fn distance_to_score(&self, distance: f32, metric: TestDistanceMetric) -> f32 {
        match metric {
            TestDistanceMetric::Euclidean => {
                // Simple exponential decay
                (-distance).exp()
            }
            TestDistanceMetric::Cosine => {
                // Cosine distance is already 0-2, convert to similarity
                1.0 - distance.clamp(0.0, 1.0)
            }
            TestDistanceMetric::InnerProduct => {
                // Inner product can be negative, normalize to 0-1
                (distance + 1.0) / 2.0
            }
        }
    }
    
    /// Get vector count
    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }
    
    /// Get vector by ID
    pub fn get_vector(&self, id: u64) -> Option<&TestVector> {
        self.vectors.get(&id)
    }
}

/// Performance test for vector operations
pub fn run_performance_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running VexFS Vector Performance Test...");
    
    let mut engine = TestVectorSearchEngine::new();
    
    // Generate test vectors
    println!("Generating test vectors...");
    let vector_count = 1000;
    let dimensions = 128;
    
    let start = Instant::now();
    for i in 0..vector_count {
        let mut data = Vec::with_capacity(dimensions);
        for j in 0..dimensions {
            data.push((i * dimensions + j) as f32 / 1000.0);
        }
        
        let metadata = TestMetadata {
            file_path: format!("/test/vector_{}.bin", i),
            file_size: (dimensions * 4) as u64,
            timestamp: 1640995200 + i as u64,
            checksum: i as u32,
        };
        
        engine.add_vector(data, metadata);
    }
    let insert_time = start.elapsed();
    
    println!("Inserted {} vectors in {:?}", vector_count, insert_time);
    
    // Test search performance
    let query: Vec<f32> = (0..dimensions).map(|i| i as f32 / 100.0).collect();
    let k = 10;
    
    println!("Running search tests...");
    
    // Test different metrics
    for metric in [TestDistanceMetric::Euclidean, TestDistanceMetric::Cosine, TestDistanceMetric::InnerProduct] {
        let start = Instant::now();
        let results = engine.search(&query, k, metric);
        let search_time = start.elapsed();
        
        println!("Metric {:?}: Found {} results in {:?}", metric, results.len(), search_time);
        
        // Show top 3 results
        for (i, result) in results.iter().take(3).enumerate() {
            println!("  {}. Vector {} - Distance: {:.4}, Score: {:.4}, File: {}", 
                i + 1, result.vector_id, result.distance, result.score, result.metadata.file_path);
        }
    }
    
    println!("Performance test completed successfully!");
    Ok(())
}

/// Functional test for vector operations
pub fn run_functional_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running VexFS Vector Functional Test...");
    
    let mut engine = TestVectorSearchEngine::new();
    
    // Add some test vectors with known relationships
    let vector1 = vec![1.0, 0.0, 0.0];
    let vector2 = vec![0.0, 1.0, 0.0];
    let vector3 = vec![0.0, 0.0, 1.0];
    let vector4 = vec![1.0, 1.0, 0.0]; // Similar to vector1 and vector2
    
    let metadata1 = TestMetadata {
        file_path: "/test/vec1.bin".to_string(),
        file_size: 12,
        timestamp: 1640995200,
        checksum: 0x12345678,
    };
    
    let metadata2 = TestMetadata {
        file_path: "/test/vec2.bin".to_string(),
        file_size: 12,
        timestamp: 1640995201,
        checksum: 0x87654321,
    };
    
    let metadata3 = TestMetadata {
        file_path: "/test/vec3.bin".to_string(),
        file_size: 12,
        timestamp: 1640995202,
        checksum: 0xABCDEF01,
    };
    
    let metadata4 = TestMetadata {
        file_path: "/test/vec4.bin".to_string(),
        file_size: 12,
        timestamp: 1640995203,
        checksum: 0x01FEDCBA,
    };
    
    let id1 = engine.add_vector(vector1, metadata1);
    let _id2 = engine.add_vector(vector2, metadata2);
    let _id3 = engine.add_vector(vector3, metadata3);
    let _id4 = engine.add_vector(vector4, metadata4);
    
    println!("Added {} vectors to test engine", engine.vector_count());
    
    // Test search with vector1 as query
    let query = vec![1.0, 0.0, 0.0];
    let results = engine.search(&query, 2, TestDistanceMetric::Euclidean);
    
    println!("Search results for query [1.0, 0.0, 0.0]:");
    for result in &results {
        println!("  Vector {}: Distance {:.4}, Score {:.4}, File: {}", 
            result.vector_id, result.distance, result.score, result.metadata.file_path);
    }
    
    // Verify results
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].vector_id, id1); // Should be exact match
    assert!(results[0].distance < 0.01); // Should be very close to 0
    
    // Test cosine similarity
    let results_cosine = engine.search(&query, 3, TestDistanceMetric::Cosine);
    println!("Cosine similarity results:");
    for result in &results_cosine {
        println!("  Vector {}: Distance {:.4}, Score {:.4}", 
            result.vector_id, result.distance, result.score);
    }
    
    println!("Functional test completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vector_engine_creation() {
        let engine = TestVectorSearchEngine::new();
        assert_eq!(engine.vector_count(), 0);
    }
    
    #[test]
    fn test_vector_addition() {
        let mut engine = TestVectorSearchEngine::new();
        let data = vec![1.0, 2.0, 3.0];
        let metadata = TestMetadata {
            file_path: "/test.bin".to_string(),
            file_size: 12,
            timestamp: 1640995200,
            checksum: 0x12345678,
        };
        
        let id = engine.add_vector(data, metadata);
        assert_eq!(id, 1);
        assert_eq!(engine.vector_count(), 1);
        
        let vector = engine.get_vector(id).unwrap();
        assert_eq!(vector.data, vec![1.0, 2.0, 3.0]);
    }
    
    #[test]
    fn test_euclidean_distance() {
        let engine = TestVectorSearchEngine::new();
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        
        let distance = engine.calculate_distance(&a, &b, TestDistanceMetric::Euclidean);
        assert!((distance - 1.414).abs() < 0.01); // sqrt(2)
    }
    
    #[test]
    fn test_cosine_distance() {
        let engine = TestVectorSearchEngine::new();
        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        
        let distance = engine.calculate_distance(&a, &b, TestDistanceMetric::Cosine);
        assert!(distance < 0.01); // Should be 0 for identical vectors
    }
}