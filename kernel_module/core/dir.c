/*
 * VexFS - Vector Extension Filesystem
 * Directory Operations
 *
 * This file implements directory operations for VexFS,
 * including directory entry management and traversal.
 */

#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/uaccess.h>

#include "../include/vexfs_core.h"
#include "../include/vexfs_block.h"

/*
 * CRITICAL FIX: VexFS directory file operations replaced with simple_dir_operations
 *
 * The custom vexfs_dir_ops caused VFS deadlocks and unkillable umount processes.
 * We now use libfs simple_dir_operations which provides battle-tested VFS-compliant
 * directory operations that prevent I/O list deadlocks and CPU spinning issues.
 *
 * Original custom operations (now disabled):
 * - .llseek     = default_llseek,
 * - .read       = generic_read_dir,
 * - .iterate_shared = vexfs_readdir,
 * - .fsync      = noop_fsync,
 *
 * Replaced with: simple_dir_operations from linux/libfs.h
 */

/*
 * Directory entry structure for on-disk storage
 */
struct vexfs_dir_entry {
    __le32 inode;              /* Inode number */
    __le16 rec_len;            /* Record length */
    __u8   name_len;           /* Name length */
    __u8   file_type;          /* File type */
    char   name[];             /* Variable length name */
};

#define VEXFS_DIR_ENTRY_SIZE(name_len) \
    (sizeof(struct vexfs_dir_entry) + (name_len) + 3) & ~3

/*
 * File type constants for directory entries
 */
#define VEXFS_FT_UNKNOWN    0
#define VEXFS_FT_REG_FILE   1
#define VEXFS_FT_DIR        2
#define VEXFS_FT_CHRDEV     3
#define VEXFS_FT_BLKDEV     4
#define VEXFS_FT_FIFO       5
#define VEXFS_FT_SOCK       6
#define VEXFS_FT_SYMLINK    7

/*
 * Convert inode mode to directory entry file type
 */
static unsigned char vexfs_type_by_mode[S_IFMT >> S_DT_SHIFT] = {
    [S_IFREG >> S_DT_SHIFT]  = VEXFS_FT_REG_FILE,
    [S_IFDIR >> S_DT_SHIFT]  = VEXFS_FT_DIR,
    [S_IFCHR >> S_DT_SHIFT]  = VEXFS_FT_CHRDEV,
    [S_IFBLK >> S_DT_SHIFT]  = VEXFS_FT_BLKDEV,
    [S_IFIFO >> S_DT_SHIFT]  = VEXFS_FT_FIFO,
    [S_IFSOCK >> S_DT_SHIFT] = VEXFS_FT_SOCK,
    [S_IFLNK >> S_DT_SHIFT]  = VEXFS_FT_SYMLINK,
};

static inline unsigned char vexfs_filetype_table(umode_t mode)
{
    return vexfs_type_by_mode[(mode & S_IFMT) >> S_DT_SHIFT];
}

/*
 * Get the first data block for a directory
 */
static struct buffer_head *vexfs_get_dir_block(struct inode *dir, int create)
{
    struct vexfs_inode_info *vi = VEXFS_I(dir);
    struct buffer_head *bh;
    __u32 block;

    printk(KERN_DEBUG "VexFS: vexfs_get_dir_block: inode %lu, i_block_count=%u, i_blocks[0]=%u, create=%d\n",
           dir->i_ino, vi->i_block_count, vi->i_blocks[0], create);

    if (vi->i_block_count == 0 && create) {
        /* Allocate first block for directory */
        printk(KERN_DEBUG "VexFS: Allocating new directory block for inode %lu\n", dir->i_ino);
        if (vexfs_alloc_block(dir->i_sb, &block) != 0) {
            printk(KERN_ERR "VexFS: Failed to allocate directory block for inode %lu\n", dir->i_ino);
            return NULL;
        }
        vi->i_blocks[0] = block;
        vi->i_block_count = 1;
        dir->i_blocks = 1;
        
        printk(KERN_DEBUG "VexFS: Allocated directory block %u for inode %lu\n", block, dir->i_ino);
        
        /* Mark inode dirty - will be written later */
        mark_inode_dirty(dir);
    } else if (vi->i_block_count == 0) {
        printk(KERN_ERR "VexFS: Directory inode %lu has no blocks and create=0\n", dir->i_ino);
        return NULL;
    } else {
        block = vi->i_blocks[0];
        printk(KERN_DEBUG "VexFS: Using existing directory block %u for inode %lu\n", block, dir->i_ino);
    }

