/*
 * VexFS v2.0 - Fast Crash Recovery Mechanism (Task 7)
 * 
 * This implements fast crash recovery for VexFS as part of the AI-Native Semantic
 * Substrate roadmap (Phase 1). Builds on the complete foundation from Tasks 1-6 to
 * provide enterprise-grade recovery capabilities with minimal downtime.
 *
 * Key Features:
 * - Checkpoint mechanism to reduce recovery scope
 * - Efficient journal replay with parallel processing
 * - Memory-mapped I/O for faster journal reading during recovery
 * - Partial transaction detection and resolution
 * - Optimized recovery order to minimize dependencies
 * - Progress tracking mechanism for recovery process
 * - Kernel-compatible parallelism for multi-core recovery
 * - Integration with complete Phase 1 journaling infrastructure
 */

#ifndef _VEXFS_V2_FAST_RECOVERY_H
#define _VEXFS_V2_FAST_RECOVERY_H

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
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/kthread.h>
#include <linux/percpu.h>
#include <linux/timer.h>
#include <linux/jiffies.h>

#include "vexfs_v2_journal.h"
#include "vexfs_v2_atomic.h"
#include "vexfs_v2_metadata_journal.h"
#include "vexfs_v2_allocation_journal.h"

/* Fast recovery operation types */
#define VEXFS_RECOVERY_OP_CHECKPOINT        0x01
#define VEXFS_RECOVERY_OP_JOURNAL_REPLAY    0x02
#define VEXFS_RECOVERY_OP_PARTIAL_CLEANUP   0x03
#define VEXFS_RECOVERY_OP_DEPENDENCY_RESOLVE 0x04
#define VEXFS_RECOVERY_OP_PROGRESS_UPDATE   0x05
#define VEXFS_RECOVERY_OP_PARALLEL_REPLAY   0x06
#define VEXFS_RECOVERY_OP_MMAP_SCAN         0x07
#define VEXFS_RECOVERY_OP_CONSISTENCY_CHECK 0x08

/* Recovery flags */
#define VEXFS_RECOVERY_FLAG_PARALLEL        0x01
#define VEXFS_RECOVERY_FLAG_MMAP_IO         0x02
#define VEXFS_RECOVERY_FLAG_CHECKPOINT      0x04
#define VEXFS_RECOVERY_FLAG_PROGRESS        0x08
#define VEXFS_RECOVERY_FLAG_DEPENDENCY_OPT  0x10
#define VEXFS_RECOVERY_FLAG_BACKGROUND      0x20
#define VEXFS_RECOVERY_FLAG_FORCE_SYNC      0x40
#define VEXFS_RECOVERY_FLAG_VERIFY          0x80

/* Recovery states */
#define VEXFS_RECOVERY_STATE_IDLE           0x00
#define VEXFS_RECOVERY_STATE_INITIALIZING   0x01
#define VEXFS_RECOVERY_STATE_SCANNING       0x02
#define VEXFS_RECOVERY_STATE_REPLAYING      0x03
#define VEXFS_RECOVERY_STATE_RESOLVING      0x04
#define VEXFS_RECOVERY_STATE_FINALIZING     0x05
#define VEXFS_RECOVERY_STATE_COMPLETE       0x06
#define VEXFS_RECOVERY_STATE_ERROR          0x07

/* Checkpoint types */
#define VEXFS_CHECKPOINT_TYPE_FULL          0x01
#define VEXFS_CHECKPOINT_TYPE_INCREMENTAL   0x02
#define VEXFS_CHECKPOINT_TYPE_METADATA_ONLY 0x03
#define VEXFS_CHECKPOINT_TYPE_EMERGENCY     0x04

/* Parallel recovery worker types */
#define VEXFS_RECOVERY_WORKER_JOURNAL       0x01
#define VEXFS_RECOVERY_WORKER_METADATA      0x02
#define VEXFS_RECOVERY_WORKER_ALLOCATION    0x03
#define VEXFS_RECOVERY_WORKER_DEPENDENCY    0x04

