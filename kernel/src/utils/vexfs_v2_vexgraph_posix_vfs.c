/*
 * VexFS v2.0 - VexGraph POSIX VFS Hooks Implementation (Task 10 - Phase 2)
 * 
 * This implements VFS hooks for seamless integration between VexGraph operations
 * and traditional POSIX filesystem operations. These hooks intercept standard
 * filesystem operations and coordinate them with graph operations.
 *
 * Key Features:
 * - VFS Hook Implementation for create, unlink, rename, mkdir, rmdir operations
 * - Transparent graph node creation/deletion during filesystem operations
 * - Edge management for directory relationships
 * - Dual-view consistency maintenance
 * - Performance optimization for mixed operations
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/dcache.h>
#include <linux/namei.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/uaccess.h>
#include <linux/path.h>
#include <linux/mount.h>
#include <linux/security.h>

#include "../include/vexfs_v2_vexgraph_posix.h"
#include "../include/vexfs_v2_internal.h"

/* Static function declarations */
static int vexfs_posix_create_graph_node_for_inode(struct inode *inode, u32 node_type);
static int vexfs_posix_delete_graph_node_for_inode(struct inode *inode);
static int vexfs_posix_create_directory_edge(struct inode *parent, struct inode *child);
static int vexfs_posix_remove_directory_edge(struct inode *parent, struct inode *child);
static int vexfs_posix_update_graph_metadata(struct inode *inode, struct dentry *dentry);

/*
 * VFS Hook Implementations
 */

/**
 * vexfs_posix_hook_create - Hook for file creation operations
 * @mnt_userns: User namespace of the mount
 * @dir: Parent directory inode
 * @dentry: Dentry for the new file
 * @mode: File mode
 * @excl: Exclusive creation flag
 *
 * This hook intercepts file creation and automatically creates corresponding
 * graph nodes and edges for seamless graph-filesystem integration.
 */
int vexfs_posix_hook_create(struct user_namespace *mnt_userns, struct inode *dir,
                           struct dentry *dentry, umode_t mode, bool excl)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct inode *inode;
    int ret;
    u32 node_type;

    if (!manager) {
        pr_debug("VexFS-POSIX: No integration manager, using standard create\n");
        return -ENOSYS; /* Fall back to standard operation */
    }

    pr_debug("VexFS-POSIX: Hook create - %s in dir %lu\n", 
             dentry->d_name.name, dir->i_ino);

    /* Acquire operation lock */
    ret = vexfs_posix_acquire_operation_lock(manager, VEXFS_POSIX_OP_CREATE, false);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to acquire operation lock for create\n");
        return ret;
    }

    /* Perform standard file creation first */
    inode = new_inode(dir->i_sb);
    if (!inode) {
        ret = -ENOMEM;
        goto out_unlock;
    }

    /* Set up inode */
    inode->i_ino = get_next_ino();
    inode->i_mode = mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    inode->i_atime = inode->i_mtime = inode->i_ctime = current_time(inode);
    inode->i_size = 0;

    /* Determine graph node type based on file type */
    if (S_ISREG(mode)) {
        node_type = VEXFS_GRAPH_NODE_FILE;
    } else if (S_ISLNK(mode)) {
        node_type = VEXFS_GRAPH_NODE_SYMLINK;
    } else {
        node_type = VEXFS_GRAPH_NODE_OTHER;
    }

    /* Create graph node for the new file */
    if (manager->flags & VEXFS_POSIX_FLAG_AUTO_NODE) {
        ret = vexfs_posix_create_graph_node_for_inode(inode, node_type);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to create graph node for new file: %d\n", ret);
            /* Continue with filesystem operation even if graph creation fails */
        }
    }

    /* Create directory relationship edge */
    if (manager->flags & VEXFS_POSIX_FLAG_AUTO_NODE) {
        ret = vexfs_posix_create_directory_edge(dir, inode);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to create directory edge: %d\n", ret);
            /* Continue with filesystem operation */
        }
    }

    /* Update graph metadata */
    ret = vexfs_posix_update_graph_metadata(inode, dentry);
    if (ret) {
        pr_debug("VexFS-POSIX: Failed to update graph metadata: %d\n", ret);
    }

    /* Insert into dcache */
    d_instantiate(dentry, inode);

    /* Update statistics */
    vexfs_posix_update_operation_stats(manager, VEXFS_POSIX_OP_CREATE, true);

    pr_debug("VexFS-POSIX: Successfully created file %s with graph integration\n",
             dentry->d_name.name);

    ret = 0;

