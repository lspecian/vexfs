/*
 * VexFS v2.0 - Full FS Journal Core Structure
 * 
 * This implements the foundational journaling system for VexFS as part of the
 * AI-Native Semantic Substrate roadmap (Phase 1). Provides block-level integrity
 * and fast crash recovery with Write-Ahead Logging (WAL) principles.
 *
 * Based on analysis of existing journaling mechanisms:
 * - Ext4/JBD2: Circular log with descriptor/commit blocks
 * - Btrfs: Copy-on-write with transaction trees
 * - XFS: Logical log with variable-length records
 * - ZFS: Intent log with transaction groups
 *
 * VexFS Journal Design:
 * - Circular log structure for efficient space utilization
 * - Strict Write-Ahead Logging for consistency
 * - Checksumming for corruption detection
 * - Non-blocking writes with asynchronous operations
 * - Integration with existing VexFS vector capabilities
 */

#ifndef _VEXFS_V2_JOURNAL_H
#define _VEXFS_V2_JOURNAL_H

#include <linux/types.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/crc32.h>

/* Journal magic numbers and version */
#define VEXFS_JOURNAL_MAGIC         0x56455846  /* "VEXF" */
#define VEXFS_JOURNAL_VERSION_MAJOR 1
#define VEXFS_JOURNAL_VERSION_MINOR 0

/* Journal block types */
#define VEXFS_JOURNAL_SUPERBLOCK    0x01
#define VEXFS_JOURNAL_DESCRIPTOR    0x02
#define VEXFS_JOURNAL_DATA          0x03
#define VEXFS_JOURNAL_COMMIT        0x04
#define VEXFS_JOURNAL_REVOCATION    0x05

/* Journal flags */
#define VEXFS_JOURNAL_ACTIVE        0x01
#define VEXFS_JOURNAL_RECOVERING    0x02
#define VEXFS_JOURNAL_ABORTING      0x04
#define VEXFS_JOURNAL_CHECKSUM      0x08
#define VEXFS_JOURNAL_ASYNC_COMMIT  0x10

/* Transaction states */
#define VEXFS_TRANS_RUNNING         0x01
#define VEXFS_TRANS_LOCKED          0x02
#define VEXFS_TRANS_FLUSHING        0x04
#define VEXFS_TRANS_COMMIT          0x08
#define VEXFS_TRANS_FINISHED        0x10

/* Journal operation types for semantic logging */
#define VEXFS_JOURNAL_OP_CREATE     0x01
#define VEXFS_JOURNAL_OP_DELETE     0x02
#define VEXFS_JOURNAL_OP_WRITE      0x03
#define VEXFS_JOURNAL_OP_TRUNCATE   0x04
#define VEXFS_JOURNAL_OP_VECTOR_ADD 0x05
#define VEXFS_JOURNAL_OP_VECTOR_DEL 0x06
#define VEXFS_JOURNAL_OP_INDEX_UPD  0x07

/* Maximum values */
#define VEXFS_JOURNAL_MAX_TRANS     256
#define VEXFS_JOURNAL_MAX_BLOCKS    65536
#define VEXFS_JOURNAL_BLOCK_SIZE    4096

/*
 * Journal Superblock - Contains journal metadata and configuration
 * Located at the beginning of the journal area
 */
struct vexfs_journal_superblock {
    __le32 j_magic;                 /* Journal magic number */
    __le32 j_version_major;         /* Major version */
    __le32 j_version_minor;         /* Minor version */
    __le32 j_flags;                 /* Journal flags */
    
    /* Journal geometry */
    __le64 j_start_block;           /* First block of journal */
    __le64 j_total_blocks;          /* Total blocks in journal */
    __le32 j_block_size;            /* Journal block size */
    __le32 j_max_trans_blocks;      /* Max blocks per transaction */
    
    /* Circular log pointers */
    __le64 j_head;                  /* Current head position */
    __le64 j_tail;                  /* Current tail position */
    __le64 j_sequence;              /* Current sequence number */
    __le64 j_commit_sequence;       /* Last committed sequence */
    
    /* Performance and reliability */
    __le32 j_commit_interval;       /* Commit interval in ms */
    __le32 j_sync_mode;             /* Synchronization mode */
    __le32 j_checksum_type;         /* Checksum algorithm */
    __le32 j_features;              /* Feature flags */
    
