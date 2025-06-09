//! Graph-Journal Integration Manager
//! 
//! This module implements the core integration between HNSW graph operations and the
//! userspace semantic journal system, providing AI-native capabilities for graph
//! analytics and semantic reasoning in FUSE context.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use crate::semantic_api::userspace_journal::{
    UserspaceSemanticJournal, UserspaceJournalConfig, JournalEventStream, StreamMessage
};
use crate::semantic_api::fuse_journal_integration::{FuseJournalIntegration, FuseJournalConfig};
use crate::vector_storage::VectorStorageManager;
use crate::anns::hnsw_optimized::OptimizedHnswGraph;
use crate::shared::errors::{VexfsError, VexfsResult};

use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Maximum stack usage for graph-journal operations (6KB limit)
const GRAPH_JOURNAL_MAX_STACK_USAGE: usize = 6144;

/// Default batch size for graph event processing
const DEFAULT_GRAPH_BATCH_SIZE: usize = 50;

/// Default analytics window size in seconds
const DEFAULT_ANALYTICS_WINDOW: u64 = 300; // 5 minutes

/// Graph-Journal Integration Manager
/// 
/// Core component that bridges HNSW graph operations with the semantic journal system,
/// providing real-time analytics, semantic reasoning, and AI-native capabilities.
pub struct GraphJournalIntegrationManager {
    /// Userspace semantic journal
    journal: Arc<UserspaceSemanticJournal>,
    /// FUSE journal integration
    fuse_integration: Arc<FuseJournalIntegration>,
    /// Vector storage manager
    vector_storage: Arc<VectorStorageManager>,
    /// Optimized HNSW graph
    hnsw_graph: Arc<Mutex<OptimizedHnswGraph>>,
    /// Graph analytics engine
    analytics_engine: Arc<RwLock<GraphAnalyticsEngine>>,
    /// Performance metrics collector
    metrics: Arc<RwLock<GraphPerformanceMetrics>>,
    /// Configuration
    config: GraphJournalConfig,
    /// Event correlation tracker
    correlation_tracker: Arc<RwLock<EventCorrelationTracker>>,
    /// Semantic reasoning engine
    reasoning_engine: Arc<RwLock<SemanticReasoningEngine>>,
}

/// Configuration for graph-journal integration
#[derive(Debug, Clone)]
pub struct GraphJournalConfig {
    /// Enable automatic graph event journaling
    pub auto_journal_graph_events: bool,
    /// Enable real-time graph analytics
    pub enable_real_time_analytics: bool,
    /// Enable semantic reasoning
    pub enable_semantic_reasoning: bool,
    /// Batch size for graph event processing
    pub graph_batch_size: usize,
    /// Analytics window size in seconds
    pub analytics_window_seconds: u64,
    /// Maximum correlation depth
    pub max_correlation_depth: usize,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Analytics options
    pub analytics_options: AnalyticsOptions,
}

impl Default for GraphJournalConfig {
    fn default() -> Self {
        Self {
            auto_journal_graph_events: true,
            enable_real_time_analytics: true,
            enable_semantic_reasoning: true,
            graph_batch_size: DEFAULT_GRAPH_BATCH_SIZE,
            analytics_window_seconds: DEFAULT_ANALYTICS_WINDOW,
            max_correlation_depth: 5,
            enable_performance_monitoring: true,
            analytics_options: AnalyticsOptions::default(),
        }
    }
}

/// Analytics configuration options
#[derive(Debug, Clone)]
pub struct AnalyticsOptions {
    /// Enable centrality measures
    pub enable_centrality_measures: bool,
    /// Enable pathfinding analytics
    pub enable_pathfinding_analytics: bool,
    /// Enable clustering analytics
    pub enable_clustering_analytics: bool,
    /// Enable graph health monitoring
    pub enable_health_monitoring: bool,
    /// Centrality calculation interval in seconds
    pub centrality_interval_seconds: u64,
    /// Clustering update interval in seconds
    pub clustering_interval_seconds: u64,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
}

