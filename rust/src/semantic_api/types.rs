//! Semantic API Types
//! 
//! This module defines the core types used in the Agent-Facing Semantic Event API,
//! providing Rust representations of the kernel semantic event structures and
//! additional types needed for the API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Semantic Event Type - Maps to kernel event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum SemanticEventType {
    // Filesystem events (0x0100)
    FilesystemCreate = 0x0101,
    FilesystemDelete = 0x0102,
    FilesystemRead = 0x0103,
    FilesystemWrite = 0x0104,
    FilesystemRename = 0x0105,
    FilesystemChmod = 0x0106,
    FilesystemChown = 0x0107,
    FilesystemTruncate = 0x0108,
    FilesystemMkdir = 0x0109,
    FilesystemRmdir = 0x010A,
    FilesystemSymlink = 0x010B,
    FilesystemHardlink = 0x010C,
    
    // Graph events (0x0200)
    GraphNodeCreate = 0x0201,
    GraphNodeDelete = 0x0202,
    GraphNodeUpdate = 0x0203,
    GraphEdgeCreate = 0x0204,
    GraphEdgeDelete = 0x0205,
    GraphEdgeUpdate = 0x0206,
    GraphPropertySet = 0x0207,
    GraphPropertyDelete = 0x0208,
    GraphTraverse = 0x0209,
    GraphQuery = 0x020A,
    
    // Vector events (0x0300)
    VectorCreate = 0x0301,
    VectorDelete = 0x0302,
    VectorUpdate = 0x0303,
    VectorSearch = 0x0304,
    VectorIndex = 0x0305,
    VectorSimilarity = 0x0306,
    VectorCluster = 0x0307,
    VectorEmbed = 0x0308,
    
    // Agent events (0x0400)
    AgentQuery = 0x0401,
    AgentReasoning = 0x0402,
    AgentDecision = 0x0403,
    AgentOrchestration = 0x0404,
    AgentLearning = 0x0405,
    AgentInteraction = 0x0406,
    
    // System events (0x0500)
    SystemMount = 0x0501,
    SystemUnmount = 0x0502,
    SystemSync = 0x0503,
    SystemCheckpoint = 0x0504,
    SystemRecovery = 0x0505,
    SystemOptimization = 0x0506,
    
    // Semantic events (0x0600)
    SemanticTransactionBegin = 0x0601,
    SemanticTransactionEnd = 0x0602,
    SemanticCausalityLink = 0x0603,
    SemanticIntentCapture = 0x0604,
    SemanticContextSwitch = 0x0605,
    SemanticLink = 0x0606,
    
    // Observability events (0x0800) - System monitoring and telemetry
    ObservabilityMetricCollected = 0x0801,
    ObservabilityLogGenerated = 0x0802,
    ObservabilityTraceSpanStart = 0x0803,
    ObservabilityTraceSpanEnd = 0x0804,
    ObservabilityAlertTriggered = 0x0805,
    ObservabilityHealthCheck = 0x0806,
    ObservabilityPerformanceCounter = 0x0807,
    ObservabilityErrorReported = 0x0808,
    ObservabilityAuditEvent = 0x0809,
    ObservabilitySystemStatus = 0x080A,
    ObservabilityResourceUsage = 0x080B,
    ObservabilityThreshold = 0x080C,
}

impl SemanticEventType {
    /// Get the category of this event type
    pub fn category(&self) -> EventCategory {
        match (*self as u32) & 0xFF00 {
            0x0100 => EventCategory::Filesystem,
            0x0200 => EventCategory::Graph,
            0x0300 => EventCategory::Vector,
            0x0400 => EventCategory::Agent,
            0x0500 => EventCategory::System,
            0x0600 => EventCategory::Semantic,
            0x0800 => EventCategory::Observability,
            _ => EventCategory::Unknown,
        }
    }
    
