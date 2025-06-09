//! Graph Health Monitor Implementation
//! 
//! This module implements comprehensive graph health monitoring and quality assessment
//! including connectivity analysis, consistency validation, and performance bottleneck detection.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::advanced_graph_analytics::*;
use crate::semantic_api::graph_journal_integration::GraphHealthMetrics;

use std::collections::{HashMap, VecDeque, HashSet};
use std::time::{SystemTime, Instant};
use uuid::Uuid;

impl GraphHealthMonitor {
    /// Create a new graph health monitor
    pub fn new(config: HealthMonitoringConfig) -> SemanticResult<Self> {
        Ok(Self {
            current_health: GraphHealthMetrics::default(),
            health_history: VecDeque::new(),
            quality_recommendations: Vec::new(),
            performance_bottlenecks: Vec::new(),
            config,
        })
    }

    /// Perform comprehensive graph health check
    pub async fn perform_comprehensive_health_check(&mut self) -> SemanticResult<GraphHealthSnapshot> {
        let start_time = Instant::now();
        
        // Mock graph data - in real implementation, this would come from the HNSW graph
        let mock_nodes = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mock_edges = vec![
            (1, 2, 1.0), (2, 3, 1.5), (3, 4, 2.0), (4, 5, 1.2), (5, 1, 0.8),
            (1, 3, 2.5), (2, 4, 1.8), (3, 5, 1.1), (6, 7, 1.3), (7, 8, 0.9),
            (8, 9, 1.6), (9, 10, 1.4), (10, 6, 2.1), (1, 6, 3.0), (5, 10, 2.8)
        ];
        
        // Calculate basic health metrics
        let basic_health = self.calculate_basic_health_metrics(&mock_nodes, &mock_edges).await?;
        
        // Calculate extended health metrics
        let extended_health = self.calculate_extended_health_metrics(&mock_nodes, &mock_edges).await?;
        
        // Calculate performance indicators
        let performance_indicators = self.calculate_performance_indicators(&mock_nodes, &mock_edges).await?;
        
        // Perform quality assessment
        let quality_assessment = self.perform_quality_assessment(&basic_health, &extended_health, &performance_indicators).await?;
        
        // Detect performance bottlenecks
        if self.config.enable_bottleneck_detection {
            self.detect_performance_bottlenecks(&mock_nodes, &mock_edges).await?;
        }
        
        // Generate quality recommendations
        self.generate_quality_recommendations(&quality_assessment).await?;
        
        // Create health snapshot
        let snapshot = GraphHealthSnapshot {
            basic_health: basic_health.clone(),
            extended_health,
            performance_indicators,
            quality_assessment,
            snapshot_timestamp: SystemTime::now(),
        };
        
        // Update current health and history
        self.current_health = basic_health;
        self.health_history.push_back(snapshot.clone());
        
        // Limit history size
        if self.health_history.len() > 100 {
            self.health_history.pop_front();
        }
        
        Ok(snapshot)
    }