impl Default for AnalyticsOptions {
    fn default() -> Self {
        Self {
            enable_centrality_measures: true,
            enable_pathfinding_analytics: true,
            enable_clustering_analytics: true,
            enable_health_monitoring: true,
            centrality_interval_seconds: 60,
            clustering_interval_seconds: 120,
            health_check_interval_seconds: 30,
        }
    }
}

/// Graph performance metrics
#[derive(Debug, Clone, Default)]
pub struct GraphPerformanceMetrics {
    /// Total graph operations processed
    pub total_operations: u64,
    /// Graph search operations
    pub search_operations: u64,
    /// Graph insertion operations
    pub insertion_operations: u64,
    /// Graph deletion operations
    pub deletion_operations: u64,
    /// Graph update operations
    pub update_operations: u64,
    /// Average search latency in microseconds
    pub avg_search_latency_us: f64,
    /// Average insertion latency in microseconds
    pub avg_insertion_latency_us: f64,
    /// Current graph size (nodes)
    pub current_graph_size: u64,
    /// Current graph edges
    pub current_graph_edges: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Error rate
    pub error_rate: f64,
    /// Last update timestamp
    pub last_update: SystemTime,
}

/// Graph analytics engine for real-time analysis
#[derive(Debug)]
pub struct GraphAnalyticsEngine {
    /// Centrality measures cache
    centrality_cache: BTreeMap<u64, CentralityMeasures>,
    /// Clustering results cache
    clustering_cache: Option<ClusteringResults>,
    /// Graph health metrics
    health_metrics: GraphHealthMetrics,
    /// Analytics history
    analytics_history: VecDeque<AnalyticsSnapshot>,
    /// Last analytics update
    last_update: SystemTime,
}

/// Centrality measures for a node
#[derive(Debug, Clone)]
pub struct CentralityMeasures {
    /// Degree centrality
    pub degree_centrality: f64,
    /// Betweenness centrality
    pub betweenness_centrality: f64,
    /// PageRank score
    pub pagerank_score: f64,
    /// Eigenvector centrality
    pub eigenvector_centrality: f64,
    /// Last calculated timestamp
    pub calculated_at: SystemTime,
}

/// Clustering analysis results
#[derive(Debug, Clone)]
pub struct ClusteringResults {
    /// Number of clusters detected
    pub num_clusters: usize,
    /// Cluster assignments (node_id -> cluster_id)
    pub cluster_assignments: HashMap<u64, usize>,
    /// Cluster centroids
    pub cluster_centroids: Vec<Vec<f32>>,
    /// Silhouette scores
    pub silhouette_scores: Vec<f64>,
    /// Overall silhouette score
    pub overall_silhouette_score: f64,
    /// Calculated timestamp
    pub calculated_at: SystemTime,
}

/// Graph health monitoring metrics
#[derive(Debug, Clone, Default)]
pub struct GraphHealthMetrics {
    /// Graph connectivity score (0.0 to 1.0)
    pub connectivity_score: f64,
    /// Average path length
    pub avg_path_length: f64,
    /// Clustering coefficient
    pub clustering_coefficient: f64,
    /// Graph density
    pub graph_density: f64,
    /// Number of disconnected components
    pub disconnected_components: usize,
    /// Quality score (0.0 to 1.0)
    pub quality_score: f64,
    /// Last health check
    pub last_health_check: SystemTime,
}

/// Analytics snapshot for historical tracking
#[derive(Debug, Clone)]
pub struct AnalyticsSnapshot {
    /// Snapshot timestamp
    pub timestamp: SystemTime,
    /// Graph size at snapshot
    pub graph_size: u64,
    /// Performance metrics snapshot
    pub performance_metrics: GraphPerformanceMetrics,
    /// Health metrics snapshot
    pub health_metrics: GraphHealthMetrics,
    /// Top centrality nodes (top 10)
    pub top_centrality_nodes: Vec<(u64, CentralityMeasures)>,
}

