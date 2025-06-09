/*
 * VexFS v2.0 - Metadata Journaling Implementation (Task 3)
 * 
 * This implements comprehensive metadata journaling for VexFS as part of the
 * AI-Native Semantic Substrate roadmap (Phase 1). Builds on the Full FS Journal
 * (Task 1) and Atomic Operations (Task 2) to provide complete metadata integrity
 * and crash recovery for all VexFS metadata structures.
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

/* Global metadata journaling manager instance */
static struct vexfs_metadata_journal_manager *global_meta_mgr = NULL;

/* Forward declarations */
static void vexfs_metadata_batch_work_fn(struct work_struct *work);
static int vexfs_metadata_process_operation(struct vexfs_metadata_journal_manager *mgr,
                                           struct vexfs_metadata_operation *op);
static struct vexfs_metadata_cache_entry *vexfs_metadata_cache_find(
    struct vexfs_metadata_journal_manager *mgr, u64 key, u32 entry_type);

/*
 * =============================================================================
 * MANAGER INITIALIZATION AND CLEANUP
 * =============================================================================
 */

/**
 * vexfs_metadata_journal_init - Initialize metadata journaling manager
 * @journal: Associated journal instance
 * @atomic_mgr: Atomic operations manager
 * 
 * Initializes the metadata journaling manager with all necessary data structures
 * and worker threads for batch processing.
 * 
 * Return: Pointer to initialized manager, NULL on failure
 */
struct vexfs_metadata_journal_manager *vexfs_metadata_journal_init(
    struct vexfs_journal *journal,
    struct vexfs_atomic_manager *atomic_mgr)
{
    struct vexfs_metadata_journal_manager *mgr;
    int ret;

    if (!journal || !atomic_mgr) {
        pr_err("VexFS: Invalid parameters for metadata journal init\n");
        return NULL;
    }

    mgr = kzalloc(sizeof(*mgr), GFP_KERNEL);
    if (!mgr) {
        pr_err("VexFS: Failed to allocate metadata journal manager\n");
        return NULL;
    }

    /* Initialize basic fields */
    mgr->journal = journal;
    mgr->atomic_mgr = atomic_mgr;
    mgr->next_op_id = 1;
    mgr->batch_size = 0;
    mgr->max_batch_size = VEXFS_META_MAX_BATCH_SIZE;
    mgr->max_cache_entries = VEXFS_META_MAX_CACHE_ENTRIES;

    /* Initialize lists and synchronization */
    INIT_LIST_HEAD(&mgr->pending_ops);
    INIT_LIST_HEAD(&mgr->cache_lru);
    INIT_LIST_HEAD(&mgr->error_log);
    mutex_init(&mgr->ops_mutex);
    mutex_init(&mgr->cache_mutex);
    init_rwsem(&mgr->manager_rwsem);
    spin_lock_init(&mgr->stats_lock);

    /* Initialize cache tree */
    mgr->cache_tree = RB_ROOT;

    /* Initialize atomic counters */
    atomic_set(&mgr->pending_count, 0);
    atomic_set(&mgr->cache_entries, 0);
    atomic_set(&mgr->error_count, 0);
    atomic64_set(&mgr->ops_processed, 0);
    atomic64_set(&mgr->cache_hits, 0);
    atomic64_set(&mgr->cache_misses, 0);
    atomic64_set(&mgr->bytes_journaled, 0);
    atomic64_set(&mgr->inode_ops, 0);
    atomic64_set(&mgr->dentry_ops, 0);
    atomic64_set(&mgr->bitmap_ops, 0);
    atomic64_set(&mgr->vector_ops, 0);
    atomic64_set(&mgr->checksum_errors, 0);

    /* Create memory caches */
    mgr->op_cache = kmem_cache_create("vexfs_meta_op",
                                     sizeof(struct vexfs_metadata_operation),
                                     0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->op_cache) {
        pr_err("VexFS: Failed to create operation cache\n");
        goto err_free_mgr;
    }

