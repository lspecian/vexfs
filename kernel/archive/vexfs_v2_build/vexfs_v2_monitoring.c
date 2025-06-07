/**
 * VexFS v2.0 Performance Monitoring Implementation
 * 
 * Specialized monitoring for batch insert operations and vector performance
 * tracking to ensure continued achievement of 100K+ ops/sec targets.
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/atomic.h>
#include <linux/time.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/string.h>
#include "vexfs_v2_monitoring.h"

/* Global monitoring instances */
struct vexfs_batch_insert_metrics vexfs_batch_metrics;
struct vexfs_vector_metrics vexfs_vector_metrics;
unsigned int vexfs_monitoring_flags = VEXFS_MONITORING_ENABLED | 
                                     VEXFS_DETAILED_TIMING_ENABLED |
                                     VEXFS_MEMORY_TRACKING_ENABLED |
                                     VEXFS_REGRESSION_DETECTION_ENABLED;

/* Proc filesystem entries */
struct proc_dir_entry *vexfs_proc_dir = NULL;
struct proc_dir_entry *vexfs_batch_metrics_proc = NULL;
struct proc_dir_entry *vexfs_vector_metrics_proc = NULL;
struct proc_dir_entry *vexfs_performance_summary_proc = NULL;

/* Performance history for regression detection */
#define PERFORMANCE_HISTORY_SIZE 10
static u64 batch_insert_performance_history[PERFORMANCE_HISTORY_SIZE];
static int performance_history_index = 0;
static bool performance_history_full = false;

/* Forward declarations removed - functions implemented inline */

/**
 * Record a batch insert operation
 */
void vexfs_record_batch_insert(u32 vector_count, u64 processing_time_ns, 
                              size_t memory_used, bool success)
{
    u64 current_time = ktime_get_ns();
    u64 current_min, current_max;
    
    /* Update operation counters */
    atomic64_inc(&vexfs_batch_metrics.total_batch_operations);
    atomic64_add(vector_count, &vexfs_batch_metrics.total_vectors_processed);
    atomic64_add(processing_time_ns, &vexfs_batch_metrics.total_processing_time_ns);
    
    if (success) {
        atomic64_inc(&vexfs_batch_metrics.successful_batch_operations);
    } else {
        atomic64_inc(&vexfs_batch_metrics.failed_batch_operations);
    }
    
    /* Update latency statistics */
    current_min = atomic64_read(&vexfs_batch_metrics.min_latency_ns);
    while (processing_time_ns < current_min) {
        if (atomic64_cmpxchg(&vexfs_batch_metrics.min_latency_ns, 
                            current_min, processing_time_ns) == current_min) {
            break;
        }
        current_min = atomic64_read(&vexfs_batch_metrics.min_latency_ns);
    }
    
    current_max = atomic64_read(&vexfs_batch_metrics.max_latency_ns);
    while (processing_time_ns > current_max) {
        if (atomic64_cmpxchg(&vexfs_batch_metrics.max_latency_ns,
                            current_max, processing_time_ns) == current_max) {
            break;
        }
        current_max = atomic64_read(&vexfs_batch_metrics.max_latency_ns);
    }
    
    /* Update memory tracking */
    atomic64_add(memory_used, &vexfs_batch_metrics.total_memory_allocated);
    current_max = atomic64_read(&vexfs_batch_metrics.peak_memory_usage);
    while (memory_used > current_max) {
        if (atomic64_cmpxchg(&vexfs_batch_metrics.peak_memory_usage,
                            current_max, memory_used) == current_max) {
            break;
        }
        current_max = atomic64_read(&vexfs_batch_metrics.peak_memory_usage);
    }
    
    /* Calculate throughput every 100 operations */
    if (atomic64_read(&vexfs_batch_metrics.total_batch_operations) % 100 == 0) {
        vexfs_update_throughput_metrics();
    }
    
    /* Record batch size category */
    vexfs_record_batch_size(vector_count);
    
    vexfs_batch_metrics.last_measurement_time = current_time;
}

/**
 * Record vector metadata operation
 */
