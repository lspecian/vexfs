# Task 23.6 Semantic Event Propagation System - Usage Guide

## Overview

This guide provides comprehensive instructions for using the Task 23.6 Semantic Event Propagation System, which transforms VexFS into an intelligent, AI-native semantic computing platform. The system provides real-time event processing, intelligent automation, and advanced analytics capabilities across all filesystem boundaries.

## 🚀 **QUICK START**

### Prerequisites

- VexFS with Task 23.4 (Semantic Journaling) and Task 23.5 (Graph Capabilities) installed
- Rust 1.70+ for building components
- Docker (optional, for containerized deployment)
- Kubernetes (optional, for orchestrated deployment)

### Basic Setup

1. **Enable Semantic Event Propagation**
```rust
use vexfs::semantic_api::*;

// Initialize the complete system
let config = SemanticEventConfig {
    enable_cross_boundary_propagation: true,
    enable_advanced_routing: true,
    enable_distributed_coordination: true,
    enable_reactive_automation: true,
    enable_advanced_analytics: true,
    ..Default::default()
};

let semantic_system = SemanticEventSystem::new(config).await?;
semantic_system.start().await?;
```

2. **Basic Event Emission**
```rust
// Emit a filesystem event
let event = SemanticEvent {
    event_id: Uuid::new_v4(),
    event_type: SemanticEventType::FilesystemWrite,
    source_boundary: EventBoundary::KernelModule,
    content: hashmap! {
        "file_path".to_string() => "/data/important.txt".to_string(),
        "operation".to_string() => "write".to_string(),
        "size_bytes".to_string() => "1024".to_string(),
    },
    metadata: hashmap! {
        "user_id".to_string() => "user123".to_string(),
        "process_id".to_string() => "1234".to_string(),
    },
    timestamp: SystemTime::now(),
};

semantic_system.emit_event(event).await?;
```

## 📋 **PHASE-BY-PHASE USAGE**

### Phase 2: Core Event Propagation

#### Basic Event Propagation
```rust
use vexfs::semantic_api::event_propagation::*;

// Initialize event propagation manager
let propagation_config = EventPropagationConfig {
    max_propagation_latency_ns: 500,
    target_throughput_events_per_sec: 50_000,
    enable_context_preservation: true,
    enable_deduplication: true,
    ..Default::default()
};

let mut propagation_manager = EventPropagationManager::new(propagation_config)?;

// Propagate events across boundaries
let event = create_filesystem_event();
propagation_manager.propagate_event(event).await?;

// Monitor propagation statistics
let stats = propagation_manager.get_statistics().await?;
println!("Events propagated: {}", stats.events_propagated);
println!("Average latency: {}ns", stats.average_latency_ns);
```

#### Kernel-FUSE Bridge Usage
```rust
use vexfs::semantic_api::kernel_fuse_bridge::*;

// Initialize bidirectional bridge
let bridge_config = KernelFuseBridgeConfig {
    max_translation_latency_ns: 200,
    enable_bidirectional_sync: true,
    enable_zero_copy: true,
    enable_context_validation: true,
    ..Default::default()
};

let mut bridge = KernelFuseBridge::new(bridge_config)?;

// Translate kernel events to FUSE events
let kernel_event = create_kernel_event();
let fuse_event = bridge.translate_kernel_to_fuse(kernel_event).await?;

// Validate context preservation
let preserved = bridge.validate_context_preservation(&kernel_event, &fuse_event).await?;
assert!(preserved, "Context must be preserved during translation");
```

### Phase 3: Advanced Routing and Filtering

