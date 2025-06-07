/*
 * VexFS v2.0 Vector Data Caching System
 * 
 * Specialized caching system for vector data that maintains SIMD alignment
 * and optimizes for vector access patterns with NUMA awareness.
 * 
 * Features:
 * - SIMD-aligned vector storage (16/32/64-byte boundaries)
 * - NUMA-aware allocation using alloc_pages_node()
 * - Custom LRU eviction with vector operation awareness
 * - Integration with VFS page cache
 * - Prefetching for sequential vector access patterns
 * - Hot vector cache for frequently accessed vectors
 */

#ifndef VEXFS_V2_VECTOR_CACHE_H
#define VEXFS_V2_VECTOR_CACHE_H

#include <linux/types.h>
#include <linux/list.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/rbtree.h>
#include <linux/workqueue.h>
#include <linux/numa.h>
#include <linux/mm.h>
#include <linux/gfp.h>
#include <asm/fpu/api.h>

/* Vector cache configuration constants */
#define VEXFS_VECTOR_CACHE_SIZE_MB      64      /* Default cache size: 64MB */
#define VEXFS_VECTOR_CACHE_MAX_ENTRIES  8192    /* Maximum cached vectors */
#define VEXFS_VECTOR_CACHE_LINE_SIZE    64      /* CPU cache line size */
#define VEXFS_VECTOR_CACHE_PREFETCH     8       /* Prefetch window size */

/* SIMD alignment requirements */
#define VEXFS_SIMD_ALIGN_16    16   /* SSE alignment */
#define VEXFS_SIMD_ALIGN_32    32   /* AVX alignment */
#define VEXFS_SIMD_ALIGN_64    64   /* AVX-512 alignment */

/* Vector cache entry flags */
#define VEXFS_CACHE_ENTRY_VALID     0x01    /* Entry contains valid data */
#define VEXFS_CACHE_ENTRY_DIRTY     0x02    /* Entry has been modified */
#define VEXFS_CACHE_ENTRY_LOCKED    0x04    /* Entry is locked for I/O */
#define VEXFS_CACHE_ENTRY_PREFETCH  0x08    /* Entry was prefetched */
#define VEXFS_CACHE_ENTRY_HOT       0x10    /* Entry is in hot cache */
#define VEXFS_CACHE_ENTRY_SIMD      0x20    /* Entry is SIMD-aligned */

/* Vector access pattern types */
#define VEXFS_ACCESS_PATTERN_RANDOM     0x01
#define VEXFS_ACCESS_PATTERN_SEQUENTIAL 0x02
#define VEXFS_ACCESS_PATTERN_SEARCH     0x04
#define VEXFS_ACCESS_PATTERN_BATCH      0x08

/* Forward declarations */
struct vexfs_vector_cache;
struct vexfs_cache_entry;
struct vexfs_cache_stats;

/*
 * Vector cache entry structure
 * Represents a single cached vector with metadata
 */
struct vexfs_cache_entry {
    /* Cache management */
    struct list_head lru_list;          /* LRU list linkage */
    struct rb_node rb_node;             /* Red-black tree for fast lookup */
    struct hlist_node hash_node;        /* Hash table linkage */
    
    /* Vector identification */
    u64 vector_id;                      /* Unique vector identifier */
    u64 file_offset;                    /* Offset in file */
    u32 vector_size;                    /* Size of vector data in bytes */
    u16 dimensions;                     /* Number of vector dimensions */
    u8 element_type;                    /* Vector element type */
    u8 flags;                           /* Entry flags */
    
    /* Memory management */
    void *vector_data;                  /* SIMD-aligned vector data */
    struct page **pages;                /* Pages backing the vector data */
    u32 page_count;                     /* Number of pages allocated */
    u32 alignment;                      /* SIMD alignment requirement */
    int numa_node;                      /* NUMA node for allocation */
    
    /* Access tracking */
    atomic_t ref_count;                 /* Reference count */
    u64 last_access_time;               /* Last access timestamp */
    u32 access_count;                   /* Total access count */
    u32 access_pattern;                 /* Detected access pattern */
    
    /* Performance optimization */
    u32 search_frequency;               /* How often used in searches */
    u32 batch_frequency;                /* How often used in batch ops */
    u32 prefetch_score;                 /* Prefetch priority score */
    
    /* Synchronization */
    spinlock_t entry_lock;              /* Per-entry lock */
    struct completion io_completion;     /* I/O completion */
    
    /* Reserved for future extensions */
    u32 reserved[4];
};

/*
 * Vector cache statistics
 */
struct vexfs_cache_stats {
    /* Hit/miss statistics */
    atomic64_t cache_hits;              /* Total cache hits */
    atomic64_t cache_misses;            /* Total cache misses */
    atomic64_t cache_evictions;         /* Total evictions */
    atomic64_t cache_insertions;        /* Total insertions */
    
    /* Memory statistics */
    atomic64_t total_memory_used;       /* Total memory in use */
    atomic64_t peak_memory_used;        /* Peak memory usage */
    atomic64_t simd_aligned_allocs;     /* SIMD-aligned allocations */
    atomic64_t numa_local_allocs;       /* NUMA-local allocations */
    
