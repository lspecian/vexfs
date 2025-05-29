/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

//! Macros for VexFS Shared Domain
//!
//! This module provides common macros used throughout the VexFS codebase.
//! Macros are designed to work in both kernel and userspace environments.

// Kernel logging constants
#[cfg(feature = "kernel")]
pub const KERN_ERR: &str = "<3>";
#[cfg(feature = "kernel")]
pub const KERN_WARNING: &str = "<4>";
#[cfg(feature = "kernel")]
pub const KERN_INFO: &str = "<6>";
#[cfg(feature = "kernel")]
pub const KERN_DEBUG: &str = "<7>";

// Kernel printk macro placeholder
#[cfg(feature = "kernel")]
#[macro_export]
macro_rules! printk {
    ($level:expr, $fmt:expr) => {
        {
            // In a real kernel module, this would call the actual printk function
            // For now, we'll use a no-op to allow compilation
            ()
        }
    };
    ($level:expr, $fmt:expr, $($arg:tt)*) => {
        {
            // In a real kernel module, this would call the actual printk function
            // For now, we'll use a no-op to allow compilation
            ()
        }
    };
}

/// Alignment and size calculation macros
#[macro_export]
macro_rules! align_up {
    ($value:expr, $align:expr) => {
        (($value) + ($align) - 1) & !((($align) - 1))
    };
}

#[macro_export]
macro_rules! align_down {
    ($value:expr, $align:expr) => {
        ($value) & !(($align) - 1)
    };
}

#[macro_export]
macro_rules! is_aligned {
    ($value:expr, $align:expr) => {
        (($value) & (($align) - 1)) == 0
    };
}

/// Calculate size in blocks
#[macro_export]
macro_rules! size_to_blocks {
    ($size:expr, $block_size:expr) => {
        (($size) + ($block_size) - 1) / ($block_size)
    };
}

/// Convert offset to block and within-block offset
#[macro_export]
macro_rules! offset_to_block_offset {
    ($offset:expr, $block_size:expr) => {
        (($offset) / ($block_size), ($offset) % ($block_size))
    };
}

/// Conditional compilation for kernel vs userspace
#[macro_export]
macro_rules! kernel_or_std {
    (kernel: $kernel_expr:expr, std: $std_expr:expr) => {
        #[cfg(feature = "kernel")]
        {
            $kernel_expr
        }
        #[cfg(not(feature = "kernel"))]
        {
            $std_expr
        }
    };
}

