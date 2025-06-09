//! Comprehensive tests for Task 23.6 Phase 2: Semantic Event Propagation System
//!
//! This module provides extensive testing for the Kernel-FUSE Event Bridge,
//! Advanced Event Router, and Integration components.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use crate::semantic_api::{
    SemanticResult, SemanticError,
    types::*,
    event_propagation_manager::*,
    kernel_fuse_bridge::*,
    advanced_event_router::*,
    event_propagation_integration::*,
};

/// Test configuration for event propagation system
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub test_duration: Duration,
    pub event_count: usize,
    pub latency_target_ns: u64,
    pub throughput_target: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_duration: Duration::from_secs(10),
            event_count: 10000,
            latency_target_ns: 500, // Sub-500ns target
            throughput_target: 50000, // >50,000 events/sec
        }
    }
}

/// Test metrics collection
#[derive(Debug, Default)]
pub struct TestMetrics {
    pub events_processed: u64,
    pub total_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub errors: u64,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
}

impl TestMetrics {
    pub fn new() -> Self {
        Self {
            min_latency_ns: u64::MAX,
            ..Default::default()
        }
    }

    pub fn record_event(&mut self, latency_ns: u64) {
        self.events_processed += 1;
        self.total_latency_ns += latency_ns;
        self.min_latency_ns = self.min_latency_ns.min(latency_ns);
        self.max_latency_ns = self.max_latency_ns.max(latency_ns);
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn end(&mut self) {
        self.end_time = Some(Instant::now());
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.events_processed > 0 {
            self.total_latency_ns / self.events_processed
        } else {
            0
        }
    }

    pub fn throughput_per_sec(&self) -> f64 {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            let duration_secs = end.duration_since(start).as_secs_f64();
            if duration_secs > 0.0 {
                self.events_processed as f64 / duration_secs
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

/// Test suite for Kernel-FUSE Event Bridge
pub struct KernelFuseBridgeTestSuite {
    config: TestConfig,
    metrics: Arc<Mutex<TestMetrics>>,
}

impl KernelFuseBridgeTestSuite {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(Mutex::new(TestMetrics::new())),
        }
    }

    /// Test basic bridge initialization and configuration
    pub async fn test_bridge_initialization(&self) -> SemanticResult<()> {
        println!("Testing Kernel-FUSE Bridge initialization...");

        let config = KernelFuseBridgeConfig {
            ring_buffer_size: 1024 * 1024, // 1MB
            max_event_size: 4096,
            compression_enabled: true,
            checksum_enabled: true,
            timeout_ms: 1000,
        };

        let bridge = KernelFuseBridge::new(config)?;
        
        // Verify bridge is properly initialized
        assert!(bridge.is_initialized());
        
        // Test configuration retrieval
        let retrieved_config = bridge.get_config();
        assert_eq!(retrieved_config.ring_buffer_size, 1024 * 1024);
        assert!(retrieved_config.compression_enabled);
        assert!(retrieved_config.checksum_enabled);

        println!("✓ Bridge initialization test passed");
        Ok(())
    }

    /// Test shared memory ring buffer operations
    pub async fn test_ring_buffer_operations(&self) -> SemanticResult<()> {
        println!("Testing shared memory ring buffer operations...");

        let buffer = SharedMemoryRingBuffer::new(4096)?;
        
        // Test basic write/read operations
        let test_data = b"test_event_data_12345";
        let write_result = buffer.write(test_data)?;
        assert!(write_result);

        let mut read_buffer = vec![0u8; test_data.len()];
        let read_result = buffer.read(&mut read_buffer)?;
        assert!(read_result);
        assert_eq!(&read_buffer, test_data);

        // Test buffer overflow handling
        let large_data = vec![0u8; 8192]; // Larger than buffer
        let overflow_result = buffer.write(&large_data);
        assert!(overflow_result.is_err());

        println!("✓ Ring buffer operations test passed");
        Ok(())
    }

    /// Test event serialization and deserialization
    pub async fn test_event_serialization(&self) -> SemanticResult<()> {
        println!("Testing event serialization/deserialization...");

        let serializer = EventSerializer::new(true, true); // compression + checksum

        // Create test event
        let event = create_test_event(1, SemanticEventType::FilesystemWrite);
        
        // Test serialization
        let start = Instant::now();
        let serialized = serializer.serialize(&event)?;
        let serialize_time = start.elapsed();

        // Test deserialization
        let start = Instant::now();
        let deserialized = serializer.deserialize(&serialized)?;
        let deserialize_time = start.elapsed();

        // Verify event integrity
        assert_eq!(event.event_id, deserialized.event_id);
        assert_eq!(event.event_type, deserialized.event_type);
        assert_eq!(event.agent_id, deserialized.agent_id);

        // Check performance targets
        let total_time_ns = (serialize_time + deserialize_time).as_nanos() as u64;
        if total_time_ns > self.config.latency_target_ns {
            println!("⚠ Serialization latency {}ns exceeds target {}ns", 
                    total_time_ns, self.config.latency_target_ns);
        }

        println!("✓ Event serialization test passed ({}ns)", total_time_ns);
        Ok(())
    }

    /// Test cross-boundary event transfer latency
    pub async fn test_cross_boundary_latency(&self) -> SemanticResult<()> {
        println!("Testing cross-boundary event transfer latency...");

        let config = KernelFuseBridgeConfig {
            ring_buffer_size: 1024 * 1024,
            max_event_size: 4096,
            compression_enabled: true,
            checksum_enabled: true,
            timeout_ms: 1000,
        };

        let bridge = KernelFuseBridge::new(config)?;
        let mut metrics = self.metrics.lock().unwrap();
        metrics.start();
        drop(metrics);

        // Test multiple event transfers
        for i in 0..1000 {
            let event = create_test_event(i, SemanticEventType::FilesystemRead);
            
            let start = Instant::now();
            bridge.transfer_to_fuse(&event).await?;
            let latency_ns = start.elapsed().as_nanos() as u64;

            let mut metrics = self.metrics.lock().unwrap();
            metrics.record_event(latency_ns);
            
            if latency_ns > self.config.latency_target_ns {
                metrics.record_error();
            }
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.end();
        
        let avg_latency = metrics.avg_latency_ns();
        let error_rate = (metrics.errors as f64 / metrics.events_processed as f64) * 100.0;

        println!("✓ Cross-boundary latency test completed:");
        println!("  Average latency: {}ns (target: {}ns)", avg_latency, self.config.latency_target_ns);
        println!("  Min latency: {}ns", metrics.min_latency_ns);
        println!("  Max latency: {}ns", metrics.max_latency_ns);
        println!("  Error rate: {:.2}%", error_rate);

        if avg_latency <= self.config.latency_target_ns {
            println!("✓ Latency target achieved!");
        } else {
            println!("⚠ Latency target missed");
        }

        Ok(())
    }

    /// Test boundary manager functionality
    pub async fn test_boundary_managers(&self) -> SemanticResult<()> {
        println!("Testing boundary manager functionality...");

        // Test kernel boundary manager
        let kernel_manager = KernelBoundaryManager::new()?;
        assert!(kernel_manager.is_connected());

        // Test FUSE boundary manager
        let fuse_manager = FuseBoundaryManager::new()?;
        assert!(fuse_manager.is_connected());

        // Test event routing through managers
        let event = create_test_event(1, SemanticEventType::GraphNodeCreate);
        
        kernel_manager.send_event(&event).await?;
        let received_event = fuse_manager.receive_event().await?;
        
        assert_eq!(event.event_id, received_event.event_id);
        assert_eq!(event.event_type, received_event.event_type);

        println!("✓ Boundary managers test passed");
        Ok(())
    }

    /// Run all bridge tests
    pub async fn run_all_tests(&self) -> SemanticResult<()> {
        println!("=== Kernel-FUSE Bridge Test Suite ===");
        
        self.test_bridge_initialization().await?;
        self.test_ring_buffer_operations().await?;
        self.test_event_serialization().await?;
        self.test_cross_boundary_latency().await?;
        self.test_boundary_managers().await?;
        
        println!("=== All Bridge Tests Completed ===\n");
        Ok(())
    }
}

/// Test suite for Advanced Event Router
pub struct AdvancedEventRouterTestSuite {
    config: TestConfig,
    metrics: Arc<Mutex<TestMetrics>>,
}

impl AdvancedEventRouterTestSuite {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(Mutex::new(TestMetrics::new())),
        }
    }

    /// Test router initialization and configuration
    pub async fn test_router_initialization(&self) -> SemanticResult<()> {
        println!("Testing Advanced Event Router initialization...");

        let config = AdvancedEventRouterConfig {
            max_rules: 1000,
            default_qos_latency_ms: 10,
            default_qos_bandwidth_mbps: 100,
            topology_discovery_enabled: true,
            load_balancing_enabled: true,
            failover_enabled: true,
        };

        let router = AdvancedEventRouter::new(config)?;
        
        // Verify router is properly initialized
        assert!(router.is_initialized());
        
        // Test configuration retrieval
        let retrieved_config = router.get_config();
        assert_eq!(retrieved_config.max_rules, 1000);
        assert!(retrieved_config.topology_discovery_enabled);

        println!("✓ Router initialization test passed");
        Ok(())
    }

    /// Test pattern-based routing rules
    pub async fn test_pattern_based_routing(&self) -> SemanticResult<()> {
        println!("Testing pattern-based routing rules...");

        let config = AdvancedEventRouterConfig::default();
        let mut router = AdvancedEventRouter::new(config)?;

        // Add routing rules
        let filesystem_rule = RoutingRule {
            id: "filesystem_rule".to_string(),
            priority: 100,
            conditions: vec![
                RoutingCondition::EventTypePattern("Filesystem.*".to_string()),
                RoutingCondition::AgentPattern("agent_.*".to_string()),
            ],
            actions: vec![
                RoutingAction::RouteToDestination("filesystem_handler".to_string()),
                RoutingAction::SetQoSLatency(5),
            ],
            enabled: true,
        };

        router.add_rule(filesystem_rule)?;

        // Test rule matching
        let fs_event = create_test_event(1, SemanticEventType::FilesystemWrite);
        let matches = router.find_matching_rules(&fs_event)?;
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "filesystem_rule");

        // Test non-matching event
        let graph_event = create_test_event(2, SemanticEventType::GraphNodeCreate);
        let no_matches = router.find_matching_rules(&graph_event)?;
        assert_eq!(no_matches.len(), 0);

        println!("✓ Pattern-based routing test passed");
        Ok(())
    }

