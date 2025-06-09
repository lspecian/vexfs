//! Integration tests for FUSE Journal Integration with Userspace Journal System
//!
//! This module provides comprehensive tests for Task 23.4.4 implementation,
//! demonstrating the integration between FUSE filesystem operations and the
//! userspace semantic journal system.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};
    
    use tempfile::tempdir;
    use uuid::Uuid;
    
    use crate::semantic_api::{
        SemanticEvent, SemanticEventType, SemanticTimestamp, EventFlags, EventPriority,
        SemanticResult, SemanticError,
        
        // FUSE journal integration components
        FuseJournalIntegration, FuseJournalConfig, FuseOperationType, FuseOperationContext,
        FuseEventMapper, FuseEventMappingConfig, FuseMappingContext,
        FuseJournalManager, FuseJournalManagerConfig, FuseMountInfo, FusePerformanceMode,
        
        // Userspace journal components
        UserspaceSemanticJournal, UserspaceJournalConfig, CompressionAlgorithm,
        
        // Cross-boundary coordination components
        CrossBoundaryTransactionCoordinator, CrossBoundaryConfig,
        JournalRecoveryManager, RecoveryConfig,
        BoundarySynchronizationManager, BoundarySyncConfig,
    };
    
    /// Test basic FUSE journal integration creation and configuration
    #[tokio::test]
    async fn test_fuse_journal_integration_creation() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        
        // Create FUSE journal manager
        let manager_config = FuseJournalManagerConfig::default();
        let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
        
        // Create FUSE event mapper
        let mapper_config = FuseEventMappingConfig::default();
        let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
        
        // Create FUSE journal integration
        let integration_config = FuseJournalConfig::default();
        let integration = FuseJournalIntegration::new(
            integration_config,
            journal_manager.clone(),
            event_mapper.clone(),
        )?;
        
        assert!(integration.is_enabled());
        assert_eq!(integration.get_active_operation_count(), 0);
        
        // Test configuration update
        let mut new_config = FuseJournalConfig::default();
        new_config.max_latency_overhead_percent = 10.0;
        integration.update_config(new_config)?;
        
        // Test enable/disable
        integration.set_enabled(false);
        assert!(!integration.is_enabled());
        
        integration.set_enabled(true);
        assert!(integration.is_enabled());
        
        Ok(())
    }
    
    /// Test FUSE event mapping functionality
    #[test]
    fn test_fuse_event_mapping() -> SemanticResult<()> {
        let mapper = FuseEventMapper::new_default();
        
        // Test filesystem operation mapping
        let fs_event = mapper.map_operation_to_completion_event(FuseOperationType::Create, true)?;
        assert_eq!(fs_event, SemanticEventType::FilesystemCreate);
        
        let fs_category = mapper.map_operation_to_category(FuseOperationType::Create);
        assert_eq!(fs_category, crate::semantic_api::types::EventCategory::Filesystem);
        
        // Test vector operation mapping
        let vector_event = mapper.map_operation_to_completion_event(FuseOperationType::VectorSearch, true)?;
        assert_eq!(vector_event, SemanticEventType::VectorSearch);
        
        let vector_category = mapper.map_operation_to_category(FuseOperationType::VectorSearch);
        assert_eq!(vector_category, crate::semantic_api::types::EventCategory::Vector);
        
        // Test graph operation mapping
        let graph_event = mapper.map_operation_to_completion_event(FuseOperationType::NodeCreate, true)?;
        assert_eq!(graph_event, SemanticEventType::GraphNodeCreate);
        
        let graph_category = mapper.map_operation_to_category(FuseOperationType::NodeCreate);
        assert_eq!(graph_category, crate::semantic_api::types::EventCategory::Graph);
        
        // Test system operation mapping
        let system_event = mapper.map_operation_to_completion_event(FuseOperationType::Mount, true)?;
        assert_eq!(system_event, SemanticEventType::SystemMount);
        
        let system_category = mapper.map_operation_to_category(FuseOperationType::Mount);
        assert_eq!(system_category, crate::semantic_api::types::EventCategory::System);
        
        // Test error event mapping
        let error_event = mapper.map_operation_to_completion_event(FuseOperationType::Create, false)?;
        assert_eq!(error_event, SemanticEventType::ObservabilityErrorReported);
        
        Ok(())
    }
    
    /// Test FUSE event flags and priority determination
    #[test]
    fn test_fuse_event_attributes() {
        let mapper = FuseEventMapper::new_default();
        
        // Test write operation flags
        let write_flags = mapper.determine_event_flags(FuseOperationType::Create);
        assert!(write_flags.contains(EventFlags::PERSISTENT));
        assert!(write_flags.contains(EventFlags::INDEXED));
        
        // Test vector operation flags
        let vector_flags = mapper.determine_event_flags(FuseOperationType::VectorInsert);
        assert!(vector_flags.contains(EventFlags::PERSISTENT));
        assert!(vector_flags.contains(EventFlags::INDEXED));
        assert!(vector_flags.contains(EventFlags::VECTOR_OPERATION));
        
        // Test graph operation flags
        let graph_flags = mapper.determine_event_flags(FuseOperationType::NodeCreate);
        assert!(graph_flags.contains(EventFlags::PERSISTENT));
        assert!(graph_flags.contains(EventFlags::INDEXED));
        assert!(graph_flags.contains(EventFlags::GRAPH_OPERATION));
        
        // Test system operation flags
        let system_flags = mapper.determine_event_flags(FuseOperationType::Mount);
        assert!(system_flags.contains(EventFlags::PERSISTENT));
        assert!(system_flags.contains(EventFlags::INDEXED));
        assert!(system_flags.contains(EventFlags::SYSTEM_CRITICAL));
        
        // Test priority determination
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Mount), EventPriority::Critical);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Sync), EventPriority::High);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Create), EventPriority::High);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Read), EventPriority::Medium);
        assert_eq!(mapper.determine_event_priority(FuseOperationType::Getattr), EventPriority::Low);
    }
    
    /// Test metadata extraction from FUSE operations
    #[test]
    fn test_fuse_metadata_extraction() {
        let mapper = FuseEventMapper::new_default();
        
        let mut additional_metadata = HashMap::new();
        additional_metadata.insert("custom_key".to_string(), "custom_value".to_string());
        
        let context = FuseMappingContext {
            operation_type: FuseOperationType::Write,
            path: "/test/document.txt".to_string(),
            inode: 12345,
            user_id: 1000,
            group_id: 1000,
            process_id: 54321,
            file_size: Some(2048),
            file_type: Some("regular".to_string()),
            permissions: Some(0o644),
            metadata: additional_metadata,
        };
        
        let extracted_metadata = mapper.extract_operation_metadata(&context);
        
        // Verify basic operation information
        assert_eq!(extracted_metadata.get("fuse_operation").unwrap(), "Write");
        assert_eq!(extracted_metadata.get("path").unwrap(), "/test/document.txt");
        assert_eq!(extracted_metadata.get("inode").unwrap(), "12345");
        assert_eq!(extracted_metadata.get("user_id").unwrap(), "1000");
        assert_eq!(extracted_metadata.get("group_id").unwrap(), "1000");
        assert_eq!(extracted_metadata.get("process_id").unwrap(), "54321");
        
        // Verify file-specific metadata
        assert_eq!(extracted_metadata.get("file_size").unwrap(), "2048");
        assert_eq!(extracted_metadata.get("file_type").unwrap(), "regular");
        assert_eq!(extracted_metadata.get("permissions").unwrap(), "644");
        
        // Verify detailed metadata
        assert_eq!(extracted_metadata.get("filename").unwrap(), "document.txt");
        assert_eq!(extracted_metadata.get("file_extension").unwrap(), "txt");
        assert_eq!(extracted_metadata.get("parent_path").unwrap(), "/test");
        assert_eq!(extracted_metadata.get("event_category").unwrap(), "Filesystem");
        assert_eq!(extracted_metadata.get("operation_domain").unwrap(), "filesystem");
        
        // Verify custom metadata
        assert_eq!(extracted_metadata.get("custom_key").unwrap(), "custom_value");
    }
    
    /// Test FUSE journal manager mount registration and management
    #[tokio::test]
    async fn test_fuse_journal_manager_mounts() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        let config = FuseJournalManagerConfig::default();
        let manager = FuseJournalManager::new(config)?;
        
        // Test mount registration
        let mount_id = manager.register_mount(
            temp_dir.path(),
            None,
            Some(FusePerformanceMode::Balanced),
            None,
        )?;
        
        // Verify mount was registered
        let mount_info = manager.get_mount_info(mount_id);
        assert!(mount_info.is_some());
        
        let mount_info = mount_info.unwrap();
        assert_eq!(mount_info.mount_id, mount_id);
        assert_eq!(mount_info.mount_path, temp_dir.path());
        assert_eq!(mount_info.performance_mode, FusePerformanceMode::Balanced);
        assert!(mount_info.journal_enabled);
        
        // Test mount listing
        let active_mounts = manager.list_active_mounts();
        assert_eq!(active_mounts.len(), 1);
        assert_eq!(active_mounts[0].mount_id, mount_id);
        
        // Test metrics
        let metrics = manager.get_metrics();
        assert_eq!(metrics.active_mounts.load(std::sync::atomic::Ordering::Relaxed), 1);
        
        // Test mount unregistration
        manager.unregister_mount(mount_id)?;
        
        let mount_info = manager.get_mount_info(mount_id);
        assert!(mount_info.is_none());
        
        let active_mounts = manager.list_active_mounts();
        assert_eq!(active_mounts.len(), 0);
        
        Ok(())
    }
    
    /// Test FUSE operation tracking and event generation
    #[tokio::test]
    async fn test_fuse_operation_tracking() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        
        // Create components
        let manager_config = FuseJournalManagerConfig::default();
        let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
        
        let mapper_config = FuseEventMappingConfig::default();
        let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
        
        let integration_config = FuseJournalConfig::default();
        let integration = FuseJournalIntegration::new(
            integration_config,
            journal_manager.clone(),
            event_mapper.clone(),
        )?;
        
        // Register a mount
        let mount_id = journal_manager.register_mount(
            temp_dir.path(),
            None,
            Some(FusePerformanceMode::Balanced),
            None,
        )?;
        
        // Start operation tracking
        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());
        
        let operation_id = integration.start_operation(
            FuseOperationType::Create,
            12345,
            temp_dir.path().join("test_file.txt").as_path(),
            1000,
            1000,
            54321,
            metadata,
        )?;
        
        assert!(operation_id > 0);
        assert_eq!(integration.get_active_operation_count(), 1);
        
        // Complete operation tracking
        integration.complete_operation(
            operation_id,
            Ok(()),
            Some(1024),
            None,
            None,
        )?;
        
        assert_eq!(integration.get_active_operation_count(), 0);
        
        // Verify metrics were updated
        let metrics = integration.get_metrics();
        assert_eq!(metrics.total_operations.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(metrics.events_generated.load(std::sync::atomic::Ordering::Relaxed), 1);
        
        Ok(())
    }
    
    /// Test FUSE performance modes and journal configuration
    #[test]
    fn test_fuse_performance_modes() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        let manager = FuseJournalManager::new(FuseJournalManagerConfig::default())?;
        
        // Test high performance mode
        let hp_mount_info = FuseMountInfo {
            mount_id: Uuid::new_v4(),
            mount_path: temp_dir.path().to_path_buf(),
            device_path: None,
            mount_time: SystemTime::now(),
            journal_enabled: true,
            journal_path: temp_dir.path().join(".vexfs_journal"),
            performance_mode: FusePerformanceMode::HighPerformance,
            metadata: HashMap::new(),
        };
        
        let hp_config = manager.create_journal_config(&hp_mount_info)?;
        assert_eq!(hp_config.target_emission_latency_ns, 500);
        assert_eq!(hp_config.buffer_size, 50000);
        assert!(!hp_config.enable_compression);
        
        // Test balanced mode
        let balanced_mount_info = FuseMountInfo {
            mount_id: Uuid::new_v4(),
            mount_path: temp_dir.path().to_path_buf(),
            device_path: None,
            mount_time: SystemTime::now(),
            journal_enabled: true,
            journal_path: temp_dir.path().join(".vexfs_journal"),
            performance_mode: FusePerformanceMode::Balanced,
            metadata: HashMap::new(),
        };
        
        let balanced_config = manager.create_journal_config(&balanced_mount_info)?;
        assert_eq!(balanced_config.target_emission_latency_ns, 1000);
        assert_eq!(balanced_config.buffer_size, 10000);
        assert!(!balanced_config.enable_compression);
        
        // Test high reliability mode
        let hr_mount_info = FuseMountInfo {
            mount_id: Uuid::new_v4(),
            mount_path: temp_dir.path().to_path_buf(),
            device_path: None,
            mount_time: SystemTime::now(),
            journal_enabled: true,
            journal_path: temp_dir.path().join(".vexfs_journal"),
            performance_mode: FusePerformanceMode::HighReliability,
            metadata: HashMap::new(),
        };
        
        let hr_config = manager.create_journal_config(&hr_mount_info)?;
        assert_eq!(hr_config.target_emission_latency_ns, 2000);
        assert_eq!(hr_config.buffer_size, 5000);
        assert!(hr_config.enable_compression);
        
        Ok(())
    }
    
    /// Test FUSE journal integration with different operation types
    #[tokio::test]
    async fn test_fuse_operation_types() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        
        // Create integration components
        let manager_config = FuseJournalManagerConfig::default();
        let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
        
        let mapper_config = FuseEventMappingConfig::default();
        let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
        
        let integration_config = FuseJournalConfig::default();
        let integration = FuseJournalIntegration::new(
            integration_config,
            journal_manager.clone(),
            event_mapper.clone(),
        )?;
        
        // Register mount
        let mount_id = journal_manager.register_mount(
            temp_dir.path(),
            None,
            Some(FusePerformanceMode::Balanced),
            None,
        )?;
        
        // Test different operation types
        let operation_types = vec![
            FuseOperationType::Create,
            FuseOperationType::Read,
            FuseOperationType::Write,
            FuseOperationType::Delete,
            FuseOperationType::VectorSearch,
            FuseOperationType::VectorInsert,
            FuseOperationType::NodeCreate,
            FuseOperationType::EdgeCreate,
            FuseOperationType::Mount,
            FuseOperationType::Sync,
        ];
        
        for (i, operation_type) in operation_types.iter().enumerate() {
            let operation_id = integration.start_operation(
                *operation_type,
                (i + 1) as u64,
                temp_dir.path().join(format!("test_file_{}.txt", i)).as_path(),
                1000,
                1000,
                54321,
                HashMap::new(),
            )?;
            
            // Add operation-specific data
            let vector_data = if matches!(operation_type, FuseOperationType::VectorSearch | FuseOperationType::VectorInsert) {
                Some(vec![0.1, 0.2, 0.3, 0.4])
            } else {
                None
            };
            
            let graph_data = if matches!(operation_type, FuseOperationType::NodeCreate | FuseOperationType::EdgeCreate) {
                let mut data = HashMap::new();
                data.insert("node_id".to_string(), format!("node_{}", i));
                Some(data)
            } else {
                None
            };
            
            integration.complete_operation(
                operation_id,
                Ok(()),
                Some(1024),
                vector_data,
                graph_data,
            )?;
        }
        
        // Verify all operations were tracked
        let metrics = integration.get_metrics();
        assert_eq!(metrics.total_operations.load(std::sync::atomic::Ordering::Relaxed), operation_types.len() as u64);
        assert_eq!(metrics.events_generated.load(std::sync::atomic::Ordering::Relaxed), operation_types.len() as u64);
        
        Ok(())
    }
    
    /// Test FUSE journal integration error handling
    #[tokio::test]
    async fn test_fuse_error_handling() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        
        // Create integration components
        let manager_config = FuseJournalManagerConfig::default();
        let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
        
        let mapper_config = FuseEventMappingConfig::default();
        let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
        
        let integration_config = FuseJournalConfig::default();
        let integration = FuseJournalIntegration::new(
            integration_config,
            journal_manager.clone(),
            event_mapper.clone(),
        )?;
        
        // Test operation when disabled
        integration.set_enabled(false);
        let operation_id = integration.start_operation(
            FuseOperationType::Create,
            12345,
            temp_dir.path().join("test_file.txt").as_path(),
            1000,
            1000,
            54321,
            HashMap::new(),
        )?;
        
        assert_eq!(operation_id, 0); // Should return dummy ID when disabled
        
        // Re-enable for further tests
        integration.set_enabled(true);
        
        // Register mount
        let mount_id = journal_manager.register_mount(
            temp_dir.path(),
            None,
            Some(FusePerformanceMode::Balanced),
            None,
        )?;
        
        // Test error operation
        let operation_id = integration.start_operation(
            FuseOperationType::Create,
            12345,
            temp_dir.path().join("test_file.txt").as_path(),
            1000,
            1000,
            54321,
            HashMap::new(),
        )?;
        
        // Complete with error
        integration.complete_operation(
            operation_id,
            Err(libc::EACCES), // Permission denied error
            None,
            None,
            None,
        )?;
        
        // Verify error was tracked
        let metrics = integration.get_metrics();
        assert_eq!(metrics.total_operations.load(std::sync::atomic::Ordering::Relaxed), 1);
        
        Ok(())
    }
    
    /// Test FUSE journal integration shutdown and cleanup
    #[tokio::test]
    async fn test_fuse_integration_shutdown() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        
        // Create integration components
        let manager_config = FuseJournalManagerConfig::default();
        let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
        
        let mapper_config = FuseEventMappingConfig::default();
        let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
        
        let integration_config = FuseJournalConfig::default();
        let integration = FuseJournalIntegration::new(
            integration_config,
            journal_manager.clone(),
            event_mapper.clone(),
        )?;
        
        // Register mount and start operations
        let mount_id = journal_manager.register_mount(
            temp_dir.path(),
            None,
            Some(FusePerformanceMode::Balanced),
            None,
        )?;
        
        let operation_id = integration.start_operation(
            FuseOperationType::Create,
            12345,
            temp_dir.path().join("test_file.txt").as_path(),
            1000,
            1000,
            54321,
            HashMap::new(),
        )?;
        
        assert_eq!(integration.get_active_operation_count(), 1);
        
        // Test shutdown
        integration.shutdown()?;
        
        assert!(!integration.is_enabled());
        assert_eq!(integration.get_active_operation_count(), 0);
        
        // Test journal manager shutdown
        journal_manager.shutdown()?;
        
        assert!(!journal_manager.is_enabled());
        assert_eq!(journal_manager.list_active_mounts().len(), 0);
        
        Ok(())
    }
    
    /// Test FUSE journal integration performance metrics
    #[tokio::test]
    async fn test_fuse_performance_metrics() -> SemanticResult<()> {
        let temp_dir = tempdir().unwrap();
        
        // Create integration components
        let manager_config = FuseJournalManagerConfig::default();
        let journal_manager = Arc::new(FuseJournalManager::new(manager_config)?);
        
        let mapper_config = FuseEventMappingConfig::default();
        let event_mapper = Arc::new(FuseEventMapper::new(mapper_config));
        
        let integration_config = FuseJournalConfig::default();
        let integration = FuseJournalIntegration::new(
            integration_config,
            journal_manager.clone(),
            event_mapper.clone(),
        )?;
        
        // Register mount
        let mount_id = journal_manager.register_mount(
            temp_dir.path(),
            None,
            Some(FusePerformanceMode::Balanced),
            None,
        )?;
        
        // Perform multiple operations to generate metrics
        for i in 0..10 {
            let operation_id = integration.start_operation(
                FuseOperationType::Write,
                (i + 1) as u64,
                temp_dir.path().join(format!("test_file_{}.txt", i)).as_path(),
                1000,
                1000,
                54321,
                HashMap::new(),
            )?;
            
            integration.complete_operation(
                operation_id,
                Ok(()),
                Some(1024),
                None,
                None,
            )?;
        }
        
        // Verify metrics
        let integration_metrics = integration.get_metrics();
        assert_eq!(integration_metrics.total_operations.load(std::sync::atomic::Ordering::Relaxed), 10);
        assert_eq!(integration_metrics.events_generated.load(std::sync::atomic::Ordering::Relaxed), 10);
        assert!(integration_metrics.get_average_latency_ns() > 0.0);
        
        let manager_metrics = journal_manager.get_metrics();
        assert_eq!(manager_metrics.active_mounts.load(std::sync::atomic::Ordering::Relaxed), 1);
        
        // Test mapper statistics
        let mapper_stats = event_mapper.get_mapping_statistics();
        assert_eq!(mapper_stats.get("filesystem_mapping_enabled").unwrap(), &1);
        assert_eq!(mapper_stats.get("vector_mapping_enabled").unwrap(), &1);
        assert_eq!(mapper_stats.get("graph_mapping_enabled").unwrap(), &1);
        assert_eq!(mapper_stats.get("system_mapping_enabled").unwrap(), &1);
        
        Ok(())
    }
}