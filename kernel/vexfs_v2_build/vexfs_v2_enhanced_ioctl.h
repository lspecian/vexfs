/*
 * VexFS v2.0 Enhanced Vector-Specific ioctl Interface
 * 
 * Comprehensive ioctl interface for vector database operations in kernel space.
 * This extends the existing basic ioctl interface with advanced vector operations,
 * index management, and performance monitoring.
 * 
 * Features:
 * - Vector creation and management
 * - In-kernel similarity search with multiple algorithms
 * - ANN index construction and management
 * - Batch operations for high throughput
 * - Performance statistics and monitoring
 * - Security validation and error handling
 */

#ifndef VEXFS_V2_ENHANCED_IOCTL_H
#define VEXFS_V2_ENHANCED_IOCTL_H

#include <linux/types.h>
#include <linux/ioctl.h>
#include <linux/uaccess.h>
#include <linux/security.h>

#include "vexfs_v2_uapi.h"

/* Enhanced ioctl magic number (different from basic) */
#define VEXFS_ENHANCED_IOC_MAGIC    'X'

/* Enhanced vector operation flags */
#define VEXFS_CREATE_VECTOR_VALIDATE    (1 << 0)
#define VEXFS_CREATE_VECTOR_OVERWRITE   (1 << 1)
#define VEXFS_CREATE_VECTOR_COMPRESS    (1 << 2)
#define VEXFS_CREATE_VECTOR_NUMA_LOCAL  (1 << 3)
#define VEXFS_CREATE_VECTOR_SIMD_ALIGN  (1 << 4)

/* Enhanced search flags */
#define VEXFS_SEARCH_USE_INDEX          (1 << 0)
#define VEXFS_SEARCH_EXACT_MATCH        (1 << 1)
#define VEXFS_SEARCH_RETURN_DISTANCES   (1 << 2)
#define VEXFS_SEARCH_RETURN_VECTORS     (1 << 3)
#define VEXFS_SEARCH_PARALLEL           (1 << 4)

/* Index type constants */
#define VEXFS_INDEX_HNSW                0x01
#define VEXFS_INDEX_IVF                 0x02
#define VEXFS_INDEX_PQ                  0x03
#define VEXFS_INDEX_LSH                 0x04
#define VEXFS_INDEX_FLAT                0x05

/* Index build flags */
#define VEXFS_INDEX_BUILD_PARALLEL      (1 << 0)
#define VEXFS_INDEX_BUILD_OPTIMIZE      (1 << 1)
#define VEXFS_INDEX_BUILD_VALIDATE      (1 << 2)
#define VEXFS_INDEX_BUILD_PERSIST       (1 << 3)

/* Batch operation types */
#define VEXFS_BATCH_INSERT              0x01
#define VEXFS_BATCH_UPDATE              0x02
#define VEXFS_BATCH_DELETE              0x03
#define VEXFS_BATCH_SEARCH              0x04

/* Statistics query types */
#define VEXFS_STATS_GLOBAL              0x01
#define VEXFS_STATS_FILE                0x02
#define VEXFS_STATS_INDEX               0x03
#define VEXFS_STATS_PERFORMANCE         0x04

/* Maximum limits for security */
#define VEXFS_MAX_VECTOR_DIMENSION      65536
#define VEXFS_MAX_BATCH_SIZE            10000
#define VEXFS_MAX_SEARCH_RESULTS        10000
#define VEXFS_MAX_INDEX_PARAMETERS      64

/* 🔥 ENHANCED IOCTL STRUCTURES 🔥 */

/**
 * Enhanced Vector Creation Request
 * 
 * Comprehensive structure for creating vector objects with metadata,
 * validation, and optimization hints.
 */
struct vexfs_create_vector_request {
    /* Vector data */
    __u32 *vector_data;             /* Vector data (IEEE 754 bits) */
    __u32 dimensions;               /* Vector dimensions */
    __u32 element_type;             /* Element type (VEXFS_VECTOR_*) */
    
    /* Metadata */
    __u64 vector_id;                /* Custom vector ID (0 = auto-assign) */
    char metadata[256];             /* Custom metadata string */
    __u32 metadata_size;            /* Size of metadata */
    
    /* Storage options */
    __u32 storage_format;           /* Storage format (VEXFS_STORAGE_*) */
    __u32 compression_type;         /* Compression type (VEXFS_COMPRESS_*) */
    __u32 alignment_bytes;          /* Memory alignment requirement */
    
