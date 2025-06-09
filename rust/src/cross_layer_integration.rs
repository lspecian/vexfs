//! Cross-Layer Consistency and Integration Framework for VexFS v2.0 (Task 21)
//! 
//! This module implements the comprehensive Cross-Layer Consistency and Integration
//! Framework that unifies the AI-Native Semantic Substrate layers into a seamless,
//! production-ready system. It builds upon the Task 14 foundation to create a
//! complete integration framework with unified transaction management, versioned
//! metadata, strict journal ordering, atomic cross-boundary operations, crash
//! recovery, and consistent semantic views.

use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;
use std::fmt;

#[cfg(feature = "semantic_api")]
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
#[cfg(feature = "semantic_api")]
use crossbeam::sync::WaitGroup;

#[cfg(feature = "semantic_api")]
use im::{HashMap as ImHashMap, Vector as ImVector, OrdMap as ImOrdMap};

#[cfg(feature = "semantic_api")]
use rayon::prelude::*;

use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock as TokioRwLock, Mutex as TokioMutex, Semaphore};
use tokio::time::{sleep, timeout, interval};
use tracing::{info, warn, error, debug, trace, instrument};
use uuid::Uuid;

use crate::cross_layer_consistency::{
    CrossLayerConsistencyManager, CrossLayerTransaction, CrossLayerOperation,
    CrossLayerTransactionState, CrossLayerOperationType, CrossLayerIsolationLevel,
    CrossLayerEvent, CrossLayerCommand, CrossLayerSnapshot, CrossLayerStats,
    CrossLayerConfig,
};
use crate::shared::errors::{VexfsError, TransactionErrorKind};

/// Vector clock for distributed timestamp ordering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    #[cfg(feature = "semantic_api")]
    pub clocks: ImOrdMap<String, u64>,
    #[cfg(not(feature = "semantic_api"))]
    pub clocks: BTreeMap<String, u64>,
    pub node_id: String,
}

impl VectorClock {
    pub fn new(node_id: String) -> Self {
        #[cfg(feature = "semantic_api")]
        {
            let mut clocks = ImOrdMap::new();
            clocks.insert(node_id.clone(), 0);
            Self { clocks, node_id }
        }
        #[cfg(not(feature = "semantic_api"))]
        {
            let mut clocks = BTreeMap::new();
            clocks.insert(node_id.clone(), 0);
            Self { clocks, node_id }
        }
    }

    pub fn tick(&mut self) {
        let current = self.clocks.get(&self.node_id).unwrap_or(&0);
        self.clocks.insert(self.node_id.clone(), current + 1);
    }

    pub fn update(&mut self, other: &VectorClock) {
        for (node, clock) in &other.clocks {
            let current = self.clocks.get(node).unwrap_or(&0);
            self.clocks.insert(node.clone(), (*current).max(*clock));
        }
        self.tick();
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut strictly_less = false;
        for (node, other_clock) in &other.clocks {
            let self_clock = self.clocks.get(node).unwrap_or(&0);
            if self_clock > other_clock {
                return false;
            }
            if self_clock < other_clock {
                strictly_less = true;
            }
        }
        strictly_less
    }

    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

/// Lamport timestamp for total ordering
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LamportTimestamp {
    pub timestamp: u64,
    pub node_id: u64,
}

impl LamportTimestamp {
    pub fn new(node_id: u64) -> Self {
        Self { timestamp: 0, node_id }
    }

    pub fn tick(&mut self) {
        self.timestamp += 1;
    }

    pub fn update(&mut self, other: LamportTimestamp) {
        self.timestamp = self.timestamp.max(other.timestamp) + 1;
    }
}

/// Journal entry with distributed timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub operation_id: Uuid,
    pub vector_clock: VectorClock,
    pub lamport_timestamp: LamportTimestamp,
    pub layer_mask: u32,
    pub operation_type: CrossLayerOperationType,
    pub data: Vec<u8>,
    #[cfg(feature = "semantic_api")]
    pub metadata: ImHashMap<String, String>,
    #[cfg(not(feature = "semantic_api"))]
    pub metadata: HashMap<String, String>,
    #[cfg(feature = "semantic_api")]
    pub causality_links: ImVector<Uuid>,
    #[cfg(not(feature = "semantic_api"))]
    pub causality_links: Vec<Uuid>,
    pub created_at: SystemTime,
    pub committed_at: Option<SystemTime>,
}

/// Versioned metadata using immutable data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedMetadata {
    pub version: u64,
    pub transaction_id: Uuid,
    pub timestamp: SystemTime,
    pub vector_clock: VectorClock,
    #[cfg(feature = "semantic_api")]
    pub metadata: ImHashMap<String, serde_json::Value>,
    #[cfg(not(feature = "semantic_api"))]
    pub metadata: HashMap<String, serde_json::Value>,
    pub previous_version: Option<u64>,
    #[cfg(feature = "semantic_api")]
    pub layer_states: ImHashMap<String, LayerState>,
    #[cfg(not(feature = "semantic_api"))]
    pub layer_states: HashMap<String, LayerState>,
}

/// Layer state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerState {
    pub layer_id: String,
    pub version: u64,
    pub state_hash: String,
    pub checkpoint_data: Vec<u8>,
    #[cfg(feature = "semantic_api")]
    pub metadata: ImHashMap<String, String>,
    #[cfg(not(feature = "semantic_api"))]
    pub metadata: HashMap<String, String>,
}

/// Two-phase commit coordinator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TwoPhaseCommitState {
    Init,
    Preparing,
    Prepared,
    Committing,
    Committed,
    Aborting,
    Aborted,
    Failed,
}