    /// Get a human-readable description of this event type
    pub fn description(&self) -> &'static str {
        match self {
            SemanticEventType::FilesystemCreate => "File or directory creation",
            SemanticEventType::FilesystemDelete => "File or directory deletion",
            SemanticEventType::FilesystemRead => "File read operation",
            SemanticEventType::FilesystemWrite => "File write operation",
            SemanticEventType::FilesystemRename => "File or directory rename",
            SemanticEventType::FilesystemChmod => "File permission change",
            SemanticEventType::FilesystemChown => "File ownership change",
            SemanticEventType::FilesystemTruncate => "File truncation",
            SemanticEventType::FilesystemMkdir => "Directory creation",
            SemanticEventType::FilesystemRmdir => "Directory removal",
            SemanticEventType::FilesystemSymlink => "Symbolic link creation",
            SemanticEventType::FilesystemHardlink => "Hard link creation",
            
            SemanticEventType::GraphNodeCreate => "Graph node creation",
            SemanticEventType::GraphNodeDelete => "Graph node deletion",
            SemanticEventType::GraphNodeUpdate => "Graph node update",
            SemanticEventType::GraphEdgeCreate => "Graph edge creation",
            SemanticEventType::GraphEdgeDelete => "Graph edge deletion",
            SemanticEventType::GraphEdgeUpdate => "Graph edge update",
            SemanticEventType::GraphPropertySet => "Graph property set",
            SemanticEventType::GraphPropertyDelete => "Graph property deletion",
            SemanticEventType::GraphTraverse => "Graph traversal operation",
            SemanticEventType::GraphQuery => "Graph query operation",
            
            SemanticEventType::VectorCreate => "Vector creation",
            SemanticEventType::VectorDelete => "Vector deletion",
            SemanticEventType::VectorUpdate => "Vector update",
            SemanticEventType::VectorSearch => "Vector similarity search",
            SemanticEventType::VectorIndex => "Vector indexing operation",
            SemanticEventType::VectorSimilarity => "Vector similarity computation",
            SemanticEventType::VectorCluster => "Vector clustering operation",
            SemanticEventType::VectorEmbed => "Vector embedding operation",
            
            SemanticEventType::AgentQuery => "AI agent query",
            SemanticEventType::AgentReasoning => "AI agent reasoning process",
            SemanticEventType::AgentDecision => "AI agent decision making",
            SemanticEventType::AgentOrchestration => "AI agent orchestration",
            SemanticEventType::AgentLearning => "AI agent learning process",
            SemanticEventType::AgentInteraction => "AI agent interaction",
            
            SemanticEventType::SystemMount => "Filesystem mount",
            SemanticEventType::SystemUnmount => "Filesystem unmount",
            SemanticEventType::SystemSync => "System synchronization",
            SemanticEventType::SystemCheckpoint => "System checkpoint",
            SemanticEventType::SystemRecovery => "System recovery",
            SemanticEventType::SystemOptimization => "System optimization",
            
            SemanticEventType::SemanticTransactionBegin => "Semantic transaction begin",
            SemanticEventType::SemanticTransactionEnd => "Semantic transaction end",
            SemanticEventType::SemanticCausalityLink => "Causality link creation",
            SemanticEventType::SemanticIntentCapture => "Intent capture",
            SemanticEventType::SemanticContextSwitch => "Context switch",
            SemanticEventType::SemanticLink => "Semantic link creation",
            
            SemanticEventType::ObservabilityMetricCollected => "System metric collection",
            SemanticEventType::ObservabilityLogGenerated => "Log entry generation",
            SemanticEventType::ObservabilityTraceSpanStart => "Trace span initiation",
            SemanticEventType::ObservabilityTraceSpanEnd => "Trace span completion",
            SemanticEventType::ObservabilityAlertTriggered => "System alert triggered",
            SemanticEventType::ObservabilityHealthCheck => "Health check execution",
            SemanticEventType::ObservabilityPerformanceCounter => "Performance counter update",
            SemanticEventType::ObservabilityErrorReported => "Error condition reported",
            SemanticEventType::ObservabilityAuditEvent => "Audit event logged",
            SemanticEventType::ObservabilitySystemStatus => "System status update",
            SemanticEventType::ObservabilityResourceUsage => "Resource usage measurement",
            SemanticEventType::ObservabilityThreshold => "Threshold monitoring event",
        }
    }
}

/// Event Category for grouping event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    Filesystem,
    Graph,
    Vector,
    Agent,
    System,
    Semantic,
    Observability,
    Unknown,
}

/// Event Flags - Maps to kernel event flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventFlags {
    pub atomic: bool,
    pub transactional: bool,
    pub causal: bool,
    pub agent_visible: bool,
    pub deterministic: bool,
    pub compressed: bool,
    pub indexed: bool,
    pub replicated: bool,
}

impl EventFlags {
    /// Create EventFlags from kernel flags value
    pub fn from_kernel_flags(flags: u32) -> Self {
        Self {
            atomic: (flags & 0x0001) != 0,
            transactional: (flags & 0x0002) != 0,
            causal: (flags & 0x0004) != 0,
            agent_visible: (flags & 0x0008) != 0,
            deterministic: (flags & 0x0010) != 0,
            compressed: (flags & 0x0020) != 0,
            indexed: (flags & 0x0040) != 0,
            replicated: (flags & 0x0080) != 0,
        }
    }
    
    /// Convert to kernel flags value
    pub fn to_kernel_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.atomic { flags |= 0x0001; }
        if self.transactional { flags |= 0x0002; }
        if self.causal { flags |= 0x0004; }
        if self.agent_visible { flags |= 0x0008; }
        if self.deterministic { flags |= 0x0010; }
        if self.compressed { flags |= 0x0020; }
        if self.indexed { flags |= 0x0040; }
        if self.replicated { flags |= 0x0080; }
        flags
    }
}

