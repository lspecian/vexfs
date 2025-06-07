/*
 * VexFS Vector Data Block Layout Optimization
 * Task 42: Optimize Vector Data Block Layout
 * 
 * This module implements SIMD-aligned vector storage and efficient
 * block allocation algorithms optimized for vector database workloads.
 */

#ifndef _VEXFS_VECTOR_BLOCK_LAYOUT_H
#define _VEXFS_VECTOR_BLOCK_LAYOUT_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>

/* Include vector inode definitions from Task 41 */
#include "vexfs_vector_inode.h"

/* Block layout constants */
#define VEXFS_BLOCK_SIZE                4096
#define VEXFS_VECTOR_BLOCK_MAGIC        0x56454342  /* "VECB" */
#define VEXFS_MAX_VECTORS_PER_BLOCK     256
#define VEXFS_VECTOR_BLOCK_HEADER_SIZE  64

/* Vector block allocation strategies */
enum vexfs_vector_alloc_strategy {
    VEXFS_ALLOC_CONTIGUOUS = 0,     /* Prefer contiguous allocation */
    VEXFS_ALLOC_ALIGNED = 1,        /* Prefer SIMD-aligned allocation */
    VEXFS_ALLOC_PACKED = 2,         /* Pack small vectors efficiently */
    VEXFS_ALLOC_SPARSE = 3,         /* Optimize for sparse vectors */
    VEXFS_ALLOC_COMPRESSED = 4,     /* Optimize for compressed vectors */
};

/* Vector block layout types */
enum vexfs_vector_block_type {
    VEXFS_BLOCK_VECTOR_DATA = 0,    /* Raw vector data */
    VEXFS_BLOCK_VECTOR_INDEX = 1,   /* Vector index data */
    VEXFS_BLOCK_VECTOR_META = 2,    /* Vector metadata */
    VEXFS_BLOCK_VECTOR_MIXED = 3,   /* Mixed content block */
};

/* Vector packing efficiency levels */
enum vexfs_vector_packing {
    VEXFS_PACK_NONE = 0,            /* No packing optimization */
    VEXFS_PACK_TIGHT = 1,           /* Tight packing for small vectors */
    VEXFS_PACK_ALIGNED = 2,         /* SIMD-aligned packing */
    VEXFS_PACK_QUANTIZED = 3,       /* Quantized vector packing */
};

/*
 * Vector Block Header Structure
 * 
 * This header appears at the beginning of each vector data block
 * and provides metadata for efficient vector access and SIMD operations.
 */
struct vexfs_vector_block_header {
    __u32 magic;                    /* Block magic number (VECB) */
    __u32 block_type;               /* Type of vector block */
    __u32 vector_count;             /* Number of vectors in this block */
    __u32 vector_dimension;         /* Dimension of vectors in this block */
    
    __u8  element_type;             /* Vector element type */
    __u8  simd_alignment;           /* SIMD alignment used */
    __u8  packing_type;             /* Packing strategy used */
    __u8  compression_type;         /* Compression algorithm */
    
    __u32 data_offset;              /* Offset to vector data */
    __u32 data_size;                /* Size of vector data */
    __u32 index_offset;             /* Offset to index data */
    __u32 index_size;               /* Size of index data */
    
    __u64 block_checksum;           /* Block integrity checksum */
    __u64 creation_time;            /* Block creation timestamp */
    
    /* Vector layout information */
    __u32 vectors_per_row;          /* Vectors per cache line */
    __u32 row_stride;               /* Bytes between vector rows */
    __u32 vector_stride;            /* Bytes between vectors */
    __u32 alignment_padding;        /* Padding for alignment */
    
    /* Performance optimization hints */
    __u32 access_pattern;           /* Expected access pattern */
    __u32 prefetch_distance;        /* Prefetch optimization */
    __u32 cache_hint;               /* Cache behavior hint */
    __u32 numa_node;                /* Preferred NUMA node */
    
