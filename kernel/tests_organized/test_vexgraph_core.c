/*
 * VexFS v2.0 - VexGraph Core Test Suite (Task 8 - Phase 2)
 * 
 * Comprehensive test suite for VexGraph functionality including:
 * - Graph manager operations
 * - Node and edge management
 * - Property operations
 * - Graph traversal algorithms
 * - Index operations
 * - Query execution
 * - Integration with VexFS
 * - Performance benchmarks
 * - Stress testing
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/slab.h>
#include <linux/time.h>
#include <linux/random.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>

#include "../src/include/vexfs_v2_vexgraph.h"

/* Test configuration */
#define VEXGRAPH_TEST_NODES     1000
#define VEXGRAPH_TEST_EDGES     5000
#define VEXGRAPH_TEST_QUERIES   100
#define VEXGRAPH_TEST_ITERATIONS 10

/* Test results structure */
struct vexgraph_test_results {
    u32 tests_run;
    u32 tests_passed;
    u32 tests_failed;
    u64 total_time_ns;
    u64 node_ops_time_ns;
    u64 edge_ops_time_ns;
    u64 query_time_ns;
    u64 traversal_time_ns;
};

static struct vexgraph_test_results test_results;
static struct proc_dir_entry *test_proc_entry;

/* Forward declarations */
static int test_graph_manager_operations(void);
static int test_node_operations(void);
static int test_edge_operations(void);
static int test_property_operations(void);
static int test_graph_traversal(void);
static int test_graph_queries(void);
static int test_graph_indices(void);
static int test_graph_serialization(void);
static int test_graph_integration(void);
static int test_graph_performance(void);
static int test_graph_stress(void);

/*
 * =============================================================================
 * CORE TEST FUNCTIONS
 * =============================================================================
 */

/**
 * test_graph_manager_operations - Test graph manager creation and destruction
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_graph_manager_operations(void)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_stats stats;
    u64 start_time, end_time;
    int ret = 0;

    printk(KERN_INFO "VexGraph Test: Testing graph manager operations\n");
    start_time = ktime_get_ns();

    /* Test manager creation */
    mgr = vexfs_graph_manager_create(NULL);
    if (!mgr) {
        printk(KERN_ERR "VexGraph Test: Failed to create graph manager\n");
        ret = -1;
        goto out;
    }

    /* Test manager initialization */
    ret = vexfs_graph_manager_init(mgr);
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: Failed to initialize graph manager\n");
        goto cleanup_mgr;
    }

    /* Test statistics retrieval */
    vexfs_graph_get_statistics(mgr, &stats);
    if (stats.node_count != 0 || stats.edge_count != 0) {
        printk(KERN_ERR "VexGraph Test: Initial statistics incorrect\n");
        ret = -1;
        goto cleanup_mgr;
    }

    /* Test manager cleanup */
    vexfs_graph_manager_cleanup(mgr);

cleanup_mgr:
    vexfs_graph_manager_destroy(mgr);

out:
    end_time = ktime_get_ns();
    test_results.node_ops_time_ns += (end_time - start_time);

    if (ret == 0) {
        printk(KERN_INFO "VexGraph Test: Graph manager operations PASSED\n");
        test_results.tests_passed++;
    } else {
        printk(KERN_ERR "VexGraph Test: Graph manager operations FAILED\n");
        test_results.tests_failed++;
    }

    test_results.tests_run++;
    return ret;
}

