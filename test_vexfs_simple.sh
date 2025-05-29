#!/bin/bash
set -e

# VexFS Simple Test Script - FUSE-based testing
# This script provides an easy way for developers to test VexFS without QEMU/VMs

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOUNT_POINT="/tmp/vexfs_test"
TEST_FILES_DIR="/tmp/vexfs_test_files"

echo "🧪 VexFS Simple Test (FUSE-based)"
echo "================================="
echo "Mount point: $MOUNT_POINT"
echo "Test files: $TEST_FILES_DIR"
echo ""

# Function to cleanup on exit
cleanup() {
    echo "🧹 Cleaning up..."
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        echo "   Unmounting VexFS..."
        fusermount -u "$MOUNT_POINT" 2>/dev/null || sudo umount "$MOUNT_POINT" 2>/dev/null || true
        sleep 1
    fi
    
    # Remove directories
    [ -d "$MOUNT_POINT" ] && rmdir "$MOUNT_POINT" 2>/dev/null || true
    [ -d "$TEST_FILES_DIR" ] && rm -rf "$TEST_FILES_DIR" 2>/dev/null || true
    
    echo "✅ Cleanup completed"
}
trap cleanup EXIT

# Check dependencies
check_dependencies() {
    echo "🔍 Checking dependencies..."
    
    # Check if FUSE is available
    if ! command -v fusermount >/dev/null 2>&1; then
        echo "❌ FUSE not found. Installing..."
        if command -v apt-get >/dev/null 2>&1; then
            sudo apt-get update && sudo apt-get install -y fuse
        elif command -v yum >/dev/null 2>&1; then
            sudo yum install -y fuse
        else
            echo "❌ Please install FUSE manually"
            exit 1
        fi
    fi
    
    # Check if user is in fuse group
    if ! groups | grep -q fuse; then
        echo "⚠️  Adding user to fuse group..."
        sudo usermod -a -G fuse "$USER"
        echo "⚠️  Please log out and log back in, then run this script again"
        exit 1
    fi
    
    echo "✅ Dependencies check passed"
}

# Build VexFS with FUSE support
build_vexfs() {
    echo "🔨 Building VexFS with FUSE support..."
    cd "$SCRIPT_DIR"
    
    if ! cargo build --features fuse_support; then
        echo "❌ Failed to build VexFS with FUSE support"
        echo ""
        echo "💡 Make sure you have:"
        echo "   - Rust toolchain installed"
        echo "   - FUSE development libraries: sudo apt-get install libfuse-dev"
        exit 1
    fi
    
    echo "✅ VexFS build completed"
}

# Setup test environment
setup_test_env() {
    echo "📁 Setting up test environment..."
    
    # Create mount point
    mkdir -p "$MOUNT_POINT"
    
    # Create test files directory
    mkdir -p "$TEST_FILES_DIR"
    
    # Create sample test files
    cat > "$TEST_FILES_DIR/sample_document.txt" << 'EOF'
VexFS is a vector-extended filesystem that provides native support for vector embeddings and similarity search operations directly at the filesystem level.
EOF

    cat > "$TEST_FILES_DIR/query_vector.vec" << 'EOF'
0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8
EOF

    cat > "$TEST_FILES_DIR/document_vector.vec" << 'EOF'
0.15,0.25,0.35,0.45,0.55,0.65,0.75,0.85
EOF

    echo "✅ Test environment ready"
}

# Mount VexFS
mount_vexfs() {
    echo "🚀 Mounting VexFS..."
    
    # Start VexFS FUSE in background
    ./target/debug/vexfs_fuse "$MOUNT_POINT" -f &
    VEXFS_PID=$!
    
    # Wait a moment for mount to complete
    sleep 2
    
    # Check if mount was successful
    if ! mountpoint -q "$MOUNT_POINT"; then
        echo "❌ Failed to mount VexFS"
        kill $VEXFS_PID 2>/dev/null || true
        exit 1
    fi
    
    echo "✅ VexFS mounted at $MOUNT_POINT"
    echo "   PID: $VEXFS_PID"
}

