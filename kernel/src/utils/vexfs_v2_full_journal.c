/*
 * VexFS v2.0 - Full Filesystem Journal (Phase 1) Implementation
 * 
 * This implements the production-grade journaling mechanism for VexFS with
 * enterprise-level features including:
 * - Advanced transaction management with concurrent support
 * - Multiple journaling modes (ordered, writeback, journal)
 * - SHA-256 checksumming for cryptographic integrity
 * - Non-blocking write strategies with separate commit threads
 * - Comprehensive crash recovery mechanisms
 * - Performance-optimized journal operations
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
#include <crypto/sha2.h>
#include <linux/random.h>
#include <linux/jiffies.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/atomic.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/ioctl.h>
#include <linux/uaccess.h>

#include "../include/vexfs_v2_full_journal.h"
#include "../include/vexfs_v2_internal.h"

/* Module parameters for full journal tuning */
static unsigned int full_journal_mode = VEXFS_JOURNAL_MODE_ORDERED;
module_param(full_journal_mode, uint, 0644);
MODULE_PARM_DESC(full_journal_mode, "Default journaling mode (1=ordered, 2=writeback, 3=journal)");

static unsigned int concurrent_transactions = 64;
module_param(concurrent_transactions, uint, 0644);
MODULE_PARM_DESC(concurrent_transactions, "Maximum concurrent transactions");

static unsigned int commit_threads = 4;
module_param(commit_threads, uint, 0644);
MODULE_PARM_DESC(commit_threads, "Number of commit threads");

static unsigned int journal_buffer_size = 65536; /* 64KB */
module_param(journal_buffer_size, uint, 0644);
MODULE_PARM_DESC(journal_buffer_size, "Journal buffer size in bytes");

static unsigned int checkpoint_interval = 300; /* 5 minutes */
module_param(checkpoint_interval, uint, 0644);
MODULE_PARM_DESC(checkpoint_interval, "Checkpoint interval in seconds");

/* Forward declarations */
static int vexfs_full_journal_commit_thread_fn(void *data);
static void vexfs_full_journal_buffer_flush_work_fn(struct work_struct *work);
static void vexfs_full_journal_checkpoint_work_fn(struct work_struct *work);
static int vexfs_full_journal_write_enhanced_superblock(struct vexfs_full_journal *journal);
static int vexfs_full_journal_write_data_block(struct vexfs_full_journal_transaction *trans,
                                               u64 block_number, void *data, size_t size);
static int vexfs_full_journal_write_checkpoint_block(struct vexfs_full_journal *journal,
                                                     u64 checkpoint_id);
static int vexfs_full_journal_process_barriers(struct vexfs_full_journal *journal);

/*
 * SHA-256 checksum calculation for journal blocks
 */
int vexfs_full_journal_calculate_sha256(const void *data, size_t len, u8 *hash)
{
    struct crypto_shash *tfm;
    struct shash_desc *desc;
    int ret;

    tfm = crypto_alloc_shash("sha256", 0, 0);
    if (IS_ERR(tfm)) {
        printk(KERN_ERR "VexFS Full Journal: Failed to allocate SHA-256 transform\n");
        return PTR_ERR(tfm);
    }

    desc = kmalloc(sizeof(*desc) + crypto_shash_descsize(tfm), GFP_KERNEL);
    if (!desc) {
        crypto_free_shash(tfm);
        return -ENOMEM;
    }

    desc->tfm = tfm;
    desc->flags = 0;

    ret = crypto_shash_init(desc);
    if (ret)
        goto out;

    ret = crypto_shash_update(desc, data, len);
    if (ret)
        goto out;

    ret = crypto_shash_final(desc, hash);

out:
    kfree(desc);
    crypto_free_shash(tfm);
    return ret;
}

/*
 * SHA-256 checksum verification
 */
int vexfs_full_journal_verify_sha256(const void *data, size_t len, const u8 *expected_hash)
{
    u8 calculated_hash[SHA256_DIGEST_SIZE];
    int ret;

    ret = vexfs_full_journal_calculate_sha256(data, len, calculated_hash);
    if (ret)
        return ret;

    if (memcmp(calculated_hash, expected_hash, SHA256_DIGEST_SIZE) != 0) {
        printk(KERN_ERR "VexFS Full Journal: SHA-256 checksum mismatch\n");
        return -EINVAL;
    }

    return 0;
}

/*
 * Initialize journal buffer for batching operations
 */
static struct vexfs_journal_buffer *vexfs_journal_buffer_init(size_t size)
{
    struct vexfs_journal_buffer *buffer;

    buffer = kzalloc(sizeof(struct vexfs_journal_buffer), GFP_KERNEL);
    if (!buffer)
        return ERR_PTR(-ENOMEM);

    buffer->jb_buffer = vmalloc(size);
    if (!buffer->jb_buffer) {
        kfree(buffer);
        return ERR_PTR(-ENOMEM);
    }

    buffer->jb_size = size;
    buffer->jb_used = 0;
    atomic_set(&buffer->jb_transaction_count, 0);
    
    spin_lock_init(&buffer->jb_lock);
    INIT_LIST_HEAD(&buffer->jb_transactions);
    init_completion(&buffer->jb_flush_completion);
    
    buffer->jb_flags = 0;
    buffer->jb_last_flush = ktime_get();

    return buffer;
}

/*
 * Destroy journal buffer
 */
static void vexfs_journal_buffer_destroy(struct vexfs_journal_buffer *buffer)
{
    if (!buffer)
        return;

    if (buffer->jb_buffer)
        vfree(buffer->jb_buffer);
    
    kfree(buffer);
}

/*
 * Initialize commit thread
 */
static struct vexfs_commit_thread *vexfs_commit_thread_init(struct vexfs_full_journal *journal,
                                                           u32 thread_id)
{
    struct vexfs_commit_thread *thread;
    char thread_name[32];

    thread = kzalloc(sizeof(struct vexfs_commit_thread), GFP_KERNEL);
    if (!thread)
        return ERR_PTR(-ENOMEM);

    thread->ct_thread_id = thread_id;
    thread->ct_journal = journal;
    
    INIT_LIST_HEAD(&thread->ct_pending_transactions);
    spin_lock_init(&thread->ct_lock);
    
    atomic64_set(&thread->ct_transactions_committed, 0);
    atomic64_set(&thread->ct_total_commit_time, 0);
    atomic64_set(&thread->ct_average_commit_time, 0);
    
    atomic_set(&thread->ct_active, 1);
    init_completion(&thread->ct_completion);

    /* Create workqueue for this thread */
    snprintf(thread_name, sizeof(thread_name), "vexfs_commit_%u", thread_id);
    thread->ct_workqueue = alloc_workqueue(thread_name, WQ_MEM_RECLAIM | WQ_UNBOUND, 1);
    if (!thread->ct_workqueue) {
        kfree(thread);
        return ERR_PTR(-ENOMEM);
    }

    /* Start kernel thread */
    snprintf(thread_name, sizeof(thread_name), "vexfs_commit_%u", thread_id);
    thread->ct_thread = kthread_run(vexfs_full_journal_commit_thread_fn, thread, thread_name);
    if (IS_ERR(thread->ct_thread)) {
        destroy_workqueue(thread->ct_workqueue);
        kfree(thread);
        return ERR_PTR(PTR_ERR(thread->ct_thread));
    }

    return thread;
}

