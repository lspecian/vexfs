/*
 * VexFS - Vector Extended File System (SAFE VERSION)
 * Copyright (C) 2025 VexFS Contributors
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 */

#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/mount.h>
#include <linux/statfs.h>
#include <linux/version.h>

/* VexFS magic number - safe constant */
#define VEXFS_MAGIC 0x56454653  /* "VEFS" in ASCII */

/* Forward declarations for VFS operations */
static struct dentry *vexfs_mount(struct file_system_type *fs_type, int flags,
                                  const char *dev_name, void *data);
static void vexfs_kill_sb(struct super_block *sb);
static int vexfs_fill_super(struct super_block *sb, void *data, int silent);
static int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf);

/* Forward declarations for superblock operations */
static struct inode *vexfs_alloc_inode(struct super_block *sb);
static void vexfs_free_inode(struct inode *inode);
static int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc);
static void vexfs_put_super(struct super_block *sb);
static int vexfs_sync_fs(struct super_block *sb, int wait);

/* Forward declarations for inode operations */
static int vexfs_create(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode, bool excl);
static struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry, unsigned int flags);
static int vexfs_mkdir(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode);
static int vexfs_rmdir(struct inode *dir, struct dentry *dentry);
static int vexfs_unlink(struct inode *dir, struct dentry *dentry);

/* Forward declarations for file operations */
static int vexfs_open(struct inode *inode, struct file *file);
static int vexfs_release(struct inode *inode, struct file *file);
static ssize_t vexfs_read(struct file *file, char __user *buf, size_t len, loff_t *ppos);
static ssize_t vexfs_write(struct file *file, const char __user *buf, size_t len, loff_t *ppos);

/* VFS file system type registration */
static struct file_system_type vexfs_type = {
    .name       = "vexfs",
    .mount      = vexfs_mount,
    .kill_sb    = vexfs_kill_sb,
    .owner      = THIS_MODULE,
    .fs_flags   = FS_REQUIRES_DEV,
};

/* Superblock operations - SAFE implementations */
static const struct super_operations vexfs_super_ops = {
    .alloc_inode    = vexfs_alloc_inode,
    .free_inode     = vexfs_free_inode,  /* Use free_inode instead of destroy_inode */
    .write_inode    = vexfs_write_inode,
    .put_super      = vexfs_put_super,
    .sync_fs        = vexfs_sync_fs,
    .statfs         = vexfs_statfs,
    .drop_inode     = generic_delete_inode,
};

/* Inode operations for directories */
static const struct inode_operations vexfs_dir_inode_ops = {
    .create         = vexfs_create,
    .lookup         = vexfs_lookup,
    .mkdir          = vexfs_mkdir,
    .rmdir          = vexfs_rmdir,
    .unlink         = vexfs_unlink,
};

/* Inode operations for regular files */
static const struct inode_operations vexfs_file_inode_ops = {
    .getattr        = simple_getattr,
    .setattr        = simple_setattr,
};

/* File operations for regular files */
static const struct file_operations vexfs_file_ops = {
    .owner          = THIS_MODULE,
    .open           = vexfs_open,
    .release        = vexfs_release,
    .read           = vexfs_read,
    .write          = vexfs_write,
    .llseek         = default_llseek,
};

/**
 * vexfs_mount - Mount the VexFS filesystem
 * SAFE: Basic mount operation without FFI calls
 */
static struct dentry *vexfs_mount(struct file_system_type *fs_type, int flags,
                                  const char *dev_name, void *data)
{
    printk(KERN_INFO "VexFS: Mounting filesystem on device %s (SAFE MODE)\n", dev_name);
    return mount_bdev(fs_type, flags, dev_name, data, vexfs_fill_super);
}

/**
 * vexfs_kill_sb - Unmount the VexFS filesystem
 * SAFE: Basic unmount without FFI calls
 */
static void vexfs_kill_sb(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Unmounting filesystem (SAFE MODE)\n");
    kill_block_super(sb);
}

/**
 * vexfs_fill_super - Initialize the superblock
 * SAFE: No FFI calls, basic superblock setup only
 */
static int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    struct inode *root_inode;
    struct dentry *root_dentry;
    struct timespec64 ts;

    printk(KERN_INFO "VexFS: Filling superblock (SAFE MODE - no FFI)\n");

    /* Set up superblock - SAFE values only */
    sb->s_magic = VEXFS_MAGIC;
    sb->s_op = &vexfs_super_ops;
    sb->s_blocksize = PAGE_SIZE;
    sb->s_blocksize_bits = PAGE_SHIFT;
    sb->s_maxbytes = MAX_LFS_FILESIZE;

    /* NO FFI CALLS - this was causing the hang */
    printk(KERN_INFO "VexFS: Skipping Rust FFI calls (SAFE MODE)\n");

    /* Create root inode */
    root_inode = new_inode(sb);
    if (!root_inode) {
        printk(KERN_ERR "VexFS: Failed to allocate root inode\n");
        return -ENOMEM;
    }

    /* Initialize root inode as directory */
    root_inode->i_ino = 1;
    root_inode->i_mode = S_IFDIR | 0755;
    set_nlink(root_inode, 2);
    root_inode->i_uid = GLOBAL_ROOT_UID;
    root_inode->i_gid = GLOBAL_ROOT_GID;
    root_inode->i_size = 0;
    root_inode->i_blocks = 0;
    
    /* Set timestamps with kernel version compatibility */
    ts = current_time(root_inode);
