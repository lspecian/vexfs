# VexFS Disk Persistence Implementation Guide

## Overview

This guide provides concrete implementation steps for transforming VexFS from an in-memory kernel module into a fully persistent, VFS-compliant filesystem with vector database capabilities.

## Current State Analysis

### Existing VFS Infrastructure
VexFS v2.0 already implements:
- ✅ Basic VFS operations (super_operations, file_operations, inode_operations)
- ✅ IOCTL interface for vector operations
- ✅ Superblock structure with vector-specific fields
- ✅ In-memory file storage system
- ✅ Vector indexing (HNSW, LSH)

### Missing Components for Disk Persistence
- ❌ Block allocation and management
- ❌ Persistent inode table
- ❌ On-disk data structures
- ❌ Write-ahead logging for transactions
- ❌ Extent-based allocation for large vector files

## Implementation Strategy

### Phase 1: Core Disk Infrastructure

#### 1.1 Enhanced Superblock Structure

**File: `kernel_module/vexfs_superblock.h`**
```c
/* On-disk superblock layout */
struct vexfs_disk_superblock {
    /* Standard filesystem fields */
    __le32 s_magic;                    /* 0x56455846 "VEXF" */
    __le32 s_block_size;               /* Block size (4096) */
    __le64 s_blocks_count;             /* Total blocks */
    __le64 s_free_blocks;              /* Free blocks */
    __le32 s_inodes_count;             /* Total inodes */
    __le32 s_free_inodes;              /* Free inodes */
    __le32 s_first_data_block;         /* First data block */
    __le32 s_log_block_size;           /* Log2(block_size) - 10 */
    
    /* Metadata locations */
    __le64 s_inode_table_block;        /* Inode table start */
    __le64 s_block_bitmap_block;       /* Block bitmap location */
    __le64 s_inode_bitmap_block;       /* Inode bitmap location */
    
    /* VexFS vector-specific fields */
    __le16 s_vector_dimensions;        /* Default vector dimensions */
    __le16 s_distance_metric;          /* Default distance metric */
    __le32 s_vector_element_type;      /* Default element type */
    __le64 s_vector_index_root;        /* Vector index root block */
    __le64 s_hnsw_index_root;          /* HNSW index root block */
    __le64 s_lsh_index_root;           /* LSH index root block */
    
    /* Transaction log */
    __le64 s_log_start_block;          /* WAL start block */
    __le64 s_log_end_block;            /* WAL end block */
    __le64 s_log_sequence;             /* Current log sequence */
    
    /* Performance optimization */
    __le32 s_simd_capabilities;        /* SIMD instruction sets */
    __le32 s_preferred_numa_node;      /* Preferred NUMA node */
    __le32 s_vector_cache_size;        /* Vector cache size */
    
    /* Filesystem state */
    __le32 s_state;                    /* Filesystem state */
    __le32 s_errors;                   /* Error handling policy */
    __le32 s_mount_count;              /* Mount count */
    __le32 s_max_mount_count;          /* Maximum mount count */
    __le32 s_last_check;               /* Last check time */
    __le32 s_check_interval;           /* Check interval */
    
    /* Reserved space */
    __u8   s_reserved[3840];           /* Pad to 4096 bytes */
    __le32 s_checksum;                 /* Superblock checksum */
};

/* In-memory superblock info */
struct vexfs_sb_info {
    /* Disk superblock */
    struct vexfs_disk_superblock *s_disk_sb;
    struct buffer_head *s_sb_bh;
    
    /* Block allocation */
    struct buffer_head *s_block_bitmap_bh;
    struct buffer_head *s_inode_bitmap_bh;
    spinlock_t s_bitmap_lock;
    
    /* Inode management */
    struct buffer_head **s_inode_table_bh;
    unsigned long s_inodes_per_block;
    unsigned long s_inode_table_blocks;
    
    /* Vector-specific caches */
    struct kmem_cache *s_vector_cache;
    struct kmem_cache *s_index_cache;
    
    /* Performance counters */
    atomic64_t s_vector_ops;
    atomic64_t s_block_allocs;
    atomic64_t s_inode_allocs;
    
    /* Locking */
    struct rw_semaphore s_resize_lock;
    spinlock_t s_lock;
};
```

