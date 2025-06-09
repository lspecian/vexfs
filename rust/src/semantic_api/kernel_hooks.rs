//! Kernel-Level Event Hooks for VexFS Semantic Operation Journal
//! 
//! This module provides hooks for intercepting filesystem operations at the
//! kernel level and emitting appropriate semantic events.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void, c_uint, c_ulong};
use std::ptr;

use crate::semantic_api::types::{SemanticEventType, EventFlags, EventPriority};
use crate::semantic_api::event_emission::{emit_filesystem_event, get_global_emission_framework};

/// Kernel operation types for event interception
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelOperationType {
    FileOpen = 0,
    FileClose = 1,
    FileRead = 2,
    FileWrite = 3,
    FileCreate = 4,
    FileDelete = 5,
    FileRename = 6,
    FileChmod = 7,
    FileChown = 8,
    FileTruncate = 9,
    DirCreate = 10,
    DirDelete = 11,
    DirRead = 12,
    SymlinkCreate = 13,
    HardlinkCreate = 14,
    Mount = 15,
    Unmount = 16,
    Sync = 17,
}

/// Kernel event context passed from C kernel module
#[repr(C)]
#[derive(Debug)]
pub struct KernelEventContext {
    pub operation_type: KernelOperationType,
    pub path: *const c_char,
    pub path_len: c_uint,
    pub inode_number: c_ulong,
    pub file_size: c_ulong,
    pub mode: c_uint,
    pub uid: c_uint,
    pub gid: c_uint,
    pub pid: c_uint,
    pub tid: c_uint,
    pub timestamp_sec: c_ulong,
    pub timestamp_nsec: c_ulong,
    pub flags: c_uint,
    pub error_code: c_int,
}

/// Hook registration structure
#[derive(Debug)]
pub struct KernelHookRegistry {
    pub filesystem_hooks_enabled: bool,
    pub system_hooks_enabled: bool,
    pub performance_monitoring: bool,
    pub error_tracking: bool,
}

impl Default for KernelHookRegistry {
    fn default() -> Self {
        Self {
            filesystem_hooks_enabled: true,
            system_hooks_enabled: true,
            performance_monitoring: true,
            error_tracking: true,
        }
    }
}

/// Global hook registry
static mut KERNEL_HOOK_REGISTRY: Option<KernelHookRegistry> = None;
static HOOK_INIT_ONCE: std::sync::Once = std::sync::Once::new();

/// Initialize kernel hooks
pub fn initialize_kernel_hooks() -> Result<(), Box<dyn std::error::Error>> {
    HOOK_INIT_ONCE.call_once(|| {
        unsafe {
            KERNEL_HOOK_REGISTRY = Some(KernelHookRegistry::default());
        }
    });
    
    tracing::info!("Kernel hooks initialized");
    Ok(())
}

/// Check if kernel hooks are enabled
pub fn are_kernel_hooks_enabled() -> bool {
    unsafe {
        KERNEL_HOOK_REGISTRY
            .as_ref()
            .map(|registry| registry.filesystem_hooks_enabled)
            .unwrap_or(false)
    }
}

/// Main kernel event hook function called from C kernel module
/// 
/// # Safety
/// This function is called from C kernel code and must handle raw pointers safely
#[no_mangle]
pub unsafe extern "C" fn vexfs_rust_emit_kernel_event(
    context: *const KernelEventContext
) -> c_int {
    if context.is_null() {
        return -1; // VEXFS_ERROR_INVAL
    }
    
    let ctx = &*context;
    
    // Check if hooks are enabled
    if !are_kernel_hooks_enabled() {
        return 0; // Success but no-op
    }
    
    // Convert C string to Rust string
    let path = if !ctx.path.is_null() && ctx.path_len > 0 {
        match CStr::from_ptr(ctx.path).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -22, // VEXFS_ERROR_INVAL
        }
    } else {
        "<unknown>".to_string()
    };
    
    // Map kernel operation to semantic event type
    let event_type = match ctx.operation_type {
        KernelOperationType::FileOpen => SemanticEventType::FilesystemRead,
        KernelOperationType::FileClose => SemanticEventType::FilesystemRead, // No specific close event
        KernelOperationType::FileRead => SemanticEventType::FilesystemRead,
        KernelOperationType::FileWrite => SemanticEventType::FilesystemWrite,
        KernelOperationType::FileCreate => SemanticEventType::FilesystemCreate,
        KernelOperationType::FileDelete => SemanticEventType::FilesystemDelete,
        KernelOperationType::FileRename => SemanticEventType::FilesystemRename,
        KernelOperationType::FileChmod => SemanticEventType::FilesystemChmod,
        KernelOperationType::FileChown => SemanticEventType::FilesystemChown,
        KernelOperationType::FileTruncate => SemanticEventType::FilesystemTruncate,
        KernelOperationType::DirCreate => SemanticEventType::FilesystemMkdir,
        KernelOperationType::DirDelete => SemanticEventType::FilesystemRmdir,
        KernelOperationType::DirRead => SemanticEventType::FilesystemRead,
        KernelOperationType::SymlinkCreate => SemanticEventType::FilesystemSymlink,
        KernelOperationType::HardlinkCreate => SemanticEventType::FilesystemHardlink,
        KernelOperationType::Mount => SemanticEventType::SystemMount,
        KernelOperationType::Unmount => SemanticEventType::SystemUnmount,
        KernelOperationType::Sync => SemanticEventType::SystemSync,
    };
    
    // Determine file type from mode
    let file_type = if ctx.mode & 0o170000 == 0o040000 {
        Some("directory".to_string())
    } else if ctx.mode & 0o170000 == 0o100000 {
        Some("regular".to_string())
    } else if ctx.mode & 0o170000 == 0o120000 {
        Some("symlink".to_string())
    } else {
        Some("other".to_string())
    };
    
    // Emit the event
    match emit_filesystem_event(
        event_type,
        path,
        Some(ctx.inode_number),
        file_type,
    ) {
        Ok(_) => 0, // VEXFS_SUCCESS
        Err(_) => -1, // VEXFS_ERROR_GENERIC
    }
}

