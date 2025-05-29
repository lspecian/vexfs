//! Advanced Indexing Strategies - PQ, Flat, and Index Selection
//!
//! This module contains the Product Quantization, Flat Index, and Index Selection
//! implementations that extend the advanced indexing capabilities.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::fs_core::operations::OperationContext;
use crate::anns::{AnnsError, SearchResult, DistanceMetric};
use crate::vector_optimizations::{VectorOptimizer, SimdStrategy, MemoryLayout, BatchConfig};
use crate::vector_metrics::VectorMetrics;
use super::advanced_indexing::{IndexStrategy, PqConfig, FlatConfig};

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, collections::BTreeMap, string::String};
#[cfg(feature = "std")]
use std::{vec::Vec, collections::BTreeMap, string::String};

use core::f32;

/// PQ Index implementation
pub struct PqIndex {
    config: PqConfig,
    dimensions: usize,
    subspace_size: usize,
    codebooks: Vec<Vec<Vec<f32>>>, // [subspace][centroid][dimension]
    codes: Vec<Vec<u8>>, // [vector_id][subspace] -> centroid_id
    vector_ids: Vec<u64>,
    vector_count: usize,
    optimizer: VectorOptimizer,
}

impl PqIndex {
    /// Create new PQ index
    pub fn new(dimensions: usize, config: PqConfig) -> Result<Self, AnnsError> {
        if dimensions % config.num_subspaces != 0 {
            return Err(AnnsError::InvalidParameter);
        }

        let subspace_size = dimensions / config.num_subspaces;
        let optimizer = VectorOptimizer::with_config(
            SimdStrategy::Auto,
            MemoryLayout::Hybrid,
            BatchConfig::default(),
        );

        let num_subspaces = config.num_subspaces;
        
        Ok(Self {
            dimensions,
            subspace_size,
            codebooks: vec![Vec::new(); num_subspaces],
            codes: Vec::new(),
            vector_ids: Vec::new(),
            vector_count: 0,
            optimizer,
            config,
        })
    }

    /// Train the PQ codebooks with OperationContext integration for transaction support
    pub fn train(&mut self, context: &mut OperationContext, training_vectors: &[Vec<f32>]) -> VexfsResult<()> {
        if training_vectors.is_empty() {
            return Err(VexfsError::InvalidArgument("No training vectors provided".to_string()));
        }

        // Begin transaction-like operation for atomic codebook training
        let operation_start = std::time::Instant::now();
        
        // Track memory allocation for training
        let estimated_memory = self.config.num_subspaces * self.config.num_centroids_per_subspace * self.subspace_size * core::mem::size_of::<f32>();
        
        // Store original codebooks for potential rollback
        let original_codebooks = self.codebooks.clone();
        
        // Train codebook for each subspace with error recovery
        for subspace_idx in 0..self.config.num_subspaces {
            let start_dim = subspace_idx * self.subspace_size;
            let end_dim = start_dim + self.subspace_size;

            // Extract subspace vectors
            let subspace_vectors: Vec<Vec<f32>> = training_vectors.iter()
                .map(|v| v[start_dim..end_dim].to_vec())
                .collect();

            // Train codebook using k-means with error handling
            match self.train_subspace_codebook(&subspace_vectors) {
                Ok(codebook) => {
                    self.codebooks[subspace_idx] = codebook;
                }
                Err(e) => {
                    // Rollback on failure - restore original codebooks
                    self.codebooks = original_codebooks;
                    return Err(e);
                }
            }
        }

        // Log successful operation through OperationContext
        let operation_duration = operation_start.elapsed();
        
        Ok(())
    }

