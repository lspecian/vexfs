# VexFS Production Testing Safety Checklist

## Pre-Production Testing Requirements

### ✅ VM Testing Completion
- [ ] All VM tests pass without errors
- [ ] No kernel panics or system crashes in VM
- [ ] Module loads and unloads cleanly
- [ ] Filesystem operations work correctly
- [ ] Superblock validation functions properly
- [ ] Error handling works for invalid inputs

### ✅ Code Organization
- [ ] C files properly organized in `kernel/` directory
- [ ] Build system updated for new structure
- [ ] Test files organized in `kernel/tests/`
- [ ] No dangerous utilities in project root

### ✅ Safety Features Implemented
- [ ] Superblock validation in kernel module
- [ ] Read-only mount option available
- [ ] Proper error handling for corrupted filesystems
- [ ] Safe mkfs utility with validation
- [ ] Loop device testing capability

## Production Testing Protocol

### Phase 1: Non-Critical Device Testing
**Requirements:**
- [ ] VM tests completed successfully
- [ ] Non-critical test device available (USB drive, spare partition)
- [ ] Full backup of test device (if contains data)
- [ ] System recovery plan documented

**Test Device Criteria:**
- [ ] Device contains no critical data
- [ ] Device is easily replaceable
- [ ] Device is not the system boot drive
- [ ] Device is not mounted by other processes

**Safety Commands:**
```bash
# 1. Verify device is unmounted
sudo umount /dev/sdX1

# 2. Backup device (if needed)
sudo dd if=/dev/sdX1 of=/backup/device_backup.img bs=1M

# 3. Format with VexFS (only after VM validation)
sudo ./mkfs.vexfs /dev/sdX1

# 4. Mount read-only first
sudo mount -t vexfs -o ro /dev/sdX1 /mnt/test

# 5. Test basic operations
ls -la /mnt/test
df -h /mnt/test

# 6. Unmount safely
sudo umount /mnt/test
```

### Phase 2: Read-Write Testing
**Only proceed if Phase 1 passes completely**

```bash
# 1. Mount read-write
sudo mount -t vexfs /dev/sdX1 /mnt/test

# 2. Test file operations
touch /mnt/test/test_file
echo "Hello VexFS" > /mnt/test/test_file
cat /mnt/test/test_file

# 3. Test directory operations
mkdir /mnt/test/test_dir
ls -la /mnt/test/

# 4. Unmount and remount test
sudo umount /mnt/test
sudo mount -t vexfs /dev/sdX1 /mnt/test
cat /mnt/test/test_file  # Should still contain data

# 5. Final unmount
sudo umount /mnt/test
```

### Phase 3: Stress Testing
**Only proceed if Phase 2 passes completely**

```bash
# 1. Multiple mount/unmount cycles
for i in {1..10}; do
    sudo mount -t vexfs /dev/sdX1 /mnt/test
    ls /mnt/test
    sudo umount /mnt/test
done

# 2. File creation stress test
sudo mount -t vexfs /dev/sdX1 /mnt/test
for i in {1..100}; do
    echo "Test file $i" > /mnt/test/file_$i.txt
done
ls /mnt/test | wc -l  # Should show 100+ files
sudo umount /mnt/test
```

## Emergency Procedures

### If System Becomes Unstable
```bash
# 1. Immediately unmount filesystem
sudo umount /mnt/test

# 2. Unload kernel module
sudo rmmod vexfs

# 3. Check for kernel errors
dmesg | tail -20

# 4. Reboot if necessary
sudo reboot
```

### If Mount Fails
```bash
# 1. Check kernel messages
dmesg | tail -10

# 2. Verify superblock
sudo hexdump -C /dev/sdX1 | head -5

# 3. Check module status
lsmod | grep vexfs

# 4. Reload module if needed
sudo rmmod vexfs
sudo insmod vexfs.ko
```

### If Data Corruption Suspected
```bash
# 1. Immediately unmount
sudo umount /mnt/test

# 2. Do not attempt to fix - restore from backup
sudo dd if=/backup/device_backup.img of=/dev/sdX1 bs=1M

# 3. Report issue for investigation
```

## Monitoring During Testing

### System Monitoring
```bash
# Monitor kernel messages in real-time
sudo dmesg -w

# Monitor system resources
htop

# Monitor filesystem operations
sudo iotop
```

### VexFS Specific Monitoring
```bash
# Check module status
lsmod | grep vexfs

# Check mount status
mount | grep vexfs

# Check filesystem statistics
df -h /mnt/test
```

## Success Criteria

### Phase 1 Success Criteria
- [ ] Filesystem mounts without errors
- [ ] Directory listing works
- [ ] No kernel error messages
- [ ] Clean unmount

### Phase 2 Success Criteria
- [ ] File creation works
- [ ] File reading works
- [ ] Directory creation works
- [ ] Data persists across mount/unmount
- [ ] No data corruption

### Phase 3 Success Criteria
- [ ] Multiple mount cycles work
- [ ] Stress testing completes
- [ ] No memory leaks detected
- [ ] System remains stable

## Failure Response

### If Any Test Fails
1. **STOP** all testing immediately
2. **UNMOUNT** filesystem if mounted
3. **UNLOAD** kernel module
4. **DOCUMENT** the failure with logs
5. **RESTORE** device from backup if needed
6. **RETURN** to VM testing to fix issues

### Documentation Required
- [ ] Exact error messages
- [ ] Kernel log output (dmesg)
- [ ] System state before failure
- [ ] Steps to reproduce
- [ ] Recovery actions taken

## Final Approval

### Before Large-Scale Testing (200GB)
- [ ] All phases completed successfully
- [ ] No issues found in any test
- [ ] System stability confirmed
- [ ] Performance acceptable
- [ ] Error handling validated

### Sign-off Required
- [ ] VM testing: PASSED
- [ ] Phase 1 testing: PASSED  
- [ ] Phase 2 testing: PASSED
- [ ] Phase 3 testing: PASSED
- [ ] Safety review: COMPLETED

**Only proceed to large-scale testing after ALL criteria are met.**

---

**Remember: The goal is safe, incremental validation. Never skip steps or rush the process.**