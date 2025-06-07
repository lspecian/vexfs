/*
 * VexFS v2.0 Comprehensive Performance Monitoring Header
 * 
 * Task 57: Implement Comprehensive Performance Monitoring
 * 
 * This header defines the comprehensive performance monitoring interface
 * for VexFS v2.0, including tracepoints, configurable logging, and
 * enhanced statistics collection.
 * 
 * Features:
 * - Tracepoint definitions for detailed performance analysis
 * - Configurable logging levels for debugging
 * - Enhanced vector operation counters
 * - Memory usage tracking with atomic counters
 * - Proc/sysfs interface declarations
 * - Performance regression detection
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_V2_COMPREHENSIVE_MONITORING_H
#define VEXFS_V2_COMPREHENSIVE_MONITORING_H

#ifdef __KERNEL__
#include <linux/types.h>
#include <linux/atomic.h>
#include <linux/tracepoint.h>
#include <linux/proc_fs.h>
#include <linux/kobject.h>
#else
#include <stdint.h>
/* Userspace type compatibility */
typedef uint8_t  __u8;
typedef uint16_t __u16;
typedef uint32_t __u32;
typedef uint64_t __u64;
typedef int8_t   __s8;
typedef int16_t  __s16;
typedef int32_t  __s32;
typedef int64_t  __s64;
#endif

/*
 * Tracepoint Declarations
 */

/* Vector operation tracepoints */
DECLARE_TRACE(vexfs_vector_insert_start,
    TP_PROTO(u32 vector_id, u32 dimensions, u64 start_time_ns),
    TP_ARGS(vector_id, dimensions, start_time_ns));

DECLARE_TRACE(vexfs_vector_insert_end,
    TP_PROTO(u32 vector_id, u64 duration_ns, bool success),
    TP_ARGS(vector_id, duration_ns, success));

DECLARE_TRACE(vexfs_vector_search_start,
    TP_PROTO(u32 query_dims, u32 k_neighbors, u64 start_time_ns),
    TP_ARGS(query_dims, k_neighbors, start_time_ns));

DECLARE_TRACE(vexfs_vector_search_end,
    TP_PROTO(u32 results_found, u64 duration_ns, bool success),
    TP_ARGS(results_found, duration_ns, success));

DECLARE_TRACE(vexfs_vector_quantize_start,
    TP_PROTO(u32 vector_count, u32 quantization_type, u64 start_time_ns),
    TP_ARGS(vector_count, quantization_type, start_time_ns));

DECLARE_TRACE(vexfs_vector_quantize_end,
    TP_PROTO(u32 vector_count, u64 duration_ns, bool success),
    TP_ARGS(vector_count, duration_ns, success));

DECLARE_TRACE(vexfs_simd_operation,
    TP_PROTO(const char *operation, u32 simd_type, u32 vector_count, u64 duration_ns),
    TP_ARGS(operation, simd_type, vector_count, duration_ns));

DECLARE_TRACE(vexfs_memory_allocation,
    TP_PROTO(size_t size, bool is_vmalloc, bool success),
    TP_ARGS(size, is_vmalloc, success));

DECLARE_TRACE(vexfs_performance_regression,
    TP_PROTO(const char *operation, u64 current_perf, u64 baseline_perf),
    TP_ARGS(operation, current_perf, baseline_perf));

/*
 * Logging Level Definitions
 */
enum vexfs_log_level {
    VEXFS_LOG_NONE = 0,     /* No logging */
    VEXFS_LOG_ERROR = 1,    /* Error messages only */
    VEXFS_LOG_WARN = 2,     /* Warnings and errors */
    VEXFS_LOG_INFO = 3,     /* Informational messages */
    VEXFS_LOG_DEBUG = 4,    /* Debug messages */
    VEXFS_LOG_TRACE = 5     /* Trace-level messages */
};

/*
 * Comprehensive Performance Metrics Structure
 */
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
    
    /* Timing Information */
    u64 monitoring_start_time;              /* Monitoring start timestamp */
    u64 last_reset_time;                    /* Last reset timestamp */
    u64 last_update_time;                   /* Last update timestamp */
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Configuration Structure for Runtime Settings
 */
