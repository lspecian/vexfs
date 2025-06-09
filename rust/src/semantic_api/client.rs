//! Client SDK for Semantic API
//! 
//! This module provides a client SDK for AI agents to interact with the
//! VexFS Semantic Event API.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use crate::semantic_api::serialization::{SemanticSerializer, SerializationConfig};
use std::collections::HashMap;
use reqwest;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use url::Url;
use serde_json;

/// Semantic API Client
#[derive(Debug, Clone)]
pub struct SemanticApiClient {
    base_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
    serializer: SemanticSerializer,
}

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub timeout_seconds: u64,
    pub user_agent: String,
    pub serialization: SerializationConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            timeout_seconds: 30,
            user_agent: "VexFS-Semantic-Client/1.0.0".to_string(),
            serialization: SerializationConfig::default(),
        }
    }
}

impl SemanticApiClient {
    /// Create a new client
    pub fn new(config: ClientConfig) -> SemanticResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| SemanticError::InternalError(format!("Failed to create HTTP client: {}", e)))?;
        
        let serializer = SemanticSerializer::new(config.serialization);
        
        Ok(Self {
            base_url: config.base_url,
            auth_token: None,
            client,
            serializer,
        })
    }
    
    /// Authenticate with the API
    pub async fn authenticate(&mut self, agent_id: &str, agent_secret: &str, scopes: Vec<String>) -> SemanticResult<()> {
        let auth_request = serde_json::json!({
            "agent_id": agent_id,
            "agent_secret": agent_secret,
            "scopes": scopes
        });
        
        let response = self.client
            .post(&format!("{}/api/v1/auth/login", self.base_url))
            .json(&auth_request)
            .send()
            .await
            .map_err(|e| SemanticError::InternalError(format!("Authentication request failed: {}", e)))?;
        
        if response.status().is_success() {
            let auth_response: ApiResponse<serde_json::Value> = response.json().await
                .map_err(|e| SemanticError::InternalError(format!("Failed to parse auth response: {}", e)))?;
            
            if let Some(data) = auth_response.data {
                if let Some(token) = data.get("token").and_then(|t| t.as_str()) {
                    self.auth_token = Some(token.to_string());
                    return Ok(());
                }
            }
        }
        
        Err(SemanticError::AuthenticationFailed("Authentication failed".to_string()))
    }
    
    /// Query events
    pub async fn query_events(&self, query: &EventQuery) -> SemanticResult<EventQueryResponse> {
        self.ensure_authenticated()?;
        
        let response = self.client
            .post(&format!("{}/api/v1/events/query", self.base_url))
            .header("Authorization", format!("Bearer {}", self.auth_token.as_ref().unwrap()))
            .json(query)
            .send()
            .await
            .map_err(|e| SemanticError::InternalError(format!("Query request failed: {}", e)))?;
        
        if response.status().is_success() {
            let query_response: EventQueryResponse = response.json().await
                .map_err(|e| SemanticError::InternalError(format!("Failed to parse query response: {}", e)))?;
            Ok(query_response)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(SemanticError::QueryError(format!("Query failed: {}", error_text)))
        }
    }
    
    /// List events with simple parameters
    pub async fn list_events(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        event_types: Option<Vec<SemanticEventType>>,
    ) -> SemanticResult<EventQueryResponse> {
        self.ensure_authenticated()?;
        
        let mut params = Vec::new();
        
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            params.push(("offset", offset.to_string()));
        }
        if let Some(types) = event_types {
            let types_str = types.iter()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<_>>()
                .join(",");
            params.push(("event_types", types_str));
        }
        
        let response = self.client
            .get(&format!("{}/api/v1/events", self.base_url))
            .header("Authorization", format!("Bearer {}", self.auth_token.as_ref().unwrap()))
            .query(&params)
            .send()
            .await
            .map_err(|e| SemanticError::InternalError(format!("List request failed: {}", e)))?;
        
        if response.status().is_success() {
            let query_response: EventQueryResponse = response.json().await
                .map_err(|e| SemanticError::InternalError(format!("Failed to parse list response: {}", e)))?;
            Ok(query_response)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(SemanticError::QueryError(format!("List failed: {}", error_text)))
        }
    }
    
    /// Get a specific event by ID
    pub async fn get_event(&self, event_id: u64) -> SemanticResult<SemanticEvent> {
        self.ensure_authenticated()?;
        
        let response = self.client
            .get(&format!("{}/api/v1/events/{}", self.base_url, event_id))
            .header("Authorization", format!("Bearer {}", self.auth_token.as_ref().unwrap()))
            .send()
            .await
            .map_err(|e| SemanticError::InternalError(format!("Get event request failed: {}", e)))?;
        
        if response.status().is_success() {
            let event: SemanticEvent = response.json().await
                .map_err(|e| SemanticError::InternalError(format!("Failed to parse event response: {}", e)))?;
            Ok(event)
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(SemanticError::InvalidRequest(format!("Event {} not found", event_id)))
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(SemanticError::QueryError(format!("Get event failed: {}", error_text)))
        }
    }
    
    /// Subscribe to real-time events via WebSocket
    pub async fn subscribe_to_events(
        &self,
        filter: EventFilter,
    ) -> SemanticResult<EventStream> {
        self.ensure_authenticated()?;
        
        // Build WebSocket URL
        let ws_url = self.base_url.replace("http://", "ws://").replace("https://", "wss://");
        let url = format!("{}/api/v1/stream", ws_url);
        
        // Add authentication header (this is simplified - real implementation would need proper WebSocket auth)
        let url = Url::parse(&url)
            .map_err(|e| SemanticError::InternalError(format!("Invalid WebSocket URL: {}", e)))?;
        
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| SemanticError::StreamError(format!("WebSocket connection failed: {}", e)))?;
        
        Ok(EventStream::new(ws_stream, self.serializer.clone()))
    }
    
    /// Get API statistics
    pub async fn get_stats(&self) -> SemanticResult<serde_json::Value> {
        self.ensure_authenticated()?;
        
        let response = self.client
            .get(&format!("{}/api/v1/stats", self.base_url))
            .header("Authorization", format!("Bearer {}", self.auth_token.as_ref().unwrap()))
            .send()
            .await
            .map_err(|e| SemanticError::InternalError(format!("Stats request failed: {}", e)))?;
        
        if response.status().is_success() {
            let stats_response: ApiResponse<serde_json::Value> = response.json().await
                .map_err(|e| SemanticError::InternalError(format!("Failed to parse stats response: {}", e)))?;
            Ok(stats_response.data.unwrap_or_else(|| serde_json::Value::Null))
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(SemanticError::QueryError(format!("Stats request failed: {}", error_text)))
        }
    }
    
    /// Check if client is authenticated
    fn ensure_authenticated(&self) -> SemanticResult<()> {
        if self.auth_token.is_none() {
            Err(SemanticError::AuthenticationFailed("Not authenticated".to_string()))
        } else {
            Ok(())
        }
    }
}

