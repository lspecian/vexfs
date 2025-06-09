/*
 * VexFS v2.0 - Safe Block/Inode Journaling Implementation (Task 5)
 * 
 * This implements the core allocation journaling functionality for VexFS,
 * building on the Phase 1 foundation to provide comprehensive allocation
 * tracking and recovery capabilities.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/bitmap.h>
#include <linux/crc32.h>
#include <linux/delay.h>
#include <linux/jiffies.h>

#include "../include/vexfs_v2_allocation_journal.h"
#include "../include/vexfs_v2_internal.h"

/* Module information */
MODULE_DESCRIPTION("VexFS v2.0 Allocation Journaling");
MODULE_VERSION("2.0.0");

/*
 * Kernel bitmap operations implementation
 * Custom implementation since bitvec is userspace-only
 */

/**
 * vexfs_kernel_bitmap_create - Create a new kernel bitmap
 * @size_bits: Size of bitmap in bits
 *
 * Returns: Pointer to new bitmap or NULL on failure
 */
struct vexfs_kernel_bitmap *vexfs_kernel_bitmap_create(u32 size_bits)
{
    struct vexfs_kernel_bitmap *bitmap;
    u32 size_bytes, size_longs;
    
    if (size_bits == 0 || size_bits > (1U << 30)) {
        pr_err("VexFS: Invalid bitmap size: %u bits\n", size_bits);
        return NULL;
    }
    
    bitmap = kzalloc(sizeof(*bitmap), GFP_KERNEL);
    if (!bitmap) {
        pr_err("VexFS: Failed to allocate bitmap structure\n");
        return NULL;
    }
    
    size_bytes = BITS_TO_LONGS(size_bits) * sizeof(unsigned long);
    size_longs = BITS_TO_LONGS(size_bits);
    
    bitmap->bits = vzalloc(size_bytes);
    if (!bitmap->bits) {
        pr_err("VexFS: Failed to allocate bitmap data (%u bytes)\n", size_bytes);
        kfree(bitmap);
        return NULL;
    }
    
    bitmap->size_bits = size_bits;
    bitmap->size_bytes = size_bytes;
    bitmap->size_longs = size_longs;
    bitmap->last_set_bit = 0;
    bitmap->last_clear_bit = 0;
    atomic_set(&bitmap->set_bits, 0);
    spin_lock_init(&bitmap->bitmap_lock);
    bitmap->checksum = 0;
    bitmap->last_update = jiffies;
    
    pr_debug("VexFS: Created bitmap: %u bits, %u bytes\n", size_bits, size_bytes);
    return bitmap;
}

/**
 * vexfs_kernel_bitmap_destroy - Destroy a kernel bitmap
 * @bitmap: Bitmap to destroy
 */
void vexfs_kernel_bitmap_destroy(struct vexfs_kernel_bitmap *bitmap)
{
    if (!bitmap)
        return;
        
    if (bitmap->bits) {
        vfree(bitmap->bits);
        bitmap->bits = NULL;
    }
    
    kfree(bitmap);
}

