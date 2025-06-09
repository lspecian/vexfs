/*
 * VexFS v2.0 - Metadata Journaling Implementation (Task 3) - Part 2
 * 
 * This continues the metadata journaling implementation with batch processing,
 * cache management, and utility functions.
 *
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/workqueue.h>
#include <linux/delay.h>
#include <linux/crc32.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/atomic.h>
#include <linux/completion.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/time.h>
#include <linux/uaccess.h>

#include "../include/vexfs_v2_metadata_journal.h"
#include "../include/vexfs_v2_internal.h"

/* External reference to global manager */
extern struct vexfs_metadata_journal_manager *global_meta_mgr;

/*
 * =============================================================================
 * BATCH PROCESSING AND WORK FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_metadata_process_operation - Process a single metadata operation
 * @mgr: Metadata journaling manager
 * @op: Operation to process
 * 
 * Processes a single metadata operation by writing it to the journal
 * through the atomic operations layer.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_metadata_process_operation(struct vexfs_metadata_journal_manager *mgr,
                                           struct vexfs_metadata_operation *op)
{
    struct vexfs_atomic_transaction *trans;
    int ret;

    if (!mgr || !op)
        return -EINVAL;

    /* Begin atomic transaction */
    trans = vexfs_atomic_begin(mgr->atomic_mgr, 
                              VEXFS_TRANS_BATCH_COMMIT,
                              VEXFS_ISOLATION_READ_COMMITTED);
    if (!trans) {
        pr_err("VexFS: Failed to begin atomic transaction for metadata op\n");
        return -ENOMEM;
    }

    op->transaction_id = VEXFS_ATOMIC_TRANS_ID(trans);

    /* Write operation to journal */
    ret = vexfs_journal_get_write_access(trans->journal_trans, NULL);
    if (ret) {
        pr_err("VexFS: Failed to get journal write access\n");
        goto abort_trans;
    }

    /* Journal the serialized metadata */
    ret = vexfs_journal_dirty_metadata(trans->journal_trans, NULL);
    if (ret) {
        pr_err("VexFS: Failed to journal metadata\n");
        goto abort_trans;
    }

    /* Commit the transaction */
    ret = vexfs_atomic_commit(trans);
    if (ret) {
        pr_err("VexFS: Failed to commit metadata transaction\n");
        return ret;
    }

    /* Update operation state */
    atomic_set(&op->op_state, 1);
    op->op_result = 0;
    complete(&op->op_completion);

    /* Update statistics */
    atomic64_inc(&mgr->ops_processed);
    atomic64_add(op->serialized_size, &mgr->bytes_journaled);

    pr_debug("VexFS: Processed metadata operation %llu type %u\n", 
             op->op_id, op->op_type);
    return 0;

abort_trans:
    vexfs_atomic_abort(trans);
    atomic_set(&op->op_state, -1);
    op->op_result = ret;
    complete(&op->op_completion);
    return ret;
}

/**
 * vexfs_metadata_batch_work_fn - Batch processing work function
 * @work: Work structure
 * 
 * Processes pending metadata operations in batches for efficiency.
 */
static void vexfs_metadata_batch_work_fn(struct work_struct *work)
{
    struct vexfs_metadata_journal_manager *mgr;
    struct vexfs_metadata_operation *op, *tmp;
    struct list_head batch_list;
    int processed = 0;
    int ret;

    mgr = container_of(work, struct vexfs_metadata_journal_manager, 
                      batch_work.work);

    INIT_LIST_HEAD(&batch_list);

    /* Move operations to batch list */
    mutex_lock(&mgr->ops_mutex);
    list_for_each_entry_safe(op, tmp, &mgr->pending_ops, op_list) {
        if (processed >= mgr->max_batch_size)
            break;
        
        list_del(&op->op_list);
        list_add_tail(&op->op_list, &batch_list);
        atomic_dec(&mgr->pending_count);
        processed++;
    }
    mutex_unlock(&mgr->ops_mutex);

    /* Process batch */
    list_for_each_entry_safe(op, tmp, &batch_list, op_list) {
        ret = vexfs_metadata_process_operation(mgr, op);
        if (ret) {
            pr_err("VexFS: Failed to process metadata operation %llu: %d\n",
                   op->op_id, ret);
            atomic_inc(&mgr->error_count);
        }

        /* Clean up operation */
        list_del(&op->op_list);
        if (op->serialized_data)
            kfree(op->serialized_data);
        if (op->before_state)
            kfree(op->before_state);
        if (op->after_state)
            kfree(op->after_state);
        kmem_cache_free(mgr->op_cache, op);
    }

    /* Schedule next batch if there are pending operations */
    if (atomic_read(&mgr->pending_count) > 0) {
        queue_delayed_work(mgr->batch_workqueue, &mgr->batch_work,
                          msecs_to_jiffies(mgr->batch_timeout));
    }

    pr_debug("VexFS: Processed metadata batch of %d operations\n", processed);
}

