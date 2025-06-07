#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

/* Use the new standard UAPI header */
#include "vexfs_v2_uapi.h"

int main() {
    printf("🔧 VexFS v2.0 FINAL CORRECTED Vector Operations Test\n");
    printf("====================================================\n");
    
    // Open the actual file, not the directory
    int fd = open("/tmp/vexfs_test/vector_test_file", O_RDWR);
    if (fd < 0) {
        perror("Failed to open VexFS test file");
        return 1;
    }
    
    printf("✅ Successfully opened VexFS test file\n");
    
    // Test vector metadata with correct structure using UAPI constants
    struct vexfs_vector_file_info meta = {
        .dimensions = 4,
        .element_type = VEXFS_VECTOR_FLOAT32,
        .vector_count = 0,
        .storage_format = VEXFS_STORAGE_DENSE,
        .data_offset = 0,
        .index_offset = 0,
        .compression_type = VEXFS_COMPRESS_NONE,
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
    
    // Test batch insert with CORRECTED structure layout using UAPI types
    float vectors[] = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0};
    __u64 ids[] = {1, 2};
    
    struct vexfs_batch_insert_request req = {
        .vectors = vectors,      // First
        .vector_count = 2,       // Second
        .dimensions = 4,         // Third
        .vector_ids = ids,       // Fourth
        .flags = VEXFS_INSERT_APPEND  // Fifth (using UAPI flag!)
    };
    
    printf("🔍 Testing batch insert with corrected structure:\n");
    printf("   vectors: %p\n", req.vectors);
    printf("   vector_count: %u\n", req.vector_count);
    printf("   dimensions: %u\n", req.dimensions);
    printf("   vector_ids: %p\n", req.vector_ids);
    printf("   flags: %u\n", req.flags);
    printf("   struct size: %zu bytes\n", sizeof(req));
    
    if (ioctl(fd, VEXFS_IOC_BATCH_INSERT, &req) == 0) {
        printf("✅ Batch insert successful (2 vectors)!\n");
    } else {
        printf("❌ Failed to batch insert vectors: %s (errno: %d)\n", strerror(errno), errno);
    }
    
    close(fd);
    printf("\n🔍 Test completed! Check dmesg for detailed logs.\n");
    return 0;
}