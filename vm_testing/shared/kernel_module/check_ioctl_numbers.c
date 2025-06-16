#include <stdio.h>
#include <sys/ioctl.h>
#include <stdint.h>

/* Test IOCTL definitions */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_file_info)
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
    uint32_t dimensions;
    float *query_vector;
    uint32_t k;
    uint64_t *result_ids;
    float *result_distances;
};

struct vexfs_batch_insert_request {
    uint32_t vector_count;
    uint32_t dimensions;
    float *vectors;
    uint64_t *vector_ids;
};

int main() {
    printf("VexFS v2.0 IOCTL Command Numbers:\n");
    printf("=================================\n");
    printf("VEXFS_IOC_SET_VECTOR_META:  0x%08lx\n", VEXFS_IOC_SET_VECTOR_META);
    printf("VEXFS_IOC_GET_VECTOR_META:  0x%08lx\n", VEXFS_IOC_GET_VECTOR_META);
    printf("VEXFS_IOC_VECTOR_SEARCH:    0x%08lx\n", VEXFS_IOC_VECTOR_SEARCH);
    printf("VEXFS_IOC_BATCH_INSERT:     0x%08lx\n", VEXFS_IOC_BATCH_INSERT);
    printf("\nStruct sizes:\n");
    printf("vexfs_vector_file_info:     %zu bytes\n", sizeof(struct vexfs_vector_file_info));
    printf("vexfs_vector_search_request: %zu bytes\n", sizeof(struct vexfs_vector_search_request));
    printf("vexfs_batch_insert_request: %zu bytes\n", sizeof(struct vexfs_batch_insert_request));
    
    return 0;
}