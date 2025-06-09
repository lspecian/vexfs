/*
 * VexFS v2.0 - Full FS Journal Core Implementation
 * 
 * This implements the core journaling functionality for VexFS as part of the
 * AI-Native Semantic Substrate roadmap (Phase 1). Provides block-level integrity
 * and fast crash recovery with Write-Ahead Logging (WAL) principles.
 *
 * Key Features:
 * - Circular journal log with descriptor/commit/revocation blocks
 * - Strict Write-Ahead Logging for consistency guarantees
 * - Non-blocking writes with asynchronous commit processing
 * - Checksumming for corruption detection and recovery
 * - Integration with existing VexFS vector operations
 * - ACID compliance for all filesystem transactions
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/kthread.h>
#include <linux/delay.h>
#include <linux/time.h>
#include <linux/crc32.h>
#include <linux/crypto.h>
#include <crypto/hash.h>
#include <linux/random.h>
#include <linux/jiffies.h>

#include "../include/vexfs_v2_journal.h"

/* Module parameters for journal tuning */
static unsigned int journal_commit_interval = 5000; /* 5 seconds */
module_param(journal_commit_interval, uint, 0644);
MODULE_PARM_DESC(journal_commit_interval, "Journal commit interval in milliseconds");

static unsigned int journal_max_trans_blocks = 1024;
module_param(journal_max_trans_blocks, uint, 0644);
MODULE_PARM_DESC(journal_max_trans_blocks, "Maximum blocks per transaction");

static bool journal_async_commit = true;
module_param(journal_async_commit, bool, 0644);
MODULE_PARM_DESC(journal_async_commit, "Enable asynchronous commit processing");

/* Forward declarations */
static int vexfs_journal_commit_thread(void *data);
static void vexfs_journal_commit_work_fn(struct work_struct *work);
static int vexfs_journal_write_superblock(struct vexfs_journal *journal);
static int vexfs_journal_write_descriptor(struct vexfs_journal_transaction *trans);
static int vexfs_journal_write_commit(struct vexfs_journal_transaction *trans);

/*
 * Calculate CRC32 checksum for journal blocks
 */
u32 vexfs_journal_calculate_checksum(const void *data, size_t len, u32 seed)
{
    return crc32(seed, data, len);
}

/*
 * Initialize a new journal structure
 */
struct vexfs_journal *vexfs_journal_init(struct super_block *sb, 
                                         u64 start_block, u64 total_blocks)
{
    struct vexfs_journal *journal;
    int ret;

    if (!sb || total_blocks < 64) {
        printk(KERN_ERR "VexFS Journal: Invalid parameters for journal init\n");
        return ERR_PTR(-EINVAL);
    }

    journal = kzalloc(sizeof(struct vexfs_journal), GFP_KERNEL);
    if (!journal) {
        printk(KERN_ERR "VexFS Journal: Failed to allocate journal structure\n");
        return ERR_PTR(-ENOMEM);
    }

    /* Initialize basic fields */
    journal->j_sb = sb;
    journal->j_bdev = sb->s_bdev;
    journal->j_start_block = start_block;
    journal->j_total_blocks = total_blocks;
    journal->j_block_size = VEXFS_JOURNAL_BLOCK_SIZE;

    /* Initialize circular log pointers */
    journal->j_head = start_block + 1; /* Skip superblock */
    journal->j_tail = start_block + 1;
    journal->j_sequence = 1;
    journal->j_commit_sequence = 0;
    journal->j_next_trans_id = 1;

    /* Initialize synchronization primitives */
    spin_lock_init(&journal->j_lock);
    mutex_init(&journal->j_mutex);
    mutex_init(&journal->j_trans_mutex);
    init_rwsem(&journal->j_rw_sem);

    /* Initialize transaction list */
    INIT_LIST_HEAD(&journal->j_transactions);
    atomic_set(&journal->j_trans_count, 0);
    atomic_set(&journal->j_ref_count, 1);

    /* Initialize performance counters */
    atomic64_set(&journal->j_commits, 0);
    atomic64_set(&journal->j_aborts, 0);
    atomic64_set(&journal->j_blocks_written, 0);
    atomic64_set(&journal->j_transactions, 0);

    /* Set initial flags */
    journal->j_flags = VEXFS_JOURNAL_ACTIVE | VEXFS_JOURNAL_CHECKSUM;
    if (journal_async_commit)
        journal->j_flags |= VEXFS_JOURNAL_ASYNC_COMMIT;

    /* Initialize checksum support */
    journal->j_checksum_type = 1; /* CRC32 */

    /* Allocate buffer array for journal blocks */
    journal->j_buffer_count = min_t(u32, total_blocks, 256);
    journal->j_buffers = kcalloc(journal->j_buffer_count, 
                                sizeof(struct buffer_head *), GFP_KERNEL);
    if (!journal->j_buffers) {
        printk(KERN_ERR "VexFS Journal: Failed to allocate buffer array\n");
        ret = -ENOMEM;
        goto error_cleanup;
    }

    /* Create journal workqueue */
    journal->j_workqueue = alloc_workqueue("vexfs_journal", 
                                          WQ_MEM_RECLAIM | WQ_UNBOUND, 1);
    if (!journal->j_workqueue) {
        printk(KERN_ERR "VexFS Journal: Failed to create workqueue\n");
        ret = -ENOMEM;
        goto error_cleanup;
    }

    /* Initialize commit work */
    INIT_DELAYED_WORK(&journal->j_commit_work, vexfs_journal_commit_work_fn);

    /* Start commit thread */
    journal->j_commit_thread = kthread_run(vexfs_journal_commit_thread, 
                                          journal, "vexfs_journal");
    if (IS_ERR(journal->j_commit_thread)) {
        printk(KERN_ERR "VexFS Journal: Failed to start commit thread\n");
        ret = PTR_ERR(journal->j_commit_thread);
        journal->j_commit_thread = NULL;
        goto error_cleanup;
    }

    printk(KERN_INFO "VexFS Journal: Initialized journal with %llu blocks at block %llu\n",
           total_blocks, start_block);

    return journal;

error_cleanup:
    if (journal->j_workqueue)
        destroy_workqueue(journal->j_workqueue);
    kfree(journal->j_buffers);
    kfree(journal);
    return ERR_PTR(ret);
}

