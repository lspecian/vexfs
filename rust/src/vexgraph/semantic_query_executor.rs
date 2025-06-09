/*
 * VexFS v2.0 - VexGraph Semantic Query Executor (Task 11)
 * 
 * This module implements the execution engine for unified semantic queries
 * that combine graph traversal with vector similarity search.
 */

use crate::vexgraph::{
    NodeId, EdgeId, PropertyType, NodeType, EdgeType, TraversalAlgorithm,
    core::{VexGraphCore, GraphNode, GraphEdge},
    error_handling::{VexGraphError, VexGraphResult},
    semantic_search_manager::{SemanticSearchManager, SemanticSearchConfig},
    semantic_query_language::{
        UnifiedSemanticQuery, UnifiedQueryResult, GraphQuery, VectorQuery,
        CombinationStrategy, ResultOrdering, PropertyOperation, PathInfo,
        ExecutionMetadata, DistanceMetric,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque, BinaryHeap};
use std::sync::Arc;
use std::cmp::Ordering;
use uuid::Uuid;

/// Query execution engine for unified semantic queries
#[derive(Debug)]
pub struct SemanticQueryExecutor {
    /// Reference to the graph core
    core: Arc<VexGraphCore>,
    
    /// Semantic search manager
    search_manager: Arc<SemanticSearchManager>,
    
    /// Query optimization settings
    optimization_config: QueryOptimizationConfig,
    
    /// Execution statistics
    stats: parking_lot::RwLock<ExecutorStatistics>,
}

/// Configuration for query optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptimizationConfig {
    /// Enable query plan optimization
    pub enable_optimization: bool,
    
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    
    /// Enable result caching
    pub enable_caching: bool,
    
    /// Enable parallel execution
    pub enable_parallel_execution: bool,
    
    /// Batch size for bulk operations
    pub batch_size: usize,
    
    /// Memory limit for query execution
    pub memory_limit_mb: usize,
}

