/*
 * VexFS v2.0 Optimized Memory Management System
 * 
 * This header defines an advanced memory management system specifically
 * optimized for vector data workloads in kernel space. It provides:
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

#ifndef _VEXFS_V2_MEMORY_MANAGER_H
#define _VEXFS_V2_MEMORY_MANAGER_H

#ifdef __KERNEL__
#include <linux/types.h>
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
#include <asm/page.h>
#include <asm/cacheflush.h>
#endif

#include "vexfs_v2_uapi.h"
#include "vexfs_v2_monitoring.h"

/*
 * Memory Management Configuration
 */
#define VEXFS_MM_MAX_POOLS              16
#define VEXFS_MM_MAX_POOL_SIZE          (64 * 1024 * 1024)  /* 64MB per pool */
#define VEXFS_MM_MIN_POOL_SIZE          (1 * 1024 * 1024)   /* 1MB minimum */
#define VEXFS_MM_POOL_GROWTH_FACTOR     2
#define VEXFS_MM_MAX_ORDER              10  /* Up to 4MB contiguous allocations */
#define VEXFS_MM_ALIGNMENT_MASK         0x3F  /* 64-byte alignment mask */

/* SIMD alignment requirements */
#define VEXFS_MM_ALIGN_SSE              16   /* 128-bit SSE */
#define VEXFS_MM_ALIGN_AVX              32   /* 256-bit AVX */
#define VEXFS_MM_ALIGN_AVX512           64   /* 512-bit AVX-512 */

/* Memory allocation flags */
#define VEXFS_MM_FLAG_NUMA_LOCAL        0x01
#define VEXFS_MM_FLAG_SIMD_ALIGN        0x02
#define VEXFS_MM_FLAG_CONTIGUOUS        0x04
#define VEXFS_MM_FLAG_USER_MAPPABLE     0x08
#define VEXFS_MM_FLAG_ZERO_FILL         0x10
#define VEXFS_MM_FLAG_HIGH_PRIORITY     0x20

/* Memory pool types */
enum vexfs_mm_pool_type {
    VEXFS_MM_POOL_VECTOR_SMALL,     /* < 4KB vectors */
    VEXFS_MM_POOL_VECTOR_MEDIUM,    /* 4KB - 64KB vectors */
    VEXFS_MM_POOL_VECTOR_LARGE,     /* 64KB - 1MB vectors */
    VEXFS_MM_POOL_VECTOR_HUGE,      /* > 1MB vectors */
    VEXFS_MM_POOL_METADATA,         /* Metadata structures */
    VEXFS_MM_POOL_SEARCH_RESULTS,   /* Search result buffers */
    VEXFS_MM_POOL_GRAPH_NODES,      /* HNSW graph nodes */
    VEXFS_MM_POOL_HASH_TABLES,      /* LSH hash tables */
    VEXFS_MM_POOL_COUNT
};

/* Memory allocation statistics */
struct vexfs_mm_stats {
    atomic64_t total_allocated;
    atomic64_t total_freed;
    atomic64_t peak_usage;
    atomic64_t current_usage;
    atomic64_t numa_local_allocs;
    atomic64_t numa_remote_allocs;
    atomic64_t simd_aligned_allocs;
    atomic64_t contiguous_allocs;
    atomic64_t pool_hits;
    atomic64_t pool_misses;
    atomic64_t large_page_allocs;
    atomic64_t user_mappings;
    atomic64_t allocation_failures;
    atomic64_t fragmentation_events;
};

/* Memory pool entry */
struct vexfs_mm_pool_entry {
    void *ptr;
    size_t size;
    int numa_node;
    unsigned int alignment;
    struct list_head list;
    atomic_t ref_count;
    u64 last_used;
};

/* Memory pool */
struct vexfs_mm_pool {
    enum vexfs_mm_pool_type type;
    size_t entry_size;
    size_t max_entries;
    size_t current_entries;
    int preferred_numa_node;
    unsigned int alignment;
    spinlock_t lock;
    struct list_head free_list;
    struct list_head used_list;
    atomic64_t hits;
    atomic64_t misses;
    atomic64_t allocations;
    atomic64_t deallocations;
};

/* Large allocation tracking */
struct vexfs_mm_large_alloc {
    void *ptr;
    size_t size;
    int numa_node;
    unsigned int order;
    struct page **pages;
    size_t page_count;
    struct rb_node rb_node;
    atomic_t ref_count;
    u64 allocated_time;
    u32 flags;
};

