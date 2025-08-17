#!/bin/bash

# VexFS Documentation Cleanup Script
# Consolidates and organizes the excessive markdown documentation

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "╔══════════════════════════════════════════════════════╗"
echo "║       VexFS Documentation Cleanup Tool                ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Create archive for docs
ARCHIVE_DIR="archive/docs-$(date +%Y%m%d)"
mkdir -p "$ARCHIVE_DIR"

# Stats
show_doc_stats() {
    echo -e "${BLUE}Documentation statistics:${NC}"
    echo "  Total .md files: $(find . -name '*.md' -type f ! -path './archive/*' ! -path './node_modules/*' ! -path './target/*' ! -path './benchmarks/venv/*' ! -path './bindings/*/node_modules/*' 2>/dev/null | wc -l)"
    echo "  Root .md files: $(ls *.md 2>/dev/null | wc -l)"
    echo "  Docs folder files: $(find docs -name '*.md' 2>/dev/null | wc -l)"
    echo
}

# Archive redundant root-level docs
archive_root_docs() {
    echo -e "${BLUE}Archiving redundant root-level docs...${NC}"
    
    # Keep only essential root docs
    local keep_files=(
        "README.md"
        "CLAUDE.md"
        "PRODUCTION_CHECKLIST.md"
        "VEXGRAPH_ROADMAP.md"
        "FUSE_STATUS.md"
    )
    
    # Archive others
    for file in *.md; do
        if [[ ! " ${keep_files[@]} " =~ " ${file} " ]]; then
            echo "  Archiving $file"
            mv "$file" "$ARCHIVE_DIR/" 2>/dev/null || true
        fi
    done
    
    echo -e "${GREEN}✓ Root docs cleaned${NC}"
}

# Clean docs folder
clean_docs_folder() {
    echo -e "${BLUE}Cleaning docs folder...${NC}"
    
    # Archive entire categories
    local dirs_to_archive=(
        "docs/architecture"  # 50+ aspirational architecture docs
        "docs/status"        # Old status reports
        "docs/testing"       # 40+ redundant test docs
        "docs/api"          # Task-specific API docs
        "docs/fs"           # DDD refactoring docs
        "docs/operations"   # Old operations guides
        "docs/user-guide/docs"  # Nested duplicate docs
        "docs/integration"  # Old integration docs
        "docs/inventory"    # Inventory docs
        "docs/troubleshooting"  # Task-specific troubleshooting
        "docs/tutorials"    # Incomplete tutorials
    )
    
    for dir in "${dirs_to_archive[@]}"; do
        if [ -d "$dir" ]; then
            echo "  Archiving $dir"
            mkdir -p "$ARCHIVE_DIR/$(dirname $dir)"
            mv "$dir" "$ARCHIVE_DIR/$(dirname $dir)/" 2>/dev/null || true
        fi
    done
    
    # Archive old TASK_* files
    find docs -name "TASK_*.md" -exec mv {} "$ARCHIVE_DIR/" \; 2>/dev/null || true
    find docs -name "FINAL_*.md" -exec mv {} "$ARCHIVE_DIR/" \; 2>/dev/null || true
    
    echo -e "${GREEN}✓ Docs folder cleaned${NC}"
}

# Archive test results
archive_test_results() {
    echo -e "${BLUE}Archiving old test results...${NC}"
    
    if [ -d "test_results" ]; then
        mv test_results "$ARCHIVE_DIR/" 2>/dev/null || true
        echo -e "${GREEN}✓ Test results archived${NC}"
    fi
    
    # Archive old test docs
    find tests -name "*.md" ! -name "README.md" -exec mv {} "$ARCHIVE_DIR/" \; 2>/dev/null || true
}

# Archive VM testing docs
archive_vm_docs() {
    echo -e "${BLUE}Archiving VM testing docs...${NC}"
    
    find vm_testing -name "*.md" ! -name "README.md" ! -name "INSTALLATION_GUIDE.md" -exec mv {} "$ARCHIVE_DIR/" \; 2>/dev/null || true
    
    # Remove duplicate shared folder
    if [ -d "vm_testing/shared" ]; then
        rm -rf vm_testing/shared 2>/dev/null || true
    fi
    
    echo -e "${GREEN}✓ VM docs cleaned${NC}"
}

