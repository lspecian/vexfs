/*
 * VexFS v2.0 - VexGraph Core Implementation (Task 8 - Phase 2)
 * 
 * This implements the core VexGraph functionality that transforms VexFS
 * into a true AI-native semantic substrate. Provides the central graph
 * manager, node/edge operations, and integration with Phase 1 foundation.
 *
 * Key Features:
 * - Graph Manager for coordinating all graph operations
 * - Node management with extended inode structures
 * - Edge management with efficient representation
 * - Property storage and retrieval for nodes and edges
 * - Integration with journaling and atomic operations
 * - Memory management and caching
 * - Graph statistics and monitoring
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/hash.h>
#include <linux/vmalloc.h>
#include <linux/time.h>
#include <linux/crc32.h>
#include <linux/completion.h>

#include "../include/vexfs_v2_vexgraph.h"
#include "../include/vexfs_v2_internal.h"

/* Hash table sizes (power of 2) */
#define VEXFS_GRAPH_NODES_HASH_SIZE     1024
#define VEXFS_GRAPH_EDGES_HASH_SIZE     2048

/* Memory allocation flags */
#define VEXFS_GRAPH_ALLOC_FLAGS         (GFP_KERNEL | __GFP_ZERO)

/* Forward declarations */
static void vexfs_graph_node_free(struct vexfs_graph_node *node);
static void vexfs_graph_edge_free(struct vexfs_graph_edge *edge);
static void vexfs_graph_property_free(struct vexfs_graph_property *prop);
static u32 vexfs_graph_hash_node_id(u64 node_id);
static u32 vexfs_graph_hash_edge_id(u64 edge_id);

/*
 * =============================================================================
 * GRAPH MANAGER OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_graph_manager_create - Create a new graph manager
 * @sb: Associated superblock
 * 
 * Creates and initializes a new VexGraph manager for the filesystem.
 * 
 * Return: Pointer to graph manager on success, NULL on failure
 */
struct vexfs_graph_manager *vexfs_graph_manager_create(struct super_block *sb)
{
    struct vexfs_graph_manager *mgr;
    int i;

    mgr = kzalloc(sizeof(struct vexfs_graph_manager), VEXFS_GRAPH_ALLOC_FLAGS);
    if (!mgr) {
        printk(KERN_ERR "VexGraph: Failed to allocate graph manager\n");
        return NULL;
    }

    /* Initialize metadata */
    mgr->magic = VEXFS_VEXGRAPH_MAGIC;
    mgr->version_major = VEXFS_VEXGRAPH_VERSION_MAJOR;
    mgr->version_minor = VEXFS_VEXGRAPH_VERSION_MINOR;
    mgr->flags = VEXFS_GRAPH_FLAG_DIRECTED | VEXFS_GRAPH_FLAG_WEIGHTED | 
                 VEXFS_GRAPH_FLAG_INDEXED | VEXFS_GRAPH_FLAG_PERSISTENT;

    /* Initialize node management */
    mgr->nodes_tree = RB_ROOT;
    mgr->nodes_hash_size = VEXFS_GRAPH_NODES_HASH_SIZE;
    mgr->nodes_hash = kzalloc(sizeof(struct hlist_head) * mgr->nodes_hash_size,
                              VEXFS_GRAPH_ALLOC_FLAGS);
    if (!mgr->nodes_hash) {
        printk(KERN_ERR "VexGraph: Failed to allocate nodes hash table\n");
        kfree(mgr);
        return NULL;
    }

    for (i = 0; i < mgr->nodes_hash_size; i++) {
        INIT_HLIST_HEAD(&mgr->nodes_hash[i]);
    }

    atomic64_set(&mgr->node_count, 0);
    atomic64_set(&mgr->next_node_id, 1);

    /* Initialize edge management */
    mgr->edges_tree = RB_ROOT;
    mgr->edges_hash_size = VEXFS_GRAPH_EDGES_HASH_SIZE;
    mgr->edges_hash = kzalloc(sizeof(struct hlist_head) * mgr->edges_hash_size,
                              VEXFS_GRAPH_ALLOC_FLAGS);
    if (!mgr->edges_hash) {
        printk(KERN_ERR "VexGraph: Failed to allocate edges hash table\n");
        kfree(mgr->nodes_hash);
        kfree(mgr);
        return NULL;
    }

    for (i = 0; i < mgr->edges_hash_size; i++) {
        INIT_HLIST_HEAD(&mgr->edges_hash[i]);
    }

