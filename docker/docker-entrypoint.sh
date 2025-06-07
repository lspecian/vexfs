#!/bin/bash
set -e

echo "🚀 Starting VexFS Unified Server with Dashboard on single port..."

# Use VEXFS_PORT environment variable (default 7680)
VEXFS_PORT="${VEXFS_PORT:-7680}"

# Create data directory if it doesn't exist
mkdir -p "${VEXFS_DATA_DIR:-/app/data}"

# Function to start VexFS unified server with dashboard
start_vexfs_server() {
    echo "🔧 Starting VexFS Unified Server on port ${VEXFS_PORT}..."
    
    # Start the unified server
    if command -v vexfs >/dev/null 2>&1; then
        echo "Using VexFS Unified Server..."
        # Set environment variables for the server
        export PORT="${VEXFS_PORT}"
        export DASHBOARD_PATH="/app/dashboard"
        vexfs &
        SERVER_PID=$!
        echo "VexFS Unified Server PID: $SERVER_PID"
    else
        echo "❌ VexFS unified server binary not found!"
        exit 1
    fi
}

# Function to wait for server to be ready
wait_for_server() {
    echo "⏳ Waiting for VexFS server to be ready..."
    for i in {1..30}; do
        if curl -f "http://localhost:${VEXFS_PORT}/api/v1/version" >/dev/null 2>&1; then
            echo "✅ VexFS server is ready!"
            return 0
        fi
        echo "Attempt $i/30: Server not ready yet..."
        sleep 2
    done
    echo "❌ VexFS server failed to start within 60 seconds"
    return 1
}

# Start services
start_vexfs_server

# Wait for server to be ready
if wait_for_server; then
    echo ""
    echo "🎉 VexFS Unified Server is now running!"
    echo "📡 VexFS Server & Dashboard: http://localhost:${VEXFS_PORT}"
    echo "🔍 Health Check: http://localhost:${VEXFS_PORT}/api/v1/version"
    echo "📊 Dashboard: http://localhost:${VEXFS_PORT}/"
    echo ""
    echo "📚 Supported APIs:"
    echo "   • ChromaDB-compatible API"
    echo "   • Qdrant-compatible API"
    echo "   • Native VexFS API"
    echo ""
    echo "🌐 API Endpoints:"
    echo "   GET/POST /api/v1/* (ChromaDB)"
    echo "   GET/POST /collections/* (Qdrant)"
    echo "   GET/POST /vexfs/* (Native)"
    echo ""
    echo "🎯 Single Port Configuration:"
    echo "   Port: ${VEXFS_PORT} (configurable via VEXFS_PORT env var)"
    echo "   API and Dashboard served from same port"
    echo ""
else
    echo "❌ Failed to start VexFS server"
    exit 1
fi

# Function to handle shutdown
cleanup() {
    echo "🛑 Shutting down VexFS..."
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Keep the container running and monitor processes
while true; do
    # Check if server is still running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "❌ VexFS server process died, restarting..."
        start_vexfs_server
    fi
    
    sleep 10
done
