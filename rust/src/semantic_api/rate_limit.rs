//! Rate Limiting for AI Agent Interactions
//!
//! This module implements sophisticated rate limiting for AI agents to ensure
//! fair resource usage and prevent abuse of the VexFS semantic API.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};

use crate::semantic_api::{SemanticResult, SemanticError};

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute per agent
    pub requests_per_minute_per_agent: u32,
    /// Burst size per agent (allows temporary spikes)
    pub burst_size_per_agent: u32,
    /// Events per minute per agent
    pub events_per_minute_per_agent: u32,
    /// Concurrent streams per agent
    pub max_concurrent_streams_per_agent: u32,
    /// Query complexity limit per agent
    pub max_query_complexity_per_agent: u32,
    /// Bandwidth limit per agent (bytes per minute)
    pub bandwidth_limit_per_agent: u64,
    /// Global rate limits
    pub global_requests_per_minute: u32,
    pub global_events_per_minute: u32,
    /// Cleanup interval for inactive agents
    pub cleanup_interval_minutes: u32,
    /// Agent inactivity timeout
    pub agent_timeout_minutes: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute_per_agent: 1000,
            burst_size_per_agent: 100,
            events_per_minute_per_agent: 10000,
            max_concurrent_streams_per_agent: 10,
            max_query_complexity_per_agent: 1000,
            bandwidth_limit_per_agent: 10 * 1024 * 1024, // 10 MB/min
            global_requests_per_minute: 50000,
            global_events_per_minute: 500000,
            cleanup_interval_minutes: 15,
            agent_timeout_minutes: 60,
        }
    }
}

/// Rate limiter for API requests
pub struct ApiRateLimiter {
    config: RateLimitConfig,
    agent_limiters: Arc<RwLock<HashMap<String, AgentRateLimiter>>>,
    global_limiter: Arc<RwLock<GlobalRateLimiter>>,
    stats: Arc<RwLock<RateLimitStats>>,
}

/// Per-agent rate limiter
#[derive(Debug)]
struct AgentRateLimiter {
    agent_id: String,
    request_limiter: TokenBucket,
    event_limiter: TokenBucket,
    bandwidth_limiter: TokenBucket,
    concurrent_streams: u32,
    query_complexity_used: u32,
    last_activity: Instant,
    total_requests: u64,
    total_events: u64,
    total_bytes: u64,
}

/// Global rate limiter
#[derive(Debug)]
struct GlobalRateLimiter {
    request_limiter: TokenBucket,
    event_limiter: TokenBucket,
    total_requests: u64,
    total_events: u64,
}

/// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    capacity: u32,
    tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

/// Rate limiting statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStats {
    pub active_agents: u32,
    pub total_requests_blocked: u64,
    pub total_events_blocked: u64,
    pub total_bandwidth_blocked: u64,
    pub global_requests_blocked: u64,
    pub global_events_blocked: u64,
    pub last_cleanup: Option<DateTime<Utc>>,
    pub average_request_rate: f64,
    pub average_event_rate: f64,
}

impl Default for RateLimitStats {
    fn default() -> Self {
        Self {
            active_agents: 0,
            total_requests_blocked: 0,
            total_events_blocked: 0,
            total_bandwidth_blocked: 0,
            global_requests_blocked: 0,
            global_events_blocked: 0,
            last_cleanup: None,
            average_request_rate: 0.0,
            average_event_rate: 0.0,
        }
    }
}

/// Rate limit violation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitViolation {
    pub violation_type: ViolationType,
    pub agent_id: String,
    pub current_rate: f64,
    pub limit: f64,
    pub retry_after_seconds: u64,
    pub message: String,
}

/// Types of rate limit violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    RequestRate,
    EventRate,
    Bandwidth,
    ConcurrentStreams,
    QueryComplexity,
    GlobalRequestRate,
    GlobalEventRate,
}

