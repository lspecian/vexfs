/*
 * VexFS v2.0 - Safe Block/Inode Journaling (Task 5)
 * 
 * This implements comprehensive allocation tracking and recovery for VexFS as part of the
 * AI-Native Semantic Substrate roadmap (Phase 1). Builds on the Full FS Journal (Task 1),
 * Atomic Operations (Task 2), and Metadata Journaling (Task 3) to provide complete
 * allocation integrity and orphan detection/resolution.
 *
 * Key Features:
 * - Block allocation journaling with bitmap change tracking
 * - Inode allocation journaling with atomic bitmap updates
 * - Orphan detection and resolution for blocks and inodes
 * - Efficient kernel-space bitmap operations (custom implementation)
 * - Separate journal area for allocation metadata if needed
 * - Fragmentation optimization through intelligent allocation strategies
 * - Background consistency checking and orphan cleanup
 * - Integration with Phase 1 journaling infrastructure
 */

#ifndef _VEXFS_V2_ALLOCATION_JOURNAL_H
#define _VEXFS_V2_ALLOCATION_JOURNAL_H

#include <linux/types.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/buffer_head.h>
#include <linux/fs.h>
#include <linux/bitmap.h>
#include <linux/percpu.h>
#include <linux/timer.h>

#include "vexfs_v2_journal.h"
#include "vexfs_v2_atomic.h"
#include "vexfs_v2_metadata_journal.h"

/* Allocation operation types for journaling */
#define VEXFS_ALLOC_OP_BLOCK_ALLOC      0x01
#define VEXFS_ALLOC_OP_BLOCK_FREE       0x02
#define VEXFS_ALLOC_OP_INODE_ALLOC      0x03
#define VEXFS_ALLOC_OP_INODE_FREE       0x04
#define VEXFS_ALLOC_OP_BITMAP_UPDATE    0x05
#define VEXFS_ALLOC_OP_GROUP_INIT       0x06
#define VEXFS_ALLOC_OP_ORPHAN_CLEANUP   0x07
#define VEXFS_ALLOC_OP_VECTOR_ALLOC     0x08
#define VEXFS_ALLOC_OP_JOURNAL_ALLOC    0x09

/* Allocation journaling flags */
#define VEXFS_ALLOC_JOURNAL_SYNC        0x01
#define VEXFS_ALLOC_JOURNAL_ASYNC       0x02
#define VEXFS_ALLOC_JOURNAL_ORDERED     0x04
#define VEXFS_ALLOC_JOURNAL_BATCH       0x08
#define VEXFS_ALLOC_JOURNAL_CHECKSUM    0x10
#define VEXFS_ALLOC_JOURNAL_BACKGROUND  0x20

/* Allocation strategy types */
#define VEXFS_ALLOC_STRATEGY_FIRST_FIT  0x01
#define VEXFS_ALLOC_STRATEGY_BEST_FIT   0x02
#define VEXFS_ALLOC_STRATEGY_WORST_FIT  0x03
#define VEXFS_ALLOC_STRATEGY_BUDDY      0x04
#define VEXFS_ALLOC_STRATEGY_VECTOR_OPT 0x05

/* Orphan detection types */
#define VEXFS_ORPHAN_TYPE_BLOCK         0x01
#define VEXFS_ORPHAN_TYPE_INODE         0x02
#define VEXFS_ORPHAN_TYPE_VECTOR_DATA   0x03
#define VEXFS_ORPHAN_TYPE_INDEX_DATA    0x04

/* Maximum values for allocation journaling */
#define VEXFS_ALLOC_MAX_GROUPS          65536
#define VEXFS_ALLOC_MAX_BATCH_SIZE      256
#define VEXFS_ALLOC_MAX_ORPHANS         4096
#define VEXFS_ALLOC_BITMAP_CACHE_SIZE   1024
#define VEXFS_ALLOC_BLOCKS_PER_GROUP    32768
#define VEXFS_ALLOC_INODES_PER_GROUP    8192

