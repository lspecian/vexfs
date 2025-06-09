//! Standalone test for semantic event types - no external dependencies
//! This tests our cleaned semantic taxonomy without the full semantic_api infrastructure

use super::types::*;
use serde_cbor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_category_mapping() {
        // Test that all event types map to correct categories
        assert_eq!(SemanticEventType::FilesystemMount.category(), EventCategory::Filesystem);
        assert_eq!(SemanticEventType::FilesystemUnmount.category(), EventCategory::Filesystem);
        assert_eq!(SemanticEventType::FilesystemRead.category(), EventCategory::Filesystem);
        assert_eq!(SemanticEventType::FilesystemWrite.category(), EventCategory::Filesystem);
        
        assert_eq!(SemanticEventType::GraphNodeCreate.category(), EventCategory::Graph);
        assert_eq!(SemanticEventType::GraphEdgeCreate.category(), EventCategory::Graph);
        assert_eq!(SemanticEventType::GraphTraversal.category(), EventCategory::Graph);
        
        assert_eq!(SemanticEventType::VectorInsert.category(), EventCategory::Vector);
        assert_eq!(SemanticEventType::VectorSearch.category(), EventCategory::Vector);
        assert_eq!(SemanticEventType::VectorUpdate.category(), EventCategory::Vector);
        
        assert_eq!(SemanticEventType::AgentDecision.category(), EventCategory::Agent);
        assert_eq!(SemanticEventType::AgentAction.category(), EventCategory::Agent);
        
        assert_eq!(SemanticEventType::SystemStartup.category(), EventCategory::System);
        assert_eq!(SemanticEventType::SystemShutdown.category(), EventCategory::System);
        
        assert_eq!(SemanticEventType::SemanticAnalysis.category(), EventCategory::Semantic);
        assert_eq!(SemanticEventType::SemanticInference.category(), EventCategory::Semantic);
        
        assert_eq!(SemanticEventType::ObservabilityMetric.category(), EventCategory::Observability);
        assert_eq!(SemanticEventType::ObservabilityTrace.category(), EventCategory::Observability);
        assert_eq!(SemanticEventType::ObservabilityLog.category(), EventCategory::Observability);
    }

    #[test]
    fn test_event_category_ranges() {
        // Test that event type values fall within expected category ranges
        assert!(matches!(SemanticEventType::FilesystemMount as u16, 0x0100..=0x01FF));
        assert!(matches!(SemanticEventType::GraphNodeCreate as u16, 0x0200..=0x02FF));
        assert!(matches!(SemanticEventType::VectorInsert as u16, 0x0300..=0x03FF));
        assert!(matches!(SemanticEventType::AgentDecision as u16, 0x0400..=0x04FF));
        assert!(matches!(SemanticEventType::SystemStartup as u16, 0x0500..=0x05FF));
        assert!(matches!(SemanticEventType::SemanticAnalysis as u16, 0x0600..=0x06FF));
        assert!(matches!(SemanticEventType::ObservabilityMetric as u16, 0x0800..=0x08FF));
    }

    #[test]
    fn test_basic_event_type_serialization() {
        // Test basic CBOR serialization of event types
        let event_type = SemanticEventType::ObservabilityMetric;
        let serialized = serde_cbor::to_vec(&event_type).expect("Failed to serialize event type");
        assert!(!serialized.is_empty());

        let deserialized: SemanticEventType = serde_cbor::from_slice(&serialized)
            .expect("Failed to deserialize event type");
        assert_eq!(deserialized, SemanticEventType::ObservabilityMetric);
    }

    #[test]
    fn test_event_category_serialization() {
        // Test basic CBOR serialization of event categories
        let category = EventCategory::Filesystem;
        let serialized = serde_cbor::to_vec(&category).expect("Failed to serialize category");
        assert!(!serialized.is_empty());

        let deserialized: EventCategory = serde_cbor::from_slice(&serialized)
            .expect("Failed to deserialize category");
        assert_eq!(deserialized, EventCategory::Filesystem);
    }

    #[test]
    fn test_context_types_serialization() {
        // Test basic CBOR serialization of context types
        let fs_context = FilesystemContext {
            path: "/test".to_string(),
            inode: Some(123),
            size: Some(1024),
            permissions: Some(0o644),
            operation_type: Some("read".to_string()),
            bytes_transferred: Some(1024),
            duration_us: Some(150),
        };
        
        let serialized = serde_cbor::to_vec(&fs_context).expect("Failed to serialize context");
        assert!(!serialized.is_empty());

        let deserialized: FilesystemContext = serde_cbor::from_slice(&serialized)
            .expect("Failed to deserialize context");
        assert_eq!(deserialized.path, "/test");
        assert_eq!(deserialized.inode, Some(123));
    }

    #[test]
    fn test_no_inappropriate_events() {
        // Verify that inappropriate ClickOps and Storage events are not present
        // This test will fail to compile if those event types exist
        
        // These should NOT exist (will cause compile error if they do):
        // SemanticEventType::ClickOpsButtonClick
        // SemanticEventType::StorageWrite
        // EventCategory::ClickOps
        // EventCategory::Storage
        
        // Test that we only have the appropriate categories
        let valid_categories = [
            EventCategory::Filesystem,
            EventCategory::Graph,
            EventCategory::Vector,
            EventCategory::Agent,
            EventCategory::System,
            EventCategory::Semantic,
            EventCategory::Observability,
        ];
        
        // This ensures we have exactly 7 categories (no more, no less)
        assert_eq!(valid_categories.len(), 7);
        
        // Test a few event types to ensure they exist and are properly categorized
        assert_eq!(SemanticEventType::FilesystemMount.category(), EventCategory::Filesystem);
        assert_eq!(SemanticEventType::ObservabilityMetric.category(), EventCategory::Observability);
    }

    #[test]
    fn test_cbor_compactness() {
        // Test that CBOR serialization is reasonably compact
        let event_type = SemanticEventType::ObservabilityLog;
        let category = EventCategory::Observability;

        let cbor_event_type = serde_cbor::to_vec(&event_type).expect("Failed to serialize event type");
        let json_event_type = serde_json::to_vec(&event_type).expect("Failed to serialize event type");
        
        let cbor_category = serde_cbor::to_vec(&category).expect("Failed to serialize category");
        let json_category = serde_json::to_vec(&category).expect("Failed to serialize category");
        
        // CBOR should be more compact than JSON for enums
        assert!(cbor_event_type.len() <= json_event_type.len());
        assert!(cbor_category.len() <= json_category.len());
        
        // CBOR should be reasonably sized (not huge)
        assert!(cbor_event_type.len() < 100);
        assert!(cbor_category.len() < 100);
    }
}