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
 */

//! Integration tests for Copy-on-Write and Snapshot functionality

use std::sync::Arc;
use vexfs::fs_core::{
    cow::{CowManager, CowMapping, CowExtent, CowBlockRef},
    snapshot::{SnapshotManager, SnapshotMetadata},
    cow_integration::{CowFilesystemOperations, CowConfig},
    operations::OperationContext,
    permissions::UserContext,
    inode::InodeManager,
    locking::LockManager,
};
use vexfs::storage::{StorageManager, TransactionManager, layout::VexfsLayout, block::BlockDevice};
use vexfs::shared::types::*;
use vexfs::shared::constants::*;

/// Create a test storage manager
fn create_test_storage() -> Arc<StorageManager> {
    let layout = VexfsLayout {
        total_blocks: 10000,
        block_size: VEXFS_DEFAULT_BLOCK_SIZE as u32,
        blocks_per_group: 1000,
        journal_blocks: 100,
        inode_table_blocks: 100,
        block_bitmap_blocks: 10,
        inode_bitmap_blocks: 10,
        superblock_blocks: 1,
        reserved_blocks: 100,
    };
    
    // Create a mock block device for testing
    let device = BlockDevice::new_memory(layout.total_blocks as usize * layout.block_size as usize);
    
    Arc::new(StorageManager::new(device, layout, 1024 * 1024).unwrap())
}

/// Create a test operation context
fn create_test_context(storage: Arc<StorageManager>) -> (OperationContext<'static>, InodeManager, LockManager) {
    let mut inode_manager = InodeManager::new((*storage).clone()).unwrap();
    let mut lock_manager = LockManager::new();
    let user = UserContext::root();
    
    let context = OperationContext::new(
        user,
        VEXFS_ROOT_INO,
        &mut inode_manager,
        &mut lock_manager,
    );
    
    (context, inode_manager, lock_manager)
}

#[test]
fn test_cow_block_ref_creation() {
    let block_ref = CowBlockRef::new(100);
    
    assert_eq!(block_ref.original_block, 100);
    assert_eq!(block_ref.current_block, 100);
    assert_eq!(block_ref.ref_count, 1);
    assert_eq!(block_ref.generation, 0);
    assert!(!block_ref.is_copied());
    assert!(!block_ref.is_shared());
    assert!(!block_ref.needs_cow());
}

#[test]
fn test_cow_block_ref_sharing() {
    let mut block_ref = CowBlockRef::new(100);
    block_ref.ref_count = 2;
    
    assert!(block_ref.is_shared());
    assert!(block_ref.needs_cow());
}

#[test]
fn test_cow_extent_creation() {
    let blocks = vec![100, 101, 102, 103];
    let extent = CowExtent::new(0, blocks);
    
    assert_eq!(extent.logical_start, 0);
    assert_eq!(extent.block_count, 4);
    assert_eq!(extent.blocks.len(), 4);
    assert_eq!(extent.get_physical_block(1), Some(101));
    assert_eq!(extent.get_physical_block(5), None);
}

#[test]
fn test_cow_extent_cow_operation() {
    let blocks = vec![100, 101, 102, 103];
    let mut extent = CowExtent::new(0, blocks);
    
    // Perform CoW on block at logical offset 1
    extent.cow_block(1, 200).unwrap();
    
    assert_eq!(extent.get_physical_block(1), Some(200));
    assert_eq!(extent.get_physical_block(0), Some(100)); // Unchanged
    assert_eq!(extent.get_physical_block(2), Some(102)); // Unchanged
}

#[test]
fn test_cow_mapping_creation() {
    let mapping = CowMapping::new(1);
    
    assert_eq!(mapping.inode, 1);
    assert_eq!(mapping.logical_size, 0);
    assert_eq!(mapping.generation, 0);
    assert_eq!(mapping.ref_count, 1);
    assert!(mapping.extents.is_empty());
}

