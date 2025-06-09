//! Query Processing and Indexing for VexFS Semantic Operation Journal
//! 
//! This module implements efficient query processing, indexing, and retrieval
//! mechanisms for semantic events with support for complex filtering and aggregation.

use std::collections::{HashMap, BTreeMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as TokioRwLock;
use tracing::{info, warn, error, debug, instrument};

use crate::semantic_api::{
    types::*,
    SemanticResult, SemanticError,
};

/// Index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub enable_timestamp_index: bool,
    pub enable_event_type_index: bool,
    pub enable_category_index: bool,
    pub enable_agent_index: bool,
    pub enable_transaction_index: bool,
    pub enable_path_index: bool,
    pub enable_full_text_search: bool,
    pub max_index_memory_mb: usize,
    pub index_update_batch_size: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            enable_timestamp_index: true,
            enable_event_type_index: true,
            enable_category_index: true,
            enable_agent_index: true,
            enable_transaction_index: true,
            enable_path_index: true,
            enable_full_text_search: false, // Disabled by default for performance
            max_index_memory_mb: 512,
            index_update_batch_size: 1000,
        }
    }
}

/// Time-based index for efficient range queries
#[derive(Debug)]
pub struct TimestampIndex {
    /// BTreeMap for efficient range queries: timestamp -> event_ids
    index: BTreeMap<chrono::DateTime<chrono::Utc>, Vec<u64>>,
}

impl TimestampIndex {
    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }
    
    pub fn insert(&mut self, timestamp: chrono::DateTime<chrono::Utc>, event_id: u64) {
        self.index.entry(timestamp).or_insert_with(Vec::new).push(event_id);
    }
    
    pub fn query_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<u64> {
        self.index
            .range(start..=end)
            .flat_map(|(_, event_ids)| event_ids.iter().copied())
            .collect()
    }
    
    pub fn get_oldest_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.index.keys().next().copied()
    }
    
    pub fn get_newest_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.index.keys().next_back().copied()
    }
    
    pub fn len(&self) -> usize {
        self.index.values().map(|v| v.len()).sum()
    }
}

/// Hash-based index for exact match queries
#[derive(Debug)]
pub struct HashIndex<T: std::hash::Hash + Eq + Clone> {
    /// HashMap for exact matches: key -> event_ids
    index: HashMap<T, Vec<u64>>,
}

impl<T: std::hash::Hash + Eq + Clone> HashIndex<T> {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }
    
    pub fn insert(&mut self, key: T, event_id: u64) {
        self.index.entry(key).or_insert_with(Vec::new).push(event_id);
    }
    
    pub fn query(&self, key: &T) -> Vec<u64> {
        self.index.get(key).cloned().unwrap_or_default()
    }
    
    pub fn query_multiple(&self, keys: &[T]) -> Vec<u64> {
        keys.iter()
            .flat_map(|key| self.query(key))
            .collect()
    }
    
    pub fn get_all_keys(&self) -> Vec<T> {
        self.index.keys().cloned().collect()
    }
    
    pub fn len(&self) -> usize {
        self.index.values().map(|v| v.len()).sum()
    }
    
    pub fn get_stats(&self) -> HashMap<T, u64> {
        self.index.iter()
            .map(|(k, v)| (k.clone(), v.len() as u64))
            .collect()
    }
}

/// Comprehensive event index system
pub struct EventIndexSystem {
    config: IndexConfig,
    
    // Core indexes
    timestamp_index: Arc<RwLock<TimestampIndex>>,
    event_type_index: Arc<RwLock<HashIndex<SemanticEventType>>>,
    category_index: Arc<RwLock<HashIndex<EventCategory>>>,
    agent_index: Arc<RwLock<HashIndex<String>>>,
    transaction_index: Arc<RwLock<HashIndex<u64>>>,
    path_index: Arc<RwLock<HashIndex<String>>>,
    priority_index: Arc<RwLock<HashIndex<EventPriority>>>,
    
    // Statistics
    index_stats: Arc<RwLock<IndexStats>>,
}

/// Index statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct IndexStats {
    pub total_events_indexed: u64,
    pub timestamp_index_size: usize,
    pub event_type_index_size: usize,
    pub category_index_size: usize,
    pub agent_index_size: usize,
    pub transaction_index_size: usize,
    pub path_index_size: usize,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
    pub index_memory_usage_mb: f64,
}