/* Maximum values for fast recovery */
#define VEXFS_RECOVERY_MAX_WORKERS          16
#define VEXFS_RECOVERY_MAX_CHECKPOINTS      64
#define VEXFS_RECOVERY_MAX_PARTIAL_TRANS    1024
#define VEXFS_RECOVERY_MAX_DEPENDENCIES     4096
#define VEXFS_RECOVERY_MMAP_CHUNK_SIZE      (64 * 1024 * 1024)  /* 64MB */
#define VEXFS_RECOVERY_PROGRESS_INTERVAL    1000  /* Progress updates every 1000 ops */

/*
 * Checkpoint descriptor for fast recovery
 */
struct vexfs_checkpoint {
    u32 checkpoint_id;                  /* Unique checkpoint ID */
    u32 checkpoint_type;                /* Type of checkpoint */
    u64 sequence_number;                /* Journal sequence at checkpoint */
    u64 timestamp;                      /* Checkpoint creation time */
    
    /* Checkpoint scope */
    u64 journal_start_seq;              /* Journal start sequence */
    u64 journal_end_seq;                /* Journal end sequence */
    u64 metadata_seq;                   /* Metadata journal sequence */
    u64 allocation_seq;                 /* Allocation journal sequence */
    
    /* Checkpoint data location */
    u64 checkpoint_block;               /* Block containing checkpoint data */
    u32 checkpoint_size;                /* Size of checkpoint data */
    u32 compressed_size;                /* Compressed size if applicable */
    
    /* Integrity verification */
    u32 checksum;                       /* Checkpoint checksum */
    u32 metadata_checksum;              /* Metadata checksum */
    u32 allocation_checksum;            /* Allocation checksum */
    
    /* Performance metrics */
    u32 creation_time_ms;               /* Time to create checkpoint */
    u32 compression_ratio;              /* Compression ratio (0-100) */
    
    /* Flags and state */
    u32 flags;                          /* Checkpoint flags */
    atomic_t ref_count;                 /* Reference count */
    
    /* List management */
    struct list_head checkpoint_list;   /* List of checkpoints */
    struct rb_node checkpoint_node;     /* Red-black tree node */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Memory-mapped journal region for fast I/O
 */
struct vexfs_mmap_journal_region {
    void *mapped_addr;                  /* Mapped address */
    u64 physical_start;                 /* Physical start address */
    size_t mapped_size;                 /* Size of mapped region */
    u64 journal_start_seq;              /* Journal start sequence in region */
    u64 journal_end_seq;                /* Journal end sequence in region */
    
    /* Access tracking */
    atomic_t access_count;              /* Number of accesses */
    unsigned long last_access;          /* Last access time */
    
    /* Synchronization */
    struct mutex mmap_mutex;            /* Memory mapping mutex */
    atomic_t ref_count;                 /* Reference count */
    
    /* List management */
    struct list_head mmap_list;         /* List of mapped regions */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Partial transaction descriptor for cleanup
 */
struct vexfs_partial_transaction {
    u64 transaction_id;                 /* Transaction ID */
    u32 transaction_type;               /* Type of transaction */
    u64 start_sequence;                 /* Start sequence number */
    u64 end_sequence;                   /* End sequence number (if complete) */
    
    /* Transaction state */
    u32 state;                          /* Current state */
    u32 operation_count;                /* Number of operations */
    u32 completed_operations;           /* Completed operations */
    
    /* Recovery information */
    void *recovery_data;                /* Recovery data */
    size_t recovery_size;               /* Size of recovery data */
    u32 recovery_method;                /* Recovery method to use */
    
    /* Dependencies */
    struct list_head dependencies;      /* List of dependencies */
    atomic_t dependency_count;          /* Number of dependencies */
    
