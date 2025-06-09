/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

//! Comprehensive test suite for configurable data journaling

#[cfg(test)]
mod tests {
    use super::super::data_journaling::*;
    use crate::shared::config::{DataJournalingMode, DataJournalingConfig};
    use crate::storage::journal::VexfsJournal;
    use crate::shared::constants::*;

    fn create_test_journal() -> VexfsJournal {
        VexfsJournal::new(4096, 1024)
    }

    fn create_test_config(mode: DataJournalingMode) -> DataJournalingConfig {
        DataJournalingConfig {
            mode,
            cow_enabled: true,
            max_data_journal_size: VEXFS_DEFAULT_MAX_DATA_JOURNAL_SIZE,
            mmap_enabled: true,
            large_write_threshold: VEXFS_DEFAULT_LARGE_WRITE_THRESHOLD,
            data_compression_enabled: false,
            space_optimization_enabled: true,
            dynamic_switching_enabled: true,
        }
    }

    #[test]
    fn test_data_journaling_mode_conversion() {
        // Test conversion from u32
        assert_eq!(DataJournalingMode::from(VEXFS_DATA_JOURNAL_METADATA_ONLY), DataJournalingMode::MetadataOnly);
        assert_eq!(DataJournalingMode::from(VEXFS_DATA_JOURNAL_ORDERED), DataJournalingMode::OrderedData);
        assert_eq!(DataJournalingMode::from(VEXFS_DATA_JOURNAL_FULL), DataJournalingMode::FullDataJournaling);
        
        // Test conversion to u32
        assert_eq!(u32::from(DataJournalingMode::MetadataOnly), VEXFS_DATA_JOURNAL_METADATA_ONLY);
        assert_eq!(u32::from(DataJournalingMode::OrderedData), VEXFS_DATA_JOURNAL_ORDERED);
        assert_eq!(u32::from(DataJournalingMode::FullDataJournaling), VEXFS_DATA_JOURNAL_FULL);
        
        // Test invalid conversion defaults to OrderedData
        assert_eq!(DataJournalingMode::from(999), DataJournalingMode::OrderedData);
    }

    #[test]
    fn test_data_journaling_config_default() {
        let config = DataJournalingConfig::default();
        assert_eq!(config.mode, DataJournalingMode::OrderedData);
        assert!(config.cow_enabled);
        assert!(config.mmap_enabled);
        assert!(config.dynamic_switching_enabled);
        assert!(config.space_optimization_enabled);
        assert!(!config.data_compression_enabled);
        assert_eq!(config.max_data_journal_size, VEXFS_DEFAULT_MAX_DATA_JOURNAL_SIZE);
        assert_eq!(config.large_write_threshold, VEXFS_DEFAULT_LARGE_WRITE_THRESHOLD);
    }

    #[test]
    fn test_data_journaling_manager_creation() {
        let config = create_test_config(DataJournalingMode::OrderedData);
        let journal = create_test_journal();
        let manager = DataJournalingManager::new(config, journal);
        
        assert_eq!(manager.get_mode(), DataJournalingMode::OrderedData);
        assert_eq!(manager.cow_blocks.len(), 0);
        assert_eq!(manager.pending_ordered_writes.len(), 0);
    }

