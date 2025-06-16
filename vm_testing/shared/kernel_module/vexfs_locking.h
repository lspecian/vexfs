/*
 * VexFS v2.0 Fine-Grained Locking Infrastructure
 * 
 * Comprehensive locking strategy for concurrent vector operations with minimal
 * contention. This header defines the locking primitives, data structures, and
 * synchronization mechanisms for high-performance concurrent vector operations.
 * 
 * Key Features:
 * - Per-vector reader/writer locks using rwsem
 * - RCU for read-mostly index structures
 * - Lock-free algorithms for high-contention operations
 * - NUMA-aware synchronization primitives
 * - Deadlock detection and prevention mechanisms
 * - Hierarchical lock ordering to prevent deadlocks
 */

#ifndef VEXFS_V2_LOCKING_H
#define VEXFS_V2_LOCKING_H

#include <linux/kernel.h>
#include <linux/types.h>
#include <linux/rwsem.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/rcu.h>
#include <linux/atomic.h>
#include <linux/percpu.h>
#include <linux/numa.h>
#include <linux/lockdep.h>
#include <linux/seqlock.h>
#include <linux/completion.h>
#include <linux/wait.h>

/* Lock ordering hierarchy to prevent deadlocks */
#define VEXFS_LOCK_ORDER_GLOBAL         0
#define VEXFS_LOCK_ORDER_INDEX          1
#define VEXFS_LOCK_ORDER_VECTOR_TABLE   2
#define VEXFS_LOCK_ORDER_VECTOR         3
#define VEXFS_LOCK_ORDER_METADATA       4

/* Lock contention thresholds */
#define VEXFS_LOCK_CONTENTION_THRESHOLD 1000
#define VEXFS_LOCK_ADAPTIVE_THRESHOLD   100
#define VEXFS_LOCK_BACKOFF_MAX_US       1000

/* RCU grace period configuration */
#define VEXFS_RCU_GRACE_PERIOD_MS       10
#define VEXFS_RCU_BATCH_SIZE            64

/* Lock-free operation limits */
#define VEXFS_LOCKFREE_RETRY_MAX        1000
#define VEXFS_LOCKFREE_BACKOFF_MIN_NS   100
#define VEXFS_LOCKFREE_BACKOFF_MAX_NS   10000

/* NUMA-aware configuration */
#define VEXFS_NUMA_LOCK_CACHE_SIZE      64
#define VEXFS_NUMA_MAX_NODES            8

/* Deadlock detection configuration */
#define VEXFS_DEADLOCK_TIMEOUT_MS       5000
#define VEXFS_DEADLOCK_CHECK_INTERVAL   100

/* 🔥 CORE LOCKING STRUCTURES 🔥 */

/**
 * Vector Lock Entry
 * 
 * Per-vector locking structure providing fine-grained synchronization
 * for individual vector operations with reader/writer semantics.
 */
struct vexfs_vector_lock {
    struct rw_semaphore rwsem;          /* Reader/writer semaphore */
    atomic_t ref_count;                 /* Reference counter */
    atomic_t reader_count;              /* Active reader count */
    atomic_t writer_count;              /* Active writer count */
    atomic64_t contention_count;        /* Lock contention counter */
    u64 vector_id;                      /* Associated vector ID */
    u32 numa_node;                      /* NUMA node affinity */
    u32 lock_order;                     /* Lock ordering level */
    struct hlist_node hash_node;        /* Hash table linkage */
    struct rcu_head rcu_head;           /* RCU cleanup */
    spinlock_t stats_lock;              /* Statistics protection */
    u64 acquire_time_total;             /* Total acquisition time */
    u64 hold_time_total;                /* Total hold time */
    u32 acquire_count;                  /* Acquisition count */
} ____cacheline_aligned;

/**
 * Index Lock Structure
 * 
 * RCU-protected locking for read-mostly index structures with
 * optimized reader performance and minimal writer contention.
 */
struct vexfs_index_lock {
    struct rcu_head rcu_head;           /* RCU synchronization */
    seqlock_t seq_lock;                 /* Sequential lock for updates */
    atomic_t reader_count;              /* Active reader count */
    atomic_t writer_waiting;            /* Writers waiting count */
    struct mutex writer_mutex;          /* Writer exclusion */
    struct completion writer_done;      /* Writer completion */
    u32 index_type;                     /* Index type identifier */
    u32 generation;                     /* Index generation number */
    atomic64_t read_ops;                /* Read operation counter */
    atomic64_t write_ops;               /* Write operation counter */
    u64 last_update_time;               /* Last update timestamp */
} ____cacheline_aligned;

