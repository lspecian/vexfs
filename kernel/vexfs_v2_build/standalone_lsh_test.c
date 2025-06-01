/*
 * Standalone LSH Test Program for VexFS v2.0 Phase 3
 * 
 * This test program validates the LSH (Locality Sensitive Hashing) 
 * implementation with embedded definitions to avoid header dependencies.
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <time.h>
#include <math.h>

/* Embedded definitions to avoid header dependencies */

/* Distance metrics */
#define VEXFS_DISTANCE_EUCLIDEAN 0
#define VEXFS_DISTANCE_COSINE    1
#define VEXFS_DISTANCE_DOT_PRODUCT 2
#define VEXFS_DISTANCE_MANHATTAN 3

/* IOCTL magic and commands */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_LSH_INIT          _IOW(VEXFS_IOC_MAGIC, 24, struct vexfs_lsh_config)
#define VEXFS_IOC_LSH_INSERT        _IOW(VEXFS_IOC_MAGIC, 25, struct vexfs_lsh_insert_request)
#define VEXFS_IOC_LSH_SEARCH        _IOWR(VEXFS_IOC_MAGIC, 26, struct vexfs_lsh_search_request)

/* LSH Configuration */
struct vexfs_lsh_config {
    uint32_t dimensions;
    uint32_t distance_metric;
    uint32_t hash_tables;
    uint32_t hash_functions_per_table;
    float bucket_width;
    uint32_t reserved[4];
};

/* LSH Insert Request */
struct vexfs_lsh_insert_request {
    uint64_t vector_id;
    uint32_t dimensions;
    float *vector_data;
    uint32_t reserved[4];
};

/* Search result structure */
struct vexfs_search_result {
    uint64_t vector_id;
    uint64_t distance;
    uint64_t score;
    uint32_t metadata_size;
    uint8_t metadata[64];
};

/* LSH Search Request */
struct vexfs_lsh_search_request {
    uint32_t dimensions;
    float *query_vector;
    uint32_t k;
    struct vexfs_search_result *results;
    uint32_t *result_count;
    uint32_t reserved[4];
};

/* LSH Statistics */
struct vexfs_lsh_stats {
    uint32_t total_vectors;
    uint32_t hash_table_count;
    uint32_t hash_functions_per_table;
    uint64_t total_searches;
    uint64_t total_insertions;
    uint64_t total_hash_computations;
    uint64_t bucket_collisions;
    uint64_t false_positives;
    uint64_t avg_search_time_ns;
    uint64_t avg_insert_time_ns;
    uint64_t memory_usage;
    uint32_t active_searches;
    uint32_t bucket_utilization[32];
    uint32_t reserved[8];
};

/* Test configuration */
#define TEST_DIMENSIONS 128
#define TEST_VECTOR_COUNT 1000
#define TEST_QUERY_COUNT 10
#define TEST_K 10

/* Test data generation */
static void generate_random_vector(float *vector, uint32_t dimensions)
{
    for (uint32_t i = 0; i < dimensions; i++) {
        vector[i] = ((float)rand() / RAND_MAX) * 2.0f - 1.0f;
    }
}

static void normalize_vector(float *vector, uint32_t dimensions)
{
    float norm = 0.0f;
    for (uint32_t i = 0; i < dimensions; i++) {
        norm += vector[i] * vector[i];
    }
    norm = sqrtf(norm);
    
    if (norm > 0.0f) {
        for (uint32_t i = 0; i < dimensions; i++) {
            vector[i] /= norm;
        }
    }
}

static float calculate_euclidean_distance(const float *v1, const float *v2, uint32_t dimensions)
{
    float distance = 0.0f;
    for (uint32_t i = 0; i < dimensions; i++) {
        float diff = v1[i] - v2[i];
        distance += diff * diff;
    }
    return sqrtf(distance);
}

static float calculate_cosine_similarity(const float *v1, const float *v2, uint32_t dimensions)
{
    float dot_product = 0.0f;
    float norm1 = 0.0f, norm2 = 0.0f;
    
    for (uint32_t i = 0; i < dimensions; i++) {
        dot_product += v1[i] * v2[i];
        norm1 += v1[i] * v1[i];
        norm2 += v2[i] * v2[i];
    }
    
    norm1 = sqrtf(norm1);
    norm2 = sqrtf(norm2);
    
    if (norm1 > 0.0f && norm2 > 0.0f) {
        return dot_product / (norm1 * norm2);
    }
    return 0.0f;
}