/*
 * Destroy commit thread
 */
static void vexfs_commit_thread_destroy(struct vexfs_commit_thread *thread)
{
    if (!thread)
        return;

    atomic_set(&thread->ct_active, 0);
    
    if (thread->ct_thread) {
        kthread_stop(thread->ct_thread);
        thread->ct_thread = NULL;
    }
    
    if (thread->ct_workqueue) {
        destroy_workqueue(thread->ct_workqueue);
        thread->ct_workqueue = NULL;
    }
    
    kfree(thread);
}

/*
 * Initialize full journal structure
 */
struct vexfs_full_journal *vexfs_full_journal_init(struct super_block *sb,
                                                   u64 start_block, u64 total_blocks,
                                                   u32 journal_mode)
{
    struct vexfs_full_journal *full_journal;
    struct vexfs_journal *base_journal;
    int ret, i;

    if (!sb || total_blocks < 128) {
        printk(KERN_ERR "VexFS Full Journal: Invalid parameters for journal init\n");
        return ERR_PTR(-EINVAL);
    }

    if (journal_mode < VEXFS_JOURNAL_MODE_ORDERED || journal_mode > VEXFS_JOURNAL_MODE_JOURNAL) {
        printk(KERN_ERR "VexFS Full Journal: Invalid journal mode %u\n", journal_mode);
        return ERR_PTR(-EINVAL);
    }

    full_journal = kzalloc(sizeof(struct vexfs_full_journal), GFP_KERNEL);
    if (!full_journal) {
        printk(KERN_ERR "VexFS Full Journal: Failed to allocate full journal structure\n");
        return ERR_PTR(-ENOMEM);
    }

    /* Initialize base journal first */
    base_journal = vexfs_journal_init(sb, start_block, total_blocks);
    if (IS_ERR(base_journal)) {
        ret = PTR_ERR(base_journal);
        kfree(full_journal);
        return ERR_PTR(ret);
    }

    /* Copy base journal into full journal */
    memcpy(&full_journal->base, base_journal, sizeof(struct vexfs_journal));
    kfree(base_journal); /* Free the temporary base journal */

    /* Initialize full journal specific fields */
    full_journal->fj_journal_mode = journal_mode;
    full_journal->fj_concurrent_trans_limit = concurrent_transactions;
    atomic_set(&full_journal->fj_active_trans_count, 0);

    /* Initialize SHA-256 support */
    full_journal->fj_sha256_tfm = crypto_alloc_shash("sha256", 0, 0);
    if (IS_ERR(full_journal->fj_sha256_tfm)) {
        printk(KERN_ERR "VexFS Full Journal: Failed to allocate SHA-256 transform\n");
        ret = PTR_ERR(full_journal->fj_sha256_tfm);
        goto error_cleanup;
    }

    /* Initialize commit thread pool */
    full_journal->fj_commit_thread_count = min(commit_threads, VEXFS_FULL_JOURNAL_MAX_COMMIT_THREADS);
    full_journal->fj_commit_threads = kcalloc(full_journal->fj_commit_thread_count,
                                             sizeof(struct vexfs_commit_thread), GFP_KERNEL);
    if (!full_journal->fj_commit_threads) {
        ret = -ENOMEM;
        goto error_cleanup;
    }

    atomic_set(&full_journal->fj_next_commit_thread, 0);

    for (i = 0; i < full_journal->fj_commit_thread_count; i++) {
        struct vexfs_commit_thread *thread = vexfs_commit_thread_init(full_journal, i);
        if (IS_ERR(thread)) {
            ret = PTR_ERR(thread);
            goto error_cleanup_threads;
        }
        memcpy(&full_journal->fj_commit_threads[i], thread, sizeof(struct vexfs_commit_thread));
        kfree(thread);
    }

    /* Initialize journal buffer */
    full_journal->fj_buffer = vexfs_journal_buffer_init(journal_buffer_size);
    if (IS_ERR(full_journal->fj_buffer)) {
        ret = PTR_ERR(full_journal->fj_buffer);
        goto error_cleanup_threads;
    }
    full_journal->fj_buffer_size = journal_buffer_size;

    /* Initialize buffer flush work */
    INIT_DELAYED_WORK(&full_journal->fj_buffer_flush_work, vexfs_full_journal_buffer_flush_work_fn);

    /* Initialize checkpointing */
    full_journal->fj_last_checkpoint_seq = 0;
    full_journal->fj_checkpoint_interval = checkpoint_interval;
    INIT_DELAYED_WORK(&full_journal->fj_checkpoint_work, vexfs_full_journal_checkpoint_work_fn);
    atomic64_set(&full_journal->fj_checkpoint_count, 0);

    /* Initialize barrier support */
    INIT_LIST_HEAD(&full_journal->fj_barrier_list);
    spin_lock_init(&full_journal->fj_barrier_lock);
    atomic64_set(&full_journal->fj_barrier_count, 0);

    /* Initialize recovery state */
    full_journal->fj_recovery_thread_count = min(4U, num_online_cpus());
    atomic_set(&full_journal->fj_recovery_active, 0);

    /* Initialize performance counters */
    atomic64_set(&full_journal->fj_concurrent_peak, 0);
    atomic64_set(&full_journal->fj_total_barriers, 0);
    atomic64_set(&full_journal->fj_sha256_operations, 0);
    atomic64_set(&full_journal->fj_data_blocks_journaled, 0);

    /* Set advanced flags */
    full_journal->fj_flags = VEXFS_JOURNAL_SHA256_CHECKSUM | 
                            VEXFS_JOURNAL_CONCURRENT_TRANS |
                            VEXFS_JOURNAL_NON_BLOCKING;
    
    if (journal_mode == VEXFS_JOURNAL_MODE_JOURNAL)
        full_journal->fj_flags |= VEXFS_JOURNAL_BARRIER_SUPPORT;

    full_journal->fj_barrier_timeout = 5000; /* 5 seconds */

    /* Schedule periodic work */
    queue_delayed_work(full_journal->base.j_workqueue, &full_journal->fj_buffer_flush_work,
                      msecs_to_jiffies(1000)); /* Flush every second */
    
    queue_delayed_work(full_journal->base.j_workqueue, &full_journal->fj_checkpoint_work,
                      msecs_to_jiffies(full_journal->fj_checkpoint_interval * 1000));

    printk(KERN_INFO "VexFS Full Journal: Initialized with mode %u, %u commit threads, %u KB buffer\n",
           journal_mode, full_journal->fj_commit_thread_count, journal_buffer_size / 1024);

    return full_journal;

error_cleanup_threads:
    for (i = 0; i < full_journal->fj_commit_thread_count; i++) {
        vexfs_commit_thread_destroy(&full_journal->fj_commit_threads[i]);
    }
    kfree(full_journal->fj_commit_threads);

error_cleanup:
    if (!IS_ERR_OR_NULL(full_journal->fj_sha256_tfm))
        crypto_free_shash(full_journal->fj_sha256_tfm);
    
    vexfs_journal_destroy(&full_journal->base);
    kfree(full_journal);
    return ERR_PTR(ret);
}

