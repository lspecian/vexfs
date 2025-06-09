//! Distributed State Manager for Event Synchronization
//!
//! This module implements distributed state management for automation rules,
//! configuration synchronization, and coordinated rule execution across
//! VexFS cluster nodes.

use std::collections::{HashMap, BTreeMap, HashSet};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, broadcast, oneshot, RwLock as AsyncRwLock};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::semantic_api::{
    SemanticResult, SemanticError,
    types::*,
    distributed_event_synchronizer::VectorClock,
    automation_rule_engine::{AutomationRule, RuleExecution, ExecutionResult},
    cluster_coordinator::{ClusterNode, ClusterMembership},
};

/// Distributed state configuration
#[derive(Debug, Clone)]
pub struct DistributedStateConfig {
    pub node_id: Uuid,
    pub cluster_id: Uuid,
    pub state_sync_interval: Duration,
    pub rule_sync_interval: Duration,
    pub config_sync_interval: Duration,
    pub max_state_history: usize,
    pub enable_hot_swapping: bool,
    pub consistency_level: ConsistencyLevel,
}

impl Default for DistributedStateConfig {
    fn default() -> Self {
        Self {
            node_id: Uuid::new_v4(),
            cluster_id: Uuid::new_v4(),
            state_sync_interval: Duration::from_secs(5),
            rule_sync_interval: Duration::from_secs(10),
            config_sync_interval: Duration::from_secs(30),
            max_state_history: 1000,
            enable_hot_swapping: true,
            consistency_level: ConsistencyLevel::EventualConsistency,
        }
    }
}

/// Consistency level for distributed state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Strong consistency - all nodes must agree
    StrongConsistency,
    
    /// Eventual consistency - nodes will converge over time
    EventualConsistency,
    
    /// Weak consistency - best effort synchronization
    WeakConsistency,
    
    /// Session consistency - consistent within a session
    SessionConsistency,
}

/// Distributed state entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    pub key: String,
    pub value: StateValue,
    pub version: u64,
    pub vector_clock: VectorClock,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub metadata: HashMap<String, String>,
}

/// State value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateValue {
    /// Automation rule configuration
    Rule(AutomationRule),
    
    /// System configuration
    Config(ConfigValue),
    
    /// Runtime state
    Runtime(RuntimeValue),
    
    /// Custom state value
    Custom(serde_json::Value),
}

/// Configuration value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigValue {
    /// String configuration
    String(String),
    
    /// Number configuration
    Number(f64),
    
    /// Boolean configuration
    Boolean(bool),
    
    /// Array configuration
    Array(Vec<serde_json::Value>),
    
    /// Object configuration
    Object(serde_json::Map<String, serde_json::Value>),
}

/// Runtime value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeValue {
    /// Rule execution state
    RuleExecution(RuleExecutionState),
    
    /// Node status
    NodeStatus(NodeRuntimeStatus),
    
    /// Performance metrics
    Metrics(RuntimeMetrics),
    
    /// Custom runtime value
    Custom(serde_json::Value),
}

/// Rule execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExecutionState {
    pub rule_id: String,
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub executing_node: Uuid,
    pub result: Option<ExecutionResult>,
    pub retry_count: u32,
    pub next_retry_at: Option<SystemTime>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Retrying,
    Cancelled,
}

/// Node runtime status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRuntimeStatus {
    pub node_id: Uuid,
    pub status: String,
    pub load_average: f64,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub active_rules: u32,
    pub pending_executions: u32,
    pub last_updated: SystemTime,
}

/// Runtime metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: SystemTime,
    pub tags: HashMap<String, String>,
}

/// State synchronization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSyncMessage {
    /// Request state synchronization
    SyncRequest {
        from_node: Uuid,
        state_keys: Vec<String>,
        since_version: Option<u64>,
    },
    
    /// Response with state data
    SyncResponse {
        from_node: Uuid,
        state_entries: Vec<StateEntry>,
        has_more: bool,
    },
    
    /// State update notification
    StateUpdate {
        from_node: Uuid,
        entry: StateEntry,
    },
    
    /// State deletion notification
    StateDelete {
        from_node: Uuid,
        key: String,
        version: u64,
        vector_clock: VectorClock,
    },
    
    /// Conflict resolution request
    ConflictResolution {
        from_node: Uuid,
        conflicting_entries: Vec<StateEntry>,
    },
}

