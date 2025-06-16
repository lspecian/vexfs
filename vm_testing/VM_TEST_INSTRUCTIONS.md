# VexFS VM Testing Instructions

## Current Status

The Alpine Linux VM is set up and ready for testing. Since the VM requires manual interaction for the initial setup, here are the complete instructions to test the fixed VexFS kernel module.

## Quick Start (If VM Already Set Up)

If you've already set up the Alpine VM:

```bash
# Run automated tests
./vm_testing/scripts/automated_vm_test_runner.sh
```

This will SSH into the VM and run all tests automatically.

## Manual Setup Instructions (First Time)

### Step 1: Start the Alpine VM

```bash
./vm_testing/scripts/start_alpine_vm.sh
```

You'll see the Alpine Linux boot messages and then a login prompt:
```
Welcome to Alpine Linux 3.19
Kernel 6.6.7-0-virt on an x86_64 (/dev/ttyS0)

localhost login: 
```

### Step 2: Login and Run Setup

Login as `root` (no password required initially):
```
localhost login: root
```

Run the automated setup script:
```sh
/mnt/shared/setup_alpine_auto.sh
```

This will:
- Configure Alpine Linux
- Set root password to `vexfs`
- Install kernel development tools
- Enable SSH access
- Reboot automatically

### Step 3: Connect via SSH (After Reboot)

After the VM reboots, from your host terminal:
```bash
ssh -p 2222 root@localhost
# Password: vexfs
```

### Step 4: Run Tests

Option A - Run all tests automatically:
```bash
/mnt/shared/run_all_tests.sh
```

Option B - Test manually step by step:
```bash
# Load the fixed module
sudo insmod /mnt/shared/vexfs_deadlock_fix.ko

# Check if loaded
lsmod | grep vexfs

# Create test filesystem
dd if=/dev/zero of=/tmp/test.img bs=1M count=10

# Mount (this previously crashed)
sudo mount -t vexfs_fixed -o loop /tmp/test.img /mnt

# Test file creation
echo "Hello VexFS!" | sudo tee /mnt/test.txt
cat /mnt/test.txt

# Test persistence
sudo umount /mnt
sudo mount -t vexfs_fixed -o loop /tmp/test.img /mnt
cat /mnt/test.txt  # Should still show "Hello VexFS!"

# Cleanup
sudo umount /mnt
sudo rmmod vexfs_deadlock_fix
```

## What Was Fixed

1. **Kernel BUG Fixed**: Removed spinlock from `vexfs_alloc_inode_num()` that was causing crashes
2. **Directory Operations**: Implemented custom readdir that works with disk storage
3. **File Persistence**: Enhanced file operations to properly persist data to disk

## Expected Test Results

✅ Module loads without kernel panic  
✅ Filesystem registers as `vexfs_fixed`  
✅ Mount operations succeed  
✅ File creation works (previously crashed here)  
✅ Directory listing works  
✅ **Files persist across unmount/remount** (main goal)  

## VM Management Commands

```bash
# Check VM status
./vm_testing/check_vm_status.sh

# Start VM (if not running)
./vm_testing/scripts/start_alpine_vm.sh

# Stop VM
kill $(cat vm_testing/qemu.pid)

# SSH to VM
ssh -p 2222 root@localhost

# View VM console (if SSH not working)
# The VM starts with serial console attached
```

## Troubleshooting

### Can't SSH to VM?
- Make sure VM completed setup (check for `.alpine_installed` file)
- Check if VM is running: `ps aux | grep qemu`
- Try the serial console (already attached when you start VM)

### Module won't load in VM?
- Check kernel version compatibility: `uname -r` in VM
- Look at dmesg for errors: `dmesg | tail -20`
- Make sure module was copied to `/mnt/shared/`

### Tests fail?
- Check dmesg for kernel errors
- Ensure the test image has enough space
- Try manual testing to isolate the issue

## Test Report Location

After running tests, results are saved to:
- In VM: `/tmp/test_results.log`
- On host: `vm_testing/shared/test_results.log`

## Safety Note

The VM provides complete isolation. Even if the kernel module crashes, your host system remains safe. This is why we use VMs for kernel development!