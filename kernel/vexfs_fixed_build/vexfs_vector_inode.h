/*
 * VexFS Vector-Enhanced Inode Structure
 *
 * This header defines the vector-enhanced inode structure that extends
 * the basic VexFS inode with vector-specific metadata for dimensions,
 * element types, and other vector-specific properties.
 */

#ifndef VEXFS_VECTOR_INODE_H
#define VEXFS_VECTOR_INODE_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/spinlock.h>
#include <linux/string.h>
#include <linux/time.h>

/* Vector element type enumeration */
typedef enum {
    VEXFS_VECTOR_FLOAT32 = 0,    /* 32-bit floating point */
    VEXFS_VECTOR_BFLOAT16 = 1,   /* 16-bit brain floating point */
    VEXFS_VECTOR_INT8 = 2,       /* 8-bit signed integer */
    VEXFS_VECTOR_UINT8 = 3,      /* 8-bit unsigned integer */
    VEXFS_VECTOR_INT16 = 4,      /* 16-bit signed integer */
    VEXFS_VECTOR_UINT16 = 5,     /* 16-bit unsigned integer */
    VEXFS_VECTOR_INT32 = 6,      /* 32-bit signed integer */
    VEXFS_VECTOR_UINT32 = 7,     /* 32-bit unsigned integer */
    VEXFS_VECTOR_FLOAT16 = 8,    /* 16-bit floating point */
    VEXFS_VECTOR_FLOAT64 = 9,    /* 64-bit floating point */
    VEXFS_VECTOR_BINARY = 10,    /* Binary vector */
    VEXFS_VECTOR_SPARSE = 11,    /* Sparse vector representation */
    VEXFS_VECTOR_UNKNOWN = 255   /* Unknown/invalid type */
} vexfs_vector_element_type_t;

/* Vector property flags */
#define VEXFS_VECTOR_FLAG_NORMALIZED    (1 << 0)  /* Vector is normalized */
#define VEXFS_VECTOR_FLAG_INDEXED       (1 << 1)  /* Vector has ANN index */
#define VEXFS_VECTOR_FLAG_QUANTIZED     (1 << 2)  /* Vector is quantized */
#define VEXFS_VECTOR_FLAG_COMPRESSED    (1 << 3)  /* Vector is compressed */
#define VEXFS_VECTOR_FLAG_IMMUTABLE     (1 << 4)  /* Vector is read-only */
#define VEXFS_VECTOR_FLAG_CACHED        (1 << 5)  /* Vector is in cache */
#define VEXFS_VECTOR_FLAG_DIRTY         (1 << 6)  /* Vector needs sync */
#define VEXFS_VECTOR_FLAG_SPARSE        (1 << 7)  /* Vector is sparse */

/* SIMD alignment requirements */
#define VEXFS_SIMD_ALIGN_16    16   /* 16-byte alignment (SSE) */
#define VEXFS_SIMD_ALIGN_32    32   /* 32-byte alignment (AVX) */
#define VEXFS_SIMD_ALIGN_64    64   /* 64-byte alignment (AVX-512) */

/* Maximum vector dimensions supported */
#define VEXFS_MAX_VECTOR_DIMENSIONS    65535

/* Vector metadata structure stored in inode */
struct vexfs_vector_metadata {
    __u16 vector_dimension;          /* Number of elements in vector */
    __u8  element_type;              /* Element type (vexfs_vector_element_type_t) */
    __u8  simd_alignment;            /* Required SIMD alignment (16/32/64) */
    __u32 vexfs_flags;               /* Vector property flags */
    __u64 index_metadata;            /* Pointer to ANN index structures */
    __u32 compression_ratio;         /* Compression ratio (if compressed) */
    __u32 original_size;             /* Original uncompressed size */
    __u64 checksum;                  /* Vector data checksum for integrity */
    __u32 access_count;              /* Number of times accessed */
    __u64 last_access_time;          /* Last access timestamp */
    __u32 reserved[4];               /* Reserved for future use */
} __packed;

/* Enhanced VexFS inode info with vector support */
struct vexfs_vector_inode_info {
    struct inode vfs_inode;          /* Standard VFS inode */
    __u32 i_block[15];               /* Block pointers */
    __u32 i_flags;                   /* Standard inode flags */
    struct timespec64 i_crtime;      /* Creation time */
    
    /* Vector-specific fields */
    struct vexfs_vector_metadata vector_meta;  /* Vector metadata */
    __u8  is_vector_file;            /* Flag indicating if this is a vector file */
    __u8  vector_version;            /* Vector format version */
    __u16 vector_reserved;           /* Reserved for alignment */
    
