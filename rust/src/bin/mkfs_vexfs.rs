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
 */

//! mkfs.vexfs - VexFS Filesystem Creation Utility
//!
//! This utility formats block devices with the VexFS filesystem.
//! It creates the superblock, block groups, and initializes all
//! necessary metadata structures.

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Write, Seek, SeekFrom};
use std::path::Path;

use vexfs::storage::superblock::{SuperblockManager, VexfsSuperblock};
use vexfs::storage::layout::{LayoutCalculator, VexfsLayout};
use vexfs::shared::errors::{VexfsError, VexfsResult, IoErrorKind};
use vexfs::shared::constants::*;

/// mkfs.vexfs command line options
#[derive(Debug)]
struct MkfsOptions {
    device: String,
    block_size: u32,
    inode_ratio: u32,
    journal_size: Option<u32>,
    enable_vectors: bool,
    vector_dimensions: Option<u16>,
    volume_label: Option<String>,
    force: bool,
    verbose: bool,
    dry_run: bool,
}

impl Default for MkfsOptions {
    fn default() -> Self {
        Self {
            device: String::new(),
            block_size: VEXFS_DEFAULT_BLOCK_SIZE as u32,
            inode_ratio: 16384, // 16KB per inode
            journal_size: None, // Auto-calculate
            enable_vectors: false,
            vector_dimensions: None,
            volume_label: None,
            force: false,
            verbose: false,
            dry_run: false,
        }
    }
}

