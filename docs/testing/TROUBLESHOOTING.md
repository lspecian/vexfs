# VexFS Testing Infrastructure - Troubleshooting Guide

**Task 33.10 Implementation**: Comprehensive troubleshooting documentation for the VexFS testing infrastructure

## Table of Contents

1. [Quick Diagnostics](#quick-diagnostics)
2. [Common Issues](#common-issues)
3. [Environment Setup Issues](#environment-setup-issues)
4. [VM Infrastructure Issues](#vm-infrastructure-issues)
5. [Test Execution Issues](#test-execution-issues)
6. [Performance Issues](#performance-issues)
7. [Component-Specific Issues](#component-specific-issues)
8. [Debug Tools and Techniques](#debug-tools-and-techniques)
9. [Log Analysis](#log-analysis)
10. [Recovery Procedures](#recovery-procedures)

## Quick Diagnostics

### System Health Check

Run this comprehensive health check to identify common issues:

```bash
#!/bin/bash
# Quick diagnostic script

echo "🔍 VexFS Testing Infrastructure Health Check"
echo "============================================="

# Check system requirements
echo "📋 System Requirements:"
echo "  RAM: $(free -h | awk '/^Mem:/ {print $2}')"
echo "  Disk: $(df -h . | awk 'NR==2 {print $4}') available"
echo "  CPU cores: $(nproc)"

# Check virtualization support
echo "🖥️  Virtualization Support:"
if lscpu | grep -q "Virtualization"; then
    echo "  ✅ Hardware virtualization supported"
else
    echo "  ❌ Hardware virtualization NOT supported"
fi

if [ -r /dev/kvm ]; then
    echo "  ✅ KVM accessible"
else
    echo "  ❌ KVM not accessible"
fi

# Check dependencies
echo "🔧 Dependencies:"
for cmd in qemu-system-x86_64 python3 cargo rustc; do
    if command -v $cmd >/dev/null 2>&1; then
        echo "  ✅ $cmd: $(command -v $cmd)"
    else
        echo "  ❌ $cmd: NOT FOUND"
    fi
done

# Check Python modules
echo "🐍 Python Modules:"
python3 -c "
import sys
sys.path.append('tests/vm_testing')
modules = ['asyncio', 'aiohttp', 'pytest', 'psutil']
for module in modules:
    try:
        __import__(module)
        print(f'  ✅ {module}: Available')
    except ImportError:
        print(f'  ❌ {module}: Missing')
"

# Check VM status
echo "🖥️  VM Infrastructure:"
if [ -f "tests/vm_testing/manage_alpine_vm.sh" ]; then
    vm_status=$(cd tests/vm_testing && ./manage_alpine_vm.sh status 2>/dev/null || echo "error")
    echo "  VM Status: $vm_status"
else
    echo "  ❌ VM management script not found"
fi

# Check test configuration
echo "⚙️  Configuration:"
if [ -f "tests/vm_testing/config/global_config.json" ]; then
    echo "  ✅ Global config found"
else
    echo "  ❌ Global config missing"
fi

echo "============================================="
echo "Health check complete. See sections below for specific issue resolution."
```

### Quick Fix Commands

```bash
# Reset VM environment
cd tests/vm_testing
./manage_alpine_vm.sh stop
./manage_alpine_vm.sh start

# Rebuild Rust components
cargo clean && cargo build --release

# Reset Python environment
pip3 install --user --force-reinstall -r tests/vm_testing/requirements.txt

# Clear test artifacts
find tests/ -name "*.log" -delete
find tests/ -name "results_*" -type d -exec rm -rf {} +
```

## Common Issues

### Issue: "Permission denied" when accessing KVM

**Symptoms**:
```bash
qemu-system-x86_64: Could not access KVM kernel module: Permission denied
```

**Diagnosis**:
```bash
# Check KVM permissions
ls -la /dev/kvm
groups $USER | grep -q kvm && echo "In KVM group" || echo "NOT in KVM group"
```

**Solution**:
```bash
# Add user to KVM group
sudo usermod -a -G kvm $USER

# Apply group changes (logout/login or use newgrp)
newgrp kvm

# Verify KVM access
kvm-ok
```

### Issue: "Module not found" Python errors

**Symptoms**:
```bash
ModuleNotFoundError: No module named 'advanced_detection'
ImportError: cannot import name 'TestFramework'
```

**Diagnosis**:
```bash
# Check Python path
echo $PYTHONPATH
python3 -c "import sys; print('\n'.join(sys.path))"

# Check module structure
find tests/vm_testing -name "*.py" | head -10
```

**Solution**:
```bash
# Set Python path
export PYTHONPATH="${PYTHONPATH}:$(pwd)/tests/vm_testing"

# Add to shell profile for persistence
echo 'export PYTHONPATH="${PYTHONPATH}:$(pwd)/tests/vm_testing"' >> ~/.bashrc

# Install missing dependencies
pip3 install --user -r tests/vm_testing/requirements.txt

# Verify imports
python3 -c "
import sys
sys.path.append('tests/vm_testing')
import advanced_detection.advanced_crash_detection
print('✅ Modules imported successfully')
"
```

### Issue: Rust compilation failures

**Symptoms**:
```bash
error[E0432]: unresolved import `crate::common::TestResult`
error[E0463]: can't find crate for `tokio`
```

**Diagnosis**:
```bash
# Check Rust toolchain
rustc --version
cargo --version

# Check project structure
ls -la tests/kernel_module/src/
cat tests/kernel_module/Cargo.toml
```

**Solution**:
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cd tests/kernel_module
cargo clean
cargo build

# Check for missing dependencies
cargo check

# Fix common import issues
# Ensure lib.rs properly exports modules:
cat > tests/kernel_module/src/lib.rs << 'EOF'
pub mod common;
pub mod stress_testing_framework;
pub mod resource_monitoring;
pub mod kernel_instrumentation;
pub mod mount_recovery;

pub use common::*;
EOF
```

## Environment Setup Issues

### Issue: Insufficient system resources

**Symptoms**:
- VM fails to start with memory allocation errors
- Tests timeout frequently
- System becomes unresponsive during testing

**Diagnosis**:
```bash
# Check available resources
free -h
df -h
top -bn1 | head -20

# Check VM resource allocation
grep -r "memory\|cpu" tests/vm_testing/config/
```

**Solution**:
```bash
# Reduce VM memory allocation
# Edit tests/vm_testing/config/vm_config.json
{
  "vm_config": {
    "memory_mb": 2048,  # Reduce from 4096
    "cpu_cores": 2      # Reduce from 4
  }
}

# Increase system swap if needed
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Monitor resource usage during tests
./run_enhanced_vm_tests.sh --level 1 &
watch -n 1 'free -h; echo "---"; ps aux | grep -E "(qemu|python|cargo)" | head -10'
```

### Issue: Network connectivity problems

**Symptoms**:
- SSH connections to VM fail
- File transfers timeout
- Network-dependent tests fail

**Diagnosis**:
```bash
# Check network configuration
ip addr show
netstat -tlnp | grep -E "(2222|8080)"

# Test VM network
ping -c 3 localhost
telnet localhost 2222

# Check firewall
sudo ufw status
sudo iptables -L | grep -E "(2222|8080)"
```

**Solution**:
```bash
# Reset network configuration
sudo systemctl restart networking

# Configure firewall
sudo ufw allow 2222/tcp
sudo ufw allow 8080/tcp

# Restart VM with network debugging
cd tests/vm_testing
./manage_alpine_vm.sh stop
QEMU_NET_DEBUG=1 ./manage_alpine_vm.sh start

# Test connectivity
./test_vm_connectivity.sh
```

## VM Infrastructure Issues

### Issue: VM fails to start

**Symptoms**:
```bash
qemu-system-x86_64: terminating on signal 15 from pid 12345
VM startup failed after 60 seconds
```

**Diagnosis**:
```bash
# Check VM logs
tail -50 tests/vm_testing/logs/vm_startup.log

# Check QEMU process
ps aux | grep qemu
pgrep -f qemu-system

# Check VM image
ls -la tests/vm_testing/alpine_vm.qcow2
qemu-img info tests/vm_testing/alpine_vm.qcow2
```

**Solution**:
```bash
# Kill any stuck QEMU processes
sudo pkill -f qemu-system-x86_64

# Recreate VM image
cd tests/vm_testing
rm -f alpine_vm.qcow2
./setup_alpine_vm.sh

# Start VM with debugging
QEMU_DEBUG=1 ./manage_alpine_vm.sh start

# Check VM console output
./manage_alpine_vm.sh console
```

### Issue: SSH connection refused

**Symptoms**:
```bash
ssh: connect to host localhost port 2222: Connection refused
Connection timeout after 30 seconds
```

**Diagnosis**:
```bash
# Check SSH service in VM
./manage_alpine_vm.sh console
# In VM console:
# ps aux | grep sshd
# netstat -tlnp | grep 22

# Check port forwarding
netstat -tlnp | grep 2222
lsof -i :2222
```

**Solution**:
```bash
# Restart SSH service in VM
./manage_alpine_vm.sh console
# In VM console:
# sudo service sshd restart
# sudo rc-service sshd restart  # Alpine Linux

# Reset SSH configuration
cd tests/vm_testing
./setup_passwordless_sudo.sh

# Test SSH manually
ssh -p 2222 -o StrictHostKeyChecking=no root@localhost 'echo "SSH working"'
```

### Issue: File transfer failures

**Symptoms**:
```bash
scp: Connection closed
rsync: connection unexpectedly closed
File transfer timeout
```

**Diagnosis**:
```bash
# Test basic connectivity
ssh -p 2222 root@localhost 'echo "Connection test"'

# Check disk space in VM
ssh -p 2222 root@localhost 'df -h'

# Check file permissions
ls -la tests/vm_testing/transfer/
```

**Solution**:
```bash
# Clean up transfer directory
rm -rf tests/vm_testing/transfer/*

# Reset file permissions
chmod 755 tests/vm_testing/transfer/
chmod 644 tests/vm_testing/transfer/*

# Use alternative transfer method
./transfer_files_to_vm.sh --method=netcat
./transfer_files_to_vm.sh --method=http
```

## Test Execution Issues

### Issue: Tests timeout frequently

**Symptoms**:
```bash
Test execution timed out after 300 seconds
Component 'stress_testing' exceeded timeout
```

**Diagnosis**:
```bash
# Check system load during tests
top -bn1 | head -10
iostat 1 5

# Check test configuration
grep -r "timeout" tests/vm_testing/config/

# Monitor test progress
tail -f tests/vm_testing/logs/test_execution.log
```

**Solution**:
```bash
# Increase timeouts globally
# Edit tests/vm_testing/config/global_config.json
{
  "test_config": {
    "timeout_seconds": 600,
    "component_timeout_seconds": 900
  }
}

# Use timeout override
./run_enhanced_vm_tests.sh --level 2 --timeout 900

# Run tests with reduced parallelism
./run_enhanced_vm_tests.sh --level 2 --max-parallel 2
```

### Issue: Inconsistent test results

**Symptoms**:
- Tests pass sometimes, fail other times
- Different results on different runs
- Race conditions in test execution

**Diagnosis**:
```bash
# Run tests multiple times
for i in {1..5}; do
    echo "Run $i:"
    ./run_enhanced_vm_tests.sh --level 1 --quick-test
    echo "---"
done

# Check for race conditions
./run_enhanced_vm_tests.sh --level 1 --debug --verbose

# Monitor system resources
./monitor_test_resources.sh &
./run_enhanced_vm_tests.sh --level 2
```

**Solution**:
```bash
# Add delays between tests
# Edit test configuration to include delays
{
  "test_config": {
    "inter_test_delay_seconds": 2,
    "component_startup_delay_seconds": 5
  }
}

# Use deterministic test ordering
./run_enhanced_vm_tests.sh --level 2 --sequential

# Increase retry attempts
./run_enhanced_vm_tests.sh --level 2 --retry-attempts 3
```

## Performance Issues

### Issue: Slow test execution

**Symptoms**:
- Tests take much longer than expected
- High CPU/memory usage
- System becomes unresponsive

**Diagnosis**:
```bash
# Profile test execution
time ./run_enhanced_vm_tests.sh --level 1

# Monitor resource usage
htop &
./run_enhanced_vm_tests.sh --level 1

# Check I/O performance
iostat -x 1 10 &
./run_enhanced_vm_tests.sh --level 1
```

**Solution**:
```bash
# Optimize VM configuration
# Edit VM config for better performance
{
  "vm_config": {
    "cpu_type": "host",
    "enable_kvm": true,
    "disk_cache": "writeback",
    "network_model": "virtio"
  }
}

# Use SSD for VM storage
mkdir -p /tmp/vexfs_vm
export VEXFS_VM_STORAGE="/tmp/vexfs_vm"

# Reduce test scope for development
./run_enhanced_vm_tests.sh --level 1 --quick-test --component basic
```

### Issue: Memory leaks during testing

**Symptoms**:
- Memory usage continuously increases
- System runs out of memory
- OOM killer activates

**Diagnosis**:
```bash
# Monitor memory usage
watch -n 1 'free -h; echo "---"; ps aux --sort=-%mem | head -10'

# Check for memory leaks in components
valgrind --tool=memcheck --leak-check=full ./target/release/test_runner

# Monitor VM memory usage
ssh -p 2222 root@localhost 'free -h; ps aux --sort=-%mem | head -5'
```

**Solution**:
```bash
# Restart VM between test suites
./run_enhanced_vm_tests.sh --level 2 --restart-vm-between-suites

# Reduce VM memory allocation
# Edit VM config
{
  "vm_config": {
    "memory_mb": 1024  # Reduce from 2048
  }
}

# Enable memory monitoring
export VEXFS_MEMORY_MONITORING=1
./run_enhanced_vm_tests.sh --level 2
```

## Component-Specific Issues

### Advanced Crash Detection Issues

**Issue**: Crash detection not working

**Symptoms**:
```bash
No crashes detected despite kernel panics
False positive crash detections
Pattern matching failures
```

**Diagnosis**:
```bash
# Test crash detection manually
python3 -c "
import sys
sys.path.append('tests/vm_testing')
from advanced_detection.advanced_crash_detection import CrashDetector
detector = CrashDetector()
print('Crash detector initialized successfully')
"

# Check pattern database
ls -la tests/vm_testing/advanced_detection/patterns/
cat tests/vm_testing/advanced_detection/patterns/crash_patterns.json
```

**Solution**:
```bash
# Update pattern database
cd tests/vm_testing/advanced_detection
./update_crash_patterns.sh

# Test with known crash logs
python3 test_crash_detection.py --test-file sample_crash.log

# Adjust detection sensitivity
# Edit advanced_detection/config.json
{
  "crash_detection": {
    "confidence_threshold": 0.7,  # Lower from 0.8
    "pattern_matching_strict": false
  }
}
```

### Syzkaller Integration Issues

**Issue**: Syzkaller fails to start

**Symptoms**:
```bash
syzkaller: failed to create manager
syzkaller: no VMs started
Fuzzing session terminated unexpectedly
```

**Diagnosis**:
```bash
# Check Syzkaller installation
which syz-manager
syz-manager -version

# Check Syzkaller configuration
cat tests/vm_testing/syzkaller/syzkaller.cfg

# Check VM compatibility
./check_syzkaller_vm_compatibility.sh
```

**Solution**:
```bash
# Reinstall Syzkaller
cd tests/vm_testing/syzkaller
./install_syzkaller.sh

# Update configuration
./generate_syzkaller_config.sh

# Test with minimal configuration
./run_syzkaller_minimal.sh
```

### eBPF Tracing Issues

**Issue**: eBPF programs fail to load

**Symptoms**:
```bash
bpf: failed to load program: Operation not permitted
eBPF verification failed
Tracing data not collected
```

**Diagnosis**:
```bash
# Check eBPF support
ls /sys/kernel/debug/tracing/
mount | grep debugfs

# Check kernel version
uname -r
cat /proc/version

# Test eBPF capabilities
bpftool prog list
```

**Solution**:
```bash
# Mount debugfs if needed
sudo mount -t debugfs debugfs /sys/kernel/debug

# Enable eBPF in VM
ssh -p 2222 root@localhost 'echo 1 > /proc/sys/kernel/unprivileged_bpf_disabled'

# Use alternative tracing methods
export VEXFS_TRACING_METHOD=ftrace
./run_enhanced_vm_tests.sh --level 2
```

## Debug Tools and Techniques

### Comprehensive Debugging Script

```bash
#!/bin/bash
# debug_vexfs_testing.sh - Comprehensive debugging tool

set -euo pipefail

DEBUG_DIR="debug_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$DEBUG_DIR"

echo "🔍 Collecting VexFS testing debug information..."

# System information
echo "📋 System Information" > "$DEBUG_DIR/system_info.txt"
{
    echo "Date: $(date)"
    echo "Hostname: $(hostname)"
    echo "Kernel: $(uname -a)"
    echo "Distribution: $(lsb_release -d 2>/dev/null || echo 'Unknown')"
    echo "Memory: $(free -h)"
    echo "Disk: $(df -h)"
    echo "CPU: $(lscpu | grep 'Model name')"
} >> "$DEBUG_DIR/system_info.txt"

# Process information
echo "🔄 Process Information" > "$DEBUG_DIR/processes.txt"
{
    echo "=== All processes ==="
    ps aux
    echo -e "\n=== QEMU processes ==="
    ps aux | grep qemu || echo "No QEMU processes"
    echo -e "\n=== Python processes ==="
    ps aux | grep python || echo "No Python processes"
    echo -e "\n=== Rust processes ==="
    ps aux | grep -E "(cargo|rustc)" || echo "No Rust processes"
} >> "$DEBUG_DIR/processes.txt"

# Network information
echo "🌐 Network Information" > "$DEBUG_DIR/network.txt"
{
    echo "=== Network interfaces ==="
    ip addr show
    echo -e "\n=== Listening ports ==="
    netstat -tlnp
    echo -e "\n=== VM connectivity test ==="
    timeout 5 telnet localhost 2222 2>&1 || echo "VM not accessible"
} >> "$DEBUG_DIR/network.txt"

# Configuration files
echo "⚙️ Configuration Files" > "$DEBUG_DIR/config.txt"
{
    echo "=== Global config ==="
    cat tests/vm_testing/config/global_config.json 2>/dev/null || echo "Global config not found"
    echo -e "\n=== VM config ==="
    cat tests/vm_testing/config/vm_config.json 2>/dev/null || echo "VM config not found"
} >> "$DEBUG_DIR/config.txt"

# Log files
echo "📝 Collecting log files..."
mkdir -p "$DEBUG_DIR/logs"
find tests/ -name "*.log" -exec cp {} "$DEBUG_DIR/logs/" \; 2>/dev/null || true

# Test results
echo "📊 Collecting test results..."
mkdir -p "$DEBUG_DIR/results"
find tests/ -name "results_*" -type d -exec cp -r {} "$DEBUG_DIR/results/" \; 2>/dev/null || true

# VM status
echo "🖥️ VM Status" > "$DEBUG_DIR/vm_status.txt"
{
    cd tests/vm_testing
    ./manage_alpine_vm.sh status 2>&1 || echo "VM status check failed"
} >> "$DEBUG_DIR/vm_status.txt"

# Environment variables
echo "🌍 Environment Variables" > "$DEBUG_DIR/environment.txt"
env | grep -E "(VEXFS|RUST|PYTHON|PATH)" >> "$DEBUG_DIR/environment.txt"

# Create archive
tar -czf "${DEBUG_DIR}.tar.gz" "$DEBUG_DIR"
rm -rf "$DEBUG_DIR"

echo "✅ Debug information collected in ${DEBUG_DIR}.tar.gz"
echo "📧 Please attach this file when reporting issues"
```

### Real-time Monitoring

```bash
#!/bin/bash
# monitor_testing.sh - Real-time test monitoring

# Terminal multiplexer for monitoring
tmux new-session -d -s vexfs_monitor

# System resources
tmux new-window -t vexfs_monitor -n 'resources'
tmux send-keys -t vexfs_monitor:resources 'htop' Enter

# Network activity
tmux new-window -t vexfs_monitor -n 'network'
tmux send-keys -t vexfs_monitor:network 'watch -n 1 "netstat -i; echo; ss -tuln | grep -E \"(2222|8080)\""' Enter

# Log monitoring
tmux new-window -t vexfs_monitor -n 'logs'
tmux send-keys -t vexfs_monitor:logs 'tail -f tests/vm_testing/logs/*.log' Enter

# VM console
tmux new-window -t vexfs_monitor -n 'vm_console'
tmux send-keys -t vexfs_monitor:vm_console 'cd tests/vm_testing && ./manage_alpine_vm.sh console' Enter

# Test execution
tmux new-window -t vexfs_monitor -n 'tests'
tmux send-keys -t vexfs_monitor:tests 'cd tests/vm_testing' Enter

echo "🖥️ Monitoring session started. Attach with: tmux attach -t vexfs_monitor"
```

## Log Analysis

### Automated Log Analysis

```python
#!/usr/bin/env python3
# analyze_test_logs.py - Automated log analysis tool

import re
import json
import sys
from pathlib import Path
from collections import defaultdict, Counter
from datetime import datetime

class TestLogAnalyzer:
    def __init__(self, log_directory: str):
        self.log_dir = Path(log_directory)
        self.patterns = {
            'error': re.compile(r'ERROR|FAILED|FATAL|PANIC', re.IGNORECASE),
            'warning': re.compile(r'WARNING|WARN', re.IGNORECASE),
            'timeout': re.compile(r'timeout|timed out', re.IGNORECASE),
            'memory': re.compile(r'out of memory|oom|memory leak', re.IGNORECASE),
            'network': re.compile(r'connection refused|network unreachable', re.IGNORECASE),
            'vm': re.compile(r'qemu|kvm|virtualization', re.IGNORECASE),
        }
    
    def analyze_logs(self):
        """Analyze all log files in the directory"""
        results = {
            'summary': defaultdict(int),
            'files': {},
            'patterns': defaultdict(list),
            'timeline': []
        }
        
        for log_file in self.log_dir.glob('*.log'):
            file_results = self.analyze_file(log_file)
            results['files'][str(log_file)] = file_results
            
            # Update summary
            for category, count in file_results['pattern_counts'].items():
                results['summary'][category] += count
        
        return results
    
    def analyze_file(self, log_file: Path):
        """Analyze a single log file"""
        results = {
            'pattern_counts': defaultdict(int),
            'issues': [],
            'timeline': []
        }
        
        try:
            with open(log_file, 'r') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue
                    
                    # Check patterns
                    for pattern_name, pattern in self.patterns.items():
                        if pattern.search(line):
                            results['pattern_counts'][pattern_name] += 1
                            results['issues'].append({
                                'line': line_num,
                                'type': pattern_name,
                                'content': line
                            })
                    
                    # Extract timestamps if present
                    timestamp_match = re.search(r'\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}', line)
                    if timestamp_match:
                        results['timeline'].append({
                            'timestamp': timestamp_match.group(),
                            'line': line_num,
                            'content': line
                        })
        
        except Exception as e:
            results['error'] = str(e)
        
        return results
    
    def generate_report(self, results):
        """Generate a human-readable report"""
        report = []
        report.append("🔍 VexFS Test Log Analysis Report")
        report.append("=" * 50)
        report.append(f"Analysis Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        report.append(f"Log Directory: {self.log_dir}")
        report.append("")
        
        # Summary
        report.append("📊 Summary:")
        if results['summary']:
            for category, count in sorted(results['summary'].items()):
                report.append(f"  {category.upper()}: {count}")
        else:
            report.append("  No issues detected")
        report.append("")
        
        # Top issues
        all_issues = []
        for file_results in results['files'].values():
            all_issues.extend(file_results.get('issues', []))
        
        if all_issues:
            report.append("🚨 Top Issues:")
            issue_counter = Counter(issue['type'] for issue in all_issues)
            for issue_type, count in issue_counter.most_common(5):
                report.append(f"  {issue_type}: {count} occurrences")
            report.append("")
        
        # File-specific analysis
        report.append("📁 File Analysis:")
        for file_path, file_results in results['files'].items():
            if file_results.get('pattern_counts'):
                report.append(f"  {Path(file_path).name}:")
                for category, count in file_results['pattern_counts'].items():
                    report.append(f"    {category}: {count}")
        
        return "\n".join(report)

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 analyze_test_logs.py <log_directory>")
        sys.exit(1)
    
    log_dir = sys.argv[1]
    analyzer = TestLogAnalyzer(log_dir)
    results = analyzer.analyze_logs()
    report = analyzer.generate_report(results)
    
    print(report)
    
    # Save detailed results
    with open('log_analysis_results.json', 'w') as f:
        json.dump(results, f, indent=2, default=str)
    
    print(f"\n💾 Detailed results saved to log_analysis_results.json")

if __name__ == "__main__":
    main()
```

## Recovery Procedures

### Complete Environment Reset

```bash
#!/bin/bash
# reset_testing_environment.sh - Complete environment reset

echo "🔄 Resetting VexFS testing environment..."

# Stop all running processes
echo "🛑 Stopping processes..."
sudo pkill -f qemu-system-x86_64 || true
sudo pkill -f python3.*vm_testing || true
sudo pkill -f cargo.*test || true

# Clean up VM
echo "🖥️ Cleaning VM environment..."
cd tests/vm_testing
./manage_alpine_vm.sh stop || true
rm -f alpine_vm.qcow2
rm -rf logs/*
rm -rf results_*
rm -rf transfer/*

# Clean up Rust artifacts
echo "🦀 Cleaning Rust artifacts..."
cd ../../tests/kernel_module
cargo clean

# Reset Python environment
echo "🐍 Resetting Python environment..."
cd ../vm_testing
pip3 uninstall -y -r requirements.txt || true
pip3 install --user -r requirements.txt

# Recreate VM
echo "🏗️ Recreating VM..."
./setup_alpine_vm.sh

# Rebuild Rust components
echo "🔨 Rebuilding Rust components..."
cd ../kernel_module
cargo build --release

# Test basic functionality
echo "✅ Testing basic functionality..."
cd ../vm_testing
./run_enhanced_vm_tests.sh --level 1 --quick-test

echo "🎉 Environment reset complete!"
```

### Selective Component Reset

```bash
#!/bin/bash
# reset_component.sh - Reset specific component

COMPONENT="$1"

if [ -z "$COMPONENT" ]; then
    echo "Usage: $0 <component_name>"
    echo "Available components: vm, rust, python, syzkaller, ebpf"
    exit 1
fi

case "$COMPONENT" in
    vm)
        echo "🖥️ Resetting VM component..."
        cd tests/vm_testing
        ./manage_alpine_vm.sh stop
        rm -f alpine_vm.qcow2
        ./setup_alpine_vm.sh
        ;;
    rust)
        echo "🦀 Resetting Rust component..."
        cd tests/kernel_module
        cargo clean
        cargo build --release
        ;;
    python)
        echo "🐍 Resetting Python component..."
        cd tests/vm_testing
        pip3 install --user --force-reinstall -r requirements.txt
        ;;
    syzkaller)
        echo "🔍 Resetting Syzkaller component..."
        cd tests/vm_testing/syzkaller
        ./install_syzkaller.sh
        ;;
    ebpf)
        echo "🕵️ Resetting eBPF component..."
        sudo mount -t debugfs debugfs /sys/kernel/debug 2>/dev/null || true
        cd tests/vm_testing/ebpf
        ./setup_ebpf.sh
        ;;
    *)
        echo "❌ Unknown component: $COMPONENT"
        exit 1
        ;;
esac

echo "✅ Component $COMPONENT reset complete!"