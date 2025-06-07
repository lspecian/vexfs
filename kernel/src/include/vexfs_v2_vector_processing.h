/*
 * VexFS v2.0 SIMD-Accelerated Vector Processing Header
 * 
 * This header defines SIMD-accelerated functions for vector normalization
 * and quantization in kernel space, implementing Task 49 requirements.
 * 
 * Features:
 * - L2 normalization using SIMD instructions
 * - Scalar quantization (float32 to int8/uint8)
 * - Product quantization with codebook generation
 * - Binary quantization for compact storage
 * - Proper kernel FPU handling with fallback scalar versions
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_V2_VECTOR_PROCESSING_H
#define VEXFS_V2_VECTOR_PROCESSING_H

#ifdef __KERNEL__
#include <linux/types.h>
#include <linux/kernel.h>
#include <asm/fpu/api.h>
#include <asm/cpufeature.h>
#else
#include <stdint.h>
/* Userspace type compatibility */
typedef uint8_t  __u8;
typedef uint16_t __u16;
typedef uint32_t __u32;
typedef uint64_t __u64;
typedef int8_t   __s8;
typedef int16_t  __s16;
typedef int32_t  __s32;
typedef int64_t  __s64;
#endif

#include "vexfs_v2_uapi.h"

/*
 * SIMD Capability Detection
 */
#define VEXFS_SIMD_NONE     0x00
#define VEXFS_SIMD_SSE2     0x01
#define VEXFS_SIMD_AVX2     0x02
#define VEXFS_SIMD_AVX512   0x04
#define VEXFS_SIMD_NEON     0x08

/*
 * Vector Processing Operation Types
 */
#define VEXFS_OP_L2_NORMALIZE       0x01
#define VEXFS_OP_SCALAR_QUANTIZE    0x02
#define VEXFS_OP_PRODUCT_QUANTIZE   0x03
#define VEXFS_OP_BINARY_QUANTIZE    0x04

/*
 * Quantization Types
 */
#define VEXFS_QUANT_INT8            0x01
#define VEXFS_QUANT_UINT8           0x02
#define VEXFS_QUANT_INT16           0x03
#define VEXFS_QUANT_UINT16          0x04

/*
 * Product Quantization Configuration
 */
struct vexfs_pq_config {
    __u32 subvector_count;      /* Number of subvectors */
    __u32 subvector_dims;       /* Dimensions per subvector */
    __u32 codebook_size;        /* Size of each codebook (typically 256) */
    __u32 training_iterations;  /* K-means training iterations */
    __u32 reserved[4];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Vector Processing Request Structure
 */
struct vexfs_vector_processing_request {
    __u32 operation_type;       /* VEXFS_OP_* */
    __u32 input_format;         /* VEXFS_VECTOR_* */
    __u32 output_format;        /* VEXFS_VECTOR_* */
    __u32 dimensions;           /* Vector dimensions */
    __u32 vector_count;         /* Number of vectors to process */
    
    /* Input data (IEEE 754 bits for kernel compatibility) */
    __u32 *input_vectors_bits;  /* Input vector data */
    
    /* Output data */
    union {
        __u32 *output_vectors_bits;     /* For normalization */
        __u8  *quantized_int8;          /* For int8 quantization */
        __u8  *quantized_uint8;         /* For uint8 quantization */
        __u8  *binary_codes;            /* For binary quantization */
        __u8  *pq_codes;                /* For product quantization */
    } output;
    
    /* Configuration for specific operations */
    union {
        struct {
            __u32 scale_factor_bits;    /* Scaling factor (IEEE 754 bits) */
            __u32 offset_bits;          /* Offset value (IEEE 754 bits) */
        } scalar_quant;
        
        struct vexfs_pq_config pq;
        
        struct {
            __u32 threshold_bits;       /* Binary threshold (IEEE 754 bits) */
        } binary_quant;
    } config;
    
