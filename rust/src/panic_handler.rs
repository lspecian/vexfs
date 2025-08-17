// Panic Handler and Recovery System for VexFS
// Provides graceful panic handling and system recovery

use std::panic;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{SystemTime, Duration};
use std::thread;
use serde::{Serialize, Deserialize};

/// Panic information for logging and recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanicInfo {
    pub message: String,
    pub location: Option<String>,
    pub thread: String,
    pub timestamp: SystemTime,
    pub backtrace: Option<String>,
}

/// Panic recovery manager
pub struct PanicRecoveryManager {
    panic_count: Arc<Mutex<usize>>,
    panic_log: Arc<Mutex<Vec<PanicInfo>>>,
    recovery_in_progress: Arc<AtomicBool>,
    max_panics_before_abort: usize,
    recovery_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + 'static>>>>,
}

impl PanicRecoveryManager {
    pub fn new() -> Arc<Self> {
        let manager = Arc::new(Self {
            panic_count: Arc::new(Mutex::new(0)),
            panic_log: Arc::new(Mutex::new(Vec::new())),
            recovery_in_progress: Arc::new(AtomicBool::new(false)),
            max_panics_before_abort: 5,
            recovery_callbacks: Arc::new(Mutex::new(Vec::new())),
        });
        
        // Install the panic handler
        let manager_clone = manager.clone();
        panic::set_hook(Box::new(move |panic_info| {
            manager_clone.handle_panic(panic_info);
        }));
        
        manager
    }
    
    /// Handle a panic
    fn handle_panic(&self, panic_info: &panic::PanicInfo) {
        // Extract panic information
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        let location = panic_info.location().map(|loc| {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        });
        
        let thread = format!("{:?}", thread::current().id());
        
        let panic_data = PanicInfo {
            message: message.clone(),
            location: location.clone(),
            thread,
            timestamp: SystemTime::now(),
            backtrace: Some(format!("{:?}", std::backtrace::Backtrace::capture())),
        };
        
        // Log the panic
        if let Ok(mut log) = self.panic_log.lock() {
            log.push(panic_data);
            
            // Keep only last 100 panics
            if log.len() > 100 {
                log.drain(0..50);
            }
        }
        
        // Increment panic count
        let panic_count = {
            let mut count = self.panic_count.lock().unwrap_or_else(|e| e.into_inner());
            *count += 1;
            *count
        };
        
        eprintln!("=== VexFS PANIC DETECTED ===");
        eprintln!("Message: {}", message);
        if let Some(loc) = location {
            eprintln!("Location: {}", loc);
        }
        eprintln!("Panic count: {}", panic_count);
        eprintln!("===========================");
        
        // Check if we should abort
        if panic_count >= self.max_panics_before_abort {
            eprintln!("Too many panics ({}), aborting!", panic_count);
            std::process::abort();
        }
        
        // Attempt recovery
        self.attempt_recovery();
    }
    
    /// Attempt to recover from panic
    fn attempt_recovery(&self) {
        // Check if recovery is already in progress
        if self.recovery_in_progress.compare_exchange(
            false,
            true,
            Ordering::SeqCst,
            Ordering::SeqCst
        ).is_err() {
            eprintln!("Recovery already in progress, skipping");
            return;
        }
        
        eprintln!("Attempting recovery...");
        
        // Run recovery callbacks
        if let Ok(callbacks) = self.recovery_callbacks.lock() {
            for callback in callbacks.iter() {
                callback();
            }
        }
        
        // Mark recovery as complete
        self.recovery_in_progress.store(false, Ordering::SeqCst);
        eprintln!("Recovery attempt complete");
    }
    
    /// Register a recovery callback
    pub fn register_recovery_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + 'static,
    {
        if let Ok(mut callbacks) = self.recovery_callbacks.lock() {
            callbacks.push(Box::new(callback));
        }
    }
    
    /// Get panic statistics
    pub fn get_panic_stats(&self) -> PanicStats {
        let count = self.panic_count.lock()
            .map(|c| *c)
            .unwrap_or(0);
        
        let recent_panics = self.panic_log.lock()
            .map(|log| log.clone())
            .unwrap_or_default();
        
        PanicStats {
            total_panics: count,
            recent_panics,
            recovery_in_progress: self.recovery_in_progress.load(Ordering::SeqCst),
        }
    }
    
    /// Reset panic count (use after successful recovery)
    pub fn reset_panic_count(&self) {
        if let Ok(mut count) = self.panic_count.lock() {
            *count = 0;
        }
    }
    
    /// Check system health
    pub fn is_healthy(&self) -> bool {
        let count = self.panic_count.lock()
            .map(|c| *c)
            .unwrap_or(0);
        
        count == 0 && !self.recovery_in_progress.load(Ordering::SeqCst)
    }
}