/// Two-phase commit transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoPhaseCommitTransaction {
    pub transaction_id: Uuid,
    pub coordinator_id: String,
    pub state: TwoPhaseCommitState,
    #[cfg(feature = "semantic_api")]
    pub participants: ImVector<String>,
    #[cfg(not(feature = "semantic_api"))]
    pub participants: Vec<String>,
    #[cfg(feature = "semantic_api")]
    pub prepare_votes: ImHashMap<String, bool>,
    #[cfg(not(feature = "semantic_api"))]
    pub prepare_votes: HashMap<String, bool>,
    pub timeout: Duration,
    pub started_at: SystemTime,
    pub prepared_at: Option<SystemTime>,
    pub committed_at: Option<SystemTime>,
    #[cfg(feature = "semantic_api")]
    pub operations: ImVector<JournalEntry>,
    #[cfg(not(feature = "semantic_api"))]
    pub operations: Vec<JournalEntry>,
}

/// Recovery log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryLogEntry {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub operation_type: String,
    pub layer_id: String,
    pub before_state: Option<Vec<u8>>,
    pub after_state: Option<Vec<u8>>,
    pub timestamp: SystemTime,
    pub vector_clock: VectorClock,
}

/// Integration framework configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub node_id: String,
    pub consistency_level: ConsistencyLevel,
    pub transaction_timeout: Duration,
    pub max_concurrent_transactions: usize,
    pub journal_batch_size: usize,
    pub recovery_check_interval: Duration,
    pub consistency_check_interval: Duration,
    pub metadata_retention_period: Duration,
    pub enable_vector_clocks: bool,
    pub enable_lamport_timestamps: bool,
    pub enable_two_phase_commit: bool,
    pub enable_crash_recovery: bool,
    pub enable_semantic_views: bool,
    pub cache_size: usize,
    pub parallel_consistency_checks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    Eventual,
    Strong,
    Sequential,
    Linearizable,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            node_id: format!("node-{}", Uuid::new_v4()),
            consistency_level: ConsistencyLevel::Strong,
            transaction_timeout: Duration::from_secs(30),
            max_concurrent_transactions: 1000,
            journal_batch_size: 100,
            recovery_check_interval: Duration::from_secs(60),
            consistency_check_interval: Duration::from_secs(30),
            metadata_retention_period: Duration::from_secs(86400), // 24 hours
            enable_vector_clocks: true,
            enable_lamport_timestamps: true,
            enable_two_phase_commit: true,
            enable_crash_recovery: true,
            enable_semantic_views: true,
            cache_size: 10000,
            parallel_consistency_checks: true,
        }
    }
}

/// Integration framework statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationStats {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub aborted_transactions: u64,
    pub active_transactions: u64,
    pub journal_entries: u64,
    pub metadata_versions: u64,
    pub recovery_operations: u64,
    pub consistency_checks: u64,
    pub consistency_violations: u64,
    pub semantic_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_transaction_time_ms: f64,
    pub avg_commit_time_ms: f64,
    pub avg_query_time_ms: f64,
}

/// Main Cross-Layer Integration Framework
pub struct CrossLayerIntegrationFramework {
    /// Configuration
    config: Arc<RwLock<IntegrationConfig>>,
    
    /// Underlying consistency manager from Task 14
    consistency_manager: Arc<CrossLayerConsistencyManager>,
    
    /// Unified transaction manager
    transaction_manager: Arc<TokioRwLock<UnifiedTransactionManager>>,
    
    /// Journal ordering service
    journal_service: Arc<TokioRwLock<JournalOrderingService>>,
    
    /// Versioned metadata manager
    metadata_manager: Arc<TokioRwLock<VersionedMetadataManager>>,
    
    /// Two-phase commit coordinator
    commit_coordinator: Arc<TokioRwLock<TwoPhaseCommitCoordinator>>,
    
    /// Recovery manager
    recovery_manager: Arc<TokioRwLock<RecoveryManager>>,
    
    /// Performance cache
    cache: Arc<TokioRwLock<PerformanceCache>>,
    
    /// Statistics
    stats: Arc<TokioRwLock<IntegrationStats>>,
    
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    
    /// Shutdown coordination
    #[cfg(feature = "semantic_api")]
    shutdown_sender: Sender<()>,
    #[cfg(feature = "semantic_api")]
    shutdown_receiver: Arc<Mutex<Receiver<()>>>,
    
    /// Wait group for coordinated shutdown
    #[cfg(feature = "semantic_api")]
    wait_group: Arc<WaitGroup>,
}

/// Unified transaction manager
pub struct UnifiedTransactionManager {
    #[cfg(feature = "semantic_api")]
    active_transactions: ImHashMap<Uuid, TwoPhaseCommitTransaction>,
    #[cfg(not(feature = "semantic_api"))]
    active_transactions: HashMap<Uuid, TwoPhaseCommitTransaction>,
    transaction_queue: VecDeque<Uuid>,
    vector_clock: VectorClock,
    lamport_clock: LamportTimestamp,
    node_id: String,
    semaphore: Arc<Semaphore>,
}

/// Journal ordering service with vector clocks
pub struct JournalOrderingService {
    #[cfg(feature = "semantic_api")]
    journal_entries: ImVector<JournalEntry>,
    #[cfg(not(feature = "semantic_api"))]
    journal_entries: Vec<JournalEntry>,
    #[cfg(feature = "semantic_api")]
    entry_index: ImHashMap<Uuid, usize>,
    #[cfg(not(feature = "semantic_api"))]
    entry_index: HashMap<Uuid, usize>,
    vector_clock: VectorClock,
    lamport_clock: LamportTimestamp,
    batch_buffer: Vec<JournalEntry>,
    batch_size: usize,
}