    /* Flags and options */
    __u32 flags;                    /* Creation flags */
    __u32 numa_node;                /* Preferred NUMA node (-1 = auto) */
    
    /* Output */
    __u64 assigned_id;              /* Assigned vector ID (output) */
    __u32 storage_size;             /* Actual storage size used (output) */
};

/**
 * Enhanced Similarity Search Request
 * 
 * Advanced similarity search with multiple algorithms, filtering,
 * and comprehensive result options.
 */
struct vexfs_enhanced_search_request {
    /* Query vector */
    __u32 *query_vector;            /* Query vector data (IEEE 754 bits) */
    __u32 dimensions;               /* Vector dimensions */
    
    /* Search parameters */
    __u32 k;                        /* Number of results to return */
    __u32 search_algorithm;         /* Search algorithm type */
    __u32 distance_metric;          /* Distance metric (VEXFS_SEARCH_*) */
    
    /* Index parameters */
    __u32 index_type;               /* Index type to use (0 = auto) */
    __u32 ef_search;                /* HNSW ef_search parameter */
    __u32 nprobe;                   /* IVF nprobe parameter */
    
    /* Filtering */
    __u64 *filter_ids;              /* Vector IDs to include/exclude */
    __u32 filter_count;             /* Number of filter IDs */
    __u32 filter_mode;              /* 0 = include, 1 = exclude */
    
    /* Result options */
    __u32 flags;                    /* Search flags */
    __u32 *result_distances;        /* Output: Distance scores (IEEE 754 bits) */
    __u64 *result_ids;              /* Output: Vector IDs */
    __u32 *result_vectors;          /* Output: Result vectors (optional) */
    char *result_metadata;          /* Output: Result metadata (optional) */
    
    /* Output statistics */
    __u32 result_count;             /* Actual number of results */
    __u32 vectors_examined;         /* Number of vectors examined */
    __u64 search_time_ns;           /* Search time in nanoseconds */
};

/**
 * Index Construction Request
 * 
 * Comprehensive index building with parameters for different
 * index types and optimization options.
 */
struct vexfs_build_index_request {
    /* Index configuration */
    __u32 index_type;               /* Index type (VEXFS_INDEX_*) */
    __u32 dimensions;               /* Vector dimensions */
    __u32 vector_count;             /* Expected number of vectors */
    
    /* HNSW parameters */
    __u32 hnsw_m;                   /* HNSW M parameter */
    __u32 hnsw_ef_construction;     /* HNSW ef_construction parameter */
    __u32 hnsw_max_layers;          /* Maximum number of layers */
    
    /* IVF parameters */
    __u32 ivf_clusters;             /* Number of clusters */
    __u32 ivf_training_vectors;     /* Number of training vectors */
    
    /* PQ parameters */
    __u32 pq_subvectors;            /* Number of subvectors */
    __u32 pq_bits_per_code;         /* Bits per quantization code */
    
    /* LSH parameters */
    __u32 lsh_hash_functions;       /* Number of hash functions */
    __u32 lsh_hash_tables;          /* Number of hash tables */
    
    /* Build options */
    __u32 flags;                    /* Build flags */
    __u32 num_threads;              /* Number of threads (0 = auto) */
    __u32 memory_limit_mb;          /* Memory limit in MB (0 = unlimited) */
    
    /* Output statistics */
    __u64 build_time_ns;            /* Build time in nanoseconds */
    __u32 index_size_bytes;         /* Index size in bytes */
    __u32 memory_used_mb;           /* Memory used during build */
    __u32 build_errors;             /* Number of build errors */
};

/**
 * Batch Operations Request
 * 
 * High-throughput batch operations for inserting, updating,
 * deleting, or searching multiple vectors efficiently.
 */
struct vexfs_batch_operations_request {
    /* Operation type */
    __u32 operation_type;           /* Batch operation type */
    __u32 vector_count;             /* Number of vectors */
    __u32 dimensions;               /* Vector dimensions */
    
    /* Vector data */
    __u32 *vectors_data;            /* Vector data array (IEEE 754 bits) */
    __u64 *vector_ids;              /* Vector IDs array */
    char *metadata_array;           /* Metadata array (optional) */
    __u32 metadata_stride;          /* Metadata stride in bytes */
    