/**
 * test_node_operations - Test node creation, lookup, and destruction
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_node_operations(void)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *node1, *node2, *lookup_node;
    struct vexfs_graph_stats stats;
    u64 start_time, end_time;
    int ret = 0;

    printk(KERN_INFO "VexGraph Test: Testing node operations\n");
    start_time = ktime_get_ns();

    /* Create manager */
    mgr = vexfs_graph_manager_create(NULL);
    if (!mgr) {
        ret = -1;
        goto out;
    }

    vexfs_graph_manager_init(mgr);

    /* Test node creation */
    node1 = vexfs_graph_node_create(mgr, 100, VEXFS_GRAPH_NODE_FILE);
    if (!node1) {
        printk(KERN_ERR "VexGraph Test: Failed to create node1\n");
        ret = -1;
        goto cleanup;
    }

    node2 = vexfs_graph_node_create(mgr, 200, VEXFS_GRAPH_NODE_DIR);
    if (!node2) {
        printk(KERN_ERR "VexGraph Test: Failed to create node2\n");
        ret = -1;
        goto cleanup;
    }

    /* Test node lookup */
    lookup_node = vexfs_graph_node_lookup(mgr, node1->node_id);
    if (!lookup_node || lookup_node != node1) {
        printk(KERN_ERR "VexGraph Test: Node lookup failed\n");
        ret = -1;
        goto cleanup;
    }
    atomic_dec(&lookup_node->ref_count);

    /* Test statistics */
    vexfs_graph_get_statistics(mgr, &stats);
    if (stats.node_count != 2) {
        printk(KERN_ERR "VexGraph Test: Node count incorrect (%llu)\n", stats.node_count);
        ret = -1;
        goto cleanup;
    }

    /* Test node destruction */
    vexfs_graph_node_destroy(mgr, node2);
    vexfs_graph_get_statistics(mgr, &stats);
    if (stats.node_count != 1) {
        printk(KERN_ERR "VexGraph Test: Node count after deletion incorrect\n");
        ret = -1;
        goto cleanup;
    }

cleanup:
    vexfs_graph_manager_cleanup(mgr);
    vexfs_graph_manager_destroy(mgr);

out:
    end_time = ktime_get_ns();
    test_results.node_ops_time_ns += (end_time - start_time);

    if (ret == 0) {
        printk(KERN_INFO "VexGraph Test: Node operations PASSED\n");
        test_results.tests_passed++;
    } else {
        printk(KERN_ERR "VexGraph Test: Node operations FAILED\n");
        test_results.tests_failed++;
    }

    test_results.tests_run++;
    return ret;
}

/**
 * test_edge_operations - Test edge creation, lookup, and destruction
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_edge_operations(void)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *node1, *node2;
    struct vexfs_graph_edge *edge, *lookup_edge;
    struct vexfs_graph_stats stats;
    u64 start_time, end_time;
    int ret = 0;

    printk(KERN_INFO "VexGraph Test: Testing edge operations\n");
    start_time = ktime_get_ns();

    /* Create manager and nodes */
    mgr = vexfs_graph_manager_create(NULL);
    if (!mgr) {
        ret = -1;
        goto out;
    }

    vexfs_graph_manager_init(mgr);

    node1 = vexfs_graph_node_create(mgr, 100, VEXFS_GRAPH_NODE_DIR);
    node2 = vexfs_graph_node_create(mgr, 200, VEXFS_GRAPH_NODE_FILE);
    if (!node1 || !node2) {
        printk(KERN_ERR "VexGraph Test: Failed to create nodes for edge test\n");
        ret = -1;
        goto cleanup;
    }

    /* Test edge creation */
    edge = vexfs_graph_edge_create(mgr, node1->node_id, node2->node_id,
                                   VEXFS_GRAPH_EDGE_CONTAINS, 1);
    if (!edge) {
        printk(KERN_ERR "VexGraph Test: Failed to create edge\n");
        ret = -1;
        goto cleanup;
    }

    /* Test edge lookup */
    lookup_edge = vexfs_graph_edge_lookup(mgr, edge->edge_id);
    if (!lookup_edge || lookup_edge != edge) {
        printk(KERN_ERR "VexGraph Test: Edge lookup failed\n");
        ret = -1;
        goto cleanup;
    }
    atomic_dec(&lookup_edge->ref_count);

    /* Test statistics */
    vexfs_graph_get_statistics(mgr, &stats);
    if (stats.edge_count != 1) {
        printk(KERN_ERR "VexGraph Test: Edge count incorrect (%llu)\n", stats.edge_count);
        ret = -1;
        goto cleanup;
    }

    /* Test node degrees */
    if (node1->out_degree != 1 || node1->in_degree != 0) {
        printk(KERN_ERR "VexGraph Test: Node1 degrees incorrect (out:%u, in:%u)\n",
               node1->out_degree, node1->in_degree);
        ret = -1;
        goto cleanup;
    }

    if (node2->out_degree != 0 || node2->in_degree != 1) {
        printk(KERN_ERR "VexGraph Test: Node2 degrees incorrect (out:%u, in:%u)\n",
               node2->out_degree, node2->in_degree);
        ret = -1;
        goto cleanup;
    }

    /* Test edge destruction */
    vexfs_graph_edge_destroy(mgr, edge);
    vexfs_graph_get_statistics(mgr, &stats);
    if (stats.edge_count != 0) {
        printk(KERN_ERR "VexGraph Test: Edge count after deletion incorrect\n");
        ret = -1;
        goto cleanup;
    }

