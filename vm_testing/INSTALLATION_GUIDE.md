# VexFS VM Testing Installation Guide

## Quick Start

1. **Setup VM Environment**:
   ```bash
   ./kernel_module/tests/vm_testing_setup.sh
   ```

2. **Start VM for Installation**:
   ```bash
   ./vm_testing/scripts/start_vm.sh
   ```

3. **Install Ubuntu in VM**:
   - Follow Ubuntu installation wizard
   - Create user account
   - Enable SSH server during installation
   - After installation, create file: `touch vm_testing/.vm_installed`

4. **Start VM for Testing**:
   ```bash
   ./vm_testing/scripts/start_vm.sh
   ```

5. **Connect to VM**:
   ```bash
   ssh -p 2222 user@localhost
   ```

6. **Mount Shared Directory in VM**:
   ```bash
   sudo mkdir -p /mnt/shared
   sudo mount -t 9p -o trans=virtio shared /mnt/shared
   ```

7. **Run VexFS Tests in VM**:
   ```bash
   /mnt/shared/test_vexfs_in_vm.sh
   ```

## VM Configuration

- **Memory**: 2GB RAM
- **CPUs**: 2 cores
- **Disk**: 20GB
- **Network**: NAT with SSH forwarding (port 2222)
- **Shared Directory**: Host `vm_testing/shared` → VM `/mnt/shared`

## Safety Features

- **Complete Isolation**: VM crashes don't affect host
- **Shared Directory**: Easy file transfer between host and VM
- **SSH Access**: Remote testing and debugging
- **Snapshot Support**: Can save VM states for testing

## Troubleshooting

### VM Won't Start
- Check KVM acceleration: `kvm-ok`
- Ensure user is in kvm group: `sudo usermod -a -G kvm $USER`

### Shared Directory Not Working
- Mount manually in VM: `sudo mount -t 9p -o trans=virtio shared /mnt/shared`
- Check permissions on host shared directory

### SSH Connection Failed
- Ensure VM is running: `ps aux | grep qemu`
- Check port forwarding: `netstat -tlnp | grep 2222`

## Testing Workflow

1. **Safe Development**: Test all kernel changes in VM first
2. **Crash Recovery**: VM crashes are isolated and recoverable
3. **Debugging**: Use VM for systematic debugging without host risk
4. **Validation**: Only deploy to host after VM validation

This setup provides a safe environment for kernel module development and testing.