    mgr->cache_entry_cache = kmem_cache_create("vexfs_meta_cache_entry",
                                              sizeof(struct vexfs_metadata_cache_entry),
                                              0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->cache_entry_cache) {
        pr_err("VexFS: Failed to create cache entry cache\n");
        goto err_destroy_op_cache;
    }

    mgr->inode_serial_cache = kmem_cache_create("vexfs_meta_inode_serial",
                                               sizeof(struct vexfs_meta_serialized_inode),
                                               0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->inode_serial_cache) {
        pr_err("VexFS: Failed to create inode serialization cache\n");
        goto err_destroy_cache_entry_cache;
    }

    mgr->dentry_serial_cache = kmem_cache_create("vexfs_meta_dentry_serial",
                                                sizeof(struct vexfs_meta_serialized_dentry) + 256,
                                                0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->dentry_serial_cache) {
        pr_err("VexFS: Failed to create dentry serialization cache\n");
        goto err_destroy_inode_serial_cache;
    }

    mgr->bitmap_serial_cache = kmem_cache_create("vexfs_meta_bitmap_serial",
                                                sizeof(struct vexfs_meta_serialized_bitmap),
                                                0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->bitmap_serial_cache) {
        pr_err("VexFS: Failed to create bitmap serialization cache\n");
        goto err_destroy_dentry_serial_cache;
    }

    mgr->vector_serial_cache = kmem_cache_create("vexfs_meta_vector_serial",
                                                sizeof(struct vexfs_meta_serialized_vector),
                                                0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->vector_serial_cache) {
        pr_err("VexFS: Failed to create vector serialization cache\n");
        goto err_destroy_bitmap_serial_cache;
    }

    /* Create workqueue for batch processing */
    mgr->batch_workqueue = alloc_workqueue("vexfs_meta_batch",
                                          WQ_MEM_RECLAIM | WQ_UNBOUND, 1);
    if (!mgr->batch_workqueue) {
        pr_err("VexFS: Failed to create batch workqueue\n");
        goto err_destroy_vector_serial_cache;
    }

    /* Initialize batch work */
    INIT_DELAYED_WORK(&mgr->batch_work, vexfs_metadata_batch_work_fn);

    /* Set configuration defaults */
    mgr->journal_flags = VEXFS_META_JOURNAL_CHECKSUM | VEXFS_META_JOURNAL_ORDERED;
    mgr->sync_mode = VEXFS_META_JOURNAL_ASYNC;
    mgr->batch_timeout = 100; /* 100ms */

    /* Set global manager */
    global_meta_mgr = mgr;

    pr_info("VexFS: Metadata journaling manager initialized successfully\n");
    return mgr;

err_destroy_vector_serial_cache:
    kmem_cache_destroy(mgr->vector_serial_cache);
err_destroy_bitmap_serial_cache:
    kmem_cache_destroy(mgr->bitmap_serial_cache);
err_destroy_dentry_serial_cache:
    kmem_cache_destroy(mgr->dentry_serial_cache);
err_destroy_inode_serial_cache:
    kmem_cache_destroy(mgr->inode_serial_cache);
err_destroy_cache_entry_cache:
    kmem_cache_destroy(mgr->cache_entry_cache);
err_destroy_op_cache:
    kmem_cache_destroy(mgr->op_cache);
err_free_mgr:
    kfree(mgr);
    return NULL;
}

/**
 * vexfs_metadata_journal_destroy - Destroy metadata journaling manager
 * @mgr: Manager to destroy
 * 
 * Cleans up all resources associated with the metadata journaling manager.
 */
