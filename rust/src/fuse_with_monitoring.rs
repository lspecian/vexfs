// FUSE implementation with integrated monitoring
use fuse::{
    FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, ReplyCreate, ReplyOpen, ReplyEmpty,
};
use libc::{ENOENT, ENOTDIR, EEXIST, EPERM, ENOTEMPTY};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use std::sync::{Arc, Mutex};
use time01::Timespec;

use crate::fuse_error_handling::{safe_lock, log_error};
use crate::monitoring::{MonitoringSystem, OperationMetrics};

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

#[derive(Debug, Clone)]
struct VexFSFile {
    ino: u64,
    name: String,
    parent: u64,
    content: Vec<u8>,
    attr: FileAttr,
    children: Vec<u64>,
}

pub struct MonitoredVexFS {
    files: Arc<Mutex<HashMap<u64, VexFSFile>>>,
    parent_name_to_ino: Arc<Mutex<HashMap<(u64, String), u64>>>,
    next_ino: Arc<Mutex<u64>>,
    monitoring: Arc<MonitoringSystem>,
}

impl MonitoredVexFS {
    pub fn new(monitoring: Arc<MonitoringSystem>) -> Self {
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
            monitoring,
        }
    }
    
    pub fn get_monitoring(&self) -> Arc<MonitoringSystem> {
        self.monitoring.clone()
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
    
    fn record_operation(&self, op_type: &str, start: Instant, success: bool, error: Option<String>) {
        let operation = OperationMetrics {
            operation_type: format!("fuse_{}", op_type),
            start_time: start,
            end_time: Some(Instant::now()),
            success,
            error_message: error,
        };
        
        self.monitoring.record_operation(operation);
    }
    
    fn update_memory_metrics(&self) {
        let file_count = self.files.lock().ok().map(|f| f.len()).unwrap_or(0);
        let estimated_memory = file_count * 1024; // Rough estimate
        
        self.monitoring.update_resource_metrics(
            estimated_memory as u64,
            file_count as u64,
            0, // connections not applicable for FUSE
        );
    }
}

