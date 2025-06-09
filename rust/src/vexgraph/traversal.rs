/*
 * VexFS v2.0 - VexGraph Phase 2 Traversal Engine
 * 
 * This module implements efficient graph traversal algorithms including BFS, DFS,
 * Dijkstra's algorithm, and topological sort for the VexGraph Phase 2 system.
 */

use crate::vexgraph::{
    NodeId, EdgeId, TraversalAlgorithm, VexGraphConfig,
    core::{VexGraphCore, GraphNode, GraphEdge},
    error_handling::{VexGraphError, VexGraphResult, Validator},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque, BinaryHeap};
use std::sync::Arc;
use std::cmp::Ordering;

/// Traversal result containing the path and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub algorithm: TraversalAlgorithm,
    pub start_node: NodeId,
    pub end_node: Option<NodeId>,
    pub path: Vec<NodeId>,
    pub edges: Vec<EdgeId>,
    pub distances: HashMap<NodeId, f64>,
    pub visited_count: usize,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, String>,
}

/// Traversal query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalQuery {
    pub algorithm: TraversalAlgorithm,
    pub start_node: NodeId,
    pub end_node: Option<NodeId>,
    pub max_depth: Option<u32>,
    pub max_results: Option<usize>,
    pub node_filter: Option<crate::vexgraph::NodeType>,
    pub edge_filter: Option<crate::vexgraph::EdgeType>,
    pub weight_threshold: Option<f64>,
    pub timeout_ms: Option<u64>,
}

/// Node with priority for Dijkstra's algorithm
#[derive(Debug, Clone)]
struct PriorityNode {
    node_id: NodeId,
    distance: f64,
}

impl Eq for PriorityNode {}

impl PartialEq for PriorityNode {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Ord for PriorityNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.distance.partial_cmp(&self.distance).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for PriorityNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Traversal statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalStatistics {
    pub total_traversals: u64,
    pub bfs_count: u64,
    pub dfs_count: u64,
    pub dijkstra_count: u64,
    pub topological_sort_count: u64,
    pub average_execution_time_ms: f64,
    pub average_path_length: f64,
    pub cache_hit_rate: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Traversal engine implementation
#[derive(Debug)]
pub struct TraversalEngine {
    /// Reference to the core graph
    core: Arc<VexGraphCore>,
    
    /// Traversal cache for frequently accessed paths
    path_cache: dashmap::DashMap<String, TraversalResult>,
    
    /// Statistics
    stats: parking_lot::RwLock<TraversalStatistics>,
    
    /// Configuration
    config: VexGraphConfig,
}

impl TraversalEngine {
    /// Create a new traversal engine
    pub async fn new(core: Arc<VexGraphCore>) -> VexGraphResult<Self> {
        let stats = TraversalStatistics {
            total_traversals: 0,
            bfs_count: 0,
            dfs_count: 0,
            dijkstra_count: 0,
            topological_sort_count: 0,
            average_execution_time_ms: 0.0,
            average_path_length: 0.0,
            cache_hit_rate: 0.0,
            last_updated: chrono::Utc::now(),
        };
        
        Ok(Self {
            core,
            path_cache: dashmap::DashMap::new(),
            stats: parking_lot::RwLock::new(stats),
            config: VexGraphConfig::default(),
        })
    }
    
    /// Start the traversal engine
    pub async fn start(&self) -> VexGraphResult<()> {
        tracing::info!("Starting VexGraph traversal engine");
        Ok(())
    }
    
    /// Stop the traversal engine
    pub async fn stop(&self) -> VexGraphResult<()> {
        tracing::info!("Stopping VexGraph traversal engine");
        self.path_cache.clear();
        Ok(())
    }
    
