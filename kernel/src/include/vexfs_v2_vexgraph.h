/*
 * VexFS v2.0 - VexGraph Core Structure (Task 8 - Phase 2)
 * 
 * This implements the native graph representation layer that transforms VexFS
 * into a true AI-native semantic substrate. Files and directories become nodes
 * and relationships become edges in a queryable property graph.
 *
 * Phase 2 Milestone: VexGraph Implementation
 * - Property Graph Model with nodes (files/dirs) and edges (relationships)
 * - Extended inode structures for graph nodes with properties
 * - Edge representation using xattrs and symlinks
 * - Graph index structure for efficient traversal
 * - Integration with Phase 1 journaling and atomic operations
 * - Kernel-compatible graph algorithms
 * - Serialization framework for graph structures
 * - Space-efficient graph representation
 *
 * Building on Complete Phase 1 Foundation:
 * - Task 1: Full FS Journal with WAL, checksumming, crash recovery
 * - Task 2: Atomic Operations with transaction management
 * - Task 3: Metadata Journaling with integrity optimization
 * - Task 4: Configurable Data Journaling with COW
 * - Task 5: Safe Block/Inode Journaling with allocation tracking
 * - Task 6: ACID Compliance with MVCC and deadlock detection
 * - Task 7: Fast Crash Recovery with enterprise-grade reliability
 */

#ifndef _VEXFS_V2_VEXGRAPH_H
#define _VEXFS_V2_VEXGRAPH_H

#include <linux/types.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/hash.h>
#include <linux/xattr.h>
#include <linux/fs.h>

#include "vexfs_v2_journal.h"
#include "vexfs_v2_atomic.h"

/* VexGraph magic numbers and version */
#define VEXFS_VEXGRAPH_MAGIC        0x56475246  /* "VGRF" */
#define VEXFS_VEXGRAPH_VERSION_MAJOR 1
#define VEXFS_VEXGRAPH_VERSION_MINOR 0

/* Graph node types */
#define VEXFS_GRAPH_NODE_FILE       0x01
#define VEXFS_GRAPH_NODE_DIR        0x02
#define VEXFS_GRAPH_NODE_VECTOR     0x03
#define VEXFS_GRAPH_NODE_COLLECTION 0x04
#define VEXFS_GRAPH_NODE_SEMANTIC   0x05

/* Graph edge types */
#define VEXFS_GRAPH_EDGE_CONTAINS   0x01  /* Directory contains file */
#define VEXFS_GRAPH_EDGE_REFERENCES 0x02  /* File references another */
#define VEXFS_GRAPH_EDGE_SIMILAR    0x03  /* Vector similarity */
#define VEXFS_GRAPH_EDGE_SEMANTIC   0x04  /* Semantic relationship */
#define VEXFS_GRAPH_EDGE_TEMPORAL   0x05  /* Temporal relationship */
#define VEXFS_GRAPH_EDGE_CUSTOM     0x06  /* User-defined relationship */

/* Graph property types */
#define VEXFS_GRAPH_PROP_STRING     0x01
#define VEXFS_GRAPH_PROP_INTEGER    0x02
#define VEXFS_GRAPH_PROP_FLOAT      0x03
#define VEXFS_GRAPH_PROP_BOOLEAN    0x04
#define VEXFS_GRAPH_PROP_VECTOR     0x05
#define VEXFS_GRAPH_PROP_TIMESTAMP  0x06

/* Graph operation types for journaling */
#define VEXFS_GRAPH_OP_NODE_CREATE  0x01
#define VEXFS_GRAPH_OP_NODE_DELETE  0x02
#define VEXFS_GRAPH_OP_NODE_UPDATE  0x03
#define VEXFS_GRAPH_OP_EDGE_CREATE  0x04
#define VEXFS_GRAPH_OP_EDGE_DELETE  0x05
#define VEXFS_GRAPH_OP_EDGE_UPDATE  0x06
#define VEXFS_GRAPH_OP_PROP_SET     0x07
#define VEXFS_GRAPH_OP_PROP_DELETE  0x08

/* Graph index types */
#define VEXFS_GRAPH_INDEX_NODE_ID   0x01
#define VEXFS_GRAPH_INDEX_EDGE_TYPE 0x02
#define VEXFS_GRAPH_INDEX_PROPERTY  0x03
#define VEXFS_GRAPH_INDEX_SPATIAL   0x04