/**
 * vexfs_kernel_bitmap_set - Set a bit in the bitmap
 * @bitmap: Target bitmap
 * @bit: Bit number to set
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_kernel_bitmap_set(struct vexfs_kernel_bitmap *bitmap, u32 bit)
{
    unsigned long flags;
    bool was_clear;
    
    if (!bitmap || bit >= bitmap->size_bits) {
        pr_err("VexFS: Invalid bitmap set operation: bit %u, size %u\n",
               bit, bitmap ? bitmap->size_bits : 0);
        return -EINVAL;
    }
    
    spin_lock_irqsave(&bitmap->bitmap_lock, flags);
    
    was_clear = !test_bit(bit, bitmap->bits);
    set_bit(bit, bitmap->bits);
    
    if (was_clear) {
        atomic_inc(&bitmap->set_bits);
        bitmap->last_set_bit = bit;
    }
    
    bitmap->last_update = jiffies;
    
    spin_unlock_irqrestore(&bitmap->bitmap_lock, flags);
    
    return 0;
}

/**
 * vexfs_kernel_bitmap_clear - Clear a bit in the bitmap
 * @bitmap: Target bitmap
 * @bit: Bit number to clear
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_kernel_bitmap_clear(struct vexfs_kernel_bitmap *bitmap, u32 bit)
{
    unsigned long flags;
    bool was_set;
    
    if (!bitmap || bit >= bitmap->size_bits) {
        pr_err("VexFS: Invalid bitmap clear operation: bit %u, size %u\n",
               bit, bitmap ? bitmap->size_bits : 0);
        return -EINVAL;
    }
    
    spin_lock_irqsave(&bitmap->bitmap_lock, flags);
    
    was_set = test_bit(bit, bitmap->bits);
    clear_bit(bit, bitmap->bits);
    
    if (was_set) {
        atomic_dec(&bitmap->set_bits);
        bitmap->last_clear_bit = bit;
    }
    
    bitmap->last_update = jiffies;
    
    spin_unlock_irqrestore(&bitmap->bitmap_lock, flags);
    
    return 0;
}

/**
 * vexfs_kernel_bitmap_test - Test if a bit is set
 * @bitmap: Target bitmap
 * @bit: Bit number to test
 *
 * Returns: 1 if bit is set, 0 if clear, negative error code on failure
 */
int vexfs_kernel_bitmap_test(struct vexfs_kernel_bitmap *bitmap, u32 bit)
{
    unsigned long flags;
    int result;
    
    if (!bitmap || bit >= bitmap->size_bits) {
        pr_err("VexFS: Invalid bitmap test operation: bit %u, size %u\n",
               bit, bitmap ? bitmap->size_bits : 0);
        return -EINVAL;
    }
    
    spin_lock_irqsave(&bitmap->bitmap_lock, flags);
    result = test_bit(bit, bitmap->bits) ? 1 : 0;
    spin_unlock_irqrestore(&bitmap->bitmap_lock, flags);
    
    return result;
}

/**
 * vexfs_kernel_bitmap_find_first_zero - Find first zero bit
 * @bitmap: Target bitmap
 * @start: Starting bit position
 *
 * Returns: Bit number of first zero bit, or bitmap size if none found
 */
int vexfs_kernel_bitmap_find_first_zero(struct vexfs_kernel_bitmap *bitmap, u32 start)
{
    unsigned long flags;
    unsigned long result;
    
    if (!bitmap || start >= bitmap->size_bits) {
        pr_err("VexFS: Invalid bitmap find_first_zero: start %u, size %u\n",
               start, bitmap ? bitmap->size_bits : 0);
        return -EINVAL;
    }
    
    spin_lock_irqsave(&bitmap->bitmap_lock, flags);
    result = find_next_zero_bit(bitmap->bits, bitmap->size_bits, start);
    spin_unlock_irqrestore(&bitmap->bitmap_lock, flags);
    
    return (result >= bitmap->size_bits) ? bitmap->size_bits : (int)result;
}

/**
 * vexfs_kernel_bitmap_find_next_zero_area - Find next zero area
 * @bitmap: Target bitmap
 * @start: Starting bit position
 * @count: Number of consecutive zero bits needed
 * @align: Alignment requirement (power of 2)
 *
 * Returns: Starting bit of zero area, or bitmap size if none found
 */
int vexfs_kernel_bitmap_find_next_zero_area(struct vexfs_kernel_bitmap *bitmap,
                                           u32 start, u32 count, u32 align)
{
    unsigned long flags;
    unsigned long result;
    
    if (!bitmap || start >= bitmap->size_bits || count == 0) {
        pr_err("VexFS: Invalid bitmap find_next_zero_area parameters\n");
        return -EINVAL;
    }
    
    /* Ensure alignment is power of 2 */
    if (align && (align & (align - 1))) {
        pr_err("VexFS: Invalid alignment: %u (must be power of 2)\n", align);
        return -EINVAL;
    }
    
    spin_lock_irqsave(&bitmap->bitmap_lock, flags);
    result = bitmap_find_next_zero_area(bitmap->bits, bitmap->size_bits,
                                       start, count, align ? align - 1 : 0);
    spin_unlock_irqrestore(&bitmap->bitmap_lock, flags);
    
    return (result >= bitmap->size_bits) ? bitmap->size_bits : (int)result;
}

