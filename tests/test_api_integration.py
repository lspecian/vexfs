#!/usr/bin/env python3
"""
VexFS API Integration Tests

Comprehensive testing of all three API dialects (ChromaDB, Qdrant, Native)
"""

import unittest
import requests
import json
import time
import numpy as np
from typing import List, Dict, Any
import subprocess
import os
import signal

class VexFSAPIIntegrationTest(unittest.TestCase):
    """Integration tests for VexFS API server"""
    
    @classmethod
    def setUpClass(cls):
        """Start the API server before tests"""
        cls.base_url = "http://localhost:7680"
        cls.api_key = "vexfs-default-key"
        
        # Start API server
        cls.server_process = subprocess.Popen(
            ["./rust/target/release/vexfs_unified_server"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            preexec_fn=os.setsid
        )
        
        # Wait for server to start
        time.sleep(5)
        
        # Get authentication token
        cls.token = cls._get_auth_token()
        
    @classmethod
    def tearDownClass(cls):
        """Stop the API server after tests"""
        if cls.server_process:
            os.killpg(os.getpgid(cls.server_process.pid), signal.SIGTERM)
            cls.server_process.wait()
    
    @classmethod
    def _get_auth_token(cls) -> str:
        """Get authentication token"""
        response = requests.post(
            f"{cls.base_url}/auth/login",
            json={"api_key": cls.api_key}
        )
        if response.status_code == 200:
            return response.json().get("token", "")
        return ""
    
    def setUp(self):
        """Setup for each test"""
        self.test_collection = f"test_collection_{int(time.time())}"
        self.headers = {
            "Authorization": self.token,
            "Content-Type": "application/json"
        }
    
    def tearDown(self):
        """Cleanup after each test"""
        # Try to delete test collection
        try:
            requests.delete(
                f"{self.base_url}/api/v1/collections/{self.test_collection}",
                headers=self.headers
            )
        except:
            pass
    
    def generate_vector(self, dim: int = 384) -> List[float]:
        """Generate a random normalized vector"""
        vec = np.random.randn(dim)
        vec = vec / np.linalg.norm(vec)
        return vec.tolist()
    
    # ============================================
    # ChromaDB API Tests
    # ============================================
    
    def test_chromadb_create_collection(self):
        """Test creating a collection via ChromaDB API"""
        response = requests.post(
            f"{self.base_url}/api/v1/collections",
            headers=self.headers,
            json={
                "name": self.test_collection,
                "metadata": {"dimension": 384}
            }
        )
        self.assertIn(response.status_code, [200, 201])
    
    def test_chromadb_add_documents(self):
        """Test adding documents via ChromaDB API"""
        # Create collection first
        self.test_chromadb_create_collection()
        
        # Add documents
        response = requests.post(
            f"{self.base_url}/api/v1/collections/{self.test_collection}/add",
            headers=self.headers,
            json={
                "ids": ["doc1", "doc2", "doc3"],
                "documents": [
                    "First document about vectors",
                    "Second document about search",
                    "Third document about embeddings"
                ],
                "embeddings": [
                    self.generate_vector(384),
                    self.generate_vector(384),
                    self.generate_vector(384)
                ],
                "metadatas": [
                    {"type": "tutorial"},
                    {"type": "reference"},
                    {"type": "guide"}
                ]
            }
        )
        self.assertEqual(response.status_code, 200)
    
    def test_chromadb_query(self):
        """Test querying via ChromaDB API"""
        # Setup collection with data
        self.test_chromadb_add_documents()
        
        # Query
        query_vector = self.generate_vector(384)
        response = requests.post(
            f"{self.base_url}/api/v1/collections/{self.test_collection}/query",
            headers=self.headers,
            json={
                "query_embeddings": [query_vector],
                "n_results": 2
            }
        )
        
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("ids", data)
        self.assertIn("distances", data)
    
    def test_chromadb_list_collections(self):
        """Test listing collections via ChromaDB API"""
        # Create a collection
        self.test_chromadb_create_collection()
        
        # List collections
        response = requests.get(
            f"{self.base_url}/api/v1/collections",
            headers=self.headers
        )
        
        self.assertEqual(response.status_code, 200)
        collections = response.json()
        self.assertIsInstance(collections, list)
    
    # ============================================
    # Qdrant API Tests
    # ============================================
    
    def test_qdrant_create_collection(self):
        """Test creating a collection via Qdrant API"""
        response = requests.put(
            f"{self.base_url}/collections/{self.test_collection}",
            headers=self.headers,
            json={
                "vectors": {
                    "size": 384,
                    "distance": "Cosine"
                }
            }
        )
        self.assertIn(response.status_code, [200, 201])
    
    def test_qdrant_upsert_points(self):
        """Test upserting points via Qdrant API"""
        # Create collection first
        self.test_qdrant_create_collection()
        
        # Upsert points
        points = []
        for i in range(5):
            points.append({
                "id": i,
                "vector": self.generate_vector(384),
                "payload": {
                    "text": f"Document {i}",
                    "category": f"cat_{i % 2}"
                }
            })
        
        response = requests.put(
            f"{self.base_url}/collections/{self.test_collection}/points",
            headers=self.headers,
            json={"points": points}
        )
        
        self.assertEqual(response.status_code, 200)
    
    def test_qdrant_search(self):
        """Test searching via Qdrant API"""
        # Setup collection with data
        self.test_qdrant_upsert_points()
        
        # Search
        response = requests.post(
            f"{self.base_url}/collections/{self.test_collection}/points/search",
            headers=self.headers,
            json={
                "vector": self.generate_vector(384),
                "limit": 3,
                "with_payload": True
            }
        )
        
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("result", data)
    
    # ============================================
    # Native VexFS API Tests
    # ============================================
    
    def test_native_create_collection(self):
        """Test creating a collection via Native API"""
        response = requests.post(
            f"{self.base_url}/vexfs/v1/collections",
            headers=self.headers,
            json={
                "name": self.test_collection,
                "dimension": 384,
                "metric": "cosine"
            }
        )
        self.assertIn(response.status_code, [200, 201])
    
    def test_native_add_documents(self):
        """Test adding documents via Native API"""
        # Create collection first
        self.test_native_create_collection()
        
        # Add documents
        documents = []
        for i in range(10):
            documents.append({
                "id": f"doc_{i}",
                "vector": self.generate_vector(384),
                "metadata": {
                    "text": f"Document {i}",
                    "timestamp": time.time()
                }
            })
        
        response = requests.post(
            f"{self.base_url}/vexfs/v1/collections/{self.test_collection}/documents",
            headers=self.headers,
            json={"documents": documents}
        )
        
        self.assertEqual(response.status_code, 200)
    
    def test_native_search(self):
        """Test searching via Native API"""
        # Setup collection with data
        self.test_native_add_documents()
        
        # Search
        response = requests.post(
            f"{self.base_url}/vexfs/v1/collections/{self.test_collection}/search",
            headers=self.headers,
            json={
                "vector": self.generate_vector(384),
                "k": 5,
                "include_metadata": True
            }
        )
        
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("results", data)
    
    # ============================================
    # Cross-API Compatibility Tests
    # ============================================
    
    def test_cross_api_compatibility(self):
        """Test that data created with one API can be accessed via another"""
        collection_name = f"cross_api_{int(time.time())}"
        
        # Create collection via ChromaDB API
        response = requests.post(
            f"{self.base_url}/api/v1/collections",
            headers=self.headers,
            json={
                "name": collection_name,
                "metadata": {"dimension": 128}
            }
        )
        self.assertIn(response.status_code, [200, 201])
        
        # Add data via Native API
        response = requests.post(
            f"{self.base_url}/vexfs/v1/collections/{collection_name}/documents",
            headers=self.headers,
            json={
                "documents": [{
                    "id": "cross_doc",
                    "vector": self.generate_vector(128),
                    "metadata": {"source": "native"}
                }]
            }
        )
        self.assertEqual(response.status_code, 200)
        
        # Query via ChromaDB API
        response = requests.post(
            f"{self.base_url}/api/v1/collections/{collection_name}/query",
            headers=self.headers,
            json={
                "query_embeddings": [self.generate_vector(128)],
                "n_results": 1
            }
        )
        self.assertEqual(response.status_code, 200)
        
        # Cleanup
        requests.delete(
            f"{self.base_url}/api/v1/collections/{collection_name}",
            headers=self.headers
        )
    
    # ============================================
    # Performance Tests
    # ============================================
    
    def test_bulk_insert_performance(self):
        """Test performance of bulk inserts"""
        # Create collection
        self.test_chromadb_create_collection()
        
        # Prepare bulk data
        num_docs = 1000
        ids = [f"doc_{i}" for i in range(num_docs)]
        documents = [f"Document {i}" for i in range(num_docs)]
        embeddings = [self.generate_vector(384) for _ in range(num_docs)]
        
        # Measure insert time
        start_time = time.time()
        
        # Insert in batches
        batch_size = 100
        for i in range(0, num_docs, batch_size):
            batch_end = min(i + batch_size, num_docs)
            response = requests.post(
                f"{self.base_url}/api/v1/collections/{self.test_collection}/add",
                headers=self.headers,
                json={
                    "ids": ids[i:batch_end],
                    "documents": documents[i:batch_end],
                    "embeddings": embeddings[i:batch_end]
                }
            )
            self.assertEqual(response.status_code, 200)
        
        elapsed = time.time() - start_time
        throughput = num_docs / elapsed
        
        print(f"\nBulk Insert Performance: {throughput:.2f} docs/sec")
        self.assertGreater(throughput, 100, "Throughput too low")
    
    def test_search_performance(self):
        """Test search performance"""
        # Setup collection with data
        self.test_chromadb_add_documents()
        
        # Perform multiple searches
        num_searches = 100
        start_time = time.time()
        
        for _ in range(num_searches):
            response = requests.post(
                f"{self.base_url}/api/v1/collections/{self.test_collection}/query",
                headers=self.headers,
                json={
                    "query_embeddings": [self.generate_vector(384)],
                    "n_results": 10
                }
            )
            self.assertEqual(response.status_code, 200)
        
        elapsed = time.time() - start_time
        qps = num_searches / elapsed
        
        print(f"\nSearch Performance: {qps:.2f} queries/sec")
        self.assertGreater(qps, 10, "Query performance too low")
    
    # ============================================
    # Error Handling Tests
    # ============================================
    
    def test_invalid_collection_name(self):
        """Test handling of invalid collection names"""
        response = requests.post(
            f"{self.base_url}/api/v1/collections",
            headers=self.headers,
            json={
                "name": "invalid/collection*name",
                "metadata": {"dimension": 384}
            }
        )
        self.assertNotEqual(response.status_code, 200)
    
    def test_dimension_mismatch(self):
        """Test handling of dimension mismatch"""
        # Create collection with dimension 128
        response = requests.post(
            f"{self.base_url}/api/v1/collections",
            headers=self.headers,
            json={
                "name": self.test_collection,
                "metadata": {"dimension": 128}
            }
        )
        self.assertIn(response.status_code, [200, 201])
        
        # Try to add vectors with dimension 256
        response = requests.post(
            f"{self.base_url}/api/v1/collections/{self.test_collection}/add",
            headers=self.headers,
            json={
                "ids": ["doc1"],
                "documents": ["Test"],
                "embeddings": [self.generate_vector(256)]  # Wrong dimension
            }
        )
        self.assertNotEqual(response.status_code, 200)
    
    def test_authentication_required(self):
        """Test that authentication is required for write operations"""
        # Try to create collection without auth
        response = requests.post(
            f"{self.base_url}/api/v1/collections",
            json={
                "name": "unauthorized_collection",
                "metadata": {"dimension": 384}
            }
        )
        self.assertEqual(response.status_code, 401)


if __name__ == "__main__":
    # Run tests with verbose output
    unittest.main(verbosity=2)