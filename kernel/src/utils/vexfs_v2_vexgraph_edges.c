/*
 * VexFS v2.0 - VexGraph Edge Operations (Task 8 - Phase 2)
 * 
 * This implements edge management and graph traversal algorithms for VexGraph.
 * Provides efficient edge creation, deletion, and traversal operations that
 * enable native graph capabilities within VexFS.
 *
 * Key Features:
 * - Edge creation and management with properties
 * - Graph traversal algorithms (BFS, DFS)
 * - Shortest path algorithms (Dijkstra)
 * - Edge indexing and lookup optimization
 * - Integration with journaling for consistency
 * - Memory-efficient edge representation
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
#include <linux/completion.h>
#include <linux/workqueue.h>

#include "../include/vexfs_v2_vexgraph.h"
#include "../include/vexfs_v2_internal.h"

/* Priority queue node for Dijkstra's algorithm */
struct vexfs_graph_pq_node {
    u64 node_id;
    u32 distance;
    struct list_head list;
};

/* Forward declarations */
static int vexfs_graph_edge_insert_tree(struct vexfs_graph_manager *mgr,
                                        struct vexfs_graph_edge *edge);
static void vexfs_graph_edge_remove_tree(struct vexfs_graph_manager *mgr,
                                         struct vexfs_graph_edge *edge);
static struct vexfs_graph_pq_node *vexfs_graph_pq_extract_min(struct list_head *pq);
static void vexfs_graph_pq_insert(struct list_head *pq, u64 node_id, u32 distance);

/*
 * =============================================================================
 * EDGE OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_graph_edge_create - Create a new graph edge
 * @mgr: Graph manager
 * @source_id: Source node ID
 * @target_id: Target node ID
 * @edge_type: Type of the edge
 * @weight: Edge weight
 * 
 * Creates a new graph edge between two nodes.
 * 
 * Return: Pointer to new edge on success, NULL on failure
 */
struct vexfs_graph_edge *vexfs_graph_edge_create(struct vexfs_graph_manager *mgr,
                                                  u64 source_id, u64 target_id,
                                                  u8 edge_type, u32 weight)
{
    struct vexfs_graph_edge *edge;
    struct vexfs_graph_node *source_node, *target_node;
    u32 hash;
    u64 edge_id;

    if (!mgr || source_id == target_id) {
        return NULL;
    }

    /* Look up source and target nodes */
    source_node = vexfs_graph_node_lookup(mgr, source_id);
    if (!source_node) {
        printk(KERN_ERR "VexGraph: Source node %llu not found\n", source_id);
        return NULL;
    }

    target_node = vexfs_graph_node_lookup(mgr, target_id);
    if (!target_node) {
        printk(KERN_ERR "VexGraph: Target node %llu not found\n", target_id);
        /* Decrement source node reference */
        atomic_dec(&source_node->ref_count);
        return NULL;
    }

    /* Allocate new edge */
    edge = kmem_cache_alloc(mgr->edge_cache, GFP_KERNEL | __GFP_ZERO);
    if (!edge) {
        printk(KERN_ERR "VexGraph: Failed to allocate edge\n");
        atomic_dec(&source_node->ref_count);
        atomic_dec(&target_node->ref_count);
        return NULL;
    }

    /* Initialize edge */
    edge_id = atomic64_inc_return(&mgr->next_edge_id);
    edge->edge_id = edge_id;
    edge->source_node_id = source_id;
    edge->target_node_id = target_id;
    edge->edge_type = edge_type;
    edge->weight = weight;
    edge->flags = 0;

    /* Initialize properties */
    INIT_LIST_HEAD(&edge->properties);
    edge->property_count = 0;

    /* Initialize synchronization */
    spin_lock_init(&edge->edge_lock);
    atomic_set(&edge->ref_count, 1);

    /* Set timestamps */
    edge->created_time = ktime_get_real_seconds();
    edge->modified_time = edge->created_time;

    /* Add to graph manager */
    down_write(&mgr->graph_sem);

    /* Insert into red-black tree */
    if (vexfs_graph_edge_insert_tree(mgr, edge) != 0) {
        up_write(&mgr->graph_sem);
        kmem_cache_free(mgr->edge_cache, edge);
        atomic_dec(&source_node->ref_count);
        atomic_dec(&target_node->ref_count);
        return NULL;
    }

    /* Add to hash table */
    hash = hash_64(edge_id, 32);
    spin_lock(&mgr->hash_lock);
    hlist_add_head(&edge->hash_node, &mgr->edges_hash[hash % mgr->edges_hash_size]);
    spin_unlock(&mgr->hash_lock);

    /* Add to node adjacency lists */
    down_write(&source_node->node_sem);
    list_add_tail(&edge->source_list, &source_node->outgoing_edges);
    source_node->out_degree++;
    source_node->modified_time = ktime_get_real_seconds();
    up_write(&source_node->node_sem);

    down_write(&target_node->node_sem);
    list_add_tail(&edge->target_list, &target_node->incoming_edges);
    target_node->in_degree++;
    target_node->modified_time = ktime_get_real_seconds();
    up_write(&target_node->node_sem);

    /* Update statistics */
    atomic64_inc(&mgr->edge_count);
    atomic64_inc(&mgr->operations_count);

    up_write(&mgr->graph_sem);

    /* Decrement node references (we don't hold them long-term) */
    atomic_dec(&source_node->ref_count);
    atomic_dec(&target_node->ref_count);

    printk(KERN_DEBUG "VexGraph: Created edge %llu (%llu -> %llu, type %u, weight %u)\n",
           edge_id, source_id, target_id, edge_type, weight);

    return edge;
}

