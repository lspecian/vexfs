#!/bin/bash

# VexFS eBPF Tracing Manager
# 
# Comprehensive tool for managing VexFS kernel module tracing with eBPF/bpftrace
# Provides easy interface for running different tracing scenarios and analysis
#
# Usage: ./vexfs_trace_manager.sh [command] [options]
#
# Author: VexFS Development Team
# Version: 1.0.0

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VEXFS_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TRACE_SCRIPTS_DIR="$SCRIPT_DIR/../scripts"
RESULTS_DIR="$SCRIPT_DIR/../results"
CONFIGS_DIR="$SCRIPT_DIR/../configs"
ANALYSIS_DIR="$SCRIPT_DIR/../analysis"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging
LOG_FILE="$RESULTS_DIR/trace_manager.log"

# Ensure directories exist
mkdir -p "$RESULTS_DIR" "$CONFIGS_DIR" "$ANALYSIS_DIR"

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Print colored output
print_color() {
    local color="$1"
    shift
    echo -e "${color}$*${NC}"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_color "$RED" "❌ Error: This script must be run as root for eBPF tracing"
        print_color "$YELLOW" "💡 Try: sudo $0 $*"
        exit 1
    fi
}

# Check dependencies
check_dependencies() {
    local missing_deps=()
    
    if ! command -v bpftrace &> /dev/null; then
        missing_deps+=("bpftrace")
    fi
    
    if ! command -v lsmod &> /dev/null; then
        missing_deps+=("lsmod")
    fi
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        print_color "$RED" "❌ Missing dependencies: ${missing_deps[*]}"
        print_color "$YELLOW" "💡 Install with: sudo apt-get install bpftrace linux-tools-$(uname -r)"
        exit 1
    fi
}

# Check if VexFS module is loaded
check_vexfs_module() {
    if ! lsmod | grep -q "vexfs"; then
        print_color "$YELLOW" "⚠️  Warning: VexFS kernel module not detected"
        print_color "$BLUE" "ℹ️  Some traces may not capture VexFS-specific events"
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        print_color "$GREEN" "✅ VexFS kernel module detected"
    fi
}

# Display help
show_help() {
    cat << EOF
VexFS eBPF Tracing Manager

USAGE:
    $0 [COMMAND] [OPTIONS]

COMMANDS:
    list                    List available tracing scripts
    run <script>           Run a specific tracing script
    monitor                Start comprehensive monitoring
    performance            Run performance-focused tracing
    memory                 Run memory analysis tracing
    custom <script>        Run custom bpftrace script
    analyze <result>       Analyze tracing results
    status                 Show current tracing status
    stop                   Stop all running traces
    clean                  Clean up old results
    help                   Show this help message

TRACING SCRIPTS:
    kernel                 General VexFS kernel operations
    performance            High-performance vector operations
    memory                 Memory allocation and leak detection

OPTIONS:
    -d, --duration <sec>   Set tracing duration (default: 60s)
    -o, --output <file>    Set output file (default: auto-generated)
    -v, --verbose          Enable verbose output
    -q, --quiet            Suppress non-essential output
    --no-module-check      Skip VexFS module check

EXAMPLES:
    $0 run kernel                    # Run kernel tracing for 60s
    $0 run performance -d 300        # Run performance tracing for 5 minutes
    $0 monitor                       # Start comprehensive monitoring
    $0 analyze results/trace_*.txt   # Analyze specific results

REQUIREMENTS:
    - Root privileges (sudo)
    - bpftrace v0.20.2+
    - VexFS kernel module (recommended)
    - Linux kernel 6.11+

EOF
}

