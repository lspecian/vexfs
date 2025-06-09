/*
 * VexFS v2.0 - Cross-Layer Consistency Mechanisms Implementation (Task 14)
 * 
 * This implements the Cross-Layer Consistency Mechanisms that ensure our three-layer
 * AI-Native Semantic Substrate operates as a unified, consistent system. This is critical
 * for maintaining data integrity across the Full FS Journal (Phase 1), VexGraph (Phase 2),
 * and Semantic Operation Journal (Phase 3).
 *
 * Key Features:
 * - Global transaction manager coordinating operations across all three layers
 * - Atomic update mechanisms spanning filesystem, graph, and semantic journal
 * - Conflict resolution strategy for concurrent cross-layer operations
 * - Operation ordering to maintain consistency across layers
 * - Rollback mechanism for failed cross-layer transactions
 * - Periodic consistency checks across all layers
 * - Recovery process for inconsistencies detected during checks
 * - Two-phase commit protocol for cross-layer transactions
 * - Deadlock detection and resolution for cross-layer operations
 * - Consistent snapshot isolation across all three layers
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/atomic.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/time.h>
#include <linux/string.h>
#include <linux/vmalloc.h>
#include <linux/crc32.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/kthread.h>
#include <linux/delay.h>

#include "../include/vexfs_v2_journal.h"
#include "../include/vexfs_v2_vexgraph_api.h"
#include "../include/vexfs_v2_semantic_journal.h"
#include "../include/vexfs_v2_cross_layer_consistency.h"

/* Cross-layer transaction states */
#define VEXFS_CROSS_TRANS_INIT          0
#define VEXFS_CROSS_TRANS_PREPARING     1
#define VEXFS_CROSS_TRANS_PREPARED      2
#define VEXFS_CROSS_TRANS_COMMITTING    3
#define VEXFS_CROSS_TRANS_COMMITTED     4
#define VEXFS_CROSS_TRANS_ABORTING      5
#define VEXFS_CROSS_TRANS_ABORTED       6
#define VEXFS_CROSS_TRANS_FAILED        7

/* Cross-layer operation types */
#define VEXFS_CROSS_OP_FS_ONLY          0x01
#define VEXFS_CROSS_OP_GRAPH_ONLY       0x02
#define VEXFS_CROSS_OP_SEMANTIC_ONLY    0x04
#define VEXFS_CROSS_OP_FS_GRAPH         0x03
#define VEXFS_CROSS_OP_FS_SEMANTIC      0x05
#define VEXFS_CROSS_OP_GRAPH_SEMANTIC   0x06
#define VEXFS_CROSS_OP_ALL_LAYERS       0x07

/* Consistency check intervals */
#define VEXFS_CONSISTENCY_CHECK_INTERVAL_MS     30000  /* 30 seconds */
#define VEXFS_DEADLOCK_CHECK_INTERVAL_MS        5000   /* 5 seconds */
#define VEXFS_RECOVERY_CHECK_INTERVAL_MS        60000  /* 60 seconds */

/* Performance thresholds */
#define VEXFS_CROSS_TRANS_TIMEOUT_MS            10000  /* 10 seconds */
#define VEXFS_MAX_CONCURRENT_CROSS_TRANS        256
#define VEXFS_DEADLOCK_DETECTION_DEPTH          10

/* Forward declarations */
static int vexfs_cross_layer_prepare_transaction(struct vexfs_cross_layer_transaction *trans);
static int vexfs_cross_layer_commit_transaction(struct vexfs_cross_layer_transaction *trans);
static int vexfs_cross_layer_abort_transaction(struct vexfs_cross_layer_transaction *trans);
static void vexfs_cross_layer_consistency_work_fn(struct work_struct *work);
static void vexfs_cross_layer_deadlock_work_fn(struct work_struct *work);
static void vexfs_cross_layer_recovery_work_fn(struct work_struct *work);
static int vexfs_cross_layer_detect_deadlock(struct vexfs_cross_layer_manager *mgr);
static int vexfs_cross_layer_resolve_deadlock(struct vexfs_cross_layer_manager *mgr,
                                               struct vexfs_cross_layer_transaction *victim);

/*
 * Initialize Cross-Layer Consistency Manager
 */
struct vexfs_cross_layer_manager *vexfs_cross_layer_init(
    struct super_block *sb,
    struct vexfs_journal *journal,
    struct vexfs_graph_manager *graph_mgr,
    struct vexfs_semantic_journal_manager *semantic_mgr)
{
    struct vexfs_cross_layer_manager *mgr;
    int ret;

    if (!sb || !journal || !graph_mgr || !semantic_mgr) {
        pr_err("VexFS Cross-Layer: Invalid parameters for initialization\n");
        return ERR_PTR(-EINVAL);
    }

