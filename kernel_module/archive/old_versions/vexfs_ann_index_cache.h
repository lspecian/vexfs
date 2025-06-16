/*
 * VexFS v2.0 ANN Index Caching System
 * 
 * Specialized caching system for Approximate Nearest Neighbor (ANN) index structures
 * to optimize vector search operations. This system provides dedicated caching for:
 * - HNSW graph structures using RCU-protected linked lists
 * - Product quantization codebooks in SIMD-aligned memory
 * - Inverted file index (IVF) centroids with efficient lookup structures
 * - Custom cache coherency mechanisms for index updates
 * - Priority-based caching based on query frequency
 * 
 * Features:
 * - Specialized kmem_cache instances for different index structure types
 * - Reference counting for cache lifetime management
 * - RCU-protected concurrent access to index structures
 * - NUMA-aware allocation for optimal performance
 * - Integration with existing vector cache and memory management systems
 */

#ifndef VEXFS_V2_ANN_INDEX_CACHE_H
#define VEXFS_V2_ANN_INDEX_CACHE_H

#include <linux/types.h>
#include <linux/list.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/rbtree.h>
#include <linux/workqueue.h>
#include <linux/numa.h>
#include <linux/mm.h>
#include <linux/gfp.h>
#include <linux/slab.h>
#include <linux/rcupdate.h>
#include <linux/mutex.h>
#include <linux/completion.h>
#include <linux/hashtable.h>

/* Include existing VexFS v2.0 headers */
#include "vexfs_v2_vector_cache.h"
#include "vexfs_v2_memory_manager.h"
#include "vexfs_v2_phase3.h"

/* ANN Index Cache Configuration Constants */
#define VEXFS_ANN_CACHE_SIZE_MB         128     /* Default ANN cache size: 128MB */
#define VEXFS_ANN_CACHE_MAX_ENTRIES     4096    /* Maximum cached index structures */
#define VEXFS_ANN_CACHE_HASH_BITS       12      /* Hash table size (4096 buckets) */
#define VEXFS_ANN_CACHE_RCU_GRACE_MS    100     /* RCU grace period in milliseconds */

/* Index structure type identifiers */
enum vexfs_ann_index_type {
    VEXFS_ANN_INDEX_HNSW_NODE = 0,      /* HNSW graph nodes */
    VEXFS_ANN_INDEX_HNSW_LAYER,         /* HNSW layer connections */
    VEXFS_ANN_INDEX_PQ_CODEBOOK,        /* Product quantization codebooks */
    VEXFS_ANN_INDEX_IVF_CENTROID,       /* IVF centroids */
    VEXFS_ANN_INDEX_LSH_HASH_TABLE,     /* LSH hash tables */
    VEXFS_ANN_INDEX_LSH_BUCKET,         /* LSH hash buckets */
    VEXFS_ANN_INDEX_SEARCH_RESULT,      /* Search result caches */
    VEXFS_ANN_INDEX_GRAPH_METADATA,     /* Graph metadata structures */
    VEXFS_ANN_INDEX_TYPE_COUNT           /* Total number of index types */
};

/* Cache entry flags */
#define VEXFS_ANN_CACHE_VALID           0x01    /* Entry contains valid data */
#define VEXFS_ANN_CACHE_DIRTY           0x02    /* Entry has been modified */
#define VEXFS_ANN_CACHE_LOCKED          0x04    /* Entry is locked for updates */
#define VEXFS_ANN_CACHE_RCU_PROTECTED   0x08    /* Entry is RCU-protected */
#define VEXFS_ANN_CACHE_HOT             0x10    /* Entry is frequently accessed */
#define VEXFS_ANN_CACHE_PREFETCHED      0x20    /* Entry was prefetched */
#define VEXFS_ANN_CACHE_NUMA_LOCAL      0x40    /* Entry is NUMA-local */
#define VEXFS_ANN_CACHE_COHERENT        0x80    /* Entry is cache-coherent */

/* Query frequency tracking */
#define VEXFS_ANN_QUERY_FREQ_WINDOW     1000    /* Query frequency window size */
#define VEXFS_ANN_HOT_THRESHOLD         100     /* Hot cache promotion threshold */
#define VEXFS_ANN_COLD_THRESHOLD        10      /* Cold cache demotion threshold */

/* Forward declarations */
struct vexfs_ann_cache;
struct vexfs_ann_cache_entry;
struct vexfs_ann_cache_stats;
struct vexfs_ann_index_ops;

