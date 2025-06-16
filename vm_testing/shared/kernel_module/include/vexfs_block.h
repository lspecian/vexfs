/*
 * VexFS Block Allocation and Management
 * 
 * This header defines block allocation, bitmap management, and
 * disk persistence operations following Linux filesystem patterns.
 */

#ifndef VEXFS_BLOCK_H
#define VEXFS_BLOCK_H

#include "vexfs_core.h"

/* Block allocation constants */
#define VEXFS_BLOCKS_PER_GROUP 8192
#define VEXFS_BITMAP_BLOCKS 1
#define VEXFS_INODE_TABLE_BLOCKS 64

/* Disk layout constants */
#define VEXFS_INODE_TABLE_BLOCK (1 + VEXFS_BITMAP_BLOCKS)  /* Block 2 */
#define VEXFS_INODES_PER_BLOCK (VEXFS_BLOCK_SIZE / sizeof(struct vexfs_inode))

/* Superblock structure (on-disk format) */
struct vexfs_super_block {
    __le32 s_magic;           /* Magic number */
    __le32 s_block_size;      /* Block size */
    __le32 s_blocks_count;    /* Total blocks */
    __le32 s_free_blocks;     /* Free blocks */
    __le32 s_inodes_count;    /* Total inodes */
    __le32 s_free_inodes;     /* Free inodes */
    __le32 s_first_data_block; /* First data block */
    __le32 s_log_block_size;  /* Block size = 1024 << s_log_block_size */
    __le32 s_blocks_per_group; /* Blocks per group */
    __le32 s_inodes_per_group; /* Inodes per group */
    __le32 s_mtime;           /* Mount time */
    __le32 s_wtime;           /* Write time */
    __le16 s_mnt_count;       /* Mount count */
    __le16 s_max_mnt_count;   /* Maximum mount count */
    __le16 s_state;           /* Filesystem state */
    __le16 s_errors;          /* Error handling */
    __le16 s_minor_rev_level; /* Minor revision level */
    __le32 s_lastcheck;       /* Last check time */
    __le32 s_checkinterval;   /* Check interval */
    __le32 s_creator_os;      /* Creator OS */
    __le32 s_rev_level;       /* Revision level */
    __le16 s_def_resuid;      /* Default reserved user ID */
    __le16 s_def_resgid;      /* Default reserved group ID */
    __le32 s_first_ino;       /* First non-reserved inode */
    __le16 s_inode_size;      /* Inode size */
    __le16 s_block_group_nr;  /* Block group number */
    __le32 s_feature_compat;  /* Compatible features */
    __le32 s_feature_incompat; /* Incompatible features */
    __le32 s_feature_ro_compat; /* Read-only compatible features */
    __u8   s_uuid[16];        /* Filesystem UUID */
    char   s_volume_name[16]; /* Volume name */
    char   s_last_mounted[64]; /* Last mount point */
    __le32 s_algorithm_usage_bitmap; /* Compression algorithms */
    __u8   s_prealloc_blocks; /* Preallocated blocks */
    __u8   s_prealloc_dir_blocks; /* Preallocated directory blocks */
    __le16 s_reserved_gdt_blocks; /* Reserved GDT blocks */
    __u8   s_journal_uuid[16]; /* Journal UUID */
    __le32 s_journal_inum;    /* Journal inode number */
    __le32 s_journal_dev;     /* Journal device */
    __le32 s_last_orphan;     /* Last orphaned inode */
    __le32 s_hash_seed[4];    /* Hash seed */
    __u8   s_def_hash_version; /* Default hash version */
    __u8   s_jnl_backup_type; /* Journal backup type */
    __le16 s_desc_size;       /* Group descriptor size */
    __le32 s_default_mount_opts; /* Default mount options */
    __le32 s_first_meta_bg;   /* First meta block group */
    __le32 s_mkfs_time;       /* Filesystem creation time */
    __le32 s_jnl_blocks[17];  /* Journal backup blocks */
    __le32 s_blocks_count_hi; /* High 32 bits of block count */
    __le32 s_r_blocks_count_hi; /* High 32 bits of reserved blocks */
    __le32 s_free_blocks_count_hi; /* High 32 bits of free blocks */
    __le16 s_min_extra_isize; /* Minimum extra inode size */
    __le16 s_want_extra_isize; /* Desired extra inode size */
    __le32 s_flags;           /* Miscellaneous flags */
    __le16 s_raid_stride;     /* RAID stride */
    __le16 s_mmp_update_interval; /* MMP update interval */
    __le64 s_mmp_block;       /* MMP block number */
    __le32 s_raid_stripe_width; /* RAID stripe width */
    __u8   s_log_groups_per_flex; /* Groups per flex group */
    __u8   s_checksum_type;   /* Metadata checksum algorithm */
    __le16 s_reserved_pad;    /* Padding */
    __le64 s_kbytes_written;  /* Kilobytes written */
    __le32 s_snapshot_inum;   /* Snapshot inode number */
    __le32 s_snapshot_id;     /* Snapshot ID */
    __le64 s_snapshot_r_blocks_count; /* Reserved blocks for snapshot */
    __le32 s_snapshot_list;   /* Snapshot list head */
    __le32 s_error_count;     /* Error count */
    __le32 s_first_error_time; /* First error time */
    __le32 s_first_error_ino; /* First error inode */
    __le64 s_first_error_block; /* First error block */
    __u8   s_first_error_func[32]; /* First error function */
    __le32 s_first_error_line; /* First error line */
    __le32 s_last_error_time; /* Last error time */
    __le32 s_last_error_ino;  /* Last error inode */
    __le32 s_last_error_line; /* Last error line */
    __le64 s_last_error_block; /* Last error block */
    __u8   s_last_error_func[32]; /* Last error function */
    __u8   s_mount_opts[64];  /* Mount options */
    __le32 s_usr_quota_inum;  /* User quota inode */
    __le32 s_grp_quota_inum;  /* Group quota inode */
    __le32 s_overhead_clusters; /* Overhead clusters */
    __le32 s_backup_bgs[2];   /* Backup block groups */
    __u8   s_encrypt_algos[4]; /* Encryption algorithms */
    __u8   s_encrypt_pw_salt[16]; /* Encryption password salt */
    __le32 s_lpf_ino;         /* Lost+found inode */
    __le32 s_prj_quota_inum;  /* Project quota inode */
    __le32 s_checksum_seed;   /* Checksum seed */
    __u8   s_wtime_hi;        /* High byte of write time */
    __u8   s_mtime_hi;        /* High byte of mount time */
    __u8   s_mkfs_time_hi;    /* High byte of mkfs time */
    __u8   s_lastcheck_hi;    /* High byte of last check time */
    __u8   s_first_error_time_hi; /* High byte of first error time */
    __u8   s_last_error_time_hi; /* High byte of last error time */
    __u8   s_pad[2];          /* Padding */
    __le16 s_encoding;        /* Filename charset encoding */
    __le16 s_encoding_flags;  /* Filename charset encoding flags */
    __le32 s_reserved[95];    /* Reserved for future use */
    __le32 s_checksum;        /* Superblock checksum */
};

