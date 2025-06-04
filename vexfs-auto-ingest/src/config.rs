use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{info, warn, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub auto_ingest: AutoIngestConfig,
    pub providers: ProvidersConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoIngestConfig {
    /// Enable/disable auto-ingestion
    pub enabled: bool,
    /// Watch paths for file changes
    pub watch_paths: Vec<PathBuf>,
    /// File patterns to include (glob patterns)
    pub include_patterns: Vec<String>,
    /// File patterns to exclude (glob patterns)
    pub exclude_patterns: Vec<String>,
    /// Maximum file size to process (in bytes)
    pub max_file_size: u64,
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    /// Batch processing size
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Default provider to use
    pub default_provider: String,
    pub openai: OpenAIConfig,
    pub ollama: OllamaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage method for embeddings
    pub method: StorageMethod,
    /// Sidecar file extension
    pub sidecar_extension: String,
    /// Use extended attributes if available
    pub use_xattr: bool,
    /// Compression for stored embeddings
    pub compress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageMethod {
    Sidecar,
    ExtendedAttributes,
    VexFSNative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<PathBuf>,
    pub json_format: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_ingest: AutoIngestConfig {
                enabled: false,
                watch_paths: vec![PathBuf::from("/mnt/vexfs")],
                include_patterns: vec![
                    "*.txt".to_string(),
                    "*.md".to_string(),
                    "*.pdf".to_string(),
                    "*.doc".to_string(),
                    "*.docx".to_string(),
                    "*.json".to_string(),
                    "*.csv".to_string(),
                ],
                exclude_patterns: vec![
                    "*.vxvec".to_string(),
                    "*.tmp".to_string(),
                    "*.log".to_string(),
                    ".git/*".to_string(),
                    ".vexfsignore".to_string(),
                ],
                max_file_size: 10 * 1024 * 1024, // 10MB
                debounce_ms: 1000,
                batch_size: 10,
            },
            providers: ProvidersConfig {
                default_provider: "ollama".to_string(),
                openai: OpenAIConfig {
                    enabled: false,
                    api_key: None,
                    model: "text-embedding-3-small".to_string(),
                    base_url: None,
                    timeout_seconds: 30,
                    max_retries: 3,
                },
                ollama: OllamaConfig {
                    enabled: true,
                    base_url: "http://localhost:11434".to_string(),
                    model: "nomic-embed-text".to_string(),
                    timeout_seconds: 60,
                    max_retries: 3,
                },
            },
            storage: StorageConfig {
                method: StorageMethod::Sidecar,
                sidecar_extension: "vxvec".to_string(),
                use_xattr: false,
                compress: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
                json_format: false,
            },
        }
    }
}

impl Config {
    /// Load configuration from multiple sources with precedence:
    /// 1. Environment variables
    /// 2. /etc/vexfs/config.toml
    /// 3. ~/.config/vexfs/config.toml
    /// 4. ./.vexfs.conf
    /// 5. Default values
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Try to load from various config file locations
        let config_paths = [
            PathBuf::from("/etc/vexfs/config.toml"),
            dirs::config_dir().map(|d| d.join("vexfs/config.toml")),
            Some(PathBuf::from(".vexfs.conf")),
        ];

        for path_opt in config_paths.iter() {
            if let Some(path) = path_opt {
                if path.exists() {
                    info!("Loading config from: {}", path.display());
                    config = Self::load_from_file(path)
                        .with_context(|| format!("Failed to load config from {}", path.display()))?;
                    break;
                }
            }
        }

        // Override with environment variables
        config.apply_env_overrides();

        // Validate configuration
        config.validate()?;

