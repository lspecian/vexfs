use anyhow::{Context, Result};
use clap::{Arg, Command, SubCommand};
use serde_json;
use std::path::PathBuf;
use std::process;
use tokio::fs;
use tracing::{error, info, warn};

mod config;
mod providers;
mod storage;

use config::Config;
use providers::{openai::OpenAIProvider, ollama::OllamaProvider, ProviderManager};
use storage::EmbeddingStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("vexfsctl")
        .version("1.0.0")
        .about("VexFS Auto-Ingestion Control Tool")
        .subcommand(
            Command::new("auto-ingest")
                .about("Manage auto-ingestion settings")
                .subcommand(
                    Command::new("on")
                        .about("Enable auto-ingestion")
                )
                .subcommand(
                    Command::new("off")
                        .about("Disable auto-ingestion")
                )
                .subcommand(
                    Command::new("status")
                        .about("Show auto-ingestion status")
                )
        )
        .subcommand(
            Command::new("status")
                .about("Show overall system status")
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed status")
                )
        )
        .subcommand(
            Command::new("re-embed")
                .about("Re-generate embeddings for files")
                .arg(
                    Arg::new("file")
                        .value_name("FILE")
                        .help("File to re-embed")
                        .required(true)
                )
                .arg(
                    Arg::new("provider")
                        .short('p')
                        .long("provider")
                        .value_name("PROVIDER")
                        .help("Provider to use (openai, ollama)")
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .action(clap::ArgAction::SetTrue)
                        .help("Force re-embedding even if current")
                )
        )
        .subcommand(
            Command::new("providers")
                .about("Manage embedding providers")
                .subcommand(
                    Command::new("list")
                        .about("List available providers")
                )
                .subcommand(
                    Command::new("test")
                        .about("Test provider connectivity")
                        .arg(
                            Arg::new("provider")
                                .value_name("PROVIDER")
                                .help("Provider to test (openai, ollama)")
                        )
                )
                .subcommand(
                    Command::new("set-default")
                        .about("Set default provider")
                        .arg(
                            Arg::new("provider")
                                .value_name("PROVIDER")
                                .help("Provider to set as default")
                                .required(true)
                        )
                )
        )
        .subcommand(
            Command::new("config")
                .about("Manage configuration")
                .subcommand(
                    Command::new("show")
                        .about("Show current configuration")
                )
                .subcommand(
                    Command::new("init")
                        .about("Initialize default configuration")
                        .arg(
                            Arg::new("path")
                                .short('p')
                                .long("path")
                                .value_name("PATH")
                                .help("Configuration file path")
                        )
                )
                .subcommand(
                    Command::new("validate")
                        .about("Validate configuration")
                )
        )
        .subcommand(
            Command::new("embeddings")
                .about("Manage embeddings")
                .subcommand(
                    Command::new("list")
                        .about("List files with embeddings")
                        .arg(
                            Arg::new("directory")
                                .value_name("DIR")
                                .help("Directory to scan")
                                .default_value(".")
                        )
                )
                .subcommand(
                    Command::new("stats")
                        .about("Show embedding statistics")
                        .arg(
                            Arg::new("directory")
                                .value_name("DIR")
                                .help("Directory to analyze")
                                .default_value(".")
                        )
                )
                .subcommand(
                    Command::new("clean")
                        .about("Clean orphaned embeddings")
                        .arg(
                            Arg::new("directory")
                                .value_name("DIR")
                                .help("Directory to clean")
                                .default_value(".")
                        )
                        .arg(
                            Arg::new("dry-run")
                                .long("dry-run")
                                .action(clap::ArgAction::SetTrue)
                                .help("Show what would be cleaned without doing it")
                        )
                )
        )
        .get_matches();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Handle subcommands
    match matches.subcommand() {
        Some(("auto-ingest", sub_matches)) => {
            handle_auto_ingest_command(sub_matches, config).await
        }
        Some(("status", sub_matches)) => {
            handle_status_command(sub_matches, config).await
        }
        Some(("re-embed", sub_matches)) => {
            handle_re_embed_command(sub_matches, config).await
        }
        Some(("providers", sub_matches)) => {
            handle_providers_command(sub_matches, config).await
        }
        Some(("config", sub_matches)) => {
            handle_config_command(sub_matches, config).await
        }
        Some(("embeddings", sub_matches)) => {
            handle_embeddings_command(sub_matches, config).await
        }
        _ => {
            eprintln!("No subcommand provided. Use --help for usage information.");
            process::exit(1);
        }
    }
}

