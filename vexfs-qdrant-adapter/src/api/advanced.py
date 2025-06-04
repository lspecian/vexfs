"""
Advanced API Endpoints for Phase 3

This module implements advanced Qdrant-compatible endpoints including
filter DSL, recommendations, scroll API, and batch operations.
"""

from typing import Dict, List, Any, Optional, Union
from fastapi import APIRouter, HTTPException, Depends, status, Query, Body
from fastapi.responses import JSONResponse
import time
import asyncio

from ..core.vexfs_client import VexFSClient, VexFSError
from ..filters.filter_engine import FilterEngine, FilterEngineError
from ..recommendations.recommend_engine import RecommendationEngine
from ..scroll.scroll_api import ScrollAPI
from ..batch.batch_operations import BatchOperations
from ..models.responses import (
    create_success_response,
    create_error_response
)
from ..utils.logging import get_logger, vexfs_operation_monitor
from ..utils.config import get_config

logger = get_logger(__name__)
router = APIRouter(prefix="/collections", tags=["advanced"])

# Global instances
_vexfs_client: Optional[VexFSClient] = None
_filter_engine: Optional[FilterEngine] = None
_recommendation_engine: Optional[RecommendationEngine] = None
_scroll_api: Optional[ScrollAPI] = None
_batch_operations: Optional[BatchOperations] = None


def get_vexfs_client() -> VexFSClient:
    """Get or create VexFS client instance"""
    global _vexfs_client
    if _vexfs_client is None:
        config = get_config()
        try:
            _vexfs_client = VexFSClient(config.vexfs.device_path)
            logger.info("VexFS client initialized for advanced operations")
        except VexFSError as e:
            logger.error("Failed to initialize VexFS client", error=str(e))
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail=f"VexFS service unavailable: {e}"
            )
    return _vexfs_client


def get_filter_engine() -> FilterEngine:
    """Get or create filter engine instance"""
    global _filter_engine
    if _filter_engine is None:
        _filter_engine = FilterEngine(get_vexfs_client())
    return _filter_engine


def get_recommendation_engine() -> RecommendationEngine:
    """Get or create recommendation engine instance"""
    global _recommendation_engine
    if _recommendation_engine is None:
        _recommendation_engine = RecommendationEngine(get_vexfs_client())
    return _recommendation_engine


def get_scroll_api() -> ScrollAPI:
    """Get or create scroll API instance"""
    global _scroll_api
    if _scroll_api is None:
        _scroll_api = ScrollAPI(get_vexfs_client())
    return _scroll_api


def get_batch_operations() -> BatchOperations:
    """Get or create batch operations instance"""
    global _batch_operations
    if _batch_operations is None:
        _batch_operations = BatchOperations(get_vexfs_client())
    return _batch_operations


