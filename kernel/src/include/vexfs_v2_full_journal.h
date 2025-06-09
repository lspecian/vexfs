/*
 * VexFS v2.0 - Full Filesystem Journal (Phase 1) Header
 * 
 * This extends the foundational journaling system with enterprise-grade features:
 * - Advanced transaction management with concurrent support
 * - Multiple journaling modes (ordered, writeback, journal)
 * - SHA-256 checksumming for cryptographic integrity
 * - Non-blocking write strategies with separate commit threads
 * - Comprehensive crash recovery mechanisms
 * - ioctl interfaces for journal management
 * - Performance-optimized journal operations
 */

#ifndef _VEXFS_V2_FULL_JOURNAL_H
#define _VEXFS_V2_FULL_JOURNAL_H

#include <linux/types.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/crypto.h>
#include <crypto/hash.h>
#include <crypto/sha2.h>
#include <linux/ioctl.h>

#include "vexfs_v2_journal.h"

/* Full Journal magic numbers and version */
#define VEXFS_FULL_JOURNAL_MAGIC    0x56455846  /* "VEXF" */
#define VEXFS_FULL_JOURNAL_VERSION  2

/* Advanced journal block types */
#define VEXFS_JOURNAL_DATA_BLOCK    0x06
#define VEXFS_JOURNAL_CHECKPOINT    0x07
#define VEXFS_JOURNAL_BARRIER       0x08

/* Journaling modes */
#define VEXFS_JOURNAL_MODE_ORDERED      0x01  /* Journal metadata, then write data */
#define VEXFS_JOURNAL_MODE_WRITEBACK    0x02  /* Journal metadata, data written anytime */
#define VEXFS_JOURNAL_MODE_JOURNAL      0x03  /* Journal both metadata and data */

/* Advanced journal flags */
#define VEXFS_JOURNAL_SHA256_CHECKSUM   0x20
#define VEXFS_JOURNAL_CONCURRENT_TRANS  0x40
#define VEXFS_JOURNAL_NON_BLOCKING      0x80
#define VEXFS_JOURNAL_BARRIER_SUPPORT   0x100

/* Transaction priority levels */
#define VEXFS_TRANS_PRIORITY_LOW        0x01
#define VEXFS_TRANS_PRIORITY_NORMAL     0x02
#define VEXFS_TRANS_PRIORITY_HIGH       0x03
#define VEXFS_TRANS_PRIORITY_CRITICAL   0x04

/* Checkpointing flags */
#define VEXFS_CHECKPOINT_FORCE          0x01
#define VEXFS_CHECKPOINT_ASYNC          0x02
#define VEXFS_CHECKPOINT_METADATA_ONLY  0x04

/* Recovery flags */
#define VEXFS_RECOVERY_FULL_SCAN        0x01
#define VEXFS_RECOVERY_FAST_MODE        0x02
#define VEXFS_RECOVERY_VERIFY_CHECKSUMS 0x04

/* Maximum values for full journal */
#define VEXFS_FULL_JOURNAL_MAX_CONCURRENT_TRANS  512
#define VEXFS_FULL_JOURNAL_MAX_COMMIT_THREADS    8
#define VEXFS_FULL_JOURNAL_BUFFER_SIZE          (64 * 1024)  /* 64KB */

/*
 * Enhanced Journal Superblock - Extends basic superblock
 */
struct vexfs_full_journal_superblock {
    struct vexfs_journal_superblock base;
    
    /* Advanced features */
    __le32 fj_journal_mode;         /* Journaling mode */
    __le32 fj_checksum_algorithm;   /* SHA-256 = 2 */
    __le32 fj_concurrent_trans;     /* Max concurrent transactions */
    __le32 fj_commit_threads;       /* Number of commit threads */
    
    /* Performance tuning */
    __le32 fj_buffer_size;          /* Journal buffer size */
    __le32 fj_checkpoint_interval;  /* Checkpoint interval in seconds */
    __le32 fj_barrier_timeout;      /* Barrier timeout in ms */
    __le32 fj_recovery_threads;     /* Recovery thread count */
    
    /* Advanced statistics */
    __le64 fj_total_checkpoints;    /* Total checkpoints created */
    __le64 fj_total_barriers;       /* Total barriers processed */
    __le64 fj_concurrent_peak;      /* Peak concurrent transactions */
    __le64 fj_recovery_time_total;  /* Total recovery time in ms */
    
