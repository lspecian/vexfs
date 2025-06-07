/*
 * VexFS v2.0 SIMD-Accelerated Vector Processing Header
 * 
 * This header defines SIMD-accelerated functions for vector normalization
 * and quantization in kernel space, implementing Task 49 requirements.
 * 
 * Features:
 * - L2 normalization using SIMD instructions
 * - Scalar quantization (float32 to int8/uint8)
 * - Product quantization with codebook generation
 * - Binary quantization for compact storage
 * - Proper kernel FPU handling with fallback scalar versions
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_V2_VECTOR_PROCESSING_H
#define VEXFS_V2_VECTOR_PROCESSING_H

#ifdef __KERNEL__
#include <linux/types.h>
#include <linux/kernel.h>
#include <asm/fpu/api.h>
#include <asm/cpufeature.h>
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

#include "vexfs_v2_uapi.h"

/*
 * SIMD Capability Detection
 */
#define VEXFS_SIMD_NONE     0x00
#define VEXFS_SIMD_SSE2     0x01
#define VEXFS_SIMD_AVX2     0x02
#define VEXFS_SIMD_AVX512   0x04
#define VEXFS_SIMD_NEON     0x08

/*
 * Vector Processing Operation Types
 */
#define VEXFS_OP_L2_NORMALIZE       0x01
#define VEXFS_OP_SCALAR_QUANTIZE    0x02
#define VEXFS_OP_PRODUCT_QUANTIZE   0x03
#define VEXFS_OP_BINARY_QUANTIZE    0x04

/*
 * Quantization Types
 */
#define VEXFS_QUANT_INT8            0x01
#define VEXFS_QUANT_UINT8           0x02
#define VEXFS_QUANT_INT16           0x03
#define VEXFS_QUANT_UINT16          0x04

/*
 * Product Quantization Configuration
 */
struct vexfs_pq_config {
    __u32 subvector_count;      /* Number of subvectors */
    __u32 subvector_dims;       /* Dimensions per subvector */
    __u32 codebook_size;        /* Size of each codebook (typically 256) */
    __u32 training_iterations;  /* K-means training iterations */
    __u32 reserved[4];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Vector Processing Request Structure
 */
struct vexfs_vector_processing_request {
    __u32 operation_type;       /* VEXFS_OP_* */
    __u32 input_format;         /* VEXFS_VECTOR_* */
    __u32 output_format;        /* VEXFS_VECTOR_* */
    __u32 dimensions;           /* Vector dimensions */
    __u32 vector_count;         /* Number of vectors to process */
    
    /* Input data (IEEE 754 bits for kernel compatibility) */
    __u32 *input_vectors_bits;  /* Input vector data */
    
    /* Output data */
    union {
        __u32 *output_vectors_bits;     /* For normalization */
        __u8  *quantized_int8;          /* For int8 quantization */
        __u8  *quantized_uint8;         /* For uint8 quantization */
        __u8  *binary_codes;            /* For binary quantization */
        __u8  *pq_codes;                /* For product quantization */
    } output;
    
    /* Configuration for specific operations */
    union {
        struct {
            __u32 scale_factor_bits;    /* Scaling factor (IEEE 754 bits) */
            __u32 offset_bits;          /* Offset value (IEEE 754 bits) */
        } scalar_quant;
        
        struct vexfs_pq_config pq;
        
        struct {
            __u32 threshold_bits;       /* Binary threshold (IEEE 754 bits) */
        } binary_quant;
    } config;
    
    /* Performance metrics */
    __u64 processing_time_ns;   /* Processing time */
    __u32 simd_level_used;      /* SIMD level actually used */
    __u32 vectors_processed;    /* Actual vectors processed */
    __u32 reserved[4];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * Vector Processing Statistics
 */
struct vexfs_vector_processing_stats {
    __u64 total_operations;
    __u64 l2_normalizations;
    __u64 scalar_quantizations;
    __u64 product_quantizations;
    __u64 binary_quantizations;
    
    /* Performance metrics */
    __u64 total_processing_time_ns;
    __u64 avg_processing_time_ns;
    __u64 simd_accelerated_ops;
    __u64 scalar_fallback_ops;
    
    /* SIMD usage statistics */
    __u64 sse2_operations;
    __u64 avx2_operations;
    __u64 avx512_operations;
    __u64 neon_operations;
    
