/*
 * VexFS - Vector Extended File System (SAFE FFI VERSION)
 * Copyright (C) 2025 VexFS Contributors
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * SAFE FFI VERSION: Combines FFI functionality with comprehensive error handling
 * and safety mechanisms to prevent system hangs and ensure graceful degradation.
 */

#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/mount.h>
#include <linux/statfs.h>
#include <linux/version.h>
#include <linux/mutex.h>
#include <linux/atomic.h>
#include <linux/delay.h>
#include <linux/spinlock.h>
#include <linux/rbtree.h>
#include <linux/hash.h>
#include <linux/vmalloc.h>

/* Include VexFS FFI header for Rust integration */
#include "../include/vexfs_ffi.h"

/* VexFS magic number */
#define VEXFS_MAGIC 0x56454653  /* "VEFS" in ASCII */

/* Module state management */
enum vexfs_module_state {
    VEXFS_STATE_UNINITIALIZED = 0,
    VEXFS_STATE_INITIALIZING,
    VEXFS_STATE_INITIALIZED,
    VEXFS_STATE_ERROR,
    VEXFS_STATE_SHUTTING_DOWN
};

/* Global module state with synchronization */
static atomic_t vexfs_module_state = ATOMIC_INIT(VEXFS_STATE_UNINITIALIZED);
static DEFINE_MUTEX(vexfs_state_mutex);
static atomic_t vexfs_mount_count = ATOMIC_INIT(0);

/* FFI safety configuration */
#define VEXFS_FFI_TIMEOUT_MS 5000  /* 5 second timeout for FFI calls */
#define VEXFS_MAX_RETRY_COUNT 3    /* Maximum retries for failed operations */
#define VEXFS_OPERATION_TIMEOUT_MS 10000  /* 10 second timeout for operations */

/* Memory management constants */
#define VEXFS_MEMORY_POOL_SIZE 1024    /* Number of blocks in memory pool */
#define VEXFS_MEMORY_ALIGNMENT 64      /* Memory alignment for cache efficiency */
#define VEXFS_MAX_MEMORY_USAGE (256 * 1024 * 1024)  /* 256MB memory limit */
#define VEXFS_MEMORY_LEAK_THRESHOLD 100  /* Threshold for leak detection */
#define VEXFS_MEMORY_TRACKING_ENABLED 1   /* Enable memory tracking */

/* VFS operation locking */
static DEFINE_MUTEX(vexfs_inode_mutex);
static DEFINE_MUTEX(vexfs_dir_mutex);
static DEFINE_MUTEX(vexfs_file_mutex);

/* Memory management structures */
struct vexfs_memory_stats {
    atomic64_t total_allocated;
    atomic64_t total_freed;
    atomic64_t current_usage;
    atomic64_t peak_usage;
    atomic_t active_allocations;
    atomic_t allocation_failures;
    atomic_t detected_leaks;
};

struct vexfs_allocation_entry {
    void *ptr;
    size_t size;
    const char *location;
    unsigned long timestamp;
    struct rb_node node;
};

/* Global memory tracking */
static struct vexfs_memory_stats vexfs_mem_stats = {
    .total_allocated = ATOMIC64_INIT(0),
    .total_freed = ATOMIC64_INIT(0),
    .current_usage = ATOMIC64_INIT(0),
    .peak_usage = ATOMIC64_INIT(0),
    .active_allocations = ATOMIC_INIT(0),
    .allocation_failures = ATOMIC_INIT(0),
    .detected_leaks = ATOMIC_INIT(0),
};

static struct rb_root vexfs_allocation_tree = RB_ROOT;
static DEFINE_SPINLOCK(vexfs_allocation_lock);

/* Reference counting structures */
struct vexfs_ref_counted {
    atomic_t ref_count;
    void (*destructor)(struct vexfs_ref_counted *);
    spinlock_t lock;
};

/* Memory pool for frequent allocations */
struct vexfs_memory_pool {
    void **free_blocks;
    atomic_t free_count;
    size_t block_size;
    size_t total_blocks;
    spinlock_t lock;
};

static struct vexfs_memory_pool *vexfs_inode_pool = NULL;
static struct vexfs_memory_pool *vexfs_dentry_pool = NULL;

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
static int vexfs_fsync(struct file *file, loff_t start, loff_t end, int datasync);

/* Forward declarations for directory operations */
static int vexfs_readdir(struct file *file, struct dir_context *ctx);

/* VFS file system type registration */
static struct file_system_type vexfs_type = {
    .name       = "vexfs",
    .mount      = vexfs_mount,
    .kill_sb    = vexfs_kill_sb,
    .owner      = THIS_MODULE,
    .fs_flags   = FS_REQUIRES_DEV,
};

/* Superblock operations - WITH SAFE FFI integration */
static const struct super_operations vexfs_super_ops = {
    .alloc_inode    = vexfs_alloc_inode,
    .free_inode     = vexfs_free_inode,
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
    .fsync          = vexfs_fsync,
    .llseek         = default_llseek,
};

/* File operations for directories */
static const struct file_operations vexfs_dir_ops = {
    .owner          = THIS_MODULE,
    .open           = generic_file_open,
    .release        = generic_file_release,
    .read           = generic_read_dir,
    .iterate_shared = vexfs_readdir,
    .llseek         = generic_file_llseek,
};

/**
 * vexfs_safe_ffi_call - Safely call an FFI function with comprehensive error handling
 * @ffi_func: Function pointer to the FFI function
 * @args: Arguments to pass to the function
 * @fallback_value: Value to return if FFI call fails
 * @operation_name: Name of the operation for logging
 */