out_unlock:
    vexfs_posix_release_operation_lock(manager, VEXFS_POSIX_OP_CREATE, false);
    return ret;
}

/**
 * vexfs_posix_hook_unlink - Hook for file deletion operations
 * @dir: Parent directory inode
 * @dentry: Dentry for the file to delete
 *
 * This hook intercepts file deletion and removes corresponding graph nodes
 * and edges while maintaining consistency.
 */
int vexfs_posix_hook_unlink(struct inode *dir, struct dentry *dentry)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct inode *inode = d_inode(dentry);
    int ret;

    if (!manager) {
        pr_debug("VexFS-POSIX: No integration manager, using standard unlink\n");
        return -ENOSYS; /* Fall back to standard operation */
    }

    if (!inode) {
        pr_err("VexFS-POSIX: No inode for dentry in unlink\n");
        return -ENOENT;
    }

    pr_debug("VexFS-POSIX: Hook unlink - %s from dir %lu\n", 
             dentry->d_name.name, dir->i_ino);

    /* Acquire operation lock */
    ret = vexfs_posix_acquire_operation_lock(manager, VEXFS_POSIX_OP_UNLINK, false);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to acquire operation lock for unlink\n");
        return ret;
    }

    /* Remove directory relationship edge first */
    if (manager->flags & VEXFS_POSIX_FLAG_PRESERVE_EDGES) {
        ret = vexfs_posix_remove_directory_edge(dir, inode);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to remove directory edge: %d\n", ret);
        }
    }

    /* Remove graph node for the file */
    ret = vexfs_posix_delete_graph_node_for_inode(inode);
    if (ret) {
        pr_warn("VexFS-POSIX: Failed to delete graph node: %d\n", ret);
        /* Continue with filesystem operation even if graph deletion fails */
    }

    /* Perform standard unlink operation */
    drop_nlink(inode);
    dput(dentry);

    /* Update statistics */
    vexfs_posix_update_operation_stats(manager, VEXFS_POSIX_OP_UNLINK, true);

    pr_debug("VexFS-POSIX: Successfully unlinked file %s with graph integration\n",
             dentry->d_name.name);

    vexfs_posix_release_operation_lock(manager, VEXFS_POSIX_OP_UNLINK, false);
    return 0;
}

/**
 * vexfs_posix_hook_rename - Hook for file/directory rename operations
 * @mnt_userns: User namespace of the mount
 * @old_dir: Old parent directory inode
 * @old_dentry: Old dentry
 * @new_dir: New parent directory inode
 * @new_dentry: New dentry
 * @flags: Rename flags
 *
 * This hook handles rename operations while maintaining graph consistency.
 */
