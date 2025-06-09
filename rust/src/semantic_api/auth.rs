//! Authentication and Authorization for AI Agents
//!
//! This module implements the authentication and authorization system for AI agents
//! interacting with VexFS, providing secure access control and agent management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug, instrument};
use uuid::Uuid;

use crate::semantic_api::{SemanticResult, SemanticError};

/// Agent registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub agent_id: String,
    pub agent_name: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub visibility_mask: u64,
    pub max_events_per_query: usize,
    pub max_concurrent_streams: usize,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

/// Agent authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToken {
    pub agent_id: String,
    pub token_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

/// JWT claims for agent authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentClaims {
    pub sub: String,           // Agent ID
    pub iat: i64,             // Issued at
    pub exp: i64,             // Expires at
    pub jti: String,          // JWT ID (token ID)
    pub scopes: Vec<String>,  // Permissions
    pub visibility_mask: u64, // Event visibility mask
}

/// Authentication manager for AI agents
pub struct AuthManager {
    agents: Arc<RwLock<HashMap<String, AgentRegistration>>>,
    tokens: Arc<RwLock<HashMap<String, AgentToken>>>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    token_expiry_hours: u64,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(secret: &str, token_expiry_hours: u64) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        let decoding_key = DecodingKey::from_secret(secret.as_ref());
        
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            encoding_key,
            decoding_key,
            token_expiry_hours,
        }
    }

    /// Register a new AI agent
    #[instrument(skip(self))]
    pub async fn register_agent(
        &self,
        agent_id: String,
        agent_name: String,
        agent_type: String,
        capabilities: Vec<String>,
        visibility_mask: u64,
        max_events_per_query: usize,
        max_concurrent_streams: usize,
    ) -> SemanticResult<AgentRegistration> {
        let mut agents = self.agents.write().await;
        
        if agents.contains_key(&agent_id) {
            return Err(SemanticError::InvalidRequest(
                format!("Agent {} already registered", agent_id)
            ));
        }
        
        let registration = AgentRegistration {
            agent_id: agent_id.clone(),
            agent_name,
            agent_type,
            capabilities,
            visibility_mask,
            max_events_per_query,
            max_concurrent_streams,
            created_at: Utc::now(),
            last_active: Utc::now(),
        };
        
        agents.insert(agent_id, registration.clone());
        
        info!("Registered agent: {}", registration.agent_id);
        Ok(registration)
    }

    /// Unregister an AI agent
    #[instrument(skip(self))]
    pub async fn unregister_agent(&self, agent_id: &str) -> SemanticResult<()> {
        let mut agents = self.agents.write().await;
        let mut tokens = self.tokens.write().await;
        
        if agents.remove(agent_id).is_none() {
            return Err(SemanticError::InvalidRequest(
                format!("Agent {} not found", agent_id)
            ));
        }
        
        // Remove all tokens for this agent
        tokens.retain(|_, token| token.agent_id != agent_id);
        
        info!("Unregistered agent: {}", agent_id);
        Ok(())
    }

    /// Generate an authentication token for an agent
    #[instrument(skip(self))]
    pub async fn generate_token(
        &self,
        agent_id: &str,
        scopes: Vec<String>,
    ) -> SemanticResult<String> {
        let agents = self.agents.read().await;
        let agent = agents.get(agent_id)
            .ok_or_else(|| SemanticError::AuthenticationFailed(
                format!("Agent {} not found", agent_id)
            ))?;

        let now = Utc::now();
        let exp = now + chrono::Duration::hours(self.token_expiry_hours as i64);
        let token_id = Uuid::new_v4().to_string();

        let claims = AgentClaims {
            sub: agent_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            jti: token_id.clone(),
            scopes: scopes.clone(),
            visibility_mask: agent.visibility_mask,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| SemanticError::AuthenticationFailed(
                format!("Failed to generate token: {}", e)
            ))?;

        let agent_token = AgentToken {
            agent_id: agent_id.to_string(),
            token_id,
            issued_at: now,
            expires_at: exp,
            scopes,
        };

        let mut tokens = self.tokens.write().await;
        tokens.insert(agent_token.token_id.clone(), agent_token);

        info!("Generated token for agent: {}", agent_id);
        Ok(token)
    }

    /// Validate an authentication token
    #[instrument(skip(self, token))]
    pub async fn validate_token(&self, token: &str) -> SemanticResult<AgentClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<AgentClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| SemanticError::AuthenticationFailed(
                format!("Invalid token: {}", e)
            ))?;

        let claims = token_data.claims;

        // Verify token exists in our store
        let tokens = self.tokens.read().await;
        if !tokens.contains_key(&claims.jti) {
            return Err(SemanticError::AuthenticationFailed(
                "Token not found".to_string()
            ));
        }

        // Verify agent still exists
        let agents = self.agents.read().await;
        if !agents.contains_key(&claims.sub) {
            return Err(SemanticError::AuthenticationFailed(
                "Agent not found".to_string()
            ));
        }

        Ok(claims)
    }

    /// Revoke a token
    #[instrument(skip(self))]
    pub async fn revoke_token(&self, token_id: &str) -> SemanticResult<()> {
        let mut tokens = self.tokens.write().await;
        if tokens.remove(token_id).is_some() {
            info!("Revoked token: {}", token_id);
            Ok(())
        } else {
            Err(SemanticError::InvalidRequest(
                format!("Token {} not found", token_id)
            ))
        }
    }

    /// Get agent information
    pub async fn get_agent(&self, agent_id: &str) -> SemanticResult<AgentRegistration> {
        let agents = self.agents.read().await;
        agents.get(agent_id)
            .cloned()
            .ok_or_else(|| SemanticError::InvalidRequest(
                format!("Agent {} not found", agent_id)
            ))
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> SemanticResult<Vec<AgentRegistration>> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    /// Check if agent has required scope
    pub fn check_scope(&self, claims: &AgentClaims, required_scope: &str) -> SemanticResult<()> {
        if claims.scopes.contains(&required_scope.to_string()) || 
           claims.scopes.contains(&"admin".to_string()) {
            Ok(())
        } else {
            Err(SemanticError::AuthorizationFailed(
                format!("Missing required scope: {}", required_scope)
            ))
        }
    }

    /// Update agent activity timestamp
    pub async fn update_agent_activity(&self, agent_id: &str) -> SemanticResult<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.last_active = Utc::now();
        }
        Ok(())
    }

    /// Clean up expired tokens
    #[instrument(skip(self))]
    pub async fn cleanup_expired_tokens(&self) -> SemanticResult<usize> {
        let mut tokens = self.tokens.write().await;
        let now = Utc::now();
        let initial_count = tokens.len();
        
        tokens.retain(|_, token| token.expires_at > now);
        
        let removed_count = initial_count - tokens.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired tokens", removed_count);
        }
        
        Ok(removed_count)
    }
}

