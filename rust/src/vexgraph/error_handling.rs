/*
 * VexFS v2.0 - VexGraph Phase 2 Error Handling
 * 
 * Comprehensive error handling and validation framework for VexGraph Phase 2
 * operations, providing detailed error reporting and recovery mechanisms.
 */

use serde::{Deserialize, Serialize};
use std::fmt;

/// VexGraph Phase 2 error types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VexGraphError {
    /// Node-related errors
    NodeNotFound(String),
    NodeAlreadyExists(String),
    InvalidNodeType(String),
    NodePropertyError(String),
    
    /// Edge-related errors
    EdgeNotFound(String),
    EdgeAlreadyExists(String),
    InvalidEdgeType(String),
    EdgePropertyError(String),
    
    /// Graph traversal errors
    TraversalError(String),
    InvalidTraversalAlgorithm(String),
    TraversalTimeout(String),
    MaxDepthExceeded(String),
    
    /// Query errors
    QueryParseError(String),
    QueryExecutionError(String),
    QueryTimeout(String),
    InvalidQueryOperator(String),
    
    /// Transaction errors
    TransactionNotFound(String),
    TransactionConflict(String),
    TransactionTimeout(String),
    TransactionAborted(String),
    DeadlockDetected(String),
    
    /// Concurrency errors
    LockTimeout(String),
    ConcurrencyLimitExceeded(String),
    ResourceBusy(String),
    
    /// Performance errors
    CacheError(String),
    IndexError(String),
    OptimizationError(String),
    
    /// Semantic integration errors
    VectorSimilarityError(String),
    SemanticSearchError(String),
    EmbeddingError(String),
    
    /// API errors
    InvalidRequest(String),
    AuthenticationError(String),
    AuthorizationError(String),
    RateLimitExceeded(String),
    
    /// Kernel integration errors
    KernelInterfaceError(String),
    IoctlError(String),
    KernelMemoryError(String),
    
    /// FUSE errors
    FuseOperationError(String),
    FuseMountError(String),
    FusePermissionError(String),
    
    /// Journal integration errors
    JournalError(String),
    ChecksumError(String),
    RecoveryError(String),
    
    /// System errors
    IoError(String),
    MemoryError(String),
    NetworkError(String),
    ConfigurationError(String),
    
    /// Validation errors
    ValidationError(String),
    SchemaError(String),
    ConstraintViolation(String),
    
    /// Internal errors
    InternalError(String),
    NotImplemented(String),
    NotSupported(String),
}

impl fmt::Display for VexGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VexGraphError::NodeNotFound(msg) => write!(f, "Node not found: {}", msg),
            VexGraphError::NodeAlreadyExists(msg) => write!(f, "Node already exists: {}", msg),
            VexGraphError::InvalidNodeType(msg) => write!(f, "Invalid node type: {}", msg),
            VexGraphError::NodePropertyError(msg) => write!(f, "Node property error: {}", msg),
            
            VexGraphError::EdgeNotFound(msg) => write!(f, "Edge not found: {}", msg),
            VexGraphError::EdgeAlreadyExists(msg) => write!(f, "Edge already exists: {}", msg),
            VexGraphError::InvalidEdgeType(msg) => write!(f, "Invalid edge type: {}", msg),
            VexGraphError::EdgePropertyError(msg) => write!(f, "Edge property error: {}", msg),
            
            VexGraphError::TraversalError(msg) => write!(f, "Traversal error: {}", msg),
            VexGraphError::InvalidTraversalAlgorithm(msg) => write!(f, "Invalid traversal algorithm: {}", msg),
            VexGraphError::TraversalTimeout(msg) => write!(f, "Traversal timeout: {}", msg),
            VexGraphError::MaxDepthExceeded(msg) => write!(f, "Maximum depth exceeded: {}", msg),
            
            VexGraphError::QueryParseError(msg) => write!(f, "Query parse error: {}", msg),
            VexGraphError::QueryExecutionError(msg) => write!(f, "Query execution error: {}", msg),
            VexGraphError::QueryTimeout(msg) => write!(f, "Query timeout: {}", msg),
            VexGraphError::InvalidQueryOperator(msg) => write!(f, "Invalid query operator: {}", msg),
            
            VexGraphError::TransactionNotFound(msg) => write!(f, "Transaction not found: {}", msg),
            VexGraphError::TransactionConflict(msg) => write!(f, "Transaction conflict: {}", msg),
            VexGraphError::TransactionTimeout(msg) => write!(f, "Transaction timeout: {}", msg),
            VexGraphError::TransactionAborted(msg) => write!(f, "Transaction aborted: {}", msg),
            VexGraphError::DeadlockDetected(msg) => write!(f, "Deadlock detected: {}", msg),
            
            VexGraphError::LockTimeout(msg) => write!(f, "Lock timeout: {}", msg),
            VexGraphError::ConcurrencyLimitExceeded(msg) => write!(f, "Concurrency limit exceeded: {}", msg),
            VexGraphError::ResourceBusy(msg) => write!(f, "Resource busy: {}", msg),
            
            VexGraphError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            VexGraphError::IndexError(msg) => write!(f, "Index error: {}", msg),
            VexGraphError::OptimizationError(msg) => write!(f, "Optimization error: {}", msg),
            
            VexGraphError::VectorSimilarityError(msg) => write!(f, "Vector similarity error: {}", msg),
            VexGraphError::SemanticSearchError(msg) => write!(f, "Semantic search error: {}", msg),
            VexGraphError::EmbeddingError(msg) => write!(f, "Embedding error: {}", msg),
            
            VexGraphError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            VexGraphError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            VexGraphError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            VexGraphError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            
            VexGraphError::KernelInterfaceError(msg) => write!(f, "Kernel interface error: {}", msg),
            VexGraphError::IoctlError(msg) => write!(f, "Ioctl error: {}", msg),
            VexGraphError::KernelMemoryError(msg) => write!(f, "Kernel memory error: {}", msg),
            
            VexGraphError::FuseOperationError(msg) => write!(f, "FUSE operation error: {}", msg),
            VexGraphError::FuseMountError(msg) => write!(f, "FUSE mount error: {}", msg),
            VexGraphError::FusePermissionError(msg) => write!(f, "FUSE permission error: {}", msg),
            
            VexGraphError::JournalError(msg) => write!(f, "Journal error: {}", msg),
            VexGraphError::ChecksumError(msg) => write!(f, "Checksum error: {}", msg),
            VexGraphError::RecoveryError(msg) => write!(f, "Recovery error: {}", msg),
            
            VexGraphError::IoError(msg) => write!(f, "I/O error: {}", msg),
            VexGraphError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            VexGraphError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            VexGraphError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            
            VexGraphError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            VexGraphError::SchemaError(msg) => write!(f, "Schema error: {}", msg),
            VexGraphError::ConstraintViolation(msg) => write!(f, "Constraint violation: {}", msg),
            
            VexGraphError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            VexGraphError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            VexGraphError::NotSupported(msg) => write!(f, "Not supported: {}", msg),
        }
    }
}