impl ApiRateLimiter {
    /// Create a new API rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let global_limiter = GlobalRateLimiter {
            request_limiter: TokenBucket::new(
                config.global_requests_per_minute,
                config.global_requests_per_minute as f64 / 60.0,
            ),
            event_limiter: TokenBucket::new(
                config.global_events_per_minute,
                config.global_events_per_minute as f64 / 60.0,
            ),
            total_requests: 0,
            total_events: 0,
        };

        Self {
            config,
            agent_limiters: Arc::new(RwLock::new(HashMap::new())),
            global_limiter: Arc::new(RwLock::new(global_limiter)),
            stats: Arc::new(RwLock::new(RateLimitStats::default())),
        }
    }

    /// Check if a request is allowed for an agent
    #[instrument(skip(self))]
    pub async fn check_request_allowed(&self, agent_id: &str) -> SemanticResult<()> {
        // Check global limits first
        {
            let mut global = self.global_limiter.write().await;
            if !global.request_limiter.consume(1) {
                self.update_stats(|stats| stats.global_requests_blocked += 1).await;
                return Err(SemanticError::RateLimitError(
                    "Global request rate limit exceeded".to_string()
                ));
            }
            global.total_requests += 1;
        }

        // Check agent-specific limits
        {
            let mut limiters = self.agent_limiters.write().await;
            let agent_limiter = limiters.entry(agent_id.to_string())
                .or_insert_with(|| self.create_agent_limiter(agent_id));

            agent_limiter.last_activity = Instant::now();

            if !agent_limiter.request_limiter.consume(1) {
                self.update_stats(|stats| stats.total_requests_blocked += 1).await;
                return Err(SemanticError::RateLimitError(
                    format!("Request rate limit exceeded for agent: {}", agent_id)
                ));
            }

            agent_limiter.total_requests += 1;
        }

        debug!("Request allowed for agent: {}", agent_id);
        Ok(())
    }

    /// Check if event emission is allowed for an agent
    #[instrument(skip(self))]
    pub async fn check_event_allowed(&self, agent_id: &str, event_count: u32) -> SemanticResult<()> {
        // Check global limits first
        {
            let mut global = self.global_limiter.write().await;
            if !global.event_limiter.consume(event_count) {
                self.update_stats(|stats| stats.global_events_blocked += event_count as u64).await;
                return Err(SemanticError::RateLimitError(
                    "Global event rate limit exceeded".to_string()
                ));
            }
            global.total_events += event_count as u64;
        }

        // Check agent-specific limits
        {
            let mut limiters = self.agent_limiters.write().await;
            let agent_limiter = limiters.entry(agent_id.to_string())
                .or_insert_with(|| self.create_agent_limiter(agent_id));

            agent_limiter.last_activity = Instant::now();

            if !agent_limiter.event_limiter.consume(event_count) {
                self.update_stats(|stats| stats.total_events_blocked += event_count as u64).await;
                return Err(SemanticError::RateLimitError(
                    format!("Event rate limit exceeded for agent: {}", agent_id)
                ));
            }

            agent_limiter.total_events += event_count as u64;
        }

        debug!("Events allowed for agent: {} (count: {})", agent_id, event_count);
        Ok(())
    }

    /// Check if bandwidth usage is allowed for an agent
    #[instrument(skip(self))]
    pub async fn check_bandwidth_allowed(&self, agent_id: &str, bytes: u64) -> SemanticResult<()> {
        let mut limiters = self.agent_limiters.write().await;
        let agent_limiter = limiters.entry(agent_id.to_string())
            .or_insert_with(|| self.create_agent_limiter(agent_id));

        agent_limiter.last_activity = Instant::now();

        if !agent_limiter.bandwidth_limiter.consume(bytes as u32) {
            self.update_stats(|stats| stats.total_bandwidth_blocked += bytes).await;
            return Err(SemanticError::RateLimitError(
                format!("Bandwidth limit exceeded for agent: {}", agent_id)
            ));
        }

        agent_limiter.total_bytes += bytes;
        debug!("Bandwidth allowed for agent: {} (bytes: {})", agent_id, bytes);
        Ok(())
    }

    /// Reserve a stream slot for an agent
    #[instrument(skip(self))]
    pub async fn reserve_stream(&self, agent_id: &str) -> SemanticResult<()> {
        let mut limiters = self.agent_limiters.write().await;
        let agent_limiter = limiters.entry(agent_id.to_string())
            .or_insert_with(|| self.create_agent_limiter(agent_id));

        if agent_limiter.concurrent_streams >= self.config.max_concurrent_streams_per_agent {
            return Err(SemanticError::RateLimitError(
                format!("Concurrent stream limit exceeded for agent: {}", agent_id)
            ));
        }

        agent_limiter.concurrent_streams += 1;
        agent_limiter.last_activity = Instant::now();
        
        info!("Stream reserved for agent: {} (total: {})", agent_id, agent_limiter.concurrent_streams);
        Ok(())
    }

    /// Release a stream slot for an agent
    #[instrument(skip(self))]
    pub async fn release_stream(&self, agent_id: &str) -> SemanticResult<()> {
        let mut limiters = self.agent_limiters.write().await;
        if let Some(agent_limiter) = limiters.get_mut(agent_id) {
            if agent_limiter.concurrent_streams > 0 {
                agent_limiter.concurrent_streams -= 1;
                info!("Stream released for agent: {} (remaining: {})", agent_id, agent_limiter.concurrent_streams);
            }
        }
        Ok(())
    }

    /// Check query complexity limit
    #[instrument(skip(self))]
    pub async fn check_query_complexity(&self, agent_id: &str, complexity: u32) -> SemanticResult<()> {
        let mut limiters = self.agent_limiters.write().await;
        let agent_limiter = limiters.entry(agent_id.to_string())
            .or_insert_with(|| self.create_agent_limiter(agent_id));

        if complexity > self.config.max_query_complexity_per_agent {
            return Err(SemanticError::RateLimitError(
                format!("Query complexity limit exceeded for agent: {} (complexity: {})", agent_id, complexity)
            ));
        }

        agent_limiter.query_complexity_used += complexity;
        agent_limiter.last_activity = Instant::now();
        
        debug!("Query complexity allowed for agent: {} (complexity: {})", agent_id, complexity);
        Ok(())
    }

    /// Get rate limiting statistics
    pub async fn get_stats(&self) -> RateLimitStats {
        let stats = self.stats.read().await;
        let limiters = self.agent_limiters.read().await;
        
        let mut updated_stats = stats.clone();
        updated_stats.active_agents = limiters.len() as u32;
        
        // Calculate average rates
        let total_requests: u64 = limiters.values().map(|l| l.total_requests).sum();
        let total_events: u64 = limiters.values().map(|l| l.total_events).sum();
        
        // Simple moving average (would be better with time windows)
        updated_stats.average_request_rate = total_requests as f64 / limiters.len().max(1) as f64;
        updated_stats.average_event_rate = total_events as f64 / limiters.len().max(1) as f64;
        
        updated_stats
    }

    /// Clean up inactive agents
    #[instrument(skip(self))]
    pub async fn cleanup_inactive_agents(&self) -> SemanticResult<usize> {
        let mut limiters = self.agent_limiters.write().await;
        let initial_count = limiters.len();
        
        let timeout = Duration::from_secs(self.config.agent_timeout_minutes as u64 * 60);
        let now = Instant::now();
        
        limiters.retain(|agent_id, limiter| {
            let is_active = now.duration_since(limiter.last_activity) < timeout;
            if !is_active {
                debug!("Removing inactive agent: {}", agent_id);
            }
            is_active
        });
        
        let cleaned_count = initial_count - limiters.len();
        
        if cleaned_count > 0 {
            self.update_stats(|stats| {
                stats.active_agents = limiters.len() as u32;
                stats.last_cleanup = Some(Utc::now());
            }).await;
            
            info!("Cleaned up {} inactive agent rate limiters", cleaned_count);
        }
        
        Ok(cleaned_count)
    }

    /// Reset rate limits for a specific agent
    pub async fn reset_agent_limits(&self, agent_id: &str) -> SemanticResult<()> {
        let mut limiters = self.agent_limiters.write().await;
        
        if limiters.remove(agent_id).is_some() {
            self.update_stats(|stats| stats.active_agents = limiters.len() as u32).await;
            info!("Reset rate limits for agent: {}", agent_id);
            Ok(())
        } else {
            Err(SemanticError::InvalidRequest(
                format!("Agent {} not found", agent_id)
            ))
        }
    }

    /// Get rate limit status for an agent
    pub async fn get_agent_status(&self, agent_id: &str) -> Option<AgentRateLimitStatus> {
        let limiters = self.agent_limiters.read().await;
        limiters.get(agent_id).map(|limiter| AgentRateLimitStatus {
            agent_id: agent_id.to_string(),
            requests_remaining: limiter.request_limiter.tokens as u32,
            events_remaining: limiter.event_limiter.tokens as u32,
            bandwidth_remaining: limiter.bandwidth_limiter.tokens as u64,
            concurrent_streams: limiter.concurrent_streams,
            total_requests: limiter.total_requests,
            total_events: limiter.total_events,
            total_bytes: limiter.total_bytes,
            last_activity: limiter.last_activity,
        })
    }

    /// Create a new agent rate limiter
    fn create_agent_limiter(&self, agent_id: &str) -> AgentRateLimiter {
        debug!("Creating rate limiter for agent: {}", agent_id);
        
        AgentRateLimiter {
            agent_id: agent_id.to_string(),
            request_limiter: TokenBucket::new(
                self.config.burst_size_per_agent,
                self.config.requests_per_minute_per_agent as f64 / 60.0,
            ),
            event_limiter: TokenBucket::new(
                self.config.events_per_minute_per_agent,
                self.config.events_per_minute_per_agent as f64 / 60.0,
            ),
            bandwidth_limiter: TokenBucket::new(
                self.config.bandwidth_limit_per_agent as u32,
                self.config.bandwidth_limit_per_agent as f64 / 60.0,
            ),
            concurrent_streams: 0,
            query_complexity_used: 0,
            last_activity: Instant::now(),
            total_requests: 0,
            total_events: 0,
            total_bytes: 0,
        }
    }

    /// Update statistics with a closure
    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut RateLimitStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut *stats);
    }
}

