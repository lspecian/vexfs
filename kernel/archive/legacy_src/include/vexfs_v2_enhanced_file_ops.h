/*
 * VexFS v2.0 Enhanced File Operations
 * 
 * Vector-optimized file operations with SIMD acceleration, memory mapping,
 * and intelligent readahead strategies for optimal vector database performance.
 * 
 * Features:
 * - SIMD-accelerated read/write operations
 * - Direct memory mapping with proper alignment
 * - Vector-aware readahead strategies
 * - Optimized user-kernel space data transfers
 * - Concurrent access optimization
 */

#ifndef VEXFS_V2_ENHANCED_FILE_OPS_H
#define VEXFS_V2_ENHANCED_FILE_OPS_H

#include <linux/types.h>
#include <linux/fs.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/uaccess.h>
#include <linux/prefetch.h>
#include <linux/highmem.h>
#include <linux/pagemap.h>
#include <linux/writeback.h>
#include <asm/fpu/api.h>

/* Vector data transfer optimization flags */
#define VEXFS_TRANSFER_SIMD_ALIGNED     (1 << 0)
#define VEXFS_TRANSFER_NUMA_LOCAL       (1 << 1)
#define VEXFS_TRANSFER_PREFETCH_ENABLED (1 << 2)
#define VEXFS_TRANSFER_BATCH_OPTIMIZED  (1 << 3)
#define VEXFS_TRANSFER_ZERO_COPY        (1 << 4)

/* Vector file access patterns */
typedef enum {
    VEXFS_ACCESS_SEQUENTIAL = 0,    /* Sequential vector access */
    VEXFS_ACCESS_RANDOM = 1,        /* Random vector access */
    VEXFS_ACCESS_BATCH = 2,         /* Batch vector operations */
    VEXFS_ACCESS_STREAMING = 3,     /* Streaming vector data */
    VEXFS_ACCESS_SEARCH = 4,        /* Vector search operations */
    VEXFS_ACCESS_UPDATE = 5         /* Vector update operations */
} vexfs_access_pattern_t;

/* Vector data transfer context */
struct vexfs_transfer_context {
    /* Transfer configuration */
    u32 flags;                      /* Transfer optimization flags */
    u32 vector_alignment;           /* Required vector alignment */
    u32 batch_size;                 /* Optimal batch size */
    u32 prefetch_size;              /* Prefetch size in vectors */
    
    /* SIMD configuration */
    u32 simd_capabilities;          /* Available SIMD instructions */
    u32 simd_vector_width;          /* SIMD vector width in bits */
    bool simd_enabled;              /* SIMD acceleration enabled */
    
    /* NUMA configuration */
    int numa_node;                  /* Preferred NUMA node */
    bool numa_aware;                /* NUMA-aware allocation */
    
    /* Access pattern tracking */
    vexfs_access_pattern_t pattern; /* Detected access pattern */
    u64 last_offset;                /* Last accessed offset */
    u64 access_count;               /* Total access count */
    u64 sequential_count;           /* Sequential access count */
    
    /* Performance counters */
    u64 bytes_transferred;          /* Total bytes transferred */
    u64 simd_operations;            /* SIMD operations performed */
    u64 cache_hits;                 /* Cache hits */
    u64 cache_misses;               /* Cache misses */
};

/* Vector readahead context */
struct vexfs_readahead_context {
    /* Readahead configuration */
    u32 window_size;                /* Readahead window size */
    u32 max_vectors;                /* Maximum vectors to readahead */
    u32 trigger_threshold;          /* Trigger threshold for readahead */
    
    /* Pattern detection */
    vexfs_access_pattern_t pattern; /* Detected access pattern */
    u64 stride_size;                /* Detected stride size */
    u64 last_offset;                /* Last accessed offset */
    
    /* Readahead state */
    u64 next_offset;                /* Next offset to readahead */
    u32 pending_requests;           /* Pending readahead requests */
    bool active;                    /* Readahead active */
    
