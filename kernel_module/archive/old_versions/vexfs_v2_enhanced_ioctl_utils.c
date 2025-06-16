/*
 * VexFS v2.0 Enhanced ioctl Utility Functions
 * 
 * This file contains security validation, error handling, and utility
 * functions for the enhanced ioctl interface.
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/fs.h>
#include <linux/uaccess.h>
#include <linux/capability.h>
#include <linux/security.h>
#include <linux/time.h>
#include <linux/ktime.h>

#include "vexfs_v2_enhanced_ioctl.h"

/* 🔥 SECURITY AND VALIDATION FUNCTIONS 🔥 */

/**
 * vexfs_validate_ioctl_request - Validate ioctl request security
 * @file: File pointer
 * @cmd: ioctl command
 * @arg: ioctl argument
 * 
 * Performs comprehensive security validation for ioctl requests
 * including capability checks and parameter validation.
 */
int vexfs_validate_ioctl_request(struct file *file, unsigned int cmd, unsigned long arg)
{
    /* Check file permissions */
    if (!file || !file->f_inode) {
        pr_err("VexFS: Invalid file in ioctl request\n");
        return -EINVAL;
    }
    
    /* Check if file is opened for writing for modification operations */
    switch (cmd) {
    case VEXFS_IOC_CREATE_VECTOR:
    case VEXFS_IOC_DELETE_VECTOR:
    case VEXFS_IOC_UPDATE_VECTOR:
    case VEXFS_IOC_BUILD_INDEX:
    case VEXFS_IOC_REBUILD_INDEX:
    case VEXFS_IOC_DROP_INDEX:
    case VEXFS_IOC_BATCH_INSERT_VECTORS:
    case VEXFS_IOC_BATCH_OPERATIONS:
        if (!(file->f_mode & FMODE_WRITE)) {
            pr_warn("VexFS: Write operation on read-only file\n");
            return -EACCES;
        }
        break;
    }
    
    /* Check capabilities for administrative operations */
    switch (cmd) {
    case VEXFS_IOC_RESET_STATS:
    case VEXFS_IOC_SET_CONFIG:
    case VEXFS_IOC_FLUSH_CACHES:
        if (!capable(CAP_SYS_ADMIN)) {
            pr_warn("VexFS: Administrative operation requires CAP_SYS_ADMIN\n");
            return -EPERM;
        }
        break;
    }
    
    /* Validate argument pointer for commands that require it */
    if (arg == 0 && _IOC_SIZE(cmd) > 0) {
        pr_warn("VexFS: NULL argument for ioctl command requiring data\n");
        return -EINVAL;
    }
    
    /* Check argument accessibility for user space pointers */
    if (arg != 0 && _IOC_SIZE(cmd) > 0) {
        if (_IOC_DIR(cmd) & _IOC_READ) {
            if (!access_ok((void __user *)arg, _IOC_SIZE(cmd))) {
                pr_warn("VexFS: Invalid user space pointer for read operation\n");
                return -EFAULT;
            }
        }
        if (_IOC_DIR(cmd) & _IOC_WRITE) {
            if (!access_ok((void __user *)arg, _IOC_SIZE(cmd))) {
                pr_warn("VexFS: Invalid user space pointer for write operation\n");
                return -EFAULT;
            }
        }
    }
    
    return 0;
}

/**
 * vexfs_check_vector_permissions - Check vector operation permissions
 * @file: File pointer
 * @operation: Operation type
 */
int vexfs_check_vector_permissions(struct file *file, __u32 operation)
{
    /* TODO: Implement fine-grained permission checking */
    /* This could include:
     * - User-specific vector access controls
     * - Namespace-based permissions
     * - Rate limiting per user
     * - Resource quota enforcement
     */
    
    return 0; /* Allow all operations for now */
}

/**
 * vexfs_validate_vector_data - Validate vector data integrity
 * @data: Vector data array
 * @dimensions: Number of dimensions
 * @element_type: Element type
 */
