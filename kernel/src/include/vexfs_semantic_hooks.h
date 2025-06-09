/*
 * VexFS Semantic Event Hooks - Kernel Module Integration
 *
 * This header defines the C interface for integrating semantic event
 * hooks into the VexFS kernel module for Task 18.3.
 */

#ifndef VEXFS_SEMANTIC_HOOKS_H
#define VEXFS_SEMANTIC_HOOKS_H

#include <linux/types.h>
#include <linux/kernel.h>
#include <linux/time.h>
#include <linux/string.h>
#include <linux/sched.h>
#include <linux/ktime.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Event hook configuration flags */
#define VEXFS_HOOK_FILESYSTEM   0x01
#define VEXFS_HOOK_SYSTEM       0x02
#define VEXFS_HOOK_PERFORMANCE  0x04
#define VEXFS_HOOK_ERROR        0x08
#define VEXFS_HOOK_ALL          0xFF

/* Kernel operation types for semantic events */
enum vexfs_kernel_operation {
    VEXFS_OP_FILE_OPEN = 0,
    VEXFS_OP_FILE_CLOSE = 1,
    VEXFS_OP_FILE_READ = 2,
    VEXFS_OP_FILE_WRITE = 3,
    VEXFS_OP_FILE_CREATE = 4,
    VEXFS_OP_FILE_DELETE = 5,
    VEXFS_OP_FILE_RENAME = 6,
    VEXFS_OP_FILE_CHMOD = 7,
    VEXFS_OP_FILE_CHOWN = 8,
    VEXFS_OP_FILE_TRUNCATE = 9,
    VEXFS_OP_DIR_CREATE = 10,
    VEXFS_OP_DIR_DELETE = 11,
    VEXFS_OP_DIR_READ = 12,
    VEXFS_OP_SYMLINK_CREATE = 13,
    VEXFS_OP_HARDLINK_CREATE = 14,
    VEXFS_OP_MOUNT = 15,
    VEXFS_OP_UNMOUNT = 16,
    VEXFS_OP_SYNC = 17,
};

/* Kernel event context structure */
struct vexfs_kernel_event_context {
    enum vexfs_kernel_operation operation_type;
    const char *path;
    u32 path_len;
    u64 inode_number;
    u64 file_size;
    u32 mode;
    u32 uid;
    u32 gid;
    u32 pid;
    u32 tid;
    u64 timestamp_sec;
    u64 timestamp_nsec;
    u32 flags;
    int error_code;
};

/* Performance tracking structure */
struct vexfs_operation_timing {
    u64 start_time_ns;
    u64 end_time_ns;
    u64 duration_ns;
    u32 cpu_id;
    u32 operation_type;
};

/* Hook statistics structure */
struct vexfs_hook_stats {
    u64 total_events;
    u64 filesystem_events;
    u64 system_events;
    u64 error_events;
    u64 dropped_events;
    u64 hook_failures;
};

/* Function declarations for Rust FFI integration */

/**
 * Initialize semantic event hooks
 * Called during module initialization
 */
int vexfs_init_semantic_hooks(u32 hook_flags);

/**
 * Cleanup semantic event hooks
 * Called during module cleanup
 */
void vexfs_cleanup_semantic_hooks(void);

/**
 * Emit a kernel semantic event
 * Called from filesystem operation hooks
 */
int vexfs_emit_kernel_event(const struct vexfs_kernel_event_context *context);

/**
 * Start operation timing
 * Called at the beginning of filesystem operations
 */
int vexfs_start_operation_timing(u32 operation_type, const char *path, u64 inode_number);

/**
 * End operation timing
 * Called at the end of filesystem operations
 */
int vexfs_end_operation_timing(u32 operation_type, const char *path, u64 inode_number, int error_code);

/**
 * Emit system event (mount, unmount, sync)
 * Called from system-level operations
 */
int vexfs_emit_system_event(u32 event_type, const char *device_path, const char *mount_point, u32 flags);

/**
 * Enable or disable semantic hooks
 * Called to dynamically control hook behavior
 */
int vexfs_set_semantic_hooks_enabled(int enabled);

/**
 * Get hook statistics
 * Called to retrieve performance and usage statistics
 */
int vexfs_get_semantic_hook_stats(struct vexfs_hook_stats *stats);

/* Convenience macros for common operations */

