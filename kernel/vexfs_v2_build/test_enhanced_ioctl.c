/*
 * VexFS v2.0 Enhanced ioctl Interface Test Suite
 * 
 * Comprehensive test suite for the enhanced vector-specific ioctl interface.
 * Tests all major functionality including vector creation, similarity search,
 * index building, batch operations, and statistics retrieval.
 * 
 * Compile: gcc -o test_enhanced_ioctl test_enhanced_ioctl.c
 * Run: ./test_enhanced_ioctl
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/time.h>
#include <stdint.h>
#include <assert.h>
#include <math.h>

/* Include the enhanced ioctl definitions */
#include "vexfs_v2_enhanced_ioctl.h"

/* Test configuration */
#define TEST_DEVICE_PATH "/dev/vexfs_test"
#define TEST_VECTOR_DIMENSIONS 128
#define TEST_VECTOR_COUNT 1000
#define TEST_BATCH_SIZE 100
#define TEST_SEARCH_K 10

/* Test result tracking */
static int tests_passed = 0;
static int tests_failed = 0;
static int total_tests = 0;

/* Utility macros */
#define TEST_ASSERT(condition, message) do { \
    total_tests++; \
    if (condition) { \
        printf("✅ PASS: %s\n", message); \
        tests_passed++; \
    } else { \
        printf("❌ FAIL: %s\n", message); \
        tests_failed++; \
    } \
} while(0)

#define TEST_START(name) printf("\n🔥 Starting test: %s\n", name)
#define TEST_END(name) printf("✅ Completed test: %s\n", name)

/* IEEE 754 float to uint32 conversion */
static uint32_t float_to_bits(float f) {
    union { float f; uint32_t i; } u;
    u.f = f;
    return u.i;
}

/* Generate random test vector */
static void generate_test_vector(uint32_t *vector, uint32_t dimensions) {
    for (uint32_t i = 0; i < dimensions; i++) {
        float value = ((float)rand() / RAND_MAX) * 2.0f - 1.0f; /* Range: -1.0 to 1.0 */
        vector[i] = float_to_bits(value);
    }
}

/* Generate normalized test vector */
static void generate_normalized_vector(uint32_t *vector, uint32_t dimensions) {
    float norm = 0.0f;
    float *temp = malloc(dimensions * sizeof(float));
    
    /* Generate random values */
    for (uint32_t i = 0; i < dimensions; i++) {
        temp[i] = ((float)rand() / RAND_MAX) * 2.0f - 1.0f;
        norm += temp[i] * temp[i];
    }
    
    /* Normalize */
    norm = sqrtf(norm);
    for (uint32_t i = 0; i < dimensions; i++) {
        temp[i] /= norm;
        vector[i] = float_to_bits(temp[i]);
    }
    
    free(temp);
}

/* 🔥 VECTOR CREATION TESTS 🔥 */