    /* Integrity and validation */
    __le32 fj_feature_flags;        /* Feature compatibility flags */
    __le32 fj_reserved[12];         /* Reserved for future use */
    __le32 fj_superblock_sha256[8]; /* SHA-256 checksum */
} __packed;

/*
 * Enhanced Journal Block Header with SHA-256
 */
struct vexfs_full_journal_block_header {
    struct vexfs_journal_block_header base;
    
    __le32 fjbh_data_length;        /* Actual data length in block */
    __le32 fjbh_compression_type;   /* Compression algorithm used */
    __le32 fjbh_sha256[8];          /* SHA-256 checksum */
    __le32 fjbh_reserved[2];        /* Reserved */
} __packed;

/*
 * Data Block - For journal mode (journaling both metadata and data)
 */
struct vexfs_journal_data_block {
    struct vexfs_full_journal_block_header fjdb_header;
    
    __le64 fjdb_original_block;     /* Original filesystem block number */
    __le32 fjdb_data_size;          /* Size of data in this block */
    __le32 fjdb_flags;              /* Data block flags */
    
    /* Variable-length data follows */
    u8 fjdb_data[0];                /* Actual data */
} __packed;

/*
 * Checkpoint Block - For limiting journal size
 */
struct vexfs_journal_checkpoint {
    struct vexfs_full_journal_block_header fjcp_header;
    
    __le64 fjcp_checkpoint_id;      /* Unique checkpoint ID */
    __le64 fjcp_last_committed_seq; /* Last committed sequence */
    __le64 fjcp_filesystem_state;   /* Filesystem state hash */
    __le32 fjcp_active_trans_count; /* Active transactions at checkpoint */
    __le32 fjcp_flags;              /* Checkpoint flags */
    
    /* Checkpoint metadata */
    __le64 fjcp_metadata_blocks;    /* Number of metadata blocks */
    __le64 fjcp_data_blocks;        /* Number of data blocks */
    __le64 fjcp_free_blocks;        /* Free blocks at checkpoint */
    __le64 fjcp_timestamp;          /* Checkpoint timestamp */
} __packed;

/*
 * Barrier Block - For ordering guarantees
 */
struct vexfs_journal_barrier {
    struct vexfs_full_journal_block_header fjbr_header;
    
    __le64 fjbr_barrier_id;         /* Unique barrier ID */
    __le32 fjbr_barrier_type;       /* Type of barrier */
    __le32 fjbr_wait_count;         /* Number of transactions to wait for */
    __le64 fjbr_timeout;            /* Barrier timeout */
    __le32 fjbr_flags;              /* Barrier flags */
    __le32 fjbr_reserved[3];        /* Reserved */
} __packed;

/*
 * Enhanced Transaction Handle
 */
struct vexfs_full_journal_transaction {
    struct vexfs_journal_transaction base;
    
    /* Advanced transaction features */
    u32 ft_priority;                /* Transaction priority */
    u32 ft_journal_mode;            /* Journaling mode for this transaction */
    atomic_t ft_barrier_count;      /* Number of barriers in transaction */
    
    /* Data journaling support */
    u32 ft_data_block_count;        /* Number of data blocks */
    u64 *ft_data_block_list;        /* List of data block numbers */
    void **ft_data_buffers;         /* Data buffers for journal mode */
    
    /* Concurrent transaction support */
    struct list_head ft_dependency_list; /* Dependencies on other transactions */
    struct completion ft_barrier_completion; /* Barrier completion */
    
    /* Performance tracking */
    ktime_t ft_start_time;          /* High-resolution start time */
    ktime_t ft_commit_time;         /* High-resolution commit time */
    u32 ft_commit_thread_id;        /* ID of commit thread handling this */
    
    /* SHA-256 context for transaction integrity */
    struct crypto_shash *ft_sha256_tfm; /* SHA-256 transform */
    struct shash_desc *ft_sha256_desc;   /* SHA-256 descriptor */
} __packed;

/*
 * Journal Buffer for batching operations
 */
struct vexfs_journal_buffer {
    void *jb_buffer;                /* Buffer memory */
    size_t jb_size;                 /* Buffer size */
    size_t jb_used;                 /* Used bytes in buffer */
    atomic_t jb_transaction_count;  /* Number of transactions in buffer */
    