#[test]
fn test_cow_mapping_add_extent() {
    let mut mapping = CowMapping::new(1);
    let blocks = vec![100, 101, 102];
    let extent = CowExtent::new(0, blocks);
    
    mapping.add_extent(extent).unwrap();
    
    assert_eq!(mapping.logical_size, 3);
    assert_eq!(mapping.generation, 1);
    assert_eq!(mapping.extents.len(), 1);
    assert_eq!(mapping.get_physical_block(1), Some(101));
}

#[test]
fn test_cow_mapping_overlapping_extents() {
    let mut mapping = CowMapping::new(1);
    
    // Add first extent
    let blocks1 = vec![100, 101, 102];
    let extent1 = CowExtent::new(0, blocks1);
    mapping.add_extent(extent1).unwrap();
    
    // Try to add overlapping extent
    let blocks2 = vec![200, 201];
    let extent2 = CowExtent::new(1, blocks2); // Overlaps with first extent
    
    assert!(mapping.add_extent(extent2).is_err());
}

#[test]
fn test_cow_mapping_snapshot_creation() {
    let mut mapping = CowMapping::new(1);
    let blocks = vec![100, 101, 102];
    let extent = CowExtent::new(0, blocks);
    mapping.add_extent(extent).unwrap();
    
    let snapshot = mapping.create_snapshot();
    
    assert_eq!(snapshot.inode, 1);
    assert_eq!(snapshot.logical_size, 3);
    assert!(snapshot.parent_mapping.is_some());
    assert!(snapshot.flags.contains(crate::fs_core::cow::CowMappingFlags::SNAPSHOT));
}

#[test]
fn test_cow_manager_creation() {
    let storage = create_test_storage();
    let cow_manager = CowManager::new(storage);
    
    // Test that we can get stats
    let stats = cow_manager.get_stats().unwrap();
    assert_eq!(stats.cow_operations, 0);
    assert_eq!(stats.blocks_copied, 0);
    assert_eq!(stats.snapshots_created, 0);
}

#[test]
fn test_cow_manager_mapping_creation() {
    let storage = create_test_storage();
    let cow_manager = CowManager::new(storage);
    
    // Get mapping for inode 1
    let mapping1 = cow_manager.get_mapping(1).unwrap();
    let mapping2 = cow_manager.get_mapping(1).unwrap();
    
    // Should return the same mapping
    assert!(Arc::ptr_eq(&mapping1, &mapping2));
    
    // Get mapping for different inode
    let mapping3 = cow_manager.get_mapping(2).unwrap();
    assert!(!Arc::ptr_eq(&mapping1, &mapping3));
}

#[test]
fn test_snapshot_metadata_creation() {
    let metadata = SnapshotMetadata::new(1, "test_snapshot".to_string(), 100);
    
    assert_eq!(metadata.id, 1);
    assert_eq!(metadata.name, "test_snapshot");
    assert_eq!(metadata.root_inode, 100);
    assert!(!metadata.is_readonly());
    assert!(!metadata.is_marked_for_deletion());
    assert_eq!(metadata.child_snapshots.len(), 0);
}

#[test]
fn test_snapshot_metadata_child_management() {
    let mut metadata = SnapshotMetadata::new(1, "parent".to_string(), 100);
    
    // Add children
    metadata.add_child(2);
    metadata.add_child(3);
    assert_eq!(metadata.child_snapshots.len(), 2);
    assert!(metadata.child_snapshots.contains(&2));
    assert!(metadata.child_snapshots.contains(&3));
    
    // Remove child
    metadata.remove_child(2);
    assert_eq!(metadata.child_snapshots.len(), 1);
    assert!(!metadata.child_snapshots.contains(&2));
    assert!(metadata.child_snapshots.contains(&3));
    
    // Adding same child twice should not duplicate
    metadata.add_child(3);
    assert_eq!(metadata.child_snapshots.len(), 1);
}

#[test]
fn test_snapshot_manager_creation() {
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage.clone()));
    let snapshot_manager = SnapshotManager::new(cow_manager, storage);
    
    // Test initial state
    let snapshots = snapshot_manager.list_snapshots().unwrap();
    assert_eq!(snapshots.len(), 0);
    
    let stats = snapshot_manager.get_stats().unwrap();
    assert_eq!(stats.total_snapshots, 0);
    assert_eq!(stats.active_snapshots, 0);
}