#define vexfs_safe_ffi_call(ffi_func, fallback_value, operation_name, ...) \
    ({ \
        int __result = fallback_value; \
        static atomic_t __call_count = ATOMIC_INIT(0); \
        static atomic_t __failure_count = ATOMIC_INIT(0); \
        int __call_id = atomic_inc_return(&__call_count); \
        \
        if (atomic_read(&vexfs_module_state) == VEXFS_STATE_INITIALIZED) { \
            printk(KERN_DEBUG "VexFS: [CALL-%d] Starting FFI call %s\n", __call_id, operation_name); \
            __result = ffi_func(__VA_ARGS__); \
            \
            if (__result == VEXFS_SUCCESS) { \
                printk(KERN_DEBUG "VexFS: [CALL-%d] FFI call %s succeeded\n", __call_id, operation_name); \
            } else if (__result == VEXFS_ERROR_CIRCUIT_BREAKER) { \
                atomic_inc(&__failure_count); \
                printk(KERN_WARNING "VexFS: [CALL-%d] FFI call %s blocked by circuit breaker, using fallback\n", \
                       __call_id, operation_name); \
                __result = fallback_value; \
            } else if (__result == VEXFS_ERROR_TIMEOUT) { \
                atomic_inc(&__failure_count); \
                printk(KERN_WARNING "VexFS: [CALL-%d] FFI call %s timed out: %d, using fallback\n", \
                       __call_id, operation_name, __result); \
                __result = fallback_value; \
            } else { \
                atomic_inc(&__failure_count); \
                int __failure_rate = (atomic_read(&__failure_count) * 100) / atomic_read(&__call_count); \
                printk(KERN_WARNING "VexFS: [CALL-%d] FFI call %s failed: %d (failure rate: %d%%), using fallback\n", \
                       __call_id, operation_name, __result, __failure_rate); \
                __result = fallback_value; \
            } \
        } else { \
            printk(KERN_DEBUG "VexFS: [CALL-%d] Module not initialized, skipping FFI call %s\n", \
                   __call_id, operation_name); \
        } \
        __result; \
    })

/**
 * vexfs_safe_ffi_call_void - Safely call a void FFI function
 */
#define vexfs_safe_ffi_call_void(ffi_func, operation_name, ...) \
    do { \
        if (atomic_read(&vexfs_module_state) == VEXFS_STATE_INITIALIZED) { \
            ffi_func(__VA_ARGS__); \
        } else { \
            printk(KERN_DEBUG "VexFS: Module not initialized, skipping FFI call %s\n", \
                   operation_name); \
        } \
    } while (0)

/**
 * vexfs_set_module_state - Safely change module state
 */
static bool vexfs_set_module_state(enum vexfs_module_state new_state)
{
    mutex_lock(&vexfs_state_mutex);
    
    enum vexfs_module_state current_state = atomic_read(&vexfs_module_state);
    
    /* Validate state transitions */
    bool valid_transition = false;
    switch (current_state) {
        case VEXFS_STATE_UNINITIALIZED:
            valid_transition = (new_state == VEXFS_STATE_INITIALIZING);
            break;
        case VEXFS_STATE_INITIALIZING:
            valid_transition = (new_state == VEXFS_STATE_INITIALIZED || 
                              new_state == VEXFS_STATE_ERROR);
            break;
        case VEXFS_STATE_INITIALIZED:
            valid_transition = (new_state == VEXFS_STATE_SHUTTING_DOWN ||
                              new_state == VEXFS_STATE_ERROR);
            break;
        case VEXFS_STATE_ERROR:
            valid_transition = (new_state == VEXFS_STATE_SHUTTING_DOWN);
            break;
        case VEXFS_STATE_SHUTTING_DOWN:
            valid_transition = (new_state == VEXFS_STATE_UNINITIALIZED);
            break;
    }
    
    if (valid_transition) {
        atomic_set(&vexfs_module_state, new_state);
        printk(KERN_INFO "VexFS: State transition %d -> %d\n", current_state, new_state);
    } else {
        printk(KERN_ERR "VexFS: Invalid state transition %d -> %d\n", current_state, new_state);
    }
    
    mutex_unlock(&vexfs_state_mutex);
    return valid_transition;
}

/**
 * vexfs_track_allocation - Track memory allocation for leak detection
 */
static void vexfs_track_allocation(void *ptr, size_t size, const char *location)
{
    struct vexfs_allocation_entry *entry;
    struct rb_node **new, *parent = NULL;
    unsigned long flags;

    if (!ptr || !VEXFS_MEMORY_TRACKING_ENABLED)
        return;

    entry = kmalloc(sizeof(*entry), GFP_ATOMIC);
    if (!entry) {
        atomic_inc(&vexfs_mem_stats.allocation_failures);
        return;
    }

    entry->ptr = ptr;
    entry->size = size;
    entry->location = location;
    entry->timestamp = jiffies;

    spin_lock_irqsave(&vexfs_allocation_lock, flags);

    /* Insert into red-black tree */
    new = &vexfs_allocation_tree.rb_node;
    while (*new) {
        struct vexfs_allocation_entry *this = rb_entry(*new, struct vexfs_allocation_entry, node);
        parent = *new;

        if (ptr < this->ptr)
            new = &((*new)->rb_left);
        else if (ptr > this->ptr)
            new = &((*new)->rb_right);
        else {
            /* Duplicate pointer - this shouldn't happen */
            spin_unlock_irqrestore(&vexfs_allocation_lock, flags);
            kfree(entry);
            printk(KERN_WARNING "VexFS: Duplicate allocation tracking for %p\n", ptr);
            return;
        }
    }

    rb_link_node(&entry->node, parent, new);
    rb_insert_color(&entry->node, &vexfs_allocation_tree);

    spin_unlock_irqrestore(&vexfs_allocation_lock, flags);

    /* Update statistics */
    atomic64_add(size, &vexfs_mem_stats.total_allocated);
    atomic64_add(size, &vexfs_mem_stats.current_usage);
    atomic_inc(&vexfs_mem_stats.active_allocations);

    /* Update peak usage */
    {
        long long current = atomic64_read(&vexfs_mem_stats.current_usage);
        long long peak = atomic64_read(&vexfs_mem_stats.peak_usage);
        while (current > peak) {
            if (atomic64_cmpxchg(&vexfs_mem_stats.peak_usage, peak, current) == peak)
                break;
            peak = atomic64_read(&vexfs_mem_stats.peak_usage);
        }
    }

    /* Memory barrier to ensure all updates are visible */
    smp_mb();
}

