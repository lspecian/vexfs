/*
 * VexFS v2.0 - VexGraph API Layer (Task 9 - Phase 2)
 * 
 * This implements the comprehensive API layer for VexGraph operations,
 * providing high-level interfaces for applications and AI agents to
 * interact with the graph-native semantic substrate.
 *
 * Phase 2 Continuation: VexGraph API Implementation
 * - Comprehensive CRUD operations for nodes and edges
 * - High-level traversal algorithms and query interface
 * - Query language and optimization engine
 * - Asynchronous operations for high-performance workloads
 * - Thread-safe concurrent access patterns
 * - Error handling and validation framework
 * - Performance monitoring and optimization
 *
 * Building on VexGraph Core Structure (Task 8):
 * - Graph Manager with red-black trees and hash tables
 * - Node/Edge implementation with properties and relationships
 * - Graph algorithms (BFS, DFS, Dijkstra)
 * - Index system for efficient queries
 * - VexFS integration and journaling
 * - Comprehensive testing framework
 */

#ifndef _VEXFS_V2_VEXGRAPH_API_H
#define _VEXFS_V2_VEXGRAPH_API_H

#include <linux/types.h>
#include <linux/completion.h>
#include <linux/workqueue.h>
#include <linux/atomic.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>

#include "vexfs_v2_vexgraph.h"

/* VexGraph API magic numbers and version */
#define VEXFS_VEXGRAPH_API_MAGIC        0x56474150  /* "VGAP" */
#define VEXFS_VEXGRAPH_API_VERSION_MAJOR 1
#define VEXFS_VEXGRAPH_API_VERSION_MINOR 0

/* API operation types */
#define VEXFS_API_OP_NODE_CREATE        0x01
#define VEXFS_API_OP_NODE_READ          0x02
#define VEXFS_API_OP_NODE_UPDATE        0x03
#define VEXFS_API_OP_NODE_DELETE        0x04
#define VEXFS_API_OP_EDGE_CREATE        0x05
#define VEXFS_API_OP_EDGE_READ          0x06
#define VEXFS_API_OP_EDGE_UPDATE        0x07
#define VEXFS_API_OP_EDGE_DELETE        0x08
#define VEXFS_API_OP_TRAVERSE           0x09
#define VEXFS_API_OP_QUERY              0x0A
#define VEXFS_API_OP_INDEX              0x0B

/* API result codes */
#define VEXFS_API_SUCCESS               0
#define VEXFS_API_ERROR_INVALID_PARAM   -1
#define VEXFS_API_ERROR_NOT_FOUND       -2
#define VEXFS_API_ERROR_EXISTS          -3
#define VEXFS_API_ERROR_NO_MEMORY       -4
#define VEXFS_API_ERROR_PERMISSION      -5
#define VEXFS_API_ERROR_BUSY            -6
#define VEXFS_API_ERROR_TIMEOUT         -7
#define VEXFS_API_ERROR_INTERNAL        -8

/* API flags */
#define VEXFS_API_FLAG_ASYNC            0x01
#define VEXFS_API_FLAG_ATOMIC           0x02
#define VEXFS_API_FLAG_CACHED           0x04
#define VEXFS_API_FLAG_INDEXED          0x08
#define VEXFS_API_FLAG_VALIDATED        0x10

/* Query language operators */
#define VEXFS_QUERY_OP_EQUALS           0x01
#define VEXFS_QUERY_OP_NOT_EQUALS       0x02
#define VEXFS_QUERY_OP_GREATER          0x03
#define VEXFS_QUERY_OP_LESS             0x04
#define VEXFS_QUERY_OP_GREATER_EQUAL    0x05
#define VEXFS_QUERY_OP_LESS_EQUAL       0x06
#define VEXFS_QUERY_OP_CONTAINS         0x07
#define VEXFS_QUERY_OP_STARTS_WITH      0x08
#define VEXFS_QUERY_OP_ENDS_WITH        0x09
#define VEXFS_QUERY_OP_REGEX            0x0A

/* Query result ordering */
#define VEXFS_QUERY_ORDER_ASC           0x01
#define VEXFS_QUERY_ORDER_DESC          0x02
#define VEXFS_QUERY_ORDER_RELEVANCE     0x03

