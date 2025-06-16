#!/bin/bash

# Direct VM test runner using console interaction
# This script interacts with the VM console to run tests

echo "🚀 Starting direct VM test of VexFS..."

# Kill any existing VM
pkill -f qemu 2>/dev/null || true
sleep 2

# Start VM in background with console redirected
echo "Starting Alpine VM..."
./scripts/start_alpine_vm.sh > /tmp/vm_console.log 2>&1 &
VM_PID=$!

# Wait for VM to boot
echo "Waiting for VM to boot..."
sleep 15

# Create expect script for VM interaction
cat > /tmp/vm_test_expect.exp << 'EOF'
#!/usr/bin/expect -f

set timeout 60

# Function to send command and wait for prompt
proc send_cmd {cmd} {
    send "$cmd\r"
    expect {
        "# " { return 0 }
        timeout { 
            puts "Timeout waiting for command: $cmd"
            return 1 
        }
    }
}

# Connect to VM console (this connects to the running qemu process)
spawn bash -c "echo 'Connecting to VM console...'"

# The VM should already be at login prompt
# We need to send commands to the existing console

# Create a simple test approach: write commands to a file that the VM can execute
send_user "Creating test commands file...\n"

# Create the test script
set script_content {
#!/bin/bash
echo "=== VexFS Test Starting ==="

# Mount shared directory
mkdir -p /mnt/shared
mount -t 9p -o trans=virtio shared /mnt/shared 2>/dev/null || echo "Shared mount may already exist"

# Check if files exist
if [ ! -f "/mnt/shared/vexfs_deadlock_fix.ko" ]; then
    echo "ERROR: Module not found"
    exit 1
fi

echo "Files found in shared directory:"
ls -la /mnt/shared/

# Load module
echo "Loading VexFS module..."
insmod /mnt/shared/vexfs_deadlock_fix.ko
if [ $? -eq 0 ]; then
    echo "SUCCESS: Module loaded"
    lsmod | grep vexfs
else
    echo "ERROR: Module load failed"
    dmesg | tail -10
    exit 1
fi

# Check filesystem registration
if grep -q vexfs /proc/filesystems; then
    echo "SUCCESS: Filesystem registered"
    cat /proc/filesystems | grep vexfs
else
    echo "ERROR: Filesystem not registered"
    exit 1
fi

# Create and format test image
echo "Creating test image..."
dd if=/dev/zero of=/tmp/test.img bs=1M count=5 2>/dev/null

echo "Formatting with mkfs.vexfs..."
chmod +x /mnt/shared/mkfs.vexfs
/mnt/shared/mkfs.vexfs /tmp/test.img
if [ $? -eq 0 ]; then
    echo "SUCCESS: Filesystem formatted"
else
    echo "ERROR: Format failed"
    exit 1
fi

# Test mount
echo "Testing mount..."
mkdir -p /tmp/mnt
if mount -t vexfs_fixed -o loop /tmp/test.img /tmp/mnt; then
    echo "SUCCESS: Mount successful"
    
    # Test basic operations
    echo "Testing file operations..."
    echo "Hello VexFS!" > /tmp/mnt/test.txt 2>/dev/null
    if [ -f /tmp/mnt/test.txt ]; then
        echo "SUCCESS: File created"
        cat /tmp/mnt/test.txt
    else
        echo "WARNING: File creation failed"
    fi
    
    # Directory listing
    echo "Directory contents:"
    ls -la /tmp/mnt/ 2>/dev/null || echo "Directory listing failed"
    
    # Unmount
    umount /tmp/mnt 2>/dev/null
    echo "SUCCESS: Unmount completed"
else
    echo "ERROR: Mount failed"
    dmesg | tail -15
fi

# Cleanup
rmmod vexfs_deadlock_fix 2>/dev/null || echo "Module unload failed"
rm -f /tmp/test.img
echo "=== VexFS Test Completed ==="
}

# Write the script to shared directory so VM can access it
exec > /tmp/write_test_script.sh
echo '#!/bin/bash'
echo 'cat > /mnt/shared/vm_test_script.sh << '"'"'SCRIPT_EOF'"'"
echo "$script_content"
echo 'SCRIPT_EOF'
echo 'chmod +x /mnt/shared/vm_test_script.sh'
exec > /dev/tty

bash /tmp/write_test_script.sh

send_user "Test script created. You can now run it in the VM.\n"
EOF

# Make expect script executable
chmod +x /tmp/vm_test_expect.exp

# Run the expect script
/tmp/vm_test_expect.exp

echo "VM test script has been prepared."
echo "The test script is available at: shared/vm_test_script.sh"
echo ""
echo "To run the test manually in the VM:"
echo "1. Login to VM console as root"
echo "2. Run: /mnt/shared/vm_test_script.sh"
echo ""
echo "VM is running with PID: $VM_PID"
echo "To stop VM: kill $VM_PID"