#define VEXFS_HOOK_FILE_OP(op_type, path, inode, mode, error) do { \
    if (vexfs_semantic_hooks_enabled()) { \
        struct vexfs_kernel_event_context ctx = { \
            .operation_type = (op_type), \
            .path = (path), \
            .path_len = (path) ? strlen(path) : 0, \
            .inode_number = (inode), \
            .mode = (mode), \
            .error_code = (error), \
            .pid = current->pid, \
            .tid = current->pid, \
            .timestamp_sec = ktime_get_real_seconds(), \
            .timestamp_nsec = ktime_get_ns() % 1000000000ULL, \
        }; \
        vexfs_emit_kernel_event(&ctx); \
    } \
} while (0)

#define VEXFS_HOOK_TIMING_START(op_type, path, inode) \
    vexfs_start_operation_timing((op_type), (path), (inode))

#define VEXFS_HOOK_TIMING_END(op_type, path, inode, error) \
    vexfs_end_operation_timing((op_type), (path), (inode), (error))

/* Hook integration points for VFS operations */

static inline void vexfs_hook_file_open(const char *path, u64 inode, u32 mode)
{
    VEXFS_HOOK_FILE_OP(VEXFS_OP_FILE_OPEN, path, inode, mode, 0);
}

static inline void vexfs_hook_file_close(const char *path, u64 inode, u32 mode)
{
    VEXFS_HOOK_FILE_OP(VEXFS_OP_FILE_CLOSE, path, inode, mode, 0);
}

static inline void vexfs_hook_file_read(const char *path, u64 inode, u64 size, int error)
{
    struct vexfs_kernel_event_context ctx = {
        .operation_type = VEXFS_OP_FILE_READ,
        .path = path,
        .path_len = path ? strlen(path) : 0,
        .inode_number = inode,
        .file_size = size,
        .error_code = error,
        .pid = current->pid,
        .tid = current->pid,
        .timestamp_sec = ktime_get_real_seconds(),
        .timestamp_nsec = ktime_get_ns() % 1000000000ULL,
    };
    vexfs_emit_kernel_event(&ctx);
}

static inline void vexfs_hook_file_write(const char *path, u64 inode, u64 size, int error)
{
    struct vexfs_kernel_event_context ctx = {
        .operation_type = VEXFS_OP_FILE_WRITE,
        .path = path,
        .path_len = path ? strlen(path) : 0,
        .inode_number = inode,
        .file_size = size,
        .error_code = error,
        .pid = current->pid,
        .tid = current->pid,
        .timestamp_sec = ktime_get_real_seconds(),
        .timestamp_nsec = ktime_get_ns() % 1000000000ULL,
    };
    vexfs_emit_kernel_event(&ctx);
}

static inline void vexfs_hook_file_create(const char *path, u64 inode, u32 mode, int error)
{
    VEXFS_HOOK_FILE_OP(VEXFS_OP_FILE_CREATE, path, inode, mode, error);
}

static inline void vexfs_hook_file_delete(const char *path, u64 inode, int error)
{
    VEXFS_HOOK_FILE_OP(VEXFS_OP_FILE_DELETE, path, inode, 0, error);
}

static inline void vexfs_hook_dir_create(const char *path, u64 inode, u32 mode, int error)
{
    VEXFS_HOOK_FILE_OP(VEXFS_OP_DIR_CREATE, path, inode, mode, error);
}

static inline void vexfs_hook_dir_delete(const char *path, u64 inode, int error)
{
    VEXFS_HOOK_FILE_OP(VEXFS_OP_DIR_DELETE, path, inode, 0, error);
}

/* Global hook state */
extern bool vexfs_semantic_hooks_enabled_flag;

static inline bool vexfs_semantic_hooks_enabled(void)
{
    return vexfs_semantic_hooks_enabled_flag;
}

/* Rust FFI function declarations */
extern int vexfs_rust_emit_kernel_event(const struct vexfs_kernel_event_context *context);
extern int vexfs_rust_hook_fs_operation_start(u32 operation_type, const char *path, u64 inode_number);
extern int vexfs_rust_hook_fs_operation_end(u32 operation_type, const char *path, u64 inode_number, int error_code, u64 duration_ns);
extern int vexfs_rust_hook_system_event(u32 event_type, const char *device_path, const char *mount_point, u32 flags);
extern int vexfs_rust_set_kernel_hooks_enabled(int enabled);
extern int vexfs_rust_get_kernel_hook_stats(u64 *total_events, u64 *filesystem_events, u64 *system_events, u64 *error_events);

#ifdef __cplusplus
}
#endif

#endif /* VEXFS_SEMANTIC_HOOKS_H */