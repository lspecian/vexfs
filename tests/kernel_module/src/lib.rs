//! VexFS Kernel Module Testing Library
//!
//! This library provides testing infrastructure for VexFS kernel module
//! validation across multiple levels of testing complexity, including
//! enhanced crash detection, performance monitoring, and stability validation.

pub mod level1_basic_validation;
pub mod level2_vm_mount_operations;
pub mod enhanced_vm_operations;
pub mod crash_detection;
pub mod kselftest_integration;
pub mod mount_test_suite;
pub mod mount_recovery;
pub mod stress_testing_framework;
pub mod kernel_instrumentation;
pub mod resource_monitoring;
pub mod advanced_detection_integration;
pub mod journal_test;
pub mod atomic_test;

pub use level1_basic_validation::*;
pub use level2_vm_mount_operations::*;
pub use crash_detection::*;
pub use kselftest_integration::*;
pub use mount_test_suite::*;
pub use mount_recovery::*;
pub use advanced_detection_integration::*;
pub use journal_test::*;
pub use atomic_test::*;