/*
 * VexFS v2.0 Phase 3 Standalone Test Program
 * 
 * This is a standalone test program with embedded definitions for testing
 * Phase 3 advanced search functionality without header dependencies.
 */

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

/* Embedded Phase 3 definitions */
#define VEXFS_IOC_MAGIC 'V'

/* Phase 3 IOCTL commands */
#define VEXFS_IOC_SET_MODEL_META        _IOW(VEXFS_IOC_MAGIC, 20, struct vexfs_model_metadata)
#define VEXFS_IOC_GET_MODEL_META        _IOR(VEXFS_IOC_MAGIC, 21, struct vexfs_model_metadata)
#define VEXFS_IOC_FILTERED_SEARCH       _IOW(VEXFS_IOC_MAGIC, 22, struct vexfs_filtered_search_request)
#define VEXFS_IOC_MULTI_VECTOR_SEARCH   _IOW(VEXFS_IOC_MAGIC, 23, struct vexfs_multi_vector_search_request)
#define VEXFS_IOC_HYBRID_SEARCH         _IOW(VEXFS_IOC_MAGIC, 24, struct vexfs_hybrid_search_request)

/* Embedding model types */
enum vexfs_embedding_model_type {
    VEXFS_EMBED_OLLAMA_NOMIC = 1,
    VEXFS_EMBED_OLLAMA_MINILM = 2,
    VEXFS_EMBED_OPENAI_SMALL = 3,
    VEXFS_EMBED_OPENAI_LARGE = 4,
    VEXFS_EMBED_SENTENCE_BERT = 5,
    VEXFS_EMBED_CUSTOM = 99
};

/* Distance metrics */
enum vexfs_distance_metric {
    VEXFS_DISTANCE_EUCLIDEAN = 0,
    VEXFS_DISTANCE_COSINE = 1,
    VEXFS_DISTANCE_DOT_PRODUCT = 2,
    VEXFS_DISTANCE_MANHATTAN = 3
};

/* Filter field types */
enum vexfs_filter_field_type {
    VEXFS_FILTER_FIELD_ID = 0,
    VEXFS_FILTER_FIELD_TIMESTAMP = 1,
    VEXFS_FILTER_FIELD_CATEGORY = 2,
    VEXFS_FILTER_FIELD_SCORE = 3,
    VEXFS_FILTER_FIELD_RANGE = 4
};

/* Filter operators */
enum vexfs_filter_operator {
    VEXFS_FILTER_EQ = 0,
    VEXFS_FILTER_NE = 1,
    VEXFS_FILTER_LT = 2,
    VEXFS_FILTER_LE = 3,
    VEXFS_FILTER_GT = 4,
    VEXFS_FILTER_GE = 5,
    VEXFS_FILTER_RANGE = 6
};

/* Constants */
#define VEXFS_MAX_MODEL_NAME 64
#define VEXFS_MAX_MODEL_DESC 256
#define VEXFS_MAX_FILTER_STRING 64
#define VEXFS_MAX_FIELD_NAME 32

/* Model metadata structure */
struct vexfs_model_metadata {
    uint32_t model_type;
    uint32_t dimensions;
    uint32_t max_sequence_length;
    uint32_t model_version;
    char model_name[VEXFS_MAX_MODEL_NAME];
    char model_description[VEXFS_MAX_MODEL_DESC];
    uint64_t creation_timestamp;
    uint32_t reserved[4];
};

/* Search result structure */
struct vexfs_search_result {
    uint64_t vector_id;
    uint64_t distance;
    uint64_t score;
    uint32_t metadata_size;
    uint32_t reserved;
};

/* Search filter structure */
struct vexfs_search_filter {
    uint32_t field_type;
    uint32_t operator;
    char field_name[VEXFS_MAX_FIELD_NAME];
    union {
        uint64_t numeric;
        char string[VEXFS_MAX_FILTER_STRING];
        struct {
            uint64_t min;
            uint64_t max;
        } range;
    } value;
    uint32_t reserved[2];
};

