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
 * Error codes for C FFI - consistent with Linux kernel error codes
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
 * Enhanced error codes for comprehensive error handling
 */
#define VEXFS_ERROR_TIMEOUT -110

#define VEXFS_ERROR_BUSY -16

#define VEXFS_ERROR_AGAIN -11

#define VEXFS_ERROR_CIRCUIT_BREAKER -1000

#define VEXFS_ERROR_RETRY_EXHAUSTED -1001

#define VEXFS_ERROR_FALLBACK_FAILED -1002

#define VEXFS_ERROR_CORRUPTION -1003

#define VEXFS_ERROR_RECOVERY_NEEDED -1004

/**
 * Error severity levels for classification
 */
#define VEXFS_SEVERITY_LOW 0

#define VEXFS_SEVERITY_MEDIUM 1

#define VEXFS_SEVERITY_HIGH 2

#define VEXFS_SEVERITY_CRITICAL 3

/**
 * Recovery hint flags
 */
#define VEXFS_RECOVERY_RETRY 0x01

#define VEXFS_RECOVERY_FALLBACK 0x02

#define VEXFS_RECOVERY_CACHE_INVALIDATE 0x04

#define VEXFS_RECOVERY_REDUCE_SCOPE 0x08

#define VEXFS_RECOVERY_READ_ONLY 0x10

#define VEXFS_RECOVERY_RESTART 0x20

#define VEXFS_RECOVERY_MANUAL 0x40

/**
 * VexFS filesystem constants
 */
#define VEXFS_NAME_LEN 255

#define VEXFS_MAX_FILE_SIZE 1099511627776

#define VEXFS_BLOCK_SIZE 4096

/**
 * Memory management constants
 */
#define VEXFS_MEMORY_POOL_SIZE 1024

#define VEXFS_MEMORY_ALIGNMENT 64

#define VEXFS_MAX_MEMORY_USAGE (256 * 1024 * 1024)

#define VEXFS_MEMORY_LEAK_THRESHOLD 100

#define VEXFS_MEMORY_TRACKING_ENABLED 1

/**
 * System hang prevention constants
 */
#define VEXFS_MAX_OPERATION_TIMEOUT_SECS 300

#define VEXFS_FILE_IO_TIMEOUT_SECS 30

#define VEXFS_DIRECTORY_TIMEOUT_SECS 15

#define VEXFS_FFI_CALL_TIMEOUT_SECS 5

#define VEXFS_MOUNT_TIMEOUT_SECS 60

#define VEXFS_MAX_CONCURRENT_OPERATIONS 100

#define VEXFS_DEADLOCK_CHECK_INTERVAL_SECS 1

#define VEXFS_LOCK_TIMEOUT_SECS 10

#define VEXFS_RESOURCE_MONITOR_INTERVAL_SECS 5

/**
 * System degradation levels
 */
#define VEXFS_DEGRADATION_NORMAL 0

#define VEXFS_DEGRADATION_LIGHT 1

#define VEXFS_DEGRADATION_MODERATE 2

#define VEXFS_DEGRADATION_HEAVY 3

#define VEXFS_DEGRADATION_READONLY 4

#define VEXFS_DEGRADATION_EMERGENCY 5

/**
 * Panic recovery strategies
 */
#define VEXFS_PANIC_CONTINUE_DEGRADED 0

#define VEXFS_PANIC_SWITCH_READONLY 1

#define VEXFS_PANIC_GRACEFUL_SHUTDOWN 2

#define VEXFS_PANIC_EMERGENCY_SHUTDOWN 3

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

/* ============================================================================
 * KERNEL FFI FUNCTIONS - Called from C kernel module
 * ============================================================================ */

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
 * Create file metadata
 * Called from vexfs_create in C kernel module
 *
 * # Arguments
 * * `dir_ptr` - Pointer to the parent directory inode
 * * `name` - Name of the file to create
 * * `inode_ptr` - Pointer to the newly created inode
 * * `mode` - File mode
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_create_file(void *dir_ptr, void *dentry_ptr, void *inode_ptr, uint32_t mode);

/**
 * Lookup inode by name in directory
 * Called from vexfs_lookup in C kernel module
 *
 * # Arguments
 * * `dir_ptr` - Pointer to the directory inode
 * * `name` - Name to look up
 * * `name_len` - Length of the name
 * * `ino` - Pointer to store found inode number
 * * `mode` - Pointer to store found inode mode
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success (found)
 * * `VEXFS_ERROR_NOENT` if not found
 * * Error code on failure
 */
int vexfs_rust_lookup_inode(void *dir_ptr, const char *name, uint32_t name_len, uint64_t *ino, uint32_t *mode);

