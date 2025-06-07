/*
 * VexFS Mock Client for Testing
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

//! Mock client for testing vexctl without a real VexFS filesystem

use crate::client::connection::{FilesystemInfo, VexfsConnection};
use crate::client::ioctl::FilesystemStatus;
use crate::client::ipc::{EmbeddingService, IpcStatistics, ServiceStatus};
use crate::{Result, VexctlError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Mock data directory path
const MOCK_DATA_DIR: &str = "/tmp/vexfs_mock_data";

/// Mock VexFS connection for testing
pub struct MockVexfsConnection {
    mount_path: PathBuf,
}

impl MockVexfsConnection {
    /// Create a new mock connection
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            mount_path: path.as_ref().to_path_buf(),
        }
    }

    /// Get mock filesystem information
    pub fn filesystem_info(&self) -> Result<FilesystemInfo> {
        let metadata_path = format!("{}/metadata/filesystem_info.json", MOCK_DATA_DIR);
        
        if let Ok(content) = std::fs::read_to_string(&metadata_path) {
            if let Ok(mock_info) = serde_json::from_str::<MockFilesystemInfo>(&content) {
                return Ok(FilesystemInfo {
                    mount_path: self.mount_path.clone(),
                    total_space: mock_info.total_space,
                    free_space: mock_info.free_space,
                    used_space: mock_info.used_space,
                    total_inodes: mock_info.total_inodes,
                    free_inodes: mock_info.free_inodes,
                    block_size: mock_info.block_size,
                    is_read_only: mock_info.is_read_only,
                    filesystem_type: mock_info.filesystem_type,
                });
            }
        }

        // Fallback to default mock data
        Ok(FilesystemInfo {
            mount_path: self.mount_path.clone(),
            total_space: 1073741824,      // 1GB
            free_space: 968884224,        // ~900MB
            used_space: 104857600,        // ~100MB
            total_inodes: 65536,
            free_inodes: 65530,
            block_size: 4096,
            is_read_only: false,
            filesystem_type: "vexfs".to_string(),
        })
    }

    /// Get mock VexFS status
    pub fn get_status(&self) -> Result<FilesystemStatus> {
        Ok(FilesystemStatus {
            version: "0.1.0-mock".to_string(),
            magic_number: 0x56455846, // "VEXF"
            is_healthy: true,
            total_vectors: 1000,
            total_indexes: 2,
            memory_usage: 256 * 1024 * 1024, // 256MB
            disk_usage: 512 * 1024 * 1024,   // 512MB
            uptime_seconds: 3600, // 1 hour
        })
    }

    /// Get mock embedding services
    pub fn list_services(&self) -> Result<Vec<EmbeddingService>> {
        Ok(vec![
            EmbeddingService {
                id: "embedding-service-1".to_string(),
                name: "Text Embedding Service".to_string(),
                status: ServiceStatus::Active,
                health_score: 95,
                current_load: 25,
                active_requests: 3,
                total_requests: 1500,
                failed_requests: 12,
                avg_response_time_ms: 45,
                last_heartbeat: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
            EmbeddingService {
                id: "embedding-service-2".to_string(),
                name: "Image Embedding Service".to_string(),
                status: ServiceStatus::Active,
                health_score: 88,
                current_load: 60,
                active_requests: 8,
                total_requests: 800,
                failed_requests: 5,
                avg_response_time_ms: 120,
                last_heartbeat: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
        ])
    }

    /// Get mock IPC statistics
    pub fn get_statistics(&self) -> Result<IpcStatistics> {
        Ok(IpcStatistics {
            total_requests: 2300,
            successful_requests: 2283,
            failed_requests: 17,
            avg_response_time_us: 75000, // 75ms
            active_services: 2,
            queued_requests: 5,
        })
    }

    /// Mock vector search
    pub fn search_vectors(&self, query: &[f32], top_k: u32) -> Result<Vec<MockSearchResult>> {
        // Load mock vectors if available
        let vectors_path = format!("{}/vectors/sample_vectors.json", MOCK_DATA_DIR);
        
        if let Ok(content) = std::fs::read_to_string(&vectors_path) {
            if let Ok(mock_data) = serde_json::from_str::<MockVectorData>(&content) {
                // Simple mock search - return first top_k vectors with mock scores
                let results = mock_data.vectors
                    .into_iter()
                    .take(top_k as usize)
                    .enumerate()
                    .map(|(i, vector)| MockSearchResult {
                        id: vector.id,
                        score: 0.95 - (i as f32 * 0.1), // Decreasing scores
                        metadata: vector.metadata,
                    })
                    .collect();
                
                return Ok(results);
            }
        }

        // Fallback mock results
        Ok(vec![
            MockSearchResult {
                id: 1,
                score: 0.95,
                metadata: serde_json::json!({
                    "filename": "document1.txt",
                    "type": "text_embedding"
                }),
            },
            MockSearchResult {
                id: 2,
                score: 0.87,
                metadata: serde_json::json!({
                    "filename": "document2.txt", 
                    "type": "text_embedding"
                }),
            },
        ])
    }

    /// Mock index listing
    pub fn list_indexes(&self) -> Result<Vec<MockIndexInfo>> {
        let indexes_path = format!("{}/indexes/default_index.json", MOCK_DATA_DIR);
        
        if let Ok(content) = std::fs::read_to_string(&indexes_path) {
            if let Ok(index_info) = serde_json::from_str::<MockIndexInfo>(&content) {
                return Ok(vec![index_info]);
            }
        }

        // Fallback mock indexes
        Ok(vec![
            MockIndexInfo {
                name: "default".to_string(),
                index_type: "HNSW".to_string(),
                dimensions: 128,
                vector_count: 1000,
                memory_usage: 256 * 1024 * 1024,
                disk_usage: 512 * 1024 * 1024,
                health_score: 95,
                avg_search_time_ms: 15,
            },
            MockIndexInfo {
                name: "text-embeddings".to_string(),
                index_type: "IVF".to_string(),
                dimensions: 768,
                vector_count: 5000,
                memory_usage: 128 * 1024 * 1024,
                disk_usage: 1024 * 1024 * 1024,
                health_score: 88,
                avg_search_time_ms: 25,
            },
        ])
    }
}

// Mock data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockFilesystemInfo {
    filesystem_type: String,
    total_space: u64,
    used_space: u64,
    free_space: u64,
    total_inodes: u64,
    free_inodes: u64,
    block_size: u32,
    is_read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockVectorData {
    vectors: Vec<MockVector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockVector {
    id: u64,
    dimensions: u32,
    data: Vec<f32>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSearchResult {
    pub id: u64,
    pub score: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockIndexInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub index_type: String,
    pub dimensions: u32,
    pub vector_count: u64,
    pub memory_usage: u64,
    pub disk_usage: u64,
    pub health_score: u8,
    pub avg_search_time_ms: u64,
}

/// Check if mock mode should be used and path exists
pub fn should_use_mock_mode(mock_enabled: bool, path: &Path) -> bool {
    if !mock_enabled {
        return false;
    }

    // Create test directory if it doesn't exist
    if !path.exists() {
        if let Err(e) = std::fs::create_dir_all(path) {
            eprintln!("Warning: Could not create test directory {}: {}", path.display(), e);
            return false;
        }
    }

    // Create mock data directory if it doesn't exist
    let mock_data_path = Path::new(MOCK_DATA_DIR);
    if !mock_data_path.exists() {
        if let Err(e) = std::fs::create_dir_all(mock_data_path) {
            eprintln!("Warning: Could not create mock data directory: {}", e);
        }
    }

    true
}

/// Parse query vector from string
pub fn parse_query_vector(query: &str) -> Result<Vec<f32>> {
    query
        .split(',')
        .map(|s| {
            s.trim().parse::<f32>().map_err(|e| {
                VexctlError::ParseError {
                    input: s.to_string(),
                    reason: format!("Invalid float: {}", e),
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_connection() {
        let mock = MockVexfsConnection::new("/tmp/test");
        let fs_info = mock.filesystem_info().unwrap();
        assert_eq!(fs_info.filesystem_type, "vexfs");
        assert!(fs_info.total_space > 0);
    }

    #[test]
    fn test_parse_query_vector() {
        let query = "1.0,2.5,3.14,-0.5";
        let vector = parse_query_vector(query).unwrap();
        assert_eq!(vector, vec![1.0, 2.5, 3.14, -0.5]);
        
        // Test error case
        let invalid_query = "1.0,invalid,3.14";
        assert!(parse_query_vector(invalid_query).is_err());
    }
}
