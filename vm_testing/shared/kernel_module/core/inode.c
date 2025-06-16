/*
 * VexFS - Vector Extension Filesystem
 * Core Inode Operations
 *
 * This file implements VFS-compliant inode operations for VexFS,
 * following Linux kernel filesystem patterns.
 */

#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/time.h>
#include <linux/stat.h>
#include <linux/uaccess.h>
#include <linux/pagemap.h>

#include "../include/vexfs_core.h"
#include "../include/vexfs_block.h"

/*
 * VexFS inode operations for regular files
 */
const struct inode_operations vexfs_file_inode_ops = {
    .setattr    = simple_setattr,
    .getattr    = simple_getattr,
};

/*
 * VexFS inode operations for directories
 */
const struct inode_operations vexfs_dir_inode_ops = {
    .create     = vexfs_create,
    .lookup     = vexfs_lookup,
    .mkdir      = vexfs_mkdir,
    .rmdir      = vexfs_rmdir,
    .unlink     = vexfs_unlink,
    .rename     = vexfs_rename,
    .setattr    = simple_setattr,
    .getattr    = simple_getattr,
};

/*
 * Initialize a VexFS inode with default values
 */
static void vexfs_init_inode(struct inode *inode, umode_t mode)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    
    inode->i_mode = mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    struct timespec64 now = current_time(inode);
    inode_set_atime_to_ts(inode, now);
    inode_set_mtime_to_ts(inode, now);
    inode_set_ctime_to_ts(inode, now);
    inode->i_blocks = 0;
    inode->i_size = 0;
    
    /* Initialize VexFS-specific inode info */
    vi->i_block_count = 0;
    vi->i_vector_count = 0;
    memset(vi->i_blocks, 0, sizeof(vi->i_blocks));
    /* DEADLOCK FIX: Removed mutex_init - VFS provides proper inode locking */
}

/*
 * Read inode from disk
 */
struct inode *vexfs_iget(struct super_block *sb, unsigned long ino)
{
    struct inode *inode;
    struct vexfs_inode_info *vi;
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct buffer_head *bh;
    struct vexfs_inode *disk_inode;
    unsigned long block_num;
    unsigned long offset;
    int i;

    inode = iget_locked(sb, ino);
    if (!inode)
        return ERR_PTR(-ENOMEM);

    if (!(inode->i_state & I_NEW))
        return inode;

    vi = VEXFS_I(inode);

    /* Calculate block and offset for this inode */
    /* Use same calculation as superblock.c for consistency */
    block_num = VEXFS_INODE_TABLE_BLOCK + ((ino - 1) / VEXFS_INODES_PER_BLOCK);
    offset = ((ino - 1) % VEXFS_INODES_PER_BLOCK) * sizeof(struct vexfs_inode);

