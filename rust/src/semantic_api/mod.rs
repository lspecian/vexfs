//! Semantic API Module
//!
//! This module provides the complete semantic event system for VexFS,
//! including distributed coordination, event synchronization, cross-boundary
//! propagation capabilities, and advanced analytics and monitoring (Phase 6).

// Core types and utilities
pub mod types;

// Event emission and hooks
pub mod event_emission;
pub mod kernel_hooks;
pub mod userspace_hooks;

// Event propagation and routing
pub mod event_propagation;
pub mod event_propagation_manager;
pub mod event_routing;
pub mod event_filtering;
pub mod kernel_fuse_bridge;
pub mod advanced_event_router;
pub mod event_propagation_integration;

// Distributed coordination (Phase 4)
pub mod distributed_coordination;
pub mod distributed_coordination_impl;
pub mod event_synchronization;

// Event ordering and conflict resolution
pub mod event_ordering_service;
pub mod boundary_sync_manager;
pub mod cross_boundary_coordinator;

// Journal integration
pub mod userspace_journal;
pub mod journal_compatibility;
pub mod semantic_persistence;
pub mod journal_recovery_manager;
pub mod event_replay_engine;
pub mod recovery_coordination_service;

// FUSE integration
pub mod fuse_journal_integration;
pub mod fuse_journal_manager;
pub mod fuse_event_mapper;

// Graph integration
pub mod graph_journal_integration;
pub mod fuse_graph_integration;
pub mod fuse_graph_integration_manager;
pub mod fuse_graph_config;
pub mod graph_performance_metrics;
pub mod advanced_graph_analytics;
pub mod advanced_graph_analytics_impl;
pub mod clustering_analyzer_impl;
pub mod graph_health_monitor_impl;

// Semantic reasoning
pub mod semantic_reasoning_engine;
pub mod semantic_reasoning_types;
pub mod semantic_reasoning_integration;

// API and client interfaces
pub mod api_server;
pub mod api_server_stub;
pub mod client;
pub mod websocket_stream;
pub mod query_processor;
pub mod query_processor_fixed;

// Authentication and authorization
pub mod auth;
pub mod rate_limit;

// Agent framework
pub mod orchestration;
pub mod memory;
pub mod tools;
pub mod agent_framework;

// Analytics and monitoring
pub mod event_analytics_engine;
pub mod complex_event_processor;
pub mod automation_rule_engine;
pub mod monitoring_dashboard;
pub mod performance_profiler;
pub mod production_manager;

// Phase 5: Reactive Automation Framework
pub mod automation_framework;
pub mod automation_framework_impl;

// Distributed systems
pub mod cluster_coordinator;
pub mod distributed_event_synchronizer;
pub mod distributed_state_manager;

// Testing and integration
pub mod integration_test;
pub mod api_integration_test;
pub mod event_propagation_test;
pub mod fuse_journal_integration_test;
pub mod standalone_test;

// Re-export commonly used types and functions
pub use types::*;

// Event emission exports
pub use event_emission::{
    EventEmissionFramework, EventEmissionConfig, EventEmissionStats,
    initialize_event_emission, shutdown_event_emission,
    emit_filesystem_event, emit_graph_event, emit_vector_event
};

// Hook exports
pub use userspace_hooks::{
    initialize_userspace_hooks, shutdown_userspace_hooks,
    register_filesystem_hook, register_graph_hook, register_vector_hook
};

// API server exports
pub use api_server::{
    ApiServer, ApiServerConfig, EventStorage, InMemoryEventStorage,
    EventListParams, EventSearchParams, StreamParams, JournalStats
};

// Authentication exports
pub use auth::{
    AuthManager, AuthMiddleware, AgentRegistration, AgentToken, AgentClaims,
    AgentRegistrationRequest, AgentAuthRequest, AuthResponse
};

// Query processing exports
pub use query_processor_fixed::{
    EventIndexSystem, IndexConfig, TimestampIndex, HashIndex, IndexStats
};

// Rate limiting exports
pub use rate_limit::{
    RateLimiter, RateLimitConfig, RateLimitResult, RateLimitError
};

// Orchestration exports
pub use orchestration::{
    AgentOrchestrator, OrchestrationConfig, AgentInfo, AgentTask,
    TaskType, TaskPriority, TaskResult, TaskStatus, OrchestrationStats
};

// Memory exports
pub use memory::{
    AgentMemoryManager, MemoryConfig, EpisodicMemory, SemanticConcept,
    WorkingMemorySet, MemoryQuery, MemoryQueryResult
};

