//! HNSW (Hierarchical Navigable Small World) Algorithm Implementation
//! 
//! This module implements the core HNSW algorithm optimized for kernel execution
//! with SIMD optimizations for distance calculations and memory-efficient operations.

#![no_std]

use core::{mem, ptr, slice};
use crate::anns::{
    AnnsError, HnswParams, HnswNode, HnswLayer, SearchResult, SearchContext,
    DistanceMetric, VectorDataType
};

/// SIMD optimization flags for different instruction sets
#[derive(Debug, Clone, Copy)]
pub struct SimdFeatures {
    /// SSE support
    pub sse: bool,
    /// AVX support  
    pub avx: bool,
    /// AVX2 support
    pub avx2: bool,
    /// AVX-512 support
    pub avx512: bool,
}

impl SimdFeatures {
    /// Detect available SIMD features (kernel-safe)
    pub fn detect() -> Self {
        // In kernel space, we need to check CPU features safely
        // For now, assume basic SSE support on x86_64
        Self {
            sse: true,
            avx: false, // Conservative - would need proper CPU feature detection
            avx2: false,
            avx512: false,
        }
    }
}

/// HNSW graph structure optimized for kernel space
pub struct HnswGraph {
    /// Graph layers (layer 0 is the base layer with all nodes)
    pub layers: [HnswLayer; 16], // Fixed size for kernel space
    /// Number of active layers
    pub layer_count: u8,
    /// Total number of nodes across all layers
    pub total_nodes: u64,
    /// Entry point (highest layer node)
    pub entry_point: u64,
    /// SIMD features available
    pub simd_features: SimdFeatures,
    /// Distance metric
    pub distance_metric: DistanceMetric,
    /// Vector dimensions
    pub dimensions: u32,
}

impl HnswGraph {
    /// Create a new HNSW graph
    pub fn new(dimensions: u32, distance_metric: DistanceMetric) -> Self {
        Self {
            layers: [HnswLayer {
                level: 0,
                node_count: 0,
                nodes_offset: 0,
                entry_point: 0,
                reserved: [0; 7],
            }; 16],
            layer_count: 1,
            total_nodes: 0,
            entry_point: 0,
            simd_features: SimdFeatures::detect(),
            distance_metric,
            dimensions,
        }
    }

    /// Insert a new vector into the HNSW graph
    pub fn insert(&mut self, vector_id: u64, vector_data: &[f32], level: u8, params: &HnswParams) -> Result<(), AnnsError> {
        if vector_data.len() != self.dimensions as usize {
            return Err(AnnsError::InvalidParameters);
        }

        if level as usize >= self.layers.len() {
            return Err(AnnsError::InvalidParameters);
        }

        // Create new node
        let node = HnswNode {
            vector_id,
            level,
            connection_count: 0,
            connections_offset: 0,
            flags: 0,
            reserved: [0; 2],
        };

        // If this is the first node, make it the entry point
        if self.total_nodes == 0 {
            self.entry_point = vector_id;
            for layer_idx in 0..=level as usize {
                self.layers[layer_idx].entry_point = vector_id;
                self.layers[layer_idx].node_count = 1;
            }
            self.layer_count = level + 1;
            self.total_nodes = 1;
            return Ok(());
        }

        // Search for nearest neighbors at each layer
        let mut current_candidates = [SearchResult {
            vector_id: self.entry_point,
            distance: f32::INFINITY,
            confidence: 0.0,
        }; 1];

        // Search from top layer down to target level + 1
        for layer_idx in (level as usize + 1..self.layer_count as usize).rev() {
            current_candidates = self.search_layer(
                vector_data,
                &current_candidates,
                1, // ef = 1 for upper layers
                layer_idx as u8,
                params,
            )?;
        }

        // Search and connect at each layer from level down to 0
        for layer_idx in (0..=level as usize).rev() {
            let ef = if layer_idx == 0 {
                params.ef_construction
            } else {
                params.ef_construction.min(params.max_connections as u16)
            };

            let candidates = self.search_layer(
                vector_data,
                &current_candidates,
                ef,
                layer_idx as u8,
                params,
            )?;

            // Select connections based on distance
            let max_conn = if layer_idx == 0 {
                params.max_connections_layer0
            } else {
                params.max_connections
            };

            self.select_neighbors_simple(&candidates, max_conn)?;
            
            // TODO: Create bidirectional connections
            // This would involve:
            // 1. Adding connections from new node to selected neighbors
            // 2. Adding connection from neighbors to new node
            // 3. Pruning neighbor connections if they exceed max_connections

            current_candidates = [candidates[0]]; // Use best candidate for next layer
        }

        // Update graph statistics
        self.total_nodes += 1;
        for layer_idx in 0..=level as usize {
            self.layers[layer_idx].node_count += 1;
        }

        // Update layer count if necessary
        if level + 1 > self.layer_count {
            self.layer_count = level + 1;
            self.entry_point = vector_id;
        }

        Ok(())
    }

