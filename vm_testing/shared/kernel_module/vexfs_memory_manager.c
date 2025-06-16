/*
 * VexFS v2.0 Optimized Memory Management System Implementation
 * 
 * This implementation provides specialized memory management strategies
 * for efficient vector data handling in kernel space, including:
 * 
 * 1. Large contiguous allocations with alloc_pages()
 * 2. NUMA-aware memory placement using alloc_pages_node()
 * 3. SIMD-aligned memory regions with appropriate flags
 * 4. Efficient memory mapping for user-space access
 * 5. Memory pools for frequently allocated vector sizes
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/mm.h>
#include <linux/gfp.h>
#include <linux/numa.h>
#include <linux/cpu.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/list.h>
#include <linux/rbtree.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/mman.h>
#include <linux/highmem.h>
#include <linux/swap.h>
#include <linux/memcontrol.h>
#include <asm/page.h>
#include <asm/cacheflush.h>
#include <asm/tlbflush.h>

#include "vexfs_v2_memory_manager.h"
#include "vexfs_v2_monitoring.h"

/*
 * Global memory manager instance
 */
struct vexfs_memory_manager *vexfs_mm = NULL;
EXPORT_SYMBOL(vexfs_mm);

/*
 * Memory pool configurations
 */
static const struct {
    enum vexfs_mm_pool_type type;
    size_t entry_size;
    size_t max_entries;
    unsigned int alignment;
} vexfs_mm_pool_configs[VEXFS_MM_POOL_COUNT] = {
    [VEXFS_MM_POOL_VECTOR_SMALL]   = { VEXFS_MM_POOL_VECTOR_SMALL,   4096,     1024, 64 },
    [VEXFS_MM_POOL_VECTOR_MEDIUM]  = { VEXFS_MM_POOL_VECTOR_MEDIUM,  65536,    512,  64 },
    [VEXFS_MM_POOL_VECTOR_LARGE]   = { VEXFS_MM_POOL_VECTOR_LARGE,   1048576,  128,  64 },
    [VEXFS_MM_POOL_VECTOR_HUGE]    = { VEXFS_MM_POOL_VECTOR_HUGE,    4194304,  32,   64 },
    [VEXFS_MM_POOL_METADATA]       = { VEXFS_MM_POOL_METADATA,       1024,     2048, 8  },
    [VEXFS_MM_POOL_SEARCH_RESULTS] = { VEXFS_MM_POOL_SEARCH_RESULTS, 8192,     256,  32 },
    [VEXFS_MM_POOL_GRAPH_NODES]    = { VEXFS_MM_POOL_GRAPH_NODES,    512,      4096, 8  },
    [VEXFS_MM_POOL_HASH_TABLES]    = { VEXFS_MM_POOL_HASH_TABLES,    16384,    128,  32 },
};

/*
 * Initialize the memory manager
 */
