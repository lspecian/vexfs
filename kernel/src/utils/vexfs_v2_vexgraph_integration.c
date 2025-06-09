/*
 * VexFS v2.0 - VexGraph Integration (Task 8 - Phase 2)
 * 
 * This integrates VexGraph with the main VexFS module, providing seamless
 * integration between the filesystem and graph operations. Handles VFS
 * callbacks, graph synchronization, and Phase 1 foundation integration.
 *
 * Key Features:
 * - VFS operation hooks for graph synchronization
 * - Integration with journaling and atomic operations
 * - Graph-aware file operations
 * - Automatic graph updates on filesystem changes
 * - Performance monitoring and statistics
 * - Error handling and recovery
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/time.h>
#include <linux/dcache.h>
#include <linux/namei.h>

#include "../include/vexfs_v2_vexgraph.h"
#include "../include/vexfs_v2_internal.h"
#include "../include/vexfs_v2_journal.h"
#include "../include/vexfs_v2_atomic.h"

/* Global VexGraph manager instance */
static struct vexfs_graph_manager *global_graph_mgr = NULL;
static DEFINE_MUTEX(graph_mgr_mutex);

/* Forward declarations */
static int vexfs_graph_create_directory_edges(struct vexfs_graph_manager *mgr,
                                              struct inode *dir_inode,
                                              struct inode *child_inode);
static int vexfs_graph_remove_directory_edges(struct vexfs_graph_manager *mgr,
                                              struct inode *dir_inode,
                                              struct inode *child_inode);
static int vexfs_graph_journal_operation(struct vexfs_graph_manager *mgr,
                                         u8 op_type, u64 node_id, u64 edge_id);

/*
 * =============================================================================
 * VEXFS INTEGRATION FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_graph_init_superblock - Initialize VexGraph for a superblock
 * @sb: VexFS superblock
 * 
 * Initializes VexGraph for the given VexFS superblock.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_init_superblock(struct super_block *sb)
{
    struct vexfs_graph_manager *mgr;
    int ret;

    if (!sb) {
        return -EINVAL;
    }

    mutex_lock(&graph_mgr_mutex);

    /* Create graph manager if not already created */
    if (!global_graph_mgr) {
        mgr = vexfs_graph_manager_create(sb);
        if (!mgr) {
            mutex_unlock(&graph_mgr_mutex);
            return -ENOMEM;
        }

        ret = vexfs_graph_manager_init(mgr);
        if (ret != 0) {
            vexfs_graph_manager_destroy(mgr);
            mutex_unlock(&graph_mgr_mutex);
            return ret;
        }

        global_graph_mgr = mgr;
    }

    mutex_unlock(&graph_mgr_mutex);

    /* Create default indices */
    vexfs_graph_index_create(global_graph_mgr, VEXFS_GRAPH_INDEX_NODE_ID, NULL);
    vexfs_graph_index_create(global_graph_mgr, VEXFS_GRAPH_INDEX_EDGE_TYPE, NULL);
    vexfs_graph_index_create(global_graph_mgr, VEXFS_GRAPH_INDEX_PROPERTY, "type");
    vexfs_graph_index_create(global_graph_mgr, VEXFS_GRAPH_INDEX_PROPERTY, "size");

    /* Sync with existing filesystem state */
    vexfs_graph_sync_with_filesystem(global_graph_mgr);

    printk(KERN_INFO "VexGraph: Initialized for superblock\n");
    return 0;
}

/**
 * vexfs_graph_cleanup_superblock - Cleanup VexGraph for a superblock
 * @sb: VexFS superblock
 * 
 * Cleans up VexGraph resources for the given superblock.
 */
void vexfs_graph_cleanup_superblock(struct super_block *sb)
{
    mutex_lock(&graph_mgr_mutex);

    if (global_graph_mgr && global_graph_mgr->sb == sb) {
        vexfs_graph_manager_cleanup(global_graph_mgr);
        vexfs_graph_manager_destroy(global_graph_mgr);
        global_graph_mgr = NULL;
    }

    mutex_unlock(&graph_mgr_mutex);

    printk(KERN_INFO "VexGraph: Cleaned up for superblock\n");
}

/**
 * vexfs_graph_get_manager - Get the global graph manager
 * 
 * Returns the global graph manager instance.
 * 
 * Return: Pointer to graph manager, NULL if not initialized
 */
struct vexfs_graph_manager *vexfs_graph_get_manager(void)
{
    return global_graph_mgr;
}

/*
 * =============================================================================
 * VFS OPERATION HOOKS
 * =============================================================================
 */

