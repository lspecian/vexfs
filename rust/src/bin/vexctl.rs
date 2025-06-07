/*
 * VexFS Control Tool (vexctl)
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

//! VexFS Control Tool - Command-line interface for VexFS management

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process;

use vexfs::commands::{Command, CommandConfig, StatusCommand};
use vexfs::output::OutputFormat;
use vexfs::{Result, VexctlError};

/// VexFS Control Tool - Manage and interact with VexFS filesystems
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Quiet mode (minimal output)
    #[arg(short, long)]
    quiet: bool,

    /// Timeout for operations in seconds
    #[arg(long, default_value_t = 30)]
    timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display filesystem status and health information
    Status {
        /// Path to the mounted VexFS filesystem
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: PathBuf,
    },
    /// Vector similarity search operations
    Search {
        /// Query vector file or dimensions
        #[arg(short, long)]
        query: Option<String>,
        
        /// Vector file to search with
        #[arg(short, long)]
        file: Option<PathBuf>,
        
        /// Number of nearest neighbors to find
        #[arg(short = 'k', long, default_value_t = 10)]
        top_k: u32,
        
        /// Distance metric (cosine, euclidean, dot, manhattan)
        #[arg(short, long, default_value = "cosine")]
        metric: String,
        
        /// Search strategy
        #[arg(short, long)]
        strategy: Option<String>,
        
        /// Mount point path
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: PathBuf,
    },
    /// Add vector embeddings to files
    AddEmbedding {
        /// File to add embedding to
        #[arg(short, long)]
        file: Option<PathBuf>,
        
        /// Vector data (comma-separated floats)
        #[arg(short, long)]
        vector: Option<String>,
        
        /// Vector format (float32, float16, binary)
        #[arg(long, default_value = "float32")]
        format: String,
        
        /// Batch processing file
        #[arg(short, long)]
        batch: Option<PathBuf>,
        
        /// Mount point path
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: PathBuf,
    },
    /// List and manage vector indexes
    ListIndexes {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
        
        /// Mount point path
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: PathBuf,
    },
    /// Create a new vector index
    CreateIndex {
        /// Index name
        #[arg(short, long)]
        name: String,
        
        /// Index type (hnsw, ivf)
        #[arg(short, long, default_value = "hnsw")]
        index_type: String,
        
        /// Vector dimensions
        #[arg(short, long)]
        dimensions: u32,
        
        /// Configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Mount point path
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: PathBuf,
    },
    /// Filesystem consistency check and repair
    Fsck {
        /// Perform repairs
        #[arg(short, long)]
        repair: bool,
        
        /// Check vector indexes
        #[arg(long)]
        check_vectors: bool,
        
        /// Mount point path
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: PathBuf,
    },
}

// OutputFormat already implements the necessary traits in the output module

fn main() {
    let cli = Cli::parse();
    
    // Set up logging based on verbosity
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else if !cli.quiet {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Create command configuration
    let config = CommandConfig::new()
        .with_format(cli.format)
        .with_verbose(cli.verbose)
        .with_quiet(cli.quiet)
        .with_timeout(cli.timeout);

    // Execute the command
    let result = match cli.command {
        Commands::Status { mount_point } => {
            let cmd = StatusCommand::new(config, mount_point);
            cmd.execute()
        }
        Commands::Search {
            query,
            file,
            top_k,
            metric,
            strategy,
            mount_point
        } => {
            // TODO: Implement search command
            eprintln!("Search command not yet implemented");
            Ok(())
        }
        Commands::AddEmbedding {
            file,
            vector,
            format,
            batch,
            mount_point
        } => {
            // TODO: Implement add-embedding command
            eprintln!("Add-embedding command not yet implemented");
            Ok(())
        }
        Commands::ListIndexes {
            detailed,
            mount_point
        } => {
            // TODO: Implement list-indexes command
            eprintln!("List-indexes command not yet implemented");
            Ok(())
        }
        Commands::CreateIndex {
            name,
            index_type,
            dimensions,
            config: index_config,
            mount_point
        } => {
            // TODO: Implement create-index command
            eprintln!("Create-index command not yet implemented");
            Ok(())
        }
        Commands::Fsck {
            repair,
            check_vectors,
            mount_point
        } => {
            // TODO: Implement fsck command
            eprintln!("Fsck command not yet implemented");
            Ok(())
        }
    };

    // Handle the result
    if let Err(e) = result {
        handle_error(e, cli.format);
        process::exit(1);
    }
}

/// Handle and display errors appropriately
fn handle_error(error: VexctlError, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let error_obj = serde_json::json!({
                "error": {
                    "type": error.category(),
                    "message": error.to_string(),
                    "suggestion": error.suggestion()
                }
            });
            eprintln!("{}", serde_json::to_string_pretty(&error_obj).unwrap());
        }
        _ => {
            eprintln!("Error: {}", error);
            if let Some(suggestion) = error.suggestion() {
                eprintln!("Suggestion: {}", suggestion);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test basic status command
        let cli = Cli::try_parse_from(&["vexctl", "status", "/mnt/vexfs"]).unwrap();
        assert!(matches!(cli.command, Commands::Status { .. }));
    }

    #[test]
    fn test_output_format_parsing() {
        let cli = Cli::try_parse_from(&["vexctl", "--format", "json", "status", "/mnt/vexfs"]).unwrap();
        assert_eq!(cli.format, OutputFormat::Json);
    }

    #[test]
    fn test_verbose_flag() {
        let cli = Cli::try_parse_from(&["vexctl", "--verbose", "status", "/mnt/vexfs"]).unwrap();
        assert!(cli.verbose);
    }
}