int vexfs_posix_hook_rename(struct user_namespace *mnt_userns, struct inode *old_dir,
                           struct dentry *old_dentry, struct inode *new_dir,
                           struct dentry *new_dentry, unsigned int flags)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct inode *inode = d_inode(old_dentry);
    int ret;

    if (!manager) {
        pr_debug("VexFS-POSIX: No integration manager, using standard rename\n");
        return -ENOSYS;
    }

    if (!inode) {
        pr_err("VexFS-POSIX: No inode for old dentry in rename\n");
        return -ENOENT;
    }

    pr_debug("VexFS-POSIX: Hook rename - %s from dir %lu to dir %lu\n", 
             old_dentry->d_name.name, old_dir->i_ino, new_dir->i_ino);

    /* Acquire operation lock */
    ret = vexfs_posix_acquire_operation_lock(manager, VEXFS_POSIX_OP_RENAME, false);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to acquire operation lock for rename\n");
        return ret;
    }

    /* Update directory edges if moving between directories */
    if (old_dir != new_dir) {
        /* Remove old directory edge */
        ret = vexfs_posix_remove_directory_edge(old_dir, inode);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to remove old directory edge: %d\n", ret);
        }

        /* Create new directory edge */
        ret = vexfs_posix_create_directory_edge(new_dir, inode);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to create new directory edge: %d\n", ret);
        }
    }

    /* Update graph metadata with new name */
    ret = vexfs_posix_update_graph_metadata(inode, new_dentry);
    if (ret) {
        pr_debug("VexFS-POSIX: Failed to update graph metadata: %d\n", ret);
    }

    /* Update timestamps */
    inode->i_ctime = current_time(inode);

    /* Update statistics */
    vexfs_posix_update_operation_stats(manager, VEXFS_POSIX_OP_RENAME, true);

    pr_debug("VexFS-POSIX: Successfully renamed with graph integration\n");

    vexfs_posix_release_operation_lock(manager, VEXFS_POSIX_OP_RENAME, false);
    return 0;
}

/**
 * vexfs_posix_hook_mkdir - Hook for directory creation operations
 * @mnt_userns: User namespace of the mount
 * @dir: Parent directory inode
 * @dentry: Dentry for the new directory
 * @mode: Directory mode
 *
 * This hook handles directory creation with graph integration.
 */
int vexfs_posix_hook_mkdir(struct user_namespace *mnt_userns, struct inode *dir,
                          struct dentry *dentry, umode_t mode)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct inode *inode;
    int ret;

    if (!manager) {
        pr_debug("VexFS-POSIX: No integration manager, using standard mkdir\n");
        return -ENOSYS;
    }

    pr_debug("VexFS-POSIX: Hook mkdir - %s in dir %lu\n", 
             dentry->d_name.name, dir->i_ino);

    /* Acquire operation lock */
    ret = vexfs_posix_acquire_operation_lock(manager, VEXFS_POSIX_OP_MKDIR, false);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to acquire operation lock for mkdir\n");
        return ret;
    }

    /* Create new inode for directory */
    inode = new_inode(dir->i_sb);
    if (!inode) {
        ret = -ENOMEM;
        goto out_unlock;
    }

    /* Set up directory inode */
    inode->i_ino = get_next_ino();
    inode->i_mode = S_IFDIR | mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    inode->i_atime = inode->i_mtime = inode->i_ctime = current_time(inode);
    set_nlink(inode, 2); /* . and .. */
    inode->i_size = 0;

    /* Create graph node for the new directory */
    if (manager->flags & VEXFS_POSIX_FLAG_AUTO_NODE) {
        ret = vexfs_posix_create_graph_node_for_inode(inode, VEXFS_GRAPH_NODE_DIRECTORY);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to create graph node for new directory: %d\n", ret);
        }
    }

    /* Create directory relationship edge */
    if (manager->flags & VEXFS_POSIX_FLAG_AUTO_NODE) {
        ret = vexfs_posix_create_directory_edge(dir, inode);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to create directory edge: %d\n", ret);
        }
    }

    /* Update graph metadata */
    ret = vexfs_posix_update_graph_metadata(inode, dentry);
    if (ret) {
        pr_debug("VexFS-POSIX: Failed to update graph metadata: %d\n", ret);
    }

    /* Increment parent directory link count */
    inc_nlink(dir);

    /* Insert into dcache */
    d_instantiate(dentry, inode);

    /* Update statistics */
    vexfs_posix_update_operation_stats(manager, VEXFS_POSIX_OP_MKDIR, true);

    pr_debug("VexFS-POSIX: Successfully created directory %s with graph integration\n",
             dentry->d_name.name);

    ret = 0;

out_unlock:
    vexfs_posix_release_operation_lock(manager, VEXFS_POSIX_OP_MKDIR, false);
    return ret;
}

/**
 * vexfs_posix_hook_rmdir - Hook for directory removal operations
 * @dir: Parent directory inode
 * @dentry: Dentry for the directory to remove
 *
 * This hook handles directory removal with graph integration.
 */
