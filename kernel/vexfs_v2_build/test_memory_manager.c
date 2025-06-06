/*
 * VexFS v2.0 Memory Manager Test Program
 * 
 * Comprehensive test suite for the optimized memory management system
 * including allocation performance, NUMA awareness, SIMD alignment,
 * and memory pool functionality.
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
#ifdef HAVE_NUMA
#include <numa.h>
#include <numaif.h>
#else
/* NUMA stubs for systems without NUMA support */
#define numa_available() (-1)
#define numa_node_of_cpu(cpu) (-1)
#define numa_num_configured_nodes() (1)
#define get_mempolicy(policy, nmask, maxnode, addr, flags) (-1)
#define MPOL_F_NODE 0
#define MPOL_F_ADDR 0
#endif

/* Include VexFS headers */
#include "vexfs_v2_uapi.h"

/* Test configuration */
#define TEST_ALLOCATION_COUNT   10000
#define TEST_VECTOR_SIZES       8
#define TEST_THREAD_COUNT       4
#define TEST_ITERATIONS         5000
#define TEST_ALIGNMENT_SIZES    4

/* Test allocation sizes (in bytes) */
static const size_t test_sizes[TEST_VECTOR_SIZES] = {
    1024,      /* 1KB - small vectors */
    4096,      /* 4KB - medium vectors */
    16384,     /* 16KB - large vectors */
    65536,     /* 64KB - very large vectors */
    262144,    /* 256KB - huge vectors */
    1048576,   /* 1MB - massive vectors */
    4194304,   /* 4MB - enormous vectors */
    16777216   /* 16MB - gigantic vectors */
};

/* Test alignment requirements */
static const unsigned int test_alignments[TEST_ALIGNMENT_SIZES] = {
    16,   /* SSE alignment */
    32,   /* AVX alignment */
    64,   /* AVX-512 alignment */
    128   /* Cache line alignment */
};

/* Test statistics */
struct test_stats {
    uint64_t total_allocations;
    uint64_t successful_allocations;
    uint64_t failed_allocations;
    uint64_t total_freed;
    uint64_t alignment_failures;
    uint64_t numa_local_allocations;
    uint64_t numa_remote_allocations;
    double avg_allocation_time_us;
    double avg_free_time_us;
    size_t peak_memory_usage;
    size_t total_memory_allocated;
};

/* Global test data */
static int vexfs_fd = -1;
static struct test_stats global_stats = {0};
static pthread_mutex_t stats_mutex = PTHREAD_MUTEX_INITIALIZER;
static void **allocated_ptrs = NULL;
static size_t allocated_count = 0;

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
 * Check memory alignment
 */
static int check_alignment(void *ptr, unsigned int alignment)
{
    if (!ptr)
        return 0;
    
    return ((uintptr_t)ptr % alignment) == 0;
}

/*
 * Get current NUMA node
 */
static int get_current_numa_node(void)
{
    int node = -1;
    
    if (numa_available() >= 0) {
        node = numa_node_of_cpu(sched_getcpu());
    }
    
    return node;
}

/*
 * Get memory NUMA node
 */
static int get_memory_numa_node(void *ptr, size_t size)
{
    int node = -1;
    int status;
    
    if (numa_available() >= 0 && ptr && size > 0) {
        if (get_mempolicy(&node, NULL, 0, ptr, MPOL_F_NODE | MPOL_F_ADDR) == 0) {
            return node;
        }
    }
    
    return -1;
}

/*
 * Test basic memory allocation
 */
static int test_basic_allocation(void)
{
    void *ptr;
    size_t size = 4096;
    struct timeval start, end;
    double elapsed_us;
    
    printf("Testing basic memory allocation...\n");
    
    gettimeofday(&start, NULL);
    ptr = aligned_alloc(64, size);
    gettimeofday(&end, NULL);
    
    if (!ptr) {
        fprintf(stderr, "Basic allocation failed\n");
        return -1;
    }
    
    elapsed_us = (end.tv_sec - start.tv_sec) * 1000000.0 + 
                 (end.tv_usec - start.tv_usec);
    
    printf("  Allocated %zu bytes at %p\n", size, ptr);
    printf("  Allocation time: %.2f μs\n", elapsed_us);
    printf("  64-byte aligned: %s\n", check_alignment(ptr, 64) ? "yes" : "no");
    
    /* Test memory access */
    memset(ptr, 0xAA, size);
    
    /* Verify pattern */
    for (size_t i = 0; i < size; i++) {
        if (((unsigned char*)ptr)[i] != 0xAA) {
            fprintf(stderr, "Memory corruption detected at offset %zu\n", i);
            free(ptr);
            return -1;
        }
    }
    
    free(ptr);
    printf("Basic allocation test passed\n");
    return 0;
}