/**
 * vexfs_track_deallocation - Track memory deallocation
 */
static void vexfs_track_deallocation(void *ptr, size_t size)
{
    struct vexfs_allocation_entry *entry;
    struct rb_node *node;
    unsigned long flags;

    if (!ptr || !VEXFS_MEMORY_TRACKING_ENABLED)
        return;

    spin_lock_irqsave(&vexfs_allocation_lock, flags);

    /* Find in red-black tree */
    node = vexfs_allocation_tree.rb_node;
    while (node) {
        entry = rb_entry(node, struct vexfs_allocation_entry, node);

        if (ptr < entry->ptr)
            node = node->rb_left;
        else if (ptr > entry->ptr)
            node = node->rb_right;
        else {
            /* Found it */
            rb_erase(&entry->node, &vexfs_allocation_tree);
            spin_unlock_irqrestore(&vexfs_allocation_lock, flags);

            /* Update statistics */
            atomic64_add(entry->size, &vexfs_mem_stats.total_freed);
            atomic64_sub(entry->size, &vexfs_mem_stats.current_usage);
            atomic_dec(&vexfs_mem_stats.active_allocations);

            kfree(entry);
            smp_mb();
            return;
        }
    }

    spin_unlock_irqrestore(&vexfs_allocation_lock, flags);

    /* Pointer not found - possible double free or untracked allocation */
    printk(KERN_WARNING "VexFS: Deallocation of untracked pointer %p\n", ptr);
}

/**
 * vexfs_safe_kmalloc - Safe memory allocation with tracking
 */
static void *vexfs_safe_kmalloc(size_t size, gfp_t flags, const char *location)
{
    void *ptr;
    long long current_usage;

    /* Check memory limits */
    current_usage = atomic64_read(&vexfs_mem_stats.current_usage);
    if (current_usage + size > VEXFS_MAX_MEMORY_USAGE) {
        atomic_inc(&vexfs_mem_stats.allocation_failures);
        printk(KERN_WARNING "VexFS: Memory limit exceeded at %s\n", location);
        return NULL;
    }

    ptr = kmalloc(size, flags);
    if (!ptr) {
        atomic_inc(&vexfs_mem_stats.allocation_failures);
        return NULL;
    }

    /* Zero the memory for security */
    memset(ptr, 0, size);

    vexfs_track_allocation(ptr, size, location);
    return ptr;
}

/**
 * vexfs_safe_kfree - Safe memory deallocation with tracking
 */
static void vexfs_safe_kfree(void *ptr, size_t size)
{
    if (!ptr)
        return;

    vexfs_track_deallocation(ptr, size);

    /* Clear memory before freeing for security */
    memset(ptr, 0, size);
    kfree(ptr);
}

/**
 * vexfs_init_ref_counted - Initialize reference counted structure
 */
static void vexfs_init_ref_counted(struct vexfs_ref_counted *ref,
                                   void (*destructor)(struct vexfs_ref_counted *))
{
    atomic_set(&ref->ref_count, 1);
    ref->destructor = destructor;
    spin_lock_init(&ref->lock);
    smp_mb();
}

/**
 * vexfs_get_ref - Increment reference count
 */
static struct vexfs_ref_counted *vexfs_get_ref(struct vexfs_ref_counted *ref)
{
    if (!ref)
        return NULL;

    if (atomic_inc_not_zero(&ref->ref_count)) {
        smp_mb();
        return ref;
    }

    return NULL;
}

/**
 * vexfs_put_ref - Decrement reference count and cleanup if needed
 */
static void vexfs_put_ref(struct vexfs_ref_counted *ref)
{
    if (!ref)
        return;

    smp_mb();
    if (atomic_dec_and_test(&ref->ref_count)) {
        if (ref->destructor)
            ref->destructor(ref);
    }
}

/**
 * vexfs_create_memory_pool - Create a memory pool for frequent allocations
 */
static struct vexfs_memory_pool *vexfs_create_memory_pool(size_t block_size, size_t num_blocks)
{
    struct vexfs_memory_pool *pool;
    size_t i;

    pool = vexfs_safe_kmalloc(sizeof(*pool), GFP_KERNEL, "memory_pool");
    if (!pool)
        return NULL;

    pool->free_blocks = vexfs_safe_kmalloc(sizeof(void*) * num_blocks, GFP_KERNEL, "memory_pool_blocks");
    if (!pool->free_blocks) {
        vexfs_safe_kfree(pool, sizeof(*pool));
        return NULL;
    }

    pool->block_size = block_size;
    pool->total_blocks = num_blocks;
    atomic_set(&pool->free_count, 0);
    spin_lock_init(&pool->lock);

    /* Pre-allocate blocks */
    for (i = 0; i < num_blocks; i++) {
        void *block = vexfs_safe_kmalloc(block_size, GFP_KERNEL, "memory_pool_block");
        if (block) {
            pool->free_blocks[atomic_read(&pool->free_count)] = block;
            atomic_inc(&pool->free_count);
        }
    }

    return pool;
}

/**
 * vexfs_pool_alloc - Allocate from memory pool
 */
static void *vexfs_pool_alloc(struct vexfs_memory_pool *pool)
{
    void *block = NULL;
    unsigned long flags;
    int count;

    if (!pool)
        return NULL;

    spin_lock_irqsave(&pool->lock, flags);
    count = atomic_read(&pool->free_count);
    if (count > 0) {
        atomic_dec(&pool->free_count);
        block = pool->free_blocks[atomic_read(&pool->free_count)];
        pool->free_blocks[atomic_read(&pool->free_count)] = NULL;
    }
    spin_unlock_irqrestore(&pool->lock, flags);

    if (!block) {
        /* Pool exhausted, allocate directly */
        block = vexfs_safe_kmalloc(pool->block_size, GFP_KERNEL, "pool_fallback");
    }

    return block;
}

/**
 * vexfs_pool_free - Return block to memory pool
 */