# List available scripts
list_scripts() {
    print_color "$CYAN" "📋 Available VexFS Tracing Scripts:"
    echo
    
    local scripts=(
        "kernel:General VexFS kernel operations tracing"
        "performance:High-performance vector operations analysis"
        "memory:Memory allocation and leak detection"
    )
    
    for script_info in "${scripts[@]}"; do
        IFS=':' read -r script_name script_desc <<< "$script_info"
        local script_file="$TRACE_SCRIPTS_DIR/vexfs_${script_name}_trace.bt"
        
        if [[ -f "$script_file" ]]; then
            print_color "$GREEN" "  ✅ $script_name"
            print_color "$NC" "     $script_desc"
            print_color "$NC" "     File: $script_file"
        else
            print_color "$RED" "  ❌ $script_name"
            print_color "$NC" "     $script_desc"
            print_color "$NC" "     File: $script_file (missing)"
        fi
        echo
    done
}

# Generate output filename
generate_output_filename() {
    local script_name="$1"
    local timestamp=$(date '+%Y%m%d_%H%M%S')
    echo "$RESULTS_DIR/vexfs_${script_name}_trace_${timestamp}.txt"
}

# Run tracing script
run_trace() {
    local script_name="$1"
    local duration="${2:-60}"
    local output_file="${3:-$(generate_output_filename "$script_name")}"
    local script_file="$TRACE_SCRIPTS_DIR/vexfs_${script_name}_trace.bt"
    
    if [[ ! -f "$script_file" ]]; then
        print_color "$RED" "❌ Error: Script not found: $script_file"
        exit 1
    fi
    
    print_color "$BLUE" "🚀 Starting VexFS $script_name tracing..."
    print_color "$NC" "   Script: $script_file"
    print_color "$NC" "   Duration: ${duration}s"
    print_color "$NC" "   Output: $output_file"
    echo
    
    log "INFO" "Starting $script_name trace (duration: ${duration}s, output: $output_file)"
    
    # Create PID file for tracking
    local pid_file="$RESULTS_DIR/.trace_${script_name}.pid"
    
    # Run bpftrace with timeout
    timeout "$duration" bpftrace "$script_file" > "$output_file" 2>&1 &
    local trace_pid=$!
    echo "$trace_pid" > "$pid_file"
    
    print_color "$GREEN" "✅ Tracing started (PID: $trace_pid)"
    print_color "$YELLOW" "⏱️  Tracing will run for ${duration} seconds..."
    
    # Wait for completion or handle interruption
    if wait "$trace_pid" 2>/dev/null; then
        print_color "$GREEN" "✅ Tracing completed successfully"
        log "INFO" "$script_name trace completed successfully"
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            print_color "$GREEN" "✅ Tracing completed (timeout reached)"
            log "INFO" "$script_name trace completed via timeout"
        else
            print_color "$RED" "❌ Tracing failed (exit code: $exit_code)"
            log "ERROR" "$script_name trace failed with exit code $exit_code"
        fi
    fi
    
    # Cleanup PID file
    rm -f "$pid_file"
    
    # Show results summary
    if [[ -f "$output_file" ]]; then
        local file_size=$(du -h "$output_file" | cut -f1)
        local line_count=$(wc -l < "$output_file")
        print_color "$CYAN" "📊 Results Summary:"
        print_color "$NC" "   File: $output_file"
        print_color "$NC" "   Size: $file_size"
        print_color "$NC" "   Lines: $line_count"
        echo
        
        # Show last few lines as preview
        print_color "$CYAN" "📋 Preview (last 10 lines):"
        tail -n 10 "$output_file" | sed 's/^/   /'
    fi
}

