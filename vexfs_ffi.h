/* VexFS - Vector Extended File System C FFI Bindings
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/* Kernel-space compatible headers */
#include <linux/types.h>
#include <linux/kernel.h>

/* Kernel-space type definitions - no standard library headers */
#ifndef __KERNEL__
#include <stdint.h>
#include <stdbool.h>
#endif


/* Generated with cbindgen:0.29.0 */

/**
 * Error codes for C FFI
 */
#define VEXFS_SUCCESS 0

#define VEXFS_ERROR_GENERIC -1

#define VEXFS_ERROR_NOMEM -12

#define VEXFS_ERROR_INVAL -22

#define VEXFS_ERROR_NOSPC -28

#define VEXFS_ERROR_PERMISSION -1

#define VEXFS_ERROR_NOENT -2

#define VEXFS_ERROR_IO -5

#define VEXFS_ERROR_EXIST -17

#define VEXFS_ERROR_NOTDIR -20

#define VEXFS_ERROR_ISDIR -21

/**
 * VexFS filesystem constants
 */
#define VEXFS_NAME_LEN 255

#define VEXFS_MAX_FILE_SIZE 1099511627776

#define VEXFS_BLOCK_SIZE 4096

/**
 * File mode constants (matching Unix/Linux)
 */
#define VEXFS_S_IFREG 32768

#define VEXFS_S_IFDIR 16384

#define VEXFS_S_IFLNK 40960

/**
 * VexFS magic number for superblock identification
 */
#define VEXFS_MAGIC 370530600531ull

/**
 * VexFS version constants
 */
#define VEXFS_VERSION_MAJOR 1

#define VEXFS_VERSION_MINOR 0

/**
 * Block size constants (configurable 4KB-64KB)
 */
#define VEXFS_MIN_BLOCK_SIZE 4096

#define VEXFS_MAX_BLOCK_SIZE 65536

#define VEXFS_DEFAULT_BLOCK_SIZE 4096

/**
 * Inode constants - Fixed to match actual struct size
 */
#define VEXFS_ROOT_INO 1

#define VEXFS_FIRST_USER_INO 11

#define VEXFS_INODE_SIZE 128

#define VEXFS_INODES_PER_BLOCK (VEXFS_DEFAULT_BLOCK_SIZE / (uint32_t)VEXFS_INODE_SIZE)

#define VEXFS_MAX_FILENAME_LEN 255

#define VEXFS_DIR_ENTRIES_PER_BLOCK ((uintptr_t)VEXFS_DEFAULT_BLOCK_SIZE / 64)

#define DT_UNKNOWN 0

#define DT_FIFO 1

#define DT_CHR 2

#define DT_DIR 4

#define DT_BLK 6

#define DT_REG 8

#define DT_LNK 10

#define DT_SOCK 12

#define DT_WHT 14

/**
 * Direct and indirect block pointer constants
 */
#define VEXFS_N_DIRECT 12

#define VEXFS_N_INDIRECT 1

#define VEXFS_N_DINDIRECT 1

#define VEXFS_N_TINDIRECT 1

/**
 * Journal constants
 */
#define VEXFS_JOURNAL_BLOCKS 1024

#define VEXFS_JOURNAL_MAGIC 1447385158

/**
 * Feature flags for superblock
 */
#define VEXFS_FEATURE_COMPAT_DIR_INDEX 1

#define VEXFS_FEATURE_COMPAT_RESIZE_INODE 2

#define VEXFS_FEATURE_COMPAT_JOURNAL 4

#define VEXFS_FEATURE_INCOMPAT_COMPRESSION 1

#define VEXFS_FEATURE_INCOMPAT_FILETYPE 2

#define VEXFS_FEATURE_INCOMPAT_64BIT 4

#define VEXFS_FEATURE_INCOMPAT_EXTENTS 8

#define VEXFS_FEATURE_RO_COMPAT_SPARSE_SUPER 1

#define VEXFS_FEATURE_RO_COMPAT_LARGE_FILE 2

#define VEXFS_FEATURE_RO_COMPAT_BTREE_DIR 4

/**
 * Filesystem states
 */
#define VEXFS_VALID_FS 1

#define VEXFS_ERROR_FS 2

/**
 * Error handling behavior
 */
#define VEXFS_ERRORS_CONTINUE 1

#define VEXFS_ERRORS_RO 2

#define VEXFS_ERRORS_PANIC 3

/**
 * File types for directory entries
 */
#define VEXFS_FT_UNKNOWN 0

#define VEXFS_FT_REG_FILE 1

#define VEXFS_FT_DIR 2

#define VEXFS_FT_CHRDEV 3

#define VEXFS_FT_BLKDEV 4

#define VEXFS_FT_FIFO 5

#define VEXFS_FT_SOCK 6

#define VEXFS_FT_SYMLINK 7

/**
 * Journal block types
 */
#define VEXFS_JOURNAL_DESCRIPTOR_BLOCK 1

#define VEXFS_JOURNAL_COMMIT_BLOCK 2

#define VEXFS_JOURNAL_SUPERBLOCK_V1 3

#define VEXFS_JOURNAL_SUPERBLOCK_V2 4

#define VEXFS_JOURNAL_REVOKE_BLOCK 5

/**
 * Vector metadata magic number
 */
#define VEXFS_VECTOR_MAGIC 1447385174

/**
 * Extent magic number
 */
#define VEXFS_EXT_MAGIC 62218

/**
 * Vector storage format version
 */
#define VECTOR_FORMAT_VERSION 1

/**
 * Maximum vector dimensions supported
 */