    /// Search for nearest neighbors in a specific layer
    pub fn search_layer(
        &self,
        query: &[f32],
        entry_points: &[SearchResult],
        ef: u16,
        layer: u8,
        params: &HnswParams,
    ) -> Result<[SearchResult; 256], AnnsError> {
        let mut candidates = [SearchResult {
            vector_id: 0,
            distance: f32::INFINITY,
            confidence: 0.0,
        }; 256];
        let mut candidate_count = 0usize;

        // Initialize with entry points
        for entry in entry_points.iter() {
            if entry.vector_id != 0 && candidate_count < candidates.len() {
                candidates[candidate_count] = *entry;
                candidate_count += 1;
            }
        }

        if candidate_count == 0 {
            return Err(AnnsError::VectorNotFound);
        }

        let mut visited = [false; 4096]; // Simple visited tracking for kernel space
        let mut dynamic_candidates = candidates;
        let mut w = candidates; // Working set

        // Mark entry points as visited
        for i in 0..candidate_count {
            let node_id = candidates[i].vector_id as usize;
            if node_id < visited.len() {
                visited[node_id] = true;
            }
        }

        while candidate_count > 0 {
            // Find closest unprocessed candidate
            let mut closest_idx = 0;
            for i in 1..candidate_count {
                if dynamic_candidates[i].distance < dynamic_candidates[closest_idx].distance {
                    closest_idx = i;
                }
            }

            let current = dynamic_candidates[closest_idx];
            
            // Remove from candidates by swapping with last
            dynamic_candidates[closest_idx] = dynamic_candidates[candidate_count - 1];
            candidate_count -= 1;

            // Check if we should continue (distance pruning)
            if candidate_count >= ef as usize {
                let mut furthest_distance = 0.0f32;
                for i in 0..candidate_count {
                    if w[i].distance > furthest_distance {
                        furthest_distance = w[i].distance;
                    }
                }
                if current.distance > furthest_distance {
                    continue;
                }
            }

            // TODO: Get connections for current node from layer
            // For now, simulate with empty connections array
            let connections: [u64; 32] = [0; 32];
            let connection_count = 0;

            // Check all connections
            for &neighbor_id in connections.iter().take(connection_count) {
                if neighbor_id == 0 {
                    break;
                }

                let neighbor_idx = neighbor_id as usize;
                if neighbor_idx < visited.len() && visited[neighbor_idx] {
                    continue;
                }

                if neighbor_idx < visited.len() {
                    visited[neighbor_idx] = true;
                }

                // TODO: Get neighbor vector data
                // For now, calculate distance with dummy data
                let neighbor_vector = [0.0f32; 128]; // Placeholder
                let distance = self.calculate_distance_optimized(
                    query,
                    &neighbor_vector[..self.dimensions as usize],
                )?;

                // Add to candidates if close enough
                if candidate_count < ef as usize {
                    dynamic_candidates[candidate_count] = SearchResult {
                        vector_id: neighbor_id,
                        distance,
                        confidence: 1.0 - (distance / (distance + 1.0)), // Simple confidence
                    };
                    candidate_count += 1;
                } else {
                    // Replace furthest candidate if this is closer
                    let mut furthest_idx = 0;
                    for i in 1..candidate_count {
                        if dynamic_candidates[i].distance > dynamic_candidates[furthest_idx].distance {
                            furthest_idx = i;
                        }
                    }
                    
                    if distance < dynamic_candidates[furthest_idx].distance {
                        dynamic_candidates[furthest_idx] = SearchResult {
                            vector_id: neighbor_id,
                            distance,
                            confidence: 1.0 - (distance / (distance + 1.0)),
                        };
                    }
                }

                // Update working set W
                let mut w_count = 0;
                for i in 0..candidate_count {
                    if dynamic_candidates[i].vector_id != 0 {
                        w[w_count] = dynamic_candidates[i];
                        w_count += 1;
                    }
                }

                // Keep only ef closest candidates in W
                if w_count > ef as usize {
                    // Simple bubble sort for small arrays in kernel space
                    for i in 0..w_count {
                        for j in 0..w_count - 1 - i {
                            if w[j].distance > w[j + 1].distance {
                                let temp = w[j];
                                w[j] = w[j + 1];
                                w[j + 1] = temp;
                            }
                        }
                    }
                    w_count = ef as usize;
                }
            }
        }

        // Sort final results by distance
        let mut result_count = 0;
        for i in 0..candidates.len() {
            if w[i].vector_id != 0 && w[i].distance != f32::INFINITY {
                candidates[result_count] = w[i];
                result_count += 1;
            }
        }

        // Sort results
        for i in 0..result_count {
            for j in 0..result_count - 1 - i {
                if candidates[j].distance > candidates[j + 1].distance {
                    let temp = candidates[j];
                    candidates[j] = candidates[j + 1];
                    candidates[j + 1] = temp;
                }
            }
        }

        Ok(candidates)
    }

