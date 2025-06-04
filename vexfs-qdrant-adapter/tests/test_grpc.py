"""
gRPC Tests for VexFS v2 Qdrant Adapter

This module tests the gRPC implementation including streaming operations
and performance characteristics.
"""

import asyncio
import pytest
import grpc
from grpc import aio
import numpy as np
from typing import List, AsyncIterator

from src.proto import qdrant_pb2, qdrant_pb2_grpc
from src.grpc_server.qdrant_service import QdrantServicer
from src.core.vexfs_client import VexFSClient
from src.utils.config import get_config


class MockVexFSClient:
    """Mock VexFS client for testing"""
    
    def __init__(self):
        self.collections = {}
        self.points = {}
        
    def create_collection(self, name: str, dimensions: int, distance: str = "Cosine"):
        self.collections[name] = {
            "dimensions": dimensions,
            "distance": distance,
            "vector_count": 0
        }
        return {
            "name": name,
            "config": {
                "params": {
                    "vectors": {
                        "size": dimensions,
                        "distance": distance
                    }
                }
            },
            "status": "green"
        }
    
    def get_collection_info(self, name: str):
        if name not in self.collections:
            raise Exception(f"Collection '{name}' not found")
        
        collection = self.collections[name]
        return {
            "status": "green",
            "optimizer_status": "ok",
            "vectors_count": collection["vector_count"],
            "indexed_vectors_count": collection["vector_count"],
            "points_count": collection["vector_count"],
            "segments_count": 1,
            "config": {
                "params": {
                    "vectors": {
                        "size": collection["dimensions"],
                        "distance": collection["distance"]
                    }
                }
            }
        }
    
    def list_collections(self):
        return {"collections": self.collections}
    
    def delete_collection(self, name: str):
        if name not in self.collections:
            raise Exception(f"Collection '{name}' not found")
        del self.collections[name]
        return True
    
    def insert_points(self, collection: str, points: List[dict]):
        if collection not in self.collections:
            raise Exception(f"Collection '{collection}' not found")
        
        if collection not in self.points:
            self.points[collection] = {}
        
        for point in points:
            self.points[collection][point["id"]] = point
        
        self.collections[collection]["vector_count"] += len(points)
        
        return {
            "operation_id": len(points),
            "status": "completed"
        }
    
    def search_vectors(self, collection: str, query_vector: List[float], 
                      limit: int = 10, distance: str = "Cosine"):
        if collection not in self.collections:
            raise Exception(f"Collection '{collection}' not found")
        
        # Mock search results
        results = []
        collection_points = self.points.get(collection, {})
        
        for i, (point_id, point_data) in enumerate(collection_points.items()):
            if i >= limit:
                break
            
            # Mock similarity score
            score = 0.9 - (i * 0.1)
            results.append(type('SearchResult', (), {
                'vector_id': point_id,
                'score': score,
                'payload': point_data.get('payload', {})
            })())
        
        return results
    
    def get_vector_metadata(self, collection: str, point_ids: List[int]):
        if collection not in self.collections:
            raise Exception(f"Collection '{collection}' not found")
        
        results = []
        collection_points = self.points.get(collection, {})
        
        for point_id in point_ids:
            if point_id in collection_points:
                results.append(collection_points[point_id])
            else:
                results.append({
                    "id": point_id,
                    "vector": None,
                    "payload": {}
                })
        
        return results


@pytest.fixture
async def grpc_servicer():
    """Create a gRPC servicer with mock VexFS client"""
    mock_client = MockVexFSClient()
    servicer = QdrantServicer(mock_client)
    return servicer


@pytest.fixture
async def grpc_server(grpc_servicer):
    """Create a test gRPC server"""
    server = aio.server()
    qdrant_pb2_grpc.add_QdrantServicer_to_server(grpc_servicer, server)
    
    listen_addr = '[::]:0'  # Use any available port
    port = server.add_insecure_port(listen_addr)
    
    await server.start()
    
    yield f'localhost:{port}'
    
    await server.stop(grace=1.0)


