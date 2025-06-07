/*
 * VexFS v2.0 Comprehensive Performance Monitoring Implementation
 * 
 * Task 57: Implement Comprehensive Performance Monitoring
 * 
 * This module provides detailed performance monitoring and statistics collection
 * for vector operations, including tracepoints, configurable logging, and
 * enhanced proc/sysfs interfaces.
 * 
 * Features:
 * - Atomic counters for high-performance statistics collection
 * - Tracepoints for detailed performance analysis with trace-cmd
 * - Configurable logging levels for debugging and performance analysis
 * - Enhanced proc/sysfs interfaces for statistics access
 * - Proper cleanup of statistics during module unload
 * - Memory usage tracking for vector data and indices
 * - Timing measurements for operation latency
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/atomic.h>
#include <linux/time.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/sysfs.h>
#include <linux/kobject.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/tracepoint.h>
#include <linux/trace_events.h>
#include <linux/debugfs.h>
#include <linux/vmalloc.h>
#include <linux/percpu.h>
#include <linux/cpumask.h>
#include <linux/workqueue.h>
#include <linux/timer.h>

#include "vexfs_v2_monitoring.h"
#include "vexfs_v2_vector_processing.h"

/*
 * Tracepoint Definitions for VexFS Performance Analysis
 */

/* Define tracepoints for vector operations */
DEFINE_TRACE(vexfs_vector_insert_start);
DEFINE_TRACE(vexfs_vector_insert_end);
DEFINE_TRACE(vexfs_vector_search_start);
DEFINE_TRACE(vexfs_vector_search_end);
DEFINE_TRACE(vexfs_vector_quantize_start);
DEFINE_TRACE(vexfs_vector_quantize_end);
DEFINE_TRACE(vexfs_simd_operation);
DEFINE_TRACE(vexfs_memory_allocation);
DEFINE_TRACE(vexfs_performance_regression);

/*
 * Enhanced Performance Monitoring Structures
 */

/* Comprehensive Vector Operation Metrics */
struct vexfs_comprehensive_metrics {
    /* Core Operation Counters */
    atomic64_t vector_inserts;              /* Total vector insert operations */
    atomic64_t vector_searches;             /* Total vector search operations */
    atomic64_t vector_updates;              /* Total vector update operations */
    atomic64_t vector_deletes;              /* Total vector delete operations */
    atomic64_t vector_quantizations;        /* Total quantization operations */
    atomic64_t vector_normalizations;       /* Total normalization operations */
    
    /* SIMD Operation Counters */
    atomic64_t avx2_operations;             /* AVX2 SIMD operations */
    atomic64_t sse2_operations;             /* SSE2 SIMD operations */
    atomic64_t scalar_fallbacks;            /* Scalar fallback operations */
    atomic64_t simd_efficiency_percent;     /* SIMD usage efficiency */
    
    /* Memory Management Counters */
    atomic64_t kmalloc_calls;               /* kmalloc allocation calls */
    atomic64_t vmalloc_calls;               /* vmalloc allocation calls */
    atomic64_t kfree_calls;                 /* kfree deallocation calls */
    atomic64_t vfree_calls;                 /* vfree deallocation calls */
    atomic64_t total_memory_allocated;      /* Total memory allocated */
    atomic64_t total_memory_freed;          /* Total memory freed */
    atomic64_t peak_memory_usage;           /* Peak memory usage */
    atomic64_t current_memory_usage;        /* Current memory usage */
    
    /* Timing Statistics (in nanoseconds) */
    atomic64_t total_insert_time_ns;        /* Total insert operation time */
    atomic64_t total_search_time_ns;        /* Total search operation time */
    atomic64_t total_quantize_time_ns;      /* Total quantization time */
    atomic64_t min_insert_latency_ns;       /* Minimum insert latency */
    atomic64_t max_insert_latency_ns;       /* Maximum insert latency */
    atomic64_t min_search_latency_ns;       /* Minimum search latency */
    atomic64_t max_search_latency_ns;       /* Maximum search latency */
    
    /* Error Counters */
    atomic64_t allocation_failures;         /* Memory allocation failures */
    atomic64_t validation_errors;           /* Vector validation errors */
    atomic64_t simd_errors;                 /* SIMD operation errors */
    atomic64_t timeout_errors;              /* Operation timeout errors */
    
    /* Performance Quality Metrics */
    atomic64_t cache_hits;                  /* Cache hit count */
    atomic64_t cache_misses;                /* Cache miss count */
    atomic64_t prefetch_hits;               /* Prefetch hit count */
    atomic64_t prefetch_misses;             /* Prefetch miss count */
    
    /* Per-CPU Statistics */
    struct percpu_ref cpu_operations;       /* Per-CPU operation tracking */
    
    /* Timing Information */
    u64 monitoring_start_time;              /* Monitoring start timestamp */
    u64 last_reset_time;                    /* Last reset timestamp */
    u64 last_update_time;                   /* Last update timestamp */
};

/* Configurable Logging Levels */
enum vexfs_log_level {
    VEXFS_LOG_NONE = 0,
    VEXFS_LOG_ERROR = 1,
    VEXFS_LOG_WARN = 2,
    VEXFS_LOG_INFO = 3,
    VEXFS_LOG_DEBUG = 4,
    VEXFS_LOG_TRACE = 5
};