    bh = sb_bread(sb, block_num);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read inode %lu\n", ino);
        iget_failed(inode);
        return ERR_PTR(-EIO);
    }
    
    /* Check if this is a valid inode block */
    if (block_num == 0 || offset >= VEXFS_BLOCK_SIZE) {
        printk(KERN_ERR "VexFS: Invalid inode %lu location (block=%lu, offset=%lu)\n",
               ino, block_num, offset);
        brelse(bh);
        iget_failed(inode);
        return ERR_PTR(-EINVAL);
    }

    disk_inode = (struct vexfs_inode *)(bh->b_data + offset);

    /* DEBUG: Print raw disk inode data for root inode */
    if (ino == VEXFS_ROOT_INO) {
        printk(KERN_DEBUG "VexFS: Root inode raw data at block=%lu, offset=%lu:\n", block_num, offset);
        printk(KERN_DEBUG "VexFS: i_mode=0x%04x, i_links_count=%u, i_uid=%u, i_gid=%u\n",
               le16_to_cpu(disk_inode->i_mode), le16_to_cpu(disk_inode->i_links_count),
               le32_to_cpu(disk_inode->i_uid), le32_to_cpu(disk_inode->i_gid));
        printk(KERN_DEBUG "VexFS: i_size=%llu, i_blocks=%u\n",
               le64_to_cpu(disk_inode->i_size), le32_to_cpu(disk_inode->i_blocks));
        printk(KERN_DEBUG "VexFS: i_block[0]=%u, i_block[1]=%u, i_block[2]=%u\n",
               le32_to_cpu(disk_inode->i_block[0]), le32_to_cpu(disk_inode->i_block[1]), le32_to_cpu(disk_inode->i_block[2]));
    }

    /* Check if disk inode is valid */
    if (le16_to_cpu(disk_inode->i_mode) == 0) {
        printk(KERN_ERR "VexFS: Inode %lu has invalid mode (0)\n", ino);
        brelse(bh);
        iget_failed(inode);
        return ERR_PTR(-ENOENT);
    }

    /* Copy disk inode to VFS inode - FIXED data types */
    inode->i_mode = le16_to_cpu(disk_inode->i_mode);
    i_uid_write(inode, le32_to_cpu(disk_inode->i_uid));  /* FIXED: 32-bit UID */
    i_gid_write(inode, le32_to_cpu(disk_inode->i_gid));  /* FIXED: 32-bit GID */
    inode->i_size = le64_to_cpu(disk_inode->i_size);     /* FIXED: 64-bit size */
    inode->i_blocks = le32_to_cpu(disk_inode->i_blocks);
    set_nlink(inode, le16_to_cpu(disk_inode->i_links_count)); /* FIXED: use links_count */
    
    struct timespec64 atime = { .tv_sec = le32_to_cpu(disk_inode->i_atime), .tv_nsec = 0 };
    struct timespec64 mtime = { .tv_sec = le32_to_cpu(disk_inode->i_mtime), .tv_nsec = 0 };
    struct timespec64 ctime = { .tv_sec = le32_to_cpu(disk_inode->i_ctime), .tv_nsec = 0 };
    inode_set_atime_to_ts(inode, atime);
    inode_set_mtime_to_ts(inode, mtime);
    inode_set_ctime_to_ts(inode, ctime);

    /* Copy VexFS-specific fields */
    vi->i_vector_count = le32_to_cpu(disk_inode->i_size);
    
    /* CRITICAL FIX: Calculate i_block_count by counting non-zero blocks */
    vi->i_block_count = 0;
    for (i = 0; i < VEXFS_DIRECT_BLOCKS; i++) {
        vi->i_blocks[i] = le32_to_cpu(disk_inode->i_block[i]);
        if (vi->i_blocks[i] != 0) {
            vi->i_block_count++;
        }
    }
    
    printk(KERN_DEBUG "VexFS: Loaded inode %lu: i_block_count=%u, i_blocks[0]=%u\n",
           ino, vi->i_block_count, vi->i_blocks[0]);

    /* DEADLOCK FIX: Removed mutex_init - VFS provides proper inode locking */

    brelse(bh);

    /* Set up inode operations based on file type */
    if (S_ISREG(inode->i_mode)) {
        /* Use enhanced file operations for proper disk persistence */
        extern const struct file_operations vexfs_file_ops_enhanced;
        extern const struct address_space_operations vexfs_aops_enhanced;
        
        inode->i_op = &vexfs_file_inode_ops;
        inode->i_fop = &vexfs_file_ops_enhanced;
        /* Initialize mapping properly before setting address space operations */
        mapping_set_gfp_mask(inode->i_mapping, GFP_KERNEL);
        inode->i_mapping->a_ops = &vexfs_aops_enhanced;
    } else if (S_ISDIR(inode->i_mode)) {
        inode->i_op = &vexfs_dir_inode_ops;
        /* Use our fixed directory operations that properly read from disk */
        extern const struct file_operations vexfs_dir_operations_fixed;
        inode->i_fop = &vexfs_dir_operations_fixed;
        inode->i_size = VEXFS_BLOCK_SIZE;
        /* CRITICAL FIX: Directories use empty_aops to prevent VFS deadlocks */
        mapping_set_gfp_mask(inode->i_mapping, GFP_KERNEL);
        inode->i_mapping->a_ops = &empty_aops;
    } else {
        /* Special files (devices, etc.) */
        init_special_inode(inode, inode->i_mode, 0);
    }

    unlock_new_inode(inode);
    return inode;
}

/*
 * Write inode to disk
 */
