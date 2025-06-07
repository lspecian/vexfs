/*
 * VexFS Vector-Enhanced Inode Structure - Test Version
 * 
 * This is a userspace-compatible version of the vector inode header
 * for testing purposes. It removes kernel dependencies while maintaining
 * the same interface and functionality.
 */

#ifndef _VEXFS_VECTOR_INODE_TEST_H
#define _VEXFS_VECTOR_INODE_TEST_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Userspace type definitions */
typedef uint8_t  __u8;
typedef uint16_t __u16;
typedef uint32_t __u32;
typedef uint64_t __u64;

/* VexFS Vector Element Types */
enum vexfs_vector_element_type {
    VEXFS_VECTOR_UNKNOWN = 0,
    VEXFS_VECTOR_FLOAT32 = 1,
    VEXFS_VECTOR_FLOAT64 = 2,
    VEXFS_VECTOR_FLOAT16 = 3,
    VEXFS_VECTOR_BFLOAT16 = 4,
    VEXFS_VECTOR_INT8 = 5,
    VEXFS_VECTOR_UINT8 = 6,
    VEXFS_VECTOR_INT16 = 7,
    VEXFS_VECTOR_UINT16 = 8,
    VEXFS_VECTOR_INT32 = 9,
    VEXFS_VECTOR_UINT32 = 10,
    VEXFS_VECTOR_BINARY = 11,
    VEXFS_VECTOR_SPARSE = 12,
};

/* SIMD Alignment Options */
enum vexfs_simd_alignment {
    VEXFS_SIMD_ALIGN_16 = 16,   /* SSE alignment */
    VEXFS_SIMD_ALIGN_32 = 32,   /* AVX alignment */
    VEXFS_SIMD_ALIGN_64 = 64,   /* AVX-512 alignment */
};

/* Vector Property Flags */
#define VEXFS_VECTOR_FLAG_NORMALIZED    (1 << 0)  /* Vector is normalized */
#define VEXFS_VECTOR_FLAG_INDEXED       (1 << 1)  /* Vector is indexed */
#define VEXFS_VECTOR_FLAG_COMPRESSED    (1 << 2)  /* Vector is compressed */
#define VEXFS_VECTOR_FLAG_QUANTIZED     (1 << 3)  /* Vector is quantized */
#define VEXFS_VECTOR_FLAG_SPARSE        (1 << 4)  /* Vector is sparse */
#define VEXFS_VECTOR_FLAG_IMMUTABLE     (1 << 5)  /* Vector is read-only */
#define VEXFS_VECTOR_FLAG_CACHED        (1 << 6)  /* Vector is cached */
#define VEXFS_VECTOR_FLAG_DIRTY         (1 << 7)  /* Vector needs sync */

/* Constants */
#define VEXFS_MAX_VECTOR_DIMENSIONS     65535
#define VEXFS_VECTOR_METADATA_VERSION   1

/*
 * VexFS Vector Metadata Structure
 * 
 * This structure contains all vector-specific metadata that extends
 * the standard VexFS inode with vector database capabilities.
 */
struct vexfs_vector_metadata {
    /* Core vector properties */
    __u8  element_type;         /* enum vexfs_vector_element_type */
    __u8  simd_alignment;       /* SIMD alignment requirement */
    __u16 vector_dimension;     /* Number of vector dimensions */
    __u32 vexfs_flags;          /* Vector property flags */
    
    /* Performance and optimization metadata */
    __u32 access_count;         /* Number of times accessed */
    __u64 last_access_time;     /* Last access timestamp */
    __u32 compression_ratio;    /* Compression ratio (0-100) */
    __u32 original_size;        /* Original uncompressed size */
    
    /* Index and clustering metadata */
    __u32 cluster_id;           /* Cluster assignment */
    __u32 index_offset;         /* Offset in index structure */
    
    /* Reserved for future use */
    __u32 reserved[4];
    
    /* Metadata version and checksum */
    __u8  metadata_version;     /* Structure version */
    __u8  padding[3];           /* Alignment padding */
    __u32 checksum;             /* Metadata integrity checksum */
} __attribute__((packed));

/*
 * VexFS Vector-Enhanced Inode Structure
 * 
 * This extends the standard VexFS inode with vector-specific metadata
 * and capabilities for vector database operations.
 */
struct vexfs_vector_inode {
    /* Standard inode fields would go here in kernel version */
    __u64 inode_number;         /* Inode number */
    __u32 file_size;            /* File size in bytes */
    __u32 block_count;          /* Number of allocated blocks */
    
    /* Vector-specific metadata */
    struct vexfs_vector_metadata vector_meta;
    
    /* Vector data location */
    __u64 vector_data_block;    /* Block containing vector data */
    __u32 vector_data_offset;   /* Offset within block */
    __u32 vector_data_size;     /* Size of vector data */
    
    /* Index and performance data */
    __u64 index_block;          /* Block containing index data */
    __u32 cache_hint;           /* Cache optimization hint */
    __u32 prefetch_distance;    /* Prefetch optimization */
};

