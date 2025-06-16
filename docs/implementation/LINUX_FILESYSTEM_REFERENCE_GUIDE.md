# Linux Filesystem Reference Guide for VexFS Implementation

## Overview

This document provides a reference guide for implementing VexFS as a legitimate Linux filesystem, based on established kernel patterns and best practices.

## Key Linux Filesystem Concepts

### 1. Mount Functions

#### `mount_nodev()` vs `mount_bdev()`

**Current VexFS (In-Memory)**:
```c
static struct dentry *vexfs_v2_mount(struct file_system_type *fs_type,
                                     int flags, const char *dev_name, void *data)
{
    return mount_nodev(fs_type, flags, data, vexfs_v2_fill_super);
}
```

**Target VexFS (Block Device)**:
```c
static struct dentry *vexfs_v2_mount(struct file_system_type *fs_type,
                                     int flags, const char *dev_name, void *data)
{
    return mount_bdev(fs_type, flags, dev_name, data, vexfs_v2_fill_super);
}
```

#### Key Differences:
- **`mount_nodev()`**: For pseudo-filesystems (tmpfs, procfs, sysfs)
- **`mount_bdev()`**: For real block device filesystems (ext4, xfs, vexfs)

### 2. Superblock Structure

#### Minimal Superblock Layout
```c
struct vexfs_superblock {
    __le32 s_magic;           /* Magic number: 0x56455846 ('VEXF') */
    __le32 s_version;         /* Filesystem version */
    __le32 s_block_size;      /* Block size in bytes */
    __le64 s_blocks_count;    /* Total blocks in filesystem */
    __le64 s_free_blocks;     /* Free blocks count */
    __le64 s_inodes_count;    /* Total inodes */
    __le64 s_free_inodes;     /* Free inodes count */
    __le32 s_first_data_block; /* First data block */
    __le32 s_inode_size;      /* Size of inode structure */
    __le32 s_state;           /* Filesystem state */
    __le32 s_errors;          /* Error handling policy */
    __le64 s_lastcheck;       /* Last check time */
    __le64 s_checkinterval;   /* Check interval */
    __u8   s_uuid[16];        /* Filesystem UUID */
    char   s_volume_name[16]; /* Volume name */
    __u8   s_reserved[928];   /* Reserved for future use */
} __packed;
```

#### Filesystem States
```c
#define VEXFS_VALID_FS    0x0001  /* Cleanly unmounted */
#define VEXFS_ERROR_FS    0x0002  /* Errors detected */
#define VEXFS_DIRTY_FS    0x0004  /* Mounted, not clean */
```

### 3. Superblock Operations

#### Essential Functions
```c
static const struct super_operations vexfs_super_ops = {
    .alloc_inode    = vexfs_alloc_inode,
    .destroy_inode  = vexfs_destroy_inode,
    .write_inode    = vexfs_write_inode,
    .evict_inode    = vexfs_evict_inode,
    .put_super      = vexfs_put_super,
    .sync_fs        = vexfs_sync_fs,
    .statfs         = vexfs_statfs,
    .remount_fs     = vexfs_remount,
    .show_options   = vexfs_show_options,
};
```

### 4. Block I/O Operations

#### Reading Superblock
```c
static int vexfs_read_superblock(struct super_block *sb)
{
    struct buffer_head *bh;
    struct vexfs_superblock *disk_sb;
    
    /* Read superblock from block 0 */
    bh = sb_bread(sb, 0);
    if (!bh) {
        printk(KERN_ERR "VexFS: Cannot read superblock\n");
        return -EIO;
    }
    
    disk_sb = (struct vexfs_superblock *)bh->b_data;
    
    /* Validate magic number */
    if (le32_to_cpu(disk_sb->s_magic) != VEXFS_MAGIC) {
        printk(KERN_ERR "VexFS: Invalid magic number\n");
        brelse(bh);
        return -EINVAL;
    }
    
    /* Copy superblock data */
    // ... copy fields to in-memory superblock ...
    
    brelse(bh);
    return 0;
}
```

#### Writing Superblock
```c
static int vexfs_write_superblock(struct super_block *sb)
{
    struct buffer_head *bh;
    struct vexfs_superblock *disk_sb;
    
    bh = sb_bread(sb, 0);
    if (!bh)
        return -EIO;
    
    disk_sb = (struct vexfs_superblock *)bh->b_data;
    
    /* Update superblock fields */
    // ... update fields from in-memory data ...
    
    mark_buffer_dirty(bh);
    sync_dirty_buffer(bh);
    brelse(bh);
    
    return 0;
}
```

