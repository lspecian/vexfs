"""
VexFS Python Client

This module provides a high-level Python interface for VexFS operations.
"""

import vexfs
from typing import Dict, List, Optional, Tuple, Any


class VexFSClient:
    """
    High-level Python client for VexFS vector-native filesystem.
    
    This client provides both object-oriented and functional interfaces
    to VexFS operations including document storage, vector search, and
    metadata management.
    """
    
    def __init__(self, mount_point: Optional[str] = None):
        """
        Initialize VexFS client.
        
        Args:
            mount_point: Optional mount point for VexFS. If provided,
                        automatically initializes the filesystem.
        """
        self._client = vexfs.VexFSClient()
        if mount_point:
            self.init(mount_point)
    
    def init(self, mount_point: str) -> None:
        """
        Initialize VexFS with a mount point.
        
        Args:
            mount_point: Path where VexFS should be mounted
            
        Raises:
            RuntimeError: If initialization fails
        """
        self._client.init(mount_point)
    
    def add(self, text: str, metadata: Optional[Dict[str, Any]] = None) -> str:
        """
        Add a document to VexFS with automatic embedding generation.
        
        Args:
            text: Document content to add
            metadata: Optional metadata dictionary
            
        Returns:
            Document ID for the added document
            
        Raises:
            RuntimeError: If document addition fails
        """
        return self._client.add(text, metadata)
    
    def query(self, vector: List[float], top_k: int = 10) -> List[Tuple[str, float, Optional[str]]]:
        """
        Query for similar documents using a vector.
        
        Args:
            vector: Query vector as list of floats
            top_k: Number of top results to return
            
        Returns:
            List of tuples containing (document_id, similarity_score, document_text)
            
        Raises:
            RuntimeError: If query fails
        """
        return self._client.query(vector, top_k)
    
    def delete(self, doc_id: str) -> None:
        """
        Delete a document by ID.
        
        Args:
            doc_id: ID of document to delete
            
        Raises:
            RuntimeError: If deletion fails
        """
        self._client.delete(doc_id)
    
    def stats(self) -> Dict[str, str]:
        """
        Get VexFS statistics.
        
        Returns:
            Dictionary containing filesystem statistics
            
        Raises:
            RuntimeError: If stats retrieval fails
        """
        return self._client.stats()
    
    def version(self) -> Dict[str, str]:
        """
        Get VexFS version information.
        
        Returns:
            Dictionary containing version information
        """
        return self._client.version()


# Convenience functions for backward compatibility
def init(mount_point: str) -> None:
    """Initialize VexFS with a mount point."""
    vexfs.init(mount_point)


def add(text: str, metadata: Optional[Dict[str, str]] = None) -> str:
    """Add a document with automatic embedding generation."""
    return vexfs.add(text, metadata)


def query(vector: List[float], top_k: int = 10) -> List[Tuple[str, float, Optional[str]]]:
    """Query for similar documents using a vector."""
    return vexfs.query(vector, top_k)


def delete(doc_id: str) -> None:
    """Delete a document by ID."""
    vexfs.delete(doc_id)


__all__ = ['VexFSClient', 'init', 'add', 'query', 'delete']