#### Event Routing Configuration
```rust
use vexfs::semantic_api::event_routing::*;

// Create routing rules
let routing_rule = EventRoutingRule {
    rule_id: "filesystem_to_analytics".to_string(),
    priority: 100,
    conditions: RoutingConditions {
        event_types: Some(vec![
            SemanticEventType::FilesystemCreate,
            SemanticEventType::FilesystemWrite,
            SemanticEventType::FilesystemDelete,
        ]),
        content_patterns: Some(vec![
            ContentPattern {
                pattern_type: PatternType::Regex,
                pattern: r".*\.(log|txt|md)$".to_string(),
                case_sensitive: false,
            }
        ]),
        ..Default::default()
    },
    target_boundaries: vec![EventBoundary::AnalyticsLayer, EventBoundary::GraphLayer],
    actions: RoutingActions {
        route_to_boundaries: Some(vec![EventBoundary::AnalyticsLayer]),
        priority_boost: Some(2),
        log_match: true,
        emit_metrics: true,
        ..Default::default()
    },
};

// Initialize routing engine
let mut routing_engine = EventRoutingEngine::new(EventRoutingConfig::default())?;
routing_engine.add_routing_rule(routing_rule).await?;

// Route events
let event = create_filesystem_event();
let routing_decision = routing_engine.route_event(event).await?;
println!("Event routed to: {:?}", routing_decision.target_boundaries);
```

#### Event Filtering Setup
```rust
use vexfs::semantic_api::event_filtering::*;

// Create content filter
let content_filter = EventFilter {
    filter_id: "sensitive_data_filter".to_string(),
    filter_type: FilterType::Content,
    conditions: FilterConditions {
        content_patterns: Some(vec![
            ContentPattern {
                pattern_type: PatternType::Regex,
                pattern: r"(?i)(password|secret|key|token)".to_string(),
                case_sensitive: false,
            }
        ]),
        ..Default::default()
    },
    actions: FilterActions {
        action: FilterAction::Block,
        log_action: true,
        emit_metrics: true,
        add_metadata: Some(hashmap! {
            "security_classification".to_string() => "sensitive".to_string(),
        }),
    },
};

// Initialize filtering engine
let mut filtering_engine = EventFilteringEngine::new(EventFilteringConfig::default())?;
filtering_engine.add_filter(content_filter).await?;

// Filter events
let event = create_event_with_sensitive_data();
let filter_result = filtering_engine.filter_event(event).await?;
if filter_result.action == FilterAction::Block {
    println!("Event blocked due to sensitive content");
}
```

### Phase 4: Distributed Event Coordination

#### Cluster Setup
```rust
use vexfs::semantic_api::distributed_coordinator::*;

// Initialize distributed coordinator
let coordinator_config = DistributedCoordinatorConfig {
    cluster_id: "vexfs-cluster-1".to_string(),
    node_id: "node-1".to_string(),
    cluster_size: 3,
    max_consensus_latency_ms: 10,
    enable_raft_consensus: true,
    enable_conflict_resolution: true,
    enable_partition_tolerance: true,
    ..Default::default()
};

let mut coordinator = DistributedEventCoordinator::new(coordinator_config)?;

// Join cluster
coordinator.join_cluster(vec![
    "node-2:8080".to_string(),
    "node-3:8080".to_string(),
]).await?;

// Achieve consensus on events
let event = create_distributed_event();
let consensus_result = coordinator.achieve_consensus(event).await?;
if consensus_result.success {
    println!("Consensus achieved in {}ms", consensus_result.latency_ms);
}
```

#### Conflict Resolution
```rust
// Handle conflicting events
let event1 = create_conflicting_event_1();
let event2 = create_conflicting_event_2();

let resolution = coordinator.resolve_conflict(&event1, &event2).await?;
match resolution.resolution_strategy.as_str() {
    "last_writer_wins" => println!("Conflict resolved using last writer wins"),
    "merge" => println!("Conflict resolved by merging events"),
    "user_intervention" => println!("Conflict requires user intervention"),
    _ => println!("Unknown resolution strategy"),
}
```

### Phase 5: Reactive Automation