impl Filesystem for MonitoredVexFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let start = Instant::now();
        let name_str = name.to_str().unwrap_or("").to_string();
        
        let result = safe_lock(&self.parent_name_to_ino, "lookup");
        match result {
            Ok(parent_name_to_ino) => {
                if let Some(&ino) = parent_name_to_ino.get(&(parent, name_str.clone())) {
                    let files_result = safe_lock(&self.files, "lookup_files");
                    if let Ok(files) = files_result {
                        if let Some(file) = files.get(&ino) {
                            reply.entry(&TTL, &file.attr, 0);
                            self.record_operation("lookup", start, true, None);
                            return;
                        }
                    }
                }
                reply.error(ENOENT);
                self.record_operation("lookup", start, false, Some("File not found".to_string()));
            }
            Err(errno) => {
                reply.error(errno);
                self.record_operation("lookup", start, false, Some("Lock error".to_string()));
            }
        }
    }
    
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let start = Instant::now();
        
        match safe_lock(&self.files, "getattr") {
            Ok(files) => {
                if let Some(file) = files.get(&ino) {
                    reply.attr(&TTL, &file.attr);
                    self.record_operation("getattr", start, true, None);
                } else {
                    reply.error(ENOENT);
                    self.record_operation("getattr", start, false, Some("Inode not found".to_string()));
                }
            }
            Err(errno) => {
                reply.error(errno);
                self.record_operation("getattr", start, false, Some("Lock error".to_string()));
            }
        }
    }
    
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let start = Instant::now();
        
        match safe_lock(&self.files, "read") {
            Ok(files) => {
                if let Some(file) = files.get(&ino) {
                    let offset = offset as usize;
                    let size = size as usize;
                    
                    if offset < file.content.len() {
                        let end = std::cmp::min(offset + size, file.content.len());
                        reply.data(&file.content[offset..end]);
                        self.record_operation("read", start, true, None);
                    } else {
                        reply.data(&[]);
                        self.record_operation("read", start, true, None);
                    }
                } else {
                    reply.error(ENOENT);
                    self.record_operation("read", start, false, Some("File not found".to_string()));
                }
            }
            Err(errno) => {
                reply.error(errno);
                self.record_operation("read", start, false, Some("Lock error".to_string()));
            }
        }
    }
    
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        let start = Instant::now();
        
        match safe_lock(&self.files, "write") {
            Ok(mut files) => {
                if let Some(file) = files.get_mut(&ino) {
                    let offset = offset as usize;
                    
                    if offset + data.len() > file.content.len() {
                        file.content.resize(offset + data.len(), 0);
                    }
                    
                    file.content[offset..offset + data.len()].copy_from_slice(data);
                    file.attr.size = file.content.len() as u64;
                    file.attr.mtime = SystemTime::now();
                    
                    reply.written(data.len() as u32);
                    self.record_operation("write", start, true, None);
                    
                    // Update memory metrics after write
                    drop(files);
                    self.update_memory_metrics();
                } else {
                    reply.error(ENOENT);
                    self.record_operation("write", start, false, Some("File not found".to_string()));
                }
            }
            Err(errno) => {
                reply.error(errno);
                self.record_operation("write", start, false, Some("Lock error".to_string()));
            }
        }
    }
    
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, _flags: u32, reply: ReplyCreate) {
        let start = Instant::now();
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Check if file already exists
        {
            match safe_lock(&self.parent_name_to_ino, "create_check") {
                Ok(parent_name_to_ino) => {
                    if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                        reply.error(EEXIST);
                        self.record_operation("create", start, false, Some("File exists".to_string()));
                        return;
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    self.record_operation("create", start, false, Some("Lock error".to_string()));
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
                    
                    if let Some(parent_dir) = files.get_mut(&parent) {
                        parent_dir.children.push(ino);
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    self.record_operation("create", start, false, Some("Lock error".to_string()));
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
                    self.record_operation("create", start, false, Some("Lock error".to_string()));
                    return;
                }
            }
        }
        
        reply.created(&TTL, &attr, 0, 0, 0);
        self.record_operation("create", start, true, None);
        self.update_memory_metrics();
    }
    
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        let start = Instant::now();
        let name_str = name.to_str().unwrap_or("").to_string();
        
        // Check if directory already exists
        {
            match safe_lock(&self.parent_name_to_ino, "mkdir_check") {
                Ok(parent_name_to_ino) => {
                    if parent_name_to_ino.contains_key(&(parent, name_str.clone())) {
                        reply.error(EEXIST);
                        self.record_operation("mkdir", start, false, Some("Directory exists".to_string()));
                        return;
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    self.record_operation("mkdir", start, false, Some("Lock error".to_string()));
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
                    
                    if let Some(parent_dir) = files.get_mut(&parent) {
                        parent_dir.children.push(ino);
                    }
                }
                Err(errno) => {
                    reply.error(errno);
                    self.record_operation("mkdir", start, false, Some("Lock error".to_string()));
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
                    self.record_operation("mkdir", start, false, Some("Lock error".to_string()));
                    return;
                }
            }
        }
        
        reply.entry(&TTL, &attr, 0);
        self.record_operation("mkdir", start, true, None);
        self.update_memory_metrics();
    }
    
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let start = Instant::now();
        
        match safe_lock(&self.files, "readdir") {
            Ok(files) => {
                if let Some(dir) = files.get(&ino) {
                    if dir.attr.kind != FileType::Directory {
                        reply.error(ENOTDIR);
                        self.record_operation("readdir", start, false, Some("Not a directory".to_string()));
                        return;
                    }
                    
                    let mut entries = vec![
                        (ino, FileType::Directory, "."),
                        (dir.parent, FileType::Directory, ".."),
                    ];
                    
                    for &child_ino in &dir.children {
                        if let Some(child) = files.get(&child_ino) {
                            entries.push((child_ino, child.attr.kind, &child.name));
                        }
                    }
                    
                    for (i, entry) in entries.iter().enumerate().skip(offset as usize) {
                        if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                            break;
                        }
                    }
                    
                    reply.ok();
                    self.record_operation("readdir", start, true, None);
                } else {
                    reply.error(ENOENT);
                    self.record_operation("readdir", start, false, Some("Directory not found".to_string()));
                }
            }
            Err(errno) => {
                reply.error(errno);
                self.record_operation("readdir", start, false, Some("Lock error".to_string()));
            }
        }
    }
    
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let start = Instant::now();
        let name_str = name.to_str().unwrap_or("").to_string();
        
        let ino_to_remove = {
            match safe_lock(&self.parent_name_to_ino, "unlink_lookup") {
                Ok(parent_name_to_ino) => {
                    parent_name_to_ino.get(&(parent, name_str.clone())).copied()
                }
                Err(errno) => {
                    reply.error(errno);
                    self.record_operation("unlink", start, false, Some("Lock error".to_string()));
                    return;
                }
            }
        };
        
        if let Some(ino) = ino_to_remove {
            {
                match safe_lock(&self.files, "unlink_remove") {
                    Ok(mut files) => {
                        if let Some(file) = files.get(&ino) {
                            if file.attr.kind == FileType::Directory {
                                reply.error(EPERM);
                                self.record_operation("unlink", start, false, Some("Is a directory".to_string()));
                                return;
                            }
                        }
                        
                        if let Some(parent_dir) = files.get_mut(&parent) {
                            parent_dir.children.retain(|&x| x != ino);
                        }
                        
                        files.remove(&ino);
                    }
                    Err(errno) => {
                        reply.error(errno);
                        self.record_operation("unlink", start, false, Some("Lock error".to_string()));
                        return;
                    }
                }
            }
            
            {
                match safe_lock(&self.parent_name_to_ino, "unlink_mapping") {
                    Ok(mut parent_name_to_ino) => {
                        parent_name_to_ino.remove(&(parent, name_str));
                    }
                    Err(errno) => {
                        reply.error(errno);
                        self.record_operation("unlink", start, false, Some("Lock error".to_string()));
                        return;
                    }
                }
            }
            
            reply.ok();
            self.record_operation("unlink", start, true, None);
            self.update_memory_metrics();
        } else {
            reply.error(ENOENT);
            self.record_operation("unlink", start, false, Some("File not found".to_string()));
        }
    }
    
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let start = Instant::now();
        let name_str = name.to_str().unwrap_or("").to_string();
        
        let ino_to_remove = {
            match safe_lock(&self.parent_name_to_ino, "rmdir_lookup") {
                Ok(parent_name_to_ino) => {
                    parent_name_to_ino.get(&(parent, name_str.clone())).copied()
                }
                Err(errno) => {
                    reply.error(errno);
                    self.record_operation("rmdir", start, false, Some("Lock error".to_string()));
                    return;
                }
            }
        };
        
        if let Some(ino) = ino_to_remove {
            {
                match safe_lock(&self.files, "rmdir_remove") {
                    Ok(mut files) => {
                        if let Some(dir) = files.get(&ino) {
                            if dir.attr.kind != FileType::Directory {
                                reply.error(ENOTDIR);
                                self.record_operation("rmdir", start, false, Some("Not a directory".to_string()));
                                return;
                            }
                            if !dir.children.is_empty() {
                                reply.error(ENOTEMPTY);
                                self.record_operation("rmdir", start, false, Some("Directory not empty".to_string()));
                                return;
                            }
                        }
                        
                        if let Some(parent_dir) = files.get_mut(&parent) {
                            parent_dir.children.retain(|&x| x != ino);
                        }
                        
                        files.remove(&ino);
                    }
                    Err(errno) => {
                        reply.error(errno);
                        self.record_operation("rmdir", start, false, Some("Lock error".to_string()));
                        return;
                    }
                }
            }
            
            {
                match safe_lock(&self.parent_name_to_ino, "rmdir_mapping") {
                    Ok(mut parent_name_to_ino) => {
                        parent_name_to_ino.remove(&(parent, name_str));
                    }
                    Err(errno) => {
                        reply.error(errno);
                        self.record_operation("rmdir", start, false, Some("Lock error".to_string()));
                        return;
                    }
                }
            }
            
            reply.ok();
            self.record_operation("rmdir", start, true, None);
            self.update_memory_metrics();
        } else {
            reply.error(ENOENT);
            self.record_operation("rmdir", start, false, Some("Directory not found".to_string()));
        }
    }
    
    fn open(&mut self, _req: &Request, ino: u64, _flags: u32, reply: ReplyOpen) {
        let start = Instant::now();
        
        match safe_lock(&self.files, "open") {
            Ok(files) => {
                if files.contains_key(&ino) {
                    reply.opened(0, 0);
                    self.record_operation("open", start, true, None);
                } else {
                    reply.error(ENOENT);
                    self.record_operation("open", start, false, Some("File not found".to_string()));
                }
            }
            Err(errno) => {
                reply.error(errno);
                self.record_operation("open", start, false, Some("Lock error".to_string()));
            }
        }
    }
}