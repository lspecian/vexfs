/**
 * VexFS v2 Common Test Utilities Implementation
 *
 * This file implements shared test utility functions to eliminate
 * code duplication across test files.
 */

#include "test_common.h"
#include "vexfs_v2_search.h"

/**
 * Print search results in a standardized format
 */
void print_search_results(const struct vexfs_search_result *results, uint32_t count) {
    printf("📊 Search Results (%u found):\n", count);
    for (uint32_t i = 0; i < count; i++) {
        printf("  [%u] Vector ID: %lu, Distance: %u\n", 
               i, results[i].vector_id, results[i].distance);
    }
}

/**
 * Print a formatted test header
 */
void print_test_header(const char *test_name) {
    printf("\n" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "\n");
    printf("🧪 %s\n", test_name);
    printf("=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "=" "\n");
}

/**
 * Print test success message
 */
void print_test_success(const char *test_name) {
    printf("✅ %s: PASSED\n", test_name);
}

/**
 * Print test failure message
 */
void print_test_failure(const char *test_name, const char *error_msg) {
    printf("❌ %s: FAILED - %s\n", test_name, error_msg);
}

/**
 * Print test separator for better readability
 */
void print_test_separator(void) {
    printf("\n" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "-" "\n");
}