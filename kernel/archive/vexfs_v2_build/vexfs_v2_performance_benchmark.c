/*
 * VexFS v2.0 Performance Benchmarking Suite
 *
 * Comprehensive performance testing for kernel-native vector operations
 * targeting 100,000+ ops/sec for both basic and vector operations.
 */

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
#include <limits.h>

#define _GNU_SOURCE
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200112L
#endif

/* VexFS v2.0 ioctl definitions */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_metadata)
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_metadata)
#define VEXFS_IOC_VECTOR_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search_request)
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)

#define VEXFS_VECTOR_FLOAT32 1

/* VexFS v2.0 structures */
struct vexfs_vector_metadata {
    uint32_t dimensions;
    uint32_t element_type;
    uint64_t vector_count;
    uint32_t storage_format;
    uint64_t data_offset;
    uint64_t index_offset;
    uint32_t compression_type;
    uint32_t alignment_bytes;
};

struct vexfs_vector_search_request {
    uint32_t dimensions;
    uint32_t k;
    float *query_vector;
    uint64_t *result_ids;
    float *result_distances;
    uint32_t flags;
};

struct vexfs_batch_insert_request {
    uint32_t dimensions;
    uint32_t vector_count;
    float *vectors;
    uint64_t *vector_ids;
    uint32_t flags;
};

/* Benchmark configuration */
#define MAX_THREADS 32
#define MAX_DIMENSIONS 1024
#define MAX_VECTORS_PER_BATCH 1000
#define DEFAULT_TEST_DURATION 10  /* seconds */
#define DEFAULT_WARMUP_DURATION 2  /* seconds */

/* Performance metrics */
struct performance_metrics {
    uint64_t operations_completed;
    uint64_t total_latency_ns;
    uint64_t min_latency_ns;
    uint64_t max_latency_ns;
    uint64_t errors;
    double throughput_ops_per_sec;
    double avg_latency_ms;
    double p95_latency_ms;
    double p99_latency_ms;
};

/* Thread context for concurrent testing */
struct thread_context {
    int thread_id;
    char *mount_point;
    int test_duration;
    int warmup_duration;
    int dimensions;
    int batch_size;
    int k_neighbors;
    struct performance_metrics metrics;
    pthread_barrier_t *start_barrier;
    volatile int *stop_flag;
};

/* Benchmark test types */
enum benchmark_type {
    BENCH_VECTOR_METADATA,
    BENCH_VECTOR_SEARCH,
    BENCH_BATCH_INSERT,
    BENCH_MIXED_WORKLOAD,
    BENCH_CONCURRENT_ACCESS,
    BENCH_SCALABILITY
};

/* Global configuration */
struct benchmark_config {
    char mount_point[256];
    int num_threads;
    int test_duration;
    int warmup_duration;
    int dimensions;
    int batch_size;
    int k_neighbors;
    int vector_count;
    enum benchmark_type test_type;
    int verbose;
};

/* Utility functions */
static uint64_t get_time_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

static double ns_to_ms(uint64_t ns) {
    return (double)ns / 1000000.0;
}

static void generate_random_vector(float *vector, int dimensions) {
    for (int i = 0; i < dimensions; i++) {
        vector[i] = ((float)rand() / RAND_MAX) * 2.0f - 1.0f;
    }
}

static void print_performance_metrics(const char *test_name, struct performance_metrics *metrics) {
    printf("\n=== %s Performance Results ===\n", test_name);
    printf("Operations Completed: %lu\n", metrics->operations_completed);
    printf("Throughput: %.2f ops/sec\n", metrics->throughput_ops_per_sec);
    printf("Average Latency: %.3f ms\n", metrics->avg_latency_ms);
    printf("Min Latency: %.3f ms\n", ns_to_ms(metrics->min_latency_ns));
    printf("Max Latency: %.3f ms\n", ns_to_ms(metrics->max_latency_ns));
    printf("P95 Latency: %.3f ms\n", metrics->p95_latency_ms);
    printf("P99 Latency: %.3f ms\n", metrics->p99_latency_ms);
    printf("Errors: %lu\n", metrics->errors);
    printf("Success Rate: %.2f%%\n", 
           (double)(metrics->operations_completed) / 
           (metrics->operations_completed + metrics->errors) * 100.0);
}