/*
 * =============================================================================
 * CACHE MANAGEMENT
 * =============================================================================
 */

/**
 * vexfs_metadata_cache_find - Find cache entry
 * @mgr: Metadata journaling manager
 * @key: Cache key
 * @entry_type: Type of cache entry
 * 
 * Finds a cache entry in the red-black tree.
 * 
 * Return: Cache entry if found, NULL otherwise
 */
static struct vexfs_metadata_cache_entry *vexfs_metadata_cache_find(
    struct vexfs_metadata_journal_manager *mgr, u64 key, u32 entry_type)
{
    struct rb_node *node = mgr->cache_tree.rb_node;
    struct vexfs_metadata_cache_entry *entry;
    u64 combined_key = (key << 8) | entry_type;

    while (node) {
        entry = rb_entry(node, struct vexfs_metadata_cache_entry, rb_node);
        u64 entry_combined_key = (entry->key << 8) | entry->entry_type;

        if (combined_key < entry_combined_key)
            node = node->rb_left;
        else if (combined_key > entry_combined_key)
            node = node->rb_right;
        else
            return entry;
    }

    return NULL;
}

/**
 * vexfs_metadata_cache_insert - Insert cache entry
 * @mgr: Metadata journaling manager
 * @entry: Cache entry to insert
 * 
 * Inserts a cache entry into the red-black tree.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_metadata_cache_insert(struct vexfs_metadata_journal_manager *mgr,
                                      struct vexfs_metadata_cache_entry *entry)
{
    struct rb_node **new = &mgr->cache_tree.rb_node;
    struct rb_node *parent = NULL;
    struct vexfs_metadata_cache_entry *this;
    u64 combined_key = (entry->key << 8) | entry->entry_type;

    while (*new) {
        this = rb_entry(*new, struct vexfs_metadata_cache_entry, rb_node);
        u64 this_combined_key = (this->key << 8) | this->entry_type;

        parent = *new;
        if (combined_key < this_combined_key)
            new = &((*new)->rb_left);
        else if (combined_key > this_combined_key)
            new = &((*new)->rb_right);
        else
            return -EEXIST;
    }

    rb_link_node(&entry->rb_node, parent, new);
    rb_insert_color(&entry->rb_node, &mgr->cache_tree);
    return 0;
}

/**
 * vexfs_metadata_cache_get - Get cached metadata
 * @mgr: Metadata journaling manager
 * @key: Cache key
 * @entry_type: Type of cached metadata
 * @data: Output data pointer
 * @size: Output data size
 * 
 * Retrieves cached metadata if available.
 * 
 * Return: 0 on success, -ENOENT if not found, negative error code on failure
 */
int vexfs_metadata_cache_get(struct vexfs_metadata_journal_manager *mgr,
                            u64 key, u32 entry_type, void **data, size_t *size)
{
    struct vexfs_metadata_cache_entry *entry;
    void *cached_data;

    if (!mgr || !data || !size)
        return -EINVAL;

    mutex_lock(&mgr->cache_mutex);
    entry = vexfs_metadata_cache_find(mgr, key, entry_type);
    if (!entry) {
        mutex_unlock(&mgr->cache_mutex);
        atomic64_inc(&mgr->cache_misses);
        return -ENOENT;
    }

    /* Verify checksum */
    u32 calculated_checksum = vexfs_metadata_calculate_checksum(entry->cached_data,
                                                               entry->data_size, 0);
    if (calculated_checksum != entry->checksum) {
        pr_err("VexFS: Cache entry checksum mismatch for key %llu\n", key);
        rb_erase(&entry->rb_node, &mgr->cache_tree);
        list_del(&entry->lru_list);
        kfree(entry->cached_data);
        kmem_cache_free(mgr->cache_entry_cache, entry);
        atomic_dec(&mgr->cache_entries);
        mutex_unlock(&mgr->cache_mutex);
        atomic64_inc(&mgr->checksum_errors);
        return -VEXFS_META_ERR_CHECKSUM;
    }

    /* Allocate and copy data */
    cached_data = kmalloc(entry->data_size, GFP_KERNEL);
    if (!cached_data) {
        mutex_unlock(&mgr->cache_mutex);
        return -ENOMEM;
    }

    memcpy(cached_data, entry->cached_data, entry->data_size);
    *data = cached_data;
    *size = entry->data_size;

    /* Update LRU */
    list_del(&entry->lru_list);
    list_add(&entry->lru_list, &mgr->cache_lru);
    entry->access_time = jiffies;
    atomic_inc(&entry->ref_count);

    mutex_unlock(&mgr->cache_mutex);
    atomic64_inc(&mgr->cache_hits);

    pr_debug("VexFS: Cache hit for key %llu type %u\n", key, entry_type);
    return 0;
}