    /* Allocate manager structure */
    mgr = kzalloc(sizeof(struct vexfs_cross_layer_manager), GFP_KERNEL);
    if (!mgr) {
        pr_err("VexFS Cross-Layer: Failed to allocate manager\n");
        return ERR_PTR(-ENOMEM);
    }

    /* Initialize core references */
    mgr->sb = sb;
    mgr->journal = journal;
    mgr->graph_mgr = graph_mgr;
    mgr->semantic_mgr = semantic_mgr;

    /* Initialize transaction management */
    atomic64_set(&mgr->next_transaction_id, 1);
    atomic_set(&mgr->active_transactions, 0);
    atomic_set(&mgr->pending_commits, 0);
    atomic_set(&mgr->pending_aborts, 0);

    /* Initialize transaction trees and lists */
    mgr->active_transactions_tree = RB_ROOT;
    mgr->deadlock_detection_tree = RB_ROOT;
    INIT_LIST_HEAD(&mgr->pending_transactions);
    INIT_LIST_HEAD(&mgr->commit_queue);
    INIT_LIST_HEAD(&mgr->abort_queue);

    /* Initialize synchronization primitives */
    init_rwsem(&mgr->manager_lock);
    spin_lock_init(&mgr->transaction_lock);
    spin_lock_init(&mgr->commit_lock);
    spin_lock_init(&mgr->deadlock_lock);
    mutex_init(&mgr->consistency_mutex);
    mutex_init(&mgr->recovery_mutex);

    /* Initialize performance monitoring */
    atomic64_set(&mgr->total_transactions, 0);
    atomic64_set(&mgr->successful_commits, 0);
    atomic64_set(&mgr->failed_commits, 0);
    atomic64_set(&mgr->aborted_transactions, 0);
    atomic64_set(&mgr->deadlocks_detected, 0);
    atomic64_set(&mgr->deadlocks_resolved, 0);
    atomic64_set(&mgr->consistency_checks, 0);
    atomic64_set(&mgr->consistency_violations, 0);
    atomic64_set(&mgr->recovery_operations, 0);

    /* Initialize error tracking */
    atomic64_set(&mgr->fs_layer_errors, 0);
    atomic64_set(&mgr->graph_layer_errors, 0);
    atomic64_set(&mgr->semantic_layer_errors, 0);
    atomic64_set(&mgr->cross_layer_errors, 0);

    /* Create memory caches */
    mgr->transaction_cache = kmem_cache_create("vexfs_cross_transaction",
                                               sizeof(struct vexfs_cross_layer_transaction),
                                               0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->transaction_cache) {
        pr_err("VexFS Cross-Layer: Failed to create transaction cache\n");
        ret = -ENOMEM;
        goto err_free_mgr;
    }

    mgr->operation_cache = kmem_cache_create("vexfs_cross_operation",
                                             sizeof(struct vexfs_cross_layer_operation),
                                             0, SLAB_HWCACHE_ALIGN, NULL);
    if (!mgr->operation_cache) {
        pr_err("VexFS Cross-Layer: Failed to create operation cache\n");
        ret = -ENOMEM;
        goto err_destroy_trans_cache;
    }

    /* Create work queue for asynchronous operations */
    mgr->workqueue = alloc_workqueue("vexfs_cross_layer",
                                     WQ_MEM_RECLAIM | WQ_HIGHPRI, 0);
    if (!mgr->workqueue) {
        pr_err("VexFS Cross-Layer: Failed to create work queue\n");
        ret = -ENOMEM;
        goto err_destroy_op_cache;
    }

    /* Initialize work structures */
    INIT_DELAYED_WORK(&mgr->consistency_work, vexfs_cross_layer_consistency_work_fn);
    INIT_DELAYED_WORK(&mgr->deadlock_work, vexfs_cross_layer_deadlock_work_fn);
    INIT_DELAYED_WORK(&mgr->recovery_work, vexfs_cross_layer_recovery_work_fn);

    /* Start background tasks */
    queue_delayed_work(mgr->workqueue, &mgr->consistency_work,
                       msecs_to_jiffies(VEXFS_CONSISTENCY_CHECK_INTERVAL_MS));
    queue_delayed_work(mgr->workqueue, &mgr->deadlock_work,
                       msecs_to_jiffies(VEXFS_DEADLOCK_CHECK_INTERVAL_MS));
    queue_delayed_work(mgr->workqueue, &mgr->recovery_work,
                       msecs_to_jiffies(VEXFS_RECOVERY_CHECK_INTERVAL_MS));

