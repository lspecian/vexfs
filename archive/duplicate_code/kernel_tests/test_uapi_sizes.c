#include <stdio.h>
#include <stdint.h>

/* Test the actual structure sizes */
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
    float    *query_vector;
    uint32_t  dimensions;
    uint32_t  k;
    uint32_t  search_type;
    float    *results;
    uint64_t *result_ids;
    uint32_t  result_count;
};

struct vexfs_batch_insert_request {
    float    *vectors;
    uint32_t  vector_count;
    uint32_t  dimensions;
    uint64_t *vector_ids;
    uint32_t  flags;
};

int main() {
    printf("Structure size analysis:\n");
    printf("========================\n");
    printf("vexfs_vector_file_info: %zu bytes\n", sizeof(struct vexfs_vector_file_info));
    printf("vexfs_vector_search_request: %zu bytes\n", sizeof(struct vexfs_vector_search_request));
    printf("vexfs_batch_insert_request: %zu bytes\n", sizeof(struct vexfs_batch_insert_request));
    printf("\nField analysis for vexfs_vector_file_info:\n");
    printf("- dimensions (uint32_t): 4 bytes\n");
    printf("- element_type (uint32_t): 4 bytes\n");
    printf("- vector_count (uint32_t): 4 bytes\n");
    printf("- storage_format (uint32_t): 4 bytes\n");
    printf("- data_offset (uint64_t): 8 bytes\n");
    printf("- index_offset (uint64_t): 8 bytes\n");
    printf("- compression_type (uint32_t): 4 bytes\n");
    printf("- alignment_bytes (uint32_t): 4 bytes\n");
    printf("Total expected: 40 bytes\n");
    
    printf("\nField analysis for vexfs_vector_search_request:\n");
    printf("- query_vector (pointer): %zu bytes\n", sizeof(void*));
    printf("- dimensions (uint32_t): 4 bytes\n");
    printf("- k (uint32_t): 4 bytes\n");
    printf("- search_type (uint32_t): 4 bytes\n");
    printf("- results (pointer): %zu bytes\n", sizeof(void*));
    printf("- result_ids (pointer): %zu bytes\n", sizeof(void*));
    printf("- result_count (uint32_t): 4 bytes\n");
    printf("Total expected: %zu bytes\n", 3 * sizeof(void*) + 16);
    
    printf("\nField analysis for vexfs_batch_insert_request:\n");
    printf("- vectors (pointer): %zu bytes\n", sizeof(void*));
    printf("- vector_count (uint32_t): 4 bytes\n");
    printf("- dimensions (uint32_t): 4 bytes\n");
    printf("- vector_ids (pointer): %zu bytes\n", sizeof(void*));
    printf("- flags (uint32_t): 4 bytes\n");
    printf("Total expected: %zu bytes\n", 2 * sizeof(void*) + 12);
    
    return 0;
}