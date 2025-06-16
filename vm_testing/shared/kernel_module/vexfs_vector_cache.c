/*
 * VexFS v2.0 Vector Data Caching System Implementation
 * 
 * Specialized caching system for vector data that maintains SIMD alignment
 * and optimizes for vector access patterns with NUMA awareness.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/mm.h>
#include <linux/gfp.h>
#include <linux/numa.h>
#include <linux/cpu.h>
#include <linux/workqueue.h>
#include <linux/hash.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/completion.h>
#include <linux/time.h>
#include <asm/fpu/api.h>
#include <asm/simd.h>

#include "vexfs_v2_vector_cache.h"
#include "vexfs_v2_monitoring.h"

/* Global vector cache instance */
static struct vexfs_vector_cache *global_vector_cache = NULL;

/* Cache configuration parameters */
static u32 cache_max_entries = VEXFS_VECTOR_CACHE_MAX_ENTRIES;
static u32 cache_max_memory_mb = VEXFS_VECTOR_CACHE_SIZE_MB;
static u32 cache_prefetch_window = VEXFS_VECTOR_CACHE_PREFETCH;

module_param(cache_max_entries, uint, 0644);
MODULE_PARM_DESC(cache_max_entries, "Maximum number of cached vectors");

module_param(cache_max_memory_mb, uint, 0644);
MODULE_PARM_DESC(cache_max_memory_mb, "Maximum cache memory in MB");

module_param(cache_prefetch_window, uint, 0644);
MODULE_PARM_DESC(cache_prefetch_window, "Prefetch window size");

/*
 * Hash function for vector cache lookup
 */
static inline u32 vexfs_cache_hash(u64 vector_id, u32 hash_table_size)
{
    return hash_64(vector_id, ilog2(hash_table_size));
}

/*
 * SIMD-aligned memory allocation with NUMA awareness
 */
void *vexfs_alloc_simd_aligned(size_t size, u32 alignment, int numa_node)
{
    void *ptr;
    size_t aligned_size;
    struct page **pages;
    u32 page_count;
    int i;
    
    /* Ensure alignment is power of 2 and at least cache line size */
    if (!alignment || (alignment & (alignment - 1)))
        alignment = VEXFS_VECTOR_CACHE_LINE_SIZE;
    
    if (alignment < VEXFS_VECTOR_CACHE_LINE_SIZE)
        alignment = VEXFS_VECTOR_CACHE_LINE_SIZE;
    
    /* Calculate aligned size */
    aligned_size = ALIGN(size, alignment);
    page_count = (aligned_size + PAGE_SIZE - 1) >> PAGE_SHIFT;
    
    /* Allocate pages on specified NUMA node */
    pages = vexfs_alloc_vector_pages(page_count, numa_node);
    if (!pages)
        return NULL;
    
    /* Map pages to get virtual address */
    ptr = vmap(pages, page_count, VM_MAP, PAGE_KERNEL);
    if (!ptr) {
        vexfs_free_vector_pages(pages, page_count);
        return NULL;
    }
    
    /* Ensure SIMD alignment */
    if (!IS_ALIGNED((unsigned long)ptr, alignment)) {
        vunmap(ptr);
        vexfs_free_vector_pages(pages, page_count);
        return NULL;
    }
    
    /* Zero the memory for security */
    memset(ptr, 0, aligned_size);
    
    /* Update statistics */
    if (global_vector_cache) {
        atomic64_inc(&global_vector_cache->stats.simd_aligned_allocs);
        if (numa_node == numa_node_id())
            atomic64_inc(&global_vector_cache->stats.numa_local_allocs);
    }
    
    return ptr;
}

/*
 * Free SIMD-aligned memory
 */
void vexfs_free_simd_aligned(void *ptr, size_t size)
{
    if (!ptr)
        return;
    
    vunmap(ptr);
    /* Note: Pages are freed by the caller who maintains the page array */
}

/*
 * Check if pointer is SIMD-aligned
 */
int vexfs_is_simd_aligned(void *ptr, u32 alignment)
{
    return IS_ALIGNED((unsigned long)ptr, alignment);
}

/*
 * Allocate NUMA-aware pages for vector data
 */
struct page **vexfs_alloc_vector_pages(u32 page_count, int numa_node)
{
    struct page **pages;
    int i;
    gfp_t gfp_flags = GFP_KERNEL | __GFP_ZERO | __GFP_NOWARN;
    
