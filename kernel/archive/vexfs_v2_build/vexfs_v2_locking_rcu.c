/*
 * VexFS v2.0 RCU and Lock-Free Algorithm Implementation
 * 
 * This file implements RCU-based synchronization for read-mostly index
 * structures and lock-free algorithms for high-contention operations.
 * Provides optimized reader performance with minimal writer overhead.
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/rcu.h>
#include <linux/atomic.h>
#include <linux/delay.h>
#include <linux/random.h>
#include <linux/cpu.h>
#include <linux/smp.h>
#include <linux/preempt.h>

#include "vexfs_v2_locking.h"

/* Lock-free operation statistics */
static DEFINE_PER_CPU(struct {
    atomic64_t cas_attempts;
    atomic64_t cas_successes;
    atomic64_t faa_operations;
    atomic64_t xchg_operations;
    atomic64_t backoff_events;
}, vexfs_lockfree_stats);

/* 🔥 INDEX LOCKING WITH RCU 🔥 */

/**
 * vexfs_index_lock_acquire - Acquire index lock with RCU optimization
 * @manager: Lock manager
 * @index_type: Type of index to lock
 * @op: Lock operation (read/write)
 */
struct vexfs_index_lock *vexfs_index_lock_acquire(struct vexfs_lock_manager *manager,
                                                  u32 index_type,
                                                  enum vexfs_lock_op op)
{
    struct vexfs_index_lock *lock;
    
    if (!manager || index_type >= ARRAY_SIZE(manager->index_locks)) {
        pr_err("VexFS: Invalid parameters in index lock acquire\n");
        return ERR_PTR(-EINVAL);
    }
    
    lock = manager->index_locks[index_type];
    if (!lock) {
        pr_err("VexFS: Index lock %u not initialized\n", index_type);
        return ERR_PTR(-ENOENT);
    }
    
    switch (op) {
    case VEXFS_LOCK_READ:
        return vexfs_index_rcu_read_lock(lock) ? lock : ERR_PTR(-EBUSY);
        
    case VEXFS_LOCK_WRITE:
        return vexfs_index_update_begin(lock) ? ERR_PTR(-EBUSY) : lock;
        
    default:
        pr_err("VexFS: Invalid index lock operation: %d\n", op);
        return ERR_PTR(-EINVAL);
    }
}

/**
 * vexfs_index_lock_release - Release index lock
 * @lock: Index lock to release
 * @op: Lock operation used for acquisition
 */
int vexfs_index_lock_release(struct vexfs_index_lock *lock, enum vexfs_lock_op op)
{
    if (!lock) {
        return -EINVAL;
    }
    
    switch (op) {
    case VEXFS_LOCK_READ:
        return vexfs_index_rcu_read_unlock(lock);
        
    case VEXFS_LOCK_WRITE:
        return vexfs_index_update_end(lock);
        
    default:
        pr_err("VexFS: Invalid index lock release operation: %d\n", op);
        return -EINVAL;
    }
}

/**
 * vexfs_index_rcu_read_lock - Acquire RCU read lock for index
 * @lock: Index lock structure
 */
int vexfs_index_rcu_read_lock(struct vexfs_index_lock *lock)
{
    if (!lock) {
        return -EINVAL;
    }
    
    /* Enter RCU read-side critical section */
    rcu_read_lock();
    
    /* Increment reader count */
    atomic_inc(&lock->reader_count);
    atomic64_inc(&lock->read_ops);
    
    /* Memory barrier to ensure ordering */
    smp_mb();
    
    pr_debug("VexFS: Acquired RCU read lock for index %u (readers: %d)\n",
             lock->index_type, atomic_read(&lock->reader_count));
    
    return 0;
}

/**
 * vexfs_index_rcu_read_unlock - Release RCU read lock for index
 * @lock: Index lock structure
 */
int vexfs_index_rcu_read_unlock(struct vexfs_index_lock *lock)
{
    if (!lock) {
        return -EINVAL;
    }
    
    /* Decrement reader count */
    atomic_dec(&lock->reader_count);
    
    /* Exit RCU read-side critical section */
    rcu_read_unlock();
    
    pr_debug("VexFS: Released RCU read lock for index %u (readers: %d)\n",
             lock->index_type, atomic_read(&lock->reader_count));
    
    return 0;
}

