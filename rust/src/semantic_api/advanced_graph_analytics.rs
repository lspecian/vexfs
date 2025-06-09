//! Advanced Graph Analytics Engine
//! 
//! This module implements Phase 3 of Task 23.5, providing comprehensive advanced
//! graph analytics algorithms including centrality measures, pathfinding algorithms,
//! enhanced clustering, and graph health monitoring.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use crate::semantic_api::graph_journal_integration::{
    GraphJournalIntegrationManager, CentralityMeasures, ClusteringResults, GraphHealthMetrics
};
use crate::semantic_api::fuse_graph_integration_manager::FuseGraphIntegrationManager;
use crate::anns::hnsw_optimized::OptimizedHnswGraph;
use crate::shared::errors::{VexfsError, VexfsResult};

use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::collections::{HashMap, BTreeMap, VecDeque, HashSet, BinaryHeap};
use std::cmp::Ordering;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Maximum stack usage for advanced analytics operations (6KB limit)
const ADVANCED_ANALYTICS_MAX_STACK_USAGE: usize = 6144;

/// Default PageRank damping factor
const DEFAULT_PAGERANK_DAMPING: f64 = 0.85;

/// Default PageRank iterations
const DEFAULT_PAGERANK_ITERATIONS: usize = 100;

/// Default PageRank convergence threshold
const DEFAULT_PAGERANK_CONVERGENCE: f64 = 1e-6;

/// Advanced Graph Analytics Engine
/// 
/// Core analytics engine providing comprehensive graph analysis capabilities:
/// - Advanced centrality measures (degree, betweenness, PageRank, eigenvector)
/// - Pathfinding algorithms (shortest path, A*, Dijkstra)
/// - Enhanced clustering with silhouette scores
/// - Graph health monitoring and quality metrics
pub struct AdvancedGraphAnalytics {
    /// Graph journal integration manager
    graph_journal_manager: Arc<GraphJournalIntegrationManager>,
    /// FUSE graph integration manager
    fuse_integration_manager: Arc<FuseGraphIntegrationManager>,
    /// Centrality calculator
    centrality_calculator: Arc<RwLock<CentralityCalculator>>,
    /// Pathfinding engine
    pathfinding_engine: Arc<RwLock<PathfindingEngine>>,
    /// Clustering analyzer
    clustering_analyzer: Arc<RwLock<ClusteringAnalyzer>>,
    /// Graph health monitor
    health_monitor: Arc<RwLock<GraphHealthMonitor>>,
    /// Analytics configuration
    config: AdvancedAnalyticsConfig,
    /// Performance metrics
    analytics_metrics: Arc<RwLock<AdvancedAnalyticsMetrics>>,
}

/// Configuration for advanced graph analytics
#[derive(Debug, Clone)]
pub struct AdvancedAnalyticsConfig {
    /// Enable centrality measures
    pub enable_centrality_measures: bool,
    /// Enable pathfinding algorithms
    pub enable_pathfinding: bool,
    /// Enable enhanced clustering
    pub enable_clustering: bool,
    /// Enable health monitoring
    pub enable_health_monitoring: bool,
    /// PageRank configuration
    pub pagerank_config: PageRankConfig,
    /// Clustering configuration
    pub clustering_config: ClusteringConfig,
    /// Health monitoring configuration
    pub health_config: HealthMonitoringConfig,
    /// Performance optimization settings
    pub performance_config: AnalyticsPerformanceConfig,
}

impl Default for AdvancedAnalyticsConfig {
    fn default() -> Self {
        Self {
            enable_centrality_measures: true,
            enable_pathfinding: true,
            enable_clustering: true,
            enable_health_monitoring: true,
            pagerank_config: PageRankConfig::default(),
            clustering_config: ClusteringConfig::default(),
            health_config: HealthMonitoringConfig::default(),
            performance_config: AnalyticsPerformanceConfig::default(),
        }
    }
}

