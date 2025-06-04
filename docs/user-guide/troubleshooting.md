# VexFS v2.0 Troubleshooting Guide

This guide helps you diagnose and resolve common issues with VexFS v2.0. Follow the systematic approach to quickly identify and fix problems.

## 🔍 Quick Diagnosis

### System Health Check

Run this comprehensive health check script:

```bash
#!/bin/bash
# VexFS v2.0 Health Check Script

echo "🔍 VexFS v2.0 System Health Check"
echo "=================================="

# Check kernel module
echo "1. Kernel Module Status:"
if lsmod | grep -q vexfs_v2; then
    echo "   ✅ vexfs_v2 module loaded"
    lsmod | grep vexfs_v2
else
    echo "   ❌ vexfs_v2 module NOT loaded"
fi

# Check filesystem registration
echo "2. Filesystem Registration:"
if cat /proc/filesystems | grep -q vexfs; then
    echo "   ✅ VexFS filesystem registered"
else
    echo "   ❌ VexFS filesystem NOT registered"
fi

# Check mounts
echo "3. Active Mounts:"
if mount | grep -q vexfs; then
    echo "   ✅ VexFS filesystems mounted:"
    mount | grep vexfs
else
    echo "   ❌ No VexFS filesystems mounted"
fi

# Check kernel logs
echo "4. Recent Kernel Messages:"
dmesg | grep -i vexfs | tail -5

# Check system resources
echo "5. System Resources:"
echo "   Memory: $(free -h | grep Mem | awk '{print $3 "/" $2}')"
echo "   Disk: $(df -h / | tail -1 | awk '{print $3 "/" $2 " (" $5 " used)"}')"
echo "   Load: $(uptime | awk -F'load average:' '{print $2}')"

echo "=================================="
echo "Health check complete!"
```

### Quick Status Commands

```bash
# Essential status checks
lsmod | grep vexfs                    # Check module
cat /proc/filesystems | grep vexfs   # Check filesystem
mount | grep vexfs                    # Check mounts
dmesg | grep vexfs | tail -10        # Check kernel messages
```

## 🚨 Common Issues and Solutions

### 1. Kernel Module Issues

#### Module Won't Load

**Symptoms:**
```bash
$ sudo insmod vexfs_v2.ko
insmod: ERROR: could not insert module vexfs_v2.ko: Invalid module format
```

**Solutions:**

```bash
# Check kernel compatibility
uname -r
modinfo vexfs_v2.ko | grep vermagic

# Rebuild for current kernel
cd kernel/vexfs_v2_build
make clean
make

# Check for missing dependencies
dmesg | grep "Unknown symbol"

# Install kernel headers if missing
sudo apt install linux-headers-$(uname -r)  # Ubuntu/Debian
sudo yum install kernel-devel-$(uname -r)   # CentOS/RHEL
```

#### Module Loads but Crashes

**Symptoms:**
```bash
$ dmesg | tail
[12345.678] BUG: kernel NULL pointer dereference at 0000000000000000
[12345.679] vexfs_v2: module verification failed
```

**Solutions:**

```bash
# Check module parameters
sudo insmod vexfs_v2.ko debug=1

# Verify build environment
cd kernel/vexfs_v2_build
make clean
make CONFIG_DEBUG_INFO=y

# Check for memory issues
free -h
echo 3 | sudo tee /proc/sys/vm/drop_caches  # Clear caches

# Load with minimal parameters
sudo insmod vexfs_v2.ko default_dimension=384 cache_size_mb=512
```

#### Permission Denied

**Symptoms:**
```bash
$ sudo insmod vexfs_v2.ko
insmod: ERROR: could not insert module vexfs_v2.ko: Operation not permitted
```

**Solutions:**

```bash
# Check secure boot status
mokutil --sb-state

# Disable secure boot temporarily (if safe)
# Or sign the module
sudo /usr/src/linux-headers-$(uname -r)/scripts/sign-file \
    sha256 /path/to/signing_key.priv /path/to/signing_key.x509 vexfs_v2.ko

# Check SELinux/AppArmor
getenforce  # SELinux
sudo aa-status  # AppArmor

# Temporarily disable if needed
sudo setenforce 0  # SELinux
sudo systemctl stop apparmor  # AppArmor
```

### 2. Mount Issues