    /* Performance metrics */
    __u64 processing_time_ns;   /* Processing time */
    __u32 simd_level_used;      /* SIMD level actually used */
    __u32 vectors_processed;    /* Actual vectors processed */
    __u32 reserved[4];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Batch Processing Configuration
 */
#define VEXFS_BATCH_SIZE_DEFAULT    32      /* Default batch size */
#define VEXFS_BATCH_SIZE_MAX        256     /* Maximum batch size */
#define VEXFS_BATCH_SIZE_MIN        4       /* Minimum batch size */

/*
 * Batch Operation Types
 */
#define VEXFS_BATCH_OP_L2_NORMALIZE     0x01
#define VEXFS_BATCH_OP_SCALAR_QUANTIZE  0x02
#define VEXFS_BATCH_OP_PRODUCT_QUANTIZE 0x03
#define VEXFS_BATCH_OP_BINARY_QUANTIZE  0x04
#define VEXFS_BATCH_OP_DISTANCE_CALC    0x05
#define VEXFS_BATCH_OP_HNSW_INSERT      0x06

/*
 * Batch Processing Request Structure
 */
struct vexfs_batch_processing_request {
    __u32 operation_type;           /* VEXFS_BATCH_OP_* */
    __u32 batch_size;               /* Number of vectors in this batch */
    __u32 dimensions;               /* Vector dimensions */
    __u32 input_format;             /* VEXFS_VECTOR_* */
    __u32 output_format;            /* VEXFS_VECTOR_* */
    
    /* Input data arrays */
    __u32 *input_vectors_bits;      /* Array of input vectors */
    
    /* Output data arrays */
    union {
        __u32 *output_vectors_bits;     /* For normalization */
        __u8  *quantized_output;        /* For quantization operations */
        __u32 *distance_results;        /* For distance calculations */
        __u64 *hnsw_node_ids;           /* For HNSW insertions */
    } output;
    
    /* Operation-specific configuration */
    union {
        struct {
            __u32 scale_factor_bits;    /* Scaling factor (IEEE 754 bits) */
            __u32 offset_bits;          /* Offset value (IEEE 754 bits) */
        } scalar_quant;
        
        struct vexfs_pq_config pq;
        
        struct {
            __u32 threshold_bits;       /* Binary threshold (IEEE 754 bits) */
        } binary_quant;
        
        struct {
            __u32 *reference_vectors;   /* Reference vectors for distance calc */
            __u32 distance_metric;      /* Distance metric type */
        } distance;
        
        struct {
            __u32 layer;                /* HNSW layer for insertion */
            __u32 max_connections;      /* Maximum connections per node */
        } hnsw;
    } config;
    
    /* Performance metrics */
    __u64 processing_time_ns;       /* Total processing time */
    __u64 fpu_context_switches;     /* Number of FPU context switches */
    __u32 simd_level_used;          /* SIMD level actually used */
    __u32 vectors_processed;        /* Actual vectors processed */
    __u32 reserved[4];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Batch Work Queue Item
 */
struct vexfs_batch_work_item {
    struct work_struct work;        /* Kernel work structure */
    struct vexfs_batch_processing_request *request;
    void (*completion_callback)(struct vexfs_batch_work_item *item, int result);
    void *callback_data;
    int result;
    atomic_t ref_count;
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Vector Processing Statistics
 */
struct vexfs_vector_processing_stats {
    __u64 total_operations;
    __u64 l2_normalizations;
    __u64 scalar_quantizations;
    __u64 product_quantizations;
    __u64 binary_quantizations;
    
    /* Performance metrics */
    __u64 total_processing_time_ns;
    __u64 avg_processing_time_ns;
    __u64 simd_accelerated_ops;
    __u64 scalar_fallback_ops;
    
    /* SIMD usage statistics */
    __u64 sse2_operations;
    __u64 avx2_operations;
    __u64 avx512_operations;
    __u64 neon_operations;
    
    /* Batch processing statistics */
    __u64 batch_operations;
    __u64 total_fpu_context_switches;
    __u64 avg_batch_size;
    __u64 batch_processing_time_ns;
    __u64 fpu_context_switch_savings;
    