int vexfs_posix_hook_rmdir(struct inode *dir, struct dentry *dentry)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct inode *inode = d_inode(dentry);
    int ret;

    if (!manager) {
        pr_debug("VexFS-POSIX: No integration manager, using standard rmdir\n");
        return -ENOSYS;
    }

    if (!inode) {
        pr_err("VexFS-POSIX: No inode for dentry in rmdir\n");
        return -ENOENT;
    }

    pr_debug("VexFS-POSIX: Hook rmdir - %s from dir %lu\n", 
             dentry->d_name.name, dir->i_ino);

    /* Acquire operation lock */
    ret = vexfs_posix_acquire_operation_lock(manager, VEXFS_POSIX_OP_RMDIR, false);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to acquire operation lock for rmdir\n");
        return ret;
    }

    /* Check if directory is empty (should only contain . and ..) */
    if (inode->i_nlink > 2) {
        ret = -ENOTEMPTY;
        goto out_unlock;
    }

    /* Remove directory relationship edge */
    if (manager->flags & VEXFS_POSIX_FLAG_PRESERVE_EDGES) {
        ret = vexfs_posix_remove_directory_edge(dir, inode);
        if (ret) {
            pr_warn("VexFS-POSIX: Failed to remove directory edge: %d\n", ret);
        }
    }

    /* Remove graph node for the directory */
    ret = vexfs_posix_delete_graph_node_for_inode(inode);
    if (ret) {
        pr_warn("VexFS-POSIX: Failed to delete graph node: %d\n", ret);
    }

    /* Perform standard rmdir operation */
    clear_nlink(inode);
    drop_nlink(dir);
    dput(dentry);

    /* Update statistics */
    vexfs_posix_update_operation_stats(manager, VEXFS_POSIX_OP_RMDIR, true);

    pr_debug("VexFS-POSIX: Successfully removed directory %s with graph integration\n",
             dentry->d_name.name);

    ret = 0;

out_unlock:
    vexfs_posix_release_operation_lock(manager, VEXFS_POSIX_OP_RMDIR, false);
    return ret;
}

/*
 * Helper Functions for Graph Operations
 */

/**
 * vexfs_posix_create_graph_node_for_inode - Create graph node for inode
 * @inode: Inode to create graph node for
 * @node_type: Type of graph node to create
 */
static int vexfs_posix_create_graph_node_for_inode(struct inode *inode, u32 node_type)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    char properties_json[512];
    int ret;

    if (!manager || !manager->api_manager) {
        pr_err("VexFS-POSIX: No API manager available for graph node creation\n");
        return -ENODEV;
    }

    /* Allocate API request and response */
    request = vexfs_api_request_alloc(manager->api_manager);
    if (!request) {
        pr_err("VexFS-POSIX: Failed to allocate API request\n");
        return -ENOMEM;
    }

    response = vexfs_api_response_alloc(manager->api_manager);
    if (!response) {
        pr_err("VexFS-POSIX: Failed to allocate API response\n");
        vexfs_api_request_free(manager->api_manager, request);
        return -ENOMEM;
    }

    /* Prepare node properties */
    snprintf(properties_json, sizeof(properties_json),
             "{\"inode\":%lu,\"mode\":%u,\"size\":%lld,\"uid\":%u,\"gid\":%u}",
             inode->i_ino, inode->i_mode, inode->i_size,
             from_kuid(&init_user_ns, inode->i_uid),
             from_kgid(&init_user_ns, inode->i_gid));

    /* Set up request */
    request->operation = VEXFS_API_OP_NODE_CREATE;
    request->params.node_create.node_type = node_type;
    request->params.node_create.properties_json = properties_json;

    /* Create graph node */
    ret = vexfs_api_node_create(manager->api_manager, request, response);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to create graph node: %d\n", ret);
        goto out_free;
    }

    /* Create mapping between inode and graph node */
    ret = vexfs_posix_create_node_mapping(manager, inode, 
                                         response->data.node_create.node_id, node_type);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to create node mapping: %d\n", ret);
        /* TODO: Delete the created graph node on mapping failure */
        goto out_free;
    }

    pr_debug("VexFS-POSIX: Created graph node %llu for inode %lu\n",
             response->data.node_create.node_id, inode->i_ino);

