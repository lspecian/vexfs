"""
VexFS v2 Qdrant Adapter - gRPC Client Example

This example demonstrates how to use the gRPC interface for high-performance
vector operations with streaming support.
"""

import asyncio
import grpc
from grpc import aio
import numpy as np
from typing import List

# Import generated protobuf classes
import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '..', 'src'))

from proto import qdrant_pb2, qdrant_pb2_grpc


class VexFSQdrantGRPCClient:
    """
    High-performance gRPC client for VexFS v2 Qdrant adapter.
    
    This client demonstrates the streaming capabilities and performance
    optimizations available through the gRPC interface.
    """
    
    def __init__(self, host: str = "localhost", port: int = 6334):
        """
        Initialize the gRPC client.
        
        Args:
            host: gRPC server host
            port: gRPC server port
        """
        self.address = f"{host}:{port}"
        self.channel = None
        self.stub = None
    
    async def connect(self):
        """Connect to the gRPC server"""
        self.channel = aio.insecure_channel(self.address)
        self.stub = qdrant_pb2_grpc.QdrantStub(self.channel)
        print(f"Connected to VexFS Qdrant gRPC server at {self.address}")
    
    async def disconnect(self):
        """Disconnect from the gRPC server"""
        if self.channel:
            await self.channel.close()
            print("Disconnected from gRPC server")
    
    async def create_collection(self, name: str, dimensions: int, distance: str = "Cosine"):
        """Create a new vector collection"""
        print(f"Creating collection '{name}' with {dimensions} dimensions...")
        
        # Map distance string to enum
        distance_map = {
            "Cosine": qdrant_pb2.Distance.COSINE,
            "Euclidean": qdrant_pb2.Distance.EUCLIDEAN,
            "Dot": qdrant_pb2.Distance.DOT
        }
        
        vector_params = qdrant_pb2.VectorParams()
        vector_params.size = dimensions
        vector_params.distance = distance_map.get(distance, qdrant_pb2.Distance.COSINE)
        
        request = qdrant_pb2.CreateCollectionRequest()
        request.collection_name = name
        request.vectors.CopyFrom(vector_params)
        
        response = await self.stub.CreateCollection(request)
        
        if response.result:
            print(f"✅ Collection '{name}' created successfully in {response.time:.3f}s")
        else:
            print(f"❌ Failed to create collection '{name}'")
        
        return response.result
    
    async def upsert_points(self, collection_name: str, points_data: List[dict]):
        """Insert points into a collection"""
        print(f"Upserting {len(points_data)} points into '{collection_name}'...")
        
        points = []
        for point_data in points_data:
            point = qdrant_pb2.Point()
            point.id.num = point_data["id"]
            
            # Set vector
            vector = qdrant_pb2.Vector()
            vector.data.extend(point_data["vector"])
            
            vectors = qdrant_pb2.Vectors()
            vectors.vector.CopyFrom(vector)
            point.vectors.CopyFrom(vectors)
            
            # Set payload if provided
            if "payload" in point_data and point_data["payload"]:
                for key, value in point_data["payload"].items():
                    if isinstance(value, str):
                        point.payload.fields[key].string_value = value
                    elif isinstance(value, (int, float)):
                        point.payload.fields[key].number_value = float(value)
                    elif isinstance(value, bool):
                        point.payload.fields[key].bool_value = value
            
            points.append(point)
        
        request = qdrant_pb2.UpsertPointsRequest()
        request.collection_name = collection_name
        request.points.extend(points)
        
        response = await self.stub.UpsertPoints(request)
        
        print(f"✅ Upserted {response.result.operation_id} points in {response.time:.3f}s")
        return response.result.operation_id
    
    async def search_points(self, collection_name: str, query_vector: List[float], limit: int = 10):
        """Search for similar vectors"""
        print(f"Searching for {limit} similar vectors in '{collection_name}'...")
        
        request = qdrant_pb2.SearchPointsRequest()
        request.collection_name = collection_name
        request.vector.extend(query_vector)
        request.limit = limit
        
        response = await self.stub.SearchPoints(request)
        
        print(f"✅ Found {len(response.result)} results in {response.time:.3f}s")
        
        results = []
        for point in response.result:
            results.append({
                "id": point.id.num,
                "score": point.score
            })
        
        return results
    
    async def stream_search_points(self, collection_name: str, query_vector: List[float], limit: int = 100):
        """Stream search results for large result sets"""
        print(f"Streaming search for {limit} vectors in '{collection_name}'...")
        
        request = qdrant_pb2.SearchPointsRequest()
        request.collection_name = collection_name
        request.vector.extend(query_vector)
        request.limit = limit
        
        total_results = 0
        async for response in self.stub.StreamSearchPoints(request):
            batch_size = len(response.result)
            total_results += batch_size
            print(f"  Received batch of {batch_size} results (total: {total_results})")
        
        print(f"✅ Streaming search completed: {total_results} total results")
        return total_results
    
    async def stream_upsert_points(self, collection_name: str, all_points: List[dict], batch_size: int = 100):
        """Stream point insertion for large datasets"""
        print(f"Streaming upsert of {len(all_points)} points in batches of {batch_size}...")
        
        async def generate_requests():
            for i in range(0, len(all_points), batch_size):
                batch = all_points[i:i + batch_size]
                
                request = qdrant_pb2.UpsertPointsRequest()
                request.collection_name = collection_name
                
                for point_data in batch:
                    point = qdrant_pb2.Point()
                    point.id.num = point_data["id"]
                    
                    vector = qdrant_pb2.Vector()
                    vector.data.extend(point_data["vector"])
                    
                    vectors = qdrant_pb2.Vectors()
                    vectors.vector.CopyFrom(vector)
                    point.vectors.CopyFrom(vectors)
                    
                    request.points.append(point)
                
                print(f"  Sending batch of {len(batch)} points...")
                yield request
        
        response = await self.stub.StreamUpsertPoints(generate_requests())
        
        print(f"✅ Streaming upsert completed: {response.result.operation_id} points in {response.time:.3f}s")
        return response.result.operation_id
    
    async def get_collection_info(self, collection_name: str):
        """Get collection information"""
        request = qdrant_pb2.GetCollectionInfoRequest()
        request.collection_name = collection_name
        
        response = await self.stub.GetCollectionInfo(request)
        
        info = {
            "status": response.result.status,
            "vectors_count": response.result.vectors_count,
            "points_count": response.result.points_count,
            "segments_count": response.result.segments_count
        }
        
        return info