int vexfs_mm_init(void)
{
    int ret;
    int i;
    
    pr_info("VexFS: Initializing optimized memory manager\n");
    
    /* Allocate memory manager structure */
    vexfs_mm = kzalloc(sizeof(struct vexfs_memory_manager), GFP_KERNEL);
    if (!vexfs_mm) {
        pr_err("VexFS: Failed to allocate memory manager\n");
        return -ENOMEM;
    }
    
    /* Initialize synchronization primitives */
    mutex_init(&vexfs_mm->manager_mutex);
    spin_lock_init(&vexfs_mm->large_allocs_lock);
    spin_lock_init(&vexfs_mm->user_mappings_lock);
    
    /* Initialize data structures */
    vexfs_mm->large_allocs = RB_ROOT;
    INIT_LIST_HEAD(&vexfs_mm->user_mappings);
    
    /* Initialize NUMA information */
    vexfs_mm->numa_node_count = num_online_nodes();
    vexfs_mm->current_numa_node = numa_node_id();
    vexfs_mm->numa_aware = (vexfs_mm->numa_node_count > 1);
    
    for (i = 0; i < MAX_NUMNODES; i++) {
        vexfs_mm->numa_nodes[i].node_id = i;
        vexfs_mm->numa_nodes[i].total_memory = 0;
        vexfs_mm->numa_nodes[i].available_memory = 0;
        vexfs_mm->numa_nodes[i].allocated_memory = 0;
        atomic64_set(&vexfs_mm->numa_nodes[i].allocation_count, 0);
        atomic64_set(&vexfs_mm->numa_nodes[i].allocation_failures, 0);
        INIT_LIST_HEAD(&vexfs_mm->numa_nodes[i].pool_list);
    }
    
    /* Initialize configuration */
    vexfs_mm->large_pages_enabled = true;
    vexfs_mm->default_alignment = 64;
    vexfs_mm->max_allocation_size = VEXFS_MM_MAX_POOL_SIZE;
    
    /* Initialize statistics */
    memset(&vexfs_mm->stats, 0, sizeof(vexfs_mm->stats));
    
    /* Create workqueue for background tasks */
    vexfs_mm->workqueue = create_singlethread_workqueue("vexfs_mm");
    if (!vexfs_mm->workqueue) {
        pr_err("VexFS: Failed to create memory manager workqueue\n");
        ret = -ENOMEM;
        goto err_free_manager;
    }
    
    /* Initialize delayed work */
    INIT_DELAYED_WORK(&vexfs_mm->cleanup_work, vexfs_mm_cleanup_worker);
    INIT_DELAYED_WORK(&vexfs_mm->defrag_work, vexfs_mm_defrag_worker);
    
    /* Initialize memory pools */
    ret = vexfs_mm_init_pools();
    if (ret) {
        pr_err("VexFS: Failed to initialize memory pools: %d\n", ret);
        goto err_destroy_workqueue;
    }
    
    /* Mark as initialized */
    atomic_set(&vexfs_mm->initialized, 1);
    
    /* Schedule background tasks */
    queue_delayed_work(vexfs_mm->workqueue, &vexfs_mm->cleanup_work, 
                       msecs_to_jiffies(30000)); /* 30 seconds */
    queue_delayed_work(vexfs_mm->workqueue, &vexfs_mm->defrag_work,
                       msecs_to_jiffies(60000)); /* 60 seconds */
    
    pr_info("VexFS: Memory manager initialized successfully\n");
    pr_info("VexFS: NUMA aware: %s, Nodes: %d, Current node: %d\n",
            vexfs_mm->numa_aware ? "yes" : "no",
            vexfs_mm->numa_node_count,
            vexfs_mm->current_numa_node);
    
    return 0;
    
err_destroy_workqueue:
    destroy_workqueue(vexfs_mm->workqueue);
err_free_manager:
    kfree(vexfs_mm);
    vexfs_mm = NULL;
    return ret;
}

/*
 * Cleanup the memory manager
 */
void vexfs_mm_exit(void)
{
    if (!vexfs_mm || !atomic_read(&vexfs_mm->initialized))
        return;
        
    pr_info("VexFS: Shutting down memory manager\n");
    
    /* Mark as not initialized */
    atomic_set(&vexfs_mm->initialized, 0);
    
    /* Cancel background work */
    cancel_delayed_work_sync(&vexfs_mm->cleanup_work);
    cancel_delayed_work_sync(&vexfs_mm->defrag_work);
    
    /* Destroy workqueue */
    if (vexfs_mm->workqueue) {
        destroy_workqueue(vexfs_mm->workqueue);
        vexfs_mm->workqueue = NULL;
    }
    
    /* Cleanup memory pools */
    vexfs_mm_cleanup_pools();
    
    /* Print final statistics */
    vexfs_mm_print_stats();
    
    /* Free memory manager */
    kfree(vexfs_mm);
    vexfs_mm = NULL;
    
    pr_info("VexFS: Memory manager shutdown complete\n");
}

/*
 * Initialize memory pools
 */
int vexfs_mm_init_pools(void)
{
    int i, ret;
    
    for (i = 0; i < VEXFS_MM_POOL_COUNT; i++) {
        ret = vexfs_mm_pool_init(
            vexfs_mm_pool_configs[i].type,
            vexfs_mm_pool_configs[i].entry_size,
            vexfs_mm_pool_configs[i].max_entries,
            vexfs_mm->current_numa_node
        );
        if (ret) {
            pr_err("VexFS: Failed to initialize pool %d: %d\n", i, ret);
            return ret;
        }
    }
    
    pr_info("VexFS: Initialized %d memory pools\n", VEXFS_MM_POOL_COUNT);
    return 0;
}

/*
 * Cleanup memory pools
 */
void vexfs_mm_cleanup_pools(void)
{
    int i;
    
    for (i = 0; i < VEXFS_MM_POOL_COUNT; i++) {
        vexfs_mm_pool_cleanup(i);
    }
}

/*
 * Core allocation function
 */