/* NUMA memory node information */
struct vexfs_mm_numa_info {
    int node_id;
    size_t total_memory;
    size_t available_memory;
    size_t allocated_memory;
    atomic64_t allocation_count;
    atomic64_t allocation_failures;
    struct list_head pool_list;
};

/* User-space mapping information */
struct vexfs_mm_user_mapping {
    struct vm_area_struct *vma;
    void *kernel_ptr;
    size_t size;
    struct page **pages;
    size_t page_count;
    atomic_t ref_count;
    struct list_head list;
    u64 created_time;
};

/* Main memory manager structure */
struct vexfs_memory_manager {
    /* Memory pools */
    struct vexfs_mm_pool pools[VEXFS_MM_POOL_COUNT];
    
    /* Large allocation tracking */
    struct rb_root large_allocs;
    spinlock_t large_allocs_lock;
    
    /* NUMA information */
    struct vexfs_mm_numa_info numa_nodes[MAX_NUMNODES];
    int numa_node_count;
    int current_numa_node;
    
    /* User-space mappings */
    struct list_head user_mappings;
    spinlock_t user_mappings_lock;
    
    /* Statistics */
    struct vexfs_mm_stats stats;
    
    /* Configuration */
    bool numa_aware;
    bool large_pages_enabled;
    unsigned int default_alignment;
    size_t max_allocation_size;
    
    /* Workqueue for background tasks */
    struct workqueue_struct *workqueue;
    struct delayed_work cleanup_work;
    struct delayed_work defrag_work;
    
    /* Synchronization */
    struct mutex manager_mutex;
    atomic_t initialized;
};

/*
 * Memory Manager API Functions
 */

/* Initialization and cleanup */
int vexfs_mm_init(void);
void vexfs_mm_exit(void);
int vexfs_mm_init_pools(void);
void vexfs_mm_cleanup_pools(void);

/* Core allocation functions */
void *vexfs_mm_alloc(size_t size, enum vexfs_mm_pool_type pool_type, u32 flags);
void *vexfs_mm_alloc_aligned(size_t size, unsigned int alignment, u32 flags);
void *vexfs_mm_alloc_contiguous(size_t size, unsigned int order, u32 flags);
void *vexfs_mm_alloc_numa(size_t size, int numa_node, u32 flags);
void vexfs_mm_free(void *ptr);

/* SIMD-aligned allocations */
void *vexfs_mm_alloc_simd_sse(size_t size, u32 flags);
void *vexfs_mm_alloc_simd_avx(size_t size, u32 flags);
void *vexfs_mm_alloc_simd_avx512(size_t size, u32 flags);

/* Large page allocations */
void *vexfs_mm_alloc_large_pages(size_t size, int numa_node, u32 flags);
void vexfs_mm_free_large_pages(void *ptr);

/* User-space mapping */
int vexfs_mm_map_to_user(void *kernel_ptr, size_t size, 
                         struct vm_area_struct *vma);
void vexfs_mm_unmap_from_user(struct vm_area_struct *vma);

/* Pool management */
int vexfs_mm_pool_init(enum vexfs_mm_pool_type type, size_t entry_size,
                       size_t max_entries, int numa_node);
void *vexfs_mm_pool_alloc(enum vexfs_mm_pool_type type);
void vexfs_mm_pool_free(enum vexfs_mm_pool_type type, void *ptr);
void vexfs_mm_pool_cleanup(enum vexfs_mm_pool_type type);

/* NUMA management */
int vexfs_mm_get_best_numa_node(void);
int vexfs_mm_get_current_numa_node(void);
void vexfs_mm_update_numa_stats(int node, size_t size, bool success);

/* Statistics and monitoring */
void vexfs_mm_get_stats(struct vexfs_mm_stats *stats);
void vexfs_mm_reset_stats(void);
size_t vexfs_mm_get_total_usage(void);
size_t vexfs_mm_get_peak_usage(void);
void vexfs_mm_print_stats(void);

/* Utility functions */
bool vexfs_mm_is_aligned(void *ptr, unsigned int alignment);
unsigned int vexfs_mm_get_alignment(void *ptr);
size_t vexfs_mm_get_allocation_size(void *ptr);
int vexfs_mm_get_numa_node(void *ptr);

