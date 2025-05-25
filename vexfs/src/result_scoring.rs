//! Result scoring, ranking and validation framework for VexFS
//!
//! This module provides comprehensive scoring, ranking and validation for search results,
//! including confidence calculation, result fusion, and quality assessment.



extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap};
use core::cmp::Ordering;

use crate::knn_search::{KnnResult, SearchParams};
use crate::anns::DistanceMetric;
use crate::vector_storage::VectorDataType;

/// Maximum number of results that can be scored
pub const MAX_SCORABLE_RESULTS: usize = 10000;

/// Confidence score calculation parameters
pub const CONFIDENCE_ALPHA: f32 = 0.8;
pub const CONFIDENCE_BETA: f32 = 0.2;

/// Result scoring and ranking error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoringError {
    /// Invalid input parameters
    InvalidParameters,
    /// Empty result set
    EmptyResults,
    /// Inconsistent scoring parameters
    InconsistentParams,
    /// Overflow in calculations
    Overflow,
    /// Invalid confidence score
    InvalidConfidence,
}

/// Scoring method for result ranking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoringMethod {
    /// Distance-based scoring (lower distance = higher score)
    Distance,
    /// Confidence-based scoring
    Confidence,
    /// Hybrid scoring combining multiple factors
    Hybrid,
    /// Reciprocal rank fusion for combining multiple searches
    ReciprocalRank,
    /// Borda count fusion
    BordaCount,
}

/// Result quality metrics
#[derive(Debug, Clone, Copy)]
pub struct QualityMetrics {
    /// Average distance of results
    pub avg_distance: f32,
    /// Standard deviation of distances
    pub distance_std: f32,
    /// Average confidence score
    pub avg_confidence: f32,
    /// Number of results above confidence threshold
    pub high_confidence_count: usize,
    /// Diversity score (how spread out the results are)
    pub diversity_score: f32,
    /// Coverage score (how well the results cover the search space)
    pub coverage_score: f32,
}

/// Scoring parameters for result ranking
#[derive(Debug, Clone)]
pub struct ScoringParams {
    /// Scoring method to use
    pub method: ScoringMethod,
    /// Weight for distance in hybrid scoring
    pub distance_weight: f32,
    /// Weight for confidence in hybrid scoring
    pub confidence_weight: f32,
    /// Weight for metadata relevance in hybrid scoring
    pub metadata_weight: f32,
    /// Minimum confidence threshold for filtering
    pub min_confidence_threshold: f32,
    /// Maximum distance threshold for filtering
    pub max_distance_threshold: f32,
    /// Whether to normalize scores to 0-1 range
    pub normalize_scores: bool,
    /// Whether to apply diversity filtering
    pub apply_diversity_filter: bool,
    /// Diversity threshold for filtering similar results
    pub diversity_threshold: f32,
}

impl Default for ScoringParams {
    fn default() -> Self {
        Self {
            method: ScoringMethod::Hybrid,
            distance_weight: 0.6,
            confidence_weight: 0.3,
            metadata_weight: 0.1,
            min_confidence_threshold: 0.0,
            max_distance_threshold: f32::INFINITY,
            normalize_scores: true,
            apply_diversity_filter: false,
            diversity_threshold: 0.9,
        }
    }
}

/// Scored result with ranking information
#[derive(Debug, Clone)]
pub struct ScoredResult {
    /// Original k-NN result
    pub result: KnnResult,
    /// Computed score (higher = better)
    pub score: f32,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Rank in the result set (1-based)
    pub rank: usize,
    /// Normalized score (0.0-1.0)
    pub normalized_score: f32,
    /// Quality assessment flags
    pub quality_flags: u32,
}

impl PartialEq for ScoredResult {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for ScoredResult {}

impl PartialOrd for ScoredResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Higher scores should come first
        other.score.partial_cmp(&self.score)
    }
}

