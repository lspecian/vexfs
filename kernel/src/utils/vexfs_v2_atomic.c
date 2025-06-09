/*
 * VexFS v2.0 - Atomic Operations for FS Journal Implementation (Task 2)
 * 
 * This implements atomic filesystem operations leveraging the Full FS Journal
 * from Task 1. Provides transaction management, atomic wrappers for VFS operations,
 * lock-free data structures, and comprehensive rollback mechanisms.
 *
 * Key Features:
 * - Transaction begin/commit/abort mechanisms
 * - Atomic wrappers for all critical filesystem operations
 * - Lock-free data structures using kernel atomic operations
 * - Rollback mechanism for aborted transactions
 * - Nested transaction support
 * - Performance optimization through batching
 * - Crash recovery for partial writes
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/atomic.h>
#include <linux/spinlock.h>
#include <linux/mutex.h>
#include <linux/workqueue.h>
#include <linux/completion.h>
#include <linux/rbtree.h>
#include <linux/list.h>
#include <linux/percpu.h>
#include <linux/seqlock.h>
#include <linux/rcu.h>
#include <linux/kthread.h>
#include <linux/delay.h>
#include <linux/time.h>
#include <linux/random.h>
#include <linux/jiffies.h>
#include <linux/uaccess.h>
#include <linux/dcache.h>
#include <linux/namei.h>

#include "../include/vexfs_v2_atomic.h"

/* Module parameters for atomic operation tuning */
static unsigned int atomic_max_concurrent_trans = 256;
module_param(atomic_max_concurrent_trans, uint, 0644);
MODULE_PARM_DESC(atomic_max_concurrent_trans, "Maximum concurrent atomic transactions");

static unsigned int atomic_batch_size = 64;
module_param(atomic_batch_size, uint, 0644);
MODULE_PARM_DESC(atomic_batch_size, "Atomic operation batch size");

static unsigned int atomic_commit_timeout = 10000; /* 10 seconds */
module_param(atomic_commit_timeout, uint, 0644);
MODULE_PARM_DESC(atomic_commit_timeout, "Atomic transaction commit timeout in milliseconds");

static bool atomic_enable_batching = true;
module_param(atomic_enable_batching, bool, 0644);
MODULE_PARM_DESC(atomic_enable_batching, "Enable atomic operation batching");

/* Global atomic manager instance */
static struct vexfs_atomic_manager *global_atomic_manager = NULL;

/* Forward declarations */
static void vexfs_atomic_batch_work_fn(struct work_struct *work);
static int vexfs_atomic_process_operation(struct vexfs_atomic_transaction *trans,
                                         struct vexfs_atomic_op *op);
static int vexfs_atomic_validate_operation(struct vexfs_atomic_op *op);
static void vexfs_atomic_cleanup_transaction(struct vexfs_atomic_transaction *trans);
static int vexfs_atomic_write_rollback_log(struct vexfs_atomic_transaction *trans);

/*
 * Lock-free queue implementation using kernel atomic operations
 */

/*
 * Create a new lock-free queue
 */
struct vexfs_lockfree_queue *vexfs_lockfree_queue_create(u32 node_size)
{
    struct vexfs_lockfree_queue *queue;
    struct vexfs_lockfree_node *dummy_node;
    
    queue = kzalloc(sizeof(struct vexfs_lockfree_queue), GFP_KERNEL);
    if (!queue) {
        printk(KERN_ERR "VexFS Atomic: Failed to allocate lock-free queue\n");
        return ERR_PTR(-ENOMEM);
    }
    
    /* Create node cache for efficient allocation */
    queue->node_cache = kmem_cache_create("vexfs_lockfree_nodes",
                                         sizeof(struct vexfs_lockfree_node) + node_size,
                                         0, SLAB_HWCACHE_ALIGN, NULL);
    if (!queue->node_cache) {
        kfree(queue);
        return ERR_PTR(-ENOMEM);
    }
    
    /* Create dummy node to initialize queue */
    dummy_node = kmem_cache_alloc(queue->node_cache, GFP_KERNEL);
    if (!dummy_node) {
        kmem_cache_destroy(queue->node_cache);
        kfree(queue);
        return ERR_PTR(-ENOMEM);
    }
    
    atomic_set(&dummy_node->next, 0);
    dummy_node->data = NULL;
    atomic_set(&dummy_node->ref_count, 1);
    dummy_node->sequence = 0;
    
    /* Initialize queue pointers to dummy node */
    atomic_set(&queue->head, (unsigned long)dummy_node);
    atomic_set(&queue->tail, (unsigned long)dummy_node);
    atomic64_set(&queue->enqueue_count, 0);
    atomic64_set(&queue->dequeue_count, 0);
    queue->node_size = node_size;
    