    /* Timing */
    unsigned long detection_time;       /* When partial transaction was detected */
    unsigned long timeout;              /* Transaction timeout */
    
    /* List management */
    struct list_head partial_list;      /* List of partial transactions */
    struct rb_node partial_node;        /* Red-black tree node */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Recovery dependency descriptor
 */
struct vexfs_recovery_dependency {
    u64 dependent_seq;                  /* Dependent sequence number */
    u64 prerequisite_seq;               /* Prerequisite sequence number */
    u32 dependency_type;                /* Type of dependency */
    u32 priority;                       /* Dependency priority */
    
    /* Resolution state */
    atomic_t resolved;                  /* Whether dependency is resolved */
    u32 resolution_method;              /* How dependency was resolved */
    
    /* List management */
    struct list_head dep_list;          /* List of dependencies */
    struct rb_node dep_node;            /* Red-black tree node */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Recovery worker descriptor for parallel processing
 */
struct vexfs_recovery_worker {
    u32 worker_id;                      /* Worker ID */
    u32 worker_type;                    /* Type of worker */
    struct task_struct *worker_thread;  /* Worker thread */
    
    /* Work assignment */
    u64 start_sequence;                 /* Start sequence for this worker */
    u64 end_sequence;                   /* End sequence for this worker */
    u32 operation_count;                /* Number of operations assigned */
    
    /* Progress tracking */
    atomic_t operations_completed;      /* Operations completed */
    atomic_t operations_failed;         /* Operations failed */
    unsigned long start_time;           /* Worker start time */
    unsigned long last_progress_time;   /* Last progress update */
    
    /* Worker state */
    atomic_t worker_state;              /* Current worker state */
    int worker_result;                  /* Worker result code */
    
    /* Synchronization */
    struct completion worker_completion; /* Worker completion */
    struct mutex worker_mutex;          /* Worker mutex */
    
    /* Statistics */
    u64 bytes_processed;                /* Bytes processed by worker */
    u32 cache_hits;                     /* Cache hits */
    u32 cache_misses;                   /* Cache misses */
    
    /* List management */
    struct list_head worker_list;       /* List of workers */
    struct rcu_head rcu;                /* RCU callback head */
};

/*
 * Recovery progress tracker
 */
struct vexfs_recovery_progress {
    /* Overall progress */
    atomic64_t total_operations;        /* Total operations to recover */
    atomic64_t completed_operations;    /* Completed operations */
    atomic64_t failed_operations;       /* Failed operations */
    
    /* Phase progress */
    atomic_t current_phase;             /* Current recovery phase */
    atomic64_t phase_operations;        /* Operations in current phase */
    atomic64_t phase_completed;         /* Completed in current phase */
    
    /* Timing information */
    unsigned long recovery_start_time;  /* Recovery start time */
    unsigned long phase_start_time;     /* Current phase start time */
    unsigned long last_update_time;     /* Last progress update */
    
    /* Performance metrics */
    atomic64_t bytes_recovered;         /* Total bytes recovered */
    atomic_t recovery_rate;             /* Operations per second */
    atomic_t estimated_time_remaining;  /* Estimated time remaining (seconds) */
    
    /* Error tracking */
    atomic_t error_count;               /* Total errors encountered */
    atomic_t warning_count;             /* Total warnings */
    
    /* Worker progress */
    atomic_t active_workers;            /* Number of active workers */
    atomic64_t worker_operations[VEXFS_RECOVERY_MAX_WORKERS]; /* Per-worker progress */
};

/*
 * Fast recovery manager
 */
struct vexfs_fast_recovery_manager {
    /* Core infrastructure integration */
    struct vexfs_journal *journal;      /* Associated journal */
    struct vexfs_atomic_manager *atomic_mgr; /* Atomic operations manager */
    struct vexfs_metadata_journal_manager *meta_mgr; /* Metadata journal manager */
    struct vexfs_allocation_journal_manager *alloc_mgr; /* Allocation journal manager */
    
