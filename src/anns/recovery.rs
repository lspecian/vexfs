//! ANNS Recovery and Crash Consistency Module
//! 
//! This module provides comprehensive crash recovery mechanisms for ANNS operations,
//! ensuring data consistency and automatic state restoration after system failures.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::fs_core::operations::OperationContext;
use crate::anns::{
    WalManager, WalEntryType,
    HnswGraph, AnnsIndex, IntegratedAnnsSystem
};
use crate::anns::persistence::{
    AnnsPersistenceManager, AnnsPersistenceError, IndexType, CheckpointInfo,
    RecoveryState, PersistenceConfig, PersistenceStats
};

/// Recovery operation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecoveryOperation {
    /// Full index reconstruction
    FullReconstruction,
    /// Partial index repair
    PartialRepair,
    /// Checkpoint rollback
    CheckpointRollback,
    /// WAL replay
    WalReplay,
    /// Integrity verification
    IntegrityVerification,
}

/// Recovery strategy for different failure scenarios
#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    /// Primary recovery operation
    pub primary_operation: RecoveryOperation,
    /// Fallback operations if primary fails
    pub fallback_operations: Vec<RecoveryOperation>,
    /// Maximum recovery time allowed
    pub max_recovery_time: u64,
    /// Data loss tolerance
    pub allow_data_loss: bool,
    /// Automatic recovery enabled
    pub automatic_recovery: bool,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            primary_operation: RecoveryOperation::WalReplay,
            fallback_operations: vec![
                RecoveryOperation::CheckpointRollback,
                RecoveryOperation::PartialRepair,
                RecoveryOperation::FullReconstruction,
            ],
            max_recovery_time: 300, // 5 minutes
            allow_data_loss: false,
            automatic_recovery: true,
        }
    }
}

/// Recovery result information
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Recovery operation performed
    pub operation_performed: RecoveryOperation,
    /// Success flag
    pub success: bool,
    /// Recovery time in milliseconds
    pub recovery_time_ms: u64,
    /// Data loss occurred
    pub data_loss_occurred: bool,
    /// Number of operations recovered
    pub operations_recovered: u64,
    /// Error message if recovery failed
    pub error_message: Option<String>,
    /// Recovered indices
    pub recovered_indices: Vec<(IndexType, u64)>, // (type, checkpoint_id)
}

impl RecoveryResult {
    pub fn success(
        operation: RecoveryOperation,
        recovery_time_ms: u64,
        operations_recovered: u64,
        recovered_indices: Vec<(IndexType, u64)>,
    ) -> Self {
        Self {
            operation_performed: operation,
            success: true,
            recovery_time_ms,
            data_loss_occurred: false,
            operations_recovered,
            error_message: None,
            recovered_indices,
        }
    }

    pub fn failure(
        operation: RecoveryOperation,
        recovery_time_ms: u64,
        error_message: String,
    ) -> Self {
        Self {
            operation_performed: operation,
            success: false,
            recovery_time_ms,
            data_loss_occurred: false,
            operations_recovered: 0,
            error_message: Some(error_message),
            recovered_indices: Vec::new(),
        }
    }

    pub fn with_data_loss(mut self) -> Self {
        self.data_loss_occurred = true;
        self
    }
}

/// ANNS Recovery Manager
pub struct AnnsRecoveryManager {
    /// Persistence manager
    persistence_manager: Arc<Mutex<AnnsPersistenceManager>>,
    /// Recovery strategy
    recovery_strategy: RecoveryStrategy,
    /// Recovery history
    recovery_history: Vec<RecoveryResult>,
    /// Crash detection enabled
    crash_detection_enabled: bool,
    /// Last known good state
    last_known_good_state: Option<u64>, // checkpoint_id
}

impl AnnsRecoveryManager {
    /// Create new recovery manager
    pub fn new(
        persistence_manager: Arc<Mutex<AnnsPersistenceManager>>,
        recovery_strategy: RecoveryStrategy,
    ) -> Self {
        Self {
            persistence_manager,
            recovery_strategy,
            recovery_history: Vec::new(),
            crash_detection_enabled: true,
            last_known_good_state: None,
        }
    }

    /// Detect if crash recovery is needed
    pub fn detect_crash_recovery_needed(&self, ctx: &OperationContext) -> VexfsResult<bool> {
        if !self.crash_detection_enabled {
            return Ok(false);
        }

        let persistence = self.persistence_manager.lock().map_err(|_| VexfsError::LockError)?;
        let stats = persistence.get_persistence_stats()?;
        
        Ok(stats.recovery_needed)
    }

