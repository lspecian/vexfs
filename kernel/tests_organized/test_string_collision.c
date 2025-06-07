#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>

int test_filename(const char *base_path, const char *filename) {
    char full_path[512];
    snprintf(full_path, sizeof(full_path), "%s/%s", base_path, filename);
    
    printf("Testing: %s -> ", filename);
    
    int fd = open(full_path, O_CREAT | O_WRONLY | O_TRUNC, 0644);
    if (fd < 0) {
        printf("FAILED (%s)\n", strerror(errno));
        return 0;
    }
    
    close(fd);
    unlink(full_path);  // Clean up
    printf("SUCCESS\n");
    return 1;
}

int main() {
    const char *mount_point = "/tmp/vexfs_v2_316_test";
    
    printf("=== Testing String Collision Theory ===\n\n");
    
    // Test the exact blocked strings
    printf("KNOWN BLOCKED STRINGS:\n");
    test_filename(mount_point, "metadata_test");
    test_filename(mount_point, "search_test");
    test_filename(mount_point, "batch_test");
    
    printf("\nTEST VARIATIONS:\n");
    // Test variations to see if it's exact string matching
    test_filename(mount_point, "metadata");
    test_filename(mount_point, "search");
    test_filename(mount_point, "batch");
    test_filename(mount_point, "test_metadata");
    test_filename(mount_point, "test_search");
    test_filename(mount_point, "test_batch");
    test_filename(mount_point, "metadata_");
    test_filename(mount_point, "search_");
    test_filename(mount_point, "batch_");
    test_filename(mount_point, "_metadata_test");
    test_filename(mount_point, "_search_test");
    test_filename(mount_point, "_batch_test");
    
    printf("\nTEST IOCTL COMMAND STRINGS:\n");
    // Test if it's related to ioctl command names
    test_filename(mount_point, "VEXFS_IOC_SET_VECTOR_META");
    test_filename(mount_point, "VEXFS_IOC_VECTOR_SEARCH");
    test_filename(mount_point, "VEXFS_IOC_BATCH_INSERT");
    test_filename(mount_point, "SET_VECTOR_META");
    test_filename(mount_point, "VECTOR_SEARCH");
    test_filename(mount_point, "BATCH_INSERT");
    
    printf("\nTEST CASE SENSITIVITY:\n");
    test_filename(mount_point, "METADATA_TEST");
    test_filename(mount_point, "SEARCH_TEST");
    test_filename(mount_point, "BATCH_TEST");
    test_filename(mount_point, "Metadata_Test");
    test_filename(mount_point, "Search_Test");
    test_filename(mount_point, "Batch_Test");
    
    return 0;
}