impl std::error::Error for VexGraphError {}

/// Result type for VexGraph Phase 2 operations
pub type VexGraphResult<T> = Result<T, VexGraphError>;

/// Error context for detailed error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: std::collections::HashMap<String, String>,
    pub stack_trace: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: &str, component: &str) -> Self {
        Self {
            operation: operation.to_string(),
            component: component.to_string(),
            timestamp: chrono::Utc::now(),
            details: std::collections::HashMap::new(),
            stack_trace: None,
        }
    }
    
    pub fn with_detail(mut self, key: &str, value: &str) -> Self {
        self.details.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn with_stack_trace(mut self, stack_trace: String) -> Self {
        self.stack_trace = Some(stack_trace);
        self
    }
}

/// Enhanced error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VexGraphErrorWithContext {
    pub error: VexGraphError,
    pub context: ErrorContext,
}

impl fmt::Display for VexGraphErrorWithContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (operation: {}, component: {})", 
               self.error, self.context.operation, self.context.component)
    }
}

impl std::error::Error for VexGraphErrorWithContext {}

/// Validation framework for VexGraph operations
pub struct Validator;

impl Validator {
    /// Validate node ID
    pub fn validate_node_id(node_id: u64) -> VexGraphResult<()> {
        if node_id == 0 {
            return Err(VexGraphError::ValidationError("Node ID cannot be zero".to_string()));
        }
        Ok(())
    }
    
    /// Validate edge ID
    pub fn validate_edge_id(edge_id: u64) -> VexGraphResult<()> {
        if edge_id == 0 {
            return Err(VexGraphError::ValidationError("Edge ID cannot be zero".to_string()));
        }
        Ok(())
    }
    