#### Workflow Definition
```rust
use vexfs::semantic_api::automation_framework::*;

// Define a reactive workflow
let workflow = ReactiveWorkflow {
    workflow_id: Uuid::new_v4(),
    name: "Automated Backup Workflow".to_string(),
    description: "Automatically backup files when disk usage exceeds threshold".to_string(),
    workflow_type: WorkflowType::Linear,
    trigger_patterns: vec![
        WorkflowTrigger::StateChange {
            state_path: "system.disk_usage_percent".to_string(),
            condition: StateCondition {
                operator: ComparisonOperator::GreaterThan,
                value: serde_json::json!(80.0),
                tolerance: Some(5.0),
            },
        }
    ],
    steps: vec![
        WorkflowStep {
            step_id: Uuid::new_v4(),
            name: "Identify Large Files".to_string(),
            action: WorkflowAction::ExecuteScript {
                script_language: ScriptLanguage::Python,
                script_content: r#"
import os
import json

def find_large_files(directory="/data", min_size_mb=100):
    large_files = []
    for root, dirs, files in os.walk(directory):
        for file in files:
            file_path = os.path.join(root, file)
            try:
                size_mb = os.path.getsize(file_path) / (1024 * 1024)
                if size_mb >= min_size_mb:
                    large_files.append({
                        "path": file_path,
                        "size_mb": size_mb
                    })
            except OSError:
                continue
    return large_files

result = find_large_files()
print(json.dumps(result))
"#.to_string(),
                parameters: HashMap::new(),
            },
            timeout: Duration::from_secs(60),
            retry_policy: RetryPolicy {
                max_retries: 3,
                backoff_strategy: BackoffStrategy::Exponential,
                base_delay: Duration::from_secs(1),
            },
            ..Default::default()
        },
        WorkflowStep {
            step_id: Uuid::new_v4(),
            name: "Create Backup".to_string(),
            action: WorkflowAction::CallAPI {
                url: "http://backup-service:8080/api/backup".to_string(),
                method: "POST".to_string(),
                headers: hashmap! {
                    "Content-Type".to_string() => "application/json".to_string(),
                },
                body: Some(serde_json::json!({
                    "source": "/data",
                    "destination": "/backups",
                    "compression": true
                })),
            },
            timeout: Duration::from_secs(300),
            retry_policy: RetryPolicy::default(),
            ..Default::default()
        }
    ],
    compensation_steps: vec![
        CompensationStep {
            step_id: Uuid::new_v4(),
            name: "Cleanup Failed Backup".to_string(),
            action: WorkflowAction::ExecuteScript {
                script_language: ScriptLanguage::Bash,
                script_content: "rm -rf /backups/incomplete_*".to_string(),
                parameters: HashMap::new(),
            },
            ..Default::default()
        }
    ],
    priority: WorkflowPriority::High,
    tenant_id: Some("tenant-1".to_string()),
    enabled: true,
    metadata: HashMap::new(),
};

// Initialize automation framework
let automation_config = ReactiveAutomationConfig {
    max_automation_latency_ms: 100,
    target_throughput_events_per_sec: 100_000,
    max_concurrent_workflows: 10_000,
    enable_fault_tolerance: true,
    enable_compensation: true,
    ..Default::default()
};

let mut automation_framework = ReactiveAutomationFramework::new(automation_config)?;

// Register workflow
automation_framework.register_workflow(workflow).await?;

// Execute workflow manually (for testing)
let workflow_id = Uuid::new_v4();
let execution_result = automation_framework.execute_workflow_by_id(workflow_id).await?;
if execution_result.success {
    println!("Workflow executed successfully in {}ms", execution_result.execution_time_ms);
}
```

#### Complex Event Processing
```rust
// Define complex event patterns
let cep_pattern = ComplexEventPattern {
    pattern_id: Uuid::new_v4(),
    name: "Security Breach Detection".to_string(),
    description: "Detect potential security breaches based on event patterns".to_string(),
    pattern_type: PatternType::Sequence,
    events: vec![
        EventPatternElement {
            event_type: SemanticEventType::UserAction,
            conditions: hashmap! {
                "action".to_string() => "failed_login".to_string(),
            },
            time_window: Some(Duration::from_secs(60)),
            min_occurrences: 3,
        },
        EventPatternElement {
            event_type: SemanticEventType::FilesystemRead,
            conditions: hashmap! {
                "file_path".to_string() => "/etc/passwd".to_string(),
            },
            time_window: Some(Duration::from_secs(300)),
            min_occurrences: 1,
        },
    ],
    confidence_threshold: 0.8,
    actions: vec![
        PatternAction::EmitAlert {
            severity: AlertSeverity::Critical,
            message: "Potential security breach detected".to_string(),
        },
        PatternAction::TriggerWorkflow {
            workflow_id: Uuid::new_v4(),
        },
    ],
};

// Register pattern with CEP engine
automation_framework.register_cep_pattern(cep_pattern).await?;
```

