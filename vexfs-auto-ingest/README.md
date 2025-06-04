# VexFS Auto-Ingestion Embedding Pipeline

An intelligent auto-ingestion daemon for VexFS that automatically generates embeddings when files are written to the filesystem. The daemon monitors file changes using inotify and generates embeddings using either local Ollama or cloud OpenAI providers.

## Features

- 🔍 **File Watcher**: Uses inotify to monitor write/create events on VexFS mountpoints
- ⚙️ **Config Toggle**: Enable/disable via environment variables or configuration files
- 🤖 **Multi-Provider Support**: OpenAI API and local Ollama integration
- 💾 **Flexible Storage**: Sidecar files, extended attributes, or VexFS native storage
- 📊 **Logging & Metrics**: Comprehensive logging with performance metrics
- 🛠️ **CLI Management**: Full control via `vexfsctl` command-line tool

## Quick Start

### 1. Installation

```bash
# Build from source
cd vexfs-auto-ingest
cargo build --release

# Install binaries
sudo cp target/release/vexfs-auto-ingest /usr/local/bin/
sudo cp target/release/vexfsctl /usr/local/bin/
```

### 2. Configuration

```bash
# Initialize default configuration
sudo vexfsctl config init

# Edit configuration
sudo nano /etc/vexfs/config.toml
```

### 3. Enable Auto-Ingestion

```bash
# Enable auto-ingestion
vexfsctl auto-ingest on

# Check status
vexfsctl status
```

### 4. Start the Daemon

```bash
# Start as daemon
sudo vexfs-auto-ingest --daemon

# Or run in foreground for testing
vexfs-auto-ingest --verbose
```

## Configuration

### Environment Variables

- `VEXFS_AUTO_EMBED=1` - Enable auto-ingestion
- `VEXFS_PROVIDER=ollama|openai` - Set default provider
- `OPENAI_API_KEY=sk-...` - OpenAI API key
- `OLLAMA_BASE_URL=http://localhost:11434` - Ollama server URL
- `VEXFS_WATCH_PATHS=/mnt/vexfs,/data` - Comma-separated watch paths
- `VEXFS_LOG_LEVEL=info|debug|trace` - Log level

### Configuration File

Create `/etc/vexfs/config.toml`:

```toml
[auto_ingest]
enabled = true
watch_paths = ["/mnt/vexfs"]
include_patterns = ["*.txt", "*.md", "*.pdf"]
exclude_patterns = ["*.vxvec", "*.tmp", ".git/*"]
max_file_size = 10485760  # 10MB
debounce_ms = 1000
batch_size = 10

[providers]
default_provider = "ollama"

[providers.ollama]
enabled = true
base_url = "http://localhost:11434"
model = "nomic-embed-text"

[providers.openai]
enabled = false
model = "text-embedding-3-small"
# api_key = "sk-..."  # Or set OPENAI_API_KEY

[storage]
method = "Sidecar"
sidecar_extension = "vxvec"
compress = true
```

### .vexfsignore Files

Create `.vexfsignore` files in directories to exclude specific files:

```
# Ignore temporary files
*.tmp
*.swp

# Ignore large media files
*.mp4
*.avi
*.mkv

# Ignore system files
.DS_Store
Thumbs.db
```

## CLI Usage

### Auto-Ingestion Control

```bash
# Enable/disable auto-ingestion
vexfsctl auto-ingest on
vexfsctl auto-ingest off
vexfsctl auto-ingest status

# Check overall system status
vexfsctl status
vexfsctl status --verbose
```

### Provider Management

```bash
# List available providers
vexfsctl providers list

# Test provider connectivity
vexfsctl providers test
vexfsctl providers test ollama

# Set default provider
vexfsctl providers set-default ollama
```

### Manual Re-embedding

```bash
# Re-embed a specific file
vexfsctl re-embed document.txt

# Force re-embedding
vexfsctl re-embed document.txt --force

# Use specific provider
vexfsctl re-embed document.txt --provider openai
```

