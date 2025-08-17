// FUSE-VexGraph Integration Bridge
// Connects filesystem operations to the graph database

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use fuse::{FileAttr, FileType};
use crate::monitoring::MonitoringSystem;

// Import VexGraph types
#[cfg(feature = "vexgraph")]
use crate::vexgraph::{
    NodeType, EdgeType, PropertyType,
    storage_backend::{StorageBackend, StorageFactory, StorageConfig, StorageError},
};

/// Bridge between FUSE filesystem and VexGraph
pub struct FuseVexGraphBridge {
    #[cfg(feature = "vexgraph")]
    storage: Box<dyn StorageBackend>,
    
    // Mapping between filesystem inodes and graph node IDs
    inode_to_node: Arc<Mutex<HashMap<u64, u64>>>,
    node_to_inode: Arc<Mutex<HashMap<u64, u64>>>,
    
    // Path to node mapping for quick lookups
    path_to_node: Arc<Mutex<HashMap<PathBuf, u64>>>,
    
    monitoring: Arc<MonitoringSystem>,
    next_node_id: Arc<Mutex<u64>>,
    next_edge_id: Arc<Mutex<u64>>,
}

impl FuseVexGraphBridge {
    /// Create a new bridge with the specified storage backend
    pub fn new(monitoring: Arc<MonitoringSystem>, storage_config: StorageConfig) -> Result<Self, String> {
        #[cfg(feature = "vexgraph")]
        {
            let mut storage = StorageFactory::from_config(&storage_config)
                .map_err(|e| format!("Failed to create storage: {:?}", e))?;
            
            storage.init()
                .map_err(|e| format!("Failed to initialize storage: {:?}", e))?;
            
            Ok(Self {
                storage,
                inode_to_node: Arc::new(Mutex::new(HashMap::new())),
                node_to_inode: Arc::new(Mutex::new(HashMap::new())),
                path_to_node: Arc::new(Mutex::new(HashMap::new())),
                monitoring,
                next_node_id: Arc::new(Mutex::new(1000)), // Start from 1000 to avoid conflicts
                next_edge_id: Arc::new(Mutex::new(1000)),
            })
        }
        
        #[cfg(not(feature = "vexgraph"))]
        {
            Ok(Self {
                inode_to_node: Arc::new(Mutex::new(HashMap::new())),
                node_to_inode: Arc::new(Mutex::new(HashMap::new())),
                path_to_node: Arc::new(Mutex::new(HashMap::new())),
                monitoring,
                next_node_id: Arc::new(Mutex::new(1000)),
                next_edge_id: Arc::new(Mutex::new(1000)),
            })
        }
    }
    
    /// Create a graph node for a file/directory
    pub fn create_node_for_file(
        &mut self,
        inode: u64,
        path: &Path,
        attr: &FileAttr,
    ) -> Result<u64, String> {
        let node_id = self.get_next_node_id();
        
        // Create mappings
        {
            let mut inode_map = self.inode_to_node.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            inode_map.insert(inode, node_id);
            
            let mut node_map = self.node_to_inode.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            node_map.insert(node_id, inode);
            
            let mut path_map = self.path_to_node.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            path_map.insert(path.to_path_buf(), node_id);
        }
        
        #[cfg(feature = "vexgraph")]
        {
            use crate::vexgraph::core::GraphNode;
            use chrono::Utc;
            
            // Determine node type based on file type
            let node_type = match attr.kind {
                FileType::Directory => NodeType::Directory,
                FileType::RegularFile => NodeType::File,
                _ => NodeType::File,
            };
            
            // Create the graph node
            let mut node = GraphNode::new(node_id, inode, node_type);
            
            // Add properties
            node.add_property(
                "path".to_string(),
                PropertyType::String(path.to_string_lossy().to_string())
            ).ok();
            
            node.add_property(
                "size".to_string(),
                PropertyType::Integer(attr.size as i64)
            ).ok();
            
            node.add_property(
                "permissions".to_string(),
                PropertyType::Integer(attr.perm as i64)
            ).ok();
            
            node.add_property(
                "created_at".to_string(),
                PropertyType::Timestamp(Utc::now())
            ).ok();
            
            // Store the node
            self.storage.store_node(&node)
                .map_err(|e| format!("Failed to store node: {:?}", e))?;
        }
        
        Ok(node_id)
    }
    
