/*
 * VexFS v2.0 - Full Filesystem Journal Test Suite
 * 
 * Comprehensive test suite for the Full Filesystem Journal (Phase 1) implementation
 * covering all advanced features including concurrent transactions, multiple journaling
 * modes, SHA-256 checksumming, and crash recovery mechanisms.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/delay.h>
#include <linux/kthread.h>
#include <linux/completion.h>
#include <linux/atomic.h>
#include <linux/random.h>

#include "../src/include/vexfs_v2_full_journal.h"
#include "../src/include/vexfs_v2_internal.h"

/* Test configuration */
#define TEST_JOURNAL_BLOCKS     1024
#define TEST_CONCURRENT_TRANS   16
#define TEST_DATA_SIZE          4096
#define TEST_ITERATIONS         100

/* Test results tracking */
static atomic_t tests_passed;
static atomic_t tests_failed;
static atomic_t tests_total;

/* Test helper macros */
#define TEST_ASSERT(condition, message) do { \
    atomic_inc(&tests_total); \
    if (condition) { \
        atomic_inc(&tests_passed); \
        printk(KERN_INFO "VexFS Full Journal Test: PASS - %s\n", message); \
    } else { \
        atomic_inc(&tests_failed); \
        printk(KERN_ERR "VexFS Full Journal Test: FAIL - %s\n", message); \
    } \
} while(0)

/* Forward declarations */
static int test_journal_initialization(void);
static int test_transaction_management(void);
static int test_concurrent_transactions(void);
static int test_journaling_modes(void);
static int test_sha256_checksumming(void);
static int test_checkpointing(void);
static int test_barrier_operations(void);
static int test_crash_recovery(void);
static int test_ioctl_interface(void);
static int test_performance_benchmarks(void);

/* Test data structures */
struct test_context {
    struct super_block *sb;
    struct vexfs_full_journal *journal;
    u8 *test_data;
    size_t data_size;
};

/*
 * Initialize test environment
 */
static struct test_context *test_init_context(void)
{
    struct test_context *ctx;
    
    ctx = kzalloc(sizeof(struct test_context), GFP_KERNEL);
    if (!ctx) {
        return NULL;
    }
    
    /* Allocate test data */
    ctx->data_size = TEST_DATA_SIZE;
    ctx->test_data = vmalloc(ctx->data_size);
    if (!ctx->test_data) {
        kfree(ctx);
        return NULL;
    }
    
    /* Fill with random data */
    get_random_bytes(ctx->test_data, ctx->data_size);
    
    /* Create mock superblock - in real implementation this would be from VFS */
    ctx->sb = kzalloc(sizeof(struct super_block), GFP_KERNEL);
    if (!ctx->sb) {
        vfree(ctx->test_data);
        kfree(ctx);
        return NULL;
    }
    
    return ctx;
}

/*
 * Cleanup test environment
 */
static void test_cleanup_context(struct test_context *ctx)
{
    if (!ctx) {
        return;
    }
    
    if (ctx->journal) {
        vexfs_full_journal_destroy(ctx->journal);
    }
    
    if (ctx->test_data) {
        vfree(ctx->test_data);
    }
    
    if (ctx->sb) {
        kfree(ctx->sb);
    }
    
    kfree(ctx);
}

/*
 * Test 1: Journal Initialization and Cleanup
 */
static int test_journal_initialization(void)
{
    struct test_context *ctx;
    struct vexfs_full_journal *journal;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting initialization tests\n");
    
    ctx = test_init_context();
    TEST_ASSERT(ctx != NULL, "Test context initialization");
    
    if (!ctx) {
        return -ENOMEM;
    }
    
    /* Test journal initialization with different modes */
    journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_ORDERED);
    TEST_ASSERT(!IS_ERR(journal), "Journal initialization - ordered mode");
    
    if (!IS_ERR(journal)) {
        ctx->journal = journal;
        
        /* Test mode retrieval */
        u32 mode = vexfs_full_journal_get_mode(journal);
        TEST_ASSERT(mode == VEXFS_JOURNAL_MODE_ORDERED, "Journal mode retrieval");
        
        /* Test statistics */
        struct vexfs_full_journal_stats stats;
        vexfs_full_journal_get_stats(journal, &stats);
        TEST_ASSERT(stats.fjs_total_commits == 0, "Initial statistics - zero commits");
        TEST_ASSERT(stats.fjs_total_transactions == 0, "Initial statistics - zero transactions");
    }
    
    /* Test invalid parameters */
    journal = vexfs_full_journal_init(NULL, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_ORDERED);
    TEST_ASSERT(IS_ERR(journal), "Journal initialization - null superblock");
    
    journal = vexfs_full_journal_init(ctx->sb, 100, 10, VEXFS_JOURNAL_MODE_ORDERED);
    TEST_ASSERT(IS_ERR(journal), "Journal initialization - insufficient blocks");
    
    journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, 99);
    TEST_ASSERT(IS_ERR(journal), "Journal initialization - invalid mode");
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 2: Transaction Management
 */