#[test]
fn test_snapshot_manager_create_snapshot() {
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage.clone()));
    let snapshot_manager = SnapshotManager::new(cow_manager, storage);
    
    // Create a snapshot
    let snapshot_id = snapshot_manager.create_snapshot(
        "test_snapshot".to_string(),
        VEXFS_ROOT_INO,
        None,
    ).unwrap();
    
    assert_eq!(snapshot_id, 1);
    
    // Verify snapshot was created
    let snapshots = snapshot_manager.list_snapshots().unwrap();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].id, 1);
    assert_eq!(snapshots[0].name, "test_snapshot");
    
    // Verify stats
    let stats = snapshot_manager.get_stats().unwrap();
    assert_eq!(stats.total_snapshots, 1);
    assert_eq!(stats.active_snapshots, 1);
}

#[test]
fn test_snapshot_manager_hierarchical_snapshots() {
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage.clone()));
    let snapshot_manager = SnapshotManager::new(cow_manager, storage);
    
    // Create parent snapshot
    let parent_id = snapshot_manager.create_snapshot(
        "parent".to_string(),
        VEXFS_ROOT_INO,
        None,
    ).unwrap();
    
    // Create child snapshot
    let child_id = snapshot_manager.create_snapshot(
        "child".to_string(),
        VEXFS_ROOT_INO,
        Some(parent_id),
    ).unwrap();
    
    // Verify hierarchy
    let child_metadata = snapshot_manager.get_snapshot(child_id).unwrap();
    assert_eq!(child_metadata.parent_snapshot, Some(parent_id));
    
    let snapshots = snapshot_manager.list_snapshots().unwrap();
    assert_eq!(snapshots.len(), 2);
}

#[test]
fn test_snapshot_manager_delete_snapshot() {
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage.clone()));
    let snapshot_manager = SnapshotManager::new(cow_manager, storage);
    
    // Create snapshot
    let snapshot_id = snapshot_manager.create_snapshot(
        "test".to_string(),
        VEXFS_ROOT_INO,
        None,
    ).unwrap();
    
    // Delete snapshot
    snapshot_manager.delete_snapshot(snapshot_id, true).unwrap();
    
    // Verify deletion
    let snapshots = snapshot_manager.list_snapshots().unwrap();
    assert_eq!(snapshots.len(), 0);
    
    let stats = snapshot_manager.get_stats().unwrap();
    assert_eq!(stats.active_snapshots, 0);
}

#[test]
fn test_cow_config_defaults() {
    let config = CowConfig::default();
    
    assert!(config.auto_cow);
    assert_eq!(config.max_snapshots, 100);
    assert_eq!(config.gc_interval_seconds, 3600);
    assert!(config.compress_snapshots);
    assert!(config.enable_incremental);
    assert_eq!(config.max_extent_size, 1024);
}

#[test]
fn test_cow_filesystem_operations_creation() {
    let storage = create_test_storage();
    let transaction_manager = Arc::new(TransactionManager::new(
        crate::storage::journal::VexfsJournal::new(VEXFS_DEFAULT_BLOCK_SIZE as u32, 100)
    ));
    let config = CowConfig::default();
    
    let cow_ops = CowFilesystemOperations::new(
        storage,
        transaction_manager,
        config,
    ).unwrap();
    
    // Test that we can get stats
    let stats = cow_ops.get_stats().unwrap();
    assert_eq!(stats.cow_stats.cow_operations, 0);
    assert_eq!(stats.snapshot_stats.total_snapshots, 0);
}

#[test]
fn test_garbage_collection() {
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage.clone()));
    let snapshot_manager = SnapshotManager::new(cow_manager, storage);
    
    // Create and delete a snapshot
    let snapshot_id = snapshot_manager.create_snapshot(
        "test".to_string(),
        VEXFS_ROOT_INO,
        None,
    ).unwrap();
    
    snapshot_manager.delete_snapshot(snapshot_id, false).unwrap(); // Mark for deletion
    
    // Run garbage collection
    let gc_result = snapshot_manager.garbage_collect().unwrap();
    
    assert_eq!(gc_result.snapshots_deleted, 1);
    assert_eq!(gc_result.errors, 0);
}