void vexfs_metadata_journal_destroy(struct vexfs_metadata_journal_manager *mgr)
{
    struct vexfs_metadata_operation *op, *tmp_op;
    struct vexfs_metadata_cache_entry *entry, *tmp_entry;

    if (!mgr)
        return;

    /* Cancel any pending work */
    cancel_delayed_work_sync(&mgr->batch_work);

    /* Destroy workqueue */
    if (mgr->batch_workqueue) {
        destroy_workqueue(mgr->batch_workqueue);
    }

    /* Clean up pending operations */
    mutex_lock(&mgr->ops_mutex);
    list_for_each_entry_safe(op, tmp_op, &mgr->pending_ops, op_list) {
        list_del(&op->op_list);
        if (op->serialized_data)
            kfree(op->serialized_data);
        if (op->before_state)
            kfree(op->before_state);
        if (op->after_state)
            kfree(op->after_state);
        kmem_cache_free(mgr->op_cache, op);
    }
    mutex_unlock(&mgr->ops_mutex);

    /* Clean up cache entries */
    mutex_lock(&mgr->cache_mutex);
    list_for_each_entry_safe(entry, tmp_entry, &mgr->cache_lru, lru_list) {
        list_del(&entry->lru_list);
        if (entry->cached_data)
            kfree(entry->cached_data);
        kmem_cache_free(mgr->cache_entry_cache, entry);
    }
    mutex_unlock(&mgr->cache_mutex);

    /* Destroy memory caches */
    if (mgr->vector_serial_cache)
        kmem_cache_destroy(mgr->vector_serial_cache);
    if (mgr->bitmap_serial_cache)
        kmem_cache_destroy(mgr->bitmap_serial_cache);
    if (mgr->dentry_serial_cache)
        kmem_cache_destroy(mgr->dentry_serial_cache);
    if (mgr->inode_serial_cache)
        kmem_cache_destroy(mgr->inode_serial_cache);
    if (mgr->cache_entry_cache)
        kmem_cache_destroy(mgr->cache_entry_cache);
    if (mgr->op_cache)
        kmem_cache_destroy(mgr->op_cache);

    /* Clear global manager */
    if (global_meta_mgr == mgr)
        global_meta_mgr = NULL;

    kfree(mgr);
    pr_info("VexFS: Metadata journaling manager destroyed\n");
}

/*
 * =============================================================================
 * SERIALIZATION FUNCTIONS
 * =============================================================================
 */

/**
 * vexfs_metadata_serialize_inode - Serialize inode metadata
 * @inode: Inode to serialize
 * @serialized: Output serialized inode structure
 * 
 * Serializes inode metadata into a kernel-compatible format for journaling.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_serialize_inode(struct inode *inode,
                                   struct vexfs_meta_serialized_inode *serialized)
{
    struct vexfs_v2_inode_info *vexfs_inode;
    u32 checksum;

    if (!inode || !serialized)
        return -EINVAL;

    vexfs_inode = VEXFS_V2_I(inode);
    memset(serialized, 0, sizeof(*serialized));

    /* Basic inode fields */
    serialized->ino = cpu_to_le64(inode->i_ino);
    serialized->mode = cpu_to_le32(inode->i_mode);
    serialized->uid = cpu_to_le32(i_uid_read(inode));
    serialized->gid = cpu_to_le32(i_gid_read(inode));
    serialized->size = cpu_to_le64(inode->i_size);
    serialized->blocks = cpu_to_le64(inode->i_blocks);

    /* Timestamps */
    serialized->atime_sec = cpu_to_le64(inode->i_atime.tv_sec);
    serialized->atime_nsec = cpu_to_le32(inode->i_atime.tv_nsec);
    serialized->mtime_sec = cpu_to_le64(inode->i_mtime.tv_sec);
    serialized->mtime_nsec = cpu_to_le32(inode->i_mtime.tv_nsec);
    serialized->ctime_sec = cpu_to_le64(inode->i_ctime.tv_sec);
    serialized->ctime_nsec = cpu_to_le32(inode->i_ctime.tv_nsec);
    serialized->crtime_sec = cpu_to_le64(vexfs_inode->i_crtime.tv_sec);
    serialized->crtime_nsec = cpu_to_le32(vexfs_inode->i_crtime.tv_nsec);

    /* VexFS-specific fields */
    serialized->i_flags = cpu_to_le32(vexfs_inode->i_flags);
    memcpy(serialized->i_block, vexfs_inode->i_block, sizeof(serialized->i_block));

    /* Vector-specific metadata */
    serialized->is_vector_file = vexfs_inode->is_vector_file;
    serialized->vector_element_type = vexfs_inode->vector_element_type;
    serialized->vector_dimensions = cpu_to_le16(vexfs_inode->vector_dimensions);
    serialized->vector_count = cpu_to_le32(vexfs_inode->vector_count);
    serialized->vector_alignment = cpu_to_le32(vexfs_inode->vector_alignment);
    serialized->vectors_per_block = cpu_to_le32(vexfs_inode->vectors_per_block);
    serialized->vector_data_size = cpu_to_le64(vexfs_inode->vector_data_size);
    serialized->hnsw_graph_block = cpu_to_le64(vexfs_inode->hnsw_graph_block);
    serialized->pq_codebook_block = cpu_to_le64(vexfs_inode->pq_codebook_block);
    serialized->hnsw_max_connections = cpu_to_le32(vexfs_inode->hnsw_max_connections);
    serialized->hnsw_ef_construction = cpu_to_le32(vexfs_inode->hnsw_ef_construction);
    serialized->vector_flags = cpu_to_le32(vexfs_inode->vector_flags);
    serialized->access_pattern = cpu_to_le32(vexfs_inode->access_pattern);
    serialized->storage_format = cpu_to_le32(vexfs_inode->storage_format);
    serialized->compression_type = cpu_to_le32(vexfs_inode->compression_type);
    serialized->data_offset = cpu_to_le64(vexfs_inode->data_offset);
    serialized->index_offset = cpu_to_le64(vexfs_inode->index_offset);

    /* Calculate checksum */
    checksum = vexfs_metadata_calculate_checksum(serialized,
                                                sizeof(*serialized) - sizeof(serialized->checksum),
                                                0);
    serialized->checksum = cpu_to_le32(checksum);

    return 0;
}

