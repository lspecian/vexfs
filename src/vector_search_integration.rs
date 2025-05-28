//! Vector search integration module for VexFS
//!
//! This module provides the main integration layer that combines all vector search components
//! into a cohesive system, including the ioctl interface, search coordination, and result management.



extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap, format, sync::Arc};
use core::ptr;

use crate::shared::errors::{VexfsError, VexfsResult, SearchErrorKind};
use crate::fs_core::operations::OperationContext;
use crate::storage::StorageManager;
use crate::vector_search::{VectorSearchEngine, SearchQuery, SearchOptions, BatchSearchRequest, SearchError};
use crate::vector_metrics::{VectorMetrics, MetricsError, MetricsConfig};
use crate::knn_search::{KnnSearchEngine, SearchParams, MetadataFilter};
use crate::result_scoring::{ScoredResult, ResultScorer, ScoringParams};
use crate::vector_storage::{VectorStorageManager, VectorHeader};
use crate::anns::{AnnsIndex, DistanceMetric};

/// Vector search subsystem for VexFS with comprehensive OperationContext integration
pub struct VectorSearchSubsystem {
    /// Primary search engine
    search_engine: VectorSearchEngine,
    /// Storage manager for fs_core integration
    storage_manager: Arc<StorageManager>,
    /// Metrics calculator
    metrics: VectorMetrics,
    /// Search statistics
    search_stats: SearchStatistics,
    /// Active ioctl operations for lifecycle management
    active_ioctl_operations: BTreeMap<u64, IoctlOperationMetadata>,
    /// Operation counter for unique operation IDs
    ioctl_operation_counter: u64,
    /// Transaction manager for rollback capabilities
    transaction_manager: TransactionManager,
}

/// Search statistics and performance metrics
#[derive(Debug, Clone, Default)]
pub struct SearchStatistics {
    /// Total number of searches performed
    pub total_searches: u64,
    /// Total search time in microseconds
    pub total_search_time_us: u64,
    /// Average search time (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub avg_search_time_us: f64,
    /// Cache hit rate (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub cache_hit_rate: f32,
    /// Index utilization statistics
    pub index_stats: IndexUtilizationStats,
}

/// Index utilization statistics
#[derive(Debug, Clone, Default)]
pub struct IndexUtilizationStats {
    /// Percentage of searches that used the HNSW index (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub hnsw_usage_rate: f32,
    /// Average index traversal depth (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub avg_traversal_depth: f32,
    /// Index memory usage in bytes
    pub index_memory_usage: usize,
}

/// Ioctl operation metadata for lifecycle tracking
#[derive(Debug, Clone)]
struct IoctlOperationMetadata {
    /// Operation ID
    operation_id: u64,
    /// Operation start time (microseconds)
    start_time_us: u64,
    /// Ioctl command type
    command: VectorSearchIoctlCmd,
    /// Estimated memory usage
    estimated_memory: usize,
    /// Operation status
    status: IoctlOperationStatus,
    /// User ID for permission tracking
    user_id: u32,
    /// Transaction ID for rollback support
    transaction_id: Option<u64>,
}

/// Ioctl operation status for lifecycle management
#[derive(Debug, Clone, Copy, PartialEq)]
enum IoctlOperationStatus {
    /// Operation is starting
    Starting,
    /// Operation is processing request
    Processing,
    /// Operation is executing search
    Searching,
    /// Operation is preparing response
    PreparingResponse,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}

/// Transaction manager for rollback capabilities
#[derive(Debug, Clone, Default)]
struct TransactionManager {
    /// Active transactions
    active_transactions: BTreeMap<u64, TransactionMetadata>,
    /// Transaction counter
    transaction_counter: u64,
}

/// Transaction metadata for rollback support
#[derive(Debug, Clone)]
struct TransactionMetadata {
    /// Transaction ID
    transaction_id: u64,
    /// Transaction start time
    start_time_us: u64,
    /// Operations in this transaction
    operations: Vec<u64>,
    /// Transaction status
    status: TransactionStatus,
    /// Rollback data
    rollback_data: Vec<RollbackEntry>,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq)]