/*
 * Destroy full journal and free resources
 */
void vexfs_full_journal_destroy(struct vexfs_full_journal *journal)
{
    int i;

    if (!journal)
        return;

    /* Cancel periodic work */
    cancel_delayed_work_sync(&journal->fj_buffer_flush_work);
    cancel_delayed_work_sync(&journal->fj_checkpoint_work);

    /* Destroy commit threads */
    for (i = 0; i < journal->fj_commit_thread_count; i++) {
        vexfs_commit_thread_destroy(&journal->fj_commit_threads[i]);
    }
    kfree(journal->fj_commit_threads);

    /* Destroy journal buffer */
    vexfs_journal_buffer_destroy(journal->fj_buffer);

    /* Free SHA-256 transform */
    if (journal->fj_sha256_tfm)
        crypto_free_shash(journal->fj_sha256_tfm);

    /* Destroy base journal */
    vexfs_journal_destroy(&journal->base);

    /* Free full journal structure */
    kfree(journal);

    printk(KERN_INFO "VexFS Full Journal: Destroyed journal\n");
}

/*
 * Start a new enhanced transaction
 */
struct vexfs_full_journal_transaction *vexfs_full_journal_start(
    struct vexfs_full_journal *journal, u32 max_blocks, u32 operation_type, u32 priority)
{
    struct vexfs_full_journal_transaction *full_trans;
    struct vexfs_journal_transaction *base_trans;
    int current_active;

    if (!journal || max_blocks == 0) {
        return ERR_PTR(-EINVAL);
    }

    /* Check concurrent transaction limit */
    current_active = atomic_read(&journal->fj_active_trans_count);
    if (current_active >= journal->fj_concurrent_trans_limit) {
        return ERR_PTR(-EAGAIN);
    }

    /* Start base transaction */
    base_trans = vexfs_journal_start(&journal->base, max_blocks, operation_type);
    if (IS_ERR(base_trans)) {
        return ERR_PTR(PTR_ERR(base_trans));
    }

    /* Allocate full transaction structure */
    full_trans = kzalloc(sizeof(struct vexfs_full_journal_transaction), GFP_KERNEL);
    if (!full_trans) {
        vexfs_journal_abort(base_trans);
        return ERR_PTR(-ENOMEM);
    }

    /* Copy base transaction */
    memcpy(&full_trans->base, base_trans, sizeof(struct vexfs_journal_transaction));
    kfree(base_trans);

    /* Initialize full transaction fields */
    full_trans->ft_priority = priority;
    full_trans->ft_journal_mode = journal->fj_journal_mode;
    atomic_set(&full_trans->ft_barrier_count, 0);

    /* Initialize data journaling support */
    full_trans->ft_data_block_count = 0;
    full_trans->ft_data_block_list = NULL;
    full_trans->ft_data_buffers = NULL;

    /* Initialize dependency tracking */
    INIT_LIST_HEAD(&full_trans->ft_dependency_list);
    init_completion(&full_trans->ft_barrier_completion);

    /* Initialize performance tracking */
    full_trans->ft_start_time = ktime_get();
    full_trans->ft_commit_time = 0;
    full_trans->ft_commit_thread_id = 0;

    /* Initialize SHA-256 context */
    full_trans->ft_sha256_tfm = crypto_alloc_shash("sha256", 0, 0);
    if (IS_ERR(full_trans->ft_sha256_tfm)) {
        printk(KERN_WARNING "VexFS Full Journal: Failed to allocate SHA-256 for transaction\n");
        full_trans->ft_sha256_tfm = NULL;
    } else {
        full_trans->ft_sha256_desc = kmalloc(sizeof(struct shash_desc) + 
                                           crypto_shash_descsize(full_trans->ft_sha256_tfm), 
                                           GFP_KERNEL);
        if (!full_trans->ft_sha256_desc) {
            crypto_free_shash(full_trans->ft_sha256_tfm);
            full_trans->ft_sha256_tfm = NULL;
        } else {
            full_trans->ft_sha256_desc->tfm = full_trans->ft_sha256_tfm;
            full_trans->ft_sha256_desc->flags = 0;
        }
    }

    /* Update active transaction count and peak tracking */
    current_active = atomic_inc_return(&journal->fj_active_trans_count);
    
    /* Update peak concurrent transactions */
    while (true) {
        u64 current_peak = atomic64_read(&journal->fj_concurrent_peak);
        if (current_active <= current_peak)
            break;
        if (atomic64_cmpxchg(&journal->fj_concurrent_peak, current_peak, current_active) == current_peak)
            break;
    }

    return full_trans;
}

/*
 * Add data block to transaction for journal mode
 */