/**
 * vexfs_metadata_deserialize_inode - Deserialize inode metadata
 * @serialized: Serialized inode structure
 * @inode: Target inode to populate
 * 
 * Deserializes inode metadata from journaled format back to kernel structures.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_deserialize_inode(struct vexfs_meta_serialized_inode *serialized,
                                     struct inode *inode)
{
    struct vexfs_v2_inode_info *vexfs_inode;
    u32 stored_checksum, calculated_checksum;

    if (!serialized || !inode)
        return -EINVAL;

    /* Verify checksum */
    stored_checksum = le32_to_cpu(serialized->checksum);
    calculated_checksum = vexfs_metadata_calculate_checksum(serialized,
                                                           sizeof(*serialized) - sizeof(serialized->checksum),
                                                           0);
    if (stored_checksum != calculated_checksum) {
        pr_err("VexFS: Inode deserialization checksum mismatch\n");
        return -VEXFS_META_ERR_CHECKSUM;
    }

    vexfs_inode = VEXFS_V2_I(inode);

    /* Basic inode fields */
    inode->i_ino = le64_to_cpu(serialized->ino);
    inode->i_mode = le32_to_cpu(serialized->mode);
    i_uid_write(inode, le32_to_cpu(serialized->uid));
    i_gid_write(inode, le32_to_cpu(serialized->gid));
    inode->i_size = le64_to_cpu(serialized->size);
    inode->i_blocks = le64_to_cpu(serialized->blocks);

    /* Timestamps */
    inode->i_atime.tv_sec = le64_to_cpu(serialized->atime_sec);
    inode->i_atime.tv_nsec = le32_to_cpu(serialized->atime_nsec);
    inode->i_mtime.tv_sec = le64_to_cpu(serialized->mtime_sec);
    inode->i_mtime.tv_nsec = le32_to_cpu(serialized->mtime_nsec);
    inode->i_ctime.tv_sec = le64_to_cpu(serialized->ctime_sec);
    inode->i_ctime.tv_nsec = le32_to_cpu(serialized->ctime_nsec);
    vexfs_inode->i_crtime.tv_sec = le64_to_cpu(serialized->crtime_sec);
    vexfs_inode->i_crtime.tv_nsec = le32_to_cpu(serialized->crtime_nsec);

    /* VexFS-specific fields */
    vexfs_inode->i_flags = le32_to_cpu(serialized->i_flags);
    memcpy(vexfs_inode->i_block, serialized->i_block, sizeof(vexfs_inode->i_block));

    /* Vector-specific metadata */
    vexfs_inode->is_vector_file = serialized->is_vector_file;
    vexfs_inode->vector_element_type = serialized->vector_element_type;
    vexfs_inode->vector_dimensions = le16_to_cpu(serialized->vector_dimensions);
    vexfs_inode->vector_count = le32_to_cpu(serialized->vector_count);
    vexfs_inode->vector_alignment = le32_to_cpu(serialized->vector_alignment);
    vexfs_inode->vectors_per_block = le32_to_cpu(serialized->vectors_per_block);
    vexfs_inode->vector_data_size = le64_to_cpu(serialized->vector_data_size);
    vexfs_inode->hnsw_graph_block = le64_to_cpu(serialized->hnsw_graph_block);
    vexfs_inode->pq_codebook_block = le64_to_cpu(serialized->pq_codebook_block);
    vexfs_inode->hnsw_max_connections = le32_to_cpu(serialized->hnsw_max_connections);
    vexfs_inode->hnsw_ef_construction = le32_to_cpu(serialized->hnsw_ef_construction);
    vexfs_inode->vector_flags = le32_to_cpu(serialized->vector_flags);
    vexfs_inode->access_pattern = le32_to_cpu(serialized->access_pattern);
    vexfs_inode->storage_format = le32_to_cpu(serialized->storage_format);
    vexfs_inode->compression_type = le32_to_cpu(serialized->compression_type);
    vexfs_inode->data_offset = le64_to_cpu(serialized->data_offset);
    vexfs_inode->index_offset = le64_to_cpu(serialized->index_offset);

    return 0;
}