/* Graph traversal algorithms */
#define VEXFS_GRAPH_TRAVERSAL_BFS   0x01
#define VEXFS_GRAPH_TRAVERSAL_DFS   0x02
#define VEXFS_GRAPH_TRAVERSAL_DIJKSTRA 0x03
#define VEXFS_GRAPH_TRAVERSAL_ASTAR 0x04

/* Maximum values */
#define VEXFS_GRAPH_MAX_NODES       1000000
#define VEXFS_GRAPH_MAX_EDGES       10000000
#define VEXFS_GRAPH_MAX_PROPERTIES  256
#define VEXFS_GRAPH_MAX_PROP_SIZE   4096
#define VEXFS_GRAPH_MAX_EDGE_WEIGHT 0xFFFFFFFF

/* Graph flags */
#define VEXFS_GRAPH_FLAG_DIRECTED   0x01
#define VEXFS_GRAPH_FLAG_WEIGHTED   0x02
#define VEXFS_GRAPH_FLAG_INDEXED    0x04
#define VEXFS_GRAPH_FLAG_PERSISTENT 0x08

/*
 * Graph Property - Key-value pair with type information
 */
struct vexfs_graph_property {
    char key[64];                   /* Property key */
    u8 type;                        /* Property type */
    u32 size;                       /* Property value size */
    union {
        char *string_val;           /* String value */
        s64 int_val;                /* Integer value */
        u64 uint_val;               /* Unsigned integer value */
        u32 float_bits;             /* Float as bits (no FPU) */
        bool bool_val;              /* Boolean value */
        u64 timestamp_val;          /* Timestamp value */
        struct {
            u32 *vector_data;       /* Vector data */
            u32 dimensions;         /* Vector dimensions */
        } vector_val;
    } value;
    struct list_head list;          /* Property list */
};

/*
 * Graph Node - Extended inode structure for graph representation
 */
struct vexfs_graph_node {
    u64 node_id;                    /* Unique node identifier */
    u64 inode_number;               /* Associated inode number */
    u8 node_type;                   /* Node type */
    u32 flags;                      /* Node flags */
    
    /* Properties */
    struct list_head properties;   /* Node properties list */
    u32 property_count;             /* Number of properties */
    
    /* Adjacency information */
    struct list_head outgoing_edges; /* Outgoing edges */
    struct list_head incoming_edges; /* Incoming edges */
    u32 out_degree;                 /* Outgoing edge count */
    u32 in_degree;                  /* Incoming edge count */
    
    /* Index and caching */
    struct rb_node rb_node;         /* Red-black tree node */
    struct hlist_node hash_node;    /* Hash table node */
    
    /* Synchronization */
    struct rw_semaphore node_sem;   /* Node read-write semaphore */
    atomic_t ref_count;             /* Reference count */
    
    /* Timestamps */
    u64 created_time;               /* Creation timestamp */
    u64 modified_time;              /* Last modification timestamp */
    u64 accessed_time;              /* Last access timestamp */
};

/*
 * Graph Edge - Relationship between two nodes
 */
struct vexfs_graph_edge {
    u64 edge_id;                    /* Unique edge identifier */
    u64 source_node_id;             /* Source node ID */
    u64 target_node_id;             /* Target node ID */
    u8 edge_type;                   /* Edge type */
    u32 weight;                     /* Edge weight (integer) */
    u32 flags;                      /* Edge flags */
    
    /* Properties */
    struct list_head properties;   /* Edge properties list */
    u32 property_count;             /* Number of properties */
    
    /* List membership */
    struct list_head source_list;   /* Source node's outgoing list */
    struct list_head target_list;   /* Target node's incoming list */
    
    /* Index and caching */
    struct rb_node rb_node;         /* Red-black tree node */
    struct hlist_node hash_node;    /* Hash table node */
    
    /* Synchronization */
    spinlock_t edge_lock;           /* Edge spinlock */
    atomic_t ref_count;             /* Reference count */
    
    /* Timestamps */
    u64 created_time;               /* Creation timestamp */
    u64 modified_time;              /* Last modification timestamp */
};

/*
 * Graph Index Entry - For efficient graph queries
 */
struct vexfs_graph_index_entry {
    u8 index_type;                  /* Index type */
    union {
        u64 node_id;                /* Node ID index */
        u8 edge_type;               /* Edge type index */
        char property_key[64];      /* Property key index */
    } key;
    
