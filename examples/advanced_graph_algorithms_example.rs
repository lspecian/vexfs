/*
 * VexFS v2.0 - Advanced Graph Algorithms and Semantic Reasoning Example
 * 
 * This example demonstrates the advanced graph algorithms and semantic reasoning
 * capabilities implemented in Task 20, including Dijkstra's shortest path,
 * Louvain community detection, and event-driven semantic reasoning.
 */

use std::sync::Arc;
use std::collections::HashMap;

#[cfg(feature = "advanced_graph_algorithms")]
use vexfs::vexgraph::{
    VexGraph, VexGraphConfig, NodeType, EdgeType, PropertyType,
    DijkstraParams, LouvainParams, MultiGraphParams,
    InferenceRule, Condition, Conclusion, Argument, ConditionType,
    SemanticEvent, SemanticEventType,
};

#[cfg(feature = "advanced_graph_algorithms")]
use vexfs::semantic_api::{
    event_emission::SemanticEventContext,
    types::{SemanticContext, GraphContext, VectorContext, AgentContext},
};

#[cfg(feature = "std")]
use tokio;

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();
    
    println!("🚀 VexFS Advanced Graph Algorithms and Semantic Reasoning Example");
    println!("================================================================");
    
    // Create VexGraph instance with advanced algorithms enabled
    let config = VexGraphConfig {
        api_server: false, // Disable API server for this example
        semantic_integration: true,
        journal_integration: true,
        ..Default::default()
    };
    
    let vexgraph = VexGraph::new(config).await?;
    vexgraph.start().await?;
    
    // Demonstrate advanced graph algorithms
    demonstrate_dijkstra_algorithm(&vexgraph).await?;
    demonstrate_louvain_community_detection(&vexgraph).await?;
    demonstrate_multi_graph_analysis(&vexgraph).await?;
    
    // Demonstrate semantic reasoning
    demonstrate_semantic_reasoning(&vexgraph).await?;
    demonstrate_event_driven_reasoning(&vexgraph).await?;
    
    // Show statistics
    show_system_statistics(&vexgraph).await?;
    
    // Clean up
    vexgraph.stop().await?;
    
    println!("\n✅ Advanced Graph Algorithms and Semantic Reasoning example completed successfully!");
    Ok(())
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn demonstrate_dijkstra_algorithm(vexgraph: &VexGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Demonstrating Dijkstra's Shortest Path Algorithm");
    println!("==================================================");
    
    // Create a sample graph with weighted edges
    let nodes = create_sample_graph_nodes(vexgraph).await?;
    let edges = create_sample_graph_edges(vexgraph, &nodes).await?;
    
    println!("Created sample graph with {} nodes and {} edges", nodes.len(), edges.len());
    
    // Execute Dijkstra's algorithm
    let dijkstra_params = DijkstraParams {
        source: nodes[0],
        target: Some(nodes[4]),
        max_distance: Some(100.0),
        edge_weight_property: Some("weight".to_string()),
        use_parallel: false,
    };
    
    println!("Executing Dijkstra's algorithm from node {} to node {}", nodes[0], nodes[4]);
    
    match vexgraph.advanced_algorithms.dijkstra_shortest_path(dijkstra_params).await {
        Ok(result) => {
            if let vexfs::vexgraph::AlgorithmResult::ShortestPath { path, total_weight, edge_weights } = result {
                println!("✅ Shortest path found:");
                println!("   Path: {:?}", path);
                println!("   Total weight: {:.2}", total_weight);
                println!("   Edge weights: {:?}", edge_weights);
            }
        },
        Err(e) => println!("❌ Dijkstra's algorithm failed: {:?}", e),
    }
    
    // Test parallel Dijkstra
    let parallel_params = DijkstraParams {
        source: nodes[0],
        target: None, // Find shortest paths to all nodes
        max_distance: None,
        edge_weight_property: None,
        use_parallel: true,
    };
    
    println!("\nExecuting parallel Dijkstra's algorithm from node {}", nodes[0]);
    
    match vexgraph.advanced_algorithms.dijkstra_shortest_path(parallel_params).await {
        Ok(result) => {
            if let vexfs::vexgraph::AlgorithmResult::ShortestPath { path, .. } = result {
                println!("✅ Parallel Dijkstra completed, found paths to {} nodes", path.len());
            }
        },
        Err(e) => println!("❌ Parallel Dijkstra failed: {:?}", e),
    }
    
    Ok(())
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn demonstrate_louvain_community_detection(vexgraph: &VexGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🏘️  Demonstrating Louvain Community Detection");
    println!("============================================");
    
    // Execute Louvain community detection
    let louvain_params = LouvainParams {
        resolution: 1.0,
        max_iterations: 100,
        tolerance: 0.01,
        use_parallel: false,
        edge_weight_property: Some("weight".to_string()),
    };
    
    println!("Executing Louvain community detection with resolution {}", louvain_params.resolution);
    
    match vexgraph.advanced_algorithms.louvain_community_detection(louvain_params).await {
        Ok(result) => {
            if let vexfs::vexgraph::AlgorithmResult::CommunityDetection { communities, modularity, num_communities } = result {
                println!("✅ Community detection completed:");
                println!("   Number of communities: {}", num_communities);
                println!("   Modularity score: {:.4}", modularity);
                println!("   Community assignments: {} nodes", communities.len());
                
                // Show community distribution
                let mut community_sizes = HashMap::new();
                for &community in communities.values() {
                    *community_sizes.entry(community).or_insert(0) += 1;
                }
                
                println!("   Community sizes:");
                for (community, size) in community_sizes {
                    println!("     Community {}: {} nodes", community, size);
                }
            }
        },
        Err(e) => println!("❌ Louvain community detection failed: {:?}", e),
    }
    
    // Test parallel Louvain
    let parallel_params = LouvainParams {
        resolution: 0.8,
        max_iterations: 50,
        tolerance: 0.05,
        use_parallel: true,
        edge_weight_property: None,
    };
    
    println!("\nExecuting parallel Louvain with resolution {}", parallel_params.resolution);
    
    match vexgraph.advanced_algorithms.louvain_community_detection(parallel_params).await {
        Ok(result) => {
            if let vexfs::vexgraph::AlgorithmResult::CommunityDetection { num_communities, modularity, .. } = result {
                println!("✅ Parallel Louvain completed: {} communities, modularity: {:.4}", num_communities, modularity);
            }
        },
        Err(e) => println!("❌ Parallel Louvain failed: {:?}", e),
    }
    
    Ok(())
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn demonstrate_multi_graph_analysis(vexgraph: &VexGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔗 Demonstrating Multi-Graph Analysis");
    println!("====================================");
    
    // Analyze different edge types
    let multi_graph_params = MultiGraphParams {
        edge_types: vec![
            "Contains".to_string(),
            "References".to_string(),
            "Similar".to_string(),
            "Semantic".to_string(),
        ],
        analyze_connectivity: true,
        compute_density: true,
        parallel_analysis: true,
    };
    
    println!("Analyzing multi-graph structure for {} edge types", multi_graph_params.edge_types.len());
    
    match vexgraph.advanced_algorithms.analyze_multi_graph(multi_graph_params).await {
        Ok(result) => {
            if let vexfs::vexgraph::AlgorithmResult::MultiGraphAnalysis { edge_types, density_by_type, connectivity_metrics } = result {
                println!("✅ Multi-graph analysis completed:");
                
                println!("   Edge type distribution:");
                for (edge_type, count) in edge_types {
                    println!("     {}: {} edges", edge_type, count);
                }
                
                println!("   Graph density by type:");
                for (edge_type, density) in density_by_type {
                    println!("     {}: {:.4}", edge_type, density);
                }
                
                println!("   Connectivity metrics:");
                for (edge_type, connectivity) in connectivity_metrics {
                    println!("     {}: {:.4}", edge_type, connectivity);
                }
            }
        },
        Err(e) => println!("❌ Multi-graph analysis failed: {:?}", e),
    }
    
    Ok(())
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn demonstrate_semantic_reasoning(vexgraph: &VexGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧠 Demonstrating Semantic Reasoning Engine");
    println!("==========================================");
    
    // Add custom inference rules
    let connectivity_rule = InferenceRule {
        id: "high_connectivity_rule".to_string(),
        name: "High Connectivity Inference".to_string(),
        description: "Infer important nodes based on high connectivity".to_string(),
        conditions: vec![
            Condition {
                predicate: "node_degree".to_string(),
                arguments: vec![
                    Argument::Variable("node".to_string()),
                    Argument::Variable("degree".to_string()),
                ],
                condition_type: ConditionType::GreaterThan,
                negated: false,
            },
            Condition {
                predicate: "threshold".to_string(),
                arguments: vec![
                    Argument::Variable("degree".to_string()),
                    Argument::Constant(PropertyType::Integer(5)),
                ],
                condition_type: ConditionType::GreaterThan,
                negated: false,
            },
        ],
        conclusions: vec![
            Conclusion {
                predicate: "important_node".to_string(),
                arguments: vec![Argument::Variable("node".to_string())],
                confidence: 0.8,
                temporal_validity: None,
            }
        ],
        priority: 90,
        confidence: 0.8,
        metadata: HashMap::new(),
    };
    
    println!("Adding custom inference rule: {}", connectivity_rule.name);
    
    match vexgraph.semantic_reasoning.add_rule(connectivity_rule).await {
        Ok(_) => println!("✅ Inference rule added successfully"),
        Err(e) => println!("❌ Failed to add inference rule: {:?}", e),
    }
    
    // Execute forward chaining inference
    println!("\nExecuting forward chaining inference...");
    
    match vexgraph.semantic_reasoning.forward_chaining_inference().await {
        Ok(result) => {
            println!("✅ Forward chaining completed:");
            println!("   Inferred facts: {}", result.facts.len());
            println!("   Applied rules: {:?}", result.applied_rules);
            println!("   Inference time: {} ms", result.inference_time_ms);
            println!("   Overall confidence: {:.4}", result.overall_confidence);
            
            // Show some inferred facts
            for (i, fact) in result.facts.iter().take(5).enumerate() {
                println!("   Fact {}: {} (confidence: {:.2})", i + 1, fact.predicate, fact.confidence);
            }
        },
        Err(e) => println!("❌ Forward chaining failed: {:?}", e),
    }
    
    Ok(())
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn demonstrate_event_driven_reasoning(vexgraph: &VexGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Demonstrating Event-Driven Reasoning");
    println!("======================================");
    
    // Create sample semantic events
    let events = vec![
        create_sample_semantic_event(SemanticEventType::GraphNodeCreate, 1001, "test_agent"),
        create_sample_semantic_event(SemanticEventType::GraphEdgeCreate, 2001, "test_agent"),
        create_sample_semantic_event(SemanticEventType::VectorSearch, 3001, "search_agent"),
        create_sample_semantic_event(SemanticEventType::FilesystemCreate, 4001, "file_agent"),
    ];
    
    println!("Processing {} semantic events for reasoning...", events.len());
    
    for (i, event) in events.iter().enumerate() {
        println!("Processing event {}: {:?}", i + 1, event.event_type);
        
        match vexgraph.semantic_reasoning.handle_event(event.clone()).await {
            Ok(_) => println!("  ✅ Event processed successfully"),
            Err(e) => println!("  ❌ Event processing failed: {:?}", e),
        }
    }
    
    // Execute inference after events
    println!("\nExecuting inference after event processing...");
    
    match vexgraph.semantic_reasoning.forward_chaining_inference().await {
        Ok(result) => {
            println!("✅ Event-driven inference completed:");
            println!("   New inferred facts: {}", result.facts.len());
            println!("   Applied rules: {:?}", result.applied_rules);
            
            // Show event-specific facts
            let event_facts: Vec<_> = result.facts.iter()
                .filter(|f| f.predicate.contains("event") || f.predicate.contains("recent"))
                .collect();
            
            println!("   Event-related facts: {}", event_facts.len());
            for fact in event_facts.iter().take(3) {
                println!("     {}: {:?} (confidence: {:.2})", fact.predicate, fact.arguments, fact.confidence);
            }
        },
        Err(e) => println!("❌ Event-driven inference failed: {:?}", e),
    }
    
    Ok(())
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn show_system_statistics(vexgraph: &VexGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📈 System Statistics");
    println!("===================");
    
    // Get advanced algorithms statistics
    match vexgraph.advanced_algorithms.get_statistics().await {
        Ok(stats) => {
            println!("Advanced Algorithms Statistics:");
            println!("  Dijkstra executions: {}", stats.dijkstra_executions);
            println!("  Louvain executions: {}", stats.louvain_executions);
            println!("  Community detections: {}", stats.community_detections);
            println!("  Multi-graph operations: {}", stats.multi_graph_operations);
            println!("  Cache hits: {}", stats.cache_hits);
            println!("  Cache misses: {}", stats.cache_misses);
            println!("  Average execution time: {:.2} ms", stats.average_execution_time_ms);
        },
        Err(e) => println!("❌ Failed to get advanced algorithms statistics: {:?}", e),
    }
    
    // Get semantic reasoning statistics
    match vexgraph.semantic_reasoning.get_statistics().await {
        Ok(stats) => {
            println!("\nSemantic Reasoning Statistics:");
            println!("  Total inferences: {}", stats.total_inferences);
            println!("  Rules applied: {}", stats.rules_applied);
            println!("  Facts inferred: {}", stats.facts_inferred);
            println!("  Event-triggered inferences: {}", stats.event_triggered_inferences);
            println!("  Cache hits: {}", stats.cache_hits);
            println!("  Cache misses: {}", stats.cache_misses);
            println!("  Average inference time: {:.2} ms", stats.average_inference_time_ms);
        },
        Err(e) => println!("❌ Failed to get semantic reasoning statistics: {:?}", e),
    }
    
    // Get overall system statistics
    match vexgraph.get_statistics().await {
        Ok(stats) => {
            println!("\nOverall System Statistics:");
            println!("  Core nodes: {}", stats.core.node_count);
            println!("  Core edges: {}", stats.core.edge_count);
            println!("  Total traversals: {}", stats.traversal.total_traversals);
            println!("  Memory usage: {} bytes", stats.core.memory_usage);
            println!("  Cache hit rate: {:.2}%", stats.core.cache_hit_rate * 100.0);
        },
        Err(e) => println!("❌ Failed to get system statistics: {:?}", e),
    }
    
    Ok(())
}

// Helper functions

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn create_sample_graph_nodes(vexgraph: &VexGraph) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let mut nodes = Vec::new();
    
    // Create 10 sample nodes
    for i in 0..10 {
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), PropertyType::String(format!("Node_{}", i)));
        properties.insert("type".to_string(), PropertyType::String("sample".to_string()));
        properties.insert("value".to_string(), PropertyType::Integer(i as i64));
        
        match vexgraph.core.create_node(NodeType::Custom, properties).await {
            Ok(node_id) => nodes.push(node_id),
            Err(e) => println!("Failed to create node {}: {:?}", i, e),
        }
    }
    
    Ok(nodes)
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
async fn create_sample_graph_edges(vexgraph: &VexGraph, nodes: &[u64]) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let mut edges = Vec::new();
    
    // Create edges with different weights and types
    let edge_configs = vec![
        (0, 1, 2.5, EdgeType::Contains),
        (1, 2, 1.8, EdgeType::References),
        (2, 3, 3.2, EdgeType::Similar),
        (3, 4, 1.5, EdgeType::Semantic),
        (4, 5, 2.1, EdgeType::Contains),
        (5, 6, 1.9, EdgeType::References),
        (6, 7, 2.8, EdgeType::Similar),
        (7, 8, 1.3, EdgeType::Semantic),
        (8, 9, 2.4, EdgeType::Contains),
        (9, 0, 3.1, EdgeType::References), // Create a cycle
        (1, 5, 4.2, EdgeType::Custom),     // Cross connections
        (3, 7, 3.8, EdgeType::Custom),
    ];
    
    for (source_idx, target_idx, weight, edge_type) in edge_configs {
        if source_idx < nodes.len() && target_idx < nodes.len() {
            let mut properties = HashMap::new();
            properties.insert("weight".to_string(), PropertyType::Float(weight));
            properties.insert("type".to_string(), PropertyType::String(format!("{:?}", edge_type)));
            
            match vexgraph.core.create_edge(
                nodes[source_idx],
                nodes[target_idx],
                edge_type,
                weight,
                properties,
            ).await {
                Ok(edge_id) => edges.push(edge_id),
                Err(e) => println!("Failed to create edge {}->{}): {:?}", source_idx, target_idx, e),
            }
        }
    }
    
    Ok(edges)
}

#[cfg(all(feature = "std", feature = "advanced_graph_algorithms"))]
fn create_sample_semantic_event(event_type: SemanticEventType, event_id: u64, agent_id: &str) -> SemanticEvent {
    SemanticEvent {
        event_id,
        event_type,
        agent_id: agent_id.to_string(),
        timestamp: vexfs::semantic_api::types::SemanticTimestamp {
            seconds: chrono::Utc::now().timestamp() as u64,
            nanoseconds: 0,
        },
        session_id: Some("example_session".to_string()),
        transaction_id: Some("example_transaction".to_string()),
        causality_id: Some("example_causality".to_string()),
        flags: vexfs::semantic_api::types::EventFlags {
            priority: vexfs::semantic_api::types::EventPriority::Normal,
            category: vexfs::semantic_api::types::EventCategory::Graph,
            requires_response: false,
            is_synthetic: false,
            is_replay: false,
        },
        context: SemanticEventContext {
            filesystem: None,
            graph: Some(GraphContext {
                node_id: Some(event_id),
                edge_id: if matches!(event_type, SemanticEventType::GraphEdgeCreate) { Some(event_id + 1000) } else { None },
                operation_type: Some(format!("{:?}", event_type)),
                properties: Some("{}".to_string()),
            }),
            vector: if matches!(event_type, SemanticEventType::VectorSearch) {
                Some(VectorContext {
                    collection_name: Some("test_collection".to_string()),
                    vector_id: Some(event_id),
                    operation_type: Some("search".to_string()),
                    metadata: Some("{}".to_string()),
                })
            } else { None },
            agent: Some(AgentContext {
                agent_type: "example_agent".to_string(),
                capabilities: vec!["reasoning".to_string(), "analysis".to_string()],
                session_context: Some("{}".to_string()),
            }),
            system: None,
            semantic: None,
            observability: None,
        },
    }
}

// Fallback main for when advanced_graph_algorithms feature is not enabled
#[cfg(not(all(feature = "std", feature = "advanced_graph_algorithms")))]
fn main() {
    println!("❌ This example requires the 'std' and 'advanced_graph_algorithms' features to be enabled.");
    println!("   Run with: cargo run --example advanced_graph_algorithms_example --features=\"std,advanced_graph_algorithms\"");
}