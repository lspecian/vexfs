# Comprehensive VM Testing Plan for VexFS

## Overview

This document provides a detailed testing plan for VexFS development, including comprehensive mounting tests in virtual machines. This plan ensures safe development while validating all filesystem functionality.

## Testing Philosophy

**Safety First**: All kernel module testing MUST be performed in isolated VMs with snapshots
**Comprehensive Coverage**: Test all aspects from basic loading to large-scale operations
**Incremental Validation**: Each phase builds on previous validated functionality
**Real-World Scenarios**: Test actual use cases and failure conditions

## VM Testing Infrastructure

### VM Environment Specifications

**Base VM Configuration**:
```yaml
VM_Specs:
  CPU: 4 cores (minimum)
  RAM: 8GB (minimum)
  Storage: 
    - System: 50GB (Ubuntu/RHEL)
    - Test Data: 100GB (separate disk)
    - VexFS Test: 200GB (separate disk for large tests)
  Network: Isolated (no external access during testing)
  Snapshots: Before each test phase
```

**VM Images Required**:
- Ubuntu 22.04 LTS (kernel 5.15.x)
- Ubuntu 24.04 LTS (kernel 6.8.x)
- RHEL 9 (kernel 5.14.x)
- Debian 12 (kernel 6.1.x)

### VM Management Scripts

**VM Lifecycle Management**:
```bash
# test_env/vm/vm_control.sh

vm_create() {
    # Create new VM from base image
    # Configure networking and storage
    # Install development tools
    # Set up monitoring
}

vm_snapshot() {
    # Create named snapshot
    # Tag with test phase
    # Store metadata
}

vm_restore() {
    # Restore from snapshot
    # Verify system state
    # Prepare for testing
}

vm_monitor() {
    # Monitor VM health
    # Detect kernel panics
    # Collect crash dumps
    # Alert on failures
}
```

## Phase 1: Basic Module Testing

### 1.1 Module Load/Unload Testing

**Test Objective**: Verify kernel module can be safely loaded and unloaded

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase1_module_basic.sh

echo "=== Phase 1.1: Basic Module Testing ==="

# Test 1: Clean module load
echo "Test 1: Loading VexFS module"
sudo insmod vexfs.ko
if [ $? -eq 0 ]; then
    echo "✅ Module loaded successfully"
else
    echo "❌ Module load failed"
    exit 1
fi

# Test 2: Verify module presence
echo "Test 2: Verifying module in kernel"
if lsmod | grep -q vexfs; then
    echo "✅ Module found in lsmod"
else
    echo "❌ Module not found in lsmod"
    exit 1
fi

# Test 3: Check kernel messages
echo "Test 3: Checking kernel messages"
dmesg | tail -20 | grep -i vexfs
if dmesg | tail -20 | grep -q "VexFS.*loaded successfully"; then
    echo "✅ Success message found"
else
    echo "⚠️  Success message not found"
fi

# Test 4: Module info
echo "Test 4: Module information"
modinfo vexfs
if [ $? -eq 0 ]; then
    echo "✅ Module info accessible"
else
    echo "❌ Module info failed"
fi

# Test 5: Clean unload
echo "Test 5: Unloading module"
sudo rmmod vexfs
if [ $? -eq 0 ]; then
    echo "✅ Module unloaded successfully"
else
    echo "❌ Module unload failed"
    exit 1
fi

# Test 6: Verify unload
echo "Test 6: Verifying module unloaded"
if ! lsmod | grep -q vexfs; then
    echo "✅ Module successfully unloaded"
else
    echo "❌ Module still loaded"
    exit 1
fi

