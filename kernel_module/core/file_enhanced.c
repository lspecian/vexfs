/*
 * VexFS - Enhanced File Operations with Disk Persistence
 * 
 * This file implements enhanced file operations that properly persist
 * block mappings to disk, ensuring data survives across unmount/remount.
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

#include "../include/vexfs_core.h"
#include "../include/vexfs_block.h"

/* Define FALLOC_FL_KEEP_SIZE if not available */
#ifndef FALLOC_FL_KEEP_SIZE
#define FALLOC_FL_KEEP_SIZE 0x01
#endif

/* Forward declarations */
int vexfs_fsync_enhanced(struct file *file, loff_t start, loff_t end, int datasync);
long vexfs_fallocate_enhanced(struct file *file, int mode, loff_t offset, loff_t len);
int vexfs_read_folio_enhanced(struct file *file, struct folio *folio);
int vexfs_writepage_enhanced(struct page *page, struct writeback_control *wbc);
int vexfs_writepages_enhanced(struct address_space *mapping, struct writeback_control *wbc);
int vexfs_write_begin_enhanced(struct file *file, struct address_space *mapping,
                               loff_t pos, unsigned len,
                               struct page **pagep, void **fsdata);
int vexfs_write_end_enhanced(struct file *file, struct address_space *mapping,
                             loff_t pos, unsigned len, unsigned copied,
                             struct page *page, void *fsdata);
sector_t vexfs_bmap_enhanced(struct address_space *mapping, sector_t block);
ssize_t vexfs_direct_IO_enhanced(struct kiocb *iocb, struct iov_iter *iter);

/*
 * Enhanced VexFS file operations with persistence
 */
const struct file_operations vexfs_file_ops_enhanced = {
    .llseek         = generic_file_llseek,
    .read_iter      = generic_file_read_iter,
    .write_iter     = generic_file_write_iter,
    .mmap           = generic_file_mmap,
    .open           = generic_file_open,
    .fsync          = vexfs_fsync_enhanced,
    .splice_write   = iter_file_splice_write,
    .fallocate      = vexfs_fallocate_enhanced,
};

/*
 * Enhanced address space operations
 */
const struct address_space_operations vexfs_aops_enhanced = {
    .writepage      = vexfs_writepage_enhanced,
    .read_folio     = vexfs_read_folio_enhanced,
    .writepages     = vexfs_writepages_enhanced,
    .write_begin    = vexfs_write_begin_enhanced,
    .write_end      = vexfs_write_end_enhanced,
    .bmap           = vexfs_bmap_enhanced,
    .direct_IO      = vexfs_direct_IO_enhanced,
};

/*
 * Get block mapping for a file offset with proper disk persistence
 */
