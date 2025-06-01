/*
 * VexFS v2.0 Ollama Client Implementation
 *
 * This file implements the C API for interfacing with Ollama to generate
 * real embeddings for VexFS v2.0 vector database validation.
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
#include <errno.h>
#include <time.h>
#include <curl/curl.h>
#include <json-c/json.h>
#include <sys/ioctl.h>

/*
 * Global state
 */
static char g_ollama_host[256] = OLLAMA_DEFAULT_HOST;
static bool g_debug_enabled = false;
static bool g_curl_initialized = false;

/*
 * HTTP response structure for curl
 */
typedef struct {
    char *data;
    size_t size;
    size_t capacity;
} http_response_t;

/*
 * Debug logging macro
 */
#define DEBUG_LOG(fmt, ...) \
    do { \
        if (g_debug_enabled) { \
            fprintf(stderr, "[OLLAMA_DEBUG] " fmt "\n", ##__VA_ARGS__); \
        } \
    } while(0)

/*
 * Utility Functions
 */

static double get_time_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1000000.0;
}

static size_t ollama_write_callback(void *contents, size_t size, size_t nmemb, http_response_t *response) {
    size_t total_size = size * nmemb;
    
    if (response->size + total_size >= response->capacity) {
        size_t new_capacity = response->capacity * 2;
        if (new_capacity < response->size + total_size + 1) {
            new_capacity = response->size + total_size + 1;
        }
        
        char *new_data = realloc(response->data, new_capacity);
        if (!new_data) {
            DEBUG_LOG("Failed to reallocate response buffer");
            return 0;
        }
        
        response->data = new_data;
        response->capacity = new_capacity;
    }
    
    memcpy(response->data + response->size, contents, total_size);
    response->size += total_size;
    response->data[response->size] = '\0';
    
    return total_size;
}

static ollama_error_t http_post_json(const char *url, const char *json_data, http_response_t *response) {
    CURL *curl;
    CURLcode res;
    struct curl_slist *headers = NULL;
    
    curl = curl_easy_init();
    if (!curl) {
        return OLLAMA_ERROR_CURL_INIT;
    }
    
    // Initialize response buffer
    response->data = malloc(4096);
    response->size = 0;
    response->capacity = 4096;
    if (!response->data) {
        curl_easy_cleanup(curl);
        return OLLAMA_ERROR_MEMORY_ALLOCATION;
    }
    response->data[0] = '\0';
    
    // Set headers
    headers = curl_slist_append(headers, "Content-Type: application/json");
    
    // Configure curl
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, json_data);
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, ollama_write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, response);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, OLLAMA_TIMEOUT_SECONDS);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    
    DEBUG_LOG("Sending POST request to: %s", url);
    DEBUG_LOG("JSON payload: %s", json_data);
    
    // Perform request
    res = curl_easy_perform(curl);
    
    // Check for errors
    ollama_error_t error = OLLAMA_SUCCESS;
    if (res != CURLE_OK) {
        DEBUG_LOG("curl_easy_perform() failed: %s", curl_easy_strerror(res));
        if (res == CURLE_OPERATION_TIMEDOUT) {
            error = OLLAMA_ERROR_TIMEOUT;
        } else {
            error = OLLAMA_ERROR_NETWORK;
        }
    } else {
        long response_code;
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &response_code);
        if (response_code != 200) {
            DEBUG_LOG("HTTP error: %ld", response_code);
            error = OLLAMA_ERROR_HTTP_ERROR;
        }
    }
    
    curl_slist_free_all(headers);
    curl_easy_cleanup(curl);
    
    if (error != OLLAMA_SUCCESS) {
        free(response->data);
        response->data = NULL;
    }
    
    return error;
}

