/*
 * VexFS v2.0 - Atomic Operations for FS Journal (Task 2)
 * 
 * This implements atomic filesystem operations leveraging the Full FS Journal
 * from Task 1. Provides transaction management, atomic wrappers for VFS operations,
 * lock-free data structures, and comprehensive rollback mechanisms.
 *
 * Key Features:
 * - Transaction begin/commit/abort mechanisms
 * - Atomic wrappers for all critical filesystem operations
 * - Lock-free data structures using kernel atomic operations
 * - Rollback mechanism for aborted transactions
 * - Nested transaction support
 * - Performance optimization through batching
 * - Crash recovery for partial writes
 */

#ifndef _VEXFS_V2_ATOMIC_H
#define _VEXFS_V2_ATOMIC_H

#include <linux/types.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/percpu.h>
#include <linux/seqlock.h>

#include "vexfs_v2_journal.h"

/* Atomic operation types */
#define VEXFS_ATOMIC_CREATE     0x01
#define VEXFS_ATOMIC_DELETE     0x02
#define VEXFS_ATOMIC_WRITE      0x03
#define VEXFS_ATOMIC_TRUNCATE   0x04
#define VEXFS_ATOMIC_RENAME     0x05
#define VEXFS_ATOMIC_LINK       0x06
#define VEXFS_ATOMIC_UNLINK     0x07
#define VEXFS_ATOMIC_MKDIR      0x08
#define VEXFS_ATOMIC_RMDIR      0x09
#define VEXFS_ATOMIC_SYMLINK    0x0A

/* Transaction isolation levels */
#define VEXFS_ISOLATION_READ_UNCOMMITTED    0x01
#define VEXFS_ISOLATION_READ_COMMITTED      0x02
#define VEXFS_ISOLATION_REPEATABLE_READ     0x03
#define VEXFS_ISOLATION_SERIALIZABLE        0x04

/* Transaction flags */
#define VEXFS_TRANS_NESTED          0x01
#define VEXFS_TRANS_READ_ONLY       0x02
#define VEXFS_TRANS_BATCH_COMMIT    0x04
#define VEXFS_TRANS_ASYNC_COMMIT    0x08
#define VEXFS_TRANS_FORCE_SYNC      0x10

/* Lock-free data structure types */
#define VEXFS_LOCKFREE_QUEUE        0x01
#define VEXFS_LOCKFREE_STACK        0x02
#define VEXFS_LOCKFREE_HASH         0x03
#define VEXFS_LOCKFREE_TREE         0x04

/* Maximum values */
#define VEXFS_MAX_NESTED_TRANS      16
#define VEXFS_MAX_ATOMIC_OPS        1024
#define VEXFS_MAX_ROLLBACK_ENTRIES  4096
#define VEXFS_ATOMIC_BATCH_SIZE     64

/*
 * Lock-free queue node for atomic operations
 */
struct vexfs_lockfree_node {
    atomic_t next;                  /* Next node pointer (encoded) */
    void *data;                     /* Node data */
    atomic_t ref_count;             /* Reference count */
    u64 sequence;                   /* Sequence number for ordering */
};

/*
 * Lock-free queue for atomic operation batching
 */
struct vexfs_lockfree_queue {
    atomic_t head;                  /* Head pointer (encoded) */
    atomic_t tail;                  /* Tail pointer (encoded) */
    atomic64_t enqueue_count;       /* Total enqueue operations */
    atomic64_t dequeue_count;       /* Total dequeue operations */
    u32 node_size;                  /* Size of each node */
    struct kmem_cache *node_cache;  /* Node allocation cache */
};

/*
 * Atomic operation descriptor
 */
struct vexfs_atomic_op {
    u32 op_type;                    /* Operation type */
    u32 op_flags;                   /* Operation flags */
    u64 op_id;                      /* Unique operation ID */
    
