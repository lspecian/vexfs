use crate::config::{Config, StorageMethod};
use crate::providers::EmbeddingResponse;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEmbedding {
    pub file_path: PathBuf,
    pub embedding: Vec<f32>,
    pub model: String,
    pub provider: String,
    pub file_hash: String,
    pub file_size: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    pub dimension: usize,
    pub model: String,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub file_hash: String,
    pub file_size: u64,
    pub compression: Option<String>,
}

pub struct EmbeddingStorage {
    config: Config,
}

impl EmbeddingStorage {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Store an embedding for a file
    pub async fn store_embedding(
        &self,
        file_path: &Path,
        embedding_response: EmbeddingResponse,
        provider: &str,
    ) -> Result<()> {
        let file_hash = self.calculate_file_hash(file_path).await?;
        let file_size = file_path.metadata()?.len();

        let stored_embedding = StoredEmbedding {
            file_path: file_path.to_path_buf(),
            embedding: embedding_response.embedding,
            model: embedding_response.model,
            provider: provider.to_string(),
            file_hash,
            file_size,
            created_at: chrono::Utc::now(),
            metadata: embedding_response.metadata,
        };

        match self.config.storage.method {
            StorageMethod::Sidecar => {
                self.store_as_sidecar(file_path, &stored_embedding).await
            }
            StorageMethod::ExtendedAttributes => {
                self.store_as_xattr(file_path, &stored_embedding).await
            }
            StorageMethod::VexFSNative => {
                self.store_as_vexfs_native(file_path, &stored_embedding).await
            }
        }
    }

    /// Retrieve an embedding for a file
    pub async fn retrieve_embedding(&self, file_path: &Path) -> Result<Option<StoredEmbedding>> {
        match self.config.storage.method {
            StorageMethod::Sidecar => self.retrieve_from_sidecar(file_path).await,
            StorageMethod::ExtendedAttributes => self.retrieve_from_xattr(file_path).await,
            StorageMethod::VexFSNative => self.retrieve_from_vexfs_native(file_path).await,
        }
    }

    /// Check if an embedding exists and is up-to-date
    pub async fn is_embedding_current(&self, file_path: &Path) -> Result<bool> {
        if let Some(stored) = self.retrieve_embedding(file_path).await? {
            let current_hash = self.calculate_file_hash(file_path).await?;
            let current_size = file_path.metadata()?.len();
            
            Ok(stored.file_hash == current_hash && stored.file_size == current_size)
        } else {
            Ok(false)
        }
    }

    /// Delete an embedding for a file
    pub async fn delete_embedding(&self, file_path: &Path) -> Result<()> {
        match self.config.storage.method {
            StorageMethod::Sidecar => self.delete_sidecar(file_path).await,
            StorageMethod::ExtendedAttributes => self.delete_xattr(file_path).await,
            StorageMethod::VexFSNative => self.delete_vexfs_native(file_path).await,
        }
    }

    /// List all files with embeddings in a directory
    pub async fn list_embeddings(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        match self.config.storage.method {
            StorageMethod::Sidecar => self.list_sidecar_embeddings(dir_path).await,
            StorageMethod::ExtendedAttributes => self.list_xattr_embeddings(dir_path).await,
            StorageMethod::VexFSNative => self.list_vexfs_embeddings(dir_path).await,
        }
    }

    // Sidecar file implementation
    async fn store_as_sidecar(&self, file_path: &Path, embedding: &StoredEmbedding) -> Result<()> {
        let sidecar_path = self.get_sidecar_path(file_path);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = sidecar_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let data = if self.config.storage.compress {
            self.compress_embedding(embedding).await?
        } else {
            serde_json::to_vec_pretty(embedding)
                .context("Failed to serialize embedding")?
        };

        fs::write(&sidecar_path, data)
            .with_context(|| format!("Failed to write sidecar file: {}", sidecar_path.display()))?;

        debug!("Stored embedding as sidecar: {}", sidecar_path.display());
        Ok(())
    }

    async fn retrieve_from_sidecar(&self, file_path: &Path) -> Result<Option<StoredEmbedding>> {
        let sidecar_path = self.get_sidecar_path(file_path);
        
        if !sidecar_path.exists() {
            return Ok(None);
        }

        let data = fs::read(&sidecar_path)
            .with_context(|| format!("Failed to read sidecar file: {}", sidecar_path.display()))?;

        let embedding = if self.config.storage.compress {
            self.decompress_embedding(&data).await?
        } else {
            serde_json::from_slice(&data)
                .context("Failed to deserialize embedding")?
        };

        debug!("Retrieved embedding from sidecar: {}", sidecar_path.display());
        Ok(Some(embedding))
    }