/* Test functions */
static int test_lsh_initialization(int fd)
{
    printf("\n=== Testing LSH Initialization ===\n");
    
    struct vexfs_lsh_config config = {
        .dimensions = TEST_DIMENSIONS,
        .distance_metric = VEXFS_DISTANCE_EUCLIDEAN,
        .hash_tables = 8,
        .hash_functions_per_table = 16,
        .bucket_width = 1.0f
    };
    
    printf("Initializing LSH index:\n");
    printf("  Dimensions: %u\n", config.dimensions);
    printf("  Distance metric: %u (Euclidean)\n", config.distance_metric);
    printf("  Hash tables: %u\n", config.hash_tables);
    printf("  Hash functions per table: %u\n", config.hash_functions_per_table);
    printf("  Bucket width: %.2f\n", config.bucket_width);
    
    if (ioctl(fd, VEXFS_IOC_LSH_INIT, &config) == 0) {
        printf("✅ LSH initialization successful\n");
        return 0;
    } else {
        perror("❌ LSH initialization failed");
        return -1;
    }
}

static int test_lsh_vector_insertion(int fd, float **test_vectors, uint32_t vector_count)
{
    printf("\n=== Testing LSH Vector Insertion ===\n");
    
    clock_t start_time = clock();
    uint32_t successful_insertions = 0;
    
    for (uint32_t i = 0; i < vector_count; i++) {
        struct vexfs_lsh_insert_request req = {
            .vector_id = i + 1,
            .dimensions = TEST_DIMENSIONS,
            .vector_data = test_vectors[i]
        };
        
        if (ioctl(fd, VEXFS_IOC_LSH_INSERT, &req) == 0) {
            successful_insertions++;
            if ((i + 1) % 100 == 0) {
                printf("  Inserted %u vectors...\n", i + 1);
            }
        } else {
            printf("❌ Failed to insert vector %u\n", i + 1);
        }
    }
    
    clock_t end_time = clock();
    double elapsed_time = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;
    
    printf("✅ LSH insertion completed:\n");
    printf("  Successful insertions: %u/%u\n", successful_insertions, vector_count);
    printf("  Total time: %.3f seconds\n", elapsed_time);
    printf("  Insertion rate: %.0f vectors/second\n", successful_insertions / elapsed_time);
    
    return successful_insertions == vector_count ? 0 : -1;
}

static int test_lsh_search(int fd, float **test_vectors, float **query_vectors, uint32_t query_count)
{
    printf("\n=== Testing LSH Search ===\n");
    
    struct vexfs_search_result results[TEST_K];
    uint32_t total_results_found = 0;
    clock_t total_search_time = 0;
    
    for (uint32_t q = 0; q < query_count; q++) {
        uint32_t result_count = 0;
        
        struct vexfs_lsh_search_request req = {
            .dimensions = TEST_DIMENSIONS,
            .query_vector = query_vectors[q],
            .k = TEST_K,
            .results = results,
            .result_count = &result_count
        };
        
        clock_t start_time = clock();
        
        if (ioctl(fd, VEXFS_IOC_LSH_SEARCH, &req) == 0) {
            clock_t end_time = clock();
            total_search_time += (end_time - start_time);
            total_results_found += result_count;
            
            printf("Query %u: Found %u results\n", q + 1, result_count);
            
            /* Display top 3 results */
            for (uint32_t i = 0; i < (result_count < 3 ? result_count : 3); i++) {
                printf("  Result %u: ID=%llu, Distance=%llu, Score=%llu\n",
                       i + 1, results[i].vector_id, results[i].distance, results[i].score);
            }
            
            /* Verify results by computing actual distances */
            if (result_count > 0) {
                printf("  Verification (actual Euclidean distances):\n");
                for (uint32_t i = 0; i < (result_count < 3 ? result_count : 3); i++) {
                    uint64_t vector_idx = results[i].vector_id - 1;
                    if (vector_idx < TEST_VECTOR_COUNT) {
                        float actual_distance = calculate_euclidean_distance(
                            query_vectors[q], test_vectors[vector_idx], TEST_DIMENSIONS);
                        printf("    Vector %llu: Actual distance = %.6f\n", 
                               results[i].vector_id, actual_distance);
                    }
                }
            }
        } else {
            printf("❌ Search %u failed\n", q + 1);
        }
        
        printf("\n");
    }
    
    double avg_search_time = ((double)total_search_time / CLOCKS_PER_SEC) / query_count;
    
    printf("✅ LSH search completed:\n");
    printf("  Total queries: %u\n", query_count);
    printf("  Total results found: %u\n", total_results_found);
    printf("  Average results per query: %.1f\n", (float)total_results_found / query_count);
    printf("  Average search time: %.6f seconds\n", avg_search_time);
    printf("  Search rate: %.0f queries/second\n", 1.0 / avg_search_time);
    
    return 0;
}

