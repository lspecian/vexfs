# VexFS Error Handling Strategy

## Overview

This document defines the consistent error handling patterns used throughout VexFS, ensuring proper error propagation between C kernel code and Rust library code.

## Error Handling Architecture

### 1. Rust Error Types

VexFS uses a structured error hierarchy defined in `rust/src/shared/errors.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum VexfsError {
    InvalidArgument(String),
    OutOfMemory,
    NoSpaceLeft,
    PermissionDenied(String),
    FileExists,
    FileNotFound,
    NotADirectory(String),
    IsDirectory,
    IoError(String),
    // ... other variants
}

pub type VexfsResult<T> = Result<T, VexfsError>;
```

### 2. C Error Codes

The C kernel module uses standard Linux error codes defined in `kernel/include/vexfs_ffi.h`:

```c
#define VEXFS_SUCCESS 0
#define VEXFS_ERROR_GENERIC -1
#define VEXFS_ERROR_NOMEM -12    // -ENOMEM
#define VEXFS_ERROR_INVAL -22    // -EINVAL
#define VEXFS_ERROR_NOSPC -28    // -ENOSPC
// ... other error codes
```

### 3. FFI Error Conversion

The FFI layer provides consistent conversion between Rust errors and C error codes:

```rust
pub fn to_ffi_result<T>(result: VexfsResult<T>) -> c_int {
    match result {
        Ok(_) => VEXFS_SUCCESS,
        Err(err) => match err {
            VexfsError::InvalidArgument(_) => VEXFS_ERROR_INVAL,
            VexfsError::OutOfMemory => VEXFS_ERROR_NOMEM,
            VexfsError::NoSpaceLeft => VEXFS_ERROR_NOSPC,
            // ... other mappings
            _ => VEXFS_ERROR_GENERIC,
        }
    }
}
```

## Error Handling Patterns

### 1. Kernel Module Error Handling

**C Kernel Module (`kernel/src/vexfs_module_entry.c`):**

```c
static int vexfs_fill_super(struct super_block *sb, void *data, int silent)
{
    int ret;
    
    /* Call Rust FFI function */
    ret = vexfs_rust_fill_super(sb);
    if (ret != VEXFS_SUCCESS) {
        printk(KERN_ERR "VexFS: Rust superblock initialization failed: %d\n", ret);
        return ret;  /* Return Linux error code directly */
    }
    
    /* Continue with C operations */
    return 0;
}
```

**Key Principles:**
- Always check FFI function return values
- Log errors with appropriate kernel log levels
- Return standard Linux error codes
- Handle null pointer checks before FFI calls

### 2. Rust FFI Error Handling

**Rust FFI Functions (`rust/src/ffi/kernel.rs`):**

```rust
#[no_mangle]
pub extern "C" fn vexfs_rust_fill_super(sb_ptr: *mut c_void) -> c_int {
    if sb_ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // Perform Rust operations
    match initialize_superblock(sb_ptr) {
        Ok(_) => VEXFS_SUCCESS,
        Err(err) => to_ffi_result(Err(err)),
    }
}
```

**Key Principles:**
- Validate all pointer parameters for null
- Use `to_ffi_result()` for consistent error conversion
- Never panic in FFI functions
- Handle all error cases explicitly

### 3. Rust Internal Error Handling

**Rust Library Code:**

```rust
pub fn create_file(path: &str, mode: u32, context: &mut OperationContext) -> VexfsResult<FileHandle> {
    // Validate arguments
    if path.is_empty() {
        return Err(VexfsError::InvalidArgument("Empty path".to_string()));
    }
    
    // Perform operations with proper error propagation
    let inode = context.inode_manager.allocate_inode(mode)?;
    let file = File::new(inode, mode)?;
    
    Ok(FileHandle::new(file))
}
```

**Key Principles:**
- Use `VexfsResult<T>` for all fallible operations
- Propagate errors with `?` operator
- Provide descriptive error messages
- Validate inputs early

## Error Categories and Handling

### 1. Memory Errors

**Rust Side:**
```rust
VexfsError::OutOfMemory
```

**C Side:**
```c
VEXFS_ERROR_NOMEM (-12)
```

**Handling:**
- Free any partially allocated resources
- Log memory allocation failures
- Return appropriate error codes

### 2. I/O Errors

**Rust Side:**
```rust
VexfsError::IoError(description)
```

**C Side:**
```c
VEXFS_ERROR_IO (-5)
```

