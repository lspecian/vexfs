/*
 * VexFS v2.0 - VexGraph Phase 2 Core Implementation
 * 
 * This module implements the core VexGraph functionality that extends the existing
 * kernel VexGraph foundation (Tasks 8-10) with advanced graph capabilities.
 */

use crate::vexgraph::{
    NodeId, EdgeId, NodeType, EdgeType, PropertyType, VexGraphConfig,
    error_handling::{VexGraphError, VexGraphResult, Validator},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use uuid::Uuid;

/// Graph node representation with vector embedding support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub inode_number: u64,
    pub node_type: NodeType,
    pub properties: HashMap<String, PropertyType>,
    pub outgoing_edges: Vec<EdgeId>,
    pub incoming_edges: Vec<EdgeId>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    
    /// Vector embeddings associated with this node (Task 11)
    #[cfg(feature = "semantic_search")]
    pub embeddings: Vec<uuid::Uuid>,
    
    /// Fallback for when semantic search is not enabled
    #[cfg(not(feature = "semantic_search"))]
    pub embeddings: Vec<String>, // Store as string IDs
}

impl GraphNode {
    pub fn new(id: NodeId, inode_number: u64, node_type: NodeType) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            inode_number,
            node_type,
            properties: HashMap::new(),
            outgoing_edges: Vec::new(),
            incoming_edges: Vec::new(),
            created_at: now,
            updated_at: now,
            embeddings: Vec::new(),
        }
    }
    
    pub fn add_property(&mut self, key: String, value: PropertyType) -> VexGraphResult<()> {
        Validator::validate_property_key(&key)?;
        self.properties.insert(key, value);
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
    
    pub fn remove_property(&mut self, key: &str) -> Option<PropertyType> {
        self.updated_at = chrono::Utc::now();
        self.properties.remove(key)
    }
    
    pub fn get_property(&self, key: &str) -> Option<&PropertyType> {
        self.properties.get(key)
    }
    
    pub fn add_outgoing_edge(&mut self, edge_id: EdgeId) {
        if !self.outgoing_edges.contains(&edge_id) {
            self.outgoing_edges.push(edge_id);
            self.updated_at = chrono::Utc::now();
        }
    }
    
    pub fn add_incoming_edge(&mut self, edge_id: EdgeId) {
        if !self.incoming_edges.contains(&edge_id) {
            self.incoming_edges.push(edge_id);
            self.updated_at = chrono::Utc::now();
        }
    }
    
    pub fn remove_outgoing_edge(&mut self, edge_id: EdgeId) {
        self.outgoing_edges.retain(|&id| id != edge_id);
        self.updated_at = chrono::Utc::now();
    }
    
    pub fn remove_incoming_edge(&mut self, edge_id: EdgeId) {
        self.incoming_edges.retain(|&id| id != edge_id);
        self.updated_at = chrono::Utc::now();
    }
    
    /// Add a vector embedding to this node (Task 11)
    #[cfg(feature = "semantic_search")]
    pub fn add_embedding(&mut self, embedding_id: uuid::Uuid) {
        if !self.embeddings.contains(&embedding_id) {
            self.embeddings.push(embedding_id);
            self.updated_at = chrono::Utc::now();
        }
    }
    
    /// Remove a vector embedding from this node (Task 11)
    #[cfg(feature = "semantic_search")]
    pub fn remove_embedding(&mut self, embedding_id: uuid::Uuid) {
        self.embeddings.retain(|&id| id != embedding_id);
        self.updated_at = chrono::Utc::now();
    }
    
    /// Get all embedding IDs for this node (Task 11)
    #[cfg(feature = "semantic_search")]
    pub fn get_embeddings(&self) -> &[uuid::Uuid] {
        &self.embeddings
    }
    
    /// Check if node has any embeddings (Task 11)
    #[cfg(feature = "semantic_search")]
    pub fn has_embeddings(&self) -> bool {
        !self.embeddings.is_empty()
    }
    
    /// Fallback methods for when semantic search is not enabled
    #[cfg(not(feature = "semantic_search"))]
    pub fn add_embedding(&mut self, embedding_id: String) {
        if !self.embeddings.contains(&embedding_id) {
            self.embeddings.push(embedding_id);
            self.updated_at = chrono::Utc::now();
        }
    }
    
    #[cfg(not(feature = "semantic_search"))]
    pub fn remove_embedding(&mut self, embedding_id: &str) {
        self.embeddings.retain(|id| id != embedding_id);
        self.updated_at = chrono::Utc::now();
    }
    
    #[cfg(not(feature = "semantic_search"))]
    pub fn get_embeddings(&self) -> &[String] {
        &self.embeddings
    }
    
    #[cfg(not(feature = "semantic_search"))]
    pub fn has_embeddings(&self) -> bool {
        !self.embeddings.is_empty()
    }
}

