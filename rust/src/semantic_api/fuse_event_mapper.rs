//! FUSE Event Mapper
//!
//! This module implements the FuseEventMapper for mapping FUSE operations to semantic events.
//! It provides comprehensive mapping of POSIX filesystem operations to VexFS semantic events,
//! with context-aware event generation and metadata extraction.
//!
//! Key Features:
//! - Comprehensive mapping of FUSE operations to semantic events
//! - Context-aware event generation with metadata extraction
//! - Support for vector operations, graph operations, and traditional filesystem events
//! - Integration with existing semantic event types and emission framework
//! - Performance-optimized mapping with minimal overhead

use std::collections::HashMap;
use std::path::Path;

use tracing::{debug, trace, instrument};

use crate::semantic_api::types::{
    SemanticEventType, EventCategory, EventFlags, EventPriority,
    SemanticResult, SemanticError
};
use crate::semantic_api::fuse_journal_integration::FuseOperationType;

/// FUSE operation result for mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuseOperationResult {
    Success,
    Error(i32),
}

/// Context for FUSE operation mapping
#[derive(Debug, Clone)]
pub struct FuseMappingContext {
    pub operation_type: FuseOperationType,
    pub path: String,
    pub inode: u64,
    pub user_id: u32,
    pub group_id: u32,
    pub process_id: u32,
    pub file_size: Option<u64>,
    pub file_type: Option<String>,
    pub permissions: Option<u32>,
    pub metadata: HashMap<String, String>,
}

/// FUSE event mapping configuration
#[derive(Debug, Clone)]
pub struct FuseEventMappingConfig {
    pub map_filesystem_events: bool,
    pub map_vector_events: bool,
    pub map_graph_events: bool,
    pub map_system_events: bool,
    pub include_detailed_metadata: bool,
    pub generate_start_events: bool,
    pub generate_completion_events: bool,
    pub generate_error_events: bool,
}

impl Default for FuseEventMappingConfig {
    fn default() -> Self {
        Self {
            map_filesystem_events: true,
            map_vector_events: true,
            map_graph_events: true,
            map_system_events: true,
            include_detailed_metadata: true,
            generate_start_events: false, // Only completion events by default for performance
            generate_completion_events: true,
            generate_error_events: true,
        }
    }
}

/// FUSE event mapper for converting FUSE operations to semantic events
pub struct FuseEventMapper {
    config: FuseEventMappingConfig,
}

impl FuseEventMapper {
    /// Create a new FUSE event mapper
    pub fn new(config: FuseEventMappingConfig) -> Self {
        Self { config }
    }
    
    /// Create a new FUSE event mapper with default configuration
    pub fn new_default() -> Self {
        Self::new(FuseEventMappingConfig::default())
    }
    
