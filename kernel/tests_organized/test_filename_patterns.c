#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int test_file_creation(const char *filename) {
    printf("Testing: %s -> ", filename);
    
    int fd = open(filename, O_CREAT | O_WRONLY, 0644);
    if (fd < 0) {
        printf("FAILED (%s)\n", strerror(errno));
        return 0;
    }
    
    close(fd);
    unlink(filename);  // Clean up
    printf("SUCCESS\n");
    return 1;
}

int main() {
    const char *mount_point = "/tmp/vexfs_v2_316_test";
    char filename[512];
    
    printf("Testing filename patterns...\n\n");
    
    // Test various filename patterns
    const char *test_names[] = {
        "simple_test_file",    // KNOWN TO WORK
        "metadata_test",       // KNOWN TO FAIL
        "search_test",         // KNOWN TO FAIL  
        "batch_test",          // KNOWN TO FAIL
        "test_file",           // Simple
        "file_test",           // Simple
        "meta_test",           // Shorter
        "search",              // Even shorter
        "batch",               // Even shorter
        "a",                   // Single char
        "test123",             // Numbers
        "test_123",            // Underscore + numbers
        "simple_file",         // Similar to working one
        "simple_test",         // Similar to working one
        "test_simple",         // Reversed
        "metadata",            // Without _test
        "search_file",         // Different suffix
        "batch_file",          // Different suffix
        NULL
    };
    
    for (int i = 0; test_names[i] != NULL; i++) {
        snprintf(filename, sizeof(filename), "%s/%s", mount_point, test_names[i]);
        test_file_creation(filename);
    }
    
    return 0;
}