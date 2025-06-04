#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

/* Include the search header for Phase 2 functionality */
#include "vexfs_v2_search.h"

/* IOCTL definitions from kernel module */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_file_info)
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)
#define VEXFS_IOC_KNN_SEARCH         _IOWR(VEXFS_IOC_MAGIC, 5, struct vexfs_knn_query)
#define VEXFS_IOC_RANGE_SEARCH       _IOWR(VEXFS_IOC_MAGIC, 6, struct vexfs_range_query)
#define VEXFS_IOC_SEARCH_STATS       _IOR(VEXFS_IOC_MAGIC, 7, struct vexfs_search_stats)

/* Structures from kernel module */
struct vexfs_vector_file_info {
    uint32_t dimensions;
    uint32_t element_type;
    uint32_t vector_count;
    uint32_t storage_format;
    uint64_t data_offset;
    uint64_t index_offset;
    uint32_t compression_type;
    uint32_t alignment_bytes;
};

struct vexfs_batch_insert_request {
    uint32_t vector_count;
    uint32_t dimensions;
    float *vectors;
    uint64_t *vector_ids;
};

void print_test_header(const char *test_name) {
    printf("\n🔍 %s\n", test_name);
    printf("================================================\n");
}

void print_search_results(struct vexfs_search_result *results, uint32_t count) {
    printf("📊 Search Results (%u found):\n", count);
    for (uint32_t i = 0; i < count; i++) {
        printf("  [%u] Vector ID: %lu, Distance: %u\n", 
               i, results[i].vector_id, results[i].distance);
    }
}

int test_vector_insertion(int fd) {
    print_test_header("Phase 1: Vector Insertion Test");
    
    // Set vector metadata
    struct vexfs_vector_file_info meta = {
        .dimensions = 4,
        .element_type = 0,
        .vector_count = 0,
        .storage_format = 0,
        .data_offset = 0,
        .index_offset = 0,
        .compression_type = 0,
        .alignment_bytes = 32
    };
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) != 0) {
        perror("❌ Failed to set vector metadata");
        return -1;
    }
    printf("✅ Vector metadata set (4 dimensions)\n");
    
    // Insert test vectors
    float vectors[] = {
        1.0, 2.0, 3.0, 4.0,    // Vector 1
        2.0, 3.0, 4.0, 5.0,    // Vector 2
        3.0, 4.0, 5.0, 6.0,    // Vector 3
        1.5, 2.5, 3.5, 4.5,    // Vector 4
        10.0, 11.0, 12.0, 13.0 // Vector 5 (distant)
    };
    uint64_t ids[] = {1, 2, 3, 4, 5};
    
    struct vexfs_batch_insert_request req = {5, 4, vectors, ids};
    if (ioctl(fd, VEXFS_IOC_BATCH_INSERT, &req) != 0) {
        perror("❌ Failed to batch insert vectors");
        return -1;
    }
    printf("✅ Inserted 5 test vectors successfully\n");
    
    return 0;
}

int test_knn_search(int fd) {
    print_test_header("Phase 2: k-NN Search Test");
    
    // Query vector similar to vector 1
    float query_vector[] = {1.1, 2.1, 3.1, 4.1};
    struct vexfs_search_result results[3];
    
    struct vexfs_knn_query knn_query = {
        .query_vector = query_vector,
        .dimensions = 4,
        .k = 3,
        .distance_metric = 0, // Euclidean
        .search_flags = 0,
        .results = results,
        .results_found = 0
    };
    
    printf("🔍 Searching for 3 nearest neighbors to [1.1, 2.1, 3.1, 4.1]\n");
    
    if (ioctl(fd, VEXFS_IOC_KNN_SEARCH, &knn_query) != 0) {
        perror("❌ k-NN search failed");
        return -1;
    }
    
    printf("✅ k-NN search completed in %lu ns\n", knn_query.search_time_ns);
    printf("📈 Vectors scanned: %u\n", knn_query.vectors_scanned);
    print_search_results(results, knn_query.results_found);
    
    return 0;
}

int test_range_search(int fd) {
    print_test_header("Phase 2: Range Search Test");
    
    // Query vector
    float query_vector[] = {2.0, 3.0, 4.0, 5.0};
    struct vexfs_search_result results[10];
    
    struct vexfs_range_query range_query = {
        .query_vector = query_vector,
        .dimensions = 4,
        .max_distance = 1000, // Large range to catch nearby vectors
        .distance_metric = 0, // Euclidean
        .max_results = 10,
        .search_flags = 0,
        .results = results,
        .results_found = 0
    };
    
    printf("🔍 Range search for vectors within distance 1000 of [2.0, 3.0, 4.0, 5.0]\n");
    
    if (ioctl(fd, VEXFS_IOC_RANGE_SEARCH, &range_query) != 0) {
        perror("❌ Range search failed");
        return -1;
    }
    
    printf("✅ Range search completed in %lu ns\n", range_query.search_time_ns);
    printf("📈 Vectors scanned: %u\n", range_query.vectors_scanned);
    print_search_results(results, range_query.results_found);
    
    return 0;
}

int test_search_stats(int fd) {
    print_test_header("Phase 2: Search Statistics Test");
    
    struct vexfs_search_stats stats;
    
    if (ioctl(fd, VEXFS_IOC_SEARCH_STATS, &stats) != 0) {
        perror("❌ Failed to get search statistics");
        return -1;
    }
    
    printf("✅ Search statistics retrieved:\n");
    printf("📊 Total vectors: %lu\n", stats.total_vectors);
    printf("📊 Total searches: %lu\n", stats.total_searches);
    printf("📊 Average search time: %u ms\n", stats.avg_search_time_ms);
    printf("📊 Index size: %lu bytes\n", stats.index_size_bytes);
    printf("📊 Cache hits: %lu\n", stats.cache_hits);
    printf("📊 Cache misses: %lu\n", stats.cache_misses);
    printf("📊 Index efficiency: %u\n", stats.index_efficiency);
    
    return 0;
}

int main() {
    printf("🚀 VexFS v2.0 Phase 2 Search Functionality Test\n");
    printf("===============================================\n");
    printf("Testing comprehensive vector search operations\n");
    
    int fd = open("/tmp/vexfs_test", O_RDONLY);
    if (fd < 0) {
        perror("❌ Failed to open VexFS mount point");
        printf("💡 Make sure VexFS is mounted at /tmp/vexfs_test\n");
        return 1;
    }
    
    // Test Phase 1: Vector insertion
    if (test_vector_insertion(fd) != 0) {
        close(fd);
        return 1;
    }
    
    // Test Phase 2: Search operations
    if (test_knn_search(fd) != 0) {
        close(fd);
        return 1;
    }
    
    if (test_range_search(fd) != 0) {
        close(fd);
        return 1;
    }
    
    if (test_search_stats(fd) != 0) {
        close(fd);
        return 1;
    }
    
    close(fd);
    
    printf("\n🎉 ALL PHASE 2 TESTS COMPLETED SUCCESSFULLY!\n");
    printf("✅ Vector insertion working\n");
    printf("✅ k-NN search working\n");
    printf("✅ Range search working\n");
    printf("✅ Search statistics working\n");
    printf("\n📋 Check dmesg for detailed kernel logs\n");
    printf("🔍 VexFS v2.0 Phase 2 search functionality is operational!\n");
    
    return 0;
}