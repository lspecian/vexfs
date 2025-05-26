# Subtask 3.2 Implementation Plan: File and Directory Operations

## Current Test Status (Baseline)
- **Total Tests**: 62
- **Passing**: 58
- **Failing**: 4 (pre-existing failures, not related to file/dir ops)
  - `anns::memory_mgmt::tests::test_memory_stats`
  - `ondisk::tests::test_layout_sizes` 
  - `ondisk::tests::test_structure_alignment`
  - `ondisk::tests::test_structure_sizes`

## Implementation Approach

### Phase 1: Core File Operations Enhancement
1. **File Creation** (`vexfs_create_file`)
   - Inode allocation via `inode_mgmt.rs`
   - Block allocation via `space_alloc.rs`
   - Journal integration for consistency

2. **File I/O Operations** 
   - `vexfs_read_file` - Read data using inode block pointers
   - `vexfs_write_file` - Write data with block allocation
   - `vexfs_truncate_file` - Truncate with block deallocation

3. **File Management**
   - `vexfs_open_file` - File opening with permission checks
   - `vexfs_close_file` - File closing with synchronization
   - `vexfs_unlink_file` - File deletion with cleanup

### Phase 2: Directory Operations Enhancement
1. **Directory Structure Operations**
   - `vexfs_create_dir` - Directory creation with parent linking
   - `vexfs_delete_dir` - Directory deletion with emptiness checking
   - `vexfs_read_dir` - Directory entry listing

2. **Directory Navigation**
   - `vexfs_lookup_dir` - Path resolution and entry lookup
   - `vexfs_rename_entry` - Rename files/directories

3. **Link Operations**
   - `vexfs_link` - Hard link creation with reference counting
   - `vexfs_symlink` - Symbolic link creation

### Phase 3: Security and Concurrency
1. **Permission System**
   - UNIX permission model (rwx for owner/group/other)
   - UID/GID validation against inode metadata
   - Special permissions (sticky bit, setuid, setgid)

2. **Locking Mechanisms**
   - Per-inode locks for metadata operations
   - Directory-level locks for structure modifications
   - Reader-writer locks for read vs write operations

### Phase 4: Integration and Testing
1. **VFS Integration** - Ensure compatibility with existing FFI layer
2. **Journal Integration** - All operations properly journaled
3. **Error Handling** - Comprehensive error reporting
4. **Performance** - Block caching and read-ahead optimization

## Foundation Available
- ✅ Complete on-disk layout (VexfsSuperblock, VexfsInode, VexfsDirEntry)
- ✅ Serialization infrastructure (OnDiskSerializable trait)
- ✅ VFS interface layer with C FFI
- ✅ Build system with kernel module support
- ✅ Inode and space management foundations
- ✅ Journal infrastructure

## Implementation Files
- `vexfs/src/file_ops.rs` - Core file operations
- `vexfs/src/dir_ops.rs` - Directory operations  
- `vexfs/src/inode_mgmt.rs` - Integration point
- `vexfs/src/journal.rs` - Consistency integration

## Success Criteria
- All standard file operations working correctly
- All directory operations working correctly
- Proper permission checking and security enforcement
- Race condition prevention through locking
- Filesystem consistency maintained
- Integration with VFS interface functional
- Foundation ready for data read/write operations (subtask 3.4)