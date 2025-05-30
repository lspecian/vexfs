# VexFS Kernel Module VM Testing Scenarios - REAL Functionality

## Overview

This document defines specific testing scenarios for the VexFS kernel module in VM environment, focusing on systematic validation of ALL kernel module functionality for REAL.

## VM Testing Environment

### VM Setup Requirements
- **VM Type**: QEMU-based Linux VM
- **Safety Level**: DANGEROUS operations isolated in VM
- **Monitoring**: Real-time dmesg and system monitoring
- **Recovery**: VM snapshots for quick recovery from crashes

### VM Access
```bash
# Start VM
cd tests/legacy/vm_management/vm/
./run_qemu_fast.sh

# SSH Access
ssh -p 2222 root@localhost

# Monitor VM logs
tail -f logs/test-kernel-vm_*.log
```

## Testing Scenarios

### Scenario 1: Basic Kernel Module Lifecycle
**Objective**: Validate fundamental module operations
**Safety Level**: RISKY (VM only)
**Duration**: 15 minutes

#### Test Steps:
```bash
# In VM:
# 1. Transfer module
scp -P 2222 kernel/build/vexfs.ko root@localhost:/tmp/

# 2. Load module
insmod /tmp/vexfs.ko
echo "Module load result: $?"

# 3. Verify module is loaded
lsmod | grep vexfs
cat /proc/modules | grep vexfs

# 4. Check kernel messages
dmesg | tail -20 | grep -i vexfs

# 5. Unload module
rmmod vexfs
echo "Module unload result: $?"

# 6. Verify module is unloaded
lsmod | grep vexfs || echo "Module successfully unloaded"
```

#### Success Criteria:
- ✅ Module loads without kernel panic
- ✅ Module appears in lsmod and /proc/modules
- ✅ VexFS initialization messages appear in dmesg
- ✅ Module unloads cleanly
- ✅ No error messages in dmesg

#### Failure Handling:
- Kernel panic → Reboot VM, analyze panic trace
- Load failure → Check dmesg for specific error
- Unload failure → Check if module is in use

### Scenario 2: Module Load/Unload Stress Test
**Objective**: Test module stability under repeated operations
**Safety Level**: RISKY (VM only)
**Duration**: 20 minutes

#### Test Steps:
```bash
# In VM:
# Repeated load/unload cycle
for i in {1..20}; do
  echo "=== Cycle $i ==="
  
  # Load module
  insmod /tmp/vexfs.ko
  if [ $? -ne 0 ]; then
    echo "FAILED: Load cycle $i"
    dmesg | tail -10
    break
  fi
  
  # Verify loaded
  lsmod | grep vexfs > /dev/null
  if [ $? -ne 0 ]; then
    echo "FAILED: Module not found after load cycle $i"
    break
  fi
  
  # Brief pause
  sleep 1
  
  # Unload module
  rmmod vexfs
  if [ $? -ne 0 ]; then
    echo "FAILED: Unload cycle $i"
    dmesg | tail -10
    break
  fi
  
  # Verify unloaded
  lsmod | grep vexfs > /dev/null
  if [ $? -eq 0 ]; then
    echo "FAILED: Module still loaded after unload cycle $i"
    break
  fi
  
  echo "SUCCESS: Cycle $i completed"
  sleep 1
done

echo "Stress test completed"
dmesg | tail -30
```

#### Success Criteria:
- ✅ All 20 cycles complete successfully
- ✅ No memory leaks detected
- ✅ No kernel warnings or errors
- ✅ Consistent behavior across cycles

### Scenario 3: Block Device Registration Test
**Objective**: Test VexFS block device handling
**Safety Level**: DANGEROUS (VM only)
**Duration**: 25 minutes

#### Test Steps:
```bash
# In VM:
# 1. Load module
insmod /tmp/vexfs.ko

# 2. Create test block device
dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=100
echo "Created 100MB test image"

# 3. Setup loop device
losetup /dev/loop0 /tmp/vexfs_test.img
echo "Loop device setup: $?"

# 4. Check if VexFS recognizes block device
ls -la /dev/loop0
file /tmp/vexfs_test.img

# 5. Test block device operations (if VexFS supports them)
# This depends on what VexFS actually implements
hexdump -C /dev/loop0 | head -5

# 6. Cleanup
losetup -d /dev/loop0
rm /tmp/vexfs_test.img
rmmod vexfs
```

