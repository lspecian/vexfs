/*
 * VexFS v2.0 Enhanced File System Registration Test Suite
 * 
 * Comprehensive test suite for testing vector-specific mount options,
 * SIMD capability detection, and compatibility checking.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/string.h>

#include "vexfs_v2_phase3.h"
#include "vexfs_v2_enhanced_registration.h"

/* Test result tracking */
static int tests_passed = 0;
static int tests_failed = 0;
static int total_tests = 0;

/* Test macros */
#define TEST_ASSERT(condition, test_name) do { \
    total_tests++; \
    if (condition) { \
        tests_passed++; \
        printk(KERN_INFO "VexFS Test: PASS - %s\n", test_name); \
    } else { \
        tests_failed++; \
        printk(KERN_ERR "VexFS Test: FAIL - %s\n", test_name); \
    } \
} while(0)

#define TEST_ASSERT_EQ(actual, expected, test_name) do { \
    total_tests++; \
    if ((actual) == (expected)) { \
        tests_passed++; \
        printk(KERN_INFO "VexFS Test: PASS - %s (got %u, expected %u)\n", \
               test_name, (unsigned)(actual), (unsigned)(expected)); \
    } else { \
        tests_failed++; \
        printk(KERN_ERR "VexFS Test: FAIL - %s (got %u, expected %u)\n", \
               test_name, (unsigned)(actual), (unsigned)(expected)); \
    } \
} while(0)

#define TEST_ASSERT_STR_EQ(actual, expected, test_name) do { \
    total_tests++; \
    if (strcmp((actual), (expected)) == 0) { \
        tests_passed++; \
        printk(KERN_INFO "VexFS Test: PASS - %s\n", test_name); \
    } else { \
        tests_failed++; \
        printk(KERN_ERR "VexFS Test: FAIL - %s (got '%s', expected '%s')\n", \
               test_name, (actual), (expected)); \
    } \
} while(0)

/* 🔥 TEST SUITE 1: MOUNT OPTION PARSING 🔥 */

static void test_default_mount_options(void)
{
    struct vexfs_mount_opts opts;
    
    printk(KERN_INFO "VexFS Test: Testing default mount options\n");
    
    vexfs_set_default_mount_options(&opts);
    
    TEST_ASSERT_EQ(opts.max_vector_dim, VEXFS_DEFAULT_MAX_VECTOR_DIM, 
                   "Default max_vector_dim");
    TEST_ASSERT_EQ(opts.default_element_type, VEXFS_DEFAULT_ELEMENT_TYPE, 
                   "Default element_type");
    TEST_ASSERT_EQ(opts.vector_alignment, VEXFS_DEFAULT_VECTOR_ALIGNMENT, 
                   "Default vector_alignment");
    TEST_ASSERT_EQ(opts.batch_size, VEXFS_DEFAULT_BATCH_SIZE, 
                   "Default batch_size");
    TEST_ASSERT_EQ(opts.cache_size_mb, VEXFS_DEFAULT_CACHE_SIZE_MB, 
                   "Default cache_size");
    TEST_ASSERT(opts.numa_aware == true, "Default NUMA awareness");
    TEST_ASSERT(opts.disable_simd == false, "Default SIMD enabled");
    TEST_ASSERT(opts.readonly == false, "Default read-write mode");
    TEST_ASSERT(opts.options_parsed == false, "Default options not parsed");
}

static void test_mount_option_parsing(void)
{
    struct vexfs_mount_opts opts;
    char options[] = "max_vector_dim=2048,default_element_type=float32,vector_alignment=64,batch_size=16";
    int ret;
    
    printk(KERN_INFO "VexFS Test: Testing mount option parsing\n");
    
    ret = vexfs_parse_options(options, &opts);
    
    TEST_ASSERT_EQ(ret, 0, "Mount option parsing success");
    TEST_ASSERT_EQ(opts.max_vector_dim, 2048, "Parsed max_vector_dim");
    TEST_ASSERT_EQ(opts.default_element_type, VEXFS_VECTOR_FLOAT32, "Parsed element_type");
    TEST_ASSERT_EQ(opts.vector_alignment, 64, "Parsed vector_alignment");
    TEST_ASSERT_EQ(opts.batch_size, 16, "Parsed batch_size");
    TEST_ASSERT(opts.options_parsed == true, "Options parsed flag set");
}

