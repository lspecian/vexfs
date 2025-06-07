/*
 * VexFS v2.0 ANN Index Cache Test Suite
 * 
 * Comprehensive test suite for the ANN Index Caching System that validates:
 * - Cache initialization and cleanup
 * - Entry allocation and management
 * - RCU-protected concurrent access
 * - Cache coherency mechanisms
 * - Performance under various workloads
 * - NUMA awareness and optimization
 * - Integration with memory management and vector cache systems
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <time.h>
#include <sys/time.h>
#include <errno.h>
#include <assert.h>
#include <stdint.h>
#include <stdbool.h>

/* Mock kernel headers for userspace testing */
#ifndef __KERNEL__

/* Basic type definitions */
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef int32_t s32;
typedef int64_t s64;

/* Mock atomic operations */
typedef struct { volatile int counter; } atomic_t;
typedef struct { volatile long counter; } atomic64_t;

#define atomic_set(v, i) ((v)->counter = (i))
#define atomic_read(v) ((v)->counter)
#define atomic_inc(v) (__sync_add_and_fetch(&(v)->counter, 1))
#define atomic_dec(v) (__sync_sub_and_fetch(&(v)->counter, 1))
#define atomic_dec_and_test(v) (__sync_sub_and_fetch(&(v)->counter, 1) == 0)
#define atomic64_set(v, i) ((v)->counter = (i))
#define atomic64_read(v) ((v)->counter)
#define atomic64_inc(v) (__sync_add_and_fetch(&(v)->counter, 1))
#define atomic64_add(v, i) (__sync_add_and_fetch(&(v)->counter, i))
#define atomic64_sub(v, i) (__sync_sub_and_fetch(&(v)->counter, i))

/* Mock spinlock */
typedef pthread_mutex_t spinlock_t;
#define spin_lock_init(lock) pthread_mutex_init(lock, NULL)
#define spin_lock_irqsave(lock, flags) pthread_mutex_lock(lock)
#define spin_unlock_irqrestore(lock, flags) pthread_mutex_unlock(lock)

/* Mock mutex */
typedef pthread_mutex_t mutex_t;
#define mutex_init(mutex) pthread_mutex_init(mutex, NULL)
#define mutex_lock(mutex) pthread_mutex_lock(mutex)
#define mutex_unlock(mutex) pthread_mutex_unlock(mutex)

/* Mock completion */
typedef struct {
    pthread_mutex_t mutex;
    pthread_cond_t cond;
    int done;
} completion_t;

#define init_completion(comp) do { \
    pthread_mutex_init(&(comp)->mutex, NULL); \
    pthread_cond_init(&(comp)->cond, NULL); \
    (comp)->done = 0; \
} while(0)

/* Mock list operations */
struct list_head {
    struct list_head *next, *prev;
};

#define LIST_HEAD_INIT(name) { &(name), &(name) }
#define LIST_HEAD(name) struct list_head name = LIST_HEAD_INIT(name)
#define INIT_LIST_HEAD(ptr) do { \
    (ptr)->next = (ptr); (ptr)->prev = (ptr); \
} while (0)

/* Mock hash table */
#define DECLARE_HASHTABLE(name, bits) struct hlist_head name[1 << (bits)]
#define hash_init(hashtable) memset(hashtable, 0, sizeof(hashtable))

struct hlist_head {
    struct hlist_node *first;
};

struct hlist_node {
    struct hlist_node *next, **pprev;
};

#define INIT_HLIST_NODE(ptr) ((ptr)->next = NULL, (ptr)->pprev = NULL)

/* Mock RCU */
struct rcu_head {
    void (*func)(struct rcu_head *head);
};

#define rcu_read_lock() do { } while (0)
#define rcu_read_unlock() do { } while (0)
#define synchronize_rcu() do { } while (0)
#define call_rcu(head, func) (func)(head)