/* Global monitoring state */
static struct vexfs_comprehensive_metrics vexfs_comp_metrics;
static enum vexfs_log_level vexfs_current_log_level = VEXFS_LOG_INFO;
static bool vexfs_tracing_enabled = true;
static bool vexfs_monitoring_active = true;

/* Proc and sysfs interfaces */
static struct proc_dir_entry *vexfs_comp_proc_dir = NULL;
static struct proc_dir_entry *vexfs_comp_metrics_proc = NULL;
static struct proc_dir_entry *vexfs_comp_config_proc = NULL;
static struct kobject *vexfs_comp_kobj = NULL;

/* Performance monitoring timer */
static struct timer_list vexfs_perf_timer;
static struct workqueue_struct *vexfs_monitoring_wq = NULL;
static struct delayed_work vexfs_monitoring_work;

/*
 * Logging Macros with Configurable Levels
 */
#define vexfs_log(level, fmt, ...) \
    do { \
        if (level <= vexfs_current_log_level) { \
            printk(KERN_INFO "VexFS[%s]: " fmt "\n", \
                   __func__, ##__VA_ARGS__); \
        } \
    } while (0)

#define vexfs_log_error(fmt, ...)   vexfs_log(VEXFS_LOG_ERROR, fmt, ##__VA_ARGS__)
#define vexfs_log_warn(fmt, ...)    vexfs_log(VEXFS_LOG_WARN, fmt, ##__VA_ARGS__)
#define vexfs_log_info(fmt, ...)    vexfs_log(VEXFS_LOG_INFO, fmt, ##__VA_ARGS__)
#define vexfs_log_debug(fmt, ...)   vexfs_log(VEXFS_LOG_DEBUG, fmt, ##__VA_ARGS__)
#define vexfs_log_trace(fmt, ...)   vexfs_log(VEXFS_LOG_TRACE, fmt, ##__VA_ARGS__)

/*
 * Tracepoint Helper Functions
 */

/**
 * Record vector insert operation with tracepoint
 */
void vexfs_trace_vector_insert(u32 vector_id, u32 dimensions, u64 start_time_ns)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_vector_insert_start(vector_id, dimensions, start_time_ns);
    }
    
    atomic64_inc(&vexfs_comp_metrics.vector_inserts);
    vexfs_log_trace("Vector insert started: id=%u, dims=%u", vector_id, dimensions);
}

/**
 * Complete vector insert operation with tracepoint
 */
void vexfs_trace_vector_insert_complete(u32 vector_id, u64 duration_ns, bool success)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_vector_insert_end(vector_id, duration_ns, success);
    }
    
    atomic64_add(duration_ns, &vexfs_comp_metrics.total_insert_time_ns);
    
    /* Update min/max latency */
    u64 current_min = atomic64_read(&vexfs_comp_metrics.min_insert_latency_ns);
    u64 current_max = atomic64_read(&vexfs_comp_metrics.max_insert_latency_ns);
    
    if (current_min == 0 || duration_ns < current_min) {
        atomic64_set(&vexfs_comp_metrics.min_insert_latency_ns, duration_ns);
    }
    if (duration_ns > current_max) {
        atomic64_set(&vexfs_comp_metrics.max_insert_latency_ns, duration_ns);
    }
    
    vexfs_log_trace("Vector insert completed: id=%u, duration=%llu ns, success=%d", 
                    vector_id, duration_ns, success);
}

/**
 * Record vector search operation with tracepoint
 */
void vexfs_trace_vector_search(u32 query_dims, u32 k_neighbors, u64 start_time_ns)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_vector_search_start(query_dims, k_neighbors, start_time_ns);
    }
    
    atomic64_inc(&vexfs_comp_metrics.vector_searches);
    vexfs_log_trace("Vector search started: dims=%u, k=%u", query_dims, k_neighbors);
}

/**
 * Complete vector search operation with tracepoint
 */
void vexfs_trace_vector_search_complete(u32 results_found, u64 duration_ns, bool success)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_vector_search_end(results_found, duration_ns, success);
    }
    
    atomic64_add(duration_ns, &vexfs_comp_metrics.total_search_time_ns);
    
    /* Update min/max search latency */
    u64 current_min = atomic64_read(&vexfs_comp_metrics.min_search_latency_ns);
    u64 current_max = atomic64_read(&vexfs_comp_metrics.max_search_latency_ns);
    
    if (current_min == 0 || duration_ns < current_min) {
        atomic64_set(&vexfs_comp_metrics.min_search_latency_ns, duration_ns);
    }
    if (duration_ns > current_max) {
        atomic64_set(&vexfs_comp_metrics.max_search_latency_ns, duration_ns);
    }
    
    vexfs_log_trace("Vector search completed: results=%u, duration=%llu ns, success=%d", 
                    results_found, duration_ns, success);
}

/**
 * Record SIMD operation with tracepoint
 */
