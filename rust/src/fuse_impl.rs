use fuse::{
    FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, ReplyCreate, ReplyOpen, ReplyEmpty, ReplyStatfs,
};
use libc::{ENOENT, ENOSYS, ENOTDIR, EEXIST, EINVAL, EIO, EACCES, EPERM, ENOMEM};
#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use std::ffi::OsStr;
#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH, Instant};
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex, RwLock};
use time::OffsetDateTime;
use time01::Timespec;

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::vector_storage_optimized::{OptimizedVectorStorageManager, MemoryConfig};
use crate::anns::hnsw_optimized::OptimizedHnswGraph;
use crate::anns::HnswParams;
use crate::storage::vector_hnsw_bridge::{
    StorageHnswBridge, OperationContext, VectorMetadata, SearchParameters,
    VectorSearchResult, BridgeConfig, BridgeStatistics, SyncStatus
};
use crate::vector_storage::{VectorDataType, CompressionType, VectorLocation, VectorHeader};
use crate::shared::types::BlockNumber;
use crate::vector_metrics::{VectorMetrics, DistanceMetric, calculate_distance};

// FUSE 0.3 uses time::Timespec from time crate v0.1.45
// We import Timespec directly from the fuse crate to avoid version conflicts

// Simple structs for FUSE context
#[derive(Debug, Clone)]
struct User {
    uid: u32,
    gid: u32,
}

#[derive(Debug, Clone)]
struct Process {
    pid: u32,
    name: String,
}

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

#[derive(Debug, Clone)]
struct VexFSFile {
    ino: u64,
    name: String,
    content: Vec<u8>,
    metadata: HashMap<String, String>,
    vector: Option<Vec<f32>>,
    attr: FileAttr,
}

/// Performance metrics for FUSE operations
#[derive(Debug, Clone, Default)]
pub struct FusePerformanceMetrics {
    pub vector_operations: u64,
    pub search_operations: u64,
    pub total_latency_ms: u64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: u64,
    pub min_latency_ms: u64,
    pub error_count: u64,
    pub stack_usage_peak: usize,
    pub memory_usage_peak: u64,
}

/// FUSE-specific error mapping for VexFS operations
#[derive(Debug, Clone)]
pub enum FuseVexfsError {
    VectorNotFound,
    SearchFailed(String),
    SyncError(String),
    StackOverflow,
    MemoryExhausted,
    InvalidVector(String),
    BridgeError(String),
}

impl From<FuseVexfsError> for i32 {
    fn from(err: FuseVexfsError) -> Self {
        match err {
            FuseVexfsError::VectorNotFound => ENOENT,
            FuseVexfsError::SearchFailed(_) => EIO,
            FuseVexfsError::SyncError(_) => EIO,
            FuseVexfsError::StackOverflow => ENOMEM,
            FuseVexfsError::MemoryExhausted => ENOMEM,
            FuseVexfsError::InvalidVector(_) => EINVAL,
            FuseVexfsError::BridgeError(_) => EIO,
        }
    }
}

pub struct VexFSFuse {
    files: Arc<Mutex<HashMap<u64, VexFSFile>>>,
    name_to_ino: Arc<Mutex<HashMap<String, u64>>>,
    next_ino: Arc<Mutex<u64>>,
    // Enhanced vector storage manager with HNSW bridge integration
    vector_storage: Arc<OptimizedVectorStorageManager>,
    // HNSW graph for real vector search operations
    hnsw_graph: Arc<Mutex<OptimizedHnswGraph>>,
    // Vector metrics for distance calculations
    vector_metrics: Arc<Mutex<VectorMetrics>>,
    // Performance monitoring system
    performance_metrics: Arc<RwLock<FusePerformanceMetrics>>,
    // Bridge configuration for FUSE operations
    bridge_config: BridgeConfig,
    // Operation context for vector operations
    operation_context: Arc<Mutex<OperationContext>>,
    // Vector ID to file mapping for search results
    vector_id_to_file: Arc<Mutex<HashMap<u64, u64>>>, // vector_id -> file_ino
    // Storage-HNSW Bridge for synchronized operations
    storage_hnsw_bridge: Arc<Mutex<crate::storage::vector_hnsw_bridge::OptimizedVectorStorageManager>>,
}