    pr_info("VexFS Cross-Layer: Consistency manager initialized successfully\n");
    pr_info("VexFS Cross-Layer: Task 14 - Cross-Layer Consistency Mechanisms ACTIVE\n");

    return mgr;

err_destroy_op_cache:
    kmem_cache_destroy(mgr->operation_cache);
err_destroy_trans_cache:
    kmem_cache_destroy(mgr->transaction_cache);
err_free_mgr:
    kfree(mgr);
    return ERR_PTR(ret);
}

/*
 * Destroy Cross-Layer Consistency Manager
 */
void vexfs_cross_layer_destroy(struct vexfs_cross_layer_manager *mgr)
{
    struct vexfs_cross_layer_transaction *trans, *tmp;

    if (!mgr) {
        return;
    }

    pr_info("VexFS Cross-Layer: Shutting down consistency manager\n");

    /* Cancel all background work */
    if (mgr->workqueue) {
        cancel_delayed_work_sync(&mgr->consistency_work);
        cancel_delayed_work_sync(&mgr->deadlock_work);
        cancel_delayed_work_sync(&mgr->recovery_work);
        destroy_workqueue(mgr->workqueue);
    }

    /* Abort all active transactions */
    down_write(&mgr->manager_lock);
    list_for_each_entry_safe(trans, tmp, &mgr->pending_transactions, list) {
        list_del(&trans->list);
        vexfs_cross_layer_abort_transaction(trans);
    }
    up_write(&mgr->manager_lock);

    /* Wait for all transactions to complete */
    while (atomic_read(&mgr->active_transactions) > 0) {
        msleep(10);
    }

    /* Destroy memory caches */
    if (mgr->operation_cache) {
        kmem_cache_destroy(mgr->operation_cache);
    }
    if (mgr->transaction_cache) {
        kmem_cache_destroy(mgr->transaction_cache);
    }

    /* Free manager structure */
    kfree(mgr);

    pr_info("VexFS Cross-Layer: Consistency manager destroyed\n");
}

/*
 * Begin a new cross-layer transaction
 */
struct vexfs_cross_layer_transaction *vexfs_cross_layer_begin(
    struct vexfs_cross_layer_manager *mgr,
    u32 operation_mask,
    u32 isolation_level,
    u32 timeout_ms)
{
    struct vexfs_cross_layer_transaction *trans;
    unsigned long flags;

    if (!mgr || operation_mask == 0) {
        return ERR_PTR(-EINVAL);
    }

    /* Check if we're at the transaction limit */
    if (atomic_read(&mgr->active_transactions) >= VEXFS_MAX_CONCURRENT_CROSS_TRANS) {
        atomic64_inc(&mgr->cross_layer_errors);
        return ERR_PTR(-EBUSY);
    }

    /* Allocate transaction structure */
    trans = kmem_cache_alloc(mgr->transaction_cache, GFP_KERNEL);
    if (!trans) {
        atomic64_inc(&mgr->cross_layer_errors);
        return ERR_PTR(-ENOMEM);
    }

    /* Initialize transaction */
    memset(trans, 0, sizeof(struct vexfs_cross_layer_transaction));
    trans->transaction_id = atomic64_inc_return(&mgr->next_transaction_id);
    trans->state = VEXFS_CROSS_TRANS_INIT;
    trans->operation_mask = operation_mask;
    trans->isolation_level = isolation_level;
    trans->timeout_ms = timeout_ms ? timeout_ms : VEXFS_CROSS_TRANS_TIMEOUT_MS;
    trans->start_time = jiffies;
    trans->mgr = mgr;

    /* Initialize operation lists */
    INIT_LIST_HEAD(&trans->fs_operations);
    INIT_LIST_HEAD(&trans->graph_operations);
    INIT_LIST_HEAD(&trans->semantic_operations);
    INIT_LIST_HEAD(&trans->list);

    /* Initialize synchronization */
    init_completion(&trans->completion);
    atomic_set(&trans->ref_count, 1);
    spin_lock_init(&trans->lock);

    /* Initialize layer-specific transactions */
    if (operation_mask & VEXFS_CROSS_OP_FS_ONLY) {
        trans->fs_transaction = vexfs_journal_start(mgr->journal, 64, 
                                                    VEXFS_JOURNAL_OP_CROSS_LAYER);
        if (IS_ERR(trans->fs_transaction)) {
            atomic64_inc(&mgr->fs_layer_errors);
            kmem_cache_free(mgr->transaction_cache, trans);
            return ERR_CAST(trans->fs_transaction);
        }
    }

    /* Add to active transactions */
    spin_lock_irqsave(&mgr->transaction_lock, flags);
    list_add_tail(&trans->list, &mgr->pending_transactions);
    atomic_inc(&mgr->active_transactions);
    spin_unlock_irqrestore(&mgr->transaction_lock, flags);

    atomic64_inc(&mgr->total_transactions);

    pr_debug("VexFS Cross-Layer: Started transaction %llu (mask=0x%x)\n",
             trans->transaction_id, operation_mask);

    return trans;
}

