/*
 * VexFS v2.0 - Allocation Operations Implementation (Task 5)
 * 
 * This implements the core allocation operations including block/inode allocation,
 * orphan detection, and background consistency checking for VexFS allocation journaling.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/bitmap.h>
#include <linux/crc32.h>
#include <linux/delay.h>
#include <linux/jiffies.h>
#include <linux/workqueue.h>
#include <linux/timer.h>

#include "../include/vexfs_v2_allocation_journal.h"
#include "../include/vexfs_v2_internal.h"

/*
 * Block allocation operations
 */

/**
 * vexfs_allocation_journal_block_alloc - Allocate blocks with journaling
 * @mgr: Allocation journal manager
 * @group_id: Target allocation group ID
 * @count: Number of blocks to allocate
 * @alignment: Alignment requirement (power of 2)
 * @allocated_blocks: Output array for allocated block numbers
 * @flags: Allocation flags
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_allocation_journal_block_alloc(struct vexfs_allocation_journal_manager *mgr,
                                         u32 group_id, u32 count, u32 alignment,
                                         u64 *allocated_blocks, u32 flags)
{
    struct vexfs_allocation_group *group;
    struct vexfs_allocation_operation *op;
    struct vexfs_atomic_transaction *trans;
    int start_bit, i, ret = 0;
    u32 allocated = 0;
    
    if (!mgr || !allocated_blocks || count == 0 || group_id >= mgr->max_groups) {
        pr_err("VexFS: Invalid block allocation parameters\n");
        return -EINVAL;
    }
    
    atomic64_inc(&mgr->allocation_requests);
    
    /* Get allocation group */
    mutex_lock(&mgr->groups_mutex);
    group = mgr->group_array[group_id];
    if (!group || !(group->flags & VEXFS_ALLOC_GROUP_ACTIVE)) {
        mutex_unlock(&mgr->groups_mutex);
        pr_err("VexFS: Invalid or inactive allocation group %u\n", group_id);
        atomic64_inc(&mgr->allocation_failures);
        return -ENOENT;
    }
    mutex_unlock(&mgr->groups_mutex);
    
    /* Check if group has enough free blocks */
    if (atomic_read(&group->free_blocks) < count) {
        pr_debug("VexFS: Group %u has insufficient free blocks (%d < %u)\n",
                 group_id, atomic_read(&group->free_blocks), count);
        atomic64_inc(&mgr->allocation_failures);
        return -ENOSPC;
    }
    
    /* Create allocation operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op) {
        pr_err("VexFS: Failed to allocate operation descriptor\n");
        atomic64_inc(&mgr->allocation_failures);
        return -ENOMEM;
    }
    
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_ALLOC_OP_BLOCK_ALLOC;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->group_id = group_id;
    op->count = count;
    op->alignment = alignment;
    op->timestamp = jiffies;
    atomic_set(&op->op_state, VEXFS_TRANS_RUNNING);
    init_completion(&op->op_completion);
    INIT_LIST_HEAD(&op->op_list);
    
    /* Start atomic transaction */
    trans = vexfs_atomic_begin(mgr->atomic_mgr, VEXFS_TRANS_BATCH_COMMIT,
                              VEXFS_ISOLATION_READ_COMMITTED);
    if (!trans) {
        pr_err("VexFS: Failed to start atomic transaction\n");
        kmem_cache_free(mgr->op_cache, op);
        atomic64_inc(&mgr->allocation_failures);
        return -ENOMEM;
    }
    
    op->transaction_id = trans->trans_id;
    
    /* Lock allocation group for allocation */
    mutex_lock(&group->alloc_mutex);
    
    /* Create before-state bitmap copy for rollback */
    op->before_bitmap = vexfs_kernel_bitmap_create(group->block_count);
    if (!op->before_bitmap) {
        pr_err("VexFS: Failed to create before-state bitmap\n");
        ret = -ENOMEM;
        goto error_unlock;
    }
    
    /* Copy current bitmap state */
    spin_lock(&group->block_bitmap->bitmap_lock);
    memcpy(op->before_bitmap->bits, group->block_bitmap->bits,
           group->block_bitmap->size_bytes);
    op->bitmap_checksum_before = vexfs_kernel_bitmap_checksum(group->block_bitmap);
    spin_unlock(&group->block_bitmap->bitmap_lock);
    
    /* Find and allocate blocks */
    start_bit = 0;
    for (i = 0; i < count; i++) {
        if (alignment > 1) {
            /* Find aligned area */
            start_bit = vexfs_kernel_bitmap_find_next_zero_area(
                group->block_bitmap, start_bit, 1, alignment);
        } else {
            /* Find next free bit */
            start_bit = vexfs_kernel_bitmap_find_first_zero(
                group->block_bitmap, start_bit);
        }
        
        if (start_bit >= group->block_count) {
            pr_debug("VexFS: No more free blocks in group %u after %d allocations\n",
                     group_id, i);
            ret = -ENOSPC;
            break;
        }
        
        /* Set the bit */
        ret = vexfs_kernel_bitmap_set(group->block_bitmap, start_bit);
        if (ret) {
            pr_err("VexFS: Failed to set bit %d in group %u\n", start_bit, group_id);
            break;
        }
        
        allocated_blocks[i] = group->start_block + start_bit;
        allocated++;
        start_bit++;
    }
    
    if (ret && allocated > 0) {
        /* Partial allocation - rollback allocated blocks */
        for (i = 0; i < allocated; i++) {
            u32 bit = allocated_blocks[i] - group->start_block;
            vexfs_kernel_bitmap_clear(group->block_bitmap, bit);
        }
        allocated = 0;
    }
    
    if (allocated > 0) {
        /* Update group counters */
        atomic_sub(allocated, &group->free_blocks);
        atomic64_add(allocated, &group->alloc_operations);
        atomic64_add(allocated, &mgr->blocks_allocated);
        
        /* Create after-state bitmap copy */
        op->after_bitmap = vexfs_kernel_bitmap_create(group->block_count);
        if (op->after_bitmap) {
            spin_lock(&group->block_bitmap->bitmap_lock);
            memcpy(op->after_bitmap->bits, group->block_bitmap->bits,
                   group->block_bitmap->size_bytes);
            op->bitmap_checksum_after = vexfs_kernel_bitmap_checksum(group->block_bitmap);
            spin_unlock(&group->block_bitmap->bitmap_lock);
        }
        
        op->start_block = allocated_blocks[0];
        op->count = allocated;
        atomic_set(&op->op_state, VEXFS_TRANS_COMMIT);
        op->op_result = 0;
        
        pr_debug("VexFS: Allocated %u blocks in group %u starting at %llu\n",
                 allocated, group_id, allocated_blocks[0]);
    } else {
        atomic_set(&op->op_state, VEXFS_TRANS_FINISHED);
        op->op_result = ret ? ret : -ENOSPC;
        atomic64_inc(&mgr->allocation_failures);
    }
    
