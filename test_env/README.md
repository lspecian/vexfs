# VexFS Test Environment

This directory contains tools to build a QEMU virtual machine image using Packer and then run the VM to test the VexFS kernel module.

## Prerequisites

*   [Packer](https://www.packer.io/downloads) installed.
*   [QEMU](https://www.qemu.org/download/) installed.
*   KVM enabled on the host system for better QEMU performance.

## Setup and Testing Procedure

1.  **Build the VM Image:**
    *   Navigate to the `test_env` directory:
        ```bash
        cd /path/to/your/repo/test_env
        ```
    *   Run Packer to build the VM image. This will download a Debian netinstall ISO, install Debian with preseed automation, and then provision it with necessary tools (Rust, kernel headers, build tools) and the VexFS source code.
        ```bash
        packer build vexfs.pkr.hcl
        ```
    *   The build process can take some time (10-20 minutes or more depending on your internet connection and system performance) as it involves OS installation.
    *   The resulting QEMU image will be placed in the `packer_output` subdirectory (e.g., `packer_output/vexfs-dev-vm/vexfs-dev-vm.qcow2`).

2.  **Run the QEMU VM:**
    *   Once the Packer build is complete, execute the `run_qemu.sh` script:
        ```bash
        ./run_qemu.sh
        ```
    *   This script will:
        *   Locate the latest `.qcow2` image created by Packer.
        *   Create a small raw disk image (`vexfs_disk.img`, 100MB) if it doesn't exist, which will be used as `/dev/vdb` for VexFS.
        *   Start QEMU with the Packer-built image as the primary disk (`/dev/vda`) and `vexfs_disk.img` as the secondary disk (`/dev/vdb`).
        *   Forward host port `2222` to the VM's SSH port `22`.
        *   Provide serial console output to your terminal.
    *   You can interact with the VM via the serial console or by SSHing into it:
        ```bash
        ssh root@localhost -p 2222
        ```
        The password for the `root` user is `password` (as set in `preseed.cfg`).

3.  **Test VexFS Inside the QEMU VM:**
    *   Once logged into the VM (either via serial console or SSH):
    *   Navigate to the VexFS source directory. The Packer script copies the source to `/usr/src/vexfs` and builds it.
        ```bash
        cd /usr/src/vexfs
        ```
        If the module wasn't built during Packer provisioning (e.g., due to an error or if you updated the source), you can try building it manually:
        ```bash
        make clean && make
        ```

    *   **Load the module:**
        ```bash
        sudo insmod vexfs.ko
        ```
        Alternatively, use the full path if not in the directory: `sudo insmod /usr/src/vexfs/vexfs.ko`.

    *   **Check `dmesg` for module load messages:**
        ```bash
        dmesg | tail
        ```
        You should see messages like:
        ```
        VexFS: vexfs_rust_init() (extern "C") called from C shim.
        VexFS: kernel::Module::init() called. Module is loading.
        VexFS: Filesystem registered successfully with kernel.
        VexFS: vexfs_module_entry: Calling vexfs_rust_init()
        ```
        (Order might vary slightly, the important part is `Module loaded` or `Filesystem registered`).

    *   **Create a mount point:**
        ```bash
        sudo mkdir -p /mnt/vexfs
        ```

    *   **Attempt to mount VexFS:**
        The extra disk image `vexfs_disk.img` is attached as `/dev/vdb` in the VM.
        ```bash
        sudo mount -t vexfs /dev/vdb /mnt/vexfs
        ```

    *   **Check `dmesg` for fill_super messages:**
        ```bash
        dmesg | tail
        ```
        You should see messages related to `vexfs_fill_super`, `vexfs_get_inode` (for root), and potentially `sb.s_root set`. For example:
        ```
        VexFS: VexfsFsType::mount called for device: "/dev/vdb", flags: ...
        VexFS: fill_super (closure via mount_bdev) called for dev: "/dev/vdb"
        VexFS: vexfs_fill_super called
        VexFS: sb.s_op set.
        VexFS: Attempting to get root inode...
        VexFS: vexfs_get_inode (root specialization) called. Mode: 040755 (or similar for directory)
        VexFS: Root inode (ino: 1) initialized successfully.
        VexFS: Root inode obtained successfully. Ino: 1
        VexFS: sb.s_root set successfully.
        VexFS: Superblock filled. Magic: 0xdeadbeef, Root Dentry: Dentry@...
        VexFS: fill_super closure completed successfully.
        ```

    *   **List the directory contents:**
        ```bash
        ls -la /mnt/vexfs
        ```
        Expected output should include "." and "..":
        ```
        total 0
        drwxr-xr-x 2 root root 0 Jan  1 00:00 .
        drwxr-xr-x 2 root root 0 Jan  1 00:00 ..
        ```
        (Timestamps and exact sizes might vary based on inode initialization).

    *   **Check `dmesg` for lookup and readdir messages (optional):**
        If you `ls /mnt/vexfs`, `dmesg` should show calls to `vexfs_lookup` (for "." and "..") and `vexfs_readdir`.

    *   **Test `vexctl status`:**
        Run the `vexctl` status command targeting the VexFS mount point:
        ```bash
        sudo vexctl status /mnt/vexfs
        ```
        Expected output from `vexctl`:
        ```
        Attempting to get status for VexFS mounted at: /mnt/vexfs
        VexFS status for '/mnt/vexfs': 12345
        Status interpretation: OK (Magic number 12345 received from kernel)
        ```
        Check `dmesg` for the corresponding ioctl log from the kernel module:
        ```bash
        dmesg | tail
        ```
        You should see messages like:
        ```
        VexFS: vexfs_unlocked_ioctl called on ino 1, cmd: 0x7601
        VexFS: VEXFS_IOCTL_GET_STATUS received.
        ```
        (The inode number `ino 1` assumes the ioctl is on the root directory).

    *   **Unmount VexFS:**
        ```bash
        sudo umount /mnt/vexfs
        ```

    *   **Unload the module:**
        ```bash
        sudo rmmod vexfs
        ```

    *   **Check `dmesg` for module unload messages:**
        ```bash
        dmesg | tail
        ```
        You should see messages like:
        ```
        VexFS: VexfsFsType::kill_sb called (from lib.rs)
        VexFS: vexfs_kill_sb called
        VexFS: VexfsSuperblock (s_fs_info) freed.
        VexFS: kernel::Module::exit() called. Module is unloading.
        VexFS: Filesystem unregistered successfully from kernel.
        VexFS: vexfs_module_entry: Calling vexfs_rust_exit()
        VexFS: vexfs_rust_exit() (extern "C") called from C shim.
        ```

4.  **Shutdown the VM:**
    From inside the VM:
    ```bash
    sudo halt -p
    ```
    This will cause the QEMU process to exit.

This procedure covers the basic lifecycle of loading, mounting, interacting with (listing), unmounting, and unloading the VexFS module.
Remember that the current VexFS is in-memory only for its structures (superblock, root inode) and doesn't persist any changes to `/dev/vdb`.
The `ls` output is minimal because the root directory is currently empty except for the implicit "." and ".." entries handled by `vexfs_readdir`.