impl Ord for ScoredResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Quality flags for results
pub mod quality_flags {
    pub const HIGH_CONFIDENCE: u32 = 1 << 0;
    pub const LOW_DISTANCE: u32 = 1 << 1;
    pub const RECENT_FILE: u32 = 1 << 2;
    pub const LARGE_FILE: u32 = 1 << 3;
    pub const EXACT_DIMENSION_MATCH: u32 = 1 << 4;
    pub const POTENTIAL_DUPLICATE: u32 = 1 << 5;
    pub const OUTLIER_DISTANCE: u32 = 1 << 6;
    pub const LOW_QUALITY: u32 = 1 << 7;
}

/// Result scorer and ranker
pub struct ResultScorer {
    /// Current scoring parameters
    params: ScoringParams,
    /// Distance statistics for normalization
    distance_stats: Option<(f32, f32)>, // (mean, std)
    /// Confidence statistics for normalization
    confidence_stats: Option<(f32, f32)>, // (mean, std)
    /// Buffer for score calculations
    score_buffer: Vec<f32>,
}

impl ResultScorer {
    /// Create new result scorer
    pub fn new(params: ScoringParams) -> Self {
        Self {
            params,
            distance_stats: None,
            confidence_stats: None,
            score_buffer: Vec::new(),
        }
    }
    
    /// Score and rank a set of k-NN results
    pub fn score_and_rank(
        &mut self,
        results: &[KnnResult],
        search_params: &SearchParams,
    ) -> Result<Vec<ScoredResult>, ScoringError> {
        if results.is_empty() {
            return Err(ScoringError::EmptyResults);
        }
        
        if results.len() > MAX_SCORABLE_RESULTS {
            return Err(ScoringError::InvalidParameters);
        }
        
        // Calculate basic statistics
        self.calculate_statistics(results)?;
        
        // Calculate scores for each result
        let mut scored_results = Vec::with_capacity(results.len());
        
        for result in results {
            let confidence = self.calculate_confidence(result, search_params)?;
            let score = self.calculate_score(result, confidence, search_params)?;
            let quality_flags = self.assess_quality(result, confidence);
            
            scored_results.push(ScoredResult {
                result: result.clone(),
                score,
                confidence,
                rank: 0, // Will be set after sorting
                normalized_score: 0.0, // Will be set after normalization
                quality_flags,
            });
        }
        
        // Sort by score (descending)
        scored_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        // Apply diversity filtering if enabled
        if self.params.apply_diversity_filter {
            scored_results = self.apply_diversity_filter(scored_results)?;
        }
        
        // Assign ranks and normalize scores
        self.assign_ranks_and_normalize(&mut scored_results)?;
        
        // Apply final filtering
        self.apply_threshold_filtering(scored_results)
    }
    
    /// Calculate confidence score for a result
    fn calculate_confidence(
        &self,
        result: &KnnResult,
        search_params: &SearchParams,
    ) -> Result<f32, ScoringError> {
        // Base confidence from distance (closer = higher confidence)
        let distance_conf = if let Some((mean_dist, std_dist)) = self.distance_stats {
            if std_dist > 0.0 {
                let z_score = (result.distance - mean_dist) / std_dist;
                // Convert z-score to confidence (closer distances get higher confidence)
                (1.0 / (1.0 + (-z_score).exp())).max(0.0).min(1.0)
            } else {
                0.5 // Default if no variance
            }
        } else {
            0.5
        };
        
        // Metadata-based confidence factors
        let metadata_conf = self.calculate_metadata_confidence(result);
        
        // Combine confidence factors
        let confidence = CONFIDENCE_ALPHA * distance_conf + CONFIDENCE_BETA * metadata_conf;
        
        Ok(confidence.max(0.0).min(1.0))
    }
    
