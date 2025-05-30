#!/bin/bash

# Connect to the testing VM via SSH
echo "Connecting to VexFS Testing VM..."
echo "Default connection: ssh -p 2222 user@localhost"
echo "Make sure the VM is running and SSH is enabled"

# Try to connect
ssh -p 2222 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null user@localhost
