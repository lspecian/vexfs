/*
 * VexFS v2.0 - VexGraph POSIX ioctl Interface Implementation (Task 10 - Phase 2)
 * 
 * This implements the ioctl interface for VexGraph-POSIX operations, providing
 * direct graph operations through filesystem paths and enabling seamless
 * integration between graph and filesystem views.
 *
 * Key Features:
 * - ioctl Interface for graph operations through POSIX paths
 * - Node creation/deletion through filesystem paths
 * - Edge management between filesystem objects
 * - Graph queries using filesystem paths
 * - Property management through extended attributes
 * - View synchronization between graph and filesystem
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/dcache.h>
#include <linux/security.h>

#include "../include/vexfs_v2_vexgraph_posix.h"
#include "../include/vexfs_v2_internal.h"

/* Static function declarations */
static int vexfs_posix_path_lookup(const char *path, struct path *result);
static int vexfs_posix_validate_ioctl_request(unsigned int cmd, unsigned long arg);
static int vexfs_posix_copy_string_from_user(char *dest, const char __user *src, size_t max_len);

/*
 * Main ioctl Interface Implementation
 */

/**
 * vexfs_posix_graph_ioctl - Main ioctl handler for VexGraph-POSIX operations
 * @file: File pointer
 * @cmd: ioctl command
 * @arg: ioctl argument
 *
 * This is the main entry point for all VexGraph-POSIX ioctl operations.
 */
