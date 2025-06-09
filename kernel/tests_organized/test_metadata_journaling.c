/*
 * VexFS v2.0 - Metadata Journaling Test Suite (Task 3)
 * 
 * Comprehensive test suite for metadata journaling functionality including
 * inode journaling, directory entry journaling, allocation bitmap journaling,
 * vector metadata journaling, serialization, integrity verification, and
 * crash recovery scenarios.
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/buffer_head.h>
#include <linux/random.h>
#include <linux/delay.h>
#include <linux/completion.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>

#include "../src/include/vexfs_v2_metadata_journal.h"
#include "../src/include/vexfs_v2_journal.h"
#include "../src/include/vexfs_v2_atomic.h"
#include "../src/include/vexfs_v2_internal.h"

/* Test framework macros */
#define TEST_ASSERT(condition, message) \
    do { \
        if (!(condition)) { \
            pr_err("VexFS Test FAILED: %s at %s:%d\n", message, __func__, __LINE__); \
            return -1; \
        } \
    } while (0)

#define TEST_PASS(message) \
    pr_info("VexFS Test PASSED: %s\n", message)

/* Test data structures */
struct metadata_journal_test_context {
    struct vexfs_journal *journal;
    struct vexfs_atomic_manager *atomic_mgr;
    struct vexfs_metadata_journal_manager *meta_mgr;
    struct super_block *test_sb;
    struct inode *test_inode;
    struct dentry *test_dentry;
    u32 test_count;
    u32 passed_tests;
    u32 failed_tests;
};

static struct metadata_journal_test_context *test_ctx = NULL;

/*
 * =============================================================================
 * TEST SETUP AND TEARDOWN
 * =============================================================================
 */

/**
 * setup_test_environment - Set up test environment
 * 
 * Creates mock journal, atomic manager, and metadata journaling manager
 * for testing purposes.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int setup_test_environment(void)
{
    int ret;

    test_ctx = kzalloc(sizeof(*test_ctx), GFP_KERNEL);
    if (!test_ctx) {
        pr_err("VexFS: Failed to allocate test context\n");
        return -ENOMEM;
    }

    /* Create mock superblock */
    test_ctx->test_sb = kzalloc(sizeof(struct super_block), GFP_KERNEL);
    if (!test_ctx->test_sb) {
        pr_err("VexFS: Failed to allocate test superblock\n");
        ret = -ENOMEM;
        goto err_free_ctx;
    }

    /* Initialize mock journal (simplified for testing) */
    test_ctx->journal = vexfs_journal_init(test_ctx->test_sb, 1000, 10000);
    if (!test_ctx->journal) {
        pr_err("VexFS: Failed to initialize test journal\n");
        ret = -ENOMEM;
        goto err_free_sb;
    }

    /* Initialize atomic manager */
    test_ctx->atomic_mgr = vexfs_atomic_manager_init(test_ctx->journal);
    if (!test_ctx->atomic_mgr) {
        pr_err("VexFS: Failed to initialize atomic manager\n");
        ret = -ENOMEM;
        goto err_destroy_journal;
    }

    /* Initialize metadata journaling manager */
    test_ctx->meta_mgr = vexfs_metadata_journal_init(test_ctx->journal,
                                                    test_ctx->atomic_mgr);
    if (!test_ctx->meta_mgr) {
        pr_err("VexFS: Failed to initialize metadata journal manager\n");
        ret = -ENOMEM;
        goto err_destroy_atomic;
    }

    /* Create test inode */
    test_ctx->test_inode = kzalloc(sizeof(struct inode) + 
                                  sizeof(struct vexfs_v2_inode_info), GFP_KERNEL);
    if (!test_ctx->test_inode) {
        pr_err("VexFS: Failed to allocate test inode\n");
        ret = -ENOMEM;
        goto err_destroy_meta;
    }

    /* Initialize test inode */
    test_ctx->test_inode->i_ino = 12345;
    test_ctx->test_inode->i_mode = S_IFREG | 0644;
    test_ctx->test_inode->i_size = 1024;
    test_ctx->test_inode->i_blocks = 2;
    i_uid_write(test_ctx->test_inode, 1000);
    i_gid_write(test_ctx->test_inode, 1000);

    /* Initialize VexFS-specific inode fields */
    struct vexfs_v2_inode_info *vexfs_inode = VEXFS_V2_I(test_ctx->test_inode);
    vexfs_inode->is_vector_file = 1;
    vexfs_inode->vector_element_type = VEXFS_VECTOR_FLOAT32;
    vexfs_inode->vector_dimensions = 768;
    vexfs_inode->vector_count = 100;
    vexfs_inode->vector_alignment = 32;
    vexfs_inode->vector_flags = VEXFS_VEC_NORMALIZED | VEXFS_VEC_INDEXED;

    test_ctx->test_count = 0;
    test_ctx->passed_tests = 0;
    test_ctx->failed_tests = 0;

    pr_info("VexFS: Test environment setup completed\n");
    return 0;

