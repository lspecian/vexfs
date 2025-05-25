//! On-disk Index Serialization Format for ANNS
//! 
//! This module implements efficient binary format with memory-mapped support
//! and incremental updates for persistent vector indices.

#![no_std]

use core::{mem, ptr, slice};
use crate::anns::{AnnsError, AnnsIndexHeader, HnswNode, HnswLayer, WalEntryHeader};
use crate::vector_storage::VectorDataType;

/// Index file format version
pub const INDEX_FORMAT_VERSION: u32 = 1;

/// Index file magic number
pub const INDEX_FILE_MAGIC: u32 = 0x56455849; // "VEXI"

/// Page size for memory mapping (4KB standard)
pub const PAGE_SIZE: usize = 4096;

/// Maximum index file size (for kernel space safety)
pub const MAX_INDEX_SIZE: u64 = 1024 * 1024 * 1024; // 1GB

/// Index file header for on-disk storage
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IndexFileHeader {
    /// File magic number
    pub magic: u32,
    /// File format version
    pub version: u32,
    /// Total file size in bytes
    pub file_size: u64,
    /// Index header offset
    pub index_header_offset: u64,
    /// Layers section offset
    pub layers_offset: u64,
    /// Layers section size
    pub layers_size: u64,
    /// Nodes section offset
    pub nodes_offset: u64,
    /// Nodes section size
    pub nodes_size: u64,
    /// Connections section offset
    pub connections_offset: u64,
    /// Connections section size
    pub connections_size: u64,
    /// Vector data section offset
    pub vectors_offset: u64,
    /// Vector data section size
    pub vectors_size: u64,
    /// Metadata section offset
    pub metadata_offset: u64,
    /// Metadata section size
    pub metadata_size: u64,
    /// Checksum of entire file (excluding this field)
    pub file_checksum: u32,
    /// Creation timestamp
    pub created_timestamp: u64,
    /// Last modification timestamp
    pub modified_timestamp: u64,
    /// File flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl IndexFileHeader {
    pub const SIZE: usize = mem::size_of::<IndexFileHeader>();
    
    pub fn new() -> Self {
        Self {
            magic: INDEX_FILE_MAGIC,
            version: INDEX_FORMAT_VERSION,
            file_size: 0,
            index_header_offset: Self::SIZE as u64,
            layers_offset: 0,
            layers_size: 0,
            nodes_offset: 0,
            nodes_size: 0,
            connections_offset: 0,
            connections_size: 0,
            vectors_offset: 0,
            vectors_size: 0,
            metadata_offset: 0,
            metadata_size: 0,
            file_checksum: 0,
            created_timestamp: 0, // TODO: get kernel time
            modified_timestamp: 0,
            flags: 0,
            reserved: [0; 32],
        }
    }
}

/// Memory-mapped index section
#[derive(Debug)]
pub struct MappedSection {
    /// Virtual address of mapped memory
    pub vaddr: u64,
    /// Physical address (if available)
    pub paddr: u64,
    /// Size of mapped region
    pub size: u64,
    /// Offset in file
    pub file_offset: u64,
    /// Access flags (read/write)
    pub flags: u32,
    /// Reference count
    pub ref_count: u32,
}

impl MappedSection {
    pub fn new(file_offset: u64, size: u64, flags: u32) -> Self {
        Self {
            vaddr: 0,
            paddr: 0,
            size,
            file_offset,
            flags,
            ref_count: 0,
        }
    }
}

/// Index serializer for converting in-memory structures to disk format
pub struct IndexSerializer {
    /// Output buffer for serialized data
    pub buffer: [u8; 65536], // 64KB buffer for kernel space
    /// Current write position
    pub position: usize,
    /// File header
    pub file_header: IndexFileHeader,
    /// Checksum accumulator
    pub checksum: u32,
}

impl IndexSerializer {
    pub fn new() -> Self {
        Self {
            buffer: [0; 65536],
            position: 0,
            file_header: IndexFileHeader::new(),
            checksum: 0,
        }
    }