/* Allocation group flags */
#define VEXFS_ALLOC_GROUP_ACTIVE        0x01
#define VEXFS_ALLOC_GROUP_FULL          0x02
#define VEXFS_ALLOC_GROUP_CORRUPTED     0x04
#define VEXFS_ALLOC_GROUP_RECOVERING    0x08
#define VEXFS_ALLOC_GROUP_VECTOR_OPT    0x10

/*
 * Kernel-space bitmap operations structure
 * Custom implementation since bitvec is userspace-only
 */
struct vexfs_kernel_bitmap {
    unsigned long *bits;                /* Bitmap data */
    u32 size_bits;                      /* Size in bits */
    u32 size_bytes;                     /* Size in bytes */
    u32 size_longs;                     /* Size in longs */
    
    /* Performance optimization */
    u32 last_set_bit;                   /* Last set bit for optimization */
    u32 last_clear_bit;                 /* Last clear bit for optimization */
    atomic_t set_bits;                  /* Number of set bits */
    
    /* Synchronization */
    spinlock_t bitmap_lock;             /* Bitmap spinlock */
    
    /* Checksum for integrity */
    u32 checksum;                       /* Bitmap checksum */
    unsigned long last_update;          /* Last update timestamp */
};

/*
 * Allocation group descriptor
 */
struct vexfs_allocation_group {
    u32 group_id;                       /* Group ID */
    u32 flags;                          /* Group flags */
    u64 start_block;                    /* First block in group */
    u32 block_count;                    /* Total blocks in group */
    u32 inode_count;                    /* Total inodes in group */
    
    /* Block allocation tracking */
    struct vexfs_kernel_bitmap *block_bitmap; /* Block allocation bitmap */
    atomic_t free_blocks;               /* Free blocks count */
    u32 largest_free_extent;            /* Largest free extent */
    
    /* Inode allocation tracking */
    struct vexfs_kernel_bitmap *inode_bitmap; /* Inode allocation bitmap */
    atomic_t free_inodes;               /* Free inodes count */
    
    /* Allocation strategy optimization */
    u32 allocation_strategy;            /* Allocation strategy */
    u32 fragmentation_score;            /* Fragmentation score (0-100) */
    u32 vector_alignment_blocks;        /* Vector data alignment */
    
    /* Journal integration */
    u64 last_journal_sequence;          /* Last journaled sequence */
    struct list_head pending_allocs;    /* Pending allocations */
    
    /* Performance counters */
    atomic64_t alloc_operations;        /* Total allocation operations */
    atomic64_t free_operations;         /* Total free operations */
    atomic64_t fragmentation_events;    /* Fragmentation events */
    
    /* Synchronization */
    struct rw_semaphore group_rwsem;    /* Group read-write semaphore */
    struct mutex alloc_mutex;           /* Allocation mutex */
    
    /* List management */
    struct list_head group_list;        /* List of allocation groups */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Allocation operation descriptor for journaling
 */
struct vexfs_allocation_operation {
    u32 op_type;                        /* Operation type */
    u32 op_flags;                       /* Operation flags */
    u64 op_id;                          /* Unique operation ID */
    u64 transaction_id;                 /* Associated transaction ID */
    
    /* Target allocation information */
    u32 group_id;                       /* Target allocation group */
    u64 start_block;                    /* Start block/inode number */
    u32 count;                          /* Number of blocks/inodes */
    u32 alignment;                      /* Alignment requirement */
    
    /* Before/after state for rollback */
    struct vexfs_kernel_bitmap *before_bitmap; /* Bitmap before operation */
    struct vexfs_kernel_bitmap *after_bitmap;  /* Bitmap after operation */
    
    /* Vector-specific allocation data */
    u32 vector_dimensions;              /* Vector dimensions */
    u32 vector_element_type;            /* Vector element type */
    u32 vectors_per_block;              /* Vectors per block */
    
    /* Integrity verification */
    u32 bitmap_checksum_before;         /* Bitmap checksum before */
    u32 bitmap_checksum_after;          /* Bitmap checksum after */
    u32 operation_checksum;             /* Operation checksum */
    
    /* Timing and ordering */
    u64 sequence_number;                /* Sequence number for ordering */
    unsigned long timestamp;            /* Operation timestamp */
    
