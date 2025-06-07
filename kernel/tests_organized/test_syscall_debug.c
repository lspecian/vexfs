#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <string.h>
#include <sys/stat.h>

void test_file_creation_detailed(const char *filename) {
    printf("=== Testing: %s ===\n", filename);
    
    // Test 1: open() with O_CREAT
    printf("1. Testing open() with O_CREAT...\n");
    int fd = open(filename, O_CREAT | O_WRONLY | O_TRUNC, 0644);
    if (fd < 0) {
        printf("   FAILED: open() returned %d, errno=%d (%s)\n", fd, errno, strerror(errno));
    } else {
        printf("   SUCCESS: open() returned fd=%d\n", fd);
        close(fd);
        
        // Test if file exists
        struct stat st;
        if (stat(filename, &st) == 0) {
            printf("   File exists after creation (size: %ld bytes)\n", st.st_size);
        } else {
            printf("   WARNING: File doesn't exist after creation!\n");
        }
        
        // Clean up
        if (unlink(filename) == 0) {
            printf("   Cleanup: File removed successfully\n");
        } else {
            printf("   Cleanup: Failed to remove file: %s\n", strerror(errno));
        }
    }
    
    // Test 2: creat() system call
    printf("2. Testing creat() system call...\n");
    fd = creat(filename, 0644);
    if (fd < 0) {
        printf("   FAILED: creat() returned %d, errno=%d (%s)\n", fd, errno, strerror(errno));
    } else {
        printf("   SUCCESS: creat() returned fd=%d\n", fd);
        close(fd);
        unlink(filename);
    }
    
    printf("\n");
}

int main() {
    const char *mount_point = "/tmp/vexfs_v2_316_test";
    char filename[256];
    
    printf("VexFS v2.0 Syscall Debug Test\n");
    printf("Mount point: %s\n\n", mount_point);
    
    // Test working filename
    snprintf(filename, sizeof(filename), "%s/working_test", mount_point);
    test_file_creation_detailed(filename);
    
    // Test problematic filenames
    snprintf(filename, sizeof(filename), "%s/metadata_test", mount_point);
    test_file_creation_detailed(filename);
    
    snprintf(filename, sizeof(filename), "%s/search_test", mount_point);
    test_file_creation_detailed(filename);
    
    snprintf(filename, sizeof(filename), "%s/batch_test", mount_point);
    test_file_creation_detailed(filename);
    
    return 0;
}