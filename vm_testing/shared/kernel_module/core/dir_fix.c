/*
 * VexFS Directory Operations Fix
 * 
 * This file contains the fix for directory operations timeout issue.
 * The problem: simple_dir_operations expects dentries to be in the dcache,
 * but VexFS stores directory entries on disk.
 * 
 * Solution: Implement a custom readdir that reads from disk but is compatible
 * with VFS expectations.
 */

#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/string.h>

#include "../include/vexfs_core.h"
#include "../include/vexfs_block.h"

/* Directory entry structure - copied from dir.c */
struct vexfs_dir_entry {
    __le32 inode;
    __le16 rec_len;
    __u8   name_len;
    __u8   file_type;
    char   name[];
};

/* File type constants */
#define VEXFS_FT_UNKNOWN    0
#define VEXFS_FT_REG_FILE   1
#define VEXFS_FT_DIR        2
#define VEXFS_FT_CHRDEV     3
#define VEXFS_FT_BLKDEV     4
#define VEXFS_FT_FIFO       5
#define VEXFS_FT_SOCK       6
#define VEXFS_FT_SYMLINK    7

/*
 * VexFS directory file operations - Fixed version
 * 
 * We need custom readdir to read from disk, but we keep other operations
 * from simple_dir_operations to maintain VFS compatibility.
 */
static int vexfs_readdir_fixed(struct file *file, struct dir_context *ctx)
{
    struct inode *inode = file_inode(file);
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    struct buffer_head *bh;
    struct vexfs_dir_entry *de;
    char *data;
    int offset = 0;
    
    /* Position 0 and 1 are for "." and ".." */
    if (ctx->pos == 0) {
        if (!dir_emit_dot(file, ctx))
            return 0;
        ctx->pos = 1;
    }
    
    if (ctx->pos == 1) {
        if (!dir_emit_dotdot(file, ctx))
            return 0;
        ctx->pos = 2;
    }
    
    /* For empty directories or no blocks allocated */
    if (vi->i_block_count == 0) {
        return 0;
    }
    
    /* Read the directory block */
    bh = sb_bread(inode->i_sb, vi->i_blocks[0]);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read directory block for inode %lu\n", 
               inode->i_ino);
        return -EIO;
    }
    
    data = bh->b_data;
    
    /* Skip to the position we need based on ctx->pos */
    int entry_num = 2; /* We already emitted . and .. */
    
    while (offset < VEXFS_BLOCK_SIZE) {
        de = (struct vexfs_dir_entry *)(data + offset);
        
        /* Check for end of entries */
        if (le16_to_cpu(de->rec_len) == 0) {
            break;
        }
        
        /* Sanity check to prevent infinite loops */
        if (le16_to_cpu(de->rec_len) < sizeof(struct vexfs_dir_entry)) {
            printk(KERN_ERR "VexFS: Invalid directory entry record length\n");
            brelse(bh);
            return -EIO;
        }
        
        /* Skip deleted entries (inode == 0) */
        if (le32_to_cpu(de->inode) != 0) {
            /* Skip "." and ".." entries in the on-disk format */
            if (!(de->name_len == 1 && de->name[0] == '.') &&
                !(de->name_len == 2 && de->name[0] == '.' && de->name[1] == '.')) {
                
                if (entry_num >= ctx->pos) {
                    /* Convert file type to DT_ format */
                    unsigned char dt_type = DT_UNKNOWN;
                    switch (de->file_type) {
                        case VEXFS_FT_REG_FILE: dt_type = DT_REG; break;
                        case VEXFS_FT_DIR:      dt_type = DT_DIR; break;
                        case VEXFS_FT_SYMLINK:  dt_type = DT_LNK; break;
                        case VEXFS_FT_CHRDEV:   dt_type = DT_CHR; break;
                        case VEXFS_FT_BLKDEV:   dt_type = DT_BLK; break;
                        case VEXFS_FT_FIFO:     dt_type = DT_FIFO; break;
                        case VEXFS_FT_SOCK:     dt_type = DT_SOCK; break;
                    }
                    
                    /* Emit the directory entry */
                    if (!dir_emit(ctx, de->name, de->name_len,
                                 le32_to_cpu(de->inode), dt_type)) {
                        brelse(bh);
                        return 0;
                    }
                    ctx->pos = entry_num + 1;
                }
                entry_num++;
            }
        }
        
        offset += le16_to_cpu(de->rec_len);
    }
    
    brelse(bh);
    return 0;
}

/* Fixed directory file operations */
const struct file_operations vexfs_dir_operations_fixed = {
    .llseek         = generic_file_llseek,
    .read           = generic_read_dir,
    .iterate_shared = vexfs_readdir_fixed,
    .fsync          = noop_fsync,
};

/*
 * Apply the directory operations fix
 * This function should be called from inode.c when setting up directory inodes
 */
void vexfs_apply_dir_fix(struct inode *inode)
{
    if (S_ISDIR(inode->i_mode)) {
        inode->i_fop = &vexfs_dir_operations_fixed;
        /* Keep using empty_aops for directories to prevent deadlocks */
        inode->i_mapping->a_ops = &empty_aops;
    }
}