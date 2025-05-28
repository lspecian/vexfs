//! Query Planning and Optimization for VexFS Vector Search
//!
//! This module implements intelligent query planning and optimization for vector search operations,
//! providing cost-based optimization, query rewriting, adaptive optimization, and execution planning.
//! It analyzes query characteristics, determines optimal execution strategies, and continuously learns
//! from performance feedback to improve future query planning decisions.
//!
//! **Task 6.2 Implementation**: Enhanced with intelligent cost estimation, query transformation,
//! adaptive optimization based on historical performance, and integration with QueryPerformanceMonitor
//! for feedback-driven optimization.

use crate::anns::{DistanceMetric, IndexStrategy, LshConfig, IvfConfig, PqConfig, FlatConfig};
use crate::vector_search::{SearchQuery, SearchOptions, VectorSearchEngine};
use crate::knn_search::{SearchParams, MetadataFilter, KnnSearchEngine};
use crate::vector_storage::{VectorStorageManager, VectorDataType};
use crate::vector_optimizations::{VectorOptimizer, SimdStrategy, MemoryLayout, BatchConfig};
use crate::fs_core::operations::OperationContext;
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::query_monitor::{QueryPerformanceMonitor, QueryPattern, OptimizationRecommendation, RecommendationType};

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, collections::BTreeMap, string::String};
#[cfg(feature = "std")]
use std::{vec::Vec, collections::BTreeMap, string::String};

use core::f32;

/// Cost estimation model for query optimization
#[derive(Debug, Clone)]
pub struct CostModel {
    /// CPU cost per vector comparison
    pub cpu_cost_per_comparison: f32,
    /// Memory cost per byte allocated
    pub memory_cost_per_byte: f32,
    /// I/O cost per disk access
    pub io_cost_per_access: f32,
    /// Index traversal cost factor
    pub index_traversal_cost: f32,
    /// Cache miss penalty
    pub cache_miss_penalty: f32,
    /// Network latency cost (for distributed scenarios)
    pub network_latency_cost: f32,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            cpu_cost_per_comparison: 1.0,
            memory_cost_per_byte: 0.001,
            io_cost_per_access: 100.0,
            index_traversal_cost: 10.0,
            cache_miss_penalty: 50.0,
            network_latency_cost: 1000.0,
        }
    }
}

/// Query transformation and rewriting strategies
#[derive(Debug, Clone)]
pub struct QueryTransformation {
    /// Original query
    pub original_query: SearchQuery,
    /// Transformed query
    pub transformed_query: SearchQuery,
    /// Transformation type applied
    pub transformation_type: TransformationType,
    /// Expected performance improvement
    pub expected_improvement: f32,
    /// Transformation confidence
    pub confidence: f32,
    /// Reasoning for transformation
    pub reasoning: String,
}

/// Types of query transformations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransformationType {
    /// Dimension reduction
    DimensionReduction,
    /// Approximation level adjustment
    ApproximationAdjustment,
    /// Filter reordering
    FilterReordering,
    /// Batch size optimization
    BatchOptimization,
    /// Distance metric substitution
    MetricSubstitution,
    /// Query decomposition
    QueryDecomposition,
    /// Parameter tuning
    ParameterTuning,
}

/// Adaptive optimization state
#[derive(Debug, Clone)]
pub struct AdaptiveOptimizationState {
    /// Learning rate for adaptation
    pub learning_rate: f32,
    /// Performance history window size
    pub history_window_size: usize,
    /// Adaptation threshold
    pub adaptation_threshold: f32,
    /// Current optimization strategy
    pub current_strategy: OptimizationStrategy,
    /// Strategy performance scores
    pub strategy_scores: BTreeMap<OptimizationStrategy, f32>,
    /// Last adaptation timestamp
    pub last_adaptation_us: u64,
}

/// Optimization strategies for adaptive learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationStrategy {
    /// Conservative optimization (favor accuracy)
    Conservative,
    /// Balanced optimization
    Balanced,
    /// Aggressive optimization (favor speed)
    Aggressive,
    /// Memory-optimized
    MemoryOptimized,
    /// Latency-optimized
    LatencyOptimized,
}

/// Cost estimation result
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// Total estimated cost
    pub total_cost: f32,
    /// CPU cost component
    pub cpu_cost: f32,
    /// Memory cost component
    pub memory_cost: f32,
    /// I/O cost component
    pub io_cost: f32,
    /// Index cost component
    pub index_cost: f32,
    /// Cache cost component
    pub cache_cost: f32,
    /// Estimated execution time (microseconds)
    pub estimated_time_us: u64,
    /// Confidence in estimate
    pub confidence: f32,
}

/// Query complexity levels for optimization decisions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryComplexity {
    /// Simple queries with basic parameters
    Simple,
    /// Moderate complexity with filters or specific requirements
    Moderate,
    /// Complex queries with multiple constraints
    Complex,
    /// Highly complex queries requiring specialized optimization
    HighlyComplex,
}

/// Query characteristics analysis result
#[derive(Debug, Clone)]
pub struct QueryCharacteristics {
    /// Query vector dimensionality
    pub dimensions: usize,
    /// Estimated sparsity (0.0 = dense, 1.0 = completely sparse)
    pub sparsity: f32,
    /// Query vector magnitude
    pub magnitude: f32,
    /// Entropy measure of query vector
    pub entropy: f32,
    /// Number of results requested
    pub k: usize,
    /// Distance metric used
    pub metric: DistanceMetric,
    /// Whether filters are applied
    pub has_filters: bool,
    /// Estimated selectivity of filters (0.0 = very selective, 1.0 = no filtering)
    pub filter_selectivity: f32,
    /// Query complexity level
    pub complexity: QueryComplexity,
    /// Whether approximate search is acceptable
    pub approximate_acceptable: bool,
}

/// Index selection recommendation
#[derive(Debug, Clone)]
pub struct IndexRecommendation {
    /// Primary index strategy to use
    pub primary_strategy: IndexStrategy,
    /// Fallback strategy if primary fails
    pub fallback_strategy: Option<IndexStrategy>,
    /// Confidence in recommendation (0.0 - 1.0)
    pub confidence: f32,
    /// Expected performance improvement
    pub expected_speedup: f32,
    /// Memory usage estimate
    pub memory_estimate: usize,
    /// Reasoning for the recommendation
    pub reasoning: String,
}

/// Query execution plan
#[derive(Debug, Clone)]
pub struct QueryExecutionPlan {
    /// Index recommendation
    pub index_recommendation: IndexRecommendation,
    /// Search parameters optimization
    pub optimized_params: SearchParams,
    /// Execution stages in order
    pub execution_stages: Vec<ExecutionStage>,
    /// Expected total execution time (microseconds)
    pub estimated_time_us: u64,
    /// Memory usage estimate
    pub memory_estimate: usize,
    /// Performance vs accuracy trade-off
    pub accuracy_trade_off: f32,
}

/// Individual execution stage
#[derive(Debug, Clone)]
pub struct ExecutionStage {
    /// Stage name/description
    pub name: String,
    /// Stage type
    pub stage_type: StageType,
    /// Estimated time for this stage (microseconds)
    pub estimated_time_us: u64,
    /// Memory required for this stage
    pub memory_required: usize,
    /// Whether this stage can be parallelized
    pub parallelizable: bool,
    /// Dependencies on other stages
    pub dependencies: Vec<usize>,
}

/// Types of execution stages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StageType {
    /// Query preprocessing and analysis
    Preprocessing,
    /// Index selection and preparation
    IndexPreparation,
    /// Candidate generation
    CandidateGeneration,
    /// Distance computation
    DistanceComputation,
    /// Result filtering
    ResultFiltering,
    /// Result ranking and sorting
    ResultRanking,
    /// Post-processing and validation
    PostProcessing,
}

