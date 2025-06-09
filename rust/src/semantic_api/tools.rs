//! Tool Calling Integration for VexFS Operations
//!
//! This module implements a standard interface for tool invocation by AI agents,
//! providing idempotent operations, result caching, and safe VexFS interactions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn, error, instrument};
use uuid::Uuid;

use crate::semantic_api::{SemanticResult, SemanticError, types::*};

/// Tool calling manager for AI agents
pub struct ToolManager {
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
    execution_cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    execution_history: Arc<RwLock<Vec<ToolExecution>>>,
    config: ToolConfig,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub max_concurrent_executions: usize,
    pub default_timeout_seconds: u64,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub max_cache_size: usize,
    pub enable_retry: bool,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub enable_execution_history: bool,
    pub max_history_size: usize,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 100,
            default_timeout_seconds: 300, // 5 minutes
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_size: 10000,
            enable_retry: true,
            max_retries: 3,
            retry_delay_seconds: 1,
            enable_execution_history: true,
            max_history_size: 100000,
        }
    }
}

/// Tool trait for implementing VexFS operations
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata;
    
    /// Execute the tool with given parameters
    async fn execute(&self, params: ToolParameters) -> ToolResult;
    
    /// Validate parameters before execution
    fn validate_parameters(&self, params: &ToolParameters) -> SemanticResult<()>;
    
    /// Check if the tool operation is idempotent
    fn is_idempotent(&self) -> bool;
    
    /// Get estimated execution time
    fn estimated_duration(&self) -> Duration;
}

/// Tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: ToolCategory,
    pub parameters_schema: serde_json::Value,
    pub return_schema: serde_json::Value,
    pub required_permissions: Vec<String>,
    pub is_idempotent: bool,
    pub estimated_duration_ms: u64,
    pub resource_requirements: ResourceRequirements,
}

/// Tool categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    Filesystem,
    Graph,
    Vector,
    Query,
    Analysis,
    System,
    Custom(String),
}

/// Resource requirements for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_intensive: bool,
    pub memory_intensive: bool,
    pub io_intensive: bool,
    pub network_required: bool,
    pub estimated_memory_mb: u64,
}

/// Tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub execution_context: ExecutionContext,
    pub timeout_seconds: Option<u64>,
    pub enable_caching: Option<bool>,
    pub idempotency_key: Option<String>,
}

/// Execution context for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub agent_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub transaction_id: Option<u64>,
    pub causality_chain_id: Option<u64>,
    pub environment: HashMap<String, serde_json::Value>,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub execution_id: String,
    pub tool_name: String,
    pub status: ExecutionStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<ToolError>,
    pub execution_time_ms: u64,
    pub cached: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tool execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
    Cancelled,
    Retrying,
}

/// Tool execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub error_type: ErrorType,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

/// Types of tool errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    InvalidParameters,
    PermissionDenied,
    ResourceNotFound,
    ResourceConflict,
    Timeout,
    InternalError,
    NetworkError,
    ValidationError,
}

/// Cached tool result
#[derive(Debug, Clone)]
struct CachedResult {
    result: ToolResult,
    cached_at: DateTime<Utc>,
    ttl_seconds: u64,
    access_count: u64,
}

/// Tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub execution_id: String,
    pub tool_name: String,
    pub agent_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub parameters_hash: String,
    pub result_size_bytes: u64,
    pub cached: bool,
    pub retry_count: u32,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new(config: ToolConfig) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            execution_cache: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Register a tool
    #[instrument(skip(self, tool))]
    pub async fn register_tool(&self, tool: Box<dyn Tool>) -> SemanticResult<()> {
        let metadata = tool.metadata();
        let tool_name = metadata.name.clone();
        
        let mut tools = self.tools.write().await;
        tools.insert(tool_name.clone(), tool);
        
        info!("Registered tool: {}", tool_name);
        Ok(())
    }

    /// Unregister a tool
    #[instrument(skip(self))]
    pub async fn unregister_tool(&self, tool_name: &str) -> SemanticResult<()> {
        let mut tools = self.tools.write().await;
        if tools.remove(tool_name).is_some() {
            info!("Unregistered tool: {}", tool_name);
            Ok(())
        } else {
            Err(SemanticError::InvalidRequest(
                format!("Tool {} not found", tool_name)
            ))
        }
    }

    /// Execute a tool
    #[instrument(skip(self))]
    pub async fn execute_tool(&self, params: ToolParameters) -> SemanticResult<ToolResult> {
        let execution_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();
        
        // Check cache first if enabled
        if params.enable_caching.unwrap_or(self.config.enable_caching) {
            if let Some(cached) = self.check_cache(&params).await? {
                debug!("Tool execution cache hit: {}", params.tool_name);
                return Ok(cached);
            }
        }

        // Get tool
        let tool = {
            let tools = self.tools.read().await;
            tools.get(&params.tool_name)
                .ok_or_else(|| SemanticError::InvalidRequest(
                    format!("Tool {} not found", params.tool_name)
                ))?
                .metadata()
        };

        // Validate parameters
        let tools_read = self.tools.read().await;
        let tool_impl = tools_read.get(&params.tool_name).unwrap();
        tool_impl.validate_parameters(&params)?;
        drop(tools_read);

        // Record execution start
        let execution_record = ToolExecution {
            execution_id: execution_id.clone(),
            tool_name: params.tool_name.clone(),
            agent_id: params.execution_context.agent_id.clone(),
            started_at: Utc::now(),
            completed_at: None,
            status: ExecutionStatus::Success,
            parameters_hash: self.compute_parameters_hash(&params),
            result_size_bytes: 0,
            cached: false,
            retry_count: 0,
        };

        if self.config.enable_execution_history {
            let mut history = self.execution_history.write().await;
            history.push(execution_record);
            
            // Trim history if too large
            if history.len() > self.config.max_history_size {
                history.remove(0);
            }
        }

        // Execute with retry logic
        let mut retry_count = 0;
        let max_retries = if self.config.enable_retry { self.config.max_retries } else { 0 };

        loop {
            let result = self.execute_tool_internal(&params, &execution_id).await;
            
            match result {
                Ok(mut tool_result) => {
                    tool_result.execution_time_ms = start_time.elapsed().as_millis() as u64;
                    
                    // Cache result if successful and caching is enabled
                    if params.enable_caching.unwrap_or(self.config.enable_caching) &&
                       matches!(tool_result.status, ExecutionStatus::Success) {
                        self.cache_result(&params, &tool_result).await?;
                    }
                    
                    return Ok(tool_result);
                }
                Err(e) if retry_count < max_retries => {
                    retry_count += 1;
                    warn!("Tool execution failed, retrying ({}/{}): {}", retry_count, max_retries, e);
                    
                    tokio::time::sleep(Duration::from_secs(self.config.retry_delay_seconds)).await;
                    continue;
                }
                Err(e) => {
                    error!("Tool execution failed after {} retries: {}", retry_count, e);
                    return Err(e);
                }
            }
        }
    }

    /// Internal tool execution
    async fn execute_tool_internal(&self, params: &ToolParameters, execution_id: &str) -> SemanticResult<ToolResult> {
        let tools = self.tools.read().await;
        let tool = tools.get(&params.tool_name)
            .ok_or_else(|| SemanticError::InvalidRequest(
                format!("Tool {} not found", params.tool_name)
            ))?;

        // Set timeout
        let timeout = Duration::from_secs(
            params.timeout_seconds.unwrap_or(self.config.default_timeout_seconds)
        );

        // Execute with timeout
        let result = tokio::time::timeout(timeout, tool.execute(params.clone())).await;
        
        match result {
            Ok(tool_result) => Ok(tool_result),
            Err(_) => {
                let error_result = ToolResult {
                    execution_id: execution_id.to_string(),
                    tool_name: params.tool_name.clone(),
                    status: ExecutionStatus::Timeout,
                    result: None,
                    error: Some(ToolError {
                        error_type: ErrorType::Timeout,
                        message: "Tool execution timed out".to_string(),
                        details: None,
                        retryable: true,
                    }),
                    execution_time_ms: timeout.as_millis() as u64,
                    cached: false,
                    metadata: HashMap::new(),
                };
                Ok(error_result)
            }
        }
    }

    /// Check execution cache
    async fn check_cache(&self, params: &ToolParameters) -> SemanticResult<Option<ToolResult>> {
        let cache_key = self.compute_cache_key(params);
        let cache = self.execution_cache.read().await;
        
        if let Some(cached) = cache.get(&cache_key) {
            let now = Utc::now();
            let age = now.signed_duration_since(cached.cached_at).num_seconds() as u64;
            
            if age < cached.ttl_seconds {
                let mut result = cached.result.clone();
                result.cached = true;
                return Ok(Some(result));
            }
        }
        
        Ok(None)
    }

    /// Cache execution result
    async fn cache_result(&self, params: &ToolParameters, result: &ToolResult) -> SemanticResult<()> {
        let cache_key = self.compute_cache_key(params);
        let cached_result = CachedResult {
            result: result.clone(),
            cached_at: Utc::now(),
            ttl_seconds: self.config.cache_ttl_seconds,
            access_count: 0,
        };
        
        let mut cache = self.execution_cache.write().await;
        
        // Remove old entries if cache is full
        if cache.len() >= self.config.max_cache_size {
            // Remove least recently used entries
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, cached)| cached.cached_at);
            
            let to_remove = cache.len() - self.config.max_cache_size + 1;
            for (key, _) in entries.iter().take(to_remove) {
                cache.remove(*key);
            }
        }
        
        cache.insert(cache_key, cached_result);
        Ok(())
    }

    /// Compute cache key for parameters
    fn compute_cache_key(&self, params: &ToolParameters) -> String {
        // Use idempotency key if provided, otherwise hash parameters
        if let Some(key) = &params.idempotency_key {
            format!("{}:{}", params.tool_name, key)
        } else {
            format!("{}:{}", params.tool_name, self.compute_parameters_hash(params))
        }
    }

    /// Compute hash of parameters
    fn compute_parameters_hash(&self, params: &ToolParameters) -> String {
        // Simple hash implementation - in production, use a proper hash function
        format!("{:x}", params.parameters.to_string().len())
    }

    /// List available tools
    pub async fn list_tools(&self) -> Vec<ToolMetadata> {
        let tools = self.tools.read().await;
        tools.values().map(|tool| tool.metadata()).collect()
    }

    /// Get tool metadata
    pub async fn get_tool_metadata(&self, tool_name: &str) -> SemanticResult<ToolMetadata> {
        let tools = self.tools.read().await;
        tools.get(tool_name)
            .map(|tool| tool.metadata())
            .ok_or_else(|| SemanticError::InvalidRequest(
                format!("Tool {} not found", tool_name)
            ))
    }

    /// Get execution history
    pub async fn get_execution_history(&self, agent_id: Option<&str>, limit: Option<usize>) -> Vec<ToolExecution> {
        let history = self.execution_history.read().await;
        let mut filtered: Vec<_> = history.iter()
            .filter(|exec| agent_id.map_or(true, |id| exec.agent_id == id))
            .cloned()
            .collect();
        
        // Sort by start time (most recent first)
        filtered.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        
        if let Some(limit) = limit {
            filtered.truncate(limit);
        }
        
        filtered
    }

    /// Clear execution cache
    pub async fn clear_cache(&self) -> SemanticResult<usize> {
        let mut cache = self.execution_cache.write().await;
        let count = cache.len();
        cache.clear();
        
        info!("Cleared {} cached tool results", count);
        Ok(count)
    }
}

