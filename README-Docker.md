# VexFS Docker Deployment Guide

This guide covers how to deploy VexFS using Docker with the **unified server architecture**.

## Architecture

VexFS uses a **single, unified server** that provides multiple API dialects:
- **ChromaDB-compatible API** (`/api/v1/*`)
- **Qdrant-compatible API** (`/collections/*`)
- **Native VexFS API** (`/vexfs/*`)

## Quick Start

### Using Docker Compose (Recommended)

```bash
# Clone the repository
git clone https://github.com/your-org/vexfs.git
cd vexfs

# Start VexFS with Docker Compose
docker-compose up -d

# Access the services
# VexFS Unified Server: http://localhost:8080
# VexFS Dashboard: http://localhost:3000
```

### Using Pre-built Images from GitHub Container Registry

```bash
# Pull the latest image from GitHub Container Registry
docker pull ghcr.io/your-org/vexfs:latest

# Run VexFS
docker run -d \
  --name vexfs \
  -p 8080:8080 \
  -p 3000:3000 \
  -v vexfs_data:/app/data \
  ghcr.io/your-org/vexfs:latest
```

## Services

### VexFS Unified Server (Port 8080)
- **Multi-API Support**: ChromaDB, Qdrant, and Native VexFS APIs
- **Health Check**: `http://localhost:8080/api/v1/version`
- **Production Ready**: Single, focused server implementation

### VexFS Dashboard (Port 3000)
- **URL**: `http://localhost:3000`
- **Features**: Web interface for managing collections, vectors, and monitoring

## API Endpoints

### Core Endpoints
```bash
# Get server version and status
curl http://localhost:8080/api/v1/version

# List all collections
curl http://localhost:8080/api/v1/collections

# Create a new collection
curl -X POST http://localhost:8080/api/v1/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my_collection",
    "metadata": {"description": "My test collection"}
  }'

# Add vectors to a collection
curl -X POST http://localhost:8080/api/v1/collections/my_collection/add \
  -H "Content-Type: application/json" \
  -d '{
    "ids": ["doc1", "doc2"],
    "embeddings": [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]],
    "metadatas": [{"type": "text"}, {"type": "text"}],
    "documents": ["Hello world", "VexFS is awesome"]
  }'

# Query vectors
curl -X POST http://localhost:8080/api/v1/collections/my_collection/query \
  -H "Content-Type: application/json" \
  -d '{
    "query_embeddings": [[0.1, 0.2, 0.3]],
    "n_results": 5
  }'
```

## Development Setup

### Building from Source

```bash
# Build the Docker image locally
docker build -t vexfs:local .

# Run the locally built image
docker run -d \
  --name vexfs-dev \
  -p 8080:8080 \
  -p 3000:3000 \
  -v $(pwd)/data:/app/data \
  vexfs:local
```

### Development with Hot Reload

For development with hot reload, use the development setup:

```bash
# Start only the dependencies
docker-compose up -d

# Run VexFS server locally for development
cargo run --features="server" --bin vexfs_server

# Run dashboard locally for development
cd vexfs-dashboard
npm run dev
```

## Production Setup

### With Nginx Reverse Proxy

```bash
# Start with production profile (includes nginx)
docker-compose --profile production up -d

# Access via nginx (port 80)
# API: http://localhost/api/
# Dashboard: http://localhost/
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | VexFS server port |
| `DASHBOARD_PORT` | `3000` | Dashboard port |
| `RUST_LOG` | `info` | Logging level |
| `VEXFS_DATA_DIR` | `/app/data` | Data storage directory |

### Custom Configuration

```bash
# Run with custom configuration
docker run -d \
  --name vexfs \
  -p 8080:8080 \
  -p 3000:3000 \
  -e RUST_LOG=debug \
  -e PORT=9000 \
  -v /host/data:/app/data \
  ghcr.io/your-org/vexfs:latest
```

## Monitoring and Health Checks

### Health Check Endpoints

```bash
# Server health
curl http://localhost:8080/api/v1/version

# Container health (Docker)
docker ps  # Check health status
```

### Logs

```bash
# View container logs
docker logs vexfs

# Follow logs in real-time
docker logs -f vexfs

# With docker-compose
docker-compose logs -f vexfs
```

## Data Persistence

VexFS stores data in `/app/data` inside the container. To persist data:

```bash
# Using named volume (recommended)
docker run -v vexfs_data:/app/data ghcr.io/your-org/vexfs:latest

# Using bind mount
docker run -v /host/path:/app/data ghcr.io/your-org/vexfs:latest
```

## Troubleshooting

### Common Issues

1. **Server not starting**: Check logs for compilation errors
   ```bash
   docker logs vexfs
   ```

2. **Port conflicts**: Change port mappings
   ```bash
   docker run -p 8081:8080 -p 3001:3000 ghcr.io/your-org/vexfs:latest
   ```

3. **Permission issues**: Ensure data directory is writable
   ```bash
   sudo chown -R 1000:1000 /host/data
   ```

### Debug Mode

```bash
# Run with debug logging
docker run -e RUST_LOG=debug ghcr.io/your-org/vexfs:latest

# Run interactively for debugging
docker run -it --entrypoint /bin/bash ghcr.io/your-org/vexfs:latest
```

## CI/CD Integration

The project includes GitHub Actions workflows that:

1. **Build and test** the application
2. **Build Docker images** for multiple architectures
3. **Push to GitHub Container Registry**
4. **Create releases** with Docker image tags

### Available Images

- `ghcr.io/your-org/vexfs:latest` - Latest stable release
- `ghcr.io/your-org/vexfs:main` - Latest main branch
- `ghcr.io/your-org/vexfs:develop` - Latest develop branch
- `ghcr.io/your-org/vexfs:v{version}` - Specific version tags

## Security Considerations

1. **Network Security**: Use reverse proxy for production
2. **Data Security**: Secure data directory permissions
3. **Container Security**: Run as non-root user (default)
4. **API Security**: Implement authentication if needed

## Performance Tuning

### Resource Limits

```bash
# Set memory and CPU limits
docker run \
  --memory=2g \
  --cpus=2 \
  ghcr.io/your-org/vexfs:latest
```

### Scaling

```bash
# Scale with docker-compose
docker-compose up --scale vexfs=3

# Use with load balancer for high availability
```

## Support

- **Issues**: [GitHub Issues](https://github.com/your-org/vexfs/issues)
- **Documentation**: [Main README](./README.md)
- **API Documentation**: Available at `/api/v1/version` endpoint