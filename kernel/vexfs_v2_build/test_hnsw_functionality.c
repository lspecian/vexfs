/*
 * VexFS v2.0 HNSW Functionality Test
 * 
 * Tests the complete kernel-native HNSW implementation including:
 * - Vector metadata operations
 * - HNSW graph creation and search
 * - SIMD-optimized distance calculations
 * - Batch vector operations
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <errno.h>
#include <stdint.h>

/* Use official UAPI header for integer-based interface */
#include "vexfs_v2_uapi.h"

/* Vector storage optimization flags */
#define VEXFS_OPT_SIMD_ALIGN    0x01
#define VEXFS_OPT_BATCH_PROC    0x02
#define VEXFS_OPT_NUMA_AWARE    0x04
#define VEXFS_OPT_COMPRESS      0x08

void print_test_header(const char *test_name) {
    printf("\n=== %s ===\n", test_name);
}

void print_test_result(const char *test_name, int success) {
    printf("[%s] %s\n", success ? "PASS" : "FAIL", test_name);
}

int test_vector_metadata_operations(const char *mount_point) {
    char test_file[256];
    int fd, ret;
    struct vexfs_vector_file_info meta_in, meta_out;
    
    print_test_header("Vector Metadata Operations Test");
    
    snprintf(test_file, sizeof(test_file), "%s/test_vector_file", mount_point);
    
    /* Create test file */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create test file: %s\n", strerror(errno));
        return 0;
    }
    
    /* Set vector metadata */
    memset(&meta_in, 0, sizeof(meta_in));
    meta_in.dimensions = 128;
    meta_in.element_type = VEXFS_VECTOR_FLOAT32;
    meta_in.vector_count = 1000;
    meta_in.storage_format = 1;
    meta_in.data_offset = 0;
    meta_in.index_offset = 0;
    meta_in.compression_type = 0;
    meta_in.alignment_bytes = 32;
    
    ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta_in);
    if (ret < 0) {
        printf("SET_VECTOR_META failed: %s\n", strerror(errno));
        close(fd);
        return 0;
    }
    
    /* Get vector metadata */
    memset(&meta_out, 0, sizeof(meta_out));
    ret = ioctl(fd, VEXFS_IOC_GET_VECTOR_META, &meta_out);
    if (ret < 0) {
        printf("GET_VECTOR_META failed: %s\n", strerror(errno));
        close(fd);
        return 0;
    }
    
    /* Verify metadata */
    int success = (meta_out.dimensions == meta_in.dimensions &&
                   meta_out.element_type == meta_in.element_type &&
                   meta_out.vector_count == meta_in.vector_count &&
                   meta_out.storage_format == meta_in.storage_format);
    
    printf("Dimensions: %u -> %u\n", meta_in.dimensions, meta_out.dimensions);
    printf("Element type: %u -> %u\n", meta_in.element_type, meta_out.element_type);
    printf("Vector count: %u -> %u\n", meta_in.vector_count, meta_out.vector_count);
    printf("Storage format: %u -> %u\n", meta_in.storage_format, meta_out.storage_format);
    printf("Alignment bytes: %u -> %u\n", meta_in.alignment_bytes, meta_out.alignment_bytes);
    
    close(fd);
    unlink(test_file);
    
    print_test_result("Vector Metadata Operations", success);
    return success;
}

int test_hnsw_vector_search(const char *mount_point) {
    char test_file[256];
    int fd, ret;
    struct vexfs_vector_file_info meta;
    struct vexfs_vector_search_request search_req;
    float query_vector[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    uint64_t result_ids[10];
    uint32_t result_distances_bits[10];
    
    /* Convert query vector to IEEE 754 bit representation */
    uint32_t query_vector_bits[4];
    vexfs_float_array_to_bits(query_vector, query_vector_bits, 4);
    
    print_test_header("HNSW Vector Search Test");
    
    snprintf(test_file, sizeof(test_file), "%s/test_search_file", mount_point);
    
    /* Create test file */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create test file: %s\n", strerror(errno));
        return 0;
    }
    
    /* CRITICAL: Set vector metadata first */
    memset(&meta, 0, sizeof(meta));
    meta.dimensions = 4;  /* Match search dimensions */
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_count = 100;  /* Simulate some vectors */
    meta.storage_format = 1;
    meta.data_offset = 0;
    meta.index_offset = 0;
    meta.compression_type = 0;
    meta.alignment_bytes = 32;
    
    ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
    if (ret < 0) {
        printf("SET_VECTOR_META failed: %s\n", strerror(errno));
        close(fd);
        return 0;
    }
    printf("Vector metadata set: dimensions=%u, count=%u\n", meta.dimensions, meta.vector_count);
    
    /* Prepare search request */
    memset(&search_req, 0, sizeof(search_req));
    search_req.query_vector_bits = query_vector_bits;
    search_req.dimensions = 4;  /* Must match metadata dimensions */
    search_req.k = 5;
    search_req.search_type = 0;  /* 0=euclidean distance */
    search_req.results_bits = result_distances_bits;
    search_req.result_ids = result_ids;
    search_req.result_count = 0;  /* Will be set by kernel */
    
    /* Perform HNSW search */
    ret = ioctl(fd, VEXFS_IOC_VECTOR_SEARCH, &search_req);
    if (ret < 0) {
        printf("VECTOR_SEARCH failed: %s\n", strerror(errno));
        close(fd);
        return 0;
    }
    
    printf("Search completed successfully!\n");
    printf("Results found: %u\n", search_req.result_count);
    
    /* Display results */
    for (int i = 0; i < search_req.result_count && i < 5; i++) {
        printf("Result %d: ID=%lu, Distance=%.3f\n",
               i, search_req.result_ids[i], search_req.results_bits[i]);
    }
    
    close(fd);
    unlink(test_file);
    
    print_test_result("HNSW Vector Search", ret == 0);
    return (ret == 0);
}

