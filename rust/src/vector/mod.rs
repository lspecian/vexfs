//! Vector Module
//!
//! This module contains all vector-related functionality for VexFS, including:
//! - Vector storage and management
//! - HNSW and KNN search algorithms
//! - Vector metrics and optimizations
//! - Vector caching and large collection handling

// Core vector modules
pub mod vector_storage;
pub mod vector_storage_optimized;
pub mod vector_metrics;
pub mod vector_cache;

// Search and indexing
pub mod knn_search;
pub mod vector_search;
pub mod vector_search_integration;

// Optimizations
pub mod vector_optimizations;
pub mod vector_large_collections;

// Vector handlers
pub mod vector_handlers;
pub mod vector_handlers_stub;

#[cfg(feature = "kernel")]
pub mod vector_handlers_kernel;

#[cfg(feature = "kernel")]
pub mod vector_search_integration_kernel;

// Re-exports for convenience
pub use vector_storage::{VectorStorageManager, VectorDataType, VectorLocation, VectorHeader};
pub use vector_storage_optimized::{OptimizedVectorStorageManager, MemoryConfig};
pub use vector_metrics::{VectorMetrics, DistanceMetric, calculate_distance};
// Note: These traits/structs not yet implemented in their modules
// pub use vector_cache::VectorCache;
// pub use knn_search::KnnSearch;
// pub use vector_search::VectorSearch;
