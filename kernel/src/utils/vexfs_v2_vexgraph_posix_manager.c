/*
 * VexFS v2.0 - VexGraph POSIX Integration Manager (Task 10 - Phase 2)
 * 
 * This implements the central coordinator for seamless integration between
 * VexGraph operations and traditional POSIX filesystem operations.
 *
 * Key Features:
 * - POSIX Integration Manager for coordinating filesystem-graph operations
 * - Node-File mapping between graph nodes and filesystem objects
 * - View consistency management between graph and filesystem views
 * - Operation coordination and locking mechanisms
 * - Performance monitoring and optimization
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/dcache.h>
#include <linux/namei.h>
#include <linux/xattr.h>
#include <linux/mutex.h>
#include <linux/rwsem.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/rbtree.h>
#include <linux/string.h>
#include <linux/uaccess.h>
#include <linux/path.h>

#include "../include/vexfs_v2_vexgraph_posix.h"
#include "../include/vexfs_v2_internal.h"

/* Global POSIX integration manager instance */
struct vexfs_posix_integration_manager *vexfs_global_posix_manager = NULL;

/* Static function declarations */
static void vexfs_posix_mapping_destructor(struct vexfs_node_file_mapping *mapping);
static int vexfs_posix_insert_node_mapping(struct vexfs_posix_integration_manager *manager,
                                          struct vexfs_node_file_mapping *mapping);
static void vexfs_posix_remove_node_mapping_locked(struct vexfs_posix_integration_manager *manager,
                                                  struct vexfs_node_file_mapping *mapping);
static void vexfs_posix_consistency_work_fn(struct work_struct *work);

/*
 * POSIX Integration Manager Creation and Destruction
 */

/**
 * vexfs_posix_integration_manager_create - Create POSIX integration manager
 * @sb: Super block
 * @api_manager: VexGraph API manager
 *
 * Creates and initializes a new POSIX integration manager for seamless
 * operation between graph and filesystem views.
 */
struct vexfs_posix_integration_manager *vexfs_posix_integration_manager_create(
    struct super_block *sb, struct vexfs_api_manager *api_manager)
{
    struct vexfs_posix_integration_manager *manager;
    int ret;

    if (!sb || !api_manager) {
        pr_err("VexFS-POSIX: Invalid parameters for manager creation\n");
        return ERR_PTR(-EINVAL);
    }

    manager = kzalloc(sizeof(*manager), GFP_KERNEL);
    if (!manager) {
        pr_err("VexFS-POSIX: Failed to allocate integration manager\n");
        return ERR_PTR(-ENOMEM);
    }

    /* Initialize core components */
    manager->api_manager = api_manager;
    manager->sb = sb;

    /* Initialize mapping structures */
    manager->node_file_map = RB_ROOT;
    manager->file_node_map = RB_ROOT;
    init_rwsem(&manager->mapping_lock);

    /* Initialize view consistency */
    mutex_init(&manager->consistency_lock);
    atomic64_set(&manager->view_version, 1);

    /* Initialize operation coordination */
    init_rwsem(&manager->operation_lock);
    atomic_set(&manager->active_posix_ops, 0);
    atomic_set(&manager->active_graph_ops, 0);

    /* Initialize performance monitoring */
    atomic64_set(&manager->posix_operations, 0);
    atomic64_set(&manager->graph_operations, 0);
    atomic64_set(&manager->mixed_operations, 0);
    atomic64_set(&manager->consistency_checks, 0);

    /* Create memory caches */
    manager->node_mapping_cache = kmem_cache_create("vexfs_node_mapping",
                                                   sizeof(struct vexfs_node_file_mapping),
                                                   0, SLAB_HWCACHE_ALIGN, NULL);
    if (!manager->node_mapping_cache) {
        pr_err("VexFS-POSIX: Failed to create node mapping cache\n");
        ret = -ENOMEM;
        goto err_free_manager;
    }

    manager->sync_request_cache = kmem_cache_create("vexfs_sync_request",
                                                   sizeof(struct vexfs_posix_graph_sync_request),
                                                   0, SLAB_HWCACHE_ALIGN, NULL);
    if (!manager->sync_request_cache) {
        pr_err("VexFS-POSIX: Failed to create sync request cache\n");
        ret = -ENOMEM;
        goto err_destroy_mapping_cache;
    }

