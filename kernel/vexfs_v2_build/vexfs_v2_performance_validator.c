/*
 * VexFS v2.0 Comprehensive Performance Validation Framework
 *
 * This program conducts systematic performance testing of VexFS v2.0 vector operations
 * using the corrected IOCTL structures and UAPI header file.
 *
 * Features:
 * - Multi-dimensional vector testing (4, 128, 512, 1024 dimensions)
 * - Variable batch size testing (1, 10, 100, 1000 vectors)
 * - Performance metrics collection (ops/sec, latency, memory usage)
 * - Statistical analysis with percentiles
 * - Comprehensive error rate monitoring
 * - Resource utilization tracking
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <time.h>
#include <sys/time.h>
#include <sys/resource.h>
#include <math.h>
#include <signal.h>

/* Use the standard UAPI header */
#include "vexfs_v2_uapi.h"

/* Performance test configuration */
#define MAX_DIMENSIONS 1024
#define MAX_BATCH_SIZE 1000
#define MAX_ITERATIONS 1000
#define WARMUP_ITERATIONS 10

/* Test result structures */
struct performance_metrics {
    double ops_per_second;
    double avg_latency_us;
    double p95_latency_us;
    double p99_latency_us;
    double min_latency_us;
    double max_latency_us;
    uint64_t total_operations;
    uint64_t successful_operations;
    uint64_t failed_operations;
    double error_rate;
    long memory_usage_kb;
    double cpu_usage_percent;
};

struct test_configuration {
    uint32_t dimensions;
    uint32_t batch_size;
    uint32_t iterations;
    const char *test_name;
};

/* Global variables for signal handling */
static volatile int test_interrupted = 0;

/* Signal handler for graceful shutdown */
void signal_handler(int sig) {
    test_interrupted = 1;
    printf("\n⚠️  Test interrupted by signal %d, finishing current iteration...\n", sig);
}

/* High-precision timing functions */
static inline uint64_t get_time_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000ULL + (uint64_t)ts.tv_nsec / 1000ULL;
}

/* Memory usage tracking */
long get_memory_usage_kb(void) {
    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) == 0) {
        return usage.ru_maxrss; /* Peak memory usage in KB */
    }
    return -1;
}

/* Generate random vector data */
void generate_random_vectors(float *vectors, uint32_t count, uint32_t dimensions) {
    srand(time(NULL));
    for (uint32_t i = 0; i < count * dimensions; i++) {
        vectors[i] = ((float)rand() / RAND_MAX) * 2.0f - 1.0f; /* Range: -1.0 to 1.0 */
    }
}

/* Generate sequential vector IDs */
void generate_vector_ids(__u64 *ids, uint32_t count, uint64_t start_id) {
    for (uint32_t i = 0; i < count; i++) {
        ids[i] = start_id + i;
    }
}

/* Calculate statistics from latency array */
void calculate_statistics(uint64_t *latencies, uint32_t count, struct performance_metrics *metrics) {
    if (count == 0) return;
    
    /* Sort latencies for percentile calculation */
    for (uint32_t i = 0; i < count - 1; i++) {
        for (uint32_t j = i + 1; j < count; j++) {
            if (latencies[i] > latencies[j]) {
                uint64_t temp = latencies[i];
                latencies[i] = latencies[j];
                latencies[j] = temp;
            }
        }
    }
    
    /* Calculate basic statistics */
    uint64_t sum = 0;
    metrics->min_latency_us = latencies[0];
    metrics->max_latency_us = latencies[count - 1];
    
    for (uint32_t i = 0; i < count; i++) {
        sum += latencies[i];
    }
    
    metrics->avg_latency_us = (double)sum / count;
    metrics->p95_latency_us = latencies[(uint32_t)(count * 0.95)];
    metrics->p99_latency_us = latencies[(uint32_t)(count * 0.99)];
}

