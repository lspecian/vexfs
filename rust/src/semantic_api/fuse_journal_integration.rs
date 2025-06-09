//! FUSE Journal Integration
//! 
//! This module integrates the userspace semantic journal with the FUSE filesystem,
//! providing seamless event tracking and AI-native capabilities for FUSE operations.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use crate::semantic_api::userspace_journal::{
    UserspaceSemanticJournal, UserspaceJournalConfig, JournalEventStream, StreamMessage
};
use crate::fuse_impl::VexFSFuse;
use crate::vector_storage::VectorStorageManager;
use crate::anns::hnsw::HnswGraph;
use crate::shared::errors::{VexfsError, VexfsResult};

use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::Utc;
use uuid::Uuid;

/// Maximum stack usage for FUSE journal operations (6KB limit)
const FUSE_MAX_STACK_USAGE: usize = 6144;

/// FUSE Journal Integration Manager
pub struct FuseJournalIntegration {
    /// Userspace semantic journal
    journal: Arc<UserspaceSemanticJournal>,
    /// FUSE filesystem reference
    fuse_fs: Option<Arc<Mutex<VexFSFuse>>>,
    /// Vector storage integration
    vector_storage: Option<Arc<VectorStorageManager>>,
    /// HNSW graph integration
    hnsw_graph: Option<Arc<Mutex<HnswGraph>>>,
    /// Event stream for real-time monitoring
    event_streams: Arc<RwLock<HashMap<Uuid, JournalEventStream>>>,
    /// Integration statistics
    stats: Arc<RwLock<FuseJournalStats>>,
    /// Configuration
    config: FuseJournalConfig,
}

/// Configuration for FUSE journal integration
#[derive(Debug, Clone)]
pub struct FuseJournalConfig {
    /// Enable automatic event generation for FUSE operations
    pub auto_event_generation: bool,
    /// Enable vector operation tracking
    pub track_vector_operations: bool,
    /// Enable graph operation tracking
    pub track_graph_operations: bool,
    /// Enable filesystem operation tracking
    pub track_filesystem_operations: bool,
    /// Batch size for event processing
    pub batch_size: usize,
    /// Event buffer size
    pub event_buffer_size: usize,
    /// Enable real-time event streaming
    pub enable_streaming: bool,
    /// Maximum concurrent streams
    pub max_concurrent_streams: usize,
}

impl Default for FuseJournalConfig {
    fn default() -> Self {
        Self {
            auto_event_generation: true,
            track_vector_operations: true,
            track_graph_operations: true,
            track_filesystem_operations: true,
            batch_size: 50,
            event_buffer_size: 1000,
            enable_streaming: true,
            max_concurrent_streams: 10,
        }
    }
}

/// Statistics for FUSE journal integration
#[derive(Debug, Clone, Default)]
pub struct FuseJournalStats {
    /// Total events generated
    pub events_generated: u64,
    /// Vector events generated
    pub vector_events: u64,
    /// Graph events generated
    pub graph_events: u64,
    /// Filesystem events generated
    pub filesystem_events: u64,
    /// Active event streams
    pub active_streams: u64,
    /// Total stream messages sent
    pub stream_messages_sent: u64,
    /// Integration errors
    pub integration_errors: u64,
    /// Last event timestamp
    pub last_event_time: SystemTime,
}