// Tools exports
pub use tools::{
    ToolManager, ToolConfig, Tool, ToolParameter, ToolResult,
    ToolExecutionContext, ToolRegistry
};

// Agent framework exports
pub use agent_framework::{
    AgentInteractionFramework, FrameworkConfig, FrameworkStats,
    AgentSession, FrameworkRequest, FrameworkResponse, FrameworkOperation
};

// Journal exports
pub use userspace_journal::{
    UserspaceSemanticJournal, UserspaceJournalConfig, JournalEntry,
    JournalStats as UserspaceJournalStats, JournalRecoveryManager
};

// Compatibility exports
pub use journal_compatibility::{
    KernelCompatibilityBridge, CompatibilityBridgeConfig, CompatibilityMode,
    KernelSemanticHeader, KernelEventHeader, ConversionStats
};

// Persistence exports
pub use semantic_persistence::{
    SemanticPersistenceManager, PersistenceConfig, PersistenceStats,
    StorageBackend, CompressionAlgorithm as PersistenceCompression
};

// Cross-boundary coordination exports
pub use cross_boundary_coordinator::{
    CrossBoundaryTransactionCoordinator, CrossBoundaryConfig, CrossBoundaryStats,
    CrossBoundaryTransaction, CrossBoundaryParticipant, DeadlockGraph
};

// Event ordering exports
pub use event_ordering_service::{
    EventOrderingService, EventOrderingConfig, OrderedSemanticEvent,
    VectorClock as EventVectorClock, EventOrderingStats
};

// Boundary synchronization exports
pub use boundary_sync_manager::{
    BoundarySynchronizationManager, BoundarySyncConfig, BoundarySyncStats,
    SynchronizationStream, StreamConfig, StreamStats
};

// Recovery exports
pub use journal_recovery_manager::{
    JournalRecoveryManager as RecoveryManager, RecoveryConfig, RecoveryStats,
    RecoveryFailureType, RecoveryStrategy, RecoveryState
};

// Event replay exports
pub use event_replay_engine::{
    EventReplayEngine, ReplayConfig, ReplayStats, ReplayOperation,
    ReplayValidationMode, ReplayState, EventFilter as ReplayEventFilter
};

// Recovery coordination exports
pub use recovery_coordination_service::{
    RecoveryCoordinationService, RecoveryCoordinationConfig,
    RecoveryCoordinationStats, RecoveryPlan, RecoveryStep
};

// FUSE journal exports
pub use fuse_journal_integration::{
    FuseJournalIntegration, FuseJournalConfig, FuseJournalStats,
    FilesystemOperation, VectorOperation, GraphOperation
};

// FUSE journal manager exports
pub use fuse_journal_manager::{
    FuseJournalManager, FuseJournalManagerConfig, FuseJournalManagerMetrics,
    FuseMountInfo, FusePerformanceMode, FuseJournalOperation
};

// Graph journal integration exports
pub use graph_journal_integration::{
    GraphJournalIntegrationManager, GraphJournalConfig, AnalyticsOptions,
    GraphPerformanceMetrics, GraphAnalyticsEngine, EventCorrelationTracker
};

// FUSE graph integration exports
pub use fuse_graph_integration_manager::{
    FuseGraphIntegrationManager, FuseGraphIntegrationMetrics,
    FuseOperationDetector, FuseAnalyticsCoordinator, FusePerformanceOptimizer
};

// Graph configuration exports
pub use fuse_graph_config::{
    FuseGraphConfig, GraphOperationSettings, SearchParameters,
    InsertionParameters, PerformanceSettings, AnalyticsSettings
};

// Graph performance exports
pub use graph_performance_metrics::{
    GraphPerformanceMetrics as GraphMetrics, MetricsConfig,
    OperationMetrics, LatencyMetrics, ThroughputMetrics, MemoryMetrics
};

// Advanced graph analytics exports
pub use advanced_graph_analytics::{
    AdvancedGraphAnalytics, AdvancedAnalyticsConfig, CentralityCalculator,
    PathfindingEngine, ClusteringAnalyzer, GraphHealthMonitor
};

// Semantic reasoning exports
pub use semantic_reasoning_engine::{
    SemanticReasoningEngine, ReasoningConfig, ReasoningStats,
    ReasoningQuery, ReasoningResult, InferenceEngine
};

// Semantic reasoning types exports
pub use semantic_reasoning_types::{
    SemanticKnowledgeGraph, SemanticConcept as ReasoningConcept,
    SemanticRelationship, ReasoningRule, InferenceResult
};

// Semantic reasoning integration exports
pub use semantic_reasoning_integration::{
    SemanticReasoningIntegration, ReasoningIntegrationConfig,
    ReasoningIntegrationStats, ReasoningEvent, ReasoningContext
};