/// PageRank algorithm configuration
#[derive(Debug, Clone)]
pub struct PageRankConfig {
    /// Damping factor (typically 0.85)
    pub damping_factor: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence threshold
    pub convergence_threshold: f64,
    /// Enable personalized PageRank
    pub enable_personalized: bool,
}

impl Default for PageRankConfig {
    fn default() -> Self {
        Self {
            damping_factor: DEFAULT_PAGERANK_DAMPING,
            max_iterations: DEFAULT_PAGERANK_ITERATIONS,
            convergence_threshold: DEFAULT_PAGERANK_CONVERGENCE,
            enable_personalized: false,
        }
    }
}

/// Clustering algorithm configuration
#[derive(Debug, Clone)]
pub struct ClusteringConfig {
    /// Number of clusters for k-means
    pub k_means_clusters: usize,
    /// Maximum k-means iterations
    pub k_means_max_iterations: usize,
    /// Enable hierarchical clustering
    pub enable_hierarchical: bool,
    /// Enable spectral clustering
    pub enable_spectral: bool,
    /// Silhouette score calculation
    pub calculate_silhouette_scores: bool,
    /// Community detection settings
    pub community_detection: CommunityDetectionConfig,
}

impl Default for ClusteringConfig {
    fn default() -> Self {
        Self {
            k_means_clusters: 5,
            k_means_max_iterations: 100,
            enable_hierarchical: true,
            enable_spectral: true,
            calculate_silhouette_scores: true,
            community_detection: CommunityDetectionConfig::default(),
        }
    }
}

/// Community detection configuration
#[derive(Debug, Clone)]
pub struct CommunityDetectionConfig {
    /// Enable modularity optimization
    pub enable_modularity_optimization: bool,
    /// Modularity resolution parameter
    pub modularity_resolution: f64,
    /// Enable Louvain algorithm
    pub enable_louvain: bool,
    /// Maximum Louvain iterations
    pub louvain_max_iterations: usize,
}

impl Default for CommunityDetectionConfig {
    fn default() -> Self {
        Self {
            enable_modularity_optimization: true,
            modularity_resolution: 1.0,
            enable_louvain: true,
            louvain_max_iterations: 100,
        }
    }
}

/// Health monitoring configuration
#[derive(Debug, Clone)]
pub struct HealthMonitoringConfig {
    /// Enable connectivity analysis
    pub enable_connectivity_analysis: bool,
    /// Enable consistency validation
    pub enable_consistency_validation: bool,
    /// Enable performance bottleneck detection
    pub enable_bottleneck_detection: bool,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    /// Quality score thresholds
    pub quality_thresholds: QualityThresholds,
}

impl Default for HealthMonitoringConfig {
    fn default() -> Self {
        Self {
            enable_connectivity_analysis: true,
            enable_consistency_validation: true,
            enable_bottleneck_detection: true,
            health_check_interval_seconds: 60,
            quality_thresholds: QualityThresholds::default(),
        }
    }
}

/// Quality score thresholds for health monitoring
#[derive(Debug, Clone)]
pub struct QualityThresholds {
    /// Minimum connectivity score (0.0 to 1.0)
    pub min_connectivity_score: f64,
    /// Maximum average path length
    pub max_avg_path_length: f64,
    /// Minimum clustering coefficient
    pub min_clustering_coefficient: f64,
    /// Maximum disconnected components ratio
    pub max_disconnected_ratio: f64,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            min_connectivity_score: 0.8,
            max_avg_path_length: 10.0,
            min_clustering_coefficient: 0.3,
            max_disconnected_ratio: 0.1,
        }
    }
}

/// Performance configuration for analytics
#[derive(Debug, Clone)]
pub struct AnalyticsPerformanceConfig {
    /// Maximum nodes to process in single batch
    pub max_batch_size: usize,
    /// Enable parallel processing
    pub enable_parallel_processing: bool,
    /// Number of worker threads
    pub worker_threads: usize,
    /// Memory limit for analytics operations
    pub memory_limit_mb: usize,
    /// Enable result caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for AnalyticsPerformanceConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            enable_parallel_processing: true,
            worker_threads: 4,
            memory_limit_mb: 512,
            enable_caching: true,
            cache_ttl_seconds: 300,
        }
    }
}