async fn handle_auto_ingest_command(matches: &clap::ArgMatches, mut config: Config) -> Result<()> {
    match matches.subcommand() {
        Some(("on", _)) => {
            config.auto_ingest.enabled = true;
            save_config(&config).await?;
            println!("✅ Auto-ingestion enabled");
        }
        Some(("off", _)) => {
            config.auto_ingest.enabled = false;
            save_config(&config).await?;
            println!("❌ Auto-ingestion disabled");
        }
        Some(("status", _)) => {
            if config.auto_ingest.enabled {
                println!("✅ Auto-ingestion is ENABLED");
                println!("   Watch paths: {:?}", config.auto_ingest.watch_paths);
                println!("   Default provider: {}", config.providers.default_provider);
                println!("   Batch size: {}", config.auto_ingest.batch_size);
                println!("   Debounce: {}ms", config.auto_ingest.debounce_ms);
            } else {
                println!("❌ Auto-ingestion is DISABLED");
            }
        }
        _ => {
            eprintln!("Invalid auto-ingest subcommand. Use --help for usage.");
            process::exit(1);
        }
    }
    Ok(())
}

async fn handle_status_command(matches: &clap::ArgMatches, config: Config) -> Result<()> {
    let verbose = matches.get_flag("verbose");

    println!("🚀 VexFS Auto-Ingestion System Status");
    println!("=====================================");

    // Auto-ingestion status
    if config.auto_ingest.enabled {
        println!("✅ Auto-ingestion: ENABLED");
    } else {
        println!("❌ Auto-ingestion: DISABLED");
    }

    // Provider status
    let mut provider_manager = create_provider_manager(&config).await?;
    let health_results = provider_manager.health_check_all().await;

    println!("\n📡 Provider Status:");
    for (provider, healthy) in &health_results {
        let status = if *healthy { "✅ HEALTHY" } else { "❌ UNHEALTHY" };
        let default_marker = if provider == provider_manager.get_default_provider() {
            " (default)"
        } else {
            ""
        };
        println!("   {}: {}{}", provider, status, default_marker);
    }

    // Storage status
    let storage = EmbeddingStorage::new(config.clone());
    println!("\n💾 Storage:");
    println!("   Method: {:?}", config.storage.method);
    println!("   Sidecar extension: .{}", config.storage.sidecar_extension);
    println!("   Compression: {}", config.storage.compress);

    if verbose {
        println!("\n🔧 Configuration Details:");
        println!("   Watch paths: {:?}", config.auto_ingest.watch_paths);
        println!("   Include patterns: {:?}", config.auto_ingest.include_patterns);
        println!("   Exclude patterns: {:?}", config.auto_ingest.exclude_patterns);
        println!("   Max file size: {} bytes", config.auto_ingest.max_file_size);
        println!("   Batch size: {}", config.auto_ingest.batch_size);
        println!("   Debounce: {}ms", config.auto_ingest.debounce_ms);

        // Show recent metrics if available
        let metrics = provider_manager.get_metrics();
        if !metrics.is_empty() {
            println!("\n📊 Recent Metrics:");
            let summary = provider_manager.get_metrics_summary(10);
            println!("   Total requests: {}", summary.total_requests);
            println!("   Success rate: {:.1}%", summary.success_rate * 100.0);
            println!("   Avg latency: {}ms", summary.avg_latency_ms);
            println!("   Total tokens: {}", summary.total_tokens);
        }
    }

    Ok(())
}