    /// Calculate basic health metrics
    async fn calculate_basic_health_metrics(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<GraphHealthMetrics> {
        // Calculate connectivity score
        let connectivity_score = self.calculate_connectivity_score(nodes, edges).await?;
        
        // Calculate average path length
        let avg_path_length = self.calculate_average_path_length(nodes, edges).await?;
        
        // Calculate clustering coefficient
        let clustering_coefficient = self.calculate_clustering_coefficient(nodes, edges).await?;
        
        // Calculate graph density
        let graph_density = self.calculate_graph_density(nodes, edges).await?;
        
        // Count disconnected components
        let disconnected_components = self.count_disconnected_components(nodes, edges).await?;
        
        // Calculate overall quality score
        let quality_score = self.calculate_overall_quality_score(
            connectivity_score,
            avg_path_length,
            clustering_coefficient,
            graph_density,
            disconnected_components,
        ).await?;
        
        Ok(GraphHealthMetrics {
            connectivity_score,
            avg_path_length,
            clustering_coefficient,
            graph_density,
            disconnected_components,
            quality_score,
            last_health_check: SystemTime::now(),
        })
    }

    /// Calculate extended health metrics
    async fn calculate_extended_health_metrics(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<ExtendedHealthMetrics> {
        // Calculate graph diameter and radius
        let (graph_diameter, graph_radius) = self.calculate_diameter_radius(nodes, edges).await?;
        
        // Calculate assortativity coefficient
        let assortativity_coefficient = self.calculate_assortativity_coefficient(nodes, edges).await?;
        
        // Calculate rich club coefficient
        let rich_club_coefficient = self.calculate_rich_club_coefficient(nodes, edges).await?;
        
        // Calculate small world coefficient
        let small_world_coefficient = self.calculate_small_world_coefficient(nodes, edges).await?;
        
        // Analyze scale-free properties
        let scale_free_properties = self.analyze_scale_free_properties(nodes, edges).await?;
        
        Ok(ExtendedHealthMetrics {
            graph_diameter,
            graph_radius,
            assortativity_coefficient,
            rich_club_coefficient,
            small_world_coefficient,
            scale_free_properties,
        })
    }

    /// Calculate performance indicators
    async fn calculate_performance_indicators(
        &self,
        nodes: &[u64],
        edges: &[(u64, u64, f64)],
    ) -> SemanticResult<PerformanceIndicators> {
        // Mock performance calculations - in real implementation, these would be based on actual metrics
        let search_performance_score = self.calculate_search_performance_score(nodes, edges).await?;
        let insertion_performance_score = self.calculate_insertion_performance_score(nodes, edges).await?;
        let memory_efficiency_score = self.calculate_memory_efficiency_score(nodes, edges).await?;
        let cache_efficiency_score = self.calculate_cache_efficiency_score(nodes, edges).await?;
        
        let overall_performance_score = (search_performance_score + insertion_performance_score + 
                                       memory_efficiency_score + cache_efficiency_score) / 4.0;
        
        Ok(PerformanceIndicators {
            search_performance_score,
            insertion_performance_score,
            memory_efficiency_score,
            cache_efficiency_score,
            overall_performance_score,
        })
    }

    /// Perform quality assessment
    async fn perform_quality_assessment(
        &self,
        basic_health: &GraphHealthMetrics,
        extended_health: &ExtendedHealthMetrics,
        performance_indicators: &PerformanceIndicators,
    ) -> SemanticResult<QualityAssessment> {
        // Calculate component quality scores
        let structural_quality_score = self.calculate_structural_quality_score(basic_health, extended_health).await?;
        let performance_quality_score = performance_indicators.overall_performance_score;
        let consistency_quality_score = self.calculate_consistency_quality_score(basic_health).await?;
        
        // Calculate overall quality score
        let overall_quality_score = (structural_quality_score + performance_quality_score + consistency_quality_score) / 3.0;
        
        // Determine quality trend
        let quality_trend = self.determine_quality_trend(overall_quality_score).await?;
        
        // Generate recommendations
        let recommendations = self.generate_quality_recommendations_for_assessment(
            structural_quality_score,
            performance_quality_score,
            consistency_quality_score,
        ).await?;
        
        Ok(QualityAssessment {
            overall_quality_score,
            structural_quality_score,
            performance_quality_score,
            consistency_quality_score,
            quality_trend,
            recommendations,
        })
    }

    /// Calculate connectivity score
    async fn calculate_connectivity_score(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        if nodes.is_empty() {
            return Ok(0.0);
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
        
        // Find connected components using DFS
        let mut visited = HashSet::new();
        let mut components = 0;
        let mut largest_component_size = 0;
        
        for &node in nodes {
            if !visited.contains(&node) {
                let component_size = self.dfs_component_size(node, &adjacency, &mut visited);
                components += 1;
                largest_component_size = largest_component_size.max(component_size);
            }
        }
        
        // Connectivity score is the fraction of nodes in the largest component
        let connectivity_score = largest_component_size as f64 / nodes.len() as f64;
        
        Ok(connectivity_score)
    }

    /// DFS to find component size
    fn dfs_component_size(
        &self,
        node: u64,
        adjacency: &HashMap<u64, Vec<u64>>,
        visited: &mut HashSet<u64>,
    ) -> usize {
        if visited.contains(&node) {
            return 0;
        }
        
        visited.insert(node);
        let mut size = 1;
        
        if let Some(neighbors) = adjacency.get(&node) {
            for &neighbor in neighbors {
                size += self.dfs_component_size(neighbor, adjacency, visited);
            }
        }
        
        size
    }

    /// Calculate average path length
    async fn calculate_average_path_length(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        if nodes.len() < 2 {
            return Ok(0.0);
        }
        
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push((to, weight));
            adjacency.entry(to).or_insert_with(Vec::new).push((from, weight));
        }
        
        let mut total_path_length = 0.0;
        let mut path_count = 0;
        
        // Calculate shortest paths between all pairs (simplified)
        for &source in nodes {
            let distances = self.dijkstra_from_source(source, &adjacency).await?;
            
            for &target in nodes {
                if source != target {
                    if let Some(&distance) = distances.get(&target) {
                        if distance < f64::INFINITY {
                            total_path_length += distance;
                            path_count += 1;
                        }
                    }
                }
            }
        }
        
        let avg_path_length = if path_count > 0 {
            total_path_length / path_count as f64
        } else {
            f64::INFINITY
        };
        
        Ok(avg_path_length)
    }

    /// Calculate clustering coefficient
    async fn calculate_clustering_coefficient(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, HashSet::new());
        }
        for &(from, to, _weight) in edges {
            adjacency.entry(from).or_insert_with(HashSet::new).insert(to);
            adjacency.entry(to).or_insert_with(HashSet::new).insert(from);
        }
        
        let mut total_clustering = 0.0;
        let mut node_count = 0;
        
        for &node in nodes {
            if let Some(neighbors) = adjacency.get(&node) {
                let degree = neighbors.len();
                
                if degree < 2 {
                    continue; // Cannot form triangles
                }
                
                // Count triangles
                let mut triangles = 0;
                let neighbors_vec: Vec<_> = neighbors.iter().collect();
                
                for i in 0..neighbors_vec.len() {
                    for j in (i + 1)..neighbors_vec.len() {
                        let neighbor1 = *neighbors_vec[i];
                        let neighbor2 = *neighbors_vec[j];
                        
                        if let Some(neighbor1_adj) = adjacency.get(&neighbor1) {
                            if neighbor1_adj.contains(&neighbor2) {
                                triangles += 1;
                            }
                        }
                    }
                }
                
                // Clustering coefficient for this node
                let possible_triangles = degree * (degree - 1) / 2;
                let clustering = if possible_triangles > 0 {
                    triangles as f64 / possible_triangles as f64
                } else {
                    0.0
                };
                
                total_clustering += clustering;
                node_count += 1;
            }
        }
        
        let avg_clustering = if node_count > 0 {
            total_clustering / node_count as f64
        } else {
            0.0
        };
        
        Ok(avg_clustering)
    }