/// Event stream for real-time events
pub struct EventStream {
    ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    serializer: SemanticSerializer,
}

impl EventStream {
    fn new(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        serializer: SemanticSerializer,
    ) -> Self {
        Self { ws_stream, serializer }
    }
    
    /// Receive the next event from the stream
    pub async fn next_event(&mut self) -> SemanticResult<Option<StreamMessage>> {
        while let Some(message) = self.ws_stream.next().await {
            match message {
                Ok(Message::Binary(data)) => {
                    match self.serializer.deserialize_stream_message(&data) {
                        Ok(stream_message) => return Ok(Some(stream_message)),
                        Err(e) => {
                            tracing::warn!("Failed to deserialize stream message: {}", e);
                            continue;
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    return Ok(None);
                }
                Ok(_) => {
                    // Ignore other message types
                    continue;
                }
                Err(e) => {
                    return Err(SemanticError::StreamError(format!("WebSocket error: {}", e)));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Close the event stream
    pub async fn close(&mut self) -> SemanticResult<()> {
        self.ws_stream.close(None).await
            .map_err(|e| SemanticError::StreamError(format!("Failed to close WebSocket: {}", e)))?;
        Ok(())
    }
}

/// High-level client builder
pub struct SemanticApiClientBuilder {
    config: ClientConfig,
}

impl SemanticApiClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }
    
    /// Set the base URL
    pub fn base_url(mut self, url: &str) -> Self {
        self.config.base_url = url.to_string();
        self
    }
    
    /// Set the timeout
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }
    
    /// Set the user agent
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.config.user_agent = user_agent.to_string();
        self
    }
    
    /// Set serialization config
    pub fn serialization(mut self, config: SerializationConfig) -> Self {
        self.config.serialization = config;
        self
    }
    
    /// Build the client
    pub fn build(self) -> SemanticResult<SemanticApiClient> {
        SemanticApiClient::new(self.config)
    }
}

impl Default for SemanticApiClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for common operations
pub mod convenience {
    use super::*;
    
    /// Create a simple client with default configuration
    pub async fn create_client(base_url: &str) -> SemanticResult<SemanticApiClient> {
        SemanticApiClientBuilder::new()
            .base_url(base_url)
            .build()
    }
    
    /// Create and authenticate a client in one step
    pub async fn create_authenticated_client(
        base_url: &str,
        agent_id: &str,
        agent_secret: &str,
        scopes: Vec<String>,
    ) -> SemanticResult<SemanticApiClient> {
        let mut client = create_client(base_url).await?;
        client.authenticate(agent_id, agent_secret, scopes).await?;
        Ok(client)
    }
    
    /// Query recent events (last hour)
    pub async fn query_recent_events(
        client: &SemanticApiClient,
        event_types: Option<Vec<SemanticEventType>>,
        limit: Option<usize>,
    ) -> SemanticResult<EventQueryResponse> {
        let now = chrono::Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        
        let query = EventQuery {
            filter: EventFilter {
                event_types,
                categories: None,
                time_range: Some(TimeRange {
                    start: one_hour_ago,
                    end: now,
                }),
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit,
            offset: None,
            sort_by: Some(SortBy::Timestamp),
            include_payload: true,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        client.query_events(&query).await
    }
    
    /// Query events by agent
    pub async fn query_agent_events(
        client: &SemanticApiClient,
        agent_id: &str,
        limit: Option<usize>,
    ) -> SemanticResult<EventQueryResponse> {
        let query = EventQuery {
            filter: EventFilter {
                event_types: None,
                categories: None,
                time_range: None,
                agent_id: Some(agent_id.to_string()),
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit,
            offset: None,
            sort_by: Some(SortBy::Timestamp),
            include_payload: true,
            include_metadata: true,
            include_causality: true,
            aggregation: None,
        };
        
        client.query_events(&query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = SemanticApiClientBuilder::new()
            .base_url("http://localhost:9000")
            .timeout(60)
            .user_agent("Test-Agent/1.0")
            .build()
            .unwrap();
        
        assert_eq!(client.base_url, "http://localhost:9000");
        assert!(client.auth_token.is_none());
    }
    
    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.user_agent, "VexFS-Semantic-Client/1.0.0");
    }
    
    #[tokio::test]
    async fn test_convenience_client_creation() {
        let result = convenience::create_client("http://localhost:8080").await;
        assert!(result.is_ok());
    }
}