# VexFS Docker Deployment Guide

## Overview

VexFS provides complete Docker containerization for easy deployment of the entire system including the API server and web dashboard.

## Quick Start

### Using Docker Compose (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/vexfs.git
cd vexfs

# Build and start all services
docker-compose up --build

# Or run in detached mode
docker-compose up -d --build
```

This will start:
- **API Server** on port `7680`
- **Web Dashboard** on port `3000` (served by the API server)
- **Health checks** with auto-restart on failure

### Using Docker Run

```bash
# Build the image
docker build -t vexfs:latest .

# Run the container
docker run -d \
  --name vexfs \
  -p 7680:7680 \
  -v vexfs_data:/app/data \
  -e RUST_LOG=info \
  -e VEXFS_PORT=7680 \
  vexfs:latest
```

## Docker Architecture

### Multi-Stage Build

The Dockerfile uses a multi-stage build for optimal image size:

1. **Builder Stage** (rust:1.82-bookworm)
   - Compiles the Rust API server
   - Builds with optimizations

2. **Dashboard Stage** (node:18-bookworm)
   - Builds the React dashboard
   - Generates static files

3. **Production Stage** (debian:bookworm-slim)
   - Minimal runtime image
   - Contains only necessary dependencies
   - Runs as non-root user

### Image Details

- **Base Image**: debian:bookworm-slim
- **Size**: ~200MB (optimized)
- **User**: vexfs (non-root)
- **Working Directory**: /app

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VEXFS_PORT` | 7680 | API server port |
| `VEXFS_HOST` | 0.0.0.0 | Bind address |
| `VEXFS_DATA_DIR` | /app/data | Data storage directory |
| `RUST_LOG` | info | Logging level |
| `VEXFS_LOG_LEVEL` | info | VexFS-specific log level |

### Docker Compose Configuration

```yaml
version: '3.8'

services:
  vexfs:
    build: .
    ports:
      - "7680:7680"
    environment:
      - RUST_LOG=info
      - VEXFS_PORT=7680
    volumes:
      - vexfs_data:/app/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:7680/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
    restart: unless-stopped

volumes:
  vexfs_data:
```

## Data Persistence

### Volume Management

Data is persisted in Docker volumes:

```bash
# List volumes
docker volume ls | grep vexfs

# Inspect volume
docker volume inspect vexfs_data

# Backup data
docker run --rm \
  -v vexfs_data:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/vexfs_backup.tar.gz /data

# Restore data
docker run --rm \
  -v vexfs_data:/data \
  -v $(pwd):/backup \
  busybox tar xzf /backup/vexfs_backup.tar.gz -C /
```

### Bind Mounts (Alternative)

For direct file access, use bind mounts:

```bash
docker run -d \
  --name vexfs \
  -p 7680:7680 \
  -v /path/to/local/data:/app/data \
  vexfs:latest
```

## Health Monitoring

### Health Check Endpoint

The container includes automated health checks:

```bash
# Check health status
docker inspect vexfs --format='{{.State.Health.Status}}'

# View health logs
docker inspect vexfs --format='{{json .State.Health}}' | jq

# Manual health check
curl http://localhost:7680/health
```

### Monitoring with Docker Stats

```bash
# Real-time resource usage
docker stats vexfs

# One-time snapshot
docker stats --no-stream vexfs
```

## Networking

### Default Configuration

- Internal port: 7680
- Protocol: HTTP
- Dashboard served from same port

### Custom Network

```bash
# Create custom network
docker network create vexfs-network

# Run with custom network
docker run -d \
  --name vexfs \
  --network vexfs-network \
  -p 7680:7680 \
  vexfs:latest
```

### Reverse Proxy Setup (Nginx)

