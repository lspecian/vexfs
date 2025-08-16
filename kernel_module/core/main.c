/*
 * VexFS v2.0 - Main Module Entry Point
 * 
 * This file contains the main module initialization and cleanup functions,
 * following Linux kernel filesystem patterns.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/fs.h>
#include <linux/slab.h>

#include "../include/vexfs_core.h"
#include "../include/vexfs_semantic.h"

/* Module information */
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS v2.0 - Vector Filesystem with Semantic Search (Fixed I/O Lists)");
MODULE_LICENSE("GPL");
MODULE_VERSION("2.0.0-fix-io-lists-b8e4c3d9");

/* Forward declarations */
static struct dentry *vexfs_mount(struct file_system_type *fs_type,
                                 int flags, const char *dev_name, void *data);
static void vexfs_kill_sb(struct super_block *sb);

/* Function prototypes from superblock.c */
extern int vexfs_fill_super(struct super_block *sb, void *data, int silent);
extern struct kmem_cache *vexfs_inode_cachep;

/**
 * vexfs_mount - Mount a VexFS filesystem
 * @fs_type: Filesystem type
 * @flags: Mount flags
 * @dev_name: Device name
 * @data: Mount data
 *
 * Returns: Dentry on success, error pointer on failure
 */
static struct dentry *vexfs_mount(struct file_system_type *fs_type,
                                 int flags, const char *dev_name, void *data)
{
    return mount_bdev(fs_type, flags, dev_name, data, vexfs_fill_super);
}

/**
 * vexfs_kill_sb - Unmount a VexFS filesystem
 * @sb: Superblock to unmount
 *
 * Note: Cleanup is now handled by vexfs_put_super() which is called
 * by kill_block_super() before destroying the superblock.
 */
static void vexfs_kill_sb(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Starting filesystem unmount\n");
    
    /* Check if sb is valid */
    if (!sb) {
        printk(KERN_WARNING "VexFS: kill_sb called with NULL sb\n");
        return;
    }
    
    /* Only sync if filesystem was successfully mounted */
    if (sb->s_fs_info) {
        /* Ensure all pending I/O is complete */
        sync_filesystem(sb);
    }
    
    /* Ensure proper cleanup */
    sb->s_flags |= SB_ACTIVE;
    
    /* kill_block_super() will call put_super() for cleanup */
    kill_block_super(sb);
    
    printk(KERN_INFO "VexFS: Filesystem unmount completed\n");
}

/* Filesystem type structure */
static struct file_system_type vexfs_fs_type = {
    .owner      = THIS_MODULE,
    .name       = "vexfs_fixed",
    .mount      = vexfs_mount,
    .kill_sb    = vexfs_kill_sb,
    .fs_flags   = FS_REQUIRES_DEV,
};

/**
 * vexfs_init - Initialize VexFS module
 *
 * Returns: 0 on success, negative error code on failure
 */
static int __init vexfs_init(void)
{
    int ret;
    
    printk(KERN_INFO "VexFS v2.0-FIXED: Initializing vector filesystem with inode lifecycle fixes\n");
    
    /* Initialize inode cache */
    vexfs_inode_cachep = kmem_cache_create("vexfs_fixed_inode_cache",
                                          sizeof(struct vexfs_inode_info),
                                          0,
                                          SLAB_RECLAIM_ACCOUNT,
                                          NULL);
    if (!vexfs_inode_cachep) {
        printk(KERN_ERR "VexFS: Failed to create inode cache\n");
        return -ENOMEM;
    }
    
    /* Register filesystem type */
    ret = register_filesystem(&vexfs_fs_type);
    if (ret) {
        printk(KERN_ERR "VexFS: Failed to register filesystem: %d\n", ret);
        kmem_cache_destroy(vexfs_inode_cachep);
        return ret;
    }
    
    printk(KERN_INFO "VexFS v2.0: Successfully registered filesystem\n");
    return 0;
}

/**
 * vexfs_exit - Cleanup VexFS module
 */
static void __exit vexfs_exit(void)
{
    printk(KERN_INFO "VexFS v2.0: Unregistering vector filesystem\n");
    
    /* Unregister filesystem type */
    unregister_filesystem(&vexfs_fs_type);
    
    /* Destroy inode cache */
    if (vexfs_inode_cachep) {
        kmem_cache_destroy(vexfs_inode_cachep);
    }
    
    printk(KERN_INFO "VexFS v2.0: Successfully unregistered filesystem\n");
}

module_init(vexfs_init);
module_exit(vexfs_exit);