static int vexfs_get_block_enhanced(struct inode *inode, sector_t block,
                                   struct buffer_head *bh, int create)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    sector_t phys_block = 0;
    int err = 0;
    bool newly_allocated = false;
    
    /* Sanity checks */
    if (!inode || !bh) {
        printk(KERN_ERR "VexFS: Invalid parameters to get_block_enhanced\n");
        return -EINVAL;
    }
    
    /* Check if block is within direct blocks range */
    if (block >= VEXFS_DIRECT_BLOCKS) {
        /* TODO: Implement indirect blocks for Phase 3 */
        printk(KERN_WARNING "VexFS: Indirect blocks not yet implemented\n");
        return -EFBIG;
    }
    
    /* Get existing block mapping */
    if (block < vi->i_block_count && vi->i_blocks[block] != 0) {
        phys_block = vi->i_blocks[block];
    }
    
    /* Allocate new block if needed */
    if (!phys_block && create) {
        __u32 new_block;
        
        /* Allocate a new data block */
        err = vexfs_alloc_block(inode->i_sb, &new_block);
        if (err) {
            printk(KERN_ERR "VexFS: Failed to allocate block: %d\n", err);
            return err;
        }
        
        /* Update in-memory inode info */
        vi->i_blocks[block] = new_block;
        if (block >= vi->i_block_count) {
            vi->i_block_count = block + 1;
        }
        inode->i_blocks = vi->i_block_count;
        
        /* Mark inode dirty to trigger writeback */
        mark_inode_dirty(inode);
        
        /* Immediately write inode to disk for persistence */
        err = vexfs_write_inode_to_disk(inode);
        if (err) {
            printk(KERN_ERR "VexFS: Failed to write inode to disk: %d\n", err);
            /* Free the allocated block on failure */
            vexfs_free_block(inode->i_sb, new_block);
            vi->i_blocks[block] = 0;
            return err;
        }
        
        phys_block = new_block;
        newly_allocated = true;
        
        printk(KERN_DEBUG "VexFS: Allocated block %u for file block %lu\n",
               new_block, (unsigned long)block);
    }
    
    /* Map the buffer head to the physical block */
    if (phys_block) {
        map_bh(bh, inode->i_sb, phys_block);
        
        if (newly_allocated) {
            /* Mark as new for proper initialization */
            set_buffer_new(bh);
            
            /* Clear the new block to avoid garbage data */
            lock_buffer(bh);
            memset(bh->b_data, 0, inode->i_sb->s_blocksize);
            set_buffer_uptodate(bh);
            mark_buffer_dirty(bh);
            unlock_buffer(bh);
        }
        
        set_buffer_mapped(bh);
    }
    
    return 0;
}

/*
 * Enhanced read folio operation
 */
int vexfs_read_folio_enhanced(struct file *file, struct folio *folio)
{
    return block_read_full_folio(folio, vexfs_get_block_enhanced);
}

/*
 * Enhanced write page operation with proper error handling
 */
int vexfs_writepage_enhanced(struct page *page, struct writeback_control *wbc)
{
    struct inode *inode = page->mapping->host;
    return __block_write_full_folio(inode, page_folio(page), 
                                    vexfs_get_block_enhanced, wbc);
}

/*
 * Enhanced write multiple pages
 */
int vexfs_writepages_enhanced(struct address_space *mapping,
                              struct writeback_control *wbc)
{
    return mpage_writepages(mapping, wbc, vexfs_get_block_enhanced);
}

/*
 * Enhanced write begin operation
 */
int vexfs_write_begin_enhanced(struct file *file, struct address_space *mapping,
                               loff_t pos, unsigned len,
                               struct page **pagep, void **fsdata)
{
    int ret;
    
    ret = block_write_begin(mapping, pos, len, pagep, vexfs_get_block_enhanced);
    if (unlikely(ret)) {
        loff_t isize = mapping->host->i_size;
        if (pos + len > isize)
            truncate_pagecache(mapping->host, isize);
    }
    
    return ret;
}

/*
 * Enhanced write end operation with immediate metadata sync
 */
int vexfs_write_end_enhanced(struct file *file, struct address_space *mapping,
                             loff_t pos, unsigned len, unsigned copied,
                             struct page *page, void *fsdata)
{
    struct inode *inode = mapping->host;
    int ret;
    
    /* Complete the write operation */
    ret = generic_write_end(file, mapping, pos, len, copied, page, fsdata);
    
    if (ret > 0) {
        /* Update timestamps */
        struct timespec64 now = current_time(inode);
        inode_set_mtime_to_ts(inode, now);
        inode_set_ctime_to_ts(inode, now);
        
        /* Update file size if needed */
        if (pos + ret > inode->i_size) {
            i_size_write(inode, pos + ret);
        }
        
        /* Mark inode dirty and sync to disk */
        mark_inode_dirty(inode);
        
        /* For enhanced persistence, write inode immediately */
        vexfs_write_inode_to_disk(inode);
    }
    
    return ret;
}

/*
 * Enhanced bmap operation
 */
sector_t vexfs_bmap_enhanced(struct address_space *mapping, sector_t block)
{
    return generic_block_bmap(mapping, block, vexfs_get_block_enhanced);
}

