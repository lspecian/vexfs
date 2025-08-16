/*
 * VexFS v2.0 - Superblock Operations
 * 
 * This file implements superblock operations for VexFS,
 * following Linux kernel filesystem patterns.
 */

#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/buffer_head.h>
#include <linux/statfs.h>
#include <linux/time.h>
#include <linux/writeback.h>

#include "../include/vexfs_core.h"
#include "../include/vexfs_block.h"

/* Global variables */
struct kmem_cache *vexfs_inode_cachep;

/* Forward declaration */
static void vexfs_put_super(struct super_block *sb);

/* Superblock operations structure */
const struct super_operations vexfs_super_ops = {
    .alloc_inode    = vexfs_alloc_inode,
    .free_inode     = vexfs_free_inode,
    .write_inode    = vexfs_write_inode,
    .evict_inode    = vexfs_evict_inode,
    .statfs         = vexfs_statfs,
    .sync_fs        = vexfs_sync_fs,
    .put_super      = vexfs_put_super,
};

/**
 * vexfs_fill_super - Fill superblock structure
 * @sb: Superblock to fill
 * @data: Mount data
 * @silent: Silent flag
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    struct vexfs_sb_info *sbi;
    struct vexfs_super_block *disk_sb;
    struct buffer_head *bh;
    struct inode *root_inode;
    int ret = -EINVAL;
    
    /* Allocate superblock info */
    sbi = kzalloc(sizeof(struct vexfs_sb_info), GFP_KERNEL);
    if (!sbi) {
        return -ENOMEM;
    }
    
    sb->s_fs_info = sbi;
    sbi->sb = sb;
    spin_lock_init(&sbi->bitmap_lock);  /* DEADLOCK FIX: Use spinlock instead of mutex */
    
    /* Set basic superblock parameters */
    sb->s_blocksize = VEXFS_BLOCK_SIZE;
    sb->s_blocksize_bits = VEXFS_BLOCK_SIZE_BITS;
    sb->s_magic = VEXFS_MAGIC;
    sb->s_op = &vexfs_super_ops;
    sb->s_maxbytes = MAX_LFS_FILESIZE;
    
    /* Read superblock from disk */
    bh = sb_bread(sb, 0);
    if (!bh) {
        if (!silent) {
            printk(KERN_ERR "VexFS: Unable to read superblock\n");
        }
        goto failed_mount;
    }
    
    disk_sb = (struct vexfs_super_block *)bh->b_data;
    
    /* Verify magic number */
    if (le32_to_cpu(disk_sb->s_magic) != VEXFS_MAGIC) {
        if (!silent) {
            printk(KERN_ERR "VexFS: Invalid magic number: 0x%x\n",
                   le32_to_cpu(disk_sb->s_magic));
        }
        goto failed_mount;
    }
    
    /* Initialize superblock info from disk */
    sbi->block_count = le32_to_cpu(disk_sb->s_blocks_count);
    sbi->inode_count = le32_to_cpu(disk_sb->s_inodes_count);
    atomic_long_set(&sbi->free_blocks, le32_to_cpu(disk_sb->s_free_blocks));  /* DEADLOCK FIX: Use atomic */
    atomic_long_set(&sbi->free_inodes, le32_to_cpu(disk_sb->s_free_inodes));  /* DEADLOCK FIX: Use atomic */
    sbi->sb_bh = bh;
    
    /* Check if root inode exists on disk first */
    struct buffer_head *inode_bh;
    sector_t inode_block = VEXFS_INODE_TABLE_BLOCK + ((VEXFS_ROOT_INO - 1) / VEXFS_INODES_PER_BLOCK);
    size_t offset = ((VEXFS_ROOT_INO - 1) % VEXFS_INODES_PER_BLOCK) * sizeof(struct vexfs_inode);
    
    inode_bh = sb_bread(sb, inode_block);
    if (!inode_bh) {
        printk(KERN_ERR "VexFS: Failed to read inode block %llu\n", (unsigned long long)inode_block);
        ret = -EIO;
        goto failed_mount;
    }
    
    struct vexfs_inode *disk_inode = (struct vexfs_inode *)(inode_bh->b_data + offset);
    bool root_exists = (le16_to_cpu(disk_inode->i_mode) != 0);
    brelse(inode_bh);
    
    if (root_exists) {
        /* Root inode exists, read it normally */
        printk(KERN_INFO "VexFS: Loading existing root inode\n");
        root_inode = vexfs_iget(sb, VEXFS_ROOT_INO);
        if (IS_ERR(root_inode)) {
            printk(KERN_ERR "VexFS: Failed to load existing root inode\n");
            ret = PTR_ERR(root_inode);
            goto failed_mount;
        }
    } else {
        /* Root inode doesn't exist, create it */
        printk(KERN_INFO "VexFS: Creating new root inode\n");
        root_inode = new_inode(sb);
        if (!root_inode) {
            ret = -ENOMEM;
            goto failed_mount;
        }
        
        /* Initialize root inode */
        root_inode->i_ino = VEXFS_ROOT_INO;
        root_inode->i_mode = S_IFDIR | 0755;
        root_inode->i_uid = GLOBAL_ROOT_UID;
        root_inode->i_gid = GLOBAL_ROOT_GID;
        root_inode->i_size = VEXFS_BLOCK_SIZE;
        root_inode->i_blocks = 1;
        set_nlink(root_inode, 2); /* . and .. */
        
        struct timespec64 now = current_time(root_inode);
        inode_set_atime_to_ts(root_inode, now);
        inode_set_mtime_to_ts(root_inode, now);
        inode_set_ctime_to_ts(root_inode, now);
        
        /* Set up directory operations */
        root_inode->i_op = &vexfs_dir_inode_ops;
        /* Use our fixed directory operations that properly read from disk */
        extern const struct file_operations vexfs_dir_operations_fixed;
        root_inode->i_fop = &vexfs_dir_operations_fixed;
        
        /* Initialize VexFS-specific inode info */
        struct vexfs_inode_info *vi = VEXFS_I(root_inode);
        vi->i_block_count = 0; /* Will be set by vexfs_init_dir */
        vi->i_vector_count = 0;
        memset(vi->i_blocks, 0, sizeof(vi->i_blocks));
        /* DEADLOCK FIX: Removed mutex_init - VFS provides proper inode locking */
        
        /* Initialize directory structure with "." and ".." entries */
        ret = vexfs_init_dir(root_inode, root_inode);
        if (ret) {
            printk(KERN_ERR "VexFS: Failed to initialize root directory: %d\n", ret);
            iput(root_inode);
            goto failed_mount;
        }
        
        /* Mark root inode dirty - will be written later */
        mark_inode_dirty(root_inode);
        
        /* No error check needed for mark_inode_dirty */
        if (0) {
            printk(KERN_ERR "VexFS: Failed to write root inode to disk: %d\n", ret);
            iput(root_inode);
            goto failed_mount;
        }
        mark_inode_dirty(root_inode);
        
        /* Insert into inode cache */
        insert_inode_hash(root_inode);
    }
    
    /* Create root dentry */
    sb->s_root = d_make_root(root_inode);
    if (!sb->s_root) {
        ret = -ENOMEM;
        goto failed_mount;
    }
    
    printk(KERN_INFO "VexFS: Mounted filesystem with %lu blocks, %lu inodes\n",
           sbi->block_count, sbi->inode_count);
    
    return 0;
    