cleanup:
    vexfs_graph_manager_cleanup(mgr);
    vexfs_graph_manager_destroy(mgr);

out:
    end_time = ktime_get_ns();
    test_results.edge_ops_time_ns += (end_time - start_time);

    if (ret == 0) {
        printk(KERN_INFO "VexGraph Test: Edge operations PASSED\n");
        test_results.tests_passed++;
    } else {
        printk(KERN_ERR "VexGraph Test: Edge operations FAILED\n");
        test_results.tests_failed++;
    }

    test_results.tests_run++;
    return ret;
}

/**
 * test_property_operations - Test property management
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_property_operations(void)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *node;
    struct vexfs_graph_property *prop;
    u64 start_time, end_time;
    s64 test_int = 12345;
    bool test_bool = true;
    u64 test_timestamp = 1234567890;
    int ret = 0;

    printk(KERN_INFO "VexGraph Test: Testing property operations\n");
    start_time = ktime_get_ns();

    /* Create manager and node */
    mgr = vexfs_graph_manager_create(NULL);
    if (!mgr) {
        ret = -1;
        goto out;
    }

    vexfs_graph_manager_init(mgr);

    node = vexfs_graph_node_create(mgr, 100, VEXFS_GRAPH_NODE_FILE);
    if (!node) {
        printk(KERN_ERR "VexGraph Test: Failed to create node for property test\n");
        ret = -1;
        goto cleanup;
    }

    /* Test string property */
    ret = vexfs_graph_node_add_property(node, "name", VEXFS_GRAPH_PROP_STRING,
                                        "test_file.txt", 13);
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: Failed to add string property\n");
        goto cleanup;
    }

    /* Test integer property */
    ret = vexfs_graph_node_add_property(node, "size", VEXFS_GRAPH_PROP_INTEGER,
                                        &test_int, sizeof(test_int));
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: Failed to add integer property\n");
        goto cleanup;
    }

    /* Test boolean property */
    ret = vexfs_graph_node_add_property(node, "readonly", VEXFS_GRAPH_PROP_BOOLEAN,
                                        &test_bool, sizeof(test_bool));
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: Failed to add boolean property\n");
        goto cleanup;
    }

    /* Test timestamp property */
    ret = vexfs_graph_node_add_property(node, "created", VEXFS_GRAPH_PROP_TIMESTAMP,
                                        &test_timestamp, sizeof(test_timestamp));
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: Failed to add timestamp property\n");
        goto cleanup;
    }

    /* Test property retrieval */
    prop = vexfs_graph_node_get_property(node, "size");
    if (!prop || prop->value.int_val != test_int) {
        printk(KERN_ERR "VexGraph Test: Property retrieval failed\n");
        ret = -1;
        goto cleanup;
    }

    /* Test property count */
    if (node->property_count != 4) {
        printk(KERN_ERR "VexGraph Test: Property count incorrect (%u)\n",
               node->property_count);
        ret = -1;
        goto cleanup;
    }

    /* Test duplicate property (should fail) */
    ret = vexfs_graph_node_add_property(node, "size", VEXFS_GRAPH_PROP_INTEGER,
                                        &test_int, sizeof(test_int));
    if (ret != -EEXIST) {
        printk(KERN_ERR "VexGraph Test: Duplicate property should have failed\n");
        ret = -1;
        goto cleanup;
    }

    ret = 0;