/**
 * Lock-Free Operation Context
 * 
 * Context structure for lock-free algorithms with retry logic,
 * backoff strategies, and contention management.
 */
struct vexfs_lockfree_ctx {
    atomic_t retry_count;               /* Current retry count */
    atomic_t backoff_delay;             /* Current backoff delay */
    atomic64_t operation_id;            /* Unique operation ID */
    u64 start_time;                     /* Operation start time */
    u32 cpu_id;                         /* CPU affinity */
    u32 numa_node;                      /* NUMA node */
    struct {
        atomic64_t attempts;            /* Total attempts */
        atomic64_t successes;           /* Successful operations */
        atomic64_t failures;            /* Failed operations */
        atomic64_t contentions;         /* Contention events */
    } stats;
} ____cacheline_aligned;

/**
 * NUMA-Aware Lock Cache
 * 
 * Per-NUMA-node lock caching for improved locality and
 * reduced cross-node synchronization overhead.
 */
struct vexfs_numa_lock_cache {
    spinlock_t cache_lock;              /* Cache protection */
    struct hlist_head lock_hash[VEXFS_NUMA_LOCK_CACHE_SIZE];
    atomic_t cache_size;                /* Current cache size */
    atomic_t hit_count;                 /* Cache hit counter */
    atomic_t miss_count;                /* Cache miss counter */
    u32 numa_node;                      /* NUMA node ID */
    struct percpu_counter active_locks; /* Active locks counter */
} ____cacheline_aligned;

/**
 * Deadlock Detection Context
 * 
 * Deadlock detection and prevention mechanism with lock
 * dependency tracking and timeout-based resolution.
 */
struct vexfs_deadlock_detector {
    struct mutex detector_mutex;        /* Detector protection */
    struct hlist_head lock_graph[256];  /* Lock dependency graph */
    atomic_t detection_active;          /* Detection active flag */
    atomic64_t deadlock_count;          /* Detected deadlocks */
    atomic64_t prevention_count;        /* Prevented deadlocks */
    struct timer_list check_timer;      /* Periodic check timer */
    struct work_struct check_work;      /* Check work queue */
    u64 last_check_time;                /* Last check timestamp */
} ____cacheline_aligned;

/**
 * Global Locking Manager
 * 
 * Central coordination structure for all VexFS locking operations
 * with statistics, configuration, and management functions.
 */
struct vexfs_lock_manager {
    /* Core lock structures */
    struct mutex global_mutex;          /* Global coordination */
    struct rw_semaphore global_rwsem;   /* Global reader/writer */
    spinlock_t hash_lock;               /* Hash table protection */
    
    /* Vector lock management */
    struct hlist_head vector_locks[1024]; /* Vector lock hash table */
    atomic_t vector_lock_count;         /* Active vector locks */
    struct kmem_cache *vector_lock_cache; /* Vector lock slab cache */
    
    /* Index lock management */
    struct vexfs_index_lock *index_locks[16]; /* Index locks array */
    atomic_t index_lock_count;          /* Active index locks */
    
    /* NUMA-aware caching */
    struct vexfs_numa_lock_cache numa_caches[VEXFS_NUMA_MAX_NODES];
    atomic_t numa_node_count;           /* Active NUMA nodes */
    
    /* Deadlock detection */
    struct vexfs_deadlock_detector deadlock_detector;
    
    /* Lock-free operation support */
    atomic64_t lockfree_operation_id;   /* Global operation ID */
    struct percpu_counter lockfree_ops; /* Lock-free operations */
    
    /* Statistics and monitoring */
    atomic64_t total_acquisitions;      /* Total lock acquisitions */
    atomic64_t total_contentions;       /* Total contentions */
    atomic64_t total_deadlocks;         /* Total deadlocks */
    atomic64_t adaptive_successes;      /* Adaptive lock successes */
    