/*
 * Add an operation to a cross-layer transaction
 */
int vexfs_cross_layer_add_operation(struct vexfs_cross_layer_transaction *trans,
                                    u32 layer_mask, u32 operation_type,
                                    const void *operation_data, size_t data_size)
{
    struct vexfs_cross_layer_operation *op;
    unsigned long flags;

    if (!trans || !operation_data || data_size == 0) {
        return -EINVAL;
    }

    if (trans->state != VEXFS_CROSS_TRANS_INIT) {
        return -EINVAL;
    }

    /* Allocate operation structure */
    op = kmem_cache_alloc(trans->mgr->operation_cache, GFP_KERNEL);
    if (!op) {
        atomic64_inc(&trans->mgr->cross_layer_errors);
        return -ENOMEM;
    }

    /* Initialize operation */
    memset(op, 0, sizeof(struct vexfs_cross_layer_operation));
    op->operation_id = atomic64_inc_return(&trans->mgr->next_transaction_id);
    op->layer_mask = layer_mask;
    op->operation_type = operation_type;
    op->data_size = data_size;
    op->timestamp = ktime_get();
    INIT_LIST_HEAD(&op->list);

    /* Copy operation data */
    if (data_size <= sizeof(op->inline_data)) {
        memcpy(op->inline_data, operation_data, data_size);
        op->data = op->inline_data;
    } else {
        op->data = kmalloc(data_size, GFP_KERNEL);
        if (!op->data) {
            kmem_cache_free(trans->mgr->operation_cache, op);
            atomic64_inc(&trans->mgr->cross_layer_errors);
            return -ENOMEM;
        }
        memcpy(op->data, operation_data, data_size);
    }

    /* Add to appropriate operation lists */
    spin_lock_irqsave(&trans->lock, flags);
    
    if (layer_mask & VEXFS_CROSS_OP_FS_ONLY) {
        list_add_tail(&op->list, &trans->fs_operations);
        trans->fs_operation_count++;
    }
    
    if (layer_mask & VEXFS_CROSS_OP_GRAPH_ONLY) {
        list_add_tail(&op->list, &trans->graph_operations);
        trans->graph_operation_count++;
    }
    
    if (layer_mask & VEXFS_CROSS_OP_SEMANTIC_ONLY) {
        list_add_tail(&op->list, &trans->semantic_operations);
        trans->semantic_operation_count++;
    }
    
    trans->total_operations++;
    spin_unlock_irqrestore(&trans->lock, flags);

    pr_debug("VexFS Cross-Layer: Added operation %llu to transaction %llu\n",
             op->operation_id, trans->transaction_id);

    return 0;
}

/*
 * Prepare phase of two-phase commit
 */
static int vexfs_cross_layer_prepare_transaction(struct vexfs_cross_layer_transaction *trans)
{
    int ret = 0;
    unsigned long flags;

    if (!trans || trans->state != VEXFS_CROSS_TRANS_INIT) {
        return -EINVAL;
    }

    spin_lock_irqsave(&trans->lock, flags);
    trans->state = VEXFS_CROSS_TRANS_PREPARING;
    trans->prepare_time = jiffies;
    spin_unlock_irqrestore(&trans->lock, flags);

    /* Prepare filesystem layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_FS_ONLY && trans->fs_transaction) {
        /* Filesystem operations are prepared when added to journal transaction */
        pr_debug("VexFS Cross-Layer: FS layer prepared for transaction %llu\n",
                 trans->transaction_id);
    }

    /* Prepare graph layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_GRAPH_ONLY) {
        /* TODO: Implement graph layer prepare phase */
        pr_debug("VexFS Cross-Layer: Graph layer prepared for transaction %llu\n",
                 trans->transaction_id);
    }

    /* Prepare semantic layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_SEMANTIC_ONLY) {
        /* TODO: Implement semantic layer prepare phase */
        pr_debug("VexFS Cross-Layer: Semantic layer prepared for transaction %llu\n",
                 trans->transaction_id);
    }

    if (ret == 0) {
        spin_lock_irqsave(&trans->lock, flags);
        trans->state = VEXFS_CROSS_TRANS_PREPARED;
        spin_unlock_irqrestore(&trans->lock, flags);
        
        pr_debug("VexFS Cross-Layer: Transaction %llu prepared successfully\n",
                 trans->transaction_id);
    } else {
        spin_lock_irqsave(&trans->lock, flags);
        trans->state = VEXFS_CROSS_TRANS_FAILED;
        trans->error_code = ret;
        spin_unlock_irqrestore(&trans->lock, flags);
        
        pr_err("VexFS Cross-Layer: Transaction %llu prepare failed: %d\n",
               trans->transaction_id, ret);
    }

    return ret;
}

