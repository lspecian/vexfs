//! Example demonstrating VexFS Semantic Operation Journal RESTful API
//! 
//! This example shows how to use the RESTful API for querying and streaming
//! semantic events from the journal, implementing Task 18.4 functionality.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::time::sleep;
use uuid::Uuid;

use vexfs::semantic_api::{
    api_server::*,
    websocket_stream::*,
    query_processor::*,
    types::*,
    event_emission::*,
    SemanticResult,
};

/// Example demonstrating the complete RESTful API workflow
#[tokio::main]
async fn main() -> SemanticResult<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 VexFS Semantic Operation Journal RESTful API Example");
    println!("======================================================");
    
    // Step 1: Set up the storage and indexing system
    println!("\n📊 Setting up storage and indexing system...");
    let storage = setup_example_storage().await?;
    let index_system = setup_example_indexing().await?;
    
    // Step 2: Create and configure the API server
    println!("\n🌐 Creating API server...");
    let server_config = ApiServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        max_connections: 1000,
        request_timeout_secs: 30,
        enable_cors: true,
        enable_compression: true,
        max_events_per_query: 10000,
        max_concurrent_streams: 100,
        rate_limit_per_minute: 1000,
    };
    
    let api_server = ApiServer::new(server_config, storage.clone());
    println!("✅ API server created and configured");
    
    // Step 3: Set up WebSocket connection manager
    println!("\n🔌 Setting up WebSocket connection manager...");
    let ws_config = WebSocketConfig {
        max_message_size: 1024 * 1024, // 1MB
        ping_interval_secs: 30,
        connection_timeout_secs: 300,
        max_buffer_size: 1000,
        enable_compression: true,
        heartbeat_interval_secs: 10,
    };
    
    let ws_manager = WebSocketConnectionManager::new(ws_config);
    println!("✅ WebSocket connection manager ready");
    
    // Step 4: Demonstrate event emission and storage
    println!("\n📝 Emitting sample events...");
    emit_sample_events(&storage, &ws_manager).await?;
    
    // Step 5: Demonstrate query functionality
    println!("\n🔍 Demonstrating query functionality...");
    demonstrate_queries(&storage).await?;
    
    // Step 6: Demonstrate indexing and performance
    println!("\n⚡ Demonstrating indexing and performance...");
    demonstrate_indexing(&index_system).await?;
    
    // Step 7: Show statistics and metrics
    println!("\n📈 Displaying statistics and metrics...");
    display_statistics(&storage, &index_system, &ws_manager).await?;
    
    // Step 8: Demonstrate API response formats
    println!("\n📋 Demonstrating API response formats...");
    demonstrate_api_responses().await?;
    
    // Step 9: Demonstrate WebSocket streaming
    println!("\n🌊 Demonstrating WebSocket streaming...");
    demonstrate_websocket_streaming(&ws_manager).await?;
    
    println!("\n🎉 Example completed successfully!");
    println!("The RESTful API for VexFS Semantic Operation Journal is ready for use.");
    
    Ok(())
}

/// Set up example storage with sample data
async fn setup_example_storage() -> SemanticResult<Arc<InMemoryEventStorage>> {
    let storage = Arc::new(InMemoryEventStorage::new());
    
    println!("  📦 Creating in-memory event storage");
    println!("  ✅ Storage initialized");
    
    Ok(storage)
}

/// Set up example indexing system
async fn setup_example_indexing() -> SemanticResult<EventIndexSystem> {
    let config = IndexConfig {
        enable_timestamp_index: true,
        enable_event_type_index: true,
        enable_category_index: true,
        enable_agent_index: true,
        enable_transaction_index: true,
        enable_path_index: true,
        enable_full_text_search: false, // Disabled for this example
        max_index_memory_mb: 512,
        index_update_batch_size: 1000,
    };
    
    let index_system = EventIndexSystem::new(config);
    
    println!("  🗂️  Creating event index system");
    println!("  ✅ Indexing system initialized");
    
    Ok(index_system)
}