/**
 * vexfs_kernel_bitmap_weight - Count set bits in bitmap
 * @bitmap: Target bitmap
 *
 * Returns: Number of set bits
 */
u32 vexfs_kernel_bitmap_weight(struct vexfs_kernel_bitmap *bitmap)
{
    if (!bitmap)
        return 0;
        
    return atomic_read(&bitmap->set_bits);
}

/**
 * vexfs_kernel_bitmap_checksum - Calculate bitmap checksum
 * @bitmap: Target bitmap
 *
 * Returns: CRC32 checksum of bitmap data
 */
u32 vexfs_kernel_bitmap_checksum(struct vexfs_kernel_bitmap *bitmap)
{
    unsigned long flags;
    u32 checksum;
    
    if (!bitmap || !bitmap->bits)
        return 0;
    
    spin_lock_irqsave(&bitmap->bitmap_lock, flags);
    checksum = crc32(0, bitmap->bits, bitmap->size_bytes);
    bitmap->checksum = checksum;
    spin_unlock_irqrestore(&bitmap->bitmap_lock, flags);
    
    return checksum;
}

/*
 * Allocation group management
 */

/**
 * vexfs_allocation_group_create - Create a new allocation group
 * @mgr: Allocation journal manager
 * @group_id: Group ID
 * @start_block: Starting block number
 * @block_count: Number of blocks in group
 * @inode_count: Number of inodes in group
 *
 * Returns: Pointer to new allocation group or NULL on failure
 */
struct vexfs_allocation_group *vexfs_allocation_group_create(
    struct vexfs_allocation_journal_manager *mgr,
    u32 group_id, u64 start_block, u32 block_count, u32 inode_count)
{
    struct vexfs_allocation_group *group;
    
    if (!mgr || block_count == 0 || inode_count == 0) {
        pr_err("VexFS: Invalid allocation group parameters\n");
        return NULL;
    }
    
    group = kmem_cache_alloc(mgr->group_cache, GFP_KERNEL);
    if (!group) {
        pr_err("VexFS: Failed to allocate allocation group\n");
        return NULL;
    }
    
    memset(group, 0, sizeof(*group));
    
    group->group_id = group_id;
    group->flags = VEXFS_ALLOC_GROUP_ACTIVE;
    group->start_block = start_block;
    group->block_count = block_count;
    group->inode_count = inode_count;
    
    atomic_set(&group->free_blocks, block_count);
    atomic_set(&group->free_inodes, inode_count);
    group->largest_free_extent = block_count;
    
    group->allocation_strategy = mgr->default_strategy;
    group->fragmentation_score = 0;
    group->vector_alignment_blocks = 1;
    
    group->last_journal_sequence = 0;
    INIT_LIST_HEAD(&group->pending_allocs);
    
    atomic64_set(&group->alloc_operations, 0);
    atomic64_set(&group->free_operations, 0);
    atomic64_set(&group->fragmentation_events, 0);
    
    init_rwsem(&group->group_rwsem);
    mutex_init(&group->alloc_mutex);
    
    INIT_LIST_HEAD(&group->group_list);
    
    pr_debug("VexFS: Created allocation group %u: blocks %llu-%llu, inodes %u\n",
             group_id, start_block, start_block + block_count - 1, inode_count);
    
    return group;
}

/**
 * vexfs_allocation_group_destroy - Destroy an allocation group
 * @group: Allocation group to destroy
 */