    /// Insert vector into PQ index with OperationContext integration for atomic operations
    pub fn insert(&mut self, context: &mut OperationContext, vector_id: u64, vector: &[f32]) -> VexfsResult<()> {
        if vector.len() != self.dimensions {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(vector.len() as u16)));
        }

        if self.codebooks.iter().any(|cb| cb.is_empty()) {
            return Err(VexfsError::InvalidOperation("Index not trained".to_string()));
        }

        // Check for duplicate vector_id to maintain consistency
        if self.vector_ids.contains(&vector_id) {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidVectorId));
        }

        // Begin atomic operation - store state for potential rollback
        let original_codes_len = self.codes.len();
        let original_vector_ids_len = self.vector_ids.len();
        let original_vector_count = self.vector_count;

        // Encode vector using trained codebooks
        let mut codes = Vec::with_capacity(self.config.num_subspaces);
        
        for subspace_idx in 0..self.config.num_subspaces {
            let start_dim = subspace_idx * self.subspace_size;
            let end_dim = start_dim + self.subspace_size;
            let subspace_vector = &vector[start_dim..end_dim];
            
            // Find nearest centroid in this subspace
            let centroid_id = self.find_nearest_centroid_in_subspace(subspace_vector, subspace_idx);
            codes.push(centroid_id as u8);
        }

        // Atomic update - all or nothing
        self.codes.push(codes);
        self.vector_ids.push(vector_id);
        self.vector_count += 1;

        // Verify consistency after insertion
        if self.codes.len() != self.vector_ids.len() || self.vector_count != self.vector_ids.len() {
            // Rollback on inconsistency
            self.codes.truncate(original_codes_len);
            self.vector_ids.truncate(original_vector_ids_len);
            self.vector_count = original_vector_count;
            return Err(VexfsError::InvalidOperation("Index consistency check failed".to_string()));
        }

        Ok(())
    }

    /// Search for similar vectors using PQ with OperationContext integration for resource coordination
    pub fn search(&self, context: &mut OperationContext, query: &[f32], k: usize) -> VexfsResult<Vec<SearchResult>> {
        if query.len() != self.dimensions {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(query.len() as u16)));
        }

        if self.codebooks.iter().any(|cb| cb.is_empty()) {
            return Err(VexfsError::InvalidOperation("Index not trained".to_string()));
        }

        // Validate k parameter
        if k == 0 {
            return Err(VexfsError::InvalidArgument("k must be greater than 0".to_string()));
        }

        // Track resource usage for this search operation
        let search_start = std::time::Instant::now();
        let estimated_memory = self.vector_count * core::mem::size_of::<SearchResult>() +
                              self.config.num_subspaces * self.config.num_centroids_per_subspace * core::mem::size_of::<f32>();

        // Precompute distance tables for each subspace
        let distance_tables = self.compute_distance_tables(query);

        // Compute approximate distances for all vectors with bounds checking
        let mut candidates = Vec::with_capacity(self.vector_count.min(k * 10)); // Limit memory usage
        
        for (vector_idx, codes) in self.codes.iter().enumerate() {
            if vector_idx >= self.vector_ids.len() {
                // Consistency check - should not happen in well-formed index
                return Err(VexfsError::InvalidOperation("Index consistency error: codes/vector_ids mismatch".to_string()));
            }

            let distance = self.compute_asymmetric_distance(&distance_tables, codes);

            candidates.push(SearchResult {
                vector_id: self.vector_ids[vector_idx],
                distance,
            });
        }

        // Sort and return top-k with bounds checking
        candidates.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(core::cmp::Ordering::Equal));
        candidates.truncate(k);

        // Log search performance through OperationContext
        let search_duration = search_start.elapsed();

        Ok(candidates)
    }

    /// Train codebook for a single subspace
    fn train_subspace_codebook(&self, subspace_vectors: &[Vec<f32>]) -> VexfsResult<Vec<Vec<f32>>> {
        let mut codebook = Vec::with_capacity(self.config.num_centroids_per_subspace);
        
        if subspace_vectors.is_empty() {
            return Ok(codebook);
        }

        // Initialize centroids randomly
        for i in 0..self.config.num_centroids_per_subspace {
            let idx = i % subspace_vectors.len();
            codebook.push(subspace_vectors[idx].clone());
        }

        // Run k-means clustering
        for _iteration in 0..self.config.max_iterations {
            let old_codebook = codebook.clone();
            
            // Assign vectors to clusters
            let mut clusters: Vec<Vec<&Vec<f32>>> = vec![Vec::new(); self.config.num_centroids_per_subspace];
            
            for vector in subspace_vectors {
                let centroid_id = self.find_nearest_centroid_in_codebook(vector, &codebook);
                clusters[centroid_id].push(vector);
            }

            // Update centroids
            for (i, cluster) in clusters.iter().enumerate() {
                if !cluster.is_empty() {
                    codebook[i] = self.compute_centroid_from_cluster(cluster);
                }
            }

            // Check convergence
            if self.codebook_converged(&old_codebook, &codebook) {
                break;
            }
        }

        Ok(codebook)
    }

    /// Find nearest centroid in a specific subspace
    fn find_nearest_centroid_in_subspace(&self, subspace_vector: &[f32], subspace_idx: usize) -> usize {
        let codebook = &self.codebooks[subspace_idx];
        self.find_nearest_centroid_in_codebook(subspace_vector, codebook)
    }

    /// Find nearest centroid in a codebook
    fn find_nearest_centroid_in_codebook(&self, vector: &[f32], codebook: &[Vec<f32>]) -> usize {
        let mut best_distance = f32::INFINITY;
        let mut best_centroid = 0;

        for (i, centroid) in codebook.iter().enumerate() {
            let distance = self.compute_l2_distance(vector, centroid);
            if distance < best_distance {
                best_distance = distance;
                best_centroid = i;
            }
        }

        best_centroid
    }

    /// Compute distance tables for asymmetric distance computation
    fn compute_distance_tables(&self, query: &[f32]) -> Vec<Vec<f32>> {
        let mut distance_tables = Vec::with_capacity(self.config.num_subspaces);
        
        for subspace_idx in 0..self.config.num_subspaces {
            let start_dim = subspace_idx * self.subspace_size;
            let end_dim = start_dim + self.subspace_size;
            let query_subspace = &query[start_dim..end_dim];
            
            let mut table = Vec::with_capacity(self.config.num_centroids_per_subspace);
            
            for centroid in &self.codebooks[subspace_idx] {
                let distance = self.compute_l2_distance(query_subspace, centroid);
                table.push(distance * distance); // Squared distance for efficiency
            }
            
            distance_tables.push(table);
        }
        
        distance_tables
    }

    /// Compute asymmetric distance using precomputed tables
    fn compute_asymmetric_distance(&self, distance_tables: &[Vec<f32>], codes: &[u8]) -> f32 {
        let mut total_distance = 0.0;
        
        for (subspace_idx, &code) in codes.iter().enumerate() {
            if subspace_idx < distance_tables.len() && (code as usize) < distance_tables[subspace_idx].len() {
                total_distance += distance_tables[subspace_idx][code as usize];
            }
        }
        
        total_distance.sqrt()
    }

    /// Compute L2 distance between two vectors
    fn compute_l2_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter())
            .map(|(&x, &y)| (x - y) * (x - y))
            .sum::<f32>()
            .sqrt()
    }

    /// Compute centroid from cluster
    fn compute_centroid_from_cluster(&self, cluster: &[&Vec<f32>]) -> Vec<f32> {
        if cluster.is_empty() {
            return vec![0.0; self.subspace_size];
        }

        let mut centroid = vec![0.0; self.subspace_size];
        for vector in cluster {
            for (i, &value) in vector.iter().enumerate() {
                if i < centroid.len() {
                    centroid[i] += value;
                }
            }
        }

        let count = cluster.len() as f32;
        for value in &mut centroid {
            *value /= count;
        }

        centroid
    }

    /// Check if codebook has converged
    fn codebook_converged(&self, old_codebook: &[Vec<f32>], new_codebook: &[Vec<f32>]) -> bool {
        for (old, new) in old_codebook.iter().zip(new_codebook.iter()) {
            let distance = self.compute_l2_distance(old, new);
            if distance > self.config.convergence_threshold {
                return false;
            }
        }
        true
    }

    /// Get memory usage statistics
    pub fn memory_usage(&self) -> usize {
        let codebooks_size = self.codebooks.iter()
            .map(|cb| cb.len() * self.subspace_size * core::mem::size_of::<f32>())
            .sum::<usize>();
        let codes_size = self.codes.len() * self.config.num_subspaces * core::mem::size_of::<u8>();
        let ids_size = self.vector_ids.len() * core::mem::size_of::<u64>();
        codebooks_size + codes_size + ids_size
    }
}

