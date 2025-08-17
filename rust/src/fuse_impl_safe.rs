// Safe FUSE implementation with proper error handling
// This is a simplified version that focuses on stability over features

use fuse::{
    FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, ReplyCreate, ReplyOpen, ReplyEmpty, ReplyStatfs,
};
use libc::{ENOENT, ENOSYS, ENOTDIR, EEXIST, EINVAL, EIO, EACCES, EPERM, ENOMEM, ENOTEMPTY};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use time01::Timespec;

use crate::fuse_error_handling::{safe_lock, log_error};

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

#[derive(Debug, Clone)]
struct VexFSFile {
    ino: u64,
    name: String,
    parent: u64,
    content: Vec<u8>,
    attr: FileAttr,
    children: Vec<u64>,  // For directories
}

pub struct SafeVexFS {
    files: Arc<Mutex<HashMap<u64, VexFSFile>>>,
    parent_name_to_ino: Arc<Mutex<HashMap<(u64, String), u64>>>,
    next_ino: Arc<Mutex<u64>>,
}

impl SafeVexFS {
    pub fn new() -> Self {
        let mut files = HashMap::new();
        
        // Create root directory
        let now = SystemTime::now();
        let root_attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        
        files.insert(1, VexFSFile {
            ino: 1,
            name: String::from("/"),
            parent: 1,
            content: Vec::new(),
            attr: root_attr,
            children: Vec::new(),
        });
        
        Self {
            files: Arc::new(Mutex::new(files)),
            parent_name_to_ino: Arc::new(Mutex::new(HashMap::new())),
            next_ino: Arc::new(Mutex::new(2)),
        }
    }
    
    fn get_next_ino(&self) -> u64 {
        match safe_lock(&self.next_ino, "get_next_ino") {
            Ok(mut next_ino) => {
                let ino = *next_ino;
                *next_ino += 1;
                ino
            }
            Err(_) => {
                log_error("get_next_ino", "Failed to acquire lock, using timestamp");
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            }
        }
    }
}

