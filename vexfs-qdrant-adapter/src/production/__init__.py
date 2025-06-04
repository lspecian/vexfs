"""
VexFS v2 Qdrant Adapter - Phase 4 Production Optimizations

Production-grade optimizations and enhancements for enterprise deployment.
"""

__version__ = "2.0.0-phase4"
__author__ = "VexFS Team"

from .optimization import ProductionOptimizations
from .connection_pool import ConnectionPoolManager
from .caching import IntelligentCaching
from .security import ProductionSecurity

__all__ = [
    "ProductionOptimizations",
    "ConnectionPoolManager", 
    "IntelligentCaching",
    "ProductionSecurity"
]