/// Centrality Calculator
/// 
/// Implements advanced centrality measures with efficient algorithms
pub struct CentralityCalculator {
    /// Cached centrality results
    centrality_cache: HashMap<u64, EnhancedCentralityMeasures>,
    /// PageRank scores cache
    pagerank_cache: HashMap<u64, f64>,
    /// Eigenvector centrality cache
    eigenvector_cache: HashMap<u64, f64>,
    /// Last calculation timestamp
    last_calculation: SystemTime,
    /// Configuration
    config: PageRankConfig,
}

/// Enhanced centrality measures with additional metrics
#[derive(Debug, Clone)]
pub struct EnhancedCentralityMeasures {
    /// Basic centrality measures
    pub basic: CentralityMeasures,
    /// In-degree centrality
    pub in_degree_centrality: f64,
    /// Out-degree centrality
    pub out_degree_centrality: f64,
    /// Total degree centrality
    pub total_degree_centrality: f64,
    /// Closeness centrality
    pub closeness_centrality: f64,
    /// Harmonic centrality
    pub harmonic_centrality: f64,
    /// Katz centrality
    pub katz_centrality: f64,
    /// Authority score (HITS algorithm)
    pub authority_score: f64,
    /// Hub score (HITS algorithm)
    pub hub_score: f64,
    /// Calculation metadata
    pub calculation_metadata: CentralityCalculationMetadata,
}

/// Metadata for centrality calculations
#[derive(Debug, Clone)]
pub struct CentralityCalculationMetadata {
    /// Calculation timestamp
    pub calculated_at: SystemTime,
    /// Calculation duration in milliseconds
    pub calculation_duration_ms: u64,
    /// Algorithm used
    pub algorithm_used: String,
    /// Convergence achieved (for iterative algorithms)
    pub convergence_achieved: bool,
    /// Number of iterations (for iterative algorithms)
    pub iterations_used: usize,
}

/// Pathfinding Engine
/// 
/// Implements comprehensive pathfinding algorithms
pub struct PathfindingEngine {
    /// Shortest path cache
    path_cache: HashMap<(u64, u64), PathfindingResult>,
    /// Distance matrix cache
    distance_cache: HashMap<(u64, u64), f64>,
    /// All-pairs shortest paths cache
    all_pairs_cache: Option<AllPairsShortestPaths>,
    /// Last pathfinding operation
    last_operation: SystemTime,
}

/// Pathfinding algorithm result
#[derive(Debug, Clone)]
pub struct PathfindingResult {
    /// Source node
    pub source: u64,
    /// Target node
    pub target: u64,
    /// Path found (sequence of node IDs)
    pub path: Vec<u64>,
    /// Total path distance/cost
    pub total_distance: f64,
    /// Path quality metrics
    pub quality_metrics: PathQualityMetrics,
    /// Algorithm used
    pub algorithm_used: PathfindingAlgorithm,
    /// Calculation timestamp
    pub calculated_at: SystemTime,
}

/// Path quality metrics
#[derive(Debug, Clone)]
pub struct PathQualityMetrics {
    /// Path length (number of hops)
    pub path_length: usize,
    /// Average edge weight
    pub avg_edge_weight: f64,
    /// Path efficiency (direct distance / path distance)
    pub path_efficiency: f64,
    /// Bottleneck edge weight
    pub bottleneck_weight: f64,
    /// Path diversity score
    pub diversity_score: f64,
}

/// Pathfinding algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum PathfindingAlgorithm {
    /// Dijkstra's algorithm
    Dijkstra,
    /// A* algorithm with heuristic
    AStar,
    /// Bidirectional search
    Bidirectional,
    /// Floyd-Warshall (all-pairs)
    FloydWarshall,
    /// Bellman-Ford algorithm
    BellmanFord,
}