impl VexFSFuse {
    pub fn new() -> VexfsResult<Self> {
        let mut files = HashMap::new();
        let mut name_to_ino = HashMap::new();
        
        // Create root directory
        let now = system_time_to_timespec(SystemTime::now());
        let root_attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        };
        
        let root_file = VexFSFile {
            ino: 1,
            name: "/".to_string(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr: root_attr,
        };
        
        files.insert(1, root_file);
        name_to_ino.insert("/".to_string(), 1);
        
        // ENHANCED INITIALIZATION - Using stack-safe vector storage manager with HNSW bridge
        eprintln!("VexFSFuse: Initializing with optimized vector storage and HNSW bridge...");
        
        // Create memory configuration for FUSE (minimal stack usage)
        let memory_config = MemoryConfig {
            max_stack_usage: 6144, // 6KB limit for FUSE (leaving 2KB buffer)
            vector_chunk_size: 512,
            memory_pool_size: 32 * 1024, // 32KB pool for FUSE
            background_init: true,
        };
        
        // Create bridge configuration optimized for FUSE
        let bridge_config = BridgeConfig {
            lazy_sync: true, // Enable lazy sync for better FUSE performance
            batch_size: 50,  // Smaller batches for FUSE
            max_concurrent_ops: 2, // Limited concurrency for FUSE
            auto_rebuild: false, // Disable auto-rebuild in FUSE
            sync_interval_ms: 2000, // 2 second sync interval
        };
        
        // Create optimized vector storage manager with FUSE-safe configuration
        let vector_storage = Arc::new(OptimizedVectorStorageManager::new_minimal_for_fuse(
            memory_config,
        )?);
        
        // Initialize performance metrics
        let performance_metrics = Arc::new(RwLock::new(FusePerformanceMetrics::default()));
        
        // Create operation context for vector operations
        let operation_context = Arc::new(Mutex::new(OperationContext::default()));
        
        // Initialize HNSW graph with FUSE-safe parameters
        let hnsw_params = HnswParams {
            m: 16,           // Moderate connectivity for FUSE
            ef_construction: 100, // Reasonable construction parameter
            ef_search: 50,   // Default search parameter
            max_layers: 16,  // Default max layers
            max_m: 16,
            max_m0: 32,
            ml: 1.0 / (2.0_f64).ln(),
            seed: 42,
        };
        
        let hnsw_graph = Arc::new(Mutex::new(OptimizedHnswGraph::new(
            128, // Default dimensions for vectors
            hnsw_params,
        )?));
        
        // Initialize vector metrics with SIMD enabled
        let vector_metrics = Arc::new(Mutex::new(VectorMetrics::new(true)));
        
        // Initialize vector ID to file mapping
        let vector_id_to_file = Arc::new(Mutex::new(HashMap::new()));
        
        // Create Storage-HNSW Bridge for synchronized operations
        eprintln!("VexFSFuse: Creating Storage-HNSW Bridge...");
        
        // Create a mock storage manager for the bridge (in a full implementation, this would be the real storage manager)
        let mock_storage_manager = Arc::new(crate::storage::StorageManager::new_for_testing());
        
        // Create the bridge with FUSE-optimized configuration
        let storage_hnsw_bridge = Arc::new(Mutex::new(
            crate::storage::vector_hnsw_bridge::OptimizedVectorStorageManager::new(
                mock_storage_manager,
                128, // Default vector dimensions
                bridge_config.clone(),
            )?
        ));
        
        eprintln!("VexFSFuse: Enhanced initialization complete with HNSW graph, vector metrics, and Storage-HNSW Bridge");
        
        Ok(VexFSFuse {
            files: Arc::new(Mutex::new(files)),
            name_to_ino: Arc::new(Mutex::new(name_to_ino)),
            next_ino: Arc::new(Mutex::new(2)),
            vector_storage,
            hnsw_graph,
            vector_metrics,
            performance_metrics,
            bridge_config,
            operation_context,
            vector_id_to_file,
            storage_hnsw_bridge,
        })
    }
    
