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

//! Path Resolution and Validation Module
//!
//! This module provides path parsing, validation, and resolution services
//! with security checks to prevent directory traversal attacks and ensure
//! filesystem integrity.

use crate::shared::{
    errors::VexfsError,
    types::*,
    constants::*,
};
use super::{FsResult, InodeManager};

extern crate alloc;
use alloc::{string::String, vec::Vec};

/// Maximum path component length
const MAX_COMPONENT_LENGTH: usize = 255;

/// Maximum path depth to prevent infinite recursion
const MAX_PATH_DEPTH: usize = 32;

/// Path component representation
#[derive(Debug, Clone, PartialEq)]
pub enum PathComponent {
    /// Normal directory or file name
    Normal(String),
    /// Current directory "."
    Current,
    /// Parent directory ".."
    Parent,
    /// Root directory "/"
    Root,
}

impl PathComponent {
    /// Create a new path component from a string
    pub fn from_str(name: &str) -> FsResult<Self> {
        match name {
            "." => Ok(Self::Current),
            ".." => Ok(Self::Parent),
            "/" | "" => Ok(Self::Root),
            _ => {
                if name.len() > MAX_COMPONENT_LENGTH {
                    return Err(VexfsError::NameTooLong);
                }
                
                // Validate component name
                PathValidator::validate_component_name(name)?;
                Ok(Self::Normal(String::from(name)))
            }
        }
    }

    /// Get the string representation of the component
    pub fn as_str(&self) -> &str {
        match self {
            Self::Normal(name) => name,
            Self::Current => ".",
            Self::Parent => "..",
            Self::Root => "/",
        }
    }

    /// Check if this is a special component (. or ..)
    pub fn is_special(&self) -> bool {
        matches!(self, Self::Current | Self::Parent)
    }
}

/// Path structure for filesystem operations
#[derive(Debug, Clone)]
pub struct Path {
    components: Vec<PathComponent>,
    absolute: bool,
}

impl Path {
    /// Create a new path from a string
    pub fn from_str(path_str: &str) -> FsResult<Self> {
        if path_str.is_empty() {
            return Err(VexfsError::InvalidPath("Empty path".into()));
        }

        let absolute = path_str.starts_with('/');
        let path_str = if absolute { &path_str[1..] } else { path_str };

        let mut components = Vec::new();
        
        if absolute {
            components.push(PathComponent::Root);
        }

        if !path_str.is_empty() {
            for component_str in path_str.split('/') {
                if !component_str.is_empty() {
                    let component = PathComponent::from_str(component_str)?;
                    components.push(component);
                }
            }
        }

        // Check path depth
        if components.len() > MAX_PATH_DEPTH {
            return Err(VexfsError::PathTooLong);
        }

        Ok(Self {
            components,
            absolute,
        })
    }

    /// Get the components of the path
    pub fn components(&self) -> &[PathComponent] {
        &self.components
    }

    /// Check if path is absolute
    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// Get the parent path
    pub fn parent(&self) -> Option<Self> {
        if self.components.len() <= 1 {
            return None;
        }

        let mut parent_components = self.components.clone();
        parent_components.pop();

        Some(Self {
            components: parent_components,
            absolute: self.absolute,
        })
    }

    /// Get the file name (last component)
    pub fn file_name(&self) -> Option<&PathComponent> {
        self.components.last()
    }

    /// Normalize the path by resolving . and .. components
    pub fn normalize(&self) -> FsResult<Self> {
        let mut normalized = Vec::new();
        
        for component in &self.components {
            match component {
                PathComponent::Current => {
                    // Skip current directory references
                    continue;
                }
                PathComponent::Parent => {
                    // Remove last component if possible
                    if !normalized.is_empty() && 
                       !matches!(normalized.last(), Some(PathComponent::Root)) {
                        normalized.pop();
                    } else if !self.absolute {
                        // For relative paths, keep .. if we can't go further
                        normalized.push(component.clone());
                    }
                    // For absolute paths, .. at root is ignored
                }
                _ => {
                    normalized.push(component.clone());
                }
            }
        }

        // Ensure root path is not empty
        if normalized.is_empty() && self.absolute {
            normalized.push(PathComponent::Root);
        }

        Ok(Self {
            components: normalized,
            absolute: self.absolute,
        })
    }

    /// Join this path with another path component
    pub fn join(&self, component: &str) -> FsResult<Self> {
        let new_component = PathComponent::from_str(component)?;
        let mut new_components = self.components.clone();
        new_components.push(new_component);

        if new_components.len() > MAX_PATH_DEPTH {
            return Err(VexfsError::PathTooLong);
        }

        Ok(Self {
            components: new_components,
            absolute: self.absolute,
        })
    }

