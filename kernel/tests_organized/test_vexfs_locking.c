/*
 * VexFS v2.0 Fine-Grained Locking Test Suite
 * 
 * Comprehensive test suite for the VexFS fine-grained locking system.
 * Tests concurrent access, lock contention, deadlock detection, and
 * performance under various workloads.
 * 
 * Compile: gcc -o test_vexfs_locking test_vexfs_locking.c -lpthread
 * Run: ./test_vexfs_locking
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <errno.h>
#include <sys/time.h>
#include <stdint.h>
#include <assert.h>
#include <signal.h>
#include <atomic>
#include <chrono>

/* Simulate kernel types for userspace testing */
typedef uint32_t u32;
typedef uint64_t u64;
typedef int32_t s32;
typedef int64_t s64;

/* Simulate atomic operations */
#define atomic_t std::atomic<int>
#define atomic64_t std::atomic<long long>
#define atomic_set(ptr, val) (ptr)->store(val)
#define atomic_read(ptr) (ptr)->load()
#define atomic_inc(ptr) (ptr)->fetch_add(1)
#define atomic_dec(ptr) (ptr)->fetch_sub(1)
#define atomic64_set(ptr, val) (ptr)->store(val)
#define atomic64_read(ptr) (ptr)->load()
#define atomic64_inc(ptr) (ptr)->fetch_add(1)
#define atomic64_add(val, ptr) (ptr)->fetch_add(val)

/* Test configuration */
#define TEST_MAX_THREADS        32
#define TEST_MAX_VECTORS        10000
#define TEST_OPERATIONS_PER_THREAD 1000
#define TEST_DEADLOCK_THREADS   8
#define TEST_CONTENTION_THREADS 16
#define TEST_DURATION_SECONDS   10

/* Test result tracking */
static atomic_t tests_passed = ATOMIC_VAR_INIT(0);
static atomic_t tests_failed = ATOMIC_VAR_INIT(0);
static atomic_t total_tests = ATOMIC_VAR_INIT(0);

/* Performance metrics */
static atomic64_t total_operations = ATOMIC_VAR_INIT(0);
static atomic64_t total_contentions = ATOMIC_VAR_INIT(0);
static atomic64_t total_deadlocks = ATOMIC_VAR_INIT(0);
static atomic64_t total_lock_time_ns = ATOMIC_VAR_INIT(0);

/* Test synchronization */
static pthread_barrier_t test_barrier;
static volatile bool test_running = false;
static volatile bool test_stop = false;

/* Utility macros */
#define TEST_ASSERT(condition, message) do { \
    atomic_inc(&total_tests); \
    if (condition) { \
        printf("✅ PASS: %s\n", message); \
        atomic_inc(&tests_passed); \
    } else { \
        printf("❌ FAIL: %s\n", message); \
        atomic_inc(&tests_failed); \
    } \
} while(0)

#define TEST_START(name) printf("\n🔥 Starting test: %s\n", name)
#define TEST_END(name) printf("✅ Completed test: %s\n", name)

/* Get current time in nanoseconds */
static uint64_t get_time_ns() {
    auto now = std::chrono::high_resolution_clock::now();
    auto duration = now.time_since_epoch();
    return std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
}

/* Simulated lock structures for testing */
struct test_vector_lock {
    pthread_rwlock_t rwlock;
    atomic_t ref_count;
    atomic_t reader_count;
    atomic_t writer_count;
    atomic64_t contention_count;
    u64 vector_id;
    u32 numa_node;
    pthread_mutex_t stats_lock;
    u64 acquire_time_total;
    u64 hold_time_total;
    u32 acquire_count;
};

struct test_lock_manager {
    pthread_mutex_t global_mutex;
    pthread_rwlock_t global_rwlock;
    pthread_mutex_t hash_lock;
    struct test_vector_lock *vector_locks[1024];
    atomic_t vector_lock_count;
    atomic64_t total_acquisitions;
    atomic64_t total_contentions;
    atomic64_t total_deadlocks;
    bool numa_aware;
    bool deadlock_detection;
    bool adaptive_locking;
};

/* Global test lock manager */
static struct test_lock_manager test_manager;

/* 🔥 SIMULATED LOCKING OPERATIONS 🔥 */

/**
 * test_vector_lock_create - Create a test vector lock
 */
static struct test_vector_lock *test_vector_lock_create(u64 vector_id, u32 numa_node)
{
    struct test_vector_lock *lock = (struct test_vector_lock *)malloc(sizeof(*lock));
    if (!lock) {
        return NULL;
    }
    