```nginx
server {
    listen 80;
    server_name vexfs.example.com;

    location / {
        proxy_pass http://localhost:7680;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Production Deployment

### Security Hardening

1. **Use specific image tags**
   ```bash
   docker build -t vexfs:v0.0.4-alpha .
   ```

2. **Limit resources**
   ```yaml
   services:
     vexfs:
       deploy:
         resources:
           limits:
             cpus: '2'
             memory: 2G
           reservations:
             cpus: '1'
             memory: 1G
   ```

3. **Read-only filesystem**
   ```bash
   docker run -d \
     --read-only \
     --tmpfs /tmp \
     -v vexfs_data:/app/data \
     vexfs:latest
   ```

### Scaling

#### Horizontal Scaling (Coming Soon)

```yaml
services:
  vexfs:
    deploy:
      replicas: 3
    environment:
      - VEXFS_CLUSTER_MODE=true
```

#### Load Balancing

```yaml
services:
  nginx:
    image: nginx
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
    depends_on:
      - vexfs1
      - vexfs2
      - vexfs3
```

## Troubleshooting

### Common Issues

#### Container Won't Start

```bash
# Check logs
docker logs vexfs

# Check detailed logs
docker logs --details --timestamps vexfs

# Interactive debug
docker run -it --rm vexfs:latest /bin/bash
```

#### Port Already in Use

```bash
# Find process using port
lsof -i :7680

# Use different port
docker run -p 8080:7680 vexfs:latest
```

#### Permission Issues

```bash
# Fix volume permissions
docker exec vexfs chown -R vexfs:vexfs /app/data

# Run with user ID
docker run --user $(id -u):$(id -g) vexfs:latest
```

### Debug Mode

```bash
# Run with debug logging
docker run -d \
  -e RUST_LOG=debug \
  -e VEXFS_LOG_LEVEL=debug \
  vexfs:latest

# Attach to running container
docker exec -it vexfs /bin/bash
```

## Maintenance

### Updates

```bash
# Pull latest code
git pull origin main

# Rebuild image
docker-compose build --no-cache

# Restart with new image
docker-compose up -d
```

### Cleanup

```bash
# Stop and remove container
docker-compose down

# Remove volumes (WARNING: deletes data)
docker-compose down -v

# Clean unused resources
docker system prune -a
```

### Backup Strategy

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backups/vexfs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Stop container
docker-compose stop

# Backup volume
docker run --rm \
  -v vexfs_data:/data \
  -v $BACKUP_DIR:/backup \
  busybox tar czf /backup/vexfs_${TIMESTAMP}.tar.gz /data

# Start container
docker-compose start

# Keep only last 7 backups
find $BACKUP_DIR -name "vexfs_*.tar.gz" -mtime +7 -delete
```

## Docker Image Registry

### Push to Docker Hub

```bash
# Tag image
docker tag vexfs:latest yourusername/vexfs:latest

# Push to registry
docker push yourusername/vexfs:latest
```

### Use from Registry

```yaml
services:
  vexfs:
    image: yourusername/vexfs:latest
    # ... rest of configuration
```

## Kubernetes Deployment (Future)

### Basic Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: vexfs
spec:
  replicas: 1
  selector:
    matchLabels:
      app: vexfs
  template:
    metadata:
      labels:
        app: vexfs
    spec:
      containers:
      - name: vexfs
        image: vexfs:latest
        ports:
        - containerPort: 7680
        env:
        - name: VEXFS_PORT
          value: "7680"
        volumeMounts:
        - name: data
          mountPath: /app/data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: vexfs-pvc
```

## Monitoring Integration

### Prometheus Metrics (Coming Soon)

```yaml
services:
  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"
```

### Grafana Dashboard (Coming Soon)

```yaml
services:
  grafana:
    image: grafana/grafana
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
```

## Support

For issues with Docker deployment:
1. Check container logs: `docker logs vexfs`
2. Verify health status: `docker inspect vexfs`
3. Review this guide for common issues
4. Open an issue on GitHub with logs and configuration

---

*Docker Deployment Guide for VexFS v0.0.4-alpha*