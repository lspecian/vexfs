//! Reactive Automation Framework for Event-Driven Behavior
//!
//! This module implements a comprehensive reactive automation framework that combines
//! complex event processing, rule-based automation, and real-time analytics to create
//! intelligent, self-managing filesystem behavior.

use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, broadcast, oneshot, Semaphore};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use futures::future::join_all;

use crate::semantic_api::{
    SemanticResult, SemanticError,
    types::*,
    complex_event_processor::{ComplexEventProcessor, ComplexEventProcessorConfig, PatternMatch, EventPattern as CEPEventPattern},
    automation_rule_engine::{AutomationRuleEngine, AutomationRuleEngineConfig, AutomationRule, RuleExecutionResult, RuleTrigger},
    event_analytics_engine::{EventAnalyticsEngine, AnalyticsConfig, AnalyticsResult},
    distributed_coordination::{DistributedEventCoordinator, DistributedCoordinatorConfig},
    event_routing::{EventRoutingEngine, EventRoutingConfig},
};

/// Reactive automation framework configuration
#[derive(Debug, Clone)]
pub struct ReactiveAutomationConfig {
    /// Performance targets
    pub max_automation_latency_ms: u64,
    pub target_throughput_events_per_sec: u64,
    pub max_concurrent_workflows: usize,
    pub max_active_automations: usize,
    
    /// Resource limits
    pub max_memory_usage_mb: u64,
    pub max_cpu_usage_percent: f64,
    pub automation_timeout: Duration,
    pub workflow_timeout: Duration,
    
    /// Reliability settings
    pub enable_fault_tolerance: bool,
    pub enable_rollback_compensation: bool,
    pub enable_hot_reload: bool,
    pub enable_multi_tenant_isolation: bool,
    
    /// Integration settings
    pub enable_machine_learning: bool,
    pub enable_natural_language_rules: bool,
    pub enable_visual_workflows: bool,
    pub enable_compliance_audit: bool,
    
    /// Scaling settings
    pub enable_horizontal_scaling: bool,
    pub auto_scaling_threshold: f64,
    pub min_instances: u32,
    pub max_instances: u32,
}

impl Default for ReactiveAutomationConfig {
    fn default() -> Self {
        Self {
            max_automation_latency_ms: 100,
            target_throughput_events_per_sec: 100_000,
            max_concurrent_workflows: 10_000,
            max_active_automations: 50_000,
            max_memory_usage_mb: 1024,
            max_cpu_usage_percent: 80.0,
            automation_timeout: Duration::from_secs(30),
            workflow_timeout: Duration::from_secs(300),
            enable_fault_tolerance: true,
            enable_rollback_compensation: true,
            enable_hot_reload: true,
            enable_multi_tenant_isolation: true,
            enable_machine_learning: true,
            enable_natural_language_rules: true,
            enable_visual_workflows: true,
            enable_compliance_audit: true,
            enable_horizontal_scaling: true,
            auto_scaling_threshold: 0.8,
            min_instances: 1,
            max_instances: 10,
        }
    }
}

/// Reactive automation workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveWorkflow {
    pub workflow_id: Uuid,
    pub name: String,
    pub description: String,
    pub workflow_type: WorkflowType,
    pub trigger_patterns: Vec<WorkflowTrigger>,
    pub steps: Vec<WorkflowStep>,
    pub compensation_steps: Vec<CompensationStep>,
    pub priority: WorkflowPriority,
    pub tenant_id: Option<String>,
    pub enabled: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of reactive workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowType {
    /// Simple linear workflow
    Linear,
    /// Parallel execution workflow
    Parallel,
    /// Conditional branching workflow
    Conditional,
    /// State machine workflow
    StateMachine,
    /// Event-driven reactive workflow
    EventDriven,
    /// Machine learning enhanced workflow
    MLEnhanced,
}

/// Workflow trigger patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowTrigger {
    /// Complex event pattern trigger
    EventPattern {
        pattern_id: Uuid,
        confidence_threshold: f64,
    },
    /// Analytics anomaly trigger
    Anomaly {
        anomaly_type: String,
        severity_threshold: f64,
    },
    /// System state change trigger
    StateChange {
        state_path: String,
        condition: StateCondition,
    },
    /// Time-based trigger
    Schedule {
        cron_expression: String,
        timezone: String,
    },
    /// External API trigger
    ExternalEvent {
        event_source: String,
        event_filter: HashMap<String, serde_json::Value>,
    },
    /// Machine learning prediction trigger
    MLPrediction {
        model_id: String,
        confidence_threshold: f64,
    },
}