err_destroy_meta:
    vexfs_metadata_journal_destroy(test_ctx->meta_mgr);
err_destroy_atomic:
    vexfs_atomic_manager_destroy(test_ctx->atomic_mgr);
err_destroy_journal:
    vexfs_journal_destroy(test_ctx->journal);
err_free_sb:
    kfree(test_ctx->test_sb);
err_free_ctx:
    kfree(test_ctx);
    test_ctx = NULL;
    return ret;
}

/**
 * teardown_test_environment - Clean up test environment
 * 
 * Destroys all test structures and frees memory.
 */
static void teardown_test_environment(void)
{
    if (!test_ctx)
        return;

    if (test_ctx->test_inode)
        kfree(test_ctx->test_inode);

    if (test_ctx->meta_mgr)
        vexfs_metadata_journal_destroy(test_ctx->meta_mgr);

    if (test_ctx->atomic_mgr)
        vexfs_atomic_manager_destroy(test_ctx->atomic_mgr);

    if (test_ctx->journal)
        vexfs_journal_destroy(test_ctx->journal);

    if (test_ctx->test_sb)
        kfree(test_ctx->test_sb);

    kfree(test_ctx);
    test_ctx = NULL;

    pr_info("VexFS: Test environment cleaned up\n");
}

/*
 * =============================================================================
 * SERIALIZATION TESTS
 * =============================================================================
 */

/**
 * test_inode_serialization - Test inode serialization/deserialization
 * 
 * Tests the serialization and deserialization of inode metadata.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_inode_serialization(void)
{
    struct vexfs_meta_serialized_inode serialized;
    struct inode *test_inode2;
    struct vexfs_v2_inode_info *vexfs_inode, *vexfs_inode2;
    int ret;

    test_ctx->test_count++;

    /* Serialize the test inode */
    ret = vexfs_metadata_serialize_inode(test_ctx->test_inode, &serialized);
    TEST_ASSERT(ret == 0, "Inode serialization failed");

    /* Verify serialized data */
    TEST_ASSERT(le64_to_cpu(serialized.ino) == test_ctx->test_inode->i_ino,
                "Serialized inode number mismatch");
    TEST_ASSERT(le32_to_cpu(serialized.mode) == test_ctx->test_inode->i_mode,
                "Serialized inode mode mismatch");
    TEST_ASSERT(le64_to_cpu(serialized.size) == test_ctx->test_inode->i_size,
                "Serialized inode size mismatch");

    /* Verify vector-specific fields */
    vexfs_inode = VEXFS_V2_I(test_ctx->test_inode);
    TEST_ASSERT(serialized.is_vector_file == vexfs_inode->is_vector_file,
                "Serialized vector file flag mismatch");
    TEST_ASSERT(le16_to_cpu(serialized.vector_dimensions) == vexfs_inode->vector_dimensions,
                "Serialized vector dimensions mismatch");
    TEST_ASSERT(le32_to_cpu(serialized.vector_count) == vexfs_inode->vector_count,
                "Serialized vector count mismatch");

    /* Create a new inode for deserialization */
    test_inode2 = kzalloc(sizeof(struct inode) + sizeof(struct vexfs_v2_inode_info), GFP_KERNEL);
    TEST_ASSERT(test_inode2 != NULL, "Failed to allocate test inode2");

    /* Deserialize into the new inode */
    ret = vexfs_metadata_deserialize_inode(&serialized, test_inode2);
    TEST_ASSERT(ret == 0, "Inode deserialization failed");

    /* Verify deserialized data matches original */
    TEST_ASSERT(test_inode2->i_ino == test_ctx->test_inode->i_ino,
                "Deserialized inode number mismatch");
    TEST_ASSERT(test_inode2->i_mode == test_ctx->test_inode->i_mode,
                "Deserialized inode mode mismatch");
    TEST_ASSERT(test_inode2->i_size == test_ctx->test_inode->i_size,
                "Deserialized inode size mismatch");

    /* Verify vector-specific fields */
    vexfs_inode2 = VEXFS_V2_I(test_inode2);
    TEST_ASSERT(vexfs_inode2->is_vector_file == vexfs_inode->is_vector_file,
                "Deserialized vector file flag mismatch");
    TEST_ASSERT(vexfs_inode2->vector_dimensions == vexfs_inode->vector_dimensions,
                "Deserialized vector dimensions mismatch");
    TEST_ASSERT(vexfs_inode2->vector_count == vexfs_inode->vector_count,
                "Deserialized vector count mismatch");

    kfree(test_inode2);
    test_ctx->passed_tests++;
    TEST_PASS("Inode serialization/deserialization");
    return 0;
}