    /* Target information */
    struct inode *target_inode;     /* Target inode */
    struct dentry *target_dentry;   /* Target dentry */
    loff_t offset;                  /* File offset for writes */
    size_t length;                  /* Operation length */
    
    /* Data for operation */
    void *op_data;                  /* Operation-specific data */
    size_t data_size;               /* Size of operation data */
    
    /* Rollback information */
    void *rollback_data;            /* Data for rollback */
    size_t rollback_size;           /* Size of rollback data */
    
    /* Completion tracking */
    struct completion op_completion; /* Operation completion */
    atomic_t op_state;              /* Operation state */
    int op_result;                  /* Operation result */
    
    /* List management */
    struct list_head op_list;       /* List of operations */
    struct rcu_head rcu;            /* RCU callback head */
};

/*
 * Atomic transaction context
 */
struct vexfs_atomic_transaction {
    /* Transaction identification */
    u64 trans_id;                   /* Unique transaction ID */
    u32 trans_flags;                /* Transaction flags */
    u32 isolation_level;            /* Isolation level */
    
    /* Nesting support */
    struct vexfs_atomic_transaction *parent_trans; /* Parent transaction */
    u32 nesting_level;              /* Current nesting level */
    
    /* Operation tracking */
    struct list_head op_list;       /* List of operations */
    atomic_t op_count;              /* Number of operations */
    u32 max_ops;                    /* Maximum operations allowed */
    
    /* Journal integration */
    struct vexfs_journal_transaction *journal_trans; /* Journal transaction */
    
    /* Lock-free operation queue */
    struct vexfs_lockfree_queue *op_queue; /* Operation queue */
    
    /* Synchronization */
    seqlock_t trans_seqlock;        /* Transaction sequence lock */
    atomic_t ref_count;             /* Reference count */
    struct completion trans_completion; /* Transaction completion */
    
    /* State management */
    atomic_t trans_state;           /* Transaction state */
    unsigned long start_time;       /* Transaction start time */
    unsigned long commit_time;      /* Transaction commit time */
    
    /* Error handling */
    int trans_error;                /* Transaction error code */
    struct list_head rollback_list; /* Rollback operation list */
    
    /* Performance tracking */
    atomic64_t bytes_written;       /* Bytes written in transaction */
    atomic64_t bytes_read;          /* Bytes read in transaction */
    u32 checkpoint_count;           /* Number of checkpoints */
    
    /* Memory management */
    struct kmem_cache *op_cache;    /* Operation allocation cache */
    
    /* List management */
    struct list_head trans_list;    /* List of transactions */
    struct rcu_head rcu;            /* RCU callback head */
};

/*
 * Rollback entry for transaction recovery
 */
struct vexfs_rollback_entry {
    u32 entry_type;                 /* Type of rollback entry */
    u64 target_block;               /* Target block number */
    void *original_data;            /* Original data before change */
    void *modified_data;            /* Modified data */
    size_t data_size;               /* Size of data */
    
    /* Metadata for complex operations */
    struct inode *target_inode;     /* Target inode */
    loff_t file_offset;             /* File offset */
    u32 operation_flags;            /* Operation flags */
    
    /* List management */
    struct list_head entry_list;    /* List of rollback entries */
    struct rcu_head rcu;            /* RCU callback head */
};

/*
 * Atomic operation manager
 */
struct vexfs_atomic_manager {
    /* Transaction management */
    struct list_head active_trans;  /* Active transactions */
    struct mutex trans_mutex;       /* Transaction list mutex */
    atomic64_t next_trans_id;       /* Next transaction ID */
    atomic_t active_trans_count;    /* Number of active transactions */
    
    /* Lock-free operation processing */
    struct vexfs_lockfree_queue *global_op_queue; /* Global operation queue */
    struct workqueue_struct *atomic_workqueue; /* Atomic operation workqueue */
    struct work_struct batch_work;  /* Batch processing work */
    