/**
 * vexfs_metadata_serialize_dentry - Serialize directory entry metadata
 * @dentry: Directory entry to serialize
 * @serialized: Output serialized dentry structure (allocated)
 * @size: Output size of serialized structure
 * 
 * Serializes directory entry metadata with variable-length name.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_serialize_dentry(struct dentry *dentry,
                                    struct vexfs_meta_serialized_dentry **serialized,
                                    size_t *size)
{
    struct vexfs_meta_serialized_dentry *ser;
    size_t name_len, total_size;
    const char *name;

    if (!dentry || !serialized || !size)
        return -EINVAL;

    name = dentry->d_name.name;
    name_len = dentry->d_name.len;
    total_size = sizeof(*ser) + name_len + 1; /* +1 for null terminator */

    ser = kzalloc(total_size, GFP_KERNEL);
    if (!ser)
        return -ENOMEM;

    /* Fill in metadata */
    ser->parent_ino = cpu_to_le64(d_inode(dentry->d_parent)->i_ino);
    ser->child_ino = cpu_to_le64(d_inode(dentry)->i_ino);
    ser->name_len = cpu_to_le32(name_len);
    ser->entry_type = cpu_to_le32(d_inode(dentry)->i_mode & S_IFMT);
    ser->hash = cpu_to_le64(dentry->d_name.hash);

    /* Copy name */
    memcpy(ser->name, name, name_len);
    ser->name[name_len] = '\0';

    *serialized = ser;
    *size = total_size;

    return 0;
}

/*
 * =============================================================================
 * INODE METADATA JOURNALING
 * =============================================================================
 */