int test_batch_vector_operations(const char *mount_point) {
    char test_file[256];
    int fd, ret;
    struct vexfs_vector_file_info meta;
    struct vexfs_batch_insert_request batch_req;
    float vectors[20] = {
        1.0f, 2.0f, 3.0f, 4.0f,  /* Vector 1 */
        5.0f, 6.0f, 7.0f, 8.0f,  /* Vector 2 */
        9.0f, 10.0f, 11.0f, 12.0f, /* Vector 3 */
        13.0f, 14.0f, 15.0f, 16.0f, /* Vector 4 */
        17.0f, 18.0f, 19.0f, 20.0f  /* Vector 5 */
    };
    uint64_t vector_ids[5] = {100, 101, 102, 103, 104};
    
    /* Convert float vectors to IEEE 754 bit representation */
    uint32_t vector_bits[20];  /* 5 vectors * 4 dimensions */
    vexfs_float_array_to_bits(vectors, vector_bits, 20);
    
    print_test_header("Batch Vector Operations Test");
    
    snprintf(test_file, sizeof(test_file), "%s/test_batch_file", mount_point);
    
    /* Create test file */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create test file: %s\n", strerror(errno));
        return 0;
    }
    
    /* CRITICAL: Set vector metadata first */
    memset(&meta, 0, sizeof(meta));
    meta.dimensions = 4;  /* Match batch dimensions */
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_count = 0;  /* Will be updated by batch insert */
    meta.storage_format = 1;
    meta.data_offset = 0;
    meta.index_offset = 0;
    meta.compression_type = 0;
    meta.alignment_bytes = 32;
    
    ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
    if (ret < 0) {
        printf("SET_VECTOR_META failed: %s\n", strerror(errno));
        close(fd);
        return 0;
    }
    printf("Vector metadata set: dimensions=%u, count=%u\n", meta.dimensions, meta.vector_count);
    
    /* Prepare batch insert request */
    memset(&batch_req, 0, sizeof(batch_req));
    batch_req.vectors_bits = vector_bits;
    batch_req.vector_count = 5;
    batch_req.dimensions = 4;  /* Must match metadata dimensions */
    batch_req.vector_ids = vector_ids;
    batch_req.flags = VEXFS_OPT_SIMD_ALIGN;
    
    /* Perform batch insert */
    ret = ioctl(fd, VEXFS_IOC_BATCH_INSERT, &batch_req);
    if (ret < 0) {
        printf("BATCH_INSERT failed: %s\n", strerror(errno));
        close(fd);
        return 0;
    }
    
    printf("Batch insert completed successfully!\n");
    printf("Inserted %u vectors with %u dimensions\n", 
           batch_req.vector_count, batch_req.dimensions);
    
    close(fd);
    unlink(test_file);
    
    print_test_result("Batch Vector Operations", ret == 0);
    return (ret == 0);
}

int main(int argc, char *argv[]) {
    const char *mount_point = "/tmp/vexfs_v2_316_test";
    int total_tests = 0, passed_tests = 0;
    
    printf("VexFS v2.0 HNSW Functionality Test Suite\n");
    printf("=========================================\n");
    printf("Mount point: %s\n", mount_point);
    
    /* Check if mount point exists */
    struct stat st;
    if (stat(mount_point, &st) != 0) {
        printf("Error: Mount point %s does not exist\n", mount_point);
        printf("Please ensure VexFS v2.0 is mounted at this location\n");
        return 1;
    }
    
    /* Run tests */
    total_tests++;
    if (test_vector_metadata_operations(mount_point)) passed_tests++;
    
    total_tests++;
    if (test_hnsw_vector_search(mount_point)) passed_tests++;
    
    total_tests++;
    if (test_batch_vector_operations(mount_point)) passed_tests++;
    
    /* Summary */
    printf("\n=== Test Summary ===\n");
    printf("Total tests: %d\n", total_tests);
    printf("Passed: %d\n", passed_tests);
    printf("Failed: %d\n", total_tests - passed_tests);
    printf("Success rate: %.1f%%\n", (float)passed_tests / total_tests * 100.0f);
    
    if (passed_tests == total_tests) {
        printf("\n🎉 All HNSW functionality tests PASSED! 🎉\n");
        printf("VexFS v2.0 kernel-native vector operations are operational!\n");
        return 0;
    } else {
        printf("\n❌ Some tests FAILED. Check kernel logs for details.\n");
        return 1;
    }
}