    /// Calculate metadata-based confidence
    fn calculate_metadata_confidence(&self, result: &KnnResult) -> f32 {
        let mut conf_factors = Vec::new();
        
        // Recent files get higher confidence
        let current_time = 1640995200; // Placeholder timestamp
        let age_days = (current_time - result.modified_timestamp) / (24 * 3600);
        let recency_factor = if age_days < 30 {
            1.0
        } else if age_days < 365 {
            0.8
        } else {
            0.6
        };
        conf_factors.push(recency_factor);
        
        // Larger files might be more significant
        let size_factor = if result.file_size > 1_000_000 {
            0.9
        } else if result.file_size > 100_000 {
            0.8
        } else {
            0.7
        };
        conf_factors.push(size_factor);
        
        // Data type consistency
        let type_factor = match result.data_type {
            VectorDataType::Float32 => 1.0, // Most common
            VectorDataType::Float16 => 0.9,
            VectorDataType::Int8 => 0.8,
            VectorDataType::Int16 => 0.8,
            VectorDataType::Binary => 0.7,
        };
        conf_factors.push(type_factor);
        
        // Average the factors
        conf_factors.iter().sum::<f32>() / conf_factors.len() as f32
    }
    
    /// Calculate final score for a result
    fn calculate_score(
        &self,
        result: &KnnResult,
        confidence: f32,
        search_params: &SearchParams,
    ) -> Result<f32, ScoringError> {
        match self.params.method {
            ScoringMethod::Distance => {
                // Simple distance-based scoring (inverted)
                Ok(1.0 / (1.0 + result.distance))
            }
            ScoringMethod::Confidence => {
                Ok(confidence)
            }
            ScoringMethod::Hybrid => {
                let distance_score = 1.0 / (1.0 + result.distance);
                let metadata_score = self.calculate_metadata_score(result);
                
                let score = self.params.distance_weight * distance_score
                    + self.params.confidence_weight * confidence
                    + self.params.metadata_weight * metadata_score;
                
                Ok(score.max(0.0).min(1.0))
            }
            ScoringMethod::ReciprocalRank | ScoringMethod::BordaCount => {
                // These require multiple result sets, so use hybrid as fallback
                let distance_score = 1.0 / (1.0 + result.distance);
                let metadata_score = self.calculate_metadata_score(result);
                
                let score = self.params.distance_weight * distance_score
                    + self.params.confidence_weight * confidence
                    + self.params.metadata_weight * metadata_score;
                
                Ok(score.max(0.0).min(1.0))
            }
        }
    }
    
    /// Calculate metadata-based score
    fn calculate_metadata_score(&self, result: &KnnResult) -> f32 {
        // Similar to confidence but for scoring
        let recency_score = {
            let current_time = 1640995200; // Placeholder
            let age_days = (current_time - result.modified_timestamp) / (24 * 3600);
            if age_days < 7 { 1.0 } else { 0.8 }
        };
        
        let size_score = if result.file_size > 500_000 { 0.9 } else { 0.7 };
        
        (recency_score + size_score) / 2.0
    }
    
    /// Calculate basic statistics for normalization
    fn calculate_statistics(&mut self, results: &[KnnResult]) -> Result<(), ScoringError> {
        if results.is_empty() {
            return Err(ScoringError::EmptyResults);
        }
        
        // Distance statistics
        let distances: Vec<f32> = results.iter().map(|r| r.distance).collect();
        let mean_dist = distances.iter().sum::<f32>() / distances.len() as f32;
        let var_dist = distances.iter()
            .map(|d| (d - mean_dist).powi(2))
            .sum::<f32>() / distances.len() as f32;
        let std_dist = var_dist.sqrt();
        
        self.distance_stats = Some((mean_dist, std_dist));
        
        Ok(())
    }
    
    /// Assess quality flags for a result
    fn assess_quality(&self, result: &KnnResult, confidence: f32) -> u32 {
        let mut flags = 0u32;
        
        if confidence > 0.8 {
            flags |= quality_flags::HIGH_CONFIDENCE;
        }
        
        if result.distance < 0.1 {
            flags |= quality_flags::LOW_DISTANCE;
        }
        
        let current_time = 1640995200; // Placeholder
        if (current_time - result.modified_timestamp) < (7 * 24 * 3600) {
            flags |= quality_flags::RECENT_FILE;
        }
        
        if result.file_size > 1_000_000 {
            flags |= quality_flags::LARGE_FILE;
        }
        
        if result.distance > 2.0 {
            flags |= quality_flags::OUTLIER_DISTANCE;
        }
        
        flags
    }
    