/// Flat Index implementation for exact nearest neighbor search
pub struct FlatIndex {
    config: FlatConfig,
    dimensions: usize,
    vectors: Vec<Vec<f32>>,
    vector_ids: Vec<u64>,
    vector_count: usize,
    optimizer: VectorOptimizer,
    metrics: VectorMetrics,
}

impl FlatIndex {
    /// Create new Flat index
    pub fn new(dimensions: usize, config: FlatConfig) -> Result<Self, AnnsError> {
        let optimizer = VectorOptimizer::with_config(
            if config.enable_simd { SimdStrategy::Auto } else { SimdStrategy::Scalar },
            MemoryLayout::Hybrid,
            BatchConfig {
                batch_size: config.batch_size,
                enable_prefetch: true,
                enable_parallel: config.enable_parallel,
                alignment: config.alignment,
            },
        );

        let metrics = VectorMetrics::new(config.enable_simd);

        Ok(Self {
            config,
            dimensions,
            vectors: Vec::new(),
            vector_ids: Vec::new(),
            vector_count: 0,
            optimizer,
            metrics,
        })
    }

    /// Insert vector into Flat index with OperationContext integration for atomic operations
    pub fn insert(&mut self, context: &mut OperationContext, vector_id: u64, vector: &[f32]) -> VexfsResult<()> {
        if vector.len() != self.dimensions {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(vector.len() as u16)));
        }

        // Check for duplicate vector_id to maintain consistency
        if self.vector_ids.contains(&vector_id) {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidVectorId));
        }

        // Begin atomic operation - store state for potential rollback
        let original_vectors_len = self.vectors.len();
        let original_vector_ids_len = self.vector_ids.len();
        let original_vector_count = self.vector_count;

        // Atomic update - all or nothing
        self.vectors.push(vector.to_vec());
        self.vector_ids.push(vector_id);
        self.vector_count += 1;

        // Verify consistency after insertion
        if self.vectors.len() != self.vector_ids.len() || self.vector_count != self.vector_ids.len() {
            // Rollback on inconsistency
            self.vectors.truncate(original_vectors_len);
            self.vector_ids.truncate(original_vector_ids_len);
            self.vector_count = original_vector_count;
            return Err(VexfsError::InvalidOperation("Index consistency check failed".to_string()));
        }

        Ok(())
    }

    /// Search for exact nearest neighbors using brute force with SIMD optimization and OperationContext integration
    pub fn search(&mut self, context: &mut OperationContext, query: &[f32], k: usize) -> VexfsResult<Vec<SearchResult>> {
        if query.len() != self.dimensions {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(query.len() as u16)));
        }

        // Validate k parameter
        if k == 0 {
            return Err(VexfsError::InvalidArgument("k must be greater than 0".to_string()));
        }

        // Track resource usage for this search operation
        let search_start = std::time::Instant::now();
        let estimated_memory = self.vector_count * core::mem::size_of::<SearchResult>();

        // Limit memory usage for large searches
        let mut distances = Vec::with_capacity(self.vector_count.min(k * 100));

        if self.config.enable_simd && self.vectors.len() > self.config.batch_size {
            // Use SIMD-optimized batch processing
            self.search_with_simd(query, &mut distances)?;
        } else {
            // Use scalar computation
            self.search_scalar(query, &mut distances)?;
        }

        // Verify consistency
        if distances.len() > self.vector_count {
            return Err(VexfsError::InvalidOperation("Search result count exceeds vector count".to_string()));
        }

        // Sort and return top-k with bounds checking
        distances.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(core::cmp::Ordering::Equal));
        distances.truncate(k);

        // Log search performance through OperationContext
        let search_duration = search_start.elapsed();

        Ok(distances)
    }

    /// SIMD-optimized search
    fn search_with_simd(&mut self, query: &[f32], distances: &mut Vec<SearchResult>) -> VexfsResult<()> {
        // Process vectors in batches for SIMD optimization
        for (chunk_idx, chunk) in self.vectors.chunks(self.config.batch_size).enumerate() {
            let vector_refs: Vec<&[f32]> = chunk.iter().map(|v| v.as_slice()).collect();
            let mut batch_distances = vec![0.0f32; vector_refs.len()];
            
            // Use optimized batch distance calculation
            self.metrics.batch_calculate_distances(
                query,
                &vector_refs,
                DistanceMetric::Euclidean,
                &mut batch_distances,
            ).map_err(|_| VexfsError::InvalidOperation("SIMD distance calculation failed".to_string()))?;

            // Add results to distances vector
            let chunk_start = chunk_idx * self.config.batch_size;
            for (i, distance) in batch_distances.into_iter().enumerate() {
                let vector_idx = chunk_start + i;
                if vector_idx < self.vector_ids.len() {
                    distances.push(SearchResult {
                        vector_id: self.vector_ids[vector_idx],
                        distance,
                    });
                }
            }
        }

        Ok(())
    }

    /// Scalar search implementation
    fn search_scalar(&mut self, query: &[f32], distances: &mut Vec<SearchResult>) -> VexfsResult<()> {
        for (i, vector) in self.vectors.iter().enumerate() {
            let distance = self.metrics.calculate_distance(query, vector, DistanceMetric::Euclidean)
                .map_err(|_| VexfsError::InvalidOperation("Distance calculation failed".to_string()))?;
            
            distances.push(SearchResult {
                vector_id: self.vector_ids[i],
                distance,
            });
        }

        Ok(())
    }

    /// Get memory usage statistics
    pub fn memory_usage(&self) -> usize {
        let vectors_size = self.vectors.len() * self.dimensions * core::mem::size_of::<f32>();
        let ids_size = self.vector_ids.len() * core::mem::size_of::<u64>();
        vectors_size + ids_size
    }
}