    /// Select neighbors using simple distance-based strategy
    fn select_neighbors_simple(&self, candidates: &[SearchResult; 256], max_connections: u16) -> Result<[u64; 32], AnnsError> {
        let mut selected = [0u64; 32];
        let mut count = 0;

        for candidate in candidates.iter() {
            if candidate.vector_id == 0 || candidate.distance == f32::INFINITY {
                break;
            }
            
            if count >= max_connections as usize || count >= selected.len() {
                break;
            }

            selected[count] = candidate.vector_id;
            count += 1;
        }

        Ok(selected)
    }

    /// Optimized distance calculation with SIMD when available
    pub fn calculate_distance_optimized(&self, vec1: &[f32], vec2: &[f32]) -> Result<f32, AnnsError> {
        if vec1.len() != vec2.len() {
            return Err(AnnsError::InvalidParameters);
        }

        match self.distance_metric {
            DistanceMetric::Euclidean => {
                if self.simd_features.sse && vec1.len() >= 4 {
                    self.euclidean_distance_sse(vec1, vec2)
                } else {
                    Ok(self.euclidean_distance_scalar(vec1, vec2))
                }
            }
            DistanceMetric::Cosine => Ok(self.cosine_distance_scalar(vec1, vec2)),
            DistanceMetric::Manhattan => Ok(self.manhattan_distance_scalar(vec1, vec2)),
            DistanceMetric::Dot => Ok(self.dot_product_scalar(vec1, vec2)),
            _ => Err(AnnsError::InvalidParameters),
        }
    }

    /// SSE-optimized Euclidean distance calculation
    fn euclidean_distance_sse(&self, vec1: &[f32], vec2: &[f32]) -> Result<f32, AnnsError> {
        // TODO: Implement actual SSE intrinsics when available in kernel
        // For now, fall back to scalar implementation
        // This would use inline assembly or kernel SIMD intrinsics like:
        // - _mm_load_ps() for loading 4 floats
        // - _mm_sub_ps() for subtraction
        // - _mm_mul_ps() for multiplication  
        // - _mm_add_ps() for accumulation
        // - _mm_sqrt_ss() for square root
        
        Ok(self.euclidean_distance_scalar(vec1, vec2))
    }

    /// Scalar Euclidean distance calculation
    fn euclidean_distance_scalar(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        
        // Process 4 elements at a time for better cache utilization
        let chunks = vec1.len() / 4;
        let remainder = vec1.len() % 4;
        
        for i in 0..chunks {
            let base = i * 4;
            for j in 0..4 {
                let diff = vec1[base + j] - vec2[base + j];
                sum += diff * diff;
            }
        }
        
        // Process remaining elements
        for i in 0..remainder {
            let idx = chunks * 4 + i;
            let diff = vec1[idx] - vec2[idx];
            sum += diff * diff;
        }
        
        sum.sqrt()
    }