    /* Statistics */
    __le64 j_total_commits;         /* Total commits performed */
    __le64 j_total_aborts;          /* Total aborts */
    __le64 j_recovery_count;        /* Number of recoveries */
    __le64 j_last_recovery_time;    /* Last recovery timestamp */
    
    /* Checksums and validation */
    __le32 j_superblock_checksum;   /* Superblock checksum */
    __le32 j_reserved[15];          /* Reserved for future use */
} __packed;

/*
 * Journal Block Header - Common header for all journal blocks
 */
struct vexfs_journal_block_header {
    __le32 jbh_magic;               /* Block magic number */
    __le32 jbh_type;                /* Block type */
    __le64 jbh_sequence;            /* Sequence number */
    __le32 jbh_checksum;            /* Block checksum */
    __le32 jbh_flags;               /* Block flags */
} __packed;

/*
 * Journal Descriptor Block - Describes a transaction
 */
struct vexfs_journal_descriptor {
    struct vexfs_journal_block_header jd_header;
    
    __le64 jd_transaction_id;       /* Transaction ID */
    __le32 jd_block_count;          /* Number of blocks in transaction */
    __le32 jd_operation_type;       /* Type of operation */
    __le64 jd_timestamp;            /* Transaction timestamp */
    __le32 jd_uid;                  /* User ID */
    __le32 jd_gid;                  /* Group ID */
    
    /* Variable-length array of block numbers follows */
    __le64 jd_blocks[0];            /* Block numbers being journaled */
} __packed;

/*
 * Journal Commit Block - Marks transaction completion
 */
struct vexfs_journal_commit {
    struct vexfs_journal_block_header jc_header;
    
    __le64 jc_transaction_id;       /* Transaction ID */
    __le64 jc_commit_time;          /* Commit timestamp */
    __le32 jc_block_count;          /* Number of blocks committed */
    __le32 jc_checksum_type;        /* Checksum algorithm used */
    __le32 jc_transaction_checksum; /* Transaction data checksum */
    __le32 jc_reserved[3];          /* Reserved */
} __packed;

/*
 * Journal Revocation Block - Invalidates stale journal entries
 */
struct vexfs_journal_revocation {
    struct vexfs_journal_block_header jr_header;
    
    __le32 jr_revocation_count;     /* Number of revoked blocks */
    __le32 jr_reserved;             /* Reserved */
    
    /* Variable-length array of revoked block numbers follows */
    __le64 jr_revoked_blocks[0];    /* Revoked block numbers */
} __packed;

/*
 * In-memory transaction handle
 */
struct vexfs_journal_transaction {
    /* Transaction identification */
    u64 t_transaction_id;           /* Unique transaction ID */
    u32 t_state;                    /* Transaction state */
    atomic_t t_ref_count;           /* Reference count */
    
    /* Transaction timing */
    unsigned long t_start_time;     /* Transaction start time */
    unsigned long t_commit_time;    /* Transaction commit time */
    
    /* Block tracking */
    u32 t_block_count;              /* Number of blocks in transaction */
    u32 t_max_blocks;               /* Maximum blocks allowed */
    u64 *t_block_list;              /* List of block numbers */
    
    /* Synchronization */
    struct mutex t_mutex;           /* Transaction mutex */
    struct completion t_completion; /* Completion for waiters */
    
    /* Journal integration */
    struct vexfs_journal *t_journal; /* Parent journal */
    struct list_head t_list;        /* List of transactions */
    
    /* Operation metadata for semantic logging */
    u32 t_operation_type;           /* Type of operation */
    u32 t_uid;                      /* User ID */
    u32 t_gid;                      /* Group ID */
    
    /* Error handling */
    int t_error;                    /* Transaction error code */
    unsigned long t_flags;          /* Transaction flags */
};

/*
 * In-memory journal structure
 */
struct vexfs_journal {
    /* Journal identification */
    struct super_block *j_sb;       /* Associated superblock */
    struct block_device *j_bdev;    /* Journal block device */
    
    /* Journal geometry */
    u64 j_start_block;              /* First block of journal */
    u64 j_total_blocks;             /* Total blocks in journal */
    u32 j_block_size;               /* Journal block size */
    