failed_mount:
    if (bh) {
        brelse(bh);
    }
    if (sbi) {
        /* DEADLOCK FIX: No mutex to destroy, spinlock cleanup is automatic */
        kfree(sbi);
        sb->s_fs_info = NULL;
    }
    return ret;
}

/**
 * vexfs_alloc_inode - Allocate a new inode
 * @sb: Superblock
 *
 * Returns: Allocated inode on success, NULL on failure
 */
struct inode *vexfs_alloc_inode(struct super_block *sb)
{
    struct vexfs_inode_info *vi;
    
    /* Check if inode cache is properly initialized */
    if (!vexfs_inode_cachep) {
        printk(KERN_ERR "VexFS: inode cache not initialized\n");
        return NULL;
    }
    
    vi = kmem_cache_alloc(vexfs_inode_cachep, GFP_KERNEL);
    if (!vi) {
        printk(KERN_ERR "VexFS: failed to allocate inode from cache\n");
        return NULL;
    }
    
    /* CRITICAL FIX: Initialize I/O list to prevent VFS deadlocks */
    inode_init_once(&vi->vfs_inode);
    
    /* DEADLOCK FIX: Removed mutex_init - VFS provides proper inode locking */
    return &vi->vfs_inode;
}

/**
 * vexfs_free_inode - Free an inode
 * @inode: Inode to free
 */
