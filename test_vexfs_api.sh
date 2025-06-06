#!/bin/bash

echo "=== VexFS API Test Script ==="
echo "Testing VexFS Docker container..."

# Wait for container to be ready
echo "Waiting for VexFS to start..."
sleep 5

# Test 1: Get collections (should return empty array initially)
echo "1. Testing GET /api/v1/collections"
curl -s http://localhost:8080/api/v1/collections || echo "Failed to connect"
echo ""

# Test 2: Create a collection
echo "2. Creating a test collection"
curl -s -X POST http://localhost:8080/api/v1/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test_collection",
    "metadata": {"description": "Test collection for demo"},
    "vector_size": 128,
    "distance": "cosine"
  }' || echo "Failed to create collection"
echo ""

# Test 3: List collections again
echo "3. Listing collections after creation"
curl -s http://localhost:8080/api/v1/collections || echo "Failed to get collections"
echo ""

# Test 4: Add some vectors
echo "4. Adding test vectors"
curl -s -X POST http://localhost:8080/api/v1/collections/test_collection/vectors \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": [
      {
        "id": "vec1",
        "vector": [0.1, 0.2, 0.3, 0.4],
        "metadata": {"type": "test"}
      },
      {
        "id": "vec2", 
        "vector": [0.5, 0.6, 0.7, 0.8],
        "metadata": {"type": "demo"}
      }
    ]
  }' || echo "Failed to add vectors"
echo ""

# Test 5: Search vectors
echo "5. Searching for similar vectors"
curl -s -X POST http://localhost:8080/api/v1/collections/test_collection/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.1, 0.2, 0.3, 0.4],
    "limit": 5
  }' || echo "Failed to search vectors"
echo ""

echo "=== VexFS API Test Complete ==="