/*
 * ANN Index Cache Entry Structure
 * Represents a cached ANN index structure with RCU protection
 */
struct vexfs_ann_cache_entry {
    /* RCU and cache management */
    struct rcu_head rcu_head;           /* RCU callback structure */
    struct list_head lru_list;          /* LRU list linkage */
    struct rb_node rb_node;             /* Red-black tree for fast lookup */
    struct hlist_node hash_node;        /* Hash table linkage */
    
    /* Index identification */
    u64 index_id;                       /* Unique index structure identifier */
    enum vexfs_ann_index_type type;     /* Type of index structure */
    u32 structure_size;                 /* Size of index structure in bytes */
    u32 element_count;                  /* Number of elements in structure */
    u8 flags;                           /* Entry flags */
    u8 numa_node;                       /* NUMA node for allocation */
    u16 reserved_flags;                 /* Reserved for future use */
    
    /* Index structure data */
    void *index_data;                   /* Pointer to index structure */
    struct page **pages;                /* Pages backing the index data */
    u32 page_count;                     /* Number of pages allocated */
    u32 alignment;                      /* Memory alignment requirement */
    
    /* Reference counting and synchronization */
    atomic_t ref_count;                 /* Reference count */
    spinlock_t entry_lock;              /* Per-entry lock */
    struct mutex update_mutex;          /* Mutex for structure updates */
    struct completion update_completion; /* Update completion */
    
    /* Access tracking and performance */
    u64 last_access_time;               /* Last access timestamp */
    u64 creation_time;                  /* Creation timestamp */
    atomic_t access_count;              /* Total access count */
    atomic_t query_frequency;           /* Query frequency counter */
    u32 search_hit_count;               /* Number of search hits */
    u32 update_count;                   /* Number of updates */
    
    /* Cache coherency */
    u64 version;                        /* Version number for coherency */
    u64 last_update_time;               /* Last update timestamp */
    atomic_t coherency_state;           /* Cache coherency state */
    
    /* Performance optimization */
    u32 prefetch_score;                 /* Prefetch priority score */
    u32 locality_score;                 /* NUMA locality score */
    u32 hotness_score;                  /* Cache hotness score */
    
    /* Index-specific metadata */
    union {
        struct {
            u32 layer_count;            /* HNSW: Number of layers */
            u32 max_connections;        /* HNSW: Max connections per layer */
            u64 entry_point_id;         /* HNSW: Entry point node ID */
        } hnsw;
        
        struct {
            u32 codebook_size;          /* PQ: Codebook size */
            u32 subvector_count;        /* PQ: Number of subvectors */
            u32 cluster_count;          /* PQ: Number of clusters */
        } pq;
        
        struct {
            u32 centroid_count;         /* IVF: Number of centroids */
            u32 dimensions;             /* IVF: Vector dimensions */
            u32 cluster_size;           /* IVF: Average cluster size */
        } ivf;
        
        struct {
            u32 hash_function_count;    /* LSH: Number of hash functions */
            u32 bucket_count;           /* LSH: Number of buckets */
            u32 collision_count;        /* LSH: Collision statistics */
        } lsh;
    } metadata;
    
    /* Reserved for future extensions */
    u64 reserved[4];
};

/*
 * ANN Index Cache Statistics
 */
struct vexfs_ann_cache_stats {
    /* Cache utilization */
    atomic64_t total_entries;           /* Total cached entries */
    atomic64_t active_entries;          /* Currently active entries */
    atomic64_t memory_usage;            /* Total memory usage in bytes */
    atomic64_t peak_memory_usage;       /* Peak memory usage */
    
    /* Access statistics */
    atomic64_t cache_hits;              /* Cache hit count */
    atomic64_t cache_misses;            /* Cache miss count */
    atomic64_t cache_evictions;         /* Cache eviction count */
    atomic64_t cache_invalidations;     /* Cache invalidation count */
    
    /* Performance metrics */
    atomic64_t avg_access_time_ns;      /* Average access time */
    atomic64_t avg_update_time_ns;      /* Average update time */
    atomic64_t rcu_grace_periods;       /* RCU grace period count */
    atomic64_t coherency_violations;    /* Cache coherency violations */
    
    /* Index type statistics */
    atomic64_t type_counts[VEXFS_ANN_INDEX_TYPE_COUNT];
    atomic64_t type_hits[VEXFS_ANN_INDEX_TYPE_COUNT];
    atomic64_t type_misses[VEXFS_ANN_INDEX_TYPE_COUNT];
    
