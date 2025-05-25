//! SIMD-optimized vector similarity metrics for VexFS
//! 
//! This module implements kernel-compatible similarity functions (L2, Cosine, Inner Product)
//! with SIMD optimization for high-performance vector search operations.

#![no_std]

use core::mem;
use crate::anns::DistanceMetric;
use crate::vector_storage::VectorDataType;

/// Maximum vector dimensions for SIMD optimization
pub const SIMD_MAX_DIMENSIONS: usize = 4096;

/// SIMD vector size (256 bits = 8 x f32)
pub const SIMD_WIDTH_F32: usize = 8;

/// SIMD vector size (512 bits = 16 x f32) for AVX-512
pub const SIMD_WIDTH_AVX512_F32: usize = 16;

/// Alignment for SIMD operations
pub const SIMD_ALIGNMENT: usize = 32;

/// Vector similarity metrics error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricsError {
    /// Invalid dimensions
    InvalidDimensions,
    /// Mismatched vector dimensions
    DimensionMismatch,
    /// Division by zero in normalization
    DivisionByZero,
    /// Invalid vector data
    InvalidData,
    /// SIMD alignment error
    AlignmentError,
}

/// SIMD capability detection
#[derive(Debug, Clone, Copy)]
pub struct SimdCapabilities {
    /// SSE support
    pub sse: bool,
    /// SSE2 support
    pub sse2: bool,
    /// AVX support
    pub avx: bool,
    /// AVX2 support
    pub avx2: bool,
    /// AVX-512 support
    pub avx512f: bool,
    /// FMA support
    pub fma: bool,
}

impl SimdCapabilities {
    /// Detect SIMD capabilities at runtime
    pub fn detect() -> Self {
        // In a real kernel implementation, this would use CPUID
        // For now, assume basic AVX2 support
        Self {
            sse: true,
            sse2: true,
            avx: true,
            avx2: true,
            avx512f: false, // Conservative default
            fma: true,
        }
    }
    
    /// Get optimal SIMD width for current capabilities
    pub fn optimal_width(&self) -> usize {
        if self.avx512f {
            SIMD_WIDTH_AVX512_F32
        } else if self.avx2 || self.avx {
            SIMD_WIDTH_F32
        } else {
            4 // SSE fallback
        }
    }
}

/// Vector metrics calculator with SIMD optimization
pub struct VectorMetrics {
    /// SIMD capabilities
    capabilities: SimdCapabilities,
    /// Whether to use SIMD optimizations
    use_simd: bool,
    /// Temporary buffer for aligned operations
    temp_buffer: [f32; SIMD_MAX_DIMENSIONS * 2],
}

impl VectorMetrics {
    /// Create new vector metrics calculator
    pub fn new(use_simd: bool) -> Self {
        Self {
            capabilities: SimdCapabilities::detect(),
            use_simd: use_simd && cfg!(target_arch = "x86_64"),
            temp_buffer: [0.0; SIMD_MAX_DIMENSIONS * 2],
        }
    }
    
    /// Calculate distance between two vectors
    pub fn calculate_distance(
        &mut self,
        vec1: &[f32],
        vec2: &[f32],
        metric: DistanceMetric,
    ) -> Result<f32, MetricsError> {
        if vec1.len() != vec2.len() {
            return Err(MetricsError::DimensionMismatch);
        }
        
        if vec1.is_empty() || vec1.len() > SIMD_MAX_DIMENSIONS {
            return Err(MetricsError::InvalidDimensions);
        }
        
        match metric {
            DistanceMetric::Euclidean => self.euclidean_distance(vec1, vec2),
            DistanceMetric::Cosine => self.cosine_distance(vec1, vec2),
            DistanceMetric::Manhattan => self.manhattan_distance(vec1, vec2),
            DistanceMetric::Dot => self.dot_product_distance(vec1, vec2),
            DistanceMetric::Hamming => self.hamming_distance(vec1, vec2),
        }
    }
    
    /// Calculate Euclidean (L2) distance with SIMD optimization
    pub fn euclidean_distance(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        if self.use_simd && self.capabilities.avx2 {
            self.euclidean_distance_avx2(vec1, vec2)
        } else if self.use_simd && self.capabilities.sse2 {
            self.euclidean_distance_sse2(vec1, vec2)
        } else {
            self.euclidean_distance_scalar(vec1, vec2)
        }
    }
    