/// Event Priority Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum EventPriority {
    Critical = 1,
    High = 2,
    Normal = 3,
    Low = 4,
    Background = 5,
}

/// Semantic Event Timestamp
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTimestamp {
    /// UTC timestamp
    pub timestamp: DateTime<Utc>,
    /// Sequence number for ordering
    pub sequence: u64,
    /// CPU ID where event occurred
    pub cpu_id: u32,
    /// Process ID that triggered event
    pub process_id: u32,
}

/// Semantic Event Context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticContext {
    /// Transaction ID if applicable
    pub transaction_id: Option<u64>,
    /// Session ID for agent interactions
    pub session_id: Option<u64>,
    /// Causality chain identifier
    pub causality_chain_id: Option<u64>,
    
    /// Filesystem context
    pub filesystem: Option<FilesystemContext>,
    
    /// Graph context
    pub graph: Option<GraphContext>,
    
    /// Vector context
    pub vector: Option<VectorContext>,
    
    /// Agent context
    pub agent: Option<AgentContext>,
    
    /// System context
    pub system: Option<SystemContext>,
    
    /// Semantic context
    pub semantic: Option<SemanticContextData>,
    
    
    /// Observability context
    pub observability: Option<ObservabilityContext>,
}

/// Filesystem-specific context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemContext {
    pub path: String,
    pub inode_number: Option<u64>,
    pub file_type: Option<String>,
}

/// Graph-specific context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphContext {
    pub node_id: Option<u64>,
    pub edge_id: Option<u64>,
    pub operation_type: Option<u32>,
}

/// Vector-specific context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorContext {
    pub vector_id: Option<u64>,
    pub dimensions: Option<u32>,
    pub element_type: Option<u32>,
}

/// Agent-specific context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContext {
    pub agent_id: String,
    pub intent: Option<String>,
    pub confidence: Option<u32>,
}

/// System-specific context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemContext {
    pub system_load: Option<u32>,
    pub memory_usage: Option<u64>,
    pub io_pressure: Option<u32>,
}

/// Semantic-specific context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticContextData {
    pub tags: HashMap<String, String>,
    pub intent: Option<String>,
    pub confidence: Option<u32>,
}


/// Observability-specific context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservabilityContext {
    pub metric_name: Option<String>,
    pub metric_value: Option<f64>,
    pub metric_unit: Option<String>,
    pub log_level: Option<String>,
    pub log_message: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub service_name: Option<String>,
    pub operation_name: Option<String>,
    pub resource_type: Option<String>,
    pub threshold_value: Option<f64>,
    pub alert_severity: Option<String>,
}

/// Causality Link
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalityLink {
    pub cause_event_id: u64,
    pub effect_event_id: u64,
    pub causality_type: u32,
    pub causality_strength: u32,
    pub causality_delay_ns: u64,
    pub description: Option<String>,
}

/// Complete Semantic Event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticEvent {
    /// Event identification
    pub event_id: u64,
    pub event_type: SemanticEventType,
    pub event_subtype: Option<u32>,
    
    /// Timing and ordering
    pub timestamp: SemanticTimestamp,
    pub global_sequence: u64,
    pub local_sequence: u64,
    
    /// Event metadata
    pub flags: EventFlags,
    pub priority: EventPriority,
    pub event_size: u32,
    
    /// Integrity and versioning
    pub event_version: u32,
    pub checksum: Option<u32>,
    pub compression_type: Option<u32>,
    pub encryption_type: Option<u32>,
    
    /// Causality tracking
    pub causality_links: Vec<CausalityLink>,
    pub parent_event_id: Option<u64>,
    pub root_cause_event_id: Option<u64>,
    
    /// Agent visibility
    pub agent_visibility_mask: u64,
    pub agent_relevance_score: u32,
    pub replay_priority: u32,
    
    /// Rich context
    pub context: SemanticContext,
    
    /// Event payload (JSON-serialized)
    pub payload: Option<serde_json::Value>,
    
    /// Event metadata (JSON-serialized)
    pub metadata: Option<serde_json::Value>,
}

/// Agent Registration Information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub agent_id: String,
    pub agent_name: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub visibility_mask: u64,
    pub max_events_per_query: usize,
    pub max_concurrent_streams: usize,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

/// Agent Authentication Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToken {
    pub agent_id: String,
    pub token_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