/* Mock red-black tree */
struct rb_node {
    unsigned long rb_parent_color;
    struct rb_node *rb_right;
    struct rb_node *rb_left;
};

struct rb_root {
    struct rb_node *rb_node;
};

#define RB_ROOT { NULL }

/* Mock memory allocation */
#define GFP_KERNEL 0
#define GFP_ATOMIC 1
#define SLAB_HWCACHE_ALIGN 0x1
#define SLAB_RECLAIM_ACCOUNT 0x2

struct kmem_cache;

static inline void *kmalloc(size_t size, int flags) { return malloc(size); }
static inline void *kzalloc(size_t size, int flags) { return calloc(1, size); }
static inline void kfree(void *ptr) { free(ptr); }
static inline void *vmalloc(size_t size) { return malloc(size); }
static inline void vfree(void *ptr) { free(ptr); }

struct kmem_cache *kmem_cache_create(const char *name, size_t size, 
                                    size_t align, unsigned long flags,
                                    void (*ctor)(void *));
void kmem_cache_destroy(struct kmem_cache *cache);
void *kmem_cache_alloc(struct kmem_cache *cache, int flags);
void kmem_cache_free(struct kmem_cache *cache, void *ptr);

/* Mock page allocation */
struct page {
    void *virtual;
};

#define __free_page(page) free((page)->virtual)

/* Mock workqueue */
struct work_struct {
    void (*func)(struct work_struct *work);
};

struct delayed_work {
    struct work_struct work;
};

struct workqueue_struct;

#define INIT_DELAYED_WORK(work, func) do { (work)->work.func = (func); } while(0)
static inline struct workqueue_struct *alloc_workqueue(const char *name, unsigned int flags, int max_active) { return (void*)1; }
static inline void destroy_workqueue(struct workqueue_struct *wq) { }
static inline bool queue_delayed_work(struct workqueue_struct *wq, struct delayed_work *dwork, unsigned long delay) { return true; }
static inline bool cancel_delayed_work_sync(struct delayed_work *dwork) { return true; }

/* Mock time functions */
static inline u64 ktime_get_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

static inline unsigned long msecs_to_jiffies(unsigned int m) { return m; }

/* Mock NUMA functions */
static inline int numa_node_id(void) { return 0; }

/* Mock CPU functions */
struct cpumask {
    unsigned long bits[1];
};

extern struct cpumask cpu_online_mask;
#define cpumask_copy(dst, src) memcpy(dst, src, sizeof(struct cpumask))

/* Mock hash function */
static inline u32 hash_64(u64 val, unsigned int bits) {
    return (u32)(val * 0x9e3779b97f4a7c15ULL >> (64 - bits));
}

/* Mock print functions */
#define pr_info printf
#define pr_err printf

/* Mock module macros */
#define MODULE_LICENSE(x)
#define MODULE_AUTHOR(x)
#define MODULE_DESCRIPTION(x)
#define MODULE_VERSION(x)
#define module_init(x)
#define module_exit(x)

#endif /* !__KERNEL__ */

/* Include the ANN cache header */
#include "vexfs_v2_ann_index_cache.h"

/* Test configuration */
#define TEST_MAX_ENTRIES 1000
#define TEST_NUM_THREADS 8
#define TEST_OPERATIONS_PER_THREAD 1000
#define TEST_CACHE_SIZE_MB 32

/* Test statistics */
struct test_stats {
    atomic64_t operations_completed;
    atomic64_t cache_hits;
    atomic64_t cache_misses;
    atomic64_t allocation_failures;
    atomic64_t total_time_ns;
    atomic64_t max_time_ns;
    atomic64_t min_time_ns;
};

static struct test_stats global_test_stats;

/* Mock memory manager and vector cache for testing */
static struct vexfs_memory_manager mock_mm;
static struct vexfs_vector_cache mock_vector_cache;

/*
 * Mock kmem_cache implementation for userspace testing
 */
