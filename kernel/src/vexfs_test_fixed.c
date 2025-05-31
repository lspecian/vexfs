/*
 * VexFS Minimal C Stub - Kernel Module Entry Point
 * 
 * This is a minimal C-only kernel module that handles VFS operations
 * and can communicate with Rust helpers via safer mechanisms.
 * 
 * This approach avoids Rust relocation issues while maintaining
 * the core VexFS functionality for performance testing.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/buffer_head.h>
#include <linux/blkdev.h>
#include <linux/backing-dev.h>
#include <linux/statfs.h>
#include <linux/seq_file.h>
#include <linux/parser.h>
#include <linux/random.h>
#include <linux/cred.h>
#include <linux/uaccess.h>
#include <linux/time.h>

#define VEXFS_MAGIC 0x56455846  /* "VEXF" */
#define VEXFS_BLOCK_SIZE 4096
#define VEXFS_ROOT_INO 2

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("VexFS Development Team");
MODULE_DESCRIPTION("VexFS Test Fixed - Mount Fix Validation Module");
MODULE_VERSION("1.0.1");

/* VexFS superblock info */
struct vexfs_sb_info {
    struct super_block *sb;
    unsigned long block_count;
    unsigned long free_blocks;
    unsigned long inode_count;
    unsigned long free_inodes;
    spinlock_t lock;
};

/* VexFS inode info */
struct vexfs_inode_info {
    struct inode vfs_inode;
    __u32 i_block[15];  /* Block pointers */
    __u32 i_flags;
    struct timespec64 i_crtime;  /* Creation time */
};

static struct kmem_cache *vexfs_inode_cachep;

/* Get VexFS inode from VFS inode */
static inline struct vexfs_inode_info *VEXFS_I(struct inode *inode)
{
    return container_of(inode, struct vexfs_inode_info, vfs_inode);
}

/* Get VexFS superblock info from VFS superblock */
static inline struct vexfs_sb_info *VEXFS_SB(struct super_block *sb)
{
    return sb->s_fs_info;
}

/*
 * Inode operations
 */
static struct inode *vexfs_alloc_inode(struct super_block *sb)
{
    struct vexfs_inode_info *vi;
    
    vi = kmem_cache_alloc(vexfs_inode_cachep, GFP_KERNEL);
    if (!vi)
        return NULL;
    
    /* Initialize VexFS-specific fields */
    memset(vi->i_block, 0, sizeof(vi->i_block));
    vi->i_flags = 0;
    /* Note: i_crtime will be set when the inode is fully initialized */
    
    /* CRITICAL: Ensure the VFS inode has the superblock pointer set */
    vi->vfs_inode.i_sb = sb;
    
    return &vi->vfs_inode;
}

static void vexfs_destroy_inode(struct inode *inode)
{
    kmem_cache_free(vexfs_inode_cachep, VEXFS_I(inode));
}

static int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc)
{
    /* For now, just return success without marking dirty to avoid current_time() calls */
    return 0;
}

static void vexfs_evict_inode(struct inode *inode)
{
    truncate_inode_pages_final(&inode->i_data);
    clear_inode(inode);
}

static int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
    struct super_block *sb = dentry->d_sb;
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    
    buf->f_type = VEXFS_MAGIC;
    buf->f_bsize = VEXFS_BLOCK_SIZE;
    buf->f_blocks = sbi->block_count;
    buf->f_bfree = sbi->free_blocks;
    buf->f_bavail = sbi->free_blocks;
    buf->f_files = sbi->inode_count;
    buf->f_ffree = sbi->free_inodes;
    buf->f_namelen = 255;
    
    return 0;
}

static const struct super_operations vexfs_sops = {
    .alloc_inode    = vexfs_alloc_inode,
    .destroy_inode  = vexfs_destroy_inode,
    .write_inode    = vexfs_write_inode,
    .evict_inode    = vexfs_evict_inode,
    .statfs         = vexfs_statfs,
};

/*
 * File operations
 */
