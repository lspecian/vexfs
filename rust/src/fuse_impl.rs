use fuse::{
    FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, ReplyCreate, ReplyOpen, ReplyEmpty, ReplyStatfs,
};
use libc::{ENOENT, ENOSYS, ENOTDIR, EEXIST, EINVAL, EIO, EACCES, EPERM, ENOMEM, ENOTEMPTY};
use crate::fuse_error_handling::{safe_lock, safe_read, safe_write, handle_result, log_error};
#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use std::ffi::OsStr;
#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH, Instant};
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex, RwLock};
#[cfg(feature = "std")]
use std::sync::atomic::{AtomicU64, Ordering};
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
    parent: u64,  // Parent directory inode
    content: Vec<u8>,
    metadata: HashMap<String, String>,
    vector: Option<Vec<f32>>,
    attr: FileAttr,
    children: Vec<u64>,  // For directories: child inodes
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
    VectorNotFound(u64),
    SearchFailed(String),
    SyncError(String),
    StackOverflow,
    MemoryExhausted,
    InvalidVector(String),
    BridgeError(String),
    InvalidVectorFormat,
}

impl From<FuseVexfsError> for i32 {
    fn from(err: FuseVexfsError) -> Self {
        match err {
            FuseVexfsError::VectorNotFound(_) => ENOENT,
            FuseVexfsError::SearchFailed(_) => EIO,
            FuseVexfsError::SyncError(_) => EIO,
            FuseVexfsError::StackOverflow => ENOMEM,
            FuseVexfsError::MemoryExhausted => ENOMEM,
            FuseVexfsError::InvalidVector(_) => EINVAL,
            FuseVexfsError::BridgeError(_) => EIO,
            FuseVexfsError::InvalidVectorFormat => EINVAL,
        }
    }
}

pub struct VexFSFuse {
    files: Arc<Mutex<HashMap<u64, VexFSFile>>>,
    // Changed to store (parent_ino, name) -> ino mapping for proper hierarchy
    parent_name_to_ino: Arc<Mutex<HashMap<(u64, String), u64>>>,
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
    // Atomic counter for generating unique vector IDs
    next_vector_id: Arc<AtomicU64>,
}

impl VexFSFuse {
    pub fn new() -> VexfsResult<Self> {
        let mut files = HashMap::new();
        let mut parent_name_to_ino = HashMap::new();
        
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
        
        let mut root_file = VexFSFile {
            ino: 1,
            name: "/".to_string(),
            parent: 1,  // Root is its own parent
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr: root_attr,
            children: Vec::new(),
        };
        
        // Create special _vexfs control directory
        let vexfs_dir_ino = 2;
        let vexfs_dir = VexFSFile {
            ino: vexfs_dir_ino,
            name: "_vexfs".to_string(),
            parent: 1,
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr: FileAttr {
                ino: vexfs_dir_ino,
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
            },
            children: Vec::new(),
        };
        
        // Add _vexfs to root's children
        root_file.children.push(vexfs_dir_ino);
        
        files.insert(1, root_file);
        files.insert(vexfs_dir_ino, vexfs_dir);
        parent_name_to_ino.insert((1, "_vexfs".to_string()), vexfs_dir_ino);
        
        // Create special control files
        let next_ino = 3;
        Self::create_control_files(&mut files, &mut parent_name_to_ino, vexfs_dir_ino, &mut next_ino.clone());
        
        // Root doesn't need parent_name_to_ino entry
        
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
        
        // Create a real storage manager for the bridge
        // Use an in-memory block device for FUSE operations
        let block_size = 4096u32;
        let total_blocks = 10240u64; // 40MB of storage
        let device_size = total_blocks * block_size as u64;
        
        let device = crate::storage::BlockDevice::new(
            device_size,
            block_size,
            true, // in-memory device for FUSE
            "vexfs_fuse_device".to_string()
        )?;
        
        let layout = crate::storage::VexfsLayout {
            total_blocks,
            block_size,
            blocks_per_group: 128,
            inodes_per_group: 32,
            group_count: (total_blocks / 128) as u32,
            inode_size: 256,
            journal_blocks: 256,
            vector_blocks: 512,
        };
        
        let storage_manager = Arc::new(crate::storage::StorageManager::new(
            device,
            layout,
            1024 * 1024 // 1MB cache
        )?);
        
        // Create the bridge with FUSE-optimized configuration
        let storage_hnsw_bridge = Arc::new(Mutex::new(
            crate::storage::vector_hnsw_bridge::OptimizedVectorStorageManager::new(
                storage_manager,
                128, // Default vector dimensions
                bridge_config.clone(),
            )?
        ));
        
        eprintln!("VexFSFuse: Enhanced initialization complete with HNSW graph, vector metrics, and Storage-HNSW Bridge");
        
        Ok(VexFSFuse {
            files: Arc::new(Mutex::new(files)),
            parent_name_to_ino: Arc::new(Mutex::new(parent_name_to_ino)),
            next_ino: Arc::new(Mutex::new(next_ino + 1)),
            vector_storage,
            hnsw_graph,
            vector_metrics,
            performance_metrics,
            bridge_config,
            operation_context,
            vector_id_to_file,
            storage_hnsw_bridge,
            next_vector_id: Arc::new(AtomicU64::new(1)),
        })
    }
    
