/*
 * VexFS v2.0 Fine-Grained Locking Implementation
 * 
 * This file implements the comprehensive locking strategy for concurrent vector
 * operations with minimal contention. Provides per-vector reader/writer locks,
 * RCU for read-mostly index structures, lock-free algorithms, NUMA-aware
 * synchronization, and deadlock detection/prevention.
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/slab.h>
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
#include <linux/delay.h>
#include <linux/time.h>
#include <linux/ktime.h>
#include <linux/hash.h>
#include <linux/jhash.h>
#include <linux/workqueue.h>
#include <linux/timer.h>

#include "vexfs_v2_locking.h"

/* Global lock manager instance */
struct vexfs_lock_manager *vexfs_global_lock_manager = NULL;
EXPORT_SYMBOL(vexfs_global_lock_manager);

/* Lock ordering validation */
static struct lockdep_map vexfs_lock_dep_map[5];

/* Statistics tracking */
static DEFINE_PER_CPU(struct vexfs_lock_stats, vexfs_lock_stats_percpu);

/* 🔥 LOCK MANAGER INITIALIZATION 🔥 */

/**
 * vexfs_lock_manager_init - Initialize the global lock manager
 * @manager: Lock manager structure to initialize
 * 
 * Initializes all locking subsystems including vector locks, index locks,
 * NUMA-aware caching, deadlock detection, and statistics tracking.
 */
int vexfs_lock_manager_init(struct vexfs_lock_manager *manager)
{
    int i, ret = 0;
    
    if (!manager) {
        pr_err("VexFS: NULL lock manager in init\n");
        return -EINVAL;
    }
    
    pr_info("VexFS: Initializing fine-grained locking system\n");
    
    /* Initialize core locks */
    mutex_init(&manager->global_mutex);
    init_rwsem(&manager->global_rwsem);
    spin_lock_init(&manager->hash_lock);
    
    /* Initialize vector lock hash table */
    for (i = 0; i < ARRAY_SIZE(manager->vector_locks); i++) {
        INIT_HLIST_HEAD(&manager->vector_locks[i]);
    }
    atomic_set(&manager->vector_lock_count, 0);
    
    /* Create vector lock slab cache */
    manager->vector_lock_cache = kmem_cache_create("vexfs_vector_locks",
                                                   sizeof(struct vexfs_vector_lock),
                                                   0, SLAB_HWCACHE_ALIGN, NULL);
    if (!manager->vector_lock_cache) {
        pr_err("VexFS: Failed to create vector lock cache\n");
        ret = -ENOMEM;
        goto cleanup_basic;
    }
    
    /* Initialize index locks */
    for (i = 0; i < ARRAY_SIZE(manager->index_locks); i++) {
        manager->index_locks[i] = kzalloc(sizeof(struct vexfs_index_lock), GFP_KERNEL);
        if (!manager->index_locks[i]) {
            pr_err("VexFS: Failed to allocate index lock %d\n", i);
            ret = -ENOMEM;
            goto cleanup_index_locks;
        }
        
        /* Initialize index lock */
        seqlock_init(&manager->index_locks[i]->seq_lock);
        mutex_init(&manager->index_locks[i]->writer_mutex);
        init_completion(&manager->index_locks[i]->writer_done);
        atomic_set(&manager->index_locks[i]->reader_count, 0);
        atomic_set(&manager->index_locks[i]->writer_waiting, 0);
        manager->index_locks[i]->index_type = i;
        manager->index_locks[i]->generation = 0;
        atomic64_set(&manager->index_locks[i]->read_ops, 0);
        atomic64_set(&manager->index_locks[i]->write_ops, 0);
    }
    atomic_set(&manager->index_lock_count, ARRAY_SIZE(manager->index_locks));
    
    /* Initialize NUMA-aware caches */
    atomic_set(&manager->numa_node_count, 0);
    for_each_online_node(i) {
        if (i >= VEXFS_NUMA_MAX_NODES) break;
        
        ret = vexfs_numa_lock_cache_init(&manager->numa_caches[i], i);
        if (ret) {
            pr_err("VexFS: Failed to initialize NUMA cache for node %d\n", i);
            goto cleanup_numa_caches;
        }
        atomic_inc(&manager->numa_node_count);
    }
    
    /* Initialize deadlock detector */
    ret = vexfs_deadlock_detector_init(&manager->deadlock_detector);
    if (ret) {
        pr_err("VexFS: Failed to initialize deadlock detector\n");
        goto cleanup_numa_caches;
    }
    
    /* Initialize lock-free operation support */
    atomic64_set(&manager->lockfree_operation_id, 0);
    ret = percpu_counter_init(&manager->lockfree_ops, 0, GFP_KERNEL);
    if (ret) {
        pr_err("VexFS: Failed to initialize lock-free operation counter\n");
        goto cleanup_deadlock;
    }
    
    /* Initialize statistics */
    atomic64_set(&manager->total_acquisitions, 0);
    atomic64_set(&manager->total_contentions, 0);
    atomic64_set(&manager->total_deadlocks, 0);
    atomic64_set(&manager->adaptive_successes, 0);
    
    /* Set default configuration */
    manager->contention_threshold = VEXFS_LOCK_CONTENTION_THRESHOLD;
    manager->adaptive_threshold = VEXFS_LOCK_ADAPTIVE_THRESHOLD;
    manager->deadlock_timeout_ms = VEXFS_DEADLOCK_TIMEOUT_MS;
    manager->numa_aware = true;
    manager->deadlock_detection = true;
    manager->adaptive_locking = true;
    
    /* Initialize lockdep classes */
    for (i = 0; i < ARRAY_SIZE(vexfs_lock_dep_map); i++) {
        lockdep_init_map(&vexfs_lock_dep_map[i], "vexfs_lock", NULL, i);
    }
    
    pr_info("VexFS: Lock manager initialized successfully\n");
    pr_info("VexFS: NUMA nodes: %d, Index locks: %d, Vector lock cache: %px\n",
            atomic_read(&manager->numa_node_count),
            atomic_read(&manager->index_lock_count),
            manager->vector_lock_cache);
    
    return 0;

cleanup_deadlock:
    vexfs_deadlock_detector_cleanup(&manager->deadlock_detector);
cleanup_numa_caches:
    for (i = 0; i < VEXFS_NUMA_MAX_NODES; i++) {
        if (manager->numa_caches[i].numa_node != 0) {
            vexfs_numa_lock_cache_cleanup(&manager->numa_caches[i]);
        }
    }
cleanup_index_locks:
    for (i = 0; i < ARRAY_SIZE(manager->index_locks); i++) {
        kfree(manager->index_locks[i]);
        manager->index_locks[i] = NULL;
    }
    if (manager->vector_lock_cache) {
        kmem_cache_destroy(manager->vector_lock_cache);
    }
cleanup_basic:
    return ret;
}