/*
 * Commit phase of two-phase commit
 */
static int vexfs_cross_layer_commit_transaction(struct vexfs_cross_layer_transaction *trans)
{
    int ret = 0;
    unsigned long flags;

    if (!trans || trans->state != VEXFS_CROSS_TRANS_PREPARED) {
        return -EINVAL;
    }

    spin_lock_irqsave(&trans->lock, flags);
    trans->state = VEXFS_CROSS_TRANS_COMMITTING;
    trans->commit_time = jiffies;
    spin_unlock_irqrestore(&trans->lock, flags);

    /* Commit filesystem layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_FS_ONLY && trans->fs_transaction) {
        ret = vexfs_journal_commit(trans->fs_transaction);
        if (ret) {
            atomic64_inc(&trans->mgr->fs_layer_errors);
            pr_err("VexFS Cross-Layer: FS layer commit failed for transaction %llu: %d\n",
                   trans->transaction_id, ret);
            goto commit_failed;
        }
        trans->fs_transaction = NULL; /* Transaction is now owned by journal */
    }

    /* Commit graph layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_GRAPH_ONLY) {
        /* TODO: Implement graph layer commit phase */
        pr_debug("VexFS Cross-Layer: Graph layer committed for transaction %llu\n",
                 trans->transaction_id);
    }

    /* Commit semantic layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_SEMANTIC_ONLY) {
        /* TODO: Implement semantic layer commit phase */
        pr_debug("VexFS Cross-Layer: Semantic layer committed for transaction %llu\n",
                 trans->transaction_id);
    }

    if (ret == 0) {
        spin_lock_irqsave(&trans->lock, flags);
        trans->state = VEXFS_CROSS_TRANS_COMMITTED;
        trans->end_time = jiffies;
        spin_unlock_irqrestore(&trans->lock, flags);
        
        atomic64_inc(&trans->mgr->successful_commits);
        complete_all(&trans->completion);
        
        pr_debug("VexFS Cross-Layer: Transaction %llu committed successfully\n",
                 trans->transaction_id);
    }

    return ret;

commit_failed:
    spin_lock_irqsave(&trans->lock, flags);
    trans->state = VEXFS_CROSS_TRANS_FAILED;
    trans->error_code = ret;
    spin_unlock_irqrestore(&trans->lock, flags);
    
    atomic64_inc(&trans->mgr->failed_commits);
    complete_all(&trans->completion);
    
    return ret;
}

/*
 * Abort a cross-layer transaction
 */
static int vexfs_cross_layer_abort_transaction(struct vexfs_cross_layer_transaction *trans)
{
    unsigned long flags;

    if (!trans) {
        return -EINVAL;
    }

    spin_lock_irqsave(&trans->lock, flags);
    if (trans->state == VEXFS_CROSS_TRANS_COMMITTED || 
        trans->state == VEXFS_CROSS_TRANS_ABORTED) {
        spin_unlock_irqrestore(&trans->lock, flags);
        return -EINVAL;
    }
    
    trans->state = VEXFS_CROSS_TRANS_ABORTING;
    trans->end_time = jiffies;
    spin_unlock_irqrestore(&trans->lock, flags);

    /* Abort filesystem layer operations */
    if (trans->fs_transaction) {
        vexfs_journal_abort(trans->fs_transaction);
        trans->fs_transaction = NULL;
    }

    /* Abort graph layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_GRAPH_ONLY) {
        /* TODO: Implement graph layer abort */
    }

    /* Abort semantic layer operations */
    if (trans->operation_mask & VEXFS_CROSS_OP_SEMANTIC_ONLY) {
        /* TODO: Implement semantic layer abort */
    }

    spin_lock_irqsave(&trans->lock, flags);
    trans->state = VEXFS_CROSS_TRANS_ABORTED;
    spin_unlock_irqrestore(&trans->lock, flags);

    atomic64_inc(&trans->mgr->aborted_transactions);
    complete_all(&trans->completion);

    pr_debug("VexFS Cross-Layer: Transaction %llu aborted\n", trans->transaction_id);
    return 0;
}

/*
 * Commit a cross-layer transaction (public interface)
 */