    /* Checkpoint management */
    struct list_head checkpoints;       /* List of checkpoints */
    struct rb_root checkpoint_tree;     /* Checkpoint tree for fast lookup */
    struct mutex checkpoint_mutex;      /* Checkpoint mutex */
    atomic_t checkpoint_count;          /* Number of checkpoints */
    u32 max_checkpoints;                /* Maximum checkpoints */
    u32 next_checkpoint_id;             /* Next checkpoint ID */
    
    /* Memory-mapped I/O management */
    struct list_head mmap_regions;      /* List of mapped regions */
    struct mutex mmap_mutex;            /* Memory mapping mutex */
    atomic_t mmap_region_count;         /* Number of mapped regions */
    size_t total_mapped_size;           /* Total mapped size */
    
    /* Partial transaction tracking */
    struct list_head partial_transactions; /* List of partial transactions */
    struct rb_root partial_tree;        /* Partial transaction tree */
    struct mutex partial_mutex;         /* Partial transaction mutex */
    atomic_t partial_count;             /* Number of partial transactions */
    
    /* Dependency management */
    struct list_head dependencies;      /* List of dependencies */
    struct rb_root dependency_tree;     /* Dependency tree */
    struct mutex dependency_mutex;      /* Dependency mutex */
    atomic_t dependency_count;          /* Number of dependencies */
    
    /* Parallel recovery workers */
    struct list_head workers;           /* List of recovery workers */
    struct mutex worker_mutex;          /* Worker mutex */
    atomic_t active_workers;            /* Number of active workers */
    u32 max_workers;                    /* Maximum workers */
    
    /* Progress tracking */
    struct vexfs_recovery_progress progress; /* Recovery progress */
    struct workqueue_struct *progress_workqueue; /* Progress workqueue */
    struct delayed_work progress_work;  /* Progress update work */
    
    /* Recovery state */
    atomic_t recovery_state;            /* Current recovery state */
    atomic_t recovery_flags;            /* Recovery flags */
    unsigned long recovery_start_time;  /* Recovery start time */
    unsigned long recovery_end_time;    /* Recovery end time */
    
    /* Configuration */
    u32 checkpoint_interval;            /* Checkpoint interval (seconds) */
    u32 parallel_threshold;             /* Threshold for parallel recovery */
    u32 mmap_threshold;                 /* Threshold for memory mapping */
    u32 progress_interval;              /* Progress update interval (ms) */
    
    /* Performance optimization */
    atomic64_t total_recoveries;        /* Total recoveries performed */
    atomic64_t total_recovery_time;     /* Total recovery time (ms) */
    atomic64_t fastest_recovery;        /* Fastest recovery time (ms) */
    atomic64_t slowest_recovery;        /* Slowest recovery time (ms) */
    
    /* Memory management */
    struct kmem_cache *checkpoint_cache; /* Checkpoint allocation cache */
    struct kmem_cache *mmap_cache;      /* Memory mapping cache */
    struct kmem_cache *partial_cache;   /* Partial transaction cache */
    struct kmem_cache *dependency_cache; /* Dependency cache */
    struct kmem_cache *worker_cache;    /* Worker cache */
    
    /* Statistics */
    atomic64_t checkpoints_created;     /* Total checkpoints created */
    atomic64_t journal_entries_replayed; /* Journal entries replayed */
    atomic64_t partial_transactions_resolved; /* Partial transactions resolved */
    atomic64_t dependencies_resolved;   /* Dependencies resolved */
    atomic64_t mmap_operations;         /* Memory mapping operations */
    
    /* Error handling */
    atomic_t error_count;               /* Total error count */
    struct list_head error_log;         /* Error log entries */
    
