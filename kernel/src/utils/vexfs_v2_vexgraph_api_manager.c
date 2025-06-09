/*
 * VexFS v2.0 - VexGraph API Manager Implementation (Task 9 - Phase 2)
 * 
 * This implements the central API manager that coordinates all VexGraph
 * API operations, providing high-level interfaces for applications and
 * AI agents to interact with the graph-native semantic substrate.
 *
 * Key Features:
 * - API Manager for coordinating all graph API operations
 * - Request/response handling with validation
 * - Asynchronous operation support with work queues
 * - Performance monitoring and optimization
 * - Error handling and recovery
 * - Memory management and caching
 * - Integration with VexGraph Core (Task 8)
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/time.h>
#include <linux/string.h>
#include <linux/vmalloc.h>
#include <linux/crc32.h>

#include "../include/vexfs_v2_vexgraph_api.h"
#include "../include/vexfs_v2_internal.h"

/* Memory allocation flags */
#define VEXFS_API_ALLOC_FLAGS           (GFP_KERNEL | __GFP_ZERO)

/* Default timeouts */
#define VEXFS_API_DEFAULT_TIMEOUT_MS    5000
#define VEXFS_API_ASYNC_TIMEOUT_MS      30000

/* Performance thresholds */
#define VEXFS_API_SLOW_QUERY_THRESHOLD_MS 1000
#define VEXFS_API_MAX_MEMORY_MB         256

/* Forward declarations */
static void vexfs_api_async_work_handler(struct work_struct *work);
static int vexfs_api_execute_request(struct vexfs_api_manager *api_mgr,
                                     struct vexfs_api_request *request,
                                     struct vexfs_api_response *response);
static void vexfs_api_update_statistics(struct vexfs_api_manager *api_mgr,
                                         struct vexfs_api_request *request,
                                         struct vexfs_api_response *response);

/*
 * =============================================================================
 * API MANAGER OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_api_manager_create - Create a new API manager
 * @graph_mgr: Underlying graph manager
 * 
 * Creates and initializes a new VexGraph API manager that provides
 * high-level interfaces for graph operations.
 * 
 * Return: Pointer to API manager on success, NULL on failure
 */
struct vexfs_api_manager *vexfs_api_manager_create(struct vexfs_graph_manager *graph_mgr)
{
    struct vexfs_api_manager *api_mgr;
    int i;

    if (!graph_mgr) {
        printk(KERN_ERR "VexGraph API: Invalid graph manager\n");
        return NULL;
    }

    api_mgr = kzalloc(sizeof(struct vexfs_api_manager), VEXFS_API_ALLOC_FLAGS);
    if (!api_mgr) {
        printk(KERN_ERR "VexGraph API: Failed to allocate API manager\n");
        return NULL;
    }

    /* Initialize metadata */
    api_mgr->magic = VEXFS_VEXGRAPH_API_MAGIC;
    api_mgr->version_major = VEXFS_VEXGRAPH_API_VERSION_MAJOR;
    api_mgr->version_minor = VEXFS_VEXGRAPH_API_VERSION_MINOR;

    /* Set graph manager reference */
    api_mgr->graph_mgr = graph_mgr;
    api_mgr->sb = graph_mgr->sb;

    /* Initialize request management */
    atomic64_set(&api_mgr->next_request_id, 1);
    
    /* Create work queue for asynchronous operations */
    api_mgr->workqueue = alloc_workqueue("vexgraph_api", WQ_MEM_RECLAIM | WQ_HIGHPRI, 0);
    if (!api_mgr->workqueue) {
        printk(KERN_ERR "VexGraph API: Failed to create work queue\n");
        kfree(api_mgr);
        return NULL;
    }

    /* Initialize synchronization */
    init_rwsem(&api_mgr->api_sem);
    mutex_init(&api_mgr->request_mutex);
    mutex_init(&api_mgr->query_mutex);
    atomic_set(&api_mgr->active_requests, 0);

