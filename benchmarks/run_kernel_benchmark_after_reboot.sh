#!/bin/bash
# VexFS Kernel Module Performance Benchmark - Post Reboot
# Run this script after rebooting to test the fixed kernel module

echo "🚀 VexFS Kernel Module Performance Benchmark"
echo "=============================================="
echo "📅 $(date)"
echo ""

# Check if we're running as root or with sudo access
if [[ $EUID -eq 0 ]]; then
    echo "✅ Running with root privileges"
elif sudo -n true 2>/dev/null; then
    echo "✅ Sudo access available"
else
    echo "❌ This script requires sudo access for kernel module testing"
    echo "Please run: sudo $0"
    exit 1
fi

# Check if kernel module is loaded
echo "🔍 Checking kernel module status..."
if lsmod | grep -q vexfs; then
    echo "✅ VexFS kernel module is loaded"
    lsmod | grep vexfs
else
    echo "❌ VexFS kernel module not loaded"
    echo "Loading kernel module..."
    cd ../kernel
    sudo insmod vexfs_minimal.ko
    if [ $? -eq 0 ]; then
        echo "✅ Kernel module loaded successfully"
    else
        echo "❌ Failed to load kernel module"
        exit 1
    fi
fi

# Check filesystem registration
echo ""
echo "🔍 Checking filesystem registration..."
if cat /proc/filesystems | grep -q vexfs; then
    echo "✅ VexFS filesystem registered"
    cat /proc/filesystems | grep vexfs
else
    echo "❌ VexFS filesystem not registered"
    exit 1
fi

# Run the comprehensive benchmark
echo ""
echo "🏃 Running comprehensive kernel vs FUSE benchmark..."
cd ../benchmarks
python3 kernel_vs_fuse_benchmark.py

echo ""
echo "🎯 Benchmark complete! Check the results above."
echo "📊 Results saved to: benchmarks/kernel_vs_fuse_results_*.json"
echo ""
echo "🔄 To update competitive performance summary:"
echo "   Edit docs/status/COMPETITIVE_PERFORMANCE_EXECUTIVE_SUMMARY.md"
echo "   Add actual kernel module performance numbers"