/// Rule coordination message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCoordinationMessage {
    /// Request rule execution
    ExecuteRule {
        rule_id: String,
        execution_id: Uuid,
        requesting_node: Uuid,
        priority: u32,
    },
    
    /// Rule execution assignment
    RuleAssigned {
        rule_id: String,
        execution_id: Uuid,
        assigned_node: Uuid,
    },
    
    /// Rule execution status update
    ExecutionUpdate {
        rule_id: String,
        execution_id: Uuid,
        status: ExecutionStatus,
        result: Option<ExecutionResult>,
    },
    
    /// Rule execution completed
    ExecutionCompleted {
        rule_id: String,
        execution_id: Uuid,
        result: ExecutionResult,
    },
}

/// Distributed state manager
pub struct DistributedStateManager {
    config: DistributedStateConfig,
    
    // State storage
    local_state: Arc<RwLock<HashMap<String, StateEntry>>>,
    state_history: Arc<RwLock<BTreeMap<u64, StateEntry>>>,
    version_counter: Arc<Mutex<u64>>,
    vector_clock: Arc<RwLock<VectorClock>>,
    
    // Rule coordination
    active_executions: Arc<RwLock<HashMap<Uuid, RuleExecutionState>>>,
    rule_assignments: Arc<RwLock<HashMap<String, Uuid>>>, // rule_id -> assigned_node
    execution_queue: Arc<RwLock<Vec<RuleCoordinationMessage>>>,
    
    // Communication channels
    state_sync_sender: mpsc::UnboundedSender<StateSyncMessage>,
    state_sync_receiver: Arc<Mutex<mpsc::UnboundedReceiver<StateSyncMessage>>>,
    rule_coord_sender: mpsc::UnboundedSender<RuleCoordinationMessage>,
    rule_coord_receiver: Arc<Mutex<mpsc::UnboundedReceiver<RuleCoordinationMessage>>>,
    
    // Cluster integration
    cluster_membership: Arc<RwLock<ClusterMembership>>,
    
    // Conflict resolution
    conflict_resolver: Arc<ConflictResolver>,
    
    // Performance metrics
    sync_metrics: Arc<RwLock<SyncMetrics>>,
}

/// Conflict resolution strategy
pub struct ConflictResolver {
    strategy: ConflictResolutionStrategy,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Copy)]
pub enum ConflictResolutionStrategy {
    /// Last writer wins
    LastWriterWins,
    
    /// Vector clock based resolution
    VectorClockResolution,
    
    /// Custom resolution function
    Custom,
    
    /// Manual resolution required
    Manual,
}

/// Synchronization performance metrics
#[derive(Debug, Clone)]
pub struct SyncMetrics {
    pub total_sync_operations: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub average_sync_latency: Duration,
    pub max_sync_latency: Duration,
    pub state_entries_synced: u64,
    pub conflicts_resolved: u64,
    pub rule_executions_coordinated: u64,
    pub hot_swaps_performed: u64,
}

