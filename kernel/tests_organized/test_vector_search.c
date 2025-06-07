/*
 * VexFS v2.0 Vector Search Test Program
 *
 * This program tests the new k-NN search functionality implemented in Phase 2.
 * It demonstrates semantic search capabilities using the new IOCTL interface.
 *
 * Phase 2 Implementation: Vector Query Operations Test
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include <math.h>
#include <errno.h>

/* Include our search header (userspace compatible definitions) */
#include "vexfs_v2_uapi.h"

/* Search operation types */
#define VEXFS_SEARCH_KNN        0x01
#define VEXFS_SEARCH_RANGE      0x02
#define VEXFS_SEARCH_SIMILARITY 0x03

/* Distance metrics */
#define VEXFS_DISTANCE_EUCLIDEAN    0x01
#define VEXFS_DISTANCE_COSINE       0x02
#define VEXFS_DISTANCE_DOT_PRODUCT  0x03
#define VEXFS_DISTANCE_MANHATTAN    0x04

/* Search result structure */
struct vexfs_search_result {
    uint64_t vector_id;
    float distance;
    uint32_t metadata_offset;
    uint32_t reserved;
};

/* k-NN search request */
struct vexfs_knn_query {
    float *query_vector;
    uint32_t dimensions;
    uint32_t k;
    uint32_t distance_metric;
    uint32_t search_flags;
    
    struct vexfs_search_result *results;
    uint32_t results_found;
    
    uint64_t search_time_ns;
    uint32_t vectors_scanned;
    uint32_t index_hits;
};

/* Search statistics */
struct vexfs_search_stats {
    uint64_t total_vectors;
    uint64_t index_size_bytes;
    uint32_t index_type;
    uint32_t index_levels;
    
    uint64_t total_searches;
    uint64_t cache_hits;
    uint64_t cache_misses;
    float avg_search_time_ms;
    
    float index_efficiency;
    uint32_t fragmentation_level;
    uint64_t last_rebuild_time;
};

/* New IOCTL commands for search operations */
#define VEXFS_IOC_KNN_SEARCH        _IOWR(VEXFS_IOC_MAGIC, 10, struct vexfs_knn_query)
#define VEXFS_IOC_SEARCH_STATS      _IOR(VEXFS_IOC_MAGIC, 13, struct vexfs_search_stats)

/* Test data */
static const char *distance_metric_names[] = {
    "Unknown",
    "Euclidean",
    "Cosine",
    "Dot Product",
    "Manhattan"
};

/* Generate a random vector for testing */
void generate_random_vector(float *vector, uint32_t dimensions, int seed)
{
    srand(seed);
    for (uint32_t i = 0; i < dimensions; i++) {
        vector[i] = ((float)rand() / RAND_MAX) * 2.0f - 1.0f;  /* Range: -1.0 to 1.0 */
    }
}

/* Generate a similar vector (for testing similarity search) */
void generate_similar_vector(float *base, float *similar, uint32_t dimensions, float similarity)
{
    for (uint32_t i = 0; i < dimensions; i++) {
        float noise = ((float)rand() / RAND_MAX) * 2.0f - 1.0f;
        similar[i] = base[i] * similarity + noise * (1.0f - similarity) * 0.1f;
    }
}

/* Print vector for debugging */
void print_vector(const char *name, float *vector, uint32_t dimensions, uint32_t max_show)
{
    printf("%s: [", name);
    uint32_t show = (dimensions < max_show) ? dimensions : max_show;
    for (uint32_t i = 0; i < show; i++) {
        printf("%.3f", vector[i]);
        if (i < show - 1) printf(", ");
    }
    if (dimensions > max_show) printf(", ...");
    printf("]\n");
}