# Start comprehensive monitoring
start_monitoring() {
    local duration="${1:-300}"  # Default 5 minutes
    
    print_color "$PURPLE" "🔍 Starting Comprehensive VexFS Monitoring..."
    print_color "$NC" "   Duration: ${duration}s"
    echo
    
    # Run all traces in parallel
    local pids=()
    local scripts=("kernel" "performance" "memory")
    
    for script in "${scripts[@]}"; do
        local output_file=$(generate_output_filename "monitor_${script}")
        print_color "$BLUE" "   Starting $script monitoring..."
        
        timeout "$duration" bpftrace "$TRACE_SCRIPTS_DIR/vexfs_${script}_trace.bt" > "$output_file" 2>&1 &
        local pid=$!
        pids+=("$pid")
        echo "$pid" > "$RESULTS_DIR/.monitor_${script}.pid"
        
        log "INFO" "Started monitoring $script (PID: $pid)"
    done
    
    print_color "$GREEN" "✅ All monitoring scripts started"
    print_color "$YELLOW" "⏱️  Monitoring will run for ${duration} seconds..."
    
    # Wait for all to complete
    for pid in "${pids[@]}"; do
        wait "$pid" 2>/dev/null || true
    done
    
    # Cleanup PID files
    rm -f "$RESULTS_DIR"/.monitor_*.pid
    
    print_color "$GREEN" "✅ Comprehensive monitoring completed"
    log "INFO" "Comprehensive monitoring completed"
    
    # Generate summary report
    generate_monitoring_summary
}

# Generate monitoring summary
generate_monitoring_summary() {
    local summary_file="$RESULTS_DIR/monitoring_summary_$(date '+%Y%m%d_%H%M%S').txt"
    
    print_color "$CYAN" "📊 Generating monitoring summary..."
    
    {
        echo "=== VexFS Comprehensive Monitoring Summary ==="
        echo "Generated: $(date)"
        echo "Duration: Comprehensive monitoring session"
        echo
        
        echo "=== Available Result Files ==="
        find "$RESULTS_DIR" -name "vexfs_monitor_*_$(date '+%Y%m%d')*.txt" -type f | while read -r file; do
            local size=$(du -h "$file" | cut -f1)
            local lines=$(wc -l < "$file")
            echo "  $(basename "$file"): $size, $lines lines"
        done
        
        echo
        echo "=== Quick Analysis ==="
        echo "Use './vexfs_trace_manager.sh analyze <result_file>' for detailed analysis"
        
    } > "$summary_file"
    
    print_color "$GREEN" "✅ Summary generated: $summary_file"
}