    struct list_head node_list;     /* Nodes matching this index */
    struct list_head edge_list;     /* Edges matching this index */
    u32 node_count;                 /* Number of matching nodes */
    u32 edge_count;                 /* Number of matching edges */
    
    struct rb_node rb_node;         /* Red-black tree node */
    struct rw_semaphore index_sem;  /* Index read-write semaphore */
};

/*
 * Graph Manager - Central coordinator for graph operations
 */
struct vexfs_graph_manager {
    /* Graph metadata */
    u32 magic;                      /* Magic number */
    u32 version_major;              /* Major version */
    u32 version_minor;              /* Minor version */
    u32 flags;                      /* Graph flags */
    
    /* Node management */
    struct rb_root nodes_tree;      /* Red-black tree of nodes */
    struct hlist_head *nodes_hash;  /* Hash table of nodes */
    u32 nodes_hash_size;            /* Hash table size */
    atomic64_t node_count;          /* Total node count */
    atomic64_t next_node_id;        /* Next node ID */
    
    /* Edge management */
    struct rb_root edges_tree;      /* Red-black tree of edges */
    struct hlist_head *edges_hash;  /* Hash table of edges */
    u32 edges_hash_size;            /* Hash table size */
    atomic64_t edge_count;          /* Total edge count */
    atomic64_t next_edge_id;        /* Next edge ID */
    
    /* Index management */
    struct rb_root indices_tree;    /* Red-black tree of indices */
    struct list_head indices_list;  /* List of all indices */
    u32 index_count;                /* Number of indices */
    
    /* Synchronization */
    struct rw_semaphore graph_sem;  /* Graph read-write semaphore */
    spinlock_t hash_lock;           /* Hash table lock */
    struct mutex index_mutex;       /* Index operations mutex */
    
    /* Memory management */
    struct kmem_cache *node_cache;  /* Node allocation cache */
    struct kmem_cache *edge_cache;  /* Edge allocation cache */
    struct kmem_cache *prop_cache;  /* Property allocation cache */
    
    /* Statistics */
    atomic64_t operations_count;    /* Total operations */
    atomic64_t traversals_count;    /* Total traversals */
    atomic64_t queries_count;       /* Total queries */
    
    /* Integration with VexFS */
    struct super_block *sb;         /* Associated superblock */
    struct vexfs_journal *journal;  /* Journal for graph operations */
    struct vexfs_atomic_manager *atomic_mgr; /* Atomic operations */
};

/*
 * Graph Query Context - For complex graph queries
 */
struct vexfs_graph_query_context {
    u8 traversal_algorithm;         /* Traversal algorithm */
    u64 start_node_id;              /* Starting node ID */
    u64 end_node_id;                /* Target node ID (optional) */
    u32 max_depth;                  /* Maximum traversal depth */
    u32 max_results;                /* Maximum results to return */
    
    /* Filters */
    u8 node_type_filter;            /* Node type filter */
    u8 edge_type_filter;            /* Edge type filter */
    char property_filter[64];       /* Property filter */
    
    /* Results */
    u64 *result_nodes;              /* Result node IDs */
    u64 *result_edges;              /* Result edge IDs */
    u32 *result_distances;          /* Result distances */
    u32 result_count;               /* Number of results */
    
    /* State */
    bool *visited_nodes;            /* Visited nodes bitmap */
    u32 *distances;                 /* Distance array */
    struct list_head queue;         /* Traversal queue */
    
    /* Synchronization */
    struct completion query_complete; /* Query completion */
    atomic_t ref_count;             /* Reference count */
};

/*
 * Graph Serialization Header - For persistent storage
 */
struct vexfs_graph_serialization_header {
    u32 magic;                      /* Magic number */
    u32 version;                    /* Serialization version */
    u64 node_count;                 /* Number of nodes */
    u64 edge_count;                 /* Number of edges */
    u64 property_count;             /* Number of properties */
    u32 flags;                      /* Serialization flags */
    u64 checksum;                   /* Data checksum */
    u64 timestamp;                  /* Serialization timestamp */
};

/*
 * =============================================================================
 * VEXGRAPH CORE FUNCTION DECLARATIONS
 * =============================================================================
 */

/* Graph Manager Operations */
struct vexfs_graph_manager *vexfs_graph_manager_create(struct super_block *sb);
void vexfs_graph_manager_destroy(struct vexfs_graph_manager *mgr);
int vexfs_graph_manager_init(struct vexfs_graph_manager *mgr);
void vexfs_graph_manager_cleanup(struct vexfs_graph_manager *mgr);