    /* Configuration */
    u32 contention_threshold;           /* Contention threshold */
    u32 adaptive_threshold;             /* Adaptive threshold */
    u32 deadlock_timeout_ms;            /* Deadlock timeout */
    bool numa_aware;                    /* NUMA awareness enabled */
    bool deadlock_detection;            /* Deadlock detection enabled */
    bool adaptive_locking;              /* Adaptive locking enabled */
} ____cacheline_aligned;

/* 🔥 LOCKING OPERATION TYPES 🔥 */

/* Lock operation types */
enum vexfs_lock_op {
    VEXFS_LOCK_READ = 0,
    VEXFS_LOCK_WRITE,
    VEXFS_LOCK_UPGRADE,
    VEXFS_LOCK_DOWNGRADE,
    VEXFS_LOCK_TRY_READ,
    VEXFS_LOCK_TRY_WRITE
};

/* Lock scope types */
enum vexfs_lock_scope {
    VEXFS_SCOPE_GLOBAL = 0,
    VEXFS_SCOPE_INDEX,
    VEXFS_SCOPE_VECTOR,
    VEXFS_SCOPE_METADATA,
    VEXFS_SCOPE_BATCH
};

/* Lock-free operation types */
enum vexfs_lockfree_op {
    VEXFS_LOCKFREE_CAS = 0,             /* Compare-and-swap */
    VEXFS_LOCKFREE_FAA,                 /* Fetch-and-add */
    VEXFS_LOCKFREE_XCHG,                /* Exchange */
    VEXFS_LOCKFREE_CMPXCHG              /* Compare-exchange */
};

/* 🔥 FUNCTION DECLARATIONS 🔥 */

/* Lock manager initialization and cleanup */
int vexfs_lock_manager_init(struct vexfs_lock_manager *manager);
void vexfs_lock_manager_cleanup(struct vexfs_lock_manager *manager);
int vexfs_lock_manager_configure(struct vexfs_lock_manager *manager,
                                 u32 contention_threshold,
                                 u32 adaptive_threshold,
                                 bool numa_aware,
                                 bool deadlock_detection);

/* Vector locking operations */
struct vexfs_vector_lock *vexfs_vector_lock_acquire(struct vexfs_lock_manager *manager,
                                                    u64 vector_id,
                                                    enum vexfs_lock_op op,
                                                    u32 timeout_ms);
int vexfs_vector_lock_release(struct vexfs_vector_lock *lock,
                              enum vexfs_lock_op op);
int vexfs_vector_lock_upgrade(struct vexfs_vector_lock *lock);
int vexfs_vector_lock_downgrade(struct vexfs_vector_lock *lock);
bool vexfs_vector_lock_try_acquire(struct vexfs_vector_lock *lock,
                                   enum vexfs_lock_op op);

/* Index locking operations */
struct vexfs_index_lock *vexfs_index_lock_acquire(struct vexfs_lock_manager *manager,
                                                  u32 index_type,
                                                  enum vexfs_lock_op op);
int vexfs_index_lock_release(struct vexfs_index_lock *lock,
                             enum vexfs_lock_op op);
int vexfs_index_rcu_read_lock(struct vexfs_index_lock *lock);
int vexfs_index_rcu_read_unlock(struct vexfs_index_lock *lock);
int vexfs_index_update_begin(struct vexfs_index_lock *lock);
int vexfs_index_update_end(struct vexfs_index_lock *lock);

/* Lock-free operations */
int vexfs_lockfree_init_ctx(struct vexfs_lockfree_ctx *ctx);
bool vexfs_lockfree_cas(atomic64_t *target, u64 expected, u64 new_value,
                        struct vexfs_lockfree_ctx *ctx);
u64 vexfs_lockfree_faa(atomic64_t *target, u64 increment,
                       struct vexfs_lockfree_ctx *ctx);
u64 vexfs_lockfree_xchg(atomic64_t *target, u64 new_value,
                        struct vexfs_lockfree_ctx *ctx);
bool vexfs_lockfree_retry(struct vexfs_lockfree_ctx *ctx);
void vexfs_lockfree_backoff(struct vexfs_lockfree_ctx *ctx);

/* NUMA-aware operations */
int vexfs_numa_lock_cache_init(struct vexfs_numa_lock_cache *cache, u32 numa_node);
void vexfs_numa_lock_cache_cleanup(struct vexfs_numa_lock_cache *cache);
struct vexfs_vector_lock *vexfs_numa_lock_cache_get(struct vexfs_numa_lock_cache *cache,
                                                    u64 vector_id);
