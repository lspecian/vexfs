// Note: Kernel integration handled via C FFI, not direct kernel crate usage
// These types are defined as stubs for Rust library interface

// Define VexFS Inode-Specific Structure
// For now, it's minimal. Could hold directory contents or file metadata later.
#[repr(C)] // If passed to C or part of a larger C-compatible struct in i_fs_info
pub struct VexfsInodeInfo {
    // Example: if it's a directory, what it contains (not used yet)
    // For root, we might not need much here initially.
    _placeholder: u32, // To avoid empty struct issues if any, and show it's VexFS specific
}

impl VexfsInodeInfo {
    fn new() -> Self {
        VexfsInodeInfo { _placeholder: 0 }
    }
}

// Forward declaration for VEXFS_DIR_FILE_OPS if needed by VEXFS_ROOT_INODE_OPS
// For now, we can use generic_dir_file_operations from the kernel crate.

// Root Inode Operations (`inode_operations`)
extern "C" fn vexfs_lookup(
    dir_inode: &Inode,
    dentry: &mut Dentry,
    _flags: u32,
) -> Result<Option<Arc<Dentry>>> {
    let name = dentry.name();
    pr_info!("VexFS: vexfs_lookup called in dir (ino {}) for name: {:?}\n", dir_inode.i_ino, name);

    if name == "." {
        // d_add(dentry, Some(dir_inode.i_noremount.inc()));
        // The kernel crate's Dentry::d_add typically takes an Arc<Inode>.
        // We need to increment the reference count of dir_inode.
        // Dentry::d_add expects an Arc<Inode>
        dentry.d_add(Arc::clone(dir_inode));
        return Ok(None); // VFS handles the dentry already provided
    } else if name == ".." {
        // For root, ".." also points to the root inode.
        // In a non-root directory, it would point to the parent.
        // Assuming dir_inode is the root inode here.
        dentry.d_add(Arc::clone(dir_inode));
        return Ok(None);
    }
    // No other files in the root directory for now.
    // d_add(dentry, None) makes it a negative dentry.
    dentry.d_add_new(None); // Or d_add(dentry, None::<Arc<Inode>>) if available
    Err(uapi::errno::ENOENT.into()) // No such file or directory
}

extern "C" fn vexfs_getattr(
    _flags: u32, // AT_STATX_SYNC_TYPE, etc.
    dentry: &Dentry,
    stat: &mut bindings::kstat,
) -> Result {
    let inode = dentry.inode(); // Get the inode associated with the dentry
    pr_info!("VexFS: vexfs_getattr called for ino: {}\n", inode.i_ino);

    // Set generic attributes from the inode.
    // The `kernel` crate might provide a helper like `generic_fillattr`.
    // If not, we set fields manually.
    stat.ino = inode.i_ino as u64;
    stat.mode = inode.i_mode;
    stat.nlink = inode.i_nlink as u32;
    stat.uid = inode.i_uid.into();
    stat.gid = inode.i_gid.into();
    stat.rdev = inode.i_rdev as u32; // Or bindings:: énorme if not applicable
    stat.size = inode.i_size as i64;
    stat.atime = inode.i_atime.into();
    stat.mtime = inode.i_mtime.into();
    stat.ctime = inode.i_ctime.into();
    
    // For block-based filesystems, these are relevant:
    stat.blksize = inode.sb().s_blocksize as u32;
    stat.blocks = inode.i_blocks as u64;

    Ok(())
}


pub static VEXFS_ROOT_INODE_OPS: InodeOperations = InodeOperations {
    lookup: Some(vexfs_lookup),
    getattr: Some(vexfs_getattr),
    // Most other operations can be None for a simple root directory for now
    create: None,
    link: None,
    unlink: None,
    symlink: None,
    mkdir: None,
    rmdir: None,
    mknod: None,
    rename: None,
    setattr: None, // Often Some(inode_setattr), a generic helper
    // listxattr, get_acl, etc. can be None
    ..InodeOperations::empty() // Initialize all other fields to None
};

// Note: File operations handled via C FFI bridge

// Placeholder for directory file operations (for readdir, etc.)
// We'll use a generic one for now, to be specialized later.
// This is assigned to Inode.i_fop for directories.
// pub static VEXFS_DIR_FILE_OPS: FileOperations = GenericInodeOperations::DIR_FILE_OPS;

