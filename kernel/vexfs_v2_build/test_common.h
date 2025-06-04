/**
 * VexFS v2 Common Test Utilities
 * 
 * This header provides shared test utility functions to eliminate
 * code duplication across test files.
 */

#ifndef VEXFS_TEST_COMMON_H
#define VEXFS_TEST_COMMON_H

#include <stdio.h>
#include <stdint.h>
#include <linux/types.h>
#include "vexfs_v2_uapi.h"
#include "vexfs_v2_search.h"

/**
 * Print search results in a standardized format
 * @param results: Array of search results
 * @param count: Number of results to print
 */
void print_search_results(const struct vexfs_search_result *results, uint32_t count);

/**
 * Print a formatted test header
 * @param test_name: Name of the test being executed
 */
void print_test_header(const char *test_name);

/**
 * Print test success message
 * @param test_name: Name of the test that succeeded
 */
void print_test_success(const char *test_name);

/**
 * Print test failure message
 * @param test_name: Name of the test that failed
 * @param error_msg: Error message to display
 */
void print_test_failure(const char *test_name, const char *error_msg);

/**
 * Print test separator for better readability
 */
void print_test_separator(void);

#endif /* VEXFS_TEST_COMMON_H */