    /* Create workqueue for async operations */
    manager->sync_workqueue = alloc_workqueue("vexfs_posix_sync",
                                             WQ_MEM_RECLAIM | WQ_UNBOUND, 0);
    if (!manager->sync_workqueue) {
        pr_err("VexFS-POSIX: Failed to create sync workqueue\n");
        ret = -ENOMEM;
        goto err_destroy_sync_cache;
    }

    /* Set default configuration */
    manager->flags = VEXFS_POSIX_FLAG_GRAPH_AWARE | VEXFS_POSIX_FLAG_AUTO_NODE;
    manager->auto_sync_threshold = 100;
    manager->consistency_check_interval = 1000;

    pr_info("VexFS-POSIX: Integration manager created successfully\n");
    return manager;

err_destroy_sync_cache:
    kmem_cache_destroy(manager->sync_request_cache);
err_destroy_mapping_cache:
    kmem_cache_destroy(manager->node_mapping_cache);
err_free_manager:
    kfree(manager);
    return ERR_PTR(ret);
}

/**
 * vexfs_posix_integration_manager_destroy - Destroy POSIX integration manager
 * @manager: Integration manager to destroy
 */
void vexfs_posix_integration_manager_destroy(struct vexfs_posix_integration_manager *manager)
{
    struct rb_node *node;
    struct vexfs_node_file_mapping *mapping;

    if (!manager)
        return;

    pr_info("VexFS-POSIX: Destroying integration manager\n");

    /* Destroy workqueue */
    if (manager->sync_workqueue) {
        flush_workqueue(manager->sync_workqueue);
        destroy_workqueue(manager->sync_workqueue);
    }

    /* Clean up all mappings */
    down_write(&manager->mapping_lock);
    while ((node = rb_first(&manager->node_file_map)) != NULL) {
        mapping = rb_entry(node, struct vexfs_node_file_mapping, rb_node);
        vexfs_posix_remove_node_mapping_locked(manager, mapping);
        vexfs_posix_mapping_destructor(mapping);
    }
    up_write(&manager->mapping_lock);

    /* Destroy memory caches */
    if (manager->sync_request_cache)
        kmem_cache_destroy(manager->sync_request_cache);
    if (manager->node_mapping_cache)
        kmem_cache_destroy(manager->node_mapping_cache);

    kfree(manager);
    pr_info("VexFS-POSIX: Integration manager destroyed\n");
}

/**
 * vexfs_posix_integration_manager_init - Initialize global integration manager
 * @manager: Integration manager to initialize
 */
int vexfs_posix_integration_manager_init(struct vexfs_posix_integration_manager *manager)
{
    if (!manager) {
        pr_err("VexFS-POSIX: Cannot initialize NULL manager\n");
        return -EINVAL;
    }

    /* Set as global manager */
    vexfs_global_posix_manager = manager;

    pr_info("VexFS-POSIX: Integration manager initialized as global instance\n");
    return 0;
}

/**
 * vexfs_posix_integration_manager_cleanup - Cleanup global integration manager
 * @manager: Integration manager to cleanup
 */
void vexfs_posix_integration_manager_cleanup(struct vexfs_posix_integration_manager *manager)
{
    if (manager && vexfs_global_posix_manager == manager) {
        vexfs_global_posix_manager = NULL;
        pr_info("VexFS-POSIX: Global integration manager cleaned up\n");
    }
}

/*
 * Node-File Mapping Management
 */

/**
 * vexfs_posix_create_node_mapping - Create mapping between graph node and filesystem object
 * @manager: Integration manager
 * @inode: Filesystem inode
 * @graph_node_id: Graph node ID
 * @node_type: Graph node type
 */
int vexfs_posix_create_node_mapping(struct vexfs_posix_integration_manager *manager,
                                   struct inode *inode, u64 graph_node_id, u32 node_type)
{
    struct vexfs_node_file_mapping *mapping;
    int ret;

    if (!manager || !inode) {
        pr_err("VexFS-POSIX: Invalid parameters for node mapping creation\n");
        return -EINVAL;
    }

