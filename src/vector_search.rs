//! Vector search and retrieval for VexFS
//!
//! This module implements the user-facing vector search interface leveraging the ANNS infrastructure,
//! providing similarity metrics, query processing, and result filtering.



extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap};
use core::cmp::Ordering;

use crate::anns::{DistanceMetric, AnnsIndex, SearchResult};
use crate::vector_metrics::{VectorMetrics, MetricsError};
use crate::vector_storage::{VectorStorageManager, VectorHeader, VectorDataType};
use crate::knn_search::{KnnSearchEngine, KnnResult, SearchParams, MetadataFilter, KnnError};
use crate::result_scoring::{ResultScorer, ScoringParams, ScoredResult, ScoringError};
use crate::fs_core::operations::OperationContext;
use crate::shared::errors::{VexfsError, SearchErrorKind};
use crate::search_cache::{SearchResultCache, CacheConfig, CacheKey, CacheStatistics};
use crate::query_planner::{QueryCharacteristics, IndexRecommendation};

/// Maximum number of results that can be returned from a search
pub const MAX_SEARCH_RESULTS: usize = 10000;

/// Maximum batch size for search requests
pub const MAX_BATCH_SIZE: usize = 100;

/// Vector search error types (maintained for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchError {
    /// Invalid query parameters
    InvalidQuery,
    /// Storage access error
    StorageError,
    /// k-NN search error
    KnnError(KnnError),
    /// Metrics calculation error
    MetricsError(MetricsError),
    /// Scoring error
    ScoringError(ScoringError),
    /// Index not available
    IndexNotAvailable,
    /// Result processing error
    ResultProcessingError,
    /// Memory allocation error
    AllocationError,
    /// Invalid batch size
    InvalidBatchSize,
}

impl From<KnnError> for SearchError {
    fn from(error: KnnError) -> Self {
        SearchError::KnnError(error)
    }
}

impl From<MetricsError> for SearchError {
    fn from(error: MetricsError) -> Self {
        SearchError::MetricsError(error)
    }
}

impl From<ScoringError> for SearchError {
    fn from(error: ScoringError) -> Self {
        SearchError::ScoringError(error)
    }
}

impl From<VexfsError> for SearchError {
    fn from(error: VexfsError) -> Self {
        match error {
            VexfsError::SearchError(SearchErrorKind::InvalidQuery) => SearchError::InvalidQuery,
            VexfsError::SearchError(SearchErrorKind::NoResults) => SearchError::ResultProcessingError,
            VexfsError::OutOfMemory => SearchError::AllocationError,
            _ => SearchError::StorageError,
        }
    }
}

impl From<SearchError> for VexfsError {
    fn from(error: SearchError) -> Self {
        match error {
            SearchError::InvalidQuery => VexfsError::SearchError(SearchErrorKind::InvalidQuery),
            SearchError::StorageError => VexfsError::SearchError(SearchErrorKind::InvalidQuery),
            SearchError::IndexNotAvailable => VexfsError::SearchError(SearchErrorKind::InvalidQuery),
            SearchError::ResultProcessingError => VexfsError::SearchError(SearchErrorKind::NoResults),
            SearchError::AllocationError => VexfsError::OutOfMemory,
            SearchError::InvalidBatchSize => VexfsError::SearchError(SearchErrorKind::InvalidQuery),
            _ => VexfsError::SearchError(SearchErrorKind::InvalidQuery),
        }
    }
}

/// Search query with all parameters
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Query vector
    pub vector: Vec<f32>,
    /// Number of results to return
    pub k: usize,
    /// Distance metric to use
    pub metric: DistanceMetric,
    /// Whether to use approximate search
    pub approximate: bool,
    /// Search expansion factor for approximate search
    pub expansion_factor: f32,
    /// Metadata filter
    pub filter: Option<MetadataFilter>,
    /// Whether to include exact distances
    pub exact_distances: bool,
    /// Whether to use SIMD optimizations
    pub use_simd: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            vector: Vec::new(),
            k: 10,
            metric: DistanceMetric::Euclidean,
            approximate: true,
            expansion_factor: 2.0,
            filter: None,
            exact_distances: true,
            use_simd: true,
        }
    }
}