    /* Reserved for future use */
    __u32 reserved[4];
} __attribute__((packed));

/*
 * Vector Block Allocation Request
 * 
 * This structure describes a request for vector block allocation
 * with specific optimization requirements.
 */
struct vexfs_vector_alloc_request {
    /* Vector characteristics */
    __u32 vector_count;             /* Number of vectors to allocate */
    __u16 vector_dimension;         /* Vector dimensionality */
    __u8  element_type;             /* Vector element type */
    __u8  simd_alignment;           /* Required SIMD alignment */
    
    /* Allocation preferences */
    enum vexfs_vector_alloc_strategy strategy;
    enum vexfs_vector_packing packing;
    __u32 optimization_flags;       /* Optimization preferences */
    
    /* Performance hints */
    __u32 access_pattern;           /* Expected access pattern */
    __u32 locality_hint;            /* Data locality preference */
    __u32 numa_node;                /* Preferred NUMA node */
    
    /* Size calculations */
    size_t total_size;              /* Total size needed */
    size_t aligned_size;            /* SIMD-aligned size */
    __u32 blocks_needed;            /* Number of blocks required */
};

/*
 * Vector Block Allocation Result
 * 
 * This structure contains the result of a vector block allocation,
 * including the allocated blocks and layout information.
 */
struct vexfs_vector_alloc_result {
    /* Allocated blocks */
    __u64 *block_numbers;           /* Array of allocated block numbers */
    __u32 block_count;              /* Number of blocks allocated */
    
    /* Layout information */
    __u32 vectors_per_block;        /* Vectors that fit per block */
    __u32 vector_stride;            /* Bytes between vectors */
    __u32 alignment_offset;         /* Offset for SIMD alignment */
    
    /* Efficiency metrics */
    __u32 packing_efficiency;       /* Percentage of space utilized */
    __u32 alignment_waste;          /* Bytes wasted for alignment */
    __u32 fragmentation_level;      /* Block fragmentation level */
    
    /* Performance characteristics */
    __u32 estimated_bandwidth;      /* Estimated memory bandwidth */
    __u32 cache_efficiency;         /* Cache utilization efficiency */
    __u32 simd_efficiency;          /* SIMD operation efficiency */
};

/*
 * Vector Block Layout Manager
 * 
 * This structure manages vector block allocation and layout optimization
 * for a VexFS filesystem instance.
 */
struct vexfs_vector_layout_manager {
    struct super_block *sb;         /* Associated superblock */
    spinlock_t lock;                /* Manager lock */
    
    /* Allocation statistics */
    atomic64_t blocks_allocated;    /* Total blocks allocated */
    atomic64_t vectors_stored;      /* Total vectors stored */
    atomic64_t bytes_allocated;     /* Total bytes allocated */
    atomic64_t alignment_waste;     /* Total alignment waste */
    
    /* Efficiency tracking */
    __u32 avg_packing_efficiency;   /* Average packing efficiency */
    __u32 avg_alignment_waste;      /* Average alignment waste */
    __u32 fragmentation_level;      /* Current fragmentation level */
    
    /* Optimization parameters */
    __u32 preferred_block_size;     /* Preferred allocation unit */
    __u32 alignment_threshold;      /* Alignment optimization threshold */
    __u32 packing_threshold;        /* Packing optimization threshold */
    
    /* NUMA and cache optimization */
    __u32 numa_node_count;          /* Number of NUMA nodes */
    __u32 cache_line_size;          /* CPU cache line size */
    __u32 simd_vector_width;        /* SIMD vector width */
    
    /* Block allocation pools */
    struct list_head free_blocks;   /* Free block list */
    struct list_head aligned_blocks; /* SIMD-aligned blocks */
    struct list_head contiguous_blocks; /* Contiguous block ranges */
    
    /* Performance counters */
    atomic64_t allocation_requests; /* Total allocation requests */
    atomic64_t alignment_hits;      /* Successful alignment optimizations */
    atomic64_t packing_optimizations; /* Successful packing optimizations */
    atomic64_t contiguous_allocations; /* Contiguous allocations */
};