/// All-pairs shortest paths result
#[derive(Debug, Clone)]
pub struct AllPairsShortestPaths {
    /// Distance matrix
    pub distances: HashMap<(u64, u64), f64>,
    /// Next hop matrix for path reconstruction
    pub next_hop: HashMap<(u64, u64), Option<u64>>,
    /// Calculation timestamp
    pub calculated_at: SystemTime,
    /// Number of nodes processed
    pub nodes_processed: usize,
}

/// Clustering Analyzer
/// 
/// Implements advanced clustering algorithms with quality metrics
pub struct ClusteringAnalyzer {
    /// Current clustering results
    current_clustering: Option<EnhancedClusteringResults>,
    /// Clustering history
    clustering_history: VecDeque<EnhancedClusteringResults>,
    /// Silhouette score cache
    silhouette_cache: HashMap<usize, f64>,
    /// Community detection results
    community_results: Option<CommunityDetectionResults>,
    /// Configuration
    config: ClusteringConfig,
}

/// Enhanced clustering results with quality metrics
#[derive(Debug, Clone)]
pub struct EnhancedClusteringResults {
    /// Basic clustering results
    pub basic: ClusteringResults,
    /// Clustering algorithm used
    pub algorithm_used: ClusteringAlgorithm,
    /// Cluster quality metrics
    pub quality_metrics: ClusterQualityMetrics,
    /// Cluster stability metrics
    pub stability_metrics: ClusterStabilityMetrics,
    /// Validation metrics
    pub validation_metrics: ClusterValidationMetrics,
    /// Calculation metadata
    pub calculation_metadata: ClusteringCalculationMetadata,
}

/// Clustering algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum ClusteringAlgorithm {
    /// K-means clustering
    KMeans,
    /// Hierarchical clustering
    Hierarchical,
    /// Spectral clustering
    Spectral,
    /// DBSCAN
    DBSCAN,
    /// Community detection (Louvain)
    Louvain,
}

/// Cluster quality metrics
#[derive(Debug, Clone)]
pub struct ClusterQualityMetrics {
    /// Overall silhouette score
    pub overall_silhouette_score: f64,
    /// Individual cluster silhouette scores
    pub cluster_silhouette_scores: Vec<f64>,
    /// Calinski-Harabasz index
    pub calinski_harabasz_index: f64,
    /// Davies-Bouldin index
    pub davies_bouldin_index: f64,
    /// Dunn index
    pub dunn_index: f64,
    /// Intra-cluster distances
    pub intra_cluster_distances: Vec<f64>,
    /// Inter-cluster distances
    pub inter_cluster_distances: Vec<Vec<f64>>,
}

/// Cluster stability metrics
#[derive(Debug, Clone)]
pub struct ClusterStabilityMetrics {
    /// Stability score (0.0 to 1.0)
    pub stability_score: f64,
    /// Cluster persistence over time
    pub cluster_persistence: Vec<f64>,
    /// Membership stability
    pub membership_stability: f64,
    /// Centroid stability
    pub centroid_stability: f64,
}

/// Cluster validation metrics
#[derive(Debug, Clone)]
pub struct ClusterValidationMetrics {
    /// Internal validation score
    pub internal_validation_score: f64,
    /// External validation score (if ground truth available)
    pub external_validation_score: Option<f64>,
    /// Relative validation score
    pub relative_validation_score: f64,
    /// Cluster compactness
    pub cluster_compactness: Vec<f64>,
    /// Cluster separation
    pub cluster_separation: Vec<f64>,
}

/// Clustering calculation metadata
#[derive(Debug, Clone)]
pub struct ClusteringCalculationMetadata {
    /// Calculation timestamp
    pub calculated_at: SystemTime,
    /// Calculation duration in milliseconds
    pub calculation_duration_ms: u64,
    /// Number of iterations
    pub iterations_used: usize,
    /// Convergence achieved
    pub convergence_achieved: bool,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
}

