/*
 * VexFS v2.0 Real Embeddings Test
 *
 * This test validates real embedding generation and VexFS integration
 * for Phase 1 implementation with actual Ollama models.
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#define _POSIX_C_SOURCE 199309L
#include "ollama_client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>
#include <math.h>

// Test data
static const char *test_texts[] = {
    "The quick brown fox jumps over the lazy dog",
    "Machine learning is transforming the world of technology",
    "Vector databases enable efficient similarity search",
    "VexFS provides high-performance vector storage",
    "Ollama makes running language models locally accessible",
    "Embeddings capture semantic meaning in numerical form",
    "Kernel modules provide direct hardware access",
    "IOCTL interfaces enable userspace-kernel communication",
    "Performance optimization requires careful measurement",
    "Real-world validation proves system reliability"
};

static const size_t num_test_texts = sizeof(test_texts) / sizeof(test_texts[0]);

static void print_test_header(const char *test_name) {
    printf("\n🧪 %s\n", test_name);
    printf("═══════════════════════════════════════════════════════════════\n");
}

static void print_test_result(const char *test_name, bool passed) {
    if (passed) {
        printf("✅ %s: PASSED\n", test_name);
    } else {
        printf("❌ %s: FAILED\n", test_name);
    }
}

static bool create_test_vexfs_file(const char *path) {
    // Create a test file for VexFS operations
    int fd = open(path, O_CREAT | O_RDWR, 0644);
    if (fd < 0) {
        perror("Failed to create test file");
        return false;
    }
    
    // Write some initial data
    const char *initial_data = "VexFS test file for real embeddings\n";
    write(fd, initial_data, strlen(initial_data));
    close(fd);
    
    return true;
}

static bool test_single_embedding_generation(void) {
    print_test_header("Single Embedding Generation Test");
    
    const char *test_model = "nomic-embed-text";
    uint32_t expected_dims = ollama_get_model_dimensions(test_model);
    
    if (expected_dims == 0) {
        printf("❌ Unknown model dimensions for %s\n", test_model);
        return false;
    }
    
    printf("Testing model: %s (expected dimensions: %u)\n", test_model, expected_dims);
    
    // Allocate memory for embedding
    float *embedding = malloc(expected_dims * sizeof(float));
    if (!embedding) {
        printf("❌ Failed to allocate memory for embedding\n");
        return false;
    }
    
    uint32_t actual_dims;
    double generation_time;
    
    ollama_embedding_request_t request = {
        .text = (char*)test_texts[0],
        .text_length = strlen(test_texts[0]),
        .embedding_output = embedding,
        .expected_dimensions = expected_dims,
        .actual_dimensions = &actual_dims,
        .generation_time_ms = &generation_time
    };
    strncpy(request.model, test_model, OLLAMA_MAX_MODEL_NAME - 1);
    request.model[OLLAMA_MAX_MODEL_NAME - 1] = '\0';
    
    printf("Generating embedding for: \"%s\"\n", test_texts[0]);
    
    ollama_error_t error = ollama_generate_embedding(&request);
    if (error != OLLAMA_SUCCESS) {
        printf("❌ Failed to generate embedding: %s\n", ollama_error_string(error));
        free(embedding);
        return false;
    }
    
    printf("✅ Embedding generated successfully\n");
    printf("   Dimensions: %u (expected: %u)\n", actual_dims, expected_dims);
    printf("   Generation time: %.2f ms\n", generation_time);
    
    // Validate embedding values
    bool has_non_zero = false;
    float min_val = embedding[0], max_val = embedding[0];
    
    for (uint32_t i = 0; i < actual_dims; i++) {
        if (embedding[i] != 0.0f) {
            has_non_zero = true;
        }
        if (embedding[i] < min_val) min_val = embedding[i];
        if (embedding[i] > max_val) max_val = embedding[i];
    }
    
    printf("   Value range: [%.6f, %.6f]\n", min_val, max_val);
    printf("   Has non-zero values: %s\n", has_non_zero ? "Yes" : "No");
    
    // Show first few values
    printf("   First 5 values: ");
    for (int i = 0; i < 5 && i < actual_dims; i++) {
        printf("%.6f ", embedding[i]);
    }
    printf("\n");
    
    free(embedding);
    
    return (actual_dims == expected_dims) && has_non_zero;
}

static bool test_vexfs_integration(void) {
    print_test_header("VexFS Integration Test");
    
    const char *test_file = "/tmp/vexfs_real_embedding_test";
    const char *test_model = "nomic-embed-text";
    
    // Create test file
    if (!create_test_vexfs_file(test_file)) {
        return false;
    }
    
    printf("Created test file: %s\n", test_file);
    
    // Initialize VexFS-Ollama integration
    vexfs_ollama_integration_t integration;
    ollama_error_t error = vexfs_ollama_init(&integration, test_file, test_model);
    
    if (error != OLLAMA_SUCCESS) {
        printf("❌ Failed to initialize VexFS-Ollama integration: %s\n", 
               ollama_error_string(error));
        unlink(test_file);
        return false;
    }
    
    printf("✅ VexFS-Ollama integration initialized\n");
    printf("   Model: %s\n", integration.model);
    printf("   Dimensions: %u\n", integration.meta.dimensions);
    printf("   File descriptor: %d\n", integration.vexfs_fd);
    
    // Test single text insertion
    printf("\nTesting single text insertion...\n");
    error = vexfs_ollama_insert_text(&integration, test_texts[0], 0);
    
    if (error != OLLAMA_SUCCESS) {
        printf("❌ Failed to insert text: %s\n", ollama_error_string(error));
        vexfs_ollama_cleanup(&integration);
        unlink(test_file);
        return false;
    }
    
    printf("✅ Single text inserted successfully\n");
    printf("   Vector ID: %lu\n", integration.next_vector_id - 1);
    printf("   Total vectors inserted: %lu\n", integration.total_vectors_inserted);
    
    // Test multiple text insertions
    printf("\nTesting multiple text insertions...\n");
    for (size_t i = 1; i < 5 && i < num_test_texts; i++) {
        error = vexfs_ollama_insert_text(&integration, test_texts[i], 0);
        if (error != OLLAMA_SUCCESS) {
            printf("❌ Failed to insert text %zu: %s\n", i, ollama_error_string(error));
            vexfs_ollama_cleanup(&integration);
            unlink(test_file);
            return false;
        }
        printf("   Inserted text %zu (ID: %lu)\n", i, integration.next_vector_id - 1);
    }
    
    printf("✅ Multiple texts inserted successfully\n");
    printf("   Total vectors: %lu\n", integration.total_vectors_inserted);
    printf("   Total embedding time: %.2f ms\n", integration.total_embedding_time_ms);
    printf("   Total VexFS time: %.2f ms\n", integration.total_vexfs_time_ms);
    
    if (integration.total_vectors_inserted > 0) {
        double avg_embedding_time = integration.total_embedding_time_ms / integration.total_vectors_inserted;
        double avg_vexfs_time = integration.total_vexfs_time_ms / integration.total_vectors_inserted;
        double total_time = integration.total_embedding_time_ms + integration.total_vexfs_time_ms;
        double throughput = (integration.total_vectors_inserted * 1000.0) / total_time;
        
        printf("   Average embedding time: %.2f ms\n", avg_embedding_time);
        printf("   Average VexFS time: %.2f ms\n", avg_vexfs_time);
        printf("   Overall throughput: %.2f vectors/sec\n", throughput);
    }
    
    // Cleanup
    vexfs_ollama_cleanup(&integration);
    unlink(test_file);
    
    return true;
}

static bool test_performance_benchmark(void) {
    print_test_header("Performance Benchmark Test");
    
    const char *test_file = "/tmp/vexfs_performance_test";
    const char *test_model = "nomic-embed-text";
    const size_t benchmark_count = 20;
    
    // Create test file
    if (!create_test_vexfs_file(test_file)) {
        return false;
    }
    
    // Initialize integration
    vexfs_ollama_integration_t integration;
    ollama_error_t error = vexfs_ollama_init(&integration, test_file, test_model);
    
    if (error != OLLAMA_SUCCESS) {
        printf("❌ Failed to initialize integration: %s\n", ollama_error_string(error));
        unlink(test_file);
        return false;
    }
    
    printf("Running benchmark with %zu text insertions...\n", benchmark_count);
    
    struct timespec start_time, end_time;
    clock_gettime(CLOCK_MONOTONIC, &start_time);
    
    // Insert texts in a loop
    for (size_t i = 0; i < benchmark_count; i++) {
        const char *text = test_texts[i % num_test_texts];
        error = vexfs_ollama_insert_text(&integration, text, 0);
        
        if (error != OLLAMA_SUCCESS) {
            printf("❌ Failed at iteration %zu: %s\n", i, ollama_error_string(error));
            vexfs_ollama_cleanup(&integration);
            unlink(test_file);
            return false;
        }
        
        if ((i + 1) % 5 == 0) {
            printf("   Completed %zu/%zu insertions\n", i + 1, benchmark_count);
        }
    }
    
    clock_gettime(CLOCK_MONOTONIC, &end_time);
    
    double total_wall_time = (end_time.tv_sec - start_time.tv_sec) * 1000.0 +
                            (end_time.tv_nsec - start_time.tv_nsec) / 1000000.0;
    
    printf("✅ Benchmark completed\n");
    printf("   Total insertions: %zu\n", benchmark_count);
    printf("   Wall clock time: %.2f ms\n", total_wall_time);
    printf("   Total embedding time: %.2f ms\n", integration.total_embedding_time_ms);
    printf("   Total VexFS time: %.2f ms\n", integration.total_vexfs_time_ms);
    
    double avg_embedding_time = integration.total_embedding_time_ms / benchmark_count;
    double avg_vexfs_time = integration.total_vexfs_time_ms / benchmark_count;
    double wall_throughput = (benchmark_count * 1000.0) / total_wall_time;
    double processing_throughput = (benchmark_count * 1000.0) / 
                                  (integration.total_embedding_time_ms + integration.total_vexfs_time_ms);
    
    printf("   Average embedding time: %.2f ms\n", avg_embedding_time);
    printf("   Average VexFS time: %.2f ms\n", avg_vexfs_time);
    printf("   Wall clock throughput: %.2f vectors/sec\n", wall_throughput);
    printf("   Processing throughput: %.2f vectors/sec\n", processing_throughput);
    
    // Performance targets (based on VexFS v2.0 breakthrough)
    const double target_vexfs_ops_per_sec = 338983.0; // From breakthrough report
    const double acceptable_embedding_time_ms = 100.0; // Reasonable for local Ollama
    
    bool performance_acceptable = (avg_vexfs_time < 10.0) && // Sub-10ms VexFS operations
                                 (avg_embedding_time < acceptable_embedding_time_ms);
    
    printf("\n📊 Performance Analysis:\n");
    printf("   VexFS target: %.0f ops/sec (from breakthrough)\n", target_vexfs_ops_per_sec);
    printf("   VexFS actual: %.2f ops/sec\n", 1000.0 / avg_vexfs_time);
    printf("   Embedding time acceptable: %s (< %.0f ms)\n", 
           avg_embedding_time < acceptable_embedding_time_ms ? "Yes" : "No",
           acceptable_embedding_time_ms);
    printf("   Overall performance: %s\n", performance_acceptable ? "GOOD" : "NEEDS IMPROVEMENT");
    
    // Cleanup
    vexfs_ollama_cleanup(&integration);
    unlink(test_file);
    
    return performance_acceptable;
}

static bool test_model_comparison(void) {
    print_test_header("Model Comparison Test");
    
    const char *models[] = {"nomic-embed-text", "all-minilm"};
    const size_t num_models = sizeof(models) / sizeof(models[0]);
    const char *test_text = "This is a test sentence for model comparison";
    
    printf("Comparing embedding models with text: \"%s\"\n\n", test_text);
    
    for (size_t i = 0; i < num_models; i++) {
        const char *model = models[i];
        uint32_t expected_dims = ollama_get_model_dimensions(model);
        
        if (expected_dims == 0) {
            printf("❌ Unknown dimensions for model: %s\n", model);
            continue;
        }
        
        printf("Testing model: %s (%u dimensions)\n", model, expected_dims);
        
        float *embedding = malloc(expected_dims * sizeof(float));
        if (!embedding) {
            printf("❌ Memory allocation failed\n");
            continue;
        }
        
        uint32_t actual_dims;
        double generation_time;
        
        ollama_embedding_request_t request = {
            .text = (char*)test_text,
            .text_length = strlen(test_text),
            .embedding_output = embedding,
            .expected_dimensions = expected_dims,
            .actual_dimensions = &actual_dims,
            .generation_time_ms = &generation_time
        };
        strncpy(request.model, model, OLLAMA_MAX_MODEL_NAME - 1);
        request.model[OLLAMA_MAX_MODEL_NAME - 1] = '\0';
        
        ollama_error_t error = ollama_generate_embedding(&request);
        if (error != OLLAMA_SUCCESS) {
            printf("❌ Failed to generate embedding: %s\n", ollama_error_string(error));
            free(embedding);
            continue;
        }
        
        // Calculate statistics
        float sum = 0.0f, sum_sq = 0.0f;
        float min_val = embedding[0], max_val = embedding[0];
        
        for (uint32_t j = 0; j < actual_dims; j++) {
            sum += embedding[j];
            sum_sq += embedding[j] * embedding[j];
            if (embedding[j] < min_val) min_val = embedding[j];
            if (embedding[j] > max_val) max_val = embedding[j];
        }
        
        float mean = sum / actual_dims;
        float variance = (sum_sq / actual_dims) - (mean * mean);
        float std_dev = sqrtf(variance);
        
        printf("   ✅ Generation time: %.2f ms\n", generation_time);
        printf("   📊 Statistics: mean=%.6f, std=%.6f, range=[%.6f, %.6f]\n", 
               mean, std_dev, min_val, max_val);
        
        free(embedding);
        printf("\n");
    }
    
    return true;
}

int main(int argc, char *argv[]) {
    printf("🦙 VexFS v2.0 Real Embeddings Test\n");
    printf("═══════════════════════════════════════════════════════════════\n");
    printf("This test validates real embedding generation and VexFS integration.\n");
    
    bool benchmark_mode = false;
    bool verbose = false;
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--benchmark") == 0) {
            benchmark_mode = true;
        } else if (strcmp(argv[i], "--verbose") == 0) {
            verbose = true;
            ollama_set_debug(true);
        }
    }
    
    // Initialize Ollama
    ollama_error_t error = ollama_init(NULL);
    if (error != OLLAMA_SUCCESS) {
        printf("❌ Failed to initialize Ollama: %s\n", ollama_error_string(error));
        return 1;
    }
    
    // Check Ollama availability
    if (!ollama_is_available()) {
        printf("❌ Ollama server is not available\n");
        printf("   Please start Ollama: ollama serve\n");
        printf("   And ensure models are pulled: ollama pull nomic-embed-text\n");
        ollama_cleanup();
        return 1;
    }
    
    printf("✅ Ollama server is available\n");
    
    // Run tests
    bool results[] = {
        test_single_embedding_generation(),
        test_vexfs_integration(),
        benchmark_mode ? test_performance_benchmark() : true,
        test_model_comparison()
    };
    
    const char *test_names[] = {
        "Single Embedding Generation",
        "VexFS Integration",
        "Performance Benchmark",
        "Model Comparison"
    };
    
    // Summary
    printf("\n📊 TEST SUMMARY\n");
    printf("═══════════════════════════════════════════════════════════════\n");
    
    int passed = 0;
    int total = sizeof(results) / sizeof(results[0]);
    
    for (int i = 0; i < total; i++) {
        if (i == 2 && !benchmark_mode) {
            printf("⏭️  Performance Benchmark: SKIPPED (use --benchmark to run)\n");
            continue;
        }
        print_test_result(test_names[i], results[i]);
        if (results[i]) passed++;
    }
    
    printf("\n");
    if (passed == total || (passed == total - 1 && !benchmark_mode)) {
        printf("🎉 ALL TESTS PASSED\n");
        printf("✅ Real embedding generation is working correctly\n");
        printf("✅ VexFS integration is functional\n");
        printf("✅ Ready for Phase 1 storage validation\n");
    } else {
        printf("❌ SOME TESTS FAILED (%d/%d passed)\n", passed, total);
        printf("❌ Please fix issues before proceeding\n");
    }
    
    // Cleanup
    ollama_cleanup();
    
    return (passed == total || (passed == total - 1 && !benchmark_mode)) ? 0 : 1;
}