    /// Map FUSE operation to start event type
    #[instrument(skip(self))]
    pub fn map_operation_to_start_event(
        &self,
        operation_type: FuseOperationType,
    ) -> SemanticResult<SemanticEventType> {
        if !self.config.generate_start_events {
            return Err(SemanticError::InvalidOperation(
                "Start events are disabled in configuration".to_string()
            ));
        }
        
        let event_type = match operation_type {
            // Filesystem operations - use existing filesystem event types
            FuseOperationType::Create => SemanticEventType::FilesystemCreate,
            FuseOperationType::Read => SemanticEventType::FilesystemRead,
            FuseOperationType::Write => SemanticEventType::FilesystemWrite,
            FuseOperationType::Delete => SemanticEventType::FilesystemDelete,
            FuseOperationType::Rename => SemanticEventType::FilesystemRename,
            FuseOperationType::Truncate => SemanticEventType::FilesystemTruncate,
            FuseOperationType::Chmod => SemanticEventType::FilesystemChmod,
            FuseOperationType::Chown => SemanticEventType::FilesystemChown,
            FuseOperationType::Mkdir => SemanticEventType::FilesystemMkdir,
            FuseOperationType::Rmdir => SemanticEventType::FilesystemRmdir,
            FuseOperationType::Symlink => SemanticEventType::FilesystemSymlink,
            FuseOperationType::Hardlink => SemanticEventType::FilesystemHardlink,
            
            // Directory operations - map to appropriate filesystem events
            FuseOperationType::Readdir => SemanticEventType::FilesystemRead,
            FuseOperationType::Lookup => SemanticEventType::FilesystemRead,
            
            // Metadata operations - map to read operations for start events
            FuseOperationType::Getattr => SemanticEventType::FilesystemRead,
            FuseOperationType::Setattr => SemanticEventType::FilesystemWrite,
            
            // Link operations
            FuseOperationType::Readlink => SemanticEventType::FilesystemRead,
            
            // Vector operations
            FuseOperationType::VectorSearch => SemanticEventType::VectorSearch,
            FuseOperationType::VectorInsert => SemanticEventType::VectorCreate,
            FuseOperationType::VectorUpdate => SemanticEventType::VectorUpdate,
            FuseOperationType::VectorDelete => SemanticEventType::VectorDelete,
            FuseOperationType::VectorIndex => SemanticEventType::VectorIndex,
            
            // Graph operations
            FuseOperationType::NodeCreate => SemanticEventType::GraphNodeCreate,
            FuseOperationType::NodeDelete => SemanticEventType::GraphNodeDelete,
            FuseOperationType::NodeUpdate => SemanticEventType::GraphNodeUpdate,
            FuseOperationType::EdgeCreate => SemanticEventType::GraphEdgeCreate,
            FuseOperationType::EdgeDelete => SemanticEventType::GraphEdgeDelete,
            FuseOperationType::GraphTraverse => SemanticEventType::GraphTraverse,
            
            // System operations
            FuseOperationType::Mount => SemanticEventType::SystemMount,
            FuseOperationType::Unmount => SemanticEventType::SystemUnmount,
            FuseOperationType::Sync => SemanticEventType::SystemSync,
            FuseOperationType::Flush => SemanticEventType::SystemSync,
            FuseOperationType::Fsync => SemanticEventType::SystemSync,
            FuseOperationType::Release => SemanticEventType::FilesystemRead, // Treat as read completion
            FuseOperationType::Open => SemanticEventType::FilesystemRead,
        };
        
        trace!("Mapped FUSE operation {:?} to start event {:?}", operation_type, event_type);
        Ok(event_type)
    }
    
    /// Map FUSE operation to completion event type
    #[instrument(skip(self))]
    pub fn map_operation_to_completion_event(
        &self,
        operation_type: FuseOperationType,
        success: bool,
    ) -> SemanticResult<SemanticEventType> {
        if !self.config.generate_completion_events {
            return Err(SemanticError::InvalidOperation(
                "Completion events are disabled in configuration".to_string()
            ));
        }
        
        // For error cases, we might want to use observability events
        if !success && self.config.generate_error_events {
            return Ok(SemanticEventType::ObservabilityErrorReported);
        }
        
        // For successful operations, use the same mapping as start events
        self.map_operation_to_start_event(operation_type)
    }
    
    /// Map FUSE operation to semantic event category
    #[instrument(skip(self))]
    pub fn map_operation_to_category(&self, operation_type: FuseOperationType) -> EventCategory {
        match operation_type {
            FuseOperationType::Create | FuseOperationType::Read | FuseOperationType::Write |
            FuseOperationType::Delete | FuseOperationType::Rename | FuseOperationType::Truncate |
            FuseOperationType::Chmod | FuseOperationType::Chown | FuseOperationType::Mkdir |
            FuseOperationType::Rmdir | FuseOperationType::Readdir | FuseOperationType::Lookup |
            FuseOperationType::Getattr | FuseOperationType::Setattr | FuseOperationType::Symlink |
            FuseOperationType::Hardlink | FuseOperationType::Readlink | FuseOperationType::Release |
            FuseOperationType::Open => EventCategory::Filesystem,
            
            FuseOperationType::VectorSearch | FuseOperationType::VectorInsert |
            FuseOperationType::VectorUpdate | FuseOperationType::VectorDelete |
            FuseOperationType::VectorIndex => EventCategory::Vector,
            
            FuseOperationType::NodeCreate | FuseOperationType::NodeDelete |
            FuseOperationType::NodeUpdate | FuseOperationType::EdgeCreate |
            FuseOperationType::EdgeDelete | FuseOperationType::GraphTraverse => EventCategory::Graph,
            
            FuseOperationType::Mount | FuseOperationType::Unmount | FuseOperationType::Sync |
            FuseOperationType::Flush | FuseOperationType::Fsync => EventCategory::System,
        }
    }
    