    /* Allocate page pointer array */
    pages = kmalloc_array(page_count, sizeof(struct page *), GFP_KERNEL);
    if (!pages)
        return NULL;
    
    /* Allocate pages on specified NUMA node */
    for (i = 0; i < page_count; i++) {
        if (numa_node >= 0 && numa_node < MAX_NUMNODES)
            pages[i] = alloc_pages_node(numa_node, gfp_flags, 0);
        else
            pages[i] = alloc_page(gfp_flags);
        
        if (!pages[i]) {
            /* Free already allocated pages */
            while (--i >= 0)
                __free_page(pages[i]);
            kfree(pages);
            return NULL;
        }
    }
    
    return pages;
}

/*
 * Free NUMA-aware pages
 */
void vexfs_free_vector_pages(struct page **pages, u32 page_count)
{
    int i;
    
    if (!pages)
        return;
    
    for (i = 0; i < page_count; i++) {
        if (pages[i])
            __free_page(pages[i]);
    }
    
    kfree(pages);
}

/*
 * Get optimal NUMA node for current CPU
 */
int vexfs_get_optimal_numa_node(void)
{
    return numa_node_id();
}

/*
 * Allocate and initialize a cache entry
 */
struct vexfs_cache_entry *vexfs_cache_entry_alloc(u64 vector_id, u32 vector_size,
                                                 u16 dimensions, u8 element_type,
                                                 u32 alignment, int numa_node)
{
    struct vexfs_cache_entry *entry;
    u32 page_count;
    
    /* Allocate entry structure */
    entry = kzalloc(sizeof(struct vexfs_cache_entry), GFP_KERNEL);
    if (!entry)
        return NULL;
    
    /* Initialize entry fields */
    entry->vector_id = vector_id;
    entry->vector_size = vector_size;
    entry->dimensions = dimensions;
    entry->element_type = element_type;
    entry->alignment = alignment;
    entry->numa_node = numa_node;
    entry->flags = VEXFS_CACHE_ENTRY_VALID;
    
    /* Calculate page count */
    page_count = (vector_size + PAGE_SIZE - 1) >> PAGE_SHIFT;
    entry->page_count = page_count;
    
    /* Allocate NUMA-aware pages */
    entry->pages = vexfs_alloc_vector_pages(page_count, numa_node);
    if (!entry->pages) {
        kfree(entry);
        return NULL;
    }
    
    /* Allocate SIMD-aligned vector data */
    entry->vector_data = vexfs_alloc_simd_aligned(vector_size, alignment, numa_node);
    if (!entry->vector_data) {
        vexfs_free_vector_pages(entry->pages, page_count);
        kfree(entry);
        return NULL;
    }
    
    /* Set SIMD alignment flag */
    if (vexfs_is_simd_aligned(entry->vector_data, alignment))
        entry->flags |= VEXFS_CACHE_ENTRY_SIMD;
    
    /* Initialize synchronization */
    atomic_set(&entry->ref_count, 1);
    spin_lock_init(&entry->entry_lock);
    init_completion(&entry->io_completion);
    
    /* Initialize access tracking */
    entry->last_access_time = ktime_get_ns();
    entry->access_count = 0;
    entry->access_pattern = VEXFS_ACCESS_PATTERN_RANDOM;
    
    /* Initialize lists */
    INIT_LIST_HEAD(&entry->lru_list);
    INIT_HLIST_NODE(&entry->hash_node);
    
    return entry;
}

/*
 * Free a cache entry
 */
void vexfs_cache_entry_free(struct vexfs_cache_entry *entry)
{
    if (!entry)
        return;
    
    /* Ensure no references remain */
    WARN_ON(atomic_read(&entry->ref_count) != 0);
    
    /* Free vector data */
    if (entry->vector_data) {
        vexfs_free_simd_aligned(entry->vector_data, entry->vector_size);
        entry->vector_data = NULL;
    }
    
    /* Free pages */
    if (entry->pages) {
        vexfs_free_vector_pages(entry->pages, entry->page_count);
        entry->pages = NULL;
    }
    
    /* Free entry structure */
    kfree(entry);
}

/*
 * Get reference to cache entry
 */
void vexfs_cache_entry_get(struct vexfs_cache_entry *entry)
{
    if (entry)
        atomic_inc(&entry->ref_count);
}