    /// Apply diversity filtering to reduce similar results
    fn apply_diversity_filter(
        &self,
        mut results: Vec<ScoredResult>,
    ) -> Result<Vec<ScoredResult>, ScoringError> {
        if results.len() <= 1 {
            return Ok(results);
        }
        
        let mut filtered = Vec::new();
        filtered.push(results.remove(0)); // Always keep the best result
        
        for candidate in results {
            let mut is_diverse = true;
            
            for existing in &filtered {
                // Calculate similarity (simplified)
                let distance_diff = (candidate.result.distance - existing.result.distance).abs();
                let normalized_diff = distance_diff / (candidate.result.distance + existing.result.distance + 1e-10);
                
                if normalized_diff < (1.0 - self.params.diversity_threshold) {
                    is_diverse = false;
                    break;
                }
            }
            
            if is_diverse {
                filtered.push(candidate);
            }
        }
        
        Ok(filtered)
    }
    
    /// Assign ranks and normalize scores
    fn assign_ranks_and_normalize(&self, results: &mut [ScoredResult]) -> Result<(), ScoringError> {
        if results.is_empty() {
            return Ok(());
        }
        
        // Assign ranks
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        // Normalize scores if requested
        if self.params.normalize_scores {
            let max_score = results.iter()
                .map(|r| r.score)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_score = results.iter()
                .map(|r| r.score)
                .fold(f32::INFINITY, f32::min);
            
            if max_score > min_score {
                let score_range = max_score - min_score;
                for result in results.iter_mut() {
                    result.normalized_score = (result.score - min_score) / score_range;
                }
            } else {
                // All scores are the same
                for result in results.iter_mut() {
                    result.normalized_score = 1.0;
                }
            }
        } else {
            for result in results.iter_mut() {
                result.normalized_score = result.score;
            }
        }
        
        Ok(())
    }
    
    /// Apply threshold filtering
    fn apply_threshold_filtering(
        &self,
        results: Vec<ScoredResult>,
    ) -> Result<Vec<ScoredResult>, ScoringError> {
        let filtered: Vec<ScoredResult> = results.into_iter()
            .filter(|r| {
                r.confidence >= self.params.min_confidence_threshold
                    && r.result.distance <= self.params.max_distance_threshold
            })
            .collect();
        
        Ok(filtered)
    }
    