/// Filesystem tool for VexFS operations
pub struct FilesystemTool {
    metadata: ToolMetadata,
}

impl FilesystemTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            name: "filesystem".to_string(),
            description: "Perform filesystem operations on VexFS".to_string(),
            version: "1.0.0".to_string(),
            category: ToolCategory::Filesystem,
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "enum": ["read", "write", "create", "delete", "list"]},
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                    "recursive": {"type": "boolean"}
                },
                "required": ["operation", "path"]
            }),
            return_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "result": {"type": "string"},
                    "metadata": {"type": "object"}
                }
            }),
            required_permissions: vec!["filesystem:read".to_string(), "filesystem:write".to_string()],
            is_idempotent: false,
            estimated_duration_ms: 100,
            resource_requirements: ResourceRequirements {
                cpu_intensive: false,
                memory_intensive: false,
                io_intensive: true,
                network_required: false,
                estimated_memory_mb: 10,
            },
        };

        Self { metadata }
    }
}

#[async_trait::async_trait]
impl Tool for FilesystemTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, params: ToolParameters) -> ToolResult {
        let execution_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();

        // Extract parameters
        let operation = params.parameters.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("read");
        let path = params.parameters.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Simulate filesystem operation
        let result = match operation {
            "read" => {
                serde_json::json!({
                    "success": true,
                    "content": format!("Content of {}", path),
                    "size": 1024
                })
            }
            "write" => {
                serde_json::json!({
                    "success": true,
                    "bytes_written": 1024
                })
            }
            "create" => {
                serde_json::json!({
                    "success": true,
                    "created": path
                })
            }
            "delete" => {
                serde_json::json!({
                    "success": true,
                    "deleted": path
                })
            }
            "list" => {
                serde_json::json!({
                    "success": true,
                    "entries": ["file1.txt", "file2.txt", "subdir/"]
                })
            }
            _ => {
                return ToolResult {
                    execution_id,
                    tool_name: params.tool_name,
                    status: ExecutionStatus::Failed,
                    result: None,
                    error: Some(ToolError {
                        error_type: ErrorType::InvalidParameters,
                        message: format!("Unknown operation: {}", operation),
                        details: None,
                        retryable: false,
                    }),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    cached: false,
                    metadata: HashMap::new(),
                };
            }
        };