/// Batch search request
#[derive(Debug, Clone)]
pub struct BatchSearchRequest {
    /// Multiple queries to execute
    pub queries: Vec<SearchQuery>,
    /// Whether to merge results across queries
    pub merge_results: bool,
    /// Maximum total results to return if merging
    pub max_total_results: usize,
}

/// Advanced search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Enable result caching
    pub enable_caching: bool,
    /// Cache expiration time in seconds
    pub cache_expiration: u64,
    /// Enable search result validation
    pub validate_results: bool,
    /// Enable search analytics
    pub enable_analytics: bool,
    /// Custom scoring parameters
    pub scoring_params: Option<ScoringParams>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            enable_caching: false,
            cache_expiration: 300, // 5 minutes
            validate_results: true,
            enable_analytics: false,
            scoring_params: None,
        }
    }
}

/// Search analytics and statistics
#[derive(Debug, Clone, Default)]
pub struct SearchAnalytics {
    /// Total search time in microseconds
    pub search_time_us: u64,
    /// Number of vectors examined
    pub vectors_examined: usize,
    /// Cache hit rate (if caching enabled)
    pub cache_hit_rate: f32,
    /// Index utilization rate
    pub index_utilization: f32,
    /// Average result confidence
    pub avg_confidence: f32,
    /// Result distribution statistics
    pub result_distribution: ResultDistribution,
}

/// Result distribution statistics
#[derive(Debug, Clone, Default)]
pub struct ResultDistribution {
    /// Distance statistics
    pub distance_stats: DistanceStats,
    /// File type distribution
    pub file_type_distribution: BTreeMap<String, usize>,
    /// Size distribution buckets
    pub size_distribution: [usize; 5], // Small, Medium, Large, XLarge, XXLarge
}

/// Distance statistics
#[derive(Debug, Clone, Default)]
pub struct DistanceStats {
    /// Minimum distance
    pub min: f32,
    /// Maximum distance
    pub max: f32,
    /// Mean distance
    pub mean: f32,
    /// Standard deviation
    pub std_dev: f32,
    /// Median distance
    pub median: f32,
}

/// Search operation metadata for lifecycle tracking
#[derive(Debug, Clone)]
struct SearchOperationMetadata {
    /// Operation ID
    operation_id: u64,
    /// Operation start time (microseconds)
    start_time_us: u64,
    /// Query characteristics
    query_size: usize,
    /// Requested result count
    k: usize,
    /// Estimated memory usage
    estimated_memory: usize,
    /// Operation status
    status: SearchOperationStatus,
    /// User ID for permission tracking
    user_id: u32,
    /// Search type (single or batch)
    search_type: SearchType,
}

/// Search operation status for lifecycle management
#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchOperationStatus {
    /// Operation is starting
    Starting,
    /// Operation is executing search
    Searching,
    /// Operation is processing results
    ProcessingResults,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}

/// Search type for operation tracking
#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchType {
    /// Single vector search
    Single,
    /// Batch vector search
    Batch,
}

/// Resource usage tracker for performance monitoring
#[derive(Debug, Clone, Default)]
struct ResourceTracker {
    /// Total memory allocated (bytes)
    total_memory_allocated: usize,
    /// Peak memory usage (bytes)
    peak_memory_usage: usize,
    /// Total operations processed
    total_operations: u64,
    /// Failed operations count
    failed_operations: u64,
    /// Average operation time (microseconds)
    avg_operation_time_us: u64,
}

