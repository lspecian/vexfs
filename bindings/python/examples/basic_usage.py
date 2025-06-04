#!/usr/bin/env python3

"""
VexFS Python Basic Usage Example

Simple example showing how to get started with VexFS for vector storage and search
"""

import sys
import os
import json
import time
import numpy as np
from typing import List, Dict, Any

# Add the parent directory to the path to import vexfs
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

try:
    import vexfs
except ImportError:
    print("❌ VexFS Python bindings not found. Please install with:")
    print("   pip install vexfs")
    sys.exit(1)


class VexFSBasicExample:
    """Basic VexFS usage example"""
    
    def __init__(self, mount_path: str = "/mnt/vexfs"):
        self.mount_path = mount_path
        self.client = vexfs.VexFSClient(mount_path)
        
    def run_example(self):
        """Run the complete basic example"""
        print("🚀 VexFS Python Basic Usage Example")
        print("===================================\n")
        
        try:
            # 1. Check VexFS status
            self._check_status()
            
            # 2. Create a collection
            self._create_collection()
            
            # 3. Add sample documents
            doc_ids = self._add_sample_documents()
            
            # 4. Perform searches
            self._perform_searches()
            
            # 5. Show collection info
            self._show_collection_info()
            
            print("\n✅ Basic example completed successfully!")
            
        except Exception as e:
            print(f"❌ Error: {e}")
            sys.exit(1)
    
    def _check_status(self):
        """Check VexFS status"""
        print("📋 Checking VexFS status...")
        
        try:
            version = self.client.get_version()
            print(f"✅ VexFS Version: {version}")
            
            # Check if mount point is accessible
            if not os.path.exists(self.mount_path):
                raise Exception(f"Mount path not found: {self.mount_path}")
                
            print(f"✅ Mount path accessible: {self.mount_path}")
            
        except Exception as e:
            print(f"❌ VexFS status check failed: {e}")
            print("\nTroubleshooting:")
            print("  1. Ensure VexFS is mounted at the specified path")
            print("  2. Check that you have read/write permissions")
            print("  3. Verify the VexFS kernel module is loaded")
            raise
    
    def _create_collection(self):
        """Create a sample collection"""
        print("\n📁 Creating collection...")
        
        collection_name = "python_examples"
        metadata = {
            "description": "Python examples collection",
            "created_at": time.time(),
            "language": "python"
        }
        
        try:
            self.client.create_collection(collection_name, metadata)
            print(f"✅ Created collection: {collection_name}")
        except Exception as e:
            if "already exists" in str(e).lower():
                print(f"ℹ️  Collection already exists: {collection_name}")
            else:
                raise
    
    def _add_sample_documents(self) -> List[str]:
        """Add sample documents with embeddings"""
        print("\n📄 Adding sample documents...")
        
        # Sample documents about AI and technology
        documents = [
            "VexFS is a high-performance vector filesystem designed for AI applications",
            "Machine learning models require efficient vector storage and retrieval systems",
            "Similarity search enables recommendation systems and content discovery",
            "Vector databases are essential infrastructure for modern AI pipelines",
            "Semantic search allows finding documents based on meaning rather than keywords",
            "Neural networks generate high-dimensional embeddings for text and images",
            "HNSW (Hierarchical Navigable Small World) provides fast approximate nearest neighbor search",
            "LSH (Locality Sensitive Hashing) enables efficient similarity detection",
            "AI applications benefit from optimized vector operations and indexing",
            "Real-time vector search is crucial for interactive AI systems"
        ]
        
        doc_ids = []
        collection_name = "python_examples"
        
        print(f"📊 Indexing {len(documents)} documents...")
        start_time = time.time()
        
        for i, doc in enumerate(documents):
            # Generate embedding (in production, use proper embedding model)
            embedding = self._generate_embedding(doc)
            
            # Add document with metadata
            metadata = {
                "text": doc,
                "category": "ai_tech",
                "index": i,
                "added_at": time.time()
            }
            
            doc_id = self.client.add_vector(
                collection_name,
                embedding,
                metadata
            )
            
            doc_ids.append(doc_id)
            print(f"  ✅ Added document {i+1}: {doc_id}")
        
        duration = time.time() - start_time
        throughput = len(documents) / duration
        
        print(f"\n📈 Indexed {len(doc_ids)} documents in {duration:.2f}s")
        print(f"⚡ Throughput: {throughput:.2f} docs/sec")
        
        return doc_ids
    
    def _perform_searches(self):
        """Perform various search queries"""
        print("\n🔍 Performing search queries...")
        
        queries = [
            "vector storage for machine learning",
            "similarity search algorithms",
            "real-time AI applications"
        ]
        
        collection_name = "python_examples"
        
        for i, query in enumerate(queries, 1):
            print(f"\n{i}. Searching: '{query}'")
            
            # Generate query embedding
            query_embedding = self._generate_embedding(query)
            
            # Perform search
            start_time = time.time()
            results = self.client.search_vectors(
                collection_name,
                query_embedding,
                top_k=3
            )
            search_time = time.time() - start_time
            
            print(f"   📊 Found {len(results)} results in {search_time*1000:.1f}ms")
            
            # Display results
            for j, result in enumerate(results, 1):
                score = result.get('score', 0)
                text = result.get('metadata', {}).get('text', 'N/A')
                print(f"   {j}. Score: {score:.4f}")
                print(f"      Text: {text[:80]}...")
    
    def _show_collection_info(self):
        """Show collection information"""
        print("\n📋 Collection Information")
        print("========================")
        
        try:
            collections = self.client.list_collections()
            print(f"Total collections: {len(collections)}")
            
            for collection in collections:
                print(f"\n📁 {collection['name']}")
                if 'metadata' in collection:
                    metadata = collection['metadata']
                    if 'description' in metadata:
                        print(f"   Description: {metadata['description']}")
                    if 'created_at' in metadata:
                        created_time = time.ctime(metadata['created_at'])
                        print(f"   Created: {created_time}")
                
                # Get collection stats
                try:
                    stats = self.client.get_collection_stats(collection['name'])
                    print(f"   Documents: {stats.get('document_count', 'N/A')}")
                    print(f"   Dimensions: {stats.get('vector_dimension', 'N/A')}")
                except:
                    print("   Stats: Not available")
                    
        except Exception as e:
            print(f"❌ Failed to get collection info: {e}")
    
    def _generate_embedding(self, text: str) -> List[float]:
        """
        Generate a simple embedding for demonstration
        In production, use proper embedding models like:
        - OpenAI embeddings
        - Sentence Transformers
        - Hugging Face models
        """
        # Simple hash-based embedding for demo
        words = text.lower().split()
        embedding = np.zeros(384, dtype=np.float32)
        
        for i, word in enumerate(words):
            # Simple hash function
            hash_val = hash(word) % len(embedding)
            embedding[hash_val] += 1.0 / (i + 1)
        
        # Normalize
        norm = np.linalg.norm(embedding)
        if norm > 0:
            embedding = embedding / norm
            
        return embedding.tolist()


def main():
    """Main function"""
    import argparse
    
    parser = argparse.ArgumentParser(description="VexFS Python Basic Usage Example")
    parser.add_argument(
        "--mount-path",
        default="/mnt/vexfs",
        help="VexFS mount path (default: /mnt/vexfs)"
    )
    
    args = parser.parse_args()
    
    # Run the example
    example = VexFSBasicExample(args.mount_path)
    example.run_example()


if __name__ == "__main__":
    main()