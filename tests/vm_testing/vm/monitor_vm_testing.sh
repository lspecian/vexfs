#!/bin/bash

# Multi-terminal monitoring script for VM testing
# This script helps set up the 3-terminal monitoring system

echo "VexFS VM Testing Monitor Setup"
echo "=============================="
echo ""
echo "This script will help you set up the 3-terminal monitoring system:"
echo "1. Terminal 1: VM Console monitoring"
echo "2. Terminal 2: dmesg monitoring in VM"
echo "3. Terminal 3: Resource monitoring in VM"
echo ""
echo "Instructions:"
echo "1. Start the VM with: ./start_test_vm.sh"
echo "2. Connect to VM in Terminal 2: ./connect_vm.sh"
echo "3. Connect to VM in Terminal 3: ./connect_vm.sh"
echo "4. In Terminal 2, run: sudo dmesg -w"
echo "5. In Terminal 3, run: watch -n 1 'free -h && echo && ps aux | head -10'"
echo "6. In Terminal 1, monitor VM console output"
echo "7. Execute tests with: ~/run_tests.sh"
echo ""
echo "Log files will be created in: $(dirname "$0")/logs/"
