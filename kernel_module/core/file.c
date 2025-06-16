/*
 * VexFS - Vector Extension Filesystem
 * File Operations
 *
 * This file implements file operations for VexFS,
 * including read, write, and memory mapping operations.
 */

#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/uaccess.h>
#include <linux/pagemap.h>
#include <linux/mpage.h>
#include <linux/writeback.h>
#include <linux/falloc.h>
#include <linux/mutex.h>
#include <linux/filelock.h>

#include "../include/vexfs_core.h"

/* Define FALLOC_FL_KEEP_SIZE if not available */
#ifndef FALLOC_FL_KEEP_SIZE
#define FALLOC_FL_KEEP_SIZE 0x01
#endif
#include "../include/vexfs_block.h"

/*
 * VexFS file operations
 */
const struct file_operations vexfs_file_ops = {
    .llseek         = generic_file_llseek,
    .read_iter      = generic_file_read_iter,
    .write_iter     = generic_file_write_iter,
    .mmap           = generic_file_mmap,
    .open           = generic_file_open,
    .fsync          = generic_file_fsync,
    .splice_write   = iter_file_splice_write,
};

/*
 * VexFS address space operations
 */
const struct address_space_operations vexfs_aops = {
    .writepage      = vexfs_writepage,
    .read_folio     = vexfs_read_folio,
    .writepages     = vexfs_writepages,
    .write_begin    = vexfs_write_begin,
    .write_end      = vexfs_write_end,
    .bmap           = vexfs_bmap,
    .direct_IO      = vexfs_direct_IO,
};

/*
 * VexFS address space operations are used for both files and directories
 * This ensures proper I/O list initialization and prevents kernel crashes
 * All inodes get proper address space operations with GFP_KERNEL mapping
 */

/*
 * Get the block number for a given file offset
 */
static sector_t vexfs_get_block_number(struct inode *inode, sector_t block)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    
    printk(KERN_INFO "vexfs_get_block_number: block=%lu, i_block_count=%u\n",
           (unsigned long)block, vi->i_block_count);
    
    if (block >= VEXFS_DIRECT_BLOCKS) {
        /* TODO: Implement indirect blocks */
        printk(KERN_INFO "vexfs_get_block_number: block >= VEXFS_DIRECT_BLOCKS, returning 0\n");
        return 0;
    }
    
    if (block >= vi->i_block_count) {
        printk(KERN_INFO "vexfs_get_block_number: block >= i_block_count, returning 0\n");
        return 0;
    }
    
    printk(KERN_INFO "vexfs_get_block_number: returning vi->i_blocks[%lu] = %u\n",
           (unsigned long)block, vi->i_blocks[block]);
    return vi->i_blocks[block];
}

/*
 * Allocate a new block for a file
 */
static int vexfs_alloc_file_block(struct inode *inode, sector_t block)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    __u32 new_block;
    
    printk(KERN_INFO "vexfs_alloc_file_block: called with block=%lu, i_block_count=%u\n",
           (unsigned long)block, vi->i_block_count);
    
    if (block >= VEXFS_DIRECT_BLOCKS) {
        /* TODO: Implement indirect blocks */
        printk(KERN_ERR "vexfs_alloc_file_block: block >= VEXFS_DIRECT_BLOCKS\n");
        return -EFBIG;
    }
    
    if (block >= vi->i_block_count) {
        /* Allocate new block */
        printk(KERN_INFO "vexfs_alloc_file_block: allocating new block\n");
        if (vexfs_alloc_block(inode->i_sb, &new_block) != 0) {
            printk(KERN_ERR "vexfs_alloc_file_block: vexfs_alloc_block failed\n");
            return -ENOSPC;
        }
        
        printk(KERN_INFO "vexfs_alloc_file_block: allocated block %u, storing in vi->i_blocks[%lu]\n",
               new_block, (unsigned long)block);
        vi->i_blocks[block] = new_block;
        vi->i_block_count = block + 1;
        inode->i_blocks++;
        
        printk(KERN_INFO "vexfs_alloc_file_block: updated i_block_count to %u\n", vi->i_block_count);
        
        /* DEADLOCK FIX: Defer inode write - let VFS handle timing */
        mark_inode_dirty(inode);
        
        return 0;
    }
    
    printk(KERN_INFO "vexfs_alloc_file_block: block already allocated\n");
    return 0; /* Block already allocated */
}

