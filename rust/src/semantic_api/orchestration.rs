//! Agent Orchestration System
//!
//! This module implements the orchestration primitives for coordinating
//! multi-agent workflows, including task queues, pub/sub messaging,
//! and conflict resolution protocols.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock, Mutex};
use tracing::{debug, info, warn, error, instrument};
use uuid::Uuid;

use crate::semantic_api::{SemanticResult, SemanticError, types::*};

/// Agent orchestration manager
pub struct AgentOrchestrator {
    task_queue: Arc<Mutex<TaskQueue>>,
    message_broker: Arc<MessageBroker>,
    conflict_resolver: Arc<ConflictResolver>,
    agent_registry: Arc<RwLock<HashMap<String, AgentInfo>>>,
    orchestration_stats: Arc<RwLock<OrchestrationStats>>,
    config: OrchestrationConfig,
}

/// Configuration for agent orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub max_concurrent_tasks_per_agent: usize,
    pub task_timeout_seconds: u64,
    pub max_queue_size: usize,
    pub conflict_resolution_timeout_seconds: u64,
    pub heartbeat_interval_seconds: u64,
    pub agent_timeout_seconds: u64,
    pub enable_task_prioritization: bool,
    pub enable_load_balancing: bool,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks_per_agent: 10,
            task_timeout_seconds: 300, // 5 minutes
            max_queue_size: 10000,
            conflict_resolution_timeout_seconds: 30,
            heartbeat_interval_seconds: 30,
            agent_timeout_seconds: 300,
            enable_task_prioritization: true,
            enable_load_balancing: true,
        }
    }
}

/// Agent information for orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub current_load: usize,
    pub max_concurrent_tasks: usize,
    pub last_heartbeat: DateTime<Utc>,
    pub status: AgentStatus,
    pub performance_metrics: AgentPerformanceMetrics,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Busy,
    Idle,
    Offline,
    Error(String),
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformanceMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub average_task_duration_ms: f64,
    pub success_rate: f64,
    pub last_updated: DateTime<Utc>,
}

/// Task queue for managing agent tasks
pub struct TaskQueue {
    high_priority: VecDeque<AgentTask>,
    normal_priority: VecDeque<AgentTask>,
    low_priority: VecDeque<AgentTask>,
    pending_tasks: HashMap<String, AgentTask>,
    completed_tasks: HashMap<String, TaskResult>,
    config: OrchestrationConfig,
}

/// Agent task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub task_id: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub assigned_agent: Option<String>,
    pub required_capabilities: Vec<String>,
    pub payload: serde_json::Value,
    pub dependencies: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub metadata: HashMap<String, String>,
}

/// Types of agent tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Query,
    Analysis,
    Reasoning,
    DataProcessing,
    Monitoring,
    Coordination,
    Custom(String),
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub execution_time_ms: u64,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Message broker for inter-agent communication
pub struct MessageBroker {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<AgentMessage>>>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<String>>>>, // topic -> agent_ids
    message_history: Arc<RwLock<VecDeque<AgentMessage>>>,
    config: MessageBrokerConfig,
}

/// Message broker configuration
#[derive(Debug, Clone)]
pub struct MessageBrokerConfig {
    pub max_channel_capacity: usize,
    pub max_message_history: usize,
    pub message_ttl_seconds: u64,
}

impl Default for MessageBrokerConfig {
    fn default() -> Self {
        Self {
            max_channel_capacity: 1000,
            max_message_history: 10000,
            message_ttl_seconds: 3600, // 1 hour
        }
    }
}

/// Inter-agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub message_id: String,
    pub from_agent: String,
    pub to_agent: Option<String>, // None for broadcast
    pub topic: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

/// Types of inter-agent messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Notification,
    Broadcast,
    Heartbeat,
    Coordination,
    Conflict,
}

/// Conflict resolver for handling competing agent operations
pub struct ConflictResolver {
    active_conflicts: Arc<RwLock<HashMap<String, Conflict>>>,
    resolution_strategies: HashMap<ConflictType, ResolutionStrategy>,
    config: ConflictResolverConfig,
}

/// Conflict resolution configuration
#[derive(Debug, Clone)]
pub struct ConflictResolverConfig {
    pub max_resolution_time_seconds: u64,
    pub enable_automatic_resolution: bool,
    pub priority_based_resolution: bool,
}