/// State condition for triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCondition {
    pub operator: ComparisonOperator,
    pub value: serde_json::Value,
    pub tolerance: Option<f64>,
}

/// Comparison operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    Matches(String), // Regex
}

/// Workflow execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: Uuid,
    pub name: String,
    pub step_type: WorkflowStepType,
    pub action: WorkflowAction,
    pub conditions: Vec<StepCondition>,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub depends_on: Vec<Uuid>,
}

/// Types of workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStepType {
    Action,
    Decision,
    Parallel,
    Loop,
    Wait,
    Compensation,
}

/// Workflow actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowAction {
    /// Execute automation rule
    ExecuteRule {
        rule_id: Uuid,
        parameters: HashMap<String, serde_json::Value>,
    },
    /// Emit semantic event
    EmitEvent {
        event_type: SemanticEventType,
        event_data: HashMap<String, serde_json::Value>,
    },
    /// Call external API
    CallAPI {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: Option<serde_json::Value>,
    },
    /// Update system state
    UpdateState {
        state_path: String,
        new_value: serde_json::Value,
    },
    /// Execute script
    ExecuteScript {
        script_language: ScriptLanguage,
        script_content: String,
        parameters: HashMap<String, serde_json::Value>,
    },
    /// Trigger sub-workflow
    TriggerWorkflow {
        workflow_id: Uuid,
        parameters: HashMap<String, serde_json::Value>,
    },
    /// Machine learning inference
    MLInference {
        model_id: String,
        input_data: HashMap<String, serde_json::Value>,
    },
}

/// Script languages supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptLanguage {
    JavaScript,
    Python,
    Lua,
    WebAssembly,
    SQL,
}

/// Step execution conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    pub condition_type: ConditionType,
    pub expression: String,
    pub expected_result: serde_json::Value,
}

/// Types of step conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    PreCondition,
    PostCondition,
    GuardCondition,
    LoopCondition,
}

/// Compensation step for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationStep {
    pub step_id: Uuid,
    pub compensates_step: Uuid,
    pub action: WorkflowAction,
    pub timeout: Duration,
}

/// Workflow priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WorkflowPriority {
    Critical = 1,
    High = 2,
    Normal = 3,
    Low = 4,
}

/// Retry policy for workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub retry_on_errors: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            retry_on_errors: vec!["timeout".to_string(), "network".to_string()],
        }
    }
}

/// Workflow execution context
#[derive(Debug, Clone)]
pub struct WorkflowExecutionContext {
    pub workflow_id: Uuid,
    pub execution_id: Uuid,
    pub tenant_id: Option<String>,
    pub trigger_event: Option<SemanticEvent>,
    pub trigger_pattern: Option<PatternMatch>,
    pub variables: HashMap<String, serde_json::Value>,
    pub execution_start: Instant,
    pub current_step: Option<Uuid>,
}

/// Workflow execution result
#[derive(Debug, Clone)]
pub struct WorkflowExecutionResult {
    pub workflow_id: Uuid,
    pub execution_id: Uuid,
    pub success: bool,
    pub steps_completed: u32,
    pub steps_failed: u32,
    pub execution_duration: Duration,
    pub error_message: Option<String>,
    pub output_data: HashMap<String, serde_json::Value>,
    pub compensation_executed: bool,
}

/// Step execution result
#[derive(Debug, Clone)]
pub struct StepExecutionResult {
    pub success: bool,
    pub output_data: HashMap<String, serde_json::Value>,
    pub execution_duration: Duration,
    pub error_message: Option<String>,
    pub compensation_required: bool,
}

/// Performance metrics for reactive automation
#[derive(Debug, Clone)]
pub struct ReactiveAutomationMetrics {
    pub automation_latency_ms: f64,
    pub throughput_events_per_sec: f64,
    pub active_workflows: u64,
    pub completed_workflows: u64,
    pub failed_workflows: u64,
    pub compensation_rate: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub pattern_match_rate: f64,
    pub rule_execution_rate: f64,
}