/**
 * vexfs_index_update_begin - Begin index update operation
 * @lock: Index lock structure
 */
int vexfs_index_update_begin(struct vexfs_index_lock *lock)
{
    unsigned int seq;
    int ret;
    
    if (!lock) {
        return -EINVAL;
    }
    
    /* Acquire writer mutex for exclusion */
    ret = mutex_lock_interruptible(&lock->writer_mutex);
    if (ret) {
        pr_warn("VexFS: Index update interrupted for index %u\n", lock->index_type);
        return ret;
    }
    
    /* Increment writer waiting count */
    atomic_inc(&lock->writer_waiting);
    
    /* Wait for all readers to finish */
    while (atomic_read(&lock->reader_count) > 0) {
        cpu_relax();
        if (need_resched()) {
            cond_resched();
        }
    }
    
    /* Begin sequential lock write */
    seq = write_seqlock(&lock->seq_lock);
    
    /* Update generation number */
    lock->generation++;
    atomic64_inc(&lock->write_ops);
    lock->last_update_time = ktime_get_ns();
    
    /* Decrement writer waiting count */
    atomic_dec(&lock->writer_waiting);
    
    pr_debug("VexFS: Began index update for index %u (generation: %u)\n",
             lock->index_type, lock->generation);
    
    return 0;
}

/**
 * vexfs_index_update_end - End index update operation
 * @lock: Index lock structure
 */
int vexfs_index_update_end(struct vexfs_index_lock *lock)
{
    if (!lock) {
        return -EINVAL;
    }
    
    /* End sequential lock write */
    write_sequnlock(&lock->seq_lock);
    
    /* Signal completion to waiting readers */
    complete_all(&lock->writer_done);
    
    /* Release writer mutex */
    mutex_unlock(&lock->writer_mutex);
    
    /* Synchronize RCU to ensure all readers see the update */
    synchronize_rcu();
    
    pr_debug("VexFS: Ended index update for index %u (generation: %u)\n",
             lock->index_type, lock->generation);
    
    return 0;
}

/* 🔥 LOCK-FREE ALGORITHMS 🔥 */

/**
 * vexfs_lockfree_init_ctx - Initialize lock-free operation context
 * @ctx: Context structure to initialize
 */
int vexfs_lockfree_init_ctx(struct vexfs_lockfree_ctx *ctx)
{
    if (!ctx) {
        return -EINVAL;
    }
    
    atomic_set(&ctx->retry_count, 0);
    atomic_set(&ctx->backoff_delay, VEXFS_LOCKFREE_BACKOFF_MIN_NS);
    atomic64_set(&ctx->operation_id, 
                 atomic64_inc_return(&vexfs_global_lock_manager->lockfree_operation_id));
    ctx->start_time = ktime_get_ns();
    ctx->cpu_id = smp_processor_id();
    ctx->numa_node = numa_node_id();
    
    /* Initialize statistics */
    atomic64_set(&ctx->stats.attempts, 0);
    atomic64_set(&ctx->stats.successes, 0);
    atomic64_set(&ctx->stats.failures, 0);
    atomic64_set(&ctx->stats.contentions, 0);
    
    pr_debug("VexFS: Initialized lock-free context %llu on CPU %u\n",
             atomic64_read(&ctx->operation_id), ctx->cpu_id);
    
    return 0;
}

/**
 * vexfs_lockfree_cas - Compare-and-swap operation with retry logic
 * @target: Target atomic variable
 * @expected: Expected value
 * @new_value: New value to set
 * @ctx: Lock-free operation context
 */
bool vexfs_lockfree_cas(atomic64_t *target, u64 expected, u64 new_value,
                        struct vexfs_lockfree_ctx *ctx)
{
    bool success;
    
    if (!target || !ctx) {
        return false;
    }
    
    /* Increment attempt counter */
    atomic64_inc(&ctx->stats.attempts);
    this_cpu_inc(vexfs_lockfree_stats.cas_attempts);
    
