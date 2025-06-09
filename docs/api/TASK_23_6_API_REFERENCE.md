# Task 23.6 Semantic Event Propagation System - API Reference

## Overview

This document provides a comprehensive API reference for the Task 23.6 Semantic Event Propagation System. The system exposes multiple APIs for different use cases: REST API for external integration, WebSocket API for real-time streaming, and native Rust API for embedded usage.

## 🌐 **REST API**

### Base URL
```
http://localhost:8080/api/v1
```

### Authentication
```http
Authorization: Bearer <token>
Content-Type: application/json
```

### Event Management Endpoints

#### POST /events
Emit a new semantic event into the system.

**Request Body:**
```json
{
  "event_type": "FilesystemWrite",
  "source_boundary": "KernelModule",
  "content": {
    "file_path": "/data/important.txt",
    "operation": "write",
    "size_bytes": "1024"
  },
  "metadata": {
    "user_id": "user123",
    "process_id": "1234"
  }
}
```

**Response:**
```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "accepted",
  "propagation_latency_ns": 387,
  "timestamp": "2025-06-08T21:30:00Z"
}
```

#### GET /events/{event_id}
Retrieve details of a specific event.

**Response:**
```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "event_type": "FilesystemWrite",
  "source_boundary": "KernelModule",
  "content": {
    "file_path": "/data/important.txt",
    "operation": "write",
    "size_bytes": "1024"
  },
  "metadata": {
    "user_id": "user123",
    "process_id": "1234"
  },
  "timestamp": "2025-06-08T21:30:00Z",
  "propagation_status": "completed",
  "routing_decisions": [
    {
      "target_boundary": "AnalyticsLayer",
      "latency_ns": 387
    }
  ]
}
```

#### GET /events
Query events with filtering and pagination.

**Query Parameters:**
- `event_type`: Filter by event type
- `source_boundary`: Filter by source boundary
- `start_time`: Start time (ISO 8601)
- `end_time`: End time (ISO 8601)
- `limit`: Maximum number of results (default: 100)
- `offset`: Pagination offset (default: 0)

**Response:**
```json
{
  "events": [
    {
      "event_id": "550e8400-e29b-41d4-a716-446655440000",
      "event_type": "FilesystemWrite",
      "timestamp": "2025-06-08T21:30:00Z"
    }
  ],
  "total_count": 1,
  "has_more": false
}
```

### Routing Management Endpoints

#### POST /routing/rules
Create a new routing rule.

**Request Body:**
```json
{
  "rule_id": "filesystem_to_analytics",
  "priority": 100,
  "conditions": {
    "event_types": ["FilesystemCreate", "FilesystemWrite", "FilesystemDelete"],
    "content_patterns": [
      {
        "pattern_type": "Regex",
        "pattern": ".*\\.(log|txt|md)$",
        "case_sensitive": false
      }
    ]
  },
  "target_boundaries": ["AnalyticsLayer", "GraphLayer"],
  "actions": {
    "route_to_boundaries": ["AnalyticsLayer"],
    "priority_boost": 2,
    "log_match": true,
    "emit_metrics": true
  }
}
```

**Response:**
```json
{
  "rule_id": "filesystem_to_analytics",
  "status": "created",
  "validation_result": {
    "valid": true,
    "estimated_performance_impact": "low"
  }
}
```

#### GET /routing/rules
List all routing rules.

**Response:**
```json
{
  "rules": [
    {
      "rule_id": "filesystem_to_analytics",
      "priority": 100,
      "enabled": true,
      "match_count": 1234,
      "average_latency_ns": 45
    }
  ]
}
```

#### PUT /routing/rules/{rule_id}
Update an existing routing rule.

#### DELETE /routing/rules/{rule_id}
Delete a routing rule.

### Automation Endpoints

#### POST /automation/workflows
Register a new reactive workflow.

**Request Body:**
```json
{
  "name": "Automated Backup Workflow",
  "description": "Automatically backup files when disk usage exceeds threshold",
  "workflow_type": "Linear",
  "trigger_patterns": [
    {
      "type": "StateChange",
      "state_path": "system.disk_usage_percent",
      "condition": {
        "operator": "GreaterThan",
        "value": 80.0,
        "tolerance": 5.0
      }
    }
  ],
  "steps": [
    {
      "name": "Identify Large Files",
      "action": {
        "type": "ExecuteScript",
        "script_language": "Python",
        "script_content": "# Python script here",
        "parameters": {}
      },
      "timeout": "60s",
      "retry_policy": {
        "max_retries": 3,
        "backoff_strategy": "Exponential",
        "base_delay": "1s"
      }
    }
  ],
  "priority": "High",
  "enabled": true
}
```

