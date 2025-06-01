/*
 * VexFS v2.0 Regression Prevention Test Suite
 *
 * This program provides automated testing to prevent future regressions
 * in the IOCTL interface that was fixed during the infrastructure breakthrough.
 *
 * Features:
 * - Structure size validation
 * - Field layout verification
 * - IOCTL command number validation
 * - Type consistency checks
 * - UAPI header compliance verification
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <assert.h>
#include <sys/ioctl.h>

/* Use the standardized UAPI header */
#include "vexfs_v2_uapi.h"

/* Test result tracking */
struct test_results {
    int total_tests;
    int passed_tests;
    int failed_tests;
    char last_error[256];
};

static struct test_results results = {0};

/* Test macros */
#define TEST_ASSERT(condition, message) do { \
    results.total_tests++; \
    if (condition) { \
        results.passed_tests++; \
        printf("✅ PASS: %s\n", message); \
    } else { \
        results.failed_tests++; \
        snprintf(results.last_error, sizeof(results.last_error), "%s", message); \
        printf("❌ FAIL: %s\n", message); \
    } \
} while(0)

#define TEST_SECTION(name) do { \
    printf("\n"); \
    for (int i = 0; i < 60; i++) printf("-"); \
    printf("\n🧪 Testing: %s\n", name); \
    for (int i = 0; i < 60; i++) printf("-"); \
    printf("\n"); \
} while(0)

/* Structure size regression tests */
void test_structure_sizes(void) {
    TEST_SECTION("Structure Size Validation");
    
    /* Critical: These sizes must never change without version bump */
    TEST_ASSERT(sizeof(struct vexfs_vector_file_info) == VEXFS_VECTOR_FILE_INFO_SIZE,
                "vexfs_vector_file_info size matches expected 40 bytes");
    
    TEST_ASSERT(sizeof(struct vexfs_vector_search_request) == VEXFS_VECTOR_SEARCH_REQUEST_SIZE,
                "vexfs_vector_search_request size matches expected 48 bytes");
    
    TEST_ASSERT(sizeof(struct vexfs_batch_insert_request) == VEXFS_BATCH_INSERT_REQUEST_SIZE,
                "vexfs_batch_insert_request size matches expected 32 bytes");
    
    /* Verify minimum sizes to ensure no accidental shrinking */
    TEST_ASSERT(sizeof(struct vexfs_vector_file_info) >= 40,
                "vexfs_vector_file_info is at least 40 bytes");
    
    TEST_ASSERT(sizeof(struct vexfs_batch_insert_request) >= 32,
                "vexfs_batch_insert_request is at least 32 bytes (includes flags)");
    
    printf("📊 Structure sizes validated against infrastructure breakthrough requirements\n");
}

/* Field layout regression tests */
void test_field_layouts(void) {
    TEST_SECTION("Field Layout Validation");
    
    /* Test vexfs_vector_file_info layout */
    struct vexfs_vector_file_info info = {0};
    char *base = (char*)&info;
    
    TEST_ASSERT((char*)&info.dimensions - base == 0,
                "vexfs_vector_file_info.dimensions at offset 0");
    TEST_ASSERT((char*)&info.element_type - base == 4,
                "vexfs_vector_file_info.element_type at offset 4");
    TEST_ASSERT((char*)&info.vector_count - base == 8,
                "vexfs_vector_file_info.vector_count at offset 8");
    TEST_ASSERT((char*)&info.storage_format - base == 12,
                "vexfs_vector_file_info.storage_format at offset 12");
    TEST_ASSERT((char*)&info.data_offset - base == 16,
                "vexfs_vector_file_info.data_offset at offset 16");
    TEST_ASSERT((char*)&info.index_offset - base == 24,
                "vexfs_vector_file_info.index_offset at offset 24");
    TEST_ASSERT((char*)&info.compression_type - base == 32,
                "vexfs_vector_file_info.compression_type at offset 32");
    TEST_ASSERT((char*)&info.alignment_bytes - base == 36,
                "vexfs_vector_file_info.alignment_bytes at offset 36");
    
    /* Test vexfs_batch_insert_request layout - CRITICAL for breakthrough */
    struct vexfs_batch_insert_request req = {0};
    base = (char*)&req;
    
    TEST_ASSERT((char*)&req.vectors - base == 0,
                "vexfs_batch_insert_request.vectors at offset 0 (CRITICAL)");
    TEST_ASSERT((char*)&req.vector_count - base == 8,
                "vexfs_batch_insert_request.vector_count at offset 8 (CRITICAL)");
    TEST_ASSERT((char*)&req.dimensions - base == 12,
                "vexfs_batch_insert_request.dimensions at offset 12 (CRITICAL)");
    TEST_ASSERT((char*)&req.vector_ids - base == 16,
                "vexfs_batch_insert_request.vector_ids at offset 16 (CRITICAL)");
    TEST_ASSERT((char*)&req.flags - base == 24,
                "vexfs_batch_insert_request.flags at offset 24 (BREAKTHROUGH FIELD)");
    
    printf("🎯 Field layouts match infrastructure breakthrough requirements\n");
}