/// Community detection results
#[derive(Debug, Clone)]
pub struct CommunityDetectionResults {
    /// Community assignments
    pub community_assignments: HashMap<u64, usize>,
    /// Number of communities
    pub num_communities: usize,
    /// Modularity score
    pub modularity_score: f64,
    /// Community sizes
    pub community_sizes: Vec<usize>,
    /// Community quality metrics
    pub community_quality: CommunityQualityMetrics,
    /// Calculation timestamp
    pub calculated_at: SystemTime,
}

/// Community quality metrics
#[derive(Debug, Clone)]
pub struct CommunityQualityMetrics {
    /// Modularity score
    pub modularity: f64,
    /// Coverage (fraction of edges within communities)
    pub coverage: f64,
    /// Performance (fraction of correctly classified edges)
    pub performance: f64,
    /// Conductance (quality of community boundaries)
    pub conductance: Vec<f64>,
    /// Internal density per community
    pub internal_density: Vec<f64>,
    /// External density per community
    pub external_density: Vec<f64>,
}

/// Graph Health Monitor
/// 
/// Implements comprehensive graph health monitoring and quality assessment
pub struct GraphHealthMonitor {
    /// Current health metrics
    current_health: GraphHealthMetrics,
    /// Health history
    health_history: VecDeque<GraphHealthSnapshot>,
    /// Quality recommendations
    quality_recommendations: Vec<QualityRecommendation>,
    /// Performance bottlenecks
    performance_bottlenecks: Vec<PerformanceBottleneck>,
    /// Configuration
    config: HealthMonitoringConfig,
}

/// Graph health snapshot
#[derive(Debug, Clone)]
pub struct GraphHealthSnapshot {
    /// Basic health metrics
    pub basic_health: GraphHealthMetrics,
    /// Extended health metrics
    pub extended_health: ExtendedHealthMetrics,
    /// Performance indicators
    pub performance_indicators: PerformanceIndicators,
    /// Quality assessment
    pub quality_assessment: QualityAssessment,
    /// Snapshot timestamp
    pub snapshot_timestamp: SystemTime,
}

/// Extended health metrics
#[derive(Debug, Clone)]
pub struct ExtendedHealthMetrics {
    /// Graph diameter
    pub graph_diameter: f64,
    /// Graph radius
    pub graph_radius: f64,
    /// Assortativity coefficient
    pub assortativity_coefficient: f64,
    /// Rich club coefficient
    pub rich_club_coefficient: f64,
    /// Small world coefficient
    pub small_world_coefficient: f64,
    /// Scale-free properties
    pub scale_free_properties: ScaleFreeProperties,
}

/// Scale-free properties
#[derive(Debug, Clone)]
pub struct ScaleFreeProperties {
    /// Power law exponent
    pub power_law_exponent: f64,
    /// Goodness of fit for power law
    pub power_law_goodness_of_fit: f64,
    /// Is scale-free
    pub is_scale_free: bool,
    /// Degree distribution entropy
    pub degree_distribution_entropy: f64,
}

/// Performance indicators
#[derive(Debug, Clone)]
pub struct PerformanceIndicators {
    /// Search performance score
    pub search_performance_score: f64,
    /// Insertion performance score
    pub insertion_performance_score: f64,
    /// Memory efficiency score
    pub memory_efficiency_score: f64,
    /// Cache efficiency score
    pub cache_efficiency_score: f64,
    /// Overall performance score
    pub overall_performance_score: f64,
}

/// Quality assessment
#[derive(Debug, Clone)]
pub struct QualityAssessment {
    /// Overall quality score (0.0 to 1.0)
    pub overall_quality_score: f64,
    /// Structural quality score
    pub structural_quality_score: f64,
    /// Performance quality score
    pub performance_quality_score: f64,
    /// Consistency quality score
    pub consistency_quality_score: f64,
    /// Quality trend
    pub quality_trend: QualityTrend,
    /// Quality recommendations
    pub recommendations: Vec<QualityRecommendation>,
}