    /// Calculate graph density
    async fn calculate_graph_density(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        let n = nodes.len() as f64;
        if n < 2.0 {
            return Ok(0.0);
        }
        
        let max_edges = n * (n - 1.0) / 2.0; // For undirected graph
        let actual_edges = edges.len() as f64;
        
        let density = actual_edges / max_edges;
        
        Ok(density)
    }

    /// Count disconnected components
    async fn count_disconnected_components(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<usize> {
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, _weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push(to);
            adjacency.entry(to).or_insert_with(Vec::new).push(from);
        }
        
        let mut visited = HashSet::new();
        let mut components = 0;
        
        for &node in nodes {
            if !visited.contains(&node) {
                self.dfs_mark_component(node, &adjacency, &mut visited);
                components += 1;
            }
        }
        
        Ok(components)
    }

    /// DFS to mark all nodes in a component
    fn dfs_mark_component(
        &self,
        node: u64,
        adjacency: &HashMap<u64, Vec<u64>>,
        visited: &mut HashSet<u64>,
    ) {
        if visited.contains(&node) {
            return;
        }
        
        visited.insert(node);
        
        if let Some(neighbors) = adjacency.get(&node) {
            for &neighbor in neighbors {
                self.dfs_mark_component(neighbor, adjacency, visited);
            }
        }
    }