/// Hook for filesystem operation start
/// Called before filesystem operations to emit start events
#[no_mangle]
pub unsafe extern "C" fn vexfs_rust_hook_fs_operation_start(
    operation_type: c_uint,
    path: *const c_char,
    inode_number: c_ulong,
) -> c_int {
    if !are_kernel_hooks_enabled() {
        return 0;
    }
    
    let path_str = if !path.is_null() {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -22, // VEXFS_ERROR_INVAL
        }
    } else {
        "<unknown>".to_string()
    };
    
    // Emit observability event for operation start
    if let Some(framework) = get_global_emission_framework() {
        let flags = EventFlags {
            atomic: false,
            transactional: false,
            causal: true,
            agent_visible: true,
            deterministic: true,
            compressed: false,
            indexed: true,
            replicated: false,
        };
        
        let context = crate::semantic_api::types::SemanticContext {
            transaction_id: None,
            session_id: None,
            causality_chain_id: None,
            filesystem: Some(crate::semantic_api::types::FilesystemContext {
                path: path_str,
                inode_number: Some(inode_number),
                file_type: None,
            }),
            graph: None,
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: Some(crate::semantic_api::types::ObservabilityContext {
                metric_name: Some("fs_operation_start".to_string()),
                metric_value: Some(operation_type as f64),
                metric_unit: Some("count".to_string()),
                log_level: Some("debug".to_string()),
                log_message: Some(format!("Filesystem operation {} started", operation_type)),
                trace_id: None,
                span_id: None,
                parent_span_id: None,
                service_name: Some("vexfs_kernel".to_string()),
                operation_name: Some("fs_operation".to_string()),
                resource_type: Some("filesystem".to_string()),
                threshold_value: None,
                alert_severity: None,
            }),
        };
        
        match framework.lock().unwrap().emit_event(
            SemanticEventType::ObservabilityTraceSpanStart,
            context,
            flags,
            EventPriority::Low,
            None,
            None,
        ) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        0
    }
}

/// Hook for filesystem operation end
/// Called after filesystem operations to emit completion events
#[no_mangle]
pub unsafe extern "C" fn vexfs_rust_hook_fs_operation_end(
    operation_type: c_uint,
    path: *const c_char,
    inode_number: c_ulong,
    error_code: c_int,
    duration_ns: c_ulong,
) -> c_int {
    if !are_kernel_hooks_enabled() {
        return 0;
    }
    
    let path_str = if !path.is_null() {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -22, // VEXFS_ERROR_INVAL
        }
    } else {
        "<unknown>".to_string()
    };
    
    // Emit observability event for operation end
    if let Some(framework) = get_global_emission_framework() {
        let flags = EventFlags {
            atomic: false,
            transactional: false,
            causal: true,
            agent_visible: true,
            deterministic: true,
            compressed: false,
            indexed: true,
            replicated: false,
        };
        
        let context = crate::semantic_api::types::SemanticContext {
            transaction_id: None,
            session_id: None,
            causality_chain_id: None,
            filesystem: Some(crate::semantic_api::types::FilesystemContext {
                path: path_str,
                inode_number: Some(inode_number),
                file_type: None,
            }),
            graph: None,
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: Some(crate::semantic_api::types::ObservabilityContext {
                metric_name: Some("fs_operation_duration".to_string()),
                metric_value: Some(duration_ns as f64),
                metric_unit: Some("nanoseconds".to_string()),
                log_level: if error_code == 0 { Some("debug".to_string()) } else { Some("error".to_string()) },
                log_message: Some(format!(
                    "Filesystem operation {} completed with code {} in {}ns", 
                    operation_type, error_code, duration_ns
                )),
                trace_id: None,
                span_id: None,
                parent_span_id: None,
                service_name: Some("vexfs_kernel".to_string()),
                operation_name: Some("fs_operation".to_string()),
                resource_type: Some("filesystem".to_string()),
                threshold_value: None,
                alert_severity: if error_code != 0 { Some("warning".to_string()) } else { None },
            }),
        };
        
        let event_type = if error_code == 0 {
            SemanticEventType::ObservabilityTraceSpanEnd
        } else {
            SemanticEventType::ObservabilityErrorReported
        };
        
        match framework.lock().unwrap().emit_event(
            event_type,
            context,
            flags,
            EventPriority::Low,
            None,
            None,
        ) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        0
    }
}