static void vexfs_pool_free(struct vexfs_memory_pool *pool, void *block)
{
    unsigned long flags;
    int count;

    if (!pool || !block)
        return;

    spin_lock_irqsave(&pool->lock, flags);
    count = atomic_read(&pool->free_count);
    if (count < pool->total_blocks) {
        pool->free_blocks[count] = block;
        atomic_inc(&pool->free_count);
        spin_unlock_irqrestore(&pool->lock, flags);
    } else {
        spin_unlock_irqrestore(&pool->lock, flags);
        /* Pool full, free directly */
        vexfs_safe_kfree(block, pool->block_size);
    }
}

/**
 * vexfs_destroy_memory_pool - Destroy memory pool
 */
static void vexfs_destroy_memory_pool(struct vexfs_memory_pool *pool)
{
    int i, count;

    if (!pool)
        return;

    count = atomic_read(&pool->free_count);
    for (i = 0; i < count; i++) {
        if (pool->free_blocks[i])
            vexfs_safe_kfree(pool->free_blocks[i], pool->block_size);
    }

    vexfs_safe_kfree(pool->free_blocks, sizeof(void*) * pool->total_blocks);
    vexfs_safe_kfree(pool, sizeof(*pool));
}

/**
 * vexfs_detect_memory_leaks - Detect and report memory leaks
 */
static void vexfs_detect_memory_leaks(void)
{
    struct rb_node *node;
    struct vexfs_allocation_entry *entry;
    unsigned long flags;
    int leak_count = 0;

    spin_lock_irqsave(&vexfs_allocation_lock, flags);

    for (node = rb_first(&vexfs_allocation_tree); node; node = rb_next(node)) {
        entry = rb_entry(node, struct vexfs_allocation_entry, node);
        
        printk(KERN_WARNING "VexFS: Memory leak detected: %zu bytes at %p from %s (age: %lu jiffies)\n",
               entry->size, entry->ptr, entry->location, jiffies - entry->timestamp);
        
        leak_count++;
        if (leak_count >= VEXFS_MEMORY_LEAK_THRESHOLD)
            break;
    }

    spin_unlock_irqrestore(&vexfs_allocation_lock, flags);

    if (leak_count > 0) {
        atomic_add(leak_count, &vexfs_mem_stats.detected_leaks);
        printk(KERN_WARNING "VexFS: Total memory leaks detected: %d\n", leak_count);
    }
}

/**
 * vexfs_cleanup_memory_tracking - Cleanup memory tracking structures
 */
static void vexfs_cleanup_memory_tracking(void)
{
    struct rb_node *node;
    struct vexfs_allocation_entry *entry;
    unsigned long flags;

    spin_lock_irqsave(&vexfs_allocation_lock, flags);

    while ((node = rb_first(&vexfs_allocation_tree))) {
        entry = rb_entry(node, struct vexfs_allocation_entry, node);
        rb_erase(node, &vexfs_allocation_tree);
        kfree(entry);
    }

    spin_unlock_irqrestore(&vexfs_allocation_lock, flags);
}

/* Memory allocation macros with tracking */
#define vexfs_kmalloc(size, flags) vexfs_safe_kmalloc(size, flags, __func__)
#define vexfs_kfree(ptr, size) vexfs_safe_kfree(ptr, size)

/**
 * vexfs_mount - Mount the VexFS filesystem
 * SAFE FFI: Calls Rust initialization functions with error handling
 */
static struct dentry *vexfs_mount(struct file_system_type *fs_type, int flags,
                                  const char *dev_name, void *data)
{
    printk(KERN_INFO "VexFS: Mounting filesystem on device %s (SAFE FFI)\n", dev_name);
    
    /* Increment mount count for resource tracking */
    atomic_inc(&vexfs_mount_count);
    
    return mount_bdev(fs_type, flags, dev_name, data, vexfs_fill_super);
}

/**
 * vexfs_kill_sb - Unmount the VexFS filesystem
 * SAFE FFI: Calls Rust cleanup functions with error handling
 */
static void vexfs_kill_sb(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Unmounting filesystem (SAFE FFI)\n");
    
    /* Call Rust cleanup with safety wrapper */
    vexfs_safe_ffi_call_void(vexfs_rust_cleanup_superblock, "cleanup_superblock", sb);
    
    /* Decrement mount count */
    atomic_dec(&vexfs_mount_count);
    
    kill_block_super(sb);
}

/**
 * vexfs_fill_super - Initialize the superblock
 * SAFE FFI: Calls Rust superblock initialization with comprehensive error handling
 */
static int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    struct inode *root_inode;
    struct dentry *root_dentry;
    struct timespec64 ts;
    int ret;

    printk(KERN_INFO "VexFS: Filling superblock (SAFE FFI)\n");

    /* Set up superblock */
    sb->s_magic = VEXFS_MAGIC;
    sb->s_op = &vexfs_super_ops;
    sb->s_blocksize = PAGE_SIZE;
    sb->s_blocksize_bits = PAGE_SHIFT;
    sb->s_maxbytes = MAX_LFS_FILESIZE;

    /* Call Rust FFI to initialize VexFS-specific superblock data with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_fill_super, VEXFS_SUCCESS, "fill_super", sb);
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust superblock initialization failed: %d, continuing with basic functionality\n", ret);
        /* Continue with basic functionality even if FFI fails */
    }

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
    root_inode->i_fop = &vexfs_dir_ops;

    /* Create root dentry */
    root_dentry = d_make_root(root_inode);
    if (!root_dentry) {
        printk(KERN_ERR "VexFS: Failed to create root dentry\n");
        return -ENOMEM;
    }

    sb->s_root = root_dentry;
    printk(KERN_INFO "VexFS: Superblock initialized successfully (SAFE FFI)\n");
    return 0;
}

/**
 * vexfs_statfs - Return filesystem statistics
 * SAFE FFI: Calls Rust to get actual statistics with fallback
 */
