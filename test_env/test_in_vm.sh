#!/bin/bash
echo "🧪 Testing VexFS kernel module in VM..."
./test_env/ssh_vm.sh "/mnt/vexfs_source/test_env/test_module.sh"