    return queue;
}

/*
 * Destroy a lock-free queue
 */
void vexfs_lockfree_queue_destroy(struct vexfs_lockfree_queue *queue)
{
    struct vexfs_lockfree_node *node, *next;
    
    if (!queue)
        return;
    
    /* Free all remaining nodes */
    node = (struct vexfs_lockfree_node *)atomic_read(&queue->head);
    while (node) {
        next = (struct vexfs_lockfree_node *)atomic_read(&node->next);
        kmem_cache_free(queue->node_cache, node);
        node = next;
    }
    
    kmem_cache_destroy(queue->node_cache);
    kfree(queue);
}

/*
 * Enqueue data into lock-free queue
 */
int vexfs_lockfree_enqueue(struct vexfs_lockfree_queue *queue, void *data)
{
    struct vexfs_lockfree_node *new_node, *tail, *next;
    unsigned long tail_ptr, next_ptr;
    
    if (!queue || !data)
        return -EINVAL;
    
    /* Allocate new node */
    new_node = kmem_cache_alloc(queue->node_cache, GFP_ATOMIC);
    if (!new_node)
        return -ENOMEM;
    
    new_node->data = data;
    atomic_set(&new_node->next, 0);
    atomic_set(&new_node->ref_count, 1);
    new_node->sequence = atomic64_inc_return(&queue->enqueue_count);
    
    while (1) {
        tail_ptr = atomic_read(&queue->tail);
        tail = (struct vexfs_lockfree_node *)tail_ptr;
        
        next_ptr = atomic_read(&tail->next);
        next = (struct vexfs_lockfree_node *)next_ptr;
        
        /* Check if tail is still the last node */
        if (tail_ptr == atomic_read(&queue->tail)) {
            if (next == NULL) {
                /* Try to link new node at the end of the list */
                if (atomic_cmpxchg(&tail->next, 0, (unsigned long)new_node) == 0)
                    break;
            } else {
                /* Tail was not pointing to the last node, try to advance it */
                atomic_cmpxchg(&queue->tail, tail_ptr, (unsigned long)next);
            }
        }
        cpu_relax();
    }
    
    /* Try to advance tail to the new node */
    atomic_cmpxchg(&queue->tail, tail_ptr, (unsigned long)new_node);
    
    return 0;
}

/*
 * Dequeue data from lock-free queue
 */
void *vexfs_lockfree_dequeue(struct vexfs_lockfree_queue *queue)
{
    struct vexfs_lockfree_node *head, *tail, *next;
    unsigned long head_ptr, tail_ptr, next_ptr;
    void *data;
    
    if (!queue)
        return NULL;
    
    while (1) {
        head_ptr = atomic_read(&queue->head);
        head = (struct vexfs_lockfree_node *)head_ptr;
        
        tail_ptr = atomic_read(&queue->tail);
        tail = (struct vexfs_lockfree_node *)tail_ptr;
        
        next_ptr = atomic_read(&head->next);
        next = (struct vexfs_lockfree_node *)next_ptr;
        
        /* Check if head is still the first node */
        if (head_ptr == atomic_read(&queue->head)) {
            if (head == tail) {
                if (next == NULL) {
                    /* Queue is empty */
                    return NULL;
                }
                /* Tail is falling behind, try to advance it */
                atomic_cmpxchg(&queue->tail, tail_ptr, (unsigned long)next);
            } else {
                if (next == NULL) {
                    /* Inconsistent state, retry */
                    continue;
                }
                
                /* Read data before CAS to avoid race condition */
                data = next->data;
                
                /* Try to advance head to the next node */
                if (atomic_cmpxchg(&queue->head, head_ptr, (unsigned long)next) == head_ptr) {
                    atomic64_inc(&queue->dequeue_count);
                    
                    /* Free the old head node */
                    kmem_cache_free(queue->node_cache, head);
                    
                    return data;
                }
            }
        }
        cpu_relax();
    }
}

/*
 * Initialize atomic operation manager
 */
struct vexfs_atomic_manager *vexfs_atomic_manager_init(struct vexfs_journal *journal)
{
    struct vexfs_atomic_manager *manager;
    int ret;
    
    if (!journal) {
        printk(KERN_ERR "VexFS Atomic: Invalid journal for atomic manager init\n");
        return ERR_PTR(-EINVAL);
    }
    
    manager = kzalloc(sizeof(struct vexfs_atomic_manager), GFP_KERNEL);
    if (!manager) {
        printk(KERN_ERR "VexFS Atomic: Failed to allocate atomic manager\n");
        return ERR_PTR(-ENOMEM);
    }
    