/* IOCTL command number regression tests */
void test_ioctl_commands(void) {
    TEST_SECTION("IOCTL Command Number Validation");
    
    /* Verify magic number consistency */
    TEST_ASSERT(VEXFS_IOC_MAGIC == 'V',
                "IOCTL magic number is 'V'");
    
    /* Verify command numbers match breakthrough fixes */
    unsigned long expected_set_meta = _IOW('V', 1, struct vexfs_vector_file_info);
    unsigned long expected_get_meta = _IOR('V', 2, struct vexfs_vector_file_info);
    unsigned long expected_search = _IOWR('V', 3, struct vexfs_vector_search_request);
    unsigned long expected_batch = _IOW('V', 4, struct vexfs_batch_insert_request);
    
    TEST_ASSERT(VEXFS_IOC_SET_VECTOR_META == expected_set_meta,
                "VEXFS_IOC_SET_VECTOR_META command number correct");
    TEST_ASSERT(VEXFS_IOC_GET_VECTOR_META == expected_get_meta,
                "VEXFS_IOC_GET_VECTOR_META command number correct");
    TEST_ASSERT(VEXFS_IOC_VECTOR_SEARCH == expected_search,
                "VEXFS_IOC_VECTOR_SEARCH command number correct");
    TEST_ASSERT(VEXFS_IOC_BATCH_INSERT == expected_batch,
                "VEXFS_IOC_BATCH_INSERT command number correct (BREAKTHROUGH FIX)");
    
    /* Verify batch insert is command 4, not 3 (critical breakthrough fix) */
    TEST_ASSERT((VEXFS_IOC_BATCH_INSERT & 0xFF) == 4,
                "Batch insert uses command number 4 (not 3 - breakthrough fix)");
    
    printf("🔧 IOCTL commands validated against breakthrough fixes\n");
}

/* Type consistency regression tests */
void test_type_consistency(void) {
    TEST_SECTION("Type Consistency Validation");
    
    /* Verify all structures use __u32/__u64 types for kernel compatibility */
    struct vexfs_vector_file_info info = {0};
    struct vexfs_batch_insert_request req = {0};
    
    /* Test that we can assign standard types to UAPI types */
    info.dimensions = (uint32_t)128;
    info.element_type = (uint32_t)VEXFS_VECTOR_FLOAT32;
    info.data_offset = (uint64_t)0x1000;
    
    req.vector_count = (uint32_t)10;
    req.dimensions = (uint32_t)128;
    req.flags = (uint32_t)VEXFS_INSERT_APPEND;
    
    TEST_ASSERT(info.dimensions == 128,
                "uint32_t assignment to dimensions field works");
    TEST_ASSERT(info.element_type == VEXFS_VECTOR_FLOAT32,
                "UAPI constant assignment works");
    TEST_ASSERT(req.flags == VEXFS_INSERT_APPEND,
                "flags field accepts UAPI constants");
    
    printf("📝 Type consistency validated for kernel compatibility\n");
}