    /* Perform compare-and-swap */
    success = atomic64_cmpxchg(target, expected, new_value) == expected;
    
    if (success) {
        atomic64_inc(&ctx->stats.successes);
        this_cpu_inc(vexfs_lockfree_stats.cas_successes);
        pr_debug("VexFS: CAS succeeded: %llu -> %llu\n", expected, new_value);
    } else {
        atomic64_inc(&ctx->stats.failures);
        atomic64_inc(&ctx->stats.contentions);
        pr_debug("VexFS: CAS failed: expected %llu, got %llu\n", 
                 expected, atomic64_read(target));
    }
    
    return success;
}

/**
 * vexfs_lockfree_faa - Fetch-and-add operation
 * @target: Target atomic variable
 * @increment: Value to add
 * @ctx: Lock-free operation context
 */
u64 vexfs_lockfree_faa(atomic64_t *target, u64 increment,
                       struct vexfs_lockfree_ctx *ctx)
{
    u64 old_value;
    
    if (!target || !ctx) {
        return 0;
    }
    
    /* Increment attempt counter */
    atomic64_inc(&ctx->stats.attempts);
    this_cpu_inc(vexfs_lockfree_stats.faa_operations);
    
    /* Perform fetch-and-add */
    old_value = atomic64_fetch_add(increment, target);
    
    atomic64_inc(&ctx->stats.successes);
    
    pr_debug("VexFS: FAA: %llu + %llu = %llu\n", 
             old_value, increment, old_value + increment);
    
    return old_value;
}

/**
 * vexfs_lockfree_xchg - Exchange operation
 * @target: Target atomic variable
 * @new_value: New value to set
 * @ctx: Lock-free operation context
 */
u64 vexfs_lockfree_xchg(atomic64_t *target, u64 new_value,
                        struct vexfs_lockfree_ctx *ctx)
{
    u64 old_value;
    
    if (!target || !ctx) {
        return 0;
    }
    
    /* Increment attempt counter */
    atomic64_inc(&ctx->stats.attempts);
    this_cpu_inc(vexfs_lockfree_stats.xchg_operations);
    
    /* Perform exchange */
    old_value = atomic64_xchg(target, new_value);
    
    atomic64_inc(&ctx->stats.successes);
    
    pr_debug("VexFS: XCHG: %llu -> %llu\n", old_value, new_value);
    
    return old_value;
}

/**
 * vexfs_lockfree_retry - Check if operation should be retried
 * @ctx: Lock-free operation context
 */
bool vexfs_lockfree_retry(struct vexfs_lockfree_ctx *ctx)
{
    int retry_count;
    
    if (!ctx) {
        return false;
    }
    
    retry_count = atomic_inc_return(&ctx->retry_count);
    
    /* Check retry limit */
    if (retry_count >= VEXFS_LOCKFREE_RETRY_MAX) {
        pr_warn("VexFS: Lock-free operation %llu exceeded retry limit (%d)\n",
                atomic64_read(&ctx->operation_id), retry_count);
        return false;
    }
    
    /* Perform exponential backoff */
    vexfs_lockfree_backoff(ctx);
    
    pr_debug("VexFS: Lock-free retry %d for operation %llu\n",
             retry_count, atomic64_read(&ctx->operation_id));
    
    return true;
}

/**
 * vexfs_lockfree_backoff - Perform exponential backoff
 * @ctx: Lock-free operation context
 */
void vexfs_lockfree_backoff(struct vexfs_lockfree_ctx *ctx)
{
    int current_delay, new_delay;
    
    if (!ctx) {
        return;
    }
    
    current_delay = atomic_read(&ctx->backoff_delay);
    
    /* Exponential backoff with jitter */
    new_delay = min(current_delay * 2, VEXFS_LOCKFREE_BACKOFF_MAX_NS);
    new_delay += prandom_u32() % (new_delay / 4); /* Add 25% jitter */
    
    atomic_set(&ctx->backoff_delay, new_delay);
    
    /* Perform the actual delay */
    if (current_delay < 1000) {
        /* Short delays: busy wait */
        ndelay(current_delay);
    } else if (current_delay < 10000) {
        /* Medium delays: CPU relax */
        cpu_relax();
    } else {
        /* Long delays: yield CPU */
        cond_resched();
    }
    
    this_cpu_inc(vexfs_lockfree_stats.backoff_events);
    
    pr_debug("VexFS: Lock-free backoff: %d ns -> %d ns\n", current_delay, new_delay);
}