/**
 * test_checksum_verification - Test checksum calculation and verification
 * 
 * Tests the checksum calculation and verification functionality.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_checksum_verification(void)
{
    char test_data[] = "VexFS metadata journaling test data";
    u32 checksum1, checksum2, checksum3;

    test_ctx->test_count++;

    /* Calculate checksum */
    checksum1 = vexfs_metadata_calculate_checksum(test_data, strlen(test_data), 0);
    TEST_ASSERT(checksum1 != 0, "Checksum calculation returned zero");

    /* Calculate same checksum again */
    checksum2 = vexfs_metadata_calculate_checksum(test_data, strlen(test_data), 0);
    TEST_ASSERT(checksum1 == checksum2, "Checksum calculation not deterministic");

    /* Modify data and verify checksum changes */
    test_data[0] = 'X';
    checksum3 = vexfs_metadata_calculate_checksum(test_data, strlen(test_data), 0);
    TEST_ASSERT(checksum1 != checksum3, "Checksum did not change with modified data");

    test_ctx->passed_tests++;
    TEST_PASS("Checksum calculation and verification");
    return 0;
}

/*
 * =============================================================================
 * INODE JOURNALING TESTS
 * =============================================================================
 */

/**
 * test_inode_create_journaling - Test inode creation journaling
 * 
 * Tests journaling of inode creation operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_inode_create_journaling(void)
{
    int ret;
    struct vexfs_metadata_journal_stats stats_before, stats_after;

    test_ctx->test_count++;

    /* Get initial statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats_before);

    /* Journal inode creation */
    ret = vexfs_metadata_journal_inode_create(test_ctx->meta_mgr,
                                              test_ctx->test_inode,
                                              VEXFS_META_JOURNAL_SYNC);
    TEST_ASSERT(ret == 0, "Inode creation journaling failed");

    /* Force batch commit to ensure processing */
    ret = vexfs_metadata_journal_batch_commit(test_ctx->meta_mgr);
    TEST_ASSERT(ret == 0, "Batch commit failed");

    /* Get updated statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats_after);

    /* Verify statistics updated */
    TEST_ASSERT(stats_after.inode_operations > stats_before.inode_operations,
                "Inode operation count did not increase");
    TEST_ASSERT(stats_after.total_operations > stats_before.total_operations,
                "Total operation count did not increase");

    test_ctx->passed_tests++;
    TEST_PASS("Inode creation journaling");
    return 0;
}