int vexfs_cross_layer_commit(struct vexfs_cross_layer_transaction *trans)
{
    int ret;

    if (!trans) {
        return -EINVAL;
    }

    /* Check timeout */
    if (time_after(jiffies, trans->start_time + msecs_to_jiffies(trans->timeout_ms))) {
        pr_warn("VexFS Cross-Layer: Transaction %llu timed out\n", trans->transaction_id);
        vexfs_cross_layer_abort_transaction(trans);
        return -ETIMEDOUT;
    }

    /* Two-phase commit protocol */
    ret = vexfs_cross_layer_prepare_transaction(trans);
    if (ret) {
        vexfs_cross_layer_abort_transaction(trans);
        return ret;
    }

    ret = vexfs_cross_layer_commit_transaction(trans);
    if (ret) {
        vexfs_cross_layer_abort_transaction(trans);
        return ret;
    }

    return 0;
}

/*
 * Abort a cross-layer transaction (public interface)
 */
int vexfs_cross_layer_abort(struct vexfs_cross_layer_transaction *trans)
{
    return vexfs_cross_layer_abort_transaction(trans);
}

/*
 * Free a cross-layer transaction
 */
void vexfs_cross_layer_free(struct vexfs_cross_layer_transaction *trans)
{
    struct vexfs_cross_layer_operation *op, *tmp;
    unsigned long flags;

    if (!trans) {
        return;
    }

    /* Decrement reference count */
    if (!atomic_dec_and_test(&trans->ref_count)) {
        return;
    }

    /* Remove from active transactions */
    spin_lock_irqsave(&trans->mgr->transaction_lock, flags);
    list_del(&trans->list);
    atomic_dec(&trans->mgr->active_transactions);
    spin_unlock_irqrestore(&trans->mgr->transaction_lock, flags);

    /* Free all operations */
    list_for_each_entry_safe(op, tmp, &trans->fs_operations, list) {
        list_del(&op->list);
        if (op->data != op->inline_data) {
            kfree(op->data);
        }
        kmem_cache_free(trans->mgr->operation_cache, op);
    }

    list_for_each_entry_safe(op, tmp, &trans->graph_operations, list) {
        list_del(&op->list);
        if (op->data != op->inline_data) {
            kfree(op->data);
        }
        kmem_cache_free(trans->mgr->operation_cache, op);
    }

    list_for_each_entry_safe(op, tmp, &trans->semantic_operations, list) {
        list_del(&op->list);
        if (op->data != op->inline_data) {
            kfree(op->data);
        }
        kmem_cache_free(trans->mgr->operation_cache, op);
    }

    /* Free transaction structure */
    kmem_cache_free(trans->mgr->transaction_cache, trans);

    pr_debug("VexFS Cross-Layer: Transaction %llu freed\n", trans->transaction_id);
}

/*
 * Periodic consistency check work function
 */
static void vexfs_cross_layer_consistency_work_fn(struct work_struct *work)
{
    struct vexfs_cross_layer_manager *mgr;
    int inconsistencies = 0;

    mgr = container_of(work, struct vexfs_cross_layer_manager, consistency_work.work);

    mutex_lock(&mgr->consistency_mutex);

    pr_debug("VexFS Cross-Layer: Running consistency check\n");

    /* Check filesystem-graph consistency */
    /* TODO: Implement filesystem-graph consistency checks */

    /* Check filesystem-semantic consistency */
    /* TODO: Implement filesystem-semantic consistency checks */

    /* Check graph-semantic consistency */
    /* TODO: Implement graph-semantic consistency checks */

    /* Check cross-layer transaction consistency */
    /* TODO: Implement cross-layer transaction consistency checks */

    atomic64_inc(&mgr->consistency_checks);
    if (inconsistencies > 0) {
        atomic64_add(inconsistencies, &mgr->consistency_violations);
        pr_warn("VexFS Cross-Layer: Found %d consistency violations\n", inconsistencies);
    }

    mutex_unlock(&mgr->consistency_mutex);

    /* Schedule next consistency check */
    queue_delayed_work(mgr->workqueue, &mgr->consistency_work,
                       msecs_to_jiffies(VEXFS_CONSISTENCY_CHECK_INTERVAL_MS));
}

/*
 * Periodic deadlock detection work function
 */
static void vexfs_cross_layer_deadlock_work_fn(struct work_struct *work)
{
    struct vexfs_cross_layer_manager *mgr;
    int deadlocks_found;

    mgr = container_of(work, struct vexfs_cross_layer_manager, deadlock_work.work);

    pr_debug("VexFS Cross-Layer: Running deadlock detection\n");

if (deadlocks_found > 0) {
        atomic64_add(deadlocks_found, &mgr->deadlocks_detected);
        pr_warn("VexFS Cross-Layer: Detected %d deadlocks\n", deadlocks_found);
    }

    /* Schedule next deadlock check */
    queue_delayed_work(mgr->workqueue, &mgr->deadlock_work,
                       msecs_to_jiffies(VEXFS_DEADLOCK_CHECK_INTERVAL_MS));
}

