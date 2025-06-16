/*
 * VexFS v2.0 Public API Header
 * 
 * This header defines the standardized public API for VexFS v2.0 kernel module.
 * All public functions follow consistent naming conventions and parameter patterns.
 * 
 * Naming Convention: vexfs_v2_<module>_<operation>
 * Parameter Order: context, input, config, output, optional
 * Return Values: 0 = success, negative = error code
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_V2_PUBLIC_API_H
#define VEXFS_V2_PUBLIC_API_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include "vexfs_v2_uapi.h"
#include "vexfs_v2_search.h"
#include "vexfs_v2_phase3.h"

/*
 * =============================================================================
 * CORE FILESYSTEM API
 * =============================================================================
 */

/**
 * vexfs_v2_core_ioctl - Main IOCTL handler for core operations
 * @file: File pointer for the VexFS file
 * @cmd: IOCTL command number
 * @arg: IOCTL argument (user pointer)
 * 
 * Handles core VexFS v2.0 IOCTL operations including metadata
 * operations and basic vector file management.
 * 
 * Return: 0 on success, negative error code on failure
 */
long vexfs_v2_core_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/**
 * vexfs_v2_core_euclidean_distance - Calculate Euclidean distance between vectors
 * @a: First vector (IEEE 754 bit representation)
 * @b: Second vector (IEEE 754 bit representation)
 * @dimensions: Number of vector dimensions
 * 
 * Calculates Euclidean distance using integer arithmetic to avoid
 * floating-point operations in kernel space.
 * 
 * Return: Distance value as uint32_t
 */
__u32 vexfs_v2_core_euclidean_distance(const uint32_t *a, const uint32_t *b, __u32 dimensions);

/**
 * vexfs_v2_core_cosine_similarity - Calculate cosine similarity between vectors
 * @a: First vector (IEEE 754 bit representation)
 * @b: Second vector (IEEE 754 bit representation)
 * @dimensions: Number of vector dimensions
 * 
 * Calculates cosine similarity using integer arithmetic to avoid
 * floating-point operations in kernel space.
 * 
 * Return: Similarity value as uint32_t
 */
__u32 vexfs_v2_core_cosine_similarity(const uint32_t *a, const uint32_t *b, __u32 dimensions);

/**
 * vexfs_v2_core_manhattan_distance - Calculate Manhattan distance between vectors
 * @a: First vector (IEEE 754 bit representation)
 * @b: Second vector (IEEE 754 bit representation)
 * @dimensions: Number of vector dimensions
 * 
 * Calculates Manhattan distance using integer arithmetic.
 * 
 * Return: Distance value as uint32_t
 */
__u32 vexfs_v2_core_manhattan_distance(const uint32_t *a, const uint32_t *b, __u32 dimensions);

/**
 * vexfs_v2_core_alloc - Allocate memory for VexFS operations
 * @size: Size of memory to allocate
 * 
 * Allocates memory using appropriate kernel allocation functions
 * with VexFS-specific optimizations.
 * 
 * Return: Pointer to allocated memory, NULL on failure
 */
void *vexfs_v2_core_alloc(size_t size);

/**
 * vexfs_v2_core_free - Free memory allocated by vexfs_v2_core_alloc
 * @ptr: Pointer to memory to free
 * 
 * Frees memory allocated by vexfs_v2_core_alloc.
 */
void vexfs_v2_core_free(void *ptr);

/*
 * =============================================================================
 * SEARCH API
 * =============================================================================
 */

/**
 * vexfs_v2_search_knn - Perform k-nearest neighbor search
 * @file: File pointer for the VexFS file
 * @query: k-NN query parameters
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Performs k-nearest neighbor search using the configured index
 * (brute force, HNSW, or LSH).
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_search_knn(struct file *file, const struct vexfs_knn_query *query,
                       struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_search_range - Perform range search within distance threshold
 * @file: File pointer for the VexFS file
 * @query: Range query parameters
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Finds all vectors within a specified distance threshold.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_search_range(struct file *file, const struct vexfs_range_query *query,
                         struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_search_batch - Perform batch search operations
 * @file: File pointer for the VexFS file
 * @batch: Batch search parameters
 * 
 * Performs multiple search operations in a single call for
 * improved performance.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_search_batch(struct file *file, const struct vexfs_batch_search *batch);

/**
 * vexfs_v2_search_ioctl - IOCTL handler for search operations
 * @file: File pointer for the VexFS file
 * @cmd: IOCTL command number
 * @arg: IOCTL argument (user pointer)
 * 
 * Handles search-related IOCTL operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
long vexfs_v2_search_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/*
 * =============================================================================
 * HNSW INDEX API
 * =============================================================================
 */

