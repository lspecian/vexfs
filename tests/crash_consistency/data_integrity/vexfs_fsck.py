#!/usr/bin/env python3
"""
VexFS Filesystem Consistency Checker (fsck)

This module provides comprehensive filesystem consistency checking for VexFS,
including metadata validation, vector index integrity, and data corruption detection.
"""

import os
import sys
import json
import logging
import hashlib
import struct
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Set
from dataclasses import dataclass
from enum import Enum

class ErrorSeverity(Enum):
    """Severity levels for filesystem errors"""
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"

@dataclass
class FsckError:
    """Represents a filesystem consistency error"""
    severity: ErrorSeverity
    error_type: str
    description: str
    location: str
    suggested_fix: str = ""
    auto_fixable: bool = False

class VexFSChecker:
    """Main filesystem consistency checker for VexFS"""
    
    def __init__(self, device_path: str, mount_point: str = None):
        self.device_path = device_path
        self.mount_point = mount_point
        self.errors: List[FsckError] = []
        self.warnings: List[FsckError] = []
        self.logger = self._setup_logging()
        self.stats = {
            'files_checked': 0,
            'directories_checked': 0,
            'vectors_checked': 0,
            'errors_found': 0,
            'warnings_found': 0,
            'bytes_verified': 0
        }
    
    def _setup_logging(self) -> logging.Logger:
        """Setup logging for fsck operations"""
        logger = logging.getLogger('vexfs_fsck')
        logger.setLevel(logging.DEBUG)
        
        # Create file handler
        log_file = Path(__file__).parent.parent / "results" / "fsck.log"
        log_file.parent.mkdir(exist_ok=True)
        
        handler = logging.FileHandler(log_file)
        formatter = logging.Formatter(
            '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
        )
        handler.setFormatter(formatter)
        logger.addHandler(handler)
        
        # Also log to console
        console_handler = logging.StreamHandler()
        console_handler.setFormatter(formatter)
        logger.addHandler(console_handler)
        
        return logger
    
    def check_filesystem(self, fix_errors: bool = False) -> Dict:
        """Perform comprehensive filesystem consistency check"""
        self.logger.info(f"Starting VexFS consistency check on {self.device_path}")
        
        results = {
            "device": self.device_path,
            "mount_point": self.mount_point,
            "check_time": None,
            "overall_status": "unknown",
            "errors": [],
            "warnings": [],
            "statistics": {},
            "fixes_applied": []
        }
        
        try:
            # Check if filesystem is mounted
            if not self.mount_point:
                self.mount_point = self._find_mount_point()
            
            if not self.mount_point:
                self._add_error(
                    ErrorSeverity.ERROR,
                    "mount_error",
                    "Filesystem is not mounted and no mount point specified",
                    self.device_path
                )
                return self._finalize_results(results)
            
            # Perform various consistency checks
            self._check_superblock()
            self._check_inode_table()
            self._check_directory_structure()
            self._check_file_integrity()
            self._check_vector_indices()
            self._check_free_space_consistency()
            self._check_journal_integrity()
            
            # Apply fixes if requested
            if fix_errors:
                fixes_applied = self._apply_fixes()
                results["fixes_applied"] = fixes_applied
            
            # Determine overall status
            if any(e.severity == ErrorSeverity.CRITICAL for e in self.errors):
                results["overall_status"] = "critical"
            elif any(e.severity == ErrorSeverity.ERROR for e in self.errors):
                results["overall_status"] = "errors"
            elif self.warnings:
                results["overall_status"] = "warnings"
            else:
                results["overall_status"] = "clean"
            
            return self._finalize_results(results)
            
        except Exception as e:
            self.logger.error(f"Error during filesystem check: {e}")
            self._add_error(
                ErrorSeverity.CRITICAL,
                "check_failure",
                f"Filesystem check failed: {str(e)}",
                self.device_path
            )
            return self._finalize_results(results)
    
    def _find_mount_point(self) -> Optional[str]:
        """Find the mount point for the device"""
        try:
            with open("/proc/mounts", "r") as f:
                for line in f:
                    parts = line.strip().split()
                    if len(parts) >= 3 and parts[0] == self.device_path:
                        return parts[1]
            return None
        except Exception as e:
            self.logger.error(f"Error finding mount point: {e}")
            return None
    
    def _check_superblock(self):
        """Check VexFS superblock integrity"""
        self.logger.info("Checking superblock integrity...")
        
        try:
            # Read superblock from device
            superblock_data = self._read_superblock()
            
            if not superblock_data:
                self._add_error(
                    ErrorSeverity.CRITICAL,
                    "superblock_missing",
                    "Cannot read superblock from device",
                    self.device_path,
                    "Filesystem may be corrupted or not VexFS"
                )
                return
            
            # Verify magic number
            if not self._verify_superblock_magic(superblock_data):
                self._add_error(
                    ErrorSeverity.CRITICAL,
                    "superblock_magic",
                    "Invalid superblock magic number",
                    self.device_path,
                    "Filesystem is not VexFS or severely corrupted"
                )
                return
            
            # Check superblock checksum
            if not self._verify_superblock_checksum(superblock_data):
                self._add_error(
                    ErrorSeverity.ERROR,
                    "superblock_checksum",
                    "Superblock checksum mismatch",
                    self.device_path,
                    "Superblock may be corrupted",
                    auto_fixable=True
                )
            
            # Validate superblock fields
            self._validate_superblock_fields(superblock_data)
            
            self.logger.info("Superblock check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "superblock_error",
                f"Error checking superblock: {str(e)}",
                self.device_path
            )
    
    def _read_superblock(self) -> Optional[bytes]:
        """Read superblock from device"""
        try:
            with open(self.device_path, "rb") as f:
                # VexFS superblock is typically at offset 1024
                f.seek(1024)
                return f.read(4096)  # Read 4KB superblock
        except Exception as e:
            self.logger.error(f"Cannot read superblock: {e}")
            return None
    
    def _verify_superblock_magic(self, data: bytes) -> bool:
        """Verify VexFS magic number in superblock"""
        # VexFS magic number (example: "VEXFS" + version)
        expected_magic = b"VEXFS001"
        return data[:8] == expected_magic
    
    def _verify_superblock_checksum(self, data: bytes) -> bool:
        """Verify superblock checksum"""
        try:
            # Extract stored checksum (last 32 bytes)
            stored_checksum = data[-32:]
            
            # Calculate checksum of data without the checksum field
            data_to_check = data[:-32]
            calculated_checksum = hashlib.sha256(data_to_check).digest()
            
            return stored_checksum == calculated_checksum
        except Exception:
            return False
    
    def _validate_superblock_fields(self, data: bytes):
        """Validate superblock field values"""
        try:
            # Parse superblock fields (example structure)
            magic = data[:8]
            version = struct.unpack('<I', data[8:12])[0]
            block_size = struct.unpack('<I', data[12:16])[0]
            total_blocks = struct.unpack('<Q', data[16:24])[0]
            free_blocks = struct.unpack('<Q', data[24:32])[0]
            
            # Validate block size
            if block_size not in [1024, 2048, 4096, 8192]:
                self._add_error(
                    ErrorSeverity.ERROR,
                    "invalid_block_size",
                    f"Invalid block size: {block_size}",
                    "superblock"
                )
            
            # Validate block counts
            if free_blocks > total_blocks:
                self._add_error(
                    ErrorSeverity.ERROR,
                    "invalid_block_count",
                    f"Free blocks ({free_blocks}) exceeds total blocks ({total_blocks})",
                    "superblock"
                )
            
            # Check version compatibility
            if version > 1:
                self._add_error(
                    ErrorSeverity.WARNING,
                    "version_mismatch",
                    f"Filesystem version {version} may not be fully supported",
                    "superblock"
                )
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "superblock_parse_error",
                f"Cannot parse superblock fields: {str(e)}",
                "superblock"
            )
    
    def _check_inode_table(self):
        """Check inode table integrity"""
        self.logger.info("Checking inode table integrity...")
        
        try:
            # This would read and validate the inode table
            # For now, we'll do basic checks on mounted filesystem
            
            if not os.path.exists(self.mount_point):
                self._add_error(
                    ErrorSeverity.ERROR,
                    "mount_point_missing",
                    f"Mount point {self.mount_point} does not exist",
                    self.mount_point
                )
                return
            
            # Check root directory inode
            root_stat = os.stat(self.mount_point)
            if not os.path.isdir(self.mount_point):
                self._add_error(
                    ErrorSeverity.CRITICAL,
                    "root_not_directory",
                    "Root inode is not a directory",
                    self.mount_point
                )
            
            # Walk filesystem and check inode consistency
            self._check_inode_consistency()
            
            self.logger.info("Inode table check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "inode_check_error",
                f"Error checking inode table: {str(e)}",
                "inode_table"
            )
    
    def _check_inode_consistency(self):
        """Check consistency of individual inodes"""
        for root, dirs, files in os.walk(self.mount_point):
            try:
                # Check directory inode
                dir_stat = os.stat(root)
                self.stats['directories_checked'] += 1
                
                # Validate directory permissions
                if not os.access(root, os.R_OK):
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "directory_unreadable",
                        f"Directory is not readable",
                        root
                    )
                
                # Check files in directory
                for file in files:
                    file_path = os.path.join(root, file)
                    try:
                        file_stat = os.stat(file_path)
                        self.stats['files_checked'] += 1
                        self.stats['bytes_verified'] += file_stat.st_size
                        
                        # Check for common inode issues
                        if file_stat.st_size < 0:
                            self._add_error(
                                ErrorSeverity.ERROR,
                                "negative_file_size",
                                f"File has negative size: {file_stat.st_size}",
                                file_path
                            )
                        
                        if file_stat.st_nlink == 0:
                            self._add_error(
                                ErrorSeverity.ERROR,
                                "zero_link_count",
                                "File has zero link count but is accessible",
                                file_path
                            )
                        
                    except OSError as e:
                        self._add_error(
                            ErrorSeverity.ERROR,
                            "file_stat_error",
                            f"Cannot stat file: {str(e)}",
                            file_path
                        )
                
            except OSError as e:
                self._add_error(
                    ErrorSeverity.ERROR,
                    "directory_stat_error",
                    f"Cannot stat directory: {str(e)}",
                    root
                )
    
    def _check_directory_structure(self):
        """Check directory structure integrity"""
        self.logger.info("Checking directory structure...")
        
        try:
            # Check for directory loops
            self._check_directory_loops()
            
            # Check directory entry consistency
            self._check_directory_entries()
            
            # Verify parent-child relationships
            self._check_parent_child_relationships()
            
            self.logger.info("Directory structure check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "directory_check_error",
                f"Error checking directory structure: {str(e)}",
                "directory_structure"
            )
    
    def _check_directory_loops(self):
        """Check for directory loops using cycle detection"""
        visited = set()
        
        def check_path(path: str, ancestors: Set[str]):
            real_path = os.path.realpath(path)
            
            if real_path in ancestors:
                self._add_error(
                    ErrorSeverity.ERROR,
                    "directory_loop",
                    f"Directory loop detected",
                    path
                )
                return
            
            if real_path in visited:
                return
            
            visited.add(real_path)
            
            try:
                if os.path.isdir(path):
                    new_ancestors = ancestors | {real_path}
                    for item in os.listdir(path):
                        item_path = os.path.join(path, item)
                        if os.path.isdir(item_path) and not os.path.islink(item_path):
                            check_path(item_path, new_ancestors)
            except PermissionError:
                pass  # Skip inaccessible directories
        
        check_path(self.mount_point, set())
    
    def _check_directory_entries(self):
        """Check directory entry consistency"""
        for root, dirs, files in os.walk(self.mount_point):
            try:
                # Check for invalid characters in names
                for name in dirs + files:
                    if '\0' in name:
                        self._add_error(
                            ErrorSeverity.ERROR,
                            "null_in_filename",
                            f"Filename contains null character",
                            os.path.join(root, name)
                        )
                    
                    if len(name) > 255:
                        self._add_error(
                            ErrorSeverity.ERROR,
                            "filename_too_long",
                            f"Filename too long: {len(name)} characters",
                            os.path.join(root, name)
                        )
                
                # Check for duplicate entries (case sensitivity issues)
                names_lower = [name.lower() for name in dirs + files]
                if len(names_lower) != len(set(names_lower)):
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "case_duplicate",
                        "Directory contains case-insensitive duplicate names",
                        root
                    )
                
            except OSError:
                pass  # Skip inaccessible directories
    
    def _check_parent_child_relationships(self):
        """Verify parent-child directory relationships"""
        # This would check that directory entries correctly reference their inodes
        # and that parent directories contain correct entries for their children
        pass
    
    def _check_file_integrity(self):
        """Check file data integrity"""
        self.logger.info("Checking file integrity...")
        
        try:
            # Check for file corruption using checksums if available
            self._check_file_checksums()
            
            # Check for truncated files
            self._check_truncated_files()
            
            # Verify file permissions and ownership
            self._check_file_permissions()
            
            self.logger.info("File integrity check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "file_integrity_error",
                f"Error checking file integrity: {str(e)}",
                "file_integrity"
            )
    
    def _check_file_checksums(self):
        """Check file checksums if available"""
        # VexFS might store checksums for files
        # This would verify file content against stored checksums
        pass
    
    def _check_truncated_files(self):
        """Check for truncated or corrupted files"""
        for root, dirs, files in os.walk(self.mount_point):
            for file in files:
                file_path = os.path.join(root, file)
                try:
                    # Try to read the entire file to detect I/O errors
                    with open(file_path, 'rb') as f:
                        while True:
                            chunk = f.read(8192)
                            if not chunk:
                                break
                except IOError as e:
                    self._add_error(
                        ErrorSeverity.ERROR,
                        "file_read_error",
                        f"Cannot read file: {str(e)}",
                        file_path
                    )
                except OSError as e:
                    self._add_error(
                        ErrorSeverity.ERROR,
                        "file_access_error",
                        f"File access error: {str(e)}",
                        file_path
                    )
    
    def _check_file_permissions(self):
        """Check file permissions and ownership consistency"""
        for root, dirs, files in os.walk(self.mount_point):
            for item in dirs + files:
                item_path = os.path.join(root, item)
                try:
                    stat_info = os.stat(item_path)
                    
                    # Check for unusual permission combinations
                    mode = stat_info.st_mode
                    if os.path.isfile(item_path) and (mode & 0o111):
                        # Executable file - check if it's actually executable
                        if not os.access(item_path, os.X_OK):
                            self._add_error(
                                ErrorSeverity.WARNING,
                                "permission_mismatch",
                                "File marked executable but not accessible as executable",
                                item_path
                            )
                    
                except OSError:
                    pass  # Skip inaccessible files
    
    def _check_vector_indices(self):
        """Check VexFS vector index integrity"""
        self.logger.info("Checking vector indices...")
        
        try:
            # Look for vector-related files and directories
            vector_dirs = []
            for root, dirs, files in os.walk(self.mount_point):
                if 'vectors' in dirs:
                    vector_dirs.append(os.path.join(root, 'vectors'))
                
                # Check for .vec files
                for file in files:
                    if file.endswith('.vec'):
                        self._check_vector_file(os.path.join(root, file))
            
            # Check vector directories
            for vector_dir in vector_dirs:
                self._check_vector_directory(vector_dir)
            
            self.logger.info("Vector index check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "vector_check_error",
                f"Error checking vector indices: {str(e)}",
                "vector_indices"
            )
    
    def _check_vector_file(self, file_path: str):
        """Check individual vector file integrity"""
        try:
            self.stats['vectors_checked'] += 1
            
            with open(file_path, 'rb') as f:
                data = f.read()
                
                # Check if file size is reasonable for vector data
                if len(data) == 0:
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "empty_vector_file",
                        "Vector file is empty",
                        file_path
                    )
                elif len(data) % 4 != 0:
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "vector_size_mismatch",
                        f"Vector file size ({len(data)}) not multiple of 4 bytes",
                        file_path
                    )
                
                # Check for obvious corruption patterns
                if data == b'\x00' * len(data):
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "vector_all_zeros",
                        "Vector file contains only zeros",
                        file_path
                    )
                elif data == b'\xff' * len(data):
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "vector_all_ones",
                        "Vector file contains only 0xFF bytes",
                        file_path
                    )
        
        except IOError as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "vector_read_error",
                f"Cannot read vector file: {str(e)}",
                file_path
            )
    
    def _check_vector_directory(self, dir_path: str):
        """Check vector directory structure"""
        try:
            vector_files = [f for f in os.listdir(dir_path) if f.endswith('.vec')]
            
            if len(vector_files) == 0:
                self._add_error(
                    ErrorSeverity.WARNING,
                    "empty_vector_directory",
                    "Vector directory contains no vector files",
                    dir_path
                )
            
            # Check for index files that should accompany vectors
            index_files = [f for f in os.listdir(dir_path) if f.endswith('.idx')]
            if len(vector_files) > 0 and len(index_files) == 0:
                self._add_error(
                    ErrorSeverity.WARNING,
                    "missing_vector_index",
                    "Vector directory has vectors but no index files",
                    dir_path
                )
        
        except OSError as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "vector_directory_error",
                f"Cannot access vector directory: {str(e)}",
                dir_path
            )
    
    def _check_free_space_consistency(self):
        """Check free space accounting consistency"""
        self.logger.info("Checking free space consistency...")
        
        try:
            # Get filesystem statistics
            statvfs = os.statvfs(self.mount_point)
            
            total_blocks = statvfs.f_blocks
            free_blocks = statvfs.f_bavail
            block_size = statvfs.f_frsize
            
            # Basic sanity checks
            if free_blocks > total_blocks:
                self._add_error(
                    ErrorSeverity.ERROR,
                    "free_space_overflow",
                    f"Free blocks ({free_blocks}) exceeds total blocks ({total_blocks})",
                    "free_space"
                )
            
            # Check if free space matches superblock
            # This would require reading the superblock and comparing
            
            self.logger.info("Free space check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "free_space_error",
                f"Error checking free space: {str(e)}",
                "free_space"
            )
    
    def _check_journal_integrity(self):
        """Check journal integrity if present"""
        self.logger.info("Checking journal integrity...")
        
        try:
            # VexFS might have a journal for crash consistency
            # This would check journal structure and replay consistency
            
            # Look for journal files
            journal_files = []
            for root, dirs, files in os.walk(self.mount_point):
                for file in files:
                    if 'journal' in file.lower() or file.endswith('.jnl'):
                        journal_files.append(os.path.join(root, file))
            
            if not journal_files:
                self._add_error(
                    ErrorSeverity.INFO,
                    "no_journal",
                    "No journal files found",
                    "journal"
                )
                return
            
            # Check each journal file
            for journal_file in journal_files:
                self._check_journal_file(journal_file)
            
            self.logger.info("Journal check completed")
            
        except Exception as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "journal_error",
                f"Error checking journal: {str(e)}",
                "journal"
            )
    
    def _check_journal_file(self, journal_path: str):
        """Check individual journal file"""
        try:
            with open(journal_path, 'rb') as f:
                # Basic journal file checks
                data = f.read(1024)  # Read header
                
                if len(data) == 0:
                    self._add_error(
                        ErrorSeverity.WARNING,
                        "empty_journal",
                        "Journal file is empty",
                        journal_path
                    )
                
                # Check for journal magic number or header
                # This would be VexFS-specific
                
        except IOError as e:
            self._add_error(
                ErrorSeverity.ERROR,
                "journal_read_error",
                f"Cannot read journal file: {str(e)}",
                journal_path
            )
    
    def _add_error(self, severity: ErrorSeverity, error_type: str, 
                   description: str, location: str, suggested_fix: str = "",
                   auto_fixable: bool = False):
        """Add an error to the error list"""
        error = FsckError(
            severity=severity,
            error_type=error_type,
            description=description,
            location=location,
            suggested_fix=suggested_fix,
            auto_fixable=auto_fixable
        )
        
        if severity in [ErrorSeverity.ERROR, ErrorSeverity.CRITICAL]:
            self.errors.append(error)
            self.stats['errors_found'] += 1
        else:
            self.warnings.append(error)
            self.stats['warnings_found'] += 1
        
        self.logger.log(
            logging.ERROR if severity in [ErrorSeverity.ERROR, ErrorSeverity.CRITICAL] else logging.WARNING,
            f"{severity.value.upper()}: {error_type} at {location}: {description}"
        )
    
    def _apply_fixes(self) -> List[str]:
        """Apply automatic fixes for auto-fixable errors"""
        fixes_applied = []
        
        for error in self.errors[:]:  # Copy list to allow modification
            if error.auto_fixable:
                try:
                    if self._apply_fix(error):
                        fixes_applied.append(f"Fixed {error.error_type} at {error.location}")
                        self.errors.remove(error)
                        self.stats['errors_found'] -= 1
                except Exception as e:
                    self.logger.error(f"Failed to apply fix for {error.error_type}: {e}")
        
        return fixes_applied
    
    def _apply_fix(self, error: FsckError) -> bool:
        """Apply a specific fix"""
        # Implement specific fixes based on error type
        if error.error_type == "superblock_checksum":
            return self._fix_superblock_checksum()
        
        # Add more fix implementations as needed
        return False
    
    def _fix_superblock_checksum(self) -> bool:
        """Fix superblock checksum"""
        try:
            # Read superblock
            superblock_data = self._read_superblock()
            if not superblock_data:
                return False
            
            # Recalculate checksum
            data_to_check = superblock_data[:-32]
            new_checksum = hashlib.sha256(data_to_check).digest()
            
            # Write back corrected superblock
            corrected_superblock = data_to_check + new_checksum
            
            with open(self.device_path, "r+b") as f:
                f.seek(1024)
                f.write(corrected_superblock)
                f.flush()
                os.fsync(f.fileno())
            
            self.logger.info("Superblock checksum fixed")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to fix superblock checksum: {e}")
            return False
    
    def _finalize_results(self, results: Dict) -> Dict:
        """Finalize and return check results"""
        import time
        
        results["check_time"] = time.time()
        results["errors"] = [
            {
                "severity": e.severity.value,
                "type": e.error_type,
                "description": e.description,
                "location": e.location,
                "suggested_fix": e.suggested_fix,
                "auto_fixable": e.auto_fixable
            }
            for e in self.errors
        ]
        results["warnings"] = [
            {
                "severity": w.severity.value,
                "type": w.error_type,
                "description": w.description,
                "location": w.location,
                "suggested_fix": w.suggested_fix,
                "auto_fixable": w.auto_fixable
            }
            for w in self.warnings
        ]
        results["statistics"] = self.stats.copy()
        
        # Save results to file
        results_file = Path(__file__).parent.parent / "results" / f"fsck_results_{int(time.time())}.json"
        results_file.parent.mkdir(exist_ok=True)
        
        with open(results_file, "w") as f:
            json.dump(results, f, indent=2)
        
        self.logger.info(f"Fsck results saved to: {results_file}")
        return results

