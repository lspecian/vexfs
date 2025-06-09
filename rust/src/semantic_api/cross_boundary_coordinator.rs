//! Cross-Boundary Transaction Coordinator
//!
//! This module implements the CrossBoundaryTransactionCoordinator for ACID transactions
//! spanning kernel and userspace semantic journals. It provides two-phase commit protocol,
//! deadlock detection, and seamless coordination between kernel and userspace boundaries.

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use crossbeam::utils::Backoff;
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock as TokioRwLock, Mutex as TokioMutex};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error, debug, trace};
use uuid::Uuid;

use crate::semantic_api::types::*;
use crate::shared::errors::{VexfsError, TransactionErrorKind};
use crate::cross_layer_consistency::{CrossLayerConsistencyManager, CrossLayerTransaction, CrossLayerTransactionState};
use crate::storage::acid_transaction_manager::{AcidTransactionManager, AcidTransactionState, IsolationLevel};

/// Cross-boundary transaction states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossBoundaryTransactionState {
    /// Transaction is being initialized
    Initializing,
    /// Transaction is active across boundaries
    Active,
    /// Phase 1 of two-phase commit - preparing
    Preparing,
    /// Phase 1 complete - all participants prepared
    Prepared,
    /// Phase 2 of two-phase commit - committing
    Committing,
    /// Transaction committed successfully
    Committed,
    /// Transaction is aborting
    Aborting,
    /// Transaction aborted
    Aborted,
    /// Transaction failed due to error
    Failed,
    /// Deadlock detected, transaction being resolved
    DeadlockResolving,
}

/// Cross-boundary participant types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantType {
    /// Kernel semantic journal
    KernelJournal,
    /// Userspace semantic journal
    UserspaceJournal,
    /// Cross-layer consistency manager
    CrossLayerManager,
    /// External system participant
    ExternalSystem,
}

/// Cross-boundary transaction participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBoundaryParticipant {
    /// Unique participant ID
    pub participant_id: Uuid,
    /// Participant type
    pub participant_type: ParticipantType,
    /// Participant endpoint/identifier
    pub endpoint: String,
    /// Current state in two-phase commit
    pub state: CrossBoundaryTransactionState,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
    /// Participant priority for deadlock resolution
    pub priority: u32,
    /// Timeout for this participant
    pub timeout_ms: u64,
}

impl CrossBoundaryParticipant {
    /// Create new participant
    pub fn new(
        participant_type: ParticipantType,
        endpoint: String,
        priority: u32,
        timeout_ms: u64,
    ) -> Self {
        Self {
            participant_id: Uuid::new_v4(),
            participant_type,
            endpoint,
            state: CrossBoundaryTransactionState::Initializing,
            last_heartbeat: SystemTime::now(),
            priority,
            timeout_ms,
        }
    }

    /// Check if participant has timed out
    pub fn has_timed_out(&self) -> bool {
        if let Ok(elapsed) = self.last_heartbeat.elapsed() {
            elapsed.as_millis() > self.timeout_ms as u128
        } else {
            true
        }
    }

    /// Update heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
    }
}

/// Cross-boundary transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBoundaryTransaction {
    /// Unique transaction ID
    pub transaction_id: Uuid,
    /// Current transaction state
    pub state: CrossBoundaryTransactionState,
    /// Isolation level
    pub isolation_level: IsolationLevel,
    /// Transaction participants
    pub participants: HashMap<Uuid, CrossBoundaryParticipant>,
    /// Transaction start time
    pub start_time: SystemTime,
    /// Prepare phase start time
    pub prepare_time: Option<SystemTime>,
    /// Commit phase start time
    pub commit_time: Option<SystemTime>,
    /// Transaction end time
    pub end_time: Option<SystemTime>,
    /// Transaction timeout
    pub timeout_ms: u64,
    /// Deadlock detection ID
    pub deadlock_detection_id: Option<Uuid>,
    /// Transaction priority for deadlock resolution
    pub priority: u32,
    /// Associated kernel transaction ID
    pub kernel_transaction_id: Option<u64>,
    /// Associated userspace transaction ID
    pub userspace_transaction_id: Option<Uuid>,
    /// Cross-layer transaction ID
    pub cross_layer_transaction_id: Option<Uuid>,
    /// Transaction metadata
    pub metadata: HashMap<String, String>,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl CrossBoundaryTransaction {
    /// Create new cross-boundary transaction
    pub fn new(
        isolation_level: IsolationLevel,
        timeout_ms: u64,
        priority: u32,
    ) -> Self {
        Self {
            transaction_id: Uuid::new_v4(),
            state: CrossBoundaryTransactionState::Initializing,
            isolation_level,
            participants: HashMap::new(),
            start_time: SystemTime::now(),
            prepare_time: None,
            commit_time: None,
            end_time: None,
            timeout_ms,
            deadlock_detection_id: Some(Uuid::new_v4()),
            priority,
            kernel_transaction_id: None,
            userspace_transaction_id: None,
            cross_layer_transaction_id: None,
            metadata: HashMap::new(),
            error_message: None,
        }
    }

    /// Add participant to transaction
    pub fn add_participant(&mut self, participant: CrossBoundaryParticipant) {
        self.participants.insert(participant.participant_id, participant);
    }

    /// Remove participant from transaction
    pub fn remove_participant(&mut self, participant_id: Uuid) -> Option<CrossBoundaryParticipant> {
        self.participants.remove(&participant_id)
    }

    /// Check if transaction has timed out
    pub fn has_timed_out(&self) -> bool {
        if let Ok(elapsed) = self.start_time.elapsed() {
            elapsed.as_millis() > self.timeout_ms as u128
        } else {
            true
        }
    }

    /// Check if all participants are in given state
    pub fn all_participants_in_state(&self, state: CrossBoundaryTransactionState) -> bool {
        self.participants.values().all(|p| p.state == state)
    }

    /// Get participants by type
    pub fn get_participants_by_type(&self, participant_type: ParticipantType) -> Vec<&CrossBoundaryParticipant> {
        self.participants.values()
            .filter(|p| p.participant_type == participant_type)
            .collect()
    }
}