void vexfs_record_metadata_operation(u64 latency_ns, bool success)
{
    printk(KERN_INFO "VexFS MONITORING: metadata operation called - latency: %llu ns, success: %d\n", latency_ns, success);
    atomic64_inc(&vexfs_vector_metrics.metadata_operations);
    
    if (success) {
        atomic64_inc(&vexfs_vector_metrics.metadata_successes);
        
        /* Update average latency using exponential moving average */
        u64 current_avg = atomic64_read(&vexfs_vector_metrics.metadata_avg_latency_ns);
        u64 new_avg = (current_avg * 7 + latency_ns) / 8; /* 7/8 weight to previous */
        atomic64_set(&vexfs_vector_metrics.metadata_avg_latency_ns, new_avg);
    } else {
        atomic64_inc(&vexfs_vector_metrics.metadata_failures);
    }
}

/**
 * Record vector search operation
 */
void vexfs_record_search_operation(u64 latency_ns, bool success)
{
    atomic64_inc(&vexfs_vector_metrics.search_operations);
    
    if (success) {
        atomic64_inc(&vexfs_vector_metrics.search_successes);
        
        /* Update average latency using exponential moving average */
        u64 current_avg = atomic64_read(&vexfs_vector_metrics.search_avg_latency_ns);
        u64 new_avg = (current_avg * 7 + latency_ns) / 8; /* 7/8 weight to previous */
        atomic64_set(&vexfs_vector_metrics.search_avg_latency_ns, new_avg);
    } else {
        atomic64_inc(&vexfs_vector_metrics.search_failures);
    }
}

/**
 * Calculate and update throughput metrics
 */
void vexfs_update_throughput_metrics(void)
{
    u64 current_time = ktime_get_ns();
    u64 time_diff = current_time - vexfs_batch_metrics.last_measurement_time;
    u64 total_ops = atomic64_read(&vexfs_batch_metrics.total_batch_operations);
    u64 total_vectors = atomic64_read(&vexfs_batch_metrics.total_vectors_processed);
    
    if (time_diff > 0 && total_ops > 0) {
        /* Calculate operations per second */
        u64 ops_per_sec = (total_vectors * 1000000000ULL) / time_diff;
        atomic64_set(&vexfs_batch_metrics.last_throughput_ops_sec, ops_per_sec);
        
        /* Check performance target */
        vexfs_check_performance_target(ops_per_sec);
        
        /* Update performance history for regression detection */
        batch_insert_performance_history[performance_history_index] = ops_per_sec;
        performance_history_index = (performance_history_index + 1) % PERFORMANCE_HISTORY_SIZE;
        if (performance_history_index == 0) {
            performance_history_full = true;
        }
        
        /* Check for performance regression */
        if (vexfs_monitoring_flags & VEXFS_REGRESSION_DETECTION_ENABLED) {
            if (vexfs_check_performance_regression()) {
                atomic64_inc(&vexfs_batch_metrics.performance_regressions);
                printk(KERN_WARNING "VexFS: Performance regression detected! "
                       "Current: %llu ops/sec, Target: %u ops/sec\n",
                       ops_per_sec, VEXFS_TARGET_BATCH_INSERT_OPS_SEC);
            }
        }
    }
}

/**
 * Check for performance regressions
 */
bool vexfs_check_performance_regression(void)
{
    if (!performance_history_full && performance_history_index < 3) {
        return false; /* Not enough data */
    }
    
    u64 current_performance = atomic64_read(&vexfs_batch_metrics.last_throughput_ops_sec);
    u64 sum = 0;
    int count = performance_history_full ? PERFORMANCE_HISTORY_SIZE : performance_history_index;
    
    /* Calculate average of recent performance */
    for (int i = 0; i < count; i++) {
        sum += batch_insert_performance_history[i];
    }
    
    u64 avg_performance = sum / count;
    
    /* Check if current performance is significantly below average */
    /* Use integer arithmetic: current < avg * 0.8 becomes current * 10 < avg * 8 */
    return ((current_performance * 10) < (avg_performance * 8));
}

/**
 * Reset all monitoring counters
 */
