#!/usr/bin/env python3
"""
Real Dataset Loader for VexFS Competitive Benchmarking

This module provides realistic datasets for vector database benchmarking,
including document embeddings, semantic search datasets, and RAG workloads.
"""

import numpy as np
import pandas as pd
import json
import logging
from pathlib import Path
from typing import List, Dict, Tuple, Optional
from dataclasses import dataclass
import requests
import zipfile
import io
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.cluster import KMeans
from sklearn.preprocessing import normalize
import hashlib

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class DatasetConfig:
    """Configuration for dataset generation"""
    name: str
    size: int
    dimensions: int
    clusters: int
    noise_level: float = 0.1
    seed: int = 42

@dataclass
class VectorDataset:
    """Container for vector dataset with metadata"""
    vectors: np.ndarray
    metadata: List[Dict]
    queries: np.ndarray
    ground_truth: List[List[int]]
    config: DatasetConfig

class RealDatasetLoader:
    """Load and generate realistic vector datasets for benchmarking"""
    
    def __init__(self, cache_dir: str = "datasets/cache"):
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        
        # Common embedding dimensions used in practice
        self.common_dimensions = [384, 768, 1536]  # sentence-transformers, OpenAI, etc.
        
        # Predefined dataset configurations
        self.dataset_configs = {
            "small_docs": DatasetConfig("small_docs", 1000, 384, 10),
            "medium_docs": DatasetConfig("medium_docs", 5000, 768, 25),
            "large_docs": DatasetConfig("large_docs", 10000, 1536, 50),
            "semantic_search": DatasetConfig("semantic_search", 2000, 384, 15),
            "rag_knowledge": DatasetConfig("rag_knowledge", 8000, 768, 30),
        }
    
    def generate_realistic_embeddings(self, config: DatasetConfig) -> np.ndarray:
        """Generate realistic embeddings with clustering behavior"""
        np.random.seed(config.seed)
        
        # Create cluster centers
        cluster_centers = np.random.randn(config.clusters, config.dimensions)
        cluster_centers = normalize(cluster_centers, norm='l2')
        
        # Generate vectors around cluster centers
        vectors = []
        vectors_per_cluster = config.size // config.clusters
        
        for i in range(config.clusters):
            # Base cluster center
            center = cluster_centers[i]
            
            # Generate vectors around this center
            for _ in range(vectors_per_cluster):
                # Add noise to cluster center
                noise = np.random.randn(config.dimensions) * config.noise_level
                vector = center + noise
                
                # Normalize to unit length (common for embeddings)
                vector = normalize(vector.reshape(1, -1), norm='l2')[0]
                vectors.append(vector)
        
        # Fill remaining vectors if needed
        remaining = config.size - len(vectors)
        for _ in range(remaining):
            center_idx = np.random.randint(0, config.clusters)
            center = cluster_centers[center_idx]
            noise = np.random.randn(config.dimensions) * config.noise_level
            vector = normalize((center + noise).reshape(1, -1), norm='l2')[0]
            vectors.append(vector)
        
        return np.array(vectors)
    
    def generate_document_metadata(self, size: int, config: DatasetConfig) -> List[Dict]:
        """Generate realistic document metadata"""
        np.random.seed(config.seed)
        
        # Document types and domains
        doc_types = ["article", "paper", "blog", "documentation", "report", "tutorial"]
        domains = ["technology", "science", "business", "education", "healthcare", "finance"]
        authors = [f"author_{i}" for i in range(1, min(size // 10, 100) + 1)]
        
        metadata = []
        for i in range(size):
            doc_id = f"doc_{i:06d}"
            
            # Generate realistic metadata
            doc_meta = {
                "id": doc_id,
                "title": f"Document {i}: {np.random.choice(domains).title()} {np.random.choice(doc_types).title()}",
                "type": np.random.choice(doc_types),
                "domain": np.random.choice(domains),
                "author": np.random.choice(authors),
                "length": np.random.randint(100, 5000),  # Character count
                "timestamp": f"2024-{np.random.randint(1, 13):02d}-{np.random.randint(1, 29):02d}",
                "cluster_id": i // (size // config.clusters),  # Which cluster this belongs to
                "tags": np.random.choice(["important", "draft", "reviewed", "archived"], 
                                       size=np.random.randint(0, 3)).tolist()
            }
            metadata.append(doc_meta)
        
        return metadata
    
    def generate_query_vectors(self, base_vectors: np.ndarray, num_queries: int = 100) -> np.ndarray:
        """Generate realistic query vectors based on the dataset"""
        np.random.seed(42)
        
        # Select random base vectors and add small perturbations
        query_indices = np.random.choice(len(base_vectors), num_queries, replace=True)
        queries = []
        
        for idx in query_indices:
            base_vector = base_vectors[idx]
            # Add small perturbation to simulate query variation
            perturbation = np.random.randn(len(base_vector)) * 0.05
            query_vector = base_vector + perturbation
            query_vector = normalize(query_vector.reshape(1, -1), norm='l2')[0]
            queries.append(query_vector)
        
        return np.array(queries)
    
    def compute_ground_truth(self, queries: np.ndarray, vectors: np.ndarray, k: int = 10) -> List[List[int]]:
        """Compute ground truth nearest neighbors for queries"""
        ground_truth = []
        
        for query in queries:
            # Compute cosine similarity
            similarities = np.dot(vectors, query)
            # Get top-k indices
            top_k_indices = np.argsort(similarities)[-k:][::-1]
            ground_truth.append(top_k_indices.tolist())
        
        return ground_truth
    
    def load_dataset(self, dataset_name: str) -> Optional[VectorDataset]:
        """Load a predefined dataset"""
        if dataset_name not in self.dataset_configs:
            logger.error(f"Unknown dataset: {dataset_name}")
            return None
        
        config = self.dataset_configs[dataset_name]
        
        # Check cache first
        cache_file = self.cache_dir / f"{dataset_name}.npz"
        if cache_file.exists():
            logger.info(f"Loading cached dataset: {dataset_name}")
            return self._load_from_cache(cache_file, config)
        
        # Generate new dataset
        logger.info(f"Generating dataset: {dataset_name}")
        dataset = self._generate_dataset(config)
        
        # Cache the dataset
        self._save_to_cache(dataset, cache_file)
        
        return dataset
    
    def _generate_dataset(self, config: DatasetConfig) -> VectorDataset:
        """Generate a complete dataset"""
        # Generate vectors
        vectors = self.generate_realistic_embeddings(config)
        
        # Generate metadata
        metadata = self.generate_document_metadata(config.size, config)
        
        # Generate queries
        queries = self.generate_query_vectors(vectors, num_queries=min(100, config.size // 10))
        
        # Compute ground truth
        ground_truth = self.compute_ground_truth(queries, vectors)
        
        return VectorDataset(
            vectors=vectors,
            metadata=metadata,
            queries=queries,
            ground_truth=ground_truth,
            config=config
        )
    
    def _save_to_cache(self, dataset: VectorDataset, cache_file: Path):
        """Save dataset to cache"""
        try:
            np.savez_compressed(
                cache_file,
                vectors=dataset.vectors,
                queries=dataset.queries,
                ground_truth=np.array(dataset.ground_truth, dtype=object),
                metadata=json.dumps(dataset.metadata),
                config=json.dumps(dataset.config.__dict__)
            )
            logger.info(f"Dataset cached to {cache_file}")
        except Exception as e:
            logger.error(f"Failed to cache dataset: {e}")
    
    def _load_from_cache(self, cache_file: Path, config: DatasetConfig) -> VectorDataset:
        """Load dataset from cache"""
        try:
            data = np.load(cache_file, allow_pickle=True)
            
            metadata = json.loads(str(data['metadata']))
            ground_truth = data['ground_truth'].tolist()
            
            return VectorDataset(
                vectors=data['vectors'],
                metadata=metadata,
                queries=data['queries'],
                ground_truth=ground_truth,
                config=config
            )
        except Exception as e:
            logger.error(f"Failed to load cached dataset: {e}")
            # Regenerate if cache is corrupted
            return self._generate_dataset(config)
    
    def create_custom_dataset(self, name: str, size: int, dimensions: int, 
                            clusters: int = None, noise_level: float = 0.1) -> VectorDataset:
        """Create a custom dataset with specified parameters"""
        if clusters is None:
            clusters = max(1, size // 100)  # Default: ~100 vectors per cluster
        
        config = DatasetConfig(
            name=name,
            size=size,
            dimensions=dimensions,
            clusters=clusters,
            noise_level=noise_level
        )
        
        return self._generate_dataset(config)
    
    def get_available_datasets(self) -> List[str]:
        """Get list of available predefined datasets"""
        return list(self.dataset_configs.keys())
    
    def get_dataset_info(self, dataset_name: str) -> Optional[Dict]:
        """Get information about a dataset"""
        if dataset_name not in self.dataset_configs:
            return None
        
        config = self.dataset_configs[dataset_name]
        return {
            "name": config.name,
            "size": config.size,
            "dimensions": config.dimensions,
            "clusters": config.clusters,
            "noise_level": config.noise_level,
            "description": self._get_dataset_description(dataset_name)
        }
    
    def _get_dataset_description(self, dataset_name: str) -> str:
        """Get description for a dataset"""
        descriptions = {
            "small_docs": "Small document collection (1K docs, 384D) - Good for quick testing",
            "medium_docs": "Medium document collection (5K docs, 768D) - Balanced performance testing",
            "large_docs": "Large document collection (10K docs, 1536D) - Stress testing",
            "semantic_search": "Semantic search dataset (2K docs, 384D) - Query-focused testing",
            "rag_knowledge": "RAG knowledge base (8K docs, 768D) - Retrieval-augmented generation"
        }
        return descriptions.get(dataset_name, "Custom dataset")
    
    def export_dataset_for_database(self, dataset: VectorDataset, format_type: str = "json") -> Dict:
        """Export dataset in format suitable for database loading"""
        if format_type == "json":
            return {
                "vectors": dataset.vectors.tolist(),
                "metadata": dataset.metadata,
                "queries": dataset.queries.tolist(),
                "ground_truth": dataset.ground_truth,
                "config": dataset.config.__dict__
            }
        elif format_type == "csv":
            # Create DataFrame for CSV export
            df_data = []
            for i, (vector, meta) in enumerate(zip(dataset.vectors, dataset.metadata)):
                row = {"id": i, "vector": vector.tolist()}
                row.update(meta)
                df_data.append(row)
            return pd.DataFrame(df_data)
        else:
            raise ValueError(f"Unsupported format: {format_type}")

class BenchmarkDatasetManager:
    """Manage datasets for competitive benchmarking"""
    
    def __init__(self):
        self.loader = RealDatasetLoader()
        self.current_datasets = {}
    
    def prepare_benchmark_datasets(self) -> Dict[str, VectorDataset]:
        """Prepare all datasets needed for competitive benchmarking"""
        logger.info("Preparing benchmark datasets...")
        
        # Load standard benchmark datasets
        dataset_names = ["small_docs", "medium_docs", "semantic_search"]
        
        for name in dataset_names:
            logger.info(f"Loading dataset: {name}")
            dataset = self.loader.load_dataset(name)
            if dataset:
                self.current_datasets[name] = dataset
                logger.info(f"✅ Loaded {name}: {dataset.config.size} vectors, {dataset.config.dimensions}D")
            else:
                logger.error(f"❌ Failed to load dataset: {name}")
        
        return self.current_datasets
    
    def get_dataset_for_test(self, size_category: str = "medium") -> Optional[VectorDataset]:
        """Get appropriate dataset for a test category"""
        size_mapping = {
            "small": "small_docs",
            "medium": "medium_docs", 
            "large": "large_docs",
            "semantic": "semantic_search",
            "rag": "rag_knowledge"
        }
        
        dataset_name = size_mapping.get(size_category)
        if not dataset_name:
            return None
        
        if dataset_name not in self.current_datasets:
            dataset = self.loader.load_dataset(dataset_name)
            if dataset:
                self.current_datasets[dataset_name] = dataset
        
        return self.current_datasets.get(dataset_name)
    
    def validate_dataset(self, dataset: VectorDataset) -> bool:
        """Validate dataset integrity"""
        try:
            # Check vector dimensions
            if len(dataset.vectors.shape) != 2:
                logger.error("Vectors must be 2D array")
                return False
            
            # Check metadata count
            if len(dataset.metadata) != len(dataset.vectors):
                logger.error("Metadata count doesn't match vector count")
                return False
            
            # Check vector normalization (should be unit vectors)
            norms = np.linalg.norm(dataset.vectors, axis=1)
            if not np.allclose(norms, 1.0, atol=1e-6):
                logger.warning("Vectors are not normalized to unit length")
            
            # Check queries
            if len(dataset.queries) == 0:
                logger.error("No query vectors found")
                return False
            
            # Check ground truth
            if len(dataset.ground_truth) != len(dataset.queries):
                logger.error("Ground truth count doesn't match query count")
                return False
            
            logger.info("✅ Dataset validation passed")
            return True
            
        except Exception as e:
            logger.error(f"Dataset validation failed: {e}")
            return False

def main():
    """Main function for testing dataset loading"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Dataset Loader for VexFS Benchmarking")
    parser.add_argument("--list", action="store_true", help="List available datasets")
    parser.add_argument("--load", type=str, help="Load specific dataset")
    parser.add_argument("--info", type=str, help="Get dataset information")
    parser.add_argument("--prepare-all", action="store_true", help="Prepare all benchmark datasets")
    
    args = parser.parse_args()
    
    loader = RealDatasetLoader()
    
    if args.list:
        print("Available datasets:")
        for name in loader.get_available_datasets():
            info = loader.get_dataset_info(name)
            print(f"  {name}: {info['description']}")
    
    elif args.info:
        info = loader.get_dataset_info(args.info)
        if info:
            print(f"Dataset: {info['name']}")
            print(f"Size: {info['size']} vectors")
            print(f"Dimensions: {info['dimensions']}")
            print(f"Clusters: {info['clusters']}")
            print(f"Description: {info['description']}")
        else:
            print(f"Dataset '{args.info}' not found")
    
    elif args.load:
        dataset = loader.load_dataset(args.load)
        if dataset:
            print(f"✅ Loaded dataset '{args.load}'")
            print(f"Vectors: {dataset.vectors.shape}")
            print(f"Queries: {dataset.queries.shape}")
            print(f"Metadata: {len(dataset.metadata)} items")
        else:
            print(f"❌ Failed to load dataset '{args.load}'")
    
    elif args.prepare_all:
        manager = BenchmarkDatasetManager()
        datasets = manager.prepare_benchmark_datasets()
        print(f"✅ Prepared {len(datasets)} datasets for benchmarking")
        for name, dataset in datasets.items():
            print(f"  {name}: {dataset.config.size} vectors, {dataset.config.dimensions}D")
    
    else:
        print("Use --help for usage information")

if __name__ == "__main__":
    main()