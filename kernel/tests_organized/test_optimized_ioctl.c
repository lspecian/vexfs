#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <errno.h>
#include <string.h>
#include <stdint.h>

/* VexFS v2.0 ioctl definitions - MUST MATCH KERNEL */
#define VEXFS_IOC_MAGIC 'V'

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

#define VEXFS_IOC_SET_VECTOR_META    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_file_info)

int main(int argc, char *argv[]) {
    if (argc != 2) {
        printf("Usage: %s <file_path>\n", argv[0]);
        return 1;
    }
    
    printf("=== Testing Optimized VexFS v2.0 ioctl ===\n");
    printf("File: %s\n", argv[1]);
    
    /* Create/open file */
    int fd = open(argv[1], O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        printf("❌ Failed to open file: %s (errno: %d)\n", strerror(errno), errno);
        return 1;
    }
    printf("✅ File opened successfully\n");
    
    /* Test SET_VECTOR_META */
    struct vexfs_vector_file_info info = {
        .dimensions = 4,
        .element_type = 1,
        .vector_count = 0,
        .storage_format = 0,
        .data_offset = 0,
        .index_offset = 0,
        .compression_type = 0,
        .alignment_bytes = 32
    };
    
    printf("Structure size: %zu bytes\n", sizeof(info));
    printf("Testing SET_VECTOR_META...\n");
    
    int result = ioctl(fd, VEXFS_IOC_SET_VECTOR_META, &info);
    if (result == 0) {
        printf("✅ SET_VECTOR_META SUCCESS!\n");
    } else {
        printf("❌ SET_VECTOR_META FAILED: %s (errno: %d)\n", strerror(errno), errno);
    }
    
    close(fd);
    return result;
}