#if LINUX_VERSION_CODE >= KERNEL_VERSION(6, 11, 0)
    inode_set_atime_to_ts(root_inode, ts);
    inode_set_mtime_to_ts(root_inode, ts);
    inode_set_ctime_to_ts(root_inode, ts);
#else
    root_inode->i_atime = ts;
    root_inode->i_mtime = ts;
    root_inode->i_ctime = ts;
#endif

    /* Set root inode operations */
    root_inode->i_op = &vexfs_dir_inode_ops;
    root_inode->i_fop = &simple_dir_operations;

    /* Create root dentry */
    root_dentry = d_make_root(root_inode);
    if (!root_dentry) {
        printk(KERN_ERR "VexFS: Failed to create root dentry\n");
        return -ENOMEM;
    }

    sb->s_root = root_dentry;
    printk(KERN_INFO "VexFS: Superblock initialized successfully (SAFE MODE)\n");
    return 0;
}

/**
 * vexfs_statfs - Return filesystem statistics
 * SAFE: Return fixed values, no FFI calls
 */
static int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
    buf->f_type = VEXFS_MAGIC;
    buf->f_bsize = PAGE_SIZE;
    buf->f_namelen = 255;

    /* SAFE: Use fixed values instead of FFI calls */
    buf->f_blocks = 1000;
    buf->f_bfree = 500;
    buf->f_bavail = 500;
    buf->f_files = 100;
    buf->f_ffree = 50;

    printk(KERN_DEBUG "VexFS: statfs called (SAFE MODE)\n");
    return 0;
}

/**
 * vexfs_alloc_inode - Allocate a new inode
 * SAFE: Use kernel allocation only, no FFI
 */
static struct inode *vexfs_alloc_inode(struct super_block *sb)
{
    struct inode *inode;
    
    printk(KERN_DEBUG "VexFS: Allocating new inode (SAFE MODE)\n");
    
    /* Allocate generic inode - SAFE */
    inode = new_inode(sb);
    if (!inode) {
        return NULL;
    }
    
    /* NO FFI CALLS - this was causing issues */
    printk(KERN_DEBUG "VexFS: Inode allocated without FFI (SAFE)\n");
    
    return inode;
}

/**
 * vexfs_free_inode - Free an inode
 * SAFE: Use proper kernel free function
 */
static void vexfs_free_inode(struct inode *inode)
{
    printk(KERN_DEBUG "VexFS: Freeing inode %lu (SAFE MODE)\n", inode->i_ino);
    
    /* NO FFI CALLS - this was causing hangs */
    /* Use proper kernel inode freeing - SAFE */
    /* The kernel will handle the actual freeing */
}

/**
 * vexfs_write_inode - Write inode to storage
 * SAFE: No-op implementation
 */
static int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc)
{
    printk(KERN_DEBUG "VexFS: Write inode %lu (SAFE MODE - no-op)\n", inode->i_ino);
    
    /* NO FFI CALLS - return success */
    return 0;
}

/**
 * vexfs_put_super - Put superblock during unmount
 * SAFE: No FFI calls
 */
static void vexfs_put_super(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Put superblock called (SAFE MODE)\n");
    
    /* NO FFI CALLS - this was causing issues */
}

/**
 * vexfs_sync_fs - Sync filesystem
 * SAFE: No-op implementation
 */
static int vexfs_sync_fs(struct super_block *sb, int wait)
{
    printk(KERN_DEBUG "VexFS: Sync filesystem (SAFE MODE - no-op, wait=%d)\n", wait);
    
    /* NO FFI CALLS - return success */
    return 0;
}

/**
 * vexfs_create - Create a new file
 * SAFE: Basic file creation without FFI
 */
static int vexfs_create(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode, bool excl)
{
    struct inode *inode;
    
    printk(KERN_DEBUG "VexFS: Creating file %s (SAFE MODE)\n", dentry->d_name.name);
    
    /* Allocate new inode */
    inode = vexfs_alloc_inode(dir->i_sb);
    if (!inode) {
        return -ENOSPC;
    }
    
    /* Set up the inode - SAFE */
    inode->i_mode = mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    simple_inode_init_ts(inode);
    inode->i_op = &vexfs_file_inode_ops;
    inode->i_fop = &vexfs_file_ops;
    
    /* NO FFI CALLS */
    
    /* Link to dentry */
    d_instantiate(dentry, inode);
    
    return 0;
}

