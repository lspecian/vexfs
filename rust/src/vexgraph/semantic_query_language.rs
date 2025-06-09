/*
 * VexFS v2.0 - VexGraph Semantic Query Language (Task 11)
 * 
 * This module implements a unified query language that combines graph traversal
 * with vector similarity search operations.
 */

use crate::vexgraph::{
    NodeId, EdgeId, PropertyType, NodeType, EdgeType, TraversalAlgorithm,
    error_handling::{VexGraphError, VexGraphResult},
    semantic_search::{EmbeddingType, SemanticSearchResult},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified semantic query that combines graph and vector operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSemanticQuery {
    /// Graph traversal component
    pub graph_query: Option<GraphQuery>,
    
    /// Vector similarity component
    pub vector_query: Option<VectorQuery>,
    
    /// Combination strategy
    pub combination: CombinationStrategy,
    
    /// Result limits and ordering
    pub result_config: ResultConfiguration,
    
    /// Query metadata
    pub metadata: QueryMetadata,
}

/// Graph traversal query specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Starting nodes for traversal
    pub start_nodes: Vec<NodeId>,
    
    /// Traversal algorithm to use
    pub algorithm: TraversalAlgorithm,
    
    /// Maximum traversal depth
    pub max_depth: Option<u32>,
    
    /// Edge type filters
    pub edge_filters: Option<Vec<EdgeTypeFilter>>,
    
    /// Node type filters
    pub node_filters: Option<Vec<NodeTypeFilter>>,
    
    /// Property-based filters
    pub property_filters: Option<Vec<PropertyFilter>>,
    
    /// Path constraints
    pub path_constraints: Option<PathConstraints>,
}

/// Vector similarity query specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQuery {
    /// Query vector for similarity search
    pub query_vector: Vec<f32>,
    
    /// Number of similar vectors to find
    pub k: usize,
    
    /// Embedding type filter
    pub embedding_type: Option<EmbeddingType>,
    
    /// Similarity threshold
    pub similarity_threshold: Option<f32>,
    
    /// Distance metric
    pub distance_metric: DistanceMetric,
    
    /// Vector-specific filters
    pub vector_filters: Option<Vec<VectorFilter>>,
}

/// Strategy for combining graph and vector query results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombinationStrategy {
    /// Intersect results (nodes must satisfy both graph and vector criteria)
    Intersection,
    
    /// Union results (nodes satisfy either graph or vector criteria)
    Union,
    
    /// Graph-first (use graph results, then rank by vector similarity)
    GraphFirst,
    
    /// Vector-first (use vector results, then filter by graph criteria)
    VectorFirst,
    
    /// Weighted combination (combine scores from both queries)
    WeightedCombination { graph_weight: f32, vector_weight: f32 },
}

/// Result configuration and ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultConfiguration {
    /// Maximum number of results to return
    pub limit: usize,
    
    /// Result ordering strategy
    pub ordering: ResultOrdering,
    
    /// Include detailed metadata in results
    pub include_metadata: bool,
    
    /// Include graph path information
    pub include_paths: bool,
    
    /// Include vector similarity scores
    pub include_similarity_scores: bool,
}

/// Result ordering strategies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultOrdering {
    /// Order by vector similarity score (descending)
    SimilarityScore,
    
    /// Order by graph distance (ascending)
    GraphDistance,
    
    /// Order by combined score
    CombinedScore,
    
    /// Order by node creation time
    CreationTime,
    
    /// Order by node update time
    UpdateTime,
    
    /// Custom ordering by property
    PropertyValue(String),
}

/// Query metadata and execution hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query identifier
    pub query_id: String,
    
    /// Query description
    pub description: Option<String>,
    
    /// Execution timeout in milliseconds
    pub timeout_ms: Option<u64>,
    
    /// Enable query optimization
    pub optimize: bool,
    
    /// Enable result caching
    pub cache_results: bool,
    
    /// Query tags for categorization
    pub tags: Vec<String>,
}

/// Edge type filter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTypeFilter {
    /// Edge types to include/exclude
    pub edge_types: Vec<EdgeType>,
    
    /// Whether this is an inclusion or exclusion filter
    pub include: bool,
    
    /// Weight range filter
    pub weight_range: Option<(f64, f64)>,
}

/// Node type filter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeFilter {
    /// Node types to include/exclude
    pub node_types: Vec<NodeType>,
    
    /// Whether this is an inclusion or exclusion filter
    pub include: bool,
}