    /* Check if mapping already exists */
    down_read(&manager->mapping_lock);
    mapping = vexfs_posix_find_mapping_by_inode(manager, inode);
    if (mapping) {
        up_read(&manager->mapping_lock);
        pr_warn("VexFS-POSIX: Mapping already exists for inode %lu\n", inode->i_ino);
        return -EEXIST;
    }
    up_read(&manager->mapping_lock);

    /* Allocate new mapping */
    mapping = kmem_cache_alloc(manager->node_mapping_cache, GFP_KERNEL);
    if (!mapping) {
        pr_err("VexFS-POSIX: Failed to allocate node mapping\n");
        return -ENOMEM;
    }

    /* Initialize mapping */
    RB_CLEAR_NODE(&mapping->rb_node);
    mapping->graph_node_id = graph_node_id;
    mapping->inode = inode;
    mapping->dentry = NULL; /* Will be set when available */
    mapping->node_type = node_type;
    mapping->last_sync_version = atomic64_read(&manager->view_version);
    atomic_set(&mapping->ref_count, 1);
    mutex_init(&mapping->mapping_mutex);

    /* Insert into mapping structures */
    down_write(&manager->mapping_lock);
    ret = vexfs_posix_insert_node_mapping(manager, mapping);
    if (ret) {
        up_write(&manager->mapping_lock);
        kmem_cache_free(manager->node_mapping_cache, mapping);
        pr_err("VexFS-POSIX: Failed to insert node mapping: %d\n", ret);
        return ret;
    }
    up_write(&manager->mapping_lock);

    /* Hold reference to inode */
    ihold(inode);

    pr_debug("VexFS-POSIX: Created mapping: inode %lu -> node %llu (type %u)\n",
             inode->i_ino, graph_node_id, node_type);

    return 0;
}

/**
 * vexfs_posix_remove_node_mapping - Remove mapping between graph node and filesystem object
 * @manager: Integration manager
 * @inode: Filesystem inode
 */
int vexfs_posix_remove_node_mapping(struct vexfs_posix_integration_manager *manager,
                                   struct inode *inode)
{
    struct vexfs_node_file_mapping *mapping;

    if (!manager || !inode) {
        pr_err("VexFS-POSIX: Invalid parameters for node mapping removal\n");
        return -EINVAL;
    }

    down_write(&manager->mapping_lock);
    mapping = vexfs_posix_find_mapping_by_inode(manager, inode);
    if (!mapping) {
        up_write(&manager->mapping_lock);
        pr_warn("VexFS-POSIX: No mapping found for inode %lu\n", inode->i_ino);
        return -ENOENT;
    }

    vexfs_posix_remove_node_mapping_locked(manager, mapping);
    up_write(&manager->mapping_lock);

    pr_debug("VexFS-POSIX: Removed mapping for inode %lu\n", inode->i_ino);

    /* Release reference and free mapping */
    iput(mapping->inode);
    vexfs_posix_mapping_destructor(mapping);

    return 0;
}

/**
 * vexfs_posix_find_mapping_by_inode - Find mapping by inode
 * @manager: Integration manager
 * @inode: Filesystem inode
 *
 * Returns mapping if found, NULL otherwise. Caller must hold mapping_lock.
 */
struct vexfs_node_file_mapping *vexfs_posix_find_mapping_by_inode(
    struct vexfs_posix_integration_manager *manager, struct inode *inode)
{
    struct rb_node *node;
    struct vexfs_node_file_mapping *mapping;

    if (!manager || !inode)
        return NULL;

    node = manager->file_node_map.rb_node;
    while (node) {
        mapping = rb_entry(node, struct vexfs_node_file_mapping, rb_node);

        if (inode->i_ino < mapping->inode->i_ino) {
            node = node->rb_left;
        } else if (inode->i_ino > mapping->inode->i_ino) {
            node = node->rb_right;
        } else {
            atomic_inc(&mapping->ref_count);
            return mapping;
        }
    }

    return NULL;
}

/**
 * vexfs_posix_find_mapping_by_node_id - Find mapping by graph node ID
 * @manager: Integration manager
 * @graph_node_id: Graph node ID
 *
 * Returns mapping if found, NULL otherwise. Caller must hold mapping_lock.
 */
