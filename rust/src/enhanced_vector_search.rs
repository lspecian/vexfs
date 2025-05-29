//! Enhanced Vector Search Engine with Advanced OperationContext Integration
//!
//! This module demonstrates the integration of EnhancedOperationContext with vector search
//! operations, providing comprehensive monitoring, telemetry, and lifecycle management.

extern crate alloc;
use alloc::{vec::Vec, string::String, format, sync::Arc};

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::fs_core::enhanced_operation_context::{
    EnhancedOperationContext, OperationType, TelemetryEventType, TelemetrySeverity,
    MemoryAllocationType, CancellationReason, LifecycleEvent, LifecycleEventType
};
use crate::vector_search::{VectorSearchEngine, SearchQuery};
use crate::anns::SearchResult;
use crate::vector_storage::VectorStorageManager;
use crate::knn_search::KnnResult;
use crate::shared::types::InodeNumber;

/// Enhanced search result with comprehensive metadata
#[derive(Debug, Clone)]
pub struct EnhancedSearchResult {
    /// Search results
    pub results: Vec<KnnResult>,
    /// Total candidates examined
    pub total_candidates: u64,
    /// Search time in microseconds
    pub search_time_us: u64,
    /// Index statistics (optional)
    pub index_stats: Option<String>,
}

/// Enhanced vector search engine with comprehensive operation monitoring
pub struct EnhancedVectorSearchEngine {
    /// Base vector search engine
    search_engine: VectorSearchEngine,
    /// Vector storage manager
    storage_manager: Arc<VectorStorageManager>,
}

impl EnhancedVectorSearchEngine {
    /// Create new enhanced vector search engine
    pub fn new(
        search_engine: VectorSearchEngine,
        storage_manager: Arc<VectorStorageManager>,
    ) -> Self {
        Self {
            search_engine,
            storage_manager,
        }
    }

    /// Perform vector search with enhanced operation context
    pub fn search_with_context(
        &self,
        query: &SearchQuery,
        context: &EnhancedOperationContext,
    ) -> VexfsResult<EnhancedSearchResult> {
        // Check for cancellation before starting
        if context.is_cancelled() {
            return Err(VexfsError::OperationCancelled("Search operation was cancelled".to_string()));
        }

        // Check timeout
        context.check_timeout()?;

        // Log operation start
        context.log_telemetry_event(
            TelemetryEventType::OperationStart,
            format!("Starting vector search with {} dimensions", query.vector.len()),
            TelemetrySeverity::Info,
        );

        // Start telemetry span
        let search_span_id = context.start_telemetry_span("vector_search");

        // Track memory allocation for query processing
        let query_memory_size = query.vector.len() * core::mem::size_of::<f32>();
        context.track_memory_allocation(
            query_memory_size,
            MemoryAllocationType::VectorData,
            "query_vector",
        )?;

        // Update progress - preparation phase
        context.update_progress(0.1, Some("query_preparation"))?;

        // Check for cancellation during processing
        if context.is_cancelled() {
            context.track_memory_deallocation(query_memory_size)?;
            context.end_telemetry_span(search_span_id);
            return Err(VexfsError::OperationCancelled("Search operation was cancelled during preparation".to_string()));
        }

        // Perform the actual search
        context.update_progress(0.3, Some("index_search"))?;
        
        // Add telemetry metrics
        context.add_telemetry_metric("query_dimensions", query.vector.len() as f64, "count");
        context.add_telemetry_metric("query_k", query.k as f64, "count");

        // Simulate search execution (in real implementation, this would call the actual search)
        let search_result = self.execute_search_with_monitoring(query, context)?;

        // Update progress - post-processing
        context.update_progress(0.8, Some("result_processing"))?;

        // Track result memory allocation
        let result_memory_size = search_result.results.len() * core::mem::size_of::<KnnResult>();
        context.track_memory_allocation(
            result_memory_size,
            MemoryAllocationType::ResultStorage,
            "search_results",
        )?;

        // Final progress update
        context.update_progress(1.0, Some("completed"))?;

        // Log completion
        context.log_telemetry_event(
            TelemetryEventType::OperationComplete,
            format!("Vector search completed with {} results", search_result.results.len()),
            TelemetrySeverity::Info,
        );

        // Add final metrics
        context.add_telemetry_metric("results_count", search_result.results.len() as f64, "count");
        context.add_telemetry_metric("search_duration_us", context.get_operation_duration_us() as f64, "microseconds");

        // End telemetry span
        context.end_telemetry_span(search_span_id);

        // Clean up memory tracking
        context.track_memory_deallocation(query_memory_size)?;

        Ok(search_result)
    }