        debug!("Final configuration: {:#?}", config);
        Ok(config)
    }

    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;
        
        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        // Auto-ingest enable/disable
        if let Ok(enabled) = std::env::var("VEXFS_AUTO_EMBED") {
            self.auto_ingest.enabled = enabled == "1" || enabled.to_lowercase() == "true";
        }

        // Provider selection
        if let Ok(provider) = std::env::var("VEXFS_PROVIDER") {
            self.providers.default_provider = provider;
        }

        // OpenAI API key
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.providers.openai.api_key = Some(api_key);
            self.providers.openai.enabled = true;
        }

        // Ollama configuration
        if let Ok(base_url) = std::env::var("OLLAMA_BASE_URL") {
            self.providers.ollama.base_url = base_url;
        }

        if let Ok(model) = std::env::var("OLLAMA_MODEL") {
            self.providers.ollama.model = model;
        }

        // Watch paths
        if let Ok(paths) = std::env::var("VEXFS_WATCH_PATHS") {
            self.auto_ingest.watch_paths = paths
                .split(',')
                .map(|s| PathBuf::from(s.trim()))
                .collect();
        }

        // Log level
        if let Ok(level) = std::env::var("VEXFS_LOG_LEVEL") {
            self.logging.level = level;
        }
    }

    fn validate(&self) -> Result<()> {
        // Validate provider configuration
        match self.providers.default_provider.as_str() {
            "openai" => {
                if !self.providers.openai.enabled {
                    anyhow::bail!("OpenAI provider selected but not enabled");
                }
                if self.providers.openai.api_key.is_none() {
                    anyhow::bail!("OpenAI provider enabled but no API key provided");
                }
            }
            "ollama" => {
                if !self.providers.ollama.enabled {
                    anyhow::bail!("Ollama provider selected but not enabled");
                }
            }
            _ => {
                anyhow::bail!("Unknown provider: {}", self.providers.default_provider);
            }
        }

        // Validate watch paths exist
        for path in &self.auto_ingest.watch_paths {
            if !path.exists() {
                warn!("Watch path does not exist: {}", path.display());
            }
        }

        Ok(())
    }

    /// Save current configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;

        info!("Configuration saved to: {}", path.as_ref().display());
        Ok(())
    }

    /// Check if a file should be ignored based on .vexfsignore files
    pub fn should_ignore_file(&self, file_path: &Path) -> bool {
        // Check if file matches exclude patterns
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        for pattern in &self.auto_ingest.exclude_patterns {
            if glob_match(pattern, file_name) {
                return true;
            }
        }

        // Check for .vexfsignore files in parent directories
        let mut current_dir = file_path.parent();
        while let Some(dir) = current_dir {
            let ignore_file = dir.join(".vexfsignore");
            if ignore_file.exists() {
                if let Ok(content) = fs::read_to_string(&ignore_file) {
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            if glob_match(line, file_name) {
                                return true;
                            }
                        }
                    }
                }
            }
            current_dir = dir.parent();
        }

        false
    }

    /// Check if a file should be included for processing
    pub fn should_process_file(&self, file_path: &Path) -> bool {
        // Check file size
        if let Ok(metadata) = file_path.metadata() {
            if metadata.len() > self.auto_ingest.max_file_size {
                return false;
            }
        }

        // Check if file should be ignored
        if self.should_ignore_file(file_path) {
            return false;
        }

        // Check if file matches include patterns
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        for pattern in &self.auto_ingest.include_patterns {
            if glob_match(pattern, file_name) {
                return true;
            }
        }

        false
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple implementation - could be replaced with a proper glob library
    if pattern == "*" {
        return true;
    }
    
    if pattern.starts_with("*.") {
        let ext = &pattern[2..];
        return text.ends_with(ext);
    }
    
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return text.starts_with(prefix);
    }
    
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.auto_ingest.enabled);
        assert_eq!(config.providers.default_provider, "ollama");
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("*.md", "README.md"));
        assert!(!glob_match("*.txt", "file.md"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_should_process_file() {
        let config = Config::default();
        
        // Should process .txt files
        assert!(config.should_process_file(Path::new("test.txt")));
        
        // Should not process .vxvec files
        assert!(!config.should_process_file(Path::new("test.vxvec")));
        
        // Should not process .log files
        assert!(!config.should_process_file(Path::new("test.log")));
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.auto_ingest.enabled, parsed.auto_ingest.enabled);
        assert_eq!(config.providers.default_provider, parsed.providers.default_provider);
    }
}