/// Agent rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRateLimitStatus {
    pub agent_id: String,
    pub requests_remaining: u32,
    pub events_remaining: u32,
    pub bandwidth_remaining: u64,
    pub concurrent_streams: u32,
    pub total_requests: u64,
    pub total_events: u64,
    pub total_bytes: u64,
    pub last_activity: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume tokens from the bucket
    fn consume(&mut self, tokens: u32) -> bool {
        self.refill();
        
        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        let tokens_to_add = elapsed * self.refill_rate;
        self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
        self.last_refill = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_request_rate_limiting() {
        let config = RateLimitConfig {
            requests_per_minute_per_agent: 2,
            burst_size_per_agent: 2,
            ..Default::default()
        };
        let rate_limiter = ApiRateLimiter::new(config);
        
        // First two requests should succeed
        assert!(rate_limiter.check_request_allowed("test_agent").await.is_ok());
        assert!(rate_limiter.check_request_allowed("test_agent").await.is_ok());
        
        // Third request should fail
        assert!(rate_limiter.check_request_allowed("test_agent").await.is_err());
    }

    #[tokio::test]
    async fn test_stream_reservation() {
        let config = RateLimitConfig {
            max_concurrent_streams_per_agent: 2,
            ..Default::default()
        };
        let rate_limiter = ApiRateLimiter::new(config);
        
        // Reserve two streams
        assert!(rate_limiter.reserve_stream("test_agent").await.is_ok());
        assert!(rate_limiter.reserve_stream("test_agent").await.is_ok());
        
        // Third stream should fail
        assert!(rate_limiter.reserve_stream("test_agent").await.is_err());
        
        // Release one stream
        assert!(rate_limiter.release_stream("test_agent").await.is_ok());
        
        // Now we should be able to reserve again
        assert!(rate_limiter.reserve_stream("test_agent").await.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_agents() {
        let config = RateLimitConfig {
            requests_per_minute_per_agent: 1,
            burst_size_per_agent: 1,
            ..Default::default()
        };
        let rate_limiter = ApiRateLimiter::new(config);
        
        // Each agent should have their own rate limit
        assert!(rate_limiter.check_request_allowed("agent1").await.is_ok());
        assert!(rate_limiter.check_request_allowed("agent2").await.is_ok());
        
        // Second request for each agent should fail
        assert!(rate_limiter.check_request_allowed("agent1").await.is_err());
        assert!(rate_limiter.check_request_allowed("agent2").await.is_err());
        
        let stats = rate_limiter.get_stats().await;
        assert_eq!(stats.active_agents, 2);
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 1.0); // 10 capacity, 1 token per second
        
        // Should be able to consume initial tokens
        assert!(bucket.consume(5));
        assert!(bucket.consume(5));
        
        // Should not be able to consume more
        assert!(!bucket.consume(1));
    }
}