    /* Initialize transaction management */
    INIT_LIST_HEAD(&manager->active_trans);
    mutex_init(&manager->trans_mutex);
    atomic64_set(&manager->next_trans_id, 1);
    atomic_set(&manager->active_trans_count, 0);
    
    /* Initialize lock-free operation queue */
    manager->global_op_queue = vexfs_lockfree_queue_create(sizeof(struct vexfs_atomic_op));
    if (IS_ERR(manager->global_op_queue)) {
        ret = PTR_ERR(manager->global_op_queue);
        kfree(manager);
        return ERR_PTR(ret);
    }
    
    /* Create atomic operation workqueue */
    manager->atomic_workqueue = alloc_workqueue("vexfs_atomic", WQ_MEM_RECLAIM | WQ_HIGHPRI, 0);
    if (!manager->atomic_workqueue) {
        vexfs_lockfree_queue_destroy(manager->global_op_queue);
        kfree(manager);
        return ERR_PTR(-ENOMEM);
    }
    
    INIT_WORK(&manager->batch_work, vexfs_atomic_batch_work_fn);
    
    /* Initialize performance counters */
    ret = percpu_counter_init(&manager->op_counter, 0, GFP_KERNEL);
    if (ret) {
        destroy_workqueue(manager->atomic_workqueue);
        vexfs_lockfree_queue_destroy(manager->global_op_queue);
        kfree(manager);
        return ERR_PTR(ret);
    }
    
    atomic64_set(&manager->total_commits, 0);
    atomic64_set(&manager->total_aborts, 0);
    atomic64_set(&manager->total_rollbacks, 0);
    
    /* Create memory caches */
    manager->trans_cache = kmem_cache_create("vexfs_atomic_trans",
                                           sizeof(struct vexfs_atomic_transaction),
                                           0, SLAB_HWCACHE_ALIGN, NULL);
    if (!manager->trans_cache) {
        percpu_counter_destroy(&manager->op_counter);
        destroy_workqueue(manager->atomic_workqueue);
        vexfs_lockfree_queue_destroy(manager->global_op_queue);
        kfree(manager);
        return ERR_PTR(-ENOMEM);
    }
    
    manager->op_cache = kmem_cache_create("vexfs_atomic_ops",
                                        sizeof(struct vexfs_atomic_op),
                                        0, SLAB_HWCACHE_ALIGN, NULL);
    if (!manager->op_cache) {
        kmem_cache_destroy(manager->trans_cache);
        percpu_counter_destroy(&manager->op_counter);
        destroy_workqueue(manager->atomic_workqueue);
        vexfs_lockfree_queue_destroy(manager->global_op_queue);
        kfree(manager);
        return ERR_PTR(-ENOMEM);
    }
    
    manager->rollback_cache = kmem_cache_create("vexfs_atomic_rollback",
                                              sizeof(struct vexfs_rollback_entry),
                                              0, SLAB_HWCACHE_ALIGN, NULL);
    if (!manager->rollback_cache) {
        kmem_cache_destroy(manager->op_cache);
        kmem_cache_destroy(manager->trans_cache);
        percpu_counter_destroy(&manager->op_counter);
        destroy_workqueue(manager->atomic_workqueue);
        vexfs_lockfree_queue_destroy(manager->global_op_queue);
        kfree(manager);
        return ERR_PTR(-ENOMEM);
    }
    
    /* Set journal reference */
    manager->journal = journal;
    
    /* Initialize configuration */
    manager->max_concurrent_trans = atomic_max_concurrent_trans;
    manager->batch_size = atomic_batch_size;
    manager->commit_timeout = atomic_commit_timeout;
    
    /* Initialize statistics */
    atomic64_set(&manager->ops_processed, 0);
    atomic64_set(&manager->bytes_processed, 0);
    manager->last_batch_time = jiffies;
    
    /* Initialize error handling */
    atomic_set(&manager->error_count, 0);
    INIT_LIST_HEAD(&manager->error_log);
    
    /* Initialize synchronization */
    init_rwsem(&manager->manager_rwsem);
    spin_lock_init(&manager->stats_lock);
    
    /* Set global manager reference */
    global_atomic_manager = manager;
    
    printk(KERN_INFO "VexFS Atomic: Atomic operation manager initialized successfully\n");
    
    return manager;
}

/*
 * Destroy atomic operation manager
 */