// Implementation of readdir (iterate_shared)
extern "C" fn vexfs_readdir(file: &File, ctx: &mut DirentEmitterContext) -> Result {
    let inode = file.f_inode();
    pr_info!("VexFS: vexfs_readdir called for dir ino: {}, pos: {}\n", inode.i_ino, ctx.pos());

    match ctx.pos() {
        0 => {
            // Emit "."
            pr_info!("VexFS: readdir emitting '.'\n");
            // Inode number of the current directory
            let ino = inode.i_ino;
            // The `emit` function might return a bool indicating success/failure or if buffer is full.
            // Let's assume it returns Result<()> or similar, or a bool we need to check.
            // The kernel crate's DirentEmitterContext::emit usually returns bool.
            if !ctx.emit(ino, b".", DirentType::DT_DIR)? {
                pr_warn!("VexFS: readdir: ctx.emit for '.' failed (buffer full?)\n");
                return Ok(()); // Buffer full, stop here for this call
            }
            ctx.set_pos(1); // Advance position
        }
        1 => {
            // Emit ".."
            pr_info!("VexFS: readdir emitting '..'\n");
            // For the root directory, ".." also points to the root inode itself.
            // A more general implementation would get the parent inode.
            // For now, as we only have the root, its parent is itself.
            let parent_ino = inode.i_ino; // Simplified for root
            if !ctx.emit(parent_ino, b"..", DirentType::DT_DIR)? {
                pr_warn!("VexFS: readdir: ctx.emit for '..' failed (buffer full?)\n");
                return Ok(()); // Buffer full
            }
            ctx.set_pos(2); // Advance position
        }
        _ => {
            // End of directory listing for this simple version
            pr_info!("VexFS: readdir: no more entries (pos {})\n", ctx.pos());
            return Ok(());
        }
    }
    Ok(())
}


pub static VEXFS_DIR_FILE_OPS: FileOperations = FileOperations {
    llseek: None, // Handled by C FFI bridge
    read: None, // Or Some(generic_dir_read) if directory reading is desired (uncommon)
    write: None, // Cannot write to a directory directly
    read_iter: None,
    write_iter: None,
    unlocked_ioctl: None,
    compat_ioctl: None,
    mmap: None, // Typically None for directories
    open: None, // Use default_file_open or None if no special handling
    flush: None,
    release: None, // Use default_file_release or None
    fsync: None,
    fasync: None,
    lock: None,
    sendpage: None,
    get_unmapped_area: None,
    check_flags: None,
    flock: None,
    splice_write: None,
    splice_read: None,
    setlease: None,
    fallocate: None,
    show_fdinfo: None,
    // iterate and iterate_shared are the modern readdir
    iterate: None, // Prefer iterate_shared if available and types match
    iterate_shared: Some(vexfs_readdir),
    unlocked_ioctl: Some(vexfs_unlocked_ioctl), // Added ioctl handler
    // Add other fields as `None` or appropriate defaults if the struct has more.
    // Use `..FileOperations::DEFAULT_DIR_FILE_OPS` or similar if available and suitable.
    // However, explicit definition is safer if the kernel crate API for FileOperations changes.
    // For now, ensure all fields from the `kernel` crate's `FileOperations` struct are listed.
    // If it's a C-style struct of function pointers from `bindings`, it's even more critical.
    // Assuming `FileOperations` is a struct from the `kernel` crate that can be initialized this way.
    // If it has a `const fn default() -> Self` or similar, that could be a base.
    // For example: `..FileOperations::default_dir_ops()` if it existed.
    // If not, all must be specified or it implies `..Default::default()` which might not be what we want.
    // Let's assume this covers the main ones and others are implicitly None or zeroed
    // if the struct definition allows partial initialization or has a suitable `Default`.
    // If `kernel::file::FileOperations` is from `bindgen`, all fields must be set.
    // The `kernel` crate usually provides a "const new" or "builder" or a base static instance.
    // For now, this explicit list with `iterate_shared` is the goal.
    // If `GenericInodeOperations::DIR_FILE_OPS` was used, it might provide defaults for some of these.
    // We are replacing it, so we need to be somewhat complete.
};

