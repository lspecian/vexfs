/**
 * VexFS Python Bindings - Vector Operations
 *
 * Optimized vector operations for VexFS Python bindings
 */

#include <vector>
#include <cmath>
#include <algorithm>
#include <stdexcept>
#include <cstdint>
#include <immintrin.h>  // For SIMD operations

namespace vexfs {
namespace vector_ops {

/**
 * SIMD-optimized vector normalization
 */
std::vector<float> normalize_vector_simd(const std::vector<float>& vector) {
    if (vector.empty()) {
        throw std::invalid_argument("Vector cannot be empty");
    }
    
    const size_t size = vector.size();
    const size_t simd_size = size - (size % 8);
    
    // Calculate magnitude using SIMD
    __m256 sum_vec = _mm256_setzero_ps();
    
    for (size_t i = 0; i < simd_size; i += 8) {
        __m256 v = _mm256_loadu_ps(&vector[i]);
        sum_vec = _mm256_fmadd_ps(v, v, sum_vec);
    }
    
    // Sum the SIMD register
    float magnitude_squared = 0.0f;
    float temp[8];
    _mm256_storeu_ps(temp, sum_vec);
    for (int i = 0; i < 8; ++i) {
        magnitude_squared += temp[i];
    }
    
    // Handle remaining elements
    for (size_t i = simd_size; i < size; ++i) {
        magnitude_squared += vector[i] * vector[i];
    }
    
    float magnitude = std::sqrt(magnitude_squared);
    if (magnitude == 0.0f) {
        throw std::invalid_argument("Cannot normalize zero vector");
    }
    
    // Normalize using SIMD
    std::vector<float> normalized(size);
    __m256 mag_vec = _mm256_set1_ps(magnitude);
    
    for (size_t i = 0; i < simd_size; i += 8) {
        __m256 v = _mm256_loadu_ps(&vector[i]);
        __m256 result = _mm256_div_ps(v, mag_vec);
        _mm256_storeu_ps(&normalized[i], result);
    }
    
    // Handle remaining elements
    for (size_t i = simd_size; i < size; ++i) {
        normalized[i] = vector[i] / magnitude;
    }
    
    return normalized;
}

/**
 * SIMD-optimized cosine similarity
 */
float cosine_similarity_simd(const std::vector<float>& a, const std::vector<float>& b) {
    if (a.size() != b.size()) {
        throw std::invalid_argument("Vectors must have the same dimension");
    }
    
    if (a.empty()) {
        throw std::invalid_argument("Vectors cannot be empty");
    }
    
    const size_t size = a.size();
    const size_t simd_size = size - (size % 8);
    
    __m256 dot_vec = _mm256_setzero_ps();
    __m256 norm_a_vec = _mm256_setzero_ps();
    __m256 norm_b_vec = _mm256_setzero_ps();
    
    for (size_t i = 0; i < simd_size; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        
        dot_vec = _mm256_fmadd_ps(va, vb, dot_vec);
        norm_a_vec = _mm256_fmadd_ps(va, va, norm_a_vec);
        norm_b_vec = _mm256_fmadd_ps(vb, vb, norm_b_vec);
    }
    
    // Sum the SIMD registers
    float dot_product = 0.0f, norm_a = 0.0f, norm_b = 0.0f;
    
    float temp_dot[8], temp_norm_a[8], temp_norm_b[8];
    _mm256_storeu_ps(temp_dot, dot_vec);
    _mm256_storeu_ps(temp_norm_a, norm_a_vec);
    _mm256_storeu_ps(temp_norm_b, norm_b_vec);
    
    for (int i = 0; i < 8; ++i) {
        dot_product += temp_dot[i];
        norm_a += temp_norm_a[i];
        norm_b += temp_norm_b[i];
    }
    
    // Handle remaining elements
    for (size_t i = simd_size; i < size; ++i) {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    norm_a = std::sqrt(norm_a);
    norm_b = std::sqrt(norm_b);
    
    if (norm_a == 0.0f || norm_b == 0.0f) {
        return 0.0f;
    }
    
    return dot_product / (norm_a * norm_b);
}

/**
 * SIMD-optimized Euclidean distance
 */
float euclidean_distance_simd(const std::vector<float>& a, const std::vector<float>& b) {
    if (a.size() != b.size()) {
        throw std::invalid_argument("Vectors must have the same dimension");
    }
    
    const size_t size = a.size();
    const size_t simd_size = size - (size % 8);
    
    __m256 sum_vec = _mm256_setzero_ps();
    
    for (size_t i = 0; i < simd_size; i += 8) {
        __m256 va = _mm256_loadu_ps(&a[i]);
        __m256 vb = _mm256_loadu_ps(&b[i]);
        __m256 diff = _mm256_sub_ps(va, vb);
        sum_vec = _mm256_fmadd_ps(diff, diff, sum_vec);
    }
    
    // Sum the SIMD register
    float distance_squared = 0.0f;
    float temp[8];
    _mm256_storeu_ps(temp, sum_vec);
    for (int i = 0; i < 8; ++i) {
        distance_squared += temp[i];
    }
    
    // Handle remaining elements
    for (size_t i = simd_size; i < size; ++i) {
        float diff = a[i] - b[i];
        distance_squared += diff * diff;
    }
    
    return std::sqrt(distance_squared);
}

/**
 * Batch vector operations for efficiency
 */
std::vector<float> batch_cosine_similarity(const std::vector<float>& query,
                                          const std::vector<std::vector<float>>& vectors) {
    std::vector<float> similarities;
    similarities.reserve(vectors.size());
    
    for (const auto& vec : vectors) {
        similarities.push_back(cosine_similarity_simd(query, vec));
    }
    
    return similarities;
}

/**
 * Top-K similarity search
 */
std::vector<std::pair<size_t, float>> top_k_similar(const std::vector<float>& query,
                                                    const std::vector<std::vector<float>>& vectors,
                                                    size_t k) {
    if (k > vectors.size()) {
        k = vectors.size();
    }
    
    std::vector<std::pair<size_t, float>> similarities;
    similarities.reserve(vectors.size());
    
    for (size_t i = 0; i < vectors.size(); ++i) {
        float sim = cosine_similarity_simd(query, vectors[i]);
        similarities.emplace_back(i, sim);
    }
    
    // Partial sort to get top-k
    std::partial_sort(similarities.begin(), similarities.begin() + k, similarities.end(),
                     [](const auto& a, const auto& b) { return a.second > b.second; });
    
    similarities.resize(k);
    return similarities;
}

/**
 * Vector quantization for memory efficiency
 */
std::vector<uint8_t> quantize_vector(const std::vector<float>& vector, float min_val, float max_val) {
    std::vector<uint8_t> quantized;
    quantized.reserve(vector.size());
    
    float scale = 255.0f / (max_val - min_val);
    
    for (float val : vector) {
        float normalized = (val - min_val) * scale;
        quantized.push_back(static_cast<uint8_t>(std::clamp(normalized, 0.0f, 255.0f)));
    }
    
    return quantized;
}

/**
 * Dequantize vector
 */
std::vector<float> dequantize_vector(const std::vector<uint8_t>& quantized, 
                                    float min_val, float max_val) {
    std::vector<float> dequantized;
    dequantized.reserve(quantized.size());
    
    float scale = (max_val - min_val) / 255.0f;
    
    for (uint8_t val : quantized) {
        dequantized.push_back(min_val + val * scale);
    }
    
    return dequantized;
}

} // namespace vector_ops
} // namespace vexfs