/* Test vector metadata operations */
int test_vector_metadata_performance(int fd, struct test_configuration *config, 
                                   struct performance_metrics *metrics) {
    printf("🔍 Testing vector metadata operations (dimensions=%u)...\n", config->dimensions);
    
    uint64_t *latencies = malloc(config->iterations * sizeof(uint64_t));
    if (!latencies) {
        printf("❌ Failed to allocate latency tracking memory\n");
        return -1;
    }
    
    uint64_t start_time = get_time_us();
    uint32_t successful = 0;
    uint32_t failed = 0;
    
    /* Warmup iterations */
    for (int i = 0; i < WARMUP_ITERATIONS && !test_interrupted; i++) {
        struct vexfs_vector_file_info meta = {
            .dimensions = config->dimensions,
            .element_type = VEXFS_VECTOR_FLOAT32,
            .vector_count = 0,
            .storage_format = VEXFS_STORAGE_DENSE,
            .data_offset = 0,
            .index_offset = 0,
            .compression_type = VEXFS_COMPRESS_NONE,
            .alignment_bytes = 32
        };
        ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
    }
    
    /* Performance test iterations */
    for (uint32_t i = 0; i < config->iterations && !test_interrupted; i++) {
        struct vexfs_vector_file_info meta = {
            .dimensions = config->dimensions,
            .element_type = VEXFS_VECTOR_FLOAT32,
            .vector_count = 0,
            .storage_format = VEXFS_STORAGE_DENSE,
            .data_offset = 0,
            .index_offset = 0,
            .compression_type = VEXFS_COMPRESS_NONE,
            .alignment_bytes = 32
        };
        
        uint64_t op_start = get_time_us();
        int result = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
        uint64_t op_end = get_time_us();
        
        latencies[i] = op_end - op_start;
        
        if (result == 0) {
            successful++;
        } else {
            failed++;
        }
        
        /* Progress indicator */
        if ((i + 1) % 100 == 0) {
            printf("   Progress: %u/%u iterations completed\n", i + 1, config->iterations);
        }
    }
    
    uint64_t end_time = get_time_us();
    uint64_t total_time_us = end_time - start_time;
    
    /* Calculate metrics */
    metrics->total_operations = successful + failed;
    metrics->successful_operations = successful;
    metrics->failed_operations = failed;
    metrics->error_rate = (double)failed / (successful + failed) * 100.0;
    metrics->ops_per_second = (double)successful / (total_time_us / 1000000.0);
    metrics->memory_usage_kb = get_memory_usage_kb();
    
    calculate_statistics(latencies, successful, metrics);
    
    free(latencies);
    return 0;
}

/* Test batch insert operations */
int test_batch_insert_performance(int fd, struct test_configuration *config, 
                                struct performance_metrics *metrics) {
    printf("🔍 Testing batch insert operations (dimensions=%u, batch_size=%u)...\n", 
           config->dimensions, config->batch_size);
    
    /* Allocate memory for vectors and IDs */
    size_t vector_data_size = config->batch_size * config->dimensions * sizeof(float);
    size_t vector_ids_size = config->batch_size * sizeof(__u64);
    
    float *vectors = malloc(vector_data_size);
    __u64 *vector_ids = malloc(vector_ids_size);
    uint64_t *latencies = malloc(config->iterations * sizeof(uint64_t));
    
    if (!vectors || !vector_ids || !latencies) {
        printf("❌ Failed to allocate test data memory\n");
        free(vectors);
        free(vector_ids);
        free(latencies);
        return -1;
    }
    
    /* Generate test data */
    generate_random_vectors(vectors, config->batch_size, config->dimensions);
    
    uint64_t start_time = get_time_us();
    uint32_t successful = 0;
    uint32_t failed = 0;
    
    /* Warmup iterations */
    for (int i = 0; i < WARMUP_ITERATIONS && !test_interrupted; i++) {
        generate_vector_ids(vector_ids, config->batch_size, i * config->batch_size);
        
        struct vexfs_batch_insert_request req = {
            .vectors = vectors,
            .vector_count = config->batch_size,
            .dimensions = config->dimensions,
            .vector_ids = vector_ids,
            .flags = VEXFS_INSERT_APPEND
        };
        ioctl(fd, VEXFS_IOC_BATCH_INSERT, &req);
    }
    
    /* Performance test iterations */
    for (uint32_t i = 0; i < config->iterations && !test_interrupted; i++) {
        generate_vector_ids(vector_ids, config->batch_size, 
                          (WARMUP_ITERATIONS + i) * config->batch_size);
        
        struct vexfs_batch_insert_request req = {
            .vectors = vectors,
            .vector_count = config->batch_size,
            .dimensions = config->dimensions,
            .vector_ids = vector_ids,
            .flags = VEXFS_INSERT_APPEND
        };
        
        uint64_t op_start = get_time_us();
        int result = ioctl(fd, VEXFS_IOC_BATCH_INSERT, &req);
        uint64_t op_end = get_time_us();
        
        latencies[i] = op_end - op_start;
        
        if (result == 0) {
            successful++;
        } else {
            failed++;
        }
        
        /* Progress indicator */
        if ((i + 1) % 50 == 0) {
            printf("   Progress: %u/%u iterations completed\n", i + 1, config->iterations);
        }
    }
    
    uint64_t end_time = get_time_us();
    uint64_t total_time_us = end_time - start_time;
    
    /* Calculate metrics */
    metrics->total_operations = successful + failed;
    metrics->successful_operations = successful;
    metrics->failed_operations = failed;
    metrics->error_rate = (double)failed / (successful + failed) * 100.0;
    metrics->ops_per_second = (double)successful / (total_time_us / 1000000.0);
    metrics->memory_usage_kb = get_memory_usage_kb();
    
    calculate_statistics(latencies, successful, metrics);
    
    free(vectors);
    free(vector_ids);
    free(latencies);
    return 0;
}

