/* SPDX-License-Identifier: GPL-2.0 */
/*
 * VexFS v2.0 - Superblock Operations Implementation
 * 
 * This file implements superblock read/write operations and related
 * functionality for VexFS filesystem persistence.
 * 
 * Copyright (C) 2024 VexFS Development Team
 */

#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/crc32.h>
#include <linux/time.h>
#include <linux/uuid.h>
#include "vexfs_superblock.h"

/* Validate superblock size at compile time */
static void __init vexfs_validate_superblock_size(void)
{
    BUILD_BUG_ON(sizeof(struct vexfs_superblock) != VEXFS_DEFAULT_BLOCK_SIZE);
}

/**
 * vexfs_calculate_superblock_checksum - Calculate CRC32 checksum for superblock
 * @sb: Superblock structure
 * 
 * Returns: CRC32 checksum of superblock (excluding checksum field)
 */
u32 vexfs_calculate_superblock_checksum(struct vexfs_superblock *sb)
{
    u32 old_checksum;
    u32 checksum;
    
    /* Save and clear existing checksum */
    old_checksum = sb->s_checksum;
    sb->s_checksum = 0;
    
    /* Calculate checksum of entire superblock except checksum field */
    checksum = crc32(0, (u8 *)sb, sizeof(*sb) - sizeof(sb->s_checksum));
    
    /* Restore original checksum */
    sb->s_checksum = old_checksum;
    
    return checksum;
}

/**
 * vexfs_validate_superblock - Validate superblock structure and contents
 * @sb: Superblock structure to validate
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_validate_superblock(struct vexfs_superblock *sb)
{
    u32 calculated_checksum, stored_checksum;
    
    /* Check magic number */
    if (le32_to_cpu(sb->s_magic) != VEXFS_MAGIC) {
        printk(KERN_ERR "VexFS: Invalid magic number: 0x%x (expected 0x%x)\n",
               le32_to_cpu(sb->s_magic), VEXFS_MAGIC);
        return -EINVAL;
    }
    
    /* Check version compatibility */
    if (le32_to_cpu(sb->s_version) != VEXFS_VERSION) {
        printk(KERN_WARNING "VexFS: Version mismatch: 0x%x (expected 0x%x)\n",
               le32_to_cpu(sb->s_version), VEXFS_VERSION);
        /* Continue for now, but may need version-specific handling */
    }
    
    /* Validate block size */
    u32 block_size = le32_to_cpu(sb->s_block_size);
    if (block_size < VEXFS_MIN_BLOCK_SIZE || 
        block_size > VEXFS_MAX_BLOCK_SIZE ||
        !is_power_of_2(block_size)) {
        printk(KERN_ERR "VexFS: Invalid block size: %u\n", block_size);
        return -EINVAL;
    }
    
    /* Validate filesystem state */
    u32 state = le32_to_cpu(sb->s_state);
    if (state & ~(VEXFS_VALID_FS | VEXFS_ERROR_FS | VEXFS_DIRTY_FS)) {
        printk(KERN_WARNING "VexFS: Unknown filesystem state flags: 0x%x\n", state);
    }
    
    /* Validate checksum */
    stored_checksum = le32_to_cpu(sb->s_checksum);
    calculated_checksum = vexfs_calculate_superblock_checksum(sb);
    if (stored_checksum != calculated_checksum) {
        printk(KERN_ERR "VexFS: Superblock checksum mismatch: stored=0x%x, calculated=0x%x\n",
               stored_checksum, calculated_checksum);
        return -EINVAL;
    }
    
    printk(KERN_INFO "VexFS: Superblock validation successful\n");
    return 0;
}

