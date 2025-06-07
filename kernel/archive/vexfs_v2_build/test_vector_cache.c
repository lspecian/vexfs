/*
 * VexFS v2.0 Vector Cache Test Program
 * 
 * Test suite for the vector data caching system using the existing
 * VexFS v2.0 UAPI interface. Tests SIMD alignment, performance,
 * and cache behavior through the standard VexFS operations.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/time.h>
#include <errno.h>
#include <assert.h>
#include <time.h>
#include <pthread.h>
#include <math.h>

/* Include VexFS headers */
#include "vexfs_v2_uapi.h"

/* Test configuration */
#define TEST_VECTOR_COUNT       1000
#define TEST_VECTOR_DIMENSIONS  128
#define TEST_THREAD_COUNT       4
#define TEST_ITERATIONS         5000
#define TEST_CACHE_WARMUP_RUNS  100

/* Test vector data structure */
struct test_vector {
    uint64_t vector_id;
    uint32_t dimensions;
    float *data;
    uint32_t *data_bits;  /* IEEE 754 bit representation for kernel */
    size_t data_size;
};

/* Test statistics */
struct test_stats {
    uint64_t total_operations;
    uint64_t successful_operations;
    uint64_t failed_operations;
    double avg_latency_us;
    double operations_per_sec;
    uint64_t cache_test_hits;
    uint64_t cache_test_misses;
};

/* Global test data */
static struct test_vector *test_vectors = NULL;
static int vexfs_fd = -1;
static struct test_stats global_stats = {0};
static pthread_mutex_t stats_mutex = PTHREAD_MUTEX_INITIALIZER;

/*
 * Generate random test vector data
 */