    /* Performance optimization */
    struct percpu_counter op_counter; /* Per-CPU operation counter */
    atomic64_t total_commits;       /* Total committed transactions */
    atomic64_t total_aborts;        /* Total aborted transactions */
    atomic64_t total_rollbacks;     /* Total rollback operations */
    
    /* Memory management */
    struct kmem_cache *trans_cache; /* Transaction allocation cache */
    struct kmem_cache *op_cache;    /* Operation allocation cache */
    struct kmem_cache *rollback_cache; /* Rollback entry cache */
    
    /* Journal integration */
    struct vexfs_journal *journal;  /* Associated journal */
    
    /* Configuration */
    u32 max_concurrent_trans;       /* Maximum concurrent transactions */
    u32 batch_size;                 /* Batch processing size */
    u32 commit_timeout;             /* Commit timeout in ms */
    
    /* Statistics */
    atomic64_t ops_processed;       /* Total operations processed */
    atomic64_t bytes_processed;     /* Total bytes processed */
    unsigned long last_batch_time;  /* Last batch processing time */
    
    /* Error handling */
    atomic_t error_count;           /* Total error count */
    struct list_head error_log;     /* Error log entries */
    
    /* Synchronization */
    struct rw_semaphore manager_rwsem; /* Manager read-write semaphore */
    spinlock_t stats_lock;          /* Statistics spinlock */
};

/*
 * Atomic VFS operation wrappers
 */
struct vexfs_atomic_vfs_ops {
    /* File operations */
    int (*atomic_create)(struct vexfs_atomic_transaction *trans,
                        struct inode *dir, struct dentry *dentry,
                        umode_t mode);
    int (*atomic_unlink)(struct vexfs_atomic_transaction *trans,
                        struct inode *dir, struct dentry *dentry);
    int (*atomic_rename)(struct vexfs_atomic_transaction *trans,
                        struct inode *old_dir, struct dentry *old_dentry,
                        struct inode *new_dir, struct dentry *new_dentry);
    
    /* Directory operations */
    int (*atomic_mkdir)(struct vexfs_atomic_transaction *trans,
                       struct inode *dir, struct dentry *dentry,
                       umode_t mode);
    int (*atomic_rmdir)(struct vexfs_atomic_transaction *trans,
                       struct inode *dir, struct dentry *dentry);
    
    /* Data operations */
    ssize_t (*atomic_write)(struct vexfs_atomic_transaction *trans,
                           struct file *file, const char __user *buf,
                           size_t count, loff_t *pos);
    int (*atomic_truncate)(struct vexfs_atomic_transaction *trans,
                          struct inode *inode, loff_t length);
    
    /* Link operations */
    int (*atomic_link)(struct vexfs_atomic_transaction *trans,
                      struct dentry *old_dentry, struct inode *dir,
                      struct dentry *new_dentry);
    int (*atomic_symlink)(struct vexfs_atomic_transaction *trans,
                         struct inode *dir, struct dentry *dentry,
                         const char *symname);
};

/* Function declarations */

/* Atomic manager initialization and cleanup */
struct vexfs_atomic_manager *vexfs_atomic_manager_init(struct vexfs_journal *journal);
void vexfs_atomic_manager_destroy(struct vexfs_atomic_manager *manager);

/* Transaction management */
struct vexfs_atomic_transaction *vexfs_atomic_begin(struct vexfs_atomic_manager *manager,
                                                   u32 flags, u32 isolation_level);
int vexfs_atomic_commit(struct vexfs_atomic_transaction *trans);
int vexfs_atomic_abort(struct vexfs_atomic_transaction *trans);
int vexfs_atomic_rollback(struct vexfs_atomic_transaction *trans);

/* Nested transaction support */
struct vexfs_atomic_transaction *vexfs_atomic_begin_nested(struct vexfs_atomic_transaction *parent,
                                                          u32 flags);