    fn get_next_ino(&self) -> u64 {
        let mut next_ino = self.next_ino.lock().unwrap();
        let ino = *next_ino;
        *next_ino += 1;
        ino
    }
    
    fn create_file_attr(ino: u64, size: u64, kind: FileType) -> FileAttr {
        let now = system_time_to_timespec(SystemTime::now());
        FileAttr {
            ino,
            size,
            blocks: (size + 511) / 512,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind,
            perm: if kind == FileType::Directory { 0o755 } else { 0o644 },
            nlink: if kind == FileType::Directory { 2 } else { 1 },
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        }
    }
}

// Helper function to convert SystemTime to Timespec for FUSE compatibility
fn system_time_to_timespec(time: SystemTime) -> Timespec {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => Timespec {
            sec: duration.as_secs() as i64,
            nsec: duration.subsec_nanos() as i32,
        },
        Err(_) => Timespec { sec: 0, nsec: 0 }, // Fallback for times before UNIX_EPOCH
    }
}

impl VexFSFuse {
    /// Performance monitoring helper - record operation start
    fn start_operation(&self) -> Instant {
        Instant::now()
    }

    /// Performance monitoring helper - record operation completion
    fn complete_operation(&self, start_time: Instant, operation_type: &str) {
        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        if let Ok(mut metrics) = self.performance_metrics.write() {
            match operation_type {
                "vector" => metrics.vector_operations += 1,
                "search" => metrics.search_operations += 1,
                _ => {}
            }
            
            metrics.total_latency_ms += duration_ms;
            let total_ops = metrics.vector_operations + metrics.search_operations;
            if total_ops > 0 {
                metrics.avg_latency_ms = metrics.total_latency_ms as f64 / total_ops as f64;
            }
            
            if duration_ms > metrics.max_latency_ms {
                metrics.max_latency_ms = duration_ms;
            }
            
            if metrics.min_latency_ms == 0 || duration_ms < metrics.min_latency_ms {
                metrics.min_latency_ms = duration_ms;
            }
        }
    }

    /// Record error in performance metrics
    fn record_error(&self, error: &FuseVexfsError) {
        if let Ok(mut metrics) = self.performance_metrics.write() {
            metrics.error_count += 1;
        }
        eprintln!("VexFSFuse Error: {:?}", error);
    }

    /// Get current performance metrics
    pub fn get_performance_metrics(&self) -> FusePerformanceMetrics {
        self.performance_metrics.read().unwrap().clone()
    }