// IOCTL handler function
extern "C" fn vexfs_unlocked_ioctl(
    file: &File,
    cmd: u32, // Command number from userspace
    _arg: usize, // Argument from userspace (pointer or value)
) -> Result<i32> { // Return type is often c_long or c_int (i32 here)
    let inode = file.f_inode();
    pr_info!("VexFS: vexfs_unlocked_ioctl called on ino {}, cmd: {:#x}\n", inode.i_ino, cmd);

    // Extract command number and magic from cmd
    // The cmd u32 is composed by userspace using _IOC macros or similar.
    // It typically encodes direction, size, magic type, and sequence number.
    // For simplicity, we assume userspace sends raw command numbers that match our constants for now.
    // A robust implementation would use `_IOC_TYPE(cmd)`, `_IOC_NR(cmd)`, etc.
    // Or match directly on the fully formed ioctl numbers.

    // We need to reconstruct the full IOCTL number as defined by _IO, _IOR, _IOW macros
    // if we want to match against them directly.
    // Or, more simply, if userspace sends just the sequence number for now.
    // The `cmd` received by the kernel function is the full ioctl number.

    // Assume crate::ioctl defines the full numbers or we define them here based on magic & nr.
    // For now, let's assume cmd is just the sequence number for simplicity of matching,
    // though this is not standard. A proper system would check type, nr, dir, size.

    // Let's assume userspace has already created the full IOCTL number.
    // The `cmd` argument here *is* that full number.
    // We need to compare `cmd` with the IOCTL numbers generated using a scheme
    // that matches how `nix::request_code_none!` or similar generates them.
    // This often involves bitmasking and comparing parts, or comparing the full value.
    
    // For now, let's manually construct the expected full IOCTL number for comparison.
    // This is a simplified way and might not be fully robust across architectures
    // without using the kernel's standard _IO, _IOR, _IOW macros translated to Rust.
    // The `nix` crate's `request_code_none!` does something like:
    // `(((IOC_NONE as u32) << DIR_SHIFT) | ((ty as u32) << TYPE_SHIFT) | (nr as u32) << NR_SHIFT | ((size as u32) << SIZE_SHIFT))`
    // We need to ensure `cmd` matches this.
    // For a simple comparison, if we only have one IOCTL:
    
    // Define expected IOCTL number (must match userspace generation)
    // This is platform and definition specific. Using nix's scheme for _IO (no data transfer)
    // #define _IO(type,nr)		_IOC(_IOC_NONE,(type),(nr),0)
    // #define _IOC(dir,type,nr,size) \
	// (((dir)  << _IOC_DIRSHIFT) | \
	//  ((type) << _IOC_TYPESHIFT) | \
	//  ((nr)   << _IOC_NRSHIFT) | \
	//  ((size) << _IOC_SIZESHIFT))
    // _IOC_NONE = 0
    // _IOC_TYPESHIFT = 8
    // _IOC_NRSHIFT = 0
    // _IOC_SIZESHIFT = 16 (approx, depends on arch)
    // Let's assume userspace `vexctl` calculated the `request_code` correctly.
    // And that `cmd` here is that value.

    const IOC_NONE: u32 = 0; // Direction: no data transfer
    const IOC_READ: u32 = 1; // Direction: read from kernel
    const IOC_WRITE: u32 = 2; // Direction: write to kernel
                             // Note: These are simplified; actual values are _IOC_READ, _IOC_WRITE from C.

    // Construct the expected command code as it would be formed by _IO macro
    // This is a conceptual reconstruction. The actual value of cmd will be pre-calculated.
    // For `ioctl_none!(VEXFS_IOCTL_MAGIC, VEXFS_IOCTL_CMD_GET_STATUS)`:
    // Direction is NONE (0), type is 'v', nr is 1. Size is 0.
    // Example: ( (0 << 30) | ('v' << 8) | (1 << 0) | (0 << 16) ) assuming Linux _IOC layout
    // This needs to be precise.
    
    // A simpler way for the kernel module is to just check the number part and type part
    // if the higher bits (direction and size) are known or assumed.
    // Let's use the constants from the ioctl module.
    let expected_cmd_type = crate::ioctl::VEXFS_IOCTL_MAGIC as u32;
    let expected_cmd_nr = crate::ioctl::VEXFS_IOCTL_CMD_GET_STATUS as u32;

    // Decoding the received cmd (conceptual, actual macros are complex)
    // Assuming Linux _IOC_NRBITS = 8, _IOC_TYPEBITS = 8
    // #define _IOC_NR(nr)		((nr) & _IOC_NRMASK)
    // #define _IOC_TYPE(nr)		(((nr) >> _IOC_TYPESHIFT) & _IOC_TYPEMASK)
    let cmd_nr = cmd & 0xFF; // Extract lower 8 bits for NR
    let cmd_type = (cmd >> 8) & 0xFF; // Extract next 8 bits for TYPE

    // TODO: Add proper IOCTL number decoding using kernel macros or robust equivalents.
    // For now, simple direct comparison for a single known IOCTL.
    // This comparison is NOT robust. A full `ioctl_num!` or equivalent on both sides is needed.
    // If userspace uses `nix::request_code_none!(b'v', 1)`, then `cmd` will be that value.
    // We should compare `cmd` directly with `nix::request_code_none!(crate::ioctl::VEXFS_IOCTL_MAGIC, crate::ioctl::VEXFS_IOCTL_CMD_GET_STATUS)`
    // if we could compute it here in the same way.
    // For now, we assume the cmd value received is exactly what we expect for GET_STATUS.
    // A specific value for `expected_get_status_cmd` would be better.
    // Let's assume `request_code_none!(b'v', 1)` results in `0x007601` (simplified example).
    // This is (IOC_NONE << ...) | (b'v' << 8) | 1
    // IOC_DIRSHIFT = 30, IOC_TYPESHIFT = 8, IOC_NRSHIFT = 0, IOC_SIZESHIFT = 16
    // For _IO (IOC_NONE): cmd = ( (b'v' as u32) << 8 ) | (1 as u32); // If size and dir are 0
    // cmd = ( ( (IOC_NONE as u32) << 30 ) | ((b'v' as u32) << 8) | (1 as u32) | ( (0 as u32) << 16) )
    // This is complex. For now, we'll use a placeholder value that vexctl will also use.
    // The command built by `nix::request_code_none!(b'v', 0x01)` is `0x00007601` on many systems.
    // (no direction, no size, type='v', nr=1)
    const VEXFS_GET_STATUS_CMD_FULL: u32 = 0x00007601; // Placeholder for actual compiled IOCTL number

    if cmd == VEXFS_GET_STATUS_CMD_FULL {
        pr_info!("VexFS: VEXFS_IOCTL_GET_STATUS received.\n");
        // Return a dummy status value. This value is the direct return of the ioctl syscall.
        Ok(12345) // Example status code
    } else if cmd_type == expected_cmd_type && cmd_nr == expected_cmd_nr {
        // This fallback comparison is also not fully robust but better than nothing.
        pr_info!("VexFS: VEXFS_IOCTL_GET_STATUS (via parts match) received.\n");
        Ok(12345)
    }
    else {
        pr_warn!("VexFS: Unknown IOCTL command received: {:#x}\n", cmd);
        Err(-25) // ENOTTY equivalent, handled by C FFI bridge
    }
}