/**
 * vexfs_graph_edge_lookup - Look up a graph edge by ID
 * @mgr: Graph manager
 * @edge_id: Edge ID to look up
 * 
 * Finds and returns a graph edge by its ID.
 * 
 * Return: Pointer to edge on success, NULL if not found
 */
struct vexfs_graph_edge *vexfs_graph_edge_lookup(struct vexfs_graph_manager *mgr,
                                                  u64 edge_id)
{
    struct rb_node *node;
    struct vexfs_graph_edge *graph_edge;

    if (!mgr) {
        return NULL;
    }

    down_read(&mgr->graph_sem);

    node = mgr->edges_tree.rb_node;
    while (node) {
        graph_edge = rb_entry(node, struct vexfs_graph_edge, rb_node);

        if (edge_id < graph_edge->edge_id) {
            node = node->rb_left;
        } else if (edge_id > graph_edge->edge_id) {
            node = node->rb_right;
        } else {
            /* Found the edge */
            atomic_inc(&graph_edge->ref_count);
            up_read(&mgr->graph_sem);
            return graph_edge;
        }
    }

    up_read(&mgr->graph_sem);
    return NULL;
}

/**
 * vexfs_graph_edge_destroy - Destroy a graph edge
 * @mgr: Graph manager
 * @edge: Edge to destroy
 * 
 * Removes and destroys a graph edge.
 */
void vexfs_graph_edge_destroy(struct vexfs_graph_manager *mgr,
                               struct vexfs_graph_edge *edge)
{
    struct vexfs_graph_node *source_node, *target_node;
    u32 hash;

    if (!mgr || !edge) {
        return;
    }

    /* Look up nodes to update adjacency lists */
    source_node = vexfs_graph_node_lookup(mgr, edge->source_node_id);
    target_node = vexfs_graph_node_lookup(mgr, edge->target_node_id);

    down_write(&mgr->graph_sem);

    /* Remove from red-black tree */
    vexfs_graph_edge_remove_tree(mgr, edge);