impl EventIndexSystem {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            config,
            timestamp_index: Arc::new(RwLock::new(TimestampIndex::new())),
            event_type_index: Arc::new(RwLock::new(HashIndex::new())),
            category_index: Arc::new(RwLock::new(HashIndex::new())),
            agent_index: Arc::new(RwLock::new(HashIndex::new())),
            transaction_index: Arc::new(RwLock::new(HashIndex::new())),
            path_index: Arc::new(RwLock::new(HashIndex::new())),
            priority_index: Arc::new(RwLock::new(HashIndex::new())),
            index_stats: Arc::new(RwLock::new(IndexStats::default())),
        }
    }
    
    /// Index a semantic event
    #[instrument(skip(self, event))]
    pub fn index_event(&self, event: &SemanticEvent) -> SemanticResult<()> {
        let event_id = event.event_id;
        
        // Timestamp index
        if self.config.enable_timestamp_index {
            let mut timestamp_index = self.timestamp_index.write().unwrap();
            timestamp_index.insert(event.timestamp.timestamp, event_id);
        }
        
        // Event type index
        if self.config.enable_event_type_index {
            let mut event_type_index = self.event_type_index.write().unwrap();
            event_type_index.insert(event.event_type, event_id);
        }
        
        // Category index
        if self.config.enable_category_index {
            let mut category_index = self.category_index.write().unwrap();
            category_index.insert(event.event_type.category(), event_id);
        }
        
        // Priority index
        {
            let mut priority_index = self.priority_index.write().unwrap();
            priority_index.insert(event.priority, event_id);
        }
        
        // Agent index
        if self.config.enable_agent_index {
            if let Some(agent_context) = &event.context.agent {
                let mut agent_index = self.agent_index.write().unwrap();
                agent_index.insert(agent_context.agent_id.clone(), event_id);
            }
        }
        
        // Transaction index
        if self.config.enable_transaction_index {
            if let Some(transaction_id) = event.context.transaction_id {
                let mut transaction_index = self.transaction_index.write().unwrap();
                transaction_index.insert(transaction_id, event_id);
            }
        }
        
        // Path index
        if self.config.enable_path_index {
            if let Some(fs_context) = &event.context.filesystem {
                let mut path_index = self.path_index.write().unwrap();
                path_index.insert(fs_context.path.clone(), event_id);
            }
        }
        
        // Update statistics
        {
            let mut stats = self.index_stats.write().unwrap();
            stats.total_events_indexed += 1;
            stats.last_update = Some(chrono::Utc::now());
            self.update_index_stats(&mut stats);
        }
        
        Ok(())
    }
    
    /// Query events using the index system
    #[instrument(skip(self, query))]
    pub fn query_events(&self, query: &EventQuery) -> SemanticResult<Vec<u64>> {
        let start_time = Instant::now();
        
        let mut candidate_sets: Vec<Vec<u64>> = Vec::new();
        
        // Apply filters using indexes
        
        // Time range filter
        if let Some(time_range) = &query.filter.time_range {
            if self.config.enable_timestamp_index {
                let timestamp_index = self.timestamp_index.read().unwrap();
                let time_candidates = timestamp_index.query_range(time_range.start, time_range.end);
                candidate_sets.push(time_candidates);
            }
        }
        
        // Event type filter
        if let Some(event_types) = &query.filter.event_types {
            if self.config.enable_event_type_index {
                let event_type_index = self.event_type_index.read().unwrap();
                let type_candidates = event_type_index.query_multiple(event_types);
                candidate_sets.push(type_candidates);
            }
        }
        
        // Category filter
        if let Some(categories) = &query.filter.categories {
            if self.config.enable_category_index {
                let category_index = self.category_index.read().unwrap();
                let category_candidates = category_index.query_multiple(categories);
                candidate_sets.push(category_candidates);
            }
        }
        
        // Agent filter
        if let Some(agent_id) = &query.filter.agent_id {
            if self.config.enable_agent_index {
                let agent_index = self.agent_index.read().unwrap();
                let agent_candidates = agent_index.query(agent_id);
                candidate_sets.push(agent_candidates);
            }
        }
        
        // Transaction filter
        if let Some(transaction_id) = query.filter.transaction_id {
            if self.config.enable_transaction_index {
                let transaction_index = self.transaction_index.read().unwrap();
                let transaction_candidates = transaction_index.query(&transaction_id);
                candidate_sets.push(transaction_candidates);
            }
        }
        
        // Path filter (exact match for now)
        if let Some(path_pattern) = &query.filter.path_pattern {
            if self.config.enable_path_index {
                let path_index = self.path_index.read().unwrap();
                let path_candidates = path_index.query(path_pattern);
                candidate_sets.push(path_candidates);
            }
        }
        
        // Priority filter
        if let Some(min_priority) = query.filter.min_priority {
            let priority_index = self.priority_index.read().unwrap();
            let mut priority_candidates = Vec::new();
            
            // Include all priorities >= min_priority (lower numeric value = higher priority)
            for priority in [EventPriority::Critical, EventPriority::High, EventPriority::Normal, EventPriority::Low, EventPriority::Background] {
                if priority <= min_priority {
                    priority_candidates.extend(priority_index.query(&priority));
                }
            }
            
            candidate_sets.push(priority_candidates);
        }
        
        // Intersect all candidate sets
        let result = if candidate_sets.is_empty() {
            // No filters applied, return empty (should be handled by caller)
            Vec::new()
        } else if candidate_sets.len() == 1 {
            candidate_sets.into_iter().next().unwrap()
        } else {
            // Intersect all sets
            let mut result_set: HashSet<u64> = candidate_sets[0].iter().copied().collect();
            
            for candidate_set in candidate_sets.into_iter().skip(1) {
                let set: HashSet<u64> = candidate_set.into_iter().collect();
                result_set = result_set.intersection(&set).copied().collect();
            }
            
            result_set.into_iter().collect()
        };
        
        let query_time = start_time.elapsed();
        debug!("Index query completed in {:?}, found {} candidates", query_time, result.len());
        
        Ok(result)
    }
    
    /// Get index statistics
    pub fn get_stats(&self) -> IndexStats {
        let mut stats = self.index_stats.write().unwrap();
        self.update_index_stats(&mut stats);
        stats.clone()
    }
    
    /// Update index statistics
    fn update_index_stats(&self, stats: &mut IndexStats) {
        stats.timestamp_index_size = self.timestamp_index.read().unwrap().len();
        stats.event_type_index_size = self.event_type_index.read().unwrap().len();
        stats.category_index_size = self.category_index.read().unwrap().len();
        stats.agent_index_size = self.agent_index.read().unwrap().len();
        stats.transaction_index_size = self.transaction_index.read().unwrap().len();
        stats.path_index_size = self.path_index.read().unwrap().len();
        
        // Rough memory usage estimation (in MB)
        stats.index_memory_usage_mb = (
            stats.timestamp_index_size * 32 +  // Timestamp + Vec overhead
            stats.event_type_index_size * 16 + // Enum + Vec overhead
            stats.category_index_size * 16 +   // Enum + Vec overhead
            stats.agent_index_size * 64 +      // String + Vec overhead
            stats.transaction_index_size * 16 + // u64 + Vec overhead
            stats.path_index_size * 128        // Path string + Vec overhead
        ) as f64 / (1024.0 * 1024.0);
    }
    
    /// Get event type distribution
    pub fn get_event_type_distribution(&self) -> HashMap<SemanticEventType, u64> {
        self.event_type_index.read().unwrap().get_stats()
    }
    
    /// Get category distribution
    pub fn get_category_distribution(&self) -> HashMap<EventCategory, u64> {
        self.category_index.read().unwrap().get_stats()
    }
    
    /// Get priority distribution
    pub fn get_priority_distribution(&self) -> HashMap<EventPriority, u64> {
        self.priority_index.read().unwrap().get_stats()
    }
    
    /// Get time range of indexed events
    pub fn get_time_range(&self) -> Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
        let timestamp_index = self.timestamp_index.read().unwrap();
        if let (Some(oldest), Some(newest)) = (timestamp_index.get_oldest_timestamp(), timestamp_index.get_newest_timestamp()) {
            Some((oldest, newest))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_api::types::*;
    
    fn create_test_event(event_id: u64, event_type: SemanticEventType) -> SemanticEvent {
        SemanticEvent {
            event_id,
            event_type,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: chrono::Utc::now(),
                sequence: event_id,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: event_id,
            local_sequence: event_id,
            flags: EventFlags {
                atomic: false,
                transactional: false,
                causal: true,
                agent_visible: true,
                deterministic: true,
                compressed: false,
                indexed: true,
                replicated: false,
            },
            priority: EventPriority::Normal,
            event_size: 256,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFFFFFFFFFFFFFFFF,
            agent_relevance_score: 100,
            replay_priority: 3,
            context: SemanticContext {
                transaction_id: Some(event_id),
                session_id: None,
                causality_chain_id: None,
                filesystem: Some(FilesystemContext {
                    path: format!("/test/file_{}.txt", event_id),
                    inode_number: Some(event_id + 10000),
                    file_type: Some("regular".to_string()),
                }),
                graph: None,
                vector: None,
                agent: Some(AgentContext {
                    agent_id: format!("agent_{}", event_id % 3),
                    intent: Some("test_intent".to_string()),
                    confidence: Some(95),
                }),
                system: None,
                semantic: None,
                observability: None,
            },
            payload: Some(serde_json::json!({
                "test_data": format!("payload for event {}", event_id),
                "number": event_id
            })),
            metadata: None,
        }
    }
    
    #[test]
    fn test_timestamp_index() {
        let mut index = TimestampIndex::new();
        let now = chrono::Utc::now();
        
        index.insert(now, 1);
        index.insert(now + chrono::Duration::seconds(10), 2);
        index.insert(now + chrono::Duration::seconds(20), 3);
        
        let results = index.query_range(now, now + chrono::Duration::seconds(15));
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        
        assert_eq!(index.get_oldest_timestamp(), Some(now));
        assert_eq!(index.get_newest_timestamp(), Some(now + chrono::Duration::seconds(20)));
    }
    
    #[test]
    fn test_hash_index() {
        let mut index = HashIndex::new();
        
        index.insert("key1".to_string(), 1);
        index.insert("key1".to_string(), 2);
        index.insert("key2".to_string(), 3);
        
        let results = index.query(&"key1".to_string());
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        
        let results = index.query(&"key2".to_string());
        assert_eq!(results.len(), 1);
        assert!(results.contains(&3));
        
        let results = index.query(&"nonexistent".to_string());
        assert!(results.is_empty());
    }
    
    #[test]
    fn test_event_index_system() {
        let config = IndexConfig::default();
        let index_system = EventIndexSystem::new(config);
        
        // Create test events
        let event1 = create_test_event(1, SemanticEventType::FilesystemCreate);
        let event2 = create_test_event(2, SemanticEventType::FilesystemDelete);
        let event3 = create_test_event(3, SemanticEventType::GraphNodeCreate);
        
        // Index events
        index_system.index_event(&event1).unwrap();
        index_system.index_event(&event2).unwrap();
        index_system.index_event(&event3).unwrap();
        
        // Test event type query
        let query = EventQuery {
            filter: EventFilter {
                event_types: Some(vec![SemanticEventType::FilesystemCreate]),
                categories: None,
                time_range: None,
                agent_id: None,
                transaction_id: None,
                causality_chain_id: None,
                path_pattern: None,
                min_priority: None,
                required_flags: None,
                tags: None,
                min_relevance_score: None,
            },
            limit: None,
            offset: None,
            sort_by: None,
            include_payload: false,
            include_metadata: false,
            include_causality: false,
            aggregation: None,
        };
        
        let results = index_system.query_events(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results.contains(&1));
        
        // Test statistics
        let stats = index_system.get_stats();
        assert_eq!(stats.total_events_indexed, 3);
        assert!(stats.index_memory_usage_mb > 0.0);
        
        // Test distributions
        let type_dist = index_system.get_event_type_distribution();
        assert_eq!(type_dist.get(&SemanticEventType::FilesystemCreate), Some(&1));
        assert_eq!(type_dist.get(&SemanticEventType::FilesystemDelete), Some(&1));
        assert_eq!(type_dist.get(&SemanticEventType::GraphNodeCreate), Some(&1));
        
        let category_dist = index_system.get_category_distribution();
        assert_eq!(category_dist.get(&EventCategory::Filesystem), Some(&2));
        assert_eq!(category_dist.get(&EventCategory::Graph), Some(&1));
    }
}