    spinlock_t jb_lock;             /* Buffer lock */
    struct list_head jb_transactions; /* Transactions in buffer */
    struct completion jb_flush_completion; /* Flush completion */
    
    /* Buffer state */
    unsigned long jb_flags;         /* Buffer flags */
    ktime_t jb_last_flush;          /* Last flush time */
} __packed;

/*
 * Commit Thread Context
 */
struct vexfs_commit_thread {
    struct task_struct *ct_thread;  /* Kernel thread */
    u32 ct_thread_id;               /* Thread ID */
    struct vexfs_full_journal *ct_journal; /* Parent journal */
    
    /* Thread-specific work queue */
    struct workqueue_struct *ct_workqueue;
    struct list_head ct_pending_transactions;
    spinlock_t ct_lock;
    
    /* Performance counters */
    atomic64_t ct_transactions_committed;
    atomic64_t ct_total_commit_time;
    atomic64_t ct_average_commit_time;
    
    /* Thread state */
    atomic_t ct_active;
    struct completion ct_completion;
} __packed;

/*
 * Enhanced Journal Structure
 */
struct vexfs_full_journal {
    struct vexfs_journal base;
    
    /* Advanced journaling features */
    u32 fj_journal_mode;            /* Current journaling mode */
    u32 fj_concurrent_trans_limit;  /* Max concurrent transactions */
    atomic_t fj_active_trans_count; /* Current active transactions */
    
    /* SHA-256 support */
    struct crypto_shash *fj_sha256_tfm; /* SHA-256 transform */
    
    /* Commit thread pool */
    struct vexfs_commit_thread *fj_commit_threads;
    u32 fj_commit_thread_count;
    atomic_t fj_next_commit_thread;
    
    /* Journal buffer for batching */
    struct vexfs_journal_buffer *fj_buffer;
    struct delayed_work fj_buffer_flush_work;
    
    /* Checkpointing */
    u64 fj_last_checkpoint_seq;     /* Last checkpoint sequence */
    u32 fj_checkpoint_interval;     /* Checkpoint interval in seconds */
    struct delayed_work fj_checkpoint_work;
    atomic64_t fj_checkpoint_count;
    
    /* Barrier support */
    struct list_head fj_barrier_list; /* Active barriers */
    spinlock_t fj_barrier_lock;
    atomic64_t fj_barrier_count;
    
    /* Recovery state */
    u32 fj_recovery_thread_count;   /* Number of recovery threads */
    atomic_t fj_recovery_active;    /* Recovery in progress */
    
    /* Advanced performance counters */
    atomic64_t fj_concurrent_peak;  /* Peak concurrent transactions */
    atomic64_t fj_total_barriers;   /* Total barriers processed */
    atomic64_t fj_sha256_operations; /* Total SHA-256 operations */
    atomic64_t fj_data_blocks_journaled; /* Data blocks journaled */
    
    /* Configuration */
    unsigned long fj_flags;         /* Full journal flags */
    u32 fj_buffer_size;             /* Journal buffer size */
    u32 fj_barrier_timeout;         /* Barrier timeout in ms */
} __packed;

/* ioctl interface definitions */
#define VEXFS_JOURNAL_IOC_MAGIC 'V'

#define VEXFS_JOURNAL_IOC_GET_STATUS    _IOR(VEXFS_JOURNAL_IOC_MAGIC, 1, struct vexfs_journal_status)
#define VEXFS_JOURNAL_IOC_SET_MODE      _IOW(VEXFS_JOURNAL_IOC_MAGIC, 2, u32)
#define VEXFS_JOURNAL_IOC_FORCE_COMMIT  _IO(VEXFS_JOURNAL_IOC_MAGIC, 3)
#define VEXFS_JOURNAL_IOC_CHECKPOINT    _IOW(VEXFS_JOURNAL_IOC_MAGIC, 4, u32)
#define VEXFS_JOURNAL_IOC_GET_STATS     _IOR(VEXFS_JOURNAL_IOC_MAGIC, 5, struct vexfs_full_journal_stats)
#define VEXFS_JOURNAL_IOC_SET_BUFFER    _IOW(VEXFS_JOURNAL_IOC_MAGIC, 6, u32)