# Show current status
show_status() {
    print_color "$CYAN" "📊 VexFS Tracing Status"
    echo
    
    # Check for running traces
    local running_traces=()
    for pid_file in "$RESULTS_DIR"/.trace_*.pid "$RESULTS_DIR"/.monitor_*.pid; do
        if [[ -f "$pid_file" ]]; then
            local pid=$(cat "$pid_file")
            if kill -0 "$pid" 2>/dev/null; then
                local script_name=$(basename "$pid_file" .pid | sed 's/^\.trace_//' | sed 's/^\.monitor_//')
                running_traces+=("$script_name (PID: $pid)")
            else
                rm -f "$pid_file"  # Clean up stale PID file
            fi
        fi
    done
    
    if [[ ${#running_traces[@]} -gt 0 ]]; then
        print_color "$GREEN" "🟢 Running Traces:"
        for trace in "${running_traces[@]}"; do
            print_color "$NC" "   $trace"
        done
    else
        print_color "$YELLOW" "🟡 No traces currently running"
    fi
    
    echo
    
    # Show recent results
    print_color "$CYAN" "📁 Recent Results:"
    find "$RESULTS_DIR" -name "vexfs_*_trace_*.txt" -type f -mtime -1 | head -5 | while read -r file; do
        local size=$(du -h "$file" | cut -f1)
        local age=$(stat -c %y "$file" | cut -d' ' -f1-2)
        print_color "$NC" "   $(basename "$file") ($size, $age)"
    done
    
    if [[ ! $(find "$RESULTS_DIR" -name "vexfs_*_trace_*.txt" -type f -mtime -1 | head -1) ]]; then
        print_color "$NC" "   No recent results found"
    fi
}

# Stop all running traces
stop_traces() {
    print_color "$YELLOW" "🛑 Stopping all VexFS traces..."
    
    local stopped_count=0
    for pid_file in "$RESULTS_DIR"/.trace_*.pid "$RESULTS_DIR"/.monitor_*.pid; do
        if [[ -f "$pid_file" ]]; then
            local pid=$(cat "$pid_file")
            local script_name=$(basename "$pid_file" .pid | sed 's/^\.trace_//' | sed 's/^\.monitor_//')
            
            if kill -0 "$pid" 2>/dev/null; then
                print_color "$NC" "   Stopping $script_name (PID: $pid)..."
                kill -TERM "$pid" 2>/dev/null || true
                sleep 2
                if kill -0 "$pid" 2>/dev/null; then
                    kill -KILL "$pid" 2>/dev/null || true
                fi
                ((stopped_count++))
                log "INFO" "Stopped trace $script_name (PID: $pid)"
            fi
            rm -f "$pid_file"
        fi
    done
    
    if [[ $stopped_count -gt 0 ]]; then
        print_color "$GREEN" "✅ Stopped $stopped_count trace(s)"
    else
        print_color "$YELLOW" "🟡 No running traces found"
    fi
}

# Clean up old results
clean_results() {
    local days="${1:-7}"
    
    print_color "$YELLOW" "🧹 Cleaning up results older than $days days..."
    
    local deleted_count=0
    while IFS= read -r -d '' file; do
        print_color "$NC" "   Removing: $(basename "$file")"
        rm -f "$file"
        ((deleted_count++))
    done < <(find "$RESULTS_DIR" -name "vexfs_*_trace_*.txt" -type f -mtime +$days -print0)
    
    # Also clean up old log entries
    if [[ -f "$LOG_FILE" ]] && [[ $(wc -l < "$LOG_FILE") -gt 1000 ]]; then
        tail -n 500 "$LOG_FILE" > "$LOG_FILE.tmp"
        mv "$LOG_FILE.tmp" "$LOG_FILE"
        print_color "$NC" "   Trimmed log file to last 500 entries"
    fi
    
    if [[ $deleted_count -gt 0 ]]; then
        print_color "$GREEN" "✅ Cleaned up $deleted_count old result file(s)"
    else
        print_color "$YELLOW" "🟡 No old results found to clean"
    fi
}

# Basic analysis of results
analyze_results() {
    local result_file="$1"
    
    if [[ ! -f "$result_file" ]]; then
        print_color "$RED" "❌ Error: Result file not found: $result_file"
        exit 1
    fi
    
    print_color "$CYAN" "🔍 Analyzing VexFS Trace Results"
    print_color "$NC" "   File: $result_file"
    echo
    
    local analysis_file="${result_file%.txt}_analysis.txt"
    
    {
        echo "=== VexFS Trace Analysis ==="
        echo "Source: $result_file"
        echo "Generated: $(date)"
        echo "File size: $(du -h "$result_file" | cut -f1)"
        echo "Total lines: $(wc -l < "$result_file")"
        echo
        
        echo "=== Error Analysis ==="
        local error_count=$(grep -c "ERROR\|❌\|🚨" "$result_file" 2>/dev/null || echo "0")
        echo "Total errors: $error_count"
        if [[ $error_count -gt 0 ]]; then
            echo "Error samples:"
            grep "ERROR\|❌\|🚨" "$result_file" | head -5 | sed 's/^/  /'
        fi
        echo
        
        echo "=== Performance Highlights ==="
        local slow_ops=$(grep -c "SLOW\|⚠️.*ms" "$result_file" 2>/dev/null || echo "0")
        echo "Slow operations: $slow_ops"
        if [[ $slow_ops -gt 0 ]]; then
            echo "Slow operation samples:"
            grep "SLOW\|⚠️.*ms" "$result_file" | head -3 | sed 's/^/  /'
        fi
        echo
        
        echo "=== Memory Analysis ==="
        local memory_events=$(grep -c "ALLOC\|FREE\|MEMORY" "$result_file" 2>/dev/null || echo "0")
        echo "Memory events: $memory_events"
        
        local oom_events=$(grep -c "OUT_OF_MEMORY\|OOM" "$result_file" 2>/dev/null || echo "0")
        if [[ $oom_events -gt 0 ]]; then
            echo "⚠️  Out of memory events: $oom_events"
        fi
        echo
        
        echo "=== Operation Summary ==="
        echo "Vector operations: $(grep -c "VECTOR" "$result_file" 2>/dev/null || echo "0")"
        echo "I/O operations: $(grep -c "READ\|WRITE" "$result_file" 2>/dev/null || echo "0")"
        echo "Search operations: $(grep -c "SEARCH" "$result_file" 2>/dev/null || echo "0")"
        echo "Lock operations: $(grep -c "LOCK" "$result_file" 2>/dev/null || echo "0")"
        echo
        
        echo "=== Recommendations ==="
        if [[ $error_count -gt 10 ]]; then
            echo "- High error rate detected, investigate error patterns"
        fi
        if [[ $slow_ops -gt 5 ]]; then
            echo "- Multiple slow operations detected, consider performance optimization"
        fi
        if [[ $oom_events -gt 0 ]]; then
            echo "- Memory pressure detected, investigate memory usage patterns"
        fi
        if [[ $error_count -eq 0 && $slow_ops -eq 0 ]]; then
            echo "- No significant issues detected in this trace"
        fi
        
    } > "$analysis_file"
    
    print_color "$GREEN" "✅ Analysis completed: $analysis_file"
    
    # Show summary
    print_color "$CYAN" "📊 Quick Summary:"
    grep -E "Total (errors|lines):|Slow operations:|Memory events:" "$analysis_file" | sed 's/^/   /'
}

# Main function
main() {
    local command="${1:-help}"
    shift || true
    
    # Parse global options
    local duration=60
    local output_file=""
    local verbose=false
    local quiet=false
    local skip_module_check=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--duration)
                duration="$2"
                shift 2
                ;;
            -o|--output)
                output_file="$2"
                shift 2
                ;;
            -v|--verbose)
                verbose=true
                shift
                ;;
            -q|--quiet)
                quiet=true
                shift
                ;;
            --no-module-check)
                skip_module_check=true
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                break
                ;;
        esac
    done
    
    # Handle commands that don't require root
    case $command in
        help|list|status)
            ;;
        *)
            check_root
            check_dependencies
            if [[ "$skip_module_check" != true ]]; then
                check_vexfs_module
            fi
            ;;
    esac
    
    # Execute command
    case $command in
        list)
            list_scripts
            ;;
        run)
            local script_name="${1:-}"
            if [[ -z "$script_name" ]]; then
                print_color "$RED" "❌ Error: Script name required"
                print_color "$YELLOW" "💡 Usage: $0 run <script_name>"
                exit 1
            fi
            run_trace "$script_name" "$duration" "$output_file"
            ;;
        monitor)
            start_monitoring "$duration"
            ;;
        performance)
            run_trace "performance" "$duration" "$output_file"
            ;;
        memory)
            run_trace "memory" "$duration" "$output_file"
            ;;
        analyze)
            local result_file="${1:-}"
            if [[ -z "$result_file" ]]; then
                print_color "$RED" "❌ Error: Result file required"
                print_color "$YELLOW" "💡 Usage: $0 analyze <result_file>"
                exit 1
            fi
            analyze_results "$result_file"
            ;;
        status)
            show_status
            ;;
        stop)
            stop_traces
            ;;
        clean)
            local days="${1:-7}"
            clean_results "$days"
            ;;
        help)
            show_help
            ;;
        *)
            print_color "$RED" "❌ Error: Unknown command: $command"
            print_color "$YELLOW" "💡 Use '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Handle interruption
trap 'print_color "$YELLOW" "\n🛑 Interrupted. Stopping traces..."; stop_traces; exit 130' INT TERM

# Initialize logging
log "INFO" "VexFS Trace Manager started with args: $*"

# Run main function
main "$@"