struct vexfs_monitoring_config {
    enum vexfs_log_level log_level;         /* Current logging level */
    bool tracing_enabled;                   /* Tracepoint tracing enabled */
    bool monitoring_active;                 /* Performance monitoring active */
    bool regression_detection;              /* Performance regression detection */
    u32 monitoring_interval_ms;             /* Monitoring update interval */
    u32 regression_threshold_percent;       /* Regression detection threshold */
    u32 reserved[2];                        /* Reserved for future use */
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Performance Summary Structure for Quick Access
 */
struct vexfs_performance_summary {
    u64 total_operations;                   /* Total vector operations */
    u64 average_insert_latency_ns;         /* Average insert latency */
    u64 average_search_latency_ns;         /* Average search latency */
    u64 current_memory_usage_bytes;        /* Current memory usage */
    u64 peak_memory_usage_bytes;           /* Peak memory usage */
    u64 memory_efficiency_percent;         /* Memory allocation efficiency */
    u64 simd_efficiency_percent;           /* SIMD usage efficiency */
    u64 cache_hit_rate_percent;            /* Cache hit rate */
    u64 error_rate_percent;                /* Overall error rate */
    u64 uptime_seconds;                     /* Monitoring uptime */
    u32 reserved[4];                        /* Reserved for future use */
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * IOCTL Commands for Comprehensive Monitoring
 */
#define VEXFS_IOC_COMP_GET_METRICS      _IOR('V', 0x70, struct vexfs_comprehensive_metrics)
#define VEXFS_IOC_COMP_GET_CONFIG       _IOR('V', 0x71, struct vexfs_monitoring_config)
#define VEXFS_IOC_COMP_SET_CONFIG       _IOW('V', 0x72, struct vexfs_monitoring_config)
#define VEXFS_IOC_COMP_GET_SUMMARY      _IOR('V', 0x73, struct vexfs_performance_summary)
#define VEXFS_IOC_COMP_RESET_METRICS    _IO('V', 0x74)
#define VEXFS_IOC_COMP_ENABLE_TRACING   _IOW('V', 0x75, bool)
#define VEXFS_IOC_COMP_SET_LOG_LEVEL    _IOW('V', 0x76, enum vexfs_log_level)

/*
 * Function Declarations
 */

#ifdef __KERNEL__

/* Initialization and cleanup */
int vexfs_comprehensive_monitoring_init(void);
void vexfs_comprehensive_monitoring_cleanup(void);

/* Tracepoint functions */
void vexfs_trace_vector_insert(u32 vector_id, u32 dimensions, u64 start_time_ns);
void vexfs_trace_vector_insert_complete(u32 vector_id, u64 duration_ns, bool success);
void vexfs_trace_vector_search(u32 query_dims, u32 k_neighbors, u64 start_time_ns);
void vexfs_trace_vector_search_complete(u32 results_found, u64 duration_ns, bool success);
void vexfs_trace_simd_operation(const char *operation, u32 simd_type, u32 vector_count, u64 duration_ns);
void vexfs_trace_memory_allocation(size_t size, bool is_vmalloc, bool success);
void vexfs_trace_memory_deallocation(size_t size, bool is_vfree);
void vexfs_trace_performance_regression(const char *operation, u64 current_perf, u64 baseline_perf);

/* Configuration and control */
void vexfs_reset_comprehensive_metrics(void);
void vexfs_get_comprehensive_summary(char *buffer, size_t buffer_size);

/* Inline helper macros for fast path monitoring */
#define VEXFS_TRACE_INSERT_START(id, dims) \
    do { \
        u64 start_time = ktime_get_ns(); \
        vexfs_trace_vector_insert(id, dims, start_time); \
    } while (0)

#define VEXFS_TRACE_INSERT_END(id, start_time, success) \
    do { \
        u64 duration = ktime_get_ns() - start_time; \
        vexfs_trace_vector_insert_complete(id, duration, success); \
    } while (0)

#define VEXFS_TRACE_SEARCH_START(dims, k) \
    do { \
        u64 start_time = ktime_get_ns(); \
        vexfs_trace_vector_search(dims, k, start_time); \
    } while (0)

#define VEXFS_TRACE_SEARCH_END(results, start_time, success) \
    do { \
        u64 duration = ktime_get_ns() - start_time; \
        vexfs_trace_vector_search_complete(results, duration, success); \
    } while (0)

#define VEXFS_TRACE_SIMD_OP(op, type, count) \
    do { \
        u64 start_time = ktime_get_ns(); \
        /* Operation happens here */ \
        u64 duration = ktime_get_ns() - start_time; \
        vexfs_trace_simd_operation(op, type, count, duration); \
    } while (0)

#define VEXFS_TRACE_ALLOC(size, vmalloc, success) \
    vexfs_trace_memory_allocation(size, vmalloc, success)

#define VEXFS_TRACE_FREE(size, vfree) \
    vexfs_trace_memory_deallocation(size, vfree)

/* Error type constants for error recording */
#define VEXFS_ERROR_ALLOCATION_FAILURE  1
#define VEXFS_ERROR_VALIDATION_ERROR    2
#define VEXFS_ERROR_SIMD_ERROR          3
#define VEXFS_ERROR_TIMEOUT_ERROR       4

/* SIMD type constants for tracing */
#define VEXFS_SIMD_TYPE_NONE    0
#define VEXFS_SIMD_TYPE_SSE2    1
#define VEXFS_SIMD_TYPE_AVX2    2
#define VEXFS_SIMD_TYPE_AVX512  3

/* Global monitoring state (extern declarations) */
extern bool vexfs_comprehensive_monitoring_enabled;
extern enum vexfs_log_level vexfs_current_log_level;
extern bool vexfs_tracing_enabled;

#endif /* __KERNEL__ */

/*
 * Userspace Interface Definitions
 */

/* Proc filesystem paths */
#define VEXFS_PROC_COMP_DIR         "/proc/vexfs_comp"
#define VEXFS_PROC_COMP_METRICS     "/proc/vexfs_comp/metrics"
#define VEXFS_PROC_COMP_CONFIG      "/proc/vexfs_comp/config"

/* Sysfs filesystem paths */
#define VEXFS_SYSFS_MONITORING_DIR  "/sys/kernel/vexfs_monitoring"
#define VEXFS_SYSFS_METRICS         "/sys/kernel/vexfs_monitoring/metrics"
#define VEXFS_SYSFS_LOG_LEVEL       "/sys/kernel/vexfs_monitoring/log_level"
#define VEXFS_SYSFS_TRACING         "/sys/kernel/vexfs_monitoring/tracing"

/* Trace-cmd integration commands */
#define VEXFS_TRACE_ENABLE_CMD      "echo 1 > /sys/kernel/debug/tracing/events/vexfs/enable"
#define VEXFS_TRACE_DISABLE_CMD     "echo 0 > /sys/kernel/debug/tracing/events/vexfs/enable"
#define VEXFS_TRACE_START_CMD       "trace-cmd record -e vexfs:*"
#define VEXFS_TRACE_REPORT_CMD      "trace-cmd report"

/*
 * Performance Monitoring Constants
 */
#define VEXFS_MONITORING_VERSION            1
#define VEXFS_MAX_LOG_MESSAGE_SIZE          256
#define VEXFS_PERFORMANCE_HISTORY_SIZE      100
#define VEXFS_DEFAULT_MONITORING_INTERVAL   5000    /* 5 seconds in ms */
#define VEXFS_DEFAULT_REGRESSION_THRESHOLD  90      /* 90% of baseline */
#define VEXFS_MAX_TRACEPOINT_NAME_LEN       64

/*
 * Utility Macros for Performance Calculations
 */
#define VEXFS_CALC_PERCENTAGE(part, total) \
    ((total) > 0 ? ((part) * 100) / (total) : 0)

#define VEXFS_CALC_AVERAGE(total, count) \
    ((count) > 0 ? (total) / (count) : 0)

#define VEXFS_NS_TO_MS(ns) ((ns) / 1000000ULL)
#define VEXFS_NS_TO_US(ns) ((ns) / 1000ULL)
#define VEXFS_MS_TO_NS(ms) ((ms) * 1000000ULL)
#define VEXFS_US_TO_NS(us) ((us) * 1000ULL)

#endif /* VEXFS_V2_COMPREHENSIVE_MONITORING_H */