/// Quality trend
#[derive(Debug, Clone, PartialEq)]
pub enum QualityTrend {
    /// Quality is improving
    Improving,
    /// Quality is stable
    Stable,
    /// Quality is degrading
    Degrading,
    /// Quality is fluctuating
    Fluctuating,
}

/// Quality recommendation
#[derive(Debug, Clone)]
pub struct QualityRecommendation {
    /// Recommendation ID
    pub recommendation_id: Uuid,
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Description
    pub description: String,
    /// Expected impact
    pub expected_impact: String,
    /// Implementation effort
    pub implementation_effort: ImplementationEffort,
    /// Created timestamp
    pub created_at: SystemTime,
}

/// Recommendation types
#[derive(Debug, Clone, PartialEq)]
pub enum RecommendationType {
    /// Performance optimization
    PerformanceOptimization,
    /// Structural improvement
    StructuralImprovement,
    /// Memory optimization
    MemoryOptimization,
    /// Consistency improvement
    ConsistencyImprovement,
    /// Security enhancement
    SecurityEnhancement,
}

/// Recommendation priority
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecommendationPriority {
    /// Critical priority
    Critical,
    /// High priority
    High,
    /// Medium priority
    Medium,
    /// Low priority
    Low,
}

/// Implementation effort
#[derive(Debug, Clone, PartialEq)]
pub enum ImplementationEffort {
    /// Low effort
    Low,
    /// Medium effort
    Medium,
    /// High effort
    High,
    /// Very high effort
    VeryHigh,
}

/// Performance bottleneck
#[derive(Debug, Clone)]
pub struct PerformanceBottleneck {
    /// Bottleneck ID
    pub bottleneck_id: Uuid,
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,
    /// Affected nodes/edges
    pub affected_elements: Vec<u64>,
    /// Severity score (0.0 to 1.0)
    pub severity_score: f64,
    /// Impact description
    pub impact_description: String,
    /// Suggested resolution
    pub suggested_resolution: String,
    /// Detected timestamp
    pub detected_at: SystemTime,
}

/// Bottleneck types
#[derive(Debug, Clone, PartialEq)]
pub enum BottleneckType {
    /// High degree node bottleneck
    HighDegreeNode,
    /// Bridge edge bottleneck
    BridgeEdge,
    /// Memory bottleneck
    Memory,
    /// CPU bottleneck
    CPU,
    /// I/O bottleneck
    IO,
    /// Cache bottleneck
    Cache,
}

/// Advanced analytics metrics
#[derive(Debug, Clone, Default)]
pub struct AdvancedAnalyticsMetrics {
    /// Total analytics operations
    pub total_operations: u64,
    /// Centrality calculations
    pub centrality_calculations: u64,
    /// Pathfinding operations
    pub pathfinding_operations: u64,
    /// Clustering operations
    pub clustering_operations: u64,
    /// Health monitoring operations
    pub health_monitoring_operations: u64,
    /// Average operation latency
    pub avg_operation_latency_ms: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Memory usage
    pub memory_usage_bytes: u64,
    /// Error rate
    pub error_rate: f64,
    /// Last update timestamp
    pub last_update: SystemTime,
}

impl AdvancedGraphAnalytics {
    /// Create a new advanced graph analytics engine
    pub fn new(
        graph_journal_manager: Arc<GraphJournalIntegrationManager>,
        fuse_integration_manager: Arc<FuseGraphIntegrationManager>,
        config: AdvancedAnalyticsConfig,
    ) -> SemanticResult<Self> {
        Ok(Self {
            graph_journal_manager,
            fuse_integration_manager,
            centrality_calculator: Arc::new(RwLock::new(CentralityCalculator::new(config.pagerank_config.clone())?)),
            pathfinding_engine: Arc::new(RwLock::new(PathfindingEngine::new()?)),
            clustering_analyzer: Arc::new(RwLock::new(ClusteringAnalyzer::new(config.clustering_config.clone())?)),
            health_monitor: Arc::new(RwLock::new(GraphHealthMonitor::new(config.health_config.clone())?)),
            config,
            analytics_metrics: Arc::new(RwLock::new(AdvancedAnalyticsMetrics::default())),
        })
    }