// Function to get a new inode.
// Refined for root inode specialization.
// The `mode` passed here is primarily S_IFDIR for the root.
// This is a simplified version. A real filesystem would read from disk or initialize based on fs data.
pub fn vexfs_get_inode(
    sb: &mut SuperBlock, // Superblock reference
    mode: bindings::umode_t, // File mode (type + permissions)
    // In a real FS, might take an inode number to fetch existing, or data to create new
) -> Result<Arc<Inode>> {
    pr_info!("VexFS: vexfs_get_inode (root specialization) called. Mode: {:#o}\n", mode);

    // For the root inode, a fixed inode number is common.
    const ROOT_INO: u64 = 1; // Fixed root inode number

    // Inode::new_anon is suitable for inodes not backed by a block device in a complex way,
    // or for initial setup.
    // `mode` should be S_IFDIR | permissions for the root directory.
    let initial_mode = if (mode & bindings::S_IFMT) == 0 {
        // If only permissions were passed, ensure S_IFDIR is set.
        bindings::S_IFDIR | mode
    } else {
        mode // Assume mode includes S_IFDIR already
    };
    
    let mut new_inode = Inode::new_anon(
        sb,                             // Superblock
        initial_mode,                   // S_IFDIR | 0o755 (example)
        bindings::ROOT_DEV,             // dev_t for root; use actual device if applicable
        &VEXFS_ROOT_INODE_OPS,          // Inode operations specific to root
        Some(&VEXFS_DIR_FILE_OPS),      // File operations (generic for dirs now)
        None,                           // Address space operations (None for simple dir)
    )?;

    // Initialize VexfsInodeInfo and store in i_fs_info
    // Box it as i_fs_info expects a raw pointer.
    let vex_inode_info = Box::try_new(VexfsInodeInfo::new())?;
    new_inode.i_fs_info = Box::into_raw(vex_inode_info) as *mut core::ffi::c_void;
    // This memory needs to be freed when the inode is destroyed (e.g., in destroy_inode or evict_inode).

    // Set specific fields for the root inode
    new_inode.i_ino = ROOT_INO;
    
    // Timestamps (set to current time)
    let now = CurrentTime::new();
    new_inode.i_atime = now;
    new_inode.i_mtime = now;
    new_inode.i_ctime = now;

    // Link count for a directory:
    // - "." entry in itself
    // - ".." entry (for root, ".." points to itself)
    // - Any subdirectories will increment the parent's link count.
    // For a newly created root directory, nlink is often 2.
    new_inode.set_nlink(2);

    // Set UID and GID (e.g., current user, or root)
    // These might need to be configured or inherited.
    // For simplicity, using 0 (root) for now.
    // Note: UID/GID assignment handled by C FFI bridge
    // new_inode.i_uid = kuid_t { val: 0 }; // Handled in C layer
    // new_inode.i_gid = kgid_t { val: 0 }; // Handled in C layer

    // Size of a directory can be block size, or number of entries, or other conventions.
    // Often, it's the size of the data needed to store its entries.
    // For an empty directory (except . and ..), a small size or blocksize is common.
    new_inode.i_size = sb.s_blocksize as i64; // Example size

    pr_info!("VexFS: Root inode (ino: {}) initialized successfully.\n", new_inode.i_ino);
    
    Ok(new_inode)
}