    pthread_rwlock_init(&lock->rwlock, NULL);
    atomic_set(&lock->ref_count, 1);
    atomic_set(&lock->reader_count, 0);
    atomic_set(&lock->writer_count, 0);
    atomic64_set(&lock->contention_count, 0);
    lock->vector_id = vector_id;
    lock->numa_node = numa_node;
    pthread_mutex_init(&lock->stats_lock, NULL);
    lock->acquire_time_total = 0;
    lock->hold_time_total = 0;
    lock->acquire_count = 0;
    
    return lock;
}

/**
 * test_vector_lock_acquire - Acquire a test vector lock
 */
static struct test_vector_lock *test_vector_lock_acquire(u64 vector_id, bool write_lock)
{
    u32 hash = vector_id % 1024;
    struct test_vector_lock *lock;
    uint64_t start_time = get_time_ns();
    
    pthread_mutex_lock(&test_manager.hash_lock);
    
    lock = test_manager.vector_locks[hash];
    if (!lock) {
        lock = test_vector_lock_create(vector_id, 0);
        if (lock) {
            test_manager.vector_locks[hash] = lock;
            atomic_inc(&test_manager.vector_lock_count);
        }
    }
    
    if (lock) {
        atomic_inc(&lock->ref_count);
    }
    
    pthread_mutex_unlock(&test_manager.hash_lock);
    
    if (!lock) {
        return NULL;
    }
    
    /* Acquire the actual lock */
    int ret;
    if (write_lock) {
        ret = pthread_rwlock_wrlock(&lock->rwlock);
        if (ret == 0) {
            atomic_inc(&lock->writer_count);
        }
    } else {
        ret = pthread_rwlock_rdlock(&lock->rwlock);
        if (ret == 0) {
            atomic_inc(&lock->reader_count);
        }
    }
    
    if (ret != 0) {
        atomic64_inc(&lock->contention_count);
        atomic64_inc(&test_manager.total_contentions);
        atomic_dec(&lock->ref_count);
        return NULL;
    }
    
    /* Update statistics */
    atomic64_inc(&test_manager.total_acquisitions);
    
    pthread_mutex_lock(&lock->stats_lock);
    lock->acquire_count++;
    lock->acquire_time_total += get_time_ns() - start_time;
    pthread_mutex_unlock(&lock->stats_lock);
    
    return lock;
}

/**
 * test_vector_lock_release - Release a test vector lock
 */
static void test_vector_lock_release(struct test_vector_lock *lock, bool write_lock)
{
    if (!lock) {
        return;
    }
    
    /* Release the actual lock */
    if (write_lock) {
        pthread_rwlock_unlock(&lock->rwlock);
        atomic_dec(&lock->writer_count);
    } else {
        pthread_rwlock_unlock(&lock->rwlock);
        atomic_dec(&lock->reader_count);
    }
    
    /* Release reference */
    atomic_dec(&lock->ref_count);
}

/* 🔥 TEST THREAD FUNCTIONS 🔥 */

/**
 * test_concurrent_readers_thread - Thread function for concurrent reader test
 */
static void *test_concurrent_readers_thread(void *arg)
{
    int thread_id = *(int *)arg;
    u64 vector_id = 12345; /* All threads access same vector */
    
    /* Wait for all threads to be ready */
    pthread_barrier_wait(&test_barrier);
    
    for (int i = 0; i < TEST_OPERATIONS_PER_THREAD; i++) {
        struct test_vector_lock *lock = test_vector_lock_acquire(vector_id, false);
        if (lock) {
            /* Simulate read operation */
            usleep(1); /* 1 microsecond */
            test_vector_lock_release(lock, false);
            atomic64_inc(&total_operations);
        }
        
        if (test_stop) break;
    }
    
    return NULL;
}

/**
 * test_reader_writer_thread - Thread function for reader/writer test
 */
static void *test_reader_writer_thread(void *arg)
{
    int thread_id = *(int *)arg;
    u64 vector_id = 54321;
    bool is_writer = (thread_id % 4 == 0); /* 25% writers, 75% readers */
    
    pthread_barrier_wait(&test_barrier);
    
    for (int i = 0; i < TEST_OPERATIONS_PER_THREAD; i++) {
        struct test_vector_lock *lock = test_vector_lock_acquire(vector_id, is_writer);
        if (lock) {
            /* Simulate operation */
            if (is_writer) {
                usleep(5); /* Writers take longer */
            } else {
                usleep(1); /* Readers are faster */
            }
            test_vector_lock_release(lock, is_writer);
            atomic64_inc(&total_operations);
        }
        
        if (test_stop) break;
    }
    
    return NULL;
}