/**
 * vexfs_v2_hnsw_init - Initialize HNSW index
 * @dimensions: Vector dimensions
 * @distance_metric: Distance metric to use
 * 
 * Initializes the Hierarchical Navigable Small World index
 * for approximate nearest neighbor search.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_hnsw_init(uint32_t dimensions, uint32_t distance_metric);

/**
 * vexfs_v2_hnsw_insert - Insert vector into HNSW index
 * @vector_id: Unique identifier for the vector
 * @vector: Vector data (IEEE 754 bit representation)
 * 
 * Inserts a vector into the HNSW index structure.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_hnsw_insert(uint64_t vector_id, const uint32_t *vector);

/**
 * vexfs_v2_hnsw_search - Search HNSW index for nearest neighbors
 * @query_vector: Query vector (IEEE 754 bit representation)
 * @k: Number of nearest neighbors to find
 * @dimensions: Vector dimensions
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Searches the HNSW index for k nearest neighbors.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_hnsw_search(const uint32_t *query_vector, uint32_t k, uint32_t dimensions,
                        struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_hnsw_get_stats - Get HNSW index statistics
 * @stats: Output structure for statistics
 * 
 * Retrieves performance and usage statistics for the HNSW index.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_hnsw_get_stats(struct vexfs_hnsw_stats *stats);

/**
 * vexfs_v2_hnsw_cleanup - Clean up HNSW index resources
 * 
 * Frees all resources associated with the HNSW index.
 */
void vexfs_v2_hnsw_cleanup(void);

/*
 * =============================================================================
 * LSH INDEX API
 * =============================================================================
 */

/**
 * vexfs_v2_lsh_init - Initialize LSH index
 * @dimensions: Vector dimensions
 * @distance_metric: Distance metric to use
 * @hash_tables: Number of hash tables
 * @hash_functions_per_table: Number of hash functions per table
 * 
 * Initializes the Locality Sensitive Hashing index for
 * approximate nearest neighbor search.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_lsh_init(uint32_t dimensions, uint32_t distance_metric,
                     uint32_t hash_tables, uint32_t hash_functions_per_table);

/**
 * vexfs_v2_lsh_insert - Insert vector into LSH index
 * @vector_id: Unique identifier for the vector
 * @vector: Vector data (IEEE 754 bit representation)
 * 
 * Inserts a vector into the LSH index structure.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_lsh_insert(uint64_t vector_id, const uint32_t *vector);

/**
 * vexfs_v2_lsh_search - Search LSH index for nearest neighbors
 * @query_vector: Query vector (IEEE 754 bit representation)
 * @k: Number of nearest neighbors to find
 * @dimensions: Vector dimensions
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Searches the LSH index for k nearest neighbors.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_lsh_search(const uint32_t *query_vector, uint32_t k, uint32_t dimensions,
                       struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_lsh_get_stats - Get LSH index statistics
 * @stats: Output structure for statistics
 * 
 * Retrieves performance and usage statistics for the LSH index.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_lsh_get_stats(struct vexfs_lsh_stats *stats);

/**
 * vexfs_v2_lsh_cleanup - Clean up LSH index resources
 * 
 * Frees all resources associated with the LSH index.
 */
void vexfs_v2_lsh_cleanup(void);

/*
 * =============================================================================
 * ADVANCED SEARCH API
 * =============================================================================
 */