/// Property-based filter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyFilter {
    /// Property key to filter on
    pub key: String,
    
    /// Filter operation
    pub operation: PropertyOperation,
    
    /// Expected value(s)
    pub value: PropertyType,
    
    /// Additional values for range operations
    pub additional_values: Option<Vec<PropertyType>>,
}

/// Property filter operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyOperation {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    In,
    NotIn,
    Range,
}

/// Path constraints for graph traversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConstraints {
    /// Minimum path length
    pub min_length: Option<u32>,
    
    /// Maximum path length
    pub max_length: Option<u32>,
    
    /// Required edge sequence patterns
    pub edge_patterns: Option<Vec<Vec<EdgeType>>>,
    
    /// Forbidden edge sequences
    pub forbidden_patterns: Option<Vec<Vec<EdgeType>>>,
    
    /// Path uniqueness requirements
    pub uniqueness: PathUniqueness,
}

/// Path uniqueness requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathUniqueness {
    /// No uniqueness requirement
    None,
    
    /// Nodes must be unique in path
    UniqueNodes,
    
    /// Edges must be unique in path
    UniqueEdges,
    
    /// Both nodes and edges must be unique
    UniqueBoth,
}

/// Vector-specific filter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilter {
    /// Dimension range to consider
    pub dimension_range: Option<(usize, usize)>,
    
    /// Value range for vector components
    pub value_range: Option<(f32, f32)>,
    
    /// Metadata filters
    pub metadata_filters: Option<Vec<PropertyFilter>>,
}

/// Distance metrics for vector similarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    /// Euclidean (L2) distance
    Euclidean,
    
    /// Cosine similarity
    Cosine,
    
    /// Manhattan (L1) distance
    Manhattan,
    
    /// Dot product similarity
    DotProduct,
    
    /// Hamming distance (for binary vectors)
    Hamming,
}

/// Combined query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedQueryResult {
    /// Node identifier
    pub node_id: NodeId,
    
    /// Combined score
    pub combined_score: f32,
    
    /// Vector similarity score (if applicable)
    pub similarity_score: Option<f32>,
    
    /// Graph distance (if applicable)
    pub graph_distance: Option<u32>,
    
    /// Path information (if requested)
    pub path_info: Option<PathInfo>,
    
    /// Node metadata (if requested)
    pub node_metadata: Option<HashMap<String, PropertyType>>,
    
    /// Vector metadata (if applicable)
    pub vector_metadata: Option<HashMap<String, String>>,
    
    /// Query execution metadata
    pub execution_metadata: ExecutionMetadata,
}

/// Path information for graph traversal results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Sequence of nodes in the path
    pub node_path: Vec<NodeId>,
    
    /// Sequence of edges in the path
    pub edge_path: Vec<EdgeId>,
    
    /// Path length
    pub length: u32,
    
    /// Total path weight
    pub total_weight: f64,
}

/// Execution metadata for query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution time in milliseconds
    pub execution_time_ms: f64,
    
    /// Whether result was cached
    pub from_cache: bool,
    
    /// Number of nodes examined
    pub nodes_examined: usize,
    
    /// Number of edges traversed
    pub edges_traversed: usize,
    
    /// Query optimization applied
    pub optimizations_applied: Vec<String>,
}

/// Query builder for constructing unified semantic queries
#[derive(Debug, Default)]
pub struct UnifiedQueryBuilder {
    graph_query: Option<GraphQuery>,
    vector_query: Option<VectorQuery>,
    combination: Option<CombinationStrategy>,
    result_config: Option<ResultConfiguration>,
    metadata: Option<QueryMetadata>,
}