echo "✅ Phase 1.1 Complete: Basic module operations successful"
```

**Expected Results**:
- [ ] Module loads without errors
- [ ] Module appears in lsmod
- [ ] Kernel messages show successful initialization
- [ ] Module info is accessible
- [ ] Module unloads cleanly
- [ ] No kernel errors or warnings

### 1.2 Stress Load/Unload Testing

**Test Objective**: Verify module stability under repeated load/unload cycles

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase1_module_stress.sh

echo "=== Phase 1.2: Module Stress Testing ==="

CYCLES=100
FAILED=0

for i in $(seq 1 $CYCLES); do
    echo "Cycle $i/$CYCLES"
    
    # Load module
    sudo insmod vexfs.ko
    if [ $? -ne 0 ]; then
        echo "❌ Load failed at cycle $i"
        ((FAILED++))
        continue
    fi
    
    # Brief pause
    sleep 0.1
    
    # Unload module
    sudo rmmod vexfs
    if [ $? -ne 0 ]; then
        echo "❌ Unload failed at cycle $i"
        ((FAILED++))
        # Force unload and continue
        sudo rmmod -f vexfs 2>/dev/null
    fi
    
    # Check for kernel errors
    if dmesg | tail -5 | grep -i "error\|panic\|oops"; then
        echo "❌ Kernel error detected at cycle $i"
        ((FAILED++))
    fi
done

echo "Stress test complete: $((CYCLES - FAILED))/$CYCLES successful"
if [ $FAILED -eq 0 ]; then
    echo "✅ Phase 1.2 Complete: Module stress test passed"
else
    echo "❌ Phase 1.2 Failed: $FAILED failures detected"
    exit 1
fi
```

## Phase 2: Filesystem Registration Testing

### 2.1 Filesystem Type Registration

**Test Objective**: Verify VexFS registers as a filesystem type

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase2_fs_registration.sh

echo "=== Phase 2.1: Filesystem Registration Testing ==="

# Load module
sudo insmod vexfs.ko

# Test 1: Check /proc/filesystems
echo "Test 1: Checking filesystem registration"
if grep -q vexfs /proc/filesystems; then
    echo "✅ VexFS registered in /proc/filesystems"
else
    echo "❌ VexFS not found in /proc/filesystems"
    cat /proc/filesystems | grep -v nodev
    exit 1
fi

# Test 2: Check filesystem capabilities
echo "Test 2: Checking filesystem capabilities"
FS_LINE=$(grep vexfs /proc/filesystems)
echo "Filesystem entry: $FS_LINE"

# Test 3: Verify mount command recognizes vexfs
echo "Test 3: Mount command recognition"
if mount -t vexfs 2>&1 | grep -q "wrong fs type\|bad option\|bad superblock"; then
    echo "✅ Mount command recognizes vexfs type"
else
    echo "⚠️  Mount command response unclear"
fi

sudo rmmod vexfs
echo "✅ Phase 2.1 Complete: Filesystem registration successful"
## Phase 3: Device Formatting Testing

### 3.1 mkfs.vexfs Testing

**Test Objective**: Verify mkfs.vexfs can format block devices

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase3_mkfs_testing.sh

echo "=== Phase 3.1: mkfs.vexfs Testing ==="

# Create test image
TEST_IMG="/tmp/vexfs_test.img"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=100

# Test 1: Basic formatting
echo "Test 1: Basic VexFS formatting"
if ./mkfs.vexfs "$TEST_IMG"; then
    echo "✅ Basic formatting successful"
else
    echo "❌ Basic formatting failed"
    exit 1
fi

# Test 2: Verify superblock
echo "Test 2: Verifying superblock"
if hexdump -C "$TEST_IMG" | head -1 | grep -q "53 46 45 56"; then
    echo "✅ VexFS magic number found"
else
    echo "❌ VexFS magic number not found"
    hexdump -C "$TEST_IMG" | head -1
    exit 1
fi

# Test 3: Format with label
echo "Test 3: Formatting with label"
if ./mkfs.vexfs -L "TestFS" "$TEST_IMG"; then
    echo "✅ Labeled formatting successful"
else
    echo "❌ Labeled formatting failed"
    exit 1
fi

# Test 4: Format different sizes
for size in 10M 50M 200M 1G; do
    echo "Test 4.$size: Formatting $size filesystem"
    dd if=/dev/zero of="/tmp/test_$size.img" bs=1M count=${size%M} 2>/dev/null
    if ./mkfs.vexfs "/tmp/test_$size.img"; then
        echo "✅ $size formatting successful"
    else
        echo "❌ $size formatting failed"
    fi
    rm -f "/tmp/test_$size.img"
done