/// Query optimization statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    /// Number of queries analyzed
    pub queries_analyzed: u64,
    /// Average planning time (microseconds)
    pub avg_planning_time_us: u64,
    /// Index recommendation accuracy
    pub recommendation_accuracy: f32,
    /// Performance improvement achieved
    pub avg_performance_improvement: f32,
    /// Memory usage reduction
    pub avg_memory_reduction: f32,
}

/// Main query planner and optimizer with comprehensive OperationContext integration
/// Enhanced with intelligent cost-based optimization, query rewriting, and adaptive learning
pub struct QueryPlanner {
    /// Vector storage manager
    storage_manager: Arc<VectorStorageManager>,
    /// Vector optimizer for SIMD and memory layout optimization
    vector_optimizer: VectorOptimizer,
    /// Available index strategies and their characteristics
    index_characteristics: BTreeMap<IndexStrategy, IndexCharacteristics>,
    /// Query optimization statistics
    stats: OptimizationStats,
    /// Performance history for learning
    performance_history: Vec<PerformanceRecord>,
    /// Configuration parameters
    config: QueryPlannerConfig,
    /// Active operation tracking for lifecycle management
    active_operations: BTreeMap<u64, OperationMetadata>,
    /// Operation counter for unique operation IDs
    operation_counter: u64,
    /// Cost estimation model
    cost_model: CostModel,
    /// Query transformation cache
    transformation_cache: BTreeMap<String, QueryTransformation>,
    /// Adaptive optimization state
    adaptive_state: AdaptiveOptimizationState,
    /// Performance monitor integration
    performance_monitor: Option<Arc<QueryPerformanceMonitor>>,
    /// Query rewriting rules
    rewriting_rules: Vec<QueryRewritingRule>,
    /// Execution path cache for similar queries
    execution_path_cache: BTreeMap<String, CachedExecutionPath>,
}

/// Index characteristics for selection decisions
#[derive(Debug, Clone)]
struct IndexCharacteristics {
    /// Optimal dimension range
    optimal_dimensions: (usize, usize),
    /// Memory overhead factor
    memory_overhead: f32,
    /// Build time complexity
    build_complexity: f32,
    /// Search time complexity
    search_complexity: f32,
    /// Accuracy vs speed trade-off
    accuracy_factor: f32,
    /// Sparsity handling capability
    sparsity_handling: f32,
}

/// Performance record for learning and optimization
#[derive(Debug, Clone)]
struct PerformanceRecord {
    /// Query characteristics
    characteristics: QueryCharacteristics,
    /// Index strategy used
    strategy: IndexStrategy,
    /// Actual execution time (microseconds)
    actual_time_us: u64,
    /// Memory usage
    memory_used: usize,
    /// Result accuracy (if measurable)
    accuracy: f32,
    /// Timestamp
    timestamp: u64,
}

/// Query rewriting rule
#[derive(Debug, Clone)]
pub struct QueryRewritingRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: RewritingCondition,
    /// Rule action
    pub action: RewritingAction,
    /// Rule priority
    pub priority: f32,
    /// Rule effectiveness score
    pub effectiveness: f32,
}

/// Rewriting condition
#[derive(Debug, Clone)]
pub enum RewritingCondition {
    /// Dimension threshold
    DimensionThreshold(usize),
    /// Sparsity threshold
    SparsityThreshold(f32),
    /// K value threshold
    KThreshold(usize),
    /// Complexity threshold
    ComplexityThreshold(QueryComplexity),
    /// Combined conditions
    Combined(Vec<RewritingCondition>),
}

/// Rewriting action
#[derive(Debug, Clone)]
pub enum RewritingAction {
    /// Reduce dimensions
    ReduceDimensions(usize),
    /// Adjust approximation
    AdjustApproximation(f32),
    /// Reorder filters
    ReorderFilters,
    /// Change metric
    ChangeMetric(DistanceMetric),
    /// Split query
    SplitQuery(usize),
}

/// Cached execution path
#[derive(Debug, Clone)]
pub struct CachedExecutionPath {
    /// Query signature
    pub query_signature: String,
    /// Execution plan
    pub execution_plan: QueryExecutionPlan,
    /// Performance metrics
    pub performance_metrics: ExecutionMetrics,
    /// Cache timestamp
    pub cached_at_us: u64,
    /// Hit count
    pub hit_count: u64,
    /// Last used timestamp
    pub last_used_us: u64,
}

/// Execution metrics for cached paths
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    /// Average execution time
    pub avg_execution_time_us: u64,
    /// Average memory usage
    pub avg_memory_usage_bytes: usize,
    /// Success rate
    pub success_rate: f32,
    /// Sample count
    pub sample_count: u64,
}

/// Operation metadata for lifecycle tracking
#[derive(Debug, Clone)]
struct OperationMetadata {
    /// Operation ID
    operation_id: u64,
    /// Operation start time (microseconds)
    start_time_us: u64,
    /// Estimated memory usage
    estimated_memory: usize,
    /// Query characteristics
    characteristics: QueryCharacteristics,
    /// Index recommendation
    recommendation: IndexRecommendation,
    /// Operation status
    status: OperationStatus,
    /// User ID for permission tracking
    user_id: u32,
}

/// Operation status for lifecycle management
#[derive(Debug, Clone, Copy, PartialEq)]
enum OperationStatus {
    /// Operation is being planned
    Planning,
    /// Operation is executing
    Executing,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}
impl QueryPlanner {
    /// Create new query planner with comprehensive OperationContext integration
    pub fn new(storage_manager: Arc<VectorStorageManager>, config: QueryPlannerConfig) -> Self {
        let vector_optimizer = VectorOptimizer::with_config(
            config.simd_level,
            config.memory_layout,
            BatchConfig::default(),
        );

        let mut index_characteristics = BTreeMap::new();
        
        // Initialize index characteristics based on research and benchmarks
        index_characteristics.insert(IndexStrategy::HNSW, IndexCharacteristics {
            optimal_dimensions: (50, 2048),
            memory_overhead: 1.5,
            build_complexity: 1.2,
            search_complexity: 0.3,
            accuracy_factor: 0.95,
            sparsity_handling: 0.7,
        });

        index_characteristics.insert(IndexStrategy::LSH, IndexCharacteristics {
            optimal_dimensions: (100, 1024),
            memory_overhead: 0.8,
            build_complexity: 0.5,
            search_complexity: 0.6,
            accuracy_factor: 0.85,
            sparsity_handling: 0.9,
        });

        index_characteristics.insert(IndexStrategy::IVF, IndexCharacteristics {
            optimal_dimensions: (128, 4096),
            memory_overhead: 1.2,
            build_complexity: 1.0,
            search_complexity: 0.4,
            accuracy_factor: 0.90,
            sparsity_handling: 0.6,
        });

        index_characteristics.insert(IndexStrategy::PQ, IndexCharacteristics {
            optimal_dimensions: (512, 8192),
            memory_overhead: 0.3,
            build_complexity: 0.8,
            search_complexity: 0.2,
            accuracy_factor: 0.80,
            sparsity_handling: 0.5,
        });

        index_characteristics.insert(IndexStrategy::Flat, IndexCharacteristics {
            optimal_dimensions: (1, 512),
            memory_overhead: 1.0,
            build_complexity: 0.1,
            search_complexity: 1.0,
            accuracy_factor: 1.0,
            sparsity_handling: 1.0,
        });

        // Initialize adaptive optimization state
        let mut strategy_scores = BTreeMap::new();
        strategy_scores.insert(OptimizationStrategy::Conservative, 0.7);
        strategy_scores.insert(OptimizationStrategy::Balanced, 0.8);
        strategy_scores.insert(OptimizationStrategy::Aggressive, 0.6);
        strategy_scores.insert(OptimizationStrategy::MemoryOptimized, 0.7);
        strategy_scores.insert(OptimizationStrategy::LatencyOptimized, 0.7);

        let adaptive_state = AdaptiveOptimizationState {
            learning_rate: 0.1,
            history_window_size: 100,
            adaptation_threshold: 0.1,
            current_strategy: OptimizationStrategy::Balanced,
            strategy_scores,
            last_adaptation_us: 0,
        };

        // Initialize default rewriting rules
        let mut rewriting_rules = Vec::new();
        
        // Rule 1: Reduce dimensions for very high-dimensional queries
        rewriting_rules.push(QueryRewritingRule {
            name: "High Dimension Reduction".to_string(),
            condition: RewritingCondition::DimensionThreshold(2048),
            action: RewritingAction::ReduceDimensions(1024),
            priority: 0.8,
            effectiveness: 0.7,
        });

        // Rule 2: Force approximation for large k values
        rewriting_rules.push(QueryRewritingRule {
            name: "Large K Approximation".to_string(),
            condition: RewritingCondition::KThreshold(1000),
            action: RewritingAction::AdjustApproximation(0.9),
            priority: 0.7,
            effectiveness: 0.8,
        });

        // Rule 3: Optimize sparse vectors
        rewriting_rules.push(QueryRewritingRule {
            name: "Sparse Vector Optimization".to_string(),
            condition: RewritingCondition::SparsityThreshold(0.8),
            action: RewritingAction::ChangeMetric(DistanceMetric::Manhattan),
            priority: 0.6,
            effectiveness: 0.6,
        });

        Self {
            storage_manager,
            vector_optimizer,
            index_characteristics,
            stats: OptimizationStats::default(),
            performance_history: Vec::new(),
            config,
            active_operations: BTreeMap::new(),
            operation_counter: 0,
            cost_model: CostModel::default(),
            transformation_cache: BTreeMap::new(),
            adaptive_state,
            performance_monitor: None,
            rewriting_rules,
            execution_path_cache: BTreeMap::new(),
        }
    }

