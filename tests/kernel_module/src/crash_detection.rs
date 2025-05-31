//! Crash Detection and Recovery Module for VexFS Testing
//! 
//! This module provides comprehensive crash detection, monitoring, and automated
//! recovery capabilities for VexFS kernel module testing in VM environments.

use std::process::{Command, Stdio, Child};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, BufWriter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashEvent {
    pub timestamp: SystemTime,
    pub event_type: CrashEventType,
    pub severity: CrashSeverity,
    pub description: String,
    pub kernel_messages: Vec<String>,
    pub recovery_attempted: bool,
    pub recovery_successful: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrashEventType {
    KernelPanic,
    Oops,
    BUG,
    Segfault,
    Hang,
    MemoryLeak,
    ResourceExhaustion,
    ModuleLoadFailure,
    ModuleUnloadFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrashSeverity {
    Critical,  // System crash, requires immediate recovery
    High,      // Serious error, may require recovery
    Medium,    // Error that may affect functionality
    Low,       // Warning or minor issue
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEvent {
    pub timestamp: SystemTime,
    pub operation: String,
    pub duration_ms: u64,
    pub memory_usage_kb: u64,
    pub cpu_usage_percent: f64,
    pub io_operations: u64,
    pub threshold_violated: bool,
    pub violation_type: Option<PerformanceViolationType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceViolationType {
    SlowOperation,
    HighMemoryUsage,
    HighCpuUsage,
    ExcessiveIO,
}

pub struct CrashDetector {
    monitoring_active: Arc<Mutex<bool>>,
    crash_events: Arc<Mutex<VecDeque<CrashEvent>>>,
    performance_events: Arc<Mutex<VecDeque<PerformanceEvent>>>,
    dmesg_monitor: Option<Child>,
    performance_monitor: Option<Child>,
    vm_config: VmMonitorConfig,
    recovery_handler: Option<Box<dyn Fn(&CrashEvent) -> bool + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct VmMonitorConfig {
    pub ssh_key_path: String,
    pub ssh_port: u16,
    pub vm_user: String,
    pub monitoring_interval_ms: u64,
    pub crash_log_path: String,
    pub performance_log_path: String,
    pub max_events_stored: usize,
    pub auto_recovery_enabled: bool,
    pub performance_thresholds: PerformanceThresholds,
}

#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub max_operation_time_ms: u64,
    pub max_memory_usage_kb: u64,
    pub max_cpu_usage_percent: f64,
    pub max_io_operations_per_second: u64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_operation_time_ms: 30000,  // 30 seconds
            max_memory_usage_kb: 2097152,  // 2GB
            max_cpu_usage_percent: 90.0,   // 90%
            max_io_operations_per_second: 10000,
        }
    }
}

impl CrashDetector {
    pub fn new(config: VmMonitorConfig) -> Self {
        Self {
            monitoring_active: Arc::new(Mutex::new(false)),
            crash_events: Arc::new(Mutex::new(VecDeque::new())),
            performance_events: Arc::new(Mutex::new(VecDeque::new())),
            dmesg_monitor: None,
            performance_monitor: None,
            vm_config: config,
            recovery_handler: None,
        }
    }

    pub fn set_recovery_handler<F>(&mut self, handler: F)
    where
        F: Fn(&CrashEvent) -> bool + Send + Sync + 'static,
    {
        self.recovery_handler = Some(Box::new(handler));
    }

    pub fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🛡️  Starting comprehensive crash detection and performance monitoring...");
        
        *self.monitoring_active.lock().unwrap() = true;
        
        // Start dmesg monitoring
        self.start_dmesg_monitoring()?;
        
        // Start performance monitoring
        self.start_performance_monitoring()?;
        
        // Start watchdog thread
        self.start_watchdog_thread();
        
        println!("✅ Crash detection and performance monitoring active");
        Ok(())
    }

    pub fn stop_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🛑 Stopping crash detection and performance monitoring...");
        
        *self.monitoring_active.lock().unwrap() = false;
        
        // Stop dmesg monitoring
        if let Some(mut process) = self.dmesg_monitor.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
        
        // Stop performance monitoring
        if let Some(mut process) = self.performance_monitor.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
        
        // Save final reports
        self.save_crash_report()?;
        self.save_performance_report()?;
        
        println!("✅ Monitoring stopped and reports saved");
        Ok(())
    }

    fn start_dmesg_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ssh_cmd = format!(
            "ssh -i {} -o StrictHostKeyChecking=no {}@localhost -p {} 'dmesg -w'",
            self.vm_config.ssh_key_path,
            self.vm_config.vm_user,
            self.vm_config.ssh_port
        );

        let mut process = Command::new("bash")
            .args(&["-c", &ssh_cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Start thread to process dmesg output
        let crash_events = Arc::clone(&self.crash_events);
        let monitoring_active = Arc::clone(&self.monitoring_active);
        
        if let Some(stdout) = process.stdout.take() {
            let reader = BufReader::new(stdout);
            thread::spawn(move || {
                for line in reader.lines() {
                    if !*monitoring_active.lock().unwrap() {
                        break;
                    }
                    
                    if let Ok(line) = line {
                        if let Some(crash_event) = Self::analyze_dmesg_line(&line) {
                            println!("🚨 Crash detected: {:?}", crash_event.event_type);
                            
                            // Store crash event
                            let mut events = crash_events.lock().unwrap();
                            events.push_back(crash_event);
                            
                            // Limit stored events
                            if events.len() > 100 {
                                events.pop_front();
                            }
                        }
                    }
                }
            });
        }

        self.dmesg_monitor = Some(process);
        Ok(())
    }

    fn start_performance_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let performance_events = Arc::clone(&self.performance_events);
        let monitoring_active = Arc::clone(&self.monitoring_active);
        let config = self.vm_config.clone();
        
        thread::spawn(move || {
            while *monitoring_active.lock().unwrap() {
                if let Ok(perf_event) = Self::collect_performance_metrics(&config) {
                    let mut events = performance_events.lock().unwrap();
                    events.push_back(perf_event);
                    
                    // Limit stored events
                    if events.len() > 1000 {
                        events.pop_front();
                    }
                }
                
                thread::sleep(Duration::from_millis(config.monitoring_interval_ms));
            }
        });
        
        Ok(())
    }

    fn start_watchdog_thread(&self) {
        let monitoring_active = Arc::clone(&self.monitoring_active);
        let crash_events = Arc::clone(&self.crash_events);
        
        thread::spawn(move || {
            let mut last_heartbeat = SystemTime::now();
            let watchdog_timeout = Duration::from_secs(300); // 5 minutes
            
            while *monitoring_active.lock().unwrap() {
                thread::sleep(Duration::from_secs(30)); // Check every 30 seconds
                
                // Check if VM is still responsive
                let ssh_test = Command::new("ssh")
                    .args(&[
                        "-i", "tests/vm_keys/vexfs_test_key",
                        "-o", "StrictHostKeyChecking=no",
                        "-o", "ConnectTimeout=5",
                        "vexfs@localhost",
                        "-p", "2222",
                        "echo 'heartbeat'"
                    ])
                    .output();
                
                match ssh_test {
                    Ok(output) if output.status.success() => {
                        last_heartbeat = SystemTime::now();
                    }
                    _ => {
                        if last_heartbeat.elapsed().unwrap_or(Duration::ZERO) > watchdog_timeout {
                            // VM appears to be hung
                            let hang_event = CrashEvent {
                                timestamp: SystemTime::now(),
                                event_type: CrashEventType::Hang,
                                severity: CrashSeverity::Critical,
                                description: "VM appears to be hung - no response to SSH".to_string(),
                                kernel_messages: vec![],
                                recovery_attempted: false,
                                recovery_successful: false,
                            };
                            
                            println!("🚨 VM hang detected by watchdog");
                            crash_events.lock().unwrap().push_back(hang_event);
                        }
                    }
                }
            }
        });
    }

    fn analyze_dmesg_line(line: &str) -> Option<CrashEvent> {
        let line_lower = line.to_lowercase();
        
        let (event_type, severity) = if line_lower.contains("kernel panic") {
            (CrashEventType::KernelPanic, CrashSeverity::Critical)
        } else if line_lower.contains("oops:") {
            (CrashEventType::Oops, CrashSeverity::High)
        } else if line_lower.contains("bug:") {
            (CrashEventType::BUG, CrashSeverity::High)
        } else if line_lower.contains("segfault") {
            (CrashEventType::Segfault, CrashSeverity::High)
        } else if line_lower.contains("out of memory") {
            (CrashEventType::MemoryLeak, CrashSeverity::Medium)
        } else if line_lower.contains("vexfs") && line_lower.contains("error") {
            (CrashEventType::ModuleLoadFailure, CrashSeverity::Medium)
        } else {
            return None;
        };

        Some(CrashEvent {
            timestamp: SystemTime::now(),
            event_type,
            severity,
            description: line.to_string(),
            kernel_messages: vec![line.to_string()],
            recovery_attempted: false,
            recovery_successful: false,
        })
    }

    fn collect_performance_metrics(config: &VmMonitorConfig) -> Result<PerformanceEvent, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Collect memory usage
        let mem_cmd = Command::new("ssh")
            .args(&[
                "-i", &config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", config.vm_user),
                "-p", &config.ssh_port.to_string(),
                "free -k | grep Mem | awk '{print $3}'"
            ])
            .output()?;

        let memory_usage = String::from_utf8_lossy(&mem_cmd.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        // Collect CPU usage
        let cpu_cmd = Command::new("ssh")
            .args(&[
                "-i", &config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", config.vm_user),
                "-p", &config.ssh_port.to_string(),
                "top -bn1 | grep 'Cpu(s)' | awk '{print $2}' | cut -d'%' -f1"
            ])
            .output()?;

        let cpu_usage = String::from_utf8_lossy(&cpu_cmd.stdout)
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        // Collect IO statistics
        let io_cmd = Command::new("ssh")
            .args(&[
                "-i", &config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", config.vm_user),
                "-p", &config.ssh_port.to_string(),
                "cat /proc/diskstats | awk '{sum+=$4+$8} END {print sum}'"
            ])
            .output()?;

        let io_operations = String::from_utf8_lossy(&io_cmd.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Check for threshold violations
        let mut threshold_violated = false;
        let mut violation_type = None;

        if duration_ms > config.performance_thresholds.max_operation_time_ms {
            threshold_violated = true;
            violation_type = Some(PerformanceViolationType::SlowOperation);
        } else if memory_usage > config.performance_thresholds.max_memory_usage_kb {
            threshold_violated = true;
            violation_type = Some(PerformanceViolationType::HighMemoryUsage);
        } else if cpu_usage > config.performance_thresholds.max_cpu_usage_percent {
            threshold_violated = true;
            violation_type = Some(PerformanceViolationType::HighCpuUsage);
        }

        Ok(PerformanceEvent {
            timestamp: SystemTime::now(),
            operation: "performance_monitoring".to_string(),
            duration_ms,
            memory_usage_kb: memory_usage,
            cpu_usage_percent: cpu_usage,
            io_operations,
            threshold_violated,
            violation_type,
        })
    }

    pub fn get_crash_summary(&self) -> CrashSummary {
        let events = self.crash_events.lock().unwrap();
        let mut summary = CrashSummary::default();
        
        for event in events.iter() {
            summary.total_events += 1;
            
            match event.event_type {
                CrashEventType::KernelPanic => summary.kernel_panics += 1,
                CrashEventType::Oops => summary.oops_count += 1,
                CrashEventType::BUG => summary.bug_count += 1,
                CrashEventType::Hang => summary.hangs += 1,
                CrashEventType::MemoryLeak => summary.memory_leaks += 1,
                _ => summary.other_events += 1,
            }
            
            if event.recovery_attempted {
                summary.recovery_attempts += 1;
                if event.recovery_successful {
                    summary.successful_recoveries += 1;
                }
            }
        }
        
        summary
    }

    pub fn get_performance_summary(&self) -> PerformanceSummary {
        let events = self.performance_events.lock().unwrap();
        let mut summary = PerformanceSummary::default();
        
        if events.is_empty() {
            return summary;
        }
        
        let mut total_duration = 0u64;
        let mut total_memory = 0u64;
        let mut total_cpu = 0f64;
        let mut violations = 0u32;
        
        for event in events.iter() {
            total_duration += event.duration_ms;
            total_memory += event.memory_usage_kb;
            total_cpu += event.cpu_usage_percent;
            
            if event.threshold_violated {
                violations += 1;
            }
        }
        
        let count = events.len() as u64;
        summary.total_measurements = count;
        summary.average_duration_ms = total_duration / count;
        summary.average_memory_usage_kb = total_memory / count;
        summary.average_cpu_usage_percent = total_cpu / count as f64;
        summary.threshold_violations = violations;
        
        summary
    }

    fn save_crash_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let events = self.crash_events.lock().unwrap();
        let file = File::create(&self.vm_config.crash_log_path)?;
        let mut writer = BufWriter::new(file);
        
        for event in events.iter() {
            let json = serde_json::to_string(event)?;
            writeln!(writer, "{}", json)?;
        }
        
        writer.flush()?;
        Ok(())
    }

    fn save_performance_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let events = self.performance_events.lock().unwrap();
        let file = File::create(&self.vm_config.performance_log_path)?;
        let mut writer = BufWriter::new(file);
        
        for event in events.iter() {
            let json = serde_json::to_string(event)?;
            writeln!(writer, "{}", json)?;
        }
        
        writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CrashSummary {
    pub total_events: u32,
    pub kernel_panics: u32,
    pub oops_count: u32,
    pub bug_count: u32,
    pub hangs: u32,
    pub memory_leaks: u32,
    pub other_events: u32,
    pub recovery_attempts: u32,
    pub successful_recoveries: u32,
}

impl std::fmt::Debug for CrashDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrashDetector")
            .field("monitoring_active", &self.monitoring_active)
            .field("crash_events", &self.crash_events)
            .field("performance_events", &self.performance_events)
            .field("vm_config", &self.vm_config)
            .field("recovery_handler", &"<function pointer>")
            .finish()
    }
}

