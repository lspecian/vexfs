#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>

#define VEXFS_IOCTL_SET_VECTOR_META    _IOW('V', 1, struct vexfs_vector_metadata)
#define VEXFS_IOCTL_BATCH_INSERT       _IOW('V', 3, struct vexfs_batch_insert_request)

struct vexfs_vector_metadata {
    uint32_t dimensions;
    uint32_t vector_count;
    uint32_t distance_metric;
    uint32_t reserved;
};

struct vexfs_batch_insert_request {
    uint32_t vector_count;
    uint32_t dimensions;
    float *vectors;
    uint64_t *vector_ids;
};

int main() {
    printf("VexFS v2.0 Block Device Vector Test\n");
    printf("===================================\n");
    
    int fd = open("/tmp/vexfs_block_test", O_RDONLY);
    if (fd < 0) {
        perror("Failed to open VexFS block device mount point");
        return 1;
    }
    
    // Test vector metadata
    struct vexfs_vector_metadata meta = {4, 0, 0, 0};
    if (ioctl(fd, VEXFS_IOCTL_SET_VECTOR_META, &meta) == 0) {
        printf("✅ Block device: Vector metadata set successfully (4 dimensions)\n");
    } else {
        perror("❌ Block device: Failed to set vector metadata");
    }
    
    // Test batch insert
    float vectors[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0};
    uint64_t ids[] = {1, 2};
    
    struct vexfs_batch_insert_request req = {2, 4, vectors, ids};
    if (ioctl(fd, VEXFS_IOCTL_BATCH_INSERT, &req) == 0) {
        printf("✅ Block device: Batch insert successful (2 vectors)\n");
    } else {
        perror("❌ Block device: Failed to batch insert vectors");
    }
    
    close(fd);
    printf("\nBlock device test completed!\n");
    return 0;
}