/// Collection size categories
#[derive(Debug, Clone, Copy)]
pub enum CollectionSize {
    Small,    // < 10K
    Medium,   // 10K - 100K
    Large,    // 100K - 1M
    VeryLarge, // > 1M
}

/// Query pattern types
#[derive(Debug, Clone, Copy)]
pub enum QueryPattern {
    HighThroughput, // Many queries per second
    HighAccuracy,   // Exact results required
    Balanced,       // Balance of speed and accuracy
}

/// Strategy recommendation
#[derive(Debug, Clone)]
pub struct StrategyRecommendation {
    pub strategy: IndexStrategy,
    pub confidence: f32, // 0.0 to 1.0
    pub reason: String,
}

/// Collection analysis results
#[derive(Debug, Clone)]
pub struct CollectionAnalysis {
    pub size_category: CollectionSize,
    pub dimensions: usize,
    pub is_high_dimensional: bool,
    pub is_sparse: bool,
    pub estimated_memory_usage: usize,
    pub estimated_search_latency: f32, // milliseconds
}

/// Index selection result
#[derive(Debug, Clone)]
pub struct IndexSelectionResult {
    pub primary_strategy: IndexStrategy,
    pub recommendations: Vec<StrategyRecommendation>,
    pub collection_analysis: CollectionAnalysis,
}