    /* Remove from hash table */
    hash = hash_64(edge->edge_id, 32);
    spin_lock(&mgr->hash_lock);
    hlist_del(&edge->hash_node);
    spin_unlock(&mgr->hash_lock);

    /* Update statistics */
    atomic64_dec(&mgr->edge_count);
    atomic64_inc(&mgr->operations_count);

    up_write(&mgr->graph_sem);

    /* Remove from node adjacency lists */
    if (source_node) {
        down_write(&source_node->node_sem);
        list_del(&edge->source_list);
        source_node->out_degree--;
        source_node->modified_time = ktime_get_real_seconds();
        up_write(&source_node->node_sem);
        atomic_dec(&source_node->ref_count);
    }

    if (target_node) {
        down_write(&target_node->node_sem);
        list_del(&edge->target_list);
        target_node->in_degree--;
        target_node->modified_time = ktime_get_real_seconds();
        up_write(&target_node->node_sem);
        atomic_dec(&target_node->ref_count);
    }

    /* Free the edge */
    kmem_cache_free(mgr->edge_cache, edge);

    printk(KERN_DEBUG "VexGraph: Destroyed edge %llu\n", edge->edge_id);
}

/*
 * =============================================================================
 * GRAPH TRAVERSAL ALGORITHMS
 * =============================================================================
 */

/**
 * vexfs_graph_traverse_bfs - Breadth-First Search traversal
 * @mgr: Graph manager
 * @ctx: Query context with traversal parameters
 * 
 * Performs BFS traversal starting from the specified node.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_traverse_bfs(struct vexfs_graph_manager *mgr,
                             struct vexfs_graph_query_context *ctx)
{
    struct list_head queue;
    struct vexfs_graph_node *current_node, *neighbor_node;
    struct vexfs_graph_edge *edge;
    u64 *queue_nodes;
    u32 queue_head = 0, queue_tail = 0;
    u32 current_depth = 0;
    u32 result_count = 0;

    if (!mgr || !ctx || ctx->max_results == 0) {
        return -EINVAL;
    }

    /* Allocate queue for BFS */
    queue_nodes = kzalloc(sizeof(u64) * ctx->max_results, GFP_KERNEL);
    if (!queue_nodes) {
        return -ENOMEM;
    }

    /* Allocate visited bitmap */
    ctx->visited_nodes = kzalloc(BITS_TO_LONGS(VEXFS_GRAPH_MAX_NODES) * sizeof(long),
                                 GFP_KERNEL);
    if (!ctx->visited_nodes) {
        kfree(queue_nodes);
        return -ENOMEM;
    }

    /* Initialize BFS */
    queue_nodes[queue_tail++] = ctx->start_node_id;
    set_bit(ctx->start_node_id % VEXFS_GRAPH_MAX_NODES, ctx->visited_nodes);

    down_read(&mgr->graph_sem);

    while (queue_head < queue_tail && current_depth < ctx->max_depth && 
           result_count < ctx->max_results) {
        
        u64 current_node_id = queue_nodes[queue_head++];
        
        /* Add to results */
        if (result_count < ctx->max_results) {
            ctx->result_nodes[result_count++] = current_node_id;
        }

        /* Look up current node */
        current_node = vexfs_graph_node_lookup(mgr, current_node_id);
        if (!current_node) {
            continue;
        }

        /* Traverse outgoing edges */
        down_read(&current_node->node_sem);
        list_for_each_entry(edge, &current_node->outgoing_edges, source_list) {
            u64 neighbor_id = edge->target_node_id;
            
            /* Apply filters */
            if (ctx->edge_type_filter != 0 && edge->edge_type != ctx->edge_type_filter) {
                continue;
            }

            /* Check if already visited */
            if (test_bit(neighbor_id % VEXFS_GRAPH_MAX_NODES, ctx->visited_nodes)) {
                continue;
            }

            /* Add to queue if space available */
            if (queue_tail < ctx->max_results) {
                queue_nodes[queue_tail++] = neighbor_id;
                set_bit(neighbor_id % VEXFS_GRAPH_MAX_NODES, ctx->visited_nodes);
            }
        }
        up_read(&current_node->node_sem);

        atomic_dec(&current_node->ref_count);
        current_depth++;
    }

    up_read(&mgr->graph_sem);

    ctx->result_count = result_count;
    atomic64_inc(&mgr->traversals_count);

    kfree(queue_nodes);

    printk(KERN_DEBUG "VexGraph: BFS traversal completed, %u nodes visited\n", result_count);
    return 0;
}