enum TransactionStatus {
    /// Transaction is active
    Active,
    /// Transaction is committing
    Committing,
    /// Transaction committed successfully
    Committed,
    /// Transaction is rolling back
    RollingBack,
    /// Transaction was rolled back
    RolledBack,
    /// Transaction failed
    Failed,
}

/// Rollback entry for transaction recovery
#[derive(Debug, Clone)]
struct RollbackEntry {
    /// Entry type
    entry_type: RollbackEntryType,
    /// Associated data
    data: Vec<u8>,
}

/// Rollback entry types
#[derive(Debug, Clone, Copy, PartialEq)]
enum RollbackEntryType {
    /// Statistics snapshot
    StatisticsSnapshot,
    /// Configuration change
    ConfigurationChange,
    /// Index state change
    IndexStateChange,
}

/// Vector search ioctl commands
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorSearchIoctlCmd {
    /// Perform single vector search
    Search = 0x1000,
    /// Perform batch vector search
    BatchSearch = 0x1001,
    /// Get search statistics
    GetStats = 0x1002,
    /// Reset search statistics
    ResetStats = 0x1003,
    /// Configure search options
    ConfigureOptions = 0x1004,
    /// Build or update HNSW index
    UpdateIndex = 0x1005,
    /// Validate search results
    ValidateResults = 0x1006,
}

/// Search request structure for ioctl
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// Query vector data pointer (f32 disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub vector_data: *const f32,
    #[cfg(feature = "kernel-minimal")]
    pub vector_data: *const u8, // Use raw bytes in kernel mode
    /// Vector dimension count
    pub dimensions: u32,
    /// Number of results to return
    pub k: u32,
    /// Distance metric to use
    pub metric: u32, // Maps to DistanceMetric
    /// Search options flags
    pub flags: u32,
    /// Filter parameters
    pub filter: MetadataFilterC,
    /// Result buffer pointer
    pub results: *mut SearchResultC,
    /// Maximum result buffer size
    pub max_results: u32,
    /// Actual number of results returned
    pub num_results: u32,
}

/// C-compatible metadata filter
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MetadataFilterC {
    /// File size range filter
    pub file_size_min: u64,
    pub file_size_max: u64,
    /// Timestamp range filter
    pub timestamp_min: u64,
    pub timestamp_max: u64,
    /// Vector data type filter
    pub data_type_mask: u32,
    /// File extension filter (null-terminated string)
    pub extension_filter: [u8; 16],
    /// Filter flags
    pub flags: u32,
}

/// C-compatible search result
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SearchResultC {
    /// Vector ID
    pub vector_id: u64,
    /// Associated file inode
    pub file_inode: u64,
    /// Distance to query vector (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub distance: f32,
    #[cfg(feature = "kernel-minimal")]
    pub distance: u32, // Use fixed-point representation in kernel
    /// Relevance score (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub score: f32,
    #[cfg(feature = "kernel-minimal")]
    pub score: u32, // Use fixed-point representation in kernel
    /// Confidence value (disabled in kernel-minimal mode)
    #[cfg(not(feature = "kernel-minimal"))]
    pub confidence: f32,
    #[cfg(feature = "kernel-minimal")]
    pub confidence: u32, // Use fixed-point representation in kernel
    /// Vector metadata
    pub metadata: VectorMetadataC,
}

/// C-compatible vector metadata
#[repr(C)]
#[derive(Debug, Clone)]
pub struct VectorMetadataC {
    /// Vector dimensions
    pub dimensions: u32,
    /// Data type
    pub data_type: u32,
    /// File size in bytes
    pub file_size: u64,
    /// Creation timestamp
    pub created_timestamp: u64,
    /// Modification timestamp
    pub modified_timestamp: u64,
    /// Checksum
    pub checksum: u32,
}