error_unlock:
    mutex_unlock(&group->alloc_mutex);
    
    /* Commit or abort transaction */
    if (allocated > 0) {
        ret = vexfs_atomic_commit(trans);
        if (ret) {
            pr_err("VexFS: Failed to commit allocation transaction: %d\n", ret);
            /* TODO: Rollback allocation */
        }
    } else {
        vexfs_atomic_abort(trans);
    }
    
    /* Add operation to journal */
    if (allocated > 0) {
        mutex_lock(&mgr->ops_mutex);
        list_add_tail(&op->op_list, &mgr->pending_ops);
        atomic_inc(&mgr->pending_count);
        mutex_unlock(&mgr->ops_mutex);
        
        atomic64_inc(&mgr->ops_processed);
    } else {
        /* Clean up failed operation */
        if (op->before_bitmap)
            vexfs_kernel_bitmap_destroy(op->before_bitmap);
        if (op->after_bitmap)
            vexfs_kernel_bitmap_destroy(op->after_bitmap);
        kmem_cache_free(mgr->op_cache, op);
    }
    
    return allocated > 0 ? 0 : (ret ? ret : -ENOSPC);
}

/**
 * vexfs_allocation_journal_block_free - Free blocks with journaling
 * @mgr: Allocation journal manager
 * @group_id: Target allocation group ID
 * @start_block: Starting block number to free
 * @count: Number of blocks to free
 * @flags: Free operation flags
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_allocation_journal_block_free(struct vexfs_allocation_journal_manager *mgr,
                                        u32 group_id, u64 start_block,
                                        u32 count, u32 flags)
{
    struct vexfs_allocation_group *group;
    struct vexfs_allocation_operation *op;
    struct vexfs_atomic_transaction *trans;
    u32 start_bit, i, freed = 0;
    int ret = 0;
    
    if (!mgr || count == 0 || group_id >= mgr->max_groups) {
        pr_err("VexFS: Invalid block free parameters\n");
        return -EINVAL;
    }
    
    /* Get allocation group */
    mutex_lock(&mgr->groups_mutex);
    group = mgr->group_array[group_id];
    if (!group || !(group->flags & VEXFS_ALLOC_GROUP_ACTIVE)) {
        mutex_unlock(&mgr->groups_mutex);
        pr_err("VexFS: Invalid or inactive allocation group %u\n", group_id);
        return -ENOENT;
    }
    mutex_unlock(&mgr->groups_mutex);
    
    /* Validate block range */
    if (start_block < group->start_block ||
        start_block + count > group->start_block + group->block_count) {
        pr_err("VexFS: Block range %llu-%llu outside group %u range %llu-%llu\n",
               start_block, start_block + count - 1, group_id,
               group->start_block, group->start_block + group->block_count - 1);
        return -EINVAL;
    }
    
    start_bit = start_block - group->start_block;
    
    /* Create free operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op) {
        pr_err("VexFS: Failed to allocate operation descriptor\n");
        return -ENOMEM;
    }
    
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_ALLOC_OP_BLOCK_FREE;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->group_id = group_id;
    op->start_block = start_block;
    op->count = count;
    op->timestamp = jiffies;
    atomic_set(&op->op_state, VEXFS_TRANS_RUNNING);
    init_completion(&op->op_completion);
    INIT_LIST_HEAD(&op->op_list);
    
    /* Start atomic transaction */
    trans = vexfs_atomic_begin(mgr->atomic_mgr, VEXFS_TRANS_BATCH_COMMIT,
                              VEXFS_ISOLATION_READ_COMMITTED);
    if (!trans) {
        pr_err("VexFS: Failed to start atomic transaction\n");
        kmem_cache_free(mgr->op_cache, op);
        return -ENOMEM;
    }
    
    op->transaction_id = trans->trans_id;
    
    /* Lock allocation group for free operation */
    mutex_lock(&group->alloc_mutex);
    
    /* Create before-state bitmap copy for rollback */
    op->before_bitmap = vexfs_kernel_bitmap_create(group->block_count);
    if (!op->before_bitmap) {
        pr_err("VexFS: Failed to create before-state bitmap\n");
        ret = -ENOMEM;
        goto error_unlock;
    }
    
    /* Copy current bitmap state */
    spin_lock(&group->block_bitmap->bitmap_lock);
    memcpy(op->before_bitmap->bits, group->block_bitmap->bits,
           group->block_bitmap->size_bytes);
    op->bitmap_checksum_before = vexfs_kernel_bitmap_checksum(group->block_bitmap);
    spin_unlock(&group->block_bitmap->bitmap_lock);
    
    /* Free blocks */
    for (i = 0; i < count; i++) {
        u32 bit = start_bit + i;
        
        /* Check if block is actually allocated */
        if (!vexfs_kernel_bitmap_test(group->block_bitmap, bit)) {
            pr_warn("VexFS: Attempting to free unallocated block %llu in group %u\n",
                    start_block + i, group_id);
            continue;
        }
        
        /* Clear the bit */
        ret = vexfs_kernel_bitmap_clear(group->block_bitmap, bit);
        if (ret) {
            pr_err("VexFS: Failed to clear bit %u in group %u\n", bit, group_id);
            break;
        }
        
        freed++;
    }
    
    if (freed > 0) {
        /* Update group counters */
        atomic_add(freed, &group->free_blocks);
        atomic64_add(freed, &group->free_operations);
        atomic64_add(freed, &mgr->blocks_freed);
        
        /* Create after-state bitmap copy */
        op->after_bitmap = vexfs_kernel_bitmap_create(group->block_count);
        if (op->after_bitmap) {
            spin_lock(&group->block_bitmap->bitmap_lock);
            memcpy(op->after_bitmap->bits, group->block_bitmap->bits,
                   group->block_bitmap->size_bytes);
            op->bitmap_checksum_after = vexfs_kernel_bitmap_checksum(group->block_bitmap);
            spin_unlock(&group->block_bitmap->bitmap_lock);
        }
        
        op->count = freed;
        atomic_set(&op->op_state, VEXFS_TRANS_COMMIT);
        op->op_result = 0;
        
        pr_debug("VexFS: Freed %u blocks in group %u starting at %llu\n",
                 freed, group_id, start_block);
    } else {
        atomic_set(&op->op_state, VEXFS_TRANS_FINISHED);
        op->op_result = ret ? ret : -EINVAL;
    }
    
