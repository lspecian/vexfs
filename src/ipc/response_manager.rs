//! Response Manager for VexFS IPC
//!
//! This module implements response correlation, delivery, and management
//! for embedding service responses.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult, IpcConfig};
use crate::ipc::protocol::*;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};

/// Response manager for handling embedding service responses
pub struct ResponseManager {
    /// Configuration
    config: IpcConfig,
    /// Pending responses
    pending_responses: BTreeMap<u64, PendingResponse>,
    /// Response statistics
    stats: ResponseStats,
    /// Response timeouts
    response_timeouts: BTreeMap<u64, u64>,
    /// Response cache for duplicate requests
    response_cache: BTreeMap<String, CachedResponse>,
}

/// Pending response information
#[derive(Debug, Clone)]
pub struct PendingResponse {
    /// Request ID
    pub request_id: u64,
    /// Service ID that should respond
    pub service_id: String,
    /// Request timestamp
    pub request_time: u64,
    /// Response timeout
    pub timeout: u64,
    /// Response callback information
    pub callback_info: ResponseCallback,
    /// Retry count
    pub retry_count: u32,
}

/// Response callback information
#[derive(Debug, Clone)]
pub struct ResponseCallback {
    /// Callback type
    pub callback_type: CallbackType,
    /// User context for the callback
    pub user_context: u64,
    /// Process ID for userspace callbacks
    pub process_id: u32,
    /// Thread ID for userspace callbacks
    pub thread_id: u32,
}

/// Callback type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum CallbackType {
    /// Synchronous blocking call
    Synchronous,
    /// Asynchronous callback
    Asynchronous,
    /// Signal-based notification
    Signal,
    /// Polling-based (no callback)
    Polling,
}

/// Cached response for duplicate request detection
#[derive(Debug, Clone)]
pub struct CachedResponse {
    /// Response data
    pub response: EmbeddingResponse,
    /// Cache timestamp
    pub cached_at: u64,
    /// Cache expiry time
    pub expires_at: u64,
    /// Hit count
    pub hit_count: u64,
}

/// Response statistics
#[derive(Debug, Clone, Default)]
pub struct ResponseStats {
    pub total_responses: u64,
    pub successful_responses: u64,
    pub failed_responses: u64,
    pub timeout_responses: u64,
    pub cached_responses: u64,
    pub average_response_time_us: u64,
    pub min_response_time_us: u64,
    pub max_response_time_us: u64,
    pub pending_responses: usize,
}

impl ResponseManager {
    /// Create a new response manager
    pub fn new(config: IpcConfig) -> Self {
        Self {
            config,
            pending_responses: BTreeMap::new(),
            stats: ResponseStats::default(),
            response_timeouts: BTreeMap::new(),
            response_cache: BTreeMap::new(),
        }
    }

    /// Register a pending response
    pub fn register_pending_response(
        &mut self,
        request_id: u64,
        service_id: String,
        callback_info: ResponseCallback,
    ) -> IpcResult<()> {
        let current_time = self.get_current_timestamp();
        let timeout = current_time + self.config.request_timeout_ms / 1000;

        let pending_response = PendingResponse {
            request_id,
            service_id,
            request_time: current_time,
            timeout,
            callback_info,
            retry_count: 0,
        };

        self.pending_responses.insert(request_id, pending_response);
        self.response_timeouts.insert(request_id, timeout);
        self.stats.pending_responses = self.pending_responses.len();

        Ok(())
    }

    /// Handle incoming response
    pub fn handle_response(&mut self, response: EmbeddingResponse) -> IpcResult<ResponseDelivery> {
        let request_id = response.request_id;
        
        // Check if we have a pending response for this request
        let pending_response = self.pending_responses.remove(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))?;

        self.response_timeouts.remove(&request_id);

        // Calculate response time
        let current_time = self.get_current_timestamp();
        let response_time_us = (current_time - pending_response.request_time) * 1_000_000;

        // Update statistics
        self.update_response_stats(&response, response_time_us);

        // Cache response if successful and cacheable
        if response.status == ResponseStatus::Success {
            self.cache_response(&response)?;
        }

        // Create response delivery
        let delivery = ResponseDelivery {
            request_id,
            response,
            callback_info: pending_response.callback_info,
            response_time_us,
            service_id: pending_response.service_id,
        };