void vexfs_allocation_group_destroy(struct vexfs_allocation_group *group)
{
    if (!group)
        return;
    
    if (group->block_bitmap) {
        vexfs_kernel_bitmap_destroy(group->block_bitmap);
        group->block_bitmap = NULL;
    }
    
    if (group->inode_bitmap) {
        vexfs_kernel_bitmap_destroy(group->inode_bitmap);
        group->inode_bitmap = NULL;
    }
    
    /* Note: group is freed by kmem_cache_free in the caller */
}

/**
 * vexfs_allocation_group_init_bitmaps - Initialize allocation group bitmaps
 * @group: Allocation group
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_allocation_group_init_bitmaps(struct vexfs_allocation_group *group)
{
    if (!group) {
        pr_err("VexFS: Invalid allocation group for bitmap initialization\n");
        return -EINVAL;
    }
    
    /* Create block allocation bitmap */
    group->block_bitmap = vexfs_kernel_bitmap_create(group->block_count);
    if (!group->block_bitmap) {
        pr_err("VexFS: Failed to create block bitmap for group %u\n",
               group->group_id);
        return -ENOMEM;
    }
    
    /* Create inode allocation bitmap */
    group->inode_bitmap = vexfs_kernel_bitmap_create(group->inode_count);
    if (!group->inode_bitmap) {
        pr_err("VexFS: Failed to create inode bitmap for group %u\n",
               group->group_id);
        vexfs_kernel_bitmap_destroy(group->block_bitmap);
        group->block_bitmap = NULL;
        return -ENOMEM;
    }
    
    pr_debug("VexFS: Initialized bitmaps for allocation group %u\n",
             group->group_id);
    
    return 0;
}

/*
 * Allocation journal manager
 */

/**
 * vexfs_allocation_journal_init - Initialize allocation journal manager
 * @journal: Associated journal
 * @atomic_mgr: Atomic operations manager
 * @meta_mgr: Metadata journal manager
 *
 * Returns: Pointer to new manager or NULL on failure
 */
struct vexfs_allocation_journal_manager *vexfs_allocation_journal_init(
    struct vexfs_journal *journal,
    struct vexfs_atomic_manager *atomic_mgr,
    struct vexfs_metadata_journal_manager *meta_mgr)
{
    struct vexfs_allocation_journal_manager *mgr;
    
    if (!journal || !atomic_mgr || !meta_mgr) {
        pr_err("VexFS: Invalid parameters for allocation journal manager\n");
        return NULL;
    }
    
    mgr = kzalloc(sizeof(*mgr), GFP_KERNEL);
    if (!mgr) {
        pr_err("VexFS: Failed to allocate allocation journal manager\n");
        return NULL;
    }
    
    mgr->journal = journal;
    mgr->atomic_mgr = atomic_mgr;
    mgr->meta_mgr = meta_mgr;
    
    /* Initialize allocation groups */
    INIT_LIST_HEAD(&mgr->allocation_groups);
    mgr->group_array = kzalloc(VEXFS_ALLOC_MAX_GROUPS * sizeof(struct vexfs_allocation_group *),
                              GFP_KERNEL);
    if (!mgr->group_array) {
        pr_err("VexFS: Failed to allocate group array\n");
        kfree(mgr);
        return NULL;
    }
    
    mutex_init(&mgr->groups_mutex);
    atomic_set(&mgr->active_groups, 0);
    mgr->max_groups = VEXFS_ALLOC_MAX_GROUPS;
    
    /* Initialize operation management */
    INIT_LIST_HEAD(&mgr->pending_ops);
    mutex_init(&mgr->ops_mutex);
    atomic_set(&mgr->pending_count, 0);
    mgr->next_op_id = 1;
    
    /* Initialize batch processing */
    mgr->batch_workqueue = alloc_workqueue("vexfs_alloc_batch",
                                          WQ_MEM_RECLAIM | WQ_UNBOUND, 1);
    if (!mgr->batch_workqueue) {
        pr_err("VexFS: Failed to create batch workqueue\n");
        kfree(mgr->group_array);
        kfree(mgr);
        return NULL;
    }
    