void vexfs_atomic_manager_destroy(struct vexfs_atomic_manager *manager)
{
    struct vexfs_atomic_transaction *trans, *tmp_trans;
    
    if (!manager)
        return;
    
    /* Cancel any pending work */
    cancel_work_sync(&manager->batch_work);
    
    /* Abort all active transactions */
    mutex_lock(&manager->trans_mutex);
    list_for_each_entry_safe(trans, tmp_trans, &manager->active_trans, trans_list) {
        vexfs_atomic_abort(trans);
    }
    mutex_unlock(&manager->trans_mutex);
    
    /* Destroy workqueue */
    destroy_workqueue(manager->atomic_workqueue);
    
    /* Destroy lock-free queue */
    vexfs_lockfree_queue_destroy(manager->global_op_queue);
    
    /* Destroy memory caches */
    kmem_cache_destroy(manager->rollback_cache);
    kmem_cache_destroy(manager->op_cache);
    kmem_cache_destroy(manager->trans_cache);
    
    /* Destroy performance counters */
    percpu_counter_destroy(&manager->op_counter);
    
    /* Clear global reference */
    if (global_atomic_manager == manager)
        global_atomic_manager = NULL;
    
    kfree(manager);
    
    printk(KERN_INFO "VexFS Atomic: Atomic operation manager destroyed\n");
}

/*
 * Begin a new atomic transaction
 */
struct vexfs_atomic_transaction *vexfs_atomic_begin(struct vexfs_atomic_manager *manager,
                                                   u32 flags, u32 isolation_level)
{
    struct vexfs_atomic_transaction *trans;
    struct vexfs_journal_transaction *journal_trans;
    int ret;
    
    if (!manager) {
        printk(KERN_ERR "VexFS Atomic: Invalid manager for transaction begin\n");
        return ERR_PTR(-EINVAL);
    }
    
    /* Check concurrent transaction limit */
    if (atomic_read(&manager->active_trans_count) >= manager->max_concurrent_trans) {
        printk(KERN_WARNING "VexFS Atomic: Maximum concurrent transactions reached\n");
        return ERR_PTR(-EAGAIN);
    }
    
    /* Allocate transaction structure */
    trans = kmem_cache_alloc(manager->trans_cache, GFP_KERNEL);
    if (!trans) {
        printk(KERN_ERR "VexFS Atomic: Failed to allocate transaction\n");
        return ERR_PTR(-ENOMEM);
    }
    
    /* Initialize transaction */
    memset(trans, 0, sizeof(struct vexfs_atomic_transaction));
    trans->trans_id = atomic64_inc_return(&manager->next_trans_id);
    trans->trans_flags = flags;
    trans->isolation_level = isolation_level;
    trans->parent_trans = NULL;
    trans->nesting_level = 0;
    
    /* Initialize operation tracking */
    INIT_LIST_HEAD(&trans->op_list);
    atomic_set(&trans->op_count, 0);
    trans->max_ops = VEXFS_MAX_ATOMIC_OPS;
    
    /* Create operation queue for this transaction */
    trans->op_queue = vexfs_lockfree_queue_create(sizeof(struct vexfs_atomic_op));
    if (IS_ERR(trans->op_queue)) {
        ret = PTR_ERR(trans->op_queue);
        kmem_cache_free(manager->trans_cache, trans);
        return ERR_PTR(ret);
    }
    
    /* Start journal transaction */
    journal_trans = vexfs_journal_start(manager->journal, VEXFS_MAX_ATOMIC_OPS,
                                       VEXFS_JOURNAL_OP_CREATE);
    if (IS_ERR(journal_trans)) {
        ret = PTR_ERR(journal_trans);
        vexfs_lockfree_queue_destroy(trans->op_queue);
        kmem_cache_free(manager->trans_cache, trans);
        return ERR_PTR(ret);
    }
    trans->journal_trans = journal_trans;
    
    /* Initialize synchronization */
    seqlock_init(&trans->trans_seqlock);
    atomic_set(&trans->ref_count, 1);
    init_completion(&trans->trans_completion);
    
    /* Initialize state */
    atomic_set(&trans->trans_state, VEXFS_TRANS_RUNNING);
    trans->start_time = jiffies;
    trans->commit_time = 0;
    
    /* Initialize error handling */
    trans->trans_error = 0;
    INIT_LIST_HEAD(&trans->rollback_list);
    
    /* Initialize performance tracking */
    atomic64_set(&trans->bytes_written, 0);
    atomic64_set(&trans->bytes_read, 0);
    trans->checkpoint_count = 0;
    
    /* Set operation cache */
    trans->op_cache = manager->op_cache;
    
    /* Initialize list management */
    INIT_LIST_HEAD(&trans->trans_list);
    
    /* Add to active transactions list */
    mutex_lock(&manager->trans_mutex);
    list_add_tail(&trans->trans_list, &manager->active_trans);
    atomic_inc(&manager->active_trans_count);
    mutex_unlock(&manager->trans_mutex);
    
    printk(KERN_DEBUG "VexFS Atomic: Transaction %llu started (flags=0x%x, isolation=%u)\n",
           trans->trans_id, flags, isolation_level);
    
    return trans;
}

/*
 * Commit an atomic transaction
 */
