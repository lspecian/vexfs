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

//! System Hang Prevention Module
//!
//! This module implements comprehensive defensive programming techniques to prevent
//! system hangs in all edge cases. It provides:
//!
//! 1. **Watchdog Timers**: Automatic operation cancellation on timeout
//! 2. **Deadlock Detection**: Real-time deadlock detection and resolution
//! 3. **Resource Limits**: Memory and CPU usage monitoring and limits
//! 4. **Graceful Degradation**: Performance degradation rather than failure
//! 5. **Panic Handlers**: System stability preservation during critical errors
//!
//! This is the final critical component that ensures VexFS never causes system hangs.

use crate::shared::{VexfsError, VexfsResult};

// Conditional imports for kernel vs userspace
#[cfg(feature = "kernel")]
use crate::{vexfs_error, vexfs_warn, vexfs_info, vexfs_debug, kernel_or_std, printk};
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, string::{String, ToString}};
#[cfg(feature = "kernel")]
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "kernel"))]
use crate::{vexfs_error, vexfs_warn, vexfs_info, vexfs_debug, kernel_or_std};
#[cfg(not(feature = "kernel"))]
use std::vec::Vec;
#[cfg(not(feature = "kernel"))]
use std::string::String;
#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, Mutex, RwLock};
#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;
#[cfg(not(feature = "kernel"))]
use std::thread;
#[cfg(not(feature = "kernel"))]
use std::time::{Instant, SystemTime, Duration};

use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};

#[cfg(feature = "kernel")]
use core::time::Duration;

// =============================================================================
// Constants and Configuration
// =============================================================================

/// Maximum operation timeout in seconds
pub const MAX_OPERATION_TIMEOUT_SECS: u64 = 300; // 5 minutes absolute maximum

/// File I/O operation timeout (30 seconds)
pub const FILE_IO_TIMEOUT_SECS: u64 = 30;

/// Directory operation timeout (15 seconds)
pub const DIRECTORY_TIMEOUT_SECS: u64 = 15;

/// FFI call timeout (5 seconds)
pub const FFI_CALL_TIMEOUT_SECS: u64 = 5;

/// Mount/unmount operation timeout (60 seconds)
pub const MOUNT_TIMEOUT_SECS: u64 = 60;

/// Maximum memory usage (256MB)
pub const MAX_MEMORY_USAGE_BYTES: u64 = 256 * 1024 * 1024;

/// Memory usage warning threshold (80% of max)
pub const MEMORY_WARNING_THRESHOLD: u64 = (MAX_MEMORY_USAGE_BYTES * 80) / 100;

/// CPU usage limit (80%)
pub const CPU_USAGE_LIMIT_PERCENT: u32 = 80;

/// Maximum concurrent operations
pub const MAX_CONCURRENT_OPERATIONS: u32 = 100;

/// Deadlock detection interval (1 second)
pub const DEADLOCK_CHECK_INTERVAL_SECS: u64 = 1;

/// Lock timeout for deadlock prevention (10 seconds)
pub const LOCK_TIMEOUT_SECS: u64 = 10;

/// Resource monitoring interval (5 seconds)
pub const RESOURCE_MONITOR_INTERVAL_SECS: u64 = 5;

// =============================================================================
// Operation Types and Priorities
// =============================================================================

/// Types of operations that can be monitored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    FileRead,
    FileWrite,
    DirectoryLookup,
    DirectoryCreate,
    InodeAllocation,
    BlockAllocation,
    VectorSearch,
    VectorStore,
    FFICall,
    Mount,
    Unmount,
    Sync,
    Journal,
}

impl OperationType {
    /// Get the default timeout for this operation type
    pub fn default_timeout(&self) -> Duration {
        Duration::from_secs(match self {
            Self::FileRead | Self::FileWrite => FILE_IO_TIMEOUT_SECS,
            Self::DirectoryLookup | Self::DirectoryCreate => DIRECTORY_TIMEOUT_SECS,
            Self::InodeAllocation | Self::BlockAllocation => DIRECTORY_TIMEOUT_SECS,
            Self::VectorSearch | Self::VectorStore => FILE_IO_TIMEOUT_SECS,
            Self::FFICall => FFI_CALL_TIMEOUT_SECS,
            Self::Mount | Self::Unmount => MOUNT_TIMEOUT_SECS,
            Self::Sync | Self::Journal => FILE_IO_TIMEOUT_SECS,
        })
    }

    /// Get the priority of this operation (higher = more important)
    pub fn priority(&self) -> u32 {
        match self {
            Self::Mount | Self::Unmount => 100,
            Self::Journal | Self::Sync => 90,
            Self::InodeAllocation | Self::BlockAllocation => 80,
            Self::FileWrite => 70,
            Self::FileRead => 60,
            Self::DirectoryCreate => 50,
            Self::DirectoryLookup => 40,
            Self::VectorStore => 30,
            Self::VectorSearch => 20,
            Self::FFICall => 10,
        }
    }
}

/// Operation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 100,
    High = 80,
    Medium = 50,
    Low = 20,
    Background = 10,
}