def main():
    """Main entry point for VexFS fsck"""
    import argparse
    
    parser = argparse.ArgumentParser(description="VexFS Filesystem Consistency Checker")
    parser.add_argument("device", help="Device path to check")
    parser.add_argument("-m", "--mount-point", help="Mount point (auto-detected if not specified)")
    parser.add_argument("-f", "--fix", action="store_true", help="Automatically fix errors")
    parser.add_argument("-v", "--verbose", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    checker = VexFSChecker(args.device, args.mount_point)
    
    print(f"VexFS Filesystem Check - Device: {args.device}")
    print("=" * 50)
    
    results = checker.check_filesystem(fix_errors=args.fix)
    
    # Print summary
    print(f"\nFilesystem Check Results:")
    print(f"Overall Status: {results['overall_status'].upper()}")
    print(f"Files checked: {results['statistics']['files_checked']}")
    print(f"Directories checked: {results['statistics']['directories_checked']}")
    print(f"Vectors checked: {results['statistics']['vectors_checked']}")
    print(f"Bytes verified: {results['statistics']['bytes_verified']:,}")
    
    if results['errors']:
        print(f"\nErrors found: {len(results['errors'])}")
        for error in results['errors']:
            print(f"  {error['severity'].upper()}: {error['description']} ({error['location']})")
    
    if results['warnings']:
        print(f"\nWarnings: {len(results['warnings'])}")
        for warning in results['warnings']:
            print(f"  {warning['severity'].upper()}: {warning['description']} ({warning['location']})")
    
    if results['fixes_applied']:
        print(f"\nFixes applied: {len(results['fixes_applied'])}")
        for fix in results['fixes_applied']:
            print(f"  {fix}")
    
    # Exit with appropriate code
    if results['overall_status'] == 'critical':
        sys.exit(4)
    elif results['overall_status'] == 'errors':
        sys.exit(2)
    elif results['overall_status'] == 'warnings':
        sys.exit(1)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()