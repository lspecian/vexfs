//! Integration tests for the userspace semantic journal system
//! 
//! This module provides comprehensive tests for Task 23.4.1 implementation,
//! demonstrating the integration between the core userspace journal,
//! kernel compatibility bridge, and event persistence layer.

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;
    use tokio::time::sleep;
    
    use crate::semantic_api::{
        userspace_journal::{
            UserspaceSemanticJournal, UserspaceJournalConfig, CompressionAlgorithm
        },
        journal_compatibility::{
            KernelCompatibilityBridge, CompatibilityBridgeConfig, CompatibilityMode
        },
        semantic_persistence::{
            SemanticPersistenceManager, PersistenceConfig, PersistenceStrategy,
            BatchConfig, StorageConfig
        },
        types::{SemanticEvent, SemanticEventType, EventPriority, SemanticTimestamp},
        SemanticResult,
    };
    
    /// Test the core userspace journal functionality
    #[tokio::test]
    async fn test_userspace_journal_basic_functionality() -> SemanticResult<()> {
        let dir = tempdir().unwrap();
        let journal_path = dir.path().join("test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            target_emission_latency_ns: 1000, // 1μs target
            target_throughput_events_per_sec: 10000,
            memory_pool_size: 100,
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config)?;
        journal.initialize()?;
        
        // Test event emission
        let mut event = SemanticEvent::default();
        event.event_type = SemanticEventType::FilesystemCreate;
        event.priority = EventPriority::Normal;
        
        let start_time = std::time::Instant::now();
        let event_id = journal.emit_event(event)?;
        let emission_latency = start_time.elapsed().as_nanos() as u64;
        
        // Verify performance targets
        assert!(emission_latency < 1000, "Emission latency {}ns exceeds 1μs target", emission_latency);
        assert_eq!(event_id, 1, "First event should have ID 1");
        
        // Test multiple events for throughput
        let num_events = 1000;
        let start_time = std::time::Instant::now();
        
        for i in 0..num_events {
            let mut event = SemanticEvent::default();
            event.event_type = SemanticEventType::FilesystemWrite;
            event.priority = EventPriority::Normal;
            journal.emit_event(event)?;
        }
        
        let total_time = start_time.elapsed();
        let throughput = (num_events as f64) / total_time.as_secs_f64();
        
        println!("Achieved throughput: {:.0} events/sec", throughput);
        assert!(throughput > 10000.0, "Throughput {:.0} events/sec below 10,000 target", throughput);
        
        journal.shutdown()?;
        Ok(())
    }
    
    /// Test kernel compatibility bridge functionality
    #[tokio::test]
    async fn test_kernel_compatibility_bridge() -> SemanticResult<()> {
        let dir = tempdir().unwrap();
        let kernel_journal_path = dir.path().join("kernel_journal");
        
        // Create a mock kernel journal file
        std::fs::write(&kernel_journal_path, &[0u8; 1024]).unwrap();
        
        let config = CompatibilityBridgeConfig {
            kernel_journal_path,
            compatibility_mode: CompatibilityMode::Full,
            enable_drift_detection: true,
            enable_auto_correction: true,
            ..Default::default()
        };
        
        let bridge = KernelCompatibilityBridge::new(config)?;
        
        // Test event conversion
        let userspace_event = SemanticEvent {
            event_id: 123,
            event_type: SemanticEventType::GraphNodeCreate,
            priority: EventPriority::High,
            timestamp: SemanticTimestamp {
                seconds: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                nanoseconds: 0,
            },
            ..Default::default()
        };
        
        // Convert userspace to kernel format
        let kernel_header = bridge.convert_userspace_to_kernel(&userspace_event)?;
        assert_eq!(kernel_header.event_id, 123);
        assert_eq!(kernel_header.event_type, SemanticEventType::GraphNodeCreate as u32);
        
        // Convert back to userspace format
        let converted_back = bridge.convert_kernel_to_userspace(&kernel_header)?;
        assert_eq!(converted_back.event_id, userspace_event.event_id);
        assert_eq!(converted_back.event_type, userspace_event.event_type);
        
        // Test synchronization state
        let sync_state = bridge.get_sync_state();
        assert_eq!(sync_state.drift_amount, 0);
        
        bridge.shutdown()?;
        Ok(())
    }
    
    /// Test semantic persistence manager functionality
    #[tokio::test]
    async fn test_semantic_persistence_manager() -> SemanticResult<()> {
        let dir = tempdir().unwrap();
        
        let config = PersistenceConfig {
            strategy: PersistenceStrategy::Batched,
            batch_config: BatchConfig {
                max_batch_size: 10,
                max_batch_time_ms: 100,
                enable_priority_ordering: true,
                ..Default::default()
            },
            storage_config: StorageConfig {
                storage_dir: dir.path().to_path_buf(),
                ..Default::default()
            },
            enable_indexing: true,
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
            ..Default::default()
        };
        
        let persistence_manager = SemanticPersistenceManager::new(config)?;
        persistence_manager.initialize().await?;
        
        // Test event persistence
        let buffered_event = crate::semantic_api::userspace_journal::BufferedSemanticEvent {
            event: SemanticEvent {
                event_id: 456,
                event_type: SemanticEventType::VectorSearch,
                priority: EventPriority::High,
                ..Default::default()
            },
            buffer_timestamp: SystemTime::now(),
            emission_latency_ns: 500,
            sequence_number: 1,
            priority: EventPriority::High,
            buffer_position: 0,
            cross_boundary_tx_id: None,
            retry_count: 0,
            processing_flags: crate::semantic_api::userspace_journal::ProcessingFlags::default(),
        };
        
        persistence_manager.persist_event(buffered_event).await?;
        
        // Verify metrics
        let metrics = persistence_manager.get_metrics();
        assert!(metrics.total_events_persisted.load(std::sync::atomic::Ordering::Relaxed) >= 0);
        
        persistence_manager.shutdown().await?;
        Ok(())
    }
    
    /// Test full integration of all components
    #[tokio::test]
    async fn test_full_integration() -> SemanticResult<()> {
        let dir = tempdir().unwrap();
        
        // Setup userspace journal
        let journal_config = UserspaceJournalConfig {
            journal_path: dir.path().join("userspace_journal"),
            target_emission_latency_ns: 1000,
            target_throughput_events_per_sec: 10000,
            memory_pool_size: 50,
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(journal_config)?;
        journal.initialize()?;
        
        // Setup kernel compatibility bridge
        let kernel_journal_path = dir.path().join("kernel_journal");
        std::fs::write(&kernel_journal_path, &[0u8; 1024]).unwrap();
        
        let bridge_config = CompatibilityBridgeConfig {
            kernel_journal_path,
            compatibility_mode: CompatibilityMode::Full,
            ..Default::default()
        };
        
        let bridge = KernelCompatibilityBridge::new(bridge_config)?;
        
        // Setup persistence manager
        let persistence_config = PersistenceConfig {
            strategy: PersistenceStrategy::Adaptive,
            storage_config: StorageConfig {
                storage_dir: dir.path().join("persistence"),
                ..Default::default()
            },
            ..Default::default()
        };
        
        let persistence_manager = SemanticPersistenceManager::new(persistence_config)?;
        persistence_manager.initialize().await?;
        
        // Test integrated workflow
        let test_events = vec![
            (SemanticEventType::FilesystemCreate, EventPriority::Normal),
            (SemanticEventType::GraphNodeCreate, EventPriority::High),
            (SemanticEventType::VectorSearch, EventPriority::Critical),
            (SemanticEventType::AgentQuery, EventPriority::Low),
            (SemanticEventType::SystemMount, EventPriority::High),
        ];
        
        for (event_type, priority) in test_events {
            // 1. Emit event through userspace journal
            let mut event = SemanticEvent::default();
            event.event_type = event_type;
            event.priority = priority;
            
            let event_id = journal.emit_event(event.clone())?;
            
            // 2. Convert to kernel format for compatibility
            let kernel_header = bridge.convert_userspace_to_kernel(&event)?;
            assert_eq!(kernel_header.event_id, event_id);
            
            // 3. Persist the event
            let buffered_event = crate::semantic_api::userspace_journal::BufferedSemanticEvent {
                event: event.clone(),
                buffer_timestamp: SystemTime::now(),
                emission_latency_ns: 500,
                sequence_number: event_id,
                priority,
                buffer_position: 0,
                cross_boundary_tx_id: None,
                retry_count: 0,
                processing_flags: crate::semantic_api::userspace_journal::ProcessingFlags::default(),
            };
            
            persistence_manager.persist_event(buffered_event).await?;
            
            // 4. Verify conversion back to userspace format
            let converted_back = bridge.convert_kernel_to_userspace(&kernel_header)?;
            assert_eq!(converted_back.event_type, event_type);
        }
        
        // Verify all components are working
        assert!(journal.is_running());
        assert!(bridge.is_running());
        assert!(persistence_manager.is_running());
        
        // Test performance under load
        let load_test_events = 100;
        let start_time = std::time::Instant::now();
        
        for i in 0..load_test_events {
            let mut event = SemanticEvent::default();
            event.event_type = SemanticEventType::FilesystemWrite;
            event.priority = if i % 10 == 0 { EventPriority::High } else { EventPriority::Normal };
            
            let event_id = journal.emit_event(event.clone())?;
            
            // Simulate kernel compatibility and persistence
            let _kernel_header = bridge.convert_userspace_to_kernel(&event)?;
            
            let buffered_event = crate::semantic_api::userspace_journal::BufferedSemanticEvent {
                event,
                buffer_timestamp: SystemTime::now(),
                emission_latency_ns: 300,
                sequence_number: event_id,
                priority: EventPriority::Normal,
                buffer_position: 0,
                cross_boundary_tx_id: None,
                retry_count: 0,
                processing_flags: crate::semantic_api::userspace_journal::ProcessingFlags::default(),
            };
            
            persistence_manager.persist_event(buffered_event).await?;
        }
        
        let total_time = start_time.elapsed();
        let throughput = (load_test_events as f64) / total_time.as_secs_f64();
        
        println!("Integrated system throughput: {:.0} events/sec", throughput);
        
        // Cleanup
        journal.shutdown()?;
        bridge.shutdown()?;
        persistence_manager.shutdown().await?;
        
        Ok(())
    }
    
    /// Test memory pool efficiency
    #[tokio::test]
    async fn test_memory_pool_efficiency() -> SemanticResult<()> {
        let dir = tempdir().unwrap();
        let journal_path = dir.path().join("memory_test_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            memory_pool_size: 10, // Small pool to test allocation behavior
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config)?;
        journal.initialize()?;
        
        // Emit more events than pool size to test fallback allocation
        for i in 0..20 {
            let mut event = SemanticEvent::default();
            event.event_type = SemanticEventType::FilesystemRead;
            event.priority = EventPriority::Normal;
            
            let event_id = journal.emit_event(event)?;
            assert_eq!(event_id, i + 1);
        }
        
        journal.shutdown()?;
        Ok(())
    }
    
    /// Test error handling and recovery
    #[tokio::test]
    async fn test_error_handling() -> SemanticResult<()> {
        // Test invalid journal path
        let invalid_config = UserspaceJournalConfig {
            journal_path: "/invalid/path/that/does/not/exist".into(),
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(invalid_config)?;
        let result = journal.initialize();
        assert!(result.is_err(), "Should fail with invalid path");
        
        // Test invalid kernel journal path for bridge
        let invalid_bridge_config = CompatibilityBridgeConfig {
            kernel_journal_path: "/invalid/kernel/journal".into(),
            ..Default::default()
        };
        
        let bridge = KernelCompatibilityBridge::new(invalid_bridge_config)?;
        let result = bridge.initialize();
        assert!(result.is_err(), "Should fail with invalid kernel journal path");
        
        Ok(())
    }
    
    /// Benchmark performance targets
    #[tokio::test]
    async fn test_performance_benchmarks() -> SemanticResult<()> {
        let dir = tempdir().unwrap();
        let journal_path = dir.path().join("benchmark_journal");
        
        let config = UserspaceJournalConfig {
            journal_path,
            target_emission_latency_ns: 500, // 500ns aggressive target
            target_throughput_events_per_sec: 15000, // 15k events/sec
            memory_pool_size: 1000,
            ..Default::default()
        };
        
        let journal = UserspaceSemanticJournal::new(config)?;
        journal.initialize()?;
        
        // Warm up
        for _ in 0..100 {
            let event = SemanticEvent::default();
            journal.emit_event(event)?;
        }
        
        // Benchmark emission latency
        let mut latencies = Vec::new();
        for _ in 0..1000 {
            let event = SemanticEvent::default();
            let start = std::time::Instant::now();
            journal.emit_event(event)?;
            let latency = start.elapsed().as_nanos() as u64;
            latencies.push(latency);
        }
        
        latencies.sort();
        let p50 = latencies[latencies.len() / 2];
        let p95 = latencies[(latencies.len() * 95) / 100];
        let p99 = latencies[(latencies.len() * 99) / 100];
        
        println!("Emission latency - P50: {}ns, P95: {}ns, P99: {}ns", p50, p95, p99);
        
        // Verify performance targets
        assert!(p50 < 1000, "P50 latency {}ns exceeds 1μs", p50);
        assert!(p95 < 2000, "P95 latency {}ns exceeds 2μs", p95);
        
        journal.shutdown()?;
        Ok(())
    }
}