/*
 * Get block mapping for a file offset
 */
static int vexfs_get_block(struct inode *inode, sector_t block,
                          struct buffer_head *bh, int create)
{
    sector_t phys_block;
    int err = 0;
    bool newly_allocated = false;
    
    /* CRITICAL DEBUG: Add debug message at very start */
    printk(KERN_ALERT "vexfs_get_block: ENTRY - block=%lu, create=%d, inode=%p\n",
           (unsigned long)block, create, inode);
    
    /* CRITICAL DEBUG: Check if inode is valid */
    if (!inode) {
        printk(KERN_ALERT "vexfs_get_block: ERROR - inode is NULL!\n");
        return -EINVAL;
    }
    
    /* CRITICAL DEBUG: Check if buffer_head is valid */
    if (!bh) {
        printk(KERN_ALERT "vexfs_get_block: ERROR - buffer_head is NULL!\n");
        return -EINVAL;
    }
    
    printk(KERN_ALERT "vexfs_get_block: About to call vexfs_get_block_number\n");
    
    /* Note: VFS layer already holds inode lock, so we don't need to lock here */
    
    phys_block = vexfs_get_block_number(inode, block);
    printk(KERN_INFO "vexfs_get_block: first lookup returned phys_block=%lu\n",
           (unsigned long)phys_block);
    
    if (!phys_block && create) {
        printk(KERN_INFO "vexfs_get_block: allocating new block\n");
        err = vexfs_alloc_file_block(inode, block);
        if (err) {
            printk(KERN_ERR "vexfs_get_block: allocation failed with err=%d\n", err);
            return err;
        }
        phys_block = vexfs_get_block_number(inode, block);
        printk(KERN_INFO "vexfs_get_block: after allocation, phys_block=%lu\n",
               (unsigned long)phys_block);
        newly_allocated = true;
    }
    
    if (phys_block) {
        printk(KERN_INFO "vexfs_get_block: mapping buffer to phys_block=%lu\n",
               (unsigned long)phys_block);
        map_bh(bh, inode->i_sb, phys_block);
        
        /* Set BH_New flag for newly allocated blocks */
        if (newly_allocated) {
            printk(KERN_INFO "vexfs_get_block: setting BH_New flag for newly allocated block\n");
            set_buffer_new(bh);
        }
        
        /* Mark buffer as mapped and ensure it will be written to disk */
        set_buffer_mapped(bh);
        set_buffer_uptodate(bh);
        
        printk(KERN_INFO "vexfs_get_block: buffer head fully configured for block %lu\n",
               (unsigned long)phys_block);
    } else if (create) {
        /* CRITICAL FIX: If we were asked to create but couldn't allocate, return error */
        printk(KERN_ERR "vexfs_get_block: failed to allocate block despite create=1\n");
        return -ENOSPC;
    } else {
        /* Read operation for non-existent block - this is normal for sparse files */
        printk(KERN_INFO "vexfs_get_block: no physical block for read operation (sparse file)\n");
        /* For read operations, we don't map anything - kernel will handle as hole */
    }
    
    printk(KERN_INFO "vexfs_get_block: completed successfully, err=%d\n", err);
    return err;
}

/*
 * Read a single page
 */
/* Read folio operation for modern kernels */
int vexfs_read_folio(struct file *file, struct folio *folio)
{
    return block_read_full_folio(folio, vexfs_get_block);
}

/*
 * Write a single page
 */
int vexfs_writepage(struct page *page, struct writeback_control *wbc)
{
    struct inode *inode = page->mapping->host;
    return __block_write_full_folio(inode, page_folio(page), vexfs_get_block, wbc);
}

/*
 * Write multiple pages
 */
int vexfs_writepages(struct address_space *mapping,
                     struct writeback_control *wbc)
{
    return mpage_writepages(mapping, wbc, vexfs_get_block);
}

/*
 * Prepare for writing to a page
 */
int vexfs_write_begin(struct file *file, struct address_space *mapping,
                      loff_t pos, unsigned len,
                      struct page **pagep, void **fsdata)
{
    int ret;
    
    printk(KERN_ALERT "vexfs_write_begin: called with pos=%lld, len=%u\n", pos, len);
    