int vexfs_validate_vector_data(const __u32 *data, __u32 dimensions, __u32 element_type)
{
    u32 i;
    
    if (!data) {
        pr_err("VexFS: NULL vector data pointer\n");
        return -EINVAL;
    }
    
    if (!vexfs_is_valid_dimension(dimensions)) {
        pr_warn("VexFS: Invalid vector dimensions: %u\n", dimensions);
        return -EINVAL;
    }
    
    if (!vexfs_is_valid_element_type(element_type)) {
        pr_warn("VexFS: Invalid element type: %u\n", element_type);
        return -EINVAL;
    }
    
    /* Validate IEEE 754 floating point values */
    for (i = 0; i < dimensions; i++) {
        u32 value = data[i];
        
        /* Check for invalid IEEE 754 values */
        u32 exponent = (value >> 23) & 0xFF;
        u32 mantissa = value & 0x7FFFFF;
        
        /* Check for NaN (exponent = 255, mantissa != 0) */
        if (exponent == 0xFF && mantissa != 0) {
            pr_warn("VexFS: NaN value detected at dimension %u\n", i);
            return -EINVAL;
        }
        
        /* Check for infinity (exponent = 255, mantissa = 0) */
        if (exponent == 0xFF && mantissa == 0) {
            pr_warn("VexFS: Infinity value detected at dimension %u\n", i);
            return -EINVAL;
        }
    }
    
    return 0;
}

/**
 * vexfs_validate_search_params - Validate search request parameters
 * @req: Search request structure
 */
int vexfs_validate_search_params(const struct vexfs_enhanced_search_request *req)
{
    if (!req) {
        pr_err("VexFS: NULL search request\n");
        return -EINVAL;
    }
    
    /* Validate dimensions */
    if (!vexfs_is_valid_dimension(req->dimensions)) {
        pr_warn("VexFS: Invalid search dimensions: %u\n", req->dimensions);
        return -EINVAL;
    }
    
    /* Validate k parameter */
    if (req->k == 0 || req->k > VEXFS_MAX_SEARCH_RESULTS) {
        pr_warn("VexFS: Invalid k parameter: %u (max: %u)\n", req->k, VEXFS_MAX_SEARCH_RESULTS);
        return -EINVAL;
    }
    
    /* Validate search algorithm */
    switch (req->search_algorithm) {
    case 0: /* Auto-select */
    case 1: /* Exact search */
    case 2: /* HNSW */
    case 3: /* IVF */
    case 4: /* LSH */
        break;
    default:
        pr_warn("VexFS: Invalid search algorithm: %u\n", req->search_algorithm);
        return -EINVAL;
    }
    
    /* Validate distance metric */
    switch (req->distance_metric) {
    case VEXFS_SEARCH_EUCLIDEAN:
    case VEXFS_SEARCH_COSINE:
    case VEXFS_SEARCH_DOT_PRODUCT:
        break;
    default:
        pr_warn("VexFS: Invalid distance metric: %u\n", req->distance_metric);
        return -EINVAL;
    }
    
    /* Validate filter parameters */
    if (req->filter_count > 0) {
        if (req->filter_count > VEXFS_MAX_SEARCH_RESULTS) {
            pr_warn("VexFS: Too many filter IDs: %u (max: %u)\n", 
                    req->filter_count, VEXFS_MAX_SEARCH_RESULTS);
            return -EINVAL;
        }
        
        if (!req->filter_ids) {
            pr_warn("VexFS: Filter count specified but no filter IDs provided\n");
            return -EINVAL;
        }
        
        if (req->filter_mode > 1) {
            pr_warn("VexFS: Invalid filter mode: %u\n", req->filter_mode);
            return -EINVAL;
        }
    }
    
    /* Validate required output pointers */
    if (!req->result_distances || !req->result_ids) {
        pr_warn("VexFS: Missing required result pointers\n");
        return -EINVAL;
    }
    
    return 0;
}

/**
 * vexfs_validate_index_params - Validate index build parameters
 * @req: Index build request structure
 */