rm -f "$TEST_IMG"
echo "✅ Phase 3.1 Complete: mkfs.vexfs testing successful"
```

### 3.2 Real Block Device Testing

**Test Objective**: Test formatting real block devices (loop devices)

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase3_real_device.sh

echo "=== Phase 3.2: Real Block Device Testing ==="

# Create test image
TEST_IMG="/tmp/vexfs_block_test.img"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=500

# Set up loop device
LOOP_DEV=$(sudo losetup -f)
sudo losetup "$LOOP_DEV" "$TEST_IMG"

echo "Using loop device: $LOOP_DEV"

# Test 1: Format loop device
echo "Test 1: Formatting loop device"
if sudo ./mkfs.vexfs "$LOOP_DEV"; then
    echo "✅ Loop device formatting successful"
else
    echo "❌ Loop device formatting failed"
    sudo losetup -d "$LOOP_DEV"
    exit 1
fi

# Test 2: Verify device formatting
echo "Test 2: Verifying device formatting"
if sudo hexdump -C "$LOOP_DEV" | head -1 | grep -q "53 46 45 56"; then
    echo "✅ Device superblock verified"
else
    echo "❌ Device superblock verification failed"
fi

# Test 3: Re-format test
echo "Test 3: Re-formatting test"
if sudo ./mkfs.vexfs -f "$LOOP_DEV"; then
    echo "✅ Re-formatting successful"
else
    echo "❌ Re-formatting failed"
fi

# Cleanup
sudo losetup -d "$LOOP_DEV"
rm -f "$TEST_IMG"

echo "✅ Phase 3.2 Complete: Real device formatting successful"
```

## Phase 4: Mount Testing (CRITICAL)

### 4.1 Basic Mount Testing

**Test Objective**: Verify VexFS can be mounted without system hangs

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase4_basic_mount.sh

echo "=== Phase 4.1: Basic Mount Testing ==="
echo "⚠️  CRITICAL: This test attempts actual mounting"
echo "⚠️  Ensure VM snapshot is taken before running"

# Preparation
TEST_IMG="/tmp/vexfs_mount_test.img"
MOUNT_POINT="/mnt/vexfs_test"
LOOP_DEV=""

cleanup() {
    echo "Cleaning up..."
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    sudo rmmod vexfs 2>/dev/null || true
    if [ -n "$LOOP_DEV" ]; then
        sudo losetup -d "$LOOP_DEV" 2>/dev/null || true
    fi
    rm -f "$TEST_IMG"
}

trap cleanup EXIT

# Create and format test filesystem
dd if=/dev/zero of="$TEST_IMG" bs=1M count=100
LOOP_DEV=$(sudo losetup -f)
sudo losetup "$LOOP_DEV" "$TEST_IMG"
sudo ./mkfs.vexfs "$LOOP_DEV"

# Create mount point
sudo mkdir -p "$MOUNT_POINT"

# Load module
sudo insmod vexfs.ko

# Test 1: Read-only mount
echo "Test 1: Read-only mount attempt"
echo "⚠️  If system hangs here, the kernel module has critical bugs"

# Set timeout for mount command
timeout 30s sudo mount -t vexfs -o ro "$LOOP_DEV" "$MOUNT_POINT"
MOUNT_RESULT=$?

if [ $MOUNT_RESULT -eq 0 ]; then
    echo "✅ Read-only mount successful!"
    
    # Test basic operations
    echo "Test 1.1: Directory listing"
    if ls -la "$MOUNT_POINT"; then
        echo "✅ Directory listing successful"
    else
        echo "❌ Directory listing failed"
    fi
    
    echo "Test 1.2: Filesystem stats"
    if df "$MOUNT_POINT"; then
        echo "✅ Filesystem stats accessible"
    else
        echo "❌ Filesystem stats failed"
    fi
    
    # Unmount
    echo "Test 1.3: Unmounting"
    if sudo umount "$MOUNT_POINT"; then
        echo "✅ Unmount successful"
    else
        echo "❌ Unmount failed"
    fi
    
elif [ $MOUNT_RESULT -eq 124 ]; then
    echo "❌ Mount timed out - likely system hang"
    echo "❌ CRITICAL: Kernel module causes system instability"
    exit 1
else
    echo "❌ Mount failed with error code: $MOUNT_RESULT"
    dmesg | tail -10
    exit 1
fi