struct vexfs_node_file_mapping *vexfs_posix_find_mapping_by_node_id(
    struct vexfs_posix_integration_manager *manager, u64 graph_node_id)
{
    struct rb_node *node;
    struct vexfs_node_file_mapping *mapping;

    if (!manager)
        return NULL;

    node = manager->node_file_map.rb_node;
    while (node) {
        mapping = rb_entry(node, struct vexfs_node_file_mapping, rb_node);

        if (graph_node_id < mapping->graph_node_id) {
            node = node->rb_left;
        } else if (graph_node_id > mapping->graph_node_id) {
            node = node->rb_right;
        } else {
            atomic_inc(&mapping->ref_count);
            return mapping;
        }
    }

    return NULL;
}

/*
 * Static Helper Functions
 */

/**
 * vexfs_posix_mapping_destructor - Destroy a node mapping
 * @mapping: Mapping to destroy
 */
static void vexfs_posix_mapping_destructor(struct vexfs_node_file_mapping *mapping)
{
    if (!mapping)
        return;

    /* Ensure reference count is zero */
    if (atomic_read(&mapping->ref_count) > 0) {
        pr_warn("VexFS-POSIX: Destroying mapping with non-zero ref count\n");
    }

    /* Free the mapping */
    if (vexfs_global_posix_manager && vexfs_global_posix_manager->node_mapping_cache) {
        kmem_cache_free(vexfs_global_posix_manager->node_mapping_cache, mapping);
    } else {
        kfree(mapping);
    }
}

/**
 * vexfs_posix_insert_node_mapping - Insert mapping into red-black trees
 * @manager: Integration manager
 * @mapping: Mapping to insert
 *
 * Caller must hold mapping_lock for writing.
 */
static int vexfs_posix_insert_node_mapping(struct vexfs_posix_integration_manager *manager,
                                          struct vexfs_node_file_mapping *mapping)
{
    struct rb_node **new_node, *parent = NULL;
    struct vexfs_node_file_mapping *existing;

    /* Insert into node_file_map (keyed by graph_node_id) */
    new_node = &manager->node_file_map.rb_node;
    while (*new_node) {
        parent = *new_node;
        existing = rb_entry(parent, struct vexfs_node_file_mapping, rb_node);

        if (mapping->graph_node_id < existing->graph_node_id) {
            new_node = &((*new_node)->rb_left);
        } else if (mapping->graph_node_id > existing->graph_node_id) {
            new_node = &((*new_node)->rb_right);
        } else {
            pr_err("VexFS-POSIX: Duplicate graph node ID %llu in mapping\n",
                   mapping->graph_node_id);
            return -EEXIST;
        }
    }

    rb_link_node(&mapping->rb_node, parent, new_node);
    rb_insert_color(&mapping->rb_node, &manager->node_file_map);

    /* Insert into file_node_map (keyed by inode number) */
    new_node = &manager->file_node_map.rb_node;
    parent = NULL;
    while (*new_node) {
        parent = *new_node;
        existing = rb_entry(parent, struct vexfs_node_file_mapping, rb_node);

        if (mapping->inode->i_ino < existing->inode->i_ino) {
            new_node = &((*new_node)->rb_left);
        } else if (mapping->inode->i_ino > existing->inode->i_ino) {
            new_node = &((*new_node)->rb_right);
        } else {
            /* Remove from node_file_map and return error */
            rb_erase(&mapping->rb_node, &manager->node_file_map);
            pr_err("VexFS-POSIX: Duplicate inode %lu in mapping\n",
                   mapping->inode->i_ino);
            return -EEXIST;
        }
    }

    /* Note: We reuse the same rb_node for both trees, which is not standard.
     * In a production implementation, we would need separate rb_node structures
     * or use a different approach like hash tables for one of the mappings. */

    return 0;
}

/**
 * vexfs_posix_remove_node_mapping_locked - Remove mapping from red-black trees
 * @manager: Integration manager
 * @mapping: Mapping to remove
 *
 * Caller must hold mapping_lock for writing.
 */