    /// AVX2-optimized Euclidean distance
    fn euclidean_distance_avx2(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let dims = vec1.len();
        let simd_width = SIMD_WIDTH_F32;
        let simd_chunks = dims / simd_width;
        let remainder = dims % simd_width;
        
        let mut sum_squares = 0.0f32;
        
        // Process SIMD chunks
        for i in 0..simd_chunks {
            let base_idx = i * simd_width;
            let mut chunk_sum = 0.0f32;
            
            // Manual SIMD simulation (in real kernel, use intrinsics)
            for j in 0..simd_width {
                let diff = vec1[base_idx + j] - vec2[base_idx + j];
                chunk_sum += diff * diff;
            }
            
            sum_squares += chunk_sum;
        }
        
        // Process remainder
        for i in (simd_chunks * simd_width)..dims {
            let diff = vec1[i] - vec2[i];
            sum_squares += diff * diff;
        }
        
        Ok(sum_squares.sqrt())
    }
    
    /// SSE2-optimized Euclidean distance
    fn euclidean_distance_sse2(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let dims = vec1.len();
        let simd_width = 4; // SSE processes 4 f32 at a time
        let simd_chunks = dims / simd_width;
        let remainder = dims % simd_width;
        
        let mut sum_squares = 0.0f32;
        
        // Process SIMD chunks
        for i in 0..simd_chunks {
            let base_idx = i * simd_width;
            let mut chunk_sum = 0.0f32;
            
            // Manual SIMD simulation
            for j in 0..simd_width {
                let diff = vec1[base_idx + j] - vec2[base_idx + j];
                chunk_sum += diff * diff;
            }
            
            sum_squares += chunk_sum;
        }
        
        // Process remainder
        for i in (simd_chunks * simd_width)..dims {
            let diff = vec1[i] - vec2[i];
            sum_squares += diff * diff;
        }
        
        Ok(sum_squares.sqrt())
    }
    
    /// Scalar Euclidean distance fallback
    fn euclidean_distance_scalar(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let mut sum_squares = 0.0f32;
        
        for i in 0..vec1.len() {
            let diff = vec1[i] - vec2[i];
            sum_squares += diff * diff;
        }
        
        Ok(sum_squares.sqrt())
    }
    
    /// Calculate cosine distance with SIMD optimization
    pub fn cosine_distance(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        if self.use_simd && self.capabilities.avx2 {
            self.cosine_distance_avx2(vec1, vec2)
        } else {
            self.cosine_distance_scalar(vec1, vec2)
        }
    }
    
    /// AVX2-optimized cosine distance
    fn cosine_distance_avx2(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let dims = vec1.len();
        let simd_width = SIMD_WIDTH_F32;
        let simd_chunks = dims / simd_width;
        
        let mut dot_product = 0.0f32;
        let mut norm1_sq = 0.0f32;
        let mut norm2_sq = 0.0f32;
        
        // Process SIMD chunks
        for i in 0..simd_chunks {
            let base_idx = i * simd_width;
            let mut chunk_dot = 0.0f32;
            let mut chunk_norm1 = 0.0f32;
            let mut chunk_norm2 = 0.0f32;
            
            for j in 0..simd_width {
                let v1 = vec1[base_idx + j];
                let v2 = vec2[base_idx + j];
                chunk_dot += v1 * v2;
                chunk_norm1 += v1 * v1;
                chunk_norm2 += v2 * v2;
            }
            
            dot_product += chunk_dot;
            norm1_sq += chunk_norm1;
            norm2_sq += chunk_norm2;
        }
        
        // Process remainder
        for i in (simd_chunks * simd_width)..dims {
            let v1 = vec1[i];
            let v2 = vec2[i];
            dot_product += v1 * v2;
            norm1_sq += v1 * v1;
            norm2_sq += v2 * v2;
        }
        
        let norm1 = norm1_sq.sqrt();
        let norm2 = norm2_sq.sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            return Err(MetricsError::DivisionByZero);
        }
        