long vexfs_posix_graph_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
    struct vexfs_posix_integration_manager *manager = vexfs_global_posix_manager;
    long ret;

    if (!manager) {
        pr_err("VexFS-POSIX: No integration manager available for ioctl\n");
        return -ENODEV;
    }

    /* Validate ioctl request */
    ret = vexfs_posix_validate_ioctl_request(cmd, arg);
    if (ret) {
        pr_err("VexFS-POSIX: Invalid ioctl request: %d\n", ret);
        return ret;
    }

    pr_debug("VexFS-POSIX: Processing ioctl command 0x%x\n", cmd);

    /* Dispatch to specific ioctl handlers */
    switch (cmd) {
    case VEXFS_IOC_GRAPH_CREATE_NODE:
        ret = vexfs_posix_ioctl_graph_create_node(manager, 
                (struct vexfs_posix_graph_node_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_DELETE_NODE:
        ret = vexfs_posix_ioctl_graph_delete_node(manager,
                (struct vexfs_posix_graph_node_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_CREATE_EDGE:
        ret = vexfs_posix_ioctl_graph_create_edge(manager,
                (struct vexfs_posix_graph_edge_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_DELETE_EDGE:
        ret = vexfs_posix_ioctl_graph_delete_edge(manager,
                (struct vexfs_posix_graph_edge_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_QUERY_NODE:
        ret = vexfs_posix_ioctl_graph_query(manager,
                (struct vexfs_posix_graph_query_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_TRAVERSE:
        ret = vexfs_posix_ioctl_graph_traverse(manager,
                (struct vexfs_posix_graph_traversal_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_SET_PROPERTY:
        ret = vexfs_posix_ioctl_graph_set_property(manager,
                (struct vexfs_posix_graph_property_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_GET_PROPERTY:
        ret = vexfs_posix_ioctl_graph_get_property(manager,
                (struct vexfs_posix_graph_property_request __user *)arg);
        break;

    case VEXFS_IOC_GRAPH_SYNC_VIEW:
        ret = vexfs_posix_ioctl_graph_sync_view(manager,
                (struct vexfs_posix_graph_sync_request __user *)arg);
        break;

    default:
        pr_warn("VexFS-POSIX: Unknown ioctl command 0x%x\n", cmd);
        ret = -ENOTTY;
        break;
    }

    pr_debug("VexFS-POSIX: ioctl command 0x%x completed with result %ld\n", cmd, ret);
    return ret;
}

/*
 * Individual ioctl Operation Implementations
 */

/**
 * vexfs_posix_ioctl_graph_create_node - Create graph node through filesystem path
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_create_node(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_node_request __user *arg)
{
    struct vexfs_posix_graph_node_request req;
    struct path path;
    struct inode *inode;
    struct vexfs_api_request *api_req;
    struct vexfs_api_response *api_resp;
    int ret;

    /* Copy request from user space */
    if (copy_from_user(&req, arg, sizeof(req))) {
        pr_err("VexFS-POSIX: Failed to copy node request from user\n");
        return -EFAULT;
    }

    /* Null-terminate strings for safety */
    req.path[PATH_MAX - 1] = '\0';
    req.properties_json[VEXFS_POSIX_MAX_PROPERTY_SIZE - 1] = '\0';

    pr_debug("VexFS-POSIX: Creating graph node for path: %s\n", req.path);

    /* Look up the filesystem path */
    ret = vexfs_posix_path_lookup(req.path, &path);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to lookup path %s: %d\n", req.path, ret);
        return ret;
    }

    inode = d_inode(path.dentry);
    if (!inode) {
        path_put(&path);
        pr_err("VexFS-POSIX: No inode for path %s\n", req.path);
        return -ENOENT;
    }

    /* Check if node already exists for this inode */
    if (vexfs_posix_is_graph_aware_inode(inode)) {
        path_put(&path);
        pr_warn("VexFS-POSIX: Graph node already exists for path %s\n", req.path);
        return -EEXIST;
    }

    /* Allocate API request and response */
    api_req = vexfs_api_request_alloc(manager->api_manager);
    if (!api_req) {
        path_put(&path);
        return -ENOMEM;
    }

    api_resp = vexfs_api_response_alloc(manager->api_manager);
    if (!api_resp) {
        vexfs_api_request_free(manager->api_manager, api_req);
        path_put(&path);
        return -ENOMEM;
    }

    /* Set up API request */
    api_req->operation = VEXFS_API_OP_NODE_CREATE;
    api_req->params.node_create.node_type = req.node_type;
    api_req->params.node_create.properties_json = req.properties_json;

    /* Create graph node */
    ret = vexfs_api_node_create(manager->api_manager, api_req, api_resp);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to create graph node: %d\n", ret);
        goto out_free;
    }

    /* Create mapping between inode and graph node */
    ret = vexfs_posix_create_node_mapping(manager, inode, 
                                         api_resp->data.node_create.node_id, req.node_type);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to create node mapping: %d\n", ret);
        /* TODO: Delete the created graph node on mapping failure */
        goto out_free;
    }

    /* Copy node ID back to user space */
    req.node_id = api_resp->data.node_create.node_id;
    if (copy_to_user(arg, &req, sizeof(req))) {
        pr_err("VexFS-POSIX: Failed to copy node ID to user\n");
        ret = -EFAULT;
        goto out_free;
    }

    pr_info("VexFS-POSIX: Created graph node %llu for path %s\n",
            req.node_id, req.path);

out_free:
    vexfs_api_response_free(manager->api_manager, api_resp);
    vexfs_api_request_free(manager->api_manager, api_req);
    path_put(&path);
    return ret;
}

/**
 * vexfs_posix_ioctl_graph_delete_node - Delete graph node through filesystem path
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_delete_node(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_node_request __user *arg)
{
    struct vexfs_posix_graph_node_request req;
    struct path path;
    struct inode *inode;
    struct vexfs_node_file_mapping *mapping;
    struct vexfs_api_request *api_req;
    struct vexfs_api_response *api_resp;
    int ret;

    /* Copy request from user space */
    if (copy_from_user(&req, arg, sizeof(req))) {
        return -EFAULT;
    }

    req.path[PATH_MAX - 1] = '\0';

    pr_debug("VexFS-POSIX: Deleting graph node for path: %s\n", req.path);

    /* Look up the filesystem path */
    ret = vexfs_posix_path_lookup(req.path, &path);
    if (ret) {
        return ret;
    }

    inode = d_inode(path.dentry);
    if (!inode) {
        path_put(&path);
        return -ENOENT;
    }

    /* Find mapping for inode */
    down_read(&manager->mapping_lock);
    mapping = vexfs_posix_find_mapping_by_inode(manager, inode);
    if (!mapping) {
        up_read(&manager->mapping_lock);
        path_put(&path);
        pr_warn("VexFS-POSIX: No graph node found for path %s\n", req.path);
        return -ENOENT;
    }
    up_read(&manager->mapping_lock);

    /* Allocate API request and response */
    api_req = vexfs_api_request_alloc(manager->api_manager);
    if (!api_req) {
        atomic_dec(&mapping->ref_count);
        path_put(&path);
        return -ENOMEM;
    }

    api_resp = vexfs_api_response_alloc(manager->api_manager);
    if (!api_resp) {
        vexfs_api_request_free(manager->api_manager, api_req);
        atomic_dec(&mapping->ref_count);
        path_put(&path);
        return -ENOMEM;
    }

    /* Set up API request */
    api_req->operation = VEXFS_API_OP_NODE_DELETE;
    api_req->params.node_delete.node_id = mapping->graph_node_id;

    /* Delete graph node */
    ret = vexfs_api_node_delete(manager->api_manager, api_req, api_resp);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to delete graph node: %d\n", ret);
        goto out_free;
    }

    /* Remove mapping */
    vexfs_posix_remove_node_mapping(manager, inode);

    pr_info("VexFS-POSIX: Deleted graph node for path %s\n", req.path);

out_free:
    vexfs_api_response_free(manager->api_manager, api_resp);
    vexfs_api_request_free(manager->api_manager, api_req);
    atomic_dec(&mapping->ref_count);
    path_put(&path);
    return ret;
}

/**
 * vexfs_posix_ioctl_graph_create_edge - Create graph edge between filesystem paths
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_create_edge(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_edge_request __user *arg)
{
    struct vexfs_posix_graph_edge_request req;
    struct path source_path, target_path;
    struct inode *source_inode, *target_inode;
    struct vexfs_node_file_mapping *source_mapping, *target_mapping;
    struct vexfs_api_request *api_req;
    struct vexfs_api_response *api_resp;
    int ret;

    /* Copy request from user space */
    if (copy_from_user(&req, arg, sizeof(req))) {
        return -EFAULT;
    }

    req.source_path[PATH_MAX - 1] = '\0';
    req.target_path[PATH_MAX - 1] = '\0';
    req.properties_json[VEXFS_POSIX_MAX_PROPERTY_SIZE - 1] = '\0';

    pr_debug("VexFS-POSIX: Creating edge from %s to %s\n", req.source_path, req.target_path);

    /* Look up source path */
    ret = vexfs_posix_path_lookup(req.source_path, &source_path);
    if (ret) {
        return ret;
    }

    /* Look up target path */
    ret = vexfs_posix_path_lookup(req.target_path, &target_path);
    if (ret) {
        path_put(&source_path);
        return ret;
    }

    source_inode = d_inode(source_path.dentry);
    target_inode = d_inode(target_path.dentry);

    if (!source_inode || !target_inode) {
        ret = -ENOENT;
        goto out_put_paths;
    }

    /* Find mappings for both inodes */
    down_read(&manager->mapping_lock);
    source_mapping = vexfs_posix_find_mapping_by_inode(manager, source_inode);
    target_mapping = vexfs_posix_find_mapping_by_inode(manager, target_inode);
    up_read(&manager->mapping_lock);

    if (!source_mapping || !target_mapping) {
        pr_err("VexFS-POSIX: Missing graph nodes for edge creation\n");
        ret = -ENOENT;
        goto out_dec_refs;
    }

    /* Allocate API request and response */
    api_req = vexfs_api_request_alloc(manager->api_manager);
    if (!api_req) {
        ret = -ENOMEM;
        goto out_dec_refs;
    }

    api_resp = vexfs_api_response_alloc(manager->api_manager);
    if (!api_resp) {
        vexfs_api_request_free(manager->api_manager, api_req);
        ret = -ENOMEM;
        goto out_dec_refs;
    }

    /* Set up API request */
    api_req->operation = VEXFS_API_OP_EDGE_CREATE;
    api_req->params.edge_create.source_node_id = source_mapping->graph_node_id;
    api_req->params.edge_create.target_node_id = target_mapping->graph_node_id;
    api_req->params.edge_create.edge_type = req.edge_type;
    api_req->params.edge_create.weight = req.weight;
    api_req->params.edge_create.properties_json = req.properties_json;

    /* Create graph edge */
    ret = vexfs_api_edge_create(manager->api_manager, api_req, api_resp);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to create graph edge: %d\n", ret);
        goto out_free_api;
    }

    /* Copy edge ID back to user space */
    req.edge_id = api_resp->data.edge_create.edge_id;
    if (copy_to_user(arg, &req, sizeof(req))) {
        ret = -EFAULT;
        goto out_free_api;
    }

    pr_info("VexFS-POSIX: Created edge %llu from %s to %s\n",
            req.edge_id, req.source_path, req.target_path);

out_free_api:
    vexfs_api_response_free(manager->api_manager, api_resp);
    vexfs_api_request_free(manager->api_manager, api_req);
out_dec_refs:
    if (source_mapping) atomic_dec(&source_mapping->ref_count);
    if (target_mapping) atomic_dec(&target_mapping->ref_count);
out_put_paths:
    path_put(&target_path);
    path_put(&source_path);
    return ret;
}

/**
 * vexfs_posix_ioctl_graph_delete_edge - Delete graph edge between filesystem paths
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_delete_edge(struct vexfs_posix_integration_manager *manager,
                                         struct vexfs_posix_graph_edge_request __user *arg)
{
    /* Implementation similar to create_edge but for deletion */
    pr_debug("VexFS-POSIX: Delete edge ioctl called\n");
    
    /* TODO: Implement edge deletion logic */
    return -ENOSYS; /* Not implemented yet */
}

/**
 * vexfs_posix_ioctl_graph_query - Execute graph query using filesystem paths
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_query(struct vexfs_posix_integration_manager *manager,
                                  struct vexfs_posix_graph_query_request __user *arg)
{
    struct vexfs_posix_graph_query_request req;
    struct vexfs_api_request *api_req;
    struct vexfs_api_response *api_resp;
    int ret;

    /* Copy request from user space */
    if (copy_from_user(&req, arg, sizeof(req))) {
        return -EFAULT;
    }

    req.query_vql[sizeof(req.query_vql) - 1] = '\0';
    req.base_path[PATH_MAX - 1] = '\0';

    pr_debug("VexFS-POSIX: Executing graph query: %s\n", req.query_vql);

    /* Allocate API request and response */
    api_req = vexfs_api_request_alloc(manager->api_manager);
    if (!api_req) {
        return -ENOMEM;
    }

    api_resp = vexfs_api_response_alloc(manager->api_manager);
    if (!api_resp) {
        vexfs_api_request_free(manager->api_manager, api_req);
        return -ENOMEM;
    }

    /* Set up API request */
    api_req->operation = VEXFS_API_OP_QUERY;
    api_req->params.query.query_string = req.query_vql;
    api_req->params.query.max_results = req.max_results;

    /* Execute query */
    ret = vexfs_api_query_execute(manager->api_manager, api_req, api_resp);
    if (ret) {
        pr_err("VexFS-POSIX: Failed to execute graph query: %d\n", ret);
        goto out_free;
    }

    /* Copy results back to user space */
    req.result_count = api_resp->data.query.result_count;
    strncpy(req.results_json, api_resp->data.query.results_json, 
            sizeof(req.results_json) - 1);
    req.results_json[sizeof(req.results_json) - 1] = '\0';

    if (copy_to_user(arg, &req, sizeof(req))) {
        ret = -EFAULT;
        goto out_free;
    }

    pr_debug("VexFS-POSIX: Query returned %u results\n", req.result_count);

out_free:
    vexfs_api_response_free(manager->api_manager, api_resp);
    vexfs_api_request_free(manager->api_manager, api_req);
    return ret;
}

/**
 * vexfs_posix_ioctl_graph_traverse - Execute graph traversal from filesystem path
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_traverse(struct vexfs_posix_integration_manager *manager,
                                     struct vexfs_posix_graph_traversal_request __user *arg)
{
    /* Implementation for graph traversal starting from filesystem path */
    pr_debug("VexFS-POSIX: Graph traversal ioctl called\n");
    
    /* TODO: Implement traversal logic */
    return -ENOSYS; /* Not implemented yet */
}

/**
 * vexfs_posix_ioctl_graph_set_property - Set graph property through filesystem path
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_set_property(struct vexfs_posix_integration_manager *manager,
                                          struct vexfs_posix_graph_property_request __user *arg)
{
    /* Implementation for setting graph properties */
    pr_debug("VexFS-POSIX: Set property ioctl called\n");
    
    /* TODO: Implement property setting logic */
    return -ENOSYS; /* Not implemented yet */
}

/**
 * vexfs_posix_ioctl_graph_get_property - Get graph property through filesystem path
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_get_property(struct vexfs_posix_integration_manager *manager,
                                          struct vexfs_posix_graph_property_request __user *arg)
{
    /* Implementation for getting graph properties */
    pr_debug("VexFS-POSIX: Get property ioctl called\n");
    
    /* TODO: Implement property getting logic */
    return -ENOSYS; /* Not implemented yet */
}