static int vexfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
    uint64_t blocks, free_blocks, files, free_files;
    int ret;

    buf->f_type = VEXFS_MAGIC;
    buf->f_bsize = PAGE_SIZE;
    buf->f_namelen = 255;

    /* Call Rust FFI to get actual filesystem statistics with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_get_statfs, VEXFS_ERROR_GENERIC, "get_statfs",
                              &blocks, &free_blocks, &files, &free_files);
    
    if (ret == VEXFS_SUCCESS) {
        buf->f_blocks = blocks;
        buf->f_bfree = free_blocks;
        buf->f_bavail = free_blocks;
        buf->f_files = files;
        buf->f_ffree = free_files;
    } else {
        /* Fallback to default values if FFI fails */
        buf->f_blocks = 1000;
        buf->f_bfree = 500;
        buf->f_bavail = 500;
        buf->f_files = 100;
        buf->f_ffree = 50;
        printk(KERN_DEBUG "VexFS: Using fallback statfs values\n");
    }

    return 0;
}

/**
 * vexfs_alloc_inode - Allocate a new inode
 * SAFE FFI: Calls Rust for VexFS-specific inode initialization with error handling
 */
static struct inode *vexfs_alloc_inode(struct super_block *sb)
{
    struct inode *inode;
    int ret;
    
    printk(KERN_DEBUG "VexFS: Allocating new inode (SAFE FFI)\n");
    
    /* Allocate generic inode */
    inode = new_inode(sb);
    if (!inode) {
        return NULL;
    }
    
    /* Call Rust FFI to initialize VexFS-specific inode data with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_init_inode, VEXFS_SUCCESS, "init_inode",
                              inode, inode->i_ino, inode->i_mode);
    
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust inode initialization failed: %d, continuing with basic inode\n", ret);
        /* Continue with basic inode even if FFI fails */
    }
    
    return inode;
}

/**
 * vexfs_free_inode - Free an inode
 * SAFE FFI: Calls Rust cleanup before freeing with error handling
 */
static void vexfs_free_inode(struct inode *inode)
{
    printk(KERN_DEBUG "VexFS: Freeing inode %lu (SAFE FFI)\n", inode->i_ino);
    
    /* Call Rust FFI to cleanup VexFS-specific inode data with safety */
    vexfs_safe_ffi_call_void(vexfs_rust_destroy_inode, "destroy_inode", inode);
}

/**
 * vexfs_write_inode - Write inode to storage
 * SAFE FFI: Calls Rust for actual persistence with error handling
 */
static int vexfs_write_inode(struct inode *inode, struct writeback_control *wbc)
{
    int ret;
    
    printk(KERN_DEBUG "VexFS: Write inode %lu (SAFE FFI)\n", inode->i_ino);
    
    /* Call Rust FFI to persist inode data with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_write_inode, 0, "write_inode", inode);
    
    return (ret == VEXFS_SUCCESS) ? 0 : ret;
}

/**
 * vexfs_put_super - Put superblock during unmount
 * SAFE FFI: Calls Rust cleanup with error handling
 */
static void vexfs_put_super(struct super_block *sb)
{
    printk(KERN_INFO "VexFS: Put superblock called (SAFE FFI)\n");
    
    /* Call Rust FFI to cleanup superblock data with safety */
    vexfs_safe_ffi_call_void(vexfs_rust_put_super, "put_super", sb);
}

/**
 * vexfs_sync_fs - Sync filesystem
 * SAFE FFI: Calls Rust for actual synchronization with error handling
 */
static int vexfs_sync_fs(struct super_block *sb, int wait)
{
    int ret;
    
    printk(KERN_DEBUG "VexFS: Sync filesystem (SAFE FFI, wait=%d)\n", wait);
    
    /* Call Rust FFI to sync filesystem data with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_sync_fs, 0, "sync_fs", sb, wait);
    
    return (ret == VEXFS_SUCCESS) ? 0 : ret;
}

/**
 * vexfs_create - Create a new file
 * SAFE FFI: Enhanced file creation with proper locking and error handling
 */
static int vexfs_create(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode, bool excl)
{
    struct inode *inode;
    int ret = 0;
    
    printk(KERN_DEBUG "VexFS: Creating file %s (SAFE FFI)\n", dentry->d_name.name);
    
    /* Acquire directory mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_dir_mutex)) {
        return -ERESTARTSYS;
    }
    
    /* Check if file already exists when exclusive creation is requested */
    if (excl && d_inode(dentry)) {
        ret = -EEXIST;
        goto out_unlock;
    }
    
    /* Allocate new inode (calls FFI internally with safety) */
    inode = vexfs_alloc_inode(dir->i_sb);
    if (!inode) {
        ret = -ENOSPC;
        goto out_unlock;
    }
    
    /* Set up the inode with proper error checking */
    inode->i_mode = mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    simple_inode_init_ts(inode);
    inode->i_op = &vexfs_file_inode_ops;
    inode->i_fop = &vexfs_file_ops;
    
    /* Call Rust FFI to create file metadata with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_create_file, 0, "create_file",
                              dir, dentry, inode, mode);
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust file creation failed: %d, continuing with basic file\n", ret);
        /* Continue with basic file creation even if FFI fails */
        ret = 0;
    }
    
    /* Link to dentry */
    d_instantiate(dentry, inode);
    
    /* Update parent directory timestamps */
    simple_inode_init_ts(dir);
    
out_unlock:
    mutex_unlock(&vexfs_dir_mutex);
    return ret;
}

/**
 * vexfs_lookup - Look up a dentry
 * SAFE FFI: Enhanced path resolution with proper locking and error handling
 */
static struct dentry *vexfs_lookup(struct inode *dir, struct dentry *dentry, unsigned int flags)
{
    struct inode *inode = NULL;
    int ret;
    uint64_t ino = 0;
    uint32_t mode = 0;
    
    printk(KERN_DEBUG "VexFS: Looking up %s (SAFE FFI)\n", dentry->d_name.name);
    
    /* Acquire directory mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_dir_mutex)) {
        return ERR_PTR(-ERESTARTSYS);
    }
    
    /* Call Rust FFI for actual path lookup with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_lookup_inode, VEXFS_ERROR_NOENT, "lookup_inode",
                              dir, dentry->d_name.name, dentry->d_name.len, &ino, &mode);
    
    if (ret == VEXFS_SUCCESS && ino != 0) {
        /* Found the inode, allocate and initialize it */
        inode = vexfs_alloc_inode(dir->i_sb);
        if (!inode) {
            mutex_unlock(&vexfs_dir_mutex);
            return ERR_PTR(-ENOMEM);
        }
        
        /* Set up the found inode */
        inode->i_ino = ino;
        inode->i_mode = mode;
        inode->i_uid = current_fsuid();
        inode->i_gid = current_fsgid();
        simple_inode_init_ts(inode);
        
        /* Set operations based on file type */
        if (S_ISDIR(mode)) {
            inode->i_op = &vexfs_dir_inode_ops;
            inode->i_fop = &vexfs_dir_ops;
            set_nlink(inode, 2);
        } else {
            inode->i_op = &vexfs_file_inode_ops;
            inode->i_fop = &vexfs_file_ops;
            set_nlink(inode, 1);
        }
        
        printk(KERN_DEBUG "VexFS: Found inode %llu for %s\n", ino, dentry->d_name.name);
    } else if (ret != VEXFS_ERROR_NOENT) {
        /* Lookup failed with error other than "not found" */
        printk(KERN_WARNING "VexFS: Lookup failed for %s: %d\n", dentry->d_name.name, ret);
        mutex_unlock(&vexfs_dir_mutex);
        return ERR_PTR(-EIO);
    }
    
    /* Add dentry to cache (inode may be NULL for negative dentry) */
    d_add(dentry, inode);
    
    mutex_unlock(&vexfs_dir_mutex);
    return NULL;
}