static void test_invalid_mount_options(void)
{
    struct vexfs_mount_opts opts;
    char invalid_options[] = "max_vector_dim=999999,invalid_option=value";
    int ret;
    
    printk(KERN_INFO "VexFS Test: Testing invalid mount option handling\n");
    
    ret = vexfs_parse_options(invalid_options, &opts);
    
    TEST_ASSERT(ret != 0, "Invalid mount options rejected");
}

static void test_boolean_option_parsing(void)
{
    struct vexfs_mount_opts opts;
    char options[] = "numa_aware=yes,disable_simd=true,readonly=1";
    int ret;
    
    printk(KERN_INFO "VexFS Test: Testing boolean option parsing\n");
    
    ret = vexfs_parse_options(options, &opts);
    
    TEST_ASSERT_EQ(ret, 0, "Boolean option parsing success");
    TEST_ASSERT(opts.numa_aware == true, "Parsed numa_aware=yes");
    TEST_ASSERT(opts.disable_simd == true, "Parsed disable_simd=true");
    TEST_ASSERT(opts.readonly == true, "Parsed readonly=1");
}

/* 🔥 TEST SUITE 2: ELEMENT TYPE CONVERSION 🔥 */

static void test_element_type_conversion(void)
{
    u32 type_id;
    const char *type_name;
    
    printk(KERN_INFO "VexFS Test: Testing element type conversion\n");
    
    /* Test string to type ID conversion */
    type_id = vexfs_string_to_element_type("float32");
    TEST_ASSERT_EQ(type_id, VEXFS_VECTOR_FLOAT32, "float32 string to ID");
    
    type_id = vexfs_string_to_element_type("float16");
    TEST_ASSERT_EQ(type_id, VEXFS_VECTOR_FLOAT16, "float16 string to ID");
    
    type_id = vexfs_string_to_element_type("int8");
    TEST_ASSERT_EQ(type_id, VEXFS_VECTOR_INT8, "int8 string to ID");
    
    type_id = vexfs_string_to_element_type("invalid");
    TEST_ASSERT_EQ(type_id, 0, "Invalid string returns 0");
    
    /* Test type ID to string conversion */
    type_name = vexfs_element_type_to_string(VEXFS_VECTOR_FLOAT32);
    TEST_ASSERT_STR_EQ(type_name, "float32", "float32 ID to string");
    
    type_name = vexfs_element_type_to_string(VEXFS_VECTOR_FLOAT16);
    TEST_ASSERT_STR_EQ(type_name, "float16", "float16 ID to string");
    
    type_name = vexfs_element_type_to_string(999);
    TEST_ASSERT_STR_EQ(type_name, "unknown", "Invalid ID returns unknown");
}

/* 🔥 TEST SUITE 3: SIMD MODE CONVERSION 🔥 */

static void test_simd_mode_conversion(void)
{
    u32 capabilities;
    
    printk(KERN_INFO "VexFS Test: Testing SIMD mode conversion\n");
    
    capabilities = vexfs_string_to_simd_mode("auto");
    TEST_ASSERT_EQ(capabilities, 0, "auto mode returns 0");
    
    capabilities = vexfs_string_to_simd_mode("sse2");
    TEST_ASSERT_EQ(capabilities, VEXFS_SIMD_SSE2, "sse2 mode");
    
    capabilities = vexfs_string_to_simd_mode("avx2");
    TEST_ASSERT_EQ(capabilities, VEXFS_SIMD_AVX2, "avx2 mode");
    
    capabilities = vexfs_string_to_simd_mode("avx512");
    TEST_ASSERT_EQ(capabilities, VEXFS_SIMD_AVX512, "avx512 mode");
    
    capabilities = vexfs_string_to_simd_mode("invalid");
    TEST_ASSERT_EQ(capabilities, 0, "Invalid SIMD mode returns 0");
}

/* 🔥 TEST SUITE 4: VALIDATION FUNCTIONS 🔥 */

