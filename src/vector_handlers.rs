//! Core vector operation handlers for VexFS kernel module
//! 
//! This module implements the actual vector processing logic for each ioctl operation.
//! All operations are designed for kernel-space execution with emphasis on performance,
//! memory safety, and integration with the VFS layer.

use crate::ioctl::*;
use crate::vector_storage::{VectorDataType, CompressionType};
use alloc::vec::Vec;
use core::ptr;

// Note: Kernel integration handled via C FFI, not direct kernel crate usage

/// Maximum vector dimension size for safety checks
const MAX_VECTOR_DIMENSION: u32 = 65536;

/// Maximum number of search results to prevent memory exhaustion
const MAX_SEARCH_RESULTS: usize = 10000;

/// Vector storage backend trait - abstracts the underlying storage mechanism
pub trait VectorStorage {
    fn store_embedding(&mut self, inode: u64, embedding: &VectorEmbedding) -> Result<u64, VectorIoctlError>;
    fn get_embedding(&self, inode: u64) -> Result<Option<VectorEmbedding>, VectorIoctlError>;
    fn update_embedding(&mut self, inode: u64, embedding: &VectorEmbedding) -> Result<(), VectorIoctlError>;
    fn delete_embedding(&mut self, inode: u64) -> Result<(), VectorIoctlError>;
    fn search_similar(&self, query: &VectorEmbedding, k: u32, ef_search: u32) -> Result<Vec<VectorSearchResult>, VectorIoctlError>;
}

/// ANNS index management trait - abstracts different indexing algorithms
pub trait ANNSIndex {
    fn add_vector(&mut self, id: u64, vector: &[f32]) -> Result<(), VectorIoctlError>;
    fn search(&self, query: &[f32], k: u32, ef_search: u32) -> Result<Vec<(u64, f32)>, VectorIoctlError>;
    fn update_vector(&mut self, id: u64, vector: &[f32]) -> Result<(), VectorIoctlError>;
    fn remove_vector(&mut self, id: u64) -> Result<(), VectorIoctlError>;
    fn optimize_index(&mut self) -> Result<(), VectorIoctlError>;
    fn get_stats(&self) -> IndexStats;
}

/// Vector embedding representation with metadata
#[derive(Debug, Clone)]
pub struct VectorEmbedding {
    pub vector_id: u64,
    pub file_inode: u64,
    pub data_type: VectorDataType,
    pub compression: CompressionType,
    pub dimensions: u32,
    pub data: Vec<u8>, // Raw vector data in specified format
    pub created_timestamp: u64,
    pub modified_timestamp: u64,
    pub checksum: u32,
    pub flags: u32,
}

/// Search result from vector similarity operations
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub vector_id: u64,
    pub file_inode: u64,
    pub similarity_score: f32,
    pub metadata: Vec<u8>,
}

/// Index statistics for monitoring and optimization
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_vectors: u64,
    pub index_size_bytes: u64,
    pub last_optimization: u64,
    pub search_performance_ms: u32,
}

/// Core vector operation handlers
pub struct VectorHandlers<S: VectorStorage, I: ANNSIndex> {
    storage: S,
    index: I,
}

impl<S: VectorStorage, I: ANNSIndex> VectorHandlers<S, I> {
    /// Create new vector handlers with storage and index backends
    pub fn new(storage: S, index: I) -> Self {
        Self { storage, index }
    }