int vexfs_write_inode_to_disk(struct inode *inode)
{
    struct vexfs_inode_info *vi = VEXFS_I(inode);
    struct vexfs_sb_info *sbi = VEXFS_SB(inode->i_sb);
    struct buffer_head *bh;
    struct vexfs_inode *disk_inode;
    unsigned long block_num;
    unsigned long offset;
    int i;

    /* Calculate block and offset for this inode */
    /* Use same calculation as superblock.c for consistency */
    block_num = VEXFS_INODE_TABLE_BLOCK + ((inode->i_ino - 1) / VEXFS_INODES_PER_BLOCK);
    offset = ((inode->i_ino - 1) % VEXFS_INODES_PER_BLOCK) * sizeof(struct vexfs_inode);

    bh = sb_bread(inode->i_sb, block_num);
    if (!bh) {
        printk(KERN_ERR "VexFS: Failed to read block for inode %lu\n", 
               inode->i_ino);
        return -EIO;
    }

    disk_inode = (struct vexfs_inode *)(bh->b_data + offset);

    /* Copy VFS inode to disk inode - FIXED data types */
    disk_inode->i_mode = cpu_to_le16(inode->i_mode);
    disk_inode->i_links_count = cpu_to_le16(inode->i_nlink);  /* FIXED: use links_count */
    disk_inode->i_uid = cpu_to_le32(i_uid_read(inode));       /* FIXED: 32-bit UID */
    disk_inode->i_gid = cpu_to_le32(i_gid_read(inode));       /* FIXED: 32-bit GID */
    disk_inode->i_size = cpu_to_le64(inode->i_size);          /* FIXED: 64-bit size */
    disk_inode->i_blocks = cpu_to_le32(inode->i_blocks);
    
    disk_inode->i_atime = cpu_to_le32(inode_get_atime_sec(inode));
    disk_inode->i_mtime = cpu_to_le32(inode_get_mtime_sec(inode));
    disk_inode->i_ctime = cpu_to_le32(inode_get_ctime_sec(inode));

    /* Copy VexFS-specific fields */
    /* Note: i_blocks already set above with VFS blocks, using i_flags for vector count */
    disk_inode->i_flags = cpu_to_le32(vi->i_vector_count);
    
    for (i = 0; i < VEXFS_DIRECT_BLOCKS; i++) {
        disk_inode->i_block[i] = cpu_to_le32(vi->i_blocks[i]);
    }

    mark_buffer_dirty(bh);
    brelse(bh);

    return 0;
}

/*
 * Create a new file
 */
int vexfs_create(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode,
                 bool excl)
{
    struct inode *inode;
    struct vexfs_sb_info *sbi = VEXFS_SB(dir->i_sb);
    __u32 ino;
    int err;

    /* Allocate new inode number */
    if (vexfs_alloc_inode_num(dir->i_sb, &ino) != 0) {
        return -ENOSPC;
    }

    /* Create new inode */
    inode = new_inode(dir->i_sb);
    if (!inode) {
        vexfs_free_inode_num(dir->i_sb, ino);
        return -ENOMEM;
    }

    inode->i_ino = ino;
    vexfs_init_inode(inode, mode | S_IFREG);
    
    /* Mark inode as new */
    inode->i_state |= I_NEW;
    
    /* Use enhanced file operations for proper disk persistence */
    extern const struct file_operations vexfs_file_ops_enhanced;
    extern const struct address_space_operations vexfs_aops_enhanced;
    
    inode->i_op = &vexfs_file_inode_ops;
    inode->i_fop = &vexfs_file_ops_enhanced;
    /* Initialize mapping properly before setting address space operations */
    mapping_set_gfp_mask(inode->i_mapping, GFP_KERNEL);
    inode->i_mapping->a_ops = &vexfs_aops_enhanced;

    /* Insert into inode hash before writing to disk */
    insert_inode_hash(inode);
    
    /* Mark inode as initialized */
    unlock_new_inode(inode);

    /* Write inode to disk */
    err = vexfs_write_inode_to_disk(inode);
    if (err) {
        drop_nlink(inode);
        iput(inode);
        vexfs_free_inode_num(dir->i_sb, ino);
        return err;
    }

    /* Add directory entry */
    err = vexfs_add_dir_entry(dir, dentry, inode);
    if (err) {
        drop_nlink(inode);
        iput(inode);
        vexfs_free_inode_num(dir->i_sb, ino);
        return err;
    }

    d_instantiate(dentry, inode);
    return 0;
}

/*
 * Look up a file in a directory
 */
struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry,
                           unsigned int flags)
{
    struct inode *inode = NULL;
    unsigned long ino;

    if (dentry->d_name.len > VEXFS_MAX_NAME_LEN)
        return ERR_PTR(-ENAMETOOLONG);

    ino = vexfs_find_dir_entry(dir, &dentry->d_name);
    if (ino) {
        inode = vexfs_iget(dir->i_sb, ino);
        if (IS_ERR(inode))
            return ERR_CAST(inode);
    }

    return d_splice_alias(inode, dentry);
}