    /// Create a parent-child relationship edge
    pub fn create_parent_edge(
        &mut self,
        parent_node_id: u64,
        child_node_id: u64,
    ) -> Result<u64, String> {
        let edge_id = self.get_next_edge_id();
        
        #[cfg(feature = "vexgraph")]
        {
            use crate::vexgraph::core::GraphEdge;
            
            // Create the edge
            let edge = GraphEdge::new(
                edge_id,
                parent_node_id,
                child_node_id,
                EdgeType::Contains,
                1.0, // Default weight
            );
            
            // Store the edge
            self.storage.store_edge(&edge)
                .map_err(|e| format!("Failed to store edge: {:?}", e))?;
        }
        
        Ok(edge_id)
    }
    
    /// Update node properties when file attributes change
    pub fn update_node_properties(
        &mut self,
        inode: u64,
        attr: &FileAttr,
    ) -> Result<(), String> {
        let node_id = {
            let inode_map = self.inode_to_node.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            match inode_map.get(&inode) {
                Some(&id) => id,
                None => return Ok(()), // Node doesn't exist in graph
            }
        };
        
        #[cfg(feature = "vexgraph")]
        {
            use crate::vexgraph::core::GraphNode;
            use chrono::Utc;
            
            // Get the existing node
            let mut node = match self.storage.get_node(&node_id)
                .map_err(|e| format!("Failed to get node: {:?}", e))? {
                Some(n) => n,
                None => return Ok(()),
            };
            
            // Update properties
            node.add_property(
                "size".to_string(),
                PropertyType::Integer(attr.size as i64)
            ).ok();
            
            node.add_property(
                "modified_at".to_string(),
                PropertyType::Timestamp(Utc::now())
            ).ok();
            
            // Store the updated node
            self.storage.store_node(&node)
                .map_err(|e| format!("Failed to update node: {:?}", e))?;
        }
        
        Ok(())
    }
    
    /// Delete a node when a file is removed
    pub fn delete_node_for_file(&mut self, inode: u64) -> Result<(), String> {
        let node_id = {
            let mut inode_map = self.inode_to_node.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            match inode_map.remove(&inode) {
                Some(id) => id,
                None => return Ok(()), // Node doesn't exist
            }
        };
        
        // Remove from mappings
        {
            let mut node_map = self.node_to_inode.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            node_map.remove(&node_id);
            
            // Remove from path mapping (need to find the path)
            let mut path_map = self.path_to_node.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            path_map.retain(|_, &mut v| v != node_id);
        }
        
        #[cfg(feature = "vexgraph")]
        {
            // Delete all edges associated with this node
            let edge_ids = self.storage.get_node_edges(&node_id)
                .map_err(|e| format!("Failed to get edges: {:?}", e))?;
            
            for edge_id in edge_ids {
                self.storage.delete_edge(&edge_id)
                    .map_err(|e| format!("Failed to delete edge: {:?}", e))?;
            }
            
            // Delete the node
            self.storage.delete_node(&node_id)
                .map_err(|e| format!("Failed to delete node: {:?}", e))?;
        }
        
        Ok(())
    }
    