/* Node Operations */
struct vexfs_graph_node *vexfs_graph_node_create(struct vexfs_graph_manager *mgr,
                                                  u64 inode_number, u8 node_type);
void vexfs_graph_node_destroy(struct vexfs_graph_manager *mgr,
                               struct vexfs_graph_node *node);
struct vexfs_graph_node *vexfs_graph_node_lookup(struct vexfs_graph_manager *mgr,
                                                  u64 node_id);
int vexfs_graph_node_add_property(struct vexfs_graph_node *node,
                                  const char *key, u8 type, const void *value, u32 size);
int vexfs_graph_node_remove_property(struct vexfs_graph_node *node, const char *key);
struct vexfs_graph_property *vexfs_graph_node_get_property(struct vexfs_graph_node *node,
                                                           const char *key);

/* Edge Operations */
struct vexfs_graph_edge *vexfs_graph_edge_create(struct vexfs_graph_manager *mgr,
                                                  u64 source_id, u64 target_id,
                                                  u8 edge_type, u32 weight);
void vexfs_graph_edge_destroy(struct vexfs_graph_manager *mgr,
                               struct vexfs_graph_edge *edge);
struct vexfs_graph_edge *vexfs_graph_edge_lookup(struct vexfs_graph_manager *mgr,
                                                  u64 edge_id);
int vexfs_graph_edge_add_property(struct vexfs_graph_edge *edge,
                                  const char *key, u8 type, const void *value, u32 size);
int vexfs_graph_edge_remove_property(struct vexfs_graph_edge *edge, const char *key);

/* Graph Traversal Operations */
int vexfs_graph_traverse_bfs(struct vexfs_graph_manager *mgr,
                             struct vexfs_graph_query_context *ctx);
int vexfs_graph_traverse_dfs(struct vexfs_graph_manager *mgr,
                             struct vexfs_graph_query_context *ctx);
int vexfs_graph_shortest_path(struct vexfs_graph_manager *mgr,
                              u64 source_id, u64 target_id,
                              u64 *path, u32 *path_length);

/* Graph Index Operations */
int vexfs_graph_index_create(struct vexfs_graph_manager *mgr, u8 index_type,
                             const char *key);
int vexfs_graph_index_destroy(struct vexfs_graph_manager *mgr, u8 index_type,
                              const char *key);
int vexfs_graph_index_update(struct vexfs_graph_manager *mgr,
                             struct vexfs_graph_node *node,
                             struct vexfs_graph_edge *edge);

/* Graph Query Operations */
struct vexfs_graph_query_context *vexfs_graph_query_create(struct vexfs_graph_manager *mgr);
void vexfs_graph_query_destroy(struct vexfs_graph_query_context *ctx);
int vexfs_graph_query_execute(struct vexfs_graph_manager *mgr,
                              struct vexfs_graph_query_context *ctx);

/* Graph Serialization Operations */
int vexfs_graph_serialize(struct vexfs_graph_manager *mgr, void *buffer, size_t size);
int vexfs_graph_deserialize(struct vexfs_graph_manager *mgr, const void *buffer, size_t size);
size_t vexfs_graph_calculate_serialized_size(struct vexfs_graph_manager *mgr);

/* Graph Integration with VexFS */
int vexfs_graph_inode_to_node(struct vexfs_graph_manager *mgr, struct inode *inode);
int vexfs_graph_node_to_inode(struct vexfs_graph_manager *mgr, u64 node_id,
                              struct inode **inode);
int vexfs_graph_sync_with_filesystem(struct vexfs_graph_manager *mgr);

/* Graph Statistics and Monitoring */
void vexfs_graph_get_statistics(struct vexfs_graph_manager *mgr,
                                struct vexfs_graph_stats *stats);
int vexfs_graph_validate_integrity(struct vexfs_graph_manager *mgr);

/*
 * Graph Statistics Structure
 */
struct vexfs_graph_stats {
    u64 node_count;
    u64 edge_count;
    u64 property_count;
    u64 index_count;
    u64 operations_count;
    u64 traversals_count;
    u64 queries_count;
    u64 memory_usage;
    u64 serialized_size;
};

#endif /* _VEXFS_V2_VEXGRAPH_H */