@pytest.fixture
async def grpc_channel(grpc_server):
    """Create a gRPC channel for testing"""
    channel = aio.insecure_channel(grpc_server)
    yield channel
    await channel.close()


@pytest.fixture
async def grpc_stub(grpc_channel):
    """Create a gRPC stub for testing"""
    return qdrant_pb2_grpc.QdrantStub(grpc_channel)


class TestGRPCCollectionOperations:
    """Test gRPC collection operations"""
    
    @pytest.mark.asyncio
    async def test_create_collection(self, grpc_stub):
        """Test collection creation via gRPC"""
        # Create vector configuration
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 128
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        # Create collection request
        request = qdrant_pb2.CreateCollectionRequest()
        request.collection_name = "test_collection"
        request.vectors.CopyFrom(vector_params)
        
        # Call gRPC method
        response = await grpc_stub.CreateCollection(request)
        
        # Verify response
        assert response.result is True
        assert response.time > 0
    
    @pytest.mark.asyncio
    async def test_get_collection_info(self, grpc_stub):
        """Test getting collection info via gRPC"""
        # First create a collection
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 128
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "test_collection"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Get collection info
        info_request = qdrant_pb2.GetCollectionInfoRequest()
        info_request.collection_name = "test_collection"
        
        response = await grpc_stub.GetCollectionInfo(info_request)
        
        # Verify response
        assert response.result.status == qdrant_pb2.CollectionInfo.GREEN
        assert response.result.vectors_count == 0
        assert response.time > 0
    
    @pytest.mark.asyncio
    async def test_list_collections(self, grpc_stub):
        """Test listing collections via gRPC"""
        # Create a test collection first
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 128
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "test_collection"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # List collections
        list_request = qdrant_pb2.ListCollectionsRequest()
        response = await grpc_stub.ListCollections(list_request)
        
        # Verify response
        assert len(response.collections) >= 1
        assert any(c.name == "test_collection" for c in response.collections)
        assert response.time > 0


class TestGRPCPointOperations:
    """Test gRPC point operations"""
    
    @pytest.mark.asyncio
    async def test_upsert_points(self, grpc_stub):
        """Test upserting points via gRPC"""
        # Create collection first
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 4
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "test_collection"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Create points
        points = []
        for i in range(3):
            point = qdrant_pb2.Point()
            point.id.num = i
            
            vector = qdrant_pb2.Vector()
            vector.data.extend([0.1 * i, 0.2 * i, 0.3 * i, 0.4 * i])
            
            vectors = qdrant_pb2.Vectors()
            vectors.vector.CopyFrom(vector)
            point.vectors.CopyFrom(vectors)
            
            points.append(point)
        
        # Upsert points
        upsert_request = qdrant_pb2.UpsertPointsRequest()
        upsert_request.collection_name = "test_collection"
        upsert_request.points.extend(points)
        
        response = await grpc_stub.UpsertPoints(upsert_request)
        
        # Verify response
        assert response.result.status == qdrant_pb2.UpdateResult.COMPLETED
        assert response.result.operation_id == 3
        assert response.time > 0
    
    @pytest.mark.asyncio
    async def test_search_points(self, grpc_stub):
        """Test searching points via gRPC"""
        # Create collection and add points
        await self._setup_collection_with_points(grpc_stub)
        
        # Search points
        search_request = qdrant_pb2.SearchPointsRequest()
        search_request.collection_name = "test_collection"
        search_request.vector.extend([0.1, 0.2, 0.3, 0.4])
        search_request.limit = 2
        
        response = await grpc_stub.SearchPoints(search_request)
        
        # Verify response
        assert len(response.result) <= 2
        assert response.time > 0
        
        # Check that results are scored points
        for point in response.result:
            assert point.HasField('id')
            assert point.score >= 0.0
    
    @pytest.mark.asyncio
    async def test_get_points(self, grpc_stub):
        """Test getting points by ID via gRPC"""
        # Create collection and add points
        await self._setup_collection_with_points(grpc_stub)
        
        # Get points
        point_ids = [qdrant_pb2.PointId(), qdrant_pb2.PointId()]
        point_ids[0].num = 0
        point_ids[1].num = 1
        
        get_request = qdrant_pb2.GetPointsRequest()
        get_request.collection_name = "test_collection"
        get_request.ids.extend(point_ids)
        
        response = await grpc_stub.GetPoints(get_request)
        
        # Verify response
        assert len(response.result) == 2
        assert response.time > 0
        
        # Check that we got the right points
        retrieved_ids = [point.id.num for point in response.result]
        assert 0 in retrieved_ids
        assert 1 in retrieved_ids
    
    async def _setup_collection_with_points(self, grpc_stub):
        """Helper method to create collection and add test points"""
        # Create collection
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 4
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "test_collection"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Add points
        points = []
        for i in range(5):
            point = qdrant_pb2.Point()
            point.id.num = i
            
            vector = qdrant_pb2.Vector()
            vector.data.extend([0.1 * i, 0.2 * i, 0.3 * i, 0.4 * i])
            
            vectors = qdrant_pb2.Vectors()
            vectors.vector.CopyFrom(vector)
            point.vectors.CopyFrom(vectors)
            
            points.append(point)
        
        upsert_request = qdrant_pb2.UpsertPointsRequest()
        upsert_request.collection_name = "test_collection"
        upsert_request.points.extend(points)
        
        await grpc_stub.UpsertPoints(upsert_request)


