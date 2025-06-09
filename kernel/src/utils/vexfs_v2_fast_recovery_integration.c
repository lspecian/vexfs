/*
 * VexFS v2.0 - Fast Recovery Integration (Task 7)
 * 
 * This file integrates the fast recovery system with the main VexFS kernel module,
 * providing seamless integration with the complete Phase 1 foundation and ensuring
 * the recovery system is properly initialized and available for crash recovery.
 */

#include "../include/vexfs_v2_fast_recovery.h"
#include "../include/vexfs_v2_internal.h"

/* Global fast recovery manager instance */
static struct vexfs_fast_recovery_manager *global_recovery_mgr = NULL;
static DEFINE_MUTEX(recovery_mgr_mutex);

/*
 * Initialize fast recovery for a VexFS superblock
 */
int vexfs_init_fast_recovery(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_fast_recovery_manager *recovery_mgr;
    int ret;

    if (!sbi) {
        printk(KERN_ERR "VexFS: Invalid superblock info for fast recovery init\n");
        return -EINVAL;
    }

    /* Check if recovery manager already exists */
    mutex_lock(&recovery_mgr_mutex);
    if (global_recovery_mgr) {
        sbi->recovery_mgr = global_recovery_mgr;
        mutex_unlock(&recovery_mgr_mutex);
        return 0;
    }

    /* Initialize fast recovery manager */
    recovery_mgr = vexfs_fast_recovery_init(
        sbi->journal,           /* Journal from Task 1 */
        sbi->atomic_mgr,        /* Atomic manager from Task 2 */
        sbi->meta_mgr,          /* Metadata journal from Task 3 */
        sbi->alloc_mgr          /* Allocation journal from Task 5 */
    );

    if (IS_ERR(recovery_mgr)) {
        ret = PTR_ERR(recovery_mgr);
        printk(KERN_ERR "VexFS: Failed to initialize fast recovery manager: %d\n", ret);
        mutex_unlock(&recovery_mgr_mutex);
        return ret;
    }

    /* Store in superblock info and global reference */
    sbi->recovery_mgr = recovery_mgr;
    global_recovery_mgr = recovery_mgr;
    
    mutex_unlock(&recovery_mgr_mutex);

    printk(KERN_INFO "VexFS: Fast recovery system initialized successfully\n");
    return 0;
}

/*
 * Cleanup fast recovery for a VexFS superblock
 */
void vexfs_cleanup_fast_recovery(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);

    if (!sbi || !sbi->recovery_mgr)
        return;

    mutex_lock(&recovery_mgr_mutex);
    
    /* Only destroy if this is the last reference */
    if (sbi->recovery_mgr == global_recovery_mgr) {
        vexfs_fast_recovery_destroy(global_recovery_mgr);
        global_recovery_mgr = NULL;
    }
    
    sbi->recovery_mgr = NULL;
    mutex_unlock(&recovery_mgr_mutex);

    printk(KERN_INFO "VexFS: Fast recovery system cleaned up\n");
}

/*
 * Perform fast recovery on mount
 */
int vexfs_perform_fast_recovery(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_fast_recovery_manager *recovery_mgr;
    u32 recovery_flags;
    int ret;

    if (!sbi || !sbi->recovery_mgr) {
        printk(KERN_ERR "VexFS: No recovery manager available for fast recovery\n");
        return -EINVAL;
    }

    recovery_mgr = sbi->recovery_mgr;

    /* Determine recovery flags based on mount options and system state */
    recovery_flags = VEXFS_RECOVERY_FLAG_PROGRESS | VEXFS_RECOVERY_FLAG_CHECKPOINT;
    
    /* Enable parallel recovery if we have multiple cores */
    if (num_online_cpus() > 1) {
        recovery_flags |= VEXFS_RECOVERY_FLAG_PARALLEL;
    }

    /* Enable memory-mapped I/O for large journals */
    if (sbi->journal && sbi->journal->j_total_blocks > 16384) { /* > 64MB */
        recovery_flags |= VEXFS_RECOVERY_FLAG_MMAP_IO;
    }

    printk(KERN_INFO "VexFS: Starting fast crash recovery (flags=0x%x)\n", recovery_flags);

    /* Perform the actual recovery */
    ret = vexfs_fast_recovery_start(recovery_mgr, recovery_flags);
    if (ret) {
        printk(KERN_ERR "VexFS: Fast recovery failed: %d\n", ret);
        return ret;
    }

    /* Create a checkpoint after successful recovery */
    ret = vexfs_fast_recovery_create_checkpoint(recovery_mgr,
                                               VEXFS_CHECKPOINT_TYPE_FULL,
                                               VEXFS_RECOVERY_FLAG_CHECKPOINT);
    if (ret) {
        printk(KERN_WARNING "VexFS: Failed to create post-recovery checkpoint: %d\n", ret);
        /* Don't fail the mount for checkpoint creation failure */
    }

    printk(KERN_INFO "VexFS: Fast crash recovery completed successfully\n");
    return 0;
}