// Event propagation manager exports
pub use event_propagation_manager::{
    EventPropagationManager, PropagationConfig, PropagationStats,
    BoundaryType, PropagationPolicy, CrossBoundaryEvent, PropagationResult
};

// Kernel-FUSE bridge exports
pub use kernel_fuse_bridge::{
    KernelFuseBridge, KernelFuseBridgeConfig, KernelFuseBridgeStats,
    TranslationContext, ConflictResolutionStrategy, TranslationMode
};

// Advanced event router exports
pub use advanced_event_router::{
    AdvancedEventRouter, AdvancedRouterConfig, RoutingRule,
    EventPattern, ContextPattern, RoutingAction, TopologyNode
};

// Event propagation integration exports
pub use event_propagation_integration::{
    IntegratedEventPropagationSystem, IntegratedPropagationConfig,
    IntegratedPerformanceMetrics, SystemStatus, ComponentsStatus
};

// Distributed coordination exports (Phase 4)
pub use distributed_coordination::{
    DistributedEventCoordinator, DistributedCoordinatorConfig, RaftConfig,
    NetworkConfig, PerformanceConfig, SecurityConfig, DistributedSemanticEvent,
    CoordinationMetadata, ConflictResolutionData, ConsistencyLevel,
    NetworkOptimizationHints, CompressionAlgorithm, RaftState, RaftLogEntry
};

// Event synchronization exports (Phase 4)
pub use event_synchronization::{
    EventSynchronizationManager, SynchronizationProtocol, CRDTManager,
    GCounter, PNCounter, LWWRegister, ORSet, TwoPhaseSet, MVRegister,
    SynchronizationMetrics, ConflictDetector, ConflictResolver
};

// Analytics exports
pub use event_analytics_engine::{
    EventAnalyticsEngine, AnalyticsConfig, StreamStatistics,
    EventPattern as AnalyticsEventPattern, Anomaly, Prediction
};

// Complex event processing exports
pub use complex_event_processor::{
    ComplexEventProcessor, ComplexEventProcessorConfig, EventPattern as CEPEventPattern,
    PatternExpression, PatternCondition, PatternMatch, PatternMatcher
};

// Automation exports
pub use automation_rule_engine::{
    AutomationRuleEngine, AutomationRuleEngineConfig, AutomationRule,
    RuleType, TriggerCondition, AutomationAction, RuleExecutionResult
};

// Phase 5: Reactive Automation Framework exports
pub use automation_framework::{
    ReactiveAutomationFramework, ReactiveAutomationConfig, ReactiveWorkflow,
    WorkflowType, WorkflowTrigger, WorkflowStep, WorkflowAction, WorkflowExecutionResult,
    ReactiveAutomationMetrics, ReactiveAutomationResult, ReactiveSystemState,
    SystemStateType, StateTransition, WorkflowStatus, ExecutionStatus
};

pub use automation_framework_impl::{
    AutomationDashboard, WorkflowSummary
};

// Monitoring exports
pub use monitoring_dashboard::{
    MonitoringDashboard, DashboardConfig, DashboardAlert,
    ChartData, DashboardWidget, RealTimeMetrics
};

// Performance profiling exports
pub use performance_profiler::{
    PerformanceProfiler, ProfilerConfig, PerformanceProfile,
    SystemMetrics, ComponentMetrics, PerformanceBottleneck
};

// Production management exports
pub use production_manager::{
    ProductionManager, ProductionConfig, ProductionStatus,
    HealthStatus, SecurityStatus, ComplianceStatus, BackupStatus
};

// Cluster coordination exports
pub use cluster_coordinator::{
    ClusterCoordinator, ClusterConfig, ClusterNode, NodeCapabilities,
    LeaderElectionState, ClusterMembership, CoordinationMetrics
};

// Distributed synchronization exports
pub use distributed_event_synchronizer::{
    DistributedEventSynchronizer, DistributedSyncConfig, RaftConfig as SyncRaftConfig,
    DistributedSemanticEvent as SyncDistributedEvent, VectorClock as SyncVectorClock
};

// Distributed state management exports
pub use distributed_state_manager::{
    DistributedStateManager, DistributedStateConfig, StateEntry,
    StateValue, ConfigValue, RuntimeValue, ConflictResolutionStrategy
};

// Error types
use std::fmt;
use serde::{Deserialize, Serialize};