    /// Determine event flags for FUSE operation
    #[instrument(skip(self))]
    pub fn determine_event_flags(&self, operation_type: FuseOperationType) -> EventFlags {
        match operation_type {
            // Write operations should be persistent and indexed
            FuseOperationType::Create | FuseOperationType::Write | FuseOperationType::Delete |
            FuseOperationType::Rename | FuseOperationType::Truncate | FuseOperationType::Chmod |
            FuseOperationType::Chown | FuseOperationType::Mkdir | FuseOperationType::Rmdir |
            FuseOperationType::Setattr | FuseOperationType::Symlink | FuseOperationType::Hardlink => {
                EventFlags::PERSISTENT | EventFlags::INDEXED
            }
            
            // Vector operations
            FuseOperationType::VectorInsert | FuseOperationType::VectorUpdate |
            FuseOperationType::VectorDelete => {
                EventFlags::PERSISTENT | EventFlags::INDEXED | EventFlags::VECTOR_OPERATION
            }
            
            FuseOperationType::VectorSearch | FuseOperationType::VectorIndex => {
                EventFlags::INDEXED | EventFlags::VECTOR_OPERATION
            }
            
            // Graph operations
            FuseOperationType::NodeCreate | FuseOperationType::NodeDelete |
            FuseOperationType::NodeUpdate | FuseOperationType::EdgeCreate |
            FuseOperationType::EdgeDelete => {
                EventFlags::PERSISTENT | EventFlags::INDEXED | EventFlags::GRAPH_OPERATION
            }
            
            FuseOperationType::GraphTraverse => {
                EventFlags::INDEXED | EventFlags::GRAPH_OPERATION
            }
            
            // System operations
            FuseOperationType::Mount | FuseOperationType::Unmount => {
                EventFlags::PERSISTENT | EventFlags::INDEXED | EventFlags::SYSTEM_CRITICAL
            }
            
            FuseOperationType::Sync | FuseOperationType::Flush | FuseOperationType::Fsync => {
                EventFlags::PERSISTENT | EventFlags::SYSTEM_CRITICAL
            }
            
            // Read operations - just indexed
            _ => EventFlags::INDEXED,
        }
    }
    
    /// Determine event priority for FUSE operation
    #[instrument(skip(self))]
    pub fn determine_event_priority(&self, operation_type: FuseOperationType) -> EventPriority {
        match operation_type {
            // Critical system operations
            FuseOperationType::Mount | FuseOperationType::Unmount => EventPriority::Critical,
            
            // High priority operations
            FuseOperationType::Sync | FuseOperationType::Fsync => EventPriority::High,
            
            // High priority write operations
            FuseOperationType::Create | FuseOperationType::Write | FuseOperationType::Delete |
            FuseOperationType::VectorInsert | FuseOperationType::VectorUpdate |
            FuseOperationType::NodeCreate | FuseOperationType::EdgeCreate => EventPriority::High,
            
            // Medium priority operations
            FuseOperationType::Read | FuseOperationType::VectorSearch |
            FuseOperationType::GraphTraverse | FuseOperationType::Readdir => EventPriority::Medium,
            
            // Low priority operations
            _ => EventPriority::Low,
        }
    }
    