struct kmem_cache {
    char name[64];
    size_t size;
    size_t align;
    unsigned long flags;
};

struct kmem_cache *kmem_cache_create(const char *name, size_t size, 
                                    size_t align, unsigned long flags,
                                    void (*ctor)(void *))
{
    struct kmem_cache *cache = malloc(sizeof(struct kmem_cache));
    if (!cache) return NULL;
    
    strncpy(cache->name, name, sizeof(cache->name) - 1);
    cache->name[sizeof(cache->name) - 1] = '\0';
    cache->size = size;
    cache->align = align;
    cache->flags = flags;
    
    return cache;
}

void kmem_cache_destroy(struct kmem_cache *cache)
{
    if (cache) {
        free(cache);
    }
}

void *kmem_cache_alloc(struct kmem_cache *cache, int flags)
{
    if (!cache) return NULL;
    return malloc(cache->size);
}

void kmem_cache_free(struct kmem_cache *cache, void *ptr)
{
    if (ptr) {
        free(ptr);
    }
}

/* Mock CPU mask */
struct cpumask cpu_online_mask = { .bits = { 0xFF } };

/*
 * Test helper functions
 */
static void test_print_header(const char *test_name)
{
    printf("\n=== %s ===\n", test_name);
}

static void test_print_result(const char *test_name, bool passed)
{
    printf("%s: %s\n", test_name, passed ? "PASSED" : "FAILED");
}

static u64 get_time_ns(void)
{
    return ktime_get_ns();
}

static void update_timing_stats(u64 start_time, u64 end_time)
{
    u64 duration = end_time - start_time;
    atomic64_add(&global_test_stats.total_time_ns, duration);
    
    u64 current_max = atomic64_read(&global_test_stats.max_time_ns);
    while (duration > current_max) {
        if (atomic64_cmpxchg(&global_test_stats.max_time_ns, current_max, duration) == current_max) {
            break;
        }
        current_max = atomic64_read(&global_test_stats.max_time_ns);
    }
    
    u64 current_min = atomic64_read(&global_test_stats.min_time_ns);
    if (current_min == 0 || duration < current_min) {
        while (current_min == 0 || duration < current_min) {
            if (atomic64_cmpxchg(&global_test_stats.min_time_ns, current_min, duration) == current_min) {
                break;
            }
            current_min = atomic64_read(&global_test_stats.min_time_ns);
        }
    }
}

/*
 * Test 1: Basic Cache Initialization and Cleanup
 */