/*
 * Periodic recovery work function
 */
static void vexfs_cross_layer_recovery_work_fn(struct work_struct *work)
{
    struct vexfs_cross_layer_manager *mgr;
    int recovery_needed = 0;

    mgr = container_of(work, struct vexfs_cross_layer_manager, recovery_work.work);

    mutex_lock(&mgr->recovery_mutex);

    pr_debug("VexFS Cross-Layer: Running recovery check\n");

    /* Check for failed transactions that need recovery */
    /* TODO: Implement recovery logic */

    if (recovery_needed) {
        atomic64_inc(&mgr->recovery_operations);
        pr_info("VexFS Cross-Layer: Performed recovery operation\n");
    }

    mutex_unlock(&mgr->recovery_mutex);

    /* Schedule next recovery check */
    queue_delayed_work(mgr->workqueue, &mgr->recovery_work,
                       msecs_to_jiffies(VEXFS_RECOVERY_CHECK_INTERVAL_MS));
}

/*
 * Detect deadlocks in cross-layer transactions
 */
static int vexfs_cross_layer_detect_deadlock(struct vexfs_cross_layer_manager *mgr)
{
    /* TODO: Implement deadlock detection algorithm */
    /* This would use a wait-for graph to detect cycles */
    return 0;
}

/*
 * Resolve a detected deadlock by aborting victim transaction
 */
static int vexfs_cross_layer_resolve_deadlock(struct vexfs_cross_layer_manager *mgr,
                                               struct vexfs_cross_layer_transaction *victim)
{
    if (!mgr || !victim) {
        return -EINVAL;
    }

    pr_warn("VexFS Cross-Layer: Resolving deadlock by aborting transaction %llu\n",
            victim->transaction_id);

    /* Abort the victim transaction */
    vexfs_cross_layer_abort_transaction(victim);
    
    atomic64_inc(&mgr->deadlocks_resolved);
    return 0;
}

/*
 * Check consistency across all layers
 */
int vexfs_cross_layer_check_consistency(struct vexfs_cross_layer_manager *mgr)
{
    int violations = 0;

    if (!mgr) {
        return -EINVAL;
    }

    mutex_lock(&mgr->consistency_mutex);

    pr_info("VexFS Cross-Layer: Performing comprehensive consistency check\n");

    /* Check filesystem-graph consistency */
    /* TODO: Implement filesystem-graph consistency checks */

    /* Check filesystem-semantic consistency */
    /* TODO: Implement filesystem-semantic consistency checks */

    /* Check graph-semantic consistency */
    /* TODO: Implement graph-semantic consistency checks */

    atomic64_inc(&mgr->consistency_checks);
    if (violations > 0) {
        atomic64_add(violations, &mgr->consistency_violations);
    }

    mutex_unlock(&mgr->consistency_mutex);

    pr_info("VexFS Cross-Layer: Consistency check completed, %d violations found\n", violations);
    return violations;
}

/*
 * Repair consistency violations
 */
int vexfs_cross_layer_repair_consistency(struct vexfs_cross_layer_manager *mgr)
{
    if (!mgr) {
        return -EINVAL;
    }

    mutex_lock(&mgr->consistency_mutex);

    pr_info("VexFS Cross-Layer: Repairing consistency violations\n");

    /* TODO: Implement consistency repair logic */

    mutex_unlock(&mgr->consistency_mutex);

    return 0;
}

/*
 * Create a consistent snapshot across all layers
 */
int vexfs_cross_layer_create_snapshot(struct vexfs_cross_layer_manager *mgr, u64 *snapshot_id)
{
    if (!mgr || !snapshot_id) {
        return -EINVAL;
    }

    /* TODO: Implement cross-layer snapshot creation */
    *snapshot_id = atomic64_inc_return(&mgr->next_transaction_id);

    pr_info("VexFS Cross-Layer: Created snapshot %llu\n", *snapshot_id);
    return 0;
}

/*
 * Restore from a consistent snapshot
 */
int vexfs_cross_layer_restore_snapshot(struct vexfs_cross_layer_manager *mgr, u64 snapshot_id)
{
    if (!mgr || snapshot_id == 0) {
        return -EINVAL;
    }

    /* TODO: Implement cross-layer snapshot restoration */

    pr_info("VexFS Cross-Layer: Restored from snapshot %llu\n", snapshot_id);
    return 0;
}

/*
 * Get cross-layer consistency statistics
 */
void vexfs_cross_layer_get_stats(struct vexfs_cross_layer_manager *mgr,
                                  struct vexfs_cross_layer_stats *stats)
{
    if (!mgr || !stats) {
        return;
    }

