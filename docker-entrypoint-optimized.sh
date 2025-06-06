#!/bin/bash
set -e

# Navigate to the application directory
cd /app

# Function to list available binaries
list_binaries() {
  echo "Available VexFS binaries:"
  echo "  vexfs_server - VexFS server"
  echo "  vexfs_fuse - FUSE implementation"
  echo "  vector_test_runner - Vector testing utility"
  echo "  comprehensive_test_runner - Comprehensive test suite"
}

# Default action: Start an interactive shell
if [ "$1" = "" ] || [ "$1" = "bash" ] || [ "$1" = "shell" ]; then
  echo "Starting VexFS interactive shell..."
  list_binaries
  exec /bin/bash
elif [ "$1" = "server" ]; then
  echo "Starting VexFS server..."
  shift
  exec ./vexfs_server "$@"
elif [ "$1" = "fuse" ]; then
  echo "Starting VexFS FUSE..."
  shift
  exec ./vexfs_fuse "$@"
elif [ "$1" = "vector-tests" ]; then
  echo "Running vector tests..."
  shift
  exec ./vector_test_runner "$@"
elif [ "$1" = "comprehensive-tests" ]; then
  echo "Running comprehensive tests..."
  shift
  exec ./comprehensive_test_runner "$@"
elif [ "$1" = "list" ]; then
  list_binaries
else
  # Execute any other command passed to the entrypoint
  echo "Executing command: $@"
  exec "$@"
fi