/*
 * Enhanced direct I/O operation
 */
ssize_t vexfs_direct_IO_enhanced(struct kiocb *iocb, struct iov_iter *iter)
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
                                  vexfs_get_block_enhanced, NULL, 0);
        if (ret > 0) {
            struct timespec64 now = current_time(inode);
            inode_set_mtime_to_ts(inode, now);
            inode_set_ctime_to_ts(inode, now);
            mark_inode_dirty(inode);
            /* Sync inode for persistence */
            vexfs_write_inode_to_disk(inode);
        }
    } else {
        /* Direct read */
        ret = __blockdev_direct_IO(iocb, inode, inode->i_sb->s_bdev, iter,
                                  vexfs_get_block_enhanced, NULL, 0);
    }
    
    return ret;
}

/*
 * Enhanced fsync operation - ensure data and metadata are on disk
 */
int vexfs_fsync_enhanced(struct file *file, loff_t start, loff_t end,
                        int datasync)
{
    struct inode *inode = file->f_mapping->host;
    struct super_block *sb = inode->i_sb;
    int ret;
    
    /* Sync file data first */
    ret = generic_file_fsync(file, start, end, datasync);
    if (ret)
        return ret;
    
    /* Write inode to disk */
    ret = vexfs_write_inode_to_disk(inode);
    if (ret)
        return ret;
    
    /* Sync the superblock to update free block counts */
    vexfs_write_super(sb);
    
    /* Sync block device to ensure everything is on disk */
    if (sb->s_bdev) {
        ret = sync_blockdev(sb->s_bdev);
    }
    
    return ret;
}

/*
 * Enhanced fallocate for proper pre-allocation with persistence
 */
long vexfs_fallocate_enhanced(struct file *file, int mode, loff_t offset,
                              loff_t len)
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
    for (block = start_block; block < end_block && block < VEXFS_DIRECT_BLOCKS; block++) {
        struct buffer_head bh_dummy;
        memset(&bh_dummy, 0, sizeof(bh_dummy));
        
        /* Use get_block with create=1 to allocate */
        ret = vexfs_get_block_enhanced(inode, block, &bh_dummy, 1);
        if (ret)
            break;
    }
    
    if (!ret && !(mode & FALLOC_FL_KEEP_SIZE) && new_size > inode->i_size) {
        i_size_write(inode, new_size);
        mark_inode_dirty(inode);
        vexfs_write_inode_to_disk(inode);
    }
    
    inode_unlock(inode);
    return ret;
}

/*
 * Enhanced truncate operation with proper block freeing
 */
void vexfs_truncate_enhanced(struct inode *inode, loff_t new_size)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    loff_t old_size = inode->i_size;
    sector_t old_blocks, new_blocks;
    int i;
    
    if (!(S_ISREG(inode->i_mode) || S_ISDIR(inode->i_mode) || 
          S_ISLNK(inode->i_mode)))
        return;
    
    old_blocks = (old_size + VEXFS_BLOCK_SIZE - 1) >> VEXFS_BLOCK_SIZE_BITS;
    new_blocks = (new_size + VEXFS_BLOCK_SIZE - 1) >> VEXFS_BLOCK_SIZE_BITS;
    
    if (new_blocks >= old_blocks)
        return; /* No truncation needed */
    
    /* Free blocks beyond new size */
    for (i = new_blocks; i < old_blocks && i < VEXFS_DIRECT_BLOCKS; i++) {
        if (vi->i_blocks[i]) {
            vexfs_free_block(inode->i_sb, vi->i_blocks[i]);
            vi->i_blocks[i] = 0;
        }
    }
    
    vi->i_block_count = new_blocks;
    inode->i_blocks = new_blocks;
    
    /* Update size and persist */
    i_size_write(inode, new_size);
    mark_inode_dirty(inode);
    vexfs_write_inode_to_disk(inode);
}