/*
 * VexFS v2.0 Vector Search Operations Header
 *
 * This header defines the search and query operations for VexFS v2.0,
 * implementing k-NN search, similarity matching, and semantic operations.
 *
 * Phase 2 Implementation: Vector Query Operations
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_V2_SEARCH_H
#define VEXFS_V2_SEARCH_H

#include <linux/types.h>
#include "vexfs_uapi.h"

/* Search operation types */
#define VEXFS_SEARCH_KNN        0x01    /* k-Nearest Neighbors */
#define VEXFS_SEARCH_RANGE      0x02    /* Range search (distance threshold) */
#define VEXFS_SEARCH_SIMILARITY 0x03    /* Similarity search (cosine, dot product) */

/* Distance metrics */
#define VEXFS_DISTANCE_EUCLIDEAN    0x01
#define VEXFS_DISTANCE_COSINE       0x02
#define VEXFS_DISTANCE_DOT_PRODUCT  0x03
#define VEXFS_DISTANCE_MANHATTAN    0x04

/* Search result structure */
struct vexfs_search_result {
    __u64 vector_id;        /* ID of the matching vector */
    __u64 distance;         /* Distance/similarity score */
    __u64 score;            /* Computed score (inverse of distance) */
    __u32 metadata_size;    /* Size of additional metadata */
    __u32 metadata_offset;  /* Offset to additional metadata */
    __u32 reserved;
};

/* k-NN search request */
struct vexfs_knn_query {
    uint32_t *query_vector;     /* Input query vector (as uint32_t to avoid FPU) */
    __u32 dimensions;           /* Vector dimensions */
    __u32 k;                    /* Number of nearest neighbors */
    __u32 distance_metric;      /* Distance calculation method */
    __u32 search_flags;         /* Search behavior flags */
    
    /* Output arrays (allocated by caller) */
    struct vexfs_search_result *results;  /* k results */
    __u32 results_found;        /* Actual number of results */
    
    /* Performance metrics */
    __u64 search_time_ns;       /* Search execution time */
    __u32 vectors_scanned;      /* Number of vectors examined */
    __u32 index_hits;           /* Index structure hits */
};

/* Range search request */
struct vexfs_range_query {
    uint32_t *query_vector;     /* Input query vector (as uint32_t to avoid FPU) */
    __u32 dimensions;           /* Vector dimensions */
    __u32 max_distance;         /* Maximum distance threshold (integer to avoid SSE) */
    __u32 distance_metric;      /* Distance calculation method */
    __u32 max_results;          /* Maximum results to return */
    __u32 search_flags;         /* Search behavior flags */
    
    /* Output arrays (allocated by caller) */
    struct vexfs_search_result *results;
    __u32 results_found;        /* Actual number of results */
    
    /* Performance metrics */
    __u64 search_time_ns;
    __u32 vectors_scanned;
    __u32 index_hits;
};

/* Batch search request for multiple queries */
struct vexfs_batch_search {
    __u32 query_count;          /* Number of queries */
    struct vexfs_knn_query *queries;  /* Array of k-NN queries */
    __u32 parallel_threads;     /* Number of parallel search threads */
    __u32 search_flags;         /* Global search flags */
    
    /* Aggregated performance metrics */
    __u64 total_search_time_ns;
    __u32 total_vectors_scanned;
    __u32 successful_queries;
    __u32 failed_queries;
};

/* Search index statistics */
struct vexfs_search_stats {
    __u64 total_vectors;        /* Total vectors in index */
    __u64 index_size_bytes;     /* Index memory usage */
    __u32 index_type;           /* Index structure type */
    __u32 index_levels;         /* Index depth/levels */
    
    /* Performance counters */
    __u64 total_searches;       /* Total search operations */
    __u64 cache_hits;           /* Index cache hits */
    __u64 cache_misses;         /* Index cache misses */
    __u32 avg_search_time_ms;   /* Average search time (integer to avoid SSE) */
    