/// Event correlation tracker for semantic reasoning
#[derive(Debug)]
pub struct EventCorrelationTracker {
    /// Active correlations (event_id -> correlation_info)
    correlations: HashMap<Uuid, CorrelationInfo>,
    /// Correlation patterns
    patterns: Vec<CorrelationPattern>,
    /// Correlation history
    history: VecDeque<CorrelationEvent>,
    /// Maximum history size
    max_history_size: usize,
}

/// Correlation information for an event
#[derive(Debug, Clone)]
pub struct CorrelationInfo {
    /// Original event ID
    pub event_id: Uuid,
    /// Related events
    pub related_events: Vec<Uuid>,
    /// Correlation strength (0.0 to 1.0)
    pub correlation_strength: f64,
    /// Correlation type
    pub correlation_type: CorrelationType,
    /// Created timestamp
    pub created_at: SystemTime,
}

/// Types of event correlations
#[derive(Debug, Clone, PartialEq)]
pub enum CorrelationType {
    /// Causal relationship (A causes B)
    Causal,
    /// Temporal relationship (A and B happen together)
    Temporal,
    /// Semantic relationship (A and B are semantically related)
    Semantic,
    /// Spatial relationship (A and B affect same graph region)
    Spatial,
}

/// Correlation pattern for pattern recognition
#[derive(Debug, Clone)]
pub struct CorrelationPattern {
    /// Pattern ID
    pub pattern_id: Uuid,
    /// Event sequence pattern
    pub event_sequence: Vec<SemanticEventType>,
    /// Pattern confidence (0.0 to 1.0)
    pub confidence: f64,
    /// Pattern frequency
    pub frequency: u64,
    /// Last seen timestamp
    pub last_seen: SystemTime,
}

/// Correlation event for history tracking
#[derive(Debug, Clone)]
pub struct CorrelationEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Primary event
    pub primary_event: Uuid,
    /// Correlated events
    pub correlated_events: Vec<Uuid>,
    /// Correlation type
    pub correlation_type: CorrelationType,
    /// Correlation strength
    pub strength: f64,
}

/// Semantic reasoning engine for AI-native capabilities
#[derive(Debug)]
pub struct SemanticReasoningEngine {
    /// Knowledge graph for semantic relationships
    knowledge_graph: SemanticKnowledgeGraph,
    /// Reasoning rules
    reasoning_rules: Vec<ReasoningRule>,
    /// Inference cache
    inference_cache: HashMap<String, InferenceResult>,
    /// Reasoning history
    reasoning_history: VecDeque<ReasoningEvent>,
}

/// Semantic knowledge graph for reasoning
#[derive(Debug)]
pub struct SemanticKnowledgeGraph {
    /// Semantic nodes (concept_id -> concept_info)
    nodes: HashMap<String, SemanticConcept>,
    /// Semantic edges (relationship_id -> relationship_info)
    edges: HashMap<String, SemanticRelationship>,
    /// Concept hierarchy
    hierarchy: BTreeMap<String, Vec<String>>,
}

/// Semantic concept in the knowledge graph
#[derive(Debug, Clone)]
pub struct SemanticConcept {
    /// Concept ID
    pub concept_id: String,
    /// Concept name
    pub name: String,
    /// Concept description
    pub description: String,
    /// Concept properties
    pub properties: HashMap<String, String>,
    /// Related graph nodes
    pub graph_nodes: Vec<u64>,
    /// Confidence score
    pub confidence: f64,
}

/// Semantic relationship between concepts
#[derive(Debug, Clone)]
pub struct SemanticRelationship {
    /// Relationship ID
    pub relationship_id: String,
    /// Source concept
    pub source_concept: String,
    /// Target concept
    pub target_concept: String,
    /// Relationship type
    pub relationship_type: String,
    /// Relationship strength
    pub strength: f64,
    /// Properties
    pub properties: HashMap<String, String>,
}

/// Reasoning rule for inference
#[derive(Debug, Clone)]
pub struct ReasoningRule {
    /// Rule ID
    pub rule_id: String,
    /// Rule name
    pub name: String,
    /// Rule conditions
    pub conditions: Vec<RuleCondition>,
    /// Rule conclusions
    pub conclusions: Vec<RuleConclusion>,
    /// Rule confidence
    pub confidence: f64,
}

