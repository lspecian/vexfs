/*
 * VexFS v2.0 Enhanced File System Registration Implementation - Part 2
 * 
 * This file contains the enhanced mount/unmount operations and filesystem
 * registration functions for the VexFS v2.0 enhanced registration system.
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

/* Forward declarations from vexfs_v2_main.c */
extern int vexfs_v2_fill_super(struct super_block *sb, void *data, int silent);
extern void vexfs_v2_kill_sb(struct super_block *sb);
extern struct dentry *vexfs_v2_mount(struct file_system_type *fs_type,
                                     int flags, const char *dev_name, void *data);

/* 🔥 ENHANCED MOUNT OPERATIONS 🔥 */

/**
 * vexfs_check_volume_compatibility - Check compatibility with existing volume
 * @sb: Super block
 * @opts: Mount options
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_check_volume_compatibility(struct super_block *sb,
                                   const struct vexfs_mount_opts *opts)
{
    struct vexfs_v2_sb_info *sbi;
    
    if (!sb || !opts)
        return -EINVAL;
    
    sbi = VEXFS_V2_SB(sb);
    if (!sbi)
        return -EINVAL;
    
    printk(KERN_INFO "VexFS v2.0: Checking volume compatibility\n");
    
    /* Check vector dimension compatibility */
    if (opts->max_vector_dim < sbi->max_vector_dim) {
        printk(KERN_ERR "VexFS v2.0: Mount max_vector_dim (%u) < volume max_vector_dim (%u)\n",
               opts->max_vector_dim, sbi->max_vector_dim);
        if (!opts->force_compatibility)
            return -EINVAL;
        printk(KERN_WARNING "VexFS v2.0: Forcing compatibility despite dimension mismatch\n");
    }
    
    /* Check element type compatibility */
    if (opts->default_element_type != sbi->default_element_type) {
        printk(KERN_WARNING "VexFS v2.0: Mount element type (%s) != volume element type (%s)\n",
               vexfs_element_type_to_string(opts->default_element_type),
               vexfs_element_type_to_string(sbi->default_element_type));
        if (!opts->force_compatibility) {
            printk(KERN_ERR "VexFS v2.0: Use force_compatibility to override element type mismatch\n");
            return -EINVAL;
        }
    }
    
    /* Check alignment compatibility */
    if (opts->vector_alignment > sbi->vector_alignment) {
        printk(KERN_WARNING "VexFS v2.0: Mount alignment (%u) > volume alignment (%u)\n",
               opts->vector_alignment, sbi->vector_alignment);
        /* This is usually safe, just warn */
    }
    
    /* Update superblock with mount options where appropriate */
    if (opts->batch_size != sbi->batch_size) {
        printk(KERN_INFO "VexFS v2.0: Updating batch size from %u to %u\n",
               sbi->batch_size, opts->batch_size);
        sbi->batch_size = opts->batch_size;
    }
    
    if (opts->debug_level > 0) {
        sbi->debug_level = opts->debug_level;
        printk(KERN_INFO "VexFS v2.0: Debug level set to %u\n", opts->debug_level);
    }
    
    printk(KERN_INFO "VexFS v2.0: Volume compatibility check passed\n");
    return 0;
}

/**
 * vexfs_apply_mount_options_to_sb - Apply mount options to superblock
 * @sb: Super block
 * @opts: Mount options
 * @check: System capability check results
 * 
 * Returns: 0 on success, negative error code on failure
 */