fn main() {
    let result = run();
    match result {
        Ok(()) => {
            println!("VexFS filesystem created successfully.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> VexfsResult<()> {
    let options = parse_args()?;
    
    if options.verbose {
        println!("mkfs.vexfs - VexFS Filesystem Creation Utility");
        println!("Device: {}", options.device);
        println!("Block size: {} bytes", options.block_size);
        println!("Inode ratio: {} bytes per inode", options.inode_ratio);
        if let Some(journal_size) = options.journal_size {
            println!("Journal size: {} blocks", journal_size);
        }
        if options.enable_vectors {
            println!("Vector support: enabled");
            if let Some(dims) = options.vector_dimensions {
                println!("Vector dimensions: {}", dims);
            }
        }
        if let Some(ref label) = options.volume_label {
            println!("Volume label: {}", label);
        }
    }

    // Safety checks
    validate_device(&options)?;
    
    if options.dry_run {
        println!("Dry run mode - no changes will be made");
        return Ok(());
    }

    // Create the filesystem
    create_filesystem(&options)?;
    
    Ok(())
}

fn parse_args() -> VexfsResult<MkfsOptions> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Err(VexfsError::InvalidArgument("device path required".to_string()));
    }

    let mut options = MkfsOptions::default();
    let mut i = 1;
    
    while i < args.len() {
        match args[i].as_str() {
            "-b" | "--block-size" => {
                i += 1;
                if i >= args.len() {
                    return Err(VexfsError::InvalidArgument("block size value required".to_string()));
                }
                options.block_size = args[i].parse()
                    .map_err(|_| VexfsError::InvalidArgument("invalid block size".to_string()))?;
            }
            "-i" | "--inode-ratio" => {
                i += 1;
                if i >= args.len() {
                    return Err(VexfsError::InvalidArgument("inode ratio value required".to_string()));
                }
                options.inode_ratio = args[i].parse()
                    .map_err(|_| VexfsError::InvalidArgument("invalid inode ratio".to_string()))?;
            }
            "-J" | "--journal-size" => {
                i += 1;
                if i >= args.len() {
                    return Err(VexfsError::InvalidArgument("journal size value required".to_string()));
                }
                options.journal_size = Some(args[i].parse()
                    .map_err(|_| VexfsError::InvalidArgument("invalid journal size".to_string()))?);
            }
            "-V" | "--enable-vectors" => {
                options.enable_vectors = true;
            }
            "-D" | "--vector-dimensions" => {
                i += 1;
                if i >= args.len() {
                    return Err(VexfsError::InvalidArgument("vector dimensions value required".to_string()));
                }
                options.vector_dimensions = Some(args[i].parse()
                    .map_err(|_| VexfsError::InvalidArgument("invalid vector dimensions".to_string()))?);
                options.enable_vectors = true; // Auto-enable vectors
            }
            "-L" | "--label" => {
                i += 1;
                if i >= args.len() {
                    return Err(VexfsError::InvalidArgument("volume label required".to_string()));
                }
                options.volume_label = Some(args[i].clone());
            }
            "-f" | "--force" => {
                options.force = true;
            }
            "-v" | "--verbose" => {
                options.verbose = true;
            }
            "-n" | "--dry-run" => {
                options.dry_run = true;
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            arg if arg.starts_with('-') => {
                return Err(VexfsError::InvalidArgument(format!("unknown option: {}", arg)));
            }
            _ => {
                if options.device.is_empty() {
                    options.device = args[i].clone();
                } else {
                    return Err(VexfsError::InvalidArgument("multiple devices specified".to_string()));
                }
            }
        }
        i += 1;
    }

    if options.device.is_empty() {
        return Err(VexfsError::InvalidArgument("device path required".to_string()));
    }

    // Validate options
    if !options.block_size.is_power_of_two() || 
       options.block_size < VEXFS_MIN_BLOCK_SIZE as u32 ||
       options.block_size > VEXFS_MAX_BLOCK_SIZE as u32 {
        return Err(VexfsError::InvalidArgument("invalid block size".to_string()));
    }

    if options.inode_ratio < 1024 {
        return Err(VexfsError::InvalidArgument("inode ratio too small".to_string()));
    }

    if let Some(dims) = options.vector_dimensions {
        if dims == 0 || dims > 4096 {
            return Err(VexfsError::InvalidArgument("invalid vector dimensions".to_string()));
        }
    }

    Ok(options)
}

fn print_usage() {
    println!("Usage: mkfs.vexfs [options] <device>");
    println!();
    println!("Options:");
    println!("  -b, --block-size SIZE     Block size in bytes (default: 4096)");
    println!("  -i, --inode-ratio RATIO   Bytes per inode (default: 16384)");
    println!("  -J, --journal-size SIZE   Journal size in blocks (auto-calculated)");
    println!("  -V, --enable-vectors      Enable vector storage support");
    println!("  -D, --vector-dimensions N Vector dimensions (implies --enable-vectors)");
    println!("  -L, --label LABEL         Volume label");
    println!("  -f, --force               Force creation (skip safety checks)");
    println!("  -v, --verbose             Verbose output");
    println!("  -n, --dry-run             Show what would be done without making changes");
    println!("  -h, --help                Show this help");
    println!();
    println!("Examples:");
    println!("  mkfs.vexfs /dev/sdb1");
    println!("  mkfs.vexfs -V -D 768 -L \"VectorDB\" /dev/sdb1");
    println!("  mkfs.vexfs -b 8192 -i 8192 /dev/sdb1");
}

fn validate_device(options: &MkfsOptions) -> VexfsResult<()> {
    let path = Path::new(&options.device);
    
    if !path.exists() {
        return Err(VexfsError::InvalidArgument(format!("device {} does not exist", options.device)));
    }

    // Check if it's a block device
    let metadata = path.metadata()
        .map_err(|e| VexfsError::IoError(IoErrorKind::DeviceError))?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if !metadata.file_type().is_block_device() && !metadata.file_type().is_file() {
            if !options.force {
                return Err(VexfsError::InvalidArgument(
                    "not a block device or regular file (use --force to override)".to_string()
                ));
            }
        }
    }

    // Check if device is mounted (basic check)
    if !options.force {
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            if mounts.contains(&options.device) {
                return Err(VexfsError::InvalidArgument(
                    "device appears to be mounted (use --force to override)".to_string()
                ));
            }
        }
    }

    // Get device size
    let device_size = get_device_size(&options.device)?;
    if device_size < 1024 * 1024 {
        return Err(VexfsError::InvalidArgument("device too small (minimum 1MB)".to_string()));
    }

    if options.verbose {
        println!("Device size: {} bytes ({:.1} MB)", device_size, device_size as f64 / (1024.0 * 1024.0));
    }

    Ok(())
}