echo "✅ Phase 4.1 Complete: Basic mount testing successful"
```

### 4.2 Read-Write Mount Testing

**Test Objective**: Test read-write mounting and basic file operations

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase4_readwrite_mount.sh

echo "=== Phase 4.2: Read-Write Mount Testing ==="

# Setup (similar to basic mount)
TEST_IMG="/tmp/vexfs_rw_test.img"
MOUNT_POINT="/mnt/vexfs_rw"
LOOP_DEV=""

cleanup() {
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    sudo rmmod vexfs 2>/dev/null || true
    if [ -n "$LOOP_DEV" ]; then
        sudo losetup -d "$LOOP_DEV" 2>/dev/null || true
    fi
    rm -f "$TEST_IMG"
}

trap cleanup EXIT

# Create and format
dd if=/dev/zero of="$TEST_IMG" bs=1M count=100
LOOP_DEV=$(sudo losetup -f)
sudo losetup "$LOOP_DEV" "$TEST_IMG"
sudo ./mkfs.vexfs "$LOOP_DEV"
sudo mkdir -p "$MOUNT_POINT"
sudo insmod vexfs.ko

# Test 1: Read-write mount
echo "Test 1: Read-write mount"
if timeout 30s sudo mount -t vexfs "$LOOP_DEV" "$MOUNT_POINT"; then
    echo "✅ Read-write mount successful"
else
    echo "❌ Read-write mount failed"
    exit 1
fi

# Test 2: File creation
echo "Test 2: File creation"
if sudo touch "$MOUNT_POINT/test_file.txt"; then
    echo "✅ File creation successful"
else
    echo "❌ File creation failed"
fi

# Test 3: File writing
echo "Test 3: File writing"
if echo "Hello VexFS" | sudo tee "$MOUNT_POINT/test_file.txt" > /dev/null; then
    echo "✅ File writing successful"
else
    echo "❌ File writing failed"
fi

# Test 4: File reading
echo "Test 4: File reading"
if sudo cat "$MOUNT_POINT/test_file.txt" | grep -q "Hello VexFS"; then
    echo "✅ File reading successful"
else
    echo "❌ File reading failed"
fi

# Test 5: Directory creation
echo "Test 5: Directory creation"
if sudo mkdir "$MOUNT_POINT/test_dir"; then
    echo "✅ Directory creation successful"
else
    echo "❌ Directory creation failed"
fi

# Test 6: File deletion
echo "Test 6: File deletion"
if sudo rm "$MOUNT_POINT/test_file.txt"; then
    echo "✅ File deletion successful"
else
    echo "❌ File deletion failed"
fi

# Test 7: Directory deletion
echo "Test 7: Directory deletion"
if sudo rmdir "$MOUNT_POINT/test_dir"; then
    echo "✅ Directory deletion successful"
else
    echo "❌ Directory deletion failed"
fi

# Test 8: Sync and unmount
echo "Test 8: Sync and unmount"
sudo sync
if sudo umount "$MOUNT_POINT"; then
    echo "✅ Unmount successful"
else
    echo "❌ Unmount failed"
fi

echo "✅ Phase 4.2 Complete: Read-write mount testing successful"
```

## Phase 5: Vector Operations Testing

### 5.1 Vector Mount Testing

**Test Objective**: Test vector operations through mounted filesystem

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase5_vector_mount.sh

echo "=== Phase 5.1: Vector Operations Mount Testing ==="

# Setup
TEST_IMG="/tmp/vexfs_vector_test.img"
MOUNT_POINT="/mnt/vexfs_vector"
LOOP_DEV=""

cleanup() {
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    sudo rmmod vexfs 2>/dev/null || true
    if [ -n "$LOOP_DEV" ]; then
        sudo losetup -d "$LOOP_DEV" 2>/dev/null || true
    fi
    rm -f "$TEST_IMG"
}

trap cleanup EXIT

# Create larger filesystem for vector testing
dd if=/dev/zero of="$TEST_IMG" bs=1M count=1000
LOOP_DEV=$(sudo losetup -f)
sudo losetup "$LOOP_DEV" "$TEST_IMG"
sudo ./mkfs.vexfs "$LOOP_DEV"
sudo mkdir -p "$MOUNT_POINT"
sudo insmod vexfs.ko

# Mount filesystem
if timeout 30s sudo mount -t vexfs "$LOOP_DEV" "$MOUNT_POINT"; then
    echo "✅ Vector test filesystem mounted"
else
    echo "❌ Vector test filesystem mount failed"
    exit 1
fi

# Test 1: Vector file creation
echo "Test 1: Vector file creation"
VECTOR_FILE="$MOUNT_POINT/test_vector.vec"
if sudo touch "$VECTOR_FILE"; then
    echo "✅ Vector file created"
else
    echo "❌ Vector file creation failed"
fi