    /* Completion tracking */
    struct completion op_completion;    /* Operation completion */
    atomic_t op_state;                  /* Operation state */
    int op_result;                      /* Operation result */
    
    /* List management */
    struct list_head op_list;           /* List of operations */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Orphan entry for detection and cleanup
 */
struct vexfs_orphan_entry {
    u32 orphan_type;                    /* Type of orphan */
    u64 block_number;                   /* Block/inode number */
    u32 group_id;                       /* Allocation group */
    
    /* Orphan metadata */
    u64 size;                           /* Size of orphaned data */
    u64 last_access_time;               /* Last access time */
    u32 reference_count;                /* Reference count */
    
    /* Detection information */
    unsigned long detection_time;       /* When orphan was detected */
    u32 detection_method;               /* How orphan was detected */
    u32 cleanup_attempts;               /* Number of cleanup attempts */
    
    /* Recovery information */
    void *recovery_data;                /* Data for recovery */
    size_t recovery_size;               /* Size of recovery data */
    
    /* List management */
    struct list_head orphan_list;       /* List of orphans */
    struct rb_node orphan_node;         /* Red-black tree node */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Allocation journal manager
 */
struct vexfs_allocation_journal_manager {
    /* Journal integration */
    struct vexfs_journal *journal;      /* Associated journal */
    struct vexfs_atomic_manager *atomic_mgr; /* Atomic operations manager */
    struct vexfs_metadata_journal_manager *meta_mgr; /* Metadata journal manager */
    
    /* Allocation groups management */
    struct list_head allocation_groups; /* List of allocation groups */
    struct vexfs_allocation_group **group_array; /* Array for fast access */
    struct mutex groups_mutex;          /* Groups list mutex */
    atomic_t active_groups;             /* Number of active groups */
    u32 max_groups;                     /* Maximum groups */
    
    /* Operation management */
    struct list_head pending_ops;       /* Pending operations */
    struct mutex ops_mutex;             /* Operations list mutex */
    atomic_t pending_count;             /* Number of pending operations */
    u64 next_op_id;                     /* Next operation ID */
    
    /* Batch processing */
    struct workqueue_struct *batch_workqueue; /* Batch processing workqueue */
    struct delayed_work batch_work;     /* Batch processing work */
    u32 batch_size;                     /* Current batch size */
    u32 max_batch_size;                 /* Maximum batch size */
    
    /* Orphan detection and cleanup */
    struct rb_root orphan_tree;         /* Orphan tree root */
    struct list_head orphan_list;       /* Orphan list */
    struct mutex orphan_mutex;          /* Orphan mutex */
    atomic_t orphan_count;              /* Number of orphans */
    u32 max_orphans;                    /* Maximum orphans */
    
    /* Background consistency checking */
    struct workqueue_struct *consistency_workqueue; /* Consistency workqueue */
    struct delayed_work consistency_work; /* Consistency work */
    struct timer_list consistency_timer; /* Consistency timer */
    u32 consistency_interval;           /* Consistency check interval */
    
    /* Bitmap cache for performance */
    struct kmem_cache *bitmap_cache;    /* Bitmap allocation cache */
    struct list_head cached_bitmaps;    /* Cached bitmaps */
    struct mutex cache_mutex;           /* Cache mutex */
    atomic_t cached_bitmap_count;       /* Number of cached bitmaps */
    
    /* Allocation strategy optimization */
    u32 default_strategy;               /* Default allocation strategy */
    u32 vector_strategy;                /* Vector-optimized strategy */
    u32 fragmentation_threshold;        /* Fragmentation threshold */
    
    /* Performance optimization */
    atomic64_t ops_processed;           /* Total operations processed */
    atomic64_t blocks_allocated;        /* Total blocks allocated */
    atomic64_t blocks_freed;            /* Total blocks freed */
    atomic64_t inodes_allocated;        /* Total inodes allocated */
    atomic64_t inodes_freed;            /* Total inodes freed */
    atomic64_t orphans_cleaned;         /* Total orphans cleaned */
    