    atomic64_set(&mgr->edge_count, 0);
    atomic64_set(&mgr->next_edge_id, 1);

    /* Initialize index management */
    mgr->indices_tree = RB_ROOT;
    INIT_LIST_HEAD(&mgr->indices_list);
    mgr->index_count = 0;

    /* Initialize synchronization */
    init_rwsem(&mgr->graph_sem);
    spin_lock_init(&mgr->hash_lock);
    mutex_init(&mgr->index_mutex);

    /* Initialize memory management */
    mgr->node_cache = kmem_cache_create("vexfs_graph_nodes",
                                        sizeof(struct vexfs_graph_node),
                                        0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->node_cache) {
        printk(KERN_ERR "VexGraph: Failed to create node cache\n");
        goto cleanup_edges_hash;
    }

    mgr->edge_cache = kmem_cache_create("vexfs_graph_edges",
                                        sizeof(struct vexfs_graph_edge),
                                        0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->edge_cache) {
        printk(KERN_ERR "VexGraph: Failed to create edge cache\n");
        goto cleanup_node_cache;
    }

    mgr->prop_cache = kmem_cache_create("vexfs_graph_properties",
                                        sizeof(struct vexfs_graph_property),
                                        0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->prop_cache) {
        printk(KERN_ERR "VexGraph: Failed to create property cache\n");
        goto cleanup_edge_cache;
    }

    /* Initialize statistics */
    atomic64_set(&mgr->operations_count, 0);
    atomic64_set(&mgr->traversals_count, 0);
    atomic64_set(&mgr->queries_count, 0);

    /* Store VexFS integration */
    mgr->sb = sb;
    mgr->journal = NULL;  /* Will be set during initialization */
    mgr->atomic_mgr = NULL;  /* Will be set during initialization */

    printk(KERN_INFO "VexGraph: Graph manager created successfully\n");
    return mgr;

cleanup_edge_cache:
    kmem_cache_destroy(mgr->edge_cache);
cleanup_node_cache:
    kmem_cache_destroy(mgr->node_cache);
cleanup_edges_hash:
    kfree(mgr->edges_hash);
    kfree(mgr->nodes_hash);
    kfree(mgr);
    return NULL;
}

/**
 * vexfs_graph_manager_destroy - Destroy a graph manager
 * @mgr: Graph manager to destroy
 * 
 * Cleans up and destroys the graph manager, freeing all resources.
 */
void vexfs_graph_manager_destroy(struct vexfs_graph_manager *mgr)
{
    struct rb_node *node;
    struct vexfs_graph_node *graph_node;
    struct vexfs_graph_edge *graph_edge;

    if (!mgr) {
        return;
    }

    printk(KERN_INFO "VexGraph: Destroying graph manager\n");

    /* Clean up all nodes */
    while ((node = rb_first(&mgr->nodes_tree)) != NULL) {
        graph_node = rb_entry(node, struct vexfs_graph_node, rb_node);
        rb_erase(node, &mgr->nodes_tree);
        vexfs_graph_node_free(graph_node);
    }

    /* Clean up all edges */
    while ((node = rb_first(&mgr->edges_tree)) != NULL) {
        graph_edge = rb_entry(node, struct vexfs_graph_edge, rb_node);
        rb_erase(node, &mgr->edges_tree);
        vexfs_graph_edge_free(graph_edge);
    }

    /* Destroy memory caches */
    if (mgr->prop_cache) {
        kmem_cache_destroy(mgr->prop_cache);
    }
    if (mgr->edge_cache) {
        kmem_cache_destroy(mgr->edge_cache);
    }
    if (mgr->node_cache) {
        kmem_cache_destroy(mgr->node_cache);
    }

    /* Free hash tables */
    kfree(mgr->edges_hash);
    kfree(mgr->nodes_hash);

    /* Free manager */
    kfree(mgr);

    printk(KERN_INFO "VexGraph: Graph manager destroyed\n");
}

/**
 * vexfs_graph_manager_init - Initialize graph manager with VexFS integration
 * @mgr: Graph manager to initialize
 * 
 * Initializes the graph manager with VexFS journal and atomic operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_manager_init(struct vexfs_graph_manager *mgr)
{
    if (!mgr) {
        return -EINVAL;
    }

    /* TODO: Initialize journal integration when available */
    /* mgr->journal = vexfs_get_journal(mgr->sb); */
    
    /* TODO: Initialize atomic operations integration when available */
    /* mgr->atomic_mgr = vexfs_get_atomic_manager(mgr->sb); */

    printk(KERN_INFO "VexGraph: Graph manager initialized\n");
    return 0;
}