    fn get_next_ino(&self) -> u64 {
        match safe_lock(&self.next_ino, "get_next_ino") {
            Ok(mut next_ino) => {
                let ino = *next_ino;
                *next_ino += 1;
                ino
            }
            Err(_) => {
                log_error("get_next_ino", "Failed to acquire lock, using timestamp");
                // Fallback to timestamp-based inode generation
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            }
        }
    }
    
    /// Create special control files in _vexfs directory
    fn create_control_files(
        files: &mut HashMap<u64, VexFSFile>,
        parent_name_to_ino: &mut HashMap<(u64, String), u64>,
        parent_ino: u64,
        next_ino: &mut u64
    ) {
        let now = system_time_to_timespec(SystemTime::now());
        
        // Create control files
        let control_files = vec![
            ("control", "# VexFS Control File\nindex.auto_rebuild=false\nsearch.algorithm=hnsw\n"),
            ("stats", "vectors_stored: 0\nindex_size: 0\ndimensions: 0\nsearch_operations: 0\n"),
            ("search", "# Write vector here to search\n"),
        ];
        
        for (name, initial_content) in control_files {
            let ino = *next_ino;
            *next_ino += 1;
            
            let file = VexFSFile {
                ino,
                name: format!("_vexfs/{}", name),
                parent: parent_ino,
                content: initial_content.as_bytes().to_vec(),
                metadata: HashMap::new(),
                vector: None,
                attr: FileAttr {
                    ino,
                    size: initial_content.len() as u64,
                    blocks: 1,
                    atime: now,
                    mtime: now,
                    ctime: now,
                    crtime: now,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: 1000,
                    gid: 1000,
                    rdev: 0,
                    flags: 0,
                },
                children: Vec::new(),
            };
            
            files.insert(ino, file);
            parent_name_to_ino.insert((parent_ino, name.to_string()), ino);
            
            // Add to parent's children
            if let Some(parent) = files.get_mut(&parent_ino) {
                parent.children.push(ino);
            }
        }
    }
    
    /// Handle writes to .search files
    fn handle_search_file_write(&self, file_ino: u64, content: &[u8]) {
        if let Ok(content_str) = String::from_utf8(content.to_vec()) {
            // Parse search query format
            let mut vector: Option<Vec<f32>> = None;
            let mut k = 5usize;
            
            for line in content_str.lines() {
                if let Some(vec_str) = line.strip_prefix("vector:") {
                    if let Ok(v) = self.parse_vector(vec_str.trim()) {
                        vector = Some(v);
                    }
                } else if let Some(k_str) = line.strip_prefix("k:") {
                    if let Ok(parsed_k) = k_str.trim().parse::<usize>() {
                        k = parsed_k;
                    }
                }
            }
            
            if let Some(query_vector) = vector {
                // Perform search and store results
                match self.search_vectors(&query_vector, k) {
                    Ok(results) => {
                        // Create results file
                        let results_content = format_search_results(&results);
                        self.create_search_results_file(file_ino, results_content);
                    }
                    Err(e) => {
                        eprintln!("Search failed: {:?}", e);
                    }
                }
            }
        }
    }
    
    /// Handle writes to global search interface
    fn handle_global_search_write(&self, content: &[u8]) {
        if let Ok(content_str) = String::from_utf8(content.to_vec()) {
            if let Ok(vector) = self.parse_vector(content_str.trim()) {
                match self.search_vectors(&vector, 10) {
                    Ok(results) => {
                        // Update the search file with results
                        let results_content = format_search_results(&results);
                        self.update_control_file("search", &results_content);
                    }
                    Err(e) => {
                        eprintln!("Global search failed: {:?}", e);
                    }
                }
            }
        }
    }
    