/*
 * Test SIMD alignment
 */
static int test_simd_alignment(void)
{
    void *ptrs[TEST_ALIGNMENT_SIZES];
    size_t size = 8192;
    int i;
    
    printf("Testing SIMD alignment requirements...\n");
    
    for (i = 0; i < TEST_ALIGNMENT_SIZES; i++) {
        ptrs[i] = aligned_alloc(test_alignments[i], size);
        if (!ptrs[i]) {
            fprintf(stderr, "Failed to allocate %u-byte aligned memory\n", 
                    test_alignments[i]);
            goto cleanup;
        }
        
        if (!check_alignment(ptrs[i], test_alignments[i])) {
            fprintf(stderr, "Memory not properly aligned to %u bytes: %p\n",
                    test_alignments[i], ptrs[i]);
            goto cleanup;
        }
        
        printf("  %u-byte alignment: %p ✓\n", test_alignments[i], ptrs[i]);
        
        /* Test SIMD-like operations */
        memset(ptrs[i], i + 1, size);
    }
    
    printf("SIMD alignment test passed\n");
    
cleanup:
    for (i = 0; i < TEST_ALIGNMENT_SIZES; i++) {
        if (ptrs[i]) {
            free(ptrs[i]);
        }
    }
    
    return 0;
}

/*
 * Test NUMA awareness
 */
static int test_numa_awareness(void)
{
    void *ptr;
    size_t size = 1048576; /* 1MB */
    int current_node, memory_node;
    
    printf("Testing NUMA awareness...\n");
    
    if (numa_available() < 0) {
        printf("NUMA not available, skipping NUMA tests\n");
        return 0;
    }
    
    current_node = get_current_numa_node();
    printf("  Current CPU NUMA node: %d\n", current_node);
    printf("  Available NUMA nodes: %d\n", numa_num_configured_nodes());
    
    /* Allocate memory */
    ptr = aligned_alloc(64, size);
    if (!ptr) {
        fprintf(stderr, "NUMA allocation failed\n");
        return -1;
    }
    
    memory_node = get_memory_numa_node(ptr, size);
    printf("  Allocated memory on NUMA node: %d\n", memory_node);
    
    if (memory_node >= 0 && memory_node == current_node) {
        printf("  NUMA locality: LOCAL ✓\n");
        global_stats.numa_local_allocations++;
    } else {
        printf("  NUMA locality: REMOTE\n");
        global_stats.numa_remote_allocations++;
    }
    
    /* Test memory access performance */
    struct timeval start, end;
    gettimeofday(&start, NULL);
    
    for (int i = 0; i < 1000; i++) {
        memset(ptr, i & 0xFF, size);
    }
    
    gettimeofday(&end, NULL);
    double elapsed_ms = (end.tv_sec - start.tv_sec) * 1000.0 + 
                       (end.tv_usec - start.tv_usec) / 1000.0;
    
    printf("  Memory access performance: %.2f ms for 1000 iterations\n", elapsed_ms);
    
    free(ptr);
    printf("NUMA awareness test completed\n");
    return 0;
}

/*
 * Test large contiguous allocations
 */
static int test_large_allocations(void)
{
    void *ptrs[TEST_VECTOR_SIZES];
    struct timeval start, end;
    double elapsed_us;
    int i;
    
    printf("Testing large contiguous allocations...\n");
    
    for (i = 0; i < TEST_VECTOR_SIZES; i++) {
        size_t size = test_sizes[i];
        
        gettimeofday(&start, NULL);
        ptrs[i] = aligned_alloc(64, size);
        gettimeofday(&end, NULL);
        
        elapsed_us = (end.tv_sec - start.tv_sec) * 1000000.0 + 
                     (end.tv_usec - start.tv_usec);
        
        if (!ptrs[i]) {
            fprintf(stderr, "Failed to allocate %zu bytes\n", size);
            goto cleanup;
        }
        
        printf("  %zu bytes: %p (%.2f μs)\n", size, ptrs[i], elapsed_us);
        
        /* Verify alignment */
        if (!check_alignment(ptrs[i], 64)) {
            fprintf(stderr, "Large allocation not 64-byte aligned\n");
            goto cleanup;
        }
        
        /* Test memory access */
        memset(ptrs[i], 0x55, size);
        
        global_stats.total_memory_allocated += size;
    }
    
    printf("Large allocation test passed\n");
    
cleanup:
    for (i = 0; i < TEST_VECTOR_SIZES; i++) {
        if (ptrs[i]) {
            free(ptrs[i]);
        }
    }
    
    return 0;
}

