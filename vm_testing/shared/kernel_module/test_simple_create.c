#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

int main() {
    const char *filename = "/tmp/vexfs_v2_316_test/simple_test_file";
    
    printf("Testing file creation in VexFS v2.0...\n");
    
    int fd = open(filename, O_CREAT | O_WRONLY, 0644);
    if (fd < 0) {
        printf("ERROR: Failed to create file: %s (errno: %d)\n", strerror(errno), errno);
        return 1;
    }
    
    printf("SUCCESS: File created successfully!\n");
    close(fd);
    
    // Test if file exists
    if (access(filename, F_OK) == 0) {
        printf("SUCCESS: File exists and is accessible\n");
    } else {
        printf("WARNING: File was created but is not accessible\n");
    }
    
    return 0;
}
