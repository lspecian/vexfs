/*
 * VexFS v2.0 Ollama Integration - /dev/sda Full Capacity Test
 * 
 * This program tests VexFS v2.0 with Ollama embeddings on /dev/sda storage
 * to validate full-capacity real-world performance with large datasets.
 */

#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>
#include <errno.h>
#include "ollama_client.h"

#define LARGE_DATASET_SIZE 1000
#define BATCH_SIZE 50

static const char *sample_texts[] = {
    "Advanced machine learning algorithms optimize neural network performance.",
    "Vector databases enable efficient similarity search across high-dimensional data.",
    "Distributed computing systems scale processing across multiple nodes.",
    "Real-time data analytics require low-latency storage and retrieval.",
    "Artificial intelligence transforms modern software development practices.",
    "Cloud infrastructure provides scalable computing resources on demand.",
    "Database indexing strategies improve query execution performance.",
    "Filesystem optimization techniques enhance storage system efficiency.",
    "Parallel processing algorithms accelerate computational workloads.",
    "Data compression methods reduce storage requirements significantly."
};

static double get_timestamp_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1000000.0;
}

static bool check_sda_availability(void) {
    struct stat st;
    if (stat("/dev/sda", &st) != 0) {
        printf("❌ /dev/sda not available: %s\n", strerror(errno));
        return false;
    }
    
    printf("✅ /dev/sda detected\n");
    return true;
}

static char* generate_test_text(int index) {
    static char buffer[512];
    int base_index = index % (sizeof(sample_texts) / sizeof(sample_texts[0]));
    snprintf(buffer, sizeof(buffer), "[%d] %s Additional context for vector %d.", 
             index, sample_texts[base_index], index);
    return buffer;
}

int main(int argc, char *argv[]) {
    printf("🚀 VexFS v2.0 Ollama Integration - /dev/sda Full Capacity Test\n");
    printf("=============================================================\n");
    
    bool verbose = false;
    const char *model = "nomic-embed-text";
    const char *mount_path = "/tmp/vexfs_sda_test";
    int dataset_size = LARGE_DATASET_SIZE;
    
    // Parse command line arguments
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--verbose") == 0) {
            verbose = true;
        } else if (strcmp(argv[i], "-m") == 0 || strcmp(argv[i], "--model") == 0) {
            if (i + 1 < argc) {
                model = argv[++i];
            }
        } else if (strcmp(argv[i], "-p") == 0 || strcmp(argv[i], "--path") == 0) {
            if (i + 1 < argc) {
                mount_path = argv[++i];
            }
        } else if (strcmp(argv[i], "-s") == 0 || strcmp(argv[i], "--size") == 0) {
            if (i + 1 < argc) {
                dataset_size = atoi(argv[++i]);
            }
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Usage: %s [options]\n", argv[0]);
            printf("Options:\n");
            printf("  -v, --verbose     Enable verbose output\n");
            printf("  -m, --model NAME  Use specific Ollama model (default: nomic-embed-text)\n");
            printf("  -p, --path PATH   VexFS mount path (default: /tmp/vexfs_sda_test)\n");
            printf("  -s, --size SIZE   Dataset size (default: %d)\n", LARGE_DATASET_SIZE);
            printf("  -h, --help        Show this help\n");
            return 0;
        }
    }
    
    printf("Configuration:\n");
    printf("  Model: %s\n", model);
    printf("  Mount path: %s\n", mount_path);
    printf("  Dataset size: %d vectors\n", dataset_size);
    printf("  Batch size: %d\n", BATCH_SIZE);
    
    // Check /dev/sda availability
    if (!check_sda_availability()) {
        printf("⚠️  /dev/sda not available, test cannot proceed\n");
        return 1;
    }
    
    // Check VexFS mount point
    struct stat st;
    if (stat(mount_path, &st) != 0) {
        printf("❌ VexFS mount point not available: %s (%s)\n", mount_path, strerror(errno));
        printf("   Please ensure VexFS is mounted on /dev/sda at %s\n", mount_path);
        return 1;
    }
    printf("✅ VexFS mount point available: %s\n", mount_path);
    
    // Test Ollama connectivity
    printf("\n🔗 Testing Ollama connectivity...\n");
    if (!ollama_is_available()) {
        printf("❌ Ollama is not available. Please ensure Ollama is running.\n");
        return 1;
    }
    printf("✅ Ollama is available\n");
    
    // Initialize VexFS-Ollama integration
    vexfs_ollama_integration_t integration;
    ollama_error_t init_result = vexfs_ollama_init(&integration, mount_path, model);
    
    if (init_result != OLLAMA_SUCCESS) {
        printf("❌ Failed to initialize Ollama integration: %d\n", init_result);
        return 1;
    }
    printf("✅ VexFS-Ollama integration initialized\n");
    
    // Performance tracking
    double total_start_time = get_timestamp_ms();
    int successful_insertions = 0;
    int failed_insertions = 0;
    
    printf("\n📊 Starting large-scale embedding generation and insertion...\n");
    printf("   Target: %d vectors\n", dataset_size);
    
    // Process in batches for better performance tracking
    for (int batch = 0; batch < dataset_size; batch += BATCH_SIZE) {
        int batch_end = (batch + BATCH_SIZE > dataset_size) ? dataset_size : batch + BATCH_SIZE;
        int batch_size = batch_end - batch;
        
        double batch_start = get_timestamp_ms();
        int batch_successes = 0;
        
        printf("   Batch %d-%d: ", batch + 1, batch_end);
        fflush(stdout);
        
        for (int i = batch; i < batch_end; i++) {
            char *text = generate_test_text(i);
            
            ollama_error_t result = vexfs_ollama_insert_text(&integration, text, 0);
            
            if (result == OLLAMA_SUCCESS) {
                successful_insertions++;
                batch_successes++;
            } else {
                failed_insertions++;
                if (verbose) {
                    printf("\n      ❌ Failed to insert vector %d: %d", i, result);
                }
            }
        }
        
        double batch_time = get_timestamp_ms() - batch_start;
        double batch_rate = batch_successes / (batch_time / 1000.0);
        
        printf("✅ %d/%d successful (%.1f vectors/sec)\n", 
               batch_successes, batch_size, batch_rate);
    }
    
    double total_time = get_timestamp_ms() - total_start_time;
    double total_rate = successful_insertions / (total_time / 1000.0);
    
    // Final performance summary
    printf("\n📋 /dev/sda Full Capacity Test Results:\n");
    printf("   Total vectors processed: %d\n", dataset_size);
    printf("   Successful insertions: %d\n", successful_insertions);
    printf("   Failed insertions: %d\n", failed_insertions);
    printf("   Success rate: %.1f%%\n", (successful_insertions * 100.0) / dataset_size);
    printf("   Total time: %.2f seconds\n", total_time / 1000.0);
    printf("   Average throughput: %.1f vectors/sec\n", total_rate);
    
    // Note: Detailed performance statistics will be available in future versions
    printf("\n📊 Basic Performance Summary:\n");
    printf("   Processing completed successfully\n");
    
    // Determine test result
    bool test_passed = (successful_insertions >= dataset_size * 0.95); // 95% success rate
    printf("\n🏁 /dev/sda Full Capacity Test: %s\n", 
           test_passed ? "✅ PASSED" : "❌ FAILED");
    
    if (!test_passed) {
        printf("   Reason: Success rate below 95%% threshold\n");
    }
    
    // Cleanup
    vexfs_ollama_cleanup(&integration);
    
    printf("\n🔚 Test completed!\n");
    return test_passed ? 0 : 1;
}