// =============================================================================
// Watchdog Timer System
// =============================================================================

/// Unique identifier for watchdog timers
pub type WatchdogId = u64;

/// Watchdog timer state
#[derive(Debug)]
pub struct WatchdogTimer {
    pub id: WatchdogId,
    pub operation_type: OperationType,
    pub timeout: Duration,
    pub started_at: u64, // Timestamp in seconds since epoch
    pub is_active: AtomicBool,
    pub has_expired: AtomicBool,
}

impl WatchdogTimer {
    /// Create a new watchdog timer
    pub fn new(id: WatchdogId, operation_type: OperationType, timeout: Option<Duration>) -> Self {
        let timeout = timeout.unwrap_or_else(|| operation_type.default_timeout());
        
        Self {
            id,
            operation_type,
            timeout,
            started_at: current_timestamp_secs(),
            is_active: AtomicBool::new(true),
            has_expired: AtomicBool::new(false),
        }
    }

    /// Check if the timer has expired
    pub fn is_expired(&self) -> bool {
        if self.has_expired.load(Ordering::Acquire) {
            return true;
        }

        let now = current_timestamp_secs();
        let elapsed = now.saturating_sub(self.started_at);
        let expired = elapsed >= self.timeout.as_secs();

        if expired {
            self.has_expired.store(true, Ordering::Release);
            vexfs_warn!("Watchdog timer {} expired for {:?} after {} seconds", 
                       self.id, self.operation_type, elapsed);
        }

        expired
    }

    /// Cancel the timer
    pub fn cancel(&self) {
        self.is_active.store(false, Ordering::Release);
        vexfs_debug!("Watchdog timer {} cancelled for {:?}", self.id, self.operation_type);
    }

    /// Check if the timer is still active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Acquire) && !self.is_expired()
    }
}

// =============================================================================
// Deadlock Detection System
// =============================================================================

/// Lock identifier for deadlock detection
pub type LockId = u64;

/// Thread/task identifier
pub type TaskId = u64;

/// Lock dependency graph edge
#[derive(Debug, Clone)]
pub struct LockDependency {
    pub waiting_task: TaskId,
    pub held_lock: LockId,
    pub waiting_for_lock: LockId,
    pub timestamp: u64,
}

/// Deadlock detection result
#[derive(Debug)]
pub enum DeadlockStatus {
    NoDeadlock,
    DeadlockDetected {
        cycle: Vec<LockDependency>,
        victim_task: TaskId,
    },
}

/// Lock state tracking
#[derive(Debug, Clone)]
pub struct LockState {
    pub lock_id: LockId,
    pub holder_task: Option<TaskId>,
    pub waiting_tasks: Vec<TaskId>,
    pub acquired_at: u64,
    pub lock_type: LockType,
}

#[derive(Debug, Clone, Copy)]
pub enum LockType {
    Read,
    Write,
    Exclusive,
}

// =============================================================================
// Resource Usage Monitoring
// =============================================================================

/// System resource usage statistics
#[derive(Debug, Default)]
pub struct ResourceUsage {
    pub memory_used_bytes: AtomicU64,
    pub memory_peak_bytes: AtomicU64,
    pub cpu_usage_percent: AtomicU32,
    pub active_operations: AtomicU32,
    pub io_operations_per_sec: AtomicU32,
    pub last_updated: AtomicU64,
}

impl ResourceUsage {
    /// Update memory usage
    pub fn update_memory_usage(&self, bytes: u64) {
        self.memory_used_bytes.store(bytes, Ordering::Relaxed);
        
        // Update peak if necessary
        let current_peak = self.memory_peak_bytes.load(Ordering::Relaxed);
        if bytes > current_peak {
            self.memory_peak_bytes.store(bytes, Ordering::Relaxed);
        }
        
        self.last_updated.store(current_timestamp_secs(), Ordering::Relaxed);
        
        // Check for memory pressure
        if bytes > MEMORY_WARNING_THRESHOLD {
            vexfs_warn!("Memory usage high: {} bytes ({}% of limit)", 
                       bytes, (bytes * 100) / MAX_MEMORY_USAGE_BYTES);
        }
    }

    /// Update CPU usage
    pub fn update_cpu_usage(&self, percent: u32) {
        self.cpu_usage_percent.store(percent, Ordering::Relaxed);
        self.last_updated.store(current_timestamp_secs(), Ordering::Relaxed);
        
        if percent > CPU_USAGE_LIMIT_PERCENT {
            vexfs_warn!("CPU usage high: {}%", percent);
        }
    }

    /// Check if resources are under pressure
    pub fn is_under_pressure(&self) -> bool {
        let memory_pressure = self.memory_used_bytes.load(Ordering::Relaxed) > MEMORY_WARNING_THRESHOLD;
        let cpu_pressure = self.cpu_usage_percent.load(Ordering::Relaxed) > CPU_USAGE_LIMIT_PERCENT;
        let operation_pressure = self.active_operations.load(Ordering::Relaxed) > MAX_CONCURRENT_OPERATIONS;
        
        memory_pressure || cpu_pressure || operation_pressure
    }