class TestGRPCStreamingOperations:
    """Test gRPC streaming operations"""
    
    @pytest.mark.asyncio
    async def test_stream_search_points(self, grpc_stub):
        """Test streaming search results"""
        # Create collection and add points
        await self._setup_large_collection(grpc_stub)
        
        # Stream search results
        search_request = qdrant_pb2.SearchPointsRequest()
        search_request.collection_name = "large_collection"
        search_request.vector.extend([0.1, 0.2, 0.3, 0.4])
        search_request.limit = 50  # Request more than batch size
        
        total_results = 0
        async for response in grpc_stub.StreamSearchPoints(search_request):
            assert len(response.result) > 0
            assert response.time > 0
            total_results += len(response.result)
            
            # Verify each result
            for point in response.result:
                assert point.HasField('id')
                assert point.score >= 0.0
        
        assert total_results > 0
    
    @pytest.mark.asyncio
    async def test_stream_upsert_points(self, grpc_stub):
        """Test streaming point insertion"""
        # Create collection
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 4
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "stream_collection"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Create streaming upsert requests
        async def generate_upsert_requests():
            for batch in range(3):  # 3 batches
                request = qdrant_pb2.UpsertPointsRequest()
                request.collection_name = "stream_collection"
                
                # Add 10 points per batch
                for i in range(10):
                    point_id = batch * 10 + i
                    point = qdrant_pb2.Point()
                    point.id.num = point_id
                    
                    vector = qdrant_pb2.Vector()
                    vector.data.extend([0.1 * point_id, 0.2 * point_id, 0.3 * point_id, 0.4 * point_id])
                    
                    vectors = qdrant_pb2.Vectors()
                    vectors.vector.CopyFrom(vector)
                    point.vectors.CopyFrom(vectors)
                    
                    request.points.append(point)
                
                yield request
        
        # Stream upsert
        response = await grpc_stub.StreamUpsertPoints(generate_upsert_requests())
        
        # Verify response
        assert response.result.status == qdrant_pb2.UpdateResult.COMPLETED
        assert response.result.operation_id == 30  # 3 batches * 10 points
        assert response.time > 0
    
    async def _setup_large_collection(self, grpc_stub):
        """Helper method to create a large collection for streaming tests"""
        # Create collection
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 4
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "large_collection"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Add many points
        points = []
        for i in range(100):  # 100 points
            point = qdrant_pb2.Point()
            point.id.num = i
            
            vector = qdrant_pb2.Vector()
            vector.data.extend([0.1 * i, 0.2 * i, 0.3 * i, 0.4 * i])
            
            vectors = qdrant_pb2.Vectors()
            vectors.vector.CopyFrom(vector)
            point.vectors.CopyFrom(vectors)
            
            points.append(point)
        
        upsert_request = qdrant_pb2.UpsertPointsRequest()
        upsert_request.collection_name = "large_collection"
        upsert_request.points.extend(points)
        
        await grpc_stub.UpsertPoints(upsert_request)