fn get_device_size(device_path: &str) -> VexfsResult<u64> {
    let mut file = File::open(device_path)
        .map_err(|_| VexfsError::IoError(IoErrorKind::DeviceError))?;
    
    // Seek to end to get size
    let size = file.seek(SeekFrom::End(0))
        .map_err(|_| VexfsError::IoError(IoErrorKind::SeekError))?;
    
    Ok(size)
}

fn create_filesystem(options: &MkfsOptions) -> VexfsResult<()> {
    if options.verbose {
        println!("Creating VexFS filesystem...");
    }

    // Get device size
    let device_size = get_device_size(&options.device)?;
    
    // Calculate filesystem layout
    let layout = LayoutCalculator::calculate_mkfs_layout(
        device_size,
        Some(options.block_size),
        Some(options.inode_ratio),
        options.journal_size,
        options.enable_vectors,
    )?;

    if options.verbose {
        print_layout_info(&layout);
    }

    // Open device for writing
    let mut device = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&options.device)
        .map_err(|_| VexfsError::IoError(IoErrorKind::DeviceError))?;

    // Create superblock manager
    let mut sb_manager = SuperblockManager::new_with_params(options.block_size, true)?;
    
    // Calculate total inodes
    let total_inodes = layout.group_count * layout.inodes_per_group;
    
    // Create filesystem superblock
    let superblock = sb_manager.create_filesystem(
        layout.total_blocks,
        total_inodes,
        layout.block_size,
        layout.blocks_per_group,
        layout.inodes_per_group,
        options.volume_label.as_deref(),
    )?;

    // Enable vectors if requested
    if options.enable_vectors {
        let mut sb = *superblock;
        let dimensions = options.vector_dimensions.unwrap_or(768); // Default to 768D
        sb.enable_vectors(dimensions, 1, 0)?; // HNSW algorithm, L2 metric
        sb_manager = SuperblockManager::new_with_params(options.block_size, true)?;
        sb_manager.create_filesystem(
            layout.total_blocks,
            total_inodes,
            layout.block_size,
            layout.blocks_per_group,
            layout.inodes_per_group,
            options.volume_label.as_deref(),
        )?;
        let sb_mut = sb_manager.get_superblock_mut()?;
        *sb_mut = sb;
    }

    if options.verbose {
        println!("Writing superblock...");
    }

    // Write superblock to device
    write_superblock(&mut device, &sb_manager, &layout)?;

    if options.verbose {
        println!("Initializing block groups...");
    }

    // Initialize block groups
    initialize_block_groups(&mut device, &layout)?;

    if options.verbose {
        println!("Writing backup superblocks...");
    }

    // Write backup superblocks
    write_backup_superblocks(&mut device, &sb_manager, &layout)?;

    // Sync to ensure all data is written
    device.sync_all()
        .map_err(|_| VexfsError::IoError(IoErrorKind::FlushError))?;

    if options.verbose {
        println!("Filesystem creation completed successfully.");
        print_filesystem_info(sb_manager.get_superblock()?, &layout);
    }

    Ok(())
}

fn print_layout_info(layout: &VexfsLayout) {
    println!("Filesystem layout:");
    println!("  Total blocks: {}", layout.total_blocks);
    println!("  Block size: {} bytes", layout.block_size);
    println!("  Block groups: {}", layout.group_count);
    println!("  Blocks per group: {}", layout.blocks_per_group);
    println!("  Inodes per group: {}", layout.inodes_per_group);
    println!("  Journal blocks: {}", layout.journal_blocks);
    if layout.vector_blocks > 0 {
        println!("  Vector blocks: {}", layout.vector_blocks);
    }
    println!("  Data blocks: {}", layout.get_data_blocks());
    println!("  Efficiency: {:.1}%", layout.get_efficiency());
}

fn write_superblock(
    device: &mut File,
    sb_manager: &SuperblockManager,
    _layout: &VexfsLayout,
) -> VexfsResult<()> {
    // Serialize superblock
    let sb_data = sb_manager.serialize_superblock()?;
    
    // Write to block 0
    device.seek(SeekFrom::Start(0))
        .map_err(|_| VexfsError::IoError(IoErrorKind::SeekError))?;
    
    device.write_all(&sb_data)
        .map_err(|_| VexfsError::IoError(IoErrorKind::WriteError))?;
    
    Ok(())
}

