#!/bin/bash

# VexFS 200GB Testing - Performance Monitoring System
# Continuously monitors system performance during VexFS testing

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WORKBENCH_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MONITORING_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load configuration
if [ -f "$WORKBENCH_ROOT/test_config.conf" ]; then
    source "$WORKBENCH_ROOT/test_config.conf"
elif [ -f "$MONITORING_DIR/monitor_config.conf" ]; then
    source "$MONITORING_DIR/monitor_config.conf"
else
    # Default configuration
    MONITOR_INTERVAL=5
    METRICS_DIR="$MONITORING_DIR/metrics"
    LOG_DIR="$MONITORING_DIR/logs"
    TARGET_DEVICE="/dev/sda1"
    MOUNT_POINT="/mnt/vexfs_test"
fi

echo -e "${BLUE}📊 VexFS Performance Monitoring System${NC}"
echo "=================================================================="
echo "Monitor Interval: ${MONITOR_INTERVAL}s"
echo "Metrics Directory: $METRICS_DIR"
echo "Log Directory: $LOG_DIR"
echo "Target Device: $TARGET_DEVICE"
echo "Mount Point: $MOUNT_POINT"
echo "=================================================================="

# Create directories
mkdir -p "$METRICS_DIR" "$LOG_DIR"

# Function to print status
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "OK" ]; then
        echo -e "${GREEN}✅ $message${NC}"
    elif [ "$status" = "WARNING" ]; then
        echo -e "${YELLOW}⚠️  $message${NC}"
    else
        echo -e "${RED}❌ $message${NC}"
    fi
}

# Function to log with timestamp
log_message() {
    local message=$1
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $message" | tee -a "$LOG_DIR/monitoring.log"
}

# Function to check dependencies
check_dependencies() {
    echo -e "\n${BLUE}🔍 Checking monitoring dependencies...${NC}"
    
    local deps=("python3" "iostat" "vmstat" "free" "df")
    for dep in "${deps[@]}"; do
        if command -v "$dep" >/dev/null 2>&1; then
            print_status "OK" "$dep available"
        else
            print_status "WARNING" "$dep not found - some metrics may be unavailable"
        fi
    done
    
    # Check Python libraries
    if python3 -c "import psutil, json, time" 2>/dev/null; then
        print_status "OK" "Python monitoring libraries available"
    else
        print_status "ERROR" "Required Python libraries missing"
        echo "Run: pip3 install psutil"
        exit 1
    fi
}

