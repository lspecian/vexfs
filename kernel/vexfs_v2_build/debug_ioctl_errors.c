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

int main() {
    const char *test_file = "/tmp/vexfs_v2_316_test/debug_ioctl";
    
    printf("=== Debugging VexFS v2.0 ioctl Errors ===\n");
    
    // Create test file
    int fd = open(test_file, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("ERROR: Failed to create test file: %s\n", strerror(errno));
        return 1;
    }
    
    printf("✅ File created successfully: %s\n", test_file);
    
    // Test ioctl
    struct vexfs_vector_file_info meta = {
        .dimensions = 128,
        .element_type = 0,
        .vector_count = 1000,
        .storage_format = 0,
        .data_offset = 0,
        .index_offset = 128 * 1000 * sizeof(float),
        .compression_type = 0,
        .alignment_bytes = 32
    };
    
    printf("Attempting ioctl VEXFS_IOC_SET_VECTOR_META...\n");
    printf("  ioctl command: 0x%lx\n", VEXFS_IOC_SET_VECTOR_META);
    printf("  struct size: %zu bytes\n", sizeof(struct vexfs_vector_file_info));
    printf("  dimensions: %u\n", meta.dimensions);
    printf("  vector_count: %u\n", meta.vector_count);
    
    int ret = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &meta);
    if (ret < 0) {
        printf("❌ ioctl FAILED: %s (errno: %d)\n", strerror(errno), errno);
        
        // Try to identify the specific error
        switch (errno) {
        case ENOTTY:
            printf("   → Device does not support this ioctl (not a VexFS file?)\n");
            break;
        case EINVAL:
            printf("   → Invalid argument (structure mismatch?)\n");
            break;
        case EACCES:
            printf("   → Permission denied\n");
            break;
        case EFAULT:
            printf("   → Bad address (memory access issue)\n");
            break;
        default:
            printf("   → Unknown error\n");
            break;
        }
    } else {
        printf("✅ ioctl SUCCESS: returned %d\n", ret);
    }
    
    close(fd);
    unlink(test_file);
    
    return 0;
}