# Filter DSL Endpoints
@router.post("/{collection_name}/points/search/filter")
@vexfs_operation_monitor("advanced_search_with_filter")
async def search_with_advanced_filter(
    collection_name: str,
    search_request: Dict[str, Any] = Body(...),
    filter_engine: FilterEngine = Depends(get_filter_engine)
) -> JSONResponse:
    """
    Advanced search with complete Qdrant filter DSL support.
    
    Supports all filter types: must, must_not, should, match, range, geo, etc.
    """
    start_time = time.time()
    
    try:
        # Extract search parameters
        query_vector = search_request.get('vector')
        if not query_vector:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Query vector is required"
            )
        
        limit = search_request.get('limit', 10)
        filter_condition = search_request.get('filter')
        with_payload = search_request.get('with_payload', True)
        with_vector = search_request.get('with_vector', False)
        
        # Perform vector search
        vexfs_client = get_vexfs_client()
        search_results = vexfs_client.search_vectors(
            collection=collection_name,
            query_vector=query_vector,
            limit=limit * 3,  # Get more results for filtering
            distance="Cosine"
        )
        
        # Apply advanced filters
        if filter_condition:
            candidate_ids = [str(result.vector_id) for result in search_results]
            filtered_ids = filter_engine.apply_filter(
                collection_name, filter_condition, candidate_ids
            )
            
            # Keep only filtered results in original order
            search_results = [
                result for result in search_results
                if str(result.vector_id) in filtered_ids
            ][:limit]
        else:
            search_results = search_results[:limit]
        
        # Format results
        formatted_results = []
        for result in search_results:
            point = {
                'id': str(result.vector_id),
                'score': float(result.score)
            }
            
            if with_payload:
                point['payload'] = {}  # Would be populated from VexFS metadata
            
            if with_vector:
                point['vector'] = None  # Would be populated from VexFS
            
            formatted_results.append(point)
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Advanced filtered search completed",
            collection=collection_name,
            results_count=len(formatted_results),
            has_filter=filter_condition is not None,
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response({
                'result': formatted_results,
                'filter_applied': filter_condition is not None
            }, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except FilterEngineError as e:
        processing_time = time.time() - start_time
        logger.error("Filter processing failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response(f"Filter error: {e}"),
            status_code=status.HTTP_400_BAD_REQUEST
        )
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error("Advanced search failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response("Advanced search failed"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


# Recommendation Endpoints
@router.post("/{collection_name}/points/recommend")
@vexfs_operation_monitor("recommend_points")
async def recommend_points(
    collection_name: str,
    recommendation_request: Dict[str, Any] = Body(...),
    recommendation_engine: RecommendationEngine = Depends(get_recommendation_engine)
) -> JSONResponse:
    """
    Generate recommendations based on positive/negative examples.
    
    Supports multiple strategies: average_vector, best_score, centroid, diversity.
    """
    start_time = time.time()
    
    try:
        # Extract recommendation parameters
        positive = recommendation_request.get('positive', [])
        negative = recommendation_request.get('negative', [])
        strategy = recommendation_request.get('strategy', 'average_vector')
        limit = recommendation_request.get('limit', 10)
        filter_condition = recommendation_request.get('filter')
        with_payload = recommendation_request.get('with_payload', True)
        with_vector = recommendation_request.get('with_vector', False)
        
        # Generate recommendations
        recommendations = recommendation_engine.recommend_points(
            collection=collection_name,
            positive=positive,
            negative=negative,
            strategy=strategy,
            limit=limit,
            filter_condition=filter_condition,
            with_payload=with_payload,
            with_vector=with_vector
        )
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Recommendations generated",
            collection=collection_name,
            positive_count=len(positive),
            negative_count=len(negative),
            strategy=strategy,
            recommendations_count=len(recommendations),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response({
                'result': recommendations,
                'strategy': strategy
            }, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error("Recommendation failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response(f"Recommendation failed: {e}"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.post("/{collection_name}/points/discover")
@vexfs_operation_monitor("discover_points")
async def discover_points(
    collection_name: str,
    discovery_request: Dict[str, Any] = Body(...),
    recommendation_engine: RecommendationEngine = Depends(get_recommendation_engine)
) -> JSONResponse:
    """
    Discover similar points for exploration using multi-hop discovery.
    """
    start_time = time.time()
    
    try:
        target = discovery_request.get('target')
        if not target:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Target point ID is required"
            )
        
        limit = discovery_request.get('limit', 10)
        filter_condition = discovery_request.get('filter')
        exploration_depth = discovery_request.get('exploration_depth', 2)
        diversity_factor = discovery_request.get('diversity_factor', 0.3)
        
        # Discover points
        discovered_points = recommendation_engine.discover_points(
            collection=collection_name,
            target=target,
            limit=limit,
            filter_condition=filter_condition,
            exploration_depth=exploration_depth,
            diversity_factor=diversity_factor
        )
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Point discovery completed",
            collection=collection_name,
            target=target,
            discovered_count=len(discovered_points),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response({
                'result': discovered_points,
                'target': target
            }, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error("Discovery failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response(f"Discovery failed: {e}"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


# Scroll API Endpoints
@router.post("/{collection_name}/points/scroll")
@vexfs_operation_monitor("scroll_points")
async def scroll_points(
    collection_name: str,
    scroll_request: Dict[str, Any] = Body(...),
    scroll_api: ScrollAPI = Depends(get_scroll_api)
) -> JSONResponse:
    """
    Scroll through collection with cursor-based pagination.
    
    Supports efficient pagination for large collections with memory management.
    """
    start_time = time.time()
    
    try:
        limit = scroll_request.get('limit', 100)
        offset = scroll_request.get('offset')
        filter_condition = scroll_request.get('filter')
        with_payload = scroll_request.get('with_payload', True)
        with_vectors = scroll_request.get('with_vectors', False)
        
        # Perform scroll operation
        scroll_result = scroll_api.scroll_points(
            collection=collection_name,
            limit=limit,
            offset=offset,
            filter_condition=filter_condition,
            with_payload=with_payload,
            with_vectors=with_vectors
        )
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Scroll operation completed",
            collection=collection_name,
            points_returned=len(scroll_result.get('points', [])),
            has_more=scroll_result.get('has_more', False),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response(scroll_result, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error("Scroll operation failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response(f"Scroll failed: {e}"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


# Batch Operations Endpoints
@router.post("/{collection_name}/points/search/batch")
@vexfs_operation_monitor("batch_search")
async def batch_search(
    collection_name: str,
    batch_request: Dict[str, Any] = Body(...),
    batch_operations: BatchOperations = Depends(get_batch_operations)
) -> JSONResponse:
    """
    Execute multiple search queries in parallel for maximum throughput.
    """
    start_time = time.time()
    
    try:
        queries = batch_request.get('searches', [])
        if not queries:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Search queries are required"
            )
        
        # Execute batch search
        results = await batch_operations.batch_search(collection_name, queries)
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Batch search completed",
            collection=collection_name,
            queries_count=len(queries),
            results_count=len(results),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response({
                'result': results,
                'queries_processed': len(queries)
            }, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error("Batch search failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response(f"Batch search failed: {e}"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.post("/{collection_name}/points/search/groups")
@vexfs_operation_monitor("grouped_search")
async def grouped_search(
    collection_name: str,
    group_request: Dict[str, Any] = Body(...),
    batch_operations: BatchOperations = Depends(get_batch_operations)
) -> JSONResponse:
    """
    Search with result grouping by field for diverse results.
    """
    start_time = time.time()
    
    try:
        query_vector = group_request.get('vector')
        if not query_vector:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Query vector is required"
            )
        
        group_by = group_request.get('group_by')
        if not group_by:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="group_by field is required"
            )
        
        limit = group_request.get('limit', 10)
        group_size = group_request.get('group_size', 3)
        filter_condition = group_request.get('filter')
        
        # Execute grouped search
        result = await batch_operations.grouped_search(
            collection=collection_name,
            query_vector=query_vector,
            group_by=group_by,
            limit=limit,
            group_size=group_size,
            filter_condition=filter_condition
        )
        
        processing_time = time.time() - start_time
        
        logger.info(
            "Grouped search completed",
            collection=collection_name,
            groups_found=result.get('total_groups', 0),
            processing_time=processing_time
        )
        
        return JSONResponse(
            content=create_success_response(result, processing_time),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        processing_time = time.time() - start_time
        logger.error("Grouped search failed", error=str(e), processing_time=processing_time)
        return JSONResponse(
            content=create_error_response(f"Grouped search failed: {e}"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


# Performance and Statistics Endpoints
@router.get("/advanced/statistics")
async def get_advanced_statistics(
    filter_engine: FilterEngine = Depends(get_filter_engine),
    recommendation_engine: RecommendationEngine = Depends(get_recommendation_engine),
    scroll_api: ScrollAPI = Depends(get_scroll_api),
    batch_operations: BatchOperations = Depends(get_batch_operations)
) -> JSONResponse:
    """
    Get performance statistics for all advanced features.
    """
    try:
        stats = {
            'filter_engine': filter_engine.get_filter_statistics(),
            'recommendation_engine': recommendation_engine.get_recommendation_statistics(),
            'scroll_api': scroll_api.get_scroll_statistics(),
            'batch_operations': batch_operations.get_batch_statistics(),
            'phase3_features': {
                'filter_dsl': 'active',
                'recommendations': 'active',
                'scroll_api': 'active',
                'batch_operations': 'active'
            }
        }
        
        return JSONResponse(
            content=create_success_response(stats),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        logger.error("Failed to get advanced statistics", error=str(e))
        return JSONResponse(
            content=create_error_response("Failed to get statistics"),
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@router.post("/advanced/validate-filter")
async def validate_filter(
    filter_request: Dict[str, Any] = Body(...),
    filter_engine: FilterEngine = Depends(get_filter_engine)
) -> JSONResponse:
    """
    Validate a filter condition without executing it.
    """
    try:
        filter_condition = filter_request.get('filter')
        if not filter_condition:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Filter condition is required"
            )
        
        validation_result = filter_engine.validate_filter(filter_condition)
        
        return JSONResponse(
            content=create_success_response(validation_result),
            status_code=status.HTTP_200_OK
        )
        
    except Exception as e:
        logger.error("Filter validation failed", error=str(e))
        return JSONResponse(
            content=create_error_response(f"Filter validation failed: {e}"),
            status_code=status.HTTP_400_BAD_REQUEST
        )