static void test_validation_functions(void)
{
    printk(KERN_INFO "VexFS Test: Testing validation functions\n");
    
    /* Test vector dimension validation */
    TEST_ASSERT(vexfs_is_valid_vector_dimension(1024) == true, "Valid dimension 1024");
    TEST_ASSERT(vexfs_is_valid_vector_dimension(2048) == true, "Valid dimension 2048");
    TEST_ASSERT(vexfs_is_valid_vector_dimension(0) == false, "Invalid dimension 0");
    TEST_ASSERT(vexfs_is_valid_vector_dimension(999999) == false, "Invalid dimension too large");
    TEST_ASSERT(vexfs_is_valid_vector_dimension(1023) == false, "Invalid non-power-of-2");
    
    /* Test alignment validation */
    TEST_ASSERT(vexfs_is_valid_alignment(16) == true, "Valid alignment 16");
    TEST_ASSERT(vexfs_is_valid_alignment(32) == true, "Valid alignment 32");
    TEST_ASSERT(vexfs_is_valid_alignment(64) == true, "Valid alignment 64");
    TEST_ASSERT(vexfs_is_valid_alignment(0) == false, "Invalid alignment 0");
    TEST_ASSERT(vexfs_is_valid_alignment(15) == false, "Invalid non-power-of-2 alignment");
    TEST_ASSERT(vexfs_is_valid_alignment(128) == false, "Invalid alignment too large");
    
    /* Test batch size validation */
    TEST_ASSERT(vexfs_is_valid_batch_size(8) == true, "Valid batch size 8");
    TEST_ASSERT(vexfs_is_valid_batch_size(16) == true, "Valid batch size 16");
    TEST_ASSERT(vexfs_is_valid_batch_size(32) == true, "Valid batch size 32");
    TEST_ASSERT(vexfs_is_valid_batch_size(0) == false, "Invalid batch size 0");
    TEST_ASSERT(vexfs_is_valid_batch_size(7) == false, "Invalid non-power-of-2 batch size");
    TEST_ASSERT(vexfs_is_valid_batch_size(128) == false, "Invalid batch size too large");
    
    /* Test power of two validation */
    TEST_ASSERT(vexfs_is_power_of_two(1) == true, "1 is power of two");
    TEST_ASSERT(vexfs_is_power_of_two(2) == true, "2 is power of two");
    TEST_ASSERT(vexfs_is_power_of_two(4) == true, "4 is power of two");
    TEST_ASSERT(vexfs_is_power_of_two(1024) == true, "1024 is power of two");
    TEST_ASSERT(vexfs_is_power_of_two(0) == false, "0 is not power of two");
    TEST_ASSERT(vexfs_is_power_of_two(3) == false, "3 is not power of two");
    TEST_ASSERT(vexfs_is_power_of_two(1023) == false, "1023 is not power of two");
}

/* 🔥 TEST SUITE 5: CAPABILITY DETECTION 🔥 */

static void test_capability_detection(void)
{
    struct vexfs_capability_check check;
    int ret;
    
    printk(KERN_INFO "VexFS Test: Testing capability detection\n");
    
    ret = vexfs_detect_system_capabilities(&check);
    
    TEST_ASSERT_EQ(ret, 0, "Capability detection success");
    TEST_ASSERT(check.cache_line_size > 0, "Cache line size detected");
    TEST_ASSERT(check.numa_node_count > 0, "NUMA node count detected");
    
    /* SIMD capabilities should be detected (may vary by system) */
    printk(KERN_INFO "VexFS Test: Detected SIMD capabilities: 0x%x\n", 
           check.detected_capabilities);
    printk(KERN_INFO "VexFS Test: Optimal vector width: %u bits\n", 
           check.optimal_vector_width);
    printk(KERN_INFO "VexFS Test: NUMA nodes: %u\n", check.numa_node_count);
    printk(KERN_INFO "VexFS Test: Cache line size: %u bytes\n", check.cache_line_size);
}

/* 🔥 TEST SUITE 6: SYSTEM REQUIREMENTS 🔥 */