/// Condition in a reasoning rule
#[derive(Debug, Clone)]
pub struct RuleCondition {
    /// Condition type
    pub condition_type: String,
    /// Condition parameters
    pub parameters: HashMap<String, String>,
}

/// Conclusion in a reasoning rule
#[derive(Debug, Clone)]
pub struct RuleConclusion {
    /// Conclusion type
    pub conclusion_type: String,
    /// Conclusion parameters
    pub parameters: HashMap<String, String>,
    /// Conclusion confidence
    pub confidence: f64,
}

/// Result of inference operation
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Inferred facts
    pub facts: Vec<InferredFact>,
    /// Inference confidence
    pub confidence: f64,
    /// Reasoning path
    pub reasoning_path: Vec<String>,
    /// Inference timestamp
    pub inferred_at: SystemTime,
}

/// Inferred fact from reasoning
#[derive(Debug, Clone)]
pub struct InferredFact {
    /// Fact statement
    pub statement: String,
    /// Fact confidence
    pub confidence: f64,
    /// Supporting evidence
    pub evidence: Vec<String>,
}

/// Reasoning event for history tracking
#[derive(Debug, Clone)]
pub struct ReasoningEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Reasoning trigger
    pub trigger: String,
    /// Inference result
    pub result: InferenceResult,
    /// Processing time in microseconds
    pub processing_time_us: u64,
}

impl GraphJournalIntegrationManager {
    /// Create a new graph-journal integration manager
    pub fn new(
        journal: Arc<UserspaceSemanticJournal>,
        fuse_integration: Arc<FuseJournalIntegration>,
        vector_storage: Arc<VectorStorageManager>,
        hnsw_graph: Arc<Mutex<OptimizedHnswGraph>>,
        config: GraphJournalConfig,
    ) -> SemanticResult<Self> {
        let analytics_engine = Arc::new(RwLock::new(GraphAnalyticsEngine::new()));
        let metrics = Arc::new(RwLock::new(GraphPerformanceMetrics::default()));
        let correlation_tracker = Arc::new(RwLock::new(EventCorrelationTracker::new()));
        let reasoning_engine = Arc::new(RwLock::new(SemanticReasoningEngine::new()));

        Ok(Self {
            journal,
            fuse_integration,
            vector_storage,
            hnsw_graph,
            analytics_engine,
            metrics,
            config,
            correlation_tracker,
            reasoning_engine,
        })
    }

    /// Initialize the integration manager
    pub async fn initialize(&self) -> SemanticResult<()> {
        // Initialize analytics engine
        if self.config.enable_real_time_analytics {
            self.initialize_analytics_engine().await?;
        }

        // Initialize semantic reasoning
        if self.config.enable_semantic_reasoning {
            self.initialize_reasoning_engine().await?;
        }

        // Start background tasks
        self.start_background_tasks().await?;

        Ok(())
    }

    /// Process a graph operation and generate semantic events
    pub async fn process_graph_operation(
        &self,
        operation: GraphOperation,
        context: &OperationContext,
    ) -> SemanticResult<GraphOperationResult> {
        let start_time = Instant::now();

        // Create semantic event for the operation
        let event = self.create_graph_semantic_event(&operation, context).await?;

        // Journal the event if enabled
        if self.config.auto_journal_graph_events {
            self.journal.write_event(&event).await?;
        }

        // Process the actual graph operation
        let result = self.execute_graph_operation(operation, context).await?;

        // Update performance metrics
        self.update_performance_metrics(&result, start_time.elapsed()).await?;

        // Trigger analytics if enabled
        if self.config.enable_real_time_analytics {
            self.trigger_analytics_update(&result).await?;
        }

        // Update correlations
        self.update_event_correlations(&event, &result).await?;

        // Trigger semantic reasoning if enabled
        if self.config.enable_semantic_reasoning {
            self.trigger_semantic_reasoning(&event, &result).await?;
        }

        Ok(result)
    }

