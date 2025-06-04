"""
Discovery Engine

This module implements discovery algorithms for exploring similar vectors
and finding diverse recommendations using VexFS v2's vector search capabilities.
"""

from typing import List, Dict, Any, Optional, Set, Tuple
import logging
import random
import math
from ..core.vexfs_client import VexFSClient, VexFSError, SearchResult
from .similarity import SimilarityCalculator

logger = logging.getLogger(__name__)


class DiscoveryEngine:
    """
    Advanced discovery engine for vector exploration.
    
    Provides algorithms for discovering similar vectors, exploring
    vector neighborhoods, and finding diverse recommendations.
    """
    
    def __init__(self, vexfs_client: VexFSClient):
        """
        Initialize discovery engine.
        
        Args:
            vexfs_client: VexFS client for vector operations
        """
        self.vexfs_client = vexfs_client
        self.similarity_calc = SimilarityCalculator(vexfs_client)
    
    def discover_similar_points(self, collection: str, target_point_id: str,
                               limit: int = 10, exploration_depth: int = 2,
                               diversity_factor: float = 0.3) -> List[Dict[str, Any]]:
        """
        Discover similar points through multi-hop exploration.
        
        Args:
            collection: Collection name
            target_point_id: Starting point for discovery
            limit: Maximum number of points to return
            exploration_depth: How many hops to explore
            diversity_factor: Balance between similarity and diversity (0.0-1.0)
            
        Returns:
            List of discovered points with scores and metadata
        """
        try:
            # Get the target vector
            target_vector = self._get_vector_for_point(collection, target_point_id)
            if not target_vector:
                raise ValueError(f"Target point {target_point_id} not found")
            
            discovered_points = {}
            exploration_queue = [(target_point_id, target_vector, 1.0, 0)]  # (id, vector, score, depth)
            visited = {target_point_id}
            
            while exploration_queue and len(discovered_points) < limit * 3:  # Explore more than needed
                current_id, current_vector, current_score, depth = exploration_queue.pop(0)
                
                if depth >= exploration_depth:
                    continue
                
                # Find similar points to current point
                similar_results = self.vexfs_client.search_vectors(
                    collection=collection,
                    query_vector=current_vector,
                    limit=min(20, limit * 2),  # Search for more candidates
                    distance="Cosine"
                )
                
                for result in similar_results:
                    point_id = str(result.vector_id)
                    
                    if point_id in visited:
                        continue
                    
                    visited.add(point_id)
                    
                    # Calculate discovery score (combination of similarity and exploration depth)
                    discovery_score = result.score * (0.9 ** depth)  # Decay with depth
                    
                    discovered_points[point_id] = {
                        'id': point_id,
                        'score': discovery_score,
                        'similarity_to_target': result.score,
                        'discovery_depth': depth + 1,
                        'discovery_path': f"{current_id} -> {point_id}"
                    }
                    
                    # Add to exploration queue for next depth
                    if depth + 1 < exploration_depth:
                        point_vector = self._get_vector_for_point(collection, point_id)
                        if point_vector:
                            exploration_queue.append((point_id, point_vector, discovery_score, depth + 1))
            
            # Apply diversity filtering
            diverse_points = self._apply_diversity_filtering(
                list(discovered_points.values()),
                limit,
                diversity_factor
            )
            
            return diverse_points
            
        except Exception as e:
            logger.error(f"Discovery failed: {e}")
            return []
    
    def explore_vector_neighborhood(self, collection: str, center_vector: List[float],
                                   radius_steps: int = 3, points_per_step: int = 5) -> List[Dict[str, Any]]:
        """
        Explore the neighborhood around a vector in multiple radius steps.
        
        Args:
            collection: Collection name
            center_vector: Center vector for exploration
            radius_steps: Number of radius steps to explore
            points_per_step: Points to find at each radius step
            
        Returns:
            List of points organized by exploration radius
        """
        neighborhood_points = []
        
        try:
            for step in range(radius_steps):
                # Create slightly perturbed vectors for exploration
                exploration_vectors = self._generate_exploration_vectors(
                    center_vector, 
                    step + 1, 
                    points_per_step
                )
                
                for i, exploration_vector in enumerate(exploration_vectors):
                    # Search around each exploration vector
                    results = self.vexfs_client.search_vectors(
                        collection=collection,
                        query_vector=exploration_vector,
                        limit=points_per_step,
                        distance="Cosine"
                    )
                    
                    for result in results:
                        neighborhood_points.append({
                            'id': str(result.vector_id),
                            'score': result.score,
                            'exploration_step': step + 1,
                            'exploration_vector_index': i,
                            'distance_from_center': self._calculate_vector_distance(
                                center_vector, exploration_vector
                            )
                        })
            
            # Remove duplicates and sort by score
            unique_points = {}
            for point in neighborhood_points:
                point_id = point['id']
                if point_id not in unique_points or point['score'] > unique_points[point_id]['score']:
                    unique_points[point_id] = point
            
            return sorted(unique_points.values(), key=lambda x: x['score'], reverse=True)
            
        except Exception as e:
            logger.error(f"Neighborhood exploration failed: {e}")
            return []
    
    def find_diverse_recommendations(self, collection: str, seed_points: List[str],
                                   target_count: int = 20, diversity_threshold: float = 0.7) -> List[Dict[str, Any]]:
        """
        Find diverse recommendations starting from seed points.
        
        Args:
            collection: Collection name
            seed_points: Starting point IDs for recommendation
            target_count: Target number of diverse recommendations
            diversity_threshold: Minimum diversity score between recommendations
            
        Returns:
            List of diverse recommendations
        """
        try:
            # Get vectors for seed points
            seed_vectors = []
            for point_id in seed_points:
                vector = self._get_vector_for_point(collection, point_id)
                if vector:
                    seed_vectors.append((point_id, vector))
            
            if not seed_vectors:
                return []
            
            # Calculate centroid of seed vectors
            centroid = self.similarity_calc.calculate_average_vector(
                collection, [point_id for point_id, _ in seed_vectors]
            )
            
            # Find candidates around centroid
            candidates = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=centroid,
                limit=target_count * 5,  # Get more candidates than needed
                distance="Cosine"
            )
            
            # Apply diversity selection
            diverse_recommendations = []
            candidate_vectors = {}
            
            for result in candidates:
                point_id = str(result.vector_id)
                if point_id not in seed_points:  # Exclude seed points
                    vector = self._get_vector_for_point(collection, point_id)
                    if vector:
                        candidate_vectors[point_id] = {
                            'id': point_id,
                            'vector': vector,
                            'score': result.score
                        }
            
            # Greedy diversity selection
            while len(diverse_recommendations) < target_count and candidate_vectors:
                best_candidate = None
                best_diversity_score = -1
                
                for candidate_id, candidate_data in candidate_vectors.items():
                    # Calculate diversity score
                    diversity_score = self._calculate_diversity_score(
                        candidate_data['vector'],
                        [rec['vector'] for rec in diverse_recommendations],
                        diversity_threshold
                    )
                    
                    # Combine similarity and diversity
                    combined_score = candidate_data['score'] * 0.6 + diversity_score * 0.4
                    
                    if combined_score > best_diversity_score:
                        best_diversity_score = combined_score
                        best_candidate = candidate_id
                
                if best_candidate:
                    candidate_data = candidate_vectors.pop(best_candidate)
                    diverse_recommendations.append({
                        'id': candidate_data['id'],
                        'score': candidate_data['score'],
                        'diversity_score': best_diversity_score,
                        'vector': candidate_data['vector']
                    })
                else:
                    break
            
            # Remove vector data from final results
            for rec in diverse_recommendations:
                del rec['vector']
            
            return diverse_recommendations
            
        except Exception as e:
            logger.error(f"Diverse recommendation failed: {e}")
            return []
    
    def explore_semantic_clusters(self, collection: str, query_vector: List[float],
                                 cluster_count: int = 5, points_per_cluster: int = 10) -> Dict[str, List[Dict[str, Any]]]:
        """
        Explore semantic clusters around a query vector.
        
        Args:
            collection: Collection name
            query_vector: Query vector for exploration
            cluster_count: Number of clusters to explore
            points_per_cluster: Points to find in each cluster
            
        Returns:
            Dictionary mapping cluster IDs to lists of points
        """
        try:
            # Generate cluster exploration vectors
            cluster_vectors = self._generate_cluster_vectors(query_vector, cluster_count)
            
            clusters = {}
            
            for i, cluster_vector in enumerate(cluster_vectors):
                cluster_id = f"cluster_{i}"
                
                # Search for points in this cluster
                results = self.vexfs_client.search_vectors(
                    collection=collection,
                    query_vector=cluster_vector,
                    limit=points_per_cluster,
                    distance="Cosine"
                )
                
                cluster_points = []
                for result in results:
                    cluster_points.append({
                        'id': str(result.vector_id),
                        'score': result.score,
                        'cluster_similarity': self.similarity_calc._calculate_cosine_similarity(
                            query_vector, cluster_vector
                        )
                    })
                
                clusters[cluster_id] = cluster_points
            
            return clusters
            
        except Exception as e:
            logger.error(f"Semantic cluster exploration failed: {e}")
            return {}
    
    def _apply_diversity_filtering(self, points: List[Dict[str, Any]], 
                                  limit: int, diversity_factor: float) -> List[Dict[str, Any]]:
        """Apply diversity filtering to a list of points"""
        if len(points) <= limit:
            return sorted(points, key=lambda x: x['score'], reverse=True)
        
        # Sort by score initially
        points.sort(key=lambda x: x['score'], reverse=True)
        
        # Select diverse subset
        selected = [points[0]]  # Start with best scoring point
        remaining = points[1:]
        
        while len(selected) < limit and remaining:
            best_candidate = None
            best_combined_score = -1
            
            for candidate in remaining:
                # Calculate diversity bonus
                diversity_bonus = diversity_factor * len(selected) / limit
                combined_score = candidate['score'] + diversity_bonus
                
                if combined_score > best_combined_score:
                    best_combined_score = combined_score
                    best_candidate = candidate
            
            if best_candidate:
                selected.append(best_candidate)
                remaining.remove(best_candidate)
            else:
                break
        
        return selected
    
    def _generate_exploration_vectors(self, center_vector: List[float], 
                                    step: int, count: int) -> List[List[float]]:
        """Generate vectors for neighborhood exploration"""
        exploration_vectors = []
        dimensions = len(center_vector)
        
        # Calculate perturbation magnitude based on step
        perturbation_magnitude = 0.1 * step
        
        for _ in range(count):
            # Generate random perturbation
            perturbation = []
            for _ in range(dimensions):
                perturbation.append(random.gauss(0, perturbation_magnitude))
            
            # Apply perturbation to center vector
            exploration_vector = []
            for i in range(dimensions):
                exploration_vector.append(center_vector[i] + perturbation[i])
            
            # Normalize the vector
            magnitude = math.sqrt(sum(x * x for x in exploration_vector))
            if magnitude > 0:
                exploration_vector = [x / magnitude for x in exploration_vector]
            
            exploration_vectors.append(exploration_vector)
        
        return exploration_vectors
    
    def _generate_cluster_vectors(self, query_vector: List[float], 
                                cluster_count: int) -> List[List[float]]:
        """Generate vectors for semantic cluster exploration"""
        cluster_vectors = []
        dimensions = len(query_vector)
        
        for i in range(cluster_count):
            # Generate cluster vector with systematic variation
            angle = (2 * math.pi * i) / cluster_count
            cluster_vector = []
            
            for j in range(dimensions):
                # Apply rotation-like transformation
                if j % 2 == 0:
                    value = query_vector[j] * math.cos(angle) - query_vector[min(j+1, dimensions-1)] * math.sin(angle)
                else:
                    value = query_vector[j] * math.sin(angle) + query_vector[j-1] * math.cos(angle)
                
                cluster_vector.append(value)
            
            # Normalize
            magnitude = math.sqrt(sum(x * x for x in cluster_vector))
            if magnitude > 0:
                cluster_vector = [x / magnitude for x in cluster_vector]
            
            cluster_vectors.append(cluster_vector)
        
        return cluster_vectors
    
    def _calculate_diversity_score(self, candidate_vector: List[float],
                                 selected_vectors: List[List[float]],
                                 diversity_threshold: float) -> float:
        """Calculate diversity score for a candidate vector"""
        if not selected_vectors:
            return 1.0
        
        # Calculate minimum similarity to selected vectors
        min_similarity = float('inf')
        for selected_vector in selected_vectors:
            similarity = self.similarity_calc._calculate_cosine_similarity(
                candidate_vector, selected_vector
            )
            min_similarity = min(min_similarity, similarity)
        
        # Convert to diversity score
        diversity_score = 1.0 - min_similarity
        
        # Apply threshold
        if min_similarity > diversity_threshold:
            diversity_score *= 0.1  # Heavily penalize similar vectors
        
        return max(0.0, diversity_score)
    
    def _calculate_vector_distance(self, vector1: List[float], vector2: List[float]) -> float:
        """Calculate Euclidean distance between vectors"""
        return math.sqrt(sum((a - b) ** 2 for a, b in zip(vector1, vector2)))
    
    def _get_vector_for_point(self, collection: str, point_id: str) -> Optional[List[float]]:
        """Get vector for a specific point"""
        try:
            # In a full implementation, this would retrieve the vector from VexFS
            # For now, simulate vector retrieval
            return self.similarity_calc._simulate_vector_for_point(point_id)
        except Exception as e:
            logger.error(f"Failed to get vector for point {point_id}: {e}")
            return None