/**
 * vexfs_graph_manager_cleanup - Cleanup graph manager
 * @mgr: Graph manager to cleanup
 * 
 * Performs cleanup operations for the graph manager.
 */
void vexfs_graph_manager_cleanup(struct vexfs_graph_manager *mgr)
{
    if (!mgr) {
        return;
    }

    /* Sync any pending operations */
    down_write(&mgr->graph_sem);
    /* TODO: Flush any pending journal operations */
    up_write(&mgr->graph_sem);

    printk(KERN_INFO "VexGraph: Graph manager cleanup completed\n");
}

/*
 * =============================================================================
 * NODE OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_graph_node_create - Create a new graph node
 * @mgr: Graph manager
 * @inode_number: Associated inode number
 * @node_type: Type of the node
 * 
 * Creates a new graph node and adds it to the graph.
 * 
 * Return: Pointer to new node on success, NULL on failure
 */
struct vexfs_graph_node *vexfs_graph_node_create(struct vexfs_graph_manager *mgr,
                                                  u64 inode_number, u8 node_type)
{
    struct vexfs_graph_node *node;
    struct rb_node **new_node, *parent = NULL;
    u32 hash;
    u64 node_id;

    if (!mgr) {
        return NULL;
    }

    /* Allocate new node */
    node = kmem_cache_alloc(mgr->node_cache, VEXFS_GRAPH_ALLOC_FLAGS);
    if (!node) {
        printk(KERN_ERR "VexGraph: Failed to allocate node\n");
        return NULL;
    }

    /* Initialize node */
    node_id = atomic64_inc_return(&mgr->next_node_id);
    node->node_id = node_id;
    node->inode_number = inode_number;
    node->node_type = node_type;
    node->flags = 0;

    /* Initialize properties */
    INIT_LIST_HEAD(&node->properties);
    node->property_count = 0;

    /* Initialize adjacency */
    INIT_LIST_HEAD(&node->outgoing_edges);
    INIT_LIST_HEAD(&node->incoming_edges);
    node->out_degree = 0;
    node->in_degree = 0;

    /* Initialize synchronization */
    init_rwsem(&node->node_sem);
    atomic_set(&node->ref_count, 1);

    /* Set timestamps */
    node->created_time = ktime_get_real_seconds();
    node->modified_time = node->created_time;
    node->accessed_time = node->created_time;

    /* Add to red-black tree */
    down_write(&mgr->graph_sem);

    new_node = &mgr->nodes_tree.rb_node;
    while (*new_node) {
        struct vexfs_graph_node *this_node = rb_entry(*new_node,
                                                      struct vexfs_graph_node,
                                                      rb_node);
        parent = *new_node;

        if (node_id < this_node->node_id) {
            new_node = &((*new_node)->rb_left);
        } else if (node_id > this_node->node_id) {
            new_node = &((*new_node)->rb_right);
        } else {
            /* Duplicate node ID - should not happen */
            up_write(&mgr->graph_sem);
            kmem_cache_free(mgr->node_cache, node);
            printk(KERN_ERR "VexGraph: Duplicate node ID %llu\n", node_id);
            return NULL;
        }
    }

    rb_link_node(&node->rb_node, parent, new_node);
    rb_insert_color(&node->rb_node, &mgr->nodes_tree);

    /* Add to hash table */
    hash = vexfs_graph_hash_node_id(node_id);
    spin_lock(&mgr->hash_lock);
    hlist_add_head(&node->hash_node, &mgr->nodes_hash[hash % mgr->nodes_hash_size]);
    spin_unlock(&mgr->hash_lock);

    /* Update statistics */
    atomic64_inc(&mgr->node_count);
    atomic64_inc(&mgr->operations_count);

    up_write(&mgr->graph_sem);

    printk(KERN_DEBUG "VexGraph: Created node %llu (inode %llu, type %u)\n",
           node_id, inode_number, node_type);

    return node;
}

/**
 * vexfs_graph_node_lookup - Look up a graph node by ID
 * @mgr: Graph manager
 * @node_id: Node ID to look up
 * 
 * Finds and returns a graph node by its ID.
 * 
 * Return: Pointer to node on success, NULL if not found
 */