impl Filesystem for SafeVexFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Look up the inode
        let parent_name_result = safe_lock(&self.parent_name_to_ino, "lookup");
        if let Ok(parent_name_to_ino) = parent_name_result {
            if let Some(&ino) = parent_name_to_ino.get(&(parent, name_str.clone())) {
                let files_result = safe_lock(&self.files, "lookup_files");
                if let Ok(files) = files_result {
                    if let Some(file) = files.get(&ino) {
                        reply.entry(&TTL, &file.attr, 0);
                        return;
                    }
                }
            }
        }
        
        reply.error(ENOENT);
    }
    
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match safe_lock(&self.files, "getattr") {
            Ok(files) => {
                if let Some(file) = files.get(&ino) {
                    reply.attr(&TTL, &file.attr);
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(errno) => reply.error(errno),
        }
    }
    
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        match safe_lock(&self.files, "read") {
            Ok(files) => {
                if let Some(file) = files.get(&ino) {
                    let offset = offset as usize;
                    let size = size as usize;
                    
                    if offset < file.content.len() {
                        let end = std::cmp::min(offset + size, file.content.len());
                        reply.data(&file.content[offset..end]);
                    } else {
                        reply.data(&[]);
                    }
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(errno) => reply.error(errno),
        }
    }
    
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        match safe_lock(&self.files, "write") {
            Ok(mut files) => {
                if let Some(file) = files.get_mut(&ino) {
                    let offset = offset as usize;
                    
                    // Extend file if necessary
                    if offset + data.len() > file.content.len() {
                        file.content.resize(offset + data.len(), 0);
                    }
                    
                    // Write data
                    file.content[offset..offset + data.len()].copy_from_slice(data);
                    
                    // Update attributes
                    file.attr.size = file.content.len() as u64;
                    file.attr.mtime = SystemTime::now();
                    
                    reply.written(data.len() as u32);
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(errno) => reply.error(errno),
        }
    }
    
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, _flags: u32, reply: ReplyCreate) {
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Check if file already exists
        {
            match safe_lock(&self.parent_name_to_ino, "create_check") {
                Ok(parent_name_to_ino) => {
                    if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                        reply.error(EEXIST);
                        return;
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        }
        
        let ino = self.get_next_ino();
        let now = SystemTime::now();
        
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            parent,
            content: Vec::new(),
            attr,
            children: Vec::new(),
        };
        
        // Insert the file
        {
            match safe_lock(&self.files, "create_insert") {
                Ok(mut files) => {
                    files.insert(ino, file);
                    
                    // Update parent's children list
                    if let Some(parent_dir) = files.get_mut(&parent) {
                        parent_dir.children.push(ino);
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        }
        
        // Update mapping
        {
            match safe_lock(&self.parent_name_to_ino, "create_mapping") {
                Ok(mut parent_name_to_ino) => {
                    parent_name_to_ino.insert((parent, name_str), ino);
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        }
        
        reply.created(&TTL, &attr, 0, 0, 0);
    }
    
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Check if directory already exists
        {
            match safe_lock(&self.parent_name_to_ino, "mkdir_check") {
                Ok(parent_name_to_ino) => {
                    if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                        reply.error(EEXIST);
                        return;
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        }
        
        let ino = self.get_next_ino();
        let now = SystemTime::now();
        
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        
        let dir = VexFSFile {
            ino,
            name: name_str.clone(),
            parent,
            content: Vec::new(),
            attr,
            children: Vec::new(),
        };
        
        // Insert the directory
        {
            match safe_lock(&self.files, "mkdir_insert") {
                Ok(mut files) => {
                    files.insert(ino, dir);
                    
                    // Update parent's children list
                    if let Some(parent_dir) = files.get_mut(&parent) {
                        parent_dir.children.push(ino);
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        }
        
        // Update mapping
        {
            match safe_lock(&self.parent_name_to_ino, "mkdir_mapping") {
                Ok(mut parent_name_to_ino) => {
                    parent_name_to_ino.insert((parent, name_str), ino);
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        match safe_lock(&self.files, "readdir") {
            Ok(files) => {
                if let Some(dir) = files.get(&ino) {
                    if dir.attr.kind != FileType::Directory {
                        reply.error(ENOTDIR);
                        return;
                    }
                    
                    let mut entries = vec![
                        (ino, FileType::Directory, "."),
                        (dir.parent, FileType::Directory, ".."),
                    ];
                    
                    // Add children
                    for &child_ino in &dir.children {
                        if let Some(child) = files.get(&child_ino) {
                            entries.push((child_ino, child.attr.kind, &child.name));
                        }
                    }
                    
                    // Send entries
                    for (i, entry) in entries.iter().enumerate().skip(offset as usize) {
                        if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                            break;
                        }
                    }
                    
                    reply.ok();
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(errno) => reply.error(errno),
        }
    }
    
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Look up the file to remove
        let ino_to_remove = {
            match safe_lock(&self.parent_name_to_ino, "unlink_lookup") {
                Ok(parent_name_to_ino) => {
                    parent_name_to_ino.get(&(parent, name_str.clone())).copied()
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        };
        
        if let Some(ino) = ino_to_remove {
            // Remove the file
            {
                match safe_lock(&self.files, "unlink_remove") {
                    Ok(mut files) => {
                        // Check it's not a directory
                        if let Some(file) = files.get(&ino) {
                            if file.attr.kind == FileType::Directory {
                                reply.error(EPERM);
                                return;
                            }
                        }
                        
                        // Remove from parent's children
                        if let Some(parent_dir) = files.get_mut(&parent) {
                            parent_dir.children.retain(|&x| x != ino);
                        }
                        
                        // Remove the file
                        files.remove(&ino);
                    }
                    Err(errno) => {
                        reply.error(errno);
                        return;
                    }
                }
            }
            
            // Remove from mapping
            {
                match safe_lock(&self.parent_name_to_ino, "unlink_mapping") {
                    Ok(mut parent_name_to_ino) => {
                        parent_name_to_ino.remove(&(parent, name_str));
                    }
                    Err(errno) => {
                        reply.error(errno);
                        return;
                    }
                }
            }
            
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Look up the directory to remove
        let ino_to_remove = {
            match safe_lock(&self.parent_name_to_ino, "rmdir_lookup") {
                Ok(parent_name_to_ino) => {
                    parent_name_to_ino.get(&(parent, name_str.clone())).copied()
                }
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        };
        
        if let Some(ino) = ino_to_remove {
            // Remove the directory
            {
                match safe_lock(&self.files, "rmdir_remove") {
                    Ok(mut files) => {
                        // Check it's a directory and empty
                        if let Some(dir) = files.get(&ino) {
                            if dir.attr.kind != FileType::Directory {
                                reply.error(ENOTDIR);
                                return;
                            }
                            if !dir.children.is_empty() {
                                reply.error(ENOTEMPTY);
                                return;
                            }
                        }
                        
                        // Remove from parent's children
                        if let Some(parent_dir) = files.get_mut(&parent) {
                            parent_dir.children.retain(|&x| x != ino);
                        }
                        
                        // Remove the directory
                        files.remove(&ino);
                    }
                    Err(errno) => {
                        reply.error(errno);
                        return;
                    }
                }
            }
            
            // Remove from mapping
            {
                match safe_lock(&self.parent_name_to_ino, "rmdir_mapping") {
                    Ok(mut parent_name_to_ino) => {
                        parent_name_to_ino.remove(&(parent, name_str));
                    }
                    Err(errno) => {
                        reply.error(errno);
                        return;
                    }
                }
            }
            
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn open(&mut self, _req: &Request, ino: u64, _flags: u32, reply: ReplyOpen) {
        match safe_lock(&self.files, "open") {
            Ok(files) => {
                if files.contains_key(&ino) {
                    reply.opened(0, 0);
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(errno) => reply.error(errno),
        }
    }
}