error_unlock:
    mutex_unlock(&group->alloc_mutex);
    
    /* Commit or abort transaction */
    if (freed > 0) {
        ret = vexfs_atomic_commit(trans);
        if (ret) {
            pr_err("VexFS: Failed to commit free transaction: %d\n", ret);
            /* TODO: Rollback free operation */
        }
    } else {
        vexfs_atomic_abort(trans);
    }
    
    /* Add operation to journal */
    if (freed > 0) {
        mutex_lock(&mgr->ops_mutex);
        list_add_tail(&op->op_list, &mgr->pending_ops);
        atomic_inc(&mgr->pending_count);
        mutex_unlock(&mgr->ops_mutex);
        
        atomic64_inc(&mgr->ops_processed);
    } else {
        /* Clean up failed operation */
        if (op->before_bitmap)
            vexfs_kernel_bitmap_destroy(op->before_bitmap);
        if (op->after_bitmap)
            vexfs_kernel_bitmap_destroy(op->after_bitmap);
        kmem_cache_free(mgr->op_cache, op);
    }
    
    return freed > 0 ? 0 : (ret ? ret : -EINVAL);
}

/*
 * Inode allocation operations
 */

/**
 * vexfs_allocation_journal_inode_alloc - Allocate inode with journaling
 * @mgr: Allocation journal manager
 * @group_id: Target allocation group ID
 * @allocated_inode: Output for allocated inode number
 * @flags: Allocation flags
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_allocation_journal_inode_alloc(struct vexfs_allocation_journal_manager *mgr,
                                         u32 group_id, u64 *allocated_inode, u32 flags)
{
    struct vexfs_allocation_group *group;
    struct vexfs_allocation_operation *op;
    struct vexfs_atomic_transaction *trans;
    int inode_bit, ret = 0;
    
    if (!mgr || !allocated_inode || group_id >= mgr->max_groups) {
        pr_err("VexFS: Invalid inode allocation parameters\n");
        return -EINVAL;
    }
    
    atomic64_inc(&mgr->allocation_requests);
    
    /* Get allocation group */
    mutex_lock(&mgr->groups_mutex);
    group = mgr->group_array[group_id];
    if (!group || !(group->flags & VEXFS_ALLOC_GROUP_ACTIVE)) {
        mutex_unlock(&mgr->groups_mutex);
        pr_err("VexFS: Invalid or inactive allocation group %u\n", group_id);
        atomic64_inc(&mgr->allocation_failures);
        return -ENOENT;
    }
    mutex_unlock(&mgr->groups_mutex);
    
    /* Check if group has free inodes */
    if (atomic_read(&group->free_inodes) <= 0) {
        pr_debug("VexFS: Group %u has no free inodes\n", group_id);
        atomic64_inc(&mgr->allocation_failures);
        return -ENOSPC;
    }
    
    /* Create allocation operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op) {
        pr_err("VexFS: Failed to allocate operation descriptor\n");
        atomic64_inc(&mgr->allocation_failures);
        return -ENOMEM;
    }
    
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_ALLOC_OP_INODE_ALLOC;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->group_id = group_id;
    op->count = 1;
    op->timestamp = jiffies;
    atomic_set(&op->op_state, VEXFS_TRANS_RUNNING);
    init_completion(&op->op_completion);
    INIT_LIST_HEAD(&op->op_list);
    
    /* Start atomic transaction */
    trans = vexfs_atomic_begin(mgr->atomic_mgr, VEXFS_TRANS_BATCH_COMMIT,
                              VEXFS_ISOLATION_READ_COMMITTED);
    if (!trans) {
        pr_err("VexFS: Failed to start atomic transaction\n");
        kmem_cache_free(mgr->op_cache, op);
        atomic64_inc(&mgr->allocation_failures);
        return -ENOMEM;
    }
    
    op->transaction_id = trans->trans_id;
    
    /* Lock allocation group for allocation */
    mutex_lock(&group->alloc_mutex);
    
    /* Find free inode */
    inode_bit = vexfs_kernel_bitmap_find_first_zero(group->inode_bitmap, 0);
    if (inode_bit >= group->inode_count) {
        pr_debug("VexFS: No free inodes in group %u\n", group_id);
        ret = -ENOSPC;
        goto error_unlock;
    }
    
    /* Create before-state bitmap copy for rollback */
    op->before_bitmap = vexfs_kernel_bitmap_create(group->inode_count);
    if (!op->before_bitmap) {
        pr_err("VexFS: Failed to create before-state bitmap\n");
        ret = -ENOMEM;
        goto error_unlock;
    }
    
    /* Copy current bitmap state */
    spin_lock(&group->inode_bitmap->bitmap_lock);
    memcpy(op->before_bitmap->bits, group->inode_bitmap->bits,
           group->inode_bitmap->size_bytes);
    op->bitmap_checksum_before = vexfs_kernel_bitmap_checksum(group->inode_bitmap);
    spin_unlock(&group->inode_bitmap->bitmap_lock);
    
    /* Set the inode bit */
    ret = vexfs_kernel_bitmap_set(group->inode_bitmap, inode_bit);
    if (ret) {
        pr_err("VexFS: Failed to set inode bit %d in group %u\n", inode_bit, group_id);
        goto error_unlock;
    }
    
    /* Calculate inode number */
    *allocated_inode = (u64)group_id * group->inode_count + inode_bit + 1;
    
    /* Update group counters */
    atomic_dec(&group->free_inodes);
    atomic64_inc(&group->alloc_operations);
    atomic64_inc(&mgr->inodes_allocated);
    
    /* Create after-state bitmap copy */
    op->after_bitmap = vexfs_kernel_bitmap_create(group->inode_count);
    if (op->after_bitmap) {
        spin_lock(&group->inode_bitmap->bitmap_lock);
        memcpy(op->after_bitmap->bits, group->inode_bitmap->bits,
               group->inode_bitmap->size_bytes);
        op->bitmap_checksum_after = vexfs_kernel_bitmap_checksum(group->inode_bitmap);
        spin_unlock(&group->inode_bitmap->bitmap_lock);
    }
    
    op->start_block = *allocated_inode;
    atomic_set(&op->op_state, VEXFS_TRANS_COMMIT);
    op->op_result = 0;
    
    pr_debug("VexFS: Allocated inode %llu in group %u\n", *allocated_inode, group_id);
    