void vexfs_reset_monitoring_counters(void)
{
    /* Reset batch insert metrics */
    atomic64_set(&vexfs_batch_metrics.total_batch_operations, 0);
    atomic64_set(&vexfs_batch_metrics.successful_batch_operations, 0);
    atomic64_set(&vexfs_batch_metrics.failed_batch_operations, 0);
    atomic64_set(&vexfs_batch_metrics.total_vectors_processed, 0);
    atomic64_set(&vexfs_batch_metrics.total_processing_time_ns, 0);
    atomic64_set(&vexfs_batch_metrics.min_latency_ns, LLONG_MAX);
    atomic64_set(&vexfs_batch_metrics.max_latency_ns, 0);
    atomic64_set(&vexfs_batch_metrics.last_throughput_ops_sec, 0);
    atomic64_set(&vexfs_batch_metrics.total_memory_allocated, 0);
    atomic64_set(&vexfs_batch_metrics.peak_memory_usage, 0);
    atomic64_set(&vexfs_batch_metrics.vmalloc_allocations, 0);
    atomic64_set(&vexfs_batch_metrics.kmalloc_allocations, 0);
    atomic64_set(&vexfs_batch_metrics.bulk_copy_operations, 0);
    atomic64_set(&vexfs_batch_metrics.scalar_validations, 0);
    atomic64_set(&vexfs_batch_metrics.simd_batch_optimizations, 0);
    atomic64_set(&vexfs_batch_metrics.cache_hits, 0);
    atomic64_set(&vexfs_batch_metrics.cache_misses, 0);
    atomic64_set(&vexfs_batch_metrics.validation_errors, 0);
    atomic64_set(&vexfs_batch_metrics.memory_allocation_errors, 0);
    atomic64_set(&vexfs_batch_metrics.copy_from_user_errors, 0);
    atomic64_set(&vexfs_batch_metrics.ioctl_structure_errors, 0);
    atomic64_set(&vexfs_batch_metrics.small_batches, 0);
    atomic64_set(&vexfs_batch_metrics.medium_batches, 0);
    atomic64_set(&vexfs_batch_metrics.large_batches, 0);
    atomic64_set(&vexfs_batch_metrics.optimal_batch_count, 0);
    atomic64_set(&vexfs_batch_metrics.target_achievements, 0);
    atomic64_set(&vexfs_batch_metrics.target_misses, 0);
    atomic64_set(&vexfs_batch_metrics.performance_regressions, 0);
    
    /* Reset vector metrics */
    atomic64_set(&vexfs_vector_metrics.metadata_operations, 0);
    atomic64_set(&vexfs_vector_metrics.metadata_successes, 0);
    atomic64_set(&vexfs_vector_metrics.metadata_failures, 0);
    atomic64_set(&vexfs_vector_metrics.metadata_avg_latency_ns, 0);
    atomic64_set(&vexfs_vector_metrics.search_operations, 0);
    atomic64_set(&vexfs_vector_metrics.search_successes, 0);
    atomic64_set(&vexfs_vector_metrics.search_failures, 0);
    atomic64_set(&vexfs_vector_metrics.search_avg_latency_ns, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_graph_builds, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_node_allocations, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_layer_traversals, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_distance_calculations, 0);
    atomic64_set(&vexfs_vector_metrics.avx2_operations, 0);
    atomic64_set(&vexfs_vector_metrics.sse2_fallback_operations, 0);
    atomic64_set(&vexfs_vector_metrics.scalar_operations, 0);
    atomic64_set(&vexfs_vector_metrics.simd_optimization_hits, 0);
    
    /* Reset performance history */
    memset(batch_insert_performance_history, 0, sizeof(batch_insert_performance_history));
    performance_history_index = 0;
    performance_history_full = false;
    
    vexfs_batch_metrics.last_measurement_time = ktime_get_ns();
    vexfs_batch_metrics.monitoring_start_time = ktime_get_ns();
    
    printk(KERN_INFO "VexFS monitoring counters reset\n");
}

/**
 * Get current performance summary
 */
