//! Enhanced Operation Context Module
//!
//! Provides rich context for filesystem operations with telemetry, lifecycle hooks,
//! and resource tracking capabilities.

use std::time::Duration;
use std::sync::Arc;

/// Enhanced context for filesystem operations
#[derive(Debug, Clone)]
pub struct EnhancedOperationContext {
    pub operation_type: OperationType,
    pub metadata: OperationMetadata,
    pub cancellation_token: Option<CancellationToken>,
    pub timeout_config: Option<TimeoutConfig>,
    pub telemetry: Option<TelemetryCollector>,
    pub progress: Option<ProgressReporter>,
    pub resources: Option<ResourceTracker>,
    pub hooks: Option<LifecycleHooks>,
    pub priority: OperationPriority,
    pub limits: ResourceLimits,
}

impl EnhancedOperationContext {
    pub fn new(operation_type: OperationType) -> Self {
        Self {
            operation_type,
            metadata: OperationMetadata::default(),
            cancellation_token: None,
            timeout_config: None,
            telemetry: None,
            progress: None,
            resources: None,
            hooks: None,
            priority: OperationPriority::Normal,
            limits: ResourceLimits::default(),
        }
    }
}

impl Default for EnhancedOperationContext {
    fn default() -> Self {
        Self::new(OperationType::Read)
    }
}

/// Operation type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Read,
    Write,
    Delete,
    Create,
    Update,
    Search,
    VectorSearch,
    BatchVectorSearch,
    IndexBuild,
    FileSystemOperation,
}

/// Memory allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAllocationType {
    Stack,
    Heap,
    Shared,
}

/// Operation metadata
#[derive(Debug, Clone)]
pub struct OperationMetadata {
    pub started_at: u64,
    pub operation_id: u64,
    pub user_id: Option<u32>,
    pub group_id: Option<u32>,
    pub operation_type: String,
    pub start_time_us: u64,
    pub description: String,
    pub parent_operation_id: Option<u64>,
    pub session_id: String,
    pub request_id: String,
    pub tags: Vec<String>,
}

impl Default for OperationMetadata {
    fn default() -> Self {
        Self {
            started_at: 0,
            operation_id: 0,
            user_id: None,
            group_id: None,
            operation_type: String::new(),
            start_time_us: 0,
            description: String::new(),
            parent_operation_id: None,
            session_id: String::new(),
            request_id: String::new(),
            tags: Vec::new(),
        }
    }
}

/// Cancellation token for async operations
#[derive(Debug, Clone)]
pub struct CancellationToken {
    pub is_cancelled: bool,
    pub reason: Option<CancellationReason>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            is_cancelled: false,
            reason: None,
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Reason for operation cancellation
#[derive(Debug, Clone)]
pub enum CancellationReason {
    UserRequested,
    Timeout,
    ResourceExhausted,
    Error(String),
}

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub duration: Duration,
    pub action: TimeoutAction,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(30),
            action: TimeoutAction::Cancel,
        }
    }
}

/// Action to take on timeout
#[derive(Debug, Clone)]
pub enum TimeoutAction {
    Cancel,
    ReturnPartial,
    ContinueBackground,
}

/// Telemetry collector
#[derive(Debug, Clone)]
pub struct TelemetryCollector {
    pub enabled: bool,
    pub events: Vec<TelemetryEvent>,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self {
            enabled: false,
            events: Vec::new(),
        }
    }

    pub fn record(&mut self, event: TelemetryEvent) {
        if self.enabled {
            self.events.push(event);
        }
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Telemetry event
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub event_type: TelemetryEventType,
    pub timestamp: u64,
    pub severity: TelemetrySeverity,
    pub message: String,
}

/// Telemetry event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryEventType {
    Start,
    Progress,
    Complete,
    Error,
    Warning,
}

/// Telemetry severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetrySeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Progress reporter
#[derive(Debug, Clone)]
pub struct ProgressReporter {
    pub total: u64,
    pub current: u64,
    pub message: String,
}

impl ProgressReporter {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            current: 0,
            message: String::new(),
        }
    }

    pub fn update(&mut self, current: u64, message: impl Into<String>) {
        self.current = current;
        self.message = message.into();
    }

    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }
}

/// Resource tracker
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    pub memory_used: u64,
    pub cpu_time_ms: u64,
    pub io_operations: u64,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            memory_used: 0,
            cpu_time_ms: 0,
            io_operations: 0,
        }
    }

    pub fn summary(&self) -> ResourceUsageSummary {
        ResourceUsageSummary {
            memory_used: self.memory_used,
            cpu_time_ms: self.cpu_time_ms,
            io_operations: self.io_operations,
        }
    }
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource usage summary
#[derive(Debug, Clone)]
pub struct ResourceUsageSummary {
    pub memory_used: u64,
    pub cpu_time_ms: u64,
    pub io_operations: u64,
}

/// Lifecycle hooks
#[derive(Debug, Clone)]
pub struct LifecycleHooks {
    pub on_start: Option<String>,
    pub on_complete: Option<String>,
    pub on_error: Option<String>,
}

impl LifecycleHooks {
    pub fn new() -> Self {
        Self {
            on_start: None,
            on_complete: None,
            on_error: None,
        }
    }
}

impl Default for LifecycleHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// Operation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationPriority {
    Low = 1,
    Normal = 5,
    High = 9,
    Critical = 10,
}

/// Resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_time_ms: Option<u64>,
    pub max_io_operations: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: None,
            max_cpu_time_ms: None,
            max_io_operations: None,
        }
    }
}