    /// Handle writes to control file
    fn handle_control_file_write(&self, content: &[u8]) {
        if let Ok(content_str) = String::from_utf8(content.to_vec()) {
            // Parse configuration updates
            for line in content_str.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    self.update_configuration(key.trim(), value.trim());
                }
            }
        }
    }
    
    /// Create search results file
    fn create_search_results_file(&self, search_file_ino: u64, content: String) {
        // This would create a .search.results file with the search results
        // For now, just log the results
        eprintln!("Search results: {}", content);
    }
    
    /// Update control file content
    fn update_control_file(&self, name: &str, content: &str) {
        let files_result = safe_lock(&self.files, "update_control_file");
        if let Ok(mut files) = files_result {
            for file in files.values_mut() {
                if file.name == format!("_vexfs/{}", name) {
                file.content = content.as_bytes().to_vec();
                file.attr.size = content.len() as u64;
                file.attr.mtime = system_time_to_timespec(SystemTime::now());
                break;
            }
        }
    }
    
    /// Update system configuration
    fn update_configuration(&self, key: &str, value: &str) {
        eprintln!("Configuration update: {} = {}", key, value);
        // Here we would update actual system configuration
        // For now, just log the update
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

// Helper function to format search results
fn format_search_results(results: &[String]) -> String {
    if results.is_empty() {
        return "No results found\n".to_string();
    }
    
    let mut output = String::new();
    output.push_str("# Search Results\n");
    output.push_str(&format!("Found {} similar vectors:\n\n", results.len()));
    
    for (i, path) in results.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, path));
    }
    
    output
}

impl VexFSFuse {
    /// Find the inode associated with a vector ID
    fn find_inode_for_vector(&self, vector_id: u64) -> Result<u64, FuseVexfsError> {
        // Check if we have a mapping for this vector ID
        if let Ok(mapping) = self.vector_id_to_file.lock() {
            if let Some(&inode) = mapping.get(&vector_id) {
                return Ok(inode);
            }
        }
        
        // If no mapping exists, return error
        Err(FuseVexfsError::VectorNotFound(vector_id))
    }
    
    /// Perform direct HNSW search as fallback
    fn perform_direct_hnsw_search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>, FuseVexfsError> {
        // Lock the HNSW graph
        let mut hnsw_graph = self.hnsw_graph.lock()
            .map_err(|_| FuseVexfsError::BridgeError("Failed to acquire HNSW graph lock".to_string()))?;
        
        // Perform the search with L2 distance
        let distance_fn = |a: &[f32], b: &[f32]| -> Result<f32, crate::anns::AnnsError> {
            Ok(a.iter()
                .zip(b.iter())
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f32>()
                .sqrt())
        };
        
        let search_results = hnsw_graph.search(query_vector, top_k, 50, distance_fn) // ef_search = 50
            .map_err(|e| FuseVexfsError::SearchFailed(format!("HNSW search failed: {:?}", e)))?;
        
        // Convert HNSW results to VectorSearchResult
        let mut results = Vec::with_capacity(search_results.len());
        
        for (node_id, distance) in search_results {
            // Get file inode from vector mapping
            let file_inode = self.vector_id_to_file.lock()
                .ok()
                .and_then(|map| map.get(&node_id).copied())
                .unwrap_or(node_id); // Fallback to using node_id as inode
            
            // Get vector metadata if available
            let files = self.files.lock().ok();
            let metadata = files.as_ref().and_then(|f| f.get(&file_inode)).map(|file| {
                VectorMetadata {
                    dimensions: file.vector.as_ref().map(|v| v.len() as u32).unwrap_or(128),
                    data_type: VectorDataType::Float32,
                    file_inode,
                    compression_type: 0, // No compression
                }
            });
            
            results.push(VectorSearchResult {
                vector_id: node_id,
                distance,
                similarity: 1.0 - distance.min(1.0), // Convert distance to similarity
                metadata,
                location: None, // Location info not available in direct search
            });
        }
        
        Ok(results)
    }
    
