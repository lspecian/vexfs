#!/bin/bash
ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -p 2222 -i test_env/vm/keys/vexfs_vm_key vexfs@localhost "$@"
