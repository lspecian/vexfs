#!/bin/bash

echo "VexFS Codebase Cleanup - Phase 3: Code Integration & Cleanup"
echo "============================================================"

# Step 1: Remove the old vexctl directory (now integrated into rust/)
echo "Removing old vexctl directory..."
if [ -d "vexctl/" ]; then
    rm -rf vexctl/
    echo "Removed vexctl/ directory (functionality moved to rust/)"
fi

# Step 2: Fix module imports in the integrated vexctl code
echo "Fixing module imports in integrated vexctl code..."

# Update vexctl.rs to use the new module paths
if [ -f "rust/src/bin/vexctl.rs" ]; then
    # Update imports to use the new module structure
    sed -i 's/use vexctl::/use crate::/g' rust/src/bin/vexctl.rs
    sed -i 's/mod error;/use crate::vexctl_error as error;/g' rust/src/bin/vexctl.rs
    echo "Updated imports in vexctl.rs"
fi

# Step 3: Update rust/src/lib.rs to include the new modules
echo "Updating rust/src/lib.rs to include vexctl modules..."
if [ -f "rust/src/lib.rs" ]; then
    # Add vexctl modules to lib.rs if not already present
    if ! grep -q "pub mod client;" rust/src/lib.rs; then
        echo "" >> rust/src/lib.rs
        echo "// VexCtl modules" >> rust/src/lib.rs
        echo "pub mod client;" >> rust/src/lib.rs
        echo "pub mod commands;" >> rust/src/lib.rs
        echo "pub mod output;" >> rust/src/lib.rs
        echo "pub mod vexctl_error;" >> rust/src/lib.rs
        echo "Added vexctl modules to lib.rs"
    fi
fi

# Step 4: Verify the rust workspace structure
echo "Verifying rust workspace structure..."
echo "Current rust/src/ structure:"
ls -la rust/src/

echo ""
echo "Current rust/src/bin/ binaries:"
ls -la rust/src/bin/

# Step 5: Test compilation of the consolidated workspace
echo ""
echo "Testing compilation of consolidated rust workspace..."
cd rust
if cargo check --quiet; then
    echo "✅ Rust workspace compiles successfully!"
else
    echo "❌ Compilation issues detected - will need manual fixes"
fi
cd ..

echo ""
echo "Phase 3 integration completed!"
echo "Summary of changes:"
echo "- Removed old vexctl/ directory"
echo "- Fixed module imports in integrated code"
echo "- Updated lib.rs to include vexctl modules"
echo "- Verified workspace structure"
echo ""
echo "Next: Test and document the clean architecture"