/**
 * vexfs_lock_manager_cleanup - Cleanup the lock manager
 * @manager: Lock manager to cleanup
 */
void vexfs_lock_manager_cleanup(struct vexfs_lock_manager *manager)
{
    int i;
    
    if (!manager) {
        return;
    }
    
    pr_info("VexFS: Cleaning up lock manager\n");
    
    /* Cleanup deadlock detector */
    vexfs_deadlock_detector_cleanup(&manager->deadlock_detector);
    
    /* Cleanup NUMA caches */
    for (i = 0; i < VEXFS_NUMA_MAX_NODES; i++) {
        if (manager->numa_caches[i].numa_node != 0) {
            vexfs_numa_lock_cache_cleanup(&manager->numa_caches[i]);
        }
    }
    
    /* Cleanup index locks */
    for (i = 0; i < ARRAY_SIZE(manager->index_locks); i++) {
        if (manager->index_locks[i]) {
            /* Wait for any pending operations */
            mutex_lock(&manager->index_locks[i]->writer_mutex);
            mutex_unlock(&manager->index_locks[i]->writer_mutex);
            kfree(manager->index_locks[i]);
            manager->index_locks[i] = NULL;
        }
    }
    
    /* Cleanup vector lock cache */
    if (manager->vector_lock_cache) {
        kmem_cache_destroy(manager->vector_lock_cache);
        manager->vector_lock_cache = NULL;
    }
    
    /* Cleanup per-CPU counters */
    percpu_counter_destroy(&manager->lockfree_ops);
    
    pr_info("VexFS: Lock manager cleanup completed\n");
}

/**
 * vexfs_lock_manager_configure - Configure lock manager parameters
 * @manager: Lock manager to configure
 * @contention_threshold: Lock contention threshold
 * @adaptive_threshold: Adaptive locking threshold
 * @numa_aware: Enable NUMA-aware optimizations
 * @deadlock_detection: Enable deadlock detection
 */
