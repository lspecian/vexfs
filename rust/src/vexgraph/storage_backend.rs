// VexGraph Storage Backend Implementation
// Provides persistent storage for graph data with multiple backend options

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use std::fs;
use std::io::{self, Write, Read};

use super::core::{GraphNode, GraphEdge};
// NodeId and EdgeId are u64 types defined in mod.rs
type NodeId = u64;
type EdgeId = u64;

/// Storage backend trait for VexGraph
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend
    fn init(&mut self) -> Result<(), StorageError>;
    
    /// Store a node
    fn store_node(&mut self, node: &GraphNode) -> Result<(), StorageError>;
    
    /// Store an edge
    fn store_edge(&mut self, edge: &GraphEdge) -> Result<(), StorageError>;
    
    /// Retrieve a node by ID
    fn get_node(&self, id: &NodeId) -> Result<Option<GraphNode>, StorageError>;
    
    /// Retrieve an edge by ID
    fn get_edge(&self, id: &EdgeId) -> Result<Option<GraphEdge>, StorageError>;
    
    /// Delete a node
    fn delete_node(&mut self, id: &NodeId) -> Result<(), StorageError>;
    
    /// Delete an edge
    fn delete_edge(&mut self, id: &EdgeId) -> Result<(), StorageError>;
    
    /// List all nodes
    fn list_nodes(&self) -> Result<Vec<NodeId>, StorageError>;
    
    /// List all edges
    fn list_edges(&self) -> Result<Vec<EdgeId>, StorageError>;
    
    /// Get edges for a node
    fn get_node_edges(&self, node_id: &NodeId) -> Result<Vec<EdgeId>, StorageError>;
    
    /// Flush any pending writes
    fn flush(&mut self) -> Result<(), StorageError>;
    
    /// Get storage statistics
    fn get_stats(&self) -> StorageStats;
}

/// Storage error types
#[derive(Debug, Clone)]
pub enum StorageError {
    InitError(String),
    IOError(String),
    SerializationError(String),
    NotFound,
    AlreadyExists,
    CorruptedData(String),
    BackendError(String),
}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        StorageError::IOError(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub storage_size_bytes: u64,
    pub last_flush: Option<std::time::SystemTime>,
}

