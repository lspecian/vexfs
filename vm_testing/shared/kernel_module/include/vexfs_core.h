/*
 * VexFS Core Filesystem Definitions
 * 
 * This header defines the core VFS-compliant filesystem structures
 * and operations, following Linux kernel filesystem patterns.
 */

#ifndef VEXFS_CORE_H
#define VEXFS_CORE_H

#include <linux/fs.h>
#include <linux/types.h>
#include <linux/buffer_head.h>
#include <linux/blkdev.h>

/* VexFS Magic Number */
#define VEXFS_MAGIC 0x56455846  /* "VEXF" */

/* VexFS Version */
#define VEXFS_VERSION_MAJOR 2
#define VEXFS_VERSION_MINOR 0
#define VEXFS_VERSION_PATCH 0

/* Block size constants */
#define VEXFS_BLOCK_SIZE 4096
#define VEXFS_BLOCK_SIZE_BITS 12

/* Inode constants */
#define VEXFS_ROOT_INO 1
#define VEXFS_MAX_INODES 65536
#define VEXFS_DIRECT_BLOCKS 12
#define VEXFS_MAX_NAME_LEN 255

/* File type constants for directory entries */
#define VEXFS_FT_UNKNOWN     0
#define VEXFS_FT_REG_FILE    1
#define VEXFS_FT_DIR         2
#define VEXFS_FT_CHRDEV      3
#define VEXFS_FT_BLKDEV      4
#define VEXFS_FT_FIFO        5
#define VEXFS_FT_SOCK        6
#define VEXFS_FT_SYMLINK     7

/* Core VexFS structures */
struct vexfs_sb_info {
    struct super_block *sb;
    unsigned long block_count;
    unsigned long inode_count;
    atomic_long_t free_blocks;    /* DEADLOCK FIX: Use atomic for thread-safe access */
    atomic_long_t free_inodes;    /* DEADLOCK FIX: Use atomic for thread-safe access */
    struct buffer_head *sb_bh;
    spinlock_t bitmap_lock;       /* DEADLOCK FIX: Fine-grained lock for bitmap operations */
};

struct vexfs_inode_info {
    struct inode vfs_inode;
    __u32 i_blocks[12];     /* Direct blocks */
    __u32 i_block_count;    /* Number of allocated blocks */
    __u32 i_vector_count;   /* Number of vectors stored */
    /* DEADLOCK FIX: Removed i_mutex - VFS provides proper inode locking */
};

/* Core filesystem operations */
extern const struct super_operations vexfs_super_ops;
extern const struct inode_operations vexfs_dir_inode_ops;
extern const struct inode_operations vexfs_file_inode_ops;
/* CRITICAL FIX: vexfs_dir_ops replaced with simple_dir_operations from libfs */
/* extern const struct file_operations vexfs_dir_ops; */
extern const struct file_operations vexfs_file_ops;
extern const struct address_space_operations vexfs_aops;

/* Core function declarations */
struct inode *vexfs_alloc_inode(struct super_block *sb);
void vexfs_free_inode(struct inode *inode);
int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc);
void vexfs_evict_inode(struct inode *inode);
int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf);

/* Core filesystem operations */
struct inode *vexfs_iget(struct super_block *sb, unsigned long ino);
int vexfs_write_inode_to_disk(struct inode *inode);

/* Inode operations */
int vexfs_create(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode, bool excl);
struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry, unsigned int flags);
int vexfs_mkdir(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode);
int vexfs_rmdir(struct inode *dir, struct dentry *dentry);
int vexfs_unlink(struct inode *dir, struct dentry *dentry);
int vexfs_rename(struct mnt_idmap *idmap, struct inode *old_dir, struct dentry *old_dentry,
                 struct inode *new_dir, struct dentry *new_dentry, unsigned int flags);

/* Directory operations */
int vexfs_init_dir(struct inode *dir, struct inode *parent);
unsigned long vexfs_find_dir_entry(struct inode *dir, const struct qstr *name);
int vexfs_add_dir_entry(struct inode *dir, struct dentry *dentry, struct inode *inode);
int vexfs_remove_dir_entry(struct inode *dir, const struct qstr *name);
int vexfs_dir_is_empty(struct inode *dir);
int vexfs_readdir(struct file *file, struct dir_context *ctx);

/* File operations */
int vexfs_read_folio(struct file *file, struct folio *folio);
int vexfs_writepage(struct page *page, struct writeback_control *wbc);
int vexfs_writepages(struct address_space *mapping, struct writeback_control *wbc);
int vexfs_write_begin(struct file *file, struct address_space *mapping,
                     loff_t pos, unsigned len, struct page **pagep, void **fsdata);
int vexfs_write_end(struct file *file, struct address_space *mapping,
                   loff_t pos, unsigned len, unsigned copied,
                   struct page *page, void *fsdata);
sector_t vexfs_bmap(struct address_space *mapping, sector_t block);
ssize_t vexfs_direct_IO(struct kiocb *iocb, struct iov_iter *iter);
void vexfs_truncate(struct inode *inode);
int vexfs_setattr(struct dentry *dentry, struct iattr *attr);
int vexfs_getattr(const struct path *path, struct kstat *stat,
                 u32 request_mask, unsigned int flags);
int vexfs_permission(struct inode *inode, int mask);

/* Extended attributes */
ssize_t vexfs_listxattr(struct dentry *dentry, char *buffer, size_t size);
ssize_t vexfs_getxattr(struct dentry *dentry, struct inode *inode,
                      const char *name, void *buffer, size_t size);
int vexfs_setxattr(struct dentry *dentry, struct inode *inode,
                  const char *name, const void *value, size_t size, int flags);
int vexfs_removexattr(struct dentry *dentry, const char *name);

/* File locking and allocation */
int vexfs_lock(struct file *file, int cmd, struct file_lock *fl);
int vexfs_lease(struct file *file, long arg);
long vexfs_fallocate(struct file *file, int mode, loff_t offset, loff_t len);

/* Helper functions */
static inline struct vexfs_sb_info *VEXFS_SB(struct super_block *sb)
{
    return sb->s_fs_info;
}

static inline struct vexfs_inode_info *VEXFS_I(struct inode *inode)
{
    return container_of(inode, struct vexfs_inode_info, vfs_inode);
}

#endif /* VEXFS_CORE_H */