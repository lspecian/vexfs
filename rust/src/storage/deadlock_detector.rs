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

//! Deadlock Detection and Resolution
//!
//! This module implements sophisticated deadlock detection using wait-for graphs
//! and provides multiple resolution strategies to maintain system liveness
//! while preserving ACID properties.

extern crate alloc;
use alloc::{vec::Vec, collections::{BTreeMap, BTreeSet, VecDeque}, string::String, format};
use core::{sync::atomic::{AtomicU64, AtomicU32, Ordering}, cmp::Ordering as CmpOrdering};

use crate::shared::{
    errors::{VexfsError, VexfsResult, TransactionErrorKind},
    types::*,
    constants::*,
};
use crate::fs_core::locking::{LockScope, LockType, LockId};

/// Deadlock detection strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlockDetectionStrategy {
    /// No deadlock detection
    None,
    /// Timeout-based detection
    Timeout,
    /// Wait-for graph cycle detection
    WaitForGraph,
    /// Hybrid approach combining multiple strategies
    Hybrid,
}

/// Deadlock resolution strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlockResolutionStrategy {
    /// Abort youngest transaction
    AbortYoungest,
    /// Abort oldest transaction
    AbortOldest,
    /// Abort transaction with lowest priority
    AbortLowestPriority,
    /// Abort transaction with fewest locks
    AbortFewestLocks,
    /// Abort transaction with least work done
    AbortLeastWork,
}

/// Wait-for graph edge representing a dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitForEdge {
    /// Transaction that is waiting
    pub waiting_transaction: TransactionId,
    /// Transaction that is being waited for
    pub blocking_transaction: TransactionId,
    /// Resource being contended
    pub resource: LockScope,
    /// Lock type being requested
    pub requested_lock_type: LockType,
    /// Timestamp when wait started
    pub wait_start_time: u64,
    /// Priority of the waiting transaction
    pub waiting_priority: u32,
}

impl WaitForEdge {
    pub fn new(
        waiting_transaction: TransactionId,
        blocking_transaction: TransactionId,
        resource: LockScope,
        requested_lock_type: LockType,
        wait_start_time: u64,
        waiting_priority: u32,
    ) -> Self {
        Self {
            waiting_transaction,
            blocking_transaction,
            resource,
            requested_lock_type,
            wait_start_time,
            waiting_priority,
        }
    }
}

/// Wait-for graph for deadlock detection
#[derive(Debug, Clone)]
pub struct WaitForGraph {
    /// Edges in the wait-for graph
    pub edges: Vec<WaitForEdge>,
    /// Adjacency list for efficient traversal
    pub adjacency_list: BTreeMap<TransactionId, Vec<TransactionId>>,
    /// Reverse adjacency list
    pub reverse_adjacency_list: BTreeMap<TransactionId, Vec<TransactionId>>,
    /// Transaction metadata
    pub transaction_metadata: BTreeMap<TransactionId, TransactionMetadata>,
}

/// Transaction metadata for deadlock resolution
#[derive(Debug, Clone)]
pub struct TransactionMetadata {
    /// Transaction start time
    pub start_time: u64,
    /// Transaction priority
    pub priority: u32,
    /// Number of locks held
    pub locks_held: u32,
    /// Amount of work done (operations performed)
    pub work_done: u32,
    /// Transaction timeout
    pub timeout_ms: u64,
}