void vexfs_free_inode(struct inode *inode)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    
    /* DEADLOCK FIX: Removed mutex_destroy - VFS provides proper inode locking */
    kmem_cache_free(vexfs_inode_cachep, vi);
}

/**
 * vexfs_write_inode - Write inode to disk
 * @inode: Inode to write
 * @wbc: Writeback control
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc)
{
    /* Delegate to the consistent implementation */
    return vexfs_write_inode_to_disk(inode);
}

/**
 * vexfs_evict_inode - Evict an inode from memory
 * @inode: Inode to evict
 */
void vexfs_evict_inode(struct inode *inode)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    
    truncate_inode_pages_final(&inode->i_data);
    clear_inode(inode);
    
    /* DEADLOCK FIX: Removed mutex_destroy - VFS provides proper inode locking */
    
    if (inode->i_nlink == 0) {
        /* Free inode number */
        vexfs_free_inode_num(inode->i_sb, inode->i_ino);
    }
}

/**
 * vexfs_statfs - Get filesystem statistics
 * @dentry: Dentry
 * @buf: Statistics buffer
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
    struct super_block *sb = dentry->d_sb;
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    
    buf->f_type = VEXFS_MAGIC;
    buf->f_bsize = sb->s_blocksize;
    buf->f_blocks = sbi->block_count;
    buf->f_bfree = atomic_long_read(&sbi->free_blocks);   /* DEADLOCK FIX: Read atomic value */
    buf->f_bavail = atomic_long_read(&sbi->free_blocks);  /* DEADLOCK FIX: Read atomic value */
    buf->f_files = sbi->inode_count;
    buf->f_ffree = atomic_long_read(&sbi->free_inodes);   /* DEADLOCK FIX: Read atomic value */
    buf->f_namelen = 255;
    
    return 0;
}

/**
 * vexfs_put_super - Clean up superblock during unmount
 * @sb: Superblock to clean up
 *
 * This function is called during unmount to properly clean up
 * the superblock and prevent hangs/deadlocks.
 */
static void vexfs_put_super(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    
    if (!sbi) {
        printk(KERN_WARNING "VexFS: put_super called with NULL sb_info\n");
        return;
    }
    
    printk(KERN_INFO "VexFS: Starting put_super cleanup\n");
    
    /* Release buffer head if held */
    if (sbi->sb_bh) {
        mark_buffer_dirty(sbi->sb_bh);
        sync_dirty_buffer(sbi->sb_bh);
        brelse(sbi->sb_bh);
        sbi->sb_bh = NULL;
    }
    
    /* Free superblock info */
    kfree(sbi);
    sb->s_fs_info = NULL;
    /* DEADLOCK FIX: No mutex to destroy, spinlock cleanup is automatic */
    
    printk(KERN_INFO "VexFS: put_super cleanup completed\n");
}

/**
 * vexfs_sync_fs - Sync filesystem
 * @sb: Superblock
 * @wait: Wait flag
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_sync_fs(struct super_block *sb, int wait)
{
    struct vexfs_sb_info *sbi;
    
    /* Check if sb is NULL first */
    if (!sb) {
        printk(KERN_WARNING "VexFS: sync_fs called with NULL sb\n");
        return 0;
    }
    
    sbi = VEXFS_SB(sb);
    if (!sbi) {
        printk(KERN_WARNING "VexFS: sync_fs called with NULL sb_info\n");
        return 0;
    }
    
    if (sbi->sb_bh) {
        mark_buffer_dirty(sbi->sb_bh);
        /* Remove sync_dirty_buffer() to prevent hanging - let kernel handle async writes */
    }
    
    return 0;
}