//! Userspace Event Hooks for VexFS Semantic Operation Journal
//! 
//! This module provides hooks for intercepting graph and vector operations
//! in userspace and emitting appropriate semantic events.

use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::time::{SystemTime, Instant};

use crate::semantic_api::types::{SemanticEventType, EventFlags, EventPriority};
use crate::semantic_api::event_emission::{emit_graph_event, emit_vector_event, get_global_emission_framework};

/// Graph operation types for event interception
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphOperationType {
    NodeCreate,
    NodeDelete,
    NodeUpdate,
    NodeQuery,
    EdgeCreate,
    EdgeDelete,
    EdgeUpdate,
    EdgeQuery,
    PropertySet,
    PropertyDelete,
    PropertyQuery,
    Traverse,
    BulkInsert,
    BulkDelete,
    Transaction,
}

/// Vector operation types for event interception
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VectorOperationType {
    VectorCreate,
    VectorDelete,
    VectorUpdate,
    VectorQuery,
    VectorSearch,
    VectorIndex,
    VectorSimilarity,
    VectorCluster,
    VectorEmbed,
    BulkInsert,
    BulkDelete,
    IndexRebuild,
}

/// Operation context for tracking
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub operation_id: u64,
    pub operation_type: String,
    pub started_at: SystemTime,
    pub metadata: HashMap<String, String>,
}

/// Hook configuration
#[derive(Debug, Clone)]
pub struct UserspaceHookConfig {
    pub graph_hooks_enabled: bool,
    pub vector_hooks_enabled: bool,
    pub performance_tracking: bool,
    pub error_tracking: bool,
    pub transaction_tracking: bool,
    pub bulk_operation_tracking: bool,
    pub detailed_logging: bool,
}

impl Default for UserspaceHookConfig {
    fn default() -> Self {
        Self {
            graph_hooks_enabled: true,
            vector_hooks_enabled: true,
            performance_tracking: true,
            error_tracking: true,
            transaction_tracking: true,
            bulk_operation_tracking: true,
            detailed_logging: false,
        }
    }
}

/// Userspace hook registry
pub struct UserspaceHookRegistry {
    config: RwLock<UserspaceHookConfig>,
    active_operations: Mutex<HashMap<u64, OperationContext>>,
    operation_counter: std::sync::atomic::AtomicU64,
    graph_hooks: RwLock<Vec<Box<dyn GraphHook + Send + Sync>>>,
    vector_hooks: RwLock<Vec<Box<dyn VectorHook + Send + Sync>>>,
}

/// Trait for graph operation hooks
pub trait GraphHook: Send + Sync {
    fn on_node_create(&self, node_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>>;
    fn on_node_delete(&self, node_id: u64) -> Result<(), Box<dyn std::error::Error>>;
    fn on_node_update(&self, node_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>>;
    fn on_edge_create(&self, edge_id: u64, from_node: u64, to_node: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>>;
    fn on_edge_delete(&self, edge_id: u64) -> Result<(), Box<dyn std::error::Error>>;
    fn on_edge_update(&self, edge_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>>;
    fn on_property_set(&self, entity_id: u64, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn on_property_delete(&self, entity_id: u64, key: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn on_traverse(&self, start_node: u64, path: &[u64]) -> Result<(), Box<dyn std::error::Error>>;
    fn on_query(&self, query: &str, result_count: usize) -> Result<(), Box<dyn std::error::Error>>;
}

/// Trait for vector operation hooks
pub trait VectorHook: Send + Sync {
    fn on_vector_create(&self, vector_id: u64, dimensions: u32, data: &[f32]) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_delete(&self, vector_id: u64) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_update(&self, vector_id: u64, data: &[f32]) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_search(&self, query_vector: &[f32], k: usize, results: &[(u64, f32)]) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_index(&self, vector_id: u64, index_type: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_similarity(&self, vector1_id: u64, vector2_id: u64, similarity: f32) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_cluster(&self, cluster_id: u64, vector_ids: &[u64]) -> Result<(), Box<dyn std::error::Error>>;
    fn on_vector_embed(&self, input: &str, vector_id: u64, dimensions: u32) -> Result<(), Box<dyn std::error::Error>>;
}

/// Default graph hook that emits semantic events
pub struct SemanticGraphHook;

impl GraphHook for SemanticGraphHook {
    fn on_node_create(&self, node_id: u64, _properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphNodeCreate,
            Some(node_id),
            None,
            Some(GraphOperationType::NodeCreate as u32),
        )?;
        Ok(())
    }

    fn on_node_delete(&self, node_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphNodeDelete,
            Some(node_id),
            None,
            Some(GraphOperationType::NodeDelete as u32),
        )?;
        Ok(())
    }

    fn on_node_update(&self, node_id: u64, _properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphNodeUpdate,
            Some(node_id),
            None,
            Some(GraphOperationType::NodeUpdate as u32),
        )?;
        Ok(())
    }

    fn on_edge_create(&self, edge_id: u64, _from_node: u64, _to_node: u64, _properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphEdgeCreate,
            None,
            Some(edge_id),
            Some(GraphOperationType::EdgeCreate as u32),
        )?;
        Ok(())
    }

    fn on_edge_delete(&self, edge_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphEdgeDelete,
            None,
            Some(edge_id),
            Some(GraphOperationType::EdgeDelete as u32),
        )?;
        Ok(())
    }

    fn on_edge_update(&self, edge_id: u64, _properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphEdgeUpdate,
            None,
            Some(edge_id),
            Some(GraphOperationType::EdgeUpdate as u32),
        )?;
        Ok(())
    }

    fn on_property_set(&self, entity_id: u64, _key: &str, _value: &str) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphPropertySet,
            Some(entity_id),
            None,
            Some(GraphOperationType::PropertySet as u32),
        )?;
        Ok(())
    }

    fn on_property_delete(&self, entity_id: u64, _key: &str) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphPropertyDelete,
            Some(entity_id),
            None,
            Some(GraphOperationType::PropertyDelete as u32),
        )?;
        Ok(())
    }

    fn on_traverse(&self, start_node: u64, _path: &[u64]) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphTraverse,
            Some(start_node),
            None,
            Some(GraphOperationType::Traverse as u32),
        )?;
        Ok(())
    }

    fn on_query(&self, _query: &str, result_count: usize) -> Result<(), Box<dyn std::error::Error>> {
        emit_graph_event(
            SemanticEventType::GraphQuery,
            None,
            None,
            Some(result_count as u32),
        )?;
        Ok(())
    }
}