    /* Circular log management */
    u64 j_head;                     /* Current head position */
    u64 j_tail;                     /* Current tail position */
    u64 j_sequence;                 /* Current sequence number */
    u64 j_commit_sequence;          /* Last committed sequence */
    
    /* Transaction management */
    struct list_head j_transactions; /* Active transactions */
    struct mutex j_trans_mutex;     /* Transaction list mutex */
    atomic_t j_trans_count;         /* Number of active transactions */
    u64 j_next_trans_id;            /* Next transaction ID */
    
    /* Synchronization */
    spinlock_t j_lock;              /* Journal spinlock */
    struct mutex j_mutex;           /* Journal mutex */
    struct rw_semaphore j_rw_sem;   /* Reader-writer semaphore */
    
    /* Commit thread and work */
    struct task_struct *j_commit_thread; /* Commit thread */
    struct workqueue_struct *j_workqueue; /* Journal workqueue */
    struct delayed_work j_commit_work;   /* Commit work */
    
    /* Journal state */
    unsigned long j_flags;          /* Journal flags */
    int j_error;                    /* Journal error state */
    atomic_t j_ref_count;           /* Reference count */
    
    /* Performance counters */
    atomic64_t j_commits;           /* Total commits */
    atomic64_t j_aborts;            /* Total aborts */
    atomic64_t j_blocks_written;    /* Total blocks written */
    atomic64_t j_transactions;      /* Total transactions */
    
    /* Recovery state */
    u64 j_recovery_start;           /* Recovery start sequence */
    u64 j_recovery_end;             /* Recovery end sequence */
    unsigned long j_recovery_time;  /* Last recovery time */
    
    /* Buffer management */
    struct buffer_head **j_buffers; /* Journal buffers */
    u32 j_buffer_count;             /* Number of buffers */
    
    /* Checksum support */
    u32 j_checksum_type;            /* Checksum algorithm */
    struct crypto_shash *j_checksum_tfm; /* Checksum transform */
};

/* Function declarations */

/* Journal initialization and cleanup */
struct vexfs_journal *vexfs_journal_init(struct super_block *sb, 
                                         u64 start_block, u64 total_blocks);
void vexfs_journal_destroy(struct vexfs_journal *journal);
int vexfs_journal_load(struct vexfs_journal *journal);
int vexfs_journal_create(struct vexfs_journal *journal);

/* Transaction management */
struct vexfs_journal_transaction *vexfs_journal_start(struct vexfs_journal *journal,
                                                     u32 max_blocks, u32 operation_type);
int vexfs_journal_extend(struct vexfs_journal_transaction *trans, u32 additional_blocks);
int vexfs_journal_commit(struct vexfs_journal_transaction *trans);
int vexfs_journal_abort(struct vexfs_journal_transaction *trans);

/* Block operations */
int vexfs_journal_get_write_access(struct vexfs_journal_transaction *trans,
                                  struct buffer_head *bh);
int vexfs_journal_dirty_metadata(struct vexfs_journal_transaction *trans,
                                struct buffer_head *bh);
int vexfs_journal_forget(struct vexfs_journal_transaction *trans,
                        struct buffer_head *bh);

/* Recovery operations */
int vexfs_journal_recover(struct vexfs_journal *journal);
int vexfs_journal_replay_transactions(struct vexfs_journal *journal,
                                     u64 start_seq, u64 end_seq);

/* Utility functions */
int vexfs_journal_flush(struct vexfs_journal *journal);
int vexfs_journal_force_commit(struct vexfs_journal *journal);
u32 vexfs_journal_calculate_checksum(const void *data, size_t len, u32 seed);

/* Statistics and monitoring */
void vexfs_journal_get_stats(struct vexfs_journal *journal, 
                            struct vexfs_journal_stats *stats);

/* Journal statistics structure */
struct vexfs_journal_stats {
    u64 total_commits;
    u64 total_aborts;
    u64 total_transactions;
    u64 blocks_written;
    u64 recovery_count;
    u32 active_transactions;
    u32 journal_utilization;
    unsigned long last_commit_time;
    unsigned long last_recovery_time;
};

#endif /* _VEXFS_V2_JOURNAL_H */