    /// Test QoS-aware routing
    pub async fn test_qos_aware_routing(&self) -> SemanticResult<()> {
        println!("Testing QoS-aware routing...");

        let config = AdvancedEventRouterConfig::default();
        let mut router = AdvancedEventRouter::new(config)?;

        // Add QoS-aware rules
        let high_priority_rule = RoutingRule {
            id: "high_priority".to_string(),
            priority: 200,
            conditions: vec![
                RoutingCondition::EventPriority(EventPriority::Critical),
            ],
            actions: vec![
                RoutingAction::SetQoSLatency(1), // 1ms latency requirement
                RoutingAction::SetQoSBandwidth(1000), // 1Gbps bandwidth
            ],
            enabled: true,
        };

        router.add_rule(high_priority_rule)?;

        // Test QoS metrics
        let critical_event = SemanticEvent {
            event_id: 1,
            event_type: SemanticEventType::SystemMount,
            agent_id: "system".to_string(),
            timestamp: SemanticTimestamp {
                seconds: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                nanoseconds: 0,
            },
            flags: EventFlags {
                priority: EventPriority::Critical,
                requires_ack: true,
                encrypted: false,
                compressed: false,
            },
            context: SemanticContext::default(),
        };

        let qos_metrics = router.calculate_qos_metrics(&critical_event)?;
        assert_eq!(qos_metrics.latency_requirement_ms, 1);
        assert_eq!(qos_metrics.bandwidth_requirement_mbps, 1000);

        println!("✓ QoS-aware routing test passed");
        Ok(())
    }