/// Graph edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: EdgeId,
    pub source_id: NodeId,
    pub target_id: NodeId,
    pub edge_type: EdgeType,
    pub weight: f64,
    pub properties: HashMap<String, PropertyType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl GraphEdge {
    pub fn new(
        id: EdgeId,
        source_id: NodeId,
        target_id: NodeId,
        edge_type: EdgeType,
        weight: f64,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            source_id,
            target_id,
            edge_type,
            weight,
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn add_property(&mut self, key: String, value: PropertyType) -> VexGraphResult<()> {
        Validator::validate_property_key(&key)?;
        self.properties.insert(key, value);
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
    
    pub fn remove_property(&mut self, key: &str) -> Option<PropertyType> {
        self.updated_at = chrono::Utc::now();
        self.properties.remove(key)
    }
    
    pub fn get_property(&self, key: &str) -> Option<&PropertyType> {
        self.properties.get(key)
    }
}

/// Core statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreStatistics {
    pub node_count: u64,
    pub edge_count: u64,
    pub property_count: u64,
    pub memory_usage: u64,
    pub cache_hit_rate: f64,
    pub operations_per_second: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// VexGraph core implementation
#[derive(Debug)]
pub struct VexGraphCore {
    /// Node storage using DashMap for concurrent access
    nodes: DashMap<NodeId, GraphNode>,
    
    /// Edge storage using DashMap for concurrent access
    edges: DashMap<EdgeId, GraphEdge>,
    
    /// Inode to node mapping for filesystem integration
    inode_to_node: DashMap<u64, NodeId>,
    
    /// Node type indices for efficient queries
    node_type_index: DashMap<NodeType, Vec<NodeId>>,
    
    /// Edge type indices for efficient queries
    edge_type_index: DashMap<EdgeType, Vec<EdgeId>>,
    
    /// Property indices for efficient property-based queries
    property_index: DashMap<String, Vec<NodeId>>,
    
    /// Next node ID counter
    next_node_id: parking_lot::Mutex<NodeId>,
    
    /// Next edge ID counter
    next_edge_id: parking_lot::Mutex<EdgeId>,
    
    /// Configuration
    config: VexGraphConfig,
    
    /// Statistics
    stats: RwLock<CoreStatistics>,
    
    /// Kernel interface handle (placeholder for FFI integration)
    kernel_handle: Option<*mut std::ffi::c_void>,
}

unsafe impl Send for VexGraphCore {}
unsafe impl Sync for VexGraphCore {}

impl VexGraphCore {
    /// Create a new VexGraph core instance
    pub async fn new(config: &VexGraphConfig) -> VexGraphResult<Self> {
        let stats = CoreStatistics {
            node_count: 0,
            edge_count: 0,
            property_count: 0,
            memory_usage: 0,
            cache_hit_rate: 0.0,
            operations_per_second: 0.0,
            last_updated: chrono::Utc::now(),
        };
        
        Ok(Self {
            nodes: DashMap::new(),
            edges: DashMap::new(),
            inode_to_node: DashMap::new(),
            node_type_index: DashMap::new(),
            edge_type_index: DashMap::new(),
            property_index: DashMap::new(),
            next_node_id: parking_lot::Mutex::new(1),
            next_edge_id: parking_lot::Mutex::new(1),
            config: config.clone(),
            stats: RwLock::new(stats),
            kernel_handle: None,
        })
    }
    
    /// Start the core system
    pub async fn start(&self) -> VexGraphResult<()> {
        tracing::info!("Starting VexGraph core");
        
        // Initialize kernel interface if enabled
        if self.config.kernel_integration {
            self.initialize_kernel_interface().await?;
        }
        
        tracing::info!("VexGraph core started successfully");
        Ok(())
    }
    
    /// Stop the core system
    pub async fn stop(&self) -> VexGraphResult<()> {
        tracing::info!("Stopping VexGraph core");
        
        // Cleanup kernel interface if initialized
        if self.kernel_handle.is_some() {
            self.cleanup_kernel_interface().await?;
        }
        
        tracing::info!("VexGraph core stopped successfully");
        Ok(())
    }
    
    /// Create a new node
    pub async fn create_node(
        &self,
        inode_number: u64,
        node_type: NodeType,
    ) -> VexGraphResult<NodeId> {
        // Generate new node ID
        let node_id = {
            let mut next_id = self.next_node_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        Validator::validate_node_id(node_id)?;
        
        // Check if inode already has a node
        if self.inode_to_node.contains_key(&inode_number) {
            return Err(VexGraphError::NodeAlreadyExists(
                format!("Node for inode {} already exists", inode_number)
            ));
        }
        
        // Create the node
        let node = GraphNode::new(node_id, inode_number, node_type);
        
        // Store the node
        self.nodes.insert(node_id, node);
        self.inode_to_node.insert(inode_number, node_id);
        
        // Update indices
        self.node_type_index
            .entry(node_type)
            .or_insert_with(Vec::new)
            .push(node_id);
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.node_count += 1;
            stats.last_updated = chrono::Utc::now();
        }
        
        // Notify kernel if integration is enabled
        if self.config.kernel_integration {
            self.notify_kernel_node_created(node_id, inode_number, node_type).await?;
        }
        
        tracing::debug!("Created node {} for inode {}", node_id, inode_number);
        Ok(node_id)
    }
    
    /// Get a node by ID
    pub async fn get_node(&self, node_id: NodeId) -> VexGraphResult<GraphNode> {
        Validator::validate_node_id(node_id)?;
        
        self.nodes
            .get(&node_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| VexGraphError::NodeNotFound(format!("Node {} not found", node_id)))
    }
    
    /// Get a node by inode number
    pub async fn get_node_by_inode(&self, inode_number: u64) -> VexGraphResult<GraphNode> {
        let node_id = self.inode_to_node
            .get(&inode_number)
            .map(|entry| *entry)
            .ok_or_else(|| VexGraphError::NodeNotFound(
                format!("Node for inode {} not found", inode_number)
            ))?;
        
        self.get_node(node_id).await
    }
    
    /// Update a node's properties
    pub async fn update_node_properties(
        &self,
        node_id: NodeId,
        properties: HashMap<String, PropertyType>,
    ) -> VexGraphResult<()> {
        Validator::validate_node_id(node_id)?;
        
        let mut node_entry = self.nodes
            .get_mut(&node_id)
            .ok_or_else(|| VexGraphError::NodeNotFound(format!("Node {} not found", node_id)))?;
        
        for (key, value) in properties {
            node_entry.add_property(key, value)?;
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.property_count = self.calculate_total_properties();
            stats.last_updated = chrono::Utc::now();
        }
        
        tracing::debug!("Updated properties for node {}", node_id);
        Ok(())
    }
    
    /// Delete a node
    pub async fn delete_node(&self, node_id: NodeId) -> VexGraphResult<()> {
        Validator::validate_node_id(node_id)?;
        
        // Get the node to find its inode number
        let node = self.nodes
            .get(&node_id)
            .ok_or_else(|| VexGraphError::NodeNotFound(format!("Node {} not found", node_id)))?;
        
        let inode_number = node.inode_number;
        let node_type = node.node_type;
        let outgoing_edges = node.outgoing_edges.clone();
        let incoming_edges = node.incoming_edges.clone();
        drop(node);
        
        // Delete all connected edges
        for edge_id in outgoing_edges.iter().chain(incoming_edges.iter()) {
            self.delete_edge(*edge_id).await?;
        }
        
        // Remove from indices
        self.inode_to_node.remove(&inode_number);
        if let Some(mut type_nodes) = self.node_type_index.get_mut(&node_type) {
            type_nodes.retain(|&id| id != node_id);
        }
        
        // Remove the node
        self.nodes.remove(&node_id);
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.node_count -= 1;
            stats.last_updated = chrono::Utc::now();
        }
        
        // Notify kernel if integration is enabled
        if self.config.kernel_integration {
            self.notify_kernel_node_deleted(node_id, inode_number).await?;
        }
        
        tracing::debug!("Deleted node {}", node_id);
        Ok(())
    }
    
    /// Create a new edge
    pub async fn create_edge(
        &self,
        source_id: NodeId,
        target_id: NodeId,
        edge_type: EdgeType,
        weight: f64,
    ) -> VexGraphResult<EdgeId> {
        Validator::validate_node_id(source_id)?;
        Validator::validate_node_id(target_id)?;
        
        // Verify nodes exist
        if !self.nodes.contains_key(&source_id) {
            return Err(VexGraphError::NodeNotFound(format!("Source node {} not found", source_id)));
        }
        if !self.nodes.contains_key(&target_id) {
            return Err(VexGraphError::NodeNotFound(format!("Target node {} not found", target_id)));
        }
        
        // Generate new edge ID
        let edge_id = {
            let mut next_id = self.next_edge_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        Validator::validate_edge_id(edge_id)?;
        
        // Create the edge
        let edge = GraphEdge::new(edge_id, source_id, target_id, edge_type, weight);
        
        // Store the edge
        self.edges.insert(edge_id, edge);
        
        // Update node adjacency lists
        if let Some(mut source_node) = self.nodes.get_mut(&source_id) {
            source_node.add_outgoing_edge(edge_id);
        }
        if let Some(mut target_node) = self.nodes.get_mut(&target_id) {
            target_node.add_incoming_edge(edge_id);
        }
        
        // Update indices
        self.edge_type_index
            .entry(edge_type)
            .or_insert_with(Vec::new)
            .push(edge_id);
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.edge_count += 1;
            stats.last_updated = chrono::Utc::now();
        }
        
        // Notify kernel if integration is enabled
        if self.config.kernel_integration {
            self.notify_kernel_edge_created(edge_id, source_id, target_id, edge_type).await?;
        }
        
        tracing::debug!("Created edge {} from {} to {}", edge_id, source_id, target_id);
        Ok(edge_id)
    }
    
    /// Get an edge by ID
    pub async fn get_edge(&self, edge_id: EdgeId) -> VexGraphResult<GraphEdge> {
        Validator::validate_edge_id(edge_id)?;
        
        self.edges
            .get(&edge_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| VexGraphError::EdgeNotFound(format!("Edge {} not found", edge_id)))
    }
    
    /// Delete an edge
    pub async fn delete_edge(&self, edge_id: EdgeId) -> VexGraphResult<()> {
        Validator::validate_edge_id(edge_id)?;
        
        // Get the edge to find its endpoints
        let edge = self.edges
            .get(&edge_id)
            .ok_or_else(|| VexGraphError::EdgeNotFound(format!("Edge {} not found", edge_id)))?;
        
        let source_id = edge.source_id;
        let target_id = edge.target_id;
        let edge_type = edge.edge_type;
        drop(edge);
        
        // Update node adjacency lists
        if let Some(mut source_node) = self.nodes.get_mut(&source_id) {
            source_node.remove_outgoing_edge(edge_id);
        }
        if let Some(mut target_node) = self.nodes.get_mut(&target_id) {
            target_node.remove_incoming_edge(edge_id);
        }
        
        // Remove from indices
        if let Some(mut type_edges) = self.edge_type_index.get_mut(&edge_type) {
            type_edges.retain(|&id| id != edge_id);
        }
        
        // Remove the edge
        self.edges.remove(&edge_id);
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.edge_count -= 1;
            stats.last_updated = chrono::Utc::now();
        }
        
        // Notify kernel if integration is enabled
        if self.config.kernel_integration {
            self.notify_kernel_edge_deleted(edge_id, source_id, target_id).await?;
        }
        
        tracing::debug!("Deleted edge {}", edge_id);
        Ok(())
    }
    
    /// Get nodes by type
    pub async fn get_nodes_by_type(&self, node_type: NodeType) -> Vec<NodeId> {
        self.node_type_index
            .get(&node_type)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }
    
    /// Get edges by type
    pub async fn get_edges_by_type(&self, edge_type: EdgeType) -> Vec<EdgeId> {
        self.edge_type_index
            .get(&edge_type)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }
    
    /// Get outgoing edges for a node
    pub async fn get_outgoing_edges(&self, node_id: NodeId) -> VexGraphResult<Vec<EdgeId>> {
        let node = self.get_node(node_id).await?;
        Ok(node.outgoing_edges)
    }
    
    /// Get incoming edges for a node
    pub async fn get_incoming_edges(&self, node_id: NodeId) -> VexGraphResult<Vec<EdgeId>> {
        let node = self.get_node(node_id).await?;
        Ok(node.incoming_edges)
    }
    
    /// Get statistics
    pub async fn get_statistics(&self) -> VexGraphResult<CoreStatistics> {
        let mut stats = self.stats.write();
        stats.node_count = self.nodes.len() as u64;
        stats.edge_count = self.edges.len() as u64;
        stats.property_count = self.calculate_total_properties();
        stats.memory_usage = self.calculate_memory_usage();
        stats.last_updated = chrono::Utc::now();
        Ok(stats.clone())
    }
    
    /// Calculate total properties across all nodes and edges
    fn calculate_total_properties(&self) -> u64 {
        let node_properties: usize = self.nodes
            .iter()
            .map(|entry| entry.properties.len())
            .sum();
        
        let edge_properties: usize = self.edges
            .iter()
            .map(|entry| entry.properties.len())
            .sum();
        
        (node_properties + edge_properties) as u64
    }
    
    /// Calculate approximate memory usage
    fn calculate_memory_usage(&self) -> u64 {
        // Rough estimation of memory usage
        let node_memory = self.nodes.len() * std::mem::size_of::<GraphNode>();
        let edge_memory = self.edges.len() * std::mem::size_of::<GraphEdge>();
        (node_memory + edge_memory) as u64
    }
    
    /// Initialize kernel interface (placeholder for FFI integration)
    async fn initialize_kernel_interface(&self) -> VexGraphResult<()> {
        tracing::debug!("Initializing kernel interface");
        // TODO: Implement actual kernel FFI integration
        Ok(())
    }
    
    /// Cleanup kernel interface
    async fn cleanup_kernel_interface(&self) -> VexGraphResult<()> {
        tracing::debug!("Cleaning up kernel interface");
        // TODO: Implement actual kernel FFI cleanup
        Ok(())
    }
    
    /// Notify kernel of node creation
    async fn notify_kernel_node_created(
        &self,
        node_id: NodeId,
        inode_number: u64,
        node_type: NodeType,
    ) -> VexGraphResult<()> {
        tracing::debug!("Notifying kernel of node creation: {} (inode {})", node_id, inode_number);
        // TODO: Implement actual kernel notification
        Ok(())
    }
    
    /// Notify kernel of node deletion
    async fn notify_kernel_node_deleted(
        &self,
        node_id: NodeId,
        inode_number: u64,
    ) -> VexGraphResult<()> {
        tracing::debug!("Notifying kernel of node deletion: {} (inode {})", node_id, inode_number);
        // TODO: Implement actual kernel notification
        Ok(())
    }
    
    /// Notify kernel of edge creation
    async fn notify_kernel_edge_created(
        &self,
        edge_id: EdgeId,
        source_id: NodeId,
        target_id: NodeId,
        edge_type: EdgeType,
    ) -> VexGraphResult<()> {
        tracing::debug!("Notifying kernel of edge creation: {} ({} -> {})", edge_id, source_id, target_id);
        // TODO: Implement actual kernel notification
        Ok(())
    }
    
    /// Notify kernel of edge deletion
    async fn notify_kernel_edge_deleted(
        &self,
        edge_id: EdgeId,
        source_id: NodeId,
        target_id: NodeId,
    ) -> VexGraphResult<()> {
        tracing::debug!("Notifying kernel of edge deletion: {} ({} -> {})", edge_id, source_id, target_id);
        // TODO: Implement actual kernel notification
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_core_creation() {
        let config = VexGraphConfig::default();
        let core = VexGraphCore::new(&config).await.unwrap();
        assert_eq!(core.nodes.len(), 0);
        assert_eq!(core.edges.len(), 0);
    }
    
    #[tokio::test]
    async fn test_node_operations() {
        let config = VexGraphConfig::default();
        let core = VexGraphCore::new(&config).await.unwrap();
        
        // Create a node
        let node_id = core.create_node(123, NodeType::File).await.unwrap();
        assert_eq!(node_id, 1);
        
        // Get the node
        let node = core.get_node(node_id).await.unwrap();
        assert_eq!(node.id, node_id);
        assert_eq!(node.inode_number, 123);
        assert_eq!(node.node_type, NodeType::File);
        
        // Get node by inode
        let node_by_inode = core.get_node_by_inode(123).await.unwrap();
        assert_eq!(node_by_inode.id, node_id);
        
        // Update properties
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), PropertyType::String("test.txt".to_string()));
        core.update_node_properties(node_id, properties).await.unwrap();
        
        let updated_node = core.get_node(node_id).await.unwrap();
        assert!(updated_node.properties.contains_key("name"));
        
        // Delete the node
        core.delete_node(node_id).await.unwrap();
        assert!(core.get_node(node_id).await.is_err());
    }
    
    #[tokio::test]
    async fn test_edge_operations() {
        let config = VexGraphConfig::default();
        let core = VexGraphCore::new(&config).await.unwrap();
        
        // Create nodes
        let source_id = core.create_node(123, NodeType::File).await.unwrap();
        let target_id = core.create_node(456, NodeType::Directory).await.unwrap();
        
        // Create an edge
        let edge_id = core.create_edge(source_id, target_id, EdgeType::Contains, 1.0).await.unwrap();
        assert_eq!(edge_id, 1);
        
        // Get the edge
        let edge = core.get_edge(edge_id).await.unwrap();
        assert_eq!(edge.id, edge_id);
        assert_eq!(edge.source_id, source_id);
        assert_eq!(edge.target_id, target_id);
        assert_eq!(edge.edge_type, EdgeType::Contains);
        
        // Check node adjacency
        let outgoing = core.get_outgoing_edges(source_id).await.unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0], edge_id);
        
        let incoming = core.get_incoming_edges(target_id).await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0], edge_id);
        
        // Delete the edge
        core.delete_edge(edge_id).await.unwrap();
        assert!(core.get_edge(edge_id).await.is_err());
    }
    
    #[tokio::test]
    async fn test_statistics() {
        let config = VexGraphConfig::default();
        let core = VexGraphCore::new(&config).await.unwrap();
        
        let stats = core.get_statistics().await.unwrap();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        
        // Create some nodes and edges
        let node1 = core.create_node(123, NodeType::File).await.unwrap();
        let node2 = core.create_node(456, NodeType::Directory).await.unwrap();
        let _edge = core.create_edge(node1, node2, EdgeType::Contains, 1.0).await.unwrap();
        
        let stats = core.get_statistics().await.unwrap();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
    }
}