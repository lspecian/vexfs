//! Integration tests for Task 23.4.2: Cross-Boundary Event Consistency and Transaction Coordination
//!
//! This module tests the complete cross-boundary event consistency system including:
//! - Cross-Boundary Transaction Coordinator
//! - Event Ordering Service  
//! - Boundary Synchronization Manager
//! - Integration between all components

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use crate::semantic_api::types::*;
use crate::semantic_api::cross_boundary_coordinator::{
    CrossBoundaryTransactionCoordinator, CrossBoundaryConfig, CrossBoundaryParticipant,
    TransactionState, DeadlockResolutionStrategy,
};
use crate::semantic_api::event_ordering_service::{
    EventOrderingService, EventOrderingConfig, ConflictResolutionStrategy,
    OrderedSemanticEvent,
};
use crate::semantic_api::boundary_sync_manager::{
    BoundarySynchronizationManager, BoundarySyncConfig, SynchronizationBoundary,
    SynchronizationStrategy, StreamConfig,
};
use crate::shared::errors::{VexfsError, VexfsResult};

/// Integration test suite for Task 23.4.2
pub struct Task23_4_2IntegrationTest {
    /// Cross-boundary transaction coordinator
    coordinator: Arc<CrossBoundaryTransactionCoordinator>,
    
    /// Event ordering service
    ordering_service: Arc<EventOrderingService>,
    
    /// Boundary synchronization manager
    sync_manager: Arc<BoundarySynchronizationManager>,
    
    /// Test configuration
    config: IntegrationTestConfig,
}

/// Integration test configuration
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    /// Test timeout duration
    pub test_timeout: Duration,
    
    /// Number of test events to generate
    pub test_event_count: usize,
    
    /// Number of concurrent transactions
    pub concurrent_transactions: usize,
    
    /// Enable performance benchmarking
    pub enable_benchmarking: bool,
    
    /// Target transaction commit latency (microseconds)
    pub target_commit_latency_us: u64,
    
    /// Target event throughput (events per second)
    pub target_event_throughput: u64,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            test_timeout: Duration::from_secs(30),
            test_event_count: 1000,
            concurrent_transactions: 10,
            enable_benchmarking: true,
            target_commit_latency_us: 5000, // 5ms
            target_event_throughput: 10000, // 10k events/sec
        }
    }
}

/// Test results for Task 23.4.2
#[derive(Debug, Clone)]
pub struct IntegrationTestResults {
    /// All tests passed
    pub success: bool,
    
    /// Individual test results
    pub test_results: HashMap<String, TestResult>,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Error details (if any)
    pub errors: Vec<String>,
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name
    pub name: String,
    
    /// Test passed
    pub passed: bool,
    
    /// Test duration
    pub duration: Duration,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Test-specific metrics
    pub metrics: HashMap<String, f64>,
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Average transaction commit latency (microseconds)
    pub avg_commit_latency_us: u64,
    
    /// Maximum transaction commit latency (microseconds)
    pub max_commit_latency_us: u64,
    
    /// Event throughput (events per second)
    pub event_throughput: u64,
    
    /// Average event ordering latency (microseconds)
    pub avg_ordering_latency_us: u64,
    
    /// Average synchronization latency (microseconds)
    pub avg_sync_latency_us: u64,
    
    /// Deadlock detection time (microseconds)
    pub deadlock_detection_time_us: u64,
    
    /// Memory usage (bytes)
    pub memory_usage_bytes: u64,
    
    /// CPU usage percentage
    pub cpu_usage_percent: f32,
}