impl Default for ConflictResolverConfig {
    fn default() -> Self {
        Self {
            max_resolution_time_seconds: 30,
            enable_automatic_resolution: true,
            priority_based_resolution: true,
        }
    }
}

/// Conflict between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub conflict_id: String,
    pub conflict_type: ConflictType,
    pub involved_agents: Vec<String>,
    pub resource_id: String,
    pub detected_at: DateTime<Utc>,
    pub resolution_deadline: DateTime<Utc>,
    pub status: ConflictStatus,
    pub resolution: Option<ConflictResolution>,
}

/// Types of conflicts
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    ResourceContention,
    TaskDuplication,
    DataInconsistency,
    CapabilityOverlap,
    PriorityConflict,
}

/// Conflict status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStatus {
    Detected,
    Analyzing,
    Resolving,
    Resolved,
    Escalated,
}

/// Conflict resolution strategies
#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    FirstComeFirstServed,
    HighestPriority,
    LoadBalancing,
    Negotiation,
    Escalation,
}

/// Conflict resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub strategy_used: String,
    pub winner_agent: Option<String>,
    pub resolution_actions: Vec<ResolutionAction>,
    pub resolved_at: DateTime<Utc>,
}

/// Resolution action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionAction {
    pub action_type: ActionType,
    pub target_agent: String,
    pub description: String,
}

/// Types of resolution actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Reassign,
    Cancel,
    Delay,
    Merge,
    Split,
}