    /// Begin serialization with file header
    pub fn begin(&mut self) -> Result<(), AnnsError> {
        self.position = 0;
        self.checksum = 0;
        
        // Reserve space for file header
        if self.buffer.len() < IndexFileHeader::SIZE {
            return Err(AnnsError::OutOfMemory);
        }
        
        self.position = IndexFileHeader::SIZE;
        Ok(())
    }

    /// Write index header to buffer
    pub fn write_index_header(&mut self, header: &AnnsIndexHeader) -> Result<u64, AnnsError> {
        let header_size = mem::size_of::<AnnsIndexHeader>();
        
        if self.position + header_size > self.buffer.len() {
            return Err(AnnsError::OutOfMemory);
        }

        let offset = self.position as u64;
        
        unsafe {
            let header_bytes = slice::from_raw_parts(
                header as *const _ as *const u8,
                header_size
            );
            self.buffer[self.position..self.position + header_size]
                .copy_from_slice(header_bytes);
        }
        
        self.update_checksum(&self.buffer[self.position..self.position + header_size]);
        self.position += header_size;
        
        self.file_header.index_header_offset = offset;
        Ok(offset)
    }

    /// Write layers section to buffer
    pub fn write_layers(&mut self, layers: &[HnswLayer]) -> Result<u64, AnnsError> {
        let layer_size = mem::size_of::<HnswLayer>();
        let total_size = layer_size * layers.len();
        
        if self.position + total_size > self.buffer.len() {
            return Err(AnnsError::OutOfMemory);
        }

        let offset = self.position as u64;
        
        for layer in layers {
            unsafe {
                let layer_bytes = slice::from_raw_parts(
                    layer as *const _ as *const u8,
                    layer_size
                );
                self.buffer[self.position..self.position + layer_size]
                    .copy_from_slice(layer_bytes);
            }
            
            self.update_checksum(&self.buffer[self.position..self.position + layer_size]);
            self.position += layer_size;
        }
        
        self.file_header.layers_offset = offset;
        self.file_header.layers_size = total_size as u64;
        Ok(offset)
    }

    /// Write nodes section to buffer
    pub fn write_nodes(&mut self, nodes: &[HnswNode]) -> Result<u64, AnnsError> {
        let node_size = mem::size_of::<HnswNode>();
        let total_size = node_size * nodes.len();
        
        if self.position + total_size > self.buffer.len() {
            return Err(AnnsError::OutOfMemory);
        }

        let offset = self.position as u64;
        
        for node in nodes {
            unsafe {
                let node_bytes = slice::from_raw_parts(
                    node as *const _ as *const u8,
                    node_size
                );
                self.buffer[self.position..self.position + node_size]
                    .copy_from_slice(node_bytes);
            }
            
            self.update_checksum(&self.buffer[self.position..self.position + node_size]);
            self.position += node_size;
        }
        
        self.file_header.nodes_offset = offset;
        self.file_header.nodes_size = total_size as u64;
        Ok(offset)
    }

    /// Write connections section to buffer
    pub fn write_connections(&mut self, connections: &[u64]) -> Result<u64, AnnsError> {
        let connection_size = mem::size_of::<u64>();
        let total_size = connection_size * connections.len();
        
        if self.position + total_size > self.buffer.len() {
            return Err(AnnsError::OutOfMemory);
        }

        let offset = self.position as u64;
        
        unsafe {
            let connections_bytes = slice::from_raw_parts(
                connections.as_ptr() as *const u8,
                total_size
            );
            self.buffer[self.position..self.position + total_size]
                .copy_from_slice(connections_bytes);
        }
        
        self.update_checksum(&self.buffer[self.position..self.position + total_size]);
        self.position += total_size;
        
        self.file_header.connections_offset = offset;
        self.file_header.connections_size = total_size as u64;
        Ok(offset)
    }

