# VexFS Quick Start for macOS

## 🚀 Get Running in 5 Minutes

Since the codebase has compilation issues, **use Docker for immediate productivity** while we fix the native build.

### Prerequisites

```bash
# Install Docker Desktop for Mac
brew install --cask docker

# Start Docker Desktop (from Applications)
open -a Docker
```

### Start VexFS (All Components)

```bash
cd /Users/lspecian/Development/vexfs

# Build and start everything
docker-compose up --build

# Services will be available at:
# - API Server: http://localhost:7680
# - Dashboard: http://localhost:3000 (if configured)
```

### Verify It's Working

```bash
# Test API health
curl http://localhost:7680/health

# Test ChromaDB API
curl http://localhost:7680/api/v1/collections

# Test Qdrant API
curl http://localhost:7680/collections
```

## 📝 Development Workflow

### Option 1: Full Docker Development

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Rebuild after code changes
docker-compose up --build
```

### Option 2: Hybrid Development (Recommended)

**Backend (Docker):**
```bash
# Run API server in Docker
docker-compose up vexfs
```

**Frontend (Native macOS):**
```bash
# Terminal 2: Dashboard development
cd vexfs-dashboard
npm install
npm run dev
# Access at http://localhost:3000
```

## 🧪 Testing

### Test Vector Operations

```bash
# Create a collection
curl -X POST http://localhost:7680/api/v1/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test_collection",
    "metadata": {"dimension": 384}
  }'

# Add vectors
curl -X POST http://localhost:7680/api/v1/collections/test_collection/add \
  -H "Content-Type: application/json" \
  -d '{
    "ids": ["id1", "id2"],
    "embeddings": [[0.1, 0.2, ...], [0.3, 0.4, ...]],
    "documents": ["doc1", "doc2"]
  }'

# Search
curl -X POST http://localhost:7680/api/v1/collections/test_collection/query \
  -H "Content-Type: application/json" \
  -d '{
    "query_embeddings": [[0.1, 0.2, ...]],
    "n_results": 5
  }'
```

## 🔧 Configuration

### Environment Variables

Create `.env` file:
```bash
# VexFS Configuration
VEXFS_PORT=7680
VEXFS_DATA_DIR=/app/data
RUST_LOG=info
ALLOW_ANONYMOUS=true

# Optional: Dashboard
DASHBOARD_PORT=3000
API_URL=http://localhost:7680
```

### Docker Compose Override

For development, create `docker-compose.override.yml`:
```yaml
version: '3.8'

services:
  vexfs:
    volumes:
      # Mount source for hot-reload (if supported)
      - ./rust:/app/rust
      - ./vexfs-dashboard:/app/vexfs-dashboard
    environment:
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
    ports:
      - "7680:7680"
      - "3000:3000"
```

## 📊 Monitoring

### View Metrics

```bash
# System metrics
curl http://localhost:7680/metrics | jq

# Health status
curl http://localhost:7680/health | jq

# Collection stats
curl http://localhost:7680/api/v1/collections | jq
```

### Dashboard Access

If dashboard is configured in Docker:
```bash
open http://localhost:3000
```

## 🐛 Troubleshooting

### Docker Won't Start

```bash
# Check Docker is running
docker ps

# Check logs
docker-compose logs vexfs

# Restart Docker Desktop
# Applications → Docker → Quit
# Then reopen Docker Desktop
```

### Port Already in Use

```bash
# Find what's using port 7680
lsof -i :7680

# Kill the process
kill -9 <PID>

# Or change port in docker-compose.yml
```

### Can't Connect to API

```bash
# Check container is running
docker-compose ps

# Check container logs
docker-compose logs vexfs

# Restart services
docker-compose restart
```

## 📈 Performance Tips

### 1. Allocate More Resources to Docker

Docker Desktop → Settings → Resources:
- **CPUs:** 4-6 cores
- **Memory:** 8GB+
- **Disk:** 60GB+

### 2. Use BuildKit

```bash
# Enable BuildKit for faster builds
export DOCKER_BUILDKIT=1
docker-compose build
```

### 3. Use Docker Volumes

Better than bind mounts for data:
```yaml
volumes:
  vexfs_data:
    driver: local
```

## 🎯 Next Steps

### While Docker is Running

1. **Explore the API:**
   - Try different API dialects (ChromaDB, Qdrant, Native)
   - Test vector operations
   - Check performance

2. **Develop Dashboard:**
   - Run natively for fast iteration
   - Connect to Dockerized backend
   - Build new features

3. **Read Codebase:**
   - Understand architecture
   - Identify what to fix for native build
   - Plan improvements

### When Ready for Native Development

See `MACOS_DEVELOPMENT_STATUS.md` for:
- Current compilation issues
- Files that need fixing
- Estimated effort
- Step-by-step fix guide

## 🔄 Switching to Native (Later)

Once codebase is fixed:

```bash
# 1. Stop Docker
docker-compose down

# 2. Build natively
export PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH"
cargo build --release --features fuse_support --bin vexfs_fuse
cargo build --release --features server --bin vexfs_unified_server

# 3. Run natively
./target/release/vexfs_unified_server &
./target/release/vexfs_fuse ~/vexfs-mount -f

# 4. Develop dashboard
cd vexfs-dashboard && npm run dev
```

## 📚 Useful Commands

```bash
# Docker Compose
docker-compose up -d          # Start in background
docker-compose down           # Stop and remove
docker-compose restart        # Restart services
docker-compose logs -f vexfs  # Follow logs
docker-compose exec vexfs sh  # Enter container

# Docker
docker ps                     # List running containers
docker images                 # List images
docker system prune -a        # Clean up everything

# Development
cd vexfs-dashboard && npm run dev    # Dashboard dev mode
cargo watch -x check                  # Watch Rust changes
git status                            # Check changes
```

## 🎉 Success Checklist

- [ ] Docker Desktop installed and running
- [ ] `docker-compose up` successful
- [ ] API responds at http://localhost:7680/health
- [ ] Can create collections
- [ ] Can add and search vectors
- [ ] Dashboard accessible (if configured)
- [ ] Logs show no errors

**You're now ready to develop with VexFS on macOS!**

For native FUSE development, see: `MACOS_DEVELOPMENT_STATUS.md`