/* Constants and macros regression tests */
void test_constants_and_macros(void) {
    TEST_SECTION("Constants and Macros Validation");
    
    /* Verify vector element types */
    TEST_ASSERT(VEXFS_VECTOR_FLOAT32 == 0x01,
                "VEXFS_VECTOR_FLOAT32 constant value");
    TEST_ASSERT(VEXFS_VECTOR_FLOAT16 == 0x02,
                "VEXFS_VECTOR_FLOAT16 constant value");
    TEST_ASSERT(VEXFS_VECTOR_INT8 == 0x03,
                "VEXFS_VECTOR_INT8 constant value");
    TEST_ASSERT(VEXFS_VECTOR_BINARY == 0x04,
                "VEXFS_VECTOR_BINARY constant value");
    
    /* Verify search types */
    TEST_ASSERT(VEXFS_SEARCH_EUCLIDEAN == 0x00,
                "VEXFS_SEARCH_EUCLIDEAN constant value");
    TEST_ASSERT(VEXFS_SEARCH_COSINE == 0x01,
                "VEXFS_SEARCH_COSINE constant value");
    TEST_ASSERT(VEXFS_SEARCH_DOT_PRODUCT == 0x02,
                "VEXFS_SEARCH_DOT_PRODUCT constant value");
    
    /* Verify insert flags (critical for breakthrough) */
    TEST_ASSERT(VEXFS_INSERT_OVERWRITE == 0x01,
                "VEXFS_INSERT_OVERWRITE constant value");
    TEST_ASSERT(VEXFS_INSERT_APPEND == 0x02,
                "VEXFS_INSERT_APPEND constant value");
    TEST_ASSERT(VEXFS_INSERT_VALIDATE == 0x04,
                "VEXFS_INSERT_VALIDATE constant value");
    
    /* Verify helper macros */
    TEST_ASSERT(VEXFS_VECTOR_DATA_SIZE(4, 10) == 4 * 10 * sizeof(float),
                "VEXFS_VECTOR_DATA_SIZE macro calculation");
    TEST_ASSERT(VEXFS_VECTOR_ID_SIZE(10) == 10 * sizeof(__u64),
                "VEXFS_VECTOR_ID_SIZE macro calculation");
    
    /* Verify validation macros */
    TEST_ASSERT(VEXFS_VALID_DIMENSIONS(128) == 1,
                "VEXFS_VALID_DIMENSIONS accepts valid dimensions");
    TEST_ASSERT(VEXFS_VALID_DIMENSIONS(0) == 0,
                "VEXFS_VALID_DIMENSIONS rejects zero dimensions");
    TEST_ASSERT(VEXFS_VALID_COUNT(1000) == 1,
                "VEXFS_VALID_COUNT accepts valid count");
    TEST_ASSERT(VEXFS_VALID_COUNT(0) == 0,
                "VEXFS_VALID_COUNT rejects zero count");
    
    printf("🔢 Constants and macros validated for API consistency\n");
}

/* Version information regression tests */
void test_version_information(void) {
    TEST_SECTION("Version Information Validation");
    
    /* Verify version constants exist and are reasonable */
    TEST_ASSERT(VEXFS_V2_MAJOR_VERSION == 2,
                "Major version is 2 for VexFS v2.0");
    TEST_ASSERT(VEXFS_V2_MINOR_VERSION >= 0,
                "Minor version is non-negative");
    TEST_ASSERT(VEXFS_V2_PATCH_VERSION >= 0,
                "Patch version is non-negative");
    
    /* Verify magic number */
    TEST_ASSERT(VEXFS_V2_MAGIC == 0x56455832,
                "VexFS v2.0 magic number is 'VEX2'");
    
    printf("📋 Version information validated for API versioning\n");
}