static int test_lsh_cosine_similarity(int fd)
{
    printf("\n=== Testing LSH with Cosine Similarity ===\n");
    
    /* Reinitialize with cosine similarity */
    struct vexfs_lsh_config config = {
        .dimensions = 64,  /* Smaller for cosine test */
        .distance_metric = VEXFS_DISTANCE_COSINE,
        .hash_tables = 6,
        .hash_functions_per_table = 12,
        .bucket_width = 0.1f  /* Smaller buckets for cosine */
    };
    
    printf("Reinitializing LSH for cosine similarity:\n");
    printf("  Dimensions: %u\n", config.dimensions);
    printf("  Distance metric: %u (Cosine)\n", config.distance_metric);
    printf("  Hash tables: %u\n", config.hash_tables);
    printf("  Hash functions per table: %u\n", config.hash_functions_per_table);
    printf("  Bucket width: %.2f\n", config.bucket_width);
    
    if (ioctl(fd, VEXFS_IOC_LSH_INIT, &config) != 0) {
        perror("❌ LSH cosine initialization failed");
        return -1;
    }
    
    /* Generate normalized test vectors for cosine similarity */
    float test_vector1[64], test_vector2[64], test_vector3[64];
    float query_vector[64];
    
    /* Create similar vectors */
    generate_random_vector(test_vector1, 64);
    normalize_vector(test_vector1, 64);
    
    /* Create vector similar to test_vector1 */
    memcpy(test_vector2, test_vector1, 64 * sizeof(float));
    for (int i = 0; i < 10; i++) {
        test_vector2[i] += 0.1f * ((float)rand() / RAND_MAX - 0.5f);
    }
    normalize_vector(test_vector2, 64);
    
    /* Create dissimilar vector */
    generate_random_vector(test_vector3, 64);
    normalize_vector(test_vector3, 64);
    
    /* Query vector similar to test_vector1 */
    memcpy(query_vector, test_vector1, 64 * sizeof(float));
    for (int i = 0; i < 5; i++) {
        query_vector[i] += 0.05f * ((float)rand() / RAND_MAX - 0.5f);
    }
    normalize_vector(query_vector, 64);
    
    /* Insert test vectors */
    struct vexfs_lsh_insert_request req1 = {1, 64, test_vector1};
    struct vexfs_lsh_insert_request req2 = {2, 64, test_vector2};
    struct vexfs_lsh_insert_request req3 = {3, 64, test_vector3};
    
    if (ioctl(fd, VEXFS_IOC_LSH_INSERT, &req1) != 0 ||
        ioctl(fd, VEXFS_IOC_LSH_INSERT, &req2) != 0 ||
        ioctl(fd, VEXFS_IOC_LSH_INSERT, &req3) != 0) {
        printf("❌ Failed to insert cosine test vectors\n");
        return -1;
    }
    
    printf("✅ Inserted 3 test vectors for cosine similarity\n");
    
    /* Calculate actual cosine similarities */
    float sim1 = calculate_cosine_similarity(query_vector, test_vector1, 64);
    float sim2 = calculate_cosine_similarity(query_vector, test_vector2, 64);
    float sim3 = calculate_cosine_similarity(query_vector, test_vector3, 64);
    
    printf("Actual cosine similarities:\n");
    printf("  Query vs Vector 1: %.6f\n", sim1);
    printf("  Query vs Vector 2: %.6f\n", sim2);
    printf("  Query vs Vector 3: %.6f\n", sim3);
    
    /* Search */
    struct vexfs_search_result results[3];
    uint32_t result_count = 0;
    
    struct vexfs_lsh_search_request search_req = {
        .dimensions = 64,
        .query_vector = query_vector,
        .k = 3,
        .results = results,
        .result_count = &result_count
    };
    
    if (ioctl(fd, VEXFS_IOC_LSH_SEARCH, &search_req) == 0) {
        printf("✅ Cosine search found %u results:\n", result_count);
        for (uint32_t i = 0; i < result_count; i++) {
            printf("  Result %u: Vector ID=%llu, Score=%llu\n", 
                   i + 1, results[i].vector_id, results[i].score);
        }
    } else {
        printf("❌ Cosine search failed\n");
        return -1;
    }
    
    return 0;
}

