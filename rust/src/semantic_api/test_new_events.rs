//! Test module for new semantic event types serialization
//! 
//! This module tests CBOR serialization/deserialization of the newly added
//! Observability event types.

#[cfg(test)]
mod tests {
    use super::super::types::*;
    // Note: serialization module not available, using direct serde
    use serde::{Serialize, Deserialize};
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_observability_event_cbor_serialization() {
        // Create an Observability event
        let observability_context = ObservabilityContext {
            metric_name: Some("vexfs.collections.delete_latency".to_string()),
            metric_value: Some(45.7),
            metric_unit: Some("milliseconds".to_string()),
            log_level: None,
            log_message: None,
            trace_id: Some("trace-abc123".to_string()),
            span_id: Some("span-def456".to_string()),
            parent_span_id: Some("span-parent789".to_string()),
            service_name: Some("vexfs-dashboard".to_string()),
            operation_name: Some("delete_collection".to_string()),
            resource_type: Some("collection".to_string()),
            threshold_value: Some(100.0),
            alert_severity: None,
        };

        let semantic_context = SemanticContext {
            transaction_id: Some(54321),
            session_id: None,
            causality_chain_id: Some(222),
            filesystem: None,
            graph: None,
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: Some(observability_context),
        };

        let event = SemanticEvent {
            event_id: 2001,
            event_type: SemanticEventType::ObservabilityMetricCollected,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: 2,
                cpu_id: 1,
                process_id: 5678,
            },
            global_sequence: 2001,
            local_sequence: 2,
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: false,
                agent_visible: true,
                deterministic: false,
                compressed: false,
                indexed: true,
                replicated: true,
            },
            priority: EventPriority::Low,
            event_size: 0, // Will be calculated
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: vec![],
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 70,
            replay_priority: 2,
            context: semantic_context,
            payload: None,
            metadata: None,
        };

        // Test CBOR serialization
        let config = SerializationConfig {
            format: SerializationFormat::Cbor,
            compression: CompressionType::None,
            compression_threshold: 1024,
            pretty_print: false,
            include_metadata: true,
        };

        let serializer = SemanticSerializer::new(config);
        
        // Serialize
        let serialized = serializer.serialize(&event).expect("Failed to serialize Observability event");
        assert!(!serialized.is_empty(), "Serialized data should not be empty");

        // Deserialize
        let deserialized: SemanticEvent = serializer.deserialize(&serialized)
            .expect("Failed to deserialize Observability event");

        // Verify event type
        assert_eq!(deserialized.event_type, SemanticEventType::ObservabilityMetricCollected);
        assert_eq!(deserialized.event_type.category(), EventCategory::Observability);

        // Verify context
        assert!(deserialized.context.observability.is_some());
        let obs_ctx = deserialized.context.observability.unwrap();
        assert_eq!(obs_ctx.metric_name, Some("vexfs.collections.delete_latency".to_string()));
        assert_eq!(obs_ctx.metric_value, Some(45.7));
        assert_eq!(obs_ctx.service_name, Some("vexfs-dashboard".to_string()));
        assert_eq!(obs_ctx.threshold_value, Some(100.0));
    }

    #[test]
    fn test_all_observability_event_types_serialization() {
        let config = SerializationConfig {
            format: SerializationFormat::Cbor,
            compression: CompressionType::None,
            compression_threshold: 1024,
            pretty_print: false,
            include_metadata: true,
        };

        let serializer = SemanticSerializer::new(config);

        // Test all Observability event types
        let observability_events = vec![
            SemanticEventType::ObservabilityMetricCollected,
            SemanticEventType::ObservabilityLogGenerated,
            SemanticEventType::ObservabilityTraceSpanStart,
            SemanticEventType::ObservabilityTraceSpanEnd,
            SemanticEventType::ObservabilityAlertTriggered,
            SemanticEventType::ObservabilityHealthCheck,
            SemanticEventType::ObservabilityPerformanceCounter,
            SemanticEventType::ObservabilityErrorReported,
            SemanticEventType::ObservabilityAuditEvent,
            SemanticEventType::ObservabilitySystemStatus,
            SemanticEventType::ObservabilityResourceUsage,
            SemanticEventType::ObservabilityThreshold,
        ];

        // Test serialization of all event types
        for (i, event_type) in observability_events.iter().enumerate() {
            let event = create_test_event(*event_type, i as u64 + 4000);
            let serialized = serializer.serialize(&event)
                .expect(&format!("Failed to serialize {:?}", event_type));
            let deserialized: SemanticEvent = serializer.deserialize(&serialized)
                .expect(&format!("Failed to deserialize {:?}", event_type));
            assert_eq!(deserialized.event_type, *event_type);
            assert_eq!(deserialized.event_type.category(), EventCategory::Observability);
        }
    }

    fn create_test_event(event_type: SemanticEventType, event_id: u64) -> SemanticEvent {
        let context = SemanticContext {
            transaction_id: Some(event_id),
            session_id: None,
            causality_chain_id: None,
            filesystem: None,
            graph: None,
            vector: None,
            agent: None,
            system: None,
            semantic: None,
            observability: Some(ObservabilityContext {
                metric_name: Some("test_metric".to_string()),
                metric_value: Some(42.0),
                metric_unit: Some("count".to_string()),
                log_level: None,
                log_message: None,
                trace_id: None,
                span_id: None,
                parent_span_id: None,
                service_name: Some("vexfs".to_string()),
                operation_name: None,
                resource_type: None,
                threshold_value: None,
                alert_severity: None,
            }),
        };

        SemanticEvent {
            event_id,
            event_type,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: 1,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: event_id,
            local_sequence: 1,
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: false,
                agent_visible: true,
                deterministic: false,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::Normal,
            event_size: 0,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: vec![],
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 50,
            replay_priority: 1,
            context,
            payload: None,
            metadata: None,
        }
    }
}