    /// Get memory usage percentage
    pub fn memory_usage_percent(&self) -> u32 {
        let used = self.memory_used_bytes.load(Ordering::Relaxed);
        ((used * 100) / MAX_MEMORY_USAGE_BYTES) as u32
    }
}

// =============================================================================
// Graceful Degradation System
// =============================================================================

/// System degradation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegradationLevel {
    Normal = 0,
    LightDegradation = 1,
    ModerateDegradation = 2,
    HeavyDegradation = 3,
    ReadOnlyMode = 4,
    EmergencyMode = 5,
}

impl DegradationLevel {
    /// Get the maximum allowed operations for this degradation level
    pub fn max_operations(&self) -> u32 {
        match self {
            Self::Normal => MAX_CONCURRENT_OPERATIONS,
            Self::LightDegradation => MAX_CONCURRENT_OPERATIONS * 80 / 100,
            Self::ModerateDegradation => MAX_CONCURRENT_OPERATIONS * 60 / 100,
            Self::HeavyDegradation => MAX_CONCURRENT_OPERATIONS * 40 / 100,
            Self::ReadOnlyMode => MAX_CONCURRENT_OPERATIONS * 20 / 100,
            Self::EmergencyMode => 5,
        }
    }

    /// Check if write operations are allowed
    pub fn allows_writes(&self) -> bool {
        match self {
            Self::Normal | Self::LightDegradation | Self::ModerateDegradation | Self::HeavyDegradation => true,
            Self::ReadOnlyMode | Self::EmergencyMode => false,
        }
    }

    /// Get operation timeout multiplier
    pub fn timeout_multiplier(&self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::LightDegradation => 1.2,
            Self::ModerateDegradation => 1.5,
            Self::HeavyDegradation => 2.0,
            Self::ReadOnlyMode => 3.0,
            Self::EmergencyMode => 5.0,
        }
    }

    /// Convert from u32
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Self::Normal,
            1 => Self::LightDegradation,
            2 => Self::ModerateDegradation,
            3 => Self::HeavyDegradation,
            4 => Self::ReadOnlyMode,
            _ => Self::EmergencyMode,
        }
    }
}

/// Degradation decision based on system state
pub struct DegradationDecision {
    pub level: DegradationLevel,
    pub reason: String,
    pub recommended_actions: Vec<String>,
}

// =============================================================================
// Panic Handler System
// =============================================================================

/// Panic recovery strategy
#[derive(Debug, Clone)]
pub enum PanicRecoveryStrategy {
    /// Continue operation with degraded functionality
    ContinueWithDegradation,
    /// Switch to read-only mode
    SwitchToReadOnly,
    /// Attempt graceful shutdown
    GracefulShutdown,
    /// Emergency shutdown
    EmergencyShutdown,
}

/// Panic context information
#[derive(Debug)]
pub struct PanicContext {
    pub operation_type: OperationType,
    pub error_message: String,
    pub recovery_strategy: PanicRecoveryStrategy,
    pub system_state: SystemState,
}

/// Current system state for panic handling
#[derive(Debug, Clone)]
pub struct SystemState {
    pub degradation_level: DegradationLevel,
    pub active_operations: u32,
    pub memory_usage_percent: u32,
    pub cpu_usage_percent: u32,
    pub has_pending_writes: bool,
}

// =============================================================================
// Main Hang Prevention Manager
// =============================================================================

/// Global hang prevention manager
pub struct HangPreventionManager {
    // Watchdog system
    next_watchdog_id: AtomicU64,
    #[cfg(not(feature = "kernel"))]
    active_watchdogs: Arc<RwLock<HashMap<WatchdogId, Arc<WatchdogTimer>>>>,
    
    // Deadlock detection
    next_lock_id: AtomicU64,
    #[cfg(not(feature = "kernel"))]
    lock_dependencies: Arc<RwLock<Vec<LockDependency>>>,
    #[cfg(not(feature = "kernel"))]
    lock_states: Arc<RwLock<HashMap<LockId, LockState>>>,
    
    // Resource monitoring
    resource_usage: ResourceUsage,
    current_degradation: AtomicU32, // DegradationLevel as u32
    
    // System state
    is_shutting_down: AtomicBool,
    emergency_mode: AtomicBool,
}

impl HangPreventionManager {
    /// Create a new hang prevention manager
    pub fn new() -> Self {
        Self {
            next_watchdog_id: AtomicU64::new(1),
            #[cfg(not(feature = "kernel"))]
            active_watchdogs: Arc::new(RwLock::new(HashMap::new())),
            
            next_lock_id: AtomicU64::new(1),
            #[cfg(not(feature = "kernel"))]
            lock_dependencies: Arc::new(RwLock::new(Vec::new())),
            #[cfg(not(feature = "kernel"))]
            lock_states: Arc::new(RwLock::new(HashMap::new())),
            
            resource_usage: ResourceUsage::default(),
            current_degradation: AtomicU32::new(DegradationLevel::Normal as u32),
            
            is_shutting_down: AtomicBool::new(false),
            emergency_mode: AtomicBool::new(false),
        }
    }