/* Vector metadata benchmark */
static void benchmark_vector_metadata(struct thread_context *ctx) {
    char test_file[512];
    struct vexfs_vector_metadata meta;
    uint64_t start_time, end_time, operation_start;
    int fd;
    
    snprintf(test_file, sizeof(test_file), "%s/metadata_test_%d", 
             ctx->mount_point, ctx->thread_id);
    
    /* Initialize metrics */
    memset(&ctx->metrics, 0, sizeof(ctx->metrics));
    ctx->metrics.min_latency_ns = UINT64_MAX;
    
    /* Warmup phase */
    uint64_t warmup_end = get_time_ns() + (ctx->warmup_duration * 1000000000ULL);
    while (get_time_ns() < warmup_end) {
        fd = open(test_file, O_CREAT | O_RDWR, 0644);
        if (fd >= 0) {
            memset(&meta, 0, sizeof(meta));
            meta.dimensions = ctx->dimensions;
            meta.element_type = VEXFS_VECTOR_FLOAT32;
            meta.vector_count = 1000;
            meta.storage_format = 1;
            meta.alignment_bytes = 32;
            
            ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
            close(fd);
            unlink(test_file);
        }
    }
    
    /* Wait for all threads to complete warmup */
    pthread_barrier_wait(ctx->start_barrier);
    
    /* Actual benchmark */
    start_time = get_time_ns();
    uint64_t end_time_target = start_time + (ctx->test_duration * 1000000000ULL);
    
    while (get_time_ns() < end_time_target && !(*ctx->stop_flag)) {
        operation_start = get_time_ns();
        
        fd = open(test_file, O_CREAT | O_RDWR, 0644);
        if (fd < 0) {
            ctx->metrics.errors++;
            continue;
        }
        
        memset(&meta, 0, sizeof(meta));
        meta.dimensions = ctx->dimensions;
        meta.element_type = VEXFS_VECTOR_FLOAT32;
        meta.vector_count = rand() % 10000 + 1000;
        meta.storage_format = 1;
        meta.alignment_bytes = 32;
        
        int ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
        close(fd);
        unlink(test_file);
        
        end_time = get_time_ns();
        uint64_t latency = end_time - operation_start;
        
        if (ret == 0) {
            ctx->metrics.operations_completed++;
            ctx->metrics.total_latency_ns += latency;
            
            if (latency < ctx->metrics.min_latency_ns) {
                ctx->metrics.min_latency_ns = latency;
            }
            if (latency > ctx->metrics.max_latency_ns) {
                ctx->metrics.max_latency_ns = latency;
            }
        } else {
            ctx->metrics.errors++;
        }
    }
    
    /* Calculate final metrics */
    end_time = get_time_ns();
    double duration_sec = (double)(end_time - start_time) / 1000000000.0;
    ctx->metrics.throughput_ops_per_sec = ctx->metrics.operations_completed / duration_sec;
    
    if (ctx->metrics.operations_completed > 0) {
        ctx->metrics.avg_latency_ms = ns_to_ms(ctx->metrics.total_latency_ns / ctx->metrics.operations_completed);
    }
}