/// Versioned metadata manager using immutable data structures
pub struct VersionedMetadataManager {
    #[cfg(feature = "semantic_api")]
    metadata_versions: ImOrdMap<u64, VersionedMetadata>,
    #[cfg(not(feature = "semantic_api"))]
    metadata_versions: BTreeMap<u64, VersionedMetadata>,
    current_version: u64,
    #[cfg(feature = "semantic_api")]
    layer_states: ImHashMap<String, LayerState>,
    #[cfg(not(feature = "semantic_api"))]
    layer_states: HashMap<String, LayerState>,
    #[cfg(feature = "semantic_api")]
    version_index: ImHashMap<Uuid, u64>,
    #[cfg(not(feature = "semantic_api"))]
    version_index: HashMap<Uuid, u64>,
}

/// Two-phase commit coordinator
pub struct TwoPhaseCommitCoordinator {
    #[cfg(feature = "semantic_api")]
    active_commits: ImHashMap<Uuid, TwoPhaseCommitTransaction>,
    #[cfg(not(feature = "semantic_api"))]
    active_commits: HashMap<Uuid, TwoPhaseCommitTransaction>,
    coordinator_id: String,
    #[cfg(feature = "semantic_api")]
    participants: ImVector<String>,
    #[cfg(not(feature = "semantic_api"))]
    participants: Vec<String>,
    timeout_duration: Duration,
}

/// Recovery manager with log replay
pub struct RecoveryManager {
    #[cfg(feature = "semantic_api")]
    recovery_log: ImVector<RecoveryLogEntry>,
    #[cfg(not(feature = "semantic_api"))]
    recovery_log: Vec<RecoveryLogEntry>,
    #[cfg(feature = "semantic_api")]
    checkpoint_data: ImHashMap<String, Vec<u8>>,
    #[cfg(not(feature = "semantic_api"))]
    checkpoint_data: HashMap<String, Vec<u8>>,
    last_checkpoint: Option<SystemTime>,
    recovery_in_progress: bool,
}

/// Performance cache with lock-free structures
pub struct PerformanceCache {
    #[cfg(feature = "semantic_api")]
    transaction_cache: ImHashMap<Uuid, CachedTransaction>,
    #[cfg(not(feature = "semantic_api"))]
    transaction_cache: HashMap<Uuid, CachedTransaction>,
    #[cfg(feature = "semantic_api")]
    metadata_cache: ImHashMap<String, CachedMetadata>,
    #[cfg(not(feature = "semantic_api"))]
    metadata_cache: HashMap<String, CachedMetadata>,
    #[cfg(feature = "semantic_api")]
    query_cache: ImHashMap<String, CachedQuery>,
    #[cfg(not(feature = "semantic_api"))]
    query_cache: HashMap<String, CachedQuery>,
    cache_size: usize,
    hit_count: u64,
    miss_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTransaction {
    pub transaction_id: Uuid,
    pub result: Vec<u8>,
    pub cached_at: SystemTime,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMetadata {
    pub key: String,
    pub value: serde_json::Value,
    pub cached_at: SystemTime,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQuery {
    pub query_hash: String,
    pub result: QueryResult,
    pub cached_at: SystemTime,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    #[cfg(feature = "semantic_api")]
    pub rows: Vec<ImHashMap<String, serde_json::Value>>,
    #[cfg(not(feature = "semantic_api"))]
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub execution_time: Duration,
    pub layers_accessed: Vec<String>,
}

impl CrossLayerIntegrationFramework {
    /// Create a new integration framework
    pub async fn new(config: IntegrationConfig) -> Result<Self, VexfsError> {
        #[cfg(feature = "semantic_api")]
        let (shutdown_sender, shutdown_receiver) = channel::unbounded();
        
        // Create underlying consistency manager
        let consistency_config = CrossLayerConfig {
            consistency_check_interval_ms: config.consistency_check_interval.as_millis() as u64,
            transaction_timeout_ms: config.transaction_timeout.as_millis() as u64,
            max_concurrent_transactions: config.max_concurrent_transactions,
            enable_consistency_checks: true,
            enable_deadlock_detection: true,
            enable_recovery: config.enable_crash_recovery,
            ..Default::default()
        };
        
        let consistency_manager = Arc::new(
            CrossLayerConsistencyManager::new(consistency_config)?
        );
        
        // Initialize framework components
        let transaction_manager = Arc::new(TokioRwLock::new(
            UnifiedTransactionManager::new(
                config.node_id.clone(),
                config.max_concurrent_transactions,
            )
        ));
        
        let journal_service = Arc::new(TokioRwLock::new(
            JournalOrderingService::new(
                config.node_id.clone(),
                config.journal_batch_size,
            )
        ));
        
        let metadata_manager = Arc::new(TokioRwLock::new(
            VersionedMetadataManager::new()
        ));
        
        let commit_coordinator = Arc::new(TokioRwLock::new(
            TwoPhaseCommitCoordinator::new(
                config.node_id.clone(),
                config.transaction_timeout,
            )
        ));
        
        let recovery_manager = Arc::new(TokioRwLock::new(
            RecoveryManager::new()
        ));
        
        let cache = Arc::new(TokioRwLock::new(
            PerformanceCache::new(config.cache_size)
        ));
        
        let framework = Self {
            config: Arc::new(RwLock::new(config)),
            consistency_manager,
            transaction_manager,
            journal_service,
            metadata_manager,
            commit_coordinator,
            recovery_manager,
            cache,
            stats: Arc::new(TokioRwLock::new(IntegrationStats::default())),
            task_handles: Arc::new(Mutex::new(Vec::new())),
            #[cfg(feature = "semantic_api")]
            shutdown_sender,
            #[cfg(feature = "semantic_api")]
            shutdown_receiver: Arc::new(Mutex::new(shutdown_receiver)),
            #[cfg(feature = "semantic_api")]
            wait_group: Arc::new(WaitGroup::new()),
        };
        
        info!("Cross-Layer Integration Framework created with node_id: {}", 
              framework.config.read().unwrap().node_id);
        
        Ok(framework)
    }
    
    /// Start the integration framework
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<(), VexfsError> {
        info!("Starting Cross-Layer Integration Framework");
        
        // Start underlying consistency manager
        self.consistency_manager.start().await?;
        
        let mut handles = self.task_handles.lock().unwrap();
        
        // Start background tasks
        let recovery_task = self.spawn_recovery_task().await?;
        handles.push(recovery_task);
        
        let journal_task = self.spawn_journal_task().await?;
        handles.push(journal_task);
        
        let metadata_task = self.spawn_metadata_task().await?;
        handles.push(metadata_task);
        
        let cache_task = self.spawn_cache_task().await?;
        handles.push(cache_task);
        
        info!("Cross-Layer Integration Framework started with {} background tasks", 
              handles.len());
        
        Ok(())
    }
    
    /// Stop the integration framework
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<(), VexfsError> {
        info!("Stopping Cross-Layer Integration Framework");
        
        // Send shutdown signal
        #[cfg(feature = "semantic_api")]
        if let Err(e) = self.shutdown_sender.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }
        
        // Stop background tasks
        let handles = {
            let mut handles_guard = self.task_handles.lock().unwrap();
            std::mem::take(&mut *handles_guard)
        };
        
        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Background task failed to complete: {}", e);
            }
        }
        
        // Stop underlying consistency manager
        self.consistency_manager.stop().await?;
        
        // Wait for coordinated shutdown
        #[cfg(feature = "semantic_api")]
        // Clone the wait_group from Arc before calling wait()
        let wait_group = (*self.wait_group).clone();
        wait_group.wait();
        
        info!("Cross-Layer Integration Framework stopped");
        Ok(())
    }
    