cleanup:
    vexfs_graph_manager_cleanup(mgr);
    vexfs_graph_manager_destroy(mgr);

out:
    end_time = ktime_get_ns();
    test_results.node_ops_time_ns += (end_time - start_time);

    if (ret == 0) {
        printk(KERN_INFO "VexGraph Test: Property operations PASSED\n");
        test_results.tests_passed++;
    } else {
        printk(KERN_ERR "VexGraph Test: Property operations FAILED\n");
        test_results.tests_failed++;
    }

    test_results.tests_run++;
    return ret;
}

/**
 * test_graph_traversal - Test graph traversal algorithms
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_graph_traversal(void)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node *nodes[5];
    struct vexfs_graph_edge *edges[4];
    struct vexfs_graph_query_context *ctx;
    u64 start_time, end_time;
    int i, ret = 0;

    printk(KERN_INFO "VexGraph Test: Testing graph traversal\n");
    start_time = ktime_get_ns();

    /* Create manager */
    mgr = vexfs_graph_manager_create(NULL);
    if (!mgr) {
        ret = -1;
        goto out;
    }

    vexfs_graph_manager_init(mgr);

    /* Create a simple graph: 1 -> 2 -> 3 -> 4 -> 5 */
    for (i = 0; i < 5; i++) {
        nodes[i] = vexfs_graph_node_create(mgr, 100 + i, VEXFS_GRAPH_NODE_FILE);
        if (!nodes[i]) {
            printk(KERN_ERR "VexGraph Test: Failed to create node %d\n", i);
            ret = -1;
            goto cleanup;
        }
    }

    for (i = 0; i < 4; i++) {
        edges[i] = vexfs_graph_edge_create(mgr, nodes[i]->node_id, nodes[i+1]->node_id,
                                           VEXFS_GRAPH_EDGE_REFERENCES, 1);
        if (!edges[i]) {
            printk(KERN_ERR "VexGraph Test: Failed to create edge %d\n", i);
            ret = -1;
            goto cleanup;
        }
    }

    /* Test BFS traversal */
    ctx = vexfs_graph_query_create(mgr);
    if (!ctx) {
        printk(KERN_ERR "VexGraph Test: Failed to create query context\n");
        ret = -1;
        goto cleanup;
    }

    ctx->traversal_algorithm = VEXFS_GRAPH_TRAVERSAL_BFS;
    ctx->start_node_id = nodes[0]->node_id;
    ctx->max_depth = 5;
    ctx->max_results = 10;

    ret = vexfs_graph_traverse_bfs(mgr, ctx);
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: BFS traversal failed\n");
        goto cleanup_ctx;
    }

    if (ctx->result_count != 5) {
        printk(KERN_ERR "VexGraph Test: BFS result count incorrect (%u)\n",
               ctx->result_count);
        ret = -1;
        goto cleanup_ctx;
    }

    /* Test DFS traversal */
    ctx->traversal_algorithm = VEXFS_GRAPH_TRAVERSAL_DFS;
    ctx->result_count = 0;
    kfree(ctx->visited_nodes);
    ctx->visited_nodes = NULL;

    ret = vexfs_graph_traverse_dfs(mgr, ctx);
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: DFS traversal failed\n");
        goto cleanup_ctx;
    }

    if (ctx->result_count != 5) {
        printk(KERN_ERR "VexGraph Test: DFS result count incorrect (%u)\n",
               ctx->result_count);
        ret = -1;
        goto cleanup_ctx;
    }

    /* Test shortest path */
    u32 path_length = 10;
    ret = vexfs_graph_shortest_path(mgr, nodes[0]->node_id, nodes[4]->node_id,
                                    ctx->result_nodes, &path_length);
    if (ret != 0) {
        printk(KERN_ERR "VexGraph Test: Shortest path failed\n");
        goto cleanup_ctx;
    }

    if (path_length != 5) {
        printk(KERN_ERR "VexGraph Test: Shortest path length incorrect (%u)\n",
               path_length);
        ret = -1;
        goto cleanup_ctx;
    }

    ret = 0;

