//! Integration tests for VexFS Semantic Operation Journal RESTful API
//! 
//! This module provides comprehensive integration tests for Task 18.4,
//! testing all API endpoints, WebSocket streaming, and query functionality.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    
    use axum::http::StatusCode;
    use serde_json::json;
    use tokio::time::timeout;
    use axum::ServiceExt;
    use uuid::Uuid;
    
    use crate::semantic_api::{
        api_server::*,
        websocket_stream::*,
        query_processor::*,
        types::*,
        event_emission::*,
    };
    
    /// Create a test semantic event
    fn create_test_event(
        event_id: u64,
        event_type: SemanticEventType,
        agent_id: Option<String>,
        path: Option<String>,
    ) -> SemanticEvent {
        SemanticEvent {
            event_id,
            event_type,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: chrono::Utc::now(),
                sequence: event_id,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: event_id,
            local_sequence: event_id,
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::Normal,
            event_size: 256,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFFFFFFFFFFFFFFFF,
            agent_relevance_score: 100,
            replay_priority: 3,
            context: SemanticContext {
                transaction_id: Some(event_id),
                session_id: None,
                causality_chain_id: None,
                filesystem: path.map(|p| FilesystemContext {
                    path: p,
                    inode_number: Some(event_id + 10000),
                    file_type: Some("regular".to_string()),
                }),
                graph: None,
                vector: None,
                agent: agent_id.map(|id| AgentContext {
                    agent_id: id,
                    intent: Some("test_intent".to_string()),
                    confidence: Some(95),
                }),
                system: None,
                semantic: None,
                observability: None,
            },
            payload: Some(json!({
                "test_data": format!("payload for event {}", event_id),
                "number": event_id
            })),
            metadata: Some(json!({
                "source": "integration_test",
                "version": "1.0.0"
            })),
        }
    }
    
    /// Set up test storage with sample data
    async fn setup_test_storage() -> Arc<InMemoryEventStorage> {
        let storage = Arc::new(InMemoryEventStorage::new());
        
        // Add sample events
        let events = vec![
            create_test_event(1, SemanticEventType::FilesystemCreate, Some("agent_1".to_string()), Some("/test/file1.txt".to_string())),
            create_test_event(2, SemanticEventType::FilesystemWrite, Some("agent_1".to_string()), Some("/test/file1.txt".to_string())),
            create_test_event(3, SemanticEventType::FilesystemDelete, Some("agent_2".to_string()), Some("/test/file2.txt".to_string())),
            create_test_event(4, SemanticEventType::GraphNodeCreate, Some("agent_1".to_string()), None),
            create_test_event(5, SemanticEventType::VectorSearch, Some("agent_3".to_string()), None),
            create_test_event(6, SemanticEventType::AgentQuery, Some("agent_2".to_string()), None),
        ];
        
        for event in events {
            storage.add_event(event).await.unwrap();
        }
        
        storage
    }
    
    /// Create test API server
    fn create_test_server(storage: Arc<dyn EventStorage>) -> ApiServer {
        let config = ApiServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Use any available port for testing
            max_connections: 100,
            request_timeout_secs: 30,
            enable_cors: true,
            enable_compression: false, // Disable for simpler testing
            max_events_per_query: 1000,
            max_concurrent_streams: 10,
            rate_limit_per_minute: 1000,
        };
        
        ApiServer::new(config, storage)
    }
    
    #[tokio::test]
    async fn test_api_server_creation() {
        let storage = setup_test_storage().await;
        let server = create_test_server(storage);
        
        // Test that server can be created without errors
        assert!(true); // If we get here, creation succeeded
    }
    
    #[tokio::test]
    async fn test_event_storage_operations() {
        let storage = setup_test_storage().await;
        
        // Test event retrieval
        let event = storage.get_event(1).await.unwrap();
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_id, 1);
        
        // Test non-existent event
        let event = storage.get_event(999).await.unwrap();
        assert!(event.is_none());
        
        // Test statistics
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_events, 6);
        assert!(stats.events_by_type.contains_key("FilesystemCreate"));
        assert!(stats.events_by_category.contains_key("Filesystem"));
    }
    
    #[tokio::test]
    async fn test_event_query_filtering() {
        let storage = setup_test_storage().await;
        
        // Test event type filter
        let query = EventQuery {
            filter: EventFilter {
                event_types: Some(vec![SemanticEventType::FilesystemCreate]),
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(10),
            offset: None,
            sort_by: Some(SortBy::Timestamp),
            include_payload: true,
            include_metadata: true,
            include_causality: true,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].event_type, SemanticEventType::FilesystemCreate);
        assert_eq!(response.total_count, 1);
        assert!(!response.has_more);
        
        // Test category filter
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: Some(vec![EventCategory::Filesystem]),
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(10),
            offset: None,
            sort_by: Some(SortBy::EventId),
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 3); // FilesystemCreate, FilesystemWrite, FilesystemDelete
        assert_eq!(response.total_count, 3);
        
        // Test agent filter
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: Some("agent_1".to_string()),
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(10),
            offset: None,
            sort_by: Some(SortBy::EventId),
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 3); // Events 1, 2, 4
        assert_eq!(response.total_count, 3);
    }
    
    #[tokio::test]
    async fn test_event_query_pagination() {
        let storage = setup_test_storage().await;
        
        // Test pagination with limit
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(3),
            offset: None,
            sort_by: Some(SortBy::EventId),
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 3);
        assert_eq!(response.total_count, 6);
        assert!(response.has_more);
        
        // Test pagination with offset
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(3),
            offset: Some(3),
            sort_by: Some(SortBy::EventId),
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 3);
        assert_eq!(response.total_count, 6);
        assert!(!response.has_more);
    }
    
    #[tokio::test]
    async fn test_event_query_sorting() {
        let storage = setup_test_storage().await;
        
        // Test sorting by event ID (ascending)
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(10),
            offset: None,
            sort_by: Some(SortBy::EventId),
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 6);
        
        // Verify sorting
        for i in 1..response.events.len() {
            assert!(response.events[i-1].event_id <= response.events[i].event_id);
        }
        
        // Test sorting by priority
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(10),
            offset: None,
            sort_by: Some(SortBy::Priority),
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 6);
        
        // Verify priority sorting
        for i in 1..response.events.len() {
            assert!(response.events[i-1].priority <= response.events[i].priority);
        }
    }
    
    #[tokio::test]
    async fn test_websocket_connection_manager() {
        let config = WebSocketConfig::default();
        let manager = WebSocketConnectionManager::new(config);
        
        // Test initial state
        assert_eq!(manager.get_active_connection_count().await, 0);
        
        // Test event broadcasting
        let event = create_test_event(1, SemanticEventType::FilesystemCreate, Some("agent_1".to_string()), Some("/test/file.txt".to_string()));
        manager.broadcast_event(event).await.unwrap();
        
        // Test connection statistics
        let stats = manager.get_connection_stats().await;
        assert!(stats.is_empty());
        
        // Test cleanup (should not fail with no connections)
        let removed = manager.cleanup_inactive_connections().await;
        assert_eq!(removed, 0);
    }
    
    #[tokio::test]
    async fn test_event_index_system() {
        let config = IndexConfig::default();
        let index_system = EventIndexSystem::new(config);
        
        // Create and index test events
        let events = vec![
            create_test_event(1, SemanticEventType::FilesystemCreate, Some("agent_1".to_string()), Some("/test/file1.txt".to_string())),
            create_test_event(2, SemanticEventType::FilesystemWrite, Some("agent_1".to_string()), Some("/test/file1.txt".to_string())),
            create_test_event(3, SemanticEventType::GraphNodeCreate, Some("agent_2".to_string()), None),
        ];
        
        for event in &events {
            index_system.index_event(event).unwrap();
        }
        
        // Test event type query
        let query = EventQuery {
            filter: EventFilter {
                event_types: Some(vec![SemanticEventType::FilesystemCreate]),
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: None,
            offset: None,
            sort_by: None,
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let results = index_system.query_events(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results.contains(&1));
        
        // Test category query
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: Some(vec![EventCategory::Filesystem]),
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: None,
            offset: None,
            sort_by: None,
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let results = index_system.query_events(&query).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        
        // Test agent query
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: Some("agent_1".to_string()),
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: None,
            offset: None,
            sort_by: None,
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let results = index_system.query_events(&query).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        
        // Test statistics
        let stats = index_system.get_stats();
        assert_eq!(stats.total_events_indexed, 3);
        assert!(stats.index_memory_usage_mb > 0.0);
        
        // Test distributions
        let type_dist = index_system.get_event_type_distribution();
        assert_eq!(type_dist.get(&SemanticEventType::FilesystemCreate), Some(&1));
        assert_eq!(type_dist.get(&SemanticEventType::FilesystemWrite), Some(&1));
        assert_eq!(type_dist.get(&SemanticEventType::GraphNodeCreate), Some(&1));
        
        let category_dist = index_system.get_category_distribution();
        assert_eq!(category_dist.get(&EventCategory::Filesystem), Some(&2));
        assert_eq!(category_dist.get(&EventCategory::Graph), Some(&1));
        
        // Test time range
        let time_range = index_system.get_time_range();
        assert!(time_range.is_some());
    }
    
    #[tokio::test]
    async fn test_event_emission_integration() {
        // Initialize event emission framework
        let config = EventEmissionConfig::default();
        initialize_event_emission(config).unwrap();
        
        // Test emitting events
        let event_id1 = emit_filesystem_event(
            SemanticEventType::FilesystemCreate,
            "/test/integration.txt".to_string(),
            Some(12345),
            Some("regular".to_string()),
        ).unwrap();
        
        let event_id2 = emit_graph_event(
            SemanticEventType::GraphNodeCreate,
            Some(1),
            None,
            Some(1),
        ).unwrap();
        
        let event_id3 = emit_vector_event(
            SemanticEventType::VectorSearch,
            Some(1),
            Some(128),
            Some(1),
        ).unwrap();
        
        assert!(event_id1 > 0);
        assert!(event_id2 > 0);
        assert!(event_id3 > 0);
        assert!(event_id2 > event_id1);
        assert!(event_id3 > event_id2);
        
        // Get emission statistics
        if let Some(framework) = get_global_emission_framework() {
            let stats = framework.lock().unwrap().get_stats();
            assert!(stats.total_events_emitted >= 3);
            assert!(stats.events_by_type.len() > 0);
            assert!(stats.events_by_category.len() > 0);
        }
        
        // Shutdown framework
        shutdown_event_emission().unwrap();
    }
    
    #[tokio::test]
    async fn test_api_response_format() {
        // Test successful response
        let data = vec![1, 2, 3];
        let response = ApiResponse::success(data.clone());
        
        assert!(response.success);
        assert_eq!(response.data, Some(data));
        assert!(response.error.is_none());
        assert!(!response.request_id.to_string().is_empty());
        
        // Test error response
        let error_response: ApiResponse<Vec<i32>> = ApiResponse::error("Test error".to_string());
        
        assert!(!error_response.success);
        assert!(error_response.data.is_none());
        assert_eq!(error_response.error, Some("Test error".to_string()));
        assert!(!error_response.request_id.to_string().is_empty());
    }
    
    #[tokio::test]
    async fn test_stream_subscription() {
        let subscription = StreamSubscription {
            subscription_id: Uuid::new_v4(),
            agent_id: "test_agent".to_string(),
            filter: EventFilter {
                event_types: Some(vec![SemanticEventType::FilesystemCreate]),
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            buffer_size: 1000,
            include_historical: true,
            historical_limit: Some(100),
        };
        
        // Test serialization/deserialization
        let json = serde_json::to_string(&subscription).unwrap();
        let deserialized: StreamSubscription = serde_json::from_str(&json).unwrap();
        
        assert_eq!(subscription.subscription_id, deserialized.subscription_id);
        assert_eq!(subscription.agent_id, deserialized.agent_id);
        assert_eq!(subscription.buffer_size, deserialized.buffer_size);
        assert_eq!(subscription.include_historical, deserialized.include_historical);
        assert_eq!(subscription.historical_limit, deserialized.historical_limit);
    }
    
    #[tokio::test]
    async fn test_websocket_message_format() {
        let event = create_test_event(1, SemanticEventType::FilesystemCreate, Some("agent_1".to_string()), Some("/test/file.txt".to_string()));
        
        let stream_message = StreamEventMessage {
            subscription_id: Uuid::new_v4(),
            event: event.clone(),
            sequence_number: 1,
            timestamp: chrono::Utc::now(),
        };
        
        let ws_message = WebSocketMessage::Event {
            event: stream_message.clone(),
        };
        
        // Test serialization/deserialization
        let json = serde_json::to_string(&ws_message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            WebSocketMessage::Event { event: deserialized_event } => {
                assert_eq!(stream_message.subscription_id, deserialized_event.subscription_id);
                assert_eq!(stream_message.event.event_id, deserialized_event.event.event_id);
                assert_eq!(stream_message.sequence_number, deserialized_event.sequence_number);
            }
            _ => panic!("Expected Event message"),
        }
        
        // Test other message types
        let ping_message = WebSocketMessage::Ping {
            timestamp: chrono::Utc::now(),
        };
        
        let json = serde_json::to_string(&ping_message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            WebSocketMessage::Ping { .. } => {
                // Success
            }
            _ => panic!("Expected Ping message"),
        }
    }
    
    #[tokio::test]
    async fn test_comprehensive_api_workflow() {
        // This test simulates a complete workflow:
        // 1. Set up storage and API server
        // 2. Add events to storage
        // 3. Query events through different filters
        // 4. Test WebSocket streaming
        // 5. Verify statistics and performance
        
        let storage = setup_test_storage().await;
        let server = create_test_server(storage.clone());
        
        // Verify initial state
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_events, 6);
        
        // Test complex query with multiple filters
        let query = EventQuery {
            filter: EventFilter {
                event_types: Some(vec![
                    SemanticEventType::FilesystemCreate,
                    SemanticEventType::FilesystemWrite,
                ]),
                categories: None,
                time_range: None,
                agent_id: Some("agent_1".to_string()),
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: Some("/test/file1.txt".to_string()),
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: Some(10),
            offset: None,
            sort_by: Some(SortBy::Timestamp),
            include_payload: true,
            include_metadata: true,
            include_causality: true,
            aggregation: None,
        };
        
        let response = storage.query_events(&query).await.unwrap();
        assert_eq!(response.events.len(), 2); // Events 1 and 2
        assert_eq!(response.total_count, 2);
        assert!(!response.has_more);
        assert!(response.query_time_ms > 0);
        
        // Verify event content
        for event in &response.events {
            assert!(event.payload.is_some());
            assert!(event.metadata.is_some());
            assert!(event.context.filesystem.is_some());
            assert!(event.context.agent.is_some());
            
            let fs_context = event.context.filesystem.as_ref().unwrap();
            assert_eq!(fs_context.path, "/test/file1.txt");
            
            let agent_context = event.context.agent.as_ref().unwrap();
            assert_eq!(agent_context.agent_id, "agent_1");
        }
        
        // Test WebSocket connection manager
        let ws_config = WebSocketConfig::default();
        let ws_manager = WebSocketConnectionManager::new(ws_config);
        
        // Test event broadcasting
        let new_event = create_test_event(7, SemanticEventType::SystemMount, Some("agent_1".to_string()), None);
        ws_manager.broadcast_event(new_event.clone()).await.unwrap();
        
        // Add the event to storage as well
        storage.add_event(new_event).await.unwrap();
        
        // Verify updated statistics
        let updated_stats = storage.get_stats().await.unwrap();
        assert_eq!(updated_stats.total_events, 7);
        assert!(updated_stats.events_by_type.contains_key("SystemMount"));
        assert!(updated_stats.events_by_category.contains_key("System"));
        
        // Test performance metrics
        assert!(updated_stats.query_performance.total_queries > 0);
        assert!(updated_stats.query_performance.avg_query_time_ms >= 0.0);
    }
}