#### Mount Command Fails

**Symptoms:**
```bash
$ sudo mount -t vexfs_v2 /dev/sdb1 /mnt/vexfs
mount: unknown filesystem type 'vexfs_v2'
```

**Solutions:**

```bash
# Verify filesystem is registered
cat /proc/filesystems | grep vexfs

# If not registered, reload module
sudo rmmod vexfs_v2
sudo insmod vexfs_v2.ko

# Check device formatting
sudo file -s /dev/sdb1
sudo blkid /dev/sdb1

# Reformat if necessary
sudo mkfs.vexfs /dev/sdb1

# Try alternative mount syntax
sudo mount -t vexfs /dev/sdb1 /mnt/vexfs
```

#### Device Busy Error

**Symptoms:**
```bash
$ sudo umount /mnt/vexfs
umount: /mnt/vexfs: device is busy
```

**Solutions:**

```bash
# Find processes using the mount
sudo lsof +D /mnt/vexfs
sudo fuser -v /mnt/vexfs

# Kill processes if safe
sudo fuser -k /mnt/vexfs

# Force unmount (last resort)
sudo umount -f /mnt/vexfs
sudo umount -l /mnt/vexfs  # Lazy unmount
```

#### Permission Issues After Mount

**Symptoms:**
```bash
$ echo "test" > /mnt/vexfs/test.txt
bash: /mnt/vexfs/test.txt: Permission denied
```

**Solutions:**

```bash
# Check mount permissions
ls -la /mnt/vexfs

# Fix ownership
sudo chown $USER:$USER /mnt/vexfs

# Check mount options
mount | grep vexfs

# Remount with proper options
sudo umount /mnt/vexfs
sudo mount -t vexfs_v2 -o defaults,user_xattr /dev/sdb1 /mnt/vexfs
```

### 3. FUSE Implementation Issues

#### FUSE Mount Fails

**Symptoms:**
```bash
$ ./target/release/vexfs_fuse /tmp/vexfs_mount
fuse: failed to access mountpoint /tmp/vexfs_mount: Permission denied
```

**Solutions:**

```bash
# Check FUSE permissions
ls -la /dev/fuse
sudo chmod 666 /dev/fuse

# Add user to fuse group
sudo usermod -a -G fuse $USER
newgrp fuse

# Check mount point
ls -la /tmp/vexfs_mount
chmod 755 /tmp/vexfs_mount

# Try with sudo (temporary)
sudo ./target/release/vexfs_fuse /tmp/vexfs_mount -o allow_other
```

#### FUSE Process Hangs

**Symptoms:**
```bash
$ ls /tmp/vexfs_mount
# Command hangs indefinitely
```

**Solutions:**

```bash
# Kill hanging FUSE process
sudo pkill -f vexfs_fuse

# Unmount forcefully
fusermount -u /tmp/vexfs_mount
sudo umount -f /tmp/vexfs_mount

# Check for zombie processes
ps aux | grep vexfs_fuse

# Restart with debug
./target/release/vexfs_fuse /tmp/vexfs_mount -f -d
```

### 4. Performance Issues

#### Slow Vector Operations

**Symptoms:**
- Vector insertions taking >1 second each
- Search queries timing out
- High CPU usage during operations

**Solutions:**

```bash
# Check system resources
top -p $(pgrep vexfs)
iostat -x 1 5

# Optimize kernel module parameters
sudo rmmod vexfs_v2
sudo insmod vexfs_v2.ko \
    cache_size_mb=4096 \
    max_concurrent_ops=2000 \
    batch_size=10000

# Check for memory pressure
free -h
echo 3 | sudo tee /proc/sys/vm/drop_caches

# Monitor I/O patterns
sudo iotop -p $(pgrep vexfs)
```

#### Memory Usage Issues

**Symptoms:**
```bash
$ dmesg | tail
[12345.678] vexfs_v2: out of memory
[12345.679] Cannot allocate memory for vector cache
```

**Solutions:**

```bash
# Check available memory
free -h
cat /proc/meminfo | grep Available

# Reduce cache sizes
sudo rmmod vexfs_v2
sudo insmod vexfs_v2.ko cache_size_mb=1024

# Enable swap if needed
sudo swapon -s
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Monitor memory usage
watch -n 1 'cat /proc/meminfo | grep -E "(MemTotal|MemAvailable|Cached)"'
```