/*
 * Put reference to cache entry
 */
void vexfs_cache_entry_put(struct vexfs_cache_entry *entry)
{
    if (entry && atomic_dec_and_test(&entry->ref_count))
        vexfs_cache_entry_free(entry);
}

/*
 * Initialize hot cache
 */
int vexfs_hot_cache_init(struct vexfs_hot_cache *hot_cache, u32 capacity)
{
    hot_cache->entries = kzalloc(capacity * sizeof(struct vexfs_cache_entry *), 
                                GFP_KERNEL);
    if (!hot_cache->entries)
        return -ENOMEM;
    
    hot_cache->capacity = capacity;
    hot_cache->count = 0;
    hot_cache->promotion_threshold = 10; /* Promote after 10 accesses */
    spin_lock_init(&hot_cache->lock);
    
    /* Initialize statistics */
    atomic64_set(&hot_cache->promotions, 0);
    atomic64_set(&hot_cache->demotions, 0);
    atomic64_set(&hot_cache->hot_hits, 0);
    
    return 0;
}

/*
 * Cleanup hot cache
 */
void vexfs_hot_cache_cleanup(struct vexfs_hot_cache *hot_cache)
{
    int i;
    
    if (!hot_cache->entries)
        return;
    
    /* Release all hot cache entries */
    spin_lock(&hot_cache->lock);
    for (i = 0; i < hot_cache->count; i++) {
        if (hot_cache->entries[i]) {
            hot_cache->entries[i]->flags &= ~VEXFS_CACHE_ENTRY_HOT;
            vexfs_cache_entry_put(hot_cache->entries[i]);
            hot_cache->entries[i] = NULL;
        }
    }
    hot_cache->count = 0;
    spin_unlock(&hot_cache->lock);
    
    kfree(hot_cache->entries);
    hot_cache->entries = NULL;
}

/*
 * Promote entry to hot cache
 */
int vexfs_hot_cache_promote(struct vexfs_vector_cache *cache, 
                           struct vexfs_cache_entry *entry)
{
    struct vexfs_hot_cache *hot_cache = &cache->hot_cache;
    int ret = 0;
    
    spin_lock(&hot_cache->lock);
    
    /* Check if already in hot cache */
    if (entry->flags & VEXFS_CACHE_ENTRY_HOT) {
        spin_unlock(&hot_cache->lock);
        return 0;
    }
    
    /* Check if hot cache is full */
    if (hot_cache->count >= hot_cache->capacity) {
        /* Demote least recently used entry */
        if (hot_cache->count > 0) {
            struct vexfs_cache_entry *lru_entry = hot_cache->entries[0];
            lru_entry->flags &= ~VEXFS_CACHE_ENTRY_HOT;
            vexfs_cache_entry_put(lru_entry);
            
            /* Shift entries */
            memmove(&hot_cache->entries[0], &hot_cache->entries[1],
                   (hot_cache->count - 1) * sizeof(struct vexfs_cache_entry *));
            hot_cache->count--;
            atomic64_inc(&hot_cache->demotions);
        }
    }
    
    /* Add to hot cache */
    if (hot_cache->count < hot_cache->capacity) {
        vexfs_cache_entry_get(entry);
        hot_cache->entries[hot_cache->count++] = entry;
        entry->flags |= VEXFS_CACHE_ENTRY_HOT;
        atomic64_inc(&hot_cache->promotions);
        ret = 0;
    } else {
        ret = -ENOSPC;
    }
    
    spin_unlock(&hot_cache->lock);
    return ret;
}

/*
 * Detect access pattern for a vector
 */
u32 vexfs_detect_access_pattern(struct vexfs_cache_entry *entry, u64 vector_id)
{
    static u64 last_vector_id = 0;
    u32 pattern = VEXFS_ACCESS_PATTERN_RANDOM;
    
    /* Detect sequential access */
    if (vector_id == last_vector_id + 1)
        pattern |= VEXFS_ACCESS_PATTERN_SEQUENTIAL;
    
    /* Detect search pattern (high frequency access) */
    if (entry && entry->search_frequency > entry->access_count / 2)
        pattern |= VEXFS_ACCESS_PATTERN_SEARCH;
    
    /* Detect batch pattern (multiple accesses in short time) */
    if (entry && entry->batch_frequency > 5)
        pattern |= VEXFS_ACCESS_PATTERN_BATCH;
    
    last_vector_id = vector_id;
    return pattern;
}