/// Emit sample events to demonstrate the system
async fn emit_sample_events(
    storage: &Arc<InMemoryEventStorage>,
    ws_manager: &WebSocketConnectionManager,
) -> SemanticResult<()> {
    let sample_events = vec![
        create_sample_event(1, SemanticEventType::FilesystemCreate, "agent_alice", "/documents/report.pdf"),
        create_sample_event(2, SemanticEventType::FilesystemWrite, "agent_alice", "/documents/report.pdf"),
        create_sample_event(3, SemanticEventType::FilesystemRead, "agent_bob", "/documents/report.pdf"),
        create_sample_event(4, SemanticEventType::GraphNodeCreate, "agent_alice", ""),
        create_sample_event(5, SemanticEventType::GraphEdgeCreate, "agent_alice", ""),
        create_sample_event(6, SemanticEventType::VectorSearch, "agent_bob", ""),
        create_sample_event(7, SemanticEventType::VectorCreate, "agent_charlie", ""),
        create_sample_event(8, SemanticEventType::AgentQuery, "agent_bob", ""),
        create_sample_event(9, SemanticEventType::SystemMount, "system", ""),
        create_sample_event(10, SemanticEventType::ObservabilityMetricCollected, "system", ""),
    ];
    
    for (i, event) in sample_events.iter().enumerate() {
        storage.add_event(event.clone()).await?;
        ws_manager.broadcast_event(event.clone()).await?;
        println!("  📝 Event {} emitted: {:?}", i + 1, event.event_type);
    }
    
    println!("  ✅ {} sample events emitted", sample_events.len());
    Ok(())
}

/// Create a sample semantic event
fn create_sample_event(
    event_id: u64,
    event_type: SemanticEventType,
    agent_id: &str,
    path: &str,
) -> SemanticEvent {
    SemanticEvent {
        event_id,
        event_type,
        event_subtype: None,
        timestamp: SemanticTimestamp {
            timestamp: chrono::Utc::now(),
            sequence: event_id,
            cpu_id: 0,
            process_id: std::process::id(),
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
        priority: match event_type {
            SemanticEventType::SystemMount => EventPriority::Critical,
            SemanticEventType::AgentQuery => EventPriority::High,
            _ => EventPriority::Normal,
        },
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
            session_id: Some(event_id / 3), // Group events into sessions
            causality_chain_id: Some(event_id / 2), // Group events into causality chains
            filesystem: if !path.is_empty() {
                Some(FilesystemContext {
                    path: path.to_string(),
                    inode_number: Some(event_id + 10000),
                    file_type: Some("regular".to_string()),
                })
            } else {
                None
            },
            graph: if matches!(event_type, SemanticEventType::GraphNodeCreate | SemanticEventType::GraphEdgeCreate) {
                Some(GraphContext {
                    node_id: Some(event_id),
                    edge_id: if event_type == SemanticEventType::GraphEdgeCreate { Some(event_id) } else { None },
                    operation_type: Some(1),
                })
            } else {
                None
            },
            vector: if matches!(event_type, SemanticEventType::VectorSearch | SemanticEventType::VectorCreate) {
                Some(VectorContext {
                    vector_id: Some(event_id),
                    dimensions: Some(128),
                    element_type: Some(1),
                })
            } else {
                None
            },
            agent: Some(AgentContext {
                agent_id: agent_id.to_string(),
                intent: Some(format!("Intent for {}", event_type.description())),
                confidence: Some(95),
            }),
            system: if agent_id == "system" {
                Some(SystemContext {
                    system_load: Some(75),
                    memory_usage: Some(1024 * 1024 * 512), // 512MB
                    io_pressure: Some(25),
                })
            } else {
                None
            },
            semantic: Some(SemanticContextData {
                tags: [
                    ("category".to_string(), format!("{:?}", event_type.category())),
                    ("priority".to_string(), "normal".to_string()),
                    ("source".to_string(), "example".to_string()),
                ].into_iter().collect(),
                intent: Some(format!("Example intent for {}", event_type.description())),
                confidence: Some(90),
            }),
            observability: if event_type == SemanticEventType::ObservabilityMetricCollected {
                Some(ObservabilityContext {
                    metric_name: Some("cpu_usage".to_string()),
                    metric_value: Some(75.5),
                    metric_unit: Some("percent".to_string()),
                    log_level: None,
                    log_message: None,
                    trace_id: Some(Uuid::new_v4().to_string()),
                    span_id: Some(Uuid::new_v4().to_string()),
                    parent_span_id: None,
                    service_name: Some("vexfs".to_string()),
                    operation_name: Some("metric_collection".to_string()),
                    resource_type: Some("cpu".to_string()),
                    threshold_value: Some(80.0),
                    alert_severity: None,
                })
            } else {
                None
            },
        },
        payload: Some(json!({
            "event_data": format!("Sample data for event {}", event_id),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metadata": {
                "source": "example",
                "version": "1.0.0",
                "environment": "development"
            }
        })),
        metadata: Some(json!({
            "created_by": "semantic_api_rest_example",
            "example_version": "1.0.0",
            "tags": ["example", "demo", "rest_api"],
            "performance": {
                "processing_time_ms": 1.5,
                "memory_usage_bytes": 1024
            }
        })),
    }
}

