/*
 * VexFS v2.0 - VexGraph Traversal API Implementation (Task 9 - Phase 2)
 * 
 * This implements the Traversal API operations for VexGraph, providing
 * high-level interfaces for graph traversal algorithms including BFS,
 * DFS, shortest path, and custom traversals.
 *
 * Key Features:
 * - Breadth-First Search (BFS) traversal
 * - Depth-First Search (DFS) traversal
 * - Shortest path algorithms (Dijkstra)
 * - Custom traversal with filters
 * - Performance optimization and result caching
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

/* Traversal result helpers */
static int vexfs_api_allocate_traversal_results(struct vexfs_api_response *response,
                                                 u32 max_results);
static void vexfs_api_free_traversal_results(struct vexfs_api_response *response);
static int vexfs_api_parse_traversal_filters(const char *filters_json,
                                              struct vexfs_graph_query_context *ctx);

/*
 * =============================================================================
 * TRAVERSAL API OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_api_traverse_bfs - Perform Breadth-First Search traversal
 * @api_mgr: API manager
 * @request: Traversal request
 * @response: Response to populate
 * 
 * Performs a BFS traversal starting from the specified node, optionally
 * with filters and depth limits.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_traverse_bfs(struct vexfs_api_manager *api_mgr,
                           struct vexfs_api_request *request,
                           struct vexfs_api_response *response)
{
    struct vexfs_graph_query_context *ctx;
    struct vexfs_graph_node *start_node;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Verify start node exists */
    start_node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                          request->params.traverse.start_node);
    if (!start_node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Start node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Create query context */
    ctx = vexfs_graph_query_create(api_mgr->graph_mgr);
    if (!ctx) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to create query context");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Configure traversal parameters */
    ctx->traversal_algorithm = VEXFS_GRAPH_TRAVERSAL_BFS;
    ctx->start_node_id = request->params.traverse.start_node;
    ctx->end_node_id = request->params.traverse.end_node;
    ctx->max_depth = request->params.traverse.max_depth;
    ctx->max_results = min(request->params.traverse.max_results, 
                           (u32)VEXFS_API_MAX_RESULTS);

    /* Parse filters if provided */
    if (request->params.traverse.filters_json) {
        result = vexfs_api_parse_traversal_filters(
            request->params.traverse.filters_json, ctx);
        if (result != 0) {
            vexfs_graph_query_destroy(ctx);
            up_read(&api_mgr->api_sem);
            vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                                "Failed to parse traversal filters");
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
    }

    /* Allocate result arrays */
    result = vexfs_api_allocate_traversal_results(response, ctx->max_results);
    if (result != 0) {
        vexfs_graph_query_destroy(ctx);
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to allocate result arrays");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Execute BFS traversal */
    result = vexfs_graph_traverse_bfs(api_mgr->graph_mgr, ctx);
    if (result != 0) {
        vexfs_api_free_traversal_results(response);
        vexfs_graph_query_destroy(ctx);
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_INTERNAL,
                            "BFS traversal failed");
        return VEXFS_API_ERROR_INTERNAL;
    }

    /* Copy results to response */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.traverse.result_count = ctx->result_count;
    response->data.traverse.nodes_visited = ctx->result_count; /* Simplified */

    /* Copy node and edge results */
    if (ctx->result_count > 0) {
        memcpy(response->data.traverse.result_nodes, ctx->result_nodes,
               sizeof(u64) * ctx->result_count);
        
        if (ctx->result_edges) {
            memcpy(response->data.traverse.result_edges, ctx->result_edges,
                   sizeof(u64) * ctx->result_count);
        }
        
        if (ctx->result_distances) {
            memcpy(response->data.traverse.distances, ctx->result_distances,
                   sizeof(u32) * ctx->result_count);
        }
    }

    /* Cleanup */
    vexfs_graph_query_destroy(ctx);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: BFS traversal from node %llu found %u results\n",
           request->params.traverse.start_node, ctx->result_count);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_traverse_dfs - Perform Depth-First Search traversal
 * @api_mgr: API manager
 * @request: Traversal request
 * @response: Response to populate
 * 
 * Performs a DFS traversal starting from the specified node, optionally
 * with filters and depth limits.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_traverse_dfs(struct vexfs_api_manager *api_mgr,
                           struct vexfs_api_request *request,
                           struct vexfs_api_response *response)
{
    struct vexfs_graph_query_context *ctx;
    struct vexfs_graph_node *start_node;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Verify start node exists */
    start_node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                          request->params.traverse.start_node);
    if (!start_node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Start node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Create query context */
    ctx = vexfs_graph_query_create(api_mgr->graph_mgr);
    if (!ctx) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to create query context");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Configure traversal parameters */
    ctx->traversal_algorithm = VEXFS_GRAPH_TRAVERSAL_DFS;
    ctx->start_node_id = request->params.traverse.start_node;
    ctx->end_node_id = request->params.traverse.end_node;
    ctx->max_depth = request->params.traverse.max_depth;
    ctx->max_results = min(request->params.traverse.max_results, 
                           (u32)VEXFS_API_MAX_RESULTS);

    /* Parse filters if provided */
    if (request->params.traverse.filters_json) {
        result = vexfs_api_parse_traversal_filters(
            request->params.traverse.filters_json, ctx);
        if (result != 0) {
            vexfs_graph_query_destroy(ctx);
            up_read(&api_mgr->api_sem);
            vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                                "Failed to parse traversal filters");
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
    }

    /* Allocate result arrays */
    result = vexfs_api_allocate_traversal_results(response, ctx->max_results);
    if (result != 0) {
        vexfs_graph_query_destroy(ctx);
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to allocate result arrays");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Execute DFS traversal */
    result = vexfs_graph_traverse_dfs(api_mgr->graph_mgr, ctx);
    if (result != 0) {
        vexfs_api_free_traversal_results(response);
        vexfs_graph_query_destroy(ctx);
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_INTERNAL,
                            "DFS traversal failed");
        return VEXFS_API_ERROR_INTERNAL;
    }

    /* Copy results to response */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.traverse.result_count = ctx->result_count;
    response->data.traverse.nodes_visited = ctx->result_count; /* Simplified */

    /* Copy node and edge results */
    if (ctx->result_count > 0) {
        memcpy(response->data.traverse.result_nodes, ctx->result_nodes,
               sizeof(u64) * ctx->result_count);
        
        if (ctx->result_edges) {
            memcpy(response->data.traverse.result_edges, ctx->result_edges,
                   sizeof(u64) * ctx->result_count);
        }
        
        if (ctx->result_distances) {
            memcpy(response->data.traverse.distances, ctx->result_distances,
                   sizeof(u32) * ctx->result_count);
        }
    }

    /* Cleanup */
    vexfs_graph_query_destroy(ctx);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: DFS traversal from node %llu found %u results\n",
           request->params.traverse.start_node, ctx->result_count);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_shortest_path - Find shortest path between two nodes
 * @api_mgr: API manager
 * @request: Traversal request
 * @response: Response to populate
 * 
 * Finds the shortest path between two nodes using Dijkstra's algorithm.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_shortest_path(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request,
                            struct vexfs_api_response *response)
{
    struct vexfs_graph_node *start_node, *end_node;
    u64 *path;
    u32 path_length;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Require both start and end nodes for shortest path */
    if (request->params.traverse.end_node == 0) {
        vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                            "End node required for shortest path");
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Verify start node exists */
    start_node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                          request->params.traverse.start_node);
    if (!start_node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Start node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Verify end node exists */
    end_node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                        request->params.traverse.end_node);
    if (!end_node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "End node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Allocate path array */
    path = kzalloc(sizeof(u64) * VEXFS_API_MAX_QUERY_DEPTH, GFP_KERNEL);
    if (!path) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to allocate path array");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Execute shortest path algorithm */
    result = vexfs_graph_shortest_path(api_mgr->graph_mgr,
                                       request->params.traverse.start_node,
                                       request->params.traverse.end_node,
                                       path, &path_length);
    if (result != 0) {
        kfree(path);
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "No path found between nodes");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Allocate result arrays */
    result = vexfs_api_allocate_traversal_results(response, path_length);
    if (result != 0) {
        kfree(path);
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to allocate result arrays");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Copy results to response */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.traverse.result_count = path_length;
    response->data.traverse.nodes_visited = path_length;

    /* Copy path to result nodes */
    memcpy(response->data.traverse.result_nodes, path,
           sizeof(u64) * path_length);

    /* Set distances (0 to path_length-1) */
    for (u32 i = 0; i < path_length; i++) {
        response->data.traverse.distances[i] = i;
    }

    /* Cleanup */
    kfree(path);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Shortest path from %llu to %llu: %u hops\n",
           request->params.traverse.start_node, request->params.traverse.end_node,
           path_length);

    return VEXFS_API_SUCCESS;
}