void *vexfs_mm_alloc(size_t size, enum vexfs_mm_pool_type pool_type, u32 flags)
{
    void *ptr = NULL;
    int numa_node;
    bool numa_local = false;
    bool simd_aligned = false;
    
    if (!vexfs_mm || !atomic_read(&vexfs_mm->initialized))
        return NULL;
        
    if (size == 0 || size > vexfs_mm->max_allocation_size)
        return NULL;
    
    /* Determine NUMA node */
    if (flags & VEXFS_MM_FLAG_NUMA_LOCAL) {
        numa_node = vexfs_mm_get_best_numa_node();
        numa_local = (numa_node == vexfs_mm->current_numa_node);
    } else {
        numa_node = NUMA_NO_NODE;
    }
    
    /* Try pool allocation first for appropriate sizes */
    if (pool_type < VEXFS_MM_POOL_COUNT && 
        size <= vexfs_mm_pool_configs[pool_type].entry_size) {
        ptr = vexfs_mm_pool_alloc(pool_type);
        if (ptr) {
            atomic64_inc(&vexfs_mm->stats.pool_hits);
            simd_aligned = vexfs_mm_is_aligned(ptr, vexfs_mm_pool_configs[pool_type].alignment);
            goto success;
        }
        atomic64_inc(&vexfs_mm->stats.pool_misses);
    }
    
    /* Use large page allocation for big requests */
    if (flags & VEXFS_MM_FLAG_CONTIGUOUS || vexfs_mm_should_use_large_pages(size)) {
        ptr = vexfs_mm_alloc_large_pages(size, numa_node, flags);
        if (ptr) {
            atomic64_inc(&vexfs_mm->stats.large_page_allocs);
            simd_aligned = (flags & VEXFS_MM_FLAG_SIMD_ALIGN);
            goto success;
        }
    }
    
    /* Fall back to aligned allocation */
    if (flags & VEXFS_MM_FLAG_SIMD_ALIGN) {
        unsigned int alignment = vexfs_mm->default_alignment;
        ptr = vexfs_mm_alloc_aligned(size, alignment, flags);
        if (ptr) {
            simd_aligned = true;
            goto success;
        }
    }
    
    /* Final fallback to regular allocation */
    if (numa_node != NUMA_NO_NODE) {
        ptr = vexfs_mm_alloc_numa(size, numa_node, flags);
    } else {
        gfp_t gfp_flags = GFP_KERNEL;
        if (flags & VEXFS_MM_FLAG_ZERO_FILL)
            gfp_flags |= __GFP_ZERO;
        if (flags & VEXFS_MM_FLAG_HIGH_PRIORITY)
            gfp_flags |= __GFP_HIGH;
            
        ptr = kmalloc(size, gfp_flags);
    }
    
success:
    if (ptr) {
        vexfs_mm_update_stats(size, numa_local, simd_aligned, true);
        
        /* Zero fill if requested and not already done */
        if ((flags & VEXFS_MM_FLAG_ZERO_FILL) && !simd_aligned) {
            memset(ptr, 0, size);
        }
    } else {
        vexfs_mm_update_stats(size, numa_local, simd_aligned, false);
        vexfs_mm_handle_allocation_failure(size, flags);
    }
    
    return ptr;
}

/*
 * SIMD-aligned allocation
 */
void *vexfs_mm_alloc_aligned(size_t size, unsigned int alignment, u32 flags)
{
    void *ptr;
    gfp_t gfp_flags = GFP_KERNEL;
    
    if (!vexfs_mm || !atomic_read(&vexfs_mm->initialized))
        return NULL;
        
    if (size == 0 || alignment == 0 || !is_power_of_2(alignment))
        return NULL;
    
    /* Set GFP flags based on requirements */
    if (flags & VEXFS_MM_FLAG_ZERO_FILL)
        gfp_flags |= __GFP_ZERO;
    if (flags & VEXFS_MM_FLAG_HIGH_PRIORITY)
        gfp_flags |= __GFP_HIGH;
    if (alignment >= 64)
        gfp_flags |= __GFP_DMA32; /* Ensure proper alignment */
    
    /* Use kmalloc for smaller allocations */
    if (size <= PAGE_SIZE) {
        ptr = kmalloc(size, gfp_flags);
        if (ptr && ((unsigned long)ptr & (alignment - 1))) {
            /* Not properly aligned, need to reallocate */
            kfree(ptr);
            ptr = NULL;
        }
    }
    
    /* Use page allocation for larger or unaligned requests */
    if (!ptr) {
        unsigned int order = max(get_order(size), get_order(alignment));
        struct page *page;
        
        if (flags & VEXFS_MM_FLAG_NUMA_LOCAL) {
            int numa_node = vexfs_mm_get_best_numa_node();
            page = alloc_pages_node(numa_node, gfp_flags, order);
        } else {
            page = alloc_pages(gfp_flags, order);
        }
        
        if (page) {
            ptr = page_address(page);
            atomic64_inc(&vexfs_mm->stats.contiguous_allocs);
        }
    }
    
    return ptr;
}

/*
 * Contiguous allocation using alloc_pages
 */