### Phase 6: Advanced Analytics and Monitoring

#### Stream Analytics Setup
```rust
use vexfs::semantic_api::stream_analytics::*;

// Initialize stream analytics engine
let analytics_config = StreamAnalyticsConfig {
    target_throughput_events_per_sec: 1_000_000,
    tumbling_window_size_ms: 1000,
    sliding_window_size_ms: 5000,
    session_timeout_ms: 30000,
    event_buffer_size: 100_000,
    enable_complex_aggregations: true,
    enable_correlation_analysis: true,
    enable_statistical_analysis: true,
    enable_pattern_detection: true,
    ..Default::default()
};

let mut analytics_engine = EventStreamAnalyticsEngine::new(analytics_config)?;

// Define analytics queries
let query = AnalyticsQuery {
    query_id: Uuid::new_v4(),
    name: "File Operation Trends".to_string(),
    query_type: QueryType::Aggregation,
    source_events: vec![
        SemanticEventType::FilesystemCreate,
        SemanticEventType::FilesystemWrite,
        SemanticEventType::FilesystemDelete,
    ],
    window: WindowDefinition {
        window_type: WindowType::Tumbling,
        size: Duration::from_secs(60),
        slide: None,
    },
    aggregations: vec![
        Aggregation {
            field: "event_type".to_string(),
            function: AggregationFunction::Count,
            group_by: Some(vec!["event_type".to_string()]),
        },
        Aggregation {
            field: "size_bytes".to_string(),
            function: AggregationFunction::Sum,
            group_by: Some(vec!["user_id".to_string()]),
        },
    ],
    filters: vec![
        AnalyticsFilter {
            field: "file_path".to_string(),
            operator: FilterOperator::Contains,
            value: serde_json::json!("/data/"),
        }
    ],
    output_format: OutputFormat::JSON,
};

// Register query
analytics_engine.register_query(query).await?;

// Process event stream
let events = generate_sample_events(1000);
let analytics_result = analytics_engine.process_event_stream(events).await?;
println!("Processed {} events in {}ms", 
         analytics_result.events_processed, 
         analytics_result.processing_latency_ns / 1_000_000);
```

#### Real-time Monitoring
```rust
use vexfs::semantic_api::monitoring_system::*;

// Initialize monitoring system
let monitoring_config = MonitoringSystemConfig {
    metrics_collection_interval_ms: 1000,
    metrics_retention_hours: 24,
    enable_prometheus_export: true,
    enable_custom_metrics: true,
    enable_real_time_alerts: true,
    enable_performance_monitoring: true,
    enable_health_monitoring: true,
    ..Default::default()
};

let mut monitoring_system = MonitoringSystem::new(monitoring_config)?;

// Define custom metrics
let custom_metric = CustomMetric {
    name: "vexfs_file_operations_per_second".to_string(),
    metric_type: MetricType::Counter,
    description: "Number of file operations per second".to_string(),
    labels: vec!["operation_type".to_string(), "user_id".to_string()],
};

monitoring_system.register_custom_metric(custom_metric).await?;

// Collect and export metrics
let metrics = monitoring_system.collect_metrics().await?;
println!("System health: {:.2}%", metrics.system_health * 100.0);
println!("Performance score: {:.2}%", metrics.performance_score * 100.0);

// Set up alerts
let alert_rule = AlertRule {
    rule_id: Uuid::new_v4(),
    name: "High Memory Usage".to_string(),
    condition: AlertCondition {
        metric: "vexfs_memory_usage_percent".to_string(),
        operator: ComparisonOperator::GreaterThan,
        threshold: 85.0,
        duration: Duration::from_secs(300),
    },
    severity: AlertSeverity::Warning,
    actions: vec![
        AlertAction::SendEmail {
            recipients: vec!["admin@example.com".to_string()],
            subject: "VexFS High Memory Usage Alert".to_string(),
        },
        AlertAction::SendWebhook {
            url: "https://hooks.slack.com/services/...".to_string(),
            payload: serde_json::json!({
                "text": "VexFS memory usage is above 85%"
            }),
        },
    ],
};

monitoring_system.register_alert_rule(alert_rule).await?;
```