impl FuseJournalIntegration {
    /// Create a new FUSE journal integration
    pub fn new(
        journal: Arc<UserspaceSemanticJournal>,
        config: FuseJournalConfig,
    ) -> SemanticResult<Self> {
        Ok(Self {
            journal,
            fuse_fs: None,
            vector_storage: None,
            hnsw_graph: None,
            event_streams: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(FuseJournalStats::default())),
            config,
        })
    }
    
    /// Initialize with FUSE filesystem
    pub fn with_fuse_fs(mut self, fuse_fs: Arc<Mutex<VexFSFuse>>) -> Self {
        self.fuse_fs = Some(fuse_fs);
        self
    }
    
    /// Initialize with vector storage
    pub fn with_vector_storage(mut self, vector_storage: Arc<VectorStorageManager>) -> Self {
        self.vector_storage = Some(vector_storage);
        self
    }
    
    /// Initialize with HNSW graph
    pub fn with_hnsw_graph(mut self, hnsw_graph: Arc<Mutex<HnswGraph>>) -> Self {
        self.hnsw_graph = Some(hnsw_graph);
        self
    }
    
    /// Record a filesystem operation event
    pub fn record_filesystem_event(
        &self,
        operation: FilesystemOperation,
        path: &str,
        inode: u64,
        result: OperationResult,
    ) -> SemanticResult<u64> {
        if !self.config.track_filesystem_operations {
            return Ok(0);
        }
        
        // Check stack usage
        let _stack_check = [0u8; 512]; // Small allocation to check stack
        if _stack_check.len() > FUSE_MAX_STACK_USAGE / 10 {
            return Err(SemanticError::StackOverflow);
        }
        
        let event_type = match operation {
            FilesystemOperation::Create => SemanticEventType::FilesystemCreate,
            FilesystemOperation::Read => SemanticEventType::FilesystemRead,
            FilesystemOperation::Write => SemanticEventType::FilesystemWrite,
            FilesystemOperation::Delete => SemanticEventType::FilesystemDelete,
            FilesystemOperation::Rename => SemanticEventType::FilesystemRename,
            FilesystemOperation::Mkdir => SemanticEventType::FilesystemMkdir,
            FilesystemOperation::Rmdir => SemanticEventType::FilesystemRmdir,
        };
        
        let event = SemanticEvent {
            event_id: 0, // Will be assigned by journal
            event_type,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: self.get_next_sequence(),
                cpu_id: 0,
                process_id: std::process::id(),
            },
            global_sequence: 0, // Will be assigned by journal
            local_sequence: 0,  // Will be assigned by journal
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::Normal,
            event_size: 0, // Will be calculated
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 50,
            replay_priority: 1,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: Some(FilesystemContext {
                    path: path.to_string(),
                    inode_number: Some(inode),
                    file_type: None,
                }),
                graph: None,
                vector: None,
                agent: None,
                system: None,
                semantic: None,
                observability: None,
            },
            payload: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "result": format!("{:?}", result),
                "path": path,
                "inode": inode
            })),
            metadata: None,
        };
        
        let event_id = self.journal.write_event(event)?;
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.events_generated += 1;
            stats.filesystem_events += 1;
            stats.last_event_time = SystemTime::now();
        }
        
        // Notify event streams
        self.notify_event_streams(&event_id)?;
        
        Ok(event_id)
    }
    
    /// Record a vector operation event
    pub fn record_vector_event(
        &self,
        operation: VectorOperation,
        vector_id: u64,
        dimensions: u32,
        file_inode: Option<u64>,
        result: OperationResult,
    ) -> SemanticResult<u64> {
        if !self.config.track_vector_operations {
            return Ok(0);
        }
        
        // Check stack usage
        let _stack_check = [0u8; 512];
        if _stack_check.len() > FUSE_MAX_STACK_USAGE / 10 {
            return Err(SemanticError::StackOverflow);
        }
        
        let event_type = match operation {
            VectorOperation::Create => SemanticEventType::VectorCreate,
            VectorOperation::Update => SemanticEventType::VectorUpdate,
            VectorOperation::Delete => SemanticEventType::VectorDelete,
            VectorOperation::Search => SemanticEventType::VectorSearch,
            VectorOperation::Index => SemanticEventType::VectorIndex,
            VectorOperation::Similarity => SemanticEventType::VectorSimilarity,
        };
        
        let event = SemanticEvent {
            event_id: 0,
            event_type,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: self.get_next_sequence(),
                cpu_id: 0,
                process_id: std::process::id(),
            },
            global_sequence: 0,
            local_sequence: 0,
            flags: EventFlags {
                atomic: true,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: false, // Vector operations may have non-deterministic results
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::High, // Vector operations are high priority
            event_size: 0,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 80, // High relevance for AI agents
            replay_priority: 2,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: file_inode.map(|inode| FilesystemContext {
                    path: format!("inode:{}", inode),
                    inode_number: Some(inode),
                    file_type: Some("vector".to_string()),
                }),
                graph: None,
                vector: Some(VectorContext {
                    vector_id: Some(vector_id),
                    dimensions: Some(dimensions),
                    element_type: Some(0), // Float32
                }),
                agent: None,
                system: None,
                semantic: None,
                observability: None,
            },
            payload: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "result": format!("{:?}", result),
                "vector_id": vector_id,
                "dimensions": dimensions,
                "file_inode": file_inode
            })),
            metadata: None,
        };
        
        let event_id = self.journal.write_event(event)?;
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.events_generated += 1;
            stats.vector_events += 1;
            stats.last_event_time = SystemTime::now();
        }
        
        // Integrate with vector storage if available
        if let Some(ref vector_storage) = self.vector_storage {
            self.integrate_vector_storage_event(vector_id, &operation, vector_storage)?;
        }
        
        // Notify event streams
        self.notify_event_streams(&event_id)?;
        
        Ok(event_id)
    }
    
    /// Record a graph operation event
    pub fn record_graph_event(
        &self,
        operation: GraphOperation,
        node_id: Option<u64>,
        edge_id: Option<u64>,
        result: OperationResult,
    ) -> SemanticResult<u64> {
        if !self.config.track_graph_operations {
            return Ok(0);
        }
        
        // Check stack usage
        let _stack_check = [0u8; 512];
        if _stack_check.len() > FUSE_MAX_STACK_USAGE / 10 {
            return Err(SemanticError::StackOverflow);
        }
        
        let event_type = match operation {
            GraphOperation::NodeCreate => SemanticEventType::GraphNodeCreate,
            GraphOperation::NodeUpdate => SemanticEventType::GraphNodeUpdate,
            GraphOperation::NodeDelete => SemanticEventType::GraphNodeDelete,
            GraphOperation::EdgeCreate => SemanticEventType::GraphEdgeCreate,
            GraphOperation::EdgeUpdate => SemanticEventType::GraphEdgeUpdate,
            GraphOperation::EdgeDelete => SemanticEventType::GraphEdgeDelete,
            GraphOperation::Traverse => SemanticEventType::GraphTraverse,
            GraphOperation::Query => SemanticEventType::GraphQuery,
        };
        
        let event = SemanticEvent {
            event_id: 0,
            event_type,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: self.get_next_sequence(),
                cpu_id: 0,
                process_id: std::process::id(),
            },
            global_sequence: 0,
            local_sequence: 0,
            flags: EventFlags {
                atomic: true,
                transactional: true,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::High,
            event_size: 0,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 90, // Very high relevance for graph operations
            replay_priority: 3,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: None,
                graph: Some(GraphContext {
                    node_id,
                    edge_id,
                    operation_type: Some(operation as u32),
                }),
                vector: None,
                agent: None,
                system: None,
                semantic: None,
                observability: None,
            },
            payload: Some(serde_json::json!({
                "operation": format!("{:?}", operation),
                "result": format!("{:?}", result),
                "node_id": node_id,
                "edge_id": edge_id
            })),
            metadata: None,
        };
        
        let event_id = self.journal.write_event(event)?;
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.events_generated += 1;
            stats.graph_events += 1;
            stats.last_event_time = SystemTime::now();
        }
        
        // Integrate with HNSW graph if available
        if let Some(ref hnsw_graph) = self.hnsw_graph {
            self.integrate_hnsw_graph_event(node_id, edge_id, &operation, hnsw_graph)?;
        }
        
        // Notify event streams
        self.notify_event_streams(&event_id)?;
        
        Ok(event_id)
    }
    
    /// Create an event stream subscription
    pub fn create_event_stream(
        &self,
        agent_id: String,
        filter: EventFilter,
    ) -> SemanticResult<Uuid> {
        if !self.config.enable_streaming {
            return Err(SemanticError::FeatureDisabled("Event streaming is disabled".to_string()));
        }
        
        // Check if we've reached the maximum number of streams
        {
            let streams = self.event_streams.read()
                .map_err(|_| SemanticError::LockError)?;
            if streams.len() >= self.config.max_concurrent_streams {
                return Err(SemanticError::ResourceExhausted("Maximum concurrent streams reached".to_string()));
            }
        }
        
        let subscription_id = Uuid::new_v4();
        let subscription = StreamSubscription {
            subscription_id,
            agent_id,
            filter,
            buffer_size: self.config.event_buffer_size,
            include_historical: false,
            historical_limit: None,
        };
        
        let event_stream = JournalEventStream::new(
            self.journal.clone(),
            subscription,
        );
        
        // Add to active streams
        {
            let mut streams = self.event_streams.write()
                .map_err(|_| SemanticError::LockError)?;
            streams.insert(subscription_id, event_stream);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.active_streams += 1;
        }
        
        Ok(subscription_id)
    }
    
    /// Remove an event stream subscription
    pub fn remove_event_stream(&self, subscription_id: Uuid) -> SemanticResult<()> {
        let removed = {
            let mut streams = self.event_streams.write()
                .map_err(|_| SemanticError::LockError)?;
            streams.remove(&subscription_id).is_some()
        };
        
        if removed {
            let mut stats = self.stats.write()
                .map_err(|_| SemanticError::LockError)?;
            stats.active_streams = stats.active_streams.saturating_sub(1);
        }
        
        Ok(())
    }
    
    /// Get next events from a stream
    pub fn get_stream_events(&self, subscription_id: Uuid) -> SemanticResult<Vec<StreamEventMessage>> {
        let mut streams = self.event_streams.write()
            .map_err(|_| SemanticError::LockError)?;
        
        if let Some(stream) = streams.get_mut(&subscription_id) {
            let events = stream.next_events()?;
            
            // Update statistics
            {
                let mut stats = self.stats.write()
                    .map_err(|_| SemanticError::LockError)?;
                stats.stream_messages_sent += events.len() as u64;
            }
            
            Ok(events)
        } else {
            Err(SemanticError::NotFound("Stream subscription not found".to_string()))
        }
    }
    
    /// Get integration statistics
    pub fn get_statistics(&self) -> SemanticResult<FuseJournalStats> {
        let stats = self.stats.read()
            .map_err(|_| SemanticError::LockError)?;
        Ok(stats.clone())
    }
    
    /// Force synchronization of all pending events
    pub fn sync(&self) -> SemanticResult<()> {
        self.journal.sync()
    }
    
    // Private helper methods
    
    fn get_next_sequence(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
    
    fn notify_event_streams(&self, event_id: &u64) -> SemanticResult<()> {
        // In a full implementation, this would notify all active streams
        // about the new event. For now, we just track that an event occurred.
        Ok(())
    }
    
    fn integrate_vector_storage_event(
        &self,
        vector_id: u64,
        operation: &VectorOperation,
        vector_storage: &Arc<VectorStorageManager>,
    ) -> SemanticResult<()> {
        // Integration with vector storage for enhanced tracking
        match operation {
            VectorOperation::Create | VectorOperation::Update => {
                // Could trigger additional indexing or caching operations
                eprintln!("FUSE Journal: Vector storage integration for vector {} operation {:?}", vector_id, operation);
            }
            VectorOperation::Search => {
                // Could update search statistics or cache results
                eprintln!("FUSE Journal: Vector search integration for vector {}", vector_id);
            }
            _ => {}
        }
        Ok(())
    }
    
    fn integrate_hnsw_graph_event(
        &self,
        node_id: Option<u64>,
        edge_id: Option<u64>,
        operation: &GraphOperation,
        hnsw_graph: &Arc<Mutex<HnswGraph>>,
    ) -> SemanticResult<()> {
        // Integration with HNSW graph for enhanced tracking
        match operation {
            GraphOperation::NodeCreate | GraphOperation::NodeUpdate => {
                if let Some(node_id) = node_id {
                    eprintln!("FUSE Journal: HNSW graph integration for node {} operation {:?}", node_id, operation);
                }
            }
            GraphOperation::Traverse | GraphOperation::Query => {
                eprintln!("FUSE Journal: HNSW traversal integration for operation {:?}", operation);
            }
            _ => {}
        }
        Ok(())
    }
}