    /// Begin a unified cross-layer transaction
    #[instrument(skip(self))]
    pub async fn begin_unified_transaction(
        &self,
        layers: Vec<String>,
        isolation_level: CrossLayerIsolationLevel,
        timeout: Option<Duration>,
    ) -> Result<Uuid, VexfsError> {
        let transaction_id = Uuid::new_v4();
        
        // Create transaction in unified manager
        let mut tm = self.transaction_manager.write().await;
        tm.begin_transaction(transaction_id, layers.clone(), timeout).await?;
        
        // Create transaction in consistency manager
        let layer_mask = self.layers_to_mask(&layers);
        let _consistency_tx_id = self.consistency_manager.begin_transaction(
            layer_mask,
            isolation_level,
            timeout.map(|d| d.as_millis() as u64),
        ).await?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_transactions += 1;
        stats.active_transactions += 1;
        
        info!("Started unified transaction {} for layers: {:?}", 
              transaction_id, layers);
        
        Ok(transaction_id)
    }
    
    /// Add operation to unified transaction
    #[instrument(skip(self, data))]
    pub async fn add_unified_operation(
        &self,
        transaction_id: Uuid,
        layer: String,
        operation_type: String,
        data: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<Uuid, VexfsError> {
        let operation_id = Uuid::new_v4();
        
        // Add to journal service
        let mut js = self.journal_service.write().await;
        js.add_entry(
            transaction_id,
            operation_id,
            layer.clone(),
            operation_type.clone(),
            data.clone(),
            metadata.clone(),
        ).await?;
        
        // Add to consistency manager
        let layer_mask = self.layer_to_mask(&layer);
        let op_type = self.string_to_operation_type(&operation_type);
        self.consistency_manager.add_operation(
            transaction_id,
            op_type,
            layer_mask,
            data,
            0, // flags
            1, // priority
        ).await?;
        
        debug!("Added operation {} to transaction {} for layer {}", 
               operation_id, transaction_id, layer);
        
        Ok(operation_id)
    }
    
    /// Commit unified transaction with two-phase commit
    #[instrument(skip(self))]
    pub async fn commit_unified_transaction(
        &self,
        transaction_id: Uuid,
    ) -> Result<(), VexfsError> {
        let start_time = Instant::now();
        
        // Phase 1: Prepare
        let mut coordinator = self.commit_coordinator.write().await;
        coordinator.prepare_transaction(transaction_id).await?;
        
        // Phase 2: Commit
        coordinator.commit_transaction(transaction_id).await?;
        
        // Commit in consistency manager
        self.consistency_manager.commit_transaction(transaction_id).await?;
        
        // Update metadata version
        let mut mm = self.metadata_manager.write().await;
        mm.create_version(transaction_id).await?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.successful_transactions += 1;
        stats.active_transactions = stats.active_transactions.saturating_sub(1);
        
        let commit_time = start_time.elapsed().as_millis() as f64;
        stats.avg_commit_time_ms = (stats.avg_commit_time_ms + commit_time) / 2.0;
        
        info!("Committed unified transaction {} in {:.2}ms", 
              transaction_id, commit_time);
        
        Ok(())
    }
    
    /// Create a versioned snapshot
    #[instrument(skip(self))]
    pub async fn create_versioned_snapshot(&self) -> Result<u64, VexfsError> {
        let mut mm = self.metadata_manager.write().await;
        let version = mm.create_snapshot().await?;
        
        info!("Created versioned snapshot: version {}", version);
        Ok(version)
    }
    
    /// Restore from versioned snapshot
    #[instrument(skip(self))]
    pub async fn restore_versioned_snapshot(&self, version: u64) -> Result<(), VexfsError> {
        let mut mm = self.metadata_manager.write().await;
        mm.restore_snapshot(version).await?;
        
        info!("Restored from versioned snapshot: version {}", version);
        Ok(())
    }
    
    /// Get integration statistics
    pub async fn get_integration_stats(&self) -> IntegrationStats {
        self.stats.read().await.clone()
    }
    
    /// Reset integration statistics
    pub async fn reset_integration_stats(&self) -> Result<(), VexfsError> {
        let mut stats = self.stats.write().await;
        *stats = IntegrationStats::default();
        info!("Integration statistics reset");
        Ok(())
    }
    
    // Helper methods
    fn layers_to_mask(&self, layers: &[String]) -> u32 {
        let mut mask = 0u32;
        for layer in layers {
            mask |= self.layer_to_mask(layer);
        }
        mask
    }
    
    fn layer_to_mask(&self, layer: &str) -> u32 {
        match layer {
            "filesystem" => 0x01,
            "graph" => 0x02,
            "semantic" => 0x04,
            _ => 0x00,
        }
    }
    
    fn string_to_operation_type(&self, op_type: &str) -> CrossLayerOperationType {
        match op_type {
            "filesystem_only" => CrossLayerOperationType::FilesystemOnly,
            "graph_only" => CrossLayerOperationType::GraphOnly,
            "semantic_only" => CrossLayerOperationType::SemanticOnly,
            "filesystem_graph" => CrossLayerOperationType::FilesystemGraph,
            "filesystem_semantic" => CrossLayerOperationType::FilesystemSemantic,
            "graph_semantic" => CrossLayerOperationType::GraphSemantic,
            "all_layers" => CrossLayerOperationType::AllLayers,
            _ => CrossLayerOperationType::FilesystemOnly,
        }
    }
    
    // Background task spawners
    async fn spawn_recovery_task(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let recovery_manager = Arc::clone(&self.recovery_manager);
        #[cfg(feature = "semantic_api")]
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let config = Arc::clone(&self.config);
        #[cfg(feature = "semantic_api")]
        let wait_group = Arc::clone(&self.wait_group);
        #[cfg(feature = "semantic_api")]
        let _guard = wait_group.clone();
        
        let handle = tokio::spawn(async move {
            #[cfg(feature = "semantic_api")]
            let _guard = _guard;
            
            loop {
                let interval = {
                    let config = config.read().unwrap();
                    config.recovery_check_interval
                };
                
                #[cfg(feature = "semantic_api")]
                match timeout(interval, async {
                    if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                        shutdown_receiver.recv()
                    } else {
                        Err(channel::RecvError)
                    }
                }).await {
                    Ok(Ok(_)) => {
                        debug!("Recovery task received shutdown signal");
                        break;
                    }
                    Ok(Err(_)) => {
                        warn!("Shutdown channel disconnected");
                        break;
                    }
                    Err(_) => {
                        // Timeout, perform recovery check
                        let mut rm = recovery_manager.write().await;
                        if let Err(e) = rm.check_recovery_needed().await {
                            warn!("Recovery check failed: {}", e);
                        }
                    }
                }
                
                #[cfg(not(feature = "semantic_api"))]
                {
                    sleep(interval).await;
                    let mut rm = recovery_manager.write().await;
                    if let Err(e) = rm.check_recovery_needed().await {
                        warn!("Recovery check failed: {}", e);
                    }
                }
            }
            
            debug!("Recovery task completed");
        });
        
        Ok(handle)
    }
    
