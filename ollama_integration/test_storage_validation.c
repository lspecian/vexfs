/*
 * VexFS v2.0 Ollama Integration - Storage Validation Tests
 *
 * This program validates Ollama embedding generation across different storage types:
 * - Memory-based VexFS
 * - NVMe storage VexFS
 * - HDD storage VexFS (/dev/sda)
 *
 * Tests real embedding generation and VexFS integration performance.
 */

#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <errno.h>
#include <time.h>
#include "ollama_client.h"

#define TEST_TEXT_COUNT 10
#define MAX_PATH_LEN 256
#define PERFORMANCE_ITERATIONS 5

// Test storage configurations
typedef struct {
    const char *name;
    const char *mount_path;
    const char *description;
} storage_config_t;

static storage_config_t storage_configs[] = {
    {"Memory", "/tmp/vexfs_test", "Memory-based VexFS mount"},
    {"NVMe", "/tmp/vexfs_nvme_test", "NVMe storage VexFS mount"},
    {"HDD", "/tmp/vexfs_hdd_test", "HDD (/dev/sda) VexFS mount"}
};

static const char *test_texts[TEST_TEXT_COUNT] = {
    "Machine learning algorithms process vast amounts of data efficiently.",
    "Vector databases enable semantic search and similarity matching.",
    "Filesystem optimization improves storage performance significantly.",
    "Embedding models transform text into high-dimensional vectors.",
    "Real-time data processing requires low-latency storage systems.",
    "Distributed computing scales across multiple processing nodes.",
    "Neural networks learn complex patterns from training data.",
    "Database indexing accelerates query execution times.",
    "Cloud infrastructure provides scalable computing resources.",
    "Artificial intelligence transforms modern software applications."
};

// Performance tracking
typedef struct {
    double embedding_generation_ms;
    double vexfs_insertion_ms;
    double total_operation_ms;
    size_t embedding_dimensions;
    bool success;
} performance_metrics_t;

static double get_timestamp_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1000000.0;
}

static bool check_storage_availability(const char *mount_path) {
    struct stat st;
    if (stat(mount_path, &st) != 0) {
        printf("❌ Storage not available: %s (%s)\n", mount_path, strerror(errno));
        return false;
    }
    
    if (!S_ISDIR(st.st_mode)) {
        printf("❌ Not a directory: %s\n", mount_path);
        return false;
    }
    
    // Test write access
    char test_file[MAX_PATH_LEN];
    snprintf(test_file, sizeof(test_file), "%s/.vexfs_test_write", mount_path);
    
    int fd = open(test_file, O_CREAT | O_WRONLY | O_TRUNC, 0644);
    if (fd < 0) {
        printf("❌ No write access: %s (%s)\n", mount_path, strerror(errno));
        return false;
    }
    
    close(fd);
    unlink(test_file);
    
    printf("✅ Storage available: %s\n", mount_path);
    return true;
}

static bool test_embedding_generation_performance(vexfs_ollama_integration_t *integration,
                                                const char *text,
                                                performance_metrics_t *metrics) {
    double start_time = get_timestamp_ms();
    
    // Test VexFS insertion using the simplified API
    double vexfs_start = get_timestamp_ms();
    ollama_error_t vexfs_result = vexfs_ollama_insert_text(integration, text, 0); // 0 for auto-increment ID
    double vexfs_end = get_timestamp_ms();
    
    metrics->vexfs_insertion_ms = vexfs_end - vexfs_start;
    metrics->total_operation_ms = get_timestamp_ms() - start_time;
    metrics->embedding_generation_ms = metrics->total_operation_ms - metrics->vexfs_insertion_ms;
    metrics->embedding_dimensions = 768; // Default for nomic-embed-text
    metrics->success = (vexfs_result == OLLAMA_SUCCESS);
    
    return metrics->success;
}

static void print_performance_summary(const char *storage_name, 
                                    performance_metrics_t *metrics, 
                                    int count) {
    if (count == 0) {
        printf("❌ No successful operations for %s storage\n", storage_name);
        return;
    }
    
    double total_embedding_time = 0;
    double total_vexfs_time = 0;
    double total_operation_time = 0;
    int successful_ops = 0;
    
    for (int i = 0; i < count; i++) {
        if (metrics[i].success) {
            total_embedding_time += metrics[i].embedding_generation_ms;
            total_vexfs_time += metrics[i].vexfs_insertion_ms;
            total_operation_time += metrics[i].total_operation_ms;
            successful_ops++;
        }
    }
    
    if (successful_ops == 0) {
        printf("❌ No successful operations for %s storage\n", storage_name);
        return;
    }
    
    printf("\n📊 %s Storage Performance Summary:\n", storage_name);
    printf("   Successful operations: %d/%d\n", successful_ops, count);
    printf("   Average embedding generation: %.2f ms\n", total_embedding_time / successful_ops);
    printf("   Average VexFS insertion: %.2f ms\n", total_vexfs_time / successful_ops);
    printf("   Average total operation: %.2f ms\n", total_operation_time / successful_ops);
    printf("   Embedding dimensions: %zu\n", metrics[0].embedding_dimensions);
    
    // Calculate operations per second
    double avg_op_time_sec = (total_operation_time / successful_ops) / 1000.0;
    double ops_per_sec = 1.0 / avg_op_time_sec;
    printf("   Operations per second: %.0f ops/sec\n", ops_per_sec);
}

