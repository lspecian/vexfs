//! Implementation methods for Distributed Event Coordinator
//! 
//! This module contains the private implementation methods for the
//! DistributedEventCoordinator, including Raft consensus, conflict resolution,
//! and network optimization.

use super::*;

impl DistributedEventCoordinator {
    /// Initialize peer connections
    async fn initialize_peer_connections(&self) -> VexfsResult<()> {
        let mut connections = self.peer_connections.write().unwrap();
        
        for peer_addr in &self.config.peer_addresses {
            let peer_id = Uuid::new_v4(); // In real implementation, this would be discovered
            let connection = PeerConnection {
                node_id: peer_id,
                address: *peer_addr,
                status: ConnectionStatus::Disconnected,
                last_heartbeat: Instant::now(),
                latency_ms: 0,
                sender: None,
            };
            connections.insert(peer_id, connection);
        }
        
        Ok(())
    }

    /// Close peer connections
    async fn close_peer_connections(&self) -> VexfsResult<()> {
        let mut connections = self.peer_connections.write().unwrap();
        connections.clear();
        Ok(())
    }

    /// Create distributed event from semantic event
    async fn create_distributed_event(&self, event: SemanticEvent) -> VexfsResult<DistributedSemanticEvent> {
        // Get current vector clock
        let mut vector_clock = VectorClock::new();
        vector_clock.increment(&self.config.node_id.to_string());

        // Generate sequence number
        let sequence_number = self.get_next_sequence_number();

        // Create coordination metadata
        let coordination_metadata = CoordinationMetadata {
            priority: event.priority,
            replication_factor: 3, // Default replication factor
            consistency_level: ConsistencyLevel::Strong,
            coordination_timeout_ms: self.config.performance_config.target_consensus_latency_ms,
            byzantine_tolerance: self.config.raft_config.byzantine_fault_tolerance,
            network_hints: NetworkOptimizationHints {
                compression: CompressionAlgorithm::LZ4,
                batching_enabled: self.config.performance_config.batch_processing,
                max_batch_size: 100,
                connection_pooling: true,
                multiplexing_enabled: true,
            },
        };

        Ok(DistributedSemanticEvent {
            event,
            vector_clock,
            origin_node: self.config.node_id,
            sequence_number,
            coordination_metadata,
            conflict_resolution: None,
        })
    }

