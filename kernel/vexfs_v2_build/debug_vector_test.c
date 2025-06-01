#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>
#include <errno.h>
#include <string.h>

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
    printf("🔧 VexFS v2.0 DEBUG Vector Operations Test\n");
    printf("==========================================\n");
    
    // Open the actual file, not the directory
    int fd = open("/tmp/vexfs_test/vector_test_file", O_RDWR);
    if (fd < 0) {
        perror("Failed to open VexFS test file");
        return 1;
    }
    
    printf("✅ Successfully opened VexFS test file\n");
    
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
    
    printf("🔍 Setting vector metadata (dimensions=%u)...\n", meta.dimensions);
    if (ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta) == 0) {
        printf("✅ Vector metadata set successfully\n");
    } else {
        printf("❌ Failed to set vector metadata: %s\n", strerror(errno));
        close(fd);
        return 1;
    }
    
    // Verify metadata was set by reading it back
    struct vexfs_vector_file_info read_meta = {0};
    printf("🔍 Reading back vector metadata...\n");
    if (ioctl(fd, VEXFS_IOC_GET_VECTOR_META, &read_meta) == 0) {
        printf("✅ Vector metadata read successfully:\n");
        printf("   dimensions: %u\n", read_meta.dimensions);
        printf("   element_type: %u\n", read_meta.element_type);
        printf("   vector_count: %u\n", read_meta.vector_count);
        printf("   storage_format: %u\n", read_meta.storage_format);
    } else {
        printf("❌ Failed to read vector metadata: %s\n", strerror(errno));
    }
    
    // Test batch insert with correct command number
    float vectors[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0};
    uint64_t ids[] = {1, 2};
    
    struct vexfs_batch_insert_request req = {
        .vector_count = 2,
        .dimensions = 4,
        .vectors = vectors,
        .vector_ids = ids
    };
    
    printf("🔍 Testing batch insert:\n");
    printf("   vector_count: %u\n", req.vector_count);
    printf("   dimensions: %u\n", req.dimensions);
    printf("   vectors pointer: %p\n", req.vectors);
    printf("   vector_ids pointer: %p\n", req.vector_ids);
    printf("   IOCTL command: 0x%lx\n", VEXFS_IOC_BATCH_INSERT);
    
    if (ioctl(fd, VEXFS_IOC_BATCH_INSERT, &req) == 0) {
        printf("✅ Batch insert successful (2 vectors)\n");
    } else {
        printf("❌ Failed to batch insert vectors: %s (errno: %d)\n", strerror(errno), errno);
        
        // Print detailed error analysis
        switch (errno) {
        case ENOTTY:
            printf("   → Device does not support this ioctl\n");
            break;
        case EINVAL:
            printf("   → Invalid argument (likely dimension mismatch)\n");
            break;
        case EFAULT:
            printf("   → Bad address (copy_from_user failed)\n");
            break;
        case ENOMEM:
            printf("   → Out of memory\n");
            break;
        default:
            printf("   → Unknown error\n");
            break;
        }
    }
    
    close(fd);
    printf("\n🔍 Test completed! Check dmesg for detailed logs.\n");
    return 0;
}