**Handling:**
- Retry operations where appropriate
- Log I/O error details
- Ensure filesystem consistency

### 3. Permission Errors

**Rust Side:**
```rust
VexfsError::PermissionDenied(operation)
```

**C Side:**
```c
VEXFS_ERROR_PERMISSION (-1)
```

**Handling:**
- Check user context and capabilities
- Log security violations
- Return permission denied errors

### 4. Filesystem Errors

**Rust Side:**
```rust
VexfsError::FileNotFound
VexfsError::FileExists
VexfsError::NotADirectory(path)
VexfsError::IsDirectory
```

**C Side:**
```c
VEXFS_ERROR_NOENT (-2)
VEXFS_ERROR_EXIST (-17)
VEXFS_ERROR_NOTDIR (-20)
VEXFS_ERROR_ISDIR (-21)
```

**Handling:**
- Validate filesystem state
- Ensure path resolution correctness
- Handle race conditions appropriately

## Safety Protocols

### 1. Null Pointer Handling

**All FFI functions must validate pointers:**

```rust
#[no_mangle]
pub extern "C" fn vexfs_rust_function(ptr: *mut c_void) -> c_int {
    if ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }
    // ... rest of function
}
```

### 2. Panic Prevention

**Never panic in FFI functions:**

```rust
// ❌ DON'T: This can crash the kernel
pub extern "C" fn bad_function() -> c_int {
    panic!("This will crash the kernel!");
}

// ✅ DO: Handle errors gracefully
pub extern "C" fn good_function() -> c_int {
    match risky_operation() {
        Ok(_) => VEXFS_SUCCESS,
        Err(_) => VEXFS_ERROR_GENERIC,
    }
}
```

### 3. Resource Cleanup

**Ensure proper cleanup on errors:**

```rust
pub extern "C" fn vexfs_rust_allocate_resource() -> *mut c_void {
    let resource = match allocate_resource() {
        Ok(r) => r,
        Err(_) => return core::ptr::null_mut(),
    };
    
    // Convert to raw pointer for C
    Box::into_raw(Box::new(resource)) as *mut c_void
}

pub extern "C" fn vexfs_rust_free_resource(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut Resource);
        }
    }
}
```

## Testing Error Handling

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let result: VexfsResult<()> = Err(VexfsError::InvalidArgument("test".to_string()));
        assert_eq!(to_ffi_result(result), VEXFS_ERROR_INVAL);
    }

    #[test]
    fn test_null_pointer_handling() {
        let result = vexfs_rust_fill_super(core::ptr::null_mut());
        assert_eq!(result, VEXFS_ERROR_INVAL);
    }
}
```

### 2. Integration Tests

```c
// Test FFI error handling in C
static int test_ffi_error_handling(void)
{
    int ret;
    
    // Test null pointer handling
    ret = vexfs_rust_fill_super(NULL);
    if (ret != VEXFS_ERROR_INVAL) {
        printk(KERN_ERR "FFI null pointer test failed\n");
        return -1;
    }
    
    return 0;
}
```

## Error Logging

### 1. Kernel Logging

```c
// Use appropriate log levels
printk(KERN_ERR "VexFS: Critical error: %d\n", error_code);
printk(KERN_WARNING "VexFS: Warning: %s\n", warning_msg);
printk(KERN_INFO "VexFS: Info: %s\n", info_msg);
printk(KERN_DEBUG "VexFS: Debug: %s\n", debug_msg);
```

### 2. Rust Logging (Userspace)

```rust
#[cfg(not(feature = "kernel"))]
use log::{error, warn, info, debug};

#[cfg(not(feature = "kernel"))]
fn handle_error(err: VexfsError) {
    match err {
        VexfsError::IoError(msg) => error!("I/O error: {}", msg),
        VexfsError::PermissionDenied(op) => warn!("Permission denied: {}", op),
        _ => info!("Error: {:?}", err),
    }
}
```

## Best Practices

1. **Consistent Error Codes**: Always use the defined error constants
2. **Descriptive Messages**: Include context in error messages
3. **Early Validation**: Check parameters before processing
4. **Resource Cleanup**: Ensure proper cleanup on all error paths
5. **Error Propagation**: Use `?` operator for clean error propagation
6. **Testing**: Test both success and failure cases
7. **Documentation**: Document error conditions in function comments
8. **Logging**: Log errors at appropriate levels with sufficient detail

This error handling strategy ensures robust, predictable behavior across the VexFS codebase and provides a solid foundation for the FFI bridge implementation.