/**
 * Open file
 * Called from vexfs_open in C kernel module
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the inode
 * * `file_ptr` - Pointer to the file structure
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_open_file(void *inode_ptr, void *file_ptr);

/**
 * Release file
 * Called from vexfs_release in C kernel module
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the inode
 * * `file_ptr` - Pointer to the file structure
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_release_file(void *inode_ptr, void *file_ptr);

/**
 * Read from file
 * Called from vexfs_read in C kernel module
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the inode
 * * `file_ptr` - Pointer to the file structure
 * * `buf` - User buffer to read into
 * * `count` - Number of bytes to read
 * * `pos` - File position
 * * `bytes_read` - Pointer to store actual bytes read
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_read_file(void *inode_ptr, void *file_ptr, void *buf, uint64_t count, uint64_t pos, uint64_t *bytes_read);

/**
 * Write to file
 * Called from vexfs_write in C kernel module
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the inode
 * * `file_ptr` - Pointer to the file structure
 * * `buf` - User buffer to write from
 * * `count` - Number of bytes to write
 * * `pos` - File position
 * * `bytes_written` - Pointer to store actual bytes written
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_write_file(void *inode_ptr, void *file_ptr, const void *buf, uint64_t count, uint64_t pos, uint64_t *bytes_written);

/**
 * Synchronize file data
 * Called from vexfs_fsync in C kernel module
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the inode
 * * `file_ptr` - Pointer to the file structure
 * * `start` - Start offset for sync
 * * `end` - End offset for sync
 * * `datasync` - Whether to sync only data (not metadata)
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_fsync_file(void *inode_ptr, void *file_ptr, uint64_t start, uint64_t end, int datasync);

/**
 * Read directory entries
 * Called from vexfs_readdir in C kernel module
 *
 * # Arguments
 * * `inode_ptr` - Pointer to the directory inode
 * * `file_ptr` - Pointer to the file structure
 * * `ctx_ptr` - Pointer to the directory context
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_readdir(void *inode_ptr, void *file_ptr, void *ctx_ptr);

/* ============================================================================
 * USERSPACE FFI FUNCTIONS - For testing and development
 * ============================================================================ */

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

/* ============================================================================
 * HANG PREVENTION FFI FUNCTIONS - System stability and hang prevention
 * ============================================================================ */

/**
 * Initialize the hang prevention system
 * Called during module initialization
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_init_hang_prevention(void);

/**
 * Shutdown the hang prevention system
 * Called during module cleanup
 */
void vexfs_rust_shutdown_hang_prevention(void);

/**
 * Start a watchdog timer for an operation
 * Called before starting long-running operations
 *
 * # Arguments
 * * `operation_type` - Type of operation (see VEXFS_OP_* constants)
 * * `timeout_secs` - Timeout in seconds (0 for default)
 * * `watchdog_id` - Pointer to store the watchdog ID
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_start_watchdog(uint32_t operation_type, uint32_t timeout_secs, uint64_t *watchdog_id);

/**
 * Cancel a watchdog timer
 * Called when operation completes successfully
 *
 * # Arguments
 * * `watchdog_id` - ID of the watchdog to cancel
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_cancel_watchdog(uint64_t watchdog_id);

/**
 * Check if an operation should be allowed
 * Called before starting operations to check system state
 *
 * # Arguments
 * * `operation_type` - Type of operation to check
 *
 * # Returns
 * * `VEXFS_SUCCESS` if operation is allowed
 * * `VEXFS_ERROR_BUSY` if operation should be denied
 * * Other error codes for specific restrictions
 */
int vexfs_rust_check_operation_allowed(uint32_t operation_type);

/**
 * Update system resource usage statistics
 * Called periodically to update resource monitoring
 *
 * # Arguments
 * * `memory_bytes` - Current memory usage in bytes
 * * `cpu_percent` - Current CPU usage percentage
 */
void vexfs_rust_update_resources(uint64_t memory_bytes, uint32_t cpu_percent);

/**
 * Get current system health status
 * Called to check overall system health
 *
 * # Arguments
 * * `degradation_level` - Pointer to store current degradation level
 * * `memory_percent` - Pointer to store memory usage percentage
 * * `cpu_percent` - Pointer to store CPU usage percentage
 * * `active_ops` - Pointer to store number of active operations
 *
 * # Returns
 * * `VEXFS_SUCCESS` on success
 * * Error code on failure
 */
int vexfs_rust_get_health_status(uint32_t *degradation_level, uint32_t *memory_percent, uint32_t *cpu_percent, uint32_t *active_ops);

/**
 * Handle a panic situation
 * Called when a critical error occurs that might cause system hang
 *
 * # Arguments
 * * `operation_type` - Type of operation that caused the panic
 * * `error_message` - Error message (null-terminated string)
 * * `recovery_strategy` - Pointer to store recommended recovery strategy
 *
 * # Returns
 * * `VEXFS_SUCCESS` on successful panic handling
 * * Error code on failure
 */
int vexfs_rust_handle_panic(uint32_t operation_type, const char *error_message, uint32_t *recovery_strategy);

/**
 * Operation type constants for hang prevention
 */
#define VEXFS_OP_FILE_READ 0
#define VEXFS_OP_FILE_WRITE 1
#define VEXFS_OP_DIRECTORY_LOOKUP 2
#define VEXFS_OP_DIRECTORY_CREATE 3
#define VEXFS_OP_INODE_ALLOCATION 4
#define VEXFS_OP_BLOCK_ALLOCATION 5
#define VEXFS_OP_VECTOR_SEARCH 6
#define VEXFS_OP_VECTOR_STORE 7
#define VEXFS_OP_FFI_CALL 8
#define VEXFS_OP_MOUNT 9
#define VEXFS_OP_UNMOUNT 10
#define VEXFS_OP_SYNC 11
#define VEXFS_OP_JOURNAL 12

#ifdef __cplusplus
}
#endif