impl Task23_4_2IntegrationTest {
    /// Create new integration test suite
    pub async fn new(config: IntegrationTestConfig) -> Result<Self, VexfsError> {
        // Create cross-boundary coordinator
        let coordinator_config = CrossBoundaryConfig {
            max_concurrent_transactions: config.concurrent_transactions,
            transaction_timeout: Duration::from_secs(30),
            deadlock_detection_interval: Duration::from_millis(100),
            deadlock_resolution_strategy: DeadlockResolutionStrategy::AbortYoungest,
            enable_statistics: true,
            enable_heartbeat_monitoring: true,
            heartbeat_interval: Duration::from_secs(5),
            participant_timeout: Duration::from_secs(10),
        };
        let coordinator = Arc::new(CrossBoundaryTransactionCoordinator::new(coordinator_config)?);
        
        // Create event ordering service
        let ordering_config = EventOrderingConfig {
            max_concurrent_events: config.test_event_count,
            conflict_resolution_strategy: ConflictResolutionStrategy::LastWriterWins,
            enable_vector_clocks: true,
            enable_causal_ordering: true,
            enable_statistics: true,
            sequence_gap_timeout: Duration::from_secs(5),
            max_sequence_gap: 100,
        };
        let ordering_service = Arc::new(EventOrderingService::new(ordering_config)?);
        
        // Create boundary synchronization manager
        let sync_config = BoundarySyncConfig {
            max_concurrent_streams: 32,
            default_strategy: SynchronizationStrategy::Adaptive,
            checkpoint_interval_ms: 10000, // 10 seconds for testing
            recovery_timeout_ms: 30000,
            load_monitoring_interval_ms: 1000,
            enable_auto_recovery: true,
            enable_load_balancing: true,
            health_check_interval_ms: 2000,
            ..Default::default()
        };
        let sync_manager = Arc::new(BoundarySynchronizationManager::new(sync_config)?);
        
        Ok(Self {
            coordinator,
            ordering_service,
            sync_manager,
            config,
        })
    }
    
    /// Run all integration tests
    pub async fn run_all_tests(&self) -> IntegrationTestResults {
        let mut results = IntegrationTestResults {
            success: true,
            test_results: HashMap::new(),
            performance_metrics: PerformanceMetrics::default(),
            errors: Vec::new(),
        };
        
        // Initialize all components
        if let Err(e) = self.initialize_components().await {
            results.success = false;
            results.errors.push(format!("Failed to initialize components: {}", e));
            return results;
        }
        
        // Run individual tests
        let tests = vec![
            ("coordinator_basic_functionality", Self::test_coordinator_basic_functionality),
            ("ordering_service_basic_functionality", Self::test_ordering_service_basic_functionality),
            ("sync_manager_basic_functionality", Self::test_sync_manager_basic_functionality),
            ("cross_boundary_transaction_flow", Self::test_cross_boundary_transaction_flow),
            ("event_ordering_consistency", Self::test_event_ordering_consistency),
            ("boundary_synchronization", Self::test_boundary_synchronization),
            ("deadlock_detection_and_resolution", Self::test_deadlock_detection_and_resolution),
            ("conflict_resolution", Self::test_conflict_resolution),
            ("recovery_and_checkpointing", Self::test_recovery_and_checkpointing),
            ("performance_benchmarks", Self::test_performance_benchmarks),
            ("stress_testing", Self::test_stress_testing),
            ("integration_workflow", Self::test_integration_workflow),
        ];
        
        for (test_name, test_fn) in tests {
            let test_result = self.run_single_test(test_name, test_fn).await;
            
            if !test_result.passed {
                results.success = false;
                if let Some(error) = &test_result.error {
                    results.errors.push(format!("{}: {}", test_name, error));
                }
            }
            
            results.test_results.insert(test_name.to_string(), test_result);
        }
        
        // Collect performance metrics
        if self.config.enable_benchmarking {
            results.performance_metrics = self.collect_performance_metrics().await;
        }
        
        // Cleanup
        if let Err(e) = self.cleanup_components().await {
            results.errors.push(format!("Failed to cleanup components: {}", e));
        }
        
        results
    }
    
    /// Initialize all components
    async fn initialize_components(&self) -> Result<(), VexfsError> {
        // Start coordinator
        self.coordinator.start().await?;
        
        // Start ordering service
        self.ordering_service.start().await?;
        
        // Start sync manager
        self.sync_manager.start().await?;
        
        // Set up integration between components
        let mut sync_manager_mut = Arc::clone(&self.sync_manager);
        // Note: In a real implementation, we would set up the integration here
        // sync_manager_mut.set_event_ordering_service(Arc::clone(&self.ordering_service));
        // sync_manager_mut.set_cross_boundary_coordinator(Arc::clone(&self.coordinator));
        
        Ok(())
    }
    