    ret = block_write_begin(mapping, pos, len, pagep, vexfs_get_block);
    if (unlikely(ret)) {
        printk(KERN_ALERT "vexfs_write_begin: block_write_begin failed with ret=%d\n", ret);
        loff_t isize = mapping->host->i_size;
        if (pos + len > isize)
            truncate_pagecache(mapping->host, isize);
    } else {
        printk(KERN_ALERT "vexfs_write_begin: block_write_begin succeeded\n");
    }
    
    return ret;
}

/*
 * Complete writing to a page
 */
int vexfs_write_end(struct file *file, struct address_space *mapping,
                    loff_t pos, unsigned len, unsigned copied,
                    struct page *page, void *fsdata)
{
    struct inode *inode = mapping->host;
    int ret;
    
    printk(KERN_ALERT "vexfs_write_end: called with pos=%lld, len=%u, copied=%u\n", pos, len, copied);
    
    /* First call generic_write_end to handle basic page operations */
    ret = generic_write_end(file, mapping, pos, len, copied, page, fsdata);
    
    printk(KERN_ALERT "vexfs_write_end: generic_write_end returned %d\n", ret);
    
    if (ret > 0) {
        /* Update inode timestamps */
        struct timespec64 now = current_time(inode);
        inode_set_mtime_to_ts(inode, now);
        inode_set_ctime_to_ts(inode, now);
        
        /* Mark inode dirty to ensure metadata is written */
        mark_inode_dirty(inode);
        
        /*
         * NOTE: Do NOT call filemap_write_and_wait_range() here as it causes
         * a deadlock in the write path. The VFS layer will handle flushing
         * dirty pages asynchronously through vexfs_writepage().
         */
        
        printk(KERN_INFO "vexfs_write_end: updated timestamps and marked inode dirty\n");
    }
    
    if (ret < len) {
        printk(KERN_ALERT "vexfs_write_end: partial write, truncating pagecache\n");
        loff_t isize = mapping->host->i_size;
        if (pos + len > isize)
            truncate_pagecache(mapping->host, isize);
    }
    
    printk(KERN_ALERT "vexfs_write_end: completed, returning %d\n", ret);
    return ret;
}

/*
 * Map logical block to physical block (for bmap system call)
 */
sector_t vexfs_bmap(struct address_space *mapping, sector_t block)
{
    return generic_block_bmap(mapping, block, vexfs_get_block);
}

/*
 * Direct I/O operations
 */
ssize_t vexfs_direct_IO(struct kiocb *iocb, struct iov_iter *iter)
{
    struct file *file = iocb->ki_filp;
    struct address_space *mapping = file->f_mapping;
    struct inode *inode = mapping->host;
    size_t count = iov_iter_count(iter);
    loff_t offset = iocb->ki_pos;
    ssize_t ret;
    
    /* Check alignment */
    if (offset & (VEXFS_BLOCK_SIZE - 1) || count & (VEXFS_BLOCK_SIZE - 1)) {
        return -EINVAL;
    }
    
    if (iov_iter_rw(iter) == WRITE) {
        /* Direct write */
        ret = __blockdev_direct_IO(iocb, inode, inode->i_sb->s_bdev, iter,
                                  vexfs_get_block, NULL, 0);
        if (ret > 0) {
            struct timespec64 now = current_time(inode);
            inode_set_mtime_to_ts(inode, now);
            inode_set_ctime_to_ts(inode, now);
            mark_inode_dirty(inode);
        }
    } else {
        /* Direct read */
        ret = __blockdev_direct_IO(iocb, inode, inode->i_sb->s_bdev, iter,
                                  vexfs_get_block, NULL, 0);
    }
    
    return ret;
}

/*
 * Truncate file to specified size
 */
void vexfs_truncate(struct inode *inode)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    loff_t old_size = inode->i_size;
    sector_t old_blocks, new_blocks;
    int i;
    
    if (!(S_ISREG(inode->i_mode) || S_ISDIR(inode->i_mode) || 
          S_ISLNK(inode->i_mode)))
        return;
    
    old_blocks = (old_size + VEXFS_BLOCK_SIZE - 1) >> VEXFS_BLOCK_SIZE_BITS;
    new_blocks = (inode->i_size + VEXFS_BLOCK_SIZE - 1) >> VEXFS_BLOCK_SIZE_BITS;
    
    if (new_blocks >= old_blocks)
        return; /* No truncation needed */
    
    /* DEADLOCK FIX: Removed mutex - VFS provides proper inode locking */
    
    /* Free blocks beyond new size */
    for (i = new_blocks; i < old_blocks && i < VEXFS_DIRECT_BLOCKS; i++) {
        if (vi->i_blocks[i]) {
            vexfs_free_block(inode->i_sb, vi->i_blocks[i]);
            vi->i_blocks[i] = 0;
            inode->i_blocks--;
        }
    }
    
    vi->i_block_count = new_blocks;
    
    /* TODO: Handle indirect blocks */
    
    /* Update timestamps */
    struct timespec64 now = current_time(inode);
    inode_set_mtime_to_ts(inode, now);
    inode_set_ctime_to_ts(inode, now);
    mark_inode_dirty(inode);
    
    /* DEADLOCK FIX: Defer inode write - let VFS handle timing */
    mark_inode_dirty(inode);
}

