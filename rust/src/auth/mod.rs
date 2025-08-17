//! Authentication and authorization module for VexFS API server
//! 
//! Provides JWT-based authentication with API keys and role-based access control

#[cfg(feature = "server")]
use axum::{
    extract::{FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[cfg(feature = "server")]
use chrono::{Duration, Utc};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID or API key ID)
    pub sub: String,
    /// Expiration time (as UTC timestamp)
    pub exp: i64,
    /// Issued at (as UTC timestamp)
    pub iat: i64,
    /// Not before (as UTC timestamp)
    pub nbf: i64,
    /// User role
    pub role: UserRole,
    /// Allowed collections (None = all)
    pub collections: Option<Vec<String>>,
    /// Rate limit (requests per minute)
    pub rate_limit: Option<u32>,
}

/// User roles for authorization
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Read-only access
    Reader,
    /// Read and write access
    Writer,
    /// Full administrative access
    Admin,
}

impl UserRole {
    /// Check if role has read permission
    pub fn can_read(&self) -> bool {
        true // All roles can read
    }
    
    /// Check if role has write permission
    pub fn can_write(&self) -> bool {
        matches!(self, UserRole::Writer | UserRole::Admin)
    }
    
    /// Check if role has admin permission
    pub fn can_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
}

/// Authentication configuration
#[derive(Clone)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,
    /// Token expiration duration
    pub token_expiration: Duration,
    /// Enable anonymous access (read-only)
    pub allow_anonymous: bool,
    /// API keys with their roles
    pub api_keys: Arc<std::collections::HashMap<String, ApiKeyInfo>>,
}

/// API key information
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub key_id: String,
    pub role: UserRole,
    pub collections: Option<Vec<String>>,
    pub rate_limit: Option<u32>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret-change-in-production".to_string()),
            token_expiration: Duration::hours(24),
            allow_anonymous: std::env::var("ALLOW_ANONYMOUS").unwrap_or_else(|_| "true".to_string()) == "true",
            api_keys: Arc::new(Self::load_api_keys()),
        }
    }
}

impl AuthConfig {
    /// Load API keys from environment or configuration
    fn load_api_keys() -> std::collections::HashMap<String, ApiKeyInfo> {
        let mut keys = std::collections::HashMap::new();
        
        // Load from environment variables
        // Format: API_KEY_1=key:role:collections:rate_limit
        for i in 1..=10 {
            let env_var = format!("API_KEY_{}", i);
            if let Ok(value) = std::env::var(&env_var) {
                if let Some((key, info)) = Self::parse_api_key(&value) {
                    keys.insert(key, info);
                }
            }
        }
        
        // Add default admin key if no keys configured
        if keys.is_empty() && std::env::var("PRODUCTION").unwrap_or_default() != "true" {
            keys.insert(
                "vexfs-default-key".to_string(),
                ApiKeyInfo {
                    key_id: "default".to_string(),
                    role: UserRole::Admin,
                    collections: None,
                    rate_limit: None,
                },
            );
        }
        
        keys
    }
    
    /// Parse API key from configuration string
    fn parse_api_key(value: &str) -> Option<(String, ApiKeyInfo)> {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() < 2 {
            return None;
        }
        
        let key = parts[0].to_string();
        let role = match parts[1] {
            "admin" => UserRole::Admin,
            "writer" => UserRole::Writer,
            "reader" => UserRole::Reader,
            _ => return None,
        };
        
        let collections = if parts.len() > 2 && !parts[2].is_empty() {
            Some(parts[2].split(',').map(|s| s.to_string()).collect())
        } else {
            None
        };
        
        let rate_limit = if parts.len() > 3 {
            parts[3].parse().ok()
        } else {
            None
        };
        
        let key_id = format!("api_key_{}", key.chars().take(8).collect::<String>());
        
        Some((key, ApiKeyInfo {
            key_id,
            role,
            collections,
            rate_limit,
        }))
    }
    
    /// Generate JWT token (simplified implementation)
    pub fn generate_token(&self, claims: &Claims) -> Result<String, AuthError> {
        // For now, use a simple base64 encoding - in production use proper JWT library
        let json = serde_json::to_string(claims)
            .map_err(|_| AuthError::TokenGenerationFailed)?;
        
        use base64::{engine::general_purpose::STANDARD, Engine};
        let token = STANDARD.encode(json.as_bytes());
        Ok(format!("Bearer {}", token))
    }
    