    INIT_DELAYED_WORK(&mgr->batch_work, NULL); /* Work function set later */
    mgr->batch_size = 0;
    mgr->max_batch_size = VEXFS_ALLOC_MAX_BATCH_SIZE;
    
    /* Initialize orphan management */
    mgr->orphan_tree = RB_ROOT;
    INIT_LIST_HEAD(&mgr->orphan_list);
    mutex_init(&mgr->orphan_mutex);
    atomic_set(&mgr->orphan_count, 0);
    mgr->max_orphans = VEXFS_ALLOC_MAX_ORPHANS;
    
    /* Initialize background consistency checking */
    mgr->consistency_workqueue = alloc_workqueue("vexfs_alloc_consistency",
                                                WQ_MEM_RECLAIM | WQ_UNBOUND, 1);
    if (!mgr->consistency_workqueue) {
        pr_err("VexFS: Failed to create consistency workqueue\n");
        destroy_workqueue(mgr->batch_workqueue);
        kfree(mgr->group_array);
        kfree(mgr);
        return NULL;
    }
    
    INIT_DELAYED_WORK(&mgr->consistency_work, NULL); /* Work function set later */
    timer_setup(&mgr->consistency_timer, NULL, 0); /* Timer function set later */
    mgr->consistency_interval = 30000; /* 30 seconds */
    
    /* Initialize memory caches */
    mgr->bitmap_cache = kmem_cache_create("vexfs_bitmap_cache",
                                         sizeof(struct vexfs_kernel_bitmap),
                                         0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->bitmap_cache) {
        pr_err("VexFS: Failed to create bitmap cache\n");
        destroy_workqueue(mgr->consistency_workqueue);
        destroy_workqueue(mgr->batch_workqueue);
        kfree(mgr->group_array);
        kfree(mgr);
        return NULL;
    }
    
    INIT_LIST_HEAD(&mgr->cached_bitmaps);
    mutex_init(&mgr->cache_mutex);
    atomic_set(&mgr->cached_bitmap_count, 0);
    
    /* Set allocation strategies */
    mgr->default_strategy = VEXFS_ALLOC_STRATEGY_FIRST_FIT;
    mgr->vector_strategy = VEXFS_ALLOC_STRATEGY_VECTOR_OPT;
    mgr->fragmentation_threshold = 75; /* 75% fragmentation threshold */
    
    /* Initialize performance counters */
    atomic64_set(&mgr->ops_processed, 0);
    atomic64_set(&mgr->blocks_allocated, 0);
    atomic64_set(&mgr->blocks_freed, 0);
    atomic64_set(&mgr->inodes_allocated, 0);
    atomic64_set(&mgr->inodes_freed, 0);
    atomic64_set(&mgr->orphans_cleaned, 0);
    
    /* Create memory caches */
    mgr->op_cache = kmem_cache_create("vexfs_alloc_op_cache",
                                     sizeof(struct vexfs_allocation_operation),
                                     0, SLAB_HWCACHE_ALIGN, NULL);
    mgr->orphan_cache = kmem_cache_create("vexfs_orphan_cache",
                                         sizeof(struct vexfs_orphan_entry),
                                         0, SLAB_HWCACHE_ALIGN, NULL);
    mgr->group_cache = kmem_cache_create("vexfs_group_cache",
                                        sizeof(struct vexfs_allocation_group),
                                        0, SLAB_HWCACHE_ALIGN, NULL);
    
    if (!mgr->op_cache || !mgr->orphan_cache || !mgr->group_cache) {
        pr_err("VexFS: Failed to create allocation caches\n");
        /* Cleanup handled in destroy function */
        vexfs_allocation_journal_destroy(mgr);
        return NULL;
    }
    