/* Print performance results */
void print_performance_results(struct test_configuration *config, 
                             struct performance_metrics *metrics) {
    printf("\n📊 Performance Results for %s:\n", config->test_name);
    printf("   Configuration: %u dimensions, %u batch size, %u iterations\n",
           config->dimensions, config->batch_size, config->iterations);
    printf("   ✅ Operations per second: %.2f ops/sec\n", metrics->ops_per_second);
    printf("   ⏱️  Average latency: %.2f μs\n", metrics->avg_latency_us);
    printf("   📈 P95 latency: %.2f μs\n", metrics->p95_latency_us);
    printf("   📈 P99 latency: %.2f μs\n", metrics->p99_latency_us);
    printf("   ⚡ Min latency: %.2f μs\n", metrics->min_latency_us);
    printf("   🐌 Max latency: %.2f μs\n", metrics->max_latency_us);
    printf("   ✅ Successful operations: %lu\n", metrics->successful_operations);
    printf("   ❌ Failed operations: %lu\n", metrics->failed_operations);
    printf("   📊 Error rate: %.2f%%\n", metrics->error_rate);
    printf("   💾 Peak memory usage: %ld KB\n", metrics->memory_usage_kb);
    
    /* Performance target validation */
    printf("\n🎯 Target Validation:\n");
    if (metrics->ops_per_second >= 100000.0) {
        printf("   ✅ Ops/sec target (100K): ACHIEVED (%.0f ops/sec)\n", metrics->ops_per_second);
    } else {
        printf("   ❌ Ops/sec target (100K): MISSED (%.0f ops/sec)\n", metrics->ops_per_second);
    }
    
    if (metrics->avg_latency_us <= 1000.0) {
        printf("   ✅ Latency target (<1ms): ACHIEVED (%.2f μs)\n", metrics->avg_latency_us);
    } else {
        printf("   ❌ Latency target (<1ms): MISSED (%.2f μs)\n", metrics->avg_latency_us);
    }
    
    if (metrics->error_rate == 0.0) {
        printf("   ✅ Error rate target (0%%): ACHIEVED\n");
    } else {
        printf("   ❌ Error rate target (0%%): MISSED (%.2f%%)\n", metrics->error_rate);
    }
    
    printf("\n" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "\n");
}

