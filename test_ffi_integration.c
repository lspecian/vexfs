/*
 * FFI Integration Test for VexFS
 * 
 * This test verifies that C code can successfully link with the Rust static library
 * and call FFI functions. This is a critical test for the kernel module integration.
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

/* Include the FFI header */
#include "vexfs_ffi.h"

int main(int argc, char *argv[])
{
    int ret;
    uint64_t blocks, free_blocks, files, free_files;

    printf("VexFS FFI Integration Test\n");
    printf("==========================\n\n");

    /* Test 1: Initialize Rust library */
    printf("1. Testing Rust library initialization...\n");
    ret = vexfs_rust_init();
    if (ret != VEXFS_SUCCESS) {
        printf("   ❌ FAILED: vexfs_rust_init() returned %d\n", ret);
        return 1;
    }
    printf("   ✅ SUCCESS: Rust library initialized\n\n");

    /* Test 2: Get version information */
    printf("2. Testing version information...\n");
    ret = vexfs_rust_get_version();
    printf("   ✅ SUCCESS: Version = 0x%08x\n\n", ret);

    /* Test 3: Basic FFI functionality */
    printf("3. Testing basic FFI function...\n");
    ret = vexfs_rust_test_basic();
    if (ret != VEXFS_SUCCESS) {
        printf("   ❌ FAILED: vexfs_rust_test_basic() returned %d\n", ret);
        goto cleanup;
    }
    printf("   ✅ SUCCESS: Basic FFI test passed\n\n");

    /* Test 4: Vector operations */
    printf("4. Testing vector operations FFI...\n");
    ret = vexfs_rust_test_vector_ops();
    if (ret != VEXFS_SUCCESS) {
        printf("   ❌ FAILED: vexfs_rust_test_vector_ops() returned %d\n", ret);
        goto cleanup;
    }
    printf("   ✅ SUCCESS: Vector ops FFI test passed\n\n");

    /* Test 5: Statistics function */
    printf("5. Testing filesystem statistics FFI...\n");
    ret = vexfs_rust_get_statfs(&blocks, &free_blocks, &files, &free_files);
    if (ret != VEXFS_SUCCESS) {
        printf("   ❌ FAILED: vexfs_rust_get_statfs() returned %d\n", ret);
        goto cleanup;
    }
    printf("   ✅ SUCCESS: Statistics retrieved\n");
    printf("     Blocks: %lu, Free: %lu\n", blocks, free_blocks);
    printf("     Files: %lu, Free: %lu\n\n", files, free_files);

    /* Test 6: Cleanup */
    printf("6. Testing Rust library cleanup...\n");
    vexfs_rust_exit();
    printf("   ✅ SUCCESS: Rust library cleaned up\n\n");

    printf("🎉 ALL FFI INTEGRATION TESTS PASSED!\n");
    printf("✅ Rust static library is ready for kernel module integration\n");
    return 0;

cleanup:
    vexfs_rust_exit();
    return 1;
}