/// Main vector search engine with comprehensive OperationContext integration
pub struct VectorSearchEngine {
    /// k-NN search engine
    knn_engine: KnnSearchEngine,
    /// Result scorer
    result_scorer: ResultScorer,
    /// Search options
    options: SearchOptions,
    /// Search analytics
    analytics: SearchAnalytics,
    /// Active search operations for lifecycle management
    active_operations: BTreeMap<u64, SearchOperationMetadata>,
    /// Operation counter for unique operation IDs
    operation_counter: u64,
    /// Resource usage tracking
    resource_tracker: ResourceTracker,
    /// Search result cache
    cache: Option<SearchResultCache>,
}

impl VectorSearchEngine {
    /// Create new vector search engine with comprehensive OperationContext integration
    pub fn new(storage: Box<VectorStorageManager>, options: SearchOptions) -> Result<Self, SearchError> {
        // Create a stub storage that implements the VectorStorage trait
        let stub_storage = Box::new(crate::vector_handlers::StubVectorStorage);
        let knn_engine = KnnSearchEngine::new(stub_storage)?;
        let scoring_params = options.scoring_params.clone().unwrap_or_default();
        let result_scorer = ResultScorer::new(scoring_params);
        
        // Initialize cache if enabled
        let cache = if options.enable_caching {
            let cache_config = CacheConfig::default();
            Some(SearchResultCache::new(cache_config))
        } else {
            None
        };
        
        Ok(Self {
            knn_engine,
            result_scorer,
            options,
            analytics: SearchAnalytics::default(),
            active_operations: BTreeMap::new(),
            operation_counter: 0,
            resource_tracker: ResourceTracker::default(),
            cache,
        })
    }
    
    /// Set HNSW index for approximate search
    pub fn set_hnsw_index(&mut self, index: AnnsIndex) {
        self.knn_engine.set_hnsw_index(index);
    }
    