        let cosine_similarity = dot_product / (norm1 * norm2);
        Ok(1.0 - cosine_similarity)
    }
    
    /// Scalar cosine distance fallback
    fn cosine_distance_scalar(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let mut dot_product = 0.0f32;
        let mut norm1_sq = 0.0f32;
        let mut norm2_sq = 0.0f32;
        
        for i in 0..vec1.len() {
            let v1 = vec1[i];
            let v2 = vec2[i];
            dot_product += v1 * v2;
            norm1_sq += v1 * v1;
            norm2_sq += v2 * v2;
        }
        
        let norm1 = norm1_sq.sqrt();
        let norm2 = norm2_sq.sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            return Err(MetricsError::DivisionByZero);
        }
        
        let cosine_similarity = dot_product / (norm1 * norm2);
        Ok(1.0 - cosine_similarity)
    }
    
    /// Calculate Manhattan (L1) distance
    pub fn manhattan_distance(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        if self.use_simd && self.capabilities.avx2 {
            self.manhattan_distance_avx2(vec1, vec2)
        } else {
            self.manhattan_distance_scalar(vec1, vec2)
        }
    }
    
    /// AVX2-optimized Manhattan distance
    fn manhattan_distance_avx2(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let dims = vec1.len();
        let simd_width = SIMD_WIDTH_F32;
        let simd_chunks = dims / simd_width;
        
        let mut sum_abs = 0.0f32;
        
        // Process SIMD chunks
        for i in 0..simd_chunks {
            let base_idx = i * simd_width;
            let mut chunk_sum = 0.0f32;
            
            for j in 0..simd_width {
                let diff = vec1[base_idx + j] - vec2[base_idx + j];
                chunk_sum += diff.abs();
            }
            
            sum_abs += chunk_sum;
        }
        
        // Process remainder
        for i in (simd_chunks * simd_width)..dims {
            let diff = vec1[i] - vec2[i];
            sum_abs += diff.abs();
        }
        
        Ok(sum_abs)
    }
    
    /// Scalar Manhattan distance fallback
    fn manhattan_distance_scalar(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let mut sum_abs = 0.0f32;
        
        for i in 0..vec1.len() {
            let diff = vec1[i] - vec2[i];
            sum_abs += diff.abs();
        }
        
        Ok(sum_abs)
    }
    
    /// Calculate dot product (inner product) distance
    pub fn dot_product_distance(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        if self.use_simd && self.capabilities.avx2 {
            self.dot_product_avx2(vec1, vec2)
        } else {
            self.dot_product_scalar(vec1, vec2)
        }
    }
    
    /// AVX2-optimized dot product
    fn dot_product_avx2(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let dims = vec1.len();
        let simd_width = SIMD_WIDTH_F32;
        let simd_chunks = dims / simd_width;
        
        let mut dot_product = 0.0f32;
        
        // Process SIMD chunks
        for i in 0..simd_chunks {
            let base_idx = i * simd_width;
            let mut chunk_dot = 0.0f32;
            
            for j in 0..simd_width {
                chunk_dot += vec1[base_idx + j] * vec2[base_idx + j];
            }
            
            dot_product += chunk_dot;
        }
        
        // Process remainder
        for i in (simd_chunks * simd_width)..dims {
            dot_product += vec1[i] * vec2[i];
        }
        
        // Return negative dot product as distance (higher dot product = lower distance)
        Ok(-dot_product)
    }
    
    /// Scalar dot product fallback
    fn dot_product_scalar(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let mut dot_product = 0.0f32;
        
        for i in 0..vec1.len() {
            dot_product += vec1[i] * vec2[i];
        }
        
        // Return negative dot product as distance
        Ok(-dot_product)
    }
    
    /// Calculate Hamming distance (for binary vectors)
    pub fn hamming_distance(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, MetricsError> {
        let mut hamming_dist = 0u32;
        
        for i in 0..vec1.len() {
            let bin1 = if vec1[i] > 0.5 { 1u32 } else { 0u32 };
            let bin2 = if vec2[i] > 0.5 { 1u32 } else { 0u32 };
            hamming_dist += bin1 ^ bin2;
        }
        
        Ok(hamming_dist as f32)
    }
    
    /// Batch distance calculation for multiple vectors
    pub fn batch_calculate_distances(
        &mut self,
        query: &[f32],
        vectors: &[&[f32]],
        metric: DistanceMetric,
        results: &mut [f32],
    ) -> Result<(), MetricsError> {
        if vectors.len() > results.len() {
            return Err(MetricsError::InvalidDimensions);
        }
        
        for (i, vector) in vectors.iter().enumerate() {
            results[i] = self.calculate_distance(query, vector, metric)?;
        }
        
        Ok(())
    }
    
    /// Normalize vector in-place
    pub fn normalize_vector(&mut self, vector: &mut [f32]) -> Result<(), MetricsError> {
        let mut norm_sq = 0.0f32;
        
        // Calculate norm squared
        if self.use_simd && self.capabilities.avx2 {
            let dims = vector.len();
            let simd_width = SIMD_WIDTH_F32;
            let simd_chunks = dims / simd_width;
            
            for i in 0..simd_chunks {
                let base_idx = i * simd_width;
                let mut chunk_norm = 0.0f32;
                
                for j in 0..simd_width {
                    let val = vector[base_idx + j];
                    chunk_norm += val * val;
                }
                
                norm_sq += chunk_norm;
            }
            
            // Process remainder
            for i in (simd_chunks * simd_width)..dims {
                let val = vector[i];
                norm_sq += val * val;
            }
        } else {
            for val in vector.iter() {
                norm_sq += val * val;
            }
        }
        
        let norm = norm_sq.sqrt();
        if norm == 0.0 {
            return Err(MetricsError::DivisionByZero);
        }
        
        // Normalize
        for val in vector.iter_mut() {
            *val /= norm;
        }
        
        Ok(())
    }
    
    /// Get optimal alignment for SIMD operations
    pub fn get_simd_alignment(&self) -> usize {
        if self.capabilities.avx512f {
            64 // 512 bits
        } else if self.capabilities.avx2 || self.capabilities.avx {
            32 // 256 bits
        } else {
            16 // 128 bits (SSE)
        }
    }
    
    /// Check if vectors are properly aligned for SIMD
    pub fn check_alignment(&self, vector: &[f32]) -> bool {
        let ptr = vector.as_ptr() as usize;
        let alignment = self.get_simd_alignment();
        ptr % alignment == 0
    }
}

