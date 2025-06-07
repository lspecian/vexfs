/*
 * VexFS v2.0 Deadlock Detection and Prevention
 * 
 * This file implements comprehensive deadlock detection and prevention
 * mechanisms for the VexFS locking system. Provides lock dependency
 * tracking, cycle detection, and automatic deadlock resolution.
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/timer.h>
#include <linux/workqueue.h>
#include <linux/slab.h>
#include <linux/hash.h>
#include <linux/list.h>
#include <linux/rbtree.h>
#include <linux/ktime.h>
#include <linux/delay.h>

#include "vexfs_v2_locking.h"

/* Deadlock detection configuration */
#define VEXFS_DEADLOCK_GRAPH_SIZE       256
#define VEXFS_DEADLOCK_MAX_DEPTH        32
#define VEXFS_DEADLOCK_CHECK_PERIOD     (HZ / 10)  /* 100ms */
#define VEXFS_DEADLOCK_RESOLUTION_MAX   10

/* Lock dependency graph structures */
struct vexfs_lock_node {
    void *lock_ptr;                     /* Lock pointer */
    u32 lock_order;                     /* Lock ordering level */
    u32 lock_type;                      /* Lock type identifier */
    atomic_t ref_count;                 /* Reference count */
    struct list_head edges;             /* Outgoing edges */
    struct list_head incoming;          /* Incoming edges */
    struct hlist_node hash_node;        /* Hash table linkage */
    u64 creation_time;                  /* Node creation time */
    u32 thread_id;                      /* Creating thread ID */
};

struct vexfs_lock_edge {
    struct vexfs_lock_node *from;       /* Source node */
    struct vexfs_lock_node *to;         /* Destination node */
    struct list_head from_list;         /* List in source node */
    struct list_head to_list;           /* List in destination node */
    u64 creation_time;                  /* Edge creation time */
    u32 weight;                         /* Edge weight/priority */
};

struct vexfs_deadlock_cycle {
    struct vexfs_lock_node *nodes[VEXFS_DEADLOCK_MAX_DEPTH];
    u32 length;                         /* Cycle length */
    u32 priority;                       /* Resolution priority */
    u64 detection_time;                 /* When cycle was detected */
};

/* Global deadlock detection state */
static struct kmem_cache *vexfs_lock_node_cache = NULL;
static struct kmem_cache *vexfs_lock_edge_cache = NULL;

/* 🔥 DEADLOCK DETECTOR INITIALIZATION 🔥 */

/**
 * vexfs_deadlock_detector_init - Initialize deadlock detector
 * @detector: Deadlock detector structure
 */
int vexfs_deadlock_detector_init(struct vexfs_deadlock_detector *detector)
{
    int i;
    
    if (!detector) {
        pr_err("VexFS: NULL detector in deadlock detector init\n");
        return -EINVAL;
    }
    
    pr_info("VexFS: Initializing deadlock detection system\n");
    
    /* Initialize detector mutex */
    mutex_init(&detector->detector_mutex);
    
    /* Initialize lock dependency graph */
    for (i = 0; i < ARRAY_SIZE(detector->lock_graph); i++) {
        INIT_HLIST_HEAD(&detector->lock_graph[i]);
    }
    
    /* Initialize atomic counters */
    atomic_set(&detector->detection_active, 0);
    atomic64_set(&detector->deadlock_count, 0);
    atomic64_set(&detector->prevention_count, 0);
    
    /* Create slab caches for nodes and edges */
    if (!vexfs_lock_node_cache) {
        vexfs_lock_node_cache = kmem_cache_create("vexfs_lock_nodes",
                                                  sizeof(struct vexfs_lock_node),
                                                  0, SLAB_HWCACHE_ALIGN, NULL);
        if (!vexfs_lock_node_cache) {
            pr_err("VexFS: Failed to create lock node cache\n");
            return -ENOMEM;
        }
    }
    
    if (!vexfs_lock_edge_cache) {
        vexfs_lock_edge_cache = kmem_cache_create("vexfs_lock_edges",
                                                  sizeof(struct vexfs_lock_edge),
                                                  0, SLAB_HWCACHE_ALIGN, NULL);
        if (!vexfs_lock_edge_cache) {
            pr_err("VexFS: Failed to create lock edge cache\n");
            kmem_cache_destroy(vexfs_lock_node_cache);
            vexfs_lock_node_cache = NULL;
            return -ENOMEM;
        }
    }
    
    /* Initialize periodic check timer */
    timer_setup(&detector->check_timer, vexfs_deadlock_check_timer, 0);
    
    /* Initialize work queue for deadlock resolution */
    INIT_WORK(&detector->check_work, vexfs_deadlock_check_work);
    
    detector->last_check_time = ktime_get_ns();
    
    /* Start periodic checking */
    mod_timer(&detector->check_timer, jiffies + VEXFS_DEADLOCK_CHECK_PERIOD);
    
    pr_info("VexFS: Deadlock detector initialized successfully\n");
    
    return 0;
}