impl Clone for CrashDetector {
    fn clone(&self) -> Self {
        Self {
            monitoring_active: Arc::new(Mutex::new(*self.monitoring_active.lock().unwrap())),
            crash_events: Arc::new(Mutex::new(self.crash_events.lock().unwrap().clone())),
            performance_events: Arc::new(Mutex::new(self.performance_events.lock().unwrap().clone())),
            dmesg_monitor: None, // Don't clone running processes
            performance_monitor: None, // Don't clone running processes
            vm_config: self.vm_config.clone(),
            recovery_handler: None, // Don't clone function pointers
        }
    }
}

#[derive(Debug, Default)]
pub struct PerformanceSummary {
    pub total_measurements: u64,
    pub average_duration_ms: u64,
    pub average_memory_usage_kb: u64,
    pub average_cpu_usage_percent: f64,
    pub threshold_violations: u32,
}

impl CrashSummary {
    pub fn stability_score(&self) -> f64 {
        if self.total_events == 0 {
            return 100.0;
        }
        
        let critical_events = self.kernel_panics + self.hangs;
        let high_severity_events = self.oops_count + self.bug_count;
        
        // Weight critical events more heavily
        let weighted_bad_events = (critical_events * 3) + (high_severity_events * 2) + self.other_events;
        let max_possible_score = self.total_events * 3;
        
        let stability = 100.0 - ((weighted_bad_events as f64 / max_possible_score as f64) * 100.0);
        stability.max(0.0)
    }
}

impl PerformanceSummary {
    pub fn performance_score(&self) -> f64 {
        if self.total_measurements == 0 {
            return 100.0;
        }
        
        let violation_rate = self.threshold_violations as f64 / self.total_measurements as f64;
        let performance_score = 100.0 - (violation_rate * 100.0);
        performance_score.max(0.0)
    }
}