static int vexfs_apply_mount_options_to_sb(struct super_block *sb,
                                          const struct vexfs_mount_opts *opts,
                                          const struct vexfs_capability_check *check)
{
    struct vexfs_v2_sb_info *sbi;
    
    if (!sb || !opts || !check)
        return -EINVAL;
    
    sbi = VEXFS_V2_SB(sb);
    if (!sbi)
        return -EINVAL;
    
    printk(KERN_INFO "VexFS v2.0: Applying mount options to superblock\n");
    
    /* Apply SIMD configuration */
    if (opts->disable_simd) {
        sbi->simd_capabilities = 0;
        sbi->simd_vector_width = 64; /* Scalar mode */
        printk(KERN_INFO "VexFS v2.0: SIMD disabled by mount option\n");
    } else if (opts->forced_simd_capabilities != 0) {
        sbi->simd_capabilities = opts->forced_simd_capabilities;
        sbi->simd_vector_width = detect_simd_vector_width(opts->forced_simd_capabilities);
        printk(KERN_INFO "VexFS v2.0: Using forced SIMD mode (0x%x, %u-bit)\n",
               sbi->simd_capabilities, sbi->simd_vector_width);
    } else {
        /* Use detected capabilities */
        sbi->simd_capabilities = check->detected_capabilities;
        sbi->simd_vector_width = check->optimal_vector_width;
        printk(KERN_INFO "VexFS v2.0: Using auto-detected SIMD (0x%x, %u-bit)\n",
               sbi->simd_capabilities, sbi->simd_vector_width);
    }
    
    /* Apply performance options */
    sbi->batch_size = opts->batch_size;
    sbi->prefetch_size = opts->prefetch_size;
    
    /* Apply NUMA configuration */
    if (opts->numa_aware && check->numa_available) {
        sbi->numa_aware = true;
        sbi->numa_node_count = check->numa_node_count;
        printk(KERN_INFO "VexFS v2.0: NUMA awareness enabled (%u nodes)\n",
               sbi->numa_node_count);
    } else {
        sbi->numa_aware = false;
        sbi->numa_node_count = 1;
        if (opts->numa_aware && !check->numa_available) {
            printk(KERN_WARNING "VexFS v2.0: NUMA requested but not available\n");
        }
    }
    
    /* Apply index configuration */
    sbi->hnsw_m = opts->hnsw_m;
    sbi->hnsw_ef_construction = opts->hnsw_ef_construction;
    
    /* Apply safety options */
    if (opts->readonly) {
        sb->s_flags |= SB_RDONLY;
        printk(KERN_INFO "VexFS v2.0: Mounted read-only\n");
    }
    
    sbi->debug_level = opts->debug_level;
    
    /* Apply cache configuration */
    sbi->cache_size_mb = opts->cache_size_mb;
    
    printk(KERN_INFO "VexFS v2.0: Mount options applied successfully\n");
    return 0;
}

/**
 * vexfs_v2_enhanced_fill_super - Enhanced superblock initialization
 * @sb: Super block to initialize
 * @data: Mount data (options string)
 * @silent: Silent mount flag
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_v2_enhanced_fill_super(struct super_block *sb, void *data, int silent)
{
    struct vexfs_mount_opts opts;
    struct vexfs_capability_check check;
    char *options = (char *)data;
    int ret;
    
    printk(KERN_INFO "VexFS v2.0: Enhanced superblock initialization\n");
    
    /* Parse mount options */
    ret = vexfs_parse_options(options, &opts);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: Failed to parse mount options\n");
        return ret;
    }
    
    /* Detect system capabilities */
    ret = vexfs_detect_system_capabilities(&check);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: Failed to detect system capabilities\n");
        return ret;
    }
    
    /* Validate SIMD requirements */
    ret = vexfs_validate_simd_requirements(&opts, &check);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: SIMD requirements validation failed\n");
        return ret;
    }
    
    /* Call original fill_super function */
    ret = vexfs_v2_fill_super(sb, data, silent);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: Original fill_super failed\n");
        return ret;
    }
    
    /* Check volume compatibility */
    ret = vexfs_check_volume_compatibility(sb, &opts);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: Volume compatibility check failed\n");
        goto cleanup_super;
    }
    
    /* Apply mount options to superblock */
    ret = vexfs_apply_mount_options_to_sb(sb, &opts, &check);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: Failed to apply mount options\n");
        goto cleanup_super;
    }
    
    /* Register vector-specific operations */
    ret = vexfs_register_vector_operations(sb);
    if (ret) {
        if (!silent)
            printk(KERN_ERR "VexFS v2.0: Failed to register vector operations\n");
        goto cleanup_super;
    }
    
    if (opts.debug_level > 0) {
        vexfs_print_mount_options(&opts);
        vexfs_print_capability_report(&check);
        vexfs_print_compatibility_status(sb);
    }
    
    printk(KERN_INFO "VexFS v2.0: Enhanced superblock initialization completed successfully\n");
    return 0;
    
cleanup_super:
    /* Cleanup will be handled by kill_sb */
    return ret;
}

/**
 * vexfs_v2_enhanced_mount - Enhanced mount operation
 * @fs_type: File system type
 * @flags: Mount flags
 * @dev_name: Device name
 * @data: Mount data (options)
 * 
 * Returns: Dentry on success, ERR_PTR on failure
 */
struct dentry *vexfs_v2_enhanced_mount(struct file_system_type *fs_type,
                                      int flags, const char *dev_name, void *data)
{
    printk(KERN_INFO "VexFS v2.0: Enhanced mount operation starting\n");
    printk(KERN_INFO "VexFS v2.0: Device: %s, Options: %s\n",
           dev_name ? dev_name : "none",
           data ? (char *)data : "none");
    
    /* Use mount_nodev with enhanced fill_super */
    return mount_nodev(fs_type, flags, data, vexfs_v2_enhanced_fill_super);
}

/**
 * vexfs_v2_enhanced_kill_sb - Enhanced superblock cleanup
 * @sb: Super block to cleanup
 */
