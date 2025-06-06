#!/bin/bash
set -e

echo "🚀 Starting VexFS Server..."
echo "📊 Running VexFS Test Suite..."
/usr/local/bin/vexfs_test_runner

echo ""
echo "🔍 Running VexFS ANNS Benchmark..."
/usr/local/bin/vexfs_benchmark

echo ""
echo "✅ VexFS Server Ready!"
echo "📡 VexFS is running and ready to accept connections"
echo "🌐 Access VexFS functionality through the test runners"

# Keep the container running
while true; do
    echo "$(date): VexFS Server is running..."
    sleep 30
done