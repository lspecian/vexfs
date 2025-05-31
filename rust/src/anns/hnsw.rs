//! Hierarchical Navigable Small World (HNSW) graph implementation
//! 
//! This module provides the core HNSW data structures and algorithms for
//! efficient approximate nearest neighbor search.

use std::vec::Vec;
use crate::anns::{AnnsError, HnswParams};

/// A node in the HNSW graph representing a vector
#[derive(Debug, Clone)]
pub struct HnswNode {
    pub vector_id: u64,
    pub layer: u8,
    pub connections: Vec<u64>,
}

impl HnswNode {
    /// Create a new HNSW node
    pub fn new(vector_id: u64, layer: u8) -> Self {
        Self {
            vector_id,
            layer,
            connections: Vec::new(),
        }
    }

    /// Add a connection to another node
    pub fn add_connection(&mut self, target_id: u64) {
        if !self.connections.contains(&target_id) {
            self.connections.push(target_id);
        }
    }

    /// Remove a connection to another node
    pub fn remove_connection(&mut self, target_id: u64) {
        self.connections.retain(|&id| id != target_id);
    }

    /// Get the number of connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

/// The main HNSW graph structure
#[derive(Debug)]
pub struct HnswGraph {
    dimensions: u32,
    params: HnswParams,
    nodes: Vec<HnswNode>,
    entry_point: Option<u64>,
    max_layer: u8,
}

impl HnswGraph {
    /// Create a new HNSW graph
    pub fn new(dimensions: u32, params: HnswParams) -> Result<Self, AnnsError> {
        if dimensions == 0 {
            return Err(AnnsError::InvalidDimensions);
        }

        Ok(Self {
            dimensions,
            params,
            nodes: Vec::new(),
            entry_point: None,
            max_layer: 0,
        })
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: HnswNode) -> Result<(), AnnsError> {
        // Update max layer if necessary
        if node.layer > self.max_layer {
            self.max_layer = node.layer;
        }

        // Set entry point if this is the first node or it's on a higher layer
        if self.entry_point.is_none() || node.layer >= self.max_layer {
            self.entry_point = Some(node.vector_id);
        }

        self.nodes.push(node);
        Ok(())
    }

    /// Get a node by vector ID
    pub fn get_node(&self, vector_id: u64) -> Option<&HnswNode> {
        self.nodes.iter().find(|node| node.vector_id == vector_id)
    }

    /// Get a mutable reference to a node by vector ID
    pub fn get_node_mut(&mut self, vector_id: u64) -> Option<&mut HnswNode> {
        self.nodes.iter_mut().find(|node| node.vector_id == vector_id)
    }

    /// Remove a node from the graph
    pub fn remove_node(&mut self, vector_id: u64) -> Result<(), AnnsError> {
        // Remove the node itself
        self.nodes.retain(|node| node.vector_id != vector_id);
        
        // Remove all connections to this node from other nodes
        for node in &mut self.nodes {
            node.remove_connection(vector_id);
        }

        // Update entry point if necessary
        if self.entry_point == Some(vector_id) {
            self.entry_point = self.nodes.iter()
                .max_by_key(|node| node.layer)
                .map(|node| node.vector_id);
        }

        Ok(())
    }

    /// Get the entry point of the graph
    pub fn entry_point(&self) -> Option<u64> {
        self.entry_point
    }

    /// Get the maximum layer in the graph
    pub fn max_layer(&self) -> u8 {
        self.max_layer
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the dimensions
    pub fn dimensions(&self) -> u32 {
        self.dimensions
    }

    /// Get the parameters
    pub fn params(&self) -> &HnswParams {
        &self.params
    }

    /// Estimate memory usage of the graph
    fn estimate_memory_usage(&self) -> u64 {
        let base_size = core::mem::size_of::<Self>() as u64;
        let nodes_size = self.nodes.len() as u64 * core::mem::size_of::<HnswNode>() as u64;
        let connections_size: u64 = self.nodes.iter()
            .map(|node| node.connections.len() as u64 * core::mem::size_of::<u64>() as u64)
            .sum();
        
        base_size + nodes_size + connections_size
    }

    /// Get memory usage estimate (public version)
    pub fn memory_usage(&self) -> u64 {
        self.estimate_memory_usage()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Check if a vector with the given ID exists in the graph
    pub fn contains_vector(&self, vector_id: u64) -> bool {
        self.nodes.iter().any(|node| node.vector_id == vector_id)
    }

    /// Clear all nodes from the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.entry_point = None;
        self.max_layer = 0;
    }

    /// Get nodes at a specific layer
    pub fn nodes_at_layer(&self, layer: u8) -> Vec<&HnswNode> {
        self.nodes.iter()
            .filter(|node| node.layer >= layer)
            .collect()
    }

    /// Generate a random layer for a new node
    pub fn generate_layer(&self) -> u8 {
        // Simple layer generation - in a full implementation this would use
        // proper random number generation with the ml parameter
        let mut layer = 0u8;
        while layer < self.params.max_layers - 1 && layer < 4 {
            // Simple deterministic layer assignment for now
            if (layer as u16) < self.params.m / 4 {
                layer += 1;
            } else {
                break;
            }
        }
        layer
    }

    /// Search for the k nearest neighbors using HNSW algorithm
    /// This is the core HNSW search traversal algorithm
    pub fn search<F>(&self, query: &[f32], k: usize, ef: u16, distance_fn: F) -> Result<Vec<(u64, f32)>, AnnsError>
    where
        F: Fn(&[f32], &[f32]) -> Result<f32, AnnsError>,
    {
        if self.is_empty() {
            return Ok(Vec::new());
        }

        let entry_point = match self.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };

        // Phase 1: Search from top layer down to layer 1
        let mut current_closest = entry_point;
        let mut current_distance = f32::INFINITY;

        // We need a way to get vector data - this will be provided by the caller
        // For now, we'll implement the graph traversal logic assuming we can get distances

        // Start from the top layer and work down to layer 1
        for layer in (1..=self.max_layer).rev() {
            current_closest = self.search_layer(query, current_closest, 1, layer, &distance_fn)?
                .into_iter()
                .next()
                .map(|(id, _)| id)
                .unwrap_or(current_closest);
        }

        // Phase 2: Search layer 0 with ef parameter
        let candidates = self.search_layer(query, current_closest, ef as usize, 0, &distance_fn)?;

        // Return top k results
        let mut results: Vec<(u64, f32)> = candidates.into_iter().take(k).collect();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }

