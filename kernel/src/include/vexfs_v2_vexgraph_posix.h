/*
 * VexFS v2.0 - VexGraph POSIX Integration Layer (Task 10 - Phase 2)
 * 
 * This implements seamless integration between VexGraph operations and
 * traditional POSIX filesystem operations, creating a unified interface
 * where files/directories can be both traditional filesystem objects
 * and graph nodes simultaneously.
 *
 * Phase 2 Completion: VexGraph-POSIX Seamless Operation
 * - POSIX Layer Extension for graph operations
 * - Node/File Mapping between graph nodes/edges and files/directories
 * - View Consistency between graph view and filesystem view
 * - Operation Optimization for transitions between graph and filesystem
 * - Locking Mechanisms to prevent conflicts
 * - POSIX Extension for graph operations
 * - ioctl Implementation for graph-specific operations
 * - Backwards Compatibility with standard POSIX operations
 *
 * Building on Complete VexGraph Foundation (Tasks 8-9):
 * - VexGraph Core Structure with complete property graph implementation
 * - VexGraph API with comprehensive interface layer
 * - Phase 1 Complete journaling infrastructure
 * - All components integrated into VexFS kernel module
 */

#ifndef _VEXFS_V2_VEXGRAPH_POSIX_H
#define _VEXFS_V2_VEXGRAPH_POSIX_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/dcache.h>
#include <linux/namei.h>
#include <linux/xattr.h>
#include <linux/mutex.h>
#include <linux/rwsem.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>

#include "vexfs_v2_vexgraph_api.h"
#include "vexfs_v2_enhanced_ioctl.h"

/* VexGraph-POSIX Integration magic numbers and version */
#define VEXFS_VEXGRAPH_POSIX_MAGIC      0x56475058  /* "VGPX" */
#define VEXFS_VEXGRAPH_POSIX_VERSION    1

/* POSIX Integration operation types */
#define VEXFS_POSIX_OP_CREATE           0x01
#define VEXFS_POSIX_OP_UNLINK           0x02
#define VEXFS_POSIX_OP_RENAME           0x03
#define VEXFS_POSIX_OP_OPEN             0x04
#define VEXFS_POSIX_OP_CLOSE            0x05
#define VEXFS_POSIX_OP_READ             0x06
#define VEXFS_POSIX_OP_WRITE            0x07
#define VEXFS_POSIX_OP_MKDIR            0x08
#define VEXFS_POSIX_OP_RMDIR            0x09
#define VEXFS_POSIX_OP_SYMLINK          0x0A

/* Graph-aware POSIX flags */
#define VEXFS_POSIX_FLAG_GRAPH_AWARE    0x01
#define VEXFS_POSIX_FLAG_AUTO_NODE      0x02
#define VEXFS_POSIX_FLAG_PRESERVE_EDGES 0x04
#define VEXFS_POSIX_FLAG_SYNC_METADATA  0x08
#define VEXFS_POSIX_FLAG_TRACK_ACCESS   0x10

/* Extended attribute names for graph metadata */
#define VEXFS_XATTR_GRAPH_NODE_ID       "user.vexfs.graph.node_id"
#define VEXFS_XATTR_GRAPH_NODE_TYPE     "user.vexfs.graph.node_type"
#define VEXFS_XATTR_GRAPH_PROPERTIES    "user.vexfs.graph.properties"
#define VEXFS_XATTR_GRAPH_EDGES         "user.vexfs.graph.edges"
#define VEXFS_XATTR_GRAPH_METADATA      "user.vexfs.graph.metadata"

/* ioctl commands for VexGraph-POSIX operations */
#define VEXFS_IOC_GRAPH_CREATE_NODE     _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 0x20, struct vexfs_posix_graph_node_request)
#define VEXFS_IOC_GRAPH_DELETE_NODE     _IOW(VEXFS_ENHANCED_IOC_MAGIC, 0x21, struct vexfs_posix_graph_node_request)
#define VEXFS_IOC_GRAPH_CREATE_EDGE     _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 0x22, struct vexfs_posix_graph_edge_request)
#define VEXFS_IOC_GRAPH_DELETE_EDGE     _IOW(VEXFS_ENHANCED_IOC_MAGIC, 0x23, struct vexfs_posix_graph_edge_request)
#define VEXFS_IOC_GRAPH_QUERY_NODE      _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 0x24, struct vexfs_posix_graph_query_request)
#define VEXFS_IOC_GRAPH_TRAVERSE        _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 0x25, struct vexfs_posix_graph_traversal_request)
#define VEXFS_IOC_GRAPH_SET_PROPERTY    _IOW(VEXFS_ENHANCED_IOC_MAGIC, 0x26, struct vexfs_posix_graph_property_request)
#define VEXFS_IOC_GRAPH_GET_PROPERTY    _IOWR(VEXFS_ENHANCED_IOC_MAGIC, 0x27, struct vexfs_posix_graph_property_request)
#define VEXFS_IOC_GRAPH_SYNC_VIEW       _IOW(VEXFS_ENHANCED_IOC_MAGIC, 0x28, struct vexfs_posix_graph_sync_request)

