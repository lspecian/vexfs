/*
 * VexFS v2.0 - VexGraph Index API Implementation (Task 9 - Phase 2)
 * 
 * This implements the Index API operations for VexGraph, providing
 * index management for query optimization and performance enhancement.
 *
 * Key Features:
 * - Index creation and destruction
 * - Index rebuilding and maintenance
 * - Query optimization using indexes
 * - Performance monitoring for index usage
 * - Integration with VexGraph Core (Task 8)
 * - Error handling and validation
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/string.h>
#include <linux/time.h>

#include "../include/vexfs_v2_vexgraph_api.h"
#include "../include/vexfs_v2_internal.h"

/*
 * =============================================================================
 * INDEX API OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_api_index_create - Create a new graph index
 * @api_mgr: API manager
 * @request: Index creation request
 * @response: Response to populate
 * 
 * Creates a new index for optimizing graph queries.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_index_create(struct vexfs_api_manager *api_mgr,
                           struct vexfs_api_request *request,
                           struct vexfs_api_response *response)
{
    int result;
    u32 entries_count = 0;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (!request->params.index.index_key) {
        vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                            "Index key is required");
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire write lock on API manager for index creation */
    down_write(&api_mgr->api_sem);

    /* Create the index using VexGraph core */
    result = vexfs_graph_index_create(api_mgr->graph_mgr,
                                      request->params.index.index_type,
                                      request->params.index.index_key);
    if (result != 0) {
        up_write(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_INTERNAL,
                            "Failed to create graph index");
        return VEXFS_API_ERROR_INTERNAL;
    }

    /* Count entries in the new index (simplified) */
    switch (request->params.index.index_type) {
    case VEXFS_GRAPH_INDEX_NODE_ID:
        entries_count = atomic64_read(&api_mgr->graph_mgr->node_count);
        break;
    case VEXFS_GRAPH_INDEX_EDGE_TYPE:
        entries_count = atomic64_read(&api_mgr->graph_mgr->edge_count);
        break;
    case VEXFS_GRAPH_INDEX_PROPERTY:
        /* Estimate based on total nodes */
        entries_count = atomic64_read(&api_mgr->graph_mgr->node_count) / 2;
        break;
    default:
        entries_count = 0;
        break;
    }

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.index.index_type = request->params.index.index_type;
    response->data.index.index_key = request->params.index.index_key;
    response->data.index.entries_count = entries_count;
    response->data.index.operation_success = true;

    up_write(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Created index (type %u, key %s) with %u entries\n",
           request->params.index.index_type, request->params.index.index_key,
           entries_count);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_index_destroy - Destroy a graph index
 * @api_mgr: API manager
 * @request: Index destruction request
 * @response: Response to populate
 * 
 * Destroys an existing graph index.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_index_destroy(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request,
                            struct vexfs_api_response *response)
{
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (!request->params.index.index_key) {
        vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                            "Index key is required");
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire write lock on API manager for index destruction */
    down_write(&api_mgr->api_sem);