    /// Test topology-aware routing
    pub async fn test_topology_aware_routing(&self) -> SemanticResult<()> {
        println!("Testing topology-aware routing...");

        let config = AdvancedEventRouterConfig {
            topology_discovery_enabled: true,
            ..Default::default()
        };
        let mut router = AdvancedEventRouter::new(config)?;

        // Simulate topology discovery
        router.discover_topology().await?;
        let topology = router.get_topology();
        
        // Verify topology contains expected nodes
        assert!(topology.nodes.len() > 0);
        
        // Test distance-based routing
        let event = create_test_event(1, SemanticEventType::VectorSearch);
        let optimal_path = router.find_optimal_path(&event, "target_node")?;
        
        assert!(optimal_path.len() > 0);
        assert!(optimal_path.total_latency_ms > 0);

        println!("✓ Topology-aware routing test passed");
        Ok(())
    }

    /// Test load balancing strategies
    pub async fn test_load_balancing(&self) -> SemanticResult<()> {
        println!("Testing load balancing strategies...");

        let config = AdvancedEventRouterConfig {
            load_balancing_enabled: true,
            ..Default::default()
        };
        let mut router = AdvancedEventRouter::new(config)?;

        // Add multiple destinations
        let destinations = vec!["dest1", "dest2", "dest3"];
        for dest in &destinations {
            router.add_destination(dest.to_string())?;
        }

        // Test round-robin load balancing
        router.set_load_balancing_strategy(LoadBalancingStrategy::RoundRobin)?;
        
        let mut destination_counts = HashMap::new();
        for i in 0..30 {
            let event = create_test_event(i, SemanticEventType::FilesystemRead);
            let selected_dest = router.select_destination(&event)?;
            *destination_counts.entry(selected_dest).or_insert(0) += 1;
        }

        // Verify roughly equal distribution
        for dest in &destinations {
            let count = destination_counts.get(dest).unwrap_or(&0);
            assert!(*count >= 8 && *count <= 12); // Allow some variance
        }

        println!("✓ Load balancing test passed");
        Ok(())
    }