# Function to start system monitoring
start_system_monitoring() {
    echo -e "\n${BLUE}🖥️  Starting system monitoring...${NC}"
    
    log_message "Starting system performance monitoring"
    
    python3 << 'EOF' &
import os
import time
import json
import psutil
from datetime import datetime

# Configuration from environment
monitor_interval = int(os.environ.get('MONITOR_INTERVAL', '5'))
metrics_dir = os.environ.get('METRICS_DIR', './metrics')
target_device = os.environ.get('TARGET_DEVICE', '/dev/sda1')
mount_point = os.environ.get('MOUNT_POINT', '/mnt/vexfs_test')

print(f"System monitoring started - interval: {monitor_interval}s")

# Create metrics file
metrics_file = f"{metrics_dir}/system_metrics.jsonl"
os.makedirs(metrics_dir, exist_ok=True)

try:
    while True:
        timestamp = datetime.now().isoformat()
        
        # CPU metrics
        cpu_percent = psutil.cpu_percent(interval=1)
        cpu_count = psutil.cpu_count()
        load_avg = os.getloadavg()
        
        # Memory metrics
        memory = psutil.virtual_memory()
        swap = psutil.swap_memory()
        
        # Disk metrics
        try:
            disk_usage = psutil.disk_usage(mount_point)
            disk_io = psutil.disk_io_counters(perdisk=True)
            
            # Get specific device stats
            device_name = target_device.split('/')[-1]
            device_io = disk_io.get(device_name, None)
            
            disk_metrics = {
                'total_gb': disk_usage.total / (1024**3),
                'used_gb': disk_usage.used / (1024**3),
                'free_gb': disk_usage.free / (1024**3),
                'percent_used': (disk_usage.used / disk_usage.total) * 100
            }
            
            if device_io:
                disk_metrics.update({
                    'read_count': device_io.read_count,
                    'write_count': device_io.write_count,
                    'read_bytes': device_io.read_bytes,
                    'write_bytes': device_io.write_bytes,
                    'read_time': device_io.read_time,
                    'write_time': device_io.write_time
                })
        except Exception as e:
            disk_metrics = {'error': str(e)}
        
        # Network metrics
        try:
            network = psutil.net_io_counters()
            network_metrics = {
                'bytes_sent': network.bytes_sent,
                'bytes_recv': network.bytes_recv,
                'packets_sent': network.packets_sent,
                'packets_recv': network.packets_recv
            }
        except Exception as e:
            network_metrics = {'error': str(e)}
        
        # Process metrics
        try:
            processes = []
            for proc in psutil.process_iter(['pid', 'name', 'cpu_percent', 'memory_percent']):
                try:
                    if 'vexfs' in proc.info['name'].lower() or proc.info['cpu_percent'] > 5.0:
                        processes.append(proc.info)
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    pass
        except Exception as e:
            processes = [{'error': str(e)}]
        
        # Compile metrics
        metrics = {
            'timestamp': timestamp,
            'cpu': {
                'percent': cpu_percent,
                'count': cpu_count,
                'load_avg_1m': load_avg[0],
                'load_avg_5m': load_avg[1],
                'load_avg_15m': load_avg[2]
            },
            'memory': {
                'total_gb': memory.total / (1024**3),
                'available_gb': memory.available / (1024**3),
                'used_gb': memory.used / (1024**3),
                'percent': memory.percent,
                'swap_total_gb': swap.total / (1024**3),
                'swap_used_gb': swap.used / (1024**3),
                'swap_percent': swap.percent
            },
            'disk': disk_metrics,
            'network': network_metrics,
            'processes': processes
        }
        
        # Write metrics
        with open(metrics_file, 'a') as f:
            f.write(json.dumps(metrics) + '\n')
        
        # Print periodic status
        if int(time.time()) % 60 == 0:  # Every minute
            print(f"[{timestamp}] CPU: {cpu_percent:.1f}%, Memory: {memory.percent:.1f}%, Disk: {disk_metrics.get('percent_used', 0):.1f}%")
        
        time.sleep(monitor_interval)

except KeyboardInterrupt:
    print("System monitoring stopped")
except Exception as e:
    print(f"Monitoring error: {e}")
EOF
    
    local monitor_pid=$!
    echo $monitor_pid > "$MONITORING_DIR/system_monitor.pid"
    print_status "OK" "System monitoring started (PID: $monitor_pid)"
}

