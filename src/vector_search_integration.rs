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

/// Vector search subsystem for VexFS
pub struct VectorSearchSubsystem {
    /// Primary search engine
    search_engine: VectorSearchEngine,
    /// Storage manager for fs_core integration
    storage_manager: Arc<StorageManager>,
    /// Metrics calculator
    metrics: VectorMetrics,
    /// Search statistics
    search_stats: SearchStatistics,
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
    /// Create new vector search subsystem with fs_core integration
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
        })
    }
    
    /// Initialize the search subsystem with HNSW index
    pub fn initialize_with_index(&mut self, index: AnnsIndex) -> VexfsResult<()> {
        self.search_engine.set_hnsw_index(index);
        Ok(())
    }
    
    /// Handle vector search ioctl commands
    pub fn handle_ioctl(
        &mut self,
        context: &mut OperationContext,
        cmd: u32,
        arg: *mut u8,
    ) -> VexfsResult<i32> {
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
        
        match search_cmd {
            VectorSearchIoctlCmd::Search => self.handle_search_ioctl(context, arg),
            VectorSearchIoctlCmd::BatchSearch => self.handle_batch_search_ioctl(context, arg),
            VectorSearchIoctlCmd::GetStats => self.handle_get_stats_ioctl(arg),
            VectorSearchIoctlCmd::ResetStats => self.handle_reset_stats_ioctl(),
            VectorSearchIoctlCmd::ConfigureOptions => self.handle_configure_options_ioctl(arg),
            VectorSearchIoctlCmd::UpdateIndex => self.handle_update_index_ioctl(arg),
            VectorSearchIoctlCmd::ValidateResults => self.handle_validate_results_ioctl(arg),
        }
    }
    
    /// Handle single vector search ioctl
    fn handle_search_ioctl(&mut self, context: &mut OperationContext, arg: *mut u8) -> VexfsResult<i32> {
        if arg.is_null() {
            return Err(VexfsError::InvalidArgument("Null argument pointer".to_string()));
        }
        
        let request = unsafe { &mut *(arg as *mut SearchRequest) };
        
        // Validate input parameters
        if request.vector_data.is_null() || request.dimensions == 0 || request.k == 0 {
            return Err(VexfsError::InvalidArgument("Invalid search parameters".to_string()));
        }
        
        // Convert C structures to Rust types
        let vector = unsafe {
            core::slice::from_raw_parts(request.vector_data, request.dimensions as usize).to_vec()
        };
        
        let metric = match request.metric {
            0 => DistanceMetric::Euclidean,
            1 => DistanceMetric::Cosine,
            2 => DistanceMetric::InnerProduct,
            _ => return Err(VexfsError::InvalidArgument("Invalid distance metric".to_string())),
        };
        
        let filter = self.convert_metadata_filter(&request.filter)?;
        
        // Create search query
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
        
        // Perform search
        let start_time = self.get_current_time_us();
        let results = self.search_engine.search(context, query)
            .map_err(|_| VexfsError::SearchError(SearchErrorKind::InvalidQuery))?;
        let end_time = self.get_current_time_us();
        
        // Update statistics
        self.update_search_stats(end_time - start_time, results.len());
        
        // Convert results to C format
        let num_results = core::cmp::min(results.len(), request.max_results as usize);
        request.num_results = num_results as u32;
        
        if !request.results.is_null() {
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
    
    /// Get current time in microseconds (placeholder)
    fn get_current_time_us(&self) -> u64 {
        1640995200_000_000 // Placeholder timestamp
    }
    
    /// Get search statistics
    pub fn get_statistics(&self) -> &SearchStatistics {
        &self.search_stats
    }
    
    /// Perform administrative search operations
    pub fn admin_search(&mut self, context: &mut OperationContext, query: SearchQuery) -> Result<Vec<ScoredResult>, SearchError> {
        self.search_engine.search(context, query)
    }
    
    /// Get search engine reference for advanced operations
    pub fn get_search_engine(&mut self) -> &mut VectorSearchEngine {
        &mut self.search_engine
    }
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