/*
 * Update access pattern for entry
 */
void vexfs_update_access_pattern(struct vexfs_cache_entry *entry, u32 pattern)
{
    if (!entry)
        return;
    
    spin_lock(&entry->entry_lock);
    
    entry->access_pattern = pattern;
    entry->access_count++;
    entry->last_access_time = ktime_get_ns();
    
    /* Update frequency counters */
    if (pattern & VEXFS_ACCESS_PATTERN_SEARCH)
        entry->search_frequency++;
    
    if (pattern & VEXFS_ACCESS_PATTERN_BATCH)
        entry->batch_frequency++;
    
    /* Calculate prefetch score */
    entry->prefetch_score = entry->access_count;
    if (pattern & VEXFS_ACCESS_PATTERN_SEQUENTIAL)
        entry->prefetch_score *= 2;
    
    spin_unlock(&entry->entry_lock);
}

/*
 * Lookup vector in cache
 */
struct vexfs_cache_entry *vexfs_cache_lookup(struct vexfs_vector_cache *cache, 
                                            u64 vector_id)
{
    struct vexfs_cache_entry *entry = NULL;
    struct hlist_head *head;
    u32 hash;
    u32 pattern;
    
    if (!cache)
        return NULL;
    
    hash = vexfs_cache_hash(vector_id, cache->hash_table_size);
    head = &cache->hash_table[hash];
    
    read_lock(&cache->cache_lock);
    
    /* Search hash table */
    hlist_for_each_entry(entry, head, hash_node) {
        if (entry->vector_id == vector_id) {
            /* Found entry - update access tracking */
            vexfs_cache_entry_get(entry);
            
            pattern = vexfs_detect_access_pattern(entry, vector_id);
            vexfs_update_access_pattern(entry, pattern);
            
            /* Move to front of LRU list */
            spin_lock(&cache->lru_lock);
            list_move(&entry->lru_list, &cache->lru_list);
            spin_unlock(&cache->lru_lock);
            
            /* Update statistics */
            atomic64_inc(&cache->stats.cache_hits);
            if (entry->flags & VEXFS_CACHE_ENTRY_HOT)
                atomic64_inc(&cache->stats.hot_cache_hits);
            
            /* Check for hot cache promotion */
            if (!(entry->flags & VEXFS_CACHE_ENTRY_HOT) &&
                entry->access_count >= cache->hot_cache.promotion_threshold) {
                vexfs_hot_cache_promote(cache, entry);
            }
            
            read_unlock(&cache->cache_lock);
            return entry;
        }
    }
    
    read_unlock(&cache->cache_lock);
    
    /* Cache miss */
    atomic64_inc(&cache->stats.cache_misses);
    return NULL;
}

/*
 * Insert vector into cache
 */
struct vexfs_cache_entry *vexfs_cache_insert(struct vexfs_vector_cache *cache,
                                            u64 vector_id, void *vector_data,
                                            u32 vector_size, u16 dimensions,
                                            u8 element_type)
{
    struct vexfs_cache_entry *entry;
    struct hlist_head *head;
    u32 hash;
    int numa_node;
    
    if (!cache || !vector_data)
        return NULL;
    
    /* Check if cache is full */
    if (vexfs_cache_is_full(cache)) {
        vexfs_cache_evict_lru(cache, 1);
    }
    
    /* Get optimal NUMA node */
    numa_node = vexfs_get_optimal_numa_node();
    
    /* Allocate new cache entry */
    entry = vexfs_cache_entry_alloc(vector_id, vector_size, dimensions,
                                   element_type, cache->default_alignment,
                                   numa_node);
    if (!entry)
        return NULL;
    
    /* Copy vector data */
    memcpy(entry->vector_data, vector_data, vector_size);
    
    /* Insert into hash table */
    hash = vexfs_cache_hash(vector_id, cache->hash_table_size);
    head = &cache->hash_table[hash];
    
    write_lock(&cache->cache_lock);
    hlist_add_head(&entry->hash_node, head);
    
    /* Add to LRU list */
    spin_lock(&cache->lru_lock);
    list_add(&entry->lru_list, &cache->lru_list);
    spin_unlock(&cache->lru_lock);
    
    /* Update counters */
    atomic_inc(&cache->entry_count);
    atomic64_add(vector_size, &cache->memory_used);
    atomic64_inc(&cache->stats.cache_insertions);
    
    write_unlock(&cache->cache_lock);
    
    return entry;
}

