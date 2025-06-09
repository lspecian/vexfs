//! Stub implementation for API server to resolve compilation issues
//! This is a temporary stub until all dependencies are properly configured

use crate::semantic_api::{SemanticResult, SemanticError};

/// Stub API server configuration
#[derive(Debug, Clone)]
pub struct ApiServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// Stub API server
pub struct ApiServer {
    config: ApiServerConfig,
}

impl ApiServer {
    pub fn new(config: ApiServerConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> SemanticResult<()> {
        // TODO: Implement actual server
        Err(SemanticError::internal("API server not yet implemented"))
    }

    pub async fn stop(&self) -> SemanticResult<()> {
        // TODO: Implement actual server shutdown
        Ok(())
    }
}

/// Stub API error type
#[derive(Debug)]
pub struct ApiError {
    message: String,
}

impl ApiError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "API Error: {}", self.message)
    }
}

impl std::error::Error for ApiError {}

/// Stub function for parsing event types
pub fn parse_event_types(_types_str: &str) -> Result<Vec<crate::semantic_api::SemanticEventType>, ApiError> {
    // TODO: Implement actual parsing
    Ok(vec![])
}

/// Stub function for parsing event categories
pub fn parse_event_categories(_categories_str: &str) -> Result<Vec<crate::semantic_api::EventCategory>, ApiError> {
    // TODO: Implement actual parsing
    Ok(vec![])
}