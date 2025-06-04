"""
Similarity Calculator

This module provides advanced similarity calculations for recommendation
algorithms, optimized for VexFS v2's high-performance vector operations.
"""

from typing import List, Dict, Any, Optional, Tuple
import logging
import math
from ..core.vexfs_client import VexFSClient, VexFSError

logger = logging.getLogger(__name__)


class SimilarityCalculator:
    """
    Advanced similarity calculator for recommendation systems.
    
    Provides multiple similarity calculation strategies optimized for
    VexFS v2's vector operations and IEEE 754 integer representation.
    """
    
    def __init__(self, vexfs_client: VexFSClient):
        """
        Initialize similarity calculator.
        
        Args:
            vexfs_client: VexFS client for vector operations
        """
        self.vexfs_client = vexfs_client
    
    def calculate_average_vector(self, collection: str, point_ids: List[str], 
                                weights: Optional[List[float]] = None) -> List[float]:
        """
        Calculate weighted average vector from multiple points.
        
        Args:
            collection: Collection name
            point_ids: List of point IDs to average
            weights: Optional weights for each point (default: equal weights)
            
        Returns:
            Average vector
            
        Raises:
            ValueError: If inputs are invalid
        """
        if not point_ids:
            raise ValueError("Point IDs list cannot be empty")
        
        if weights is not None and len(weights) != len(point_ids):
            raise ValueError("Weights must match number of point IDs")
        
        # Get vectors for all points
        vectors = self._get_vectors_for_points(collection, point_ids)
        
        if not vectors:
            raise ValueError("No vectors found for provided point IDs")
        
        # Ensure all vectors have the same dimensions
        dimensions = len(vectors[0])
        if not all(len(v) == dimensions for v in vectors):
            raise ValueError("All vectors must have the same dimensions")
        
        # Calculate weighted average
        if weights is None:
            weights = [1.0 / len(vectors)] * len(vectors)
        else:
            # Normalize weights
            weight_sum = sum(weights)
            weights = [w / weight_sum for w in weights]
        
        # Compute weighted average
        avg_vector = [0.0] * dimensions
        for i, vector in enumerate(vectors):
            weight = weights[i]
            for j in range(dimensions):
                avg_vector[j] += vector[j] * weight
        
        return avg_vector
    
    def calculate_centroid_vector(self, positive_vectors: List[List[float]], 
                                 negative_vectors: Optional[List[List[float]]] = None,
                                 positive_weight: float = 1.0,
                                 negative_weight: float = 0.5) -> List[float]:
        """
        Calculate centroid vector from positive and negative examples.
        
        Args:
            positive_vectors: Vectors to move towards
            negative_vectors: Vectors to move away from
            positive_weight: Weight for positive examples
            negative_weight: Weight for negative examples
            
        Returns:
            Centroid vector for recommendation
        """
        if not positive_vectors:
            raise ValueError("Must provide at least one positive vector")
        
        dimensions = len(positive_vectors[0])
        
        # Calculate positive centroid
        positive_centroid = [0.0] * dimensions
        for vector in positive_vectors:
            if len(vector) != dimensions:
                raise ValueError("All vectors must have the same dimensions")
            for i in range(dimensions):
                positive_centroid[i] += vector[i]
        
        # Average positive vectors
        for i in range(dimensions):
            positive_centroid[i] /= len(positive_vectors)
        
        # Apply positive weight
        for i in range(dimensions):
            positive_centroid[i] *= positive_weight
        
        # Handle negative examples
        if negative_vectors:
            negative_centroid = [0.0] * dimensions
            for vector in negative_vectors:
                if len(vector) != dimensions:
                    raise ValueError("All vectors must have the same dimensions")
                for i in range(dimensions):
                    negative_centroid[i] += vector[i]
            
            # Average negative vectors
            for i in range(dimensions):
                negative_centroid[i] /= len(negative_vectors)
            
            # Subtract weighted negative centroid
            for i in range(dimensions):
                positive_centroid[i] -= negative_centroid[i] * negative_weight
        
        return positive_centroid
    
    def calculate_best_score_vector(self, collection: str, point_ids: List[str],
                                   query_vector: List[float]) -> List[float]:
        """
        Calculate recommendation vector based on best scoring examples.
        
        Args:
            collection: Collection name
            point_ids: Point IDs to consider
            query_vector: Reference query vector
            
        Returns:
            Vector optimized for best score strategy
        """
        if not point_ids:
            raise ValueError("Point IDs list cannot be empty")
        
        # Get vectors and calculate similarities
        vectors = self._get_vectors_for_points(collection, point_ids)
        similarities = []
        
        for vector in vectors:
            similarity = self._calculate_cosine_similarity(query_vector, vector)
            similarities.append(similarity)
        
        # Weight vectors by their similarity scores
        weights = []
        max_similarity = max(similarities) if similarities else 1.0
        
        for similarity in similarities:
            # Use exponential weighting to emphasize best matches
            weight = math.exp(similarity / max_similarity)
            weights.append(weight)
        
        # Calculate weighted average
        return self.calculate_average_vector(collection, point_ids, weights)
    
    def calculate_diversity_vector(self, vectors: List[List[float]], 
                                  diversity_factor: float = 0.3) -> List[float]:
        """
        Calculate a vector that promotes diversity in recommendations.
        
        Args:
            vectors: Input vectors to diversify from
            diversity_factor: How much to emphasize diversity (0.0-1.0)
            
        Returns:
            Diversity-promoting vector
        """
        if not vectors:
            raise ValueError("Vectors list cannot be empty")
        
        dimensions = len(vectors[0])
        
        # Calculate centroid
        centroid = [0.0] * dimensions
        for vector in vectors:
            for i in range(dimensions):
                centroid[i] += vector[i]
        
        for i in range(dimensions):
            centroid[i] /= len(vectors)
        
        # Calculate variance for each dimension
        variances = [0.0] * dimensions
        for vector in vectors:
            for i in range(dimensions):
                diff = vector[i] - centroid[i]
                variances[i] += diff * diff
        
        for i in range(dimensions):
            variances[i] /= len(vectors)
        
        # Create diversity vector by emphasizing high-variance dimensions
        diversity_vector = centroid.copy()
        max_variance = max(variances) if variances else 1.0
        
        for i in range(dimensions):
            # Boost dimensions with high variance
            variance_weight = variances[i] / max_variance
            diversity_boost = diversity_factor * variance_weight
            diversity_vector[i] *= (1.0 + diversity_boost)
        
        return diversity_vector
    
    def calculate_vector_similarity_matrix(self, vectors: List[List[float]]) -> List[List[float]]:
        """
        Calculate similarity matrix between all pairs of vectors.
        
        Args:
            vectors: List of vectors to compare
            
        Returns:
            Similarity matrix (symmetric)
        """
        n = len(vectors)
        similarity_matrix = [[0.0] * n for _ in range(n)]
        
        for i in range(n):
            for j in range(i, n):
                if i == j:
                    similarity_matrix[i][j] = 1.0
                else:
                    similarity = self._calculate_cosine_similarity(vectors[i], vectors[j])
                    similarity_matrix[i][j] = similarity
                    similarity_matrix[j][i] = similarity  # Symmetric
        
        return similarity_matrix
    
    def find_most_diverse_subset(self, vectors: List[List[float]], 
                                subset_size: int) -> List[int]:
        """
        Find the most diverse subset of vectors using greedy selection.
        
        Args:
            vectors: Input vectors
            subset_size: Size of subset to select
            
        Returns:
            Indices of most diverse vectors
        """
        if subset_size >= len(vectors):
            return list(range(len(vectors)))
        
        if subset_size <= 0:
            return []
        
        # Calculate similarity matrix
        similarity_matrix = self.calculate_vector_similarity_matrix(vectors)
        
        # Greedy selection for diversity
        selected_indices = []
        
        # Start with the vector that has lowest average similarity to all others
        avg_similarities = []
        for i in range(len(vectors)):
            avg_sim = sum(similarity_matrix[i]) / len(vectors)
            avg_similarities.append(avg_sim)
        
        # Select first vector (least similar to others on average)
        first_idx = avg_similarities.index(min(avg_similarities))
        selected_indices.append(first_idx)
        
        # Greedily select remaining vectors
        while len(selected_indices) < subset_size:
            best_idx = -1
            best_min_similarity = float('inf')
            
            for i in range(len(vectors)):
                if i in selected_indices:
                    continue
                
                # Find minimum similarity to already selected vectors
                min_similarity = min(similarity_matrix[i][j] for j in selected_indices)
                
                # Select vector with highest minimum similarity (most diverse)
                if min_similarity < best_min_similarity:
                    best_min_similarity = min_similarity
                    best_idx = i
            
            if best_idx != -1:
                selected_indices.append(best_idx)
            else:
                break
        
        return selected_indices
    
    def _get_vectors_for_points(self, collection: str, point_ids: List[str]) -> List[List[float]]:
        """Get vectors for specified point IDs"""
        try:
            # In a full implementation, this would efficiently retrieve vectors from VexFS
            # For now, we'll simulate vector retrieval
            vectors = []
            
            for point_id in point_ids:
                # Simulate vector retrieval - in reality this would use VexFS metadata
                vector = self._simulate_vector_for_point(point_id)
                vectors.append(vector)
            
            return vectors
            
        except VexFSError as e:
            logger.error(f"Failed to retrieve vectors: {e}")
            return []
    
    def _simulate_vector_for_point(self, point_id: str, dimensions: int = 128) -> List[float]:
        """Simulate vector for a point (for testing purposes)"""
        # Generate deterministic vector based on point ID
        import hashlib
        
        # Create deterministic seed from point ID
        seed = int(hashlib.md5(point_id.encode()).hexdigest()[:8], 16)
        
        # Generate vector with consistent values
        vector = []
        for i in range(dimensions):
            # Use simple linear congruential generator for deterministic values
            seed = (seed * 1103515245 + 12345) & 0x7fffffff
            value = (seed / 0x7fffffff) * 2.0 - 1.0  # Normalize to [-1, 1]
            vector.append(value)
        
        # Normalize vector
        magnitude = math.sqrt(sum(x * x for x in vector))
        if magnitude > 0:
            vector = [x / magnitude for x in vector]
        
        return vector
    
    def _calculate_cosine_similarity(self, vector1: List[float], vector2: List[float]) -> float:
        """Calculate cosine similarity between two vectors"""
        if len(vector1) != len(vector2):
            raise ValueError("Vectors must have the same dimensions")
        
        # Calculate dot product
        dot_product = sum(a * b for a, b in zip(vector1, vector2))
        
        # Calculate magnitudes
        magnitude1 = math.sqrt(sum(a * a for a in vector1))
        magnitude2 = math.sqrt(sum(b * b for b in vector2))
        
        # Avoid division by zero
        if magnitude1 == 0 or magnitude2 == 0:
            return 0.0
        
        return dot_product / (magnitude1 * magnitude2)
    
    def _calculate_euclidean_distance(self, vector1: List[float], vector2: List[float]) -> float:
        """Calculate Euclidean distance between two vectors"""
        if len(vector1) != len(vector2):
            raise ValueError("Vectors must have the same dimensions")
        
        return math.sqrt(sum((a - b) ** 2 for a, b in zip(vector1, vector2)))
    
    def _calculate_dot_product(self, vector1: List[float], vector2: List[float]) -> float:
        """Calculate dot product between two vectors"""
        if len(vector1) != len(vector2):
            raise ValueError("Vectors must have the same dimensions")
        
        return sum(a * b for a, b in zip(vector1, vector2))