    /// Test failover mechanisms
    pub async fn test_failover_mechanisms(&self) -> SemanticResult<()> {
        println!("Testing failover mechanisms...");

        let config = AdvancedEventRouterConfig {
            failover_enabled: true,
            ..Default::default()
        };
        let mut router = AdvancedEventRouter::new(config)?;

        // Add primary and backup destinations
        router.add_destination("primary".to_string())?;
        router.add_destination("backup".to_string())?;
        router.set_backup_destination("primary", "backup")?;

        // Simulate primary failure
        router.mark_destination_failed("primary").await?;
        
        // Test event routing to backup
        let event = create_test_event(1, SemanticEventType::AgentQuery);
        let selected_dest = router.select_destination(&event)?;
        assert_eq!(selected_dest, "backup");

        // Test primary recovery
        router.mark_destination_recovered("primary").await?;
        let selected_dest_after_recovery = router.select_destination(&event)?;
        assert_eq!(selected_dest_after_recovery, "primary");

        println!("✓ Failover mechanisms test passed");
        Ok(())
    }

    /// Test routing performance
    pub async fn test_routing_performance(&self) -> SemanticResult<()> {
        println!("Testing routing performance...");

        let config = AdvancedEventRouterConfig::default();
        let mut router = AdvancedEventRouter::new(config)?;

        // Add multiple complex rules
        for i in 0..100 {
            let rule = RoutingRule {
                id: format!("rule_{}", i),
                priority: i,
                conditions: vec![
                    RoutingCondition::EventTypePattern(format!(".*{}", i % 10)),
                    RoutingCondition::AgentPattern("agent_.*".to_string()),
                ],
                actions: vec![
                    RoutingAction::RouteToDestination(format!("dest_{}", i % 5)),
                ],
                enabled: true,
            };
            router.add_rule(rule)?;
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.start();
        drop(metrics);

        // Test routing performance
        for i in 0..self.config.event_count {
            let event = create_test_event(i as u64, SemanticEventType::FilesystemWrite);
            
            let start = Instant::now();
            let _matches = router.find_matching_rules(&event)?;
            let latency_ns = start.elapsed().as_nanos() as u64;

            let mut metrics = self.metrics.lock().unwrap();
            metrics.record_event(latency_ns);
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.end();
        
        let avg_latency = metrics.avg_latency_ns();
        let throughput = metrics.throughput_per_sec();

        println!("✓ Routing performance test completed:");
        println!("  Average routing latency: {}ns", avg_latency);
        println!("  Routing throughput: {:.0} events/sec", throughput);

        Ok(())
    }

    /// Run all router tests
    pub async fn run_all_tests(&self) -> SemanticResult<()> {
        println!("=== Advanced Event Router Test Suite ===");
        
        self.test_router_initialization().await?;
        self.test_pattern_based_routing().await?;
        self.test_qos_aware_routing().await?;
        self.test_topology_aware_routing().await?;
        self.test_load_balancing().await?;
        self.test_failover_mechanisms().await?;
        self.test_routing_performance().await?;
        
        println!("=== All Router Tests Completed ===\n");
        Ok(())
    }
}

/// Test suite for Integrated Event Propagation System
pub struct IntegratedSystemTestSuite {
    config: TestConfig,
    metrics: Arc<Mutex<TestMetrics>>,
}

impl IntegratedSystemTestSuite {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(Mutex::new(TestMetrics::new())),
        }
    }