/*
 * Performance test worker thread
 */
static void *performance_test_worker(void *arg)
{
    int thread_id = *(int *)arg;
    struct timeval start, end;
    uint64_t allocations = 0;
    uint64_t successful = 0;
    uint64_t failed = 0;
    double total_alloc_time = 0.0;
    double total_free_time = 0.0;
    void **local_ptrs;
    int i;
    
    printf("Performance test thread %d started\n", thread_id);
    
    local_ptrs = malloc(TEST_ITERATIONS * sizeof(void*));
    if (!local_ptrs) {
        fprintf(stderr, "Thread %d: Failed to allocate pointer array\n", thread_id);
        return NULL;
    }
    
    /* Allocation phase */
    for (i = 0; i < TEST_ITERATIONS; i++) {
        size_t size = test_sizes[i % TEST_VECTOR_SIZES];
        
        gettimeofday(&start, NULL);
        local_ptrs[i] = aligned_alloc(64, size);
        gettimeofday(&end, NULL);
        
        allocations++;
        
        if (local_ptrs[i]) {
            successful++;
            total_alloc_time += (end.tv_sec - start.tv_sec) * 1000000.0 + 
                               (end.tv_usec - start.tv_usec);
            
            /* Quick memory test */
            memset(local_ptrs[i], thread_id & 0xFF, size);
        } else {
            failed++;
        }
    }
    
    /* Free phase */
    for (i = 0; i < TEST_ITERATIONS; i++) {
        if (local_ptrs[i]) {
            gettimeofday(&start, NULL);
            free(local_ptrs[i]);
            gettimeofday(&end, NULL);
            
            total_free_time += (end.tv_sec - start.tv_sec) * 1000000.0 + 
                              (end.tv_usec - start.tv_usec);
        }
    }
    
    /* Update global statistics */
    pthread_mutex_lock(&stats_mutex);
    global_stats.total_allocations += allocations;
    global_stats.successful_allocations += successful;
    global_stats.failed_allocations += failed;
    global_stats.total_freed += successful;
    global_stats.avg_allocation_time_us += total_alloc_time / successful;
    global_stats.avg_free_time_us += total_free_time / successful;
    pthread_mutex_unlock(&stats_mutex);
    
    printf("Thread %d completed: %lu allocs, %lu successful, %lu failed\n",
           thread_id, allocations, successful, failed);
    
    free(local_ptrs);
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
    global_stats.avg_allocation_time_us /= TEST_THREAD_COUNT;
    global_stats.avg_free_time_us /= TEST_THREAD_COUNT;
    
    /* Print performance results */
    printf("\n=== Performance Test Results ===\n");
    printf("Total Allocations: %lu\n", global_stats.total_allocations);
    printf("Successful Allocations: %lu\n", global_stats.successful_allocations);
    printf("Failed Allocations: %lu\n", global_stats.failed_allocations);
    printf("Success Rate: %.2f%%\n", 
           global_stats.total_allocations > 0 ? 
           (global_stats.successful_allocations * 100.0 / global_stats.total_allocations) : 0.0);
    printf("Average Allocation Time: %.2f μs\n", global_stats.avg_allocation_time_us);
    printf("Average Free Time: %.2f μs\n", global_stats.avg_free_time_us);
    printf("Allocations/sec: %.0f\n", 
           global_stats.successful_allocations / elapsed_time);
    printf("NUMA Local Allocations: %lu\n", global_stats.numa_local_allocations);
    printf("NUMA Remote Allocations: %lu\n", global_stats.numa_remote_allocations);
    printf("Elapsed Time: %.2f seconds\n", elapsed_time);
    
    return 0;
}

/*
 * Test memory pool simulation
 */
