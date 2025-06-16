/*
 * VexFS v2.0 Enhanced File System Registration Implementation
 * 
 * This file implements the enhanced filesystem registration system with
 * vector-specific mount options, SIMD capability detection, and compatibility
 * checking for optimal vector database performance.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/parser.h>
#include <linux/string.h>
#include <linux/slab.h>
#include <linux/cpufeature.h>
#include <linux/numa.h>
#include <linux/seq_file.h>
#include <linux/statfs.h>
#include <asm/fpu/api.h>

#include "vexfs_v2_phase3.h"
#include "vexfs_v2_enhanced_registration.h"

/* Global filesystem type structure */
static struct file_system_type vexfs_v2_enhanced_fs_type;

/* 🔥 MOUNT OPTION PARSING IMPLEMENTATION 🔥 */

/**
 * vexfs_set_default_mount_options - Initialize mount options with defaults
 * @opts: Mount options structure to initialize
 */
void vexfs_set_default_mount_options(struct vexfs_mount_opts *opts)
{
    if (!opts)
        return;
    
    memset(opts, 0, sizeof(*opts));
    
    /* Vector configuration defaults */
    opts->max_vector_dim = VEXFS_DEFAULT_MAX_VECTOR_DIM;
    opts->default_element_type = VEXFS_DEFAULT_ELEMENT_TYPE;
    opts->vector_alignment = VEXFS_DEFAULT_VECTOR_ALIGNMENT;
    opts->batch_size = VEXFS_DEFAULT_BATCH_SIZE;
    opts->cache_size_mb = VEXFS_DEFAULT_CACHE_SIZE_MB;
    
    /* SIMD configuration defaults */
    opts->simd_mode = 0; /* Auto-detect */
    opts->numa_aware = true;
    opts->prefetch_size = VEXFS_DEFAULT_PREFETCH_SIZE;
    
    /* Compression defaults */
    opts->compression_enabled = false;
    opts->compression_level = 1;
    
    /* Index configuration defaults */
    opts->hnsw_m = VEXFS_DEFAULT_HNSW_M;
    opts->hnsw_ef_construction = VEXFS_DEFAULT_HNSW_EF_CONSTRUCTION;
    opts->pq_subvectors = VEXFS_DEFAULT_PQ_SUBVECTORS;
    opts->ivf_clusters = VEXFS_DEFAULT_IVF_CLUSTERS;
    
    /* Safety defaults */
    opts->force_compatibility = false;
    opts->disable_simd = false;
    opts->readonly = false;
    opts->debug_level = 0;
    
    /* Validation flags */
    opts->options_parsed = false;
    opts->capabilities_validated = false;
}

/**
 * vexfs_string_to_element_type - Convert string to element type ID
 * @type_name: Element type name string
 * 
 * Returns: Element type ID or 0 if not found
 */
u32 vexfs_string_to_element_type(const char *type_name)
{
    const struct vexfs_element_type_map *map;
    
    if (!type_name)
        return 0;
    
    for (map = vexfs_element_types; map->name; map++) {
        if (strcmp(type_name, map->name) == 0)
            return map->type_id;
    }
    
    return 0;
}

/**
 * vexfs_element_type_to_string - Convert element type ID to string
 * @type_id: Element type ID
 * 
 * Returns: Element type name string or "unknown"
 */
const char *vexfs_element_type_to_string(u32 type_id)
{
    const struct vexfs_element_type_map *map;
    
    for (map = vexfs_element_types; map->name; map++) {
        if (map->type_id == type_id)
            return map->name;
    }
    
    return "unknown";
}

/**
 * vexfs_string_to_simd_mode - Convert string to SIMD mode
 * @mode_name: SIMD mode name string
 * 
 * Returns: SIMD capabilities mask or 0 for auto-detect
 */
u32 vexfs_string_to_simd_mode(const char *mode_name)
{
    const struct vexfs_simd_mode_map *map;
    
    if (!mode_name)
        return 0;
    
    for (map = vexfs_simd_modes; map->name; map++) {
        if (strcmp(mode_name, map->name) == 0)
            return map->required_capabilities;
    }
    
    return 0;
}

/**
 * vexfs_parse_boolean_option - Parse boolean mount option
 * @value: String value ("true", "false", "yes", "no", "1", "0")
 * 
 * Returns: Boolean value or false if invalid
 */
static bool vexfs_parse_boolean_option(const char *value)
{
    if (!value)
        return false;
    
    if (strcmp(value, "true") == 0 || strcmp(value, "yes") == 0 || 
        strcmp(value, "1") == 0 || strcmp(value, "on") == 0)
        return true;
    
    if (strcmp(value, "false") == 0 || strcmp(value, "no") == 0 || 
        strcmp(value, "0") == 0 || strcmp(value, "off") == 0)
        return false;
    
    return false;
}