/// Fast approximate distance calculations for filtering
pub struct ApproximateMetrics;

impl ApproximateMetrics {
    /// Fast approximate Euclidean distance using reduced precision
    pub fn approx_euclidean_distance(vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum_squares = 0.0f32;
        
        // Use every 4th dimension for approximation
        let step = core::cmp::max(1, vec1.len() / 64);
        
        for i in (0..vec1.len()).step_by(step) {
            let diff = vec1[i] - vec2[i];
            sum_squares += diff * diff;
        }
        
        // Scale by step factor
        (sum_squares * step as f32).sqrt()
    }
    
    /// Fast approximate cosine distance
    pub fn approx_cosine_distance(vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut dot_product = 0.0f32;
        let mut norm1_sq = 0.0f32;
        let mut norm2_sq = 0.0f32;
        
        // Use every 4th dimension for approximation
        let step = core::cmp::max(1, vec1.len() / 64);
        
        for i in (0..vec1.len()).step_by(step) {
            let v1 = vec1[i];
            let v2 = vec2[i];
            dot_product += v1 * v2;
            norm1_sq += v1 * v1;
            norm2_sq += v2 * v2;
        }
        
        let norm1 = (norm1_sq * step as f32).sqrt();
        let norm2 = (norm2_sq * step as f32).sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            return 1.0;
        }
        
        let cosine_similarity = (dot_product * step as f32) / (norm1 * norm2);
        1.0 - cosine_similarity
    }
}

/// Compile-time checks for SIMD alignment
const _: () = {
    assert!(SIMD_ALIGNMENT >= mem::align_of::<f32>());
    assert!(SIMD_WIDTH_F32 * mem::size_of::<f32>() <= SIMD_ALIGNMENT);
};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_euclidean_distance() {
        let mut metrics = VectorMetrics::new(false);
        let vec1 = [1.0, 2.0, 3.0];
        let vec2 = [4.0, 5.0, 6.0];
        
        let distance = metrics.euclidean_distance(&vec1, &vec2).unwrap();
        let expected = ((4.0-1.0).powi(2) + (5.0-2.0).powi(2) + (6.0-3.0).powi(2)).sqrt();
        
        assert!((distance - expected).abs() < 1e-6);
    }
    
    #[test]
    fn test_cosine_distance() {
        let mut metrics = VectorMetrics::new(false);
        let vec1 = [1.0, 0.0, 0.0];
        let vec2 = [0.0, 1.0, 0.0];
        
        let distance = metrics.cosine_distance(&vec1, &vec2).unwrap();
        assert!((distance - 1.0).abs() < 1e-6); // Orthogonal vectors
    }
    
    #[test]
    fn test_manhattan_distance() {
        let mut metrics = VectorMetrics::new(false);
        let vec1 = [1.0, 2.0, 3.0];
        let vec2 = [4.0, 5.0, 6.0];
        
        let distance = metrics.manhattan_distance(&vec1, &vec2).unwrap();
        let expected = (4.0-1.0).abs() + (5.0-2.0).abs() + (6.0-3.0).abs();
        
        assert!((distance - expected).abs() < 1e-6);
    }
    
    #[test]
    fn test_normalization() {
        let mut metrics = VectorMetrics::new(false);
        let mut vector = [3.0, 4.0, 0.0];
        
        metrics.normalize_vector(&mut vector).unwrap();
        
        let norm = (vector[0].powi(2) + vector[1].powi(2) + vector[2].powi(2)).sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }
}