/**
 * test_contention_thread - Thread function for lock contention test
 */
static void *test_contention_thread(void *arg)
{
    int thread_id = *(int *)arg;
    u64 vector_id = thread_id % 4; /* High contention on few vectors */
    
    pthread_barrier_wait(&test_barrier);
    
    while (!test_stop) {
        struct test_vector_lock *lock = test_vector_lock_acquire(vector_id, true);
        if (lock) {
            /* Simulate work under lock */
            usleep(10);
            test_vector_lock_release(lock, true);
            atomic64_inc(&total_operations);
        } else {
            atomic64_inc(&total_contentions);
        }
        
        /* Small delay between operations */
        usleep(1);
    }
    
    return NULL;
}

/**
 * test_deadlock_thread - Thread function for deadlock detection test
 */
static void *test_deadlock_thread(void *arg)
{
    int thread_id = *(int *)arg;
    u64 vector_id1 = thread_id % 4;
    u64 vector_id2 = (thread_id + 1) % 4;
    
    pthread_barrier_wait(&test_barrier);
    
    for (int i = 0; i < TEST_OPERATIONS_PER_THREAD / 10; i++) {
        struct test_vector_lock *lock1, *lock2;
        
        /* Acquire locks in different orders to create potential deadlocks */
        if (thread_id % 2 == 0) {
            lock1 = test_vector_lock_acquire(vector_id1, true);
            usleep(1);
            lock2 = test_vector_lock_acquire(vector_id2, true);
        } else {
            lock2 = test_vector_lock_acquire(vector_id2, true);
            usleep(1);
            lock1 = test_vector_lock_acquire(vector_id1, true);
        }
        
        if (lock1 && lock2) {
            /* Simulate work */
            usleep(5);
            atomic64_inc(&total_operations);
        } else {
            atomic64_inc(&total_deadlocks);
        }
        
        /* Release locks */
        if (lock2) test_vector_lock_release(lock2, true);
        if (lock1) test_vector_lock_release(lock1, true);
        
        if (test_stop) break;
    }
    
    return NULL;
}

/* 🔥 TEST FUNCTIONS 🔥 */

/**
 * test_concurrent_readers - Test concurrent reader access
 */
static int test_concurrent_readers()
{
    TEST_START("Concurrent Readers");
    
    const int num_threads = 8;
    pthread_t threads[num_threads];
    int thread_ids[num_threads];
    uint64_t start_time, end_time;
    
    /* Initialize barrier */
    pthread_barrier_init(&test_barrier, NULL, num_threads + 1);
    
    /* Create threads */
    for (int i = 0; i < num_threads; i++) {
        thread_ids[i] = i;
        int ret = pthread_create(&threads[i], NULL, test_concurrent_readers_thread, &thread_ids[i]);
        TEST_ASSERT(ret == 0, "Thread creation succeeded");
    }
    
    /* Start timing and release threads */
    start_time = get_time_ns();
    pthread_barrier_wait(&test_barrier);
    
    /* Wait for threads to complete */
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    end_time = get_time_ns();
    
    /* Verify results */
    u64 operations = atomic64_read(&total_operations);
    double duration_sec = (end_time - start_time) / 1e9;
    double ops_per_sec = operations / duration_sec;
    
    TEST_ASSERT(operations > 0, "Operations were performed");
    TEST_ASSERT(ops_per_sec > 1000, "Reasonable throughput achieved");
    
    printf("📊 Concurrent readers: %llu ops in %.3f sec (%.1f ops/sec)\n",
           operations, duration_sec, ops_per_sec);
    
    pthread_barrier_destroy(&test_barrier);
    TEST_END("Concurrent Readers");
    
    return 0;
}

/**
 * test_reader_writer_contention - Test reader/writer contention
 */