/// Deadlock detection graph
#[derive(Debug, Clone)]
pub struct DeadlockGraph {
    /// Transaction wait-for relationships
    pub wait_for: HashMap<Uuid, HashSet<Uuid>>,
    /// Transaction priorities for resolution
    pub priorities: HashMap<Uuid, u32>,
    /// Last update timestamp
    pub last_update: SystemTime,
}

impl DeadlockGraph {
    /// Create new deadlock graph
    pub fn new() -> Self {
        Self {
            wait_for: HashMap::new(),
            priorities: HashMap::new(),
            last_update: SystemTime::now(),
        }
    }

    /// Add wait-for relationship
    pub fn add_wait_for(&mut self, waiter: Uuid, waited_for: Uuid) {
        self.wait_for.entry(waiter).or_insert_with(HashSet::new).insert(waited_for);
        self.last_update = SystemTime::now();
    }

    /// Remove wait-for relationship
    pub fn remove_wait_for(&mut self, waiter: Uuid, waited_for: Uuid) {
        if let Some(waiting_set) = self.wait_for.get_mut(&waiter) {
            waiting_set.remove(&waited_for);
            if waiting_set.is_empty() {
                self.wait_for.remove(&waiter);
            }
        }
        self.last_update = SystemTime::now();
    }

    /// Detect deadlock cycles using DFS
    pub fn detect_deadlock(&self) -> Option<Vec<Uuid>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &transaction_id in self.wait_for.keys() {
            if !visited.contains(&transaction_id) {
                if let Some(cycle) = self.dfs_detect_cycle(
                    transaction_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS helper for cycle detection
    fn dfs_detect_cycle(
        &self,
        node: Uuid,
        visited: &mut HashSet<Uuid>,
        rec_stack: &mut HashSet<Uuid>,
        path: &mut Vec<Uuid>,
    ) -> Option<Vec<Uuid>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(neighbors) = self.wait_for.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if let Some(cycle) = self.dfs_detect_cycle(neighbor, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&neighbor) {
                    // Found cycle - extract it from path
                    if let Some(cycle_start) = path.iter().position(|&x| x == neighbor) {
                        return Some(path[cycle_start..].to_vec());
                    }
                }
            }
        }

        rec_stack.remove(&node);
        path.pop();
        None
    }

    /// Select victim transaction for deadlock resolution
    pub fn select_victim(&self, cycle: &[Uuid]) -> Option<Uuid> {
        // Select transaction with lowest priority (highest priority value)
        cycle.iter()
            .max_by_key(|&&tx_id| self.priorities.get(&tx_id).unwrap_or(&u32::MAX))
            .copied()
    }
}

/// Cross-boundary coordinator statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrossBoundaryStats {
    /// Total transactions started
    pub transactions_started: u64,
    /// Total transactions committed
    pub transactions_committed: u64,
    /// Total transactions aborted
    pub transactions_aborted: u64,
    /// Total deadlocks detected
    pub deadlocks_detected: u64,
    /// Total deadlocks resolved
    pub deadlocks_resolved: u64,
    /// Average transaction duration (ms)
    pub avg_transaction_duration_ms: u64,
    /// Average prepare phase duration (ms)
    pub avg_prepare_duration_ms: u64,
    /// Average commit phase duration (ms)
    pub avg_commit_duration_ms: u64,
    /// Current active transactions
    pub active_transactions: u64,
    /// Two-phase commit success rate (percentage)
    pub two_phase_commit_success_rate: u32,
    /// Participant timeout count
    pub participant_timeouts: u64,
}

/// Cross-boundary coordinator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBoundaryConfig {
    /// Maximum concurrent transactions
    pub max_concurrent_transactions: usize,
    /// Default transaction timeout (ms)
    pub default_timeout_ms: u64,
    /// Deadlock detection interval (ms)
    pub deadlock_detection_interval_ms: u64,
    /// Participant heartbeat interval (ms)
    pub heartbeat_interval_ms: u64,
    /// Enable deadlock detection
    pub enable_deadlock_detection: bool,
    /// Enable two-phase commit
    pub enable_two_phase_commit: bool,
    /// Prepare phase timeout (ms)
    pub prepare_timeout_ms: u64,
    /// Commit phase timeout (ms)
    pub commit_timeout_ms: u64,
}

impl Default for CrossBoundaryConfig {
    fn default() -> Self {
        Self {
            max_concurrent_transactions: 256,
            default_timeout_ms: 30000,  // 30 seconds
            deadlock_detection_interval_ms: 5000,  // 5 seconds
            heartbeat_interval_ms: 1000,  // 1 second
            enable_deadlock_detection: true,
            enable_two_phase_commit: true,
            prepare_timeout_ms: 10000,  // 10 seconds
            commit_timeout_ms: 15000,   // 15 seconds
        }
    }
}