int vexfs_validate_index_params(const struct vexfs_build_index_request *req)
{
    if (!req) {
        pr_err("VexFS: NULL index build request\n");
        return -EINVAL;
    }
    
    /* Validate index type */
    if (!vexfs_is_valid_index_type(req->index_type)) {
        pr_warn("VexFS: Invalid index type: %u\n", req->index_type);
        return -EINVAL;
    }
    
    /* Validate dimensions */
    if (!vexfs_is_valid_dimension(req->dimensions)) {
        pr_warn("VexFS: Invalid index dimensions: %u\n", req->dimensions);
        return -EINVAL;
    }
    
    /* Validate vector count */
    if (req->vector_count == 0 || req->vector_count > 10000000) { /* 10M limit */
        pr_warn("VexFS: Invalid vector count for index: %u\n", req->vector_count);
        return -EINVAL;
    }
    
    /* Validate type-specific parameters */
    switch (req->index_type) {
    case VEXFS_INDEX_HNSW:
        if (req->hnsw_m == 0 || req->hnsw_m > 64) {
            pr_warn("VexFS: Invalid HNSW M parameter: %u\n", req->hnsw_m);
            return -EINVAL;
        }
        if (req->hnsw_ef_construction == 0 || req->hnsw_ef_construction > 1000) {
            pr_warn("VexFS: Invalid HNSW ef_construction: %u\n", req->hnsw_ef_construction);
            return -EINVAL;
        }
        break;
        
    case VEXFS_INDEX_IVF:
        if (req->ivf_clusters == 0 || req->ivf_clusters > req->vector_count) {
            pr_warn("VexFS: Invalid IVF clusters: %u\n", req->ivf_clusters);
            return -EINVAL;
        }
        break;
        
    case VEXFS_INDEX_PQ:
        if (req->pq_subvectors == 0 || req->pq_subvectors > req->dimensions) {
            pr_warn("VexFS: Invalid PQ subvectors: %u\n", req->pq_subvectors);
            return -EINVAL;
        }
        if (req->pq_bits_per_code == 0 || req->pq_bits_per_code > 16) {
            pr_warn("VexFS: Invalid PQ bits per code: %u\n", req->pq_bits_per_code);
            return -EINVAL;
        }
        break;
        
    case VEXFS_INDEX_LSH:
        if (req->lsh_hash_functions == 0 || req->lsh_hash_functions > 64) {
            pr_warn("VexFS: Invalid LSH hash functions: %u\n", req->lsh_hash_functions);
            return -EINVAL;
        }
        if (req->lsh_hash_tables == 0 || req->lsh_hash_tables > 32) {
            pr_warn("VexFS: Invalid LSH hash tables: %u\n", req->lsh_hash_tables);
            return -EINVAL;
        }
        break;
    }
    
    /* Validate memory limit */
    if (req->memory_limit_mb > 0 && req->memory_limit_mb < 100) {
        pr_warn("VexFS: Memory limit too low: %u MB (minimum: 100 MB)\n", req->memory_limit_mb);
        return -EINVAL;
    }
    
    return 0;
}

/**
 * vexfs_validate_batch_params - Validate batch operation parameters
 * @req: Batch operation request structure
 */
