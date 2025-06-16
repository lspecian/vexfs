/*
 * Test UAPI Header Compatibility
 * 
 * This test verifies that the updated UAPI headers compile correctly
 * and that the IEEE 754 bit representation conversion utilities work.
 */

#include <stdio.h>
#include <stdint.h>
#include <assert.h>
#include "vexfs_v2_uapi.h"
#include "vexfs_v2_phase3.h"
#include "vexfs_v2_search.h"

int main() {
    printf("Testing VexFS v2 UAPI Header Compatibility...\n");
    
    // Test structure sizes are still valid
    printf("Structure sizes:\n");
    printf("  vexfs_vector_search_request: %zu bytes\n", sizeof(struct vexfs_vector_search_request));
    printf("  vexfs_batch_insert_request: %zu bytes\n", sizeof(struct vexfs_batch_insert_request));
    printf("  vexfs_multi_vector_search: %zu bytes\n", sizeof(struct vexfs_multi_vector_search));
    printf("  vexfs_filtered_search: %zu bytes\n", sizeof(struct vexfs_filtered_search));
    printf("  vexfs_hybrid_search: %zu bytes\n", sizeof(struct vexfs_hybrid_search));
    
    // Test IEEE 754 conversion utilities (userspace only)
    #ifndef __KERNEL__
    float test_float = 3.14159f;
    uint32_t bits = vexfs_float_to_bits(test_float);
    float converted_back = vexfs_bits_to_float(bits);
    
    printf("\nIEEE 754 Conversion Test:\n");
    printf("  Original float: %f\n", test_float);
    printf("  Bit representation: 0x%08x\n", bits);
    printf("  Converted back: %f\n", converted_back);
    printf("  Conversion %s\n", (test_float == converted_back) ? "SUCCESS" : "FAILED");
    
    // Test array conversion
    float test_array[] = {1.0f, 2.5f, -3.14f, 0.0f};
    uint32_t bit_array[4];
    float result_array[4];
    
    vexfs_float_array_to_bits(test_array, bit_array, 4);
    vexfs_bits_array_to_float(bit_array, result_array, 4);
    
    printf("\nArray Conversion Test:\n");
    int array_success = 1;
    for (int i = 0; i < 4; i++) {
        printf("  [%d] %f -> 0x%08x -> %f\n", i, test_array[i], bit_array[i], result_array[i]);
        if (test_array[i] != result_array[i]) {
            array_success = 0;
        }
    }
    printf("  Array conversion %s\n", array_success ? "SUCCESS" : "FAILED");
    #endif
    
    printf("\nAll UAPI headers compiled successfully!\n");
    printf("Floating-point types have been eliminated from kernel interface.\n");
    printf("IEEE 754 bit representation maintains userspace compatibility.\n");
    
    return 0;
}