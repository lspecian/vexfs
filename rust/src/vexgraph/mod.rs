/*
 * VexFS v2.0 - VexGraph Native Graph Representation and API
 * 
 * This module implements the VexGraph Native Graph Representation and API
 * that builds upon the VexGraph foundation and the Full Filesystem Journal
 * to create sophisticated graph capabilities with native vector integration.
 *
 * VexGraph Implementation Features:
 * - Extended VexGraph core structure with inode-based node IDs
 * - Efficient graph traversal algorithms (BFS, DFS, Dijkstra, topological sort)
 * - Enhanced Property Graph Model with named directed relationships
 * - Semantic search integration with VexFS vector capabilities
 * - RESTful API server with complete CRUD operations
 * - Kernel-level graph primitives with ioctl interfaces
 * - FUSE client extensions with graph-aware operations
 * - Performance optimization with indexing and query planning
 * - Thread-safe concurrent operations with transaction support
 * - Comprehensive error handling and validation
 */

pub mod core;
pub mod traversal;
pub mod property_graph;
pub mod semantic_integration;
pub mod api_server;
pub mod kernel_primitives;
pub mod fuse_extensions;
pub mod performance;
pub mod concurrency;
pub mod error_handling;

// Semantic Search Integration (Task 11)
pub mod semantic_search;
pub mod semantic_search_manager;
pub mod semantic_query_language;
pub mod semantic_query_executor;
pub mod semantic_plugin_system;
pub mod vexserver_integration;

// Advanced Graph Algorithms and Semantic Reasoning (Task 20)
pub mod advanced_algorithms;
pub mod semantic_reasoning;

// Re-export main types and functions
pub use core::*;
pub use traversal::*;
pub use property_graph::*;
pub use semantic_integration::*;
pub use api_server::*;
pub use kernel_primitives::*;
pub use fuse_extensions::*;
pub use performance::*;
pub use concurrency::*;
pub use error_handling::*;

// Semantic Search Integration (Task 11)
pub use semantic_search::*;
pub use semantic_search_manager::*;
pub use semantic_query_language::*;
pub use semantic_query_executor::*;
pub use semantic_plugin_system::*;
pub use vexserver_integration::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;

/// VexGraph Phase 2 version information
pub const vexgraph_VERSION_MAJOR: u32 = 2;
pub const vexgraph_VERSION_MINOR: u32 = 0;
pub const vexgraph_VERSION_PATCH: u32 = 0;

/// Maximum values for VexGraph Phase 2
pub const MAX_GRAPH_NODES: usize = 10_000_000;
pub const MAX_GRAPH_EDGES: usize = 100_000_000;
pub const MAX_PROPERTIES_PER_NODE: usize = 1024;
pub const MAX_PROPERTY_VALUE_SIZE: usize = 65536;
pub const MAX_TRAVERSAL_DEPTH: u32 = 1000;
pub const MAX_CONCURRENT_QUERIES: usize = 1000;

/// Node types for VexGraph Phase 2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeType {
    File = 0x01,
    Directory = 0x02,
    Vector = 0x03,
    Collection = 0x04,
    Semantic = 0x05,
    Custom = 0x06,
}

/// Edge types for VexGraph Phase 2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EdgeType {
    Contains = 0x01,
    References = 0x02,
    Similar = 0x03,
    Semantic = 0x04,
    Temporal = 0x05,
    Custom = 0x06,
    Dependency = 0x07,
    Hierarchy = 0x08,
}

/// Property types for VexGraph Phase 2
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyType {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Vector(Vec<f32>),
    Timestamp(chrono::DateTime<chrono::Utc>),
    Json(serde_json::Value),
    Binary(Vec<u8>),
}

/// Graph traversal algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TraversalAlgorithm {
    BreadthFirstSearch = 0x01,
    DepthFirstSearch = 0x02,
    Dijkstra = 0x03,
    AStar = 0x04,
    TopologicalSort = 0x05,
    PageRank = 0x06,
}

/// Query operators for graph queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum QueryOperator {
    Equals = 0x01,
    NotEquals = 0x02,
    Greater = 0x03,
    Less = 0x04,
    GreaterEqual = 0x05,
    LessEqual = 0x06,
    Contains = 0x07,
    StartsWith = 0x08,
    EndsWith = 0x09,
    Regex = 0x0A,
    VectorSimilar = 0x0B,
}

