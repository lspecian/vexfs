//! Approximate Nearest Neighbor Search (ANNS) Module
//!
//! This module provides comprehensive ANNS capabilities for VexFS, including
//! HNSW graphs, advanced indexing strategies, memory optimization, persistence,
//! and crash recovery mechanisms.

pub mod hnsw;
pub mod hnsw_optimized;
pub mod stack_safety_monitor;
pub mod indexing;
pub mod advanced_indexing;
pub mod advanced_strategies;
pub mod memory_mgmt;
pub mod memory_optimization;
pub mod serialization;
pub mod wal;
pub mod persistence;
pub mod recovery;
pub mod performance_validation;
pub mod realistic_benchmark;
pub mod integration;

// Re-export key types and functions
pub use hnsw::*;
pub use stack_safety_monitor::*;
pub use indexing::*;
pub use advanced_indexing::*;
pub use advanced_strategies::*;
pub use memory_mgmt::*;
pub use memory_optimization::*;
pub use serialization::*;
pub use wal::*;
pub use persistence::*;
pub use recovery::*;
pub use performance_validation::*;
pub use realistic_benchmark::*;
pub use integration::*;