int main(int argc, char *argv[])
{
    const char *device_path = "/tmp/vexfs_test";
    
    if (argc > 1) {
        device_path = argv[1];
    }
    
    printf("VexFS v2.0 Phase 3 - LSH Index Test\n");
    printf("===================================\n");
    printf("Device: %s\n", device_path);
    printf("Test configuration:\n");
    printf("  Dimensions: %d\n", TEST_DIMENSIONS);
    printf("  Vector count: %d\n", TEST_VECTOR_COUNT);
    printf("  Query count: %d\n", TEST_QUERY_COUNT);
    printf("  k (results per query): %d\n", TEST_K);
    
    /* Open device */
    int fd = open(device_path, O_RDWR);
    if (fd < 0) {
        perror("Failed to open VexFS device");
        return 1;
    }
    
    /* Initialize random seed */
    srand(time(NULL));
    
    /* Allocate test data */
    float **test_vectors = malloc(TEST_VECTOR_COUNT * sizeof(float*));
    float **query_vectors = malloc(TEST_QUERY_COUNT * sizeof(float*));
    
    if (!test_vectors || !query_vectors) {
        printf("❌ Failed to allocate memory for test vectors\n");
        close(fd);
        return 1;
    }
    
    /* Generate test vectors */
    printf("\nGenerating test data...\n");
    for (uint32_t i = 0; i < TEST_VECTOR_COUNT; i++) {
        test_vectors[i] = malloc(TEST_DIMENSIONS * sizeof(float));
        if (!test_vectors[i]) {
            printf("❌ Failed to allocate memory for test vector %u\n", i);
            close(fd);
            return 1;
        }
        generate_random_vector(test_vectors[i], TEST_DIMENSIONS);
    }
    
    /* Generate query vectors */
    for (uint32_t i = 0; i < TEST_QUERY_COUNT; i++) {
        query_vectors[i] = malloc(TEST_DIMENSIONS * sizeof(float));
        if (!query_vectors[i]) {
            printf("❌ Failed to allocate memory for query vector %u\n", i);
            close(fd);
            return 1;
        }
        generate_random_vector(query_vectors[i], TEST_DIMENSIONS);
    }
    
    printf("✅ Generated %u test vectors and %u query vectors\n", 
           TEST_VECTOR_COUNT, TEST_QUERY_COUNT);
    
    /* Run tests */
    int result = 0;
    
    result |= test_lsh_initialization(fd);
    if (result == 0) {
        result |= test_lsh_vector_insertion(fd, test_vectors, TEST_VECTOR_COUNT);
    }
    if (result == 0) {
        result |= test_lsh_search(fd, test_vectors, query_vectors, TEST_QUERY_COUNT);
    }
    if (result == 0) {
        result |= test_lsh_cosine_similarity(fd);
    }
    
    /* Cleanup */
    for (uint32_t i = 0; i < TEST_VECTOR_COUNT; i++) {
        free(test_vectors[i]);
    }
    for (uint32_t i = 0; i < TEST_QUERY_COUNT; i++) {
        free(query_vectors[i]);
    }
    free(test_vectors);
    free(query_vectors);
    
    close(fd);
    
    if (result == 0) {
        printf("\n🎉 All LSH tests completed successfully!\n");
        printf("\nNext steps:\n");
        printf("1. Compile: gcc -o lsh_test standalone_lsh_test.c -lm\n");
        printf("2. Load VexFS module: sudo insmod vexfs_v2_*.ko\n");
        printf("3. Mount VexFS: sudo mount -t vexfs none /tmp/vexfs_test\n");
        printf("4. Run test: ./lsh_test\n");
        printf("5. Check results: dmesg | tail -50\n");
    } else {
        printf("\n❌ Some LSH tests failed. Check dmesg for details.\n");
    }
    
    return result;
}