"""
Recommendation Engine

This module implements the main recommendation engine for VexFS v2 Qdrant adapter,
providing positive/negative example recommendations, discovery features, and
strategy-based recommendations optimized for VexFS vector search performance.
"""

from typing import List, Dict, Any, Optional, Union
import time
import logging
from enum import Enum
from ..core.vexfs_client import VexFSClient, VexFSError
from ..filters.filter_engine import FilterEngine
from .similarity import SimilarityCalculator
from .discovery import DiscoveryEngine

logger = logging.getLogger(__name__)


class RecommendationStrategy(str, Enum):
    """Recommendation strategies"""
    AVERAGE_VECTOR = "average_vector"
    BEST_SCORE = "best_score"
    CENTROID = "centroid"
    DIVERSITY = "diversity"


class RecommendationEngine:
    """
    Advanced recommendation system for Qdrant compatibility.
    
    This engine provides sophisticated recommendation algorithms that leverage
    VexFS v2's high-performance vector search capabilities to generate
    recommendations based on positive/negative examples, discovery algorithms,
    and various recommendation strategies.
    
    Performance targets:
    - Recommendation generation: <50ms for typical requests
    - Filter integration: >200K ops/sec
    - Memory efficient: <100MB for large operations
    """
    
    def __init__(self, vexfs_client: VexFSClient):
        """
        Initialize recommendation engine.
        
        Args:
            vexfs_client: VexFS client for vector operations
        """
        self.vexfs_client = vexfs_client
        self.similarity_calc = SimilarityCalculator(vexfs_client)
        self.discovery_engine = DiscoveryEngine(vexfs_client)
        self.filter_engine = FilterEngine(vexfs_client)
        
        # Performance tracking
        self._recommendation_stats = {
            'total_recommendations': 0,
            'successful_recommendations': 0,
            'failed_recommendations': 0,
            'total_execution_time': 0.0,
            'avg_execution_time': 0.0,
            'strategy_usage': {
                'average_vector': 0,
                'best_score': 0,
                'centroid': 0,
                'diversity': 0
            }
        }
    
    def recommend_points(self, collection: str, 
                        positive: Optional[List[str]] = None,
                        negative: Optional[List[str]] = None,
                        strategy: str = "average_vector",
                        limit: int = 10,
                        filter_condition: Optional[Dict[str, Any]] = None,
                        with_payload: bool = True,
                        with_vector: bool = False,
                        diversity_factor: float = 0.3) -> List[Dict[str, Any]]:
        """
        Generate recommendations based on positive/negative examples.
        
        Args:
            collection: Collection name
            positive: List of positive example point IDs
            negative: List of negative example point IDs
            strategy: Recommendation strategy to use
            limit: Maximum number of recommendations
            filter_condition: Optional filter to apply
            with_payload: Include payload in results
            with_vector: Include vector in results
            diversity_factor: Balance between similarity and diversity (0.0-1.0)
            
        Returns:
            List of ranked recommendations
            
        Raises:
            ValueError: If inputs are invalid
        """
        start_time = time.time()
        self._recommendation_stats['total_recommendations'] += 1
        
        try:
            # Validate inputs
            if not positive and not negative:
                raise ValueError("Must provide at least one positive or negative example")
            
            if limit <= 0 or limit > 1000:
                raise ValueError("Limit must be between 1 and 1000")
            
            # Track strategy usage
            if strategy in self._recommendation_stats['strategy_usage']:
                self._recommendation_stats['strategy_usage'][strategy] += 1
            
            # Calculate recommendation vector based on strategy
            recommendation_vector = self._calculate_recommendation_vector(
                collection, positive, negative, strategy
            )
            
            # Perform vector search with recommendation vector
            search_results = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=recommendation_vector,
                limit=limit * 3,  # Get more candidates for filtering/diversity
                distance="Cosine"
            )
            
            # Convert to point IDs for filtering
            candidate_ids = [str(result.vector_id) for result in search_results]
            
            # Apply filters if specified
            if filter_condition:
                filtered_ids = self.filter_engine.apply_filter(
                    collection, filter_condition, candidate_ids
                )
                # Keep only filtered results in original order
                search_results = [
                    result for result in search_results 
                    if str(result.vector_id) in filtered_ids
                ]
            
            # Exclude positive/negative examples from results
            excluded_ids = set()
            if positive:
                excluded_ids.update(positive)
            if negative:
                excluded_ids.update(negative)
            
            search_results = [
                result for result in search_results
                if str(result.vector_id) not in excluded_ids
            ]
            
            # Apply diversity filtering if requested
            if diversity_factor > 0:
                search_results = self._apply_diversity_filtering(
                    search_results, limit, diversity_factor
                )
            else:
                search_results = search_results[:limit]
            
            # Format results
            recommendations = []
            for result in search_results:
                recommendation = {
                    'id': str(result.vector_id),
                    'score': float(result.score),
                    'strategy': strategy
                }
                
                # Add payload if requested
                if with_payload:
                    payload = self._get_point_payload(collection, str(result.vector_id))
                    recommendation['payload'] = payload
                
                # Add vector if requested
                if with_vector:
                    vector = self._get_point_vector(collection, str(result.vector_id))
                    recommendation['vector'] = vector
                
                recommendations.append(recommendation)
            
            execution_time = time.time() - start_time
            
            logger.info(
                "Recommendations generated successfully",
                collection=collection,
                positive_count=len(positive) if positive else 0,
                negative_count=len(negative) if negative else 0,
                strategy=strategy,
                recommendations_count=len(recommendations),
                execution_time=execution_time
            )
            
            self._update_stats(start_time, True)
            return recommendations
            
        except Exception as e:
            self._update_stats(start_time, False)
            logger.error(f"Recommendation generation failed: {e}")
            raise
    
    def discover_points(self, collection: str, target: str,
                       limit: int = 10,
                       filter_condition: Optional[Dict[str, Any]] = None,
                       exploration_depth: int = 2,
                       diversity_factor: float = 0.3) -> List[Dict[str, Any]]:
        """
        Discover similar points for exploration.
        
        Args:
            collection: Collection name
            target: Target point ID for discovery
            limit: Maximum number of points to discover
            filter_condition: Optional filter to apply
            exploration_depth: How deep to explore (1-3)
            diversity_factor: Balance between similarity and diversity
            
        Returns:
            List of discovered points
        """
        start_time = time.time()
        
        try:
            # Use discovery engine for multi-hop exploration
            discovered_points = self.discovery_engine.discover_similar_points(
                collection=collection,
                target_point_id=target,
                limit=limit,
                exploration_depth=exploration_depth,
                diversity_factor=diversity_factor
            )
            
            # Apply filters if specified
            if filter_condition and discovered_points:
                point_ids = [point['id'] for point in discovered_points]
                filtered_ids = self.filter_engine.apply_filter(
                    collection, filter_condition, point_ids
                )
                discovered_points = [
                    point for point in discovered_points
                    if point['id'] in filtered_ids
                ]
            
            execution_time = time.time() - start_time
            
            logger.info(
                "Point discovery completed",
                collection=collection,
                target=target,
                discovered_count=len(discovered_points),
                execution_time=execution_time
            )
            
            return discovered_points[:limit]
            
        except Exception as e:
            logger.error(f"Point discovery failed: {e}")
            return []
    
    def recommend_with_context(self, collection: str,
                              context_points: List[str],
                              context_weights: Optional[List[float]] = None,
                              limit: int = 10,
                              filter_condition: Optional[Dict[str, Any]] = None) -> List[Dict[str, Any]]:
        """
        Generate recommendations based on contextual points with weights.
        
        Args:
            collection: Collection name
            context_points: List of context point IDs
            context_weights: Optional weights for each context point
            limit: Maximum number of recommendations
            filter_condition: Optional filter to apply
            
        Returns:
            List of contextual recommendations
        """
        try:
            # Calculate weighted average vector from context points
            context_vector = self.similarity_calc.calculate_average_vector(
                collection, context_points, context_weights
            )
            
            # Search for similar points
            search_results = self.vexfs_client.search_vectors(
                collection=collection,
                query_vector=context_vector,
                limit=limit * 2,
                distance="Cosine"
            )
            
            # Apply filters and exclude context points
            candidate_ids = [str(result.vector_id) for result in search_results]
            
            if filter_condition:
                filtered_ids = self.filter_engine.apply_filter(
                    collection, filter_condition, candidate_ids
                )
                search_results = [
                    result for result in search_results
                    if str(result.vector_id) in filtered_ids
                ]
            
            # Exclude context points
            search_results = [
                result for result in search_results
                if str(result.vector_id) not in context_points
            ]
            
            # Format results
            recommendations = []
            for result in search_results[:limit]:
                recommendations.append({
                    'id': str(result.vector_id),
                    'score': float(result.score),
                    'context_similarity': float(result.score)
                })
            
            return recommendations
            
        except Exception as e:
            logger.error(f"Contextual recommendation failed: {e}")
            return []
    
    def get_recommendation_explanation(self, collection: str,
                                     recommended_id: str,
                                     positive: Optional[List[str]] = None,
                                     negative: Optional[List[str]] = None) -> Dict[str, Any]:
        """
        Get explanation for why a point was recommended.
        
        Args:
            collection: Collection name
            recommended_id: ID of recommended point
            positive: Positive examples used
            negative: Negative examples used
            
        Returns:
            Explanation dictionary
        """
        try:
            explanation = {
                'recommended_id': recommended_id,
                'explanation_type': 'similarity_based',
                'factors': []
            }
            
            # Get recommended point vector
            recommended_vector = self._get_point_vector(collection, recommended_id)
            if not recommended_vector:
                return explanation
            
            # Analyze similarity to positive examples
            if positive:
                positive_similarities = []
                for pos_id in positive:
                    pos_vector = self._get_point_vector(collection, pos_id)
                    if pos_vector:
                        similarity = self.similarity_calc._calculate_cosine_similarity(
                            recommended_vector, pos_vector
                        )
                        positive_similarities.append({
                            'example_id': pos_id,
                            'similarity': similarity
                        })
                
                if positive_similarities:
                    avg_positive_similarity = sum(s['similarity'] for s in positive_similarities) / len(positive_similarities)
                    explanation['factors'].append({
                        'type': 'positive_similarity',
                        'average_similarity': avg_positive_similarity,
                        'individual_similarities': positive_similarities
                    })
            
            # Analyze dissimilarity to negative examples
            if negative:
                negative_similarities = []
                for neg_id in negative:
                    neg_vector = self._get_point_vector(collection, neg_id)
                    if neg_vector:
                        similarity = self.similarity_calc._calculate_cosine_similarity(
                            recommended_vector, neg_vector
                        )
                        negative_similarities.append({
                            'example_id': neg_id,
                            'similarity': similarity
                        })
                
                if negative_similarities:
                    avg_negative_similarity = sum(s['similarity'] for s in negative_similarities) / len(negative_similarities)
                    explanation['factors'].append({
                        'type': 'negative_dissimilarity',
                        'average_similarity': avg_negative_similarity,
                        'individual_similarities': negative_similarities
                    })
            
            return explanation
            
        except Exception as e:
            logger.error(f"Recommendation explanation failed: {e}")
            return {'error': str(e)}
    
    def _calculate_recommendation_vector(self, collection: str,
                                       positive: Optional[List[str]],
                                       negative: Optional[List[str]],
                                       strategy: str) -> List[float]:
        """Calculate recommendation vector based on strategy"""
        
        if strategy == RecommendationStrategy.AVERAGE_VECTOR:
            if positive:
                return self.similarity_calc.calculate_average_vector(collection, positive)
            else:
                raise ValueError("Average vector strategy requires positive examples")
        
        elif strategy == RecommendationStrategy.CENTROID:
            positive_vectors = []
            negative_vectors = []
            
            if positive:
                for point_id in positive:
                    vector = self._get_point_vector(collection, point_id)
                    if vector:
                        positive_vectors.append(vector)
            
            if negative:
                for point_id in negative:
                    vector = self._get_point_vector(collection, point_id)
                    if vector:
                        negative_vectors.append(vector)
            
            if not positive_vectors:
                raise ValueError("Centroid strategy requires positive examples")
            
            return self.similarity_calc.calculate_centroid_vector(
                positive_vectors, negative_vectors
            )
        
        elif strategy == RecommendationStrategy.BEST_SCORE:
            if not positive:
                raise ValueError("Best score strategy requires positive examples")
            
            # Use first positive example as reference
            reference_vector = self._get_point_vector(collection, positive[0])
            if not reference_vector:
                raise ValueError("Could not get reference vector")
            
            return self.similarity_calc.calculate_best_score_vector(
                collection, positive, reference_vector
            )
        
        elif strategy == RecommendationStrategy.DIVERSITY:
            if not positive:
                raise ValueError("Diversity strategy requires positive examples")
            
            positive_vectors = []
            for point_id in positive:
                vector = self._get_point_vector(collection, point_id)
                if vector:
                    positive_vectors.append(vector)
            
            if not positive_vectors:
                raise ValueError("Could not get positive vectors")
            
            return self.similarity_calc.calculate_diversity_vector(positive_vectors)
        
        else:
            raise ValueError(f"Unknown recommendation strategy: {strategy}")
    
    def _apply_diversity_filtering(self, search_results: List[Any],
                                  limit: int, diversity_factor: float) -> List[Any]:
        """Apply diversity filtering to search results"""
        if len(search_results) <= limit:
            return search_results
        
        # Get vectors for all results
        result_vectors = []
        for result in search_results:
            vector = self._get_point_vector("", str(result.vector_id))  # Collection not needed for simulation
            if vector:
                result_vectors.append(vector)
            else:
                result_vectors.append(None)
        
        # Select diverse subset
        selected_indices = []
        selected_vectors = []
        
        # Start with best scoring result
        selected_indices.append(0)
        if result_vectors[0]:
            selected_vectors.append(result_vectors[0])
        
        # Greedily select diverse results
        while len(selected_indices) < limit and len(selected_indices) < len(search_results):
            best_idx = -1
            best_score = -1
            
            for i, result in enumerate(search_results):
                if i in selected_indices or not result_vectors[i]:
                    continue
                
                # Calculate diversity score
                min_similarity = float('inf')
                for selected_vector in selected_vectors:
                    similarity = self.similarity_calc._calculate_cosine_similarity(
                        result_vectors[i], selected_vector
                    )
                    min_similarity = min(min_similarity, similarity)
                
                # Combine original score with diversity
                diversity_bonus = (1.0 - min_similarity) * diversity_factor
                combined_score = result.score + diversity_bonus
                
                if combined_score > best_score:
                    best_score = combined_score
                    best_idx = i
            
            if best_idx != -1:
                selected_indices.append(best_idx)
                if result_vectors[best_idx]:
                    selected_vectors.append(result_vectors[best_idx])
            else:
                break
        
        return [search_results[i] for i in selected_indices]
    
    def _get_point_vector(self, collection: str, point_id: str) -> Optional[List[float]]:
        """Get vector for a specific point"""
        # In a full implementation, this would retrieve from VexFS
        return self.similarity_calc._simulate_vector_for_point(point_id)
    
    def _get_point_payload(self, collection: str, point_id: str) -> Dict[str, Any]:
        """Get payload for a specific point"""
        # In a full implementation, this would retrieve from VexFS metadata
        return {"id": point_id, "metadata": "simulated"}
    
    def _update_stats(self, start_time: float, success: bool):
        """Update performance statistics"""
        execution_time = time.time() - start_time
        self._recommendation_stats['total_execution_time'] += execution_time
        
        if success:
            self._recommendation_stats['successful_recommendations'] += 1
        else:
            self._recommendation_stats['failed_recommendations'] += 1
        
        # Update average execution time
        total_recommendations = self._recommendation_stats['total_recommendations']
        if total_recommendations > 0:
            self._recommendation_stats['avg_execution_time'] = (
                self._recommendation_stats['total_execution_time'] / total_recommendations
            )
    
    def get_recommendation_statistics(self) -> Dict[str, Any]:
        """Get recommendation engine performance statistics"""
        return {
            'recommendation_stats': self._recommendation_stats.copy(),
            'performance_targets': {
                'target_generation_time_ms': 50,
                'target_filter_ops_per_sec': 200000,
                'target_memory_usage_mb': 100
            }
        }
    
    def clear_statistics(self):
        """Clear all performance statistics"""
        self._recommendation_stats = {
            'total_recommendations': 0,
            'successful_recommendations': 0,
            'failed_recommendations': 0,
            'total_execution_time': 0.0,
            'avg_execution_time': 0.0,
            'strategy_usage': {
                'average_vector': 0,
                'best_score': 0,
                'centroid': 0,
                'diversity': 0
            }
        }