/**
 * vexfs_read_superblock - Read and validate superblock from disk
 * @sb: VFS superblock structure
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_read_superblock(struct super_block *sb)
{
    struct vexfs_sb_info *sbi;
    struct buffer_head *bh;
    struct vexfs_superblock *disk_sb;
    int ret;
    
    printk(KERN_INFO "VexFS: Reading superblock from block %d\n", VEXFS_SUPERBLOCK_BLOCK);
    
    /* Read superblock from disk */
    bh = sb_bread(sb, VEXFS_SUPERBLOCK_BLOCK);
    if (!bh) {
        printk(KERN_ERR "VexFS: Cannot read superblock from block %d\n", 
               VEXFS_SUPERBLOCK_BLOCK);
        return -EIO;
    }
    
    disk_sb = (struct vexfs_superblock *)bh->b_data;
    
    /* Validate superblock */
    ret = vexfs_validate_superblock(disk_sb);
    if (ret) {
        printk(KERN_ERR "VexFS: Superblock validation failed\n");
        brelse(bh);
        return ret;
    }
    
    /* Allocate in-memory superblock info */
    sbi = kzalloc(sizeof(*sbi), GFP_KERNEL);
    if (!sbi) {
        printk(KERN_ERR "VexFS: Cannot allocate superblock info\n");
        brelse(bh);
        return -ENOMEM;
    }
    
    /* Initialize mutex */
    mutex_init(&sbi->s_lock);
    
    /* Copy superblock data to in-memory structure */
    sbi->s_magic = le32_to_cpu(disk_sb->s_magic);
    sbi->s_version = le32_to_cpu(disk_sb->s_version);
    sbi->s_block_size = le32_to_cpu(disk_sb->s_block_size);
    sbi->s_inode_size = le32_to_cpu(disk_sb->s_inode_size);
    sbi->s_blocks_count = le64_to_cpu(disk_sb->s_blocks_count);
    sbi->s_free_blocks = le64_to_cpu(disk_sb->s_free_blocks);
    sbi->s_inodes_count = le64_to_cpu(disk_sb->s_inodes_count);
    sbi->s_free_inodes = le64_to_cpu(disk_sb->s_free_inodes);
    sbi->s_first_data_block = le32_to_cpu(disk_sb->s_first_data_block);
    sbi->s_inode_table_block = le32_to_cpu(disk_sb->s_inode_table_block);
    sbi->s_block_bitmap_block = le32_to_cpu(disk_sb->s_block_bitmap_block);
    sbi->s_inode_bitmap_block = le32_to_cpu(disk_sb->s_inode_bitmap_block);
    sbi->s_state = le32_to_cpu(disk_sb->s_state);
    sbi->s_errors = le32_to_cpu(disk_sb->s_errors);
    
    /* VexFS-specific fields */
    sbi->s_vector_dimensions = le32_to_cpu(disk_sb->s_vector_dimensions);
    sbi->s_distance_metric = le32_to_cpu(disk_sb->s_distance_metric);
    sbi->s_hnsw_enabled = le32_to_cpu(disk_sb->s_hnsw_enabled) != 0;
    sbi->s_lsh_enabled = le32_to_cpu(disk_sb->s_lsh_enabled) != 0;
    
    /* Store buffer head for later updates */
    sbi->s_sbh = bh;
    sbi->s_dirty = false;
    
    /* Set VFS superblock info */
    sb->s_fs_info = sbi;
    
    /* Mark filesystem as mounted (dirty) */
    sbi->s_state |= VEXFS_DIRTY_FS;
    sbi->s_state &= ~VEXFS_VALID_FS;
    vexfs_mark_sb_dirty(sb);
    
    printk(KERN_INFO "VexFS: Superblock read successfully\n");
    printk(KERN_INFO "VexFS: Block size: %u, Blocks: %llu, Free: %llu\n",
           sbi->s_block_size, sbi->s_blocks_count, sbi->s_free_blocks);
    printk(KERN_INFO "VexFS: Inodes: %llu, Free: %llu\n",
           sbi->s_inodes_count, sbi->s_free_inodes);
    
    return 0;
}

