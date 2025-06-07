/*
 * VexFS v2.0 Internal API Header
 * 
 * This header defines internal functions and utilities used within
 * the VexFS v2.0 kernel module. These functions are not part of the
 * public API and should not be used by external modules.
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_V2_INTERNAL_H
#define VEXFS_V2_INTERNAL_H

#include <linux/types.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>

/*
 * =============================================================================
 * INTERNAL CONSTANTS AND MACROS
 * =============================================================================
 */

/* Internal validation macros */
#define VEXFS_V2_VALIDATE_DIMENSIONS(dim) \
    ((dim) > 0 && (dim) <= 65536)

#define VEXFS_V2_VALIDATE_VECTOR_COUNT(count) \
    ((count) > 0 && (count) <= 1000000)

#define VEXFS_V2_VALIDATE_POINTER(ptr) \
    ((ptr) != NULL)

/* Internal error codes */
#define VEXFS_V2_ERR_INVALID_STATE      -1000
#define VEXFS_V2_ERR_INDEX_CORRUPTED    -1001
#define VEXFS_V2_ERR_MEMORY_EXHAUSTED   -1002
#define VEXFS_V2_ERR_CONCURRENT_ACCESS  -1003

/* Internal buffer sizes */
#define VEXFS_V2_INTERNAL_BUFFER_SIZE   4096
#define VEXFS_V2_MAX_INTERNAL_VECTORS   10000

/*
 * =============================================================================
 * INTERNAL DATA STRUCTURES
 * =============================================================================
 */

/* Internal vector storage structure */
struct vexfs_v2_internal_vector {
    uint64_t vector_id;
    uint32_t dimensions;
    uint32_t *data_bits;        /* IEEE 754 bit representation */
    uint64_t timestamp;
    uint32_t flags;
    struct list_head list;
};

/* Internal index state */
struct vexfs_v2_internal_index_state {
    uint32_t index_type;
    uint32_t vector_count;
    uint32_t dimensions;
    bool is_initialized;
    bool is_building;
    struct mutex state_mutex;
    void *index_data;
};

/* Internal search context */
struct vexfs_v2_internal_search_context {
    struct file *file;
    uint32_t search_type;
    uint32_t distance_metric;
    uint64_t start_time_ns;
    uint32_t vectors_examined;
    bool use_index;
};

/*
 * =============================================================================
 * INTERNAL UTILITY FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_v2_internal_validate_vector - Validate vector data
 * @vector: Vector data to validate
 * @dimensions: Expected dimensions
 * 
 * Validates that vector data is properly formatted and within
 * acceptable ranges.
 * 
 * Return: 0 if valid, negative error code if invalid
 */
static inline int vexfs_v2_internal_validate_vector(const uint32_t *vector, uint32_t dimensions)
{
    if (!VEXFS_V2_VALIDATE_POINTER(vector))
        return -EINVAL;
    
    if (!VEXFS_V2_VALIDATE_DIMENSIONS(dimensions))
        return -EINVAL;
    
    return 0;
}

/**
 * vexfs_v2_internal_calculate_hash - Calculate hash for vector
 * @vector: Vector data (IEEE 754 bit representation)
 * @dimensions: Vector dimensions
 * 
 * Calculates a hash value for the vector for use in hash tables
 * and duplicate detection.
 * 
 * Return: Hash value as uint32_t
 */
static inline uint32_t vexfs_v2_internal_calculate_hash(const uint32_t *vector, uint32_t dimensions)
{
    uint32_t hash = 0;
    uint32_t i;
    
    for (i = 0; i < dimensions; i++) {
        hash ^= vector[i];
        hash = (hash << 1) | (hash >> 31);  /* Rotate left by 1 */
    }
    
    return hash;
}

/**
 * vexfs_v2_internal_copy_vector - Copy vector data safely
 * @dest: Destination buffer
 * @src: Source vector data
 * @dimensions: Vector dimensions
 * 
 * Safely copies vector data with bounds checking.
 * 
 * Return: 0 on success, negative error code on failure
 */
static inline int vexfs_v2_internal_copy_vector(uint32_t *dest, const uint32_t *src, uint32_t dimensions)
{
    if (!VEXFS_V2_VALIDATE_POINTER(dest) || !VEXFS_V2_VALIDATE_POINTER(src))
        return -EINVAL;
    
    if (!VEXFS_V2_VALIDATE_DIMENSIONS(dimensions))
        return -EINVAL;
    
    memcpy(dest, src, dimensions * sizeof(uint32_t));
    return 0;
}

/**
 * vexfs_v2_internal_get_timestamp - Get current timestamp
 * 
 * Returns current timestamp in nanoseconds for internal use.
 * 
 * Return: Timestamp in nanoseconds
 */