/* Maximum values for API operations */
#define VEXFS_API_MAX_BATCH_SIZE        1000
#define VEXFS_API_MAX_QUERY_DEPTH       100
#define VEXFS_API_MAX_RESULTS           10000
#define VEXFS_API_MAX_CONCURRENT_OPS    64

/*
 * API Request Structure - Generic request for all operations
 */
struct vexfs_api_request {
    u32 magic;                          /* Magic number */
    u32 version;                        /* API version */
    u8 operation;                       /* Operation type */
    u32 flags;                          /* Operation flags */
    u64 request_id;                     /* Unique request ID */
    
    /* Request parameters */
    union {
        struct {
            u64 inode_number;           /* Inode for node creation */
            u8 node_type;               /* Node type */
            char *properties_json;      /* Properties as JSON */
        } node_create;
        
        struct {
            u64 node_id;                /* Node ID to read */
            bool include_properties;    /* Include properties */
            bool include_edges;         /* Include edge information */
        } node_read;
        
        struct {
            u64 node_id;                /* Node ID to update */
            char *properties_json;      /* Updated properties */
            bool merge_properties;      /* Merge or replace */
        } node_update;
        
        struct {
            u64 node_id;                /* Node ID to delete */
            bool cascade_edges;         /* Delete connected edges */
        } node_delete;
        
        struct {
            u64 source_id;              /* Source node ID */
            u64 target_id;              /* Target node ID */
            u8 edge_type;               /* Edge type */
            u32 weight;                 /* Edge weight */
            char *properties_json;      /* Edge properties */
        } edge_create;
        
        struct {
            u64 edge_id;                /* Edge ID to read */
            bool include_properties;    /* Include properties */
        } edge_read;
        
        struct {
            u64 edge_id;                /* Edge ID to update */
            u32 weight;                 /* New weight */
            char *properties_json;      /* Updated properties */
        } edge_update;
        
        struct {
            u64 edge_id;                /* Edge ID to delete */
        } edge_delete;
        
        struct {
            u8 algorithm;               /* Traversal algorithm */
            u64 start_node;             /* Starting node */
            u64 end_node;               /* Target node (optional) */
            u32 max_depth;              /* Maximum depth */
            u32 max_results;            /* Maximum results */
            char *filters_json;         /* Filters as JSON */
        } traverse;
        
        struct {
            char *query_string;         /* Query in VexGraph QL */
            u32 max_results;            /* Maximum results */
            u32 timeout_ms;             /* Query timeout */
            bool use_index;             /* Use index optimization */
        } query;
        
        struct {
            u8 index_type;              /* Index type */
            char *index_key;            /* Index key */
            bool create_index;          /* Create or destroy */
        } index;
    } params;
    
    /* Asynchronous operation support */
    struct completion *completion;      /* Completion for async ops */
    struct work_struct work;            /* Work queue item */
    
    /* Timing and performance */
    u64 start_time;                     /* Request start time */
    u64 end_time;                       /* Request end time */
    
    /* Reference counting */
    atomic_t ref_count;                 /* Reference count */
};

/*
 * API Response Structure - Generic response for all operations
 */
struct vexfs_api_response {
    u32 magic;                          /* Magic number */
    u32 version;                        /* API version */
    u64 request_id;                     /* Matching request ID */
    int result_code;                    /* Result code */
    char error_message[256];            /* Error message */
    
    /* Response data */
    union {
        struct {
            u64 node_id;                /* Created node ID */
        } node_create;
        
        struct {
            u64 node_id;                /* Node ID */
            u8 node_type;               /* Node type */
            u64 inode_number;           /* Associated inode */
            char *properties_json;      /* Properties as JSON */
            u64 *outgoing_edges;        /* Outgoing edge IDs */
            u64 *incoming_edges;        /* Incoming edge IDs */
            u32 out_degree;             /* Outgoing edge count */
            u32 in_degree;              /* Incoming edge count */
        } node_read;
        
        struct {
            u64 node_id;                /* Updated node ID */
            u32 properties_updated;     /* Number of properties updated */
        } node_update;
        
        struct {
            u64 node_id;                /* Deleted node ID */
            u32 edges_deleted;          /* Number of edges deleted */
        } node_delete;
        
        struct {
            u64 edge_id;                /* Created edge ID */
        } edge_create;
        
