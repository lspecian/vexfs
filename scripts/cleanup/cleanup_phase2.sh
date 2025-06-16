#!/bin/bash

echo "VexFS Codebase Cleanup - Phase 2: Consolidation"
echo "==============================================="

# Step 1: Move documentation from core/ and search/ to docs/
echo "Moving documentation to docs/ directory..."
if [ -f "core/README.md" ]; then
    mv core/README.md docs/architecture/CORE_ARCHITECTURE.md
    echo "Moved core/README.md to docs/architecture/CORE_ARCHITECTURE.md"
fi

if [ -f "search/README.md" ]; then
    mv search/README.md docs/architecture/SEARCH_ARCHITECTURE.md
    echo "Moved search/README.md to docs/architecture/SEARCH_ARCHITECTURE.md"
fi

# Step 2: Remove empty core/ and search/ directories
echo "Removing empty core/ and search/ directories..."
rmdir core/ 2>/dev/null && echo "Removed core/ directory"
rmdir search/ 2>/dev/null && echo "Removed search/ directory"

# Step 3: Move vexctl functionality to rust workspace
echo "Integrating vexctl into rust workspace..."

# Create vexctl binary in rust/src/bin/
if [ -f "vexctl/src/main.rs" ]; then
    cp vexctl/src/main.rs rust/src/bin/vexctl.rs
    echo "Copied vexctl main to rust/src/bin/vexctl.rs"
fi

# Move vexctl modules to rust/src/
if [ -d "vexctl/src/client" ]; then
    cp -r vexctl/src/client rust/src/
    echo "Copied vexctl client module to rust/src/"
fi

if [ -d "vexctl/src/commands" ]; then
    cp -r vexctl/src/commands rust/src/
    echo "Copied vexctl commands module to rust/src/"
fi

if [ -d "vexctl/src/output" ]; then
    cp -r vexctl/src/output rust/src/
    echo "Copied vexctl output module to rust/src/"
fi

if [ -f "vexctl/src/error.rs" ]; then
    cp vexctl/src/error.rs rust/src/vexctl_error.rs
    echo "Copied vexctl error module to rust/src/vexctl_error.rs"
fi

# Step 4: Clean up duplicate server binaries (keep only unified server)
echo "Cleaning up duplicate server binaries..."
rm -f rust/src/bin/vexfs_server.rs
rm -f rust/src/bin/vexfs_server_enhanced.rs
echo "Removed duplicate server binaries (keeping vexfs_unified_server.rs)"

# Step 5: Update rust/Cargo.toml to include vexctl binary
echo "Updating rust/Cargo.toml..."
if ! grep -q "name = \"vexctl\"" rust/Cargo.toml; then
    # Add vexctl binary to Cargo.toml
    sed -i '/\[\[bin\]\]/a\\n[[bin]]\nname = "vexctl"\npath = "src/bin/vexctl.rs"' rust/Cargo.toml
    echo "Added vexctl binary to rust/Cargo.toml"
fi

echo ""
echo "Phase 2 consolidation completed!"
echo "Summary of changes:"
echo "- Moved documentation from core/ and search/ to docs/architecture/"
echo "- Integrated vexctl into rust workspace"
echo "- Removed duplicate server binaries"
echo "- Updated Cargo.toml configuration"
echo ""
echo "Next: Review and test the consolidated structure"