#### 1.2 Enhanced Inode Structure

**File: `kernel_module/vexfs_inode.h`**
```c
/* On-disk inode structure */
struct vexfs_disk_inode {
    /* Standard inode fields */
    __le16 i_mode;                     /* File mode */
    __le16 i_uid;                      /* User ID */
    __le16 i_gid;                      /* Group ID */
    __le16 i_links_count;              /* Hard links count */
    __le32 i_size;                     /* File size in bytes */
    __le32 i_atime;                    /* Access time */
    __le32 i_ctime;                    /* Creation time */
    __le32 i_mtime;                    /* Modification time */
    __le32 i_dtime;                    /* Deletion time */
    __le32 i_blocks;                   /* Block count */
    __le32 i_flags;                    /* File flags */
    
    /* VexFS vector-specific fields */
    __le32 i_vector_count;             /* Number of vectors */
    __le16 i_vector_dimensions;        /* Vector dimensions */
    __le16 i_vector_element_type;      /* Element type */
    __le64 i_vector_index_block;       /* Vector index block */
    __le32 i_vector_flags;             /* Vector-specific flags */
    
    /* Block pointers */
    __le32 i_block[15];                /* Direct and indirect blocks */
    
    /* Extended attributes */
    __le32 i_file_acl;                 /* File ACL block */
    __le32 i_dir_acl;                  /* Directory ACL block */
    __le32 i_faddr;                    /* Fragment address */
    
    /* Reserved space */
    __u8   i_reserved[32];             /* Reserved for future use */
};

/* In-memory inode info */
struct vexfs_inode_info {
    /* Vector-specific data */
    u32 i_vector_count;
    u16 i_vector_dimensions;
    u16 i_vector_element_type;
    u64 i_vector_index_block;
    u32 i_vector_flags;
    
    /* Block allocation */
    u32 i_block[15];
    struct rw_semaphore i_data_sem;
    
    /* Caching */
    struct list_head i_vector_cache;
    spinlock_t i_vector_lock;
    
    /* Standard VFS inode */
    struct inode vfs_inode;
};
```

#### 1.3 Block Allocation Implementation

**File: `kernel_module/vexfs_balloc.c`**
```c
#include <linux/buffer_head.h>
#include <linux/bitmap.h>
#include "vexfs_superblock.h"

/**
 * vexfs_alloc_block - Allocate a new block
 * @sb: superblock
 * 
 * Returns: block number on success, 0 on failure
 */
unsigned long vexfs_alloc_block(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = sb->s_fs_info;
    struct vexfs_disk_superblock *disk_sb = sbi->s_disk_sb;
    unsigned long block_nr;
    int ret;
    
    spin_lock(&sbi->s_bitmap_lock);
    
    /* Check if we have free blocks */
    if (le64_to_cpu(disk_sb->s_free_blocks) == 0) {
        spin_unlock(&sbi->s_bitmap_lock);
        return 0;
    }
    
    /* Find first free block in bitmap */
    block_nr = find_first_zero_bit(
        (unsigned long *)sbi->s_block_bitmap_bh->b_data,
        le64_to_cpu(disk_sb->s_blocks_count)
    );
    
    if (block_nr >= le64_to_cpu(disk_sb->s_blocks_count)) {
        spin_unlock(&sbi->s_bitmap_lock);
        return 0;
    }
    
    /* Mark block as allocated */
    set_bit(block_nr, (unsigned long *)sbi->s_block_bitmap_bh->b_data);
    mark_buffer_dirty(sbi->s_block_bitmap_bh);
    
    /* Update free block count */
    disk_sb->s_free_blocks = cpu_to_le64(le64_to_cpu(disk_sb->s_free_blocks) - 1);
    mark_buffer_dirty(sbi->s_sb_bh);
    
    /* Update performance counters */
    atomic64_inc(&sbi->s_block_allocs);
    
    spin_unlock(&sbi->s_bitmap_lock);
    
    return block_nr;
}

/**
 * vexfs_free_block - Free a block
 * @sb: superblock
 * @block: block number to free
 */
void vexfs_free_block(struct super_block *sb, unsigned long block)
{
    struct vexfs_sb_info *sbi = sb->s_fs_info;
    struct vexfs_disk_superblock *disk_sb = sbi->s_disk_sb;
    
    if (block == 0 || block >= le64_to_cpu(disk_sb->s_blocks_count)) {
        printk(KERN_ERR "VexFS: Invalid block number %lu\n", block);
        return;
    }
    
    spin_lock(&sbi->s_bitmap_lock);
    
    /* Clear block in bitmap */
    clear_bit(block, (unsigned long *)sbi->s_block_bitmap_bh->b_data);
    mark_buffer_dirty(sbi->s_block_bitmap_bh);
    
    /* Update free block count */
    disk_sb->s_free_blocks = cpu_to_le64(le64_to_cpu(disk_sb->s_free_blocks) + 1);
    mark_buffer_dirty(sbi->s_sb_bh);
    
    spin_unlock(&sbi->s_bitmap_lock);
}

/**
 * vexfs_alloc_vector_blocks - Allocate contiguous blocks for vectors
 * @sb: superblock
 * @count: number of blocks needed
 * @blocks: array to store allocated block numbers
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_alloc_vector_blocks(struct super_block *sb, unsigned long count, 
                              unsigned long *blocks)
{
    unsigned long i;
    
    for (i = 0; i < count; i++) {
        blocks[i] = vexfs_alloc_block(sb);
        if (blocks[i] == 0) {
            /* Free previously allocated blocks */
            while (i > 0) {
                vexfs_free_block(sb, blocks[--i]);
            }
            return -ENOSPC;
        }
    }
    
    return 0;
}
```