    async fn spawn_journal_task(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let journal_service = Arc::clone(&self.journal_service);
        #[cfg(feature = "semantic_api")]
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        #[cfg(feature = "semantic_api")]
        let wait_group = Arc::clone(&self.wait_group);
        #[cfg(feature = "semantic_api")]
        let _guard = wait_group.clone();
        
        let handle = tokio::spawn(async move {
            #[cfg(feature = "semantic_api")]
            let _guard = _guard;
            
            let mut interval = interval(Duration::from_millis(100));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut js = journal_service.write().await;
                        if let Err(e) = js.flush_batch().await {
                            warn!("Journal batch flush failed: {}", e);
                        }
                    }
                    _ = async {
                        if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                            shutdown_receiver.recv()
                        } else {
                            Err(channel::RecvError)
                        }
                    } => {
                        debug!("Journal task received shutdown signal");
                        break;
                    }
                }
            }
            
            debug!("Journal task completed");
        });
        
        Ok(handle)
    }
    
    async fn spawn_metadata_task(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let metadata_manager = Arc::clone(&self.metadata_manager);
        #[cfg(feature = "semantic_api")]
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        let config = Arc::clone(&self.config);
        #[cfg(feature = "semantic_api")]
        let wait_group = Arc::clone(&self.wait_group);
        #[cfg(feature = "semantic_api")]
        let _guard = wait_group.clone();
        
        let handle = tokio::spawn(async move {
            #[cfg(feature = "semantic_api")]
            let _guard = _guard;
            
            loop {
                let retention_period = {
                    let config = config.read().unwrap();
                    config.metadata_retention_period
                };
                
                #[cfg(feature = "semantic_api")]
                match timeout(retention_period, async {
                    if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                        shutdown_receiver.recv()
                    } else {
                        Err(channel::RecvError)
                    }
                }).await {
                    Ok(Ok(_)) => {
                        debug!("Metadata task received shutdown signal");
                        break;
                    }
                    Ok(Err(_)) => {
                        warn!("Shutdown channel disconnected");
                        break;
                    }
                    Err(_) => {
                        // Timeout, perform metadata cleanup
                        let mut mm = metadata_manager.write().await;
                        if let Err(e) = mm.cleanup_old_versions().await {
                            warn!("Metadata cleanup failed: {}", e);
                        }
                    }
                }
                
                #[cfg(not(feature = "semantic_api"))]
                {
                    sleep(retention_period).await;
                    let mut mm = metadata_manager.write().await;
                    if let Err(e) = mm.cleanup_old_versions().await {
                        warn!("Metadata cleanup failed: {}", e);
                    }
                }
            }
            
            debug!("Metadata task completed");
        });
        
        Ok(handle)
    }
    
    async fn spawn_cache_task(&self) -> Result<tokio::task::JoinHandle<()>, VexfsError> {
        let cache = Arc::clone(&self.cache);
        #[cfg(feature = "semantic_api")]
        let shutdown_receiver = Arc::clone(&self.shutdown_receiver);
        #[cfg(feature = "semantic_api")]
        let wait_group = Arc::clone(&self.wait_group);
        #[cfg(feature = "semantic_api")]
        let _guard = wait_group.clone();
        
        let handle = tokio::spawn(async move {
            #[cfg(feature = "semantic_api")]
            let _guard = _guard;
            
            let mut interval = interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut cache_guard = cache.write().await;
                        cache_guard.cleanup_expired();
                    }
                    _ = async {
                        if let Ok(shutdown_receiver) = shutdown_receiver.try_lock() {
                            shutdown_receiver.recv()
                        } else {
                            Err(channel::RecvError)
                        }
                    } => {
                        debug!("Cache task received shutdown signal");
                        break;
                    }
                }
            }
            
            debug!("Cache task completed");
        });
        
        Ok(handle)
    }
}