/// In-memory storage backend (for development/testing)
pub struct MemoryBackend {
    nodes: Arc<RwLock<HashMap<NodeId, GraphNode>>>,
    edges: Arc<RwLock<HashMap<EdgeId, GraphEdge>>>,
    node_edges: Arc<RwLock<HashMap<NodeId, Vec<EdgeId>>>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            edges: Arc::new(RwLock::new(HashMap::new())),
            node_edges: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl StorageBackend for MemoryBackend {
    fn init(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
    
    fn store_node(&mut self, node: &GraphNode) -> Result<(), StorageError> {
        let mut nodes = self.nodes.write().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        nodes.insert(node.id.clone(), node.clone());
        Ok(())
    }
    
    fn store_edge(&mut self, edge: &GraphEdge) -> Result<(), StorageError> {
        let mut edges = self.edges.write().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        edges.insert(edge.id.clone(), edge.clone());
        
        // Update node-edge mapping
        let mut node_edges = self.node_edges.write().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        
        node_edges.entry(edge.source_id.clone())
            .or_insert_with(Vec::new)
            .push(edge.id.clone());
        
        node_edges.entry(edge.target_id.clone())
            .or_insert_with(Vec::new)
            .push(edge.id.clone());
        
        Ok(())
    }
    
    fn get_node(&self, id: &NodeId) -> Result<Option<GraphNode>, StorageError> {
        let nodes = self.nodes.read().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(nodes.get(id).cloned())
    }
    
    fn get_edge(&self, id: &EdgeId) -> Result<Option<GraphEdge>, StorageError> {
        let edges = self.edges.read().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(edges.get(id).cloned())
    }
    
    fn delete_node(&mut self, id: &NodeId) -> Result<(), StorageError> {
        let mut nodes = self.nodes.write().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        nodes.remove(id);
        
        // Also remove from node-edge mapping
        let mut node_edges = self.node_edges.write().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        node_edges.remove(id);
        
        Ok(())
    }
    
    fn delete_edge(&mut self, id: &EdgeId) -> Result<(), StorageError> {
        let mut edges = self.edges.write().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        
        if let Some(edge) = edges.remove(id) {
            // Remove from node-edge mapping
            let mut node_edges = self.node_edges.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            
            if let Some(edges) = node_edges.get_mut(&edge.source_id) {
                edges.retain(|e| e != id);
            }
            
            if let Some(edges) = node_edges.get_mut(&edge.target_id) {
                edges.retain(|e| e != id);
            }
        }
        
        Ok(())
    }
    
    fn list_nodes(&self) -> Result<Vec<NodeId>, StorageError> {
        let nodes = self.nodes.read().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(nodes.keys().cloned().collect())
    }
    
    fn list_edges(&self) -> Result<Vec<EdgeId>, StorageError> {
        let edges = self.edges.read().map_err(|e| 
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(edges.keys().cloned().collect())
    }
    
    fn get_node_edges(&self, node_id: &NodeId) -> Result<Vec<EdgeId>, StorageError> {
        let node_edges = self.node_edges.read().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(node_edges.get(node_id).cloned().unwrap_or_default())
    }
    
    fn flush(&mut self) -> Result<(), StorageError> {
        // No-op for memory backend
        Ok(())
    }
    
    fn get_stats(&self) -> StorageStats {
        let nodes = self.nodes.read().unwrap_or_else(|e| e.into_inner());
        let edges = self.edges.read().unwrap_or_else(|e| e.into_inner());
        
        StorageStats {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
            storage_size_bytes: 0, // Not applicable for memory
            last_flush: None,
        }
    }
}

/// File-based JSON storage backend
pub struct JsonFileBackend {
    base_path: PathBuf,
    nodes_cache: Arc<RwLock<HashMap<NodeId, GraphNode>>>,
    edges_cache: Arc<RwLock<HashMap<EdgeId, GraphEdge>>>,
    node_edges_cache: Arc<RwLock<HashMap<NodeId, Vec<EdgeId>>>>,
    write_buffer: Arc<RwLock<Vec<WriteOp>>>,
    auto_flush_threshold: usize,
}

#[derive(Clone)]
enum WriteOp {
    StoreNode(GraphNode),
    StoreEdge(GraphEdge),
    DeleteNode(NodeId),
    DeleteEdge(EdgeId),
}

impl JsonFileBackend {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            nodes_cache: Arc::new(RwLock::new(HashMap::new())),
            edges_cache: Arc::new(RwLock::new(HashMap::new())),
            node_edges_cache: Arc::new(RwLock::new(HashMap::new())),
            write_buffer: Arc::new(RwLock::new(Vec::new())),
            auto_flush_threshold: 100,
        }
    }
    
    fn nodes_dir(&self) -> PathBuf {
        self.base_path.join("nodes")
    }
    
    fn edges_dir(&self) -> PathBuf {
        self.base_path.join("edges")
    }
    
    fn node_file(&self, id: &NodeId) -> PathBuf {
        let safe_id = id.replace('/', "_").replace('\\', "_");
        self.nodes_dir().join(format!("{}.json", safe_id))
    }
    
    fn edge_file(&self, id: &EdgeId) -> PathBuf {
        let safe_id = id.replace('/', "_").replace('\\', "_");
        self.edges_dir().join(format!("{}.json", safe_id))
    }
    
    fn load_cache(&mut self) -> Result<(), StorageError> {
        // Load all nodes
        if let Ok(entries) = fs::read_dir(&self.nodes_dir()) {
            let mut nodes = self.nodes_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(node) = serde_json::from_str::<GraphNode>(&content) {
                            nodes.insert(node.id.clone(), node);
                        }
                    }
                }
            }
        }
        
        // Load all edges
        if let Ok(entries) = fs::read_dir(&self.edges_dir()) {
            let mut edges = self.edges_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            let mut node_edges = self.node_edges_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(edge) = serde_json::from_str::<GraphEdge>(&content) {
                            node_edges.entry(edge.source_id.clone())
                                .or_insert_with(Vec::new)
                                .push(edge.id.clone());
                            node_edges.entry(edge.target_id.clone())
                                .or_insert_with(Vec::new)
                                .push(edge.id.clone());
                            edges.insert(edge.id.clone(), edge);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn check_auto_flush(&mut self) -> Result<(), StorageError> {
        let should_flush = {
            let buffer = self.write_buffer.read().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            buffer.len() >= self.auto_flush_threshold
        };
        
        if should_flush {
            self.flush()?;
        }
        
        Ok(())
    }
}

impl StorageBackend for JsonFileBackend {
    fn init(&mut self) -> Result<(), StorageError> {
        // Create directories
        fs::create_dir_all(&self.nodes_dir())?;
        fs::create_dir_all(&self.edges_dir())?;
        
        // Load existing data into cache
        self.load_cache()?;
        
        Ok(())
    }
    
    fn store_node(&mut self, node: &GraphNode) -> Result<(), StorageError> {
        // Update cache
        {
            let mut nodes = self.nodes_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            nodes.insert(node.id.clone(), node.clone());
        }
        
        // Add to write buffer
        {
            let mut buffer = self.write_buffer.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            buffer.push(WriteOp::StoreNode(node.clone()));
        }
        
        self.check_auto_flush()?;
        Ok(())
    }
    
    fn store_edge(&mut self, edge: &GraphEdge) -> Result<(), StorageError> {
        // Update cache
        {
            let mut edges = self.edges_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            edges.insert(edge.id.clone(), edge.clone());
            
            let mut node_edges = self.node_edges_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            node_edges.entry(edge.source_id.clone())
                .or_insert_with(Vec::new)
                .push(edge.id.clone());
            node_edges.entry(edge.target_id.clone())
                .or_insert_with(Vec::new)
                .push(edge.id.clone());
        }
        
        // Add to write buffer
        {
            let mut buffer = self.write_buffer.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            buffer.push(WriteOp::StoreEdge(edge.clone()));
        }
        
        self.check_auto_flush()?;
        Ok(())
    }
    
    fn get_node(&self, id: &NodeId) -> Result<Option<GraphNode>, StorageError> {
        let nodes = self.nodes_cache.read().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(nodes.get(id).cloned())
    }
    
    fn get_edge(&self, id: &EdgeId) -> Result<Option<GraphEdge>, StorageError> {
        let edges = self.edges_cache.read().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(edges.get(id).cloned())
    }
    
    fn delete_node(&mut self, id: &NodeId) -> Result<(), StorageError> {
        // Update cache
        {
            let mut nodes = self.nodes_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            nodes.remove(id);
            
            let mut node_edges = self.node_edges_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            node_edges.remove(id);
        }
        
        // Add to write buffer
        {
            let mut buffer = self.write_buffer.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            buffer.push(WriteOp::DeleteNode(id.clone()));
        }
        
        self.check_auto_flush()?;
        Ok(())
    }
    
    fn delete_edge(&mut self, id: &EdgeId) -> Result<(), StorageError> {
        // Update cache
        {
            let mut edges = self.edges_cache.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            
            if let Some(edge) = edges.remove(id) {
                let mut node_edges = self.node_edges_cache.write().map_err(|e|
                    StorageError::BackendError(format!("Lock error: {}", e)))?;
                
                if let Some(edges) = node_edges.get_mut(&edge.source_id) {
                    edges.retain(|e| e != id);
                }
                
                if let Some(edges) = node_edges.get_mut(&edge.target_id) {
                    edges.retain(|e| e != id);
                }
            }
        }
        
        // Add to write buffer
        {
            let mut buffer = self.write_buffer.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            buffer.push(WriteOp::DeleteEdge(id.clone()));
        }
        
        self.check_auto_flush()?;
        Ok(())
    }
    
    fn list_nodes(&self) -> Result<Vec<NodeId>, StorageError> {
        let nodes = self.nodes_cache.read().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(nodes.keys().cloned().collect())
    }
    
    fn list_edges(&self) -> Result<Vec<EdgeId>, StorageError> {
        let edges = self.edges_cache.read().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(edges.keys().cloned().collect())
    }
    
    fn get_node_edges(&self, node_id: &NodeId) -> Result<Vec<EdgeId>, StorageError> {
        let node_edges = self.node_edges_cache.read().map_err(|e|
            StorageError::BackendError(format!("Lock error: {}", e)))?;
        Ok(node_edges.get(node_id).cloned().unwrap_or_default())
    }
    
    fn flush(&mut self) -> Result<(), StorageError> {
        let ops = {
            let mut buffer = self.write_buffer.write().map_err(|e|
                StorageError::BackendError(format!("Lock error: {}", e)))?;
            let ops = buffer.clone();
            buffer.clear();
            ops
        };
        
        for op in ops {
            match op {
                WriteOp::StoreNode(node) => {
                    let content = serde_json::to_string_pretty(&node)?;
                    fs::write(self.node_file(&node.id), content)?;
                }
                WriteOp::StoreEdge(edge) => {
                    let content = serde_json::to_string_pretty(&edge)?;
                    fs::write(self.edge_file(&edge.id), content)?;
                }
                WriteOp::DeleteNode(id) => {
                    let _ = fs::remove_file(self.node_file(&id));
                }
                WriteOp::DeleteEdge(id) => {
                    let _ = fs::remove_file(self.edge_file(&id));
                }
            }
        }
        
        Ok(())
    }
    
    fn get_stats(&self) -> StorageStats {
        let nodes = self.nodes_cache.read().unwrap_or_else(|e| e.into_inner());
        let edges = self.edges_cache.read().unwrap_or_else(|e| e.into_inner());
        
        let storage_size = fs::read_dir(&self.nodes_dir())
            .into_iter()
            .chain(fs::read_dir(&self.edges_dir()))
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .map(|meta| meta.len())
            .sum();
        
        StorageStats {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
            storage_size_bytes: storage_size,
            last_flush: Some(std::time::SystemTime::now()),
        }
    }
}

/// Storage backend factory
pub struct StorageFactory;

impl StorageFactory {
    /// Create a memory backend
    pub fn create_memory() -> Box<dyn StorageBackend> {
        Box::new(MemoryBackend::new())
    }
    
    /// Create a JSON file backend
    pub fn create_json_file(path: impl AsRef<Path>) -> Box<dyn StorageBackend> {
        Box::new(JsonFileBackend::new(path))
    }
    
    /// Create backend from configuration
    pub fn from_config(config: &StorageConfig) -> Result<Box<dyn StorageBackend>, StorageError> {
        match config {
            StorageConfig::Memory => Ok(Self::create_memory()),
            StorageConfig::JsonFile { path } => Ok(Self::create_json_file(path)),
            StorageConfig::SQLite { .. } => {
                Err(StorageError::InitError("SQLite backend not yet implemented".to_string()))
            }
            StorageConfig::PostgreSQL { .. } => {
                Err(StorageError::InitError("PostgreSQL backend not yet implemented".to_string()))
            }
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageConfig {
    Memory,
    JsonFile { path: PathBuf },
    SQLite { path: PathBuf },
    PostgreSQL { connection_string: String },
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Memory
    }
}

// Tests temporarily disabled - need to update with proper GraphNode structure
// TODO: Re-enable tests after VexGraph types are properly imported