/*
 * VexFS v2.0 Simple Performance Benchmark
 *
 * Simplified version without pthread barriers to avoid hanging issues
 */

#define _GNU_SOURCE
#define _POSIX_C_SOURCE 200112L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <time.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <pthread.h>
#include <math.h>
#include <stdint.h>
#include <getopt.h>

/* VexFS v2.0 ioctl definitions (must match kernel module) */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_file_info)
#define VEXFS_IOC_VECTOR_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search_request)
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)

/* Vector element types */
#define VEXFS_VECTOR_FLOAT32    0x01
#define VEXFS_VECTOR_FLOAT16    0x02
#define VEXFS_VECTOR_INT8       0x03
#define VEXFS_VECTOR_BINARY     0x04

/* Vector storage optimization flags */
#define VEXFS_OPT_SIMD_ALIGN    0x01
#define VEXFS_OPT_BATCH_PROC    0x02
#define VEXFS_OPT_NUMA_AWARE    0x04
#define VEXFS_OPT_COMPRESS      0x08

/* Structures (must match kernel module EXACTLY) */
struct vexfs_vector_file_info {
    uint32_t dimensions;
    uint32_t element_type;
    uint32_t vector_count;        /* FIXED: uint32_t to match kernel */
    uint32_t storage_format;
    uint64_t data_offset;
    uint64_t index_offset;
    uint32_t compression_type;
    uint32_t alignment_bytes;
};

struct vexfs_vector_search_request {
    float *query_vector;
    uint32_t dimensions;
    uint32_t k;                    /* Number of nearest neighbors */
    uint32_t search_type;          /* 0=euclidean, 1=cosine, 2=dot_product */
    float *results;                /* Output: distances */
    uint64_t *result_ids;          /* Output: vector IDs */
    uint32_t result_count;         /* Output: actual results found */
};

struct vexfs_batch_insert_request {
    float *vectors;
    uint32_t vector_count;
    uint32_t dimensions;
    uint64_t *vector_ids;          /* Optional: custom IDs */
    uint32_t flags;                /* Insert flags */
};

/* Performance metrics */
struct performance_metrics {
    uint64_t operations_completed;
    uint64_t errors;
    double throughput_ops_per_sec;
    double duration_sec;
};

/* Global test configuration */
static char mount_point[256] = "/tmp/vexfs_v2_test";
static int test_duration = 10;
static int dimensions = 128;
static int batch_size = 50;
static int k_neighbors = 10;

/* Utility functions */
static uint64_t get_time_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

static void generate_random_vector(float *vector, int dims) {
    for (int i = 0; i < dims; i++) {
        vector[i] = ((float)rand() / RAND_MAX) * 2.0f - 1.0f;
    }
}

/* Test vector metadata operations */
static struct performance_metrics test_vector_metadata(void) {
    struct performance_metrics metrics = {0};
    char test_file[512];
    struct vexfs_vector_file_info meta;
    uint64_t start_time, end_time;
    int fd;
    
    snprintf(test_file, sizeof(test_file), "%s/metadata_test", mount_point);
    
    printf("Testing Vector Metadata Operations...\n");
    
    start_time = get_time_ns();
    uint64_t end_time_target = start_time + (test_duration * 1000000000ULL);
    
    while (get_time_ns() < end_time_target) {
        fd = open(test_file, O_CREAT | O_RDWR, 0644);
        if (fd < 0) {
            metrics.errors++;
            continue;
        }
        
        memset(&meta, 0, sizeof(meta));
        meta.dimensions = dimensions;
        meta.element_type = VEXFS_VECTOR_FLOAT32;
        meta.vector_count = rand() % 10000 + 1000;
        meta.storage_format = 1;
        meta.data_offset = 0;
        meta.index_offset = 0;
        meta.compression_type = 0;
        meta.alignment_bytes = 32;
        
        int ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
        close(fd);
        unlink(test_file);
        
        if (ret == 0) {
            metrics.operations_completed++;
        } else {
            metrics.errors++;
        }
    }
    
    end_time = get_time_ns();
    metrics.duration_sec = (double)(end_time - start_time) / 1000000000.0;
    metrics.throughput_ops_per_sec = metrics.operations_completed / metrics.duration_sec;
    
    return metrics;
}

/* Test vector search operations */
static struct performance_metrics test_vector_search(void) {
    struct performance_metrics metrics = {0};
    char test_file[512];
    struct vexfs_vector_file_info meta;
    struct vexfs_vector_search_request search_req;
    float query_vector[1024];
    uint64_t result_ids[100];
    float result_distances[100];
    uint64_t start_time, end_time;
    int fd;
    