    /// Validate JWT token (simplified implementation)
    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        // For now, use simple base64 decoding - in production use proper JWT library
        let token = token.strip_prefix("Bearer ").unwrap_or(token);
        
        use base64::{engine::general_purpose::STANDARD, Engine};
        let decoded = STANDARD.decode(token)
            .map_err(|_| AuthError::InvalidToken)?;
        
        let json = String::from_utf8(decoded)
            .map_err(|_| AuthError::InvalidToken)?;
        
        let claims: Claims = serde_json::from_str(&json)
            .map_err(|_| AuthError::InvalidToken)?;
        
        // Check expiration
        let now = Utc::now().timestamp();
        if claims.exp < now {
            return Err(AuthError::TokenExpired);
        }
        
        Ok(claims)
    }
    
    /// Authenticate with API key
    pub fn authenticate_api_key(&self, api_key: &str) -> Result<Claims, AuthError> {
        let info = self.api_keys
            .get(api_key)
            .ok_or(AuthError::InvalidApiKey)?;
        
        let now = Utc::now();
        Ok(Claims {
            sub: info.key_id.clone(),
            exp: (now + self.token_expiration).timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            role: info.role.clone(),
            collections: info.collections.clone(),
            rate_limit: info.rate_limit,
        })
    }
}

/// Authentication errors
#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    InvalidApiKey,
    TokenExpired,
    TokenGenerationFailed,
    Unauthorized,
    Forbidden,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired"),
            AuthError::TokenGenerationFailed => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate token"),
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
        };
        
        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}

/// Authenticated user extractor for Axum
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: Claims,
}

/// Implementation of FromRequestParts for automatic extraction in handlers
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract JWT from Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok());
        
        if let Some(bearer) = auth_header {
            // Validate JWT token
            let config = AuthConfig::default();
            let claims = config.validate_token(bearer)?;
            
            // Check if token is expired
            let now = Utc::now().timestamp();
            if claims.exp < now {
                return Err(AuthError::TokenExpired);
            }
            
            return Ok(AuthUser { claims });
        }
        
        // Try to extract API key from X-API-Key header
        let api_key = parts
            .headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok());
        
        if let Some(key) = api_key {
            let config = AuthConfig::default();
            let claims = config.authenticate_api_key(key)?;
            return Ok(AuthUser { claims });
        }
        
        // Check if anonymous access is allowed
        let config = AuthConfig::default();
        if config.allow_anonymous {
            let now = Utc::now();
            return Ok(AuthUser {
                claims: Claims {
                    sub: "anonymous".to_string(),
                    exp: (now + Duration::hours(1)).timestamp(),
                    iat: now.timestamp(),
                    nbf: now.timestamp(),
                    role: UserRole::Reader,
                    collections: None,
                    rate_limit: Some(10), // Low rate limit for anonymous
                },
            });
        }
        
        Err(AuthError::Unauthorized)
    }
}

/// Optional authentication extractor (allows anonymous access)
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuth(AuthUser::from_request_parts(parts, state).await.ok()))
    }
}

/// Middleware to check collection access
pub fn check_collection_access(user: &AuthUser, collection: &str) -> Result<(), AuthError> {
    // Admin can access everything
    if user.claims.role.can_admin() {
        return Ok(());
    }
    
    // Check if user has access to this collection
    if let Some(allowed) = &user.claims.collections {
        if allowed.contains(&collection.to_string()) {
            return Ok(());
        }
        return Err(AuthError::Forbidden);
    }
    
    // No collection restrictions means access to all
    Ok(())
}

/// Generate an API key
pub fn generate_api_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    
    (0..32)
        .map(|_| {
            let idx = fastrand::usize(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_role_permissions() {
        assert!(UserRole::Reader.can_read());
        assert!(!UserRole::Reader.can_write());
        assert!(!UserRole::Reader.can_admin());
        
        assert!(UserRole::Writer.can_read());
        assert!(UserRole::Writer.can_write());
        assert!(!UserRole::Writer.can_admin());
        
        assert!(UserRole::Admin.can_read());
        assert!(UserRole::Admin.can_write());
        assert!(UserRole::Admin.can_admin());
    }
    
    #[test]
    fn test_api_key_parsing() {
        let (key, info) = AuthConfig::parse_api_key("mykey:admin:coll1,coll2:100").unwrap();
        assert_eq!(key, "mykey");
        assert_eq!(info.role, UserRole::Admin);
        assert_eq!(info.collections.unwrap().len(), 2);
        assert_eq!(info.rate_limit.unwrap(), 100);
    }
}