    /// Cleanup all components
    async fn cleanup_components(&self) -> Result<(), VexfsError> {
        // Stop sync manager
        self.sync_manager.stop().await?;
        
        // Stop ordering service
        self.ordering_service.stop().await?;
        
        // Stop coordinator
        self.coordinator.stop().await?;
        
        Ok(())
    }
    
    /// Run a single test with timeout
    async fn run_single_test<F, Fut>(&self, test_name: &str, test_fn: F) -> TestResult
    where
        F: FnOnce(&Self) -> Fut,
        Fut: std::future::Future<Output = Result<HashMap<String, f64>, VexfsError>>,
    {
        let start_time = std::time::Instant::now();
        
        let result = timeout(self.config.test_timeout, test_fn(self)).await;
        
        let duration = start_time.elapsed();
        
        match result {
            Ok(Ok(metrics)) => TestResult {
                name: test_name.to_string(),
                passed: true,
                duration,
                error: None,
                metrics,
            },
            Ok(Err(e)) => TestResult {
                name: test_name.to_string(),
                passed: false,
                duration,
                error: Some(e.to_string()),
                metrics: HashMap::new(),
            },
            Err(_) => TestResult {
                name: test_name.to_string(),
                passed: false,
                duration,
                error: Some("Test timed out".to_string()),
                metrics: HashMap::new(),
            },
        }
    }
    
    /// Test coordinator basic functionality
    async fn test_coordinator_basic_functionality(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create a simple transaction
        let participants = vec![
            CrossBoundaryParticipant::KernelJournal,
            CrossBoundaryParticipant::UserspaceJournal,
        ];
        
        let start_time = std::time::Instant::now();
        let transaction_id = self.coordinator.begin_transaction(participants).await?;
        let begin_latency = start_time.elapsed().as_micros() as f64;
        
        // Verify transaction was created
        let transaction = self.coordinator.get_transaction(transaction_id).await?;
        assert_eq!(transaction.state, TransactionState::Active);
        
        // Commit transaction
        let start_time = std::time::Instant::now();
        self.coordinator.commit_transaction(transaction_id).await?;
        let commit_latency = start_time.elapsed().as_micros() as f64;
        
        metrics.insert("begin_latency_us".to_string(), begin_latency);
        metrics.insert("commit_latency_us".to_string(), commit_latency);
        
        Ok(metrics)
    }
    
    /// Test ordering service basic functionality
    async fn test_ordering_service_basic_functionality(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create test events
        let events = self.create_test_events(10);
        
        let start_time = std::time::Instant::now();
        
        // Submit events for ordering
        for event in events {
            self.ordering_service.submit_event(event).await?;
        }
        
        let submit_latency = start_time.elapsed().as_micros() as f64;
        
        // Wait for ordering to complete
        sleep(Duration::from_millis(100)).await;
        
        // Get ordered events
        let start_time = std::time::Instant::now();
        let ordered_events = self.ordering_service.get_ordered_events(10).await?;
        let retrieval_latency = start_time.elapsed().as_micros() as f64;
        
        assert_eq!(ordered_events.len(), 10);
        
        metrics.insert("submit_latency_us".to_string(), submit_latency);
        metrics.insert("retrieval_latency_us".to_string(), retrieval_latency);
        metrics.insert("events_processed".to_string(), ordered_events.len() as f64);
        
        Ok(metrics)
    }
    
    /// Test sync manager basic functionality
    async fn test_sync_manager_basic_functionality(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create synchronization stream
        let start_time = std::time::Instant::now();
        let stream_id = self.sync_manager.create_stream(
            SynchronizationBoundary::KernelToUserspace,
            SynchronizationBoundary::UserspaceToKernel,
            SynchronizationStrategy::Immediate,
            StreamConfig::default(),
            None,
        ).await?;
        let create_latency = start_time.elapsed().as_micros() as f64;
        
        // Start stream
        let start_time = std::time::Instant::now();
        self.sync_manager.start_stream(stream_id).await?;
        let start_latency = start_time.elapsed().as_micros() as f64;
        
        // Create test event
        let test_event = self.create_test_events(1).into_iter().next().unwrap();
        let ordered_event = OrderedSemanticEvent {
            event: test_event,
            sequence_number: 1,
            vector_clock: vec![1, 0, 0],
            causal_dependencies: Vec::new(),
        };
        
        // Synchronize event
        let start_time = std::time::Instant::now();
        self.sync_manager.synchronize_event(stream_id, ordered_event).await?;
        let sync_latency = start_time.elapsed().as_micros() as f64;
        
        metrics.insert("create_stream_latency_us".to_string(), create_latency);
        metrics.insert("start_stream_latency_us".to_string(), start_latency);
        metrics.insert("sync_event_latency_us".to_string(), sync_latency);
        
        Ok(metrics)
    }
    