int vexfs_atomic_commit_nested(struct vexfs_atomic_transaction *trans);
int vexfs_atomic_abort_nested(struct vexfs_atomic_transaction *trans);

/* Operation management */
int vexfs_atomic_add_operation(struct vexfs_atomic_transaction *trans,
                              struct vexfs_atomic_op *op);
int vexfs_atomic_execute_operation(struct vexfs_atomic_transaction *trans,
                                  struct vexfs_atomic_op *op);
int vexfs_atomic_batch_execute(struct vexfs_atomic_transaction *trans);

/* Lock-free data structures */
struct vexfs_lockfree_queue *vexfs_lockfree_queue_create(u32 node_size);
void vexfs_lockfree_queue_destroy(struct vexfs_lockfree_queue *queue);
int vexfs_lockfree_enqueue(struct vexfs_lockfree_queue *queue, void *data);
void *vexfs_lockfree_dequeue(struct vexfs_lockfree_queue *queue);

/* Rollback management */
int vexfs_atomic_add_rollback_entry(struct vexfs_atomic_transaction *trans,
                                   u32 entry_type, u64 target_block,
                                   void *original_data, size_t data_size);
int vexfs_atomic_execute_rollback(struct vexfs_atomic_transaction *trans);

/* VFS integration */
int vexfs_atomic_register_vfs_ops(struct vexfs_atomic_manager *manager,
                                 struct vexfs_atomic_vfs_ops *ops);
int vexfs_atomic_vfs_create(struct vexfs_atomic_transaction *trans,
                           struct inode *dir, struct dentry *dentry,
                           umode_t mode);
int vexfs_atomic_vfs_unlink(struct vexfs_atomic_transaction *trans,
                           struct inode *dir, struct dentry *dentry);
ssize_t vexfs_atomic_vfs_write(struct vexfs_atomic_transaction *trans,
                              struct file *file, const char __user *buf,
                              size_t count, loff_t *pos);

/* Performance optimization */
int vexfs_atomic_optimize_batch_size(struct vexfs_atomic_manager *manager);
int vexfs_atomic_checkpoint(struct vexfs_atomic_transaction *trans);
int vexfs_atomic_force_commit_all(struct vexfs_atomic_manager *manager);

/* Crash recovery */
int vexfs_atomic_recover_partial_writes(struct vexfs_atomic_manager *manager);
int vexfs_atomic_validate_transaction_integrity(struct vexfs_atomic_transaction *trans);

/* Statistics and monitoring */
void vexfs_atomic_get_stats(struct vexfs_atomic_manager *manager,
                           struct vexfs_atomic_stats *stats);

/* Atomic operation statistics */
struct vexfs_atomic_stats {
    u64 total_transactions;
    u64 committed_transactions;
    u64 aborted_transactions;
    u64 rollback_operations;
    u64 operations_processed;
    u64 bytes_processed;
    u32 active_transactions;
    u32 average_batch_size;
    u32 average_commit_time;
    u32 lock_contention_count;
    u64 memory_usage;
    u32 error_count;
};

/* Utility macros */
#define VEXFS_ATOMIC_TRANS_ID(trans) ((trans) ? (trans)->trans_id : 0)
#define VEXFS_ATOMIC_IS_NESTED(trans) ((trans) && (trans)->parent_trans)
#define VEXFS_ATOMIC_NESTING_LEVEL(trans) ((trans) ? (trans)->nesting_level : 0)

/* Error codes specific to atomic operations */
#define VEXFS_ATOMIC_ERR_TRANS_FULL     -1001
#define VEXFS_ATOMIC_ERR_NESTED_LIMIT   -1002
#define VEXFS_ATOMIC_ERR_ROLLBACK_FAIL  -1003
#define VEXFS_ATOMIC_ERR_ISOLATION      -1004
#define VEXFS_ATOMIC_ERR_DEADLOCK       -1005

#endif /* _VEXFS_V2_ATOMIC_H */