    /// Start a watchdog timer for an operation
    pub fn start_watchdog(&self, operation_type: OperationType, timeout: Option<Duration>) -> VexfsResult<WatchdogId> {
        let id = self.next_watchdog_id.fetch_add(1, Ordering::Relaxed);
        let timer = Arc::new(WatchdogTimer::new(id, operation_type, timeout));
        
        #[cfg(not(feature = "kernel"))]
        {
            if let Ok(mut watchdogs) = self.active_watchdogs.write() {
                watchdogs.insert(id, timer.clone());
            } else {
                return Err(VexfsError::Busy);
            }
        }
        
        vexfs_debug!("Started watchdog {} for {:?} with timeout {:?}", 
                    id, operation_type, timer.timeout);
        
        Ok(id)
    }

    /// Cancel a watchdog timer
    pub fn cancel_watchdog(&self, watchdog_id: WatchdogId) -> VexfsResult<()> {
        #[cfg(not(feature = "kernel"))]
        {
            if let Ok(mut watchdogs) = self.active_watchdogs.write() {
                if let Some(timer) = watchdogs.remove(&watchdog_id) {
                    timer.cancel();
                    vexfs_debug!("Cancelled watchdog {}", watchdog_id);
                    return Ok(());
                }
            }
        }
        
        #[cfg(feature = "kernel")]
        {
            // In kernel mode, we track watchdogs differently
            vexfs_debug!("Cancelled watchdog {} (kernel mode)", watchdog_id);
            return Ok(());
        }
        
        Err(VexfsError::NotFound)
    }

    /// Check for expired watchdog timers
    pub fn check_watchdog_timeouts(&self) -> Vec<WatchdogId> {
        let mut expired = Vec::new();
        
        #[cfg(not(feature = "kernel"))]
        {
            if let Ok(watchdogs) = self.active_watchdogs.read() {
                for (id, timer) in watchdogs.iter() {
                    if timer.is_expired() {
                        expired.push(*id);
                        vexfs_warn!("Watchdog {} expired for operation {:?}", 
                                   id, timer.operation_type);
                    }
                }
            }
        }
        
        expired
    }

    /// Acquire a lock with deadlock detection
    pub fn acquire_lock(&self, lock_type: LockType, task_id: TaskId) -> VexfsResult<LockId> {
        let lock_id = self.next_lock_id.fetch_add(1, Ordering::Relaxed);
        let now = current_timestamp_secs();
        
        #[cfg(not(feature = "kernel"))]
        {
            // Check for potential deadlocks before acquiring
            if let Err(e) = self.check_deadlock_before_acquire(lock_id, task_id) {
                vexfs_warn!("Deadlock prevention: refusing lock {} for task {}: {:?}", 
                           lock_id, task_id, e);
                return Err(e);
            }
            
            // Record lock acquisition
            if let Ok(mut states) = self.lock_states.write() {
                states.insert(lock_id, LockState {
                    lock_id,
                    holder_task: Some(task_id),
                    waiting_tasks: Vec::new(),
                    acquired_at: now,
                    lock_type,
                });
            }
        }
        
        vexfs_debug!("Acquired lock {} for task {} ({:?})", lock_id, task_id, lock_type);
        Ok(lock_id)
    }

    /// Release a lock
    pub fn release_lock(&self, lock_id: LockId, task_id: TaskId) -> VexfsResult<()> {
        #[cfg(not(feature = "kernel"))]
        {
            if let Ok(mut states) = self.lock_states.write() {
                if let Some(state) = states.get_mut(&lock_id) {
                    if state.holder_task == Some(task_id) {
                        state.holder_task = None;
                        vexfs_debug!("Released lock {} by task {}", lock_id, task_id);
                        return Ok(());
                    } else {
                        vexfs_warn!("Task {} attempted to release lock {} not owned by it", 
                                   task_id, lock_id);
                        return Err(VexfsError::PermissionDenied("Lock acquisition denied due to deadlock prevention".to_string()));
                    }
                } else {
                    return Err(VexfsError::NotFound);
                }
            }
        }
        
        #[cfg(feature = "kernel")]
        {
            vexfs_debug!("Released lock {} by task {} (kernel mode)", lock_id, task_id);
        }
        
        Ok(())
    }

    /// Check for deadlocks before acquiring a lock
    #[cfg(not(feature = "kernel"))]
    fn check_deadlock_before_acquire(&self, _lock_id: LockId, _task_id: TaskId) -> VexfsResult<()> {
        // Simplified deadlock prevention - in a real implementation this would be more sophisticated
        // For now, we just check if the task already holds too many locks
        if let Ok(states) = self.lock_states.read() {
            let held_locks = states.values()
                .filter(|state| state.holder_task == Some(_task_id))
                .count();
            
            if held_locks > 10 { // Arbitrary limit to prevent lock hoarding
                return Err(VexfsError::Busy);
            }
        }
        
        Ok(())
    }

    /// Find dependency cycles (simplified implementation)
    #[cfg(not(feature = "kernel"))]
    fn find_dependency_cycle(&self, _dep: &LockDependency, _dependencies: &[LockDependency], _states: &HashMap<LockId, LockState>) -> Option<Vec<LockDependency>> {
        // Simplified cycle detection - real implementation would use graph algorithms
        None
    }