/*
 * ioctl structures
 */
struct vexfs_journal_status {
    u32 js_mode;                    /* Current journaling mode */
    u32 js_active_transactions;     /* Active transactions */
    u64 js_head_sequence;           /* Head sequence number */
    u64 js_tail_sequence;           /* Tail sequence number */
    u32 js_utilization;             /* Journal utilization percentage */
    u32 js_flags;                   /* Journal flags */
};

struct vexfs_full_journal_stats {
    /* Base statistics */
    u64 fjs_total_commits;
    u64 fjs_total_aborts;
    u64 fjs_total_transactions;
    u64 fjs_blocks_written;
    
    /* Advanced statistics */
    u64 fjs_concurrent_peak;
    u64 fjs_total_checkpoints;
    u64 fjs_total_barriers;
    u64 fjs_sha256_operations;
    u64 fjs_data_blocks_journaled;
    
    /* Performance metrics */
    u64 fjs_average_commit_time;
    u64 fjs_average_transaction_size;
    u32 fjs_buffer_utilization;
    u32 fjs_commit_thread_efficiency;
};

/* Function declarations */

/* Full journal initialization and cleanup */
struct vexfs_full_journal *vexfs_full_journal_init(struct super_block *sb,
                                                   u64 start_block, u64 total_blocks,
                                                   u32 journal_mode);
void vexfs_full_journal_destroy(struct vexfs_full_journal *journal);
int vexfs_full_journal_load(struct vexfs_full_journal *journal);
int vexfs_full_journal_create(struct vexfs_full_journal *journal);

/* Enhanced transaction management */
struct vexfs_full_journal_transaction *vexfs_full_journal_start(
    struct vexfs_full_journal *journal, u32 max_blocks, u32 operation_type, u32 priority);
int vexfs_full_journal_commit(struct vexfs_full_journal_transaction *trans);
int vexfs_full_journal_abort(struct vexfs_full_journal_transaction *trans);

/* Data journaling operations */
int vexfs_full_journal_add_data_block(struct vexfs_full_journal_transaction *trans,
                                      u64 block_number, void *data, size_t size);
int vexfs_full_journal_write_data_blocks(struct vexfs_full_journal_transaction *trans);

/* Checkpointing operations */
int vexfs_full_journal_create_checkpoint(struct vexfs_full_journal *journal, u32 flags);
int vexfs_full_journal_restore_checkpoint(struct vexfs_full_journal *journal, u64 checkpoint_id);

/* Barrier operations */
int vexfs_full_journal_add_barrier(struct vexfs_full_journal_transaction *trans,
                                   u32 barrier_type, u32 timeout);
int vexfs_full_journal_wait_barrier(struct vexfs_full_journal_transaction *trans);

/* SHA-256 checksumming */
int vexfs_full_journal_calculate_sha256(const void *data, size_t len, u8 *hash);
int vexfs_full_journal_verify_sha256(const void *data, size_t len, const u8 *expected_hash);

/* Advanced recovery operations */
int vexfs_full_journal_recover(struct vexfs_full_journal *journal, u32 flags);
int vexfs_full_journal_scan_for_transactions(struct vexfs_full_journal *journal,
                                             u64 start_seq, u64 end_seq);
int vexfs_full_journal_replay_transaction(struct vexfs_full_journal *journal,
                                          u64 transaction_id);

/* Journal mode management */
int vexfs_full_journal_set_mode(struct vexfs_full_journal *journal, u32 mode);
u32 vexfs_full_journal_get_mode(struct vexfs_full_journal *journal);

/* Buffer management */
int vexfs_full_journal_flush_buffer(struct vexfs_full_journal *journal);
int vexfs_full_journal_resize_buffer(struct vexfs_full_journal *journal, u32 new_size);

/* ioctl interface */
long vexfs_full_journal_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/* Statistics and monitoring */
void vexfs_full_journal_get_stats(struct vexfs_full_journal *journal,
                                  struct vexfs_full_journal_stats *stats);

/* Utility functions */
int vexfs_full_journal_force_commit_all(struct vexfs_full_journal *journal);
int vexfs_full_journal_sync(struct vexfs_full_journal *journal);

#endif /* _VEXFS_V2_FULL_JOURNAL_H */