    __u32 reserved[3];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * IOCTL Commands for Vector Processing
 */
#define VEXFS_IOC_VECTOR_PROCESS    _IOWR(VEXFS_IOC_MAGIC, 30, struct vexfs_vector_processing_request)
#define VEXFS_IOC_GET_PROC_STATS    _IOR(VEXFS_IOC_MAGIC, 31, struct vexfs_vector_processing_stats)
#define VEXFS_IOC_GET_SIMD_CAPS     _IOR(VEXFS_IOC_MAGIC, 32, __u32)
#define VEXFS_IOC_BATCH_PROCESS     _IOWR(VEXFS_IOC_MAGIC, 33, struct vexfs_batch_processing_request)

#ifdef __KERNEL__

/*
 * Core Vector Processing Functions
 */

/* SIMD capability detection */
__u32 vexfs_detect_simd_capabilities(void);

/* L2 Normalization Functions */
int vexfs_l2_normalize_vectors(const __u32 *input_bits, __u32 *output_bits,
                              __u32 dimensions, __u32 vector_count);
int vexfs_l2_normalize_vectors_simd(const __u32 *input_bits, __u32 *output_bits,
                                   __u32 dimensions, __u32 vector_count, __u32 simd_level);
int vexfs_l2_normalize_vectors_scalar(const __u32 *input_bits, __u32 *output_bits,
                                     __u32 dimensions, __u32 vector_count);

/* Scalar Quantization Functions */
int vexfs_scalar_quantize_int8(const __u32 *input_bits, __s8 *output,
                              __u32 dimensions, __u32 vector_count,
                              __u32 scale_bits, __u32 offset_bits);
int vexfs_scalar_quantize_uint8(const __u32 *input_bits, __u8 *output,
                               __u32 dimensions, __u32 vector_count,
                               __u32 scale_bits, __u32 offset_bits);
int vexfs_scalar_quantize_simd(const __u32 *input_bits, void *output,
                              __u32 dimensions, __u32 vector_count,
                              __u32 quant_type, __u32 scale_bits, __u32 offset_bits,
                              __u32 simd_level);

/* Product Quantization Functions */
int vexfs_product_quantize(const __u32 *input_bits, __u8 *output_codes,
                          __u32 dimensions, __u32 vector_count,
                          const struct vexfs_pq_config *config);
int vexfs_product_quantize_with_codebooks(const __u32 *input_bits, __u8 *output_codes,
                                         __u32 dimensions, __u32 vector_count,
                                         const struct vexfs_pq_config *config,
                                         const __u32 *codebooks_bits);
int vexfs_train_pq_codebooks(const __u32 *training_data_bits,
                            __u32 dimensions, __u32 training_count,
                            const struct vexfs_pq_config *config,
                            __u32 *codebooks_bits);

/*
 * I/O Path Optimization for Vector Data (Task 56)
 */

/* I/O Operation Types */
#define VEXFS_IO_OP_READ            0x01
#define VEXFS_IO_OP_WRITE           0x02
#define VEXFS_IO_OP_READAHEAD       0x03
#define VEXFS_IO_OP_DIRECT_IO       0x04
#define VEXFS_IO_OP_ASYNC_WRITE     0x05

/* I/O Optimization Flags */
#define VEXFS_IO_FLAG_VECTOR_AWARE  0x01    /* Vector-aware readahead */
#define VEXFS_IO_FLAG_EXTENT_OPT    0x02    /* Extent allocation optimization */
#define VEXFS_IO_FLAG_ASYNC         0x04    /* Asynchronous I/O */
#define VEXFS_IO_FLAG_DIRECT        0x08    /* Direct I/O bypass page cache */
#define VEXFS_IO_FLAG_SEQUENTIAL    0x10    /* Sequential access pattern */
#define VEXFS_IO_FLAG_RANDOM        0x20    /* Random access pattern */
#define VEXFS_IO_FLAG_BATCH         0x40    /* Batch I/O operations */

/* Vector Access Patterns */
#define VEXFS_ACCESS_SEQUENTIAL     0x01    /* Sequential vector access */
#define VEXFS_ACCESS_RANDOM         0x02    /* Random vector access */
#define VEXFS_ACCESS_CLUSTERED      0x03    /* Clustered vector access */
#define VEXFS_ACCESS_SIMILARITY     0x04    /* Similarity-based access */

/* I/O Scheduler Types */
#define VEXFS_SCHED_VECTOR_CFQ      0x01    /* Vector-aware CFQ */
#define VEXFS_SCHED_VECTOR_DEADLINE 0x02    /* Vector-aware deadline */
#define VEXFS_SCHED_VECTOR_NOOP     0x03    /* Vector-aware noop */

/*
 * Vector-Aware Readahead Configuration
 */
struct vexfs_readahead_config {
    __u32 window_size;              /* Readahead window size in bytes */
    __u32 vector_cluster_size;      /* Vectors to read together */
    __u32 access_pattern;           /* Expected access pattern */
    __u32 similarity_threshold;     /* Threshold for similarity-based readahead */
    __u32 max_readahead_vectors;    /* Maximum vectors to readahead */
    __u32 adaptive_window;          /* Enable adaptive window sizing */
    __u32 reserved[2];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Extent Allocation Configuration
 */
struct vexfs_extent_config {
    __u32 min_extent_size;          /* Minimum extent size in blocks */
    __u32 max_extent_size;          /* Maximum extent size in blocks */
    __u32 vector_alignment;         /* Vector data alignment requirement */
    __u32 fragmentation_threshold;  /* Fragmentation threshold percentage */
    __u32 preallocation_size;       /* Preallocation size for vector files */
    __u32 cluster_allocation;       /* Enable clustered allocation */
    __u32 reserved[2];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Asynchronous I/O Configuration
 */
struct vexfs_async_io_config {
    __u32 max_concurrent_ops;       /* Maximum concurrent async operations */
    __u32 queue_depth;              /* I/O queue depth */
    __u32 batch_size;               /* Batch size for async operations */
    __u32 completion_timeout_ms;    /* Completion timeout in milliseconds */
    __u32 priority;                 /* I/O priority level */
    __u32 numa_node;                /* Preferred NUMA node for I/O */
    __u32 reserved[2];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Direct I/O Configuration
 */
struct vexfs_direct_io_config {
    __u32 min_transfer_size;        /* Minimum transfer size for direct I/O */
    __u32 max_transfer_size;        /* Maximum transfer size for direct I/O */
    __u32 alignment_requirement;    /* Memory alignment requirement */
    __u32 bypass_threshold;         /* Size threshold to bypass page cache */
    __u32 vector_batch_size;        /* Vectors per direct I/O operation */
    __u32 enable_zero_copy;         /* Enable zero-copy transfers */
    __u32 reserved[2];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * I/O Path Optimization Request
 */
struct vexfs_io_optimization_request {
    __u32 operation_type;           /* VEXFS_IO_OP_* */
    __u32 optimization_flags;       /* VEXFS_IO_FLAG_* */
    __u32 vector_count;             /* Number of vectors involved */
    __u32 vector_dimensions;        /* Vector dimensions */
    __u64 file_offset;              /* File offset for I/O operation */
    __u64 data_size;                /* Size of data to transfer */
    