/*
 * =============================================================================
 * TRAVERSAL HELPER FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_api_allocate_traversal_results - Allocate result arrays for traversal
 * @response: Response structure
 * @max_results: Maximum number of results
 * 
 * Allocates memory for traversal result arrays.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_allocate_traversal_results(struct vexfs_api_response *response,
                                                 u32 max_results)
{
    if (!response || max_results == 0) {
        return -EINVAL;
    }

    /* Allocate result node array */
    response->data.traverse.result_nodes = kzalloc(sizeof(u64) * max_results,
                                                    GFP_KERNEL);
    if (!response->data.traverse.result_nodes) {
        return -ENOMEM;
    }

    /* Allocate result edge array */
    response->data.traverse.result_edges = kzalloc(sizeof(u64) * max_results,
                                                    GFP_KERNEL);
    if (!response->data.traverse.result_edges) {
        kfree(response->data.traverse.result_nodes);
        response->data.traverse.result_nodes = NULL;
        return -ENOMEM;
    }

    /* Allocate distances array */
    response->data.traverse.distances = kzalloc(sizeof(u32) * max_results,
                                                 GFP_KERNEL);
    if (!response->data.traverse.distances) {
        kfree(response->data.traverse.result_edges);
        kfree(response->data.traverse.result_nodes);
        response->data.traverse.result_edges = NULL;
        response->data.traverse.result_nodes = NULL;
        return -ENOMEM;
    }

    return 0;
}