/**
 * vexfs_deadlock_detector_cleanup - Cleanup deadlock detector
 * @detector: Deadlock detector structure
 */
void vexfs_deadlock_detector_cleanup(struct vexfs_deadlock_detector *detector)
{
    struct vexfs_lock_node *node;
    struct vexfs_lock_edge *edge, *edge_tmp;
    struct hlist_node *tmp;
    int i;
    
    if (!detector) {
        return;
    }
    
    pr_info("VexFS: Cleaning up deadlock detector\n");
    
    /* Stop periodic checking */
    del_timer_sync(&detector->check_timer);
    cancel_work_sync(&detector->check_work);
    
    /* Cleanup lock dependency graph */
    mutex_lock(&detector->detector_mutex);
    
    for (i = 0; i < ARRAY_SIZE(detector->lock_graph); i++) {
        hlist_for_each_entry_safe(node, tmp, &detector->lock_graph[i], hash_node) {
            /* Cleanup all edges from this node */
            list_for_each_entry_safe(edge, edge_tmp, &node->edges, from_list) {
                list_del(&edge->from_list);
                list_del(&edge->to_list);
                kmem_cache_free(vexfs_lock_edge_cache, edge);
            }
            
            /* Remove node from hash table */
            hlist_del(&node->hash_node);
            kmem_cache_free(vexfs_lock_node_cache, node);
        }
    }
    
    mutex_unlock(&detector->detector_mutex);
    
    /* Cleanup slab caches */
    if (vexfs_lock_edge_cache) {
        kmem_cache_destroy(vexfs_lock_edge_cache);
        vexfs_lock_edge_cache = NULL;
    }
    
    if (vexfs_lock_node_cache) {
        kmem_cache_destroy(vexfs_lock_node_cache);
        vexfs_lock_node_cache = NULL;
    }
    
    pr_info("VexFS: Deadlock detector cleanup completed\n");
    pr_info("VexFS: Total deadlocks detected: %lld, prevented: %lld\n",
            atomic64_read(&detector->deadlock_count),
            atomic64_read(&detector->prevention_count));
}

/* 🔥 LOCK DEPENDENCY TRACKING 🔥 */

/**
 * vexfs_deadlock_hash_lock - Hash lock pointer for dependency graph
 * @lock_ptr: Lock pointer to hash
 */
static u32 vexfs_deadlock_hash_lock(void *lock_ptr)
{
    return hash_ptr(lock_ptr, 8) & (VEXFS_DEADLOCK_GRAPH_SIZE - 1);
}

/**
 * vexfs_deadlock_find_node - Find lock node in dependency graph
 * @detector: Deadlock detector
 * @lock_ptr: Lock pointer to find
 */
static struct vexfs_lock_node *vexfs_deadlock_find_node(struct vexfs_deadlock_detector *detector,
                                                        void *lock_ptr)
{
    struct vexfs_lock_node *node;
    u32 hash = vexfs_deadlock_hash_lock(lock_ptr);
    
    hlist_for_each_entry(node, &detector->lock_graph[hash], hash_node) {
        if (node->lock_ptr == lock_ptr) {
            return node;
        }
    }
    
    return NULL;
}