    /* Configuration structures */
    struct vexfs_readahead_config readahead;
    struct vexfs_extent_config extent;
    struct vexfs_async_io_config async_io;
    struct vexfs_direct_io_config direct_io;
    
    /* Performance metrics */
    __u64 io_start_time_ns;         /* I/O operation start time */
    __u64 io_completion_time_ns;    /* I/O operation completion time */
    __u64 bytes_transferred;        /* Actual bytes transferred */
    __u32 extents_allocated;        /* Number of extents allocated */
    __u32 readahead_hits;           /* Readahead cache hits */
    __u32 readahead_misses;         /* Readahead cache misses */
    __u32 async_operations;         /* Number of async operations */
    __u32 direct_io_operations;     /* Number of direct I/O operations */
    __u32 reserved[3];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * I/O Performance Statistics
 */
struct vexfs_io_performance_stats {
    /* General I/O statistics */
    __u64 total_read_operations;
    __u64 total_write_operations;
    __u64 total_bytes_read;
    __u64 total_bytes_written;
    __u64 total_io_time_ns;
    __u64 avg_io_latency_ns;
    
    /* Vector-specific I/O statistics */
    __u64 vector_read_operations;
    __u64 vector_write_operations;
    __u64 vectors_read;
    __u64 vectors_written;
    __u64 vector_io_time_ns;
    
    /* Readahead statistics */
    __u64 readahead_operations;
    __u64 readahead_hits;
    __u64 readahead_misses;
    __u64 readahead_bytes;
    __u64 readahead_efficiency_percent;
    