/// Panic statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanicStats {
    pub total_panics: usize,
    pub recent_panics: Vec<PanicInfo>,
    pub recovery_in_progress: bool,
}

/// Guard that catches panics in a specific scope
pub struct PanicGuard {
    name: String,
    manager: Arc<PanicRecoveryManager>,
}

impl PanicGuard {
    pub fn new(name: impl Into<String>, manager: Arc<PanicRecoveryManager>) -> Self {
        Self {
            name: name.into(),
            manager,
        }
    }
    
    /// Execute a function with panic protection
    pub fn execute<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce() -> R + panic::UnwindSafe,
    {
        match panic::catch_unwind(f) {
            Ok(result) => Ok(result),
            Err(err) => {
                let message = if let Some(s) = err.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = err.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                
                eprintln!("Panic caught in {}: {}", self.name, message);
                Err(message)
            }
        }
    }
}

/// Thread-safe wrapper that recovers from poisoned locks
pub struct RecoverableMutex<T> {
    inner: Arc<Mutex<T>>,
    name: String,
}

impl<T> RecoverableMutex<T> {
    pub fn new(value: T, name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            name: name.into(),
        }
    }
    
    /// Lock the mutex, recovering from poison if necessary
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, T>, String> {
        match self.inner.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                eprintln!("Recovering poisoned mutex: {}", self.name);
                Ok(poisoned.into_inner())
            }
        }
    }
    
    /// Try to lock the mutex
    pub fn try_lock(&self) -> Result<std::sync::MutexGuard<'_, T>, String> {
        match self.inner.try_lock() {
            Ok(guard) => Ok(guard),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                eprintln!("Recovering poisoned mutex: {}", self.name);
                Ok(poisoned.into_inner())
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                Err("Mutex is locked".to_string())
            }
        }
    }
}

/// Watchdog timer that monitors for system hangs
pub struct WatchdogTimer {
    last_heartbeat: Arc<Mutex<SystemTime>>,
    timeout: Duration,
    running: Arc<AtomicBool>,
    recovery_callback: Option<Box<dyn Fn() + Send + 'static>>,
}

impl WatchdogTimer {
    pub fn new(timeout: Duration) -> Arc<Self> {
        Arc::new(Self {
            last_heartbeat: Arc::new(Mutex::new(SystemTime::now())),
            timeout,
            running: Arc::new(AtomicBool::new(false)),
            recovery_callback: None,
        })
    }
    
    /// Start the watchdog
    pub fn start(self: Arc<Self>) {
        self.running.store(true, Ordering::SeqCst);
        
        let watchdog = self.clone();
        thread::spawn(move || {
            while watchdog.running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));
                
                if let Ok(last_heartbeat) = watchdog.last_heartbeat.lock() {
                    if let Ok(elapsed) = SystemTime::now().duration_since(*last_heartbeat) {
                        if elapsed > watchdog.timeout {
                            eprintln!("Watchdog timeout! System may be hung.");
                            
                            if let Some(ref callback) = watchdog.recovery_callback {
                                callback();
                            }
                            
                            // Reset heartbeat to avoid repeated triggers
                            drop(last_heartbeat);
                            if let Ok(mut heartbeat) = watchdog.last_heartbeat.lock() {
                                *heartbeat = SystemTime::now();
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// Send a heartbeat to the watchdog
    pub fn heartbeat(&self) {
        if let Ok(mut last_heartbeat) = self.last_heartbeat.lock() {
            *last_heartbeat = SystemTime::now();
        }
    }
    
    /// Stop the watchdog
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_panic_guard() {
        let manager = PanicRecoveryManager::new();
        let guard = PanicGuard::new("test", manager.clone());
        
        // Test successful execution
        let result = guard.execute(|| 42);
        assert_eq!(result.unwrap(), 42);
        
        // Test panic catching
        let result = guard.execute(|| {
            panic!("test panic");
        });
        assert!(result.is_err());
    }
    
    #[test]
    fn test_recoverable_mutex() {
        let mutex = RecoverableMutex::new(42, "test_mutex");
        
        // Normal lock
        {
            let guard = mutex.lock().unwrap();
            assert_eq!(*guard, 42);
        }
        
        // Simulate poisoned lock recovery
        let mutex_clone = mutex.inner.clone();
        let handle = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("poison the lock");
        });
        
        let _ = handle.join();
        
        // Should recover from poison
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 42);
    }
    
    #[test]
    fn test_watchdog() {
        let watchdog = WatchdogTimer::new(Duration::from_millis(100));
        let watchdog_clone = watchdog.clone();
        
        watchdog.start();
        
        // Send heartbeats
        for _ in 0..5 {
            watchdog_clone.heartbeat();
            thread::sleep(Duration::from_millis(50));
        }
        
        watchdog_clone.stop();
    }
}