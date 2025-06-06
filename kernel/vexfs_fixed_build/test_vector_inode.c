/*
 * VexFS Vector-Enhanced Inode Test Program
 * 
 * This test program validates the vector-enhanced inode structure
 * and operations for VexFS Task 41 implementation.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <assert.h>

/* Include our test-compatible vector inode definitions */
#include "vexfs_vector_inode_test.h"

/* Test helper functions */
static void test_vector_metadata_initialization(void)
{
    struct vexfs_vector_metadata meta;
    
    printf("Testing vector metadata initialization...\n");
    
    vexfs_init_vector_metadata(&meta);
    
    assert(meta.element_type == VEXFS_VECTOR_UNKNOWN);
    assert(meta.simd_alignment == VEXFS_SIMD_ALIGN_16);
    assert(meta.vector_dimension == 0);
    assert(meta.vexfs_flags == 0);
    
    printf("✓ Vector metadata initialization test passed\n");
}

static void test_vector_element_sizes(void)
{
    printf("Testing vector element size calculations...\n");
    
    assert(vexfs_vector_element_size(VEXFS_VECTOR_INT8) == 1);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_UINT8) == 1);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_INT16) == 2);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_UINT16) == 2);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_BFLOAT16) == 2);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_FLOAT16) == 2);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_INT32) == 4);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_UINT32) == 4);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_FLOAT32) == 4);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_FLOAT64) == 8);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_BINARY) == 1);
    assert(vexfs_vector_element_size(VEXFS_VECTOR_SPARSE) == 0);
    
    printf("✓ Vector element size test passed\n");
}

static void test_vector_data_size_calculations(void)
{
    struct vexfs_vector_metadata meta;
    
    printf("Testing vector data size calculations...\n");
    
    /* Test FLOAT32 vector */
    vexfs_init_vector_metadata(&meta);
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_dimension = 768;  /* Common embedding dimension */
    
    size_t expected_size = 768 * 4;  /* 768 * sizeof(float32) */
    assert(vexfs_vector_data_size(&meta) == expected_size);
    
    /* Test binary vector */
    meta.element_type = VEXFS_VECTOR_BINARY;
    meta.vector_dimension = 1024;
    
    expected_size = (1024 + 7) / 8;  /* 1024 bits = 128 bytes */
    assert(vexfs_vector_data_size(&meta) == expected_size);
    
    /* Test sparse vector */
    meta.element_type = VEXFS_VECTOR_SPARSE;
    meta.original_size = 2048;
    
    assert(vexfs_vector_data_size(&meta) == 2048);
    
    printf("✓ Vector data size calculation test passed\n");
}

static void test_vector_flags(void)
{
    struct vexfs_vector_metadata meta;
    
    printf("Testing vector flags...\n");
    
    vexfs_init_vector_metadata(&meta);
    
    /* Test flag setting */
    meta.vexfs_flags |= VEXFS_VECTOR_FLAG_NORMALIZED;
    assert(vexfs_is_vector_normalized(&meta) == true);
    
    meta.vexfs_flags |= VEXFS_VECTOR_FLAG_INDEXED;
    assert(vexfs_is_vector_indexed(&meta) == true);
    
    meta.vexfs_flags |= VEXFS_VECTOR_FLAG_COMPRESSED;
    assert(vexfs_is_vector_compressed(&meta) == true);
    
    /* Test flag clearing */
    meta.vexfs_flags &= ~VEXFS_VECTOR_FLAG_NORMALIZED;
    assert(vexfs_is_vector_normalized(&meta) == false);
    
    printf("✓ Vector flags test passed\n");
}

static void test_vector_validation(void)
{
    struct vexfs_vector_metadata meta;
    
    printf("Testing vector metadata validation...\n");
    
    /* Test valid metadata */
    vexfs_init_vector_metadata(&meta);
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_dimension = 768;
    meta.simd_alignment = VEXFS_SIMD_ALIGN_32;
    
    assert(vexfs_validate_vector_metadata(&meta) == true);
    
    /* Test invalid dimension (zero) */
    meta.vector_dimension = 0;
    assert(vexfs_validate_vector_metadata(&meta) == false);
    
    /* Test invalid dimension (too large) */
    meta.vector_dimension = VEXFS_MAX_VECTOR_DIMENSIONS + 1;
    assert(vexfs_validate_vector_metadata(&meta) == false);
    
    /* Test invalid element type */
    meta.vector_dimension = 768;
    meta.element_type = 99;  /* Invalid type */
    assert(vexfs_validate_vector_metadata(&meta) == false);
    
    /* Test invalid SIMD alignment */
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.simd_alignment = 48;  /* Invalid alignment */
    assert(vexfs_validate_vector_metadata(&meta) == false);
    
    printf("✓ Vector validation test passed\n");
}

