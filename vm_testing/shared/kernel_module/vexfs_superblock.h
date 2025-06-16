/* SPDX-License-Identifier: GPL-2.0 */
/*
 * VexFS v2.0 - Superblock and Disk Persistence Structures
 *
 * This file defines the on-disk superblock structure and related
 * constants for VexFS filesystem persistence.
 *
 * Copyright (C) 2024 VexFS Development Team
 */

#ifndef _VEXFS_SUPERBLOCK_H
#define _VEXFS_SUPERBLOCK_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/mutex.h>

/* VexFS Magic Number: 'VEXF' */
#define VEXFS_MAGIC 0x56455846

/* VexFS Version */
#define VEXFS_VERSION_MAJOR 2
#define VEXFS_VERSION_MINOR 0
#define VEXFS_VERSION ((VEXFS_VERSION_MAJOR << 16) | VEXFS_VERSION_MINOR)

/* Default block size (4KB) */
#define VEXFS_DEFAULT_BLOCK_SIZE 4096
#define VEXFS_MIN_BLOCK_SIZE 512
#define VEXFS_MAX_BLOCK_SIZE 65536

/* Filesystem states */
#define VEXFS_VALID_FS    0x0001  /* Cleanly unmounted */
#define VEXFS_ERROR_FS    0x0002  /* Errors detected */
#define VEXFS_DIRTY_FS    0x0004  /* Mounted, not clean */

/* Error handling policies */
#define VEXFS_ERRORS_CONTINUE 1   /* Continue on errors */
#define VEXFS_ERRORS_RO       2   /* Remount read-only on errors */
#define VEXFS_ERRORS_PANIC    3   /* Panic on errors */

/* Superblock location */
#define VEXFS_SUPERBLOCK_BLOCK 0  /* Superblock is at block 0 */

/**
 * struct vexfs_superblock - On-disk superblock structure
 * 
 * This structure defines the layout of the VexFS superblock as stored
 * on disk. It must be exactly one block size (4096 bytes by default).
 * 
 * All multi-byte fields are stored in little-endian format.
 */
struct vexfs_superblock {
    /* Basic filesystem identification */
    __le32 s_magic;           /* Magic number: 0x56455846 ('VEXF') */
    __le32 s_version;         /* Filesystem version */
    __le32 s_block_size;      /* Block size in bytes */
    __le32 s_inode_size;      /* Size of inode structure */
    
    /* Block and inode counts */
    __le64 s_blocks_count;    /* Total blocks in filesystem */
    __le64 s_free_blocks;     /* Free blocks count */
    __le64 s_inodes_count;    /* Total inodes */
    __le64 s_free_inodes;     /* Free inodes count */
    
    /* Layout information */
    __le32 s_first_data_block; /* First data block */
    __le32 s_inode_table_block; /* First inode table block */
    __le32 s_block_bitmap_block; /* Block bitmap location */
    __le32 s_inode_bitmap_block; /* Inode bitmap location */
    
    /* Filesystem state and error handling */
    __le32 s_state;           /* Filesystem state */
    __le32 s_errors;          /* Error handling policy */
    __le32 s_mount_count;     /* Number of mounts since last check */
    __le32 s_max_mount_count; /* Maximum mounts before check */
    
    /* Timestamps */
    __le64 s_mkfs_time;       /* Filesystem creation time */
    __le64 s_mount_time;      /* Last mount time */
    __le64 s_write_time;      /* Last write time */
    __le64 s_lastcheck;       /* Last check time */
    __le64 s_checkinterval;   /* Check interval */
    
    /* Identification */
    __u8   s_uuid[16];        /* Filesystem UUID */
    char   s_volume_name[16]; /* Volume name */
    char   s_last_mounted[64]; /* Last mount point */
    
    /* VexFS-specific features */
    __le32 s_vector_dimensions; /* Default vector dimensions */
    __le32 s_distance_metric;   /* Default distance metric */
    __le32 s_hnsw_enabled;      /* HNSW index enabled */
    __le32 s_lsh_enabled;       /* LSH index enabled */
    
    /* Performance and tuning */
    __le32 s_read_ahead_blocks; /* Read-ahead block count */
    __le32 s_write_behind_blocks; /* Write-behind block count */
    __le32 s_cache_size;        /* Cache size hint */
    __le32 s_reserved_blocks;   /* Reserved blocks percentage */
    
    /* Reserved space for future extensions */
    __u8   s_reserved[3744];   /* Reserved for future use */
    
    /* Checksum (last field) */
    __le32 s_checksum;        /* Superblock checksum */
} __packed;

/* Note: Superblock size validation will be done at runtime */
/* static_assert requires C11, validated in vexfs_superblock.c instead */

/**
 * struct vexfs_sb_info - In-memory superblock information
 * 
 * This structure holds the in-memory representation of superblock
 * data and additional runtime information.
 */
struct vexfs_sb_info {
    /* Cached superblock data */
    u32 s_magic;
    u32 s_version;
    u32 s_block_size;
    u32 s_inode_size;
    u64 s_blocks_count;
    u64 s_free_blocks;
    u64 s_inodes_count;
    u64 s_free_inodes;
    u32 s_first_data_block;
    u32 s_inode_table_block;
    u32 s_block_bitmap_block;
    u32 s_inode_bitmap_block;
    u32 s_state;
    u32 s_errors;
    
    /* Runtime state */
    struct mutex s_lock;       /* Superblock lock */
    struct buffer_head *s_sbh; /* Superblock buffer head */
    bool s_dirty;              /* Superblock needs writing */
    
    /* VexFS-specific runtime data */
    u32 s_vector_dimensions;
    u32 s_distance_metric;
    bool s_hnsw_enabled;
    bool s_lsh_enabled;
    
    /* Original VexFS structures (preserved during transition) */
    struct vexfs_v2_info *vexfs_info; /* Existing VexFS data */
};

/* Function declarations */
int vexfs_read_superblock(struct super_block *sb);
int vexfs_write_superblock(struct super_block *sb);
int vexfs_sync_superblock(struct super_block *sb);
u32 vexfs_calculate_superblock_checksum(struct vexfs_superblock *sb);
int vexfs_validate_superblock(struct vexfs_superblock *sb);

/* Superblock access macros */
#define VEXFS_SB(sb) ((struct vexfs_sb_info *)(sb)->s_fs_info)

/* Utility functions */
static inline void vexfs_mark_sb_dirty(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    sbi->s_dirty = true;
    /* Note: s_dirt field was removed in modern kernels */
}

static inline bool vexfs_sb_is_dirty(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    return sbi->s_dirty;
}

#endif /* _VEXFS_SUPERBLOCK_H */