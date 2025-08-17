#!/bin/bash
set -e

# VexFS Docker Entrypoint Script
echo "Starting VexFS Unified Server..."

# Create data directory if it doesn't exist
mkdir -p "${VEXFS_DATA_DIR:-/app/data}"

# Set default values
export RUST_LOG="${RUST_LOG:-info}"
export VEXFS_PORT="${VEXFS_PORT:-7680}"
export VEXFS_DATA_DIR="${VEXFS_DATA_DIR:-/app/data}"

echo "Configuration:"
echo "  RUST_LOG: $RUST_LOG"
echo "  VEXFS_PORT: $VEXFS_PORT"
echo "  VEXFS_DATA_DIR: $VEXFS_DATA_DIR"

# Start the VexFS unified server
exec /usr/local/bin/vexfs