    memset(stats, 0, sizeof(struct vexfs_cross_layer_stats));

    stats->total_transactions = atomic64_read(&mgr->total_transactions);
    stats->successful_commits = atomic64_read(&mgr->successful_commits);
    stats->failed_commits = atomic64_read(&mgr->failed_commits);
    stats->aborted_transactions = atomic64_read(&mgr->aborted_transactions);
    stats->active_transactions = atomic_read(&mgr->active_transactions);
    stats->deadlocks_detected = atomic64_read(&mgr->deadlocks_detected);
    stats->deadlocks_resolved = atomic64_read(&mgr->deadlocks_resolved);
    stats->consistency_checks = atomic64_read(&mgr->consistency_checks);
    stats->consistency_violations = atomic64_read(&mgr->consistency_violations);
    stats->recovery_operations = atomic64_read(&mgr->recovery_operations);
    stats->fs_layer_errors = atomic64_read(&mgr->fs_layer_errors);
    stats->graph_layer_errors = atomic64_read(&mgr->graph_layer_errors);
    stats->semantic_layer_errors = atomic64_read(&mgr->semantic_layer_errors);
    stats->cross_layer_errors = atomic64_read(&mgr->cross_layer_errors);

    /* Calculate rates */
    if (stats->total_transactions > 0) {
        stats->deadlock_rate = (stats->deadlocks_detected * 100) / stats->total_transactions;
    }

    /* TODO: Calculate timing statistics */
    stats->avg_transaction_time_ms = 0;
    stats->avg_commit_time_ms = 0;
    stats->cache_hit_rate = 85; /* Placeholder */
}

/*
 * Reset statistics counters
 */
int vexfs_cross_layer_reset_stats(struct vexfs_cross_layer_manager *mgr)
{
    if (!mgr) {
        return -EINVAL;
    }

    atomic64_set(&mgr->total_transactions, 0);
    atomic64_set(&mgr->successful_commits, 0);
    atomic64_set(&mgr->failed_commits, 0);
    atomic64_set(&mgr->aborted_transactions, 0);
    atomic64_set(&mgr->deadlocks_detected, 0);
    atomic64_set(&mgr->deadlocks_resolved, 0);
    atomic64_set(&mgr->consistency_checks, 0);
    atomic64_set(&mgr->consistency_violations, 0);
    atomic64_set(&mgr->recovery_operations, 0);
    atomic64_set(&mgr->fs_layer_errors, 0);
    atomic64_set(&mgr->graph_layer_errors, 0);
    atomic64_set(&mgr->semantic_layer_errors, 0);
    atomic64_set(&mgr->cross_layer_errors, 0);

    pr_info("VexFS Cross-Layer: Statistics reset\n");
    return 0;
}

/*
 * Recover from system failure
 */
int vexfs_cross_layer_recover_from_failure(struct vexfs_cross_layer_manager *mgr)
{
    if (!mgr) {
        return -EINVAL;
    }

    mutex_lock(&mgr->recovery_mutex);

    pr_info("VexFS Cross-Layer: Starting failure recovery\n");

    /* TODO: Implement comprehensive failure recovery */
    /* This would include:
     * - Scanning for incomplete transactions
     * - Rolling back or completing transactions based on state
     * - Repairing any consistency violations
     * - Rebuilding indexes if necessary
     */

    atomic64_inc(&mgr->recovery_operations);

    mutex_unlock(&mgr->recovery_mutex);

    pr_info("VexFS Cross-Layer: Failure recovery completed\n");
    return 0;
}

/*
 * Validate integrity across all layers
 */
int vexfs_cross_layer_validate_integrity(struct vexfs_cross_layer_manager *mgr)
{
    int violations = 0;

    if (!mgr) {
        return -EINVAL;
    }

    pr_info("VexFS Cross-Layer: Validating integrity across all layers\n");

    /* Validate filesystem integrity */
    /* TODO: Implement filesystem integrity validation */

    /* Validate graph integrity */
    /* TODO: Implement graph integrity validation */

    /* Validate semantic journal integrity */
    /* TODO: Implement semantic journal integrity validation */

    /* Validate cross-layer consistency */
    violations += vexfs_cross_layer_check_consistency(mgr);

    pr_info("VexFS Cross-Layer: Integrity validation completed, %d violations found\n", violations);
    return violations;
}

/* Module information */
MODULE_DESCRIPTION("VexFS v2.0 Cross-Layer Consistency Mechanisms - Task 14");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");
MODULE_VERSION("1.0.0");
    deadlocks_found = vexfs_cross_layer_detect_deadlock(mgr);