    /// Search a specific layer of the HNSW graph
    fn search_layer<F>(
        &self,
        query: &[f32],
        entry_point: u64,
        num_closest: usize,
        layer: u8,
        distance_fn: &F,
    ) -> Result<Vec<(u64, f32)>, AnnsError>
    where
        F: Fn(&[f32], &[f32]) -> Result<f32, AnnsError>,
    {
        use std::collections::{BinaryHeap, HashSet};
        use std::cmp::Ordering;

        // Custom struct for the priority queue (max-heap by default)
        #[derive(Debug)]
        struct Candidate {
            vector_id: u64,
            distance: f32,
        }

        impl PartialEq for Candidate {
            fn eq(&self, other: &Self) -> bool {
                self.distance == other.distance
            }
        }

        impl Eq for Candidate {}

        impl PartialOrd for Candidate {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                // Reverse ordering for min-heap behavior
                other.distance.partial_cmp(&self.distance)
            }
        }

        impl Ord for Candidate {
            fn cmp(&self, other: &Self) -> Ordering {
                self.partial_cmp(other).unwrap_or(Ordering::Equal)
            }
        }

        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new(); // Dynamic candidate list
        let mut w = BinaryHeap::new(); // Result set

        // For this implementation, we'll need to simulate getting vector data
        // In a real implementation, this would call the distance function with actual vector data
        // For now, we'll use a placeholder approach

        // Add entry point to candidates
        visited.insert(entry_point);
        
        // Simulate distance calculation - in real implementation this would get actual vector data
        let entry_distance = 0.5; // Placeholder distance
        
        candidates.push(Candidate {
            vector_id: entry_point,
            distance: entry_distance,
        });
        
        w.push(Candidate {
            vector_id: entry_point,
            distance: entry_distance,
        });

        // Main search loop
        while let Some(current) = candidates.pop() {
            // If current is farther than the farthest in w, stop
            if let Some(farthest) = w.peek() {
                if current.distance > farthest.distance && w.len() >= num_closest {
                    break;
                }
            }

            // Get the current node
            if let Some(node) = self.get_node(current.vector_id) {
                // Only consider nodes at this layer or higher
                if node.layer >= layer {
                    // Examine all connections
                    for &neighbor_id in &node.connections {
                        if !visited.contains(&neighbor_id) {
                            visited.insert(neighbor_id);

                            // Simulate distance calculation for neighbor
                            // In real implementation: distance_fn(query, neighbor_vector_data)?
                            let neighbor_distance = 0.6; // Placeholder distance

                            let neighbor_candidate = Candidate {
                                vector_id: neighbor_id,
                                distance: neighbor_distance,
                            };

                            // Add to candidates if it's promising
                            if let Some(farthest) = w.peek() {
                                if neighbor_distance < farthest.distance || w.len() < num_closest {
                                    candidates.push(neighbor_candidate);
                                    w.push(Candidate {
                                        vector_id: neighbor_id,
                                        distance: neighbor_distance,
                                    });

                                    // Keep w size limited
                                    if w.len() > num_closest {
                                        w.pop();
                                    }
                                }
                            } else {
                                candidates.push(neighbor_candidate);
                                w.push(Candidate {
                                    vector_id: neighbor_id,
                                    distance: neighbor_distance,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Convert result set to vector
        let mut results: Vec<(u64, f32)> = w.into_iter()
            .map(|c| (c.vector_id, c.distance))
            .collect();

        // Sort by distance (ascending)
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        Ok(results)
    }
}

/// Layer information for HNSW
#[derive(Debug, Clone)]
pub struct HnswLayer {
    pub layer_id: u8,
    pub node_count: u32,
}

impl HnswLayer {
    /// Create a new layer
    pub fn new(layer_id: u8) -> Self {
        Self {
            layer_id,
            node_count: 0,
        }
    }

    /// Increment node count
    pub fn add_node(&mut self) {
        self.node_count += 1;
    }

    /// Decrement node count
    pub fn remove_node(&mut self) {
        if self.node_count > 0 {
            self.node_count -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_node_creation() {
        let node = HnswNode::new(1, 0);
        assert_eq!(node.vector_id, 1);
        assert_eq!(node.layer, 0);
        assert_eq!(node.connection_count(), 0);
    }

    #[test]
    fn test_hnsw_graph_creation() {
        let params = HnswParams::default();
        let graph = HnswGraph::new(128, params).unwrap();
        assert_eq!(graph.dimensions(), 128);
        assert_eq!(graph.node_count(), 0);
        assert!(graph.is_empty());
    }

    #[test]
    fn test_add_node() {
        let params = HnswParams::default();
        let mut graph = HnswGraph::new(128, params).unwrap();
        
        let node = HnswNode::new(1, 0);
        graph.add_node(node).unwrap();
        
        assert_eq!(graph.node_count(), 1);
        assert!(!graph.is_empty());
        assert_eq!(graph.entry_point(), Some(1));
    }
}