out_free:
    vexfs_api_response_free(manager->api_manager, response);
    vexfs_api_request_free(manager->api_manager, request);
    return ret;
}

/**
 * vexfs_posix_delete_graph_node_for_inode - Delete graph node for inode
 * @inode: Inode to delete graph node for
 */
static int vexfs_posix_delete_graph_node_for_inode(struct inode *inode)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct vexfs_node_file_mapping *mapping;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    int ret;

    if (!manager || !manager->api_manager) {
        pr_err("VexFS-POSIX: No API manager available for graph node deletion\n");
        return -ENODEV;
    }

    /* Find mapping for inode */
    down_read(&manager->mapping_lock);
    mapping = vexfs_posix_find_mapping_by_inode(manager, inode);
    if (!mapping) {
        up_read(&manager->mapping_lock);
        pr_debug("VexFS-POSIX: No mapping found for inode %lu\n", inode->i_ino);
        return 0; /* Not an error - inode might not be graph-aware */
    }
    up_read(&manager->mapping_lock);

    /* Allocate API request and response */
    request = vexfs_api_request_alloc(manager->api_manager);
    if (!request) {
        atomic_dec(&mapping->ref_count);
        return -ENOMEM;
    }

    response = vexfs_api_response_alloc(manager->api_manager);
    if (!response) {
        vexfs_api_request_free(manager->api_manager, request);
        atomic_dec(&mapping->ref_count);
        return -ENOMEM;
    }

    /* Set up request */
    request->operation = VEXFS_API_OP_NODE_DELETE;
    request->params.node_delete.node_id = mapping->graph_node_id;

    /* Delete graph node */
    ret = vexfs_api_node_delete(manager->api_manager, request, response);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to delete graph node %llu: %d\n",
               mapping->graph_node_id, ret);
    } else {
        pr_debug("VexFS-POSIX: Deleted graph node %llu for inode %lu\n",
                 mapping->graph_node_id, inode->i_ino);
    }

    /* Remove mapping */
    vexfs_posix_remove_node_mapping(manager, inode);

    vexfs_api_response_free(manager->api_manager, response);
    vexfs_api_request_free(manager->api_manager, request);
    atomic_dec(&mapping->ref_count);

    return ret;
}

/**
 * vexfs_posix_create_directory_edge - Create edge for directory relationship
 * @parent: Parent directory inode
 * @child: Child inode
 */
static int vexfs_posix_create_directory_edge(struct inode *parent, struct inode *child)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    struct vexfs_node_file_mapping *parent_mapping, *child_mapping;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    int ret;

    if (!manager || !manager->api_manager) {
        return -ENODEV;
    }

    /* Find mappings for both inodes */
    down_read(&manager->mapping_lock);
    parent_mapping = vexfs_posix_find_mapping_by_inode(manager, parent);
    child_mapping = vexfs_posix_find_mapping_by_inode(manager, child);
    up_read(&manager->mapping_lock);

    if (!parent_mapping || !child_mapping) {
        pr_debug("VexFS-POSIX: Missing mappings for directory edge creation\n");
        if (parent_mapping) atomic_dec(&parent_mapping->ref_count);
        if (child_mapping) atomic_dec(&child_mapping->ref_count);
        return 0; /* Not an error - nodes might not be graph-aware */
    }

    /* Allocate API request and response */
    request = vexfs_api_request_alloc(manager->api_manager);
    if (!request) {
        ret = -ENOMEM;
        goto out_dec_refs;
    }

    response = vexfs_api_response_alloc(manager->api_manager);
    if (!response) {
        vexfs_api_request_free(manager->api_manager, request);
        ret = -ENOMEM;
        goto out_dec_refs;
    }

    /* Set up request for directory edge */
    request->operation = VEXFS_API_OP_EDGE_CREATE;
    request->params.edge_create.source_node_id = parent_mapping->graph_node_id;
    request->params.edge_create.target_node_id = child_mapping->graph_node_id;
    request->params.edge_create.edge_type = VEXFS_GRAPH_EDGE_CONTAINS;
    request->params.edge_create.weight = 1;
    request->params.edge_create.properties_json = "{}";

    /* Create directory edge */
    ret = vexfs_api_edge_create(manager->api_manager, request, response);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to create directory edge: %d\n", ret);
    } else {
        pr_debug("VexFS-POSIX: Created directory edge %llu -> %llu\n",
                 parent_mapping->graph_node_id, child_mapping->graph_node_id);
    }

    vexfs_api_response_free(manager->api_manager, response);
    vexfs_api_request_free(manager->api_manager, request);