    /* Batch parameters */
    __u32 batch_size;               /* Processing batch size */
    __u32 flags;                    /* Operation flags */
    __u32 num_threads;              /* Number of threads (0 = auto) */
    
    /* For batch search */
    __u32 k_per_query;              /* Results per query (search only) */
    __u32 *search_results;          /* Search results (output) */
    __u64 *search_result_ids;       /* Search result IDs (output) */
    
    /* Output statistics */
    __u32 successful_operations;    /* Number of successful operations */
    __u32 failed_operations;        /* Number of failed operations */
    __u64 total_time_ns;            /* Total processing time */
    __u32 *error_codes;             /* Error codes for failed operations */
};

/**
 * Vector Statistics Request
 * 
 * Comprehensive statistics and performance monitoring
 * for vector database operations.
 */
struct vexfs_vector_stats_request {
    /* Query parameters */
    __u32 stats_type;               /* Statistics type */
    __u64 file_id;                  /* File ID (for file-specific stats) */
    __u32 index_type;               /* Index type (for index-specific stats) */
    
    /* Global statistics */
    __u64 total_vectors;            /* Total number of vectors */
    __u64 total_searches;           /* Total number of searches */
    __u64 total_insertions;         /* Total number of insertions */
    __u64 total_deletions;          /* Total number of deletions */
    
    /* Performance statistics */
    __u64 avg_search_time_ns;       /* Average search time */
    __u64 avg_insert_time_ns;       /* Average insertion time */
    __u64 cache_hit_rate;           /* Cache hit rate (percentage * 100) */
    __u64 index_efficiency;         /* Index efficiency (percentage * 100) */
    
    /* Memory statistics */
    __u64 memory_used_bytes;        /* Total memory used */
    __u64 index_memory_bytes;       /* Memory used by indices */
    __u64 vector_memory_bytes;      /* Memory used by vectors */
    __u64 cache_memory_bytes;       /* Memory used by caches */
    
    /* SIMD statistics */
    __u64 simd_operations;          /* Number of SIMD operations */
    __u64 simd_time_saved_ns;       /* Time saved by SIMD */
    __u32 simd_capabilities;        /* Available SIMD capabilities */
    
    /* Index statistics */
    __u32 active_indices;           /* Number of active indices */
    __u32 index_build_count;        /* Number of indices built */
    __u64 index_build_time_total;   /* Total index build time */
    
    /* Error statistics */
    __u32 search_errors;            /* Number of search errors */
    __u32 insert_errors;            /* Number of insertion errors */
    __u32 index_errors;             /* Number of index errors */
};

/* 🔥 ENHANCED IOCTL COMMAND DEFINITIONS 🔥 */

/* Vector creation and management */
#define VEXFS_IOC_CREATE_VECTOR         _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 10, struct vexfs_create_vector_request)
#define VEXFS_IOC_DELETE_VECTOR         _IOW(VEXFS_ENHANCED_IOC_MAGIC, 11, __u64)
#define VEXFS_IOC_UPDATE_VECTOR         _IOW(VEXFS_ENHANCED_IOC_MAGIC, 12, struct vexfs_create_vector_request)

/* Enhanced similarity search */
#define VEXFS_IOC_SIMILARITY_SEARCH     _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 20, struct vexfs_enhanced_search_request)
#define VEXFS_IOC_RANGE_SEARCH          _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 21, struct vexfs_enhanced_search_request)
#define VEXFS_IOC_EXACT_SEARCH          _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 22, struct vexfs_enhanced_search_request)

/* Index construction and management */
#define VEXFS_IOC_BUILD_INDEX           _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 30, struct vexfs_build_index_request)
#define VEXFS_IOC_REBUILD_INDEX         _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 31, struct vexfs_build_index_request)
#define VEXFS_IOC_DROP_INDEX            _IOW(VEXFS_ENHANCED_IOC_MAGIC, 32, __u32)
#define VEXFS_IOC_OPTIMIZE_INDEX        _IOW(VEXFS_ENHANCED_IOC_MAGIC, 33, __u32)

/* Batch operations */
#define VEXFS_IOC_BATCH_OPERATIONS      _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 40, struct vexfs_batch_operations_request)
#define VEXFS_IOC_BATCH_INSERT_VECTORS  _IOW(VEXFS_ENHANCED_IOC_MAGIC, 41, struct vexfs_batch_operations_request)
#define VEXFS_IOC_BATCH_SEARCH_VECTORS  _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 42, struct vexfs_batch_operations_request)