    printk(KERN_DEBUG "VexFS: Reading directory block %u for inode %lu\n", block, dir->i_ino);
    bh = sb_bread(dir->i_sb, block);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read directory block %u for inode %lu\n", block, dir->i_ino);
        return NULL;
    }

    return bh;
}

/*
 * Initialize a new directory with "." and ".." entries
 */
int vexfs_init_dir(struct inode *dir, struct inode *parent)
{
    struct buffer_head *bh;
    struct vexfs_dir_entry *de;
    char *data;
    int offset = 0;

    bh = vexfs_get_dir_block(dir, 1);
    if (!bh) {
        return -EIO;
    }

    data = bh->b_data;
    memset(data, 0, VEXFS_BLOCK_SIZE);

    /* Create "." entry */
    de = (struct vexfs_dir_entry *)data;
    de->inode = cpu_to_le32(dir->i_ino);
    de->name_len = 1;
    de->file_type = VEXFS_FT_DIR;
    de->rec_len = cpu_to_le16(VEXFS_DIR_ENTRY_SIZE(1));
    strcpy(de->name, ".");
    
    offset += le16_to_cpu(de->rec_len);

    /* Create ".." entry */
    de = (struct vexfs_dir_entry *)(data + offset);
    de->inode = cpu_to_le32(parent->i_ino);
    de->name_len = 2;
    de->file_type = VEXFS_FT_DIR;
    de->rec_len = cpu_to_le16(VEXFS_BLOCK_SIZE - offset);
    strcpy(de->name, "..");

    mark_buffer_dirty(bh);
    brelse(bh);

    return 0;
}

/*
 * Find a directory entry by name
 */
unsigned long vexfs_find_dir_entry(struct inode *dir, const struct qstr *name)
{
    struct buffer_head *bh;
    struct vexfs_dir_entry *de;
    char *data;
    int offset = 0;
    unsigned long ino = 0;

    bh = vexfs_get_dir_block(dir, 0);
    if (!bh) {
        return 0;
    }

    data = bh->b_data;

    while (offset < VEXFS_BLOCK_SIZE) {
        de = (struct vexfs_dir_entry *)(data + offset);
        
        if (le16_to_cpu(de->rec_len) == 0) {
            break;
        }

        if (le32_to_cpu(de->inode) != 0 && 
            de->name_len == name->len &&
            strncmp(de->name, name->name, name->len) == 0) {
            ino = le32_to_cpu(de->inode);
            break;
        }

        offset += le16_to_cpu(de->rec_len);
    }

    brelse(bh);
    return ino;
}

/*
 * Add a directory entry
 */
int vexfs_add_dir_entry(struct inode *dir, struct dentry *dentry, 
                        struct inode *inode)
{
    struct buffer_head *bh;
    struct vexfs_dir_entry *de, *new_de;
    char *data;
    int offset = 0;
    int name_len = dentry->d_name.len;
    int rec_len = VEXFS_DIR_ENTRY_SIZE(name_len);
    int found_space = 0;

    if (name_len > VEXFS_MAX_NAME_LEN) {
        return -ENAMETOOLONG;
    }

    bh = vexfs_get_dir_block(dir, 0);
    if (!bh) {
        return -EIO;
    }

    data = bh->b_data;

    /* Find space for new entry */
    while (offset < VEXFS_BLOCK_SIZE) {
        de = (struct vexfs_dir_entry *)(data + offset);
        
        if (le16_to_cpu(de->rec_len) == 0) {
            /* End of directory, add entry here */
            de->inode = cpu_to_le32(inode->i_ino);
            de->name_len = name_len;
            de->file_type = vexfs_filetype_table(inode->i_mode);
            de->rec_len = cpu_to_le16(VEXFS_BLOCK_SIZE - offset);
            strncpy(de->name, dentry->d_name.name, name_len);
            found_space = 1;
            break;
        }

        /* Check if we can split this entry */
        int actual_len = VEXFS_DIR_ENTRY_SIZE(de->name_len);
        int available = le16_to_cpu(de->rec_len) - actual_len;
        
        if (available >= rec_len) {
            /* Split the entry */
            new_de = (struct vexfs_dir_entry *)(data + offset + actual_len);
            new_de->inode = cpu_to_le32(inode->i_ino);
            new_de->name_len = name_len;
            new_de->file_type = vexfs_filetype_table(inode->i_mode);
            new_de->rec_len = cpu_to_le16(available);
            strncpy(new_de->name, dentry->d_name.name, name_len);
            
            /* Update previous entry's rec_len */
            de->rec_len = cpu_to_le16(actual_len);
            found_space = 1;
            break;
        }

        offset += le16_to_cpu(de->rec_len);
    }