/**
 * vexfs_parse_options - Parse mount options string
 * @options: Mount options string
 * @opts: Mount options structure to populate
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_parse_options(char *options, struct vexfs_mount_opts *opts)
{
    char *p;
    substring_t args[MAX_OPT_ARGS];
    int token;
    int ret = 0;
    
    if (!opts) {
        printk(KERN_ERR "VexFS v2.0: Invalid mount options structure\n");
        return -EINVAL;
    }
    
    /* Set defaults first */
    vexfs_set_default_mount_options(opts);
    
    if (!options) {
        opts->options_parsed = true;
        return 0;
    }
    
    printk(KERN_INFO "VexFS v2.0: Parsing mount options: %s\n", options);
    
    while ((p = strsep(&options, ",")) != NULL) {
        if (!*p)
            continue;
        
        token = match_token(p, vexfs_mount_tokens, args);
        
        switch (token) {
        case Opt_max_vector_dim:
            if (match_int(&args[0], &opts->max_vector_dim)) {
                vexfs_report_mount_error("max_vector_dim", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (!vexfs_is_valid_vector_dimension(opts->max_vector_dim)) {
                vexfs_report_mount_error("max_vector_dim", args[0].from, "dimension out of range");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_default_element_type:
            opts->default_element_type = vexfs_string_to_element_type(args[0].from);
            if (opts->default_element_type == 0) {
                vexfs_report_mount_error("default_element_type", args[0].from, "unknown element type");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_vector_alignment:
            if (match_int(&args[0], &opts->vector_alignment)) {
                vexfs_report_mount_error("vector_alignment", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (!vexfs_is_valid_alignment(opts->vector_alignment)) {
                vexfs_report_mount_error("vector_alignment", args[0].from, "invalid alignment");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_batch_size:
            if (match_int(&args[0], &opts->batch_size)) {
                vexfs_report_mount_error("batch_size", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (!vexfs_is_valid_batch_size(opts->batch_size)) {
                vexfs_report_mount_error("batch_size", args[0].from, "batch size out of range");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_cache_size:
            if (match_int(&args[0], &opts->cache_size_mb)) {
                vexfs_report_mount_error("cache_size", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (opts->cache_size_mb < 1 || opts->cache_size_mb > 4096) {
                vexfs_report_mount_error("cache_size", args[0].from, "cache size out of range (1-4096 MB)");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_simd_mode:
            opts->forced_simd_capabilities = vexfs_string_to_simd_mode(args[0].from);
            if (strcmp(args[0].from, "auto") != 0 && opts->forced_simd_capabilities == 0) {
                vexfs_report_mount_error("simd_mode", args[0].from, "unknown SIMD mode");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_numa_aware:
            opts->numa_aware = vexfs_parse_boolean_option(args[0].from);
            break;
            
        case Opt_prefetch_size:
            if (match_int(&args[0], &opts->prefetch_size)) {
                vexfs_report_mount_error("prefetch_size", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (opts->prefetch_size < 1 || opts->prefetch_size > 64) {
                vexfs_report_mount_error("prefetch_size", args[0].from, "prefetch size out of range (1-64)");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_compression:
            opts->compression_enabled = vexfs_parse_boolean_option(args[0].from);
            break;
            
        case Opt_hnsw_m:
            if (match_int(&args[0], &opts->hnsw_m)) {
                vexfs_report_mount_error("hnsw_m", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (opts->hnsw_m < 2 || opts->hnsw_m > 64) {
                vexfs_report_mount_error("hnsw_m", args[0].from, "HNSW M out of range (2-64)");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_hnsw_ef_construction:
            if (match_int(&args[0], &opts->hnsw_ef_construction)) {
                vexfs_report_mount_error("hnsw_ef_construction", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (opts->hnsw_ef_construction < 16 || opts->hnsw_ef_construction > 2048) {
                vexfs_report_mount_error("hnsw_ef_construction", args[0].from, "HNSW ef_construction out of range (16-2048)");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        case Opt_force_compatibility:
            opts->force_compatibility = true;
            break;
            
        case Opt_disable_simd:
            opts->disable_simd = true;
            break;
            
        case Opt_readonly:
            opts->readonly = true;
            break;
            
        case Opt_debug_level:
            if (match_int(&args[0], &opts->debug_level)) {
                vexfs_report_mount_error("debug_level", args[0].from, "invalid integer");
                ret = -EINVAL;
                goto out;
            }
            if (opts->debug_level > 5) {
                vexfs_report_mount_error("debug_level", args[0].from, "debug level out of range (0-5)");
                ret = -EINVAL;
                goto out;
            }
            break;
            
        default:
            printk(KERN_WARNING "VexFS v2.0: Unknown mount option: %s\n", p);
            ret = -EINVAL;
            goto out;
        }
    }
    
    opts->options_parsed = true;
    
out:
    if (ret == 0) {
        printk(KERN_INFO "VexFS v2.0: Mount options parsed successfully\n");
        if (opts->debug_level > 0)
            vexfs_print_mount_options(opts);
    } else {
        printk(KERN_ERR "VexFS v2.0: Failed to parse mount options\n");
    }
    
    return ret;
}

/* 🔥 CAPABILITY DETECTION AND VALIDATION 🔥 */

/**
 * vexfs_detect_system_capabilities - Detect system capabilities
 * @check: Capability check structure to populate
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_detect_system_capabilities(struct vexfs_capability_check *check)
{
    if (!check)
        return -EINVAL;
    
    memset(check, 0, sizeof(*check));
    
    /* Detect SIMD capabilities using existing function */
    check->detected_capabilities = detect_simd_capabilities();
    check->optimal_vector_width = detect_simd_vector_width(check->detected_capabilities);
    check->simd_supported = (check->detected_capabilities != 0);
    
    /* Check FPU usability */
    check->fpu_usable = irq_fpu_usable();
    if (!check->fpu_usable) {
        check->warning_message = "FPU not usable in current context";
    }
    
    /* Check NUMA availability */
#ifdef CONFIG_NUMA
    check->numa_available = true;
    check->numa_node_count = num_online_nodes();
#else
    check->numa_available = false;
    check->numa_node_count = 1;
#endif
    
    /* Check large pages */
#ifdef CONFIG_HUGETLB_PAGE
    check->large_pages_available = true;
#else
    check->large_pages_available = false;
#endif
    
    /* Get cache line size */
    check->cache_line_size = cache_line_size();
    
    printk(KERN_INFO "VexFS v2.0: System capabilities detected:\n");
    printk(KERN_INFO "  SIMD: %s (capabilities=0x%x, width=%u)\n",
           check->simd_supported ? "supported" : "not supported",
           check->detected_capabilities, check->optimal_vector_width);
    printk(KERN_INFO "  NUMA: %s (%u nodes)\n",
           check->numa_available ? "available" : "not available",
           check->numa_node_count);
    printk(KERN_INFO "  Large pages: %s\n",
           check->large_pages_available ? "available" : "not available");
    printk(KERN_INFO "  Cache line size: %u bytes\n", check->cache_line_size);
    
    return 0;
}

/**
 * vexfs_validate_simd_requirements - Validate SIMD requirements
 * @opts: Mount options
 * @check: System capability check results
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_validate_simd_requirements(const struct vexfs_mount_opts *opts,
                                    const struct vexfs_capability_check *check)
{
    if (!opts || !check)
        return -EINVAL;
    
    /* If SIMD is disabled, skip validation */
    if (opts->disable_simd) {
        printk(KERN_INFO "VexFS v2.0: SIMD disabled by mount option\n");
        return 0;
    }
    
    /* Check if forced SIMD mode is supported */
    if (opts->forced_simd_capabilities != 0) {
        if ((check->detected_capabilities & opts->forced_simd_capabilities) != opts->forced_simd_capabilities) {
            printk(KERN_ERR "VexFS v2.0: Forced SIMD mode not supported by CPU\n");
            printk(KERN_ERR "  Required: 0x%x, Available: 0x%x\n",
                   opts->forced_simd_capabilities, check->detected_capabilities);
            return -ENODEV;
        }
        printk(KERN_INFO "VexFS v2.0: Using forced SIMD mode (0x%x)\n",
               opts->forced_simd_capabilities);
    }
    
    /* Warn if no SIMD support available */
    if (!check->simd_supported) {
        printk(KERN_WARNING "VexFS v2.0: No SIMD support detected, performance will be reduced\n");
        if (!opts->force_compatibility) {
            printk(KERN_ERR "VexFS v2.0: Use force_compatibility option to proceed without SIMD\n");
            return -ENODEV;
        }
    }
    
    /* Check FPU usability */
    if (!check->fpu_usable) {
        printk(KERN_WARNING "VexFS v2.0: FPU not usable, SIMD operations may fail\n");
    }
    
    return 0;
}

/* 🔥 VALIDATION HELPER FUNCTIONS 🔥 */

/**
 * vexfs_is_valid_vector_dimension - Check if vector dimension is valid
 * @dim: Vector dimension to check
 * 
 * Returns: true if valid, false otherwise
 */
bool vexfs_is_valid_vector_dimension(u32 dim)
{
    return (dim >= 1 && dim <= 65536 && vexfs_is_power_of_two(dim));
}

/**
 * vexfs_is_valid_alignment - Check if alignment is valid
 * @alignment: Alignment to check
 * 
 * Returns: true if valid, false otherwise
 */
bool vexfs_is_valid_alignment(u32 alignment)
{
    return (alignment >= 1 && alignment <= 64 && vexfs_is_power_of_two(alignment));
}

/**
 * vexfs_is_valid_batch_size - Check if batch size is valid
 * @batch_size: Batch size to check
 * 
 * Returns: true if valid, false otherwise
 */
bool vexfs_is_valid_batch_size(u32 batch_size)
{
    return (batch_size >= 1 && batch_size <= 64 && vexfs_is_power_of_two(batch_size));
}

/**
 * vexfs_is_power_of_two - Check if value is a power of two
 * @value: Value to check
 * 
 * Returns: true if power of two, false otherwise
 */
bool vexfs_is_power_of_two(u32 value)
{
    return value != 0 && (value & (value - 1)) == 0;
}

/* 🔥 ERROR REPORTING FUNCTIONS 🔥 */

/**
 * vexfs_report_mount_error - Report mount option parsing error
 * @option: Option name
 * @value: Option value
 * @reason: Error reason
 */
void vexfs_report_mount_error(const char *option, const char *value, const char *reason)
{
    printk(KERN_ERR "VexFS v2.0: Mount option error - %s=%s: %s\n",
           option ? option : "unknown",
           value ? value : "null",
           reason ? reason : "unknown error");
}

/**
 * vexfs_report_capability_warning - Report capability warning
 * @capability: Capability name
 * @impact: Impact description
 */
void vexfs_report_capability_warning(const char *capability, const char *impact)
{
    printk(KERN_WARNING "VexFS v2.0: Capability warning - %s: %s\n",
           capability ? capability : "unknown",
           impact ? impact : "unknown impact");
}

/* 🔥 DEBUG AND MONITORING FUNCTIONS 🔥 */

/**
 * vexfs_print_mount_options - Print parsed mount options
 * @opts: Mount options to print
 */
void vexfs_print_mount_options(const struct vexfs_mount_opts *opts)
{
    if (!opts)
        return;
    
    printk(KERN_INFO "VexFS v2.0: Mount options:\n");
    printk(KERN_INFO "  Vector: max_dim=%u, type=%s, alignment=%u\n",
           opts->max_vector_dim,
           vexfs_element_type_to_string(opts->default_element_type),
           opts->vector_alignment);
    printk(KERN_INFO "  Performance: batch_size=%u, cache_size=%u MB\n",
           opts->batch_size, opts->cache_size_mb);
    printk(KERN_INFO "  SIMD: mode=0x%x, numa_aware=%s, disable_simd=%s\n",
           opts->forced_simd_capabilities,
           opts->numa_aware ? "yes" : "no",
           opts->disable_simd ? "yes" : "no");
    printk(KERN_INFO "  Index: hnsw_m=%u, hnsw_ef=%u\n",
           opts->hnsw_m, opts->hnsw_ef_construction);
    printk(KERN_INFO "  Safety: force_compat=%s, readonly=%s, debug=%u\n",
           opts->force_compatibility ? "yes" : "no",
           opts->readonly ? "yes" : "no",
           opts->debug_level);
}

/**
 * vexfs_print_capability_report - Print system capability report
 * @check: Capability check results to print
 */
void vexfs_print_capability_report(const struct vexfs_capability_check *check)
{
    if (!check)
        return;
    
    printk(KERN_INFO "VexFS v2.0: System capability report:\n");
    printk(KERN_INFO "  SIMD: %s (0x%x, %u-bit vectors)\n",
           check->simd_supported ? "supported" : "not supported",
           check->detected_capabilities, check->optimal_vector_width);
    printk(KERN_INFO "  NUMA: %s (%u nodes)\n",
           check->numa_available ? "available" : "not available",
           check->numa_node_count);
    printk(KERN_INFO "  Large pages: %s\n",
           check->large_pages_available ? "available" : "not available");
    printk(KERN_INFO "  FPU: %s\n",
           check->fpu_usable ? "usable" : "not usable");
    printk(KERN_INFO "  Cache line: %u bytes\n", check->cache_line_size);
    
    if (check->warning_message)
        printk(KERN_WARNING "VexFS v2.0: Warning: %s\n", check->warning_message);
    if (check->error_message)
        printk(KERN_ERR "VexFS v2.0: Error: %s\n", check->error_message);
}