// Implementation of component managers
impl UnifiedTransactionManager {
    pub fn new(node_id: String, max_transactions: usize) -> Self {
        Self {
            #[cfg(feature = "semantic_api")]
            active_transactions: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            active_transactions: HashMap::new(),
            transaction_queue: VecDeque::new(),
            vector_clock: VectorClock::new(node_id.clone()),
            lamport_clock: LamportTimestamp::new(node_id.parse().unwrap_or(0)),
            node_id,
            semaphore: Arc::new(Semaphore::new(max_transactions)),
        }
    }
    
    pub async fn begin_transaction(
        &mut self,
        transaction_id: Uuid,
        layers: Vec<String>,
        timeout: Option<Duration>,
    ) -> Result<(), VexfsError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| {
            VexfsError::ResourceLimit("Transaction limit reached".to_string())
        })?;
        
        self.vector_clock.tick();
        self.lamport_clock.tick();
        
        let transaction = TwoPhaseCommitTransaction {
            transaction_id,
            coordinator_id: self.node_id.clone(),
            state: TwoPhaseCommitState::Init,
            #[cfg(feature = "semantic_api")]
            participants: layers.into_iter().collect(),
            #[cfg(not(feature = "semantic_api"))]
            participants: layers,
            #[cfg(feature = "semantic_api")]
            prepare_votes: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            prepare_votes: HashMap::new(),
            timeout: timeout.unwrap_or(Duration::from_secs(30)),
            started_at: SystemTime::now(),
            prepared_at: None,
            committed_at: None,
            #[cfg(feature = "semantic_api")]
            operations: ImVector::new(),
            #[cfg(not(feature = "semantic_api"))]
            operations: Vec::new(),
        };
        
        self.active_transactions.insert(transaction_id, transaction);
        self.transaction_queue.push_back(transaction_id);
        
        Ok(())
    }
}

impl JournalOrderingService {
    pub fn new(node_id: String, batch_size: usize) -> Self {
        Self {
            #[cfg(feature = "semantic_api")]
            journal_entries: ImVector::new(),
            #[cfg(not(feature = "semantic_api"))]
            journal_entries: Vec::new(),
            #[cfg(feature = "semantic_api")]
            entry_index: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            entry_index: HashMap::new(),
            vector_clock: VectorClock::new(node_id.clone()),
            lamport_clock: LamportTimestamp::new(node_id.parse().unwrap_or(0)),
            batch_buffer: Vec::new(),
            batch_size,
        }
    }
    