    /* Access pattern statistics */
    atomic64_t sequential_accesses;     /* Sequential access count */
    atomic64_t random_accesses;         /* Random access count */
    atomic64_t search_accesses;         /* Search operation count */
    atomic64_t batch_accesses;          /* Batch operation count */
    
    /* Performance statistics */
    atomic64_t prefetch_hits;           /* Successful prefetches */
    atomic64_t prefetch_misses;         /* Failed prefetches */
    atomic64_t hot_cache_hits;          /* Hot cache hits */
    atomic64_t simd_operations;         /* SIMD operations performed */
    
    /* Timing statistics */
    u64 avg_lookup_time_ns;             /* Average lookup time */
    u64 avg_insertion_time_ns;          /* Average insertion time */
    u64 avg_eviction_time_ns;           /* Average eviction time */
};

/*
 * Hot vector cache for frequently accessed vectors
 */
struct vexfs_hot_cache {
    struct vexfs_cache_entry **entries; /* Array of hot cache entries */
    u32 capacity;                       /* Maximum hot cache entries */
    u32 count;                          /* Current hot cache entries */
    u32 promotion_threshold;            /* Access count for promotion */
    spinlock_t lock;                    /* Hot cache lock */
    
    /* Hot cache statistics */
    atomic64_t promotions;              /* Vectors promoted to hot cache */
    atomic64_t demotions;               /* Vectors demoted from hot cache */
    atomic64_t hot_hits;                /* Hits in hot cache */
};

/*
 * Vector prefetcher for sequential access patterns
 */
struct vexfs_vector_prefetcher {
    struct work_struct prefetch_work;   /* Async prefetch work */
    struct list_head prefetch_queue;    /* Prefetch request queue */
    spinlock_t queue_lock;              /* Prefetch queue lock */
    
    /* Prefetch configuration */
    u32 prefetch_window;                /* Number of vectors to prefetch */
    u32 prefetch_threshold;             /* Trigger threshold */
    u32 max_prefetch_size;              /* Maximum prefetch size */
    
    /* Prefetch statistics */
    atomic64_t prefetch_requests;       /* Total prefetch requests */
    atomic64_t prefetch_completions;    /* Completed prefetches */
    atomic64_t prefetch_cancellations;  /* Cancelled prefetches */
};

/*
 * Main vector cache structure
 */
struct vexfs_vector_cache {
    /* Cache configuration */
    u32 max_entries;                    /* Maximum cache entries */
    u32 max_memory_mb;                  /* Maximum memory usage (MB) */
    u32 default_alignment;              /* Default SIMD alignment */
    u32 numa_node_count;                /* Number of NUMA nodes */
    
    /* Cache storage */
    struct rb_root entry_tree;          /* Red-black tree for entries */
    struct hlist_head *hash_table;      /* Hash table for fast lookup */
    u32 hash_table_size;                /* Hash table size */
    struct list_head lru_list;          /* LRU list for eviction */
    
    /* Current state */
    atomic_t entry_count;               /* Current number of entries */
    atomic64_t memory_used;             /* Current memory usage */
    u32 current_numa_node;              /* Current NUMA node */
    
    /* Hot cache */
    struct vexfs_hot_cache hot_cache;   /* Hot vector cache */
    
    /* Prefetcher */
    struct vexfs_vector_prefetcher prefetcher; /* Vector prefetcher */
    
    /* Synchronization */
    rwlock_t cache_lock;                /* Main cache lock */
    spinlock_t lru_lock;                /* LRU list lock */
    spinlock_t hash_lock;               /* Hash table lock */
    
    /* Statistics */
    struct vexfs_cache_stats stats;     /* Cache statistics */
    
    /* NUMA awareness */
    struct {
        atomic64_t allocations;         /* Allocations per NUMA node */
        atomic64_t memory_used;         /* Memory used per NUMA node */
        u32 preferred_node;             /* Preferred NUMA node */
    } numa_stats[MAX_NUMNODES];
    
    /* Integration with VFS page cache */
    struct address_space *mapping;      /* Address space for integration */
    struct backing_dev_info *bdi;       /* Backing device info */
    
    /* Reserved for future extensions */
    void *reserved[8];
};

/* Function declarations */

/* Cache initialization and cleanup */
struct vexfs_vector_cache *vexfs_vector_cache_create(u32 max_entries, u32 max_memory_mb);
void vexfs_vector_cache_destroy(struct vexfs_vector_cache *cache);
int vexfs_vector_cache_init(struct vexfs_vector_cache *cache);
void vexfs_vector_cache_cleanup(struct vexfs_vector_cache *cache);

/* Cache operations */
struct vexfs_cache_entry *vexfs_cache_lookup(struct vexfs_vector_cache *cache, 
                                            u64 vector_id);
struct vexfs_cache_entry *vexfs_cache_insert(struct vexfs_vector_cache *cache,
                                            u64 vector_id, void *vector_data,
                                            u32 vector_size, u16 dimensions,
                                            u8 element_type);