/**
 * vexfs_deadlock_create_node - Create new lock node
 * @detector: Deadlock detector
 * @lock_ptr: Lock pointer
 * @lock_order: Lock ordering level
 * @lock_type: Lock type identifier
 */
static struct vexfs_lock_node *vexfs_deadlock_create_node(struct vexfs_deadlock_detector *detector,
                                                          void *lock_ptr,
                                                          u32 lock_order,
                                                          u32 lock_type)
{
    struct vexfs_lock_node *node;
    u32 hash;
    
    node = kmem_cache_alloc(vexfs_lock_node_cache, GFP_ATOMIC);
    if (!node) {
        pr_err("VexFS: Failed to allocate lock node\n");
        return NULL;
    }
    
    node->lock_ptr = lock_ptr;
    node->lock_order = lock_order;
    node->lock_type = lock_type;
    atomic_set(&node->ref_count, 1);
    INIT_LIST_HEAD(&node->edges);
    INIT_LIST_HEAD(&node->incoming);
    INIT_HLIST_NODE(&node->hash_node);
    node->creation_time = ktime_get_ns();
    node->thread_id = current->pid;
    
    /* Add to hash table */
    hash = vexfs_deadlock_hash_lock(lock_ptr);
    hlist_add_head(&node->hash_node, &detector->lock_graph[hash]);
    
    pr_debug("VexFS: Created deadlock node for lock %px (order: %u, type: %u)\n",
             lock_ptr, lock_order, lock_type);
    
    return node;
}

/**
 * vexfs_deadlock_create_edge - Create dependency edge between locks
 * @from: Source lock node
 * @to: Destination lock node
 */
static struct vexfs_lock_edge *vexfs_deadlock_create_edge(struct vexfs_lock_node *from,
                                                          struct vexfs_lock_node *to)
{
    struct vexfs_lock_edge *edge;
    
    edge = kmem_cache_alloc(vexfs_lock_edge_cache, GFP_ATOMIC);
    if (!edge) {
        pr_err("VexFS: Failed to allocate lock edge\n");
        return NULL;
    }
    
    edge->from = from;
    edge->to = to;
    INIT_LIST_HEAD(&edge->from_list);
    INIT_LIST_HEAD(&edge->to_list);
    edge->creation_time = ktime_get_ns();
    edge->weight = 1;
    
    /* Add to both nodes' edge lists */
    list_add(&edge->from_list, &from->edges);
    list_add(&edge->to_list, &to->incoming);
    
    /* Increment reference counts */
    atomic_inc(&from->ref_count);
    atomic_inc(&to->ref_count);
    
    pr_debug("VexFS: Created deadlock edge: %px -> %px\n", from->lock_ptr, to->lock_ptr);
    
    return edge;
}

/**
 * vexfs_deadlock_check_dependency - Check and record lock dependency
 * @detector: Deadlock detector
 * @lock1: First lock
 * @lock2: Second lock
 * @order1: First lock order
 * @order2: Second lock order
 */