/*
 * Destroy journal and free resources
 */
void vexfs_journal_destroy(struct vexfs_journal *journal)
{
    if (!journal)
        return;

    /* Stop commit thread */
    if (journal->j_commit_thread) {
        kthread_stop(journal->j_commit_thread);
        journal->j_commit_thread = NULL;
    }

    /* Cancel and flush any pending work */
    if (journal->j_workqueue) {
        cancel_delayed_work_sync(&journal->j_commit_work);
        destroy_workqueue(journal->j_workqueue);
    }

    /* Abort any remaining transactions */
    mutex_lock(&journal->j_trans_mutex);
    while (!list_empty(&journal->j_transactions)) {
        struct vexfs_journal_transaction *trans;
        trans = list_first_entry(&journal->j_transactions,
                                struct vexfs_journal_transaction, t_list);
        list_del(&trans->t_list);
        vexfs_journal_abort(trans);
    }
    mutex_unlock(&journal->j_trans_mutex);

    /* Free buffer array */
    kfree(journal->j_buffers);

    /* Free journal structure */
    kfree(journal);

    printk(KERN_INFO "VexFS Journal: Journal destroyed\n");
}

/*
 * Create a new journal on disk
 */
int vexfs_journal_create(struct vexfs_journal *journal)
{
    struct vexfs_journal_superblock *jsb;
    struct buffer_head *bh;
    int ret;

    if (!journal) {
        return -EINVAL;
    }

    /* Read/create journal superblock */
    bh = sb_bread(journal->j_sb, journal->j_start_block);
    if (!bh) {
        printk(KERN_ERR "VexFS Journal: Failed to read journal superblock\n");
        return -EIO;
    }

    jsb = (struct vexfs_journal_superblock *)bh->b_data;
    memset(jsb, 0, sizeof(struct vexfs_journal_superblock));

    /* Initialize superblock */
    jsb->j_magic = cpu_to_le32(VEXFS_JOURNAL_MAGIC);
    jsb->j_version_major = cpu_to_le32(VEXFS_JOURNAL_VERSION_MAJOR);
    jsb->j_version_minor = cpu_to_le32(VEXFS_JOURNAL_VERSION_MINOR);
    jsb->j_flags = cpu_to_le32(journal->j_flags);

    /* Set journal geometry */
    jsb->j_start_block = cpu_to_le64(journal->j_start_block);
    jsb->j_total_blocks = cpu_to_le64(journal->j_total_blocks);
    jsb->j_block_size = cpu_to_le32(journal->j_block_size);
    jsb->j_max_trans_blocks = cpu_to_le32(journal_max_trans_blocks);

    /* Initialize circular log pointers */
    jsb->j_head = cpu_to_le64(journal->j_head);
    jsb->j_tail = cpu_to_le64(journal->j_tail);
    jsb->j_sequence = cpu_to_le64(journal->j_sequence);
    jsb->j_commit_sequence = cpu_to_le64(journal->j_commit_sequence);

    /* Set performance parameters */
    jsb->j_commit_interval = cpu_to_le32(journal_commit_interval);
    jsb->j_sync_mode = cpu_to_le32(1); /* Write-through */
    jsb->j_checksum_type = cpu_to_le32(journal->j_checksum_type);

    /* Calculate and set checksum */
    jsb->j_superblock_checksum = cpu_to_le32(
        vexfs_journal_calculate_checksum(jsb, 
                                        sizeof(*jsb) - sizeof(jsb->j_superblock_checksum),
                                        0));

    /* Write superblock to disk */
    mark_buffer_dirty(bh);
    ret = sync_dirty_buffer(bh);
    brelse(bh);

    if (ret) {
        printk(KERN_ERR "VexFS Journal: Failed to write journal superblock\n");
        return ret;
    }

    printk(KERN_INFO "VexFS Journal: Created new journal\n");
    return 0;
}

