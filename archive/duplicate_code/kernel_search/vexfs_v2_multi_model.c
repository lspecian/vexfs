/*
 * VexFS v2.0 Phase 3: Multi-Model Embedding Support
 * 
 * This module provides support for multiple embedding models with different
 * dimensions and characteristics. It handles model metadata, validation,
 * and compatibility checking for various AI embedding providers.
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/time.h>
#include <linux/uaccess.h>

#include "../core/vexfs_v2_phase3.h"

/* Global model metadata storage */
static struct vexfs_model_metadata current_model = {
    .model_type = VEXFS_EMBED_MODEL_UNKNOWN,
    .dimensions = 0,
    .max_sequence_length = 0,
    .model_version = 0,
    .model_name = "unknown",
    .model_description = "No model configured",
    .creation_timestamp = 0
};

/* Model registry with default configurations */
static const struct {
    vexfs_embedding_model_t model_type;
    uint32_t default_dimensions;
    uint32_t max_sequence_length;
    const char *name;
    const char *description;
} model_registry[] = {
    {
        .model_type = VEXFS_EMBED_OLLAMA_NOMIC,
        .default_dimensions = 768,
        .max_sequence_length = 8192,
        .name = "nomic-embed-text",
        .description = "Ollama Nomic Embed Text model (768D)"
    },
    {
        .model_type = VEXFS_EMBED_OLLAMA_MINILM,
        .default_dimensions = 384,
        .max_sequence_length = 512,
        .name = "all-minilm",
        .description = "Ollama All-MiniLM model (384D)"
    },
    {
        .model_type = VEXFS_EMBED_OPENAI_SMALL,
        .default_dimensions = 1536,
        .max_sequence_length = 8191,
        .name = "text-embedding-3-small",
        .description = "OpenAI Text Embedding 3 Small (1536D)"
    },
    {
        .model_type = VEXFS_EMBED_OPENAI_LARGE,
        .default_dimensions = 3072,
        .max_sequence_length = 8191,
        .name = "text-embedding-3-large",
        .description = "OpenAI Text Embedding 3 Large (3072D)"
    },
    {
        .model_type = VEXFS_EMBED_SENTENCE_BERT,
        .default_dimensions = 768,
        .max_sequence_length = 512,
        .name = "sentence-transformers",
        .description = "Sentence-BERT model (variable dimensions)"
    }
};

#define MODEL_REGISTRY_SIZE (sizeof(model_registry) / sizeof(model_registry[0]))

/* Phase 3 statistics */
struct vexfs_phase3_stats phase3_stats = {0};

/**
 * Get model information from registry
 */
static const void *get_model_info(vexfs_embedding_model_t model_type)
{
    int i;
    
    for (i = 0; i < MODEL_REGISTRY_SIZE; i++) {
        if (model_registry[i].model_type == model_type) {
            return &model_registry[i];
        }
    }
    
    return NULL;
}

/**
 * Convert model type to string representation
 */
const char *vexfs_model_type_to_string(vexfs_embedding_model_t model_type)
{
    const void *info = get_model_info(model_type);
    
    if (info) {
        return ((typeof(&model_registry[0]))info)->name;
    }
    
    switch (model_type) {
        case VEXFS_EMBED_MODEL_UNKNOWN:
            return "unknown";
        case VEXFS_EMBED_CUSTOM:
            return "custom";
        default:
            return "invalid";
    }
}

/**
 * Get default dimensions for a model type
 */
uint32_t vexfs_get_model_default_dimensions(vexfs_embedding_model_t model_type)
{
    const void *info = get_model_info(model_type);
    
    if (info) {
        return ((typeof(&model_registry[0]))info)->default_dimensions;
    }
    
    return 0; /* Unknown model */
}

/**
 * Validate model compatibility with current configuration
 */
int vexfs_validate_model_compatibility(vexfs_embedding_model_t model_type, uint32_t dimensions)
{
    const void *info;
    uint32_t expected_dims;
    
    /* Allow unknown/custom models with any dimensions */
    if (model_type == VEXFS_EMBED_MODEL_UNKNOWN || model_type == VEXFS_EMBED_CUSTOM) {
        if (dimensions == 0 || dimensions > 4096) {
            printk(KERN_WARNING "VexFS: Invalid dimensions %u for custom model\n", dimensions);
            return -EINVAL;
        }
        return 0;
    }
    
    info = get_model_info(model_type);
    if (!info) {
        printk(KERN_ERR "VexFS: Unknown model type %d\n", model_type);
        return -EINVAL;
    }
    
    expected_dims = ((typeof(&model_registry[0]))info)->default_dimensions;
    
    /* For sentence-transformers, allow variable dimensions */
    if (model_type == VEXFS_EMBED_SENTENCE_BERT) {
        if (dimensions < 128 || dimensions > 1024) {
            printk(KERN_WARNING "VexFS: Sentence-BERT dimensions %u outside typical range (128-1024)\n", dimensions);
            return -EINVAL;
        }
        return 0;
    }
    
    /* For other models, check exact match */
    if (dimensions != expected_dims) {
        printk(KERN_ERR "VexFS: Model %s expects %u dimensions, got %u\n",
               ((typeof(&model_registry[0]))info)->name, expected_dims, dimensions);
        return -EINVAL;
    }
    
    return 0;
}