# Function to start VexFS-specific monitoring
start_vexfs_monitoring() {
    echo -e "\n${BLUE}🧠 Starting VexFS-specific monitoring...${NC}"
    
    log_message "Starting VexFS-specific monitoring"
    
    python3 << 'EOF' &
import os
import time
import json
import subprocess
from datetime import datetime

# Configuration
monitor_interval = int(os.environ.get('MONITOR_INTERVAL', '5'))
metrics_dir = os.environ.get('METRICS_DIR', './metrics')
mount_point = os.environ.get('MOUNT_POINT', '/mnt/vexfs_test')

print(f"VexFS monitoring started - interval: {monitor_interval}s")

# Create VexFS metrics file
vexfs_metrics_file = f"{metrics_dir}/vexfs_metrics.jsonl"
os.makedirs(metrics_dir, exist_ok=True)

def get_kernel_module_info():
    """Get VexFS kernel module information"""
    try:
        result = subprocess.run(['lsmod'], capture_output=True, text=True)
        for line in result.stdout.split('\n'):
            if 'vexfs' in line.lower():
                parts = line.split()
                return {
                    'name': parts[0],
                    'size': int(parts[1]),
                    'used_by': int(parts[2]) if parts[2].isdigit() else 0,
                    'loaded': True
                }
        return {'loaded': False}
    except Exception as e:
        return {'error': str(e)}

def get_filesystem_stats():
    """Get filesystem-specific statistics"""
    try:
        # Check if mount point exists and is mounted
        result = subprocess.run(['mount'], capture_output=True, text=True)
        mounted = mount_point in result.stdout
        
        if not mounted:
            return {'mounted': False}
        
        # Get file count and directory structure
        try:
            result = subprocess.run(['find', mount_point, '-type', 'f'], 
                                  capture_output=True, text=True)
            file_count = len(result.stdout.strip().split('\n')) if result.stdout.strip() else 0
            
            result = subprocess.run(['find', mount_point, '-type', 'd'], 
                                  capture_output=True, text=True)
            dir_count = len(result.stdout.strip().split('\n')) if result.stdout.strip() else 0
        except Exception:
            file_count = 0
            dir_count = 0
        
        # Get largest files
        try:
            result = subprocess.run(['find', mount_point, '-type', 'f', '-exec', 'ls', '-la', '{}', '+'], 
                                  capture_output=True, text=True)
            files = result.stdout.strip().split('\n')
            largest_files = []
            for file_line in files[-10:]:  # Last 10 files
                if file_line and mount_point in file_line:
                    parts = file_line.split()
                    if len(parts) >= 9:
                        largest_files.append({
                            'size': int(parts[4]) if parts[4].isdigit() else 0,
                            'name': parts[-1]
                        })
        except Exception:
            largest_files = []
        
        return {
            'mounted': True,
            'file_count': file_count,
            'directory_count': dir_count,
            'largest_files': largest_files
        }
    except Exception as e:
        return {'error': str(e)}

try:
    while True:
        timestamp = datetime.now().isoformat()
        
        # Get kernel module info
        kernel_info = get_kernel_module_info()
        
        # Get filesystem stats
        fs_stats = get_filesystem_stats()
        
        # Get dmesg output for VexFS
        try:
            result = subprocess.run(['dmesg', '--since', '1 minute ago'], 
                                  capture_output=True, text=True)
            vexfs_logs = [line for line in result.stdout.split('\n') 
                         if 'vexfs' in line.lower()]
        except Exception:
            vexfs_logs = []
        
        # Compile VexFS metrics
        vexfs_metrics = {
            'timestamp': timestamp,
            'kernel_module': kernel_info,
            'filesystem': fs_stats,
            'recent_logs': vexfs_logs[-5:] if vexfs_logs else []  # Last 5 log entries
        }
        
        # Write metrics
        with open(vexfs_metrics_file, 'a') as f:
            f.write(json.dumps(vexfs_metrics) + '\n')
        
        # Print status
        if int(time.time()) % 60 == 0:  # Every minute
            status = "MOUNTED" if fs_stats.get('mounted') else "NOT MOUNTED"
            files = fs_stats.get('file_count', 0)
            print(f"[{timestamp}] VexFS: {status}, Files: {files}")
        
        time.sleep(monitor_interval)

except KeyboardInterrupt:
    print("VexFS monitoring stopped")
except Exception as e:
    print(f"VexFS monitoring error: {e}")
EOF
    
    local vexfs_monitor_pid=$!
    echo $vexfs_monitor_pid > "$MONITORING_DIR/vexfs_monitor.pid"
    print_status "OK" "VexFS monitoring started (PID: $vexfs_monitor_pid)"
}