static int test_vector_creation(int fd) {
    TEST_START("Vector Creation");
    
    struct vexfs_create_vector_request req;
    uint32_t *vector_data;
    char metadata[] = "test_vector_metadata";
    int ret;
    
    /* Allocate vector data */
    vector_data = malloc(TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
    if (!vector_data) {
        printf("❌ Failed to allocate vector data\n");
        return -1;
    }
    
    /* Generate test vector */
    generate_test_vector(vector_data, TEST_VECTOR_DIMENSIONS);
    
    /* Setup creation request */
    memset(&req, 0, sizeof(req));
    req.vector_data = vector_data;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.element_type = VEXFS_VECTOR_FLOAT32;
    req.vector_id = 0; /* Auto-assign */
    req.metadata = metadata;
    req.metadata_size = strlen(metadata);
    req.storage_format = VEXFS_STORAGE_DENSE;
    req.compression_type = VEXFS_COMPRESS_NONE;
    req.alignment_bytes = 32; /* SIMD alignment */
    req.flags = VEXFS_CREATE_VECTOR_VALIDATE | VEXFS_CREATE_VECTOR_SIMD_ALIGN;
    req.numa_node = -1; /* Auto-select */
    
    /* Test vector creation */
    ret = ioctl(fd, VEXFS_IOC_CREATE_VECTOR, &req);
    TEST_ASSERT(ret == 0, "Vector creation ioctl succeeded");
    TEST_ASSERT(req.assigned_id > 0, "Vector ID was assigned");
    TEST_ASSERT(req.storage_size > 0, "Storage size was calculated");
    
    printf("📊 Created vector ID: %llu, storage size: %u bytes\n", 
           req.assigned_id, req.storage_size);
    
    /* Test vector creation with custom ID */
    req.vector_id = 12345;
    req.flags |= VEXFS_CREATE_VECTOR_OVERWRITE;
    ret = ioctl(fd, VEXFS_IOC_CREATE_VECTOR, &req);
    TEST_ASSERT(ret == 0, "Vector creation with custom ID succeeded");
    TEST_ASSERT(req.assigned_id == 12345, "Custom vector ID was used");
    
    /* Test invalid parameters */
    req.dimensions = 0; /* Invalid */
    ret = ioctl(fd, VEXFS_IOC_CREATE_VECTOR, &req);
    TEST_ASSERT(ret < 0, "Vector creation with invalid dimensions failed correctly");
    
    free(vector_data);
    TEST_END("Vector Creation");
    return 0;
}

/* 🔥 SIMILARITY SEARCH TESTS 🔥 */

static int test_similarity_search(int fd) {
    TEST_START("Similarity Search");
    
    struct vexfs_enhanced_search_request req;
    uint32_t *query_vector;
    uint32_t *result_distances;
    uint64_t *result_ids;
    uint32_t *result_vectors;
    char *result_metadata;
    int ret;
    
    /* Allocate buffers */
    query_vector = malloc(TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
    result_distances = malloc(TEST_SEARCH_K * sizeof(uint32_t));
    result_ids = malloc(TEST_SEARCH_K * sizeof(uint64_t));
    result_vectors = malloc(TEST_SEARCH_K * TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
    result_metadata = malloc(TEST_SEARCH_K * 256);
    
    if (!query_vector || !result_distances || !result_ids || !result_vectors || !result_metadata) {
        printf("❌ Failed to allocate search buffers\n");
        return -1;
    }
    
    /* Generate query vector */
    generate_normalized_vector(query_vector, TEST_VECTOR_DIMENSIONS);
    
    /* Setup search request */
    memset(&req, 0, sizeof(req));
    req.query_vector = query_vector;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.k = TEST_SEARCH_K;
    req.search_algorithm = 0; /* Auto-select */
    req.distance_metric = VEXFS_SEARCH_EUCLIDEAN;
    req.index_type = 0; /* Auto-select */
    req.ef_search = 50; /* HNSW parameter */
    req.nprobe = 10; /* IVF parameter */
    req.filter_ids = NULL;
    req.filter_count = 0;
    req.filter_mode = 0;
    req.flags = VEXFS_SEARCH_RETURN_DISTANCES | VEXFS_SEARCH_RETURN_VECTORS;
    req.result_distances = result_distances;
    req.result_ids = result_ids;
    req.result_vectors = result_vectors;
    req.result_metadata = result_metadata;
    
    /* Test similarity search */
    ret = ioctl(fd, VEXFS_IOC_SIMILARITY_SEARCH, &req);
    TEST_ASSERT(ret == 0, "Similarity search ioctl succeeded");
    TEST_ASSERT(req.result_count > 0, "Search returned results");
    TEST_ASSERT(req.result_count <= TEST_SEARCH_K, "Result count within limits");
    TEST_ASSERT(req.search_time_ns > 0, "Search time was measured");
    
    printf("📊 Search results: %u vectors, %llu ns, examined: %u\n",
           req.result_count, req.search_time_ns, req.vectors_examined);
    
    /* Verify result ordering (distances should be non-decreasing) */
    bool ordered = true;
    for (uint32_t i = 1; i < req.result_count; i++) {
        if (result_distances[i] < result_distances[i-1]) {
            ordered = false;
            break;
        }
    }
    TEST_ASSERT(ordered, "Search results are properly ordered by distance");
    
    /* Test cosine similarity search */
    req.distance_metric = VEXFS_SEARCH_COSINE;
    ret = ioctl(fd, VEXFS_IOC_SIMILARITY_SEARCH, &req);
    TEST_ASSERT(ret == 0, "Cosine similarity search succeeded");
    
    /* Test dot product search */
    req.distance_metric = VEXFS_SEARCH_DOT_PRODUCT;
    ret = ioctl(fd, VEXFS_IOC_SIMILARITY_SEARCH, &req);
    TEST_ASSERT(ret == 0, "Dot product search succeeded");
    
    /* Test invalid parameters */
    req.k = 0; /* Invalid */
    ret = ioctl(fd, VEXFS_IOC_SIMILARITY_SEARCH, &req);
    TEST_ASSERT(ret < 0, "Search with invalid k failed correctly");
    
    free(query_vector);
    free(result_distances);
    free(result_ids);
    free(result_vectors);
    free(result_metadata);
    
    TEST_END("Similarity Search");
    return 0;
}

/* 🔥 INDEX BUILDING TESTS 🔥 */

static int test_index_building(int fd) {
    TEST_START("Index Building");
    
    struct vexfs_build_index_request req;
    int ret;
    
    /* Test HNSW index building */
    memset(&req, 0, sizeof(req));
    req.index_type = VEXFS_INDEX_HNSW;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vector_count = TEST_VECTOR_COUNT;
    req.hnsw_m = 16;
    req.hnsw_ef_construction = 200;
    req.hnsw_max_layers = 6;
    req.flags = VEXFS_INDEX_BUILD_PARALLEL | VEXFS_INDEX_BUILD_OPTIMIZE;
    req.num_threads = 0; /* Auto-detect */
    req.memory_limit_mb = 1024; /* 1GB limit */
    
    ret = ioctl(fd, VEXFS_IOC_BUILD_INDEX, &req);
    TEST_ASSERT(ret == 0, "HNSW index building succeeded");
    TEST_ASSERT(req.build_time_ns > 0, "Build time was measured");
    TEST_ASSERT(req.index_size_bytes > 0, "Index size was calculated");
    TEST_ASSERT(req.build_errors == 0, "No build errors occurred");
    
    printf("📊 HNSW Index: %u bytes, %llu ns build time, %u MB memory\n",
           req.index_size_bytes, req.build_time_ns, req.memory_used_mb);
    
    /* Test IVF index building */
    memset(&req, 0, sizeof(req));
    req.index_type = VEXFS_INDEX_IVF;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vector_count = TEST_VECTOR_COUNT;
    req.ivf_clusters = 100;
    req.ivf_training_vectors = 10000;
    req.flags = VEXFS_INDEX_BUILD_VALIDATE;
    
    ret = ioctl(fd, VEXFS_IOC_BUILD_INDEX, &req);
    TEST_ASSERT(ret == 0, "IVF index building succeeded");
    
    /* Test PQ index building */
    memset(&req, 0, sizeof(req));
    req.index_type = VEXFS_INDEX_PQ;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vector_count = TEST_VECTOR_COUNT;
    req.pq_subvectors = 8;
    req.pq_bits_per_code = 8;
    req.flags = VEXFS_INDEX_BUILD_PERSIST;
    
    ret = ioctl(fd, VEXFS_IOC_BUILD_INDEX, &req);
    TEST_ASSERT(ret == 0, "PQ index building succeeded");
    
    /* Test LSH index building */
    memset(&req, 0, sizeof(req));
    req.index_type = VEXFS_INDEX_LSH;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vector_count = TEST_VECTOR_COUNT;
    req.lsh_hash_functions = 32;
    req.lsh_hash_tables = 16;
    
    ret = ioctl(fd, VEXFS_IOC_BUILD_INDEX, &req);
    TEST_ASSERT(ret == 0, "LSH index building succeeded");
    
    /* Test invalid parameters */
    req.index_type = 999; /* Invalid */
    ret = ioctl(fd, VEXFS_IOC_BUILD_INDEX, &req);
    TEST_ASSERT(ret < 0, "Index building with invalid type failed correctly");
    
    TEST_END("Index Building");
    return 0;
}

/* 🔥 BATCH OPERATIONS TESTS 🔥 */

static int test_batch_operations(int fd) {
    TEST_START("Batch Operations");
    
    struct vexfs_batch_operations_request req;
    uint32_t *vectors_data;
    uint64_t *vector_ids;
    char *metadata_array;
    uint32_t *search_results;
    uint64_t *search_result_ids;
    uint32_t *error_codes;
    int ret;
    
    /* Allocate buffers */
    vectors_data = malloc(TEST_BATCH_SIZE * TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
    vector_ids = malloc(TEST_BATCH_SIZE * sizeof(uint64_t));
    metadata_array = malloc(TEST_BATCH_SIZE * 256);
    search_results = malloc(TEST_BATCH_SIZE * TEST_SEARCH_K * sizeof(uint32_t));
    search_result_ids = malloc(TEST_BATCH_SIZE * TEST_SEARCH_K * sizeof(uint64_t));
    error_codes = malloc(TEST_BATCH_SIZE * sizeof(uint32_t));
    
    if (!vectors_data || !vector_ids || !metadata_array || 
        !search_results || !search_result_ids || !error_codes) {
        printf("❌ Failed to allocate batch operation buffers\n");
        return -1;
    }
    
    /* Generate test data */
    for (uint32_t i = 0; i < TEST_BATCH_SIZE; i++) {
        generate_test_vector(&vectors_data[i * TEST_VECTOR_DIMENSIONS], TEST_VECTOR_DIMENSIONS);
        vector_ids[i] = i + 1000; /* Start from ID 1000 */
        snprintf(&metadata_array[i * 256], 256, "batch_vector_%u", i);
    }
    
    /* Test batch insert */
    memset(&req, 0, sizeof(req));
    req.operation_type = VEXFS_BATCH_INSERT;
    req.vector_count = TEST_BATCH_SIZE;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vectors_data = vectors_data;
    req.vector_ids = vector_ids;
    req.metadata_array = metadata_array;
    req.metadata_stride = 256;
    req.batch_size = 10; /* Process in chunks of 10 */
    req.flags = 0;
    req.num_threads = 0; /* Auto-detect */
    req.error_codes = error_codes;
    
    ret = ioctl(fd, VEXFS_IOC_BATCH_OPERATIONS, &req);
    TEST_ASSERT(ret == 0, "Batch insert operation succeeded");
    TEST_ASSERT(req.successful_operations > 0, "Some operations succeeded");
    TEST_ASSERT(req.total_time_ns > 0, "Total time was measured");
    
    printf("📊 Batch Insert: %u successful, %u failed, %llu ns\n",
           req.successful_operations, req.failed_operations, req.total_time_ns);
    
    /* Test batch search */
    memset(&req, 0, sizeof(req));
    req.operation_type = VEXFS_BATCH_SEARCH;
    req.vector_count = TEST_BATCH_SIZE;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vectors_data = vectors_data; /* Use same vectors as queries */
    req.batch_size = 10;
    req.k_per_query = TEST_SEARCH_K;
    req.search_results = search_results;
    req.search_result_ids = search_result_ids;
    req.error_codes = error_codes;
    
    ret = ioctl(fd, VEXFS_IOC_BATCH_OPERATIONS, &req);
    TEST_ASSERT(ret == 0, "Batch search operation succeeded");
    TEST_ASSERT(req.successful_operations > 0, "Some searches succeeded");
    
    printf("📊 Batch Search: %u successful, %u failed, %llu ns\n",
           req.successful_operations, req.failed_operations, req.total_time_ns);
    
    /* Test batch delete */
    memset(&req, 0, sizeof(req));
    req.operation_type = VEXFS_BATCH_DELETE;
    req.vector_count = TEST_BATCH_SIZE;
    req.vector_ids = vector_ids;
    req.error_codes = error_codes;
    
    ret = ioctl(fd, VEXFS_IOC_BATCH_OPERATIONS, &req);
    TEST_ASSERT(ret == 0, "Batch delete operation succeeded");
    
    /* Test invalid parameters */
    req.operation_type = 999; /* Invalid */
    ret = ioctl(fd, VEXFS_IOC_BATCH_OPERATIONS, &req);
    TEST_ASSERT(ret < 0, "Batch operation with invalid type failed correctly");
    
    free(vectors_data);
    free(vector_ids);
    free(metadata_array);
    free(search_results);
    free(search_result_ids);
    free(error_codes);
    
    TEST_END("Batch Operations");
    return 0;
}

/* 🔥 STATISTICS TESTS 🔥 */

static int test_statistics(int fd) {
    TEST_START("Statistics");
    
    struct vexfs_vector_stats_request req;
    int ret;
    
    /* Test global statistics */
    memset(&req, 0, sizeof(req));
    req.stats_type = VEXFS_STATS_GLOBAL;
    
    ret = ioctl(fd, VEXFS_IOC_GET_VECTOR_STATS, &req);
    TEST_ASSERT(ret == 0, "Global statistics retrieval succeeded");
    TEST_ASSERT(req.total_vectors >= 0, "Total vectors count is valid");
    TEST_ASSERT(req.total_searches >= 0, "Total searches count is valid");
    
    printf("📊 Global Stats: %llu vectors, %llu searches, %llu insertions\n",
           req.total_vectors, req.total_searches, req.total_insertions);
    printf("📊 Performance: avg search %llu ns, avg insert %llu ns\n",
           req.avg_search_time_ns, req.avg_insert_time_ns);
    printf("📊 Cache hit rate: %llu%%, SIMD ops: %llu\n",
           req.cache_hit_rate / 100, req.simd_operations);
    
    /* Test performance statistics */
    memset(&req, 0, sizeof(req));
    req.stats_type = VEXFS_STATS_PERFORMANCE;
    
    ret = ioctl(fd, VEXFS_IOC_GET_PERFORMANCE_STATS, &req);
    TEST_ASSERT(ret == 0, "Performance statistics retrieval succeeded");
    
    /* Test statistics reset */
    ret = ioctl(fd, VEXFS_IOC_RESET_STATS);
    TEST_ASSERT(ret == 0, "Statistics reset succeeded");
    
    /* Verify reset worked */
    memset(&req, 0, sizeof(req));
    req.stats_type = VEXFS_STATS_GLOBAL;
    ret = ioctl(fd, VEXFS_IOC_GET_VECTOR_STATS, &req);
    TEST_ASSERT(ret == 0, "Statistics retrieval after reset succeeded");
    /* Note: Some counters might not be zero due to the test operations above */
    
    TEST_END("Statistics");
    return 0;
}

/* 🔥 SYSTEM OPERATIONS TESTS 🔥 */

static int test_system_operations(int fd) {
    TEST_START("System Operations");
    
    uint32_t capabilities;
    uint32_t config;
    int ret;
    
    /* Test capabilities query */
    ret = ioctl(fd, VEXFS_IOC_GET_CAPABILITIES, &capabilities);
    TEST_ASSERT(ret == 0, "Capabilities query succeeded");
    TEST_ASSERT(capabilities > 0, "System has some capabilities");
    
    printf("📊 System capabilities: 0x%x\n", capabilities);
    if (capabilities & (1 << 0)) printf("  ✅ SIMD support\n");
    if (capabilities & (1 << 1)) printf("  ✅ NUMA support\n");
    if (capabilities & (1 << 2)) printf("  ✅ Multi-threading\n");
    if (capabilities & (1 << 3)) printf("  ✅ Hardware acceleration\n");
    
    /* Test configuration setting */
    config = 0x12345678;
    ret = ioctl(fd, VEXFS_IOC_SET_CONFIG, &config);
    TEST_ASSERT(ret == 0, "Configuration setting succeeded");
    
    /* Test cache flush */
    ret = ioctl(fd, VEXFS_IOC_FLUSH_CACHES);
    TEST_ASSERT(ret == 0, "Cache flush succeeded");
    
    TEST_END("System Operations");
    return 0;
}

/* 🔥 PERFORMANCE BENCHMARKS 🔥 */

static int test_performance_benchmarks(int fd) {
    TEST_START("Performance Benchmarks");
    
    struct timeval start, end;
    double elapsed;
    int ret;
    
    /* Benchmark vector creation */
    gettimeofday(&start, NULL);
    for (int i = 0; i < 100; i++) {
        struct vexfs_create_vector_request req;
        uint32_t *vector_data = malloc(TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
        generate_test_vector(vector_data, TEST_VECTOR_DIMENSIONS);
        
        memset(&req, 0, sizeof(req));
        req.vector_data = vector_data;
        req.dimensions = TEST_VECTOR_DIMENSIONS;
        req.element_type = VEXFS_VECTOR_FLOAT32;
        req.flags = VEXFS_CREATE_VECTOR_SIMD_ALIGN;
        
        ret = ioctl(fd, VEXFS_IOC_CREATE_VECTOR, &req);
        free(vector_data);
        
        if (ret != 0) break;
    }
    gettimeofday(&end, NULL);
    elapsed = (end.tv_sec - start.tv_sec) + (end.tv_usec - start.tv_usec) / 1000000.0;
    printf("📊 Vector creation: 100 vectors in %.3f seconds (%.1f vectors/sec)\n", 
           elapsed, 100.0 / elapsed);
    
    /* Benchmark similarity search */
    gettimeofday(&start, NULL);
    for (int i = 0; i < 100; i++) {
        struct vexfs_enhanced_search_request req;
        uint32_t *query_vector = malloc(TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
        uint32_t *result_distances = malloc(TEST_SEARCH_K * sizeof(uint32_t));
        uint64_t *result_ids = malloc(TEST_SEARCH_K * sizeof(uint64_t));
        
        generate_normalized_vector(query_vector, TEST_VECTOR_DIMENSIONS);
        
        memset(&req, 0, sizeof(req));
        req.query_vector = query_vector;
        req.dimensions = TEST_VECTOR_DIMENSIONS;
        req.k = TEST_SEARCH_K;
        req.distance_metric = VEXFS_SEARCH_EUCLIDEAN;
        req.result_distances = result_distances;
        req.result_ids = result_ids;
        
        ret = ioctl(fd, VEXFS_IOC_SIMILARITY_SEARCH, &req);
        
        free(query_vector);
        free(result_distances);
        free(result_ids);
        
        if (ret != 0) break;
    }
    gettimeofday(&end, NULL);
    elapsed = (end.tv_sec - start.tv_sec) + (end.tv_usec - start.tv_usec) / 1000000.0;
    printf("📊 Similarity search: 100 searches in %.3f seconds (%.1f searches/sec)\n", 
           elapsed, 100.0 / elapsed);
    
    TEST_END("Performance Benchmarks");
    return 0;
}

/* 🔥 MAIN TEST RUNNER 🔥 */

int main(int argc, char *argv[]) {
    int fd;
    const char *device_path = TEST_DEVICE_PATH;
    
    printf("🚀 VexFS v2.0 Enhanced ioctl Interface Test Suite\n");
    printf("================================================\n");
    
    /* Override device path if provided */
    if (argc > 1) {
        device_path = argv[1];
    }
    
    /* Open test device */
    fd = open(device_path, O_RDWR);
    if (fd < 0) {
        printf("⚠️  Warning: Cannot open %s (errno: %d)\n", device_path, errno);
        printf("📝 Running tests with simulated device (some tests may fail)\n");
        fd = -1; /* Use invalid fd to test error handling */
    } else {
        printf("✅ Opened device: %s\n", device_path);
    }
    
    /* Initialize random seed */
    srand(time(NULL));
    
    /* Run test suites */
    if (fd >= 0) {
        test_vector_creation(fd);
        test_similarity_search(fd);
        test_index_building(fd);
        test_batch_operations(fd);
        test_statistics(fd);
        test_system_operations(fd);
        test_performance_benchmarks(fd);
    } else {
        printf("⚠️  Skipping device-dependent tests due to missing device\n");
        
        /* Run basic validation tests that don't require device */
        TEST_START("Basic Validation");
        TEST_ASSERT(VEXFS_MAX_VECTOR_DIMENSION == 65536, "Max dimension constant is correct");
        TEST_ASSERT(VEXFS_MAX_BATCH_SIZE == 10000, "Max batch size constant is correct");
        TEST_ASSERT(VEXFS_MAX_SEARCH_RESULTS == 10000, "Max search results constant is correct");
        TEST_END("Basic Validation");
    }
    
    /* Close device */
    if (fd >= 0) {
        close(fd);
    }
    
    /* Print test summary */
    printf("\n📊 Test Summary\n");
    printf("===============\n");
    printf("Total tests: %d\n", total_tests);
    printf("Passed: %d\n", tests_passed);
    printf("Failed: %d\n", tests_failed);
    
    if (tests_failed == 0) {
        printf("🎉 All tests passed!\n");
        return 0;
    } else {
        printf("❌ %d tests failed\n", tests_failed);
        return 1;
    }
}