void vexfs_v2_enhanced_kill_sb(struct super_block *sb)
{
    printk(KERN_INFO "VexFS v2.0: Enhanced superblock cleanup\n");
    
    /* Unregister vector operations */
    vexfs_unregister_vector_operations(sb);
    
    /* Call original kill_sb */
    vexfs_v2_kill_sb(sb);
    
    printk(KERN_INFO "VexFS v2.0: Enhanced superblock cleanup completed\n");
}

/* 🔥 VECTOR OPERATIONS REGISTRATION 🔥 */

/**
 * vexfs_register_vector_operations - Register vector-specific VFS operations
 * @sb: Super block
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_register_vector_operations(struct super_block *sb)
{
    struct vexfs_v2_sb_info *sbi;
    
    if (!sb)
        return -EINVAL;
    
    sbi = VEXFS_V2_SB(sb);
    if (!sbi)
        return -EINVAL;
    
    printk(KERN_INFO "VexFS v2.0: Registering vector-specific operations\n");
    
    /* Vector operations are already registered in the original implementation */
    /* This function serves as a placeholder for future vector-specific VFS operations */
    
    /* Mark vector operations as registered */
    sbi->vector_ops_registered = true;
    
    printk(KERN_INFO "VexFS v2.0: Vector operations registered successfully\n");
    return 0;
}

/**
 * vexfs_unregister_vector_operations - Unregister vector-specific VFS operations
 * @sb: Super block
 */
void vexfs_unregister_vector_operations(struct super_block *sb)
{
    struct vexfs_v2_sb_info *sbi;
    
    if (!sb)
        return;
    
    sbi = VEXFS_V2_SB(sb);
    if (!sbi)
        return;
    
    if (sbi->vector_ops_registered) {
        printk(KERN_INFO "VexFS v2.0: Unregistering vector-specific operations\n");
        
        /* Vector operations cleanup would go here */
        
        sbi->vector_ops_registered = false;
        printk(KERN_INFO "VexFS v2.0: Vector operations unregistered\n");
    }
}

/* 🔥 ENHANCED FILESYSTEM TYPE STRUCTURE 🔥 */

static struct file_system_type vexfs_v2_enhanced_fs_type = {
    .owner          = THIS_MODULE,
    .name           = "vexfs",
    .mount          = vexfs_v2_enhanced_mount,
    .kill_sb        = vexfs_v2_enhanced_kill_sb,
    .fs_flags       = FS_REQUIRES_DEV | FS_BINARY_MOUNTDATA,
};

/* 🔥 ENHANCED FILESYSTEM REGISTRATION FUNCTIONS 🔥 */

/**
 * vexfs_register_enhanced_filesystem - Register enhanced filesystem
 * 
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_register_enhanced_filesystem(void)
{
    int ret;
    
    printk(KERN_INFO "VexFS v2.0: Registering enhanced filesystem\n");
    
    /* Check minimum system requirements */
    if (!vexfs_check_minimum_requirements()) {
        printk(KERN_ERR "VexFS v2.0: Minimum system requirements not met\n");
        return -ENODEV;
    }
    
    /* Check kernel version compatibility */
    if (!vexfs_check_kernel_version_compatibility()) {
        printk(KERN_ERR "VexFS v2.0: Kernel version not compatible\n");
        return -ENODEV;
    }
    
    /* Register the filesystem */
    ret = register_filesystem(&vexfs_v2_enhanced_fs_type);
    if (ret) {
        printk(KERN_ERR "VexFS v2.0: Failed to register enhanced filesystem: %d\n", ret);
        return ret;
    }
    
    printk(KERN_INFO "VexFS v2.0: Enhanced filesystem registered successfully\n");
    return 0;
}

/**
 * vexfs_unregister_enhanced_filesystem - Unregister enhanced filesystem
 */
void vexfs_unregister_enhanced_filesystem(void)
{
    printk(KERN_INFO "VexFS v2.0: Unregistering enhanced filesystem\n");
    
    unregister_filesystem(&vexfs_v2_enhanced_fs_type);
    
    printk(KERN_INFO "VexFS v2.0: Enhanced filesystem unregistered\n");
}

/* 🔥 SYSTEM REQUIREMENT CHECKING 🔥 */

/**
 * vexfs_check_minimum_requirements - Check minimum system requirements
 * 
 * Returns: true if requirements met, false otherwise
 */
bool vexfs_check_minimum_requirements(void)
{
    /* Check for basic CPU features */
    if (!boot_cpu_has(X86_FEATURE_FPU)) {
        printk(KERN_ERR "VexFS v2.0: FPU support required\n");
        return false;
    }
    
    /* Check for minimum memory */
    if (totalram_pages() < (64 * 1024 * 1024 / PAGE_SIZE)) { /* 64MB minimum */
        printk(KERN_ERR "VexFS v2.0: Insufficient memory (minimum 64MB required)\n");
        return false;
    }
    
    printk(KERN_INFO "VexFS v2.0: Minimum system requirements met\n");
    return true;
}