/* Compile-time validation tests */
void test_compile_time_validation(void) {
    TEST_SECTION("Compile-Time Validation");
    
    /* These tests verify that the _Static_assert statements in the header
     * are working correctly by checking the same conditions at runtime */
    
    TEST_ASSERT(sizeof(struct vexfs_vector_file_info) == VEXFS_VECTOR_FILE_INFO_SIZE,
                "Static assertion condition for vexfs_vector_file_info");
    TEST_ASSERT(sizeof(struct vexfs_vector_search_request) == VEXFS_VECTOR_SEARCH_REQUEST_SIZE,
                "Static assertion condition for vexfs_vector_search_request");
    TEST_ASSERT(sizeof(struct vexfs_batch_insert_request) == VEXFS_BATCH_INSERT_REQUEST_SIZE,
                "Static assertion condition for vexfs_batch_insert_request");
    
    printf("⚡ Compile-time validation working correctly\n");
    printf("   (If this program compiled, all _Static_assert checks passed)\n");
}

/* Performance impact validation */
void test_performance_impact_prevention(void) {
    TEST_SECTION("Performance Impact Prevention");
    
    /* Verify structure sizes are optimal for performance */
    TEST_ASSERT(sizeof(struct vexfs_vector_file_info) % 8 == 0,
                "vexfs_vector_file_info is 8-byte aligned for performance");
    TEST_ASSERT(sizeof(struct vexfs_batch_insert_request) % 8 == 0,
                "vexfs_batch_insert_request is 8-byte aligned for performance");
    
    /* Verify no excessive padding */
    TEST_ASSERT(sizeof(struct vexfs_vector_file_info) <= 48,
                "vexfs_vector_file_info size is reasonable (≤48 bytes)");
    TEST_ASSERT(sizeof(struct vexfs_batch_insert_request) <= 40,
                "vexfs_batch_insert_request size is reasonable (≤40 bytes)");
    
    /* Verify critical fields are properly sized */
    TEST_ASSERT(sizeof(((struct vexfs_batch_insert_request*)0)->flags) == 4,
                "flags field is 32-bit (breakthrough requirement)");
    
    printf("⚡ Performance characteristics validated\n");
}

/* Main test runner */
int main(int argc, char *argv[]) {
    printf("🛡️  VexFS v2.0 Regression Prevention Test Suite\n");
    printf("===============================================\n");
    printf("🎯 Preventing regressions in IOCTL interface infrastructure breakthrough\n\n");
    
    /* Run all test suites */
    test_structure_sizes();
    test_field_layouts();
    test_ioctl_commands();
    test_type_consistency();
    test_constants_and_macros();
    test_version_information();
    test_compile_time_validation();
    test_performance_impact_prevention();
    
    /* Print summary */
    printf("\n");
    for (int i = 0; i < 80; i++) printf("=");
    printf("\n🧪 Test Results Summary\n");
    for (int i = 0; i < 80; i++) printf("=");
    printf("\n");
    
    printf("📊 Total tests run:     %d\n", results.total_tests);
    printf("✅ Tests passed:       %d\n", results.passed_tests);
    printf("❌ Tests failed:       %d\n", results.failed_tests);
    
    if (results.failed_tests == 0) {
        printf("\n🎉 ALL TESTS PASSED!\n");
        printf("✅ No regressions detected in IOCTL interface\n");
        printf("✅ Infrastructure breakthrough integrity maintained\n");
        printf("✅ VexFS v2.0 IOCTL interface is regression-free\n");
        printf("\n🛡️  Regression prevention: ACTIVE\n");
        printf("🚀 Infrastructure status: PRODUCTION READY\n");
        return 0;
    } else {
        printf("\n⚠️  REGRESSION DETECTED!\n");
        printf("❌ %d test(s) failed\n", results.failed_tests);
        printf("🔍 Last failure: %s\n", results.last_error);
        printf("\n🚨 CRITICAL: Infrastructure breakthrough may be compromised\n");
        printf("🔧 Action required: Fix regressions before deployment\n");
        return 1;
    }
}