/* Function prototypes */

/* Layout manager initialization and cleanup */
struct vexfs_vector_layout_manager *vexfs_vector_layout_init(struct super_block *sb);
void vexfs_vector_layout_destroy(struct vexfs_vector_layout_manager *manager);

/* Vector block allocation */
int vexfs_allocate_vector_blocks(struct vexfs_vector_layout_manager *manager,
                                struct vexfs_vector_alloc_request *request,
                                struct vexfs_vector_alloc_result *result);

/* Vector block deallocation */
int vexfs_deallocate_vector_blocks(struct vexfs_vector_layout_manager *manager,
                                  __u64 *block_numbers, __u32 block_count);

/* Layout optimization */
int vexfs_optimize_vector_layout(struct vexfs_vector_layout_manager *manager,
                                struct vexfs_vector_metadata *meta,
                                struct vexfs_vector_alloc_request *request);

/* SIMD alignment helpers */
size_t vexfs_calculate_simd_aligned_size(size_t size, __u8 alignment);
__u32 vexfs_calculate_alignment_offset(__u64 block_addr, __u8 alignment);
bool vexfs_is_simd_aligned(__u64 addr, __u8 alignment);

/* Vector packing algorithms */
int vexfs_pack_vectors_tight(struct vexfs_vector_metadata *meta,
                            void *block_data, __u32 block_size,
                            __u32 *vectors_packed);

int vexfs_pack_vectors_aligned(struct vexfs_vector_metadata *meta,
                              void *block_data, __u32 block_size,
                              __u32 *vectors_packed);

int vexfs_pack_vectors_quantized(struct vexfs_vector_metadata *meta,
                                void *block_data, __u32 block_size,
                                __u32 *vectors_packed);

/* Block layout analysis */
int vexfs_analyze_block_efficiency(struct vexfs_vector_block_header *header,
                                  __u32 *packing_efficiency,
                                  __u32 *alignment_waste,
                                  __u32 *fragmentation);

/* Performance optimization */
int vexfs_optimize_for_access_pattern(struct vexfs_vector_layout_manager *manager,
                                     __u32 access_pattern,
                                     struct vexfs_vector_alloc_request *request);

/* Statistics and monitoring */
int vexfs_get_layout_statistics(struct vexfs_vector_layout_manager *manager,
                               struct vexfs_vector_layout_stats *stats);

/* Block header operations */
int vexfs_init_vector_block_header(struct vexfs_vector_block_header *header,
                                  struct vexfs_vector_metadata *meta,
                                  __u32 vector_count);

int vexfs_validate_vector_block_header(struct vexfs_vector_block_header *header);

/* Utility functions */
static inline __u32 vexfs_vectors_per_block(__u16 dimension, __u8 element_type, __u8 alignment)
{
    size_t vector_size = dimension * vexfs_vector_element_size(element_type);
    size_t aligned_size = vexfs_calculate_simd_aligned_size(vector_size, alignment);
    size_t usable_space = VEXFS_BLOCK_SIZE - VEXFS_VECTOR_BLOCK_HEADER_SIZE;
    
    return (__u32)(usable_space / aligned_size);
}

static inline size_t vexfs_calculate_vector_block_waste(__u16 dimension, __u8 element_type, __u8 alignment)
{
    size_t vector_size = dimension * vexfs_vector_element_size(element_type);
    size_t aligned_size = vexfs_calculate_simd_aligned_size(vector_size, alignment);
    
    return aligned_size - vector_size;
}

static inline bool vexfs_should_use_contiguous_allocation(__u32 vector_count, size_t vector_size)
{
    /* Use contiguous allocation for large vectors or many vectors */
    return (vector_count > 100) || (vector_size > 1024);
}

#endif /* _VEXFS_VECTOR_BLOCK_LAYOUT_H */