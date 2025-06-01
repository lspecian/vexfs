/*
 * VexFS v2.0 Phase 3: Advanced Indexing & Multi-Model Support
 * 
 * This header defines the advanced indexing infrastructure and multi-model
 * embedding support for VexFS v2.0 Phase 3 implementation.
 */

#ifndef VEXFS_V2_PHASE3_H
#define VEXFS_V2_PHASE3_H

#ifdef __KERNEL__
#include <linux/types.h>
#include <linux/kernel.h>
#include <linux/ioctl.h>
#else
#include <stdint.h>
#include <sys/ioctl.h>
/* Userspace type compatibility */
typedef uint8_t  __u8;
typedef uint16_t __u16;
typedef uint32_t __u32;
typedef uint64_t __u64;
#endif

/* Include the base UAPI definitions */
#include "vexfs_v2_uapi.h"

/* Phase 3 Feature Flags */
#define VEXFS_FEATURE_MULTI_MODEL     (1 << 0)
#define VEXFS_FEATURE_HNSW_INDEX      (1 << 1)
#define VEXFS_FEATURE_LSH_INDEX       (1 << 2)
#define VEXFS_FEATURE_HYBRID_SEARCH   (1 << 3)
#define VEXFS_FEATURE_FILTERED_SEARCH (1 << 4)

/* Multi-Model Embedding Support */
typedef enum {
    VEXFS_EMBED_MODEL_UNKNOWN = 0,
    VEXFS_EMBED_OLLAMA_NOMIC = 1,      /* nomic-embed-text (768D) */
    VEXFS_EMBED_OLLAMA_MINILM = 2,     /* all-minilm (384D) */
    VEXFS_EMBED_OPENAI_SMALL = 3,      /* text-embedding-3-small (1536D) */
    VEXFS_EMBED_OPENAI_LARGE = 4,      /* text-embedding-3-large (3072D) */
    VEXFS_EMBED_SENTENCE_BERT = 5,     /* sentence-transformers (variable) */
    VEXFS_EMBED_CUSTOM = 99            /* Custom model */
} vexfs_embedding_model_t;

/* Model Metadata Structure */
struct vexfs_model_metadata {
    vexfs_embedding_model_t model_type;
    uint32_t dimensions;
    uint32_t max_sequence_length;
    uint32_t model_version;
    char model_name[64];
    char model_description[128];
    uint64_t creation_timestamp;
    uint32_t reserved[8];
} __packed;

/* Advanced Index Types */
typedef enum {
    VEXFS_INDEX_BRUTE_FORCE = 0,       /* Current implementation */
    VEXFS_INDEX_HNSW = 1,              /* Hierarchical Navigable Small World */
    VEXFS_INDEX_LSH = 2,               /* Locality Sensitive Hashing */
    VEXFS_INDEX_IVF = 3,               /* Inverted File */
    VEXFS_INDEX_HYBRID = 4             /* Multiple index combination */
} vexfs_index_type_t;

/* HNSW Index Configuration */
struct vexfs_hnsw_config {
    uint32_t max_connections;          /* M parameter */
    uint32_t ef_construction;          /* efConstruction parameter */
    uint32_t max_layers;               /* Maximum number of layers */
    uint32_t entry_point_search;       /* ef parameter for search */
    float level_multiplier;            /* Level generation multiplier */
    uint32_t reserved[4];
} __packed;

/* LSH Index Configuration */
struct vexfs_lsh_config {
    uint32_t num_hash_tables;          /* Number of hash tables */
    uint32_t num_hash_functions;       /* Hash functions per table */
    uint32_t bucket_size;              /* Target bucket size */
    float hash_width;                  /* Hash function width */
    uint32_t reserved[4];
} __packed;

/* Advanced Index Metadata */
struct vexfs_index_metadata {
    vexfs_index_type_t index_type;
    uint32_t vector_count;
    uint32_t dimensions;
    uint64_t index_size_bytes;
    uint64_t build_timestamp;
    uint64_t last_update_timestamp;
    
    union {
        struct vexfs_hnsw_config hnsw;
        struct vexfs_lsh_config lsh;
        uint32_t raw_config[16];
    } config;
    
    uint32_t reserved[8];
} __packed;

/* Multi-Vector Search Request */
struct vexfs_multi_vector_search {
    uint32_t query_count;              /* Number of query vectors */
    uint32_t dimensions;
    uint32_t k;                        /* Results per query */
    uint32_t distance_metric;
    float *query_vectors;              /* Array of query vectors */
    uint64_t *result_ids;              /* Output: vector IDs */
    float *result_distances;           /* Output: distances */
    uint32_t *result_counts;           /* Output: results per query */
} __packed;