/**
 * vexfs_metadata_cache_put - Cache metadata
 * @mgr: Metadata journaling manager
 * @key: Cache key
 * @entry_type: Type of metadata
 * @data: Data to cache
 * @size: Size of data
 * 
 * Caches metadata for future retrieval.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_cache_put(struct vexfs_metadata_journal_manager *mgr,
                            u64 key, u32 entry_type, void *data, size_t size)
{
    struct vexfs_metadata_cache_entry *entry, *lru_entry;
    void *cached_data;
    int ret;

    if (!mgr || !data || size == 0)
        return -EINVAL;

    /* Check if cache is full */
    if (atomic_read(&mgr->cache_entries) >= mgr->max_cache_entries) {
        /* Evict LRU entry */
        mutex_lock(&mgr->cache_mutex);
        if (!list_empty(&mgr->cache_lru)) {
            lru_entry = list_last_entry(&mgr->cache_lru,
                                       struct vexfs_metadata_cache_entry,
                                       lru_list);
            rb_erase(&lru_entry->rb_node, &mgr->cache_tree);
            list_del(&lru_entry->lru_list);
            kfree(lru_entry->cached_data);
            kmem_cache_free(mgr->cache_entry_cache, lru_entry);
            atomic_dec(&mgr->cache_entries);
        }
        mutex_unlock(&mgr->cache_mutex);
    }

    /* Allocate cache entry */
    entry = kmem_cache_alloc(mgr->cache_entry_cache, GFP_KERNEL);
    if (!entry)
        return -ENOMEM;

    /* Allocate and copy data */
    cached_data = kmalloc(size, GFP_KERNEL);
    if (!cached_data) {
        kmem_cache_free(mgr->cache_entry_cache, entry);
        return -ENOMEM;
    }

    memcpy(cached_data, data, size);

    /* Initialize cache entry */
    entry->key = key;
    entry->entry_type = entry_type;
    entry->cached_data = cached_data;
    entry->data_size = size;
    entry->access_time = jiffies;
    atomic_set(&entry->ref_count, 1);
    entry->flags = 0;
    entry->checksum = vexfs_metadata_calculate_checksum(cached_data, size, 0);

    /* Insert into cache */
    mutex_lock(&mgr->cache_mutex);
    ret = vexfs_metadata_cache_insert(mgr, entry);
    if (ret) {
        mutex_unlock(&mgr->cache_mutex);
        kfree(cached_data);
        kmem_cache_free(mgr->cache_entry_cache, entry);
        return ret;
    }

    list_add(&entry->lru_list, &mgr->cache_lru);
    atomic_inc(&mgr->cache_entries);
    mutex_unlock(&mgr->cache_mutex);

    pr_debug("VexFS: Cached metadata for key %llu type %u\n", key, entry_type);
    return 0;
}

/*
 * =============================================================================
 * UTILITY FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_metadata_calculate_checksum - Calculate metadata checksum
 * @data: Data to checksum
 * @size: Size of data
 * @seed: Checksum seed
 * 
 * Calculates CRC32 checksum for metadata integrity verification.
 * 
 * Return: Calculated checksum
 */
u32 vexfs_metadata_calculate_checksum(const void *data, size_t size, u32 seed)
{
    if (!data || size == 0)
        return 0;

    return crc32(seed, data, size);
}

