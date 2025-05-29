#!/usr/bin/env python3
"""
VexFS Python SDK Basic Usage Example

This example demonstrates how to use the VexFS Python SDK for:
- Initializing VexFS
- Adding documents with metadata
- Querying for similar documents
- Deleting documents
- Getting filesystem statistics
"""

import sys
import os
import numpy as np
from typing import List, Dict, Any

# Add the bindings directory to the path so we can import vexfs
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../bindings/python/src'))

try:
    import vexfs
    from client import VexFSClient
except ImportError as e:
    print(f"Error importing VexFS: {e}")
    print("Make sure you have built the Python bindings with 'maturin develop'")
    sys.exit(1)


def main():
    print("🚀 VexFS Python SDK Basic Usage Example")
    print("=" * 50)
    
    # Example 1: Using the high-level client
    print("\n📚 Example 1: Using VexFSClient")
    try:
        # Initialize client
        client = VexFSClient()
        
        # For this example, we'll use a temporary mount point
        mount_point = "/tmp/vexfs_example"
        os.makedirs(mount_point, exist_ok=True)
        
        print(f"Initializing VexFS at: {mount_point}")
        client.init(mount_point)
        
        # Get version info
        version_info = client.version()
        print(f"VexFS Version: {version_info}")
        
        # Add some documents
        print("\n📝 Adding documents...")
        
        doc1_id = client.add(
            "The quick brown fox jumps over the lazy dog",
            {"category": "animals", "type": "sentence"}
        )
        print(f"Added document 1: {doc1_id}")
        
        doc2_id = client.add(
            "Machine learning is a subset of artificial intelligence",
            {"category": "technology", "type": "definition"}
        )
        print(f"Added document 2: {doc2_id}")
        
        doc3_id = client.add(
            "Vector databases enable semantic search capabilities",
            {"category": "technology", "type": "explanation"}
        )
        print(f"Added document 3: {doc3_id}")
        
        # Generate a query vector (in practice, this would come from an embedding model)
        print("\n🔍 Querying for similar documents...")
        query_vector = generate_sample_vector()
        
        results = client.query(query_vector, top_k=2)
        print(f"Found {len(results)} similar documents:")
        
        for i, (doc_id, score, text) in enumerate(results, 1):
            print(f"  {i}. ID: {doc_id}")
            print(f"     Score: {score:.4f}")
            print(f"     Text: {text}")
            print()
        
        # Get statistics
        print("📊 Filesystem Statistics:")
        stats = client.stats()
        for key, value in stats.items():
            print(f"  {key}: {value}")
        
        # Delete a document
        print(f"\n🗑️  Deleting document: {doc1_id}")
        client.delete(doc1_id)
        print("Document deleted successfully")
        
    except Exception as e:
        print(f"❌ Error with VexFSClient: {e}")
    
    # Example 2: Using the functional API
    print("\n📚 Example 2: Using Functional API")
    try:
        # Initialize VexFS
        mount_point = "/tmp/vexfs_functional"
        os.makedirs(mount_point, exist_ok=True)
        
        print(f"Initializing VexFS at: {mount_point}")
        vexfs.init(mount_point)
        
        # Add documents
        print("\n📝 Adding documents...")
        
        doc1_id = vexfs.add(
            "Python is a high-level programming language",
            {"language": "python", "topic": "programming"}
        )
        print(f"Added document: {doc1_id}")
        
        doc2_id = vexfs.add(
            "Rust is a systems programming language focused on safety",
            {"language": "rust", "topic": "programming"}
        )
        print(f"Added document: {doc2_id}")
        
        # Query documents
        print("\n🔍 Querying documents...")
        query_vector = generate_sample_vector()
        results = vexfs.query(query_vector, 5)
        
        print(f"Found {len(results)} results:")
        for doc_id, score, text in results:
            print(f"  - {doc_id}: {score:.4f} - {text}")
        
        # Delete document
        print(f"\n🗑️  Deleting document: {doc2_id}")
        vexfs.delete(doc2_id)
        print("Document deleted successfully")
        
    except Exception as e:
        print(f"❌ Error with functional API: {e}")
    
    print("\n✅ Example completed!")


def generate_sample_vector() -> List[float]:
    """
    Generate a sample vector for querying.
    In a real application, this would come from an embedding model.
    """
    # Generate a random normalized vector
    vector = np.random.normal(0, 1, 384)  # 384-dimensional vector
    vector = vector / np.linalg.norm(vector)  # Normalize
    return vector.tolist()


if __name__ == "__main__":
    main()