void vexfs_get_performance_summary(char *buffer, size_t buffer_size)
{
    u64 total_ops = atomic64_read(&vexfs_batch_metrics.total_batch_operations);
    u64 successful_ops = atomic64_read(&vexfs_batch_metrics.successful_batch_operations);
    u64 total_vectors = atomic64_read(&vexfs_batch_metrics.total_vectors_processed);
    u64 current_throughput = atomic64_read(&vexfs_batch_metrics.last_throughput_ops_sec);
    u64 target_achievements = atomic64_read(&vexfs_batch_metrics.target_achievements);
    u64 target_misses = atomic64_read(&vexfs_batch_metrics.target_misses);
    u64 min_latency = atomic64_read(&vexfs_batch_metrics.min_latency_ns);
    u64 max_latency = atomic64_read(&vexfs_batch_metrics.max_latency_ns);
    
    snprintf(buffer, buffer_size,
        "VexFS v2.0 Performance Summary\n"
        "==============================\n"
        "Batch Insert Operations:\n"
        "  Total Operations: %llu\n"
        "  Successful: %llu (%llu%%)\n"
        "  Total Vectors Processed: %llu\n"
        "  Current Throughput: %llu ops/sec\n"
        "  Target Achievements: %llu\n"
        "  Target Misses: %llu\n"
        "  Min Latency: %llu ns\n"
        "  Max Latency: %llu ns\n"
        "  Performance Target: %s\n"
        "\n"
        "Memory Usage:\n"
        "  Total Allocated: %llu bytes\n"
        "  Peak Usage: %llu bytes\n"
        "  vmalloc Allocations: %llu\n"
        "  kmalloc Allocations: %llu\n"
        "\n"
        "Optimizations:\n"
        "  Bulk Copy Operations: %llu\n"
        "  Scalar Validations: %llu\n"
        "  SIMD Batch Optimizations: %llu\n"
        "  Optimal Batch Count: %llu\n",
        total_ops, successful_ops,
        total_ops > 0 ? (successful_ops * 100 / total_ops) : 0,
        total_vectors, current_throughput, target_achievements, target_misses,
        min_latency, max_latency,
        current_throughput >= VEXFS_TARGET_BATCH_INSERT_OPS_SEC ? "MET" : "MISSED",
        atomic64_read(&vexfs_batch_metrics.total_memory_allocated),
        atomic64_read(&vexfs_batch_metrics.peak_memory_usage),
        atomic64_read(&vexfs_batch_metrics.vmalloc_allocations),
        atomic64_read(&vexfs_batch_metrics.kmalloc_allocations),
        atomic64_read(&vexfs_batch_metrics.bulk_copy_operations),
        atomic64_read(&vexfs_batch_metrics.scalar_validations),
        atomic64_read(&vexfs_batch_metrics.simd_batch_optimizations),
        atomic64_read(&vexfs_batch_metrics.optimal_batch_count));
}

/* Proc filesystem show function implementations */
static int vexfs_batch_metrics_show(struct seq_file *m, void *v)
{
    seq_printf(m, "VexFS v2.0 Batch Insert Metrics\n");
    seq_printf(m, "================================\n");
    seq_printf(m, "total_batch_operations: %llu\n", 
               atomic64_read(&vexfs_batch_metrics.total_batch_operations));
    seq_printf(m, "successful_batch_operations: %llu\n",
               atomic64_read(&vexfs_batch_metrics.successful_batch_operations));
    seq_printf(m, "failed_batch_operations: %llu\n",
               atomic64_read(&vexfs_batch_metrics.failed_batch_operations));
    seq_printf(m, "total_vectors_processed: %llu\n",
               atomic64_read(&vexfs_batch_metrics.total_vectors_processed));
    seq_printf(m, "last_throughput_ops_sec: %llu\n",
               atomic64_read(&vexfs_batch_metrics.last_throughput_ops_sec));
    seq_printf(m, "min_latency_ns: %llu\n",
               atomic64_read(&vexfs_batch_metrics.min_latency_ns));
    seq_printf(m, "max_latency_ns: %llu\n",
               atomic64_read(&vexfs_batch_metrics.max_latency_ns));
    seq_printf(m, "target_achievements: %llu\n",
               atomic64_read(&vexfs_batch_metrics.target_achievements));
    seq_printf(m, "target_misses: %llu\n",
               atomic64_read(&vexfs_batch_metrics.target_misses));
    seq_printf(m, "performance_regressions: %llu\n",
               atomic64_read(&vexfs_batch_metrics.performance_regressions));
    
    return 0;
}

