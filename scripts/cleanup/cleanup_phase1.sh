#!/bin/bash

echo "VexFS Codebase Cleanup - Phase 1: Safe Deletions"
echo "================================================="

# Remove duplicate Docker configurations
echo "Removing duplicate Docker configurations..."
rm -f Dockerfile.optimized Dockerfile.server Dockerfile.vexfs-server
rm -f docker-compose.vexfs.yml
rm -f docker-entrypoint-optimized.sh
rm -f vexfs-server-entrypoint.sh

# Remove duplicate build systems
echo "Removing duplicate build systems..."
rm -f Makefile.unified

# Remove orphaned binaries
echo "Removing orphaned binaries..."
rm -f mkfs.vexfs

# Remove unused development directories
echo "Removing unused development directories..."
rm -rf workbench/
rm -rf venv_fuse_test/
rm -rf rust_objects/
rm -rf unified_test_results/

# Remove any remaining backup files (if they exist)
echo "Removing backup files..."
rm -f vexfs_pre_consolidation_backup_*.tar.gz
rm -f vexfs-developer-package-*.tar.gz
rm -f vexfs_rust_combined.o

echo ""
echo "Phase 1 cleanup completed!"
echo "Remaining files:"
ls -la | grep -E "(Dockerfile|docker-|Makefile)"