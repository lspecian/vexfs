#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int test_file_creation(const char *filename) {
    printf("Testing creation of: %s\n", filename);
    
    int fd = open(filename, O_CREAT | O_WRONLY, 0644);
    if (fd < 0) {
        printf("ERROR: Failed to create %s: %s (errno: %d)\n", filename, strerror(errno), errno);
        return 0;
    }
    
    printf("SUCCESS: %s created successfully!\n", filename);
    close(fd);
    
    // Test if file exists
    if (access(filename, F_OK) == 0) {
        printf("SUCCESS: %s exists and is accessible\n", filename);
        return 1;
    } else {
        printf("WARNING: %s was created but is not accessible\n", filename);
        return 0;
    }
}

int main() {
    const char *mount_point = "/tmp/vexfs_v2_316_test";
    char filename[512];
    int success_count = 0;
    
    printf("Testing file creation patterns from benchmark...\n\n");
    
    // Test the files that the benchmark tries to create
    snprintf(filename, sizeof(filename), "%s/metadata_test", mount_point);
    success_count += test_file_creation(filename);
    
    printf("\n");
    snprintf(filename, sizeof(filename), "%s/search_test", mount_point);
    success_count += test_file_creation(filename);
    
    printf("\n");
    snprintf(filename, sizeof(filename), "%s/batch_test", mount_point);
    success_count += test_file_creation(filename);
    
    printf("\n");
    snprintf(filename, sizeof(filename), "%s/simple_test_file", mount_point);
    success_count += test_file_creation(filename);
    
    printf("\n=== SUMMARY ===\n");
    printf("Successfully created: %d/4 files\n", success_count);
    
    // List directory contents
    printf("\nDirectory contents:\n");
    char cmd[256];
    snprintf(cmd, sizeof(cmd), "ls -la %s/", mount_point);
    system(cmd);
    
    return success_count == 4 ? 0 : 1;
}