static int vexfs_vector_metrics_show(struct seq_file *m, void *v)
{
    seq_printf(m, "VexFS v2.0 Vector Operations Metrics\n");
    seq_printf(m, "====================================\n");
    seq_printf(m, "metadata_operations: %llu\n",
               atomic64_read(&vexfs_vector_metrics.metadata_operations));
    seq_printf(m, "metadata_successes: %llu\n",
               atomic64_read(&vexfs_vector_metrics.metadata_successes));
    seq_printf(m, "metadata_failures: %llu\n",
               atomic64_read(&vexfs_vector_metrics.metadata_failures));
    seq_printf(m, "search_operations: %llu\n",
               atomic64_read(&vexfs_vector_metrics.search_operations));
    seq_printf(m, "search_successes: %llu\n",
               atomic64_read(&vexfs_vector_metrics.search_successes));
    seq_printf(m, "search_failures: %llu\n",
               atomic64_read(&vexfs_vector_metrics.search_failures));
    seq_printf(m, "avx2_operations: %llu\n",
               atomic64_read(&vexfs_vector_metrics.avx2_operations));
    seq_printf(m, "sse2_fallback_operations: %llu\n",
               atomic64_read(&vexfs_vector_metrics.sse2_fallback_operations));
    seq_printf(m, "scalar_operations: %llu\n",
               atomic64_read(&vexfs_vector_metrics.scalar_operations));
    
    return 0;
}

static int vexfs_performance_summary_show(struct seq_file *m, void *v)
{
    char summary_buffer[2048];
    vexfs_get_performance_summary(summary_buffer, sizeof(summary_buffer));
    seq_printf(m, "%s", summary_buffer);
    return 0;
}

/* Proc filesystem operations - Must be after show function implementations */
static int vexfs_batch_metrics_open(struct inode *inode, struct file *file)
{
    return single_open(file, vexfs_batch_metrics_show, NULL);
}

static int vexfs_vector_metrics_open(struct inode *inode, struct file *file)
{
    return single_open(file, vexfs_vector_metrics_show, NULL);
}

static int vexfs_performance_summary_open(struct inode *inode, struct file *file)
{
    return single_open(file, vexfs_performance_summary_show, NULL);
}