    /// Perform vector search with comprehensive OperationContext integration
    pub fn search(&mut self, context: &mut OperationContext, query: SearchQuery) -> Result<Vec<ScoredResult>, SearchError> {
        let start_time = self.get_current_time_us();
        
        // Start operation tracking for lifecycle management
        let operation_id = self.start_search_operation(context, &query, SearchType::Single, start_time)?;
        
        // Validate query with error recovery
        if query.vector.is_empty() || query.k == 0 || query.k > MAX_SEARCH_RESULTS {
            self.fail_search_operation(operation_id, "Invalid query parameters".to_string())?;
            return Err(SearchError::InvalidQuery);
        }
        
        // Check cache first if enabled
        if let Some(ref mut cache) = self.cache {
            let cache_key = crate::search_cache::CacheKey::from_query(&query);
            if let Ok(Some(cached_results)) = cache.lookup(context, &cache_key) {
                // Cache hit - complete operation and return cached results
                let end_time = self.get_current_time_us();
                self.complete_search_operation(operation_id, end_time - start_time, 0)?;
                return Ok(cached_results);
            }
        }
        
        // Estimate and track resource usage
        let estimated_memory = self.estimate_search_memory(&query);
        self.update_operation_memory(operation_id, estimated_memory)?;
        
        // Check resource constraints
        if estimated_memory > 64 * 1024 * 1024 { // 64MB limit
            self.fail_search_operation(operation_id, "Memory limit exceeded".to_string())?;
            return Err(SearchError::AllocationError);
        }
        
        // Update operation status to searching
        self.update_operation_status(operation_id, SearchOperationStatus::Searching)?;
        
        // Convert to k-NN search parameters
        let search_params = SearchParams {
            k: query.k,
            metric: query.metric,
            expansion_factor: query.expansion_factor,
            approximate: query.approximate,
            use_simd: query.use_simd,
            filter: query.filter.clone(),
            exact_distances: query.exact_distances,
        };
        
        // Perform k-NN search with error handling
        let knn_results = match self.knn_engine.search(context, &query.vector, &search_params) {
            Ok(results) => results,
            Err(e) => {
                self.fail_search_operation(operation_id, "k-NN search failed".to_string())?;
                return Err(e.into());
            }
        };
        
        // Update operation status to processing results
        self.update_operation_status(operation_id, SearchOperationStatus::ProcessingResults)?;
        
        // Score and rank results with error handling
        let results = match self.result_scorer.score_and_rank(&knn_results, &search_params) {
            Ok(results) => results,
            Err(e) => {
                self.fail_search_operation(operation_id, "Result scoring failed".to_string())?;
                return Err(e.into());
            }
        };
        
        // Update analytics with operation context
        let end_time = self.get_current_time_us();
        let operation_time = end_time - start_time;
        self.analytics.search_time_us = operation_time;
        self.analytics.vectors_examined = results.len();
        
        // Validate results if enabled
        let final_results = if self.options.validate_results {
            match self.validate_results(results) {
                Ok(results) => results,
                Err(e) => {
                    self.fail_search_operation(operation_id, "Result validation failed".to_string())?;
                    return Err(e);
                }
            }
        } else {
            results
        };
        
        // Cache results if cache is enabled
        if let Some(cache) = &mut self.cache {
            let cache_key = crate::search_cache::CacheKey::from_query(&query);
            
            // Create query characteristics for cache decision
            let sparsity = {
                let zero_count = query.vector.iter().filter(|&&x| x == 0.0).count();
                zero_count as f32 / query.vector.len() as f32
            };
            let magnitude = query.vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let entropy = {
                // Simplified entropy calculation
                let mut histogram = [0u32; 10];
                let max_val = query.vector.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
                if max_val > 0.0 {
                    for &val in &query.vector {
                        let normalized = (val.abs() / max_val * 9.0) as usize;
                        let bin = normalized.min(9);
                        histogram[bin] += 1;
                    }
                    let total = query.vector.len() as f32;
                    let mut entropy = 0.0;
                    for &count in &histogram {
                        if count > 0 {
                            let p = count as f32 / total;
                            entropy -= p * p.log2();
                        }
                    }
                    entropy
                } else {
                    0.0
                }
            };
            let complexity = {
                let dimension_factor = if query.vector.len() > 512 { 2.0 } else if query.vector.len() > 128 { 1.5 } else { 1.0 };
                let k_factor = if query.k > 100 { 2.0 } else if query.k > 50 { 1.5 } else { 1.0 };
                let filter_factor = if query.filter.is_some() { 1.5 } else { 1.0 };
                let approx_factor = if query.approximate { 0.8 } else { 1.2 };
                let complexity_score = dimension_factor * k_factor * filter_factor * approx_factor;
                
                if complexity_score >= 4.0 {
                    crate::query_planner::QueryComplexity::HighlyComplex
                } else if complexity_score >= 2.5 {
                    crate::query_planner::QueryComplexity::Complex
                } else if complexity_score >= 1.5 {
                    crate::query_planner::QueryComplexity::Moderate
                } else {
                    crate::query_planner::QueryComplexity::Simple
                }
            };
            
            let query_characteristics = crate::query_planner::QueryCharacteristics {
                dimensions: query.vector.len(),
                sparsity,
                magnitude,
                entropy,
                k: query.k,
                metric: query.metric,
                has_filters: query.filter.is_some(),
                filter_selectivity: 1.0, // Default selectivity
                complexity,
                approximate_acceptable: query.approximate,
            };
            
            // Create index recommendation for cache
            let index_recommendation = crate::query_planner::IndexRecommendation {
                primary_strategy: crate::anns::IndexStrategy::HNSW,
                fallback_strategy: None,
                confidence: 0.8,
                expected_speedup: 2.0,
                memory_estimate: estimated_memory,
                reasoning: "Vector search result".to_string(),
            };
            
            // Insert into cache (ignore errors to not affect search performance)
            let _ = cache.insert(context, cache_key, final_results.clone(), query_characteristics, index_recommendation);
        }
        
        // Complete operation successfully
        self.complete_search_operation(operation_id, operation_time, estimated_memory)?;
        
        // Update resource tracker
        self.update_resource_tracker(operation_time, estimated_memory, true);
        
        Ok(final_results)
    }
    
