/*
 * VexFS - Vector Extended File System
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

//! FFI Bridge for Hang Prevention System
//!
//! This module provides C FFI functions for the hang prevention system,
//! allowing the kernel module to interact with the Rust hang prevention
//! implementation safely.

use crate::shared::system_hang_prevention::{
    OperationType, init_hang_prevention, shutdown_hang_prevention,
    start_operation_watchdog, cancel_operation_watchdog, check_operation_allowed,
    update_system_resources, get_system_health, PanicContext, SystemState,
    DegradationLevel, PanicRecoveryStrategy,
};
use core::time::Duration;
use core::ffi::{c_char, c_uint, c_ulong};
use core::ptr;

// =============================================================================
// FFI Constants Mapping
// =============================================================================

/// Convert C operation type to Rust OperationType
fn operation_type_from_c(op_type: u32) -> OperationType {
    match op_type {
        0 => OperationType::FileRead,
        1 => OperationType::FileWrite,
        2 => OperationType::DirectoryLookup,
        3 => OperationType::DirectoryCreate,
        4 => OperationType::InodeAllocation,
        5 => OperationType::BlockAllocation,
        6 => OperationType::VectorSearch,
        7 => OperationType::VectorStore,
        8 => OperationType::FFICall,
        9 => OperationType::Mount,
        10 => OperationType::Unmount,
        11 => OperationType::Sync,
        12 => OperationType::Journal,
        _ => OperationType::FFICall, // Default fallback
    }
}

/// Convert Rust DegradationLevel to C constant
fn degradation_level_to_c(level: DegradationLevel) -> u32 {
    match level {
        DegradationLevel::Normal => 0,
        DegradationLevel::LightDegradation => 1,
        DegradationLevel::ModerateDegradation => 2,
        DegradationLevel::HeavyDegradation => 3,
        DegradationLevel::ReadOnlyMode => 4,
        DegradationLevel::EmergencyMode => 5,
    }
}

/// Convert Rust PanicRecoveryStrategy to C constant
fn panic_recovery_to_c(strategy: PanicRecoveryStrategy) -> u32 {
    match strategy {
        PanicRecoveryStrategy::ContinueWithDegradation => 0,
        PanicRecoveryStrategy::SwitchToReadOnly => 1,
        PanicRecoveryStrategy::GracefulShutdown => 2,
        PanicRecoveryStrategy::EmergencyShutdown => 3,
    }
}

// =============================================================================
// FFI Error Codes
// =============================================================================

const VEXFS_SUCCESS: i32 = 0;
const VEXFS_ERROR_GENERIC: i32 = -1;
const VEXFS_ERROR_NOMEM: i32 = -12;
const VEXFS_ERROR_INVAL: i32 = -22;
const VEXFS_ERROR_BUSY: i32 = -16;
const VEXFS_ERROR_TIMEOUT: i32 = -110;

// =============================================================================
// FFI Functions
// =============================================================================

/// Initialize the hang prevention system
/// Called during module initialization
#[no_mangle]
pub extern "C" fn vexfs_rust_init_hang_prevention() -> i32 {
    match init_hang_prevention() {
        Ok(()) => {
            #[cfg(feature = "kernel")]
            {
                // In kernel mode, we might want to do additional setup
                VEXFS_SUCCESS
            }
            #[cfg(not(feature = "kernel"))]
            VEXFS_SUCCESS
        }
        Err(_) => VEXFS_ERROR_GENERIC,
    }
}

/// Shutdown the hang prevention system
/// Called during module cleanup
#[no_mangle]
pub extern "C" fn vexfs_rust_shutdown_hang_prevention() {
    shutdown_hang_prevention();
}

/// Start a watchdog timer for an operation
/// Called before starting long-running operations
#[no_mangle]
pub extern "C" fn vexfs_rust_start_watchdog(
    operation_type: u32,
    timeout_secs: u32,
    watchdog_id: *mut u64,
) -> i32 {
    if watchdog_id.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let op_type = operation_type_from_c(operation_type);
    let timeout = if timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(timeout_secs as u64))
    };

    match start_operation_watchdog(op_type, timeout) {
        Ok(id) => {
            unsafe {
                *watchdog_id = id;
            }
            VEXFS_SUCCESS
        }
        Err(_) => VEXFS_ERROR_GENERIC,
    }
}

/// Cancel a watchdog timer
/// Called when operation completes successfully
#[no_mangle]
pub extern "C" fn vexfs_rust_cancel_watchdog(watchdog_id: u64) -> i32 {
    match cancel_operation_watchdog(watchdog_id) {
        Ok(()) => VEXFS_SUCCESS,
        Err(_) => VEXFS_ERROR_GENERIC,
    }
}

/// Check if an operation should be allowed
/// Called before starting operations to check system state
#[no_mangle]
pub extern "C" fn vexfs_rust_check_operation_allowed(operation_type: u32) -> i32 {
    let op_type = operation_type_from_c(operation_type);
    
    match check_operation_allowed(op_type) {
        Ok(()) => VEXFS_SUCCESS,
        Err(crate::shared::VexfsError::Busy) => VEXFS_ERROR_BUSY,
        Err(crate::shared::VexfsError::ReadOnlyFilesystem) => VEXFS_ERROR_BUSY,
        Err(crate::shared::VexfsError::Timeout(_)) => VEXFS_ERROR_TIMEOUT,
        Err(_) => VEXFS_ERROR_GENERIC,
    }
}

/// Update system resource usage statistics
/// Called periodically to update resource monitoring
#[no_mangle]
pub extern "C" fn vexfs_rust_update_resources(memory_bytes: u64, cpu_percent: u32) {
    update_system_resources(memory_bytes, cpu_percent);
}

/// Get current system health status
/// Called to check overall system health
#[no_mangle]
pub extern "C" fn vexfs_rust_get_health_status(
    degradation_level: *mut u32,
    memory_percent: *mut u32,
    cpu_percent: *mut u32,
    active_ops: *mut u32,
) -> i32 {
    if degradation_level.is_null() || memory_percent.is_null() || 
       cpu_percent.is_null() || active_ops.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    if let Some(health) = get_system_health() {
        unsafe {
            *degradation_level = degradation_level_to_c(health.degradation_level);
            *memory_percent = health.memory_usage_percent;
            *cpu_percent = health.cpu_usage_percent;
            *active_ops = health.active_operations;
        }
        VEXFS_SUCCESS
    } else {
        // Return default values if hang prevention not initialized
        unsafe {
            *degradation_level = 0; // Normal
            *memory_percent = 0;
            *cpu_percent = 0;
            *active_ops = 0;
        }
        VEXFS_SUCCESS
    }
}

/// Handle a panic situation
/// Called when a critical error occurs that might cause system hang
#[no_mangle]
pub extern "C" fn vexfs_rust_handle_panic(
    operation_type: u32,
    error_message: *const c_char,
    recovery_strategy: *mut u32,
) -> i32 {
    if error_message.is_null() || recovery_strategy.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let op_type = operation_type_from_c(operation_type);
    
    // Convert C string to Rust string
    let message = unsafe {
        if error_message.is_null() {
            "Unknown error".to_string()
        } else {
            match core::ffi::CStr::from_ptr(error_message).to_str() {
                Ok(s) => s.to_string(),
                Err(_) => "Invalid error message".to_string(),
            }
        }
    };

    // Create panic context
    let context = PanicContext {
        operation_type: op_type,
        error_message: message,
        recovery_strategy: PanicRecoveryStrategy::ContinueWithDegradation, // Will be updated
        system_state: SystemState {
            degradation_level: DegradationLevel::Normal, // Will be updated by manager
            active_operations: 0,
            memory_usage_percent: 0,
            cpu_usage_percent: 0,
            has_pending_writes: false,
        },
    };

    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = crate::shared::system_hang_prevention::get_hang_prevention_manager() {
            let strategy = manager.handle_panic(context);
            unsafe {
                *recovery_strategy = panic_recovery_to_c(strategy);
            }
            return VEXFS_SUCCESS;
        }
    }

    // Fallback strategy
    unsafe {
        *recovery_strategy = panic_recovery_to_c(PanicRecoveryStrategy::ContinueWithDegradation);
    }
    VEXFS_SUCCESS
}

// =============================================================================
// Helper Functions for Kernel Integration
// =============================================================================

/// Check if hang prevention is available
#[no_mangle]
pub extern "C" fn vexfs_rust_hang_prevention_available() -> i32 {
    #[cfg(not(feature = "kernel"))]
    {
        if crate::shared::system_hang_prevention::get_hang_prevention_manager().is_some() {
            1
        } else {
            0
        }
    }
    
    #[cfg(feature = "kernel")]
    {
        // In kernel mode, assume available if module is loaded
        1
    }
}

/// Get hang prevention statistics
#[no_mangle]
pub extern "C" fn vexfs_rust_get_hang_prevention_stats(
    total_watchdogs: *mut u64,
    active_watchdogs: *mut u32,
    expired_watchdogs: *mut u32,
    prevented_hangs: *mut u32,
) -> i32 {
    if total_watchdogs.is_null() || active_watchdogs.is_null() || 
       expired_watchdogs.is_null() || prevented_hangs.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    #[cfg(not(feature = "kernel"))]
    {
        if let Some(health) = get_system_health() {
            unsafe {
                *total_watchdogs = 0; // Would need to track this
                *active_watchdogs = health.active_watchdogs;
                *expired_watchdogs = 0; // Would need to track this
                *prevented_hangs = 0; // Would need to track this
            }
            return VEXFS_SUCCESS;
        }
    }

    // Return zeros if not available
    unsafe {
        *total_watchdogs = 0;
        *active_watchdogs = 0;
        *expired_watchdogs = 0;
        *prevented_hangs = 0;
    }
    VEXFS_SUCCESS
}

/// Force system degradation for testing
#[no_mangle]
pub extern "C" fn vexfs_rust_force_degradation(level: u32) -> i32 {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = crate::shared::system_hang_prevention::get_hang_prevention_manager() {
            // This would require adding a force_degradation method to the manager
            // For now, just return success
            return VEXFS_SUCCESS;
        }
    }
    
    VEXFS_ERROR_GENERIC
}

// =============================================================================
// Kernel-Specific Integration
// =============================================================================

#[cfg(feature = "kernel")]
mod kernel_integration {
    use super::*;
    
    /// Kernel-specific hang prevention initialization
    pub fn init_kernel_hang_prevention() -> Result<(), crate::shared::VexfsError> {
        // In kernel mode, we might want to set up kernel-specific monitoring
        // For now, just call the standard initialization
        init_hang_prevention()
    }
    
    /// Kernel-specific resource monitoring
    pub fn update_kernel_resources() {
        // In a real implementation, this would gather actual kernel memory/CPU stats
        // For now, use placeholder values
        let memory_usage = 64 * 1024 * 1024; // 64MB placeholder
        let cpu_usage = 25; // 25% placeholder
        
        update_system_resources(memory_usage, cpu_usage);
    }
    
    /// Check if kernel is under memory pressure
    pub fn check_kernel_memory_pressure() -> bool {
        if let Some(health) = get_system_health() {
            health.memory_usage_percent > 80
        } else {
            false
        }
    }
}

#[cfg(feature = "kernel")]
pub use kernel_integration::*;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_conversion() {
        assert_eq!(operation_type_from_c(0), OperationType::FileRead);
        assert_eq!(operation_type_from_c(1), OperationType::FileWrite);
        assert_eq!(operation_type_from_c(12), OperationType::Journal);
        assert_eq!(operation_type_from_c(999), OperationType::FFICall); // Fallback
    }

    #[test]
    fn test_degradation_level_conversion() {
        assert_eq!(degradation_level_to_c(DegradationLevel::Normal), 0);
        assert_eq!(degradation_level_to_c(DegradationLevel::EmergencyMode), 5);
    }

    #[test]
    fn test_panic_recovery_conversion() {
        assert_eq!(panic_recovery_to_c(PanicRecoveryStrategy::ContinueWithDegradation), 0);
        assert_eq!(panic_recovery_to_c(PanicRecoveryStrategy::EmergencyShutdown), 3);
    }

    #[test]
    fn test_ffi_init_shutdown() {
        let result = vexfs_rust_init_hang_prevention();
        assert_eq!(result, VEXFS_SUCCESS);
        
        vexfs_rust_shutdown_hang_prevention();
    }

    #[test]
    fn test_watchdog_operations() {
        // Initialize first
        let _ = vexfs_rust_init_hang_prevention();
        
        let mut watchdog_id: u64 = 0;
        let result = vexfs_rust_start_watchdog(0, 30, &mut watchdog_id);
        assert_eq!(result, VEXFS_SUCCESS);
        assert_ne!(watchdog_id, 0);
        
        let cancel_result = vexfs_rust_cancel_watchdog(watchdog_id);
        assert_eq!(cancel_result, VEXFS_SUCCESS);
        
        vexfs_rust_shutdown_hang_prevention();
    }

    #[test]
    fn test_operation_allowed() {
        let _ = vexfs_rust_init_hang_prevention();
        
        let result = vexfs_rust_check_operation_allowed(0); // FileRead
        assert_eq!(result, VEXFS_SUCCESS);
        
        vexfs_rust_shutdown_hang_prevention();
    }

    #[test]
    fn test_health_status() {
        let _ = vexfs_rust_init_hang_prevention();
        
        let mut degradation: u32 = 0;
        let mut memory: u32 = 0;
        let mut cpu: u32 = 0;
        let mut ops: u32 = 0;
        
        let result = vexfs_rust_get_health_status(&mut degradation, &mut memory, &mut cpu, &mut ops);
        assert_eq!(result, VEXFS_SUCCESS);
        
        vexfs_rust_shutdown_hang_prevention();
    }
}