# Test 2: IOCTL operations (if implemented)
echo "Test 2: IOCTL vector operations"
if [ -f "./vector_test_client" ]; then
    # Test vector addition via IOCTL
    if sudo ./vector_test_client add_vector "$VECTOR_FILE" "[1.0,2.0,3.0,4.0]"; then
        echo "✅ Vector addition via IOCTL successful"
    else
        echo "❌ Vector addition via IOCTL failed"
    fi
    
    # Test vector search via IOCTL
    if sudo ./vector_test_client search "$MOUNT_POINT" "[1.1,2.1,3.1,4.1]" --top-k=5; then
        echo "✅ Vector search via IOCTL successful"
    else
        echo "❌ Vector search via IOCTL failed"
    fi
else
    echo "⚠️  Vector test client not available - skipping IOCTL tests"
fi

# Test 3: Large file operations
echo "Test 3: Large file operations"
if sudo dd if=/dev/zero of="$MOUNT_POINT/large_file" bs=1M count=10; then
    echo "✅ Large file creation successful"
else
    echo "❌ Large file creation failed"
fi

# Test 4: Multiple files
echo "Test 4: Multiple file operations"
for i in {1..100}; do
    sudo touch "$MOUNT_POINT/file_$i.txt"
done
if [ $(ls "$MOUNT_POINT"/file_*.txt | wc -l) -eq 100 ]; then
    echo "✅ Multiple file creation successful"
else
    echo "❌ Multiple file creation failed"
fi

# Cleanup and unmount
sudo sync
if sudo umount "$MOUNT_POINT"; then
    echo "✅ Vector test unmount successful"
else
    echo "❌ Vector test unmount failed"
fi

echo "✅ Phase 5.1 Complete: Vector operations mount testing successful"
```

## Phase 6: Production Scale Testing

### 6.1 Large Dataset Testing

**Test Objective**: Test VexFS with large-scale datasets (200GB+)

**Test Requirements**:
- Dedicated 500GB+ test partition
- Extended test time (4-8 hours)
- Comprehensive monitoring
- Automated failure detection

**Test Procedure**:
```bash
#!/bin/bash
# test_env/scripts/phase6_large_dataset.sh

echo "=== Phase 6.1: Large Dataset Testing ==="
echo "⚠️  This test requires significant disk space and time"

# Configuration
DATASET_SIZE="200G"
TEST_DEVICE="/dev/sdb1"  # Adjust as needed
MOUNT_POINT="/mnt/vexfs_production"

# Safety checks
if [ ! -b "$TEST_DEVICE" ]; then
    echo "❌ Test device $TEST_DEVICE not found"
    echo "Please create a 500GB+ partition for testing"
    exit 1
fi

echo "⚠️  WARNING: This will format $TEST_DEVICE"
echo "⚠️  All data on $TEST_DEVICE will be lost"
read -p "Continue? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "Test cancelled"
    exit 0
fi

cleanup() {
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    sudo rmmod vexfs 2>/dev/null || true
}

trap cleanup EXIT

# Format device
echo "Formatting $TEST_DEVICE with VexFS..."
if sudo ./mkfs.vexfs "$TEST_DEVICE"; then
    echo "✅ Large device formatted successfully"
else
    echo "❌ Large device formatting failed"
    exit 1
fi

# Load module and mount
sudo mkdir -p "$MOUNT_POINT"
sudo insmod vexfs.ko

echo "Mounting large VexFS filesystem..."
if timeout 60s sudo mount -t vexfs "$TEST_DEVICE" "$MOUNT_POINT"; then
    echo "✅ Large filesystem mounted successfully"
else
    echo "❌ Large filesystem mount failed"
    exit 1
fi

# Test 1: Large file creation
echo "Test 1: Creating large files"
for size in 1G 5G 10G 50G; do
    echo "  Creating ${size} file..."
    if timeout 3600s sudo dd if=/dev/zero of="$MOUNT_POINT/large_${size}.dat" bs=1M count=${size%G}000 status=progress; then
        echo "  ✅ ${size} file created successfully"
    else
        echo "  ❌ ${size} file creation failed"
    fi
done

# Test 2: Vector dataset simulation
echo "Test 2: Vector dataset simulation (1M vectors)"
sudo mkdir -p "$MOUNT_POINT/vectors"
python3 << 'EOF'
import os
import struct
import random
import time

base_path = "/mnt/vexfs_production/vectors"
vectors_created = 0
batch_size = 1000