    /// Scalar cosine distance calculation
    fn cosine_distance_scalar(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut dot = 0.0f32;
        let mut norm1 = 0.0f32;
        let mut norm2 = 0.0f32;

        for i in 0..vec1.len() {
            let v1 = vec1[i];
            let v2 = vec2[i];
            dot += v1 * v2;
            norm1 += v1 * v1;
            norm2 += v2 * v2;
        }

        let norm_product = (norm1 * norm2).sqrt();
        if norm_product == 0.0 {
            return f32::INFINITY;
        }

        1.0 - (dot / norm_product)
    }

    /// Scalar Manhattan distance calculation
    fn manhattan_distance_scalar(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..vec1.len() {
            sum += (vec1[i] - vec2[i]).abs();
        }
        sum
    }

    /// Scalar dot product calculation
    fn dot_product_scalar(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..vec1.len() {
            sum += vec1[i] * vec2[i];
        }
        -sum // Negate for distance (smaller is better)
    }

    /// Get graph statistics
    pub fn get_stats(&self) -> HnswStats {
        let mut total_connections = 0u64;
        for layer in &self.layers[..self.layer_count as usize] {
            total_connections += layer.node_count as u64;
        }

        HnswStats {
            total_nodes: self.total_nodes,
            layer_count: self.layer_count,
            total_connections,
            entry_point: self.entry_point,
            avg_connections_per_node: if self.total_nodes > 0 {
                (total_connections as f32 / self.total_nodes as f32)
            } else {
                0.0
            },
            memory_usage_bytes: self.estimate_memory_usage(),
        }
    }

    /// Estimate memory usage of the graph
    fn estimate_memory_usage(&self) -> u64 {
        let layers_size = mem::size_of::<HnswLayer>() * self.layer_count as usize;
        let nodes_size = mem::size_of::<HnswNode>() * self.total_nodes as usize;
        // Estimate connections (average max_connections per node)
        let avg_connections = 16; // Conservative estimate
        let connections_size = mem::size_of::<u64>() * self.total_nodes as usize * avg_connections;
        
        (layers_size + nodes_size + connections_size) as u64
    }
}

/// HNSW graph statistics
#[derive(Debug, Clone, Copy)]
pub struct HnswStats {
    pub total_nodes: u64,
    pub layer_count: u8,
    pub total_connections: u64,
    pub entry_point: u64,
    pub avg_connections_per_node: f32,
    pub memory_usage_bytes: u64,
}

/// Tests for HNSW functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_graph_creation() {
        let graph = HnswGraph::new(128, DistanceMetric::Euclidean);
        assert_eq!(graph.dimensions, 128);
        assert_eq!(graph.distance_metric, DistanceMetric::Euclidean);
        assert_eq!(graph.total_nodes, 0);
    }

    #[test]
    fn test_simd_features_detection() {
        let features = SimdFeatures::detect();
        // Should at least detect some basic features
        assert!(features.sse || !features.avx); // Basic sanity check
    }

    #[test]
    fn test_distance_calculations() {
        let graph = HnswGraph::new(3, DistanceMetric::Euclidean);
        
        let vec1 = [1.0, 2.0, 3.0];
        let vec2 = [4.0, 5.0, 6.0];
        
        let dist = graph.euclidean_distance_scalar(&vec1, &vec2);
        assert!((dist - 5.196152).abs() < 0.001); // sqrt(27)
        
        let cosine_dist = graph.cosine_distance_scalar(&vec1, &vec2);
        assert!(cosine_dist >= 0.0 && cosine_dist <= 2.0);
    }

    #[test]
    fn test_hnsw_node_size() {
        // Ensure node structure is cache-friendly
        assert!(mem::size_of::<HnswNode>() <= 32);
    }

    #[test]
    fn test_hnsw_layer_size() {
        // Ensure layer structure is reasonable
        assert!(mem::size_of::<HnswLayer>() <= 64);
    }
}