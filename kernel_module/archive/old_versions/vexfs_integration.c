/*
 * VexFS v2.0 Phase 3 - Integration Module
 * 
 * This module integrates all Phase 3 components:
 * - Multi-Model Embedding Support
 * - Advanced Search Operations  
 * - HNSW Index Implementation
 * - LSH Index Implementation
 * 
 * Provides unified IOCTL interface and coordinates between components.
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/fs.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/mutex.h>
#include <linux/atomic.h>

#ifdef __KERNEL__
#include "vexfs.h"
#else
#include "vexfs.h"
#endif

/* Integration state */
struct vexfs_phase3_state {
    bool multi_model_initialized;
    bool advanced_search_initialized;
    bool hnsw_initialized;
    bool lsh_initialized;
    
    /* Current configuration */
    uint32_t dimensions;
    uint32_t distance_metric;
    vexfs_embedding_model_t current_model;
    
    /* Index selection */
    vexfs_index_type_t active_index_type;
    
    /* Statistics */
    atomic64_t total_phase3_operations;
    atomic64_t multi_model_operations;
    atomic64_t advanced_search_operations;
    atomic64_t hnsw_operations;
    atomic64_t lsh_operations;
    
    struct mutex state_mutex;
    uint32_t reserved[8];
};

static struct vexfs_phase3_state global_phase3_state;
static DEFINE_MUTEX(phase3_global_mutex);

/* Forward declarations for external functions */
extern int vexfs_multi_model_init(void);
extern void vexfs_multi_model_cleanup(void);
extern int vexfs_set_model_metadata(struct vexfs_model_metadata *metadata);
extern int vexfs_get_model_metadata(struct vexfs_model_metadata *metadata);

extern int vexfs_advanced_search_init(void);
extern void vexfs_advanced_search_cleanup(void);
extern long vexfs_advanced_search_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

extern int vexfs_hnsw_init(uint32_t dimensions, uint32_t max_connections, uint32_t ef_construction);
extern void vexfs_hnsw_cleanup(void);
extern int vexfs_hnsw_insert(uint64_t vector_id, const uint32_t *vector);
extern int vexfs_hnsw_search(const uint32_t *query_vector, uint32_t k,
                            struct vexfs_search_result *results, uint32_t *result_count);

extern int vexfs_lsh_init(uint32_t dimensions, uint32_t distance_metric, 
                         uint32_t hash_tables, uint32_t hash_functions_per_table);
extern void vexfs_lsh_cleanup(void);
extern int vexfs_lsh_insert(uint64_t vector_id, const uint32_t *vector);
extern int vexfs_lsh_search(const uint32_t *query_vector, uint32_t k,
                           struct vexfs_search_result *results, uint32_t *result_count);

/*
 * Initialize Phase 3 integration
 */
int vexfs_phase3_init(void)
{
    mutex_lock(&phase3_global_mutex);
    
    memset(&global_phase3_state, 0, sizeof(global_phase3_state));
    mutex_init(&global_phase3_state.state_mutex);
    
    /* Initialize statistics */
    atomic64_set(&global_phase3_state.total_phase3_operations, 0);
    atomic64_set(&global_phase3_state.multi_model_operations, 0);
    atomic64_set(&global_phase3_state.advanced_search_operations, 0);
    atomic64_set(&global_phase3_state.hnsw_operations, 0);
    atomic64_set(&global_phase3_state.lsh_operations, 0);
    
    /* Set defaults */
    global_phase3_state.dimensions = 0;
    global_phase3_state.distance_metric = VEXFS_DISTANCE_EUCLIDEAN;
    global_phase3_state.current_model = VEXFS_EMBED_OLLAMA_NOMIC;
    global_phase3_state.active_index_type = VEXFS_INDEX_BRUTE_FORCE;
    
    mutex_unlock(&phase3_global_mutex);
    
    printk(KERN_INFO "VexFS Phase 3: Integration module initialized\n");
    return 0;
}

/*
 * Cleanup Phase 3 integration
 */