/*
 * Evict LRU entries from cache
 */
void vexfs_cache_evict_lru(struct vexfs_vector_cache *cache, u32 count)
{
    struct vexfs_cache_entry *entry, *tmp;
    u32 evicted = 0;
    
    if (!cache)
        return;
    
    write_lock(&cache->cache_lock);
    spin_lock(&cache->lru_lock);
    
    /* Evict from tail of LRU list */
    list_for_each_entry_safe_reverse(entry, tmp, &cache->lru_list, lru_list) {
        if (evicted >= count)
            break;
        
        /* Skip hot cache entries */
        if (entry->flags & VEXFS_CACHE_ENTRY_HOT)
            continue;
        
        /* Skip locked entries */
        if (entry->flags & VEXFS_CACHE_ENTRY_LOCKED)
            continue;
        
        /* Remove from lists */
        list_del(&entry->lru_list);
        hlist_del(&entry->hash_node);
        
        /* Update counters */
        atomic_dec(&cache->entry_count);
        atomic64_sub(entry->vector_size, &cache->memory_used);
        atomic64_inc(&cache->stats.cache_evictions);
        
        /* Free entry */
        vexfs_cache_entry_put(entry);
        evicted++;
    }
    
    spin_unlock(&cache->lru_lock);
    write_unlock(&cache->cache_lock);
}

/*
 * Create vector cache
 */
struct vexfs_vector_cache *vexfs_vector_cache_create(u32 max_entries, u32 max_memory_mb)
{
    struct vexfs_vector_cache *cache;
    u32 hash_table_size;
    int i;
    
    cache = kzalloc(sizeof(struct vexfs_vector_cache), GFP_KERNEL);
    if (!cache)
        return NULL;
    
    /* Initialize configuration */
    cache->max_entries = max_entries;
    cache->max_memory_mb = max_memory_mb;
    cache->default_alignment = VEXFS_SIMD_ALIGN_32; /* AVX alignment */
    cache->numa_node_count = num_online_nodes();
    
    /* Initialize hash table */
    hash_table_size = roundup_pow_of_two(max_entries / 4);
    cache->hash_table_size = hash_table_size;
    cache->hash_table = kzalloc(hash_table_size * sizeof(struct hlist_head), 
                               GFP_KERNEL);
    if (!cache->hash_table) {
        kfree(cache);
        return NULL;
    }
    
    for (i = 0; i < hash_table_size; i++)
        INIT_HLIST_HEAD(&cache->hash_table[i]);
    
    /* Initialize data structures */
    cache->entry_tree = RB_ROOT;
    INIT_LIST_HEAD(&cache->lru_list);
    
    /* Initialize locks */
    rwlock_init(&cache->cache_lock);
    spin_lock_init(&cache->lru_lock);
    spin_lock_init(&cache->hash_lock);
    
    /* Initialize counters */
    atomic_set(&cache->entry_count, 0);
    atomic64_set(&cache->memory_used, 0);
    
    /* Initialize hot cache */
    if (vexfs_hot_cache_init(&cache->hot_cache, max_entries / 10) != 0) {
        kfree(cache->hash_table);
        kfree(cache);
        return NULL;
    }
    
    /* Initialize statistics */
    memset(&cache->stats, 0, sizeof(cache->stats));
    
    /* Set global cache */
    global_vector_cache = cache;
    
    return cache;
}

/*
 * Destroy vector cache
 */
void vexfs_vector_cache_destroy(struct vexfs_vector_cache *cache)
{
    struct vexfs_cache_entry *entry, *tmp;
    int i;
    
    if (!cache)
        return;
    
    /* Clear global cache reference */
    if (global_vector_cache == cache)
        global_vector_cache = NULL;
    
    /* Cleanup hot cache */
    vexfs_hot_cache_cleanup(&cache->hot_cache);
    
    /* Free all cache entries */
    write_lock(&cache->cache_lock);
    list_for_each_entry_safe(entry, tmp, &cache->lru_list, lru_list) {
        list_del(&entry->lru_list);
        hlist_del(&entry->hash_node);
        vexfs_cache_entry_put(entry);
    }
    write_unlock(&cache->cache_lock);
    
    /* Free hash table */
    kfree(cache->hash_table);
    
    /* Free cache structure */
    kfree(cache);
}

