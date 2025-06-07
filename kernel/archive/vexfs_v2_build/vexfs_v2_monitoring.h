#ifndef VEXFS_V2_MONITORING_H
#define VEXFS_V2_MONITORING_H

#include <linux/atomic.h>
#include <linux/time.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>

/**
 * VexFS v2.0 Performance Monitoring Framework
 * 
 * Specialized monitoring for batch insert operations and vector performance
 * tracking to ensure continued achievement of 100K+ ops/sec targets.
 */

/* Performance Counters for Batch Insert Operations */
struct vexfs_batch_insert_metrics {
    /* Operation Counters */
    atomic64_t total_batch_operations;      /* Total batch insert calls */
    atomic64_t successful_batch_operations; /* Successful batch inserts */
    atomic64_t failed_batch_operations;     /* Failed batch inserts */
    atomic64_t total_vectors_processed;     /* Total vectors inserted */
    
    /* Performance Metrics */
    atomic64_t total_processing_time_ns;    /* Total processing time in nanoseconds */
    atomic64_t min_latency_ns;              /* Minimum operation latency */
    atomic64_t max_latency_ns;              /* Maximum operation latency */
    atomic64_t last_throughput_ops_sec;     /* Last measured throughput */
    
    /* Memory Usage Tracking */
    atomic64_t total_memory_allocated;      /* Total memory allocated for batches */
    atomic64_t peak_memory_usage;           /* Peak memory usage */
    atomic64_t vmalloc_allocations;         /* Number of vmalloc allocations */
    atomic64_t kmalloc_allocations;         /* Number of kmalloc allocations */
    
    /* Optimization Tracking */
    atomic64_t bulk_copy_operations;        /* Bulk copy operations performed */
    atomic64_t scalar_validations;          /* Scalar validations performed */
    atomic64_t simd_batch_optimizations;    /* SIMD batch optimizations used */
    atomic64_t cache_hits;                  /* Cache hits during processing */
    atomic64_t cache_misses;                /* Cache misses during processing */
    
    /* Error Analysis */
    atomic64_t validation_errors;           /* Vector validation errors */
    atomic64_t memory_allocation_errors;    /* Memory allocation failures */
    atomic64_t copy_from_user_errors;       /* copy_from_user failures */
    atomic64_t ioctl_structure_errors;      /* Ioctl structure errors */
    
    /* Batch Size Analysis */
    atomic64_t small_batches;               /* Batches < 64 vectors */
    atomic64_t medium_batches;              /* Batches 64-256 vectors */
    atomic64_t large_batches;               /* Batches > 256 vectors */
    atomic64_t optimal_batch_count;         /* Batches using optimal size (256) */
    
    /* Performance Targets Tracking */
    atomic64_t target_achievements;         /* Times 100K ops/sec target was met */
    atomic64_t target_misses;               /* Times target was missed */
    atomic64_t performance_regressions;     /* Detected performance regressions */
    
    /* Timing Statistics */
    u64 last_measurement_time;              /* Last performance measurement */
    u64 monitoring_start_time;              /* Monitoring start timestamp */
};

/* Global Vector Operations Metrics */
struct vexfs_vector_metrics {
    /* Vector Metadata Operations */
    atomic64_t metadata_operations;         /* Total metadata operations */
    atomic64_t metadata_successes;          /* Successful metadata operations */
    atomic64_t metadata_failures;           /* Failed metadata operations */
    atomic64_t metadata_avg_latency_ns;     /* Average metadata latency */
    
    /* Vector Search Operations */
    atomic64_t search_operations;           /* Total search operations */
    atomic64_t search_successes;            /* Successful search operations */
    atomic64_t search_failures;             /* Failed search operations */
    atomic64_t search_avg_latency_ns;       /* Average search latency */
    
    /* HNSW Algorithm Metrics */
    atomic64_t hnsw_graph_builds;           /* HNSW graph constructions */
    atomic64_t hnsw_node_allocations;       /* HNSW node allocations */
    atomic64_t hnsw_layer_traversals;       /* Layer traversals performed */
    atomic64_t hnsw_distance_calculations;  /* Distance calculations */
    
    /* SIMD Performance Tracking */
    atomic64_t avx2_operations;             /* AVX2 operations performed */
    atomic64_t sse2_fallback_operations;    /* SSE2 fallback operations */
    atomic64_t scalar_operations;           /* Scalar operations performed */
    atomic64_t simd_optimization_hits;      /* SIMD optimizations applied */
};

/* Performance Thresholds and Targets */
#define VEXFS_TARGET_BATCH_INSERT_OPS_SEC   100000  /* 100K ops/sec target */
#define VEXFS_TARGET_METADATA_OPS_SEC       100000  /* 100K ops/sec target */
#define VEXFS_TARGET_SEARCH_OPS_SEC         100000  /* 100K ops/sec target */
#define VEXFS_MAX_ACCEPTABLE_LATENCY_NS     1000000 /* 1ms max latency */
#define VEXFS_PERFORMANCE_REGRESSION_THRESHOLD 0.9  /* 90% of previous performance */

