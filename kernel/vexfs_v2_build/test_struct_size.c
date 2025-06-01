#include <stdio.h>
#include <stdint.h>

/* Structure definitions - MUST MATCH KERNEL EXACTLY */
struct vexfs_vector_file_info {
    uint32_t dimensions;
    uint32_t element_type;
    uint32_t vector_count;
    uint32_t storage_format;
    uint64_t data_offset;
    uint64_t index_offset;
    uint32_t compression_type;
    uint32_t alignment_bytes;
} __attribute__((packed));

int main() {
    printf("Structure size: %zu bytes\n", sizeof(struct vexfs_vector_file_info));
    printf("Field offsets:\n");
    printf("  dimensions: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, dimensions));
    printf("  element_type: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, element_type));
    printf("  vector_count: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, vector_count));
    printf("  storage_format: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, storage_format));
    printf("  data_offset: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, data_offset));
    printf("  index_offset: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, index_offset));
    printf("  compression_type: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, compression_type));
    printf("  alignment_bytes: %zu\n", __builtin_offsetof(struct vexfs_vector_file_info, alignment_bytes));
    return 0;
}