    if (!found_space) {
        brelse(bh);
        return -ENOSPC;
    }

    mark_buffer_dirty(bh);
    /* Remove sync_dirty_buffer() to prevent hanging - let kernel handle async writes */
    brelse(bh);

    /* Update directory timestamps */
    struct timespec64 now = current_time(dir);
    inode_set_mtime_to_ts(dir, now);
    inode_set_ctime_to_ts(dir, now);
    mark_inode_dirty(dir);

    return 0;
}

/*
 * Remove a directory entry
 */
int vexfs_remove_dir_entry(struct inode *dir, const struct qstr *name)
{
    struct buffer_head *bh;
    struct vexfs_dir_entry *de, *prev_de = NULL;
    char *data;
    int offset = 0;
    int found = 0;

    bh = vexfs_get_dir_block(dir, 0);
    if (!bh) {
        return -EIO;
    }

    data = bh->b_data;

    while (offset < VEXFS_BLOCK_SIZE) {
        de = (struct vexfs_dir_entry *)(data + offset);
        
        if (le16_to_cpu(de->rec_len) == 0) {
            break;
        }

        if (le32_to_cpu(de->inode) != 0 && 
            de->name_len == name->len &&
            strncmp(de->name, name->name, name->len) == 0) {
            
            /* Found the entry to remove */
            if (prev_de) {
                /* Merge with previous entry */
                prev_de->rec_len = cpu_to_le16(
                    le16_to_cpu(prev_de->rec_len) + le16_to_cpu(de->rec_len));
            } else {
                /* First entry, just mark as deleted */
                de->inode = 0;
            }
            found = 1;
            break;
        }

        prev_de = de;
        offset += le16_to_cpu(de->rec_len);
    }

    if (!found) {
        brelse(bh);
        return -ENOENT;
    }

    mark_buffer_dirty(bh);
    /* Remove sync_dirty_buffer() to prevent hanging - let kernel handle async writes */
    brelse(bh);

    /* Update directory timestamps */
    struct timespec64 now = current_time(dir);
    inode_set_mtime_to_ts(dir, now);
    inode_set_ctime_to_ts(dir, now);
    mark_inode_dirty(dir);

    return 0;
}

/*
 * Check if directory is empty (only contains "." and "..")
 */
int vexfs_dir_is_empty(struct inode *dir)
{
    struct buffer_head *bh;
    struct vexfs_dir_entry *de;
    char *data;
    int offset = 0;
    int entry_count = 0;

    bh = vexfs_get_dir_block(dir, 0);
    if (!bh) {
        return 1; /* Assume empty if can't read */
    }

    data = bh->b_data;

    while (offset < VEXFS_BLOCK_SIZE) {
        de = (struct vexfs_dir_entry *)(data + offset);
        
        if (le16_to_cpu(de->rec_len) == 0) {
            break;
        }

        if (le32_to_cpu(de->inode) != 0) {
            entry_count++;
            /* More than "." and ".." means not empty */
            if (entry_count > 2) {
                brelse(bh);
                return 0;
            }
        }

        offset += le16_to_cpu(de->rec_len);
    }

    brelse(bh);
    return (entry_count <= 2);
}

/*
 * Read directory entries (for readdir system call)
 */
int vexfs_readdir(struct file *file, struct dir_context *ctx)
{
    struct inode *inode = file_inode(file);
    struct buffer_head *bh;
    struct vexfs_dir_entry *de;
    char *data;
    int offset = 0;
    loff_t pos = ctx->pos;
    int current_entry = 0;

    if (pos >= inode->i_size) {
        return 0;
    }

    bh = vexfs_get_dir_block(inode, 0);
    if (!bh) {
        return -EIO;
    }

    data = bh->b_data;

    while (offset < VEXFS_BLOCK_SIZE) {
        de = (struct vexfs_dir_entry *)(data + offset);
        
        if (le16_to_cpu(de->rec_len) == 0) {
            break;
        }

        if (le32_to_cpu(de->inode) != 0) {
            if (current_entry >= pos) {
                char name[VEXFS_MAX_NAME_LEN + 1];
                strncpy(name, de->name, de->name_len);
                name[de->name_len] = '\0';
                
                if (!dir_emit(ctx, name, de->name_len,
                             le32_to_cpu(de->inode), de->file_type)) {
                    brelse(bh);
                    return 0;
                }
                ctx->pos++;
            }
            current_entry++;
        }

        offset += le16_to_cpu(de->rec_len);
    }

    brelse(bh);
    return 0;
}