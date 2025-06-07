#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <time.h>
#include <stdint.h>

/* VexFS v2.0 ioctl definitions */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_file_info)
#define VEXFS_IOC_VECTOR_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search_request)
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)

/* Structure definitions matching kernel module */
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

struct vexfs_vector_search_request {
    float *query_vector;
    uint32_t dimensions;
    uint32_t k;
    uint32_t search_type;
    float *results;
    uint64_t *result_ids;
    uint32_t result_count;
};

struct vexfs_batch_insert_request {
    float *vectors;
    uint32_t vector_count;
    uint32_t dimensions;
    uint64_t *vector_ids;
    uint32_t flags;
};

/* Performance metrics */
struct benchmark_results {
    double metadata_ops_per_sec;
    double search_ops_per_sec;
    double batch_ops_per_sec;
    double avg_metadata_latency_ms;
    double avg_search_latency_ms;
    double avg_batch_latency_ms;
    int metadata_errors;
    int search_errors;
    int batch_errors;
};

double get_time_ms() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1000000.0;
}

int test_metadata_operations(const char *mount_point, int iterations, double *ops_per_sec, double *avg_latency_ms) {
    char test_file[512];
    int errors = 0;
    double start_time, end_time, total_time = 0;
    
    // Use working filename instead of blocked "metadata_test"
    snprintf(test_file, sizeof(test_file), "%s/meta_operations", mount_point);
    
    printf("Testing Vector Metadata Operations (%d iterations)...\n", iterations);
    
    double test_start = get_time_ms();
    
    for (int i = 0; i < iterations; i++) {
        start_time = get_time_ms();
        
        int fd = open(test_file, O_CREAT | O_RDWR, 0644);
        if (fd < 0) {
            printf("ERROR: Failed to create %s: %s\n", test_file, strerror(errno));
            errors++;
            continue;
        }
        
        struct vexfs_vector_file_info meta = {
            .dimensions = 128,
            .element_type = 0,
            .vector_count = 1000,
            .storage_format = 0,
            .data_offset = 0,
            .index_offset = 128 * 1000 * sizeof(float),
            .compression_type = 0,
            .alignment_bytes = 32
        };
        
        int ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
        close(fd);
        
        if (ret < 0) {
            errors++;
        }
        
        end_time = get_time_ms();
        total_time += (end_time - start_time);
        
        // Clean up
        unlink(test_file);
    }
    
    double test_end = get_time_ms();
    double total_test_time = (test_end - test_start) / 1000.0; // Convert to seconds
    
    *ops_per_sec = iterations / total_test_time;
    *avg_latency_ms = total_time / iterations;
    
    printf("  Completed: %.1f ops/sec, %.2f ms avg latency, %d errors\n", 
           *ops_per_sec, *avg_latency_ms, errors);
    
    return errors;
}

int test_search_operations(const char *mount_point, int iterations, double *ops_per_sec, double *avg_latency_ms) {
    char test_file[512];
    int errors = 0;
    double start_time, end_time, total_time = 0;
    
    // Use working filename instead of blocked "search_test"
    snprintf(test_file, sizeof(test_file), "%s/search_operations", mount_point);
    
    printf("Testing Vector Search Operations (%d iterations)...\n", iterations);
    
    // Set up test file with metadata
    int fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("ERROR: Failed to create search test file\n");
        return iterations; // All operations failed
    }
    
    struct vexfs_vector_file_info meta = {
        .dimensions = 4,
        .element_type = 0,
        .vector_count = 100,
        .storage_format = 0,
        .data_offset = 0,
        .index_offset = 4 * 100 * sizeof(float),
        .compression_type = 0,
        .alignment_bytes = 32
    };
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) < 0) {
        printf("ERROR: Failed to set vector metadata for search test\n");
        close(fd);
        unlink(test_file);
        return iterations;
    }
    
    close(fd);
    
    // Prepare search data
    float query_vector[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    float results[10];
    uint64_t result_ids[10];
    
    double test_start = get_time_ms();
    
    for (int i = 0; i < iterations; i++) {
        start_time = get_time_ms();
        
        fd = open(test_file, O_RDWR);
        if (fd < 0) {
            errors++;
            continue;
        }
        
        struct vexfs_vector_search_request search_req = {
            .query_vector = query_vector,
            .dimensions = 4,
            .k = 5,
            .search_type = 0,
            .results = results,
            .result_ids = result_ids,
            .result_count = 0
        };
        
        int ret = ioctl(fd, VEXFS_IOC_VECTOR_SEARCH, &search_req);
        close(fd);
        
        if (ret < 0) {
            errors++;
        }
        
        end_time = get_time_ms();
        total_time += (end_time - start_time);
    }
    
    double test_end = get_time_ms();
    double total_test_time = (test_end - test_start) / 1000.0;
    
    *ops_per_sec = iterations / total_test_time;
    *avg_latency_ms = total_time / iterations;
    
    printf("  Completed: %.1f ops/sec, %.2f ms avg latency, %d errors\n", 
           *ops_per_sec, *avg_latency_ms, errors);
    
    // Clean up
    unlink(test_file);
    return errors;
}