/* Maximum values for POSIX integration */
#define VEXFS_POSIX_MAX_PATH_DEPTH      256
#define VEXFS_POSIX_MAX_PROPERTY_SIZE   4096
#define VEXFS_POSIX_MAX_EDGES_PER_NODE  1000
#define VEXFS_POSIX_MAX_CONCURRENT_OPS  128

/*
 * POSIX Integration Manager - Central coordinator for filesystem-graph operations
 */
struct vexfs_posix_integration_manager {
    /* Core components */
    struct vexfs_api_manager *api_manager;
    struct super_block *sb;
    
    /* Node-File mapping */
    struct rb_root node_file_map;           /* Maps graph nodes to filesystem objects */
    struct rb_root file_node_map;           /* Maps filesystem objects to graph nodes */
    struct rw_semaphore mapping_lock;       /* Protects mapping structures */
    
    /* View consistency */
    struct mutex consistency_lock;          /* Ensures view consistency */
    atomic64_t view_version;                /* Version counter for consistency */
    struct workqueue_struct *sync_workqueue; /* Async consistency operations */
    
    /* Operation coordination */
    struct rw_semaphore operation_lock;     /* Coordinates mixed operations */
    atomic_t active_posix_ops;              /* Count of active POSIX operations */
    atomic_t active_graph_ops;              /* Count of active graph operations */
    
    /* Performance monitoring */
    atomic64_t posix_operations;            /* Total POSIX operations */
    atomic64_t graph_operations;            /* Total graph operations */
    atomic64_t mixed_operations;            /* Operations using both views */
    atomic64_t consistency_checks;          /* Consistency validation operations */
    
    /* Memory management */
    struct kmem_cache *node_mapping_cache;  /* Cache for node mappings */
    struct kmem_cache *sync_request_cache;  /* Cache for sync requests */
    
    /* Configuration */
    u32 flags;                              /* Integration flags */
    u32 auto_sync_threshold;                /* Threshold for automatic sync */
    u32 consistency_check_interval;        /* Interval for consistency checks */
};

/*
 * Node-File Mapping Entry
 */
struct vexfs_node_file_mapping {
    struct rb_node rb_node;                 /* Red-black tree node */
    u64 graph_node_id;                      /* Graph node ID */
    struct inode *inode;                    /* Associated inode */
    struct dentry *dentry;                  /* Associated dentry (if available) */
    u32 node_type;                          /* Graph node type */
    u64 last_sync_version;                  /* Last synchronization version */
    atomic_t ref_count;                     /* Reference count */
    struct mutex mapping_mutex;             /* Protects this mapping */
};

/*
 * POSIX-Graph Operation Context
 */
struct vexfs_posix_graph_context {
    struct vexfs_posix_integration_manager *manager;
    struct inode *inode;
    struct dentry *dentry;
    struct vexfs_node_file_mapping *mapping;
    u32 operation_type;
    u32 flags;
    u64 operation_id;
    struct completion completion;
};

/*
 * ioctl Request Structures for VexGraph-POSIX Operations
 */

/* Graph node creation/deletion through POSIX */
struct vexfs_posix_graph_node_request {
    char path[PATH_MAX];                    /* Filesystem path */
    u32 node_type;                          /* Graph node type */
    char properties_json[VEXFS_POSIX_MAX_PROPERTY_SIZE]; /* Node properties */
    u64 node_id;                            /* Graph node ID (output for create) */
    u32 flags;                              /* Operation flags */
};

/* Graph edge creation/deletion through POSIX */
struct vexfs_posix_graph_edge_request {
    char source_path[PATH_MAX];             /* Source file path */
    char target_path[PATH_MAX];             /* Target file path */
    u32 edge_type;                          /* Edge type */
    u32 weight;                             /* Edge weight */
    char properties_json[VEXFS_POSIX_MAX_PROPERTY_SIZE]; /* Edge properties */
    u64 edge_id;                            /* Edge ID (output for create) */
    u32 flags;                              /* Operation flags */
};