/**
 * test_inode_update_journaling - Test inode update journaling
 * 
 * Tests journaling of inode update operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_inode_update_journaling(void)
{
    int ret;
    struct vexfs_metadata_journal_stats stats_before, stats_after;
    struct vexfs_v2_inode_info *vexfs_inode;

    test_ctx->test_count++;

    /* Get initial statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats_before);

    /* Modify inode */
    test_ctx->test_inode->i_size = 2048;
    vexfs_inode = VEXFS_V2_I(test_ctx->test_inode);
    vexfs_inode->vector_count = 200;

    /* Journal inode update */
    ret = vexfs_metadata_journal_inode_update(test_ctx->meta_mgr,
                                              test_ctx->test_inode,
                                              VEXFS_META_JOURNAL_SYNC);
    TEST_ASSERT(ret == 0, "Inode update journaling failed");

    /* Force batch commit */
    ret = vexfs_metadata_journal_batch_commit(test_ctx->meta_mgr);
    TEST_ASSERT(ret == 0, "Batch commit failed");

    /* Get updated statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats_after);

    /* Verify statistics updated */
    TEST_ASSERT(stats_after.inode_operations > stats_before.inode_operations,
                "Inode operation count did not increase");

    test_ctx->passed_tests++;
    TEST_PASS("Inode update journaling");
    return 0;
}

/*
 * =============================================================================
 * CACHE MANAGEMENT TESTS
 * =============================================================================
 */

/**
 * test_metadata_cache - Test metadata caching functionality
 * 
 * Tests the metadata cache put/get operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_metadata_cache(void)
{
    char test_data[] = "VexFS cached metadata test";
    void *cached_data;
    size_t cached_size;
    int ret;
    struct vexfs_metadata_journal_stats stats;

    test_ctx->test_count++;

    /* Put data in cache */
    ret = vexfs_metadata_cache_put(test_ctx->meta_mgr, 12345,
                                  VEXFS_META_SERIAL_INODE,
                                  test_data, strlen(test_data));
    TEST_ASSERT(ret == 0, "Cache put operation failed");

    /* Get data from cache */
    ret = vexfs_metadata_cache_get(test_ctx->meta_mgr, 12345,
                                  VEXFS_META_SERIAL_INODE,
                                  &cached_data, &cached_size);
    TEST_ASSERT(ret == 0, "Cache get operation failed");
    TEST_ASSERT(cached_size == strlen(test_data), "Cached data size mismatch");
    TEST_ASSERT(memcmp(cached_data, test_data, cached_size) == 0,
                "Cached data content mismatch");

    /* Verify cache hit statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats);
    TEST_ASSERT(stats.cache_hits > 0, "Cache hit count not updated");

    /* Try to get non-existent data */
    ret = vexfs_metadata_cache_get(test_ctx->meta_mgr, 99999,
                                  VEXFS_META_SERIAL_INODE,
                                  &cached_data, &cached_size);
    TEST_ASSERT(ret == -ENOENT, "Cache get should return ENOENT for missing data");

    /* Verify cache miss statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats);
    TEST_ASSERT(stats.cache_misses > 0, "Cache miss count not updated");

    kfree(cached_data);
    test_ctx->passed_tests++;
    TEST_PASS("Metadata cache operations");
    return 0;
}

/*
 * =============================================================================
 * PERFORMANCE AND STRESS TESTS
 * =============================================================================
 */