**Response:**
```json
{
  "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "registered",
  "validation_result": {
    "valid": true,
    "estimated_resource_usage": "medium"
  }
}
```

#### GET /automation/workflows
List all registered workflows.

#### GET /automation/workflows/{workflow_id}/executions
Get execution history for a workflow.

**Response:**
```json
{
  "executions": [
    {
      "execution_id": "550e8400-e29b-41d4-a716-446655440001",
      "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "completed",
      "start_time": "2025-06-08T21:30:00Z",
      "end_time": "2025-06-08T21:30:05Z",
      "execution_time_ms": 5000,
      "steps_completed": 2,
      "steps_total": 2
    }
  ]
}
```

### Analytics Endpoints

#### POST /analytics/queries
Register a new analytics query.

**Request Body:**
```json
{
  "name": "File Operation Trends",
  "query_type": "Aggregation",
  "source_events": ["FilesystemCreate", "FilesystemWrite", "FilesystemDelete"],
  "window": {
    "window_type": "Tumbling",
    "size": "60s"
  },
  "aggregations": [
    {
      "field": "event_type",
      "function": "Count",
      "group_by": ["event_type"]
    },
    {
      "field": "size_bytes",
      "function": "Sum",
      "group_by": ["user_id"]
    }
  ],
  "filters": [
    {
      "field": "file_path",
      "operator": "Contains",
      "value": "/data/"
    }
  ],
  "output_format": "JSON"
}
```

**Response:**
```json
{
  "query_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "registered",
  "estimated_throughput": 50000
}
```

#### GET /analytics/queries/{query_id}/results
Get results from an analytics query.

**Response:**
```json
{
  "query_id": "550e8400-e29b-41d4-a716-446655440000",
  "results": [
    {
      "window_start": "2025-06-08T21:30:00Z",
      "window_end": "2025-06-08T21:31:00Z",
      "aggregations": {
        "FilesystemWrite": 1234,
        "FilesystemRead": 5678,
        "FilesystemDelete": 123
      }
    }
  ],
  "metadata": {
    "events_processed": 7035,
    "processing_latency_ms": 15
  }
}
```

### Monitoring Endpoints

#### GET /monitoring/health
Get system health status.

**Response:**
```json
{
  "status": "healthy",
  "components": {
    "event_propagation": {
      "status": "healthy",
      "latency_ns": 387,
      "throughput_eps": 75000
    },
    "routing_engine": {
      "status": "healthy",
      "active_rules": 25,
      "match_rate": 0.85
    },
    "automation_framework": {
      "status": "healthy",
      "active_workflows": 15,
      "execution_rate": 0.98
    },
    "analytics_engine": {
      "status": "healthy",
      "active_queries": 8,
      "processing_latency_ms": 12
    }
  },
  "overall_health_score": 0.96
}
```

#### GET /monitoring/metrics
Get system metrics.

**Response:**
```json
{
  "timestamp": "2025-06-08T21:30:00Z",
  "metrics": {
    "events_per_second": 75000,
    "average_propagation_latency_ns": 387,
    "memory_usage_mb": 512,
    "cpu_usage_percent": 25.5,
    "disk_usage_percent": 45.2,
    "network_throughput_mbps": 100.5
  },
  "performance_score": 0.94
}
```

## 🔌 **WebSocket API**

### Connection
```javascript
const ws = new WebSocket('ws://localhost:8080/ws/events');
```

### Real-time Event Streaming

#### Subscribe to Events
```json
{
  "type": "subscribe",
  "filters": {
    "event_types": ["FilesystemWrite", "FilesystemRead"],
    "source_boundaries": ["KernelModule", "FuseLayer"]
  }
}
```

#### Event Stream Message
```json
{
  "type": "event",
  "data": {
    "event_id": "550e8400-e29b-41d4-a716-446655440000",
    "event_type": "FilesystemWrite",
    "source_boundary": "KernelModule",
    "content": {
      "file_path": "/data/important.txt",
      "operation": "write",
      "size_bytes": "1024"
    },
    "timestamp": "2025-06-08T21:30:00Z"
  }
}
```

#### Analytics Results Streaming
```json
{
  "type": "subscribe_analytics",
  "query_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

```json
{
  "type": "analytics_result",
  "query_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": {
    "window_start": "2025-06-08T21:30:00Z",
    "window_end": "2025-06-08T21:31:00Z",
    "results": {
      "total_events": 1234,
      "events_by_type": {
        "FilesystemWrite": 800,
        "FilesystemRead": 434
      }
    }
  }
}
```

## 🦀 **Rust API**

### Core Types

#### SemanticEvent
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEvent {
    pub event_id: Uuid,
    pub event_type: SemanticEventType,
    pub source_boundary: EventBoundary,
    pub content: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: SystemTime,
}
```

