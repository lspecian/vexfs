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

//! Data Journaling Configuration Interface
//!
//! This module provides runtime configuration management for data journaling modes,
//! including mount options, /proc interface, and persistent configuration storage.

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::config::{DataJournalingMode, DataJournalingConfig};
use crate::shared::constants::*;
use crate::storage::superblock::VexfsSuperblock;

/// Configuration persistence location
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigPersistence {
    /// Store in superblock
    Superblock,
    /// Store in dedicated configuration area
    ConfigArea,
    /// Store in /proc interface (runtime only)
    ProcInterface,
}

/// Mount option parser for data journaling
pub struct DataJournalingMountOptions;

impl DataJournalingMountOptions {
    /// Parse mount options for data journaling configuration
    pub fn parse_mount_options(options: &str) -> VexfsResult<DataJournalingConfig> {
        let mut config = DataJournalingConfig::default();
        
        for option in options.split(',') {
            let option = option.trim();
            if option.is_empty() {
                continue;
            }
            
            match option {
                // Journaling mode options
                "data=metadata" | "data=metadata_only" => {
                    config.mode = DataJournalingMode::MetadataOnly;
                }
                "data=ordered" => {
                    config.mode = DataJournalingMode::OrderedData;
                }
                "data=journal" | "data=full" => {
                    config.mode = DataJournalingMode::FullDataJournaling;
                }
                
                // COW options
                "cow" => {
                    config.cow_enabled = true;
                }
                "nocow" => {
                    config.cow_enabled = false;
                }
                
                // Memory mapping options
                "mmap" => {
                    config.mmap_enabled = true;
                }
                "nommap" => {
                    config.mmap_enabled = false;
                }
                
                // Compression options
                "data_compress" => {
                    config.data_compression_enabled = true;
                }
                "nodata_compress" => {
                    config.data_compression_enabled = false;
                }
                
                // Space optimization
                "space_optimize" => {
                    config.space_optimization_enabled = true;
                }
                "nospace_optimize" => {
                    config.space_optimization_enabled = false;
                }
                
                // Dynamic switching
                "dynamic_switch" => {
                    config.dynamic_switching_enabled = true;
                }
                "nodynamic_switch" => {
                    config.dynamic_switching_enabled = false;
                }
                
                // Parameterized options
                _ if option.starts_with("max_data_journal=") => {
                    if let Some(value_str) = option.strip_prefix("max_data_journal=") {
                        if let Ok(value) = Self::parse_size(value_str) {
                            config.max_data_journal_size = value;
                        } else {
                            return Err(VexfsError::InvalidArgument(
                                format!("Invalid max_data_journal value: {}", value_str)
                            ));
                        }
                    }
                }
                
                _ if option.starts_with("large_write_threshold=") => {
                    if let Some(value_str) = option.strip_prefix("large_write_threshold=") {
                        if let Ok(value) = Self::parse_size(value_str) {
                            config.large_write_threshold = value;
                        } else {
                            return Err(VexfsError::InvalidArgument(
                                format!("Invalid large_write_threshold value: {}", value_str)
                            ));
                        }
                    }
                }
                
                // Ignore unknown options (they might be for other subsystems)
                _ => {}
            }
        }
        
        Ok(config)
    }
    
    /// Parse size string (e.g., "64M", "1G", "512K")
    fn parse_size(size_str: &str) -> Result<u64, ()> {
        let size_str = size_str.trim();
        if size_str.is_empty() {
            return Err(());
        }
        
        let (number_part, suffix) = if let Some(last_char) = size_str.chars().last() {
            if last_char.is_alphabetic() {
                (&size_str[..size_str.len()-1], Some(last_char.to_ascii_uppercase()))
            } else {
                (size_str, None)
            }
        } else {
            return Err(());
        };
        
        let base_value: u64 = number_part.parse().map_err(|_| ())?;
        
        let multiplier = match suffix {
            Some('K') => 1024,
            Some('M') => 1024 * 1024,
            Some('G') => 1024 * 1024 * 1024,
            Some('T') => 1024 * 1024 * 1024 * 1024,
            None => 1,
            _ => return Err(()),
        };
        
        Ok(base_value * multiplier)
    }
    
