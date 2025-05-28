/*
 * VexFS Control Tool Library
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
 */

//! VexFS Control Tool Library
//!
//! This library provides the core functionality for the vexctl command-line tool,
//! including VexFS client operations, IOCTL interface, and command implementations.

pub mod client;
pub mod commands;
pub mod error;
pub mod output;

pub use error::{VexctlError, Result};