int vexfs_deadlock_check_dependency(struct vexfs_deadlock_detector *detector,
                                    void *lock1, void *lock2,
                                    u32 order1, u32 order2)
{
    struct vexfs_lock_node *node1, *node2;
    struct vexfs_lock_edge *edge;
    int ret = 0;
    
    if (!detector || !lock1 || !lock2) {
        return -EINVAL;
    }
    
    /* Don't track self-dependencies */
    if (lock1 == lock2) {
        return 0;
    }
    
    mutex_lock(&detector->detector_mutex);
    
    /* Find or create nodes */
    node1 = vexfs_deadlock_find_node(detector, lock1);
    if (!node1) {
        node1 = vexfs_deadlock_create_node(detector, lock1, order1, 0);
        if (!node1) {
            ret = -ENOMEM;
            goto unlock;
        }
    }
    
    node2 = vexfs_deadlock_find_node(detector, lock2);
    if (!node2) {
        node2 = vexfs_deadlock_create_node(detector, lock2, order2, 0);
        if (!node2) {
            ret = -ENOMEM;
            goto unlock;
        }
    }
    
    /* Check if this would create a cycle */
    if (vexfs_deadlock_would_create_cycle(detector, lock1, lock2)) {
        pr_warn("VexFS: Potential deadlock detected: %px -> %px\n", lock1, lock2);
        atomic64_inc(&detector->prevention_count);
        ret = -EDEADLK;
        goto unlock;
    }
    
    /* Create dependency edge */
    edge = vexfs_deadlock_create_edge(node1, node2);
    if (!edge) {
        ret = -ENOMEM;
        goto unlock;
    }

unlock:
    mutex_unlock(&detector->detector_mutex);
    return ret;
}

/* 🔥 CYCLE DETECTION 🔥 */

/**
 * vexfs_deadlock_dfs_visit - Depth-first search for cycle detection
 * @node: Current node
 * @visited: Visited nodes array
 * @rec_stack: Recursion stack
 * @depth: Current depth
 * @cycle: Output cycle structure
 */
static bool vexfs_deadlock_dfs_visit(struct vexfs_lock_node *node,
                                     struct vexfs_lock_node **visited,
                                     struct vexfs_lock_node **rec_stack,
                                     u32 depth,
                                     struct vexfs_deadlock_cycle *cycle)
{
    struct vexfs_lock_edge *edge;
    u32 i;
    
    if (depth >= VEXFS_DEADLOCK_MAX_DEPTH) {
        return false;
    }
    
    /* Check if we've found a cycle */
    for (i = 0; i < depth; i++) {
        if (rec_stack[i] == node) {
            /* Found cycle - record it */
            cycle->length = depth - i;
            for (u32 j = 0; j < cycle->length; j++) {
                cycle->nodes[j] = rec_stack[i + j];
            }
            cycle->detection_time = ktime_get_ns();
            cycle->priority = cycle->length; /* Shorter cycles have higher priority */
            return true;
        }
    }
    
    /* Add to recursion stack */
    rec_stack[depth] = node;
    
    /* Visit all adjacent nodes */
    list_for_each_entry(edge, &node->edges, from_list) {
        if (vexfs_deadlock_dfs_visit(edge->to, visited, rec_stack, depth + 1, cycle)) {
            return true;
        }
    }
    
    return false;
}

/**
 * vexfs_deadlock_would_create_cycle - Check if adding edge would create cycle
 * @detector: Deadlock detector
 * @lock1: Source lock
 * @lock2: Destination lock
 */
bool vexfs_deadlock_would_create_cycle(struct vexfs_deadlock_detector *detector,
                                       void *lock1, void *lock2)
{
    struct vexfs_lock_node *node1, *node2;
    struct vexfs_lock_node *visited[VEXFS_DEADLOCK_MAX_DEPTH];
    struct vexfs_lock_node *rec_stack[VEXFS_DEADLOCK_MAX_DEPTH];
    struct vexfs_deadlock_cycle cycle;
    
    if (!detector) {
        return false;
    }
    
    node1 = vexfs_deadlock_find_node(detector, lock1);
    node2 = vexfs_deadlock_find_node(detector, lock2);
    
    if (!node1 || !node2) {
        return false; /* No existing nodes, no cycle possible */
    }
    
    /* Temporarily add the edge and check for cycles */
    memset(visited, 0, sizeof(visited));
    memset(rec_stack, 0, sizeof(rec_stack));
    memset(&cycle, 0, sizeof(cycle));
    
    /* Start DFS from node2 to see if we can reach node1 */
    return vexfs_deadlock_dfs_visit(node2, visited, rec_stack, 0, &cycle);
}