    /* NUMA statistics */
    atomic64_t numa_local_hits;         /* NUMA-local cache hits */
    atomic64_t numa_remote_hits;        /* NUMA-remote cache hits */
    atomic64_t numa_migrations;         /* NUMA page migrations */
    
    /* Query frequency statistics */
    atomic64_t hot_promotions;          /* Hot cache promotions */
    atomic64_t cold_demotions;          /* Cold cache demotions */
    atomic64_t prefetch_hits;           /* Prefetch hits */
    atomic64_t prefetch_misses;         /* Prefetch misses */
};

/*
 * ANN Index Operations Structure
 * Defines operations for different index types
 */
struct vexfs_ann_index_ops {
    /* Index structure operations */
    int (*create)(struct vexfs_ann_cache_entry *entry, void *params);
    int (*destroy)(struct vexfs_ann_cache_entry *entry);
    int (*update)(struct vexfs_ann_cache_entry *entry, void *update_data);
    int (*validate)(struct vexfs_ann_cache_entry *entry);
    
    /* Serialization operations */
    int (*serialize)(struct vexfs_ann_cache_entry *entry, void *buffer, size_t size);
    int (*deserialize)(struct vexfs_ann_cache_entry *entry, void *buffer, size_t size);
    
    /* Cache coherency operations */
    int (*invalidate)(struct vexfs_ann_cache_entry *entry);
    int (*refresh)(struct vexfs_ann_cache_entry *entry);
    
    /* Performance optimization */
    u32 (*calculate_hotness)(struct vexfs_ann_cache_entry *entry);
    int (*prefetch_related)(struct vexfs_ann_cache_entry *entry);
    
    /* Index-specific operations */
    void *private_ops;                  /* Index-specific operations */
};

/*
 * ANN Index Cache Structure
 * Main cache management structure
 */
struct vexfs_ann_cache {
    /* Cache configuration */
    size_t max_memory_usage;            /* Maximum memory usage */
    u32 max_entries;                    /* Maximum number of entries */
    u32 rcu_grace_period_ms;            /* RCU grace period */
    
    /* Cache storage */
    DECLARE_HASHTABLE(cache_hash, VEXFS_ANN_CACHE_HASH_BITS);
    struct rb_root cache_tree;          /* Red-black tree for ordered access */
    struct list_head lru_list;          /* LRU list for eviction */
    struct list_head hot_list;          /* Hot cache list */
    
    /* Specialized kmem_cache instances */
    struct kmem_cache *caches[VEXFS_ANN_INDEX_TYPE_COUNT];
    
    /* Synchronization */
    spinlock_t cache_lock;              /* Main cache lock */
    struct mutex update_mutex;          /* Update operations mutex */
    struct rw_semaphore coherency_sem;  /* Cache coherency semaphore */
    
    /* Memory management integration */
    struct vexfs_memory_manager *mm;    /* Memory manager instance */
    struct vexfs_vector_cache *vector_cache; /* Vector cache integration */
    
    /* Background maintenance */
    struct workqueue_struct *maintenance_wq; /* Maintenance workqueue */
    struct delayed_work cleanup_work;   /* Cleanup work */
    struct delayed_work coherency_work; /* Coherency check work */
    struct delayed_work prefetch_work;  /* Prefetch work */
    
    /* Index operations */
    struct vexfs_ann_index_ops *ops[VEXFS_ANN_INDEX_TYPE_COUNT];
    
    /* Statistics and monitoring */
    struct vexfs_ann_cache_stats stats;
    
    /* NUMA awareness */
    int preferred_numa_node;            /* Preferred NUMA node */
    struct cpumask allowed_cpus;        /* Allowed CPU mask */
    
    /* Configuration parameters */
    u32 hot_threshold;                  /* Hot cache threshold */
    u32 cold_threshold;                 /* Cold cache threshold */
    u32 prefetch_window;                /* Prefetch window size */
    u32 coherency_check_interval_ms;    /* Coherency check interval */
    
    /* Reserved for future extensions */
    u64 reserved[8];
};

/* Function declarations */

/* Cache initialization and cleanup */
int vexfs_ann_cache_init(struct vexfs_ann_cache **cache,
                        struct vexfs_memory_manager *mm,
                        struct vexfs_vector_cache *vector_cache);
void vexfs_ann_cache_destroy(struct vexfs_ann_cache *cache);

/* Cache entry management */
struct vexfs_ann_cache_entry *vexfs_ann_cache_get(struct vexfs_ann_cache *cache,
                                                  u64 index_id,
                                                  enum vexfs_ann_index_type type);