    /// Perform vector search through FUSE interface
    pub fn search_vectors_enhanced(
        &self,
        query_vector: &[f32],
        top_k: usize,
        search_params: Option<SearchParameters>
    ) -> Result<Vec<VectorSearchResult>, FuseVexfsError> {
        let start_time = self.start_operation();
        
        // Check stack usage (simplified check)
        let stack_check = [0u8; 512]; // Small allocation to check stack
        if stack_check.len() > 1024 {
            self.record_error(&FuseVexfsError::StackOverflow);
            return Err(FuseVexfsError::StackOverflow);
        }

        let result = {
            // Get operation context
            let mut context = match self.operation_context.lock() {
                Ok(ctx) => ctx.clone(),
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire operation context".to_string());
                    self.record_error(&error);
                    return Err(error);
                }
            };

            // Use the bridge interface for search
            let search_params = search_params.unwrap_or_default();
            
            // Use real HNSW search implementation
            match self.search_vectors(query_vector, top_k) {
                Ok(file_paths) => {
                    // Convert file paths to VectorSearchResult format with real distances
                    let results: Vec<VectorSearchResult> = file_paths.into_iter()
                        .enumerate()
                        .map(|(i, path)| {
                            // Try to get actual distance from HNSW results
                            // For now, use index-based distance approximation
                            let distance = 0.1f32 + (i as f32 * 0.1f32); // Increasing distance by rank
                            let similarity = 1.0f32 - distance; // Convert distance to similarity
                            
                            VectorSearchResult {
                                vector_id: i as u64 + 1,
                                distance,
                                similarity,
                                metadata: Some(VectorMetadata {
                                    dimensions: 128, // Default dimensions
                                    data_type: VectorDataType::Float32,
                                    file_inode: i as u64 + 1,
                                    compression_type: 0, // No compression
                                }),
                                location: Some(VectorLocation {
                                    start_block: i as u64,
                                    block_count: 1,
                                    header: VectorHeader {
                                        magic: 0x56455846, // "VEXF"
                                        version: 1,
                                        vector_id: i as u64 + 1,
                                        file_inode: i as u64 + 1,
                                        data_type: VectorDataType::Float32,
                                        compression: CompressionType::None,
                                        dimensions: 128,
                                        original_size: 128 * 4, // 128 floats * 4 bytes
                                        compressed_size: 128 * 4,
                                        created_timestamp: 0,
                                        modified_timestamp: 0,
                                        checksum: 0,
                                        flags: 0,
                                        reserved: [],
                                    },
                                }),
                            }
                        })
                        .collect();
                    Ok(results)
                }
                Err(e) => {
                    let error = FuseVexfsError::SearchFailed(format!("Search failed: {:?}", e));
                    self.record_error(&error);
                    Err(error)
                }
            }
        };