impl Default for QueryOptimizationConfig {
    fn default() -> Self {
        Self {
            enable_optimization: true,
            max_execution_time_ms: 30000, // 30 seconds
            enable_caching: true,
            enable_parallel_execution: true,
            batch_size: 1000,
            memory_limit_mb: 512,
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorStatistics {
    pub total_queries_executed: u64,
    pub average_execution_time_ms: f64,
    pub cache_hit_rate: f64,
    pub optimization_success_rate: f64,
    pub memory_usage_mb: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Query execution plan
#[derive(Debug, Clone)]
struct QueryPlan {
    /// Execution steps in order
    steps: Vec<ExecutionStep>,
    
    /// Estimated cost
    estimated_cost: f64,
    
    /// Expected result count
    expected_results: usize,
    
    /// Memory requirements
    memory_requirement_mb: usize,
}

/// Individual execution step
#[derive(Debug, Clone)]
enum ExecutionStep {
    /// Execute graph traversal
    GraphTraversal {
        query: GraphQuery,
        estimated_nodes: usize,
    },
    
    /// Execute vector search
    VectorSearch {
        query: VectorQuery,
        estimated_results: usize,
    },
    
    /// Combine results from previous steps
    CombineResults {
        strategy: CombinationStrategy,
        left_step: usize,
        right_step: usize,
    },
    
    /// Filter results
    FilterResults {
        filters: Vec<ResultFilter>,
        input_step: usize,
    },
    
    /// Sort and limit results
    SortAndLimit {
        ordering: ResultOrdering,
        limit: usize,
        input_step: usize,
    },
}

/// Result filter for post-processing
#[derive(Debug, Clone)]
enum ResultFilter {
    SimilarityThreshold(f32),
    GraphDistanceLimit(u32),
    NodeTypeFilter(Vec<NodeType>),
    PropertyFilter(String, PropertyOperation, PropertyType),
}

/// Intermediate execution result
#[derive(Debug, Clone)]
struct IntermediateResult {
    /// Node results with scores
    nodes: Vec<ScoredNode>,
    
    /// Execution metadata
    metadata: ExecutionMetadata,
}

/// Node with associated scores
#[derive(Debug, Clone)]
struct ScoredNode {
    node_id: NodeId,
    graph_score: Option<f32>,
    vector_score: Option<f32>,
    combined_score: f32,
    graph_distance: Option<u32>,
    path_info: Option<PathInfo>,
    metadata: HashMap<String, String>,
}

impl Eq for ScoredNode {}

impl PartialEq for ScoredNode {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }
}

impl Ord for ScoredNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher scores first
        other.combined_score.partial_cmp(&self.combined_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for ScoredNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl SemanticQueryExecutor {
    /// Create a new query executor
    pub async fn new(
        core: Arc<VexGraphCore>,
        search_config: SemanticSearchConfig,
        optimization_config: QueryOptimizationConfig,
    ) -> VexGraphResult<Self> {
        let search_manager = Arc::new(
            SemanticSearchManager::new(core.clone(), search_config).await?
        );

        let stats = ExecutorStatistics {
            total_queries_executed: 0,
            average_execution_time_ms: 0.0,
            cache_hit_rate: 0.0,
            optimization_success_rate: 0.0,
            memory_usage_mb: 0.0,
            last_updated: chrono::Utc::now(),
        };

        Ok(Self {
            core,
            search_manager,
            optimization_config,
            stats: parking_lot::RwLock::new(stats),
        })
    }

    /// Execute a unified semantic query
    pub async fn execute_query(
        &self,
        query: UnifiedSemanticQuery,
    ) -> VexGraphResult<Vec<UnifiedQueryResult>> {
        let start_time = std::time::Instant::now();
        let query_id = query.metadata.query_id.clone();

        tracing::info!("Executing unified semantic query: {}", query_id);

        // Create execution plan
        let plan = if self.optimization_config.enable_optimization {
            self.create_optimized_plan(&query).await?
        } else {
            self.create_basic_plan(&query).await?
        };

        tracing::debug!("Created execution plan with {} steps", plan.steps.len());

        // Execute the plan
        let results = self.execute_plan(plan, &query).await?;

        // Update statistics
        let execution_time = start_time.elapsed().as_millis() as f64;
        self.update_statistics(execution_time, results.len());

        tracing::info!(
            "Query {} completed in {:.2}ms with {} results",
            query_id,
            execution_time,
            results.len()
        );

        Ok(results)
    }

    /// Create an optimized execution plan
    async fn create_optimized_plan(&self, query: &UnifiedSemanticQuery) -> VexGraphResult<QueryPlan> {
        let mut steps = Vec::new();
        let mut estimated_cost = 0.0;
        let mut expected_results = 0;

        // Analyze query components
        let has_graph = query.graph_query.is_some();
        let has_vector = query.vector_query.is_some();

        match (&query.graph_query, &query.vector_query, &query.combination) {
            // Vector-first strategies (more selective)
            (Some(graph_q), Some(vector_q), CombinationStrategy::VectorFirst) |
            (Some(graph_q), Some(vector_q), CombinationStrategy::Intersection) => {
                // Execute vector search first (typically more selective)
                steps.push(ExecutionStep::VectorSearch {
                    query: vector_q.clone(),
                    estimated_results: vector_q.k,
                });
                estimated_cost += self.estimate_vector_search_cost(vector_q);
                expected_results = vector_q.k;

                // Then filter by graph constraints
                steps.push(ExecutionStep::GraphTraversal {
                    query: graph_q.clone(),
                    estimated_nodes: expected_results * 10, // Estimate expansion
                });
                estimated_cost += self.estimate_graph_traversal_cost(graph_q);

                // Combine results
                steps.push(ExecutionStep::CombineResults {
                    strategy: query.combination,
                    left_step: 0,
                    right_step: 1,
                });
                estimated_cost += expected_results as f64 * 0.1;
            }

            // Graph-first strategies
            (Some(graph_q), Some(vector_q), CombinationStrategy::GraphFirst) => {
                // Execute graph traversal first
                steps.push(ExecutionStep::GraphTraversal {
                    query: graph_q.clone(),
                    estimated_nodes: self.estimate_graph_result_size(graph_q),
                });
                estimated_cost += self.estimate_graph_traversal_cost(graph_q);
                expected_results = self.estimate_graph_result_size(graph_q);

                // Then vector search on results
                steps.push(ExecutionStep::VectorSearch {
                    query: vector_q.clone(),
                    estimated_results: vector_q.k.min(expected_results),
                });
                estimated_cost += self.estimate_vector_search_cost(vector_q);

                // Combine results
                steps.push(ExecutionStep::CombineResults {
                    strategy: query.combination,
                    left_step: 0,
                    right_step: 1,
                });
                estimated_cost += expected_results as f64 * 0.1;
            }

            // Union strategy (execute both in parallel)
            (Some(graph_q), Some(vector_q), CombinationStrategy::Union) => {
                steps.push(ExecutionStep::GraphTraversal {
                    query: graph_q.clone(),
                    estimated_nodes: self.estimate_graph_result_size(graph_q),
                });
                
                steps.push(ExecutionStep::VectorSearch {
                    query: vector_q.clone(),
                    estimated_results: vector_q.k,
                });

                estimated_cost += self.estimate_graph_traversal_cost(graph_q);
                estimated_cost += self.estimate_vector_search_cost(vector_q);
                expected_results = self.estimate_graph_result_size(graph_q) + vector_q.k;

                steps.push(ExecutionStep::CombineResults {
                    strategy: query.combination,
                    left_step: 0,
                    right_step: 1,
                });
            }

            // Weighted combination
            (Some(graph_q), Some(vector_q), CombinationStrategy::WeightedCombination { .. }) => {
                // Execute both queries
                steps.push(ExecutionStep::GraphTraversal {
                    query: graph_q.clone(),
                    estimated_nodes: self.estimate_graph_result_size(graph_q),
                });
                
                steps.push(ExecutionStep::VectorSearch {
                    query: vector_q.clone(),
                    estimated_results: vector_q.k,
                });

                estimated_cost += self.estimate_graph_traversal_cost(graph_q);
                estimated_cost += self.estimate_vector_search_cost(vector_q);
                expected_results = (self.estimate_graph_result_size(graph_q) + vector_q.k) / 2;

                steps.push(ExecutionStep::CombineResults {
                    strategy: query.combination,
                    left_step: 0,
                    right_step: 1,
                });
            }

            // Single query types
            (Some(graph_q), None, _) => {
                steps.push(ExecutionStep::GraphTraversal {
                    query: graph_q.clone(),
                    estimated_nodes: self.estimate_graph_result_size(graph_q),
                });
                estimated_cost += self.estimate_graph_traversal_cost(graph_q);
                expected_results = self.estimate_graph_result_size(graph_q);
            }

            (None, Some(vector_q), _) => {
                steps.push(ExecutionStep::VectorSearch {
                    query: vector_q.clone(),
                    estimated_results: vector_q.k,
                });
                estimated_cost += self.estimate_vector_search_cost(vector_q);
                expected_results = vector_q.k;
            }

            (None, None, _) => {
                return Err(VexGraphError::InvalidArgument(
                    "Query must contain at least one component".to_string()
                ));
            }
        }

        // Add sorting and limiting step
        steps.push(ExecutionStep::SortAndLimit {
            ordering: query.result_config.ordering,
            limit: query.result_config.limit,
            input_step: steps.len() - 1,
        });

        let memory_requirement_mb = (expected_results * 1024) / (1024 * 1024); // Rough estimate

        Ok(QueryPlan {
            steps,
            estimated_cost,
            expected_results,
            memory_requirement_mb,
        })
    }

    /// Create a basic execution plan without optimization
    async fn create_basic_plan(&self, query: &UnifiedSemanticQuery) -> VexGraphResult<QueryPlan> {
        // For simplicity, just create a straightforward plan
        self.create_optimized_plan(query).await
    }

    /// Execute a query plan
    async fn execute_plan(
        &self,
        plan: QueryPlan,
        query: &UnifiedSemanticQuery,
    ) -> VexGraphResult<Vec<UnifiedQueryResult>> {
        let mut step_results: Vec<IntermediateResult> = Vec::new();

        for (step_index, step) in plan.steps.iter().enumerate() {
            let step_start = std::time::Instant::now();
            
            let result = match step {
                ExecutionStep::GraphTraversal { query: graph_q, .. } => {
                    self.execute_graph_traversal(graph_q).await?
                }
                
                ExecutionStep::VectorSearch { query: vector_q, .. } => {
                    self.execute_vector_search(vector_q).await?
                }
                
                ExecutionStep::CombineResults { strategy, left_step, right_step } => {
                    let left_result = &step_results[*left_step];
                    let right_result = &step_results[*right_step];
                    self.combine_results(left_result, right_result, *strategy)?
                }
                
                ExecutionStep::FilterResults { filters, input_step } => {
                    let input_result = &step_results[*input_step];
                    self.filter_results(input_result, filters).await?
                }
                
                ExecutionStep::SortAndLimit { ordering, limit, input_step } => {
                    let input_result = &step_results[*input_step];
                    self.sort_and_limit_results(input_result, *ordering, *limit)?
                }
            };

            let step_time = step_start.elapsed().as_millis() as f64;
            tracing::debug!("Step {} completed in {:.2}ms", step_index, step_time);

            step_results.push(result);
        }

        // Convert final result to UnifiedQueryResult
        let final_result = step_results.into_iter().last()
            .ok_or_else(|| VexGraphError::InternalError("No execution results".to_string()))?;

        let mut unified_results = Vec::new();
        for scored_node in final_result.nodes {
            let unified_result = UnifiedQueryResult {
                node_id: scored_node.node_id,
                combined_score: scored_node.combined_score,
                similarity_score: scored_node.vector_score,
                graph_distance: scored_node.graph_distance,
                path_info: scored_node.path_info,
                node_metadata: if query.result_config.include_metadata {
                    Some(self.get_node_properties(scored_node.node_id).await?)
                } else {
                    None
                },
                vector_metadata: if query.result_config.include_metadata {
                    Some(scored_node.metadata)
                } else {
                    None
                },
                execution_metadata: final_result.metadata.clone(),
            };
            unified_results.push(unified_result);
        }

        Ok(unified_results)
    }

    /// Execute graph traversal
    async fn execute_graph_traversal(&self, query: &GraphQuery) -> VexGraphResult<IntermediateResult> {
        let start_time = std::time::Instant::now();
        let mut nodes_examined = 0;
        let mut edges_traversed = 0;

        let mut result_nodes = Vec::new();

        match query.algorithm {
            TraversalAlgorithm::BreadthFirstSearch => {
                result_nodes = self.execute_bfs(query, &mut nodes_examined, &mut edges_traversed).await?;
            }
            TraversalAlgorithm::DepthFirstSearch => {
                result_nodes = self.execute_dfs(query, &mut nodes_examined, &mut edges_traversed).await?;
            }
            TraversalAlgorithm::Dijkstra => {
                result_nodes = self.execute_dijkstra(query, &mut nodes_examined, &mut edges_traversed).await?;
            }
            _ => {
                return Err(VexGraphError::NotImplemented(
                    format!("Traversal algorithm {:?} not yet implemented", query.algorithm)
                ));
            }
        }

        let execution_time = start_time.elapsed().as_millis() as f64;

        let metadata = ExecutionMetadata {
            execution_time_ms: execution_time,
            from_cache: false,
            nodes_examined,
            edges_traversed,
            optimizations_applied: vec!["graph_traversal".to_string()],
        };

        Ok(IntermediateResult {
            nodes: result_nodes,
            metadata,
        })
    }

    /// Execute vector search
    async fn execute_vector_search(&self, query: &VectorQuery) -> VexGraphResult<IntermediateResult> {
        let start_time = std::time::Instant::now();

        // Convert to semantic search query
        let semantic_query = crate::vexgraph::semantic_search::SemanticQuery {
            query_vector: query.query_vector.clone(),
            k: query.k,
            embedding_type: query.embedding_type,
            graph_constraints: None,
            similarity_threshold: query.similarity_threshold,
            include_metadata: true,
        };

        let search_results = self.search_manager.semantic_search(semantic_query).await?;

        let mut scored_nodes = Vec::new();
        for result in search_results {
            let scored_node = ScoredNode {
                node_id: result.node_id,
                graph_score: None,
                vector_score: Some(result.similarity_score),
                combined_score: result.similarity_score,
                graph_distance: result.graph_distance,
                path_info: None,
                metadata: result.metadata,
            };
            scored_nodes.push(scored_node);
        }

        let execution_time = start_time.elapsed().as_millis() as f64;

        let metadata = ExecutionMetadata {
            execution_time_ms: execution_time,
            from_cache: false,
            nodes_examined: scored_nodes.len(),
            edges_traversed: 0,
            optimizations_applied: vec!["vector_search".to_string()],
        };

        Ok(IntermediateResult {
            nodes: scored_nodes,
            metadata,
        })
    }

    /// Execute breadth-first search
    async fn execute_bfs(
        &self,
        query: &GraphQuery,
        nodes_examined: &mut usize,
        edges_traversed: &mut usize,
    ) -> VexGraphResult<Vec<ScoredNode>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();

        // Initialize with start nodes
        for &start_node in &query.start_nodes {
            queue.push_back((start_node, 0));
            visited.insert(start_node);
        }

        while let Some((current_node, depth)) = queue.pop_front() {
            *nodes_examined += 1;

            // Check depth limit
            if let Some(max_depth) = query.max_depth {
                if depth >= max_depth {
                    continue;
                }
            }

            // Apply node filters
            if self.node_passes_filters(current_node, query).await? {
                let scored_node = ScoredNode {
                    node_id: current_node,
                    graph_score: Some(1.0 / (depth as f32 + 1.0)), // Distance-based score
                    vector_score: None,
                    combined_score: 1.0 / (depth as f32 + 1.0),
                    graph_distance: Some(depth),
                    path_info: None, // TODO: Track paths if needed
                    metadata: HashMap::new(),
                };
                results.push(scored_node);
            }

            // Explore neighbors
            if let Ok(outgoing_edges) = self.core.get_outgoing_edges(current_node).await {
                for edge_id in outgoing_edges {
                    if let Ok(edge) = self.core.get_edge(edge_id).await {
                        *edges_traversed += 1;
                        
                        if self.edge_passes_filters(&edge, query) && !visited.contains(&edge.target_id) {
                            visited.insert(edge.target_id);
                            queue.push_back((edge.target_id, depth + 1));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Execute depth-first search
    async fn execute_dfs(
        &self,
        query: &GraphQuery,
        nodes_examined: &mut usize,
        edges_traversed: &mut usize,
    ) -> VexGraphResult<Vec<ScoredNode>> {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut results = Vec::new();

        // Initialize with start nodes
        for &start_node in &query.start_nodes {
            stack.push((start_node, 0));
        }

        while let Some((current_node, depth)) = stack.pop() {
            if visited.contains(&current_node) {
                continue;
            }
            
            visited.insert(current_node);
            *nodes_examined += 1;

            // Check depth limit
            if let Some(max_depth) = query.max_depth {
                if depth >= max_depth {
                    continue;
                }
            }

            // Apply node filters
            if self.node_passes_filters(current_node, query).await? {
                let scored_node = ScoredNode {
                    node_id: current_node,
                    graph_score: Some(1.0 / (depth as f32 + 1.0)),
                    vector_score: None,
                    combined_score: 1.0 / (depth as f32 + 1.0),
                    graph_distance: Some(depth),
                    path_info: None,
                    metadata: HashMap::new(),
                };
                results.push(scored_node);
            }

            // Explore neighbors
            if let Ok(outgoing_edges) = self.core.get_outgoing_edges(current_node).await {
                for edge_id in outgoing_edges {
                    if let Ok(edge) = self.core.get_edge(edge_id).await {
                        *edges_traversed += 1;
                        
                        if self.edge_passes_filters(&edge, query) && !visited.contains(&edge.target_id) {
                            stack.push((edge.target_id, depth + 1));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Execute Dijkstra's algorithm
    async fn execute_dijkstra(
        &self,
        query: &GraphQuery,
        nodes_examined: &mut usize,
        edges_traversed: &mut usize,
    ) -> VexGraphResult<Vec<ScoredNode>> {
        let mut distances: HashMap<NodeId, f64> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut results = Vec::new();

        // Initialize with start nodes
        for &start_node in &query.start_nodes {
            distances.insert(start_node, 0.0);
            heap.push(std::cmp::Reverse((0.0 as u64, start_node))); // Use u64 for ordering
        }

        while let Some(std::cmp::Reverse((current_dist_bits, current_node))) = heap.pop() {
            let current_dist = f64::from_bits(current_dist_bits);
            *nodes_examined += 1;

            // Skip if we've found a better path
            if let Some(&best_dist) = distances.get(&current_node) {
                if current_dist > best_dist {
                    continue;
                }
            }

            // Apply node filters
            if self.node_passes_filters(current_node, query).await? {
                let scored_node = ScoredNode {
                    node_id: current_node,
                    graph_score: Some(1.0 / (current_dist + 1.0)),
                    vector_score: None,
                    combined_score: 1.0 / (current_dist + 1.0),
                    graph_distance: Some(current_dist as u32),
                    path_info: None,
                    metadata: HashMap::new(),
                };
                results.push(scored_node);
            }

            // Explore neighbors
            if let Ok(outgoing_edges) = self.core.get_outgoing_edges(current_node).await {
                for edge_id in outgoing_edges {
                    if let Ok(edge) = self.core.get_edge(edge_id).await {
                        *edges_traversed += 1;
                        
                        if self.edge_passes_filters(&edge, query) {
                            let new_dist = current_dist + edge.weight;
                            
                            if let Some(&existing_dist) = distances.get(&edge.target_id) {
                                if new_dist >= existing_dist {
                                    continue;
                                }
                            }
                            
                            distances.insert(edge.target_id, new_dist);
                            heap.push(std::cmp::Reverse((new_dist.to_bits(), edge.target_id)));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check if a node passes the query filters
    async fn node_passes_filters(&self, node_id: NodeId, query: &GraphQuery) -> VexGraphResult<bool> {
        let node = self.core.get_node(node_id).await?;

        // Check node type filters
        if let Some(node_filters) = &query.node_filters {
            for filter in node_filters {
                let matches = filter.node_types.contains(&node.node_type);
                if filter.include && !matches {
                    return Ok(false);
                }
                if !filter.include && matches {
                    return Ok(false);
                }
            }
        }

        // Check property filters
        if let Some(property_filters) = &query.property_filters {
            for filter in property_filters {
                if !self.property_passes_filter(&node.properties, filter)? {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Check if an edge passes the query filters
    fn edge_passes_filters(&self, edge: &GraphEdge, query: &GraphQuery) -> bool {
        // Check edge type filters
        if let Some(edge_filters) = &query.edge_filters {
            for filter in edge_filters {
                let matches = filter.edge_types.contains(&edge.edge_type);
                if filter.include && !matches {
                    return false;
                }
                if !filter.include && matches {
                    return false;
                }

                // Check weight range
                if let Some((min_weight, max_weight)) = filter.weight_range {
                    if edge.weight < min_weight || edge.weight > max_weight {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Check if a property passes a filter
    fn property_passes_filter(
        &self,
        properties: &HashMap<String, PropertyType>,
        filter: &crate::vexgraph::semantic_query_language::PropertyFilter,
    ) -> VexGraphResult<bool> {
        let property_value = match properties.get(&filter.key) {
            Some(value) => value,
            None => return Ok(false), // Property doesn't exist
        };

        match filter.operation {
            PropertyOperation::Equals => Ok(property_value == &filter.value),
            PropertyOperation::NotEquals => Ok(property_value != &filter.value),
            PropertyOperation::GreaterThan => {
                self.compare_properties(property_value, &filter.value).map(|ord| ord == Ordering::Greater)
            }
            PropertyOperation::LessThan => {
                self.compare_properties(property_value, &filter.value).map(|ord| ord == Ordering::Less)
            }
            PropertyOperation::GreaterThanOrEqual => {
                self.compare_properties(property_value, &filter.value).map(|ord| ord != Ordering::Less)
            }
            PropertyOperation::LessThanOrEqual => {
                self.compare_properties(property_value, &filter.value).map(|ord| ord != Ordering::Greater)
            }
            PropertyOperation::Contains => {
                match (property_value, &filter.value) {
                    (PropertyType::String(s1), PropertyType::String(s2)) => Ok(s1.contains(s2)),
                    _ => Ok(false),
                }
            }
            PropertyOperation::StartsWith => {
                match (property_value, &filter.value) {
                    (PropertyType::String(s1), PropertyType::String(s2)) => Ok(s1.starts_with(s2)),
                    _ => Ok(false),
                }
            }
            PropertyOperation::EndsWith => {
                match (property_value, &filter.value) {
                    (PropertyType::String(s1), PropertyType::String(s2)) => Ok(s1.ends_with(s2)),
                    _ => Ok(false),
                }
            }
            _ => Err(VexGraphError::NotImplemented(
                format!("Property operation {:?} not yet implemented", filter.operation)
            )),
        }
    }

    /// Compare two property values
    fn compare_properties(&self, a: &PropertyType, b: &PropertyType) -> VexGraphResult<Ordering> {
        match (a, b) {
            (PropertyType::Integer(a), PropertyType::Integer(b)) => Ok(a.cmp(b)),
            (PropertyType::Float(a), PropertyType::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(Ordering::Equal)),
            (PropertyType::String(a), PropertyType::String(b)) => Ok(a.cmp(b)),
            (PropertyType::Boolean(a), PropertyType::Boolean(b)) => Ok(a.cmp(b)),
            _ => Err(VexGraphError::InvalidArgument(
                "Cannot compare incompatible property types".to_string()
            )),
        }
    }

    /// Combine results from two execution steps
    fn combine_results(
        &self,
        left: &IntermediateResult,
        right: &IntermediateResult,
        strategy: CombinationStrategy,
    ) -> VexGraphResult<IntermediateResult> {
        let mut combined_nodes = Vec::new();
        let combined_metadata = ExecutionMetadata {
            execution_time_ms: left.metadata.execution_time_ms + right.metadata.execution_time_ms,
            from_cache: left.metadata.from_cache && right.metadata.from_cache,
            nodes_examined: left.metadata.nodes_examined + right.metadata.nodes_examined,
            edges_traversed: left.metadata.edges_traversed + right.metadata.edges_traversed,
            optimizations_applied: {
                let mut opts = left.metadata.optimizations_applied.clone();
                opts.extend(right.metadata.optimizations_applied.clone());
                opts.push("result_combination".to_string());
                opts
            },
        };

        match strategy {
            CombinationStrategy::Intersection => {
                let right_nodes: HashMap<NodeId, &ScoredNode> = right.nodes.iter()
                    .map(|node| (node.node_id, node))
                    .collect();

                for left_node in &left.nodes {
                    if let Some(right_node) = right_nodes.get(&left_node.node_id) {
                        let combined_score = (left_node.combined_score + right_node.combined_score) / 2.0;
                        let combined_node = ScoredNode {
                            node_id: left_node.node_id,
                            graph_score: left_node.graph_score.or(right_node.graph_score),
                            vector_score: left_node.vector_score.or(right_node.vector_score),
                            combined_score,
                            graph_distance: left_node.graph_distance.or(right_node.graph_distance),
                            path_info: left_node.path_info.clone().or_else(|| right_node.path_info.clone()),
                            metadata: {
                                let mut meta = left_node.metadata.clone();
                                meta.extend(right_node.metadata.clone());
                                meta
                            },
                        };
                        combined_nodes.push(combined_node);
                    }
                }
            }

            CombinationStrategy::Union => {
                let mut seen_nodes = HashSet::new();
                
                // Add all left nodes
                for node in &left.nodes {
                    combined_nodes.push(node.clone());
                    seen_nodes.insert(node.node_id);
                }
                
                // Add right nodes that aren't already present
                for node in &right.nodes {
                    if !seen_nodes.contains(&node.node_id) {
                        combined_nodes.push(node.clone());
                    }
                }
            }

            CombinationStrategy::GraphFirst => {
                // Use graph results as primary, enhance with vector scores
                let right_nodes: HashMap<NodeId, &ScoredNode> = right.nodes.iter()
                    .map(|node| (node.node_id, node))
                    .collect();

                for left_node in &left.nodes {
                    let mut combined_node = left_node.clone();
                    if let Some(right_node) = right_nodes.get(&left_node.node_id) {
                        combined_node.vector_score = right_node.vector_score;
                        combined_node.combined_score = left_node.combined_score * 0.7 +
                            right_node.combined_score * 0.3;
                    }
                    combined_nodes.push(combined_node);
                }
            }

            CombinationStrategy::VectorFirst => {
                // Use vector results as primary, enhance with graph scores
                let left_nodes: HashMap<NodeId, &ScoredNode> = left.nodes.iter()
                    .map(|node| (node.node_id, node))
                    .collect();

                for right_node in &right.nodes {
                    let mut combined_node = right_node.clone();
                    if let Some(left_node) = left_nodes.get(&right_node.node_id) {
                        combined_node.graph_score = left_node.graph_score;
                        combined_node.graph_distance = left_node.graph_distance;
                        combined_node.path_info = left_node.path_info.clone();
                        combined_node.combined_score = right_node.combined_score * 0.7 +
                            left_node.combined_score * 0.3;
                    }
                    combined_nodes.push(combined_node);
                }
            }

            CombinationStrategy::WeightedCombination { graph_weight, vector_weight } => {
                let right_nodes: HashMap<NodeId, &ScoredNode> = right.nodes.iter()
                    .map(|node| (node.node_id, node))
                    .collect();

                for left_node in &left.nodes {
                    if let Some(right_node) = right_nodes.get(&left_node.node_id) {
                        let combined_score = left_node.combined_score * graph_weight +
                            right_node.combined_score * vector_weight;
                        let combined_node = ScoredNode {
                            node_id: left_node.node_id,
                            graph_score: left_node.graph_score,
                            vector_score: right_node.vector_score,
                            combined_score,
                            graph_distance: left_node.graph_distance,
                            path_info: left_node.path_info.clone(),
                            metadata: {
                                let mut meta = left_node.metadata.clone();
                                meta.extend(right_node.metadata.clone());
                                meta
                            },
                        };
                        combined_nodes.push(combined_node);
                    }
                }
            }
        }

        Ok(IntermediateResult {
            nodes: combined_nodes,
            metadata: combined_metadata,
        })
    }

    /// Filter results based on criteria
    async fn filter_results(
        &self,
        input: &IntermediateResult,
        filters: &[ResultFilter],
    ) -> VexGraphResult<IntermediateResult> {
        let mut filtered_nodes = Vec::new();

        for node in &input.nodes {
            let mut passes_all_filters = true;

            for filter in filters {
                match filter {
                    ResultFilter::SimilarityThreshold(threshold) => {
                        if let Some(score) = node.vector_score {
                            if score < *threshold {
                                passes_all_filters = false;
                                break;
                            }
                        }
                    }
                    ResultFilter::GraphDistanceLimit(max_distance) => {
                        if let Some(distance) = node.graph_distance {
                            if distance > *max_distance {
                                passes_all_filters = false;
                                break;
                            }
                        }
                    }
                    ResultFilter::NodeTypeFilter(allowed_types) => {
                        let graph_node = self.core.get_node(node.node_id).await?;
                        if !allowed_types.contains(&graph_node.node_type) {
                            passes_all_filters = false;
                            break;
                        }
                    }
                    ResultFilter::PropertyFilter(key, operation, expected_value) => {
                        let graph_node = self.core.get_node(node.node_id).await?;
                        let filter = crate::vexgraph::semantic_query_language::PropertyFilter {
                            key: key.clone(),
                            operation: *operation,
                            value: expected_value.clone(),
                            additional_values: None,
                        };
                        if !self.property_passes_filter(&graph_node.properties, &filter)? {
                            passes_all_filters = false;
                            break;
                        }
                    }
                }
            }

            if passes_all_filters {
                filtered_nodes.push(node.clone());
            }
        }

        Ok(IntermediateResult {
            nodes: filtered_nodes,
            metadata: input.metadata.clone(),
        })
    }

    /// Sort and limit results
    fn sort_and_limit_results(
        &self,
        input: &IntermediateResult,
        ordering: ResultOrdering,
        limit: usize,
    ) -> VexGraphResult<IntermediateResult> {
        let mut sorted_nodes = input.nodes.clone();

        match ordering {
            ResultOrdering::SimilarityScore => {
                sorted_nodes.sort_by(|a, b| {
                    let a_score = a.vector_score.unwrap_or(0.0);
                    let b_score = b.vector_score.unwrap_or(0.0);
                    b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
                });
            }
            ResultOrdering::GraphDistance => {
                sorted_nodes.sort_by(|a, b| {
                    let a_dist = a.graph_distance.unwrap_or(u32::MAX);
                    let b_dist = b.graph_distance.unwrap_or(u32::MAX);
                    a_dist.cmp(&b_dist)
                });
            }
            ResultOrdering::CombinedScore => {
                sorted_nodes.sort_by(|a, b| {
                    b.combined_score.partial_cmp(&a.combined_score).unwrap_or(Ordering::Equal)
                });
            }
            _ => {
                // For other orderings, use combined score as default
                sorted_nodes.sort_by(|a, b| {
                    b.combined_score.partial_cmp(&a.combined_score).unwrap_or(Ordering::Equal)
                });
            }
        }

        sorted_nodes.truncate(limit);

        Ok(IntermediateResult {
            nodes: sorted_nodes,
            metadata: input.metadata.clone(),
        })
    }

    /// Get node properties
    async fn get_node_properties(&self, node_id: NodeId) -> VexGraphResult<HashMap<String, PropertyType>> {
        let node = self.core.get_node(node_id).await?;
        Ok(node.properties)
    }

    /// Estimate cost functions for query optimization
    fn estimate_vector_search_cost(&self, _query: &VectorQuery) -> f64 {
        // Simplified cost estimation
        100.0 // Base cost for vector search
    }

    fn estimate_graph_traversal_cost(&self, query: &GraphQuery) -> f64 {
        // Simplified cost estimation based on start nodes and depth
        let base_cost = query.start_nodes.len() as f64 * 10.0;
        let depth_multiplier = query.max_depth.unwrap_or(5) as f64;
        base_cost * depth_multiplier
    }

    fn estimate_graph_result_size(&self, query: &GraphQuery) -> usize {
        // Simplified estimation
        let base_size = query.start_nodes.len() * 10;
        let depth_multiplier = query.max_depth.unwrap_or(3) as usize;
        base_size * depth_multiplier
    }

    /// Update execution statistics
    fn update_statistics(&self, execution_time_ms: f64, result_count: usize) {
        let mut stats = self.stats.write();
        stats.total_queries_executed += 1;
        
        // Update average execution time
        let total_time = stats.average_execution_time_ms * (stats.total_queries_executed - 1) as f64 + execution_time_ms;
        stats.average_execution_time_ms = total_time / stats.total_queries_executed as f64;
        
        stats.last_updated = chrono::Utc::now();
        
        tracing::debug!(
            "Updated executor statistics: {} queries, {:.2}ms avg, {} results",
            stats.total_queries_executed,
            stats.average_execution_time_ms,
            result_count
        );
    }

    /// Get executor statistics
    pub async fn get_statistics(&self) -> ExecutorStatistics {
        self.stats.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vexgraph::{NodeType, VexGraphConfig};

    #[tokio::test]
    async fn test_query_executor_creation() {
        let config = VexGraphConfig::default();
        let core = Arc::new(VexGraphCore::new(&config).await.unwrap());
        let search_config = SemanticSearchConfig::default();
        let optimization_config = QueryOptimizationConfig::default();
        
        let executor = SemanticQueryExecutor::new(core, search_config, optimization_config).await.unwrap();
        let stats = executor.get_statistics().await;
        assert_eq!(stats.total_queries_executed, 0);
    }
}