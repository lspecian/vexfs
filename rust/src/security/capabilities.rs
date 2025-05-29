/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

//! Capability-Based Access Control for VexFS
//!
//! This module implements capability-based security for IOCTL operations
//! and vector-specific operations, extending the existing security validation
//! from Task 7.3 with enhanced privilege escalation detection and rate limiting.

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
};
use super::{SecurityError, SecurityContext};

#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap as HashMap;

#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, vec, string::String, format};
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, vec, string::{String, ToString}, format};

/// Maximum number of IOCTL operations per second per user
pub const MAX_IOCTL_RATE_PER_SECOND: u32 = 100;

/// Maximum number of failed authentication attempts before lockout
pub const MAX_AUTH_FAILURES: u32 = 5;

/// Lockout duration in seconds after max failures
pub const AUTH_LOCKOUT_DURATION: u64 = 300; // 5 minutes

/// Security levels for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SecurityLevel {
    /// Guest level - minimal privileges
    Guest = 0,
    /// Regular user level
    User = 1,
    /// Power user level - additional privileges
    PowerUser = 2,
    /// Administrator level - most privileges
    Admin = 3,
    /// System level - full privileges
    System = 4,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        Self::User
    }
}

/// Individual capabilities that can be granted or revoked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Capability {
    // Vector operation capabilities
    VectorRead = 0x0001,
    VectorWrite = 0x0002,
    VectorCreate = 0x0004,
    VectorDelete = 0x0008,
    VectorSearch = 0x0010,
    VectorEncrypt = 0x0020,
    VectorDecrypt = 0x0040,
    
    // Index operation capabilities
    IndexCreate = 0x0100,
    IndexRebuild = 0x0200,
    IndexOptimize = 0x0400,
    IndexDelete = 0x0800,
    
    // ACL capabilities
    AclRead = 0x1000,
    AclModify = 0x2000,
    
    // Administrative capabilities
    KeyManagement = 0x10000,
    SecurityAudit = 0x20000,
    SystemConfig = 0x40000,
    
    // IOCTL-specific capabilities
    IoctlBasic = 0x100000,
    IoctlAdvanced = 0x200000,
    IoctlAdmin = 0x400000,
    
    // Debug and development capabilities
    DebugAccess = 0x1000000,
    RawAccess = 0x2000000,
}

/// Set of capabilities
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    /// Bitmask of capabilities
    capabilities: u64,
}

impl CapabilitySet {
    /// Create an empty capability set
    pub fn new() -> Self {
        Self { capabilities: 0 }
    }

    /// Create capability set with all capabilities
    pub fn all() -> Self {
        Self { capabilities: u64::MAX }
    }

    /// Create capability set for a specific security level
    pub fn for_level(level: SecurityLevel) -> Self {
        let mut caps = Self::new();
        
        match level {
            SecurityLevel::Guest => {
                caps.add(Capability::VectorRead);
                caps.add(Capability::VectorSearch);
                caps.add(Capability::IoctlBasic);
            }
            SecurityLevel::User => {
                caps.add(Capability::VectorRead);
                caps.add(Capability::VectorWrite);
                caps.add(Capability::VectorCreate);
                caps.add(Capability::VectorSearch);
                caps.add(Capability::VectorEncrypt);
                caps.add(Capability::VectorDecrypt);
                caps.add(Capability::AclRead);
                caps.add(Capability::IoctlBasic);
                caps.add(Capability::IoctlAdvanced);
            }
            SecurityLevel::PowerUser => {
                caps = Self::for_level(SecurityLevel::User);
                caps.add(Capability::VectorDelete);
                caps.add(Capability::IndexCreate);
                caps.add(Capability::IndexOptimize);
                caps.add(Capability::AclModify);
            }
            SecurityLevel::Admin => {
                caps = Self::for_level(SecurityLevel::PowerUser);
                caps.add(Capability::IndexRebuild);
                caps.add(Capability::IndexDelete);
                caps.add(Capability::KeyManagement);
                caps.add(Capability::SecurityAudit);
                caps.add(Capability::IoctlAdmin);
            }
            SecurityLevel::System => {
                caps = Self::all();
            }
        }
        
        caps
    }

    /// Check if capability is present
    pub fn has(&self, capability: Capability) -> bool {
        (self.capabilities & (capability as u64)) != 0
    }

    /// Add a capability
    pub fn add(&mut self, capability: Capability) {
        self.capabilities |= capability as u64;
    }

    /// Remove a capability
    pub fn remove(&mut self, capability: Capability) {
        self.capabilities &= !(capability as u64);
    }

