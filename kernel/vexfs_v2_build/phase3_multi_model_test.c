/*
 * VexFS v2.0 Phase 3 Multi-Model Test Program
 * 
 * This program tests the multi-model embedding support functionality
 * of VexFS v2.0 Phase 3, including model metadata operations and
 * compatibility validation.
 */

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

/* Include Phase 3 definitions */
#include "vexfs_v2_phase3.h"

void print_test_header(const char *test_name) {
    printf("\n🧪 %s\n", test_name);
    printf("================================================\n");
}

void print_model_info(const struct vexfs_model_metadata *model) {
    printf("📊 Model Information:\n");
    printf("   Type: %d (%s)\n", model->model_type, 
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

int test_model_metadata_operations(int fd) {
    struct vexfs_model_metadata model;
    int ret;
    
    print_test_header("Multi-Model Metadata Operations Test");
    
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
    
    /* Test 3: Set OpenAI Small model */
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
    } else {
        printf("❌ Failed to set OpenAI Small model metadata: %d\n", ret);
        return ret;
    }
    
    /* Test 4: Verify updated metadata */
    printf("\n🔧 Test 4: Verifying updated model metadata...\n");
    memset(&model, 0, sizeof(model));
    ret = ioctl(fd, VEXFS_IOC_GET_MODEL_META, &model);
    if (ret == 0) {
        printf("✅ Updated model metadata retrieved successfully\n");
        print_model_info(&model);
        
        if (model.model_type == VEXFS_EMBED_OPENAI_SMALL && model.dimensions == 1536) {
            printf("✅ Model metadata correctly updated to OpenAI Small\n");
        } else {
            printf("❌ Model metadata not correctly updated\n");
            return -1;
        }
    } else {
        printf("❌ Failed to get updated model metadata: %d\n", ret);
        return ret;
    }
    
    /* Test 5: Set custom model */
    printf("\n🔧 Test 5: Setting custom model metadata...\n");
    memset(&model, 0, sizeof(model));
    model.model_type = VEXFS_EMBED_CUSTOM;
    model.dimensions = 512;
    model.max_sequence_length = 1024;
    model.model_version = 1;
    strcpy(model.model_name, "custom-bert-base");
    strcpy(model.model_description, "Custom BERT Base model fine-tuned for domain");
    
    ret = ioctl(fd, VEXFS_IOC_SET_MODEL_META, &model);
    if (ret == 0) {
        printf("✅ Custom model metadata set successfully\n");
        
        /* Verify custom model */
        memset(&model, 0, sizeof(model));
        ret = ioctl(fd, VEXFS_IOC_GET_MODEL_META, &model);
        if (ret == 0) {
            print_model_info(&model);
        }
    } else {
        printf("❌ Failed to set custom model metadata: %d\n", ret);
        return ret;
    }
    
    return 0;
}

int test_model_validation(int fd) {
    struct vexfs_model_metadata model;
    int ret;
    
    print_test_header("Model Validation Test");
    
    /* Test 1: Invalid dimensions for known model */
    printf("🔧 Test 1: Testing invalid dimensions for Ollama Nomic (should fail)...\n");
    memset(&model, 0, sizeof(model));
    model.model_type = VEXFS_EMBED_OLLAMA_NOMIC;
    model.dimensions = 1024; /* Wrong - should be 768 */
    model.max_sequence_length = 8192;
    strcpy(model.model_name, "nomic-embed-text");
    
    ret = ioctl(fd, VEXFS_IOC_SET_MODEL_META, &model);
    if (ret != 0) {
        printf("✅ Correctly rejected invalid dimensions for Ollama Nomic\n");
    } else {
        printf("❌ Should have rejected invalid dimensions for Ollama Nomic\n");
        return -1;
    }
    
    /* Test 2: Valid Sentence-BERT with variable dimensions */
    printf("\n🔧 Test 2: Testing Sentence-BERT with variable dimensions...\n");
    memset(&model, 0, sizeof(model));
    model.model_type = VEXFS_EMBED_SENTENCE_BERT;
    model.dimensions = 384; /* Valid range for Sentence-BERT */
    model.max_sequence_length = 512;
    strcpy(model.model_name, "sentence-transformers/all-MiniLM-L6-v2");
    strcpy(model.model_description, "Sentence-BERT MiniLM model");
    
    ret = ioctl(fd, VEXFS_IOC_SET_MODEL_META, &model);
    if (ret == 0) {
        printf("✅ Sentence-BERT with 384 dimensions accepted\n");
    } else {
        printf("❌ Failed to set valid Sentence-BERT model: %d\n", ret);
        return ret;
    }
    
    /* Test 3: Invalid dimensions for custom model */
    printf("\n🔧 Test 3: Testing invalid dimensions for custom model (should fail)...\n");
    memset(&model, 0, sizeof(model));
    model.model_type = VEXFS_EMBED_CUSTOM;
    model.dimensions = 5000; /* Too large */
    model.max_sequence_length = 1024;
    strcpy(model.model_name, "invalid-custom");
    
    ret = ioctl(fd, VEXFS_IOC_SET_MODEL_META, &model);
    if (ret != 0) {
        printf("✅ Correctly rejected invalid dimensions for custom model\n");
    } else {
        printf("❌ Should have rejected invalid dimensions for custom model\n");
        return -1;
    }
    
    return 0;
}

int main() {
    int fd;
    int ret = 0;
    
    printf("🚀 VexFS v2.0 Phase 3 Multi-Model Test Suite\n");
    printf("=============================================\n");
    printf("Testing multi-model embedding support functionality\n");
    
    /* Open VexFS mount point */
    fd = open("/tmp/vexfs_test", O_RDONLY);
    if (fd < 0) {
        perror("❌ Failed to open VexFS mount point");
        printf("💡 Make sure VexFS v2.0 is mounted at /tmp/vexfs_test\n");
        return 1;
    }
    
    printf("✅ VexFS mount point opened successfully\n");
    
    /* Run test suites */
    ret = test_model_metadata_operations(fd);
    if (ret != 0) {
        printf("\n❌ Model metadata operations test failed\n");
        goto cleanup;
    }
    
    ret = test_model_validation(fd);
    if (ret != 0) {
        printf("\n❌ Model validation test failed\n");
        goto cleanup;
    }
    
    printf("\n🎉 All Phase 3 Multi-Model tests passed!\n");
    printf("📊 Multi-model embedding support is working correctly\n");
    printf("\n🔍 Check dmesg for detailed kernel logs\n");
    
cleanup:
    close(fd);
    return ret;
}