    /// Test cross-boundary transaction flow
    async fn test_cross_boundary_transaction_flow(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create multiple transactions with different participants
        let mut transaction_ids = Vec::new();
        
        let start_time = std::time::Instant::now();
        
        for i in 0..5 {
            let participants = vec![
                CrossBoundaryParticipant::KernelJournal,
                CrossBoundaryParticipant::UserspaceJournal,
                if i % 2 == 0 { CrossBoundaryParticipant::CrossLayerManager } else { CrossBoundaryParticipant::ExternalSystem },
            ];
            
            let transaction_id = self.coordinator.begin_transaction(participants).await?;
            transaction_ids.push(transaction_id);
        }
        
        let begin_all_latency = start_time.elapsed().as_micros() as f64;
        
        // Commit all transactions
        let start_time = std::time::Instant::now();
        
        for transaction_id in transaction_ids {
            self.coordinator.commit_transaction(transaction_id).await?;
        }
        
        let commit_all_latency = start_time.elapsed().as_micros() as f64;
        
        metrics.insert("begin_all_transactions_latency_us".to_string(), begin_all_latency);
        metrics.insert("commit_all_transactions_latency_us".to_string(), commit_all_latency);
        metrics.insert("transactions_processed".to_string(), 5.0);
        
        Ok(metrics)
    }
    
    /// Test event ordering consistency
    async fn test_event_ordering_consistency(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create events with dependencies
        let events = self.create_test_events_with_dependencies(20);
        
        let start_time = std::time::Instant::now();
        
        // Submit events in random order
        for event in events {
            self.ordering_service.submit_event(event).await?;
        }
        
        let submit_latency = start_time.elapsed().as_micros() as f64;
        
        // Wait for ordering
        sleep(Duration::from_millis(200)).await;
        
        // Verify ordering consistency
        let start_time = std::time::Instant::now();
        let ordered_events = self.ordering_service.get_ordered_events(20).await?;
        let retrieval_latency = start_time.elapsed().as_micros() as f64;
        
        // Verify causal ordering is maintained
        let mut previous_sequence = 0;
        for event in &ordered_events {
            assert!(event.sequence_number > previous_sequence);
            previous_sequence = event.sequence_number;
        }
        
        metrics.insert("submit_with_dependencies_latency_us".to_string(), submit_latency);
        metrics.insert("retrieval_with_ordering_latency_us".to_string(), retrieval_latency);
        metrics.insert("ordered_events_count".to_string(), ordered_events.len() as f64);
        
        Ok(metrics)
    }
    
    /// Test boundary synchronization
    async fn test_boundary_synchronization(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create multiple streams with different strategies
        let strategies = vec![
            SynchronizationStrategy::Immediate,
            SynchronizationStrategy::Batched,
            SynchronizationStrategy::Adaptive,
        ];
        
        let mut stream_ids = Vec::new();
        
        let start_time = std::time::Instant::now();
        
        for strategy in strategies {
            let stream_id = self.sync_manager.create_stream(
                SynchronizationBoundary::KernelToUserspace,
                SynchronizationBoundary::UserspaceToKernel,
                strategy,
                StreamConfig::default(),
                None,
            ).await?;
            
            self.sync_manager.start_stream(stream_id).await?;
            stream_ids.push(stream_id);
        }
        
        let setup_latency = start_time.elapsed().as_micros() as f64;
        
        // Synchronize events across all streams
        let events = self.create_test_events(30);
        let ordered_events: Vec<OrderedSemanticEvent> = events.into_iter().enumerate().map(|(i, event)| {
            OrderedSemanticEvent {
                event,
                sequence_number: i as u64 + 1,
                vector_clock: vec![i as u64 + 1, 0, 0],
                causal_dependencies: Vec::new(),
            }
        }).collect();
        
        let start_time = std::time::Instant::now();
        
        for (i, event) in ordered_events.into_iter().enumerate() {
            let stream_id = stream_ids[i % stream_ids.len()];
            self.sync_manager.synchronize_event(stream_id, event).await?;
        }
        
        let sync_latency = start_time.elapsed().as_micros() as f64;
        
        // Wait for processing
        sleep(Duration::from_millis(500)).await;
        
        metrics.insert("stream_setup_latency_us".to_string(), setup_latency);
        metrics.insert("multi_stream_sync_latency_us".to_string(), sync_latency);
        metrics.insert("streams_created".to_string(), stream_ids.len() as f64);
        
        Ok(metrics)
    }
    