impl UnifiedQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add graph traversal component
    pub fn with_graph_query(mut self, graph_query: GraphQuery) -> Self {
        self.graph_query = Some(graph_query);
        self
    }

    /// Add vector similarity component
    pub fn with_vector_query(mut self, vector_query: VectorQuery) -> Self {
        self.vector_query = Some(vector_query);
        self
    }

    /// Set combination strategy
    pub fn with_combination(mut self, combination: CombinationStrategy) -> Self {
        self.combination = Some(combination);
        self
    }

    /// Set result configuration
    pub fn with_result_config(mut self, config: ResultConfiguration) -> Self {
        self.result_config = Some(config);
        self
    }

    /// Set query metadata
    pub fn with_metadata(mut self, metadata: QueryMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the unified query
    pub fn build(self) -> VexGraphResult<UnifiedSemanticQuery> {
        // Validate that at least one query component is provided
        if self.graph_query.is_none() && self.vector_query.is_none() {
            return Err(VexGraphError::InvalidArgument(
                "At least one of graph_query or vector_query must be provided".to_string()
            ));
        }

        // Set default combination strategy
        let combination = self.combination.unwrap_or_else(|| {
            match (&self.graph_query, &self.vector_query) {
                (Some(_), Some(_)) => CombinationStrategy::Intersection,
                (Some(_), None) => CombinationStrategy::GraphFirst,
                (None, Some(_)) => CombinationStrategy::VectorFirst,
                (None, None) => unreachable!(), // Already checked above
            }
        });

        // Set default result configuration
        let result_config = self.result_config.unwrap_or_else(|| ResultConfiguration {
            limit: 100,
            ordering: ResultOrdering::CombinedScore,
            include_metadata: false,
            include_paths: false,
            include_similarity_scores: true,
        });

        // Set default metadata
        let metadata = self.metadata.unwrap_or_else(|| QueryMetadata {
            query_id: uuid::Uuid::new_v4().to_string(),
            description: None,
            timeout_ms: Some(30000), // 30 seconds
            optimize: true,
            cache_results: true,
            tags: Vec::new(),
        });

        Ok(UnifiedSemanticQuery {
            graph_query: self.graph_query,
            vector_query: self.vector_query,
            combination,
            result_config,
            metadata,
        })
    }
}

/// Graph query builder for constructing graph traversal queries
#[derive(Debug, Default)]
pub struct GraphQueryBuilder {
    start_nodes: Vec<NodeId>,
    algorithm: Option<TraversalAlgorithm>,
    max_depth: Option<u32>,
    edge_filters: Vec<EdgeTypeFilter>,
    node_filters: Vec<NodeTypeFilter>,
    property_filters: Vec<PropertyFilter>,
    path_constraints: Option<PathConstraints>,
}

impl GraphQueryBuilder {
    /// Create a new graph query builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set starting nodes
    pub fn start_from(mut self, nodes: Vec<NodeId>) -> Self {
        self.start_nodes = nodes;
        self
    }

    /// Set traversal algorithm
    pub fn algorithm(mut self, algorithm: TraversalAlgorithm) -> Self {
        self.algorithm = Some(algorithm);
        self
    }

    /// Set maximum depth
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Add edge type filter
    pub fn filter_edges(mut self, edge_types: Vec<EdgeType>, include: bool) -> Self {
        self.edge_filters.push(EdgeTypeFilter {
            edge_types,
            include,
            weight_range: None,
        });
        self
    }

    /// Add node type filter
    pub fn filter_nodes(mut self, node_types: Vec<NodeType>, include: bool) -> Self {
        self.node_filters.push(NodeTypeFilter {
            node_types,
            include,
        });
        self
    }

    /// Add property filter
    pub fn filter_property(
        mut self,
        key: String,
        operation: PropertyOperation,
        value: PropertyType,
    ) -> Self {
        self.property_filters.push(PropertyFilter {
            key,
            operation,
            value,
            additional_values: None,
        });
        self
    }

    /// Set path constraints
    pub fn with_path_constraints(mut self, constraints: PathConstraints) -> Self {
        self.path_constraints = Some(constraints);
        self
    }

    /// Build the graph query
    pub fn build(self) -> VexGraphResult<GraphQuery> {
        if self.start_nodes.is_empty() {
            return Err(VexGraphError::InvalidArgument(
                "At least one starting node must be provided".to_string()
            ));
        }

        let algorithm = self.algorithm.unwrap_or(TraversalAlgorithm::BreadthFirstSearch);

        Ok(GraphQuery {
            start_nodes: self.start_nodes,
            algorithm,
            max_depth: self.max_depth,
            edge_filters: if self.edge_filters.is_empty() { None } else { Some(self.edge_filters) },
            node_filters: if self.node_filters.is_empty() { None } else { Some(self.node_filters) },
            property_filters: if self.property_filters.is_empty() { None } else { Some(self.property_filters) },
            path_constraints: self.path_constraints,
        })
    }
}

/// Vector query builder for constructing vector similarity queries
#[derive(Debug)]
pub struct VectorQueryBuilder {
    query_vector: Option<Vec<f32>>,
    k: usize,
    embedding_type: Option<EmbeddingType>,
    similarity_threshold: Option<f32>,
    distance_metric: DistanceMetric,
    vector_filters: Vec<VectorFilter>,
}

