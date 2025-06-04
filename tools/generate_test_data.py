#!/usr/bin/env python3

"""
VexFS Test Data Generator

Generates comprehensive test datasets for VexFS development and benchmarking
"""

import json
import numpy as np
import argparse
import os
from typing import List, Dict, Any
import time
import random


class TestDataGenerator:
    """Generate various types of test data for VexFS"""
    
    def __init__(self, output_dir: str = "test_data"):
        self.output_dir = output_dir
        os.makedirs(output_dir, exist_ok=True)
        
        # Sample text corpus for generating realistic embeddings
        self.text_corpus = [
            "artificial intelligence and machine learning",
            "vector databases and similarity search",
            "neural networks and deep learning",
            "natural language processing and text analysis",
            "computer vision and image recognition",
            "recommendation systems and collaborative filtering",
            "data science and statistical analysis",
            "cloud computing and distributed systems",
            "software engineering and system design",
            "cybersecurity and information protection",
            "blockchain technology and cryptocurrency",
            "internet of things and edge computing",
            "augmented reality and virtual reality",
            "robotics and autonomous systems",
            "quantum computing and quantum algorithms",
            "bioinformatics and computational biology",
            "financial technology and algorithmic trading",
            "healthcare technology and medical devices",
            "renewable energy and smart grids",
            "transportation and autonomous vehicles"
        ]
    
    def generate_embeddings_dataset(
        self, 
        name: str,
        num_vectors: int,
        dimensions: int,
        include_metadata: bool = True
    ) -> Dict[str, Any]:
        """Generate a dataset of random embeddings"""
        
        print(f"📊 Generating {name} dataset: {num_vectors} vectors × {dimensions} dimensions")
        
        # Generate random vectors (normalized)
        vectors = []
        metadata = []
        
        for i in range(num_vectors):
            # Generate random vector
            vector = np.random.randn(dimensions).astype(np.float32)
            # Normalize to unit length
            vector = vector / np.linalg.norm(vector)
            vectors.append(vector.tolist())
            
            if include_metadata:
                # Generate realistic metadata
                category = random.choice([
                    "technology", "science", "business", "health", 
                    "education", "entertainment", "sports", "travel"
                ])
                
                source = random.choice([
                    "web", "document", "api", "upload", "crawl"
                ])
                
                text_sample = random.choice(self.text_corpus)
                
                meta = {
                    "id": f"{name}_{i:06d}",
                    "text": f"{text_sample} - sample {i}",
                    "category": category,
                    "source": source,
                    "timestamp": time.time() - random.randint(0, 86400 * 30),  # Last 30 days
                    "confidence": random.uniform(0.7, 1.0),
                    "index": i
                }
                metadata.append(meta)
        
        dataset = {
            "name": name,
            "description": f"Test dataset with {num_vectors} {dimensions}D vectors",
            "num_vectors": num_vectors,
            "dimensions": dimensions,
            "vectors": vectors,
            "metadata": metadata if include_metadata else None,
            "generated_at": time.time(),
            "generator_version": "1.0.0"
        }
        
        return dataset
    
    def generate_benchmark_suite(self) -> Dict[str, Any]:
        """Generate a comprehensive benchmark suite"""
        
        print("🏁 Generating benchmark suite...")
        
        benchmark_configs = [
            {"name": "tiny", "vectors": 100, "dims": 128},
            {"name": "small", "vectors": 1000, "dims": 384},
            {"name": "medium", "vectors": 10000, "dims": 768},
            {"name": "large", "vectors": 50000, "dims": 1024},
            {"name": "xlarge", "vectors": 100000, "dims": 1536}
        ]
        
        benchmark_suite = {
            "name": "VexFS Benchmark Suite",
            "description": "Comprehensive benchmarking datasets for VexFS performance testing",
            "datasets": {},
            "generated_at": time.time()
        }
        
        for config in benchmark_configs:
            dataset = self.generate_embeddings_dataset(
                config["name"],
                config["vectors"],
                config["dims"]
            )
            
            # Save individual dataset
            filename = f"benchmark_{config['name']}.json"
            filepath = os.path.join(self.output_dir, filename)
            
            with open(filepath, 'w') as f:
                json.dump(dataset, f, indent=2)
            
            print(f"  ✅ Saved {filename}")
            
            # Add to suite (without vectors to keep manifest small)
            benchmark_suite["datasets"][config["name"]] = {
                "file": filename,
                "vectors": config["vectors"],
                "dimensions": config["dims"],
                "size_mb": os.path.getsize(filepath) / (1024 * 1024)
            }
        
        return benchmark_suite
    
    def generate_query_sets(self, num_queries: int = 100) -> Dict[str, Any]:
        """Generate query sets for testing search functionality"""
        
        print(f"🔍 Generating {num_queries} test queries...")
        
        query_types = [
            {"name": "random", "description": "Random normalized vectors"},
            {"name": "corpus_based", "description": "Queries based on text corpus"},
            {"name": "clustered", "description": "Queries clustered around specific regions"}
        ]
        
        query_sets = {
            "name": "VexFS Query Test Sets",
            "description": "Test queries for search functionality validation",
            "generated_at": time.time(),
            "query_types": {}
        }
        
        for query_type in query_types:
            queries = []
            
            for i in range(num_queries):
                if query_type["name"] == "random":
                    # Random normalized vector
                    vector = np.random.randn(384).astype(np.float32)
                    vector = vector / np.linalg.norm(vector)
                    text = f"Random query {i}"
                    
                elif query_type["name"] == "corpus_based":
                    # Based on text corpus
                    text = random.choice(self.text_corpus)
                    vector = self._text_to_embedding(text)
                    
                elif query_type["name"] == "clustered":
                    # Clustered around specific points
                    cluster_center = np.random.randn(384).astype(np.float32)
                    cluster_center = cluster_center / np.linalg.norm(cluster_center)
                    
                    # Add small random noise
                    noise = np.random.randn(384).astype(np.float32) * 0.1
                    vector = cluster_center + noise
                    vector = vector / np.linalg.norm(vector)
                    text = f"Clustered query {i}"
                
                queries.append({
                    "id": f"{query_type['name']}_query_{i:03d}",
                    "text": text,
                    "vector": vector.tolist(),
                    "expected_results": 10,  # Expected number of results
                    "metadata": {
                        "type": query_type["name"],
                        "index": i
                    }
                })
            
            query_sets["query_types"][query_type["name"]] = {
                "description": query_type["description"],
                "queries": queries
            }
        
        return query_sets
    
    def generate_stress_test_data(self) -> Dict[str, Any]:
        """Generate data for stress testing"""
        
        print("💪 Generating stress test data...")
        
        stress_tests = {
            "name": "VexFS Stress Tests",
            "description": "High-load test scenarios for VexFS",
            "generated_at": time.time(),
            "scenarios": {}
        }
        
        scenarios = [
            {
                "name": "high_dimensional",
                "description": "High-dimensional vectors (4096D)",
                "vectors": 1000,
                "dimensions": 4096
            },
            {
                "name": "many_small",
                "description": "Many small vectors (64D)",
                "vectors": 100000,
                "dimensions": 64
            },
            {
                "name": "concurrent_writes",
                "description": "Concurrent write simulation",
                "vectors": 10000,
                "dimensions": 512
            }
        ]
        
        for scenario in scenarios:
            dataset = self.generate_embeddings_dataset(
                scenario["name"],
                scenario["vectors"],
                scenario["dimensions"]
            )
            
            filename = f"stress_{scenario['name']}.json"
            filepath = os.path.join(self.output_dir, filename)
            
            with open(filepath, 'w') as f:
                json.dump(dataset, f, indent=2)
            
            stress_tests["scenarios"][scenario["name"]] = {
                "description": scenario["description"],
                "file": filename,
                "vectors": scenario["vectors"],
                "dimensions": scenario["dimensions"],
                "size_mb": os.path.getsize(filepath) / (1024 * 1024)
            }
            
            print(f"  ✅ Generated {filename}")
        
        return stress_tests
    
    def _text_to_embedding(self, text: str) -> np.ndarray:
        """Convert text to a simple embedding (demo purposes)"""
        words = text.lower().split()
        embedding = np.zeros(384, dtype=np.float32)
        
        for i, word in enumerate(words):
            hash_val = hash(word) % len(embedding)
            embedding[hash_val] += 1.0 / (i + 1)
        
        # Normalize
        norm = np.linalg.norm(embedding)
        if norm > 0:
            embedding = embedding / norm
            
        return embedding
    
    def generate_all(self):
        """Generate all test datasets"""
        
        print("🚀 VexFS Test Data Generator")
        print("===========================\n")
        
        # Generate benchmark suite
        benchmark_suite = self.generate_benchmark_suite()
        
        # Generate query sets
        query_sets = self.generate_query_sets()
        
        # Generate stress test data
        stress_tests = self.generate_stress_test_data()
        
        # Save manifests
        manifests = {
            "benchmark_suite": benchmark_suite,
            "query_sets": query_sets,
            "stress_tests": stress_tests
        }
        
        for name, manifest in manifests.items():
            filename = f"{name}_manifest.json"
            filepath = os.path.join(self.output_dir, filename)
            
            with open(filepath, 'w') as f:
                json.dump(manifest, f, indent=2)
            
            print(f"📋 Saved {filename}")
        
        # Generate summary
        self._generate_summary()
        
        print(f"\n✅ Test data generation complete!")
        print(f"📁 Output directory: {self.output_dir}")
        print(f"📊 Total files generated: {len(os.listdir(self.output_dir))}")
        
        # Calculate total size
        total_size = sum(
            os.path.getsize(os.path.join(self.output_dir, f))
            for f in os.listdir(self.output_dir)
        )
        print(f"💾 Total size: {total_size / (1024 * 1024):.1f} MB")
    
    def _generate_summary(self):
        """Generate a summary of all test data"""
        
        files = os.listdir(self.output_dir)
        
        summary = {
            "name": "VexFS Test Data Summary",
            "description": "Summary of all generated test datasets",
            "generated_at": time.time(),
            "output_directory": self.output_dir,
            "files": {},
            "total_files": len(files),
            "total_size_mb": 0
        }
        
        for filename in files:
            filepath = os.path.join(self.output_dir, filename)
            size_mb = os.path.getsize(filepath) / (1024 * 1024)
            
            summary["files"][filename] = {
                "size_mb": round(size_mb, 2),
                "type": self._get_file_type(filename)
            }
            
            summary["total_size_mb"] += size_mb
        
        summary["total_size_mb"] = round(summary["total_size_mb"], 2)
        
        # Save summary
        summary_path = os.path.join(self.output_dir, "SUMMARY.json")
        with open(summary_path, 'w') as f:
            json.dump(summary, f, indent=2)
    
    def _get_file_type(self, filename: str) -> str:
        """Determine file type from filename"""
        if "benchmark" in filename:
            return "benchmark_dataset"
        elif "stress" in filename:
            return "stress_test"
        elif "query" in filename:
            return "query_set"
        elif "manifest" in filename:
            return "manifest"
        elif "SUMMARY" in filename:
            return "summary"
        else:
            return "unknown"


def main():
    """Main function"""
    parser = argparse.ArgumentParser(description="Generate VexFS test data")
    parser.add_argument(
        "--output-dir",
        default="test_data",
        help="Output directory for test data (default: test_data)"
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Generate smaller datasets for quick testing"
    )
    
    args = parser.parse_args()
    
    generator = TestDataGenerator(args.output_dir)
    
    if args.quick:
        print("⚡ Quick mode: generating smaller datasets")
        # Override with smaller sizes for quick testing
        generator.text_corpus = generator.text_corpus[:5]
    
    generator.generate_all()


if __name__ == "__main__":
    main()