"""
VexFS v2 Qdrant Adapter - Recommendation System

This module implements advanced recommendation algorithms for VexFS v2,
providing positive/negative example recommendations, discovery features,
and strategy-based recommendations optimized for VexFS vector search.
"""

from .recommend_engine import RecommendationEngine
from .similarity import SimilarityCalculator
from .discovery import DiscoveryEngine

__all__ = [
    "RecommendationEngine",
    "SimilarityCalculator", 
    "DiscoveryEngine"
]