int vexfs_lock_manager_configure(struct vexfs_lock_manager *manager,
                                 u32 contention_threshold,
                                 u32 adaptive_threshold,
                                 bool numa_aware,
                                 bool deadlock_detection)
{
    if (!manager) {
        return -EINVAL;
    }
    
    mutex_lock(&manager->global_mutex);
    
    manager->contention_threshold = contention_threshold;
    manager->adaptive_threshold = adaptive_threshold;
    manager->numa_aware = numa_aware;
    manager->deadlock_detection = deadlock_detection;
    
    pr_info("VexFS: Lock manager configured - contention: %u, adaptive: %u, NUMA: %s, deadlock: %s\n",
            contention_threshold, adaptive_threshold,
            numa_aware ? "enabled" : "disabled",
            deadlock_detection ? "enabled" : "disabled");
    
    mutex_unlock(&manager->global_mutex);
    
    return 0;
}

/* 🔥 VECTOR LOCKING OPERATIONS 🔥 */

/**
 * vexfs_lock_hash_vector_id - Hash vector ID for lock table
 * @vector_id: Vector ID to hash
 */
u32 vexfs_lock_hash_vector_id(u64 vector_id)
{
    return jhash_2words((u32)vector_id, (u32)(vector_id >> 32), 0) & 1023;
}

/**
 * vexfs_vector_lock_create - Create a new vector lock
 * @manager: Lock manager
 * @vector_id: Vector ID
 * @numa_node: Preferred NUMA node
 */
static struct vexfs_vector_lock *vexfs_vector_lock_create(struct vexfs_lock_manager *manager,
                                                          u64 vector_id,
                                                          u32 numa_node)
{
    struct vexfs_vector_lock *lock;
    
    /* Allocate from slab cache */
    lock = kmem_cache_alloc(manager->vector_lock_cache, GFP_KERNEL);
    if (!lock) {
        pr_err("VexFS: Failed to allocate vector lock for ID %llu\n", vector_id);
        return NULL;
    }
    
    /* Initialize lock structure */
    init_rwsem(&lock->rwsem);
    atomic_set(&lock->ref_count, 1);
    atomic_set(&lock->reader_count, 0);
    atomic_set(&lock->writer_count, 0);
    atomic64_set(&lock->contention_count, 0);
    lock->vector_id = vector_id;
    lock->numa_node = numa_node;
    lock->lock_order = VEXFS_LOCK_ORDER_VECTOR;
    INIT_HLIST_NODE(&lock->hash_node);
    spin_lock_init(&lock->stats_lock);
    lock->acquire_time_total = 0;
    lock->hold_time_total = 0;
    lock->acquire_count = 0;
    
    atomic_inc(&manager->vector_lock_count);
    
    pr_debug("VexFS: Created vector lock for ID %llu on NUMA node %u\n",
             vector_id, numa_node);
    
    return lock;
}

/**
 * vexfs_vector_lock_destroy_rcu - RCU callback for lock destruction
 * @rcu_head: RCU head from the lock structure
 */
static void vexfs_vector_lock_destroy_rcu(struct rcu_head *rcu_head)
{
    struct vexfs_vector_lock *lock = container_of(rcu_head, struct vexfs_vector_lock, rcu_head);
    
    pr_debug("VexFS: RCU destroying vector lock for ID %llu\n", lock->vector_id);
    
    /* Free the lock back to slab cache */
    kmem_cache_free(vexfs_global_lock_manager->vector_lock_cache, lock);
    atomic_dec(&vexfs_global_lock_manager->vector_lock_count);
}

/**
 * vexfs_vector_lock_acquire - Acquire a vector lock
 * @manager: Lock manager
 * @vector_id: Vector ID to lock
 * @op: Lock operation type
 * @timeout_ms: Timeout in milliseconds (0 = no timeout)
 */