void vexfs_trace_simd_operation(const char *operation, u32 simd_type, u32 vector_count, u64 duration_ns)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_simd_operation(operation, simd_type, vector_count, duration_ns);
    }
    
    switch (simd_type) {
    case VEXFS_SIMD_AVX2:
        atomic64_inc(&vexfs_comp_metrics.avx2_operations);
        break;
    case VEXFS_SIMD_SSE2:
        atomic64_inc(&vexfs_comp_metrics.sse2_operations);
        break;
    default:
        atomic64_inc(&vexfs_comp_metrics.scalar_fallbacks);
        break;
    }
    
    vexfs_log_trace("SIMD operation: %s, type=%u, vectors=%u, duration=%llu ns", 
                    operation, simd_type, vector_count, duration_ns);
}

/**
 * Record memory allocation with tracepoint
 */
void vexfs_trace_memory_allocation(size_t size, bool is_vmalloc, bool success)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_memory_allocation(size, is_vmalloc, success);
    }
    
    if (success) {
        if (is_vmalloc) {
            atomic64_inc(&vexfs_comp_metrics.vmalloc_calls);
        } else {
            atomic64_inc(&vexfs_comp_metrics.kmalloc_calls);
        }
        
        atomic64_add(size, &vexfs_comp_metrics.total_memory_allocated);
        atomic64_add(size, &vexfs_comp_metrics.current_memory_usage);
        
        /* Update peak memory usage */
        u64 current_usage = atomic64_read(&vexfs_comp_metrics.current_memory_usage);
        u64 peak_usage = atomic64_read(&vexfs_comp_metrics.peak_memory_usage);
        if (current_usage > peak_usage) {
            atomic64_set(&vexfs_comp_metrics.peak_memory_usage, current_usage);
        }
    } else {
        atomic64_inc(&vexfs_comp_metrics.allocation_failures);
    }
    
    vexfs_log_trace("Memory allocation: size=%zu, vmalloc=%d, success=%d", 
                    size, is_vmalloc, success);
}

/**
 * Record memory deallocation
 */
void vexfs_trace_memory_deallocation(size_t size, bool is_vfree)
{
    if (is_vfree) {
        atomic64_inc(&vexfs_comp_metrics.vfree_calls);
    } else {
        atomic64_inc(&vexfs_comp_metrics.kfree_calls);
    }
    
    atomic64_add(size, &vexfs_comp_metrics.total_memory_freed);
    atomic64_sub(size, &vexfs_comp_metrics.current_memory_usage);
    
    vexfs_log_trace("Memory deallocation: size=%zu, vfree=%d", size, is_vfree);
}

/**
 * Record performance regression with tracepoint
 */
void vexfs_trace_performance_regression(const char *operation, u64 current_perf, u64 baseline_perf)
{
    if (vexfs_tracing_enabled) {
        trace_vexfs_performance_regression(operation, current_perf, baseline_perf);
    }
    
    vexfs_log_warn("Performance regression detected: %s, current=%llu, baseline=%llu", 
                   operation, current_perf, baseline_perf);
}

/*
 * Proc Filesystem Interface
 */

/**
 * Show comprehensive metrics in proc
 */
