//! Minimal auth module for when server feature is not enabled

use serde::{Deserialize, Serialize};

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