async fn handle_re_embed_command(matches: &clap::ArgMatches, config: Config) -> Result<()> {
    let file_path = PathBuf::from(matches.get_one::<String>("file").unwrap());
    let provider_name = matches.get_one::<String>("provider");
    let force = matches.get_flag("force");

    if !file_path.exists() {
        eprintln!("❌ File does not exist: {}", file_path.display());
        process::exit(1);
    }

    if !config.should_process_file(&file_path) {
        eprintln!("❌ File type not supported for embedding: {}", file_path.display());
        process::exit(1);
    }

    let storage = EmbeddingStorage::new(config.clone());

    // Check if embedding is current
    if !force && storage.is_embedding_current(&file_path).await? {
        println!("ℹ️  Embedding is already current for: {}", file_path.display());
        println!("   Use --force to re-embed anyway");
        return Ok(());
    }

    // Create provider manager
    let mut provider_manager = create_provider_manager(&config).await?;

    // Use specified provider or default
    if let Some(provider) = provider_name {
        provider_manager.set_default_provider(provider.clone())?;
    }

    println!("🔄 Re-embedding file: {}", file_path.display());
    println!("   Provider: {}", provider_manager.get_default_provider());

    // Read file content
    let content = match extract_text_content(&file_path).await {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Failed to extract text: {}", e);
            process::exit(1);
        }
    };

    if content.trim().is_empty() {
        eprintln!("❌ File has no text content");
        process::exit(1);
    }

    // Generate embedding
    let start_time = std::time::Instant::now();
    let embedding_response = provider_manager.embed(content).await?;
    let duration = start_time.elapsed();

    // Store embedding
    let provider = provider_manager.get_default_provider().to_string();
    storage.store_embedding(&file_path, embedding_response, &provider).await?;

    println!("✅ Embedding generated and stored");
    println!("   Dimension: {}", storage.retrieve_embedding(&file_path).await?.unwrap().embedding.len());
    println!("   Time: {:?}", duration);

    Ok(())
}

async fn handle_providers_command(matches: &clap::ArgMatches, mut config: Config) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let provider_manager = create_provider_manager(&config).await?;
            let provider_names = provider_manager.get_provider_names();
            let default_provider = provider_manager.get_default_provider();

            println!("📡 Available Providers:");
            for name in provider_names {
                let default_marker = if name == default_provider { " (default)" } else { "" };
                println!("   • {}{}", name, default_marker);
            }
        }
        Some(("test", sub_matches)) => {
            let provider_manager = create_provider_manager(&config).await?;
            
            if let Some(provider_name) = sub_matches.get_one::<String>("provider") {
                // Test specific provider
                println!("🧪 Testing provider: {}", provider_name);
                match provider_manager.health_check(provider_name).await {
                    Ok(healthy) => {
                        if healthy {
                            println!("✅ Provider {} is healthy", provider_name);
                        } else {
                            println!("❌ Provider {} is not healthy", provider_name);
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to test provider {}: {}", provider_name, e);
                    }
                }
            } else {
                // Test all providers
                println!("🧪 Testing all providers:");
                let health_results = provider_manager.health_check_all().await;
                for (provider, healthy) in health_results {
                    let status = if healthy { "✅ HEALTHY" } else { "❌ UNHEALTHY" };
                    println!("   {}: {}", provider, status);
                }
            }
        }
        Some(("set-default", sub_matches)) => {
            let provider_name = sub_matches.get_one::<String>("provider").unwrap();
            config.providers.default_provider = provider_name.clone();
            save_config(&config).await?;
            println!("✅ Default provider set to: {}", provider_name);
        }
        _ => {
            eprintln!("Invalid providers subcommand. Use --help for usage.");
            process::exit(1);
        }
    }
    Ok(())
}