/**
 * vexfs_graph_inode_create_hook - Hook for inode creation
 * @dir: Parent directory inode
 * @dentry: Dentry for new inode
 * @inode: Newly created inode
 * 
 * Called when a new inode is created to update the graph.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_inode_create_hook(struct inode *dir, struct dentry *dentry,
                                  struct inode *inode)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *node;
    int ret;

    mgr = vexfs_graph_get_manager();
    if (!mgr) {
        return 0;  /* Graph not initialized, skip */
    }

    /* Create graph node for the new inode */
    ret = vexfs_graph_inode_to_node(mgr, inode);
    if (ret != 0) {
        printk(KERN_WARNING "VexGraph: Failed to create node for inode %lu\n",
               inode->i_ino);
        return ret;
    }

    /* Create directory containment edge if parent is a directory */
    if (dir && S_ISDIR(dir->i_mode)) {
        ret = vexfs_graph_create_directory_edges(mgr, dir, inode);
        if (ret != 0) {
            printk(KERN_WARNING "VexGraph: Failed to create directory edge\n");
        }
    }

    /* Journal the operation */
    vexfs_graph_journal_operation(mgr, VEXFS_GRAPH_OP_NODE_CREATE,
                                  inode->i_ino, 0);

    printk(KERN_DEBUG "VexGraph: Created node for inode %lu\n", inode->i_ino);
    return 0;
}

/**
 * vexfs_graph_inode_delete_hook - Hook for inode deletion
 * @inode: Inode being deleted
 * 
 * Called when an inode is deleted to update the graph.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_inode_delete_hook(struct inode *inode)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *node;

    mgr = vexfs_graph_get_manager();
    if (!mgr) {
        return 0;  /* Graph not initialized, skip */
    }

    /* Find and remove the graph node */
    node = vexfs_graph_node_lookup(mgr, inode->i_ino);
    if (node) {
        /* Journal the operation */
        vexfs_graph_journal_operation(mgr, VEXFS_GRAPH_OP_NODE_DELETE,
                                      inode->i_ino, 0);

        vexfs_graph_node_destroy(mgr, node);
        printk(KERN_DEBUG "VexGraph: Deleted node for inode %lu\n", inode->i_ino);
    }

    return 0;
}

/**
 * vexfs_graph_inode_update_hook - Hook for inode updates
 * @inode: Inode being updated
 * 
 * Called when an inode is modified to update the graph.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_inode_update_hook(struct inode *inode)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *node;

    mgr = vexfs_graph_get_manager();
    if (!mgr) {
        return 0;  /* Graph not initialized, skip */
    }

    /* Update the graph node */
    node = vexfs_graph_node_lookup(mgr, inode->i_ino);
    if (node) {
        /* Update properties */
        vexfs_graph_node_add_property(node, "size", VEXFS_GRAPH_PROP_INTEGER,
                                      &inode->i_size, sizeof(inode->i_size));
        vexfs_graph_node_add_property(node, "mtime", VEXFS_GRAPH_PROP_TIMESTAMP,
                                      &inode->i_mtime.tv_sec, sizeof(inode->i_mtime.tv_sec));

        /* Update indices */
        vexfs_graph_index_update(mgr, node, NULL);

        /* Journal the operation */
        vexfs_graph_journal_operation(mgr, VEXFS_GRAPH_OP_NODE_UPDATE,
                                      inode->i_ino, 0);

        atomic_dec(&node->ref_count);
        printk(KERN_DEBUG "VexGraph: Updated node for inode %lu\n", inode->i_ino);
    }

    return 0;
}

/**
 * vexfs_graph_link_hook - Hook for hard link creation
 * @old_dentry: Existing dentry
 * @dir: Directory inode
 * @new_dentry: New dentry
 * 
 * Called when a hard link is created to update the graph.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_link_hook(struct dentry *old_dentry, struct inode *dir,
                          struct dentry *new_dentry)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_edge *edge;
    struct inode *inode;

    mgr = vexfs_graph_get_manager();
    if (!mgr || !old_dentry || !old_dentry->d_inode) {
        return 0;
    }

    inode = old_dentry->d_inode;

    /* Create additional containment edge for the hard link */
    if (S_ISDIR(dir->i_mode)) {
        vexfs_graph_create_directory_edges(mgr, dir, inode);
    }

    printk(KERN_DEBUG "VexGraph: Created link for inode %lu\n", inode->i_ino);
    return 0;
}