impl Default for VectorQueryBuilder {
    fn default() -> Self {
        Self {
            query_vector: None,
            k: 10,
            embedding_type: None,
            similarity_threshold: None,
            distance_metric: DistanceMetric::Cosine,
            vector_filters: Vec::new(),
        }
    }
}

impl VectorQueryBuilder {
    /// Create a new vector query builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set query vector
    pub fn query_vector(mut self, vector: Vec<f32>) -> Self {
        self.query_vector = Some(vector);
        self
    }

    /// Set number of results
    pub fn k(mut self, k: usize) -> Self {
        self.k = k;
        self
    }

    /// Set embedding type filter
    pub fn embedding_type(mut self, embedding_type: EmbeddingType) -> Self {
        self.embedding_type = Some(embedding_type);
        self
    }

    /// Set similarity threshold
    pub fn similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = Some(threshold);
        self
    }

    /// Set distance metric
    pub fn distance_metric(mut self, metric: DistanceMetric) -> Self {
        self.distance_metric = metric;
        self
    }

    /// Add vector filter
    pub fn add_filter(mut self, filter: VectorFilter) -> Self {
        self.vector_filters.push(filter);
        self
    }

    /// Build the vector query
    pub fn build(self) -> VexGraphResult<VectorQuery> {
        let query_vector = self.query_vector.ok_or_else(|| {
            VexGraphError::InvalidArgument("Query vector must be provided".to_string())
        })?;

        if query_vector.is_empty() {
            return Err(VexGraphError::InvalidArgument(
                "Query vector cannot be empty".to_string()
            ));
        }

        Ok(VectorQuery {
            query_vector,
            k: self.k,
            embedding_type: self.embedding_type,
            similarity_threshold: self.similarity_threshold,
            distance_metric: self.distance_metric,
            vector_filters: if self.vector_filters.is_empty() { None } else { Some(self.vector_filters) },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_query_builder() {
        let graph_query = GraphQueryBuilder::new()
            .start_from(vec![1, 2, 3])
            .algorithm(TraversalAlgorithm::BreadthFirstSearch)
            .max_depth(5)
            .build()
            .unwrap();

        let vector_query = VectorQueryBuilder::new()
            .query_vector(vec![1.0, 2.0, 3.0, 4.0])
            .k(10)
            .embedding_type(EmbeddingType::Text)
            .build()
            .unwrap();

        let unified_query = UnifiedQueryBuilder::new()
            .with_graph_query(graph_query)
            .with_vector_query(vector_query)
            .with_combination(CombinationStrategy::Intersection)
            .build()
            .unwrap();

        assert!(unified_query.graph_query.is_some());
        assert!(unified_query.vector_query.is_some());
        assert_eq!(unified_query.combination, CombinationStrategy::Intersection);
    }

    #[test]
    fn test_graph_query_builder() {
        let query = GraphQueryBuilder::new()
            .start_from(vec![1, 2])
            .algorithm(TraversalAlgorithm::DepthFirstSearch)
            .max_depth(3)
            .filter_edges(vec![EdgeType::Contains], true)
            .filter_nodes(vec![NodeType::File], true)
            .build()
            .unwrap();

        assert_eq!(query.start_nodes, vec![1, 2]);
        assert_eq!(query.algorithm, TraversalAlgorithm::DepthFirstSearch);
        assert_eq!(query.max_depth, Some(3));
        assert!(query.edge_filters.is_some());
        assert!(query.node_filters.is_some());
    }

    #[test]
    fn test_vector_query_builder() {
        let query = VectorQueryBuilder::new()
            .query_vector(vec![1.0, 2.0, 3.0])
            .k(5)
            .embedding_type(EmbeddingType::Image)
            .similarity_threshold(0.8)
            .distance_metric(DistanceMetric::Euclidean)
            .build()
            .unwrap();

        assert_eq!(query.query_vector, vec![1.0, 2.0, 3.0]);
        assert_eq!(query.k, 5);
        assert_eq!(query.embedding_type, Some(EmbeddingType::Image));
        assert_eq!(query.similarity_threshold, Some(0.8));
        assert_eq!(query.distance_metric, DistanceMetric::Euclidean);
    }

    #[test]
    fn test_query_validation() {
        // Test empty query
        let result = UnifiedQueryBuilder::new().build();
        assert!(result.is_err());

        // Test empty start nodes
        let result = GraphQueryBuilder::new().build();
        assert!(result.is_err());

        // Test empty query vector
        let result = VectorQueryBuilder::new().build();
        assert!(result.is_err());
    }
}