    /// Add multiple capabilities
    pub fn add_multiple(&mut self, capabilities: &[Capability]) {
        for &cap in capabilities {
            self.add(cap);
        }
    }

    /// Check if all required capabilities are present
    pub fn has_all(&self, required: &[Capability]) -> bool {
        required.iter().all(|&cap| self.has(cap))
    }

    /// Check if any of the capabilities are present
    pub fn has_any(&self, capabilities: &[Capability]) -> bool {
        capabilities.iter().any(|&cap| self.has(cap))
    }

    /// Intersect with another capability set
    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            capabilities: self.capabilities & other.capabilities,
        }
    }

    /// Union with another capability set
    pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            capabilities: self.capabilities | other.capabilities,
        }
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::for_level(SecurityLevel::User)
    }
}

/// Rate limiting information for a user
#[derive(Debug, Clone)]
struct RateLimitInfo {
    /// Number of operations in current window
    operation_count: u32,
    /// Start of current time window
    window_start: u64,
    /// Number of consecutive failures
    failure_count: u32,
    /// Timestamp of last failure
    last_failure: u64,
    /// Whether user is currently locked out
    locked_out: bool,
    /// Lockout expiration time
    lockout_expires: u64,
}

impl RateLimitInfo {
    fn new() -> Self {
        Self {
            operation_count: 0,
            window_start: 0,
            failure_count: 0,
            last_failure: 0,
            locked_out: false,
            lockout_expires: 0,
        }
    }
}

/// Privilege escalation detection patterns
#[derive(Debug, Clone)]
pub struct EscalationPattern {
    /// Pattern name
    pub name: String,
    /// Sequence of operations that indicate escalation
    pub operations: Vec<String>,
    /// Time window for pattern detection (seconds)
    pub time_window: u64,
    /// Severity level
    pub severity: u8,
}

/// Privilege escalation detector
pub struct PrivilegeEscalationDetector {
    /// Known escalation patterns
    patterns: Vec<EscalationPattern>,
    /// Recent operations by user
    user_operations: HashMap<UserId, Vec<(String, u64)>>,
    /// Detected escalation attempts
    escalation_attempts: Vec<EscalationAttempt>,
}

/// Detected escalation attempt
#[derive(Debug, Clone)]
pub struct EscalationAttempt {
    /// User ID
    pub user_id: UserId,
    /// Pattern that was matched
    pub pattern_name: String,
    /// Timestamp of detection
    pub detected_at: u64,
    /// Operations that triggered the detection
    pub operations: Vec<String>,
    /// Severity level
    pub severity: u8,
}

impl PrivilegeEscalationDetector {
    /// Create a new privilege escalation detector
    pub fn new() -> Self {
        let mut detector = Self {
            patterns: Vec::new(),
            user_operations: HashMap::new(),
            escalation_attempts: Vec::new(),
        };
        
        detector.load_default_patterns();
        detector
    }

    /// Load default escalation patterns
    fn load_default_patterns(&mut self) {
        // Rapid capability enumeration
        self.patterns.push(EscalationPattern {
            name: "rapid_capability_enum".to_string(),
            operations: vec![
                "ioctl_get_status".to_string(),
                "ioctl_get_index_info".to_string(),
                "ioctl_validate_index".to_string(),
            ],
            time_window: 5,
            severity: 3,
        });

        // Administrative operation attempts
        self.patterns.push(EscalationPattern {
            name: "admin_operation_attempt".to_string(),
            operations: vec![
                "ioctl_manage_index".to_string(),
                "ioctl_set_search_params".to_string(),
            ],
            time_window: 10,
            severity: 5,
        });
    }

    /// Record an operation for escalation detection
    pub fn record_operation(&mut self, user_id: UserId, operation: &str, timestamp: u64) {
        let user_ops = self.user_operations.entry(user_id).or_insert_with(Vec::new);
        user_ops.push((operation.to_string(), timestamp));

        // Keep only recent operations (last hour)
        user_ops.retain(|(_, ts)| timestamp - *ts <= 3600);

        // Check for escalation patterns
        self.check_patterns(user_id, timestamp);
    }

    /// Check for escalation patterns
    fn check_patterns(&mut self, user_id: UserId, current_time: u64) {
        if let Some(user_ops) = self.user_operations.get(&user_id) {
            for pattern in &self.patterns {
                if self.matches_pattern(user_ops, pattern, current_time) {
                    let attempt = EscalationAttempt {
                        user_id,
                        pattern_name: pattern.name.clone(),
                        detected_at: current_time,
                        operations: pattern.operations.clone(),
                        severity: pattern.severity,
                    };
                    
                    self.escalation_attempts.push(attempt);
                }
            }
        }
    }