/**
 * vexfs_graph_traverse_dfs - Depth-First Search traversal
 * @mgr: Graph manager
 * @ctx: Query context with traversal parameters
 * 
 * Performs DFS traversal starting from the specified node.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_traverse_dfs(struct vexfs_graph_manager *mgr,
                             struct vexfs_graph_query_context *ctx)
{
    struct vexfs_graph_node *current_node;
    struct vexfs_graph_edge *edge;
    u64 *stack_nodes;
    u32 stack_top = 0;
    u32 current_depth = 0;
    u32 result_count = 0;

    if (!mgr || !ctx || ctx->max_results == 0) {
        return -EINVAL;
    }

    /* Allocate stack for DFS */
    stack_nodes = kzalloc(sizeof(u64) * ctx->max_results, GFP_KERNEL);
    if (!stack_nodes) {
        return -ENOMEM;
    }

    /* Allocate visited bitmap */
    ctx->visited_nodes = kzalloc(BITS_TO_LONGS(VEXFS_GRAPH_MAX_NODES) * sizeof(long),
                                 GFP_KERNEL);
    if (!ctx->visited_nodes) {
        kfree(stack_nodes);
        return -ENOMEM;
    }

    /* Initialize DFS */
    stack_nodes[stack_top++] = ctx->start_node_id;

    down_read(&mgr->graph_sem);

    while (stack_top > 0 && current_depth < ctx->max_depth && 
           result_count < ctx->max_results) {
        
        u64 current_node_id = stack_nodes[--stack_top];
        
        /* Check if already visited */
        if (test_bit(current_node_id % VEXFS_GRAPH_MAX_NODES, ctx->visited_nodes)) {
            continue;
        }

        /* Mark as visited */
        set_bit(current_node_id % VEXFS_GRAPH_MAX_NODES, ctx->visited_nodes);

        /* Add to results */
        if (result_count < ctx->max_results) {
            ctx->result_nodes[result_count++] = current_node_id;
        }

        /* Look up current node */
        current_node = vexfs_graph_node_lookup(mgr, current_node_id);
        if (!current_node) {
            continue;
        }

        /* Traverse outgoing edges (in reverse order for DFS) */
        down_read(&current_node->node_sem);
        list_for_each_entry_reverse(edge, &current_node->outgoing_edges, source_list) {
            u64 neighbor_id = edge->target_node_id;
            
            /* Apply filters */
            if (ctx->edge_type_filter != 0 && edge->edge_type != ctx->edge_type_filter) {
                continue;
            }

            /* Check if already visited */
            if (test_bit(neighbor_id % VEXFS_GRAPH_MAX_NODES, ctx->visited_nodes)) {
                continue;
            }

            /* Add to stack if space available */
            if (stack_top < ctx->max_results) {
                stack_nodes[stack_top++] = neighbor_id;
            }
        }
        up_read(&current_node->node_sem);

        atomic_dec(&current_node->ref_count);
        current_depth++;
    }

    up_read(&mgr->graph_sem);

    ctx->result_count = result_count;
    atomic64_inc(&mgr->traversals_count);

    kfree(stack_nodes);

    printk(KERN_DEBUG "VexGraph: DFS traversal completed, %u nodes visited\n", result_count);
    return 0;
}