void *vexfs_mm_alloc_contiguous(size_t size, unsigned int order, u32 flags)
{
    struct page *pages;
    void *ptr;
    gfp_t gfp_flags = GFP_KERNEL;
    int numa_node = NUMA_NO_NODE;
    
    if (!vexfs_mm || !atomic_read(&vexfs_mm->initialized))
        return NULL;
        
    if (size == 0 || order > VEXFS_MM_MAX_ORDER)
        return NULL;
    
    /* Set GFP flags */
    if (flags & VEXFS_MM_FLAG_ZERO_FILL)
        gfp_flags |= __GFP_ZERO;
    if (flags & VEXFS_MM_FLAG_HIGH_PRIORITY)
        gfp_flags |= __GFP_HIGH;
    
    /* Determine NUMA node */
    if (flags & VEXFS_MM_FLAG_NUMA_LOCAL) {
        numa_node = vexfs_mm_get_best_numa_node();
    }
    
    /* Allocate pages */
    if (numa_node != NUMA_NO_NODE) {
        pages = alloc_pages_node(numa_node, gfp_flags, order);
    } else {
        pages = alloc_pages(gfp_flags, order);
    }
    
    if (!pages)
        return NULL;
    
    ptr = page_address(pages);
    
    /* Track large allocation */
    if (order > 0) {
        struct vexfs_mm_large_alloc *large_alloc;
        
        large_alloc = kmalloc(sizeof(*large_alloc), GFP_KERNEL);
        if (large_alloc) {
            large_alloc->ptr = ptr;
            large_alloc->size = size;
            large_alloc->numa_node = numa_node;
            large_alloc->order = order;
            large_alloc->pages = &pages;
            large_alloc->page_count = 1 << order;
            atomic_set(&large_alloc->ref_count, 1);
            large_alloc->allocated_time = ktime_get_ns();
            large_alloc->flags = flags;
            
            /* Add to tracking tree */
            spin_lock(&vexfs_mm->large_allocs_lock);
            /* TODO: Add RB tree insertion logic */
            spin_unlock(&vexfs_mm->large_allocs_lock);
        }
    }
    
    atomic64_inc(&vexfs_mm->stats.contiguous_allocs);
    return ptr;
}

/*
 * NUMA-aware allocation
 */
void *vexfs_mm_alloc_numa(size_t size, int numa_node, u32 flags)
{
    void *ptr;
    gfp_t gfp_flags = GFP_KERNEL;
    
    if (!vexfs_mm || !atomic_read(&vexfs_mm->initialized))
        return NULL;
        
    if (size == 0 || numa_node < 0 || numa_node >= MAX_NUMNODES)
        return NULL;
    
    /* Set GFP flags */
    if (flags & VEXFS_MM_FLAG_ZERO_FILL)
        gfp_flags |= __GFP_ZERO;
    if (flags & VEXFS_MM_FLAG_HIGH_PRIORITY)
        gfp_flags |= __GFP_HIGH;
    
    /* Try node-local allocation first */
    gfp_flags |= __GFP_THISNODE;
    ptr = kmalloc_node(size, gfp_flags, numa_node);
    
    if (!ptr) {
        /* Fall back to any node */
        gfp_flags &= ~__GFP_THISNODE;
        ptr = kmalloc(size, gfp_flags);
        
        if (ptr) {
            atomic64_inc(&vexfs_mm->stats.numa_remote_allocs);
        }
    } else {
        atomic64_inc(&vexfs_mm->stats.numa_local_allocs);
    }
    
    /* Update NUMA statistics */
    vexfs_mm_update_numa_stats(numa_node, size, ptr != NULL);
    
    return ptr;
}

/*
 * Free allocated memory
 */
void vexfs_mm_free(void *ptr)
{
    if (!ptr || !vexfs_mm || !atomic_read(&vexfs_mm->initialized))
        return;
    
    /* Check if it's a vmalloc address */
    if (is_vmalloc_addr(ptr)) {
        vfree(ptr);
        atomic64_inc(&vexfs_mm->stats.total_freed);
        return;
    }
    
    /* Check if it's a large allocation */
    /* TODO: Implement large allocation lookup and cleanup */
    
    /* Regular kfree */
    kfree(ptr);
    atomic64_inc(&vexfs_mm->stats.total_freed);
}

/*
 * SIMD-specific allocation functions
 */
void *vexfs_mm_alloc_simd_sse(size_t size, u32 flags)
{
    return vexfs_mm_alloc_aligned(size, VEXFS_MM_ALIGN_SSE, 
                                  flags | VEXFS_MM_FLAG_SIMD_ALIGN);
}