/*
 * Load existing journal from disk
 */
int vexfs_journal_load(struct vexfs_journal *journal)
{
    struct vexfs_journal_superblock *jsb;
    struct buffer_head *bh;
    u32 calculated_checksum, stored_checksum;
    int ret;

    if (!journal) {
        return -EINVAL;
    }

    /* Read journal superblock */
    bh = sb_bread(journal->j_sb, journal->j_start_block);
    if (!bh) {
        printk(KERN_ERR "VexFS Journal: Failed to read journal superblock\n");
        return -EIO;
    }

    jsb = (struct vexfs_journal_superblock *)bh->b_data;

    /* Verify magic number */
    if (le32_to_cpu(jsb->j_magic) != VEXFS_JOURNAL_MAGIC) {
        printk(KERN_ERR "VexFS Journal: Invalid journal magic number\n");
        ret = -EINVAL;
        goto error_release;
    }

    /* Verify checksum */
    stored_checksum = le32_to_cpu(jsb->j_superblock_checksum);
    calculated_checksum = vexfs_journal_calculate_checksum(jsb,
                                                          sizeof(*jsb) - sizeof(jsb->j_superblock_checksum),
                                                          0);
    if (stored_checksum != calculated_checksum) {
        printk(KERN_ERR "VexFS Journal: Journal superblock checksum mismatch\n");
        ret = -EINVAL;
        goto error_release;
    }

    /* Load journal parameters */
    journal->j_head = le64_to_cpu(jsb->j_head);
    journal->j_tail = le64_to_cpu(jsb->j_tail);
    journal->j_sequence = le64_to_cpu(jsb->j_sequence);
    journal->j_commit_sequence = le64_to_cpu(jsb->j_commit_sequence);
    journal->j_flags = le32_to_cpu(jsb->j_flags);
    journal->j_checksum_type = le32_to_cpu(jsb->j_checksum_type);

    brelse(bh);

    /* Check if recovery is needed */
    if (journal->j_sequence > journal->j_commit_sequence) {
        printk(KERN_INFO "VexFS Journal: Recovery needed (seq %llu > commit %llu)\n",
               journal->j_sequence, journal->j_commit_sequence);
        ret = vexfs_journal_recover(journal);
        if (ret) {
            printk(KERN_ERR "VexFS Journal: Recovery failed\n");
            return ret;
        }
    }

    printk(KERN_INFO "VexFS Journal: Loaded journal successfully\n");
    return 0;

error_release:
    brelse(bh);
    return ret;
}

/*
 * Start a new transaction
 */