impl WaitForGraph {
    /// Create new wait-for graph
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            adjacency_list: BTreeMap::new(),
            reverse_adjacency_list: BTreeMap::new(),
            transaction_metadata: BTreeMap::new(),
        }
    }

    /// Add edge to wait-for graph
    pub fn add_edge(&mut self, edge: WaitForEdge) {
        // Add to adjacency lists
        self.adjacency_list
            .entry(edge.waiting_transaction)
            .or_insert_with(Vec::new)
            .push(edge.blocking_transaction);
        
        self.reverse_adjacency_list
            .entry(edge.blocking_transaction)
            .or_insert_with(Vec::new)
            .push(edge.waiting_transaction);
        
        self.edges.push(edge);
    }

    /// Remove edge from wait-for graph
    pub fn remove_edge(&mut self, waiting_tx: TransactionId, blocking_tx: TransactionId) {
        // Remove from edges
        self.edges.retain(|edge| {
            !(edge.waiting_transaction == waiting_tx && edge.blocking_transaction == blocking_tx)
        });
        
        // Remove from adjacency lists
        if let Some(adj_list) = self.adjacency_list.get_mut(&waiting_tx) {
            adj_list.retain(|&tx| tx != blocking_tx);
            if adj_list.is_empty() {
                self.adjacency_list.remove(&waiting_tx);
            }
        }
        
        if let Some(rev_adj_list) = self.reverse_adjacency_list.get_mut(&blocking_tx) {
            rev_adj_list.retain(|&tx| tx != waiting_tx);
            if rev_adj_list.is_empty() {
                self.reverse_adjacency_list.remove(&blocking_tx);
            }
        }
    }

    /// Remove all edges involving a transaction
    pub fn remove_transaction(&mut self, transaction_id: TransactionId) {
        // Collect edges to remove
        let edges_to_remove: Vec<(TransactionId, TransactionId)> = self.edges
            .iter()
            .filter(|edge| {
                edge.waiting_transaction == transaction_id || edge.blocking_transaction == transaction_id
            })
            .map(|edge| (edge.waiting_transaction, edge.blocking_transaction))
            .collect();
        
        // Remove edges
        for (waiting_tx, blocking_tx) in edges_to_remove {
            self.remove_edge(waiting_tx, blocking_tx);
        }
        
        // Remove metadata
        self.transaction_metadata.remove(&transaction_id);
    }

    /// Detect cycles in wait-for graph using DFS
    pub fn detect_cycles(&self) -> Vec<Vec<TransactionId>> {
        let mut cycles = Vec::new();
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();
        let mut path = Vec::new();
        
        for &transaction_id in self.adjacency_list.keys() {
            if !visited.contains(&transaction_id) {
                self.dfs_detect_cycle(
                    transaction_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }
        
        cycles
    }

    /// DFS helper for cycle detection
    fn dfs_detect_cycle(
        &self,
        transaction_id: TransactionId,
        visited: &mut BTreeSet<TransactionId>,
        rec_stack: &mut BTreeSet<TransactionId>,
        path: &mut Vec<TransactionId>,
        cycles: &mut Vec<Vec<TransactionId>>,
    ) {
        visited.insert(transaction_id);
        rec_stack.insert(transaction_id);
        path.push(transaction_id);
        
        if let Some(neighbors) = self.adjacency_list.get(&transaction_id) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    self.dfs_detect_cycle(neighbor, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(&neighbor) {
                    // Found a cycle
                    if let Some(cycle_start) = path.iter().position(|&tx| tx == neighbor) {
                        let cycle = path[cycle_start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }
        
        path.pop();
        rec_stack.remove(&transaction_id);
    }

    /// Add transaction metadata
    pub fn add_transaction_metadata(&mut self, transaction_id: TransactionId, metadata: TransactionMetadata) {
        self.transaction_metadata.insert(transaction_id, metadata);
    }

    /// Get transaction metadata
    pub fn get_transaction_metadata(&self, transaction_id: TransactionId) -> Option<&TransactionMetadata> {
        self.transaction_metadata.get(&transaction_id)
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Get number of transactions in graph
    pub fn transaction_count(&self) -> usize {
        let mut transactions = BTreeSet::new();
        for edge in &self.edges {
            transactions.insert(edge.waiting_transaction);
            transactions.insert(edge.blocking_transaction);
        }
        transactions.len()
    }
}

/// Deadlock detector
pub struct DeadlockDetector {
    /// Wait-for graph
    wait_for_graph: WaitForGraph,
    /// Detection strategy
    detection_strategy: DeadlockDetectionStrategy,
    /// Resolution strategy
    resolution_strategy: DeadlockResolutionStrategy,
    /// Detection statistics
    stats: DeadlockStats,
    /// Detection interval (milliseconds)
    detection_interval_ms: u64,
    /// Timeout threshold (milliseconds)
    timeout_threshold_ms: u64,
    /// Last detection run timestamp
    last_detection_run: AtomicU64,
}

/// Deadlock detection statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct DeadlockStats {
    /// Total deadlocks detected
    pub deadlocks_detected: u64,
    /// Total deadlocks resolved
    pub deadlocks_resolved: u64,
    /// Transactions aborted due to deadlock
    pub transactions_aborted: u64,
    /// Detection runs performed
    pub detection_runs: u64,
    /// Average detection time (microseconds)
    pub avg_detection_time_us: u64,
    /// Current wait-for graph size
    pub current_graph_size: u32,
    /// Maximum graph size seen
    pub max_graph_size: u32,
    /// False positive detections
    pub false_positives: u64,
}

impl DeadlockDetector {
    /// Create new deadlock detector
    pub fn new(
        detection_strategy: DeadlockDetectionStrategy,
        resolution_strategy: DeadlockResolutionStrategy,
    ) -> Self {
        Self {
            wait_for_graph: WaitForGraph::new(),
            detection_strategy,
            resolution_strategy,
            stats: DeadlockStats::default(),
            detection_interval_ms: 1000, // 1 second
            timeout_threshold_ms: 30000, // 30 seconds
            last_detection_run: AtomicU64::new(0),
        }
    }

    /// Add wait relationship to graph
    pub fn add_wait_relationship(
        &mut self,
        waiting_transaction: TransactionId,
        blocking_transaction: TransactionId,
        resource: LockScope,
        requested_lock_type: LockType,
        waiting_priority: u32,
    ) -> VexfsResult<()> {
        let current_time = get_current_timestamp();
        
        let edge = WaitForEdge::new(
            waiting_transaction,
            blocking_transaction,
            resource,
            requested_lock_type,
            current_time,
            waiting_priority,
        );
        
        self.wait_for_graph.add_edge(edge);
        
        // Update statistics
        let graph_size = self.wait_for_graph.transaction_count() as u32;
        self.stats.current_graph_size = graph_size;
        if graph_size > self.stats.max_graph_size {
            self.stats.max_graph_size = graph_size;
        }
        
        Ok(())
    }

    /// Remove wait relationship from graph
    pub fn remove_wait_relationship(
        &mut self,
        waiting_transaction: TransactionId,
        blocking_transaction: TransactionId,
    ) {
        self.wait_for_graph.remove_edge(waiting_transaction, blocking_transaction);
        self.stats.current_graph_size = self.wait_for_graph.transaction_count() as u32;
    }

    /// Remove transaction from deadlock detection
    pub fn remove_transaction(&mut self, transaction_id: TransactionId) {
        self.wait_for_graph.remove_transaction(transaction_id);
        self.stats.current_graph_size = self.wait_for_graph.transaction_count() as u32;
    }

    /// Add transaction metadata for deadlock resolution
    pub fn add_transaction_metadata(
        &mut self,
        transaction_id: TransactionId,
        start_time: u64,
        priority: u32,
        locks_held: u32,
        work_done: u32,
        timeout_ms: u64,
    ) {
        let metadata = TransactionMetadata {
            start_time,
            priority,
            locks_held,
            work_done,
            timeout_ms,
        };
        
        self.wait_for_graph.add_transaction_metadata(transaction_id, metadata);
    }

    /// Run deadlock detection
    pub fn detect_deadlocks(&mut self) -> VexfsResult<Vec<TransactionId>> {
        let start_time = get_current_timestamp();
        self.stats.detection_runs += 1;
        
        let victims = match self.detection_strategy {
            DeadlockDetectionStrategy::None => Vec::new(),
            DeadlockDetectionStrategy::Timeout => self.detect_timeout_deadlocks()?,
            DeadlockDetectionStrategy::WaitForGraph => self.detect_graph_deadlocks()?,
            DeadlockDetectionStrategy::Hybrid => self.detect_hybrid_deadlocks()?,
        };
        
        // Update detection time statistics
        let detection_time = get_current_timestamp() - start_time;
        self.update_avg_detection_time(detection_time);
        
        self.last_detection_run.store(start_time, Ordering::Relaxed);
        
        if !victims.is_empty() {
            self.stats.deadlocks_detected += 1;
            self.stats.transactions_aborted += victims.len() as u64;
        }
        
        Ok(victims)
    }

    /// Detect deadlocks using timeout strategy
    fn detect_timeout_deadlocks(&mut self) -> VexfsResult<Vec<TransactionId>> {
        let current_time = get_current_timestamp();
        let mut victims = Vec::new();
        
        for edge in &self.wait_for_graph.edges {
            let wait_time = current_time - edge.wait_start_time;
            
            if wait_time > self.timeout_threshold_ms {
                // Check if transaction has timed out
                if let Some(metadata) = self.wait_for_graph.get_transaction_metadata(edge.waiting_transaction) {
                    let total_time = current_time - metadata.start_time;
                    if total_time > metadata.timeout_ms {
                        victims.push(edge.waiting_transaction);
                    }
                }
            }
        }
        
        victims.sort_unstable();
        victims.dedup();
        Ok(victims)
    }

    /// Detect deadlocks using wait-for graph cycle detection
    fn detect_graph_deadlocks(&mut self) -> VexfsResult<Vec<TransactionId>> {
        let cycles = self.wait_for_graph.detect_cycles();
        let mut victims = Vec::new();
        
        for cycle in cycles {
            if cycle.len() > 1 {
                // Select victim based on resolution strategy
                if let Some(victim) = self.select_deadlock_victim(&cycle) {
                    victims.push(victim);
                }
            }
        }
        
        Ok(victims)
    }

    /// Detect deadlocks using hybrid strategy
    fn detect_hybrid_deadlocks(&mut self) -> VexfsResult<Vec<TransactionId>> {
        let mut victims = Vec::new();
        
        // First, check for timeout-based deadlocks
        let timeout_victims = self.detect_timeout_deadlocks()?;
        victims.extend(timeout_victims);
        
        // Then, check for cycle-based deadlocks
        let cycle_victims = self.detect_graph_deadlocks()?;
        victims.extend(cycle_victims);
        
        victims.sort_unstable();
        victims.dedup();
        Ok(victims)
    }

    /// Select victim transaction for deadlock resolution
    fn select_deadlock_victim(&self, cycle: &[TransactionId]) -> Option<TransactionId> {
        if cycle.is_empty() {
            return None;
        }
        
        match self.resolution_strategy {
            DeadlockResolutionStrategy::AbortYoungest => {
                self.select_youngest_transaction(cycle)
            }
            DeadlockResolutionStrategy::AbortOldest => {
                self.select_oldest_transaction(cycle)
            }
            DeadlockResolutionStrategy::AbortLowestPriority => {
                self.select_lowest_priority_transaction(cycle)
            }
            DeadlockResolutionStrategy::AbortFewestLocks => {
                self.select_fewest_locks_transaction(cycle)
            }
            DeadlockResolutionStrategy::AbortLeastWork => {
                self.select_least_work_transaction(cycle)
            }
        }
    }

    /// Select youngest transaction in cycle
    fn select_youngest_transaction(&self, cycle: &[TransactionId]) -> Option<TransactionId> {
        cycle.iter()
            .filter_map(|&tx_id| {
                self.wait_for_graph.get_transaction_metadata(tx_id)
                    .map(|metadata| (tx_id, metadata.start_time))
            })
            .max_by_key(|(_, start_time)| *start_time)
            .map(|(tx_id, _)| tx_id)
    }

    /// Select oldest transaction in cycle
    fn select_oldest_transaction(&self, cycle: &[TransactionId]) -> Option<TransactionId> {
        cycle.iter()
            .filter_map(|&tx_id| {
                self.wait_for_graph.get_transaction_metadata(tx_id)
                    .map(|metadata| (tx_id, metadata.start_time))
            })
            .min_by_key(|(_, start_time)| *start_time)
            .map(|(tx_id, _)| tx_id)
    }

    /// Select transaction with lowest priority
    fn select_lowest_priority_transaction(&self, cycle: &[TransactionId]) -> Option<TransactionId> {
        cycle.iter()
            .filter_map(|&tx_id| {
                self.wait_for_graph.get_transaction_metadata(tx_id)
                    .map(|metadata| (tx_id, metadata.priority))
            })
            .min_by_key(|(_, priority)| *priority)
            .map(|(tx_id, _)| tx_id)
    }

    /// Select transaction with fewest locks
    fn select_fewest_locks_transaction(&self, cycle: &[TransactionId]) -> Option<TransactionId> {
        cycle.iter()
            .filter_map(|&tx_id| {
                self.wait_for_graph.get_transaction_metadata(tx_id)
                    .map(|metadata| (tx_id, metadata.locks_held))
            })
            .min_by_key(|(_, locks_held)| *locks_held)
            .map(|(tx_id, _)| tx_id)
    }

    /// Select transaction with least work done
    fn select_least_work_transaction(&self, cycle: &[TransactionId]) -> Option<TransactionId> {
        cycle.iter()
            .filter_map(|&tx_id| {
                self.wait_for_graph.get_transaction_metadata(tx_id)
                    .map(|metadata| (tx_id, metadata.work_done))
            })
            .min_by_key(|(_, work_done)| *work_done)
            .map(|(tx_id, _)| tx_id)
    }

    /// Update average detection time
    fn update_avg_detection_time(&mut self, detection_time: u64) {
        let runs = self.stats.detection_runs;
        if runs == 1 {
            self.stats.avg_detection_time_us = detection_time;
        } else {
            // Running average
            self.stats.avg_detection_time_us = 
                (self.stats.avg_detection_time_us * (runs - 1) + detection_time) / runs;
        }
    }

    /// Check if detection should run based on interval
    pub fn should_run_detection(&self) -> bool {
        let current_time = get_current_timestamp();
        let last_run = self.last_detection_run.load(Ordering::Relaxed);
        
        current_time - last_run >= self.detection_interval_ms
    }

    /// Get deadlock statistics
    pub fn get_stats(&self) -> DeadlockStats {
        self.stats
    }

    /// Set detection interval
    pub fn set_detection_interval(&mut self, interval_ms: u64) {
        self.detection_interval_ms = interval_ms;
    }

    /// Set timeout threshold
    pub fn set_timeout_threshold(&mut self, threshold_ms: u64) {
        self.timeout_threshold_ms = threshold_ms;
    }

    /// Get current wait-for graph
    pub fn get_wait_for_graph(&self) -> &WaitForGraph {
        &self.wait_for_graph
    }
}

/// Helper function to get current timestamp
fn get_current_timestamp() -> u64 {
    // In kernel context, would use appropriate kernel time functions
    static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(1);
    TIMESTAMP_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_core::locking::LockScope;

    #[test]
    fn test_wait_for_graph_creation() {
        let graph = WaitForGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.transaction_count(), 0);
    }

    #[test]
    fn test_wait_for_graph_add_edge() {
        let mut graph = WaitForGraph::new();
        let edge = WaitForEdge::new(
            1, 2, LockScope::Inode(100), LockType::Write, 1000, 1
        );
        
        graph.add_edge(edge);
        assert!(!graph.is_empty());
        assert_eq!(graph.transaction_count(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = WaitForGraph::new();
        
        // Create a cycle: 1 -> 2 -> 3 -> 1
        graph.add_edge(WaitForEdge::new(1, 2, LockScope::Inode(100), LockType::Write, 1000, 1));
        graph.add_edge(WaitForEdge::new(2, 3, LockScope::Inode(101), LockType::Write, 1001, 1));
        graph.add_edge(WaitForEdge::new(3, 1, LockScope::Inode(102), LockType::Write, 1002, 1));
        
        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
        
        // Should detect the cycle
        let cycle = &cycles[0];
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn test_deadlock_detector_creation() {
        let detector = DeadlockDetector::new(
            DeadlockDetectionStrategy::WaitForGraph,
            DeadlockResolutionStrategy::AbortYoungest,
        );
        
        assert_eq!(detector.detection_strategy, DeadlockDetectionStrategy::WaitForGraph);
        assert_eq!(detector.resolution_strategy, DeadlockResolutionStrategy::AbortYoungest);
    }

    #[test]
    fn test_deadlock_detection() {
        let mut detector = DeadlockDetector::new(
            DeadlockDetectionStrategy::WaitForGraph,
            DeadlockResolutionStrategy::AbortYoungest,
        );
        
        // Add transaction metadata
        detector.add_transaction_metadata(1, 1000, 1, 1, 0, 30000);
        detector.add_transaction_metadata(2, 1001, 1, 1, 0, 30000);
        detector.add_transaction_metadata(3, 1002, 1, 1, 0, 30000);
        
        // Create deadlock cycle
        detector.add_wait_relationship(1, 2, LockScope::Inode(100), LockType::Write, 1).unwrap();
        detector.add_wait_relationship(2, 3, LockScope::Inode(101), LockType::Write, 1).unwrap();
        detector.add_wait_relationship(3, 1, LockScope::Inode(102), LockType::Write, 1).unwrap();
        
        let victims = detector.detect_deadlocks().unwrap();
        assert!(!victims.is_empty());
    }
}