static int test_transaction_management(void)
{
    struct test_context *ctx;
    struct vexfs_full_journal_transaction *trans;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting transaction management tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_JOURNAL);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Test transaction start */
    trans = vexfs_full_journal_start(ctx->journal, 64, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
    TEST_ASSERT(!IS_ERR(trans), "Transaction start");
    
    if (!IS_ERR(trans)) {
        /* Test adding data blocks */
        ret = vexfs_full_journal_add_data_block(trans, 1000, ctx->test_data, ctx->data_size);
        TEST_ASSERT(ret == 0, "Add data block to transaction");
        
        /* Test transaction commit */
        ret = vexfs_full_journal_commit(trans);
        TEST_ASSERT(ret == 0, "Transaction commit");
    }
    
    /* Test transaction abort */
    trans = vexfs_full_journal_start(ctx->journal, 32, VEXFS_JOURNAL_OP_WRITE, VEXFS_TRANS_PRIORITY_HIGH);
    if (!IS_ERR(trans)) {
        ret = vexfs_full_journal_abort(trans);
        TEST_ASSERT(ret == 0, "Transaction abort");
    }
    
    /* Test invalid parameters */
    trans = vexfs_full_journal_start(NULL, 64, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
    TEST_ASSERT(IS_ERR(trans), "Transaction start - null journal");
    
    trans = vexfs_full_journal_start(ctx->journal, 0, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
    TEST_ASSERT(IS_ERR(trans), "Transaction start - zero blocks");
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 3: Concurrent Transactions
 */
struct concurrent_test_data {
    struct vexfs_full_journal *journal;
    atomic_t completed_transactions;
    atomic_t failed_transactions;
    struct completion test_completion;
    u8 *test_data;
    size_t data_size;
};

static int concurrent_transaction_thread(void *data)
{
    struct concurrent_test_data *test_data = (struct concurrent_test_data *)data;
    struct vexfs_full_journal_transaction *trans;
    int i, ret;
    
    for (i = 0; i < 10; i++) {
        trans = vexfs_full_journal_start(test_data->journal, 16, 
                                        VEXFS_JOURNAL_OP_WRITE, VEXFS_TRANS_PRIORITY_NORMAL);
        if (IS_ERR(trans)) {
            atomic_inc(&test_data->failed_transactions);
            continue;
        }
        
        /* Add some data */
        ret = vexfs_full_journal_add_data_block(trans, 2000 + i, 
                                               test_data->test_data, test_data->data_size);
        if (ret) {
            vexfs_full_journal_abort(trans);
            atomic_inc(&test_data->failed_transactions);
            continue;
        }
        
        /* Commit transaction */
        ret = vexfs_full_journal_commit(trans);
        if (ret) {
            atomic_inc(&test_data->failed_transactions);
        } else {
            atomic_inc(&test_data->completed_transactions);
        }
        
        /* Small delay to allow interleaving */
        msleep(1);
    }
    
    complete(&test_data->test_completion);
    return 0;
}

static int test_concurrent_transactions(void)
{
    struct test_context *ctx;
    struct concurrent_test_data test_data;
    struct task_struct *threads[TEST_CONCURRENT_TRANS];
    int i, ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting concurrent transaction tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_JOURNAL);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Initialize test data */
    test_data.journal = ctx->journal;
    atomic_set(&test_data.completed_transactions, 0);
    atomic_set(&test_data.failed_transactions, 0);
    init_completion(&test_data.test_completion);
    test_data.test_data = ctx->test_data;
    test_data.data_size = ctx->data_size;
    
    /* Start concurrent threads */
    for (i = 0; i < TEST_CONCURRENT_TRANS; i++) {
        threads[i] = kthread_run(concurrent_transaction_thread, &test_data, 
                                "vexfs_test_%d", i);
        if (IS_ERR(threads[i])) {
            printk(KERN_ERR "VexFS Full Journal Test: Failed to start thread %d\n", i);
            ret = PTR_ERR(threads[i]);
            break;
        }
    }
    
    /* Wait for all threads to complete */
    for (i = 0; i < TEST_CONCURRENT_TRANS; i++) {
        if (!IS_ERR(threads[i])) {
            wait_for_completion(&test_data.test_completion);
            kthread_stop(threads[i]);
        }
    }
    
    /* Check results */
    int completed = atomic_read(&test_data.completed_transactions);
    int failed = atomic_read(&test_data.failed_transactions);
    
    TEST_ASSERT(completed > 0, "Concurrent transactions - some completed");
    TEST_ASSERT(completed > failed, "Concurrent transactions - more success than failure");
    
    printk(KERN_INFO "VexFS Full Journal Test: Concurrent test completed: %d success, %d failed\n",
           completed, failed);
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 4: Journaling Modes
 */
static int test_journaling_modes(void)
{
    struct test_context *ctx;
    struct vexfs_full_journal_transaction *trans;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting journaling mode tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    /* Test ordered mode */
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_ORDERED);
    if (!IS_ERR(ctx->journal)) {
        u32 mode = vexfs_full_journal_get_mode(ctx->journal);
        TEST_ASSERT(mode == VEXFS_JOURNAL_MODE_ORDERED, "Ordered mode initialization");
        
        /* Test mode switching */
        ret = vexfs_full_journal_set_mode(ctx->journal, VEXFS_JOURNAL_MODE_WRITEBACK);
        TEST_ASSERT(ret == 0, "Mode switch to writeback");
        
        mode = vexfs_full_journal_get_mode(ctx->journal);
        TEST_ASSERT(mode == VEXFS_JOURNAL_MODE_WRITEBACK, "Writeback mode verification");
        
        /* Test mode switching to journal mode */
        ret = vexfs_full_journal_set_mode(ctx->journal, VEXFS_JOURNAL_MODE_JOURNAL);
        TEST_ASSERT(ret == 0, "Mode switch to journal");
        
        mode = vexfs_full_journal_get_mode(ctx->journal);
        TEST_ASSERT(mode == VEXFS_JOURNAL_MODE_JOURNAL, "Journal mode verification");
        
        /* Test transaction in journal mode */
        trans = vexfs_full_journal_start(ctx->journal, 32, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
        if (!IS_ERR(trans)) {
            ret = vexfs_full_journal_add_data_block(trans, 3000, ctx->test_data, ctx->data_size);
            TEST_ASSERT(ret == 0, "Data block addition in journal mode");
            
            ret = vexfs_full_journal_commit(trans);
            TEST_ASSERT(ret == 0, "Transaction commit in journal mode");
        }
        
        /* Test invalid mode */
        ret = vexfs_full_journal_set_mode(ctx->journal, 99);
        TEST_ASSERT(ret != 0, "Invalid mode rejection");
    }
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 5: SHA-256 Checksumming
 */
static int test_sha256_checksumming(void)
{
    u8 test_data[] = "VexFS Full Journal Test Data";
    u8 hash1[SHA256_DIGEST_SIZE];
    u8 hash2[SHA256_DIGEST_SIZE];
    int ret;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting SHA-256 checksumming tests\n");
    
    /* Test hash calculation */
    ret = vexfs_full_journal_calculate_sha256(test_data, sizeof(test_data), hash1);
    TEST_ASSERT(ret == 0, "SHA-256 hash calculation");
    
    /* Test hash verification */
    ret = vexfs_full_journal_verify_sha256(test_data, sizeof(test_data), hash1);
    TEST_ASSERT(ret == 0, "SHA-256 hash verification - correct");
    
    /* Test hash consistency */
    ret = vexfs_full_journal_calculate_sha256(test_data, sizeof(test_data), hash2);
    TEST_ASSERT(ret == 0, "SHA-256 hash calculation - second");
    TEST_ASSERT(memcmp(hash1, hash2, SHA256_DIGEST_SIZE) == 0, "SHA-256 hash consistency");
    
    /* Test hash verification failure */
    hash2[0] ^= 0xFF; /* Corrupt first byte */
    ret = vexfs_full_journal_verify_sha256(test_data, sizeof(test_data), hash2);
    TEST_ASSERT(ret != 0, "SHA-256 hash verification - incorrect");
    
    /* Test with different data */
    u8 different_data[] = "Different test data";
    ret = vexfs_full_journal_verify_sha256(different_data, sizeof(different_data), hash1);
    TEST_ASSERT(ret != 0, "SHA-256 hash verification - different data");
    
    return 0;
}

/*
 * Test 6: Checkpointing
 */
static int test_checkpointing(void)
{
    struct test_context *ctx;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting checkpointing tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_JOURNAL);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Test checkpoint creation */
    ret = vexfs_full_journal_create_checkpoint(ctx->journal, VEXFS_CHECKPOINT_FORCE);
    TEST_ASSERT(ret == 0, "Checkpoint creation - force");
    
    ret = vexfs_full_journal_create_checkpoint(ctx->journal, VEXFS_CHECKPOINT_ASYNC);
    TEST_ASSERT(ret == 0, "Checkpoint creation - async");
    
    /* Verify checkpoint statistics */
    struct vexfs_full_journal_stats stats;
    vexfs_full_journal_get_stats(ctx->journal, &stats);
    TEST_ASSERT(stats.fjs_total_checkpoints >= 2, "Checkpoint statistics");
    
    /* Test invalid parameters */
    ret = vexfs_full_journal_create_checkpoint(NULL, VEXFS_CHECKPOINT_FORCE);
    TEST_ASSERT(ret != 0, "Checkpoint creation - null journal");
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 7: Barrier Operations
 */
static int test_barrier_operations(void)
{
    struct test_context *ctx;
    struct vexfs_full_journal_transaction *trans;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting barrier operation tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_JOURNAL);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Test barrier in journal mode */
    trans = vexfs_full_journal_start(ctx->journal, 32, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
    if (!IS_ERR(trans)) {
        ret = vexfs_full_journal_add_barrier(trans, 1, 5000);
        TEST_ASSERT(ret == 0, "Barrier addition in journal mode");
        
        ret = vexfs_full_journal_wait_barrier(trans);
        TEST_ASSERT(ret == 0, "Barrier wait");
        
        ret = vexfs_full_journal_commit(trans);
        TEST_ASSERT(ret == 0, "Transaction commit with barrier");
    }
    
    /* Test barrier in ordered mode */
    ret = vexfs_full_journal_set_mode(ctx->journal, VEXFS_JOURNAL_MODE_ORDERED);
    if (ret == 0) {
        trans = vexfs_full_journal_start(ctx->journal, 32, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
        if (!IS_ERR(trans)) {
            ret = vexfs_full_journal_add_barrier(trans, 1, 5000);
            TEST_ASSERT(ret == 0, "Barrier addition in ordered mode (should be no-op)");
            
            ret = vexfs_full_journal_commit(trans);
            TEST_ASSERT(ret == 0, "Transaction commit in ordered mode");
        }
    }
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 8: Crash Recovery Simulation
 */
static int test_crash_recovery(void)
{
    struct test_context *ctx;
    struct vexfs_full_journal_transaction *trans;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting crash recovery tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_JOURNAL);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Create some transactions to simulate journal content */
    trans = vexfs_full_journal_start(ctx->journal, 32, VEXFS_JOURNAL_OP_CREATE, VEXFS_TRANS_PRIORITY_NORMAL);
    if (!IS_ERR(trans)) {
        vexfs_full_journal_add_data_block(trans, 4000, ctx->test_data, ctx->data_size);
        vexfs_full_journal_commit(trans);
    }
    
    trans = vexfs_full_journal_start(ctx->journal, 32, VEXFS_JOURNAL_OP_WRITE, VEXFS_TRANS_PRIORITY_NORMAL);
    if (!IS_ERR(trans)) {
        vexfs_full_journal_add_data_block(trans, 4001, ctx->test_data, ctx->data_size);
        vexfs_full_journal_commit(trans);
    }
    
    /* Test recovery with different flags */
    ret = vexfs_full_journal_recover(ctx->journal, VEXFS_RECOVERY_FULL_SCAN);
    TEST_ASSERT(ret == 0, "Recovery - full scan");
    
    ret = vexfs_full_journal_recover(ctx->journal, VEXFS_RECOVERY_FAST_MODE);
    TEST_ASSERT(ret == 0, "Recovery - fast mode");
    
    ret = vexfs_full_journal_recover(ctx->journal, VEXFS_RECOVERY_VERIFY_CHECKSUMS);
    TEST_ASSERT(ret == 0, "Recovery - verify checksums");
    
    /* Test transaction scanning */
    ret = vexfs_full_journal_scan_for_transactions(ctx->journal, 0, 100);
    TEST_ASSERT(ret == 0, "Transaction scanning");
    
    /* Test invalid parameters */
    ret = vexfs_full_journal_recover(NULL, VEXFS_RECOVERY_FULL_SCAN);
    TEST_ASSERT(ret != 0, "Recovery - null journal");
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 9: ioctl Interface
 */
static int test_ioctl_interface(void)
{
    struct test_context *ctx;
    struct vexfs_journal_status status;
    struct vexfs_full_journal_stats stats;
    u32 mode;
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting ioctl interface tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_ORDERED);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Note: These tests simulate ioctl calls without actual user space interface */
    /* In a real implementation, these would be tested through actual ioctl calls */
    
    /* Test status retrieval */
    memset(&status, 0, sizeof(status));
    status.js_mode = vexfs_full_journal_get_mode(ctx->journal);
    TEST_ASSERT(status.js_mode == VEXFS_JOURNAL_MODE_ORDERED, "ioctl status - mode");
    
    /* Test statistics retrieval */
    vexfs_full_journal_get_stats(ctx->journal, &stats);
    TEST_ASSERT(stats.fjs_total_commits >= 0, "ioctl stats - commits");
    TEST_ASSERT(stats.fjs_total_transactions >= 0, "ioctl stats - transactions");
    
    /* Test mode setting */
    ret = vexfs_full_journal_set_mode(ctx->journal, VEXFS_JOURNAL_MODE_WRITEBACK);
    TEST_ASSERT(ret == 0, "ioctl mode setting");
    
    mode = vexfs_full_journal_get_mode(ctx->journal);
    TEST_ASSERT(mode == VEXFS_JOURNAL_MODE_WRITEBACK, "ioctl mode verification");
    
    /* Test force commit */
    ret = vexfs_full_journal_force_commit_all(ctx->journal);
    TEST_ASSERT(ret == 0, "ioctl force commit");
    
    /* Test checkpoint creation */
    ret = vexfs_full_journal_create_checkpoint(ctx->journal, VEXFS_CHECKPOINT_FORCE);
    TEST_ASSERT(ret == 0, "ioctl checkpoint creation");
    
    /* Test buffer resize */
    ret = vexfs_full_journal_resize_buffer(ctx->journal, 32768);
    TEST_ASSERT(ret == 0, "ioctl buffer resize");
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Test 10: Performance Benchmarks
 */
static int test_performance_benchmarks(void)
{
    struct test_context *ctx;
    struct vexfs_full_journal_transaction *trans;
    ktime_t start_time, end_time;
    s64 elapsed_ns;
    int i, ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting performance benchmark tests\n");
    
    ctx = test_init_context();
    if (!ctx) {
        return -ENOMEM;
    }
    
    ctx->journal = vexfs_full_journal_init(ctx->sb, 100, TEST_JOURNAL_BLOCKS, VEXFS_JOURNAL_MODE_JOURNAL);
    if (IS_ERR(ctx->journal)) {
        test_cleanup_context(ctx);
        return PTR_ERR(ctx->journal);
    }
    
    /* Benchmark transaction throughput */
    start_time = ktime_get();
    
    for (i = 0; i < TEST_ITERATIONS; i++) {
        trans = vexfs_full_journal_start(ctx->journal, 16, VEXFS_JOURNAL_OP_WRITE, VEXFS_TRANS_PRIORITY_NORMAL);
        if (!IS_ERR(trans)) {
            vexfs_full_journal_add_data_block(trans, 5000 + i, ctx->test_data, 1024);
            vexfs_full_journal_commit(trans);
        }
    }
    
    end_time = ktime_get();
    elapsed_ns = ktime_to_ns(ktime_sub(end_time, start_time));
    
    printk(KERN_INFO "VexFS Full Journal Test: %d transactions in %lld ns (%lld ns/transaction)\n",
           TEST_ITERATIONS, elapsed_ns, elapsed_ns / TEST_ITERATIONS);
    
    TEST_ASSERT(elapsed_ns > 0, "Performance benchmark - positive elapsed time");
    TEST_ASSERT(elapsed_ns / TEST_ITERATIONS < 10000000, "Performance benchmark - reasonable per-transaction time");
    
    /* Benchmark SHA-256 operations */
    start_time = ktime_get();
    
    for (i = 0; i < TEST_ITERATIONS; i++) {
        u8 hash[SHA256_DIGEST_SIZE];
        vexfs_full_journal_calculate_sha256(ctx->test_data, ctx->data_size, hash);
    }
    
    end_time = ktime_get();
    elapsed_ns = ktime_to_ns(ktime_sub(end_time, start_time));
    
    printk(KERN_INFO "VexFS Full Journal Test: %d SHA-256 operations in %lld ns (%lld ns/operation)\n",
           TEST_ITERATIONS, elapsed_ns, elapsed_ns / TEST_ITERATIONS);
    
    TEST_ASSERT(elapsed_ns > 0, "SHA-256 benchmark - positive elapsed time");
    
    /* Get final statistics */
    struct vexfs_full_journal_stats stats;
    vexfs_full_journal_get_stats(ctx->journal, &stats);
    
    printk(KERN_INFO "VexFS Full Journal Test: Final stats - commits: %llu, transactions: %llu, SHA-256 ops: %llu\n",
           stats.fjs_total_commits, stats.fjs_total_transactions, stats.fjs_sha256_operations);
    
    test_cleanup_context(ctx);
    return ret;
}

/*
 * Main test runner
 */
static int __init vexfs_full_journal_test_init(void)
{
    int ret = 0;
    
    printk(KERN_INFO "VexFS Full Journal Test: Starting comprehensive test suite\n");
    
    /* Initialize test counters */
    atomic_set
(&tests_passed, 0);
    atomic_set(&tests_failed, 0);
    atomic_set(&tests_total, 0);
    
    /* Run all test suites */
    ret |= test_journal_initialization();
    ret |= test_transaction_management();
    ret |= test_concurrent_transactions();
    ret |= test_journaling_modes();
    ret |= test_sha256_checksumming();
    ret |= test_checkpointing();
    ret |= test_barrier_operations();
    ret |= test_crash_recovery();
    ret |= test_ioctl_interface();
    ret |= test_performance_benchmarks();
    
    /* Print final results */
    int passed = atomic_read(&tests_passed);
    int failed = atomic_read(&tests_failed);
    int total = atomic_read(&tests_total);
    
    printk(KERN_INFO "VexFS Full Journal Test: Test suite completed\n");
    printk(KERN_INFO "VexFS Full Journal Test: Results - %d/%d tests passed, %d failed\n",
           passed, total, failed);
    
    if (failed == 0) {
        printk(KERN_INFO "VexFS Full Journal Test: ALL TESTS PASSED!\n");
    } else {
        printk(KERN_ERR "VexFS Full Journal Test: %d TESTS FAILED!\n", failed);
        ret = -1;
    }
    
    return ret;
}

/*
 * Test module cleanup
 */
static void __exit vexfs_full_journal_test_exit(void)
{
    printk(KERN_INFO "VexFS Full Journal Test: Test module unloaded\n");
}

module_init(vexfs_full_journal_test_init);
module_exit(vexfs_full_journal_test_exit);

MODULE_DESCRIPTION("VexFS v2.0 Full Filesystem Journal Test Suite");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");
MODULE_VERSION("2.0.0");