    /// Execute a traversal query
    pub async fn execute_traversal(&self, query: TraversalQuery) -> VexGraphResult<TraversalResult> {
        let start_time = std::time::Instant::now();
        
        // Validate input
        Validator::validate_node_id(query.start_node)?;
        if let Some(end_node) = query.end_node {
            Validator::validate_node_id(end_node)?;
        }
        if let Some(max_depth) = query.max_depth {
            Validator::validate_traversal_depth(max_depth)?;
        }
        
        // Check cache first
        let cache_key = self.generate_cache_key(&query);
        if let Some(cached_result) = self.path_cache.get(&cache_key) {
            self.update_cache_hit_stats().await;
            return Ok(cached_result.clone());
        }
        
        // Execute the appropriate algorithm
        let result = match query.algorithm {
            TraversalAlgorithm::BreadthFirstSearch => self.breadth_first_search(&query).await?,
            TraversalAlgorithm::DepthFirstSearch => self.depth_first_search(&query).await?,
            TraversalAlgorithm::Dijkstra => self.dijkstra_shortest_path(&query).await?,
            TraversalAlgorithm::TopologicalSort => self.topological_sort(&query).await?,
            _ => return Err(VexGraphError::InvalidTraversalAlgorithm(
                format!("Algorithm {:?} not implemented", query.algorithm)
            )),
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Create result with timing information
        let mut final_result = result;
        final_result.execution_time_ms = execution_time;
        
        // Cache the result
        self.path_cache.insert(cache_key, final_result.clone());
        
        // Update statistics
        self.update_traversal_stats(&query, execution_time, &final_result).await;
        
        Ok(final_result)
    }
    
    /// Breadth-First Search implementation
    async fn breadth_first_search(&self, query: &TraversalQuery) -> VexGraphResult<TraversalResult> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent = HashMap::new();
        let mut distances = HashMap::new();
        let mut path_edges = Vec::new();
        
        queue.push_back(query.start_node);
        visited.insert(query.start_node);
        distances.insert(query.start_node, 0.0);
        
        let mut current_depth = 0;
        let max_depth = query.max_depth.unwrap_or(crate::vexgraph::MAX_TRAVERSAL_DEPTH);
        
        while let Some(current_node) = queue.pop_front() {
            // Check if we've reached the target
            if let Some(end_node) = query.end_node {
                if current_node == end_node {
                    break;
                }
            }
            
            // Check depth limit
            if current_depth >= max_depth {
                break;
            }
            
            // Get outgoing edges
            let outgoing_edges = self.core.get_outgoing_edges(current_node).await?;
            
            for edge_id in outgoing_edges {
                let edge = self.core.get_edge(edge_id).await?;
                
                // Apply edge filter
                if let Some(edge_filter) = query.edge_filter {
                    if edge.edge_type != edge_filter {
                        continue;
                    }
                }
                
                // Apply weight threshold
                if let Some(threshold) = query.weight_threshold {
                    if edge.weight < threshold {
                        continue;
                    }
                }
                
                let target_node = edge.target_id;
                
                if !visited.contains(&target_node) {
                    // Apply node filter
                    if let Some(node_filter) = query.node_filter {
                        let node = self.core.get_node(target_node).await?;
                        if node.node_type != node_filter {
                            continue;
                        }
                    }
                    
                    visited.insert(target_node);
                    parent.insert(target_node, current_node);
                    distances.insert(target_node, distances[&current_node] + edge.weight);
                    queue.push_back(target_node);
                    path_edges.push(edge_id);
                }
            }
            
            current_depth += 1;
        }
        
        // Reconstruct path if end node was specified and found
        let path = if let Some(end_node) = query.end_node {
            if visited.contains(&end_node) {
                self.reconstruct_path(&parent, query.start_node, end_node)
            } else {
                Vec::new()
            }
        } else {
            visited.into_iter().collect()
        };
        
        Ok(TraversalResult {
            algorithm: TraversalAlgorithm::BreadthFirstSearch,
            start_node: query.start_node,
            end_node: query.end_node,
            path,
            edges: path_edges,
            distances,
            visited_count: visited.len(),
            execution_time_ms: 0, // Will be set by caller
            metadata: HashMap::new(),
        })
    }
    