/*
 * Create a checkpoint (called periodically or on demand)
 */
int vexfs_create_recovery_checkpoint(struct super_block *sb, u32 checkpoint_type)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_fast_recovery_manager *recovery_mgr;
    u32 flags = VEXFS_RECOVERY_FLAG_CHECKPOINT;
    int ret;

    if (!sbi || !sbi->recovery_mgr) {
        return -EINVAL;
    }

    recovery_mgr = sbi->recovery_mgr;

    ret = vexfs_fast_recovery_create_checkpoint(recovery_mgr, checkpoint_type, flags);
    if (ret) {
        printk(KERN_ERR "VexFS: Failed to create checkpoint (type %u): %d\n",
               checkpoint_type, ret);
        return ret;
    }

    printk(KERN_DEBUG "VexFS: Created checkpoint (type %u)\n", checkpoint_type);
    return 0;
}

/*
 * Get recovery statistics (for monitoring and debugging)
 */
int vexfs_get_recovery_stats(struct super_block *sb, struct vexfs_fast_recovery_stats *stats)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_fast_recovery_manager *recovery_mgr;

    if (!sbi || !sbi->recovery_mgr || !stats) {
        return -EINVAL;
    }

    recovery_mgr = sbi->recovery_mgr;
    vexfs_fast_recovery_get_stats(recovery_mgr, stats);

    return 0;
}

/*
 * Check if recovery is needed (called during mount)
 */
bool vexfs_recovery_needed(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_journal *journal;

    if (!sbi || !sbi->journal) {
        return false;
    }

    journal = sbi->journal;

    /* Recovery is needed if journal head != tail (indicating unprocessed entries) */
    if (journal->j_head != journal->j_tail) {
        printk(KERN_INFO "VexFS: Recovery needed - journal head=%llu, tail=%llu\n",
               journal->j_head, journal->j_tail);
        return true;
    }

    /* Check if journal was not cleanly unmounted */
    if (journal->j_flags & VEXFS_JOURNAL_RECOVERING) {
        printk(KERN_INFO "VexFS: Recovery needed - journal in recovery state\n");
        return true;
    }

    return false;
}

/*
 * Emergency recovery (for critical situations)
 */
int vexfs_emergency_recovery(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_fast_recovery_manager *recovery_mgr;
    u32 emergency_flags;
    int ret;

    if (!sbi || !sbi->recovery_mgr) {
        printk(KERN_ERR "VexFS: No recovery manager for emergency recovery\n");
        return -EINVAL;
    }

    recovery_mgr = sbi->recovery_mgr;

    /* Emergency recovery flags - prioritize speed and basic consistency */
    emergency_flags = VEXFS_RECOVERY_FLAG_PROGRESS | 
                     VEXFS_RECOVERY_FLAG_FORCE_SYNC |
                     VEXFS_RECOVERY_FLAG_PARALLEL;

    printk(KERN_WARNING "VexFS: Starting emergency recovery\n");

    ret = vexfs_fast_recovery_start(recovery_mgr, emergency_flags);
    if (ret) {
        printk(KERN_ERR "VexFS: Emergency recovery failed: %d\n", ret);
        return ret;
    }

    printk(KERN_INFO "VexFS: Emergency recovery completed\n");
    return 0;
}

/*
 * Periodic checkpoint creation (called from timer or workqueue)
 */
static void vexfs_periodic_checkpoint_work(struct work_struct *work)
{
    struct vexfs_sb_info *sbi = container_of(work, struct vexfs_sb_info, 
                                            checkpoint_work.work);
    int ret;

    if (!sbi || !sbi->recovery_mgr) {
        return;
    }

    /* Create incremental checkpoint */
    ret = vexfs_create_recovery_checkpoint(sbi->sb, VEXFS_CHECKPOINT_TYPE_INCREMENTAL);
    if (ret) {
        printk(KERN_WARNING "VexFS: Periodic checkpoint creation failed: %d\n", ret);
    }

    /* Reschedule for next checkpoint */
    if (sbi->recovery_mgr) {
        queue_delayed_work(system_wq, &sbi->checkpoint_work,
                          msecs_to_jiffies(sbi->recovery_mgr->checkpoint_interval * 1000));
    }
}