/* Graph query through POSIX paths */
struct vexfs_posix_graph_query_request {
    char query_vql[1024];                   /* VQL query string */
    char base_path[PATH_MAX];               /* Base path for relative queries */
    char results_json[8192];                /* Query results (output) */
    u32 max_results;                        /* Maximum number of results */
    u32 result_count;                       /* Actual number of results (output) */
    u32 flags;                              /* Query flags */
};

/* Graph traversal through POSIX paths */
struct vexfs_posix_graph_traversal_request {
    char start_path[PATH_MAX];              /* Starting file path */
    u32 algorithm;                          /* Traversal algorithm */
    u32 max_depth;                          /* Maximum traversal depth */
    char filter_json[1024];                 /* Traversal filters */
    char results_paths[8192];               /* Result paths (output) */
    u32 max_results;                        /* Maximum number of results */
    u32 result_count;                       /* Actual number of results (output) */
    u32 flags;                              /* Traversal flags */
};

/* Property management through POSIX */
struct vexfs_posix_graph_property_request {
    char path[PATH_MAX];                    /* File path */
    char property_name[256];                /* Property name */
    char property_value[VEXFS_POSIX_MAX_PROPERTY_SIZE]; /* Property value */
    u32 property_type;                      /* Property type */
    u32 flags;                              /* Operation flags */
};

/* View synchronization request */
struct vexfs_posix_graph_sync_request {
    char path[PATH_MAX];                    /* Path to synchronize (empty = all) */
    u32 sync_direction;                     /* Sync direction (graph->fs, fs->graph, both) */
    u32 flags;                              /* Sync flags */
    u64 sync_version;                       /* Sync version (output) */
};

/*
 * Extended VFS Operations with Graph Integration
 */
struct vexfs_graph_aware_file_operations {
    const struct file_operations *base_ops; /* Base file operations */
    
    /* Graph-aware operations */
    ssize_t (*graph_read)(struct file *file, char __user *buf, size_t count, loff_t *ppos);
    ssize_t (*graph_write)(struct file *file, const char __user *buf, size_t count, loff_t *ppos);
    long (*graph_ioctl)(struct file *file, unsigned int cmd, unsigned long arg);
    int (*graph_open)(struct inode *inode, struct file *file);
    int (*graph_release)(struct inode *inode, struct file *file);
    
    /* Consistency operations */
    int (*sync_graph_view)(struct file *file);
    int (*validate_consistency)(struct file *file);
};

struct vexfs_graph_aware_inode_operations {
    const struct inode_operations *base_ops; /* Base inode operations */
    
    /* Graph-aware operations */
    int (*graph_create)(struct user_namespace *mnt_userns, struct inode *dir,
                       struct dentry *dentry, umode_t mode, bool excl);
    int (*graph_unlink)(struct inode *dir, struct dentry *dentry);
    int (*graph_rename)(struct user_namespace *mnt_userns, struct inode *old_dir,
                       struct dentry *old_dentry, struct inode *new_dir,
                       struct dentry *new_dentry, unsigned int flags);
    int (*graph_mkdir)(struct user_namespace *mnt_userns, struct inode *dir,
                      struct dentry *dentry, umode_t mode);
    int (*graph_rmdir)(struct inode *dir, struct dentry *dentry);
    
    /* Extended attribute operations for graph metadata */
    ssize_t (*graph_getxattr)(struct dentry *dentry, struct inode *inode,
                             const char *name, void *buffer, size_t size);
    int (*graph_setxattr)(struct dentry *dentry, struct inode *inode,
                         const char *name, const void *value, size_t size, int flags);
    ssize_t (*graph_listxattr)(struct dentry *dentry, char *buffer, size_t size);
    int (*graph_removexattr)(struct dentry *dentry, const char *name);
};

/*
 * Function Declarations
 */

/* POSIX Integration Manager */
struct vexfs_posix_integration_manager *vexfs_posix_integration_manager_create(
    struct super_block *sb, struct vexfs_api_manager *api_manager);
void vexfs_posix_integration_manager_destroy(struct vexfs_posix_integration_manager *manager);
int vexfs_posix_integration_manager_init(struct vexfs_posix_integration_manager *manager);
void vexfs_posix_integration_manager_cleanup(struct vexfs_posix_integration_manager *manager);

/* Node-File Mapping */
int vexfs_posix_create_node_mapping(struct vexfs_posix_integration_manager *manager,
                                   struct inode *inode, u64 graph_node_id, u32 node_type);
int vexfs_posix_remove_node_mapping(struct vexfs_posix_integration_manager *manager,
                                   struct inode *inode);
struct vexfs_node_file_mapping *vexfs_posix_find_mapping_by_inode(
    struct vexfs_posix_integration_manager *manager, struct inode *inode);
