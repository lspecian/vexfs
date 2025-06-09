/*
 * VexFS v2.0 - Cross-Layer Consistency Mechanisms Header (Task 14)
 * 
 * This header defines the structures and interfaces for the Cross-Layer Consistency
 * Mechanisms that ensure our three-layer AI-Native Semantic Substrate operates as
 * a unified, consistent system.
 */

#ifndef _VEXFS_V2_CROSS_LAYER_CONSISTENCY_H
#define _VEXFS_V2_CROSS_LAYER_CONSISTENCY_H

#include <linux/types.h>
#include <linux/list.h>
#include <linux/rbtree.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/rwsem.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/time.h>

/* Forward declarations */
struct vexfs_journal;
struct vexfs_graph_manager;
struct vexfs_semantic_journal_manager;
struct vexfs_journal_transaction;

/* Cross-layer transaction isolation levels */
#define VEXFS_CROSS_ISOLATION_READ_UNCOMMITTED  0
#define VEXFS_CROSS_ISOLATION_READ_COMMITTED     1
#define VEXFS_CROSS_ISOLATION_REPEATABLE_READ    2
#define VEXFS_CROSS_ISOLATION_SERIALIZABLE       3
#define VEXFS_CROSS_ISOLATION_SNAPSHOT           4

/* Cross-layer operation priorities */
#define VEXFS_CROSS_PRIORITY_CRITICAL    1
#define VEXFS_CROSS_PRIORITY_HIGH        2
#define VEXFS_CROSS_PRIORITY_NORMAL      3
#define VEXFS_CROSS_PRIORITY_LOW         4
#define VEXFS_CROSS_PRIORITY_BACKGROUND  5

/* Cross-layer consistency flags */
#define VEXFS_CROSS_FLAG_ATOMIC          0x01
#define VEXFS_CROSS_FLAG_DURABLE         0x02
#define VEXFS_CROSS_FLAG_CONSISTENT      0x04
#define VEXFS_CROSS_FLAG_ISOLATED        0x08
#define VEXFS_CROSS_FLAG_ORDERED         0x10
#define VEXFS_CROSS_FLAG_RECOVERABLE     0x20

/* Maximum sizes */
#define VEXFS_CROSS_MAX_OPERATION_DATA   4096
#define VEXFS_CROSS_INLINE_DATA_SIZE     256

/**
 * Cross-layer operation structure
 */
struct vexfs_cross_layer_operation {
    u64 operation_id;                           /* Unique operation ID */
    u32 layer_mask;                             /* Which layers this operation affects */
    u32 operation_type;                         /* Type of operation */
    ktime_t timestamp;                          /* When operation was created */
    size_t data_size;                           /* Size of operation data */
    void *data;                                 /* Operation data pointer */
    u8 inline_data[VEXFS_CROSS_INLINE_DATA_SIZE]; /* Inline data storage */
    u32 flags;                                  /* Operation flags */
    u32 priority;                               /* Operation priority */
    int result;                                 /* Operation result */
    struct list_head list;                      /* List linkage */
};

/**
 * Cross-layer transaction structure
 */
struct vexfs_cross_layer_transaction {
    u64 transaction_id;                         /* Unique transaction ID */
    u32 state;                                  /* Current transaction state */
    u32 operation_mask;                         /* Which layers are involved */
    u32 isolation_level;                        /* Transaction isolation level */
    u32 timeout_ms;                             /* Transaction timeout */
    
    /* Timing information */
    unsigned long start_time;                   /* Transaction start time */
    unsigned long prepare_time;                 /* Prepare phase start time */
    unsigned long commit_time;                  /* Commit phase start time */
    unsigned long end_time;                     /* Transaction end time */
    
    /* Layer-specific transactions */
    struct vexfs_journal_transaction *fs_transaction;     /* Filesystem transaction */
    void *graph_transaction;                    /* Graph transaction (opaque) */
    void *semantic_transaction;                 /* Semantic transaction (opaque) */
    
    /* Operation lists */
    struct list_head fs_operations;             /* Filesystem operations */
    struct list_head graph_operations;          /* Graph operations */
    struct list_head semantic_operations;       /* Semantic operations */
    