struct vexfs_vector_lock *vexfs_vector_lock_acquire(struct vexfs_lock_manager *manager,
                                                    u64 vector_id,
                                                    enum vexfs_lock_op op,
                                                    u32 timeout_ms)
{
    struct vexfs_vector_lock *lock = NULL;
    struct hlist_head *head;
    u32 hash;
    u32 numa_node;
    ktime_t start_time;
    int ret;
    
    if (!manager) {
        pr_err("VexFS: NULL manager in vector lock acquire\n");
        return ERR_PTR(-EINVAL);
    }
    
    start_time = ktime_get();
    hash = vexfs_lock_hash_vector_id(vector_id);
    head = &manager->vector_locks[hash];
    numa_node = vexfs_numa_get_preferred_node(vector_id);
    
    /* Try NUMA cache first if enabled */
    if (manager->numa_aware && numa_node < VEXFS_NUMA_MAX_NODES) {
        lock = vexfs_numa_lock_cache_get(&manager->numa_caches[numa_node], vector_id);
        if (lock) {
            goto acquire_lock;
        }
    }
    
    /* Search hash table */
    spin_lock(&manager->hash_lock);
    hlist_for_each_entry(lock, head, hash_node) {
        if (lock->vector_id == vector_id) {
            atomic_inc(&lock->ref_count);
            spin_unlock(&manager->hash_lock);
            goto acquire_lock;
        }
    }
    
    /* Create new lock */
    lock = vexfs_vector_lock_create(manager, vector_id, numa_node);
    if (!lock) {
        spin_unlock(&manager->hash_lock);
        return ERR_PTR(-ENOMEM);
    }
    
    /* Add to hash table */
    hlist_add_head(&lock->hash_node, head);
    spin_unlock(&manager->hash_lock);
    
acquire_lock:
    /* Perform the actual lock acquisition */
    switch (op) {
    case VEXFS_LOCK_READ:
        if (timeout_ms > 0) {
            ret = down_read_timeout(&lock->rwsem, msecs_to_jiffies(timeout_ms));
            if (ret) {
                pr_warn("VexFS: Read lock timeout for vector %llu\n", vector_id);
                goto error_timeout;
            }
        } else {
            down_read(&lock->rwsem);
        }
        atomic_inc(&lock->reader_count);
        break;
        
    case VEXFS_LOCK_WRITE:
        if (timeout_ms > 0) {
            ret = down_write_timeout(&lock->rwsem, msecs_to_jiffies(timeout_ms));
            if (ret) {
                pr_warn("VexFS: Write lock timeout for vector %llu\n", vector_id);
                goto error_timeout;
            }
        } else {
            down_write(&lock->rwsem);
        }
        atomic_inc(&lock->writer_count);
        break;
        
    case VEXFS_LOCK_TRY_READ:
        if (!down_read_trylock(&lock->rwsem)) {
            atomic64_inc(&lock->contention_count);
            goto error_contention;
        }
        atomic_inc(&lock->reader_count);
        break;
        
    case VEXFS_LOCK_TRY_WRITE:
        if (!down_write_trylock(&lock->rwsem)) {
            atomic64_inc(&lock->contention_count);
            goto error_contention;
        }
        atomic_inc(&lock->writer_count);
        break;
        
    default:
        pr_err("VexFS: Invalid lock operation: %d\n", op);
        goto error_invalid;
    }
    
    /* Update statistics */
    atomic64_inc(&manager->total_acquisitions);
    
    /* Update lock statistics */
    spin_lock(&lock->stats_lock);
    lock->acquire_count++;
    lock->acquire_time_total += ktime_to_ns(ktime_sub(ktime_get(), start_time));
    spin_unlock(&lock->stats_lock);
    
    pr_debug("VexFS: Acquired %s lock for vector %llu\n",
             vexfs_lock_op_name(op), vector_id);
    
    return lock;

error_timeout:
    atomic64_inc(&manager->total_contentions);
    /* Fall through */
error_contention:
    /* Fall through */
error_invalid:
    /* Release reference */
    if (atomic_dec_and_test(&lock->ref_count)) {
        /* Remove from hash table and schedule RCU destruction */
        spin_lock(&manager->hash_lock);
        hlist_del_rcu(&lock->hash_node);
        spin_unlock(&manager->hash_lock);
        call_rcu(&lock->rcu_head, vexfs_vector_lock_destroy_rcu);
    }
    
    return ERR_PTR(-EBUSY);
}

/**
 * vexfs_vector_lock_release - Release a vector lock
 * @lock: Vector lock to release
 * @op: Lock operation type used for acquisition
 */
int vexfs_vector_lock_release(struct vexfs_vector_lock *lock, enum vexfs_lock_op op)
{
    if (!lock) {
        pr_err("VexFS: NULL lock in release\n");
        return -EINVAL;
    }
    
    /* Release the actual lock */
    switch (op) {
    case VEXFS_LOCK_READ:
    case VEXFS_LOCK_TRY_READ:
        up_read(&lock->rwsem);
        atomic_dec(&lock->reader_count);
        break;
        
    case VEXFS_LOCK_WRITE:
    case VEXFS_LOCK_TRY_WRITE:
        up_write(&lock->rwsem);
        atomic_dec(&lock->writer_count);
        break;
        
    default:
        pr_err("VexFS: Invalid lock operation for release: %d\n", op);
        return -EINVAL;
    }
    
    pr_debug("VexFS: Released %s lock for vector %llu\n",
             vexfs_lock_op_name(op), lock->vector_id);
    
    /* Release reference */
    if (atomic_dec_and_test(&lock->ref_count)) {
        /* Remove from hash table and schedule RCU destruction */
        spin_lock(&vexfs_global_lock_manager->hash_lock);
        hlist_del_rcu(&lock->hash_node);
        spin_unlock(&vexfs_global_lock_manager->hash_lock);
        call_rcu(&lock->rcu_head, vexfs_vector_lock_destroy_rcu);
    }
    
    return 0;
}