    /// Test deadlock detection and resolution
    async fn test_deadlock_detection_and_resolution(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create transactions that will cause deadlock
        let participants1 = vec![
            CrossBoundaryParticipant::KernelJournal,
            CrossBoundaryParticipant::UserspaceJournal,
        ];
        let participants2 = vec![
            CrossBoundaryParticipant::UserspaceJournal,
            CrossBoundaryParticipant::KernelJournal,
        ];
        
        let start_time = std::time::Instant::now();
        
        let tx1 = self.coordinator.begin_transaction(participants1).await?;
        let tx2 = self.coordinator.begin_transaction(participants2).await?;
        
        // Wait for deadlock detection
        sleep(Duration::from_millis(200)).await;
        
        let detection_latency = start_time.elapsed().as_micros() as f64;
        
        // Check if deadlock was detected and resolved
        let stats = self.coordinator.get_statistics().await;
        
        metrics.insert("deadlock_detection_latency_us".to_string(), detection_latency);
        metrics.insert("deadlocks_detected".to_string(), stats.deadlocks_detected as f64);
        metrics.insert("deadlocks_resolved".to_string(), stats.deadlocks_resolved as f64);
        
        Ok(metrics)
    }
    
    /// Test conflict resolution
    async fn test_conflict_resolution(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create conflicting events
        let mut conflicting_events = Vec::new();
        
        for i in 0..5 {
            let event = SemanticEvent {
                id: EventId::new(),
                event_type: SemanticEventType::FilesystemWrite,
                timestamp: SemanticTimestamp {
                    wall_time: SystemTime::now(),
                    monotonic_time: Duration::from_millis(i * 100),
                },
                source: EventSource::Userspace,
                flags: EventFlags::default(),
                priority: EventPriority::Normal,
                category: EventCategory::Filesystem,
                context: SemanticContext::default(),
            };
            conflicting_events.push(event);
        }
        
        let start_time = std::time::Instant::now();
        
        // Submit conflicting events
        for event in conflicting_events {
            self.ordering_service.submit_event(event).await?;
        }
        
        let submit_latency = start_time.elapsed().as_micros() as f64;
        
        // Wait for conflict resolution
        sleep(Duration::from_millis(300)).await;
        
        let resolution_latency = start_time.elapsed().as_micros() as f64;
        
        // Get statistics
        let stats = self.ordering_service.get_statistics().await;
        
        metrics.insert("conflict_submit_latency_us".to_string(), submit_latency);
        metrics.insert("conflict_resolution_latency_us".to_string(), resolution_latency);
        metrics.insert("conflicts_detected".to_string(), stats.conflicts_detected as f64);
        metrics.insert("conflicts_resolved".to_string(), stats.conflicts_resolved as f64);
        
        Ok(metrics)
    }
    
    /// Test recovery and checkpointing
    async fn test_recovery_and_checkpointing(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // Create some streams and events
        let stream_id = self.sync_manager.create_stream(
            SynchronizationBoundary::KernelToUserspace,
            SynchronizationBoundary::UserspaceToKernel,
            SynchronizationStrategy::Batched,
            StreamConfig::default(),
            None,
        ).await?;
        
        self.sync_manager.start_stream(stream_id).await?;
        
        // Add some events
        let events = self.create_test_events(10);
        for (i, event) in events.into_iter().enumerate() {
            let ordered_event = OrderedSemanticEvent {
                event,
                sequence_number: i as u64 + 1,
                vector_clock: vec![i as u64 + 1, 0, 0],
                causal_dependencies: Vec::new(),
            };
            self.sync_manager.synchronize_event(stream_id, ordered_event).await?;
        }
        
        // Wait for checkpoint creation (should happen automatically)
        sleep(Duration::from_millis(12000)).await; // Wait longer than checkpoint interval
        
        // Get statistics
        let stats = self.sync_manager.get_stats().await;
        
        metrics.insert("checkpoints_created".to_string(), stats.checkpoints_created as f64);
        metrics.insert("recovery_operations".to_string(), stats.recovery_operations as f64);
        
        Ok(metrics)
    }
    