    /// Perform batch search with comprehensive OperationContext integration
    pub fn batch_search(&mut self, context: &mut OperationContext, batch: BatchSearchRequest) -> Result<Vec<Vec<ScoredResult>>, SearchError> {
        let start_time = self.get_current_time_us();
        
        // Validate batch request
        if batch.queries.is_empty() || batch.queries.len() > MAX_BATCH_SIZE {
            return Err(SearchError::InvalidBatchSize);
        }
        
        // Create a dummy query for operation tracking (using first query characteristics)
        let dummy_query = batch.queries.first().unwrap().clone();
        let operation_id = self.start_search_operation(context, &dummy_query, SearchType::Batch, start_time)?;
        
        // Estimate total memory usage for batch
        let total_estimated_memory: usize = batch.queries.iter()
            .map(|q| self.estimate_search_memory(q))
            .sum();
        
        self.update_operation_memory(operation_id, total_estimated_memory)?;
        
        // Check resource constraints for batch
        if total_estimated_memory > 256 * 1024 * 1024 { // 256MB limit for batch
            self.fail_search_operation(operation_id, "Batch memory limit exceeded".to_string())?;
            return Err(SearchError::AllocationError);
        }
        
        self.update_operation_status(operation_id, SearchOperationStatus::Searching)?;
        
        let mut all_results = Vec::with_capacity(batch.queries.len());
        let mut successful_queries = 0;
        
        // Process each query in the batch
        for (i, query) in batch.queries.into_iter().enumerate() {
            match self.search(context, query) {
                Ok(results) => {
                    all_results.push(results);
                    successful_queries += 1;
                }
                Err(e) => {
                    // Log individual query failure but continue with batch
                    let _query_failure = (i, e, operation_id);
                    all_results.push(Vec::new()); // Empty results for failed query
                }
            }
        }
        
        // Check if any queries succeeded
        if successful_queries == 0 {
            self.fail_search_operation(operation_id, "All batch queries failed".to_string())?;
            return Err(SearchError::ResultProcessingError);
        }
        
        self.update_operation_status(operation_id, SearchOperationStatus::ProcessingResults)?;
        
        let final_results = if batch.merge_results {
            // Merge and re-rank results
            match self.merge_batch_results(all_results, batch.max_total_results) {
                Ok(merged) => vec![merged],
                Err(e) => {
                    self.fail_search_operation(operation_id, "Batch merge failed".to_string())?;
                    return Err(e);
                }
            }
        } else {
            all_results
        };
        
        // Complete batch operation
        let end_time = self.get_current_time_us();
        let operation_time = end_time - start_time;
        self.complete_search_operation(operation_id, operation_time, total_estimated_memory)?;
        
        // Update resource tracker for batch operation
        self.update_resource_tracker(operation_time, total_estimated_memory, true);
        
        Ok(final_results)
    }
    
    /// Merge results from multiple searches
    fn merge_batch_results(
        &mut self,
        result_sets: Vec<Vec<ScoredResult>>,
        max_results: usize,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        // Simple merge by taking the best results from each set
        let mut all_results = Vec::new();
        
        for results in result_sets {
            all_results.extend(results);
        }
        
        // Sort by relevance score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        // Deduplicate by vector ID
        let mut seen_ids = BTreeMap::new();
        let mut deduped = Vec::new();
        
        for result in all_results {
            if !seen_ids.contains_key(&result.result.vector_id) && deduped.len() < max_results {
                seen_ids.insert(result.result.vector_id, true);
                deduped.push(result);
            }
        }
        
        Ok(deduped)
    }
    