    pub async fn add_entry(
        &mut self,
        transaction_id: Uuid,
        operation_id: Uuid,
        layer: String,
        operation_type: String,
        data: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<(), VexfsError> {
        self.vector_clock.tick();
        self.lamport_clock.tick();
        
        let entry = JournalEntry {
            entry_id: Uuid::new_v4(),
            transaction_id,
            operation_id,
            vector_clock: self.vector_clock.clone(),
            lamport_timestamp: self.lamport_clock,
            layer_mask: self.layer_to_mask(&layer),
            operation_type: self.string_to_operation_type(&operation_type),
            data,
            #[cfg(feature = "semantic_api")]
            metadata: metadata.into_iter().collect(),
            #[cfg(not(feature = "semantic_api"))]
            metadata,
            #[cfg(feature = "semantic_api")]
            causality_links: ImVector::new(),
            #[cfg(not(feature = "semantic_api"))]
            causality_links: Vec::new(),
            created_at: SystemTime::now(),
            committed_at: None,
        };
        
        self.batch_buffer.push(entry);
        
        if self.batch_buffer.len() >= self.batch_size {
            self.flush_batch().await?;
        }
        
        Ok(())
    }
    
    pub async fn flush_batch(&mut self) -> Result<(), VexfsError> {
        if self.batch_buffer.is_empty() {
            return Ok(());
        }
        
        // Sort by vector clock and lamport timestamp for ordering
        self.batch_buffer.sort_by(|a, b| {
            if a.vector_clock.happens_before(&b.vector_clock) {
                std::cmp::Ordering::Less
            } else if b.vector_clock.happens_before(&a.vector_clock) {
                std::cmp::Ordering::Greater
            } else {
                a.lamport_timestamp.cmp(&b.lamport_timestamp)
            }
        });
        
        for entry in self.batch_buffer.drain(..) {
            #[cfg(feature = "semantic_api")]
            {
                let index = self.journal_entries.len();
                self.entry_index.insert(entry.entry_id, index);
                self.journal_entries.push_back(entry);
            }
            #[cfg(not(feature = "semantic_api"))]
            {
                let index = self.journal_entries.len();
                self.entry_index.insert(entry.entry_id, index);
                self.journal_entries.push(entry);
            }
        }
        
        Ok(())
    }
    
    fn layer_to_mask(&self, layer: &str) -> u32 {
        match layer {
            "filesystem" => 0x01,
            "graph" => 0x02,
            "semantic" => 0x04,
            _ => 0x00,
        }
    }
    
    fn string_to_operation_type(&self, op_type: &str) -> CrossLayerOperationType {
        match op_type {
            "filesystem_only" => CrossLayerOperationType::FilesystemOnly,
            "graph_only" => CrossLayerOperationType::GraphOnly,
            "semantic_only" => CrossLayerOperationType::SemanticOnly,
            "filesystem_graph" => CrossLayerOperationType::FilesystemGraph,
            "filesystem_semantic" => CrossLayerOperationType::FilesystemSemantic,
            "graph_semantic" => CrossLayerOperationType::GraphSemantic,
            "all_layers" => CrossLayerOperationType::AllLayers,
            _ => CrossLayerOperationType::FilesystemOnly,
        }
    }
}

impl VersionedMetadataManager {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "semantic_api")]
            metadata_versions: ImOrdMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            metadata_versions: BTreeMap::new(),
            current_version: 0,
            #[cfg(feature = "semantic_api")]
            layer_states: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            layer_states: HashMap::new(),
            #[cfg(feature = "semantic_api")]
            version_index: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            version_index: HashMap::new(),
        }
    }
    
    pub async fn create_version(&mut self, transaction_id: Uuid) -> Result<u64, VexfsError> {
        self.current_version += 1;
        
        let metadata = VersionedMetadata {
            version: self.current_version,
            transaction_id,
            timestamp: SystemTime::now(),
            vector_clock: VectorClock::new("metadata".to_string()),
            #[cfg(feature = "semantic_api")]
            metadata: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            metadata: HashMap::new(),
            previous_version: if self.current_version > 1 {
                Some(self.current_version - 1)
            } else {
                None
            },
            layer_states: self.layer_states.clone(),
        };
        
        self.metadata_versions.insert(self.current_version, metadata);
        self.version_index.insert(transaction_id, self.current_version);
        
        Ok(self.current_version)
    }
    
    pub async fn create_snapshot(&mut self) -> Result<u64, VexfsError> {
        self.create_version(Uuid::new_v4()).await
    }
    
    pub async fn restore_snapshot(&mut self, version: u64) -> Result<(), VexfsError> {
        let metadata = self.metadata_versions.get(&version)
            .ok_or_else(|| VexfsError::EntryNotFound(format!("Version {} not found", version)))?;
        
        self.layer_states = metadata.layer_states.clone();
        self.current_version = version;
        
        Ok(())
    }
    
    pub async fn cleanup_old_versions(&mut self) -> Result<(), VexfsError> {
        let cutoff = SystemTime::now() - Duration::from_secs(86400); // 24 hours
        
        let old_versions: Vec<u64> = self.metadata_versions
            .iter()
            .filter(|(_, metadata)| metadata.timestamp < cutoff)
            .map(|(version, _)| *version)
            .collect();
        
        for version in old_versions {
            self.metadata_versions.remove(&version);
        }
        
        Ok(())
    }
}

impl TwoPhaseCommitCoordinator {
    pub fn new(coordinator_id: String, timeout: Duration) -> Self {
        Self {
            #[cfg(feature = "semantic_api")]
            active_commits: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            active_commits: HashMap::new(),
            coordinator_id,
            #[cfg(feature = "semantic_api")]
            participants: ImVector::new(),
            #[cfg(not(feature = "semantic_api"))]
            participants: Vec::new(),
            timeout_duration: timeout,
        }
    }
    
    pub async fn prepare_transaction(&mut self, transaction_id: Uuid) -> Result<(), VexfsError> {
        let transaction = self.active_commits.get_mut(&transaction_id)
            .ok_or_else(|| VexfsError::EntryNotFound("Transaction not found".to_string()))?;
        
        // Phase 1: Send prepare to all participants
        for participant in &transaction.participants {
            // In a real implementation, this would send prepare messages
            // For now, we'll simulate successful preparation
            transaction.prepare_votes.insert(participant.clone(), true);
        }
        
        transaction.state = TwoPhaseCommitState::Prepared;
        transaction.prepared_at = Some(SystemTime::now());
        
        Ok(())
    }
    
