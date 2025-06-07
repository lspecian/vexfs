/*
 * VexFS Status Command
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

//! Status command implementation

use crate::client::{VexfsConnection, IoctlClient, IpcClient};
use crate::commands::{Command, CommandConfig};
use crate::output::{Formatter};
use crate::output::formatter::{Status, StatusFormat};
use crate::{Result, VexctlError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tabled::Tabled;

/// Status command for displaying VexFS filesystem information
pub struct StatusCommand {
    config: CommandConfig,
    mount_point: PathBuf,
}

impl StatusCommand {
    /// Create a new status command
    pub fn new(config: CommandConfig, mount_point: PathBuf) -> Self {
        Self { config, mount_point }
    }
}

impl Command for StatusCommand {
    fn execute(&self) -> Result<()> {
        let formatter = Formatter::new(self.config.format);
        
        if !self.config.quiet {
            println!("{}", formatter.header("VexFS Status Report"));
        }

        // Connect to the filesystem
        let connection = VexfsConnection::connect(&self.mount_point)?;
        
        // Gather status information
        let status_info = self.gather_status_info(&connection)?;
        
        // Display the status
        self.display_status(&formatter, &status_info)?;
        
        Ok(())
    }

    fn name(&self) -> &'static str {
        "status"
    }

    fn description(&self) -> &'static str {
        "Display VexFS filesystem status and health information"
    }
}

impl StatusCommand {
    /// Gather comprehensive status information
    fn gather_status_info(&self, connection: &VexfsConnection) -> Result<StatusInfo> {
        let ioctl_client = IoctlClient::new(connection);
        let ipc_client = IpcClient::new(connection);
        
        // Get filesystem information
        let fs_info = connection.filesystem_info()?;
        
        // Get VexFS-specific status
        let vexfs_status = ioctl_client.get_status()?;
        
        // Get vector index information
        let index_info = self.get_index_info(&ioctl_client)?;
        
        // Get embedding services information
        let embedding_services = ipc_client.list_services().unwrap_or_default();
        
        // Get IPC statistics
        let ipc_stats = ipc_client.get_statistics().unwrap_or_default();
        
        // Calculate overall health
        let health = self.calculate_health(&fs_info, &vexfs_status, &index_info, &embedding_services);
        
        Ok(StatusInfo {
            filesystem: fs_info,
            vexfs_status,
            index_info,
            embedding_services,
            ipc_stats,
            health,
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }
    
    /// Get vector index information
    fn get_index_info(&self, ioctl_client: &IoctlClient) -> Result<Vec<IndexInfo>> {
        // TODO: Implement actual index enumeration
        // For now, return mock data
        Ok(vec![
            IndexInfo {
                name: "default".to_string(),
                index_type: "HNSW".to_string(),
                dimensions: 1536,
                vector_count: 10000,
                memory_usage: 256 * 1024 * 1024, // 256MB
                disk_usage: 512 * 1024 * 1024,   // 512MB
                health_score: 95,
                avg_search_time_ms: 15,
                last_optimized: chrono::Utc::now().timestamp() as u64 - 86400, // 1 day ago
            },
            IndexInfo {
                name: "text-embeddings".to_string(),
                index_type: "IVF".to_string(),
                dimensions: 768,
                vector_count: 50000,
                memory_usage: 128 * 1024 * 1024, // 128MB
                disk_usage: 1024 * 1024 * 1024,  // 1GB
                health_score: 88,
                avg_search_time_ms: 25,
                last_optimized: chrono::Utc::now().timestamp() as u64 - 172800, // 2 days ago
            },
        ])
    }
    
    /// Calculate overall system health
    fn calculate_health(
        &self,
        fs_info: &crate::client::connection::FilesystemInfo,
        vexfs_status: &crate::client::ioctl::FilesystemStatus,
        index_info: &[IndexInfo],
        embedding_services: &[crate::client::ipc::EmbeddingService],
    ) -> HealthInfo {
        let mut health_score = 100u8;
        let mut issues = Vec::new();
        
        // Check filesystem health
        if !vexfs_status.is_healthy {
            health_score = health_score.saturating_sub(30);
            issues.push("VexFS kernel module reports unhealthy state".to_string());
        }
        
        // Check space utilization
        let space_util = fs_info.space_utilization();
        if space_util > 90.0 {
            health_score = health_score.saturating_sub(20);
            issues.push(format!("High disk usage: {:.1}%", space_util));
        } else if space_util > 80.0 {
            health_score = health_score.saturating_sub(10);
            issues.push(format!("Moderate disk usage: {:.1}%", space_util));
        }
        
        // Check inode utilization
        let inode_util = fs_info.inode_utilization();
        if inode_util > 90.0 {
            health_score = health_score.saturating_sub(15);
            issues.push(format!("High inode usage: {:.1}%", inode_util));
        }
        
        // Check index health
        let unhealthy_indexes = index_info.iter()
            .filter(|idx| idx.health_score < 80)
            .count();
        if unhealthy_indexes > 0 {
            health_score = health_score.saturating_sub(15);
            issues.push(format!("{} indexes need attention", unhealthy_indexes));
        }
        
        // Check embedding services
        let inactive_services = embedding_services.iter()
            .filter(|svc| svc.status != crate::client::ipc::ServiceStatus::Active)
            .count();
        if inactive_services > 0 {
            health_score = health_score.saturating_sub(10);
            issues.push(format!("{} embedding services inactive", inactive_services));
        }
        
        let status = if health_score >= 90 {
            Status::Ok
        } else if health_score >= 70 {
            Status::Warning
        } else {
            Status::Error
        };
        
        HealthInfo {
            status,
            score: health_score,
            issues,
        }
    }
    
    /// Display status information
    fn display_status(&self, formatter: &Formatter, status: &StatusInfo) -> Result<()> {
        match self.config.format {
            crate::output::OutputFormat::Json => {
                println!("{}", formatter.format(status)?);
            }
            crate::output::OutputFormat::Table => {
                self.display_table_format(formatter, status)?;
            }
            crate::output::OutputFormat::Human => {
                self.display_human_format(formatter, status)?;
            }
            crate::output::OutputFormat::Compact => {
                self.display_compact_format(formatter, status)?;
            }
        }
        Ok(())
    }
    
    /// Display in human-readable format
    fn display_human_format(&self, formatter: &Formatter, status: &StatusInfo) -> Result<()> {
        // Overall health
        println!("\n{}", formatter.subheader("Overall Health"));
        println!("{}", formatter.key_value("Status", &status.health.status.format_status(formatter)));
        println!("{}", formatter.key_value("Health Score", &format!("{}/100", status.health.score)));
        
        if !status.health.issues.is_empty() {
            println!("\n{}", formatter.subheader("Issues"));
            for issue in &status.health.issues {
                println!("  {}", formatter.warning(issue));
            }
        }
        
        // Filesystem information
        println!("\n{}", formatter.subheader("Filesystem"));
        println!("{}", formatter.key_value("Mount Point", &status.filesystem.mount_path.display().to_string()));
        println!("{}", formatter.key_value("Type", &status.filesystem.filesystem_type));
        println!("{}", formatter.key_value("Total Space", &formatter.file_size(status.filesystem.total_space)));
        println!("{}", formatter.key_value("Used Space", &formatter.file_size(status.filesystem.used_space)));
        println!("{}", formatter.key_value("Free Space", &formatter.file_size(status.filesystem.free_space)));
        println!("{}", formatter.key_value("Space Utilization", &formatter.percentage(status.filesystem.space_utilization())));
        println!("{}", formatter.key_value("Total Inodes", &status.filesystem.total_inodes.to_string()));
        println!("{}", formatter.key_value("Free Inodes", &status.filesystem.free_inodes.to_string()));
        println!("{}", formatter.key_value("Inode Utilization", &formatter.percentage(status.filesystem.inode_utilization())));
        
        // VexFS status
        println!("\n{}", formatter.subheader("VexFS Status"));
        println!("{}", formatter.key_value("Version", &status.vexfs_status.version));
        println!("{}", formatter.key_value("Healthy", &status.vexfs_status.is_healthy.to_string()));
        println!("{}", formatter.key_value("Magic Number", &format!("0x{:X}", status.vexfs_status.magic_number)));
        
        // Vector indexes
        if !status.index_info.is_empty() {
            println!("\n{}", formatter.subheader("Vector Indexes"));
            for index in &status.index_info {
                println!("\n  {}", formatter.key_value("Name", &index.name));
                println!("  {}", formatter.key_value("Type", &index.index_type));
                println!("  {}", formatter.key_value("Dimensions", &index.dimensions.to_string()));
                println!("  {}", formatter.key_value("Vectors", &index.vector_count.to_string()));
                println!("  {}", formatter.key_value("Memory Usage", &formatter.file_size(index.memory_usage)));
                println!("  {}", formatter.key_value("Disk Usage", &formatter.file_size(index.disk_usage)));
                println!("  {}", formatter.key_value("Health Score", &formatter.percentage(index.health_score as f64)));
                println!("  {}", formatter.key_value("Avg Search Time", &format!("{}ms", index.avg_search_time_ms)));
                println!("  {}", formatter.key_value("Last Optimized", &formatter.timestamp(index.last_optimized)));
            }
        }
        
        // Embedding services
        if !status.embedding_services.is_empty() {
            println!("\n{}", formatter.subheader("Embedding Services"));
            for service in &status.embedding_services {
                println!("\n  {}", formatter.key_value("ID", &service.id));
                println!("  {}", formatter.key_value("Name", &service.name));
                println!("  {}", formatter.key_value("Status", &crate::client::ipc::format_service_status(service.status)));
                println!("  {}", formatter.key_value("Health Score", &formatter.percentage(service.health_score as f64)));
                println!("  {}", formatter.key_value("Load", &formatter.percentage(service.current_load as f64)));
                println!("  {}", formatter.key_value("Active Requests", &service.active_requests.to_string()));
                println!("  {}", formatter.key_value("Total Requests", &service.total_requests.to_string()));
                println!("  {}", formatter.key_value("Failed Requests", &service.failed_requests.to_string()));
                println!("  {}", formatter.key_value("Avg Response Time", &format!("{}ms", service.avg_response_time_ms)));
            }
        }
        
        // IPC statistics
        println!("\n{}", formatter.subheader("IPC Statistics"));
        println!("{}", formatter.key_value("Total Requests", &status.ipc_stats.total_requests.to_string()));
        println!("{}", formatter.key_value("Successful Requests", &status.ipc_stats.successful_requests.to_string()));
        println!("{}", formatter.key_value("Failed Requests", &status.ipc_stats.failed_requests.to_string()));
        let success_rate = if status.ipc_stats.total_requests > 0 {
            (status.ipc_stats.successful_requests as f64 / status.ipc_stats.total_requests as f64) * 100.0
        } else {
            0.0
        };
        println!("{}", formatter.key_value("Success Rate", &formatter.percentage(success_rate)));
        println!("{}", formatter.key_value("Avg Response Time", &formatter.duration(status.ipc_stats.avg_response_time_us)));
        println!("{}", formatter.key_value("Active Services", &status.ipc_stats.active_services.to_string()));
        println!("{}", formatter.key_value("Queued Requests", &status.ipc_stats.queued_requests.to_string()));
        
        Ok(())
    }
    
    /// Display in table format
    fn display_table_format(&self, formatter: &Formatter, status: &StatusInfo) -> Result<()> {
        // Create table rows for key metrics
        let table_data = vec![
            StatusTableRow {
                metric: "Overall Health".to_string(),
                value: format!("{}/100", status.health.score),
                status: status.health.status.format_status(formatter),
            },
            StatusTableRow {
                metric: "Space Utilization".to_string(),
                value: format!("{:.1}%", status.filesystem.space_utilization()),
                status: if status.filesystem.space_utilization() > 90.0 { "HIGH" } else { "OK" }.to_string(),
            },
            StatusTableRow {
                metric: "Vector Indexes".to_string(),
                value: status.index_info.len().to_string(),
                status: "OK".to_string(),
            },
            StatusTableRow {
                metric: "Embedding Services".to_string(),
                value: status.embedding_services.len().to_string(),
                status: "OK".to_string(),
            },
        ];
        
        println!("{}", formatter.format_table_data(&table_data)?);
        Ok(())
    }
    
    /// Display in compact format
    fn display_compact_format(&self, formatter: &Formatter, status: &StatusInfo) -> Result<()> {
        println!("Health: {}/100, Space: {:.1}%, Indexes: {}, Services: {}",
            status.health.score,
            status.filesystem.space_utilization(),
            status.index_info.len(),
            status.embedding_services.len()
        );
        Ok(())
    }
}

// Data structures for status information

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusInfo {
    filesystem: crate::client::connection::FilesystemInfo,
    vexfs_status: crate::client::ioctl::FilesystemStatus,
    index_info: Vec<IndexInfo>,
    embedding_services: Vec<crate::client::ipc::EmbeddingService>,
    ipc_stats: crate::client::ipc::IpcStatistics,
    health: HealthInfo,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexInfo {
    name: String,
    index_type: String,
    dimensions: u32,
    vector_count: u64,
    memory_usage: u64,
    disk_usage: u64,
    health_score: u8,
    avg_search_time_ms: u64,
    last_optimized: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthInfo {
    status: Status,
    score: u8,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Tabled, serde::Serialize)]
struct StatusTableRow {
    metric: String,
    value: String,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandConfig;
    use std::path::PathBuf;

    #[test]
    fn test_status_command_creation() {
        let config = CommandConfig::default();
        let mount_point = PathBuf::from("/mnt/vexfs");
        let cmd = StatusCommand::new(config, mount_point);
        
        assert_eq!(cmd.name(), "status");
        assert!(!cmd.description().is_empty());
    }

    #[test]
    fn test_health_calculation() {
        // This would test the health calculation logic
        // Implementation depends on having mock data structures
    }
}