    __u32 reserved[8];
}
#ifdef __KERNEL__
__packed
#endif
;

/*
 * IOCTL Commands for Vector Processing
 */
#define VEXFS_IOC_VECTOR_PROCESS    _IOWR(VEXFS_IOC_MAGIC, 30, struct vexfs_vector_processing_request)
#define VEXFS_IOC_GET_PROC_STATS    _IOR(VEXFS_IOC_MAGIC, 31, struct vexfs_vector_processing_stats)
#define VEXFS_IOC_GET_SIMD_CAPS     _IOR(VEXFS_IOC_MAGIC, 32, __u32)

#ifdef __KERNEL__

/*
 * Core Vector Processing Functions
 */

/* SIMD capability detection */
__u32 vexfs_detect_simd_capabilities(void);

/* L2 Normalization Functions */
int vexfs_l2_normalize_vectors(const __u32 *input_bits, __u32 *output_bits,
                              __u32 dimensions, __u32 vector_count);
int vexfs_l2_normalize_vectors_simd(const __u32 *input_bits, __u32 *output_bits,
                                   __u32 dimensions, __u32 vector_count, __u32 simd_level);
int vexfs_l2_normalize_vectors_scalar(const __u32 *input_bits, __u32 *output_bits,
                                     __u32 dimensions, __u32 vector_count);

/* Scalar Quantization Functions */
int vexfs_scalar_quantize_int8(const __u32 *input_bits, __s8 *output,
                              __u32 dimensions, __u32 vector_count,
                              __u32 scale_bits, __u32 offset_bits);
int vexfs_scalar_quantize_uint8(const __u32 *input_bits, __u8 *output,
                               __u32 dimensions, __u32 vector_count,
                               __u32 scale_bits, __u32 offset_bits);
int vexfs_scalar_quantize_simd(const __u32 *input_bits, void *output,
                              __u32 dimensions, __u32 vector_count,
                              __u32 quant_type, __u32 scale_bits, __u32 offset_bits,
                              __u32 simd_level);

/* Product Quantization Functions */
int vexfs_product_quantize(const __u32 *input_bits, __u8 *output_codes,
                          __u32 dimensions, __u32 vector_count,
                          const struct vexfs_pq_config *config);
int vexfs_product_quantize_with_codebooks(const __u32 *input_bits, __u8 *output_codes,
                                         __u32 dimensions, __u32 vector_count,
                                         const struct vexfs_pq_config *config,
                                         const __u32 *codebooks_bits);
int vexfs_train_pq_codebooks(const __u32 *training_data_bits,
                            __u32 dimensions, __u32 training_count,
                            const struct vexfs_pq_config *config,
                            __u32 *codebooks_bits);
int vexfs_train_pq_codebooks_kmeans(const __u32 *training_data_bits,
                                   __u32 dimensions, __u32 training_count,
                                   const struct vexfs_pq_config *config,
                                   __u32 *codebooks_bits);
__u32 vexfs_compute_subvector_distance(const __u32 *vec1_bits, const __u32 *vec2_bits,
                                      __u32 dimensions);
int vexfs_pq_search_with_codes(const __u32 *query_bits, const __u8 *pq_codes,
                              __u32 dimensions, __u32 vector_count,
                              const struct vexfs_pq_config *config,
                              const __u32 *codebooks_bits,
                              __u32 *result_indices, __u32 k);

/* Binary Quantization Functions */
int vexfs_binary_quantize(const __u32 *input_bits, __u8 *output_codes,
                         __u32 dimensions, __u32 vector_count,
                         __u32 threshold_bits);
int vexfs_binary_quantize_simd(const __u32 *input_bits, __u8 *output_codes,
                              __u32 dimensions, __u32 vector_count,
                              __u32 threshold_bits, __u32 simd_level);

/* SIMD-specific implementations */
#ifdef CONFIG_X86_64
int vexfs_l2_normalize_avx2(const __u32 *input_bits, __u32 *output_bits,
                           __u32 dimensions, __u32 vector_count);
int vexfs_l2_normalize_avx512(const __u32 *input_bits, __u32 *output_bits,
                             __u32 dimensions, __u32 vector_count);
int vexfs_scalar_quantize_avx2(const __u32 *input_bits, void *output,
                               __u32 dimensions, __u32 vector_count,
                               __u32 quant_type, __u32 scale_bits, __u32 offset_bits);
int vexfs_binary_quantize_avx2(const __u32 *input_bits, __u8 *output_codes,
                               __u32 dimensions, __u32 vector_count,
                               __u32 threshold_bits);
int vexfs_product_quantize_avx2(const __u32 *input_bits, __u8 *output_codes,
                               __u32 dimensions, __u32 vector_count,
                               const struct vexfs_pq_config *config,
                               const __u32 *codebooks_bits);
#endif

#ifdef CONFIG_ARM64
int vexfs_l2_normalize_neon(const __u32 *input_bits, __u32 *output_bits,
                           __u32 dimensions, __u32 vector_count);
int vexfs_scalar_quantize_neon(const __u32 *input_bits, void *output,
                              __u32 dimensions, __u32 vector_count,
                              __u32 quant_type, __u32 scale_bits, __u32 offset_bits);
int vexfs_binary_quantize_neon(const __u32 *input_bits, __u8 *output_codes,
                              __u32 dimensions, __u32 vector_count,
                              __u32 threshold_bits);
#endif

/* Utility Functions */
__u32 vexfs_ieee754_sqrt(__u32 input_bits);
__u32 vexfs_ieee754_reciprocal(__u32 input_bits);
void vexfs_ieee754_vector_scale(const __u32 *input_bits, __u32 *output_bits,
                               __u32 count, __u32 scale_bits);

/* Statistics and Monitoring */
void vexfs_get_vector_processing_stats(struct vexfs_vector_processing_stats *stats);
void vexfs_reset_vector_processing_stats(void);

/* Module initialization */
int vexfs_vector_processing_init(void);
void vexfs_vector_processing_exit(void);

/* IOCTL handler */
long vexfs_vector_processing_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

#endif /* __KERNEL__ */

#endif /* VEXFS_V2_VECTOR_PROCESSING_H */