use fuse::{
    FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, ReplyCreate, ReplyOpen, ReplyEmpty, ReplyStatfs,
};
use libc::{ENOENT, ENOSYS, ENOTDIR, EEXIST, EINVAL, EIO, EACCES, EPERM};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use time01::Timespec;

use crate::shared::errors::{VexfsError, VexfsResult};

// FUSE 0.3 uses time::Timespec from time crate v0.1.45
// We import Timespec directly from the fuse crate to avoid version conflicts

// Simple structs for FUSE context
#[derive(Debug, Clone)]
struct User {
    uid: u32,
    gid: u32,
}

#[derive(Debug, Clone)]
struct Process {
    pid: u32,
    name: String,
}

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

#[derive(Debug, Clone)]
struct VexFSFile {
    ino: u64,
    name: String,
    content: Vec<u8>,
    metadata: HashMap<String, String>,
    vector: Option<Vec<f32>>,
    attr: FileAttr,
}

pub struct VexFSFuse {
    files: Arc<Mutex<HashMap<u64, VexFSFile>>>,
    name_to_ino: Arc<Mutex<HashMap<String, u64>>>,
    next_ino: Arc<Mutex<u64>>,
    // Temporarily remove VexFS components to isolate the stack overflow issue
    // vector_storage: Arc<Mutex<VectorStorageManager>>,
    // search_engine: Arc<Mutex<VectorSearchEngine>>,
}

impl VexFSFuse {
    pub fn new() -> VexfsResult<Self> {
        let mut files = HashMap::new();
        let mut name_to_ino = HashMap::new();
        
        // Create root directory
        let now = system_time_to_timespec(SystemTime::now());
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
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        };
        
        let root_file = VexFSFile {
            ino: 1,
            name: "/".to_string(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr: root_attr,
        };
        
        files.insert(1, root_file);
        name_to_ino.insert("/".to_string(), 1);
        
        // MINIMAL INITIALIZATION - No VexFS components to avoid stack overflow
        eprintln!("VexFSFuse: Minimal initialization complete");
        
        Ok(VexFSFuse {
            files: Arc::new(Mutex::new(files)),
            name_to_ino: Arc::new(Mutex::new(name_to_ino)),
            next_ino: Arc::new(Mutex::new(2)),
            // Temporarily commented out to isolate stack overflow
            // vector_storage: Arc::new(Mutex::new(vector_storage)),
            // search_engine: Arc::new(Mutex::new(search_engine)),
        })
    }
    
    fn get_next_ino(&self) -> u64 {
        let mut next_ino = self.next_ino.lock().unwrap();
        let ino = *next_ino;
        *next_ino += 1;
        ino
    }
    
    fn create_file_attr(ino: u64, size: u64, kind: FileType) -> FileAttr {
        let now = system_time_to_timespec(SystemTime::now());
        FileAttr {
            ino,
            size,
            blocks: (size + 511) / 512,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind,
            perm: if kind == FileType::Directory { 0o755 } else { 0o644 },
            nlink: if kind == FileType::Directory { 2 } else { 1 },
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        }
    }
}

// Helper function to convert SystemTime to Timespec for FUSE compatibility
fn system_time_to_timespec(time: SystemTime) -> Timespec {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => Timespec {
            sec: duration.as_secs() as i64,
            nsec: duration.subsec_nanos() as i32,
        },
        Err(_) => Timespec { sec: 0, nsec: 0 }, // Fallback for times before UNIX_EPOCH
    }
}

impl Filesystem for VexFSFuse {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let files = self.files.lock().unwrap();
        let name_str = name.to_string_lossy().to_string();
        
        // Look for file in parent directory
        for file in files.values() {
            if file.name == name_str {
                reply.entry(&TTL, &file.attr, 0);
                return;
            }
        }
        