    /// Get current graph analytics
    pub async fn get_graph_analytics(&self) -> SemanticResult<GraphAnalyticsSnapshot> {
        let analytics = self.analytics_engine.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire analytics engine read lock".to_string())
        })?;

        let metrics = self.metrics.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire metrics read lock".to_string())
        })?;

        Ok(GraphAnalyticsSnapshot {
            timestamp: SystemTime::now(),
            performance_metrics: metrics.clone(),
            health_metrics: analytics.health_metrics.clone(),
            centrality_measures: analytics.centrality_cache.clone(),
            clustering_results: analytics.clustering_cache.clone(),
            analytics_history: analytics.analytics_history.clone(),
        })
    }

    /// Get semantic reasoning results
    pub async fn get_reasoning_results(&self, query: &str) -> SemanticResult<InferenceResult> {
        let reasoning_engine = self.reasoning_engine.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire reasoning engine read lock".to_string())
        })?;

        reasoning_engine.infer(query)
    }

    /// Get event correlations
    pub async fn get_event_correlations(&self, event_id: Uuid) -> SemanticResult<Vec<CorrelationInfo>> {
        let tracker = self.correlation_tracker.read().map_err(|_| {
            SemanticError::LockError("Failed to acquire correlation tracker read lock".to_string())
        })?;

        Ok(tracker.get_correlations(event_id))
    }

    // Private implementation methods...

    async fn initialize_analytics_engine(&self) -> SemanticResult<()> {
        // Initialize analytics engine with current graph state
        let mut analytics = self.analytics_engine.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire analytics engine write lock".to_string())
        })?;

        analytics.initialize().await?;
        Ok(())
    }

    async fn initialize_reasoning_engine(&self) -> SemanticResult<()> {
        // Initialize semantic reasoning engine
        let mut reasoning = self.reasoning_engine.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire reasoning engine write lock".to_string())
        })?;

        reasoning.initialize().await?;
        Ok(())
    }

    async fn start_background_tasks(&self) -> SemanticResult<()> {
        // Start background analytics tasks
        // Start background reasoning tasks
        // Start background correlation tasks
        Ok(())
    }

    async fn create_graph_semantic_event(
        &self,
        operation: &GraphOperation,
        context: &OperationContext,
    ) -> SemanticResult<SemanticEvent> {
        // Create semantic event based on graph operation
        let event_type = match operation {
            GraphOperation::Search(_) => SemanticEventType::GraphQuery,
            GraphOperation::Insert(_) => SemanticEventType::GraphNodeCreate,
            GraphOperation::Delete(_) => SemanticEventType::GraphNodeDelete,
            GraphOperation::Update(_) => SemanticEventType::GraphNodeUpdate,
        };

        Ok(SemanticEvent {
            event_id: Uuid::new_v4(),
            event_type,
            timestamp: SystemTime::now(),
            process_id: context.process_id,
            thread_id: context.thread_id,
            user_id: context.user_id,
            inode: context.inode,
            path: context.path.clone(),
            operation_id: context.operation_id,
            parent_operation_id: context.parent_operation_id,
            causality_id: context.causality_id,
            intent_id: context.intent_id,
            context_data: self.create_graph_context_data(operation).await?,
            metadata: HashMap::new(),
        })
    }

    async fn create_graph_context_data(&self, operation: &GraphOperation) -> SemanticResult<Vec<u8>> {
        // Create context data specific to graph operations
        // This would include operation parameters, graph state, etc.
        Ok(Vec::new()) // Placeholder
    }

    async fn execute_graph_operation(
        &self,
        operation: GraphOperation,
        context: &OperationContext,
    ) -> SemanticResult<GraphOperationResult> {
        // Execute the actual graph operation
        // This would delegate to the HNSW graph implementation
        Ok(GraphOperationResult::default()) // Placeholder
    }

    async fn update_performance_metrics(
        &self,
        result: &GraphOperationResult,
        duration: Duration,
    ) -> SemanticResult<()> {
        let mut metrics = self.metrics.write().map_err(|_| {
            SemanticError::LockError("Failed to acquire metrics write lock".to_string())
        })?;

        metrics.total_operations += 1;
        metrics.last_update = SystemTime::now();
        // Update specific metrics based on operation type and result
        Ok(())
    }

    async fn trigger_analytics_update(&self, result: &GraphOperationResult) -> SemanticResult<()> {
        // Trigger analytics update based on operation result
        Ok(())
    }

    async fn update_event_correlations(
        &self,
        event: &SemanticEvent,
        result: &GraphOperationResult,
    ) -> SemanticResult<()> {
        // Update event correlations
        Ok(())
    }

    async fn trigger_semantic_reasoning(
        &self,
        event: &SemanticEvent,
        result: &GraphOperationResult,
    ) -> SemanticResult<()> {
        // Trigger semantic reasoning based on event and result
        Ok(())
    }
}

