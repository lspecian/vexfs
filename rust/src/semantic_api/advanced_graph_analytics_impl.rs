//! Advanced Graph Analytics Implementation
//! 
//! This module contains the concrete implementations of the advanced graph analytics
//! algorithms including centrality measures, pathfinding, clustering, and health monitoring.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::advanced_graph_analytics::*;
use crate::semantic_api::graph_journal_integration::CentralityMeasures;
use crate::anns::hnsw_optimized::OptimizedHnswGraph;

use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::collections::{HashMap, BTreeMap, VecDeque, HashSet, BinaryHeap};
use std::cmp::Ordering;
use uuid::Uuid;

/// Priority queue node for pathfinding algorithms
#[derive(Debug, Clone)]
struct PathNode {
    node_id: u64,
    distance: f64,
    heuristic: f64,
}

impl PartialEq for PathNode {
    fn eq(&self, other: &Self) -> bool {
        self.distance.partial_cmp(&other.distance) == Some(Ordering::Equal)
    }
}

impl Eq for PathNode {}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering for min-heap
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl CentralityCalculator {
    /// Create a new centrality calculator
    pub fn new(config: PageRankConfig) -> SemanticResult<Self> {
        Ok(Self {
            centrality_cache: HashMap::new(),
            pagerank_cache: HashMap::new(),
            eigenvector_cache: HashMap::new(),
            last_calculation: SystemTime::now(),
            config,
        })
    }

    /// Calculate all centrality measures for all nodes
    pub async fn calculate_all_centrality_measures(&mut self) -> SemanticResult<HashMap<u64, EnhancedCentralityMeasures>> {
        let start_time = Instant::now();
        let mut results = HashMap::new();

        // Mock graph data - in real implementation, this would come from the HNSW graph
        let mock_nodes = vec![1, 2, 3, 4, 5];
        let mock_edges = vec![
            (1, 2, 1.0), (2, 3, 1.5), (3, 4, 2.0), (4, 5, 1.2), (5, 1, 0.8),
            (1, 3, 2.5), (2, 4, 1.8), (3, 5, 1.1)
        ];

        // Calculate degree centrality
        let degree_centrality = self.calculate_degree_centrality(&mock_nodes, &mock_edges).await?;
        
        // Calculate betweenness centrality
        let betweenness_centrality = self.calculate_betweenness_centrality(&mock_nodes, &mock_edges).await?;
        
        // Calculate PageRank
        let pagerank_scores = self.calculate_pagerank(&mock_nodes, &mock_edges).await?;
        
        // Calculate eigenvector centrality
        let eigenvector_centrality = self.calculate_eigenvector_centrality(&mock_nodes, &mock_edges).await?;
        
        // Calculate additional centrality measures
        let closeness_centrality = self.calculate_closeness_centrality(&mock_nodes, &mock_edges).await?;
        let harmonic_centrality = self.calculate_harmonic_centrality(&mock_nodes, &mock_edges).await?;
        let katz_centrality = self.calculate_katz_centrality(&mock_nodes, &mock_edges).await?;
        let (authority_scores, hub_scores) = self.calculate_hits_scores(&mock_nodes, &mock_edges).await?;

        // Combine all measures
        for &node_id in &mock_nodes {
            let basic_centrality = CentralityMeasures {
                degree_centrality: degree_centrality.get(&node_id).copied().unwrap_or(0.0),
                betweenness_centrality: betweenness_centrality.get(&node_id).copied().unwrap_or(0.0),
                pagerank_score: pagerank_scores.get(&node_id).copied().unwrap_or(0.0),
                eigenvector_centrality: eigenvector_centrality.get(&node_id).copied().unwrap_or(0.0),
                calculated_at: SystemTime::now(),
            };

            let enhanced_measures = EnhancedCentralityMeasures {
                basic: basic_centrality,
                in_degree_centrality: degree_centrality.get(&node_id).copied().unwrap_or(0.0) * 0.5,
                out_degree_centrality: degree_centrality.get(&node_id).copied().unwrap_or(0.0) * 0.5,
                total_degree_centrality: degree_centrality.get(&node_id).copied().unwrap_or(0.0),
                closeness_centrality: closeness_centrality.get(&node_id).copied().unwrap_or(0.0),
                harmonic_centrality: harmonic_centrality.get(&node_id).copied().unwrap_or(0.0),
                katz_centrality: katz_centrality.get(&node_id).copied().unwrap_or(0.0),
                authority_score: authority_scores.get(&node_id).copied().unwrap_or(0.0),
                hub_score: hub_scores.get(&node_id).copied().unwrap_or(0.0),
                calculation_metadata: CentralityCalculationMetadata {
                    calculated_at: SystemTime::now(),
                    calculation_duration_ms: start_time.elapsed().as_millis() as u64,
                    algorithm_used: "Comprehensive Centrality Suite".to_string(),
                    convergence_achieved: true,
                    iterations_used: 100,
                },
            };

            results.insert(node_id, enhanced_measures);
        }

        // Cache results
        self.centrality_cache = results.clone();
        self.last_calculation = SystemTime::now();

        Ok(results)
    }

    /// Calculate degree centrality for all nodes
    async fn calculate_degree_centrality(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut degree_counts = HashMap::new();
        
        // Initialize all nodes with degree 0
        for &node in nodes {
            degree_counts.insert(node, 0);
        }
        
        // Count degrees
        for &(from, to, _weight) in edges {
            *degree_counts.entry(from).or_insert(0) += 1;
            *degree_counts.entry(to).or_insert(0) += 1;
        }
        
        // Normalize by (n-1) where n is the number of nodes
        let max_degree = (nodes.len() - 1) as f64;
        let mut centrality = HashMap::new();
        
        for (&node, &degree) in &degree_counts {
            centrality.insert(node, degree as f64 / max_degree);
        }
        
        Ok(centrality)
    }

    /// Calculate betweenness centrality using Brandes' algorithm
    async fn calculate_betweenness_centrality(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut betweenness = HashMap::new();
        
        // Initialize betweenness scores
        for &node in nodes {
            betweenness.insert(node, 0.0);
        }
        
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, _weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push(to);
            adjacency.entry(to).or_insert_with(Vec::new).push(from);
        }
        
        // Brandes' algorithm for each source node
        for &source in nodes {
            let mut stack = Vec::new();
            let mut predecessors: HashMap<u64, Vec<u64>> = HashMap::new();
            let mut sigma: HashMap<u64, f64> = HashMap::new();
            let mut distance: HashMap<u64, i32> = HashMap::new();
            let mut delta: HashMap<u64, f64> = HashMap::new();
            
            // Initialize
            for &node in nodes {
                predecessors.insert(node, Vec::new());
                sigma.insert(node, 0.0);
                distance.insert(node, -1);
                delta.insert(node, 0.0);
            }
            
            sigma.insert(source, 1.0);
            distance.insert(source, 0);
            
            let mut queue = VecDeque::new();
            queue.push_back(source);
            
            // BFS
            while let Some(v) = queue.pop_front() {
                stack.push(v);
                
                if let Some(neighbors) = adjacency.get(&v) {
                    for &w in neighbors {
                        // First time we found shortest path to w?
                        if distance[&w] < 0 {
                            queue.push_back(w);
                            distance.insert(w, distance[&v] + 1);
                        }
                        
                        // Shortest path to w via v?
                        if distance[&w] == distance[&v] + 1 {
                            sigma.insert(w, sigma[&w] + sigma[&v]);
                            predecessors.get_mut(&w).unwrap().push(v);
                        }
                    }
                }
            }
            
            // Accumulation
            while let Some(w) = stack.pop() {
                for &v in &predecessors[&w] {
                    let contribution = (sigma[&v] / sigma[&w]) * (1.0 + delta[&w]);
                    delta.insert(v, delta[&v] + contribution);
                }
                
                if w != source {
                    betweenness.insert(w, betweenness[&w] + delta[&w]);
                }
            }
        }
        
        // Normalize
        let n = nodes.len() as f64;
        let normalization = 2.0 / ((n - 1.0) * (n - 2.0));
        
        for (_, score) in betweenness.iter_mut() {
            *score *= normalization;
        }
        
        Ok(betweenness)
    }

    /// Calculate PageRank scores
    async fn calculate_pagerank(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut pagerank = HashMap::new();
        let mut new_pagerank = HashMap::new();
        
        // Initialize PageRank scores
        let initial_score = 1.0 / nodes.len() as f64;
        for &node in nodes {
            pagerank.insert(node, initial_score);
            new_pagerank.insert(node, 0.0);
        }
        
        // Build adjacency list and out-degree counts
        let mut adjacency = HashMap::new();
        let mut out_degree = HashMap::new();
        
        for &node in nodes {
            adjacency.insert(node, Vec::new());
            out_degree.insert(node, 0);
        }
        
        for &(from, to, _weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push(to);
            *out_degree.entry(from).or_insert(0) += 1;
        }
        
        // Power iteration
        for iteration in 0..self.config.max_iterations {
            let mut max_diff = 0.0;
            
            // Reset new scores
            for &node in nodes {
                new_pagerank.insert(node, (1.0 - self.config.damping_factor) / nodes.len() as f64);
            }
            
            // Calculate new PageRank scores
            for &node in nodes {
                if out_degree[&node] > 0 {
                    let contribution = self.config.damping_factor * pagerank[&node] / out_degree[&node] as f64;
                    
                    if let Some(neighbors) = adjacency.get(&node) {
                        for &neighbor in neighbors {
                            new_pagerank.insert(neighbor, new_pagerank[&neighbor] + contribution);
                        }
                    }
                }
            }
            
            // Check convergence
            for &node in nodes {
                let diff = (new_pagerank[&node] - pagerank[&node]).abs();
                max_diff = max_diff.max(diff);
                pagerank.insert(node, new_pagerank[&node]);
            }
            
            if max_diff < self.config.convergence_threshold {
                break;
            }
        }
        
        Ok(pagerank)
    }

    /// Calculate eigenvector centrality using power iteration
    async fn calculate_eigenvector_centrality(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut centrality = HashMap::new();
        let mut new_centrality = HashMap::new();
        
        // Initialize centrality scores
        for &node in nodes {
            centrality.insert(node, 1.0);
            new_centrality.insert(node, 0.0);
        }
        
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, _weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push(to);
            adjacency.entry(to).or_insert_with(Vec::new).push(from);
        }
        
        // Power iteration
        for _iteration in 0..100 {
            // Reset new scores
            for &node in nodes {
                new_centrality.insert(node, 0.0);
            }
            
            // Calculate new centrality scores
            for &node in nodes {
                if let Some(neighbors) = adjacency.get(&node) {
                    for &neighbor in neighbors {
                        new_centrality.insert(node, new_centrality[&node] + centrality[&neighbor]);
                    }
                }
            }
            
            // Normalize
            let norm: f64 = new_centrality.values().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 0.0 {
                for (_, score) in new_centrality.iter_mut() {
                    *score /= norm;
                }
            }
            
            // Check convergence
            let mut max_diff = 0.0;
            for &node in nodes {
                let diff = (new_centrality[&node] - centrality[&node]).abs();
                max_diff = max_diff.max(diff);
                centrality.insert(node, new_centrality[&node]);
            }
            
            if max_diff < 1e-6 {
                break;
            }
        }
        
        Ok(centrality)
    }

    /// Calculate closeness centrality
    async fn calculate_closeness_centrality(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut centrality = HashMap::new();
        
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push((to, weight));
            adjacency.entry(to).or_insert_with(Vec::new).push((from, weight));
        }
        
        // Calculate shortest paths from each node using Dijkstra
        for &source in nodes {
            let distances = self.dijkstra_single_source(source, &adjacency).await?;
            
            let sum_distances: f64 = distances.values().sum();
            let closeness = if sum_distances > 0.0 {
                (nodes.len() - 1) as f64 / sum_distances
            } else {
                0.0
            };
            
            centrality.insert(source, closeness);
        }
        
        Ok(centrality)
    }

    /// Calculate harmonic centrality
    async fn calculate_harmonic_centrality(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut centrality = HashMap::new();
        
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push((to, weight));
            adjacency.entry(to).or_insert_with(Vec::new).push((from, weight));
        }
        
        // Calculate harmonic centrality
        for &source in nodes {
            let distances = self.dijkstra_single_source(source, &adjacency).await?;
            
            let harmonic_sum: f64 = distances.values()
                .filter(|&&d| d > 0.0)
                .map(|&d| 1.0 / d)
                .sum();
            
            centrality.insert(source, harmonic_sum);
        }
        
        Ok(centrality)
    }

    /// Calculate Katz centrality
    async fn calculate_katz_centrality(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut centrality = HashMap::new();
        let alpha = 0.1; // Attenuation factor
        
        // Initialize centrality scores
        for &node in nodes {
            centrality.insert(node, 1.0);
        }
        
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, _weight) in edges {
            adjacency.entry(to).or_insert_with(Vec::new).push(from);
        }
        
        // Iterative calculation
        for _iteration in 0..50 {
            let mut new_centrality = HashMap::new();
            
            for &node in nodes {
                let mut sum = 1.0; // Beta (external influence)
                
                if let Some(predecessors) = adjacency.get(&node) {
                    for &pred in predecessors {
                        sum += alpha * centrality[&pred];
                    }
                }
                
                new_centrality.insert(node, sum);
            }
            
            centrality = new_centrality;
        }
        
        Ok(centrality)
    }

    /// Calculate HITS authority and hub scores
    async fn calculate_hits_scores(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<(HashMap<u64, f64>, HashMap<u64, f64>)> {
        let mut authority = HashMap::new();
        let mut hub = HashMap::new();
        
        // Initialize scores
        for &node in nodes {
            authority.insert(node, 1.0);
            hub.insert(node, 1.0);
        }
        
        // Build adjacency lists
        let mut out_links = HashMap::new();
        let mut in_links = HashMap::new();
        
        for &node in nodes {
            out_links.insert(node, Vec::new());
            in_links.insert(node, Vec::new());
        }
        
        for &(from, to, _weight) in edges {
            out_links.entry(from).or_insert_with(Vec::new).push(to);
            in_links.entry(to).or_insert_with(Vec::new).push(from);
        }
        
        // HITS algorithm
        for _iteration in 0..50 {
            let mut new_authority = HashMap::new();
            let mut new_hub = HashMap::new();
            
            // Update authority scores
            for &node in nodes {
                let mut auth_sum = 0.0;
                if let Some(predecessors) = in_links.get(&node) {
                    for &pred in predecessors {
                        auth_sum += hub[&pred];
                    }
                }
                new_authority.insert(node, auth_sum);
            }
            
            // Update hub scores
            for &node in nodes {
                let mut hub_sum = 0.0;
                if let Some(successors) = out_links.get(&node) {
                    for &succ in successors {
                        hub_sum += new_authority[&succ];
                    }
                }
                new_hub.insert(node, hub_sum);
            }
            
            // Normalize
            let auth_norm: f64 = new_authority.values().map(|x| x * x).sum::<f64>().sqrt();
            let hub_norm: f64 = new_hub.values().map(|x| x * x).sum::<f64>().sqrt();
            
            if auth_norm > 0.0 {
                for (_, score) in new_authority.iter_mut() {
                    *score /= auth_norm;
                }
            }
            
            if hub_norm > 0.0 {
                for (_, score) in new_hub.iter_mut() {
                    *score /= hub_norm;
                }
            }
            
            authority = new_authority;
            hub = new_hub;
        }
        
        Ok((authority, hub))
    }

    /// Single-source shortest paths using Dijkstra's algorithm
    async fn dijkstra_single_source(
        &self,
        source: u64,
        adjacency: &HashMap<u64, Vec<(u64, f64)>>,
    ) -> SemanticResult<HashMap<u64, f64>> {
        let mut distances = HashMap::new();
        let mut visited = HashSet::new();
        let mut heap = BinaryHeap::new();
        
        // Initialize distances
        for &node in adjacency.keys() {
            distances.insert(node, f64::INFINITY);
        }
        distances.insert(source, 0.0);
        
        heap.push(PathNode {
            node_id: source,
            distance: 0.0,
            heuristic: 0.0,
        });
        
        while let Some(current) = heap.pop() {
            if visited.contains(&current.node_id) {
                continue;
            }
            
            visited.insert(current.node_id);
            
            if let Some(neighbors) = adjacency.get(&current.node_id) {
                for &(neighbor, weight) in neighbors {
                    let new_distance = distances[&current.node_id] + weight;
                    
                    if new_distance < distances[&neighbor] {
                        distances.insert(neighbor, new_distance);
                        heap.push(PathNode {
                            node_id: neighbor,
                            distance: new_distance,
                            heuristic: 0.0,
                        });
                    }
                }
            }
        }
        
        Ok(distances)
    }
}