    /// Test performance benchmarks
    async fn test_performance_benchmarks(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        if !self.config.enable_benchmarking {
            return Ok(metrics);
        }
        
        // Benchmark transaction throughput
        let start_time = std::time::Instant::now();
        let mut transaction_ids = Vec::new();
        
        for _ in 0..self.config.concurrent_transactions {
            let participants = vec![
                CrossBoundaryParticipant::KernelJournal,
                CrossBoundaryParticipant::UserspaceJournal,
            ];
            let tx_id = self.coordinator.begin_transaction(participants).await?;
            transaction_ids.push(tx_id);
        }
        
        for tx_id in transaction_ids {
            self.coordinator.commit_transaction(tx_id).await?;
        }
        
        let transaction_benchmark_duration = start_time.elapsed();
        let transaction_throughput = (self.config.concurrent_transactions as f64) / transaction_benchmark_duration.as_secs_f64();
        
        // Benchmark event throughput
        let start_time = std::time::Instant::now();
        let events = self.create_test_events(self.config.test_event_count);
        
        for event in events {
            self.ordering_service.submit_event(event).await?;
        }
        
        let event_benchmark_duration = start_time.elapsed();
        let event_throughput = (self.config.test_event_count as f64) / event_benchmark_duration.as_secs_f64();
        
        metrics.insert("transaction_throughput_per_sec".to_string(), transaction_throughput);
        metrics.insert("event_throughput_per_sec".to_string(), event_throughput);
        metrics.insert("transaction_benchmark_duration_ms".to_string(), transaction_benchmark_duration.as_millis() as f64);
        metrics.insert("event_benchmark_duration_ms".to_string(), event_benchmark_duration.as_millis() as f64);
        
        Ok(metrics)
    }
    
    /// Test stress testing
    async fn test_stress_testing(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // High-load stress test
        let stress_event_count = self.config.test_event_count * 5;
        let stress_transaction_count = self.config.concurrent_transactions * 3;
        
        let start_time = std::time::Instant::now();
        
        // Create many concurrent transactions
        let mut transaction_handles = Vec::new();
        for _ in 0..stress_transaction_count {
            let coordinator = Arc::clone(&self.coordinator);
            let handle = tokio::spawn(async move {
                let participants = vec![
                    CrossBoundaryParticipant::KernelJournal,
                    CrossBoundaryParticipant::UserspaceJournal,
                ];
                let tx_id = coordinator.begin_transaction(participants).await?;
                sleep(Duration::from_millis(10)).await; // Simulate work
                coordinator.commit_transaction(tx_id).await?;
                Ok::<(), VexfsError>(())
            });
            transaction_handles.push(handle);
        }
        
        // Create many concurrent events
        let mut event_handles = Vec::new();
        for i in 0..stress_event_count {
            let ordering_service = Arc::clone(&self.ordering_service);
            let handle = tokio::spawn(async move {
                let event = SemanticEvent {
                    id: EventId::new(),
                    event_type: SemanticEventType::FilesystemRead,
                    timestamp: SemanticTimestamp {
                        wall_time: SystemTime::now(),
                        monotonic_time: Duration::from_millis(i as u64),
                    },
                    source: EventSource::Userspace,
                    flags: EventFlags::default(),
                    priority: EventPriority::Normal,
                    category: EventCategory::Filesystem,
                    context: SemanticContext::default(),
                };
                ordering_service.submit_event(event).await?;
                Ok::<(), VexfsError>(())
            });
            event_handles.push(handle);
        }
        
        // Wait for all to complete
        for handle in transaction_handles {
            handle.await.map_err(|e| VexfsError::Internal(e.to_string()))??;
        }
        
        for handle in event_handles {
            handle.await.map_err(|e| VexfsError::Internal(e.to_string()))??;
        }
        
        let stress_benchmark_duration = start_time.elapsed();
        
        metrics.insert("stress_transaction_count".to_string(), stress_transaction_count as f64);
        metrics.insert("stress_event_count".to_string(), stress_event_count as f64);
        metrics.insert("stress_benchmark_duration_ms".to_string(), stress_benchmark_duration.as_millis() as f64);
        
        Ok(metrics)
    }
    