    /// Check if operations match a pattern
    fn matches_pattern(
        &self,
        operations: &[(String, u64)],
        pattern: &EscalationPattern,
        current_time: u64,
    ) -> bool {
        if pattern.operations.is_empty() {
            return false;
        }

        // Find operations within the time window
        let recent_ops: Vec<&String> = operations
            .iter()
            .filter(|(_, ts)| current_time - *ts <= pattern.time_window)
            .map(|(op, _)| op)
            .collect();

        // Check if pattern sequence is present
        let mut pattern_index = 0;
        for op in recent_ops {
            if pattern_index < pattern.operations.len() && op == &pattern.operations[pattern_index] {
                pattern_index += 1;
                if pattern_index == pattern.operations.len() {
                    return true;
                }
            }
        }

        false
    }

    /// Get recent escalation attempts for a user
    pub fn get_user_attempts(&self, user_id: UserId, since: u64) -> Vec<&EscalationAttempt> {
        self.escalation_attempts
            .iter()
            .filter(|attempt| attempt.user_id == user_id && attempt.detected_at >= since)
            .collect()
    }

    /// Clear old escalation attempts
    pub fn cleanup_old_attempts(&mut self, before: u64) {
        self.escalation_attempts.retain(|attempt| attempt.detected_at >= before);
    }
}

/// IOCTL security validator with enhanced capabilities
pub struct IoctlSecurityValidator {
    /// Rate limiting information by user
    rate_limits: HashMap<UserId, RateLimitInfo>,
    /// Privilege escalation detector
    escalation_detector: PrivilegeEscalationDetector,
    /// Security configuration
    config: SecurityValidatorConfig,
}

/// Security validator configuration
#[derive(Debug, Clone)]
pub struct SecurityValidatorConfig {
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable privilege escalation detection
    pub enable_escalation_detection: bool,
    /// Maximum operations per second
    pub max_ops_per_second: u32,
    /// Maximum authentication failures
    pub max_auth_failures: u32,
    /// Lockout duration in seconds
    pub lockout_duration: u64,
}

impl Default for SecurityValidatorConfig {
    fn default() -> Self {
        Self {
            enable_rate_limiting: true,
            enable_escalation_detection: true,
            max_ops_per_second: MAX_IOCTL_RATE_PER_SECOND,
            max_auth_failures: MAX_AUTH_FAILURES,
            lockout_duration: AUTH_LOCKOUT_DURATION,
        }
    }
}

impl IoctlSecurityValidator {
    /// Create a new IOCTL security validator
    pub fn new() -> Self {
        Self {
            rate_limits: HashMap::new(),
            escalation_detector: PrivilegeEscalationDetector::new(),
            config: SecurityValidatorConfig::default(),
        }
    }

    /// Validate IOCTL operation security
    pub fn validate_operation(
        &mut self,
        context: &SecurityContext,
        ioctl_cmd: u8,
        data_size: usize,
        current_time: u64,
    ) -> VexfsResult<()> {
        let user_id = context.user.uid;

        // Check if user is locked out
        if self.config.enable_rate_limiting {
            self.check_lockout(user_id, current_time)?;
        }

        // Check rate limiting
        if self.config.enable_rate_limiting {
            self.check_rate_limit(user_id, current_time)?;
        }

        // Check capability requirements
        self.check_ioctl_capabilities(context, ioctl_cmd)?;

        // Validate data size
        self.validate_data_size(ioctl_cmd, data_size)?;

        // Record operation for escalation detection
        if self.config.enable_escalation_detection {
            let operation_name = self.ioctl_cmd_to_name(ioctl_cmd);
            self.escalation_detector.record_operation(user_id, &operation_name, current_time);
        }

        // Update rate limiting counters
        if self.config.enable_rate_limiting {
            self.update_rate_limit(user_id, current_time);
        }

        Ok(())
    }