# Archive other redundant docs
archive_misc_docs() {
    echo -e "${BLUE}Archiving miscellaneous docs...${NC}"
    
    # Archive profiling docs
    if [ -d "profiling/docs" ]; then
        mv profiling/docs "$ARCHIVE_DIR/profiling_docs" 2>/dev/null || true
    fi
    
    # Archive kernel module docs except README
    find kernel_module -name "*.md" ! -name "README.md" -depth -exec mv {} "$ARCHIVE_DIR/" \; 2>/dev/null || true
    
    # Archive dashboard docs
    if [ -d "vexfs-dashboard/docs" ]; then
        mv vexfs-dashboard/docs "$ARCHIVE_DIR/dashboard_docs" 2>/dev/null || true
    fi
    
    # Archive developer-package (duplicate of docs)
    if [ -d "developer-package" ]; then
        mv developer-package "$ARCHIVE_DIR/" 2>/dev/null || true
    fi
    
    echo -e "${GREEN}✓ Misc docs cleaned${NC}"
}

# Create new clean doc structure
create_clean_docs() {
    echo -e "${BLUE}Creating clean documentation structure...${NC}"
    
    # Ensure clean structure
    mkdir -p docs/{api,guides,reference}
    
    # Create consolidated docs
    cat > docs/README.md << 'EOF'
# VexFS Documentation

## Essential Documents

### Getting Started
- [Main README](../README.md) - Project overview and quick start
- [FUSE Status](../FUSE_STATUS.md) - Current FUSE implementation status
- [Production Checklist](../PRODUCTION_CHECKLIST.md) - Production readiness checklist

### Development
- [VexGraph Roadmap](../VEXGRAPH_ROADMAP.md) - Graph database implementation plan
- [Claude Instructions](../CLAUDE.md) - AI assistant context

### API Reference
- API documentation in `api/` directory

### Guides
- User and developer guides in `guides/` directory

### Technical Reference
- Architecture and implementation details in `reference/` directory
EOF
    
    echo -e "${GREEN}✓ Clean structure created${NC}"
}

# Create doc index
create_doc_index() {
    echo -e "${BLUE}Creating documentation index...${NC}"
    
    cat > DOCUMENTATION.md << 'EOF'
# VexFS Documentation Index

## Core Documentation (5 files)
- `README.md` - Main project documentation
- `CLAUDE.md` - AI assistant instructions
- `PRODUCTION_CHECKLIST.md` - Production readiness
- `VEXGRAPH_ROADMAP.md` - Graph database roadmap
- `FUSE_STATUS.md` - FUSE implementation status

## Documentation Folders
- `docs/api/` - API reference
- `docs/guides/` - User and developer guides
- `docs/reference/` - Technical reference

## Component Documentation
- `kernel_module/README.md` - Kernel module documentation
- `rust/src/fuse_README.md` - FUSE implementation
- `tests/README.md` - Testing documentation
- `vm_testing/README.md` - VM testing guide

## Archived
All other documentation has been archived to `archive/docs-YYYYMMDD/`
EOF
    
    echo -e "${GREEN}✓ Documentation index created${NC}"
}

# Main execution
main() {
    show_doc_stats
    
    echo -e "${YELLOW}This will archive ~250+ markdown files${NC}"
    echo "Archive location: $ARCHIVE_DIR"
    echo
    read -p "Continue? (y/N): " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled"
        exit 0
    fi
    
    archive_root_docs
    clean_docs_folder
    archive_test_results
    archive_vm_docs
    archive_misc_docs
    create_clean_docs
    create_doc_index
    
    echo
    show_doc_stats
    
    echo -e "${GREEN}✓ Documentation cleanup complete!${NC}"
    echo "Archived to: $ARCHIVE_DIR"
}

# Handle auto mode
if [ "$1" = "--auto" ]; then
    REPLY="y"
    archive_root_docs
    clean_docs_folder
    archive_test_results
    archive_vm_docs
    archive_misc_docs
    create_clean_docs
    create_doc_index
    show_doc_stats
else
    main
fi