/* Monitoring Control Flags */
#define VEXFS_MONITORING_ENABLED            1
#define VEXFS_DETAILED_TIMING_ENABLED       2
#define VEXFS_MEMORY_TRACKING_ENABLED       4
#define VEXFS_REGRESSION_DETECTION_ENABLED  8
#define VEXFS_CACHE_ANALYSIS_ENABLED        16

/* Global monitoring instances */
extern struct vexfs_batch_insert_metrics vexfs_batch_metrics;
extern struct vexfs_vector_metrics vexfs_vector_metrics;
extern unsigned int vexfs_monitoring_flags;

/* Monitoring Function Declarations */

/**
 * Initialize the VexFS monitoring system
 */
int vexfs_monitoring_init(void);

/**
 * Cleanup the VexFS monitoring system
 */
void vexfs_monitoring_cleanup(void);

/**
 * Record a batch insert operation
 */
void vexfs_record_batch_insert(u32 vector_count, u64 processing_time_ns, 
                              size_t memory_used, bool success);

/**
 * Record vector metadata operation
 */
void vexfs_record_metadata_operation(u64 latency_ns, bool success);

/**
 * Record vector search operation
 */
void vexfs_record_search_operation(u64 latency_ns, bool success);

/**
 * Calculate and update throughput metrics
 */
void vexfs_update_throughput_metrics(void);

/**
 * Check for performance regressions
 */
bool vexfs_check_performance_regression(void);

/**
 * Reset all monitoring counters
 */
void vexfs_reset_monitoring_counters(void);

/**
 * Get current performance summary
 */
void vexfs_get_performance_summary(char *buffer, size_t buffer_size);

/* Inline helper functions for fast path monitoring */

/**
 * Record batch insert start - minimal overhead
 */
static inline u64 vexfs_batch_insert_start(void)
{
    if (likely(vexfs_monitoring_flags & VEXFS_MONITORING_ENABLED)) {
        return ktime_get_ns();
    }
    return 0;
}

/**
 * Record batch insert completion - minimal overhead
 */
static inline void vexfs_batch_insert_end(u64 start_time, u32 vector_count, 
                                         size_t memory_used, bool success)
{
    if (likely(vexfs_monitoring_flags & VEXFS_MONITORING_ENABLED)) {
        u64 processing_time = ktime_get_ns() - start_time;
        vexfs_record_batch_insert(vector_count, processing_time, memory_used, success);
    }
}

/**
 * Record memory allocation for monitoring
 */
static inline void vexfs_record_memory_allocation(size_t size, bool is_vmalloc)
{
    if (likely(vexfs_monitoring_flags & VEXFS_MEMORY_TRACKING_ENABLED)) {
        atomic64_add(size, &vexfs_batch_metrics.total_memory_allocated);
        if (is_vmalloc) {
            atomic64_inc(&vexfs_batch_metrics.vmalloc_allocations);
        } else {
            atomic64_inc(&vexfs_batch_metrics.kmalloc_allocations);
        }
    }
}

/**
 * Record optimization usage
 */
static inline void vexfs_record_optimization(int optimization_type)
{
    if (likely(vexfs_monitoring_flags & VEXFS_MONITORING_ENABLED)) {
        switch (optimization_type) {
        case 1: /* Bulk copy */
            atomic64_inc(&vexfs_batch_metrics.bulk_copy_operations);
            break;
        case 2: /* Scalar validation */
            atomic64_inc(&vexfs_batch_metrics.scalar_validations);
            break;
        case 3: /* SIMD batch optimization */
            atomic64_inc(&vexfs_batch_metrics.simd_batch_optimizations);
            break;
        }
    }
}

/**
 * Record batch size category
 */
static inline void vexfs_record_batch_size(u32 vector_count)
{
    if (likely(vexfs_monitoring_flags & VEXFS_MONITORING_ENABLED)) {
        if (vector_count < 64) {
            atomic64_inc(&vexfs_batch_metrics.small_batches);
        } else if (vector_count <= 256) {
            atomic64_inc(&vexfs_batch_metrics.medium_batches);
            if (vector_count == 256) {
                atomic64_inc(&vexfs_batch_metrics.optimal_batch_count);
            }
        } else {
            atomic64_inc(&vexfs_batch_metrics.large_batches);
        }
    }
}

/**
 * Check if performance target is being met
 */
static inline void vexfs_check_performance_target(u64 ops_per_sec)
{
    if (likely(vexfs_monitoring_flags & VEXFS_MONITORING_ENABLED)) {
        if (ops_per_sec >= VEXFS_TARGET_BATCH_INSERT_OPS_SEC) {
            atomic64_inc(&vexfs_batch_metrics.target_achievements);
        } else {
            atomic64_inc(&vexfs_batch_metrics.target_misses);
        }
    }
}

/* Proc filesystem interface for monitoring */
extern struct proc_dir_entry *vexfs_proc_dir;
extern struct proc_dir_entry *vexfs_batch_metrics_proc;
extern struct proc_dir_entry *vexfs_vector_metrics_proc;
extern struct proc_dir_entry *vexfs_performance_summary_proc;

#endif /* VEXFS_V2_MONITORING_H */