/* On-disk inode structure - FIXED to match mkfs.vexfs format */
struct vexfs_inode {
    __le16 i_mode;            /* File mode */
    __le16 i_links_count;     /* Links count */
    __le32 i_uid;             /* Owner UID - FIXED: 32-bit to match mkfs */
    __le32 i_gid;             /* Group ID - FIXED: 32-bit to match mkfs */
    __le64 i_size;            /* File size - FIXED: 64-bit to match mkfs */
    __le32 i_atime;           /* Access time */
    __le32 i_ctime;           /* Creation time */
    __le32 i_mtime;           /* Modification time */
    __le32 i_blocks;          /* Blocks count */
    __le32 i_block[12];       /* Pointers to blocks - FIXED: 12 blocks to match mkfs */
    __le32 i_flags;           /* File flags */
    __le32 i_generation;      /* File version */
    __le32 i_reserved[3];     /* Reserved for future use */
};

/* Block allocation functions */
int vexfs_alloc_block(struct super_block *sb, __u32 *block);
void vexfs_free_block(struct super_block *sb, __u32 block);
int vexfs_alloc_inode_num(struct super_block *sb, __u32 *ino);
void vexfs_free_inode_num(struct super_block *sb, __u32 ino);

/* Superblock operations */
int vexfs_read_super(struct super_block *sb);
int vexfs_write_super(struct super_block *sb);
int vexfs_sync_fs(struct super_block *sb, int wait);

/* Block I/O operations */
struct buffer_head *vexfs_bread(struct super_block *sb, __u32 block);
int vexfs_bwrite(struct super_block *sb, __u32 block, void *data);

/* Bitmap operations */
int vexfs_test_bit(void *bitmap, int bit);
void vexfs_set_bit(void *bitmap, int bit);
void vexfs_clear_bit(void *bitmap, int bit);
int vexfs_find_first_zero_bit(void *bitmap, int size);

#endif /* VEXFS_BLOCK_H */