    /* Synchronization */
    struct rw_semaphore manager_rwsem;  /* Manager read-write semaphore */
    spinlock_t stats_lock;              /* Statistics spinlock */
    struct completion recovery_completion; /* Recovery completion */
};

/* Function declarations */

/* Manager initialization and cleanup */
struct vexfs_fast_recovery_manager *vexfs_fast_recovery_init(
    struct vexfs_journal *journal,
    struct vexfs_atomic_manager *atomic_mgr,
    struct vexfs_metadata_journal_manager *meta_mgr,
    struct vexfs_allocation_journal_manager *alloc_mgr);
void vexfs_fast_recovery_destroy(struct vexfs_fast_recovery_manager *mgr);

/* Checkpoint management */
int vexfs_fast_recovery_create_checkpoint(struct vexfs_fast_recovery_manager *mgr,
                                         u32 checkpoint_type, u32 flags);
struct vexfs_checkpoint *vexfs_fast_recovery_find_latest_checkpoint(
    struct vexfs_fast_recovery_manager *mgr);
int vexfs_fast_recovery_load_checkpoint(struct vexfs_fast_recovery_manager *mgr,
                                       struct vexfs_checkpoint *checkpoint);
int vexfs_fast_recovery_cleanup_old_checkpoints(struct vexfs_fast_recovery_manager *mgr,
                                               u32 keep_count);

/* Memory-mapped journal I/O */
struct vexfs_mmap_journal_region *vexfs_fast_recovery_mmap_journal(
    struct vexfs_fast_recovery_manager *mgr,
    u64 start_seq, u64 end_seq);
void vexfs_fast_recovery_munmap_journal(struct vexfs_mmap_journal_region *region);
int vexfs_fast_recovery_scan_mmap_journal(struct vexfs_fast_recovery_manager *mgr,
                                         struct vexfs_mmap_journal_region *region,
                                         u64 start_seq, u64 end_seq);

/* Parallel recovery workers */
int vexfs_fast_recovery_create_workers(struct vexfs_fast_recovery_manager *mgr,
                                      u32 worker_count, u32 worker_type);
int vexfs_fast_recovery_assign_work(struct vexfs_fast_recovery_manager *mgr,
                                   u64 start_seq, u64 end_seq);
int vexfs_fast_recovery_wait_workers(struct vexfs_fast_recovery_manager *mgr);
void vexfs_fast_recovery_cleanup_workers(struct vexfs_fast_recovery_manager *mgr);

/* Journal replay operations */
int vexfs_fast_recovery_replay_journal(struct vexfs_fast_recovery_manager *mgr,
                                      u64 start_seq, u64 end_seq, u32 flags);
int vexfs_fast_recovery_parallel_replay(struct vexfs_fast_recovery_manager *mgr,
                                       u64 start_seq, u64 end_seq, u32 worker_count);
int vexfs_fast_recovery_replay_single_entry(struct vexfs_fast_recovery_manager *mgr,
                                           void *journal_entry, u64 sequence);

/* Partial transaction detection and resolution */
int vexfs_fast_recovery_detect_partial_transactions(struct vexfs_fast_recovery_manager *mgr,
                                                   u64 start_seq, u64 end_seq);
int vexfs_fast_recovery_resolve_partial_transaction(struct vexfs_fast_recovery_manager *mgr,
                                                   struct vexfs_partial_transaction *partial);
int vexfs_fast_recovery_cleanup_partial_transactions(struct vexfs_fast_recovery_manager *mgr);

/* Dependency resolution */
int vexfs_fast_recovery_analyze_dependencies(struct vexfs_fast_recovery_manager *mgr,
                                            u64 start_seq, u64 end_seq);
int vexfs_fast_recovery_resolve_dependencies(struct vexfs_fast_recovery_manager *mgr);
int vexfs_fast_recovery_optimize_recovery_order(struct vexfs_fast_recovery_manager *mgr,
                                               u64 *sequence_order, u32 count);

/* Progress tracking */
int vexfs_fast_recovery_init_progress(struct vexfs_fast_recovery_manager *mgr,
                                     u64 total_operations);
int vexfs_fast_recovery_update_progress(struct vexfs_fast_recovery_manager *mgr,
                                       u64 completed_operations, u32 phase);
int vexfs_fast_recovery_get_progress(struct vexfs_fast_recovery_manager *mgr,
                                    struct vexfs_recovery_progress *progress);

/* Main recovery operations */
int vexfs_fast_recovery_start(struct vexfs_fast_recovery_manager *mgr, u32 flags);
int vexfs_fast_recovery_full_recovery(struct vexfs_fast_recovery_manager *mgr, u32 flags);
int vexfs_fast_recovery_incremental_recovery(struct vexfs_fast_recovery_manager *mgr,
                                            u64 from_checkpoint, u32 flags);
int vexfs_fast_recovery_emergency_recovery(struct vexfs_fast_recovery_manager *mgr, u32 flags);

/* Performance optimization */
int vexfs_fast_recovery_optimize_for_size(struct vexfs_fast_recovery_manager *mgr,
                                         u64 journal_size);
int vexfs_fast_recovery_optimize_for_speed(struct vexfs_fast_recovery_manager *mgr,
                                          u32 available_cores);
int vexfs_fast_recovery_adaptive_optimization(struct vexfs_fast_recovery_manager *mgr);

/* Statistics and monitoring */
void vexfs_fast_recovery_get_stats(struct vexfs_fast_recovery_manager *mgr,
                                  struct vexfs_fast_recovery_stats *stats);

/* Fast recovery statistics */
struct vexfs_fast_recovery_stats {
    u64 total_recoveries;
    u64 total_recovery_time_ms;
    u64 average_recovery_time_ms;
    u64 fastest_recovery_ms;
    u64 slowest_recovery_ms;
    u64 checkpoints_created;
    u64 checkpoints_used;
    u64 journal_entries_replayed;
    u64 partial_transactions_resolved;
    u64 dependencies_resolved;
    u64 mmap_operations;
    u64 parallel_recoveries;
    u32 average_workers_used;
    u32 current_checkpoint_count;
    u32 current_mmap_regions;
    u64 total_bytes_recovered;
    u64 recovery_throughput_mbps;
    u32 error_count;
    u32 warning_count;
    unsigned long last_recovery_time;
    unsigned long last_checkpoint_time;
};

/* Utility macros */
#define VEXFS_RECOVERY_IS_PARALLEL(mgr) \
    (atomic_read(&(mgr)->recovery_flags) & VEXFS_RECOVERY_FLAG_PARALLEL)
#define VEXFS_RECOVERY_IS_MMAP_ENABLED(mgr) \
    (atomic_read(&(mgr)->recovery_flags) & VEXFS_RECOVERY_FLAG_MMAP_IO)
#define VEXFS_RECOVERY_PROGRESS_PERCENT(mgr) \
    ((atomic64_read(&(mgr)->progress.completed_operations) * 100) / \
     max(atomic64_read(&(mgr)->progress.total_operations), 1ULL))

/* Error codes specific to fast recovery */
#define VEXFS_RECOVERY_ERR_NO_CHECKPOINT    -7001
#define VEXFS_RECOVERY_ERR_MMAP_FAILED      -7002
#define VEXFS_RECOVERY_ERR_WORKER_FAILED    -7003
#define VEXFS_RECOVERY_ERR_PARTIAL_UNRESOLVED -7004
#define VEXFS_RECOVERY_ERR_DEPENDENCY_CYCLE -7005
#define VEXFS_RECOVERY_ERR_PROGRESS_TIMEOUT -7006
#define VEXFS_RECOVERY_ERR_INVALID_STATE    -7007
#define VEXFS_RECOVERY_ERR_RESOURCE_LIMIT   -7008

#endif /* _VEXFS_V2_FAST_RECOVERY_H */