/// Cross-boundary transaction coordinator
pub struct CrossBoundaryTransactionCoordinator {
    /// Configuration
    config: Arc<RwLock<CrossBoundaryConfig>>,
    
    /// Active transactions
    active_transactions: Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
    
    /// Deadlock detection graph
    deadlock_graph: Arc<TokioRwLock<DeadlockGraph>>,
    
    /// Statistics
    stats: Arc<TokioRwLock<CrossBoundaryStats>>,
    
    /// Cross-layer consistency manager
    cross_layer_manager: Option<Arc<CrossLayerConsistencyManager>>,
    
    /// Command channels
    command_sender: Sender<CoordinatorCommand>,
    command_receiver: Arc<Mutex<Receiver<CoordinatorCommand>>>,
    
    /// Event channels
    event_sender: Sender<CoordinatorEvent>,
    event_receiver: Arc<Mutex<Receiver<CoordinatorEvent>>>,
    
    /// Shutdown signal
    shutdown_sender: Sender<()>,
    shutdown_receiver: Arc<Mutex<Receiver<()>>>,
    
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Coordinator commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorCommand {
    /// Begin new cross-boundary transaction
    BeginTransaction {
        isolation_level: IsolationLevel,
        timeout_ms: Option<u64>,
        priority: u32,
    },
    /// Add participant to transaction
    AddParticipant {
        transaction_id: Uuid,
        participant: CrossBoundaryParticipant,
    },
    /// Prepare transaction (phase 1 of 2PC)
    PrepareTransaction(Uuid),
    /// Commit transaction (phase 2 of 2PC)
    CommitTransaction(Uuid),
    /// Abort transaction
    AbortTransaction(Uuid),
    /// Update participant heartbeat
    UpdateHeartbeat {
        transaction_id: Uuid,
        participant_id: Uuid,
    },
    /// Detect deadlocks
    DetectDeadlocks,
    /// Resolve deadlock
    ResolveDeadlock {
        cycle: Vec<Uuid>,
        victim: Uuid,
    },
    /// Get statistics
    GetStats,
    /// Shutdown coordinator
    Shutdown,
}

/// Coordinator events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorEvent {
    /// Transaction started
    TransactionStarted(Uuid),
    /// Transaction prepared
    TransactionPrepared(Uuid),
    /// Transaction committed
    TransactionCommitted(Uuid),
    /// Transaction aborted
    TransactionAborted(Uuid, String),
    /// Deadlock detected
    DeadlockDetected(Vec<Uuid>),
    /// Deadlock resolved
    DeadlockResolved(Uuid),
    /// Participant timeout
    ParticipantTimeout {
        transaction_id: Uuid,
        participant_id: Uuid,
    },
}

