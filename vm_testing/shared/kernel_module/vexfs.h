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
#include "vexfs_uapi.h"
#include "vexfs_search.h"

/* Phase 3 Feature Flags */
#define VEXFS_FEATURE_MULTI_MODEL     (1 << 0)
#define VEXFS_FEATURE_HNSW_INDEX      (1 << 1)
#define VEXFS_FEATURE_LSH_INDEX       (1 << 2)
#define VEXFS_FEATURE_HYBRID_SEARCH   (1 << 3)
#define VEXFS_FEATURE_FILTERED_SEARCH (1 << 4)

/* Filter Operation Constants */
#define VEXFS_FILTER_EQ    0    /* Equal */
#define VEXFS_FILTER_NE    1    /* Not Equal */
#define VEXFS_FILTER_GT    2    /* Greater Than */
#define VEXFS_FILTER_GE    3    /* Greater Than or Equal */
#define VEXFS_FILTER_LT    4    /* Less Than */
#define VEXFS_FILTER_LE    5    /* Less Than or Equal */
#define VEXFS_FILTER_IN    6    /* In Set */
#define VEXFS_FILTER_NOT_IN 7   /* Not In Set */

/* Filter Field Type Constants */
#define VEXFS_FILTER_FIELD_ID       0    /* Vector ID field */
#define VEXFS_FILTER_FIELD_METADATA 1    /* Metadata field */
#define VEXFS_FILTER_FIELD_TIMESTAMP 2   /* Timestamp field */
#define VEXFS_FILTER_FIELD_CATEGORY 3    /* Category field */
#define VEXFS_FILTER_FIELD_SCORE    4    /* Score field */
#define VEXFS_FILTER_FIELD_RANGE    5    /* Range field */
#define VEXFS_FILTER_FIELD_CUSTOM   6    /* Custom field */

/* String and buffer size constants */
#define VEXFS_MAX_FILTER_STRING     256
#define HNSW_MAX_LAYERS             16

/* Search Filter Structure */
struct vexfs_search_filter {
    uint32_t field_type;        /* Field type (ID, metadata, custom) */
    char field_name[32];        /* Field name for metadata/custom */
    uint32_t operator;          /* Filter operation (EQ, GT, etc.) */
    union {
        uint64_t numeric;       /* Numeric value */
        char string[64];        /* String value */
        uint64_t *set;          /* Set of values for IN/NOT_IN */
        struct {
            uint64_t min;       /* Range minimum value */
            uint64_t max;       /* Range maximum value */
        } range;                /* Range values for range filters */
    } value;
    uint32_t set_size;          /* Size of set for IN/NOT_IN operations */
};

/* Distance Metric Constants (use Phase 2 values for compatibility) */
#define VEXFS_DISTANCE_EUCLIDEAN    0x01
#define VEXFS_DISTANCE_COSINE       0x02
#define VEXFS_DISTANCE_DOT_PRODUCT  0x03
#define VEXFS_DISTANCE_MANHATTAN    0x04


/* Common Constants */
#ifndef UINT64_MAX
#define UINT64_MAX ((uint64_t)-1)
#endif

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
    uint32_t level_multiplier_bits;    /* Level generation multiplier (IEEE 754 bits) */
    uint32_t reserved[4];
} __packed;

/* LSH Index Configuration */
struct vexfs_lsh_config {
    uint32_t num_hash_tables;          /* Number of hash tables */
    uint32_t num_hash_functions;       /* Hash functions per table */
    uint32_t bucket_size;              /* Target bucket size */
    uint32_t hash_width_bits;          /* Hash function width (IEEE 754 bits) */
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
    uint32_t k_per_query;              /* Results per individual query */
    uint32_t distance_metric;
    uint32_t *query_vectors_bits;      /* Array of query vectors (IEEE 754 bits) */
    uint64_t *result_ids;              /* Output: vector IDs */
    uint32_t *result_distances_bits;   /* Output: distances (IEEE 754 bits) */
    uint32_t *result_counts;           /* Output: results per query */
    struct vexfs_search_result *results; /* Output: search results */
} __packed;

/* Filtered Search Request */
struct vexfs_filtered_search {
    uint32_t dimensions;
    uint32_t k;
    uint32_t distance_metric;
    uint32_t *query_vector_bits;       /* Query vector (IEEE 754 bits) */
    
    /* Filter criteria */
    uint32_t filter_count;
    struct {
        char field_name[32];
        uint32_t operator_type;        /* EQ, GT, LT, IN, etc. */
        char value[64];
    } *filters;
    
    /* Results */
    uint64_t *result_ids;
    uint32_t *result_distances_bits;   /* Distances (IEEE 754 bits) */
    uint32_t result_count;
    struct vexfs_search_result *results; /* Output: search results */
} __packed;