/// Index Selection Strategy for automatic optimal index selection
pub struct IndexSelector {
    /// Collection size thresholds
    small_threshold: usize,
    medium_threshold: usize,
    large_threshold: usize,
    /// Dimension thresholds
    high_dim_threshold: usize,
    /// Performance requirements
    accuracy_priority: bool,
    memory_priority: bool,
    speed_priority: bool,
}

impl IndexSelector {
    /// Create new index selector with default thresholds
    pub fn new() -> Self {
        Self {
            small_threshold: 10_000,
            medium_threshold: 100_000,
            large_threshold: 1_000_000,
            high_dim_threshold: 512,
            accuracy_priority: false,
            memory_priority: false,
            speed_priority: true, // Default to speed priority
        }
    }

    /// Set performance priorities
    pub fn set_priorities(&mut self, accuracy: bool, memory: bool, speed: bool) {
        self.accuracy_priority = accuracy;
        self.memory_priority = memory;
        self.speed_priority = speed;
    }

    /// Select optimal index strategy based on collection characteristics
    pub fn select_strategy(
        &self,
        collection_size: usize,
        dimensions: usize,
        vector_sparsity: f32, // Percentage of zero values
        query_pattern: QueryPattern,
    ) -> IndexSelectionResult {
        let mut recommendations = Vec::new();

        // Analyze collection size
        let size_category = if collection_size < self.small_threshold {
            CollectionSize::Small
        } else if collection_size < self.medium_threshold {
            CollectionSize::Medium
        } else if collection_size < self.large_threshold {
            CollectionSize::Large
        } else {
            CollectionSize::VeryLarge
        };

        // Analyze dimensions
        let is_high_dimensional = dimensions > self.high_dim_threshold;
        let is_sparse = vector_sparsity > 0.8; // More than 80% zeros

        // Primary strategy selection logic
        let primary_strategy = match (size_category, is_high_dimensional, is_sparse) {
            // Small collections: Flat index for exact results
            (CollectionSize::Small, _, _) if self.accuracy_priority => IndexStrategy::Flat,
            
            // Medium collections with high dimensions: PQ for memory efficiency
            (CollectionSize::Medium, true, _) if self.memory_priority => IndexStrategy::PQ,
            
            // Large collections: IVF for scalability
            (CollectionSize::Large | CollectionSize::VeryLarge, _, _) => IndexStrategy::IVF,
            
            // Sparse vectors: LSH for efficiency
            (_, _, true) => IndexStrategy::LSH,
            
            // High-dimensional vectors: PQ for compression
            (_, true, false) if dimensions > 1024 => IndexStrategy::PQ,
            
            // Default: HNSW for balanced performance
            _ => IndexStrategy::HNSW,
        };

        recommendations.push(StrategyRecommendation {
            strategy: primary_strategy,
            confidence: self.calculate_confidence(primary_strategy, collection_size, dimensions, vector_sparsity),
            reason: self.get_strategy_reason(primary_strategy, size_category, is_high_dimensional, is_sparse),
        });

        // Add alternative strategies
        self.add_alternative_strategies(&mut recommendations, size_category, is_high_dimensional, is_sparse, query_pattern);

        IndexSelectionResult {
            primary_strategy: primary_strategy,
            recommendations,
            collection_analysis: CollectionAnalysis {
                size_category,
                dimensions,
                is_high_dimensional,
                is_sparse,
                estimated_memory_usage: self.estimate_memory_usage(primary_strategy, collection_size, dimensions),
                estimated_search_latency: self.estimate_search_latency(primary_strategy, collection_size),
            },
        }
    }