/// Orchestration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStats {
    pub active_agents: u32,
    pub total_tasks_queued: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub total_messages_sent: u64,
    pub total_conflicts_detected: u64,
    pub total_conflicts_resolved: u64,
    pub average_task_completion_time_ms: f64,
    pub average_queue_wait_time_ms: f64,
    pub current_queue_size: usize,
    pub last_updated: DateTime<Utc>,
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator
    pub fn new(config: OrchestrationConfig) -> Self {
        let task_queue = TaskQueue::new(config.clone());
        let message_broker = MessageBroker::new(MessageBrokerConfig::default());
        let conflict_resolver = ConflictResolver::new(ConflictResolverConfig::default());

        Self {
            task_queue: Arc::new(Mutex::new(task_queue)),
            message_broker: Arc::new(message_broker),
            conflict_resolver: Arc::new(conflict_resolver),
            agent_registry: Arc::new(RwLock::new(HashMap::new())),
            orchestration_stats: Arc::new(RwLock::new(OrchestrationStats::default())),
            config,
        }
    }

    /// Register an agent with the orchestrator
    #[instrument(skip(self))]
    pub async fn register_agent(&self, agent_info: AgentInfo) -> SemanticResult<()> {
        let mut registry = self.agent_registry.write().await;
        registry.insert(agent_info.agent_id.clone(), agent_info.clone());
        
        self.update_stats(|stats| stats.active_agents = registry.len() as u32).await;
        
        info!("Registered agent: {}", agent_info.agent_id);
        Ok(())
    }

    /// Unregister an agent
    #[instrument(skip(self))]
    pub async fn unregister_agent(&self, agent_id: &str) -> SemanticResult<()> {
        let mut registry = self.agent_registry.write().await;
        if registry.remove(agent_id).is_some() {
            self.update_stats(|stats| stats.active_agents = registry.len() as u32).await;
            info!("Unregistered agent: {}", agent_id);
            Ok(())
        } else {
            Err(SemanticError::InvalidRequest(
                format!("Agent {} not found", agent_id)
            ))
        }
    }

    /// Submit a task to the queue
    #[instrument(skip(self))]
    pub async fn submit_task(&self, task: AgentTask) -> SemanticResult<String> {
        let task_id = task.task_id.clone();
        
        // Check for conflicts
        if let Some(conflict) = self.conflict_resolver.detect_conflict(&task).await? {
            warn!("Conflict detected for task: {}", task_id);
            self.conflict_resolver.handle_conflict(conflict).await?;
        }
        
        // Add to queue
        {
            let mut queue = self.task_queue.lock().await;
            queue.enqueue(task)?;
        }
        
        self.update_stats(|stats| {
            stats.total_tasks_queued += 1;
            stats.current_queue_size += 1;
        }).await;
        
        // Try to assign immediately
        self.try_assign_tasks().await?;
        
        info!("Task submitted: {}", task_id);
        Ok(task_id)
    }

    /// Try to assign pending tasks to available agents
    #[instrument(skip(self))]
    pub async fn try_assign_tasks(&self) -> SemanticResult<usize> {
        let mut assigned_count = 0;
        
        loop {
            let task = {
                let mut queue = self.task_queue.lock().await;
                queue.dequeue()
            };
            
            let task = match task {
                Some(t) => t,
                None => break, // No more tasks
            };
            
            // Find suitable agent
            if let Some(agent_id) = self.find_suitable_agent(&task).await? {
                // Assign task
                self.assign_task_to_agent(task, &agent_id).await?;
                assigned_count += 1;
            } else {
                // Put task back in queue
                let mut queue = self.task_queue.lock().await;
                queue.enqueue(task)?;
                break; // No suitable agents available
            }
        }
        
        if assigned_count > 0 {
            debug!("Assigned {} tasks to agents", assigned_count);
        }
        
        Ok(assigned_count)
    }

    /// Find a suitable agent for a task
    async fn find_suitable_agent(&self, task: &AgentTask) -> SemanticResult<Option<String>> {
        let registry = self.agent_registry.read().await;
        
        let mut suitable_agents: Vec<_> = registry
            .values()
            .filter(|agent| {
                // Check if agent is active and has capacity
                matches!(agent.status, AgentStatus::Active | AgentStatus::Idle) &&
                agent.current_load < agent.max_concurrent_tasks &&
                // Check if agent has required capabilities
                task.required_capabilities.iter().all(|cap| agent.capabilities.contains(cap))
            })
            .collect();
        
        if suitable_agents.is_empty() {
            return Ok(None);
        }
        
        // Sort by load balancing criteria if enabled
        if self.config.enable_load_balancing {
            suitable_agents.sort_by(|a, b| {
                // Primary: current load (ascending)
                let load_cmp = a.current_load.cmp(&b.current_load);
                if load_cmp != std::cmp::Ordering::Equal {
                    return load_cmp;
                }
                
                // Secondary: success rate (descending)
                b.performance_metrics.success_rate
                    .partial_cmp(&a.performance_metrics.success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        
        Ok(Some(suitable_agents[0].agent_id.clone()))
    }

    /// Assign a task to a specific agent
    async fn assign_task_to_agent(&self, mut task: AgentTask, agent_id: &str) -> SemanticResult<()> {
        task.assigned_agent = Some(agent_id.to_string());
        
        // Update agent load
        {
            let mut registry = self.agent_registry.write().await;
            if let Some(agent) = registry.get_mut(agent_id) {
                agent.current_load += 1;
                agent.status = if agent.current_load >= agent.max_concurrent_tasks {
                    AgentStatus::Busy
                } else {
                    AgentStatus::Active
                };
            }
        }
        
        // Send task to agent via message broker
        let message = AgentMessage {
            message_id: Uuid::new_v4().to_string(),
            from_agent: "orchestrator".to_string(),
            to_agent: Some(agent_id.to_string()),
            topic: "task_assignment".to_string(),
            message_type: MessageType::Request,
            payload: serde_json::to_value(&task)?,
            timestamp: Utc::now(),
            expires_at: task.deadline,
            correlation_id: Some(task.task_id.clone()),
            reply_to: Some("orchestrator".to_string()),
        };
        
        self.message_broker.publish(&message).await?;
        
        info!("Assigned task {} to agent {}", task.task_id, agent_id);
        Ok(())
    }

    /// Handle task completion
    #[instrument(skip(self))]
    pub async fn handle_task_completion(&self, result: TaskResult) -> SemanticResult<()> {
        // Update agent load
        {
            let mut registry = self.agent_registry.write().await;
            if let Some(agent) = registry.get_mut(&result.agent_id) {
                if agent.current_load > 0 {
                    agent.current_load -= 1;
                }
                
                agent.status = if agent.current_load == 0 {
                    AgentStatus::Idle
                } else {
                    AgentStatus::Active
                };
                
                // Update performance metrics
                agent.performance_metrics.last_updated = Utc::now();
                match result.status {
                    TaskStatus::Completed => {
                        agent.performance_metrics.tasks_completed += 1;
                    }
                    TaskStatus::Failed => {
                        agent.performance_metrics.tasks_failed += 1;
                    }
                    _ => {}
                }
                
                let total_tasks = agent.performance_metrics.tasks_completed + agent.performance_metrics.tasks_failed;
                if total_tasks > 0 {
                    agent.performance_metrics.success_rate = 
                        agent.performance_metrics.tasks_completed as f64 / total_tasks as f64;
                }
            }
        }
        
        // Update statistics
        self.update_stats(|stats| {
            match result.status {
                TaskStatus::Completed => stats.total_tasks_completed += 1,
                TaskStatus::Failed => stats.total_tasks_failed += 1,
                _ => {}
            }
            stats.current_queue_size = stats.current_queue_size.saturating_sub(1);
        }).await;
        
        // Try to assign more tasks
        self.try_assign_tasks().await?;
        
        info!("Task completed: {} by agent {}", result.task_id, result.agent_id);
        Ok(())
    }

    /// Subscribe agent to a topic
    pub async fn subscribe_agent(&self, agent_id: &str, topic: &str) -> SemanticResult<broadcast::Receiver<AgentMessage>> {
        self.message_broker.subscribe(agent_id, topic).await
    }

    /// Publish message to a topic
    pub async fn publish_message(&self, message: &AgentMessage) -> SemanticResult<()> {
        self.message_broker.publish(message).await
    }

    /// Get orchestration statistics
    pub async fn get_stats(&self) -> OrchestrationStats {
        let stats = self.orchestration_stats.read().await;
        let mut updated_stats = stats.clone();
        updated_stats.last_updated = Utc::now();
        updated_stats
    }

    /// Update statistics with a closure
    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut OrchestrationStats),
    {
        let mut stats = self.orchestration_stats.write().await;
        update_fn(&mut *stats);
    }
}

impl TaskQueue {
    fn new(config: OrchestrationConfig) -> Self {
        Self {
            high_priority: VecDeque::new(),
            normal_priority: VecDeque::new(),
            low_priority: VecDeque::new(),
            pending_tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
            config,
        }
    }

    fn enqueue(&mut self, task: AgentTask) -> SemanticResult<()> {
        if self.total_size() >= self.config.max_queue_size {
            return Err(SemanticError::InternalError(
                "Task queue is full".to_string()
            ));
        }

        match task.priority {
            TaskPriority::Critical | TaskPriority::High => {
                self.high_priority.push_back(task);
            }
            TaskPriority::Normal => {
                self.normal_priority.push_back(task);
            }
            TaskPriority::Low => {
                self.low_priority.push_back(task);
            }
        }

        Ok(())
    }

    fn dequeue(&mut self) -> Option<AgentTask> {
        // Priority order: high -> normal -> low
        self.high_priority.pop_front()
            .or_else(|| self.normal_priority.pop_front())
            .or_else(|| self.low_priority.pop_front())
    }

    fn total_size(&self) -> usize {
        self.high_priority.len() + self.normal_priority.len() + self.low_priority.len()
    }
}

impl MessageBroker {
    fn new(config: MessageBrokerConfig) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(VecDeque::new())),
            config,
        }
    }

    async fn subscribe(&self, agent_id: &str, topic: &str) -> SemanticResult<broadcast::Receiver<AgentMessage>> {
        let mut channels = self.channels.write().await;
        let mut subscribers = self.subscribers.write().await;

        // Create channel if it doesn't exist
        let sender = channels.entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(self.config.max_channel_capacity);
                tx
            });

        // Add subscriber
        subscribers.entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(agent_id.to_string());

        Ok(sender.subscribe())
    }

    async fn publish(&self, message: &AgentMessage) -> SemanticResult<()> {
        // Add to history
        {
            let mut history = self.message_history.write().await;
            history.push_back(message.clone());
            
            // Trim history if too large
            while history.len() > self.config.max_message_history {
                history.pop_front();
            }
        }

        // Send to topic channel
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(&message.topic) {
            let _ = sender.send(message.clone()); // Ignore if no receivers
        }

        Ok(())
    }
}