static inline uint64_t vexfs_v2_internal_get_timestamp(void)
{
    struct timespec64 ts;
    ktime_get_real_ts64(&ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

/*
 * =============================================================================
 * INTERNAL MEMORY MANAGEMENT
 * =============================================================================
 */

/**
 * vexfs_v2_internal_alloc_vector - Allocate internal vector structure
 * @dimensions: Vector dimensions
 * 
 * Allocates and initializes an internal vector structure.
 * 
 * Return: Pointer to allocated structure, NULL on failure
 */
struct vexfs_v2_internal_vector *vexfs_v2_internal_alloc_vector(uint32_t dimensions);

/**
 * vexfs_v2_internal_free_vector - Free internal vector structure
 * @vector: Vector structure to free
 * 
 * Frees an internal vector structure and its associated data.
 */
void vexfs_v2_internal_free_vector(struct vexfs_v2_internal_vector *vector);

/**
 * vexfs_v2_internal_alloc_aligned - Allocate SIMD-aligned memory
 * @size: Size to allocate
 * @alignment: Required alignment (must be power of 2)
 * 
 * Allocates memory with specified alignment for SIMD operations.
 * 
 * Return: Pointer to aligned memory, NULL on failure
 */
void *vexfs_v2_internal_alloc_aligned(size_t size, size_t alignment);

/**
 * vexfs_v2_internal_free_aligned - Free SIMD-aligned memory
 * @ptr: Pointer to aligned memory
 * 
 * Frees memory allocated by vexfs_v2_internal_alloc_aligned.
 */
void vexfs_v2_internal_free_aligned(void *ptr);

/*
 * =============================================================================
 * INTERNAL INDEX MANAGEMENT
 * =============================================================================
 */

/**
 * vexfs_v2_internal_init_index_state - Initialize index state
 * @state: Index state structure to initialize
 * @index_type: Type of index (HNSW, LSH, etc.)
 * @dimensions: Vector dimensions
 * 
 * Initializes internal index state structure.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_internal_init_index_state(struct vexfs_v2_internal_index_state *state,
                                      uint32_t index_type, uint32_t dimensions);

/**
 * vexfs_v2_internal_cleanup_index_state - Clean up index state
 * @state: Index state structure to clean up
 * 
 * Cleans up internal index state structure and frees resources.
 */
void vexfs_v2_internal_cleanup_index_state(struct vexfs_v2_internal_index_state *state);

/**
 * vexfs_v2_internal_lock_index - Lock index for exclusive access
 * @state: Index state structure
 * 
 * Acquires exclusive lock on index for thread-safe operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_internal_lock_index(struct vexfs_v2_internal_index_state *state);

/**
 * vexfs_v2_internal_unlock_index - Unlock index
 * @state: Index state structure
 * 
 * Releases exclusive lock on index.
 */
void vexfs_v2_internal_unlock_index(struct vexfs_v2_internal_index_state *state);

/*
 * =============================================================================
 * INTERNAL SEARCH UTILITIES
 * =============================================================================
 */

/**
 * vexfs_v2_internal_init_search_context - Initialize search context
 * @context: Search context to initialize
 * @file: File pointer
 * @search_type: Type of search operation
 * 
 * Initializes internal search context for tracking search operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_internal_init_search_context(struct vexfs_v2_internal_search_context *context,
                                         struct file *file, uint32_t search_type);

/**
 * vexfs_v2_internal_finalize_search_context - Finalize search context
 * @context: Search context to finalize
 * 
 * Finalizes search context and records performance metrics.
 */
void vexfs_v2_internal_finalize_search_context(struct vexfs_v2_internal_search_context *context);

/**
 * vexfs_v2_internal_should_use_index - Determine if index should be used
 * @vector_count: Number of vectors in dataset
 * @k: Number of results requested
 * 
 * Determines whether to use index-based search or brute force
 * based on dataset size and query parameters.
 * 
 * Return: true if index should be used, false for brute force
 */
bool vexfs_v2_internal_should_use_index(uint32_t vector_count, uint32_t k);

/*
 * =============================================================================
 * INTERNAL ERROR HANDLING
 * =============================================================================
 */

/**
 * vexfs_v2_internal_log_error - Log internal error with context
 * @function: Function name where error occurred
 * @error_code: Error code
 * @message: Additional error message
 * 
 * Logs internal errors with context information for debugging.
 */
void vexfs_v2_internal_log_error(const char *function, int error_code, const char *message);

/**
 * vexfs_v2_internal_handle_critical_error - Handle critical internal error
 * @error_code: Critical error code
 * @context: Error context information
 * 
 * Handles critical errors that may require module shutdown or recovery.
 * 
 * Return: Recovery action code
 */
int vexfs_v2_internal_handle_critical_error(int error_code, const char *context);

/*
 * =============================================================================
 * INTERNAL DEBUGGING AND VALIDATION
 * =============================================================================
 */

#ifdef VEXFS_V2_DEBUG
/**
 * vexfs_v2_internal_debug_print_vector - Print vector for debugging
 * @vector: Vector to print
 * @dimensions: Vector dimensions
 * @label: Label for the vector
 * 
 * Prints vector data for debugging purposes (only in debug builds).
 */
void vexfs_v2_internal_debug_print_vector(const uint32_t *vector, uint32_t dimensions, const char *label);

/**
 * vexfs_v2_internal_validate_index_integrity - Validate index integrity
 * @state: Index state to validate
 * 
 * Performs comprehensive validation of index data structures
 * (only in debug builds).
 * 
 * Return: 0 if valid, negative error code if corrupted
 */
int vexfs_v2_internal_validate_index_integrity(const struct vexfs_v2_internal_index_state *state);
#else
#define vexfs_v2_internal_debug_print_vector(vector, dimensions, label) do { } while (0)
#define vexfs_v2_internal_validate_index_integrity(state) (0)
#endif

#endif /* VEXFS_V2_INTERNAL_H */