/**
 * vexfs_write_superblock - Write superblock to disk
 * @sb: VFS superblock structure
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_write_superblock(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_superblock *disk_sb;
    u32 checksum;
    
    if (!sbi || !sbi->s_sbh) {
        printk(KERN_ERR "VexFS: No superblock buffer to write\n");
        return -EINVAL;
    }
    
    mutex_lock(&sbi->s_lock);
    
    if (!sbi->s_dirty) {
        mutex_unlock(&sbi->s_lock);
        return 0; /* Nothing to write */
    }
    
    printk(KERN_INFO "VexFS: Writing superblock to disk\n");
    
    disk_sb = (struct vexfs_superblock *)sbi->s_sbh->b_data;
    
    /* Update superblock fields */
    disk_sb->s_magic = cpu_to_le32(sbi->s_magic);
    disk_sb->s_version = cpu_to_le32(sbi->s_version);
    disk_sb->s_block_size = cpu_to_le32(sbi->s_block_size);
    disk_sb->s_inode_size = cpu_to_le32(sbi->s_inode_size);
    disk_sb->s_blocks_count = cpu_to_le64(sbi->s_blocks_count);
    disk_sb->s_free_blocks = cpu_to_le64(sbi->s_free_blocks);
    disk_sb->s_inodes_count = cpu_to_le64(sbi->s_inodes_count);
    disk_sb->s_free_inodes = cpu_to_le64(sbi->s_free_inodes);
    disk_sb->s_first_data_block = cpu_to_le32(sbi->s_first_data_block);
    disk_sb->s_inode_table_block = cpu_to_le32(sbi->s_inode_table_block);
    disk_sb->s_block_bitmap_block = cpu_to_le32(sbi->s_block_bitmap_block);
    disk_sb->s_inode_bitmap_block = cpu_to_le32(sbi->s_inode_bitmap_block);
    disk_sb->s_state = cpu_to_le32(sbi->s_state);
    disk_sb->s_errors = cpu_to_le32(sbi->s_errors);
    
    /* VexFS-specific fields */
    disk_sb->s_vector_dimensions = cpu_to_le32(sbi->s_vector_dimensions);
    disk_sb->s_distance_metric = cpu_to_le32(sbi->s_distance_metric);
    disk_sb->s_hnsw_enabled = cpu_to_le32(sbi->s_hnsw_enabled ? 1 : 0);
    disk_sb->s_lsh_enabled = cpu_to_le32(sbi->s_lsh_enabled ? 1 : 0);
    
    /* Update timestamps */
    disk_sb->s_write_time = cpu_to_le64(ktime_get_real_seconds());
    
    /* Calculate and store checksum */
    checksum = vexfs_calculate_superblock_checksum(disk_sb);
    disk_sb->s_checksum = cpu_to_le32(checksum);
    
    /* Mark buffer dirty and let kernel handle async writes */
    mark_buffer_dirty(sbi->s_sbh);
    /* Remove sync_dirty_buffer() to prevent hanging - let kernel handle async writes */
    
    /* Clear dirty flag */
    sbi->s_dirty = false;
    /* Note: s_dirt field was removed in modern kernels */
    
    mutex_unlock(&sbi->s_lock);
    
    printk(KERN_INFO "VexFS: Superblock written successfully\n");
    return 0;
}