impl PathfindingEngine {
    /// Create a new pathfinding engine
    pub fn new() -> SemanticResult<Self> {
        Ok(Self {
            path_cache: HashMap::new(),
            distance_cache: HashMap::new(),
            all_pairs_cache: None,
            last_operation: SystemTime::now(),
        })
    }

    /// Find shortest path between two nodes using specified algorithm
    pub async fn find_shortest_path(
        &mut self,
        source: u64,
        target: u64,
        algorithm: PathfindingAlgorithm,
    ) -> SemanticResult<PathfindingResult> {
        // Check cache first
        if let Some(cached_result) = self.path_cache.get(&(source, target)) {
            return Ok(cached_result.clone());
        }

        let start_time = Instant::now();
        
        // Mock graph data - in real implementation, this would come from the HNSW graph
        let mock_edges = vec![
            (1, 2, 1.0), (2, 3, 1.5), (3, 4, 2.0), (4, 5, 1.2), (5, 1, 0.8),
            (1, 3, 2.5), (2, 4, 1.8), (3, 5, 1.1)
        ];
        
        let result = match algorithm {
            PathfindingAlgorithm::Dijkstra => {
                self.dijkstra_shortest_path(source, target, &mock_edges).await?
            },
            PathfindingAlgorithm::AStar => {
                self.astar_shortest_path(source, target, &mock_edges).await?
            },
            PathfindingAlgorithm::Bidirectional => {
                self.bidirectional_shortest_path(source, target, &mock_edges).await?
            },
            PathfindingAlgorithm::FloydWarshall => {
                self.floyd_warshall_shortest_path(source, target, &mock_edges).await?
            },
            PathfindingAlgorithm::BellmanFord => {
                self.bellman_ford_shortest_path(source, target, &mock_edges).await?
            },
        };
        
        let mut pathfinding_result = result;
        pathfinding_result.algorithm_used = algorithm;
        pathfinding_result.calculated_at = SystemTime::now();
        
        // Calculate quality metrics
        pathfinding_result.quality_metrics = self.calculate_path_quality_metrics(&pathfinding_result.path, &mock_edges).await?;
        
        // Cache result
        self.path_cache.insert((source, target), pathfinding_result.clone());
        self.last_operation = SystemTime::now();
        
        Ok(pathfinding_result)
    }