/// Demonstrate various query capabilities
async fn demonstrate_queries(storage: &Arc<InMemoryEventStorage>) -> SemanticResult<()> {
    // Query 1: Filter by event type
    println!("  🔍 Query 1: Filter by event type (FilesystemCreate)");
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
    
    let response = storage.query_events(&query).await?;
    println!("    📊 Found {} events, total: {}, has_more: {}, query_time: {}ms",
             response.events.len(), response.total_count, response.has_more, response.query_time_ms);
    
    // Query 2: Filter by category
    println!("  🔍 Query 2: Filter by category (Filesystem)");
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
    
    let response = storage.query_events(&query).await?;
    println!("    📊 Found {} filesystem events", response.events.len());
    
    // Query 3: Filter by agent
    println!("  🔍 Query 3: Filter by agent (agent_alice)");
    let query = EventQuery {
        filter: EventFilter {
            event_types: None,
            categories: None,
            time_range: None,
            agent_id: Some("agent_alice".to_string()),
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
    
    let response = storage.query_events(&query).await?;
    println!("    📊 Found {} events by agent_alice", response.events.len());
    
    // Query 4: Complex filter with multiple criteria
    println!("  🔍 Query 4: Complex filter (Filesystem events by agent_alice)");
    let query = EventQuery {
        filter: EventFilter {
            event_types: None,
            categories: Some(vec![EventCategory::Filesystem]),
            time_range: None,
            agent_id: Some("agent_alice".to_string()),
            transaction_id: None,
            causality_chain_id: None,
            path_pattern: Some("/documents/report.pdf".to_string()),
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
    
    let response = storage.query_events(&query).await?;
    println!("    📊 Found {} matching events with complex filter", response.events.len());
    
    // Query 5: Pagination example
    println!("  🔍 Query 5: Pagination (first 3 events)");
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
    
    let response = storage.query_events(&query).await?;
    println!("    📊 Page 1: {} events, has_more: {}", response.events.len(), response.has_more);
    
    // Query 6: Second page
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
    
    let response = storage.query_events(&query).await?;
    println!("    📊 Page 2: {} events, has_more: {}", response.events.len(), response.has_more);
    
    println!("  ✅ Query demonstrations completed");
    Ok(())
}

/// Demonstrate indexing capabilities
async fn demonstrate_indexing(index_system: &EventIndexSystem) -> SemanticResult<()> {
    // Index sample events
    let sample_events = vec![
        create_sample_event(1, SemanticEventType::FilesystemCreate, "agent_alice", "/documents/report.pdf"),
        create_sample_event(2, SemanticEventType::GraphNodeCreate, "agent_bob", ""),
        create_sample_event(3, SemanticEventType::VectorSearch, "agent_charlie", ""),
    ];
    
    println!("  🗂️  Indexing {} sample events", sample_events.len());
    for event in &sample_events {
        index_system.index_event(event)?;
    }
    
    // Test index queries
    println!("  🔍 Testing index-based queries");
    
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
    
    let results = index_system.query_events(&query)?;
    println!("    📊 Index query found {} events", results.len());
    
    // Show index statistics
    let stats = index_system.get_stats();
    println!("    📈 Index statistics:");
    println!("      - Total events indexed: {}", stats.total_events_indexed);
    println!("      - Memory usage: {:.2} MB", stats.index_memory_usage_mb);
    println!("      - Timestamp index size: {}", stats.timestamp_index_size);
    println!("      - Event type index size: {}", stats.event_type_index_size);
    
    // Show distributions
    let type_dist = index_system.get_event_type_distribution();
    println!("    📊 Event type distribution:");
    for (event_type, count) in type_dist {
        println!("      - {:?}: {}", event_type, count);
    }
    
    let category_dist = index_system.get_category_distribution();
    println!("    📊 Category distribution:");
    for (category, count) in category_dist {
        println!("      - {:?}: {}", category, count);
    }
    
    println!("  ✅ Indexing demonstration completed");
    Ok(())
}

/// Display comprehensive statistics
async fn display_statistics(
    storage: &Arc<InMemoryEventStorage>,
    index_system: &EventIndexSystem,
    ws_manager: &WebSocketConnectionManager,
) -> SemanticResult<()> {
    // Storage statistics
    let storage_stats = storage.get_stats().await?;
    println!("  📊 Storage Statistics:");
    println!("    - Total events: {}", storage_stats.total_events);
    println!("    - Events by type: {} types", storage_stats.events_by_type.len());
    println!("    - Events by category: {} categories", storage_stats.events_by_category.len());
    println!("    - Events by priority: {} priorities", storage_stats.events_by_priority.len());
    println!("    - Storage size: {} bytes", storage_stats.storage_size_bytes);
    println!("    - Active streams: {}", storage_stats.active_streams);
    
    if let (Some(oldest), Some(newest)) = (storage_stats.oldest_event_timestamp, storage_stats.newest_event_timestamp) {
        println!("    - Time range: {} to {}", oldest.format("%Y-%m-%d %H:%M:%S"), newest.format("%Y-%m-%d %H:%M:%S"));
    }
    
    println!("    - Query performance:");
    println!("      - Average query time: {:.2} ms", storage_stats.query_performance.avg_query_time_ms);
    println!("      - P95 query time: {:.2} ms", storage_stats.query_performance.p95_query_time_ms);
    println!("      - P99 query time: {:.2} ms", storage_stats.query_performance.p99_query_time_ms);
    println!("      - Total queries: {}", storage_stats.query_performance.total_queries);
    println!("      - Cache hit rate: {:.2}%", storage_stats.query_performance.cache_hit_rate * 100.0);
    
    // Index statistics
    let index_stats = index_system.get_stats();
    println!("  🗂️  Index Statistics:");
    println!("    - Total events indexed: {}", index_stats.total_events_indexed);
    println!("    - Memory usage: {:.2} MB", index_stats.index_memory_usage_mb);
    println!("    - Index sizes:");
    println!("      - Timestamp: {}", index_stats.timestamp_index_size);
    println!("      - Event type: {}", index_stats.event_type_index_size);
    println!("      - Category: {}", index_stats.category_index_size);
    println!("      - Agent: {}", index_stats.agent_index_size);
    println!("      - Transaction: {}", index_stats.transaction_index_size);
    println!("      - Path: {}", index_stats.path_index_size);
    
    // WebSocket statistics
    let ws_stats = ws_manager.get_connection_stats().await;
    println!("  🔌 WebSocket Statistics:");
    println!("    - Active connections: {}", ws_manager.get_active_connection_count().await);
    println!("    - Connection details: {} tracked", ws_stats.len());
    
    println!("  ✅ Statistics display completed");
    Ok(())
}

/// Demonstrate API response formats
async fn demonstrate_api_responses() -> SemanticResult<()> {
    // Success response
    let data = vec!["item1", "item2", "item3"];
    let success_response = ApiResponse::success(data.clone());
    
    println!("  📋 Success Response:");
    println!("    - Success: {}", success_response.success);
    println!("    - Data: {:?}", success_response.data);
    println!("    - Request ID: {}", success_response.request_id);
    println!("    - Timestamp: {}", success_response.timestamp);
    
    // Error response
    let error_response: ApiResponse<Vec<String>> = ApiResponse::error("Example error message".to_string());
    
    println!("  📋 Error Response:");
    println!("    - Success: {}", error_response.success);
    println!("    - Error: {:?}", error_response.error);
    println!("    - Request ID: {}", error_response.request_id);
    println!("    - Timestamp: {}", error_response.timestamp);
    
    // JSON serialization
    let json_success = serde_json::to_string_pretty(&success_response).unwrap();
    println!("  📋 JSON Success Response:");
    println!("{}", json_success);
    
    let json_error = serde_json::to_string_pretty(&error_response).unwrap();
    println!("  📋 JSON Error Response:");
    println!("{}", json_error);
    
    println!("  ✅ API response format demonstration completed");
    Ok(())
}

/// Demonstrate WebSocket streaming capabilities
async fn demonstrate_websocket_streaming(ws_manager: &WebSocketConnectionManager) -> SemanticResult<()> {
    // Create a sample subscription
    let subscription = StreamSubscription {
        subscription_id: Uuid::new_v4(),
        agent_id: "example_agent".to_string(),
        filter: EventFilter {
            event_types: Some(vec![SemanticEventType::FilesystemCreate, SemanticEventType::FilesystemWrite]),
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
    
    println!("  🌊 Sample Stream Subscription:");
    println!("    - Subscription ID: {}", subscription.subscription_id);
    println!("    - Agent ID: {}", subscription.agent_id);
    println!("    - Buffer size: {}", subscription.buffer_size);
    println!("    - Include historical: {}", subscription.include_historical);
    println!("    - Historical limit: {:?}", subscription.historical_limit);
    
    // Create sample WebSocket messages
    let ping_message = WebSocketMessage::Ping {
        timestamp: chrono::Utc::now(),
    };
    
    let subscribe_message = WebSocketMessage::Subscribe {
        subscription: subscription.clone(),
    };
    
    let error_message = WebSocketMessage::Error {
        error: "Example error message".to_string(),
        code: 400,
    };
    
    println!("  🌊 Sample WebSocket Messages:");
    
    // Serialize messages to JSON
    let ping_json = serde_json::to_string_pretty(&ping_message).unwrap();
    println!("    📨 Ping Message:");
    println!("{}", ping_json);
    
    let subscribe_json = serde_json::to_string_pretty(&subscribe_message).unwrap();
    println!("    📨 Subscribe Message:");
    println!("{}", subscribe_json);
    
    let error_json = serde_json::to_string_pretty(&error_message).unwrap();
    println!("    📨 Error Message:");
    println!("{}", error_json);
    
    // Simulate event streaming
    println!("  🌊 Simulating event streaming...");
    for i in 1..=3 {
        let event = create_sample_event(
            100 + i,
            SemanticEventType::FilesystemWrite,
            "streaming_agent",
            &format!("/stream/file_{}.txt", i),
        );
        
        ws_manager.broadcast_event(event.clone()).await?;
        
        let stream_message = StreamEventMessage {
            subscription_id: subscription.subscription_id,
            event,
            sequence_number: i,
            timestamp: chrono::Utc::now(),
        };
        
        let ws_event_message = WebSocketMessage::Event {
            event: stream_message,
        };
        
        println!("    📡 Broadcasted event {}: {:?}", i, ws_event_message);
        sleep(Duration::from_millis(100)).await; // Simulate real-time streaming
    }
    
    println!("  ✅ WebSocket streaming demonstration completed");
    Ok(())
}