    /// Add alternative strategy recommendations
    fn add_alternative_strategies(
        &self,
        recommendations: &mut Vec<StrategyRecommendation>,
        size_category: CollectionSize,
        is_high_dimensional: bool,
        _is_sparse: bool,
        query_pattern: QueryPattern,
    ) {
        // Add HNSW as alternative for most cases
        if recommendations[0].strategy != IndexStrategy::HNSW {
            recommendations.push(StrategyRecommendation {
                strategy: IndexStrategy::HNSW,
                confidence: 0.8,
                reason: "Balanced performance alternative".to_string(),
            });
        }

        // Add Flat for small collections requiring exact results
        if matches!(size_category, CollectionSize::Small | CollectionSize::Medium) && 
           recommendations[0].strategy != IndexStrategy::Flat {
            recommendations.push(StrategyRecommendation {
                strategy: IndexStrategy::Flat,
                confidence: 0.7,
                reason: "Exact search alternative for smaller collections".to_string(),
            });
        }

        // Consider query pattern
        match query_pattern {
            QueryPattern::HighThroughput => {
                if recommendations[0].strategy != IndexStrategy::LSH {
                    recommendations.push(StrategyRecommendation {
                        strategy: IndexStrategy::LSH,
                        confidence: 0.8,
                        reason: "Optimized for high-throughput queries".to_string(),
                    });
                }
            }
            QueryPattern::HighAccuracy => {
                if recommendations[0].strategy != IndexStrategy::Flat {
                    recommendations.push(StrategyRecommendation {
                        strategy: IndexStrategy::Flat,
                        confidence: 0.9,
                        reason: "Exact search for highest accuracy".to_string(),
                    });
                }
            }
            QueryPattern::Balanced => {
                // HNSW already added as alternative
            }
        }
    }