    /* Set configuration */
    mgr->journal_flags = VEXFS_ALLOC_JOURNAL_ORDERED | VEXFS_ALLOC_JOURNAL_CHECKSUM;
    mgr->sync_mode = VEXFS_META_JOURNAL_ASYNC;
    mgr->batch_timeout = 1000; /* 1 second */
    mgr->orphan_cleanup_interval = 60000; /* 60 seconds */
    
    /* Initialize statistics */
    atomic64_set(&mgr->allocation_requests, 0);
    atomic64_set(&mgr->allocation_failures, 0);
    atomic64_set(&mgr->fragmentation_score, 0);
    atomic64_set(&mgr->consistency_checks, 0);
    atomic64_set(&mgr->consistency_errors, 0);
    
    /* Initialize error handling */
    atomic_set(&mgr->error_count, 0);
    INIT_LIST_HEAD(&mgr->error_log);
    
    /* Initialize synchronization */
    init_rwsem(&mgr->manager_rwsem);
    spin_lock_init(&mgr->stats_lock);
    
    pr_info("VexFS: Allocation journal manager initialized successfully\n");
    
    return mgr;
}

/**
 * vexfs_allocation_journal_destroy - Destroy allocation journal manager
 * @mgr: Manager to destroy
 */
void vexfs_allocation_journal_destroy(struct vexfs_allocation_journal_manager *mgr)
{
    struct vexfs_allocation_group *group, *tmp_group;
    
    if (!mgr)
        return;
    
    pr_info("VexFS: Destroying allocation journal manager\n");
    
    /* Stop background work */
    if (mgr->batch_workqueue) {
        cancel_delayed_work_sync(&mgr->batch_work);
        destroy_workqueue(mgr->batch_workqueue);
    }
    
    if (mgr->consistency_workqueue) {
        cancel_delayed_work_sync(&mgr->consistency_work);
        destroy_workqueue(mgr->consistency_workqueue);
    }
    
    del_timer_sync(&mgr->consistency_timer);
    
    /* Destroy allocation groups */
    mutex_lock(&mgr->groups_mutex);
    list_for_each_entry_safe(group, tmp_group, &mgr->allocation_groups, group_list) {
        list_del(&group->group_list);
        vexfs_allocation_group_destroy(group);
        if (mgr->group_cache)
            kmem_cache_free(mgr->group_cache, group);
    }
    mutex_unlock(&mgr->groups_mutex);
    
    /* Destroy memory caches */
    if (mgr->bitmap_cache)
        kmem_cache_destroy(mgr->bitmap_cache);
    if (mgr->op_cache)
        kmem_cache_destroy(mgr->op_cache);
    if (mgr->orphan_cache)
        kmem_cache_destroy(mgr->orphan_cache);
    if (mgr->group_cache)
        kmem_cache_destroy(mgr->group_cache);
    
    /* Free group array */
    if (mgr->group_array)
        kfree(mgr->group_array);
    
    kfree(mgr);
    
    pr_info("VexFS: Allocation journal manager destroyed\n");
}

/* Export symbols for kernel module use */
EXPORT_SYMBOL(vexfs_kernel_bitmap_create);
EXPORT_SYMBOL(vexfs_kernel_bitmap_destroy);
EXPORT_SYMBOL(vexfs_kernel_bitmap_set);
EXPORT_SYMBOL(vexfs_kernel_bitmap_clear);
EXPORT_SYMBOL(vexfs_kernel_bitmap_test);
EXPORT_SYMBOL(vexfs_kernel_bitmap_find_first_zero);
EXPORT_SYMBOL(vexfs_kernel_bitmap_find_next_zero_area);
EXPORT_SYMBOL(vexfs_kernel_bitmap_weight);
EXPORT_SYMBOL(vexfs_kernel_bitmap_checksum);

EXPORT_SYMBOL(vexfs_allocation_group_create);
EXPORT_SYMBOL(vexfs_allocation_group_destroy);
EXPORT_SYMBOL(vexfs_allocation_group_init_bitmaps);

EXPORT_SYMBOL(vexfs_allocation_journal_init);
EXPORT_SYMBOL(vexfs_allocation_journal_destroy);