    /// Handle ADD_EMBEDDING ioctl operation
    pub fn handle_add_embedding(
        &mut self,
        request: &AddEmbeddingRequest,
        user_buffer: *const u8,
    ) -> Result<AddEmbeddingResponse, VectorIoctlError> {
        // Validate input parameters
        self.validate_add_request(request)?;

        // Copy vector data from userspace
        let vector_data = self.copy_vector_from_user(
            user_buffer,
            request.data_type,
            request.dimensions,
        )?;

        // Create embedding metadata
        let embedding = VectorEmbedding {
            vector_id: 0, // Will be assigned by storage
            file_inode: request.file_inode,
            data_type: request.data_type,
            compression: request.compression,
            dimensions: request.dimensions,
            data: vector_data.clone(),
            created_timestamp: self.get_current_timestamp(),
            modified_timestamp: self.get_current_timestamp(),
            checksum: self.calculate_checksum(&vector_data),
            flags: request.flags,
        };

        // Store embedding in persistent storage
        let vector_id = self.storage.store_embedding(request.file_inode, &embedding)?;

        // Convert to f32 for indexing if needed
        let f32_vector = self.convert_to_f32(&vector_data, request.data_type)?;

        // Add to ANNS index for fast similarity search
        self.index.add_vector(vector_id, &f32_vector)?;

        Ok(AddEmbeddingResponse {
            vector_id,
            result: VectorIoctlError::Success,
            processing_time_us: 0, // TODO: Implement timing
            storage_location: 0, // TODO: Implement storage tracking
            compressed_size: vector_data.len() as u32,
            checksum: embedding.checksum,
            flags: 0,
            reserved: [0; 5],
        })
    }

    /// Handle GET_EMBEDDING ioctl operation
    pub fn handle_get_embedding(
        &self,
        request: &GetEmbeddingRequest,
    ) -> Result<GetEmbeddingResponse, VectorIoctlError> {
        // Validate input parameters
        if request.file_inode == 0 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        // Retrieve embedding from storage
        let embedding = self.storage.get_embedding(request.file_inode)?
            .ok_or(VectorIoctlError::EmbeddingNotFound)?;

        // Validate user buffer size
        if request.buffer_size < embedding.data.len() as u32 {
            return Err(VectorIoctlError::BufferTooSmall);
        }

        Ok(GetEmbeddingResponse {
            vector_id: embedding.vector_id,
            result: VectorIoctlError::Success,
            data_type: embedding.data_type,
            compression: embedding.compression,
            dimensions: embedding.dimensions,
            original_size: embedding.data.len() as u32,
            actual_size: embedding.data.len() as u32,
            created_timestamp: embedding.created_timestamp,
            modified_timestamp: embedding.modified_timestamp,
            checksum: embedding.checksum,
            flags: embedding.flags,
            reserved: [0; 3],
        })
    }

    /// Handle UPDATE_EMBEDDING ioctl operation
    pub fn handle_update_embedding(
        &mut self,
        request: &UpdateEmbeddingRequest,
        user_buffer: *const u8,
    ) -> Result<UpdateEmbeddingResponse, VectorIoctlError> {
        // Validate input parameters
        self.validate_update_request(request)?;

        // Check if embedding exists
        let mut embedding = self.storage.get_embedding(request.file_inode)?
            .ok_or(VectorIoctlError::EmbeddingNotFound)?;

        // Copy new vector data from userspace
        let new_vector_data = self.copy_vector_from_user(
            user_buffer,
            request.data_type,
            request.dimensions,
        )?;

        // Update embedding metadata
        embedding.data_type = request.data_type;
        embedding.compression = request.compression;
        embedding.dimensions = request.dimensions;
        embedding.data = new_vector_data.clone();
        embedding.modified_timestamp = self.get_current_timestamp();
        embedding.checksum = self.calculate_checksum(&new_vector_data);
        embedding.flags = request.flags;

        // Update in persistent storage
        self.storage.update_embedding(request.file_inode, &embedding)?;

        // Convert to f32 and update in ANNS index
        let f32_vector = self.convert_to_f32(&new_vector_data, request.data_type)?;
        self.index.update_vector(embedding.vector_id, &f32_vector)?;

        Ok(UpdateEmbeddingResponse {
            vector_id: embedding.vector_id,
            result: VectorIoctlError::Success,
            processing_time_us: 0, // TODO: Implement timing
            new_storage_location: 0, // TODO: Implement storage tracking
            new_compressed_size: new_vector_data.len() as u32,
            update_timestamp: embedding.modified_timestamp,
            new_checksum: embedding.checksum,
            flags: 0,
            reserved: [0; 3],
        })
    }