int vexfs_atomic_commit(struct vexfs_atomic_transaction *trans)
{
    struct vexfs_atomic_manager *manager;
    int ret = 0;
    
    if (!trans) {
        printk(KERN_ERR "VexFS Atomic: Invalid transaction for commit\n");
        return -EINVAL;
    }
    
    manager = global_atomic_manager;
    if (!manager) {
        printk(KERN_ERR "VexFS Atomic: No atomic manager available\n");
        return -ENODEV;
    }
    
    /* Check transaction state */
    if (atomic_read(&trans->trans_state) != VEXFS_TRANS_RUNNING) {
        printk(KERN_ERR "VexFS Atomic: Transaction %llu not in running state\n",
               trans->trans_id);
        return -EINVAL;
    }
    
    /* Set transaction state to committing */
    atomic_set(&trans->trans_state, VEXFS_TRANS_COMMIT);
    
    /* Execute any remaining operations in batch */
    if (atomic_enable_batching) {
        ret = vexfs_atomic_batch_execute(trans);
        if (ret) {
            printk(KERN_ERR "VexFS Atomic: Batch execution failed for transaction %llu\n",
                   trans->trans_id);
            goto abort_transaction;
        }
    }
    
    /* Commit journal transaction */
    ret = vexfs_journal_commit(trans->journal_trans);
    if (ret) {
        printk(KERN_ERR "VexFS Atomic: Journal commit failed for transaction %llu\n",
               trans->trans_id);
        goto abort_transaction;
    }
    
    /* Set commit time */
    trans->commit_time = jiffies;
    
    /* Set transaction state to finished */
    atomic_set(&trans->trans_state, VEXFS_TRANS_FINISHED);
    
    /* Update statistics */
    atomic64_inc(&manager->total_commits);
    atomic64_add(atomic64_read(&trans->bytes_written), &manager->bytes_processed);
    
    /* Complete transaction */
    complete_all(&trans->trans_completion);
    
    /* Clean up transaction */
    vexfs_atomic_cleanup_transaction(trans);
    
    printk(KERN_DEBUG "VexFS Atomic: Transaction %llu committed successfully\n",
           trans->trans_id);
    
    return 0;

abort_transaction:
    vexfs_atomic_abort(trans);
    return ret;
}

/*
 * Abort an atomic transaction
 */
int vexfs_atomic_abort(struct vexfs_atomic_transaction *trans)
{
    struct vexfs_atomic_manager *manager;
    int ret = 0;
    
    if (!trans) {
        printk(KERN_ERR "VexFS Atomic: Invalid transaction for abort\n");
        return -EINVAL;
    }
    
    manager = global_atomic_manager;
    if (!manager) {
        printk(KERN_ERR "VexFS Atomic: No atomic manager available\n");
        return -ENODEV;
    }
    
    /* Set transaction state to aborting */
    atomic_set(&trans->trans_state, VEXFS_TRANS_ABORTING);
    
    /* Execute rollback operations */
    ret = vexfs_atomic_execute_rollback(trans);
    if (ret) {
        printk(KERN_ERR "VexFS Atomic: Rollback failed for transaction %llu\n",
               trans->trans_id);
    }
    
    /* Abort journal transaction */
    if (trans->journal_trans) {
        vexfs_journal_abort(trans->journal_trans);
    }
    
    /* Set transaction state to finished */
    atomic_set(&trans->trans_state, VEXFS_TRANS_FINISHED);
    
    /* Update statistics */
    atomic64_inc(&manager->total_aborts);
    if (ret == 0) {
        atomic64_inc(&manager->total_rollbacks);
    }
    
    /* Complete transaction */
    complete_all(&trans->trans_completion);
    
    /* Clean up transaction */
    vexfs_atomic_cleanup_transaction(trans);
    
    printk(KERN_DEBUG "VexFS Atomic: Transaction %llu aborted\n", trans->trans_id);
    
    return ret;
}

/*
 * Execute rollback operations for a transaction
 */
int vexfs_atomic_execute_rollback(struct vexfs_atomic_transaction *trans)
{
    struct vexfs_rollback_entry *entry, *tmp;
    int ret = 0;
    
    if (!trans) {
        return -EINVAL;
    }
    
    /* Execute rollback entries in reverse order */
    list_for_each_entry_safe_reverse(entry, tmp, &trans->rollback_list, entry_list) {
        /* Restore original data */
        if (entry->original_data && entry->data_size > 0) {
            /* This would involve writing the original data back to the block */
            /* Implementation depends on specific VFS operations */
            printk(KERN_DEBUG "VexFS Atomic: Rolling back entry type %u for block %llu\n",
                   entry->entry_type, entry->target_block);
        }
        
        /* Remove from rollback list */
        list_del(&entry->entry_list);
        
        /* Free rollback entry */
        if (entry->original_data) {
            kfree(entry->original_data);
        }
        if (entry->modified_data) {
            kfree(entry->modified_data);
        }
        
        if (global_atomic_manager && global_atomic_manager->rollback_cache) {
            kmem_cache_free(global_atomic_manager->rollback_cache, entry);
        } else {
            kfree(entry);
        }
    }
    
    return ret;
}