/**
 * vexfs_api_free_traversal_results - Free traversal result arrays
 * @response: Response structure
 * 
 * Frees memory allocated for traversal result arrays.
 */
static void vexfs_api_free_traversal_results(struct vexfs_api_response *response)
{
    if (!response) {
        return;
    }

    if (response->data.traverse.distances) {
        kfree(response->data.traverse.distances);
        response->data.traverse.distances = NULL;
    }

    if (response->data.traverse.result_edges) {
        kfree(response->data.traverse.result_edges);
        response->data.traverse.result_edges = NULL;
    }

    if (response->data.traverse.result_nodes) {
        kfree(response->data.traverse.result_nodes);
        response->data.traverse.result_nodes = NULL;
    }
}

/**
 * vexfs_api_parse_traversal_filters - Parse JSON filters for traversal
 * @filters_json: JSON string containing filters
 * @ctx: Query context to populate
 * 
 * Parses traversal filters from JSON and applies them to the query context.
 * This is a simplified implementation for kernel space.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_parse_traversal_filters(const char *filters_json,
                                              struct vexfs_graph_query_context *ctx)
{
    const char *ptr = filters_json;
    char key[64];
    char value[256];

    if (!filters_json || !ctx) {
        return -EINVAL;
    }

    /* Simplified filter parsing for kernel space
     * Expected format: {"node_type": 1, "edge_type": 2, "property": "name"}
     */

    /* Skip opening brace */
    while (*ptr && (*ptr == ' ' || *ptr == '\t' || *ptr == '{')) {
        ptr++;
    }

    /* Parse key-value pairs */
    while (*ptr && *ptr != '}') {
        /* Skip whitespace */
        while (*ptr && (*ptr == ' ' || *ptr == '\t' || *ptr == ',')) {
            ptr++;
        }

        if (*ptr == '}') break;

        /* Parse key (expect quoted string) */
        if (*ptr != '"') {
            printk(KERN_ERR "VexGraph API: Expected quoted key in filter JSON\n");
            return -EINVAL;
        }
        ptr++; /* Skip opening quote */

        /* Extract key */
        int key_len = 0;
        while (*ptr && *ptr != '"' && key_len < sizeof(key) - 1) {
            key[key_len++] = *ptr++;
        }
        key[key_len] = '\0';

        if (*ptr != '"') {
            printk(KERN_ERR "VexGraph API: Unterminated key in filter JSON\n");
            return -EINVAL;
        }
        ptr++; /* Skip closing quote */

        /* Skip whitespace and colon */
        while (*ptr && (*ptr == ' ' || *ptr == '\t' || *ptr == ':')) {
            ptr++;
        }

        /* Parse value based on key */
        if (strcmp(key, "node_type") == 0) {
            /* Numeric node type */
            if (*ptr >= '0' && *ptr <= '9') {
                u8 node_type = 0;
                while (*ptr >= '0' && *ptr <= '9') {
                    node_type = node_type * 10 + (*ptr - '0');
                    ptr++;
                }
                ctx->node_type_filter = node_type;
                printk(KERN_INFO "VexGraph API: Set node type filter to %u\n", node_type);
            }
        } else if (strcmp(key, "edge_type") == 0) {
            /* Numeric edge type */
            if (*ptr >= '0' && *ptr <= '9') {
                u8 edge_type = 0;
                while (*ptr >= '0' && *ptr <= '9') {
                    edge_type = edge_type * 10 + (*ptr - '0');
                    ptr++;
                }
                ctx->edge_type_filter = edge_type;
                printk(KERN_INFO "VexGraph API: Set edge type filter to %u\n", edge_type);
            }
        } else if (strcmp(key, "property") == 0) {
            /* String property filter */
            if (*ptr == '"') {
                ptr++; /* Skip opening quote */
                int value_len = 0;
                while (*ptr && *ptr != '"' && value_len < sizeof(value) - 1) {
                    value[value_len++] = *ptr++;
                }
                value[value_len] = '\0';
                
                if (*ptr == '"') ptr++; /* Skip closing quote */

                strncpy(ctx->property_filter, value, sizeof(ctx->property_filter) - 1);
                ctx->property_filter[sizeof(ctx->property_filter) - 1] = '\0';
                printk(KERN_INFO "VexGraph API: Set property filter to %s\n", value);
            }
        }

        /* Skip to next key-value pair */
        while (*ptr && *ptr != ',' && *ptr != '}') {
            ptr++;
        }
    }

    return 0;
}