// Placeholder types for compilation
#[derive(Debug, Clone)]
pub enum GraphOperation {
    Search(GraphSearchParams),
    Insert(GraphInsertParams),
    Delete(GraphDeleteParams),
    Update(GraphUpdateParams),
}

#[derive(Debug, Clone)]
pub struct GraphSearchParams {
    pub query_vector: Vec<f32>,
    pub k: usize,
    pub ef_search: usize,
}

#[derive(Debug, Clone)]
pub struct GraphInsertParams {
    pub node_id: u64,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct GraphDeleteParams {
    pub node_id: u64,
}

#[derive(Debug, Clone)]
pub struct GraphUpdateParams {
    pub node_id: u64,
    pub vector: Option<Vec<f32>>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphOperationResult {
    pub success: bool,
    pub operation_id: Uuid,
    pub processing_time_us: u64,
    pub nodes_affected: Vec<u64>,
    pub edges_affected: Vec<(u64, u64)>,
    pub result_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GraphAnalyticsSnapshot {
    pub timestamp: SystemTime,
    pub performance_metrics: GraphPerformanceMetrics,
    pub health_metrics: GraphHealthMetrics,
    pub centrality_measures: BTreeMap<u64, CentralityMeasures>,
    pub clustering_results: Option<ClusteringResults>,
    pub analytics_history: VecDeque<AnalyticsSnapshot>,
}

// Implementation stubs for compilation
impl GraphAnalyticsEngine {
    fn new() -> Self {
        Self {
            centrality_cache: BTreeMap::new(),
            clustering_cache: None,
            health_metrics: GraphHealthMetrics::default(),
            analytics_history: VecDeque::new(),
            last_update: SystemTime::now(),
        }
    }

    async fn initialize(&mut self) -> SemanticResult<()> {
        Ok(())
    }
}

impl EventCorrelationTracker {
    fn new() -> Self {
        Self {
            correlations: HashMap::new(),
            patterns: Vec::new(),
            history: VecDeque::new(),
            max_history_size: 1000,
        }
    }

    fn get_correlations(&self, _event_id: Uuid) -> Vec<CorrelationInfo> {
        Vec::new()
    }
}

impl SemanticReasoningEngine {
    fn new() -> Self {
        Self {
            knowledge_graph: SemanticKnowledgeGraph::new(),
            reasoning_rules: Vec::new(),
            inference_cache: HashMap::new(),
            reasoning_history: VecDeque::new(),
        }
    }

    async fn initialize(&mut self) -> SemanticResult<()> {
        Ok(())
    }

    fn infer(&self, _query: &str) -> SemanticResult<InferenceResult> {
        Ok(InferenceResult {
            facts: Vec::new(),
            confidence: 0.0,
            reasoning_path: Vec::new(),
            inferred_at: SystemTime::now(),
        })
    }
}

impl SemanticKnowledgeGraph {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            hierarchy: BTreeMap::new(),
        }
    }
}

// Import required types from other modules
use crate::fs_core::operations::OperationContext;