#### SemanticEventType
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticEventType {
    FilesystemCreate,
    FilesystemWrite,
    FilesystemRead,
    FilesystemDelete,
    FilesystemMove,
    FilesystemPermissionChange,
    UserAction,
    SystemEvent,
    NetworkEvent,
    SecurityEvent,
    PerformanceEvent,
    Custom(String),
}
```

#### EventBoundary
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventBoundary {
    KernelModule,
    FuseLayer,
    UserSpace,
    NetworkLayer,
    StorageLayer,
    AnalyticsLayer,
    GraphLayer,
    ExternalSystem,
}
```

### Event Propagation API

#### EventPropagationManager
```rust
impl EventPropagationManager {
    pub fn new(config: EventPropagationConfig) -> Result<Self>;
    
    pub async fn propagate_event(&mut self, event: SemanticEvent) -> Result<PropagationResult>;
    
    pub async fn get_statistics(&self) -> Result<PropagationStatistics>;
    
    pub async fn update_config(&mut self, config: EventPropagationConfig) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct PropagationResult {
    pub event_id: Uuid,
    pub propagation_latency_ns: u64,
    pub target_boundaries: Vec<EventBoundary>,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct PropagationStatistics {
    pub events_propagated: u64,
    pub average_latency_ns: u64,
    pub throughput_events_per_sec: u64,
    pub error_rate: f64,
}
```

### Event Routing API

#### EventRoutingEngine
```rust
impl EventRoutingEngine {
    pub fn new(config: EventRoutingConfig) -> Result<Self>;
    
    pub async fn add_routing_rule(&mut self, rule: EventRoutingRule) -> Result<()>;
    
    pub async fn remove_routing_rule(&mut self, rule_id: &str) -> Result<()>;
    
    pub async fn route_event(&self, event: SemanticEvent) -> Result<RoutingDecision>;
    
    pub async fn get_routing_statistics(&self) -> Result<RoutingStatistics>;
}

#[derive(Debug, Clone)]
pub struct EventRoutingRule {
    pub rule_id: String,
    pub priority: u32,
    pub conditions: RoutingConditions,
    pub target_boundaries: Vec<EventBoundary>,
    pub actions: RoutingActions,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub event_id: Uuid,
    pub matched_rules: Vec<String>,
    pub target_boundaries: Vec<EventBoundary>,
    pub routing_latency_ns: u64,
}
```

### Automation API

#### ReactiveAutomationFramework
```rust
impl ReactiveAutomationFramework {
    pub fn new(config: ReactiveAutomationConfig) -> Result<Self>;
    
    pub async fn register_workflow(&mut self, workflow: ReactiveWorkflow) -> Result<Uuid>;
    
    pub async fn execute_workflow_by_id(&self, workflow_id: Uuid) -> Result<WorkflowExecutionResult>;
    
    pub async fn register_cep_pattern(&mut self, pattern: ComplexEventPattern) -> Result<Uuid>;
    
    pub async fn get_workflow_statistics(&self) -> Result<AutomationStatistics>;
}

#[derive(Debug, Clone)]
pub struct ReactiveWorkflow {
    pub workflow_id: Uuid,
    pub name: String,
    pub description: String,
    pub workflow_type: WorkflowType,
    pub trigger_patterns: Vec<WorkflowTrigger>,
    pub steps: Vec<WorkflowStep>,
    pub compensation_steps: Vec<CompensationStep>,
    pub priority: WorkflowPriority,
    pub tenant_id: Option<String>,
    pub enabled: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionResult {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub success: bool,
    pub execution_time_ms: u64,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub error_message: Option<String>,
}
```

### Analytics API

#### EventStreamAnalyticsEngine
```rust
impl EventStreamAnalyticsEngine {
    pub fn new(config: StreamAnalyticsConfig) -> Result<Self>;
    
    pub async fn register_query(&mut self, query: AnalyticsQuery) -> Result<Uuid>;
    
    pub async fn process_event_stream(&self, events: Vec<SemanticEvent>) -> Result<AnalyticsResult>;
    
    pub async fn get_query_results(&self, query_id: Uuid) -> Result<Vec<QueryResult>>;
    
    pub async fn get_analytics_statistics(&self) -> Result<AnalyticsStatistics>;
}

#[derive(Debug, Clone)]
pub struct AnalyticsQuery {
    pub query_id: Uuid,
    pub name: String,
    pub query_type: QueryType,
    pub source_events: Vec<SemanticEventType>,
    pub window: WindowDefinition,
    pub aggregations: Vec<Aggregation>,
    pub filters: Vec<AnalyticsFilter>,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct AnalyticsResult {
    pub query_id: Uuid,
    pub events_processed: u64,
    pub processing_latency_ns: u64,
    pub results: Vec<QueryResult>,
}
```