### Phase 2: Vector Storage Implementation

#### 2.1 Vector Block Format

**File: `kernel_module/vexfs_vector.h`**
```c
/* Vector block header */
struct vexfs_vector_block_header {
    __le32 vb_magic;                   /* Vector block magic */
    __le32 vb_vector_count;            /* Vectors in this block */
    __le16 vb_dimensions;              /* Vector dimensions */
    __le16 vb_element_type;            /* Element type */
    __le32 vb_block_size;              /* Block size */
    __le32 vb_data_offset;             /* Offset to vector data */
    __le32 vb_index_offset;            /* Offset to vector index */
    __le32 vb_checksum;                /* Block checksum */
    __le64 vb_next_block;              /* Next vector block */
    __le64 vb_prev_block;              /* Previous vector block */
    __u8   vb_reserved[32];            /* Reserved space */
};

/* Vector entry in block */
struct vexfs_vector_entry {
    __le64 ve_vector_id;               /* Unique vector ID */
    __le32 ve_offset;                  /* Offset within block */
    __le32 ve_size;                    /* Vector size in bytes */
    __le32 ve_flags;                   /* Vector flags */
    __le32 ve_checksum;                /* Vector checksum */
};

/* Vector metadata */
struct vexfs_vector_metadata {
    __le64 vm_vector_id;               /* Vector ID */
    __le64 vm_file_offset;             /* Offset in file */
    __le32 vm_dimensions;              /* Vector dimensions */
    __le16 vm_element_type;            /* Element type */
    __le16 vm_flags;                   /* Metadata flags */
    __le64 vm_timestamp;               /* Creation timestamp */
    __le32 vm_checksum;                /* Metadata checksum */
    __u8   vm_reserved[16];            /* Reserved space */
};
```

#### 2.2 Vector I/O Operations

