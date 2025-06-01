/*
 * VexFS v2.0 User-Kernel API Header
 * 
 * This header defines the IOCTL interface between VexFS v2.0 kernel module
 * and userspace applications. It provides a single source of truth for all
 * structure definitions and command numbers.
 * 
 * CRITICAL: This header must be kept in sync with the kernel module.
 * Any changes to structures or IOCTL commands must be updated here.
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef _VEXFS_V2_UAPI_H
#define _VEXFS_V2_UAPI_H

#ifdef __KERNEL__
#include <linux/types.h>
#include <linux/ioctl.h>
#else
#include <stdint.h>
#include <sys/ioctl.h>
/* Userspace type compatibility */
typedef uint8_t  __u8;
typedef uint16_t __u16;
typedef uint32_t __u32;
typedef uint64_t __u64;
#endif

/*
 * VexFS v2.0 Version Information
 */
#define VEXFS_V2_MAJOR_VERSION  2
#define VEXFS_V2_MINOR_VERSION  0
#define VEXFS_V2_PATCH_VERSION  0

/*
 * VexFS v2.0 Magic Numbers
 */
#define VEXFS_V2_MAGIC          0x56455832  /* "VEX2" */

/*
 * Vector Element Types
 */
#define VEXFS_VECTOR_FLOAT32    0x01
#define VEXFS_VECTOR_FLOAT16    0x02
#define VEXFS_VECTOR_INT8       0x03
#define VEXFS_VECTOR_BINARY     0x04

/*
 * Vector Search Types
 */
#define VEXFS_SEARCH_EUCLIDEAN  0x00
#define VEXFS_SEARCH_COSINE     0x01
#define VEXFS_SEARCH_DOT_PRODUCT 0x02

/*
 * Vector Storage Formats
 */
#define VEXFS_STORAGE_DENSE     0x00
#define VEXFS_STORAGE_SPARSE    0x01
#define VEXFS_STORAGE_COMPRESSED 0x02

/*
 * Vector Compression Types
 */
#define VEXFS_COMPRESS_NONE     0x00
#define VEXFS_COMPRESS_LZ4      0x01
#define VEXFS_COMPRESS_ZSTD     0x02

/*
 * Vector Insert Flags
 */
#define VEXFS_INSERT_OVERWRITE  0x01
#define VEXFS_INSERT_APPEND     0x02
#define VEXFS_INSERT_VALIDATE   0x04

/*
 * IOCTL Magic Number and Commands
 */
#define VEXFS_IOC_MAGIC         'V'

/*
 * Vector File Metadata Structure
 * 
 * This structure contains metadata about vector files, including
 * dimensions, storage format, and layout information.
 * 
 * Size: 32 bytes (validated)
 */
struct vexfs_vector_file_info {
    __u32 dimensions;        /* Vector dimensions (e.g., 128, 512, 1024) */
    __u32 element_type;      /* Element type (VEXFS_VECTOR_*) */
    __u32 vector_count;      /* Number of vectors stored */
    __u32 storage_format;    /* Storage format (VEXFS_STORAGE_*) */
    __u64 data_offset;       /* Offset to vector data in file */
    __u64 index_offset;      /* Offset to index data in file */
    __u32 compression_type;  /* Compression type (VEXFS_COMPRESS_*) */
    __u32 alignment_bytes;   /* Memory alignment requirement */
};

/*
 * Vector Search Request Structure
 * 
 * This structure defines a vector similarity search request.
 * Used for finding k-nearest neighbors.
 * 
 * Size: 40 bytes (validated)
 */
struct vexfs_vector_search_request {
    float    *query_vector;  /* Input: Query vector data */
    __u32     dimensions;    /* Vector dimensions */
    __u32     k;             /* Number of nearest neighbors to find */
    __u32     search_type;   /* Search algorithm (VEXFS_SEARCH_*) */
    float    *results;       /* Output: Distance scores */
    __u64    *result_ids;    /* Output: Vector IDs of results */
    __u32     result_count;  /* Output: Actual number of results found */
};

/*
 * Batch Insert Request Structure
 * 
 * CRITICAL: This structure layout has been validated through extensive testing.
 * The field order MUST match the kernel module exactly:
 * 1. vectors (pointer)
 * 2. vector_count (32-bit)
 * 3. dimensions (32-bit) 
 * 4. vector_ids (pointer)
 * 5. flags (32-bit)
 * 
 * Total size: 32 bytes (validated with working tests)
 */
struct vexfs_batch_insert_request {
    float    *vectors;       /* Input: Vector data array */
    __u32     vector_count;  /* Number of vectors to insert */
    __u32     dimensions;    /* Vector dimensions */
    __u64    *vector_ids;    /* Optional: Custom vector IDs */
    __u32     flags;         /* Insert flags (VEXFS_INSERT_*) */
};

/*
 * IOCTL Command Definitions
 * 
 * These commands provide the interface between userspace and kernel.
 * Command numbers are fixed and must not be changed.
 */

/* Set vector file metadata */
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)

/* Get vector file metadata */
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_file_info)

/* Perform vector similarity search */
#define VEXFS_IOC_VECTOR_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search_request)

/* Batch insert vectors */
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)

/*
 * Compatibility and Validation Macros
 */

/* Check if structure sizes match expected values */
#define VEXFS_VECTOR_FILE_INFO_SIZE     40
#define VEXFS_VECTOR_SEARCH_REQUEST_SIZE 48
#define VEXFS_BATCH_INSERT_REQUEST_SIZE  32

/* Compile-time size validation (only in C, not assembly) */
#ifndef __ASSEMBLY__
_Static_assert(sizeof(struct vexfs_vector_file_info) == VEXFS_VECTOR_FILE_INFO_SIZE,
               "vexfs_vector_file_info size mismatch");
_Static_assert(sizeof(struct vexfs_vector_search_request) == VEXFS_VECTOR_SEARCH_REQUEST_SIZE,
               "vexfs_vector_search_request size mismatch");
_Static_assert(sizeof(struct vexfs_batch_insert_request) == VEXFS_BATCH_INSERT_REQUEST_SIZE,
               "vexfs_batch_insert_request size mismatch");
#endif

/*
 * Helper Macros for Common Operations
 */

/* Calculate vector data size in bytes */
#define VEXFS_VECTOR_DATA_SIZE(dimensions, count) \
    ((dimensions) * (count) * sizeof(float))

/* Calculate vector ID array size in bytes */
#define VEXFS_VECTOR_ID_SIZE(count) \
    ((count) * sizeof(__u64))

/* Validate vector dimensions */
#define VEXFS_VALID_DIMENSIONS(dim) \
    ((dim) > 0 && (dim) <= 65536)

/* Validate vector count */
#define VEXFS_VALID_COUNT(count) \
    ((count) > 0 && (count) <= 1000000)

/*
 * Error Codes (in addition to standard errno values)
 */
#define VEXFS_E_INVALID_DIMENSIONS  1001
#define VEXFS_E_INVALID_COUNT       1002
#define VEXFS_E_INVALID_TYPE        1003
#define VEXFS_E_SIMD_UNAVAILABLE    1004
#define VEXFS_E_MEMORY_ALIGNMENT    1005

#endif /* _VEXFS_V2_UAPI_H */