void vexfs_phase3_cleanup(void)
{
    mutex_lock(&phase3_global_mutex);
    
    /* Cleanup all components */
    if (global_phase3_state.lsh_initialized) {
        vexfs_lsh_cleanup();
        global_phase3_state.lsh_initialized = false;
    }
    
    if (global_phase3_state.hnsw_initialized) {
        vexfs_hnsw_cleanup();
        global_phase3_state.hnsw_initialized = false;
    }
    
    if (global_phase3_state.advanced_search_initialized) {
        vexfs_advanced_search_cleanup();
        global_phase3_state.advanced_search_initialized = false;
    }
    
    if (global_phase3_state.multi_model_initialized) {
        vexfs_multi_model_cleanup();
        global_phase3_state.multi_model_initialized = false;
    }
    
    mutex_unlock(&phase3_global_mutex);
    
    printk(KERN_INFO "VexFS Phase 3: Integration cleanup completed\n");
}

/*
 * Handle multi-model metadata IOCTL
 */
static long handle_multi_model_ioctl(unsigned int cmd, unsigned long arg)
{
    struct vexfs_model_metadata metadata;
    int ret;
    
    atomic64_inc(&global_phase3_state.multi_model_operations);
    
    if (!global_phase3_state.multi_model_initialized) {
        ret = vexfs_multi_model_init();
        if (ret) {
            return ret;
        }
        global_phase3_state.multi_model_initialized = true;
    }
    
    switch (cmd) {
    case VEXFS_IOC_SET_MODEL_META:
        if (copy_from_user(&metadata, (void __user *)arg, sizeof(metadata))) {
            return -EFAULT;
        }
        
        ret = vexfs_set_model_metadata(&metadata);
        if (ret == 0) {
            /* Update global state */
            mutex_lock(&global_phase3_state.state_mutex);
            global_phase3_state.current_model = metadata.model_type;
            global_phase3_state.dimensions = metadata.dimensions;
            mutex_unlock(&global_phase3_state.state_mutex);
        }
        return ret;
        
    case VEXFS_IOC_GET_MODEL_META:
        ret = vexfs_get_model_metadata(&metadata);
        if (ret == 0) {
            if (copy_to_user((void __user *)arg, &metadata, sizeof(metadata))) {
                return -EFAULT;
            }
        }
        return ret;
        
    default:
        return -ENOTTY;
    }
}

/*
 * Handle advanced search IOCTL
 */
static long handle_advanced_search_ioctl(unsigned int cmd, unsigned long arg)
{
    int ret;
    
    atomic64_inc(&global_phase3_state.advanced_search_operations);
    
    if (!global_phase3_state.advanced_search_initialized) {
        if (global_phase3_state.dimensions == 0) {
            printk(KERN_ERR "VexFS Phase 3: Dimensions not set, cannot initialize advanced search\n");
            return -EINVAL;
        }
        
        ret = vexfs_advanced_search_init();
        if (ret) {
            return ret;
        }
        global_phase3_state.advanced_search_initialized = true;
    }
    
    switch (cmd) {
    case VEXFS_IOC_FILTERED_SEARCH:
    case VEXFS_IOC_MULTI_VECTOR_SEARCH:
    case VEXFS_IOC_HYBRID_SEARCH:
        /* Delegate to advanced search IOCTL handler */
        return vexfs_advanced_search_ioctl(NULL, cmd, arg);
        
    default:
        return -ENOTTY;
    }
}

/*
 * Handle HNSW index IOCTL
 */