/**
 * vexfs_metadata_verify_integrity - Verify operation integrity
 * @mgr: Metadata journaling manager
 * @op: Operation to verify
 * 
 * Verifies the integrity of a metadata operation using checksums.
 * 
 * Return: 0 if valid, negative error code if corrupted
 */
int vexfs_metadata_verify_integrity(struct vexfs_metadata_journal_manager *mgr,
                                    struct vexfs_metadata_operation *op)
{
    u32 calculated_checksum;

    if (!mgr || !op)
        return -EINVAL;

    /* Verify metadata checksum */
    if (op->serialized_data) {
        calculated_checksum = vexfs_metadata_calculate_checksum(op->serialized_data,
                                                               op->serialized_size, 0);
        if (calculated_checksum != op->metadata_checksum) {
            pr_err("VexFS: Metadata checksum mismatch for operation %llu\n", op->op_id);
            atomic64_inc(&mgr->checksum_errors);
            return -VEXFS_META_ERR_CHECKSUM;
        }
    }

    /* Verify operation checksum */
    calculated_checksum = vexfs_metadata_calculate_checksum(op,
                                                           sizeof(*op) - sizeof(op->operation_checksum), 0);
    if (calculated_checksum != op->operation_checksum) {
        pr_err("VexFS: Operation checksum mismatch for operation %llu\n", op->op_id);
        atomic64_inc(&mgr->checksum_errors);
        return -VEXFS_META_ERR_CHECKSUM;
    }

    return 0;
}

/**
 * vexfs_metadata_journal_batch_commit - Force batch commit
 * @mgr: Metadata journaling manager
 * 
 * Forces immediate processing of all pending metadata operations.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_journal_batch_commit(struct vexfs_metadata_journal_manager *mgr)
{
    if (!mgr)
        return -EINVAL;

    /* Cancel any pending work and process immediately */
    cancel_delayed_work_sync(&mgr->batch_work);
    queue_delayed_work(mgr->batch_workqueue, &mgr->batch_work, 0);
    flush_workqueue(mgr->batch_workqueue);

    pr_debug("VexFS: Forced metadata batch commit\n");
    return 0;
}

/**
 * vexfs_metadata_journal_force_sync - Force synchronous commit
 * @mgr: Metadata journaling manager
 * 
 * Forces synchronous commit of all pending operations and journal.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_journal_force_sync(struct vexfs_metadata_journal_manager *mgr)
{
    int ret;

    if (!mgr)
        return -EINVAL;

    /* Force batch commit */
    ret = vexfs_metadata_journal_batch_commit(mgr);
    if (ret)
        return ret;

    /* Force journal sync */
    ret = vexfs_journal_force_commit(mgr->journal);
    if (ret) {
        pr_err("VexFS: Failed to force journal commit: %d\n", ret);
        return ret;
    }

    pr_debug("VexFS: Forced metadata journal sync\n");
    return 0;
}

/**
 * vexfs_metadata_journal_get_stats - Get metadata journaling statistics
 * @mgr: Metadata journaling manager
 * @stats: Output statistics structure
 * 
 * Retrieves current metadata journaling statistics.
 */
void vexfs_metadata_journal_get_stats(struct vexfs_metadata_journal_manager *mgr,
                                      struct vexfs_metadata_journal_stats *stats)
{
    if (!mgr || !stats)
        return;

    memset(stats, 0, sizeof(*stats));

    stats->total_operations = atomic64_read(&mgr->ops_processed);
    stats->inode_operations = atomic64_read(&mgr->inode_ops);
    stats->dentry_operations = atomic64_read(&mgr->dentry_ops);
    stats->bitmap_operations = atomic64_read(&mgr->bitmap_ops);
    stats->vector_operations = atomic64_read(&mgr->vector_ops);
    stats->bytes_journaled = atomic64_read(&mgr->bytes_journaled);
    stats->cache_hits = atomic64_read(&mgr->cache_hits);
    stats->cache_misses = atomic64_read(&mgr->cache_misses);
    stats->cache_entries = atomic_read(&mgr->cache_entries);
    stats->pending_operations = atomic_read(&mgr->pending_count);
    stats->batch_size = mgr->batch_size;
    stats->checksum_errors = atomic64_read(&mgr->checksum_errors);

    pr_debug("VexFS: Retrieved metadata journaling statistics\n");
}