    /* Performance optimization fields */
    void *cached_vector_data;        /* Cached vector data pointer */
    size_t cached_data_size;         /* Size of cached data */
    spinlock_t vector_lock;          /* Lock for vector operations */
};

/* Macros for accessing vector inode */
#define VEXFS_VECTOR_I(inode) container_of(inode, struct vexfs_vector_inode_info, vfs_inode)
#define IS_VECTOR_FILE(inode) (VEXFS_VECTOR_I(inode)->is_vector_file)

/* Vector inode initialization and management functions */
static inline void vexfs_init_vector_metadata(struct vexfs_vector_metadata *meta)
{
    memset(meta, 0, sizeof(*meta));
    meta->element_type = VEXFS_VECTOR_UNKNOWN;
    meta->simd_alignment = VEXFS_SIMD_ALIGN_16;  /* Default to SSE alignment */
}

static inline bool vexfs_is_vector_normalized(const struct vexfs_vector_metadata *meta)
{
    return (meta->vexfs_flags & VEXFS_VECTOR_FLAG_NORMALIZED) != 0;
}

static inline bool vexfs_is_vector_indexed(const struct vexfs_vector_metadata *meta)
{
    return (meta->vexfs_flags & VEXFS_VECTOR_FLAG_INDEXED) != 0;
}

static inline bool vexfs_is_vector_compressed(const struct vexfs_vector_metadata *meta)
{
    return (meta->vexfs_flags & VEXFS_VECTOR_FLAG_COMPRESSED) != 0;
}

static inline size_t vexfs_vector_element_size(vexfs_vector_element_type_t type)
{
    switch (type) {
        case VEXFS_VECTOR_INT8:
        case VEXFS_VECTOR_UINT8:
            return 1;
        case VEXFS_VECTOR_BFLOAT16:
        case VEXFS_VECTOR_FLOAT16:
        case VEXFS_VECTOR_INT16:
        case VEXFS_VECTOR_UINT16:
            return 2;
        case VEXFS_VECTOR_FLOAT32:
        case VEXFS_VECTOR_INT32:
        case VEXFS_VECTOR_UINT32:
            return 4;
        case VEXFS_VECTOR_FLOAT64:
            return 8;
        case VEXFS_VECTOR_BINARY:
            return 1;  /* Binary vectors use 1 bit per element, but stored as bytes */
        case VEXFS_VECTOR_SPARSE:
            return 0;  /* Sparse vectors have variable size */
        default:
            return 0;
    }
}

static inline size_t vexfs_vector_data_size(const struct vexfs_vector_metadata *meta)
{
    if (meta->element_type == VEXFS_VECTOR_SPARSE) {
        return meta->original_size;  /* For sparse vectors, use original size */
    }
    
    size_t element_size = vexfs_vector_element_size(meta->element_type);
    if (element_size == 0) {
        return 0;
    }
    
    if (meta->element_type == VEXFS_VECTOR_BINARY) {
        /* Binary vectors: 1 bit per element, rounded up to bytes */
        return (meta->vector_dimension + 7) / 8;
    }
    
    return meta->vector_dimension * element_size;
}

/* Vector validation functions */
static inline bool vexfs_validate_vector_metadata(const struct vexfs_vector_metadata *meta)
{
    /* Check dimension bounds */
    if (meta->vector_dimension == 0 || meta->vector_dimension > VEXFS_MAX_VECTOR_DIMENSIONS) {
        return false;
    }
    
    /* Check element type */
    if (meta->element_type > VEXFS_VECTOR_SPARSE && meta->element_type != VEXFS_VECTOR_UNKNOWN) {
        return false;
    }
    
    /* Check SIMD alignment */
    if (meta->simd_alignment != VEXFS_SIMD_ALIGN_16 && 
        meta->simd_alignment != VEXFS_SIMD_ALIGN_32 && 
        meta->simd_alignment != VEXFS_SIMD_ALIGN_64) {
        return false;
    }
    
    return true;
}

/* Vector inode operations */
int vexfs_create_vector_inode(struct inode *dir, struct dentry *dentry, 
                             umode_t mode, const struct vexfs_vector_metadata *meta);
int vexfs_read_vector_data(struct inode *inode, void *buffer, size_t size, loff_t offset);
int vexfs_write_vector_data(struct inode *inode, const void *buffer, size_t size, loff_t offset);
int vexfs_update_vector_metadata(struct inode *inode, const struct vexfs_vector_metadata *meta);
int vexfs_sync_vector_inode(struct inode *inode);

/* Vector cache management */
int vexfs_cache_vector_data(struct inode *inode);
void vexfs_invalidate_vector_cache(struct inode *inode);
void vexfs_free_vector_cache(struct inode *inode);

#endif /* VEXFS_VECTOR_INODE_H */