/* Filtered Search Request */
struct vexfs_filtered_search {
    uint32_t dimensions;
    uint32_t k;
    uint32_t distance_metric;
    float *query_vector;
    
    /* Filter criteria */
    uint32_t filter_count;
    struct {
        char field_name[32];
        uint32_t operator_type;        /* EQ, GT, LT, IN, etc. */
        char value[64];
    } *filters;
    
    /* Results */
    uint64_t *result_ids;
    float *result_distances;
    uint32_t result_count;
} __packed;

/* Hybrid Search Request (Vector + Keyword) */
struct vexfs_hybrid_search {
    /* Vector component */
    uint32_t dimensions;
    float *query_vector;
    float vector_weight;               /* 0.0 - 1.0 */
    
    /* Keyword component */
    char keyword_query[256];
    float keyword_weight;              /* 0.0 - 1.0 */
    
    /* Search parameters */
    uint32_t k;
    uint32_t distance_metric;
    
    /* Results */
    uint64_t *result_ids;
    float *result_scores;              /* Combined scores */
    uint32_t result_count;
} __packed;

/* Phase 3 IOCTL Commands */
#define VEXFS_IOC_SET_MODEL_META      _IOW(VEXFS_IOC_MAGIC, 20, struct vexfs_model_metadata)
#define VEXFS_IOC_GET_MODEL_META      _IOR(VEXFS_IOC_MAGIC, 21, struct vexfs_model_metadata)
#define VEXFS_IOC_BUILD_INDEX         _IOW(VEXFS_IOC_MAGIC, 22, struct vexfs_index_metadata)
#define VEXFS_IOC_GET_INDEX_INFO      _IOR(VEXFS_IOC_MAGIC, 23, struct vexfs_index_metadata)
#define VEXFS_IOC_MULTI_VECTOR_SEARCH _IOWR(VEXFS_IOC_MAGIC, 24, struct vexfs_multi_vector_search)
#define VEXFS_IOC_FILTERED_SEARCH     _IOWR(VEXFS_IOC_MAGIC, 25, struct vexfs_filtered_search)
#define VEXFS_IOC_HYBRID_SEARCH       _IOWR(VEXFS_IOC_MAGIC, 26, struct vexfs_hybrid_search)

/* Phase 3 Function Declarations */

/* Multi-Model Support */
int vexfs_set_model_metadata(struct vexfs_model_metadata *model_meta);
int vexfs_get_model_metadata(struct vexfs_model_metadata *model_meta);
int vexfs_validate_model_compatibility(vexfs_embedding_model_t model_type, uint32_t dimensions);

/* Advanced Indexing */
int vexfs_build_hnsw_index(struct vexfs_index_metadata *index_meta);
int vexfs_build_lsh_index(struct vexfs_index_metadata *index_meta);
int vexfs_get_index_info(struct vexfs_index_metadata *index_meta);

/* Advanced Search Operations */
int vexfs_multi_vector_search(struct vexfs_multi_vector_search *search_req);
int vexfs_filtered_search(struct vexfs_filtered_search *search_req);
int vexfs_hybrid_search(struct vexfs_hybrid_search *search_req);

/* Index Management */
int vexfs_index_insert_vector(vexfs_index_type_t index_type, uint64_t vector_id, float *vector, uint32_t dimensions);
int vexfs_index_remove_vector(vexfs_index_type_t index_type, uint64_t vector_id);
int vexfs_index_update_vector(vexfs_index_type_t index_type, uint64_t vector_id, float *new_vector, uint32_t dimensions);

/* Performance Monitoring */
struct vexfs_phase3_stats {
    uint64_t multi_model_operations;
    uint64_t hnsw_searches;
    uint64_t lsh_searches;
    uint64_t filtered_searches;
    uint64_t hybrid_searches;
    uint64_t index_builds;
    uint64_t index_updates;
    
    /* Performance metrics */
    uint64_t avg_hnsw_search_time_ns;
    uint64_t avg_lsh_search_time_ns;
    uint64_t avg_index_build_time_ns;
    
    uint32_t reserved[16];
};

extern struct vexfs_phase3_stats phase3_stats;

/* Utility Functions */
const char *vexfs_model_type_to_string(vexfs_embedding_model_t model_type);
const char *vexfs_index_type_to_string(vexfs_index_type_t index_type);
uint32_t vexfs_get_model_default_dimensions(vexfs_embedding_model_t model_type);

#endif /* VEXFS_V2_PHASE3_H */