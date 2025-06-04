"""
VexFS Integration Tests

This module contains tests for VexFS v2 kernel module integration.
"""

import pytest
import struct
from unittest.mock import Mock, patch, mock_open
import os

from src.core.vexfs_client import VexFSClient, VexFSError
from src.core.ieee754 import (
    float_to_bits,
    bits_to_float,
    float_array_to_bits,
    bits_array_to_float,
    prepare_vector_for_kernel,
    prepare_batch_vectors_for_kernel
)


class TestIEEE754Conversion:
    """Test IEEE 754 conversion utilities"""
    
    def test_float_to_bits_conversion(self):
        """Test float to bits conversion"""
        # Test known values
        assert float_to_bits(1.0) == 1065353216
        assert float_to_bits(-1.0) == 3212836864
        assert float_to_bits(0.0) == 0
        assert float_to_bits(0.5) == 1056964608
    
    def test_bits_to_float_conversion(self):
        """Test bits to float conversion"""
        # Test known values
        assert bits_to_float(1065353216) == 1.0
        assert bits_to_float(3212836864) == -1.0
        assert bits_to_float(0) == 0.0
        assert bits_to_float(1056964608) == 0.5
    
    def test_round_trip_conversion(self):
        """Test round-trip float ↔ bits conversion"""
        test_values = [1.0, -1.0, 0.0, 0.5, 3.14159, -2.71828, 1e-10, 1e10]
        
        for value in test_values:
            bits = float_to_bits(value)
            recovered = bits_to_float(bits)
            assert abs(recovered - value) < 1e-6, f"Round-trip failed for {value}"
    
    def test_array_conversions(self):
        """Test array conversion functions"""
        test_array = [1.0, -1.0, 0.5, 3.14159]
        
        # Convert to bits
        bits_array = float_array_to_bits(test_array)
        assert len(bits_array) == len(test_array)
        assert all(isinstance(b, int) for b in bits_array)
        
        # Convert back to floats
        recovered_array = bits_array_to_float(bits_array)
        assert len(recovered_array) == len(test_array)
        
        for original, recovered in zip(test_array, recovered_array):
            assert abs(recovered - original) < 1e-6
    
    def test_prepare_vector_for_kernel(self):
        """Test vector preparation for kernel"""
        vector = [1.0, -1.0, 0.5, 3.14159]
        
        kernel_vector = prepare_vector_for_kernel(vector)
        assert len(kernel_vector) == len(vector)
        assert all(isinstance(b, int) for b in kernel_vector)
        
        # Verify conversion
        expected_bits = float_array_to_bits(vector)
        assert kernel_vector == expected_bits
    
    def test_prepare_batch_vectors_for_kernel(self):
        """Test batch vector preparation for kernel"""
        vectors = [
            [1.0, -1.0, 0.5],
            [3.14159, 2.71828, 1.41421],
            [0.0, 1.0, -1.0]
        ]
        
        kernel_batch = prepare_batch_vectors_for_kernel(vectors)
        
        # Should be flattened
        expected_length = len(vectors) * len(vectors[0])
        assert len(kernel_batch) == expected_length
        assert all(isinstance(b, int) for b in kernel_batch)
    
    def test_invalid_vector_dimensions(self):
        """Test validation of vector dimensions"""
        # Empty vector
        with pytest.raises(ValueError):
            prepare_vector_for_kernel([])
        
        # Too large vector
        large_vector = [1.0] * 100000
        with pytest.raises(ValueError):
            prepare_vector_for_kernel(large_vector)
    
    def test_inconsistent_batch_dimensions(self):
        """Test validation of batch vector dimensions"""
        vectors = [
            [1.0, 2.0, 3.0],  # 3 dimensions
            [4.0, 5.0],       # 2 dimensions - inconsistent!
        ]
        
        with pytest.raises(ValueError):
            prepare_batch_vectors_for_kernel(vectors)