    /// Validate search results
    fn validate_results(&self, mut results: Vec<ScoredResult>) -> Result<Vec<ScoredResult>, SearchError> {
        // Remove invalid results
        results.retain(|r| {
            r.confidence > 0.1 // Minimum confidence threshold
                && r.result.distance.is_finite()
                && r.score.is_finite()
        });
        
        Ok(results)
    }
    
    /// Get current time in microseconds (placeholder implementation)
    fn get_current_time_us(&self) -> u64 {
        // In a real kernel implementation, this would use kernel time functions
        1640995200_000_000 // Placeholder timestamp in microseconds
    }
    
    /// Get search analytics
    pub fn get_analytics(&self) -> &SearchAnalytics {
        &self.analytics
    }
    
    /// Reset analytics
    pub fn reset_analytics(&mut self) {
        self.analytics = SearchAnalytics::default();
        self.resource_tracker = ResourceTracker::default();
    }

    /// Start search operation tracking for lifecycle management
    fn start_search_operation(
        &mut self,
        context: &OperationContext,
        query: &SearchQuery,
        search_type: SearchType,
        start_time: u64,
    ) -> Result<u64, SearchError> {
        self.operation_counter += 1;
        let operation_id = self.operation_counter;
        
        let metadata = SearchOperationMetadata {
            operation_id,
            start_time_us: start_time,
            query_size: query.vector.len(),
            k: query.k,
            estimated_memory: 0, // Will be updated later
            status: SearchOperationStatus::Starting,
            user_id: context.user.uid,
            search_type,
        };
        
        self.active_operations.insert(operation_id, metadata);
        Ok(operation_id)
    }

    /// Update operation memory estimate
    fn update_operation_memory(&mut self, operation_id: u64, memory: usize) -> Result<(), SearchError> {
        if let Some(metadata) = self.active_operations.get_mut(&operation_id) {
            metadata.estimated_memory = memory;
            Ok(())
        } else {
            Err(SearchError::InvalidQuery)
        }
    }

    /// Update operation status
    fn update_operation_status(&mut self, operation_id: u64, status: SearchOperationStatus) -> Result<(), SearchError> {
        if let Some(metadata) = self.active_operations.get_mut(&operation_id) {
            metadata.status = status;
            Ok(())
        } else {
            Err(SearchError::InvalidQuery)
        }
    }

    /// Complete search operation successfully
    fn complete_search_operation(&mut self, operation_id: u64, execution_time: u64, memory_used: usize) -> Result<(), SearchError> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = SearchOperationStatus::Completed;
            
            // Update analytics with completed operation
            self.analytics.vectors_examined += metadata.k;
            
