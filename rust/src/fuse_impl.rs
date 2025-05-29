use fuse::{
    FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyWrite, ReplyCreate, ReplyOpen,
};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};
use std::sync::{Arc, Mutex};

use crate::{VexfsResult, VexfsError};
use crate::vector_storage::VectorStorage;
use crate::vector_search::VectorSearchEngine;

const TTL: Duration = Duration::from_secs(1);

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
    vector_storage: Arc<Mutex<VectorStorage>>,
    search_engine: Arc<Mutex<VectorSearchEngine>>,
}

impl VexFSFuse {
    pub fn new() -> VexfsResult<Self> {
        let mut files = HashMap::new();
        let mut name_to_ino = HashMap::new();
        
        // Create root directory
        let root_attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
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
        
        Ok(VexFSFuse {
            files: Arc::new(Mutex::new(files)),
            name_to_ino: Arc::new(Mutex::new(name_to_ino)),
            next_ino: Arc::new(Mutex::new(2)),
            vector_storage: Arc::new(Mutex::new(VectorStorage::new()?)),
            search_engine: Arc::new(Mutex::new(VectorSearchEngine::new()?)),
        })
    }
    
    fn get_next_ino(&self) -> u64 {
        let mut next_ino = self.next_ino.lock().unwrap();
        let ino = *next_ino;
        *next_ino += 1;
        ino
    }
    
    fn create_file_attr(ino: u64, size: u64, kind: FileType) -> FileAttr {
        let now = std::time::SystemTime::now();
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
            blksize: 512,
        }
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
            file.attr.mtime = std::time::SystemTime::now();
            
            // Try to extract vector from content if it's a .vec file
            if file.name.ends_with(".vec") {
                if let Ok(content_str) = String::from_utf8(file.content.clone()) {
                    if let Ok(vector) = self.parse_vector(&content_str) {
                        file.vector = Some(vector.clone());
                        
                        // Store in vector storage
                        if let Ok(mut storage) = self.vector_storage.lock() {
                            let _ = storage.store_vector(&file.ino.to_string(), &vector);
                        }
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
}

impl VexFSFuse {
    fn parse_vector(&self, content: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        content
            .trim()
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect::<Result<Vec<f32>, _>>()
            .map_err(|e| e.into())
    }
    
    pub fn search_vectors(&self, query_vector: &[f32], top_k: usize) -> VexfsResult<Vec<String>> {
        let search_engine = self.search_engine.lock().unwrap();
        search_engine.search(query_vector, top_k)
    }
}