/**
 * vexfs_v2_advanced_filtered_search - Perform filtered vector search
 * @request: Filtered search request parameters
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Performs vector search with metadata filtering capabilities.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_advanced_filtered_search(const struct vexfs_filtered_search *request,
                                     struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_advanced_multi_vector_search - Perform multi-vector search
 * @request: Multi-vector search request parameters
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Performs search with multiple query vectors simultaneously.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_advanced_multi_vector_search(const struct vexfs_multi_vector_search *request,
                                         struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_advanced_hybrid_search - Perform hybrid vector + keyword search
 * @request: Hybrid search request parameters
 * @results: Output array for search results
 * @result_count: Output parameter for number of results found
 * 
 * Performs combined vector similarity and keyword search.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_advanced_hybrid_search(const struct vexfs_hybrid_search *request,
                                   struct vexfs_search_result *results, uint32_t *result_count);

/**
 * vexfs_v2_advanced_ioctl - IOCTL handler for advanced search operations
 * @file: File pointer for the VexFS file
 * @cmd: IOCTL command number
 * @arg: IOCTL argument (user pointer)
 * 
 * Handles advanced search IOCTL operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
long vexfs_v2_advanced_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/*
 * =============================================================================
 * MULTI-MODEL API
 * =============================================================================
 */

/**
 * vexfs_v2_model_set_metadata - Set embedding model metadata
 * @model_meta: Model metadata structure
 * 
 * Sets metadata for the embedding model used in the filesystem.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_model_set_metadata(const struct vexfs_model_metadata *model_meta);

/**
 * vexfs_v2_model_get_metadata - Get embedding model metadata
 * @model_meta: Output structure for model metadata
 * 
 * Retrieves metadata for the current embedding model.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_model_get_metadata(struct vexfs_model_metadata *model_meta);

/**
 * vexfs_v2_model_validate_compatibility - Validate model compatibility
 * @model_type: Embedding model type
 * @dimensions: Vector dimensions
 * 
 * Validates that the specified model type and dimensions are compatible.
 * 
 * Return: 0 if compatible, negative error code if incompatible
 */
int vexfs_v2_model_validate_compatibility(vexfs_embedding_model_t model_type, uint32_t dimensions);

/**
 * vexfs_v2_model_get_default_dimensions - Get default dimensions for model
 * @model_type: Embedding model type
 * 
 * Returns the default vector dimensions for the specified model type.
 * 
 * Return: Default dimensions, 0 if unknown model type
 */
uint32_t vexfs_v2_model_get_default_dimensions(vexfs_embedding_model_t model_type);

/**
 * vexfs_v2_model_type_to_string - Convert model type to string
 * @model_type: Embedding model type
 * 
 * Converts model type enum to human-readable string.
 * 
 * Return: String representation of model type
 */
const char *vexfs_v2_model_type_to_string(vexfs_embedding_model_t model_type);

/*
 * =============================================================================
 * PHASE 3 INTEGRATION API
 * =============================================================================
 */

/**
 * vexfs_v2_phase3_init - Initialize Phase 3 components
 * 
 * Initializes all Phase 3 advanced features including multi-model
 * support and advanced indexing.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_phase3_init(void);

/**
 * vexfs_v2_phase3_cleanup - Clean up Phase 3 components
 * 
 * Cleans up all Phase 3 resources and components.
 */
void vexfs_v2_phase3_cleanup(void);

/**
 * vexfs_v2_phase3_ioctl - IOCTL handler for Phase 3 operations
 * @file: File pointer for the VexFS file
 * @cmd: IOCTL command number
 * @arg: IOCTL argument (user pointer)
 * 
 * Handles Phase 3 specific IOCTL operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
long vexfs_v2_phase3_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/**
 * vexfs_v2_phase3_get_stats - Get Phase 3 statistics
 * @stats: Output structure for Phase 3 statistics
 * 
 * Retrieves performance and usage statistics for Phase 3 features.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_phase3_get_stats(struct vexfs_phase3_stats *stats);

/*
 * =============================================================================
 * MONITORING API
 * =============================================================================
 */

/**
 * vexfs_v2_monitor_get_stats - Get monitoring statistics
 * @stats: Output structure for search statistics
 * 
 * Retrieves comprehensive performance and usage statistics.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_v2_monitor_get_stats(struct vexfs_search_stats *stats);

/**
 * vexfs_v2_monitor_record_operation - Record operation for monitoring
 * @latency_ns: Operation latency in nanoseconds
 * @success: Whether the operation was successful
 * 
 * Records an operation for performance monitoring and statistics.
 */
void vexfs_v2_monitor_record_operation(uint64_t latency_ns, bool success);

/**
 * vexfs_v2_monitor_reset_counters - Reset monitoring counters
 * 
 * Resets all monitoring counters and statistics.
 */
void vexfs_v2_monitor_reset_counters(void);

#endif /* VEXFS_V2_PUBLIC_API_H */