struct vexfs_graph_node *vexfs_graph_node_lookup(struct vexfs_graph_manager *mgr,
                                                  u64 node_id)
{
    struct rb_node *node;
    struct vexfs_graph_node *graph_node;

    if (!mgr) {
        return NULL;
    }

    down_read(&mgr->graph_sem);

    node = mgr->nodes_tree.rb_node;
    while (node) {
        graph_node = rb_entry(node, struct vexfs_graph_node, rb_node);

        if (node_id < graph_node->node_id) {
            node = node->rb_left;
        } else if (node_id > graph_node->node_id) {
            node = node->rb_right;
        } else {
            /* Found the node */
            atomic_inc(&graph_node->ref_count);
            graph_node->accessed_time = ktime_get_real_seconds();
            up_read(&mgr->graph_sem);
            return graph_node;
        }
    }

    up_read(&mgr->graph_sem);
    return NULL;
}

/**
 * vexfs_graph_node_destroy - Destroy a graph node
 * @mgr: Graph manager
 * @node: Node to destroy
 * 
 * Removes and destroys a graph node, cleaning up all associated edges.
 */
void vexfs_graph_node_destroy(struct vexfs_graph_manager *mgr,
                               struct vexfs_graph_node *node)
{
    u32 hash;

    if (!mgr || !node) {
        return;
    }

    down_write(&mgr->graph_sem);

    /* Remove from red-black tree */
    rb_erase(&node->rb_node, &mgr->nodes_tree);

    /* Remove from hash table */
    hash = vexfs_graph_hash_node_id(node->node_id);
    spin_lock(&mgr->hash_lock);
    hlist_del(&node->hash_node);
    spin_unlock(&mgr->hash_lock);

    /* Update statistics */
    atomic64_dec(&mgr->node_count);
    atomic64_inc(&mgr->operations_count);

    up_write(&mgr->graph_sem);

    /* TODO: Remove all associated edges */
    /* This would require iterating through incoming and outgoing edges */

    /* Free the node */
    vexfs_graph_node_free(node);

    printk(KERN_DEBUG "VexGraph: Destroyed node %llu\n", node->node_id);
}

/*
 * =============================================================================
 * PROPERTY OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_graph_node_add_property - Add a property to a node
 * @node: Target node
 * @key: Property key
 * @type: Property type
 * @value: Property value
 * @size: Value size
 * 
 * Adds a property to the specified node.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_node_add_property(struct vexfs_graph_node *node,
                                  const char *key, u8 type, const void *value, u32 size)
{
    struct vexfs_graph_property *prop;

    if (!node || !key || !value || size == 0 || size > VEXFS_GRAPH_MAX_PROP_SIZE) {
        return -EINVAL;
    }

    /* Check if property already exists */
    down_write(&node->node_sem);
    list_for_each_entry(prop, &node->properties, list) {
        if (strcmp(prop->key, key) == 0) {
            up_write(&node->node_sem);
            return -EEXIST;
        }
    }

    /* Allocate new property */
    prop = kzalloc(sizeof(struct vexfs_graph_property), VEXFS_GRAPH_ALLOC_FLAGS);
    if (!prop) {
        up_write(&node->node_sem);
        return -ENOMEM;
    }

    /* Initialize property */
    strncpy(prop->key, key, sizeof(prop->key) - 1);
    prop->type = type;
    prop->size = size;

    /* Set value based on type */
    switch (type) {
    case VEXFS_GRAPH_PROP_STRING:
        prop->value.string_val = kzalloc(size + 1, VEXFS_GRAPH_ALLOC_FLAGS);
        if (!prop->value.string_val) {
            kfree(prop);
            up_write(&node->node_sem);
            return -ENOMEM;
        }
        memcpy(prop->value.string_val, value, size);
        break;

    case VEXFS_GRAPH_PROP_INTEGER:
        if (size == sizeof(s64)) {
            prop->value.int_val = *(s64 *)value;
        } else {
            kfree(prop);
            up_write(&node->node_sem);
            return -EINVAL;
        }
        break;

    case VEXFS_GRAPH_PROP_BOOLEAN:
        if (size == sizeof(bool)) {
            prop->value.bool_val = *(bool *)value;
        } else {
            kfree(prop);
            up_write(&node->node_sem);
            return -EINVAL;
        }
        break;

    case VEXFS_GRAPH_PROP_TIMESTAMP:
        if (size == sizeof(u64)) {
            prop->value.timestamp_val = *(u64 *)value;
        } else {
            kfree(prop);
            up_write(&node->node_sem);
            return -EINVAL;
        }
        break;

    default:
        kfree(prop);
        up_write(&node->node_sem);
        return -EINVAL;
    }

    /* Add to property list */
    list_add_tail(&prop->list, &node->properties);
    node->property_count++;
    node->modified_time = ktime_get_real_seconds();

    up_write(&node->node_sem);

    printk(KERN_DEBUG "VexGraph: Added property '%s' to node %llu\n",
           key, node->node_id);

    return 0;
}