    /* Operation counts */
    u32 fs_operation_count;                     /* Number of FS operations */
    u32 graph_operation_count;                  /* Number of graph operations */
    u32 semantic_operation_count;               /* Number of semantic operations */
    u32 total_operations;                       /* Total operation count */
    
    /* Synchronization */
    struct completion completion;               /* Transaction completion */
    atomic_t ref_count;                         /* Reference count */
    spinlock_t lock;                            /* Transaction lock */
    
    /* Error handling */
    int error_code;                             /* Error code if failed */
    char error_message[256];                    /* Error message */
    
    /* Deadlock detection */
    struct list_head deadlock_list;             /* Deadlock detection list */
    u64 deadlock_detection_id;                  /* Deadlock detection ID */
    
    /* Manager reference */
    struct vexfs_cross_layer_manager *mgr;      /* Manager reference */
    struct list_head list;                      /* Manager's transaction list */
};

/**
 * Cross-layer consistency manager
 */
struct vexfs_cross_layer_manager {
    /* Core references */
    struct super_block *sb;                     /* Superblock */
    struct vexfs_journal *journal;              /* FS journal */
    struct vexfs_graph_manager *graph_mgr;      /* Graph manager */
    struct vexfs_semantic_journal_manager *semantic_mgr; /* Semantic manager */
    
    /* Transaction management */
    atomic64_t next_transaction_id;             /* Next transaction ID */
    atomic_t active_transactions;               /* Active transaction count */
    atomic_t pending_commits;                   /* Pending commit count */
    atomic_t pending_aborts;                    /* Pending abort count */
    
    /* Transaction tracking */
    struct rb_root active_transactions_tree;    /* Active transactions tree */
    struct rb_root deadlock_detection_tree;     /* Deadlock detection tree */
    struct list_head pending_transactions;      /* Pending transactions list */
    struct list_head commit_queue;              /* Commit queue */
    struct list_head abort_queue;               /* Abort queue */
    
    /* Synchronization */
    struct rw_semaphore manager_lock;           /* Manager-wide lock */
    spinlock_t transaction_lock;                /* Transaction list lock */
    spinlock_t commit_lock;                     /* Commit queue lock */
    spinlock_t deadlock_lock;                   /* Deadlock detection lock */
    struct mutex consistency_mutex;             /* Consistency check mutex */
    struct mutex recovery_mutex;                /* Recovery mutex */
    
    /* Memory management */
    struct kmem_cache *transaction_cache;       /* Transaction cache */
    struct kmem_cache *operation_cache;         /* Operation cache */
    
    /* Background work */
    struct workqueue_struct *workqueue;        /* Work queue */
    struct delayed_work consistency_work;       /* Consistency check work */
    struct delayed_work deadlock_work;          /* Deadlock detection work */
    struct delayed_work recovery_work;          /* Recovery work */
    
    /* Performance monitoring */
    atomic64_t total_transactions;              /* Total transactions */
    atomic64_t successful_commits;              /* Successful commits */
    atomic64_t failed_commits;                  /* Failed commits */
    atomic64_t aborted_transactions;            /* Aborted transactions */
    atomic64_t deadlocks_detected;              /* Deadlocks detected */
    atomic64_t deadlocks_resolved;              /* Deadlocks resolved */
    atomic64_t consistency_checks;              /* Consistency checks */
    atomic64_t consistency_violations;          /* Consistency violations */
    atomic64_t recovery_operations;             /* Recovery operations */
    
    /* Error tracking */
    atomic64_t fs_layer_errors;                 /* FS layer errors */
    atomic64_t graph_layer_errors;              /* Graph layer errors */
    atomic64_t semantic_layer_errors;           /* Semantic layer errors */
    atomic64_t cross_layer_errors;              /* Cross-layer errors */
    
    /* Configuration */
    u32 flags;                                  /* Manager flags */
    u32 consistency_check_interval_ms;          /* Consistency check interval */
    u32 deadlock_check_interval_ms;             /* Deadlock check interval */
    u32 recovery_check_interval_ms;             /* Recovery check interval */
    u32 transaction_timeout_ms;                 /* Default transaction timeout */
    u32 max_concurrent_transactions;            /* Max concurrent transactions */
};

