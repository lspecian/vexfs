/*
 * VexFS - Vector Extended File System
 * Copyright (C) 2025 VexFS Contributors
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/mount.h>
#include <linux/statfs.h>
#include <linux/version.h>

/* Include the FFI header generated from Rust (if available) */
#ifdef VEXFS_RUST_FFI_ENABLED
#include "vexfs_ffi.h"
#else
/* Define constants needed for C-only build */
#define VEXFS_MAGIC 0x56454653  /* "VEFS" in ASCII */
#endif

/* Forward declarations for VFS operations */
static struct dentry *vexfs_mount(struct file_system_type *fs_type, int flags,
                                  const char *dev_name, void *data);
static void vexfs_kill_sb(struct super_block *sb);
static int vexfs_fill_super(struct super_block *sb, void *data, int silent);
static int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf);

/* Forward declarations for superblock operations */
static struct inode *vexfs_alloc_inode(struct super_block *sb);
static void vexfs_destroy_inode(struct inode *inode);
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

/* Forward declaration for RCU callback */
static void vexfs_free_inode_rcu(struct rcu_head *head);

/* VFS file system type registration */
static struct file_system_type vexfs_type = {
    .name       = "vexfs",
    .mount      = vexfs_mount,
    .kill_sb    = vexfs_kill_sb,
    .owner      = THIS_MODULE,
    .fs_flags   = FS_REQUIRES_DEV,
};

/* Superblock operations */
static const struct super_operations vexfs_super_ops = {
    .alloc_inode    = vexfs_alloc_inode,
    .destroy_inode  = vexfs_destroy_inode,
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

/* Include FFI header for Rust library functions */
/* vexfs_rust_ffi.h removed - using cbindgen-generated header only */

/**
 * vexfs_mount - Mount the VexFS filesystem
 */
static struct dentry *vexfs_mount(struct file_system_type *fs_type, int flags,
                                  const char *dev_name, void *data)
{
    printk(KERN_INFO "VexFS: Mounting filesystem on device %s\n", dev_name);
    return mount_bdev(fs_type, flags, dev_name, data, vexfs_fill_super);
}

/**
 * vexfs_kill_sb - Unmount the VexFS filesystem
 */
static void vexfs_kill_sb(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Unmounting filesystem\n");
    kill_block_super(sb);
}

/**
 * vexfs_fill_super - Initialize the superblock
 */
static int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    struct inode *root_inode;
    struct dentry *root_dentry;
    struct timespec64 ts;

    printk(KERN_INFO "VexFS: Filling superblock\n");

    /* Set up superblock */
    sb->s_magic = VEXFS_MAGIC;
    sb->s_op = &vexfs_super_ops;
    sb->s_blocksize = PAGE_SIZE;
    sb->s_blocksize_bits = PAGE_SHIFT;
    sb->s_maxbytes = MAX_LFS_FILESIZE;

    /* Initialize Rust superblock components (if available) */
#ifdef VEXFS_RUST_FFI_ENABLED
    {
        int ret = vexfs_rust_fill_super(sb);
        if (ret) {
            if (!silent)
                printk(KERN_ERR "VexFS: Failed to initialize superblock (Rust): %d\n", ret);
            return ret;
        }
        printk(KERN_INFO "VexFS: Rust superblock components initialized\n");
    }
#else
    printk(KERN_INFO "VexFS: C-only build - Rust components disabled\n");
#endif

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
    printk(KERN_INFO "VexFS: Superblock initialized successfully\n");
    return 0;
}

/**
 * vexfs_statfs - Return filesystem statistics
 */
static int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
    /* Declare variables at beginning of function for C90 compatibility */
#ifdef VEXFS_RUST_FFI_ENABLED
    int ret;
    uint64_t blocks = 0, free_blocks = 0, files = 0, free_files = 0;
#endif
    
    buf->f_type = VEXFS_MAGIC;
    buf->f_bsize = PAGE_SIZE;
    buf->f_namelen = 255;

    /* Get filesystem statistics from Rust implementation (if available) */
#ifdef VEXFS_RUST_FFI_ENABLED
    
    ret = vexfs_rust_get_statfs(&blocks, &free_blocks, &files, &free_files);
    if (ret) {
        /* Use default values on error */
        printk(KERN_WARNING "VexFS: Failed to get statfs from Rust: %d\n", ret);
        buf->f_blocks = 1000;
        buf->f_bfree = 500;
        buf->f_bavail = 500;
        buf->f_files = 100;
        buf->f_ffree = 50;
    } else {
        buf->f_blocks = blocks;
        buf->f_bfree = free_blocks;
        buf->f_bavail = free_blocks;
        buf->f_files = files;
        buf->f_ffree = free_files;
    }
#else
    /* C-only build: Use default values */
    buf->f_blocks = 1000;
    buf->f_bfree = 500;
    buf->f_bavail = 500;
    buf->f_files = 100;
    buf->f_ffree = 50;
#endif

    return 0;
}

/**
 * vexfs_alloc_inode - Allocate a new inode
 */
