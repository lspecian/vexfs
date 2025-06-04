"""
Filter DSL Parser

This module parses Qdrant filter DSL into structured filter objects
that can be efficiently executed against VexFS metadata operations.
"""

from typing import Dict, List, Any, Optional, Set, Union
import logging
from .filter_types import (
    FilterCondition,
    MatchFilter,
    RangeFilter,
    GeoFilter,
    HasIdFilter,
    IsEmptyFilter,
    IsNullFilter,
    BooleanFilter,
    validate_filter_structure,
    is_boolean_filter,
    is_field_filter,
    is_id_filter
)

logger = logging.getLogger(__name__)


class FilterParseError(Exception):
    """Filter parsing error"""
    pass


class FilterParser:
    """
    Qdrant Filter DSL Parser
    
    Parses complex filter expressions into structured objects that can be
    efficiently executed using VexFS metadata operations.
    """
    
    def __init__(self):
        self.supported_operators = {
            'match', 'range', 'geo_radius', 'geo_bounding_box',
            'has_id', 'is_empty', 'is_null',
            'must', 'must_not', 'should'
        }
    
    def parse_filter(self, filter_dict: Optional[Dict[str, Any]]) -> Optional[Dict[str, Any]]:
        """
        Parse a filter dictionary into a structured filter object.
        
        Args:
            filter_dict: Raw filter dictionary from request
            
        Returns:
            Parsed and validated filter structure
            
        Raises:
            FilterParseError: If filter is invalid
        """
        if filter_dict is None:
            return None
        
        if not isinstance(filter_dict, dict):
            raise FilterParseError("Filter must be a dictionary")
        
        if not filter_dict:
            return None
        
        # Validate basic structure
        if not validate_filter_structure(filter_dict):
            unknown_keys = set(filter_dict.keys()) - self.supported_operators - {'key', 'min_should_match'}
            if unknown_keys:
                raise FilterParseError(f"Unsupported filter operators: {unknown_keys}")
        
        try:
            return self._parse_filter_recursive(filter_dict)
        except Exception as e:
            logger.error(f"Filter parsing failed: {e}")
            raise FilterParseError(f"Invalid filter structure: {e}")
    
    def _parse_filter_recursive(self, filter_dict: Dict[str, Any]) -> Dict[str, Any]:
        """Recursively parse filter structure"""
        
        # Handle boolean filters (must, must_not, should)
        if is_boolean_filter(filter_dict):
            return self._parse_boolean_filter(filter_dict)
        
        # Handle ID filters
        if is_id_filter(filter_dict):
            return self._parse_id_filter(filter_dict)
        
        # Handle field-based filters
        if is_field_filter(filter_dict):
            return self._parse_field_filter(filter_dict)
        
        raise FilterParseError(f"Unknown filter type: {filter_dict}")
    
    def _parse_boolean_filter(self, filter_dict: Dict[str, Any]) -> Dict[str, Any]:
        """Parse boolean logic filters"""
        parsed = {
            'type': 'boolean',
            'conditions': {}
        }
        
        # Parse must conditions (AND logic)
        if 'must' in filter_dict:
            must_conditions = filter_dict['must']
            if not isinstance(must_conditions, list):
                raise FilterParseError("'must' must be a list of conditions")
            
            parsed['conditions']['must'] = [
                self._parse_filter_recursive(condition)
                for condition in must_conditions
            ]
        
        # Parse must_not conditions (NOT logic)
        if 'must_not' in filter_dict:
            must_not_conditions = filter_dict['must_not']
            if not isinstance(must_not_conditions, list):
                raise FilterParseError("'must_not' must be a list of conditions")
            
            parsed['conditions']['must_not'] = [
                self._parse_filter_recursive(condition)
                for condition in must_not_conditions
            ]
        
        # Parse should conditions (OR logic)
        if 'should' in filter_dict:
            should_conditions = filter_dict['should']
            if not isinstance(should_conditions, list):
                raise FilterParseError("'should' must be a list of conditions")
            
            parsed['conditions']['should'] = [
                self._parse_filter_recursive(condition)
                for condition in should_conditions
            ]
            
            # Handle min_should_match
            if 'min_should_match' in filter_dict:
                min_match = filter_dict['min_should_match']
                if not isinstance(min_match, int) or min_match < 1:
                    raise FilterParseError("'min_should_match' must be a positive integer")
                parsed['conditions']['min_should_match'] = min_match
        
        return parsed
    
    def _parse_id_filter(self, filter_dict: Dict[str, Any]) -> Dict[str, Any]:
        """Parse ID-based filters"""
        if 'has_id' not in filter_dict:
            raise FilterParseError("ID filter must have 'has_id' field")
        
        ids = filter_dict['has_id']
        if not isinstance(ids, list) or not ids:
            raise FilterParseError("'has_id' must be a non-empty list")
        
        # Normalize IDs to strings for consistent handling
        normalized_ids = []
        for id_val in ids:
            if isinstance(id_val, (int, str)):
                normalized_ids.append(str(id_val))
            else:
                normalized_ids.append(str(id_val))
        
        return {
            'type': 'has_id',
            'ids': normalized_ids
        }
    
    def _parse_field_filter(self, filter_dict: Dict[str, Any]) -> Dict[str, Any]:
        """Parse field-based filters"""
        
        # Get the field key
        field_key = filter_dict.get('key')
        if not field_key:
            raise FilterParseError("Field filter must have 'key' field")
        
        # Determine filter type and parse accordingly
        if 'match' in filter_dict:
            return self._parse_match_filter(field_key, filter_dict['match'])
        
        elif 'range' in filter_dict:
            return self._parse_range_filter(field_key, filter_dict['range'])
        
        elif 'geo_radius' in filter_dict:
            return self._parse_geo_radius_filter(field_key, filter_dict['geo_radius'])
        
        elif 'geo_bounding_box' in filter_dict:
            return self._parse_geo_bounding_box_filter(field_key, filter_dict['geo_bounding_box'])
        
        elif 'is_empty' in filter_dict:
            return self._parse_is_empty_filter(field_key)
        
        elif 'is_null' in filter_dict:
            return self._parse_is_null_filter(field_key)
        
        else:
            raise FilterParseError(f"Unknown field filter type for key '{field_key}'")
    
    def _parse_match_filter(self, field_key: str, match_condition: Dict[str, Any]) -> Dict[str, Any]:
        """Parse match filter"""
        if 'value' not in match_condition:
            raise FilterParseError("Match filter must have 'value' field")
        
        return {
            'type': 'match',
            'field': field_key,
            'value': match_condition['value'],
            'any': match_condition.get('any', False)  # Support for any-of matching
        }
    
    def _parse_range_filter(self, field_key: str, range_condition: Dict[str, Any]) -> Dict[str, Any]:
        """Parse range filter"""
        valid_ops = {'gte', 'gt', 'lte', 'lt'}
        range_ops = {k: v for k, v in range_condition.items() if k in valid_ops}
        
        if not range_ops:
            raise FilterParseError(f"Range filter must have at least one of: {valid_ops}")
        
        # Validate numeric values
        for op, value in range_ops.items():
            if not isinstance(value, (int, float)):
                raise FilterParseError(f"Range filter value for '{op}' must be numeric")
        
        return {
            'type': 'range',
            'field': field_key,
            'conditions': range_ops
        }
    
    def _parse_geo_radius_filter(self, field_key: str, geo_condition: Dict[str, Any]) -> Dict[str, Any]:
        """Parse geo radius filter"""
        if 'center' not in geo_condition or 'radius' not in geo_condition:
            raise FilterParseError("Geo radius filter must have 'center' and 'radius'")
        
        center = geo_condition['center']
        if not isinstance(center, (list, tuple)) or len(center) != 2:
            raise FilterParseError("Geo center must be [lat, lon] array")
        
        lat, lon = center
        if not isinstance(lat, (int, float)) or not isinstance(lon, (int, float)):
            raise FilterParseError("Geo coordinates must be numeric")
        
        if not (-90 <= lat <= 90):
            raise FilterParseError("Latitude must be between -90 and 90")
        
        if not (-180 <= lon <= 180):
            raise FilterParseError("Longitude must be between -180 and 180")
        
        radius = geo_condition['radius']
        if not isinstance(radius, (int, float)) or radius <= 0:
            raise FilterParseError("Geo radius must be a positive number")
        
        return {
            'type': 'geo_radius',
            'field': field_key,
            'center': [float(lat), float(lon)],
            'radius': float(radius)
        }
    
    def _parse_geo_bounding_box_filter(self, field_key: str, geo_condition: Dict[str, Any]) -> Dict[str, Any]:
        """Parse geo bounding box filter"""
        if 'top_left' not in geo_condition or 'bottom_right' not in geo_condition:
            raise FilterParseError("Geo bounding box must have 'top_left' and 'bottom_right'")
        
        top_left = geo_condition['top_left']
        bottom_right = geo_condition['bottom_right']
        
        for point_name, point in [('top_left', top_left), ('bottom_right', bottom_right)]:
            if not isinstance(point, (list, tuple)) or len(point) != 2:
                raise FilterParseError(f"Geo {point_name} must be [lat, lon] array")
            
            lat, lon = point
            if not isinstance(lat, (int, float)) or not isinstance(lon, (int, float)):
                raise FilterParseError(f"Geo {point_name} coordinates must be numeric")
        
        return {
            'type': 'geo_bounding_box',
            'field': field_key,
            'top_left': [float(top_left[0]), float(top_left[1])],
            'bottom_right': [float(bottom_right[0]), float(bottom_right[1])]
        }
    
    def _parse_is_empty_filter(self, field_key: str) -> Dict[str, Any]:
        """Parse is_empty filter"""
        return {
            'type': 'is_empty',
            'field': field_key
        }
    
    def _parse_is_null_filter(self, field_key: str) -> Dict[str, Any]:
        """Parse is_null filter"""
        return {
            'type': 'is_null',
            'field': field_key
        }
    
    def get_filter_fields(self, parsed_filter: Dict[str, Any]) -> Set[str]:
        """
        Extract all field names referenced in a filter.
        
        Args:
            parsed_filter: Parsed filter structure
            
        Returns:
            Set of field names used in the filter
        """
        fields = set()
        
        def extract_fields(filter_obj):
            if isinstance(filter_obj, dict):
                if filter_obj.get('type') == 'boolean':
                    conditions = filter_obj.get('conditions', {})
                    for condition_list in conditions.values():
                        if isinstance(condition_list, list):
                            for condition in condition_list:
                                extract_fields(condition)
                elif 'field' in filter_obj:
                    fields.add(filter_obj['field'])
        
        extract_fields(parsed_filter)
        return fields
    
    def estimate_filter_complexity(self, parsed_filter: Dict[str, Any]) -> int:
        """
        Estimate the computational complexity of a filter.
        
        Args:
            parsed_filter: Parsed filter structure
            
        Returns:
            Complexity score (higher = more complex)
        """
        if not parsed_filter:
            return 0
        
        complexity = 0
        
        def calculate_complexity(filter_obj):
            nonlocal complexity
            
            if isinstance(filter_obj, dict):
                filter_type = filter_obj.get('type')
                
                if filter_type == 'boolean':
                    # Boolean filters add complexity based on number of conditions
                    conditions = filter_obj.get('conditions', {})
                    for condition_type, condition_list in conditions.items():
                        if isinstance(condition_list, list):
                            complexity += len(condition_list)
                            for condition in condition_list:
                                calculate_complexity(condition)
                
                elif filter_type in ['match', 'has_id']:
                    complexity += 1  # Simple filters
                
                elif filter_type == 'range':
                    complexity += 2  # Range filters are more expensive
                
                elif filter_type in ['geo_radius', 'geo_bounding_box']:
                    complexity += 5  # Geo filters are expensive
                
                elif filter_type in ['is_empty', 'is_null']:
                    complexity += 1  # Simple existence checks
        
        calculate_complexity(parsed_filter)
        return complexity