/// Authentication middleware for API requests
pub struct AuthMiddleware {
    auth_manager: Arc<AuthManager>,
}

impl AuthMiddleware {
    pub fn new(auth_manager: Arc<AuthManager>) -> Self {
        Self { auth_manager }
    }

    /// Validate request and extract claims
    pub async fn validate_request(&self, auth_header: Option<&str>) -> SemanticResult<AgentClaims> {
        let auth_header = auth_header.ok_or_else(|| {
            SemanticError::AuthenticationFailed("Missing authorization header".to_string())
        })?;

        if !auth_header.starts_with("Bearer ") {
            return Err(SemanticError::AuthenticationFailed(
                "Invalid authorization header format".to_string()
            ));
        }

        let token = &auth_header[7..]; // Remove "Bearer " prefix
        self.auth_manager.validate_token(token).await
    }

    /// Authorize request with required scope
    pub async fn authorize_request(
        &self,
        auth_header: Option<&str>,
        required_scope: &str,
    ) -> SemanticResult<AgentClaims> {
        let claims = self.validate_request(auth_header).await?;
        self.auth_manager.check_scope(&claims, required_scope)?;
        
        // Update agent activity
        self.auth_manager.update_agent_activity(&claims.sub).await?;
        
        Ok(claims)
    }
}

/// Standard scopes for agent permissions
pub mod scopes {
    pub const READ_EVENTS: &str = "read:events";
    pub const WRITE_EVENTS: &str = "write:events";
    pub const QUERY_EVENTS: &str = "query:events";
    pub const STREAM_EVENTS: &str = "stream:events";
    pub const MANAGE_AGENTS: &str = "manage:agents";
    pub const ADMIN: &str = "admin";
}

/// Agent registration request
#[derive(Debug, Deserialize)]
pub struct AgentRegistrationRequest {
    pub agent_id: String,
    pub agent_name: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub visibility_mask: Option<u64>,
    pub max_events_per_query: Option<usize>,
    pub max_concurrent_streams: Option<usize>,
}

/// Agent authentication request
#[derive(Debug, Deserialize)]
pub struct AgentAuthRequest {
    pub agent_id: String,
    pub scopes: Vec<String>,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_registration() {
        let auth_manager = AuthManager::new("test_secret", 24);
        
        let result = auth_manager.register_agent(
            "test_agent".to_string(),
            "Test Agent".to_string(),
            "reasoning".to_string(),
            vec!["query".to_string(), "analyze".to_string()],
            0xFF,
            1000,
            5,
        ).await;
        
        assert!(result.is_ok());
        let registration = result.unwrap();
        assert_eq!(registration.agent_id, "test_agent");
        assert_eq!(registration.agent_name, "Test Agent");
    }

    #[tokio::test]
    async fn test_token_generation_and_validation() {
        let auth_manager = AuthManager::new("test_secret", 24);
        
        // Register agent first
        auth_manager.register_agent(
            "test_agent".to_string(),
            "Test Agent".to_string(),
            "reasoning".to_string(),
            vec!["query".to_string()],
            0xFF,
            1000,
            5,
        ).await.unwrap();
        
        // Generate token
        let token = auth_manager.generate_token(
            "test_agent",
            vec!["read:events".to_string()],
        ).await.unwrap();
        
        // Validate token
        let claims = auth_manager.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "test_agent");
        assert!(claims.scopes.contains(&"read:events".to_string()));
    }

    #[tokio::test]
    async fn test_scope_checking() {
        let auth_manager = AuthManager::new("test_secret", 24);
        
        let claims = AgentClaims {
            sub: "test_agent".to_string(),
            iat: 0,
            exp: 0,
            jti: "test_token".to_string(),
            scopes: vec!["read:events".to_string()],
            visibility_mask: 0xFF,
        };
        
        // Should succeed with correct scope
        assert!(auth_manager.check_scope(&claims, "read:events").is_ok());
        
        // Should fail with missing scope
        assert!(auth_manager.check_scope(&claims, "write:events").is_err());
    }
}