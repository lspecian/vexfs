"""
VexFS v2 Qdrant Adapter - Filter DSL Engine

This module implements complete Qdrant filter DSL support for VexFS v2,
providing high-performance filtering capabilities that leverage VexFS
metadata operations for optimal performance.
"""

from .filter_engine import FilterEngine
from .filter_parser import FilterParser
from .filter_executor import FilterExecutor
from .filter_types import (
    FilterCondition,
    MatchFilter,
    RangeFilter,
    GeoFilter,
    HasIdFilter,
    IsEmptyFilter,
    IsNullFilter,
    BooleanFilter
)

__all__ = [
    "FilterEngine",
    "FilterParser", 
    "FilterExecutor",
    "FilterCondition",
    "MatchFilter",
    "RangeFilter",
    "GeoFilter",
    "HasIdFilter",
    "IsEmptyFilter",
    "IsNullFilter",
    "BooleanFilter"
]