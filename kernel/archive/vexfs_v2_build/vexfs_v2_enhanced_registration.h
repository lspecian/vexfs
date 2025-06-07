/*
 * VexFS v2.0 Enhanced File System Registration
 * 
 * Extended filesystem registration to support vector-specific mount options
 * and capability detection for optimal vector database performance.
 * 
 * Features:
 * - Vector-specific mount options (max_vector_dim, default_element_type, etc.)
 * - SIMD capability detection at mount time using cpu_has_feature()
 * - Compatibility checks for existing VexFS volumes
 * - Vector-specific operations registration with the VFS layer
 */

#ifndef VEXFS_V2_ENHANCED_REGISTRATION_H
#define VEXFS_V2_ENHANCED_REGISTRATION_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/parser.h>
#include <linux/cpufeature.h>
#include <linux/numa.h>

/* Enhanced mount option tokens */
enum vexfs_mount_options {
    /* Vector-specific options */
    Opt_max_vector_dim,
    Opt_default_element_type,
    Opt_vector_alignment,
    Opt_batch_size,
    Opt_cache_size,
    
    /* SIMD and performance options */
    Opt_simd_mode,
    Opt_numa_aware,
    Opt_prefetch_size,
    Opt_compression,
    
    /* Index configuration options */
    Opt_hnsw_m,
    Opt_hnsw_ef_construction,
    Opt_pq_subvectors,
    Opt_ivf_clusters,
    
    /* Compatibility and safety options */
    Opt_force_compatibility,
    Opt_disable_simd,
    Opt_readonly,
    Opt_debug_level,
    
    /* Error handling */
    Opt_err
};

/* Mount option parsing table */
static const match_table_t vexfs_mount_tokens = {
    /* Vector-specific options */
    {Opt_max_vector_dim, "max_vector_dim=%u"},
    {Opt_default_element_type, "default_element_type=%s"},
    {Opt_vector_alignment, "vector_alignment=%u"},
    {Opt_batch_size, "batch_size=%u"},
    {Opt_cache_size, "cache_size=%u"},
    
    /* SIMD and performance options */
    {Opt_simd_mode, "simd_mode=%s"},
    {Opt_numa_aware, "numa_aware=%s"},
    {Opt_prefetch_size, "prefetch_size=%u"},
    {Opt_compression, "compression=%s"},
    
    /* Index configuration options */
    {Opt_hnsw_m, "hnsw_m=%u"},
    {Opt_hnsw_ef_construction, "hnsw_ef_construction=%u"},
    {Opt_pq_subvectors, "pq_subvectors=%u"},
    {Opt_ivf_clusters, "ivf_clusters=%u"},
    
    /* Compatibility and safety options */
    {Opt_force_compatibility, "force_compatibility"},
    {Opt_disable_simd, "disable_simd"},
    {Opt_readonly, "readonly"},
    {Opt_debug_level, "debug_level=%u"},
    
    /* Error handling */
    {Opt_err, NULL}
};

/* Vector element type mapping */
struct vexfs_element_type_map {
    const char *name;
    u32 type_id;
    u32 size_bytes;
    u32 alignment;
};

static const struct vexfs_element_type_map vexfs_element_types[] = {
    {"float32", VEXFS_VECTOR_FLOAT32, 4, 4},
    {"float16", VEXFS_VECTOR_FLOAT16, 2, 2},
    {"int8", VEXFS_VECTOR_INT8, 1, 1},
    {"binary", VEXFS_VECTOR_BINARY, 1, 1},
    {NULL, 0, 0, 0}
};

/* SIMD mode mapping */
struct vexfs_simd_mode_map {
    const char *name;
    u32 required_capabilities;
    u32 vector_width;
    const char *description;
};

static const struct vexfs_simd_mode_map vexfs_simd_modes[] = {
    {"auto", 0, 0, "Automatic SIMD detection"},
    {"sse2", VEXFS_SIMD_SSE2, 128, "Force SSE2 mode"},
    {"avx2", VEXFS_SIMD_AVX2, 256, "Force AVX2 mode"},
    {"avx512", VEXFS_SIMD_AVX512, 512, "Force AVX-512 mode"},
    {"scalar", 0, 64, "Disable SIMD (scalar mode)"},
    {NULL, 0, 0, NULL}
};

/* Enhanced mount options structure */
struct vexfs_mount_opts {
    /* Vector configuration */
    u32 max_vector_dim;
    u32 default_element_type;
    u32 vector_alignment;
    u32 batch_size;
    u32 cache_size_mb;
    