    /// Get next sequence number for this node
    fn get_next_sequence_number(&self) -> u64 {
        static SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);
        SEQUENCE_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Start Raft consensus worker
    async fn start_raft_consensus_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(10));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Raft consensus worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = coordinator.process_raft_consensus().await {
                            error!("Error in Raft consensus processing: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start leader election worker
    async fn start_leader_election_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut election_timeout = coordinator.get_election_timeout();
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Leader election worker shutting down");
                        break;
                    }
                    _ = sleep(election_timeout) => {
                        if let Err(e) = coordinator.handle_election_timeout().await {
                            error!("Error handling election timeout: {}", e);
                        }
                        election_timeout = coordinator.get_election_timeout();
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start log replication worker
    async fn start_log_replication_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(50));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Log replication worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if coordinator.is_leader() {
                            if let Err(e) = coordinator.replicate_log_entries().await {
                                error!("Error replicating log entries: {}", e);
                            }
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start heartbeat worker
    async fn start_heartbeat_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(coordinator.config.raft_config.heartbeat_interval_ms));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Heartbeat worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if coordinator.is_leader() {
                            if let Err(e) = coordinator.send_heartbeats().await {
                                error!("Error sending heartbeats: {}", e);
                            }
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start event coordination worker
    async fn start_event_coordination_worker(
        &self, 
        mut command_receiver: mpsc::UnboundedReceiver<CoordinationCommand>,
        mut shutdown: broadcast::Receiver<()>
    ) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Event coordination worker shutting down");
                        break;
                    }
                    command = command_receiver.recv() => {
                        match command {
                            Some(cmd) => {
                                if let Err(e) = coordinator.handle_coordination_command(cmd).await {
                                    error!("Error handling coordination command: {}", e);
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start conflict resolution worker
    async fn start_conflict_resolution_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Conflict resolution worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = coordinator.process_conflict_resolution().await {
                            error!("Error in conflict resolution: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start network optimization worker
    async fn start_network_optimization_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(1000));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Network optimization worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = coordinator.optimize_network_performance().await {
                            error!("Error in network optimization: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start performance monitoring worker
    async fn start_performance_monitoring_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(5000));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Performance monitoring worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        coordinator.update_performance_metrics().await;
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Start health monitoring worker
    async fn start_health_monitoring_worker(&self, mut shutdown: broadcast::Receiver<()>) -> VexfsResult<tokio::task::JoinHandle<()>> {
        let coordinator = self.clone_for_worker();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(10000));
            
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Health monitoring worker shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = coordinator.check_cluster_health().await {
                            error!("Error checking cluster health: {}", e);
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }

    /// Process Raft consensus
    async fn process_raft_consensus(&self) -> VexfsResult<()> {
        // Check for pending log entries to commit
        let commit_index = self.commit_index.load(Ordering::Acquire);
        let last_applied = self.last_applied.load(Ordering::Acquire);
        
        if commit_index > last_applied {
            self.apply_committed_entries(last_applied + 1, commit_index).await?;
            self.last_applied.store(commit_index, Ordering::Release);
        }
        
        Ok(())
    }

    /// Handle election timeout
    async fn handle_election_timeout(&self) -> VexfsResult<()> {
        let current_state = self.raft_state.read().unwrap().clone();
        
        match current_state {
            RaftState::Follower | RaftState::Candidate => {
                self.start_election().await?;
            }
            RaftState::Leader => {
                // Leaders don't timeout
            }
        }
        
        Ok(())
    }

    /// Start leader election
    async fn start_election(&self) -> VexfsResult<()> {
        info!("Starting leader election for node {}", self.config.node_id);
        
        // Transition to candidate state
        *self.raft_state.write().unwrap() = RaftState::Candidate;
        
        // Increment current term
        let new_term = self.current_term.fetch_add(1, Ordering::AcqRel) + 1;
        
        // Vote for self
        *self.voted_for.write().unwrap() = Some(self.config.node_id);
        
        // Send RequestVote messages to all peers
        self.send_vote_requests(new_term).await?;
        
        Ok(())
    }

    /// Send vote requests to all peers
    async fn send_vote_requests(&self, term: u64) -> VexfsResult<()> {
        let log_entries = self.log_entries.read().unwrap();
        let last_log_index = log_entries.len() as u64;
        let last_log_term = log_entries.last().map(|e| e.term).unwrap_or(0);
        
        let message = RaftMessage::RequestVote {
            term,
            candidate_id: self.config.node_id,
            last_log_index,
            last_log_term,
        };
        
        self.broadcast_message(message).await?;
        Ok(())
    }

    /// Replicate log entries to followers
    async fn replicate_log_entries(&self) -> VexfsResult<()> {
        let connections = self.peer_connections.read().unwrap();
        let log_entries = self.log_entries.read().unwrap();
        let current_term = self.current_term.load(Ordering::Acquire);
        
        for (peer_id, connection) in connections.iter() {
            if connection.status == ConnectionStatus::Connected {
                let next_index = self.next_index.read().unwrap().get(peer_id).copied().unwrap_or(1);
                let prev_log_index = next_index.saturating_sub(1);
                let prev_log_term = if prev_log_index > 0 {
                    log_entries.get((prev_log_index - 1) as usize).map(|e| e.term).unwrap_or(0)
                } else {
                    0
                };
                
                let entries_to_send = log_entries
                    .iter()
                    .skip(next_index as usize)
                    .take(self.config.raft_config.max_entries_per_append)
                    .cloned()
                    .collect();
                
                let message = RaftMessage::AppendEntries {
                    term: current_term,
                    leader_id: self.config.node_id,
                    prev_log_index,
                    prev_log_term,
                    entries: entries_to_send,
                    leader_commit: self.commit_index.load(Ordering::Acquire),
                };
                
                self.send_message_to_peer(*peer_id, message).await?;
            }
        }
        
        Ok(())
    }

    /// Send heartbeats to all followers
    async fn send_heartbeats(&self) -> VexfsResult<()> {
        let current_term = self.current_term.load(Ordering::Acquire);
        let message = RaftMessage::Heartbeat {
            term: current_term,
            leader_id: self.config.node_id,
        };
        
        self.broadcast_message(message).await?;
        Ok(())
    }

    /// Handle coordination command
    async fn handle_coordination_command(&self, command: CoordinationCommand) -> VexfsResult<()> {
        match command {
            CoordinationCommand::SubmitEvent(event) => {
                self.submit_event_to_raft(event).await?;
            }
            CoordinationCommand::RequestElection => {
                self.start_election().await?;
            }
            CoordinationCommand::CompactLog => {
                self.compact_log().await?;
            }
            CoordinationCommand::CreateSnapshot => {
                self.create_snapshot().await?;
            }
            CoordinationCommand::RecoverFromFailure => {
                self.recover_from_failure().await?;
            }
            CoordinationCommand::UpdateConfig(new_config) => {
                self.update_configuration(new_config).await?;
            }
        }
        
        Ok(())
    }

    /// Submit event to Raft log
    async fn submit_event_to_raft(&self, event: DistributedSemanticEvent) -> VexfsResult<()> {
        if !self.is_leader() {
            return Err(VexfsError::InvalidOperation("Only leader can submit events".to_string()));
        }
        
        let current_term = self.current_term.load(Ordering::Acquire);
        let mut log_entries = self.log_entries.write().unwrap();
        let next_index = log_entries.len() as u64 + 1;
        
        let log_entry = RaftLogEntry {
            index: next_index,
            term: current_term,
            event,
            timestamp: SystemTime::now(),
            checksum: 0, // Calculate actual checksum
        };
        
        log_entries.push(log_entry);
        Ok(())
    }

    /// Apply committed entries
    async fn apply_committed_entries(&self, start_index: u64, end_index: u64) -> VexfsResult<()> {
        let log_entries = self.log_entries.read().unwrap();
        
        for index in start_index..=end_index {
            if let Some(entry) = log_entries.get((index - 1) as usize) {
                self.apply_log_entry(entry).await?;
            }
        }
        
        Ok(())
    }

    /// Apply a single log entry
    async fn apply_log_entry(&self, entry: &RaftLogEntry) -> VexfsResult<()> {
        // Move event from pending to committed
        let event_id = entry.event.event.event_id;
        
        if let Some(event) = self.pending_events.write().unwrap().remove(&event_id) {
            self.committed_events.write().unwrap().push_back(event.clone());
            
            // Integrate with propagation manager if available
            if let Some(propagation_manager) = &self.propagation_manager {
                let manager = propagation_manager.lock().unwrap();
                // Propagate the committed event
                // manager.propagate_event(...);
            }
        }
        
        Ok(())
    }

    /// Process conflict resolution
    async fn process_conflict_resolution(&self) -> VexfsResult<()> {
        // Check for conflicts in pending events
        let pending_events: Vec<_> = self.pending_events.read().unwrap().values().cloned().collect();
        
        for event in &pending_events {
            if let Some(conflicts) = self.detect_conflicts(event, &pending_events).await? {
                let resolution = self.conflict_resolver.resolve_conflicts(conflicts).await?;
                self.apply_conflict_resolution(resolution).await?;
            }
        }
        
        Ok(())
    }

    /// Detect conflicts between events
    async fn detect_conflicts(
        &self, 
        event: &DistributedSemanticEvent, 
        all_events: &[DistributedSemanticEvent]
    ) -> VexfsResult<Option<Vec<DistributedSemanticEvent>>> {
        let mut conflicts = Vec::new();
        
        for other_event in all_events {
            if other_event.event.event_id != event.event.event_id {
                if self.events_conflict(event, other_event) {
                    conflicts.push(other_event.clone());
                }
            }
        }
        
        if conflicts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(conflicts))
        }
    }

    /// Check if two events conflict
    fn events_conflict(&self, event1: &DistributedSemanticEvent, event2: &DistributedSemanticEvent) -> bool {
        // Check for resource conflicts
        if let (Some(ctx1), Some(ctx2)) = (&event1.event.context, &event2.event.context) {
            // Check filesystem path conflicts
            if let (Some(fs1), Some(fs2)) = (&ctx1.filesystem, &ctx2.filesystem) {
                if fs1.path == fs2.path {
                    // Same path - potential conflict
                    return true;
                }
            }
            
            // Check graph node conflicts
            if let (Some(graph1), Some(graph2)) = (&ctx1.graph, &ctx2.graph) {
                if graph1.node_id == graph2.node_id {
                    // Same node - potential conflict
                    return true;
                }
            }
        }
        
        false
    }

    /// Apply conflict resolution
    async fn apply_conflict_resolution(&self, resolution: ConflictResolutionData) -> VexfsResult<()> {
        // Update conflict resolution statistics
        let mut stats = self.conflict_resolver.resolution_stats.write().unwrap();
        *stats.entry(resolution.conflict_type).or_insert(0) += 1;
        
        // Update coordination metrics
        self.coordination_metrics.write().unwrap().conflicts_resolved += 1;
        
        Ok(())
    }

    /// Optimize network performance
    async fn optimize_network_performance(&self) -> VexfsResult<()> {
        // Analyze connection latencies
        let connections = self.peer_connections.read().unwrap();
        let mut total_latency = 0u64;
        let mut connection_count = 0;
        
        for connection in connections.values() {
            if connection.status == ConnectionStatus::Connected {
                total_latency += connection.latency_ms;
                connection_count += 1;
            }
        }
        
        if connection_count > 0 {
            let avg_latency = total_latency / connection_count as u64;
            
            // Adjust batching based on latency
            if avg_latency > self.config.performance_config.target_consensus_latency_ms {
                // Increase batching to reduce overhead
                // Implementation would adjust batching parameters
            }
        }
        
        Ok(())
    }

    /// Update performance metrics
    async fn update_performance_metrics(&self) {
        let mut metrics = self.coordination_metrics.write().unwrap();
        
        // Calculate average latency
        let latencies = self.latency_histogram.read().unwrap();
        if !latencies.is_empty() {
            let sum: u64 = latencies.iter().sum();
            metrics.avg_consensus_latency_ms = sum as f64 / latencies.len() as f64;
        }
        
        // Calculate consistency percentage
        if metrics.total_events > 0 {
            metrics.consistency_percentage = 
                (metrics.successful_coordinations as f64 / metrics.total_events as f64) * 100.0;
        }
        
        // Calculate network throughput
        // Implementation would calculate events per second
        metrics.network_throughput = metrics.successful_coordinations as f64 / 60.0; // Events per minute
    }

    /// Check cluster health
    async fn check_cluster_health(&self) -> VexfsResult<()> {
        let connections = self.peer_connections.read().unwrap();
        let mut healthy_nodes = 0;
        let total_nodes = connections.len() + 1; // Include self
        
        for connection in connections.values() {
            if connection.status == ConnectionStatus::Connected {
                healthy_nodes += 1;
            }
        }
        
        let health_percentage = (healthy_nodes as f64 / total_nodes as f64) * 100.0;
        
        if health_percentage < 50.0 {
            warn!("Cluster health degraded: {}% of nodes healthy", health_percentage);
        }
        
        Ok(())
    }

    /// Update coordination metrics
    async fn update_coordination_metrics(&self, success: bool, latency_ms: u64) {
        let mut metrics = self.coordination_metrics.write().unwrap();
        metrics.total_events += 1;
        
        if success {
            metrics.successful_coordinations += 1;
        } else {
            metrics.failed_coordinations += 1;
        }
        
        // Update average latency
        let total_latency = metrics.avg_consensus_latency_ms * (metrics.total_events - 1) as f64;
        metrics.avg_consensus_latency_ms = (total_latency + latency_ms as f64) / metrics.total_events as f64;
    }

    /// Get election timeout with jitter
    fn get_election_timeout(&self) -> Duration {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (min, max) = self.config.raft_config.election_timeout_ms;
        let timeout_ms = rng.gen_range(min..=max);
        Duration::from_millis(timeout_ms)
    }

    /// Broadcast message to all peers
    async fn broadcast_message(&self, message: RaftMessage) -> VexfsResult<()> {
        let connections = self.peer_connections.read().unwrap();
        
        for peer_id in connections.keys() {
            if let Err(e) = self.send_message_to_peer(*peer_id, message.clone()).await {
                warn!("Failed to send message to peer {}: {}", peer_id, e);
            }
        }
        
        Ok(())
    }

    /// Send message to specific peer
    async fn send_message_to_peer(&self, peer_id: Uuid, message: RaftMessage) -> VexfsResult<()> {
        let connections = self.peer_connections.read().unwrap();
        
        if let Some(connection) = connections.get(&peer_id) {
            if let Some(sender) = &connection.sender {
                sender.send(message)?;
            }
        }
        
        Ok(())
    }

    /// Clone coordinator for worker tasks
    fn clone_for_worker(&self) -> Self {
        Self {
            config: self.config.clone(),
            raft_state: self.raft_state.clone(),
            current_term: self.current_term.clone(),
            voted_for: self.voted_for.clone(),
            log_entries: self.log_entries.clone(),
            commit_index: self.commit_index.clone(),
            last_applied: self.last_applied.clone(),
            next_index: self.next_index.clone(),
            match_index: self.match_index.clone(),
            peer_connections: self.peer_connections.clone(),
            connection_pool: self.connection_pool.clone(),
            pending_events: self.pending_events.clone(),
            committed_events: self.committed_events.clone(),
            conflict_resolver: self.conflict_resolver.clone(),
            crdt_state: self.crdt_state.clone(),
            coordination_metrics: self.coordination_metrics.clone(),
            latency_histogram: self.latency_histogram.clone(),
            propagation_manager: self.propagation_manager.clone(),
            routing_engine: self.routing_engine.clone(),
            shutdown_sender: self.shutdown_sender.clone(),
            command_sender: self.command_sender.clone(),
            worker_handles: self.worker_handles.clone(),
        }
    }

    // Additional methods for log compaction, snapshots, recovery, etc.
    async fn compact_log(&self) -> VexfsResult<()> {
        // Implementation for log compaction
        Ok(())
    }

    async fn create_snapshot(&self) -> VexfsResult<()> {
        // Implementation for snapshot creation
        Ok(())
    }

    async fn recover_from_failure(&self) -> VexfsResult<()> {
        // Implementation for failure recovery
        Ok(())
    }

    async fn update_configuration(&self, _new_config: DistributedCoordinatorConfig) -> VexfsResult<()> {
        // Implementation for configuration updates
        Ok(())
    }
}

// Implementation for supporting types
impl ConnectionPool {
    fn new(max_connections: usize, timeout: Duration) -> Self {
        Self {
            available: Arc::new(RwLock::new(VecDeque::new())),
            active: Arc::new(AtomicUsize::new(0)),
            max_connections,
            timeout,
        }
    }
}

impl ConflictResolver {
    fn new() -> Self {
        Self {
            strategies: HashMap::new(),
            custom_resolvers: HashMap::new(),
            resolution_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn resolve_conflicts(&self, conflicts: Vec<DistributedSemanticEvent>) -> VexfsResult<ConflictResolutionData> {
        // Default resolution strategy: Last Writer Wins
        let resolution_data = ConflictResolutionData {
            conflict_type: ConflictType::ConcurrentUpdate,
            resolution_strategy: ConflictResolutionStrategy::LastWriterWins,
            conflicting_events: conflicts.iter().map(|e| e.event.event_id).collect(),
            resolution_timestamp: SystemTime::now(),
            resolution_metadata: HashMap::new(),
        };
        
        Ok(resolution_data)
    }
}

impl CRDTState {
    fn new() -> Self {
        Self {
            g_counters: HashMap::new(),
            pn_counters: HashMap::new(),
            lww_registers: HashMap::new(),
            or_sets: HashMap::new(),
            vector_clocks: HashMap::new(),
        }
    }
}

// CRDT implementations
impl GCounter {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    fn increment(&mut self, node_id: Uuid) {
        *self.counters.entry(node_id).or_insert(0) += 1;
    }

    fn value(&self) -> u64 {
        self.counters.values().sum()
    }

    fn merge(&mut self, other: &GCounter) {
        for (node_id, count) in &other.counters {
            let current = self.counters.entry(*node_id).or_insert(0);
            *current = (*current).max(*count);
        }
    }
}

impl PNCounter {
    fn new() -> Self {
        Self {
            positive: GCounter::new(),
            negative: GCounter::new(),
        }
    }

    fn increment(&mut self, node_id: Uuid) {
        self.positive.increment(node_id);
    }

    fn decrement(&mut self, node_id: Uuid) {
        self.negative.increment(node_id);
    }

    fn value(&self) -> i64 {
        self.positive.value() as i64 - self.negative.value() as i64
    }

    fn merge(&mut self, other: &PNCounter) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}

impl LWWRegister {
    fn new(initial_value: String, node_id: Uuid) -> Self {
        Self {
            value: initial_value,
            timestamp: SystemTime::now(),
            node_id,
        }
    }

    fn set(&mut self, value: String, node_id: Uuid) {
        let now = SystemTime::now();
        if now > self.timestamp || (now == self.timestamp && node_id > self.node_id) {
            self.value = value;
            self.timestamp = now;
            self.node_id = node_id;
        }
    }

    fn merge(&