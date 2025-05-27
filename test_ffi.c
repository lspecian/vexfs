/*
 * VexFS - Vector Extended File System
 * Copyright (C) 2025 VexFS Contributors
 *
 * Test program for VexFS Rust FFI integration
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "vexfs_ffi.h"

int main(void) {
    printf("VexFS FFI Test Program\n");
    printf("======================\n\n");

    // Test 1: Basic FFI test
    printf("1. Testing basic FFI connection...\n");
    int result = vexfs_rust_test_basic();
    printf("   vexfs_rust_test_basic() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");

    // Test 2: Version check
    printf("\n2. Testing version information...\n");
    int version = vexfs_rust_get_version();
    int major = (version >> 16) & 0xFF;
    int minor = (version >> 8) & 0xFF;
    int patch = version & 0xFF;
    printf("   VexFS version: %d.%d.%d (raw: %d)\n", major, minor, patch, version);

    // Test 3: Initialization
    printf("\n3. Testing initialization...\n");
    result = vexfs_rust_init();
    printf("   vexfs_rust_init() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");

    // Test 4: Vector operations test
    printf("\n4. Testing vector operations...\n");
    result = vexfs_rust_test_vector_ops();
    printf("   vexfs_rust_test_vector_ops() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");

    // Test 5: Statistics test
    printf("\n5. Testing filesystem statistics...\n");
    uint64_t blocks, free_blocks, files, free_files;
    result = vexfs_rust_get_statfs(&blocks, &free_blocks, &files, &free_files);
    printf("   vexfs_rust_get_statfs() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");
    if (result == VEXFS_SUCCESS) {
        printf("   Total blocks: %lu, Free: %lu\n", blocks, free_blocks);
        printf("   Total files: %lu, Free: %lu\n", files, free_files);
    }

    // Test 6: Userspace functions (if available)
    printf("\n6. Testing userspace functions...\n");
    result = vexfs_rust_userspace_init();
    printf("   vexfs_rust_userspace_init() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");

    result = vexfs_rust_vector_search();
    printf("   vexfs_rust_vector_search() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");

    result = vexfs_rust_vector_storage();
    printf("   vexfs_rust_vector_storage() = %d %s\n", 
           result, (result == VEXFS_SUCCESS) ? "(SUCCESS)" : "(FAILED)");

    // Test 7: Cleanup
    printf("\n7. Testing cleanup...\n");
    vexfs_rust_exit();
    printf("   vexfs_rust_exit() completed\n");

    // Test 8: Error handling (null pointer test)
    printf("\n8. Testing error handling...\n");
    result = vexfs_rust_get_statfs(NULL, NULL, NULL, NULL);
    printf("   vexfs_rust_get_statfs(null ptrs) = %d %s\n", 
           result, (result == VEXFS_ERROR_INVAL) ? "(CORRECTLY REJECTED)" : "(UNEXPECTED)");

    result = vexfs_rust_fill_super(NULL);
    printf("   vexfs_rust_fill_super(null ptr) = %d %s\n", 
           result, (result == VEXFS_ERROR_INVAL) ? "(CORRECTLY REJECTED)" : "(UNEXPECTED)");

    printf("\nAll FFI tests completed!\n");
    return 0;
}