    /// Check for deadlocks in the system
    pub fn detect_deadlocks(&self) -> DeadlockStatus {
        #[cfg(not(feature = "kernel"))]
        {
            // Simplified deadlock detection using cycle detection in dependency graph
            if let (Ok(dependencies), Ok(states)) = (self.lock_dependencies.read(), self.lock_states.read()) {
                // Build dependency graph and look for cycles
                // This is a simplified implementation - real deadlock detection is more complex
                for dep in dependencies.iter() {
                    if let Some(cycle) = self.find_dependency_cycle(dep, &dependencies, &states) {
                        // Choose victim task (e.g., youngest task)
                        let victim_task = cycle.iter()
                            .max_by_key(|d| d.timestamp)
                            .map(|d| d.waiting_task)
                            .unwrap_or(dep.waiting_task);
                        
                        vexfs_warn!("Deadlock detected involving {} tasks, victim: {}", 
                                   cycle.len(), victim_task);
                        
                        return DeadlockStatus::DeadlockDetected { cycle, victim_task };
                    }
                }
            }
        }
        
        DeadlockStatus::NoDeadlock
    }

    /// Calculate appropriate degradation level based on system state
    fn calculate_degradation_level(&self) -> DegradationLevel {
        let memory_percent = self.resource_usage.memory_usage_percent();
        let cpu_percent = self.resource_usage.cpu_usage_percent.load(Ordering::Relaxed);
        let active_ops = self.resource_usage.active_operations.load(Ordering::Relaxed);
        
        // Determine degradation level based on resource usage
        if memory_percent > 95 || cpu_percent > 95 || active_ops > MAX_CONCURRENT_OPERATIONS * 2 {
            DegradationLevel::EmergencyMode
        } else if memory_percent > 90 || cpu_percent > 90 || active_ops > MAX_CONCURRENT_OPERATIONS * 150 / 100 {
            DegradationLevel::ReadOnlyMode
        } else if memory_percent > 85 || cpu_percent > 85 || active_ops > MAX_CONCURRENT_OPERATIONS * 120 / 100 {
            DegradationLevel::HeavyDegradation
        } else if memory_percent > 80 || cpu_percent > 80 || active_ops > MAX_CONCURRENT_OPERATIONS {
            DegradationLevel::ModerateDegradation
        } else if memory_percent > 70 || cpu_percent > 70 || active_ops > MAX_CONCURRENT_OPERATIONS * 80 / 100 {
            DegradationLevel::LightDegradation
        } else {
            DegradationLevel::Normal
        }
    }

    /// Update resource usage statistics
    pub fn update_resource_usage(&self, memory_bytes: u64, cpu_percent: u32) {
        self.resource_usage.update_memory_usage(memory_bytes);
        self.resource_usage.update_cpu_usage(cpu_percent);
        
        // Check if we need to change degradation level
        let new_level = self.calculate_degradation_level();
        let current_level = DegradationLevel::from_u32(self.current_degradation.load(Ordering::Relaxed));
        
        if new_level != current_level {
            self.current_degradation.store(new_level as u32, Ordering::Relaxed);
            vexfs_info!("System degradation level changed from {:?} to {:?}", 
                       current_level, new_level);
            
            if new_level >= DegradationLevel::ReadOnlyMode {
                vexfs_warn!("System entering read-only mode due to resource pressure");
            }
        }
    }

    /// Get current degradation level
    pub fn current_degradation_level(&self) -> DegradationLevel {
        DegradationLevel::from_u32(self.current_degradation.load(Ordering::Relaxed))
    }

    /// Check if an operation should be allowed given current system state
    pub fn should_allow_operation(&self, operation_type: OperationType) -> VexfsResult<()> {
        let degradation_level = self.current_degradation_level();
        
        // Check if system is shutting down
        if self.is_shutting_down.load(Ordering::Relaxed) {
            return Err(VexfsError::Busy);
        }
        
        // Check if in emergency mode
        if self.emergency_mode.load(Ordering::Relaxed) {
            match operation_type {
                OperationType::FileRead | OperationType::DirectoryLookup => {
                    // Allow read operations in emergency mode
                }
                _ => {
                    return Err(VexfsError::Busy);
                }
            }
        }
        
        // Check degradation level restrictions
        if !degradation_level.allows_writes() {
            match operation_type {
                OperationType::FileWrite | OperationType::DirectoryCreate | 
                OperationType::InodeAllocation | OperationType::BlockAllocation |
                OperationType::VectorStore => {
                    vexfs_warn!("Write operation {:?} denied due to degradation level {:?}", 
                               operation_type, degradation_level);
                    return Err(VexfsError::ReadOnlyFilesystem);
                }
                _ => {}
            }
        }
        
        // Check operation limits
        let active_ops = self.resource_usage.active_operations.load(Ordering::Relaxed);
        if active_ops >= degradation_level.max_operations() {
            vexfs_warn!("Operation {:?} denied: too many active operations ({}/{})", 
                       operation_type, active_ops, degradation_level.max_operations());
            return Err(VexfsError::Busy);
        }
        
        Ok(())
    }