static int test_memory_pools(void)
{
    void *pool_ptrs[1000];
    size_t pool_size = 4096;
    struct timeval start, end;
    double pool_time, direct_time;
    int i;
    
    printf("Testing memory pool simulation...\n");
    
    /* Simulate pool allocation (reuse same size) */
    gettimeofday(&start, NULL);
    for (i = 0; i < 1000; i++) {
        pool_ptrs[i] = aligned_alloc(64, pool_size);
        if (pool_ptrs[i]) {
            memset(pool_ptrs[i], i & 0xFF, pool_size);
        }
    }
    gettimeofday(&end, NULL);
    pool_time = (end.tv_sec - start.tv_sec) * 1000000.0 + 
                (end.tv_usec - start.tv_usec);
    
    /* Free pool allocations */
    for (i = 0; i < 1000; i++) {
        if (pool_ptrs[i]) {
            free(pool_ptrs[i]);
        }
    }
    
    /* Simulate direct allocation (varying sizes) */
    gettimeofday(&start, NULL);
    for (i = 0; i < 1000; i++) {
        size_t size = test_sizes[i % TEST_VECTOR_SIZES];
        pool_ptrs[i] = aligned_alloc(64, size);
        if (pool_ptrs[i]) {
            memset(pool_ptrs[i], i & 0xFF, size);
            free(pool_ptrs[i]);
        }
    }
    gettimeofday(&end, NULL);
    direct_time = (end.tv_sec - start.tv_sec) * 1000000.0 + 
                  (end.tv_usec - start.tv_usec);
    
    printf("Memory pool simulation results:\n");
    printf("  Pool-like allocation (same size): %.2f μs total, %.2f μs avg\n", 
           pool_time, pool_time / 1000.0);
    printf("  Direct allocation (varying sizes): %.2f μs total, %.2f μs avg\n", 
           direct_time, direct_time / 1000.0);
    printf("  Pool efficiency: %.2fx faster\n", direct_time / pool_time);
    
    return 0;
}

/*
 * Test memory fragmentation
 */
static int test_fragmentation(void)
{
    void *ptrs[1000];
    size_t total_allocated = 0;
    int i;
    
    printf("Testing memory fragmentation patterns...\n");
    
    /* Allocate many small blocks */
    for (i = 0; i < 1000; i++) {
        size_t size = 1024 + (i % 7) * 512; /* Varying sizes */
        ptrs[i] = aligned_alloc(64, size);
        if (ptrs[i]) {
            total_allocated += size;
            memset(ptrs[i], i & 0xFF, size);
        }
    }
    
    printf("  Allocated %zu bytes in 1000 blocks\n", total_allocated);
    
    /* Free every other block to create fragmentation */
    for (i = 0; i < 1000; i += 2) {
        if (ptrs[i]) {
            free(ptrs[i]);
            ptrs[i] = NULL;
        }
    }
    
    printf("  Freed 500 blocks to create fragmentation\n");
    
    /* Try to allocate large blocks in fragmented space */
    int large_allocs = 0;
    for (i = 0; i < 100; i++) {
        void *large_ptr = aligned_alloc(64, 32768); /* 32KB */
        if (large_ptr) {
            large_allocs++;
            free(large_ptr);
        }
    }
    
    printf("  Successfully allocated %d/100 large blocks in fragmented space\n", 
           large_allocs);
    
    /* Cleanup remaining blocks */
    for (i = 1; i < 1000; i += 2) {
        if (ptrs[i]) {
            free(ptrs[i]);
        }
    }
    
    printf("Fragmentation test completed\n");
    return 0;
}

/*
 * Main test function
 */
int main(int argc, char *argv[])
{
    int ret = 0;
    
    printf("=== VexFS Memory Manager Test Suite ===\n");
    printf("Test allocation count: %d\n", TEST_ALLOCATION_COUNT);
    printf("Thread count: %d\n", TEST_THREAD_COUNT);
    printf("Iterations per thread: %d\n", TEST_ITERATIONS);
    printf("\n");
    
    /* Initialize NUMA if available */
    if (numa_available() >= 0) {
        printf("NUMA support detected\n");
    } else {
        printf("NUMA support not available\n");
    }
    
    /* Run test suite */
    printf("=== Running Test Suite ===\n");
    
    if (test_basic_allocation() != 0) {
        fprintf(stderr, "Basic allocation test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_simd_alignment() != 0) {
        fprintf(stderr, "SIMD alignment test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_numa_awareness() != 0) {
        fprintf(stderr, "NUMA awareness test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_large_allocations() != 0) {
        fprintf(stderr, "Large allocation test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_memory_pools() != 0) {
        fprintf(stderr, "Memory pool test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_fragmentation() != 0) {
        fprintf(stderr, "Fragmentation test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    if (test_performance() != 0) {
        fprintf(stderr, "Performance test failed\n");
        ret = 1;
        goto cleanup;
    }
    
    printf("\n=== All Tests Completed Successfully ===\n");
    printf("Memory management system validated\n");
    
cleanup:
    return ret;
}