#### Success Criteria:
- ✅ Loop device creates successfully
- ✅ VexFS module handles block device presence
- ✅ No kernel errors during block operations
- ✅ Clean cleanup of resources

### Scenario 4: Filesystem Registration Test
**Objective**: Test VexFS filesystem type registration
**Safety Level**: DANGEROUS (VM only)
**Duration**: 30 minutes

#### Test Steps:
```bash
# In VM:
# 1. Load module
insmod /tmp/vexfs.ko

# 2. Check filesystem registration
cat /proc/filesystems | grep vexfs
echo "VexFS filesystem registration: $?"

# 3. Create test filesystem image
dd if=/dev/zero of=/tmp/vexfs_fs.img bs=1M count=200
losetup /dev/loop0 /tmp/vexfs_fs.img

# 4. Test mkfs (if available)
# Note: This may not exist yet, test what's actually implemented
which mkfs.vexfs && mkfs.vexfs /dev/loop0 || echo "mkfs.vexfs not available"

# 5. Test mount attempt
mkdir -p /mnt/vexfs_test
mount -t vexfs /dev/loop0 /mnt/vexfs_test
MOUNT_RESULT=$?
echo "Mount result: $MOUNT_RESULT"

if [ $MOUNT_RESULT -eq 0 ]; then
  echo "=== MOUNT SUCCESSFUL ==="
  
  # Test basic operations
  ls -la /mnt/vexfs_test/
  
  # Test file creation
  echo "Hello VexFS" > /mnt/vexfs_test/test.txt
  cat /mnt/vexfs_test/test.txt
  
  # Test directory creation
  mkdir /mnt/vexfs_test/testdir
  ls -la /mnt/vexfs_test/
  
  # Unmount
  cd /
  umount /mnt/vexfs_test
  echo "Unmount result: $?"
else
  echo "=== MOUNT FAILED ==="
  dmesg | tail -10
fi

# 6. Cleanup
losetup -d /dev/loop0
rm /tmp/vexfs_fs.img
rmmod vexfs
```

#### Success Criteria:
- ✅ VexFS appears in /proc/filesystems
- ✅ Mount operation succeeds (if filesystem is implemented)
- ✅ Basic file operations work (if implemented)
- ✅ Unmount succeeds cleanly
- ✅ No kernel panics during filesystem operations

### Scenario 5: Vector Operations Test (VexFS Specific)
**Objective**: Test VexFS-specific vector storage functionality
**Safety Level**: DANGEROUS (VM only)
**Duration**: 35 minutes

#### Test Steps:
```bash
# In VM:
# 1. Setup VexFS filesystem
insmod /tmp/vexfs.ko
dd if=/dev/zero of=/tmp/vexfs_vector.img bs=1M count=500
losetup /dev/loop0 /tmp/vexfs_vector.img

# 2. Format and mount (if tools exist)
if which mkfs.vexfs; then
  mkfs.vexfs /dev/loop0
  mkdir -p /mnt/vexfs_vector
  mount -t vexfs /dev/loop0 /mnt/vexfs_vector
  
  if [ $? -eq 0 ]; then
    echo "=== TESTING VECTOR OPERATIONS ==="
    cd /mnt/vexfs_vector
    
    # Test vector file creation
    # This depends on VexFS implementation
    echo "Testing vector storage..."
    
    # Create test vector data
    for i in {1..10}; do
      echo "vector_$i: [1.0, 2.0, 3.0, 4.0]" > "vector_$i.vec"
    done
    
    # Test vector retrieval
    ls -la *.vec
    cat vector_1.vec
    
    # Test vector search (if implemented)
    # This would test VexFS-specific functionality
    
    cd /
    umount /mnt/vexfs_vector
  fi
fi

# 3. Cleanup
losetup -d /dev/loop0
rm /tmp/vexfs_vector.img
rmmod vexfs
```

#### Success Criteria:
- ✅ Vector files can be created
- ✅ Vector data can be stored and retrieved
- ✅ VexFS-specific operations work (if implemented)
- ✅ No data corruption