        struct {
            u64 edge_id;                /* Edge ID */
            u64 source_id;              /* Source node ID */
            u64 target_id;              /* Target node ID */
            u8 edge_type;               /* Edge type */
            u32 weight;                 /* Edge weight */
            char *properties_json;      /* Properties as JSON */
        } edge_read;
        
        struct {
            u64 edge_id;                /* Updated edge ID */
            u32 properties_updated;     /* Number of properties updated */
        } edge_update;
        
        struct {
            u64 edge_id;                /* Deleted edge ID */
        } edge_delete;
        
        struct {
            u64 *result_nodes;          /* Result node IDs */
            u64 *result_edges;          /* Result edge IDs */
            u32 *distances;             /* Distances from start */
            u32 result_count;           /* Number of results */
            u32 nodes_visited;          /* Nodes visited during traversal */
        } traverse;
        
        struct {
            char *results_json;         /* Query results as JSON */
            u32 result_count;           /* Number of results */
            u32 execution_time_ms;      /* Query execution time */
            bool used_index;            /* Whether index was used */
        } query;
        
        struct {
            u8 index_type;              /* Index type */
            char *index_key;            /* Index key */
            u32 entries_count;          /* Number of index entries */
            bool operation_success;     /* Operation success */
        } index;
    } data;
    
    /* Performance metrics */
    u64 execution_time_ns;              /* Execution time in nanoseconds */
    u64 memory_used;                    /* Memory used for operation */
    u32 cache_hits;                     /* Cache hits during operation */
    u32 cache_misses;                   /* Cache misses during operation */
};

/*
 * Query Language Structure - VexGraph Query Language (VQL)
 */
struct vexfs_query_condition {
    char property_key[64];              /* Property key */
    u8 operator;                        /* Query operator */
    union {
        char *string_value;             /* String value */
        s64 int_value;                  /* Integer value */
        u32 float_bits;                 /* Float value as bits */
        bool bool_value;                /* Boolean value */
    } value;
    struct list_head list;              /* Condition list */
};

struct vexfs_query_filter {
    u8 node_type;                       /* Node type filter */
    u8 edge_type;                       /* Edge type filter */
    struct list_head conditions;       /* Query conditions */
    u8 logic_operator;                  /* AND/OR between conditions */
};

struct vexfs_query_order {
    char property_key[64];              /* Property to order by */
    u8 direction;                       /* Order direction */
    struct list_head list;              /* Order list */
};

struct vexfs_query_plan {
    struct vexfs_query_filter filter;  /* Query filter */
    struct list_head order_by;          /* Order by clauses */
    u32 limit;                          /* Result limit */
    u32 offset;                         /* Result offset */
    bool use_index;                     /* Use index optimization */
    char *index_hint;                   /* Index hint */
};

/*
 * API Manager - Central coordinator for API operations
 */
struct vexfs_api_manager {
    u32 magic;                          /* Magic number */
    u32 version_major;                  /* Major version */
    u32 version_minor;                  /* Minor version */
    
    /* Core graph manager */
    struct vexfs_graph_manager *graph_mgr; /* Underlying graph manager */
    
    /* Request management */
    atomic64_t next_request_id;         /* Next request ID */
    struct workqueue_struct *workqueue; /* Work queue for async ops */
    
    /* Concurrency control */
    struct rw_semaphore api_sem;        /* API read-write semaphore */
    struct mutex request_mutex;         /* Request serialization */
    atomic_t active_requests;           /* Active request count */
    
    /* Performance monitoring */
    atomic64_t total_requests;          /* Total requests processed */
    atomic64_t successful_requests;     /* Successful requests */
    atomic64_t failed_requests;         /* Failed requests */
    atomic64_t avg_response_time_ns;    /* Average response time */
    
    /* Query optimization */
    struct kmem_cache *query_cache;     /* Query plan cache */
    struct rb_root query_plan_tree;     /* Query plan tree */
    struct mutex query_mutex;           /* Query optimization mutex */
    
    /* Memory management */
    struct kmem_cache *request_cache;   /* Request allocation cache */
    struct kmem_cache *response_cache;  /* Response allocation cache */
    
    /* Error handling */
    u32 error_count[16];                /* Error count by type */
    char last_error[256];               /* Last error message */
    