/* Function prototypes */

/* Metadata initialization and validation */
static inline void vexfs_init_vector_metadata(struct vexfs_vector_metadata *meta)
{
    if (!meta) return;
    
    meta->element_type = VEXFS_VECTOR_UNKNOWN;
    meta->simd_alignment = VEXFS_SIMD_ALIGN_16;
    meta->vector_dimension = 0;
    meta->vexfs_flags = 0;
    meta->access_count = 0;
    meta->last_access_time = 0;
    meta->compression_ratio = 0;
    meta->original_size = 0;
    meta->cluster_id = 0;
    meta->index_offset = 0;
    meta->metadata_version = VEXFS_VECTOR_METADATA_VERSION;
    meta->checksum = 0;
    
    /* Clear reserved fields */
    for (int i = 0; i < 4; i++) {
        meta->reserved[i] = 0;
    }
    for (int i = 0; i < 3; i++) {
        meta->padding[i] = 0;
    }
}

/* Element size calculations */
static inline size_t vexfs_vector_element_size(enum vexfs_vector_element_type type)
{
    switch (type) {
        case VEXFS_VECTOR_INT8:
        case VEXFS_VECTOR_UINT8:
        case VEXFS_VECTOR_BINARY:
            return 1;
        case VEXFS_VECTOR_INT16:
        case VEXFS_VECTOR_UINT16:
        case VEXFS_VECTOR_FLOAT16:
        case VEXFS_VECTOR_BFLOAT16:
            return 2;
        case VEXFS_VECTOR_INT32:
        case VEXFS_VECTOR_UINT32:
        case VEXFS_VECTOR_FLOAT32:
            return 4;
        case VEXFS_VECTOR_FLOAT64:
            return 8;
        case VEXFS_VECTOR_SPARSE:
            return 0;  /* Variable size */
        default:
            return 0;
    }
}

/* Vector data size calculation */
static inline size_t vexfs_vector_data_size(const struct vexfs_vector_metadata *meta)
{
    if (!meta) return 0;
    
    if (meta->element_type == VEXFS_VECTOR_SPARSE) {
        return meta->original_size;
    }
    
    if (meta->element_type == VEXFS_VECTOR_BINARY) {
        /* Binary vectors: pack bits into bytes */
        return (meta->vector_dimension + 7) / 8;
    }
    
    size_t element_size = vexfs_vector_element_size((enum vexfs_vector_element_type)meta->element_type);
    return meta->vector_dimension * element_size;
}

/* Vector property flag helpers */
static inline bool vexfs_is_vector_normalized(const struct vexfs_vector_metadata *meta)
{
    return meta && (meta->vexfs_flags & VEXFS_VECTOR_FLAG_NORMALIZED);
}

static inline bool vexfs_is_vector_indexed(const struct vexfs_vector_metadata *meta)
{
    return meta && (meta->vexfs_flags & VEXFS_VECTOR_FLAG_INDEXED);
}

static inline bool vexfs_is_vector_compressed(const struct vexfs_vector_metadata *meta)
{
    return meta && (meta->vexfs_flags & VEXFS_VECTOR_FLAG_COMPRESSED);
}

static inline bool vexfs_is_vector_sparse(const struct vexfs_vector_metadata *meta)
{
    return meta && (meta->vexfs_flags & VEXFS_VECTOR_FLAG_SPARSE);
}

/* Metadata validation */
static inline bool vexfs_validate_vector_metadata(const struct vexfs_vector_metadata *meta)
{
    if (!meta) return false;
    
    /* Check vector dimension bounds */
    if (meta->vector_dimension == 0 || meta->vector_dimension > VEXFS_MAX_VECTOR_DIMENSIONS) {
        return false;
    }
    
    /* Check element type validity */
    if (meta->element_type < VEXFS_VECTOR_UNKNOWN || meta->element_type > VEXFS_VECTOR_SPARSE) {
        return false;
    }
    
    /* Check SIMD alignment validity */
    if (meta->simd_alignment != VEXFS_SIMD_ALIGN_16 &&
        meta->simd_alignment != VEXFS_SIMD_ALIGN_32 &&
        meta->simd_alignment != VEXFS_SIMD_ALIGN_64) {
        return false;
    }
    
    /* Check compression ratio bounds */
    if (meta->compression_ratio > 100) {
        return false;
    }
    
    return true;
}

/* SIMD alignment helpers */
static inline size_t vexfs_align_to_simd(size_t size, enum vexfs_simd_alignment alignment)
{
    size_t align = (size_t)alignment;
    return (size + align - 1) & ~(align - 1);
}

static inline bool vexfs_is_simd_aligned(const void *ptr, enum vexfs_simd_alignment alignment)
{
    uintptr_t addr = (uintptr_t)ptr;
    return (addr & ((size_t)alignment - 1)) == 0;
}

#endif /* _VEXFS_VECTOR_INODE_TEST_H */