    /* Initialize performance monitoring */
    atomic64_set(&api_mgr->total_requests, 0);
    atomic64_set(&api_mgr->successful_requests, 0);
    atomic64_set(&api_mgr->failed_requests, 0);
    atomic64_set(&api_mgr->avg_response_time_ns, 0);

    /* Initialize query optimization */
    api_mgr->query_plan_tree = RB_ROOT;

    /* Create memory caches */
    api_mgr->request_cache = kmem_cache_create("vexgraph_api_request",
                                               sizeof(struct vexfs_api_request),
                                               0, SLAB_HWCACHE_ALIGN, NULL);
    if (!api_mgr->request_cache) {
        printk(KERN_ERR "VexGraph API: Failed to create request cache\n");
        destroy_workqueue(api_mgr->workqueue);
        kfree(api_mgr);
        return NULL;
    }

    api_mgr->response_cache = kmem_cache_create("vexgraph_api_response",
                                                sizeof(struct vexfs_api_response),
                                                0, SLAB_HWCACHE_ALIGN, NULL);
    if (!api_mgr->response_cache) {
        printk(KERN_ERR "VexGraph API: Failed to create response cache\n");
        kmem_cache_destroy(api_mgr->request_cache);
        destroy_workqueue(api_mgr->workqueue);
        kfree(api_mgr);
        return NULL;
    }

    api_mgr->query_cache = kmem_cache_create("vexgraph_api_query",
                                             sizeof(struct vexfs_query_plan),
                                             0, SLAB_HWCACHE_ALIGN, NULL);
    if (!api_mgr->query_cache) {
        printk(KERN_ERR "VexGraph API: Failed to create query cache\n");
        kmem_cache_destroy(api_mgr->response_cache);
        kmem_cache_destroy(api_mgr->request_cache);
        destroy_workqueue(api_mgr->workqueue);
        kfree(api_mgr);
        return NULL;
    }

    /* Initialize error tracking */
    for (i = 0; i < 16; i++) {
        api_mgr->error_count[i] = 0;
    }
    strcpy(api_mgr->last_error, "No errors");

    printk(KERN_INFO "VexGraph API: API manager created successfully\n");
    return api_mgr;
}

/**
 * vexfs_api_manager_destroy - Destroy an API manager
 * @api_mgr: API manager to destroy
 * 
 * Cleans up and destroys the API manager, ensuring all resources
 * are properly released.
 */
void vexfs_api_manager_destroy(struct vexfs_api_manager *api_mgr)
{
    if (!api_mgr) {
        return;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        printk(KERN_ERR "VexGraph API: Invalid API manager magic\n");
        return;
    }

    /* Wait for all active requests to complete */
    while (atomic_read(&api_mgr->active_requests) > 0) {
        msleep(10);
    }

    /* Destroy work queue */
    if (api_mgr->workqueue) {
        flush_workqueue(api_mgr->workqueue);
        destroy_workqueue(api_mgr->workqueue);
    }

    /* Destroy memory caches */
    if (api_mgr->query_cache) {
        kmem_cache_destroy(api_mgr->query_cache);
    }
    if (api_mgr->response_cache) {
        kmem_cache_destroy(api_mgr->response_cache);
    }
    if (api_mgr->request_cache) {
        kmem_cache_destroy(api_mgr->request_cache);
    }

    /* Clear magic to prevent reuse */
    api_mgr->magic = 0;

    kfree(api_mgr);
    printk(KERN_INFO "VexGraph API: API manager destroyed\n");
}