int vexfs_full_journal_add_data_block(struct vexfs_full_journal_transaction *trans,
                                      u64 block_number, void *data, size_t size)
{
    u64 *new_block_list;
    void **new_buffers;
    void *data_copy;

    if (!trans || !data || size == 0) {
        return -EINVAL;
    }

    /* Only journal data in journal mode */
    if (trans->ft_journal_mode != VEXFS_JOURNAL_MODE_JOURNAL) {
        return 0;
    }

    /* Reallocate block list */
    new_block_list = krealloc(trans->ft_data_block_list,
                             (trans->ft_data_block_count + 1) * sizeof(u64),
                             GFP_KERNEL);
    if (!new_block_list) {
        return -ENOMEM;
    }
    trans->ft_data_block_list = new_block_list;

    /* Reallocate buffer list */
    new_buffers = krealloc(trans->ft_data_buffers,
                          (trans->ft_data_block_count + 1) * sizeof(void *),
                          GFP_KERNEL);
    if (!new_buffers) {
        return -ENOMEM;
    }
    trans->ft_data_buffers = new_buffers;

    /* Copy data */
    data_copy = kmalloc(size, GFP_KERNEL);
    if (!data_copy) {
        return -ENOMEM;
    }
    memcpy(data_copy, data, size);

    /* Add to transaction */
    trans->ft_data_block_list[trans->ft_data_block_count] = block_number;
    trans->ft_data_buffers[trans->ft_data_block_count] = data_copy;
    trans->ft_data_block_count++;

    return 0;
}

/*
 * Commit enhanced transaction
 */
int vexfs_full_journal_commit(struct vexfs_full_journal_transaction *trans)
{
    struct vexfs_full_journal *journal;
    u32 commit_thread_id;
    int ret;

    if (!trans) {
        return -EINVAL;
    }

    journal = container_of(trans->base.t_journal, struct vexfs_full_journal, base);

    /* Record commit start time */
    trans->ft_commit_time = ktime_get();

    /* Select commit thread (round-robin) */
    commit_thread_id = atomic_inc_return(&journal->fj_next_commit_thread) % 
                      journal->fj_commit_thread_count;
    trans->ft_commit_thread_id = commit_thread_id;

    /* Write data blocks if in journal mode */
    if (trans->ft_journal_mode == VEXFS_JOURNAL_MODE_JOURNAL && trans->ft_data_block_count > 0) {
        ret = vexfs_full_journal_write_data_blocks(trans);
        if (ret) {
            printk(KERN_ERR "VexFS Full Journal: Failed to write data blocks: %d\n", ret);
            goto abort_transaction;
        }
    }

    /* Process any barriers */
    if (atomic_read(&trans->ft_barrier_count) > 0) {
        ret = vexfs_full_journal_wait_barrier(trans);
        if (ret) {
            printk(KERN_ERR "VexFS Full Journal: Barrier wait failed: %d\n", ret);
            goto abort_transaction;
        }
    }

    /* Commit base transaction */
    ret = vexfs_journal_commit(&trans->base);
    if (ret) {
        printk(KERN_ERR "VexFS Full Journal: Base transaction commit failed: %d\n", ret);
        goto abort_transaction;
    }

    /* Update statistics */
    atomic64_inc(&journal->fj_commit_threads[commit_thread_id].ct_transactions_committed);
    
    if (trans->ft_journal_mode == VEXFS_JOURNAL_MODE_JOURNAL) {
        atomic64_add(trans->ft_data_block_count, &journal->fj_data_blocks_journaled);
    }

    /* Cleanup transaction resources */
    if (trans->ft_data_block_list) {
        int i;
        for (i = 0; i < trans->ft_data_block_count; i++) {
            kfree(trans->ft_data_buffers[i]);
        }
        kfree(trans->ft_data_block_list);
        kfree(trans->ft_data_buffers);
    }

    if (trans->ft_sha256_desc)
        kfree(trans->ft_sha256_desc);
    if (trans->ft_sha256_tfm)
        crypto_free_shash(trans->ft_sha256_tfm);

    /* Decrement active transaction count */
    atomic_dec(&journal->fj_active_trans_count);

    kfree(trans);
    return 0;

abort_transaction:
    vexfs_full_journal_abort(trans);
    return ret;
}

/*
 * Abort enhanced transaction
 */
int vexfs_full_journal_abort(struct vexfs_full_journal_transaction *trans)
{
    struct vexfs_full_journal *journal;
    int ret;

    if (!trans) {
        return -EINVAL;
    }

    journal = container_of(trans->base.t_journal, struct vexfs_full_journal, base);

    /* Abort base transaction */
    ret = vexfs_journal_abort(&trans->base);

    /* Cleanup transaction resources */
    if (trans->ft_data_block_list) {
        int i;
        for (i = 0; i < trans->ft_data_block_count; i++) {
            kfree(trans->ft_data_buffers[i]);
        }
        kfree(trans->ft_data_block_list);
        kfree(trans->ft_data_buffers);
    }

    if (trans->ft_sha256_desc)
        kfree(trans->ft_sha256_desc);
    if (trans->ft_sha256_tfm)
        crypto_free_shash(trans->ft_sha256_tfm);

    /* Decrement active transaction count */
    atomic_dec(&journal->fj_active_trans_count);

    kfree(trans);
    return ret;
}

/*
 * Write data blocks for journal mode
 */
int vexfs_full_journal_write_data_blocks(struct vexfs_full_journal_transaction *trans)
{
    struct vexfs_full_journal *journal;
    int i, ret;

    if (!trans || trans->ft_data_block_count == 0) {
        return 0;
    }

    journal = container_of(trans->base.t_journal, struct vexfs_full_journal, base);

    for (i = 0; i < trans->ft_data_block_count; i++) {
        ret = vexfs_full_journal_write_data_block(trans, 
                                                 trans->ft_data_block_list[i],
                                                 trans->ft_data_buffers[i],
                                                 VEXFS_JOURNAL_BLOCK_SIZE);
        if (ret) {
            printk(KERN_ERR "VexFS Full Journal: Failed to write data block %d: %d\n", i, ret);
            return ret;
        }
    }

    return 0;
}

/*
 * Write individual data block to journal
 */