int vexfs_cache_remove(struct vexfs_vector_cache *cache, u64 vector_id);
void vexfs_cache_evict_lru(struct vexfs_vector_cache *cache, u32 count);

/* Entry management */
struct vexfs_cache_entry *vexfs_cache_entry_alloc(u64 vector_id, u32 vector_size,
                                                 u16 dimensions, u8 element_type,
                                                 u32 alignment, int numa_node);
void vexfs_cache_entry_free(struct vexfs_cache_entry *entry);
void vexfs_cache_entry_get(struct vexfs_cache_entry *entry);
void vexfs_cache_entry_put(struct vexfs_cache_entry *entry);

/* SIMD-aligned memory allocation */
void *vexfs_alloc_simd_aligned(size_t size, u32 alignment, int numa_node);
void vexfs_free_simd_aligned(void *ptr, size_t size);
int vexfs_is_simd_aligned(void *ptr, u32 alignment);

/* NUMA-aware allocation */
struct page **vexfs_alloc_vector_pages(u32 page_count, int numa_node);
void vexfs_free_vector_pages(struct page **pages, u32 page_count);
int vexfs_get_optimal_numa_node(void);

/* Hot cache management */
int vexfs_hot_cache_init(struct vexfs_hot_cache *hot_cache, u32 capacity);
void vexfs_hot_cache_cleanup(struct vexfs_hot_cache *hot_cache);
int vexfs_hot_cache_promote(struct vexfs_vector_cache *cache, 
                           struct vexfs_cache_entry *entry);
int vexfs_hot_cache_demote(struct vexfs_vector_cache *cache,
                          struct vexfs_cache_entry *entry);

/* Prefetching */
int vexfs_prefetcher_init(struct vexfs_vector_prefetcher *prefetcher);
void vexfs_prefetcher_cleanup(struct vexfs_vector_prefetcher *prefetcher);
int vexfs_prefetch_vectors(struct vexfs_vector_cache *cache, u64 start_vector_id,
                          u32 count, u32 access_pattern);
void vexfs_prefetch_worker(struct work_struct *work);

/* Access pattern detection */
u32 vexfs_detect_access_pattern(struct vexfs_cache_entry *entry, u64 vector_id);
void vexfs_update_access_pattern(struct vexfs_cache_entry *entry, u32 pattern);

/* Cache statistics and monitoring */
void vexfs_cache_update_stats(struct vexfs_vector_cache *cache, 
                             struct vexfs_cache_entry *entry, bool hit);
void vexfs_cache_print_stats(struct vexfs_vector_cache *cache);
void vexfs_cache_reset_stats(struct vexfs_vector_cache *cache);

/* Integration with VFS page cache */
int vexfs_cache_integrate_vfs(struct vexfs_vector_cache *cache,
                             struct address_space *mapping);
void vexfs_cache_sync_vfs(struct vexfs_vector_cache *cache);

/* Vector operations with FPU context */
int vexfs_cache_vector_operation(struct vexfs_cache_entry *entry,
                                void (*operation)(void *data, size_t size),
                                bool use_simd);

/* Cache tuning and optimization */
void vexfs_cache_tune_parameters(struct vexfs_vector_cache *cache);
int vexfs_cache_optimize_numa_placement(struct vexfs_vector_cache *cache);
void vexfs_cache_balance_load(struct vexfs_vector_cache *cache);

/* Debug and testing */
#ifdef CONFIG_VEXFS_DEBUG
void vexfs_cache_validate_integrity(struct vexfs_vector_cache *cache);
void vexfs_cache_dump_state(struct vexfs_vector_cache *cache);
int vexfs_cache_stress_test(struct vexfs_vector_cache *cache);
#endif

/* Inline helper functions */

static inline bool vexfs_cache_entry_is_hot(struct vexfs_cache_entry *entry)
{
    return (entry->flags & VEXFS_CACHE_ENTRY_HOT) != 0;
}

static inline bool vexfs_cache_entry_is_simd_aligned(struct vexfs_cache_entry *entry)
{
    return (entry->flags & VEXFS_CACHE_ENTRY_SIMD) != 0;
}

static inline u64 vexfs_cache_hit_rate(struct vexfs_vector_cache *cache)
{
    u64 hits = atomic64_read(&cache->stats.cache_hits);
    u64 misses = atomic64_read(&cache->stats.cache_misses);
    return (hits + misses) ? (hits * 100) / (hits + misses) : 0;
}

static inline u64 vexfs_cache_memory_usage_mb(struct vexfs_vector_cache *cache)
{
    return atomic64_read(&cache->memory_used) / (1024 * 1024);
}

static inline bool vexfs_cache_is_full(struct vexfs_vector_cache *cache)
{
    return atomic_read(&cache->entry_count) >= cache->max_entries ||
           vexfs_cache_memory_usage_mb(cache) >= cache->max_memory_mb;
}

#endif /* VEXFS_V2_VECTOR_CACHE_H */