# Function to start performance alerting
start_alerting() {
    echo -e "\n${BLUE}🚨 Starting performance alerting...${NC}"
    
    log_message "Starting performance alerting system"
    
    python3 << 'EOF' &
import os
import time
import json
from datetime import datetime

# Configuration
monitor_interval = int(os.environ.get('MONITOR_INTERVAL', '5')) * 2  # Check every 2 intervals
metrics_dir = os.environ.get('METRICS_DIR', './metrics')
log_dir = os.environ.get('LOG_DIR', './logs')

# Alert thresholds
CPU_THRESHOLD = 90.0
MEMORY_THRESHOLD = 90.0
DISK_THRESHOLD = 95.0
LOAD_THRESHOLD = 8.0

print(f"Performance alerting started - check interval: {monitor_interval}s")

def check_alerts():
    """Check for performance issues and generate alerts"""
    try:
        metrics_file = f"{metrics_dir}/system_metrics.jsonl"
        if not os.path.exists(metrics_file):
            return
        
        # Read last few metrics
        with open(metrics_file, 'r') as f:
            lines = f.readlines()
            if not lines:
                return
            
            # Get last metric
            last_metric = json.loads(lines[-1])
            
            alerts = []
            timestamp = datetime.now().isoformat()
            
            # CPU alert
            cpu_percent = last_metric.get('cpu', {}).get('percent', 0)
            if cpu_percent > CPU_THRESHOLD:
                alerts.append(f"HIGH CPU USAGE: {cpu_percent:.1f}% (threshold: {CPU_THRESHOLD}%)")
            
            # Memory alert
            memory_percent = last_metric.get('memory', {}).get('percent', 0)
            if memory_percent > MEMORY_THRESHOLD:
                alerts.append(f"HIGH MEMORY USAGE: {memory_percent:.1f}% (threshold: {MEMORY_THRESHOLD}%)")
            
            # Disk alert
            disk_percent = last_metric.get('disk', {}).get('percent_used', 0)
            if disk_percent > DISK_THRESHOLD:
                alerts.append(f"HIGH DISK USAGE: {disk_percent:.1f}% (threshold: {DISK_THRESHOLD}%)")
            
            # Load average alert
            load_avg = last_metric.get('cpu', {}).get('load_avg_1m', 0)
            if load_avg > LOAD_THRESHOLD:
                alerts.append(f"HIGH LOAD AVERAGE: {load_avg:.2f} (threshold: {LOAD_THRESHOLD})")
            
            # Write alerts if any
            if alerts:
                alert_data = {
                    'timestamp': timestamp,
                    'alerts': alerts,
                    'metrics': last_metric
                }
                
                alert_file = f"{log_dir}/performance_alerts.jsonl"
                os.makedirs(log_dir, exist_ok=True)
                
                with open(alert_file, 'a') as f:
                    f.write(json.dumps(alert_data) + '\n')
                
                # Print alerts to console
                print(f"\n🚨 PERFORMANCE ALERT [{timestamp}]:")
                for alert in alerts:
                    print(f"   {alert}")
                print()
    
    except Exception as e:
        print(f"Alert checking error: {e}")

try:
    while True:
        check_alerts()
        time.sleep(monitor_interval)

except KeyboardInterrupt:
    print("Performance alerting stopped")
except Exception as e:
    print(f"Alerting error: {e}")
EOF
    
    local alert_pid=$!
    echo $alert_pid > "$MONITORING_DIR/alerting.pid"
    print_status "OK" "Performance alerting started (PID: $alert_pid)"
}