    /// Extract metadata from FUSE operation context
    #[instrument(skip(self, context))]
    pub fn extract_operation_metadata(
        &self,
        context: &FuseMappingContext,
    ) -> HashMap<String, String> {
        let mut metadata = context.metadata.clone();
        
        // Add basic operation information
        metadata.insert("fuse_operation".to_string(), format!("{:?}", context.operation_type));
        metadata.insert("path".to_string(), context.path.clone());
        metadata.insert("inode".to_string(), context.inode.to_string());
        metadata.insert("user_id".to_string(), context.user_id.to_string());
        metadata.insert("group_id".to_string(), context.group_id.to_string());
        metadata.insert("process_id".to_string(), context.process_id.to_string());
        
        // Add file-specific metadata if available
        if let Some(size) = context.file_size {
            metadata.insert("file_size".to_string(), size.to_string());
        }
        
        if let Some(ref file_type) = context.file_type {
            metadata.insert("file_type".to_string(), file_type.clone());
        }
        
        if let Some(permissions) = context.permissions {
            metadata.insert("permissions".to_string(), format!("{:o}", permissions));
        }
        
        // Add operation category
        let category = self.map_operation_to_category(context.operation_type);
        metadata.insert("event_category".to_string(), format!("{:?}", category));
        
        // Add detailed metadata if enabled
        if self.config.include_detailed_metadata {
            metadata.insert("detailed_mapping".to_string(), "true".to_string());
            
            // Add path components
            let path = Path::new(&context.path);
            if let Some(parent) = path.parent() {
                metadata.insert("parent_path".to_string(), parent.to_string_lossy().to_string());
            }
            if let Some(filename) = path.file_name() {
                metadata.insert("filename".to_string(), filename.to_string_lossy().to_string());
            }
            if let Some(extension) = path.extension() {
                metadata.insert("file_extension".to_string(), extension.to_string_lossy().to_string());
            }
            
            // Add operation-specific metadata
            match context.operation_type {
                FuseOperationType::VectorSearch | FuseOperationType::VectorInsert |
                FuseOperationType::VectorUpdate | FuseOperationType::VectorDelete |
                FuseOperationType::VectorIndex => {
                    metadata.insert("operation_domain".to_string(), "vector".to_string());
                }
                
                FuseOperationType::NodeCreate | FuseOperationType::NodeDelete |
                FuseOperationType::NodeUpdate | FuseOperationType::EdgeCreate |
                FuseOperationType::EdgeDelete | FuseOperationType::GraphTraverse => {
                    metadata.insert("operation_domain".to_string(), "graph".to_string());
                }
                
                _ => {
                    metadata.insert("operation_domain".to_string(), "filesystem".to_string());
                }
            }
        }
        
        trace!("Extracted metadata for FUSE operation {:?}: {} entries", 
               context.operation_type, metadata.len());
        
        metadata
    }
    
    /// Check if operation should be mapped based on configuration
    #[instrument(skip(self))]
    pub fn should_map_operation(&self, operation_type: FuseOperationType) -> bool {
        let category = self.map_operation_to_category(operation_type);
        
        match category {
            EventCategory::Filesystem => self.config.map_filesystem_events,
            EventCategory::Vector => self.config.map_vector_events,
            EventCategory::Graph => self.config.map_graph_events,
            EventCategory::System => self.config.map_system_events,
            _ => true, // Map other categories by default
        }
    }
    