        Ok(delivery)
    }

    /// Handle batch response
    pub fn handle_batch_response(&mut self, batch_response: Vec<EmbeddingResponse>) -> IpcResult<Vec<ResponseDelivery>> {
        let mut deliveries = Vec::with_capacity(batch_response.len());

        for response in batch_response {
            let delivery = self.handle_response(response)?;
            deliveries.push(delivery);
        }

        Ok(deliveries)
    }

    /// Check for cached response
    pub fn check_cache(&mut self, request_hash: &str) -> Option<EmbeddingResponse> {
        let current_time = self.get_current_timestamp();
        
        // First check if entry exists and is valid
        let should_remove = if let Some(cached) = self.response_cache.get(request_hash) {
            current_time > cached.expires_at
        } else {
            false
        };
        
        if should_remove {
            // Remove expired entry
            self.response_cache.remove(request_hash);
            return None;
        }
        
        // Now safely get mutable reference and update
        if let Some(cached) = self.response_cache.get_mut(request_hash) {
            cached.hit_count += 1;
            self.stats.cached_responses += 1;
            return Some(cached.response.clone());
        }

        None
    }

    /// Check for timed out responses
    pub fn check_timeouts(&mut self) -> Vec<u64> {
        let current_time = self.get_current_timestamp();
        
        let timed_out_requests: Vec<u64> = self.response_timeouts.iter()
            .filter(|(_, &timeout)| current_time > timeout)
            .map(|(&request_id, _)| request_id)
            .collect();

        // Remove timed out responses
        for &request_id in &timed_out_requests {
            self.pending_responses.remove(&request_id);
            self.response_timeouts.remove(&request_id);
            self.stats.timeout_responses += 1;
        }

        self.stats.pending_responses = self.pending_responses.len();

        timed_out_requests
    }

    /// Get pending response
    pub fn get_pending_response(&self, request_id: u64) -> Option<&PendingResponse> {
        self.pending_responses.get(&request_id)
    }

    /// Get all pending responses
    pub fn get_pending_responses(&self) -> Vec<&PendingResponse> {
        self.pending_responses.values().collect()
    }

    /// Get pending responses for a service
    pub fn get_pending_responses_for_service(&self, service_id: &str) -> Vec<&PendingResponse> {
        self.pending_responses.values()
            .filter(|response| response.service_id == service_id)
            .collect()
    }

    /// Cancel pending response
    pub fn cancel_pending_response(&mut self, request_id: u64) -> IpcResult<()> {
        self.pending_responses.remove(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))?;
        
        self.response_timeouts.remove(&request_id);
        self.stats.pending_responses = self.pending_responses.len();
        
        Ok(())
    }

    /// Get response statistics
    pub fn get_stats(&self) -> &ResponseStats {
        &self.stats
    }

    /// Get average response time
    pub fn get_average_response_time(&self) -> u64 {
        self.stats.average_response_time_us
    }

    /// Perform maintenance operations
    pub fn perform_maintenance(&mut self) -> IpcResult<ResponseMaintenanceResult> {
        let mut result = ResponseMaintenanceResult::default();

        // Check for timeouts
        let timed_out = self.check_timeouts();
        result.timed_out_responses = timed_out.len();

        // Clean up expired cache entries
        let current_time = self.get_current_timestamp();
        let expired_cache_keys: Vec<String> = self.response_cache.iter()
            .filter(|(_, cached)| current_time > cached.expires_at)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &expired_cache_keys {
            self.response_cache.remove(key);
        }
        result.expired_cache_entries = expired_cache_keys.len();

        Ok(result)
    }

    // Private helper methods

    fn update_response_stats(&mut self, response: &EmbeddingResponse, response_time_us: u64) {
        self.stats.total_responses += 1;
        self.stats.pending_responses = self.pending_responses.len();

        match response.status {
            ResponseStatus::Success => {
                self.stats.successful_responses += 1;
            }
            _ => {
                self.stats.failed_responses += 1;
            }
        }

        // Update response time statistics
        if self.stats.average_response_time_us == 0 {
            self.stats.average_response_time_us = response_time_us;
            self.stats.min_response_time_us = response_time_us;
            self.stats.max_response_time_us = response_time_us;
        } else {
            // Exponential moving average
            self.stats.average_response_time_us = 
                (self.stats.average_response_time_us * 9 + response_time_us) / 10;
            
            if response_time_us < self.stats.min_response_time_us {
                self.stats.min_response_time_us = response_time_us;
            }
            
            if response_time_us > self.stats.max_response_time_us {
                self.stats.max_response_time_us = response_time_us;
            }
        }
    }

    fn cache_response(&mut self, response: &EmbeddingResponse) -> IpcResult<()> {
        // Generate cache key based on request characteristics
        let cache_key = self.generate_cache_key(response);
        
        let current_time = self.get_current_timestamp();
        let cache_ttl = 300; // 5 minutes cache TTL
        
        let cached_response = CachedResponse {
            response: response.clone(),
            cached_at: current_time,
            expires_at: current_time + cache_ttl,
            hit_count: 0,
        };

        // Limit cache size
        if self.response_cache.len() >= 1000 {
            // Remove oldest entry
            if let Some((oldest_key, _)) = self.response_cache.iter()
                .min_by_key(|(_, cached)| cached.cached_at)
                .map(|(k, v)| (k.clone(), v.clone()))
            {
                self.response_cache.remove(&oldest_key);
            }
        }

        self.response_cache.insert(cache_key, cached_response);
        Ok(())
    }

    fn generate_cache_key(&self, response: &EmbeddingResponse) -> String {
        // Generate cache key based on response characteristics
        // In a real implementation, would hash request parameters
        format!("response_{}", response.request_id)
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in seconds
        0 // Placeholder
    }
}

