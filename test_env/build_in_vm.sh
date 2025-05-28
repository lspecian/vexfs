#!/bin/bash
echo "🔨 Building VexFS in VM..."
./test_env/ssh_vm.sh "cd ~/vexfs_build/vexfs && source ~/.cargo/env && make clean && make vm-build"
