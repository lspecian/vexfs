//! Stack Safety Monitor for HNSW Operations
//! 
//! This module provides comprehensive stack usage monitoring and overflow prevention
//! for HNSW operations in FUSE userspace context with 8KB stack limits.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::VecDeque;
use crate::shared::errors::{VexfsError, VexfsResult};

/// Maximum safe stack usage in FUSE context (6KB safety limit)
pub const MAX_SAFE_STACK_USAGE: usize = 6 * 1024;

/// Critical stack usage threshold (7KB - emergency limit)
pub const CRITICAL_STACK_THRESHOLD: usize = 7 * 1024;

/// Stack usage sample for monitoring
#[derive(Debug, Clone)]
pub struct StackUsageSample {
    pub operation: String,
    pub estimated_usage: usize,
    pub timestamp: std::time::Instant,
    pub call_depth: usize,
}

/// Stack safety monitor for HNSW operations
#[derive(Debug)]
pub struct StackSafetyMonitor {
    current_usage: AtomicUsize,
    max_observed: AtomicUsize,
    samples: Arc<std::sync::Mutex<VecDeque<StackUsageSample>>>,
    max_samples: usize,
}

impl StackSafetyMonitor {
    /// Create new stack safety monitor
    pub fn new() -> Self {
        Self {
            current_usage: AtomicUsize::new(0),
            max_observed: AtomicUsize::new(0),
            samples: Arc::new(std::sync::Mutex::new(VecDeque::new())),
            max_samples: 1000,
        }
    }

    /// Check stack usage before operation
    pub fn check_usage(&self, estimated_additional: usize) -> VexfsResult<()> {
        let current = self.current_usage.load(Ordering::Relaxed);
        let projected = current + estimated_additional;

        // Update max observed
        let max_obs = self.max_observed.load(Ordering::Relaxed);
        if projected > max_obs {
            self.max_observed.store(projected, Ordering::Relaxed);
        }

        // Check against critical threshold
        if projected > CRITICAL_STACK_THRESHOLD {
            return Err(VexfsError::StackOverflow);
        }

        // Warn if approaching safety limit
        if projected > MAX_SAFE_STACK_USAGE {
            eprintln!("⚠️  Stack usage approaching limit: {} bytes (limit: {} bytes)", 
                     projected, MAX_SAFE_STACK_USAGE);
        }

        Ok(())
    }

    /// Enter operation scope with stack tracking
    pub fn enter_operation(&self, operation: &str, estimated_usage: usize) -> VexfsResult<StackGuard> {
        self.check_usage(estimated_usage)?;
        
        let previous = self.current_usage.fetch_add(estimated_usage, Ordering::Relaxed);
        
        // Record sample
        self.record_sample(operation, estimated_usage, 0);
        
        Ok(StackGuard {
            monitor: self,
            usage: estimated_usage,
        })
    }

    /// Record stack usage sample
    fn record_sample(&self, operation: &str, usage: usize, call_depth: usize) {
        let sample = StackUsageSample {
            operation: operation.to_string(),
            estimated_usage: usage,
            timestamp: std::time::Instant::now(),
            call_depth,
        };

        if let Ok(mut samples) = self.samples.lock() {
            samples.push_back(sample);
            
            // Keep only recent samples
            while samples.len() > self.max_samples {
                samples.pop_front();
            }
        }
    }

    /// Get current stack usage
    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Get maximum observed stack usage
    pub fn max_observed(&self) -> usize {
        self.max_observed.load(Ordering::Relaxed)
    }

    /// Get stack usage statistics
    pub fn get_statistics(&self) -> StackStatistics {
        let current = self.current_usage();
        let max_observed = self.max_observed();
        
        let (sample_count, avg_usage) = if let Ok(samples) = self.samples.lock() {
            let count = samples.len();
            let avg = if count > 0 {
                samples.iter().map(|s| s.estimated_usage).sum::<usize>() / count
            } else {
                0
            };
            (count, avg)
        } else {
            (0, 0)
        };

        StackStatistics {
            current_usage: current,
            max_observed,
            average_usage: avg_usage,
            sample_count,
            safety_margin: MAX_SAFE_STACK_USAGE.saturating_sub(max_observed),
            is_safe: max_observed <= MAX_SAFE_STACK_USAGE,
        }
    }

