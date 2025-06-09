/*
 * VexFS v2.0 - VexGraph API Test Suite (Task 9 - Phase 2)
 * 
 * Comprehensive test suite for the VexGraph API layer, validating all
 * CRUD operations, traversal algorithms, query language, and performance
 * characteristics.
 *
 * Test Categories:
 * - API Manager lifecycle tests
 * - Node CRUD operation tests
 * - Edge CRUD operation tests
 * - Traversal algorithm tests
 * - Query language and optimization tests
 * - Index management tests
 * - Performance and concurrency tests
 * - Error handling and validation tests
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/time.h>
#include <linux/random.h>
#include <linux/delay.h>

#include "../src/include/vexfs_v2_vexgraph_api.h"
#include "../src/include/vexfs_v2_internal.h"

/* Test configuration */
#define VEXFS_API_TEST_NODES        100
#define VEXFS_API_TEST_EDGES        200
#define VEXFS_API_TEST_PROPERTIES   10
#define VEXFS_API_TEST_ITERATIONS   1000

/* Test result tracking */
static u32 tests_passed = 0;
static u32 tests_failed = 0;
static u64 total_test_time = 0;

/* Test helper macros */
#define VEXFS_API_TEST_ASSERT(condition, message) \
    do { \
        if (!(condition)) { \
            printk(KERN_ERR "VexGraph API Test FAILED: %s\n", message); \
            tests_failed++; \
            return -1; \
        } else { \
            tests_passed++; \
        } \
    } while (0)

#define VEXFS_API_TEST_START(test_name) \
    do { \
        printk(KERN_INFO "VexGraph API Test: Starting %s\n", test_name); \
        start_time = ktime_get_ns(); \
    } while (0)

#define VEXFS_API_TEST_END(test_name) \
    do { \
        end_time = ktime_get_ns(); \
        test_duration = end_time - start_time; \
        total_test_time += test_duration; \
        printk(KERN_INFO "VexGraph API Test: %s completed in %llu ns\n", \
               test_name, test_duration); \
    } while (0)

/* Forward declarations */
static int test_api_manager_lifecycle(void);
static int test_node_crud_operations(void);
static int test_edge_crud_operations(void);
static int test_traversal_algorithms(void);
static int test_query_language(void);
static int test_index_management(void);
static int test_performance_benchmarks(void);
static int test_error_handling(void);
static int test_concurrent_operations(void);

/* Test fixtures */
static struct vexfs_graph_manager *test_graph_mgr = NULL;
static struct vexfs_api_manager *test_api_mgr = NULL;

/*
 * =============================================================================
 * TEST SETUP AND TEARDOWN
 * =============================================================================
 */

/**
 * vexfs_api_test_setup - Set up test environment
 * 
 * Creates test graph manager and API manager for testing.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_test_setup(void)
{
    printk(KERN_INFO "VexGraph API Test: Setting up test environment\n");

    /* Create test graph manager */
    test_graph_mgr = vexfs_graph_manager_create(NULL); /* NULL superblock for testing */
    if (!test_graph_mgr) {
        printk(KERN_ERR "VexGraph API Test: Failed to create test graph manager\n");
        return -ENOMEM;
    }

    /* Initialize graph manager */
    if (vexfs_graph_manager_init(test_graph_mgr) != 0) {
        vexfs_graph_manager_destroy(test_graph_mgr);
        test_graph_mgr = NULL;
        printk(KERN_ERR "VexGraph API Test: Failed to initialize test graph manager\n");
        return -EINVAL;
    }

    /* Create test API manager */
    test_api_mgr = vexfs_api_manager_create(test_graph_mgr);
    if (!test_api_mgr) {
        vexfs_graph_manager_destroy(test_graph_mgr);
        test_graph_mgr = NULL;
        printk(KERN_ERR "VexGraph API Test: Failed to create test API manager\n");
        return -ENOMEM;
    }

    /* Initialize API manager */
    if (vexfs_api_manager_init(test_api_mgr) != 0) {
        vexfs_api_manager_destroy(test_api_mgr);
        vexfs_graph_manager_destroy(test_graph_mgr);
        test_api_mgr = NULL;
        test_graph_mgr = NULL;
        printk(KERN_ERR "VexGraph API Test: Failed to initialize test API manager\n");
        return -EINVAL;
    }

    printk(KERN_INFO "VexGraph API Test: Test environment setup complete\n");
    return 0;
}