impl ConflictResolver {
    fn new(config: ConflictResolverConfig) -> Self {
        let mut resolution_strategies = HashMap::new();
        resolution_strategies.insert(ConflictType::ResourceContention, ResolutionStrategy::HighestPriority);
        resolution_strategies.insert(ConflictType::TaskDuplication, ResolutionStrategy::FirstComeFirstServed);
        resolution_strategies.insert(ConflictType::DataInconsistency, ResolutionStrategy::Escalation);
        resolution_strategies.insert(ConflictType::CapabilityOverlap, ResolutionStrategy::LoadBalancing);
        resolution_strategies.insert(ConflictType::PriorityConflict, ResolutionStrategy::Negotiation);

        Self {
            active_conflicts: Arc::new(RwLock::new(HashMap::new())),
            resolution_strategies,
            config,
        }
    }

    async fn detect_conflict(&self, _task: &AgentTask) -> SemanticResult<Option<Conflict>> {
        // TODO: Implement conflict detection logic
        // This would analyze the task against current active tasks and resources
        Ok(None)
    }

    async fn handle_conflict(&self, conflict: Conflict) -> SemanticResult<ConflictResolution> {
        // TODO: Implement conflict resolution logic
        // This would apply the appropriate resolution strategy
        
        let resolution = ConflictResolution {
            strategy_used: "placeholder".to_string(),
            winner_agent: None,
            resolution_actions: vec![],
            resolved_at: Utc::now(),
        };

        Ok(resolution)
    }
}