int vexfs_validate_batch_params(const struct vexfs_batch_operations_request *req)
{
    if (!req) {
        pr_err("VexFS: NULL batch request\n");
        return -EINVAL;
    }
    
    /* Validate operation type */
    switch (req->operation_type) {
    case VEXFS_BATCH_INSERT:
    case VEXFS_BATCH_UPDATE:
    case VEXFS_BATCH_DELETE:
    case VEXFS_BATCH_SEARCH:
        break;
    default:
        pr_warn("VexFS: Invalid batch operation type: %u\n", req->operation_type);
        return -EINVAL;
    }
    
    /* Validate vector count */
    if (req->vector_count == 0 || req->vector_count > VEXFS_MAX_BATCH_SIZE) {
        pr_warn("VexFS: Invalid batch vector count: %u (max: %u)\n", 
                req->vector_count, VEXFS_MAX_BATCH_SIZE);
        return -EINVAL;
    }
    
    /* Validate dimensions */
    if (!vexfs_is_valid_dimension(req->dimensions)) {
        pr_warn("VexFS: Invalid batch dimensions: %u\n", req->dimensions);
        return -EINVAL;
    }
    
    /* Validate batch size */
    if (req->batch_size > req->vector_count) {
        pr_warn("VexFS: Batch size larger than vector count: %u > %u\n", 
                req->batch_size, req->vector_count);
        return -EINVAL;
    }
    
    /* Validate operation-specific parameters */
    switch (req->operation_type) {
    case VEXFS_BATCH_INSERT:
    case VEXFS_BATCH_UPDATE:
        if (!req->vectors_data) {
            pr_warn("VexFS: Missing vector data for batch insert/update\n");
            return -EINVAL;
        }
        break;
        
    case VEXFS_BATCH_DELETE:
        if (!req->vector_ids) {
            pr_warn("VexFS: Missing vector IDs for batch delete\n");
            return -EINVAL;
        }
        break;
        
    case VEXFS_BATCH_SEARCH:
        if (!req->vectors_data) {
            pr_warn("VexFS: Missing query vectors for batch search\n");
            return -EINVAL;
        }
        if (req->k_per_query == 0 || req->k_per_query > VEXFS_MAX_SEARCH_RESULTS) {
            pr_warn("VexFS: Invalid k per query: %u\n", req->k_per_query);
            return -EINVAL;
        }
        if (!req->search_results || !req->search_result_ids) {
            pr_warn("VexFS: Missing search result buffers\n");
            return -EINVAL;
        }
        break;
    }
    
    return 0;
}

/* 🔥 ERROR HANDLING AND LOGGING 🔥 */

/**
 * vexfs_log_ioctl_error - Log ioctl operation errors
 * @file: File pointer
 * @cmd: ioctl command
 * @error: Error code
 * @operation: Operation description
 */
void vexfs_log_ioctl_error(struct file *file, unsigned int cmd, int error, const char *operation)
{
    const char *cmd_name = "unknown";
    
    /* Map command to human-readable name */
    switch (cmd) {
    case VEXFS_IOC_CREATE_VECTOR:
        cmd_name = "CREATE_VECTOR";
        break;
    case VEXFS_IOC_DELETE_VECTOR:
        cmd_name = "DELETE_VECTOR";
        break;
    case VEXFS_IOC_SIMILARITY_SEARCH:
        cmd_name = "SIMILARITY_SEARCH";
        break;
    case VEXFS_IOC_BUILD_INDEX:
        cmd_name = "BUILD_INDEX";
        break;
    case VEXFS_IOC_BATCH_OPERATIONS:
        cmd_name = "BATCH_OPERATIONS";
        break;
    case VEXFS_IOC_GET_VECTOR_STATS:
        cmd_name = "GET_VECTOR_STATS";
        break;
    }
    
    pr_err("VexFS: ioctl %s (%s) failed with error %d (file: %px)\n",
           cmd_name, operation ? operation : "unknown", error, file);
}

/**
 * vexfs_log_ioctl_performance - Log ioctl operation performance
 * @file: File pointer
 * @cmd: ioctl command
 * @duration_ns: Operation duration in nanoseconds
 */
void vexfs_log_ioctl_performance(struct file *file, unsigned int cmd, __u64 duration_ns)
{
    /* Only log slow operations (> 1ms) to avoid spam */
    if (duration_ns > 1000000) {
        const char *cmd_name = "unknown";
        
        switch (cmd) {
        case VEXFS_IOC_CREATE_VECTOR:
            cmd_name = "CREATE_VECTOR";
            break;
        case VEXFS_IOC_SIMILARITY_SEARCH:
            cmd_name = "SIMILARITY_SEARCH";
            break;
        case VEXFS_IOC_BUILD_INDEX:
            cmd_name = "BUILD_INDEX";
            break;
        case VEXFS_IOC_BATCH_OPERATIONS:
            cmd_name = "BATCH_OPERATIONS";
            break;
        }
        
        pr_info("VexFS: ioctl %s completed in %llu ns (%llu ms)\n",
                cmd_name, duration_ns, duration_ns / 1000000);
    }
}