### Embedding Management

```bash
# List files with embeddings
vexfsctl embeddings list /mnt/vexfs

# Show embedding statistics
vexfsctl embeddings stats /mnt/vexfs

# Clean orphaned embeddings
vexfsctl embeddings clean /mnt/vexfs
vexfsctl embeddings clean /mnt/vexfs --dry-run
```

### Configuration Management

```bash
# Show current configuration
vexfsctl config show

# Validate configuration
vexfsctl config validate

# Initialize default config
vexfsctl config init --path /etc/vexfs/config.toml
```

## Supported File Types

### Text Files
- `.txt` - Plain text
- `.md` - Markdown
- `.json` - JSON documents
- `.csv` - CSV files
- `.log` - Log files

### Documents (Future)
- `.pdf` - PDF documents
- `.doc/.docx` - Microsoft Word documents

## Storage Methods

### Sidecar Files (Default)
Embeddings stored as `.filename.ext.vxvec` files alongside original files.

```
document.txt
document.txt.vxvec  # Contains embedding data
```

### Extended Attributes (Linux)
Embeddings stored in file extended attributes (xattr).

```bash
# View embedding metadata
getfattr -n user.vexfs.embedding document.txt
```

### VexFS Native (Future)
Direct integration with VexFS vector storage APIs.

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   File Watcher  │───▶│  Event Processor │───▶│ Embedding Queue │
│    (inotify)    │    │   (Debouncer)    │    │   (Batching)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
┌─────────────────┐    ┌──────────────────┐             │
│ Embedding Store │◀───│   AI Providers   │◀────────────┘
│  (Sidecar/xattr)│    │ (OpenAI/Ollama)  │
└─────────────────┘    └──────────────────┘
```

## Performance

### Benchmarks
- **File Detection**: < 1ms latency via inotify
- **Embedding Generation**: 
  - Ollama (local): 100-500ms per file
  - OpenAI: 200-1000ms per file
- **Storage**: < 10ms for sidecar files
- **Memory Usage**: ~50MB base + ~10MB per 1000 files in queue

### Optimization
- **Batching**: Process multiple files together
- **Debouncing**: Avoid duplicate processing during rapid changes
- **Async Processing**: Non-blocking file monitoring
- **Compression**: Optional embedding compression

## Monitoring

### Logs
```bash
# View daemon logs
journalctl -u vexfs-auto-ingest -f

# Debug mode
vexfs-auto-ingest --verbose
```

### Metrics
```bash
# Real-time status
vexfsctl status --verbose

# Provider metrics
vexfsctl providers test
```

## Troubleshooting

### Common Issues

**Auto-ingestion not working:**
```bash
# Check if enabled
vexfsctl auto-ingest status

# Check provider health
vexfsctl providers test

# Check file permissions
ls -la /mnt/vexfs
```

**Ollama connection failed:**
```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Check model is available
ollama list
ollama pull nomic-embed-text
```

**OpenAI API errors:**
```bash
# Check API key
echo $OPENAI_API_KEY

# Test API connectivity
curl -H "Authorization: Bearer $OPENAI_API_KEY" \
     https://api.openai.com/v1/models
```

### Debug Mode

```bash
# Run with full debugging
RUST_LOG=debug vexfs-auto-ingest --verbose

# Check configuration
vexfsctl config validate
vexfsctl config show
```

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with features
cargo build --features "openai,ollama"
```

### Testing

```bash
# Unit tests
cargo test

# Integration tests with Ollama
OLLAMA_BASE_URL=http://localhost:11434 cargo test -- --ignored

# Integration tests with OpenAI
OPENAI_API_KEY=sk-... cargo test -- --ignored
```

## License

Apache 2.0 License - see [LICENSE](LICENSE) for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Support

- **Issues**: [GitHub Issues](https://github.com/vexfs/vexfs-auto-ingest/issues)
- **Documentation**: [VexFS Docs](https://docs.vexfs.org)
- **Community**: [VexFS Discord](https://discord.gg/vexfs)