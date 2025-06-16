/*
 * VexFS Semantic Extensions
 * 
 * This header defines the semantic vector database extensions
 * that build on top of the core VFS-compliant filesystem.
 */

#ifndef VEXFS_SEMANTIC_H
#define VEXFS_SEMANTIC_H

#include "vexfs_core.h"
#include <linux/ioctl.h>

/* Semantic feature flags */
#define VEXFS_FEATURE_VECTORS     (1 << 0)
#define VEXFS_FEATURE_SEARCH      (1 << 1)
#define VEXFS_FEATURE_INDEXING    (1 << 2)
#define VEXFS_FEATURE_MULTI_MODEL (1 << 3)

/* Vector constants */
#define VEXFS_MAX_VECTOR_DIM 4096
#define VEXFS_MAX_VECTORS_PER_FILE 1000000

/* Distance metrics */
#define VEXFS_DISTANCE_EUCLIDEAN    0x01
#define VEXFS_DISTANCE_COSINE       0x02
#define VEXFS_DISTANCE_DOT_PRODUCT  0x03
#define VEXFS_DISTANCE_MANHATTAN    0x04

/* Embedding models */
typedef enum {
    VEXFS_EMBED_MODEL_UNKNOWN = 0,
    VEXFS_EMBED_OLLAMA_NOMIC = 1,
    VEXFS_EMBED_OLLAMA_MINILM = 2,
    VEXFS_EMBED_OPENAI_SMALL = 3,
    VEXFS_EMBED_OPENAI_LARGE = 4,
    VEXFS_EMBED_CUSTOM = 99
} vexfs_embedding_model_t;

/* Vector data structure */
struct vexfs_vector {
    __u32 id;
    __u32 dimension;
    __u32 model_type;
    __u32 flags;
    float *data;  /* Vector components */
    char metadata[256];
};

/* Search request structure */
struct vexfs_search_request {
    struct vexfs_vector query;
    __u32 k;  /* Number of results */
    __u32 distance_metric;
    __u32 flags;
};

/* Search result structure */
struct vexfs_search_result {
    __u32 vector_id;
    __u32 distance;  /* Fixed-point distance */
    char metadata[256];
};

/* IOCTL definitions */
#define VEXFS_IOC_MAGIC 'V'
#define VEXFS_IOC_ADD_VECTOR    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector)
#define VEXFS_IOC_SEARCH        _IOWR(VEXFS_IOC_MAGIC, 2, struct vexfs_search_request)
#define VEXFS_IOC_GET_STATS     _IOR(VEXFS_IOC_MAGIC, 3, struct vexfs_stats)

/* Semantic operations */
extern const struct file_operations vexfs_semantic_fops;

/* Function declarations */
long vexfs_ioctl(struct file *file, unsigned int cmd, unsigned long arg);
int vexfs_add_vector(struct inode *inode, struct vexfs_vector *vector);
int vexfs_search_vectors(struct inode *inode, struct vexfs_search_request *req,
                        struct vexfs_search_result *results);

/* Extended attributes for semantic metadata */
int vexfs_setxattr(struct dentry *dentry, struct inode *inode,
                  const char *name, const void *value, size_t size, int flags);
ssize_t vexfs_getxattr(struct dentry *dentry, struct inode *inode,
                      const char *name, void *buffer, size_t size);
ssize_t vexfs_listxattr(struct dentry *dentry, char *buffer, size_t size);
int vexfs_removexattr(struct dentry *dentry, const char *name);

#endif /* VEXFS_SEMANTIC_H */