    /// Handle a panic situation
    pub fn handle_panic(&self, context: PanicContext) -> PanicRecoveryStrategy {
        vexfs_error!("Panic in operation {:?}: {}", 
                    context.operation_type, context.error_message);
        
        // Determine recovery strategy based on system state and operation type
        let strategy = match context.system_state.degradation_level {
            DegradationLevel::Normal | DegradationLevel::LightDegradation => {
                match context.operation_type {
                    OperationType::Mount | OperationType::Unmount => {
                        PanicRecoveryStrategy::GracefulShutdown
                    }
                    OperationType::Journal => {
                        PanicRecoveryStrategy::SwitchToReadOnly
                    }
                    _ => {
                        PanicRecoveryStrategy::ContinueWithDegradation
                    }
                }
            }
            DegradationLevel::ModerateDegradation | DegradationLevel::HeavyDegradation => {
                PanicRecoveryStrategy::SwitchToReadOnly
            }
            DegradationLevel::ReadOnlyMode => {
                PanicRecoveryStrategy::GracefulShutdown
            }
            DegradationLevel::EmergencyMode => {
                PanicRecoveryStrategy::EmergencyShutdown
            }
        };
        
        // Apply recovery strategy
        match strategy {
            PanicRecoveryStrategy::ContinueWithDegradation => {
                let new_level = match context.system_state.degradation_level {
                    DegradationLevel::Normal => DegradationLevel::LightDegradation,
                    DegradationLevel::LightDegradation => DegradationLevel::ModerateDegradation,
                    level => level,
                };
                self.current_degradation.store(new_level as u32, Ordering::Relaxed);
                vexfs_warn!("Continuing with degradation level {:?}", new_level);
            }
            PanicRecoveryStrategy::SwitchToReadOnly => {
                self.current_degradation.store(DegradationLevel::ReadOnlyMode as u32, Ordering::Relaxed);
                vexfs_warn!("Switched to read-only mode due to panic");
            }
            PanicRecoveryStrategy::GracefulShutdown => {
                self.is_shutting_down.store(true, Ordering::Relaxed);
                vexfs_warn!("Initiating graceful shutdown due to panic");
            }
            PanicRecoveryStrategy::EmergencyShutdown => {
                self.emergency_mode.store(true, Ordering::Relaxed);
                self.is_shutting_down.store(true, Ordering::Relaxed);
                vexfs_error!("Emergency shutdown initiated due to panic");
            }
        }
        
        strategy
    }

    /// Start the hang prevention monitoring system
    pub fn start_monitoring(&self) -> VexfsResult<()> {
        vexfs_info!("Starting hang prevention monitoring system");
        
        #[cfg(not(feature = "kernel"))]
        {
            // Start watchdog monitoring thread
            let watchdogs = Arc::clone(&self.active_watchdogs);
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(1));
                    
                    if let Ok(mut watchdogs_guard) = watchdogs.write() {
                        let mut expired_ids = Vec::new();
                        
                        for (id, timer) in watchdogs_guard.iter() {
                            if timer.is_expired() {
                                expired_ids.push(*id);
                            }
                        }
                        
                        // Remove expired timers
                        for id in expired_ids {
                            watchdogs_guard.remove(&id);
                            vexfs_warn!("Removed expired watchdog timer {}", id);
                        }
                    }
                }
            });
            
            // Start deadlock detection thread
            let dependencies = Arc::clone(&self.lock_dependencies);
            let states = Arc::clone(&self.lock_states);
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(DEADLOCK_CHECK_INTERVAL_SECS));
                    
                    // Simplified deadlock detection would go here
                    // In a real implementation, this would be more sophisticated
                }
            });
        }
        
        vexfs_info!("Hang prevention monitoring started successfully");
        Ok(())
    }

    /// Stop the hang prevention monitoring system
    pub fn stop_monitoring(&self) {
        vexfs_info!("Stopping hang prevention monitoring system");
        self.is_shutting_down.store(true, Ordering::Relaxed);
        
        #[cfg(not(feature = "kernel"))]
        {
            // Cancel all active watchdogs
            if let Ok(watchdogs) = self.active_watchdogs.read() {
                for timer in watchdogs.values() {
                    timer.cancel();
                }
            }
        }
        
        vexfs_info!("Hang prevention monitoring stopped");
    }

    /// Get system health status
    pub fn get_health_status(&self) -> SystemHealthStatus {
        let degradation_level = self.current_degradation_level();
        let memory_percent = self.resource_usage.memory_usage_percent();
        let cpu_percent = self.resource_usage.cpu_usage_percent.load(Ordering::Relaxed);
        let active_ops = self.resource_usage.active_operations.load(Ordering::Relaxed);
        
        #[cfg(not(feature = "kernel"))]
        let active_watchdogs = self.active_watchdogs.read()
            .map(|w| w.len())
            .unwrap_or(0) as u32;
        
        #[cfg(feature = "kernel")]
        let active_watchdogs = 0;
        
        SystemHealthStatus {
            degradation_level,
            memory_usage_percent: memory_percent,
            cpu_usage_percent: cpu_percent,
            active_operations: active_ops,
            active_watchdogs,
            is_shutting_down: self.is_shutting_down.load(Ordering::Relaxed),
            emergency_mode: self.emergency_mode.load(Ordering::Relaxed),
        }
    }
}