impl Default for ReactiveAutomationMetrics {
    fn default() -> Self {
        Self {
            automation_latency_ms: 0.0,
            throughput_events_per_sec: 0.0,
            active_workflows: 0,
            completed_workflows: 0,
            failed_workflows: 0,
            compensation_rate: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            pattern_match_rate: 0.0,
            rule_execution_rate: 0.0,
        }
    }
}

/// Reactive automation result
#[derive(Debug, Clone)]
pub struct ReactiveAutomationResult {
    pub processing_latency_ms: f64,
    pub pattern_matches: Vec<PatternMatch>,
    pub analytics_result: AnalyticsResult,
    pub triggered_workflows: Vec<WorkflowExecutionResult>,
    pub system_state: ReactiveSystemState,
}

/// Reactive automation framework
pub struct ReactiveAutomationFramework {
    config: ReactiveAutomationConfig,
    
    // Core engines
    cep_engine: Arc<ComplexEventProcessor>,
    rule_engine: Arc<AutomationRuleEngine>,
    analytics_engine: Arc<EventAnalyticsEngine>,
    routing_engine: Arc<EventRoutingEngine>,
    coordination_engine: Option<Arc<DistributedEventCoordinator>>,
    
    // Workflow management
    workflows: Arc<RwLock<HashMap<Uuid, ReactiveWorkflow>>>,
    active_executions: Arc<RwLock<HashMap<Uuid, WorkflowExecutionContext>>>,
    execution_history: Arc<RwLock<VecDeque<WorkflowExecutionResult>>>,
    
    // State management
    reactive_state: Arc<RwLock<ReactiveSystemState>>,
    
    // Communication channels
    event_sender: mpsc::UnboundedSender<SemanticEvent>,
    event_receiver: Arc<Mutex<mpsc::UnboundedReceiver<SemanticEvent>>>,
    workflow_trigger_sender: mpsc::UnboundedSender<WorkflowTriggerEvent>,
    workflow_trigger_receiver: Arc<Mutex<mpsc::UnboundedReceiver<WorkflowTriggerEvent>>>,
    
    // Execution control
    execution_semaphore: Arc<Semaphore>,
    
    // Performance monitoring
    performance_metrics: Arc<RwLock<ReactiveAutomationMetrics>>,
}

/// Reactive system state
#[derive(Debug, Clone)]
pub struct ReactiveSystemState {
    pub current_state: SystemStateType,
    pub state_variables: HashMap<String, serde_json::Value>,
    pub last_state_change: SystemTime,
    pub state_history: VecDeque<StateTransition>,
    pub active_workflows: HashMap<Uuid, WorkflowStatus>,
}

/// System state types
#[derive(Debug, Clone, PartialEq)]
pub enum SystemStateType {
    Normal,
    HighLoad,
    Degraded,
    Emergency,
    Maintenance,
    Scaling,
}

/// State transition record
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from_state: SystemStateType,
    pub to_state: SystemStateType,
    pub timestamp: SystemTime,
    pub trigger_event: Option<SemanticEvent>,
    pub reason: String,
}

/// Workflow status tracking
#[derive(Debug, Clone)]
pub struct WorkflowStatus {
    pub workflow_id: Uuid,
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub current_step: Option<Uuid>,
    pub progress_percentage: f64,
    pub started_at: SystemTime,
    pub estimated_completion: Option<SystemTime>,
}

/// Execution status
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Compensating,
    Cancelled,
}

/// Workflow trigger event
#[derive(Debug, Clone)]
pub enum WorkflowTriggerEvent {
    PatternMatch(PatternMatch),
    AnalyticsAnomaly(String),
    StateChange(StateTransition),
    ExternalTrigger {
        source: String,
        data: HashMap<String, serde_json::Value>,
    },
    MLPrediction {
        model_id: String,
        prediction: serde_json::Value,
        confidence: f64,
    },
}