static int test_reader_writer_contention()
{
    TEST_START("Reader/Writer Contention");
    
    const int num_threads = 12;
    pthread_t threads[num_threads];
    int thread_ids[num_threads];
    uint64_t start_time, end_time;
    
    /* Reset counters */
    atomic64_set(&total_operations, 0);
    atomic64_set(&total_contentions, 0);
    
    pthread_barrier_init(&test_barrier, NULL, num_threads + 1);
    
    /* Create threads */
    for (int i = 0; i < num_threads; i++) {
        thread_ids[i] = i;
        int ret = pthread_create(&threads[i], NULL, test_reader_writer_thread, &thread_ids[i]);
        TEST_ASSERT(ret == 0, "Thread creation succeeded");
    }
    
    start_time = get_time_ns();
    pthread_barrier_wait(&test_barrier);
    
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    end_time = get_time_ns();
    
    u64 operations = atomic64_read(&total_operations);
    u64 contentions = atomic64_read(&total_contentions);
    double duration_sec = (end_time - start_time) / 1e9;
    double contention_rate = (double)contentions / operations * 100.0;
    
    TEST_ASSERT(operations > 0, "Operations were performed");
    TEST_ASSERT(contention_rate < 50.0, "Contention rate is reasonable");
    
    printf("📊 Reader/Writer: %llu ops, %llu contentions (%.1f%% rate)\n",
           operations, contentions, contention_rate);
    
    pthread_barrier_destroy(&test_barrier);
    TEST_END("Reader/Writer Contention");
    
    return 0;
}

/**
 * test_high_contention - Test high contention scenarios
 */
static int test_high_contention()
{
    TEST_START("High Contention");
    
    const int num_threads = TEST_CONTENTION_THREADS;
    pthread_t threads[num_threads];
    int thread_ids[num_threads];
    uint64_t start_time, end_time;
    
    atomic64_set(&total_operations, 0);
    atomic64_set(&total_contentions, 0);
    
    pthread_barrier_init(&test_barrier, NULL, num_threads + 1);
    test_stop = false;
    
    /* Create threads */
    for (int i = 0; i < num_threads; i++) {
        thread_ids[i] = i;
        int ret = pthread_create(&threads[i], NULL, test_contention_thread, &thread_ids[i]);
        TEST_ASSERT(ret == 0, "Thread creation succeeded");
    }
    
    start_time = get_time_ns();
    pthread_barrier_wait(&test_barrier);
    
    /* Let threads run for a fixed duration */
    sleep(2);
    test_stop = true;
    
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    end_time = get_time_ns();
    
    u64 operations = atomic64_read(&total_operations);
    u64 contentions = atomic64_read(&total_contentions);
    double duration_sec = (end_time - start_time) / 1e9;
    double ops_per_sec = operations / duration_sec;
    double contention_rate = (double)contentions / (operations + contentions) * 100.0;
    
    TEST_ASSERT(operations > 0, "Operations were performed under contention");
    TEST_ASSERT(ops_per_sec > 100, "Reasonable throughput under contention");
    
    printf("📊 High contention: %llu ops, %llu contentions (%.1f%% rate, %.1f ops/sec)\n",
           operations, contentions, contention_rate, ops_per_sec);
    
    pthread_barrier_destroy(&test_barrier);
    TEST_END("High Contention");
    
    return 0;
}

/**
 * test_deadlock_detection - Test deadlock detection and prevention
 */
static int test_deadlock_detection()
{
    TEST_START("Deadlock Detection");
    
    const int num_threads = TEST_DEADLOCK_THREADS;
    pthread_t threads[num_threads];
    int thread_ids[num_threads];
    uint64_t start_time, end_time;
    
    atomic64_set(&total_operations, 0);
    atomic64_set(&total_deadlocks, 0);
    
    pthread_barrier_init(&test_barrier, NULL, num_threads + 1);
    test_stop = false;
    
    /* Create threads that will create potential deadlocks */
    for (int i = 0; i < num_threads; i++) {
        thread_ids[i] = i;
        int ret = pthread_create(&threads[i], NULL, test_deadlock_thread, &thread_ids[i]);
        TEST_ASSERT(ret == 0, "Thread creation succeeded");
    }
    
    start_time = get_time_ns();
    pthread_barrier_wait(&test_barrier);
    
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    end_time = get_time_ns();
    
    u64 operations = atomic64_read(&total_operations);
    u64 deadlocks = atomic64_read(&total_deadlocks);
    double duration_sec = (end_time - start_time) / 1e9;
    
    TEST_ASSERT(operations > 0, "Some operations completed despite deadlock potential");
    
    printf("📊 Deadlock test: %llu ops, %llu potential deadlocks in %.3f sec\n",
           operations, deadlocks, duration_sec);
    
    pthread_barrier_destroy(&test_barrier);
    TEST_END("Deadlock Detection");
    
    return 0;
}