async fn handle_config_command(matches: &clap::ArgMatches, config: Config) -> Result<()> {
    match matches.subcommand() {
        Some(("show", _)) => {
            let config_json = serde_json::to_string_pretty(&config)?;
            println!("🔧 Current Configuration:");
            println!("{}", config_json);
        }
        Some(("init", sub_matches)) => {
            let config_path = if let Some(path) = sub_matches.get_one::<String>("path") {
                PathBuf::from(path)
            } else {
                PathBuf::from("/etc/vexfs/config.toml")
            };

            let default_config = Config::default();
            default_config.save_to_file(&config_path)?;
            println!("✅ Default configuration saved to: {}", config_path.display());
        }
        Some(("validate", _)) => {
            match config.validate() {
                Ok(_) => println!("✅ Configuration is valid"),
                Err(e) => {
                    println!("❌ Configuration validation failed: {}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Invalid config subcommand. Use --help for usage.");
            process::exit(1);
        }
    }
    Ok(())
}

async fn handle_embeddings_command(matches: &clap::ArgMatches, config: Config) -> Result<()> {
    let storage = EmbeddingStorage::new(config);

    match matches.subcommand() {
        Some(("list", sub_matches)) => {
            let directory = PathBuf::from(sub_matches.get_one::<String>("directory").unwrap());
            let embeddings = storage.list_embeddings(&directory).await?;

            println!("📄 Files with embeddings in {}:", directory.display());
            if embeddings.is_empty() {
                println!("   (none found)");
            } else {
                for file_path in embeddings {
                    if let Ok(Some(embedding)) = storage.retrieve_embedding(&file_path).await {
                        println!("   • {} ({}D, {})", 
                            file_path.display(),
                            embedding.embedding.len(),
                            embedding.provider
                        );
                    }
                }
            }
        }
        Some(("stats", sub_matches)) => {
            let directory = PathBuf::from(sub_matches.get_one::<String>("directory").unwrap());
            let stats = storage.get_storage_stats(&directory).await?;

            println!("📊 Embedding Statistics for {}:", directory.display());
            println!("   Total files: {}", stats.total_files);
            println!("   Total size: {} bytes", stats.total_size_bytes);
            println!("   Storage method: {:?}", stats.storage_method);
        }
        Some(("clean", sub_matches)) => {
            let directory = PathBuf::from(sub_matches.get_one::<String>("directory").unwrap());
            let dry_run = sub_matches.get_flag("dry-run");

            println!("🧹 Cleaning orphaned embeddings in {}:", directory.display());
            
            let embeddings = storage.list_embeddings(&directory).await?;
            let mut orphaned = Vec::new();

            for embedding_file in embeddings {
                // Check if the original file still exists
                if !embedding_file.exists() {
                    orphaned.push(embedding_file);
                }
            }

            if orphaned.is_empty() {
                println!("   No orphaned embeddings found");
            } else {
                println!("   Found {} orphaned embeddings:", orphaned.len());
                for file_path in &orphaned {
                    println!("     • {}", file_path.display());
                }

                if !dry_run {
                    for file_path in orphaned {
                        if let Err(e) = storage.delete_embedding(&file_path).await {
                            warn!("Failed to delete embedding for {}: {}", file_path.display(), e);
                        }
                    }
                    println!("   ✅ Orphaned embeddings cleaned");
                } else {
                    println!("   (dry run - no files deleted)");
                }
            }
        }
        _ => {
            eprintln!("Invalid embeddings subcommand. Use --help for usage.");
            process::exit(1);
        }
    }
    Ok(())
}

async fn create_provider_manager(config: &Config) -> Result<ProviderManager> {
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
    }

    Ok(provider_manager)
}

async fn save_config(config: &Config) -> Result<()> {
    let config_path = dirs::config_dir()
        .map(|d| d.join("vexfs/config.toml"))
        .unwrap_or_else(|| PathBuf::from(".vexfs.conf"));

    config.save_to_file(&config_path)
}

async fn extract_text_content(file_path: &PathBuf) -> Result<String> {
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "txt" | "md" | "json" | "csv" | "log" => {
            fs::read_to_string(file_path).await
                .with_context(|| format!("Failed to read text file: {}", file_path.display()))
        }
        _ => {
            // Try to read as text anyway
            match fs::read_to_string(file_path).await {
                Ok(content) => Ok(content),
                Err(_) => {
                    Err(anyhow::anyhow!("Cannot extract text from file: {}", file_path.display()))
                }
            }
        }
    }
}