static void test_system_requirements(void)
{
    bool result;
    
    printk(KERN_INFO "VexFS Test: Testing system requirements\n");
    
    result = vexfs_check_minimum_requirements();
    TEST_ASSERT(result == true, "Minimum requirements met");
    
    result = vexfs_check_kernel_version_compatibility();
    TEST_ASSERT(result == true, "Kernel version compatible");
    
    /* Test CPU feature checking */
    result = vexfs_check_cpu_features(0); /* No features required */
    TEST_ASSERT(result == true, "No CPU features check passes");
}

/* 🔥 TEST SUITE 7: INTEGRATION TESTS 🔥 */

static void test_mount_option_integration(void)
{
    struct vexfs_mount_opts opts;
    struct vexfs_capability_check check;
    char complex_options[] = "max_vector_dim=4096,default_element_type=float32,"
                            "vector_alignment=32,batch_size=8,cache_size=128,"
                            "simd_mode=auto,numa_aware=yes,hnsw_m=32,"
                            "hnsw_ef_construction=400,debug_level=2";
    int ret;
    
    printk(KERN_INFO "VexFS Test: Testing mount option integration\n");
    
    /* Parse complex mount options */
    ret = vexfs_parse_options(complex_options, &opts);
    TEST_ASSERT_EQ(ret, 0, "Complex mount options parsed");
    
    /* Detect capabilities */
    ret = vexfs_detect_system_capabilities(&check);
    TEST_ASSERT_EQ(ret, 0, "Capabilities detected for integration");
    
    /* Validate SIMD requirements */
    ret = vexfs_validate_simd_requirements(&opts, &check);
    TEST_ASSERT_EQ(ret, 0, "SIMD requirements validated");
    
    /* Verify parsed values */
    TEST_ASSERT_EQ(opts.max_vector_dim, 4096, "Integration: max_vector_dim");
    TEST_ASSERT_EQ(opts.vector_alignment, 32, "Integration: vector_alignment");
    TEST_ASSERT_EQ(opts.batch_size, 8, "Integration: batch_size");
    TEST_ASSERT_EQ(opts.cache_size_mb, 128, "Integration: cache_size");
    TEST_ASSERT_EQ(opts.hnsw_m, 32, "Integration: hnsw_m");
    TEST_ASSERT_EQ(opts.hnsw_ef_construction, 400, "Integration: hnsw_ef_construction");
    TEST_ASSERT_EQ(opts.debug_level, 2, "Integration: debug_level");
    TEST_ASSERT(opts.numa_aware == true, "Integration: numa_aware");
}

/* 🔥 MAIN TEST RUNNER 🔥 */

static int __init test_enhanced_registration_init(void)
{
    printk(KERN_INFO "VexFS Enhanced Registration Test Suite Starting\n");
    printk(KERN_INFO "================================================\n");
    
    /* Reset test counters */
    tests_passed = 0;
    tests_failed = 0;
    total_tests = 0;
    
    /* Run test suites */
    test_default_mount_options();
    test_mount_option_parsing();
    test_invalid_mount_options();
    test_boolean_option_parsing();
    
    test_element_type_conversion();
    test_simd_mode_conversion();
    
    test_validation_functions();
    test_capability_detection();
    test_system_requirements();
    
    test_mount_option_integration();
    
    /* Print test results */
    printk(KERN_INFO "================================================\n");
    printk(KERN_INFO "VexFS Enhanced Registration Test Results:\n");
    printk(KERN_INFO "  Total tests: %d\n", total_tests);
    printk(KERN_INFO "  Passed: %d\n", tests_passed);
    printk(KERN_INFO "  Failed: %d\n", tests_failed);
    
    if (tests_failed == 0) {
        printk(KERN_INFO "🎉 ALL TESTS PASSED! 🎉\n");
    } else {
        printk(KERN_ERR "❌ %d TESTS FAILED ❌\n", tests_failed);
    }
    
    printk(KERN_INFO "================================================\n");
    
    /* Return success regardless of test results for module loading */
    return 0;
}

static void __exit test_enhanced_registration_exit(void)
{
    printk(KERN_INFO "VexFS Enhanced Registration Test Suite Unloaded\n");
}

module_init(test_enhanced_registration_init);
module_exit(test_enhanced_registration_exit);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 Enhanced Registration Test Suite");
MODULE_VERSION("2.0.0");