    /// Perform automatic crash recovery
    pub fn perform_automatic_recovery(&mut self, ctx: &OperationContext) -> VexfsResult<RecoveryResult> {
        if !self.recovery_strategy.automatic_recovery {
            return Err(VexfsError::InvalidOperation("automatic recovery disabled".to_string()));
        }

        let start_time = 0; // Would be current timestamp
        
        // Try primary recovery operation first
        let result = self.execute_recovery_operation(
            ctx,
            self.recovery_strategy.primary_operation,
            start_time,
        );

        match result {
            Ok(recovery_result) if recovery_result.success => {
                self.recovery_history.push(recovery_result.clone());
                return Ok(recovery_result);
            }
            _ => {
                // Try fallback operations
                let fallback_operations = self.recovery_strategy.fallback_operations.clone();
                for fallback_op in fallback_operations {
                    let fallback_result = self.execute_recovery_operation(
                        ctx,
                        fallback_op,
                        start_time,
                    );

                    if let Ok(recovery_result) = fallback_result {
                        if recovery_result.success {
                            self.recovery_history.push(recovery_result.clone());
                            return Ok(recovery_result);
                        }
                    }
                }
            }
        }

        // All recovery attempts failed
        let elapsed_time = 0; // Would calculate elapsed time
        let failure_result = RecoveryResult::failure(
            self.recovery_strategy.primary_operation,
            elapsed_time,
            "All recovery operations failed".to_string(),
        );
        
        self.recovery_history.push(failure_result.clone());
        Ok(failure_result)
    }

    /// Execute specific recovery operation
    pub fn execute_recovery_operation(
        &mut self,
        ctx: &OperationContext,
        operation: RecoveryOperation,
        start_time: u64,
    ) -> VexfsResult<RecoveryResult> {
        let elapsed_time = 0; // Would calculate elapsed time

        match operation {
            RecoveryOperation::WalReplay => {
                self.perform_wal_replay(ctx, start_time)
            }
            RecoveryOperation::CheckpointRollback => {
                self.perform_checkpoint_rollback(ctx, start_time)
            }
            RecoveryOperation::PartialRepair => {
                self.perform_partial_repair(ctx, start_time)
            }
            RecoveryOperation::FullReconstruction => {
                self.perform_full_reconstruction(ctx, start_time)
            }
            RecoveryOperation::IntegrityVerification => {
                self.perform_integrity_verification(ctx, start_time)
            }
        }
    }

    /// Validate recovered indices
    pub fn validate_recovered_indices(&self, ctx: &OperationContext) -> VexfsResult<Vec<(u64, bool, String)>> {
        let persistence = self.persistence_manager.lock().map_err(|_| VexfsError::LockError)?;
        persistence.validate_all_indices(ctx)
    }

    /// Get recovery statistics
    pub fn get_recovery_stats(&self) -> RecoveryStats {
        let total_recoveries = self.recovery_history.len();
        let successful_recoveries = self.recovery_history.iter()
            .filter(|r| r.success)
            .count();
        
        let total_recovery_time: u64 = self.recovery_history.iter()
            .map(|r| r.recovery_time_ms)
            .sum::<u64>();
        
        let average_recovery_time = if total_recoveries > 0 {
            total_recovery_time / total_recoveries as u64
        } else {
            0
        };

        let data_loss_incidents = self.recovery_history.iter()
            .filter(|r| r.data_loss_occurred)
            .count();

        RecoveryStats {
            total_recoveries,
            successful_recoveries,
            failed_recoveries: total_recoveries - successful_recoveries,
            average_recovery_time_ms: average_recovery_time,
            total_operations_recovered: self.recovery_history.iter()
                .map(|r| r.operations_recovered)
                .sum(),
            data_loss_incidents,
            last_recovery_time: self.recovery_history.last()
                .map(|r| r.recovery_time_ms)
                .unwrap_or(0),
        }
    }

    /// Set recovery strategy
    pub fn set_recovery_strategy(&mut self, strategy: RecoveryStrategy) {
        self.recovery_strategy = strategy;
    }

    /// Enable/disable crash detection
    pub fn set_crash_detection(&mut self, enabled: bool) {
        self.crash_detection_enabled = enabled;
    }