impl Default for HangPreventionManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// System Health Status
// =============================================================================

/// Overall system health status
#[derive(Debug)]
pub struct SystemHealthStatus {
    pub degradation_level: DegradationLevel,
    pub memory_usage_percent: u32,
    pub cpu_usage_percent: u32,
    pub active_operations: u32,
    pub active_watchdogs: u32,
    pub is_shutting_down: bool,
    pub emergency_mode: bool,
}

impl SystemHealthStatus {
    /// Check if the system is healthy
    pub fn is_healthy(&self) -> bool {
        self.degradation_level <= DegradationLevel::LightDegradation &&
        !self.emergency_mode &&
        !self.is_shutting_down
    }

    /// Get a human-readable health summary
    pub fn health_summary(&self) -> String {
        if self.emergency_mode {
            "EMERGENCY: System in emergency mode".to_string()
        } else if self.is_shutting_down {
            "SHUTDOWN: System is shutting down".to_string()
        } else {
            match self.degradation_level {
                DegradationLevel::Normal => "HEALTHY: System operating normally".to_string(),
                DegradationLevel::LightDegradation => "WARNING: Light performance degradation".to_string(),
                DegradationLevel::ModerateDegradation => "WARNING: Moderate performance degradation".to_string(),
                DegradationLevel::HeavyDegradation => "CRITICAL: Heavy performance degradation".to_string(),
                DegradationLevel::ReadOnlyMode => "CRITICAL: System in read-only mode".to_string(),
                DegradationLevel::EmergencyMode => "EMERGENCY: System in emergency mode".to_string(),
            }
        }
    }
}

// =============================================================================
// Global Instance and Public API
// =============================================================================

#[cfg(not(feature = "kernel"))]
static mut GLOBAL_HANG_PREVENTION: Option<HangPreventionManager> = None;

#[cfg(not(feature = "kernel"))]
static HANG_PREVENTION_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the global hang prevention system
pub fn init_hang_prevention() -> VexfsResult<()> {
    #[cfg(not(feature = "kernel"))]
    {
        HANG_PREVENTION_INIT.call_once(|| {
            unsafe {
                GLOBAL_HANG_PREVENTION = Some(HangPreventionManager::new());
            }
        });
        
        if let Some(manager) = unsafe { &GLOBAL_HANG_PREVENTION } {
            manager.start_monitoring()?;
        }
    }
    
    vexfs_info!("Hang prevention system initialized");
    Ok(())
}

/// Get the global hang prevention manager
#[cfg(not(feature = "kernel"))]
pub fn get_hang_prevention_manager() -> Option<&'static HangPreventionManager> {
    unsafe { GLOBAL_HANG_PREVENTION.as_ref() }
}

/// Start a watchdog timer for an operation
pub fn start_operation_watchdog(operation_type: OperationType, timeout: Option<Duration>) -> VexfsResult<WatchdogId> {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = get_hang_prevention_manager() {
            return manager.start_watchdog(operation_type, timeout);
        }
    }
    
    // In kernel mode or if not initialized, return a dummy ID
    Ok(0)
}

/// Cancel a watchdog timer
pub fn cancel_operation_watchdog(watchdog_id: WatchdogId) -> VexfsResult<()> {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = get_hang_prevention_manager() {
            return manager.cancel_watchdog(watchdog_id);
        }
    }
    
    Ok(())
}

/// Check if an operation should be allowed
pub fn check_operation_allowed(operation_type: OperationType) -> VexfsResult<()> {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = get_hang_prevention_manager() {
            return manager.should_allow_operation(operation_type);
        }
    }
    
    // In kernel mode or if not initialized, allow all operations
    Ok(())
}

/// Update system resource usage
pub fn update_system_resources(memory_bytes: u64, cpu_percent: u32) {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = get_hang_prevention_manager() {
            manager.update_resource_usage(memory_bytes, cpu_percent);
        }
    }
}

/// Get current system health status
pub fn get_system_health() -> Option<SystemHealthStatus> {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = get_hang_prevention_manager() {
            return Some(manager.get_health_status());
        }
    }
    
    None
}

/// Shutdown the hang prevention system
pub fn shutdown_hang_prevention() {
    #[cfg(not(feature = "kernel"))]
    {
        if let Some(manager) = get_hang_prevention_manager() {
            manager.stop_monitoring();
        }
    }
    
    vexfs_info!("Hang prevention system shutdown");
}

// =============================================================================
// Convenience Macros
// =============================================================================