/**
 * vexfs_vector_lock_upgrade - Upgrade read lock to write lock
 * @lock: Vector lock to upgrade
 */
int vexfs_vector_lock_upgrade(struct vexfs_vector_lock *lock)
{
    if (!lock) {
        return -EINVAL;
    }
    
    /* Release read lock */
    up_read(&lock->rwsem);
    atomic_dec(&lock->reader_count);
    
    /* Acquire write lock */
    down_write(&lock->rwsem);
    atomic_inc(&lock->writer_count);
    
    pr_debug("VexFS: Upgraded lock for vector %llu\n", lock->vector_id);
    
    return 0;
}

/**
 * vexfs_vector_lock_downgrade - Downgrade write lock to read lock
 * @lock: Vector lock to downgrade
 */
int vexfs_vector_lock_downgrade(struct vexfs_vector_lock *lock)
{
    if (!lock) {
        return -EINVAL;
    }
    
    /* Downgrade write lock to read lock */
    downgrade_write(&lock->rwsem);
    atomic_dec(&lock->writer_count);
    atomic_inc(&lock->reader_count);
    
    pr_debug("VexFS: Downgraded lock for vector %llu\n", lock->vector_id);
    
    return 0;
}

/* 🔥 UTILITY FUNCTIONS 🔥 */

/**
 * vexfs_numa_get_preferred_node - Get preferred NUMA node for vector
 * @vector_id: Vector ID
 */
u32 vexfs_numa_get_preferred_node(u64 vector_id)
{
    /* Simple hash-based NUMA node selection */
    return (u32)(vector_id % num_online_nodes());
}

/**
 * vexfs_lock_op_name - Get human-readable lock operation name
 * @op: Lock operation
 */
const char *vexfs_lock_op_name(enum vexfs_lock_op op)
{
    switch (op) {
    case VEXFS_LOCK_READ: return "READ";
    case VEXFS_LOCK_WRITE: return "WRITE";
    case VEXFS_LOCK_UPGRADE: return "UPGRADE";
    case VEXFS_LOCK_DOWNGRADE: return "DOWNGRADE";
    case VEXFS_LOCK_TRY_READ: return "TRY_READ";
    case VEXFS_LOCK_TRY_WRITE: return "TRY_WRITE";
    default: return "UNKNOWN";
    }
}

/**
 * vexfs_lock_scope_name - Get human-readable lock scope name
 * @scope: Lock scope
 */
const char *vexfs_lock_scope_name(enum vexfs_lock_scope scope)
{
    switch (scope) {
    case VEXFS_SCOPE_GLOBAL: return "GLOBAL";
    case VEXFS_SCOPE_INDEX: return "INDEX";
    case VEXFS_SCOPE_VECTOR: return "VECTOR";
    case VEXFS_SCOPE_METADATA: return "METADATA";
    case VEXFS_SCOPE_BATCH: return "BATCH";
    default: return "UNKNOWN";
    }
}

/**
 * vexfs_lock_order_valid - Validate lock ordering
 * @order1: First lock order
 * @order2: Second lock order
 */
bool vexfs_lock_order_valid(u32 order1, u32 order2)
{
    /* Locks must be acquired in ascending order */
    return order1 <= order2;
}

/**
 * vexfs_lock_validate_ordering - Validate lock ordering with lockdep
 * @lock1: First lock
 * @lock2: Second lock
 * @order1: First lock order
 * @order2: Second lock order
 */
void vexfs_lock_validate_ordering(void *lock1, void *lock2, u32 order1, u32 order2)
{
    if (!vexfs_lock_order_valid(order1, order2)) {
        pr_warn("VexFS: Invalid lock ordering detected: %u -> %u\n", order1, order2);
        WARN_ON_ONCE(1);
    }
    
    /* Use lockdep to validate ordering */
    if (order1 < ARRAY_SIZE(vexfs_lock_dep_map) && order2 < ARRAY_SIZE(vexfs_lock_dep_map)) {
        lock_acquire(&vexfs_lock_dep_map[order1], 0, 0, 0, 0, NULL, _THIS_IP_);
        lock_acquire(&vexfs_lock_dep_map[order2], 0, 0, 0, 0, NULL, _THIS_IP_);
        lock_release(&vexfs_lock_dep_map[order2], 0, _THIS_IP_);
        lock_release(&vexfs_lock_dep_map[order1], 0, _THIS_IP_);
    }
}