/* Hybrid Search Request (Vector + Keyword) */
struct vexfs_hybrid_search {
    /* Vector component */
    uint32_t dimensions;
    uint32_t *query_vector_bits;       /* Query vector (IEEE 754 bits) */
    uint32_t vector_weight_bits;       /* 0.0 - 1.0 (IEEE 754 bits) */
    
    /* Keyword component */
    char keyword_query[256];
    uint32_t keyword_weight_bits;      /* 0.0 - 1.0 (IEEE 754 bits) */
    
    /* Search parameters */
    uint32_t k;
    uint32_t distance_metric;
    uint32_t primary_metric;           /* Primary distance metric */
    uint32_t secondary_metric;         /* Secondary distance metric */
    uint32_t primary_weight_bits;      /* Primary metric weight (IEEE 754 bits) */
    uint32_t secondary_weight_bits;    /* Secondary metric weight (IEEE 754 bits) */
    
    /* Results */
    uint64_t *result_ids;
    uint32_t *result_scores_bits;      /* Combined scores (IEEE 754 bits) */
    uint32_t result_count;
    struct vexfs_search_result *results; /* Output: search results */
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

/* Phase 3 IOCTL Handler */
long vexfs_v2_phase3_ioctl_handler(struct file *file, unsigned int cmd, unsigned long arg);

/* Multi-Model Support */
int vexfs_set_model_metadata(struct vexfs_model_metadata *model_meta);
int vexfs_get_model_metadata(struct vexfs_model_metadata *model_meta);
int vexfs_validate_model_compatibility(vexfs_embedding_model_t model_type, uint32_t dimensions);

/* Advanced Indexing */
int vexfs_build_hnsw_index(struct vexfs_index_metadata *index_meta);
int vexfs_build_lsh_index(struct vexfs_index_metadata *index_meta);
int vexfs_get_index_info(struct vexfs_index_metadata *index_meta);

/* HNSW Statistics Structure */
struct vexfs_hnsw_stats {
    uint32_t node_count;
    uint32_t max_layer;
    uint64_t entry_point_id;
    uint64_t total_searches;
    uint64_t total_insertions;
    uint64_t total_deletions;
    uint64_t distance_calculations;
    uint64_t layer_traversals;
    uint64_t avg_search_time_ns;
    uint64_t avg_insert_time_ns;
    uint64_t memory_usage;
    uint32_t active_searches;
    uint32_t layer_distribution[HNSW_MAX_LAYERS];
};

/* LSH Statistics Structure */
struct vexfs_lsh_stats {
    uint32_t total_vectors;
    uint32_t hash_table_count;
    uint32_t hash_functions_per_table;
    uint64_t total_searches;
    uint64_t total_insertions;
    uint64_t total_hash_computations;
    uint64_t bucket_collisions;
    uint64_t false_positives;
    uint64_t avg_search_time_ns;
    uint64_t avg_insert_time_ns;
    uint64_t memory_usage;
    uint32_t active_searches;
    uint32_t bucket_utilization[32]; /* Max 32 hash tables */
};

/* Advanced Search Statistics Structure */
struct vexfs_advanced_search_stats {
    uint64_t filtered_searches;
    uint64_t multi_vector_searches;
    uint64_t hybrid_searches;
    uint64_t total_filters_applied;
    uint64_t total_vectors_processed;
    uint64_t avg_filter_time_ns;
    uint64_t avg_multi_search_time_ns;
    uint64_t avg_hybrid_time_ns;
};

/* Advanced Search Operations */
int vexfs_multi_vector_search(const struct vexfs_multi_vector_search *request,
                             struct vexfs_search_result *results, uint32_t *result_count);
int vexfs_filtered_search(const struct vexfs_filtered_search *request,
                         struct vexfs_search_result *results, uint32_t *result_count);
int vexfs_hybrid_search(const struct vexfs_hybrid_search *request,
                       struct vexfs_search_result *results, uint32_t *result_count);

/* Advanced Search IOCTL Handler */
long vexfs_advanced_search_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

/* Advanced Search Statistics */
void vexfs_get_advanced_search_stats(struct vexfs_advanced_search_stats *stats);

/* Advanced Search Module Functions */
int vexfs_advanced_search_init(void);
void vexfs_advanced_search_exit(void);

/* Index Management */
int vexfs_index_insert_vector(vexfs_index_type_t index_type, uint64_t vector_id, uint32_t *vector_bits, uint32_t dimensions);
int vexfs_index_remove_vector(vexfs_index_type_t index_type, uint64_t vector_id);
int vexfs_index_update_vector(vexfs_index_type_t index_type, uint64_t vector_id, uint32_t *new_vector_bits, uint32_t dimensions);

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
