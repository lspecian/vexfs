/*
 * VexFS v2.0 - Block Allocation and Management
 * 
 * This file implements block allocation, bitmap management, and
 * disk I/O operations following Linux filesystem patterns.
 */

#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/bitmap.h>

#include "../include/vexfs_core.h"
#include "../include/vexfs_block.h"

/**
 * vexfs_alloc_block - Allocate a new block
 * @sb: Superblock
 * @block: Pointer to store allocated block number
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_alloc_block(struct super_block *sb, __u32 *block)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct buffer_head *bitmap_bh;
    unsigned long bit;
    int ret = 0;
    __u32 first_data_block = VEXFS_INODE_TABLE_BLOCK + VEXFS_INODE_TABLE_BLOCKS;
    unsigned long flags;
    
    /* DEADLOCK FIX: Check free blocks atomically without mutex */
    if (atomic_long_read(&sbi->free_blocks) == 0) {
        return -ENOSPC;
    }
    
    /* Read block bitmap */
    bitmap_bh = sb_bread(sb, 1); /* Bitmap is at block 1 */
    if (!bitmap_bh) {
        return -EIO;
    }
    
    /* DEADLOCK FIX: Use spinlock for bitmap operations only */
    spin_lock_irqsave(&sbi->bitmap_lock, flags);
    
    /* Find first free bit starting from first data block */
    bit = find_next_zero_bit((unsigned long *)bitmap_bh->b_data, sbi->block_count, first_data_block);
    if (bit >= sbi->block_count) {
        printk(KERN_ERR "VexFS: No free data blocks available (first_data_block=%u, total=%u)\n",
               first_data_block, sbi->block_count);
        ret = -ENOSPC;
        goto out_unlock;
    }
    
    /* Set bit in bitmap */
    set_bit(bit, (unsigned long *)bitmap_bh->b_data);
    mark_buffer_dirty(bitmap_bh);
    
    /* Update free block count atomically */
    atomic_long_dec(&sbi->free_blocks);
    
    *block = bit;
    
    printk(KERN_DEBUG "VexFS: Allocated data block %u (first_data_block=%u)\n", bit, first_data_block);
    
out_unlock:
    spin_unlock_irqrestore(&sbi->bitmap_lock, flags);
    brelse(bitmap_bh);
    return ret;
}

/**
 * vexfs_free_block - Free a block
 * @sb: Superblock
 * @block: Block number to free
 */
void vexfs_free_block(struct super_block *sb, __u32 block)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct buffer_head *bitmap_bh;
    unsigned long flags;
    
    if (block >= sbi->block_count) {
        printk(KERN_ERR "VexFS: Trying to free invalid block %u\n", block);
        return;
    }
    
    /* Read block bitmap */
    bitmap_bh = sb_bread(sb, 1);
    if (!bitmap_bh) {
        printk(KERN_ERR "VexFS: Unable to read block bitmap\n");
        return;
    }
    
    /* DEADLOCK FIX: Use spinlock for bitmap operations only */
    spin_lock_irqsave(&sbi->bitmap_lock, flags);
    
    /* Clear bit in bitmap */
    if (!test_bit(block, (unsigned long *)bitmap_bh->b_data)) {
        printk(KERN_ERR "VexFS: Trying to free already free block %u\n", block);
        goto out_unlock;
    }
    
    clear_bit(block, (unsigned long *)bitmap_bh->b_data);
    mark_buffer_dirty(bitmap_bh);
    
    /* Update free block count atomically */
    atomic_long_inc(&sbi->free_blocks);
    
out_unlock:
    spin_unlock_irqrestore(&sbi->bitmap_lock, flags);
    brelse(bitmap_bh);
}

