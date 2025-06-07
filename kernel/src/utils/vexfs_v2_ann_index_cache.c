/*
 * VexFS v2.0 ANN Index Caching System Implementation
 * 
 * This file implements the specialized caching system for Approximate Nearest
 * Neighbor (ANN) index structures. It provides high-performance caching with:
 * - RCU-protected concurrent access
 * - NUMA-aware memory allocation
 * - Specialized kmem_cache instances
 * - Cache coherency mechanisms
 * - Priority-based caching
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/mm.h>
#include <linux/gfp.h>
#include <linux/atomic.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/completion.h>
#include <linux/workqueue.h>
#include <linux/delay.h>
#include <linux/jiffies.h>
#include <linux/hash.h>
#include <linux/hashtable.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/rcupdate.h>
#include <linux/numa.h>
#include <linux/cpu.h>
#include <linux/cpumask.h>
#include <linux/time.h>
#include <linux/ktime.h>

#ifdef __KERNEL__
#include "vexfs_v2_ann_index_cache.h"
#else
#include "vexfs_v2_ann_index_cache.h"
#endif

/* Global ANN cache instance */
static struct vexfs_ann_cache *global_ann_cache = NULL;
static DEFINE_MUTEX(ann_cache_global_mutex);

/* Cache entry slab names for different index types */
static const char *cache_names[VEXFS_ANN_INDEX_TYPE_COUNT] = {
    "vexfs_hnsw_node",
    "vexfs_hnsw_layer", 
    "vexfs_pq_codebook",
    "vexfs_ivf_centroid",
    "vexfs_lsh_hash_table",
    "vexfs_lsh_bucket",
    "vexfs_search_result",
    "vexfs_graph_metadata"
};

/* Cache entry sizes for different index types */
static const size_t cache_sizes[VEXFS_ANN_INDEX_TYPE_COUNT] = {
    sizeof(struct vexfs_ann_cache_entry) + 1024,   /* HNSW node */
    sizeof(struct vexfs_ann_cache_entry) + 512,    /* HNSW layer */
    sizeof(struct vexfs_ann_cache_entry) + 4096,   /* PQ codebook */
    sizeof(struct vexfs_ann_cache_entry) + 2048,   /* IVF centroid */
    sizeof(struct vexfs_ann_cache_entry) + 8192,   /* LSH hash table */
    sizeof(struct vexfs_ann_cache_entry) + 256,    /* LSH bucket */
    sizeof(struct vexfs_ann_cache_entry) + 1024,   /* Search result */
    sizeof(struct vexfs_ann_cache_entry) + 128     /* Graph metadata */
};

/*
 * Initialize the ANN index cache system
 */
int vexfs_ann_cache_init(struct vexfs_ann_cache **cache,
                        struct vexfs_memory_manager *mm,
                        struct vexfs_vector_cache *vector_cache)
{
    struct vexfs_ann_cache *ann_cache;
    int ret = 0;
    int i;
    
    if (!cache || !mm) {
        return -EINVAL;
    }
    
    /* Allocate main cache structure */
    ann_cache = kzalloc(sizeof(struct vexfs_ann_cache), GFP_KERNEL);
    if (!ann_cache) {
        return -ENOMEM;
    }
    
    /* Initialize configuration */
    ann_cache->max_memory_usage = VEXFS_ANN_CACHE_SIZE_MB * 1024 * 1024;
    ann_cache->max_entries = VEXFS_ANN_CACHE_MAX_ENTRIES;
    ann_cache->rcu_grace_period_ms = VEXFS_ANN_CACHE_RCU_GRACE_MS;
    ann_cache->hot_threshold = VEXFS_ANN_HOT_THRESHOLD;
    ann_cache->cold_threshold = VEXFS_ANN_COLD_THRESHOLD;
    ann_cache->prefetch_window = 16;
    ann_cache->coherency_check_interval_ms = 5000;
    
    /* Initialize data structures */
    hash_init(ann_cache->cache_hash);
    ann_cache->cache_tree = RB_ROOT;
    INIT_LIST_HEAD(&ann_cache->lru_list);
    INIT_LIST_HEAD(&ann_cache->hot_list);
    