    /// Handle DELETE_EMBEDDING ioctl operation
    pub fn handle_delete_embedding(
        &mut self,
        request: &DeleteEmbeddingRequest,
    ) -> Result<DeleteEmbeddingResponse, VectorIoctlError> {
        // Validate input parameters
        if request.file_inode == 0 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        // Get embedding to retrieve vector_id
        let embedding = self.storage.get_embedding(request.file_inode)?
            .ok_or(VectorIoctlError::EmbeddingNotFound)?;

        // Remove from ANNS index first
        self.index.remove_vector(embedding.vector_id)?;

        // Remove from persistent storage
        self.storage.delete_embedding(request.file_inode)?;

        Ok(DeleteEmbeddingResponse {
            vector_id: embedding.vector_id,
            result: VectorIoctlError::Success,
            processing_time_us: 0, // TODO: Implement timing
            freed_blocks: 1, // TODO: Calculate actual freed blocks
            deletion_timestamp: self.get_current_timestamp(),
            flags: 0,
            reserved: [0; 6],
        })
    }

    /// Handle VECTOR_SEARCH ioctl operation
    pub fn handle_vector_search(
        &self,
        request: &VectorSearchRequest,
        query_buffer: *const u8,
    ) -> Result<VectorSearchResponse, VectorIoctlError> {
        // Validate input parameters
        self.validate_search_request(request)?;

        // Copy query vector from userspace
        let query_data = self.copy_vector_from_user(
            query_buffer,
            request.data_type,
            request.dimensions,
        )?;

        // Convert query to f32 for search
        let f32_query = self.convert_to_f32(&query_data, request.data_type)?;

        // Perform similarity search using ANNS index
        let search_results = self.index.search(&f32_query, request.k, request.ef_search)?;

        // Convert results to response format
        let mut results = Vec::with_capacity(search_results.len());
        for (vector_id, similarity) in search_results {
            // Retrieve metadata for each result
            if let Ok(Some(embedding)) = self.storage.get_embedding_by_vector_id(vector_id) {
                results.push(VectorSearchResult {
                    vector_id,
                    file_inode: embedding.file_inode,
                    similarity_score: similarity,
                    metadata: Vec::new(), // Would include file metadata
                });
            }
        }

        Ok(VectorSearchResponse {
            result_count: results.len() as u32,
            search_time_us: 0, // Would be measured
            distance_calculations: 0, // TODO: Track distance calculations
            nodes_visited: 0, // TODO: Track nodes visited
            index_size: self.index.get_stats().total_vectors as u64,
            flags: 0,
            reserved: [0; 6],
        })
    }

    /// Handle HYBRID_SEARCH ioctl operation (combines vector and metadata search)
    pub fn handle_hybrid_search(
        &self,
        request: &HybridSearchRequest,
        query_buffer: *const u8,
        metadata_buffer: *const u8,
    ) -> Result<HybridSearchResponse, VectorIoctlError> {
        // Validate input parameters
        self.validate_hybrid_search_request(request)?;

        // Copy query vector from userspace
        let query_data = self.copy_vector_from_user(
            query_buffer,
            request.data_type,
            request.dimensions,
        )?;

        // Copy metadata filter from userspace
        let metadata_filter = if request.metadata_filter_size > 0 {
            self.copy_metadata_from_user(metadata_buffer, request.metadata_filter_size)?
        } else {
            Vec::new()
        };

        // Convert query to f32 for search
        let f32_query = self.convert_to_f32(&query_data, request.data_type)?;

        // Perform vector similarity search
        let vector_results = self.index.search(&f32_query, request.k * 2, request.ef_search)?;

        // Apply metadata filtering and combine scores
        let mut hybrid_results = Vec::new();
        for (vector_id, vector_score) in vector_results {
            if let Ok(Some(embedding)) = self.storage.get_embedding_by_vector_id(vector_id) {
                // Apply metadata filter
                let metadata_score = self.calculate_metadata_score(&embedding, &metadata_filter)?;
                
                if metadata_score > 0.0 {
                    // Combine vector and metadata scores
                    let combined_score = request.vector_weight * vector_score 
                        + request.metadata_weight * metadata_score;

                    hybrid_results.push((vector_id, embedding.file_inode, combined_score));
                }
            }
        }

        // Sort by combined score and limit results
        hybrid_results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        hybrid_results.truncate(request.k as usize);

        Ok(HybridSearchResponse {
            num_results: hybrid_results.len() as u32,
            total_searched: self.index.get_stats().total_vectors as u32,
            search_time_us: 0, // Would be measured
            vector_score_avg: 0.0, // Would be calculated
            metadata_score_avg: 0.0, // Would be calculated
        })
    }