/* Test basic k-NN search functionality */
int test_knn_search_basic(int fd)
{
    printf("\n🔍 Testing Basic k-NN Search\n");
    printf("════════════════════════════════════════════════════════════════\n");
    
    const uint32_t dimensions = 4;
    const uint32_t k = 5;
    
    /* Allocate memory for query vector and results */
    float *query_vector = malloc(dimensions * sizeof(float));
    struct vexfs_search_result *results = malloc(k * sizeof(struct vexfs_search_result));
    
    if (!query_vector || !results) {
        printf("❌ Failed to allocate memory\n");
        free(query_vector);
        free(results);
        return -1;
    }
    
    /* Generate a test query vector */
    generate_random_vector(query_vector, dimensions, 42);
    print_vector("Query vector", query_vector, dimensions, 10);
    
    /* Set up k-NN query */
    struct vexfs_knn_query query = {
        .query_vector = query_vector,
        .dimensions = dimensions,
        .k = k,
        .distance_metric = VEXFS_DISTANCE_EUCLIDEAN,
        .search_flags = 0,
        .results = results,
        .results_found = 0
    };
    
    printf("Searching for %u nearest neighbors using %s distance...\n", 
           k, distance_metric_names[query.distance_metric]);
    
    /* Perform the search */
    int ret = ioctl(fd, VEXFS_IOC_KNN_SEARCH, &query);
    
    if (ret == 0) {
        printf("✅ k-NN search completed successfully\n");
        printf("   Results found: %u\n", query.results_found);
        printf("   Search time: %.3f ms\n", query.search_time_ns / 1000000.0);
        printf("   Vectors scanned: %u\n", query.vectors_scanned);
        printf("   Index hits: %u\n", query.index_hits);
        
        printf("\n📊 Search Results:\n");
        for (uint32_t i = 0; i < query.results_found; i++) {
            printf("   %u. Vector ID: %lu, Distance: %.6f\n", 
                   i + 1, query.results[i].vector_id, query.results[i].distance);
        }
    } else {
        printf("❌ k-NN search failed: %d\n", ret);
        if (ret == -ENOSYS) {
            printf("   Note: Search functionality not yet implemented in kernel\n");
        }
    }
    
    free(query_vector);
    free(results);
    
    return ret;
}

/* Test different distance metrics */
int test_distance_metrics(int fd)
{
    printf("\n📏 Testing Different Distance Metrics\n");
    printf("════════════════════════════════════════════════════════════════\n");
    
    const uint32_t dimensions = 8;
    const uint32_t k = 3;
    
    float *query_vector = malloc(dimensions * sizeof(float));
    struct vexfs_search_result *results = malloc(k * sizeof(struct vexfs_search_result));
    
    if (!query_vector || !results) {
        printf("❌ Failed to allocate memory\n");
        free(query_vector);
        free(results);
        return -1;
    }
    
    /* Generate a test query vector */
    generate_random_vector(query_vector, dimensions, 123);
    print_vector("Query vector", query_vector, dimensions, 8);
    
    /* Test each distance metric */
    uint32_t metrics[] = {
        VEXFS_DISTANCE_EUCLIDEAN,
        VEXFS_DISTANCE_COSINE,
        VEXFS_DISTANCE_DOT_PRODUCT,
        VEXFS_DISTANCE_MANHATTAN
    };
    
    for (int m = 0; m < 4; m++) {
        printf("\n🔸 Testing %s distance:\n", distance_metric_names[metrics[m]]);
        
        struct vexfs_knn_query query = {
            .query_vector = query_vector,
            .dimensions = dimensions,
            .k = k,
            .distance_metric = metrics[m],
            .search_flags = 0,
            .results = results,
            .results_found = 0
        };
        
        int ret = ioctl(fd, VEXFS_IOC_KNN_SEARCH, &query);
        
        if (ret == 0) {
            printf("   ✅ Search completed in %.3f ms\n", query.search_time_ns / 1000000.0);
            for (uint32_t i = 0; i < query.results_found; i++) {
                printf("   %u. ID: %lu, Distance: %.6f\n", 
                       i + 1, query.results[i].vector_id, query.results[i].distance);
            }
        } else {
            printf("   ❌ Search failed: %d\n", ret);
        }
    }
    
    free(query_vector);
    free(results);
    
    return 0;
}

/* Test search performance with different vector dimensions */
int test_search_performance(int fd)
{
    printf("\n⚡ Testing Search Performance\n");
    printf("════════════════════════════════════════════════════════════════\n");
    
    uint32_t test_dimensions[] = {4, 16, 64, 128, 256, 512};
    const uint32_t k = 10;
    const int num_tests = sizeof(test_dimensions) / sizeof(test_dimensions[0]);
    
    printf("Testing search performance across different vector dimensions:\n\n");
    printf("Dimensions | Search Time (ms) | Vectors Scanned | Throughput (ops/sec)\n");
    printf("-----------|------------------|-----------------|--------------------\n");
    
    for (int t = 0; t < num_tests; t++) {
        uint32_t dimensions = test_dimensions[t];
        
        float *query_vector = malloc(dimensions * sizeof(float));
        struct vexfs_search_result *results = malloc(k * sizeof(struct vexfs_search_result));
        
        if (!query_vector || !results) {
            printf("❌ Failed to allocate memory for %u dimensions\n", dimensions);
            free(query_vector);
            free(results);
            continue;
        }
        
        generate_random_vector(query_vector, dimensions, t + 1);
        
        struct vexfs_knn_query query = {
            .query_vector = query_vector,
            .dimensions = dimensions,
            .k = k,
            .distance_metric = VEXFS_DISTANCE_EUCLIDEAN,
            .search_flags = 0,
            .results = results,
            .results_found = 0
        };
        
        int ret = ioctl(fd, VEXFS_IOC_KNN_SEARCH, &query);
        
        if (ret == 0) {
            double search_time_ms = query.search_time_ns / 1000000.0;
            double throughput = (query.vectors_scanned * 1000.0) / search_time_ms;
            
            printf("%10u | %15.3f | %15u | %18.0f\n", 
                   dimensions, search_time_ms, query.vectors_scanned, throughput);
        } else {
            printf("%10u | %15s | %15s | %18s\n", 
                   dimensions, "FAILED", "N/A", "N/A");
        }
        
        free(query_vector);
        free(results);
    }
    
    return 0;
}