    /* Initialize synchronization */
    spin_lock_init(&ann_cache->cache_lock);
    mutex_init(&ann_cache->update_mutex);
    init_rwsem(&ann_cache->coherency_sem);
    
    /* Set up memory management integration */
    ann_cache->mm = mm;
    ann_cache->vector_cache = vector_cache;
    
    /* Initialize NUMA awareness */
    ann_cache->preferred_numa_node = numa_node_id();
    cpumask_copy(&ann_cache->allowed_cpus, cpu_online_mask);
    
    /* Create specialized kmem_cache instances */
    for (i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        ann_cache->caches[i] = kmem_cache_create(
            cache_names[i],
            cache_sizes[i],
            0,  /* align */
            SLAB_HWCACHE_ALIGN | SLAB_RECLAIM_ACCOUNT,
            NULL  /* ctor */
        );
        
        if (!ann_cache->caches[i]) {
            pr_err("VexFS ANN Cache: Failed to create kmem_cache for %s\n", 
                   cache_names[i]);
            ret = -ENOMEM;
            goto cleanup_caches;
        }
    }
    
    /* Create maintenance workqueue */
    ann_cache->maintenance_wq = alloc_workqueue("vexfs_ann_cache",
                                               WQ_MEM_RECLAIM | WQ_UNBOUND, 0);
    if (!ann_cache->maintenance_wq) {
        ret = -ENOMEM;
        goto cleanup_caches;
    }
    
    /* Initialize work structures */
    INIT_DELAYED_WORK(&ann_cache->cleanup_work, vexfs_ann_cache_cleanup_work);
    INIT_DELAYED_WORK(&ann_cache->coherency_work, vexfs_ann_cache_coherency_work);
    INIT_DELAYED_WORK(&ann_cache->prefetch_work, vexfs_ann_cache_prefetch_work);
    
    /* Initialize statistics */
    memset(&ann_cache->stats, 0, sizeof(ann_cache->stats));
    
    /* Schedule background maintenance */
    queue_delayed_work(ann_cache->maintenance_wq, &ann_cache->cleanup_work,
                      msecs_to_jiffies(10000));
    queue_delayed_work(ann_cache->maintenance_wq, &ann_cache->coherency_work,
                      msecs_to_jiffies(ann_cache->coherency_check_interval_ms));
    
    *cache = ann_cache;
    
    mutex_lock(&ann_cache_global_mutex);
    global_ann_cache = ann_cache;
    mutex_unlock(&ann_cache_global_mutex);
    
    pr_info("VexFS ANN Cache: Initialized with %zu MB capacity, %u max entries\n",
            ann_cache->max_memory_usage / (1024 * 1024), ann_cache->max_entries);
    
    return 0;
    
cleanup_caches:
    for (i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        if (ann_cache->caches[i]) {
            kmem_cache_destroy(ann_cache->caches[i]);
        }
    }
    kfree(ann_cache);
    return ret;
}

/*
 * Destroy the ANN index cache system
 */
void vexfs_ann_cache_destroy(struct vexfs_ann_cache *cache)
{
    struct vexfs_ann_cache_entry *entry, *tmp;
    int i;
    
    if (!cache) {
        return;
    }
    
    mutex_lock(&ann_cache_global_mutex);
    if (global_ann_cache == cache) {
        global_ann_cache = NULL;
    }
    mutex_unlock(&ann_cache_global_mutex);
    
    /* Cancel background work */
    if (cache->maintenance_wq) {
        cancel_delayed_work_sync(&cache->cleanup_work);
        cancel_delayed_work_sync(&cache->coherency_work);
        cancel_delayed_work_sync(&cache->prefetch_work);
        destroy_workqueue(cache->maintenance_wq);
    }
    
    /* Flush all cache entries */
    vexfs_ann_cache_flush(cache);
    
    /* Wait for RCU grace period */
    synchronize_rcu();
    
    /* Destroy kmem_cache instances */
    for (i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        if (cache->caches[i]) {
            kmem_cache_destroy(cache->caches[i]);
        }
    }
    
    /* Clean up any remaining entries */
    list_for_each_entry_safe(entry, tmp, &cache->lru_list, lru_list) {
        list_del(&entry->lru_list);
        kfree(entry);
    }
    
    kfree(cache);
    
    pr_info("VexFS ANN Cache: Destroyed cache instance\n");
}