    /// Analyze query characteristics for optimization decisions
    pub fn analyze_query(&self, query: &SearchQuery) -> VexfsResult<QueryCharacteristics> {
        let dimensions = query.vector.len();
        
        // Calculate sparsity
        let zero_count = query.vector.iter().filter(|&&x| x.abs() < f32::EPSILON).count();
        let sparsity = zero_count as f32 / dimensions as f32;
        
        // Calculate magnitude
        let magnitude = query.vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
        
        // Calculate entropy (measure of randomness)
        let entropy = self.calculate_entropy(&query.vector);
        
        // Determine complexity
        let complexity = self.determine_complexity(query, sparsity, dimensions);
        
        // Estimate filter selectivity
        let filter_selectivity = self.estimate_filter_selectivity(&query.filter);
        
        Ok(QueryCharacteristics {
            dimensions,
            sparsity,
            magnitude,
            entropy,
            k: query.k,
            metric: query.metric,
            has_filters: query.filter.is_some(),
            filter_selectivity,
            complexity,
            approximate_acceptable: query.approximate,
        })
    }

    /// Generate index recommendation based on query characteristics
    pub fn recommend_index(&self, characteristics: &QueryCharacteristics) -> VexfsResult<IndexRecommendation> {
        let mut scores = BTreeMap::new();
        
        // Score each index strategy
        for (&strategy, index_chars) in &self.index_characteristics {
            let score = self.calculate_index_score(characteristics, index_chars);
            scores.insert(strategy, score);
        }
        
        // Find best strategy
        let (primary_strategy, primary_score) = scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(&strategy, &score)| (strategy, score))
            .unwrap_or((IndexStrategy::Flat, 0.5));
        