/**
 * vexfs_posix_ioctl_graph_sync_view - Synchronize graph and filesystem views
 * @manager: Integration manager
 * @arg: User space argument
 */
long vexfs_posix_ioctl_graph_sync_view(struct vexfs_posix_integration_manager *manager,
                                       struct vexfs_posix_graph_sync_request __user *arg)
{
    /* Implementation for view synchronization */
    pr_debug("VexFS-POSIX: Sync view ioctl called\n");
    
    /* TODO: Implement view synchronization logic */
    return -ENOSYS; /* Not implemented yet */
}

/*
 * Helper Functions
 */

/**
 * vexfs_posix_path_lookup - Look up filesystem path
 * @path: Path string
 * @result: Resulting path structure
 */
static int vexfs_posix_path_lookup(const char *path, struct path *result)
{
    int ret;

    if (!path || !result) {
        return -EINVAL;
    }

    ret = kern_path(path, LOOKUP_FOLLOW, result);
    if (ret) {
        pr_debug("VexFS-POSIX: Failed to lookup path %s: %d\n", path, ret);
        return ret;
    }

    return 0;
}

/**
 * vexfs_posix_validate_ioctl_request - Validate ioctl request
 * @cmd: ioctl command
 * @arg: ioctl argument
 */
static int vexfs_posix_validate_ioctl_request(unsigned int cmd, unsigned long arg)
{
    /* Basic validation */
    if (!arg) {
        return -EINVAL;
    }

    /* Check if command is in valid range */
    if (_IOC_TYPE(cmd) != VEXFS_ENHANCED_IOC_MAGIC) {
        return -ENOTTY;
    }

    /* Additional validation based on command */
    switch (cmd) {
    case VEXFS_IOC_GRAPH_CREATE_NODE:
    case VEXFS_IOC_GRAPH_DELETE_NODE:
    case VEXFS_IOC_GRAPH_CREATE_EDGE:
    case VEXFS_IOC_GRAPH_DELETE_EDGE:
    case VEXFS_IOC_GRAPH_QUERY_NODE:
    case VEXFS_IOC_GRAPH_TRAVERSE:
    case VEXFS_IOC_GRAPH_SET_PROPERTY:
    case VEXFS_IOC_GRAPH_GET_PROPERTY:
    case VEXFS_IOC_GRAPH_SYNC_VIEW:
        /* Valid commands */
        break;
    default:
        return -ENOTTY;
    }

    return 0;
}

/**
 * vexfs_posix_copy_string_from_user - Safely copy string from user space
 * @dest: Destination buffer
 * @src: Source user space pointer
 * @max_len: Maximum length to copy
 */
static int vexfs_posix_copy_string_from_user(char *dest, const char __user *src, size_t max_len)
{
    long ret;

    if (!dest || !src || max_len == 0) {
        return -EINVAL;
    }

    ret = strncpy_from_user(dest, src, max_len);
    if (ret < 0) {
        return ret;
    }

    /* Ensure null termination */
    dest[max_len - 1] = '\0';
    return 0;
}

/* Module information */
MODULE_DESCRIPTION("VexFS v2.0 VexGraph POSIX ioctl Interface");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");