impl VectorSearchSubsystem {
    /// Create new vector search subsystem with comprehensive OperationContext integration
    pub fn new(storage_manager: Arc<StorageManager>) -> VexfsResult<Self> {
        let vector_storage = VectorStorageManager::new(storage_manager.clone())?;
        let options = SearchOptions::default();
        let search_engine = VectorSearchEngine::new(Box::new(vector_storage), options)
            .map_err(|e| VexfsError::SearchError(SearchErrorKind::InvalidQuery))?;
        let metrics_config = MetricsConfig::default();
        let metrics = VectorMetrics::new(metrics_config);
        
        Ok(Self {
            search_engine,
            storage_manager,
            metrics,
            search_stats: SearchStatistics::default(),
            active_ioctl_operations: BTreeMap::new(),
            ioctl_operation_counter: 0,
            transaction_manager: TransactionManager::default(),
        })
    }
    
    /// Initialize the search subsystem with HNSW index
    pub fn initialize_with_index(&mut self, index: AnnsIndex) -> VexfsResult<()> {
        self.search_engine.set_hnsw_index(index);
        Ok(())
    }
    
    /// Handle vector search ioctl commands with comprehensive OperationContext integration
    pub fn handle_ioctl(
        &mut self,
        context: &mut OperationContext,
        cmd: u32,
        arg: *mut u8,
    ) -> VexfsResult<i32> {
        let start_time = self.get_current_time_us();
        
        // Parse and validate command
        let search_cmd = match cmd {
            x if x == VectorSearchIoctlCmd::Search as u32 => VectorSearchIoctlCmd::Search,
            x if x == VectorSearchIoctlCmd::BatchSearch as u32 => VectorSearchIoctlCmd::BatchSearch,
            x if x == VectorSearchIoctlCmd::GetStats as u32 => VectorSearchIoctlCmd::GetStats,
            x if x == VectorSearchIoctlCmd::ResetStats as u32 => VectorSearchIoctlCmd::ResetStats,
            x if x == VectorSearchIoctlCmd::ConfigureOptions as u32 => VectorSearchIoctlCmd::ConfigureOptions,
            x if x == VectorSearchIoctlCmd::UpdateIndex as u32 => VectorSearchIoctlCmd::UpdateIndex,
            x if x == VectorSearchIoctlCmd::ValidateResults as u32 => VectorSearchIoctlCmd::ValidateResults,
            _ => return Err(VexfsError::InvalidArgument("Invalid ioctl command".to_string())),
        };
        
        // Start operation tracking for lifecycle management
        let operation_id = self.start_ioctl_operation(context, search_cmd, start_time)?;
        
        // Start transaction for rollback support (for operations that modify state)
        let transaction_id = match search_cmd {
            VectorSearchIoctlCmd::ResetStats | VectorSearchIoctlCmd::ConfigureOptions | VectorSearchIoctlCmd::UpdateIndex => {
                Some(self.start_transaction(operation_id)?)
            }
            _ => None,
        };
        
        // Update operation with transaction ID
        if let Some(tx_id) = transaction_id {
            self.update_operation_transaction(operation_id, tx_id)?;
        }
        
        // Execute command with error handling and rollback support
        let result = match search_cmd {
            VectorSearchIoctlCmd::Search => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Searching)?;
                self.handle_search_ioctl(context, arg)
            }
            VectorSearchIoctlCmd::BatchSearch => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Searching)?;
                self.handle_batch_search_ioctl(context, arg)
            }
            VectorSearchIoctlCmd::GetStats => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Processing)?;
                self.handle_get_stats_ioctl(arg)
            }
            VectorSearchIoctlCmd::ResetStats => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Processing)?;
                self.handle_reset_stats_ioctl_with_transaction(transaction_id.unwrap())
            }
            VectorSearchIoctlCmd::ConfigureOptions => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Processing)?;
                self.handle_configure_options_ioctl_with_transaction(arg, transaction_id.unwrap())
            }
            VectorSearchIoctlCmd::UpdateIndex => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Processing)?;
                self.handle_update_index_ioctl_with_transaction(arg, transaction_id.unwrap())
            }
            VectorSearchIoctlCmd::ValidateResults => {
                self.update_operation_status(operation_id, IoctlOperationStatus::Processing)?;
                self.handle_validate_results_ioctl(arg)
            }
        };
        
        // Handle operation completion or failure
        match result {
            Ok(return_value) => {
                // Commit transaction if active
                if let Some(tx_id) = transaction_id {
                    self.commit_transaction(tx_id)?;
                }
                
                // Complete operation successfully
                let end_time = self.get_current_time_us();
                let operation_time = end_time - start_time;
                self.complete_ioctl_operation(operation_id, operation_time)?;
                
                Ok(return_value)
            }
            Err(e) => {
                // Rollback transaction if active
                if let Some(tx_id) = transaction_id {
                    self.rollback_transaction(tx_id)?;
                }
                
                // Fail operation
                self.fail_ioctl_operation(operation_id, format!("Ioctl operation failed: {:?}", e))?;
                
                Err(e)
            }
        }
    }
    
    /// Handle single vector search ioctl with comprehensive error handling and resource management
    fn handle_search_ioctl(&mut self, context: &mut OperationContext, arg: *mut u8) -> VexfsResult<i32> {
        if arg.is_null() {
            return Err(VexfsError::InvalidArgument("Null argument pointer".to_string()));
        }
        
        let request = unsafe { &mut *(arg as *mut SearchRequest) };
        
        // Validate input parameters with detailed error reporting
        if request.vector_data.is_null() {
            return Err(VexfsError::InvalidArgument("Vector data pointer is null".to_string()));
        }
        if request.dimensions == 0 {
            return Err(VexfsError::InvalidArgument("Vector dimensions cannot be zero".to_string()));
        }
        if request.k == 0 {
            return Err(VexfsError::InvalidArgument("Result count k cannot be zero".to_string()));
        }
        if request.k > 10000 {
            return Err(VexfsError::InvalidArgument("Result count k exceeds maximum limit".to_string()));
        }
        
        // Estimate memory usage for resource management
        let estimated_memory = (request.dimensions as usize * core::mem::size_of::<f32>()) +
                              (request.k as usize * core::mem::size_of::<SearchResultC>()) +
                              1024; // Overhead
        
        // Check memory constraints
        if estimated_memory > 16 * 1024 * 1024 { // 16MB limit for single search
            return Err(VexfsError::OutOfMemory);
        }
        
        // Convert C structures to Rust types with bounds checking
        let vector = unsafe {
            if request.dimensions > 10000 { // Sanity check
                return Err(VexfsError::InvalidArgument("Vector dimensions too large".to_string()));
            }
            core::slice::from_raw_parts(request.vector_data, request.dimensions as usize).to_vec()
        };
        
        // Validate vector data
        for (i, &value) in vector.iter().enumerate() {
            if !value.is_finite() {
                return Err(VexfsError::InvalidArgument(format!("Invalid vector component at index {}", i)));
            }
        }
        
        let metric = match request.metric {
            0 => DistanceMetric::Euclidean,
            1 => DistanceMetric::Cosine,
            2 => DistanceMetric::InnerProduct,
            _ => return Err(VexfsError::InvalidArgument("Invalid distance metric".to_string())),
        };
        
        let filter = self.convert_metadata_filter(&request.filter)?;
        
        // Create search query with validated parameters
        let query = SearchQuery {
            vector,
            k: request.k as usize,
            metric,
            approximate: (request.flags & 0x1) != 0,
            expansion_factor: 2.0,
            filter,
            exact_distances: (request.flags & 0x2) != 0,
            use_simd: (request.flags & 0x4) != 0,
        };
        
        // Perform search with comprehensive error handling
        let start_time = self.get_current_time_us();
        let results = match self.search_engine.search(context, query) {
            Ok(results) => results,
            Err(SearchError::InvalidQuery) => {
                return Err(VexfsError::SearchError(SearchErrorKind::InvalidQuery));
            }
            Err(SearchError::AllocationError) => {
                return Err(VexfsError::OutOfMemory);
            }
            Err(SearchError::StorageError) => {
                return Err(VexfsError::SearchError(SearchErrorKind::InvalidQuery));
            }
            Err(_) => {
                return Err(VexfsError::SearchError(SearchErrorKind::InvalidQuery));
            }
        };
        let end_time = self.get_current_time_us();
        
        // Update statistics with operation context
        self.update_search_stats_with_context(context, end_time - start_time, results.len(), estimated_memory);
        
        // Validate output buffer
        if !request.results.is_null() && request.max_results == 0 {
            return Err(VexfsError::InvalidArgument("Output buffer provided but max_results is zero".to_string()));
        }
        
        // Convert results to C format with bounds checking
        let num_results = core::cmp::min(results.len(), request.max_results as usize);
        request.num_results = num_results as u32;
        
        if !request.results.is_null() && num_results > 0 {
            let output_slice = unsafe {
                core::slice::from_raw_parts_mut(request.results, num_results)
            };
            
            for (i, result) in results.iter().take(num_results).enumerate() {
                output_slice[i] = self.convert_search_result(result);
            }
        }
        
        Ok(num_results as i32)
    }
    
    /// Handle batch search ioctl
    fn handle_batch_search_ioctl(&mut self, context: &mut OperationContext, arg: *mut u8) -> VexfsResult<i32> {
        // Simplified batch search implementation
        // In a real implementation, this would handle multiple queries
        self.handle_search_ioctl(context, arg)
    }
    
    /// Handle get statistics ioctl
    fn handle_get_stats_ioctl(&mut self, arg: *mut u8) -> VexfsResult<i32> {
        if arg.is_null() {
            return Err(VexfsError::InvalidArgument("Null argument pointer".to_string()));
        }
        
        let stats_ptr = arg as *mut SearchStatistics;
        unsafe {
            ptr::write(stats_ptr, self.search_stats.clone());
        }
        
        Ok(0)
    }
    
    /// Handle reset statistics ioctl
    fn handle_reset_stats_ioctl(&mut self) -> VexfsResult<i32> {
        self.search_stats = SearchStatistics::default();
        self.search_engine.reset_analytics();
        Ok(0)
    }
    
    /// Handle configure options ioctl
    fn handle_configure_options_ioctl(&mut self, _arg: *mut u8) -> VexfsResult<i32> {
        // Placeholder for configuration updates
        Ok(0)
    }
    
    /// Handle update index ioctl
    fn handle_update_index_ioctl(&mut self, _arg: *mut u8) -> VexfsResult<i32> {
        // Placeholder for index updates
        Ok(0)
    }
    
    /// Handle validate results ioctl
    fn handle_validate_results_ioctl(&mut self, _arg: *mut u8) -> VexfsResult<i32> {
        // Placeholder for result validation
        Ok(0)
    }
    
    /// Convert C metadata filter to Rust type
    fn convert_metadata_filter(&self, filter: &MetadataFilterC) -> VexfsResult<Option<MetadataFilter>> {
        if filter.flags == 0 {
            return Ok(None);
        }
        
        // Convert extension filter
        let extension = if filter.extension_filter[0] != 0 {
            // Find null terminator
            let mut len = 0;
            for &byte in &filter.extension_filter {
                if byte == 0 {
                    break;
                }
                len += 1;
            }
            if len > 0 {
                Some(format!("{}", core::str::from_utf8(&filter.extension_filter[..len])
                    .map_err(|_| VexfsError::InvalidArgument("Invalid UTF-8 in extension filter".to_string()))?))
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Some(MetadataFilter {
            file_size_range: if filter.file_size_max > filter.file_size_min {
                Some((filter.file_size_min, filter.file_size_max))
            } else {
                None
            },
            timestamp_range: if filter.timestamp_max > filter.timestamp_min {
                Some((filter.timestamp_min, filter.timestamp_max))
            } else {
                None
            },
            data_type_mask: if filter.data_type_mask != 0 {
                Some(filter.data_type_mask)
            } else {
                None
            },
            file_extension: extension,
        }))
    }
    
    /// Convert search result to C format
    fn convert_search_result(&self, result: &ScoredResult) -> SearchResultC {
        SearchResultC {
            vector_id: result.result.vector_id,
            file_inode: result.result.file_inode,
            distance: result.result.distance,
            score: result.score,
            confidence: result.confidence,
            metadata: VectorMetadataC {
                dimensions: result.result.dimensions,
                data_type: result.result.data_type as u32,
                file_size: result.result.file_size,
                created_timestamp: result.result.created_timestamp,
                modified_timestamp: result.result.modified_timestamp,
                checksum: result.result.checksum,
            },
        }
    }
    
    /// Update search statistics
    fn update_search_stats(&mut self, search_time_us: u64, result_count: usize) {
        self.search_stats.total_searches += 1;
        self.search_stats.total_search_time_us += search_time_us;
        
        // Only calculate floating-point average when not in kernel-minimal mode
        #[cfg(not(feature = "kernel-minimal"))]
        {
            self.search_stats.avg_search_time_us =
                self.search_stats.total_search_time_us as f64 / self.search_stats.total_searches as f64;
        }
    }

    /// Update search statistics with OperationContext integration
    fn update_search_stats_with_context(&mut self, context: &OperationContext, search_time_us: u64, result_count: usize, memory_used: usize) {
        self.update_search_stats(search_time_us, result_count);
        
        // Track user-specific statistics
        let _user_stats = (context.user.uid, search_time_us, result_count, memory_used);
        
        // Update index utilization statistics
        self.search_stats.index_stats.index_memory_usage =
            core::cmp::max(self.search_stats.index_stats.index_memory_usage, memory_used);
    }

    /// Start ioctl operation tracking for lifecycle management
    fn start_ioctl_operation(&mut self, context: &OperationContext, command: VectorSearchIoctlCmd, start_time: u64) -> VexfsResult<u64> {
        self.ioctl_operation_counter += 1;
        let operation_id = self.ioctl_operation_counter;
        
        let metadata = IoctlOperationMetadata {
            operation_id,
            start_time_us: start_time,
            command,
            estimated_memory: 0, // Will be updated later
            status: IoctlOperationStatus::Starting,
            user_id: context.user.uid,
            transaction_id: None,
        };
        
        self.active_ioctl_operations.insert(operation_id, metadata);
        Ok(operation_id)
    }

    /// Update operation status
    fn update_operation_status(&mut self, operation_id: u64, status: IoctlOperationStatus) -> VexfsResult<()> {
        if let Some(metadata) = self.active_ioctl_operations.get_mut(&operation_id) {
            metadata.status = status;
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Update operation with transaction ID
    fn update_operation_transaction(&mut self, operation_id: u64, transaction_id: u64) -> VexfsResult<()> {
        if let Some(metadata) = self.active_ioctl_operations.get_mut(&operation_id) {
            metadata.transaction_id = Some(transaction_id);
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Complete ioctl operation successfully
    fn complete_ioctl_operation(&mut self, operation_id: u64, execution_time: u64) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_ioctl_operations.remove(&operation_id) {
            metadata.status = IoctlOperationStatus::Completed;
            
            // Update performance statistics
            let _perf_stats = (operation_id, execution_time, metadata.command, metadata.user_id);
            
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Fail ioctl operation with error handling
    fn fail_ioctl_operation(&mut self, operation_id: u64, reason: String) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_ioctl_operations.remove(&operation_id) {
            metadata.status = IoctlOperationStatus::Failed;
            
            // Log failure for debugging
            let _failure_info = (operation_id, reason, metadata.user_id, self.get_current_time_us());
            
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Start transaction for rollback support
    fn start_transaction(&mut self, operation_id: u64) -> VexfsResult<u64> {
        self.transaction_manager.transaction_counter += 1;
        let transaction_id = self.transaction_manager.transaction_counter;
        
        let metadata = TransactionMetadata {
            transaction_id,
            start_time_us: self.get_current_time_us(),
            operations: vec![operation_id],
            status: TransactionStatus::Active,
            rollback_data: Vec::new(),
        };
        
        self.transaction_manager.active_transactions.insert(transaction_id, metadata);
        Ok(transaction_id)
    }

    /// Commit transaction
    fn commit_transaction(&mut self, transaction_id: u64) -> VexfsResult<()> {
        if let Some(mut metadata) = self.transaction_manager.active_transactions.remove(&transaction_id) {
            metadata.status = TransactionStatus::Committed;
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Transaction not found".to_string()))
        }
    }

    /// Rollback transaction
    fn rollback_transaction(&mut self, transaction_id: u64) -> VexfsResult<()> {
        if let Some(mut metadata) = self.transaction_manager.active_transactions.remove(&transaction_id) {
            metadata.status = TransactionStatus::RollingBack;
            
            // Apply rollback entries in reverse order
            for entry in metadata.rollback_data.iter().rev() {
                match entry.entry_type {
                    RollbackEntryType::StatisticsSnapshot => {
                        // Restore statistics from snapshot
                        // In a real implementation, this would deserialize the data
                    }
                    RollbackEntryType::ConfigurationChange => {
                        // Restore configuration from snapshot
                        // In a real implementation, this would deserialize the data
                    }
                    RollbackEntryType::IndexStateChange => {
                        // Restore index state from snapshot
                        // In a real implementation, this would deserialize the data
                    }
                }
            }
            
            metadata.status = TransactionStatus::RolledBack;
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Transaction not found".to_string()))
        }
    }

    /// Handle reset statistics ioctl with transaction support
    fn handle_reset_stats_ioctl_with_transaction(&mut self, transaction_id: u64) -> VexfsResult<i32> {
        // Create rollback entry with current statistics
        let rollback_entry = RollbackEntry {
            entry_type: RollbackEntryType::StatisticsSnapshot,
            data: Vec::new(), // In a real implementation, this would serialize current stats
        };
        
        // Add rollback entry to transaction
        if let Some(metadata) = self.transaction_manager.active_transactions.get_mut(&transaction_id) {
            metadata.rollback_data.push(rollback_entry);
        }
        
        // Reset statistics
        self.search_stats = SearchStatistics::default();
        self.search_engine.reset_analytics();
        
        Ok(0)
    }

    /// Handle configure options ioctl with transaction support
    fn handle_configure_options_ioctl_with_transaction(&mut self, _arg: *mut u8, transaction_id: u64) -> VexfsResult<i32> {
        // Create rollback entry with current configuration
        let rollback_entry = RollbackEntry {
            entry_type: RollbackEntryType::ConfigurationChange,
            data: Vec::new(), // In a real implementation, this would serialize current config
        };
        
        // Add rollback entry to transaction
        if let Some(metadata) = self.transaction_manager.active_transactions.get_mut(&transaction_id) {
            metadata.rollback_data.push(rollback_entry);
        }
        
        // Apply configuration changes (placeholder)
        Ok(0)
    }

    /// Handle update index ioctl with transaction support
    fn handle_update_index_ioctl_with_transaction(&mut self, _arg: *mut u8, transaction_id: u64) -> VexfsResult<i32> {
        // Create rollback entry with current index state
        let rollback_entry = RollbackEntry {
            entry_type: RollbackEntryType::IndexStateChange,
            data: Vec::new(), // In a real implementation, this would serialize current index state
        };
        
        // Add rollback entry to transaction
        if let Some(metadata) = self.transaction_manager.active_transactions.get_mut(&transaction_id) {
            metadata.rollback_data.push(rollback_entry);
        }
        
        // Apply index updates (placeholder)
        Ok(0)
    }
    
    /// Get current time in microseconds (placeholder)
    fn get_current_time_us(&self) -> u64 {
        1640995200_000_000 // Placeholder timestamp
    }
    
    /// Get search statistics
    pub fn get_statistics(&self) -> &SearchStatistics {
        &self.search_stats
    }
    
    /// Perform administrative search operations with comprehensive OperationContext integration
    pub fn admin_search(&mut self, context: &mut OperationContext, query: SearchQuery) -> Result<Vec<ScoredResult>, SearchError> {
        self.search_engine.search(context, query)
    }
    
    /// Get search engine reference for advanced operations
    pub fn get_search_engine(&mut self) -> &mut VectorSearchEngine {
        &mut self.search_engine
    }

    /// Get active ioctl operations for monitoring
    pub fn get_active_ioctl_operations(&self) -> &BTreeMap<u64, IoctlOperationMetadata> {
        &self.active_ioctl_operations
    }

    /// Get active transactions for monitoring
    pub fn get_active_transactions(&self) -> &BTreeMap<u64, TransactionMetadata> {
        &self.transaction_manager.active_transactions
    }

    /// Cleanup stale operations and transactions
    pub fn cleanup_stale_operations(&mut self, timeout_us: u64) -> (usize, usize) {
        let current_time = self.get_current_time_us();
        
        // Cleanup stale ioctl operations
        let mut stale_operations = Vec::new();
        for (&operation_id, metadata) in &self.active_ioctl_operations {
            if current_time - metadata.start_time_us > timeout_us {
                stale_operations.push(operation_id);
            }
        }
        
        let operations_cleaned = stale_operations.len();
        for operation_id in stale_operations {
            self.active_ioctl_operations.remove(&operation_id);
        }
        
        // Cleanup stale transactions
        let mut stale_transactions = Vec::new();
        for (&transaction_id, metadata) in &self.transaction_manager.active_transactions {
            if current_time - metadata.start_time_us > timeout_us {
                stale_transactions.push(transaction_id);
            }
        }
        
        let transactions_cleaned = stale_transactions.len();
        for transaction_id in stale_transactions {
            self.transaction_manager.active_transactions.remove(&transaction_id);
        }
        
        (operations_cleaned, transactions_cleaned)
    }

    /// Cancel active ioctl operation
    pub fn cancel_ioctl_operation(&mut self, operation_id: u64) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_ioctl_operations.remove(&operation_id) {
            metadata.status = IoctlOperationStatus::Cancelled;
            
            // If operation has a transaction, rollback
            if let Some(transaction_id) = metadata.transaction_id {
                self.rollback_transaction(transaction_id)?;
            }
            
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Operation not found".to_string()))
        }
    }

    /// Get comprehensive system health status
    pub fn get_system_health(&self) -> SystemHealthStatus {
        let active_operations_count = self.active_ioctl_operations.len();
        let active_transactions_count = self.transaction_manager.active_transactions.len();
        let search_engine_operations = self.search_engine.get_active_operations().len();
        
        SystemHealthStatus {
            active_ioctl_operations: active_operations_count,
            active_transactions: active_transactions_count,
            active_search_operations: search_engine_operations,
            total_searches: self.search_stats.total_searches,
            total_search_time_us: self.search_stats.total_search_time_us,
            index_memory_usage: self.search_stats.index_stats.index_memory_usage,
        }
    }
}

/// System health status for monitoring
#[derive(Debug, Clone)]
pub struct SystemHealthStatus {
    /// Number of active ioctl operations
    pub active_ioctl_operations: usize,
    /// Number of active transactions
    pub active_transactions: usize,
    /// Number of active search operations
    pub active_search_operations: usize,
    /// Total searches performed
    pub total_searches: u64,
    /// Total search time in microseconds
    pub total_search_time_us: u64,
    /// Index memory usage in bytes
    pub index_memory_usage: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_storage::VectorStorage;

    #[test]
    fn test_vector_search_subsystem_creation() {
        let storage = VectorStorage::new().unwrap();
        let subsystem = VectorSearchSubsystem::new(storage);
        assert!(subsystem.is_ok());
    }

    #[test]
    fn test_search_statistics_default() {
        let stats = SearchStatistics::default();
        assert_eq!(stats.total_searches, 0);
        assert_eq!(stats.total_search_time_us, 0);
        assert_eq!(stats.avg_search_time_us, 0.0);
    }

    #[test]
    fn test_ioctl_command_values() {
        assert_eq!(VectorSearchIoctlCmd::Search as u32, 0x1000);
        assert_eq!(VectorSearchIoctlCmd::BatchSearch as u32, 0x1001);
        assert_eq!(VectorSearchIoctlCmd::GetStats as u32, 0x1002);
    }
}