static struct inode *vexfs_alloc_inode(struct super_block *sb)
{
    struct inode *inode;
    
    printk(KERN_DEBUG "VexFS: Allocating new inode\n");
    
    /* Allocate generic inode */
    inode = new_inode(sb);
    if (!inode) {
        return NULL;
    }
    
    /* Call Rust layer to initialize VexFS-specific data */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_new_inode) {
        void *result = vexfs_rust_new_inode(sb, inode->i_ino, inode->i_mode);
        if (!result) {
            iput(inode);
            return NULL;
        }
    }
#endif
    
    return inode;
}

/**
 * RCU callback for inode cleanup (if needed for older kernel compatibility)
 */
static void vexfs_free_inode_rcu(struct rcu_head *head)
{
    struct inode *inode = container_of(head, struct inode, i_rcu);
    /* Free the inode using the generic implementation */
    free_inode_nonrcu(inode);
}

/**
 * vexfs_destroy_inode - Destroy an inode
 */
static void vexfs_destroy_inode(struct inode *inode)
{
    printk(KERN_DEBUG "VexFS: Destroying inode %lu\n", inode->i_ino);
    
    /* Call Rust layer to cleanup VexFS-specific data */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_destroy_inode) {
        vexfs_rust_destroy_inode(inode);
    }
#endif
    
    /* For modern kernels, use call_rcu instead of __free_inode */
    call_rcu(&inode->i_rcu, vexfs_free_inode_rcu);
}

/**
 * vexfs_write_inode - Write inode to storage
 */
static int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc)
{
    int ret = 0;
    
    printk(KERN_DEBUG "VexFS: Writing inode %lu\n", inode->i_ino);
    
    /* Call Rust layer to handle inode persistence */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_write_inode) {
        ret = vexfs_rust_write_inode(inode);
    }
#endif
    
    return ret;
}

/**
 * vexfs_put_super - Put superblock during unmount
 */
static void vexfs_put_super(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Put superblock called\n");
    
    /* Call Rust layer for cleanup */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_put_super) {
        vexfs_rust_put_super(sb);
    }
#endif
}

/**
 * vexfs_sync_fs - Sync filesystem
 */
static int vexfs_sync_fs(struct super_block *sb, int wait)
{
    int ret = 0;
    
    printk(KERN_DEBUG "VexFS: Sync filesystem (wait=%d)\n", wait);
    
    /* Call Rust layer to handle synchronization */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_sync_fs) {
        ret = vexfs_rust_sync_fs(sb, wait);
    }
#endif
    
    return ret;
}

/**
 * vexfs_create - Create a new file
 */
static int vexfs_create(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode, bool excl)
{
    struct inode *inode;
    int ret = 0;
    
    printk(KERN_DEBUG "VexFS: Creating file %s in dir %lu\n",
           dentry->d_name.name, dir->i_ino);
    
    /* Allocate new inode */
    inode = vexfs_alloc_inode(dir->i_sb);
    if (!inode) {
        return -ENOSPC;
    }
    
    /* Set up the inode */
    inode->i_mode = mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    /* Use simple_inode_init_ts for modern kernels */
    simple_inode_init_ts(inode);
    inode->i_op = &vexfs_file_inode_ops;
    inode->i_fop = &vexfs_file_ops;
    
    /* Initialize VexFS-specific data */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_init_inode) {
        ret = vexfs_rust_init_inode(inode, inode->i_ino, mode);
        if (ret != 0) {
            iput(inode);
            return ret;
        }
    }
#endif
    
    /* Link to dentry */
    d_instantiate(dentry, inode);
    
    return 0;
}

/**
 * vexfs_lookup - Look up a dentry
 */
static struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry, unsigned int flags)
{
    printk(KERN_DEBUG "VexFS: Looking up %s in dir %lu\n",
           dentry->d_name.name, dir->i_ino);
    
    /* For now, return NULL (not found) - will implement proper lookup later */
    d_add(dentry, NULL);
    return NULL;
}

/**
 * vexfs_mkdir - Create a directory
 */
static int vexfs_mkdir(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode)
{
    struct inode *inode;
    int ret = 0;
    
    printk(KERN_DEBUG "VexFS: Creating directory %s in dir %lu\n",
           dentry->d_name.name, dir->i_ino);
    
    /* Allocate new inode */
    inode = vexfs_alloc_inode(dir->i_sb);
    if (!inode) {
        return -ENOSPC;
    }
    
    /* Set up the directory inode */
    inode->i_mode = S_IFDIR | mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    /* Use simple_inode_init_ts for modern kernels */
    simple_inode_init_ts(inode);
    inode->i_op = &vexfs_dir_inode_ops;
    inode->i_fop = &simple_dir_operations;
    set_nlink(inode, 2); /* . and .. */
    
    /* Initialize VexFS-specific data */
#ifdef VEXFS_RUST_FFI_ENABLED
    if (vexfs_rust_init_inode) {
        ret = vexfs_rust_init_inode(inode, inode->i_ino, inode->i_mode);
        if (ret != 0) {
            iput(inode);
            return ret;
        }
    }
#endif
    
    /* Update parent directory */
    inc_nlink(dir);
    
    /* Link to dentry */
    d_instantiate(dentry, inode);
    
    return 0;
}