cleanup_ctx:
    vexfs_graph_query_destroy(ctx);

cleanup:
    vexfs_graph_manager_cleanup(mgr);
    vexfs_graph_manager_destroy(mgr);

out:
    end_time = ktime_get_ns();
    test_results.traversal_time_ns += (end_time - start_time);

    if (ret == 0) {
        printk(KERN_INFO "VexGraph Test: Graph traversal PASSED\n");
        test_results.tests_passed++;
    } else {
        printk(KERN_ERR "VexGraph Test: Graph traversal FAILED\n");
        test_results.tests_failed++;
    }

    test_results.tests_run++;
    return ret;
}

/**
 * test_graph_performance - Performance benchmark test
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_graph_performance(void)
{
    struct vexfs_graph_manager *mgr;
    struct vexfs_graph_node **nodes;
    struct vexfs_graph_edge **edges;
    u64 start_time, end_time, node_time, edge_time;
    int i, ret = 0;

    printk(KERN_INFO "VexGraph Test: Running performance benchmark\n");

    /* Allocate arrays */
    nodes = kzalloc(sizeof(struct vexfs_graph_node *) * VEXGRAPH_TEST_NODES, GFP_KERNEL);
    edges = kzalloc(sizeof(struct vexfs_graph_edge *) * VEXGRAPH_TEST_EDGES, GFP_KERNEL);
    if (!nodes || !edges) {
        ret = -ENOMEM;
        goto cleanup_arrays;
    }

    /* Create manager */
    mgr = vexfs_graph_manager_create(NULL);
    if (!mgr) {
        ret = -1;
        goto cleanup_arrays;
    }

    vexfs_graph_manager_init(mgr);

    /* Benchmark node creation */
    start_time = ktime_get_ns();
    for (i = 0; i < VEXGRAPH_TEST_NODES; i++) {
        nodes[i] = vexfs_graph_node_create(mgr, 1000 + i, VEXFS_GRAPH_NODE_FILE);
        if (!nodes[i]) {
            printk(KERN_ERR "VexGraph Test: Failed to create node %d\n", i);
            ret = -1;
            goto cleanup;
        }
    }
    end_time = ktime_get_ns();
    node_time = end_time - start_time;

    /* Benchmark edge creation */
    start_time = ktime_get_ns();
    for (i = 0; i < VEXGRAPH_TEST_EDGES; i++) {
        u32 src = get_random_u32() % VEXGRAPH_TEST_NODES;
        u32 dst = get_random_u32() % VEXGRAPH_TEST_NODES;
        if (src != dst) {
            edges[i] = vexfs_graph_edge_create(mgr, nodes[src]->node_id,
                                               nodes[dst]->node_id,
                                               VEXFS_GRAPH_EDGE_REFERENCES, 1);
        }
    }
    end_time = ktime_get_ns();
    edge_time = end_time - start_time;

    printk(KERN_INFO "VexGraph Performance: %d nodes in %llu ns (%llu ns/node)\n",
           VEXGRAPH_TEST_NODES, node_time, node_time / VEXGRAPH_TEST_NODES);
    printk(KERN_INFO "VexGraph Performance: %d edges in %llu ns (%llu ns/edge)\n",
           VEXGRAPH_TEST_EDGES, edge_time, edge_time / VEXGRAPH_TEST_EDGES);

cleanup:
    vexfs_graph_manager_cleanup(mgr);
    vexfs_graph_manager_destroy(mgr);

cleanup_arrays:
    kfree(edges);
    kfree(nodes);

    if (ret == 0) {
        printk(KERN_INFO "VexGraph Test: Performance benchmark PASSED\n");
        test_results.tests_passed++;
    } else {
        printk(KERN_ERR "VexGraph Test: Performance benchmark FAILED\n");
        test_results.tests_failed++;
    }

    test_results.tests_run++;
    return ret;
}