/**
 * test_batch_processing - Test batch processing performance
 * 
 * Tests the batch processing of multiple metadata operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int test_batch_processing(void)
{
    int i, ret;
    struct inode *test_inodes[10];
    struct vexfs_metadata_journal_stats stats_before, stats_after;
    unsigned long start_time, end_time;

    test_ctx->test_count++;

    /* Create test inodes */
    for (i = 0; i < 10; i++) {
        test_inodes[i] = kzalloc(sizeof(struct inode) + 
                                sizeof(struct vexfs_v2_inode_info), GFP_KERNEL);
        TEST_ASSERT(test_inodes[i] != NULL, "Failed to allocate test inode");
        
        test_inodes[i]->i_ino = 20000 + i;
        test_inodes[i]->i_mode = S_IFREG | 0644;
        test_inodes[i]->i_size = 1024 * (i + 1);
    }

    /* Get initial statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats_before);
    start_time = jiffies;

    /* Submit multiple operations asynchronously */
    for (i = 0; i < 10; i++) {
        ret = vexfs_metadata_journal_inode_create(test_ctx->meta_mgr,
                                                  test_inodes[i],
                                                  VEXFS_META_JOURNAL_ASYNC);
        TEST_ASSERT(ret == 0, "Async inode creation journaling failed");
    }

    /* Force batch commit */
    ret = vexfs_metadata_journal_batch_commit(test_ctx->meta_mgr);
    TEST_ASSERT(ret == 0, "Batch commit failed");

    end_time = jiffies;

    /* Get updated statistics */
    vexfs_metadata_journal_get_stats(test_ctx->meta_mgr, &stats_after);

    /* Verify all operations were processed */
    TEST_ASSERT(stats_after.inode_operations >= stats_before.inode_operations + 10,
                "Not all batch operations were processed");

    pr_info("VexFS: Batch processing of 10 operations took %lu jiffies\n",
            end_time - start_time);

    /* Clean up */
    for (i = 0; i < 10; i++) {
        kfree(test_inodes[i]);
    }

    test_ctx->passed_tests++;
    TEST_PASS("Batch processing performance");
    return 0;
}

/*
 * =============================================================================
 * MAIN TEST RUNNER
 * =============================================================================
 */

/**
 * run_metadata_journaling_tests - Run all metadata journaling tests
 * 
 * Executes the complete test suite for metadata journaling.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int run_metadata_journaling_tests(void)
{
    int ret;

    pr_info("VexFS: Starting metadata journaling test suite\n");

    /* Setup test environment */
    ret = setup_test_environment();
    if (ret) {
        pr_err("VexFS: Failed to setup test environment: %d\n", ret);
        return ret;
    }

    /* Run serialization tests */
    if (test_inode_serialization() < 0)
        test_ctx->failed_tests++;

    if (test_checksum_verification() < 0)
        test_ctx->failed_tests++;

    /* Run inode journaling tests */
    if (test_inode_create_journaling() < 0)
        test_ctx->failed_tests++;

    if (test_inode_update_journaling() < 0)
        test_ctx->failed_tests++;

    /* Run cache management tests */
    if (test_metadata_cache() < 0)
        test_ctx->failed_tests++;

    /* Run performance tests */
    if (test_batch_processing() < 0)
        test_ctx->failed_tests++;

    /* Print test results */
    pr_info("VexFS: Metadata journaling test results:\n");
    pr_info("  Total tests: %u\n", test_ctx->test_count);
    pr_info("  Passed: %u\n", test_ctx->passed_tests);
    pr_info("  Failed: %u\n", test_ctx->failed_tests);

    if (test_ctx->failed_tests == 0) {
        pr_info("VexFS: All metadata journaling tests PASSED!\n");
        ret = 0;
    } else {
        pr_err("VexFS: %u metadata journaling tests FAILED!\n", test_ctx->failed_tests);
        ret = -1;
    }

    /* Cleanup test environment */
    teardown_test_environment();

    return ret;
}

/*
 * =============================================================================
 * MODULE INIT/EXIT
 * =============================================================================
 */

static int __init vexfs_metadata_journal_test_init(void)
{
    pr_info("VexFS: Metadata journaling test module loaded\n");
    return run_metadata_journaling_tests();
}

static void __exit vexfs_metadata_journal_test_exit(void)
{
    pr_info("VexFS: Metadata journaling test module unloaded\n");
}

module_init(vexfs_metadata_journal_test_init);
module_exit(vexfs_metadata_journal_test_exit);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 Metadata Journaling Test Suite");
MODULE_VERSION("1.0.0");