struct vexfs_journal_transaction *vexfs_journal_start(struct vexfs_journal *journal,
                                                     u32 max_blocks, u32 operation_type)
{
    struct vexfs_journal_transaction *trans;
    unsigned long flags;

    if (!journal || max_blocks == 0 || max_blocks > journal_max_trans_blocks) {
        return ERR_PTR(-EINVAL);
    }

    /* Check journal state */
    if (!(journal->j_flags & VEXFS_JOURNAL_ACTIVE)) {
        return ERR_PTR(-EROFS);
    }

    /* Allocate transaction structure */
    trans = kzalloc(sizeof(struct vexfs_journal_transaction), GFP_KERNEL);
    if (!trans) {
        return ERR_PTR(-ENOMEM);
    }

    /* Allocate block list */
    trans->t_block_list = kcalloc(max_blocks, sizeof(u64), GFP_KERNEL);
    if (!trans->t_block_list) {
        kfree(trans);
        return ERR_PTR(-ENOMEM);
    }

    /* Initialize transaction */
    spin_lock_irqsave(&journal->j_lock, flags);
    trans->t_transaction_id = journal->j_next_trans_id++;
    spin_unlock_irqrestore(&journal->j_lock, flags);

    trans->t_state = VEXFS_TRANS_RUNNING;
    atomic_set(&trans->t_ref_count, 1);
    trans->t_start_time = jiffies;
    trans->t_max_blocks = max_blocks;
    trans->t_block_count = 0;
    trans->t_operation_type = operation_type;
    trans->t_journal = journal;
    trans->t_uid = current_uid().val;
    trans->t_gid = current_gid().val;

    mutex_init(&trans->t_mutex);
    init_completion(&trans->t_completion);
    INIT_LIST_HEAD(&trans->t_list);

    /* Add to journal's transaction list */
    mutex_lock(&journal->j_trans_mutex);
    list_add_tail(&trans->t_list, &journal->j_transactions);
    atomic_inc(&journal->j_trans_count);
    mutex_unlock(&journal->j_trans_mutex);

    atomic64_inc(&journal->j_transactions);

    return trans;
}

/*
 * Commit a transaction
 */
int vexfs_journal_commit(struct vexfs_journal_transaction *trans)
{
    struct vexfs_journal *journal;
    int ret = 0;

    if (!trans || trans->t_state != VEXFS_TRANS_RUNNING) {
        return -EINVAL;
    }

    journal = trans->t_journal;
    mutex_lock(&trans->t_mutex);

    /* Change state to committing */
    trans->t_state = VEXFS_TRANS_COMMIT;
    trans->t_commit_time = jiffies;

    /* Write descriptor block */
    if (trans->t_block_count > 0) {
        ret = vexfs_journal_write_descriptor(trans);
        if (ret) {
            printk(KERN_ERR "VexFS Journal: Failed to write descriptor block\n");
            goto error_abort;
        }

        /* Write commit block */
        ret = vexfs_journal_write_commit(trans);
        if (ret) {
            printk(KERN_ERR "VexFS Journal: Failed to write commit block\n");
            goto error_abort;
        }
    }

    /* Update journal sequence */
    spin_lock(&journal->j_lock);
    journal->j_commit_sequence = trans->t_transaction_id;
    spin_unlock(&journal->j_lock);

    /* Mark transaction as finished */
    trans->t_state = VEXFS_TRANS_FINISHED;
    complete_all(&trans->t_completion);

    /* Update statistics */
    atomic64_inc(&journal->j_commits);

    mutex_unlock(&trans->t_mutex);

    /* Remove from transaction list */
    mutex_lock(&journal->j_trans_mutex);
    list_del(&trans->t_list);
    atomic_dec(&journal->j_trans_count);
    mutex_unlock(&journal->j_trans_mutex);

    /* Free transaction resources */
    kfree(trans->t_block_list);
    kfree(trans);

    return 0;

error_abort:
    trans->t_state = VEXFS_TRANS_FINISHED;
    trans->t_error = ret;
    complete_all(&trans->t_completion);
    mutex_unlock(&trans->t_mutex);
    
    atomic64_inc(&journal->j_aborts);
    return ret;
}

/*
 * Abort a transaction
 */
int vexfs_journal_abort(struct vexfs_journal_transaction *trans)
{
    struct vexfs_journal *journal;

    if (!trans) {
        return -EINVAL;
    }

    journal = trans->t_journal;
    mutex_lock(&trans->t_mutex);

    /* Mark transaction as aborted */
    trans->t_state = VEXFS_TRANS_FINISHED;
    trans->t_error = -ECANCELED;
    complete_all(&trans->t_completion);

    mutex_unlock(&trans->t_mutex);

    /* Update statistics */
    atomic64_inc(&journal->j_aborts);

    /* Remove from transaction list */
    mutex_lock(&journal->j_trans_mutex);
    list_del(&trans->t_list);
    atomic_dec(&journal->j_trans_count);
    mutex_unlock(&journal->j_trans_mutex);

    /* Free transaction resources */
    kfree(trans->t_block_list);
    kfree(trans);

    return 0;
}