    /// Calculate quality metrics for a result set
    pub fn calculate_quality_metrics(&self, results: &[ScoredResult]) -> QualityMetrics {
        if results.is_empty() {
            return QualityMetrics {
                avg_distance: 0.0,
                distance_std: 0.0,
                avg_confidence: 0.0,
                high_confidence_count: 0,
                diversity_score: 0.0,
                coverage_score: 0.0,
            };
        }
        
        let distances: Vec<f32> = results.iter().map(|r| r.result.distance).collect();
        let confidences: Vec<f32> = results.iter().map(|r| r.confidence).collect();
        
        let avg_distance = distances.iter().sum::<f32>() / distances.len() as f32;
        let distance_variance = distances.iter()
            .map(|d| (d - avg_distance).powi(2))
            .sum::<f32>() / distances.len() as f32;
        let distance_std = distance_variance.sqrt();
        
        let avg_confidence = confidences.iter().sum::<f32>() / confidences.len() as f32;
        
        let high_confidence_count = results.iter()
            .filter(|r| r.confidence > 0.8)
            .count();
        
        // Simple diversity calculation
        let diversity_score = if distances.len() > 1 {
            distance_std / (avg_distance + 1e-10)
        } else {
            0.0
        };
        
        // Simple coverage score (how well distributed the results are)
        let coverage_score = if results.len() > 1 {
            let max_dist = distances.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let min_dist = distances.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            if max_dist > min_dist {
                (max_dist - min_dist) / max_dist
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        QualityMetrics {
            avg_distance,
            distance_std,
            avg_confidence,
            high_confidence_count,
            diversity_score,
            coverage_score,
        }
    }
}

/// Result fusion utility for combining multiple search results
pub struct ResultFusion;

impl ResultFusion {
    /// Combine multiple result sets using reciprocal rank fusion
    pub fn reciprocal_rank_fusion(
        result_sets: &[Vec<ScoredResult>],
        k_constant: f32,
    ) -> Result<Vec<ScoredResult>, ScoringError> {
        if result_sets.is_empty() {
            return Err(ScoringError::EmptyResults);
        }
        
        let mut score_map: BTreeMap<u64, f32> = BTreeMap::new();
        let mut result_map: BTreeMap<u64, ScoredResult> = BTreeMap::new();
        
        for result_set in result_sets {
            for (rank, result) in result_set.iter().enumerate() {
                let vector_id = result.result.vector_id;
                let rrf_score = 1.0 / (k_constant + (rank + 1) as f32);
                
                *score_map.entry(vector_id).or_insert(0.0) += rrf_score;
                result_map.entry(vector_id).or_insert_with(|| result.clone());
            }
        }
        
        let mut combined_results: Vec<ScoredResult> = score_map.into_iter()
            .map(|(vector_id, score)| {
                let mut result = result_map.remove(&vector_id).unwrap();
                result.score = score;
                result
            })
            .collect();
        
        combined_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        // Reassign ranks
        for (i, result) in combined_results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        Ok(combined_results)
    }
    
    /// Combine multiple result sets using Borda count fusion
    pub fn borda_count_fusion(
        result_sets: &[Vec<ScoredResult>],
    ) -> Result<Vec<ScoredResult>, ScoringError> {
        if result_sets.is_empty() {
            return Err(ScoringError::EmptyResults);
        }
        
        let mut score_map: BTreeMap<u64, f32> = BTreeMap::new();
        let mut result_map: BTreeMap<u64, ScoredResult> = BTreeMap::new();
        
        for result_set in result_sets {
            let max_rank = result_set.len();
            for result in result_set {
                let vector_id = result.result.vector_id;
                let borda_score = (max_rank - result.rank + 1) as f32;
                
                *score_map.entry(vector_id).or_insert(0.0) += borda_score;
                result_map.entry(vector_id).or_insert_with(|| result.clone());
            }
        }
        
        let mut combined_results: Vec<ScoredResult> = score_map.into_iter()
            .map(|(vector_id, score)| {
                let mut result = result_map.remove(&vector_id).unwrap();
                result.score = score;
                result
            })
            .collect();
        
        combined_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        // Reassign ranks
        for (i, result) in combined_results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        Ok(combined_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_storage::VectorDataType;

    #[test]
    fn test_result_scoring() {
        let mut scorer = ResultScorer::new(ScoringParams::default());
        
        let results = vec![
            KnnResult {
                vector_id: 1,
                file_inode: 100,
                distance: 0.5,
                dimensions: 128,
                data_type: VectorDataType::Float32,
                file_size: 1024,
                created_timestamp: 1640995200,
                modified_timestamp: 1640995200,
            },
            KnnResult {
                vector_id: 2,
                file_inode: 101,
                distance: 1.0,
                dimensions: 128,
                data_type: VectorDataType::Float32,
                file_size: 2048,
                created_timestamp: 1640995200,
                modified_timestamp: 1640995200,
            },
        ];
        
        let search_params = SearchParams::default();
        let scored = scorer.score_and_rank(&results, &search_params).unwrap();
        
        assert_eq!(scored.len(), 2);
        assert!(scored[0].score >= scored[1].score); // Should be sorted by score
    }

    #[test]
    fn test_reciprocal_rank_fusion() {
        let set1 = vec![
            ScoredResult {
                result: KnnResult {
                    vector_id: 1,
                    file_inode: 100,
                    distance: 0.5,
                    dimensions: 128,
                    data_type: VectorDataType::Float32,
                    file_size: 1024,
                    created_timestamp: 1640995200,
                    modified_timestamp: 1640995200,
                },
                score: 0.9,
                confidence: 0.8,
                rank: 1,
                normalized_score: 1.0,
                quality_flags: 0,
            },
        ];
        
        let result_sets = vec![set1];
        let fused = ResultFusion::reciprocal_rank_fusion(&result_sets, 60.0).unwrap();
        
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].rank, 1);
    }
}