    /// Reset monitoring state
    pub fn reset(&self) {
        self.current_usage.store(0, Ordering::Relaxed);
        self.max_observed.store(0, Ordering::Relaxed);
        if let Ok(mut samples) = self.samples.lock() {
            samples.clear();
        }
    }
}

impl Default for StackSafetyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for stack usage tracking
pub struct StackGuard<'a> {
    monitor: &'a StackSafetyMonitor,
    usage: usize,
}

impl<'a> Drop for StackGuard<'a> {
    fn drop(&mut self) {
        self.monitor.current_usage.fetch_sub(self.usage, Ordering::Relaxed);
    }
}

/// Stack usage statistics
#[derive(Debug, Clone)]
pub struct StackStatistics {
    pub current_usage: usize,
    pub max_observed: usize,
    pub average_usage: usize,
    pub sample_count: usize,
    pub safety_margin: usize,
    pub is_safe: bool,
}

impl StackStatistics {
    /// Check if stack usage is within safe limits
    pub fn is_within_limits(&self) -> bool {
        self.max_observed <= MAX_SAFE_STACK_USAGE
    }

    /// Get safety percentage (0-100)
    pub fn safety_percentage(&self) -> f64 {
        if self.max_observed == 0 {
            100.0
        } else {
            ((MAX_SAFE_STACK_USAGE as f64 - self.max_observed as f64) / MAX_SAFE_STACK_USAGE as f64) * 100.0
        }
    }
}

/// Stack-safe iterator for HNSW operations
pub struct StackSafeIterator<T> {
    items: VecDeque<T>,
    monitor: Arc<StackSafetyMonitor>,
    operation_name: String,
    per_item_cost: usize,
}

impl<T> StackSafeIterator<T> {
    /// Create new stack-safe iterator
    pub fn new(
        items: impl IntoIterator<Item = T>,
        monitor: Arc<StackSafetyMonitor>,
        operation_name: String,
        per_item_cost: usize,
    ) -> Self {
        Self {
            items: items.into_iter().collect(),
            monitor,
            operation_name,
            per_item_cost,
        }
    }

    /// Process items with stack safety checks
    pub fn process_with_safety<F, R>(&mut self, mut processor: F) -> VexfsResult<Vec<R>>
    where
        F: FnMut(T) -> VexfsResult<R>,
    {
        let mut results = Vec::new();
        
        while let Some(item) = self.items.pop_front() {
            // Check stack safety before processing each item
            let _guard = self.monitor.enter_operation(&self.operation_name, self.per_item_cost)?;
            
            let result = processor(item)?;
            results.push(result);
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_monitor_basic() {
        let monitor = StackSafetyMonitor::new();
        
        assert_eq!(monitor.current_usage(), 0);
        assert_eq!(monitor.max_observed(), 0);
        
        // Test entering operation
        {
            let _guard = monitor.enter_operation("test", 1024).unwrap();
            assert_eq!(monitor.current_usage(), 1024);
        }
        
        // Guard should clean up
        assert_eq!(monitor.current_usage(), 0);
        assert_eq!(monitor.max_observed(), 1024);
    }

    #[test]
    fn test_stack_overflow_detection() {
        let monitor = StackSafetyMonitor::new();
        
        // This should fail - exceeds critical threshold
        let result = monitor.enter_operation("overflow_test", CRITICAL_STACK_THRESHOLD + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_statistics() {
        let monitor = StackSafetyMonitor::new();
        
        {
            let _guard1 = monitor.enter_operation("test1", 1024).unwrap();
            {
                let _guard2 = monitor.enter_operation("test2", 2048).unwrap();
                // Peak usage should be 3072
            }
        }
        
        let stats = monitor.get_statistics();
        assert_eq!(stats.max_observed, 3072);
        assert!(stats.is_within_limits());
        assert!(stats.safety_percentage() > 50.0);
    }

    #[test]
    fn test_stack_safe_iterator() {
        let monitor = Arc::new(StackSafetyMonitor::new());
        let items = vec![1, 2, 3, 4, 5];
        
        let mut iterator = StackSafeIterator::new(
            items,
            monitor.clone(),
            "test_iteration".to_string(),
            100,
        );
        
        let results = iterator.process_with_safety(|x| Ok(x * 2)).unwrap();
        assert_eq!(results, vec![2, 4, 6, 8, 10]);
        
        let stats = monitor.get_statistics();
        assert!(stats.is_within_limits());
    }
}