/* Statistics and monitoring */
#define VEXFS_IOC_GET_VECTOR_STATS      _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 50, struct vexfs_vector_stats_request)
#define VEXFS_IOC_RESET_STATS           _IO(VEXFS_ENHANCED_IOC_MAGIC, 51)
#define VEXFS_IOC_GET_PERFORMANCE_STATS _IOR(VEXFS_ENHANCED_IOC_MAGIC, 52, struct vexfs_vector_stats_request)

/* System and configuration */
#define VEXFS_IOC_GET_CAPABILITIES      _IOR(VEXFS_ENHANCED_IOC_MAGIC, 60, __u32)
#define VEXFS_IOC_SET_CONFIG            _IOW(VEXFS_ENHANCED_IOC_MAGIC, 61, __u32)
#define VEXFS_IOC_FLUSH_CACHES          _IO(VEXFS_ENHANCED_IOC_MAGIC, 62)

/* 🔥 FUNCTION DECLARATIONS 🔥 */

/* Main enhanced ioctl handler */
long vexfs_enhanced_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/* Vector creation and management */
int vexfs_ioctl_create_vector(struct file *file, struct vexfs_create_vector_request __user *arg);
int vexfs_ioctl_delete_vector(struct file *file, __u64 __user *arg);
int vexfs_ioctl_update_vector(struct file *file, struct vexfs_create_vector_request __user *arg);

/* Enhanced search operations */
int vexfs_ioctl_similarity_search(struct file *file, struct vexfs_enhanced_search_request __user *arg);
int vexfs_ioctl_range_search(struct file *file, struct vexfs_enhanced_search_request __user *arg);
int vexfs_ioctl_exact_search(struct file *file, struct vexfs_enhanced_search_request __user *arg);

/* Index management */
int vexfs_ioctl_build_index(struct file *file, struct vexfs_build_index_request __user *arg);
int vexfs_ioctl_rebuild_index(struct file *file, struct vexfs_build_index_request __user *arg);
int vexfs_ioctl_drop_index(struct file *file, __u32 __user *arg);
int vexfs_ioctl_optimize_index(struct file *file, __u32 __user *arg);

/* Batch operations */
int vexfs_ioctl_batch_operations(struct file *file, struct vexfs_batch_operations_request __user *arg);
int vexfs_ioctl_batch_insert(struct file *file, struct vexfs_batch_operations_request __user *arg);
int vexfs_ioctl_batch_search(struct file *file, struct vexfs_batch_operations_request __user *arg);

/* Statistics and monitoring */
int vexfs_ioctl_get_stats(struct file *file, struct vexfs_vector_stats_request __user *arg);
int vexfs_ioctl_reset_stats(struct file *file);
int vexfs_ioctl_get_performance_stats(struct file *file, struct vexfs_vector_stats_request __user *arg);

/* System operations */
int vexfs_ioctl_get_capabilities(struct file *file, __u32 __user *arg);
int vexfs_ioctl_set_config(struct file *file, __u32 __user *arg);
int vexfs_ioctl_flush_caches(struct file *file);

/* Security and validation */
int vexfs_validate_ioctl_request(struct file *file, unsigned int cmd, unsigned long arg);
int vexfs_check_vector_permissions(struct file *file, __u32 operation);
int vexfs_validate_vector_data(const __u32 *data, __u32 dimensions, __u32 element_type);
int vexfs_validate_search_params(const struct vexfs_enhanced_search_request *req);
int vexfs_validate_index_params(const struct vexfs_build_index_request *req);
int vexfs_validate_batch_params(const struct vexfs_batch_operations_request *req);

/* Error handling and logging */
void vexfs_log_ioctl_error(struct file *file, unsigned int cmd, int error, const char *operation);
void vexfs_log_ioctl_performance(struct file *file, unsigned int cmd, __u64 duration_ns);

/* Utility functions */
bool vexfs_is_valid_vector_id(__u64 vector_id);
bool vexfs_is_valid_dimension(__u32 dimensions);
bool vexfs_is_valid_element_type(__u32 element_type);
bool vexfs_is_valid_index_type(__u32 index_type);
__u32 vexfs_calculate_vector_size(__u32 dimensions, __u32 element_type);
__u32 vexfs_estimate_index_size(__u32 vector_count, __u32 dimensions, __u32 index_type);

#endif /* VEXFS_V2_ENHANCED_IOCTL_H */