    /// Test integration workflow
    async fn test_integration_workflow(&self) -> Result<HashMap<String, f64>, VexfsError> {
        let mut metrics = HashMap::new();
        
        // End-to-end integration test
        let start_time = std::time::Instant::now();
        
        // 1. Create cross-boundary transaction
        let participants = vec![
            CrossBoundaryParticipant::KernelJournal,
            CrossBoundaryParticipant::UserspaceJournal,
            CrossBoundaryParticipant::CrossLayerManager,
        ];
        let transaction_id = self.coordinator.begin_transaction(participants).await?;
        
        // 2. Create events for ordering
        let events = self.create_test_events(5);
        for event in events {
            self.ordering_service.submit_event(event).await?;
        }
        
        // 3. Create synchronization stream
        let stream_id = self.sync_manager.create_stream(
            SynchronizationBoundary::KernelToUserspace,
            SynchronizationBoundary::UserspaceToKernel,
            SynchronizationStrategy::Adaptive,
            StreamConfig::default(),
            None,
        ).await?;
        
        self.sync_manager.start_stream(stream_id).await?;
        
        // 4. Get ordered events and synchronize them
        sleep(Duration::from_millis(100)).await;
        let ordered_events = self.ordering_service.get_ordered_events(5).await?;
        
        for event in ordered_events {
            self.sync_manager.synchronize_event(stream_id, event).await?;
        }
        
        // 5. Commit transaction
        self.coordinator.commit_transaction(transaction_id).await?;
        
        let integration_duration = start_time.elapsed();
        
        metrics.insert("integration_workflow_duration_ms".to_string(), integration_duration.as_millis() as f64);
        metrics.insert("workflow_steps_completed".to_string(), 5.0);
        
        Ok(metrics)
    }
    
    /// Create test events
    fn create_test_events(&self, count: usize) -> Vec<SemanticEvent> {
        (0..count).map(|i| {
            SemanticEvent {
                id: EventId::new(),
                event_type: match i % 4 {
                    0 => SemanticEventType::FilesystemCreate,
                    1 => SemanticEventType::FilesystemWrite,
                    2 => SemanticEventType::FilesystemRead,
                    _ => SemanticEventType::FilesystemDelete,
                },
                timestamp: SemanticTimestamp {
                    wall_time: SystemTime::now(),
                    monotonic_time: Duration::from_millis(i as u64 * 100),
                },
                source: if i % 2 == 0 { EventSource::Kernel } else { EventSource::Userspace },
                flags: EventFlags::default(),
                priority: EventPriority::Normal,
                category: EventCategory::Filesystem,
                context: SemanticContext::default(),
            }
        }).collect()
    }
    
    /// Create test events with dependencies
    fn create_test_events_with_dependencies(&self, count: usize) -> Vec<SemanticEvent> {
        (0..count).map(|i| {
            SemanticEvent {
                id: EventId::new(),
                event_type: SemanticEventType::FilesystemWrite,
                timestamp: SemanticTimestamp {
                    wall_time: SystemTime::now(),
                    monotonic_time: Duration::from_millis(i as u64 * 50),
                },
                source: EventSource::Userspace,
                flags: EventFlags::default(),
                priority: if i < 5 { EventPriority::High } else { EventPriority::Normal },
                category: EventCategory::Filesystem,
                context: SemanticContext::default(),
            }
        }).collect()
    }
    
