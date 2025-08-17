#!/bin/bash

# VexFS Cleanup Script
# Archives legacy code and documentation for a cleaner codebase

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "╔══════════════════════════════════════════════════════╗"
echo "║          VexFS Codebase Cleanup Tool                  ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Create archive directory
ARCHIVE_DIR="archive/$(date +%Y%m%d)"
mkdir -p "$ARCHIVE_DIR"

# Function to show size before cleanup
show_stats() {
    echo -e "${BLUE}Current codebase statistics:${NC}"
    echo "  Rust files: $(find rust/src -name "*.rs" -type f 2>/dev/null | wc -l)"
    echo "  Doc files: $(find docs -type f 2>/dev/null | wc -l)"
    echo "  Total size: $(du -sh . 2>/dev/null | cut -f1)"
    echo "  Git objects: $(git count-objects -v | grep size-pack | cut -d: -f2)"
    echo
}

# Function to archive aspirational docs
archive_docs() {
    echo -e "${BLUE}Archiving aspirational documentation...${NC}"
    
    local docs_to_archive=(
        "docs/implementation"
        "docs/architecture/advanced"
        "docs/api/v2"
        "docs/FINAL_*.md"
        "docs/archive"
        "docs/deprecation"
    )
    
    for doc in "${docs_to_archive[@]}"; do
        if [ -e "$doc" ]; then
            echo "  Archiving $doc"
            mkdir -p "$ARCHIVE_DIR/docs"
            mv "$doc" "$ARCHIVE_DIR/docs/" 2>/dev/null || true
        fi
    done
    
    echo -e "${GREEN}✓ Documentation archived${NC}"
}

# Function to remove redundant implementations
remove_redundant() {
    echo -e "${BLUE}Removing redundant implementations...${NC}"
    
    local files_to_remove=(
        # Duplicate vector implementations
        "rust/src/anns"
        "rust/src/enhanced_vector_*.rs"
        "rust/src/hybrid_*.rs"
        "rust/src/query_planner.rs"
        "rust/src/query_optimizer.rs"
        "rust/src/query_monitor.rs"
        
        # Unused features
        "rust/src/vexgraph"
        "rust/src/domain"
        "rust/src/ipc"
        "rust/src/commands"
        "rust/src/semantic_api"
        "rust/src/client"
        
        # Test files in wrong location
        "rust/src/*_test.rs"
        "rust/src/bin/task_*.rs"
    )
    
    for file in "${files_to_remove[@]}"; do
        if [ -e "$file" ]; then
            echo "  Moving $file to archive"
            mkdir -p "$ARCHIVE_DIR/redundant"
            mv $file "$ARCHIVE_DIR/redundant/" 2>/dev/null || true
        fi
    done
    
    echo -e "${GREEN}✓ Redundant code archived${NC}"
}

# Function to clean build artifacts
clean_build() {
    echo -e "${BLUE}Cleaning build artifacts...${NC}"
    
    # Rust target directory
    if [ -d "rust/target" ]; then
        echo "  Cleaning Rust build (this may take a while)..."
        cd rust && cargo clean && cd ..
    fi
    
    # Kernel module objects
    if [ -d "kernel_module" ]; then
        echo "  Cleaning kernel module build..."
        cd kernel_module && make clean 2>/dev/null || true && cd ..
    fi
    
    # VM testing artifacts
    if [ -d "vm_testing/old_vms" ]; then
        echo "  Removing old VM images..."
        rm -rf vm_testing/old_vms
    fi
    
    # Node modules if they exist
    if [ -d "vexfs-dashboard/node_modules" ]; then
        echo "  Removing node_modules..."
        rm -rf vexfs-dashboard/node_modules
    fi
    
    echo -e "${GREEN}✓ Build artifacts cleaned${NC}"
}

# Function to consolidate test files
consolidate_tests() {
    echo -e "${BLUE}Consolidating test files...${NC}"
    
    # Create proper test structure
    mkdir -p tests/{unit,integration,benchmarks}
    
    # Move test files from src
    find rust/src -name "*_test.rs" -exec mv {} tests/unit/ \; 2>/dev/null || true
    find rust/src -name "*_bench.rs" -exec mv {} tests/benchmarks/ \; 2>/dev/null || true
    
    # Move integration tests
    [ -f "tests/integration_test_suite.sh" ] && mv tests/*.sh tests/integration/ 2>/dev/null || true
    
    echo -e "${GREEN}✓ Tests consolidated${NC}"
}

# Function to create clean structure
create_clean_structure() {
    echo -e "${BLUE}Creating clean directory structure...${NC}"
    
    # Ensure clean directories exist
    mkdir -p {src,tests,docs,scripts,docker}
    mkdir -p src/{fuse,api,storage,vector,shared}
    mkdir -p docs/{api,guides,development}
    mkdir -p tests/{unit,integration,benchmarks}
    
    # Move scripts to scripts directory
    mv *.sh scripts/ 2>/dev/null || true
    mv scripts/cleanup.sh . # Keep cleanup script in root
    
    echo -e "${GREEN}✓ Clean structure created${NC}"
}

# Function to update gitignore
update_gitignore() {
    echo -e "${BLUE}Updating .gitignore...${NC}"
    
    cat >> .gitignore << EOF

# Archive
/archive/

# Build artifacts
/rust/target/
/kernel_module/*.o
/kernel_module/*.ko
/kernel_module/*.mod*
*.pyc
__pycache__/

# IDE
.vscode/
.idea/
*.swp
*.swo

# Logs
*.log
/logs/

# Temporary
/tmp/
/temp/
EOF
    
    echo -e "${GREEN}✓ .gitignore updated${NC}"
}

# Main menu
show_menu() {
    echo
    echo "Select cleanup operations:"
    echo "1) Archive aspirational documentation"
    echo "2) Remove redundant implementations"
    echo "3) Clean build artifacts"
    echo "4) Consolidate test files"
    echo "5) Create clean directory structure"
    echo "6) Full cleanup (all of the above)"
    echo "7) Show statistics"
    echo "q) Quit"
    echo
    read -p "Option: " choice
    
    case $choice in
        1) archive_docs ;;
        2) remove_redundant ;;
        3) clean_build ;;
        4) consolidate_tests ;;
        5) create_clean_structure ;;
        6) 
            archive_docs
            remove_redundant
            clean_build
            consolidate_tests
            create_clean_structure
            update_gitignore
            ;;
        7) show_stats ;;
        q) exit 0 ;;
        *) echo -e "${RED}Invalid option${NC}" ;;
    esac
}

# Confirmation prompt
confirm_cleanup() {
    echo -e "${YELLOW}⚠️  Warning: This will restructure the codebase${NC}"
    echo "Archive will be created at: $ARCHIVE_DIR"
    echo
    read -p "Continue with cleanup? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cleanup cancelled"
        exit 0
    fi
}

# Main execution
main() {
    show_stats
    
    if [ "$1" = "--auto" ]; then
        confirm_cleanup
        archive_docs
        remove_redundant
        clean_build
        consolidate_tests
        create_clean_structure
        update_gitignore
        echo
        show_stats
        echo -e "${GREEN}✓ Cleanup complete!${NC}"
        echo "Archive created at: $ARCHIVE_DIR"
    else
        while true; do
            show_menu
        done
    fi
}

main "$@"