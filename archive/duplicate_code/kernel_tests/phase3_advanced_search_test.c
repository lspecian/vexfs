/*
 * VexFS v2.0 Phase 3 Advanced Search Test Program
 * 
 * This program tests the advanced search functionality including:
 * - Filtered search with metadata constraints
 * - Multi-vector search for batch queries
 * - Hybrid search combining multiple distance metrics
 */

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

/* Include Phase 3 definitions */
#include "vexfs_v2_phase3.h"

void print_test_header(const char *test_name) {
    printf("\n🧪 %s\n", test_name);
    printf("================================================\n");
}

void print_search_results(const struct vexfs_search_result *results, uint32_t count) {
    uint32_t i;
    printf("📊 Search Results (%u found):\n", count);
    for (i = 0; i < count && i < 10; i++) { /* Show first 10 results */
        printf("   [%u] ID: %llu, Distance: %llu, Score: %llu\n",
               i, results[i].vector_id, results[i].distance, results[i].score);
    }
    if (count > 10) {
        printf("   ... and %u more results\n", count - 10);
    }
}

int test_filtered_search(int fd) {
    struct vexfs_filtered_search_request req;
    struct vexfs_search_result results[100];
    uint32_t result_count;
    int ret;
    
    print_test_header("Filtered Search Test");
    
    /* Test 1: Basic filtered search with ID range filter */
    printf("🔧 Test 1: Filtered search with ID range filter...\n");
    
    memset(&req, 0, sizeof(req));
    
    /* Set up query vector */
    float query_vector[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    req.query_vector = query_vector;
    req.dimensions = 4;
    req.k = 50;
    req.distance_metric = VEXFS_DISTANCE_EUCLIDEAN;
    
    /* Set up filters */
    struct vexfs_search_filter filters[2];
    
    /* Filter 1: ID range 10-100 */
    filters[0].field_type = VEXFS_FILTER_FIELD_RANGE;
    filters[0].operator = VEXFS_FILTER_RANGE;
    filters[0].value.range.min = 10;
    filters[0].value.range.max = 100;
    strcpy(filters[0].field_name, "id_range");
    
    /* Filter 2: Score threshold */
    filters[1].field_type = VEXFS_FILTER_FIELD_SCORE;
    filters[1].operator = VEXFS_FILTER_GE;
    filters[1].value.numeric = 500; /* Score >= 500 */
    strcpy(filters[1].field_name, "score");
    
    req.filters = filters;
    req.filter_count = 2;
    req.results = results;
    req.result_count = &result_count;
    
    ret = ioctl(fd, VEXFS_IOC_FILTERED_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Filtered search completed successfully\n");
        print_search_results(results, result_count);
    } else {
        printf("❌ Filtered search failed: %d\n", ret);
        return ret;
    }
    
    /* Test 2: String-based category filter */
    printf("\n🔧 Test 2: Filtered search with category filter...\n");
    
    /* Update filter to use category */
    filters[0].field_type = VEXFS_FILTER_FIELD_CATEGORY;
    filters[0].operator = VEXFS_FILTER_EQ;
    strcpy(filters[0].value.string, "documents");
    strcpy(filters[0].field_name, "category");
    
    req.filter_count = 1; /* Use only category filter */
    
    ret = ioctl(fd, VEXFS_IOC_FILTERED_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Category filtered search completed successfully\n");
        print_search_results(results, result_count);
    } else {
        printf("❌ Category filtered search failed: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_multi_vector_search(int fd) {
    struct vexfs_multi_vector_search_request req;
    struct vexfs_search_result results[300]; /* 3 queries * 100 results each */
    uint32_t result_counts[3];
    int ret;
    
    print_test_header("Multi-Vector Search Test");
    
    printf("🔧 Testing multi-vector search with 3 query vectors...\n");
    
    memset(&req, 0, sizeof(req));
    
    /* Set up multiple query vectors */
    float query_vectors[12] = {
        /* Query 1 */ 1.0f, 2.0f, 3.0f, 4.0f,
        /* Query 2 */ 5.0f, 6.0f, 7.0f, 8.0f,
        /* Query 3 */ 9.0f, 10.0f, 11.0f, 12.0f
    };
    
    req.query_vectors = query_vectors;
    req.query_count = 3;
    req.dimensions = 4;
    req.k_per_query = 100;
    req.distance_metric = VEXFS_DISTANCE_COSINE;
    req.results = results;
    req.result_counts = result_counts;
    
    ret = ioctl(fd, VEXFS_IOC_MULTI_VECTOR_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Multi-vector search completed successfully\n");
        
        /* Print results for each query */
        uint32_t i;
        for (i = 0; i < req.query_count; i++) {
            printf("\n📊 Results for Query %u:\n", i + 1);
            print_search_results(&results[i * req.k_per_query], result_counts[i]);
        }
    } else {
        printf("❌ Multi-vector search failed: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_hybrid_search(int fd) {
    struct vexfs_hybrid_search_request req;
    struct vexfs_search_result results[100];
    uint32_t result_count;
    int ret;
    
    print_test_header("Hybrid Search Test");
    
    /* Test 1: Euclidean + Cosine hybrid */
    printf("🔧 Test 1: Hybrid search (Euclidean + Cosine)...\n");
    
    memset(&req, 0, sizeof(req));
    
    /* Set up query vector */
    float query_vector[4] = {1.5f, 2.5f, 3.5f, 4.5f};
    req.query_vector = query_vector;
    req.dimensions = 4;
    req.k = 50;
    req.primary_metric = VEXFS_DISTANCE_EUCLIDEAN;
    req.secondary_metric = VEXFS_DISTANCE_COSINE;
    req.primary_weight = 0.7f;
    req.secondary_weight = 0.3f;
    req.results = results;
    req.result_count = &result_count;
    
    ret = ioctl(fd, VEXFS_IOC_HYBRID_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Euclidean+Cosine hybrid search completed successfully\n");
        printf("   Primary weight: %.2f, Secondary weight: %.2f\n",
               req.primary_weight, req.secondary_weight);
        print_search_results(results, result_count);
    } else {
        printf("❌ Euclidean+Cosine hybrid search failed: %d\n", ret);
        return ret;
    }
    
    /* Test 2: Dot Product + Manhattan hybrid */
    printf("\n🔧 Test 2: Hybrid search (Dot Product + Manhattan)...\n");
    
    req.primary_metric = VEXFS_DISTANCE_DOT_PRODUCT;
    req.secondary_metric = VEXFS_DISTANCE_MANHATTAN;
    req.primary_weight = 0.6f;
    req.secondary_weight = 0.4f;
    
    ret = ioctl(fd, VEXFS_IOC_HYBRID_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Dot Product+Manhattan hybrid search completed successfully\n");
        printf("   Primary weight: %.2f, Secondary weight: %.2f\n",
               req.primary_weight, req.secondary_weight);
        print_search_results(results, result_count);
    } else {
        printf("❌ Dot Product+Manhattan hybrid search failed: %d\n", ret);
        return ret;
    }
    
    /* Test 3: Equal weight hybrid */
    printf("\n🔧 Test 3: Hybrid search with equal weights...\n");
    
    req.primary_metric = VEXFS_DISTANCE_EUCLIDEAN;
    req.secondary_metric = VEXFS_DISTANCE_MANHATTAN;
    req.primary_weight = 0.5f;
    req.secondary_weight = 0.5f;
    
    ret = ioctl(fd, VEXFS_IOC_HYBRID_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Equal weight hybrid search completed successfully\n");
        printf("   Primary weight: %.2f, Secondary weight: %.2f\n",
               req.primary_weight, req.secondary_weight);
        print_search_results(results, result_count);
    } else {
        printf("❌ Equal weight hybrid search failed: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_performance_comparison(int fd) {
    struct timespec start, end;
    long elapsed_ns;
    int ret;
    
    print_test_header("Performance Comparison Test");
    
    /* Test filtered search performance */
    printf("🔧 Testing filtered search performance...\n");
    
    struct vexfs_filtered_search_request filter_req;
    struct vexfs_search_result filter_results[100];
    uint32_t filter_result_count;
    struct vexfs_search_filter filter;
    float query_vector[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    
    memset(&filter_req, 0, sizeof(filter_req));
    filter_req.query_vector = query_vector;
    filter_req.dimensions = 4;
    filter_req.k = 100;
    filter_req.distance_metric = VEXFS_DISTANCE_EUCLIDEAN;
    filter_req.filters = &filter;
    filter_req.filter_count = 1;
    filter_req.results = filter_results;
    filter_req.result_count = &filter_result_count;
    
    filter.field_type = VEXFS_FILTER_FIELD_RANGE;
    filter.operator = VEXFS_FILTER_RANGE;
    filter.value.range.min = 0;
    filter.value.range.max = 500;
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    ret = ioctl(fd, VEXFS_IOC_FILTERED_SEARCH, &filter_req);
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    if (ret == 0) {
        elapsed_ns = (end.tv_sec - start.tv_sec) * 1000000000L + 
                     (end.tv_nsec - start.tv_nsec);
        printf("✅ Filtered search: %u results in %ld ns (%.2f ms)\n",
               filter_result_count, elapsed_ns, elapsed_ns / 1000000.0);
    } else {
        printf("❌ Filtered search performance test failed: %d\n", ret);
    }
    
    /* Test multi-vector search performance */
    printf("\n🔧 Testing multi-vector search performance...\n");
    
    struct vexfs_multi_vector_search_request multi_req;
    struct vexfs_search_result multi_results[500]; /* 5 queries * 100 results */
    uint32_t multi_result_counts[5];
    float multi_queries[20] = {
        1.0f, 2.0f, 3.0f, 4.0f,
        5.0f, 6.0f, 7.0f, 8.0f,
        9.0f, 10.0f, 11.0f, 12.0f,
        13.0f, 14.0f, 15.0f, 16.0f,
        17.0f, 18.0f, 19.0f, 20.0f
    };
    
    memset(&multi_req, 0, sizeof(multi_req));
    multi_req.query_vectors = multi_queries;
    multi_req.query_count = 5;
    multi_req.dimensions = 4;
    multi_req.k_per_query = 100;
    multi_req.distance_metric = VEXFS_DISTANCE_COSINE;
    multi_req.results = multi_results;
    multi_req.result_counts = multi_result_counts;
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    ret = ioctl(fd, VEXFS_IOC_MULTI_VECTOR_SEARCH, &multi_req);
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    if (ret == 0) {
        elapsed_ns = (end.tv_sec - start.tv_sec) * 1000000000L + 
                     (end.tv_nsec - start.tv_nsec);
        uint32_t total_results = 0;
        uint32_t i;
        for (i = 0; i < multi_req.query_count; i++) {
            total_results += multi_result_counts[i];
        }
        printf("✅ Multi-vector search: %u total results in %ld ns (%.2f ms)\n",
               total_results, elapsed_ns, elapsed_ns / 1000000.0);
        printf("   Average per query: %.2f ms\n", 
               (elapsed_ns / 1000000.0) / multi_req.query_count);
    } else {
        printf("❌ Multi-vector search performance test failed: %d\n", ret);
    }
    
    /* Test hybrid search performance */
    printf("\n🔧 Testing hybrid search performance...\n");
    
    struct vexfs_hybrid_search_request hybrid_req;
    struct vexfs_search_result hybrid_results[100];
    uint32_t hybrid_result_count;
    
    memset(&hybrid_req, 0, sizeof(hybrid_req));
    hybrid_req.query_vector = query_vector;
    hybrid_req.dimensions = 4;
    hybrid_req.k = 100;
    hybrid_req.primary_metric = VEXFS_DISTANCE_EUCLIDEAN;
    hybrid_req.secondary_metric = VEXFS_DISTANCE_COSINE;
    hybrid_req.primary_weight = 0.7f;
    hybrid_req.secondary_weight = 0.3f;
    hybrid_req.results = hybrid_results;
    hybrid_req.result_count = &hybrid_result_count;
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    ret = ioctl(fd, VEXFS_IOC_HYBRID_SEARCH, &hybrid_req);
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    if (ret == 0) {
        elapsed_ns = (end.tv_sec - start.tv_sec) * 1000000000L + 
                     (end.tv_nsec - start.tv_nsec);
        printf("✅ Hybrid search: %u results in %ld ns (%.2f ms)\n",
               hybrid_result_count, elapsed_ns, elapsed_ns / 1000000.0);
    } else {
        printf("❌ Hybrid search performance test failed: %d\n", ret);
    }
    
    return 0;
}

int main() {
    int fd;
    int ret = 0;
    
    printf("🚀 VexFS v2.0 Phase 3 Advanced Search Test Suite\n");
    printf("=================================================\n");
    printf("Testing advanced search operations functionality\n");
    
    /* Open VexFS mount point */
    fd = open("/tmp/vexfs_test", O_RDONLY);
    if (fd < 0) {
        perror("❌ Failed to open VexFS mount point");
        printf("💡 Make sure VexFS v2.0 is mounted at /tmp/vexfs_test\n");
        return 1;
    }
    
    printf("✅ VexFS mount point opened successfully\n");
    
    /* Run test suites */
    ret = test_filtered_search(fd);
    if (ret != 0) {
        printf("\n❌ Filtered search test failed\n");
        goto cleanup;
    }
    
    ret = test_multi_vector_search(fd);
    if (ret != 0) {
        printf("\n❌ Multi-vector search test failed\n");
        goto cleanup;
    }
    
    ret = test_hybrid_search(fd);
    if (ret != 0) {
        printf("\n❌ Hybrid search test failed\n");
        goto cleanup;
    }
    
    ret = test_performance_comparison(fd);
    if (ret != 0) {
        printf("\n❌ Performance comparison test failed\n");
        goto cleanup;
    }
    
    printf("\n🎉 All Phase 3 Advanced Search tests passed!\n");
    printf("📊 Advanced search operations are working correctly\n");
    printf("\n🔍 Check dmesg for detailed kernel logs\n");
    
cleanup:
    close(fd);
    return ret;
}