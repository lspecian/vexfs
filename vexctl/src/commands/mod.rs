/*
 * VexFS Control Tool Commands
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

//! Command implementations for vexctl

pub mod status;
pub mod search;
pub mod embedding;
pub mod index;
pub mod fsck;

pub use status::StatusCommand;
pub use search::SearchCommand;
pub use embedding::EmbeddingCommand;
pub use index::IndexCommand;
pub use fsck::FsckCommand;

use crate::output::OutputFormat;
use crate::Result;

/// Common trait for all vexctl commands
pub trait Command {
    /// Execute the command
    fn execute(&self) -> Result<()>;
    
    /// Get command name
    fn name(&self) -> &'static str;
    
    /// Get command description
    fn description(&self) -> &'static str;
}

/// Common configuration for commands
#[derive(Debug, Clone)]
pub struct CommandConfig {
    /// Output format
    pub format: OutputFormat,
    /// Verbose output
    pub verbose: bool,
    /// Quiet mode (minimal output)
    pub quiet: bool,
    /// Mount point path
    pub mount_point: Option<String>,
    /// Timeout for operations
    pub timeout_seconds: u64,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Human,
            verbose: false,
            quiet: false,
            mount_point: None,
            timeout_seconds: 30,
        }
    }
}

impl CommandConfig {
    /// Create a new command configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }
    
    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
    
    /// Enable quiet mode
    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }
    
    /// Set mount point
    pub fn with_mount_point(mut self, mount_point: String) -> Self {
        self.mount_point = Some(mount_point);
        self
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
}