    /* Quality metrics */
    __u32 index_efficiency;     /* Index efficiency ratio (integer to avoid SSE) */
    __u32 fragmentation_level;  /* Index fragmentation */
    __u64 last_rebuild_time;    /* Last index rebuild timestamp */
};

/* Search configuration */
struct vexfs_search_config {
    __u32 index_type;           /* HNSW, LSH, etc. */
    __u32 cache_size_mb;        /* Search cache size */
    __u32 parallel_threads;     /* Default parallel threads */
    __u32 prefetch_size;        /* Vector prefetch size */
    
    /* Index-specific parameters */
    union {
        struct {
            __u32 m;            /* HNSW: connections per node */
            __u32 ef_construction; /* HNSW: construction parameter */
            __u32 ef_search;    /* HNSW: search parameter */
            __u32 max_levels;   /* HNSW: maximum levels */
        } hnsw;
        
        struct {
            __u32 hash_tables;  /* LSH: number of hash tables */
            __u32 hash_bits;    /* LSH: bits per hash */
            __u32 bucket_width_bits; /* LSH: bucket width (IEEE 754 bits) */
        } lsh;
    } params;
};

/* Search flags */
#define VEXFS_SEARCH_EXACT          0x01    /* Exact search (no approximation) */
#define VEXFS_SEARCH_APPROXIMATE    0x02    /* Allow approximate results */
#define VEXFS_SEARCH_PARALLEL       0x04    /* Use parallel search */
#define VEXFS_SEARCH_CACHED         0x08    /* Use cached results */
#define VEXFS_SEARCH_PREFETCH       0x10    /* Prefetch related vectors */

/* New IOCTL commands for search operations */
#define VEXFS_IOC_KNN_SEARCH        _IOWR(VEXFS_IOC_MAGIC, 10, struct vexfs_knn_query)
#define VEXFS_IOC_RANGE_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 11, struct vexfs_range_query)
#define VEXFS_IOC_BATCH_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 12, struct vexfs_batch_search)
#define VEXFS_IOC_SEARCH_STATS      _IOR(VEXFS_IOC_MAGIC, 13, struct vexfs_search_stats)
#define VEXFS_IOC_SEARCH_CONFIG     _IOW(VEXFS_IOC_MAGIC, 14, struct vexfs_search_config)
#define VEXFS_IOC_REBUILD_INDEX     _IO(VEXFS_IOC_MAGIC, 15)

/* Function prototypes for kernel implementation */
#ifdef __KERNEL__

/* Core search functions */
int vexfs_knn_search(struct file *file, struct vexfs_knn_query *query);
int vexfs_range_search(struct file *file, struct vexfs_range_query *query);
int vexfs_batch_search(struct file *file, struct vexfs_batch_search *batch);

/* Index management */
int vexfs_build_search_index(struct vexfs_vector_file_info *meta);
int vexfs_rebuild_search_index(struct file *file);
int vexfs_update_search_index(struct file *file, __u64 vector_id, uint32_t *vector);

/* Distance calculations (integer arithmetic to avoid SSE issues) */
__u32 vexfs_euclidean_distance(const uint32_t *a, const uint32_t *b, __u32 dimensions);
__u32 vexfs_cosine_similarity(const uint32_t *a, const uint32_t *b, __u32 dimensions);
__s32 vexfs_dot_product(const uint32_t *a, const uint32_t *b, __u32 dimensions);
__u32 vexfs_manhattan_distance(const uint32_t *a, const uint32_t *b, __u32 dimensions);

/* Search statistics and monitoring */
int vexfs_get_search_stats(struct file *file, struct vexfs_search_stats *stats);
int vexfs_configure_search(struct file *file, struct vexfs_search_config *config);

/* Memory management for search operations */
void *vexfs_search_alloc(size_t size);
void vexfs_search_free(void *ptr);

#endif /* __KERNEL__ */

#endif /* VEXFS_V2_SEARCH_H */