fn initialize_block_groups(device: &mut File, layout: &VexfsLayout) -> VexfsResult<()> {
    for group_id in 0..layout.group_count {
        let group_layout = layout.get_group_layout(group_id)?;
        
        // Initialize block bitmap (all free initially)
        let bitmap_size = layout.block_size as usize;
        let bitmap_data = vec![0u8; bitmap_size]; // All bits 0 = all blocks free
        
        device.seek(SeekFrom::Start(group_layout.block_bitmap_block * layout.block_size as u64))
            .map_err(|_| VexfsError::IoError(IoErrorKind::SeekError))?;
        
        device.write_all(&bitmap_data)
            .map_err(|_| VexfsError::IoError(IoErrorKind::WriteError))?;
        
        // Initialize inode bitmap (all free initially)
        device.seek(SeekFrom::Start(group_layout.inode_bitmap_block * layout.block_size as u64))
            .map_err(|_| VexfsError::IoError(IoErrorKind::SeekError))?;
        
        device.write_all(&bitmap_data)
            .map_err(|_| VexfsError::IoError(IoErrorKind::WriteError))?;
        
        // Initialize inode table (zero out)
        let inode_table_size = group_layout.inode_table_blocks as usize * layout.block_size as usize;
        let inode_table_data = vec![0u8; inode_table_size];
        
        device.seek(SeekFrom::Start(group_layout.inode_table_block * layout.block_size as u64))
            .map_err(|_| VexfsError::IoError(IoErrorKind::SeekError))?;
        
        device.write_all(&inode_table_data)
            .map_err(|_| VexfsError::IoError(IoErrorKind::WriteError))?;
    }
    
    Ok(())
}

fn write_backup_superblocks(
    device: &mut File,
    sb_manager: &SuperblockManager,
    layout: &VexfsLayout,
) -> VexfsResult<()> {
    let sb_layout = layout.get_superblock_layout();
    let sb_data = sb_manager.serialize_superblock()?;
    
    for &backup_block in &sb_layout.backup_blocks {
        device.seek(SeekFrom::Start(backup_block * layout.block_size as u64))
            .map_err(|_| VexfsError::IoError(IoErrorKind::SeekError))?;
        
        device.write_all(&sb_data)
            .map_err(|_| VexfsError::IoError(IoErrorKind::WriteError))?;
    }
    
    Ok(())
}

fn print_filesystem_info(superblock: &VexfsSuperblock, layout: &VexfsLayout) {
    // Copy values to avoid packed field references
    let magic = superblock.s_magic;
    let version_major = superblock.s_version_major;
    let version_minor = superblock.s_version_minor;
    let block_size = superblock.s_block_size;
    let blocks_count = superblock.s_blocks_count;
    let free_blocks_count = superblock.s_free_blocks_count;
    let inodes_count = superblock.s_inodes_count;
    let free_inodes_count = superblock.s_free_inodes_count;
    let group_count = superblock.s_group_count;
    let journal_blocks = superblock.s_journal_blocks;
    let vector_dimensions = superblock.s_vector_dimensions;
    let vector_algorithm = superblock.s_vector_algorithm;
    
    println!();
    println!("VexFS filesystem information:");
    println!("  Magic: 0x{:x}", magic);
    println!("  Version: {}.{}", version_major, version_minor);
    println!("  Block size: {} bytes", block_size);
    println!("  Total blocks: {}", blocks_count);
    println!("  Free blocks: {}", free_blocks_count);
    println!("  Total inodes: {}", inodes_count);
    println!("  Free inodes: {}", free_inodes_count);
    println!("  Block groups: {}", group_count);
    
    if let Ok(volume_name) = superblock.get_volume_name() {
        if !volume_name.is_empty() {
            println!("  Volume label: {}", volume_name);
        }
    }
    
    if superblock.supports_vectors() {
        println!("  Vector support: enabled");
        println!("  Vector dimensions: {}", vector_dimensions);
        println!("  Vector algorithm: {}", vector_algorithm);
    }
    
    println!("  Journal blocks: {}", journal_blocks);
    println!("  Data blocks: {}", layout.get_data_blocks());
    println!("  Utilization efficiency: {:.1}%", layout.get_efficiency());
}