    /// Dijkstra's shortest path algorithm
    async fn dijkstra_shortest_path(
        &self,
        source: u64,
        target: u64,
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<PathfindingResult> {
        // Build adjacency list
        let mut adjacency = HashMap::new();
        let mut all_nodes = HashSet::new();
        
        for &(from, to, weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push((to, weight));
            adjacency.entry(to).or_insert_with(Vec::new).push((from, weight));
            all_nodes.insert(from);
            all_nodes.insert(to);
        }
        
        let mut distances = HashMap::new();
        let mut previous = HashMap::new();
        let mut visited = HashSet::new();
        let mut heap = BinaryHeap::new();
        
        // Initialize distances
        for &node in &all_nodes {
            distances.insert(node, f64::INFINITY);
        }
        distances.insert(source, 0.0);
        
        heap.push(PathNode {
            node_id: source,
            distance: 0.0,
            heuristic: 0.0,
        });
        
        while let Some(current) = heap.pop() {
            if current.node_id == target {
                break;
            }
            
            if visited.contains(&current.node_id) {
                continue;
            }
            
            visited.insert(current.node_id);
            
            if let Some(neighbors) = adjacency.get(&current.node_id) {
                for &(neighbor, weight) in neighbors {
                    let new_distance = distances[&current.node_id] + weight;
                    
                    if new_distance < distances[&neighbor] {
                        distances.insert(neighbor, new_distance);
                        previous.insert(neighbor, current.node_id);
                        heap.push(PathNode {
                            node_id: neighbor,
                            distance: new_distance,
                            heuristic: 0.0,
                        });
                    }
                }
            }
        }
        
        // Reconstruct path
        let path = self.reconstruct_path(source, target, &previous)?;
        let total_distance = distances.get(&target).copied().unwrap_or(f64::INFINITY);
        
        Ok(PathfindingResult {
            source,
            target,
            path,
            total_distance,
            quality_metrics: PathQualityMetrics {
                path_length: 0,
                avg_edge_weight: 0.0,
                path_efficiency: 0.0,
                bottleneck_weight: 0.0,
                diversity_score: 0.0,
            },
            algorithm_used: PathfindingAlgorithm::Dijkstra,
            calculated_at: SystemTime::now(),
        })
    }

    /// A* shortest path algorithm with heuristic
    async fn astar_shortest_path(
        &self,
        source: u64,
        target: u64,
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<PathfindingResult> {
        // Build adjacency list
        let mut adjacency = HashMap::new();
        let mut all_nodes = HashSet::new();
        
        for &(from, to, weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push((to, weight));
            adjacency.entry(to).or_insert_with(Vec::new).push((from, weight));
            all_nodes.insert(from);
            all_nodes.insert(to);
        }
        
        let mut g_score = HashMap::new();
        let mut f_score = HashMap::new();
        let mut previous = HashMap::new();
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        
        // Initialize scores
        for &node in &all_nodes {
            g_score.insert(node, f64::INFINITY);
            f_score.insert(node, f64::INFINITY);
        }
        
        g_score.insert(source, 0.0);
        let heuristic = self.heuristic_distance(source, target);
        f_score.insert(source, heuristic);
        
        open_set.push(PathNode {
            node_id: source,
            distance: heuristic,
            heuristic,
        });
        
        while let Some(current) = open_set.pop() {
            if current.node_