/* 🔥 NUMA-AWARE LOCK CACHING 🔥 */

/**
 * vexfs_numa_lock_cache_init - Initialize NUMA lock cache
 * @cache: Cache structure to initialize
 * @numa_node: NUMA node ID
 */
int vexfs_numa_lock_cache_init(struct vexfs_numa_lock_cache *cache, u32 numa_node)
{
    int i, ret;
    
    if (!cache) {
        return -EINVAL;
    }
    
    spin_lock_init(&cache->cache_lock);
    
    /* Initialize hash buckets */
    for (i = 0; i < VEXFS_NUMA_LOCK_CACHE_SIZE; i++) {
        INIT_HLIST_HEAD(&cache->lock_hash[i]);
    }
    
    atomic_set(&cache->cache_size, 0);
    atomic_set(&cache->hit_count, 0);
    atomic_set(&cache->miss_count, 0);
    cache->numa_node = numa_node;
    
    /* Initialize per-CPU counter */
    ret = percpu_counter_init(&cache->active_locks, 0, GFP_KERNEL);
    if (ret) {
        pr_err("VexFS: Failed to initialize NUMA cache counter for node %u\n", numa_node);
        return ret;
    }
    
    pr_info("VexFS: Initialized NUMA lock cache for node %u\n", numa_node);
    
    return 0;
}

/**
 * vexfs_numa_lock_cache_cleanup - Cleanup NUMA lock cache
 * @cache: Cache structure to cleanup
 */
void vexfs_numa_lock_cache_cleanup(struct vexfs_numa_lock_cache *cache)
{
    struct vexfs_vector_lock *lock;
    struct hlist_node *tmp;
    int i;
    
    if (!cache) {
        return;
    }
    
    pr_info("VexFS: Cleaning up NUMA lock cache for node %u\n", cache->numa_node);
    
    /* Clear all hash buckets */
    spin_lock(&cache->cache_lock);
    for (i = 0; i < VEXFS_NUMA_LOCK_CACHE_SIZE; i++) {
        hlist_for_each_entry_safe(lock, tmp, &cache->lock_hash[i], hash_node) {
            hlist_del(&lock->hash_node);
            /* Note: Don't free the lock here, it's managed by the main hash table */
        }
    }
    spin_unlock(&cache->cache_lock);
    
    /* Cleanup per-CPU counter */
    percpu_counter_destroy(&cache->active_locks);
    
    pr_info("VexFS: NUMA cache cleanup completed for node %u (final size: %d)\n",
            cache->numa_node, atomic_read(&cache->cache_size));
}

/**
 * vexfs_numa_lock_cache_get - Get lock from NUMA cache
 * @cache: NUMA cache
 * @vector_id: Vector ID to look up
 */
struct vexfs_vector_lock *vexfs_numa_lock_cache_get(struct vexfs_numa_lock_cache *cache,
                                                    u64 vector_id)
{
    struct vexfs_vector_lock *lock = NULL;
    struct hlist_head *head;
    u32 hash;
    
    if (!cache) {
        return NULL;
    }
    
    hash = vexfs_lock_hash_vector_id(vector_id) % VEXFS_NUMA_LOCK_CACHE_SIZE;
    head = &cache->lock_hash[hash];
    
    spin_lock(&cache->cache_lock);
    hlist_for_each_entry(lock, head, hash_node) {
        if (lock->vector_id == vector_id && lock->numa_node == cache->numa_node) {
            atomic_inc(&lock->ref_count);
            atomic_inc(&cache->hit_count);
            spin_unlock(&cache->cache_lock);
            
            pr_debug("VexFS: NUMA cache hit for vector %llu on node %u\n",
                     vector_id, cache->numa_node);
            return lock;
        }
    }
    spin_unlock(&cache->cache_lock);
    
    atomic_inc(&cache->miss_count);
    
    pr_debug("VexFS: NUMA cache miss for vector %llu on node %u\n",
             vector_id, cache->numa_node);
    
    return NULL;
}