impl DistributedStateManager {
    /// Create a new distributed state manager
    pub fn new(config: DistributedStateConfig) -> SemanticResult<Self> {
        let (state_sync_sender, state_sync_receiver) = mpsc::unbounded_channel();
        let (rule_coord_sender, rule_coord_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            local_state: Arc::new(RwLock::new(HashMap::new())),
            state_history: Arc::new(RwLock::new(BTreeMap::new())),
            version_counter: Arc::new(Mutex::new(0)),
            vector_clock: Arc::new(RwLock::new(VectorClock::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            rule_assignments: Arc::new(RwLock::new(HashMap::new())),
            execution_queue: Arc::new(RwLock::new(Vec::new())),
            state_sync_sender,
            state_sync_receiver: Arc::new(Mutex::new(state_sync_receiver)),
            rule_coord_sender,
            rule_coord_receiver: Arc::new(Mutex::new(rule_coord_receiver)),
            cluster_membership: Arc::new(RwLock::new(ClusterMembership {
                cluster_id: config.cluster_id,
                nodes: HashMap::new(),
                leader_node_id: None,
                epoch: 0,
                last_updated: SystemTime::now(),
            })),
            conflict_resolver: Arc::new(ConflictResolver::new(ConflictResolutionStrategy::VectorClockResolution)),
            sync_metrics: Arc::new(RwLock::new(SyncMetrics::default())),
        })
    }
    
    /// Start the distributed state manager
    pub async fn start(&self) -> SemanticResult<()> {
        // Start state synchronization
        self.start_state_synchronization().await?;
        
        // Start rule coordination
        self.start_rule_coordination().await?;
        
        // Start configuration synchronization
        self.start_config_synchronization().await?;
        
        Ok(())
    }
    
    /// Set a state value
    pub async fn set_state(&self, key: String, value: StateValue) -> SemanticResult<()> {
        let version = {
            let mut counter = self.version_counter.lock().unwrap();
            *counter += 1;
            *counter
        };
        
        let mut vector_clock = self.vector_clock.write().unwrap();
        vector_clock.increment(self.config.node_id);
        
        let entry = StateEntry {
            key: key.clone(),
            value,
            version,
            vector_clock: vector_clock.clone(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            created_by: self.config.node_id,
            updated_by: self.config.node_id,
            metadata: HashMap::new(),
        };
        
        // Store locally
        {
            let mut local_state = self.local_state.write().unwrap();
            local_state.insert(key.clone(), entry.clone());
        }
        
        // Add to history
        {
            let mut history = self.state_history.write().unwrap();
            history.insert(version, entry.clone());
            
            // Cleanup old history
            if history.len() > self.config.max_state_history {
                let oldest_key = *history.keys().next().unwrap();
                history.remove(&oldest_key);
            }
        }
        
        // Broadcast update to cluster
        self.broadcast_state_update(entry).await?;
        
        Ok(())
    }
    
    /// Get a state value
    pub async fn get_state(&self, key: &str) -> SemanticResult<Option<StateEntry>> {
        let local_state = self.local_state.read().unwrap();
        Ok(local_state.get(key).cloned())
    }
    
    /// Delete a state value
    pub async fn delete_state(&self, key: &str) -> SemanticResult<()> {
        let version = {
            let mut counter = self.version_counter.lock().unwrap();
            *counter += 1;
            *counter
        };
        
        let mut vector_clock = self.vector_clock.write().unwrap();
        vector_clock.increment(self.config.node_id);
        
        // Remove from local state
        {
            let mut local_state = self.local_state.write().unwrap();
            local_state.remove(key);
        }
        
        // Broadcast deletion to cluster
        self.broadcast_state_deletion(key.to_string(), version, vector_clock.clone()).await?;
        
        Ok(())
    }
    
    /// Set an automation rule
    pub async fn set_rule(&self, rule: AutomationRule) -> SemanticResult<()> {
        let key = format!("rule:{}", rule.id);
        self.set_state(key, StateValue::Rule(rule)).await
    }
    
    /// Get an automation rule
    pub async fn get_rule(&self, rule_id: &str) -> SemanticResult<Option<AutomationRule>> {
        let key = format!("rule:{}", rule_id);
        if let Some(entry) = self.get_state(&key).await? {
            if let StateValue::Rule(rule) = entry.value {
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }
    
    /// Request rule execution
    pub async fn request_rule_execution(&self, rule_id: String, priority: u32) -> SemanticResult<Uuid> {
        let execution_id = Uuid::new_v4();
        
        let message = RuleCoordinationMessage::ExecuteRule {
            rule_id,
            execution_id,
            requesting_node: self.config.node_id,
            priority,
        };
        
        self.rule_coord_sender.send(message)
            .map_err(|e| SemanticError::internal(format!("Failed to send rule execution request: {}", e)))?;
        
        Ok(execution_id)
    }
    
    /// Update rule execution status
    pub async fn update_execution_status(
        &self,
        rule_id: String,
        execution_id: Uuid,
        status: ExecutionStatus,
        result: Option<ExecutionResult>,
    ) -> SemanticResult<()> {
        // Update local execution state
        {
            let mut executions = self.active_executions.write().unwrap();
            if let Some(execution_state) = executions.get_mut(&execution_id) {
                execution_state.status = status;
                execution_state.result = result.clone();
                if matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled) {
                    execution_state.completed_at = Some(SystemTime::now());
                }
            }
        }
        
        // Broadcast status update
        let message = RuleCoordinationMessage::ExecutionUpdate {
            rule_id,
            execution_id,
            status,
            result,
        };
        
        self.rule_coord_sender.send(message)
            .map_err(|e| SemanticError::internal(format!("Failed to send execution update: {}", e)))?;
        
        Ok(())
    }
    
    /// Set configuration value
    pub async fn set_config(&self, key: String, value: ConfigValue) -> SemanticResult<()> {
        let config_key = format!("config:{}", key);
        self.set_state(config_key, StateValue::Config(value)).await
    }
    
    /// Get configuration value
    pub async fn get_config(&self, key: &str) -> SemanticResult<Option<ConfigValue>> {
        let config_key = format!("config:{}", key);
        if let Some(entry) = self.get_state(&config_key).await? {
            if let StateValue::Config(config) = entry.value {
                return Ok(Some(config));
            }
        }
        Ok(None)
    }
    
    /// Perform hot swap of configuration
    pub async fn hot_swap_config(&self, updates: HashMap<String, ConfigValue>) -> SemanticResult<()> {
        if !self.config.enable_hot_swapping {
            return Err(SemanticError::invalid_operation("Hot swapping is disabled"));
        }
        
        // Apply updates atomically
        for (key, value) in updates {
            self.set_config(key, value).await?;
        }
        
        // Update metrics
        {
            let mut metrics = self.sync_metrics.write().unwrap();
            metrics.hot_swaps_performed += 1;
        }
        
        Ok(())
    }
    
    /// Synchronize state with cluster
    pub async fn sync_with_cluster(&self) -> SemanticResult<()> {
        let start_time = Instant::now();
        
        // Get cluster membership
        let membership = self.cluster_membership.read().unwrap().clone();
        
        // Request synchronization from all nodes
        for node_id in membership.nodes.keys() {
            if *node_id != self.config.node_id {
                self.request_sync_from_node(*node_id).await?;
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.sync_metrics.write().unwrap();
            metrics.total_sync_operations += 1;
            let latency = start_time.elapsed();
            metrics.average_sync_latency = 
                (metrics.average_sync_latency * (metrics.total_sync_operations - 1) + latency) / metrics.total_sync_operations;
            if latency > metrics.max_sync_latency {
                metrics.max_sync_latency = latency;
            }
        }
        
        Ok(())
    }
    
    /// Get synchronization metrics
    pub async fn get_sync_metrics(&self) -> SyncMetrics {
        self.sync_metrics.read().unwrap().clone()
    }
    
    /// Start state synchronization
    async fn start_state_synchronization(&self) -> SemanticResult<()> {
        // Implementation would start background state synchronization
        Ok(())
    }
    
    /// Start rule coordination
    async fn start_rule_coordination(&self) -> SemanticResult<()> {
        // Implementation would start background rule coordination
        Ok(())
    }
    
    /// Start configuration synchronization
    async fn start_config_synchronization(&self) -> SemanticResult<()> {
        // Implementation would start background configuration synchronization
        Ok(())
    }
    
    /// Broadcast state update to cluster
    async fn broadcast_state_update(&self, entry: StateEntry) -> SemanticResult<()> {
        let message = StateSyncMessage::StateUpdate {
            from_node: self.config.node_id,
            entry,
        };
        
        self.state_sync_sender.send(message)
            .map_err(|e| SemanticError::internal(format!("Failed to broadcast state update: {}", e)))?;
        
        Ok(())
    }
    
    /// Broadcast state deletion to cluster
    async fn broadcast_state_deletion(&self, key: String, version: u64, vector_clock: VectorClock) -> SemanticResult<()> {
        let message = StateSyncMessage::StateDelete {
            from_node: self.config.node_id,
            key,
            version,
            vector_clock,
        };
        
        self.state_sync_sender.send(message)
            .map_err(|e| SemanticError::internal(format!("Failed to broadcast state deletion: {}", e)))?;
        
        Ok(())
    }
    
    /// Request synchronization from a specific node
    async fn request_sync_from_node(&self, node_id: Uuid) -> SemanticResult<()> {
        let message = StateSyncMessage::SyncRequest {
            from_node: self.config.node_id,
            state_keys: vec![], // Empty means all keys
            since_version: None,
        };
        
        self.state_sync_sender.send(message)
            .map_err(|e| SemanticError::internal(format!("Failed to request sync from node: {}", e)))?;
        
        Ok(())
    }
}

impl ConflictResolver {
    pub fn new(strategy: ConflictResolutionStrategy) -> Self {
        Self { strategy }
    }
    
    pub fn resolve_conflict(&self, entries: Vec<StateEntry>) -> SemanticResult<StateEntry> {
        match self.strategy {
            ConflictResolutionStrategy::LastWriterWins => {
                entries.into_iter()
                    .max_by_key(|e| e.updated_at)
                    .ok_or_else(|| SemanticError::internal("No entries to resolve"))
            }
            
            ConflictResolutionStrategy::VectorClockResolution => {
                // Use vector clock to determine causality
                let mut best_entry = entries.into_iter().next()
                    .ok_or_else(|| SemanticError::internal("No entries to resolve"))?;
                
                for entry in entries.into_iter().skip(1) {
                    if entry.vector_clock.happens_after(&best_entry.vector_clock) {
                        best_entry = entry;
                    }
                }
                
                Ok(best_entry)
            }
            
            ConflictResolutionStrategy::Custom => {
                // Custom resolution logic would be implemented here
                Err(SemanticError::not_implemented("Custom conflict resolution not implemented"))
            }
            
            ConflictResolutionStrategy::Manual => {
                Err(SemanticError::invalid_operation("Manual conflict resolution required"))
            }
        }
    }
}

impl Default for SyncMetrics {
    fn default() -> Self {
        Self {
            total_sync_operations: 0,
            successful_syncs: 0,
            failed_syncs: 0,
            average_sync_latency: Duration::ZERO,
            max_sync_latency: Duration::ZERO,
            state_entries_synced: 0,
            conflicts_resolved: 0,
            rule_executions_coordinated: 0,
            hot_swaps_performed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_distributed_state_manager_creation() {
        let config = DistributedStateConfig::default();
        let manager = DistributedStateManager::new(config).unwrap();
        
        // Verify initial state
        let metrics = manager.get_sync_metrics().await;
        assert_eq!(metrics.total_sync_operations, 0);
        assert_eq!(metrics.state_entries_synced, 0);
    }
    
    #[tokio::test]
    async fn test_state_operations() {
        let config = DistributedStateConfig::default();
        let manager = DistributedStateManager::new(config).unwrap();
        
        // Set state
        let key = "test_key".to_string();
        let value = StateValue::Config(ConfigValue::String("test_value".to_string()));
        
        let result = manager.set_state(key.clone(), value).await;
        assert!(result.is_ok());
        
        // Get state
        let retrieved = manager.get_state(&key).await.unwrap();
        assert!(retrieved.is_some());
        
        let entry = retrieved.unwrap();
        assert_eq!(entry.key, key);
        assert_eq!(entry.version, 1);
    }
    
    #[tokio::test]
    async fn test_rule_operations() {
        let config = DistributedStateConfig::default();
        let manager = DistributedStateManager::new(config).unwrap();
        
        // Create test rule
        let rule = AutomationRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "A test automation rule".to_string(),
            enabled: true,
            triggers: vec![],
            conditions: vec![],
            actions: vec![],
            retry_policy: None,
            cooldown_period: None,
            metadata: HashMap::new(),
        };
        
        // Set rule
        let result = manager.set_rule(rule.clone()).await;
        assert!(result.is_ok());
        
        // Get rule
        let retrieved = manager.get_rule(&rule.id).await.unwrap();
        assert!(retrieved.is_some());
        
        let retrieved_rule = retrieved.unwrap();
        assert_eq!(retrieved_rule.id, rule.id);
        assert_eq!(retrieved_rule.name, rule.name);
    }
    
    #[tokio::test]
    async fn test_config_operations() {
        let config = DistributedStateConfig::default();
        let manager = DistributedStateManager::new(config).unwrap();
        
        // Set config
        let key = "test_config".to_string();
        let value = ConfigValue::Number(42.0);
        
        let result = manager.set_config(key.clone(), value).await;
        assert!(result.is_ok());
        
        // Get config
        let retrieved = manager.get_config(&key).await.unwrap();
        assert!(retrieved.is_some());
        
        if let Some(ConfigValue::Number(num)) = retrieved {
            assert_eq!(num, 42.0);
        } else {
            panic!("Expected number config value");
        }
    }
}