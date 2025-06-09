//! Integration Tests for VexFS Semantic Event Interception and Hooks
//! 
//! This module tests the complete event interception and hook system
//! for Task 18.3, verifying that events are properly captured and
//! emitted across all layers.

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_event_emission_framework_initialization() {
        let config = EventEmissionConfig {
            enabled: true,
            buffer_size: 1000,
            batch_size: 10,
            flush_interval_ms: 50,
            max_events_per_second: 100,
            enable_kernel_events: true,
            enable_userspace_events: true,
            enable_graph_events: true,
            enable_vector_events: true,
            enable_agent_events: true,
            enable_system_events: true,
            enable_semantic_events: true,
            enable_observability_events: true,
            thread_safe: true,
            compression_enabled: false,
        };

        // Initialize event emission framework
        initialize_event_emission(config).unwrap();

        // Verify framework is available
        assert!(get_global_emission_framework().is_some());

        // Test emitting events
        let event_id1 = emit_filesystem_event(
            SemanticEventType::FilesystemCreate,
            "/test/file1.txt".to_string(),
            Some(12345),
            Some("regular".to_string()),
        ).unwrap();

        let event_id2 = emit_graph_event(
            SemanticEventType::GraphNodeCreate,
            Some(67890),
            None,
            Some(1),
        ).unwrap();

        let event_id3 = emit_vector_event(
            SemanticEventType::VectorCreate,
            Some(11111),
            Some(128),
            Some(1),
        ).unwrap();

        assert!(event_id1 > 0);
        assert!(event_id2 > 0);
        assert!(event_id3 > 0);

        // Wait for processing
        sleep(Duration::from_millis(200)).await;

        // Check statistics
        if let Some(framework) = get_global_emission_framework() {
            let stats = framework.lock().unwrap().get_stats();
            assert_eq!(stats.total_events_emitted, 3);
            assert!(stats.events_by_category.get("Filesystem").unwrap_or(&0) >= &1);
            assert!(stats.events_by_category.get("Graph").unwrap_or(&0) >= &1);
            assert!(stats.events_by_category.get("Vector").unwrap_or(&0) >= &1);
        }

        // Cleanup
        shutdown_event_emission().unwrap();
    }

    #[test]
    fn test_kernel_hooks_initialization() {
        // Initialize kernel hooks
        initialize_kernel_hooks().unwrap();

        // Verify hooks are enabled
        assert!(are_kernel_hooks_enabled());

        // Cleanup
        cleanup_kernel_hooks();
    }

    #[test]
    fn test_userspace_hooks_initialization() {
        let config = UserspaceHookConfig {
            graph_hooks_enabled: true,
            vector_hooks_enabled: true,
            performance_tracking: true,
            error_tracking: true,
            transaction_tracking: true,
            bulk_operation_tracking: true,
            detailed_logging: false,
        };

        // Initialize userspace hooks
        initialize_userspace_hooks(config).unwrap();

        // Verify registry is available
        assert!(get_userspace_registry().is_some());

        // Test hook operations
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), "test_node".to_string());
        properties.insert("type".to_string(), "document".to_string());

        // Test graph hooks
        hook_graph_node_create(12345, &properties).unwrap();
        hook_graph_edge_create(67890, 12345, 54321, &properties).unwrap();

        // Test vector hooks
        let vector_data = vec![1.0, 2.0, 3.0, 4.0];
        hook_vector_create(11111, 4, &vector_data).unwrap();

        let query_vector = vec![0.5, 1.5, 2.5, 3.5];
        let search_results = vec![(11111, 0.95), (22222, 0.87), (33333, 0.76)];
        hook_vector_search(&query_vector, 3, &search_results).unwrap();
    }

    #[tokio::test]
    async fn test_cross_layer_event_integration() {
        // Initialize all components
        let emission_config = EventEmissionConfig::default();
        initialize_event_emission(emission_config).unwrap();
        
        initialize_kernel_hooks().unwrap();
        
        let userspace_config = UserspaceHookConfig::default();
        initialize_userspace_hooks(userspace_config).unwrap();

        // Test filesystem operations
        emit_filesystem_event(
            SemanticEventType::FilesystemCreate,
            "/test/integration/file.txt".to_string(),
            Some(98765),
            Some("regular".to_string()),
        ).unwrap();

        emit_filesystem_event(
            SemanticEventType::FilesystemWrite,
            "/test/integration/file.txt".to_string(),
            Some(98765),
            Some("regular".to_string()),
        ).unwrap();

        // Test graph operations
        let mut node_props = HashMap::new();
        node_props.insert("title".to_string(), "Integration Test Document".to_string());
        node_props.insert("content_type".to_string(), "text/plain".to_string());
        
        hook_graph_node_create(98765, &node_props).unwrap();

        let mut edge_props = HashMap::new();
        edge_props.insert("relationship".to_string(), "contains".to_string());
        
        hook_graph_edge_create(11111, 98765, 12345, &edge_props).unwrap();

        // Test vector operations
        let document_vector = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        hook_vector_create(98765, 5, &document_vector).unwrap();

        let search_query = vec![0.15, 0.25, 0.35, 0.45, 0.55];
        let results = vec![(98765, 0.98), (12345, 0.85)];
        hook_vector_search(&search_query, 2, &results).unwrap();

        // Wait for event processing
        sleep(Duration::from_millis(300)).await;

        // Verify events were captured
        if let Some(framework) = get_global_emission_framework() {
            let stats = framework.lock().unwrap().get_stats();
            
            // Should have filesystem, graph, and vector events
            assert!(stats.total_events_emitted >= 6);
            assert!(stats.events_by_category.get("Filesystem").unwrap_or(&0) >= &2);
            assert!(stats.events_by_category.get("Graph").unwrap_or(&0) >= &2);
            assert!(stats.events_by_category.get("Vector").unwrap_or(&0) >= &2);
        }

        // Cleanup
        cleanup_kernel_hooks();
        shutdown_event_emission().unwrap();
    }

    #[tokio::test]
    async fn test_event_rate_limiting() {
        let config = EventEmissionConfig {
            enabled: true,
            max_events_per_second: 5, // Very low limit for testing
            ..Default::default()
        };

        initialize_event_emission(config).unwrap();

        let mut successful_events = 0;
        let mut rate_limited_events = 0;

        // Try to emit 10 events rapidly
        for i in 0..10 {
            match emit_filesystem_event(
                SemanticEventType::FilesystemRead,
                format!("/test/rate_limit_{}.txt", i),
                Some(i as u64),
                Some("regular".to_string()),
            ) {
                Ok(_) => successful_events += 1,
                Err(_) => rate_limited_events += 1,
            }
        }

        // Should have some rate limiting
        assert!(successful_events <= 5);
        assert!(rate_limited_events > 0);

        // Check rate limit statistics
        if let Some(framework) = get_global_emission_framework() {
            let stats = framework.lock().unwrap().get_stats();
            assert!(stats.rate_limit_hits > 0);
        }

        shutdown_event_emission().unwrap();
    }

    #[test]
    fn test_operation_context_tracking() {
        let config = UserspaceHookConfig::default();
        initialize_userspace_hooks(config).unwrap();

        if let Some(registry) = get_userspace_registry() {
            // Start tracking an operation
            let mut metadata = HashMap::new();
            metadata.insert("operation_type".to_string(), "bulk_insert".to_string());
            metadata.insert("batch_size".to_string(), "1000".to_string());

            let op_id = registry.start_operation("graph_bulk_insert".to_string(), metadata);
            assert_eq!(registry.get_active_operation_count(), 1);

            // End the operation
            let context = registry.end_operation(op_id);
            assert!(context.is_some());
            assert_eq!(registry.get_active_operation_count(), 0);

            let ctx = context.unwrap();
            assert_eq!(ctx.operation_type, "graph_bulk_insert");
            assert_eq!(ctx.metadata.get("batch_size").unwrap(), "1000");
        }
    }

    #[tokio::test]
    async fn test_event_categorization() {
        let config = EventEmissionConfig::default();
        initialize_event_emission(config).unwrap();

        // Emit events of different categories
        emit_filesystem_event(
            SemanticEventType::FilesystemCreate,
            "/test/categorization.txt".to_string(),
            Some(1001),
            Some("regular".to_string()),
        ).unwrap();

        emit_graph_event(
            SemanticEventType::GraphNodeCreate,
            Some(2001),
            None,
            Some(1),
        ).unwrap();

        emit_vector_event(
            SemanticEventType::VectorSearch,
            None,
            Some(256),
            Some(10),
        ).unwrap();

        // Wait for processing
        sleep(Duration::from_millis(100)).await;

        // Verify categorization
        if let Some(framework) = get_global_emission_framework() {
            let stats = framework.lock().unwrap().get_stats();
            
            // Check event type counts
            assert!(stats.events_by_type.get("FilesystemCreate").unwrap_or(&0) >= &1);
            assert!(stats.events_by_type.get("GraphNodeCreate").unwrap_or(&0) >= &1);
            assert!(stats.events_by_type.get("VectorSearch").unwrap_or(&0) >= &1);
            
            // Check category counts
            assert!(stats.events_by_category.get("Filesystem").unwrap_or(&0) >= &1);
            assert!(stats.events_by_category.get("Graph").unwrap_or(&0) >= &1);
            assert!(stats.events_by_category.get("Vector").unwrap_or(&0) >= &1);
        }

        shutdown_event_emission().unwrap();
    }

    #[tokio::test]
    async fn test_event_context_propagation() {
        let config = EventEmissionConfig::default();
        initialize_event_emission(config).unwrap();

        // Create a context with transaction and causality information
        let context = SemanticContext {
            transaction_id: Some(12345),
            session_id: Some(67890),
            causality_chain_id: Some(11111),
            filesystem: Some(FilesystemContext {
                path: "/test/context_propagation.txt".to_string(),
                inode_number: Some(22222),
                file_type: Some("regular".to_string()),
            }),
            graph: None,
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: None,
        };

        let flags = EventFlags {
            atomic: true,
            transactional: true,
            causal: true,
            agent_visible: true,
            deterministic: true,
            compressed: false,
            indexed: true,
            replicated: false,
        };

        // Emit event with context
        if let Some(framework) = get_global_emission_framework() {
            let event_id = framework.lock().unwrap().emit_event(
                SemanticEventType::FilesystemWrite,
                context,
                flags,
                EventPriority::High,
                Some(serde_json::json!({"bytes_written": 1024})),
                Some(serde_json::json!({"operation": "context_test"})),
            ).unwrap();

            assert!(event_id > 0);
        }

        // Wait for processing
        sleep(Duration::from_millis(100)).await;

        shutdown_event_emission().unwrap();
    }
}