    /* Extent allocation statistics */
    __u64 extents_allocated;
    __u64 extent_fragmentation_percent;
    __u64 avg_extent_size;
    __u64 preallocation_hits;
    __u64 preallocation_misses;
    
    /* Asynchronous I/O statistics */
    __u64 async_read_operations;
    __u64 async_write_operations;
    __u64 async_completion_time_ns;
    __u64 async_queue_depth_avg;
    __u64 async_batch_efficiency_percent;
    
    /* Direct I/O statistics */
    __u64 direct_read_operations;
    __u64 direct_write_operations;
    __u64 direct_io_bytes;
    __u64 direct_io_time_ns;
    __u64 zero_copy_operations;
    
    __u32 reserved[4];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * I/O Scheduler Configuration
 */
struct vexfs_io_scheduler_config {
    __u32 scheduler_type;           /* VEXFS_SCHED_* */
    __u32 vector_priority_boost;    /* Priority boost for vector operations */
    __u32 batch_merge_threshold;    /* Threshold for merging I/O requests */
    __u32 seek_penalty;             /* Seek penalty for random access */
    __u32 read_ahead_factor;        /* Read-ahead factor for sequential access */
    __u32 write_back_delay_ms;      /* Write-back delay in milliseconds */
    __u32 reserved[2];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * IOCTL Commands for I/O Path Optimization
 */
#define VEXFS_IOC_IO_OPTIMIZE       _IOWR(VEXFS_IOC_MAGIC, 34, struct vexfs_io_optimization_request)
#define VEXFS_IOC_GET_IO_STATS      _IOR(VEXFS_IOC_MAGIC, 35, struct vexfs_io_performance_stats)
#define VEXFS_IOC_SET_IO_SCHEDULER  _IOW(VEXFS_IOC_MAGIC, 36, struct vexfs_io_scheduler_config)
#define VEXFS_IOC_GET_IO_SCHEDULER  _IOR(VEXFS_IOC_MAGIC, 37, struct vexfs_io_scheduler_config)

#ifdef __KERNEL__

/*
 * I/O Path Optimization Functions
 */

/* Vector-aware readahead functions */
int vexfs_vector_readahead_init(struct file *file, struct vexfs_readahead_config *config);
int vexfs_vector_readahead_predict(struct file *file, loff_t offset, size_t count,
                                  loff_t *readahead_offset, size_t *readahead_size);
int vexfs_vector_readahead_execute(struct file *file, loff_t offset, size_t size);
void vexfs_vector_readahead_update_pattern(struct file *file, loff_t offset, size_t count);

/* Extent allocation optimization functions */
int vexfs_extent_allocate_optimized(struct inode *inode, __u64 start_block,
                                   __u32 block_count, struct vexfs_extent_config *config);
int vexfs_extent_preallocation(struct inode *inode, __u64 expected_size,
                              struct vexfs_extent_config *config);
int vexfs_extent_defragment(struct inode *inode, struct vexfs_extent_config *config);
__u32 vexfs_extent_calculate_fragmentation(struct inode *inode);

/* Asynchronous I/O functions */
int vexfs_async_io_init(struct vexfs_async_io_config *config);
int vexfs_async_read_vectors(struct file *file, loff_t offset, size_t count,
                           struct vexfs_async_io_config *config);
int vexfs_async_write_vectors(struct file *file, loff_t offset, const void *data,
                            size_t count, struct vexfs_async_io_config *config);
int vexfs_async_io_wait_completion(struct file *file, __u32 timeout_ms);
void vexfs_async_io_cleanup(void);

/* Direct I/O functions */
int vexfs_direct_io_read(struct file *file, loff_t offset, void *buffer,
                        size_t count, struct vexfs_direct_io_config *config);
int vexfs_direct_io_write(struct file *file, loff_t offset, const void *buffer,
                         size_t count, struct vexfs_direct_io_config *config);
int vexfs_direct_io_vector_transfer(struct file *file, loff_t offset,
                                   void *vectors, __u32 vector_count, __u32 dimensions,
                                   bool is_write, struct vexfs_direct_io_config *config);

/* I/O scheduler functions */
int vexfs_io_scheduler_init(struct vexfs_io_scheduler_config *config);
int vexfs_io_scheduler_queue_request(struct bio *bio, __u32 vector_priority);
int vexfs_io_scheduler_merge_requests(struct bio *bio1, struct bio *bio2);
void vexfs_io_scheduler_cleanup(void);

/* I/O performance monitoring functions */
void vexfs_io_stats_init(void);
void vexfs_io_stats_record_operation(__u32 operation_type, __u64 bytes, __u64 latency_ns);
void vexfs_io_stats_record_readahead(__u32 hits, __u32 misses, __u64 bytes);
void vexfs_io_stats_record_extent_allocation(__u32 extents, __u64 total_size);
void vexfs_io_stats_record_async_operation(__u64 completion_time_ns, __u32 queue_depth);
void vexfs_io_stats_record_direct_io(__u64 bytes, __u64 latency_ns, bool zero_copy);
int vexfs_io_stats_get(struct vexfs_io_performance_stats *stats);
void vexfs_io_stats_cleanup(void);

/* Utility functions */
__u32 vexfs_io_detect_access_pattern(struct file *file, loff_t offset, size_t count);
__u32 vexfs_io_calculate_optimal_batch_size(__u32 vector_dimensions, __u32 vector_count);
int vexfs_io_align_transfer(loff_t *offset, size_t *count, __u32 alignment);
bool vexfs_io_should_use_direct_io(struct file *file, size_t transfer_size);

#endif /* __KERNEL__ */

/* Continue with existing function declarations */
int vexfs_train_pq_codebooks_continued(const __u32 *training_data_bits,
                            __u32 dimensions, __u32 training_count,
                            const struct vexfs_pq_config *config,
                            __u32 *codebooks_bits);
int vexfs_train_pq_codebooks_kmeans(const __u32 *training_data_bits,
                                   __u32 dimensions, __u32 training_count,
                                   const struct vexfs_pq_config *config,
                                   __u32 *codebooks_bits);
__u32 vexfs_compute_subvector_distance(const __u32 *vec1_bits, const __u32 *vec2_bits,
                                      __u32 dimensions);
int vexfs_pq_search_with_codes(const __u32 *query_bits, const __u8 *pq_codes,
                              __u32 dimensions, __u32 vector_count,
                              const struct vexfs_pq_config *config,
                              const __u32 *codebooks_bits,
                              __u32 *result_indices, __u32 k);

/* Binary Quantization Functions */
int vexfs_binary_quantize(const __u32 *input_bits, __u8 *output_codes,
                         __u32 dimensions, __u32 vector_count,
                         __u32 threshold_bits);
int vexfs_binary_quantize_simd(const __u32 *input_bits, __u8 *output_codes,
                              __u32 dimensions, __u32 vector_count,
                              __u32 threshold_bits, __u32 simd_level);

/* SIMD-specific implementations */
#ifdef CONFIG_X86_64
int vexfs_l2_normalize_avx2(const __u32 *input_bits, __u32 *output_bits,
                           __u32 dimensions, __u32 vector_count);
int vexfs_l2_normalize_avx512(const __u32 *input_bits, __u32 *output_bits,
                             __u32 dimensions, __u32 vector_count);
int vexfs_scalar_quantize_avx2(const __u32 *input_bits, void *output,
                               __u32 dimensions, __u32 vector_count,
                               __u32 quant_type, __u32 scale_bits, __u32 offset_bits);
int vexfs_binary_quantize_avx2(const __u32 *input_bits, __u8 *output_codes,
                               __u32 dimensions, __u32 vector_count,
                               __u32 threshold_bits);
int vexfs_product_quantize_avx2(const __u32 *input_bits, __u8 *output_codes,
                               __u32 dimensions, __u32 vector_count,
                               const struct vexfs_pq_config *config,
                               const __u32 *codebooks_bits);
#endif

#ifdef CONFIG_ARM64
int vexfs_l2_normalize_neon(const __u32 *input_bits, __u32 *output_bits,
                           __u32 dimensions, __u32 vector_count);
int vexfs_scalar_quantize_neon(const __u32 *input_bits, void *output,
                              __u32 dimensions, __u32 vector_count,
                              __u32 quant_type, __u32 scale_bits, __u32 offset_bits);
int vexfs_binary_quantize_neon(const __u32 *input_bits, __u8 *output_codes,
                              __u32 dimensions, __u32 vector_count,
                              __u32 threshold_bits);
#endif

/* Utility Functions */
__u32 vexfs_ieee754_sqrt(__u32 input_bits);
__u32 vexfs_ieee754_reciprocal(__u32 input_bits);
void vexfs_ieee754_vector_scale(const __u32 *input_bits, __u32 *output_bits,
                               __u32 count, __u32 scale_bits);

/* PQ-HNSW Integration Functions */
int vexfs_hybrid_pq_hnsw_search(const __u32 *query_bits, __u32 dimensions,
                                const struct vexfs_pq_config *pq_config,
                                const __u8 *pq_codes, const __u32 *codebooks_bits,
                                __u32 vector_count, __u32 k,
                                struct vexfs_search_result *results,
                                __u32 *result_count);

int vexfs_create_pq_enhanced_hnsw_node(__u64 vector_id, const __u32 *vector_bits,
                                       __u32 dimensions, const struct vexfs_pq_config *pq_config,
                                       const __u32 *codebooks_bits, __u8 *pq_codes_out);

__u32 vexfs_pq_approximate_distance(const __u8 *pq_codes1, const __u8 *pq_codes2,
                                   const struct vexfs_pq_config *pq_config,
                                   const __u32 *codebooks_bits);

int vexfs_batch_pq_encode_for_hnsw(const __u32 *vectors_bits, __u32 vector_count,
                                  __u32 dimensions, const struct vexfs_pq_config *pq_config,
                                  const __u32 *codebooks_bits, __u8 *pq_codes_out);

int vexfs_pq_hnsw_integrated_search(const __u32 *query_vector, __u32 dimensions,
                                    __u32 k, __u32 distance_metric,
                                    struct vexfs_search_result *results,
                                    __u32 *result_count);

/* Batch Processing Functions */
int vexfs_batch_process_vectors(struct vexfs_batch_processing_request *request);
int vexfs_batch_l2_normalize(const __u32 *input_bits, __u32 *output_bits,
                             __u32 dimensions, __u32 batch_size);
int vexfs_batch_scalar_quantize(const __u32 *input_bits, void *output,
                                __u32 dimensions, __u32 batch_size,
                                __u32 quant_type, __u32 scale_bits, __u32 offset_bits);
int vexfs_batch_product_quantize(const __u32 *input_bits, __u8 *output_codes,
                                 __u32 dimensions, __u32 batch_size,
                                 const struct vexfs_pq_config *config,
                                 const __u32 *codebooks_bits);
int vexfs_batch_binary_quantize(const __u32 *input_bits, __u8 *output_codes,
                                __u32 dimensions, __u32 batch_size,
                                __u32 threshold_bits);
int vexfs_batch_distance_calculate(const __u32 *vectors1_bits, const __u32 *vectors2_bits,
                                   __u32 *distances, __u32 dimensions, __u32 batch_size,
                                   __u32 distance_metric);
int vexfs_batch_hnsw_insert(const __u32 *vectors_bits, __u64 *node_ids,
                            __u32 dimensions, __u32 batch_size,
                            __u32 layer, __u32 max_connections);

/* Asynchronous Batch Processing */
int vexfs_submit_batch_work(struct vexfs_batch_processing_request *request,
                           void (*completion_callback)(struct vexfs_batch_work_item *item, int result),
                           void *callback_data);
void vexfs_batch_work_handler(struct work_struct *work);
void vexfs_batch_work_cleanup(struct vexfs_batch_work_item *item);

/* Batch Processing Optimization */
__u32 vexfs_calculate_optimal_batch_size(__u32 dimensions, __u32 operation_type,
                                        __u32 available_memory);
int vexfs_batch_processing_init(void);
void vexfs_batch_processing_exit(void);

/* Statistics and Monitoring */
void vexfs_get_vector_processing_stats(struct vexfs_vector_processing_stats *stats);
void vexfs_reset_vector_processing_stats(void);

/* Module initialization */
int vexfs_vector_processing_init(void);
void vexfs_vector_processing_exit(void);

/* IOCTL handler */
long vexfs_vector_processing_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

#endif /* __KERNEL__ */

#endif /* VEXFS_V2_VECTOR_PROCESSING_H */