    /// Execute search with detailed monitoring
    fn execute_search_with_monitoring(
        &self,
        query: &SearchQuery,
        context: &EnhancedOperationContext,
    ) -> VexfsResult<EnhancedSearchResult> {
        // Start index search span
        let index_span_id = context.start_telemetry_span("index_search_execution");

        // Check for cancellation
        if context.is_cancelled() {
            context.end_telemetry_span(index_span_id);
            return Err(VexfsError::OperationCancelled("Search cancelled during index search".to_string()));
        }

        // Log search parameters
        context.log_telemetry_event(
            TelemetryEventType::Debug,
            format!("Executing search with k={}, dimensions={}", query.k, query.vector.len()),
            TelemetrySeverity::Debug,
        );

        // Simulate the actual search operation
        // In a real implementation, this would delegate to the actual search engine
        let result = EnhancedSearchResult {
            results: Vec::new(), // Placeholder - would contain actual results
            total_candidates: 0,
            search_time_us: context.get_operation_duration_us(),
            index_stats: None,
        };

        // Log search execution details
        context.add_telemetry_metric("index_candidates_examined", result.total_candidates as f64, "count");

        context.end_telemetry_span(index_span_id);

        Ok(result)
    }

    /// Perform batch search with enhanced monitoring
    pub fn batch_search_with_context(
        &self,
        queries: &[SearchQuery],
        context: &EnhancedOperationContext,
    ) -> VexfsResult<Vec<EnhancedSearchResult>> {
        // Check for cancellation
        if context.is_cancelled() {
            return Err(VexfsError::OperationCancelled("Batch search operation was cancelled".to_string()));
        }

        // Log batch operation start
        context.log_telemetry_event(
            TelemetryEventType::OperationStart,
            format!("Starting batch search with {} queries", queries.len()),
            TelemetrySeverity::Info,
        );

        let batch_span_id = context.start_telemetry_span("batch_vector_search");

        // Track memory for batch processing
        let batch_memory_size = queries.len() * core::mem::size_of::<SearchQuery>();
        context.track_memory_allocation(
            batch_memory_size,
            MemoryAllocationType::VectorData,
            "batch_queries",
        )?;

        let mut results: Vec<EnhancedSearchResult> = Vec::with_capacity(queries.len());
        let total_queries = queries.len();

        for (i, query) in queries.iter().enumerate() {
            // Check for cancellation between queries
            if context.is_cancelled() {
                context.track_memory_deallocation(batch_memory_size)?;
                context.end_telemetry_span(batch_span_id);
                return Err(VexfsError::OperationCancelled(
                    format!("Batch search cancelled at query {}/{}", i + 1, total_queries)
                ));
            }

            // Update progress
            let progress = (i as f32) / (total_queries as f32);
            context.update_progress(progress, Some("batch_processing"))?;

            // Execute individual search
            let result = self.execute_search_with_monitoring(query, context)?;
            results.push(result);

            // Add per-query metrics
            context.add_telemetry_metric(
                &format!("query_{}_duration_us", i),
                context.get_operation_duration_us() as f64,
                "microseconds",
            );
        }

        // Final progress
        context.update_progress(1.0, Some("batch_completed"))?;

        // Log completion
        context.log_telemetry_event(
            TelemetryEventType::OperationComplete,
            format!("Batch search completed: {} queries processed", results.len()),
            TelemetrySeverity::Info,
        );

        // Add batch metrics
        context.add_telemetry_metric("batch_size", results.len() as f64, "count");
        context.add_telemetry_metric("batch_duration_us", context.get_operation_duration_us() as f64, "microseconds");

        context.end_telemetry_span(batch_span_id);
        context.track_memory_deallocation(batch_memory_size)?;

        Ok(results)
    }