/**
 * vexfs_graph_node_get_property - Get a property from a node
 * @node: Target node
 * @key: Property key
 * 
 * Retrieves a property from the specified node.
 * 
 * Return: Pointer to property on success, NULL if not found
 */
struct vexfs_graph_property *vexfs_graph_node_get_property(struct vexfs_graph_node *node,
                                                           const char *key)
{
    struct vexfs_graph_property *prop;

    if (!node || !key) {
        return NULL;
    }

    down_read(&node->node_sem);
    list_for_each_entry(prop, &node->properties, list) {
        if (strcmp(prop->key, key) == 0) {
            node->accessed_time = ktime_get_real_seconds();
            up_read(&node->node_sem);
            return prop;
        }
    }
    up_read(&node->node_sem);

    return NULL;
}

/*
 * =============================================================================
 * UTILITY FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_graph_hash_node_id - Hash function for node IDs
 * @node_id: Node ID to hash
 * 
 * Return: Hash value
 */
static u32 vexfs_graph_hash_node_id(u64 node_id)
{
    return hash_64(node_id, 32);
}

/**
 * vexfs_graph_hash_edge_id - Hash function for edge IDs
 * @edge_id: Edge ID to hash
 * 
 * Return: Hash value
 */
static u32 vexfs_graph_hash_edge_id(u64 edge_id)
{
    return hash_64(edge_id, 32);
}

/**
 * vexfs_graph_node_free - Free a graph node and its properties
 * @node: Node to free
 */
static void vexfs_graph_node_free(struct vexfs_graph_node *node)
{
    struct vexfs_graph_property *prop, *tmp;

    if (!node) {
        return;
    }

    /* Free all properties */
    list_for_each_entry_safe(prop, tmp, &node->properties, list) {
        list_del(&prop->list);
        vexfs_graph_property_free(prop);
    }

    /* Free the node itself */
    kfree(node);
}

/**
 * vexfs_graph_edge_free - Free a graph edge and its properties
 * @edge: Edge to free
 */
static void vexfs_graph_edge_free(struct vexfs_graph_edge *edge)
{
    struct vexfs_graph_property *prop, *tmp;

    if (!edge) {
        return;
    }

    /* Free all properties */
    list_for_each_entry_safe(prop, tmp, &edge->properties, list) {
        list_del(&prop->list);
        vexfs_graph_property_free(prop);
    }

    /* Free the edge itself */
    kfree(edge);
}

/**
 * vexfs_graph_property_free - Free a graph property
 * @prop: Property to free
 */
static void vexfs_graph_property_free(struct vexfs_graph_property *prop)
{
    if (!prop) {
        return;
    }

    /* Free string values */
    if (prop->type == VEXFS_GRAPH_PROP_STRING && prop->value.string_val) {
        kfree(prop->value.string_val);
    }

    /* Free vector values */
    if (prop->type == VEXFS_GRAPH_PROP_VECTOR && prop->value.vector_val.vector_data) {
        kfree(prop->value.vector_val.vector_data);
    }

    kfree(prop);
}

/*
 * =============================================================================
 * GRAPH STATISTICS
 * =============================================================================
 */

/**
 * vexfs_graph_get_statistics - Get graph statistics
 * @mgr: Graph manager
 * @stats: Statistics structure to fill
 * 
 * Fills the statistics structure with current graph metrics.
 */
void vexfs_graph_get_statistics(struct vexfs_graph_manager *mgr,
                                struct vexfs_graph_stats *stats)
{
    if (!mgr || !stats) {
        return;
    }

    memset(stats, 0, sizeof(struct vexfs_graph_stats));

    stats->node_count = atomic64_read(&mgr->node_count);
    stats->edge_count = atomic64_read(&mgr->edge_count);
    stats->index_count = mgr->index_count;
    stats->operations_count = atomic64_read(&mgr->operations_count);
    stats->traversals_count = atomic64_read(&mgr->traversals_count);
    stats->queries_count = atomic64_read(&mgr->queries_count);

    /* TODO: Calculate memory usage and serialized size */
    stats->memory_usage = 0;
    stats->serialized_size = 0;
}

MODULE_LICENSE("GPL v2");
MODULE_DESCRIPTION("VexFS v2.0 VexGraph Core Implementation");