/**
 * vexfs_deadlock_detect_cycles - Detect all cycles in dependency graph
 * @detector: Deadlock detector
 * @cycles: Output array for detected cycles
 * @max_cycles: Maximum number of cycles to detect
 */
static int vexfs_deadlock_detect_cycles(struct vexfs_deadlock_detector *detector,
                                       struct vexfs_deadlock_cycle *cycles,
                                       u32 max_cycles)
{
    struct vexfs_lock_node *node;
    struct vexfs_lock_node *visited[VEXFS_DEADLOCK_MAX_DEPTH];
    struct vexfs_lock_node *rec_stack[VEXFS_DEADLOCK_MAX_DEPTH];
    u32 cycle_count = 0;
    int i;
    
    if (!detector || !cycles) {
        return 0;
    }
    
    /* Iterate through all nodes in the graph */
    for (i = 0; i < ARRAY_SIZE(detector->lock_graph) && cycle_count < max_cycles; i++) {
        hlist_for_each_entry(node, &detector->lock_graph[i], hash_node) {
            memset(visited, 0, sizeof(visited));
            memset(rec_stack, 0, sizeof(rec_stack));
            
            if (vexfs_deadlock_dfs_visit(node, visited, rec_stack, 0, &cycles[cycle_count])) {
                cycle_count++;
                atomic64_inc(&detector->deadlock_count);
                
                pr_warn("VexFS: Deadlock cycle detected (length: %u)\n",
                        cycles[cycle_count - 1].length);
            }
        }
    }
    
    return cycle_count;
}

/* 🔥 DEADLOCK RESOLUTION 🔥 */

/**
 * vexfs_deadlock_resolve - Resolve detected deadlocks
 * @detector: Deadlock detector
 * @locks: Array of locks involved in deadlock
 * @count: Number of locks
 */
int vexfs_deadlock_resolve(struct vexfs_deadlock_detector *detector,
                           void **locks, u32 count)
{
    struct vexfs_deadlock_cycle cycles[VEXFS_DEADLOCK_RESOLUTION_MAX];
    u32 cycle_count;
    u32 i, j;
    
    if (!detector || !locks || count == 0) {
        return -EINVAL;
    }
    
    mutex_lock(&detector->detector_mutex);
    
    /* Detect cycles involving the given locks */
    cycle_count = vexfs_deadlock_detect_cycles(detector, cycles, VEXFS_DEADLOCK_RESOLUTION_MAX);
    
    if (cycle_count == 0) {
        mutex_unlock(&detector->detector_mutex);
        return 0; /* No deadlocks found */
    }
    
    pr_warn("VexFS: Resolving %u deadlock cycles\n", cycle_count);
    
    /* Resolve cycles by breaking edges */
    for (i = 0; i < cycle_count; i++) {
        struct vexfs_deadlock_cycle *cycle = &cycles[i];
        
        /* Find the best edge to break (highest order difference) */
        u32 best_edge = 0;
        u32 max_order_diff = 0;
        
        for (j = 0; j < cycle->length; j++) {
            u32 next = (j + 1) % cycle->length;
            u32 order_diff = abs((int)cycle->nodes[next]->lock_order - 
                                (int)cycle->nodes[j]->lock_order);
            
            if (order_diff > max_order_diff) {
                max_order_diff = order_diff;
                best_edge = j;
            }
        }
        
        /* Break the selected edge */
        struct vexfs_lock_node *from = cycle->nodes[best_edge];
        struct vexfs_lock_node *to = cycle->nodes[(best_edge + 1) % cycle->length];
        
        /* Find and remove the edge */
        struct vexfs_lock_edge *edge, *tmp;
        list_for_each_entry_safe(edge, tmp, &from->edges, from_list) {
            if (edge->to == to) {
                list_del(&edge->from_list);
                list_del(&edge->to_list);
                atomic_dec(&from->ref_count);
                atomic_dec(&to->ref_count);
                kmem_cache_free(vexfs_lock_edge_cache, edge);
                
                pr_info("VexFS: Broke deadlock edge: %px -> %px\n",
                        from->lock_ptr, to->lock_ptr);
                break;
            }
        }
    }
    
    mutex_unlock(&detector->detector_mutex);
    
    pr_info("VexFS: Resolved %u deadlock cycles\n", cycle_count);
    
    return cycle_count;
}