/// Response delivery information
#[derive(Debug, Clone)]
pub struct ResponseDelivery {
    /// Request ID
    pub request_id: u64,
    /// Response data
    pub response: EmbeddingResponse,
    /// Callback information
    pub callback_info: ResponseCallback,
    /// Response time in microseconds
    pub response_time_us: u64,
    /// Service ID that provided the response
    pub service_id: String,
}

/// Response maintenance result
#[derive(Debug, Clone, Default)]
pub struct ResponseMaintenanceResult {
    pub timed_out_responses: usize,
    pub expired_cache_entries: usize,
}

/// Response correlator for matching responses to requests
pub struct ResponseCorrelator {
    /// Correlation map
    correlations: BTreeMap<u64, CorrelationInfo>,
    /// Correlation statistics
    stats: CorrelationStats,
}

/// Correlation information
#[derive(Debug, Clone)]
pub struct CorrelationInfo {
    /// Request ID
    pub request_id: u64,
    /// Service ID
    pub service_id: String,
    /// Request timestamp
    pub request_time: u64,
    /// Expected response time
    pub expected_response_time: u64,
    /// Correlation metadata
    pub metadata: BTreeMap<String, String>,
}

/// Correlation statistics
#[derive(Debug, Clone, Default)]
pub struct CorrelationStats {
    pub total_correlations: u64,
    pub successful_correlations: u64,
    pub failed_correlations: u64,
    pub orphaned_responses: u64,
    pub duplicate_responses: u64,
}

impl ResponseCorrelator {
    /// Create a new response correlator
    pub fn new() -> Self {
        Self {
            correlations: BTreeMap::new(),
            stats: CorrelationStats::default(),
        }
    }

    /// Add correlation
    pub fn add_correlation(&mut self, correlation_info: CorrelationInfo) -> IpcResult<()> {
        let request_id = correlation_info.request_id;
        
        if self.correlations.contains_key(&request_id) {
            return Err(IpcError::CorrelationFailed(request_id));
        }

        self.correlations.insert(request_id, correlation_info);
        self.stats.total_correlations += 1;
        
        Ok(())
    }

    /// Remove correlation
    pub fn remove_correlation(&mut self, request_id: u64) -> IpcResult<CorrelationInfo> {
        let correlation_info = self.correlations.remove(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))?;