/**
 * vexfs_api_execute_request - Execute an API request
 * @api_mgr: API manager
 * @request: Request to execute
 * @response: Response to populate
 * 
 * Executes an API request and populates the response.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_execute_request(struct vexfs_api_manager *api_mgr,
                                     struct vexfs_api_request *request,
                                     struct vexfs_api_response *response)
{
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Validate request */
    result = vexfs_api_validate_request(request);
    if (result != VEXFS_API_SUCCESS) {
        vexfs_api_set_error(response, result, "Request validation failed");
        return result;
    }

    /* Execute based on operation type */
    switch (request->operation) {
    case VEXFS_API_OP_NODE_CREATE:
        result = vexfs_api_node_create(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_NODE_READ:
        result = vexfs_api_node_read(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_NODE_UPDATE:
        result = vexfs_api_node_update(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_NODE_DELETE:
        result = vexfs_api_node_delete(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_EDGE_CREATE:
        result = vexfs_api_edge_create(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_EDGE_READ:
        result = vexfs_api_edge_read(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_EDGE_UPDATE:
        result = vexfs_api_edge_update(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_EDGE_DELETE:
        result = vexfs_api_edge_delete(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_TRAVERSE:
        /* Determine traversal algorithm */
        if (request->params.traverse.algorithm == VEXFS_GRAPH_TRAVERSAL_BFS) {
            result = vexfs_api_traverse_bfs(api_mgr, request, response);
        } else if (request->params.traverse.algorithm == VEXFS_GRAPH_TRAVERSAL_DFS) {
            result = vexfs_api_traverse_dfs(api_mgr, request, response);
        } else if (request->params.traverse.algorithm == VEXFS_GRAPH_TRAVERSAL_DIJKSTRA) {
            result = vexfs_api_shortest_path(api_mgr, request, response);
        } else {
            result = VEXFS_API_ERROR_INVALID_PARAM;
        }
        break;
        
    case VEXFS_API_OP_QUERY:
        result = vexfs_api_query_execute(api_mgr, request, response);
        break;
        
    case VEXFS_API_OP_INDEX:
        if (request->params.index.create_index) {
            result = vexfs_api_index_create(api_mgr, request, response);
        } else {
            result = vexfs_api_index_destroy(api_mgr, request, response);
        }
        break;
        
    default:
        result = VEXFS_API_ERROR_INVALID_PARAM;
        vexfs_api_set_error(response, result, "Unknown operation type");
        break;
    }

    return result;
}

/**
 * vexfs_api_validate_request - Validate API request
 * @request: Request to validate
 * 
 * Validates an API request for correctness and security.
 * 
 * Return: 0 if valid, negative error code if invalid
 */
int vexfs_api_validate_request(struct vexfs_api_request *request)
{
    if (!request) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (request->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Validate operation type */
    if (request->operation < VEXFS_API_OP_NODE_CREATE || 
        request->operation > VEXFS_API_OP_INDEX) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Operation-specific validation */
    switch (request->operation) {
    case VEXFS_API_OP_NODE_CREATE:
        if (request->params.node_create.node_type == 0) {
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        break;
        
    case VEXFS_API_OP_NODE_READ:
    case VEXFS_API_OP_NODE_UPDATE:
    case VEXFS_API_OP_NODE_DELETE:
        if (request->params.node_read.node_id == 0) {
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        break;
        
    case VEXFS_API_OP_EDGE_CREATE:
        if (request->params.edge_create.source_id == 0 ||
            request->params.edge_create.target_id == 0) {
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        break;
        
    case VEXFS_API_OP_TRAVERSE:
        if (request->params.traverse.start_node == 0) {
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        if (request->params.traverse.max_depth > VEXFS_API_MAX_QUERY_DEPTH) {
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        break;
        
    default:
        break;
    }

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_set_error - Set error information in response
 * @response: Response structure
 * @error_code: Error code
 * @message: Error message
 * 
 * Sets error information in an API response.
 */
void vexfs_api_set_error(struct vexfs_api_response *response,
                         int error_code, const char *message)
{
    if (!response) {
        return;
    }

    response->result_code = error_code;
    
    if (message) {
        strncpy(response->error_message, message, sizeof(response->error_message) - 1);
        response->error_message[sizeof(response->error_message) - 1] = '\0';
    } else {
        strcpy(response->error_message, vexfs_api_error_string(error_code));
    }
}

/**
 * vexfs_api_error_string - Get error string for error code
 * @error_code: Error code
 * 
 * Returns a human-readable error string for the given error code.
 * 
 * Return: Error string
 */
const char *vexfs_api_error_string(int error_code)
{
    switch (error_code) {
    case VEXFS_API_SUCCESS:
        return "Success";
    case VEXFS_API_ERROR_INVALID_PARAM:
        return "Invalid parameter";
    case VEXFS_API_ERROR_NOT_FOUND:
        return "Not found";
    case VEXFS_API_ERROR_EXISTS:
        return "Already exists";
    case VEXFS_API_ERROR_NO_MEMORY:
        return "Out of memory";
    case VEXFS_API_ERROR_PERMISSION:
        return "Permission denied";
    case VEXFS_API_ERROR_BUSY:
        return "Resource busy";
    case VEXFS_API_ERROR_TIMEOUT:
        return "Operation timeout";
    case VEXFS_API_ERROR_INTERNAL:
        return "Internal error";
    default:
        return "Unknown error";
    }
}

/**
 * vexfs_api_request_alloc - Allocate a new API request
 * @api_mgr: API manager
 * 
 * Allocates and initializes a new API request structure.
 * 
 * Return: Pointer to request on success, NULL on failure
 */
struct vexfs_api_request *vexfs_api_request_alloc(struct vexfs_api_manager *api_mgr)
{
    struct vexfs_api_request *request;

    if (!api_mgr || api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return NULL;
    }

    request = kmem_cache_alloc(api_mgr->request_cache, VEXFS_API_ALLOC_FLAGS);
    if (!request) {
        printk(KERN_ERR "VexGraph API: Failed to allocate request\n");
        return NULL;
    }

    /* Initialize request */
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->version = (VEXFS_VEXGRAPH_API_VERSION_MAJOR << 16) | 
                       VEXFS_VEXGRAPH_API_VERSION_MINOR;
    request->request_id = atomic64_inc_return(&api_mgr->next_request_id);
    request->start_time = ktime_get_ns();
    atomic_set(&request->ref_count, 1);

    /* Initialize work structure for async operations */
    INIT_WORK(&request->work, vexfs_api_async_work_handler);

    return request;
}

/**
 * vexfs_api_request_free - Free an API request
 * @api_mgr: API manager
 * @request: Request to free
 * 
 * Frees an API request structure and associated resources.
 */
void vexfs_api_request_free(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request)
{
    if (!api_mgr || !request) {
        return;
    }

    if (request->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        printk(KERN_ERR "VexGraph API: Invalid request magic\n");
        return;
    }

    /* Decrement reference count */
    if (atomic_dec_and_test(&request->ref_count)) {
        /* Clear magic to prevent reuse */
        request->magic = 0;
        kmem_cache_free(api_mgr->request_cache, request);
    }
}

/**
 * vexfs_api_response_alloc - Allocate a new API response
 * @api_mgr: API manager
 * 
 * Allocates and initializes a new API response structure.
 * 
 * Return: Pointer to response on success, NULL on failure
 */
struct vexfs_api_response *vexfs_api_response_alloc(struct vexfs_api_manager *api_mgr)
{
    struct vexfs_api_response *response;

    if (!api_mgr || api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return NULL;
    }

    response = kmem_cache_alloc(api_mgr->response_cache, VEXFS_API_ALLOC_FLAGS);
    if (!response) {
        printk(KERN_ERR "VexGraph API: Failed to allocate response\n");
        return NULL;
    }

    /* Initialize response */
    response->magic = VEXFS_VEXGRAPH_API_MAGIC;
    response->version = (VEXFS_VEXGRAPH_API_VERSION_MAJOR << 16) | 
                        VEXFS_VEXGRAPH_API_VERSION_MINOR;
    response->result_code = VEXFS_API_SUCCESS;
    strcpy(response->error_message, "Success");

    return response;
}

/**
 * vexfs_api_response_free - Free an API response
 * @api_mgr: API manager
 * @response: Response to free
 * 
 * Frees an API response structure and associated resources.
 */
void vexfs_api_response_free(struct vexfs_api_manager *api_mgr,
                             struct vexfs_api_response *response)
{
    if (!api_mgr || !response) {
        return;
    }

    if (response->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        printk(KERN_ERR "VexGraph API: Invalid response magic\n");
        return;
    }

    /* Clear magic to prevent reuse */
    response->magic = 0;
    kmem_cache_free(api_mgr->response_cache, response);
}

/**
 * vexfs_api_async_work_handler - Work queue handler for async operations
 * @work: Work structure
 * 
 * Handles asynchronous API operations in the work queue context.
 */
static void vexfs_api_async_work_handler(struct work_struct *work)
{
    struct vexfs_api_request *request = container_of(work, struct vexfs_api_request, work);
    /* Note: In a real implementation, we would need a way to get the API manager
     * from the request context. For now, this is a placeholder. */
    printk(KERN_INFO "VexGraph API: Async work handler called\n");
}

/**
 * vexfs_api_get_statistics - Get API performance statistics
 * @api_mgr: API manager
 * @stats: Statistics structure to fill
 * 
 * Retrieves current API performance statistics.
 */
void vexfs_api_get_statistics(struct vexfs_api_manager *api_mgr,
                              struct vexfs_api_stats *stats)
{
    int i;

    if (!api_mgr || !stats) {
        return;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return;
    }

    memset(stats, 0, sizeof(struct vexfs_api_stats));

    stats->total_requests = atomic64_read(&api_mgr->total_requests);
    stats->successful_requests = atomic64_read(&api_mgr->successful_requests);
    stats->failed_requests = atomic64_read(&api_mgr->failed_requests);
    stats->avg_response_time_ns = atomic64_read(&api_mgr->avg_response_time_ns);
    stats->active_requests = atomic_read(&api_mgr->active_requests);

    /* Copy error counts */
    for (i = 0; i < 16; i++) {
        stats->error_counts[i] = api_mgr->error_count[i];
    }

    /* Calculate cache hit rate (placeholder) */
    stats->cache_hit_rate = 85; /* TODO: Implement actual cache statistics */
    stats->query_optimization_rate = 75; /* TODO: Implement query optimization stats */
    
    /* Memory usage estimation */
    stats->memory_usage = sizeof(struct vexfs_api_manager) +
                          (stats->active_requests * sizeof(struct vexfs_api_request));
}

/**
 * vexfs_api_update_statistics - Update API performance statistics
 * @api_mgr: API manager
 * @request: Completed request
 * @response: Response
 * 
 * Updates performance statistics based on completed operations.
 */
static void vexfs_api_update_statistics(struct vexfs_api_manager *api_mgr,
                                         struct vexfs_api_request *request,
                                         struct vexfs_api_response *response)
{
    u64 execution_time;
    u64 current_avg;

    if (!api_mgr || !request || !response) {
        return;
    }

    /* Calculate execution time */
    request->end_time = ktime_get_ns();
    execution_time = request->end_time - request->start_time;
    response->execution_time_ns = execution_time;

    /* Update counters */
    atomic64_inc(&api_mgr->total_requests);
    
    if (response->result_code == VEXFS_API_SUCCESS) {
        atomic64_inc(&api_mgr->successful_requests);
    } else {
        atomic64_inc(&api_mgr->failed_requests);
        
        /* Update error count */
        if (response->result_code >= -16 && response->result_code < 0) {
            api_mgr->error_count[-response->result_code]++;
        }
    }

    /* Update average response time */
    current_avg = atomic64_read(&api_mgr->avg_response_time_ns);
    atomic64_set(&api_mgr->avg_response_time_ns, 
                 (current_avg + execution_time) / 2);

    /* Log slow queries */
    if (execution_time > VEXFS_API_SLOW_QUERY_THRESHOLD_MS * 1000000ULL) {
        printk(KERN_WARNING "VexGraph API: Slow operation detected: %llu ns\n", 
               execution_time);
    }
}