# Function to create monitoring dashboard
create_dashboard() {
    echo -e "\n${BLUE}📈 Creating monitoring dashboard...${NC}"
    
    cat > "$MONITORING_DIR/dashboard.py" << 'EOF'
#!/usr/bin/env python3

import os
import json
import time
from datetime import datetime, timedelta

def load_metrics(metrics_file, hours=1):
    """Load metrics from the last N hours"""
    if not os.path.exists(metrics_file):
        return []
    
    cutoff_time = datetime.now() - timedelta(hours=hours)
    metrics = []
    
    with open(metrics_file, 'r') as f:
        for line in f:
            try:
                metric = json.loads(line.strip())
                metric_time = datetime.fromisoformat(metric['timestamp'])
                if metric_time >= cutoff_time:
                    metrics.append(metric)
            except (json.JSONDecodeError, KeyError, ValueError):
                continue
    
    return metrics

def display_dashboard():
    """Display real-time monitoring dashboard"""
    metrics_dir = os.environ.get('METRICS_DIR', './metrics')
    
    while True:
        os.system('clear')
        print("=" * 80)
        print("🚀 VexFS Performance Monitoring Dashboard")
        print("=" * 80)
        print(f"Last Updated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print()
        
        # Load recent metrics
        system_metrics = load_metrics(f"{metrics_dir}/system_metrics.jsonl", hours=1)
        vexfs_metrics = load_metrics(f"{metrics_dir}/vexfs_metrics.jsonl", hours=1)
        
        if system_metrics:
            latest = system_metrics[-1]
            
            # System overview
            print("📊 SYSTEM OVERVIEW")
            print("-" * 40)
            cpu = latest.get('cpu', {})
            memory = latest.get('memory', {})
            disk = latest.get('disk', {})
            
            print(f"CPU Usage:     {cpu.get('percent', 0):.1f}%")
            print(f"Memory Usage:  {memory.get('percent', 0):.1f}% ({memory.get('used_gb', 0):.1f}GB / {memory.get('total_gb', 0):.1f}GB)")
            print(f"Disk Usage:    {disk.get('percent_used', 0):.1f}% ({disk.get('used_gb', 0):.1f}GB / {disk.get('total_gb', 0):.1f}GB)")
            print(f"Load Average:  {cpu.get('load_avg_1m', 0):.2f}")
            print()
            
            # Performance trends (last hour)
            if len(system_metrics) > 1:
                print("📈 PERFORMANCE TRENDS (Last Hour)")
                print("-" * 40)
                
                cpu_values = [m.get('cpu', {}).get('percent', 0) for m in system_metrics]
                memory_values = [m.get('memory', {}).get('percent', 0) for m in system_metrics]
                
                print(f"CPU:    Min: {min(cpu_values):.1f}%  Max: {max(cpu_values):.1f}%  Avg: {sum(cpu_values)/len(cpu_values):.1f}%")
                print(f"Memory: Min: {min(memory_values):.1f}%  Max: {max(memory_values):.1f}%  Avg: {sum(memory_values)/len(memory_values):.1f}%")
                print()
        
        # VexFS status
        if vexfs_metrics:
            latest_vexfs = vexfs_metrics[-1]
            
            print("🧠 VEXFS STATUS")
            print("-" * 40)
            
            kernel_info = latest_vexfs.get('kernel_module', {})
            fs_info = latest_vexfs.get('filesystem', {})
            
            print(f"Kernel Module: {'✅ Loaded' if kernel_info.get('loaded') else '❌ Not Loaded'}")
            print(f"Filesystem:    {'✅ Mounted' if fs_info.get('mounted') else '❌ Not Mounted'}")
            
            if fs_info.get('mounted'):
                print(f"Files:         {fs_info.get('file_count', 0):,}")
                print(f"Directories:   {fs_info.get('directory_count', 0):,}")
            
            recent_logs = latest_vexfs.get('recent_logs', [])
            if recent_logs:
                print(f"Recent Logs:   {len(recent_logs)} entries")
            print()
        
        # Alerts
        alert_file = f"{os.environ.get('LOG_DIR', './logs')}/performance_alerts.jsonl"
        if os.path.exists(alert_file):
            with open(alert_file, 'r') as f:
                lines = f.readlines()
                if lines:
                    print("🚨 RECENT ALERTS")
                    print("-" * 40)
                    
                    # Show last 3 alerts
                    for line in lines[-3:]:
                        try:
                            alert = json.loads(line.strip())
                            timestamp = alert['timestamp'][:19]  # Remove microseconds
                            for alert_msg in alert['alerts']:
                                print(f"[{timestamp}] {alert_msg}")
                        except (json.JSONDecodeError, KeyError):
                            continue
                    print()
        
        print("Press Ctrl+C to exit")
        print("=" * 80)
        
        time.sleep(5)

if __name__ == "__main__":
    try:
        display_dashboard()
    except KeyboardInterrupt:
        print("\nDashboard stopped")
EOF
    
    chmod +x "$MONITORING_DIR/dashboard.py"
    print_status "OK" "Monitoring dashboard created: $MONITORING_DIR/dashboard.py"
}

# Function to stop monitoring
stop_monitoring() {
    echo -e "\n${BLUE}🛑 Stopping monitoring processes...${NC}"
    
    local pids=("system_monitor.pid" "vexfs_monitor.pid" "alerting.pid")
    
    for pid_file in "${pids[@]}"; do
        if [ -f "$MONITORING_DIR/$pid_file" ]; then
            local pid=$(cat "$MONITORING_DIR/$pid_file")
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid"
                print_status "OK" "Stopped process $pid"
            fi
            rm -f "$MONITORING_DIR/$pid_file"
        fi
    done
    
    log_message "All monitoring processes stopped"
}

# Signal handlers
trap stop_monitoring EXIT INT TERM

# Main execution
main() {
    echo -e "${BLUE}Starting VexFS monitoring system...${NC}\n"
    
    check_dependencies
    start_system_monitoring
    start_vexfs_monitoring
    start_alerting
    create_dashboard
    
    echo -e "\n${GREEN}🎉 VexFS Monitoring System ACTIVE!${NC}"
    echo "=================================================================="
    echo "Monitoring processes are running in the background"
    echo ""
    echo "Available commands:"
    echo "- View dashboard: python3 $MONITORING_DIR/dashboard.py"
    echo "- View logs: tail -f $LOG_DIR/monitoring.log"
    echo "- View metrics: tail -f $METRICS_DIR/system_metrics.jsonl"
    echo "- Stop monitoring: kill $$"
    echo ""
    echo "Press Ctrl+C to stop all monitoring"
    
    log_message "VexFS monitoring system fully operational"
    
    # Keep script running
    while true; do
        sleep 60
        log_message "Monitoring system heartbeat - all processes active"
    done
}

# Execute main function
main "$@"