    /// Handle MANAGE_INDEX ioctl operation
    pub fn handle_manage_index(
        &mut self,
        request: &ManageIndexRequest,
    ) -> Result<ManageIndexResponse, VectorIoctlError> {
        // Validate input parameters and permissions
        if request.operation_type > 3 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        let result = match request.operation_type {
            0 => {
                // Optimize index
                self.index.optimize_index()?;
                "Index optimization completed"
            },
            1 => {
                // Get statistics
                let stats = self.index.get_stats();
                return Ok(ManageIndexResponse {
                    operation_result: 0,
                    message_size: 0,
                    index_size_bytes: stats.index_size_bytes,
                    total_vectors: stats.total_vectors,
                    last_optimization: stats.last_optimization,
                });
            },
            2 => {
                // Rebuild index
                self.rebuild_index()?;
                "Index rebuild completed"
            },
            3 => {
                // Validate index integrity
                self.validate_index_integrity()?;
                "Index validation completed"
            },
            _ => return Err(VectorIoctlError::InvalidParameter),
        };

        let stats = self.index.get_stats();
        Ok(ManageIndexResponse {
            operation_result: 0,
            message_size: result.len() as u32,
            index_size_bytes: stats.index_size_bytes,
            total_vectors: stats.total_vectors,
            last_optimization: stats.last_optimization,
        })
    }

    // Helper methods for validation and data processing

    fn validate_add_request(&self, request: &AddEmbeddingRequest) -> Result<(), VectorIoctlError> {
        if request.file_inode == 0 {
            return Err(VectorIoctlError::InvalidParameter);
        }
        
        if request.dimensions == 0 || request.dimensions > MAX_VECTOR_DIMENSION {
            return Err(VectorIoctlError::InvalidDimensions);
        }

        // Validate data type
        match request.data_type {
            VectorDataType::Float32 | VectorDataType::Float16 | 
            VectorDataType::Int8 | VectorDataType::Int16 | VectorDataType::Binary => {},
        }

        Ok(())
    }

    fn validate_update_request(&self, request: &UpdateEmbeddingRequest) -> Result<(), VectorIoctlError> {
        if request.file_inode == 0 {
            return Err(VectorIoctlError::InvalidParameter);
        }
        
        if request.dimensions == 0 || request.dimensions > MAX_VECTOR_DIMENSION {
            return Err(VectorIoctlError::InvalidDimensions);
        }

        Ok(())
    }

    fn validate_search_request(&self, request: &VectorSearchRequest) -> Result<(), VectorIoctlError> {
        if request.dimensions == 0 || request.dimensions > MAX_VECTOR_DIMENSION {
            return Err(VectorIoctlError::InvalidDimensions);
        }

        if request.k == 0 || request.k > MAX_SEARCH_RESULTS as u32 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        if request.ef_search == 0 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        Ok(())
    }

    fn validate_hybrid_search_request(&self, request: &HybridSearchRequest) -> Result<(), VectorIoctlError> {
        if request.dimensions == 0 || request.dimensions > MAX_VECTOR_DIMENSION {
            return Err(VectorIoctlError::InvalidDimensions);
        }

        if request.k == 0 || request.k > MAX_SEARCH_RESULTS as u32 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        if request.vector_weight + request.metadata_weight <= 0.0 {
            return Err(VectorIoctlError::InvalidParameter);
        }

        Ok(())
    }

    fn copy_vector_from_user(
        &self,
        user_ptr: *const u8,
        data_type: VectorDataType,
        dimensions: u32,
    ) -> Result<Vec<u8>, VectorIoctlError> {
        if user_ptr.is_null() {
            return Err(VectorIoctlError::InvalidParameter);
        }

        let element_size = match data_type {
            VectorDataType::Float32 => 4,
            VectorDataType::Float16 => 2,
            VectorDataType::Int8 => 1,
            VectorDataType::Int16 => 2,
            VectorDataType::Binary => 1,
        };

        let total_size = dimensions as usize * element_size;
        let mut buffer = vec![0u8; total_size];

        // Use proper kernel API for copying from userspace
        let user_slice = UserSlicePtr::new(user_ptr as *mut u8, total_size)
            .map_err(|_| VectorIoctlError::InvalidParameter)?;
        let mut reader = user_slice.reader();
        reader.read_raw(&mut buffer)
            .map_err(|_| VectorIoctlError::MemoryError)?;

        Ok(buffer)
    }