    /// Create mapping context from FUSE operation parameters
    pub fn create_mapping_context(
        operation_type: FuseOperationType,
        path: &str,
        inode: u64,
        user_id: u32,
        group_id: u32,
        process_id: u32,
        additional_metadata: Option<HashMap<String, String>>,
    ) -> FuseMappingContext {
        FuseMappingContext {
            operation_type,
            path: path.to_string(),
            inode,
            user_id,
            group_id,
            process_id,
            file_size: None,
            file_type: None,
            permissions: None,
            metadata: additional_metadata.unwrap_or_default(),
        }
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: FuseEventMappingConfig) {
        self.config = config;
        debug!("Updated FUSE event mapper configuration");
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &FuseEventMappingConfig {
        &self.config
    }
    
    /// Get mapping statistics
    pub fn get_mapping_statistics(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        
        // Add configuration-based statistics
        stats.insert("filesystem_mapping_enabled".to_string(), 
                     if self.config.map_filesystem_events { 1 } else { 0 });
        stats.insert("vector_mapping_enabled".to_string(), 
                     if self.config.map_vector_events { 1 } else { 0 });
        stats.insert("graph_mapping_enabled".to_string(), 
                     if self.config.map_graph_events { 1 } else { 0 });
        stats.insert("system_mapping_enabled".to_string(), 
                     if self.config.map_system_events { 1 } else { 0 });
        stats.insert("detailed_metadata_enabled".to_string(), 
                     if self.config.include_detailed_metadata { 1 } else { 0 });
        stats.insert("start_events_enabled".to_string(), 
                     if self.config.generate_start_events { 1 } else { 0 });
        stats.insert("completion_events_enabled".to_string(), 
                     if self.config.generate_completion_events { 1 } else { 0 });
        stats.insert("error_events_enabled".to_string(), 
                     if self.config.generate_error_events { 1 } else { 0 });
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fuse_event_mapper_creation() {
        let config = FuseEventMappingConfig::default();
        let mapper = FuseEventMapper::new(config);
        
        assert!(mapper.config.map_filesystem_events);
        assert!(mapper.config.map_vector_events);
        assert!(mapper.config.map_graph_events);
        assert!(mapper.config.map_system_events);
    }
    
    #[test]
    fn test_operation_to_completion_event_mapping() {
        let mapper = FuseEventMapper::new_default();
        
        // Test filesystem operations
        let result = mapper.map_operation_to_completion_event(FuseOperationType::Create, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticEventType::FilesystemCreate);
        
        // Test vector operations
        let result = mapper.map_operation_to_completion_event(FuseOperationType::VectorSearch, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticEventType::VectorSearch);
        
        // Test graph operations
        let result = mapper.map_operation_to_completion_event(FuseOperationType::NodeCreate, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticEventType::GraphNodeCreate);
        
        // Test system operations
        let result = mapper.map_operation_to_completion_event(FuseOperationType::Mount, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticEventType::SystemMount);
    }
    
    #[test]
    fn test_operation_to_category_mapping() {
        let mapper = FuseEventMapper::new_default();
        
        assert_eq!(mapper.map_operation_to_category(FuseOperationType::Create), EventCategory::Filesystem);
        assert_eq!(mapper.map_operation_to_category(FuseOperationType::VectorSearch), EventCategory::Vector);
        assert_eq!(mapper.map_operation_to_category(FuseOperationType::NodeCreate), EventCategory::Graph);
        assert_eq!(mapper.map_operation_to_category(FuseOperationType::Mount), EventCategory::System);
    }
    
    #[test]
    fn test_event_flags_determination() {
        let mapper = FuseEventMapper::new_default();
        
        // Test write operations
        let flags = mapper.determine_event_flags(FuseOperationType::Create);
        assert!(flags.contains(EventFlags::PERSISTENT));
        assert!(flags.contains(EventFlags::INDEXED));
        
        // Test vector operations
        let flags = mapper.determine_event_flags(FuseOperationType::VectorInsert);
        assert!(flags.contains(EventFlags::PERSISTENT));
        assert!(flags.contains(EventFlags::INDEXED));
        assert!(flags.contains(EventFlags::VECTOR_OPERATION));
        
        // Test graph operations
        let flags = mapper.determine_event_flags(FuseOperationType::NodeCreate);
        assert!(flags.contains(EventFlags::PERSISTENT));
        assert!(flags.contains(EventFlags::INDEXED));
        assert!(flags.contains(EventFlags::GRAPH_OPERATION));
        
        // Test system operations
        let flags = mapper.determine_event_flags(FuseOperationType::Mount);
        assert!(flags.contains(EventFlags::PERSISTENT));
        assert!(flags.contains(EventFlags::INDEXED));
        assert!(flags.contains(EventFlags::SYSTEM_CRITICAL));
    }
    
    #[test]
    fn test_event_priority_determination() {
        let mapper = FuseEventMapper::new_default();
        
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Mount), EventPriority::Critical);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Sync), EventPriority::High);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Create), EventPriority::High);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Read), EventPriority::Medium);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Getattr), EventPriority::Low);
    }
    
    #[test]
    fn test_metadata_extraction() {
        let mapper = FuseEventMapper::new_default();
        
        let context = FuseMappingContext {
            operation_type: FuseOperationType::Create,
            path: "/test/file.txt".to_string(),
            inode: 123,
            user_id: 1000,
            group_id: 1000,
            process_id: 12345,
            file_size: Some(1024),
            file_type: Some("regular".to_string()),
            permissions: Some(0o644),
            metadata: HashMap::new(),
        };
        
        let metadata = mapper.extract_operation_metadata(&context);
        
        assert_eq!(metadata.get("fuse_operation").unwrap(), "Create");
        assert_eq!(metadata.get("path").unwrap(), "/test/file.txt");
        assert_eq!(metadata.get("inode").unwrap(), "123");
        assert_eq!(metadata.get("user_id").unwrap(), "1000");
        assert_eq!(metadata.get("file_size").unwrap(), "1024");
        assert_eq!(metadata.get("file_type").unwrap(), "regular");
        assert_eq!(metadata.get("permissions").unwrap(), "644");
        assert_eq!(metadata.get("filename").unwrap(), "file.txt");
        assert_eq!(metadata.get("file_extension").unwrap(), "txt");
        assert_eq!(metadata.get("event_category").unwrap(), "Filesystem");
    }
    
    #[test]
    fn test_should_map_operation() {
        let mut config = FuseEventMappingConfig::default();
        config.map_vector_events = false;
        
        let mapper = FuseEventMapper::new(config);
        
        assert!(mapper.should_map_operation(FuseOperationType::Create));
        assert!(!mapper.should_map_operation(FuseOperationType::VectorSearch));
        assert!(mapper.should_map_operation(FuseOperationType::NodeCreate));
        assert!(mapper.should_map_operation(FuseOperationType::Mount));
    }
    
    #[test]
    fn test_error_event_mapping() {
        let mapper = FuseEventMapper::new_default();
        
        // Test error mapping
        let result = mapper.map_operation_to_completion_event(FuseOperationType::Create, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SemanticEventType::ObservabilityErrorReported);
    }
    
    #[test]
    fn test_mapping_context_creation() {
        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());
        
        let context = FuseEventMapper::create_mapping_context(
            FuseOperationType::Write,
            "/test/path",
            456,
            1001,
            1001,
            54321,
            Some(metadata.clone()),
        );
        
        assert_eq!(context.operation_type, FuseOperationType::Write);
        assert_eq!(context.path, "/test/path");
        assert_eq!(context.inode, 456);
        assert_eq!(context.user_id, 1001);
        assert_eq!(context.metadata.get("test_key").unwrap(), "test_value");
    }
    
    #[test]
    fn test_mapping_statistics() {
        let mapper = FuseEventMapper::new_default();
        let stats = mapper.get_mapping_statistics();
        
        assert_eq!(stats.get("filesystem_mapping_enabled").unwrap(), &1);
        assert_eq!(stats.get("vector_mapping_enabled").unwrap(), &1);
        assert_eq!(stats.get("graph_mapping_enabled").unwrap(), &1);
        assert_eq!(stats.get("system_mapping_enabled").unwrap(), &1);
        assert_eq!(stats.get("detailed_metadata_enabled").unwrap(), &1);
        assert_eq!(stats.get("start_events_enabled").unwrap(), &0);
        assert_eq!(stats.get("completion_events_enabled").unwrap(), &1);
        assert_eq!(stats.get("error_events_enabled").unwrap(), &1);
    }
}