    /// Check if user is currently locked out
    fn check_lockout(&mut self, user_id: UserId, current_time: u64) -> VexfsResult<()> {
        if let Some(rate_info) = self.rate_limits.get_mut(&user_id) {
            if rate_info.locked_out {
                if current_time >= rate_info.lockout_expires {
                    // Lockout expired, reset
                    rate_info.locked_out = false;
                    rate_info.failure_count = 0;
                } else {
                    return Err(VexfsError::PermissionDenied(
                        format!("User {} is locked out until {}", user_id, rate_info.lockout_expires)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Check rate limiting
    fn check_rate_limit(&mut self, user_id: UserId, current_time: u64) -> VexfsResult<()> {
        let rate_info = self.rate_limits.entry(user_id).or_insert_with(RateLimitInfo::new);

        // Reset window if needed (1-second windows)
        if current_time - rate_info.window_start >= 1 {
            rate_info.operation_count = 0;
            rate_info.window_start = current_time;
        }

        // Check if rate limit exceeded
        if rate_info.operation_count >= self.config.max_ops_per_second {
            return Err(VexfsError::ResourceLimit("Rate limit exceeded".into()));
        }

        Ok(())
    }

    /// Update rate limiting counters
    fn update_rate_limit(&mut self, user_id: UserId, current_time: u64) {
        if let Some(rate_info) = self.rate_limits.get_mut(&user_id) {
            rate_info.operation_count += 1;
        }
    }

    /// Check IOCTL capability requirements
    fn check_ioctl_capabilities(&self, context: &SecurityContext, ioctl_cmd: u8) -> VexfsResult<()> {
        let required_caps = self.get_required_capabilities(ioctl_cmd);
        
        if !context.capabilities.has_all(&required_caps) {
            return Err(VexfsError::PermissionDenied(
                format!("Missing required capabilities for IOCTL 0x{:02x}", ioctl_cmd)
            ));
        }

        Ok(())
    }

    /// Get required capabilities for an IOCTL command
    fn get_required_capabilities(&self, ioctl_cmd: u8) -> Vec<Capability> {
        match ioctl_cmd {
            // Basic vector operations
            0x01 => vec![Capability::VectorCreate, Capability::IoctlBasic], // ADD_EMBEDDING
            0x02 => vec![Capability::VectorRead, Capability::IoctlBasic],   // GET_EMBEDDING
            0x03 => vec![Capability::VectorWrite, Capability::IoctlBasic],  // UPDATE_EMBEDDING
            0x04 => vec![Capability::VectorDelete, Capability::IoctlBasic], // DELETE_EMBEDDING
            0x05 => vec![Capability::VectorSearch, Capability::IoctlBasic], // VECTOR_SEARCH
            0x06 => vec![Capability::VectorSearch, Capability::IoctlAdvanced], // HYBRID_SEARCH
            
            // Index management operations
            0x07 => vec![Capability::IndexCreate, Capability::IoctlAdvanced], // MANAGE_INDEX
            
            // Administrative operations
            0x10 => vec![Capability::IoctlBasic],                           // GET_STATUS
            0x11 => vec![Capability::VectorSearch, Capability::IoctlAdvanced], // BATCH_SEARCH
            0x12 => vec![Capability::SystemConfig, Capability::IoctlAdmin], // SET_SEARCH_PARAMS
            0x13 => vec![Capability::IoctlBasic],                           // GET_INDEX_INFO
            0x14 => vec![Capability::SecurityAudit, Capability::IoctlAdvanced], // VALIDATE_INDEX
            
            // Unknown commands require admin privileges
            _ => vec![Capability::IoctlAdmin],
        }
    }

    /// Validate data size for IOCTL operation
    fn validate_data_size(&self, ioctl_cmd: u8, data_size: usize) -> VexfsResult<()> {
        let max_size = match ioctl_cmd {
            0x01 | 0x03 => 32 * 1024 * 1024, // 32MB for vector data
            0x05 | 0x06 => 1024 * 1024,      // 1MB for search queries
            0x11 => 10 * 1024 * 1024,        // 10MB for batch operations
            _ => 64 * 1024,                   // 64KB for other operations
        };

        if data_size > max_size {
            return Err(VexfsError::InvalidArgument(
                format!("Data size {} exceeds maximum {} for IOCTL 0x{:02x}", 
                       data_size, max_size, ioctl_cmd)
            ));
        }

        Ok(())
    }

    /// Convert IOCTL command to operation name
    fn ioctl_cmd_to_name(&self, ioctl_cmd: u8) -> String {
        match ioctl_cmd {
            0x01 => "ioctl_add_embedding".to_string(),
            0x02 => "ioctl_get_embedding".to_string(),
            0x03 => "ioctl_update_embedding".to_string(),
            0x04 => "ioctl_delete_embedding".to_string(),
            0x05 => "ioctl_vector_search".to_string(),
            0x06 => "ioctl_hybrid_search".to_string(),
            0x07 => "ioctl_manage_index".to_string(),
            0x10 => "ioctl_get_status".to_string(),
            0x11 => "ioctl_batch_search".to_string(),
            0x12 => "ioctl_set_search_params".to_string(),
            0x13 => "ioctl_get_index_info".to_string(),
            0x14 => "ioctl_validate_index".to_string(),
            _ => format!("ioctl_unknown_0x{:02x}", ioctl_cmd),
        }
    }

    /// Cleanup old data
    pub fn cleanup(&mut self, current_time: u64) {
        // Remove old rate limit entries
        self.rate_limits.retain(|_, info| {
            current_time - info.window_start <= 3600 // Keep for 1 hour
        });

        // Cleanup old escalation attempts
        self.escalation_detector.cleanup_old_attempts(current_time - 86400); // Keep for 24 hours
    }
}

/// Main capability manager
pub struct CapabilityManager {
    /// IOCTL security validator
    ioctl_validator: IoctlSecurityValidator,
    /// User capability assignments
    user_capabilities: HashMap<UserId, CapabilitySet>,
    /// Default capabilities by security level
    level_capabilities: HashMap<SecurityLevel, CapabilitySet>,
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new() -> Self {
        let mut manager = Self {
            ioctl_validator: IoctlSecurityValidator::new(),
            user_capabilities: HashMap::new(),
            level_capabilities: HashMap::new(),
        };

        // Initialize default capabilities for each level
        for level in [SecurityLevel::Guest, SecurityLevel::User, SecurityLevel::PowerUser, 
                     SecurityLevel::Admin, SecurityLevel::System] {
            manager.level_capabilities.insert(level, CapabilitySet::for_level(level));
        }

        manager
    }

    /// Validate IOCTL operation
    pub fn validate_ioctl_operation(
        &mut self,
        context: &SecurityContext,
        ioctl_cmd: u8,
        data_size: usize,
    ) -> VexfsResult<()> {
        let current_time = 0; // TODO: get current time
        self.ioctl_validator.validate_operation(context, ioctl_cmd, data_size, current_time)
    }

    /// Set capabilities for a user
    pub fn set_user_capabilities(&mut self, user_id: UserId, capabilities: CapabilitySet) {
        self.user_capabilities.insert(user_id, capabilities);
    }

    /// Get effective capabilities for a user
    pub fn get_user_capabilities(&self, user_id: UserId, security_level: SecurityLevel) -> CapabilitySet {
        // Start with level-based capabilities
        let mut caps = self.level_capabilities.get(&security_level)
            .cloned()
            .unwrap_or_else(|| CapabilitySet::for_level(security_level));

        // Add user-specific capabilities
        if let Some(user_caps) = self.user_capabilities.get(&user_id) {
            caps = caps.union(user_caps);
        }

        caps
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    /// Total number of users with custom capabilities
    pub total_users: usize,
    /// Number of currently locked out users
    pub locked_out_users: usize,
    /// Number of escalation attempts detected
    pub escalation_attempts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_core::permissions::UserContext;

    #[test]
    fn test_capability_set() {
        let mut caps = CapabilitySet::new();
        assert!(!caps.has(Capability::VectorRead));

        caps.add(Capability::VectorRead);
        assert!(caps.has(Capability::VectorRead));

        caps.remove(Capability::VectorRead);
        assert!(!caps.has(Capability::VectorRead));
    }

    #[test]
    fn test_security_levels() {
        let guest_caps = CapabilitySet::for_level(SecurityLevel::Guest);
        let user_caps = CapabilitySet::for_level(SecurityLevel::User);
        let admin_caps = CapabilitySet::for_level(SecurityLevel::Admin);

        assert!(guest_caps.has(Capability::VectorRead));
        assert!(!guest_caps.has(Capability::VectorWrite));

        assert!(user_caps.has(Capability::VectorRead));
        assert!(user_caps.has(Capability::VectorWrite));
        assert!(!user_caps.has(Capability::KeyManagement));

        assert!(admin_caps.has(Capability::VectorRead));
        assert!(admin_caps.has(Capability::VectorWrite));
        assert!(admin_caps.has(Capability::KeyManagement));
    }

    #[test]
    fn test_escalation_detection() {
        let mut detector = PrivilegeEscalationDetector::new();
        
        // Record operations that match a pattern
        detector.record_operation(1000, "ioctl_get_status", 100);
        detector.record_operation(1000, "ioctl_get_index_info", 102);
        detector.record_operation(1000, "ioctl_validate_index", 104);
        
        let attempts = detector.get_user_attempts(1000, 90);
        assert!(!attempts.is_empty());
    }
}