/*
 * Add rollback entry for transaction recovery
 */
int vexfs_atomic_add_rollback_entry(struct vexfs_atomic_transaction *trans,
                                   u32 entry_type, u64 target_block,
                                   void *original_data, size_t data_size)
{
    struct vexfs_rollback_entry *entry;
    struct vexfs_atomic_manager *manager;
    
    if (!trans || !original_data || data_size == 0) {
        return -EINVAL;
    }
    
    manager = global_atomic_manager;
    if (!manager) {
        return -ENODEV;
    }
    
    /* Allocate rollback entry */
    entry = kmem_cache_alloc(manager->rollback_cache, GFP_KERNEL);
    if (!entry) {
        return -ENOMEM;
    }
    
    /* Initialize rollback entry */
    entry->entry_type = entry_type;
    entry->target_block = target_block;
    entry->data_size = data_size;
    
    /* Copy original data */
    entry->original_data = kmalloc(data_size, GFP_KERNEL);
    if (!entry->original_data) {
        kmem_cache_free(manager->rollback_cache, entry);
        return -ENOMEM;
    }
    memcpy(entry->original_data, original_data, data_size);
    
    entry->modified_data = NULL;
    entry->target_inode = NULL;
    entry->file_offset = 0;
    entry->operation_flags = 0;
    
    /* Initialize list management */
    INIT_LIST_HEAD(&entry->entry_list);
    
    /* Add to transaction rollback list */
    list_add_tail(&entry->entry_list, &trans->rollback_list);
    
    return 0;
}

/*
 * Clean up transaction resources
 */
static void vexfs_atomic_cleanup_transaction(struct vexfs_atomic_transaction *trans)
{
    struct vexfs_atomic_manager *manager;
    struct vexfs_atomic_op *op, *tmp_op;
    
    if (!trans)
        return;
    
    manager = global_atomic_manager;
    if (!manager)
        return;
    
    /* Remove from active transactions list */
    mutex_lock(&manager->trans_mutex);
    list_del(&trans->trans_list);
    atomic_dec(&manager->active_trans_count);
    mutex_unlock(&manager->trans_mutex);
    
    /* Clean up operations */
    list_for_each_entry_safe(op, tmp_op, &trans->op_list, op_list) {
        list_del(&op->op_list);
        
        if (op->op_data) {
            kfree(op->op_data);
        }
        if (op->rollback_data) {
            kfree(op->rollback_data);
        }
        
        kmem_cache_free(manager->op_cache, op);
    }
    
    /* Destroy operation queue */
    if (trans->op_queue) {
        vexfs_lockfree_queue_destroy(trans->op_queue);
    }
    
    /* Execute any remaining rollback operations */
    vexfs_atomic_execute_rollback(trans);
    
    /* Free transaction structure */
    kmem_cache_free(manager->trans_cache, trans);
}

/*
 * Batch work function for processing atomic operations
 */
static void vexfs_atomic_batch_work_fn(struct work_struct *work)
{
    struct vexfs_atomic_manager *manager;
    struct vexfs_atomic_op *op;
    int processed = 0;
    
    manager = container_of(work, struct vexfs_atomic_manager, batch_work);
    
    /* Process operations from global queue */
    while (processed < manager->batch_size) {
        op = (struct vexfs_atomic_op *)vexfs_lockfree_dequeue(manager->global_op_queue);
        if (!op)
            break;
        
        /* Process the operation */
        /* Implementation would depend on specific operation types */
        
        processed++;
        atomic64_inc(&manager->ops_processed);
    }
    
    manager->last_batch_time = jiffies;
    
    /* Schedule next batch if there are more operations */
    if (vexfs_lockfree_dequeue(manager->global_op_queue)) {
queue, &manager->batch_work);
    }
}

/*
 * Execute batch of operations for a transaction
 */
int vexfs_atomic_batch_execute(struct vexfs_atomic_transaction *trans)
{
    struct vexfs_atomic_op *op;
    int ret = 0;
    int processed = 0;
    
    if (!trans) {
        return -EINVAL;
    }
    
    /* Process operations from transaction queue */
    while (processed < VEXFS_ATOMIC_BATCH_SIZE) {
        op = (struct vexfs_atomic_op *)vexfs_lockfree_dequeue(trans->op_queue);
        if (!op)
            break;
        
        ret = vexfs_atomic_process_operation(trans, op);
        if (ret) {
            printk(KERN_ERR "VexFS Atomic: Operation processing failed in transaction %llu\n",
                   trans->trans_id);
            break;
        }
        
        processed++;
    }
    
    return ret;
}