    /// Depth-First Search implementation
    async fn depth_first_search(&self, query: &TraversalQuery) -> VexGraphResult<TraversalResult> {
        let mut stack = Vec::new();
        let mut visited = HashSet::new();
        let mut parent = HashMap::new();
        let mut distances = HashMap::new();
        let mut path_edges = Vec::new();
        
        stack.push(query.start_node);
        distances.insert(query.start_node, 0.0);
        
        let max_depth = query.max_depth.unwrap_or(crate::vexgraph::MAX_TRAVERSAL_DEPTH);
        let mut current_depth = 0;
        
        while let Some(current_node) = stack.pop() {
            if visited.contains(&current_node) {
                continue;
            }
            
            visited.insert(current_node);
            
            // Check if we've reached the target
            if let Some(end_node) = query.end_node {
                if current_node == end_node {
                    break;
                }
            }
            
            // Check depth limit
            if current_depth >= max_depth {
                continue;
            }
            
            // Get outgoing edges
            let outgoing_edges = self.core.get_outgoing_edges(current_node).await?;
            
            for edge_id in outgoing_edges {
                let edge = self.core.get_edge(edge_id).await?;
                
                // Apply edge filter
                if let Some(edge_filter) = query.edge_filter {
                    if edge.edge_type != edge_filter {
                        continue;
                    }
                }
                
                // Apply weight threshold
                if let Some(threshold) = query.weight_threshold {
                    if edge.weight < threshold {
                        continue;
                    }
                }
                
                let target_node = edge.target_id;
                
                if !visited.contains(&target_node) {
                    // Apply node filter
                    if let Some(node_filter) = query.node_filter {
                        let node = self.core.get_node(target_node).await?;
                        if node.node_type != node_filter {
                            continue;
                        }
                    }
                    
                    parent.insert(target_node, current_node);
                    distances.insert(target_node, distances[&current_node] + edge.weight);
                    stack.push(target_node);
                    path_edges.push(edge_id);
                }
            }
            
            current_depth += 1;
        }
        
        // Reconstruct path if end node was specified and found
        let path = if let Some(end_node) = query.end_node {
            if visited.contains(&end_node) {
                self.reconstruct_path(&parent, query.start_node, end_node)
            } else {
                Vec::new()
            }
        } else {
            visited.into_iter().collect()
        };
        
        Ok(TraversalResult {
            algorithm: TraversalAlgorithm::DepthFirstSearch,
            start_node: query.start_node,
            end_node: query.end_node,
            path,
            edges: path_edges,
            distances,
            visited_count: visited.len(),
            execution_time_ms: 0, // Will be set by caller
            metadata: HashMap::new(),
        })
    }
    
    /// Dijkstra's shortest path algorithm implementation
    async fn dijkstra_shortest_path(&self, query: &TraversalQuery) -> VexGraphResult<TraversalResult> {
        let mut heap = BinaryHeap::new();
        let mut distances = HashMap::new();
        let mut parent = HashMap::new();
        let mut visited = HashSet::new();
        let mut path_edges = Vec::new();
        
        // Initialize
        distances.insert(query.start_node, 0.0);
        heap.push(PriorityNode {
            node_id: query.start_node,
            distance: 0.0,
        });
        
        while let Some(PriorityNode { node_id: current_node, distance: current_distance }) = heap.pop() {
            if visited.contains(&current_node) {
                continue;
            }
            
            visited.insert(current_node);
            
            // Check if we've reached the target
            if let Some(end_node) = query.end_node {
                if current_node == end_node {
                    break;
                }
            }
            
            // Get outgoing edges
            let outgoing_edges = self.core.get_outgoing_edges(current_node).await?;
            
            for edge_id in outgoing_edges {
                let edge = self.core.get_edge(edge_id).await?;
                
                // Apply edge filter
                if let Some(edge_filter) = query.edge_filter {
                    if edge.edge_type != edge_filter {
                        continue;
                    }
                }
                
                // Apply weight threshold
                if let Some(threshold) = query.weight_threshold {
                    if edge.weight < threshold {
                        continue;
                    }
                }
                
                let target_node = edge.target_id;
                let new_distance = current_distance + edge.weight;
                
                if !visited.contains(&target_node) {
                    // Apply node filter
                    if let Some(node_filter) = query.node_filter {
                        let node = self.core.get_node(target_node).await?;
                        if node.node_type != node_filter {
                            continue;
                        }
                    }
                    
                    let should_update = distances.get(&target_node)
                        .map_or(true, |&old_distance| new_distance < old_distance);
                    
                    if should_update {
                        distances.insert(target_node, new_distance);
                        parent.insert(target_node, current_node);
                        heap.push(PriorityNode {
                            node_id: target_node,
                            distance: new_distance,
                        });
                        path_edges.push(edge_id);
                    }
                }
            }
        }
        
        // Reconstruct path if end node was specified and found
        let path = if let Some(end_node) = query.end_node {
            if distances.contains_key(&end_node) {
                self.reconstruct_path(&parent, query.start_node, end_node)
            } else {
                Vec::new()
            }
        } else {
            distances.keys().cloned().collect()
        };
        
        Ok(TraversalResult {
            algorithm: TraversalAlgorithm::Dijkstra,
            start_node: query.start_node,
            end_node: query.end_node,
            path,
            edges: path_edges,
            distances,
            visited_count: visited.len(),
            execution_time_ms: 0, // Will be set by caller
            metadata: HashMap::new(),
        })
    }
    