/// Default vector hook that emits semantic events
pub struct SemanticVectorHook;

impl VectorHook for SemanticVectorHook {
    fn on_vector_create(&self, vector_id: u64, dimensions: u32, _data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorCreate,
            Some(vector_id),
            Some(dimensions),
            Some(1), // f32 type
        )?;
        Ok(())
    }

    fn on_vector_delete(&self, vector_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorDelete,
            Some(vector_id),
            None,
            None,
        )?;
        Ok(())
    }

    fn on_vector_update(&self, vector_id: u64, data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorUpdate,
            Some(vector_id),
            Some(data.len() as u32),
            Some(1), // f32 type
        )?;
        Ok(())
    }

    fn on_vector_search(&self, query_vector: &[f32], k: usize, _results: &[(u64, f32)]) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorSearch,
            None,
            Some(query_vector.len() as u32),
            Some(k as u32),
        )?;
        Ok(())
    }

    fn on_vector_index(&self, vector_id: u64, _index_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorIndex,
            Some(vector_id),
            None,
            None,
        )?;
        Ok(())
    }

    fn on_vector_similarity(&self, vector1_id: u64, vector2_id: u64, _similarity: f32) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorSimilarity,
            Some(vector1_id),
            None,
            Some(vector2_id as u32),
        )?;
        Ok(())
    }

    fn on_vector_cluster(&self, _cluster_id: u64, vector_ids: &[u64]) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorCluster,
            vector_ids.first().copied(),
            Some(vector_ids.len() as u32),
            None,
        )?;
        Ok(())
    }

    fn on_vector_embed(&self, _input: &str, vector_id: u64, dimensions: u32) -> Result<(), Box<dyn std::error::Error>> {
        emit_vector_event(
            SemanticEventType::VectorEmbed,
            Some(vector_id),
            Some(dimensions),
            None,
        )?;
        Ok(())
    }
}

impl UserspaceHookRegistry {
    /// Create a new userspace hook registry
    pub fn new(config: UserspaceHookConfig) -> Self {
        let mut registry = Self {
            config: RwLock::new(config),
            active_operations: Mutex::new(HashMap::new()),
            operation_counter: std::sync::atomic::AtomicU64::new(0),
            graph_hooks: RwLock::new(Vec::new()),
            vector_hooks: RwLock::new(Vec::new()),
        };
        
        // Register default semantic hooks
        registry.register_graph_hook(Box::new(SemanticGraphHook));
        registry.register_vector_hook(Box::new(SemanticVectorHook));
        
        registry
    }

    /// Register a graph hook
    pub fn register_graph_hook(&self, hook: Box<dyn GraphHook + Send + Sync>) {
        let mut hooks = self.graph_hooks.write().unwrap();
        hooks.push(hook);
    }

    /// Register a vector hook
    pub fn register_vector_hook(&self, hook: Box<dyn VectorHook + Send + Sync>) {
        let mut hooks = self.vector_hooks.write().unwrap();
        hooks.push(hook);
    }

    /// Start tracking an operation
    pub fn start_operation(&self, operation_type: String, metadata: HashMap<String, String>) -> u64 {
        let operation_id = self.operation_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let context = OperationContext {
            operation_id,
            operation_type,
            started_at: SystemTime::now(),
            metadata,
        };
        
        let mut operations = self.active_operations.lock().unwrap();
        operations.insert(operation_id, context);
        
        operation_id
    }

    /// End tracking an operation
    pub fn end_operation(&self, operation_id: u64) -> Option<OperationContext> {
        let mut operations = self.active_operations.lock().unwrap();
        operations.remove(&operation_id)
    }

    /// Get active operation count
    pub fn get_active_operation_count(&self) -> usize {
        self.active_operations.lock().unwrap().len()
    }