        reply.error(ENOENT);
    }
    
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let files = self.files.lock().unwrap();
        
        if let Some(file) = files.get(&ino) {
            reply.attr(&TTL, &file.attr);
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let files = self.files.lock().unwrap();
        
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
    
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        let mut files = self.files.lock().unwrap();
        
        if let Some(file) = files.get_mut(&ino) {
            let offset = offset as usize;
            
            // Extend content if necessary
            if offset + data.len() > file.content.len() {
                file.content.resize(offset + data.len(), 0);
            }
            
            // Write data
            file.content[offset..offset + data.len()].copy_from_slice(data);
            
            // Update file attributes
            file.attr.size = file.content.len() as u64;
            file.attr.mtime = system_time_to_timespec(SystemTime::now());
            
            // Try to extract vector from content if it's a .vec file (simplified)
            if file.name.ends_with(".vec") {
                if let Ok(content_str) = String::from_utf8(file.content.clone()) {
                    if let Ok(vector) = self.parse_vector(&content_str) {
                        file.vector = Some(vector.clone());
                        eprintln!("Vector parsed successfully for file {}: {} dimensions", file.name, vector.len());
                    }
                }
            }
            
            reply.written(data.len() as u32);
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, _flags: u32, reply: ReplyCreate) {
        let name_str = name.to_string_lossy().to_string();
        let ino = self.get_next_ino();
        
        let attr = Self::create_file_attr(ino, 0, FileType::RegularFile);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
        };
        
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
        }
        
        {
            let mut name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.insert(name_str, ino);
        }
        
        reply.created(&TTL, &attr, 0, 0, 0);
    }
    
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let files = self.files.lock().unwrap();
        
        if ino == 1 {
            // Root directory
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");
                
                let mut entry_offset = 2;
                for file in files.values() {
                    if file.ino != 1 {
                        reply.add(file.ino, entry_offset, file.attr.kind, &file.name);
                        entry_offset += 1;
                    }
                }
            }
        }
        
        reply.ok();
    }
    
    fn setattr(&mut self, _req: &Request, ino: u64, mode: Option<u32>, uid: Option<u32>,
               gid: Option<u32>, size: Option<u64>, atime: Option<Timespec>,
               mtime: Option<Timespec>, _fh: Option<u64>, crtime: Option<Timespec>,
               _chgtime: Option<Timespec>, _bkuptime: Option<Timespec>,
               flags: Option<u32>, reply: ReplyAttr) {
        let mut files = self.files.lock().unwrap();
        
        if let Some(file) = files.get_mut(&ino) {
            // Update file attributes
            if let Some(mode) = mode {
                file.attr.perm = mode as u16;
            }
            if let Some(uid) = uid {
                file.attr.uid = uid;
            }
            if let Some(gid) = gid {
                file.attr.gid = gid;
            }
            if let Some(size) = size {
                file.attr.size = size;
                file.content.resize(size as usize, 0);
            }
            if let Some(atime) = atime {
                file.attr.atime = atime;
            }
            if let Some(mtime) = mtime {
                file.attr.mtime = mtime;
            }
            if let Some(crtime) = crtime {
                file.attr.crtime = crtime;
            }
            if let Some(flags) = flags {
                file.attr.flags = flags;
            }
            
            reply.attr(&TTL, &file.attr);
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn mknod(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32,
             _rdev: u32, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        let ino = self.get_next_ino();
        
        // Determine file type from mode
        let file_type = if mode & libc::S_IFDIR != 0 {
            FileType::Directory
        } else {
            FileType::RegularFile
        };
        
        let attr = Self::create_file_attr(ino, 0, file_type);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
        };
        
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
        }
        
        {
            let mut name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.insert(name_str, ino);
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        let ino = self.get_next_ino();
        
        let attr = Self::create_file_attr(ino, 0, FileType::Directory);
        
        let file = VexFSFile {
            ino,
            name: name_str.clone(),
            content: Vec::new(),
            metadata: HashMap::new(),
            vector: None,
            attr,
        };
        
        {
            let mut files = self.files.lock().unwrap();
            files.insert(ino, file);
        }
        
        {
            let mut name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.insert(name_str, ino);
        }
        
        reply.entry(&TTL, &attr, 0);
    }
    
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_string_lossy().to_string();
        
        let ino_to_remove = {
            let name_to_ino = self.name_to_ino.lock().unwrap();
            name_to_ino.get(&name_str).copied()
        };
        
        if let Some(ino) = ino_to_remove {
            {
                let mut files = self.files.lock().unwrap();
                files.remove(&ino);
            }
            
            {
                let mut name_to_ino = self.name_to_ino.lock().unwrap();
                name_to_ino.remove(&name_str);
            }
            
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        // For simplicity, treat rmdir the same as unlink
        self.unlink(_req, parent, name, reply);
    }
    
    fn open(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        let files = self.files.lock().unwrap();
        
        if files.contains_key(&ino) {
            reply.opened(0, 0); // fh=0, flags=0
        } else {
            reply.error(ENOENT);
        }
    }
    
    fn flush(&mut self, _req: &Request, ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        // For a simple implementation, just return success
        reply.ok();
    }
    
    fn release(&mut self, _req: &Request, ino: u64, _fh: u64, _flags: u32, _lock_owner: u64,
               _flush: bool, reply: ReplyEmpty) {
        // For a simple implementation, just return success
        reply.ok();
    }
}

impl VexFSFuse {
    fn parse_vector(&self, content: &str) -> std::result::Result<Vec<f32>, Box<dyn std::error::Error>> {
        content
            .trim()
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect::<std::result::Result<Vec<f32>, _>>()
            .map_err(|e| e.into())
    }
    
    pub fn search_vectors(&self, query_vector: &[f32], top_k: usize) -> VexfsResult<Vec<String>> {
        // Minimal implementation without VexFS components
        eprintln!("Vector search requested: {} dimensions, top_k={}", query_vector.len(), top_k);
        
        // Return files with vectors
        let files = self.files.lock().unwrap();
        let file_paths: Vec<String> = files.values()
            .filter(|file| file.vector.is_some())
            .take(top_k)
            .map(|file| file.name.clone())
            .collect();
        
        Ok(file_paths)
    }
}