/**
 * Set model metadata
 */
int vexfs_set_model_metadata(struct vexfs_model_metadata *model_meta)
{
    const void *info;
    int ret;
    
    if (!model_meta) {
        return -EINVAL;
    }
    
    /* Validate model compatibility */
    ret = vexfs_validate_model_compatibility(model_meta->model_type, model_meta->dimensions);
    if (ret) {
        return ret;
    }
    
    /* Update current model metadata */
    current_model.model_type = model_meta->model_type;
    current_model.dimensions = model_meta->dimensions;
    current_model.max_sequence_length = model_meta->max_sequence_length;
    current_model.model_version = model_meta->model_version;
    current_model.creation_timestamp = ktime_get_real_seconds();
    
    /* Set name and description from registry or user input */
    info = get_model_info(model_meta->model_type);
    if (info) {
        strncpy(current_model.model_name, 
                ((typeof(&model_registry[0]))info)->name, 
                sizeof(current_model.model_name) - 1);
        strncpy(current_model.model_description, 
                ((typeof(&model_registry[0]))info)->description, 
                sizeof(current_model.model_description) - 1);
        
        /* Use registry defaults if not specified */
        if (current_model.max_sequence_length == 0) {
            current_model.max_sequence_length = ((typeof(&model_registry[0]))info)->max_sequence_length;
        }
    } else {
        /* Custom model - use provided names */
        strncpy(current_model.model_name, model_meta->model_name, 
                sizeof(current_model.model_name) - 1);
        strncpy(current_model.model_description, model_meta->model_description, 
                sizeof(current_model.model_description) - 1);
    }
    
    /* Ensure null termination */
    current_model.model_name[sizeof(current_model.model_name) - 1] = '\0';
    current_model.model_description[sizeof(current_model.model_description) - 1] = '\0';
    
    /* Update statistics */
    phase3_stats.multi_model_operations++;
    
    printk(KERN_INFO "VexFS: Model metadata set - %s (%u dimensions)\n",
           current_model.model_name, current_model.dimensions);
    
    return 0;
}

/**
 * Get current model metadata
 */
int vexfs_get_model_metadata(struct vexfs_model_metadata *model_meta)
{
    if (!model_meta) {
        return -EINVAL;
    }
    
    /* Copy current model metadata to user buffer */
    memcpy(model_meta, &current_model, sizeof(struct vexfs_model_metadata));
    
    return 0;
}

/**
 * Handle model metadata IOCTL commands
 */
long vexfs_multi_model_ioctl(unsigned int cmd, unsigned long arg)
{
    struct vexfs_model_metadata model_meta;
    int ret;
    
    switch (cmd) {
        case VEXFS_IOC_SET_MODEL_META:
            if (copy_from_user(&model_meta, (void __user *)arg, sizeof(model_meta))) {
                return -EFAULT;
            }
            
            ret = vexfs_set_model_metadata(&model_meta);
            if (ret) {
                return ret;
            }
            break;
            
        case VEXFS_IOC_GET_MODEL_META:
            ret = vexfs_get_model_metadata(&model_meta);
            if (ret) {
                return ret;
            }
            
            if (copy_to_user((void __user *)arg, &model_meta, sizeof(model_meta))) {
                return -EFAULT;
            }
            break;
            
        default:
            return -ENOTTY;
    }
    
    return 0;
}

/**
 * Initialize multi-model support
 */
int vexfs_multi_model_init(void)
{
    /* Initialize with unknown model */
    current_model.model_type = VEXFS_EMBED_MODEL_UNKNOWN;
    current_model.dimensions = 0;
    current_model.creation_timestamp = ktime_get_real_seconds();
    
    /* Clear statistics */
    memset(&phase3_stats, 0, sizeof(phase3_stats));
    
    printk(KERN_INFO "VexFS: Multi-model support initialized\n");
    printk(KERN_INFO "VexFS: Supported models: Ollama (nomic, minilm), OpenAI (small, large), Sentence-BERT, Custom\n");
    
    return 0;
}

/**
 * Cleanup multi-model support
 */
void vexfs_multi_model_cleanup(void)
{
    printk(KERN_INFO "VexFS: Multi-model support cleaned up\n");
    printk(KERN_INFO "VexFS: Total multi-model operations: %llu\n", 
           phase3_stats.multi_model_operations);
}

/* Export symbols for use by main module */
EXPORT_SYMBOL(vexfs_set_model_metadata);
EXPORT_SYMBOL(vexfs_get_model_metadata);
EXPORT_SYMBOL(vexfs_validate_model_compatibility);
EXPORT_SYMBOL(vexfs_model_type_to_string);
EXPORT_SYMBOL(vexfs_get_model_default_dimensions);
EXPORT_SYMBOL(vexfs_multi_model_ioctl);
EXPORT_SYMBOL(vexfs_multi_model_init);
EXPORT_SYMBOL(vexfs_multi_model_cleanup);
EXPORT_SYMBOL(phase3_stats);