/// Event Query Filter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter by event types
    pub event_types: Option<Vec<SemanticEventType>>,
    
    /// Filter by event categories
    pub categories: Option<Vec<EventCategory>>,
    
    /// Filter by time range
    pub time_range: Option<TimeRange>,
    
    /// Filter by agent ID
    pub agent_id: Option<String>,
    
    /// Filter by transaction ID
    pub transaction_id: Option<u64>,
    
    /// Filter by causality chain
    pub causality_chain_id: Option<u64>,
    
    /// Filter by filesystem path pattern
    pub path_pattern: Option<String>,
    
    /// Filter by priority level
    pub min_priority: Option<EventPriority>,
    
    /// Filter by flags
    pub required_flags: Option<EventFlags>,
    
    /// Filter by custom tags
    pub tags: Option<HashMap<String, String>>,
    
    /// Minimum relevance score
    pub min_relevance_score: Option<u32>,
}

/// Time Range for filtering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Event Query Request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventQuery {
    /// Query filter
    pub filter: EventFilter,
    
    /// Maximum number of events to return
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
    
    /// Sort order
    pub sort_by: Option<SortBy>,
    
    /// Include event payload in results
    pub include_payload: bool,
    
    /// Include event metadata in results
    pub include_metadata: bool,
    
    /// Include causality links in results
    pub include_causality: bool,
    
    /// Aggregation options
    pub aggregation: Option<AggregationOptions>,
}

/// Sort options for event queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortBy {
    Timestamp,
    EventId,
    Priority,
    RelevanceScore,
    GlobalSequence,
}

/// Aggregation options for event queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregationOptions {
    pub group_by: Vec<GroupBy>,
    pub metrics: Vec<AggregationMetric>,
    pub time_bucket: Option<TimeBucket>,
}

/// Group by options for aggregation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupBy {
    EventType,
    Category,
    AgentId,
    Priority,
    Hour,
    Day,
    Week,
}

/// Aggregation metrics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationMetric {
    Count,
    AverageRelevanceScore,
    TotalSize,
    UniqueAgents,
}

/// Time bucket for time-based aggregation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeBucket {
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

/// Event Query Response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventQueryResponse {
    pub events: Vec<SemanticEvent>,
    pub total_count: usize,
    pub has_more: bool,
    pub aggregation_results: Option<AggregationResults>,
    pub query_time_ms: u64,
}

/// Aggregation Results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregationResults {
    pub groups: Vec<AggregationGroup>,
    pub total_metrics: HashMap<String, serde_json::Value>,
}

/// Aggregation Group
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregationGroup {
    pub group_key: HashMap<String, String>,
    pub metrics: HashMap<String, serde_json::Value>,
    pub event_count: usize,
}

/// Stream Subscription Request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSubscription {
    pub subscription_id: Uuid,
    pub agent_id: String,
    pub filter: EventFilter,
    pub buffer_size: usize,
    pub include_historical: bool,
    pub historical_limit: Option<usize>,
}

/// Stream Event Message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamEventMessage {
    pub subscription_id: Uuid,
    pub event: SemanticEvent,
    pub sequence_number: u64,
    pub timestamp: DateTime<Utc>,
}

/// API Response wrapper
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub request_id: Uuid,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
            request_id: Uuid::new_v4(),
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: Utc::now(),
            request_id: Uuid::new_v4(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_category() {
        assert_eq!(SemanticEventType::FilesystemCreate.category(), EventCategory::Filesystem);
        assert_eq!(SemanticEventType::GraphNodeCreate.category(), EventCategory::Graph);
        assert_eq!(SemanticEventType::VectorSearch.category(), EventCategory::Vector);
        assert_eq!(SemanticEventType::AgentQuery.category(), EventCategory::Agent);
        assert_eq!(SemanticEventType::SystemMount.category(), EventCategory::System);
        assert_eq!(SemanticEventType::SemanticTransactionBegin.category(), EventCategory::Semantic);
        assert_eq!(SemanticEventType::ObservabilityMetricCollected.category(), EventCategory::Observability);
    }

    #[test]
    fn test_event_flags_conversion() {
        let flags = EventFlags {
            atomic: true,
            transactional: false,
            causal: true,
            agent_visible: true,
            deterministic: false,
            compressed: false,
            indexed: true,
            replicated: false,
        };
        
        let kernel_flags = flags.to_kernel_flags();
        let converted_flags = EventFlags::from_kernel_flags(kernel_flags);
        
        assert_eq!(flags, converted_flags);
    }

    #[test]
    fn test_api_response() {
        let response = ApiResponse::success("test data".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test data".to_string()));
        assert!(response.error.is_none());
        
        let error_response: ApiResponse<String> = ApiResponse::error("test error".to_string());
        assert!(!error_response.success);
        assert!(error_response.data.is_none());
        assert_eq!(error_response.error, Some("test error".to_string()));
    }
}