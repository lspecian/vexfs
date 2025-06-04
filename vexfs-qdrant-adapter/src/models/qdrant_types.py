"""
Qdrant Data Models

This module defines Pydantic models that match Qdrant's API specification.
These models ensure compatibility with existing Qdrant clients while providing
validation and serialization for VexFS v2 backend operations.
"""

from typing import Dict, List, Optional, Union, Any
from pydantic import BaseModel, Field, validator, RootModel
from enum import Enum
import uuid


class Distance(str, Enum):
    """Vector distance metrics supported by VexFS v2"""
    COSINE = "Cosine"
    EUCLIDEAN = "Euclidean"
    DOT = "Dot"


class VectorParams(BaseModel):
    """Vector configuration parameters"""
    size: int = Field(..., ge=1, le=65536, description="Vector dimensions")
    distance: Distance = Field(default=Distance.COSINE, description="Distance metric")


class HnswConfig(BaseModel):
    """HNSW index configuration (for Qdrant compatibility)"""
    m: int = Field(default=16, ge=4, le=64, description="Number of bi-directional links")
    ef_construct: int = Field(default=200, ge=4, le=1000, description="Construction parameter")
    full_scan_threshold: int = Field(default=10000, ge=1, description="Full scan threshold")
    max_indexing_threads: int = Field(default=0, ge=0, description="Max indexing threads")
    on_disk: bool = Field(default=False, description="Store index on disk")
    payload_m: Optional[int] = Field(default=None, ge=4, le=64, description="Payload links")


class OptimizerConfig(BaseModel):
    """Optimizer configuration (for Qdrant compatibility)"""
    deleted_threshold: float = Field(default=0.2, ge=0.0, le=1.0)
    vacuum_min_vector_number: int = Field(default=1000, ge=0)
    default_segment_number: int = Field(default=0, ge=0)
    max_segment_size: Optional[int] = Field(default=None, ge=1000)
    memmap_threshold: Optional[int] = Field(default=None, ge=1000)
    indexing_threshold: int = Field(default=20000, ge=0)
    flush_interval_sec: int = Field(default=5, ge=1)
    max_optimization_threads: int = Field(default=1, ge=1)


class VectorConfig(BaseModel):
    """Vector configuration for collections"""
    size: int = Field(..., ge=1, le=65536, description="Vector dimensions")
    distance: Distance = Field(default=Distance.COSINE, description="Distance metric")
    hnsw_config: Optional[HnswConfig] = Field(default=None, description="HNSW configuration")


class CollectionParams(BaseModel):
    """Collection parameters"""
    vectors: VectorConfig = Field(..., description="Vector configuration")
    shard_number: int = Field(default=1, ge=1, description="Number of shards")
    replication_factor: int = Field(default=1, ge=1, description="Replication factor")
    write_consistency_factor: int = Field(default=1, ge=1, description="Write consistency")
    on_disk_payload: bool = Field(default=True, description="Store payload on disk")


class CreateCollection(BaseModel):
    """Collection creation request"""
    vectors: Union[VectorConfig, Dict[str, VectorConfig]] = Field(..., description="Vector config")
    shard_number: Optional[int] = Field(default=1, ge=1, description="Number of shards")
    replication_factor: Optional[int] = Field(default=1, ge=1, description="Replication factor")
    write_consistency_factor: Optional[int] = Field(default=1, ge=1, description="Write consistency")
    on_disk_payload: Optional[bool] = Field(default=True, description="Store payload on disk")
    hnsw_config: Optional[HnswConfig] = Field(default=None, description="HNSW configuration")
    optimizer_config: Optional[OptimizerConfig] = Field(default=None, description="Optimizer config")
    wal_config: Optional[Dict[str, Any]] = Field(default=None, description="WAL configuration")
    quantization_config: Optional[Dict[str, Any]] = Field(default=None, description="Quantization config")
    init_from: Optional[Dict[str, Any]] = Field(default=None, description="Initialize from")
    timeout: Optional[int] = Field(default=None, ge=1, description="Operation timeout")


class PointId(RootModel[Union[int, str, uuid.UUID]]):
    """Point identifier"""
    root: Union[int, str, uuid.UUID]

    @validator('root')
    def validate_id(cls, v):
        if isinstance(v, str):
            try:
                # Try to parse as UUID
                return uuid.UUID(v)
            except ValueError:
                # Return as string if not UUID
                return v
        return v


class Vector(RootModel[List[float]]):
    """Vector data"""
    root: List[float]

    @validator('root')
    def validate_vector(cls, v):
        if not v:
            raise ValueError("Vector cannot be empty")
        if not all(isinstance(x, (int, float)) for x in v):
            raise ValueError("Vector must contain only numbers")
        return [float(x) for x in v]


class Payload(RootModel[Dict[str, Any]]):
    """Point payload data"""
    root: Dict[str, Any] = Field(default_factory=dict)


class PointStruct(BaseModel):
    """Point structure for upsert operations"""
    id: Union[int, str, uuid.UUID] = Field(..., description="Point ID")
    vector: Union[List[float], Dict[str, List[float]]] = Field(..., description="Vector data")
    payload: Optional[Dict[str, Any]] = Field(default=None, description="Point payload")

    @validator('vector')
    def validate_vector(cls, v):
        if isinstance(v, list):
            if not v:
                raise ValueError("Vector cannot be empty")
            return [float(x) for x in v]
        elif isinstance(v, dict):
            # Named vectors support
            for name, vec in v.items():
                if not vec:
                    raise ValueError(f"Vector '{name}' cannot be empty")
                v[name] = [float(x) for x in vec]
            return v
        else:
            raise ValueError("Vector must be a list of floats or dict of named vectors")