int vexfs_ann_cache_put(struct vexfs_ann_cache *cache,
                       struct vexfs_ann_cache_entry *entry);
int vexfs_ann_cache_insert(struct vexfs_ann_cache *cache,
                          struct vexfs_ann_cache_entry *entry);
int vexfs_ann_cache_remove(struct vexfs_ann_cache *cache, u64 index_id);

/* Cache operations */
int vexfs_ann_cache_lookup(struct vexfs_ann_cache *cache,
                          u64 index_id,
                          enum vexfs_ann_index_type type,
                          struct vexfs_ann_cache_entry **entry);
int vexfs_ann_cache_update(struct vexfs_ann_cache *cache,
                          u64 index_id,
                          void *update_data);
int vexfs_ann_cache_invalidate(struct vexfs_ann_cache *cache, u64 index_id);
int vexfs_ann_cache_flush(struct vexfs_ann_cache *cache);

/* Reference counting */
void vexfs_ann_cache_entry_get(struct vexfs_ann_cache_entry *entry);
void vexfs_ann_cache_entry_put(struct vexfs_ann_cache_entry *entry);

/* RCU operations */
int vexfs_ann_cache_rcu_update(struct vexfs_ann_cache *cache,
                              u64 index_id,
                              void *new_data,
                              size_t data_size);
void vexfs_ann_cache_rcu_free(struct rcu_head *rcu);

/* Cache coherency */
int vexfs_ann_cache_check_coherency(struct vexfs_ann_cache *cache);
int vexfs_ann_cache_sync(struct vexfs_ann_cache *cache);
int vexfs_ann_cache_invalidate_range(struct vexfs_ann_cache *cache,
                                    u64 start_id, u64 end_id);

/* Performance optimization */
int vexfs_ann_cache_prefetch(struct vexfs_ann_cache *cache,
                            u64 *index_ids, u32 count);
int vexfs_ann_cache_promote_hot(struct vexfs_ann_cache *cache,
                               struct vexfs_ann_cache_entry *entry);
int vexfs_ann_cache_demote_cold(struct vexfs_ann_cache *cache,
                               struct vexfs_ann_cache_entry *entry);

/* Statistics and monitoring */
int vexfs_ann_cache_get_stats(struct vexfs_ann_cache *cache,
                             struct vexfs_ann_cache_stats *stats);
int vexfs_ann_cache_reset_stats(struct vexfs_ann_cache *cache);
void vexfs_ann_cache_print_stats(struct vexfs_ann_cache *cache);

/* Index type operations registration */
int vexfs_ann_cache_register_ops(struct vexfs_ann_cache *cache,
                                enum vexfs_ann_index_type type,
                                struct vexfs_ann_index_ops *ops);
void vexfs_ann_cache_unregister_ops(struct vexfs_ann_cache *cache,
                                   enum vexfs_ann_index_type type);

/* NUMA operations */
int vexfs_ann_cache_set_numa_policy(struct vexfs_ann_cache *cache,
                                   int preferred_node);
int vexfs_ann_cache_migrate_entry(struct vexfs_ann_cache *cache,
                                 struct vexfs_ann_cache_entry *entry,
                                 int target_node);

/* Background maintenance */
void vexfs_ann_cache_cleanup_work(struct work_struct *work);
void vexfs_ann_cache_coherency_work(struct work_struct *work);
void vexfs_ann_cache_prefetch_work(struct work_struct *work);

/* Utility functions */
static inline bool vexfs_ann_cache_entry_is_hot(struct vexfs_ann_cache_entry *entry)
{
    return (entry->flags & VEXFS_ANN_CACHE_HOT) != 0;
}

static inline bool vexfs_ann_cache_entry_is_valid(struct vexfs_ann_cache_entry *entry)
{
    return (entry->flags & VEXFS_ANN_CACHE_VALID) != 0;
}

static inline bool vexfs_ann_cache_entry_is_dirty(struct vexfs_ann_cache_entry *entry)
{
    return (entry->flags & VEXFS_ANN_CACHE_DIRTY) != 0;
}

static inline u64 vexfs_ann_cache_get_hit_ratio(struct vexfs_ann_cache *cache)
{
    u64 hits = atomic64_read(&cache->stats.cache_hits);
    u64 misses = atomic64_read(&cache->stats.cache_misses);
    return (hits + misses) ? (hits * 100) / (hits + misses) : 0;
}

#endif /* VEXFS_V2_ANN_INDEX_CACHE_H */