/**
 * vexfs_check_kernel_version_compatibility - Check kernel version compatibility
 * 
 * Returns: true if compatible, false otherwise
 */
bool vexfs_check_kernel_version_compatibility(void)
{
    /* VexFS v2.0 requires Linux 4.4+ for modern VFS features */
    if (LINUX_VERSION_CODE < KERNEL_VERSION(4, 4, 0)) {
        printk(KERN_ERR "VexFS v2.0: Kernel version 4.4+ required\n");
        return false;
    }
    
    printk(KERN_INFO "VexFS v2.0: Kernel version compatible\n");
    return true;
}

/**
 * vexfs_check_cpu_features - Check required CPU features
 * @required_features: Required CPU feature mask
 * 
 * Returns: true if features available, false otherwise
 */
bool vexfs_check_cpu_features(u32 required_features)
{
    if (required_features & VEXFS_SIMD_SSE2) {
        if (!boot_cpu_has(X86_FEATURE_XMM2)) {
            printk(KERN_ERR "VexFS v2.0: SSE2 support required but not available\n");
            return false;
        }
    }
    
    if (required_features & VEXFS_SIMD_AVX2) {
        if (!boot_cpu_has(X86_FEATURE_AVX2)) {
            printk(KERN_ERR "VexFS v2.0: AVX2 support required but not available\n");
            return false;
        }
    }
    
    if (required_features & VEXFS_SIMD_AVX512) {
        if (!boot_cpu_has(X86_FEATURE_AVX512F)) {
            printk(KERN_ERR "VexFS v2.0: AVX-512 support required but not available\n");
            return false;
        }
    }
    
    return true;
}

/* 🔥 DEBUG AND MONITORING FUNCTIONS 🔥 */

/**
 * vexfs_print_compatibility_status - Print volume compatibility status
 * @sb: Super block
 */
void vexfs_print_compatibility_status(struct super_block *sb)
{
    struct vexfs_v2_sb_info *sbi;
    
    if (!sb)
        return;
    
    sbi = VEXFS_V2_SB(sb);
    if (!sbi)
        return;
    
    printk(KERN_INFO "VexFS v2.0: Volume compatibility status:\n");
    printk(KERN_INFO "  Vector dimensions: %u (max: %u)\n",
           sbi->default_vector_dim, sbi->max_vector_dim);
    printk(KERN_INFO "  Element type: %s\n",
           vexfs_element_type_to_string(sbi->default_element_type));
    printk(KERN_INFO "  Vector alignment: %u bytes\n", sbi->vector_alignment);
    printk(KERN_INFO "  SIMD capabilities: 0x%x (%u-bit vectors)\n",
           sbi->simd_capabilities, sbi->simd_vector_width);
    printk(KERN_INFO "  NUMA awareness: %s\n",
           sbi->numa_aware ? "enabled" : "disabled");
    printk(KERN_INFO "  Vector operations: %s\n",
           sbi->vector_ops_registered ? "registered" : "not registered");
}

/**
 * vexfs_show_mount_options - Show mount options in /proc/mounts
 * @seq: Sequence file
 * @dentry: Dentry
 * 
 * Returns: 0 on success
 */
int vexfs_show_mount_options(struct seq_file *seq, struct dentry *dentry)
{
    struct super_block *sb;
    struct vexfs_v2_sb_info *sbi;
    
    if (!seq || !dentry)
        return 0;
    
    sb = dentry->d_sb;
    if (!sb)
        return 0;
    
    sbi = VEXFS_V2_SB(sb);
    if (!sbi)
        return 0;
    
    /* Show current mount options */
    seq_printf(seq, ",max_vector_dim=%u", sbi->max_vector_dim);
    seq_printf(seq, ",default_element_type=%s",
               vexfs_element_type_to_string(sbi->default_element_type));
    seq_printf(seq, ",vector_alignment=%u", sbi->vector_alignment);
    seq_printf(seq, ",batch_size=%u", sbi->batch_size);
    
    if (sbi->simd_capabilities == 0)
        seq_puts(seq, ",disable_simd");
    else
        seq_printf(seq, ",simd_capabilities=0x%x", sbi->simd_capabilities);
    
    if (sbi->numa_aware)
        seq_puts(seq, ",numa_aware=yes");
    else
        seq_puts(seq, ",numa_aware=no");
    
    if (sb->s_flags & SB_RDONLY)
        seq_puts(seq, ",readonly");
    
    if (sbi->debug_level > 0)
        seq_printf(seq, ",debug_level=%u", sbi->debug_level);
    
    return 0;
}