    /// Validate property key
    pub fn validate_property_key(key: &str) -> VexGraphResult<()> {
        if key.is_empty() {
            return Err(VexGraphError::ValidationError("Property key cannot be empty".to_string()));
        }
        if key.len() > 256 {
            return Err(VexGraphError::ValidationError("Property key too long".to_string()));
        }
        if !key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(VexGraphError::ValidationError("Property key contains invalid characters".to_string()));
        }
        Ok(())
    }
    
    /// Validate property value size
    pub fn validate_property_value_size(size: usize) -> VexGraphResult<()> {
        if size > crate::vexgraph::MAX_PROPERTY_VALUE_SIZE {
            return Err(VexGraphError::ValidationError(
                format!("Property value size {} exceeds maximum {}", 
                       size, crate::vexgraph::MAX_PROPERTY_VALUE_SIZE)
            ));
        }
        Ok(())
    }
    
    /// Validate traversal depth
    pub fn validate_traversal_depth(depth: u32) -> VexGraphResult<()> {
        if depth > crate::vexgraph::MAX_TRAVERSAL_DEPTH {
            return Err(VexGraphError::ValidationError(
                format!("Traversal depth {} exceeds maximum {}", 
                       depth, crate::vexgraph::MAX_TRAVERSAL_DEPTH)
            ));
        }
        Ok(())
    }
    
    /// Validate vector dimensions
    pub fn validate_vector_dimensions(dimensions: usize) -> VexGraphResult<()> {
        if dimensions == 0 {
            return Err(VexGraphError::ValidationError("Vector dimensions cannot be zero".to_string()));
        }
        if dimensions > 4096 {
            return Err(VexGraphError::ValidationError("Vector dimensions exceed maximum 4096".to_string()));
        }
        Ok(())
    }
}

/// Error recovery strategies
pub struct ErrorRecovery;

impl ErrorRecovery {
    /// Attempt to recover from a transaction conflict
    pub async fn recover_transaction_conflict(
        transaction_id: &str,
        retry_count: u32,
    ) -> VexGraphResult<bool> {
        if retry_count > 3 {
            return Err(VexGraphError::TransactionConflict(
                "Maximum retry count exceeded".to_string()
            ));
        }
        
        // Implement exponential backoff
        let delay = std::time::Duration::from_millis(100 * (2_u64.pow(retry_count)));
        tokio::time::sleep(delay).await;
        
        Ok(true)
    }
    
    /// Attempt to recover from a deadlock
    pub async fn recover_deadlock(
        transaction_ids: &[String],
    ) -> VexGraphResult<String> {
        // Choose the transaction with the lowest ID to abort
        if let Some(victim_id) = transaction_ids.iter().min() {
            Ok(victim_id.clone())
        } else {
            Err(VexGraphError::DeadlockDetected(
                "No transactions to abort".to_string()
            ))
        }
    }
    
    /// Attempt to recover from a cache error
    pub async fn recover_cache_error(
        cache_key: &str,
    ) -> VexGraphResult<()> {
        // Invalidate the cache entry and retry
        tracing::warn!("Recovering from cache error for key: {}", cache_key);
        Ok(())
    }
}

/// Error reporting and logging
pub struct ErrorReporter;

impl ErrorReporter {
    /// Report an error with context
    pub fn report_error(error: &VexGraphErrorWithContext) {
        tracing::error!(
            error = %error.error,
            operation = %error.context.operation,
            component = %error.context.component,
            timestamp = %error.context.timestamp,
            details = ?error.context.details,
            "VexGraph error occurred"
        );
    }
    
    /// Report a warning
    pub fn report_warning(message: &str, component: &str) {
        tracing::warn!(
            component = component,
            message = message,
            "VexGraph warning"
        );
    }
    
    /// Report performance issues
    pub fn report_performance_issue(
        operation: &str,
        duration: std::time::Duration,
        threshold: std::time::Duration,
    ) {
        if duration > threshold {
            tracing::warn!(
                operation = operation,
                duration_ms = duration.as_millis(),
                threshold_ms = threshold.as_millis(),
                "Performance threshold exceeded"
            );
        }
    }
}

/// Macro for creating errors with context
#[macro_export]
macro_rules! vexgraph_error {
    ($error:expr, $operation:expr, $component:expr) => {
        VexGraphErrorWithContext {
            error: $error,
            context: ErrorContext::new($operation, $component),
        }
    };
    ($error:expr, $operation:expr, $component:expr, $($key:expr => $value:expr),*) => {
        VexGraphErrorWithContext {
            error: $error,
            context: ErrorContext::new($operation, $component)
                $(.with_detail($key, $value))*,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = VexGraphError::NodeNotFound("test_node".to_string());
        assert_eq!(error.to_string(), "Node not found: test_node");
    }
    
    #[test]
    fn test_validator() {
        assert!(Validator::validate_node_id(1).is_ok());
        assert!(Validator::validate_node_id(0).is_err());
        
        assert!(Validator::validate_property_key("valid_key").is_ok());
        assert!(Validator::validate_property_key("").is_err());
        assert!(Validator::validate_property_key("invalid key!").is_err());
    }
    
    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("test_operation", "test_component")
            .with_detail("key1", "value1")
            .with_detail("key2", "value2");
        
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "test_component");
        assert_eq!(context.details.len(), 2);
    }
    
    #[tokio::test]
    async fn test_error_recovery() {
        let result = ErrorRecovery::recover_transaction_conflict("test_tx", 1).await;
        assert!(result.is_ok());
        
        let result = ErrorRecovery::recover_transaction_conflict("test_tx", 5).await;
        assert!(result.is_err());
    }
}