    /// Get resource usage summary for the search engine
    pub fn get_resource_usage(&self, context: &EnhancedOperationContext) -> crate::fs_core::enhanced_operation_context::ResourceUsageSummary {
        context.get_resource_usage_summary()
    }
}

/// Example lifecycle hook for search operations
pub fn search_lifecycle_hook(
    context: &EnhancedOperationContext,
    event: &LifecycleEvent,
) {
    match event.event_type {
        LifecycleEventType::Start => {
            // Log search operation start
            context.log_telemetry_event(
                TelemetryEventType::Debug,
                "Search operation lifecycle: Started".to_string(),
                TelemetrySeverity::Debug,
            );
        }
        LifecycleEventType::Complete => {
            // Log successful completion
            let duration = context.get_operation_duration_us();
            context.log_telemetry_event(
                TelemetryEventType::PerformanceMilestone,
                format!("Search operation completed in {} microseconds", duration),
                TelemetrySeverity::Info,
            );
        }
        LifecycleEventType::Error => {
            // Log error occurrence
            context.log_telemetry_event(
                TelemetryEventType::Error,
                "Search operation encountered an error".to_string(),
                TelemetrySeverity::Error,
            );
        }
        LifecycleEventType::Cancel => {
            // Log cancellation
            context.log_telemetry_event(
                TelemetryEventType::Warning,
                "Search operation was cancelled".to_string(),
                TelemetrySeverity::Warning,
            );
        }
        LifecycleEventType::ResourceLimit => {
            // Log resource limit hit
            context.log_telemetry_event(
                TelemetryEventType::Warning,
                "Search operation hit resource limits".to_string(),
                TelemetrySeverity::Warning,
            );
        }
        LifecycleEventType::TimeoutWarning => {
            // Log timeout warning
            context.log_telemetry_event(
                TelemetryEventType::Warning,
                "Search operation approaching timeout".to_string(),
                TelemetrySeverity::Warning,
            );
        }
        _ => {
            // Handle other lifecycle events
            context.log_telemetry_event(
                TelemetryEventType::Debug,
                format!("Search operation lifecycle event: {:?}", event.event_type),
                TelemetrySeverity::Debug,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_core::enhanced_operation_context::{EnhancedOperationContext, OperationType};
    use crate::fs_core::operations::OperationContext;
    use crate::fs_core::permissions::UserContext;

    #[test]
    fn test_enhanced_search_context_creation() {
        // Test basic enhanced operation context functionality
        // Note: This test would require proper setup of base context in a real implementation
        
        // Test cancellation token
        use crate::fs_core::enhanced_operation_context::CancellationToken;
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        
        // Test progress reporter
        use crate::fs_core::enhanced_operation_context::ProgressReporter;
        let progress = ProgressReporter::new();
        assert_eq!(progress.get_progress(), 0.0);
    }

    #[test]
    fn test_resource_tracking() {
        // Test resource tracker functionality
        use crate::fs_core::enhanced_operation_context::{ResourceTracker, MemoryAllocationType};
        
        let tracker = ResourceTracker::new();
        let result = tracker.track_memory_allocation(1024, MemoryAllocationType::VectorData, "test");
        
        // Verify that the allocation operation succeeds
        assert!(result.is_ok());
        
        // Test deallocation
        let dealloc_result = tracker.track_memory_deallocation(512);
        assert!(dealloc_result.is_ok());
    }

    #[test]
    fn test_telemetry_collection() {
        // Test telemetry collector
        use crate::fs_core::enhanced_operation_context::{TelemetryCollector, TelemetryEventType, TelemetrySeverity};
        
        let collector = TelemetryCollector::new();
        collector.log_event(
            TelemetryEventType::OperationStart,
            "Test event".to_string(),
            TelemetrySeverity::Info,
        );
        
        // Test span creation
        let span_id = collector.start_span("test_span");
        assert!(span_id > 0);
        
        collector.end_span(span_id);
    }
}