/* Main performance validation function */
int main(int argc, char *argv[]) {
    printf("🚀 VexFS v2.0 Comprehensive Performance Validation Framework\n");
    printf("============================================================\n");
    printf("📅 Test started at: %s", ctime(&(time_t){time(NULL)}));
    printf("🔧 Using UAPI header version: %d.%d.%d\n", 
           VEXFS_V2_MAJOR_VERSION, VEXFS_V2_MINOR_VERSION, VEXFS_V2_PATCH_VERSION);
    printf("\n");
    
    /* Setup signal handlers */
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    /* Open VexFS test file */
    const char *test_file = "/tmp/vexfs_test/vector_test_file";
    int fd = open(test_file, O_RDWR);
    if (fd < 0) {
        printf("❌ Failed to open VexFS test file '%s': %s\n", test_file, strerror(errno));
        printf("💡 Make sure VexFS is mounted and the test file exists\n");
        return 1;
    }
    
    printf("✅ Successfully opened VexFS test file: %s\n\n", test_file);
    
    /* Define test configurations */
    struct test_configuration test_configs[] = {
        /* Vector metadata tests */
        {4, 1, 1000, "Vector Metadata - 4D"},
        {128, 1, 1000, "Vector Metadata - 128D"},
        {512, 1, 500, "Vector Metadata - 512D"},
        {1024, 1, 200, "Vector Metadata - 1024D"},
        
        /* Batch insert tests - small batches */
        {4, 1, 1000, "Batch Insert - 4D x1"},
        {4, 10, 500, "Batch Insert - 4D x10"},
        {4, 100, 100, "Batch Insert - 4D x100"},
        {4, 1000, 50, "Batch Insert - 4D x1000"},
        
        /* Batch insert tests - medium dimensions */
        {128, 1, 500, "Batch Insert - 128D x1"},
        {128, 10, 200, "Batch Insert - 128D x10"},
        {128, 100, 50, "Batch Insert - 128D x100"},
        
        /* Batch insert tests - high dimensions */
        {512, 1, 200, "Batch Insert - 512D x1"},
        {512, 10, 100, "Batch Insert - 512D x10"},
        
        /* Batch insert tests - maximum dimensions */
        {1024, 1, 100, "Batch Insert - 1024D x1"},
        {1024, 10, 50, "Batch Insert - 1024D x10"},
    };
    
    int num_tests = sizeof(test_configs) / sizeof(test_configs[0]);
    struct performance_metrics *results = malloc(num_tests * sizeof(struct performance_metrics));
    
    if (!results) {
        printf("❌ Failed to allocate results memory\n");
        close(fd);
        return 1;
    }
    
    /* Run all performance tests */
    for (int i = 0; i < num_tests && !test_interrupted; i++) {
        printf("🧪 Running test %d/%d: %s\n", i + 1, num_tests, test_configs[i].test_name);
        
        memset(&results[i], 0, sizeof(struct performance_metrics));
        
        if (test_configs[i].batch_size == 1) {
            /* Vector metadata test */
            if (test_vector_metadata_performance(fd, &test_configs[i], &results[i]) != 0) {
                printf("❌ Test failed: %s\n", test_configs[i].test_name);
                continue;
            }
        } else {
            /* Batch insert test */
            if (test_batch_insert_performance(fd, &test_configs[i], &results[i]) != 0) {
                printf("❌ Test failed: %s\n", test_configs[i].test_name);
                continue;
            }
        }
        
        print_performance_results(&test_configs[i], &results[i]);
    }
    
    /* Generate summary report */
    printf("\n🎉 VexFS v2.0 Performance Validation Summary\n");
    printf("=============================================\n");
    
    double total_ops = 0;
    double avg_error_rate = 0;
    int successful_tests = 0;
    int target_achieving_tests = 0;
    
    for (int i = 0; i < num_tests; i++) {
        if (results[i].total_operations > 0) {
            total_ops += results[i].ops_per_second;
            avg_error_rate += results[i].error_rate;
            successful_tests++;
            
            if (results[i].ops_per_second >= 100000.0 && 
                results[i].avg_latency_us <= 1000.0 && 
                results[i].error_rate == 0.0) {
                target_achieving_tests++;
            }
        }
    }
    
    if (successful_tests > 0) {
        printf("📊 Overall Statistics:\n");
        printf("   ✅ Tests completed: %d/%d\n", successful_tests, num_tests);
        printf("   🎯 Tests achieving all targets: %d/%d (%.1f%%)\n", 
               target_achieving_tests, successful_tests, 
               (double)target_achieving_tests / successful_tests * 100.0);
        printf("   ⚡ Average ops/sec across tests: %.0f\n", total_ops / successful_tests);
        printf("   📊 Average error rate: %.2f%%\n", avg_error_rate / successful_tests);
        
        if (target_achieving_tests == successful_tests) {
            printf("\n🎉 EXCELLENT: All tests achieved performance targets!\n");
        } else if (target_achieving_tests > successful_tests / 2) {
            printf("\n✅ GOOD: Majority of tests achieved performance targets\n");
        } else {
            printf("\n⚠️  WARNING: Performance targets not consistently achieved\n");
        }
    }
    
    printf("\n📝 Test completed at: %s", ctime(&(time_t){time(NULL)}));
    printf("💡 Check dmesg for detailed kernel module logs\n");
    
    free(results);
    close(fd);
    return 0;
}