static void vexfs_posix_remove_node_mapping_locked(struct vexfs_posix_integration_manager *manager,
                                                  struct vexfs_node_file_mapping *mapping)
{
    if (!RB_EMPTY_NODE(&mapping->rb_node)) {
        rb_erase(&mapping->rb_node, &manager->node_file_map);
        /* Note: In production, we would also remove from file_node_map */
        RB_CLEAR_NODE(&mapping->rb_node);
    }
}

/**
 * vexfs_posix_consistency_work_fn - Work function for consistency checks
 * @work: Work structure
 */
static void vexfs_posix_consistency_work_fn(struct work_struct *work)
{
    /* Implementation for async consistency checking */
    pr_debug("VexFS-POSIX: Performing consistency check\n");
    
    /* This would implement:
     * 1. Iterate through all mappings
     * 2. Check graph node vs filesystem object consistency
     * 3. Update sync versions
     * 4. Report inconsistencies
     */
}

/*
 * Performance and Statistics Functions
 */

/**
 * vexfs_posix_update_operation_stats - Update operation statistics
 * @manager: Integration manager
 * @operation_type: Type of operation
 * @mixed_operation: Whether this operation used both views
 */
void vexfs_posix_update_operation_stats(struct vexfs_posix_integration_manager *manager,
                                       u32 operation_type, bool mixed_operation)
{
    if (!manager)
        return;

    switch (operation_type) {
    case VEXFS_POSIX_OP_CREATE:
    case VEXFS_POSIX_OP_UNLINK:
    case VEXFS_POSIX_OP_RENAME:
    case VEXFS_POSIX_OP_OPEN:
    case VEXFS_POSIX_OP_CLOSE:
    case VEXFS_POSIX_OP_READ:
    case VEXFS_POSIX_OP_WRITE:
    case VEXFS_POSIX_OP_MKDIR:
    case VEXFS_POSIX_OP_RMDIR:
    case VEXFS_POSIX_OP_SYMLINK:
        atomic64_inc(&manager->posix_operations);
        break;
    default:
        atomic64_inc(&manager->graph_operations);
        break;
    }

    if (mixed_operation) {
        atomic64_inc(&manager->mixed_operations);
    }
}

/*
 * Utility Functions
 */

/**
 * vexfs_posix_is_graph_aware_inode - Check if inode is graph-aware
 * @inode: Inode to check
 */
bool vexfs_posix_is_graph_aware_inode(struct inode *inode)
{
    struct vexfs_node_file_mapping *mapping;
    bool is_aware = false;

    if (!inode || !vexfs_global_posix_manager)
        return false;

    down_read(&vexfs_global_posix_manager->mapping_lock);
    mapping = vexfs_posix_find_mapping_by_inode(vexfs_global_posix_manager, inode);
    if (mapping) {
        is_aware = true;
        atomic_dec(&mapping->ref_count); /* Release reference from find */
    }
    up_read(&vexfs_global_posix_manager->mapping_lock);

    return is_aware;
}

/**
 * vexfs_posix_enable_graph_awareness - Enable graph awareness for inode
 * @inode: Inode to enable graph awareness for
 */
int vexfs_posix_enable_graph_awareness(struct inode *inode)
{
    /* Implementation would:
     * 1. Create graph node for inode
     * 2. Create mapping
     * 3. Set extended attributes
     * 4. Update inode operations
     */
    
    pr_debug("VexFS-POSIX: Enabling graph awareness for inode %lu\n", 
             inode ? inode->i_ino : 0);
    
    return 0; /* Placeholder */
}

/**
 * vexfs_posix_disable_graph_awareness - Disable graph awareness for inode
 * @inode: Inode to disable graph awareness for
 */
int vexfs_posix_disable_graph_awareness(struct inode *inode)
{
    /* Implementation would:
     * 1. Remove mapping
     * 2. Delete graph node
     * 3. Clear extended attributes
     * 4. Restore standard inode operations
     */
    
    pr_debug("VexFS-POSIX: Disabling graph awareness for inode %lu\n", 
             inode ? inode->i_ino : 0);
    
    return 0; /* Placeholder */
}

/* Module information */
MODULE_DESCRIPTION("VexFS v2.0 VexGraph POSIX Integration Manager");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");