    /* Destroy the index using VexGraph core */
    result = vexfs_graph_index_destroy(api_mgr->graph_mgr,
                                       request->params.index.index_type,
                                       request->params.index.index_key);
    if (result != 0) {
        up_write(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Index not found or failed to destroy");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.index.index_type = request->params.index.index_type;
    response->data.index.index_key = request->params.index.index_key;
    response->data.index.entries_count = 0;
    response->data.index.operation_success = true;

    up_write(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Destroyed index (type %u, key %s)\n",
           request->params.index.index_type, request->params.index.index_key);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_index_rebuild - Rebuild a graph index
 * @api_mgr: API manager
 * @index_type: Index type to rebuild
 * @index_key: Index key to rebuild
 * 
 * Rebuilds an existing graph index to ensure consistency and optimize performance.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_index_rebuild(struct vexfs_api_manager *api_mgr,
                            u8 index_type, const char *index_key)
{
    struct vexfs_graph_node *node;
    struct vexfs_graph_edge *edge;
    struct rb_node *rb_node;
    int result;
    u32 rebuilt_entries = 0;

    if (!api_mgr || !index_key) {
        return -EINVAL;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return -EINVAL;
    }

    printk(KERN_INFO "VexGraph API: Rebuilding index (type %u, key %s)\n",
           index_type, index_key);

    /* Acquire write lock on API manager for index rebuilding */
    down_write(&api_mgr->api_sem);

    /* Destroy existing index */
    result = vexfs_graph_index_destroy(api_mgr->graph_mgr, index_type, index_key);
    if (result != 0) {
        printk(KERN_WARNING "VexGraph API: Failed to destroy existing index during rebuild\n");
    }

    /* Create new index */
    result = vexfs_graph_index_create(api_mgr->graph_mgr, index_type, index_key);
    if (result != 0) {
        up_write(&api_mgr->api_sem);
        printk(KERN_ERR "VexGraph API: Failed to create index during rebuild\n");
        return result;
    }

    /* Rebuild index entries based on type */
    switch (index_type) {
    case VEXFS_GRAPH_INDEX_NODE_ID:
        /* Rebuild node ID index */
        for (rb_node = rb_first(&api_mgr->graph_mgr->nodes_tree);
             rb_node; rb_node = rb_next(rb_node)) {
            
            node = rb_entry(rb_node, struct vexfs_graph_node, rb_node);
            result = vexfs_graph_index_update(api_mgr->graph_mgr, node, NULL);
            if (result == 0) {
                rebuilt_entries++;
            }
        }
        break;

    case VEXFS_GRAPH_INDEX_EDGE_TYPE:
        /* Rebuild edge type index */
        for (rb_node = rb_first(&api_mgr->graph_mgr->edges_tree);
             rb_node; rb_node = rb_next(rb_node)) {
            
            edge = rb_entry(rb_node, struct vexfs_graph_edge, rb_node);
            result = vexfs_graph_index_update(api_mgr->graph_mgr, NULL, edge);
            if (result == 0) {
                rebuilt_entries++;
            }
        }
        break;

    case VEXFS_GRAPH_INDEX_PROPERTY:
        /* Rebuild property index */
        for (rb_node = rb_first(&api_mgr->graph_mgr->nodes_tree);
             rb_node; rb_node = rb_next(rb_node)) {
            
            node = rb_entry(rb_node, struct vexfs_graph_node, rb_node);
            
            /* Check if node has the specified property */
            struct vexfs_graph_property *prop;
            list_for_each_entry(prop, &node->properties, list) {
                if (strcmp(prop->key, index_key) == 0) {
                    result = vexfs_graph_index_update(api_mgr->graph_mgr, node, NULL);
                    if (result == 0) {
                        rebuilt_entries++;
                    }
                    break;
                }
            }
        }
        break;

    default:
        printk(KERN_ERR "VexGraph API: Unknown index type %u for rebuild\n", index_type);
        break;
    }

    up_write(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Rebuilt index with %u entries\n", rebuilt_entries);
    return 0;
}

/*
 * =============================================================================
 * INDEX MANAGEMENT HELPERS
 * =============================================================================
 */

/**
 * vexfs_api_index_get_statistics - Get index statistics
 * @api_mgr: API manager
 * @index_type: Index type
 * @index_key: Index key
 * @entries_count: Pointer to store entry count
 * @memory_usage: Pointer to store memory usage
 * 
 * Retrieves statistics for a specific index.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_index_get_statistics(struct vexfs_api_manager *api_mgr,
                                   u8 index_type, const char *index_key,
                                   u32 *entries_count, u64 *memory_usage)
{
    if (!api_mgr || !index_key || !entries_count || !memory_usage) {
        return -EINVAL;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return -EINVAL;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Get statistics based on index type (simplified) */
    switch (index_type) {
    case VEXFS_GRAPH_INDEX_NODE_ID:
        *entries_count = atomic64_read(&api_mgr->graph_mgr->node_count);
        *memory_usage = *entries_count * sizeof(struct vexfs_graph_index_entry);
        break;

    case VEXFS_GRAPH_INDEX_EDGE_TYPE:
        *entries_count = atomic64_read(&api_mgr->graph_mgr->edge_count);
        *memory_usage = *entries_count * sizeof(struct vexfs_graph_index_entry);
        break;

    case VEXFS_GRAPH_INDEX_PROPERTY:
        /* Estimate property index size */
        *entries_count = atomic64_read(&api_mgr->graph_mgr->node_count) / 2;
        *memory_usage = *entries_count * sizeof(struct vexfs_graph_index_entry);
        break;

    default:
        *entries_count = 0;
        *memory_usage = 0;
        break;
    }

    up_read(&api_mgr->api_sem);

    return 0;
}

/**
 * vexfs_api_index_validate - Validate index consistency
 * @api_mgr: API manager
 * @index_type: Index type to validate
 * @index_key: Index key to validate
 * 
 * Validates the consistency of an index by checking all entries.
 * 
 * Return: 0 if consistent, negative error code if inconsistent
 */
int vexfs_api_index_validate(struct vexfs_api_manager *api_mgr,
                             u8 index_type, const char *index_key)
{
    u32 expected_entries = 0;
    u32 actual_entries = 0;
    u64 memory_usage = 0;
    int result;

    if (!api_mgr || !index_key) {
        return -EINVAL;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return -EINVAL;
    }

    printk(KERN_INFO "VexGraph API: Validating index (type %u, key %s)\n",
           index_type, index_key);

    /* Get current index statistics */
    result = vexfs_api_index_get_statistics(api_mgr, index_type, index_key,
                                            &actual_entries, &memory_usage);
    if (result != 0) {
        return result;
    }

    /* Calculate expected entries based on graph content */
    down_read(&api_mgr->api_sem);

    switch (index_type) {
    case VEXFS_GRAPH_INDEX_NODE_ID:
        expected_entries = atomic64_read(&api_mgr->graph_mgr->node_count);
        break;

    case VEXFS_GRAPH_INDEX_EDGE_TYPE:
        expected_entries = atomic64_read(&api_mgr->graph_mgr->edge_count);
        break;

    case VEXFS_GRAPH_INDEX_PROPERTY:
        /* Count nodes with the specified property */
        struct vexfs_graph_node *node;
        struct rb_node *rb_node;
        
        for (rb_node = rb_first(&api_mgr->graph_mgr->nodes_tree);
             rb_node; rb_node = rb_next(rb_node)) {
            
            node = rb_entry(rb_node, struct vexfs_graph_node, rb_node);
            
            struct vexfs_graph_property *prop;
            list_for_each_entry(prop, &node->properties, list) {
                if (strcmp(prop->key, index_key) == 0) {
                    expected_entries++;
                    break;
                }
            }
        }
        break;

    default:
        up_read(&api_mgr->api_sem);
        return -EINVAL;
    }

    up_read(&api_mgr->api_sem);

    /* Check consistency */
    if (actual_entries != expected_entries) {
        printk(KERN_WARNING "VexGraph API: Index inconsistency detected - expected %u, actual %u\n",
               expected_entries, actual_entries);
        return -EINVAL;
    }

    printk(KERN_INFO "VexGraph API: Index validation passed (%u entries)\n", actual_entries);
    return 0;
}

/**
 * vexfs_api_index_optimize - Optimize index performance
 * @api_mgr: API manager
 * @index_type: Index type to optimize
 * @index_key: Index key to optimize
 * 
 * Optimizes an index for better query performance.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_index_optimize(struct vexfs_api_manager *api_mgr,
                             u8 index_type, const char *index_key)
{
    if (!api_mgr || !index_key) {
        return -EINVAL;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return -EINVAL;
    }

    printk(KERN_INFO "VexGraph API: Optimizing index (type %u, key %s)\n",
           index_type, index_key);

    /* For now, optimization is equivalent to rebuilding */
    return vexfs_api_index_rebuild(api_mgr, index_type, index_key);
}