/**
 * vexfs_graph_shortest_path - Find shortest path between two nodes
 * @mgr: Graph manager
 * @source_id: Source node ID
 * @target_id: Target node ID
 * @path: Array to store path node IDs
 * @path_length: Pointer to store path length
 * 
 * Uses Dijkstra's algorithm to find the shortest path.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_graph_shortest_path(struct vexfs_graph_manager *mgr,
                              u64 source_id, u64 target_id,
                              u64 *path, u32 *path_length)
{
    struct list_head priority_queue;
    struct vexfs_graph_pq_node *pq_node;
    struct vexfs_graph_node *current_node;
    struct vexfs_graph_edge *edge;
    u32 *distances;
    u64 *predecessors;
    bool *visited;
    u32 current_distance;
    u64 current_node_id;
    int ret = 0;

    if (!mgr || !path || !path_length || source_id == target_id) {
        return -EINVAL;
    }

    /* Allocate arrays for Dijkstra's algorithm */
    distances = kzalloc(sizeof(u32) * VEXFS_GRAPH_MAX_NODES, GFP_KERNEL);
    predecessors = kzalloc(sizeof(u64) * VEXFS_GRAPH_MAX_NODES, GFP_KERNEL);
    visited = kzalloc(sizeof(bool) * VEXFS_GRAPH_MAX_NODES, GFP_KERNEL);

    if (!distances || !predecessors || !visited) {
        ret = -ENOMEM;
        goto cleanup;
    }

    /* Initialize distances to infinity */
    memset(distances, 0xFF, sizeof(u32) * VEXFS_GRAPH_MAX_NODES);
    distances[source_id % VEXFS_GRAPH_MAX_NODES] = 0;

    /* Initialize priority queue */
    INIT_LIST_HEAD(&priority_queue);
    vexfs_graph_pq_insert(&priority_queue, source_id, 0);

    down_read(&mgr->graph_sem);

    while (!list_empty(&priority_queue)) {
        /* Extract minimum distance node */
        pq_node = vexfs_graph_pq_extract_min(&priority_queue);
        if (!pq_node) {
            break;
        }

        current_node_id = pq_node->node_id;
        current_distance = pq_node->distance;
        kfree(pq_node);

        /* Check if we reached the target */
        if (current_node_id == target_id) {
            break;
        }

        /* Skip if already visited */
        if (visited[current_node_id % VEXFS_GRAPH_MAX_NODES]) {
            continue;
        }

        visited[current_node_id % VEXFS_GRAPH_MAX_NODES] = true;

        /* Look up current node */
        current_node = vexfs_graph_node_lookup(mgr, current_node_id);
        if (!current_node) {
            continue;
        }

        /* Relax edges */
        down_read(&current_node->node_sem);
        list_for_each_entry(edge, &current_node->outgoing_edges, source_list) {
            u64 neighbor_id = edge->target_node_id;
            u32 neighbor_idx = neighbor_id % VEXFS_GRAPH_MAX_NODES;
            u32 new_distance = current_distance + edge->weight;

            if (!visited[neighbor_idx] && new_distance < distances[neighbor_idx]) {
                distances[neighbor_idx] = new_distance;
                predecessors[neighbor_idx] = current_node_id;
                vexfs_graph_pq_insert(&priority_queue, neighbor_id, new_distance);
            }
        }
        up_read(&current_node->node_sem);

        atomic_dec(&current_node->ref_count);
    }

    up_read(&mgr->graph_sem);

    /* Reconstruct path */
    if (distances[target_id % VEXFS_GRAPH_MAX_NODES] != 0xFFFFFFFF) {
        u32 path_idx = 0;
        u64 current = target_id;

        /* Build path in reverse */
        while (current != source_id && path_idx < *path_length) {
            path[path_idx++] = current;
            current = predecessors[current % VEXFS_GRAPH_MAX_NODES];
        }
        path[path_idx++] = source_id;

        /* Reverse the path */
        for (u32 i = 0; i < path_idx / 2; i++) {
            u64 temp = path[i];
            path[i] = path[path_idx - 1 - i];
            path[path_idx - 1 - i] = temp;
        }

        *path_length = path_idx;
    } else {
        *path_length = 0;
        ret = -ENOENT;  /* No path found */
    }

    /* Clean up remaining priority queue nodes */
    while (!list_empty(&priority_queue)) {
        pq_node = list_first_entry(&priority_queue, struct vexfs_graph_pq_node, list);
        list_del(&pq_node->list);
        kfree(pq_node);
    }