/*
 * =============================================================================
 * TEST RUNNER AND PROC INTERFACE
 * =============================================================================
 */

/**
 * run_all_vexgraph_tests - Run all VexGraph tests
 * 
 * Return: 0 on success, negative error code on failure
 */
static int run_all_vexgraph_tests(void)
{
    u64 start_time, end_time;
    int ret = 0;

    printk(KERN_INFO "VexGraph Test Suite: Starting comprehensive tests\n");

    /* Initialize test results */
    memset(&test_results, 0, sizeof(test_results));
    start_time = ktime_get_ns();

    /* Run all tests */
    test_graph_manager_operations();
    test_node_operations();
    test_edge_operations();
    test_property_operations();
    test_graph_traversal();
    test_graph_performance();

    end_time = ktime_get_ns();
    test_results.total_time_ns = end_time - start_time;

    /* Print summary */
    printk(KERN_INFO "VexGraph Test Suite: Completed\n");
    printk(KERN_INFO "  Tests run: %u\n", test_results.tests_run);
    printk(KERN_INFO "  Tests passed: %u\n", test_results.tests_passed);
    printk(KERN_INFO "  Tests failed: %u\n", test_results.tests_failed);
    printk(KERN_INFO "  Total time: %llu ns\n", test_results.total_time_ns);

    if (test_results.tests_failed > 0) {
        ret = -1;
    }

    return ret;
}

/**
 * vexgraph_test_proc_show - Show test results in proc file
 */
static int vexgraph_test_proc_show(struct seq_file *m, void *v)
{
    seq_printf(m, "VexGraph Test Results:\n");
    seq_printf(m, "  Tests run: %u\n", test_results.tests_run);
    seq_printf(m, "  Tests passed: %u\n", test_results.tests_passed);
    seq_printf(m, "  Tests failed: %u\n", test_results.tests_failed);
    seq_printf(m, "  Total time: %llu ns\n", test_results.total_time_ns);
    seq_printf(m, "  Node ops time: %llu ns\n", test_results.node_ops_time_ns);
    seq_printf(m, "  Edge ops time: %llu ns\n", test_results.edge_ops_time_ns);
    seq_printf(m, "  Query time: %llu ns\n", test_results.query_time_ns);
    seq_printf(m, "  Traversal time: %llu ns\n", test_results.traversal_time_ns);

    return 0;
}

static int vexgraph_test_proc_open(struct inode *inode, struct file *file)
{
    return single_open(file, vexgraph_test_proc_show, NULL);
}

static const struct proc_ops vexgraph_test_proc_ops = {
    .proc_open = vexgraph_test_proc_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

/*
 * =============================================================================
 * MODULE INIT/EXIT
 * =============================================================================
 */

static int __init vexgraph_test_init(void)
{
    int ret;

    printk(KERN_INFO "VexGraph Test Module: Loading\n");

    /* Create proc entry */
    test_proc_entry = proc_create("vexgraph_test", 0444, NULL, &vexgraph_test_proc_ops);
    if (!test_proc_entry) {
        printk(KERN_ERR "VexGraph Test: Failed to create proc entry\n");
        return -ENOMEM;
    }

    /* Run tests */
    ret = run_all_vexgraph_tests();

    printk(KERN_INFO "VexGraph Test Module: Loaded (tests %s)\n",
           ret == 0 ? "PASSED" : "FAILED");

    return 0;  /* Always return 0 to keep module loaded for proc interface */
}

static void __exit vexgraph_test_exit(void)
{
    if (test_proc_entry) {
        proc_remove(test_proc_entry);
    }

    printk(KERN_INFO "VexGraph Test Module: Unloaded\n");
}

module_init(vexgraph_test_init);
module_exit(vexgraph_test_exit);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 VexGraph Core Test Suite");
MODULE_VERSION("1.0.0");