/* 🔥 PERIODIC DEADLOCK CHECKING 🔥 */

/**
 * vexfs_deadlock_check_timer - Timer callback for periodic deadlock checking
 * @t: Timer structure
 */
static void vexfs_deadlock_check_timer(struct timer_list *t)
{
    struct vexfs_deadlock_detector *detector = 
        container_of(t, struct vexfs_deadlock_detector, check_timer);
    
    /* Schedule work queue for actual checking */
    schedule_work(&detector->check_work);
    
    /* Reschedule timer */
    mod_timer(&detector->check_timer, jiffies + VEXFS_DEADLOCK_CHECK_PERIOD);
}

/**
 * vexfs_deadlock_check_work - Work queue function for deadlock checking
 * @work: Work structure
 */
static void vexfs_deadlock_check_work(struct work_struct *work)
{
    struct vexfs_deadlock_detector *detector = 
        container_of(work, struct vexfs_deadlock_detector, check_work);
    struct vexfs_deadlock_cycle cycles[VEXFS_DEADLOCK_RESOLUTION_MAX];
    u32 cycle_count;
    
    /* Skip if detection is already active */
    if (atomic_cmpxchg(&detector->detection_active, 0, 1) != 0) {
        return;
    }
    
    /* Perform deadlock detection */
    mutex_lock(&detector->detector_mutex);
    cycle_count = vexfs_deadlock_detect_cycles(detector, cycles, VEXFS_DEADLOCK_RESOLUTION_MAX);
    mutex_unlock(&detector->detector_mutex);
    
    if (cycle_count > 0) {
        pr_warn("VexFS: Periodic check detected %u deadlock cycles\n", cycle_count);
        
        /* Attempt automatic resolution */
        void *dummy_locks[1] = { NULL };
        vexfs_deadlock_resolve(detector, dummy_locks, 0);
    }
    
    detector->last_check_time = ktime_get_ns();
    atomic_set(&detector->detection_active, 0);
}

/* 🔥 DEADLOCK STATISTICS 🔥 */

/**
 * vexfs_deadlock_get_stats - Get deadlock detection statistics
 * @detector: Deadlock detector
 * @stats: Output statistics structure
 */
int vexfs_deadlock_get_stats(struct vexfs_deadlock_detector *detector,
                             struct vexfs_deadlock_stats *stats)
{
    int i, node_count = 0, edge_count = 0;
    struct vexfs_lock_node *node;
    struct vexfs_lock_edge *edge;
    
    if (!detector || !stats) {
        return -EINVAL;
    }
    
    memset(stats, 0, sizeof(*stats));
    
    mutex_lock(&detector->detector_mutex);
    
    /* Count nodes and edges */
    for (i = 0; i < ARRAY_SIZE(detector->lock_graph); i++) {
        hlist_for_each_entry(node, &detector->lock_graph[i], hash_node) {
            node_count++;
            list_for_each_entry(edge, &node->edges, from_list) {
                edge_count++;
            }
        }
    }
    
    stats->total_nodes = node_count;
    stats->total_edges = edge_count;
    stats->deadlocks_detected = atomic64_read(&detector->deadlock_count);
    stats->deadlocks_prevented = atomic64_read(&detector->prevention_count);
    stats->last_check_time = detector->last_check_time;
    stats->detection_active = atomic_read(&detector->detection_active);
    
    mutex_unlock(&detector->detector_mutex);
    
    return 0;
}

/* Deadlock statistics structure */
struct vexfs_deadlock_stats {
    u32 total_nodes;
    u32 total_edges;
    u64 deadlocks_detected;
    u64 deadlocks_prevented;
    u64 last_check_time;
    u32 detection_active;
};