    async fn delete_sidecar(&self, file_path: &Path) -> Result<()> {
        let sidecar_path = self.get_sidecar_path(file_path);
        
        if sidecar_path.exists() {
            fs::remove_file(&sidecar_path)
                .with_context(|| format!("Failed to delete sidecar file: {}", sidecar_path.display()))?;
            debug!("Deleted sidecar file: {}", sidecar_path.display());
        }

        Ok(())
    }

    async fn list_sidecar_embeddings(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let mut embeddings = Vec::new();
        let extension = format!(".{}", self.config.storage.sidecar_extension);

        if !dir_path.exists() {
            return Ok(embeddings);
        }

        let entries = fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory: {}", dir_path.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(&self.config.storage.sidecar_extension) {
                // Get the original file path by removing the sidecar extension
                if let Some(original_name) = path.file_stem().and_then(|s| s.to_str()) {
                    let original_path = dir_path.join(original_name);
                    if original_path.exists() {
                        embeddings.push(original_path);
                    }
                }
            }
        }

        Ok(embeddings)
    }

    // Extended attributes implementation
    async fn store_as_xattr(&self, file_path: &Path, embedding: &StoredEmbedding) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let data = if self.config.storage.compress {
                self.compress_embedding(embedding).await?
            } else {
                serde_json::to_vec(embedding)
                    .context("Failed to serialize embedding")?
            };

            let path_cstr = CString::new(file_path.as_os_str().as_bytes())
                .context("Invalid file path")?;
            let attr_name = CString::new("user.vexfs.embedding")
                .context("Invalid attribute name")?;

            unsafe {
                let result = libc::setxattr(
                    path_cstr.as_ptr(),
                    attr_name.as_ptr(),
                    data.as_ptr() as *const libc::c_void,
                    data.len(),
                    0,
                );

                if result != 0 {
                    let error = std::io::Error::last_os_error();
                    return Err(anyhow::anyhow!("Failed to set extended attribute: {}", error));
                }
            }

            debug!("Stored embedding as xattr: {}", file_path.display());
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            warn!("Extended attributes not supported on this platform, falling back to sidecar");
            self.store_as_sidecar(file_path, embedding).await
        }
    }

    async fn retrieve_from_xattr(&self, file_path: &Path) -> Result<Option<StoredEmbedding>> {
        #[cfg(target_os = "linux")]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let path_cstr = CString::new(file_path.as_os_str().as_bytes())
                .context("Invalid file path")?;
            let attr_name = CString::new("user.vexfs.embedding")
                .context("Invalid attribute name")?;

            unsafe {
                // First, get the size of the attribute
                let size = libc::getxattr(
                    path_cstr.as_ptr(),
                    attr_name.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                );

                if size < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::NotFound {
                        return Ok(None);
                    }
                    return Err(anyhow::anyhow!("Failed to get extended attribute size: {}", error));
                }

                // Now get the actual data
                let mut data = vec![0u8; size as usize];
                let result = libc::getxattr(
                    path_cstr.as_ptr(),
                    attr_name.as_ptr(),
                    data.as_mut_ptr() as *mut libc::c_void,
                    size as usize,
                );

                if result != size {
                    return Err(anyhow::anyhow!("Failed to read extended attribute"));
                }

                let embedding = if self.config.storage.compress {
                    self.decompress_embedding(&data).await?
                } else {
                    serde_json::from_slice(&data)
                        .context("Failed to deserialize embedding")?
                };

                debug!("Retrieved embedding from xattr: {}", file_path.display());
                Ok(Some(embedding))
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            warn!("Extended attributes not supported on this platform, falling back to sidecar");
            self.retrieve_from_sidecar(file_path).await
        }
    }

    async fn delete_xattr(&self, file_path: &Path) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let path_cstr = CString::new(file_path.as_os_str().as_bytes())
                .context("Invalid file path")?;
            let attr_name = CString::new("user.vexfs.embedding")
                .context("Invalid attribute name")?;

            unsafe {
                let result = libc::removexattr(path_cstr.as_ptr(), attr_name.as_ptr());
                if result != 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() != std::io::ErrorKind::NotFound {
                        return Err(anyhow::anyhow!("Failed to remove extended attribute: {}", error));
                    }
                }
            }

            debug!("Deleted xattr embedding: {}", file_path.display());
        }

        #[cfg(not(target_os = "linux"))]
        {
            warn!("Extended attributes not supported on this platform, falling back to sidecar");
            self.delete_sidecar(file_path).await?;
        }

        Ok(())
    }

    async fn list_xattr_embeddings(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        // For xattr, we need to check each file individually
        let mut embeddings = Vec::new();

        if !dir_path.exists() {
            return Ok(embeddings);
        }

        let entries = fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory: {}", dir_path.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if self.retrieve_from_xattr(&path).await?.is_some() {
                    embeddings.push(path);
                }
            }
        }

        Ok(embeddings)
    }

    // VexFS native implementation (placeholder for future VexFS API integration)
    async fn store_as_vexfs_native(&self, file_path: &Path, embedding: &StoredEmbedding) -> Result<()> {
        warn!("VexFS native storage not yet implemented, falling back to sidecar");
        self.store_as_sidecar(file_path, embedding).await
    }

    async fn retrieve_from_vexfs_native(&self, file_path: &Path) -> Result<Option<StoredEmbedding>> {
        warn!("VexFS native storage not yet implemented, falling back to sidecar");
        self.retrieve_from_sidecar(file_path).await
    }

    async fn delete_vexfs_native(&self, file_path: &Path) -> Result<()> {
        warn!("VexFS native storage not yet implemented, falling back to sidecar");
        self.delete_sidecar(file_path).await
    }

    async fn list_vexfs_embeddings(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        warn!("VexFS native storage not yet implemented, falling back to sidecar");
        self.list_sidecar_embeddings(dir_path).await
    }

    // Helper methods
    fn get_sidecar_path(&self, file_path: &Path) -> PathBuf {
        let mut sidecar_path = file_path.to_path_buf();
        let extension = format!(".{}", self.config.storage.sidecar_extension);
        
        if let Some(current_ext) = sidecar_path.extension() {
            let new_ext = format!("{}{}", current_ext.to_string_lossy(), extension);
            sidecar_path.set_extension(new_ext);
        } else {
            let file_name = sidecar_path.file_name()
                .unwrap_or_default()
                .to_string_lossy();
            sidecar_path.set_file_name(format!("{}{}", file_name, extension));
        }

        sidecar_path
    }

    async fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};

        let content = fs::read(file_path)
            .with_context(|| format!("Failed to read file for hashing: {}", file_path.display()))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();

        Ok(hex::encode(hash))
    }

    async fn compress_embedding(&self, embedding: &StoredEmbedding) -> Result<Vec<u8>> {
        // Simple compression using JSON + gzip
        let json_data = serde_json::to_vec(embedding)
            .context("Failed to serialize embedding for compression")?;

        // For now, just return the JSON data
        // TODO: Implement actual compression (e.g., using flate2)
        Ok(json_data)
    }

    async fn decompress_embedding(&self, data: &[u8]) -> Result<StoredEmbedding> {
        // Simple decompression
        // TODO: Implement actual decompression when compression is added
        serde_json::from_slice(data)
            .context("Failed to deserialize compressed embedding")
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self, dir_path: &Path) -> Result<StorageStats> {
        let embeddings = self.list_embeddings(dir_path).await?;
        let mut total_size = 0u64;
        let mut total_files = 0usize;

        for file_path in &embeddings {
            if let Ok(Some(_)) = self.retrieve_embedding(file_path).await {
                total_files += 1;
                
                match self.config.storage.method {
                    StorageMethod::Sidecar => {
                        let sidecar_path = self.get_sidecar_path(file_path);
                        if let Ok(metadata) = sidecar_path.metadata() {
                            total_size += metadata.len();
                        }
                    }
                    _ => {
                        // For xattr and native, size calculation is more complex
                        // For now, estimate based on embedding dimension
                        total_size += 1024; // Rough estimate
                    }
                }
            }
        }

        Ok(StorageStats {
            total_files,
            total_size_bytes: total_size,
            storage_method: self.config.storage.method.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub storage_method: StorageMethod,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_sidecar_path_generation() {
        let config = Config::default();
        let storage = EmbeddingStorage::new(config);
        
        let file_path = Path::new("test.txt");
        let sidecar_path = storage.get_sidecar_path(file_path);
        
        assert_eq!(sidecar_path, PathBuf::from("test.txt.vxvec"));
    }

    #[tokio::test]
    async fn test_file_hash_calculation() {
        let config = Config::default();
        let storage = EmbeddingStorage::new(config);
        
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let hash = storage.calculate_file_hash(&file_path).await.unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex string length
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let config = Config::default();
        let storage = EmbeddingStorage::new(config);
        
        let temp_dir = tempdir().unwrap();
        let stats = storage.get_storage_stats(temp_dir.path()).await.unwrap();
        
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }
}