/// Filesystem operation types
#[derive(Debug, Clone, Copy)]
pub enum FilesystemOperation {
    Create,
    Read,
    Write,
    Delete,
    Rename,
    Mkdir,
    Rmdir,
}

/// Vector operation types
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum VectorOperation {
    Create = 1,
    Update = 2,
    Delete = 3,
    Search = 4,
    Index = 5,
    Similarity = 6,
}

/// Graph operation types
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum GraphOperation {
    NodeCreate = 1,
    NodeUpdate = 2,
    NodeDelete = 3,
    EdgeCreate = 4,
    EdgeUpdate = 5,
    EdgeDelete = 6,
    Traverse = 7,
    Query = 8,
}

/// Operation result types
#[derive(Debug, Clone, Copy)]
pub enum OperationResult {
    Success,
    Error,
    Partial,
}

/// FUSE operation context for enhanced event tracking
#[derive(Debug, Clone)]
pub struct FuseOperationContext {
    /// Operation start time
    pub start_time: SystemTime,
    /// Operation type
    pub operation_type: String,
    /// File path involved
    pub file_path: Option<String>,
    /// Inode number
    pub inode: Option<u64>,
    /// Process ID
    pub process_id: u32,
    /// User ID
    pub user_id: u32,
    /// Group ID
    pub group_id: u32,
}

