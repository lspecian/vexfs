#include <stdint.h>
#include <stdio.h>
#include <sys/ioctl.h>

/* Exact structure definitions */
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
    float query_vector[128];
    uint32_t dimensions;
    uint32_t k;
    uint32_t search_type;
    uint32_t reserved;
};

struct vexfs_batch_insert_request {
    uint32_t vector_count;
    uint32_t dimensions;
    uint32_t element_type;
    uint32_t storage_format;
    uint64_t vectors_ptr;
    uint64_t metadata_ptr;
};

/* IOCTL command definitions */
#define VEXFS_IOCTL_MAGIC 'V'
#define VEXFS_SET_VECTOR_META    _IOW(VEXFS_IOCTL_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_GET_VECTOR_META    _IOR(VEXFS_IOCTL_MAGIC, 2, struct vexfs_vector_file_info)
#define VEXFS_VECTOR_SEARCH      _IOWR(VEXFS_IOCTL_MAGIC, 3, struct vexfs_vector_search_request)
#define VEXFS_BATCH_INSERT       _IOW(VEXFS_IOCTL_MAGIC, 4, struct vexfs_batch_insert_request)

int main() {
    printf("Structure sizes:\n");
    printf("  vexfs_vector_file_info: %zu bytes\n", sizeof(struct vexfs_vector_file_info));
    printf("  vexfs_vector_search_request: %zu bytes\n", sizeof(struct vexfs_vector_search_request));
    printf("  vexfs_batch_insert_request: %zu bytes\n", sizeof(struct vexfs_batch_insert_request));
    
    printf("\nIOCTL command numbers:\n");
    printf("  VEXFS_SET_VECTOR_META: 0x%08lx\n", (unsigned long)VEXFS_SET_VECTOR_META);
    printf("  VEXFS_GET_VECTOR_META: 0x%08lx\n", (unsigned long)VEXFS_GET_VECTOR_META);
    printf("  VEXFS_VECTOR_SEARCH: 0x%08lx\n", (unsigned long)VEXFS_VECTOR_SEARCH);
    printf("  VEXFS_BATCH_INSERT: 0x%08lx\n", (unsigned long)VEXFS_BATCH_INSERT);
    
    return 0;
}