    /* Performance tracking */
    u64 readahead_hits;             /* Readahead hits */
    u64 readahead_misses;           /* Readahead misses */
    u64 bytes_readahead;            /* Total bytes read ahead */
};

/* Vector memory mapping context */
struct vexfs_mmap_context {
    /* Mapping configuration */
    u32 alignment;                  /* Memory alignment requirement */
    u32 page_order;                 /* Page allocation order */
    bool huge_pages;                /* Use huge pages if available */
    bool numa_local;                /* NUMA-local allocation */
    
    /* Mapping state */
    void *kernel_addr;              /* Kernel virtual address */
    dma_addr_t dma_addr;            /* DMA address if applicable */
    u32 mapping_flags;              /* Mapping flags */
    
    /* Access tracking */
    u64 access_count;               /* Access count */
    u64 last_access_time;           /* Last access timestamp */
    vexfs_access_pattern_t pattern; /* Access pattern */
    
    /* Performance counters */
    u64 page_faults;                /* Page fault count */
    u64 tlb_misses;                 /* TLB miss count */
};

/* Enhanced file operations structure */
struct vexfs_enhanced_file_ops {
    /* Standard file operations */
    const struct file_operations *base_ops;
    
    /* Vector-enhanced operations */
    ssize_t (*vector_read)(struct file *file, char __user *buf,
                          size_t count, loff_t *ppos,
                          struct vexfs_transfer_context *ctx);
    ssize_t (*vector_write)(struct file *file, const char __user *buf,
                           size_t count, loff_t *ppos,
                           struct vexfs_transfer_context *ctx);
    
    /* Memory mapping operations */
    int (*vector_mmap)(struct file *file, struct vm_area_struct *vma,
                      struct vexfs_mmap_context *ctx);
    void (*vector_munmap)(struct vm_area_struct *vma,
                         struct vexfs_mmap_context *ctx);
    
    /* Readahead operations */
    int (*vector_readahead)(struct file *file, loff_t offset, size_t count,
                           struct vexfs_readahead_context *ctx);
    void (*update_readahead)(struct file *file, loff_t offset, size_t count,
                            struct vexfs_readahead_context *ctx);
    
    /* Batch operations */
    ssize_t (*batch_read)(struct file *file, struct iovec *iov, int iovcnt,
                         loff_t *ppos, struct vexfs_transfer_context *ctx);
    ssize_t (*batch_write)(struct file *file, const struct iovec *iov, int iovcnt,
                          loff_t *ppos, struct vexfs_transfer_context *ctx);
    
    /* Direct I/O operations */
    ssize_t (*direct_read)(struct file *file, char __user *buf,
                          size_t count, loff_t *ppos,
                          struct vexfs_transfer_context *ctx);
    ssize_t (*direct_write)(struct file *file, const char __user *buf,
                           size_t count, loff_t *ppos,
                           struct vexfs_transfer_context *ctx);
    
    /* Synchronization operations */
    int (*vector_fsync)(struct file *file, loff_t start, loff_t end, int datasync);
    int (*vector_flush)(struct file *file, fl_owner_t id);
    
    /* Performance monitoring */
    void (*get_stats)(struct file *file, struct vexfs_transfer_context *ctx);
    void (*reset_stats)(struct file *file);
};

/* Function declarations */

/* Enhanced file operations initialization */
int vexfs_init_enhanced_file_ops(struct super_block *sb);
void vexfs_cleanup_enhanced_file_ops(struct super_block *sb);

/* Transfer context management */
int vexfs_init_transfer_context(struct vexfs_transfer_context *ctx,
                               struct file *file);
void vexfs_cleanup_transfer_context(struct vexfs_transfer_context *ctx);
void vexfs_update_transfer_context(struct vexfs_transfer_context *ctx,
                                  loff_t offset, size_t count);

/* Readahead context management */
int vexfs_init_readahead_context(struct vexfs_readahead_context *ctx,
                                struct file *file);
void vexfs_cleanup_readahead_context(struct vexfs_readahead_context *ctx);
void vexfs_update_readahead_pattern(struct vexfs_readahead_context *ctx,
                                   loff_t offset, size_t count);