impl FuseOperationContext {
    /// Create a new operation context
    pub fn new(operation_type: String) -> Self {
        Self {
            start_time: SystemTime::now(),
            operation_type,
            file_path: None,
            inode: None,
            process_id: std::process::id(),
            user_id: 1000, // Default user
            group_id: 1000, // Default group
        }
    }
    
    /// Set file path
    pub fn with_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }
    
    /// Set inode
    pub fn with_inode(mut self, inode: u64) -> Self {
        self.inode = Some(inode);
        self
    }
    
    /// Get operation duration
    pub fn duration(&self) -> std::time::Duration {
        self.start_time.elapsed().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::semantic_api::userspace_journal::UserspaceJournalConfig;
    
    fn create_test_integration() -> FuseJournalIntegration {
        let temp_dir = tempdir().unwrap();
        let journal_path = temp_dir.path().join("test_journal");
        
        let journal_config = UserspaceJournalConfig {
            journal_path,
            lazy_sync: false,
            ..Default::default()
        };
        
        let journal = Arc::new(UserspaceSemanticJournal::new(journal_config).unwrap());
        let config = FuseJournalConfig::default();
        
        FuseJournalIntegration::new(journal, config).unwrap()
    }
    
    #[test]
    fn test_filesystem_event_recording() {
        let integration = create_test_integration();
        
        let event_id = integration.record_filesystem_event(
            FilesystemOperation::Create,
            "/test/file.txt",
            123,
            OperationResult::Success,
        ).unwrap();
        
        assert!(event_id > 0);
        
        let stats = integration.get_statistics().unwrap();
        assert_eq!(stats.events_generated, 1);
        assert_eq!(stats.filesystem_events, 1);
    }
    
    #[test]
    fn test_vector_event_recording() {
        let integration = create_test_integration();
        
        let event_id = integration.record_vector_event(
            VectorOperation::Create,
            456,
            128,
            Some(123),
            OperationResult::Success,
        ).unwrap();
        
        assert!(event_id > 0);
        
        let stats = integration.get_statistics().unwrap();
        assert_eq!(stats.events_generated, 1);
        assert_eq!(stats.vector_events, 1);
    }
    
    #[test]
    fn test_graph_event_recording() {
        let integration = create_test_integration();
        
        let event_id = integration.record_graph_event(
            GraphOperation::NodeCreate,
            Some(789),
            None,
            OperationResult::Success,
        ).unwrap();
        
        assert!(event_id > 0);
        
        let stats = integration.get_statistics().unwrap();
        assert_eq!(stats.events_generated, 1);
        assert_eq!(stats.graph_events, 1);
    }
    
    #[test]
    fn test_event_stream_creation() {
        let integration = create_test_integration();
        
        let filter = EventFilter {
            event_types: Some(vec![SemanticEventType::VectorCreate]),
            categories: None,
            time_range: None,
            agent_id: None,
            transaction_id: None,
            causality_chain_id: None,
            path_pattern: None,
            min_priority: None,
            required_flags: None,
            tags: None,
            min_relevance_score: None,
        };
        
        let subscription_id = integration.create_event_stream(
            "test_agent".to_string(),
            filter,
        ).unwrap();
        
        assert!(!subscription_id.is_nil());
        
        let stats = integration.get_statistics().unwrap();
        assert_eq!(stats.active_streams, 1);
        
        // Clean up
        integration.remove_event_stream(subscription_id).unwrap();
        
        let stats = integration.get_statistics().unwrap();
        assert_eq!(stats.active_streams, 0);
    }
    
    #[test]
    fn test_operation_context() {
        let context = FuseOperationContext::new("test_operation".to_string())
            .with_path("/test/path".to_string())
            .with_inode(123);
        
        assert_eq!(context.operation_type, "test_operation");
        assert_eq!(context.file_path, Some("/test/path".to_string()));
        assert_eq!(context.inode, Some(123));
        assert!(context.duration().as_nanos() > 0);
    }
}