/**
 * vexfs_metadata_journal_inode_create - Journal inode creation
 * @mgr: Metadata journaling manager
 * @inode: Inode being created
 * @flags: Journaling flags
 * 
 * Journals the creation of a new inode with all metadata.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_journal_inode_create(struct vexfs_metadata_journal_manager *mgr,
                                        struct inode *inode, u32 flags)
{
    struct vexfs_metadata_operation *op;
    struct vexfs_meta_serialized_inode *serialized;
    int ret;

    if (!mgr || !inode)
        return -EINVAL;

    /* Allocate operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op)
        return -ENOMEM;

    /* Allocate serialized inode */
    serialized = kmem_cache_alloc(mgr->inode_serial_cache, GFP_KERNEL);
    if (!serialized) {
        kmem_cache_free(mgr->op_cache, op);
        return -ENOMEM;
    }

    /* Serialize inode metadata */
    ret = vexfs_metadata_serialize_inode(inode, serialized);
    if (ret) {
        kmem_cache_free(mgr->inode_serial_cache, serialized);
        kmem_cache_free(mgr->op_cache, op);
        return ret;
    }

    /* Initialize operation */
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_META_OP_INODE_CREATE;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->target_inode = inode;
    op->serialized_data = serialized;
    op->serialized_size = sizeof(*serialized);
    op->serialized_type = VEXFS_META_SERIAL_INODE;
    op->sequence_number = atomic64_inc_return(&mgr->ops_processed);
    op->timestamp = jiffies;
    
    /* Calculate checksums */
    op->metadata_checksum = vexfs_metadata_calculate_checksum(serialized,
                                                             sizeof(*serialized), 0);
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
    atomic64_inc(&mgr->inode_ops);

    /* Schedule batch processing if needed */
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

    pr_debug("VexFS: Journaled inode create for ino %lu\n", inode->i_ino);
    return ret;
}

/**
 * vexfs_metadata_journal_inode_update - Journal inode update
 * @mgr: Metadata journaling manager
 * @inode: Inode being updated
 * @flags: Journaling flags
 * 
 * Journals updates to an existing inode's metadata.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_metadata_journal_inode_update(struct vexfs_metadata_journal_manager *mgr,
                                        struct inode *inode, u32 flags)
{
    struct vexfs_metadata_operation *op;
    struct vexfs_meta_serialized_inode *serialized;
    int ret;

    if (!mgr || !inode)
        return -EINVAL;

    /* Allocate operation */
    op = kmem_cache_alloc(mgr->op_cache, GFP_KERNEL);
    if (!op)
        return -ENOMEM;

    /* Allocate serialized inode */
    serialized = kmem_cache_alloc(mgr->inode_serial_cache, GFP_KERNEL);
    if (!serialized) {
        kmem_cache_free(mgr->op_cache, op);
        return -ENOMEM;
    }

    /* Serialize current inode state */
    ret = vexfs_metadata_serialize_inode(inode, serialized);
    if (ret) {
        kmem_cache_free(mgr->inode_serial_cache, serialized);
        kmem_cache_free(mgr->op_cache, op);
        return ret;
    }

    /* Initialize operation */
    memset(op, 0, sizeof(*op));
    op->op_type = VEXFS_META_OP_INODE_UPDATE;
    op->op_flags = flags;
    op->op_id = atomic64_inc_return((atomic64_t *)&mgr->next_op_id);
    op->target_inode = inode;
    op->serialized_data = serialized;
    op->serialized_size = sizeof(*serialized);
    op->serialized_type = VEXFS_META_SERIAL_INODE;
    op->sequence_number = atomic64_inc_return(&mgr->ops_processed);
    op->timestamp = jiffies;
    
    /* Calculate checksums */
    op->metadata_checksum = vexfs_metadata_calculate_checksum(serialized,
                                                             sizeof(*serialized), 0);
    op->operation_checksum = vexfs_metadata_calculate_checksum(op,
                                                              sizeof(*op) - sizeof(op->operation_checksum), 0);

    init_completion(&op->op_completion);
    atomic_set(&op->op_state, 0);
    INIT_LIST_HEAD(&op->op_list);

    /* Add to pending operations */
    mutex_lock(&mgr->ops_mutex);
    list_add_tail(&op->op_list, &mgr->pending_ops);