static void test_common_vector_configurations(void)
{
    struct vexfs_vector_metadata meta;
    
    printf("Testing common vector configurations...\n");
    
    /* Test OpenAI text-embedding-3-small (1536D) */
    vexfs_init_vector_metadata(&meta);
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_dimension = 1536;
    meta.simd_alignment = VEXFS_SIMD_ALIGN_32;
    meta.vexfs_flags = VEXFS_VECTOR_FLAG_NORMALIZED;
    
    assert(vexfs_validate_vector_metadata(&meta) == true);
    assert(vexfs_vector_data_size(&meta) == 1536 * 4);
    assert(vexfs_is_vector_normalized(&meta) == true);
    
    /* Test Ollama nomic-embed-text (768D) */
    meta.vector_dimension = 768;
    meta.vexfs_flags |= VEXFS_VECTOR_FLAG_INDEXED;
    
    assert(vexfs_validate_vector_metadata(&meta) == true);
    assert(vexfs_vector_data_size(&meta) == 768 * 4);
    assert(vexfs_is_vector_indexed(&meta) == true);
    
    /* Test quantized vector (INT8) */
    meta.element_type = VEXFS_VECTOR_INT8;
    meta.vector_dimension = 1024;
    meta.vexfs_flags = VEXFS_VECTOR_FLAG_QUANTIZED | VEXFS_VECTOR_FLAG_COMPRESSED;
    
    assert(vexfs_validate_vector_metadata(&meta) == true);
    assert(vexfs_vector_data_size(&meta) == 1024 * 1);
    assert(vexfs_is_vector_compressed(&meta) == true);
    
    printf("✓ Common vector configurations test passed\n");
}

static void test_performance_metadata(void)
{
    struct vexfs_vector_metadata meta;
    
    printf("Testing performance metadata fields...\n");
    
    vexfs_init_vector_metadata(&meta);
    meta.element_type = VEXFS_VECTOR_FLOAT32;
    meta.vector_dimension = 768;
    
    /* Test compression metadata */
    meta.vexfs_flags |= VEXFS_VECTOR_FLAG_COMPRESSED;
    meta.compression_ratio = 75;  /* 75% compression */
    meta.original_size = vexfs_vector_data_size(&meta);
    
    assert(meta.compression_ratio == 75);
    assert(meta.original_size == 768 * 4);
    
    /* Test access tracking */
    meta.access_count = 0;
    meta.last_access_time = 1704067200;  /* Example timestamp */
    
    /* Simulate access */
    meta.access_count++;
    meta.last_access_time = 1704067260;  /* 1 minute later */
    
    assert(meta.access_count == 1);
    assert(meta.last_access_time == 1704067260);
    
    printf("✓ Performance metadata test passed\n");
}

static void print_test_summary(void)
{
    printf("\n=== VexFS Vector-Enhanced Inode Test Summary ===\n");
    printf("✓ All tests passed successfully!\n");
    printf("\nImplemented features:\n");
    printf("  • Vector metadata structure with 12 element types\n");
    printf("  • SIMD alignment support (16/32/64-byte)\n");
    printf("  • Vector property flags (8 different flags)\n");
    printf("  • Data size calculations for all vector types\n");
    printf("  • Metadata validation with bounds checking\n");
    printf("  • Performance tracking (access count, timestamps)\n");
    printf("  • Compression metadata support\n");
    printf("  • Support for dimensions up to 65,535\n");
    printf("\nSupported vector types:\n");
    printf("  • FLOAT32, FLOAT64, FLOAT16, BFLOAT16\n");
    printf("  • INT8, UINT8, INT16, UINT16, INT32, UINT32\n");
    printf("  • BINARY, SPARSE vectors\n");
    printf("\nReady for integration with VexFS v2 kernel module!\n");
}

int main(void)
{
    printf("VexFS Vector-Enhanced Inode Test Suite\n");
    printf("======================================\n\n");
    
    test_vector_metadata_initialization();
    test_vector_element_sizes();
    test_vector_data_size_calculations();
    test_vector_flags();
    test_vector_validation();
    test_common_vector_configurations();
    test_performance_metadata();
    
    print_test_summary();
    
    return 0;
}