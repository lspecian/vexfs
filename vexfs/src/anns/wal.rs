//! Write-Ahead Log (WAL) for ANNS operations
//! 
//! This module provides functionality for logging ANNS operations
//! to ensure data consistency and recovery capabilities.

use std::vec::Vec;
use core::mem;
use crate::anns::AnnsError;

/// Entry types for WAL operations
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum WalEntryType {
    Insert = 1,
    Delete = 2,
    Update = 3,
    IndexRebuild = 4,
    Checkpoint = 5,
}

impl WalEntryType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(WalEntryType::Insert),
            2 => Some(WalEntryType::Delete),
            3 => Some(WalEntryType::Update),
            4 => Some(WalEntryType::IndexRebuild),
            5 => Some(WalEntryType::Checkpoint),
            _ => None,
        }
    }

    /// Convert to u8
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// WAL entry header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WalEntryHeader {
    pub entry_type: u8,
    pub timestamp: u64,
    pub sequence_number: u64,
    pub data_size: u32,
    pub checksum: u32,
}

impl WalEntryHeader {
    /// Create a new WAL entry header
    pub fn new(entry_type: WalEntryType, sequence_number: u64, data_size: u32) -> Self {
        Self {
            entry_type: entry_type.to_u8(),
            timestamp: 0, // Would be system timestamp in real implementation
            sequence_number,
            data_size,
            checksum: 0, // Calculated later
        }
    }

    /// Get header size
    pub fn size() -> usize {
        mem::size_of::<Self>()
    }

    /// Calculate checksum for the header and data
    pub fn calculate_checksum(&mut self, data: &[u8]) {
        let mut checksum = 0u32;
        
        // Include header fields in checksum (except checksum itself)
        checksum = checksum.wrapping_add(self.entry_type as u32);
        checksum = checksum.wrapping_add((self.timestamp & 0xFFFFFFFF) as u32);
        checksum = checksum.wrapping_add((self.timestamp >> 32) as u32);
        checksum = checksum.wrapping_add((self.sequence_number & 0xFFFFFFFF) as u32);
        checksum = checksum.wrapping_add((self.sequence_number >> 32) as u32);
        checksum = checksum.wrapping_add(self.data_size);
        
        // Include data in checksum
        for &byte in data {
            checksum = checksum.wrapping_add(byte as u32);
        }
        
        self.checksum = checksum;
    }

    /// Verify checksum
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        let mut expected_checksum = 0u32;
        
        expected_checksum = expected_checksum.wrapping_add(self.entry_type as u32);
        expected_checksum = expected_checksum.wrapping_add((self.timestamp & 0xFFFFFFFF) as u32);
        expected_checksum = expected_checksum.wrapping_add((self.timestamp >> 32) as u32);
        expected_checksum = expected_checksum.wrapping_add((self.sequence_number & 0xFFFFFFFF) as u32);
        expected_checksum = expected_checksum.wrapping_add((self.sequence_number >> 32) as u32);
        expected_checksum = expected_checksum.wrapping_add(self.data_size);
        
        for &byte in data {
            expected_checksum = expected_checksum.wrapping_add(byte as u32);
        }
        
        expected_checksum == self.checksum
    }
}

/// WAL entry containing header and data
#[derive(Debug, Clone)]
pub struct WalEntry {
    pub header: WalEntryHeader,
    pub data: Vec<u8>,
}

impl WalEntry {
    /// Create a new WAL entry
    pub fn new(entry_type: WalEntryType, sequence_number: u64, data: Vec<u8>) -> Self {
        let mut header = WalEntryHeader::new(entry_type, sequence_number, data.len() as u32);
        header.calculate_checksum(&data);
        
        Self { header, data }
    }

    /// Verify the entry's integrity
    pub fn verify(&self) -> bool {
        self.header.verify_checksum(&self.data) && 
        self.data.len() == self.header.data_size as usize
    }

    /// Get entry type
    pub fn entry_type(&self) -> Option<WalEntryType> {
        WalEntryType::from_u8(self.header.entry_type)
    }

    /// Get sequence number
    pub fn sequence_number(&self) -> u64 {
        self.header.sequence_number
    }

