#!/bin/bash

echo "Fixing corrupted Cargo.toml binary sections..."

# Create a clean binary section based on actual files in rust/src/bin/
cat > rust/cargo_bins_clean.toml << 'EOF'

# Binary targets
[[bin]]
name = "vexctl"
path = "src/bin/vexctl.rs"

[[bin]]
name = "vexfs_fuse"
path = "src/bin/vexfs_fuse.rs"
required-features = ["fuse_support"]

[[bin]]
name = "vexfs_unified_server"
path = "src/bin/vexfs_unified_server.rs"
required-features = ["server"]

[[bin]]
name = "mkfs_vexfs"
path = "src/bin/mkfs_vexfs.rs"
required-features = ["std"]

[[bin]]
name = "anns_benchmark_test"
path = "src/bin/anns_benchmark_test.rs"

[[bin]]
name = "comprehensive_test_runner"
path = "src/bin/comprehensive_test_runner.rs"

[[bin]]
name = "vector_test_runner"
path = "src/bin/vector_test_runner.rs"
EOF

# Remove the corrupted binary sections from Cargo.toml (from line 114 onwards)
head -n 113 rust/Cargo.toml > rust/Cargo_clean.toml

# Append the clean binary sections
cat rust/cargo_bins_clean.toml >> rust/Cargo_clean.toml

# Replace the original file
mv rust/Cargo_clean.toml rust/Cargo.toml

# Clean up temporary file
rm rust/cargo_bins_clean.toml

echo "Cargo.toml binary sections fixed!"
echo "Testing compilation..."
cd rust
if cargo check --quiet; then
    echo "✅ Rust workspace now compiles successfully!"
else
    echo "❌ Still have compilation issues - checking specific errors..."
    cargo check
fi
cd ..