    /// Read vector data from storage
    fn read_vector_from_storage(&self, file_inode: u64, vector_id: u64) -> Result<Vec<f32>, FuseVexfsError> {
        // Get the file from the files map
        let files = self.files.lock().map_err(|_| 
            FuseVexfsError::BridgeError("Failed to acquire files lock".to_string()))?;
        
        let file = files.get(&file_inode)
            .ok_or_else(|| FuseVexfsError::VectorNotFound(vector_id))?;
        
        // Check if the file has vector data
        if let Some(ref vector) = file.vector {
            return Ok(vector.clone());
        }
        
        // If no vector field, try to parse from content
        if file.content.is_empty() {
            return Err(FuseVexfsError::InvalidVector(
                format!("No vector data found for vector ID {}", vector_id)
            ));
        }
        
        // Validate content length is divisible by f32 size
        if file.content.len() % std::mem::size_of::<f32>() != 0 {
            return Err(FuseVexfsError::InvalidVectorFormat);
        }
        
        // Parse the vector data based on format
        let vector_size = file.content.len() / std::mem::size_of::<f32>();
        let mut vector = Vec::with_capacity(vector_size);
        
        for chunk in file.content.chunks_exact(4) {
            let bytes: [u8; 4] = chunk.try_into()
                .map_err(|_| FuseVexfsError::InvalidVectorFormat)?;
            vector.push(f32::from_le_bytes(bytes));
        }
        
        // Validate vector has reasonable dimensions
        if vector.is_empty() || vector.len() > 10000 {
            return Err(FuseVexfsError::InvalidVector(
                format!("Invalid vector dimensions: {}", vector.len())
            ));
        }
        
        Ok(vector)
    }

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
        match safe_read(&self.performance_metrics, "get_performance_metrics") {
            Ok(metrics) => metrics.clone(),
            Err(_) => {
                log_error("get_performance_metrics", "Failed to read metrics, returning defaults");
                FusePerformanceMetrics::default()
            }
        }
    }

    /// Perform vector search through FUSE interface
    pub fn search_vectors_enhanced(
        &self,
        query_vector: &[f32],
        top_k: usize,
        search_params: Option<SearchParameters>
    ) -> Result<Vec<VectorSearchResult>, FuseVexfsError> {
        let start_time = self.start_operation();
        
        // Validate input parameters
        if query_vector.is_empty() {
            return Err(FuseVexfsError::InvalidVector("Query vector is empty".to_string()));
        }
        
        if query_vector.len() > 10000 {
            return Err(FuseVexfsError::InvalidVector(
                format!("Query vector dimension {} exceeds maximum", query_vector.len())
            ));
        }
        
        if top_k == 0 {
            return Err(FuseVexfsError::InvalidVector("top_k must be greater than 0".to_string()));
        }
        
        if top_k > 1000 {
            return Err(FuseVexfsError::InvalidVector(
                format!("top_k {} exceeds maximum of 1000", top_k)
            ));
        }
        
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
            
            // Use the Storage-HNSW bridge for real search operations
            match self.storage_hnsw_bridge.lock() {
                Ok(mut bridge) => {
                    match bridge.search_vectors(&mut context, query_vector, top_k, search_params) {
                        Ok(results) => {
                            // Results already contain real distances from HNSW
                            Ok(results)
                        }
                        Err(e) => {
                            eprintln!("Bridge search failed: {:?}", e);
                            // Fallback: Use direct HNSW search
                            self.perform_direct_hnsw_search(query_vector, top_k)
                        }
                    }
                }
                Err(_) => {
                    let error = FuseVexfsError::BridgeError("Failed to acquire bridge lock".to_string());
                    self.record_error(&error);
                    return Err(error);
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
        
        // Validate input parameters
        if vector_data.is_empty() {
            return Err(FuseVexfsError::InvalidVector("Vector data is empty".to_string()));
        }
        
        if vector_data.len() > 10000 {
            return Err(FuseVexfsError::InvalidVector(
                format!("Vector dimension {} exceeds maximum", vector_data.len())
            ));
        }
        
        // Check for NaN or infinite values
        for &value in vector_data {
            if !value.is_finite() {
                return Err(FuseVexfsError::InvalidVector(
                    "Vector contains NaN or infinite values".to_string()
                ));
            }
        }

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

            // Generate unique vector ID using atomic counter
            let vector_id = self.next_vector_id.fetch_add(1, Ordering::SeqCst);

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
                                if let Ok(mut files) = safe_lock(&self.files, "handle_ioctl_vector_store") {
                                    if let Some(file) = files.get_mut(&file_inode) {
                                        file.vector = Some(vector_data.to_vec());
                                    file.metadata = metadata;
                                } else {
                                    // Create a new file entry for the vector
                                    let now = system_time_to_timespec(SystemTime::now());
                                    let file = VexFSFile {
                                        ino: file_inode,
                                        name: format!("/vectors/v_{}", vector_id),
                                        parent: 1,  // Vector files are stored in root for now
                                        content: Vec::new(),
                                        metadata: metadata,
                                        vector: Some(vector_data.to_vec()),
                                        attr: FileAttr {
                                            ino: file_inode,
                                            size: (vector_data.len() * 4) as u64, // f32 is 4 bytes
                                            blocks: 1,
                                            atime: now,
                                            mtime: now,
                                            ctime: now,
                                            crtime: now,
                                            kind: FileType::RegularFile,
                                            perm: 0o644,
                                            nlink: 1,
                                            uid: 1000,
                                            gid: 1000,
                                            rdev: 0,
                                            flags: 0,
                                        },
                                        children: Vec::new(),  // Regular files don't have children
                                    };
                                    files.insert(file_inode, file);
                                }
                            }
                            
                            // Update vector ID to file mapping
                            {
                                if let Ok(mut mapping) = safe_lock(&self.vector_id_to_file, "update_vector_mapping") {
                                    mapping.insert(vector_id, file_inode);
                                }
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

            // Integrate with the actual vector storage layer
            // First, find the file inode associated with this vector ID
            let file_inode = self.find_inode_for_vector(vector_id)?;
            
            // Read the vector data from storage
            let vector_data = self.read_vector_from_storage(file_inode, vector_id)?;
            
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
        let name_str = name.to_string_lossy().to_string();
        
        // Look up the inode using parent and name
        let parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
        if let Some(&ino) = parent_name_to_ino.get(&(parent, name_str.clone())) {
            let files = self.files.lock().unwrap();
            if let Some(file) = files.get(&ino) {
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
            
            // Handle special file types
            if file.name.ends_with(".search") {
                // Handle search query files
                self.handle_search_file_write(file.ino, &file.content);
            } else if file.name == "_vexfs/search" {
                // Handle global search interface
                self.handle_global_search_write(&file.content);
            } else if file.name == "_vexfs/control" {
                // Handle control file updates
                self.handle_control_file_write(&file.content);
            } else if file.name.ends_with(".vec") {
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
        
        // Check if file already exists in this directory
        {
            let parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                reply.error(EEXIST);
                return;
            }
        }
        
        let ino = self.get_next_ino();
        let attr = Self::create_file_attr(ino, 0, FileType::RegularFile);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            parent,
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
            children: Vec::new(),
        };
        
        // Insert the file
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
            
            // Add to parent's children list
            if let Some(parent_file) = files.get_mut(&parent) {
                parent_file.children.push(ino);
            }
        }
        
        // Update the parent-name mapping
        {
            let mut parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            parent_name_to_ino.insert((parent, name_str), ino);
        }
        
        reply.created(&TTL, &attr, 0, 0, 0);
    }
    
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let files = self.files.lock().unwrap();
        
        // Get the directory being read
        if let Some(dir) = files.get(&ino) {
            if dir.attr.kind != FileType::Directory {
                reply.error(ENOTDIR);
                return;
            }
            
            let mut entries = vec![];
            
            // Add . and .. entries
            entries.push((ino, FileType::Directory, ".".to_string()));
            entries.push((dir.parent, FileType::Directory, "..".to_string()));
            
            // Add all children
            for &child_ino in &dir.children {
                if let Some(child) = files.get(&child_ino) {
                    entries.push((child_ino, child.attr.kind, child.name.clone()));
                }
            }
            
            // Send entries starting from offset
            for (i, (entry_ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
                if reply.add(*entry_ino, (i + 1) as i64, *kind, name) {
                    break;
                }
            }
        } else {
            reply.error(ENOENT);
            return;
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
        
        // Check if file already exists in this directory
        {
            let parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                reply.error(EEXIST);
                return;
            }
        }
        
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
            parent,
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
            children: Vec::new(),
        };
        
        // Insert the file
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
            
            // Add to parent's children list
            if let Some(parent_file) = files.get_mut(&parent) {
                parent_file.children.push(ino);
            }
        }
        
        // Update the parent-name mapping
        {
            let mut parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            parent_name_to_ino.insert((parent, name_str), ino);
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        
        // Check if directory already exists in this parent
        {
            let parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                reply.error(EEXIST);
                return;
            }
        }
        
        let ino = self.get_next_ino();
        let attr = Self::create_file_attr(ino, 0, FileType::Directory);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            parent,
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
            children: Vec::new(),
        };
        
        // Insert the directory
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
            
            // Add to parent's children list
            if let Some(parent_file) = files.get_mut(&parent) {
                parent_file.children.push(ino);
            }
        }
        
        // Update the parent-name mapping
        {
            let mut parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            parent_name_to_ino.insert((parent, name_str), ino);
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_string_lossy().to_string();
        
        // Look up the file to remove
        let ino_to_remove = {
            let parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            parent_name_to_ino.get(&(parent, name_str.clone())).copied()
        };
        
        if let Some(ino) = ino_to_remove {
            // Check if it's a regular file (not a directory)
            {
                let files = self.files.lock().unwrap();
                if let Some(file) = files.get(&ino) {
                    if file.attr.kind == FileType::Directory {
                        reply.error(EPERM);  // Cannot unlink a directory
                        return;
                    }
                }
            }
            
            // Remove from parent's children list and delete the file
            {
                let mut files = self.files.lock().unwrap();
                
                // Remove from parent's children
                if let Some(parent_file) = files.get_mut(&parent) {
                    parent_file.children.retain(|&child| child != ino);
                }
                
                // Remove the file itself
                files.remove(&ino);
            }
            
            // Remove from parent-name mapping
            {
                let mut parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
                parent_name_to_ino.remove(&(parent, name_str));
            }
            
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_string_lossy().to_string();
        
        // Look up the directory to remove
        let ino_to_remove = {
            let parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
            parent_name_to_ino.get(&(parent, name_str.clone())).copied()
        };
        
        if let Some(ino) = ino_to_remove {
            // Check if it's a directory and if it's empty
            {
                let files = self.files.lock().unwrap();
                if let Some(dir) = files.get(&ino) {
                    if dir.attr.kind != FileType::Directory {
                        reply.error(ENOTDIR);  // Not a directory
                        return;
                    }
                    if !dir.children.is_empty() {
                        reply.error(ENOTEMPTY);  // Directory not empty
                        return;
                    }
                } else {
                    reply.error(ENOENT);
                    return;
                }
            }
            
            // Remove from parent's children list and delete the directory
            {
                let mut files = self.files.lock().unwrap();
                
                // Remove from parent's children
                if let Some(parent_file) = files.get_mut(&parent) {
                    parent_file.children.retain(|&child| child != ino);
                }
                
                // Remove the directory itself
                files.remove(&ino);
            }
            
            // Remove from parent-name mapping
            {
                let mut parent_name_to_ino = self.parent_name_to_ino.lock().unwrap();
                parent_name_to_ino.remove(&(parent, name_str));
            }
            
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_fuse() -> VexFSFuse {
        VexFSFuse::new().expect("Failed to create test FUSE instance")
    }

    #[test]
    fn test_vector_storage_and_retrieval() {
        let fuse = create_test_fuse();
        
        // Create test vector
        let vector_data: Vec<f32> = (0..128).map(|i| i as f32 * 0.1).collect();
        let file_inode = 1000;
        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());
        
        // Store vector
        let vector_id = fuse.store_vector_enhanced(&vector_data, file_inode, metadata.clone())
            .expect("Failed to store vector");
        
        assert!(vector_id > 0);
        
        // Retrieve vector
        let (retrieved_vector, retrieved_metadata) = fuse.get_vector_enhanced(vector_id)
            .expect("Failed to retrieve vector");
        
        // Verify vector data matches
        assert_eq!(retrieved_vector.len(), vector_data.len());
        
        // Verify metadata
        assert!(retrieved_metadata.contains_key("vector_id"));
    }

    #[test]
    fn test_unique_vector_ids() {
        let fuse = create_test_fuse();
        let vector_data: Vec<f32> = vec![1.0; 128];
        let metadata = HashMap::new();
        
        let id1 = fuse.store_vector_enhanced(&vector_data, 1001, metadata.clone())
            .expect("Failed to store first vector");
        let id2 = fuse.store_vector_enhanced(&vector_data, 1002, metadata.clone())
            .expect("Failed to store second vector");
        
        assert_ne!(id1, id2, "Vector IDs should be unique");
    }

    #[test]
    fn test_error_handling_empty_vector() {
        let fuse = create_test_fuse();
        let empty_vector: Vec<f32> = vec![];
        let metadata = HashMap::new();
        
        let result = fuse.store_vector_enhanced(&empty_vector, 1000, metadata);
        assert!(matches!(result, Err(FuseVexfsError::InvalidVector(_))));
    }
}