/* Filtered search request */
struct vexfs_filtered_search_request {
    const float *query_vector;
    uint32_t dimensions;
    uint32_t k;
    uint32_t distance_metric;
    const struct vexfs_search_filter *filters;
    uint32_t filter_count;
    struct vexfs_search_result *results;
    uint32_t *result_count;
    uint32_t reserved[4];
};

/* Multi-vector search request */
struct vexfs_multi_vector_search_request {
    const float *query_vectors;
    uint32_t query_count;
    uint32_t dimensions;
    uint32_t k_per_query;
    uint32_t distance_metric;
    struct vexfs_search_result *results;
    uint32_t *result_counts;
    uint32_t reserved[4];
};

/* Hybrid search request */
struct vexfs_hybrid_search_request {
    const float *query_vector;
    uint32_t dimensions;
    uint32_t k;
    uint32_t primary_metric;
    uint32_t secondary_metric;
    float primary_weight;
    float secondary_weight;
    struct vexfs_search_result *results;
    uint32_t *result_count;
    uint32_t reserved[4];
};

void print_test_header(const char *test_name) {
    printf("\n🧪 %s\n", test_name);
    printf("================================================\n");
}

void print_model_info(const struct vexfs_model_metadata *model) {
    printf("📊 Model Information:\n");
    printf("   Type: %u (%s)\n", model->model_type, 
           model->model_type == VEXFS_EMBED_OLLAMA_NOMIC ? "Ollama Nomic" :
           model->model_type == VEXFS_EMBED_OLLAMA_MINILM ? "Ollama MiniLM" :
           model->model_type == VEXFS_EMBED_OPENAI_SMALL ? "OpenAI Small" :
           model->model_type == VEXFS_EMBED_OPENAI_LARGE ? "OpenAI Large" :
           model->model_type == VEXFS_EMBED_SENTENCE_BERT ? "Sentence-BERT" :
           model->model_type == VEXFS_EMBED_CUSTOM ? "Custom" : "Unknown");
    printf("   Dimensions: %u\n", model->dimensions);
    printf("   Max Sequence Length: %u\n", model->max_sequence_length);
    printf("   Model Version: %u\n", model->model_version);
    printf("   Name: %s\n", model->model_name);
    printf("   Description: %s\n", model->model_description);
    printf("   Created: %llu\n", model->creation_timestamp);
}

void print_search_results(const struct vexfs_search_result *results, uint32_t count) {
    uint32_t i;
    printf("📊 Search Results (%u found):\n", count);
    for (i = 0; i < count && i < 5; i++) { /* Show first 5 results */
        printf("   [%u] ID: %llu, Distance: %llu, Score: %llu\n",
               i, results[i].vector_id, results[i].distance, results[i].score);
    }
    if (count > 5) {
        printf("   ... and %u more results\n", count - 5);
    }
}