static ollama_error_t http_get(const char *url, http_response_t *response) {
    CURL *curl;
    CURLcode res;
    
    curl = curl_easy_init();
    if (!curl) {
        return OLLAMA_ERROR_CURL_INIT;
    }
    
    // Initialize response buffer
    response->data = malloc(4096);
    response->size = 0;
    response->capacity = 4096;
    if (!response->data) {
        curl_easy_cleanup(curl);
        return OLLAMA_ERROR_MEMORY_ALLOCATION;
    }
    response->data[0] = '\0';
    
    // Configure curl
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, ollama_write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, response);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, OLLAMA_TIMEOUT_SECONDS);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    
    DEBUG_LOG("Sending GET request to: %s", url);
    
    // Perform request
    res = curl_easy_perform(curl);
    
    // Check for errors
    ollama_error_t error = OLLAMA_SUCCESS;
    if (res != CURLE_OK) {
        DEBUG_LOG("curl_easy_perform() failed: %s", curl_easy_strerror(res));
        if (res == CURLE_OPERATION_TIMEDOUT) {
            error = OLLAMA_ERROR_TIMEOUT;
        } else {
            error = OLLAMA_ERROR_NETWORK;
        }
    } else {
        long response_code;
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &response_code);
        if (response_code != 200) {
            DEBUG_LOG("HTTP error: %ld", response_code);
            error = OLLAMA_ERROR_HTTP_ERROR;
        }
    }
    
    curl_easy_cleanup(curl);
    
    if (error != OLLAMA_SUCCESS) {
        free(response->data);
        response->data = NULL;
    }
    
    return error;
}

/*
 * Core API Implementation
 */

ollama_error_t ollama_init(const char *host) {
    if (host) {
        strncpy(g_ollama_host, host, sizeof(g_ollama_host) - 1);
        g_ollama_host[sizeof(g_ollama_host) - 1] = '\0';
    }
    
    if (!g_curl_initialized) {
        CURLcode res = curl_global_init(CURL_GLOBAL_DEFAULT);
        if (res != CURLE_OK) {
            DEBUG_LOG("curl_global_init() failed: %s", curl_easy_strerror(res));
            return OLLAMA_ERROR_CURL_INIT;
        }
        g_curl_initialized = true;
    }
    
    DEBUG_LOG("Ollama client initialized with host: %s", g_ollama_host);
    return OLLAMA_SUCCESS;
}

void ollama_cleanup(void) {
    if (g_curl_initialized) {
        curl_global_cleanup();
        g_curl_initialized = false;
    }
    DEBUG_LOG("Ollama client cleaned up");
}

bool ollama_is_available(void) {
    char url[512];
    snprintf(url, sizeof(url), "%s/api/tags", g_ollama_host);
    
    http_response_t response;
    ollama_error_t error = http_get(url, &response);
    
    if (error == OLLAMA_SUCCESS) {
        free(response.data);
        return true;
    }
    
    return false;
}

ollama_error_t ollama_generate_embedding(ollama_embedding_request_t *request) {
    char url[512];
    snprintf(url, sizeof(url), "%s/api/embeddings", g_ollama_host);
    
    // Create JSON request
    json_object *json_request = json_object_new_object();
    json_object *model_obj = json_object_new_string(request->model);
    json_object *prompt_obj = json_object_new_string(request->text);
    
    json_object_object_add(json_request, "model", model_obj);
    json_object_object_add(json_request, "prompt", prompt_obj);
    
    const char *json_string = json_object_to_json_string(json_request);
    
    double start_time = get_time_ms();
    
    http_response_t response;
    ollama_error_t error = http_post_json(url, json_string, &response);
    
    json_object_put(json_request);
    
    if (error != OLLAMA_SUCCESS) {
        return error;
    }
    
    // Parse response
    json_object *root = json_tokener_parse(response.data);
    free(response.data);
    
    if (!root) {
        return OLLAMA_ERROR_JSON_PARSE;
    }
    
    json_object *embedding_array;
    if (!json_object_object_get_ex(root, "embedding", &embedding_array)) {
        json_object_put(root);
        return OLLAMA_ERROR_INVALID_RESPONSE;
    }
    
    int array_len = json_object_array_length(embedding_array);
    if (array_len != request->expected_dimensions) {
        DEBUG_LOG("Dimension mismatch: expected %u, got %d", request->expected_dimensions, array_len);
        json_object_put(root);
        return OLLAMA_ERROR_INVALID_DIMENSIONS;
    }
    
    // Extract embedding values
    for (int i = 0; i < array_len; i++) {
        json_object *value_obj = json_object_array_get_idx(embedding_array, i);
        request->embedding_output[i] = (float)json_object_get_double(value_obj);
    }
    
    *request->actual_dimensions = array_len;
    
    double end_time = get_time_ms();
    if (request->generation_time_ms) {
        *request->generation_time_ms = end_time - start_time;
    }
    
    json_object_put(root);
    
    DEBUG_LOG("Generated embedding for text (length=%zu) in %.2f ms", 
              request->text_length, end_time - start_time);
    
    return OLLAMA_SUCCESS;
}