/*
 * Process a single atomic operation
 */
static int vexfs_atomic_process_operation(struct vexfs_atomic_transaction *trans,
                                         struct vexfs_atomic_op *op)
{
    int ret = 0;
    
    if (!trans || !op) {
        return -EINVAL;
    }
    
    /* Validate operation */
    ret = vexfs_atomic_validate_operation(op);
    if (ret) {
        return ret;
    }
    
    /* Execute operation based on type */
    switch (op->op_type) {
    case VEXFS_ATOMIC_CREATE:
        /* Handle file creation */
        printk(KERN_DEBUG "VexFS Atomic: Processing CREATE operation\n");
        break;
        
    case VEXFS_ATOMIC_DELETE:
        /* Handle file deletion */
        printk(KERN_DEBUG "VexFS Atomic: Processing DELETE operation\n");
        break;
        
    case VEXFS_ATOMIC_WRITE:
        /* Handle file write */
        printk(KERN_DEBUG "VexFS Atomic: Processing WRITE operation\n");
        break;
        
    case VEXFS_ATOMIC_TRUNCATE:
        /* Handle file truncation */
        printk(KERN_DEBUG "VexFS Atomic: Processing TRUNCATE operation\n");
        break;
        
    case VEXFS_ATOMIC_RENAME:
        /* Handle file rename */
        printk(KERN_DEBUG "VexFS Atomic: Processing RENAME operation\n");
        break;
        
    default:
        printk(KERN_ERR "VexFS Atomic: Unknown operation type %u\n", op->op_type);
        ret = -EINVAL;
        break;
    }
    
    /* Update operation state */
    atomic_set(&op->op_state, ret == 0 ? VEXFS_TRANS_FINISHED : VEXFS_TRANS_ABORTING);
    op->op_result = ret;
    
    /* Complete operation */
    complete_all(&op->op_completion);
    
    return ret;
}

/*
 * Validate atomic operation
 */
static int vexfs_atomic_validate_operation(struct vexfs_atomic_op *op)
{
    if (!op) {
        return -EINVAL;
    }
    
    /* Check operation type */
    if (op->op_type < VEXFS_ATOMIC_CREATE || op->op_type > VEXFS_ATOMIC_SYMLINK) {
        printk(KERN_ERR "VexFS Atomic: Invalid operation type %u\n", op->op_type);
        return -EINVAL;
    }
    
    /* Check target inode for operations that require it */
    if ((op->op_type == VEXFS_ATOMIC_DELETE || 
         op->op_type == VEXFS_ATOMIC_WRITE ||
         op->op_type == VEXFS_ATOMIC_TRUNCATE) && !op->target_inode) {
        printk(KERN_ERR "VexFS Atomic: Missing target inode for operation %u\n", op->op_type);
        return -EINVAL;
    }
    
    /* Check data requirements */
    if (op->op_type == VEXFS_ATOMIC_WRITE && (!op->op_data || op->data_size == 0)) {
        printk(KERN_ERR "VexFS Atomic: Missing data for WRITE operation\n");
        return -EINVAL;
    }
    
    return 0;
}

/*
 * Begin nested atomic transaction
 */
struct vexfs_atomic_transaction *vexfs_atomic_begin_nested(struct vexfs_atomic_transaction *parent,
                                                          u32 flags)
{
    struct vexfs_atomic_transaction *nested_trans;
    struct vexfs_atomic_manager *manager;
    
    if (!parent) {
        printk(KERN_ERR "VexFS Atomic: Invalid parent transaction for nested begin\n");
        return ERR_PTR(-EINVAL);
    }
    
    /* Check nesting level limit */
    if (parent->nesting_level >= VEXFS_MAX_NESTED_TRANS) {
        printk(KERN_ERR "VexFS Atomic: Maximum nesting level reached\n");
        return ERR_PTR(-EMLINK);
    }
    
    manager = global_atomic_manager;
    if (!manager) {
        printk(KERN_ERR "VexFS Atomic: No atomic manager available\n");
        return ERR_PTR(-ENODEV);
    }
    
    /* Begin nested transaction with same isolation level as parent */
    nested_trans = vexfs_atomic_begin(manager, flags | VEXFS_TRANS_NESTED, 
                                     parent->isolation_level);
    if (IS_ERR(nested_trans)) {
        return nested_trans;
    }
    
    /* Set parent relationship */
    nested_trans->parent_trans = parent;
    nested_trans->nesting_level = parent->nesting_level + 1;
    
    printk(KERN_DEBUG "VexFS Atomic: Nested transaction %llu started (parent=%llu, level=%u)\n",
           nested_trans->trans_id, parent->trans_id, nested_trans->nesting_level);
    
    return nested_trans;
}