int test_model_metadata(int fd) {
    struct vexfs_model_metadata model;
    int ret;
    
    print_test_header("Multi-Model Metadata Test");
    
    /* Test 1: Set Ollama Nomic model */
    printf("🔧 Test 1: Setting Ollama Nomic model metadata...\n");
    memset(&model, 0, sizeof(model));
    model.model_type = VEXFS_EMBED_OLLAMA_NOMIC;
    model.dimensions = 768;
    model.max_sequence_length = 8192;
    model.model_version = 1;
    strcpy(model.model_name, "nomic-embed-text");
    strcpy(model.model_description, "Ollama Nomic Embed Text model");
    
    ret = ioctl(fd, VEXFS_IOC_SET_MODEL_META, &model);
    if (ret == 0) {
        printf("✅ Ollama Nomic model metadata set successfully\n");
    } else {
        printf("❌ Failed to set Ollama Nomic model metadata: %d\n", ret);
        return ret;
    }
    
    /* Test 2: Get model metadata */
    printf("\n🔧 Test 2: Getting current model metadata...\n");
    memset(&model, 0, sizeof(model));
    ret = ioctl(fd, VEXFS_IOC_GET_MODEL_META, &model);
    if (ret == 0) {
        printf("✅ Model metadata retrieved successfully\n");
        print_model_info(&model);
    } else {
        printf("❌ Failed to get model metadata: %d\n", ret);
        return ret;
    }
    
    /* Test 3: Set OpenAI model */
    printf("\n🔧 Test 3: Setting OpenAI Small model metadata...\n");
    memset(&model, 0, sizeof(model));
    model.model_type = VEXFS_EMBED_OPENAI_SMALL;
    model.dimensions = 1536;
    model.max_sequence_length = 8191;
    model.model_version = 3;
    strcpy(model.model_name, "text-embedding-3-small");
    strcpy(model.model_description, "OpenAI Text Embedding 3 Small");
    
    ret = ioctl(fd, VEXFS_IOC_SET_MODEL_META, &model);
    if (ret == 0) {
        printf("✅ OpenAI Small model metadata set successfully\n");
        
        /* Verify the change */
        memset(&model, 0, sizeof(model));
        ret = ioctl(fd, VEXFS_IOC_GET_MODEL_META, &model);
        if (ret == 0) {
            print_model_info(&model);
        }
    } else {
        printf("❌ Failed to set OpenAI Small model metadata: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_filtered_search(int fd) {
    struct vexfs_filtered_search_request req;
    struct vexfs_search_result results[50];
    uint32_t result_count;
    struct vexfs_search_filter filter;
    float query_vector[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    uint32_t query_vector_bits[4];
    vexfs_float_array_to_bits(query_vector, query_vector_bits, 4);
    int ret;
    
    print_test_header("Filtered Search Test");
    
    printf("🔧 Testing filtered search with range filter...\n");
    
    memset(&req, 0, sizeof(req));
    memset(&filter, 0, sizeof(filter));
    
    /* Set up filter */
    filter.field_type = VEXFS_FILTER_FIELD_RANGE;
    filter.operator = VEXFS_FILTER_RANGE;
    filter.value.range.min = 10;
    filter.value.range.max = 100;
    strcpy(filter.field_name, "id_range");
    
    /* Set up request */
    req.query_vector = query_vector;
    req.dimensions = 4;
    req.k = 50;
    req.distance_metric = VEXFS_DISTANCE_EUCLIDEAN;
    req.filters = &filter;
    req.filter_count = 1;
    req.results = results;
    req.result_count = &result_count;
    
    ret = ioctl(fd, VEXFS_IOC_FILTERED_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Filtered search completed successfully\n");
        print_search_results(results, result_count);
    } else {
        printf("❌ Filtered search failed: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_multi_vector_search(int fd) {
    struct vexfs_multi_vector_search_request req;
    struct vexfs_search_result results[150]; /* 3 queries * 50 results each */
    uint32_t result_counts[3];
    float query_vectors[12] = {
        /* Query 1 */ 1.0f, 2.0f, 3.0f, 4.0f,
        /* Query 2 */ 5.0f, 6.0f, 7.0f, 8.0f,
        /* Query 3 */ 9.0f, 10.0f, 11.0f, 12.0f
    };
    int ret;
    
    print_test_header("Multi-Vector Search Test");
    
    printf("🔧 Testing multi-vector search with 3 query vectors...\n");
    
    memset(&req, 0, sizeof(req));
    
    req.query_vectors = query_vectors;
    req.query_count = 3;
    req.dimensions = 4;
    req.k_per_query = 50;
    req.distance_metric = VEXFS_DISTANCE_COSINE;
    req.results = results;
    req.result_counts = result_counts;
    
    ret = ioctl(fd, VEXFS_IOC_MULTI_VECTOR_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Multi-vector search completed successfully\n");
        
        /* Print results for each query */
        uint32_t i;
        for (i = 0; i < req.query_count; i++) {
            printf("\n📊 Results for Query %u:\n", i + 1);
            print_search_results(&results[i * req.k_per_query], result_counts[i]);
        }
    } else {
        printf("❌ Multi-vector search failed: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_hybrid_search(int fd) {
    struct vexfs_hybrid_search_request req;
    struct vexfs_search_result results[50];
    uint32_t result_count;
    float query_vector[4] = {1.5f, 2.5f, 3.5f, 4.5f};
    int ret;
    
    print_test_header("Hybrid Search Test");
    
    printf("🔧 Testing hybrid search (Euclidean + Cosine)...\n");
    
    memset(&req, 0, sizeof(req));
    
    req.query_vector = query_vector;
    req.dimensions = 4;
    req.k = 50;
    req.primary_metric = VEXFS_DISTANCE_EUCLIDEAN;
    req.secondary_metric = VEXFS_DISTANCE_COSINE;
    req.primary_weight = 0.7f;
    req.secondary_weight = 0.3f;
    req.results = results;
    req.result_count = &result_count;
    
    ret = ioctl(fd, VEXFS_IOC_HYBRID_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Hybrid search completed successfully\n");
        printf("   Primary weight: %.2f, Secondary weight: %.2f\n",
               req.primary_weight, req.secondary_weight);
        print_search_results(results, result_count);
    } else {
        printf("❌ Hybrid search failed: %d\n", ret);
        return ret;
    }
    
    /* Test 2: Different metrics */
    printf("\n🔧 Testing hybrid search (Dot Product + Manhattan)...\n");
    
    req.primary_metric = VEXFS_DISTANCE_DOT_PRODUCT;
    req.secondary_metric = VEXFS_DISTANCE_MANHATTAN;
    req.primary_weight = 0.6f;
    req.secondary_weight = 0.4f;
    
    ret = ioctl(fd, VEXFS_IOC_HYBRID_SEARCH, &req);
    if (ret == 0) {
        printf("✅ Dot Product+Manhattan hybrid search completed successfully\n");
        printf("   Primary weight: %.2f, Secondary weight: %.2f\n",
               req.primary_weight, req.secondary_weight);
        print_search_results(results, result_count);
    } else {
        printf("❌ Dot Product+Manhattan hybrid search failed: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int main() {
    int fd;
    int ret = 0;
    
    printf("🚀 VexFS v2.0 Phase 3 Standalone Test Suite\n");
    printf("============================================\n");
    printf("Testing Phase 3 multi-model and advanced search functionality\n");
    
    /* Open VexFS mount point */
    fd = open("/tmp/vexfs_test", O_RDONLY);
    if (fd < 0) {
        perror("❌ Failed to open VexFS mount point");
        printf("💡 Make sure VexFS v2.0 is mounted at /tmp/vexfs_test\n");
        return 1;
    }
    
    printf("✅ VexFS mount point opened successfully\n");
    
    /* Run test suites */
    ret = test_model_metadata(fd);
    if (ret != 0) {
        printf("\n❌ Model metadata test failed\n");
        goto cleanup;
    }
    
    ret = test_filtered_search(fd);
    if (ret != 0) {
        printf("\n❌ Filtered search test failed\n");
        goto cleanup;
    }
    
    ret = test_multi_vector_search(fd);
    if (ret != 0) {
        printf("\n❌ Multi-vector search test failed\n");
        goto cleanup;
    }
    
    ret = test_hybrid_search(fd);
    if (ret != 0) {
        printf("\n❌ Hybrid search test failed\n");
        goto cleanup;
    }
    
    printf("\n🎉 All Phase 3 tests passed!\n");
    printf("📊 Phase 3 functionality is working correctly:\n");
    printf("   ✅ Multi-model embedding support\n");
    printf("   ✅ Filtered search operations\n");
    printf("   ✅ Multi-vector batch search\n");
    printf("   ✅ Hybrid search with multiple metrics\n");
    printf("\n🔍 Check dmesg for detailed kernel logs\n");
    
cleanup:
    close(fd);
    return ret;
}