## 🔧 **CONFIGURATION EXAMPLES**

### Production Configuration
```toml
# config/production.toml
[event_propagation]
max_propagation_latency_ns = 500
target_throughput_events_per_sec = 100_000
enable_context_preservation = true
enable_deduplication = true
enable_performance_optimization = true

[event_routing]
max_pattern_matching_latency_ns = 50
enable_dynamic_reconfiguration = true
enable_load_balancing = true
enable_pattern_caching = true
max_routing_rules = 100_000

[distributed_coordination]
cluster_size = 3
max_consensus_latency_ms = 10
enable_raft_consensus = true
enable_conflict_resolution = true
enable_partition_tolerance = true

[reactive_automation]
max_automation_latency_ms = 100
target_throughput_events_per_sec = 100_000
max_concurrent_workflows = 10_000
enable_fault_tolerance = true
enable_compensation = true

[stream_analytics]
target_throughput_events_per_sec = 1_000_000
processing_latency_target_ns = 1_000_000
enable_real_time_processing = true
enable_pattern_discovery = true
enable_anomaly_detection = true
enable_predictive_analytics = true

[monitoring]
metrics_collection_interval_ms = 1000
enable_prometheus_export = true
enable_real_time_alerts = true
enable_performance_monitoring = true
enable_health_monitoring = true
```

### Development Configuration
```toml
# config/development.toml
[event_propagation]
max_propagation_latency_ns = 1000
target_throughput_events_per_sec = 10_000
enable_context_preservation = true
enable_deduplication = false
enable_performance_optimization = false

[event_routing]
max_pattern_matching_latency_ns = 100
enable_dynamic_reconfiguration = true
enable_load_balancing = false
enable_pattern_caching = true
max_routing_rules = 1_000

[distributed_coordination]
cluster_size = 1
max_consensus_latency_ms = 50
enable_raft_consensus = false
enable_conflict_resolution = true
enable_partition_tolerance = false

[reactive_automation]
max_automation_latency_ms = 500
target_throughput_events_per_sec = 10_000
max_concurrent_workflows = 100
enable_fault_tolerance = true
enable_compensation = true

[stream_analytics]
target_throughput_events_per_sec = 100_000
processing_latency_target_ns = 10_000_000
enable_real_time_processing = true
enable_pattern_discovery = false
enable_anomaly_detection = false
enable_predictive_analytics = false

[monitoring]
metrics_collection_interval_ms = 5000
enable_prometheus_export = false
enable_real_time_alerts = false
enable_performance_monitoring = true
enable_health_monitoring = true
```

## 🚨 **TROUBLESHOOTING**

### Common Issues and Solutions

#### High Latency
```rust
// Check propagation statistics
let stats = propagation_manager.get_statistics().await?;
if stats.average_latency_ns > 500 {
    // Optimize configuration
    let optimized_config = EventPropagationConfig {
        enable_performance_optimization: true,
        memory_pool_size: 1_000_000,
        batch_size: 1000,
        ..config
    };
    propagation_manager.update_config(optimized_config).await?;
}
```