struct vexfs_node_file_mapping *vexfs_posix_find_mapping_by_node_id(
    struct vexfs_posix_integration_manager *manager, u64 graph_node_id);

/* VFS Hook Implementation */
int vexfs_posix_hook_create(struct user_namespace *mnt_userns, struct inode *dir,
                           struct dentry *dentry, umode_t mode, bool excl);
int vexfs_posix_hook_unlink(struct inode *dir, struct dentry *dentry);
int vexfs_posix_hook_rename(struct user_namespace *mnt_userns, struct inode *old_dir,
                           struct dentry *old_dentry, struct inode *new_dir,
                           struct dentry *new_dentry, unsigned int flags);
int vexfs_posix_hook_mkdir(struct user_namespace *mnt_userns, struct inode *dir,
                          struct dentry *dentry, umode_t mode);
int vexfs_posix_hook_rmdir(struct inode *dir, struct dentry *dentry);

/* File Operations Integration */
ssize_t vexfs_posix_graph_read(struct file *file, char __user *buf, size_t count, loff_t *ppos);
ssize_t vexfs_posix_graph_write(struct file *file, const char __user *buf, size_t count, loff_t *ppos);
long vexfs_posix_graph_ioctl(struct file *file, unsigned int cmd, unsigned long arg);
int vexfs_posix_graph_open(struct inode *inode, struct file *file);
int vexfs_posix_graph_release(struct inode *inode, struct file *file);

/* Extended Attributes for Graph Metadata */
ssize_t vexfs_posix_graph_getxattr(struct dentry *dentry, struct inode *inode,
                                  const char *name, void *buffer, size_t size);
int vexfs_posix_graph_setxattr(struct dentry *dentry, struct inode *inode,
                              const char *name, const void *value, size_t size, int flags);
ssize_t vexfs_posix_graph_listxattr(struct dentry *dentry, char *buffer, size_t size);
int vexfs_posix_graph_removexattr(struct dentry *dentry, const char *name);

/* View Consistency Management */
int vexfs_posix_sync_graph_to_filesystem(struct vexfs_posix_integration_manager *manager,
                                        struct vexfs_node_file_mapping *mapping);
int vexfs_posix_sync_filesystem_to_graph(struct vexfs_posix_integration_manager *manager,
                                        struct vexfs_node_file_mapping *mapping);
int vexfs_posix_validate_view_consistency(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_node_file_mapping *mapping);
int vexfs_posix_schedule_consistency_check(struct vexfs_posix_integration_manager *manager);

/* ioctl Interface Implementation */
long vexfs_posix_ioctl_graph_create_node(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_node_request __user *arg);
long vexfs_posix_ioctl_graph_delete_node(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_node_request __user *arg);
long vexfs_posix_ioctl_graph_create_edge(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_edge_request __user *arg);
long vexfs_posix_ioctl_graph_delete_edge(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_edge_request __user *arg);
long vexfs_posix_ioctl_graph_query(struct vexfs_posix_integration_manager *manager,
                                  struct vexfs_posix_graph_query_request __user *arg);
long vexfs_posix_ioctl_graph_traverse(struct vexfs_posix_integration_manager *manager,
                                     struct vexfs_posix_graph_traversal_request __user *arg);

/* Locking and Coordination */
int vexfs_posix_acquire_operation_lock(struct vexfs_posix_integration_manager *manager,
                                      u32 operation_type, bool exclusive);
void vexfs_posix_release_operation_lock(struct vexfs_posix_integration_manager *manager,
                                       u32 operation_type, bool exclusive);
int vexfs_posix_wait_for_consistency(struct vexfs_posix_integration_manager *manager,
                                    u64 required_version);

/* Performance and Statistics */
void vexfs_posix_update_operation_stats(struct vexfs_posix_integration_manager *manager,
                                       u32 operation_type, bool mixed_operation);
int vexfs_posix_get_performance_stats(struct vexfs_posix_integration_manager *manager,
                                     struct vexfs_posix_performance_stats *stats);

/* Utility Functions */
int vexfs_posix_path_to_node_id(struct vexfs_posix_integration_manager *manager,
                               const char *path, u64 *node_id);
int vexfs_posix_node_id_to_path(struct vexfs_posix_integration_manager *manager,
                               u64 node_id, char *path, size_t path_size);
bool vexfs_posix_is_graph_aware_inode(struct inode *inode);
int vexfs_posix_enable_graph_awareness(struct inode *inode);
int vexfs_posix_disable_graph_awareness(struct inode *inode);

/* Global integration manager instance */
extern struct vexfs_posix_integration_manager *vexfs_global_posix_manager;

#endif /* _VEXFS_V2_VEXGRAPH_POSIX_H */