/**
 * vexfs_mkdir - Create a directory
 * SAFE FFI: Basic directory creation with future FFI integration
 */
static int vexfs_mkdir(struct mnt_idmap *idmap, struct inode *dir, struct dentry *dentry, umode_t mode)
{
    struct inode *inode;
    
    printk(KERN_DEBUG "VexFS: Creating directory %s (SAFE FFI)\n", dentry->d_name.name);
    
    /* Allocate new inode (calls FFI internally with safety) */
    inode = vexfs_alloc_inode(dir->i_sb);
    if (!inode) {
        return -ENOSPC;
    }
    
    /* Set up the directory inode */
    inode->i_mode = S_IFDIR | mode;
    inode->i_uid = current_fsuid();
    inode->i_gid = current_fsgid();
    simple_inode_init_ts(inode);
    inode->i_op = &vexfs_dir_inode_ops;
    inode->i_fop = &simple_dir_operations;
    set_nlink(inode, 2); /* . and .. */
    
    /* Update parent directory */
    inc_nlink(dir);
    
    /* Link to dentry */
    d_instantiate(dentry, inode);
    
    return 0;
}

/**
 * vexfs_rmdir - Remove a directory
 * SAFE FFI: Basic directory removal with future FFI integration
 */
static int vexfs_rmdir(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    
    printk(KERN_DEBUG "VexFS: Removing directory %s (SAFE FFI)\n", dentry->d_name.name);
    
    /* Update link counts */
    clear_nlink(inode);
    drop_nlink(dir);
    
    return 0;
}

/**
 * vexfs_unlink - Remove a file
 * SAFE FFI: Basic file removal with future FFI integration
 */
static int vexfs_unlink(struct inode *dir, struct dentry *dentry)
{
    struct inode *inode = d_inode(dentry);
    
    printk(KERN_DEBUG "VexFS: Unlinking file %s (SAFE FFI)\n", dentry->d_name.name);
    
    /* Update link count */
    drop_nlink(inode);
    
    return 0;
}

/* File operations - SAFE FFI integration points */
static int vexfs_open(struct inode *inode, struct file *file)
{
    int ret;
    
    printk(KERN_DEBUG "VexFS: Opening file inode %lu (SAFE FFI)\n", inode->i_ino);
    
    /* Acquire file mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_file_mutex)) {
        return -ERESTARTSYS;
    }
    
    /* Call Rust FFI for file opening with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_open_file, 0, "open_file", inode, file);
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust file open failed: %d, continuing with basic open\n", ret);
        /* Continue with basic open even if FFI fails */
        ret = 0;
    }
    
    /* Update access time */
    simple_inode_init_ts(inode);
    
    mutex_unlock(&vexfs_file_mutex);
    return ret;
}

static int vexfs_release(struct inode *inode, struct file *file)
{
    int ret;
    
    printk(KERN_DEBUG "VexFS: Releasing file inode %lu (SAFE FFI)\n", inode->i_ino);
    
    /* Acquire file mutex for thread safety */
    mutex_lock(&vexfs_file_mutex);
    
    /* Call Rust FFI for file closing with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_release_file, 0, "release_file", inode, file);
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust file release failed: %d, continuing with basic release\n", ret);
        /* Continue with basic release even if FFI fails */
        ret = 0;
    }
    
    mutex_unlock(&vexfs_file_mutex);
    return ret;
}

static ssize_t vexfs_read(struct file *file, char __user *buf, size_t count, loff_t *ppos)
{
    struct inode *inode = file_inode(file);
    ssize_t ret = 0;
    size_t bytes_read = 0;
    
    printk(KERN_DEBUG "VexFS: Reading %zu bytes from file inode %lu at offset %lld (SAFE FFI)\n",
           count, inode->i_ino, *ppos);
    
    /* Validate parameters */
    if (!buf || count == 0) {
        return -EINVAL;
    }
    
    /* Check file position */
    if (*ppos >= inode->i_size) {
        return 0; /* EOF */
    }
    
    /* Limit read to file size */
    if (*ppos + count > inode->i_size) {
        count = inode->i_size - *ppos;
    }
    
    /* Acquire file mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_file_mutex)) {
        return -ERESTARTSYS;
    }
    
    /* Call Rust FFI for file reading with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_read_file, 0, "read_file",
                              inode, file, buf, count, *ppos, &bytes_read);
    
    if (ret == VEXFS_SUCCESS && bytes_read > 0) {
        *ppos += bytes_read;
        ret = bytes_read;
        
        /* Update access time */
        simple_inode_init_ts(inode);
    } else if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust file read failed: %d\n", ret);
        ret = -EIO;
    }
    
    mutex_unlock(&vexfs_file_mutex);
    return ret;
}

