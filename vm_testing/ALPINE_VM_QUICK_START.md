# VexFS Alpine VM Quick Start Guide

## VM Setup Complete! ✅

The Alpine Linux VM is now ready for VexFS kernel module testing. Alpine was chosen because:
- **Small**: Only 50MB ISO vs 1.5GB for Ubuntu
- **Fast**: Boots in seconds
- **Simple**: Easy automated setup
- **Safe**: Complete isolation from host

## Files Ready in VM

The following have been copied to the shared directory:
- `vexfs_deadlock_fix.ko` - The fixed kernel module
- `test_vexfs_alpine.sh` - Basic testing script
- `persistence/` - All persistence test scripts

## How to Use the VM

### Step 1: Start the Alpine VM

```bash
./vm_testing/scripts/start_alpine_vm.sh
```

On first boot, you'll see a login prompt. Login as `root` (no password).

### Step 2: Run Initial Setup (First Time Only)

```sh
# In the VM, run:
/mnt/shared/setup_alpine_auto.sh
```

This will:
- Set up Alpine Linux automatically
- Install kernel development tools
- Create user account (vexfs/vexfs)
- Configure SSH access
- Reboot when done

### Step 3: Connect via SSH (After Setup)

From your host terminal:
```bash
ssh -p 2222 root@localhost
# Password: vexfs
```

Or:
```bash
ssh -p 2222 vexfs@localhost
# Password: vexfs
```

### Step 4: Test VexFS Module

In the VM (via SSH):
```bash
# Mount shared directory if needed
sudo mount -t 9p -o trans=virtio shared /mnt/shared

# Run the test script
/mnt/shared/test_vexfs_alpine.sh
```

## Manual Testing Commands

If you want to test manually:

```bash
# Load module
sudo insmod /mnt/shared/vexfs_deadlock_fix.ko

# Check if loaded
lsmod | grep vexfs

# Check filesystem registration
cat /proc/filesystems | grep vexfs

# Create test image
dd if=/dev/zero of=/tmp/test.img bs=1M count=10

# Mount (this is where it crashed before)
sudo mount -t vexfs_fixed -o loop /tmp/test.img /mnt

# Test file operations
echo "Hello VexFS!" | sudo tee /mnt/test.txt
cat /mnt/test.txt

# Unmount
sudo umount /mnt

# Unload module
sudo rmmod vexfs_deadlock_fix
```

## VM Management

### Start VM (Headless)
```bash
./vm_testing/scripts/start_alpine_vm.sh
```

### Stop VM
```bash
# Find the PID
cat vm_testing/qemu.pid

# Kill the VM
kill $(cat vm_testing/qemu.pid)
```

### Connect to VM Console
If SSH isn't working, the VM starts with serial console.

## What's Fixed

The kernel module has these fixes:
1. **Spinlock removed** from `vexfs_alloc_inode_num()` - no more kernel BUG
2. **Directory operations** - Custom readdir that works with disk storage
3. **Enhanced file operations** - Proper block persistence

## Expected Test Results

✅ Module loads without crash
✅ Filesystem registers properly
✅ Mount operations work
✅ Directory listing works
✅ File creation works (previously crashed here)
✅ Files persist across unmount/remount

## Troubleshooting

### Can't connect via SSH?
- Make sure Alpine setup completed (touched `.alpine_installed`)
- Check if VM is running: `ps aux | grep qemu`
- Try serial console (it's already open when you start VM)

### Module won't load?
- Check kernel version compatibility
- Look at dmesg for errors
- Make sure the module was copied to shared/

### Mount fails?
- The filesystem might need formatting with mkfs.vexfs
- Check dmesg for specific errors

## Safety Note

This VM provides complete isolation. Even if the kernel module crashes in the VM, your host system remains safe. This is perfect for testing kernel code that previously caused system crashes.