void *vexfs_mm_alloc_simd_avx(size_t size, u32 flags)
{
    return vexfs_mm_alloc_aligned(size, VEXFS_MM_ALIGN_AVX,
                                  flags | VEXFS_MM_FLAG_SIMD_ALIGN);
}

void *vexfs_mm_alloc_simd_avx512(size_t size, u32 flags)
{
    return vexfs_mm_alloc_aligned(size, VEXFS_MM_ALIGN_AVX512,
                                  flags | VEXFS_MM_FLAG_SIMD_ALIGN);
}

/*
 * Large page allocation
 */
void *vexfs_mm_alloc_large_pages(size_t size, int numa_node, u32 flags)
{
    unsigned int order = get_order(size);
    return vexfs_mm_alloc_contiguous(size, order, flags);
}

/*
 * Free large pages
 */
void vexfs_mm_free_large_pages(void *ptr)
{
    /* TODO: Implement large page specific cleanup */
    vexfs_mm_free(ptr);
}

/*
 * Initialize a memory pool
 */
int vexfs_mm_pool_init(enum vexfs_mm_pool_type type, size_t entry_size,
                       size_t max_entries, int numa_node)
{
    struct vexfs_mm_pool *pool;
    
    if (!vexfs_mm || type >= VEXFS_MM_POOL_COUNT)
        return -EINVAL;
    
    pool = &vexfs_mm->pools[type];
    
    pool->type = type;
    pool->entry_size = entry_size;
    pool->max_entries = max_entries;
    pool->current_entries = 0;
    pool->preferred_numa_node = numa_node;
    pool->alignment = vexfs_mm_pool_configs[type].alignment;
    
    spin_lock_init(&pool->lock);
    INIT_LIST_HEAD(&pool->free_list);
    INIT_LIST_HEAD(&pool->used_list);
    
    atomic64_set(&pool->hits, 0);
    atomic64_set(&pool->misses, 0);
    atomic64_set(&pool->allocations, 0);
    atomic64_set(&pool->deallocations, 0);
    
    return 0;
}

/*
 * Allocate from memory pool
 */
void *vexfs_mm_pool_alloc(enum vexfs_mm_pool_type type)
{
    struct vexfs_mm_pool *pool;
    struct vexfs_mm_pool_entry *entry;
    void *ptr = NULL;
    unsigned long flags;
    
    if (!vexfs_mm || type >= VEXFS_MM_POOL_COUNT)
        return NULL;
    
    pool = &vexfs_mm->pools[type];
    
    spin_lock_irqsave(&pool->lock, flags);
    
    /* Try to get from free list */
    if (!list_empty(&pool->free_list)) {
        entry = list_first_entry(&pool->free_list, 
                                 struct vexfs_mm_pool_entry, list);
        list_del(&entry->list);
        list_add(&entry->list, &pool->used_list);
        
        ptr = entry->ptr;
        atomic_inc(&entry->ref_count);
        entry->last_used = ktime_get_ns();
        
        atomic64_inc(&pool->hits);
        atomic64_inc(&pool->allocations);
    } else {
        atomic64_inc(&pool->misses);
    }
    
    spin_unlock_irqrestore(&pool->lock, flags);
    
    /* Allocate new entry if pool is empty */
    if (!ptr && pool->current_entries < pool->max_entries) {
        entry = kmalloc(sizeof(*entry), GFP_KERNEL);
        if (entry) {
            ptr = vexfs_mm_alloc_aligned(pool->entry_size, pool->alignment,
                                         VEXFS_MM_FLAG_NUMA_LOCAL | VEXFS_MM_FLAG_ZERO_FILL);
            if (ptr) {
                entry->ptr = ptr;
                entry->size = pool->entry_size;
                entry->numa_node = pool->preferred_numa_node;
                entry->alignment = pool->alignment;
                atomic_set(&entry->ref_count, 1);
                entry->last_used = ktime_get_ns();
                
                spin_lock_irqsave(&pool->lock, flags);
                list_add(&entry->list, &pool->used_list);
                pool->current_entries++;
                atomic64_inc(&pool->allocations);
                spin_unlock_irqrestore(&pool->lock, flags);
            } else {
                kfree(entry);
                ptr = NULL;
            }
        }
    }
    
    return ptr;
}

/*
 * Free to memory pool
 */