    /// Calculate comprehensive centrality measures for all nodes
    pub async fn calculate_centrality_measures(&self) -> SemanticResult<HashMap<u64, EnhancedCentralityMeasures>> {
        let start_time = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            metrics.total_operations += 1;
            metrics.centrality_calculations += 1;
        }

        let mut calculator = self.centrality_calculator.write()
            .map_err(|e| SemanticError::lock(format!("Failed to acquire centrality calculator lock: {}", e)))?;
        
        let result = calculator.calculate_all_centrality_measures().await?;
        
        // Update metrics with timing
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            let duration = start_time.elapsed().as_millis() as f64;
            metrics.avg_operation_latency_ms = 
                (metrics.avg_operation_latency_ms * (metrics.total_operations - 1) as f64 + duration) / metrics.total_operations as f64;
            metrics.last_update = SystemTime::now();
        }

        Ok(result)
    }

    /// Find shortest path between two nodes using specified algorithm
    pub async fn find_shortest_path(
        &self,
        source: u64,
        target: u64,
        algorithm: PathfindingAlgorithm,
    ) -> SemanticResult<PathfindingResult> {
        let start_time = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            metrics.total_operations += 1;
            metrics.pathfinding_operations += 1;
        }

        let mut engine = self.pathfinding_engine.write()
            .map_err(|e| SemanticError::lock(format!("Failed to acquire pathfinding engine lock: {}", e)))?;
        
        let result = engine.find_shortest_path(source, target, algorithm).await?;
        
        // Update metrics with timing
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            let duration = start_time.elapsed().as_millis() as f64;
            metrics.avg_operation_latency_ms = 
                (metrics.avg_operation_latency_ms * (metrics.total_operations - 1) as f64 + duration) / metrics.total_operations as f64;
            metrics.last_update = SystemTime::now();
        }

        Ok(result)
    }

    /// Perform enhanced clustering analysis with quality metrics
    pub async fn perform_clustering_analysis(&self) -> SemanticResult<EnhancedClusteringResults> {
        let start_time = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            metrics.total_operations += 1;
            metrics.clustering_operations += 1;
        }

        let mut analyzer = self.clustering_analyzer.write()
            .map_err(|e| SemanticError::lock(format!("Failed to acquire clustering analyzer lock: {}", e)))?;
        
        let result = analyzer.perform_enhanced_clustering().await?;
        
        // Update metrics with timing
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            let duration = start_time.elapsed().as_millis() as f64;
            metrics.avg_operation_latency_ms = 
                (metrics.avg_operation_latency_ms * (metrics.total_operations - 1) as f64 + duration) / metrics.total_operations as f64;
            metrics.last_update = SystemTime::now();
        }

        Ok(result)
    }

    /// Perform comprehensive graph health monitoring
    pub async fn monitor_graph_health(&self) -> SemanticResult<GraphHealthSnapshot> {
        let start_time = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            metrics.total_operations += 1;
            metrics.health_monitoring_operations += 1;
        }

        let mut monitor = self.health_monitor.write()
            .map_err(|e| SemanticError::lock(format!("Failed to acquire health monitor lock: {}", e)))?;
        
        let result = monitor.perform_comprehensive_health_check().await?;
        
        // Update metrics with timing
        {
            let mut metrics = self.analytics_metrics.write()
                .map_err(|e| SemanticError::lock(format!("Failed to acquire analytics metrics lock: {}", e)))?;
            let duration = start_time.elapsed().as_millis() as f64;
            metrics.avg_operation_latency_ms = 
                (metrics.avg_operation_latency_ms * (metrics.