    /// Write vector data section to buffer
    pub fn write_vectors(&mut self, vectors: &[u8]) -> Result<u64, AnnsError> {
        if self.position + vectors.len() > self.buffer.len() {
            return Err(AnnsError::OutOfMemory);
        }

        let offset = self.position as u64;
        
        self.buffer[self.position..self.position + vectors.len()]
            .copy_from_slice(vectors);
        
        self.update_checksum(&vectors);
        self.position += vectors.len();
        
        self.file_header.vectors_offset = offset;
        self.file_header.vectors_size = vectors.len() as u64;
        Ok(offset)
    }

    /// Finalize serialization and write file header
    pub fn finalize(&mut self) -> Result<&[u8], AnnsError> {
        // Update file header with final information
        self.file_header.file_size = self.position as u64;
        self.file_header.modified_timestamp = 0; // TODO: get kernel time
        self.file_header.file_checksum = self.checksum;
        
        // Write file header at the beginning
        unsafe {
            let header_bytes = slice::from_raw_parts(
                &self.file_header as *const _ as *const u8,
                IndexFileHeader::SIZE
            );
            self.buffer[0..IndexFileHeader::SIZE].copy_from_slice(header_bytes);
        }
        
        Ok(&self.buffer[0..self.position])
    }

    /// Update running checksum
    fn update_checksum(&mut self, data: &[u8]) {
        for &byte in data {
            self.checksum = self.checksum.wrapping_add(byte as u32);
        }
    }

    /// Reset serializer for reuse
    pub fn reset(&mut self) {
        self.position = 0;
        self.checksum = 0;
        self.file_header = IndexFileHeader::new();
        // Keep buffer as-is for reuse
    }
}

/// Index deserializer for loading from disk format
pub struct IndexDeserializer {
    /// Input data buffer
    pub data: *const u8,
    /// Data size
    pub size: usize,
    /// Current read position
    pub position: usize,
    /// File header
    pub file_header: IndexFileHeader,
    /// Index header
    pub index_header: AnnsIndexHeader,
}

impl IndexDeserializer {
    pub fn new(data: *const u8, size: usize) -> Result<Self, AnnsError> {
        if size < IndexFileHeader::SIZE {
            return Err(AnnsError::InvalidFormat);
        }

        // Read file header
        let file_header = unsafe {
            ptr::read_unaligned(data as *const IndexFileHeader)
        };

        // Validate file header
        if file_header.magic != INDEX_FILE_MAGIC {
            return Err(AnnsError::InvalidFormat);
        }

        if file_header.version != INDEX_FORMAT_VERSION {
            return Err(AnnsError::InvalidFormat);
        }

        if file_header.file_size > size as u64 {
            return Err(AnnsError::InvalidFormat);
        }

        // Read index header
        let index_header_offset = file_header.index_header_offset as usize;
        if index_header_offset + mem::size_of::<AnnsIndexHeader>() > size {
            return Err(AnnsError::InvalidFormat);
        }

        let index_header = unsafe {
            ptr::read_unaligned(
                data.add(index_header_offset) as *const AnnsIndexHeader
            )
        };

        // Validate index header
        if index_header.magic != AnnsIndexHeader::MAGIC {
            return Err(AnnsError::InvalidFormat);
        }

        Ok(Self {
            data,
            size,
            position: IndexFileHeader::SIZE,
            file_header,
            index_header,
        })
    }

    /// Read layers section
    pub fn read_layers(&self) -> Result<&[HnswLayer], AnnsError> {
        let offset = self.file_header.layers_offset as usize;
        let size = self.file_header.layers_size as usize;
        
        if offset + size > self.size {
            return Err(AnnsError::CorruptedIndex);
        }

        let layer_count = size / mem::size_of::<HnswLayer>();
        
        unsafe {
            let layers_ptr = self.data.add(offset) as *const HnswLayer;
            Ok(slice::from_raw_parts(layers_ptr, layer_count))
        }
    }