/**
 * vexfs_rmdir - Remove a directory
 */
static int vexfs_rmdir(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    
    printk(KERN_DEBUG "VexFS: Removing directory %s from dir %lu\n",
           dentry->d_name.name, dir->i_ino);
    
    /* TODO: Check if directory is empty */
    
    /* Update link counts */
    clear_nlink(inode);
    drop_nlink(dir);
    
    return 0;
}

/**
 * vexfs_unlink - Remove a file
 */
static int vexfs_unlink(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    
    printk(KERN_DEBUG "VexFS: Unlinking file %s from dir %lu\n",
           dentry->d_name.name, dir->i_ino);
    
    /* Update link count */
    drop_nlink(inode);
    
    return 0;
}

/* File operations */
static int vexfs_open(struct inode *inode, struct file *file)
{
    printk(KERN_DEBUG "VexFS: Opening file (inode %lu)\n", inode->i_ino);
    return 0;
}

static int vexfs_release(struct inode *inode, struct file *file)
{
    printk(KERN_DEBUG "VexFS: Releasing file (inode %lu)\n", inode->i_ino);
    return 0;
}

static ssize_t vexfs_read(struct file *file, char __user *buf, size_t count, loff_t *ppos)
{
    printk(KERN_DEBUG "VexFS: Reading from file\n");
    return 0; /* Temporary stub */
}

static ssize_t vexfs_write(struct file *file, const char __user *buf, size_t count, loff_t *ppos)
{
    printk(KERN_DEBUG "VexFS: Writing to file\n");
    return count; /* Temporary stub */
}

static loff_t vexfs_llseek(struct file *file, loff_t offset, int whence)
{
    return generic_file_llseek(file, offset, whence);
}


/**
 * vexfs_init_module - Initialize the VexFS module
 */
static int __init vexfs_init_module(void)
{
    int ret;

    printk(KERN_INFO "VexFS: Initializing module v0.1.0\n");

    /* Initialize Rust components (if available) */
#ifdef VEXFS_RUST_FFI_ENABLED
    ret = vexfs_rust_init();
    if (ret) {
        printk(KERN_ERR "VexFS: Failed to initialize Rust components: %d\n", ret);
        return ret;
    }
    printk(KERN_INFO "VexFS: Rust components initialized successfully\n");

    /* Test basic FFI functionality */
    ret = vexfs_rust_test_basic();
    if (ret) {
        printk(KERN_WARNING "VexFS: Basic FFI test failed: %d\n", ret);
    } else {
        printk(KERN_INFO "VexFS: Basic FFI test passed\n");
    }

    /* Test vector operations FFI */
    ret = vexfs_rust_test_vector_ops();
    if (ret) {
        printk(KERN_WARNING "VexFS: Vector ops FFI test failed: %d\n", ret);
    } else {
        printk(KERN_INFO "VexFS: Vector ops FFI test passed\n");
    }

    /* Get and display version */
    ret = vexfs_rust_get_version();
    printk(KERN_INFO "VexFS: Rust library version: 0x%08x\n", ret);
#else
    printk(KERN_INFO "VexFS: C-only build - Rust components disabled\n");
#endif

    /* Register filesystem with VFS */
    ret = register_filesystem(&vexfs_type);
    if (ret) {
        printk(KERN_ERR "VexFS: Failed to register filesystem: %d\n", ret);
#ifdef VEXFS_RUST_FFI_ENABLED
        vexfs_rust_exit();
#endif
        return ret;
    }

    printk(KERN_INFO "VexFS: Module loaded successfully\n");
    printk(KERN_INFO "VexFS: Filesystem registered as 'vexfs'\n");
    return 0;
}

/**
 * vexfs_exit_module - Cleanup the VexFS module
 */
static void __exit vexfs_exit_module(void)
{
    printk(KERN_INFO "VexFS: Unloading module\n");

    /* Unregister filesystem from VFS */
    unregister_filesystem(&vexfs_type);
    printk(KERN_INFO "VexFS: Filesystem unregistered\n");

    /* Cleanup Rust components (if available) */
#ifdef VEXFS_RUST_FFI_ENABLED
    vexfs_rust_exit();
    printk(KERN_INFO "VexFS: Rust components cleaned up\n");
#else
    printk(KERN_INFO "VexFS: C-only build - no Rust cleanup needed\n");
#endif
    printk(KERN_INFO "VexFS: Module unloaded successfully\n");
}

module_init(vexfs_init_module);
module_exit(vexfs_exit_module);

// Rust runtime symbols needed for kernel modules
void rust_eh_personality(void) {
    // Empty implementation for kernel - panic handling is different
}

MODULE_LICENSE("GPL");
MODULE_AUTHOR("VexFS Contributors");
MODULE_DESCRIPTION("VexFS: Vector-Native File System");
MODULE_VERSION("0.1.0");