/*
 * Allocate a new cache entry
 */
static struct vexfs_ann_cache_entry *vexfs_ann_cache_entry_alloc(
    struct vexfs_ann_cache *cache,
    enum vexfs_ann_index_type type)
{
    struct vexfs_ann_cache_entry *entry;
    
    if (type >= VEXFS_ANN_INDEX_TYPE_COUNT) {
        return NULL;
    }
    
    entry = kmem_cache_alloc(cache->caches[type], GFP_KERNEL);
    if (!entry) {
        return NULL;
    }
    
    /* Initialize entry */
    memset(entry, 0, sizeof(*entry));
    INIT_LIST_HEAD(&entry->lru_list);
    INIT_HLIST_NODE(&entry->hash_node);
    entry->type = type;
    atomic_set(&entry->ref_count, 1);
    spin_lock_init(&entry->entry_lock);
    mutex_init(&entry->update_mutex);
    init_completion(&entry->update_completion);
    entry->creation_time = ktime_get_ns();
    entry->last_access_time = entry->creation_time;
    entry->numa_node = cache->preferred_numa_node;
    atomic_set(&entry->access_count, 0);
    atomic_set(&entry->query_frequency, 0);
    atomic_set(&entry->coherency_state, 1);
    entry->version = 1;
    
    return entry;
}

/*
 * Free a cache entry
 */
static void vexfs_ann_cache_entry_free(struct vexfs_ann_cache_entry *entry)
{
    struct vexfs_ann_cache *cache = global_ann_cache;
    
    if (!entry || !cache) {
        return;
    }
    
    /* Free allocated pages */
    if (entry->pages && entry->page_count > 0) {
        int i;
        for (i = 0; i < entry->page_count; i++) {
            if (entry->pages[i]) {
                __free_page(entry->pages[i]);
            }
        }
        kfree(entry->pages);
    }
    
    /* Free index data if allocated separately */
    if (entry->index_data && entry->structure_size > cache_sizes[entry->type]) {
        vfree(entry->index_data);
    }
    
    /* Return to appropriate kmem_cache */
    kmem_cache_free(cache->caches[entry->type], entry);
}

/*
 * RCU callback for freeing cache entries
 */
void vexfs_ann_cache_rcu_free(struct rcu_head *rcu)
{
    struct vexfs_ann_cache_entry *entry;
    
    entry = container_of(rcu, struct vexfs_ann_cache_entry, rcu_head);
    vexfs_ann_cache_entry_free(entry);
}

/*
 * Get reference to cache entry
 */
void vexfs_ann_cache_entry_get(struct vexfs_ann_cache_entry *entry)
{
    if (entry) {
        atomic_inc(&entry->ref_count);
    }
}

/*
 * Put reference to cache entry
 */
void vexfs_ann_cache_entry_put(struct vexfs_ann_cache_entry *entry)
{
    if (entry && atomic_dec_and_test(&entry->ref_count)) {
        call_rcu(&entry->rcu_head, vexfs_ann_cache_rcu_free);
    }
}

/*
 * Hash function for cache entries
 */
static inline u32 vexfs_ann_cache_hash(u64 index_id)
{
    return hash_64(index_id, VEXFS_ANN_CACHE_HASH_BITS);
}

/*
 * Insert entry into red-black tree
 */
static int vexfs_ann_cache_rb_insert(struct vexfs_ann_cache *cache,
                                    struct vexfs_ann_cache_entry *entry)
{
    struct rb_node **new = &cache->cache_tree.rb_node;
    struct rb_node *parent = NULL;
    struct vexfs_ann_cache_entry *this;
    
    while (*new) {
        this = rb_entry(*new, struct vexfs_ann_cache_entry, rb_node);
        parent = *new;
        
        if (entry->index_id < this->index_id) {
            new = &((*new)->rb_left);
        } else if (entry->index_id > this->index_id) {
            new = &((*new)->rb_right);
        } else {
            return -EEXIST;
        }
    }
    