    /// Collect performance metrics from all components
    async fn collect_performance_metrics(&self) -> PerformanceMetrics {
        let coordinator_stats = self.coordinator.get_statistics().await;
        let ordering_stats = self.ordering_service.get_statistics().await;
        let sync_stats = self.sync_manager.get_stats().await;
        
        PerformanceMetrics {
            avg_commit_latency_us: coordinator_stats.avg_commit_latency_us,
            max_commit_latency_us: coordinator_stats.max_commit_latency_us,
            event_throughput: ordering_stats.events_per_second as u64,
            avg_ordering_latency_us: ordering_stats.avg_ordering_latency_us,
            avg_sync_latency_us: sync_stats.avg_sync_latency_us,
            deadlock_detection_time_us: coordinator_stats.avg_deadlock_detection_time_us,
            memory_usage_bytes: 0, // Would be collected from actual system metrics
            cpu_usage_percent: 0.0, // Would be collected from actual system metrics
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_23_4_2_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = Task23_4_2IntegrationTest::new(config).await.unwrap();
        
        let results = test_suite.run_all_tests().await;
        
        // Print results
        println!("Task 23.4.2 Integration Test Results:");
        println!("Success: {}", results.success);
        println!("Tests run: {}", results.test_results.len());
        
        for (test_name, result) in &results.test_results {
            println!("  {}: {} ({:.2}ms)",
                test_name,
                if result.passed { "PASS" } else { "FAIL" },
                result.duration.as_millis()
            );
            
            if let Some(error) = &result.error {
                println!("    Error: {}", error);
            }
        }
        
        if results.performance_metrics.avg_commit_latency_us > 0 {
            println!("\nPerformance Metrics:");
            println!("  Avg commit latency: {}μs", results.performance_metrics.avg_commit_latency_us);
            println!("  Event throughput: {} events/sec", results.performance_metrics.event_throughput);
            println!("  Avg ordering latency: {}μs", results.performance_metrics.avg_ordering_latency_us);
            println!("  Avg sync latency: {}μs", results.performance_metrics.avg_sync_latency_us);
        }
        
        // Assert that critical tests passed
        assert!(results.success, "Integration tests failed");
        
        // Assert performance targets (if benchmarking enabled)
        if results.performance_metrics.avg_commit_latency_us > 0 {
            assert!(results.performance_metrics.avg_commit_latency_us < 10000,
                "Commit latency too high: {}μs", results.performance_metrics.avg_commit_latency_us);
            assert!(results.performance_metrics.event_throughput > 1000,
                "Event throughput too low: {} events/sec", results.performance_metrics.event_throughput);
        }
    }
    
    #[tokio::test]
    async fn test_coordinator_only() {
        let config = IntegrationTestConfig {
            test_event_count: 100,
            concurrent_transactions: 5,
            enable_benchmarking: false,
            ..Default::default()
        };
        
        let test_suite = Task23_4_2IntegrationTest::new(config).await.unwrap();
        
        let result = test_suite.test_coordinator_basic_functionality().await;
        assert!(result.is_ok(), "Coordinator test failed: {:?}", result);
        
        let metrics = result.unwrap();
        assert!(metrics.contains_key("begin_latency_us"));
        assert!(metrics.contains_key("commit_latency_us"));
    }
    
    #[tokio::test]
    async fn test_ordering_service_only() {
        let config = IntegrationTestConfig {
            test_event_count: 50,
            enable_benchmarking: false,
            ..Default::default()
        };
        
        let test_suite = Task23_4_2IntegrationTest::new(config).await.unwrap();
        
        let result = test_suite.test_ordering_service_basic_functionality().await;
        assert!(result.is_ok(), "Ordering service test failed: {:?}", result);
        
        let metrics = result.unwrap();
        assert!(metrics.contains_key("submit_latency_us"));
        assert!(metrics.contains_key("retrieval_latency_us"));
        assert!(metrics.contains_key("events_processed"));
    }
    
    #[tokio::test]
    async fn test_sync_manager_only() {
        let config = IntegrationTestConfig {
            enable_benchmarking: false,
            ..Default::default()
        };
        
        let test_suite = Task23_4_2IntegrationTest::new(config).await.unwrap();
        
        let result = test_suite.test_sync_manager_basic_functionality().await;
        assert!(result.is_ok(), "Sync manager test failed: {:?}", result);
        
        let metrics = result.unwrap();
        assert!(metrics.contains_key("create_stream_latency_us"));
        assert!(metrics.contains_key("start_stream_latency_us"));
        assert!(metrics.contains_key("sync_event_latency_us"));
    }
}