/// Graph node identifier (using inode numbers as specified)
pub type NodeId = u64;

/// Graph edge identifier
pub type EdgeId = u64;

/// Graph transaction identifier
pub type TransactionId = Uuid;

/// VexGraph main structure
#[derive(Debug)]
pub struct VexGraph {
    /// Core graph manager
    pub core: Arc<core::VexGraphCore>,
    
    /// Traversal engine
    pub traversal: Arc<traversal::TraversalEngine>,
    
    /// Property graph manager
    pub property_graph: Arc<property_graph::PropertyGraphManager>,
    
    /// Semantic integration layer
    pub semantic: Arc<semantic_integration::SemanticIntegration>,
    
    /// API server
    pub api_server: Option<Arc<api_server::VexGraphApiServer>>,
    
    /// Kernel primitives interface
    pub kernel_primitives: Arc<kernel_primitives::KernelPrimitives>,
    
    /// FUSE extensions
    pub fuse_extensions: Arc<fuse_extensions::FuseExtensions>,
    
    /// Performance optimizer
    pub performance: Arc<performance::PerformanceOptimizer>,
    
    /// Concurrency manager
    pub concurrency: Arc<concurrency::ConcurrencyManager>,
    
    /// Advanced graph algorithms engine
    pub advanced_algorithms: Arc<advanced_algorithms::AdvancedGraphAlgorithms>,
    
    /// Semantic reasoning engine
    pub semantic_reasoning: Arc<semantic_reasoning::SemanticReasoningEngine>,
    
    /// Configuration
    pub config: VexGraphConfig,
}

/// VexGraph Phase 2 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VexGraphConfig {
    /// Enable kernel integration
    pub kernel_integration: bool,
    
    /// Enable FUSE extensions
    pub fuse_extensions: bool,
    
    /// Enable API server
    pub api_server: bool,
    
    /// API server bind address
    pub api_bind_address: String,
    
    /// API server port
    pub api_port: u16,
    
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,
    
    /// Query timeout in seconds
    pub query_timeout_seconds: u64,
    
    /// Enable performance optimization
    pub performance_optimization: bool,
    
    /// Enable semantic integration
    pub semantic_integration: bool,
    
    /// Vector similarity threshold
    pub vector_similarity_threshold: f32,
    
    /// Cache size for graph operations
    pub cache_size: usize,
    
    /// Enable transaction support
    pub transaction_support: bool,
    
    /// Journal integration
    pub journal_integration: bool,
}

impl Default for VexGraphConfig {
    fn default() -> Self {
        Self {
            kernel_integration: true,
            fuse_extensions: true,
            api_server: true,
            api_bind_address: "127.0.0.1".to_string(),
            api_port: 8080,
            max_concurrent_queries: MAX_CONCURRENT_QUERIES,
            query_timeout_seconds: 30,
            performance_optimization: true,
            semantic_integration: true,
            vector_similarity_threshold: 0.8,
            cache_size: 10000,
            transaction_support: true,
            journal_integration: true,
        }
    }
}

impl VexGraph {
    /// Create a new VexGraph Phase 2 instance
    pub async fn new(config: VexGraphConfig) -> Result<Self, VexGraphError> {
        // Initialize core components
        let core = Arc::new(core::VexGraphCore::new(&config).await?);
        let traversal = Arc::new(traversal::TraversalEngine::new(core.clone()).await?);
        let property_graph = Arc::new(property_graph::PropertyGraphManager::new(core.clone()).await?);
        let semantic = Arc::new(semantic_integration::SemanticIntegration::new(core.clone()).await?);
        let kernel_primitives = Arc::new(kernel_primitives::KernelPrimitives::new(&config).await?);
        let fuse_extensions = Arc::new(fuse_extensions::FuseExtensions::new(core.clone(), config.clone()).await?);
        let performance = Arc::new(performance::PerformanceOptimizer::new(core.clone(), config.clone()).await?);
        let concurrency = Arc::new(concurrency::ConcurrencyManager::new(core.clone(), config.clone()).await?);
        
        // Initialize advanced algorithms and reasoning engines
        let advanced_algorithms = Arc::new(<advanced_algorithms::AdvancedGraphAlgorithms>::new(core.clone(), config.clone()).await?);
        let semantic_reasoning = Arc::new(semantic_reasoning::SemanticReasoningEngine::new(core.clone(), config.clone()).await?);
        
        // Initialize API server if enabled
        let api_server = if config.api_server {
            Some(Arc::new(api_server::VexGraphApiServer::new(
                core.clone(),
                traversal.clone(),
                property_graph.clone(),
                semantic.clone(),
                &config,
            ).await?))
        } else {
            None
        };
        
        Ok(Self {
            core,
            traversal,
            property_graph,
            semantic,
            api_server,
            kernel_primitives,
            fuse_extensions,
            performance,
            concurrency,
            advanced_algorithms,
            semantic_reasoning,
            config,
        })
    }
    