static const struct proc_ops vexfs_batch_metrics_proc_ops = {
    .proc_open = vexfs_batch_metrics_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

static const struct proc_ops vexfs_vector_metrics_proc_ops = {
    .proc_open = vexfs_vector_metrics_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

static const struct proc_ops vexfs_performance_summary_proc_ops = {
    .proc_open = vexfs_performance_summary_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

/**
 * Initialize the VexFS monitoring system
 */
int vexfs_monitoring_init(void)
{
    /* Initialize batch insert metrics */
    atomic64_set(&vexfs_batch_metrics.total_batch_operations, 0);
    atomic64_set(&vexfs_batch_metrics.successful_batch_operations, 0);
    atomic64_set(&vexfs_batch_metrics.failed_batch_operations, 0);
    atomic64_set(&vexfs_batch_metrics.total_vectors_processed, 0);
    atomic64_set(&vexfs_batch_metrics.total_processing_time_ns, 0);
    atomic64_set(&vexfs_batch_metrics.min_latency_ns, LLONG_MAX);
    atomic64_set(&vexfs_batch_metrics.max_latency_ns, 0);
    atomic64_set(&vexfs_batch_metrics.last_throughput_ops_sec, 0);
    atomic64_set(&vexfs_batch_metrics.total_memory_allocated, 0);
    atomic64_set(&vexfs_batch_metrics.peak_memory_usage, 0);
    atomic64_set(&vexfs_batch_metrics.vmalloc_allocations, 0);
    atomic64_set(&vexfs_batch_metrics.kmalloc_allocations, 0);
    atomic64_set(&vexfs_batch_metrics.bulk_copy_operations, 0);
    atomic64_set(&vexfs_batch_metrics.scalar_validations, 0);
    atomic64_set(&vexfs_batch_metrics.simd_batch_optimizations, 0);
    atomic64_set(&vexfs_batch_metrics.cache_hits, 0);
    atomic64_set(&vexfs_batch_metrics.cache_misses, 0);
    atomic64_set(&vexfs_batch_metrics.validation_errors, 0);
    atomic64_set(&vexfs_batch_metrics.memory_allocation_errors, 0);
    atomic64_set(&vexfs_batch_metrics.copy_from_user_errors, 0);
    atomic64_set(&vexfs_batch_metrics.ioctl_structure_errors, 0);
    atomic64_set(&vexfs_batch_metrics.small_batches, 0);
    atomic64_set(&vexfs_batch_metrics.medium_batches, 0);
    atomic64_set(&vexfs_batch_metrics.large_batches, 0);
    atomic64_set(&vexfs_batch_metrics.optimal_batch_count, 0);
    atomic64_set(&vexfs_batch_metrics.target_achievements, 0);
    atomic64_set(&vexfs_batch_metrics.target_misses, 0);
    atomic64_set(&vexfs_batch_metrics.performance_regressions, 0);
    
    vexfs_batch_metrics.last_measurement_time = ktime_get_ns();
    vexfs_batch_metrics.monitoring_start_time = ktime_get_ns();
    
    /* Initialize vector metrics */
    atomic64_set(&vexfs_vector_metrics.metadata_operations, 0);
    atomic64_set(&vexfs_vector_metrics.metadata_successes, 0);
    atomic64_set(&vexfs_vector_metrics.metadata_failures, 0);
    atomic64_set(&vexfs_vector_metrics.metadata_avg_latency_ns, 0);
    atomic64_set(&vexfs_vector_metrics.search_operations, 0);
    atomic64_set(&vexfs_vector_metrics.search_successes, 0);
    atomic64_set(&vexfs_vector_metrics.search_failures, 0);
    atomic64_set(&vexfs_vector_metrics.search_avg_latency_ns, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_graph_builds, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_node_allocations, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_layer_traversals, 0);
    atomic64_set(&vexfs_vector_metrics.hnsw_distance_calculations, 0);
    atomic64_set(&vexfs_vector_metrics.avx2_operations, 0);
    atomic64_set(&vexfs_vector_metrics.sse2_fallback_operations, 0);
    atomic64_set(&vexfs_vector_metrics.scalar_operations, 0);
    atomic64_set(&vexfs_vector_metrics.simd_optimization_hits, 0);
    
    /* Initialize performance history */
    memset(batch_insert_performance_history, 0, sizeof(batch_insert_performance_history));
    performance_history_index = 0;
    performance_history_full = false;
    
    /* Create proc filesystem entries */
    vexfs_proc_dir = proc_mkdir("vexfs_v2", NULL);
    if (!vexfs_proc_dir) {
        printk(KERN_ERR "VexFS: Failed to create proc directory\n");
        return -ENOMEM;
    }
    
    vexfs_batch_metrics_proc = proc_create("batch_metrics", 0444, vexfs_proc_dir,
                                          &vexfs_batch_metrics_proc_ops);
    if (!vexfs_batch_metrics_proc) {
        printk(KERN_ERR "VexFS: Failed to create batch_metrics proc entry\n");
        goto cleanup_proc_dir;
    }
    
    vexfs_vector_metrics_proc = proc_create("vector_metrics", 0444, vexfs_proc_dir,
                                           &vexfs_vector_metrics_proc_ops);
    if (!vexfs_vector_metrics_proc) {
        printk(KERN_ERR "VexFS: Failed to create vector_metrics proc entry\n");
        goto cleanup_batch_proc;
    }
    
    vexfs_performance_summary_proc = proc_create("performance_summary", 0444, vexfs_proc_dir,
                                                 &vexfs_performance_summary_proc_ops);
    if (!vexfs_performance_summary_proc) {
        printk(KERN_ERR "VexFS: Failed to create performance_summary proc entry\n");
        goto cleanup_vector_proc;
    }
    
    printk(KERN_INFO "VexFS v2.0 monitoring system initialized\n");
    return 0;
    
cleanup_vector_proc:
    proc_remove(vexfs_vector_metrics_proc);
cleanup_batch_proc:
    proc_remove(vexfs_batch_metrics_proc);
cleanup_proc_dir:
    proc_remove(vexfs_proc_dir);
    return -ENOMEM;
}

/**
 * Cleanup the VexFS monitoring system
 */
void vexfs_monitoring_cleanup(void)
{
    if (vexfs_performance_summary_proc) {
        proc_remove(vexfs_performance_summary_proc);
    }
    if (vexfs_vector_metrics_proc) {
        proc_remove(vexfs_vector_metrics_proc);
    }
    if (vexfs_batch_metrics_proc) {
        proc_remove(vexfs_batch_metrics_proc);
    }
    if (vexfs_proc_dir) {
        proc_remove(vexfs_proc_dir);
    }
    
    printk(KERN_INFO "VexFS v2.0 monitoring system cleaned up\n");
}