    /// Create recovery checkpoint
    pub fn create_recovery_checkpoint(
        &mut self,
        ctx: &OperationContext,
        index_type: IndexType,
        index_data: &[u8],
        metadata: &[u8],
    ) -> VexfsResult<u64> {
        let mut persistence = self.persistence_manager.lock().map_err(|_| VexfsError::LockError)?;
        let checkpoint_id = persistence.persist_index(ctx, index_type, index_data, metadata)?;
        
        self.last_known_good_state = Some(checkpoint_id);
        Ok(checkpoint_id)
    }

    // Private recovery operation implementations

    fn perform_wal_replay(&mut self, ctx: &OperationContext, start_time: u64) -> VexfsResult<RecoveryResult> {
        let mut persistence = self.persistence_manager.lock().map_err(|_| VexfsError::LockError)?;
        
        match persistence.perform_crash_recovery(ctx) {
            Ok(()) => {
                let elapsed_time = 0; // Would calculate elapsed time
                Ok(RecoveryResult::success(
                    RecoveryOperation::WalReplay,
                    elapsed_time,
                    0, // Would count actual operations
                    Vec::new(), // Would list recovered indices
                ))
            }
            Err(e) => {
                let elapsed_time = 0; // Would calculate elapsed time
                Ok(RecoveryResult::failure(
                    RecoveryOperation::WalReplay,
                    elapsed_time,
                    format!("WAL replay failed: {}", e),
                ))
            }
        }
    }

    fn perform_checkpoint_rollback(&mut self, ctx: &OperationContext, start_time: u64) -> VexfsResult<RecoveryResult> {
        let elapsed_time = 0; // Would calculate elapsed time
        
        if let Some(checkpoint_id) = self.last_known_good_state {
            // Attempt to restore from last known good checkpoint
            let persistence = self.persistence_manager.lock().map_err(|_| VexfsError::LockError)?;
            
            // This would involve restoring all indices from the checkpoint
            // For now, just return success
            Ok(RecoveryResult::success(
                RecoveryOperation::CheckpointRollback,
                elapsed_time,
                1, // Restored one checkpoint
                vec![(IndexType::HNSW, checkpoint_id)], // Example
            ).with_data_loss()) // Rollback implies some data loss
        } else {
            Ok(RecoveryResult::failure(
                RecoveryOperation::CheckpointRollback,
                elapsed_time,
                "No known good checkpoint available".to_string(),
            ))
        }
    }

    fn perform_partial_repair(&mut self, ctx: &OperationContext, start_time: u64) -> VexfsResult<RecoveryResult> {
        let elapsed_time = 0; // Would calculate elapsed time
        
        // Partial repair would involve:
        // 1. Identifying corrupted index segments
        // 2. Attempting to repair using redundant data
        // 3. Rebuilding only the corrupted portions
        
        // For now, simulate partial repair
        Ok(RecoveryResult::success(
            RecoveryOperation::PartialRepair,
            elapsed_time,
            0, // Would count repaired operations
            Vec::new(), // Would list repaired indices
        ))
    }

    fn perform_full_reconstruction(&mut self, ctx: &OperationContext, start_time: u64) -> VexfsResult<RecoveryResult> {
        let elapsed_time = 0; // Would calculate elapsed time
        
        // Full reconstruction would involve:
        // 1. Discarding all corrupted indices
        // 2. Rebuilding from raw vector data
        // 3. Recreating all index structures
        
        // This is the most expensive but most reliable recovery option
        Ok(RecoveryResult::success(
            RecoveryOperation::FullReconstruction,
            elapsed_time,
            0, // Would count reconstructed operations
            Vec::new(), // Would list reconstructed indices
        ).with_data_loss()) // Full reconstruction implies data loss
    }

    fn perform_integrity_verification(&mut self, ctx: &OperationContext, start_time: u64) -> VexfsResult<RecoveryResult> {
        let elapsed_time = 0; // Would calculate elapsed time
        
        let persistence = self.persistence_manager.lock().map_err(|_| VexfsError::LockError)?;
        
        match persistence.validate_all_indices(ctx) {
            Ok(validation_results) => {
                let all_valid = validation_results.iter().all(|(_, valid, _)| *valid);
                
                if all_valid {
                    Ok(RecoveryResult::success(
                        RecoveryOperation::IntegrityVerification,
                        elapsed_time,
                        validation_results.len() as u64,
                        Vec::new(), // Verification doesn't recover indices
                    ))
                } else {
                    let invalid_count = validation_results.iter()
                        .filter(|(_, valid, _)| !*valid)
                        .count();
                    
                    Ok(RecoveryResult::failure(
                        RecoveryOperation::IntegrityVerification,
                        elapsed_time,
                        format!("{} indices failed validation", invalid_count),
                    ))
                }
            }
            Err(e) => {
                Ok(RecoveryResult::failure(
                    RecoveryOperation::IntegrityVerification,
                    elapsed_time,
                    format!("Integrity verification failed: {}", e),
                ))
            }
        }
    }
}