    snprintf(test_file, sizeof(test_file), "%s/search_test", mount_point);
    
    printf("Testing Vector Search Operations...\n");
    
    /* Setup test file with metadata */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create search test file: %s\n", strerror(errno));
        return metrics;
    }
    
    memset(&meta, 0, sizeof(meta));
    meta.dimensions = dimensions;
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_count = 10000;
    meta.storage_format = 1;
    meta.data_offset = 0;
    meta.index_offset = 0;
    meta.compression_type = 0;
    meta.alignment_bytes = 32;
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) < 0) {
        printf("Failed to set vector metadata for search test: %s\n", strerror(errno));
        close(fd);
        unlink(test_file);
        return metrics;
    }
    
    start_time = get_time_ns();
    uint64_t end_time_target = start_time + (test_duration * 1000000000ULL);
    
    while (get_time_ns() < end_time_target) {
        generate_random_vector(query_vector, dimensions);
        
        memset(&search_req, 0, sizeof(search_req));
        search_req.query_vector = query_vector;
        search_req.dimensions = dimensions;
        search_req.k = k_neighbors;
        search_req.search_type = 0;  /* 0=euclidean distance */
        search_req.results = result_distances;
        search_req.result_ids = result_ids;
        search_req.result_count = 0;  /* Will be set by kernel */
        
        int ret = ioctl(fd, VEXFS_IOC_VECTOR_SEARCH, &search_req);
        
        if (ret == 0) {
            metrics.operations_completed++;
        } else {
            metrics.errors++;
        }
    }
    
    close(fd);
    unlink(test_file);
    
    end_time = get_time_ns();
    metrics.duration_sec = (double)(end_time - start_time) / 1000000000.0;
    metrics.throughput_ops_per_sec = metrics.operations_completed / metrics.duration_sec;
    
    return metrics;
}

/* Test batch insert operations */
static struct performance_metrics test_batch_insert(void) {
    struct performance_metrics metrics = {0};
    char test_file[512];
    struct vexfs_vector_file_info meta;
    struct vexfs_batch_insert_request batch_req;
    float *vectors;
    uint64_t *vector_ids;
    uint64_t start_time, end_time;
    int fd;
    
    snprintf(test_file, sizeof(test_file), "%s/batch_test", mount_point);
    
    printf("Testing Batch Insert Operations...\n");
    
    /* Allocate batch data */
    vectors = malloc(batch_size * dimensions * sizeof(float));
    vector_ids = malloc(batch_size * sizeof(uint64_t));
    
    if (!vectors || !vector_ids) {
        printf("Failed to allocate batch data\n");
        return metrics;
    }
    
    /* Setup test file with metadata */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create batch test file: %s\n", strerror(errno));
        free(vectors);
        free(vector_ids);
        return metrics;
    }
    
    memset(&meta, 0, sizeof(meta));
    meta.dimensions = dimensions;
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_count = 0;
    meta.storage_format = 1;
    meta.data_offset = 0;
    meta.index_offset = 0;
    meta.compression_type = 0;
    meta.alignment_bytes = 32;
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) < 0) {
        printf("Failed to set vector metadata for batch test: %s\n", strerror(errno));
        close(fd);
        unlink(test_file);
        free(vectors);
        free(vector_ids);
        return metrics;
    }
    
    start_time = get_time_ns();
    uint64_t end_time_target = start_time + (test_duration * 1000000000ULL);
    
    while (get_time_ns() < end_time_target) {
        /* Generate batch data */
        for (int i = 0; i < batch_size; i++) {
            generate_random_vector(&vectors[i * dimensions], dimensions);
            vector_ids[i] = rand();
        }
        
        memset(&batch_req, 0, sizeof(batch_req));
        batch_req.vectors = vectors;
        batch_req.vector_count = batch_size;
        batch_req.dimensions = dimensions;
        batch_req.vector_ids = vector_ids;
        batch_req.flags = VEXFS_OPT_SIMD_ALIGN;
        
        int ret = ioctl(fd, VEXFS_IOC_BATCH_INSERT, &batch_req);
        
        if (ret == 0) {
            metrics.operations_completed++;
        } else {
            metrics.errors++;
        }
    }
    
    close(fd);
    unlink(test_file);
    free(vectors);
    free(vector_ids);
    
    end_time = get_time_ns();
    metrics.duration_sec = (double)(end_time - start_time) / 1000000000.0;
    metrics.throughput_ops_per_sec = metrics.operations_completed / metrics.duration_sec;
    
    return metrics;
}