class TestGRPCPerformance:
    """Test gRPC performance characteristics"""
    
    @pytest.mark.asyncio
    async def test_concurrent_requests(self, grpc_stub):
        """Test handling concurrent gRPC requests"""
        # Create collection
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 4
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "concurrent_test"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Add some points first
        points = []
        for i in range(10):
            point = qdrant_pb2.Point()
            point.id.num = i
            
            vector = qdrant_pb2.Vector()
            vector.data.extend([0.1 * i, 0.2 * i, 0.3 * i, 0.4 * i])
            
            vectors = qdrant_pb2.Vectors()
            vectors.vector.CopyFrom(vector)
            point.vectors.CopyFrom(vectors)
            
            points.append(point)
        
        upsert_request = qdrant_pb2.UpsertPointsRequest()
        upsert_request.collection_name = "concurrent_test"
        upsert_request.points.extend(points)
        
        await grpc_stub.UpsertPoints(upsert_request)
        
        # Create concurrent search requests
        async def search_task():
            search_request = qdrant_pb2.SearchPointsRequest()
            search_request.collection_name = "concurrent_test"
            search_request.vector.extend([0.1, 0.2, 0.3, 0.4])
            search_request.limit = 5
            
            response = await grpc_stub.SearchPoints(search_request)
            return len(response.result)
        
        # Run 10 concurrent searches
        tasks = [search_task() for _ in range(10)]
        results = await asyncio.gather(*tasks)
        
        # Verify all requests completed successfully
        assert len(results) == 10
        assert all(result > 0 for result in results)
    
    @pytest.mark.asyncio
    async def test_large_batch_performance(self, grpc_stub):
        """Test performance with large batches"""
        # Create collection
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = 128  # Larger vectors
        vector_params.distance = qdrant_pb2.Distance.COSINE
        
        create_request = qdrant_pb2.CreateCollectionRequest()
        create_request.collection_name = "performance_test"
        create_request.vectors.CopyFrom(vector_params)
        
        await grpc_stub.CreateCollection(create_request)
        
        # Create large batch of points
        points = []
        for i in range(1000):  # 1000 points
            point = qdrant_pb2.Point()
            point.id.num = i
            
            vector = qdrant_pb2.Vector()
            # Generate random-like vector data
            vector_data = [np.sin(i * 0.01 + j * 0.1) for j in range(128)]
            vector.data.extend(vector_data)
            
            vectors = qdrant_pb2.Vectors()
            vectors.vector.CopyFrom(vector)
            point.vectors.CopyFrom(vectors)
            
            points.append(point)
        
        # Measure upsert performance
        import time
        start_time = time.time()
        
        upsert_request = qdrant_pb2.UpsertPointsRequest()
        upsert_request.collection_name = "performance_test"
        upsert_request.points.extend(points)
        
        response = await grpc_stub.UpsertPoints(upsert_request)
        
        end_time = time.time()
        processing_time = end_time - start_time
        
        # Verify performance
        assert response.result.status == qdrant_pb2.UpdateResult.COMPLETED
        assert response.result.operation_id == 1000
        
        # Calculate ops/sec (should be high with VexFS backend)
        ops_per_sec = 1000 / processing_time
        print(f"gRPC Upsert Performance: {ops_per_sec:.2f} ops/sec")
        
        # With mock client, this should be very fast
        assert ops_per_sec > 1000  # At least 1000 ops/sec


if __name__ == "__main__":
    pytest.main([__file__, "-v"])