### 5. Vector Search Issues

#### Search Returns No Results

**Symptoms:**
- Vector search queries return empty results
- Known similar vectors not found

**Diagnostic Steps:**

```python
import vexfs
import numpy as np

# Connect and diagnose
client = vexfs.Client('/mnt/vexfs')
collection = client.get_collection('test_collection')

# Check collection status
info = collection.info()
print(f"Vector count: {info.vector_count}")
print(f"Dimension: {info.dimension}")
print(f"Algorithm: {info.algorithm}")

# Verify vector insertion
test_vector = np.random.random(384).astype(np.float32)
result = collection.insert(vector=test_vector, metadata={"test": True})
print(f"Inserted vector ID: {result.id}")

# Test search with same vector
results = collection.search(vector=test_vector, limit=1)
print(f"Search results: {len(results)}")
if results:
    print(f"Distance to self: {results[0].distance}")
```

**Solutions:**

```python
# Check vector normalization
def normalize_vector(vector):
    norm = np.linalg.norm(vector)
    return vector / norm if norm > 0 else vector

# Verify dimensions match
if query_vector.shape[0] != collection.dimension:
    print(f"Dimension mismatch: {query_vector.shape[0]} vs {collection.dimension}")

# Check distance metric
results = collection.search(
    vector=query_vector,
    limit=10,
    distance_metric="cosine"  # Try different metrics
)

# Increase search parameters for HNSW
collection.configure_search(ef_search=200)
```

#### Inconsistent Search Results

**Symptoms:**
- Same query returns different results
- Search quality varies significantly

**Solutions:**

```python
# Check index integrity
integrity = collection.check_integrity()
if not integrity.passed:
    print("Index integrity issues found")
    collection.reindex()

# Verify vector quality
def check_vector_quality(vectors):
    # Check for NaN/inf values
    if not np.isfinite(vectors).all():
        print("Warning: Vectors contain NaN or infinite values")
    
    # Check for zero vectors
    norms = np.linalg.norm(vectors, axis=1)
    zero_count = np.sum(norms == 0)
    if zero_count > 0:
        print(f"Warning: {zero_count} zero vectors found")

# Rebuild index with better parameters
collection.reindex(
    algorithm="hnsw",
    parameters={
        "m": 32,
        "ef_construction": 400,
        "ef_search": 200
    }
)
```

### 6. API and SDK Issues

#### Python SDK Import Errors

**Symptoms:**
```python
>>> import vexfs
ImportError: No module named 'vexfs'
```

**Solutions:**

```bash
# Check installation
pip list | grep vexfs

# Reinstall if needed
pip uninstall vexfs
pip install vexfs-v2

# Check Python path
python -c "import sys; print('\n'.join(sys.path))"

# Install in development mode
cd bindings/python
pip install -e .
```

#### Connection Errors

**Symptoms:**
```python
>>> client = vexfs.Client('/mnt/vexfs')
ConnectionError: Cannot connect to VexFS at /mnt/vexfs
```

**Solutions:**

```python
# Check mount status
import os
print(f"Mount exists: {os.path.exists('/mnt/vexfs')}")
print(f"Is directory: {os.path.isdir('/mnt/vexfs')}")
print(f"Is writable: {os.access('/mnt/vexfs', os.W_OK)}")

# Try alternative connection methods
client = vexfs.Client('/mnt/vexfs', timeout=30)

# Check for permission issues
import stat
st = os.stat('/mnt/vexfs')
print(f"Permissions: {stat.filemode(st.st_mode)}")
```

### 7. Build Issues

#### Compilation Errors

**Symptoms:**
```bash
$ make
error: linking with `cc` failed: exit status: 1
```

**Solutions:**

```bash
# Check build dependencies
sudo apt install build-essential linux-headers-$(uname -r)

# Clean and rebuild
make clean
make V=1  # Verbose output

# Check for missing symbols
nm vexfs_v2.ko | grep -i undefined

# Verify kernel source
ls /lib/modules/$(uname -r)/build
```

#### Rust Build Failures

**Symptoms:**
```bash
$ cargo build --release
error: failed to compile `vexfs` v2.0.0
```

**Solutions:**

