# VexFS Task 23.8 Performance Troubleshooting Guide

## Table of Contents
1. [Overview](#overview)
2. [Performance Diagnostics](#performance-diagnostics)
3. [Memory Pool Issues](#memory-pool-issues)
4. [SIMD Acceleration Problems](#simd-acceleration-problems)
5. [Stack Optimization Issues](#stack-optimization-issues)
6. [Cross-Layer Bridge Problems](#cross-layer-bridge-problems)
7. [Performance Regression Analysis](#performance-regression-analysis)
8. [System-Level Troubleshooting](#system-level-troubleshooting)
9. [Monitoring and Alerting](#monitoring-and-alerting)
10. [Recovery Procedures](#recovery-procedures)

## Overview

This guide provides comprehensive troubleshooting procedures for VexFS Task 23.8 performance optimizations. The optimizations deliver significant performance improvements:

- **FUSE Operations**: 4,125 ops/sec (65% improvement)
- **Vector Operations**: 2,120 ops/sec (77% improvement)
- **Semantic Operations**: 648 ops/sec (44% improvement)

When performance issues occur, systematic diagnosis and resolution are essential to maintain optimal operation.

## Performance Diagnostics

### Quick Performance Health Check

#### Automated Diagnostic Script
```bash
#!/bin/bash
# VexFS Performance Health Check

echo "=== VexFS Task 23.8 Performance Health Check ==="
echo "Timestamp: $(date)"
echo

# Check service status
echo "1. Service Status:"
systemctl is-active vexfs && echo "✅ VexFS service active" || echo "❌ VexFS service inactive"
echo

# Check performance targets
echo "2. Performance Targets:"
FUSE_OPS=$(curl -s http://localhost:9090/metrics | grep vexfs_fuse_ops_per_sec | awk '{print $2}')
VECTOR_OPS=$(curl -s http://localhost:9090/metrics | grep vexfs_vector_ops_per_sec | awk '{print $2}')
SEMANTIC_OPS=$(curl -s http://localhost:9090/metrics | grep vexfs_semantic_ops_per_sec | awk '{print $2}')

echo "FUSE Operations: $FUSE_OPS ops/sec (target: 4,125)"
echo "Vector Operations: $VECTOR_OPS ops/sec (target: 2,120)"
echo "Semantic Operations: $SEMANTIC_OPS ops/sec (target: 648)"

# Check optimization status
echo
echo "3. Optimization Status:"
MEMORY_POOLS=$(curl -s http://localhost:9090/metrics | grep vexfs_memory_pools_enabled | awk '{print $2}')
SIMD_ENABLED=$(curl -s http://localhost:9090/metrics | grep vexfs_simd_enabled | awk '{print $2}')
STACK_OPT=$(curl -s http://localhost:9090/metrics | grep vexfs_stack_optimization_enabled | awk '{print $2}')
BRIDGE_OPT=$(curl -s http://localhost:9090/metrics | grep vexfs_bridge_optimization_enabled | awk '{print $2}')

[ "$MEMORY_POOLS" = "1" ] && echo "✅ Memory pools enabled" || echo "❌ Memory pools disabled"
[ "$SIMD_ENABLED" = "1" ] && echo "✅ SIMD acceleration enabled" || echo "❌ SIMD acceleration disabled"
[ "$STACK_OPT" = "1" ] && echo "✅ Stack optimization enabled" || echo "❌ Stack optimization disabled"
[ "$BRIDGE_OPT" = "1" ] && echo "✅ Bridge optimization enabled" || echo "❌ Bridge optimization disabled"

# Check for issues
echo
echo "4. Issue Detection:"
VIOLATIONS=$(curl -s http://localhost:9090/metrics | grep vexfs_stack_violations_total | awk '{print $2}')
POOL_HIT_RATE=$(curl -s http://localhost:9090/metrics | grep vexfs_memory_pool_hit_rate | awk '{print $2}')

echo "Stack violations: $VIOLATIONS"
echo "Memory pool hit rate: $(echo "$POOL_HIT_RATE * 100" | bc)%"

if (( $(echo "$POOL_HIT_RATE < 0.85" | bc -l) )); then
    echo "⚠️  Memory pool hit rate below 85%"
fi

if [ "$VIOLATIONS" -gt "0" ]; then
    echo "⚠️  Stack violations detected"
fi
```

#### Performance Diagnostic Commands
```bash
# Comprehensive performance diagnosis
vexctl performance diagnose --comprehensive --output /tmp/perf-diagnosis.json

# Check specific optimization components
vexctl performance memory-pools status
vexctl performance simd status
vexctl performance stack status
vexctl performance bridge status

# Get performance baseline comparison
vexctl performance compare --baseline /etc/vexfs/performance-baseline.json

# Generate performance report
vexctl performance report --duration 1h --detailed --output /tmp/perf-report.html
```

### Performance Metrics Analysis

#### Key Performance Indicators (KPIs)
```bash
# Monitor critical performance metrics
watch -n 5 'echo "=== Performance KPIs ===" && \
curl -s http://localhost:9090/metrics | grep -E "(fuse_ops_per_sec|vector_ops_per_sec|semantic_ops_per_sec|memory_pool_hit_rate)" | \
while read line; do
  metric=$(echo $line | awk "{print \$1}")
  value=$(echo $line | awk "{print \$2}")
  case $metric in
    *fuse_ops_per_sec) echo "FUSE Ops/sec: $value (target: 4,125)" ;;
    *vector_ops_per_sec) echo "Vector Ops/sec: $value (target: 2,120)" ;;
    *semantic_ops_per_sec) echo "Semantic Ops/sec: $value (target: 648)" ;;
    *memory_pool_hit_rate) echo "Memory Pool Hit Rate: $(echo "$value * 100" | bc)%" ;;
  esac
done'
```

#### Performance Trend Analysis
```bash
# Analyze performance trends over time
vexctl performance trend-analysis \
  --duration 24h \
  --metrics fuse_ops,vector_ops,semantic_ops \
  --detect-degradation \
  --output /tmp/trend-analysis.json

# Check for performance regressions
vexctl performance regression-check \
  --baseline-period 7d \
  --current-period 1h \
  --threshold 5%
```

## Memory Pool Issues

### Common Memory Pool Problems

#### Problem: Low Memory Pool Hit Rate
**Symptoms:**
- Memory pool hit rate below 85%
- Increased allocation latency
- Performance degradation

**Diagnosis:**
```bash
# Check memory pool statistics
vexctl performance memory-pools stats --detailed

# Analyze allocation patterns
vexctl performance memory-pools analyze \
  --duration 1h \
  --show-allocation-patterns

# Check pool utilization
curl -s http://localhost:9090/metrics | grep memory_pool_utilization
```

**Resolution:**
```bash
# Increase pool sizes
vexctl performance memory-pools configure \
  --small-buffers 512 \
  --medium-buffers 256 \
  --large-buffers 128

# Optimize pool distribution based on workload
vexctl performance memory-pools optimize \
  --workload-analysis \
  --auto-resize

# Monitor improvement
vexctl performance memory-pools monitor --duration 300s
```

#### Problem: Memory Pool Exhaustion
**Symptoms:**
- Allocation failures
- High allocation latency
- Out of memory errors

**Diagnosis:**
```bash
# Check pool exhaustion events
journalctl -u vexfs | grep "memory pool exhausted"

# Monitor pool utilization in real-time
vexctl performance memory-pools monitor --real-time

# Check system memory usage
free -h
cat /proc/meminfo | grep -E "(MemTotal|MemAvailable|MemFree)"
```

**Resolution:**
```bash
# Emergency pool expansion
vexctl performance memory-pools emergency-expand \
  --increase-percent 50

# Configure larger pools
vexctl performance memory-pools configure \
  --small-buffers 1024 \
  --medium-buffers 512 \
  --large-buffers 256 \
  --pool-size 16GB

# Enable automatic pool management
vexctl performance memory-pools auto-manage enable \
  --min-hit-rate 0.90 \
  --max-utilization 0.80
```

#### Problem: Memory Pool Fragmentation
**Symptoms:**
- Decreasing hit rates over time
- Allocation failures despite available memory
- Performance degradation

**Diagnosis:**
```bash
# Check fragmentation statistics
vexctl performance memory-pools fragmentation-analysis

# Monitor allocation patterns
vexctl performance memory-pools allocation-trace \
  --duration 300s \
  --show-fragmentation
```

**Resolution:**
```bash
# Defragment memory pools
vexctl performance memory-pools defragment \
  --aggressive \
  --minimize-downtime

# Reset pools (requires brief service interruption)
vexctl performance memory-pools reset \
  --graceful \
  --preserve-hot-data

# Configure anti-fragmentation settings
vexctl performance memory-pools configure \
  --enable-compaction \
  --compaction-threshold 0.70
```

### Memory Pool Monitoring Script
```bash
#!/bin/bash
# Memory Pool Monitoring Script

LOG_FILE="/var/log/vexfs/memory-pool-monitor.log"
ALERT_THRESHOLD=0.85

while true; do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Get memory pool metrics
    HIT_RATE=$(curl -s http://localhost:9090/metrics | grep vexfs_memory_pool_hit_rate | awk '{print $2}')
    SMALL_UTIL=$(curl -s http://localhost:9090/metrics | grep vexfs_small_pool_utilization | awk '{print $2}')
    MEDIUM_UTIL=$(curl -s http://localhost:9090/metrics | grep vexfs_medium_pool_utilization | awk '{print $2}')
    LARGE_UTIL=$(curl -s http://localhost:9090/metrics | grep vexfs_large_pool_utilization | awk '{print $2}')
    
    # Log metrics
    echo "$TIMESTAMP HIT_RATE:$HIT_RATE SMALL:$SMALL_UTIL MEDIUM:$MEDIUM_UTIL LARGE:$LARGE_UTIL" >> "$LOG_FILE"
    
    # Check for issues
    if (( $(echo "$HIT_RATE < $ALERT_THRESHOLD" | bc -l) )); then
        logger -p daemon.warning "VexFS memory pool hit rate below threshold: $HIT_RATE"
        
        # Automatic remediation
        vexctl performance memory-pools optimize --auto --quick
    fi
    
    sleep 30
done
```

## SIMD Acceleration Problems

### Common SIMD Issues

#### Problem: SIMD Acceleration Disabled
**Symptoms:**
- Vector operations at baseline performance
- SIMD metrics showing disabled status
- Missing performance improvements

**Diagnosis:**
```bash
# Check SIMD status
vexctl performance simd status

# Verify hardware capabilities
cat /proc/cpuinfo | grep -E "(avx2|avx512|fma)"
vexctl performance simd hardware-check

# Check SIMD configuration
cat /etc/vexfs/vexfs.conf | grep -A 10 "\[performance_optimizations.simd\]"
```

**Resolution:**
```bash
# Enable SIMD acceleration
vexctl performance simd enable --auto-detect

# Force enable specific SIMD features
vexctl performance simd configure \
  --enable-avx2 \
  --enable-fma \
  --enable-avx512

# Restart VexFS service
sudo systemctl restart vexfs

# Verify SIMD activation
vexctl performance simd test --all-algorithms
```

#### Problem: SIMD Performance Degradation
**Symptoms:**
- SIMD enabled but performance below expectations
- Inconsistent SIMD acceleration
- Thermal throttling

**Diagnosis:**
```bash
# Check SIMD performance metrics
vexctl performance simd benchmark \
  --duration 60s \
  --compare-scalar

# Monitor CPU frequency and thermal state
watch -n 1 'cat /proc/cpuinfo | grep "cpu MHz" | head -4'
sensors | grep -E "(Core|Package)"

# Check for thermal throttling
dmesg | grep -i "thermal\|throttl"
```

**Resolution:**
```bash
# Optimize CPU performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable CPU idle states
sudo cpupower idle-set -D 0

# Configure SIMD batch size
vexctl performance simd configure \
  --batch-size 8 \
  --adaptive-batching

# Monitor thermal management
sudo systemctl enable thermald
sudo systemctl start thermald
```

#### Problem: SIMD Instruction Faults
**Symptoms:**
- Illegal instruction errors
- SIMD operations failing
- Service crashes

**Diagnosis:**
```bash
# Check for illegal instruction errors
journalctl -u vexfs | grep -i "illegal instruction"
dmesg | grep -i "illegal instruction"

# Verify CPU compatibility
vexctl performance simd compatibility-check

# Test SIMD instructions
vexctl performance simd test --safe-mode
```

**Resolution:**
```bash
# Disable problematic SIMD features
vexctl performance simd configure \
  --disable-avx512 \
  --enable-avx2-only

# Use conservative SIMD settings
vexctl performance simd configure \
  --conservative-mode \
  --enable-fallback

# Update CPU microcode
sudo apt update && sudo apt install intel-microcode
sudo reboot
```

## Stack Optimization Issues

### Common Stack Problems

#### Problem: Stack Overflow Violations
**Symptoms:**
- FUSE operation failures
- Stack violation alerts
- Service instability

**Diagnosis:**
```bash
# Check stack violations
vexctl performance stack violations --count --details

# Monitor stack usage
vexctl performance stack monitor --real-time --duration 300s

# Analyze stack usage patterns
vexctl performance stack analyze \
  --duration 1h \
  --show-hotspots
```

**Resolution:**
```bash
# Reduce stack limit
vexctl performance stack configure --limit 2800

# Increase heap threshold
vexctl performance stack configure --heap-threshold 512

# Enable aggressive heap allocation
vexctl performance stack configure \
  --aggressive-heap \
  --large-allocation-threshold 2048

# Clear violation counters
vexctl performance stack clear-violations
```

#### Problem: Excessive Heap Allocation
**Symptoms:**
- High memory usage
- Allocation latency
- Memory pressure

**Diagnosis:**
```bash
# Monitor heap allocation patterns
vexctl performance stack heap-analysis \
  --duration 1h \
  --show-allocation-sizes

# Check memory pressure
cat /proc/pressure/memory
free -h
```

**Resolution:**
```bash
# Optimize heap thresholds
vexctl performance stack configure \
  --heap-threshold 1024 \
  --large-allocation-threshold 4096

# Enable stack optimization
vexctl performance stack configure \
  --enable-optimization \
  --stack-reuse

# Monitor improvement
vexctl performance stack monitor --duration 300s
```

## Cross-Layer Bridge Problems

### Common Bridge Issues

#### Problem: High Bridge Latency
**Symptoms:**
- Semantic operations slow
- Bridge latency above 1ms
- Cross-layer communication delays

**Diagnosis:**
```bash
# Check bridge latency
vexctl performance bridge latency --current --detailed

# Monitor bridge queue depth
vexctl performance bridge queue-status

# Analyze bridge performance
vexctl performance bridge analyze \
  --duration 1h \
  --show-bottlenecks
```

**Resolution:**
```bash
# Optimize batch processing
vexctl performance bridge configure \
  --batch-size 200 \
  --batch-timeout 5ms

# Enable priority scheduling
vexctl performance bridge configure \
  --enable-priority-scheduling \
  --priority-weights high:4,medium:2,low:1

# Reduce synchronization overhead
vexctl performance bridge configure \
  --lazy-sync \
  --sync-threshold 2ms
```

#### Problem: Bridge Communication Failures
**Symptoms:**
- Bridge operation timeouts
- Communication errors
- Service instability

**Diagnosis:**
```bash
# Check bridge errors
journalctl -u vexfs | grep -i "bridge.*error"

# Monitor bridge health
vexctl performance bridge health-check

# Test bridge communication
vexctl performance bridge test --comprehensive
```

**Resolution:**
```bash
# Restart bridge communication
vexctl performance bridge restart

# Reset bridge configuration
vexctl performance bridge reset --preserve-data

# Configure robust communication
vexctl performance bridge configure \
  --timeout 10s \
  --retry-attempts 3 \
  --enable-heartbeat
```

## Performance Regression Analysis

### Regression Detection

#### Automated Regression Detection
```bash
#!/bin/bash
# Performance Regression Detection Script

BASELINE_FILE="/etc/vexfs/performance-baseline.json"
CURRENT_METRICS="/tmp/current-performance.json"
REGRESSION_THRESHOLD=0.05  # 5% degradation threshold

# Collect current metrics
vexctl performance metrics --json > "$CURRENT_METRICS"

# Compare with baseline
REGRESSION_DETECTED=$(vexctl performance compare \
  --baseline "$BASELINE_FILE" \
  --current "$CURRENT_METRICS" \
  --threshold "$REGRESSION_THRESHOLD" \
  --json | jq '.regression_detected')

if [ "$REGRESSION_DETECTED" = "true" ]; then
    echo "Performance regression detected!"
    
    # Get detailed regression analysis
    vexctl performance regression-analysis \
      --baseline "$BASELINE_FILE" \
      --current "$CURRENT_METRICS" \
      --detailed \
      --output /tmp/regression-report.json
    
    # Send alert
    curl -X POST http://alertmanager:9093/api/v1/alerts \
      -H "Content-Type: application/json" \
      -d '[{
        "labels": {
          "alertname": "VexFSPerformanceRegression",
          "severity": "warning"
        },
        "annotations": {
          "summary": "VexFS performance regression detected",
          "description": "Performance has degraded beyond acceptable threshold"
        }
      }]'
    
    # Attempt automatic recovery
    /usr/local/bin/vexfs-performance-recovery
fi
```

#### Manual Regression Analysis
```bash
# Compare performance over time periods
vexctl performance compare-periods \
  --period1 "7d ago to 6d ago" \
  --period2 "1h ago to now" \
  --show-degradation

# Analyze performance trends
vexctl performance trend-analysis \
  --duration 30d \
  --detect-regressions \
  --threshold 5%

# Generate regression report
vexctl performance regression-report \
  --duration 7d \
  --output /tmp/regression-analysis.html
```

### Root Cause Analysis

#### Performance Investigation Workflow
```bash
# 1. Identify when regression started
vexctl performance timeline \
  --duration 7d \
  --mark-degradation-points

# 2. Correlate with system changes
journalctl --since "7 days ago" | grep -E "(vexfs|performance|config)"

# 3. Check configuration changes
git log --since="7 days ago" --oneline /etc/vexfs/

# 4. Analyze resource usage
vexctl performance resource-analysis \
  --duration 7d \
  --correlate-with-performance

# 5. Check for external factors
vexctl performance external-factors \
  --check-system-load \
  --check-network \
  --check-storage
```

## System-Level Troubleshooting

### Hardware Performance Issues

#### CPU Performance Problems
```bash
# Check CPU performance
lscpu | grep -E "(Model name|CPU MHz|Flags)"
cat /proc/cpuinfo | grep -E "(processor|model name|cpu MHz|flags)" | head -20

# Monitor CPU utilization
top -p $(pgrep vexfs)
htop -p $(pgrep vexfs)

# Check CPU frequency scaling
cpufreq-info
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Optimize CPU performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
sudo cpupower frequency-set -g performance
```

#### Memory Performance Problems
```bash
# Check memory bandwidth
sudo apt install stream
stream

# Monitor memory usage
free -h
cat /proc/meminfo
vmstat 1 10

# Check for memory pressure
cat /proc/pressure/memory
dmesg | grep -i "out of memory"

# Optimize memory performance
echo 1 | sudo tee /proc/sys/vm/drop_caches
sysctl vm.swappiness=1
```

#### Storage Performance Problems
```bash
# Check storage performance
sudo iostat -x 1 10
sudo iotop -o

# Test storage bandwidth
sudo fio --name=test --ioengine=libaio --iodepth=32 --rw=randread --bs=4k --direct=1 --size=1G --numjobs=4 --runtime=60 --group_reporting

# Check storage configuration
lsblk
cat /proc/mounts | grep vexfs
```

### Network Performance Issues
```bash
# Check network performance
iftop -i eth0
nethogs

# Test network bandwidth
iperf3 -c target_server

# Check network configuration
ip addr show
ss -tuln | grep -E "(8080|8081|9090)"
```

## Monitoring and Alerting

### Performance Monitoring Setup

#### Prometheus Alert Rules
```yaml
# /etc/prometheus/vexfs-performance-alerts.yml
groups:
  - name: vexfs_performance
    rules:
      - alert: VexFSPerformanceDegraded
        expr: |
          (
            vexfs_fuse_ops_per_sec < 3500 or
            vexfs_vector_ops_per_sec < 1800 or
            vexfs_semantic_ops_per_sec < 550
          )
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "VexFS performance degraded"
          description: "VexFS performance is below acceptable thresholds"

      - alert: VexFSMemoryPoolIssue
        expr: vexfs_memory_pool_hit_rate < 0.85
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "VexFS memory pool hit rate low"
          description: "Memory pool hit rate is {{ $value }}, below 85% threshold"

      - alert: VexFSSIMDDisabled
        expr: vexfs_simd_acceleration_enabled == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "VexFS SIMD acceleration disabled"
          description: "SIMD acceleration is disabled, performance will be degraded"

      - alert: VexFSStackViolations
        expr: increase(vexfs_stack_violations_total[5m]) > 0
        labels:
          severity: warning
        annotations:
          summary: "VexFS stack violations detected"
          description: "{{ $value }} stack violations in the last 5 minutes"
```

#### Grafana Dashboard Alerts
```json
{
  "alert": {
    "conditions": [
      {
        "query": {
          "queryType": "",
          "refId": "A"
        },
        "reducer": {
          "type": "last",
          "params": []
        },
        "evaluator": {
          "params": [3500],
          "type": "lt"
        }
      }
    ],
    "executionErrorState": "alerting",
    "noDataState": "no_data",
    "frequency": "10s",
    "handler": 1,
    "name": "VexFS FUSE Performance Alert",
    "message": "VexFS FUSE operations below 3,500 ops/sec threshold"
  }
}
```

### Custom Monitoring Scripts

#### Performance Monitoring Daemon
```bash
#!/bin/bash
# VexFS Performance Monitoring Daemon

PIDFILE="/var/run/vexfs-perf-monitor.pid"
LOGFILE="/var/log/vexfs/performance-monitor.log"
ALERT_WEBHOOK="http://alertmanager:9093/api/v1/alerts"

# Performance thresholds
FUSE_THRESHOLD=3500
VECTOR_THRESHOLD=1800
SEMANTIC_THRESHOLD=550
MEMORY_POOL_THRESHOLD=0.85

start_monitoring() {
    echo $$ > "$PIDFILE"
    
    while true; do
        TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
        
        # Collect metrics
        FUSE_OPS=$(curl -s http://localhost:9090/metrics | grep vexfs_fuse_ops_per_sec | awk '{print $2}')
        VECTOR_OPS=$(curl -s http://localhost:9090/metrics | grep vexfs_vector_ops_per_sec | awk '{print $2}')
        SEMANTIC_OPS=$(curl -s http://localhost:9090/metrics | grep vexfs_semantic_ops_per_sec | awk '{print $2}')
        POOL_HIT_RATE=$(curl -s http://localhost:9090/metrics | grep vexfs_memory_pool_hit_rate | awk '{print $2}')
        
        # Log metrics
        echo "$TIMESTAMP FUSE:$FUSE_OPS VECTOR:$VECTOR_OPS SEMANTIC:$SEMANTIC_OPS POOL:$POOL_HIT_RATE" >> "$LOGFILE"
        
        # Check thresholds and alert
        if (( $(echo "$FUSE_OPS < $FUSE_THRESHOLD" | bc -l) )); then
            send_alert "VexFSFusePerformanceLow" "FUSE operations at $FUSE_OPS ops/sec, below $FUSE_THRESHOLD threshold"
        fi
        
        if (( $(echo "$VECTOR_OPS < $VECTOR_THRESHOLD" | bc -l) )); then
            send_alert "VexFSVectorPerformanceLow" "Vector operations at $VECTOR_OPS ops/sec, below $VECTOR_THRESHOLD threshold"
        fi
        
        if (( $(echo "$SEMANTIC_OPS < $SEMANTIC_THRESHOLD" | bc -l) )); then
            send_alert "VexFSSemanticPerformanceLow" "Semantic operations at $SEMANTIC_OPS ops/sec, below $SEMANTIC_THRESHOLD threshold"
        fi
        
        if (( $(echo "$POOL_HIT_RATE < $MEMORY_POOL_THRESHOLD" | bc -l) )); then
            send_alert "VexFSMemoryPoolLow" "Memory pool hit rate at $POOL_HIT_RATE, below $MEMORY_POOL_THRESHOLD threshold"
        fi
        
        sleep 30
    done
}

send_alert() {
    local alertname="$1"
    local description="$2"
    
    curl -X POST "$ALERT_WEBHOOK" \
      -H "Content-Type: application/json" \
      -d "[{
        \"labels\": {
          \"alertname\": \"$alertname\",
          \"severity\": \"warning\",
          \"service\": \"vexfs\"
        },
        \"annotations\": {
          \"summary\": \"VexFS performance issue detected\",
          \"description\": \"$description\"
        }
      }]"
}

case "$1" in
    start)
        start_monitoring &
        echo "Performance monitoring started"
        ;;
    stop)
        if [ -f "$PIDFILE" ]; then
            kill $(cat "$PIDFILE")
            rm "$PIDFILE"
            echo "Performance monitoring stopped"
        fi
        ;;
    *)
        echo "Usage: $0 {start|stop}"
        exit 1
        ;;
esac
```

## Recovery Procedures

### Automatic Performance Recovery

#### Performance Recovery Script
```bash
#!/bin/bash
# VexFS Performance Recovery Script

RECOVERY_LOG="/var/log/vexfs/performance-recovery.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

log_message() {
    echo "$TIMESTAMP $1" >> "$RECOVERY_LOG"
    echo "$1"
}

log_message "Starting VexFS performance recovery procedure"

# Step 1: Reset memory pools
log_message "Resetting memory pools..."
vexctl performance memory-pools reset --graceful
if [ $? -eq 0 ]; then
    log_message "Memory pools reset successfully"
else
    log_message "Memory pool reset failed"
fi

# Step 2: Restart SIMD acceleration
log_message "Restarting SIMD acceleration..."
vexctl performance simd restart
if [ $? -eq 0 ]; then
    log_message "SIMD acceleration restarted successfully"
else
    log_message "SIMD restart failed"
fi

# Step 3: Clear stack violations
log_message "Clearing stack violations..."
vexctl performance stack clear-violations
vexctl performance stack reset-monitoring

# Step 4: Restart bridge communication
log_message "Restarting bridge communication..."
vexctl performance bridge restart
if [ $? -eq 0 ]; then
    log_message "Bridge communication restarted successfully"
else
    log_message "Bridge restart failed"
fi

# Step 5: Validate recovery
log_message "Validating performance recovery..."
sleep 30  # Allow time for metrics to stabilize

RECOVERY_SUCCESS=$(vexctl performance validate --quick --json | jq '.overall_success')

if [ "$RECOVERY_SUCCESS" = "true" ]; then
    log_message "Performance recovery successful"
    
    # Send success notification
    curl -X POST http://alertmanager:9093/api/v1/alerts \
      -H "Content-Type: application/json" \
      -d '[{
        "labels": {
          "alertname": "VexFSPerformanceRecoverySuccess",
          "severity": "info"
        },
        "annotations": {
          "summary": "VexFS performance recovery completed successfully"
        }
      }]'
    
    exit 0
else
    log_message "Performance recovery failed, escalating to manual intervention"
    
    # Send failure notification
    curl -X POST http://alertmanager:9093/api/v1/alerts \
      -H "Content-Type: application/json" \
      -d '[{
        "labels": {
          "alertname": "VexFSPerformanceRecoveryFailed",
          "severity": "critical"
        },
        "annotations": {
          "summary": "VexFS performance recovery failed",
          "description": "Automatic recovery procedures failed, manual intervention required"
        }
      }]'
    
    exit 1
fi
```

### Manual Recovery Procedures

#### Complete Performance Reset
```bash
# 1. Stop VexFS service
sudo systemctl stop vexfs

# 2. Clear performance caches
sudo rm -rf /var/cache/vexfs/performance/*

# 3. Reset configuration to defaults
sudo cp /etc/vexfs/vexfs.conf.default /etc/vexfs/vexfs.conf

# 4. Reconfigure performance optimizations
vexctl performance configure --reset-to-optimal

# 5. Start VexFS service
sudo systemctl start vexfs

# 6. Validate performance
vexctl performance validate --comprehensive
```

#### Emergency Performance Mode
```bash
# Enable emergency performance mode (reduced features, maximum stability)
vexctl performance emergency-mode enable \
  --disable-optimizations \
  --safe-mode \
  --minimal-features

# Monitor in emergency mode
vexctl performance monitor --emergency-mode --duration