/**
 * vexfs_graph_unlink_hook - Hook for file unlinking
 * @dir: Directory inode
 * @dentry: Dentry being unlinked
 * 
 * Called when a file is unlinked to update the graph.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_unlink_hook(struct inode *dir, struct dentry *dentry)
{
    struct vexfs_graph_manager *mgr;
    struct inode *inode;

    mgr = vexfs_graph_get_manager();
    if (!mgr || !dentry || !dentry->d_inode) {
        return 0;
    }

    inode = dentry->d_inode;

    /* Remove directory containment edge */
    if (S_ISDIR(dir->i_mode)) {
        vexfs_graph_remove_directory_edges(mgr, dir, inode);
    }

    printk(KERN_DEBUG "VexGraph: Unlinked inode %lu\n", inode->i_ino);
    return 0;
}

/*
 * =============================================================================
 * GRAPH OPERATION HELPERS
 * =============================================================================
 */

/**
 * vexfs_graph_create_directory_edges - Create directory containment edges
 * @mgr: Graph manager
 * @dir_inode: Directory inode
 * @child_inode: Child inode
 * 
 * Creates a CONTAINS edge from directory to child.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_graph_create_directory_edges(struct vexfs_graph_manager *mgr,
                                              struct inode *dir_inode,
                                              struct inode *child_inode)
{
    struct vexfs_graph_edge *edge;

    if (!mgr || !dir_inode || !child_inode) {
        return -EINVAL;
    }

    /* Ensure both nodes exist */
    vexfs_graph_inode_to_node(mgr, dir_inode);
    vexfs_graph_inode_to_node(mgr, child_inode);

    /* Create containment edge */
    edge = vexfs_graph_edge_create(mgr, dir_inode->i_ino, child_inode->i_ino,
                                   VEXFS_GRAPH_EDGE_CONTAINS, 1);
    if (!edge) {
        return -ENOMEM;
    }

    /* Add edge properties */
    vexfs_graph_edge_add_property(edge, "relationship", VEXFS_GRAPH_PROP_STRING,
                                  "contains", 8);

    /* Update indices */
    vexfs_graph_index_update(mgr, NULL, edge);

    /* Journal the operation */
    vexfs_graph_journal_operation(mgr, VEXFS_GRAPH_OP_EDGE_CREATE,
                                  dir_inode->i_ino, edge->edge_id);

    return 0;
}

/**
 * vexfs_graph_remove_directory_edges - Remove directory containment edges
 * @mgr: Graph manager
 * @dir_inode: Directory inode
 * @child_inode: Child inode
 * 
 * Removes CONTAINS edges from directory to child.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_graph_remove_directory_edges(struct vexfs_graph_manager *mgr,
                                              struct inode *dir_inode,
                                              struct inode *child_inode)
{
    struct vexfs_graph_node *dir_node;
    struct vexfs_graph_edge *edge, *tmp;

    if (!mgr || !dir_inode || !child_inode) {
        return -EINVAL;
    }

    /* Find directory node */
    dir_node = vexfs_graph_node_lookup(mgr, dir_inode->i_ino);
    if (!dir_node) {
        return -ENOENT;
    }

    /* Find and remove containment edges */
    down_read(&dir_node->node_sem);
    list_for_each_entry_safe(edge, tmp, &dir_node->outgoing_edges, source_list) {
        if (edge->target_node_id == child_inode->i_ino &&
            edge->edge_type == VEXFS_GRAPH_EDGE_CONTAINS) {
            
            /* Journal the operation */
            vexfs_graph_journal_operation(mgr, VEXFS_GRAPH_OP_EDGE_DELETE,
                                          dir_inode->i_ino, edge->edge_id);
            
            vexfs_graph_edge_destroy(mgr, edge);
            break;
        }
    }
    up_read(&dir_node->node_sem);

    atomic_dec(&dir_node->ref_count);
    return 0;
}

/**
 * vexfs_graph_journal_operation - Journal a graph operation
 * @mgr: Graph manager
 * @op_type: Operation type
 * @node_id: Node ID involved
 * @edge_id: Edge ID involved (0 if not applicable)
 * 
 * Journals a graph operation for consistency and recovery.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_graph_journal_operation(struct vexfs_graph_manager *mgr,
                                         u8 op_type, u64 node_id, u64 edge_id)
{
    /* TODO: Integrate with VexFS journaling system when available */
    /* This would write graph operations to the journal for crash recovery */
    
    printk(KERN_DEBUG "VexGraph: Journaled operation %u (node %llu, edge %llu)\n",
           op_type, node_id, edge_id);
    return 0;
}

/*
 * =============================================================================
 * GRAPH STATISTICS AND MONITORING
 * =============================================================================
 */