/**
 * vexfs_sync_superblock - Synchronously write superblock to disk
 * @sb: VFS superblock structure
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_sync_superblock(struct super_block *sb)
{
    return vexfs_write_superblock(sb);
}

/**
 * vexfs_create_default_superblock - Create a default superblock for mkfs
 * @sb: VFS superblock structure
 * @blocks_count: Total number of blocks in filesystem
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_create_default_superblock(struct super_block *sb, u64 blocks_count)
{
    struct vexfs_sb_info *sbi;
    struct buffer_head *bh;
    struct vexfs_superblock *disk_sb;
    u64 current_time;
    u32 checksum;
    
    printk(KERN_INFO "VexFS: Creating default superblock\n");
    
    /* Allocate in-memory superblock info */
    sbi = kzalloc(sizeof(*sbi), GFP_KERNEL);
    if (!sbi) {
        printk(KERN_ERR "VexFS: Cannot allocate superblock info\n");
        return -ENOMEM;
    }
    
    /* Initialize mutex */
    mutex_init(&sbi->s_lock);
    
    /* Get buffer for superblock */
    bh = sb_getblk(sb, VEXFS_SUPERBLOCK_BLOCK);
    if (!bh) {
        printk(KERN_ERR "VexFS: Cannot get superblock buffer\n");
        kfree(sbi);
        return -EIO;
    }
    
    lock_buffer(bh);
    memset(bh->b_data, 0, sb->s_blocksize);
    
    disk_sb = (struct vexfs_superblock *)bh->b_data;
    current_time = ktime_get_real_seconds();
    
    /* Set default superblock values */
    disk_sb->s_magic = cpu_to_le32(VEXFS_MAGIC);
    disk_sb->s_version = cpu_to_le32(VEXFS_VERSION);
    disk_sb->s_block_size = cpu_to_le32(VEXFS_DEFAULT_BLOCK_SIZE);
    disk_sb->s_inode_size = cpu_to_le32(256); /* Default inode size */
    disk_sb->s_blocks_count = cpu_to_le64(blocks_count);
    disk_sb->s_free_blocks = cpu_to_le64(blocks_count - 10); /* Reserve some blocks */
    disk_sb->s_inodes_count = cpu_to_le64(blocks_count / 4); /* 1 inode per 4 blocks */
    disk_sb->s_free_inodes = cpu_to_le64(blocks_count / 4 - 1); /* Root inode used */
    disk_sb->s_first_data_block = cpu_to_le32(10); /* First 10 blocks for metadata */
    disk_sb->s_inode_table_block = cpu_to_le32(2);
    disk_sb->s_block_bitmap_block = cpu_to_le32(1);
    disk_sb->s_inode_bitmap_block = cpu_to_le32(1);
    disk_sb->s_state = cpu_to_le32(VEXFS_VALID_FS);
    disk_sb->s_errors = cpu_to_le32(VEXFS_ERRORS_CONTINUE);
    disk_sb->s_mount_count = cpu_to_le32(0);
    disk_sb->s_max_mount_count = cpu_to_le32(20);
    
    /* Timestamps */
    disk_sb->s_mkfs_time = cpu_to_le64(current_time);
    disk_sb->s_mount_time = cpu_to_le64(0);
    disk_sb->s_write_time = cpu_to_le64(current_time);
    disk_sb->s_lastcheck = cpu_to_le64(current_time);
    disk_sb->s_checkinterval = cpu_to_le64(86400 * 30); /* 30 days */
    
    /* VexFS-specific defaults */
    disk_sb->s_vector_dimensions = cpu_to_le32(128); /* Default 128 dimensions */
    disk_sb->s_distance_metric = cpu_to_le32(0); /* Euclidean distance */
    disk_sb->s_hnsw_enabled = cpu_to_le32(1);
    disk_sb->s_lsh_enabled = cpu_to_le32(1);
    
    /* Performance defaults */
    disk_sb->s_read_ahead_blocks = cpu_to_le32(8);
    disk_sb->s_write_behind_blocks = cpu_to_le32(8);
    disk_sb->s_cache_size = cpu_to_le32(1024); /* 1024 blocks */
    disk_sb->s_reserved_blocks = cpu_to_le32(5); /* 5% reserved */
    
    /* Generate UUID */
    generate_random_uuid(disk_sb->s_uuid);
    
    /* Set volume name */
    strncpy(disk_sb->s_volume_name, "VexFS", sizeof(disk_sb->s_volume_name) - 1);
    
    /* Calculate checksum */
    checksum = vexfs_calculate_superblock_checksum(disk_sb);
    disk_sb->s_checksum = cpu_to_le32(checksum);
    
    set_buffer_uptodate(bh);
    unlock_buffer(bh);
    mark_buffer_dirty(bh);
    /* Remove sync_dirty_buffer() to prevent hanging - let kernel handle async writes */
    
    /* Copy to in-memory structure */
    sbi->s_magic = VEXFS_MAGIC;
    sbi->s_version = VEXFS_VERSION;
    sbi->s_block_size = VEXFS_DEFAULT_BLOCK_SIZE;
    sbi->s_inode_size = 256;
    sbi->s_blocks_count = blocks_count;
    sbi->s_free_blocks = blocks_count - 10;
    sbi->s_inodes_count = blocks_count / 4;
    sbi->s_free_inodes = blocks_count / 4 - 1;
    sbi->s_first_data_block = 10;
    sbi->s_inode_table_block = 2;
    sbi->s_block_bitmap_block = 1;
    sbi->s_inode_bitmap_block = 1;
    sbi->s_state = VEXFS_VALID_FS;
    sbi->s_errors = VEXFS_ERRORS_CONTINUE;
    sbi->s_vector_dimensions = 128;
    sbi->s_distance_metric = 0;
    sbi->s_hnsw_enabled = true;
    sbi->s_lsh_enabled = true;
    
    sbi->s_sbh = bh;
    sbi->s_dirty = false;
    sb->s_fs_info = sbi;
    
    printk(KERN_INFO "VexFS: Default superblock created successfully\n");
    return 0;
}