/// Logging macros that work in both kernel and userspace
#[macro_export]
macro_rules! vexfs_error {
    ($fmt:expr) => {
        kernel_or_std!(
            kernel: printk!(KERN_ERR, concat!("vexfs: ", $fmt)),
            std: eprintln!("vexfs error: {}", $fmt)
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        kernel_or_std!(
            kernel: printk!(KERN_ERR, concat!("vexfs: ", $fmt), $($arg)*),
            std: eprintln!("vexfs error: {}", format!($fmt, $($arg)*))
        )
    };
}

#[macro_export]
macro_rules! vexfs_warn {
    ($fmt:expr) => {
        kernel_or_std!(
            kernel: printk!(KERN_WARNING, concat!("vexfs: ", $fmt)),
            std: eprintln!("vexfs warning: {}", $fmt)
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        kernel_or_std!(
            kernel: printk!(KERN_WARNING, concat!("vexfs: ", $fmt), $($arg)*),
            std: eprintln!("vexfs warning: {}", format!($fmt, $($arg)*))
        )
    };
}

#[macro_export]
macro_rules! vexfs_info {
    ($fmt:expr) => {
        kernel_or_std!(
            kernel: printk!(KERN_INFO, concat!("vexfs: ", $fmt)),
            std: println!("vexfs info: {}", $fmt)
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        kernel_or_std!(
            kernel: printk!(KERN_INFO, concat!("vexfs: ", $fmt), $($arg)*),
            std: println!("vexfs info: {}", format!($fmt, $($arg)*))
        )
    };
}

#[macro_export]
macro_rules! vexfs_debug {
    ($fmt:expr) => {
        #[cfg(debug_assertions)]
        kernel_or_std!(
            kernel: printk!(KERN_DEBUG, concat!("vexfs: ", $fmt)),
            std: println!("vexfs debug: {}", $fmt)
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        kernel_or_std!(
            kernel: printk!(KERN_DEBUG, concat!("vexfs: ", $fmt), $($arg)*),
            std: println!("vexfs debug: {}", format!($fmt, $($arg)*))
        )
    };
}

/// Assertion macros for both debug and release builds
#[macro_export]
macro_rules! vexfs_assert {
    ($condition:expr) => {
        if !($condition) {
            vexfs_error!("Assertion failed: {}", stringify!($condition));
            kernel_or_std!(
                kernel: panic!("VexFS assertion failed"),
                std: panic!("VexFS assertion failed: {}", stringify!($condition))
            );
        }
    };
    ($condition:expr, $msg:expr) => {
        if !($condition) {
            vexfs_error!("Assertion failed: {} - {}", stringify!($condition), $msg);
            kernel_or_std!(
                kernel: panic!("VexFS assertion failed: {}", $msg),
                std: panic!("VexFS assertion failed: {} - {}", stringify!($condition), $msg)
            );
        }
    };
}

#[macro_export]
macro_rules! vexfs_debug_assert {
    ($condition:expr) => {
        #[cfg(debug_assertions)]
        vexfs_assert!($condition);
    };
    ($condition:expr, $msg:expr) => {
        #[cfg(debug_assertions)]
        vexfs_assert!($condition, $msg);
    };
}

/// Memory allocation macros
#[macro_export]
macro_rules! vexfs_alloc {
    ($size:expr) => {
        kernel_or_std!(
            kernel: kmalloc($size, GFP_KERNEL),
            std: std::alloc::alloc(std::alloc::Layout::from_size_align($size, 8).unwrap()) as *mut u8
        )
    };
}

#[macro_export]
macro_rules! vexfs_alloc_zeroed {
    ($size:expr) => {
        kernel_or_std!(
            kernel: kzalloc($size, GFP_KERNEL),
            std: std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align($size, 8).unwrap()) as *mut u8
        )
    };
}

#[macro_export]
macro_rules! vexfs_free {
    ($ptr:expr) => {
        kernel_or_std!(
            kernel: kfree($ptr),
            std: std::alloc::dealloc($ptr as *mut u8, std::alloc::Layout::from_size_align(8, 8).unwrap())
        )
    };
}

/// Error handling macros
#[macro_export]
macro_rules! vexfs_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                vexfs_debug!("Error in {}: {:?}", stringify!($expr), err);
                return Err(err);
            }
        }
    };
}

#[macro_export]
macro_rules! vexfs_try_or_return {
    ($expr:expr, $default:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                vexfs_debug!("Error in {}: {:?}", stringify!($expr), err);
                return $default;
            }
        }
    };
}

#[macro_export]
macro_rules! vexfs_ok_or_return {
    ($expr:expr, $error:expr) => {
        match $expr {
            Some(val) => val,
            None => {
                vexfs_debug!("None value in {}", stringify!($expr));
                return Err($error);
            }
        }
    };
}

/// Bit manipulation macros
#[macro_export]
macro_rules! set_bit {
    ($value:expr, $bit:expr) => {
        ($value) |= (1 << ($bit))
    };
}

#[macro_export]
macro_rules! clear_bit {
    ($value:expr, $bit:expr) => {
        ($value) &= !(1 << ($bit))
    };
}

#[macro_export]
macro_rules! test_bit {
    ($value:expr, $bit:expr) => {
        (($value) & (1 << ($bit))) != 0
    };
}

#[macro_export]
macro_rules! toggle_bit {
    ($value:expr, $bit:expr) => {
        ($value) ^= (1 << ($bit))
    };
}

/// Timing and performance macros
#[macro_export]
macro_rules! vexfs_time_operation {
    ($operation:expr) => {{
        let start = crate::shared::utils::current_timestamp();
        let result = $operation;
        let end = crate::shared::utils::current_timestamp();
        vexfs_debug!("Operation {} took {} ns", stringify!($operation), end - start);
        result
    }};
}

#[macro_export]
macro_rules! vexfs_profile {
    ($name:expr, $operation:expr) => {{
        let start = crate::shared::utils::current_timestamp();
        let result = $operation;
        let end = crate::shared::utils::current_timestamp();
        vexfs_debug!("Profile {}: {} ns", $name, end - start);
        result
    }};
}

/// Type conversion macros
#[macro_export]
macro_rules! to_le_bytes {
    ($value:expr) => {
        ($value).to_le_bytes()
    };
}