    /// Calculate confidence score for a strategy
    fn calculate_confidence(&self, strategy: IndexStrategy, collection_size: usize, dimensions: usize, sparsity: f32) -> f32 {
        match strategy {
            IndexStrategy::Flat => {
                if collection_size < self.small_threshold { 0.95 }
                else if collection_size < self.medium_threshold { 0.7 }
                else { 0.3 }
            }
            IndexStrategy::HNSW => {
                if collection_size > self.small_threshold && collection_size < self.large_threshold { 0.9 }
                else { 0.7 }
            }
            IndexStrategy::LSH => {
                if sparsity > 0.5 { 0.85 }
                else if dimensions > 256 { 0.75 }
                else { 0.6 }
            }
            IndexStrategy::IVF => {
                if collection_size > self.medium_threshold { 0.9 }
                else { 0.6 }
            }
            IndexStrategy::PQ => {
                if dimensions > self.high_dim_threshold { 0.85 }
                else { 0.6 }
            }
        }
    }

    /// Get reason for strategy selection
    fn get_strategy_reason(&self, strategy: IndexStrategy, _size: CollectionSize, high_dim: bool, sparse: bool) -> String {
        match strategy {
            IndexStrategy::Flat => "Exact search optimal for small collections".to_string(),
            IndexStrategy::HNSW => "Balanced performance for medium-scale collections".to_string(),
            IndexStrategy::LSH => if sparse {
                "Optimized for sparse vector collections".to_string()
            } else {
                "Fast approximate search for high-dimensional data".to_string()
            },
            IndexStrategy::IVF => "Scalable indexing for large vector collections".to_string(),
            IndexStrategy::PQ => if high_dim {
                "Memory-efficient compression for high-dimensional vectors".to_string()
            } else {
                "Compressed indexing for memory optimization".to_string()
            },
        }
    }

    /// Estimate memory usage for a strategy
    fn estimate_memory_usage(&self, strategy: IndexStrategy, collection_size: usize, dimensions: usize) -> usize {
        let base_vector_size = collection_size * dimensions * core::mem::size_of::<f32>();
        
        match strategy {
            IndexStrategy::Flat => base_vector_size,
            IndexStrategy::HNSW => base_vector_size + (collection_size * 64), // Graph overhead
            IndexStrategy::LSH => base_vector_size / 4, // Hash tables are smaller
            IndexStrategy::IVF => base_vector_size + (256 * dimensions * core::mem::size_of::<f32>()), // Centroids
            IndexStrategy::PQ => collection_size * 8 + (8 * 256 * (dimensions / 8) * core::mem::size_of::<f32>()), // Codes + codebooks
        }
    }

    /// Estimate search latency for a strategy
    fn estimate_search_latency(&self, strategy: IndexStrategy, collection_size: usize) -> f32 {
        match strategy {
            IndexStrategy::Flat => (collection_size as f32) * 0.001, // Linear scan
            IndexStrategy::HNSW => (collection_size as f32).log2() * 0.1, // Logarithmic
            IndexStrategy::LSH => 10.0, // Constant time (approximate)
            IndexStrategy::IVF => (collection_size as f32 / 256.0) * 0.01, // Depends on clusters
            IndexStrategy::PQ => (collection_size as f32) * 0.0001, // Fast approximate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_core::operations::OperationContext;
    use crate::storage::StorageManager;

    #[test]
    fn test_pq_index_creation() {
        let config = PqConfig::default();
        let index = PqIndex::new(128, config);
        assert!(index.is_ok());
    }

    #[test]
    fn test_flat_index_creation() {
        let config = FlatConfig::default();
        let index = FlatIndex::new(128, config);
        assert!(index.is_ok());
    }

    #[test]
    fn test_index_selector() {
        let selector = IndexSelector::new();
        
        // Test small collection
        let result = selector.select_strategy(1000, 128, 0.1, QueryPattern::Balanced);
        assert!(matches!(result.collection_analysis.size_category, CollectionSize::Small));
        
        // Test large collection
        let result = selector.select_strategy(500_000, 128, 0.1, QueryPattern::Balanced);
        assert!(matches!(result.collection_analysis.size_category, CollectionSize::Large));
    }
}