"""
API Response Models

This module defines response models for the VexFS Qdrant adapter API.
These models ensure consistent response formatting and proper serialization.
"""

from typing import Any, Dict, List, Optional, Union
from pydantic import BaseModel, Field
from .qdrant_types import (
    CollectionInfo,
    CollectionsResponse,
    SearchResult,
    UpdateResult,
    GetResult,
    ClusterInfo,
    HealthCheck,
    ScoredPoint,
    Record
)


class BaseResponse(BaseModel):
    """Base response model with common fields"""
    status: str = Field(default="ok", description="Response status")
    time: float = Field(default=0.0, description="Processing time in seconds")


class CollectionResponse(BaseResponse):
    """Collection operation response"""
    result: Optional[CollectionInfo] = Field(default=None, description="Collection information")


class CollectionsListResponse(BaseResponse):
    """Collections listing response"""
    result: CollectionsResponse = Field(..., description="Collections list")


class SearchResponse(BaseResponse):
    """Search operation response"""
    result: List[ScoredPoint] = Field(default_factory=list, description="Search results")


class UpsertResponse(BaseResponse):
    """Upsert operation response"""
    result: UpdateResult = Field(..., description="Upsert result")


class DeleteResponse(BaseResponse):
    """Delete operation response"""
    result: UpdateResult = Field(..., description="Delete result")


class GetPointsResponse(BaseResponse):
    """Get points response"""
    result: List[Record] = Field(default_factory=list, description="Retrieved points")


class ClusterResponse(BaseResponse):
    """Cluster information response"""
    result: ClusterInfo = Field(..., description="Cluster information")


class HealthResponse(BaseResponse):
    """Health check response"""
    result: HealthCheck = Field(..., description="Health information")


class ErrorResponse(BaseModel):
    """Error response model"""
    detail: str = Field(..., description="Error message")
    status: str = Field(default="error", description="Error status")


class VexFSStatsResponse(BaseResponse):
    """VexFS performance statistics response"""
    result: Dict[str, Any] = Field(..., description="VexFS statistics")


# Utility functions for response creation
def create_success_response(result: Any, processing_time: float = 0.0) -> Dict[str, Any]:
    """Create a successful response"""
    return {
        "result": result,
        "status": "ok",
        "time": processing_time
    }


def create_error_response(error_message: str, status_code: int = 400) -> Dict[str, Any]:
    """Create an error response"""
    return {
        "detail": error_message,
        "status": "error"
    }


def create_collection_info_response(
    name: str,
    dimensions: int,
    distance: str,
    vector_count: int = 0
) -> Dict[str, Any]:
    """Create a collection info response"""
    return create_success_response({
        "status": "green",
        "optimizer_status": "ok",
        "vectors_count": vector_count,
        "indexed_vectors_count": vector_count,
        "points_count": vector_count,
        "segments_count": 1,
        "config": {
            "params": {
                "vectors": {
                    "size": dimensions,
                    "distance": distance
                }
            }
        },
        "payload_schema": {}
    })


def create_search_response(results: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Create a search results response"""
    scored_points = []
    for result in results:
        scored_points.append({
            "id": result["vector_id"],
            "version": 0,
            "score": result["score"],
            "payload": result.get("payload", {}),
            "vector": result.get("vector")
        })
    
    return create_success_response(scored_points)


def create_upsert_response(operation_id: int, status: str = "completed") -> Dict[str, Any]:
    """Create an upsert operation response"""
    return create_success_response({
        "operation_id": operation_id,
        "status": status
    })


def create_delete_response(operation_id: int, status: str = "completed") -> Dict[str, Any]:
    """Create a delete operation response"""
    return create_success_response({
        "operation_id": operation_id,
        "status": status
    })


def create_collections_list_response(collections: Dict[str, Any]) -> Dict[str, Any]:
    """Create a collections list response"""
    collection_list = []
    for name, info in collections.get("collections", {}).items():
        collection_list.append({"name": name})
    
    return create_success_response({
        "collections": collection_list
    })


def create_cluster_info_response() -> Dict[str, Any]:
    """Create cluster information response"""
    return create_success_response({
        "peer_id": 1,
        "peers_count": 1,
        "raft_info": {
            "term": 1,
            "commit": 1,
            "pending_operations": 0,
            "leader": 1,
            "role": "Leader"
        }
    })


def create_health_response() -> Dict[str, Any]:
    """Create health check response"""
    return {
        "title": "qdrant - vector search engine",
        "version": "1.7.0-vexfs"
    }


def create_get_points_response(points: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Create get points response"""
    records = []
    for point in points:
        records.append({
            "id": point["id"],
            "payload": point.get("payload", {}),
            "vector": point.get("vector")
        })
    
    return create_success_response(records)