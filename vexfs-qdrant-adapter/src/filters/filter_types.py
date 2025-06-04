"""
Filter Type Definitions

This module defines all filter types supported by the Qdrant Filter DSL,
providing type safety and validation for complex filter operations.
"""

from typing import Dict, List, Any, Union, Optional
from pydantic import BaseModel, Field, validator
from enum import Enum
import uuid


class FilterType(str, Enum):
    """Supported filter types"""
    MATCH = "match"
    RANGE = "range"
    GEO_RADIUS = "geo_radius"
    GEO_BOUNDING_BOX = "geo_bounding_box"
    HAS_ID = "has_id"
    IS_EMPTY = "is_empty"
    IS_NULL = "is_null"
    MUST = "must"
    MUST_NOT = "must_not"
    SHOULD = "should"


class FilterCondition(BaseModel):
    """Base filter condition"""
    key: Optional[str] = Field(default=None, description="Field key to filter on")
    
    class Config:
        extra = "allow"


class MatchFilter(FilterCondition):
    """Match filter for exact value matching"""
    match: Dict[str, Any] = Field(..., description="Match condition")
    
    @validator('match')
    def validate_match(cls, v):
        if 'value' not in v:
            raise ValueError("Match filter must have 'value' field")
        return v


class RangeFilter(FilterCondition):
    """Range filter for numeric/date ranges"""
    range: Dict[str, Union[int, float]] = Field(..., description="Range condition")
    
    @validator('range')
    def validate_range(cls, v):
        valid_ops = {'gte', 'gt', 'lte', 'lt'}
        if not any(op in v for op in valid_ops):
            raise ValueError(f"Range filter must have at least one of: {valid_ops}")
        return v


class GeoPoint(BaseModel):
    """Geographic point"""
    lat: float = Field(..., ge=-90, le=90, description="Latitude")
    lon: float = Field(..., ge=-180, le=180, description="Longitude")


class GeoFilter(FilterCondition):
    """Geographic filter for location-based queries"""
    geo_radius: Optional[Dict[str, Any]] = Field(default=None, description="Geo radius filter")
    geo_bounding_box: Optional[Dict[str, Any]] = Field(default=None, description="Geo bounding box filter")
    
    @validator('geo_radius')
    def validate_geo_radius(cls, v):
        if v is not None:
            if 'center' not in v or 'radius' not in v:
                raise ValueError("Geo radius filter must have 'center' and 'radius'")
            center = v['center']
            if not isinstance(center, (list, tuple)) or len(center) != 2:
                raise ValueError("Geo center must be [lat, lon] array")
        return v
    
    @validator('geo_bounding_box')
    def validate_geo_bounding_box(cls, v):
        if v is not None:
            if 'top_left' not in v or 'bottom_right' not in v:
                raise ValueError("Geo bounding box must have 'top_left' and 'bottom_right'")
        return v


class HasIdFilter(BaseModel):
    """Filter for specific point IDs"""
    has_id: List[Union[int, str, uuid.UUID]] = Field(..., description="Point IDs to match")
    
    @validator('has_id')
    def validate_has_id(cls, v):
        if not v:
            raise ValueError("has_id filter cannot be empty")
        return v


class IsEmptyFilter(FilterCondition):
    """Filter for empty fields"""
    is_empty: Dict[str, Any] = Field(default_factory=dict, description="Empty field condition")


class IsNullFilter(FilterCondition):
    """Filter for null fields"""
    is_null: Dict[str, Any] = Field(default_factory=dict, description="Null field condition")


class BooleanFilter(BaseModel):
    """Boolean logic filter (must, must_not, should)"""
    must: Optional[List[Dict[str, Any]]] = Field(default=None, description="AND conditions")
    must_not: Optional[List[Dict[str, Any]]] = Field(default=None, description="NOT conditions")
    should: Optional[List[Dict[str, Any]]] = Field(default=None, description="OR conditions")
    min_should_match: Optional[int] = Field(default=None, ge=1, description="Minimum should matches")
    
    @validator('must', 'must_not', 'should')
    def validate_conditions(cls, v):
        if v is not None and not v:
            raise ValueError("Boolean filter conditions cannot be empty lists")
        return v
    
    def __init__(self, **data):
        super().__init__(**data)
        if not any([self.must, self.must_not, self.should]):
            raise ValueError("Boolean filter must have at least one condition type")


class FilterRequest(BaseModel):
    """Complete filter request"""
    filter: Optional[Union[BooleanFilter, Dict[str, Any]]] = Field(default=None, description="Filter conditions")
    
    def get_filter_dict(self) -> Optional[Dict[str, Any]]:
        """Get filter as dictionary for processing"""
        if self.filter is None:
            return None
        if isinstance(self.filter, dict):
            return self.filter
        return self.filter.dict(exclude_none=True)


# Utility functions for filter validation
def validate_filter_structure(filter_dict: Dict[str, Any]) -> bool:
    """Validate filter structure is correct"""
    if not isinstance(filter_dict, dict):
        return False
    
    # Check for valid top-level keys
    valid_keys = {
        'must', 'must_not', 'should', 'min_should_match',
        'key', 'match', 'range', 'geo_radius', 'geo_bounding_box',
        'has_id', 'is_empty', 'is_null'
    }
    
    return all(key in valid_keys for key in filter_dict.keys())


def is_boolean_filter(filter_dict: Dict[str, Any]) -> bool:
    """Check if filter is a boolean filter"""
    boolean_keys = {'must', 'must_not', 'should'}
    return any(key in filter_dict for key in boolean_keys)


def is_field_filter(filter_dict: Dict[str, Any]) -> bool:
    """Check if filter is a field-based filter"""
    field_keys = {'match', 'range', 'geo_radius', 'geo_bounding_box', 'is_empty', 'is_null'}
    return any(key in filter_dict for key in field_keys)


def is_id_filter(filter_dict: Dict[str, Any]) -> bool:
    """Check if filter is an ID filter"""
    return 'has_id' in filter_dict