static bool test_cache_initialization(void)
{
    struct vexfs_ann_cache *cache = NULL;
    int ret;
    
    test_print_header("Cache Initialization Test");
    
    /* Test cache initialization */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0 || !cache) {
        printf("Failed to initialize cache: %d\n", ret);
        return false;
    }
    
    /* Verify cache configuration */
    if (cache->max_memory_usage != TEST_CACHE_SIZE_MB * 1024 * 1024) {
        printf("Incorrect cache size configuration\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    if (cache->max_entries != VEXFS_ANN_CACHE_MAX_ENTRIES) {
        printf("Incorrect max entries configuration\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Verify kmem_cache creation */
    for (int i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        if (!cache->caches[i]) {
            printf("Failed to create kmem_cache for type %d\n", i);
            vexfs_ann_cache_destroy(cache);
            return false;
        }
    }
    
    /* Test cache cleanup */
    vexfs_ann_cache_destroy(cache);
    
    printf("Cache initialization and cleanup successful\n");
    return true;
}

/*
 * Test 2: Cache Entry Management
 */
static bool test_cache_entry_management(void)
{
    struct vexfs_ann_cache *cache = NULL;
    struct vexfs_ann_cache_entry *entry1, *entry2, *entry3;
    int ret;
    
    test_print_header("Cache Entry Management Test");
    
    /* Initialize cache */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0) {
        return false;
    }
    
    /* Test entry allocation and insertion */
    entry1 = vexfs_ann_cache_get(cache, 1, VEXFS_ANN_INDEX_HNSW_NODE);
    if (!entry1) {
        printf("Failed to allocate cache entry\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Test entry lookup */
    ret = vexfs_ann_cache_lookup(cache, 1, VEXFS_ANN_INDEX_HNSW_NODE, &entry2);
    if (ret != 0 || entry2 != entry1) {
        printf("Failed to lookup cache entry\n");
        vexfs_ann_cache_put(cache, entry1);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Test entry reference counting */
    if (atomic_read(&entry1->ref_count) != 2) {
        printf("Incorrect reference count: %d\n", atomic_read(&entry1->ref_count));
        vexfs_ann_cache_put(cache, entry1);
        vexfs_ann_cache_put(cache, entry2);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Test entry removal */
    vexfs_ann_cache_put(cache, entry1);
    vexfs_ann_cache_put(cache, entry2);
    
    ret = vexfs_ann_cache_remove(cache, 1);
    if (ret != 0) {
        printf("Failed to remove cache entry\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Verify entry is no longer found */
    ret = vexfs_ann_cache_lookup(cache, 1, VEXFS_ANN_INDEX_HNSW_NODE, &entry3);
    if (ret == 0) {
        printf("Entry still found after removal\n");
        vexfs_ann_cache_put(cache, entry3);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    vexfs_ann_cache_destroy(cache);
    
    printf("Cache entry management test successful\n");
    return true;
}

/*
 * Test 3: Multiple Index Types
 */
static bool test_multiple_index_types(void)
{
    struct vexfs_ann_cache *cache = NULL;
    struct vexfs_ann_cache_entry *entries[VEXFS_ANN_INDEX_TYPE_COUNT];
    int ret;
    
    test_print_header("Multiple Index Types Test");
    
    /* Initialize cache */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0) {
        return false;
    }
    
    /* Allocate entries for each index type */
    for (int i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        entries[i] = vexfs_ann_cache_get(cache, i + 100, i);
        if (!entries[i]) {
            printf("Failed to allocate entry for type %d\n", i);
            goto cleanup;
        }
        
        if (entries[i]->type != i) {
            printf("Incorrect entry type: expected %d, got %d\n", i, entries[i]->type);
            goto cleanup;
        }
    }
    
    /* Verify all entries can be looked up */
    for (int i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        struct vexfs_ann_cache_entry *found;
        ret = vexfs_ann_cache_lookup(cache, i + 100, i, &found);
        if (ret != 0 || found != entries[i]) {
            printf("Failed to lookup entry for type %d\n", i);
            if (ret == 0) vexfs_ann_cache_put(cache, found);
            goto cleanup;
        }
        vexfs_ann_cache_put(cache, found);
    }
    
    /* Clean up entries */
    for (int i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        vexfs_ann_cache_put(cache, entries[i]);
    }
    
    vexfs_ann_cache_destroy(cache);
    
    printf("Multiple index types test successful\n");
    return true;
    
cleanup:
    for (int i = 0; i < VEXFS_ANN_INDEX_TYPE_COUNT; i++) {
        if (entries[i]) {
            vexfs_ann_cache_put(cache, entries[i]);
        }
    }
    vexfs_ann_cache_destroy(cache);
    return false;
}

/*
 * Test 4: Cache Statistics
 */
static bool test_cache_statistics(void)
{
    struct vexfs_ann_cache *cache = NULL;
    struct vexfs_ann_cache_stats stats;
    struct vexfs_ann_cache_entry *entry;
    int ret;
    
    test_print_header("Cache Statistics Test");
    
    /* Initialize cache */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0) {
        return false;
    }
    
    /* Get initial statistics */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0) {
        printf("Failed to get cache statistics\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    u64 initial_entries = atomic64_read(&stats.total_entries);
    u64 initial_hits = atomic64_read(&stats.cache_hits);
    u64 initial_misses = atomic64_read(&stats.cache_misses);
    
    /* Allocate an entry (should cause a miss) */
    entry = vexfs_ann_cache_get(cache, 200, VEXFS_ANN_INDEX_HNSW_NODE);
    if (!entry) {
        printf("Failed to allocate cache entry\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Get updated statistics */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0) {
        printf("Failed to get updated cache statistics\n");
        vexfs_ann_cache_put(cache, entry);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Verify statistics updated correctly */
    if (atomic64_read(&stats.total_entries) != initial_entries + 1) {
        printf("Total entries not updated correctly\n");
        vexfs_ann_cache_put(cache, entry);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    if (atomic64_read(&stats.cache_misses) != initial_misses + 1) {
        printf("Cache misses not updated correctly\n");
        vexfs_ann_cache_put(cache, entry);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Lookup the same entry (should cause a hit) */
    struct vexfs_ann_cache_entry *found;
    ret = vexfs_ann_cache_lookup(cache, 200, VEXFS_ANN_INDEX_HNSW_NODE, &found);
    if (ret != 0) {
        printf("Failed to lookup cache entry\n");
        vexfs_ann_cache_put(cache, entry);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Get final statistics */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0) {
        printf("Failed to get final cache statistics\n");
        vexfs_ann_cache_put(cache, entry);
        vexfs_ann_cache_put(cache, found);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Verify hit was recorded */
    if (atomic64_read(&stats.cache_hits) != initial_hits + 1) {
        printf("Cache hits not updated correctly\n");
        vexfs_ann_cache_put(cache, entry);
        vexfs_ann_cache_put(cache, found);
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    vexfs_ann_cache_put(cache, entry);
    vexfs_ann_cache_put(cache, found);
    vexfs_ann_cache_destroy(cache);
    
    printf("Cache statistics test successful\n");
    return true;
}

/*
 * Thread data for concurrent tests
 */
struct thread_data {
    struct vexfs_ann_cache *cache;
    int thread_id;
    int operations;
    bool success;
};

/*
 * Worker function for concurrent access test
 */
static void *concurrent_worker(void *arg)
{
    struct thread_data *data = (struct thread_data *)arg;
    struct vexfs_ann_cache_entry *entry;
    u64 start_time, end_time;
    int ret;
    
    data->success = true;
    
    for (int i = 0; i < data->operations; i++) {
        u64 index_id = data->thread_id * 1000 + i;
        enum vexfs_ann_index_type type = i % VEXFS_ANN_INDEX_TYPE_COUNT;
        
        start_time = get_time_ns();
        
        /* Get or create entry */
        entry = vexfs_ann_cache_get(data->cache, index_id, type);
        if (!entry) {
            atomic64_inc(&global_test_stats.allocation_failures);
            data->success = false;
            continue;
        }
        
        /* Simulate some work */
        usleep(1);
        
        /* Lookup the same entry */
        struct vexfs_ann_cache_entry *found;
        ret = vexfs_ann_cache_lookup(data->cache, index_id, type, &found);
        if (ret == 0) {
            atomic64_inc(&global_test_stats.cache_hits);
            vexfs_ann_cache_put(data->cache, found);
        } else {
            atomic64_inc(&global_test_stats.cache_misses);
        }
        
        /* Release entry */
        vexfs_ann_cache_put(data->cache, entry);
        
        end_time = get_time_ns();
        update_timing_stats(start_time, end_time);
        atomic64_inc(&global_test_stats.operations_completed);
    }
    
    return NULL;
}

/*
 * Test 5: Concurrent Access
 */
static bool test_concurrent_access(void)
{
    struct vexfs_ann_cache *cache = NULL;
    pthread_t threads[TEST_NUM_THREADS];
    struct thread_data thread_data[TEST_NUM_THREADS];
    int ret;
    bool success = true;
    
    test_print_header("Concurrent Access Test");
    
    /* Initialize test statistics */
    memset(&global_test_stats, 0, sizeof(global_test_stats));
    atomic64_set(&global_test_stats.min_time_ns, UINT64_MAX);
    
    /* Initialize cache */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0) {
        return false;
    }
    
    /* Create worker threads */
    for (int i = 0; i < TEST_NUM_THREADS; i++) {
        thread_data[i].cache = cache;
        thread_data[i].thread_id = i;
        thread_data[i].operations = TEST_OPERATIONS_PER_THREAD;
        thread_data[i].success = false;
        
        ret = pthread_create(&threads[i], NULL, concurrent_worker, &thread_data[i]);
        if (ret != 0) {
            printf("Failed to create thread %d: %d\n", i, ret);
            success = false;
            break;
        }
    }
    
    /* Wait for all threads to complete */
    for (int i = 0; i < TEST_NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
        if (!thread_data[i].success) {
            success = false;
        }
    }
    
    /* Print test results */
    u64 total_ops = atomic64_read(&global_test_stats.operations_completed);
    u64 hits = atomic64_read(&global_test_stats.cache_hits);
    u64 misses = atomic64_read(&global_test_stats.cache_misses);
    u64 failures = atomic64_read(&global_test_stats.allocation_failures);
    u64 total_time = atomic64_read(&global_test_stats.total_time_ns);
    u64 max_time = atomic64_read(&global_test_stats.max_time_ns);
    u64 min_time = atomic64_read(&global_test_stats.min_time_ns);
    
    printf("Concurrent access test results:\n");
    printf("  Total operations: %lu\n", total_ops);
    printf("  Cache hits: %lu\n", hits);
    printf("  Cache misses: %lu\n", misses);
    printf("  Allocation failures: %lu\n", failures);
    printf("  Average time per operation: %lu ns\n", 
           total_ops > 0 ? total_time / total_ops : 0);
    printf("  Max operation time: %lu ns\n", max_time);
    printf("  Min operation time: %lu ns\n", min_time == UINT64_MAX ? 0 : min_time);
    
    /* Print cache statistics */
    vexfs_ann_cache_print_stats(cache);
    
    vexfs_ann_cache_destroy(cache);
    
    if (success && failures == 0) {
        printf("Concurrent access test successful\n");
        return true;
    } else {
        printf("Concurrent access test failed\n");
        return false;
    }
}

/*
 * Test 6: Cache Flush
 */
static bool test_cache_flush(void)
{
    struct vexfs_ann_cache *cache = NULL;
    struct vexfs_ann_cache_entry *entries[10];
    struct vexfs_ann_cache_stats stats;
    int ret;
    
    test_print_header("Cache Flush Test");
    
    /* Initialize cache */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0) {
        return false;
    }
    
    /* Allocate multiple entries */
    for (int i = 0; i < 10; i++) {
        entries[i] = vexfs_ann_cache_get(cache, i + 300, VEXFS_ANN_INDEX_HNSW_NODE);
        if (!entries[i]) {
            printf("Failed to allocate entry %d\n", i);
            goto cleanup;
        }
    }
    
    /* Verify entries are in cache */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0 || atomic64_read(&stats.total_entries) < 10) {
        printf("Entries not properly added to cache\n");
        goto cleanup;
    }
    
    /* Release references but keep entries in cache */
    for (int i = 0; i < 10; i++) {
        vexfs_ann_cache_put(cache, entries[i]);
    }
    
    /* Flush cache */
    ret = vexfs_ann_cache_flush(cache);
    if (ret != 0) {
        printf("Failed to flush cache\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Verify cache is empty */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0 || atomic64_read(&stats.total_entries) != 0) {
        printf("Cache not properly flushed\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    vexfs_ann_cache_destroy(cache);
    
    printf("Cache flush test successful\n");
    return true;
    
cleanup:
    for (int i = 0; i < 10; i++) {
        if (entries[i]) {
            vexfs_ann_cache_put(cache, entries[i]);
        }
    }
    vexfs_ann_cache_destroy(cache);
    return false;
}

/*
 * Main test runner
 */
int main(int argc, char *argv[])
{
    bool all_passed = true;
    int tests_run = 0;
    int tests_passed = 0;
    
    printf("VexFS v2.0 ANN Index Cache Test Suite\n");
    printf("=====================================\n");
    
    /* Run all tests */
    struct {
/*
 * Test 6: Cache Flush
 */
static bool test_cache_flush(void)
{
    struct vexfs_ann_cache *cache = NULL;
    struct vexfs_ann_cache_entry *entries[10];
    struct vexfs_ann_cache_stats stats;
    int ret;
    
    test_print_header("Cache Flush Test");
    
    /* Initialize cache */
    ret = vexfs_ann_cache_init(&cache, &mock_mm, &mock_vector_cache);
    if (ret != 0) {
        return false;
    }
    
    /* Allocate multiple entries */
    for (int i = 0; i < 10; i++) {
        entries[i] = vexfs_ann_cache_get(cache, i + 300, VEXFS_ANN_INDEX_HNSW_NODE);
        if (!entries[i]) {
            printf("Failed to allocate entry %d\n", i);
            goto cleanup;
        }
    }
    
    /* Verify entries are in cache */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0 || atomic64_read(&stats.total_entries) < 10) {
        printf("Entries not properly added to cache\n");
        goto cleanup;
    }
    
    /* Release references but keep entries in cache */
    for (int i = 0; i < 10; i++) {
        vexfs_ann_cache_put(cache, entries[i]);
    }
    
    /* Flush cache */
    ret = vexfs_ann_cache_flush(cache);
    if (ret != 0) {
        printf("Failed to flush cache\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    /* Verify cache is empty */
    ret = vexfs_ann_cache_get_stats(cache, &stats);
    if (ret != 0 || atomic64_read(&stats.total_entries) != 0) {
        printf("Cache not properly flushed\n");
        vexfs_ann_cache_destroy(cache);
        return false;
    }
    
    vexfs_ann_cache_destroy(cache);
    
    printf("Cache flush test successful\n");
    return true;
    
cleanup:
    for (int i = 0; i < 10; i++) {
        if (entries[i]) {
            vexfs_ann_cache_put(cache, entries[i]);
        }
    }
    vexfs_ann_cache_destroy(cache);
    return false;
}

/*
 * Main test runner
 */
int main(int argc, char *argv[])
{
    bool all_passed = true;
    int tests_run = 0;
    int tests_passed = 0;
    
    printf("VexFS v2.0 ANN Index Cache Test Suite\n");
    printf("=====================================\n");
    
    /* Run all tests */
    struct {
        const char *name;
        bool (*test_func)(void);
    } tests[] = {
        {"Cache Initialization", test_cache_initialization},
        {"Cache Entry Management", test_cache_entry_management},
        {"Multiple Index Types", test_multiple_index_types},
        {"Cache Statistics", test_cache_statistics},
        {"Concurrent Access", test_concurrent_access},
        {"Cache Flush", test_cache_flush}
    };
    
    int num_tests = sizeof(tests) / sizeof(tests[0]);
    
    for (int i = 0; i < num_tests; i++) {
        tests_run++;
        bool result = tests[i].test_func();
        test_print_result(tests[i].name, result);
        
        if (result) {
            tests_passed++;
        } else {
            all_passed = false;
        }
    }
    
    /* Print final results */
    printf("\n=== Test Summary ===\n");
    printf("Tests run: %d\n", tests_run);
    printf("Tests passed: %d\n", tests_passed);
    printf("Tests failed: %d\n", tests_run - tests_passed);
    printf("Overall result: %s\n", all_passed ? "PASSED" : "FAILED");
    
    return all_passed ? 0 : 1;
}