# Run tests
run_tests() {
    echo "🧪 Running VexFS tests..."
    
    echo "📝 Test 1: Basic file operations"
    echo "Hello VexFS!" > "$MOUNT_POINT/test.txt"
    if [ -f "$MOUNT_POINT/test.txt" ]; then
        echo "✅ File creation successful"
        content=$(cat "$MOUNT_POINT/test.txt")
        if [ "$content" = "Hello VexFS!" ]; then
            echo "✅ File read successful"
        else
            echo "❌ File read failed"
        fi
    else
        echo "❌ File creation failed"
    fi
    
    echo ""
    echo "📊 Test 2: Vector operations"
    cp "$TEST_FILES_DIR/query_vector.vec" "$MOUNT_POINT/"
    cp "$TEST_FILES_DIR/document_vector.vec" "$MOUNT_POINT/"
    
    if [ -f "$MOUNT_POINT/query_vector.vec" ] && [ -f "$MOUNT_POINT/document_vector.vec" ]; then
        echo "✅ Vector files copied successfully"
        echo "   Query vector: $(cat "$MOUNT_POINT/query_vector.vec")"
        echo "   Document vector: $(cat "$MOUNT_POINT/document_vector.vec")"
    else
        echo "❌ Vector file operations failed"
    fi
    
    echo ""
    echo "📂 Test 3: Directory listing"
    echo "Files in VexFS:"
    ls -la "$MOUNT_POINT/"
    
    echo ""
    echo "💾 Test 4: File system info"
    df -h "$MOUNT_POINT" || true
}

# Test SDK integration
test_sdk_integration() {
    echo ""
    echo "🐍 Test 5: Python SDK integration (if available)"
    
    if command -v python3 >/dev/null 2>&1; then
        cat > /tmp/test_vexfs_python.py << EOF
#!/usr/bin/env python3
import os
import sys

# Test basic file operations through VexFS mount
mount_point = "$MOUNT_POINT"

try:
    # Test file creation
    test_file = os.path.join(mount_point, "python_test.txt")
    with open(test_file, "w") as f:
        f.write("Python SDK test successful!")
    
    # Test file reading
    with open(test_file, "r") as f:
        content = f.read()
    
    print(f"✅ Python integration test passed: {content}")
    
    # Test vector file
    vector_file = os.path.join(mount_point, "python_vector.vec")
    with open(vector_file, "w") as f:
        f.write("0.1,0.2,0.3,0.4,0.5")
    
    print("✅ Python vector file created")
    
except Exception as e:
    print(f"❌ Python integration test failed: {e}")
    sys.exit(1)
EOF

        python3 /tmp/test_vexfs_python.py
        rm -f /tmp/test_vexfs_python.py
    else
        echo "⚠️  Python3 not available, skipping Python SDK test"
    fi
}

# Show usage examples
show_usage_examples() {
    echo ""
    echo "💡 VexFS Usage Examples:"
    echo "========================"
    echo ""
    echo "Basic file operations:"
    echo "  echo 'Hello World' > $MOUNT_POINT/document.txt"
    echo "  cat $MOUNT_POINT/document.txt"
    echo ""
    echo "Vector operations:"
    echo "  echo '0.1,0.2,0.3,0.4' > $MOUNT_POINT/vector.vec"
    echo "  cat $MOUNT_POINT/vector.vec"
    echo ""
    echo "Directory operations:"
    echo "  ls -la $MOUNT_POINT/"
    echo "  mkdir $MOUNT_POINT/vectors"
    echo ""
    echo "SDK integration:"
    echo "  # Python"
    echo "  import vexfs"
    echo "  vexfs.init('$MOUNT_POINT')"
    echo ""
    echo "  # TypeScript"
    echo "  const client = new VexFSClient({ mountPoint: '$MOUNT_POINT' })"
    echo ""
}

# Main execution
main() {
    echo "Starting VexFS simple test..."
    echo ""
    
    check_dependencies
    build_vexfs
    setup_test_env
    mount_vexfs
    run_tests
    test_sdk_integration
    show_usage_examples
    
    echo ""
    echo "🎉 VexFS test completed successfully!"
    echo ""
    echo "🔧 VexFS is now running at: $MOUNT_POINT"
    echo "   You can interact with it using normal file operations"
    echo "   Press Ctrl+C to stop and unmount"
    echo ""
    
    # Keep running until user stops
    echo "⏳ Keeping VexFS mounted... (Press Ctrl+C to stop)"
    wait $VEXFS_PID 2>/dev/null || true
}

# Run main function
main "$@"