            Ok(())
        } else {
            Err(SearchError::InvalidQuery)
        }
    }

    /// Fail search operation with error handling
    fn fail_search_operation(&mut self, operation_id: u64, reason: String) -> Result<(), SearchError> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = SearchOperationStatus::Failed;
            
            // Log failure for debugging (in a real implementation, this would use proper logging)
            let _failure_info = (operation_id, reason, metadata.user_id, self.get_current_time_us());
            
            // Update resource tracker with failed operation
            self.update_resource_tracker(0, metadata.estimated_memory, false);
            
            Ok(())
        } else {
            Err(SearchError::InvalidQuery)
        }
    }

    /// Estimate memory usage for a search operation
    fn estimate_search_memory(&self, query: &SearchQuery) -> usize {
        let vector_memory = query.vector.len() * core::mem::size_of::<f32>();
        let result_memory = query.k * core::mem::size_of::<ScoredResult>();
        let overhead = 1024; // Fixed overhead
        
        vector_memory + result_memory + overhead
    }

    /// Update resource tracker with operation results
    fn update_resource_tracker(&mut self, operation_time: u64, memory_used: usize, success: bool) {
        self.resource_tracker.total_operations += 1;
        
        if !success {
            self.resource_tracker.failed_operations += 1;
        }
        
        // Update memory tracking
        self.resource_tracker.total_memory_allocated += memory_used;
        if memory_used > self.resource_tracker.peak_memory_usage {
            self.resource_tracker.peak_memory_usage = memory_used;
        }
        
        // Update average operation time
        let total_time = self.resource_tracker.avg_operation_time_us * (self.resource_tracker.total_operations - 1) + operation_time;
        self.resource_tracker.avg_operation_time_us = total_time / self.resource_tracker.total_operations;
    }

    /// Cancel active search operation
    pub fn cancel_search_operation(&mut self, operation_id: u64) -> Result<(), SearchError> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = SearchOperationStatus::Cancelled;
            Ok(())
        } else {
            Err(SearchError::InvalidQuery)
        }
    }

    /// Get active operations for monitoring
    pub fn get_active_operations(&self) -> &BTreeMap<u64, SearchOperationMetadata> {
        &self.active_operations
    }

    /// Get resource usage statistics
    pub fn get_resource_stats(&self) -> &ResourceTracker {
        &self.resource_tracker
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
    
    
    /// Calculate vector sparsity (ratio of zero elements)
    fn calculate_sparsity(&self, vector: &[f32]) -> f32 {
        let zero_count = vector.iter().filter(|&&x| x == 0.0).count();
        zero_count as f32 / vector.len() as f32
    }
    
    /// Calculate vector magnitude (L2 norm)
    fn calculate_magnitude(&self, vector: &[f32]) -> f32 {
        vector.iter().map(|&x| x * x).sum::<f32>().sqrt()
    }
    
    /// Calculate vector entropy (simplified measure of randomness)
    fn calculate_entropy(&self, vector: &[f32]) -> f32 {
        // Simplified entropy calculation based on value distribution
        let mut histogram = [0u32; 10]; // 10 bins for simplicity
        let max_val = vector.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        
        if max_val == 0.0 {
            return 0.0;
        }
        
        for &val in vector {
            let normalized = (val.abs() / max_val * 9.0) as usize;
            let bin = normalized.min(9);
            histogram[bin] += 1;
        }
        
        let total = vector.len() as f32;
        let mut entropy = 0.0;
        for &count in &histogram {
            if count > 0 {
                let p = count as f32 / total;
                entropy -= p * p.log2();
            }
        }
        
        entropy
    }
    
    /// Determine query complexity based on characteristics
    fn determine_query_complexity(&self, query: &SearchQuery) -> crate::query_planner::QueryComplexity {
        let dimension_factor = if query.vector.len() > 512 { 2.0 } else if query.vector.len() > 128 { 1.5 } else { 1.0 };
        let k_factor = if query.k > 100 { 2.0 } else if query.k > 50 { 1.5 } else { 1.0 };
        let filter_factor = if query.filter.is_some() { 1.5 } else { 1.0 };
        let approx_factor = if query.approximate { 0.8 } else { 1.2 };
        
        let complexity_score = dimension_factor * k_factor * filter_factor * approx_factor;
        
        if complexity_score >= 4.0 {
            crate::query_planner::QueryComplexity::HighlyComplex
        } else if complexity_score >= 2.5 {
            crate::query_planner::QueryComplexity::Complex
        } else if complexity_score >= 1.5 {
            crate::query_planner::QueryComplexity::Moderate
        } else {
            crate::query_planner::QueryComplexity::Simple
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_handlers::VectorStorage;

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.k, 10);
        assert_eq!(query.metric, DistanceMetric::Euclidean);
        assert!(query.approximate);
        assert!(query.use_simd);
    }

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();
        assert!(!options.enable_caching);
        assert!(options.validate_results);
        assert!(!options.enable_analytics);
    }

    #[test]
    fn test_batch_search_request() {
        let query = SearchQuery::default();
        let batch = BatchSearchRequest {
            queries: vec![query],
            merge_results: false,
            max_total_results: 100,
        };
        
        assert_eq!(batch.queries.len(), 1);
        assert!(!batch.merge_results);
    }
}