/* Test search statistics */
int test_search_statistics(int fd)
{
    printf("\n📈 Testing Search Statistics\n");
    printf("════════════════════════════════════════════════════════════════\n");
    
    struct vexfs_search_stats stats;
    int ret = ioctl(fd, VEXFS_IOC_SEARCH_STATS, &stats);
    
    if (ret == 0) {
        printf("✅ Search statistics retrieved successfully\n\n");
        printf("📊 Vector Database Statistics:\n");
        printf("   Total vectors: %lu\n", stats.total_vectors);
        printf("   Index size: %lu bytes (%.2f MB)\n", 
               stats.index_size_bytes, stats.index_size_bytes / (1024.0 * 1024.0));
        printf("   Index type: %u\n", stats.index_type);
        printf("   Index levels: %u\n", stats.index_levels);
        
        printf("\n🔍 Search Performance:\n");
        printf("   Total searches: %lu\n", stats.total_searches);
        printf("   Cache hits: %lu\n", stats.cache_hits);
        printf("   Cache misses: %lu\n", stats.cache_misses);
        printf("   Average search time: %.3f ms\n", stats.avg_search_time_ms);
        
        printf("\n⚙️  Index Quality:\n");
        printf("   Index efficiency: %.2f%%\n", stats.index_efficiency * 100.0);
        printf("   Fragmentation level: %u\n", stats.fragmentation_level);
        printf("   Last rebuild: %lu\n", stats.last_rebuild_time);
        
        if (stats.total_searches > 0) {
            double cache_hit_rate = (double)stats.cache_hits / (stats.cache_hits + stats.cache_misses) * 100.0;
            printf("   Cache hit rate: %.2f%%\n", cache_hit_rate);
        }
    } else {
        printf("❌ Failed to retrieve search statistics: %d\n", ret);
        if (ret == -ENOSYS) {
            printf("   Note: Statistics functionality not yet implemented\n");
        }
    }
    
    return ret;
}

int main(int argc, char *argv[])
{
    printf("🔍 VexFS v2.0 Vector Search Test Program\n");
    printf("═══════════════════════════════════════════════════════════════\n");
    printf("Phase 2 Implementation: Testing k-NN Search and Query Operations\n");
    
    const char *mount_point = "/tmp/vexfs_test";
    if (argc > 1) {
        mount_point = argv[1];
    }
    
    printf("Using VexFS mount point: %s\n", mount_point);
    
    /* Open VexFS mount point */
    int fd = open(mount_point, O_RDONLY);
    if (fd < 0) {
        perror("❌ Failed to open VexFS mount point");
        printf("   Make sure VexFS is mounted at %s\n", mount_point);
        printf("   Usage: %s [mount_point]\n", argv[0]);
        return 1;
    }
    
    printf("✅ VexFS mount point opened successfully\n");
    
    /* Run test suite */
    int tests_passed = 0;
    int total_tests = 4;
    
    if (test_knn_search_basic(fd) == 0) tests_passed++;
    if (test_distance_metrics(fd) == 0) tests_passed++;
    if (test_search_performance(fd) == 0) tests_passed++;
    if (test_search_statistics(fd) == 0) tests_passed++;
    
    /* Summary */
    printf("\n🏁 Test Summary\n");
    printf("════════════════════════════════════════════════════════════════\n");
    printf("Tests passed: %d/%d\n", tests_passed, total_tests);
    
    if (tests_passed == total_tests) {
        printf("🎉 All tests passed! VexFS v2.0 search functionality is working.\n");
    } else if (tests_passed > 0) {
        printf("⚠️  Some tests passed. Search functionality partially implemented.\n");
    } else {
        printf("❌ No tests passed. Search functionality not yet available.\n");
        printf("   This is expected if Phase 2 search implementation is not complete.\n");
    }
    
    printf("\n💡 Next Steps:\n");
    printf("   - Implement kernel-level search operations\n");
    printf("   - Add indexing structures (HNSW, LSH)\n");
    printf("   - Optimize performance for large vector datasets\n");
    printf("   - Add semantic filesystem operations\n");
    
    close(fd);
    return (tests_passed == total_tests) ? 0 : 1;
}