/**
 * vexfs_alloc_inode_num - Allocate a new inode number
 * @sb: Superblock
 * @ino: Pointer to store allocated inode number
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_alloc_inode_num(struct super_block *sb, __u32 *ino)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct buffer_head *bh;
    struct vexfs_inode *disk_inode;
    int i, block, offset;
    int ret = -ENOSPC;
    
    /* Check free inodes atomically without mutex */
    if (atomic_long_read(&sbi->free_inodes) == 0) {
        return -ENOSPC;
    }
    
    /* Scan inode table to find a free inode (skip root inode at position 0) */
    for (i = 1; i < sbi->inode_count; i++) {
        /* Calculate block and offset for this inode */
        /* Use same calculation as superblock.c for consistency */
        block = VEXFS_INODE_TABLE_BLOCK + ((i - 1) / VEXFS_INODES_PER_BLOCK);
        offset = ((i - 1) % VEXFS_INODES_PER_BLOCK) * sizeof(struct vexfs_inode);
        
        /* Read the block containing this inode */
        bh = sb_bread(sb, block);
        if (!bh) {
            ret = -EIO;
            break;
        }
        
        /* Check if this inode is free (mode == 0 means free) */
        disk_inode = (struct vexfs_inode *)(bh->b_data + offset);
        if (le16_to_cpu(disk_inode->i_mode) == 0) {
            /* Found a free inode */
            *ino = i; /* inode numbers start at 1, but i already starts at 1 */
            atomic_long_dec(&sbi->free_inodes);
            ret = 0;
            brelse(bh);
            break;
        }
        
        brelse(bh);
    }
    
    return ret;
}

/**
 * vexfs_free_inode_num - Free an inode number
 * @sb: Superblock
 * @ino: Inode number to free
 */
void vexfs_free_inode_num(struct super_block *sb, __u32 ino)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    
    if (ino >= sbi->inode_count || ino == VEXFS_ROOT_INO) {
        printk(KERN_ERR "VexFS: Trying to free invalid inode %u\n", ino);
        return;
    }
    
    /* Update free inode count atomically */
    atomic_long_inc(&sbi->free_inodes);
    
    /* Note: The actual inode clearing happens when the inode is evicted */
    printk(KERN_DEBUG "VexFS: Freed inode %u\n", ino);
}

/**
 * vexfs_bread - Read a block from disk
 * @sb: Superblock
 * @block: Block number to read
 *
 * Returns: Buffer head on success, NULL on failure
 */
struct buffer_head *vexfs_bread(struct super_block *sb, __u32 block)
{
    return sb_bread(sb, block);
}

/**
 * vexfs_bwrite - Write a block to disk
 * @sb: Superblock
 * @block: Block number to write
 * @data: Data to write
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_bwrite(struct super_block *sb, __u32 block, void *data)
{
    struct buffer_head *bh;
    
    bh = sb_getblk(sb, block);
    if (!bh) {
        return -EIO;
    }
    
    lock_buffer(bh);
    memcpy(bh->b_data, data, sb->s_blocksize);
    set_buffer_uptodate(bh);
    mark_buffer_dirty(bh);
    unlock_buffer(bh);
    
    brelse(bh);
    return 0;
}

/**
 * vexfs_read_super - Read superblock from disk
 * @sb: Superblock
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_read_super(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct buffer_head *bh;
    struct vexfs_super_block *disk_sb;
    
    bh = sb_bread(sb, 0);
    if (!bh) {
        return -EIO;
    }
    
    disk_sb = (struct vexfs_super_block *)bh->b_data;
    
    /* Update in-memory superblock info */
    sbi->block_count = le32_to_cpu(disk_sb->s_blocks_count);
    sbi->inode_count = le32_to_cpu(disk_sb->s_inodes_count);
    atomic_long_set(&sbi->free_blocks, le32_to_cpu(disk_sb->s_free_blocks));  /* DEADLOCK FIX: Use atomic */
    atomic_long_set(&sbi->free_inodes, le32_to_cpu(disk_sb->s_free_inodes));  /* DEADLOCK FIX: Use atomic */
    
    if (sbi->sb_bh) {
        brelse(sbi->sb_bh);
    }
    sbi->sb_bh = bh;
    
    return 0;
}

/**
 * vexfs_write_super - Write superblock to disk
 * @sb: Superblock
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_write_super(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_super_block *disk_sb;
    
    if (!sbi->sb_bh) {
        return -EIO;
    }
    
    disk_sb = (struct vexfs_super_block *)sbi->sb_bh->b_data;
    
    /* Update disk superblock */
    disk_sb->s_free_blocks = cpu_to_le32(atomic_long_read(&sbi->free_blocks));   /* DEADLOCK FIX: Read atomic value */
    disk_sb->s_free_inodes = cpu_to_le32(atomic_long_read(&sbi->free_inodes));   /* DEADLOCK FIX: Read atomic value */
    disk_sb->s_wtime = cpu_to_le32(ktime_get_real_seconds());
    
    mark_buffer_dirty(sbi->sb_bh);
    /* Remove sync_dirty_buffer() to prevent hanging - let kernel handle async writes */
    
    return 0;
}