out_dec_refs:
    atomic_dec(&parent_mapping->ref_count);
    atomic_dec(&child_mapping->ref_count);
    return ret;
}

/**
 * vexfs_posix_remove_directory_edge - Remove edge for directory relationship
 * @parent: Parent directory inode
 * @child: Child inode
 */
static int vexfs_posix_remove_directory_edge(struct inode *parent, struct inode *child)
{
    /* Implementation similar to create_directory_edge but for deletion */
    pr_debug("VexFS-POSIX: Removing directory edge between inodes %lu and %lu\n",
             parent->i_ino, child->i_ino);
    
    /* TODO: Implement edge removal logic */
    return 0;
}

/**
 * vexfs_posix_update_graph_metadata - Update graph metadata for inode
 * @inode: Inode to update metadata for
 * @dentry: Dentry with name information
 */
static int vexfs_posix_update_graph_metadata(struct inode *inode, struct dentry *dentry)
{
    /* Implementation would update graph node properties with filesystem metadata */
    pr_debug("VexFS-POSIX: Updating graph metadata for inode %lu (%s)\n",
             inode->i_ino, dentry ? dentry->d_name.name : "unknown");
    
    /* TODO: Implement metadata update logic */
    return 0;
}

/*
 * Operation Locking Functions
 */

/**
 * vexfs_posix_acquire_operation_lock - Acquire lock for operation coordination
 * @manager: Integration manager
 * @operation_type: Type of operation
 * @exclusive: Whether to acquire exclusive lock
 */
int vexfs_posix_acquire_operation_lock(struct vexfs_posix_integration_manager *manager,
                                      u32 operation_type, bool exclusive)
{
    if (!manager) {
        return -EINVAL;
    }

    if (exclusive) {
        down_write(&manager->operation_lock);
    } else {
        down_read(&manager->operation_lock);
    }

    /* Update operation counters */
    switch (operation_type) {
    case VEXFS_POSIX_OP_CREATE:
    case VEXFS_POSIX_OP_UNLINK:
    case VEXFS_POSIX_OP_RENAME:
    case VEXFS_POSIX_OP_MKDIR:
    case VEXFS_POSIX_OP_RMDIR:
        atomic_inc(&manager->active_posix_ops);
        break;
    default:
        atomic_inc(&manager->active_graph_ops);
        break;
    }

    return 0;
}

/**
 * vexfs_posix_release_operation_lock - Release operation lock
 * @manager: Integration manager
 * @operation_type: Type of operation
 * @exclusive: Whether lock was acquired exclusively
 */
void vexfs_posix_release_operation_lock(struct vexfs_posix_integration_manager *manager,
                                       u32 operation_type, bool exclusive)
{
    if (!manager) {
        return;
    }

    /* Update operation counters */
    switch (operation_type) {
    case VEXFS_POSIX_OP_CREATE:
case VEXFS_POSIX_OP_UNLINK:
    case VEXFS_POSIX_OP_RENAME:
    case VEXFS_POSIX_OP_MKDIR:
    case VEXFS_POSIX_OP_RMDIR:
        atomic_dec(&manager->active_posix_ops);
        break;
    default:
        atomic_dec(&manager->active_graph_ops);
        break;
    }

    if (exclusive) {
        up_write(&manager->operation_lock);
    } else {
        up_read(&manager->operation_lock);
    }
}

/* Module information */
MODULE_DESCRIPTION("VexFS v2.0 VexGraph POSIX VFS Hooks");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");