### Scenario 6: Error Handling and Recovery Test
**Objective**: Test VexFS error handling and recovery
**Safety Level**: DANGEROUS (VM only)
**Duration**: 30 minutes

#### Test Steps:
```bash
# In VM:
# 1. Load module
insmod /tmp/vexfs.ko

# 2. Test invalid operations
echo "=== TESTING ERROR HANDLING ==="

# Try to mount non-existent device
mount -t vexfs /dev/nonexistent /mnt/test 2>&1
echo "Invalid mount result: $?"

# Try to mount invalid filesystem
dd if=/dev/urandom of=/tmp/invalid.img bs=1M count=10
losetup /dev/loop0 /tmp/invalid.img
mount -t vexfs /dev/loop0 /mnt/test 2>&1
echo "Invalid filesystem mount result: $?"

# Check kernel messages for proper error handling
dmesg | tail -20

# 3. Test resource exhaustion
echo "=== TESTING RESOURCE LIMITS ==="

# Create many loop devices (test resource limits)
for i in {1..10}; do
  dd if=/dev/zero of="/tmp/test_$i.img" bs=1M count=10
  losetup "/dev/loop$i" "/tmp/test_$i.img" 2>/dev/null
done

# Try to mount all of them
for i in {1..10}; do
  mkdir -p "/mnt/test_$i"
  mount -t vexfs "/dev/loop$i" "/mnt/test_$i" 2>&1
done

# Check system state
dmesg | tail -30
free -h

# 4. Cleanup
for i in {1..10}; do
  umount "/mnt/test_$i" 2>/dev/null
  losetup -d "/dev/loop$i" 2>/dev/null
  rm "/tmp/test_$i.img" 2>/dev/null
done

losetup -d /dev/loop0 2>/dev/null
rm /tmp/invalid.img
rmmod vexfs
```

#### Success Criteria:
- ✅ Invalid operations fail gracefully
- ✅ Proper error messages in dmesg
- ✅ No kernel panics on invalid input
- ✅ Resource limits are respected
- ✅ System remains stable after errors

## Monitoring and Safety Protocols

### Real-Time Monitoring Setup
```bash
# Terminal 1: VM Console monitoring
cd tests/legacy/vm_management/vm/
tail -f logs/test-kernel-vm_*.log

# Terminal 2: dmesg monitoring in VM
ssh -p 2222 root@localhost "watch -n 1 'dmesg | tail -10'"

# Terminal 3: Resource monitoring in VM
ssh -p 2222 root@localhost "watch -n 2 'free -h; echo ---; lsmod | grep vexfs'"
```

### Emergency Procedures

#### Kernel Panic Response
1. **STOP** all testing immediately
2. Capture VM console output
3. Reboot VM from snapshot
4. Analyze panic trace
5. Fix issue before continuing

#### VM Hang Response
1. Wait 30 seconds for recovery
2. Force VM reboot if no response
3. Check VM logs for hang cause
4. Restore from snapshot

#### Module Load Failure
1. Check dmesg for specific error
2. Verify module file integrity
3. Check kernel version compatibility
4. Fix compilation issues if needed

## Test Execution Order

### Phase 1: Basic Validation
1. Scenario 1: Basic Lifecycle
2. Scenario 2: Stress Test

### Phase 2: Advanced Features
3. Scenario 3: Block Device Test
4. Scenario 4: Filesystem Test

### Phase 3: VexFS Specific
5. Scenario 5: Vector Operations
6. Scenario 6: Error Handling

## Success Metrics

### Overall Success Criteria
- [ ] All scenarios complete without kernel panic
- [ ] Module loads/unloads consistently
- [ ] Filesystem operations work (if implemented)
- [ ] Vector operations work (if implemented)
- [ ] Error handling is robust
- [ ] No memory leaks detected

### Performance Metrics
- Module load time < 5 seconds
- Module unload time < 2 seconds
- Filesystem mount time < 10 seconds
- No memory growth over 20 load/unload cycles

This comprehensive testing plan ensures we validate ALL VexFS kernel module functionality for REAL in a safe VM environment.