### Monitoring API

#### MonitoringSystem
```rust
impl MonitoringSystem {
    pub fn new(config: MonitoringSystemConfig) -> Result<Self>;
    
    pub async fn register_custom_metric(&mut self, metric: CustomMetric) -> Result<()>;
    
    pub async fn collect_metrics(&self) -> Result<SystemMetrics>;
    
    pub async fn register_alert_rule(&mut self, rule: AlertRule) -> Result<Uuid>;
    
    pub async fn get_system_health(&self) -> Result<SystemHealth>;
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub timestamp: SystemTime,
    pub events_per_second: u64,
    pub average_propagation_latency_ns: u64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_throughput_mbps: f64,
    pub system_health: f64,
    pub performance_score: f64,
}

#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub component_health: HashMap<String, ComponentHealth>,
    pub health_score: f64,
    pub last_updated: SystemTime,
}
```

## 🔧 **Configuration API**

### EventPropagationConfig
```rust
#[derive(Debug, Clone)]
pub struct EventPropagationConfig {
    pub max_propagation_latency_ns: u64,
    pub target_throughput_events_per_sec: u64,
    pub enable_context_preservation: bool,
    pub enable_deduplication: bool,
    pub enable_performance_optimization: bool,
    pub enable_batching: bool,
    pub batch_size: usize,
    pub memory_pool_size: usize,
    pub worker_thread_count: usize,
}

impl Default for EventPropagationConfig {
    fn default() -> Self {
        Self {
            max_propagation_latency_ns: 500,
            target_throughput_events_per_sec: 50_000,
            enable_context_preservation: true,
            enable_deduplication: true,
            enable_performance_optimization: false,
            enable_batching: false,
            batch_size: 100,
            memory_pool_size: 1_000_000,
            worker_thread_count: num_cpus::get(),
        }
    }
}
```

### StreamAnalyticsConfig
```rust
#[derive(Debug, Clone)]
pub struct StreamAnalyticsConfig {
    pub target_throughput_events_per_sec: u64,
    pub processing_latency_target_ns: u64,
    pub tumbling_window_size_ms: u64,
    pub sliding_window_size_ms: u64,
    pub session_timeout_ms: u64,
    pub event_buffer_size: usize,
    pub aggregation_buffer_size: usize,
    pub enable_complex_aggregations: bool,
    pub enable_correlation_analysis: bool,
    pub enable_statistical_analysis: bool,
    pub enable_pattern_detection: bool,
    pub enable_real_time_processing: bool,
    pub enable_parallel_processing: bool,
    pub worker_thread_count: usize,
}
```

## 📊 **Error Handling**

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum SemanticEventError {
    #[error("Event propagation failed: {0}")]
    PropagationError(String),
    
    #[error("Routing rule validation failed: {0}")]
    RoutingValidationError(String),
    
    #[error("Workflow execution failed: {0}")]
    WorkflowExecutionError(String),
    
    #[error("Analytics query failed: {0}")]
    AnalyticsQueryError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("System resource exhausted: {0}")]
    ResourceExhaustedError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### HTTP Error Responses
```json
{
  "error": {
    "code": "PROPAGATION_FAILED",
    "message": "Event propagation failed due to network timeout",
    "details": {
      "event_id": "550e8400-e29b-41d4-a716-446655440000",
      "retry_after": 5000
    }
  }
}
```

## 🔐 **Authentication & Authorization**

### API Key Authentication
```http
Authorization: Bearer sk-1234567890abcdef
```

### JWT Token Authentication
```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Role-Based Access Control
```rust
#[derive(Debug, Clone)]
pub enum Permission {
    EventRead,
    EventWrite,
    RoutingRuleManage,
    WorkflowManage,
    AnalyticsQuery,
    SystemMonitor,
    SystemAdmin,
}

#[derive(Debug, Clone)]
pub struct UserRole {
    pub role_name: String,
    pub permissions: Vec<Permission>,
    pub resource_limits: ResourceLimits,
}
```

## 📈 **Rate Limiting**

### Rate Limit Headers
```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1625097600
```

### Rate Limit Configuration
```rust
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub enable_per_user_limits: bool,
    pub enable_per_endpoint_limits: bool,
}
```

---

*This API reference provides comprehensive documentation for all interfaces exposed by the Task 23.6 Semantic Event Propagation System.*