#define MAX_VECTOR_DIMENSIONS 4096

/**
 * Vector block size alignment (64 bytes for cache efficiency)
 */
#define VECTOR_ALIGNMENT 64

#define VectorHeader_MAGIC 1447379800

/**
 * Maximum vector dimensions for SIMD optimization
 */
#define SIMD_MAX_DIMENSIONS 4096

/**
 * SIMD vector size (256 bits = 8 x f32)
 */
#define SIMD_WIDTH_F32 8

/**
 * SIMD vector size (512 bits = 16 x f32) for AVX-512
 */
#define SIMD_WIDTH_AVX512_F32 16

/**
 * Alignment for SIMD operations
 */
#define SIMD_ALIGNMENT 32

/**
 * Maximum number of results that can be returned
 */
#define MAX_KNN_RESULTS 10000

/**
 * Maximum number of candidate vectors to consider during search
 */
#define MAX_CANDIDATES 100000

/**
 * Threshold for switching to exact search
 */
#define EXACT_SEARCH_THRESHOLD 1000

/**
 * Maximum number of results that can be scored
 */
#define MAX_SCORABLE_RESULTS 10000

/**
 * Confidence score calculation parameters
 */
#define CONFIDENCE_ALPHA 0.8

#define CONFIDENCE_BETA 0.2

#define HIGH_CONFIDENCE (1 << 0)

#define LOW_DISTANCE (1 << 1)

#define RECENT_FILE (1 << 2)

#define LARGE_FILE (1 << 3)

#define EXACT_DIMENSION_MATCH (1 << 4)

#define POTENTIAL_DUPLICATE (1 << 5)

#define OUTLIER_DISTANCE (1 << 6)

#define LOW_QUALITY (1 << 7)

/**
 * Maximum number of results that can be returned from a search
 */
#define MAX_SEARCH_RESULTS 10000

/**
 * Maximum batch size for search requests
 */
#define MAX_BATCH_SIZE 100

/**
 * Initialize the VexFS Rust components
 * Called during module_init from C kernel module
 */
int vexfs_rust_init(void);

/**
 * Cleanup the VexFS Rust components
 * Called during module_exit from C kernel module
 */
void vexfs_rust_exit(void);

/**
 * Initialize the VexFS superblock structure
 * Called during filesystem mount from C kernel module
 *
 * # Arguments
 * * `sb_ptr` - Pointer to the Linux superblock structure
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_fill_super(void *sb_ptr);

/**
 * Test function to verify FFI is working
 * This is a simple test function that can be called from C
 */
int vexfs_rust_test_basic(void);

/**
 * Test function for vector operations
 * This tests that vector-related code can be called via FFI
 */
int vexfs_rust_test_vector_ops(void);

/**
 * Get version information
 * Returns a packed version number (major << 16 | minor << 8 | patch)
 */
int vexfs_rust_get_version(void);

/**
 * Filesystem statistics FFI function
 * Called to fill filesystem statistics from Rust implementation
 *
 * # Arguments
 * * `blocks` - Pointer to store total blocks
 * * `free_blocks` - Pointer to store free blocks
 * * `files` - Pointer to store total files
 * * `free_files` - Pointer to store free files
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_get_statfs(uint64_t *blocks,
                          uint64_t *free_blocks,
                          uint64_t *files,
                          uint64_t *free_files);

/**
 * Create and initialize a new inode
 * Called from C kernel module when creating inodes
 *
 * # Arguments
 * * `sb_ptr` - Pointer to the Linux superblock structure
 * * `ino` - Inode number to assign
 * * `mode` - File mode (permissions and type)
 *
 * # Returns
 * * Pointer to allocated inode on success
 * * NULL on failure
 */
void *vexfs_rust_new_inode(void *sb_ptr, uint64_t ino, uint32_t mode);

/**
 * Initialize VFS-specific inode data
 * Called from C kernel module after Linux inode allocation
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the Linux inode structure
 * * `ino` - Inode number
 * * `mode` - File mode
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_init_inode(void *inode_ptr, uint64_t ino, uint32_t mode);

/**
 * Cleanup VFS-specific inode data
 * Called from C kernel module before inode deallocation
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the Linux inode structure
 */
void vexfs_rust_destroy_inode(void *inode_ptr);

/**
 * Write inode to storage
 * Called from C kernel module when inode needs to be persisted
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the Linux inode structure
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_write_inode(void *inode_ptr);

/**
 * Synchronize filesystem data
 * Called from C kernel module during sync operations
 *
 * # Arguments
 * * `sb_ptr` - Pointer to the Linux superblock structure
 * * `wait` - Whether to wait for completion
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_sync_fs(void *sb_ptr, int wait);

/**
 * Put (cleanup) superblock
 * Called from C kernel module during unmount
 *
 * # Arguments
 * * `sb_ptr` - Pointer to the Linux superblock structure
 */
void vexfs_rust_put_super(void *sb_ptr);

/**
 * Cleanup superblock during unmount
 * Called from vexfs_kill_sb in C kernel module
 *
 * # Arguments
 * * `sb_ptr` - Pointer to the Linux superblock structure
 */
void vexfs_rust_cleanup_superblock(void *sb_ptr);

/**
 * Userspace test function for vector search operations
 */
int vexfs_rust_vector_search(void);

/**
 * Userspace test function for vector storage operations
 */
int vexfs_rust_vector_storage(void);

/**
 * Userspace initialization for testing
 */
int vexfs_rust_userspace_init(void);

#ifdef __cplusplus
}
#endif