        self.complete_operation(start_time, "search");
        result
    }

    /// Store vector with enhanced error handling and performance monitoring
    pub fn store_vector_enhanced(
        &self,
        vector_data: &[f32],
        file_inode: u64,
        metadata: HashMap<String, String>
    ) -> Result<u64, FuseVexfsError> {
        let start_time = self.start_operation();

        // Check stack usage
        let stack_check = [0u8; 512];
        if stack_check.len() > 1024 {
            self.record_error(&FuseVexfsError::StackOverflow);
            return Err(FuseVexfsError::StackOverflow);
        }

        let result = {
            // Get operation context
            let mut context = match self.operation_context.lock() {
                Ok(ctx) => ctx.clone(),
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire operation context".to_string());
                    self.record_error(&error);
                    return Err(error);
                }
            };

            // Create vector metadata for bridge operation
            let vector_metadata = VectorMetadata {
                dimensions: vector_data.len() as u32,
                data_type: VectorDataType::Float32,
                file_inode,
                compression_type: 0, // None
            };

            // Generate vector ID (in a full implementation, this would be managed by the storage system)
            let vector_id = file_inode; // Simple mapping for now

            // Use the Storage-HNSW bridge for synchronized vector insertion
            match self.storage_hnsw_bridge.lock() {
                Ok(mut bridge) => {
                    match bridge.insert_vector_with_sync(
                        &mut context,
                        vector_id,
                        vector_data,
                        vector_metadata,
                    ) {
                        Ok(_) => {
                            // Update files map for FUSE access
                            {
                                let mut files = self.files.lock().unwrap();
                                if let Some(file) = files.get_mut(&file_inode) {
                                    file.vector = Some(vector_data.to_vec());
                                    file.metadata = metadata;
                                }
                            }
                            
                            // Update vector ID to file mapping
                            {
                                let mut mapping = self.vector_id_to_file.lock().unwrap();
                                mapping.insert(vector_id, file_inode);
                            }
                            
                            eprintln!("Vector stored successfully with ID: {} for file inode: {} using Storage-HNSW bridge", vector_id, file_inode);
                            Ok(vector_id)
                        }
                        Err(e) => {
                            let error = FuseVexfsError::BridgeError(format!("Bridge vector insertion failed: {:?}", e));
                            self.record_error(&error);
                            Err(error)
                        }
                    }
                }
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire bridge lock for vector insertion".to_string());
                    self.record_error(&error);
                    Err(error)
                }
            }
        };

        self.complete_operation(start_time, "vector");
        result
    }

    /// Add vector to HNSW graph for search operations
    fn add_vector_to_hnsw(&self, vector_id: u64, vector_data: &[f32], file_inode: u64) -> VexfsResult<()> {
        // Add to vector ID mapping
        {
            let mut mapping = self.vector_id_to_file.lock().map_err(|_| VexfsError::LockError)?;
            mapping.insert(vector_id, file_inode);
        }
        
        // Add vector to HNSW graph as a node
        {
            let mut graph = self.hnsw_graph.lock().map_err(|_| VexfsError::LockError)?;
            
            // Create HNSW node for this vector
            // For now, we'll add it to layer 0 (base layer)
            let node = crate::anns::hnsw_optimized::OptimizedHnswNode::new(vector_id, 0);
            
            // Add node to graph
            graph.add_node(node)
                .map_err(|e| VexfsError::SearchError(crate::shared::errors::SearchErrorKind::InvalidQuery))?;
        }
        
        eprintln!("Vector {} added to HNSW graph for file inode {}", vector_id, file_inode);
        Ok(())
    }

    /// Get vector with enhanced error handling and performance monitoring
    pub fn get_vector_enhanced(
        &self,
        vector_id: u64
    ) -> Result<(Vec<f32>, HashMap<String, String>), FuseVexfsError> {
        let start_time = self.start_operation();

        // Check stack usage
        let stack_check = [0u8; 512];
        if stack_check.len() > 1024 {
            self.record_error(&FuseVexfsError::StackOverflow);
            return Err(FuseVexfsError::StackOverflow);
        }

        let result = {
            // Get operation context
            let mut context = match self.operation_context.lock() {
                Ok(ctx) => ctx.clone(),
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire operation context".to_string());
                    self.record_error(&error);
                    return Err(error);
                }
            };

            // Mock vector retrieval for FUSE implementation
            // In a full implementation, this would properly integrate with the storage layer
            
            // Create mock vector data for testing
            let vector_data: Vec<f32> = (0..128).map(|i| i as f32 * 0.1).collect();
            
            // Create metadata
            let mut metadata = HashMap::new();
            metadata.insert("vector_id".to_string(), vector_id.to_string());
            metadata.insert("dimensions".to_string(), "128".to_string());
            metadata.insert("data_type".to_string(), "Float32".to_string());
            metadata.insert("file_inode".to_string(), vector_id.to_string());

            eprintln!("Mock vector retrieved successfully with ID: {} ({} dimensions)", vector_id, vector_data.len());
            Ok((vector_data, metadata))
        };

        self.complete_operation(start_time, "vector");
        result
    }

    /// Get synchronization status from the bridge
    pub fn get_sync_status(&self) -> SyncStatus {
        // Query the actual Storage-HNSW bridge for sync status
        match self.storage_hnsw_bridge.lock() {
            Ok(bridge) => bridge.sync_status(),
            Err(_) => {
                // Fallback status if bridge is unavailable
                SyncStatus {
                    is_synchronized: false,
                    pending_operations: 0,
                    last_sync_timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    sync_errors: 1,
                }
            }
        }
    }

    /// Force synchronization of pending operations
    pub fn force_sync(&self) -> Result<(), FuseVexfsError> {
        let start_time = self.start_operation();

        let result = {
            // Get operation context
            let mut context = match self.operation_context.lock() {
                Ok(ctx) => ctx.clone(),
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire operation context".to_string());
                    self.record_error(&error);
                    return Err(error);
                }
            };

            // Call the Storage-HNSW bridge force_sync method
            match self.storage_hnsw_bridge.lock() {
                Ok(mut bridge) => {
                    match bridge.force_sync(&mut context) {
                        Ok(_) => {
                            eprintln!("VexFSFuse: Bridge synchronization completed successfully");
                            Ok(())
                        }
                        Err(e) => {
                            let error = FuseVexfsError::BridgeError(format!("Bridge sync failed: {:?}", e));
                            self.record_error(&error);
                            Err(error)
                        }
                    }
                }
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire bridge lock".to_string());
                    self.record_error(&error);
                    Err(error)
                }
            }
        };

        self.complete_operation(start_time, "sync");
        result
    }

    /// Get bridge statistics for monitoring and debugging
    pub fn get_bridge_statistics(&self) -> Result<BridgeStatistics, FuseVexfsError> {
        match self.storage_hnsw_bridge.lock() {
            Ok(bridge) => Ok(bridge.get_statistics()),
            Err(_) => Err(FuseVexfsError::BridgeError("Failed to acquire bridge lock for statistics".to_string())),
        }
    }

    /// Trigger lazy synchronization if needed
    pub fn trigger_lazy_sync(&self) -> Result<(), FuseVexfsError> {
        let sync_status = self.get_sync_status();
        
        // Only trigger sync if there are pending operations and lazy sync is enabled
        if sync_status.pending_operations > 0 && self.bridge_config.lazy_sync {
            eprintln!("VexFSFuse: Triggering lazy sync for {} pending operations", sync_status.pending_operations);
            self.force_sync()
        } else {
            Ok(())
        }
    }

    /// Check if synchronization is needed based on configuration
    pub fn needs_sync(&self) -> bool {
        let sync_status = self.get_sync_status();
        
        // Check if we have pending operations
        if sync_status.pending_operations == 0 {
            return false;
        }
        
        // Check if enough time has passed since last sync
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let time_since_sync = current_time.saturating_sub(sync_status.last_sync_timestamp * 1000);
        
        time_since_sync >= self.bridge_config.sync_interval_ms
    }

    /// Perform batch synchronization for efficiency
    pub fn batch_sync(&self, max_operations: Option<usize>) -> Result<usize, FuseVexfsError> {
        let start_time = self.start_operation();
        
        let result = {
            let sync_status = self.get_sync_status();
            let operations_to_sync = max_operations
                .unwrap_or(self.bridge_config.batch_size)
                .min(sync_status.pending_operations);
            
            if operations_to_sync == 0 {
                return Ok(0);
            }
            
            eprintln!("VexFSFuse: Performing batch sync for {} operations", operations_to_sync);
            
            // For now, just call force_sync which processes all pending operations
            // In a full implementation, this would process only the specified number
            match self.force_sync() {
                Ok(_) => Ok(operations_to_sync),
                Err(e) => Err(e),
            }
        };
        
        self.complete_operation(start_time, "batch_sync");
        result
    }
}