error_unlock:
    mutex_unlock(&group->alloc_mutex);
    
    /* Commit or abort transaction */
    if (ret == 0) {
        ret = vexfs_atomic_commit(trans);
        if (ret) {
            pr_err("VexFS: Failed to commit inode allocation transaction: %d\n", ret);
            /* TODO: Rollback allocation */
        }
    } else {
        vexfs_atomic_abort(trans);
        atomic64_inc(&mgr->allocation_failures);
    }
    
    /* Add operation to journal */
    if (ret == 0) {
        mutex_lock(&mgr->ops_mutex);
        list_add_tail(&op->op_list, &mgr->pending_ops);
        atomic_inc(&mgr->pending_count);
        mutex_unlock(&mgr->ops_mutex);
        
        atomic64_inc(&mgr->ops_processed);
    } else {
        /* Clean up failed operation */
        if (op->before_bitmap)
            vexfs_kernel_bitmap_destroy(op->before_bitmap);
        if (op->after_bitmap)
            vexfs_kernel_bitmap_destroy(op->after_bitmap);
        kmem_cache_free(mgr->op_cache, op);
    }
    
    return ret;
}

/**
 * vexfs_allocation_journal_inode_free - Free inode with journaling
 * @mgr: Allocation journal manager
 * @group_id: Target allocation group ID
 * @inode_number: Inode number to free
 * @flags: Free operation flags
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_allocation_journal_inode_free(struct vexfs_allocation_journal_manager *mgr,
                                        u32 group_id, u64 inode_number, u32 flags)
{
    struct vexfs_allocation_group *group;
    struct vexfs_allocation_operation *op;
    struct vexfs_atomic_transaction *trans;
    u32 inode_bit;
    int ret = 0;
    
    if (!mgr || inode_number == 0 || group_id >= mgr->max_groups) {
        pr_err("VexFS: Invalid inode free parameters\n");
        return -EINVAL;
    }
    
    /* Get allocation group */
    mutex_lock(&mgr->groups_mutex);
    group = mgr->group_array[group_id];
    if (!group || !(group->flags & VEXFS_ALLOC_GROUP_ACTIVE)) {
        mutex_unlock(&mgr->groups_mutex);
        pr_err("VexFS: Invalid or inactive allocation group %u\n", group_id);
        return -ENOENT;
    }
    mutex_unlock(&mgr->groups_mutex);
    
    /* Calculate inode bit */
    inode_bit = (inode_number - 1) % group->inode_count;
    
    /* Validate inode belongs to this group */
    if ((inode_number - 1) / group->inode_count != group_id) {
        pr_err("VexFS: Inode %llu does not belong to group %u\n",
               inode_number, group_id);
        return -EINVAL;
    }
    
    /* Create free operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op) {
        pr_err("VexFS: Failed to allocate operation descriptor\n");
        return -ENOMEM;
    }
    
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_ALLOC_OP_INODE_FREE;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->group_id = group_id;
    op->start_block = inode_number;
    op->count = 1;
    op->timestamp = jiffies;
    atomic_set(&op->op_state, VEXFS_TRANS_RUNNING);
    init_completion(&op->op_completion);
    INIT_LIST_HEAD(&op->op_list);
    
    /* Start atomic transaction */
    trans = vexfs_atomic_begin(mgr->atomic_mgr, VEXFS_TRANS_BATCH_COMMIT,
                              VEXFS_ISOLATION_READ_COMMITTED);
    if (!trans) {
        pr_err("VexFS: Failed to start atomic transaction\n");
        kmem_cache_free(mgr->op_cache, op);
        return -ENOMEM;
    }
    
    op->transaction_id = trans->trans_id;
    
    /* Lock allocation group for free operation */
    mutex_lock(&group->alloc_mutex);
    
    /* Check if inode is actually allocated */
    if (!vexfs_kernel_bitmap_test(group->inode_bitmap, inode_bit)) {
        pr_warn("VexFS: Attempting to free unallocated inode %llu in group %u\n",
                inode_number, group_id);
        ret = -EINVAL;
        goto error_unlock;
    }
    
    /* Create before-state bitmap copy for rollback */
    op->before_bitmap = vexfs_kernel_bitmap_create(group->inode_count);
    if (!op->before_bitmap) {
        pr_err("VexFS: Failed to create before-state bitmap\n");
        ret = -ENOMEM;
        goto error_unlock;
    }
    
    /* Copy current bitmap state */
    spin_lock(&group->inode_bitmap->bitmap_lock);
    memcpy(op->before_bitmap->bits, group->inode_bitmap->bits,
           group->inode_bitmap->size_bytes);
    op->bitmap_checksum_before = vexfs_kernel_bitmap_checksum(group->inode_bitmap);
    spin_unlock(&group->inode_bitmap->bitmap_lock);
    
    /* Clear the inode bit */
    ret = vexfs_kernel_bitmap_clear(group->inode_bitmap, inode_bit);
    if (ret) {
        pr_err("VexFS: Failed to clear inode bit %u in group %u\n", inode_bit, group_id);
        goto error_unlock;
    }
    
    /* Update group counters */
    atomic_inc(&group->free_inodes);
    atomic64_inc(&group->free_operations);
    atomic64_inc(&mgr->inodes_freed);
    
    /* Create after-state bitmap copy */
    op->after_bitmap = vexfs_kernel_bitmap_create(group->inode_count);
    if (op->after_bitmap) {
        spin_lock(&group->inode_bitmap->bitmap_lock);
        memcpy(op->after_bitmap->bits, group->inode_bitmap->bits,
               group->inode_bitmap->size_bytes);
        op->bitmap_checksum_after = vexfs_kernel_bitmap_checksum(group->inode_bitmap);
        spin_unlock(&group->inode_bitmap->bitmap_lock);
    }
    
    atomic_set(&op->op_state, VEXFS_TRANS_COMMIT);
    op->op_result = 0;
    
    pr_debug("VexFS: Freed inode %llu in group %u\n", inode_number, group_id);
    
error_unlock:
    mutex_unlock(&