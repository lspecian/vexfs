"""
Streaming Operations for VexFS v2 Qdrant Adapter

This module provides efficient streaming support for large-scale vector operations,
optimized for memory usage and high throughput.
"""

import asyncio
import time
from typing import AsyncIterator, List, Dict, Any, Optional, Tuple
import grpc
from grpc import aio
from google.protobuf import struct_pb2

from ..proto import qdrant_pb2
from ..core.vexfs_client import VexFSClient, VexFSError
from ..utils.logging import get_logger

logger = get_logger(__name__)


class StreamingOperations:
    """
    Efficient streaming support for large vector operations.
    
    This class provides memory-efficient streaming for:
    - Large search result sets
    - Bulk point insertion
    - Batch point retrieval
    """
    
    def __init__(self, vexfs_client: VexFSClient):
        """
        Initialize streaming operations.
        
        Args:
            vexfs_client: VexFS v2 client instance
        """
        self.vexfs_client = vexfs_client
        self.logger = logger
        
        # Streaming configuration
        self.default_batch_size = 100
        self.max_batch_size = 1000
        self.memory_limit_mb = 100  # 100MB memory limit for streaming
    
    def _create_point_id(self, value: int) -> qdrant_pb2.PointId:
        """Create protobuf PointId from integer"""
        point_id = qdrant_pb2.PointId()
        point_id.num = value
        return point_id
    
    async def stream_search_results(
        self, 
        collection_name: str,
        query_vector: List[float],
        total_limit: int,
        batch_size: Optional[int] = None,
        distance: str = "Cosine"
    ) -> AsyncIterator[List[qdrant_pb2.ScoredPoint]]:
        """
        Stream search results for large result sets with pagination.
        
        Args:
            collection_name: Collection to search in
            query_vector: Query vector
            total_limit: Total number of results to return
            batch_size: Size of each batch (default: 100)
            distance: Distance metric
            
        Yields:
            Batches of scored points
        """
        if batch_size is None:
            batch_size = min(self.default_batch_size, total_limit)
        
        batch_size = min(batch_size, self.max_batch_size)
        offset = 0
        
        self.logger.info(
            "Starting streaming search",
            collection=collection_name,
            total_limit=total_limit,
            batch_size=batch_size,
            query_dimensions=len(query_vector)
        )
        
        try:
            while offset < total_limit:
                current_limit = min(batch_size, total_limit - offset)
                
                # Perform vector search for this batch
                results = self.vexfs_client.search_vectors(
                    collection=collection_name,
                    query_vector=query_vector,
                    limit=current_limit,
                    distance=distance
                )
                
                if not results:
                    break
                
                # Convert results to protobuf format
                scored_points = []
                for result in results:
                    scored_point = qdrant_pb2.ScoredPoint()
                    scored_point.id.CopyFrom(self._create_point_id(result.vector_id))
                    scored_point.score = result.score
                    scored_point.payload.CopyFrom(struct_pb2.Struct())
                    scored_points.append(scored_point)
                
                # Yield this batch
                yield scored_points
                
                offset += len(results)
                
                # If we got fewer results than requested, we're done
                if len(results) < current_limit:
                    break
                
                # Small delay to prevent overwhelming the system
                await asyncio.sleep(0.001)  # 1ms delay
            
            self.logger.info(
                "Streaming search completed",
                collection=collection_name,
                total_results=offset
            )
            
        except Exception as e:
            self.logger.error(
                "Streaming search failed",
                collection=collection_name,
                error=str(e)
            )
            raise
    
    async def stream_upsert_points(
        self, 
        request_iterator: AsyncIterator[qdrant_pb2.UpsertPointsRequest]
    ) -> Tuple[int, str]:
        """
        Stream point insertion for bulk operations.
        
        Args:
            request_iterator: Async iterator of upsert requests
            
        Returns:
            Tuple of (total_points_inserted, collection_name)
        """
        total_points = 0
        collection_name = None
        batch_buffer = []
        
        self.logger.info("Starting streaming upsert")
        
        try:
            async for request in request_iterator:
                if collection_name is None:
                    collection_name = request.collection_name
                
                # Convert protobuf points to VexFS format
                for point in request.points:
                    point_data = {
                        "id": self._convert_point_id(point.id),
                        "payload": {}
                    }
                    
                    # Handle vectors
                    if point.vectors.HasField('vector'):
                        point_data["vector"] = list(point.vectors.vector.data)
                    elif point.vectors.HasField('named_vectors'):
                        # Use the first named vector
                        first_vector_name = next(iter(point.vectors.named_vectors.vectors.keys()))
                        point_data["vector"] = list(point.vectors.named_vectors.vectors[first_vector_name].data)
                    
                    # Handle payload
                    if point.HasField('payload'):
                        payload_dict = {}
                        for key, value in point.payload.fields.items():
                            if value.HasField('string_value'):
                                payload_dict[key] = value.string_value
                            elif value.HasField('number_value'):
                                payload_dict[key] = value.number_value
                            elif value.HasField('bool_value'):
                                payload_dict[key] = value.bool_value
                        point_data["payload"] = payload_dict
                    
                    batch_buffer.append(point_data)
                
                # Insert batch when buffer reaches optimal size
                if len(batch_buffer) >= self.default_batch_size:
                    self.vexfs_client.insert_points(collection_name, batch_buffer)
                    total_points += len(batch_buffer)
                    
                    self.logger.debug(
                        "Streaming upsert batch processed",
                        batch_size=len(batch_buffer),
                        total_points=total_points
                    )
                    
                    batch_buffer = []
                    
                    # Small delay to prevent overwhelming the system
                    await asyncio.sleep(0.001)
            
            # Insert remaining points in buffer
            if batch_buffer:
                self.vexfs_client.insert_points(collection_name, batch_buffer)
                total_points += len(batch_buffer)
            
            self.logger.info(
                "Streaming upsert completed",
                collection=collection_name,
                total_points=total_points
            )
            
            return total_points, collection_name
            
        except Exception as e:
            self.logger.error(
                "Streaming upsert failed",
                collection=collection_name,
                total_points=total_points,
                error=str(e)
            )
            raise
    
    def _convert_point_id(self, point_id) -> int:
        """Convert protobuf PointId to integer"""
        if point_id.HasField('num'):
            return int(point_id.num)
        elif point_id.HasField('uuid'):
            # Convert UUID string to hash
            return hash(point_id.uuid) & 0x7FFFFFFFFFFFFFFF
        else:
            return 0
    
    async def stream_get_points(
        self, 
        collection_name: str,
        point_ids: List[int],
        batch_size: Optional[int] = None,
        with_payload: bool = True,
        with_vectors: bool = False
    ) -> AsyncIterator[List[qdrant_pb2.RetrievedPoint]]:
        """
        Stream point retrieval for large ID lists.
        
        Args:
            collection_name: Collection name
            point_ids: List of point IDs to retrieve
            batch_size: Size of each batch
            with_payload: Include payload in results
            with_vectors: Include vectors in results
            
        Yields:
            Batches of retrieved points
        """
        if batch_size is None:
            batch_size = self.default_batch_size
        
        batch_size = min(batch_size, self.max_batch_size)
        
        self.logger.info(
            "Starting streaming get points",
            collection=collection_name,
            total_ids=len(point_ids),
            batch_size=batch_size
        )
        
        try:
            for i in range(0, len(point_ids), batch_size):
                batch_ids = point_ids[i:i + batch_size]
                
                # Get points metadata for this batch
                points = self.vexfs_client.get_vector_metadata(collection_name, batch_ids)
                
                # Convert to protobuf format
                retrieved_points = []
                for point in points:
                    retrieved_point = qdrant_pb2.RetrievedPoint()
                    retrieved_point.id.CopyFrom(self._create_point_id(point["id"]))
                    
                    if with_payload:
                        # Add payload if available
                        payload = struct_pb2.Struct()
                        if point.get("payload"):
                            for key, value in point["payload"].items():
                                if isinstance(value, str):
                                    payload.fields[key].string_value = value
                                elif isinstance(value, (int, float)):
                                    payload.fields[key].number_value = float(value)
                                elif isinstance(value, bool):
                                    payload.fields[key].bool_value = value
                        retrieved_point.payload.CopyFrom(payload)
                    
                    if with_vectors and point.get("vector"):
                        # Add vector if available and requested
                        vector = qdrant_pb2.Vector()
                        vector.data.extend(point["vector"])
                        vectors = qdrant_pb2.Vectors()
                        vectors.vector.CopyFrom(vector)
                        retrieved_point.vectors.CopyFrom(vectors)
                    
                    retrieved_points.append(retrieved_point)
                
                # Yield this batch
                yield retrieved_points
                
                # Small delay to prevent overwhelming the system
                await asyncio.sleep(0.001)
            
            self.logger.info(
                "Streaming get points completed",
                collection=collection_name,
                total_retrieved=len(point_ids)
            )
            
        except Exception as e:
            self.logger.error(
                "Streaming get points failed",
                collection=collection_name,
                error=str(e)
            )
            raise
    
    async def stream_scroll_points(
        self,
        collection_name: str,
        batch_size: Optional[int] = None,
        filter_conditions: Optional[Dict[str, Any]] = None,
        with_payload: bool = True,
        with_vectors: bool = False
    ) -> AsyncIterator[List[qdrant_pb2.RetrievedPoint]]:
        """
        Stream scroll through all points in a collection.
        
        Args:
            collection_name: Collection name
            batch_size: Size of each batch
            filter_conditions: Optional filter conditions
            with_payload: Include payload in results
            with_vectors: Include vectors in results
            
        Yields:
            Batches of retrieved points
        """
        if batch_size is None:
            batch_size = self.default_batch_size
        
        batch_size = min(batch_size, self.max_batch_size)
        
        self.logger.info(
            "Starting streaming scroll",
            collection=collection_name,
            batch_size=batch_size,
            with_payload=with_payload,
            with_vectors=with_vectors
        )
        
        try:
            # Get collection info to determine total points
            collection_info = self.vexfs_client.get_collection_info(collection_name)
            total_points = collection_info.get("points_count", 0)
            
            if total_points == 0:
                self.logger.info("No points to scroll", collection=collection_name)
                return
            
            # For now, simulate scrolling by getting points in batches
            # In a full implementation, this would use cursor-based pagination
            offset = 0
            
            while offset < total_points:
                current_batch_size = min(batch_size, total_points - offset)
                
                # Generate point IDs for this batch (simplified approach)
                batch_ids = list(range(offset, offset + current_batch_size))
                
                # Get points for this batch
                points = self.vexfs_client.get_vector_metadata(collection_name, batch_ids)
                
                if not points:
                    break
                
                # Convert to protobuf format
                retrieved_points = []
                for point in points:
                    retrieved_point = qdrant_pb2.RetrievedPoint()
                    retrieved_point.id.CopyFrom(self._create_point_id(point["id"]))
                    
                    if with_payload:
                        payload = struct_pb2.Struct()
                        if point.get("payload"):
                            for key, value in point["payload"].items():
                                if isinstance(value, str):
                                    payload.fields[key].string_value = value
                                elif isinstance(value, (int, float)):
                                    payload.fields[key].number_value = float(value)
                                elif isinstance(value, bool):
                                    payload.fields[key].bool_value = value
                        retrieved_point.payload.CopyFrom(payload)
                    
                    retrieved_points.append(retrieved_point)
                
                # Yield this batch
                yield retrieved_points
                
                offset += len(points)
                
                # Small delay to prevent overwhelming the system
                await asyncio.sleep(0.001)
            
            self.logger.info(
                "Streaming scroll completed",
                collection=collection_name,
                total_scrolled=offset
            )
            
        except Exception as e:
            self.logger.error(
                "Streaming scroll failed",
                collection=collection_name,
                error=str(e)
            )
            raise
    
    def get_memory_usage_mb(self) -> float:
        """Get current memory usage in MB (simplified)"""
        import psutil
        import os
        
        process = psutil.Process(os.getpid())
        return process.memory_info().rss / 1024 / 1024
    
    def should_yield_control(self) -> bool:
        """Check if we should yield control to prevent blocking"""
        return self.get_memory_usage_mb() > self.memory_limit_mb