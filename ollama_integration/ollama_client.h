/*
 * VexFS v2.0 Ollama Client API Header
 * 
 * This header provides a C API for interfacing with Ollama to generate
 * real embeddings for VexFS v2.0 vector database validation.
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef _OLLAMA_CLIENT_H
#define _OLLAMA_CLIENT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Include VexFS v2.0 UAPI for compatibility */
#include "../kernel/vexfs_v2_build/vexfs_v2_uapi.h"

/*
 * Ollama Configuration Constants
 */
#define OLLAMA_DEFAULT_HOST     "http://localhost:11434"
#define OLLAMA_MAX_MODEL_NAME   256
#define OLLAMA_MAX_TEXT_LENGTH  8192
#define OLLAMA_MAX_DIMENSIONS   4096
#define OLLAMA_MAX_RETRIES      3
#define OLLAMA_TIMEOUT_SECONDS  30

/*
 * Supported Embedding Models
 */
#define OLLAMA_MODEL_NOMIC_EMBED_TEXT   "nomic-embed-text"      /* 768D */
#define OLLAMA_MODEL_ALL_MINILM         "all-minilm"            /* 384D */
#define OLLAMA_MODEL_MXBAI_EMBED_LARGE  "mxbai-embed-large"     /* 1024D */
#define OLLAMA_MODEL_SNOWFLAKE_ARCTIC   "snowflake-arctic-embed" /* 1024D */

/*
 * Error Codes
 */
typedef enum {
    OLLAMA_SUCCESS = 0,
    OLLAMA_ERROR_NETWORK = -1,
    OLLAMA_ERROR_JSON_PARSE = -2,
    OLLAMA_ERROR_MODEL_NOT_FOUND = -3,
    OLLAMA_ERROR_INVALID_DIMENSIONS = -4,
    OLLAMA_ERROR_MEMORY_ALLOCATION = -5,
    OLLAMA_ERROR_TIMEOUT = -6,
    OLLAMA_ERROR_INVALID_RESPONSE = -7,
    OLLAMA_ERROR_CURL_INIT = -8,
    OLLAMA_ERROR_HTTP_ERROR = -9,
    OLLAMA_ERROR_BUFFER_OVERFLOW = -10
} ollama_error_t;

/*
 * Model Information Structure
 */
typedef struct {
    char name[OLLAMA_MAX_MODEL_NAME];
    uint32_t dimensions;
    bool is_available;
    double avg_generation_time_ms;
    uint64_t total_embeddings_generated;
} ollama_model_info_t;

/*
 * Embedding Request Structure
 */
typedef struct {
    char model[OLLAMA_MAX_MODEL_NAME];
    char *text;
    size_t text_length;
    float *embedding_output;
    uint32_t expected_dimensions;
    uint32_t *actual_dimensions;
    double *generation_time_ms;
} ollama_embedding_request_t;

/*
 * Batch Embedding Request Structure
 */
typedef struct {
    char model[OLLAMA_MAX_MODEL_NAME];
    char **texts;
    size_t text_count;
    float *embeddings_output;  /* Flattened array: text_count * dimensions */
    uint32_t expected_dimensions;
    uint32_t *actual_dimensions;
    double *total_generation_time_ms;
    uint32_t max_concurrent_requests;
} ollama_batch_embedding_request_t;

/*
 * VexFS Integration Structure
 * 
 * This structure bridges Ollama embeddings with VexFS v2.0 IOCTL interface
 */
typedef struct {
    int vexfs_fd;                           /* VexFS file descriptor */
    struct vexfs_vector_file_info meta;     /* VexFS metadata */
    char model[OLLAMA_MAX_MODEL_NAME];      /* Ollama model name */
    uint32_t batch_size;                    /* Vectors per batch */
    uint64_t next_vector_id;                /* Auto-incrementing ID */
    double total_embedding_time_ms;         /* Performance tracking */
    double total_vexfs_time_ms;             /* Performance tracking */
    uint64_t total_vectors_inserted;        /* Statistics */
} vexfs_ollama_integration_t;

/*
 * Performance Statistics Structure
 */
typedef struct {
    uint64_t total_embeddings_generated;
    uint64_t total_vectors_inserted;
    double avg_embedding_time_ms;
    double avg_vexfs_insert_time_ms;
    double total_throughput_vectors_per_sec;
    uint64_t memory_usage_bytes;
    uint32_t error_count;
    uint32_t retry_count;
} ollama_performance_stats_t;

/*
 * Core API Functions
 */

/**
 * Initialize Ollama client
 * @param host Ollama server host (NULL for default)
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t ollama_init(const char *host);

/**
 * Cleanup Ollama client resources
 */