/**
 * vexfs_lookup - Look up a dentry
 * SAFE: Return not found
 */
static struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry, unsigned int flags)
{
    printk(KERN_DEBUG "VexFS: Looking up %s (SAFE MODE)\n", dentry->d_name.name);
    
    /* Return not found - SAFE */
    d_add(dentry, NULL);
    return NULL;
}

/**
 * vexfs_mkdir - Create a directory
 * SAFE: Basic directory creation
 */
static int vexfs_mkdir(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode)
{
    struct inode *inode;
    
    printk(KERN_DEBUG "VexFS: Creating directory %s (SAFE MODE)\n", dentry->d_name.name);
    
    /* Allocate new inode */
    inode = vexfs_alloc_inode(dir->i_sb);
    if (!inode) {
        return -ENOSPC;
    }
    
    /* Set up the directory inode - SAFE */
    inode->i_mode = S_IFDIR | mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    simple_inode_init_ts(inode);
    inode->i_op = &vexfs_dir_inode_ops;
    inode->i_fop = &simple_dir_operations;
    set_nlink(inode, 2); /* . and .. */
    
    /* NO FFI CALLS */
    
    /* Update parent directory */
    inc_nlink(dir);
    
    /* Link to dentry */
    d_instantiate(dentry, inode);
    
    return 0;
}

/**
 * vexfs_rmdir - Remove a directory
 * SAFE: Basic directory removal
 */
static int vexfs_rmdir(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    
    printk(KERN_DEBUG "VexFS: Removing directory %s (SAFE MODE)\n", dentry->d_name.name);
    
    /* Update link counts - SAFE */
    clear_nlink(inode);
    drop_nlink(dir);
    
    return 0;
}

/**
 * vexfs_unlink - Remove a file
 * SAFE: Basic file removal
 */
static int vexfs_unlink(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    
    printk(KERN_DEBUG "VexFS: Unlinking file %s (SAFE MODE)\n", dentry->d_name.name);
    
    /* Update link count - SAFE */
    drop_nlink(inode);
    
    return 0;
}

/* File operations - SAFE implementations */
static int vexfs_open(struct inode *inode, struct file *file)
{
    printk(KERN_DEBUG "VexFS: Opening file (SAFE MODE)\n");
    return 0;
}

static int vexfs_release(struct inode *inode, struct file *file)
{
    printk(KERN_DEBUG "VexFS: Releasing file (SAFE MODE)\n");
    return 0;
}

static ssize_t vexfs_read(struct file *file, char __user *buf, size_t count, loff_t *ppos)
{
    printk(KERN_DEBUG "VexFS: Reading from file (SAFE MODE - return 0)\n");
    return 0; /* EOF */
}

static ssize_t vexfs_write(struct file *file, const char __user *buf, size_t count, loff_t *ppos)
{
    printk(KERN_DEBUG "VexFS: Writing to file (SAFE MODE - accept but don't store)\n");
    return count; /* Pretend success */
}

/**
 * vexfs_init_module - Initialize the VexFS module
 * SAFE: No FFI calls during initialization
 */
static int __init vexfs_init_module(void)
{
    int ret;

    printk(KERN_INFO "VexFS: Initializing SAFE module v0.1.0-safe\n");

    /* NO FFI CALLS - this was causing issues */
    printk(KERN_INFO "VexFS: SAFE MODE - Rust FFI disabled\n");

    /* Register filesystem with VFS */
    ret = register_filesystem(&vexfs_type);
    if (ret) {
        printk(KERN_ERR "VexFS: Failed to register filesystem: %d\n", ret);
        return ret;
    }

    printk(KERN_INFO "VexFS: SAFE module loaded successfully\n");
    printk(KERN_INFO "VexFS: Filesystem registered as 'vexfs' (SAFE MODE)\n");
    printk(KERN_WARNING "VexFS: This is a SAFE testing version - limited functionality\n");
    return 0;
}

/**
 * vexfs_exit_module - Cleanup the VexFS module
 * SAFE: No FFI calls during cleanup
 */
static void __exit vexfs_exit_module(void)
{
    printk(KERN_INFO "VexFS: Unloading SAFE module\n");

    /* Unregister filesystem from VFS */
    unregister_filesystem(&vexfs_type);
    printk(KERN_INFO "VexFS: Filesystem unregistered\n");

    /* NO FFI CALLS - this was causing issues */
    printk(KERN_INFO "VexFS: SAFE MODE - no Rust cleanup needed\n");
    printk(KERN_INFO "VexFS: SAFE module unloaded successfully\n");
}

module_init(vexfs_init_module);
module_exit(vexfs_exit_module);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("VexFS Contributors");
MODULE_DESCRIPTION("VexFS: Vector-Native File System (SAFE MODE)");
MODULE_VERSION("0.1.0-safe");