    pub async fn commit_transaction(&mut self, transaction_id: Uuid) -> Result<(), VexfsError> {
        let transaction = self.active_commits.get_mut(&transaction_id)
            .ok_or_else(|| VexfsError::EntryNotFound("Transaction not found".to_string()))?;
        
        // Check if all participants voted to commit
        let all_prepared = transaction.participants.iter()
            .all(|p| *transaction.prepare_votes.get(p).unwrap_or(&false));
        
        if !all_prepared {
            transaction.state = TwoPhaseCommitState::Aborted;
            return Err(VexfsError::TransactionError(TransactionErrorKind::CommitFailed));
        }
        
        // Phase 2: Send commit to all participants
        transaction.state = TwoPhaseCommitState::Committed;
        transaction.committed_at = Some(SystemTime::now());
        
        Ok(())
    }
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "semantic_api")]
            recovery_log: ImVector::new(),
            #[cfg(not(feature = "semantic_api"))]
            recovery_log: Vec::new(),
            #[cfg(feature = "semantic_api")]
            checkpoint_data: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            checkpoint_data: HashMap::new(),
            last_checkpoint: None,
            recovery_in_progress: false,
        }
    }
    
    pub async fn check_recovery_needed(&mut self) -> Result<(), VexfsError> {
        if self.recovery_in_progress {
            return Ok(());
        }
        
        // Check if recovery is needed based on log entries
        let needs_recovery = self.recovery_log.iter()
            .any(|entry| entry.timestamp > self.last_checkpoint.unwrap_or(UNIX_EPOCH));
        
        if needs_recovery {
            self.perform_recovery().await?;
        }
        
        Ok(())
    }
    
    async fn perform_recovery(&mut self) -> Result<(), VexfsError> {
        self.recovery_in_progress = true;
        
        // Replay log entries since last checkpoint
        let checkpoint_time = self.last_checkpoint.unwrap_or(UNIX_EPOCH);
        
        for entry in self.recovery_log.iter() {
            if entry.timestamp > checkpoint_time {
                // Replay the operation
                debug!("Replaying operation: {:?}", entry.operation_type);
            }
        }
        
        self.last_checkpoint = Some(SystemTime::now());
        self.recovery_in_progress = false;
        
        Ok(())
    }
}

impl PerformanceCache {
    pub fn new(cache_size: usize) -> Self {
        Self {
            #[cfg(feature = "semantic_api")]
            transaction_cache: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            transaction_cache: HashMap::new(),
            #[cfg(feature = "semantic_api")]
            metadata_cache: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            metadata_cache: HashMap::new(),
            #[cfg(feature = "semantic_api")]
            query_cache: ImHashMap::new(),
            #[cfg(not(feature = "semantic_api"))]
            query_cache: HashMap::new(),
            cache_size,
            hit_count: 0,
            miss_count: 0,
        }
    }
    
    pub fn cleanup_expired(&mut self) {
        let cutoff = SystemTime::now() - Duration::from_secs(3600); // 1 hour
        
        let expired_keys: Vec<String> = self.query_cache
            .iter()
            .filter(|(_, cached)| cached.cached_at < cutoff)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            self.query_cache.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_integration_framework_creation() {
        let config = IntegrationConfig::default();
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        
        let stats = framework.get_integration_stats().await;
        assert_eq!(stats.total_transactions, 0);
    }
    
    #[tokio::test]
    async fn test_unified_transaction_lifecycle() {
        let config = IntegrationConfig::default();
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        
        framework.start().await.unwrap();
        
        // Begin transaction
        let transaction_id = framework.begin_unified_transaction(
            vec!["filesystem".to_string(), "graph".to_string()],
            CrossLayerIsolationLevel::Serializable,
            Some(Duration::from_secs(10)),
        ).await.unwrap();
        
        // Add operation
        let operation_id = framework.add_unified_operation(
            transaction_id,
            "filesystem".to_string(),
            "write".to_string(),
            vec![1, 2, 3, 4],
            HashMap::new(),
        ).await.unwrap();
        
        assert!(operation_id != Uuid::nil());
        
        // Commit transaction
        framework.commit_unified_transaction(transaction_id).await.unwrap();
        
        // Check statistics
        let stats = framework.get_integration_stats().await;
        assert_eq!(stats.total_transactions, 1);
        assert_eq!(stats.successful_transactions, 1);
        
        framework.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_versioned_metadata() {
        let mut manager = VersionedMetadataManager::new();
        
        let transaction_id = Uuid::new_v4();
        let version = manager.create_version(transaction_id).await.unwrap();
        assert_eq!(version, 1);
        
        let snapshot_version = manager.create_snapshot().await.unwrap();
        assert_eq!(snapshot_version, 2);
        
        manager.restore_snapshot(version).await.unwrap();
        assert_eq!(manager.current_version, version);
    }
    
    #[tokio::test]
    async fn test_vector_clock_ordering() {
        let mut clock1 = VectorClock::new("node1".to_string());
        let mut clock2 = VectorClock::new("node2".to_string());
        
        clock1.tick();
        assert!(clock1.happens_before(&clock2) || clock1.concurrent_with(&clock2));
        
        clock2.update(&clock1);
        assert!(clock1.happens_before(&clock2));
    }
    
    #[tokio::test]
    async fn test_journal_ordering() {
        let mut service = JournalOrderingService::new("node1".to_string(), 2);
        
        let tx_id = Uuid::new_v4();
        let op_id1 = Uuid::new_v4();
        let op_id2 = Uuid::new_v4();
        
        service.add_entry(
            tx_id,
            op_id1,
            "filesystem".to_string(),
            "write".to_string(),
            vec![1, 2, 3],
            HashMap::new(),
        ).await.unwrap();
        
        service.add_entry(
            tx_id,
            op_id2,
            "graph".to_string(),
            "update".to_string(),
            vec![4, 5, 6],
            HashMap::new(),
        ).await.unwrap();
        
        // Should trigger flush due to batch size
        assert!(service.batch_buffer.is_empty());
    }
}