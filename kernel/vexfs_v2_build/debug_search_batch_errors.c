#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <stdint.h>

/* VexFS v2.0 ioctl definitions */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_IOC_VECTOR_SEARCH      _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search_request)
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)

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

int main() {
    const char *test_file = "/tmp/vexfs_v2_316_test/debug_search_batch";
    
    printf("=== Debugging VexFS v2.0 Search & Batch Errors ===\n");
    
    // Create test file
    int fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("ERROR: Failed to create test file: %s\n", strerror(errno));
        return 1;
    }
    
    printf("✅ File created successfully: %s\n", test_file);
    
    // Set metadata first
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
    
    printf("\n--- Testing SET_VECTOR_META ---\n");
    int ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
    if (ret < 0) {
        printf("❌ SET_VECTOR_META FAILED: %s (errno: %d)\n", strerror(errno), errno);
        close(fd);
        unlink(test_file);
        return 1;
    } else {
        printf("✅ SET_VECTOR_META SUCCESS: returned %d\n", ret);
    }
    
    // Test Vector Search
    printf("\n--- Testing VECTOR_SEARCH ---\n");
    float query_vector[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    float results[10];
    uint64_t result_ids[10];
    
    struct vexfs_vector_search_request search_req = {
        .query_vector = query_vector,
        .dimensions = 4,
        .k = 5,
        .search_type = 0,
        .results = results,
        .result_ids = result_ids,
        .result_count = 0
    };
    
    printf("  query_vector: [%.1f, %.1f, %.1f, %.1f]\n", 
           query_vector[0], query_vector[1], query_vector[2], query_vector[3]);
    printf("  dimensions: %u\n", search_req.dimensions);
    printf("  k: %u\n", search_req.k);
    printf("  ioctl command: 0x%lx\n", VEXFS_IOC_VECTOR_SEARCH);
    printf("  struct size: %zu bytes\n", sizeof(struct vexfs_vector_search_request));
    
    ret = ioctl(fd, VEXFS_IOC_VECTOR_SEARCH, &search_req);
    if (ret < 0) {
        printf("❌ VECTOR_SEARCH FAILED: %s (errno: %d)\n", strerror(errno), errno);
        
        switch (errno) {
        case EINVAL:
            printf("   → Invalid argument (dimension mismatch or invalid parameters)\n");
            break;
        case ENODATA:
            printf("   → No data available (empty vector file)\n");
            break;
        case EFAULT:
            printf("   → Bad address (memory access issue)\n");
            break;
        case ENOTTY:
            printf("   → Device does not support this ioctl\n");
            break;
        default:
            printf("   → Unknown error\n");
            break;
        }
    } else {
        printf("✅ VECTOR_SEARCH SUCCESS: returned %d\n", ret);
        printf("  result_count: %u\n", search_req.result_count);
    }
    
    // Test Batch Insert
    printf("\n--- Testing BATCH_INSERT ---\n");
    const int batch_size = 5;
    float vectors[batch_size * 4];
    uint64_t vector_ids[batch_size];
    
    // Initialize test vectors
    for (int i = 0; i < batch_size * 4; i++) {
        vectors[i] = (float)(i % 10);
    }
    
    // Initialize vector IDs
    for (int i = 0; i < batch_size; i++) {
        vector_ids[i] = 100 + i;
    }
    
    struct vexfs_batch_insert_request batch_req = {
        .vectors = vectors,
        .vector_count = batch_size,
        .dimensions = 4,
        .vector_ids = vector_ids,
        .flags = 0
    };
    
    printf("  dimensions: %u\n", batch_req.dimensions);
    printf("  vector_count: %u\n", batch_req.vector_count);
    printf("  vector_ids[0]: %lu\n", batch_req.vector_ids[0]);
    printf("  ioctl command: 0x%lx\n", VEXFS_IOC_BATCH_INSERT);
    printf("  struct size: %zu bytes\n", sizeof(struct vexfs_batch_insert_request));
    
    ret = ioctl(fd, VEXFS_IOC_BATCH_INSERT, &batch_req);
    if (ret < 0) {
        printf("❌ BATCH_INSERT FAILED: %s (errno: %d)\n", strerror(errno), errno);
        
        switch (errno) {
        case EINVAL:
            printf("   → Invalid argument (dimension mismatch or invalid parameters)\n");
            break;
        case ENOMEM:
            printf("   → Out of memory\n");
            break;
        case EFAULT:
            printf("   → Bad address (memory access issue)\n");
            break;
        case ENOTTY:
            printf("   → Device does not support this ioctl\n");
            break;
        default:
            printf("   → Unknown error\n");
            break;
        }
    } else {
        printf("✅ BATCH_INSERT SUCCESS: returned %d\n", ret);
    }
    
    close(fd);
    unlink(test_file);
    
    return 0;
}