int vexfs_numa_lock_cache_put(struct vexfs_numa_lock_cache *cache,
                              struct vexfs_vector_lock *lock);
u32 vexfs_numa_get_preferred_node(u64 vector_id);

/* Deadlock detection and prevention */
int vexfs_deadlock_detector_init(struct vexfs_deadlock_detector *detector);
void vexfs_deadlock_detector_cleanup(struct vexfs_deadlock_detector *detector);
int vexfs_deadlock_check_dependency(struct vexfs_deadlock_detector *detector,
                                    void *lock1, void *lock2, u32 order1, u32 order2);
bool vexfs_deadlock_would_create_cycle(struct vexfs_deadlock_detector *detector,
                                       void *lock1, void *lock2);
int vexfs_deadlock_resolve(struct vexfs_deadlock_detector *detector,
                           void **locks, u32 count);

/* Adaptive locking */
bool vexfs_adaptive_should_spin(struct vexfs_vector_lock *lock);
int vexfs_adaptive_lock_acquire(struct vexfs_vector_lock *lock,
                                enum vexfs_lock_op op,
                                u32 timeout_ms);
void vexfs_adaptive_update_stats(struct vexfs_vector_lock *lock,
                                 u64 wait_time, bool success);

/* Lock statistics and monitoring */
int vexfs_lock_get_stats(struct vexfs_lock_manager *manager,
                         struct vexfs_lock_stats *stats);
int vexfs_lock_reset_stats(struct vexfs_lock_manager *manager);
void vexfs_lock_dump_contention(struct vexfs_lock_manager *manager);
void vexfs_lock_dump_deadlocks(struct vexfs_lock_manager *manager);

/* Utility functions */
u32 vexfs_lock_hash_vector_id(u64 vector_id);
bool vexfs_lock_order_valid(u32 order1, u32 order2);
void vexfs_lock_validate_ordering(void *lock1, void *lock2, u32 order1, u32 order2);
const char *vexfs_lock_op_name(enum vexfs_lock_op op);
const char *vexfs_lock_scope_name(enum vexfs_lock_scope scope);

/* Debug and testing support */
#ifdef CONFIG_VEXFS_DEBUG_LOCKING
void vexfs_lock_debug_enable(struct vexfs_lock_manager *manager);
void vexfs_lock_debug_disable(struct vexfs_lock_manager *manager);
void vexfs_lock_debug_trace(const char *operation, void *lock, u32 order);
#else
static inline void vexfs_lock_debug_enable(struct vexfs_lock_manager *manager) {}
static inline void vexfs_lock_debug_disable(struct vexfs_lock_manager *manager) {}
static inline void vexfs_lock_debug_trace(const char *operation, void *lock, u32 order) {}
#endif

/* Lock statistics structure */
struct vexfs_lock_stats {
    u64 total_acquisitions;
    u64 total_contentions;
    u64 total_deadlocks;
    u64 adaptive_successes;
    u64 lockfree_operations;
    u64 numa_cache_hits;
    u64 numa_cache_misses;
    u64 avg_hold_time_ns;
    u64 max_contention_time_ns;
    u32 active_vector_locks;
    u32 active_index_locks;
    u32 deadlock_detection_runs;
};

/* External global lock manager instance */
extern struct vexfs_lock_manager *vexfs_global_lock_manager;

/* Convenience macros */
#define vexfs_vector_read_lock(mgr, id) \
    vexfs_vector_lock_acquire(mgr, id, VEXFS_LOCK_READ, 0)
#define vexfs_vector_write_lock(mgr, id) \
    vexfs_vector_lock_acquire(mgr, id, VEXFS_LOCK_WRITE, 0)
#define vexfs_vector_unlock(lock, op) \
    vexfs_vector_lock_release(lock, op)

#define vexfs_index_read_lock(mgr, type) \
    vexfs_index_lock_acquire(mgr, type, VEXFS_LOCK_READ)
#define vexfs_index_write_lock(mgr, type) \
    vexfs_index_lock_acquire(mgr, type, VEXFS_LOCK_WRITE)
#define vexfs_index_unlock(lock, op) \
    vexfs_index_lock_release(lock, op)

#endif /* VEXFS_V2_LOCKING_H */