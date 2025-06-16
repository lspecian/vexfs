/*
 * VexFS FFI (Foreign Function Interface) Header
 * 
 * This header defines the interface between the C kernel module
 * and the Rust filesystem implementation.
 * 
 * Copyright (C) 2024 VexFS Development Team
 * Licensed under GPL v2
 */

#ifndef VEXFS_FFI_H
#define VEXFS_FFI_H

#include <linux/types.h>
#include <linux/fs.h>

/*
 * =============================================================================
 * FFI FUNCTION DECLARATIONS
 * =============================================================================
 */

/**
 * vexfs_rust_init - Initialize Rust components
 * 
 * Called during module initialization to set up the Rust filesystem core.
 * 
 * Return: 0 on success, negative error code on failure
 */
extern int vexfs_rust_init(void);

/**
 * vexfs_rust_cleanup - Cleanup Rust components
 * 
 * Called during module cleanup to free Rust resources.
 */
extern void vexfs_rust_cleanup(void);

/**
 * vexfs_rust_read_file - Read data from file
 * @inode_ptr: Pointer to inode structure
 * @file_ptr: Pointer to file structure
 * @buf: User buffer to read data into
 * @count: Number of bytes to read
 * @pos: File position to read from
 * @bytes_read: Output parameter for actual bytes read
 * 
 * Reads data from the file at the specified position into the user buffer.
 * 
 * Return: 0 on success, negative error code on failure
 */
extern int vexfs_rust_read_file(void *inode_ptr, void *file_ptr, void *buf,
                               __u64 count, __u64 pos, __u64 *bytes_read);

/**
 * vexfs_rust_write_file - Write data to file
 * @inode_ptr: Pointer to inode structure
 * @file_ptr: Pointer to file structure
 * @buf: User buffer containing data to write
 * @count: Number of bytes to write
 * @pos: File position to write to
 * @bytes_written: Output parameter for actual bytes written
 * 
 * Writes data from the user buffer to the file at the specified position.
 * 
 * Return: 0 on success, negative error code on failure
 */
extern int vexfs_rust_write_file(void *inode_ptr, void *file_ptr, const void *buf,
                                __u64 count, __u64 pos, __u64 *bytes_written);

/**
 * vexfs_rust_create_file - Create a new file
 * @dir_inode_ptr: Pointer to directory inode
 * @dentry_ptr: Pointer to dentry structure
 * @mode: File mode/permissions
 * 
 * Creates a new file in the specified directory.
 * 
 * Return: 0 on success, negative error code on failure
 */
extern int vexfs_rust_create_file(void *dir_inode_ptr, void *dentry_ptr, __u16 mode);

/**
 * vexfs_rust_unlink_file - Remove a file
 * @dir_inode_ptr: Pointer to directory inode
 * @dentry_ptr: Pointer to dentry structure
 * 
 * Removes a file from the specified directory.
 * 
 * Return: 0 on success, negative error code on failure
 */
extern int vexfs_rust_unlink_file(void *dir_inode_ptr, void *dentry_ptr);

/**
 * vexfs_rust_sync_file - Synchronize file data to storage
 * @inode_ptr: Pointer to inode structure
 * @file_ptr: Pointer to file structure
 * @start: Start offset for sync
 * @end: End offset for sync
 * @datasync: Whether to sync only data (not metadata)
 * 
 * Synchronizes file data to persistent storage.
 * 
 * Return: 0 on success, negative error code on failure
 */
extern int vexfs_rust_sync_file(void *inode_ptr, void *file_ptr,
                               __u64 start, __u64 end, int datasync);

/*
 * =============================================================================
 * FFI ERROR CODES
 * =============================================================================
 */

#define VEXFS_FFI_SUCCESS           0
#define VEXFS_FFI_ERROR_GENERIC    -1
#define VEXFS_FFI_ERROR_INVAL      -22  /* EINVAL */
#define VEXFS_FFI_ERROR_NOENT      -2   /* ENOENT */
#define VEXFS_FFI_ERROR_NOMEM      -12  /* ENOMEM */
#define VEXFS_FFI_ERROR_NOSPC      -28  /* ENOSPC */
#define VEXFS_FFI_ERROR_IO         -5   /* EIO */

#endif /* VEXFS_FFI_H */