/*
 * Start periodic checkpointing
 */
int vexfs_start_periodic_checkpoints(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);

    if (!sbi || !sbi->recovery_mgr) {
        return -EINVAL;
    }

    INIT_DELAYED_WORK(&sbi->checkpoint_work, vexfs_periodic_checkpoint_work);
    
    /* Schedule first checkpoint */
    queue_delayed_work(system_wq, &sbi->checkpoint_work,
                      msecs_to_jiffies(sbi->recovery_mgr->checkpoint_interval * 1000));

    printk(KERN_INFO "VexFS: Periodic checkpointing started (interval: %u seconds)\n",
           sbi->recovery_mgr->checkpoint_interval);

    return 0;
}

/*
 * Stop periodic checkpointing
 */
void vexfs_stop_periodic_checkpoints(struct super_block *sb)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);

    if (!sbi) {
        return;
    }

    cancel_delayed_work_sync(&sbi->checkpoint_work);
    printk(KERN_INFO "VexFS: Periodic checkpointing stopped\n");
}

/*
 * Recovery status for administrative monitoring
 */
int vexfs_get_recovery_status(struct super_block *sb, char *buffer, size_t buffer_size)
{
    struct vexfs_sb_info *sbi = VEXFS_SB(sb);
    struct vexfs_fast_recovery_manager *recovery_mgr;
    struct vexfs_fast_recovery_stats stats;
    int len = 0;

    if (!sbi || !sbi->recovery_mgr || !buffer) {
        return -EINVAL;
    }

    recovery_mgr = sbi->recovery_mgr;
    vexfs_fast_recovery_get_stats(recovery_mgr, &stats);

    len += snprintf(buffer + len, buffer_size - len,
                   "VexFS Fast Recovery Status:\n");
    len += snprintf(buffer + len, buffer_size - len,
                   "  Total Recoveries: %llu\n", stats.total_recoveries);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Average Recovery Time: %llu ms\n", stats.average_recovery_time_ms);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Fastest Recovery: %llu ms\n", stats.fastest_recovery_ms);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Slowest Recovery: %llu ms\n", stats.slowest_recovery_ms);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Checkpoints Created: %llu\n", stats.checkpoints_created);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Journal Entries Replayed: %llu\n", stats.journal_entries_replayed);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Partial Transactions Resolved: %llu\n", stats.partial_transactions_resolved);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Current Checkpoints: %u\n", stats.current_checkpoint_count);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Memory Mapped Regions: %u\n", stats.current_mmap_regions);
    len += snprintf(buffer + len, buffer_size - len,
                   "  Error Count: %u\n", stats.error_count);

    return len;
}

/*
 * Module initialization for fast recovery
 */
int __init vexfs_fast_recovery_module_init(void)
{
    printk(KERN_INFO "VexFS: Fast Recovery module initialized\n");
    return 0;
}

/*
 * Module cleanup for fast recovery
 */
void __exit vexfs_fast_recovery_module_exit(void)
{
    mutex_lock(&recovery_mgr_mutex);
    if (global_recovery_mgr) {
        vexfs_fast_recovery_destroy(global_recovery_mgr);
        global_recovery_mgr = NULL;
    }
    mutex_unlock(&recovery_mgr_mutex);
    
    printk(KERN_INFO "VexFS: Fast Recovery module cleaned up\n");
}

/* Export symbols for use by other VexFS modules */
EXPORT_SYMBOL(vexfs_init_fast_recovery);
EXPORT_SYMBOL(vexfs_cleanup_fast_recovery);
EXPORT_SYMBOL(vexfs_perform_fast_recovery);
EXPORT_SYMBOL(vexfs_create_recovery_checkpoint);
EXPORT_SYMBOL(vexfs_get_recovery_stats);
EXPORT_SYMBOL(vexfs_recovery_needed);
EXPORT_SYMBOL(vexfs_emergency_recovery);
EXPORT_SYMBOL(vexfs_start_periodic_checkpoints);
EXPORT_SYMBOL(vexfs_stop_periodic_checkpoints);
EXPORT_SYMBOL(vexfs_get_recovery_status);