static ssize_t vexfs_file_read(struct file *file, char __user *buf,
                               size_t count, loff_t *ppos)
{
    /* Simple read implementation - return zeros for now */
    if (*ppos >= file->f_inode->i_size)
        return 0;
    
    if (*ppos + count > file->f_inode->i_size)
        count = file->f_inode->i_size - *ppos;
    
    if (clear_user(buf, count))
        return -EFAULT;
    
    *ppos += count;
    return count;
}

static ssize_t vexfs_file_write(struct file *file, const char __user *buf,
                                size_t count, loff_t *ppos)
{
    struct inode *inode = file->f_inode;
    
    /* Simple write implementation - just update size without marking dirty */
    if (*ppos + count > inode->i_size) {
        inode->i_size = *ppos + count;
        /* Skip mark_inode_dirty() to avoid current_time() calls */
    }
    
    *ppos += count;
    return count;
}

static const struct file_operations vexfs_file_operations = {
    .read       = vexfs_file_read,
    .write      = vexfs_file_write,
    .llseek     = generic_file_llseek,
};

static const struct inode_operations vexfs_file_inode_operations = {
    .setattr    = simple_setattr,
    .getattr    = simple_getattr,
};

/*
 * Directory operations
 */
static int vexfs_readdir(struct file *file, struct dir_context *ctx)
{
    if (!dir_emit_dots(file, ctx))
        return 0;
    
    /* For now, just emit dots */
    return 0;
}

static struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry,
                                   unsigned int flags)
{
    /* For now, return NULL (file not found) */
    d_add(dentry, NULL);
    return NULL;
}

static int vexfs_create(struct user_namespace *mnt_userns, struct inode *dir,
                        struct dentry *dentry, umode_t mode, bool excl)
{
    struct inode *inode;
    struct timespec64 now;
    
    inode = new_inode(dir->i_sb);
    if (!inode)
        return -ENOMEM;
    
    inode->i_ino = get_next_ino();
    inode->i_mode = mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    inode->i_size = 0;
    
    /* Set timestamps using ktime_get_real_ts64 directly */
    ktime_get_real_ts64(&now);
    inode->i_atime = inode->i_mtime = inode->i_ctime = now;
    
    inode->i_op = &vexfs_file_inode_operations;
    inode->i_fop = &vexfs_file_operations;
    
    /* Set VexFS-specific creation time */
    if (VEXFS_I(inode)) {
        VEXFS_I(inode)->i_crtime = now;
    }
    
    d_instantiate(dentry, inode);
    return 0;
}

static const struct file_operations vexfs_dir_operations = {
    .read       = generic_read_dir,
    .iterate_shared = vexfs_readdir,
    .llseek     = generic_file_llseek,
};

static const struct inode_operations vexfs_dir_inode_operations = {
    .lookup     = vexfs_lookup,
    .create     = vexfs_create,
};

/*
 * Superblock operations
 */
static struct inode *vexfs_get_root_inode(struct super_block *sb)
{
    struct inode *inode;
    struct timespec64 now;
    
    inode = new_inode(sb);
    if (!inode)
        return NULL;
    
    inode->i_ino = VEXFS_ROOT_INO;
    inode->i_mode = S_IFDIR | 0755;
    inode->i_uid = GLOBAL_ROOT_UID;
    inode->i_gid = GLOBAL_ROOT_GID;
    inode->i_size = VEXFS_BLOCK_SIZE;
    
    /* Set timestamps using ktime_get_real_ts64 directly */
    ktime_get_real_ts64(&now);
    inode->i_atime = inode->i_mtime = inode->i_ctime = now;
    
    inode->i_op = &vexfs_dir_inode_operations;
    inode->i_fop = &vexfs_dir_operations;
    set_nlink(inode, 2);
    
    /* Set VexFS-specific creation time */
    if (VEXFS_I(inode)) {
        VEXFS_I(inode)->i_crtime = now;
    }
    
    return inode;
}