    /// Read nodes section
    pub fn read_nodes(&self) -> Result<&[HnswNode], AnnsError> {
        let offset = self.file_header.nodes_offset as usize;
        let size = self.file_header.nodes_size as usize;
        
        if offset + size > self.size {
            return Err(AnnsError::CorruptedIndex);
        }

        let node_count = size / mem::size_of::<HnswNode>();
        
        unsafe {
            let nodes_ptr = self.data.add(offset) as *const HnswNode;
            Ok(slice::from_raw_parts(nodes_ptr, node_count))
        }
    }

    /// Read connections section
    pub fn read_connections(&self) -> Result<&[u64], AnnsError> {
        let offset = self.file_header.connections_offset as usize;
        let size = self.file_header.connections_size as usize;
        
        if offset + size > self.size {
            return Err(AnnsError::CorruptedIndex);
        }

        let connection_count = size / mem::size_of::<u64>();
        
        unsafe {
            let connections_ptr = self.data.add(offset) as *const u64;
            Ok(slice::from_raw_parts(connections_ptr, connection_count))
        }
    }

    /// Read vector data section
    pub fn read_vectors(&self) -> Result<&[u8], AnnsError> {
        let offset = self.file_header.vectors_offset as usize;
        let size = self.file_header.vectors_size as usize;
        
        if offset + size > self.size {
            return Err(AnnsError::CorruptedIndex);
        }

        unsafe {
            Ok(slice::from_raw_parts(self.data.add(offset), size))
        }
    }

    /// Verify file integrity using checksum
    pub fn verify_integrity(&self) -> Result<(), AnnsError> {
        let mut checksum = 0u32;
        
        // Calculate checksum of all data except the checksum field itself
        unsafe {
            let data_slice = slice::from_raw_parts(
                self.data.add(IndexFileHeader::SIZE), 
                self.file_header.file_size as usize - IndexFileHeader::SIZE
            );
            
            for &byte in data_slice {
                checksum = checksum.wrapping_add(byte as u32);
            }
        }

        if checksum != self.file_header.file_checksum {
            return Err(AnnsError::CorruptedIndex);
        }

        Ok(())
    }
}

/// Memory mapping manager for large indices
pub struct MemoryMapper {
    /// Active mapped sections
    pub sections: [MappedSection; 16], // Fixed size for kernel space
    /// Number of active sections
    pub section_count: usize,
    /// Total mapped memory
    pub total_mapped: u64,
    /// Maximum mappable memory
    pub max_memory: u64,
}

impl MemoryMapper {
    pub fn new(max_memory_mb: u32) -> Self {
        Self {
            sections: [MappedSection::new(0, 0, 0); 16],
            section_count: 0,
            total_mapped: 0,
            max_memory: max_memory_mb as u64 * 1024 * 1024,
        }
    }