    /// Generate mount options string from configuration
    pub fn generate_mount_options(config: &DataJournalingConfig) -> String {
        let mut options = Vec::new();
        
        // Data journaling mode
        match config.mode {
            DataJournalingMode::MetadataOnly => options.push("data=metadata".to_string()),
            DataJournalingMode::OrderedData => options.push("data=ordered".to_string()),
            DataJournalingMode::FullDataJournaling => options.push("data=journal".to_string()),
        }
        
        // Boolean options
        if config.cow_enabled {
            options.push("cow".to_string());
        } else {
            options.push("nocow".to_string());
        }
        
        if config.mmap_enabled {
            options.push("mmap".to_string());
        } else {
            options.push("nommap".to_string());
        }
        
        if config.data_compression_enabled {
            options.push("data_compress".to_string());
        }
        
        if config.space_optimization_enabled {
            options.push("space_optimize".to_string());
        }
        
        if config.dynamic_switching_enabled {
            options.push("dynamic_switch".to_string());
        }
        
        // Size options
        options.push(format!("max_data_journal={}", Self::format_size(config.max_data_journal_size)));
        options.push(format!("large_write_threshold={}", Self::format_size(config.large_write_threshold)));
        
        options.join(",")
    }
    
    /// Format size as human-readable string
    fn format_size(size: u64) -> String {
        if size >= 1024 * 1024 * 1024 * 1024 {
            format!("{}T", size / (1024 * 1024 * 1024 * 1024))
        } else if size >= 1024 * 1024 * 1024 {
            format!("{}G", size / (1024 * 1024 * 1024))
        } else if size >= 1024 * 1024 {
            format!("{}M", size / (1024 * 1024))
        } else if size >= 1024 {
            format!("{}K", size / 1024)
        } else {
            format!("{}", size)
        }
    }
}

/// Runtime configuration interface
pub struct DataJournalingRuntimeConfig {
    /// Current configuration
    config: DataJournalingConfig,
    /// Persistence method
    persistence: ConfigPersistence,
}

impl DataJournalingRuntimeConfig {
    /// Create new runtime configuration
    pub fn new(config: DataJournalingConfig, persistence: ConfigPersistence) -> Self {
        Self {
            config,
            persistence,
        }
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &DataJournalingConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, new_config: DataJournalingConfig) -> VexfsResult<()> {
        // Validate configuration
        self.validate_config(&new_config)?;
        
        // Store old config for rollback
        let old_config = self.config.clone();
        
        // Apply new configuration
        self.config = new_config;
        
        // Persist configuration
        if let Err(e) = self.persist_config() {
            // Rollback on persistence failure
            self.config = old_config;
            return Err(e);
        }
        
        Ok(())
    }
    