static void print_results(const char *test_name, struct performance_metrics *metrics) {
    printf("\n=== %s Results ===\n", test_name);
    printf("Operations Completed: %lu\n", metrics->operations_completed);
    printf("Errors: %lu\n", metrics->errors);
    printf("Duration: %.2f seconds\n", metrics->duration_sec);
    printf("Throughput: %.2f ops/sec\n", metrics->throughput_ops_per_sec);
    if (metrics->operations_completed + metrics->errors > 0) {
        printf("Success Rate: %.2f%%\n", 
               (double)metrics->operations_completed / 
               (metrics->operations_completed + metrics->errors) * 100.0);
    }
}

static void print_usage(const char *program_name) {
    printf("Usage: %s [options]\n", program_name);
    printf("Options:\n");
    printf("  -m <mount_point>  VexFS v2.0 mount point (default: /tmp/vexfs_v2_test)\n");
    printf("  -d <duration>     Test duration in seconds (default: 10)\n");
    printf("  -D <dimensions>   Vector dimensions (default: 128)\n");
    printf("  -b <batch_size>   Batch size for insert operations (default: 50)\n");
    printf("  -k <neighbors>    K neighbors for search (default: 10)\n");
    printf("  -h                Show this help\n");
}

int main(int argc, char *argv[]) {
    int opt;
    
    while ((opt = getopt(argc, argv, "m:d:D:b:k:h")) != -1) {
        switch (opt) {
            case 'm':
                strncpy(mount_point, optarg, sizeof(mount_point) - 1);
                break;
            case 'd':
                test_duration = atoi(optarg);
                break;
            case 'D':
                dimensions = atoi(optarg);
                if (dimensions > 1024) dimensions = 1024;
                break;
            case 'b':
                batch_size = atoi(optarg);
                if (batch_size > 1000) batch_size = 1000;
                break;
            case 'k':
                k_neighbors = atoi(optarg);
                break;
            case 'h':
                print_usage(argv[0]);
                return 0;
            default:
                print_usage(argv[0]);
                return 1;
        }
    }
    
    /* Validate mount point exists */
    struct stat st;
    if (stat(mount_point, &st) != 0) {
        printf("Error: Mount point %s does not exist\n", mount_point);
        return 1;
    }
    
    printf("\n🚀 VexFS v2.0 Simple Performance Benchmark\n");
    printf("==========================================\n");
    printf("Mount Point: %s\n", mount_point);
    printf("Test Duration: %d seconds\n", test_duration);
    printf("Dimensions: %d\n", dimensions);
    printf("Batch Size: %d\n", batch_size);
    printf("K Neighbors: %d\n", k_neighbors);
    printf("\n");
    
    /* Initialize random seed */
    srand(time(NULL));
    
    /* Run sequential tests */
    struct performance_metrics metadata_metrics = test_vector_metadata();
    struct performance_metrics search_metrics = test_vector_search();
    struct performance_metrics batch_metrics = test_batch_insert();
    
    /* Print individual results */
    print_results("Vector Metadata Operations", &metadata_metrics);
    print_results("Vector Search Operations", &search_metrics);
    print_results("Batch Insert Operations", &batch_metrics);
    
    /* Overall summary */
    double total_throughput = metadata_metrics.throughput_ops_per_sec + 
                             search_metrics.throughput_ops_per_sec + 
                             batch_metrics.throughput_ops_per_sec;
    uint64_t total_ops = metadata_metrics.operations_completed + 
                        search_metrics.operations_completed + 
                        batch_metrics.operations_completed;
    
    printf("\n🎯 OVERALL PERFORMANCE SUMMARY\n");
    printf("==============================\n");
    printf("Total Operations: %lu\n", total_ops);
    printf("Combined Throughput: %.2f ops/sec\n", total_throughput);
    printf("Target Achievement: %.1f%% (Target: 100,000 ops/sec)\n", 
           (total_throughput / 100000.0) * 100.0);
    
    if (total_throughput >= 100000.0) {
        printf("🎉 TARGET ACHIEVED! VexFS v2.0 exceeds 100,000 ops/sec!\n");
    } else {
        printf("🔧 Optimization needed to reach 100,000 ops/sec target\n");
    }
    
    return 0;
}