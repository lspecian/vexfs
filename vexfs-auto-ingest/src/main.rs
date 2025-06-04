mod config;
mod providers;
mod storage;
mod watcher;

use anyhow::{Context, Result};
use clap::{Arg, Command};
use config::Config;
use providers::{openai::OpenAIProvider, ollama::OllamaProvider, ProviderManager};
use std::path::PathBuf;
use std::sync::Arc;
use storage::EmbeddingStorage;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use watcher::{FileEvent, FileWatcher};

#[derive(Debug)]
struct IngestionStats {
    files_processed: u64,
    files_failed: u64,
    embeddings_generated: u64,
    total_processing_time_ms: u64,
    last_activity: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for IngestionStats {
    fn default() -> Self {
        Self {
            files_processed: 0,
            files_failed: 0,
            embeddings_generated: 0,
            total_processing_time_ms: 0,
            last_activity: None,
        }
    }
}

struct AutoIngestDaemon {
    config: Config,
    provider_manager: Arc<Mutex<ProviderManager>>,
    storage: Arc<EmbeddingStorage>,
    stats: Arc<Mutex<IngestionStats>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl AutoIngestDaemon {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize provider manager
        let mut provider_manager = ProviderManager::new(config.providers.default_provider.clone());

        // Add OpenAI provider if enabled
        if config.providers.openai.enabled {
            if let Some(api_key) = &config.providers.openai.api_key {
                let openai_provider = OpenAIProvider::new(
                    api_key.clone(),
                    config.providers.openai.model.clone(),
                    config.providers.openai.base_url.clone(),
                    config.providers.openai.timeout_seconds,
                    config.providers.openai.max_retries,
                )?;
                provider_manager.add_provider("openai".to_string(), Box::new(openai_provider));
                info!("OpenAI provider initialized");
            } else {
                warn!("OpenAI provider enabled but no API key provided");
            }
        }

        // Add Ollama provider if enabled
        if config.providers.ollama.enabled {
            let ollama_provider = OllamaProvider::new(
                config.providers.ollama.base_url.clone(),
                config.providers.ollama.model.clone(),
                config.providers.ollama.timeout_seconds,
                config.providers.ollama.max_retries,
            )?;
            provider_manager.add_provider("ollama".to_string(), Box::new(ollama_provider));
            info!("Ollama provider initialized");
        }

        // Initialize storage
        let storage = Arc::new(EmbeddingStorage::new(config.clone()));

        Ok(Self {
            config,
            provider_manager: Arc::new(Mutex::new(provider_manager)),
            storage,
            stats: Arc::new(Mutex::new(IngestionStats::default())),
            shutdown_tx: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if !self.config.auto_ingest.enabled {
            info!("Auto-ingestion is disabled in configuration");
            return Ok(());
        }

        info!("Starting VexFS Auto-Ingestion Daemon");

        // Health check providers
        self.health_check_providers().await?;

        // Create file event channel
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<FileEvent>();

        // Start file watcher
        let mut file_watcher = FileWatcher::new(self.config.clone(), event_tx)?;
        file_watcher.start()?;

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Process file events
        let provider_manager = Arc::clone(&self.provider_manager);
        let storage = Arc::clone(&self.storage);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();

        let processing_task = tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut last_batch_time = tokio::time::Instant::now();
            let batch_timeout = Duration::from_secs(5);

            loop {
                tokio::select! {
                    // Receive file events
                    event = event_rx.recv() => {
                        match event {
                            Some(file_event) => {
                                debug!("Received file event: {:?}", file_event);
                                batch.push(file_event);

                                // Process batch if it's full or timeout reached
                                if batch.len() >= config.auto_ingest.batch_size ||
                                   last_batch_time.elapsed() >= batch_timeout {
                                    Self::process_batch(
                                        &batch,
                                        &provider_manager,
                                        &storage,
                                        &stats,
                                        &config,
                                    ).await;
                                    batch.clear();
                                    last_batch_time = tokio::time::Instant::now();
                                }
                            }
                            None => {
                                info!("File event channel closed");
                                break;
                            }
                        }
                    }

                    // Batch timeout
                    _ = sleep(batch_timeout) => {
                        if !batch.is_empty() && last_batch_time.elapsed() >= batch_timeout {
                            Self::process_batch(
                                &batch,
                                &provider_manager,
                                &storage,
                                &stats,
                                &config,
                            ).await;
                            batch.clear();
                            last_batch_time = tokio::time::Instant::now();
                        }
                    }

                    // Shutdown signal
                    _ = &mut shutdown_rx => {
                        info!("Received shutdown signal");
                        break;
                    }
                }
            }

            // Process any remaining events in batch
            if !batch.is_empty() {
                Self::process_batch(
                    &batch,
                    &provider_manager,
                    &storage,
                    &stats,
                    &config,
                ).await;
            }
        });

        // Start status reporting task
        let stats_clone = Arc::clone(&self.stats);
        let status_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let stats = stats_clone.lock().await;
                info!(
                    "Ingestion stats - Processed: {}, Failed: {}, Embeddings: {}, Avg time: {}ms",
                    stats.files_processed,
                    stats.files_failed,
                    stats.embeddings_generated,
                    if stats.files_processed > 0 {
                        stats.total_processing_time_ms / stats.files_processed
                    } else {
                        0
                    }
                );
            }
        });

        // Wait for shutdown
        processing_task.await?;
        status_task.abort();

        info!("VexFS Auto-Ingestion Daemon stopped");
        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }

    async fn health_check_providers(&self) -> Result<()> {
        let provider_manager = self.provider_manager.lock().await;
        let health_results = provider_manager.health_check_all().await;

        for (provider, healthy) in health_results {
            if healthy {
                info!("Provider {} is healthy", provider);
            } else {
                warn!("Provider {} is not healthy", provider);
            }
        }

        // Check if default provider is healthy
        let default_provider = provider_manager.get_default_provider();
        if let Some(&healthy) = health_results.get(default_provider) {
            if !healthy {
                return Err(anyhow::anyhow!(
                    "Default provider {} is not healthy",
                    default_provider
                ));
            }
        }

        Ok(())
    }

    async fn process_batch(
        batch: &[FileEvent],
        provider_manager: &Arc<Mutex<ProviderManager>>,
        storage: &Arc<EmbeddingStorage>,
        stats: &Arc<Mutex<IngestionStats>>,
        config: &Config,
    ) {
        if batch.is_empty() {
            return;
        }

        info!("Processing batch of {} files", batch.len());

        for event in batch {
            let start_time = tokio::time::Instant::now();
            
            match Self::process_file_event(event, provider_manager, storage, config).await {
                Ok(generated_embedding) => {
                    let mut stats_guard = stats.lock().await;
                    stats_guard.files_processed += 1;
                    if generated_embedding {
                        stats_guard.embeddings_generated += 1;
                    }
                    stats_guard.total_processing_time_ms += start_time.elapsed().as_millis() as u64;
                    stats_guard.last_activity = Some(chrono::Utc::now());
                    
                    info!("Successfully processed file: {}", event.path.display());
                }
                Err(e) => {
                    let mut stats_guard = stats.lock().await;
                    stats_guard.files_failed += 1;
                    stats_guard.total_processing_time_ms += start_time.elapsed().as_millis() as u64;
                    stats_guard.last_activity = Some(chrono::Utc::now());
                    
                    error!("Failed to process file {}: {}", event.path.display(), e);
                }
            }
        }
    }

    async fn process_file_event(
        event: &FileEvent,
        provider_manager: &Arc<Mutex<ProviderManager>>,
        storage: &Arc<EmbeddingStorage>,
        config: &Config,
    ) -> Result<bool> {
        let file_path = &event.path;

        // Check if file still exists and should be processed
        if !file_path.exists() {
            debug!("File no longer exists: {}", file_path.display());
            return Ok(false);
        }

        if !config.should_process_file(file_path) {
            debug!("File should not be processed: {}", file_path.display());
            return Ok(false);
        }

        // Check if embedding is already up-to-date
        if storage.is_embedding_current(file_path).await? {
            debug!("Embedding is already current for file: {}", file_path.display());
            return Ok(false);
        }

        // Read file content
        let content = match Self::extract_text_content(file_path).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to extract text from {}: {}", file_path.display(), e);
                return Err(e);
            }
        };

        if content.trim().is_empty() {
            debug!("File has no text content: {}", file_path.display());
            return Ok(false);
        }

        // Generate embedding
        let mut provider_manager_guard = provider_manager.lock().await;
        let embedding_response = provider_manager_guard.embed(content).await?;
        let provider_name = provider_manager_guard.get_default_provider().to_string();
        drop(provider_manager_guard);

        // Store embedding
        storage.store_embedding(file_path, embedding_response, &provider_name).await?;

        Ok(true)
    }

    async fn extract_text_content(file_path: &PathBuf) -> Result<String> {
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "txt" | "md" | "json" | "csv" | "log" => {
                // Plain text files
                std::fs::read_to_string(file_path)
                    .with_context(|| format!("Failed to read text file: {}", file_path.display()))
            }
            "pdf" => {
                // PDF extraction would require additional dependencies
                warn!("PDF text extraction not yet implemented for: {}", file_path.display());
                Err(anyhow::anyhow!("PDF text extraction not implemented"))
            }
            "doc" | "docx" => {
                // Word document extraction would require additional dependencies
                warn!("Word document text extraction not yet implemented for: {}", file_path.display());
                Err(anyhow::anyhow!("Word document text extraction not implemented"))
            }
            _ => {
                // Try to read as text anyway
                match std::fs::read_to_string(file_path) {
                    Ok(content) => Ok(content),
                    Err(_) => {
                        warn!("Unknown file type or binary file: {}", file_path.display());
                        Err(anyhow::anyhow!("Cannot extract text from file"))
                    }
                }
            }
        }
    }

    pub async fn get_stats(&self) -> IngestionStats {
        self.stats.lock().await.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("vexfs-auto-ingest")
        .version("1.0.0")
        .about("VexFS Auto-Ingestion Embedding Pipeline Daemon")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .arg(
            Arg::new("daemon")
                .short('d')
                .long("daemon")
                .action(clap::ArgAction::SetTrue)
                .help("Run as daemon")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity")
        )
        .get_matches();

    // Initialize logging
    let log_level = match matches.get_count("verbose") {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level))
        )
        .init();

    // Load configuration
    let config = if let Some(config_path) = matches.get_one::<String>("config") {
        Config::load_from_file(config_path)?
    } else {
        Config::load()?
    };

    info!("VexFS Auto-Ingestion Daemon starting...");
    info!("Configuration loaded: auto_ingest.enabled = {}", config.auto_ingest.enabled);

    // Create and start daemon
    let mut daemon = AutoIngestDaemon::new(config).await?;

    // Handle shutdown signals
    let daemon_handle = Arc::new(Mutex::new(daemon));
    let daemon_clone = Arc::clone(&daemon_handle);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        info!("Received shutdown signal");
        let mut daemon = daemon_clone.lock().await;
        daemon.stop().await;
    });

    // Start the daemon
    let mut daemon_guard = daemon_handle.lock().await;
    daemon_guard.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_daemon_creation() {
        let config = Config::default();
        let daemon = AutoIngestDaemon::new(config).await;
        assert!(daemon.is_ok());
    }

    #[tokio::test]
    async fn test_text_extraction() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, world!").unwrap();

        let content = AutoIngestDaemon::extract_text_content(&file_path).await.unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_stats_initialization() {
        let config = Config::default();
        let daemon = AutoIngestDaemon::new(config).await.unwrap();
        let stats = daemon.get_stats().await;
        
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_failed, 0);
        assert_eq!(stats.embeddings_generated, 0);
    }
}