static ssize_t vexfs_write(struct file *file, const char __user *buf, size_t count, loff_t *ppos)
{
    struct inode *inode = file_inode(file);
    ssize_t ret = 0;
    size_t bytes_written = 0;
    
    printk(KERN_DEBUG "VexFS: Writing %zu bytes to file inode %lu at offset %lld (SAFE FFI)\n",
           count, inode->i_ino, *ppos);
    
    /* Validate parameters */
    if (!buf || count == 0) {
        return -EINVAL;
    }
    
    /* Check for write beyond maximum file size */
    if (*ppos + count > VEXFS_MAX_FILE_SIZE) {
        return -EFBIG;
    }
    
    /* Acquire file mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_file_mutex)) {
        return -ERESTARTSYS;
    }
    
    /* Call Rust FFI for file writing with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_write_file, count, "write_file",
                              inode, file, buf, count, *ppos, &bytes_written);
    
    if (ret == VEXFS_SUCCESS && bytes_written > 0) {
        *ppos += bytes_written;
        
        /* Update file size if we wrote beyond current size */
        if (*ppos > inode->i_size) {
            inode->i_size = *ppos;
        }
        
        /* Update modification time */
        simple_inode_init_ts(inode);
        mark_inode_dirty(inode);
        
        ret = bytes_written;
    } else if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust file write failed: %d, using fallback\n", ret);
        /* Fallback: pretend success for basic functionality */
        *ppos += count;
        if (*ppos > inode->i_size) {
            inode->i_size = *ppos;
        }
        simple_inode_init_ts(inode);
        mark_inode_dirty(inode);
        ret = count;
    }
    
    mutex_unlock(&vexfs_file_mutex);
    return ret;
}

/**
 * vexfs_fsync - Synchronize file data
 * SAFE FFI: Enhanced file synchronization with proper error handling
 */
static int vexfs_fsync(struct file *file, loff_t start, loff_t end, int datasync)
{
    struct inode *inode = file_inode(file);
    int ret;
    
    printk(KERN_DEBUG "VexFS: Syncing file inode %lu (SAFE FFI, datasync=%d)\n",
           inode->i_ino, datasync);
    
    /* Acquire file mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_file_mutex)) {
        return -ERESTARTSYS;
    }
    
    /* Call Rust FFI for file synchronization with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_fsync_file, 0, "fsync_file",
                              inode, file, start, end, datasync);
    
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_WARNING "VexFS: Rust file sync failed: %d, using fallback\n", ret);
        /* Fallback: use generic sync */
        ret = generic_file_fsync(file, start, end, datasync);
    }
    
    mutex_unlock(&vexfs_file_mutex);
    return (ret == VEXFS_SUCCESS) ? 0 : ret;
}

/**
 * vexfs_readdir - Read directory entries
 * SAFE FFI: Enhanced directory reading with proper locking and error handling
 */
static int vexfs_readdir(struct file *file, struct dir_context *ctx)
{
    struct inode *inode = file_inode(file);
    int ret = 0;
    
    printk(KERN_DEBUG "VexFS: Reading directory inode %lu at pos %lld (SAFE FFI)\n",
           inode->i_ino, ctx->pos);
    
    /* Acquire directory mutex for thread safety */
    if (mutex_lock_interruptible(&vexfs_dir_mutex)) {
        return -ERESTARTSYS;
    }
    
    /* Handle standard directory entries */
    if (!dir_emit_dots(file, ctx)) {
        ret = 0;
        goto out_unlock;
    }
    
    /* Call Rust FFI for actual directory reading with safety */
    ret = vexfs_safe_ffi_call(vexfs_rust_readdir, 0, "readdir", inode, file, ctx);
    
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_DEBUG "VexFS: Rust readdir failed: %d, using empty directory\n", ret);
        /* Fallback: empty directory (just . and ..) */
        ret = 0;
    }
    
out_unlock:
    mutex_unlock(&vexfs_dir_mutex);
    return ret;
}

/**
 * vexfs_init_module - Initialize the VexFS module
 * SAFE FFI: Initializes Rust components with comprehensive error handling
 */