    /// Map a section of the index file into memory
    pub fn map_section(&mut self, file_offset: u64, size: u64, flags: u32) -> Result<usize, AnnsError> {
        if self.section_count >= self.sections.len() {
            return Err(AnnsError::OutOfMemory);
        }

        if self.total_mapped + size > self.max_memory {
            // Try to evict some sections
            self.evict_lru_section()?;
        }

        // Align to page boundaries
        let aligned_offset = (file_offset / PAGE_SIZE as u64) * PAGE_SIZE as u64;
        let aligned_size = ((size + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) * PAGE_SIZE as u64;

        let section_idx = self.section_count;
        self.sections[section_idx] = MappedSection::new(aligned_offset, aligned_size, flags);
        
        // TODO: Actual memory mapping would use kernel memory management functions
        // For now, simulate with dummy addresses
        self.sections[section_idx].vaddr = 0x1000000 + (section_idx as u64 * aligned_size);
        self.sections[section_idx].ref_count = 1;

        self.section_count += 1;
        self.total_mapped += aligned_size;

        Ok(section_idx)
    }

    /// Unmap a section from memory
    pub fn unmap_section(&mut self, section_idx: usize) -> Result<(), AnnsError> {
        if section_idx >= self.section_count {
            return Err(AnnsError::InvalidParameters);
        }

        let section = &mut self.sections[section_idx];
        if section.ref_count > 0 {
            section.ref_count -= 1;
        }

        if section.ref_count == 0 {
            // TODO: Actual unmapping would use kernel memory management
            self.total_mapped -= section.size;
            
            // Compact sections array
            for i in section_idx..self.section_count - 1 {
                self.sections[i] = self.sections[i + 1];
            }
            self.section_count -= 1;
        }

        Ok(())
    }

    /// Evict least recently used section
    fn evict_lru_section(&mut self) -> Result<(), AnnsError> {
        if self.section_count == 0 {
            return Err(AnnsError::OutOfMemory);
        }

        // Find section with lowest reference count
        let mut min_refs = u32::MAX;
        let mut victim_idx = 0;

        for i in 0..self.section_count {
            if self.sections[i].ref_count < min_refs {
                min_refs = self.sections[i].ref_count;
                victim_idx = i;
            }
        }

        self.unmap_section(victim_idx)
    }

    /// Get virtual address for a file offset
    pub fn get_virtual_address(&self, file_offset: u64) -> Option<u64> {
        for section in &self.sections[..self.section_count] {
            let section_end = section.file_offset + section.size;
            if file_offset >= section.file_offset && file_offset < section_end {
                let offset_in_section = file_offset - section.file_offset;
                return Some(section.vaddr + offset_in_section);
            }
        }
        None
    }

    /// Get memory mapping statistics
    pub fn get_stats(&self) -> MappingStats {
        MappingStats {
            active_sections: self.section_count as u32,
            total_mapped_bytes: self.total_mapped,
            max_memory_bytes: self.max_memory,
            fragmentation_score: self.calculate_fragmentation(),
        }
    }

    /// Calculate memory fragmentation score
    fn calculate_fragmentation(&self) -> u8 {
        if self.section_count <= 1 {
            return 0;
        }

        // Simple fragmentation metric based on number of sections
        let fragmentation = (self.section_count * 100) / 16; // Max 16 sections
        fragmentation.min(100) as u8
    }
}

/// Memory mapping statistics
#[derive(Debug, Clone, Copy)]
pub struct MappingStats {
    pub active_sections: u32,
    pub total_mapped_bytes: u64,
    pub max_memory_bytes: u64,
    pub fragmentation_score: u8,
}

/// Tests for serialization functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_file_header_size() {
        // Ensure header is reasonably sized and aligned
        assert!(IndexFileHeader::SIZE <= 256);
        assert_eq!(IndexFileHeader::SIZE % 8, 0); // 8-byte aligned
    }

    #[test]
    fn test_serializer_creation() {
        let mut serializer = IndexSerializer::new();
        assert_eq!(serializer.position, 0);
        assert_eq!(serializer.checksum, 0);
        
        serializer.begin().unwrap();
        assert_eq!(serializer.position, IndexFileHeader::SIZE);
    }

    #[test]
    fn test_memory_mapper() {
        let mut mapper = MemoryMapper::new(64); // 64MB
        assert_eq!(mapper.section_count, 0);
        assert_eq!(mapper.total_mapped, 0);

        let section_idx = mapper.map_section(0, PAGE_SIZE as u64, 0).unwrap();
        assert_eq!(section_idx, 0);
        assert_eq!(mapper.section_count, 1);
        assert_eq!(mapper.total_mapped, PAGE_SIZE as u64);
    }

    #[test]
    fn test_page_alignment() {
        let offset = 1000u64;
        let aligned = (offset / PAGE_SIZE as u64) * PAGE_SIZE as u64;
        assert_eq!(aligned, 0); // Should align down to page boundary
        
        let size = 5000u64;
        let aligned_size = ((size + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) * PAGE_SIZE as u64;
        assert_eq!(aligned_size, 8192); // Should align up to page boundary
    }
}