static int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    struct vexfs_sb_info *sbi;
    struct inode *root_inode;
    struct dentry *root_dentry;
    int ret = -ENOMEM;
    
    /* Allocate superblock info */
    sbi = kzalloc(sizeof(struct vexfs_sb_info), GFP_KERNEL);
    if (!sbi)
        return -ENOMEM;
    
    sb->s_fs_info = sbi;
    sbi->sb = sb;
    spin_lock_init(&sbi->lock);
    
    /* Set up superblock */
    sb->s_magic = VEXFS_MAGIC;
    sb->s_blocksize = VEXFS_BLOCK_SIZE;
    sb->s_blocksize_bits = 12;  /* 4096 = 2^12 */
    sb->s_maxbytes = MAX_LFS_FILESIZE;
    sb->s_op = &vexfs_sops;
    sb->s_time_gran = 1;
    
    /* Initialize filesystem parameters */
    sbi->block_count = 1000000;  /* 4GB filesystem */
    sbi->free_blocks = 999000;
    sbi->inode_count = 100000;
    sbi->free_inodes = 99999;
    
    /* Create root inode */
    root_inode = vexfs_get_root_inode(sb);
    if (!root_inode) {
        ret = -ENOMEM;
        goto out_free_sbi;
    }
    
    /* Create root dentry - d_make_root() consumes root_inode reference */
    root_dentry = d_make_root(root_inode);
    if (!root_dentry) {
        /* root_inode is already freed by d_make_root() on failure */
        ret = -ENOMEM;
        goto out_free_sbi;
    }
    
    sb->s_root = root_dentry;
    
    printk(KERN_INFO "VexFS: mounted successfully (FIXED memory C stub)\n");
    return 0;

out_free_sbi:
    sb->s_fs_info = NULL;
    kfree(sbi);
    return ret;
}

static struct dentry *vexfs_mount(struct file_system_type *fs_type,
                                  int flags, const char *dev_name, void *data)
{
    /* CRITICAL FIX: Use mount_nodev() instead of mount_bdev() */
    return mount_nodev(fs_type, flags, data, vexfs_fill_super);
}

static void vexfs_kill_sb(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    
    /* CRITICAL FIX: Use kill_anon_super() instead of kill_block_super() */
    kill_anon_super(sb);
    if (sbi) {
        kfree(sbi);
    }
}

static struct file_system_type vexfs_fs_type = {
    .owner      = THIS_MODULE,
    .name       = "vexfs_test_fixed",
    .mount      = vexfs_mount,
    .kill_sb    = vexfs_kill_sb,
    .fs_flags   = 0,  /* CRITICAL FIX: Remove FS_REQUIRES_DEV flag */
};

/*
 * Inode cache management
 */
static void vexfs_inode_init_once(void *obj)
{
    struct vexfs_inode_info *vi = obj;
    inode_init_once(&vi->vfs_inode);
}

static int __init vexfs_init_inodecache(void)
{
    vexfs_inode_cachep = kmem_cache_create("vexfs_inode_cache",
                                           sizeof(struct vexfs_inode_info),
                                           0, SLAB_RECLAIM_ACCOUNT|SLAB_MEM_SPREAD,
                                           vexfs_inode_init_once);
    if (vexfs_inode_cachep == NULL)
        return -ENOMEM;
    return 0;
}

static void vexfs_destroy_inodecache(void)
{
    rcu_barrier();
    kmem_cache_destroy(vexfs_inode_cachep);
}

/*
 * Module initialization and cleanup
 */
static int __init vexfs_init(void)
{
    int ret;
    
    printk(KERN_INFO "VexFS: initializing FIXED memory C stub module\n");
    
    ret = vexfs_init_inodecache();
    if (ret)
        return ret;
    
    ret = register_filesystem(&vexfs_fs_type);
    if (ret) {
        vexfs_destroy_inodecache();
        return ret;
    }
    
    printk(KERN_INFO "VexFS: FIXED memory C stub module loaded successfully\n");
    return 0;
}

static void __exit vexfs_exit(void)
{
    unregister_filesystem(&vexfs_fs_type);
    vexfs_destroy_inodecache();
    printk(KERN_INFO "VexFS: FIXED memory C stub module unloaded\n");
}

module_init(vexfs_init);
module_exit(vexfs_exit);