    fn copy_metadata_from_user(
        &self,
        user_ptr: *const u8,
        size: u32,
    ) -> Result<Vec<u8>, VectorIoctlError> {
        if user_ptr.is_null() || size == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = vec![0u8; size as usize];
        
        // Use proper kernel API for copying from userspace
        let user_slice = UserSlicePtr::new(user_ptr as *mut u8, size as usize)
            .map_err(|_| VectorIoctlError::InvalidParameter)?;
        let mut reader = user_slice.reader();
        reader.read_raw(&mut buffer)
            .map_err(|_| VectorIoctlError::MemoryError)?;

        Ok(buffer)
    }

    fn convert_to_f32(&self, data: &[u8], data_type: VectorDataType) -> Result<Vec<f32>, VectorIoctlError> {
        match data_type {
            VectorDataType::Float32 => {
                if data.len() % 4 != 0 {
                    return Err(VectorIoctlError::InvalidParameter);
                }
                let f32_slice = unsafe {
                    core::slice::from_raw_parts(data.as_ptr() as *const f32, data.len() / 4)
                };
                Ok(f32_slice.to_vec())
            },
            VectorDataType::Float16 => {
                // Convert f16 to f32 - simplified implementation
                let mut result = Vec::new();
                for chunk in data.chunks_exact(2) {
                    let f16_bits = u16::from_le_bytes([chunk[0], chunk[1]]);
                    // Simplified f16 to f32 conversion - in real implementation would use proper f16 library
                    let f32_val = (f16_bits as f32) / 65535.0;
                    result.push(f32_val);
                }
                Ok(result)
            },
            VectorDataType::Int8 => {
                Ok(data.iter().map(|&b| b as i8 as f32 / 127.0).collect())
            },
            VectorDataType::Int16 => {
                if data.len() % 2 != 0 {
                    return Err(VectorIoctlError::InvalidParameter);
                }
                let mut result = Vec::new();
                for chunk in data.chunks_exact(2) {
                    let i16_val = i16::from_le_bytes([chunk[0], chunk[1]]);
                    result.push(i16_val as f32 / 32767.0);
                }
                Ok(result)
            },
            VectorDataType::Binary => {
                Ok(data.iter().map(|&b| if b != 0 { 1.0 } else { 0.0 }).collect())
            },
        }
    }

    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        // Simple CRC32-like checksum - in real implementation would use proper CRC
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_mul(31).wrapping_add(byte as u32);
        }
        checksum
    }

    fn calculate_metadata_score(&self, embedding: &VectorEmbedding, filter: &[u8]) -> Result<f32, VectorIoctlError> {
        // Simplified metadata scoring - in real implementation would parse metadata properly
        if filter.is_empty() {
            return Ok(1.0);
        }

        // Basic string matching score for demonstration
        let score = if embedding.flags & 0x1 != 0 { 0.8 } else { 0.2 };
        Ok(score)
    }

    fn get_current_timestamp(&self) -> u64 {
        // In real kernel implementation, would use proper kernel time functions
        0 // Placeholder
    }

    fn rebuild_index(&mut self) -> Result<(), VectorIoctlError> {
        // Rebuild the entire ANNS index from storage
        // This is a placeholder - real implementation would iterate through all embeddings
        Ok(())
    }

    fn validate_index_integrity(&self) -> Result<(), VectorIoctlError> {
        // Validate that the index is consistent with storage
        // This is a placeholder - real implementation would check index consistency
        Ok(())
    }
}

// Extension trait to add missing method to storage trait
pub trait VectorStorageExt: VectorStorage {
    fn get_embedding_by_vector_id(&self, vector_id: u64) -> Result<Option<VectorEmbedding>, VectorIoctlError>;
}