#### Memory Issues
```rust
// Monitor memory usage
let memory_stats = monitoring_system.get_memory_statistics().await?;
if memory_stats.usage_percent > 80.0 {
    // Trigger cleanup
    analytics_engine.cleanup_old_data().await?;
    automation_framework.cleanup_completed_workflows().await?;
    
    // Adjust buffer sizes
    let reduced_config = StreamAnalyticsConfig {
        event_buffer_size: 50_000,
        aggregation_buffer_size: 25_000,
        ..analytics_config
    };
    analytics_engine.update_config(reduced_config).await?;
}
```

#### Consensus Failures
```rust
// Check cluster health
let cluster_status = coordinator.get_cluster_status().await?;
if cluster_status.healthy_nodes < cluster_status.total_nodes / 2 + 1 {
    // Attempt cluster recovery
    coordinator.trigger_leader_election().await?;
    
    // Check for network partitions
    let partition_status = coordinator.check_network_partitions().await?;
    if partition_status.partitioned {
        println!("Network partition detected, waiting for recovery...");
        coordinator.wait_for_partition_recovery().await?;
    }
}
```

## 📊 **PERFORMANCE TUNING**

### Optimization Guidelines

#### Event Propagation Optimization
```rust
// High-throughput configuration
let high_throughput_config = EventPropagationConfig {
    max_propagation_latency_ns: 500,
    target_throughput_events_per_sec: 200_000,
    enable_performance_optimization: true,
    enable_batching: true,
    batch_size: 1000,
    memory_pool_size: 10_000_000,
    worker_thread_count: num_cpus::get(),
    ..Default::default()
};
```

#### Analytics Performance Tuning
```rust
// High-performance analytics configuration
let high_perf_analytics_config = StreamAnalyticsConfig {
    target_throughput_events_per_sec: 2_000_000,
    processing_latency_target_ns: 500_000,
    enable_parallel_processing: true,
    worker_thread_count: num_cpus::get() * 2,
    enable_simd_optimization: true,
    enable_gpu_acceleration: true,
    memory_pool_size: 100_000_000,
    ..Default::default()
};
```

## 🎯 **BEST PRACTICES**

### Event Design
- **Use structured event data**: Consistent field naming and types
- **Include sufficient context**: Ensure events contain all necessary information
- **Optimize event size**: Balance information content with performance
- **Use appropriate event types**: Choose the most specific event type available

### Routing Rules
- **Order by specificity**: More specific rules should have higher priority
- **Use efficient patterns**: Prefer simple patterns over complex regex when possible
- **Cache frequently used patterns**: Enable pattern caching for better performance
- **Monitor rule performance**: Regularly review rule execution statistics

### Automation Workflows
- **Design for idempotency**: Workflows should be safe to retry
- **Include compensation logic**: Always define rollback procedures
- **Use appropriate timeouts**: Balance responsiveness with reliability
- **Monitor workflow performance**: Track execution times and success rates

### Monitoring and Alerting
- **Set meaningful thresholds**: Base alerts on actual operational experience
- **Avoid alert fatigue**: Ensure alerts are actionable and not too frequent
- **Use escalation policies**: Define clear escalation paths for critical alerts
- **Regular review**: Periodically review and update alert rules

## 📚 **ADDITIONAL RESOURCES**

### Documentation
- [System Architecture](../architecture/TASK_23_6_SYSTEM_ARCHITECTURE.md)
- [API Reference](../api/semantic-event-api.md)
- [Deployment Guide](../operations/deployment-guide.md)
- [Performance Benchmarks](../performance/benchmarks.md)

### Examples
- [Complete Integration Test](../../examples/task_23_6_complete_integration_test.rs)
- [Phase-specific Examples](../../examples/)
- [Configuration Templates](../../config/templates/)

### Support
- GitHub Issues: [VexFS Issues](https://github.com/vexfs/vexfs/issues)
- Documentation: [VexFS Docs](https://docs.vexfs.org)
- Community: [VexFS Discord](https://discord.gg/vexfs)

---

*This usage guide provides comprehensive instructions for utilizing the Task 23.6 Semantic Event Propagation System to transform VexFS into an intelligent semantic computing platform.*