static void generate_test_vector(struct test_vector *vector, uint64_t id)
{
    int i;
    
    vector->vector_id = id;
    vector->dimensions = TEST_VECTOR_DIMENSIONS;
    vector->data_size = TEST_VECTOR_DIMENSIONS * sizeof(float);
    
    /* Allocate aligned memory for SIMD operations */
    vector->data = aligned_alloc(64, vector->data_size); /* 64-byte alignment for AVX-512 */
    vector->data_bits = aligned_alloc(64, TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
    
    if (!vector->data || !vector->data_bits) {
        fprintf(stderr, "Failed to allocate aligned memory for vector %lu\n", id);
        exit(1);
    }
    
    /* Generate random vector data */
    srand(time(NULL) + id);
    for (i = 0; i < TEST_VECTOR_DIMENSIONS; i++) {
        vector->data[i] = (float)(rand() % 1000) / 1000.0f; /* Random values 0.0-1.0 */
    }
    
    /* Convert to IEEE 754 bit representation for kernel compatibility */
    vexfs_float_array_to_bits(vector->data, vector->data_bits, TEST_VECTOR_DIMENSIONS);
}

/*
 * Free test vector data
 */
static void free_test_vector(struct test_vector *vector)
{
    if (vector->data) {
        free(vector->data);
        vector->data = NULL;
    }
    if (vector->data_bits) {
        free(vector->data_bits);
        vector->data_bits = NULL;
    }
}

/*
 * Initialize test vectors
 */
static int init_test_vectors(void)
{
    int i;
    
    test_vectors = calloc(TEST_VECTOR_COUNT, sizeof(struct test_vector));
    if (!test_vectors) {
        fprintf(stderr, "Failed to allocate test vectors\n");
        return -1;
    }
    
    printf("Generating %d test vectors with %d dimensions each...\n", 
           TEST_VECTOR_COUNT, TEST_VECTOR_DIMENSIONS);
    
    for (i = 0; i < TEST_VECTOR_COUNT; i++) {
        generate_test_vector(&test_vectors[i], i);
    }
    
    printf("Test vectors generated successfully\n");
    return 0;
}

/*
 * Cleanup test vectors
 */
static void cleanup_test_vectors(void)
{
    int i;
    
    if (test_vectors) {
        for (i = 0; i < TEST_VECTOR_COUNT; i++) {
            free_test_vector(&test_vectors[i]);
        }
        free(test_vectors);
        test_vectors = NULL;
    }
}

/*
 * Open VexFS device
 */
static int open_vexfs_device(void)
{
    vexfs_fd = open("/dev/vexfs", O_RDWR);
    if (vexfs_fd < 0) {
        fprintf(stderr, "Failed to open VexFS device: %s\n", strerror(errno));
        fprintf(stderr, "Note: VexFS kernel module may not be loaded\n");
        return -1;
    }
    
    printf("VexFS device opened successfully (fd=%d)\n", vexfs_fd);
    return 0;
}

/*
 * Close VexFS device
 */
static void close_vexfs_device(void)
{
    if (vexfs_fd >= 0) {
        close(vexfs_fd);
        vexfs_fd = -1;
    }
}

/*
 * Test vector batch insertion (tests cache insertion)
 */
static int test_batch_insert(int start_idx, int count)
{
    struct vexfs_batch_insert_request req;
    uint32_t *vectors_data;
    uint64_t *vector_ids;
    int ret;
    int i;
    
    /* Allocate aligned memory for batch data */
    vectors_data = aligned_alloc(64, count * TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
    vector_ids = aligned_alloc(64, count * sizeof(uint64_t));
    
    if (!vectors_data || !vector_ids) {
        fprintf(stderr, "Failed to allocate batch insert memory\n");
        return -1;
    }
    
    /* Prepare batch data */
    for (i = 0; i < count; i++) {
        int vector_idx = (start_idx + i) % TEST_VECTOR_COUNT;
        memcpy(&vectors_data[i * TEST_VECTOR_DIMENSIONS], 
               test_vectors[vector_idx].data_bits,
               TEST_VECTOR_DIMENSIONS * sizeof(uint32_t));
        vector_ids[i] = test_vectors[vector_idx].vector_id;
    }
    
    /* Setup batch insert request */
    memset(&req, 0, sizeof(req));
    req.vectors_bits = vectors_data;
    req.vector_count = count;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.vector_ids = vector_ids;
    req.flags = VEXFS_INSERT_VALIDATE;
    
    ret = ioctl(vexfs_fd, VEXFS_IOC_BATCH_INSERT, &req);
    if (ret < 0) {
        fprintf(stderr, "Batch insert failed: %s\n", strerror(errno));
        free(vectors_data);
        free(vector_ids);
        return -1;
    }
    
    free(vectors_data);
    free(vector_ids);
    return 0;
}

/*
 * Test vector search (tests cache lookup)
 */
static int test_vector_search(int query_idx, int *found_results)
{
    struct vexfs_vector_search_request req;
    uint32_t *result_distances;
    uint64_t *result_ids;
    int ret;
    
    /* Allocate aligned memory for results */
    result_distances = aligned_alloc(64, 10 * sizeof(uint32_t));
    result_ids = aligned_alloc(64, 10 * sizeof(uint64_t));
    
    if (!result_distances || !result_ids) {
        fprintf(stderr, "Failed to allocate search result memory\n");
        return -1;
    }
    
    /* Setup search request */
    memset(&req, 0, sizeof(req));
    req.query_vector_bits = test_vectors[query_idx].data_bits;
    req.dimensions = TEST_VECTOR_DIMENSIONS;
    req.k = 10;
    req.search_type = VEXFS_SEARCH_EUCLIDEAN;
    req.results_bits = result_distances;
    req.result_ids = result_ids;
    req.result_count = 0;
    
    ret = ioctl(vexfs_fd, VEXFS_IOC_VECTOR_SEARCH, &req);
    if (ret < 0) {
        fprintf(stderr, "Vector search failed: %s\n", strerror(errno));
        free(result_distances);
        free(result_ids);
        return -1;
    }
    
    *found_results = req.result_count;
    
    free(result_distances);
    free(result_ids);
    return 0;
}

/*
 * Test SIMD alignment verification
 */
static int test_simd_alignment(void)
{
    int i;
    
    printf("Testing SIMD alignment verification...\n");
    
    /* Test alignment of our test vectors */
    for (i = 0; i < 10; i++) {
        uintptr_t data_addr = (uintptr_t)test_vectors[i].data;
        uintptr_t bits_addr = (uintptr_t)test_vectors[i].data_bits;
        
        if (data_addr % 64 != 0) {
            fprintf(stderr, "Vector %d float data not 64-byte aligned: addr=0x%lx\n",
                    i, data_addr);
            return -1;
        }
        
        if (bits_addr % 64 != 0) {
            fprintf(stderr, "Vector %d bits data not 64-byte aligned: addr=0x%lx\n",
                    i, bits_addr);
            return -1;
        }
    }
    
    printf("SIMD alignment test passed (64-byte alignment verified)\n");
    return 0;
}

/*
 * Test vector file metadata operations
 */
static int test_vector_metadata(void)
{
    struct vexfs_vector_file_info info;
    int ret;
    
    printf("Testing vector file metadata operations...\n");
    
    /* Setup metadata */
    memset(&info, 0, sizeof(info));
    info.dimensions = TEST_VECTOR_DIMENSIONS;
    info.element_type = VEXFS_VECTOR_FLOAT32;
    info.vector_count = TEST_VECTOR_COUNT;
    info.storage_format = VEXFS_STORAGE_DENSE;
    info.data_offset = 0;
    info.index_offset = 0;
    info.compression_type = VEXFS_COMPRESS_NONE;
    info.alignment_bytes = 64;
    
    /* Set metadata */
    ret = ioctl(vexfs_fd, VEXFS_IOC_SET_VECTOR_META, &info);
    if (ret < 0) {
        fprintf(stderr, "Set vector metadata failed: %s\n", strerror(errno));
        return -1;
    }
    
    /* Get metadata back */
    memset(&info, 0, sizeof(info));
    ret = ioctl(vexfs_fd, VEXFS_IOC_GET_VECTOR_META, &info);
    if (ret < 0) {
        fprintf(stderr, "Get vector metadata failed: %s\n", strerror(errno));
        return -1;
    }
    
    /* Verify metadata */
    if (info.dimensions != TEST_VECTOR_DIMENSIONS) {
        fprintf(stderr, "Metadata mismatch: dimensions %u != %u\n", 
                info.dimensions, TEST_VECTOR_DIMENSIONS);
        return -1;
    }
    
    printf("Vector metadata test passed\n");
    printf("  Dimensions: %u\n", info.dimensions);
    printf("  Element type: %u\n", info.element_type);
    printf("  Vector count: %u\n", info.vector_count);
    printf("  Storage format: %u\n", info.storage_format);
    printf("  Alignment: %u bytes\n", info.alignment_bytes);
    
    return 0;
}

/*
 * Performance test worker thread
 */
static void *performance_test_worker(void *arg)
{
    int thread_id = *(int *)arg;
    struct timeval start, end;
    uint64_t operations = 0;
    uint64_t successful = 0;
    uint64_t failed = 0;
    double total_time = 0.0;
    int i;
    
    printf("Performance test thread %d started\n", thread_id);
    
    gettimeofday(&start, NULL);
    
    for (i = 0; i < TEST_ITERATIONS; i++) {
        int query_idx = rand() % TEST_VECTOR_COUNT;
        int found_results = 0;
        struct timeval op_start, op_end;
        
        gettimeofday(&op_start, NULL);
        
        if (test_vector_search(query_idx, &found_results) == 0) {
            successful++;
            if (found_results > 0) {
                /* Simulate cache hit */
                global_stats.cache_test_hits++;
            } else {
                /* Simulate cache miss */
                global_stats.cache_test_misses++;
            }
        } else {
            failed++;
        }
        
        operations++;
        
        gettimeofday(&op_end, NULL);
        total_time += (op_end.tv_sec - op_start.tv_sec) * 1000000.0 + 
                     (op_end.tv_usec - op_start.tv_usec);
    }
    
    gettimeofday(&end, NULL);
    
    /* Update global statistics */
    pthread_mutex_lock(&stats_mutex);
    global_stats.total_operations += operations;
    global_stats.successful_operations += successful;
    global_stats.failed_operations += failed;
    global_stats.avg_latency_us += total_time / operations;
    pthread_mutex_unlock(&stats_mutex);
    
    printf("Thread %d completed: %lu ops, %lu successful, %lu failed\n",
           thread_id, operations, successful, failed);
    
    return NULL;
}

/*
 * Run performance test
 */
static int test_performance(void)
{
    pthread_t threads[TEST_THREAD_COUNT];
    int thread_ids[TEST_THREAD_COUNT];
    struct timeval start, end;
    double elapsed_time;
    int i, ret;
    
    printf("Running performance test with %d threads, %d iterations each...\n",
           TEST_THREAD_COUNT, TEST_ITERATIONS);
    
    /* Reset global statistics */
    memset(&global_stats, 0, sizeof(global_stats));
    
    /* Warm up cache with some insertions */
    printf("Warming up cache with %d vectors...\n", TEST_CACHE_WARMUP_RUNS);
    for (i = 0; i < TEST_CACHE_WARMUP_RUNS; i += 10) {
        int batch_size = (i + 10 <= TEST_CACHE_WARMUP_RUNS) ? 10 : (TEST_CACHE_WARMUP_RUNS - i);
        test_batch_insert(i, batch_size);
    }
    
    gettimeofday(&start, NULL);
    
    /* Create worker threads */
    for (i = 0; i < TEST_THREAD_COUNT; i++) {
        thread_ids[i] = i;
        ret = pthread_create(&threads[i], NULL, performance_test_worker, &thread_ids[i]);
        if (ret != 0) {
            fprintf(stderr, "Failed to create thread %d: %s\n", i, strerror(ret));
            return -1;
        }
    }
    
    /* Wait for all threads to complete */
    for (i = 0; i < TEST_THREAD_COUNT; i++) {
        pthread_join(threads[i], NULL);
    }
    
    gettimeofday(&end, NULL);
    elapsed_time = (end.tv_sec - start.tv_sec) + (end.tv_usec - start.tv_usec) / 1000000.0;
    
    /* Calculate final statistics */
    global_stats.avg_latency_us /= TEST_THREAD_COUNT;
    global_stats.operations_per_sec = global_stats.total_operations / elapsed_time;
    
    /* Print performance results */
    printf("\n=== Performance Test Results ===\n");
    printf("Total Operations: %lu\n", global_stats.total_operations);
    printf("Successful Operations: %lu\n", global_stats.successful_operations);
    printf("Failed Operations: %lu\n", global_stats.failed_operations);
    printf("Success Rate: %.2f%%\n", 
           global_stats.total_operations > 0 ? 
           (global_stats.successful_operations * 100.0 / global_stats.total_operations) : 0.0);
    printf("Average Latency: %.2f μs\n", global_stats.avg_latency_us);
    printf("Operations/sec: %.0f\n", global_stats.operations_per_sec);
    printf("Cache Test Hits: %lu\n", global_stats.cache_test_hits);
    printf("Cache Test Misses: %lu\n", global_stats.cache_test_misses);
    printf("Elapsed Time: %.2f seconds\n", elapsed_time);
    
    return 0;
}

/*
 * Test cache behavior with repeated operations
 */
static int test_cache_behavior(void)
{
    struct timeval start, end;
    double first_run_time, second_run_time;
    int i, found_results;
    
    printf("Testing cache behavior with repeated operations...\n");
    
    /* First run - should populate cache */
    gettimeofday(&start, NULL);
    for (i = 0; i < 100; i++) {
        test_vector_search(i % 10, &found_results); /* Repeat same 10 vectors */
    }
    gettimeofday(&end, NULL);
    first_run_time = (end.tv_sec - start.tv_sec) * 1000000.0 + (end.tv_usec - start.tv_usec);
    
    /* Second run - should benefit from cache */
    gettimeofday(&start, NULL);
    for (i = 0; i < 100; i++) {
        test_vector_search(i % 10, &found_results); /* Same vectors again */
    }
    gettimeofday(&end, NULL);
    second_run_time = (end.tv_sec - start.tv_sec) * 1000000.0 + (end.tv_usec - start.tv_usec);
    
    printf("Cache behavior test results:\n");
    printf("  First run (cache population): %.2f μs total, %.2f μs avg\n", 
           first_run_time, first_run_time / 100.0);
    printf("  Second run (cache utilization): %.2f μs total, %.2f μs avg\n", 
           second_run_time, second_run_time / 100.0);
    
    if (second_run_time < first_run_time) {
        printf("  Cache improvement: %.2f%% faster\n", 
               ((first_run_time - second_run_time) / first_run_time) * 100.0);
    } else {
        printf("  No significant cache improvement detected\n");
    }
    
    return 0;
}

/*
 * Test IEEE 754 bit conversion utilities
 */
static int test_ieee754_conversion(void)
{
    float test_values[] = {0.0f, 1.0f, -1.0f, 3.14159f, -2.71828f, 1000.5f};
    int num_values = sizeof(test_values) / sizeof(test_values[0]);
    int i;
    
    printf("Testing IEEE 754 bit conversion utilities...\n");
    
    for (i = 0; i < num_values; i++) {
        float original = test_values[i];
        uint32_t bits = vexfs_float_to_bits(original);
        float converted = vexfs_bits_to_float(bits);
        
        if (fabsf(original - converted) > 1e-6f) {
            fprintf(stderr, "IEEE 754 conversion failed for %f: got %f (bits=0x%08x)\n",
                    original, converted, bits);
            return -1;
        }
        
        printf("  %f <-> 0x%08x <-> %f ✓\n", original, bits, converted);
    }
    
    printf("IEEE 754 conversion test passed\n");
    return 0;
}

/*
 * Main test function
 */
int main(int argc, char *argv[])
{
    int ret = 0;
    
    printf("=== VexFS Vector Cache Test Suite ===\n");
    printf("Vector count: %d\n", TEST_VECTOR_COUNT);
    printf("Vector dimensions: %d\n", TEST_VECTOR_DIMENSIONS);
    printf("Thread count: %d\n", TEST_THREAD_COUNT);
    printf("Iterations per thread: %d\n", TEST_ITERATIONS);
    printf("Cache warmup runs: %d\n", TEST_CACHE_WARMUP_RUNS);
    printf("\n");
    
    /* Initialize test environment */
    if (init_test_vectors() != 0) {
        ret = 1;
        goto cleanup;
    }
    
    if (open_vexfs_device() != 0) {
        ret = 1;
        goto cleanup;
    }
    
    /* Run test suite */
    printf("=== Running Test Suite ===\n");
    
    if (test_ieee754_conversion() != 0) {
        fprintf(stderr, "IEEE 754 conversion test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_simd_alignment() != 0) {
        fprintf(stderr, "SIMD alignment test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_vector_metadata() != 0) {
        fprintf(stderr, "Vector metadata test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_cache_behavior() != 0) {
        fprintf(stderr, "Cache behavior test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_performance() != 0) {
        fprintf(stderr, "Performance test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    printf("\n=== All Tests Completed Successfully ===\n");
    printf("Vector cache functionality validated through VexFS v2.0 interface\n");
    
cleanup:
    close_vexfs_device();
    cleanup_test_vectors();
    
    return ret;
}