**File: `kernel_module/vexfs_vector_io.c`**
```c
/**
 * vexfs_read_vector_block - Read vector block from disk
 * @inode: inode containing vectors
 * @block_nr: block number to read
 * @buffer: buffer to store vector data
 * @size: size of buffer
 * 
 * Returns: number of bytes read, negative error code on failure
 */
ssize_t vexfs_read_vector_block(struct inode *inode, unsigned long block_nr,
                                void *buffer, size_t size)
{
    struct super_block *sb = inode->i_sb;
    struct buffer_head *bh;
    struct vexfs_vector_block_header *header;
    size_t bytes_to_copy;
    
    /* Read block from disk */
    bh = sb_bread(sb, block_nr);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read vector block %lu\n", block_nr);
        return -EIO;
    }
    
    /* Validate block header */
    header = (struct vexfs_vector_block_header *)bh->b_data;
    if (le32_to_cpu(header->vb_magic) != VEXFS_VECTOR_BLOCK_MAGIC) {
        printk(KERN_ERR "VexFS: Invalid vector block magic\n");
        brelse(bh);
        return -EINVAL;
    }
    
    /* Calculate bytes to copy */
    bytes_to_copy = min(size, (size_t)(sb->s_blocksize - sizeof(*header)));
    
    /* Copy vector data */
    memcpy(buffer, bh->b_data + le32_to_cpu(header->vb_data_offset), bytes_to_copy);
    
    brelse(bh);
    return bytes_to_copy;
}

/**
 * vexfs_write_vector_block - Write vector block to disk
 * @inode: inode containing vectors
 * @block_nr: block number to write
 * @buffer: buffer containing vector data
 * @size: size of data to write
 * 
 * Returns: number of bytes written, negative error code on failure
 */
ssize_t vexfs_write_vector_block(struct inode *inode, unsigned long block_nr,
                                 const void *buffer, size_t size)
{
    struct super_block *sb = inode->i_sb;
    struct buffer_head *bh;
    struct vexfs_vector_block_header *header;
    size_t bytes_to_copy;
    u32 checksum;
    
    /* Read existing block or allocate new one */
    bh = sb_bread(sb, block_nr);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read vector block %lu\n", block_nr);
        return -EIO;
    }
    
    /* Initialize block header if new */
    header = (struct vexfs_vector_block_header *)bh->b_data;
    if (le32_to_cpu(header->vb_magic) != VEXFS_VECTOR_BLOCK_MAGIC) {
        memset(header, 0, sizeof(*header));
        header->vb_magic = cpu_to_le32(VEXFS_VECTOR_BLOCK_MAGIC);
        header->vb_block_size = cpu_to_le32(sb->s_blocksize);
        header->vb_data_offset = cpu_to_le32(sizeof(*header));
    }
    
    /* Calculate bytes to copy */
    bytes_to_copy = min(size, (size_t)(sb->s_blocksize - sizeof(*header)));
    
    /* Copy vector data */
    memcpy(bh->b_data + le32_to_cpu(header->vb_data_offset), buffer, bytes_to_copy);
    
    /* Update header */
    header->vb_vector_count = cpu_to_le32(le32_to_cpu(header->vb_vector_count) + 1);
    
    /* Calculate and store checksum */
    checksum = crc32(0, bh->b_data + sizeof(header->vb_checksum), 
                     sb->s_blocksize - sizeof(header->vb_checksum));
    header->vb_checksum = cpu_to_le32(checksum);
    
    /* Mark buffer dirty and sync */
    mark_buffer_dirty(bh);
    sync_dirty_buffer(bh);
    brelse(bh);
    
    return bytes_to_copy;
}
```

### Phase 3: Transaction Support

#### 3.1 Write-Ahead Logging

**File: `kernel_module/vexfs_journal.h`**
```c
/* Transaction types */
#define VEXFS_TXN_VECTOR_INSERT    1
#define VEXFS_TXN_VECTOR_DELETE    2
#define VEXFS_TXN_VECTOR_UPDATE    3
#define VEXFS_TXN_INDEX_UPDATE     4
#define VEXFS_TXN_METADATA_UPDATE  5

/* Transaction header */
struct vexfs_txn_header {
    __le32 th_magic;                   /* Transaction magic */
    __le32 th_type;                    /* Transaction type */
    __le64 th_sequence;                /* Sequence number */
    __le64 th_timestamp;               /* Transaction timestamp */
    __le32 th_data_size;               /* Transaction data size */
    __le32 th_vector_count;            /* Vectors affected */
    __le64 th_inode;                   /* Affected inode */
    __le32 th_checksum;                /* Transaction checksum */
    __u8   th_reserved[32];            /* Reserved space */
};

/* Transaction context */
struct vexfs_transaction {
    struct super_block *t_sb;          /* Superblock */
    u32 t_type;                        /* Transaction type */
    u64 t_sequence;                    /* Sequence number */
    struct list_head t_buffers;        /* Modified buffers */
    spinlock_t t_lock;                 /* Transaction lock */
    atomic_t t_refcount;               /* Reference count */
    int t_state;                       /* Transaction state */
};

/* Transaction states */
#define VEXFS_TXN_ACTIVE     1
#define VEXFS_TXN_COMMITTING 2
#define VEXFS_TXN_COMMITTED  3
#define VEXFS_TXN_ABORTED    4
```

