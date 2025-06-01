/*
 * VexFS v2.0 Ollama Basic Connectivity Test
 * 
 * This test validates basic Ollama connectivity and functionality
 * for the Phase 1 integration.
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include "ollama_client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

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

static bool test_ollama_init(void) {
    print_test_header("Ollama Initialization Test");
    
    ollama_error_t error = ollama_init(NULL);
    if (error != OLLAMA_SUCCESS) {
        printf("Failed to initialize Ollama client: %s\n", ollama_error_string(error));
        return false;
    }
    
    printf("Ollama client initialized successfully\n");
    return true;
}

static bool test_ollama_connectivity(void) {
    print_test_header("Ollama Connectivity Test");
    
    if (!ollama_is_available()) {
        printf("Ollama server is not available at default host\n");
        printf("Please ensure Ollama is running: ollama serve\n");
        return false;
    }
    
    printf("Ollama server is available and responding\n");
    return true;
}

static bool test_connectivity_function(void) {
    print_test_header("Connectivity Function Test");
    
    ollama_error_t error = ollama_run_connectivity_test();
    if (error != OLLAMA_SUCCESS) {
        printf("Connectivity test failed: %s\n", ollama_error_string(error));
        return false;
    }
    
    printf("Connectivity test passed\n");
    return true;
}

static bool test_model_dimensions(void) {
    print_test_header("Model Dimensions Test");
    
    struct {
        const char *model;
        uint32_t expected_dims;
    } test_models[] = {
        {"nomic-embed-text", 768},
        {"all-minilm", 384},
        {"mxbai-embed-large", 1024},
        {"snowflake-arctic-embed", 1024},
        {"unknown-model", 0}
    };
    
    bool all_passed = true;
    
    for (size_t i = 0; i < sizeof(test_models) / sizeof(test_models[0]); i++) {
        uint32_t dims = ollama_get_model_dimensions(test_models[i].model);
        printf("Model: %-20s Expected: %4u Got: %4u ", 
               test_models[i].model, test_models[i].expected_dims, dims);
        
        if (dims == test_models[i].expected_dims) {
            printf("✅\n");
        } else {
            printf("❌\n");
            all_passed = false;
        }
    }
    
    return all_passed;
}

static bool test_dimension_validation(void) {
    print_test_header("Dimension Validation Test");
    
    struct {
        uint32_t dims;
        bool expected_valid;
    } test_cases[] = {
        {0, false},
        {1, true},
        {384, true},
        {768, true},
        {1024, true},
        {65536, true},
        {65537, false}
    };
    
    bool all_passed = true;
    
    for (size_t i = 0; i < sizeof(test_cases) / sizeof(test_cases[0]); i++) {
        bool is_valid = ollama_validate_dimensions(test_cases[i].dims);
        printf("Dimensions: %6u Expected: %s Got: %s ", 
               test_cases[i].dims, 
               test_cases[i].expected_valid ? "valid  " : "invalid",
               is_valid ? "valid  " : "invalid");
        
        if (is_valid == test_cases[i].expected_valid) {
            printf("✅\n");
        } else {
            printf("❌\n");
            all_passed = false;
        }
    }
    
    return all_passed;
}

static bool test_memory_calculation(void) {
    print_test_header("Memory Calculation Test");
    
    struct {
        uint32_t text_count;
        uint32_t dimensions;
        size_t expected_min_size;
    } test_cases[] = {
        {1, 384, 384 * sizeof(float) + sizeof(uint64_t)},
        {10, 768, 10 * 768 * sizeof(float) + 10 * sizeof(uint64_t)},
        {100, 1024, 100 * 1024 * sizeof(float) + 100 * sizeof(uint64_t)}
    };
    
    bool all_passed = true;
    
    for (size_t i = 0; i < sizeof(test_cases) / sizeof(test_cases[0]); i++) {
        size_t calculated = ollama_calculate_batch_memory(test_cases[i].text_count, 
                                                         test_cases[i].dimensions);
        printf("Count: %3u Dims: %4u Expected: >= %8zu Got: %8zu ", 
               test_cases[i].text_count, test_cases[i].dimensions,
               test_cases[i].expected_min_size, calculated);
        
        if (calculated >= test_cases[i].expected_min_size) {
            printf("✅\n");
        } else {
            printf("❌\n");
            all_passed = false;
        }
    }
    
    return all_passed;
}

static bool test_error_strings(void) {
    print_test_header("Error String Test");
    
    ollama_error_t errors[] = {
        OLLAMA_SUCCESS,
        OLLAMA_ERROR_NETWORK,
        OLLAMA_ERROR_JSON_PARSE,
        OLLAMA_ERROR_MODEL_NOT_FOUND,
        OLLAMA_ERROR_INVALID_DIMENSIONS,
        OLLAMA_ERROR_MEMORY_ALLOCATION,
        OLLAMA_ERROR_TIMEOUT,
        OLLAMA_ERROR_INVALID_RESPONSE,
        OLLAMA_ERROR_CURL_INIT,
        OLLAMA_ERROR_HTTP_ERROR,
        OLLAMA_ERROR_BUFFER_OVERFLOW
    };
    
    bool all_passed = true;
    
    for (size_t i = 0; i < sizeof(errors) / sizeof(errors[0]); i++) {
        const char *error_str = ollama_error_string(errors[i]);
        printf("Error %2d: %s ", errors[i], error_str);
        
        if (error_str && strlen(error_str) > 0) {
            printf("✅\n");
        } else {
            printf("❌\n");
            all_passed = false;
        }
    }
    
    return all_passed;
}

static bool test_debug_functionality(void) {
    print_test_header("Debug Functionality Test");
    
    printf("Testing debug enable/disable...\n");
    ollama_set_debug(true);
    printf("Debug enabled (should see debug messages above)\n");
    
    ollama_set_debug(false);
    printf("Debug disabled (should not see debug messages above)\n");
    
    return true;
}

int main(int argc, char *argv[]) {
    printf("🦙 VexFS v2.0 Ollama Basic Connectivity Test\n");
    printf("═══════════════════════════════════════════════════════════════\n");
    printf("This test validates basic Ollama functionality for Phase 1 integration.\n");
    
    bool verbose = false;
    if (argc > 1 && strcmp(argv[1], "--verbose") == 0) {
        verbose = true;
        ollama_set_debug(true);
    }
    
    // Test results
    bool results[] = {
        test_ollama_init(),
        test_ollama_connectivity(),
        test_connectivity_function(),
        test_model_dimensions(),
        test_dimension_validation(),
        test_memory_calculation(),
        test_error_strings(),
        test_debug_functionality()
    };
    
    const char *test_names[] = {
        "Ollama Initialization",
        "Ollama Connectivity",
        "Connectivity Function",
        "Model Dimensions",
        "Dimension Validation",
        "Memory Calculation",
        "Error Strings",
        "Debug Functionality"
    };
    
    // Summary
    printf("\n📊 TEST SUMMARY\n");
    printf("═══════════════════════════════════════════════════════════════\n");
    
    int passed = 0;
    int total = sizeof(results) / sizeof(results[0]);
    
    for (int i = 0; i < total; i++) {
        print_test_result(test_names[i], results[i]);
        if (results[i]) passed++;
    }
    
    printf("\n");
    if (passed == total) {
        printf("🎉 ALL TESTS PASSED (%d/%d)\n", passed, total);
        printf("✅ Ollama basic functionality is working correctly\n");
        printf("✅ Ready for Phase 1 real embedding integration\n");
    } else {
        printf("❌ SOME TESTS FAILED (%d/%d passed)\n", passed, total);
        printf("❌ Please fix issues before proceeding with Phase 1\n");
    }
    
    // Cleanup
    ollama_cleanup();
    
    return (passed == total) ? 0 : 1;
}