// Default implementation that would need to be overridden by actual storage backends
impl<T: VectorStorage> VectorStorageExt for T {
    fn get_embedding_by_vector_id(&self, _vector_id: u64) -> Result<Option<VectorEmbedding>, VectorIoctlError> {
        // Default implementation - would need to be implemented by storage backend
        Err(VectorIoctlError::OperationNotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    // Mock implementations for testing
    struct MockStorage {
        embeddings: Vec<VectorEmbedding>,
    }

    impl VectorStorage for MockStorage {
        fn store_embedding(&mut self, inode: u64, embedding: &VectorEmbedding) -> Result<u64, VectorIoctlError> {
            let vector_id = self.embeddings.len() as u64 + 1;
            let mut new_embedding = embedding.clone();
            new_embedding.vector_id = vector_id;
            self.embeddings.push(new_embedding);
            Ok(vector_id)
        }

        fn get_embedding(&self, inode: u64) -> Result<Option<VectorEmbedding>, VectorIoctlError> {
            Ok(self.embeddings.iter().find(|e| e.file_inode == inode).cloned())
        }

        fn update_embedding(&mut self, inode: u64, embedding: &VectorEmbedding) -> Result<(), VectorIoctlError> {
            if let Some(existing) = self.embeddings.iter_mut().find(|e| e.file_inode == inode) {
                *existing = embedding.clone();
                Ok(())
            } else {
                Err(VectorIoctlError::EmbeddingNotFound)
            }
        }

        fn delete_embedding(&mut self, inode: u64) -> Result<(), VectorIoctlError> {
            if let Some(pos) = self.embeddings.iter().position(|e| e.file_inode == inode) {
                self.embeddings.remove(pos);
                Ok(())
            } else {
                Err(VectorIoctlError::EmbeddingNotFound)
            }
        }

        fn search_similar(&self, _query: &VectorEmbedding, _k: u32, _ef_search: u32) -> Result<Vec<VectorSearchResult>, VectorIoctlError> {
            Ok(Vec::new())
        }
    }

    struct MockIndex;

    impl ANNSIndex for MockIndex {
        fn add_vector(&mut self, _id: u64, _vector: &[f32]) -> Result<(), VectorIoctlError> {
            Ok(())
        }

        fn search(&self, _query: &[f32], k: u32, _ef_search: u32) -> Result<Vec<(u64, f32)>, VectorIoctlError> {
            Ok(vec![(1, 0.9), (2, 0.8)].into_iter().take(k as usize).collect())
        }

        fn update_vector(&mut self, _id: u64, _vector: &[f32]) -> Result<(), VectorIoctlError> {
            Ok(())
        }

        fn remove_vector(&mut self, _id: u64) -> Result<(), VectorIoctlError> {
            Ok(())
        }

        fn optimize_index(&mut self) -> Result<(), VectorIoctlError> {
            Ok(())
        }

        fn get_stats(&self) -> IndexStats {
            IndexStats {
                total_vectors: 100,
                index_size_bytes: 1024 * 1024,
                last_optimization: 0,
                search_performance_ms: 5,
            }
        }
    }

    #[test]
    fn test_add_embedding_validation() {
        let storage = MockStorage { embeddings: Vec::new() };
        let index = MockIndex;
        let handlers = VectorHandlers::new(storage, index);

        let request = AddEmbeddingRequest {
            vector_id: 0,
            file_inode: 0, // Invalid
            dimensions: 128,
            data_type: VectorDataType::Float32,
            compression: 0, // CompressionType::None as u8
            data_size: 512,
            flags: 0,
            metadata_size: 0,
            reserved: [0; 6],
        };

        assert!(handlers.validate_add_request(&request).is_err());
    }

    #[test]
    fn test_search_validation() {
        let storage = MockStorage { embeddings: Vec::new() };
        let index = MockIndex;
        let handlers = VectorHandlers::new(storage, index);

        let request = VectorSearchRequest {
            dimensions: 0, // Invalid
            k: 10,
            metric: crate::anns::DistanceMetric::Euclidean,
            ef_search: 100,
            use_metadata_filter: 0,
            file_inode_filter: 0,
            min_confidence: 0,
            max_distance_scaled: 0,
            flags: 0,
            reserved: [0; 4],
        };

        assert!(handlers.validate_search_request(&request).is_err());
    }
}