    /// Get timestamp
    pub fn timestamp(&self) -> u64 {
        self.header.timestamp
    }

    /// Get data size
    pub fn data_size(&self) -> u32 {
        self.header.data_size
    }

    /// Get data reference
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Serialize entry to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        // Serialize header
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &self.header as *const WalEntryHeader as *const u8,
                mem::size_of::<WalEntryHeader>(),
            )
        };
        result.extend_from_slice(header_bytes);
        
        // Serialize data
        result.extend_from_slice(&self.data);
        
        result
    }

    /// Deserialize entry from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, AnnsError> {
        if bytes.len() < WalEntryHeader::size() {
            return Err(AnnsError::WalCorrupted);
        }

        // Deserialize header
        let header = unsafe {
            *(bytes.as_ptr() as *const WalEntryHeader)
        };

        let data_start = WalEntryHeader::size();
        let data_end = data_start + header.data_size as usize;
        
        if bytes.len() < data_end {
            return Err(AnnsError::WalCorrupted);
        }

        // Deserialize data
        let data = bytes[data_start..data_end].to_vec();
        
        let entry = Self { header, data };
        
        // Verify integrity
        if !entry.verify() {
            return Err(AnnsError::WalCorrupted);
        }
        
        Ok(entry)
    }
}

/// Write-Ahead Log manager
pub struct WalManager {
    entries: Vec<WalEntry>,
    next_sequence: u64,
    max_entries: usize,
    total_size: usize,
    max_size: usize,
}

impl WalManager {
    /// Create a new WAL manager
    pub fn new(max_entries: usize, max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            next_sequence: 1,
            max_entries,
            total_size: 0,
            max_size,
        }
    }

    /// Append a new entry to the WAL
    pub fn append(&mut self, entry_type: WalEntryType, data: Vec<u8>) -> Result<u64, AnnsError> {
        let entry = WalEntry::new(entry_type, self.next_sequence, data);
        let entry_size = entry.serialize().len();
        
        // Check size limits
        if self.total_size + entry_size > self.max_size {
            return Err(AnnsError::WalFull);
        }
        
        if self.entries.len() >= self.max_entries {
            // Remove oldest entry to make space
            let old_entry = self.entries.remove(0);
            self.total_size -= old_entry.serialize().len();
        }
        
        let sequence = self.next_sequence;
        self.entries.push(entry);
        self.total_size += entry_size;
        self.next_sequence += 1;
        
        Ok(sequence)
    }

    /// Get entry by sequence number
    pub fn get_entry(&self, sequence: u64) -> Option<&WalEntry> {
        self.entries.iter().find(|entry| entry.sequence_number() == sequence)
    }

    /// Get all entries since a given sequence number
    pub fn get_entries_since(&self, sequence: u64) -> Vec<&WalEntry> {
        self.entries.iter()
            .filter(|entry| entry.sequence_number() >= sequence)
            .collect()
    }

    /// Get all entries of a specific type
    pub fn get_entries_by_type(&self, entry_type: WalEntryType) -> Vec<&WalEntry> {
        self.entries.iter()
            .filter(|entry| entry.entry_type() == Some(entry_type))
            .collect()
    }

    /// Clear all entries up to a given sequence number
    pub fn truncate(&mut self, sequence: u64) -> Result<(), AnnsError> {
        let mut removed_size = 0;
        
        self.entries.retain(|entry| {
            if entry.sequence_number() <= sequence {
                removed_size += entry.serialize().len();
                false
            } else {
                true
            }
        });
        
        self.total_size -= removed_size;
        Ok(())
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_size = 0;
        self.next_sequence = 1;
    }

    /// Get current entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get total size in bytes
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get next sequence number
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Check if WAL is full
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.max_entries || self.total_size >= self.max_size
    }

    /// Get statistics
    pub fn stats(&self) -> WalStats {
        WalStats {
            entry_count: self.entries.len(),
            total_size: self.total_size,
            max_entries: self.max_entries,
            max_size: self.max_size,
            next_sequence: self.next_sequence,
            utilization_percent: (self.total_size as f32 / self.max_size as f32) * 100.0,
        }
    }

    /// Serialize entire WAL to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        
        for entry in &self.entries {
            let entry_bytes = entry.serialize();
            result.extend_from_slice(&entry_bytes);
        }
        
        result
    }

    /// Deserialize WAL from bytes
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), AnnsError> {
        self.clear();
        
        let mut offset = 0;
        
        while offset < data.len() {
            if offset + WalEntryHeader::size() > data.len() {
                break; // Incomplete header
            }
            
            // Read header to get entry size
            let header = unsafe {
                *((data.as_ptr() as *const u8).add(offset) as *const WalEntryHeader)
            };
            
            let entry_size = WalEntryHeader::size() + header.data_size as usize;
            
            if offset + entry_size > data.len() {
                return Err(AnnsError::WalCorrupted);
            }
            
            // Deserialize entire entry
            let entry_bytes = &data[offset..offset + entry_size];
            let entry = WalEntry::deserialize(entry_bytes)?;
            
            // Get sequence number before moving entry
            let sequence_number = entry.sequence_number();
            
            self.entries.push(entry);
            self.total_size += entry_size;
            
            // Update next sequence if this entry has a higher sequence number
            if sequence_number >= self.next_sequence {
                self.next_sequence = sequence_number + 1;
            }
            
            offset += entry_size;
        }
        
        Ok(())
    }
}