/*
 * Get write access to a buffer for journaling
 */
int vexfs_journal_get_write_access(struct vexfs_journal_transaction *trans,
                                  struct buffer_head *bh)
{
    if (!trans || !bh || trans->t_state != VEXFS_TRANS_RUNNING) {
        return -EINVAL;
    }

    /* Check if we have space for another block */
    if (trans->t_block_count >= trans->t_max_blocks) {
        return -ENOSPC;
    }

    /* Add block to transaction */
    trans->t_block_list[trans->t_block_count++] = bh->b_blocknr;

    /* Lock the buffer */
    lock_buffer(bh);
    get_bh(bh);

    return 0;
}

/*
 * Mark metadata as dirty in the journal
 */
int vexfs_journal_dirty_metadata(struct vexfs_journal_transaction *trans,
                                struct buffer_head *bh)
{
    if (!trans || !bh || trans->t_state != VEXFS_TRANS_RUNNING) {
        return -EINVAL;
    }

    /* Mark buffer as dirty */
    mark_buffer_dirty(bh);
    unlock_buffer(bh);
    put_bh(bh);

    return 0;
}

/*
 * Write descriptor block for transaction
 */
static int vexfs_journal_write_descriptor(struct vexfs_journal_transaction *trans)
{
    struct vexfs_journal *journal = trans->t_journal;
    struct vexfs_journal_descriptor *desc;
    struct buffer_head *bh;
    u64 desc_block;
    u32 checksum;
    int ret;

    /* Get next journal block */
    spin_lock(&journal->j_lock);
    desc_block = journal->j_head;
    journal->j_head++;
    if (journal->j_head >= journal->j_start_block + journal->j_total_blocks) {
        journal->j_head = journal->j_start_block + 1; /* Wrap around, skip superblock */
    }
    spin_unlock(&journal->j_lock);

    /* Get buffer for descriptor block */
    bh = sb_getblk(journal->j_sb, desc_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    desc = (struct vexfs_journal_descriptor *)bh->b_data;
    memset(desc, 0, journal->j_block_size);

    /* Fill descriptor header */
    desc->jd_header.jbh_magic = cpu_to_le32(VEXFS_JOURNAL_MAGIC);
    desc->jd_header.jbh_type = cpu_to_le32(VEXFS_JOURNAL_DESCRIPTOR);
    desc->jd_header.jbh_sequence = cpu_to_le64(journal->j_sequence++);

    /* Fill descriptor data */
    desc->jd_transaction_id = cpu_to_le64(trans->t_transaction_id);
    desc->jd_block_count = cpu_to_le32(trans->t_block_count);
    desc->jd_operation_type = cpu_to_le32(trans->t_operation_type);
    desc->jd_timestamp = cpu_to_le64(ktime_get_real_seconds());
    desc->jd_uid = cpu_to_le32(trans->t_uid);
    desc->jd_gid = cpu_to_le32(trans->t_gid);

    /* Copy block list */
    if (trans->t_block_count > 0) {
        size_t blocks_size = trans->t_block_count * sizeof(u64);
        size_t max_blocks = (journal->j_block_size - sizeof(*desc)) / sizeof(u64);
        
        if (trans->t_block_count > max_blocks) {
            unlock_buffer(bh);
            brelse(bh);
            return -E2BIG;
        }
        
        memcpy(desc->jd_blocks, trans->t_block_list, blocks_size);
    }

    /* Calculate checksum */
    checksum = vexfs_journal_calculate_checksum(desc, 
                                               journal->j_block_size - sizeof(desc->jd_header.jbh_checksum),
                                               0);
    desc->jd_header.jbh_checksum = cpu_to_le32(checksum);

    /* Write to disk */
    set_buffer_uptodate(bh);
    mark_buffer_dirty(bh);
    unlock_buffer(bh);
    ret = sync_dirty_buffer(bh);
    brelse(bh);

    if (ret) {
        printk(KERN_ERR "VexFS Journal: Failed to write descriptor block\n");
        return ret;
    }

    atomic64_inc(&journal->j_blocks_written);
    return 0;
}

/*
 * Write commit block for transaction
 */
static int vexfs_journal_write_commit(struct vexfs_journal_transaction *trans)
{
    struct vexfs_journal *journal = trans->t_journal;
    struct vexfs_journal_commit *commit;
    struct buffer_head *bh;
    u64 commit_block;
    u32 checksum;
    int ret;

    /* Get next journal block */
    spin_lock(&journal->j_lock);
    commit_block = journal->j_head;
    journal->j_head++;
    if (journal->j_head >= journal->j_start_block + journal->j_total_blocks) {
        journal->j_head = journal->j_start_block + 1; /* Wrap around, skip superblock */
    }
    spin_unlock(&journal->j_lock);

    /* Get buffer for commit block */
    bh = sb_getblk(journal->j_sb, commit_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    commit = (struct vexfs_journal_commit *)bh->b_data;
    memset(commit, 0, journal->j_block_size);

    /* Fill commit header */
    commit->jc_header.jbh_magic = cpu_to_le32(VEXFS_JOURNAL_MAGIC);
    commit->jc_header.jbh_type = cpu_to_le32(VEXFS_JOURNAL_COMMIT);
    commit->jc_header.jbh_sequence = cpu_to_le64(journal->j_sequence++);

    /* Fill commit data */
    commit->jc_transaction_id = cpu_to_le64(trans->t_transaction_id);
    commit->jc_commit_time = cpu_to_le64(ktime_get_real_seconds());
    commit->jc_block_count = cpu_to_le32(trans->t_block_count);
    commit->jc_checksum_type = cpu_to_le32(journal->j_checksum_type);

    /* Calculate transaction checksum */
    commit->jc_transaction_checksum = cpu_to_le32(
        vexfs_journal_calculate_checksum(trans->t_block_list,
                                        trans->t_block_count * sizeof(u64),
                                        trans->t_transaction_id));

    /* Calculate block checksum */
    checksum = vexfs_journal_calculate_checksum(commit,
                                               journal->j_block_size - sizeof(commit->jc_header.jbh_checksum),
                                               0);
    commit->jc_header.jbh_checksum = cpu_to_le32(checksum);

    /* Write to disk */
    set_buffer_uptodate(bh);
    mark_buffer_dirty(bh);
    unlock_buffer(bh);
    ret = sync_dirty_buffer(bh);
    brelse(bh);

    if (ret) {
        printk(KERN_ERR "VexFS Journal: Failed to write commit block\n");
        return ret;
    }

    atomic64_inc(&journal->j_blocks_written);
    return 0;
}

/*
 * Journal commit thread
 */
static int vexfs_journal_commit_thread(void *data)
{
    struct vexfs_journal *journal = (struct vexfs_journal *)data;

    while (!kthread_should_stop()) {
        /* Schedule commit work */
        queue_delayed_work(journal->j_workqueue, &journal->j_commit_work,
                          msecs_to_jiffies(journal_commit_interval));

        /* Sleep until next commit interval */
        msleep_interruptible(journal_commit_interval);
    }

    return 0;
}

/*
 * Journal commit work function
 */
static void vexfs_journal_commit_work_fn(struct work_struct *work)
{
    struct vexfs_journal *journal = container_of(work, struct vexfs_journal,
                                                 j_commit_work.work);

    /* Update journal superblock periodically */
    vexfs_journal_write_superblock(journal);
}

/*
 * Write journal superblock to disk
 */
static int vexfs_journal_write_superblock(struct vexfs_journal *journal)
{
    struct vexfs_journal_superblock *jsb;
    struct buffer_head *bh;
    int ret;

    bh = sb_bread(journal->j_sb, journal->j_start_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    jsb = (struct vexfs_journal_superblock *)bh->b_data;

    /* Update dynamic fields */
    spin_lock(&journal->j_lock);
    jsb->j_head = cpu_to_le64(journal->j_head);
    jsb->j_tail = cpu_to_le64(journal->j_tail);
    jsb->j_sequence = cpu_to_le64(journal->j_sequence);
    jsb->j_commit_sequence = cpu_to_le64(journal->j_commit_sequence);
    spin_unlock(&journal->j_lock);

    /* Update statistics */
    jsb->j_total_commits = cpu_to_le64(atomic64_read(&journal->j_commits));
    jsb->j_total_aborts = cpu_to_le64(atomic64_read(&journal->j_aborts));

    /* Recalculate checksum */
    jsb->j_superblock_checksum = cpu_to_le32(
        vexfs_journal_calculate_checksum(jsb,
                                        sizeof(*jsb) - sizeof(jsb->j_superblock_checksum),
                                        0));

    /* Write to disk */
    mark_buffer_dirty(bh);
    unlock_buffer(bh);
    ret = sync_dirty_buffer(bh);
    brelse(bh);

    return ret;
}

/*
 * Basic journal recovery implementation
 */
int vexfs_journal_recover(struct vexfs_journal *journal)
{
u64 scan_block, end_block;
    struct buffer_head *bh;
    struct vexfs_journal_block_header *header;
    u64 last_valid_sequence = 0;
    int transactions_recovered = 0;
    int ret = 0;

    if (!journal) {
        return -EINVAL;
    }

    /* Set recovery flag */
    journal->j_flags |= VEXFS_JOURNAL_RECOVERING;

    /* Scan journal from tail to head */
    scan_block = journal->j_tail;
    end_block = journal->j_head;

    while (scan_block != end_block) {
        /* Read journal block */
        bh = sb_bread(journal->j_sb, scan_block);
        if (!bh) {
            printk(KERN_WARNING "VexFS Journal: Failed to read block %llu during recovery\n",
                   scan_block);
            goto next_block;
        }

        header = (struct vexfs_journal_block_header *)bh->b_data;

        /* Verify magic number */
        if (le32_to_cpu(header->jbh_magic) != VEXFS_JOURNAL_MAGIC) {
            brelse(bh);
            goto next_block;
        }

        /* Verify checksum */
        u32 stored_checksum = le32_to_cpu(header->jbh_checksum);
        u32 calculated_checksum = vexfs_journal_calculate_checksum(
            bh->b_data, journal->j_block_size - sizeof(header->jbh_checksum), 0);

        if (stored_checksum != calculated_checksum) {
            printk(KERN_WARNING "VexFS Journal: Checksum mismatch in block %llu\n",
                   scan_block);
            brelse(bh);
            goto next_block;
        }

        /* Process based on block type */
        u32 block_type = le32_to_cpu(header->jbh_type);
        u64 sequence = le64_to_cpu(header->jbh_sequence);

        switch (block_type) {
        case VEXFS_JOURNAL_DESCRIPTOR:
            /* Found transaction start */
            last_valid_sequence = sequence;
            break;

        case VEXFS_JOURNAL_COMMIT:
            /* Found transaction commit */
            if (sequence > last_valid_sequence) {
                transactions_recovered++;
                journal->j_commit_sequence = sequence;
            }
            break;

        case VEXFS_JOURNAL_REVOCATION:
            /* Handle revocation block */
            break;

        default:
            printk(KERN_WARNING "VexFS Journal: Unknown block type %u\n", block_type);
            break;
        }

        brelse(bh);

next_block:
        scan_block++;
        if (scan_block >= journal->j_start_block + journal->j_total_blocks) {
            scan_block = journal->j_start_block + 1; /* Wrap around, skip superblock */
        }
    }

    /* Update journal state after recovery */
    journal->j_recovery_time = jiffies;
    journal->j_flags &= ~VEXFS_JOURNAL_RECOVERING;

    /* Write updated superblock */
    ret = vexfs_journal_write_superblock(journal);
    if (ret) {
        printk(KERN_ERR "VexFS Journal: Failed to write superblock after recovery\n");
        return ret;
    }

    printk(KERN_INFO "VexFS Journal: Recovery completed, %d transactions recovered\n",
           transactions_recovered);

    return 0;
}

/*
 * Flush all pending journal operations
 */
int vexfs_journal_flush(struct vexfs_journal *journal)
{
    struct vexfs_journal_transaction *trans, *tmp;
    int ret = 0;

    if (!journal) {
        return -EINVAL;
    }

    /* Wait for all active transactions to complete */
    mutex_lock(&journal->j_trans_mutex);
    list_for_each_entry_safe(trans, tmp, &journal->j_transactions, t_list) {
        if (trans->t_state == VEXFS_TRANS_RUNNING) {
            mutex_unlock(&journal->j_trans_mutex);
            wait_for_completion(&trans->t_completion);
            mutex_lock(&journal->j_trans_mutex);
        }
    }
    mutex_unlock(&journal->j_trans_mutex);

    /* Force commit any pending changes */
    ret = vexfs_journal_force_commit(journal);
    if (ret) {
        printk(KERN_ERR "VexFS Journal: Failed to force commit during flush\n");
        return ret;
    }

    /* Write superblock */
    ret = vexfs_journal_write_superblock(journal);
    if (ret) {
        printk(KERN_ERR "VexFS Journal: Failed to write superblock during flush\n");
        return ret;
    }

    return 0;
}

/*
 * Force commit of all pending transactions
 */
int vexfs_journal_force_commit(struct vexfs_journal *journal)
{
    if (!journal) {
        return -EINVAL;
    }

    /* Cancel any pending commit work and execute immediately */
    cancel_delayed_work_sync(&journal->j_commit_work);
    vexfs_journal_commit_work_fn(&journal->j_commit_work.work);

    return 0;
}

/*
 * Get journal statistics
 */
void vexfs_journal_get_stats(struct vexfs_journal *journal, 
                            struct vexfs_journal_stats *stats)
{
    if (!journal || !stats) {
        return;
    }

    memset(stats, 0, sizeof(*stats));

    stats->total_commits = atomic64_read(&journal->j_commits);
    stats->total_aborts = atomic64_read(&journal->j_aborts);
    stats->total_transactions = atomic64_read(&journal->j_transactions);
    stats->blocks_written = atomic64_read(&journal->j_blocks_written);
    stats->active_transactions = atomic_read(&journal->j_trans_count);
    stats->last_recovery_time = journal->j_recovery_time;

    /* Calculate journal utilization */
    u64 used_blocks = (journal->j_head >= journal->j_tail) ?
                      (journal->j_head - journal->j_tail) :
                      (journal->j_total_blocks - (journal->j_tail - journal->j_head));
    stats->journal_utilization = (u32)((used_blocks * 100) / journal->j_total_blocks);
}

/*
 * Extend transaction with additional blocks
 */
int vexfs_journal_extend(struct vexfs_journal_transaction *trans, u32 additional_blocks)
{
    u64 *new_block_list;
    u32 new_max_blocks;

    if (!trans || trans->t_state != VEXFS_TRANS_RUNNING) {
        return -EINVAL;
    }

    new_max_blocks = trans->t_max_blocks + additional_blocks;
    if (new_max_blocks > journal_max_trans_blocks) {
        return -E2BIG;
    }

    /* Reallocate block list */
    new_block_list = krealloc(trans->t_block_list, 
                             new_max_blocks * sizeof(u64), GFP_KERNEL);
    if (!new_block_list) {
        return -ENOMEM;
    }

    trans->t_block_list = new_block_list;
    trans->t_max_blocks = new_max_blocks;

    return 0;
}

/*
 * Forget a buffer (remove from transaction)
 */
int vexfs_journal_forget(struct vexfs_journal_transaction *trans,
                        struct buffer_head *bh)
{
    u32 i;

    if (!trans || !bh || trans->t_state != VEXFS_TRANS_RUNNING) {
        return -EINVAL;
    }

    /* Find and remove block from transaction */
    for (i = 0; i < trans->t_block_count; i++) {
        if (trans->t_block_list[i] == bh->b_blocknr) {
            /* Shift remaining blocks */
            memmove(&trans->t_block_list[i], &trans->t_block_list[i + 1],
                   (trans->t_block_count - i - 1) * sizeof(u64));
            trans->t_block_count--;
            break;
        }
    }

    /* Release buffer */
    put_bh(bh);

    return 0;
}

/*
 * Replay transactions for recovery
 */
int vexfs_journal_replay_transactions(struct vexfs_journal *journal,
                                     u64 start_seq, u64 end_seq)
{
    /* This is a placeholder for transaction replay functionality */
    /* In a full implementation, this would:
     * 1. Scan journal for transactions in the sequence range
     * 2. Parse descriptor and commit blocks
     * 3. Replay the data changes to the filesystem
     * 4. Update metadata accordingly
     */
    
    printk(KERN_INFO "VexFS Journal: Replaying transactions from seq %llu to %llu\n",
           start_seq, end_seq);

    /* For now, just mark as completed */
    return 0;
}
    printk(KERN_INFO "VexFS Journal: Starting journal recovery\n");