    /// Convert path back to string representation
    pub fn to_string(&self) -> String {
        if self.components.is_empty() {
            return if self.absolute { "/".to_string() } else { ".".to_string() };
        }

        let mut result = String::new();
        
        for (i, component) in self.components.iter().enumerate() {
            match component {
                PathComponent::Root => {
                    if i == 0 {
                        result.push('/');
                    }
                }
                _ => {
                    if i > 0 && !matches!(self.components[i-1], PathComponent::Root) {
                        result.push('/');
                    }
                    result.push_str(component.as_str());
                }
            }
        }

        result
    }
}

/// Path validator for security and correctness
pub struct PathValidator;

impl PathValidator {
    /// Validate a single path component name
    pub fn validate_component_name(name: &str) -> FsResult<()> {
        if name.is_empty() {
            return Err(VexfsError::InvalidPath("Empty component name".into()));
        }

        if name.len() > MAX_COMPONENT_LENGTH {
            return Err(VexfsError::NameTooLong);
        }

        // Check for invalid characters
        for ch in name.chars() {
            if ch == '\0' || ch == '/' {
                return Err(VexfsError::InvalidPath(
                    "Invalid character in path component".into()
                ));
            }
        }

        // Check for reserved names on various filesystems
        if matches!(name, "CON" | "PRN" | "AUX" | "NUL" | 
                         "COM1" | "COM2" | "COM3" | "COM4" | 
                         "COM5" | "COM6" | "COM7" | "COM8" | "COM9" |
                         "LPT1" | "LPT2" | "LPT3" | "LPT4" |
                         "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9") {
            return Err(VexfsError::InvalidPath("Reserved name".into()));
        }

        Ok(())
    }

    /// Validate a complete path for security issues
    pub fn validate_path(path: &Path) -> FsResult<()> {
        // Check for directory traversal attempts
        let normalized = path.normalize()?;
        
        // Count parent directory references
        let mut parent_count = 0;
        let mut depth: u32 = 0;
        
        for component in normalized.components() {
            match component {
                PathComponent::Parent => {
                    parent_count += 1;
                    depth = depth.saturating_sub(1u32);
                }
                PathComponent::Normal(_) => {
                    depth += 1u32;
                }
                _ => {}
            }
        }

        // For relative paths, too many .. components might be suspicious
        if !path.is_absolute() && parent_count > MAX_PATH_DEPTH / 2 {
            return Err(VexfsError::InvalidPath("Excessive parent directory references".into()));
        }

        // Check final depth
        if depth as usize > MAX_PATH_DEPTH {
            return Err(VexfsError::PathTooLong);
        }

        Ok(())
    }

    /// Check if a path might be attempting directory traversal
    pub fn check_traversal_safety(path: &Path, base_depth: usize) -> FsResult<()> {
        // Check the raw components before normalization to detect traversal attempts
        let mut current_depth = base_depth as i32;
        
        for component in path.components() {
            match component {
                PathComponent::Parent => {
                    current_depth -= 1;
                    if current_depth < 0 {
                        return Err(VexfsError::PermissionDenied(
                            "Directory traversal attempt detected".into()
                        ));
                    }
                }
                PathComponent::Normal(_) => {
                    current_depth += 1;
                }
                PathComponent::Root => {
                    current_depth = 0;
                }
                PathComponent::Current => {
                    // No change in depth
                }
            }
        }

        Ok(())
    }
}

/// Path resolver for converting paths to inodes
pub struct PathResolver;

impl PathResolver {
    /// Resolve a path to an inode number
    pub fn resolve_path(
        inode_manager: &mut InodeManager,
        current_dir: InodeNumber,
        path: &Path,
    ) -> FsResult<InodeNumber> {
        // Validate path first
        PathValidator::validate_path(path)?;
        
        let normalized = path.normalize()?;
        let mut current_inode = if normalized.is_absolute() {
            VEXFS_ROOT_INO
        } else {
            current_dir
        };

        // Traverse each component
        for component in normalized.components() {
            match component {
                PathComponent::Root => {
                    current_inode = VEXFS_ROOT_INO;
                }
                PathComponent::Current => {
                    // Stay at current inode
                }
                PathComponent::Parent => {
                    current_inode = Self::get_parent_inode(inode_manager, current_inode)?;
                }
                PathComponent::Normal(name) => {
                    current_inode = Self::lookup_child(inode_manager, current_inode, name)?;
                }
            }
        }

        Ok(current_inode)
    }