/**
 * vexfs_numa_lock_cache_put - Put lock into NUMA cache
 * @cache: NUMA cache
 * @lock: Lock to cache
 */
int vexfs_numa_lock_cache_put(struct vexfs_numa_lock_cache *cache,
                              struct vexfs_vector_lock *lock)
{
    struct hlist_head *head;
    u32 hash;
    
    if (!cache || !lock) {
        return -EINVAL;
    }
    
    /* Only cache locks from the same NUMA node */
    if (lock->numa_node != cache->numa_node) {
        return -EINVAL;
    }
    
    hash = vexfs_lock_hash_vector_id(lock->vector_id) % VEXFS_NUMA_LOCK_CACHE_SIZE;
    head = &cache->lock_hash[hash];
    
    spin_lock(&cache->cache_lock);
    
    /* Check if already cached */
    struct vexfs_vector_lock *existing;
    hlist_for_each_entry(existing, head, hash_node) {
        if (existing->vector_id == lock->vector_id) {
            spin_unlock(&cache->cache_lock);
            return 0; /* Already cached */
        }
    }
    
    /* Add to cache */
    hlist_add_head(&lock->hash_node, head);
    atomic_inc(&cache->cache_size);
    percpu_counter_inc(&cache->active_locks);
    
    spin_unlock(&cache->cache_lock);
    
    pr_debug("VexFS: Cached vector lock %llu in NUMA node %u (cache size: %d)\n",
             lock->vector_id, cache->numa_node, atomic_read(&cache->cache_size));
    
    return 0;
}

/* 🔥 LOCK-FREE STATISTICS 🔥 */

/**
 * vexfs_lockfree_get_stats - Get lock-free operation statistics
 * @stats: Output statistics structure
 */
int vexfs_lockfree_get_stats(struct vexfs_lockfree_stats *stats)
{
    int cpu;
    
    if (!stats) {
        return -EINVAL;
    }
    
    memset(stats, 0, sizeof(*stats));
    
    /* Aggregate per-CPU statistics */
    for_each_online_cpu(cpu) {
        stats->cas_attempts += atomic64_read(&per_cpu(vexfs_lockfree_stats.cas_attempts, cpu));
        stats->cas_successes += atomic64_read(&per_cpu(vexfs_lockfree_stats.cas_successes, cpu));
        stats->faa_operations += atomic64_read(&per_cpu(vexfs_lockfree_stats.faa_operations, cpu));
        stats->xchg_operations += atomic64_read(&per_cpu(vexfs_lockfree_stats.xchg_operations, cpu));
        stats->backoff_events += atomic64_read(&per_cpu(vexfs_lockfree_stats.backoff_events, cpu));
    }
    
    return 0;
}

/**
 * vexfs_lockfree_reset_stats - Reset lock-free operation statistics
 */
void vexfs_lockfree_reset_stats(void)
{
    int cpu;
    
    for_each_online_cpu(cpu) {
        atomic64_set(&per_cpu(vexfs_lockfree_stats.cas_attempts, cpu), 0);
        atomic64_set(&per_cpu(vexfs_lockfree_stats.cas_successes, cpu), 0);
        atomic64_set(&per_cpu(vexfs_lockfree_stats.faa_operations, cpu), 0);
        atomic64_set(&per_cpu(vexfs_lockfree_stats.xchg_operations, cpu), 0);
        atomic64_set(&per_cpu(vexfs_lockfree_stats.backoff_events, cpu), 0);
    }
    
    pr_info("VexFS: Lock-free statistics reset\n");
}

/* Lock-free statistics structure for external use */
struct vexfs_lockfree_stats {
    u64 cas_attempts;
    u64 cas_successes;
    u64 faa_operations;
    u64 xchg_operations;
    u64 backoff_events;
};