#[macro_export]
macro_rules! from_le_bytes {
    ($type:ty, $bytes:expr) => {
        <$type>::from_le_bytes($bytes)
    };
}

#[macro_export]
macro_rules! to_be_bytes {
    ($value:expr) => {
        ($value).to_be_bytes()
    };
}

#[macro_export]
macro_rules! from_be_bytes {
    ($type:ty, $bytes:expr) => {
        <$type>::from_be_bytes($bytes)
    };
}

/// Container macros
#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $type:ty, $field:ident) => {{
        let offset = memoffset::offset_of!($type, $field);
        ($ptr as *const u8).sub(offset) as *const $type
    }};
}

#[macro_export]
macro_rules! container_of_mut {
    ($ptr:expr, $type:ty, $field:ident) => {{
        let offset = memoffset::offset_of!($type, $field);
        ($ptr as *mut u8).sub(offset) as *mut $type
    }};
}

/// Checksum validation macro
#[macro_export]
macro_rules! validate_checksum {
    ($data:expr, $checksum:expr) => {{
        let calculated = crate::shared::utils::crc32($data);
        if calculated != $checksum {
            vexfs_error!("Checksum mismatch: calculated 0x{:08x}, expected 0x{:08x}", 
                        calculated, $checksum);
            return Err(crate::shared::errors::VexfsError::ChecksumMismatch);
        }
    }};
}

/// Alias macros for backward compatibility
#[macro_export]
macro_rules! vexfs_log_info {
    ($($arg:tt)*) => { crate::vexfs_info!($($arg)*) };
}

#[macro_export]
macro_rules! vexfs_log_debug {
    ($($arg:tt)*) => { crate::vexfs_debug!($($arg)*) };
}

#[macro_export]
macro_rules! vexfs_log_warn {
    ($($arg:tt)*) => { crate::vexfs_warn!($($arg)*) };
}

#[macro_export]
macro_rules! vexfs_log_error {
    ($($arg:tt)*) => { crate::vexfs_error!($($arg)*) };
}

/// Filesystem operation validation macros
#[macro_export]
macro_rules! validate_block_bounds {
    ($block_num:expr, $max_blocks:expr) => {
        if $block_num >= $max_blocks {
            return Err(crate::shared::errors::VexfsError::InvalidBlock);
        }
    };
}

#[macro_export]
macro_rules! validate_inode_bounds {
    ($inode_num:expr, $max_inodes:expr) => {
        if $inode_num == 0 || $inode_num >= $max_inodes {
            return Err(crate::shared::errors::VexfsError::InvalidInode);
        }
    };
}

#[macro_export]
macro_rules! validate_file_size {
    ($size:expr) => {
        if $size > crate::shared::constants::MAX_FILE_SIZE {
            return Err(crate::shared::errors::VexfsError::FileTooLarge);
        }
    };
}

#[macro_export]
macro_rules! validate_directory_name {
    ($name:expr) => {
        if $name.is_empty() ||
           $name.len() > crate::shared::constants::MAX_FILENAME_LEN ||
           $name == "." ||
           $name == ".." ||
           $name.contains('\0') {
            return Err(crate::shared::errors::VexfsError::InvalidName);
        }
    };
}

#[macro_export]
macro_rules! fs_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                crate::vexfs_error!("Filesystem operation failed: {:?}", e);
                return Err(e);
            }
        }
    };
}

/// Vector validation macros
#[macro_export]
macro_rules! validate_vector_dimensions {
    ($vector:expr, $expected:expr) => {{
        if $vector.len() != $expected {
            return Err(crate::shared::errors::VexfsError::VectorError(
                crate::shared::errors::VectorErrorKind::DimensionMismatch {
                    expected: $expected as u16,
                    found: $vector.len() as u16,
                }
            ));
        }
    }};
}

#[macro_export]
macro_rules! validate_vector_range {
    ($index:expr, $max:expr) => {{
        if $index >= $max {
            return Err(crate::shared::errors::VexfsError::VectorError(
                crate::shared::errors::VectorErrorKind::IndexOutOfRange {
                    index: $index,
                    max: $max,
                }
            ));
        }
    }};
}

/// Unsafe block with safety comment
#[macro_export]
macro_rules! unsafe_block {
    (safety: $safety:expr; $block:block) => {
        // Safety: $safety
        unsafe $block
    };
}