        self.stats.successful_correlations += 1;
        Ok(correlation_info)
    }

    /// Check correlation
    pub fn check_correlation(&self, request_id: u64) -> Option<&CorrelationInfo> {
        self.correlations.get(&request_id)
    }

    /// Handle orphaned response
    pub fn handle_orphaned_response(&mut self, response: &EmbeddingResponse) -> IpcResult<()> {
        self.stats.orphaned_responses += 1;
        
        // Log orphaned response for debugging
        // In a real implementation, might try to recover or notify administrators
        
        Ok(())
    }

    /// Get correlation statistics
    pub fn get_stats(&self) -> &CorrelationStats {
        &self.stats
    }

    /// Clean up expired correlations
    pub fn cleanup_expired(&mut self, current_time: u64) -> usize {
        let expired_correlations: Vec<u64> = self.correlations.iter()
            .filter(|(_, info)| current_time > info.expected_response_time)
            .map(|(&request_id, _)| request_id)
            .collect();

        for request_id in &expired_correlations {
            self.correlations.remove(request_id);
            self.stats.failed_correlations += 1;
        }

        expired_correlations.len()
    }
}

/// Response validator for validating incoming responses
pub struct ResponseValidator;

impl ResponseValidator {
    /// Validate embedding response
    pub fn validate_response(response: &EmbeddingResponse) -> IpcResult<()> {
        // Validate request ID
        if response.request_id == 0 {
            return Err(IpcError::InvalidMessage("Invalid request ID".to_string()));
        }

        // Validate status consistency
        match response.status {
            ResponseStatus::Success => {
                if response.embedding.is_none() {
                    return Err(IpcError::InvalidMessage("Missing embedding in successful response".to_string()));
                }
                if response.error.is_some() {
                    return Err(IpcError::InvalidMessage("Error message in successful response".to_string()));
                }
            }
            ResponseStatus::Error | ResponseStatus::InternalError => {
                if response.error.is_none() {
                    return Err(IpcError::InvalidMessage("Missing error message in error response".to_string()));
                }
            }
            ResponseStatus::Timeout => {
                // Timeout responses may or may not have error messages
            }
            ResponseStatus::Overloaded => {
                // Overloaded responses should have error messages
                if response.error.is_none() {
                    return Err(IpcError::InvalidMessage("Missing error message in overloaded response".to_string()));
                }
            }
            ResponseStatus::InvalidRequest => {
                // Invalid request responses should have error messages
                if response.error.is_none() {
                    return Err(IpcError::InvalidMessage("Missing error message in invalid request response".to_string()));
                }
            }
            ResponseStatus::ModelNotFound => {
                // Model not found responses should have error messages
                if response.error.is_none() {
                    return Err(IpcError::InvalidMessage("Missing error message in model not found response".to_string()));
                }
            }
        }

        // Validate embedding dimensions if present
        if let Some(ref embedding) = response.embedding {
            if embedding.is_empty() {
                return Err(IpcError::InvalidMessage("Empty embedding vector".to_string()));
            }
            
            // Check for invalid values
            for &value in embedding {
                if value.is_nan() || value.is_infinite() {
                    return Err(IpcError::InvalidMessage("Invalid embedding value".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Validate batch response
    pub fn validate_batch_response(responses: &[EmbeddingResponse]) -> IpcResult<()> {
        if responses.is_empty() {
            return Err(IpcError::InvalidMessage("Empty batch response".to_string()));
        }

        // Validate each response in the batch
        for response in responses {
            Self::validate_response(response)?;
        }

        // Check for duplicate request IDs
        let mut request_ids = Vec::new();
        for response in responses {
            if request_ids.contains(&response.request_id) {
                return Err(IpcError::InvalidMessage("Duplicate request ID in batch".to_string()));
            }
            request_ids.push(response.request_id);
        }

        Ok(())
    }
}

/// Response aggregator for combining multiple responses
pub struct ResponseAggregator {
    /// Aggregation rules
    rules: Vec<AggregationRule>,
}

/// Aggregation rule
#[derive(Debug, Clone)]
pub struct AggregationRule {
    /// Rule name
    pub name: String,
    /// Aggregation type
    pub aggregation_type: AggregationType,
    /// Weight for weighted aggregation
    pub weight: f32,
    /// Condition for applying the rule
    pub condition: AggregationCondition,
}

/// Aggregation type
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationType {
    /// Average of all responses
    Average,
    /// Weighted average
    WeightedAverage,
    /// Take the first successful response
    FirstSuccess,
    /// Take the response with highest confidence
    HighestConfidence,
    /// Majority vote
    MajorityVote,
}

/// Aggregation condition
#[derive(Debug, Clone)]
pub struct AggregationCondition {
    /// Minimum number of responses required
    pub min_responses: usize,
    /// Maximum response time allowed
    pub max_response_time_ms: u64,
    /// Required service types
    pub required_service_types: Vec<String>,
}

impl ResponseAggregator {
    /// Create a new response aggregator
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// Add aggregation rule
    pub fn add_rule(&mut self, rule: AggregationRule) {
        self.rules.push(rule);
    }

    /// Aggregate responses
    pub fn aggregate_responses(&self, responses: &[EmbeddingResponse]) -> IpcResult<EmbeddingResponse> {
        if responses.is_empty() {
            return Err(IpcError::InvalidMessage("No responses to aggregate".to_string()));
        }

        // Find applicable rule
        for rule in &self.rules {
            if self.rule_applies(rule, responses) {
                return self.apply_aggregation_rule(rule, responses);
            }
        }

        // Default: return first successful response
        for response in responses {
            if response.status == ResponseStatus::Success {
                return Ok(response.clone());
            }
        }

        // If no successful responses, return the first one
        Ok(responses[0].clone())
    }

    fn rule_applies(&self, rule: &AggregationRule, responses: &[EmbeddingResponse]) -> bool {
        // Check minimum responses
        if responses.len() < rule.condition.min_responses {
            return false;
        }

        // Check response times
        for response in responses {
            if response.processing_time_us / 1000 > rule.condition.max_response_time_ms {
                return false;
            }
        }

        true
    }

    fn apply_aggregation_rule(&self, rule: &AggregationRule, responses: &[EmbeddingResponse]) -> IpcResult<EmbeddingResponse> {
        match rule.aggregation_type {
            AggregationType::Average => self.average_responses(responses),
            AggregationType::WeightedAverage => self.weighted_average_responses(responses, rule.weight),
            AggregationType::FirstSuccess => self.first_success_response(responses),
            AggregationType::HighestConfidence => self.highest_confidence_response(responses),
            AggregationType::MajorityVote => self.majority_vote_response(responses),
        }
    }

    fn average_responses(&self, responses: &[EmbeddingResponse]) -> IpcResult<EmbeddingResponse> {
        let successful_responses: Vec<&EmbeddingResponse> = responses.iter()
            .filter(|r| r.status == ResponseStatus::Success && r.embedding.is_some())
            .collect();

        if successful_responses.is_empty() {
            return Ok(responses[0].clone());
        }

        // Average the embeddings
        let first_embedding = successful_responses[0].embedding.as_ref().unwrap();
        let mut averaged_embedding = vec![0.0; first_embedding.len()];

        for response in &successful_responses {
            if let Some(ref embedding) = response.embedding {
                for (i, &value) in embedding.iter().enumerate() {
                    averaged_embedding[i] += value;
                }
            }
        }

        let count = successful_responses.len() as f32;
        for value in &mut averaged_embedding {
            *value /= count;
        }

        // Create aggregated response
        let mut aggregated = successful_responses[0].clone();
        aggregated.embedding = Some(averaged_embedding);
        aggregated.processing_time_us = successful_responses.iter()
            .map(|r| r.processing_time_us)
            .sum::<u64>() / successful_responses.len() as u64;

        Ok(aggregated)
    }

    fn weighted_average_responses(&self, responses: &[EmbeddingResponse], _weight: f32) -> IpcResult<EmbeddingResponse> {
        // Simplified: just use regular average for now
        self.average_responses(responses)
    }

    fn first_success_response(&self, responses: &[EmbeddingResponse]) -> IpcResult<EmbeddingResponse> {
        for response in responses {
            if response.status == ResponseStatus::Success {
                return Ok(response.clone());
            }
        }
        Ok(responses[0].clone())
    }

    fn highest_confidence_response(&self, responses: &[EmbeddingResponse]) -> IpcResult<EmbeddingResponse> {
        // For now, just return first successful response
        // In a real implementation, would look at confidence scores in metadata
        self.first_success_response(responses)
    }

    fn majority_vote_response(&self, responses: &[EmbeddingResponse]) -> IpcResult<EmbeddingResponse> {
        // For now, just return first successful response
        // In a real implementation, would implement proper majority voting
        self.first_success_response(responses)
    }
}