    rb_link_node(&entry->rb_node, parent, new);
    rb_insert_color(&entry->rb_node, &cache->cache_tree);
    
    return 0;
}

/*
 * Remove entry from red-black tree
 */
static void vexfs_ann_cache_rb_remove(struct vexfs_ann_cache *cache,
                                     struct vexfs_ann_cache_entry *entry)
{
    rb_erase(&entry->rb_node, &cache->cache_tree);
}

/*
 * Find entry in red-black tree
 */
static struct vexfs_ann_cache_entry *vexfs_ann_cache_rb_find(
    struct vexfs_ann_cache *cache, u64 index_id)
{
    struct rb_node *node = cache->cache_tree.rb_node;
    struct vexfs_ann_cache_entry *entry;
    
    while (node) {
        entry = rb_entry(node, struct vexfs_ann_cache_entry, rb_node);
        
        if (index_id < entry->index_id) {
            node = node->rb_left;
        } else if (index_id > entry->index_id) {
            node = node->rb_right;
        } else {
            return entry;
        }
    }
    
    return NULL;
}

/*
 * Lookup cache entry
 */
int vexfs_ann_cache_lookup(struct vexfs_ann_cache *cache,
                          u64 index_id,
                          enum vexfs_ann_index_type type,
                          struct vexfs_ann_cache_entry **entry)
{
    struct vexfs_ann_cache_entry *found = NULL;
    unsigned long flags;
    u32 hash_key;
    
    if (!cache || !entry) {
        return -EINVAL;
    }
    
    hash_key = vexfs_ann_cache_hash(index_id);
    
    rcu_read_lock();
    spin_lock_irqsave(&cache->cache_lock, flags);
    
    /* First try hash table lookup */
    hash_for_each_possible_rcu(cache->cache_hash, found, hash_node, hash_key) {
        if (found->index_id == index_id && found->type == type) {
            if (vexfs_ann_cache_entry_is_valid(found)) {
                vexfs_ann_cache_entry_get(found);
                
                /* Update access statistics */
                found->last_access_time = ktime_get_ns();
                atomic_inc(&found->access_count);
                atomic_inc(&found->query_frequency);
                atomic64_inc(&cache->stats.cache_hits);
                
                /* Move to front of LRU list */
                list_move(&found->lru_list, &cache->lru_list);
                
                spin_unlock_irqrestore(&cache->cache_lock, flags);
                rcu_read_unlock();
                
                *entry = found;
                return 0;
            }
        }
    }
    
    spin_unlock_irqrestore(&cache->cache_lock, flags);
    rcu_read_unlock();
    
    /* Cache miss */
    atomic64_inc(&cache->stats.cache_misses);
    atomic64_inc(&cache->stats.type_misses[type]);
    
    *entry = NULL;
    return -ENOENT;
}

/*
 * Insert cache entry
 */
int vexfs_ann_cache_insert(struct vexfs_ann_cache *cache,
                          struct vexfs_ann_cache_entry *entry)
{
    unsigned long flags;
    u32 hash_key;
    int ret = 0;
    
    if (!cache || !entry) {
        return -EINVAL;
    }
    
    hash_key = vexfs_ann_cache_hash(entry->index_id);
    
    spin_lock_irqsave(&cache->cache_lock, flags);
    
    /* Check if entry already exists */
    if (vexfs_ann_cache_rb_find(cache, entry->index_id)) {
        ret = -EEXIST;
        goto unlock;
    }
    
    /* Check cache capacity */
    if (atomic64_read(&cache->stats.total_entries) >= cache->max_entries) {
        /* TODO: Implement LRU eviction */
        ret = -ENOSPC;
        goto unlock;
    }
    
    /* Insert into data structures */
    ret = vexfs_ann_cache_rb_insert(cache, entry);
    if (ret) {
        goto unlock;
    }
    
    hash_add_rcu(cache->cache_hash, &entry->hash_node, hash_key);
    list_add(&entry->lru_list, &cache->lru_list);
    
    /* Update statistics */
    atomic64_inc(&cache->stats.total_entries);
    atomic64_inc(&cache->stats.active_entries);
    atomic64_inc(&cache->stats.type_counts[entry->type]);
    atomic64_add(&cache->stats.memory_usage, entry->structure_size);
    