    /// Topological sort implementation
    async fn topological_sort(&self, query: &TraversalQuery) -> VexGraphResult<TraversalResult> {
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        
        // Calculate in-degrees for all nodes starting from the start node
        let mut to_process = VecDeque::new();
        to_process.push_back(query.start_node);
        let mut processed = HashSet::new();
        
        while let Some(node_id) = to_process.pop_front() {
            if processed.contains(&node_id) {
                continue;
            }
            processed.insert(node_id);
            
            in_degree.entry(node_id).or_insert(0);
            
            let outgoing_edges = self.core.get_outgoing_edges(node_id).await?;
            for edge_id in outgoing_edges {
                let edge = self.core.get_edge(edge_id).await?;
                let target_node = edge.target_id;
                
                *in_degree.entry(target_node).or_insert(0) += 1;
                to_process.push_back(target_node);
            }
        }
        
        // Find nodes with no incoming edges
        for (&node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id);
            }
        }
        
        // Process nodes in topological order
        while let Some(current_node) = queue.pop_front() {
            result.push(current_node);
            visited.insert(current_node);
            
            let outgoing_edges = self.core.get_outgoing_edges(current_node).await?;
            for edge_id in outgoing_edges {
                let edge = self.core.get_edge(edge_id).await?;
                let target_node = edge.target_id;
                
                if let Some(degree) = in_degree.get_mut(&target_node) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(target_node);
                    }
                }
            }
        }
        
        // Check for cycles
        if result.len() != in_degree.len() {
            return Err(VexGraphError::TraversalError(
                "Graph contains cycles - topological sort not possible".to_string()
            ));
        }
        
        Ok(TraversalResult {
            algorithm: TraversalAlgorithm::TopologicalSort,
            start_node: query.start_node,
            end_node: query.end_node,
            path: result,
            edges: Vec::new(),
            distances: HashMap::new(),
            visited_count: visited.len(),
            execution_time_ms: 0, // Will be set by caller
            metadata: HashMap::new(),
        })
    }
    
    /// Reconstruct path from parent map
    fn reconstruct_path(
        &self,
        parent: &HashMap<NodeId, NodeId>,
        start: NodeId,
        end: NodeId,
    ) -> Vec<NodeId> {
        let mut path = Vec::new();
        let mut current = end;
        
        while current != start {
            path.push(current);
            if let Some(&p) = parent.get(&current) {
                current = p;
            } else {
                return Vec::new(); // No path found
            }
        }
        
        path.push(start);
        path.reverse();
        path
    }
    
    /// Generate cache key for a traversal query
    fn generate_cache_key(&self, query: &TraversalQuery) -> String {
        format!(
            "{:?}:{}:{:?}:{:?}:{:?}:{:?}:{:?}",
            query.algorithm,
            query.start_node,
            query.end_node,
            query.max_depth,
            query.node_filter,
            query.edge_filter,
            query.weight_threshold
        )
    }
    
    /// Update cache hit statistics
    async fn update_cache_hit_stats(&self) {
        // Implementation for cache hit statistics
    }
    
    /// Update traversal statistics
    async fn update_traversal_stats(
        &self,
        query: &TraversalQuery,
        execution_time: u64,
        result: &TraversalResult,
    ) {
        let mut stats = self.stats.write();
        stats.total_traversals += 1;
        
        match query.algorithm {
            TraversalAlgorithm::BreadthFirstSearch => stats.bfs_count += 1,
            TraversalAlgorithm::DepthFirstSearch => stats.dfs_count += 1,
            TraversalAlgorithm::Dijkstra => stats.dijkstra_count += 1,
            TraversalAlgorithm::TopologicalSort => stats.topological_sort_count += 1,
            _ => {}
        }
        
        // Update averages
        let total = stats.total_traversals as f64;
        stats.average_execution_time_ms = 
            (stats.average_execution_time_ms * (total - 1.0) + execution_time as f64) / total;
        stats.average_path_length = 
            (stats.average_path_length * (total - 1.0) + result.path.len() as f64) / total;
        
        stats.last_updated = chrono::Utc::now();
    }
    
    /// Get traversal statistics
    pub async fn get_statistics(&self) -> VexGraphResult<TraversalStatistics> {
        Ok(self.stats.read().clone())
    }
    
    /// Clear the path cache
    pub async fn clear_cache(&self) -> VexGraphResult<()> {
        self.path_cache.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vexgraph::{NodeType, EdgeType};
    
    #[tokio::test]
    async fn test_traversal_engine_creation() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let engine = TraversalEngine::new(core).await.unwrap();
        
        let stats = engine.get_statistics().await.unwrap();
        assert_eq!(stats.total_traversals, 0);
    }
    
    #[tokio::test]
    async fn test_bfs_traversal() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let engine = TraversalEngine::new(core.clone()).await.unwrap();
        
        // Create test graph
        let node1 = core.create_node(1, NodeType::File).await.unwrap();
        let node2 = core.create_node(2, NodeType::File).await.unwrap();
        let node3 = core.create_node(3, NodeType::File).await.unwrap();
        
        let _edge1 = core.create_edge(node1, node2, EdgeType::References, 1.0).await.unwrap();
        let _edge2 = core.create_edge(node2, node3, EdgeType::References, 1.0).await.unwrap();
        
        // Test BFS
        let query = TraversalQuery {
            algorithm: TraversalAlgorithm::BreadthFirstSearch,
            start_node: node1,
            end_node: Some(node3),
            max_depth: None,
            max_results: None,
            node_filter: None,
            edge_filter: None,
            weight_threshold: None,
            timeout_ms: None,
        };
        
        let result = engine.execute_traversal(query).await.unwrap();
        assert_eq!(result.path.len(), 3);
        assert_eq!(result.path[0], node1);
        assert_eq!(result.path[2], node3);
    }
    
    #[tokio::test]
    async fn test_dijkstra_shortest_path() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let engine = TraversalEngine::new(core.clone()).await.unwrap();
        
        // Create test graph with weighted edges
        let node1 = core.create_node(1, NodeType::File).await.unwrap();
        let node2 = core.create_node(2, NodeType::File).await.unwrap();
        let node3 = core.create_node(3, NodeType::File).await.unwrap();
        
        let _edge1 = core.create_edge(node1, node2, EdgeType::References, 2.0).await.unwrap();
        let _edge2 = core.create_edge(node1, node3, EdgeType::References, 5.0).await.unwrap();
        let _edge3 = core.create_edge(node2, node3, EdgeType::References, 1.0).await.unwrap();
        
        // Test Dijkstra
        let query = TraversalQuery {
            algorithm: TraversalAlgorithm::Dijkstra,
            start_node: node1,
            end_node: Some(node3),
            max_depth: None,
            max_results: None,
            node_filter: None,
            edge_filter: None,
            weight_threshold: None,
            timeout_ms: None,
        };
        
        let result = engine.execute_traversal(query).await.unwrap();
        assert_eq!(result.path.len(), 3);
        assert_eq!(result.distances[&node3], 3.0); // 2 + 1 = 3
    }
}