    /* Memory management */
    struct kmem_cache *op_cache;        /* Operation allocation cache */
    struct kmem_cache *orphan_cache;    /* Orphan entry allocation cache */
    struct kmem_cache *group_cache;     /* Group allocation cache */
    
    /* Configuration */
    u32 journal_flags;                  /* Journaling flags */
    u32 sync_mode;                      /* Synchronization mode */
    u32 batch_timeout;                  /* Batch timeout in ms */
    u32 orphan_cleanup_interval;        /* Orphan cleanup interval */
    
    /* Statistics */
    atomic64_t allocation_requests;     /* Total allocation requests */
    atomic64_t allocation_failures;     /* Allocation failures */
    atomic64_t fragmentation_score;     /* Overall fragmentation score */
    atomic64_t consistency_checks;      /* Consistency checks performed */
    atomic64_t consistency_errors;      /* Consistency errors found */
    
    /* Error handling */
    atomic_t error_count;               /* Total error count */
    struct list_head error_log;         /* Error log entries */
    
    /* Synchronization */
    struct rw_semaphore manager_rwsem;  /* Manager read-write semaphore */
    spinlock_t stats_lock;              /* Statistics spinlock */
};

/* Function declarations */

/* Manager initialization and cleanup */
struct vexfs_allocation_journal_manager *vexfs_allocation_journal_init(
    struct vexfs_journal *journal,
    struct vexfs_atomic_manager *atomic_mgr,
    struct vexfs_metadata_journal_manager *meta_mgr);
void vexfs_allocation_journal_destroy(struct vexfs_allocation_journal_manager *mgr);

/* Allocation group management */
struct vexfs_allocation_group *vexfs_allocation_group_create(
    struct vexfs_allocation_journal_manager *mgr,
    u32 group_id, u64 start_block, u32 block_count, u32 inode_count);
void vexfs_allocation_group_destroy(struct vexfs_allocation_group *group);
int vexfs_allocation_group_init_bitmaps(struct vexfs_allocation_group *group);

/* Kernel bitmap operations */
struct vexfs_kernel_bitmap *vexfs_kernel_bitmap_create(u32 size_bits);
void vexfs_kernel_bitmap_destroy(struct vexfs_kernel_bitmap *bitmap);
int vexfs_kernel_bitmap_set(struct vexfs_kernel_bitmap *bitmap, u32 bit);
int vexfs_kernel_bitmap_clear(struct vexfs_kernel_bitmap *bitmap, u32 bit);
int vexfs_kernel_bitmap_test(struct vexfs_kernel_bitmap *bitmap, u32 bit);
int vexfs_kernel_bitmap_find_first_zero(struct vexfs_kernel_bitmap *bitmap, u32 start);
int vexfs_kernel_bitmap_find_next_zero_area(struct vexfs_kernel_bitmap *bitmap,
                                           u32 start, u32 count, u32 align);
u32 vexfs_kernel_bitmap_weight(struct vexfs_kernel_bitmap *bitmap);
u32 vexfs_kernel_bitmap_checksum(struct vexfs_kernel_bitmap *bitmap);

/* Block allocation journaling */
int vexfs_allocation_journal_block_alloc(struct vexfs_allocation_journal_manager *mgr,
                                         u32 group_id, u32 count, u32 alignment,
                                         u64 *allocated_blocks, u32 flags);
int vexfs_allocation_journal_block_free(struct vexfs_allocation_journal_manager *mgr,
                                        u32 group_id, u64 start_block,
                                        u32 count, u32 flags);

/* Inode allocation journaling */
int vexfs_allocation_journal_inode_alloc(struct vexfs_allocation_journal_manager *mgr,
                                         u32 group_id, u64 *allocated_inode, u32 flags);
int vexfs_allocation_journal_inode_free(struct vexfs_allocation_journal_manager *mgr,
                                        u32 group_id, u64 inode_number, u32 flags);

/* Vector-specific allocation */
int vexfs_allocation_journal_vector_alloc(struct vexfs_allocation_journal_manager *mgr,
                                          u32 dimensions, u32 element_type,
                                          u32 vector_count, u64 *allocated_blocks,
                                          u32 *block_count, u32 flags);

/* Atomic bitmap updates */
int vexfs_allocation_atomic_bitmap_update(struct vexfs_allocation_journal_manager *mgr,
                                          struct vexfs_allocation_group *group,
                                          struct vexfs_allocation_operation *op);

/* Orphan detection and resolution */
int vexfs_allocation_detect_orphans(struct vexfs_allocation_journal_manager *mgr,
                                   u32 group_id);
int vexfs_allocation_cleanup_orphan(struct vexfs_allocation_journal_manager *mgr,
                                   struct vexfs_orphan_entry *orphan);
int vexfs_allocation_resolve_orphans(struct vexfs_allocation_journal_manager *mgr);

/* Background consistency checking */
int vexfs_allocation_consistency_check(struct vexfs_allocation_journal_manager *mgr,
                                      u32 group_id);
int vexfs_allocation_full_consistency_check(struct vexfs_allocation_journal_manager *mgr);

/* Fragmentation optimization */
int vexfs_allocation_optimize_fragmentation(struct vexfs_allocation_journal_manager *mgr,
                                           u32 group_id);
u32 vexfs_allocation_calculate_fragmentation(struct vexfs_allocation_group *group);
int vexfs_allocation_defragment_group(struct vexfs_allocation_journal_manager *mgr,
                                     u32 group_id);

/* Allocation strategy management */
int vexfs_allocation_set_strategy(struct vexfs_allocation_journal_manager *mgr,
                                 u32 group_id, u32 strategy);
int vexfs_allocation_optimize_for_vectors(struct vexfs_allocation_journal_manager *mgr,
                                         u32 group_id, u32 dimensions);

/* Batch processing */
int vexfs_allocation_journal_batch_commit(struct vexfs_allocation_journal_manager *mgr);
int vexfs_allocation_journal_force_sync(struct vexfs_allocation_journal_manager *mgr);

/* Recovery operations */
int vexfs_allocation_journal_recover(struct vexfs_allocation_journal_manager *mgr);
int vexfs_allocation_journal_replay_operations(struct vexfs_allocation_journal_manager *mgr,
                                              u64 start_seq, u64 end_seq);

/* Statistics and monitoring */
void vexfs_allocation_journal_get_stats(struct vexfs_allocation_journal_manager *mgr,
                                       struct vexfs_allocation_journal_stats *stats);

/* Allocation journaling statistics */
struct vexfs_allocation_journal_stats {
    u64 total_operations;
    u64 block_allocations;
    u64 block_frees;
    u64 inode_allocations;
    u64 inode_frees;
    u64 vector_allocations;
    u64 orphans_detected;
    u64 orphans_cleaned;
    u64 consistency_checks;
    u64 consistency_errors;
    u32 active_groups;
    u32 fragmentation_score;
    u32 pending_operations;
    u32 cached_bitmaps;
    u64 bytes_allocated;
    u64 bytes_freed;
    unsigned long last_consistency_check;
    unsigned long last_orphan_cleanup;
};

/* Utility macros */
#define VEXFS_ALLOC_OP_ID(op) ((op) ? (op)->op_id : 0)
#define VEXFS_ALLOC_GROUP_BLOCKS_FREE(group) \
    (atomic_read(&(group)->free_blocks))
#define VEXFS_ALLOC_GROUP_INODES_FREE(group) \
    (atomic_read(&(group)->free_inodes))
#define VEXFS_ALLOC_IS_VECTOR_OPTIMIZED(group) \
    ((group)->flags & VEXFS_ALLOC_GROUP_VECTOR_OPT)

/* Error codes specific to allocation journaling */
#define VEXFS_ALLOC_ERR_NO_SPACE        -3001
#define VEXFS_ALLOC_ERR_INVALID_GROUP   -3002
#define VEXFS_ALLOC_ERR_BITMAP_CORRUPT  -3003
#define VEXFS_ALLOC_ERR_ORPHAN_LIMIT    -3004
#define VEXFS_ALLOC_ERR_FRAGMENTATION   -3005
#define VEXFS_ALLOC_ERR_ALIGNMENT       -3006

#endif /* _VEXFS_V2_ALLOCATION_JOURNAL_H */