/*
 * Create a new directory
 */
int vexfs_mkdir(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode)
{
    struct inode *inode;
    __u32 ino;
    int err;

    /* Allocate new inode number */
    if (vexfs_alloc_inode_num(dir->i_sb, &ino) != 0) {
        return -ENOSPC;
    }

    /* Create new inode */
    inode = new_inode(dir->i_sb);
    if (!inode) {
        vexfs_free_inode_num(dir->i_sb, ino);
        return -ENOMEM;
    }

    inode->i_ino = ino;
    vexfs_init_inode(inode, mode | S_IFDIR);
    
    inode->i_op = &vexfs_dir_inode_ops;
    /* Use our fixed directory operations that properly read from disk */
    extern const struct file_operations vexfs_dir_operations_fixed;
    inode->i_fop = &vexfs_dir_operations_fixed;
    inode->i_size = VEXFS_BLOCK_SIZE;

    /* Increment link count for "." entry */
    inc_nlink(inode);
    /* Increment parent link count for ".." entry */
    inc_nlink(dir);

    /* Write inode to disk */
    err = vexfs_write_inode_to_disk(inode);
    if (err) {
        drop_nlink(dir);
        iput(inode);
        vexfs_free_inode_num(dir->i_sb, ino);
        return err;
    }

    /* Initialize directory with "." and ".." entries */
    err = vexfs_init_dir(inode, dir);
    if (err) {
        drop_nlink(dir);
        iput(inode);
        vexfs_free_inode_num(dir->i_sb, ino);
        return err;
    }

    /* Add directory entry to parent */
    err = vexfs_add_dir_entry(dir, dentry, inode);
    if (err) {
        drop_nlink(dir);
        iput(inode);
        vexfs_free_inode_num(dir->i_sb, ino);
        return err;
    }

    d_instantiate(dentry, inode);
    return 0;
}

/*
 * Remove a directory
 */
int vexfs_rmdir(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    int err;

    /* Check if directory is empty */
    if (!vexfs_dir_is_empty(inode))
        return -ENOTEMPTY;

    /* Remove directory entry from parent */
    err = vexfs_remove_dir_entry(dir, &dentry->d_name);
    if (err)
        return err;

    /* Update link counts */
    drop_nlink(inode);  /* Remove "." link */
    drop_nlink(dir);    /* Remove ".." link */
    drop_nlink(inode);  /* Remove parent's link to this directory */

    return 0;
}

/*
 * Remove a file
 */
int vexfs_unlink(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    int err;

    /* Remove directory entry */
    err = vexfs_remove_dir_entry(dir, &dentry->d_name);
    if (err)
        return err;

    /* Update link count */
    drop_nlink(inode);

    return 0;
}

/*
 * Rename a file or directory
 */
int vexfs_rename(struct mnt_idmap *idmap, struct inode *old_dir, struct dentry *old_dentry,
                struct inode *new_dir, struct dentry *new_dentry,
                unsigned int flags)
{
    struct inode *old_inode = d_inode(old_dentry);
    struct inode *new_inode = d_inode(new_dentry);
    int err;

    if (flags & ~RENAME_NOREPLACE)
        return -EINVAL;

    /* Remove old directory entry */
    err = vexfs_remove_dir_entry(old_dir, &old_dentry->d_name);
    if (err)
        return err;

    /* If target exists, remove it */
    if (new_inode) {
        if (S_ISDIR(new_inode->i_mode)) {
            if (!vexfs_dir_is_empty(new_inode))
                return -ENOTEMPTY;
            drop_nlink(new_dir);
        }
        drop_nlink(new_inode);
        err = vexfs_remove_dir_entry(new_dir, &new_dentry->d_name);
        if (err) {
            /* Try to restore old entry on failure */
            vexfs_add_dir_entry(old_dir, old_dentry, old_inode);
            return err;
        }
    }

    /* Add new directory entry */
    err = vexfs_add_dir_entry(new_dir, new_dentry, old_inode);
    if (err) {
        /* Restore old entry on failure */
        vexfs_add_dir_entry(old_dir, old_dentry, old_inode);
        return err;
    }

    /* Update directory link counts if moving a directory */
    if (S_ISDIR(old_inode->i_mode) && old_dir != new_dir) {
        drop_nlink(old_dir);
        inc_nlink(new_dir);
    }

    return 0;
}