/**
 * vexfs_api_test_teardown - Tear down test environment
 * 
 * Cleans up test graph manager and API manager.
 */
static void vexfs_api_test_teardown(void)
{
    printk(KERN_INFO "VexGraph API Test: Tearing down test environment\n");

    if (test_api_mgr) {
        vexfs_api_manager_cleanup(test_api_mgr);
        vexfs_api_manager_destroy(test_api_mgr);
        test_api_mgr = NULL;
    }

    if (test_graph_mgr) {
        vexfs_graph_manager_cleanup(test_graph_mgr);
        vexfs_graph_manager_destroy(test_graph_mgr);
        test_graph_mgr = NULL;
    }

    printk(KERN_INFO "VexGraph API Test: Test environment teardown complete\n");
}

/*
 * =============================================================================
 * API MANAGER LIFECYCLE TESTS
 * =============================================================================
 */

/**
 * test_api_manager_lifecycle - Test API manager creation and destruction
 * 
 * Tests the basic lifecycle operations of the API manager.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_api_manager_lifecycle(void)
{
    u64 start_time, end_time, test_duration;
    struct vexfs_api_stats stats;

    VEXFS_API_TEST_START("API Manager Lifecycle");

    /* Test API manager is properly initialized */
    VEXFS_API_TEST_ASSERT(test_api_mgr != NULL, "API manager should be created");
    VEXFS_API_TEST_ASSERT(test_api_mgr->magic == VEXFS_VEXGRAPH_API_MAGIC,
                           "API manager should have correct magic number");
    VEXFS_API_TEST_ASSERT(test_api_mgr->graph_mgr == test_graph_mgr,
                           "API manager should reference correct graph manager");

    /* Test statistics retrieval */
    vexfs_api_get_statistics(test_api_mgr, &stats);
    VEXFS_API_TEST_ASSERT(stats.total_requests == 0,
                           "Initial request count should be zero");
    VEXFS_API_TEST_ASSERT(stats.successful_requests == 0,
                           "Initial successful request count should be zero");
    VEXFS_API_TEST_ASSERT(stats.failed_requests == 0,
                           "Initial failed request count should be zero");

    /* Test memory allocation functions */
    struct vexfs_api_request *request = vexfs_api_request_alloc(test_api_mgr);
    VEXFS_API_TEST_ASSERT(request != NULL, "Request allocation should succeed");
    VEXFS_API_TEST_ASSERT(request->magic == VEXFS_VEXGRAPH_API_MAGIC,
                           "Request should have correct magic number");

    struct vexfs_api_response *response = vexfs_api_response_alloc(test_api_mgr);
    VEXFS_API_TEST_ASSERT(response != NULL, "Response allocation should succeed");
    VEXFS_API_TEST_ASSERT(response->magic == VEXFS_VEXGRAPH_API_MAGIC,
                           "Response should have correct magic number");

    /* Test cleanup */
    vexfs_api_response_free(test_api_mgr, response);
    vexfs_api_request_free(test_api_mgr, request);

    VEXFS_API_TEST_END("API Manager Lifecycle");
    return 0;
}

/*
 * =============================================================================
 * NODE CRUD OPERATION TESTS
 * =============================================================================
 */