### 5. Fill Super Function Pattern

#### Standard Implementation
```c
static int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    struct vexfs_sb_info *sbi;
    int ret;
    
    /* Allocate private superblock info */
    sbi = kzalloc(sizeof(*sbi), GFP_KERNEL);
    if (!sbi)
        return -ENOMEM;
    
    sb->s_fs_info = sbi;
    
    /* Set basic superblock parameters */
    sb->s_magic = VEXFS_MAGIC;
    sb->s_op = &vexfs_super_ops;
    sb->s_export_op = &vexfs_export_ops;
    sb->s_xattr = vexfs_xattr_handlers;
    
    /* Set block size (must be power of 2, >= 512) */
    if (!sb_set_blocksize(sb, VEXFS_DEFAULT_BLOCK_SIZE)) {
        if (!silent)
            printk(KERN_ERR "VexFS: Bad block size\n");
        ret = -EINVAL;
        goto failed;
    }
    
    /* Read and validate superblock */
    ret = vexfs_read_superblock(sb);
    if (ret)
        goto failed;
    
    /* Create root inode */
    ret = vexfs_create_root_inode(sb);
    if (ret)
        goto failed;
    
    return 0;
    
failed:
    kfree(sbi);
    sb->s_fs_info = NULL;
    return ret;
}
```

### 6. Kill Super Function

#### Standard Implementation
```c
static void vexfs_kill_sb(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = sb->s_fs_info;
    
    if (sbi) {
        /* Write superblock if dirty */
        if (sb->s_dirt)
            vexfs_write_superblock(sb);
        
        /* Clean up private data */
        kfree(sbi);
        sb->s_fs_info = NULL;
    }
    
    kill_block_super(sb);
}
```

## Implementation Strategy for VexFS

### Phase 1: Minimal Superblock Persistence

1. **Define VexFS Superblock Structure**
   - Magic number: `0x56455846` ('VEXF')
   - Version, block size, block counts
   - Filesystem state and error handling

2. **Implement Superblock I/O**
   - `vexfs_read_superblock()` function
   - `vexfs_write_superblock()` function
   - Proper error handling and validation

3. **Update Mount/Unmount**
   - Change from `mount_nodev()` to `mount_bdev()`
   - Update `fill_super` to read from disk
   - Update `kill_super` to write to disk

4. **Add Filesystem Registration**
   - Update `file_system_type` structure
   - Add proper filesystem flags
   - Ensure compatibility with VFS layer

### Phase 2: Basic File Operations

1. **Inode Management**
   - Define on-disk inode structure
   - Implement inode allocation/deallocation
   - Add inode read/write operations

2. **Block Allocation**
   - Simple bitmap-based allocation
   - Block allocation/deallocation functions
   - Free space management

3. **Directory Operations**
   - Simple directory entry format
   - Directory read/write operations
   - Name lookup implementation

## Key Linux Kernel Helpers

### Buffer Head Operations
- `sb_bread()` - Read block from device
- `mark_buffer_dirty()` - Mark buffer for writing
- `sync_dirty_buffer()` - Synchronous write
- `brelse()` - Release buffer head

### Superblock Helpers
- `sb_set_blocksize()` - Set filesystem block size
- `kill_block_super()` - Standard block device cleanup
- `mount_bdev()` - Mount block device filesystem

### Memory Management
- `kzalloc()` / `kfree()` - Kernel memory allocation
- `GFP_KERNEL` - Standard allocation flags

## Error Handling Patterns

### Standard Error Codes
- `-ENOMEM` - Out of memory
- `-EIO` - I/O error
- `-EINVAL` - Invalid argument
- `-ENOSPC` - No space left on device
- `-ENOENT` - No such file or directory

### Cleanup Patterns
```c
int function(void)
{
    int ret = 0;
    void *ptr = NULL;
    
    ptr = kzalloc(size, GFP_KERNEL);
    if (!ptr) {
        ret = -ENOMEM;
        goto cleanup;
    }
    
    /* ... do work ... */
    
cleanup:
    kfree(ptr);
    return ret;
}
```

## References

- Linux Kernel Documentation: filesystems/
- fs/libfs.c - Simple filesystem helpers
- fs/ext2/ - Reference implementation
- include/linux/fs.h - VFS interface definitions
- Documentation/filesystems/vfs.txt - VFS documentation

## Next Steps for VexFS

1. Study existing simple filesystems (ramfs, ext2)
2. Implement minimal superblock structure
3. Add block device I/O operations
4. Test with loop device mounting
5. Verify persistence across mount/unmount cycles