/*
 * Utility Functions Implementation
 */

const char *ollama_error_string(ollama_error_t error) {
    switch (error) {
        case OLLAMA_SUCCESS: return "Success";
        case OLLAMA_ERROR_NETWORK: return "Network error";
        case OLLAMA_ERROR_JSON_PARSE: return "JSON parsing error";
        case OLLAMA_ERROR_MODEL_NOT_FOUND: return "Model not found";
        case OLLAMA_ERROR_INVALID_DIMENSIONS: return "Invalid dimensions";
        case OLLAMA_ERROR_MEMORY_ALLOCATION: return "Memory allocation error";
        case OLLAMA_ERROR_TIMEOUT: return "Request timeout";
        case OLLAMA_ERROR_INVALID_RESPONSE: return "Invalid response";
        case OLLAMA_ERROR_CURL_INIT: return "CURL initialization error";
        case OLLAMA_ERROR_HTTP_ERROR: return "HTTP error";
        case OLLAMA_ERROR_BUFFER_OVERFLOW: return "Buffer overflow";
        default: return "Unknown error";
    }
}

uint32_t ollama_get_model_dimensions(const char *model_name) {
    if (strstr(model_name, "nomic-embed-text")) return 768;
    if (strstr(model_name, "all-minilm")) return 384;
    if (strstr(model_name, "mxbai-embed-large")) return 1024;
    if (strstr(model_name, "snowflake-arctic")) return 1024;
    return 0; // Unknown model
}

bool ollama_validate_dimensions(uint32_t dimensions) {
    return VEXFS_VALID_DIMENSIONS(dimensions);
}

size_t ollama_calculate_batch_memory(uint32_t text_count, uint32_t dimensions) {
    return text_count * dimensions * sizeof(float) + text_count * sizeof(uint64_t);
}

void ollama_set_debug(bool enable) {
    g_debug_enabled = enable;
    DEBUG_LOG("Debug logging %s", enable ? "enabled" : "disabled");
}

ollama_error_t ollama_run_connectivity_test(void) {
    DEBUG_LOG("Running Ollama connectivity test...");
    
    if (!ollama_is_available()) {
        DEBUG_LOG("Ollama server is not available");
        return OLLAMA_ERROR_NETWORK;
    }
    
    DEBUG_LOG("Ollama connectivity test passed");
    return OLLAMA_SUCCESS;
}

/*
 * VexFS Integration Implementation (Simplified for Phase 1)
 */