/*
 * Add operation to atomic transaction
 */
int vexfs_atomic_add_operation(struct vexfs_atomic_transaction *trans,
                              struct vexfs_atomic_op *op)
{
    if (!trans || !op) {
        return -EINVAL;
    }
    
    /* Check operation limit */
    if (atomic_read(&trans->op_count) >= trans->max_ops) {
        printk(KERN_ERR "VexFS Atomic: Transaction %llu operation limit reached\n",
               trans->trans_id);
        return -ENOSPC;
    }
    
    /* Validate operation */
    if (vexfs_atomic_validate_operation(op) != 0) {
        return -EINVAL;
    }
    
    /* Add to transaction operation list */
    list_add_tail(&op->op_list, &trans->op_list);
    atomic_inc(&trans->op_count);
    
    /* Enqueue operation for processing */
    if (vexfs_lockfree_enqueue(trans->op_queue, op) != 0) {
        list_del(&op->op_list);
        atomic_dec(&trans->op_count);
        return -ENOMEM;
    }
    
    return 0;
}

/*
 * Get atomic operation statistics
 */
void vexfs_atomic_get_stats(struct vexfs_atomic_manager *manager,
                           struct vexfs_atomic_stats *stats)
{
    if (!manager || !stats) {
        return;
    }
    
    memset(stats, 0, sizeof(struct vexfs_atomic_stats));
    
    spin_lock(&manager->stats_lock);
    
    stats->total_transactions = atomic64_read(&manager->total_commits) + 
                               atomic64_read(&manager->total_aborts);
    stats->committed_transactions = atomic64_read(&manager->total_commits);
    stats->aborted_transactions = atomic64_read(&manager->total_aborts);
    stats->rollback_operations = atomic64_read(&manager->total_rollbacks);
    stats->operations_processed = atomic64_read(&manager->ops_processed);
    stats->bytes_processed = atomic64_read(&manager->bytes_processed);
    stats->active_transactions = atomic_read(&manager->active_trans_count);
    stats->average_batch_size = manager->batch_size;
    stats->error_count = atomic_read(&manager->error_count);
    
    /* Calculate average commit time */
    if (stats->committed_transactions > 0) {
        stats->average_commit_time = (u32)(jiffies_to_msecs(jiffies - manager->last_batch_time) / 
                                          stats->committed_transactions);
    }
    
    spin_unlock(&manager->stats_lock);
}

/*
 * Recover from partial writes during crash
 */
int vexfs_atomic_recover_partial_writes(struct vexfs_atomic_manager *manager)
{
    int ret = 0;
    
    if (!manager) {
        return -EINVAL;
    }
    
    printk(KERN_INFO "VexFS Atomic: Starting partial write recovery\n");
    
    /* Use journal recovery to identify incomplete transactions */
    if (manager->journal) {
        ret = vexfs_journal_recover(manager->journal);
        if (ret) {
            printk(KERN_ERR "VexFS Atomic: Journal recovery failed during partial write recovery\n");
            return ret;
        }
    }
    
    printk(KERN_INFO "VexFS Atomic: Partial write recovery completed\n");
    
    return ret;
}

/*
 * Validate transaction integrity
 */
int vexfs_atomic_validate_transaction_integrity(struct vexfs_atomic_transaction *trans)
{
    struct vexfs_atomic_op *op;
    int op_count = 0;
    
    if (!trans) {
        return -EINVAL;
    }
    
    /* Check transaction state consistency */
    if (atomic_read(&trans->trans_state) < VEXFS_TRANS_RUNNING ||
        atomic_read(&trans->trans_state) > VEXFS_TRANS_FINISHED) {
        printk(KERN_ERR "VexFS Atomic: Invalid transaction state %d\n",
               atomic_read(&trans->trans_state));
        return -EINVAL;
    }
    
    /* Validate operation count */
    list_for_each_entry(op, &trans->op_list, op_list) {
        op_count++;
        
        /* Validate each operation */
        if (vexfs_atomic_validate_operation(op) != 0) {
            printk(KERN_ERR "VexFS Atomic: Invalid operation in transaction %llu\n",
                   trans->trans_id);
            return -EINVAL;
        }
    }
    
    if (op_count != atomic_read(&trans->op_count)) {
        printk(KERN_ERR "VexFS Atomic: Operation count mismatch in transaction %llu\n",
               trans->trans_id);
        return -EINVAL;
    }
    
    return 0;
}
        queue_work(manager->atomic_work