    /// Find similar files based on vector embeddings
    #[cfg(feature = "vexgraph")]
    pub fn find_similar_files(
        &self,
        inode: u64,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<(u64, f32)>, String> {
        let node_id = {
            let inode_map = self.inode_to_node.lock()
                .map_err(|e| format!("Lock error: {}", e))?;
            match inode_map.get(&inode) {
                Some(&id) => id,
                None => return Ok(Vec::new()),
            }
        };
        
        // This would integrate with the semantic search functionality
        // For now, return empty results
        Ok(Vec::new())
    }
    
    /// Get graph statistics
    pub fn get_stats(&self) -> GraphStats {
        #[cfg(feature = "vexgraph")]
        {
            let storage_stats = self.storage.get_stats();
            GraphStats {
                total_nodes: storage_stats.total_nodes,
                total_edges: storage_stats.total_edges,
                total_mappings: self.inode_to_node.lock()
                    .map(|m| m.len()).unwrap_or(0),
            }
        }
        
        #[cfg(not(feature = "vexgraph"))]
        {
            GraphStats {
                total_nodes: 0,
                total_edges: 0,
                total_mappings: self.inode_to_node.lock()
                    .map(|m| m.len()).unwrap_or(0),
            }
        }
    }
    
    /// Flush any pending writes
    pub fn flush(&mut self) -> Result<(), String> {
        #[cfg(feature = "vexgraph")]
        {
            self.storage.flush()
                .map_err(|e| format!("Failed to flush storage: {:?}", e))?;
        }
        Ok(())
    }
    
    // Helper methods
    
    fn get_next_node_id(&self) -> u64 {
        let mut next_id = self.next_node_id.lock().unwrap_or_else(|e| e.into_inner());
        let id = *next_id;
        *next_id += 1;
        id
    }
    
    fn get_next_edge_id(&self) -> u64 {
        let mut next_id = self.next_edge_id.lock().unwrap_or_else(|e| e.into_inner());
        let id = *next_id;
        *next_id += 1;
        id
    }
}

/// Statistics about the graph
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_mappings: usize,
}

/// Builder for creating FUSE-VexGraph integrated filesystem
pub struct IntegratedFilesystemBuilder {
    storage_config: StorageConfig,
    enable_monitoring: bool,
    enable_semantic_search: bool,
}

impl IntegratedFilesystemBuilder {
    pub fn new() -> Self {
        Self {
            storage_config: StorageConfig::Memory,
            enable_monitoring: true,
            enable_semantic_search: false,
        }
    }
    
    pub fn with_storage(mut self, config: StorageConfig) -> Self {
        self.storage_config = config;
        self
    }
    
    pub fn with_monitoring(mut self, enable: bool) -> Self {
        self.enable_monitoring = enable;
        self
    }
    
    pub fn with_semantic_search(mut self, enable: bool) -> Self {
        self.enable_semantic_search = enable;
        self
    }
    
    pub fn build(self) -> Result<IntegratedFilesystem, String> {
        let monitoring = if self.enable_monitoring {
            Arc::new(MonitoringSystem::new())
        } else {
            Arc::new(MonitoringSystem::new())
        };
        
        let bridge = FuseVexGraphBridge::new(monitoring.clone(), self.storage_config)?;
        
        Ok(IntegratedFilesystem {
            bridge: Arc::new(Mutex::new(bridge)),
            monitoring,
            enable_semantic_search: self.enable_semantic_search,
        })
    }
}

/// Integrated FUSE filesystem with VexGraph
pub struct IntegratedFilesystem {
    bridge: Arc<Mutex<FuseVexGraphBridge>>,
    monitoring: Arc<MonitoringSystem>,
    enable_semantic_search: bool,
}

impl IntegratedFilesystem {
    /// Get the bridge for direct access
    pub fn get_bridge(&self) -> Arc<Mutex<FuseVexGraphBridge>> {
        self.bridge.clone()
    }
    
    /// Get monitoring system
    pub fn get_monitoring(&self) -> Arc<MonitoringSystem> {
        self.monitoring.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    
    #[test]
    fn test_bridge_creation() {
        let monitoring = Arc::new(MonitoringSystem::new());
        let bridge = FuseVexGraphBridge::new(
            monitoring,
            StorageConfig::Memory
        );
        
        assert!(bridge.is_ok());
    }
    
    #[test]
    fn test_node_creation() {
        let monitoring = Arc::new(MonitoringSystem::new());
        let mut bridge = FuseVexGraphBridge::new(
            monitoring,
            StorageConfig::Memory
        ).unwrap();
        
        let attr = FileAttr {
            ino: 1,
            size: 1024,
            blocks: 2,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        };
        
        let result = bridge.create_node_for_file(1, Path::new("/test.txt"), &attr);
        assert!(result.is_ok());
        
        let stats = bridge.get_stats();
        assert_eq!(stats.total_mappings, 1);
    }
}