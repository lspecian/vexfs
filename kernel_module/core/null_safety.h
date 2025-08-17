/*
 * VexFS NULL Pointer Safety Macros and Checks
 *
 * This header provides safety macros to prevent NULL pointer
 * dereferences that can cause kernel panics.
 */

#ifndef _VEXFS_NULL_SAFETY_H
#define _VEXFS_NULL_SAFETY_H

#include <linux/kernel.h>
#include <linux/fs.h>

/* Safe pointer validation macros */
#define VEXFS_CHECK_PTR(ptr, name, ret) do { \
    if (unlikely(!(ptr))) { \
        printk(KERN_ERR "VexFS: %s is NULL at %s:%d\n", \
               name, __FILE__, __LINE__); \
        dump_stack(); \
        return ret; \
    } \
} while(0)

#define VEXFS_CHECK_PTR_VOID(ptr, name) do { \
    if (unlikely(!(ptr))) { \
        printk(KERN_ERR "VexFS: %s is NULL at %s:%d\n", \
               name, __FILE__, __LINE__); \
        dump_stack(); \
        return; \
    } \
} while(0)

#define VEXFS_CHECK_PTR_NULL(ptr, name) do { \
    if (unlikely(!(ptr))) { \
        printk(KERN_ERR "VexFS: %s is NULL at %s:%d\n", \
               name, __FILE__, __LINE__); \
        dump_stack(); \
        return NULL; \
    } \
} while(0)

/* Validate inode and its components */
static inline int vexfs_validate_inode(struct inode *inode, const char *func)
{
    if (unlikely(!inode)) {
        printk(KERN_ERR "VexFS: %s: inode is NULL\n", func);
        return -EINVAL;
    }
    
    if (unlikely(!inode->i_sb)) {
        printk(KERN_ERR "VexFS: %s: inode->i_sb is NULL for inode %lu\n", 
               func, inode->i_ino);
        return -EINVAL;
    }
    
    if (unlikely(!inode->i_sb->s_fs_info)) {
        printk(KERN_ERR "VexFS: %s: inode->i_sb->s_fs_info is NULL for inode %lu\n",
               func, inode->i_ino);
        return -EINVAL;
    }
    
    return 0;
}

/* Validate superblock */
static inline int vexfs_validate_sb(struct super_block *sb, const char *func)
{
    if (unlikely(!sb)) {
        printk(KERN_ERR "VexFS: %s: superblock is NULL\n", func);
        return -EINVAL;
    }
    
    if (unlikely(!sb->s_fs_info)) {
        printk(KERN_ERR "VexFS: %s: sb->s_fs_info is NULL\n", func);
        return -EINVAL;
    }
    
    return 0;
}

/* Safe VEXFS_INODE macro wrapper */
static inline struct vexfs_inode_info *vexfs_inode_safe(struct inode *inode)
{
    if (unlikely(!inode))
        return NULL;
    
    return container_of(inode, struct vexfs_inode_info, vfs_inode);
}

/* Safe VEXFS_SB macro wrapper */
static inline struct vexfs_sb_info *vexfs_sb_safe(struct super_block *sb)
{
    if (unlikely(!sb || !sb->s_fs_info))
        return NULL;
    
    return (struct vexfs_sb_info *)sb->s_fs_info;
}

/* Memory allocation with NULL check and zeroing */
static inline void *vexfs_kzalloc(size_t size, gfp_t flags)
{
    void *ptr = kzalloc(size, flags);
    if (unlikely(!ptr)) {
        printk(KERN_ERR "VexFS: Failed to allocate %zu bytes\n", size);
    }
    return ptr;
}

/* Buffer head validation */
static inline int vexfs_validate_bh(struct buffer_head *bh, const char *func)
{
    if (unlikely(!bh)) {
        printk(KERN_ERR "VexFS: %s: buffer_head is NULL\n", func);
        return -EIO;
    }
    
    if (unlikely(!bh->b_data)) {
        printk(KERN_ERR "VexFS: %s: buffer_head->b_data is NULL\n", func);
        return -EIO;
    }
    
    return 0;
}

/* Debug mode for extra safety checks */
#ifdef CONFIG_VEXFS_DEBUG
#define VEXFS_DEBUG_CHECK(condition, msg) do { \
    if (unlikely(!(condition))) { \
        printk(KERN_WARNING "VexFS DEBUG: %s at %s:%d\n", \
               msg, __FILE__, __LINE__); \
    } \
} while(0)
#else
#define VEXFS_DEBUG_CHECK(condition, msg) do {} while(0)
#endif

#endif /* _VEXFS_NULL_SAFETY_H */