/* Vector search benchmark */
static void benchmark_vector_search(struct thread_context *ctx) {
    char test_file[512];
    struct vexfs_vector_metadata meta;
    struct vexfs_vector_search_request search_req;
    float query_vector[MAX_DIMENSIONS];
    uint64_t result_ids[100];
    float result_distances[100];
    uint64_t start_time, end_time, operation_start;
    int fd;
    
    snprintf(test_file, sizeof(test_file), "%s/search_test_%d", 
             ctx->mount_point, ctx->thread_id);
    
    /* Initialize metrics */
    memset(&ctx->metrics, 0, sizeof(ctx->metrics));
    ctx->metrics.min_latency_ns = UINT64_MAX;
    
    /* Setup test file with metadata */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create test file: %s\n", strerror(errno));
        return;
    }
    
    memset(&meta, 0, sizeof(meta));
    meta.dimensions = ctx->dimensions;
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_count = 10000;
    meta.storage_format = 1;
    meta.alignment_bytes = 32;
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) < 0) {
        printf("Failed to set vector metadata: %s\n", strerror(errno));
        close(fd);
        return;
    }
    
    /* Warmup phase */
    uint64_t warmup_end = get_time_ns() + (ctx->warmup_duration * 1000000000ULL);
    while (get_time_ns() < warmup_end) {
        generate_random_vector(query_vector, ctx->dimensions);
        
        memset(&search_req, 0, sizeof(search_req));
        search_req.dimensions = ctx->dimensions;
        search_req.k = ctx->k_neighbors;
        search_req.query_vector = query_vector;
        search_req.result_ids = result_ids;
        search_req.result_distances = result_distances;
        search_req.flags = 0;
        
        ioctl(fd, VEXFS_IOC_VECTOR_SEARCH, &search_req);
    }
    
    /* Wait for all threads to complete warmup */
    pthread_barrier_wait(ctx->start_barrier);
    
    /* Actual benchmark */
    start_time = get_time_ns();
    uint64_t end_time_target = start_time + (ctx->test_duration * 1000000000ULL);
    
    while (get_time_ns() < end_time_target && !(*ctx->stop_flag)) {
        generate_random_vector(query_vector, ctx->dimensions);
        
        operation_start = get_time_ns();
        
        memset(&search_req, 0, sizeof(search_req));
        search_req.dimensions = ctx->dimensions;
        search_req.k = ctx->k_neighbors;
        search_req.query_vector = query_vector;
        search_req.result_ids = result_ids;
        search_req.result_distances = result_distances;
        search_req.flags = 0;
        
        int ret = ioctl(fd, VEXFS_IOC_VECTOR_SEARCH, &search_req);
        
        end_time = get_time_ns();
        uint64_t latency = end_time - operation_start;
        
        if (ret == 0) {
            ctx->metrics.operations_completed++;
            ctx->metrics.total_latency_ns += latency;
            
            if (latency < ctx->metrics.min_latency_ns) {
                ctx->metrics.min_latency_ns = latency;
            }
            if (latency > ctx->metrics.max_latency_ns) {
                ctx->metrics.max_latency_ns = latency;
            }
        } else {
            ctx->metrics.errors++;
        }
    }
    
    close(fd);
    unlink(test_file);
    
    /* Calculate final metrics */
    end_time = get_time_ns();
    double duration_sec = (double)(end_time - start_time) / 1000000000.0;
    ctx->metrics.throughput_ops_per_sec = ctx->metrics.operations_completed / duration_sec;
    
    if (ctx->metrics.operations_completed > 0) {
        ctx->metrics.avg_latency_ms = ns_to_ms(ctx->metrics.total_latency_ns / ctx->metrics.operations_completed);
    }
}

/* Batch insert benchmark */
static void benchmark_batch_insert(struct thread_context *ctx) {
    char test_file[512];
    struct vexfs_vector_metadata meta;
    struct vexfs_batch_insert_request batch_req;
    float *vectors;
    uint64_t *vector_ids;
    uint64_t start_time, end_time, operation_start;
    int fd;
    
    snprintf(test_file, sizeof(test_file), "%s/batch_test_%d", 
             ctx->mount_point, ctx->thread_id);
    
    /* Allocate batch data */
    vectors = malloc(ctx->batch_size * ctx->dimensions * sizeof(float));
    vector_ids = malloc(ctx->batch_size * sizeof(uint64_t));
    
    if (!vectors || !vector_ids) {
        printf("Failed to allocate batch data\n");
        return;
    }
    
    /* Initialize metrics */
    memset(&ctx->metrics, 0, sizeof(ctx->metrics));
    ctx->metrics.min_latency_ns = UINT64_MAX;
    
    /* Setup test file with metadata */
    fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("Failed to create test file: %s\n", strerror(errno));
        free(vectors);
        free(vector_ids);
        return;
    }
    
    memset(&meta, 0, sizeof(meta));
    meta.dimensions = ctx->dimensions;
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_count = 0;
    meta.storage_format = 1;
    meta.alignment_bytes = 32;
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) < 0) {
        printf("Failed to set vector metadata: %s\n", strerror(errno));
        close(fd);
        free(vectors);
        free(vector_ids);
        return;
    }
    
    /* Warmup phase */
    uint64_t warmup_end = get_time_ns() + (ctx->warmup_duration * 1000000000ULL);
    while (get_time_ns() < warmup_end) {
        for (int i = 0; i < ctx->batch_size; i++) {
            generate_random_vector(&vectors[i * ctx->dimensions], ctx->dimensions);
            vector_ids[i] = rand();
        }
        
        memset(&batch_req, 0, sizeof(batch_req));
        batch_req.dimensions = ctx->dimensions;
        batch_req.vector_count = ctx->batch_size;
        batch_req.vectors = vectors;
        batch_req.vector_ids = vector_ids;
        batch_req.flags = 0;
        
        ioctl(fd, VEXFS_IOC_BATCH_INSERT, &batch_req);
    }
    
    /* Wait for all threads to complete warmup */
    pthread_barrier_wait(ctx->start_barrier);
    
    /* Actual benchmark */
    start_time = get_time_ns();
    uint64_t end_time_target = start_time + (ctx->test_duration * 1000000000ULL);
    
    while (get_time_ns() < end_time_target && !(*ctx->stop_flag)) {
        /* Generate batch data */
        for (int i = 0; i < ctx->batch_size; i++) {
            generate_random_vector(&vectors[i * ctx->dimensions], ctx->dimensions);
            vector_ids[i] = rand();
        }
        
        operation_start = get_time_ns();
        
        memset(&batch_req, 0, sizeof(batch_req));
        batch_req.dimensions = ctx->dimensions;
        batch_req.vector_count = ctx->batch_size;
        batch_req.vectors = vectors;
        batch_req.vector_ids = vector_ids;
        batch_req.flags = 0;
        
        int ret = ioctl(fd, VEXFS_IOC_BATCH_INSERT, &batch_req);
        
        end_time = get_time_ns();
        uint64_t latency = end_time - operation_start;
        
        if (ret == 0) {
            ctx->metrics.operations_completed++;
            ctx->metrics.total_latency_ns += latency;
            
            if (latency < ctx->metrics.min_latency_ns) {
                ctx->metrics.min_latency_ns = latency;
            }
            if (latency > ctx->metrics.max_latency_ns) {
                ctx->metrics.max_latency_ns = latency;
            }
        } else {
            ctx->metrics.errors++;
        }
    }
    
    close(fd);
    unlink(test_file);
    free(vectors);
    free(vector_ids);
    
    /* Calculate final metrics */
    end_time = get_time_ns();
    double duration_sec = (double)(end_time - start_time) / 1000000000.0;
    ctx->metrics.throughput_ops_per_sec = ctx->metrics.operations_completed / duration_sec;
    
    if (ctx->metrics.operations_completed > 0) {
        ctx->metrics.avg_latency_ms = ns_to_ms(ctx->metrics.total_latency_ns / ctx->metrics.operations_completed);
    }
}