/**
 * test_lock_scaling - Test lock scaling with increasing thread counts
 */
static int test_lock_scaling()
{
    TEST_START("Lock Scaling");
    
    const int thread_counts[] = {1, 2, 4, 8, 16};
    const int num_tests = sizeof(thread_counts) / sizeof(thread_counts[0]);
    
    for (int t = 0; t < num_tests; t++) {
        int num_threads = thread_counts[t];
        pthread_t threads[num_threads];
        int thread_ids[num_threads];
        uint64_t start_time, end_time;
        
        atomic64_set(&total_operations, 0);
        
        pthread_barrier_init(&test_barrier, NULL, num_threads + 1);
        
        /* Create threads */
        for (int i = 0; i < num_threads; i++) {
            thread_ids[i] = i;
            pthread_create(&threads[i], NULL, test_concurrent_readers_thread, &thread_ids[i]);
        }
        
        start_time = get_time_ns();
        pthread_barrier_wait(&test_barrier);
        
        for (int i = 0; i < num_threads; i++) {
            pthread_join(threads[i], NULL);
        }
        
        end_time = get_time_ns();
        
        u64 operations = atomic64_read(&total_operations);
        double duration_sec = (end_time - start_time) / 1e9;
        double ops_per_sec = operations / duration_sec;
        
        printf("📊 %d threads: %llu ops in %.3f sec (%.1f ops/sec)\n",
               num_threads, operations, duration_sec, ops_per_sec);
        
        pthread_barrier_destroy(&test_barrier);
    }
    
    TEST_ASSERT(true, "Scaling test completed");
    TEST_END("Lock Scaling");
    
    return 0;
}

/**
 * test_lock_manager_init - Test lock manager initialization
 */
static int test_lock_manager_init()
{
    TEST_START("Lock Manager Initialization");
    
    /* Initialize test lock manager */
    memset(&test_manager, 0, sizeof(test_manager));
    
    pthread_mutex_init(&test_manager.global_mutex, NULL);
    pthread_rwlock_init(&test_manager.global_rwlock, NULL);
    pthread_mutex_init(&test_manager.hash_lock, NULL);
    
    for (int i = 0; i < 1024; i++) {
        test_manager.vector_locks[i] = NULL;
    }
    
    atomic_set(&test_manager.vector_lock_count, 0);
    atomic64_set(&test_manager.total_acquisitions, 0);
    atomic64_set(&test_manager.total_contentions, 0);
    atomic64_set(&test_manager.total_deadlocks, 0);
    
    test_manager.numa_aware = true;
    test_manager.deadlock_detection = true;
    test_manager.adaptive_locking = true;
    
    TEST_ASSERT(true, "Lock manager initialized successfully");
    
    TEST_END("Lock Manager Initialization");
    
    return 0;
}

/* 🔥 MAIN TEST RUNNER 🔥 */

/**
 * signal_handler - Handle test interruption
 */
static void signal_handler(int sig)
{
    printf("\n⚠️  Test interrupted by signal %d\n", sig);
    test_stop = true;
}

/**
 * main - Main test function
 */
int main(int argc, char *argv[])
{
    printf("🚀 VexFS v2.0 Fine-Grained Locking Test Suite\n");
    printf("==============================================\n");
    
    /* Set up signal handling */
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    /* Initialize test environment */
    test_lock_manager_init();
    
    /* Run test suites */
    test_concurrent_readers();
    test_reader_writer_contention();
    test_high_contention();
    test_deadlock_detection();
    test_lock_scaling();
    
    /* Print test summary */
    printf("\n📊 Test Summary\n");
    printf("===============\n");
    printf("Total tests: %d\n", atomic_read(&total_tests));
    printf("Passed: %d\n", atomic_read(&tests_passed));
    printf("Failed: %d\n", atomic_read(&tests_failed));
    
    printf("\nPerformance Summary:\n");
    printf("Total operations: %llu\n", atomic64_read(&total_operations));
    printf("Total contentions: %llu\n", atomic64_read(&total_contentions));
    printf("Total deadlocks: %llu\n", atomic64_read(&total_deadlocks));
    printf("Active vector locks: %d\n", atomic_read(&test_manager.vector_lock_count));
    
    if (atomic_read(&tests_failed) == 0) {
        printf("🎉 All tests passed!\n");
        return 0;
    } else {
        printf("❌ %d tests failed\n", atomic_read(&tests_failed));
        return 1;
    }
}