#### 3.2 Transaction Implementation

**File: `kernel_module/vexfs_journal.c`**
```c
/**
 * vexfs_begin_transaction - Start a new transaction
 * @sb: superblock
 * @type: transaction type
 * 
 * Returns: transaction handle or NULL on failure
 */
struct vexfs_transaction *vexfs_begin_transaction(struct super_block *sb, int type)
{
    struct vexfs_sb_info *sbi = sb->s_fs_info;
    struct vexfs_transaction *txn;
    
    txn = kmalloc(sizeof(*txn), GFP_KERNEL);
    if (!txn)
        return NULL;
    
    /* Initialize transaction */
    txn->t_sb = sb;
    txn->t_type = type;
    txn->t_sequence = atomic64_inc_return(&sbi->s_txn_sequence);
    INIT_LIST_HEAD(&txn->t_buffers);
    spin_lock_init(&txn->t_lock);
    atomic_set(&txn->t_refcount, 1);
    txn->t_state = VEXFS_TXN_ACTIVE;
    
    return txn;
}

/**
 * vexfs_commit_transaction - Commit a transaction
 * @txn: transaction to commit
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_commit_transaction(struct vexfs_transaction *txn)
{
    struct super_block *sb = txn->t_sb;
    struct vexfs_txn_header header;
    struct buffer_head *log_bh;
    unsigned long log_block;
    int ret = 0;
    
    spin_lock(&txn->t_lock);
    
    if (txn->t_state != VEXFS_TXN_ACTIVE) {
        spin_unlock(&txn->t_lock);
        return -EINVAL;
    }
    
    txn->t_state = VEXFS_TXN_COMMITTING;
    spin_unlock(&txn->t_lock);
    
    /* Allocate log block */
    log_block = vexfs_alloc_block(sb);
    if (log_block == 0) {
        ret = -ENOSPC;
        goto abort;
    }
    
    /* Prepare transaction header */
    memset(&header, 0, sizeof(header));
    header.th_magic = cpu_to_le32(VEXFS_TXN_MAGIC);
    header.th_type = cpu_to_le32(txn->t_type);
    header.th_sequence = cpu_to_le64(txn->t_sequence);
    header.th_timestamp = cpu_to_le64(ktime_get_real_seconds());
    
    /* Write transaction to log */
    log_bh = sb_getblk(sb, log_block);
    if (!log_bh) {
        ret = -EIO;
        goto free_block;
    }
    
    memcpy(log_bh->b_data, &header, sizeof(header));
    mark_buffer_dirty(log_bh);
    sync_dirty_buffer(log_bh);
    brelse(log_bh);
    
    /* Sync all modified buffers */
    // Implementation for syncing transaction buffers
    
    txn->t_state = VEXFS_TXN_COMMITTED;
    return 0;
    
free_block:
    vexfs_free_block(sb, log_block);
abort:
    txn->t_state = VEXFS_TXN_ABORTED;
    return ret;
}
```

## Integration with Existing VexFS

### Modified File Operations