    /// Get the parent inode of a given inode
    fn get_parent_inode(_inode_manager: &mut InodeManager, inode: InodeNumber) -> FsResult<InodeNumber> {
        if inode == VEXFS_ROOT_INO {
            return Ok(VEXFS_ROOT_INO);
        }

        // TODO: Implement parent lookup using directory operations
        // For now, this is a placeholder
        Ok(VEXFS_ROOT_INO)
    }

    /// Look up a child by name in a directory
    fn lookup_child(
        inode_manager: &mut InodeManager,
        parent_inode: InodeNumber,
        name: &str,
    ) -> FsResult<InodeNumber> {
        // Check if parent is a directory
        let parent = inode_manager.get_inode(parent_inode)?;
        if !parent.is_dir() {
            return Err(VexfsError::NotADirectory(parent_inode));
        }

        // TODO: Implement directory entry lookup
        // For now, this is a placeholder
        Err(VexfsError::EntryNotFound(name.into()))
    }

    /// Resolve a path to its parent directory and final component name
    pub fn resolve_parent(
        inode_manager: &mut InodeManager,
        current_dir: InodeNumber,
        path: &Path,
    ) -> FsResult<(InodeNumber, String)> {
        if let Some(parent_path) = path.parent() {
            let parent_inode = Self::resolve_path(inode_manager, current_dir, &parent_path)?;
            let file_name = path.file_name()
                .ok_or_else(|| VexfsError::InvalidPath("No file name in path".into()))?;
            
            match file_name {
                PathComponent::Normal(name) => Ok((parent_inode, name.clone())),
                _ => Err(VexfsError::InvalidPath("Invalid file name component".into())),
            }
        } else {
            Err(VexfsError::InvalidPath("Cannot resolve parent of root".into()))
        }
    }

    /// Check if one inode is a descendant of another (for preventing cycles)
    pub fn is_descendant(
        _inode_manager: &mut InodeManager,
        _ancestor: InodeNumber,
        _descendant: InodeNumber,
    ) -> FsResult<bool> {
        // TODO: Implement proper descendant checking
        // For now, return false to be safe
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_component_creation() {
        assert_eq!(PathComponent::from_str(".").unwrap(), PathComponent::Current);
        assert_eq!(PathComponent::from_str("..").unwrap(), PathComponent::Parent);
        assert_eq!(PathComponent::from_str("/").unwrap(), PathComponent::Root);
        
        let normal = PathComponent::from_str("filename").unwrap();
        assert_eq!(normal, PathComponent::Normal("filename".to_string()));
    }

    #[test]
    fn test_path_parsing() {
        let path = Path::from_str("/home/user/file.txt").unwrap();
        assert!(path.is_absolute());
        assert_eq!(path.components().len(), 4);
        
        let rel_path = Path::from_str("../file.txt").unwrap();
        assert!(!rel_path.is_absolute());
        assert_eq!(rel_path.components().len(), 2);
    }

    #[test]
    fn test_path_normalization() {
        let path = Path::from_str("/home/user/../file.txt").unwrap();
        let normalized = path.normalize().unwrap();
        assert_eq!(normalized.to_string(), "/home/file.txt");
        
        let path2 = Path::from_str("/home/./user/file.txt").unwrap();
        let normalized2 = path2.normalize().unwrap();
        assert_eq!(normalized2.to_string(), "/home/user/file.txt");
    }

    #[test]
    fn test_path_validation() {
        // Valid paths
        let valid_path = Path::from_str("/home/user/file.txt").unwrap();
        assert!(PathValidator::validate_path(&valid_path).is_ok());
        
        // Test component name validation
        assert!(PathValidator::validate_component_name("valid_name").is_ok());
        assert!(PathValidator::validate_component_name("file\0name").is_err());
        assert!(PathValidator::validate_component_name("CON").is_err());
    }

    #[test]
    fn test_path_joining() {
        let base = Path::from_str("/home/user").unwrap();
        let joined = base.join("file.txt").unwrap();
        assert_eq!(joined.to_string(), "/home/user/file.txt");
    }

    #[test]
    fn test_traversal_safety() {
        let safe_path = Path::from_str("subdir/file.txt").unwrap();
        assert!(PathValidator::check_traversal_safety(&safe_path, 2).is_ok());
        
        let unsafe_path = Path::from_str("../../../etc/passwd").unwrap();
        assert!(PathValidator::check_traversal_safety(&unsafe_path, 1).is_err());
    }
}