```bash
# Update Rust
rustup update

# Check Rust version
rustc --version

# Clean cargo cache
cargo clean
rm -rf target/

# Install required targets
rustup target add x86_64-unknown-linux-gnu

# Build with verbose output
cargo build --release --verbose
```

## 🔧 Advanced Debugging

### Enable Debug Logging

```bash
# Kernel module debug
sudo rmmod vexfs_v2
sudo insmod vexfs_v2.ko debug=1 log_level=7

# Check debug messages
dmesg -w | grep vexfs

# Python SDK debug
export VEXFS_LOG_LEVEL=DEBUG
python your_script.py
```

### Performance Profiling

```bash
# Profile kernel module
sudo perf record -g -p $(pgrep vexfs)
sudo perf report

# Monitor system calls
sudo strace -p $(pgrep vexfs_fuse) -f

# Memory profiling
valgrind --tool=memcheck ./target/release/vexfs_fuse /tmp/vexfs_mount
```

### Network Debugging (if applicable)

```bash
# Monitor network traffic
sudo tcpdump -i any port 8000

# Check listening ports
netstat -tlnp | grep vexfs
ss -tlnp | grep vexfs
```

## 📋 Diagnostic Information Collection

When reporting issues, collect this information:

```bash
#!/bin/bash
# VexFS Diagnostic Information Collection

echo "VexFS v2.0 Diagnostic Report" > vexfs_diagnostic.txt
echo "Generated: $(date)" >> vexfs_diagnostic.txt
echo "==============================" >> vexfs_diagnostic.txt

# System information
echo "System Information:" >> vexfs_diagnostic.txt
uname -a >> vexfs_diagnostic.txt
cat /etc/os-release >> vexfs_diagnostic.txt
free -h >> vexfs_diagnostic.txt
df -h >> vexfs_diagnostic.txt

# VexFS status
echo -e "\nVexFS Status:" >> vexfs_diagnostic.txt
lsmod | grep vexfs >> vexfs_diagnostic.txt
cat /proc/filesystems | grep vexfs >> vexfs_diagnostic.txt
mount | grep vexfs >> vexfs_diagnostic.txt

# Recent logs
echo -e "\nRecent Kernel Messages:" >> vexfs_diagnostic.txt
dmesg | grep -i vexfs | tail -20 >> vexfs_diagnostic.txt

# Build information
echo -e "\nBuild Information:" >> vexfs_diagnostic.txt
if [ -f kernel/vexfs_v2_build/vexfs_v2.ko ]; then
    modinfo kernel/vexfs_v2_build/vexfs_v2.ko >> vexfs_diagnostic.txt
fi

echo "Diagnostic report saved to vexfs_diagnostic.txt"
```

## 🆘 Getting Help

### Before Asking for Help

1. **Run the health check script** from the beginning of this guide
2. **Check recent kernel messages**: `dmesg | grep vexfs | tail -20`
3. **Verify your setup** matches the installation guide
4. **Search existing issues** on GitHub

### Reporting Issues

When reporting issues, include:

1. **System information**: OS, kernel version, hardware specs
2. **VexFS version**: Module version and build information
3. **Exact error messages**: Copy-paste complete error output
4. **Steps to reproduce**: Minimal example that triggers the issue
5. **Diagnostic report**: Output from the diagnostic script above

### Community Resources

- **GitHub Issues**: [Report bugs and request features](https://github.com/lspecian/vexfs/issues)
- **Discussions**: [Community Q&A](https://github.com/lspecian/vexfs/discussions)
- **Documentation**: [Complete documentation](../README.md)

### Emergency Recovery

If VexFS becomes completely unresponsive:

```bash
# Emergency cleanup
sudo pkill -f vexfs
sudo umount -f /mnt/vexfs
sudo rmmod vexfs_v2

# Clear any locks
sudo rm -f /var/lock/vexfs*

# Restart from clean state
cd kernel/vexfs_v2_build
make clean && make
sudo insmod vexfs_v2.ko
```

---

**Remember**: Most issues are resolved by checking the basics first - module loading, filesystem registration, and mount status. When in doubt, start with the health check script! 🔍

**Need more help?** Check our [community discussions](https://github.com/lspecian/vexfs/discussions) or [open an issue](https://github.com/lspecian/vexfs/issues).