    // Graph operation hooks
    pub fn hook_node_create(&self, node_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.graph_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.graph_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_node_create(node_id, properties) {
                tracing::warn!("Graph hook failed for node create: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_node_delete(&self, node_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.graph_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.graph_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_node_delete(node_id) {
                tracing::warn!("Graph hook failed for node delete: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_node_update(&self, node_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.graph_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.graph_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_node_update(node_id, properties) {
                tracing::warn!("Graph hook failed for node update: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_edge_create(&self, edge_id: u64, from_node: u64, to_node: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.graph_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.graph_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_edge_create(edge_id, from_node, to_node, properties) {
                tracing::warn!("Graph hook failed for edge create: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_edge_delete(&self, edge_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.graph_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.graph_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_edge_delete(edge_id) {
                tracing::warn!("Graph hook failed for edge delete: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_edge_update(&self, edge_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.graph_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.graph_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_edge_update(edge_id, properties) {
                tracing::warn!("Graph hook failed for edge update: {}", e);
            }
        }
        Ok(())
    }

    // Vector operation hooks
    pub fn hook_vector_create(&self, vector_id: u64, dimensions: u32, data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.vector_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.vector_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_vector_create(vector_id, dimensions, data) {
                tracing::warn!("Vector hook failed for vector create: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_vector_delete(&self, vector_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.vector_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.vector_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_vector_delete(vector_id) {
                tracing::warn!("Vector hook failed for vector delete: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_vector_update(&self, vector_id: u64, data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.vector_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.vector_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_vector_update(vector_id, data) {
                tracing::warn!("Vector hook failed for vector update: {}", e);
            }
        }
        Ok(())
    }

    pub fn hook_vector_search(&self, query_vector: &[f32], k: usize, results: &[(u64, f32)]) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().unwrap();
        if !config.vector_hooks_enabled {
            return Ok(());
        }
        
        let hooks = self.vector_hooks.read().unwrap();
        for hook in hooks.iter() {
            if let Err(e) = hook.on_vector_search(query_vector, k, results) {
                tracing::warn!("Vector hook failed for vector search: {}", e);
            }
        }
        Ok(())
    }
}

/// Global userspace hook registry
static mut GLOBAL_USERSPACE_REGISTRY: Option<Arc<UserspaceHookRegistry>> = None;
static USERSPACE_INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Initialize userspace hooks
pub fn initialize_userspace_hooks(config: UserspaceHookConfig) -> Result<(), Box<dyn std::error::Error>> {
    USERSPACE_INIT_ONCE.call_once(|| {
        let registry = UserspaceHookRegistry::new(config);
        unsafe {
            GLOBAL_USERSPACE_REGISTRY = Some(Arc::new(registry));
        }
    });
    
    tracing::info!("Userspace hooks initialized");
    Ok(())
}

/// Get the global userspace hook registry
pub fn get_userspace_registry() -> Option<Arc<UserspaceHookRegistry>> {
    unsafe { GLOBAL_USERSPACE_REGISTRY.clone() }
}

/// Convenience functions for hooking operations
pub fn hook_graph_node_create(node_id: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(registry) = get_userspace_registry() {
        registry.hook_node_create(node_id, properties)
    } else {
        Ok(())
    }
}

pub fn hook_graph_edge_create(edge_id: u64, from_node: u64, to_node: u64, properties: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(registry) = get_userspace_registry() {
        registry.hook_edge_create(edge_id, from_node, to_node, properties)
    } else {
        Ok(())
    }
}

pub fn hook_vector_create(vector_id: u64, dimensions: u32, data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(registry) = get_userspace_registry() {
        registry.hook_vector_create(vector_id, dimensions, data)
    } else {
        Ok(())
    }
}

pub fn hook_vector_search(query_vector: &[f32], k: usize, results: &[(u64, f32)]) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(registry) = get_userspace_registry() {
        registry.hook_vector_search(query_vector, k, results)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_userspace_hook_registry() {
        let config = UserspaceHookConfig::default();
        let registry = UserspaceHookRegistry::new(config);
        
        // Test operation tracking
        let op_id = registry.start_operation("test_op".to_string(), HashMap::new());
        assert_eq!(registry.get_active_operation_count(), 1);
        
        let context = registry.end_operation(op_id);
        assert!(context.is_some());
        assert_eq!(registry.get_active_operation_count(), 0);
    }

    #[test]
    fn test_graph_hooks() {
        let config = UserspaceHookConfig::default();
        let registry = UserspaceHookRegistry::new(config);
        
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), "test_node".to_string());
        
        // Should not panic
        registry.hook_node_create(123, &properties).unwrap();
        registry.hook_node_delete(123).unwrap();
    }

    #[test]
    fn test_vector_hooks() {
        let config = UserspaceHookConfig::default();
        let registry = UserspaceHookRegistry::new(config);
        
        let data = vec![1.0, 2.0, 3.0, 4.0];
        
        // Should not panic
        registry.hook_vector_create(456, 4, &data).unwrap();
        registry.hook_vector_delete(456).unwrap();
    }
}