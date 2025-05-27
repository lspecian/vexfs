//! Userspace stub for vector_handlers module
//! Provides the necessary types for compilation without kernel dependencies

use crate::ondisk::VexfsInode;

/// Userspace stub for VectorStorage trait
pub trait VectorStorage {
    type Error;
    
    fn store_vector(&mut self, inode: &VexfsInode, vector: &[f32]) -> Result<(), Self::Error>;
    fn load_vector(&self, inode: &VexfsInode) -> Result<Vec<f32>, Self::Error>;
    fn get_all_vector_ids(&self) -> Result<Vec<u64>, Self::Error>;
    fn get_vector_header(&self, vector_id: u64) -> Result<crate::vector_storage::VectorHeader, Self::Error>;
    fn get_vector_data(&self, vector_id: u64) -> Result<Vec<f32>, Self::Error>;
    fn get_vector_count(&self) -> Result<usize, Self::Error>;
}

/// Userspace stub for VectorEmbedding
#[derive(Debug, Clone)]
pub struct VectorEmbedding {
    pub id: u64,
    pub data: Vec<f32>,
    pub metadata: Option<Vec<u8>>,
}

impl VectorEmbedding {
    pub fn new(id: u64, data: Vec<f32>) -> Self {
        Self {
            id,
            data,
            metadata: None,
        }
    }
    
    pub fn with_metadata(id: u64, data: Vec<f32>, metadata: Vec<u8>) -> Self {
        Self {
            id,
            data,
            metadata: Some(metadata),
        }
    }
}

/// Stub implementation for testing
pub struct StubVectorStorage;

impl VectorStorage for StubVectorStorage {
    type Error = String;
    
    fn store_vector(&mut self, _inode: &VexfsInode, _vector: &[f32]) -> Result<(), Self::Error> {
        Ok(())
    }
    
    fn load_vector(&self, _inode: &VexfsInode) -> Result<Vec<f32>, Self::Error> {
        Ok(vec![0.0; 128]) // Return dummy vector
    }
    
    fn get_all_vector_ids(&self) -> Result<Vec<u64>, Self::Error> {
        Ok(vec![1, 2, 3]) // Return dummy vector IDs
    }
    
    fn get_vector_header(&self, _vector_id: u64) -> Result<crate::vector_storage::VectorHeader, Self::Error> {
        Ok(crate::vector_storage::VectorHeader {
            magic: crate::vector_storage::VectorHeader::MAGIC,
            version: crate::vector_storage::VECTOR_FORMAT_VERSION,
            vector_id: _vector_id,
            file_inode: 1,
            data_type: crate::vector_storage::VectorDataType::Float32,
            compression: crate::vector_storage::CompressionType::None,
            dimensions: 128,
            original_size: 512,
            compressed_size: 512,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        })
    }
    
    fn get_vector_data(&self, _vector_id: u64) -> Result<Vec<f32>, Self::Error> {
        Ok(vec![0.0; 128]) // Return dummy vector data
    }
    
    fn get_vector_count(&self) -> Result<usize, Self::Error> {
        Ok(3) // Return dummy count
    }
}