/*
 * Set file attributes
 */
int vexfs_setattr(struct dentry *dentry, struct iattr *attr)
{
    struct inode *inode = d_inode(dentry);
    int error;
    
    error = setattr_prepare(&nop_mnt_idmap, dentry, attr);
    if (error)
        return error;
    
    if (attr->ia_valid & ATTR_SIZE) {
        loff_t old_size = inode->i_size;
        
        if (attr->ia_size != old_size) {
            error = inode_newsize_ok(inode, attr->ia_size);
            if (error)
                return error;
            
            truncate_setsize(inode, attr->ia_size);
            vexfs_truncate(inode);
        }
    }
    
    setattr_copy(&nop_mnt_idmap, inode, attr);
    mark_inode_dirty(inode);
    
    return 0;
}

/*
 * Get file attributes
 */
int vexfs_getattr(const struct path *path, struct kstat *stat,
                 u32 request_mask, unsigned int flags)
{
    struct inode *inode = d_inode(path->dentry);
    
    generic_fillattr(&nop_mnt_idmap, STATX_BASIC_STATS, inode, stat);
    return 0;
}

/*
 * File permission check
 */
int vexfs_permission(struct inode *inode, int mask)
{
    return generic_permission(&nop_mnt_idmap, inode, mask);
}

/*
 * Extended attribute operations (for semantic metadata)
 */
ssize_t vexfs_listxattr(struct dentry *dentry, char *buffer, size_t size)
{
    /* TODO: Implement extended attribute listing */
    return -EOPNOTSUPP;
}

ssize_t vexfs_getxattr(struct dentry *dentry, struct inode *inode,
                      const char *name, void *buffer, size_t size)
{
    /* TODO: Implement extended attribute retrieval */
    return -EOPNOTSUPP;
}

int vexfs_setxattr(struct dentry *dentry, struct inode *inode,
                  const char *name, const void *value, size_t size, int flags)
{
    /* TODO: Implement extended attribute setting */
    return -EOPNOTSUPP;
}

int vexfs_removexattr(struct dentry *dentry, const char *name)
{
    /* TODO: Implement extended attribute removal */
    return -EOPNOTSUPP;
}

/*
 * Memory mapping support
 */
/*
 * File locking support
 */
int vexfs_lock(struct file *file, int cmd, struct file_lock *fl)
{
    return posix_lock_file(file, fl, NULL);
}

/*
 * File lease support
 */
int vexfs_lease(struct file *file, long arg)
{
    return generic_setlease(file, arg, NULL, NULL);
}

/*
 * Fallocate support for pre-allocation
 */
long vexfs_fallocate(struct file *file, int mode, loff_t offset, loff_t len)
{
    struct inode *inode = file_inode(file);
    loff_t new_size;
    sector_t start_block, end_block, block;
    int ret = 0;
    
    /* Only support simple allocation for now */
    if (mode & ~FALLOC_FL_KEEP_SIZE)
        return -EOPNOTSUPP;
    
    inode_lock(inode);
    
    new_size = offset + len;
    start_block = offset >> VEXFS_BLOCK_SIZE_BITS;
    end_block = (new_size + VEXFS_BLOCK_SIZE - 1) >> VEXFS_BLOCK_SIZE_BITS;
    
    /* Pre-allocate blocks */
    for (block = start_block; block < end_block; block++) {
        ret = vexfs_alloc_file_block(inode, block);
        if (ret)
            break;
    }
    
    if (!ret && !(mode & FALLOC_FL_KEEP_SIZE) && new_size > inode->i_size) {
        i_size_write(inode, new_size);
        mark_inode_dirty(inode);
    }
    
    inode_unlock(inode);
    return ret;
}