void ollama_cleanup(void);

/**
 * Check if Ollama server is available
 * @return true if available, false otherwise
 */
bool ollama_is_available(void);

/**
 * List available embedding models
 * @param models Output array of model info
 * @param max_models Maximum number of models to return
 * @param actual_count Actual number of models found
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t ollama_list_models(ollama_model_info_t *models, 
                                  uint32_t max_models, 
                                  uint32_t *actual_count);

/**
 * Pull/download a model if not available
 * @param model_name Name of the model to pull
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t ollama_pull_model(const char *model_name);

/**
 * Generate embedding for a single text
 * @param request Embedding request structure
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t ollama_generate_embedding(ollama_embedding_request_t *request);

/**
 * Generate embeddings for multiple texts (batch processing)
 * @param request Batch embedding request structure
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t ollama_generate_batch_embeddings(ollama_batch_embedding_request_t *request);

/*
 * VexFS Integration Functions
 */

/**
 * Initialize VexFS-Ollama integration
 * @param integration Integration structure to initialize
 * @param vexfs_path Path to VexFS file
 * @param model_name Ollama model to use
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t vexfs_ollama_init(vexfs_ollama_integration_t *integration,
                                 const char *vexfs_path,
                                 const char *model_name);

/**
 * Insert text as vector into VexFS using Ollama embeddings
 * @param integration Integration context
 * @param text Text to embed and insert
 * @param vector_id Optional custom vector ID (0 for auto-increment)
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t vexfs_ollama_insert_text(vexfs_ollama_integration_t *integration,
                                        const char *text,
                                        uint64_t vector_id);

/**
 * Insert multiple texts as vectors into VexFS (batch operation)
 * @param integration Integration context
 * @param texts Array of texts to embed and insert
 * @param text_count Number of texts
 * @param vector_ids Optional custom vector IDs (NULL for auto-increment)
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t vexfs_ollama_batch_insert_texts(vexfs_ollama_integration_t *integration,
                                               const char **texts,
                                               uint32_t text_count,
                                               const uint64_t *vector_ids);

/**
 * Search VexFS for similar vectors using text query
 * @param integration Integration context
 * @param query_text Text to search for
 * @param k Number of nearest neighbors
 * @param result_ids Output array for vector IDs
 * @param result_scores Output array for similarity scores
 * @param actual_results Actual number of results found
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t vexfs_ollama_search_text(vexfs_ollama_integration_t *integration,
                                        const char *query_text,
                                        uint32_t k,
                                        uint64_t *result_ids,
                                        float *result_scores,
                                        uint32_t *actual_results);

/**
 * Get performance statistics
 * @param integration Integration context
 * @param stats Output statistics structure
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t vexfs_ollama_get_stats(vexfs_ollama_integration_t *integration,
                                      ollama_performance_stats_t *stats);

/**
 * Cleanup VexFS-Ollama integration
 * @param integration Integration context to cleanup
 */
void vexfs_ollama_cleanup(vexfs_ollama_integration_t *integration);

/*
 * Utility Functions
 */

/**
 * Get error string for error code
 * @param error Error code
 * @return Human-readable error string
 */
const char *ollama_error_string(ollama_error_t error);

/**
 * Get model dimensions by name
 * @param model_name Model name
 * @return Number of dimensions, 0 if unknown
 */
uint32_t ollama_get_model_dimensions(const char *model_name);

/**
 * Validate embedding dimensions against VexFS limits
 * @param dimensions Number of dimensions to validate
 * @return true if valid, false otherwise
 */
bool ollama_validate_dimensions(uint32_t dimensions);

/**
 * Calculate memory requirements for batch operation
 * @param text_count Number of texts
 * @param dimensions Vector dimensions
 * @return Required memory in bytes
 */
size_t ollama_calculate_batch_memory(uint32_t text_count, uint32_t dimensions);

/*
 * Debug and Testing Functions
 */

/**
 * Enable debug logging
 * @param enable true to enable, false to disable
 */
void ollama_set_debug(bool enable);

/**
 * Test Ollama connectivity and basic functionality
 * @return OLLAMA_SUCCESS if all tests pass, error code otherwise
 */
ollama_error_t ollama_run_connectivity_test(void);

/**
 * Benchmark embedding generation performance
 * @param model_name Model to benchmark
 * @param text_samples Array of test texts
 * @param sample_count Number of test texts
 * @param avg_time_ms Output average generation time
 * @return OLLAMA_SUCCESS on success, error code on failure
 */
ollama_error_t ollama_benchmark_model(const char *model_name,
                                      const char **text_samples,
                                      uint32_t sample_count,
                                      double *avg_time_ms);

#endif /* _OLLAMA_CLIENT_H */