**File: `kernel_module/vexfs_main.c` (modifications)**
```c
/* Enhanced file write with transaction support */
static ssize_t vexfs_v2_file_write(struct file *file, const char __user *buf,
                                   size_t count, loff_t *ppos)
{
    struct inode *inode = file_inode(file);
    struct vexfs_transaction *txn;
    ssize_t ret;
    
    /* Begin transaction for write operation */
    txn = vexfs_begin_transaction(inode->i_sb, VEXFS_TXN_VECTOR_INSERT);
    if (!txn)
        return -ENOMEM;
    
    /* Perform the write operation */
    ret = vexfs_write_with_transaction(file, buf, count, ppos, txn);
    
    if (ret > 0) {
        /* Commit transaction on success */
        if (vexfs_commit_transaction(txn) < 0) {
            /* Transaction commit failed, but data was written */
            printk(KERN_WARNING "VexFS: Transaction commit failed\n");
        }
    } else {
        /* Abort transaction on failure */
        vexfs_abort_transaction(txn);
    }
    
    vexfs_put_transaction(txn);
    return ret;
}

/* Enhanced vector IOCTL with transaction support */
static long vexfs_vector_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
    struct inode *inode = file_inode(file);
    struct vexfs_transaction *txn = NULL;
    long ret;
    
    /* Determine if transaction is needed */
    switch (cmd) {
    case VEXFS_IOC_BATCH_INSERT:
    case VEXFS_IOC_VECTOR_DELETE:
    case VEXFS_IOC_VECTOR_UPDATE:
        txn = vexfs_begin_transaction(inode->i_sb, VEXFS_TXN_VECTOR_INSERT);
        if (!txn)
            return -ENOMEM;
        break;
    }
    
    /* Execute IOCTL operation */
    ret = vexfs_vector_ioctl_impl(file, cmd, arg, txn);
    
    /* Handle transaction completion */
    if (txn) {
        if (ret == 0) {
            if (vexfs_commit_transaction(txn) < 0) {
                printk(KERN_WARNING "VexFS: IOCTL transaction commit failed\n");
                ret = -EIO;
            }
        } else {
            vexfs_abort_transaction(txn);
        }
        vexfs_put_transaction(txn);
    }
    
    return ret;
}
```

## Testing Strategy

### Unit Tests
1. **Block allocation/deallocation**
2. **Vector block I/O operations**
3. **Transaction commit/abort**
4. **Superblock persistence**

### Integration Tests
1. **Mount/unmount with disk persistence**
2. **Vector operations with transactions**
3. **Crash recovery testing**
4. **Performance benchmarking**

### Test Implementation
```c
/* Test block allocation */
static int test_block_allocation(struct super_block *sb)
{
    unsigned long blocks[10];
    int i, ret;
    
    /* Allocate blocks */
    ret = vexfs_alloc_vector_blocks(sb, 10, blocks);
    if (ret < 0) {
        printk(KERN_ERR "Block allocation failed: %d\n", ret);
        return ret;
    }
    
    /* Verify blocks are different */
    for (i = 0; i < 10; i++) {
        if (blocks[i] == 0) {
            printk(KERN_ERR "Invalid block number at index %d\n", i);
            return -EINVAL;
        }
    }
    
    /* Free blocks */
    for (i = 0; i < 10; i++) {
        vexfs_free_block(sb, blocks[i]);
    }
    
    return 0;
}
```

## Performance Considerations

### Optimization Strategies
1. **Block caching for frequently accessed vectors**
2. **Batch allocation for large vector insertions**
3. **NUMA-aware memory allocation**
4. **SIMD-optimized vector operations**

### Memory Management
```c
/* Vector cache management */
struct vexfs_vector_cache {
    struct kmem_cache *vc_cache;
    struct list_head vc_lru;
    spinlock_t vc_lock;
    atomic_t vc_size;
    u32 vc_max_size;
};

/* Initialize vector cache */
int vexfs_init_vector_cache(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = sb->s_fs_info;
    
    sbi->s_vector_cache = kmem_cache_create("vexfs_vector",
                                           sizeof(struct vexfs_vector_entry),
                                           0, SLAB_HWCACHE_ALIGN, NULL);
    if (!sbi->s_vector_cache)
        return -ENOMEM;
    
    return 0;
}
```

## Migration Path

### Phase 1: Basic Persistence (Week 1-2)
- Implement block allocation
- Add persistent superblock
- Convert file storage to block-based

### Phase 2: Vector Storage (Week 3-4)
- Implement vector block format
- Add vector I/O operations
- Integrate with existing IOCTL interface

### Phase 3: Transactions (Week 5-6)
- Implement write-ahead logging
- Add transaction support to operations
- Test crash recovery

### Phase 4: Optimization (Week 7-8)
- Add performance optimizations
- Implement caching strategies
- Benchmark and tune performance

## Conclusion

This implementation guide provides a concrete roadmap for transforming VexFS into a fully persistent filesystem while maintaining its vector database capabilities. The phased approach ensures incremental progress with testable milestones at each stage.

The key architectural decisions include:
- **Block-based storage** with bitmap allocation
- **Transaction safety** through write-ahead logging
- **Vector-optimized** block formats
- **Performance optimization** through caching and SIMD

The implementation leverages existing VexFS infrastructure while adding the necessary persistence layer for production use.