    /// Validate configuration
    fn validate_config(&self, config: &DataJournalingConfig) -> VexfsResult<()> {
        // Check size limits
        if config.max_data_journal_size > VEXFS_MAX_JOURNAL_SIZE {
            return Err(VexfsError::InvalidArgument(
                "max_data_journal_size exceeds maximum limit".to_string()
            ));
        }
        
        if config.max_data_journal_size < VEXFS_MIN_JOURNAL_SIZE {
            return Err(VexfsError::InvalidArgument(
                "max_data_journal_size below minimum limit".to_string()
            ));
        }
        
        if config.large_write_threshold > config.max_data_journal_size {
            return Err(VexfsError::InvalidArgument(
                "large_write_threshold cannot exceed max_data_journal_size".to_string()
            ));
        }
        
        // Validate mode-specific requirements
        match config.mode {
            DataJournalingMode::FullDataJournaling => {
                if config.max_data_journal_size == 0 {
                    return Err(VexfsError::InvalidArgument(
                        "Full data journaling requires non-zero max_data_journal_size".to_string()
                    ));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Persist configuration
    fn persist_config(&self) -> VexfsResult<()> {
        match self.persistence {
            ConfigPersistence::Superblock => {
                self.persist_to_superblock()
            }
            ConfigPersistence::ConfigArea => {
                self.persist_to_config_area()
            }
            ConfigPersistence::ProcInterface => {
                // /proc interface is runtime-only, no persistence needed
                Ok(())
            }
        }
    }
    
    /// Persist configuration to superblock
    fn persist_to_superblock(&self) -> VexfsResult<()> {
        // TODO: Implement superblock persistence
        // This would involve:
        // 1. Reading current superblock
        // 2. Updating data journaling fields
        // 3. Writing back to storage
        Ok(())
    }
    
    /// Persist configuration to dedicated config area
    fn persist_to_config_area(&self) -> VexfsResult<()> {
        // TODO: Implement config area persistence
        // This would involve:
        // 1. Serializing configuration
        // 2. Writing to dedicated config blocks
        // 3. Updating config area metadata
        Ok(())
    }
    
    /// Load configuration from storage
    pub fn load_config(persistence: ConfigPersistence) -> VexfsResult<DataJournalingConfig> {
        match persistence {
            ConfigPersistence::Superblock => {
                Self::load_from_superblock()
            }
            ConfigPersistence::ConfigArea => {
                Self::load_from_config_area()
            }
            ConfigPersistence::ProcInterface => {
                // /proc interface uses default config
                Ok(DataJournalingConfig::default())
            }
        }
    }
    
    /// Load configuration from superblock
    fn load_from_superblock() -> VexfsResult<DataJournalingConfig> {
        // TODO: Implement superblock loading
        // For now, return default configuration
        Ok(DataJournalingConfig::default())
    }
    
    /// Load configuration from config area
    fn load_from_config_area() -> VexfsResult<DataJournalingConfig> {
        // TODO: Implement config area loading
        // For now, return default configuration
        Ok(DataJournalingConfig::default())
    }
}

/// /proc interface for runtime configuration
pub struct DataJournalingProcInterface;

impl DataJournalingProcInterface {
    /// Generate /proc/vexfs/data_journaling content
    pub fn generate_proc_content(config: &DataJournalingConfig) -> String {
        let mut content = String::new();
        
        content.push_str(&format!("mode: {}\n", Self::mode_to_string(config.mode)));
        content.push_str(&format!("cow_enabled: {}\n", config.cow_enabled));
        content.push_str(&format!("max_data_journal_size: {}\n", 
                                DataJournalingMountOptions::format_size(config.max_data_journal_size)));
        content.push_str(&format!("mmap_enabled: {}\n", config.mmap_enabled));
        content.push_str(&format!("large_write_threshold: {}\n", 
                                DataJournalingMountOptions::format_size(config.large_write_threshold)));
        content.push_str(&format!("data_compression_enabled: {}\n", config.data_compression_enabled));
        content.push_str(&format!("space_optimization_enabled: {}\n", config.space_optimization_enabled));
        content.push_str(&format!("dynamic_switching_enabled: {}\n", config.dynamic_switching_enabled));
        
        content
    }
    
    /// Parse /proc interface write command
    pub fn parse_proc_command(command: &str, current_config: &DataJournalingConfig) -> VexfsResult<DataJournalingConfig> {
        let mut config = current_config.clone();
        
        let command = command.trim();
        if command.is_empty() {
            return Ok(config);
        }
        
        // Parse key=value format
        if let Some((key, value)) = command.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            
            match key {
                "mode" => {
                    config.mode = Self::string_to_mode(value)?;
                }
                "cow_enabled" => {
                    config.cow_enabled = Self::parse_bool(value)?;
                }
                "max_data_journal_size" => {
                    config.max_data_journal_size = DataJournalingMountOptions::parse_size(value)
                        .map_err(|_| VexfsError::InvalidArgument(format!("Invalid size: {}", value)))?;
                }
                "mmap_enabled" => {
                    config.mmap_enabled = Self::parse_bool(value)?;
                }
                "large_write_threshold" => {
                    config.large_write_threshold = DataJournalingMountOptions::parse_size(value)
                        .map_err(|_| VexfsError::InvalidArgument(format!("Invalid size: {}", value)))?;
                }
                "data_compression_enabled" => {
                    config.data_compression_enabled = Self::parse_bool(value)?;
                }
                "space_optimization_enabled" => {
                    config.space_optimization_enabled = Self::parse_bool(value)?;
                }
                "dynamic_switching_enabled" => {
                    config.dynamic_switching_enabled = Self::parse_bool(value)?;
                }
                _ => {
                    return Err(VexfsError::InvalidArgument(format!("Unknown parameter: {}", key)));
                }
            }
        } else {
            return Err(VexfsError::InvalidArgument("Invalid command format, expected key=value".to_string()));
        }
        
        Ok(config)
    }
    
    /// Convert mode to string
    fn mode_to_string(mode: DataJournalingMode) -> &'static str {
        match mode {
            DataJournalingMode::MetadataOnly => "metadata_only",
            DataJournalingMode::OrderedData => "ordered",
            DataJournalingMode::FullDataJournaling => "full_journaling",
        }
    }
    
    /// Convert string to mode
    fn string_to_mode(s: &str) -> VexfsResult<DataJournalingMode> {
        match s {
            "metadata_only" | "metadata" => Ok(DataJournalingMode::MetadataOnly),
            "ordered" | "ordered_data" => Ok(DataJournalingMode::OrderedData),
            "full_journaling" | "full" | "journal" => Ok(DataJournalingMode::FullDataJournaling),
            _ => Err(VexfsError::InvalidArgument(format!("Invalid mode: {}", s))),
        }
    }
    
    /// Parse boolean value
    fn parse_bool(s: &str) -> VexfsResult<bool> {
        match s.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(VexfsError::InvalidArgument(format!("Invalid boolean value: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_option_parsing() {
        let options = "data=journal,cow,mmap,max_data_journal=128M,large_write_threshold=2M";
        let config = DataJournalingMountOptions::parse_mount_options(options).unwrap();
        
        assert_eq!(config.mode, DataJournalingMode::FullDataJournaling);
        assert!(config.cow_enabled);
        assert!(config.mmap_enabled);
        assert_eq!(config.max_data_journal_size, 128 * 1024 * 1024);
        assert_eq!(config.large_write_threshold, 2 * 1024 * 1024);
    }

    #[test]
    fn test_size_parsing() {
        assert_eq!(DataJournalingMountOptions::parse_size("1024").unwrap(), 1024);
        assert_eq!(DataJournalingMountOptions::parse_size("1K").unwrap(), 1024);
        assert_eq!(DataJournalingMountOptions::parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(DataJournalingMountOptions::parse_size("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(DataJournalingMountOptions::parse_size("2T").unwrap(), 2 * 1024 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_mount_option_generation() {
        let config = DataJournalingConfig {
            mode: DataJournalingMode::OrderedData,
            cow_enabled: true,
            max_data_journal_size: 64 * 1024 * 1024,
            mmap_enabled: false,
            large_write_threshold: 1024 * 1024,
            data_compression_enabled: true,
            space_optimization_enabled: true,
            dynamic_switching_enabled: true,
        };
        
        let options = DataJournalingMountOptions::generate_mount_options(&config);
        assert!(options.contains("data=ordered"));
        assert!(options.contains("cow"));
        assert!(options.contains("nommap"));
        assert!(options.contains("data_compress"));
    }

    #[test]
    fn test_proc_interface() {
        let config = DataJournalingConfig::default();
        let content = DataJournalingProcInterface::generate_proc_content(&config);
        assert!(content.contains("mode: ordered"));
        assert!(content.contains("cow_enabled: true"));
    }

    #[test]
    fn test_proc_command_parsing() {
        let config = DataJournalingConfig::default();
        let new_config = DataJournalingProcInterface::parse_proc_command("mode=full_journaling", &config).unwrap();
        assert_eq!(new_config.mode, DataJournalingMode::FullDataJournaling);
        
        let new_config = DataJournalingProcInterface::parse_proc_command("cow_enabled=false", &config).unwrap();
        assert!(!new_config.cow_enabled);
    }

    #[test]
    fn test_config_validation() {
        let runtime_config = DataJournalingRuntimeConfig::new(
            DataJournalingConfig::default(),
            ConfigPersistence::ProcInterface
        );
        
        let mut invalid_config = DataJournalingConfig::default();
        invalid_config.max_data_journal_size = VEXFS_MAX_JOURNAL_SIZE + 1;
        
        assert!(runtime_config.validate_config(&invalid_config).is_err());
    }
}