ollama_error_t vexfs_ollama_init(vexfs_ollama_integration_t *integration,
                                 const char *vexfs_path,
                                 const char *model_name) {
    memset(integration, 0, sizeof(*integration));
    
    // Open VexFS file
    integration->vexfs_fd = open(vexfs_path, O_RDWR);
    if (integration->vexfs_fd < 0) {
        DEBUG_LOG("Failed to open VexFS file: %s", strerror(errno));
        return OLLAMA_ERROR_NETWORK; // Reuse for file errors
    }
    
    // Set model name
    strncpy(integration->model, model_name, OLLAMA_MAX_MODEL_NAME - 1);
    integration->model[OLLAMA_MAX_MODEL_NAME - 1] = '\0';
    
    // Get model dimensions
    uint32_t dimensions = ollama_get_model_dimensions(model_name);
    if (dimensions == 0) {
        close(integration->vexfs_fd);
        return OLLAMA_ERROR_INVALID_DIMENSIONS;
    }
    
    // Initialize VexFS metadata
    integration->meta.dimensions = dimensions;
    integration->meta.element_type = VEXFS_VECTOR_FLOAT32;
    integration->meta.vector_count = 0;
    integration->meta.storage_format = VEXFS_STORAGE_DENSE;
    integration->meta.data_offset = 0;
    integration->meta.index_offset = 0;
    integration->meta.compression_type = VEXFS_COMPRESS_NONE;
    integration->meta.alignment_bytes = 32;
    
    // Set VexFS metadata
    if (ioctl(integration->vexfs_fd, VEXFS_IOC_SET_VECTOR_META, &integration->meta) != 0) {
        DEBUG_LOG("Failed to set VexFS metadata: %s", strerror(errno));
        close(integration->vexfs_fd);
        return OLLAMA_ERROR_NETWORK; // Reuse for ioctl errors
    }
    
    integration->batch_size = 100; // Default batch size
    integration->next_vector_id = 1;
    
    DEBUG_LOG("VexFS-Ollama integration initialized: model=%s, dimensions=%u", 
              model_name, dimensions);
    
    return OLLAMA_SUCCESS;
}

ollama_error_t vexfs_ollama_insert_text(vexfs_ollama_integration_t *integration,
                                        const char *text,
                                        uint64_t vector_id) {
    // Generate embedding
    float *embedding = malloc(integration->meta.dimensions * sizeof(float));
    if (!embedding) {
        return OLLAMA_ERROR_MEMORY_ALLOCATION;
    }
    
    uint32_t actual_dimensions;
    double embedding_time;
    
    ollama_embedding_request_t request = {
        .text = (char*)text,
        .text_length = strlen(text),
        .embedding_output = embedding,
        .expected_dimensions = integration->meta.dimensions,
        .actual_dimensions = &actual_dimensions,
        .generation_time_ms = &embedding_time
    };
    strncpy(request.model, integration->model, OLLAMA_MAX_MODEL_NAME - 1);
    request.model[OLLAMA_MAX_MODEL_NAME - 1] = '\0';
    
    ollama_error_t error = ollama_generate_embedding(&request);
    if (error != OLLAMA_SUCCESS) {
        free(embedding);
        return error;
    }
    
    // Use auto-increment ID if not provided
    uint64_t id = vector_id ? vector_id : integration->next_vector_id++;
    
    // Insert into VexFS
    struct vexfs_batch_insert_request vexfs_request = {
        .vectors = embedding,
        .vector_count = 1,
        .dimensions = integration->meta.dimensions,
        .vector_ids = &id,
        .flags = VEXFS_INSERT_APPEND
    };
    
    double vexfs_start = get_time_ms();
    if (ioctl(integration->vexfs_fd, VEXFS_IOC_BATCH_INSERT, &vexfs_request) != 0) {
        DEBUG_LOG("Failed to insert vector into VexFS: %s", strerror(errno));
        free(embedding);
        return OLLAMA_ERROR_NETWORK; // Reuse for ioctl errors
    }
    double vexfs_end = get_time_ms();
    
    // Update statistics
    integration->total_embedding_time_ms += embedding_time;
    integration->total_vexfs_time_ms += (vexfs_end - vexfs_start);
    integration->total_vectors_inserted++;
    
    free(embedding);
    
    DEBUG_LOG("Inserted text as vector ID %lu (embedding: %.2f ms, vexfs: %.2f ms)", 
              id, embedding_time, vexfs_end - vexfs_start);
    
    return OLLAMA_SUCCESS;
}

void vexfs_ollama_cleanup(vexfs_ollama_integration_t *integration) {
    if (integration->vexfs_fd >= 0) {
        close(integration->vexfs_fd);
        integration->vexfs_fd = -1;
    }
    
    DEBUG_LOG("VexFS-Ollama integration cleaned up");
}