static int vexfs_comp_metrics_show(struct seq_file *m, void *v)
{
    u64 total_ops, total_time, avg_latency;
    u64 memory_efficiency, simd_efficiency;
    
    seq_printf(m, "VexFS v2.0 Comprehensive Performance Metrics\n");
    seq_printf(m, "==========================================\n\n");
    
    /* Core Operations */
    seq_printf(m, "Core Operations:\n");
    seq_printf(m, "  Vector Inserts:       %llu\n", atomic64_read(&vexfs_comp_metrics.vector_inserts));
    seq_printf(m, "  Vector Searches:      %llu\n", atomic64_read(&vexfs_comp_metrics.vector_searches));
    seq_printf(m, "  Vector Updates:       %llu\n", atomic64_read(&vexfs_comp_metrics.vector_updates));
    seq_printf(m, "  Vector Deletes:       %llu\n", atomic64_read(&vexfs_comp_metrics.vector_deletes));
    seq_printf(m, "  Quantizations:        %llu\n", atomic64_read(&vexfs_comp_metrics.vector_quantizations));
    seq_printf(m, "  Normalizations:       %llu\n", atomic64_read(&vexfs_comp_metrics.vector_normalizations));
    seq_printf(m, "\n");
    
    /* SIMD Operations */
    seq_printf(m, "SIMD Operations:\n");
    seq_printf(m, "  AVX2 Operations:      %llu\n", atomic64_read(&vexfs_comp_metrics.avx2_operations));
    seq_printf(m, "  SSE2 Operations:      %llu\n", atomic64_read(&vexfs_comp_metrics.sse2_operations));
    seq_printf(m, "  Scalar Fallbacks:     %llu\n", atomic64_read(&vexfs_comp_metrics.scalar_fallbacks));
    
    total_ops = atomic64_read(&vexfs_comp_metrics.avx2_operations) + 
                atomic64_read(&vexfs_comp_metrics.sse2_operations) + 
                atomic64_read(&vexfs_comp_metrics.scalar_fallbacks);
    if (total_ops > 0) {
        simd_efficiency = ((atomic64_read(&vexfs_comp_metrics.avx2_operations) + 
                           atomic64_read(&vexfs_comp_metrics.sse2_operations)) * 100) / total_ops;
        seq_printf(m, "  SIMD Efficiency:      %llu%%\n", simd_efficiency);
    }
    seq_printf(m, "\n");
    
    /* Memory Management */
    seq_printf(m, "Memory Management:\n");
    seq_printf(m, "  kmalloc Calls:        %llu\n", atomic64_read(&vexfs_comp_metrics.kmalloc_calls));
    seq_printf(m, "  vmalloc Calls:        %llu\n", atomic64_read(&vexfs_comp_metrics.vmalloc_calls));
    seq_printf(m, "  kfree Calls:          %llu\n", atomic64_read(&vexfs_comp_metrics.kfree_calls));
    seq_printf(m, "  vfree Calls:          %llu\n", atomic64_read(&vexfs_comp_metrics.vfree_calls));
    seq_printf(m, "  Total Allocated:      %llu bytes\n", atomic64_read(&vexfs_comp_metrics.total_memory_allocated));
    seq_printf(m, "  Total Freed:          %llu bytes\n", atomic64_read(&vexfs_comp_metrics.total_memory_freed));
    seq_printf(m, "  Current Usage:        %llu bytes\n", atomic64_read(&vexfs_comp_metrics.current_memory_usage));
    seq_printf(m, "  Peak Usage:           %llu bytes\n", atomic64_read(&vexfs_comp_metrics.peak_memory_usage));
    
    if (atomic64_read(&vexfs_comp_metrics.total_memory_allocated) > 0) {
        memory_efficiency = (atomic64_read(&vexfs_comp_metrics.total_memory_freed) * 100) / 
                           atomic64_read(&vexfs_comp_metrics.total_memory_allocated);
        seq_printf(m, "  Memory Efficiency:    %llu%%\n", memory_efficiency);
    }
    seq_printf(m, "\n");
    
    /* Timing Statistics */
    seq_printf(m, "Timing Statistics:\n");
    seq_printf(m, "  Total Insert Time:    %llu ns\n", atomic64_read(&vexfs_comp_metrics.total_insert_time_ns));
    seq_printf(m, "  Total Search Time:    %llu ns\n", atomic64_read(&vexfs_comp_metrics.total_search_time_ns));
    seq_printf(m, "  Min Insert Latency:   %llu ns\n", atomic64_read(&vexfs_comp_metrics.min_insert_latency_ns));
    seq_printf(m, "  Max Insert Latency:   %llu ns\n", atomic64_read(&vexfs_comp_metrics.max_insert_latency_ns));
    seq_printf(m, "  Min Search Latency:   %llu ns\n", atomic64_read(&vexfs_comp_metrics.min_search_latency_ns));
    seq_printf(m, "  Max Search Latency:   %llu ns\n", atomic64_read(&vexfs_comp_metrics.max_search_latency_ns));
    
    /* Calculate average latencies */
    if (atomic64_read(&vexfs_comp_metrics.vector_inserts) > 0) {
        avg_latency = atomic64_read(&vexfs_comp_metrics.total_insert_time_ns) / 
                     atomic64_read(&vexfs_comp_metrics.vector_inserts);
        seq_printf(m, "  Avg Insert Latency:   %llu ns\n", avg_latency);
    }
    
    if (atomic64_read(&vexfs_comp_metrics.vector_searches) > 0) {
        avg_latency = atomic64_read(&vexfs_comp_metrics.total_search_time_ns) / 
                     atomic64_read(&vexfs_comp_metrics.vector_searches);
        seq_printf(m, "  Avg Search Latency:   %llu ns\n", avg_latency);
    }
    seq_printf(m, "\n");
    
    /* Error Counters */
    seq_printf(m, "Error Counters:\n");
    seq_printf(m, "  Allocation Failures:  %llu\n", atomic64_read(&vexfs_comp_metrics.allocation_failures));
    seq_printf(m, "  Validation Errors:    %llu\n", atomic64_read(&vexfs_comp_metrics.validation_errors));
    seq_printf(m, "  SIMD Errors:          %llu\n", atomic64_read(&vexfs_comp_metrics.simd_errors));
    seq_printf(m, "  Timeout Errors:       %llu\n", atomic64_read(&vexfs_comp_metrics.timeout_errors));
    seq_printf(m, "\n");
    
    /* Cache Performance */
    seq_printf(m, "Cache Performance:\n");
    seq_printf(m, "  Cache Hits:           %llu\n", atomic64_read(&vexfs_comp_metrics.cache_hits));
    seq_printf(m, "  Cache Misses:         %llu\n", atomic64_read(&vexfs_comp_metrics.cache_misses));
    seq_printf(m, "  Prefetch Hits:        %llu\n", atomic64_read(&vexfs_comp_metrics.prefetch_hits));
    seq_printf(m, "  Prefetch Misses:      %llu\n", atomic64_read(&vexfs_comp_metrics.prefetch_misses));
    
    total_ops = atomic64_read(&vexfs_comp_metrics.cache_hits) + 
                atomic64_read(&vexfs_comp_metrics.cache_misses);
    if (total_ops > 0) {
        u64 cache_hit_rate = (atomic64_read(&vexfs_comp_metrics.cache_hits) * 100) / total_ops;
        seq_printf(m, "  Cache Hit Rate:       %llu%%\n", cache_hit_rate);
    }
    seq_printf(m, "\n");
    
    /* Monitoring Status */
    seq_printf(m, "Monitoring Status:\n");
    seq_printf(m, "  Monitoring Active:    %s\n", vexfs_monitoring_active ? "Yes" : "No");
    seq_printf(m, "  Tracing Enabled:      %s\n", vexfs_tracing_enabled ? "Yes" : "No");
    seq_printf(m, "  Log Level:            %d\n", vexfs_current_log_level);
    seq_printf(m, "  Start Time:           %llu ns\n", vexfs_comp_metrics.monitoring_start_time);
    seq_printf(m, "  Last Reset:           %llu ns\n", vexfs_comp_metrics.last_reset_time);
    seq_printf(m, "  Last Update:          %llu ns\n", vexfs_comp_metrics.last_update_time);
    
    return 0;
}