/*
 * Perform vector operation with FPU context
 */
int vexfs_cache_vector_operation(struct vexfs_cache_entry *entry,
                                void (*operation)(void *data, size_t size),
                                bool use_simd)
{
    if (!entry || !operation)
        return -EINVAL;
    
    /* Acquire FPU context if using SIMD */
    if (use_simd && may_use_simd()) {
        kernel_fpu_begin();
        operation(entry->vector_data, entry->vector_size);
        kernel_fpu_end();
        
        /* Update SIMD operation counter */
        if (global_vector_cache)
            atomic64_inc(&global_vector_cache->stats.simd_operations);
    } else {
        operation(entry->vector_data, entry->vector_size);
    }
    
    return 0;
}

/*
 * Update cache statistics
 */
void vexfs_cache_update_stats(struct vexfs_vector_cache *cache, 
                             struct vexfs_cache_entry *entry, bool hit)
{
    if (!cache)
        return;
    
    if (hit) {
        atomic64_inc(&cache->stats.cache_hits);
        if (entry && (entry->flags & VEXFS_CACHE_ENTRY_HOT))
            atomic64_inc(&cache->stats.hot_cache_hits);
    } else {
        atomic64_inc(&cache->stats.cache_misses);
    }
    
    /* Update memory statistics */
    atomic64_set(&cache->stats.total_memory_used, atomic64_read(&cache->memory_used));
    if (atomic64_read(&cache->memory_used) > atomic64_read(&cache->stats.peak_memory_used))
        atomic64_set(&cache->stats.peak_memory_used, atomic64_read(&cache->memory_used));
}

/*
 * Print cache statistics
 */
void vexfs_cache_print_stats(struct vexfs_vector_cache *cache)
{
    if (!cache)
        return;
    
    printk(KERN_INFO "VexFS Vector Cache Statistics:\n");
    printk(KERN_INFO "  Entries: %d/%d\n", 
           atomic_read(&cache->entry_count), cache->max_entries);
    printk(KERN_INFO "  Memory: %llu/%u MB\n", 
           vexfs_cache_memory_usage_mb(cache), cache->max_memory_mb);
    printk(KERN_INFO "  Hit Rate: %llu%%\n", vexfs_cache_hit_rate(cache));
    printk(KERN_INFO "  Cache Hits: %llu\n", 
           atomic64_read(&cache->stats.cache_hits));
    printk(KERN_INFO "  Cache Misses: %llu\n", 
           atomic64_read(&cache->stats.cache_misses));
    printk(KERN_INFO "  Hot Cache Hits: %llu\n", 
           atomic64_read(&cache->stats.hot_cache_hits));
    printk(KERN_INFO "  SIMD Operations: %llu\n", 
           atomic64_read(&cache->stats.simd_operations));
    printk(KERN_INFO "  NUMA Local Allocs: %llu\n", 
           atomic64_read(&cache->stats.numa_local_allocs));
}

/* Module initialization */
static int __init vexfs_vector_cache_init_module(void)
{
    printk(KERN_INFO "VexFS Vector Cache: Initializing\n");
    
    /* Create global cache instance */
    global_vector_cache = vexfs_vector_cache_create(cache_max_entries, 
                                                   cache_max_memory_mb);
    if (!global_vector_cache) {
        printk(KERN_ERR "VexFS Vector Cache: Failed to create cache\n");
        return -ENOMEM;
    }
    
    printk(KERN_INFO "VexFS Vector Cache: Initialized with %u entries, %u MB\n",
           cache_max_entries, cache_max_memory_mb);
    
    return 0;
}

/* Module cleanup */
static void __exit vexfs_vector_cache_exit_module(void)
{
    printk(KERN_INFO "VexFS Vector Cache: Cleaning up\n");
    
    if (global_vector_cache) {
        vexfs_cache_print_stats(global_vector_cache);
        vexfs_vector_cache_destroy(global_vector_cache);
        global_vector_cache = NULL;
    }
    
    printk(KERN_INFO "VexFS Vector Cache: Cleanup complete\n");
}

module_init(vexfs_vector_cache_init_module);
module_exit(vexfs_vector_cache_exit_module);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 Vector Data Caching System");
MODULE_VERSION("1.0.0");