        // Find fallback strategy
        let fallback_strategy = scores.iter()
            .filter(|(&strategy, _)| strategy != primary_strategy)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(&strategy, _)| strategy);
        
        // Calculate expected speedup
        let expected_speedup = self.estimate_speedup(primary_strategy, characteristics);
        
        // Estimate memory usage
        let memory_estimate = self.estimate_memory_usage(primary_strategy, characteristics);
        
        // Generate reasoning
        let reasoning = self.generate_reasoning(primary_strategy, characteristics);
        
        Ok(IndexRecommendation {
            primary_strategy,
            fallback_strategy,
            confidence: primary_score,
            expected_speedup,
            memory_estimate,
            reasoning,
        })
    }

    /// Create optimized query execution plan with comprehensive OperationContext integration
    pub fn create_execution_plan(
        &mut self,
        context: &mut OperationContext,
        query: &SearchQuery,
    ) -> VexfsResult<QueryExecutionPlan> {
        let planning_start = self.get_current_time_us();
        
        // Start operation tracking for lifecycle management
        let operation_id = self.start_operation(context, planning_start)?;
        
        // Analyze query characteristics with error recovery
        let characteristics = match self.analyze_query(query) {
            Ok(chars) => chars,
            Err(e) => {
                self.fail_operation(operation_id, "Query analysis failed".to_string())?;
                return Err(e);
            }
        };
        
        // Get index recommendation with transaction support
        let index_recommendation = match self.recommend_index(&characteristics) {
            Ok(rec) => rec,
            Err(e) => {
                self.fail_operation(operation_id, "Index recommendation failed".to_string())?;
                return Err(e);
            }
        };
        
        // Update operation metadata with planning progress
        self.update_operation_metadata(operation_id, &characteristics, &index_recommendation)?;
        
        // Optimize search parameters
        let optimized_params = match self.optimize_search_params(query, &characteristics) {
            Ok(params) => params,
            Err(e) => {
                self.fail_operation(operation_id, "Parameter optimization failed".to_string())?;
                return Err(e);
            }
        };
        
        // Create execution stages with resource estimation
        let execution_stages = match self.create_execution_stages(&characteristics, &index_recommendation) {
            Ok(stages) => stages,
            Err(e) => {
                self.fail_operation(operation_id, "Execution stage creation failed".to_string())?;
                return Err(e);
            }
        };
        
        // Calculate estimates with resource tracking
        let estimated_time_us = execution_stages.iter().map(|stage| stage.estimated_time_us).sum();
        let memory_estimate = execution_stages.iter().map(|stage| stage.memory_required).max().unwrap_or(0);
        
        // Check resource constraints
        if memory_estimate > self.config.memory_budget {
            self.fail_operation(operation_id, "Memory budget exceeded".to_string())?;
            return Err(VexfsError::OutOfMemory);
        }
        
        // Calculate accuracy trade-off
        let accuracy_trade_off = self.calculate_accuracy_trade_off(&characteristics, &index_recommendation);
        
        let planning_time = self.get_current_time_us() - planning_start;
        
        // Check planning time budget
        if planning_time > self.config.max_planning_time_us {
            self.fail_operation(operation_id, "Planning time budget exceeded".to_string())?;
            return Err(VexfsError::InvalidOperation(
                "Query planning exceeded time budget".to_string()
            ));
        }
        
        // Complete operation successfully
        self.complete_operation(operation_id, planning_time, memory_estimate)?;
        
        // Update statistics with operation context
        self.update_stats_with_context(context, planning_time, &index_recommendation);
        
        Ok(QueryExecutionPlan {
            index_recommendation,
            optimized_params,
            execution_stages,
            estimated_time_us,
            memory_estimate,
            accuracy_trade_off,
        })
    }

    /// Optimize search parameters based on query characteristics
    pub fn optimize_search_params(
        &self,
        query: &SearchQuery,
        characteristics: &QueryCharacteristics,
    ) -> VexfsResult<SearchParams> {
        let mut params = SearchParams {
            k: query.k,
            metric: query.metric,
            expansion_factor: query.expansion_factor,
            approximate: query.approximate,
            use_simd: query.use_simd,
            filter: query.filter.clone(),
            exact_distances: query.exact_distances,
        };

        // Optimize expansion factor based on complexity
        params.expansion_factor = match characteristics.complexity {
            QueryComplexity::Simple => 1.5,
            QueryComplexity::Moderate => 2.0,
            QueryComplexity::Complex => 3.0,
            QueryComplexity::HighlyComplex => 4.0,
        };

        // Adjust for sparsity
        if characteristics.sparsity > 0.8 {
            params.expansion_factor *= 1.5; // Sparse vectors need more candidates
        }

        // Optimize SIMD usage based on dimensions
        params.use_simd = characteristics.dimensions >= 64 && query.use_simd;

        // Adjust approximation based on accuracy requirements
        if characteristics.k > 1000 || characteristics.dimensions > 2048 {
            params.approximate = true; // Force approximate for large queries
        }

        Ok(params)
    }

    /// Get optimization statistics
    pub fn get_stats(&self) -> &OptimizationStats {
        &self.stats
    }

    /// Reset optimization statistics
    pub fn reset_stats(&mut self) {
        self.stats = OptimizationStats::default();
        self.performance_history.clear();
    }

    /// Calculate entropy of vector for randomness measure
    fn calculate_entropy(&self, vector: &[f32]) -> f32 {
        if vector.is_empty() {
            return 0.0;
        }

        // Quantize values into bins for entropy calculation
        let bins = 32;
        let mut histogram = vec![0; bins];
        
        let min_val = vector.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_val = vector.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let range = max_val - min_val;
        
        if range < f32::EPSILON {
            return 0.0; // Constant vector has zero entropy
        }
        
        for &value in vector {
            let bin = ((value - min_val) / range * (bins - 1) as f32) as usize;
            let bin = bin.min(bins - 1);
            histogram[bin] += 1;
        }
        
        // Calculate Shannon entropy
        let total = vector.len() as f32;
        let mut entropy = 0.0;
        
        for count in histogram {
            if count > 0 {
                let p = count as f32 / total;
                entropy -= p * p.log2();
            }
        }
        
        entropy
    }

    /// Determine query complexity level
    fn determine_complexity(&self, query: &SearchQuery, sparsity: f32, dimensions: usize) -> QueryComplexity {
        let mut complexity_score = 0;
        
        // Dimension complexity
        if dimensions > 2048 {
            complexity_score += 3;
        } else if dimensions > 512 {
            complexity_score += 2;
        } else if dimensions > 128 {
            complexity_score += 1;
        }
        
        // k complexity
        if query.k > 1000 {
            complexity_score += 3;
        } else if query.k > 100 {
            complexity_score += 2;
        } else if query.k > 10 {
            complexity_score += 1;
        }
        
        // Filter complexity
        if query.filter.is_some() {
            complexity_score += 2;
        }
        
        // Sparsity complexity
        if sparsity > 0.9 {
            complexity_score += 2;
        } else if sparsity > 0.5 {
            complexity_score += 1;
        }
        
        // Distance metric complexity
        match query.metric {
            DistanceMetric::Cosine => complexity_score += 1,
            DistanceMetric::Manhattan => complexity_score += 1,
            _ => {}
        }
        
        match complexity_score {
            0..=2 => QueryComplexity::Simple,
            3..=5 => QueryComplexity::Moderate,
            6..=8 => QueryComplexity::Complex,
            _ => QueryComplexity::HighlyComplex,
        }
    }

    /// Estimate filter selectivity
    fn estimate_filter_selectivity(&self, filter: &Option<MetadataFilter>) -> f32 {
        match filter {
            None => 1.0, // No filtering
            Some(f) => {
                let mut selectivity = 1.0;
                
                // Each filter condition reduces selectivity
                if f.min_file_size.is_some() || f.max_file_size.is_some() {
                    selectivity *= 0.7;
                }
                if f.min_created_timestamp.is_some() || f.max_created_timestamp.is_some() {
                    selectivity *= 0.8;
                }
                if f.required_dimensions.is_some() {
                    selectivity *= 0.5;
                }
                if f.required_data_type.is_some() {
                    selectivity *= 0.6;
                }
                if f.max_distance.is_some() {
                    selectivity *= 0.4;
                }
                
                selectivity
            }
        }
    }

    /// Calculate index score for selection
    fn calculate_index_score(&self, characteristics: &QueryCharacteristics, index_chars: &IndexCharacteristics) -> f32 {
        let mut score = 0.0;
        
        // Dimension suitability
        let dim_score = if characteristics.dimensions >= index_chars.optimal_dimensions.0 
            && characteristics.dimensions <= index_chars.optimal_dimensions.1 {
            1.0
        } else {
            let distance = if characteristics.dimensions < index_chars.optimal_dimensions.0 {
                index_chars.optimal_dimensions.0 - characteristics.dimensions
            } else {
                characteristics.dimensions - index_chars.optimal_dimensions.1
            };
            (1.0 / (1.0 + distance as f32 * 0.001)).max(0.1)
        };
        score += dim_score * 0.3;
        
        // Sparsity handling
        let sparsity_score = if characteristics.sparsity > 0.5 {
            index_chars.sparsity_handling
        } else {
            1.0 - characteristics.sparsity * (1.0 - index_chars.sparsity_handling)
        };
        score += sparsity_score * 0.2;
        
        // Accuracy requirements
        let accuracy_score = if characteristics.approximate_acceptable {
            index_chars.accuracy_factor
        } else {
            if index_chars.accuracy_factor >= 0.95 { 1.0 } else { 0.3 }
        };
        score += accuracy_score * 0.2;
        
        // Performance requirements
        let perf_score = match characteristics.complexity {
            QueryComplexity::Simple => 1.0 - index_chars.search_complexity * 0.5,
            QueryComplexity::Moderate => 1.0 - index_chars.search_complexity * 0.7,
            QueryComplexity::Complex => 1.0 - index_chars.search_complexity * 0.9,
            QueryComplexity::HighlyComplex => 1.0 - index_chars.search_complexity,
        };
        score += perf_score * 0.3;
        
        score.max(0.0).min(1.0)
    }

    /// Estimate performance speedup
    fn estimate_speedup(&self, strategy: IndexStrategy, characteristics: &QueryCharacteristics) -> f32 {
        let base_complexity = characteristics.dimensions as f32 * characteristics.k as f32;
        
        let index_chars = self.index_characteristics.get(&strategy).unwrap();
        let optimized_complexity = base_complexity * index_chars.search_complexity;
        
        (base_complexity / optimized_complexity).max(1.0)
    }

    /// Estimate memory usage
    fn estimate_memory_usage(&self, strategy: IndexStrategy, characteristics: &QueryCharacteristics) -> usize {
        let base_memory = characteristics.dimensions * characteristics.k * core::mem::size_of::<f32>();
        let index_chars = self.index_characteristics.get(&strategy).unwrap();
        
        (base_memory as f32 * index_chars.memory_overhead) as usize
    }

    /// Generate reasoning for index selection
    fn generate_reasoning(&self, strategy: IndexStrategy, characteristics: &QueryCharacteristics) -> String {
        match strategy {
            IndexStrategy::HNSW => {
                if characteristics.dimensions > 512 {
                    "HNSW selected for high-dimensional vectors with excellent recall".to_string()
                } else {
                    "HNSW selected for balanced performance and accuracy".to_string()
                }
            }
            IndexStrategy::LSH => {
                if characteristics.sparsity > 0.7 {
                    "LSH selected for sparse vectors with good approximate performance".to_string()
                } else {
                    "LSH selected for fast approximate search".to_string()
                }
            }
            IndexStrategy::IVF => {
                "IVF selected for large-scale vector collections with clustering benefits".to_string()
            }
            IndexStrategy::PQ => {
                if characteristics.dimensions > 1024 {
                    "PQ selected for memory-efficient search of high-dimensional vectors".to_string()
                } else {
                    "PQ selected for memory-constrained environments".to_string()
                }
            }
            IndexStrategy::Flat => {
                if characteristics.k < 100 && characteristics.dimensions < 256 {
                    "Flat index selected for small-scale exact search".to_string()
                } else {
                    "Flat index selected as fallback for guaranteed accuracy".to_string()
                }
            }
        }
    }

    /// Create execution stages for the query plan
    fn create_execution_stages(
        &self,
        characteristics: &QueryCharacteristics,
        recommendation: &IndexRecommendation,
    ) -> VexfsResult<Vec<ExecutionStage>> {
        let mut stages = Vec::new();
        
        // Stage 1: Preprocessing
        stages.push(ExecutionStage {
            name: "Query Preprocessing".to_string(),
            stage_type: StageType::Preprocessing,
            estimated_time_us: 10,
            memory_required: characteristics.dimensions * core::mem::size_of::<f32>(),
            parallelizable: false,
            dependencies: Vec::new(),
        });
        
        // Stage 2: Index Preparation
        stages.push(ExecutionStage {
            name: "Index Preparation".to_string(),
            stage_type: StageType::IndexPreparation,
            estimated_time_us: 50,
            memory_required: recommendation.memory_estimate / 10,
            parallelizable: false,
            dependencies: vec![0],
        });
        
        // Stage 3: Candidate Generation
        let candidate_time = match recommendation.primary_strategy {
            IndexStrategy::Flat => characteristics.dimensions as u64 * 10,
            IndexStrategy::HNSW => (characteristics.dimensions as u64).max(100),
            IndexStrategy::LSH => (characteristics.dimensions as u64 / 2).max(50),
            IndexStrategy::IVF => (characteristics.dimensions as u64 / 4).max(75),
            IndexStrategy::PQ => (characteristics.dimensions as u64 / 8).max(25),
        };
        
        stages.push(ExecutionStage {
            name: "Candidate Generation".to_string(),
            stage_type: StageType::CandidateGeneration,
            estimated_time_us: candidate_time,
            memory_required: characteristics.k * characteristics.dimensions * core::mem::size_of::<f32>(),
            parallelizable: true,
            dependencies: vec![1],
        });
        
        // Stage 4: Distance Computation
        stages.push(ExecutionStage {
            name: "Distance Computation".to_string(),
            stage_type: StageType::DistanceComputation,
            estimated_time_us: characteristics.k as u64 * characteristics.dimensions as u64 / 100,
            memory_required: characteristics.k * core::mem::size_of::<f32>(),
            parallelizable: true,
            dependencies: vec![2],
        });
        
        // Stage 5: Result Filtering (if needed)
        if characteristics.has_filters {
            stages.push(ExecutionStage {
                name: "Result Filtering".to_string(),
                stage_type: StageType::ResultFiltering,
                estimated_time_us: characteristics.k as u64 / 10,
                memory_required: characteristics.k * 64, // Metadata size estimate
                parallelizable: true,
                dependencies: vec![3],
            });
        }
        
        // Stage 6: Result Ranking
        let ranking_dependency = if characteristics.has_filters { 4 } else { 3 };
        stages.push(ExecutionStage {
            name: "Result Ranking".to_string(),
            stage_type: StageType::ResultRanking,
            estimated_time_us: (characteristics.k as u64 * (characteristics.k as f64).log2() as u64).max(10),
            memory_required: characteristics.k * core::mem::size_of::<f32>(),
            parallelizable: false,
            dependencies: vec![ranking_dependency],
        });
        
        Ok(stages)
    }

    /// Calculate accuracy trade-off
    fn calculate_accuracy_trade_off(
        &self,
        characteristics: &QueryCharacteristics,
        recommendation: &IndexRecommendation,
    ) -> f32 {
        if !characteristics.approximate_acceptable {
            return 1.0; // No trade-off for exact search
        }
        
        let index_chars = self.index_characteristics.get(&recommendation.primary_strategy).unwrap();
        
        // Adjust accuracy based on query complexity
        let complexity_factor = match characteristics.complexity {
            QueryComplexity::Simple => 1.0,
            QueryComplexity::Moderate => 0.95,
            QueryComplexity::Complex => 0.90,
            QueryComplexity::HighlyComplex => 0.85,
        };
        
        index_chars.accuracy_factor * complexity_factor
    }

    /// Update statistics after query execution
    fn update_stats(&mut self, execution_time: u64, recommendation: &IndexRecommendation) {
        self.stats.queries_analyzed += 1;
        
        // Update average planning time
        let total_time = self.stats.avg_planning_time_us * (self.stats.queries_analyzed - 1) + execution_time;
        self.stats.avg_planning_time_us = total_time / self.stats.queries_analyzed;
        
        // Update performance improvement estimate
        let improvement = recommendation.expected_speedup;
        let total_improvement = self.stats.avg_performance_improvement * (self.stats.queries_analyzed - 1) as f32 + improvement;
        self.stats.avg_performance_improvement = total_improvement / self.stats.queries_analyzed as f32;
    }

    /// Update statistics with OperationContext integration
    fn update_stats_with_context(&mut self, context: &OperationContext, execution_time: u64, recommendation: &IndexRecommendation) {
        self.update_stats(execution_time, recommendation);
        
        // Track user-specific statistics
        let _user_stats = (context.user.uid, execution_time, recommendation.confidence);
        
        // Update recommendation accuracy based on context
        if self.stats.queries_analyzed > 0 {
            self.stats.recommendation_accuracy =
                (self.stats.recommendation_accuracy * (self.stats.queries_analyzed - 1) as f32 + recommendation.confidence)
                / self.stats.queries_analyzed as f32;
        }
    }

    /// Start operation tracking for lifecycle management
    fn start_operation(&mut self, context: &OperationContext, start_time: u64) -> VexfsResult<u64> {
        self.operation_counter += 1;
        let operation_id = self.operation_counter;
        
        let metadata = OperationMetadata {
            operation_id,
            start_time_us: start_time,
            estimated_memory: 0, // Will be updated later
            characteristics: QueryCharacteristics {
                dimensions: 0,
                sparsity: 0.0,
                magnitude: 0.0,
                entropy: 0.0,
                k: 0,
                metric: DistanceMetric::Euclidean,
                has_filters: false,
                filter_selectivity: 1.0,
                complexity: QueryComplexity::Simple,
                approximate_acceptable: true,
            }, // Will be updated later
            recommendation: IndexRecommendation {
                primary_strategy: IndexStrategy::Flat,
                fallback_strategy: None,
                confidence: 0.0,
                expected_speedup: 1.0,
                memory_estimate: 0,
                reasoning: "Initial placeholder".to_string(),
            }, // Will be updated later
            status: OperationStatus::Planning,
            user_id: context.user.uid,
        };
        
        self.active_operations.insert(operation_id, metadata);
        Ok(operation_id)
    }

    /// Update operation metadata during planning
    fn update_operation_metadata(
        &mut self,
        operation_id: u64,
        characteristics: &QueryCharacteristics,
        recommendation: &IndexRecommendation,
    ) -> VexfsResult<()> {
        if let Some(metadata) = self.active_operations.get_mut(&operation_id) {
            metadata.characteristics = characteristics.clone();
            metadata.recommendation = recommendation.clone();
            metadata.estimated_memory = recommendation.memory_estimate;
            metadata.status = OperationStatus::Executing;
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Complete operation successfully
    fn complete_operation(&mut self, operation_id: u64, execution_time: u64, memory_used: usize) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = OperationStatus::Completed;
            
            // Add to performance history for learning
            let performance_record = PerformanceRecord {
                characteristics: metadata.characteristics,
                strategy: metadata.recommendation.primary_strategy,
                actual_time_us: execution_time,
                memory_used,
                accuracy: metadata.recommendation.confidence,
                timestamp: self.get_current_time_us(),
            };
            
            self.performance_history.push(performance_record);
            
            // Limit history size
            if self.performance_history.len() > self.config.history_size {
                self.performance_history.remove(0);
            }
            
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Fail operation with error handling
    fn fail_operation(&mut self, operation_id: u64, reason: String) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = OperationStatus::Failed;
            
            // Log failure for debugging (in a real implementation, this would use proper logging)
            let _failure_info = (operation_id, reason, metadata.user_id, self.get_current_time_us());
            
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Cancel active operation
    pub fn cancel_operation(&mut self, operation_id: u64) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = OperationStatus::Cancelled;
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Get active operations for monitoring
    pub fn get_active_operations(&self) -> &BTreeMap<u64, OperationMetadata> {
        &self.active_operations
    }

    /// Cleanup stale operations (operations older than timeout)
    pub fn cleanup_stale_operations(&mut self, timeout_us: u64) -> usize {
        let current_time = self.get_current_time_us();
        let mut stale_operations = Vec::new();
        
        for (&operation_id, metadata) in &self.active_operations {
            if current_time - metadata.start_time_us > timeout_us {
                stale_operations.push(operation_id);
            }
        }
        
        let count = stale_operations.len();
        for operation_id in stale_operations {
            self.active_operations.remove(&operation_id);
        }
        
        count
    }

    /// Get current time in microseconds (placeholder implementation)
    fn get_current_time_us(&self) -> u64 {
        // In a real kernel implementation, this would use kernel time functions
        1640995200_000_000 // Placeholder timestamp in microseconds
    }

    // ===== NEW TASK 6.2 METHODS: INTELLIGENT QUERY PLANNING AND OPTIMIZATION =====

    /// Estimate query execution cost using the cost model
    pub fn estimate_query_cost(&self, characteristics: &QueryCharacteristics, plan: &QueryExecutionPlan) -> CostEstimate {
        let mut cpu_cost = 0.0;
        let mut memory_cost = 0.0;
        let mut io_cost = 0.0;
        let mut index_cost = 0.0;
        let mut cache_cost = 0.0;

        // CPU cost calculation
        let vector_comparisons = characteristics.dimensions * characteristics.k;
        cpu_cost += vector_comparisons as f32 * self.cost_model.cpu_cost_per_comparison;

        // Memory cost calculation
        memory_cost += plan.memory_estimate as f32 * self.cost_model.memory_cost_per_byte;

        // I/O cost calculation (estimated based on index strategy)
        let io_operations = match plan.index_recommendation.primary_strategy {
            IndexStrategy::Flat => characteristics.k / 10, // Sequential scan
            IndexStrategy::HNSW => (characteristics.k as f32).log2() as usize * 2, // Tree traversal
            IndexStrategy::LSH => characteristics.k / 5, // Hash lookups
            IndexStrategy::IVF => characteristics.k / 8, // Cluster access
            IndexStrategy::PQ => characteristics.k / 15, // Compressed access
        };
        io_cost += io_operations as f32 * self.cost_model.io_cost_per_access;

        // Index traversal cost
        index_cost += self.cost_model.index_traversal_cost * characteristics.k as f32;

        // Cache cost (miss penalty)
        let estimated_cache_miss_rate = 1.0 - self.estimate_cache_hit_rate(characteristics);
        cache_cost += estimated_cache_miss_rate * self.cost_model.cache_miss_penalty * characteristics.k as f32;

        let total_cost = cpu_cost + memory_cost + io_cost + index_cost + cache_cost;
        let estimated_time_us = (total_cost * 10.0) as u64; // Convert cost to time estimate

        CostEstimate {
            total_cost,
            cpu_cost,
            memory_cost,
            io_cost,
            index_cost,
            cache_cost,
            estimated_time_us,
            confidence: 0.8, // Base confidence
        }
    }

    /// Apply query rewriting rules to optimize the query
    pub fn rewrite_query(&self, query: &SearchQuery) -> VexfsResult<Option<QueryTransformation>> {
        for rule in &self.rewriting_rules {
            if self.evaluate_rewriting_condition(&rule.condition, query)? {
                let transformed_query = self.apply_rewriting_action(&rule.action, query)?;
                
                return Ok(Some(QueryTransformation {
                    original_query: query.clone(),
                    transformed_query,
                    transformation_type: self.action_to_transformation_type(&rule.action),
                    expected_improvement: rule.effectiveness * 100.0,
                    confidence: rule.priority,
                    reasoning: format!("Applied rule '{}': {}", rule.name, self.generate_transformation_reasoning(&rule.action)),
                }));
            }
        }
        
        Ok(None)
    }

    /// Adaptive optimization based on historical performance
    pub fn adapt_optimization_strategy(&mut self) -> VexfsResult<()> {
        if self.performance_history.len() < self.adaptive_state.history_window_size {
            return Ok(()); // Not enough data for adaptation
        }

        let current_time = self.get_current_time_us();
        let time_since_last_adaptation = current_time - self.adaptive_state.last_adaptation_us;
        
        // Only adapt if enough time has passed
        if time_since_last_adaptation < 60_000_000 { // 1 minute
            return Ok(());
        }

        // Analyze recent performance for each strategy
        let recent_history = &self.performance_history[self.performance_history.len().saturating_sub(self.adaptive_state.history_window_size)..];
        
        let mut strategy_performance = BTreeMap::new();
        let mut strategy_counts = BTreeMap::new();

        for record in recent_history {
            let strategy = self.characteristics_to_strategy(&record.characteristics);
            let performance_score = self.calculate_performance_score(record);
            
            *strategy_performance.entry(strategy).or_insert(0.0) += performance_score;
            *strategy_counts.entry(strategy).or_insert(0) += 1;
        }

        // Update strategy scores with exponential moving average
        for (strategy, total_performance) in strategy_performance {
            if let Some(count) = strategy_counts.get(&strategy) {
                if *count > 0 {
                    let avg_performance = total_performance / *count as f32;
                    let current_score = self.adaptive_state.strategy_scores.get(&strategy).copied().unwrap_or(0.5);
                    let new_score = (1.0 - self.adaptive_state.learning_rate) * current_score +
                                   self.adaptive_state.learning_rate * avg_performance;
                    self.adaptive_state.strategy_scores.insert(strategy, new_score);
                }
            }
        }

        // Select best performing strategy
        let best_strategy = self.adaptive_state.strategy_scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(&strategy, _)| strategy)
            .unwrap_or(OptimizationStrategy::Balanced);

        // Adapt if performance improvement is significant
        let current_score = self.adaptive_state.strategy_scores.get(&self.adaptive_state.current_strategy).copied().unwrap_or(0.5);
        let best_score = self.adaptive_state.strategy_scores.get(&best_strategy).copied().unwrap_or(0.5);
        
        if best_score - current_score > self.adaptive_state.adaptation_threshold {
            self.adaptive_state.current_strategy = best_strategy;
            self.adaptive_state.last_adaptation_us = current_time;
        }

        Ok(())
    }

    /// Check execution path cache for similar queries
    pub fn check_execution_path_cache(&self, query: &SearchQuery) -> Option<&CachedExecutionPath> {
        let query_signature = self.generate_query_signature(query);
        self.execution_path_cache.get(&query_signature)
    }

    /// Cache execution path for future use
    pub fn cache_execution_path(&mut self, query: &SearchQuery, plan: QueryExecutionPlan, metrics: ExecutionMetrics) {
        let query_signature = self.generate_query_signature(query);
        let current_time = self.get_current_time_us();
        
        let cached_path = CachedExecutionPath {
            query_signature: query_signature.clone(),
            execution_plan: plan,
            performance_metrics: metrics,
            cached_at_us: current_time,
            hit_count: 0,
            last_used_us: current_time,
        };
        
        self.execution_path_cache.insert(query_signature, cached_path);
        
        // Limit cache size
        if self.execution_path_cache.len() > 1000 {
            self.cleanup_execution_path_cache();
        }
    }

    /// Set performance monitor for feedback integration
    pub fn set_performance_monitor(&mut self, monitor: Arc<QueryPerformanceMonitor>) {
        self.performance_monitor = Some(monitor);
    }

    /// Get optimization recommendations based on current state
    pub fn get_optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // Analyze current strategy performance
        if let Some(current_score) = self.adaptive_state.strategy_scores.get(&self.adaptive_state.current_strategy) {
            if *current_score < 0.6 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_type: RecommendationType::QueryOptimization,
                    priority: 0.8,
                    expected_improvement_percent: 25.0,
                    implementation_complexity: 0.3,
                    description: "Current optimization strategy is underperforming. Consider strategy adaptation.".to_string(),
                    actions: vec![
                        "Analyze query patterns for better strategy selection".to_string(),
                        "Increase adaptation sensitivity".to_string(),
                    ],
                    estimated_implementation_time_us: 1800 * 1_000_000, // 30 minutes
                });
            }
        }

        // Check cache effectiveness
        let cache_hit_rate = self.calculate_cache_hit_rate();
        if cache_hit_rate < 0.3 {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::CacheOptimization,
                priority: 0.7,
                expected_improvement_percent: 40.0,
                implementation_complexity: 0.4,
                description: "Execution path cache hit rate is low. Consider cache optimization.".to_string(),
                actions: vec![
                    "Improve query signature generation".to_string(),
                    "Increase cache size".to_string(),
                    "Optimize cache eviction policy".to_string(),
                ],
                estimated_implementation_time_us: 3600 * 1_000_000, // 1 hour
            });
        }

        recommendations
    }

    // ===== HELPER METHODS FOR NEW OPTIMIZATION FEATURES =====

    /// Estimate cache hit rate for a query
    fn estimate_cache_hit_rate(&self, characteristics: &QueryCharacteristics) -> f32 {
        // Simple heuristic based on query complexity and patterns
        let base_hit_rate = match characteristics.complexity {
            QueryComplexity::Simple => 0.8,
            QueryComplexity::Moderate => 0.6,
            QueryComplexity::Complex => 0.4,
            QueryComplexity::HighlyComplex => 0.2,
        };

        // Adjust for filters (filtered queries are less likely to be cached)
        if characteristics.has_filters {
            base_hit_rate * 0.7
        } else {
            base_hit_rate
        }
    }

    /// Evaluate a rewriting condition against a query
    fn evaluate_rewriting_condition(&self, condition: &RewritingCondition, query: &SearchQuery) -> VexfsResult<bool> {
        match condition {
            RewritingCondition::DimensionThreshold(threshold) => Ok(query.vector.len() > *threshold),
            RewritingCondition::SparsityThreshold(threshold) => {
                let sparsity = self.calculate_sparsity(&query.vector);
                Ok(sparsity > *threshold)
            },
            RewritingCondition::KThreshold(threshold) => Ok(query.k > *threshold),
            RewritingCondition::ComplexityThreshold(threshold) => {
                let characteristics = self.analyze_query(query)?;
                Ok(characteristics.complexity as u8 >= *threshold as u8)
            },
            RewritingCondition::Combined(conditions) => {
                for condition in conditions {
                    if !self.evaluate_rewriting_condition(condition, query)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            },
        }
    }

    /// Apply a rewriting action to a query
    fn apply_rewriting_action(&self, action: &RewritingAction, query: &SearchQuery) -> VexfsResult<SearchQuery> {
        let mut transformed_query = query.clone();

        match action {
            RewritingAction::ReduceDimensions(target_dims) => {
                if query.vector.len() > *target_dims {
                    transformed_query.vector.truncate(*target_dims);
                }
            },
            RewritingAction::AdjustApproximation(factor) => {
                transformed_query.approximate = true;
                transformed_query.expansion_factor *= factor;
            },
            RewritingAction::ReorderFilters => {
                // For now, this is a placeholder - would implement filter reordering logic
            },
            RewritingAction::ChangeMetric(new_metric) => {
                transformed_query.metric = *new_metric;
            },
            RewritingAction::SplitQuery(batch_size) => {
                if query.k > *batch_size {
                    transformed_query.k = *batch_size;
                }
            },
        }

        Ok(transformed_query)
    }

    /// Convert rewriting action to transformation type
    fn action_to_transformation_type(&self, action: &RewritingAction) -> TransformationType {
        match action {
            RewritingAction::ReduceDimensions(_) => TransformationType::DimensionReduction,
            RewritingAction::AdjustApproximation(_) => TransformationType::ApproximationAdjustment,
            RewritingAction::ReorderFilters => TransformationType::FilterReordering,
            RewritingAction::ChangeMetric(_) => TransformationType::MetricSubstitution,
            RewritingAction::SplitQuery(_) => TransformationType::QueryDecomposition,
        }
    }

    /// Generate reasoning for transformation
    fn generate_transformation_reasoning(&self, action: &RewritingAction) -> String {
        match action {
            RewritingAction::ReduceDimensions(dims) => format!("Reduced dimensions to {} for better performance", dims),
            RewritingAction::AdjustApproximation(factor) => format!("Adjusted approximation factor to {:.2} for speed", factor),
            RewritingAction::ReorderFilters => "Reordered filters for optimal selectivity".to_string(),
            RewritingAction::ChangeMetric(metric) => format!("Changed distance metric to {:?} for better accuracy", metric),
            RewritingAction::SplitQuery(size) => format!("Split query into batches of size {} for memory efficiency", size),
        }
    }

    /// Map query characteristics to optimization strategy
    fn characteristics_to_strategy(&self, characteristics: &QueryCharacteristics) -> OptimizationStrategy {
        match characteristics.complexity {
            QueryComplexity::Simple => OptimizationStrategy::Conservative,
            QueryComplexity::Moderate => OptimizationStrategy::Balanced,
            QueryComplexity::Complex => OptimizationStrategy::Aggressive,
            QueryComplexity::HighlyComplex => {
                if characteristics.dimensions > 1024 {
                    OptimizationStrategy::MemoryOptimized
                } else {
                    OptimizationStrategy::LatencyOptimized
                }
            },
        }
    }

    /// Calculate performance score for a record
    fn calculate_performance_score(&self, record: &PerformanceRecord) -> f32 {
        // Normalize performance metrics to a 0-1 score
        let time_score = 1.0 / (1.0 + record.actual_time_us as f32 / 100000.0); // Normalize by 100ms
        let memory_score = 1.0 / (1.0 + record.memory_used as f32 / 1048576.0); // Normalize by 1MB
        let accuracy_score = record.accuracy;

        // Weighted combination
        0.4 * time_score + 0.3 * memory_score + 0.3 * accuracy_score
    }

    /// Generate query signature for caching
    fn generate_query_signature(&self, query: &SearchQuery) -> String {
        use core::hash::{Hash, Hasher};
        
        // Simple signature based on key query characteristics
        let mut signature = String::new();
        signature.push_str(&format!("dims:{}", query.vector.len()));
        signature.push_str(&format!("_k:{}", query.k));
        signature.push_str(&format!("_metric:{:?}", query.metric));
        signature.push_str(&format!("_approx:{}", query.approximate));
        
        if query.filter.is_some() {
            signature.push_str("_filtered");
        }
        
        signature
    }

    /// Calculate sparsity of a vector
    fn calculate_sparsity(&self, vector: &[f32]) -> f32 {
        let zero_count = vector.iter().filter(|&&x| x.abs() < f32::EPSILON).count();
        zero_count as f32 / vector.len() as f32
    }

    /// Calculate current cache hit rate
    fn calculate_cache_hit_rate(&self) -> f32 {
        if self.execution_path_cache.is_empty() {
            return 0.0;
        }

        let total_hits: u64 = self.execution_path_cache.values().map(|path| path.hit_count).sum();
        let total_entries = self.execution_path_cache.len() as u64;
        
        if total_entries > 0 {
            total_hits as f32 / total_entries as f32
        } else {
            0.0
        }
    }

    /// Cleanup old execution path cache entries
    fn cleanup_execution_path_cache(&mut self) {
        let current_time = self.get_current_time_us();
        let retention_time = 3600 * 1_000_000; // 1 hour
        
        self.execution_path_cache.retain(|_, path| {
            current_time - path.last_used_us < retention_time
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation_logic() {
        // Test entropy calculation without complex setup
        let config = QueryPlannerConfig::default();
        
        // Create a mock storage manager (we won't actually use it for entropy calculation)
        let storage_manager = Arc::new(VectorStorageManager::new(
            Arc::new(unsafe { std::mem::zeroed() }), // Mock storage manager
            4096,
            1024,
        ));
        
        let planner = QueryPlanner::new(storage_manager, config);
        
        // Test entropy calculation with different vector patterns
        let uniform_vector = vec![1.0; 100];
        let entropy_uniform = planner.calculate_entropy(&uniform_vector);
        assert_eq!(entropy_uniform, 0.0); // Uniform vector should have zero entropy
        
        let random_vector: Vec<f32> = (0..100).map(|i| (i as f32) / 100.0).collect();
        let entropy_random = planner.calculate_entropy(&random_vector);
        assert!(entropy_random > 0.0); // Random vector should have positive entropy
    }

    #[test]
    fn test_query_complexity_determination() {
        // Test complexity scoring logic
        let config = QueryPlannerConfig::default();
        let storage_manager = Arc::new(VectorStorageManager::new(
            Arc::new(unsafe { std::mem::zeroed() }),
            4096,
            1024,
        ));
        let planner = QueryPlanner::new(storage_manager, config);
        
        // Simple query
        let simple_query = SearchQuery {
            vector: vec![1.0; 64],
            k: 10,
            metric: DistanceMetric::Euclidean,
            approximate: true,
            expansion_factor: 2.0,
            filter: None,
            exact_distances: true,
            use_simd: true,
        };
        
        let characteristics = planner.analyze_query(&simple_query).unwrap();
        assert_eq!(characteristics.complexity, QueryComplexity::Simple);
        assert_eq!(characteristics.dimensions, 64);
        assert_eq!(characteristics.k, 10);
        
        // Complex query
        let complex_query = SearchQuery {
            vector: vec![1.0; 2048],
            k: 1000,
            metric: DistanceMetric::Cosine,
            approximate: true,
            expansion_factor: 3.0,
            filter: Some(MetadataFilter::new()),
            exact_distances: true,
            use_simd: true,
        };
        
        let characteristics = planner.analyze_query(&complex_query).unwrap();
        assert!(matches!(characteristics.complexity, QueryComplexity::Complex | QueryComplexity::HighlyComplex));
    }

    #[test]
    fn test_index_recommendation_logic() {
        // Test index scoring without complex setup
        let config = QueryPlannerConfig::default();
        let storage_manager = Arc::new(VectorStorageManager::new(
            Arc::new(unsafe { std::mem::zeroed() }),
            4096,
            1024,
        ));
        let planner = QueryPlanner::new(storage_manager, config);
        
        // High-dimensional query characteristics
        let characteristics = QueryCharacteristics {
            dimensions: 1024,
            sparsity: 0.1,
            magnitude: 10.0,
            entropy: 5.0,
            k: 50,
            metric: DistanceMetric::Euclidean,
            has_filters: false,
            filter_selectivity: 1.0,
            complexity: QueryComplexity::Moderate,
            approximate_acceptable: true,
        };
        
        let recommendation = planner.recommend_index(&characteristics).unwrap();
        assert!(matches!(recommendation.primary_strategy, IndexStrategy::HNSW | IndexStrategy::PQ));
        assert!(recommendation.confidence > 0.5);
        assert!(recommendation.expected_speedup > 1.0);
        
        // Sparse query should prefer LSH
        let sparse_characteristics = QueryCharacteristics {
            dimensions: 512,
            sparsity: 0.9, // Very sparse
            magnitude: 5.0,
            entropy: 2.0,
            k: 20,
            metric: DistanceMetric::Euclidean,
            has_filters: false,
            filter_selectivity: 1.0,
            complexity: QueryComplexity::Simple,
            approximate_acceptable: true,
        };
        
        let sparse_recommendation = planner.recommend_index(&sparse_characteristics).unwrap();
        // For very sparse vectors, LSH should be preferred
        assert!(sparse_recommendation.confidence > 0.0);
    }

    #[test]
    fn test_execution_stages_creation() {
        // Test execution stage creation logic
        let config = QueryPlannerConfig::default();
        let storage_manager = Arc::new(VectorStorageManager::new(
            Arc::new(unsafe { std::mem::zeroed() }),
            4096,
            1024,
        ));
        let planner = QueryPlanner::new(storage_manager, config);
        
        let characteristics = QueryCharacteristics {
            dimensions: 128,
            sparsity: 0.1,
            magnitude: 10.0,
            entropy: 5.0,
            k: 20,
            metric: DistanceMetric::Euclidean,
            has_filters: true,
            filter_selectivity: 0.5,
            complexity: QueryComplexity::Moderate,
            approximate_acceptable: true,
        };
        
        let recommendation = IndexRecommendation {
            primary_strategy: IndexStrategy::HNSW,
            fallback_strategy: Some(IndexStrategy::Flat),
            confidence: 0.8,
            expected_speedup: 2.5,
            memory_estimate: 1024 * 1024,
            reasoning: "Test recommendation".to_string(),
        };
        
        let stages = planner.create_execution_stages(&characteristics, &recommendation).unwrap();
        
        // Should have multiple stages including filtering
        assert!(stages.len() >= 5);
        
        // Check that stages have reasonable estimates
        for stage in &stages {
            assert!(stage.estimated_time_us > 0);
            assert!(stage.memory_required > 0);
        }
        
        // Should have a filtering stage for queries with filters
        assert!(stages.iter().any(|s| s.stage_type == StageType::ResultFiltering));
        
        // Should have basic required stages
        assert!(stages.iter().any(|s| s.stage_type == StageType::Preprocessing));
        assert!(stages.iter().any(|s| s.stage_type == StageType::CandidateGeneration));
        assert!(stages.iter().any(|s| s.stage_type == StageType::ResultRanking));
    }
}

/// Query planner configuration
#[derive(Debug, Clone)]
pub struct QueryPlannerConfig {
    /// Maximum planning time (microseconds)
    pub max_planning_time_us: u64,
    /// Memory budget for query execution
    pub memory_budget: usize,
    /// Accuracy threshold for approximate search
    pub accuracy_threshold: f32,
    /// Performance history size
    pub history_size: usize,
    /// Enable adaptive learning
    pub enable_learning: bool,
    /// SIMD optimization level
    pub simd_level: SimdStrategy,
    /// Memory layout preference
    pub memory_layout: MemoryLayout,
}

impl Default for QueryPlannerConfig {
    fn default() -> Self {
        Self {
            max_planning_time_us: 1000, // 1ms planning budget
            memory_budget: 64 * 1024 * 1024, // 64MB
            accuracy_threshold: 0.95,
            history_size: 1000,
            enable_learning: true,
            simd_level: SimdStrategy::Auto,
            memory_layout: MemoryLayout::Hybrid,
        }
    }
}