        ToolResult {
            execution_id,
            tool_name: params.tool_name,
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            cached: false,
            metadata: HashMap::new(),
        }
    }

    fn validate_parameters(&self, params: &ToolParameters) -> SemanticResult<()> {
        if !params.parameters.is_object() {
            return Err(SemanticError::ValidationError(
                "Parameters must be an object".to_string()
            ));
        }

        if params.parameters.get("operation").is_none() {
            return Err(SemanticError::ValidationError(
                "Missing required parameter: operation".to_string()
            ));
        }

        if params.parameters.get("path").is_none() {
            return Err(SemanticError::ValidationError(
                "Missing required parameter: path".to_string()
            ));
        }

        Ok(())
    }

    fn is_idempotent(&self) -> bool {
        false // Filesystem operations are generally not idempotent
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(100)
    }
}

/// Graph tool for VexGraph operations
pub struct GraphTool {
    metadata: ToolMetadata,
}

impl GraphTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            name: "graph".to_string(),
            description: "Perform graph operations on VexGraph".to_string(),
            version: "1.0.0".to_string(),
            category: ToolCategory::Graph,
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "enum": ["create_node", "create_edge", "query", "traverse"]},
                    "node_id": {"type": "string"},
                    "edge_id": {"type": "string"},
                    "properties": {"type": "object"},
                    "query": {"type": "string"}
                },
                "required": ["operation"]
            }),
            return_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "result": {"type": "object"}
                }
            }),
            required_permissions: vec!["graph:read".to_string(), "graph:write".to_string()],
            is_idempotent: true,
            estimated_duration_ms: 50,
            resource_requirements: ResourceRequirements {
                cpu_intensive: true,
                memory_intensive: true,
                io_intensive: false,
                network_required: false,
                estimated_memory_mb: 50,
            },
        };

        Self { metadata }
    }
}

#[async_trait::async_trait]
impl Tool for GraphTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, params: ToolParameters) -> ToolResult {
        let execution_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();

        // Simulate graph operation
        let result = serde_json::json!({
            "success": true,
            "nodes": 10,
            "edges": 15
        });

        ToolResult {
            execution_id,
            tool_name: params.tool_name,
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            cached: false,
            metadata: HashMap::new(),
        }
    }

    fn validate_parameters(&self, params: &ToolParameters) -> SemanticResult<()> {
        if params.parameters.get("operation").is_none() {
            return Err(SemanticError::ValidationError(
                "Missing required parameter: operation".to_string()
            ));
        }
        Ok(())
    }

    fn is_idempotent(&self) -> bool {
        true
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registration() {
        let tool_manager = ToolManager::new(ToolConfig::default());
        let filesystem_tool = Box::new(FilesystemTool::new());
        
        assert!(tool_manager.register_tool(filesystem_tool).await.is_ok());
        
        let tools = tool_manager.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "filesystem");
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool_manager = ToolManager::new(ToolConfig::default());
        let filesystem_tool = Box::new(FilesystemTool::new());
        
        tool_manager.register_tool(filesystem_tool).await.unwrap();
        
        let params = ToolParameters {
            tool_name: "filesystem".to_string(),
            parameters: serde_json::json!({
                "operation": "read",
                "path": "/test/file.txt"
            }),
            execution_context: ExecutionContext {
                agent_id: "test_agent".to_string(),
                session_id: None,
                task_id: None,
                transaction_id: None,
                causality_chain_id: None,
                environment: HashMap::new(),
            },
            timeout_seconds: None,
            enable_caching: None,
            idempotency_key: None,
        };
        
        let result = tool_manager.execute_tool(params).await.unwrap();
        assert!(matches!(result.status, ExecutionStatus::Success));
        assert!(result.result.is_some());
    }

    #[tokio::test]
    async fn test_tool_caching() {
        let mut config = ToolConfig::default();
        config.enable_caching = true;
        
        let tool_manager = ToolManager::new(config);
        let filesystem_tool = Box::new(FilesystemTool::new());
        
        tool_manager.register_tool(filesystem_tool).await.unwrap();
        
        let params = ToolParameters {
            tool_name: "filesystem".to_string(),
            parameters: serde_json::json!({
                "operation": "read",
                "path": "/test/file.txt"
            }),
            execution_context: ExecutionContext {
                agent_id: "test_agent".to_string(),
                session_id: None,
                task_id: None,
                transaction_id: None,
                causality_chain_id: None,
                environment: HashMap::new(),
            },
            timeout_seconds: None,
            enable_caching: Some(true),
            idempotency_key: Some("test_key".to_string()),
        };
        
        // First execution
        let result1 = tool_manager.execute_tool(params.clone()).await.unwrap();
        assert!(!result1.cached);
        
        // Second execution should be cached
        let result2 = tool_manager.execute_tool(params).await.unwrap();
        assert!(result2.cached);
    }
}