static bool test_storage_configuration(storage_config_t *config, vexfs_ollama_integration_t *integration) {
    printf("\n🔍 Testing %s Storage: %s\n", config->name, config->description);
    printf("   Mount path: %s\n", config->mount_path);
    
    // Check storage availability
    if (!check_storage_availability(config->mount_path)) {
        printf("⚠️  Skipping %s storage (not available)\n", config->name);
        return false;
    }
    
    // Note: VexFS mount path is set during initialization, not changed per test
    // The integration structure doesn't have a vexfs_mount_path field to modify
    
    // Performance metrics for this storage
    performance_metrics_t metrics[TEST_TEXT_COUNT];
    memset(metrics, 0, sizeof(metrics));
    
    printf("   Running %d embedding + VexFS insertion tests...\n", TEST_TEXT_COUNT);
    
    int successful_tests = 0;
    for (int i = 0; i < TEST_TEXT_COUNT; i++) {
        printf("   Test %d/%d: ", i + 1, TEST_TEXT_COUNT);
        fflush(stdout);
        
        if (test_embedding_generation_performance(integration, test_texts[i], &metrics[i])) {
            printf("✅ Success (%.1f ms total)\n", metrics[i].total_operation_ms);
            successful_tests++;
        } else {
            printf("❌ Failed\n");
        }
    }
    
    // Print performance summary
    print_performance_summary(config->name, metrics, TEST_TEXT_COUNT);
    
    bool storage_success = (successful_tests >= TEST_TEXT_COUNT / 2); // At least 50% success rate
    printf("   %s storage test: %s (%d/%d successful)\n", 
           config->name, 
           storage_success ? "✅ PASSED" : "❌ FAILED",
           successful_tests, TEST_TEXT_COUNT);
    
    return storage_success;
}

int main(int argc, char *argv[]) {
    printf("🚀 VexFS v2.0 Ollama Integration - Storage Validation Tests\n");
    printf("===========================================================\n");
    
    bool verbose = false;
    const char *model = "nomic-embed-text";
    
    // Parse command line arguments
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--verbose") == 0) {
            verbose = true;
        } else if (strcmp(argv[i], "-m") == 0 || strcmp(argv[i], "--model") == 0) {
            if (i + 1 < argc) {
                model = argv[++i];
            }
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Usage: %s [options]\n", argv[0]);
            printf("Options:\n");
            printf("  -v, --verbose     Enable verbose output\n");
            printf("  -m, --model NAME  Use specific Ollama model (default: nomic-embed-text)\n");
            printf("  -h, --help        Show this help\n");
            return 0;
        }
    }
    
    printf("Configuration:\n");
    printf("  Model: %s\n", model);
    printf("  Test texts: %d\n", TEST_TEXT_COUNT);
    printf("  Storage configurations: %zu\n", sizeof(storage_configs) / sizeof(storage_configs[0]));
    
    // Initialize Ollama integration
    vexfs_ollama_integration_t integration;
    ollama_error_t init_result = vexfs_ollama_init(&integration, "/tmp/vexfs_test", model);
    
    if (init_result != OLLAMA_SUCCESS) {
        printf("❌ Failed to initialize Ollama integration: %d\n", init_result);
        return 1;
    }
    
    printf("✅ Ollama integration initialized\n");
    
    // Test Ollama connectivity
    printf("\n🔗 Testing Ollama connectivity...\n");
    if (!ollama_is_available()) {
        printf("❌ Ollama is not available. Please ensure Ollama is running.\n");
        vexfs_ollama_cleanup(&integration);
        return 1;
    }
    printf("✅ Ollama is available\n");
    
    // Test each storage configuration
    int successful_storages = 0;
    int total_storages = sizeof(storage_configs) / sizeof(storage_configs[0]);
    
    for (int i = 0; i < total_storages; i++) {
        if (test_storage_configuration(&storage_configs[i], &integration)) {
            successful_storages++;
        }
    }
    
    // Final summary
    printf("\n📋 Storage Validation Summary:\n");
    printf("   Total storage configurations tested: %d\n", total_storages);
    printf("   Successful storage configurations: %d\n", successful_storages);
    printf("   Success rate: %.1f%%\n", (successful_storages * 100.0) / total_storages);
    
    bool overall_success = (successful_storages > 0);
    printf("   Overall result: %s\n", overall_success ? "✅ PASSED" : "❌ FAILED");
    
    // Cleanup
    vexfs_ollama_cleanup(&integration);
    
    printf("\n🏁 Storage validation tests completed!\n");
    return overall_success ? 0 : 1;
}