/// Hook for system events (mount, unmount, sync)
#[no_mangle]
pub unsafe extern "C" fn vexfs_rust_hook_system_event(
    event_type: c_uint,
    device_path: *const c_char,
    mount_point: *const c_char,
    flags: c_uint,
) -> c_int {
    if !are_kernel_hooks_enabled() {
        return 0;
    }
    
    let device_str = if !device_path.is_null() {
        match CStr::from_ptr(device_path).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -22, // VEXFS_ERROR_INVAL
        }
    } else {
        "<unknown>".to_string()
    };
    
    let mount_str = if !mount_point.is_null() {
        match CStr::from_ptr(mount_point).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -22, // VEXFS_ERROR_INVAL
        }
    } else {
        "<unknown>".to_string()
    };
    
    let semantic_event_type = match event_type {
        0 => SemanticEventType::SystemMount,
        1 => SemanticEventType::SystemUnmount,
        2 => SemanticEventType::SystemSync,
        _ => return -22, // VEXFS_ERROR_INVAL
    };
    
    // Emit system event
    match emit_filesystem_event(
        semantic_event_type,
        format!("{}:{}", device_str, mount_str),
        None,
        Some("system".to_string()),
    ) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Enable or disable kernel hooks
#[no_mangle]
pub unsafe extern "C" fn vexfs_rust_set_kernel_hooks_enabled(enabled: c_int) -> c_int {
    if let Some(registry) = &mut KERNEL_HOOK_REGISTRY {
        registry.filesystem_hooks_enabled = enabled != 0;
        registry.system_hooks_enabled = enabled != 0;
        0
    } else {
        -1
    }
}

/// Get kernel hook statistics
#[no_mangle]
pub unsafe extern "C" fn vexfs_rust_get_kernel_hook_stats(
    total_events: *mut c_ulong,
    filesystem_events: *mut c_ulong,
    system_events: *mut c_ulong,
    error_events: *mut c_ulong,
) -> c_int {
    if total_events.is_null() || filesystem_events.is_null() || 
       system_events.is_null() || error_events.is_null() {
        return -22; // VEXFS_ERROR_INVAL
    }
    
    if let Some(framework) = get_global_emission_framework() {
        let stats = framework.lock().unwrap().get_stats();
        
        *total_events = stats.total_events_emitted;
        *filesystem_events = stats.events_by_category.get("Filesystem").copied().unwrap_or(0);
        *system_events = stats.events_by_category.get("System").copied().unwrap_or(0);
        *error_events = stats.events_dropped;
        
        0
    } else {
        -1
    }
}

/// Cleanup kernel hooks
pub fn cleanup_kernel_hooks() {
    unsafe {
        KERNEL_HOOK_REGISTRY = None;
    }
    tracing::info!("Kernel hooks cleaned up");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_kernel_hook_initialization() {
        initialize_kernel_hooks().unwrap();
        assert!(are_kernel_hooks_enabled());
    }

    #[test]
    fn test_kernel_event_context() {
        let path = CString::new("/test/file.txt").unwrap();
        let context = KernelEventContext {
            operation_type: KernelOperationType::FileCreate,
            path: path.as_ptr(),
            path_len: path.as_bytes().len() as c_uint,
            inode_number: 12345,
            file_size: 1024,
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            pid: 1234,
            tid: 1234,
            timestamp_sec: 1234567890,
            timestamp_nsec: 123456789,
            flags: 0,
            error_code: 0,
        };
        
        // Test that we can safely access the context
        assert_eq!(context.operation_type, KernelOperationType::FileCreate);
        assert_eq!(context.inode_number, 12345);
        assert_eq!(context.file_size, 1024);
    }
}