async def main():
    """Main example demonstrating gRPC client usage"""
    print("VexFS v2 Qdrant Adapter - gRPC Client Example")
    print("=" * 50)
    
    client = VexFSQdrantGRPCClient()
    
    try:
        # Connect to server
        await client.connect()
        
        # Create a test collection
        collection_name = "example_collection"
        dimensions = 128
        
        await client.create_collection(collection_name, dimensions, "Cosine")
        
        # Generate some test data
        print("\nGenerating test data...")
        test_points = []
        for i in range(1000):
            vector = np.random.random(dimensions).tolist()
            test_points.append({
                "id": i,
                "vector": vector,
                "payload": {
                    "category": f"category_{i % 5}",
                    "value": i * 0.1
                }
            })
        
        # Demonstrate regular upsert
        print("\n--- Regular Upsert ---")
        small_batch = test_points[:10]
        await client.upsert_points(collection_name, small_batch)
        
        # Demonstrate streaming upsert for large datasets
        print("\n--- Streaming Upsert ---")
        await client.stream_upsert_points(collection_name, test_points, batch_size=100)
        
        # Get collection info
        print("\n--- Collection Info ---")
        info = await client.get_collection_info(collection_name)
        print(f"Collection status: {info['status']}")
        print(f"Points count: {info['points_count']}")
        print(f"Vectors count: {info['vectors_count']}")
        
        # Demonstrate regular search
        print("\n--- Regular Search ---")
        query_vector = np.random.random(dimensions).tolist()
        results = await client.search_points(collection_name, query_vector, limit=5)
        
        print("Top 5 results:")
        for i, result in enumerate(results):
            print(f"  {i+1}. ID: {result['id']}, Score: {result['score']:.4f}")
        
        # Demonstrate streaming search
        print("\n--- Streaming Search ---")
        total_results = await client.stream_search_points(collection_name, query_vector, limit=50)
        
        print(f"\n🎉 Example completed successfully!")
        print(f"   - Created collection with {dimensions} dimensions")
        print(f"   - Inserted {len(test_points)} points via streaming")
        print(f"   - Performed search operations")
        print(f"   - Demonstrated streaming capabilities")
        
    except grpc.aio.AioRpcError as e:
        print(f"❌ gRPC Error: {e.code()} - {e.details()}")
    except Exception as e:
        print(f"❌ Error: {e}")
    finally:
        await client.disconnect()


if __name__ == "__main__":
    asyncio.run(main())