    /* SIMD configuration */
    u32 simd_mode;
    u32 forced_simd_capabilities;
    u32 forced_vector_width;
    bool numa_aware;
    u32 prefetch_size;
    
    /* Compression settings */
    bool compression_enabled;
    u32 compression_level;
    
    /* Index configuration */
    u32 hnsw_m;
    u32 hnsw_ef_construction;
    u32 pq_subvectors;
    u32 ivf_clusters;
    
    /* Compatibility and safety */
    bool force_compatibility;
    bool disable_simd;
    bool readonly;
    u32 debug_level;
    
    /* Validation flags */
    bool options_parsed;
    bool capabilities_validated;
};

/* Default mount options */
#define VEXFS_DEFAULT_MAX_VECTOR_DIM    4096
#define VEXFS_DEFAULT_ELEMENT_TYPE      VEXFS_VECTOR_FLOAT32
#define VEXFS_DEFAULT_VECTOR_ALIGNMENT  32
#define VEXFS_DEFAULT_BATCH_SIZE        8
#define VEXFS_DEFAULT_CACHE_SIZE_MB     64
#define VEXFS_DEFAULT_PREFETCH_SIZE     16
#define VEXFS_DEFAULT_HNSW_M            16
#define VEXFS_DEFAULT_HNSW_EF_CONSTRUCTION 200
#define VEXFS_DEFAULT_PQ_SUBVECTORS     8
#define VEXFS_DEFAULT_IVF_CLUSTERS      256

/* Capability validation results */
struct vexfs_capability_check {
    bool simd_supported;
    bool numa_available;
    bool large_pages_available;
    bool fpu_usable;
    u32 detected_capabilities;
    u32 optimal_vector_width;
    u32 cache_line_size;
    u32 numa_node_count;
    const char *warning_message;
    const char *error_message;
};

/* Function declarations */

/* Mount option parsing */
int vexfs_parse_options(char *options, struct vexfs_mount_opts *opts);
int vexfs_validate_mount_options(struct vexfs_mount_opts *opts);
void vexfs_set_default_mount_options(struct vexfs_mount_opts *opts);

/* Capability detection and validation */
int vexfs_detect_system_capabilities(struct vexfs_capability_check *check);
int vexfs_validate_simd_requirements(const struct vexfs_mount_opts *opts,
                                    const struct vexfs_capability_check *check);
int vexfs_check_volume_compatibility(struct super_block *sb,
                                   const struct vexfs_mount_opts *opts);

/* Enhanced filesystem registration */
int vexfs_register_enhanced_filesystem(void);
void vexfs_unregister_enhanced_filesystem(void);

/* Enhanced mount/unmount operations */
struct dentry *vexfs_v2_enhanced_mount(struct file_system_type *fs_type,
                                      int flags, const char *dev_name, void *data);
int vexfs_v2_enhanced_fill_super(struct super_block *sb, void *data, int silent);
void vexfs_v2_enhanced_kill_sb(struct super_block *sb);

/* Vector-specific VFS operations registration */
int vexfs_register_vector_operations(struct super_block *sb);
void vexfs_unregister_vector_operations(struct super_block *sb);

/* Utility functions */
const char *vexfs_element_type_to_string(u32 type_id);
u32 vexfs_string_to_element_type(const char *type_name);
const char *vexfs_simd_mode_to_string(u32 capabilities);
u32 vexfs_string_to_simd_mode(const char *mode_name);

/* Mount option validation helpers */
bool vexfs_is_valid_vector_dimension(u32 dim);
bool vexfs_is_valid_alignment(u32 alignment);
bool vexfs_is_valid_batch_size(u32 batch_size);
bool vexfs_is_power_of_two(u32 value);

/* System requirement checking */
bool vexfs_check_minimum_requirements(void);
bool vexfs_check_kernel_version_compatibility(void);
bool vexfs_check_cpu_features(u32 required_features);

/* Error reporting */
void vexfs_report_mount_error(const char *option, const char *value, const char *reason);
void vexfs_report_capability_warning(const char *capability, const char *impact);

/* Debug and monitoring */
void vexfs_print_mount_options(const struct vexfs_mount_opts *opts);
void vexfs_print_capability_report(const struct vexfs_capability_check *check);
void vexfs_print_compatibility_status(struct super_block *sb);

/* Mount option string builders */
int vexfs_build_mount_option_string(const struct vexfs_mount_opts *opts,
                                   char *buffer, size_t buffer_size);
int vexfs_show_mount_options(struct seq_file *seq, struct dentry *dentry);

#endif /* VEXFS_V2_ENHANCED_REGISTRATION_H */