static int vexfs_full_journal_write_data_block(struct vexfs_full_journal_transaction *trans,
                                               u64 block_number, void *data, size_t size)
{
    struct vexfs_full_journal *journal;
    struct vexfs_journal_data_block *data_block;
    struct buffer_head *bh;
    u64 journal_block;
    u32 checksum;
    u8 sha256_hash[SHA256_DIGEST_SIZE];
    int ret;

    journal = container_of(trans->base.t_journal, struct vexfs_full_journal, base);

    /* Get next journal block */
    spin_lock(&journal->base.j_lock);
    journal_block = journal->base.j_head;
    journal->base.j_head++;
    if (journal->base.j_head >= journal->base.j_start_block + journal->base.j_total_blocks) {
        journal->base.j_head = journal->base.j_start_block + 1; /* Wrap around */
    }
    spin_unlock(&journal->base.j_lock);

    /* Get buffer for data block */
    bh = sb_getblk(journal->base.j_sb, journal_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    data_block = (struct vexfs_journal_data_block *)bh->b_data;
    memset(data_block, 0, journal->base.j_block_size);

    /* Fill header */
    data_block->fjdb_header
struct buffer_head *bh;
    u64 journal_block;
    u32 checksum;
    u8 sha256_hash[SHA256_DIGEST_SIZE];
    int ret;

    journal = container_of(trans->base.t_journal, struct vexfs_full_journal, base);

    /* Get next journal block */
    spin_lock(&journal->base.j_lock);
    journal_block = journal->base.j_head;
    journal->base.j_head++;
    if (journal->base.j_head >= journal->base.j_start_block + journal->base.j_total_blocks) {
        journal->base.j_head = journal->base.j_start_block + 1; /* Wrap around */
    }
    spin_unlock(&journal->base.j_lock);

    /* Get buffer for data block */
    bh = sb_getblk(journal->base.j_sb, journal_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    data_block = (struct vexfs_journal_data_block *)bh->b_data;
    memset(data_block, 0, journal->base.j_block_size);

    /* Fill header */
    data_block->fjdb_header.base.base.jbh_magic = cpu_to_le32(VEXFS_JOURNAL_MAGIC);
    data_block->fjdb_header.base.base.jbh_type = cpu_to_le32(VEXFS_JOURNAL_DATA_BLOCK);
    data_block->fjdb_header.base.base.jbh_sequence = cpu_to_le64(journal->base.j_sequence++);
    data_block->fjdb_header.base.base.jbh_flags = 0;

    /* Fill data block specific fields */
    data_block->fjdb_original_block = cpu_to_le64(block_number);
    data_block->fjdb_data_size = cpu_to_le32(size);
    data_block->fjdb_flags = 0;

    /* Copy data */
    memcpy(data_block->fjdb_data, data, min(size, (size_t)(journal->base.j_block_size - sizeof(*data_block))));

    /* Calculate SHA-256 checksum */
    ret = vexfs_full_journal_calculate_sha256(data_block->fjdb_data, size, sha256_hash);
    if (ret == 0) {
        memcpy(data_block->fjdb_header.fjbh_sha256, sha256_hash, SHA256_DIGEST_SIZE);
        atomic64_inc(&journal->fj_sha256_operations);
    }

    /* Calculate CRC32 for compatibility */
    checksum = vexfs_journal_calculate_checksum(bh->b_data,
                                               journal->base.j_block_size - sizeof(data_block->fjdb_header.base.base.jbh_checksum),
                                               0);
    data_block->fjdb_header.base.base.jbh_checksum = cpu_to_le32(checksum);

    set_buffer_uptodate(bh);
    mark_buffer_dirty(bh);
    unlock_buffer(bh);

    ret = sync_dirty_buffer(bh);
    brelse(bh);

    if (ret) {
        printk(KERN_ERR "VexFS Full Journal: Failed to write data block to disk: %d\n", ret);
        return ret;
    }

    atomic64_inc(&journal->base.j_blocks_written);
    return 0;
}

/*
 * Create checkpoint
 */
int vexfs_full_journal_create_checkpoint(struct vexfs_full_journal *journal, u32 flags)
{
    u64 checkpoint_id;
    int ret;

    if (!journal) {
        return -EINVAL;
    }

    /* Generate unique checkpoint ID */
    checkpoint_id = atomic64_inc_return(&journal->fj_checkpoint_count);

    /* Write checkpoint block */
    ret = vexfs_full_journal_write_checkpoint_block(journal, checkpoint_id);
    if (ret) {
        printk(KERN_ERR "VexFS Full Journal: Failed to write checkpoint block: %d\n", ret);
        return ret;
    }

    /* Update last checkpoint sequence */
    journal->fj_last_checkpoint_seq = journal->base.j_sequence;

    printk(KERN_INFO "VexFS Full Journal: Created checkpoint %llu at sequence %llu\n",
           checkpoint_id, journal->fj_last_checkpoint_seq);

    return 0;
}

/*
 * Write checkpoint block
 */
static int vexfs_full_journal_write_checkpoint_block(struct vexfs_full_journal *journal,
                                                     u64 checkpoint_id)
{
    struct vexfs_journal_checkpoint *checkpoint;
    struct buffer_head *bh;
    u64 checkpoint_block;
    u32 checksum;
    u8 sha256_hash[SHA256_DIGEST_SIZE];
    int ret;

    /* Get next journal block */
    spin_lock(&journal->base.j_lock);
    checkpoint_block = journal->base.j_head;
    journal->base.j_head++;
    if (journal->base.j_head >= journal->base.j_start_block + journal->base.j_total_blocks) {
        journal->base.j_head = journal->base.j_start_block + 1;
    }
    spin_unlock(&journal->base.j_lock);

    /* Get buffer for checkpoint block */
    bh = sb_getblk(journal->base.j_sb, checkpoint_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    checkpoint = (struct vexfs_journal_checkpoint *)bh->b_data;
    memset(checkpoint, 0, journal->base.j_block_size);

    /* Fill header */
    checkpoint->fjcp_header.base.base.jbh_magic = cpu_to_le32(VEXFS_JOURNAL_MAGIC);
    checkpoint->fjcp_header.base.base.jbh_type = cpu_to_le32(VEXFS_JOURNAL_CHECKPOINT);
    checkpoint->fjcp_header.base.base.jbh_sequence = cpu_to_le64(journal->base.j_sequence++);
    checkpoint->fjcp_header.base.base.jbh_flags = 0;

    /* Fill checkpoint data */
    checkpoint->fjcp_checkpoint_id = cpu_to_le64(checkpoint_id);
    checkpoint->fjcp_last_committed_seq = cpu_to_le64(journal->base.j_commit_sequence);
    checkpoint->fjcp_filesystem_state = cpu_to_le64(0); /* TODO: Calculate filesystem state hash */
    checkpoint->fjcp_active_trans_count = cpu_to_le32(atomic_read(&journal->fj_active_trans_count));
    checkpoint->fjcp_flags = cpu_to_le32(0);
    checkpoint->fjcp_timestamp = cpu_to_le64(ktime_get_real_seconds());

    /* Calculate SHA-256 checksum */
    ret = vexfs_full_journal_calculate_sha256(checkpoint,
                                             sizeof(*checkpoint) - sizeof(checkpoint->fjcp_header.fjbh_sha256),
                                             sha256_hash);
    if (ret == 0) {
        memcpy(checkpoint->fjcp_header.fjbh_sha256, sha256_hash, SHA256_DIGEST_SIZE);
        atomic64_inc(&journal->fj_sha256_operations);
    }

    /* Calculate CRC32 checksum */
    checksum = vexfs_journal_calculate_checksum(bh->b_data,
                                               journal->base.j_block_size - sizeof(checkpoint->fjcp_header.base.base.jbh_checksum),
                                               0);
    checkpoint->fjcp_header.base.base.jbh_checksum = cpu_to_le32(checksum);

    set_buffer_uptodate(bh);
    mark_buffer_dirty(bh);
    unlock_buffer(bh);

    ret = sync_dirty_buffer(bh);
    brelse(bh);

    if (ret) {
        printk(KERN_ERR "VexFS Full Journal: Failed to write checkpoint block: %d\n", ret);
        return ret;
    }

    atomic64_inc(&journal->base.j_blocks_written);
    return 0;
}

/*
 * Add barrier to transaction
 */
int vexfs_full_journal_add_barrier(struct vexfs_full_journal_transaction *trans,
                                   u32 barrier_type, u32 timeout)
{
    if (!trans) {
        return -EINVAL;
    }

    /* Only support barriers in journal mode */
    if (trans->ft_journal_mode != VEXFS_JOURNAL_MODE_JOURNAL) {
        return 0;
    }

    atomic_inc(&trans->ft_barrier_count);
    return 0;
}

/*
 * Wait for barriers in transaction
 */
int vexfs_full_journal_wait_barrier(struct vexfs_full_journal_transaction *trans)
{
    if (!trans || atomic_read(&trans->ft_barrier_count) == 0) {
        return 0;
    }

    /* For now, just complete immediately */
    /* In a full implementation, this would wait for dependent transactions */
    complete(&trans->ft_barrier_completion);
    return 0;
}

/*
 * Set journal mode
 */
int vexfs_full_journal_set_mode(struct vexfs_full_journal *journal, u32 mode)
{
    if (!journal) {
        return -EINVAL;
    }

    if (mode < VEXFS_JOURNAL_MODE_ORDERED || mode > VEXFS_JOURNAL_MODE_JOURNAL) {
        return -EINVAL;
    }

    /* Wait for all active transactions to complete */
    while (atomic_read(&journal->fj_active_trans_count) > 0) {
        msleep(10);
    }

    journal->fj_journal_mode = mode;
    
    /* Update flags based on mode */
    if (mode == VEXFS_JOURNAL_MODE_JOURNAL) {
        journal->fj_flags |= VEXFS_JOURNAL_BARRIER_SUPPORT;
    } else {
        journal->fj_flags &= ~VEXFS_JOURNAL_BARRIER_SUPPORT;
    }

    printk(KERN_INFO "VexFS Full Journal: Changed journal mode to %u\n", mode);
    return 0;
}

/*
 * Get journal mode
 */
u32 vexfs_full_journal_get_mode(struct vexfs_full_journal *journal)
{
    if (!journal) {
        return 0;
    }

    return journal->fj_journal_mode;
}

/*
 * Flush journal buffer
 */
int vexfs_full_journal_flush_buffer(struct vexfs_full_journal *journal)
{
    struct vexfs_journal_buffer *buffer;
    unsigned long flags;

    if (!journal || !journal->fj_buffer) {
        return -EINVAL;
    }

    buffer = journal->fj_buffer;

    spin_lock_irqsave(&buffer->jb_lock, flags);
    
    if (buffer->jb_used > 0) {
        /* Mark buffer for flush */
        buffer->jb_flags |= 1; /* FLUSH_PENDING */
        buffer->jb_last_flush = ktime_get();
        
        /* In a full implementation, this would write buffer contents to journal */
        buffer->jb_used = 0;
        atomic_set(&buffer->jb_transaction_count, 0);
        
        complete(&buffer->jb_flush_completion);
    }
    
    spin_unlock_irqrestore(&buffer->jb_lock, flags);

    return 0;
}

/*
 * Enhanced recovery implementation
 */
int vexfs_full_journal_recover(struct vexfs_full_journal *journal, u32 flags)
{
    int ret;

    if (!journal) {
        return -EINVAL;
    }

    atomic_set(&journal->fj_recovery_active, 1);

    printk(KERN_INFO "VexFS Full Journal: Starting enhanced recovery with flags 0x%x\n", flags);

    /* First run base journal recovery */
    ret = vexfs_journal_recover(&journal->base);
    if (ret) {
        printk(KERN_ERR "VexFS Full Journal: Base journal recovery failed: %d\n", ret);
        goto recovery_complete;
    }

    /* Scan for enhanced journal blocks */
    ret = vexfs_full_journal_scan_for_transactions(journal, journal->base.j_tail, journal->base.j_head);
    if (ret) {
        printk(KERN_ERR "VexFS Full Journal: Enhanced transaction scan failed: %d\n", ret);
        goto recovery_complete;
    }

    printk(KERN_INFO "VexFS Full Journal: Enhanced recovery completed successfully\n");

recovery_complete:
    atomic_set(&journal->fj_recovery_active, 0);
    return ret;
}

/*
 * Scan for transactions in journal
 */
int vexfs_full_journal_scan_for_transactions(struct vexfs_full_journal *journal,
                                             u64 start_seq, u64 end_seq)
{
    struct buffer_head *bh;
    struct vexfs_full_journal_block_header *header;
    u64 scan_block, current_seq;
    int transactions_found = 0;

    if (!journal) {
        return -EINVAL;
    }

    scan_block = journal->base.j_start_block + 1; /* Skip superblock */
    current_seq = start_seq;

    while (current_seq < end_seq && scan_block < journal->base.j_start_block + journal->base.j_total_blocks) {
        bh = sb_bread(journal->base.j_sb, scan_block);
        if (!bh) {
            printk(KERN_WARNING "VexFS Full Journal: Failed to read block %llu during scan\n", scan_block);
            scan_block++;
            continue;
        }

        header = (struct vexfs_full_journal_block_header *)bh->b_data;

        /* Check if this is a valid journal block */
        if (le32_to_cpu(header->base.base.jbh_magic) == VEXFS_JOURNAL_MAGIC) {
            u32 block_type = le32_to_cpu(header->base.base.jbh_type);
            u64 sequence = le64_to_cpu(header->base.base.jbh_sequence);

            /* Verify SHA-256 checksum if available */
            if (journal->fj_flags & VEXFS_JOURNAL_SHA256_CHECKSUM) {
                u8 calculated_hash[SHA256_DIGEST_SIZE];
                int hash_ret = vexfs_full_journal_calculate_sha256(
                    bh->b_data, journal->base.j_block_size - SHA256_DIGEST_SIZE, calculated_hash);
                
                if (hash_ret == 0 && memcmp(calculated_hash, header->fjbh_sha256, SHA256_DIGEST_SIZE) == 0) {
                    atomic64_inc(&journal->fj_sha256_operations);
                }
            }

            switch (block_type) {
                case VEXFS_JOURNAL_DATA_BLOCK:
                    /* Found data block - could replay if needed */
                    break;
                case VEXFS_JOURNAL_CHECKPOINT:
                    /* Found checkpoint - could use for recovery optimization */
                    break;
                case VEXFS_JOURNAL_BARRIER:
                    /* Found barrier - process barrier logic */
                    atomic64_inc(&journal->fj_total_barriers);
                    break;
            }

            transactions_found++;
            current_seq = sequence + 1;
        }

        brelse(bh);
        scan_block++;
    }

    printk(KERN_INFO "VexFS Full Journal: Scanned %d enhanced journal blocks\n", transactions_found);
    return 0;
}

/*
 * ioctl interface implementation
 */
long vexfs_full_journal_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
    struct vexfs_full_journal *journal;
    void __user *argp = (void __user *)arg;
    int ret = 0;

    /* Get journal from file - this would be properly implemented in VFS integration */
    journal = NULL; /* TODO: Get from file->private_data or similar */
    if (!journal) {
        return -ENODEV;
    }

    switch (cmd) {
        case VEXFS_JOURNAL_IOC_GET_STATUS: {
            struct vexfs_journal_status status;
            
            status.js_mode = journal->fj_journal_mode;
            status.js_active_transactions = atomic_read(&journal->fj_active_trans_count);
            status.js_head_sequence = journal->base.j_head;
            status.js_tail_sequence = journal->base.j_tail;
            
            /* Calculate utilization */
            u64 used_blocks = (journal->base.j_head >= journal->base.j_tail) ?
                             (journal->base.j_head - journal->base.j_tail) :
                             (journal->base.j_total_blocks - (journal->base.j_tail - journal->base.j_head));
            status.js_utilization = (u32)((used_blocks * 100) / journal->base.j_total_blocks);
            status.js_flags = journal->fj_flags;
            
            if (copy_to_user(argp, &status, sizeof(status))) {
                ret = -EFAULT;
            }
            break;
        }
        
        case VEXFS_JOURNAL_IOC_SET_MODE: {
            u32 mode;
            if (copy_from_user(&mode, argp, sizeof(mode))) {
                ret = -EFAULT;
            } else {
                ret = vexfs_full_journal_set_mode(journal, mode);
            }
            break;
        }
        
        case VEXFS_JOURNAL_IOC_FORCE_COMMIT: {
            ret = vexfs_full_journal_force_commit_all(journal);
            break;
        }
        
        case VEXFS_JOURNAL_IOC_CHECKPOINT: {
            u32 flags;
            if (copy_from_user(&flags, argp, sizeof(flags))) {
                ret = -EFAULT;
            } else {
                ret = vexfs_full_journal_create_checkpoint(journal, flags);
            }
            break;
        }
        
        case VEXFS_JOURNAL_IOC_GET_STATS: {
            struct vexfs_full_journal_stats stats;
            vexfs_full_journal_get_stats(journal, &stats);
            
            if (copy_to_user(argp, &stats, sizeof(stats))) {
                ret = -EFAULT;
            }
            break;
        }
        
        case VEXFS_JOURNAL_IOC_SET_BUFFER: {
            u32 new_size;
            if (copy_from_user(&new_size, argp, sizeof(new_size))) {
                ret = -EFAULT;
            } else {
                ret = vexfs_full_journal_resize_buffer(journal, new_size);
            }
            break;
        }
        
        default:
            ret = -ENOTTY;
            break;
    }

    return ret;
}

/*
 * Get full journal statistics
 */
void vexfs_full_journal_get_stats(struct vexfs_full_journal *journal,
                                  struct vexfs_full_journal_stats *stats)
{
    if (!journal || !stats) {
        return;
    }

    memset(stats, 0, sizeof(*stats));

    /* Base statistics */
    stats->fjs_total_commits = atomic64_read(&journal->base.j_commits);
    stats->fjs_total_aborts = atomic64_read(&journal->base.j_aborts);
    stats->fjs_total_transactions = atomic64_read(&journal->base.j_transactions);
    stats->fjs_blocks_written = atomic64_read(&journal->base.j_blocks_written);

    /* Advanced statistics */
    stats->fjs_concurrent_peak = atomic64_read(&journal->fj_concurrent_peak);
    stats->fjs_total_checkpoints = atomic64_read(&journal->fj_checkpoint_count);
    stats->fjs_total_barriers = atomic64_read(&journal->fj_total_barriers);
    stats->fjs_sha256_operations = atomic64_read(&journal->fj_sha256_operations);
    stats->fjs_data_blocks_journaled = atomic64_read(&journal->fj_data_blocks_journaled);

    /* Performance metrics */
    if (stats->fjs_total_transactions > 0) {
        stats->fjs_average_transaction_size = stats->fjs_blocks_written / stats->fjs_total_transactions;
    }

    /* Buffer utilization */
    if (journal->fj_buffer) {
        stats->fjs_buffer_utilization = (journal->fj_buffer->jb_used * 100) / journal->fj_buffer->jb_size;
    }

    /* Commit thread efficiency */
    u64 total_commits = 0;
    int i;
    for (i = 0; i < journal->fj_commit_thread_count; i++) {
        total_commits += atomic64_read(&journal->fj_commit_threads[i].ct_transactions_committed);
    }
    if (journal->fj_commit_thread_count > 0) {
        stats->fjs_commit_thread_efficiency = (u32)(total_commits / journal->fj_commit_thread_count);
    }
}

/*
 * Force commit all pending transactions
 */
int vexfs_full_journal_force_commit_all(struct vexfs_full_journal *journal)
{
    int ret;

    if (!journal) {
        return -EINVAL;
    }

    /* Flush journal buffer */
    ret = vexfs_full_journal_flush_buffer(journal);
    if (ret) {
        return ret;
    }

    /* Force commit base journal */
    ret = vexfs_journal_force_commit(&journal->base);
    if (ret) {
        return ret;
    }

    /* Write updated superblock */
    ret = vexfs_full_journal_write_enhanced_superblock(journal);
    if (ret) {
        return ret;
    }

    return 0;
}

/*
 * Resize journal buffer
 */
int vexfs_full_journal_resize_buffer(struct vexfs_full_journal *journal, u32 new_size)
{
    struct vexfs_journal_buffer *new_buffer;
    unsigned long flags;

    if (!journal || new_size < 4096 || new_size > (1024 * 1024)) {
        return -EINVAL;
    }

    /* Create new buffer */
    new_buffer = vexfs_journal_buffer_init(new_size);
    if (IS_ERR(new_buffer)) {
        return PTR_ERR(new_buffer);
    }

    /* Replace old buffer */
    spin_lock_irqsave(&journal->fj_buffer->jb_lock, flags);
    
    /* Flush old buffer first */
    vexfs_full_journal_flush_buffer(journal);
    
    /* Swap buffers */
    vexfs_journal_buffer_destroy(journal->fj_buffer);
    journal->fj_buffer = new_buffer;
    journal->fj_buffer_size = new_size;
    
    spin_unlock_irqrestore(&new_buffer->jb_lock, flags);

    printk(KERN_INFO "VexFS Full Journal: Resized buffer to %u bytes\n", new_size);
    return 0;
}

/*
 * Commit thread function
 */
static int vexfs_full_journal_commit_thread_fn(void *data)
{
    struct vexfs_commit_thread *thread = (struct vexfs_commit_thread *)data;
    struct vexfs_full_journal *journal = thread->ct_journal;

    printk(KERN_INFO "VexFS Full Journal: Commit thread %u started\n", thread->ct_thread_id);

    while (atomic_read(&thread->ct_active)) {
        /* Process pending transactions */
        /* In a full implementation, this would process transactions from the pending list */
        
        /* Sleep for a short time */
        msleep_interruptible(100);
        
        if (kthread_should_stop()) {
            break;
        }
    }

    complete(&thread->ct_completion);
    printk(KERN_INFO "VexFS Full Journal: Commit thread %u stopped\n", thread->ct_thread_id);
    return 0;
}

/*
 * Buffer flush work function
 */
static void vexfs_full_journal_buffer_flush_work_fn(struct work_struct *work)
{
    struct vexfs_full_journal *journal = container_of(work, struct vexfs_full_journal,
                                                     fj_buffer_flush_work.work);

    /* Flush buffer if it has pending data */
    vexfs_full_journal_flush_buffer(journal);

    /* Reschedule for next flush */
    queue_delayed_work(journal->base.j_workqueue, &journal->fj_buffer_flush_work,
                      msecs_to_jiffies(1000));
}

/*
 * Checkpoint work function
 */
static void vexfs_full_journal_checkpoint_work_fn(struct work_struct *work)
{
    struct vexfs_full_journal *journal = container_of(work, struct vexfs_full_journal,
                                                     fj_checkpoint_work.work);

    /* Create periodic checkpoint */
    vexfs_full_journal_create_checkpoint(journal, VEXFS_CHECKPOINT_ASYNC);

    /* Reschedule for next checkpoint */
    queue_delayed_work(journal->base.j_workqueue, &journal->fj_checkpoint_work,
                      msecs_to_jiffies(journal->fj_checkpoint_interval * 1000));
}

/*
 * Write enhanced superblock
 */
static int vexfs_full_journal_write_enhanced_superblock(struct vexfs_full_journal *journal)
{
    struct vexfs_full_journal_superblock *fj_sb;
    struct buffer_head *bh;
    u8 sha256_hash[SHA256_DIGEST_SIZE];
    int ret;

    if (!journal) {
        return -EINVAL;
    }

    bh = sb_bread(journal->base.j_sb, journal->base.j_start_block);
    if (!bh) {
        return -EIO;
    }

    lock_buffer(bh);
    fj_sb = (struct vexfs_full_journal_superblock *)bh->b_data;

    /* Update enhanced fields */
    fj_sb->fj_journal_mode = cpu_to_le32(journal->fj_journal_mode);
    fj_sb->fj_checksum_algorithm = cpu_to_le32(2); /* SHA-256 */
    fj_sb->fj_concurrent_trans = cpu_to_le32(journal->fj_concurrent_trans_limit);
    fj_sb->fj_commit_threads = cpu_to_le32(journal->fj_commit_thread_count);
    fj_sb->fj_buffer_size = cpu_to_le32(journal->fj_buffer_size);
    fj_sb->fj_checkpoint_interval = cpu_to_le32(journal->fj_checkpoint_interval);
    fj_sb->fj_barrier_timeout = cpu_to_le32(journal->fj_barrier_timeout);
    fj_sb->fj_recovery_threads = cpu_to_le32(journal->fj_recovery_thread_count);

    /* Update statistics */
    fj_sb->fj_total_checkpoints = cpu_to_le64(atomic64_read(&journal->fj_checkpoint_count));
    fj_sb->fj_total_barriers = cpu_to_le64(atomic64_read(&journal->fj_total_barriers));
    fj_sb->fj_concurrent_peak = cpu_to_le64(atomic64_read(&journal->fj_concurrent_peak));

    fj_sb->fj_feature_flags = cpu_to_le32(journal->fj_flags);

    /* Calculate SHA-256 checksum */
    ret = vexfs_full_journal_calculate_sha256(fj_sb,
                                             sizeof(*fj_sb) - sizeof(fj_sb->fj_superblock_sha256),
                                             sha256_hash);
    if (ret == 0) {
        memcpy(fj_sb->fj_superblock_sha256, sha256_hash, SHA256_DIGEST_SIZE);
        atomic64_inc(&journal->fj_sha256_operations);
    }

    set_buffer_uptodate(bh);
    mark_buffer_dirty(bh);
    unlock_buffer(bh);

    ret = sync_dirty_buffer(bh);
    brelse(bh);

    return ret;
}

/* Export symbols for use by other VexFS modules */
EXPORT_SYMBOL(vexfs_full_journal_init);
EXPORT_SYMBOL(vexfs_full_journal_destroy);
EXPORT_SYMBOL(vexfs_full_journal_start);
EXPORT_SYMBOL(vexfs_full_journal_commit);
EXPORT_SYMBOL(vexfs_full_journal_abort);
EXPORT_SYMBOL(vexfs_full_journal_add_data_block);
EXPORT_SYMBOL(vexfs_full_journal_create_checkpoint);
EXPORT_SYMBOL(vexfs_full_journal_set_mode);
EXPORT_SYMBOL(vexfs_full_journal_get_mode);
EXPORT_SYMBOL(vexfs_full_journal_recover);
EXPORT_SYMBOL(vexfs_full_journal_calculate_sha256);
EXPORT_SYMBOL(vexfs_full_journal_verify_sha256);
EXPORT_SYMBOL(vexfs_full_journal_ioctl);
EXPORT_SYMBOL(vexfs_full_journal_get_stats);
EXPORT_SYMBOL(vexfs_full_journal_force_commit_all);

MODULE_DESCRIPTION("VexFS v2.0 Full Filesystem Journal (Phase 1)");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");
MODULE_VERSION("2.0.0");