    /* Set entry as valid */
    entry->flags |= VEXFS_ANN_CACHE_VALID;
    
unlock:
    spin_unlock_irqrestore(&cache->cache_lock, flags);
    return ret;
}

/*
 * Remove cache entry
 */
int vexfs_ann_cache_remove(struct vexfs_ann_cache *cache, u64 index_id)
{
    struct vexfs_ann_cache_entry *entry;
    unsigned long flags;
    int ret = 0;
    
    if (!cache) {
        return -EINVAL;
    }
    
    spin_lock_irqsave(&cache->cache_lock, flags);
    
    entry = vexfs_ann_cache_rb_find(cache, index_id);
    if (!entry) {
        ret = -ENOENT;
        goto unlock;
    }
    
    /* Remove from data structures */
    vexfs_ann_cache_rb_remove(cache, entry);
    hash_del_rcu(&entry->hash_node);
    list_del(&entry->lru_list);
    
    /* Update statistics */
    atomic64_dec(&cache->stats.total_entries);
    atomic64_dec(&cache->stats.active_entries);
    atomic64_dec(&cache->stats.type_counts[entry->type]);
    atomic64_sub(&cache->stats.memory_usage, entry->structure_size);
    
    /* Mark entry as invalid */
    entry->flags &= ~VEXFS_ANN_CACHE_VALID;
    
    spin_unlock_irqrestore(&cache->cache_lock, flags);
    
    /* Release reference (will trigger RCU free) */
    vexfs_ann_cache_entry_put(entry);
    
    return 0;
    
unlock:
    spin_unlock_irqrestore(&cache->cache_lock, flags);
    return ret;
}

/*
 * Get cache entry (lookup + allocate if not found)
 */
struct vexfs_ann_cache_entry *vexfs_ann_cache_get(struct vexfs_ann_cache *cache,
                                                  u64 index_id,
                                                  enum vexfs_ann_index_type type)
{
    struct vexfs_ann_cache_entry *entry = NULL;
    int ret;
    
    if (!cache) {
        return NULL;
    }
    
    /* Try to find existing entry */
    ret = vexfs_ann_cache_lookup(cache, index_id, type, &entry);
    if (ret == 0) {
        return entry;
    }
    
    /* Allocate new entry */
    entry = vexfs_ann_cache_entry_alloc(cache, type);
    if (!entry) {
        return NULL;
    }
    
    entry->index_id = index_id;
    
    /* Insert into cache */
    ret = vexfs_ann_cache_insert(cache, entry);
    if (ret) {
        vexfs_ann_cache_entry_put(entry);
        return NULL;
    }
    
    return entry;
}

/*
 * Put cache entry (decrease reference count)
 */
int vexfs_ann_cache_put(struct vexfs_ann_cache *cache,
                       struct vexfs_ann_cache_entry *entry)
{
    if (!cache || !entry) {
        return -EINVAL;
    }
    
    vexfs_ann_cache_entry_put(entry);
    return 0;
}

/*
 * Flush all cache entries
 */
int vexfs_ann_cache_flush(struct vexfs_ann_cache *cache)
{
    struct vexfs_ann_cache_entry *entry, *tmp;
    unsigned long flags;
    
    if (!cache) {
        return -EINVAL;
    }
    
    spin_lock_irqsave(&cache->cache_lock, flags);
    
    list_for_each_entry_safe(entry, tmp, &cache->lru_list, lru_list) {
        /* Remove from data structures */
        vexfs_ann_cache_rb_remove(cache, entry);
        hash_del_rcu(&entry->hash_node);
        list_del(&entry->lru_list);
        
        /* Mark as invalid */
        entry->flags &= ~VEXFS_ANN_CACHE_VALID;
        
        /* Release reference */
        vexfs_ann_cache_entry_put(entry);
    }
    
    /* Reset statistics */
    atomic64_set(&cache->stats.total_entries, 0);
    atomic64_set(&cache->stats.active_entries, 0);
    atomic64_set(&cache->stats.memory_usage, 0);
    
    spin_unlock_irqrestore(&cache->cache_lock, flags);
    
    /* Wait for RCU grace period */
    synchronize_rcu();
    
    return 0;
}