void vexfs_mm_pool_free(enum vexfs_mm_pool_type type, void *ptr)
{
    struct vexfs_mm_pool *pool;
    struct vexfs_mm_pool_entry *entry, *tmp;
    unsigned long flags;
    bool found = false;
    
    if (!vexfs_mm || !ptr || type >= VEXFS_MM_POOL_COUNT)
        return;
    
    pool = &vexfs_mm->pools[type];
    
    spin_lock_irqsave(&pool->lock, flags);
    
    /* Find the entry */
    list_for_each_entry_safe(entry, tmp, &pool->used_list, list) {
        if (entry->ptr == ptr) {
            if (atomic_dec_and_test(&entry->ref_count)) {
                list_del(&entry->list);
                list_add(&entry->list, &pool->free_list);
                atomic64_inc(&pool->deallocations);
            }
            found = true;
            break;
        }
    }
    
    spin_unlock_irqrestore(&pool->lock, flags);
    
    if (!found) {
        /* Not from pool, use regular free */
        vexfs_mm_free(ptr);
    }
}

/*
 * Cleanup memory pool
 */
void vexfs_mm_pool_cleanup(enum vexfs_mm_pool_type type)
{
    struct vexfs_mm_pool *pool;
    struct vexfs_mm_pool_entry *entry, *tmp;
    unsigned long flags;
    
    if (!vexfs_mm || type >= VEXFS_MM_POOL_COUNT)
        return;
    
    pool = &vexfs_mm->pools[type];
    
    spin_lock_irqsave(&pool->lock, flags);
    
    /* Free all entries */
    list_for_each_entry_safe(entry, tmp, &pool->free_list, list) {
        list_del(&entry->list);
        vexfs_mm_free(entry->ptr);
        kfree(entry);
        pool->current_entries--;
    }
    
    list_for_each_entry_safe(entry, tmp, &pool->used_list, list) {
        list_del(&entry->list);
        vexfs_mm_free(entry->ptr);
        kfree(entry);
        pool->current_entries--;
    }
    
    spin_unlock_irqrestore(&pool->lock, flags);
}

/*
 * Get best NUMA node for allocation
 */
int vexfs_mm_get_best_numa_node(void)
{
    if (!vexfs_mm || !vexfs_mm->numa_aware)
        return NUMA_NO_NODE;
    
    /* For now, return current node. Could be enhanced with load balancing */
    return vexfs_mm->current_numa_node;
}

/*
 * Get current NUMA node
 */
int vexfs_mm_get_current_numa_node(void)
{
    return numa_node_id();
}

/*
 * Update NUMA statistics
 */
void vexfs_mm_update_numa_stats(int node, size_t size, bool success)
{
    if (!vexfs_mm || node < 0 || node >= MAX_NUMNODES)
        return;
    
    if (success) {
        atomic64_inc(&vexfs_mm->numa_nodes[node].allocation_count);
        vexfs_mm->numa_nodes[node].allocated_memory += size;
    } else {
        atomic64_inc(&vexfs_mm->numa_nodes[node].allocation_failures);
    }
}

/*
 * Get memory statistics
 */
void vexfs_mm_get_stats(struct vexfs_mm_stats *stats)
{
    if (!vexfs_mm || !stats)
        return;
    
    memcpy(stats, &vexfs_mm->stats, sizeof(*stats));
}

/*
 * Reset statistics
 */
void vexfs_mm_reset_stats(void)
{
    if (!vexfs_mm)
        return;
    
    memset(&vexfs_mm->stats, 0, sizeof(vexfs_mm->stats));
}

/*
 * Get total memory usage
 */
size_t vexfs_mm_get_total_usage(void)
{
    if (!vexfs_mm)
        return 0;
    
    return atomic64_read(&vexfs_mm->stats.current_usage);
}

/*
 * Get peak memory usage
 */
size_t vexfs_mm_get_peak_usage(void)
{
    if (!vexfs_mm)
        return 0;
    
read(&vexfs_mm->stats.peak_usage);
}

/*
 * Print memory statistics
 */
void vexfs_mm_print_stats(void)
{
    if (!vexfs_mm)
        return;
    
    pr_info("VexFS Memory Manager Statistics:\n");
    pr_info("  Total Allocated: %llu bytes\n", 
            atomic64_read(&vexfs_mm->stats.total_allocated));
    pr_info("  Total Freed: %llu bytes\n",
            atomic64_read(&vexfs_mm->stats.total_freed));
    pr_info("  Current Usage: %llu bytes\n",
            atomic64_read(&vexfs_mm->stats.current_usage));
    pr_info("  Peak Usage: %llu bytes\n",
            atomic64_read(&vexfs_mm->stats.peak_usage));
    pr_info("  NUMA Local Allocs: %llu\n",
            atomic64_read(&vexfs_mm->stats.numa_local_allocs));
    pr_info("  NUMA Remote Allocs: %llu\n",
            atomic64_read(&vexfs_mm->stats.numa_remote_allocs));
    pr_info("  SIMD Aligned Allocs: %llu\n",
            atomic64_read(&vexfs_mm->stats.simd_aligned_allocs));
    pr_info("  Contiguous Allocs: %llu\n",
            atomic64_read(&vexfs_mm->stats.contiguous_allocs));
    pr_info("  Pool Hits: %llu\n",
            atomic64_read(&vexfs_mm->stats.pool_hits));
    pr_info("  Pool Misses: %llu\n",
            atomic64_read(&vexfs_mm->stats.pool_misses));
    pr_info("  Large Page Allocs: %llu\n",
            atomic64_read(&vexfs_mm->stats.large_page_allocs));
    pr_info("  User Mappings: %llu\n",
            atomic64_read(&vexfs_mm->stats.user_mappings));
    pr_info("  Allocation Failures: %llu\n",
            atomic64_read(&vexfs_mm->stats.allocation_failures));
}

