"""
API Endpoint Tests

This module contains tests for the VexFS v2 Qdrant adapter API endpoints.
"""

import pytest
import asyncio
from fastapi.testclient import TestClient
from unittest.mock import Mock, patch
import json

from src.main import app
from src.core.vexfs_client import VexFSClient, VexFSError


@pytest.fixture
def client():
    """Test client fixture"""
    return TestClient(app)


@pytest.fixture
def mock_vexfs_client():
    """Mock VexFS client fixture"""
    mock_client = Mock(spec=VexFSClient)
    
    # Mock successful responses
    mock_client.create_collection.return_value = {
        "name": "test_collection",
        "config": {
            "params": {
                "vectors": {
                    "size": 128,
                    "distance": "Cosine"
                }
            }
        },
        "status": "green"
    }
    
    mock_client.get_collection_info.return_value = {
        "status": "green",
        "optimizer_status": "ok",
        "vectors_count": 0,
        "indexed_vectors_count": 0,
        "points_count": 0,
        "segments_count": 1,
        "config": {
            "params": {
                "vectors": {
                    "size": 128,
                    "distance": "Cosine"
                }
            }
        }
    }
    
    mock_client.list_collections.return_value = {
        "collections": {
            "test_collection": {
                "status": "green",
                "vectors_count": 0,
                "config": {
                    "params": {
                        "vectors": {
                            "size": 128,
                            "distance": "Cosine"
                        }
                    }
                }
            }
        }
    }
    
    mock_client.insert_points.return_value = {
        "operation_id": 1,
        "status": "completed"
    }
    
    mock_client.search_vectors.return_value = []
    
    return mock_client


class TestCollectionsAPI:
    """Test collection management endpoints"""
    
    @patch('src.api.collections.get_vexfs_client')
    def test_create_collection(self, mock_get_client, client, mock_vexfs_client):
        """Test collection creation"""
        mock_get_client.return_value = mock_vexfs_client
        
        collection_config = {
            "vectors": {
                "size": 128,
                "distance": "Cosine"
            }
        }
        
        response = client.put("/collections/test_collection", json=collection_config)
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "result" in data
        
        mock_vexfs_client.create_collection.assert_called_once_with(
            name="test_collection",
            dimensions=128,
            distance="Cosine"
        )
    
    @patch('src.api.collections.get_vexfs_client')
    def test_get_collection_info(self, mock_get_client, client, mock_vexfs_client):
        """Test getting collection information"""
        mock_get_client.return_value = mock_vexfs_client
        
        response = client.get("/collections/test_collection")
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "result" in data
        
        mock_vexfs_client.get_collection_info.assert_called_once_with("test_collection")
    
    @patch('src.api.collections.get_vexfs_client')
    def test_list_collections(self, mock_get_client, client, mock_vexfs_client):
        """Test listing collections"""
        mock_get_client.return_value = mock_vexfs_client
        
        response = client.get("/collections")
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "result" in data
        assert "collections" in data["result"]
        
        mock_vexfs_client.list_collections.assert_called_once()
    
    @patch('src.api.collections.get_vexfs_client')
    def test_delete_collection(self, mock_get_client, client, mock_vexfs_client):
        """Test collection deletion"""
        mock_get_client.return_value = mock_vexfs_client
        mock_vexfs_client.delete_collection.return_value = True
        
        response = client.delete("/collections/test_collection")
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        
        mock_vexfs_client.delete_collection.assert_called_once_with("test_collection")


class TestPointsAPI:
    """Test point operations endpoints"""
    
    @patch('src.api.points.get_vexfs_client')
    def test_upsert_points(self, mock_get_client, client, mock_vexfs_client):
        """Test point upsert"""
        mock_get_client.return_value = mock_vexfs_client
        
        points_data = {
            "points": [
                {
                    "id": 1,
                    "vector": [0.1] * 128,
                    "payload": {"key": "value"}
                }
            ]
        }
        
        response = client.put("/collections/test_collection/points", json=points_data)
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "result" in data
        
        mock_vexfs_client.insert_points.assert_called_once()
    
    @patch('src.api.points.get_vexfs_client')
    def test_search_points(self, mock_get_client, client, mock_vexfs_client):
        """Test vector search"""
        mock_get_client.return_value = mock_vexfs_client
        
        search_request = {
            "vector": [0.1] * 128,
            "limit": 10,
            "with_payload": True,
            "with_vector": False
        }
        
        response = client.post("/collections/test_collection/points/search", json=search_request)
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "result" in data
        
        mock_vexfs_client.search_vectors.assert_called_once()


class TestClusterAPI:
    """Test cluster information endpoints"""
    
    def test_health_check(self, client):
        """Test health check endpoint"""
        response = client.get("/")
        
        assert response.status_code == 200
        data = response.json()
        assert "title" in data
        assert "version" in data
        assert "backend" in data
    
    def test_cluster_info(self, client):
        """Test cluster information endpoint"""
        response = client.get("/cluster")
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "result" in data


class TestErrorHandling:
    """Test error handling"""
    
    @patch('src.api.collections.get_vexfs_client')
    def test_collection_not_found(self, mock_get_client, client, mock_vexfs_client):
        """Test collection not found error"""
        mock_get_client.return_value = mock_vexfs_client
        mock_vexfs_client.get_collection_info.side_effect = VexFSError("Collection 'nonexistent' not found")
        
        response = client.get("/collections/nonexistent")
        
        assert response.status_code == 404
        data = response.json()
        assert data["status"] == "error"
    
    @patch('src.api.collections.get_vexfs_client')
    def test_collection_already_exists(self, mock_get_client, client, mock_vexfs_client):
        """Test collection already exists error"""
        mock_get_client.return_value = mock_vexfs_client
        mock_vexfs_client.create_collection.side_effect = VexFSError("Collection 'test' already exists")
        
        collection_config = {
            "vectors": {
                "size": 128,
                "distance": "Cosine"
            }
        }
        
        response = client.put("/collections/test", json=collection_config)
        
        assert response.status_code == 409
        data = response.json()
        assert data["status"] == "error"


if __name__ == "__main__":
    pytest.main([__file__])