/// Create a static string from format
#[macro_export]
macro_rules! static_format {
    ($fmt:expr, $($arg:expr),*) => {{
        use core::fmt::Write;
        let mut buffer = heapless::String::<256>::new();
        write!(buffer, $fmt, $($arg),*).unwrap();
        buffer
    }};
}

/// Conditional compilation for different targets
#[macro_export]
macro_rules! target_specific {
    (x86_64: $x86_64_expr:expr, aarch64: $aarch64_expr:expr, default: $default_expr:expr) => {
        #[cfg(target_arch = "x86_64")]
        {
            $x86_64_expr
        }
        #[cfg(target_arch = "aarch64")]
        {
            $aarch64_expr
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            $default_expr
        }
    };
}

/// Create a compile-time constant
#[macro_export]
macro_rules! const_assert {
    ($condition:expr) => {
        const _: () = assert!($condition);
    };
    ($condition:expr, $message:expr) => {
        const _: () = assert!($condition, $message);
    };
}

/// Generate getter and setter methods
#[macro_export]
macro_rules! getter_setter {
    ($field:ident: $type:ty) => {
        pub fn $field(&self) -> $type {
            self.$field
        }
        
        paste::paste! {
            pub fn [<set_ $field>](&mut self, value: $type) {
                self.$field = value;
            }
        }
    };
}

/// Define a versioned structure
#[macro_export]
macro_rules! versioned_struct {
    ($name:ident, $version:expr, { $($field:ident: $type:ty),* $(,)? }) => {
        #[repr(C)]
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            pub version: u32,
            $(pub $field: $type,)*
        }
        
        impl $name {
            pub const VERSION: u32 = $version;
            
            pub fn new($($field: $type),*) -> Self {
                Self {
                    version: Self::VERSION,
                    $($field,)*
                }
            }
            
            pub fn is_compatible(&self) -> bool {
                self.version == Self::VERSION
            }
        }
    };
}

/// Create a Result type alias
#[macro_export]
macro_rules! result_type {
    ($name:ident, $error:ty) => {
        pub type $name<T> = Result<T, $error>;
    };
}

/// Define constants with validation
#[macro_export]
macro_rules! validated_const {
    ($name:ident: $type:ty = $value:expr, validate: $condition:expr) => {
        pub const $name: $type = $value;
        const_assert!($condition, concat!("Validation failed for constant ", stringify!($name)));
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_macros() {
        assert_eq!(align_up!(10, 8), 16);
        assert_eq!(align_down!(10, 8), 8);
        assert!(is_aligned!(16, 8));
        assert!(!is_aligned!(10, 8));
    }

    #[test]
    fn test_size_macros() {
        assert_eq!(size_to_blocks!(10, 4), 3);
        assert_eq!(size_to_blocks!(8, 4), 2);
        assert_eq!(offset_to_block_offset!(10, 4), (2, 2));
    }

    #[test]
    fn test_bit_macros() {
        let mut value = 0u32;
        set_bit!(value, 3);
        assert_eq!(value, 8);
        assert!(test_bit!(value, 3));
        clear_bit!(value, 3);
        assert_eq!(value, 0);
        assert!(!test_bit!(value, 3));
    }

    #[test]
    fn test_versioned_struct() {
        versioned_struct!(TestStruct, 1, {
            field1: u32,
            field2: String,
        });

        let test = TestStruct::new(42, "hello".to_string());
        assert_eq!(test.version, 1);
        assert_eq!(test.field1, 42);
        assert_eq!(test.field2, "hello");
        assert!(test.is_compatible());
    }

    #[test]
    fn test_validated_const() {
        validated_const!(TEST_CONST: u32 = 42, validate: TEST_CONST > 0);
        assert_eq!(TEST_CONST, 42);
    }
}
// ===========================
// Kernel Compatibility Macros
// ===========================

/// Kernel-style logging macros for compatibility with existing code
#[macro_export]
macro_rules! pr_info {
    ($fmt:expr $(, $args:expr)*) => {
        vexfs_info!($fmt $(, $args)*)
    };
}

#[macro_export]
macro_rules! pr_err {
    ($fmt:expr $(, $args:expr)*) => {
        vexfs_error!($fmt $(, $args)*)
    };
}

#[macro_export]
macro_rules! pr_warn {
    ($fmt:expr $(, $args:expr)*) => {
        vexfs_warn!($fmt $(, $args)*)
    };
}