int test_batch_operations(const char *mount_point, int iterations, double *ops_per_sec, double *avg_latency_ms) {
    char test_file[512];
    int errors = 0;
    double start_time, end_time, total_time = 0;
    
    // Use working filename instead of blocked "batch_test"
    snprintf(test_file, sizeof(test_file), "%s/batch_operations", mount_point);
    
    printf("Testing Batch Insert Operations (%d iterations)...\n", iterations);
    
    // Prepare batch data
    const int batch_size = 5;
    const int dimensions = 4;
    float vectors[batch_size * dimensions];
    uint64_t vector_ids[batch_size];
    
    // Initialize test vectors
    for (int i = 0; i < batch_size * dimensions; i++) {
        vectors[i] = (float)(i % 10);
    }
    
    // Initialize vector IDs
    for (int i = 0; i < batch_size; i++) {
        vector_ids[i] = 100 + i;
    }
    
    double test_start = get_time_ms();
    
    for (int i = 0; i < iterations; i++) {
        start_time = get_time_ms();
        
        int fd = open(test_file, O_CREAT | O_RDWR, 0644);
        if (fd < 0) {
            errors++;
            continue;
        }
        
        // Set metadata first
        struct vexfs_vector_file_info meta = {
            .dimensions = dimensions,
            .element_type = 0,
            .vector_count = 0,
            .storage_format = 0,
            .data_offset = 0,
            .index_offset = 0,
            .compression_type = 0,
            .alignment_bytes = 32
        };
        
        if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) < 0) {
            close(fd);
            unlink(test_file);
            errors++;
            continue;
        }
        
        // Update vector IDs for this iteration
        for (int j = 0; j < batch_size; j++) {
            vector_ids[j] = 100 + i * batch_size + j;
        }
        
        // Perform batch insert
        struct vexfs_batch_insert_request batch_req = {
            .vectors = vectors,
            .vector_count = batch_size,
            .dimensions = dimensions,
            .vector_ids = vector_ids,
            .flags = 0
        };
        
        int ret = ioctl(fd, VEXFS_IOC_BATCH_INSERT, &batch_req);
        close(fd);
        
        if (ret < 0) {
            errors++;
        }
        
        end_time = get_time_ms();
        total_time += (end_time - start_time);
        
        // Clean up
        unlink(test_file);
    }
    
    double test_end = get_time_ms();
    double total_test_time = (test_end - test_start) / 1000.0;
    
    *ops_per_sec = iterations / total_test_time;
    *avg_latency_ms = total_time / iterations;
    
    printf("  Completed: %.1f ops/sec, %.2f ms avg latency, %d errors\n", 
           *ops_per_sec, *avg_latency_ms, errors);
    
    return errors;
}

int main() {
    const char *mount_point = "/tmp/vexfs_v2_monitored";
    const int iterations = 100;
    struct benchmark_results results = {0};
    
    printf("=== VexFS v2.0 Performance Benchmark (Working Filenames) ===\n");
    printf("Mount point: %s\n", mount_point);
    printf("Iterations per test: %d\n\n", iterations);
    
    // Test 1: Vector Metadata Operations
    results.metadata_errors = test_metadata_operations(mount_point, iterations, 
                                                      &results.metadata_ops_per_sec, 
                                                      &results.avg_metadata_latency_ms);
    
    printf("\n");
    
    // Test 2: Vector Search Operations
    results.search_errors = test_search_operations(mount_point, iterations, 
                                                   &results.search_ops_per_sec, 
                                                   &results.avg_search_latency_ms);
    
    printf("\n");
    
    // Test 3: Batch Insert Operations
    results.batch_errors = test_batch_operations(mount_point, iterations, 
                                                 &results.batch_ops_per_sec, 
                                                 &results.avg_batch_latency_ms);
    
    printf("\n=== BENCHMARK RESULTS SUMMARY ===\n");
    printf("Vector Metadata Operations:\n");
    printf("  Throughput: %.1f ops/sec\n", results.metadata_ops_per_sec);
    printf("  Avg Latency: %.2f ms\n", results.avg_metadata_latency_ms);
    printf("  Error Rate: %.1f%% (%d/%d)\n", 
           (results.metadata_errors * 100.0) / iterations, results.metadata_errors, iterations);
    
    printf("\nVector Search Operations:\n");
    printf("  Throughput: %.1f ops/sec\n", results.search_ops_per_sec);
    printf("  Avg Latency: %.2f ms\n", results.avg_search_latency_ms);
    printf("  Error Rate: %.1f%% (%d/%d)\n", 
           (results.search_errors * 100.0) / iterations, results.search_errors, iterations);
    
    printf("\nBatch Insert Operations:\n");
    printf("  Throughput: %.1f ops/sec\n", results.batch_ops_per_sec);
    printf("  Avg Latency: %.2f ms\n", results.avg_batch_latency_ms);
    printf("  Error Rate: %.1f%% (%d/%d)\n", 
           (results.batch_errors * 100.0) / iterations, results.batch_errors, iterations);
    
    printf("\n=== PERFORMANCE TARGETS ===\n");
    printf("Target: 100,000+ ops/sec for all operations\n");
    
    // Check if targets are met
    int targets_met = 0;
    if (results.metadata_ops_per_sec >= 100000) targets_met++;
    if (results.search_ops_per_sec >= 100000) targets_met++;
    if (results.batch_ops_per_sec >= 100000) targets_met++;
    
    printf("Targets achieved: %d/3\n", targets_met);
    
    if (targets_met == 3) {
        printf("🎉 ALL PERFORMANCE TARGETS ACHIEVED! 🎉\n");
    } else {
        printf("⚠️  Performance optimization needed for %d operation(s)\n", 3 - targets_met);
    }
    
    return 0;
}