/* Background maintenance */
void vexfs_mm_schedule_cleanup(void);
void vexfs_mm_schedule_defragmentation(void);
void vexfs_mm_cleanup_worker(struct work_struct *work);
void vexfs_mm_defrag_worker(struct work_struct *work);

/* Error handling */
const char *vexfs_mm_get_error_string(int error_code);
void vexfs_mm_handle_allocation_failure(size_t size, u32 flags);

/*
 * Inline helper functions for common operations
 */

/* Get optimal pool type for vector size */
static inline enum vexfs_mm_pool_type vexfs_mm_get_vector_pool_type(size_t size)
{
    if (size < 4096)
        return VEXFS_MM_POOL_VECTOR_SMALL;
    else if (size < 65536)
        return VEXFS_MM_POOL_VECTOR_MEDIUM;
    else if (size < 1048576)
        return VEXFS_MM_POOL_VECTOR_LARGE;
    else
        return VEXFS_MM_POOL_VECTOR_HUGE;
}

/* Calculate required alignment for SIMD operations */
static inline unsigned int vexfs_mm_get_simd_alignment(u32 simd_capabilities)
{
    if (simd_capabilities & VEXFS_SIMD_AVX512)
        return VEXFS_MM_ALIGN_AVX512;
    else if (simd_capabilities & VEXFS_SIMD_AVX)
        return VEXFS_MM_ALIGN_AVX;
    else
        return VEXFS_MM_ALIGN_SSE;
}

/* Check if allocation should use large pages */
static inline bool vexfs_mm_should_use_large_pages(size_t size)
{
    return size >= (2 * 1024 * 1024); /* 2MB threshold */
}

/* Calculate order for page allocation */
static inline unsigned int vexfs_mm_size_to_order(size_t size)
{
    return get_order(size);
}

/* Update allocation statistics */
static inline void vexfs_mm_update_stats(size_t size, bool numa_local, 
                                          bool simd_aligned, bool success)
{
    extern struct vexfs_memory_manager *vexfs_mm;
    
    if (success) {
        atomic64_add(size, &vexfs_mm->stats.total_allocated);
        atomic64_add(size, &vexfs_mm->stats.current_usage);
        
        if (numa_local)
            atomic64_inc(&vexfs_mm->stats.numa_local_allocs);
        else
            atomic64_inc(&vexfs_mm->stats.numa_remote_allocs);
            
        if (simd_aligned)
            atomic64_inc(&vexfs_mm->stats.simd_aligned_allocs);
            
        /* Update peak usage */
        u64 current = atomic64_read(&vexfs_mm->stats.current_usage);
        u64 peak = atomic64_read(&vexfs_mm->stats.peak_usage);
        if (current > peak)
            atomic64_set(&vexfs_mm->stats.peak_usage, current);
    } else {
        atomic64_inc(&vexfs_mm->stats.allocation_failures);
    }
}

/*
 * Memory allocation macros for common patterns
 */

/* Allocate vector data with optimal settings */
#define VEXFS_MM_ALLOC_VECTOR(size) \
    vexfs_mm_alloc(size, vexfs_mm_get_vector_pool_type(size), \
                   VEXFS_MM_FLAG_NUMA_LOCAL | VEXFS_MM_FLAG_SIMD_ALIGN)

/* Allocate SIMD-aligned vector data */
#define VEXFS_MM_ALLOC_VECTOR_SIMD(size, alignment) \
    vexfs_mm_alloc_aligned(size, alignment, \
                           VEXFS_MM_FLAG_NUMA_LOCAL | VEXFS_MM_FLAG_SIMD_ALIGN)

/* Allocate user-mappable memory */
#define VEXFS_MM_ALLOC_USER_MAPPABLE(size) \
    vexfs_mm_alloc(size, VEXFS_MM_POOL_VECTOR_MEDIUM, \
                   VEXFS_MM_FLAG_USER_MAPPABLE | VEXFS_MM_FLAG_ZERO_FILL)

/* Allocate contiguous memory for large vectors */
#define VEXFS_MM_ALLOC_CONTIGUOUS(size) \
    vexfs_mm_alloc_contiguous(size, vexfs_mm_size_to_order(size), \
                              VEXFS_MM_FLAG_CONTIGUOUS | VEXFS_MM_FLAG_NUMA_LOCAL)

/*
 * Global memory manager instance
 */
extern struct vexfs_memory_manager *vexfs_mm;

#endif /* _VEXFS_V2_MEMORY_MANAGER_H */