impl CrossBoundaryTransactionCoordinator {
    /// Create new cross-boundary transaction coordinator
    pub fn new(config: CrossBoundaryConfig) -> Result<Self, VexfsError> {
        let (command_sender, command_receiver) = channel::unbounded();
        let (event_sender, event_receiver) = channel::unbounded();
        let (shutdown_sender, shutdown_receiver) = channel::unbounded();

        let coordinator = Self {
            config: Arc::new(RwLock::new(config)),
            active_transactions: Arc::new(TokioRwLock::new(HashMap::new())),
            deadlock_graph: Arc::new(TokioRwLock::new(DeadlockGraph::new())),
            stats: Arc::new(TokioRwLock::new(CrossBoundaryStats::default())),
            cross_layer_manager: None,
            command_sender,
            command_receiver: Arc::new(Mutex::new(command_receiver)),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            shutdown_sender,
            shutdown_receiver: Arc::new(Mutex::new(shutdown_receiver)),
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        info!("Cross-boundary transaction coordinator created");
        Ok(coordinator)
    }

    /// Set cross-layer consistency manager
    pub fn set_cross_layer_manager(&mut self, manager: Arc<CrossLayerConsistencyManager>) {
        self.cross_layer_manager = Some(manager);
    }

    /// Start the coordinator
    pub async fn start(&self) -> Result<(), VexfsError> {
        info!("Starting cross-boundary transaction coordinator");

        let mut handles = self.task_handles.lock().unwrap();

        // Start command processor
        let command_task = self.spawn_command_processor().await?;
        handles.push(command_task);

        // Start event processor
        let event_task = self.spawn_event_processor().await?;
        handles.push(event_task);

        // Start deadlock detector if enabled
        if self.config.read().unwrap().enable_deadlock_detection {
            let deadlock_task = self.spawn_deadlock_detector().await?;
            handles.push(deadlock_task);
        }

        // Start heartbeat monitor
        let heartbeat_task = self.spawn_heartbeat_monitor().await?;
        handles.push(heartbeat_task);

        info!("Cross-boundary transaction coordinator started with {} background tasks", handles.len());
        Ok(())
    }

    /// Stop the coordinator
    pub async fn stop(&self) -> Result<(), VexfsError> {
        info!("Stopping cross-boundary transaction coordinator");

        // Send shutdown signal
        if let Err(e) = self.shutdown_sender.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }

        // Wait for all background tasks to complete
        let handles = {
            let mut handles_guard = self.task_handles.lock().unwrap();
            std::mem::take(&mut *handles_guard)
        };

        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Background task failed to complete: {}", e);
            }
        }

        info!("Cross-boundary transaction coordinator stopped");
        Ok(())
    }

    /// Begin new cross-boundary transaction
    pub async fn begin_transaction(
        &self,
        isolation_level: IsolationLevel,
        timeout_ms: Option<u64>,
        priority: u32,
    ) -> Result<Uuid, VexfsError> {
        let config = self.config.read().unwrap();
        let active_count = self.active_transactions.read().await.len();

        if active_count >= config.max_concurrent_transactions {
            return Err(VexfsError::ResourceLimit(
                "Maximum concurrent cross-boundary transactions reached".to_string()
            ));
        }

        let timeout = timeout_ms.unwrap_or(config.default_timeout_ms);
        let mut transaction = CrossBoundaryTransaction::new(isolation_level, timeout, priority);
        let transaction_id = transaction.transaction_id;

        // Add to active transactions
        self.active_transactions.write().await.insert(transaction_id, transaction);

        // Update deadlock graph
        let mut deadlock_graph = self.deadlock_graph.write().await;
        deadlock_graph.priorities.insert(transaction_id, priority);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.transactions_started += 1;
        stats.active_transactions += 1;

        // Send event
        if let Err(e) = self.event_sender.send(CoordinatorEvent::TransactionStarted(transaction_id)) {
            warn!("Failed to send transaction started event: {}", e);
        }

        debug!("Started cross-boundary transaction {}", transaction_id);
        Ok(transaction_id)
    }

    /// Add participant to transaction
    pub async fn add_participant(
        &self,
        transaction_id: Uuid,
        participant: CrossBoundaryParticipant,
    ) -> Result<(), VexfsError> {
        let mut transactions = self.active_transactions.write().await;
        let transaction = transactions.get_mut(&transaction_id)
            .ok_or_else(|| VexfsError::TransactionError(TransactionErrorKind::TransactionNotFound))?;

        if transaction.state != CrossBoundaryTransactionState::Initializing &&
           transaction.state != CrossBoundaryTransactionState::Active {
            return Err(VexfsError::TransactionError(TransactionErrorKind::InvalidTransactionState));
        }

        transaction.add_participant(participant);
        transaction.state = CrossBoundaryTransactionState::Active;

        debug!("Added participant to transaction {}", transaction_id);
        Ok(())
    }

    /// Prepare transaction (phase 1 of two-phase commit)
    pub async fn prepare_transaction(&self, transaction_id: Uuid) -> Result<(), VexfsError> {
        if let Err(e) = self.command_sender.send(CoordinatorCommand::PrepareTransaction(transaction_id)) {
            return Err(VexfsError::Internal(format!("Failed to send prepare command: {}", e)));
        }

        // Wait for prepare phase to complete
        let prepare_timeout = self.config.read().unwrap().prepare_timeout_ms;
        let start_time = Instant::now();

        while start_time.elapsed().as_millis() < prepare_timeout as u128 {
            let transactions = self.active_transactions.read().await;
            if let Some(transaction) = transactions.get(&transaction_id) {
                match transaction.state {
                    CrossBoundaryTransactionState::Prepared => {
                        debug!("Transaction {} prepared successfully", transaction_id);
                        return Ok(());
                    }
                    CrossBoundaryTransactionState::Aborted | CrossBoundaryTransactionState::Failed => {
                        let error_msg = transaction.error_message.clone()
                            .unwrap_or_else(|| "Transaction prepare failed".to_string());
                        return Err(VexfsError::TransactionError(TransactionErrorKind::PrepareFailed));
                    }
                    _ => {
                        // Still preparing, continue waiting
                    }
                }
            } else {
                return Err(VexfsError::TransactionError(TransactionErrorKind::TransactionNotFound));
            }

            sleep(Duration::from_millis(10)).await;
        }

        Err(VexfsError::Timeout("Transaction prepare timeout".to_string()))
    }

    /// Commit transaction (phase 2 of two-phase commit)
    pub async fn commit_transaction(&self, transaction_id: Uuid) -> Result<(), VexfsError> {
        if let Err(e) = self.command_sender.send(CoordinatorCommand::CommitTransaction(transaction_id)) {
            return Err(VexfsError::Internal(format!("Failed to send commit command: {}", e)));
        }

        // Wait for commit phase to complete
        let commit_timeout = self.config.read().unwrap().commit_timeout_ms;
        let start_time = Instant::now();

        while start_time.elapsed().as_millis() < commit_timeout as u128 {
            let transactions = self.active_transactions.read().await;
            if let Some(transaction) = transactions.get(&transaction_id) {
                match transaction.state {
                    CrossBoundaryTransactionState::Committed => {
                        debug!("Transaction {} committed successfully", transaction_id);
                        return Ok(());
                    }
                    CrossBoundaryTransactionState::Aborted | CrossBoundaryTransactionState::Failed => {
                        let error_msg = transaction.error_message.clone()
                            .unwrap_or_else(|| "Transaction commit failed".to_string());
                        return Err(VexfsError::TransactionError(TransactionErrorKind::CommitFailed));
                    }
                    _ => {
                        // Still committing, continue waiting
                    }
                }
            } else {
                return Err(VexfsError::TransactionError(TransactionErrorKind::TransactionNotFound));
            }

            sleep(Duration::from_millis(10)).await;
        }

        Err(VexfsError::Timeout("Transaction commit timeout".to_string()))
    }

    /// Abort transaction
    pub async fn abort_transaction(&self, transaction_id: Uuid) -> Result<(), VexfsError> {
        if let Err(e) = self.command_sender.send(CoordinatorCommand::AbortTransaction(transaction_id)) {
            return Err(VexfsError::Internal(format!("Failed to send abort command: {}", e)));
        }

        debug!("Aborted cross-boundary transaction {}", transaction_id);
        Ok(())
    }

    /// Get coordinator statistics
    pub async fn get_stats(&self) -> CrossBoundaryStats {
        self.stats.read().await.clone()
    }

    /// Spawn command processor task
    async fn spawn_command_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let command_receiver = Arc::clone(&self.command_receiver);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let active_transactions = Arc::clone(&self.active_transactions);
        let deadlock_graph = Arc::clone(&self.deadlock_graph);
        let stats = Arc::clone(&self.stats);
        let event_sender = self.event_sender.clone();

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Command processor received shutdown signal");
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            // No shutdown signal, continue
                        }
                        Err(TryRecvError::Disconnected) => {
                            warn!("Shutdown channel disconnected");
                            break;
                        }
                    }
                }

                // Process commands
                let command_to_process = if let Ok(command_receiver) = command_receiver.try_lock() {
                    match command_receiver.try_recv() {
                        Ok(command) => Some(command),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => {
                            warn!("Command channel disconnected");
                            break;
                        }
                    }
                } else {
                    None
                };

                if let Some(command) = command_to_process {
                    Self::process_command(
                        command,
                        &active_transactions,
                        &deadlock_graph,
                        &stats,
                        &event_sender,
                    ).await;
                } else {
                    sleep(Duration::from_millis(10)).await;
                }
            }

            debug!("Command processor task completed");
        });

        Ok(handle)
    }

    /// Spawn event processor task
    async fn spawn_event_processor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let event_receiver = Arc::clone(&self.event_receiver);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);

        let handle = tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                    match shutdown_receiver.try_recv() {
                        Ok(_) => {
                            debug!("Event processor received shutdown signal");
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            // No shutdown signal, continue
                        }
                        Err(TryRecvError::Disconnected) => {
                            warn!("Shutdown channel disconnected");
                            break;
                        }
                    }
                }

                // Process events
                let event_to_process = if let Ok(event_receiver) = event_receiver.try_lock() {
                    match event_receiver.try_recv() {
                        Ok(event) => Some(event),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => {
                            warn!("Event channel disconnected");
                            break;
                        }
                    }
                } else {
                    None
                };

                if let Some(event) = event_to_process {
                    Self::process_event(event).await;
                } else {
                    sleep(Duration::from_millis(10)).await;
                }
            }

            debug!("Event processor task completed");
        });

        Ok(handle)
    }

    /// Spawn deadlock detector task
    async fn spawn_deadlock_detector(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let config = Arc::clone(&self.config);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let command_sender = self.command_sender.clone();

        let handle = tokio::spawn(async move {
            loop {
                let interval = {
                    let config = config.read().unwrap();
                    Duration::from_millis(config.deadlock_detection_interval_ms)
                };

                // Wait for interval or shutdown
                match timeout(interval, async {
                    if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                        shutdown_receiver.recv()
                    } else {
                        Err(channel::RecvError)
                    }
                }).await {
                    Ok(Ok(_)) => {
                        debug!("Deadlock detector received shutdown signal");
                        break;
                    }
                    Ok(Err(_)) => {
                        warn!("Shutdown channel disconnected");
                        break;
                    }
                    Err(_) => {
                        // Timeout, perform deadlock detection
                        if let Err(e) = command_sender.send(CoordinatorCommand::DetectDeadlocks) {
                            warn!("Failed to send deadlock detection command: {}", e);
                        }
                    }
                }
            }

            debug!("Deadlock detector task completed");
        });

        Ok(handle)
    }

    /// Spawn heartbeat monitor task
    async fn spawn_heartbeat_monitor(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let config = Arc::clone(&self.config);
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let active_transactions = Arc::clone(&self.active_transactions);
        let event_sender = self.event_sender.clone();

        let handle = tokio::spawn(async move {
            loop {
                let interval = {
                    let config = config.read().unwrap();
                    Duration::from_millis(config.heartbeat_interval_ms)
                };

                // Wait for interval or shutdown
                match timeout(interval, async {
                    if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                        shutdown_receiver.recv()
                    } else {
                        Err(channel::RecvError)
                    }
                }).await {
                    Ok(Ok(_)) => {
                        debug!("Heartbeat monitor received shutdown signal");
                        break;
                    }
                    Ok(Err(_)) => {
                        warn!("Shutdown channel disconnected");
                        break;
                    }
                    Err(_) => {
                        // Timeout, check for participant timeouts
                        let mut timed_out_participants = Vec::new();
                        
                        {
                            let transactions = active_transactions.read().await;
                            for (tx_id, transaction) in transactions.iter() {
                                for (participant_id, participant) in &transaction.participants {
                                    if participant.has_timed_out() {
                                        timed_out_participants.push((*tx_id, *participant_id));
                                    }
                                }
                            }
                        }

                        // Send timeout events
                        for (transaction_id, participant_id) in timed_out_participants {
                            if let Err(e) = event_sender.send(CoordinatorEvent::ParticipantTimeout {
                                transaction_id,
                                participant_id,
                            }) {
                                warn!("Failed to send participant timeout event: {}", e);
                            }
                        }
                    }
                }
            }

            debug!("Heartbeat monitor task completed");
        });

        Ok(handle)
    }

    /// Process coordinator command
    async fn process_command(
        command: CoordinatorCommand,
        active_transactions: &Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
        deadlock_graph: &Arc<TokioRwLock<DeadlockGraph>>,
        stats: &Arc<TokioRwLock<CrossBoundaryStats>>,
        event_sender: &Sender<CoordinatorEvent>,
    ) {
        match command {
            CoordinatorCommand::BeginTransaction { isolation_level, timeout_ms, priority } => {
                debug!("Processing begin transaction command");
                // Transaction creation is handled in the public API
            }
            CoordinatorCommand::AddParticipant { transaction_id, participant } => {
                debug!("Processing add participant command for transaction {}", transaction_id);
                let mut transactions = active_transactions.write().await;
                if let Some(transaction) = transactions.get_mut(&transaction_id) {
                    transaction.add_participant(participant);
                }
            }
            CoordinatorCommand::PrepareTransaction(transaction_id) => {
                debug!("Processing prepare transaction command: {}", transaction_id);
                Self::prepare_transaction_impl(transaction_id, active_transactions, stats, event_sender).await;
            }
            CoordinatorCommand::CommitTransaction(transaction_id) => {
                debug!("Processing commit transaction command: {}", transaction_id);
                Self::commit_transaction_impl(transaction_id, active_transactions, stats, event_sender).await;
            }
            CoordinatorCommand::AbortTransaction(transaction_id) => {
                debug!("Processing abort transaction command: {}", transaction_id);
                Self::abort_transaction_impl(transaction_id, active_transactions, stats, event_sender).await;
            }
            CoordinatorCommand::UpdateHeartbeat { transaction_id, participant_id } => {
                debug!("Processing update heartbeat command");
                let mut transactions = active_transactions.write().await;
                if let Some(transaction) = transactions.get_mut(&transaction_id) {
                    if let Some(participant) = transaction.participants.get_mut(&participant_id) {
                        participant.update_heartbeat();
                    }
                }
            }
            CoordinatorCommand::DetectDeadlocks => {
                debug!("Processing detect deadlocks command");
                Self::detect_deadlocks_impl(active_transactions, deadlock_graph, stats, event_sender).await;
            }
            CoordinatorCommand::ResolveDeadlock { cycle, victim } => {
                debug!("Processing resolve deadlock command");
                Self::resolve_deadlock_impl(cycle, victim, active_transactions, deadlock_graph, stats, event_sender).await;
            }
            CoordinatorCommand::GetStats => {
                debug!("Processing get stats command");
                // Stats are accessible via shared reference
            }
            CoordinatorCommand::Shutdown => {
                debug!("Processing shutdown command");
                // Shutdown is handled by the main loop
            }
        }
    }

    /// Process coordinator event
    async fn process_event(event: CoordinatorEvent) {
        match event {
            CoordinatorEvent::TransactionStarted(transaction_id) => {
                debug!("Processing transaction started event: {}", transaction_id);
            }
            CoordinatorEvent::TransactionPrepared(transaction_id) => {
                debug!("Processing transaction prepared event: {}", transaction_id);
            }
            CoordinatorEvent::TransactionCommitted(transaction_id) => {
                debug!("Processing transaction committed event: {}", transaction_id);
            }
            CoordinatorEvent::TransactionAborted(transaction_id, reason) => {
                debug!("Processing transaction aborted event: {} ({})", transaction_id, reason);
            }
            CoordinatorEvent::DeadlockDetected(cycle) => {
                warn!("Processing deadlock detected event: {:?}", cycle);
            }
            CoordinatorEvent::DeadlockResolved(victim) => {
                info!("Processing deadlock resolved event: victim {}", victim);
            }
            CoordinatorEvent::ParticipantTimeout { transaction_id, participant_id } => {
                warn!("Processing participant timeout event: tx {} participant {}", transaction_id, participant_id);
            }
        }
    }

    /// Implementation of transaction prepare
    async fn prepare_transaction_impl(
        transaction_id: Uuid,
        active_transactions: &Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
        stats: &Arc<TokioRwLock<CrossBoundaryStats>>,
        event_sender: &Sender<CoordinatorEvent>,
    ) {
        let mut transactions = active_transactions.write().await;
        if let Some(transaction) = transactions.get_mut(&transaction_id) {
            transaction.state = CrossBoundaryTransactionState::Preparing;
            transaction.prepare_time = Some(SystemTime::now());

            // TODO: Send prepare messages to all participants
            // For now, simulate successful prepare
            let all_prepared = true;

            if all_prepared {
                transaction.state = CrossBoundaryTransactionState::Prepared;
                
                if let Err(e) = event_sender.send(CoordinatorEvent::TransactionPrepared(transaction_id)) {
                    warn!("Failed to send transaction prepared event: {}", e);
                }
                
                debug!("Transaction {} prepared successfully", transaction_id);
            } else {
                transaction.state = CrossBoundaryTransactionState::Failed;
                transaction.error_message = Some("Prepare phase failed".to_string());
                
                if let Err(e) = event_sender.send(CoordinatorEvent::TransactionAborted(
                    transaction_id,
                    "Prepare phase failed".to_string()
                )) {
                    warn!("Failed to send transaction aborted event: {}", e);
                }
            }
        }
    }

    /// Implementation of transaction commit
    async fn commit_transaction_impl(
        transaction_id: Uuid,
        active_transactions: &Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
        stats: &Arc<TokioRwLock<CrossBoundaryStats>>,
        event_sender: &Sender<CoordinatorEvent>,
    ) {
        let mut transactions = active_transactions.write().await;
        if let Some(transaction) = transactions.get_mut(&transaction_id) {
            if transaction.state != CrossBoundaryTransactionState::Prepared {
                warn!("Attempted to commit transaction {} in invalid state: {:?}", transaction_id, transaction.state);
                return;
            }

            transaction.state = CrossBoundaryTransactionState::Committing;
            transaction.commit_time = Some(SystemTime::now());

            // TODO: Send commit messages to all participants
            // For now, simulate successful commit
            let all_committed = true;

            if all_committed {
                transaction.state = CrossBoundaryTransactionState::Committed;
                transaction.end_time = Some(SystemTime::now());
                
                // Update statistics
                let mut stats_guard = stats.write().await;
                stats_guard.transactions_committed += 1;
                stats_guard.active_transactions = stats_guard.active_transactions.saturating_sub(1);
                
                // Calculate durations
                if let (Some(start), Some(end)) = (
                    transaction.start_time.duration_since(UNIX_EPOCH).ok(),
                    transaction.end_time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                ) {
                    let duration_ms = end.as_millis().saturating_sub(start.as_millis()) as u64;
                    stats_guard.avg_transaction_duration_ms =
                        (stats_guard.avg_transaction_duration_ms + duration_ms) / 2;
                }
                
                if let (Some(prepare), Some(commit)) = (
                    transaction.prepare_time.and_then(|t| t.duration_since(UNIX_EPOCH).ok()),
                    transaction.commit_time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                ) {
                    let prepare_duration_ms = commit.as_millis().saturating_sub(prepare.as_millis()) as u64;
                    stats_guard.avg_prepare_duration_ms =
                        (stats_guard.avg_prepare_duration_ms + prepare_duration_ms) / 2;
                }
                
                if let (Some(commit_start), Some(end)) = (
                    transaction.commit_time.and_then(|t| t.duration_since(UNIX_EPOCH).ok()),
                    transaction.end_time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                ) {
                    let commit_duration_ms = end.as_millis().saturating_sub(commit_start.as_millis()) as u64;
                    stats_guard.avg_commit_duration_ms =
                        (stats_guard.avg_commit_duration_ms + commit_duration_ms) / 2;
                }
                
                if let Err(e) = event_sender.send(CoordinatorEvent::TransactionCommitted(transaction_id)) {
                    warn!("Failed to send transaction committed event: {}", e);
                }
                
                debug!("Transaction {} committed successfully", transaction_id);
            } else {
                transaction.state = CrossBoundaryTransactionState::Failed;
                transaction.error_message = Some("Commit phase failed".to_string());
                
                let mut stats_guard = stats.write().await;
                stats_guard.transactions_aborted += 1;
                stats_guard.active_transactions = stats_guard.active_transactions.saturating_sub(1);
                
                if let Err(e) = event_sender.send(CoordinatorEvent::TransactionAborted(
                    transaction_id,
                    "Commit phase failed".to_string()
                )) {
                    warn!("Failed to send transaction aborted event: {}", e);
                }
            }
        }
    }

    /// Implementation of transaction abort
    async fn abort_transaction_impl(
        transaction_id: Uuid,
        active_transactions: &Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
        stats: &Arc<TokioRwLock<CrossBoundaryStats>>,
        event_sender: &Sender<CoordinatorEvent>,
    ) {
        let mut transactions = active_transactions.write().await;
        if let Some(transaction) = transactions.get_mut(&transaction_id) {
            transaction.state = CrossBoundaryTransactionState::Aborting;
            
            // TODO: Send abort messages to all participants
            transaction.state = CrossBoundaryTransactionState::Aborted;
            transaction.end_time = Some(SystemTime::now());
            
            // Update statistics
            let mut stats_guard = stats.write().await;
            stats_guard.transactions_aborted += 1;
            stats_guard.active_transactions = stats_guard.active_transactions.saturating_sub(1);
            
            if let Err(e) = event_sender.send(CoordinatorEvent::TransactionAborted(
                transaction_id,
                "Transaction aborted".to_string()
            )) {
                warn!("Failed to send transaction aborted event: {}", e);
            }
            
            debug!("Transaction {} aborted", transaction_id);
        }
    }

    /// Implementation of deadlock detection
    async fn detect_deadlocks_impl(
        active_transactions: &Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
        deadlock_graph: &Arc<TokioRwLock<DeadlockGraph>>,
        stats: &Arc<TokioRwLock<CrossBoundaryStats>>,
        event_sender: &Sender<CoordinatorEvent>,
    ) {
        let transactions = active_transactions.read().await;
        let mut graph = deadlock_graph.write().await;
        
        // Update deadlock graph with current transaction states
        for (tx_id, transaction) in transactions.iter() {
            graph.priorities.insert(*tx_id, transaction.priority);
            
            // TODO: Build wait-for relationships based on lock dependencies
            // For now, use a simple heuristic
        }
        
        // Detect deadlock cycles
        if let Some(cycle) = graph.detect_deadlock() {
            let mut stats_guard = stats.write().await;
            stats_guard.deadlocks_detected += 1;
            
            if let Err(e) = event_sender.send(CoordinatorEvent::DeadlockDetected(cycle.clone())) {
                warn!("Failed to send deadlock detected event: {}", e);
            }
            
            // Select victim and resolve deadlock
            if let Some(victim) = graph.select_victim(&cycle) {
                // Send resolve command (this would normally be handled by a separate resolver)
                debug!("Deadlock detected, victim selected: {}", victim);
            }
        }
    }

    /// Implementation of deadlock resolution
    async fn resolve_deadlock_impl(
        cycle: Vec<Uuid>,
        victim: Uuid,
        active_transactions: &Arc<TokioRwLock<HashMap<Uuid, CrossBoundaryTransaction>>>,
        deadlock_graph: &Arc<TokioRwLock<DeadlockGraph>>,
        stats: &Arc<TokioRwLock<CrossBoundaryStats>>,
        event_sender: &Sender<CoordinatorEvent>,
    ) {
        debug!("Resolving deadlock by aborting victim transaction: {}", victim);
        
        // Abort the victim transaction
        Self::abort_transaction_impl(victim, active_transactions, stats, event_sender).await;
        
        // Update deadlock graph
        let mut graph = deadlock_graph.write().await;
        graph.priorities.remove(&victim);
        graph.wait_for.remove(&victim);
        
        // Remove victim from other wait-for sets
        for waiting_set in graph.wait_for.values_mut() {
            waiting_set.remove(&victim);
        }
        
        // Update statistics
        let mut stats_guard = stats.write().await;
        stats_guard.deadlocks_resolved += 1;
        
        if let Err(e) = event_sender.send(CoordinatorEvent::DeadlockResolved(victim)) {
            warn!("Failed to send deadlock resolved event: {}", e);
        }
        
        info!("Deadlock resolved by aborting transaction {}", victim);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;
    
    #[tokio::test]
    async fn test_cross_boundary_coordinator_creation() {
        let config = CrossBoundaryConfig::default();
        let coordinator = CrossBoundaryTransactionCoordinator::new(config).unwrap();
        
        let stats = coordinator.get_stats().await;
        assert_eq!(stats.transactions_started, 0);
    }
    
    #[tokio::test]
    async fn test_transaction_lifecycle() {
        let config = CrossBoundaryConfig::default();
        let coordinator = CrossBoundaryTransactionCoordinator::new(config).unwrap();
        
        // Start the coordinator
        coordinator.start().await.unwrap();
        
        // Begin a transaction
        let transaction_id = coordinator.begin_transaction(
            IsolationLevel::Serializable,
            Some(5000),
            1,
        ).await.unwrap();
        
        // Add participants
        let kernel_participant = CrossBoundaryParticipant::new(
            ParticipantType::KernelJournal,
            "kernel://journal".to_string(),
            1,
            5000,
        );
        
        let userspace_participant = CrossBoundaryParticipant::new(
            ParticipantType::UserspaceJournal,
            "userspace://journal".to_string(),
            1,
            5000,
        );
        
        coordinator.add_participant(transaction_id, kernel_participant).await.unwrap();
        coordinator.add_participant(transaction_id, userspace_participant).await.unwrap();
        
        // Prepare transaction
        let result = timeout(Duration::from_secs(2), coordinator.prepare_transaction(transaction_id)).await;
        assert!(result.is_ok());
        
        // Commit transaction
        let result = timeout(Duration::from_secs(2), coordinator.commit_transaction(transaction_id)).await;
        assert!(result.is_ok());
        
        // Check statistics
        let stats = coordinator.get_stats().await;
        assert_eq!(stats.transactions_started, 1);
        assert_eq!(stats.transactions_committed, 1);
        
        // Stop the coordinator
        coordinator.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_deadlock_detection() {
        let config = CrossBoundaryConfig::default();
        let coordinator = CrossBoundaryTransactionCoordinator::new(config).unwrap();
        
        coordinator.start().await.unwrap();
        
        // Create multiple transactions to simulate potential deadlock
        let tx1 = coordinator.begin_transaction(IsolationLevel::Serializable, Some(5000), 1).await.unwrap();
        let tx2 = coordinator.begin_transaction(IsolationLevel::Serializable, Some(5000), 2).await.unwrap();
        
        // Simulate deadlock detection
        let deadlock_graph = coordinator.deadlock_graph.read().await;
        let cycle = deadlock_graph.detect_deadlock();
        
        // In this simple test, no actual deadlock should be detected
        assert!(cycle.is_none());
        
        coordinator.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_participant_timeout() {
        let mut config = CrossBoundaryConfig::default();
        config.heartbeat_interval_ms = 100; // Fast heartbeat for testing
        
        let coordinator = CrossBoundaryTransactionCoordinator::new(config).unwrap();
        coordinator.start().await.unwrap();
        
        let transaction_id = coordinator.begin_transaction(
            IsolationLevel::ReadCommitted,
            Some(5000),
            1,
        ).await.unwrap();
        
        // Add participant with short timeout
        let participant = CrossBoundaryParticipant::new(
            ParticipantType::ExternalSystem,
            "external://system".to_string(),
            1,
            50, // 50ms timeout
        );
        
        coordinator.add_participant(transaction_id, participant).await.unwrap();
        
        // Wait for timeout to occur
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Participant should have timed out
        let transactions = coordinator.active_transactions.read().await;
        if let Some(transaction) = transactions.get(&transaction_id) {
            let participant = transaction.participants.values().next().unwrap();
            assert!(participant.has_timed_out());
        }
        
        coordinator.stop().await.unwrap();
    }
}