    /// Calculate overall quality score
    async fn calculate_overall_quality_score(
        &self,
        connectivity_score: f64,
        avg_path_length: f64,
        clustering_coefficient: f64,
        graph_density: f64,
        disconnected_components: usize,
    ) -> SemanticResult<f64> {
        let thresholds = &self.config.quality_thresholds;
        
        // Normalize each metric to 0-1 scale
        let connectivity_norm = connectivity_score.min(1.0);
        
        let path_length_norm = if avg_path_length < f64::INFINITY {
            (thresholds.max_avg_path_length - avg_path_length).max(0.0) / thresholds.max_avg_path_length
        } else {
            0.0
        };
        
        let clustering_norm = clustering_coefficient.min(1.0);
        let density_norm = graph_density.min(1.0);
        
        let components_norm = if disconnected_components <= 1 {
            1.0
        } else {
            (1.0 - (disconnected_components - 1) as f64 * thresholds.max_disconnected_ratio).max(0.0)
        };
        
        // Weighted average
        let quality_score = (connectivity_norm * 0.3 + 
                           path_length_norm * 0.2 + 
                           clustering_norm * 0.2 + 
                           density_norm * 0.15 + 
                           components_norm * 0.15).min(1.0);
        
        Ok(quality_score)
    }

    /// Calculate diameter and radius
    async fn calculate_diameter_radius(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<(f64, f64)> {
        // Build adjacency list
        let mut adjacency = HashMap::new();
        for &node in nodes {
            adjacency.insert(node, Vec::new());
        }
        for &(from, to, weight) in edges {
            adjacency.entry(from).or_insert_with(Vec::new).push((to, weight));
            adjacency.entry(to).or_insert_with(Vec::new).push((from, weight));
        }
        
        let mut max_distance = 0.0; // Diameter
        let mut min_eccentricity = f64::INFINITY; // Radius
        
        for &source in nodes {
            let distances = self.dijkstra_from_source(source, &adjacency).await?;
            
            // Find eccentricity (maximum distance from this node)
            let mut eccentricity = 0.0;
            for &target in nodes {
                if source != target {
                    if let Some(&distance) = distances.get(&target) {
                        if distance < f64::INFINITY {
                            eccentricity = eccentricity.max(distance);
                        }
                    }
                }
            }
            
            max_distance = max_distance.max(eccentricity);
            if eccentricity < f64::INFINITY {
                min_eccentricity = min_eccentricity.min(eccentricity);
            }
        }
        
        let diameter = max_distance;
        let radius = if min_eccentricity < f64::INFINITY { min_eccentricity } else { 0.0 };
        
        Ok((diameter, radius))
    }

    /// Calculate assortativity coefficient
    async fn calculate_assortativity_coefficient(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Calculate node degrees
        let mut degrees = HashMap::new();
        for &node in nodes {
            degrees.insert(node, 0);
        }
        for &(from, to, _weight) in edges {
            *degrees.entry(from).or_insert(0) += 1;
            *degrees.entry(to).or_insert(0) += 1;
        }
        
        // Calculate assortativity (simplified)
        let mut sum_degree_products = 0.0;
        let mut sum_degrees = 0.0;
        let mut sum_degree_squares = 0.0;
        let edge_count = edges.len() as f64;
        
        for &(from, to, _weight) in edges {
            let deg_from = degrees[&from] as f64;
            let deg_to = degrees[&to] as f64;
            
            sum_degree_products += deg_from * deg_to;
            sum_degrees += deg_from + deg_to;
            sum_degree_squares += deg_from * deg_from + deg_to * deg_to;
        }
        
        if edge_count > 0.0 {
            let mean_degree_product = sum_degree_products / edge_count;
            let mean_degree = sum_degrees / (2.0 * edge_count);
            let mean_degree_square = sum_degree_squares / (2.0 * edge_count);
            
            let numerator = mean_degree_product - mean_degree * mean_degree;
            let denominator = mean_degree_square - mean_degree * mean_degree;
            
            if denominator > 0.0 {
                Ok(numerator / denominator)
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }

    /// Calculate rich club coefficient
    async fn calculate_rich_club_coefficient(&self, _nodes: &[u64], _edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Simplified implementation
        Ok(0.35) // Mock value
    }

    /// Calculate small world coefficient
    async fn calculate_small_world_coefficient(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        let clustering_coefficient = self.calculate_clustering_coefficient(nodes, edges).await?;
        let avg_path_length = self.calculate_average_path_length(nodes, edges).await?;
        
        // Small world coefficient is typically clustering / path_length ratio compared to random graph
        // Simplified calculation
        let random_clustering = 2.0 * edges.len() as f64 / (nodes.len() * (nodes.len() - 1)) as f64;
        let random_path_length = (nodes.len() as f64).ln() / (2.0 * edges.len() as f64 / nodes.len() as f64).ln();
        
        let clustering_ratio = if random_clustering > 0.0 {
            clustering_coefficient / random_clustering
        } else {
            1.0
        };
        
        let path_length_ratio = if random_path_length > 0.0 && avg_path_length < f64::INFINITY {
            avg_path_length / random_path_length
        } else {
            1.0
        };
        
        let small_world_coefficient = if path_length_ratio > 0.0 {
            clustering_ratio / path_length_ratio
        } else {
            0.0
        };
        
        Ok(small_world_coefficient)
    }

    /// Analyze scale-free properties
    async fn analyze_scale_free_properties(&self, nodes: &[u64], edges: &[(u64, u64, f64)]) -> SemanticResult<ScaleFreeProperties> {
        // Calculate degree distribution
        let mut degrees = HashMap::new();
        for &node in nodes {
            degrees.insert(node, 0);
        }
        for &(from, to, _weight) in edges {
            *degrees.entry(from).or_insert(0) += 1;
            *degrees.entry(to).or_insert(0) += 1;
        }
        
        // Simplified power law analysis
        let power_law_exponent = 2.5; // Mock value
        let power_law_goodness_of_fit = 0.85; // Mock value
        let is_scale_free = power_law_goodness_of_fit > 0.8;
        
        // Calculate degree distribution entropy
        let mut degree_counts = HashMap::new();
        for &degree in degrees.values() {
            *degree_counts.entry(degree).or_insert(0) += 1;
        }
        
        let total_nodes = nodes.len() as f64;
        let mut entropy = 0.0;
        
        for &count in degree_counts.values() {
            let probability = count as f64 / total_nodes;
            if probability > 0.0 {
                entropy -= probability * probability.log2();
            }
        }
        
        Ok(ScaleFreeProperties {
            power_law_exponent,
            power_law_goodness_of_fit,
            is_scale_free,
            degree_distribution_entropy: entropy,
        })
    }

    /// Calculate search performance score
    async fn calculate_search_performance_score(&self, _nodes: &[u64], _edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Mock implementation - would be based on actual search metrics
        Ok(0.85)
    }

    /// Calculate insertion performance score
    async fn calculate_insertion_performance_score(&self, _nodes: &[u64], _edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Mock implementation - would be based on actual insertion metrics
        Ok(0.78)
    }

    /// Calculate memory efficiency score
    async fn calculate_memory_efficiency_score(&self, _nodes: &[u64], _edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Mock implementation - would be based on actual memory usage
        Ok(0.82)
    }

    /// Calculate cache efficiency score
    async fn calculate_cache_efficiency_score(&self, _nodes: &[u64], _edges: &[(u64, u64, f64)]) -> SemanticResult<f64> {
        // Mock implementation - would be based on actual cache metrics
        Ok(0.88)
    }

    /// Calculate structural quality score
    async fn calculate_structural_quality_score(
        &self,
        basic_health: &GraphHealthMetrics,
        extended_health: &ExtendedHealthMetrics,
    ) -> SemanticResult<f64> {
        let structural_score = (basic_health.connectivity_score * 0.3 +
                              basic_health.clustering_coefficient * 0.25 +
                              basic_health.graph_density * 0.2 +
                              (1.0 - extended_health.assortativity_coefficient.abs()) * 0.15 +
                              extended_health.small_world_coefficient.min(1.0) * 0.1).min(1.0);
        
        Ok(structural_score)
    }

    /// Calculate consistency quality score
    async fn calculate_consistency_quality_score(&self, basic_health: &GraphHealthMetrics) -> SemanticResult<f64> {
        // Consistency based on disconnected components and connectivity
        let consistency_score = if basic_health.disconnected_components <= 1 {
            basic_health.connectivity_score
        } else {
            basic_health.connectivity_score * 0.5
        };
        
        Ok(consistency_score)
    }

    /// Determine quality trend
    async fn determine_quality_trend(&self, current_quality: f64) -> SemanticResult<QualityTrend> {
        if self.health_history.len() < 2 {
            return Ok(QualityTrend::Stable);
        }
        
        let recent_scores: Vec<f64> = self.health_history.iter()
            .rev()
            .take(5)
            .map(|snapshot| snapshot.quality_assessment.overall_quality_score)
            .collect();
        
        if recent_scores.len() < 2 {
            return Ok(QualityTrend::Stable);
        }
        
        let trend_slope = self.calculate_trend_slope(&recent_scores);
        
        if trend_slope > 0.05 {
            Ok(QualityTrend::Improving)
        } else if trend_slope < -0.05 {
            Ok(QualityTrend::Degrading)
        } else {
            let variance = self.calculate_variance(&recent_scores);
            if variance > 0.01 {
                Ok(QualityTrend::Fluctuating)
            } else {
                Ok(QualityTrend::Stable)
            }
        }
    }

    /// Calculate trend slope using linear regression
    fn calculate_trend_slope(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64) * (i as f64)).sum();
        
        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator.abs() < 1e-10 {
            return 0.0;
        }
        
        (n * sum_xy - sum_x * sum_y) / denominator
    }

    /// Calculate variance
    fn calculate_variance(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|&x| (x - mean) * (x - mean))
            .sum::<f64>() / values.len() as f64;
        
        variance
    }

    /// Generate quality recommendations for assessment
    async fn generate_quality_recommendations_for_assessment(
        &self,
        structural_quality: f64,
        performance_quality: f64,
        consistency_quality: f64,
    ) -> SemanticResult<Vec<QualityRecommendation>> {
        let mut recommendations = Vec::new();
        
        if structural_quality < 0.7 {
            recommendations.push(QualityRecommendation {
                recommendation_id: Uuid::new_v4(),
                recommendation_type: RecommendationType::StructuralImprovement,
                priority: RecommendationPriority::High,
                description: "Graph structure shows poor connectivity and clustering".to_string(),
                expected_impact: "Improved search performance and graph traversal efficiency".to_string(),
                implementation_effort: ImplementationEffort::Medium,
                created_at: SystemTime::now(),
            });
        }
        
        if performance_quality < 0.6 {
            recommendations.push(QualityRe