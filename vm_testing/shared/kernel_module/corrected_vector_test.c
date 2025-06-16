#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>

/* Correct IOCTL definitions from kernel module */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)
#define VEXFS_IOC_GET_VECTOR_META    _IOR(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_file_info)
#define VEXFS_IOC_BATCH_INSERT       _IOW(VEXFS_IOC_MAGIC, 4, struct vexfs_batch_insert_request)

/* Correct structures from kernel module */
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

struct vexfs_batch_insert_request {
    uint32_t vector_count;
    uint32_t dimensions;
    float *vectors;
    uint64_t *vector_ids;
};

int main() {
    printf("🔧 VexFS v2.0 CORRECTED Vector Operations Test\n");
    printf("==============================================\n");
    
    int fd = open("/tmp/vexfs_test", O_RDONLY);
    if (fd < 0) {
        perror("Failed to open VexFS mount point");
        return 1;
    }
    
    // Test vector metadata with correct structure
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
    
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) == 0) {
        printf("✅ Vector metadata set successfully (4 dimensions)\n");
    } else {
        perror("❌ Failed to set vector metadata");
    }
    
    // Test batch insert with correct command number
    float vectors[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0};
    uint64_t ids[] = {1, 2};
    
    struct vexfs_batch_insert_request req = {2, 4, vectors, ids};
    if (ioctl(fd, VEXFS_IOC_BATCH_INSERT, &req) == 0) {
        printf("✅ Batch insert successful (2 vectors)\n");
    } else {
        perror("❌ Failed to batch insert vectors");
    }
    
    close(fd);
    printf("\n🔍 Test completed! Check dmesg for detailed logs.\n");
    return 0;
}