    #[test]
    fn test_metadata_only_mode() {
        let config = create_test_config(DataJournalingMode::MetadataOnly);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Test metadata-only write
        let test_data = b"test data for metadata only mode";
        let result = manager.write_data(1000, 0, test_data);
        assert!(result.is_ok());
        
        // Verify statistics
        let stats = manager.get_stats();
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.metadata_only_ops, 1);
        assert_eq!(stats.ordered_data_ops, 0);
        assert_eq!(stats.full_journal_ops, 0);
    }

    #[test]
    fn test_ordered_data_mode() {
        let config = create_test_config(DataJournalingMode::OrderedData);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Test ordered data write
        let test_data = b"test data for ordered mode";
        let result = manager.write_data(2000, 0, test_data);
        assert!(result.is_ok());
        
        // Verify pending writes
        assert_eq!(manager.pending_ordered_writes.len(), 1);
        
        // Verify statistics
        let stats = manager.get_stats();
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.metadata_only_ops, 0);
        assert_eq!(stats.ordered_data_ops, 1);
        assert_eq!(stats.full_journal_ops, 0);
    }

    #[test]
    fn test_full_data_journaling_mode() {
        let config = create_test_config(DataJournalingMode::FullDataJournaling);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Test full data journaling write
        let test_data = b"test data for full journaling mode";
        let result = manager.write_data(3000, 0, test_data);
        assert!(result.is_ok());
        
        // Verify statistics
        let stats = manager.get_stats();
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.metadata_only_ops, 0);
        assert_eq!(stats.ordered_data_ops, 0);
        assert_eq!(stats.full_journal_ops, 1);
        assert_eq!(stats.data_bytes_journaled, test_data.len() as u64);
    }

    #[test]
    fn test_large_write_handling() {
        let config = create_test_config(DataJournalingMode::FullDataJournaling);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Create large data that exceeds threshold
        let large_data = vec![0u8; (VEXFS_DEFAULT_LARGE_WRITE_THRESHOLD + 1024) as usize];
        let result = manager.write_data(4000, 0, &large_data);
        assert!(result.is_ok());
        
        // Verify statistics
        let stats = manager.get_stats();
        assert_eq!(stats.large_write_ops, 1);
        assert!(stats.data_bytes_journaled > 0);
    }

    #[test]
    fn test_cow_operations() {
        let config = create_test_config(DataJournalingMode::FullDataJournaling);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Create medium-sized data that should trigger COW
        let cow_data = vec![0u8; 8192]; // 8KB should trigger COW
        let result = manager.write_data(5000, 0, &cow_data);
        assert!(result.is_ok());
        
        // Verify COW was used
        let stats = manager.get_stats();
        assert_eq!(stats.cow_operations, 1);
        assert!(manager.cow_blocks.len() > 0);
    }

    #[test]
    fn test_dynamic_mode_switching() {
        let config = create_test_config(DataJournalingMode::MetadataOnly);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Initial mode
        assert_eq!(manager.get_mode(), DataJournalingMode::MetadataOnly);
        
        // Switch to ordered data mode
        let result = manager.set_mode(DataJournalingMode::OrderedData);
        assert!(result.is_ok());
        assert_eq!(manager.get_mode(), DataJournalingMode::OrderedData);
        
        // Switch to full journaling mode
        let result = manager.set_mode(DataJournalingMode::FullDataJournaling);
        assert!(result.is_ok());
        assert_eq!(manager.get_mode(), DataJournalingMode::FullDataJournaling);
    }

    #[test]
    fn test_dynamic_switching_disabled() {
        let mut config = create_test_config(DataJournalingMode::MetadataOnly);
        config.dynamic_switching_enabled = false;
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Try to switch mode when disabled
        let result = manager.set_mode(DataJournalingMode::OrderedData);
        assert!(result.is_err());
        assert_eq!(manager.get_mode(), DataJournalingMode::MetadataOnly);
    }

    #[test]
    fn test_pending_ordered_writes_flush() {
        let config = create_test_config(DataJournalingMode::OrderedData);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Add multiple ordered writes
        for i in 0..5 {
            let test_data = format!("test data {}", i);
            let result = manager.write_data(6000 + i as u64, 0, test_data.as_bytes());
            assert!(result.is_ok());
        }
        
        assert_eq!(manager.pending_ordered_writes.len(), 5);
        
        // Flush pending operations
        let result = manager.flush_pending_operations();
        assert!(result.is_ok());
        assert_eq!(manager.pending_ordered_writes.len(), 0);
    }

    #[test]
    fn test_cow_block_cleanup() {
        let config = create_test_config(DataJournalingMode::FullDataJournaling);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Create COW blocks
        let cow_data = vec![0u8; 8192];
        for i in 0..3 {
            let result = manager.write_data(7000 + i as u64, 0, &cow_data);
            assert!(result.is_ok());
        }
        
        assert!(manager.cow_blocks.len() > 0);
        
        // Simulate reference count going to zero
        for cow_block in &mut manager.cow_blocks {
            cow_block.ref_count = 0;
        }
        
        // Cleanup COW blocks
        let result = manager.cleanup_cow_blocks();
        assert!(result.is_ok());
        assert_eq!(manager.cow_blocks.len(), 0);
    }

    #[test]
    fn test_journal_space_optimization() {
        let config = create_test_config(DataJournalingMode::FullDataJournaling);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Perform optimization
        let result = manager.optimize_journal_space();
        assert!(result.is_ok());
    }

    #[test]
    fn test_space_optimization_disabled() {
        let mut config = create_test_config(DataJournalingMode::FullDataJournaling);
        config.space_optimization_enabled = false;
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Optimization should still succeed but do nothing
        let result = manager.optimize_journal_space();
        assert!(result.is_ok());
    }

    #[test]
    fn test_configuration_update() {
        let config = create_test_config(DataJournalingMode::MetadataOnly);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Update configuration
        let new_config = create_test_config(DataJournalingMode::FullDataJournaling);
        let result = manager.update_config(new_config);
        assert!(result.is_ok());
        assert_eq!(manager.get_mode(), DataJournalingMode::FullDataJournaling);
    }

    #[test]
    fn test_statistics_tracking() {
        let config = create_test_config(DataJournalingMode::OrderedData);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        // Perform various operations
        let test_data = b"test data";
        
        // Switch to metadata-only and write
        manager.set_mode(DataJournalingMode::MetadataOnly).unwrap();
        manager.write_data(8000, 0, test_data).unwrap();
        
        // Switch to ordered and write
        manager.set_mode(DataJournalingMode::OrderedData).unwrap();
        manager.write_data(8001, 0, test_data).unwrap();
        
        // Switch to full journaling and write
        manager.set_mode(DataJournalingMode::FullDataJournaling).unwrap();
        manager.write_data(8002, 0, test_data).unwrap();
        
        // Check statistics
        let stats = manager.get_stats();
        assert_eq!(stats.total_operations, 3);
        assert_eq!(stats.metadata_only_ops, 1);
        assert_eq!(stats.ordered_data_ops, 1);
        assert_eq!(stats.full_journal_ops, 1);
    }

    #[test]
    fn test_data_journal_operation_types() {
        // Test operation type creation and comparison
        let op1 = DataJournalOperation {
            op_type: DataJournalOpType::DirectWrite,
            block_number: 1000,
            offset: 0,
            length: 100,
            original_data: vec![0; 100],
            new_data: vec![1; 100],
            cow_block: None,
        };
        
        assert_eq!(op1.op_type, DataJournalOpType::DirectWrite);
        assert_eq!(op1.block_number, 1000);
        assert_eq!(op1.length, 100);
        assert!(op1.cow_block.is_none());
    }

    #[test]
    fn test_cow_block_creation() {
        let cow_block = CowBlock {
            original_block: 1000,
            cow_block: 2000,
            ref_count: 1,
            data_size: 4096,
            original_checksum: 0x12345678,
            cow_checksum: 0x87654321,
        };
        
        assert_eq!(cow_block.original_block, 1000);
        assert_eq!(cow_block.cow_block, 2000);
        assert_eq!(cow_block.ref_count, 1);
        assert_eq!(cow_block.data_size, 4096);
    }

    #[test]
    fn test_multiple_mode_operations() {
        let config = create_test_config(DataJournalingMode::MetadataOnly);
        let journal = create_test_journal();
        let mut manager = DataJournalingManager::new(config, journal);
        
        let test_data = b"multi-mode test data";
        
        // Test all three modes in sequence
        for (i, mode) in [
            DataJournalingMode::MetadataOnly,
            DataJournalingMode::OrderedData,
            DataJournalingMode::FullDataJournaling,
        ].iter().enumerate() {
            manager.set_mode(*mode).unwrap();
            let result = manager.write_data(9000 + i as u64, 0, test_data);
            assert!(result.is_ok());
        }
        
        // Flush all pending operations
        manager.flush_pending_operations().unwrap();
        
        let stats = manager.get_stats();
        assert_eq!(stats.total_operations, 3);
    }
}