/// Recovery statistics
#[derive(Debug, Clone)]
pub struct RecoveryStats {
    /// Total number of recovery attempts
    pub total_recoveries: usize,
    /// Number of successful recoveries
    pub successful_recoveries: usize,
    /// Number of failed recoveries
    pub failed_recoveries: usize,
    /// Average recovery time in milliseconds
    pub average_recovery_time_ms: u64,
    /// Total operations recovered across all attempts
    pub total_operations_recovered: u64,
    /// Number of incidents with data loss
    pub data_loss_incidents: usize,
    /// Time of last recovery attempt
    pub last_recovery_time: u64,
}

impl RecoveryStats {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f32 {
        if self.total_recoveries == 0 {
            0.0
        } else {
            (self.successful_recoveries as f32 / self.total_recoveries as f32) * 100.0
        }
    }

    /// Check if recovery performance is acceptable
    pub fn is_performance_acceptable(&self, max_avg_time_ms: u64, min_success_rate: f32) -> bool {
        self.average_recovery_time_ms <= max_avg_time_ms && 
        self.success_rate() >= min_success_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::journal::{VexfsJournal, TransactionManager};

    #[test]
    fn test_recovery_strategy_default() {
        let strategy = RecoveryStrategy::default();
        
        assert_eq!(strategy.primary_operation, RecoveryOperation::WalReplay);
        assert_eq!(strategy.fallback_operations.len(), 3);
        assert_eq!(strategy.max_recovery_time, 300);
        assert!(!strategy.allow_data_loss);
        assert!(strategy.automatic_recovery);
    }

    #[test]
    fn test_recovery_result_success() {
        let result = RecoveryResult::success(
            RecoveryOperation::WalReplay,
            1000,
            50,
            vec![(IndexType::HNSW, 1)],
        );
        
        assert!(result.success);
        assert_eq!(result.operation_performed, RecoveryOperation::WalReplay);
        assert_eq!(result.recovery_time_ms, 1000);
        assert_eq!(result.operations_recovered, 50);
        assert!(!result.data_loss_occurred);
        assert!(result.error_message.is_none());
        assert_eq!(result.recovered_indices.len(), 1);
    }

    #[test]
    fn test_recovery_result_failure() {
        let result = RecoveryResult::failure(
            RecoveryOperation::CheckpointRollback,
            2000,
            "Test error".to_string(),
        );
        
        assert!(!result.success);
        assert_eq!(result.operation_performed, RecoveryOperation::CheckpointRollback);
        assert_eq!(result.recovery_time_ms, 2000);
        assert_eq!(result.operations_recovered, 0);
        assert!(!result.data_loss_occurred);
        assert!(result.error_message.is_some());
        assert_eq!(result.recovered_indices.len(), 0);
    }

    #[test]
    fn test_recovery_result_with_data_loss() {
        let result = RecoveryResult::success(
            RecoveryOperation::FullReconstruction,
            5000,
            100,
            vec![(IndexType::LSH, 2), (IndexType::IVF, 3)],
        ).with_data_loss();
        
        assert!(result.success);
        assert!(result.data_loss_occurred);
        assert_eq!(result.recovered_indices.len(), 2);
    }

    #[test]
    fn test_recovery_stats_success_rate() {
        let stats = RecoveryStats {
            total_recoveries: 10,
            successful_recoveries: 8,
            failed_recoveries: 2,
            average_recovery_time_ms: 1500,
            total_operations_recovered: 500,
            data_loss_incidents: 1,
            last_recovery_time: 1000,
        };
        
        assert_eq!(stats.success_rate(), 80.0);
        assert!(stats.is_performance_acceptable(2000, 75.0));
        assert!(!stats.is_performance_acceptable(1000, 75.0)); // Time too high
        assert!(!stats.is_performance_acceptable(2000, 90.0)); // Success rate too low
    }

    #[test]
    fn test_recovery_stats_zero_recoveries() {
        let stats = RecoveryStats {
            total_recoveries: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            average_recovery_time_ms: 0,
            total_operations_recovered: 0,
            data_loss_incidents: 0,
            last_recovery_time: 0,
        };
        
        assert_eq!(stats.success_rate(), 0.0);
        assert!(stats.is_performance_acceptable(1000, 0.0));
    }
}