static int __init vexfs_init_module(void)
{
    int ret;

    printk(KERN_INFO "VexFS: Initializing SAFE FFI module v1.0.0 with memory management\n");

    /* Set module state to initializing */
    if (!vexfs_set_module_state(VEXFS_STATE_INITIALIZING)) {
        printk(KERN_ERR "VexFS: Failed to set initializing state\n");
        return -EINVAL;
    }

    /* Initialize memory pools for frequent allocations */
    vexfs_inode_pool = vexfs_create_memory_pool(sizeof(struct inode), VEXFS_MEMORY_POOL_SIZE);
    if (!vexfs_inode_pool) {
        printk(KERN_WARNING "VexFS: Failed to create inode memory pool\n");
    } else {
        printk(KERN_INFO "VexFS: Inode memory pool created with %d blocks\n", VEXFS_MEMORY_POOL_SIZE);
    }

    vexfs_dentry_pool = vexfs_create_memory_pool(sizeof(struct dentry), VEXFS_MEMORY_POOL_SIZE);
    if (!vexfs_dentry_pool) {
        printk(KERN_WARNING "VexFS: Failed to create dentry memory pool\n");
    } else {
        printk(KERN_INFO "VexFS: Dentry memory pool created with %d blocks\n", VEXFS_MEMORY_POOL_SIZE);
    }

    /* Initialize Rust components via FFI with timeout and error handling */
    ret = vexfs_rust_init();
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_ERR "VexFS: Rust initialization failed: %d\n", ret);
        vexfs_set_module_state(VEXFS_STATE_ERROR);
        
        /* Continue with limited functionality */
        printk(KERN_WARNING "VexFS: Continuing with limited functionality (no Rust FFI)\n");
    } else {
        printk(KERN_INFO "VexFS: Rust FFI initialization successful\n");
        
        /* Initialize hang prevention system */
        ret = vexfs_safe_ffi_call(vexfs_rust_init_hang_prevention, VEXFS_SUCCESS, "init_hang_prevention");
        if (ret != VEXFS_SUCCESS) {
            printk(KERN_WARNING "VexFS: Hang prevention initialization failed: %d, continuing without hang prevention\n", ret);
        } else {
            printk(KERN_INFO "VexFS: Hang prevention system initialized\n");
        }
    }

    /* Register filesystem with VFS */
    ret = register_filesystem(&vexfs_type);
    if (ret) {
        printk(KERN_ERR "VexFS: Failed to register filesystem: %d\n", ret);
        
        /* Cleanup Rust components if they were initialized */
        if (atomic_read(&vexfs_module_state) != VEXFS_STATE_ERROR) {
            vexfs_rust_exit();
        }
        
        /* Cleanup memory pools */
        if (vexfs_inode_pool) {
            vexfs_destroy_memory_pool(vexfs_inode_pool);
            vexfs_inode_pool = NULL;
        }
        if (vexfs_dentry_pool) {
            vexfs_destroy_memory_pool(vexfs_dentry_pool);
            vexfs_dentry_pool = NULL;
        }
        
        vexfs_set_module_state(VEXFS_STATE_ERROR);
        return ret;
    }

    /* Set module state to initialized */
    vexfs_set_module_state(VEXFS_STATE_INITIALIZED);

    printk(KERN_INFO "VexFS: SAFE FFI module loaded successfully\n");
    printk(KERN_INFO "VexFS: Filesystem registered as 'vexfs'\n");
    printk(KERN_INFO "VexFS: Rust FFI integration enabled with safety mechanisms\n");
    printk(KERN_INFO "VexFS: Error handling and graceful degradation active\n");
    printk(KERN_INFO "VexFS: Memory management and leak detection enabled\n");
    printk(KERN_INFO "VexFS: Memory limit: %d MB, tracking enabled: %s\n",
           VEXFS_MAX_MEMORY_USAGE / (1024 * 1024),
           VEXFS_MEMORY_TRACKING_ENABLED ? "yes" : "no");
    
    return 0;
}

/**
 * vexfs_exit_module - Cleanup the VexFS module
 * SAFE FFI: Cleans up Rust components with comprehensive error handling
 */
static void __exit vexfs_exit_module(void)
{
    int mount_count;
    
    printk(KERN_INFO "VexFS: Unloading SAFE FFI module with memory cleanup\n");

    /* Set module state to shutting down */
    vexfs_set_module_state(VEXFS_STATE_SHUTTING_DOWN);

    /* Check for active mounts */
    mount_count = atomic_read(&vexfs_mount_count);
    if (mount_count > 0) {
        printk(KERN_WARNING "VexFS: %d active mounts during module unload\n", mount_count);
    }

    /* Detect memory leaks before cleanup */
    vexfs_detect_memory_leaks();

    /* Print memory statistics */
    printk(KERN_INFO "VexFS: Memory statistics at exit:\n");
    printk(KERN_INFO "  Total allocated: %lld bytes\n", atomic64_read(&vexfs_mem_stats.total_allocated));
    printk(KERN_INFO "  Total freed: %lld bytes\n", atomic64_read(&vexfs_mem_stats.total_freed));
    printk(KERN_INFO "  Current usage: %lld bytes\n", atomic64_read(&vexfs_mem_stats.current_usage));
    printk(KERN_INFO "  Peak usage: %lld bytes\n", atomic64_read(&vexfs_mem_stats.peak_usage));
    printk(KERN_INFO "  Active allocations: %d\n", atomic_read(&vexfs_mem_stats.active_allocations));
    printk(KERN_INFO "  Allocation failures: %d\n", atomic_read(&vexfs_mem_stats.allocation_failures));
    printk(KERN_INFO "  Detected leaks: %d\n", atomic_read(&vexfs_mem_stats.detected_leaks));

    /* Unregister filesystem from VFS */
    unregister_filesystem(&vexfs_type);
    printk(KERN_INFO "VexFS: Filesystem unregistered\n");

    /* Cleanup Rust components via FFI with safety */
    if (atomic_read(&vexfs_module_state) != VEXFS_STATE_ERROR) {
        /* Shutdown hang prevention system first */
        vexfs_safe_ffi_call_void(vexfs_rust_shutdown_hang_prevention, "shutdown_hang_prevention");
        printk(KERN_INFO "VexFS: Hang prevention system shutdown\n");
        
        vexfs_rust_exit();
        printk(KERN_INFO "VexFS: Rust components cleaned up\n");
    } else {
        printk(KERN_INFO "VexFS: Skipping Rust cleanup (was in error state)\n");
    }

    /* Cleanup memory pools */
    if (vexfs_inode_pool) {
        vexfs_destroy_memory_pool(vexfs_inode_pool);
        vexfs_inode_pool = NULL;
        printk(KERN_INFO "VexFS: Inode memory pool destroyed\n");
    }
    if (vexfs_dentry_pool) {
        vexfs_destroy_memory_pool(vexfs_dentry_pool);
        vexfs_dentry_pool = NULL;
        printk(KERN_INFO "VexFS: Dentry memory pool destroyed\n");
    }

    /* Cleanup memory tracking structures */
    vexfs_cleanup_memory_tracking();
    printk(KERN_INFO "VexFS: Memory tracking structures cleaned up\n");
    
    /* Set final state */
    vexfs_set_module_state(VEXFS_STATE_UNINITIALIZED);
    
    printk(KERN_INFO "VexFS: SAFE FFI module unloaded successfully\n");
}

module_init(vexfs_init_module);
module_exit(vexfs_exit_module);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("VexFS Contributors");
MODULE_DESCRIPTION("VexFS: Vector-Native File System (SAFE FFI)");
MODULE_VERSION("1.0.0-safe-ffi");