/*
 * Utility functions
 */
bool vexfs_mm_is_aligned(void *ptr, unsigned int alignment)
{
    if (!ptr || alignment == 0)
        return false;
    
    return ((unsigned long)ptr & (alignment - 1)) == 0;
}

unsigned int vexfs_mm_get_alignment(void *ptr)
{
    unsigned long addr = (unsigned long)ptr;
    unsigned int alignment = 1;
    
    if (!ptr)
        return 0;
    
    /* Find the largest power of 2 that divides the address */
    while ((addr & alignment) == 0 && alignment <= 4096) {
        alignment <<= 1;
    }
    
    return alignment >> 1;
}

size_t vexfs_mm_get_allocation_size(void *ptr)
{
    /* TODO: Implement allocation size tracking */
    if (!ptr)
        return 0;
    
    return ksize(ptr);
}

int vexfs_mm_get_numa_node(void *ptr)
{
    struct page *page;
    
    if (!ptr)
        return NUMA_NO_NODE;
    
    if (is_vmalloc_addr(ptr)) {
        page = vmalloc_to_page(ptr);
    } else {
        page = virt_to_page(ptr);
    }
    
    if (!page)
        return NUMA_NO_NODE;
    
    return page_to_nid(page);
}

/*
 * User-space mapping functions
 */
int vexfs_mm_map_to_user(void *kernel_ptr, size_t size, struct vm_area_struct *vma)
{
    struct vexfs_mm_user_mapping *mapping;
    struct page **pages;
    size_t page_count;
    unsigned long pfn;
    int ret = 0;
    int i;
    
    if (!vexfs_mm || !kernel_ptr || !vma || size == 0)
        return -EINVAL;
    
    page_count = (size + PAGE_SIZE - 1) >> PAGE_SHIFT;
    
    /* Allocate page array */
    pages = kmalloc_array(page_count, sizeof(struct page *), GFP_KERNEL);
    if (!pages)
        return -ENOMEM;
    
    /* Get pages for kernel memory */
    if (is_vmalloc_addr(kernel_ptr)) {
        for (i = 0; i < page_count; i++) {
            pages[i] = vmalloc_to_page(kernel_ptr + i * PAGE_SIZE);
            if (!pages[i]) {
                ret = -EFAULT;
                goto err_free_pages;
            }
        }
    } else {
        pfn = virt_to_pfn(kernel_ptr);
        for (i = 0; i < page_count; i++) {
            pages[i] = pfn_to_page(pfn + i);
            if (!pages[i]) {
                ret = -EFAULT;
                goto err_free_pages;
            }
        }
    }
    
    /* Create mapping structure */
    mapping = kmalloc(sizeof(*mapping), GFP_KERNEL);
    if (!mapping) {
        ret = -ENOMEM;
        goto err_free_pages;
    }
    
    mapping->vma = vma;
    mapping->kernel_ptr = kernel_ptr;
    mapping->size = size;
    mapping->pages = pages;
    mapping->page_count = page_count;
    atomic_set(&mapping->ref_count, 1);
    mapping->created_time = ktime_get_ns();
    
    /* Map pages to user space */
    ret = remap_pfn_range(vma, vma->vm_start, page_to_pfn(pages[0]),
                          size, vma->vm_page_prot);
    if (ret)
        goto err_free_mapping;
    
    /* Add to tracking list */
    spin_lock(&vexfs_mm->user_mappings_lock);
    list_add(&mapping->list, &vexfs_mm->user_mappings);
    spin_unlock(&vexfs_mm->user_mappings_lock);
    
    atomic64_inc(&vexfs_mm->stats.user_mappings);
    return 0;
    
err_free_mapping:
    kfree(mapping);
err_free_pages:
    kfree(pages);
    return ret;
}