class PointsList(BaseModel):
    """List of points for batch operations"""
    points: List[PointStruct] = Field(..., description="List of points")


class SearchRequest(BaseModel):
    """Vector search request"""
    vector: List[float] = Field(..., description="Query vector")
    limit: int = Field(default=10, ge=1, le=10000, description="Maximum results")
    offset: int = Field(default=0, ge=0, description="Results offset")
    with_payload: Union[bool, List[str]] = Field(default=True, description="Include payload")
    with_vector: bool = Field(default=False, description="Include vector in results")
    score_threshold: Optional[float] = Field(default=None, description="Minimum score threshold")
    filter: Optional[Dict[str, Any]] = Field(default=None, description="Payload filter")
    params: Optional[Dict[str, Any]] = Field(default=None, description="Search parameters")

    @validator('vector')
    def validate_vector(cls, v):
        if not v:
            raise ValueError("Query vector cannot be empty")
        return [float(x) for x in v]


class ScoredPoint(BaseModel):
    """Search result point with score"""
    id: Union[int, str, uuid.UUID] = Field(..., description="Point ID")
    version: int = Field(default=0, description="Point version")
    score: float = Field(..., description="Similarity score")
    payload: Optional[Dict[str, Any]] = Field(default=None, description="Point payload")
    vector: Optional[Union[List[float], Dict[str, List[float]]]] = Field(default=None, description="Point vector")


class SearchResult(BaseModel):
    """Search operation result"""
    result: List[ScoredPoint] = Field(default_factory=list, description="Search results")


class PointIdsList(BaseModel):
    """List of point IDs"""
    ids: List[Union[int, str, uuid.UUID]] = Field(..., description="Point IDs")


class DeletePoints(BaseModel):
    """Delete points request"""
    ids: Optional[List[Union[int, str, uuid.UUID]]] = Field(default=None, description="Point IDs to delete")
    filter: Optional[Dict[str, Any]] = Field(default=None, description="Filter for deletion")

    @validator('ids', 'filter')
    def validate_delete_criteria(cls, v, values):
        ids = values.get('ids') if 'ids' in values else v
        filter_val = values.get('filter') if 'filter' in values else (v if 'filter' in cls.__fields__ else None)
        
        if not ids and not filter_val:
            raise ValueError("Either 'ids' or 'filter' must be provided")
        return v


class CollectionInfo(BaseModel):
    """Collection information response"""
    status: str = Field(..., description="Collection status")
    optimizer_status: str = Field(default="ok", description="Optimizer status")
    vectors_count: int = Field(default=0, description="Number of vectors")
    indexed_vectors_count: int = Field(default=0, description="Number of indexed vectors")
    points_count: int = Field(default=0, description="Number of points")
    segments_count: int = Field(default=1, description="Number of segments")
    config: Dict[str, Any] = Field(..., description="Collection configuration")
    payload_schema: Dict[str, Any] = Field(default_factory=dict, description="Payload schema")


class CollectionDescription(BaseModel):
    """Collection description for listing"""
    name: str = Field(..., description="Collection name")


class CollectionsResponse(BaseModel):
    """Collections listing response"""
    collections: List[CollectionDescription] = Field(default_factory=list, description="Collections list")


class UpdateResult(BaseModel):
    """Update operation result"""
    operation_id: int = Field(..., description="Operation ID")
    status: str = Field(default="completed", description="Operation status")


class ClusterInfo(BaseModel):
    """Cluster information"""
    peer_id: int = Field(default=1, description="Peer ID")
    peers_count: int = Field(default=1, description="Number of peers")
    raft_info: Dict[str, Any] = Field(default_factory=dict, description="Raft information")


class HealthCheck(BaseModel):
    """Health check response"""
    title: str = Field(default="qdrant - vector search engine", description="Service title")
    version: str = Field(default="1.7.0", description="Service version")


class PointRequest(BaseModel):
    """Single point request"""
    ids: List[Union[int, str, uuid.UUID]] = Field(..., description="Point IDs")
    with_payload: Union[bool, List[str]] = Field(default=True, description="Include payload")
    with_vector: bool = Field(default=False, description="Include vector")


class Record(BaseModel):
    """Point record"""
    id: Union[int, str, uuid.UUID] = Field(..., description="Point ID")
    payload: Optional[Dict[str, Any]] = Field(default=None, description="Point payload")
    vector: Optional[Union[List[float], Dict[str, List[float]]]] = Field(default=None, description="Point vector")


class GetResult(BaseModel):
    """Get points result"""
    result: List[Record] = Field(default_factory=list, description="Retrieved points")


# Response wrapper models for API consistency
class ApiResponse(BaseModel):
    """Generic API response wrapper"""
    result: Any = Field(..., description="Response data")
    status: str = Field(default="ok", description="Response status")
    time: float = Field(default=0.0, description="Processing time")


class ErrorResponse(BaseModel):
    """Error response"""
    error: str = Field(..., description="Error message")
    status: str = Field(default="error", description="Response status")