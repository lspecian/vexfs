/*
 * VexFS Safe Function Wrappers
 * 
 * Provides safer versions of critical functions with comprehensive
 * NULL pointer checks to prevent kernel panics.
 */

#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>

#include "../include/vexfs_core.h"
#include "null_safety.h"

/**
 * vexfs_get_block_safe - Safe wrapper for vexfs_get_block
 * 
 * Adds comprehensive NULL checks before processing block requests
 */
int vexfs_get_block_safe(struct inode *inode, sector_t iblock,
                         struct buffer_head *bh, int create)
{
    struct vexfs_inode_info *vi;
    struct vexfs_sb_info *sbi;
    unsigned long phys_block = 0;
    int err;
    
    /* Validate all pointers */
    err = vexfs_validate_inode(inode, __func__);
    if (err)
        return err;
    
    if (!bh) {
        printk(KERN_ERR "VexFS: %s: buffer_head is NULL\n", __func__);
        return -EINVAL;
    }
    
    /* Get VexFS-specific structures safely */
    vi = vexfs_inode_safe(inode);
    if (!vi) {
        printk(KERN_ERR "VexFS: %s: Failed to get VexFS inode info\n", __func__);
        return -EINVAL;
    }
    
    sbi = vexfs_sb_safe(inode->i_sb);
    if (!sbi) {
        printk(KERN_ERR "VexFS: %s: Failed to get VexFS sb info\n", __func__);
        return -EINVAL;
    }
    
    /* Bounds check */
    if (iblock >= VEXFS_MAX_FILE_BLOCKS) {
        printk(KERN_ERR "VexFS: %s: block %llu exceeds maximum\n",
               __func__, (unsigned long long)iblock);
        return -EFBIG;
    }
    
    /* Lock inode for thread safety */
    mutex_lock(&inode->i_mutex);
    
    /* Get physical block */
    if (iblock < VEXFS_DIRECT_BLOCKS && iblock < vi->i_block_count) {
        phys_block = vi->i_blocks[iblock];
    }
    
    /* Allocate if needed and allowed */
    if (!phys_block && create) {
        phys_block = vexfs_alloc_block(inode->i_sb);
        if (!phys_block) {
            mutex_unlock(&inode->i_mutex);
            return -ENOSPC;
        }
        
        /* Update inode */
        vi->i_blocks[iblock] = phys_block;
        if (iblock >= vi->i_block_count) {
            vi->i_block_count = iblock + 1;
        }
        mark_inode_dirty(inode);
    }
    
    mutex_unlock(&inode->i_mutex);
    
    /* Map buffer head if we have a block */
    if (phys_block) {
        map_bh(bh, inode->i_sb, phys_block);
        if (create) {
            set_buffer_new(bh);
        }
    }
    
    return 0;
}

/**
 * vexfs_iget_safe - Safe wrapper for vexfs_iget
 * 
 * Adds comprehensive NULL checks when reading inodes
 */
struct inode *vexfs_iget_safe(struct super_block *sb, unsigned long ino)
{
    struct inode *inode;
    struct vexfs_inode_info *vi;
    struct vexfs_sb_info *sbi;
    struct buffer_head *bh;
    struct vexfs_disk_inode *disk_inode;
    unsigned long block_num, offset;
    int err;
    
    /* Validate superblock */
    err = vexfs_validate_sb(sb, __func__);
    if (err)
        return ERR_PTR(err);
    
    sbi = vexfs_sb_safe(sb);
    if (!sbi)
        return ERR_PTR(-EINVAL);
    
    /* Validate inode number */
    if (ino < VEXFS_ROOT_INODE || ino > le32_to_cpu(sbi->sb->s_inodes_count)) {
        printk(KERN_ERR "VexFS: Invalid inode number %lu\n", ino);
        return ERR_PTR(-EINVAL);
    }
    
    /* Get inode from VFS */
    inode = iget_locked(sb, ino);
    if (!inode)
        return ERR_PTR(-ENOMEM);
    
    /* If already in cache, return it */
    if (!(inode->i_state & I_NEW))
        return inode;
    
    vi = vexfs_inode_safe(inode);
    if (!vi) {
        iget_failed(inode);
        return ERR_PTR(-EINVAL);
    }
    
    /* Calculate block and offset */
    block_num = VEXFS_INODE_TABLE_START + 
                ((ino - 1) / VEXFS_INODES_PER_BLOCK);
    offset = ((ino - 1) % VEXFS_INODES_PER_BLOCK) * sizeof(struct vexfs_disk_inode);
    
    /* Read inode from disk */
    bh = sb_bread(sb, block_num);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read inode block %lu for inode %lu\n",
               block_num, ino);
        iget_failed(inode);
        return ERR_PTR(-EIO);
    }
    
    /* Validate buffer */
    if (!bh->b_data || offset + sizeof(struct vexfs_disk_inode) > bh->b_size) {
        printk(KERN_ERR "VexFS: Invalid buffer for inode %lu\n", ino);
        brelse(bh);
        iget_failed(inode);
        return ERR_PTR(-EIO);
    }
    
    disk_inode = (struct vexfs_disk_inode *)((char *)bh->b_data + offset);
    
    /* Copy data from disk inode to VFS inode */
    inode->i_mode = le16_to_cpu(disk_inode->i_mode);
    inode->i_uid = make_kuid(&init_user_ns, le32_to_cpu(disk_inode->i_uid));
    inode->i_gid = make_kgid(&init_user_ns, le32_to_cpu(disk_inode->i_gid));
    inode->i_size = le64_to_cpu(disk_inode->i_size);
    set_nlink(inode, le16_to_cpu(disk_inode->i_links_count));
    inode->i_blocks = le32_to_cpu(disk_inode->i_blocks);
    
    /* Copy block pointers */
    vi->i_block_count = 0;
    for (int i = 0; i < VEXFS_DIRECT_BLOCKS; i++) {
        vi->i_blocks[i] = le32_to_cpu(disk_inode->i_block[i]);
        if (vi->i_blocks[i] != 0) {
            vi->i_block_count++;
        }
    }
    
    brelse(bh);
    
    /* Set up operations */
    if (S_ISREG(inode->i_mode)) {
        inode->i_op = &vexfs_file_inode_ops;
        inode->i_fop = &vexfs_file_ops;
        inode->i_mapping->a_ops = &vexfs_aops;
    } else if (S_ISDIR(inode->i_mode)) {
        inode->i_op = &vexfs_dir_inode_ops;
        inode->i_fop = &vexfs_dir_operations;
        inode->i_mapping->a_ops = &empty_aops;
    } else {
        init_special_inode(inode, inode->i_mode, 0);
    }
    
    unlock_new_inode(inode);
    return inode;
}