static int vexfs_comp_metrics_open(struct inode *inode, struct file *file)
{
    return single_open(file, vexfs_comp_metrics_show, NULL);
}

static const struct proc_ops vexfs_comp_metrics_proc_ops = {
    .proc_open = vexfs_comp_metrics_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

/**
 * Configuration interface in proc
 */
static ssize_t vexfs_comp_config_write(struct file *file, const char __user *buffer,
                                       size_t count, loff_t *pos)
{
    char cmd[64];
    int value;
    
    if (count >= sizeof(cmd))
        return -EINVAL;
    
    if (copy_from_user(cmd, buffer, count))
        return -EFAULT;
    
    cmd[count] = '\0';
    
    if (sscanf(cmd, "log_level %d", &value) == 1) {
        if (value >= VEXFS_LOG_NONE && value <= VEXFS_LOG_TRACE) {
            vexfs_current_log_level = value;
            vexfs_log_info("Log level set to %d", value);
        }
    } else if (sscanf(cmd, "tracing %d", &value) == 1) {
        vexfs_tracing_enabled = (value != 0);
        vexfs_log_info("Tracing %s", vexfs_tracing_enabled ? "enabled" : "disabled");
    } else if (sscanf(cmd, "monitoring %d", &value) == 1) {
        vexfs_monitoring_active = (value != 0);
        vexfs_log_info("Monitoring %s", vexfs_monitoring_active ? "enabled" : "disabled");
    } else if (strncmp(cmd, "reset", 5) == 0) {
        /* Reset all counters */
        memset(&vexfs_comp_metrics, 0, sizeof(vexfs_comp_metrics));
        vexfs_comp_metrics.monitoring_start_time = ktime_get_ns();
        vexfs_comp_metrics.last_reset_time = ktime_get_ns();
        vexfs_log_info("All metrics reset");
    }
    
    return count;
}

static int vexfs_comp_config_show(struct seq_file *m, void *v)
{
    seq_printf(m, "VexFS v2.0 Comprehensive Monitoring Configuration\n");
    seq_printf(m, "================================================\n\n");
    seq_printf(m, "Current Settings:\n");
    seq_printf(m, "  log_level:    %d (0=none, 1=error, 2=warn, 3=info, 4=debug, 5=trace)\n", vexfs_current_log_level);
    seq_printf(m, "  tracing:      %d (0=disabled, 1=enabled)\n", vexfs_tracing_enabled ? 1 : 0);
    seq_printf(m, "  monitoring:   %d (0=disabled, 1=enabled)\n", vexfs_monitoring_active ? 1 : 0);
    seq_printf(m, "\n");
    seq_printf(m, "Commands:\n");
    seq_printf(m, "  echo 'log_level N' > /proc/vexfs_comp/config\n");
    seq_printf(m, "  echo 'tracing N' > /proc/vexfs_comp/config\n");
    seq_printf(m, "  echo 'monitoring N' > /proc/vexfs_comp/config\n");
    seq_printf(m, "  echo 'reset' > /proc/vexfs_comp/config\n");
    
    return 0;
}

static int vexfs_comp_config_open(struct inode *inode, struct file *file)
{
    return single_open(file, vexfs_comp_config_show, NULL);
}

static const struct proc_ops vexfs_comp_config_proc_ops = {
    .proc_open = vexfs_comp_config_open,
    .proc_read = seq_read,
    .proc_write = vexfs_comp_config_write,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

/*
 * Sysfs Interface
 */

static ssize_t vexfs_sysfs_metrics_show(struct kobject *kobj, struct kobj_attribute *attr, char *buf)
{
    return sprintf(buf, "inserts=%llu searches=%llu memory_usage=%llu peak_memory=%llu\n",
                   atomic64_read(&vexfs_comp_metrics.vector_inserts),
                   atomic64_read(&vexfs_comp_metrics.vector_searches),
                   atomic64_read(&vexfs_comp_metrics.current_memory_usage),
                   atomic64_read(&vexfs_comp_metrics.peak_memory_usage));
}

static ssize_t vexfs_sysfs_log_level_show(struct kobject *kobj, struct kobj_attribute *attr, char *buf)
{
    return sprintf(buf, "%d\n", vexfs_current_log_level);
}

static ssize_t vexfs_sysfs_log_level_store(struct kobject *kobj, struct kobj_attribute *attr,
static ssize_t vexfs_sysfs_log_level_store(struct kobject *kobj, struct kobj_attribute *attr,
                                           const char *buf, size_t count)
{
    int level;
    
    if (sscanf(buf, "%d", &level) == 1) {
        if (level >= VEXFS_LOG_NONE && level <= VEXFS_LOG_TRACE) {
            vexfs_current_log_level = level;
            vexfs_log_info("Log level set to %d via sysfs", level);
        }
    }
    
    return count;
}

static ssize_t vexfs_sysfs_tracing_show(struct kobject *kobj, struct kobj_attribute *attr, char *buf)
{
    return sprintf(buf, "%d\n", vexfs_tracing_enabled ? 1 : 0);
}

static ssize_t vexfs_sysfs_tracing_store(struct kobject *kobj, struct kobj_attribute *attr,
                                         const char *buf, size_t count)
{
    int enabled;
    
    if (sscanf(buf, "%d", &enabled) == 1) {
        vexfs_tracing_enabled = (enabled != 0);
        vexfs_log_info("Tracing %s via sysfs", vexfs_tracing_enabled ? "enabled" : "disabled");
    }
    
    return count;
}

/* Sysfs attributes */
static struct kobj_attribute vexfs_metrics_attr = __ATTR(metrics, 0444, vexfs_sysfs_metrics_show, NULL);
static struct kobj_attribute vexfs_log_level_attr = __ATTR(log_level, 0644, vexfs_sysfs_log_level_show, vexfs_sysfs_log_level_store);
static struct kobj_attribute vexfs_tracing_attr = __ATTR(tracing, 0644, vexfs_sysfs_tracing_show, vexfs_sysfs_tracing_store);

static struct attribute *vexfs_comp_attrs[] = {
    &vexfs_metrics_attr.attr,
    &vexfs_log_level_attr.attr,
    &vexfs_tracing_attr.attr,
    NULL,
};

static struct attribute_group vexfs_comp_attr_group = {
    .attrs = vexfs_comp_attrs,
};

/*
 * Performance Monitoring Work Queue
 */

/**
 * Periodic performance monitoring work
 */
static void vexfs_monitoring_work_func(struct work_struct *work)
{
    u64 current_time = ktime_get_ns();
    u64 time_diff;
    
    if (!vexfs_monitoring_active) {
        return;
    }
    
    /* Update last update time */
    vexfs_comp_metrics.last_update_time = current_time;
    
    /* Calculate SIMD efficiency */
    u64 total_simd_ops = atomic64_read(&vexfs_comp_metrics.avx2_operations) + 
                        atomic64_read(&vexfs_comp_metrics.sse2_operations);
    u64 total_ops = total_simd_ops + atomic64_read(&vexfs_comp_metrics.scalar_fallbacks);
    
    if (total_ops > 0) {
        u64 efficiency = (total_simd_ops * 100) / total_ops;
        atomic64_set(&vexfs_comp_metrics.simd_efficiency_percent, efficiency);
    }
    
    /* Check for performance regressions */
    static u64 last_insert_count = 0;
    static u64 last_insert_time = 0;
    static u64 last_check_time = 0;
    
    u64 current_insert_count = atomic64_read(&vexfs_comp_metrics.vector_inserts);
    u64 current_insert_time = atomic64_read(&vexfs_comp_metrics.total_insert_time_ns);
    
    if (last_check_time > 0) {
        time_diff = current_time - last_check_time;
        
        if (time_diff > 5000000000ULL) { /* 5 seconds */
            u64 insert_diff = current_insert_count - last_insert_count;
            u64 time_diff_ns = current_insert_time - last_insert_time;
            
            if (insert_diff > 0 && time_diff_ns > 0) {
                u64 avg_latency = time_diff_ns / insert_diff;
                
                /* Check if average latency is significantly higher than minimum */
                u64 min_latency = atomic64_read(&vexfs_comp_metrics.min_insert_latency_ns);
                if (min_latency > 0 && avg_latency > (min_latency * 2)) {
                    vexfs_trace_performance_regression("vector_insert", avg_latency, min_latency);
                }
            }
            
            last_insert_count = current_insert_count;
            last_insert_time = current_insert_time;
            last_check_time = current_time;
        }
    } else {
        last_check_time = current_time;
        last_insert_count = current_insert_count;
        last_insert_time = current_insert_time;
    }
    
    /* Schedule next monitoring cycle */
    if (vexfs_monitoring_active) {
        queue_delayed_work(vexfs_monitoring_wq, &vexfs_monitoring_work, HZ * 5); /* Every 5 seconds */
    }
}

/**
 * Performance monitoring timer callback
 */
static void vexfs_perf_timer_callback(struct timer_list *timer)
{
    if (vexfs_monitoring_active && vexfs_monitoring_wq) {
        queue_delayed_work(vexfs_monitoring_wq, &vexfs_monitoring_work, 0);
    }
    
    /* Restart timer for next cycle */
    if (vexfs_monitoring_active) {
        mod_timer(&vexfs_perf_timer, jiffies + HZ * 10); /* Every 10 seconds */
    }
}

/*
 * Public API Functions
 */

/**
 * Initialize comprehensive performance monitoring
 */
int vexfs_comprehensive_monitoring_init(void)
{
    int ret = 0;
    
    vexfs_log_info("Initializing comprehensive performance monitoring");
    
    /* Initialize metrics structure */
    memset(&vexfs_comp_metrics, 0, sizeof(vexfs_comp_metrics));
    vexfs_comp_metrics.monitoring_start_time = ktime_get_ns();
    vexfs_comp_metrics.last_reset_time = vexfs_comp_metrics.monitoring_start_time;
    
    /* Create proc directory */
    vexfs_comp_proc_dir = proc_mkdir("vexfs_comp", NULL);
    if (!vexfs_comp_proc_dir) {
        vexfs_log_error("Failed to create proc directory");
        return -ENOMEM;
    }
    
    /* Create proc entries */
    vexfs_comp_metrics_proc = proc_create("metrics", 0444, vexfs_comp_proc_dir, 
                                         &vexfs_comp_metrics_proc_ops);
    if (!vexfs_comp_metrics_proc) {
        vexfs_log_error("Failed to create metrics proc entry");
        ret = -ENOMEM;
        goto cleanup_proc_dir;
    }
    
    vexfs_comp_config_proc = proc_create("config", 0644, vexfs_comp_proc_dir, 
                                        &vexfs_comp_config_proc_ops);
    if (!vexfs_comp_config_proc) {
        vexfs_log_error("Failed to create config proc entry");
        ret = -ENOMEM;
        goto cleanup_metrics_proc;
    }
    
    /* Create sysfs interface */
    vexfs_comp_kobj = kobject_create_and_add("vexfs_monitoring", kernel_kobj);
    if (!vexfs_comp_kobj) {
        vexfs_log_error("Failed to create sysfs kobject");
        ret = -ENOMEM;
        goto cleanup_config_proc;
    }
    
    ret = sysfs_create_group(vexfs_comp_kobj, &vexfs_comp_attr_group);
    if (ret) {
        vexfs_log_error("Failed to create sysfs attribute group");
        goto cleanup_kobject;
    }
    
    /* Create monitoring workqueue */
    vexfs_monitoring_wq = create_singlethread_workqueue("vexfs_monitoring");
    if (!vexfs_monitoring_wq) {
        vexfs_log_error("Failed to create monitoring workqueue");
        ret = -ENOMEM;
        goto cleanup_sysfs;
    }
    
    /* Initialize delayed work */
    INIT_DELAYED_WORK(&vexfs_monitoring_work, vexfs_monitoring_work_func);
    
    /* Initialize and start performance timer */
    timer_setup(&vexfs_perf_timer, vexfs_perf_timer_callback, 0);
    mod_timer(&vexfs_perf_timer, jiffies + HZ * 10); /* Start in 10 seconds */
    
    /* Start monitoring work */
    queue_delayed_work(vexfs_monitoring_wq, &vexfs_monitoring_work, HZ * 5);
    
    vexfs_log_info("Comprehensive performance monitoring initialized successfully");
    return 0;

cleanup_sysfs:
    sysfs_remove_group(vexfs_comp_kobj, &vexfs_comp_attr_group);
cleanup_kobject:
    kobject_put(vexfs_comp_kobj);
cleanup_config_proc:
    proc_remove(vexfs_comp_config_proc);
cleanup_metrics_proc:
    proc_remove(vexfs_comp_metrics_proc);
cleanup_proc_dir:
    proc_remove(vexfs_comp_proc_dir);
    
    vexfs_log_error("Failed to initialize comprehensive performance monitoring");
    return ret;
}

/**
 * Cleanup comprehensive performance monitoring
 */
void vexfs_comprehensive_monitoring_cleanup(void)
{
    vexfs_log_info("Cleaning up comprehensive performance monitoring");
    
    /* Stop monitoring */
    vexfs_monitoring_active = false;
    
    /* Stop and delete timer */
    del_timer_sync(&vexfs_perf_timer);
    
    /* Cancel and flush work */
    if (vexfs_monitoring_wq) {
        cancel_delayed_work_sync(&vexfs_monitoring_work);
        destroy_workqueue(vexfs_monitoring_wq);
        vexfs_monitoring_wq = NULL;
    }
    
    /* Remove sysfs interface */
    if (vexfs_comp_kobj) {
        sysfs_remove_group(vexfs_comp_kobj, &vexfs_comp_attr_group);
        kobject_put(vexfs_comp_kobj);
        vexfs_comp_kobj = NULL;
    }
    
    /* Remove proc entries */
    if (vexfs_comp_config_proc) {
        proc_remove(vexfs_comp_config_proc);
        vexfs_comp_config_proc = NULL;
    }
    
    if (vexfs_comp_metrics_proc) {
        proc_remove(vexfs_comp_metrics_proc);
        vexfs_comp_metrics_proc = NULL;
    }
    
    if (vexfs_comp_proc_dir) {
        proc_remove(vexfs_comp_proc_dir);
        vexfs_comp_proc_dir = NULL;
    }
    
    /* Clear metrics */
    memset(&vexfs_comp_metrics, 0, sizeof(vexfs_comp_metrics));
    
    vexfs_log_info("Comprehensive performance monitoring cleanup completed");
}

/**
 * Reset all monitoring counters
 */
void vexfs_reset_comprehensive_metrics(void)
{
    u64 current_time = ktime_get_ns();
    
    vexfs_log_info("Resetting comprehensive performance metrics");
    
    /* Preserve timing information */
    u64 start_time = vexfs_comp_metrics.monitoring_start_time;
    
    /* Clear all metrics */
    memset(&vexfs_comp_metrics, 0, sizeof(vexfs_comp_metrics));
    
    /* Restore timing information */
    vexfs_comp_metrics.monitoring_start_time = start_time;
    vexfs_comp_metrics.last_reset_time = current_time;
    vexfs_comp_metrics.last_update_time = current_time;
}

/**
 * Get comprehensive performance summary
 */
void vexfs_get_comprehensive_summary(char *buffer, size_t buffer_size)
{
    u64 total_ops, avg_latency, memory_efficiency, simd_efficiency;
    
    if (!buffer || buffer_size == 0) {
        return;
    }
    
    total_ops = atomic64_read(&vexfs_comp_metrics.vector_inserts) + 
                atomic64_read(&vexfs_comp_metrics.vector_searches);
    
    if (atomic64_read(&vexfs_comp_metrics.vector_inserts) > 0) {
        avg_latency = atomic64_read(&vexfs_comp_metrics.total_insert_time_ns) / 
                     atomic64_read(&vexfs_comp_metrics.vector_inserts);
    } else {
        avg_latency = 0;
    }
    
    if (atomic64_read(&vexfs_comp_metrics.total_memory_allocated) > 0) {
        memory_efficiency = (atomic64_read(&vexfs_comp_metrics.total_memory_freed) * 100) / 
                           atomic64_read(&vexfs_comp_metrics.total_memory_allocated);
    } else {
        memory_efficiency = 0;
    }
    
    simd_efficiency = atomic64_read(&vexfs_comp_metrics.simd_efficiency_percent);
    
    snprintf(buffer, buffer_size,
             "VexFS Comprehensive Performance Summary:\n"
             "Total Operations: %llu\n"
             "Average Insert Latency: %llu ns\n"
             "Current Memory Usage: %llu bytes\n"
             "Peak Memory Usage: %llu bytes\n"
             "Memory Efficiency: %llu%%\n"
             "SIMD Efficiency: %llu%%\n"
             "Allocation Failures: %llu\n"
             "Cache Hit Rate: %llu%%\n",
             total_ops,
             avg_latency,
             atomic64_read(&vexfs_comp_metrics.current_memory_usage),
             atomic64_read(&vexfs_comp_metrics.peak_memory_usage),
             memory_efficiency,
             simd_efficiency,
             atomic64_read(&vexfs_comp_metrics.allocation_failures),
             (atomic64_read(&vexfs_comp_metrics.cache_hits) * 100) / 
             (atomic64_read(&vexfs_comp_metrics.cache_hits) + atomic64_read(&vexfs_comp_metrics.cache_misses) + 1));
}

/*
 * Inline Helper Functions for Integration
 */

/**
 * Record cache hit/miss
 */
static inline void vexfs_record_cache_access(bool hit)
{
    if (vexfs_monitoring_active) {
        if (hit) {
            atomic64_inc(&vexfs_comp_metrics.cache_hits);
        } else {
            atomic64_inc(&vexfs_comp_metrics.cache_misses);
        }
    }
}

/**
 * Record prefetch hit/miss
 */
static inline void vexfs_record_prefetch_access(bool hit)
{
    if (vexfs_monitoring_active) {
        if (hit) {
            atomic64_inc(&vexfs_comp_metrics.prefetch_hits);
        } else {
            atomic64_inc(&vexfs_comp_metrics.prefetch_misses);
        }
    }
}

/**
 * Record error occurrence
 */
static inline void vexfs_record_error(int error_type)
{
    if (vexfs_monitoring_active) {
        switch (error_type) {
        case 1: /* Allocation failure */
            atomic64_inc(&vexfs_comp_metrics.allocation_failures);
            break;
        case 2: /* Validation error */
            atomic64_inc(&vexfs_comp_metrics.validation_errors);
            break;
        case 3: /* SIMD error */
            atomic64_inc(&vexfs_comp_metrics.simd_errors);
            break;
        case 4: /* Timeout error */
            atomic64_inc(&vexfs_comp_metrics.timeout_errors);
            break;
        }
    }
}

/* Export symbols for use by other VexFS modules */
EXPORT_SYMBOL(vexfs_trace_vector_insert);
EXPORT_SYMBOL(vexfs_trace_vector_insert_complete);
EXPORT_SYMBOL(vexfs_trace_vector_search);
EXPORT_SYMBOL(vexfs_trace_vector_search_complete);
EXPORT_SYMBOL(vexfs_trace_simd_operation);
EXPORT_SYMBOL(vexfs_trace_memory_allocation);
EXPORT_SYMBOL(vexfs_trace_memory_deallocation);
EXPORT_SYMBOL(vexfs_trace_performance_regression);
EXPORT_SYMBOL(vexfs_comprehensive_monitoring_init);
EXPORT_SYMBOL(vexfs_comprehensive_monitoring_cleanup);
EXPORT_SYMBOL(vexfs_reset_comprehensive_metrics);
EXPORT_SYMBOL(vexfs_get_comprehensive_summary);

MODULE_DESCRIPTION("VexFS v2.0 Comprehensive Performance Monitoring");
MODULE_AUTHOR("VexFS Development Team");
MODULE_LICENSE("GPL v2");