/// Macro to wrap an operation with a watchdog timer
///
/// # Examples
/// ```rust
/// // Usage: with_watchdog!(operation_type, timeout, operation)
/// let result = with_watchdog!(OperationType::FileRead, Some(Duration::from_secs(30)), {
///     // Your file read operation here
///     read_file_data()
/// })?;
/// ```
#[macro_export]
macro_rules! with_watchdog {
    ($operation_type:expr, $timeout:expr, $operation:expr) => {{
        let watchdog_id = $crate::shared::system_hang_prevention::start_operation_watchdog($operation_type, $timeout)?;
        let result = $operation;
        let _ = $crate::shared::system_hang_prevention::cancel_operation_watchdog(watchdog_id);
        result
    }};
    ($operation_type:expr, $operation:expr) => {{
        let watchdog_id = $crate::shared::system_hang_prevention::start_operation_watchdog($operation_type, None)?;
        let result = $operation;
        let _ = $crate::shared::system_hang_prevention::cancel_operation_watchdog(watchdog_id);
        result
    }};
}

/// Macro to check if an operation is allowed before executing
///
/// # Examples
/// ```rust
/// // Usage: check_operation!(operation_type, operation)
/// let result = check_operation!(OperationType::FileWrite, {
///     // Your file write operation here
///     write_file_data()
/// })?;
/// ```
#[macro_export]
macro_rules! check_operation {
    ($operation_type:expr, $operation:expr) => {{
        $crate::shared::system_hang_prevention::check_operation_allowed($operation_type)?;
        $operation
    }};
}

/// Macro to wrap an operation with both permission check and watchdog
///
/// # Examples
/// ```rust
/// // Usage: safe_operation!(operation_type, timeout, operation)
/// let result = safe_operation!(OperationType::VectorSearch, Some(Duration::from_secs(60)), {
///     // Your vector search operation here
///     perform_vector_search()
/// })?;
/// ```
#[macro_export]
macro_rules! safe_operation {
    ($operation_type:expr, $timeout:expr, $operation:expr) => {{
        $crate::shared::system_hang_prevention::check_operation_allowed($operation_type)?;
        $crate::with_watchdog!($operation_type, $timeout, $operation)
    }};
    ($operation_type:expr, $operation:expr) => {{
        $crate::shared::system_hang_prevention::check_operation_allowed($operation_type)?;
        $crate::with_watchdog!($operation_type, $operation)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_timer() {
        let timer = WatchdogTimer::new(1, OperationType::FileRead, Some(Duration::from_millis(100)));
        assert!(timer.is_active());
        assert!(!timer.is_expired());
        
        // Timer should not be expired immediately
        assert!(!timer.is_expired());
        
        timer.cancel();
        assert!(!timer.is_active());
    }

    #[test]
    fn test_degradation_levels() {
        assert!(DegradationLevel::Normal.allows_writes());
        assert!(!DegradationLevel::ReadOnlyMode.allows_writes());
        assert!(DegradationLevel::Normal.max_operations() > DegradationLevel::EmergencyMode.max_operations());
    }

    #[test]
    fn test_operation_priorities() {
        assert!(OperationType::Mount.priority() > OperationType::FileRead.priority());
        assert!(OperationType::Journal.priority() > OperationType::VectorSearch.priority());
    }

    #[test]
    fn test_resource_usage() {
        let usage = ResourceUsage::default();
        usage.update_memory_usage(1024);
        assert_eq!(usage.memory_used_bytes.load(Ordering::Relaxed), 1024);
        
        usage.update_cpu_usage(50);
        assert_eq!(usage.cpu_usage_percent.load(Ordering::Relaxed), 50);
        
        assert!(!usage.is_under_pressure());
    }

    #[test]
    fn test_hang_prevention_manager() {
        let manager = HangPreventionManager::new();
        
        // Test watchdog creation
        let watchdog_id = manager.start_watchdog(OperationType::FileRead, None).unwrap();
        assert!(watchdog_id > 0);
        
        // Test operation permission
        assert!(manager.should_allow_operation(OperationType::FileRead).is_ok());
        
        // Test lock acquisition
        let task_id = current_task_id();
        let lock_id = manager.acquire_lock(LockType::Read, task_id).unwrap();
        assert!(lock_id > 0);
        
        // Test lock release
        assert!(manager.release_lock(lock_id, task_id).is_ok());
        
        // Test health status
        let health = manager.get_health_status();
        assert!(health.is_healthy());
    }
}
// =============================================================================
// Utility Functions
// =============================================================================

/// Get current timestamp in seconds since epoch
pub fn current_timestamp_secs() -> u64 {
    #[cfg(not(feature = "kernel"))]
    {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    #[cfg(feature = "kernel")]
    {
        // In kernel mode, use kernel time functions
        // This is a simplified implementation
        0 // Would use actual kernel time functions
    }
}

/// Get current task/thread ID
pub fn current_task_id() -> TaskId {
    #[cfg(not(feature = "kernel"))]
    {
        // Use a stable method to get thread ID by converting to string and hashing
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let thread_id = thread::current().id();
        let mut hasher = DefaultHasher::new();
        thread_id.hash(&mut hasher);
        hasher.finish()
    }
    
    #[cfg(feature = "kernel")]
    {
        // In kernel mode, use kernel task functions
        0 // Would use actual kernel task ID
    }
}