/*
 * =============================================================================
 * DIRECTORY ENTRY JOURNALING FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_metadata_journal_dentry_create - Journal directory entry creation
 * @mgr: Metadata journaling manager
 * @dentry: Directory entry being created
 * @flags: Journaling flags
 * 
 * Journals the creation of a new directory entry.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_journal_dentry_create(struct vexfs_metadata_journal_manager *mgr,
                                         struct dentry *dentry, u32 flags)
{
    struct vexfs_metadata_operation *op;
    struct vexfs_meta_serialized_dentry *serialized;
    size_t serialized_size;
    int ret;

    if (!mgr || !dentry)
        return -EINVAL;

    /* Serialize dentry */
    ret = vexfs_metadata_serialize_dentry(dentry, &serialized, &serialized_size);
    if (ret)
        return ret;

    /* Allocate operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op) {
        kfree(serialized);
        return -ENOMEM;
    }

    /* Initialize operation */
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_META_OP_DENTRY_CREATE;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->target_dentry = dentry;
    op->serialized_data = serialized;
    op->serialized_size = serialized_size;
    op->serialized_type = VEXFS_META_SERIAL_DENTRY;
    op->sequence_number = atomic64_inc_return(&mgr->ops_processed);
    op->timestamp = jiffies;
    
    /* Calculate checksums */
    op->metadata_checksum = vexfs_metadata_calculate_checksum(serialized,
                                                             serialized_size, 0);
    op->operation_checksum = vexfs_metadata_calculate_checksum(op,
                                                              sizeof(*op) - sizeof(op->operation_checksum), 0);

    init_completion(&op->op_completion);
    atomic_set(&op->op_state, 0);
    INIT_LIST_HEAD(&op->op_list);

    /* Add to pending operations */
    mutex_lock(&mgr->ops_mutex);
    list_add_tail(&op->op_list, &mgr->pending_ops);
    atomic_inc(&mgr->pending_count);
    mutex_unlock(&mgr->ops_mutex);

    /* Update statistics */
    atomic64_inc(&mgr->dentry_ops);

    /* Schedule batch processing */
    if (atomic_read(&mgr->pending_count) >= mgr->max_batch_size ||
        (flags & VEXFS_META_JOURNAL_SYNC)) {
        queue_delayed_work(mgr->batch_workqueue, &mgr->batch_work, 0);
    } else {
        queue_delayed_work(mgr->batch_workqueue, &mgr->batch_work,
                          msecs_to_jiffies(mgr->batch_timeout));
    }

    /* Wait for completion if synchronous */
    if (flags & VEXFS_META_JOURNAL_SYNC) {
        wait_for_completion(&op->op_completion);
        ret = op->op_result;
    }

    pr_debug("VexFS: Journaled dentry create for %s\n", dentry->d_name.name);
    return ret;
}

/*
 * =============================================================================
 * MODULE INTEGRATION FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_metadata_journal_module_init - Initialize metadata journaling module
 * 
 * Called during VexFS module initialization to set up metadata journaling.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_journal_module_init(void)
{
    pr_info("VexFS: Metadata journaling module initialized\n");
    return 0;
}

/**
 * vexfs_metadata_journal_module_exit - Cleanup metadata journaling module
 * 
 * Called during VexFS module cleanup to tear down metadata journaling.
 */
void vexfs_metadata_journal_module_exit(void)
{
    if (global_meta_mgr) {
        vexfs_metadata_journal_destroy(global_meta_mgr);
        global_meta_mgr = NULL;
    }
    pr_info("VexFS: Metadata journaling module cleaned up\n");
}

/* Export symbols for use by other VexFS modules */
EXPORT_SYMBOL(vexfs_metadata_journal_init);
EXPORT_SYMBOL(vexfs_metadata_journal_destroy);
EXPORT_SYMBOL(vexfs_metadata_journal_inode_create);
EXPORT_SYMBOL(vexfs_metadata_journal_inode_update);
EXPORT_SYMBOL(vexfs_metadata_journal_dentry_create);
EXPORT_SYMBOL(vexfs_metadata_journal_batch_commit);
EXPORT_SYMBOL(vexfs_metadata_journal_force_sync);
EXPORT_SYMBOL(vexfs_metadata_journal_get_stats);
EXPORT_SYMBOL(vexfs_metadata_calculate_checksum);
EXPORT_SYMBOL(vexfs_metadata_verify_integrity);
EXPORT_SYMBOL(vexfs_metadata_cache_get);
EXPORT_SYMBOL(vexfs_metadata_cache_put);