static long handle_hnsw_ioctl(unsigned int cmd, unsigned long arg)
{
    int ret;
    
    atomic64_inc(&global_phase3_state.hnsw_operations);
    
    switch (cmd) {
    case VEXFS_IOC_BUILD_INDEX:
        {
            struct vexfs_index_metadata index_meta;
            if (copy_from_user(&index_meta, (void __user *)arg, sizeof(index_meta))) {
                return -EFAULT;
            }
            
            if (index_meta.index_type == VEXFS_INDEX_HNSW) {
                if (global_phase3_state.hnsw_initialized) {
                    vexfs_hnsw_cleanup();
                }
                
                ret = vexfs_hnsw_init(index_meta.dimensions,
                                    index_meta.config.hnsw.max_connections,
                                    index_meta.config.hnsw.ef_construction);
                if (ret == 0) {
                    global_phase3_state.hnsw_initialized = true;
                    global_phase3_state.active_index_type = VEXFS_INDEX_HNSW;
                    
                    /* Update global dimensions */
                    mutex_lock(&global_phase3_state.state_mutex);
                    global_phase3_state.dimensions = index_meta.dimensions;
                    mutex_unlock(&global_phase3_state.state_mutex);
                }
            }
            return ret;
        }
        
    case VEXFS_IOC_BATCH_INSERT:
        {
            /* Use existing batch insert for HNSW */
            if (!global_phase3_state.hnsw_initialized) {
                return -EINVAL;
            }
            /* Delegate to main IOCTL handler */
            return -ENOTTY; /* Let main handler deal with it */
        }
        
    case VEXFS_IOC_KNN_SEARCH:
        {
            /* Use existing KNN search for HNSW */
            if (!global_phase3_state.hnsw_initialized) {
                return -EINVAL;
            }
            /* Delegate to main IOCTL handler */
            return -ENOTTY; /* Let main handler deal with it */
        }
        
    default:
        return -ENOTTY;
    }
}

/*
 * Handle LSH index IOCTL
 */
static long handle_lsh_ioctl(unsigned int cmd, unsigned long arg)
{
    int ret;
    
    atomic64_inc(&global_phase3_state.lsh_operations);
    
    switch (cmd) {
    case VEXFS_IOC_BUILD_INDEX:
        {
            struct vexfs_index_metadata index_meta;
            if (copy_from_user(&index_meta, (void __user *)arg, sizeof(index_meta))) {
                return -EFAULT;
            }
            
            if (index_meta.index_type == VEXFS_INDEX_LSH) {
                if (global_phase3_state.lsh_initialized) {
                    vexfs_lsh_cleanup();
                }
                
                ret = vexfs_lsh_init(index_meta.dimensions, VEXFS_DISTANCE_EUCLIDEAN,
                                   index_meta.config.lsh.num_hash_tables,
                                   index_meta.config.lsh.num_hash_functions);
                if (ret == 0) {
                    global_phase3_state.lsh_initialized = true;
                    global_phase3_state.active_index_type = VEXFS_INDEX_LSH;
                    
                    /* Update global state */
                    mutex_lock(&global_phase3_state.state_mutex);
                    global_phase3_state.dimensions = index_meta.dimensions;
                    global_phase3_state.distance_metric = VEXFS_DISTANCE_EUCLIDEAN;
                    mutex_unlock(&global_phase3_state.state_mutex);
                }
            }
            return ret;
        }
        
    case VEXFS_IOC_BATCH_INSERT:
        {
            /* Use existing batch insert for LSH */
            if (!global_phase3_state.lsh_initialized) {
                return -EINVAL;
            }
            /* Delegate to main IOCTL handler */
            return -ENOTTY; /* Let main handler deal with it */
        }
        
    case VEXFS_IOC_KNN_SEARCH:
        {
            /* Use existing KNN search for LSH */
            if (!global_phase3_state.lsh_initialized) {
                return -EINVAL;
            }
            /* Delegate to main IOCTL handler */
            return -ENOTTY; /* Let main handler deal with it */
        }
        
    default:
        return -ENOTTY;
    }
}

/*
 * Main Phase 3 IOCTL handler
 */