/// WAL statistics
#[derive(Debug, Clone)]
pub struct WalStats {
    pub entry_count: usize,
    pub total_size: usize,
    pub max_entries: usize,
    pub max_size: usize,
    pub next_sequence: u64,
    pub utilization_percent: f32,
}

impl WalStats {
    /// Check if utilization is high (>80%)
    pub fn is_high_utilization(&self) -> bool {
        self.utilization_percent > 80.0
    }

    /// Check if WAL is nearly full (>95%)
    pub fn is_nearly_full(&self) -> bool {
        self.utilization_percent > 95.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_entry_type() {
        assert_eq!(WalEntryType::Insert.to_u8(), 1);
        assert_eq!(WalEntryType::from_u8(1), Some(WalEntryType::Insert));
        assert_eq!(WalEntryType::from_u8(99), None);
    }

    #[test]
    fn test_wal_entry_creation() {
        let data = vec![1, 2, 3, 4];
        let entry = WalEntry::new(WalEntryType::Insert, 1, data.clone());
        
        assert_eq!(entry.entry_type(), Some(WalEntryType::Insert));
        assert_eq!(entry.sequence_number(), 1);
        assert_eq!(entry.data_size(), 4);
        assert_eq!(entry.data(), &data);
        assert!(entry.verify());
    }

    #[test]
    fn test_wal_manager() {
        let mut wal = WalManager::new(10, 1024);
        assert_eq!(wal.entry_count(), 0);
        assert_eq!(wal.next_sequence(), 1);
        
        let seq = wal.append(WalEntryType::Insert, vec![1, 2, 3]).unwrap();
        assert_eq!(seq, 1);
        assert_eq!(wal.entry_count(), 1);
        assert_eq!(wal.next_sequence(), 2);
        
        let entry = wal.get_entry(1).unwrap();
        assert_eq!(entry.entry_type(), Some(WalEntryType::Insert));
    }

    #[test]
    fn test_wal_serialization() {
        let data = vec![1, 2, 3, 4];
        let entry = WalEntry::new(WalEntryType::Insert, 1, data);
        
        let serialized = entry.serialize();
        let deserialized = WalEntry::deserialize(&serialized).unwrap();
        
        assert_eq!(entry.entry_type(), deserialized.entry_type());
        assert_eq!(entry.sequence_number(), deserialized.sequence_number());
        assert_eq!(entry.data(), deserialized.data());
    }

    #[test]
    fn test_wal_stats() {
        let stats = WalStats {
            entry_count: 5,
            total_size: 850,
            max_entries: 10,
            max_size: 1000,
            next_sequence: 6,
            utilization_percent: 85.0,
        };
        
        assert!(stats.is_high_utilization());
        assert!(!stats.is_nearly_full());
    }
}