impl ReactiveAutomationFramework {
    /// Create a new reactive automation framework
    pub async fn new(config: ReactiveAutomationConfig) -> SemanticResult<Self> {
        // Initialize core engines
        let cep_config = ComplexEventProcessorConfig {
            max_active_patterns: config.max_active_automations,
            match_latency_target_ns: config.max_automation_latency_ms * 1_000_000,
            ..Default::default()
        };
        let cep_engine = Arc::new(ComplexEventProcessor::new(cep_config)?);
        
        let rule_config = AutomationRuleEngineConfig {
            max_concurrent_executions: config.max_concurrent_workflows,
            execution_timeout: config.automation_timeout,
            ..Default::default()
        };
        let rule_engine = Arc::new(AutomationRuleEngine::new(rule_config)?);
        
        let analytics_config = AnalyticsConfig {
            processing_latency_target_ns: config.max_automation_latency_ms * 1_000_000,
            ..Default::default()
        };
        let analytics_engine = Arc::new(EventAnalyticsEngine::new(analytics_config)?);
        
        let routing_config = EventRoutingConfig::default();
        let routing_engine = Arc::new(EventRoutingEngine::new(routing_config)?);
        
        // Initialize communication channels
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (workflow_trigger_sender, workflow_trigger_receiver) = mpsc::unbounded_channel();
        
        // Initialize execution control
        let execution_semaphore = Arc::new(Semaphore::new(config.max_concurrent_workflows));
        
        // Initialize state management
        let reactive_state = Arc::new(RwLock::new(ReactiveSystemState {
            current_state: SystemStateType::Normal,
            state_variables: HashMap::new(),
            last_state_change: SystemTime::now(),
            state_history: VecDeque::new(),
            active_workflows: HashMap::new(),
        }));
        
        Ok(Self {
            config,
            cep_engine,
            rule_engine,
            analytics_engine,
            routing_engine,
            coordination_engine: None,
            workflows: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(VecDeque::new())),
            reactive_state,
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            workflow_trigger_sender,
            workflow_trigger_receiver: Arc::new(Mutex::new(workflow_trigger_receiver)),
            execution_semaphore,
            performance_metrics: Arc::new(RwLock::new(ReactiveAutomationMetrics::default())),
        })
    }
    
    /// Start the reactive automation framework
    pub async fn start(&self) -> SemanticResult<()> {
        // Start core engines
        self.cep_engine.start().await?;
        self.rule_engine.start().await?;
        self.analytics_engine.start().await?;
        
        // Start event processing workers
        self.start_event_processing_workers().await?;
        
        // Start workflow execution workers
        self.start_workflow_execution_workers().await?;
        
        // Start performance monitoring
        self.start_performance_monitoring().await?;
        
        Ok(())
    }
    
    /// Process incoming event with reactive automation
    pub async fn process_event(&self, event: SemanticEvent) -> SemanticResult<ReactiveAutomationResult> {
        let start_time = Instant::now();
        
        // Route event through analytics engine
        let analytics_result = self.analytics_engine.process_event(event.clone()).await?;
        
        // Process through complex event processor
        let pattern_matches = self.cep_engine.process_event(event.clone()).await?;
        
        // Check for workflow triggers
        let triggered_workflows = self.check_workflow_triggers(&event, &pattern_matches, &analytics_result).await?;
        
        // Execute triggered workflows
        let mut workflow_results = Vec::new();
        for workflow_id in triggered_workflows {
            if let Some(workflow) = self.workflows.read().unwrap().get(&workflow_id).cloned() {
                let result = self.execute_workflow(workflow, Some(event.clone())).await?;
                workflow_results.push(result);
            }
        }
        
        // Update performance metrics
        let processing_latency = start_time.elapsed();
        self.update_performance_metrics(processing_latency, workflow_results.len()).await;
        
        Ok(ReactiveAutomationResult {
            processing_latency_ms: processing_latency.as_millis() as f64,
            pattern_matches,
            analytics_result,
            triggered_workflows: workflow_results,
            system_state: self.get_current_system_state().await,
        })
    }
    
    /// Add a reactive workflow
    pub async fn add_workflow(&self, workflow: ReactiveWorkflow) -> SemanticResult<()> {
        let workflow_id = workflow.workflow_id;
        
        // Validate workflow
        self.validate_workflow(&workflow)?;
        
        // Store workflow
        self.workflows.write().unwrap().insert(workflow_id, workflow);
        
        Ok(())
    }
    
    /// Remove a workflow
    pub async fn remove_workflow(&self, workflow_id: Uuid) -> SemanticResult<()> {
        self.workflows.write().unwrap().remove(&workflow_id);
        Ok(())
    }
    
    /// Execute a workflow manually
    pub async fn execute_workflow_manually(&self, workflow_id: Uuid, parameters: HashMap<String, serde_json::Value>) -> SemanticResult<WorkflowExecutionResult> {
        let workflow = self.workflows.read().unwrap().get(&workflow_id).cloned()
            .ok_or_else(|| SemanticError::not_found(format!("Workflow {} not found", workflow_id)))?;
        
        self.execute_workflow(workflow, None).await
    }
    
    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> ReactiveAutomationMetrics {
        self.performance_metrics.read().unwrap().clone()
    }
    
    /// Get current system state
    pub async fn get_current_system_state(&self) -> ReactiveSystemState {
        self.reactive_state.read().unwrap().clone()
    }
    
    /// Get active workflows
    pub async fn get_active_workflows(&self) -> Vec<WorkflowStatus> {
        self.reactive_state.read().unwrap().active_workflows.values().cloned().collect()
    }
    
    /// Get workflow execution history
    pub async fn get_execution_history(&self, workflow_id: Option<Uuid>, limit: usize) -> Vec<WorkflowExecutionResult> {
        let history = self.execution_history.read().unwrap();
        
        history.iter()
            .filter(|result| workflow_id.map_or(true, |id| result.workflow_id == id))
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Validate workflow definition
    fn validate_workflow(&self, workflow: &ReactiveWorkflow) -> SemanticResult<()> {
        // Validate workflow steps
        for step in &workflow.steps {
            if step.timeout > self.config.workflow_timeout {
                return Err(SemanticError::validation("Step timeout exceeds workflow timeout"));
            }
        }
        
        // Validate trigger patterns
        for trigger in &workflow.trigger_patterns {
            self.validate_workflow_trigger(trigger)?;
        }
        
        Ok(())
    }
    
    /// Validate workflow trigger
    fn validate_workflow_trigger(&self, trigger: &WorkflowTrigger) -> SemanticResult<()> {
        match trigger {
            WorkflowTrigger::EventPattern { confidence_threshold, .. } => {
                if *confidence_threshold < 0.0 || *confidence_threshold > 1.0 {
                    return Err(SemanticError::validation("Confidence threshold must be between 0.0 and 1.0"));
                }
            },
            WorkflowTrigger::Anomaly { severity_threshold, .. } => {
                if *severity_threshold < 0.0 {
                    return Err(SemanticError::validation("Severity threshold must be non-negative"));
                }
            },
            WorkflowTrigger::Schedule { cron_expression, .. } => {
                if cron_expression.is_empty() {
                    return Err(SemanticError::validation("Cron expression cannot be empty"));
                }
            },
            _ => {}
        }
        Ok(())
    }
    
    /// Check for workflow triggers
    async fn check_workflow_triggers(&self, event: &SemanticEvent, pattern_matches: &[PatternMatch], analytics_result: &AnalyticsResult) -> SemanticResult<Vec<Uuid>> {
        let mut triggered_workflows = Vec::new();
        let workflows = self.workflows.read().unwrap();
        
        for workflow in workflows.values() {
            if !workflow.enabled {
                continue;
            }
            
            for trigger in &workflow.trigger_patterns {
                if self.evaluate_trigger(trigger, event, pattern_matches, analytics_result).await? {
                    triggered_workflows.push(workflow.workflow_id);
                    break;
                }
            }
        }
        
        Ok(triggered_workflows)
    }
    
    /// Evaluate a workflow trigger
    async fn evaluate_trigger(&self, trigger: &WorkflowTrigger, event: &SemanticEvent, pattern_matches: &[PatternMatch], analytics_result: &AnalyticsResult) -> SemanticResult<bool> {
        match trigger {
            WorkflowTrigger::EventPattern { pattern_id, confidence_threshold } => {
                for pattern_match in pattern_matches {
                    if pattern_match.pattern_id == *pattern_id && pattern_match.confidence >= *confidence_threshold {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            WorkflowTrigger::Anomaly { anomaly_type, severity_threshold } => {
                for anomaly in &analytics_result.anomalies_detected {
                    if anomaly.anomaly_type.to_string() == *anomaly_type && anomaly.confidence >= *severity_threshold {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            WorkflowTrigger::StateChange { state_path, condition } => {
                // Check if the event represents a state change that matches the condition
                self.evaluate_state_condition(state_path, condition, event).await
            },
            _ => {
                // For other trigger types, return false for now
                Ok(false)
            }
        }
    }
    
    /// Evaluate state condition
    async fn evaluate_state_condition(&self, _state_path: &str, _condition: &StateCondition, _event: &SemanticEvent) -> SemanticResult<bool> {
        // Simplified implementation - would check actual state values
        Ok(false)
    }
    
    /// Execute a workflow
    async fn execute_workflow(&self, workflow: ReactiveWorkflow, trigger_event: Option<SemanticEvent>) -> SemanticResult<WorkflowExecutionResult> {
        let execution_id = Uuid::new_v4();
        let start_time = Instant::now();
        
        // Acquire execution permit
        let _permit = self.execution_semaphore.acquire().await
            .map_err(|e| SemanticError::internal(format!("Failed to acquire execution permit: {}", e)))?;
        
        // Create execution context
        let context = WorkflowExecutionContext {
            workflow_id: workflow.workflow_id,
            execution_id,
            tenant_id: workflow.tenant_id.clone(),
            trigger_event,
            trigger_pattern: None,
            variables: HashMap::new(),
            execution_start: start_time,
            current_step: None,
        };
        
        // Track active execution
        self.active_executions.write().unwrap().insert(execution_id, context.clone());
        
        // Update workflow status
        self.update_workflow_status(workflow.workflow_id, execution_id, ExecutionStatus::Running).await;
        
        let mut steps_completed = 0;
        let mut steps_failed = 0;
        let mut error_message = None;
        let mut output_data = HashMap::new();
        
        // Execute workflow steps
        for step in &workflow.steps {
            match self.execute_workflow_step(step, &context).await {
                Ok(step_result) => {
                    steps_completed += 1;
                    if let Some(data) = step_result.output_data.get("result") {
                        output_data.insert(format!("step_{}", step.step_id), data.clone());
                    }
                },
                Err(e) => {
                    steps_failed += 1;
                    error_message = Some(e.to_string());
                    
                    // If compensation is enabled and this step failed, execute compensation
                    if self.config.enable_rollback_compensation {
                        self.execute_compensation(&workflow, &context, step.step_id).await?;
                    }
                    
                    break;
                }
            }
        }
        
        let execution_duration = start_time.elapsed();
        let success = steps_failed == 0;
        
        // Update workflow status
        let final_status = if success { ExecutionStatus::Completed } else { ExecutionStatus::Failed };
        self.update_workflow_status(workflow.workflow_id, execution_id, final_status).await;
        
        // Remove from active executions
        self.active_executions.write().unwrap().remove(&execution_id);
        
        let result = WorkflowExecutionResult {
            workflow_id: workflow.workflow_id,
            execution_id,
            success,
            steps_completed,
            steps_failed,
            execution_duration,
            error_message,
            output_data,
            compensation_executed: !success && self.config.enable_rollback_compensation,
        };
        
        // Store in execution history
        let mut history = self.execution_history.write().unwrap();
        history.push_back(result.clone());
        if history.len() > 10000 {
            history.pop_front();
        }
        
        Ok(result)
    }
    
    /// Execute a workflow step
    async fn execute_workflow_step(&self, step: &WorkflowStep, context: &WorkflowExecutionContext) -> SemanticResult<StepExecutionResult> {
        let start_time = Instant::now();
        
        // Execute the step action
        let result = match &step.action {
            WorkflowAction::ExecuteRule { rule_id, parameters } => {
                self.rule_engine.execute_rule_manually(*rule_id, parameters.clone()).await?;
                StepExecutionResult {
                    success: true,
                    output_data: HashMap::new(),
                    execution_duration: start_time.elapsed(),
                    error_message: None,
                    compensation_required: false,
                }
            },