#[test]
fn test_space_efficiency_calculation() {
    use vexfs::fs_core::cow::CowStats;
    use vexfs::fs_core::snapshot::SnapshotStats;
    
    let cow_stats = CowStats {
        blocks_copied: 100,
        space_saved: 50 * VEXFS_DEFAULT_BLOCK_SIZE as u64,
        ..Default::default()
    };
    
    let snapshot_stats = SnapshotStats {
        total_space_used: 200 * VEXFS_DEFAULT_BLOCK_SIZE as u64,
        space_saved: 100 * VEXFS_DEFAULT_BLOCK_SIZE as u64,
        ..Default::default()
    };
    
    // Test space efficiency calculation
    let efficiency = cow_stats.space_efficiency();
    assert!(efficiency >= 0.0);
    assert!(efficiency <= 100.0);
    
    let compression_ratio = snapshot_stats.compression_ratio();
    assert!(compression_ratio >= 1.0);
}

#[test]
fn test_cow_stats_space_efficiency() {
    use vexfs::fs_core::cow::CowStats;
    
    let mut stats = CowStats::default();
    stats.blocks_copied = 100;
    stats.space_saved = 50 * VEXFS_DEFAULT_BLOCK_SIZE as u64;
    
    let efficiency = stats.space_efficiency();
    assert!(efficiency > 0.0);
    assert!(efficiency <= 100.0);
    
    // Test edge case: no blocks copied
    let empty_stats = CowStats::default();
    assert_eq!(empty_stats.space_efficiency(), 100.0);
}

#[test]
fn test_snapshot_stats_compression_ratio() {
    use vexfs::fs_core::snapshot::SnapshotStats;
    
    let mut stats = SnapshotStats::default();
    stats.total_space_used = 1000;
    stats.space_saved = 200;
    
    let ratio = stats.compression_ratio();
    assert_eq!(ratio, 1.2); // (1000 + 200) / 1000
    
    // Test edge case: no space used
    let empty_stats = SnapshotStats::default();
    assert_eq!(empty_stats.compression_ratio(), 1.0);
}

#[test]
fn test_concurrent_cow_operations() {
    use std::thread;
    use std::sync::Arc;
    
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage));
    
    let handles: Vec<_> = (0..10).map(|i| {
        let cow_manager = cow_manager.clone();
        thread::spawn(move || {
            let inode = i as u64 + 1;
            let mapping = cow_manager.get_mapping(inode).unwrap();
            
            // Each thread works with a different inode
            assert_eq!(mapping.read().inode, inode);
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_snapshot_consistency() {
    let storage = create_test_storage();
    let cow_manager = Arc::new(CowManager::new(storage.clone()));
    let snapshot_manager = SnapshotManager::new(cow_manager, storage);
    
    // Create multiple snapshots
    let snapshot1 = snapshot_manager.create_snapshot(
        "snapshot1".to_string(),
        VEXFS_ROOT_INO,
        None,
    ).unwrap();
    
    let snapshot2 = snapshot_manager.create_snapshot(
        "snapshot2".to_string(),
        VEXFS_ROOT_INO,
        Some(snapshot1),
    ).unwrap();
    
    let snapshot3 = snapshot_manager.create_snapshot(
        "snapshot3".to_string(),
        VEXFS_ROOT_INO,
        Some(snapshot2),
    ).unwrap();
    
    // Verify all snapshots exist and have correct relationships
    let snapshots = snapshot_manager.list_snapshots().unwrap();
    assert_eq!(snapshots.len(), 3);
    
    let snap2_meta = snapshot_manager.get_snapshot(snapshot2).unwrap();
    assert_eq!(snap2_meta.parent_snapshot, Some(snapshot1));
    
    let snap3_meta = snapshot_manager.get_snapshot(snapshot3).unwrap();
    assert_eq!(snap3_meta.parent_snapshot, Some(snapshot2));
}