long vexfs_phase3_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
    atomic64_inc(&global_phase3_state.total_phase3_operations);
    
    /* Route to appropriate handler based on command */
    switch (cmd) {
    /* Multi-model commands (20-21) */
    case VEXFS_IOC_SET_MODEL_META:
    case VEXFS_IOC_GET_MODEL_META:
        return handle_multi_model_ioctl(cmd, arg);
        
    /* Advanced search commands (24-26) */
    case VEXFS_IOC_FILTERED_SEARCH:
    case VEXFS_IOC_MULTI_VECTOR_SEARCH:
    case VEXFS_IOC_HYBRID_SEARCH:
        return handle_advanced_search_ioctl(cmd, arg);
        
    /* Index building commands (22-23) */
    case VEXFS_IOC_BUILD_INDEX:
    case VEXFS_IOC_GET_INDEX_INFO:
        /* Determine which index type and route accordingly */
        if (global_phase3_state.active_index_type == VEXFS_INDEX_HNSW) {
            return handle_hnsw_ioctl(cmd, arg);
        } else if (global_phase3_state.active_index_type == VEXFS_INDEX_LSH) {
            return handle_lsh_ioctl(cmd, arg);
        }
        return -EINVAL;
        
    default:
        return -ENOTTY;
    }
}

/*
 * Get Phase 3 statistics
 */
int vexfs_phase3_get_stats(struct vexfs_phase3_stats *stats)
{
    if (!stats) {
        return -EINVAL;
    }
    
    memset(stats, 0, sizeof(*stats));
    
    /* Copy current state */
    mutex_lock(&global_phase3_state.state_mutex);
    mutex_unlock(&global_phase3_state.state_mutex);
    
    /* Copy statistics - only fields that exist in vexfs_phase3_stats */
    stats->multi_model_operations = (uint64_t)atomic64_read(&global_phase3_state.multi_model_operations);
    stats->hnsw_searches = (uint64_t)atomic64_read(&global_phase3_state.hnsw_operations);
    stats->lsh_searches = (uint64_t)atomic64_read(&global_phase3_state.lsh_operations);
    stats->filtered_searches = 0; /* Not tracked in this integration layer */
    stats->hybrid_searches = 0;   /* Not tracked in this integration layer */
    stats->index_builds = 0;      /* Not tracked in this integration layer */
    stats->index_updates = 0;     /* Not tracked in this integration layer */
    
    /* Performance metrics - set to zero for now */
    stats->avg_hnsw_search_time_ns = 0;
    stats->avg_lsh_search_time_ns = 0;
    stats->avg_index_build_time_ns = 0;
    
    return 0;
}

/*
 * Smart index selection based on query characteristics
 */
int vexfs_phase3_smart_search(const uint32_t *query_vector, uint32_t k, uint32_t dimensions,
                             struct vexfs_search_result *results, uint32_t *result_count)
{
    int ret = -ENODEV;
    
    /* Select best index based on current configuration and data size */
    if (global_phase3_state.hnsw_initialized && k <= 100) {
        /* HNSW is best for moderate k values */
        ret = vexfs_hnsw_search(query_vector, k, results, result_count);
        if (ret == 0) {
            atomic64_inc(&global_phase3_state.hnsw_operations);
            return ret;
        }
    }
    
    if (global_phase3_state.lsh_initialized && k >= 10) {
        /* LSH is good for larger k values and approximate results */
        ret = vexfs_lsh_search(query_vector, k, results, result_count);
        if (ret == 0) {
            atomic64_inc(&global_phase3_state.lsh_operations);
            return ret;
        }
    }
    
    /* Fallback to brute force search (Phase 2) */
    printk(KERN_DEBUG "VexFS Phase 3: Falling back to brute force search\n");
    return -ENODEV; /* Caller should use Phase 2 search */
}

/* Export symbols for module integration */
EXPORT_SYMBOL(vexfs_phase3_init);
EXPORT_SYMBOL(vexfs_phase3_cleanup);
EXPORT_SYMBOL(vexfs_phase3_ioctl);

/* Alias for main module compatibility */
long vexfs_v2_phase3_ioctl_handler(struct file *file, unsigned int cmd, unsigned long arg)
{
    return vexfs_phase3_ioctl(file, cmd, arg);
}
EXPORT_SYMBOL(vexfs_v2_phase3_ioctl_handler);
EXPORT_SYMBOL(vexfs_phase3_get_stats);
EXPORT_SYMBOL(vexfs_phase3_smart_search);

MODULE_DESCRIPTION("VexFS v2.0 Phase 3 Integration Module");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");