void vexfs_mm_unmap_from_user(struct vm_area_struct *vma)
{
    struct vexfs_mm_user_mapping *mapping, *tmp;
    unsigned long flags;
    
    if (!vexfs_mm || !vma)
        return;
    
    spin_lock_irqsave(&vexfs_mm->user_mappings_lock, flags);
    
    list_for_each_entry_safe(mapping, tmp, &vexfs_mm->user_mappings, list) {
        if (mapping->vma == vma) {
            list_del(&mapping->list);
            
            if (atomic_dec_and_test(&mapping->ref_count)) {
                kfree(mapping->pages);
                kfree(mapping);
            }
            break;
        }
    }
    
    spin_unlock_irqrestore(&vexfs_mm->user_mappings_lock, flags);
}

/*
 * Background maintenance workers
 */
void vexfs_mm_cleanup_worker(struct work_struct *work)
{
    struct delayed_work *dwork = to_delayed_work(work);
    struct vexfs_memory_manager *mm = container_of(dwork, 
                                                   struct vexfs_memory_manager,
                                                   cleanup_work);
    int i;
    
    if (!atomic_read(&mm->initialized))
        return;
    
    /* Cleanup unused pool entries */
    for (i = 0; i < VEXFS_MM_POOL_COUNT; i++) {
        struct vexfs_mm_pool *pool = &mm->pools[i];
        struct vexfs_mm_pool_entry *entry, *tmp;
        unsigned long flags;
        u64 current_time = ktime_get_ns();
        u64 timeout = 60ULL * NSEC_PER_SEC; /* 60 seconds */
        
        spin_lock_irqsave(&pool->lock, flags);
        
        list_for_each_entry_safe(entry, tmp, &pool->free_list, list) {
            if (current_time - entry->last_used > timeout) {
                list_del(&entry->list);
                vexfs_mm_free(entry->ptr);
                kfree(entry);
                pool->current_entries--;
            }
        }
        
        spin_unlock_irqrestore(&pool->lock, flags);
    }
    
    /* Schedule next cleanup */
    queue_delayed_work(mm->workqueue, &mm->cleanup_work,
                       msecs_to_jiffies(30000)); /* 30 seconds */
}

void vexfs_mm_defrag_worker(struct work_struct *work)
{
    struct delayed_work *dwork = to_delayed_work(work);
    struct vexfs_memory_manager *mm = container_of(dwork,
                                                   struct vexfs_memory_manager,
                                                   defrag_work);
    
    if (!atomic_read(&mm->initialized))
        return;
    
    /* TODO: Implement memory defragmentation logic */
    
    /* Schedule next defragmentation */
    queue_delayed_work(mm->workqueue, &mm->defrag_work,
                       msecs_to_jiffies(60000)); /* 60 seconds */
}

void vexfs_mm_schedule_cleanup(void)
{
    if (vexfs_mm && atomic_read(&vexfs_mm->initialized)) {
        queue_delayed_work(vexfs_mm->workqueue, &vexfs_mm->cleanup_work, 0);
    }
}

void vexfs_mm_schedule_defragmentation(void)
{
    if (vexfs_mm && atomic_read(&vexfs_mm->initialized)) {
        queue_delayed_work(vexfs_mm->workqueue, &vexfs_mm->defrag_work, 0);
    }
}

/*
 * Error handling
 */
const char *vexfs_mm_get_error_string(int error_code)
{
    switch (error_code) {
    case -ENOMEM:
        return "Out of memory";
    case -EINVAL:
        return "Invalid argument";
    case -EFAULT:
        return "Memory fault";
    case -ENOSPC:
        return "No space left";
    case -EAGAIN:
        return "Resource temporarily unavailable";
    default:
        return "Unknown error";
    }
}

void vexfs_mm_handle_allocation_failure(size_t size, u32 flags)
{
    if (!vexfs_mm)
        return;
    
    atomic64_inc(&vexfs_mm->stats.allocation_failures);
    
    pr_warn("VexFS: Memory allocation failed: size=%zu, flags=0x%x\n",
            size, flags);
    
    /* Try to trigger cleanup */
    vexfs_mm_schedule_cleanup();
    
    /* If this is a critical allocation, try defragmentation */
    if (flags & VEXFS_MM_FLAG_HIGH_PRIORITY) {
        vexfs_mm_schedule_defragmentation();
    }
}

/*
 * Module initialization and cleanup
 */
static int __init vexfs_memory_manager_init(void)
{
    return vexfs_mm_init();
}

static void __exit vexfs_memory_manager_exit(void)
{
    vexfs_mm_exit();
}

module_init(vexfs_memory_manager_init);
module_exit(vexfs_memory_manager_exit);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 Optimized Memory Management System");
MODULE_VERSION("2.0.0");
    return atomic64_