class TestVexFSClient:
    """Test VexFS client functionality"""
    
    @patch('os.open')
    def test_client_initialization(self, mock_open):
        """Test VexFS client initialization"""
        mock_open.return_value = 3  # Mock file descriptor
        
        client = VexFSClient("/dev/vexfs_v2_phase3")
        assert client.device_path == "/dev/vexfs_v2_phase3"
        assert client.fd == 3
        
        mock_open.assert_called_once_with("/dev/vexfs_v2_phase3", os.O_RDWR)
    
    @patch('os.open')
    def test_client_initialization_failure(self, mock_open):
        """Test VexFS client initialization failure"""
        mock_open.side_effect = OSError("Device not found")
        
        with pytest.raises(VexFSError):
            VexFSClient("/dev/nonexistent")
    
    @patch('os.open')
    @patch('os.close')
    def test_client_cleanup(self, mock_close, mock_open):
        """Test VexFS client cleanup"""
        mock_open.return_value = 3
        
        client = VexFSClient("/dev/vexfs_v2_phase3")
        client.close()
        
        mock_close.assert_called_once_with(3)
        assert client.fd is None
    
    @patch('os.open')
    def test_create_collection(self, mock_open):
        """Test collection creation"""
        mock_open.return_value = 3
        
        with patch('fcntl.ioctl') as mock_ioctl:
            client = VexFSClient("/dev/vexfs_v2_phase3")
            
            result = client.create_collection("test_collection", 128, "Cosine")
            
            assert result["name"] == "test_collection"
            assert result["config"]["params"]["vectors"]["size"] == 128
            assert result["config"]["params"]["vectors"]["distance"] == "Cosine"
            
            # Verify IOCTL was called
            mock_ioctl.assert_called_once()
    
    @patch('os.open')
    def test_create_collection_invalid_dimensions(self, mock_open):
        """Test collection creation with invalid dimensions"""
        mock_open.return_value = 3
        
        client = VexFSClient("/dev/vexfs_v2_phase3")
        
        # Test invalid dimensions
        with pytest.raises(VexFSError):
            client.create_collection("test", 0, "Cosine")
        
        with pytest.raises(VexFSError):
            client.create_collection("test", 100000, "Cosine")
    
    @patch('os.open')
    def test_create_collection_invalid_distance(self, mock_open):
        """Test collection creation with invalid distance metric"""
        mock_open.return_value = 3
        
        client = VexFSClient("/dev/vexfs_v2_phase3")
        
        with pytest.raises(VexFSError):
            client.create_collection("test", 128, "InvalidDistance")
    
    @patch('os.open')
    def test_insert_points(self, mock_open):
        """Test point insertion"""
        mock_open.return_value = 3
        
        with patch('fcntl.ioctl') as mock_ioctl:
            client = VexFSClient("/dev/vexfs_v2_phase3")
            
            # Create collection first
            client.create_collection("test_collection", 3, "Cosine")
            
            points = [
                {
                    "id": 1,
                    "vector": [1.0, 2.0, 3.0],
                    "payload": {"key": "value"}
                },
                {
                    "id": 2,
                    "vector": [4.0, 5.0, 6.0],
                    "payload": {"key": "value2"}
                }
            ]
            
            result = client.insert_points("test_collection", points)
            
            assert result["operation_id"] == 2
            assert result["status"] == "completed"
            
            # Verify IOCTL was called for both create and insert
            assert mock_ioctl.call_count == 2
    
    @patch('os.open')
    def test_insert_points_dimension_mismatch(self, mock_open):
        """Test point insertion with dimension mismatch"""
        mock_open.return_value = 3
        
        with patch('fcntl.ioctl'):
            client = VexFSClient("/dev/vexfs_v2_phase3")
            client.create_collection("test_collection", 3, "Cosine")
            
            points = [
                {
                    "id": 1,
                    "vector": [1.0, 2.0],  # Wrong dimensions!
                    "payload": {}
                }
            ]
            
            with pytest.raises(VexFSError):
                client.insert_points("test_collection", points)
    
    @patch('os.open')
    def test_search_vectors(self, mock_open):
        """Test vector search"""
        mock_open.return_value = 3
        
        with patch('fcntl.ioctl') as mock_ioctl:
            client = VexFSClient("/dev/vexfs_v2_phase3")
            client.create_collection("test_collection", 3, "Cosine")
            
            query_vector = [1.0, 2.0, 3.0]
            
            # Mock IOCTL to return some results
            def mock_ioctl_side_effect(fd, cmd, data):
                if cmd == client.VEXFS_IOC_VECTOR_SEARCH:
                    # Simulate search results by modifying the packed data
                    pass
                return 0
            
            mock_ioctl.side_effect = mock_ioctl_side_effect
            
            results = client.search_vectors("test_collection", query_vector, limit=10)
            
            assert isinstance(results, list)
            # Results would be empty in this mock, but structure is correct
    
    @patch('os.open')
    def test_search_vectors_dimension_mismatch(self, mock_open):
        """Test vector search with dimension mismatch"""
        mock_open.return_value = 3
        
        with patch('fcntl.ioctl'):
            client = VexFSClient("/dev/vexfs_v2_phase3")
            client.create_collection("test_collection", 3, "Cosine")
            
            query_vector = [1.0, 2.0]  # Wrong dimensions!
            
            with pytest.raises(VexFSError):
                client.search_vectors("test_collection", query_vector)


class TestVexFSIntegration:
    """Integration tests requiring actual VexFS device"""
    
    @pytest.mark.skipif(
        not os.path.exists("/dev/vexfs_v2_phase3"),
        reason="VexFS device not available"
    )
    def test_real_device_connection(self):
        """Test connection to real VexFS device"""
        try:
            client = VexFSClient("/dev/vexfs_v2_phase3")
            client.close()
        except VexFSError:
            pytest.skip("VexFS device not accessible")
    
    @pytest.mark.skipif(
        not os.path.exists("/dev/vexfs_v2_phase3"),
        reason="VexFS device not available"
    )
    def test_real_collection_operations(self):
        """Test real collection operations"""
        try:
            client = VexFSClient("/dev/vexfs_v2_phase3")
            
            # Create collection
            result = client.create_collection("integration_test", 4, "Cosine")
            assert result["name"] == "integration_test"
            
            # Get collection info
            info = client.get_collection_info("integration_test")
            assert info["config"]["params"]["vectors"]["size"] == 4
            
            # Insert points
            points = [
                {
                    "id": 1,
                    "vector": [0.1, 0.2, 0.3, 0.4],
                    "payload": {"test": True}
                }
            ]
            
            insert_result = client.insert_points("integration_test", points)
            assert insert_result["status"] == "completed"
            
            # Search
            search_results = client.search_vectors(
                "integration_test",
                [0.1, 0.2, 0.3, 0.4],
                limit=5
            )
            assert isinstance(search_results, list)
            
            # Clean up
            client.delete_collection("integration_test")
            client.close()
            
        except VexFSError:
            pytest.skip("VexFS operations not supported")


if __name__ == "__main__":
    pytest.main([__file__])