    /// Start the VexGraph Phase 2 system
    pub async fn start(&self) -> Result<(), VexGraphError> {
        tracing::info!("Starting VexGraph Phase 2 system");
        
        // Start core components
        self.core.start().await?;
        self.traversal.start().await?;
        self.property_graph.start().await?;
        self.semantic.start().await?;
        self.kernel_primitives.start().await?;
        self.fuse_extensions.start().await?;
        self.performance.start().await?;
        self.concurrency.start().await?;
        advanced_algorithms::AdvancedGraphAlgorithms::start(&*self.advanced_algorithms).await?;
        self.semantic_reasoning.start().await?;
        
        // Start API server if enabled
        if let Some(api_server) = &self.api_server {
            api_server.start().await?;
        }
        
        tracing::info!("VexGraph Phase 2 system started successfully");
        Ok(())
    }
    
    /// Stop the VexGraph Phase 2 system
    pub async fn stop(&self) -> Result<(), VexGraphError> {
        tracing::info!("Stopping VexGraph Phase 2 system");
        
        // Stop API server first
        if let Some(api_server) = &self.api_server {
            api_server.stop().await?;
        }
        
        // Stop components in reverse order
        self.semantic_reasoning.stop().await?;
        advanced_algorithms::AdvancedGraphAlgorithms::stop(&*self.advanced_algorithms).await?;
        self.concurrency.stop().await?;
        self.performance.stop().await?;
        self.fuse_extensions.stop().await?;
        self.kernel_primitives.stop().await?;
        self.semantic.stop().await?;
        self.property_graph.stop().await?;
        self.traversal.stop().await?;
        self.core.stop().await?;
        
        tracing::info!("VexGraph system stopped successfully");
        Ok(())
    }
    
    /// Get system statistics
    pub async fn get_statistics(&self) -> Result<VexGraphStatistics, VexGraphError> {
        let core_stats = self.core.get_statistics().await?;
        let traversal_stats = self.traversal.get_statistics().await?;
        let property_stats = self.property_graph.get_statistics().await?;
        let semantic_stats = self.semantic.get_statistics().await?;
        let performance_stats = self.performance.get_statistics().await?;
        let concurrency_stats = self.concurrency.get_statistics().await?;
        
        Ok(VexGraphStatistics {
            core: core_stats,
            traversal: traversal_stats,
            property_graph: property_stats,
            semantic: semantic_stats,
            performance: performance_stats,
            concurrency: concurrency_stats,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// VexGraph statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct VexGraphStatistics {
    pub core: core::CoreStatistics,
    pub traversal: traversal::TraversalStatistics,
    pub property_graph: property_graph::PropertyGraphStatistics,
    pub semantic: semantic_integration::SemanticStatistics,
    pub performance: performance::PerformanceStatistics,
    pub concurrency: concurrency::ConcurrencyStatistics,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_vexgraph_creation() {
        let config = VexGraphConfig::default();
        let vexgraph = VexGraph::new(config).await;
        assert!(vexgraph.is_ok());
    }
    
    #[tokio::test]
    async fn test_vexgraph_lifecycle() {
        let config = VexGraphConfig {
            api_server: false, // Disable API server for testing
            ..Default::default()
        };
        
        let vexgraph = VexGraph::new(config).await.unwrap();
        
        // Test start
        assert!(vexgraph.start().await.is_ok());
        
        // Test statistics
        assert!(vexgraph.get_statistics().await.is_ok());
        
        // Test stop
        assert!(vexgraph.stop().await.is_ok());
    }
}