/// Semantic API error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticError {
    /// Invalid operation
    InvalidOperation(String),
    /// Configuration error
    ConfigurationError(String),
    /// Network error
    NetworkError(String),
    /// Serialization error
    SerializationError(String),
    /// Authentication error
    AuthenticationError(String),
    /// Authorization error
    AuthorizationError(String),
    /// Rate limit exceeded
    RateLimitExceeded(String),
    /// Resource not found
    ResourceNotFound(String),
    /// Resource conflict
    ResourceConflict(String),
    /// Timeout error
    TimeoutError(String),
    /// Internal error
    InternalError(String),
    /// Database error
    DatabaseError(String),
    /// IO error
    IoError(String),
    /// Parse error
    ParseError(String),
    /// Validation error
    ValidationError(String),
    /// Consensus error
    ConsensusError(String),
    /// Synchronization error
    SynchronizationError(String),
    /// Conflict resolution error
    ConflictResolutionError(String),
    /// Causality violation
    CausalityViolation(String),
    /// Byzantine fault detected
    ByzantineFault(String),
    /// Network partition
    NetworkPartition(String),
    /// Recovery failure
    RecoveryFailure(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            SemanticError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            SemanticError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            SemanticError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            SemanticError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            SemanticError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            SemanticError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            SemanticError::ResourceNotFound(msg) => write!(f, "Resource not found: {}", msg),
            SemanticError::ResourceConflict(msg) => write!(f, "Resource conflict: {}", msg),
            SemanticError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            SemanticError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            SemanticError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            SemanticError::IoError(msg) => write!(f, "IO error: {}", msg),
            SemanticError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SemanticError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            SemanticError::ConsensusError(msg) => write!(f, "Consensus error: {}", msg),
            SemanticError::SynchronizationError(msg) => write!(f, "Synchronization error: {}", msg),
            SemanticError::ConflictResolutionError(msg) => write!(f, "Conflict resolution error: {}", msg),
            SemanticError::CausalityViolation(msg) => write!(f, "Causality violation: {}", msg),
            SemanticError::ByzantineFault(msg) => write!(f, "Byzantine fault: {}", msg),
            SemanticError::NetworkPartition(msg) => write!(f, "Network partition: {}", msg),
            SemanticError::RecoveryFailure(msg) => write!(f, "Recovery failure: {}", msg),
        }
    }
}

impl std::error::Error for SemanticError {}

impl From<std::io::Error> for SemanticError {
    fn from(err: std::io::Error) -> Self {
        SemanticError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for SemanticError {
    fn from(err: serde_json::Error) -> Self {
        SemanticError::SerializationError(err.to_string())
    }
}

impl From<crossbeam::channel::SendError<crate::semantic_api::distributed_coordination::CoordinationCommand>> for SemanticError {
    fn from(err: crossbeam::channel::SendError<crate::semantic_api::distributed_coordination::CoordinationCommand>) -> Self {
        SemanticError::NetworkError(err.to_string())
    }
}

impl From<crossbeam::channel::SendError<crate::semantic_api::distributed_coordination::RaftMessage>> for SemanticError {
    fn from(err: crossbeam::channel::SendError<crate::semantic_api::distributed_coordination::RaftMessage>) -> Self {
        SemanticError::NetworkError(err.to_string())
    }
}

/// Result type for semantic API operations
pub type SemanticResult<T> = Result<T, SemanticError>;

/// Semantic API configuration
#[derive(Debug, Clone)]
pub struct SemanticApiConfig {
    /// Enable event emission
    pub event_emission_enabled: bool,
    /// Enable distributed coordination
    pub distributed_coordination_enabled: bool,
    /// Enable event synchronization
    pub event_synchronization_enabled: bool,
    /// Enable analytics
    pub analytics_enabled: bool,
    /// Enable monitoring
    pub monitoring_enabled: bool,
    /// API server configuration
    pub api_server_config: Option<ApiServerConfig>,
    /// Authentication configuration
    pub auth_config: Option<auth::AuthConfig>,
    /// Rate limiting configuration
    pub rate_limit_config: Option<rate_limit::RateLimitConfig>,
}

impl Default for SemanticApiConfig {
    fn default() -> Self {
        Self {
            event_emission_enabled: true,
            distributed_coordination_enabled: false,
            event_synchronization_enabled: false,
            analytics_enabled: false,
            monitoring_enabled: false,
            api_server_config: None,
            auth_config: None,
            rate_limit_config: None,
        }
    }
}

/// Initialize the semantic API
pub fn initialize_semantic_api(_config: SemanticApiConfig) -> SemanticResult<()> {
    // Initialize core components
    Ok(())
}

/// Shutdown the semantic API
pub fn shutdown_semantic_api() -> SemanticResult<()> {
    // Shutdown all components
    Ok(())
}