    /* Integration */
    struct super_block *sb;             /* Associated superblock */
};

/*
 * =============================================================================
 * VEXGRAPH API FUNCTION DECLARATIONS
 * =============================================================================
 */

/* API Manager Operations */
struct vexfs_api_manager *vexfs_api_manager_create(struct vexfs_graph_manager *graph_mgr);
void vexfs_api_manager_destroy(struct vexfs_api_manager *api_mgr);
int vexfs_api_manager_init(struct vexfs_api_manager *api_mgr);
void vexfs_api_manager_cleanup(struct vexfs_api_manager *api_mgr);

/* Node API Operations */
int vexfs_api_node_create(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response);
int vexfs_api_node_read(struct vexfs_api_manager *api_mgr,
                        struct vexfs_api_request *request,
                        struct vexfs_api_response *response);
int vexfs_api_node_update(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response);
int vexfs_api_node_delete(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response);

/* Edge API Operations */
int vexfs_api_edge_create(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response);
int vexfs_api_edge_read(struct vexfs_api_manager *api_mgr,
                        struct vexfs_api_request *request,
                        struct vexfs_api_response *response);
int vexfs_api_edge_update(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response);
int vexfs_api_edge_delete(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response);

/* Traversal API Operations */
int vexfs_api_traverse_bfs(struct vexfs_api_manager *api_mgr,
                           struct vexfs_api_request *request,
                           struct vexfs_api_response *response);
int vexfs_api_traverse_dfs(struct vexfs_api_manager *api_mgr,
                           struct vexfs_api_request *request,
                           struct vexfs_api_response *response);
int vexfs_api_shortest_path(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request,
                            struct vexfs_api_response *response);

/* Query API Operations */
int vexfs_api_query_execute(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request,
                            struct vexfs_api_response *response);
int vexfs_api_query_parse(const char *query_string,
                          struct vexfs_query_plan *plan);
int vexfs_api_query_optimize(struct vexfs_api_manager *api_mgr,
                             struct vexfs_query_plan *plan);

/* Index API Operations */
int vexfs_api_index_create(struct vexfs_api_manager *api_mgr,
                           struct vexfs_api_request *request,
                           struct vexfs_api_response *response);
int vexfs_api_index_destroy(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request,
                            struct vexfs_api_response *response);
int vexfs_api_index_rebuild(struct vexfs_api_manager *api_mgr,
                            u8 index_type, const char *index_key);

/* Asynchronous Operations */
int vexfs_api_request_async(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request,
                            void (*callback)(struct vexfs_api_response *));
int vexfs_api_wait_completion(struct vexfs_api_request *request,
                              unsigned long timeout_ms);

/* Batch Operations */
int vexfs_api_batch_execute(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *requests,
                            struct vexfs_api_response *responses,
                            u32 count);

/* Performance and Monitoring */
void vexfs_api_get_statistics(struct vexfs_api_manager *api_mgr,
                              struct vexfs_api_stats *stats);
int vexfs_api_performance_report(struct vexfs_api_manager *api_mgr,
                                 char *buffer, size_t size);

/* Error Handling and Validation */
int vexfs_api_validate_request(struct vexfs_api_request *request);
void vexfs_api_set_error(struct vexfs_api_response *response,
                         int error_code, const char *message);
const char *vexfs_api_error_string(int error_code);

/* Memory Management */
struct vexfs_api_request *vexfs_api_request_alloc(struct vexfs_api_manager *api_mgr);
void vexfs_api_request_free(struct vexfs_api_manager *api_mgr,
                            struct vexfs_api_request *request);
struct vexfs_api_response *vexfs_api_response_alloc(struct vexfs_api_manager *api_mgr);
void vexfs_api_response_free(struct vexfs_api_manager *api_mgr,
                             struct vexfs_api_response *response);

/*
 * API Statistics Structure
 */
struct vexfs_api_stats {
    u64 total_requests;
    u64 successful_requests;
    u64 failed_requests;
    u64 avg_response_time_ns;
    u64 min_response_time_ns;
    u64 max_response_time_ns;
    u32 active_requests;
    u32 cache_hit_rate;
    u32 query_optimization_rate;
    u64 memory_usage;
    u32 error_counts[16];
};

#endif /* _VEXFS_V2_VEXGRAPH_API_H */