/* Thread worker function */
static void* benchmark_worker(void *arg) {
    struct thread_context *ctx = (struct thread_context*)arg;
    
    /* Set thread-specific random seed */
    srand(time(NULL) + ctx->thread_id);
    
    switch (ctx->thread_id % 3) {
        case 0:
            benchmark_vector_metadata(ctx);
            break;
        case 1:
            benchmark_vector_search(ctx);
            break;
        case 2:
            benchmark_batch_insert(ctx);
            break;
    }
    
    return NULL;
}

/* Main benchmark execution */
static void run_benchmark(struct benchmark_config *config) {
    pthread_t threads[MAX_THREADS];
    struct thread_context contexts[MAX_THREADS];
    pthread_barrier_t start_barrier;
    volatile int stop_flag = 0;
    
    printf("\n🚀 VexFS v2.0 Performance Benchmark Suite\n");
    printf("==========================================\n");
    printf("Mount Point: %s\n", config->mount_point);
    printf("Threads: %d\n", config->num_threads);
    printf("Test Duration: %d seconds\n", config->test_duration);
    printf("Warmup Duration: %d seconds\n", config->warmup_duration);
    printf("Dimensions: %d\n", config->dimensions);
    printf("Batch Size: %d\n", config->batch_size);
    printf("K Neighbors: %d\n", config->k_neighbors);
    printf("\n");
    
    /* Initialize barrier for thread synchronization */
    pthread_barrier_init(&start_barrier, NULL, config->num_threads);
    
    /* Create and start threads */
    for (int i = 0; i < config->num_threads; i++) {
        contexts[i].thread_id = i;
        contexts[i].mount_point = config->mount_point;
        contexts[i].test_duration = config->test_duration;
        contexts[i].warmup_duration = config->warmup_duration;
        contexts[i].dimensions = config->dimensions;
        contexts[i].batch_size = config->batch_size;
        contexts[i].k_neighbors = config->k_neighbors;
        contexts[i].start_barrier = &start_barrier;
        contexts[i].stop_flag = &stop_flag;
        
        pthread_create(&threads[i], NULL, benchmark_worker, &contexts[i]);
    }
    
    /* Wait for all threads to complete */
    for (int i = 0; i < config->num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    /* Aggregate results */
    struct performance_metrics metadata_metrics = {0};
    struct performance_metrics search_metrics = {0};
    struct performance_metrics batch_metrics = {0};
    
    metadata_metrics.min_latency_ns = UINT64_MAX;
    search_metrics.min_latency_ns = UINT64_MAX;
    batch_metrics.min_latency_ns = UINT64_MAX;
    
    for (int i = 0; i < config->num_threads; i++) {
        struct performance_metrics *m;
        
        switch (i % 3) {
            case 0:
                m = &metadata_metrics;
                break;
            case 1:
                m = &search_metrics;
                break;
            case 2:
                m = &batch_metrics;
                break;
        }
        
        m->operations_completed += contexts[i].metrics.operations_completed;
        m->total_latency_ns += contexts[i].metrics.total_latency_ns;
        m->errors += contexts[i].metrics.errors;
        m->throughput_ops_per_sec += contexts[i].metrics.throughput_ops_per_sec;
        
        if (contexts[i].metrics.min_latency_ns < m->min_latency_ns) {
            m->min_latency_ns = contexts[i].metrics.min_latency_ns;
        }
        if (contexts[i].metrics.max_latency_ns > m->max_latency_ns) {
            m->max_latency_ns = contexts[i].metrics.max_latency_ns;
        }
    }
    
    /* Calculate averages */
    if (metadata_metrics.operations_completed > 0) {
        metadata_metrics.avg_latency_ms = ns_to_ms(metadata_metrics.total_latency_ns / metadata_metrics.operations_completed);
    }
    if (search_metrics.operations_completed > 0) {
        search_metrics.avg_latency_ms = ns_to_ms(search_metrics.total_latency_ns / search_metrics.operations_completed);
    }
    if (batch_metrics.operations_completed > 0) {
        batch_metrics.avg_latency_ms = ns_to_ms(batch_metrics.total_latency_ns / batch_metrics.operations_completed);
    }
    
    /* Print results */
    print_performance_metrics("Vector Metadata Operations", &metadata_metrics);
    print_performance_metrics("Vector Search Operations", &search_metrics);
    print_performance_metrics("Batch Insert Operations", &batch_metrics);
    
    /* Overall summary */
    uint64_t total_ops = metadata_metrics.operations_completed + 
                        search_metrics.operations_completed + 
                        batch_metrics.operations_completed;
    double total_throughput = metadata_metrics.throughput_ops_per_sec + 
                             search_metrics.throughput_ops_per_sec + 
                             batch_metrics.throughput_ops_per_sec;
    
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
    
    pthread_barrier_destroy(&start_barrier);
}

/* Usage information */
static void print_usage(const char *program_name) {
    printf("Usage: %s [options]\n", program_name);
    printf("Options:\n");
    printf("  -m <mount_point>  VexFS v2.0 mount point (default: /tmp/vexfs_v2_316_test)\n");
    printf("  -t <threads>      Number of threads (default: 4)\n");
    printf("  -d <duration>     Test duration in seconds (default: 10)\n");
    printf("  -w <warmup>       Warmup duration in seconds (default: 2)\n");
    printf("  -D <dimensions>   Vector dimensions (default: 128)\n");
    printf("  -b <batch_size>   Batch size for insert operations (default: 100)\n");
    printf("  -k <neighbors>    K neighbors for search (default: 10)\n");
    printf("  -v                Verbose output\n");
    printf("  -h                Show this help\n");
}

int main(int argc, char *argv[]) {
    struct benchmark_config config = {
        .mount_point = "/tmp/vexfs_v2_316_test",
        .num_threads = 4,
        .test_duration = DEFAULT_TEST_DURATION,
        .warmup_duration = DEFAULT_WARMUP_DURATION,
        .dimensions = 128,
        .batch_size = 100,
        .k_neighbors = 10,
        .vector_count = 10000,
        .test_type = BENCH_MIXED_WORKLOAD,
        .verbose = 0
    };
    
    int opt;
    while ((opt = getopt(argc, argv, "m:t:d:w:D:b:k:vh")) != -1) {
        switch (opt) {
            case 'm':
                strncpy(config.mount_point, optarg, sizeof(config.mount_point) - 1);
                break;
            case 't':
                config.num_threads = atoi(optarg);
                if (config.num_threads > MAX_THREADS) {
                    config.num_threads = MAX_THREADS;
                }
                break;
            case 'd':
                config.test_duration = atoi(optarg);
                break;
            case 'w':
                config.warmup_duration = atoi(optarg);
                break;
            case 'D':
                config.dimensions = atoi(optarg);
                if (config.dimensions > MAX_DIMENSIONS) {
                    config.dimensions = MAX_DIMENSIONS;
                }
                break;
            case 'b':
                config.batch_size = atoi(optarg);
                if (config.batch_size > MAX_VECTORS_PER_BATCH) {
                    config.batch_size = MAX_VECTORS_PER_BATCH;
                }
                break;
            case 'k':
                config.k_neighbors = atoi(optarg);
                break;
            case 'v':
                config.verbose = 1;
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
    if (stat(config.mount_point, &st) != 0) {
        printf("Error: Mount point %s does not exist\n", config.mount_point);
        return 1;
    }
    
    /* Run the benchmark */
    run_benchmark(&config);
    
    return 0;
}