/*
 * Get cache statistics
 */
int vexfs_ann_cache_get_stats(struct vexfs_ann_cache *cache,
                             struct vexfs_ann_cache_stats *stats)
{
    if (!cache || !stats) {
        return -EINVAL;
    }
    
    memcpy(stats, &cache->stats, sizeof(*stats));
    return 0;
}

/*
 * Print cache statistics
 */
void vexfs_ann_cache_print_stats(struct vexfs_ann_cache *cache)
{
    u64 hits, misses, hit_ratio;
    
    if (!cache) {
        return;
    }
    
    hits = atomic64_read(&cache->stats.cache_hits);
    misses = atomic64_read(&cache->stats.cache_misses);
    hit_ratio = vexfs_ann_cache_get_hit_ratio(cache);
    
    pr_info("VexFS ANN Cache Statistics:\n");
    pr_info("  Total entries: %llu\n", atomic64_read(&cache->stats.total_entries));
    pr_info("  Active entries: %llu\n", atomic64_read(&cache->stats.active_entries));
    pr_info("  Memory usage: %llu MB\n", 
            atomic64_read(&cache->stats.memory_usage) / (1024 * 1024));
    pr_info("  Cache hits: %llu\n", hits);
    pr_info("  Cache misses: %llu\n", misses);
    pr_info("  Hit ratio: %llu%%\n", hit_ratio);
    pr_info("  Evictions: %llu\n", atomic64_read(&cache->stats.cache_evictions));
    pr_info("  NUMA local hits: %llu\n", atomic64_read(&cache->stats.numa_local_hits));
    pr_info("  NUMA remote hits: %llu\n", atomic64_read(&cache->stats.numa_remote_hits));
}

/*
 * Background cleanup work
 */
void vexfs_ann_cache_cleanup_work(struct work_struct *work)
{
    struct vexfs_ann_cache *cache;
    struct delayed_work *dwork;
    
    dwork = container_of(work, struct delayed_work, work);
    cache = container_of(dwork, struct vexfs_ann_cache, cleanup_work);
    
    /* TODO: Implement LRU cleanup logic */
    
    /* Reschedule cleanup work */
    queue_delayed_work(cache->maintenance_wq, &cache->cleanup_work,
                      msecs_to_jiffies(10000));
}

/*
 * Background coherency check work
 */
void vexfs_ann_cache_coherency_work(struct work_struct *work)
{
    struct vexfs_ann_cache *cache;
    struct delayed_work *dwork;
    
    dwork = container_of(work, struct delayed_work, work);
    cache = container_of(dwork, struct vexfs_ann_cache, coherency_work);
    
    /* TODO: Implement coherency check logic */
    
    /* Reschedule coherency work */
    queue_delayed_work(cache->maintenance_wq, &cache->coherency_work,
                      msecs_to_jiffies(cache->coherency_check_interval_ms));
}

/*
 * Background prefetch work
 */
void vexfs_ann_cache_prefetch_work(struct work_struct *work)
{
    struct vexfs_ann_cache *cache;
    struct delayed_work *dwork;
    
    dwork = container_of(work, struct delayed_work, work);
    cache = container_of(dwork, struct vexfs_ann_cache, prefetch_work);
    
    /* TODO: Implement prefetch logic */
    
    /* Reschedule prefetch work */
    queue_delayed_work(cache->maintenance_wq, &cache->prefetch_work,
                      msecs_to_jiffies(5000));
}

/* Module initialization and cleanup */
static int __init vexfs_ann_cache_module_init(void)
{
    pr_info("VexFS ANN Index Cache module loaded\n");
    return 0;
}

static void __exit vexfs_ann_cache_module_exit(void)
{
    if (global_ann_cache) {
        vexfs_ann_cache_destroy(global_ann_cache);
    }
    pr_info("VexFS ANN Index Cache module unloaded\n");
}

module_init(vexfs_ann_cache_module_init);
module_exit(vexfs_ann_cache_module_exit);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 ANN Index Caching System");
MODULE_VERSION("2.0.0");