/**
 * Cross-layer consistency statistics
 */
struct vexfs_cross_layer_stats {
    u64 total_transactions;                     /* Total transactions */
    u64 successful_commits;                     /* Successful commits */
    u64 failed_commits;                         /* Failed commits */
    u64 aborted_transactions;                   /* Aborted transactions */
    u64 active_transactions;                    /* Currently active transactions */
    u64 deadlocks_detected;                     /* Deadlocks detected */
    u64 deadlocks_resolved;                     /* Deadlocks resolved */
    u64 consistency_checks;                     /* Consistency checks performed */
    u64 consistency_violations;                 /* Consistency violations found */
    u64 recovery_operations;                    /* Recovery operations performed */
    u64 fs_layer_errors;                        /* FS layer errors */
    u64 graph_layer_errors;                     /* Graph layer errors */
    u64 semantic_layer_errors;                  /* Semantic layer errors */
    u64 cross_layer_errors;                     /* Cross-layer errors */
    u64 avg_transaction_time_ms;                /* Average transaction time */
    u64 avg_commit_time_ms;                     /* Average commit time */
    u32 cache_hit_rate;                         /* Cache hit rate percentage */
    u32 deadlock_rate;                          /* Deadlock rate percentage */
};

/* Function prototypes */

/* Manager lifecycle */
struct vexfs_cross_layer_manager *vexfs_cross_layer_init(
    struct super_block *sb,
    struct vexfs_journal *journal,
    struct vexfs_graph_manager *graph_mgr,
    struct vexfs_semantic_journal_manager *semantic_mgr);

void vexfs_cross_layer_destroy(struct vexfs_cross_layer_manager *mgr);

/* Transaction management */
struct vexfs_cross_layer_transaction *vexfs_cross_layer_begin(
    struct vexfs_cross_layer_manager *mgr,
    u32 operation_mask,
    u32 isolation_level,
    u32 timeout_ms);

int vexfs_cross_layer_add_operation(struct vexfs_cross_layer_transaction *trans,
                                    u32 layer_mask, u32 operation_type,
                                    const void *operation_data, size_t data_size);

int vexfs_cross_layer_commit(struct vexfs_cross_layer_transaction *trans);
int vexfs_cross_layer_abort(struct vexfs_cross_layer_transaction *trans);
void vexfs_cross_layer_free(struct vexfs_cross_layer_transaction *trans);

/* Consistency operations */
int vexfs_cross_layer_check_consistency(struct vexfs_cross_layer_manager *mgr);
int vexfs_cross_layer_repair_consistency(struct vexfs_cross_layer_manager *mgr);
int vexfs_cross_layer_create_snapshot(struct vexfs_cross_layer_manager *mgr, u64 *snapshot_id);
int vexfs_cross_layer_restore_snapshot(struct vexfs_cross_layer_manager *mgr, u64 snapshot_id);

/* Deadlock management */
int vexfs_cross_layer_detect_deadlocks(struct vexfs_cross_layer_manager *mgr);
int vexfs_cross_layer_resolve_deadlock(struct vexfs_cross_layer_manager *mgr, u64 victim_transaction_id);

/* Statistics and monitoring */
void vexfs_cross_layer_get_stats(struct vexfs_cross_layer_manager *mgr,
                                  struct vexfs_cross_layer_stats *stats);
int vexfs_cross_layer_reset_stats(struct vexfs_cross_layer_manager *mgr);

/* Recovery operations */
int vexfs_cross_layer_recover_from_failure(struct vexfs_cross_layer_manager *mgr);
int vexfs_cross_layer_validate_integrity(struct vexfs_cross_layer_manager *mgr);

/* Configuration */
int vexfs_cross_layer_set_config(struct vexfs_cross_layer_manager *mgr,
                                  const char *key, const char *value);
int vexfs_cross_layer_get_config(struct vexfs_cross_layer_manager *mgr,
                                  const char *key, char *value, size_t value_size);

#endif /* _VEXFS_V2_CROSS_LAYER_CONSISTENCY_H */