/* Memory mapping context management */
int vexfs_init_mmap_context(struct vexfs_mmap_context *ctx,
                           struct vm_area_struct *vma);
void vexfs_cleanup_mmap_context(struct vexfs_mmap_context *ctx);

/* Enhanced read/write operations */
ssize_t vexfs_enhanced_read(struct file *file, char __user *buf,
                           size_t count, loff_t *ppos);
ssize_t vexfs_enhanced_write(struct file *file, const char __user *buf,
                            size_t count, loff_t *ppos);

/* SIMD-accelerated data transfer */
ssize_t vexfs_simd_copy_to_user(char __user *dst, const void *src,
                                size_t count, u32 alignment,
                                u32 simd_capabilities);
ssize_t vexfs_simd_copy_from_user(void *dst, const char __user *src,
                                  size_t count, u32 alignment,
                                  u32 simd_capabilities);

/* Memory mapping operations */
int vexfs_enhanced_mmap(struct file *file, struct vm_area_struct *vma);
vm_fault_t vexfs_enhanced_fault(struct vm_fault *vmf);
void vexfs_enhanced_close(struct vm_area_struct *vma);

/* Readahead operations */
int vexfs_vector_readahead(struct file *file, loff_t offset, size_t count);
void vexfs_update_access_pattern(struct file *file, loff_t offset, size_t count);

/* Batch operations */
ssize_t vexfs_batch_read_vectors(struct file *file, struct iovec *iov,
                                int iovcnt, loff_t *ppos);
ssize_t vexfs_batch_write_vectors(struct file *file, const struct iovec *iov,
                                 int iovcnt, loff_t *ppos);

/* Direct I/O operations */
ssize_t vexfs_direct_read_vectors(struct file *file, char __user *buf,
                                 size_t count, loff_t *ppos);
ssize_t vexfs_direct_write_vectors(struct file *file, const char __user *buf,
                                  size_t count, loff_t *ppos);

/* Synchronization operations */
int vexfs_enhanced_fsync(struct file *file, loff_t start, loff_t end, int datasync);
int vexfs_enhanced_flush(struct file *file, fl_owner_t id);

/* Access pattern detection */
vexfs_access_pattern_t vexfs_detect_access_pattern(struct file *file,
                                                  loff_t offset, size_t count);
void vexfs_update_access_stats(struct file *file, loff_t offset, size_t count,
                              vexfs_access_pattern_t pattern);

/* Performance optimization */
u32 vexfs_calculate_optimal_batch_size(struct file *file, size_t count);
u32 vexfs_calculate_optimal_alignment(struct file *file, size_t count);
bool vexfs_should_use_simd(struct file *file, size_t count);
bool vexfs_should_prefetch(struct file *file, loff_t offset, size_t count);

/* NUMA optimization */
int vexfs_get_optimal_numa_node(struct file *file);
void *vexfs_numa_alloc_aligned(size_t size, u32 alignment, int node);
void vexfs_numa_free_aligned(void *ptr, size_t size);

/* Cache management */
void vexfs_prefetch_vectors(struct file *file, loff_t offset, size_t count);
void vexfs_invalidate_vector_cache(struct file *file, loff_t offset, size_t count);
void vexfs_flush_vector_cache(struct file *file);

/* Error handling and debugging */
void vexfs_report_transfer_error(struct file *file, int error,
                                const char *operation, loff_t offset, size_t count);
void vexfs_log_performance_stats(struct file *file,
                                const struct vexfs_transfer_context *ctx);

/* Utility functions */
bool vexfs_is_vector_aligned(loff_t offset, size_t count, u32 alignment);
u32 vexfs_round_up_to_alignment(u32 value, u32 alignment);
size_t vexfs_calculate_transfer_size(size_t requested, u32 alignment, u32 batch_size);

/* File operations structure */
extern const struct file_operations vexfs_enhanced_file_operations;
extern const struct vm_operations_struct vexfs_enhanced_vm_operations;

#endif /* VEXFS_V2_ENHANCED_FILE_OPS_H */