cleanup:
    kfree(visited);
    kfree(predecessors);
    kfree(distances);

    if (ret == 0) {
        printk(KERN_DEBUG "VexGraph: Shortest path found, length %u\n", *path_length);
    }

    return ret;
}

/*
 * =============================================================================
 * UTILITY FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_graph_edge_insert_tree - Insert edge into red-black tree
 * @mgr: Graph manager
 * @edge: Edge to insert
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_graph_edge_insert_tree(struct vexfs_graph_manager *mgr,
                                        struct vexfs_graph_edge *edge)
{
    struct rb_node **new_node = &mgr->edges_tree.rb_node;
    struct rb_node *parent = NULL;

    while (*new_node) {
        struct vexfs_graph_edge *this_edge = rb_entry(*new_node,
                                                      struct vexfs_graph_edge,
                                                      rb_node);
        parent = *new_node;

        if (edge->edge_id < this_edge->edge_id) {
            new_node = &((*new_node)->rb_left);
        } else if (edge->edge_id > this_edge->edge_id) {
            new_node = &((*new_node)->rb_right);
        } else {
            /* Duplicate edge ID */
            return -EEXIST;
        }
    }

    rb_link_node(&edge->rb_node, parent, new_node);
    rb_insert_color(&edge->rb_node, &mgr->edges_tree);

    return 0;
}

/**
 * vexfs_graph_edge_remove_tree - Remove edge from red-black tree
 * @mgr: Graph manager
 * @edge: Edge to remove
 */
static void vexfs_graph_edge_remove_tree(struct vexfs_graph_manager *mgr,
                                         struct vexfs_graph_edge *edge)
{
    rb_erase(&edge->rb_node, &mgr->edges_tree);
}

/**
 * vexfs_graph_pq_insert - Insert node into priority queue
 * @pq: Priority queue head
 * @node_id: Node ID
 * @distance: Distance value
 */
static void vexfs_graph_pq_insert(struct list_head *pq, u64 node_id, u32 distance)
{
    struct vexfs_graph_pq_node *new_node, *pos;

    new_node = kzalloc(sizeof(struct vexfs_graph_pq_node), GFP_KERNEL);
    if (!new_node) {
        return;
    }

    new_node->node_id = node_id;
    new_node->distance = distance;

    /* Insert in sorted order */
    list_for_each_entry(pos, pq, list) {
        if (distance < pos->distance) {
            list_add_tail(&new_node->list, &pos->list);
            return;
        }
    }

    /* Add at the end */
    list_add_tail(&new_node->list, pq);
}

/**
 * vexfs_graph_pq_extract_min - Extract minimum distance node from priority queue
 * @pq: Priority queue head
 * 
 * Return: Pointer to minimum node, NULL if queue is empty
 */
static struct vexfs_graph_pq_node *vexfs_graph_pq_extract_min(struct list_head *pq)
{
    struct vexfs_graph_pq_node *min_node;

    if (list_empty(pq)) {
        return NULL;
    }

    min_node = list_first_entry(pq, struct vexfs_graph_pq_node, list);
    list_del(&min_node->list);

    return min_node;
}

MODULE_LICENSE("GPL v2");
MODULE_DESCRIPTION("VexFS v2.0 VexGraph Edge Operations and Traversal");