/**
 * vexfs_alloc_block_safe - Safe wrapper for block allocation
 * 
 * Adds NULL checks and validation for block allocation
 */
unsigned int vexfs_alloc_block_safe(struct super_block *sb)
{
    struct vexfs_sb_info *sbi;
    struct buffer_head *bitmap_bh;
    unsigned int bit, block;
    int err;
    
    /* Validate superblock */
    err = vexfs_validate_sb(sb, __func__);
    if (err)
        return 0;
    
    sbi = vexfs_sb_safe(sb);
    if (!sbi || !sbi->sb)
        return 0;
    
    /* Lock allocation */
    mutex_lock(&sbi->s_lock);
    
    /* Check free blocks */
    if (le32_to_cpu(sbi->sb->s_free_blocks_count) == 0) {
        mutex_unlock(&sbi->s_lock);
        return 0;
    }
    
    /* Read bitmap */
    bitmap_bh = sb_bread(sb, VEXFS_DATA_BITMAP_BLOCK);
    if (!bitmap_bh) {
        mutex_unlock(&sbi->s_lock);
        return 0;
    }
    
    /* Find free bit */
    bit = find_first_zero_bit((unsigned long *)bitmap_bh->b_data,
                               le32_to_cpu(sbi->sb->s_blocks_count));
    
    if (bit >= le32_to_cpu(sbi->sb->s_blocks_count)) {
        brelse(bitmap_bh);
        mutex_unlock(&sbi->s_lock);
        return 0;
    }
    
    /* Set bit and calculate block */
    set_bit(bit, (unsigned long *)bitmap_bh->b_data);
    block = le32_to_cpu(sbi->sb->s_first_data_block) + bit;
    
    /* Update superblock */
    le32_add_cpu(&sbi->sb->s_free_blocks_count, -1);
    
    /* Mark dirty and sync */
    mark_buffer_dirty(bitmap_bh);
    mark_buffer_dirty(sbi->sb_bh);
    
    if (sb->s_flags & SB_SYNCHRONOUS) {
        sync_dirty_buffer(bitmap_bh);
        sync_dirty_buffer(sbi->sb_bh);
    }
    
    brelse(bitmap_bh);
    mutex_unlock(&sbi->s_lock);
    
    return block;
}

/**
 * vexfs_free_block_safe - Safe wrapper for block deallocation
 * 
 * Adds NULL checks and validation for block deallocation
 */
void vexfs_free_block_safe(struct super_block *sb, unsigned int block)
{
    struct vexfs_sb_info *sbi;
    struct buffer_head *bitmap_bh;
    unsigned int bit;
    int err;
    
    /* Validate parameters */
    err = vexfs_validate_sb(sb, __func__);
    if (err)
        return;
    
    if (!block) {
        printk(KERN_WARNING "VexFS: Attempt to free block 0\n");
        return;
    }
    
    sbi = vexfs_sb_safe(sb);
    if (!sbi || !sbi->sb)
        return;
    
    /* Calculate bit position */
    if (block < le32_to_cpu(sbi->sb->s_first_data_block)) {
        printk(KERN_WARNING "VexFS: Attempt to free reserved block %u\n", block);
        return;
    }
    
    bit = block - le32_to_cpu(sbi->sb->s_first_data_block);
    if (bit >= le32_to_cpu(sbi->sb->s_blocks_count)) {
        printk(KERN_WARNING "VexFS: Attempt to free invalid block %u\n", block);
        return;
    }
    
    /* Lock allocation */
    mutex_lock(&sbi->s_lock);
    
    /* Read bitmap */
    bitmap_bh = sb_bread(sb, VEXFS_DATA_BITMAP_BLOCK);
    if (!bitmap_bh) {
        mutex_unlock(&sbi->s_lock);
        return;
    }
    
    /* Check if already free */
    if (!test_bit(bit, (unsigned long *)bitmap_bh->b_data)) {
        printk(KERN_WARNING "VexFS: Double free of block %u\n", block);
        brelse(bitmap_bh);
        mutex_unlock(&sbi->s_lock);
        return;
    }
    
    /* Clear bit */
    clear_bit(bit, (unsigned long *)bitmap_bh->b_data);
    
    /* Update superblock */
    le32_add_cpu(&sbi->sb->s_free_blocks_count, 1);
    
    /* Mark dirty and sync */
    mark_buffer_dirty(bitmap_bh);
    mark_buffer_dirty(sbi->sb_bh);
    
    if (sb->s_flags & SB_SYNCHRONOUS) {
        sync_dirty_buffer(bitmap_bh);
        sync_dirty_buffer(sbi->sb_bh);
    }
    
    brelse(bitmap_bh);
    mutex_unlock(&sbi->s_lock);
}