for batch in range(1000):  # 1000 batches of 1000 = 1M vectors
    batch_dir = f"{base_path}/batch_{batch:04d}"
    os.makedirs(batch_dir, exist_ok=True)
    
    for i in range(batch_size):
        vector_file = f"{batch_dir}/vector_{i:04d}.vec"
        # Create 512-dimensional float32 vector
        vector = [random.random() for _ in range(512)]
        
        with open(vector_file, 'wb') as f:
            for val in vector:
                f.write(struct.pack('f', val))
        
        vectors_created += 1
        
        if vectors_created % 10000 == 0:
            print(f"Created {vectors_created} vectors...")

print(f"✅ Created {vectors_created} vector files")
EOF

echo "✅ Phase 6.1 Complete: Large dataset testing successful"
```

## Test Execution Framework

### Automated Test Runner

```bash
#!/bin/bash
# test_env/scripts/run_comprehensive_tests.sh

echo "=== VexFS Comprehensive Testing Framework ==="

# Configuration
VM_NAME="vexfs-test-vm"
SNAPSHOT_BASE="clean-system"
TEST_RESULTS_DIR="test_results/$(date +%Y%m%d_%H%M%S)"

# Create results directory
mkdir -p "$TEST_RESULTS_DIR"

# Test phases
PHASES=(
    "phase1_module_basic.sh"
    "phase1_module_stress.sh"
    "phase2_fs_registration.sh"
    "phase3_mkfs_testing.sh"
    "phase3_real_device.sh"
    "phase4_basic_mount.sh"
    "phase4_readwrite_mount.sh"
    "phase5_vector_mount.sh"
    "phase6_large_dataset.sh"
)

# Execute tests
for phase in "${PHASES[@]}"; do
    echo "=== Executing $phase ==="
    
    # Restore VM to clean state
    vm_restore "$VM_NAME" "$SNAPSHOT_BASE"
    
    # Run test
    if ./test_env/scripts/"$phase" > "$TEST_RESULTS_DIR/$phase.log" 2>&1; then
        echo "✅ $phase PASSED"
    else
        echo "❌ $phase FAILED"
        echo "Check log: $TEST_RESULTS_DIR/$phase.log"
        
        # Decide whether to continue
        read -p "Continue with remaining tests? (y/n): " continue_tests
        if [ "$continue_tests" != "y" ]; then
            break
        fi
    fi
done

echo "=== Testing Complete ==="
echo "Results available in: $TEST_RESULTS_DIR"
```

## Success Criteria

### Phase 1 Success Criteria
- [ ] Module loads and unloads cleanly 100 times
- [ ] No kernel errors or warnings in dmesg
- [ ] Module information accessible via modinfo

### Phase 2 Success Criteria
- [ ] VexFS appears in /proc/filesystems
- [ ] Mount command recognizes vexfs filesystem type

### Phase 3 Success Criteria
- [ ] mkfs.vexfs formats various image sizes
- [ ] Superblock magic number correctly written
- [ ] Loop device formatting works

### Phase 4 Success Criteria (CRITICAL)
- [ ] Read-only mount succeeds without system hang
- [ ] Read-write mount succeeds
- [ ] Basic file operations work (create, read, write, delete)
- [ ] Directory operations work
- [ ] Clean unmount possible

### Phase 5 Success Criteria
- [ ] Vector files can be created
- [ ] IOCTL operations work (if implemented)
- [ ] Large files can be created and accessed
- [ ] Multiple files can be managed

### Phase 6 Success Criteria
- [ ] 200GB+ filesystem can be created and mounted
- [ ] Large files (50GB+) can be created
- [ ] 1M+ vector files can be stored
- [ ] Filesystem remains stable under load

## Risk Mitigation

### Critical Risks
1. **System Hangs**: All testing in VMs with snapshots
2. **Data Loss**: No testing on production systems
3. **Kernel Panics**: Automated crash detection and recovery
4. **Resource Exhaustion**: Monitoring and limits

### Safety Protocols
1. **VM-Only Testing**: Never test kernel module on host systems
2. **Snapshot Management**: Take snapshots before each phase
3. **Timeout Protection**: All mount operations have timeouts
4. **Automated Recovery**: Scripts handle cleanup and restoration

## Conclusion

This comprehensive testing plan provides a systematic approach to validating VexFS functionality from basic module operations through production-scale testing. The emphasis on VM-based testing ensures safety while the incremental approach allows for early detection of issues.

**Next Steps**: Implement the test scripts and VM infrastructure, then execute Phase 1 testing to validate the current kernel module safety improvements.