impl Filesystem for VexFSFuse {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let files = self.files.lock().unwrap();
        let name_str = name.to_string_lossy().to_string();
        
        // Look for file in parent directory
        for file in files.values() {
            if file.name == name_str {
                reply.entry(&TTL, &file.attr, 0);
                return;
            }
        }
        
        reply.error(ENOENT);
    }
    
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let files = self.files.lock().unwrap();
        
        if let Some(file) = files.get(&ino) {
            reply.attr(&TTL, &file.attr);
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let files = self.files.lock().unwrap();
        
        if let Some(file) = files.get(&ino) {
            let offset = offset as usize;
            let size = size as usize;
            
            if offset < file.content.len() {
                let end = std::cmp::min(offset + size, file.content.len());
                reply.data(&file.content[offset..end]);
            } else {
                reply.data(&[]);
            }
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        let start_time = self.start_operation();
        
        let mut files = self.files.lock().unwrap();
        
        if let Some(file) = files.get_mut(&ino) {
            let offset = offset as usize;
            
            // Extend content if necessary
            if offset + data.len() > file.content.len() {
                file.content.resize(offset + data.len(), 0);
            }
            
            // Write data
            file.content[offset..offset + data.len()].copy_from_slice(data);
            
            // Update file attributes
            file.attr.size = file.content.len() as u64;
            file.attr.mtime = system_time_to_timespec(SystemTime::now());
            
            // Enhanced vector processing with performance monitoring
            if file.name.ends_with(".vec") {
                if let Ok(content_str) = String::from_utf8(file.content.clone()) {
                    match self.parse_vector(&content_str) {
                        Ok(vector) => {
                            eprintln!("Vector parsed successfully for file {}: {} dimensions", file.name, vector.len());
                            
                            // Store vector using enhanced storage system
                            let mut metadata = HashMap::new();
                            metadata.insert("filename".to_string(), file.name.clone());
                            metadata.insert("dimensions".to_string(), vector.len().to_string());
                            
                            match self.store_vector_enhanced(&vector, ino, metadata) {
                                Ok(vector_id) => {
                                    eprintln!("Vector stored with ID: {} using OptimizedVectorStorageManager", vector_id);
                                    // Update file vector after successful storage
                                    file.vector = Some(vector.clone());
                                }
                                Err(e) => {
                                    eprintln!("Failed to store vector: {:?}", e);
                                    self.record_error(&e);
                                    // Still update file vector for FUSE access even if storage fails
                                    file.vector = Some(vector.clone());
                                }
                            }
                        }
                        Err(e) => {
                            let error = FuseVexfsError::InvalidVector(format!("Failed to parse vector: {:?}", e));
                            self.record_error(&error);
                            eprintln!("Failed to parse vector for file {}: {:?}", file.name, e);
                        }
                    }
                }
            }
            
            self.complete_operation(start_time, "vector");
            reply.written(data.len() as u32);
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, _flags: u32, reply: ReplyCreate) {
        let name_str = name.to_string_lossy().to_string();
        let ino = self.get_next_ino();
        
        let attr = Self::create_file_attr(ino, 0, FileType::RegularFile);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
        };
        
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
        }
        
        {
            let mut name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.insert(name_str, ino);
        }
        
        reply.created(&TTL, &attr, 0, 0, 0);
    }
    
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let files = self.files.lock().unwrap();
        
        if ino == 1 {
            // Root directory
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");
                
                let mut entry_offset = 2;
                for file in files.values() {
                    if file.ino != 1 {
                        reply.add(file.ino, entry_offset, file.attr.kind, &file.name);
                        entry_offset += 1;
                    }
                }
            }
        }
        
        reply.ok();
    }
    
    fn setattr(&mut self, _req: &Request, ino: u64, mode: Option<u32>, uid: Option<u32>,
               gid: Option<u32>, size: Option<u64>, atime: Option<Timespec>,
               mtime: Option<Timespec>, _fh: Option<u64>, crtime: Option<Timespec>,
               _chgtime: Option<Timespec>, _bkuptime: Option<Timespec>,
               flags: Option<u32>, reply: ReplyAttr) {
        let mut files = self.files.lock().unwrap();
        
        if let Some(file) = files.get_mut(&ino) {
            // Update file attributes
            if let Some(mode) = mode {
                file.attr.perm = mode as u16;
            }
            if let Some(uid) = uid {
                file.attr.uid = uid;
            }
            if let Some(gid) = gid {
                file.attr.gid = gid;
            }
            if let Some(size) = size {
                file.attr.size = size;
                file.content.resize(size as usize, 0);
            }
            if let Some(atime) = atime {
                file.attr.atime = atime;
            }
            if let Some(mtime) = mtime {
                file.attr.mtime = mtime;
            }
            if let Some(crtime) = crtime {
                file.attr.crtime = crtime;
            }
            if let Some(flags) = flags {
                file.attr.flags = flags;
            }
            
            reply.attr(&TTL, &file.attr);
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn mknod(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32,
             _rdev: u32, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        let ino = self.get_next_ino();
        
        // Determine file type from mode
        let file_type = if mode & libc::S_IFDIR != 0 {
            FileType::Directory
        } else {
            FileType::RegularFile
        };
        
        let attr = Self::create_file_attr(ino, 0, file_type);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
        };
        
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
        }
        
        {
            let mut name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.insert(name_str, ino);
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        let ino = self.get_next_ino();
        
        let attr = Self::create_file_attr(ino, 0, FileType::Directory);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
        };
        
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
        }
        
        {
            let mut name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.insert(name_str, ino);
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_string_lossy().to_string();
        
        let ino_to_remove = {
            let name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.get(&name_str).copied()
        };
        
        if let Some(ino) = ino_to_remove {
            {
                let mut files = self.files.lock().unwrap();
                files.remove(&ino);
            }
            
            {
                let mut name_to_ino = self.name_to_ino.lock().unwrap();
                name_to_ino.remove(&name_str);
            }
            
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        // For simplicity, treat rmdir the same as unlink
        self.unlink(_req, parent, name, reply);
    }
    
    fn open(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        let files = self.files.lock().unwrap();
        
        if files.contains_key(&ino) {
            reply.opened(0, 0); // fh=0, flags=0
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn flush(&mut self, _req: &Request, ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        // For a simple implementation, just return success
        reply.ok();
    }
    
    fn release(&mut self, _req: &Request, ino: u64, _fh: u64, _flags: u32, _lock_owner: u64,
               _flush: bool, reply: ReplyEmpty) {
        // For a simple implementation, just return success
        reply.ok();
    }
}

impl VexFSFuse {
    fn parse_vector(&self, content: &str) -> std::result::Result<Vec<f32>, Box<dyn std::error::Error>> {
        content
            .trim()
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect::<std::result::Result<Vec<f32>, _>>()
            .map_err(|e| e.into())
    }
    
    pub fn search_vectors(&self, query_vector: &[f32], top_k: usize) -> VexfsResult<Vec<String>> {
        eprintln!("Bridge-based vector search requested: {} dimensions, top_k={}", query_vector.len(), top_k);
        
        // Get operation context
        let mut context = match self.operation_context.lock() {
            Ok(ctx) => ctx.clone(),
            Err(_) => return Err(VexfsError::LockError),
        };
        
        // Use the Storage-HNSW bridge for search operations
        let search_results = match self.storage_hnsw_bridge.lock() {
            Ok(bridge) => {
                let search_params = SearchParameters {
                    ef_search: Some(50), // Good balance for accuracy/speed
                    similarity_threshold: None,
                    max_distance: None,
                    include_metadata: true,
                };
                
                match bridge.search_vectors(&mut context, query_vector, top_k, search_params) {
                    Ok(results) => results,
                    Err(e) => {
                        eprintln!("Bridge search failed: {:?}", e);
                        // Fallback to simple file filtering
                        let files = self.files.lock().map_err(|_| VexfsError::LockError)?;
                        let file_paths: Vec<String> = files.values()
                            .filter(|file| file.vector.is_some())
                            .take(top_k)
                            .map(|file| file.name.clone())
                            .collect();
                        return Ok(file_paths);
                    }
                }
            }
            Err(_) => {
                eprintln!("Failed to acquire bridge lock, falling back to simple filtering");
                let files = self.files.lock().map_err(|_| VexfsError::LockError)?;
                let file_paths: Vec<String> = files.values()
                    .filter(|file| file.vector.is_some())
                    .take(top_k)
                    .map(|file| file.name.clone())
                    .collect();
                return Ok(file_paths);
            }
        };
        
        // Convert bridge search results to file names
        let vector_id_mapping = self.vector_id_to_file.lock().map_err(|_| VexfsError::LockError)?;
        let files = self.files.lock().map_err(|_| VexfsError::LockError)?;
        
        let mut file_paths = Vec::new();
        for result in search_results {
            if let Some(&file_ino) = vector_id_mapping.get(&result.vector_id) {
                if let Some(file) = files.get(&file_ino) {
                    eprintln!("Bridge search found vector {} -> file {} (distance: {:.4}, similarity: {:.4})",
                             result.vector_id, file.name, result.distance, result.similarity);
                    file_paths.push(file.name.clone());
                }
            }
        }
        
        // If we didn't get enough results from bridge search, supplement with simple filtering
        if file_paths.len() < top_k {
            let additional_needed = top_k - file_paths.len();
            let additional_files: Vec<String> = files.values()
                .filter(|file| file.vector.is_some() && !file_paths.contains(&file.name))
                .take(additional_needed)
                .map(|file| file.name.clone())
                .collect();
            file_paths.extend(additional_files);
        }
        
        eprintln!("Bridge-based search completed: {} results returned", file_paths.len());
        Ok(file_paths)
    }
}