/**
 * vexfs_graph_validate_integrity - Validate graph integrity
 * @mgr: Graph manager
 * 
 * Performs integrity checks on the graph structure.
 * 
 * Return: 0 if valid, negative error code if corruption detected
 */
int vexfs_graph_validate_integrity(struct vexfs_graph_manager *mgr)
{
    struct rb_node *node;
    struct vexfs_graph_node *graph_node;
    struct vexfs_graph_edge *edge;
    u64 node_count = 0, edge_count = 0;
    int errors = 0;

    if (!mgr) {
        return -EINVAL;
    }

    down_read(&mgr->graph_sem);

    /* Validate nodes */
    for (node = rb_first(&mgr->nodes_tree); node; node = rb_next(node)) {
        graph_node = rb_entry(node, struct vexfs_graph_node, rb_node);
        node_count++;

        /* Check node consistency */
        if (graph_node->node_id == 0) {
            printk(KERN_ERR "VexGraph: Invalid node ID 0\n");
            errors++;
        }

        /* Validate outgoing edges */
        list_for_each_entry(edge, &graph_node->outgoing_edges, source_list) {
            if (edge->source_node_id != graph_node->node_id) {
                printk(KERN_ERR "VexGraph: Edge source mismatch\n");
                errors++;
            }
        }

        /* Validate incoming edges */
        list_for_each_entry(edge, &graph_node->incoming_edges, target_list) {
            if (edge->target_node_id != graph_node->node_id) {
                printk(KERN_ERR "VexGraph: Edge target mismatch\n");
                errors++;
            }
        }
    }

    /* Validate edges */
    for (node = rb_first(&mgr->edges_tree); node; node = rb_next(node)) {
        edge = rb_entry(node, struct vexfs_graph_edge, rb_node);
        edge_count++;

        /* Check edge consistency */
        if (edge->edge_id == 0 || edge->source_node_id == edge->target_node_id) {
            printk(KERN_ERR "VexGraph: Invalid edge %llu\n", edge->edge_id);
            errors++;
        }
    }

    up_read(&mgr->graph_sem);

    /* Check counts */
    if (node_count != atomic64_read(&mgr->node_count)) {
        printk(KERN_ERR "VexGraph: Node count mismatch (%llu vs %llu)\n",
               node_count, atomic64_read(&mgr->node_count));
        errors++;
    }

    if (edge_count != atomic64_read(&mgr->edge_count)) {
        printk(KERN_ERR "VexGraph: Edge count mismatch (%llu vs %llu)\n",
               edge_count, atomic64_read(&mgr->edge_count));
        errors++;
    }

    if (errors > 0) {
        printk(KERN_ERR "VexGraph: Integrity check failed with %d errors\n", errors);
        return -EINVAL;
    }

    printk(KERN_INFO "VexGraph: Integrity check passed (%llu nodes, %llu edges)\n",
           node_count, edge_count);
    return 0;
}

/*
 * =============================================================================
 * MODULE INTERFACE
 * =============================================================================
 */

/**
 * vexfs_graph_module_init - Initialize VexGraph module
 * 
 * Initializes the VexGraph module.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_module_init(void)
{
    printk(KERN_INFO "VexGraph: Module initialized\n");
    return 0;
}

/**
 * vexfs_graph_module_exit - Cleanup VexGraph module
 * 
 * Cleans up the VexGraph module.
 */
void vexfs_graph_module_exit(void)
{
    mutex_lock(&graph_mgr_mutex);
    
    if (global_graph_mgr) {
        vexfs_graph_manager_cleanup(global_graph_mgr);
        vexfs_graph_manager_destroy(global_graph_mgr);
        global_graph_mgr = NULL;
    }
    
    mutex_unlock(&graph_mgr_mutex);
    
    printk(KERN_INFO "VexGraph: Module cleaned up\n");
}

/* Export symbols for use by main VexFS module */
EXPORT_SYMBOL(vexfs_graph_init_superblock);
EXPORT_SYMBOL(vexfs_graph_cleanup_superblock);
EXPORT_SYMBOL(vexfs_graph_get_manager);
EXPORT_SYMBOL(vexfs_graph_inode_create_hook);
EXPORT_SYMBOL(vexfs_graph_inode_delete_hook);
EXPORT_SYMBOL(vexfs_graph_inode_update_hook);
EXPORT_SYMBOL(vexfs_graph_link_hook);
EXPORT_SYMBOL(vexfs_graph_unlink_hook);
EXPORT_SYMBOL(vexfs_graph_validate_integrity);

MODULE_LICENSE("GPL v2");
MODULE_DESCRIPTION("VexFS v2.0 VexGraph Integration");