/* 🔥 UTILITY FUNCTIONS 🔥 */

/**
 * vexfs_is_valid_vector_id - Check if vector ID is valid
 * @vector_id: Vector ID to validate
 */
bool vexfs_is_valid_vector_id(__u64 vector_id)
{
    /* Vector ID 0 is reserved for auto-assignment */
    /* Vector IDs should be reasonable (not too large) */
    return vector_id > 0 && vector_id < 0xFFFFFFFFFFFFFFFFULL;
}

/**
 * vexfs_is_valid_dimension - Check if dimension count is valid
 * @dimensions: Dimension count to validate
 */
bool vexfs_is_valid_dimension(__u32 dimensions)
{
    return dimensions > 0 && dimensions <= VEXFS_MAX_VECTOR_DIMENSION;
}

/**
 * vexfs_is_valid_element_type - Check if element type is valid
 * @element_type: Element type to validate
 */
bool vexfs_is_valid_element_type(__u32 element_type)
{
    switch (element_type) {
    case VEXFS_VECTOR_FLOAT32:
    case VEXFS_VECTOR_FLOAT16:
    case VEXFS_VECTOR_INT8:
    case VEXFS_VECTOR_BINARY:
        return true;
    default:
        return false;
    }
}

/**
 * vexfs_is_valid_index_type - Check if index type is valid
 * @index_type: Index type to validate
 */
bool vexfs_is_valid_index_type(__u32 index_type)
{
    switch (index_type) {
    case VEXFS_INDEX_HNSW:
    case VEXFS_INDEX_IVF:
    case VEXFS_INDEX_PQ:
    case VEXFS_INDEX_LSH:
    case VEXFS_INDEX_FLAT:
        return true;
    default:
        return false;
    }
}

/**
 * vexfs_calculate_vector_size - Calculate vector storage size
 * @dimensions: Number of dimensions
 * @element_type: Element type
 */
__u32 vexfs_calculate_vector_size(__u32 dimensions, __u32 element_type)
{
    if (!vexfs_is_valid_dimension(dimensions) || !vexfs_is_valid_element_type(element_type)) {
        return 0;
    }
    
    switch (element_type) {
    case VEXFS_VECTOR_FLOAT32:
        return dimensions * sizeof(__u32);
    case VEXFS_VECTOR_FLOAT16:
        return dimensions * sizeof(__u16);
    case VEXFS_VECTOR_INT8:
        return dimensions * sizeof(__u8);
    case VEXFS_VECTOR_BINARY:
        return (dimensions + 7) / 8; /* Bits to bytes */
    default:
        return 0;
    }
}

/**
 * vexfs_estimate_index_size - Estimate index storage size
 * @vector_count: Number of vectors
 * @dimensions: Vector dimensions
 * @index_type: Index type
 */
__u32 vexfs_estimate_index_size(__u32 vector_count, __u32 dimensions, __u32 index_type)
{
    __u32 base_size = vector_count * dimensions * sizeof(__u32);
    
    switch (index_type) {
    case VEXFS_INDEX_HNSW:
        /* HNSW typically uses 1.5-2x the base vector size */
        return base_size * 2;
        
    case VEXFS_INDEX_IVF:
        /* IVF uses cluster centroids + inverted lists */
        return base_size / 4 + vector_count * sizeof(__u32);
        
    case VEXFS_INDEX_PQ:
        /* PQ uses codebooks + quantized vectors */
        return dimensions * 256 * sizeof(__u32) + vector_count;
        
    case VEXFS_INDEX_LSH:
        /* LSH uses hash tables */
        return vector_count * 32 * sizeof(__u32); /* 32 hash functions */
        
    case VEXFS_INDEX_FLAT:
        /* Flat index is just the vectors */
        return base_size;
        
    default:
        return base_size;
    }
}