    /// Test integrated system initialization
    pub async fn test_system_initialization(&self) -> SemanticResult<()> {
        println!("Testing integrated system initialization...");

        let config = IntegratedSystemConfig {
            bridge_config: KernelFuseBridgeConfig::default(),
            router_config: AdvancedEventRouterConfig::default(),
            performance_monitoring_enabled: true,
            adaptive_optimization_enabled: true,
            max_pipeline_depth: 1000,
        };

        let system = IntegratedEventPropagationSystem::new(config)?;
        
        // Verify system is properly initialized
        assert!(system.is_initialized());
        
        // Test system status
        let status = system.get_system_status();
        assert_eq!(status.overall_health, SystemHealth::Healthy);

        println!("✓ Integrated system initialization test passed");
        Ok(())
    }

    /// Test end-to-end event propagation
    pub async fn test_end_to_end_propagation(&self) -> SemanticResult<()> {
        println!("Testing end-to-end event propagation...");

        let config = IntegratedSystemConfig::default();
        let mut system = IntegratedEventPropagationSystem::new(config)?;

        // Start the system
        system.start().await?;

        let mut metrics = self.metrics.lock().unwrap();
        metrics.start();
        drop(metrics);

        // Test event propagation pipeline
        for i in 0..1000 {
            let event = create_test_event(i, SemanticEventType::FilesystemWrite);
            
            let start = Instant::now();
            system.propagate_event(event).await?;
            let latency_ns = start.elapsed().as_nanos() as u64;

            let mut metrics = self.metrics.lock().unwrap();
            metrics.record_event(latency_ns);
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.end();
        
        let avg_latency = metrics.avg_latency_ns();
        let throughput = metrics.throughput_per_sec();

        println!("✓ End-to-end propagation test completed:");
        println!("  Average latency: {}ns (target: {}ns)", avg_latency, self.config.latency_target_ns);
        println!("  Throughput: {:.0} events/sec (target: {})", throughput, self.config.throughput_target);

        // Stop the system
        system.stop().await?;

        Ok(())
    }

    /// Test performance monitoring
    pub async fn test_performance_monitoring(&self) -> SemanticResult<()> {
        println!("Testing performance monitoring...");

        let config = IntegratedSystemConfig {
            performance_monitoring_enabled: true,
            ..Default::default()
        };
        let mut system = IntegratedEventPropagationSystem::new(config)?;

        system.start().await?;

        // Generate events to monitor
        for i in 0..100 {
            let event = create_test_event(i, SemanticEventType::VectorSearch);
            system.propagate_event(event).await?;
        }

        // Check performance metrics
        let metrics = system.get_performance_metrics();
        assert!(metrics.total_events_processed > 0);
        assert!(metrics.average_latency_ns > 0);
        assert!(metrics.throughput_events_per_sec > 0.0);

        system.stop().await?;

        println!("✓ Performance monitoring test passed");
        Ok(())
    }

    /// Test adaptive optimization
    pub async fn test_adaptive_optimization(&self) -> SemanticResult<()> {
        println!("Testing adaptive optimization...");

        let config = IntegratedSystemConfig {
            adaptive_optimization_enabled: true,
            ..Default::default()
        };
        let mut system = IntegratedEventPropagationSystem::new(config)?;

        system.start().await?;

        // Simulate high load to trigger optimization
        for i in 0..1000 {
            let event = create_test_event(i, SemanticEventType::GraphEdgeCreate);
            system.propagate_event(event).await?;
        }

        // Check if optimizations were applied
        let optimizer_status = system.get_optimizer_status();
        assert!(optimizer_status.optimizations_applied > 0);

        system.stop().await?;

        println!("✓ Adaptive optimization test passed");
        Ok(())
    }

    /// Test system resilience under load
    pub async fn test_system_resilience(&self) -> SemanticResult<()> {
        println!("Testing system resilience under load...");

        let config = IntegratedSystemConfig::default();
        let mut system = IntegratedEventPropagationSystem::new(config)?;

        system.start().await?;

        // Simulate high concurrent load
        let mut handles = Vec::new();
        for thread_id in 0..10 {
            let system_clone = system.clone();
            let handle = tokio::spawn(async move {
                for i in 0..100 {
                    let event = create_test_event(
                        (thread_id * 100 + i) as u64, 
                        SemanticEventType::ObservabilityMetricCollected
                    );
                    let _ = system_clone.propagate_event(event).await;
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.await.map_err(|e| SemanticError::internal(format!("Thread join error: {}", e)))?;
        }

        // Verify system is still healthy
        let status = system.get_system_status();
        assert_eq!(status.overall_health, SystemHealth::Healthy);

        system.stop().await?;

        println!("✓ System resilience test passed");
        Ok(())
    }

    /// Run all integrated system tests
    pub async fn run_all_tests(&self) -> SemanticResult<()> {
        println!("=== Integrated Event Propagation System Test Suite ===");
        
        self.test_system_initialization().await?;
        self.test_end_to_end_propagation().await?;
        self.test_performance_monitoring().await?;
        self.test_adaptive_optimization().await?;
        self.test_system_resilience().await?;
        
        println!("=== All Integrated System Tests Completed ===\n");
        Ok(())
    }
}

/// Helper function to create test events
fn create_test_event(id: u64, event_type: SemanticEventType) -> SemanticEvent {
    SemanticEvent {
        event_id: id,
        event_type,
        agent_id: format!("test_agent_{}", id % 10),
        timestamp: SemanticTimestamp {
            seconds: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            nanoseconds: 0,
        },
        flags: EventFlags {
            priority: match id % 3 {
                0 => EventPriority::Low,
                1 => EventPriority::Normal,
                _ => EventPriority::High,
            },
            requires_ack: id % 2 == 0,
            encrypted: false,
            compressed: true,
        },
        context: SemanticContext::default(),
    }
}

/// Main test runner for Task 23.6 Phase 2
pub async fn run_task_23_6_phase_2_tests() -> SemanticResult<()> {
    println!("🚀 Starting Task 23.6 Phase 2: Semantic Event Propagation System Tests");
    println!("================================================================");

    let test_config = TestConfig::default();

    // Run Kernel-FUSE Bridge tests
    let bridge_suite = KernelFuseBridgeTestSuite::new(test_config.clone());
    bridge_suite.run_all_tests().await?;

    // Run Advanced Event Router tests
    let router_suite = AdvancedEventRouterTestSuite::new(test_config.clone());
    router_suite.run_all_tests().await?;

    // Run Integrated System tests
    let integration_suite = IntegratedSystemTestSuite::new(test_config.clone());
    integration_suite.run_all_tests().await?;

    println!("🎉 All Task 23.6 Phase 2 tests completed successfully!");
    println!("================================================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kernel_fuse_bridge_basic() {
        let config = TestConfig {
            event_count: 100,
            ..Default::default()
        };
        let suite = KernelFuseBridgeTestSuite::new(config);
        suite.test_bridge_initialization().await.unwrap();
    }

    #[tokio::test]
    async fn test_advanced_router_basic() {
        let config = TestConfig {
            event_count: 100,
            ..Default::default()
        };
        let suite = AdvancedEventRouterTestSuite::new(config);
        suite.test_router_initialization().await.unwrap();
    }

    #[tokio::test]
    async fn test_integrated_system_basic() {
        let config = TestConfig {
            event_count: 100,
            ..Default::default()
        };
        let suite = IntegratedSystemTestSuite::new(config);
        suite.test_system_initialization().await.unwrap();
    }
}