/**
 * test_node_crud_operations - Test node CRUD operations
 * 
 * Tests create, read, update, and delete operations for graph nodes.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_node_crud_operations(void)
{
    u64 start_time, end_time, test_duration;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    u64 created_node_id;
    int result;

    VEXFS_API_TEST_START("Node CRUD Operations");

    /* Allocate request and response */
    request = vexfs_api_request_alloc(test_api_mgr);
    response = vexfs_api_response_alloc(test_api_mgr);
    VEXFS_API_TEST_ASSERT(request != NULL && response != NULL,
                           "Request and response allocation should succeed");

    /* Test node creation */
    request->operation = VEXFS_API_OP_NODE_CREATE;
    request->params.node_create.inode_number = 12345;
    request->params.node_create.node_type = VEXFS_GRAPH_NODE_FILE;
    request->params.node_create.properties_json = "{\"name\":\"test_file\",\"size\":1024}";

    result = vexfs_api_node_create(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Node creation should succeed");
    VEXFS_API_TEST_ASSERT(response->result_code == VEXFS_API_SUCCESS,
                           "Node creation response should indicate success");
    
    created_node_id = response->data.node_create.node_id;
    VEXFS_API_TEST_ASSERT(created_node_id > 0, "Created node should have valid ID");

    /* Test node reading */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_NODE_READ;
    request->params.node_read.node_id = created_node_id;
    request->params.node_read.include_properties = true;
    request->params.node_read.include_edges = true;

    result = vexfs_api_node_read(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Node reading should succeed");
    VEXFS_API_TEST_ASSERT(response->data.node_read.node_id == created_node_id,
                           "Read node should have correct ID");
    VEXFS_API_TEST_ASSERT(response->data.node_read.node_type == VEXFS_GRAPH_NODE_FILE,
                           "Read node should have correct type");

    /* Test node updating */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_NODE_UPDATE;
    request->params.node_update.node_id = created_node_id;
    request->params.node_update.properties_json = "{\"name\":\"updated_file\",\"size\":2048}";
    request->params.node_update.merge_properties = false;

    result = vexfs_api_node_update(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Node updating should succeed");
    VEXFS_API_TEST_ASSERT(response->data.node_update.node_id == created_node_id,
                           "Updated node should have correct ID");

    /* Test node deletion */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_NODE_DELETE;
    request->params.node_delete.node_id = created_node_id;
    request->params.node_delete.cascade_edges = true;

    result = vexfs_api_node_delete(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Node deletion should succeed");
    VEXFS_API_TEST_ASSERT(response->data.node_delete.node_id == created_node_id,
                           "Deleted node should have correct ID");

    /* Verify node is deleted */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_NODE_READ;
    request->params.node_read.node_id = created_node_id;

    result = vexfs_api_node_read(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_ERROR_NOT_FOUND,
                           "Reading deleted node should fail");

    /* Cleanup */
    vexfs_api_response_free(test_api_mgr, response);
    vexfs_api_request_free(test_api_mgr, request);

    VEXFS_API_TEST_END("Node CRUD Operations");
    return 0;
}

/*
 * =============================================================================
 * EDGE CRUD OPERATION TESTS
 * =============================================================================
 */

/**
 * test_edge_crud_operations - Test edge CRUD operations
 * 
 * Tests create, read, update, and delete operations for graph edges.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_edge_crud_operations(void)
{
    u64 start_time, end_time, test_duration;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    u64 source_node_id, target_node_id, created_edge_id;
    int result;

    VEXFS_API_TEST_START("Edge CRUD Operations");

    /* Allocate request and response */
    request = vexfs_api_request_alloc(test_api_mgr);
    response = vexfs_api_response_alloc(test_api_mgr);
    VEXFS_API_TEST_ASSERT(request != NULL && response != NULL,
                           "Request and response allocation should succeed");

    /* Create source node */
    request->operation = VEXFS_API_OP_NODE_CREATE;
    request->params.node_create.inode_number = 11111;
    request->params.node_create.node_type = VEXFS_GRAPH_NODE_DIR;
    request->params.node_create.properties_json = "{\"name\":\"source_dir\"}";

    result = vexfs_api_node_create(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Source node creation should succeed");
    source_node_id = response->data.node_create.node_id;

    /* Create target node */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_NODE_CREATE;
    request->params.node_create.inode_number = 22222;
    request->params.node_create.node_type = VEXFS_GRAPH_NODE_FILE;
    request->params.node_create.properties_json = "{\"name\":\"target_file\"}";

    result = vexfs_api_node_create(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Target node creation should succeed");
    target_node_id = response->data.node_create.node_id;

    /* Test edge creation */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_EDGE_CREATE;
    request->params.edge_create.source_id = source_node_id;
    request->params.edge_create.target_id = target_node_id;
    request->params.edge_create.edge_type = VEXFS_GRAPH_EDGE_CONTAINS;
    request->params.edge_create.weight = 100;
    request->params.edge_create.properties_json = "{\"relationship\":\"contains\"}";

    result = vexfs_api_edge_create(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Edge creation should succeed");
    created_edge_id = response->data.edge_create.edge_id;
    VEXFS_API_TEST_ASSERT(created_edge_id > 0, "Created edge should have valid ID");

    /* Test edge reading */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_EDGE_READ;
    request->params.edge_read.edge_id = created_edge_id;
    request->params.edge_read.include_properties = true;

    result = vexfs_api_edge_read(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Edge reading should succeed");
    VEXFS_API_TEST_ASSERT(response->data.edge_read.edge_id == created_edge_id,
                           "Read edge should have correct ID");
    VEXFS_API_TEST_ASSERT(response->data.edge_read.source_id == source_node_id,
                           "Read edge should have correct source ID");
    VEXFS_API_TEST_ASSERT(response->data.edge_read.target_id == target_node_id,
                           "Read edge should have correct target ID");

    /* Test edge updating */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_EDGE_UPDATE;
    request->params.edge_update.edge_id = created_edge_id;
    request->params.edge_update.weight = 200;
    request->params.edge_update.properties_json = "{\"relationship\":\"updated_contains\"}";

    result = vexfs_api_edge_update(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Edge updating should succeed");

    /* Test edge deletion */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_EDGE_DELETE;
    request->params.edge_delete.edge_id = created_edge_id;

    result = vexfs_api_edge_delete(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Edge deletion should succeed");

    /* Cleanup nodes */
    memset(request, 0, sizeof(struct vexfs_api_request));
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_NODE_DELETE;
    request->params.node_delete.node_id = source_node_id;
    vexfs_api_node_delete(test_api_mgr, request, response);

    request->params.node_delete.node_id = target_node_id;
    vexfs_api_node_delete(test_api_mgr, request, response);

    /* Cleanup */
    vexfs_api_response_free(test_api_mgr, response);
    vexfs_api_request_free(test_api_mgr, request);

    VEXFS_API_TEST_END("Edge CRUD Operations");
    return 0;
}

/*
 * =============================================================================
 * TRAVERSAL ALGORITHM TESTS
 * =============================================================================
 */

/**
 * test_traversal_algorithms - Test graph traversal algorithms
 * 
 * Tests BFS, DFS, and shortest path algorithms.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_traversal_algorithms(void)
{
    u64 start_time, end_time, test_duration;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    u64 node_ids[5];
    int result, i;

    VEXFS_API_TEST_START("Traversal Algorithms");

    /* Allocate request and response */
    request = vexfs_api_request_alloc(test_api_mgr);
    response = vexfs_api_response_alloc(test_api_mgr);
    VEXFS_API_TEST_ASSERT(request != NULL && response != NULL,
                           "Request and response allocation should succeed");

    /* Create a small graph for testing */
    for (i = 0; i < 5; i++) {
        memset(request, 0, sizeof(struct vexfs_api_request));
        memset(response, 0, sizeof(struct vexfs_api_response));
        
        request->magic = VEXFS_VEXGRAPH_API_MAGIC;
        request->operation = VEXFS_API_OP_NODE_CREATE;
        request->params.node_create.inode_number = 30000 + i;
        request->params.node_create.node_type = VEXFS_GRAPH_NODE_FILE;
        request->params.node_create.properties_json = "{\"test\":\"traversal\"}";

        result = vexfs_api_node_create(test_api_mgr, request, response);
        VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Test node creation should succeed");
        node_ids[i] = response->data.node_create.node_id;
    }

    /* Create edges to form a connected graph */
    for (i = 0; i < 4; i++) {
        memset(request, 0, sizeof(struct vexfs_api_request));
        memset(response, 0, sizeof(struct vexfs_api_response));
        
        request->magic = VEXFS_VEXGRAPH_API_MAGIC;
        request->operation = VEXFS_API_OP_EDGE_CREATE;
        request->params.edge_create.source_id = node_ids[i];
        request->params.edge_create.target_id = node_ids[i + 1];
        request->params.edge_create.edge_type = VEXFS_GRAPH_EDGE_REFERENCES;
        request->params.edge_create.weight = 10;

        result = vexfs_api_edge_create(test_api_mgr, request, response);
        VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Test edge creation should succeed");
    }

    /* Test BFS traversal */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_TRAVERSE;
    request->params.traverse.algorithm = VEXFS_GRAPH_TRAVERSAL_BFS;
    request->params.traverse.start_node = node_ids[0];
    request->params.traverse.max_depth = 10;
    request->params.traverse.max_results = 100;

    result = vexfs_api_traverse_bfs(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "BFS traversal should succeed");
    VEXFS_API_TEST_ASSERT(response->data.traverse.result_count > 0,
                           "BFS should find at least one node");

    /* Test DFS traversal */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_TRAVERSE;
    request->params.traverse.algorithm = VEXFS_GRAPH_TRAVERSAL_DFS;
    request->params.traverse.start_node = node_ids[0];
    request->params.traverse.max_depth = 10;
    request->params.traverse.max_results = 100;

    result = vexfs_api_traverse_dfs(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "DFS traversal should succeed");
    VEXFS_API_TEST_ASSERT(response->data.traverse.result_count > 0,
                           "DFS should find at least one node");

    /* Test shortest path */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_TRAVERSE;
    request->params.traverse.algorithm = VEXFS_GRAPH_TRAVERSAL_DIJKSTRA;
    request->params.traverse.start_node = node_ids[0];
    request->params.traverse.end_node = node_ids[4];
    request->params.traverse.max_depth = 10;
    request->params.traverse.max_results = 100;

    result = vexfs_api_shortest_path(test_api_mgr, request, response);
    VEXFS_API_TEST_ASSERT(result == VEXFS_API_SUCCESS, "Shortest path should succeed");
    VEXFS_API_TEST_ASSERT(response->data.traverse.result_count > 0,
                           "Shortest path should find a path");

    /* Cleanup test graph */
    for (i = 0; i < 5; i++) {
        memset(request, 0, sizeof(struct vexfs_api_request));
        request->magic = VEXFS_VEXGRAPH_API_MAGIC;
        request->operation = VEXFS_API_OP_NODE_DELETE;
        request->params.node_delete.node_id = node_ids[i];
        request->params.node_delete.cascade_edges = true;
        vexfs_api_node_delete(test_api_mgr, request, response);
    }

    /* Cleanup */
    vexfs_api_response_free(test_api_mgr, response);
    vexfs_api_request_free(test_api_mgr, request);

    VEXFS_API_TEST_END("Traversal Algorithms");
    return 0;
}

/*
 * =============================================================================
 * QUERY LANGUAGE TESTS
 * =============================================================================
 */

/**
 * test_query_language - Test VexGraph Query Language
 * 
 * Tests query parsing, execution, and optimization.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_query_language(void)
{
    u64 start_time, end_time, test_duration;
    struct vexfs_api_request *request;
    struct vexfs_api_response *response;
    struct vexfs_query_plan plan;
    int result;

    VEXFS_API_TEST_START("Query Language");

    /* Allocate request and response */
    request = vexfs_api_request_alloc(test_api_mgr);
    response = vexfs_api_response_alloc(test_api_mgr);
    VEXFS_API_TEST_ASSERT(request != NULL && response != NULL,
                           "Request and response allocation should succeed");

    /* Test query parsing */
    result = vexfs_api_query_parse("MATCH (n:File) RETURN n", &plan);
    VEXFS_API_TEST_ASSERT(result == 0, "Query parsing should succeed");
    VEXFS_API_TEST_ASSERT(plan.filter.node_type == VEXFS_GRAPH_NODE_FILE,
                           "Parsed query should have correct node type filter");

    /* Test query execution */
    memset(request, 0, sizeof(struct vexfs_api_request));
    memset(response, 0, sizeof(struct vexfs_api_response));
    
    request->magic = VEXFS_VEXGRAPH_API_MAGIC;
    request->operation = VEXFS_API_OP_QUERY;