impl Default for OrchestrationStats {
    fn default() -> Self {
        Self {
            active_agents: 0,
            total_tasks_queued: 0,
            total_tasks_completed: 0,
            total_tasks_failed: 0,
            total_messages_sent: 0,
            total_conflicts_detected: 0,
            total_conflicts_resolved: 0,
            average_task_completion_time_ms: 0.0,
            average_queue_wait_time_ms: 0.0,
            current_queue_size: 0,
            last_updated: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_registration() {
        let orchestrator = AgentOrchestrator::new(OrchestrationConfig::default());
        
        let agent_info = AgentInfo {
            agent_id: "test_agent".to_string(),
            agent_type: "reasoning".to_string(),
            capabilities: vec!["query".to_string(), "analyze".to_string()],
            current_load: 0,
            max_concurrent_tasks: 5,
            last_heartbeat: Utc::now(),
            status: AgentStatus::Active,
            performance_metrics: AgentPerformanceMetrics {
                tasks_completed: 0,
                tasks_failed: 0,
                average_task_duration_ms: 0.0,
                success_rate: 1.0,
                last_updated: Utc::now(),
            },
        };
        
        assert!(orchestrator.register_agent(agent_info).await.is_ok());
        
        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.active_agents, 1);
    }

    #[tokio::test]
    async fn test_task_submission() {
        let orchestrator = AgentOrchestrator::new(OrchestrationConfig::default());
        
        let task = AgentTask {
            task_id: Uuid::new_v4().to_string(),
            task_type: TaskType::Query,
            priority: TaskPriority::Normal,
            assigned_agent: None,
            required_capabilities: vec!["query".to_string()],
            payload: serde_json::json!({"query": "test"}),
            dependencies: vec![],
            created_at: Utc::now(),
            scheduled_at: None,
            deadline: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 300,
            metadata: HashMap::new(),
        };
        
        let task_id = orchestrator.submit_task(task).await.unwrap();
        assert!(!task_id.is_empty());
        
        let stats = orchestrator.get_stats().await;
        assert_eq!(stats.total_tasks_queued, 1);
    }

    #[test]
    fn test_task_queue_priority() {
        let mut queue = TaskQueue::new(OrchestrationConfig::default());
        
        let low_task = AgentTask {
            task_id: "low".to_string(),
            priority: TaskPriority::Low,
            task_type: TaskType::Query,
            assigned_agent: None,
            required_capabilities: vec![],
            payload: serde_json::json!({}),
            dependencies: vec![],
            created_at: Utc::now(),
            scheduled_at: None,
            deadline: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 300,
            metadata: HashMap::new(),
        };
        
        let high_task = AgentTask {
            task_id: "high".to_string(),
            priority: TaskPriority::High,
            ..low_task.clone()
        };
        
        // Add low priority first, then high priority
        queue.enqueue(low_task).unwrap();
        queue.enqueue(high_task).unwrap();
        
        // High priority should come out first
        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.task_id, "high");
        
        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.task_id, "low");
    }
}