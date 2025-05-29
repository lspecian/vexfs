# Common Issues and Solutions

Troubleshoot common VexFS issues with this comprehensive guide covering installation, configuration, performance, and operational problems.

## 🚀 Installation Issues

### Issue: Rust Compiler Not Found

**Symptoms:**
```bash
error: could not find `rustc`
```

**Solution:**
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Issue: Build Dependencies Missing

**Symptoms:**
```bash
error: failed to run custom build command for `openssl-sys`
```

**Solution:**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libfuse-dev

# CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel fuse-devel

# macOS
brew install openssl pkg-config
```

### Issue: Python SDK Installation Fails

**Symptoms:**
```bash
error: Microsoft Visual C++ 14.0 is required
```

**Solution:**
```bash
# Windows: Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Linux: Install maturin
pip install maturin

# Build from source
cd bindings/python
maturin develop
```

## 🔧 Configuration Issues

### Issue: Port Already in Use

**Symptoms:**
```bash
Error: Address already in use (os error 98)
```

**Solution:**
```bash
# Find process using port 8000
sudo netstat -tlnp | grep :8000
sudo lsof -i :8000

# Kill the process
sudo kill -9 <PID>

# Or use different port
export VEXFS_PORT=8080
vexfs_server --port 8080
```

### Issue: Permission Denied

**Symptoms:**
```bash
Permission denied (os error 13)
```

**Solution:**
```bash
# Fix data directory permissions
sudo chown -R vexfs:vexfs /var/lib/vexfs
sudo chmod -R 755 /var/lib/vexfs

# Fix log directory permissions
sudo chown -R vexfs:vexfs /var/log/vexfs
sudo chmod -R 755 /var/log/vexfs

# Fix configuration permissions
sudo chown vexfs:vexfs /etc/vexfs/config.toml
sudo chmod 644 /etc/vexfs/config.toml
```

### Issue: Configuration File Not Found

**Symptoms:**
```bash
Error: No such file or directory: config.toml
```

**Solution:**
```bash
# Create default configuration
sudo mkdir -p /etc/vexfs
sudo tee /etc/vexfs/config.toml << EOF
[server]
bind_address = "0.0.0.0"
port = 8000
data_directory = "/var/lib/vexfs"

[vector]
default_dimension = 384
cache_size = "2GB"

[logging]
level = "info"
EOF

# Set permissions
sudo chown vexfs:vexfs /etc/vexfs/config.toml
```

## 🌐 Network and Connectivity Issues

### Issue: Server Not Responding

**Symptoms:**
```bash
curl: (7) Failed to connect to localhost port 8000: Connection refused
```

**Diagnosis:**
```bash
# Check if VexFS is running
sudo systemctl status vexfs
ps aux | grep vexfs

# Check port binding
sudo netstat -tlnp | grep :8000

# Check logs
sudo journalctl -u vexfs -f
tail -f /var/log/vexfs/vexfs.log
```

**Solution:**
```bash
# Start VexFS service
sudo systemctl start vexfs

# Enable auto-start
sudo systemctl enable vexfs

# Check firewall
sudo ufw status
sudo ufw allow 8000/tcp

# Test locally
curl http://localhost:8000/api/v1/version
```

### Issue: Timeout Errors

**Symptoms:**
```bash
Error: request timeout
```

**Solution:**
```bash
# Increase timeout in client
# Python
import vexfs
vexfs.configure({"timeout": 60})

# TypeScript
const client = new VexFSClient({
  timeout: 60000  // 60 seconds
});

# Check server load
top
htop
iostat -x 1

# Optimize server configuration
[server]
request_timeout = 60
max_connections = 1000
```

### Issue: SSL/TLS Certificate Errors

**Symptoms:**
```bash
SSL certificate verify failed
```

**Solution:**
```bash
# Check certificate validity
openssl x509 -in /etc/vexfs/ssl/cert.pem -text -noout

# Regenerate self-signed certificate
sudo openssl req -x509 -newkey rsa:4096 \
  -keyout /etc/vexfs/ssl/key.pem \
  -out /etc/vexfs/ssl/cert.pem \
  -days 365 -nodes

# Or disable TLS for testing
[security]
enable_tls = false
```

## 💾 Data and Storage Issues

### Issue: Vector Dimension Mismatch

**Symptoms:**
```bash
Error: Vector dimension mismatch: expected 384, got 512
```

**Solution:**
```python
# Ensure consistent dimensions
VECTOR_DIMENSION = 384

def validate_vector(vector):
    if len(vector) != VECTOR_DIMENSION:
        raise ValueError(f"Expected {VECTOR_DIMENSION} dimensions, got {len(vector)}")
    return vector

# Pad or truncate vectors if needed
def normalize_vector_dimension(vector, target_dim=384):
    if len(vector) > target_dim:
        return vector[:target_dim]  # Truncate
    elif len(vector) < target_dim:
        return vector + [0.0] * (target_dim - len(vector))  # Pad
    return vector
```

### Issue: Out of Memory

**Symptoms:**
```bash
Error: Cannot allocate memory
```

**Solution:**
```bash
# Check memory usage
free -h
ps aux --sort=-%mem | head

# Reduce cache size
[vector]
cache_size = "1GB"  # Reduce from default

# Enable swap (temporary solution)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Optimize memory settings
[performance]
memory_map = true
compression = "zstd"
```

### Issue: Disk Space Full

**Symptoms:**
```bash
Error: No space left on device
```

**Solution:**
```bash
# Check disk usage
df -h
du -sh /var/lib/vexfs/*

# Clean up old logs
sudo find /var/log/vexfs -name "*.log.*" -mtime +7 -delete

# Clean up old backups
sudo find /var/lib/vexfs/backups -mtime +30 -delete

# Enable compression
[performance]
compression = "zstd"
compression_level = 3

# Configure log rotation
[logging]
max_size = "50MB"
max_files = 5
```

## 🔍 Performance Issues

### Issue: Slow Query Performance

**Symptoms:**
- Query latency > 100ms
- High CPU usage during searches

**Diagnosis:**
```bash
# Check system resources
top
iostat -x 1
sar -u 1

# Check VexFS metrics
curl http://localhost:9091/metrics | grep vexfs_query_duration
```

**Solution:**
```toml
# Optimize index settings
[vector]
index_type = "hnsw"
hnsw_ef_search = 50  # Reduce for faster search
cache_size = "8GB"   # Increase cache

# Optimize performance settings
[performance]
worker_threads = 8   # Match CPU cores
preload_index = true
memory_map = true
```

### Issue: High Memory Usage

**Symptoms:**
- Memory usage continuously growing
- Out of memory errors

**Solution:**
```bash
# Monitor memory usage
watch -n 1 'free -h && ps aux | grep vexfs'

# Configure memory limits
[vector]
cache_size = "4GB"  # Set explicit limit

[performance]
memory_map = false  # Disable if causing issues
compression = "zstd"  # Enable compression

# Restart service periodically (if needed)
sudo systemctl restart vexfs
```

### Issue: Slow Document Insertion

**Symptoms:**
- Insertion rate < 1000 docs/second
- High latency for add operations

**Solution:**
```python
# Use batch operations
documents = [("text1", {"meta": "data1"}), ("text2", {"meta": "data2"})]
doc_ids = vexfs.add_batch(documents)

# Optimize batch size
batch_size = 1000  # Experiment with different sizes
for i in range(0, len(documents), batch_size):
    batch = documents[i:i + batch_size]
    vexfs.add_batch(batch)
```

```toml
# Server optimization
[performance]
batch_size = 2000
async_writes = true
sync_interval = 5000  # Sync less frequently
```

## 🔐 Security Issues

### Issue: Authentication Failures

**Symptoms:**
```bash
Error: Unauthorized (401)
```

**Solution:**
```bash
# Check auth token format
cat /etc/vexfs/auth_tokens

# Regenerate tokens
echo "admin:$(echo -n 'new_secret' | sha256sum | cut -d' ' -f1):read,write,admin" | sudo tee /etc/vexfs/auth_tokens

# Test authentication
curl -H "Authorization: Bearer your_token" http://localhost:8000/api/v1/version
```

### Issue: Certificate Validation Errors

**Symptoms:**
```bash
certificate verify failed: self signed certificate
```

**Solution:**
```python
# Python: Disable SSL verification (development only)
import ssl
ssl._create_default_https_context = ssl._create_unverified_context

# Or use proper certificates
# Let's Encrypt for production
sudo certbot certonly --standalone -d your-domain.com
```

## 🐛 API and SDK Issues

### Issue: Python Import Errors

**Symptoms:**
```python
ImportError: No module named 'vexfs'
```

**Solution:**
```bash
# Install from PyPI
pip install vexfs

# Or build from source
cd bindings/python
pip install maturin
maturin develop

# Check installation
python -c "import vexfs; print('VexFS imported successfully')"
```

### Issue: TypeScript Compilation Errors

**Symptoms:**
```bash
error TS2307: Cannot find module 'vexfs-sdk'
```

**Solution:**
```bash
# Install SDK
npm install vexfs-sdk

# Check TypeScript version
npm install -g typescript@latest

# Update tsconfig.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "moduleResolution": "node",
    "esModuleInterop": true
  }
}
```

### Issue: REST API Errors

**Symptoms:**
```bash
HTTP 500 Internal Server Error
```

**Solution:**
```bash
# Check server logs
tail -f /var/log/vexfs/vexfs.log

# Validate request format
curl -X POST http://localhost:8000/api/v1/collections/test/add \
  -H "Content-Type: application/json" \
  -d '{
    "ids": ["doc1"],
    "documents": ["test"],
    "embeddings": [[0.1, 0.2, 0.3]]
  }'

# Check API documentation
curl http://localhost:8000/api/v1/version
```

## 🔄 Migration Issues

### Issue: ChromaDB Compatibility Problems

**Symptoms:**
- API responses don't match expected format
- Missing fields in responses

**Solution:**
```python
# Verify VexFS compatibility
import requests

# Test basic compatibility
response = requests.get("http://localhost:8000/api/v1/version")
print(response.json())

# Run compatibility test
python3 test_chromadb_compatibility.py
```

### Issue: Data Migration Failures

**Symptoms:**
- Incomplete data transfer
- Corrupted embeddings

**Solution:**
```python
# Validate data before migration
def validate_migration_data(documents):
    for doc in documents:
        assert "id" in doc
        assert "embedding" in doc
        assert len(doc["embedding"]) == 384  # Check dimension
        assert all(isinstance(x, (int, float)) for x in doc["embedding"])

# Migrate in smaller batches
batch_size = 100  # Reduce if having issues
for i in range(0, len(documents), batch_size):
    batch = documents[i:i + batch_size]
    try:
        migrate_batch(batch)
    except Exception as e:
        print(f"Failed batch {i}: {e}")
        # Handle failed batch
```

## 🔧 Diagnostic Tools

### Health Check Script

```bash
#!/bin/bash
# vexfs-health-check.sh

echo "VexFS Health Check"
echo "=================="

# Check service status
echo "Service Status:"
sudo systemctl is-active vexfs
sudo systemctl is-enabled vexfs

# Check port binding
echo -e "\nPort Binding:"
sudo netstat -tlnp | grep :8000

# Check API response
echo -e "\nAPI Response:"
curl -s http://localhost:8000/api/v1/version || echo "API not responding"

# Check disk space
echo -e "\nDisk Space:"
df -h /var/lib/vexfs

# Check memory usage
echo -e "\nMemory Usage:"
free -h

# Check logs for errors
echo -e "\nRecent Errors:"
sudo journalctl -u vexfs --since "1 hour ago" | grep -i error | tail -5
```

### Performance Monitoring

```python
#!/usr/bin/env python3
# vexfs-monitor.py

import requests
import time
import json

def monitor_vexfs():
    """Monitor VexFS performance metrics"""
    
    while True:
        try:
            # Get metrics
            response = requests.get("http://localhost:9091/metrics", timeout=5)
            
            if response.status_code == 200:
                metrics = response.text
                
                # Parse key metrics
                for line in metrics.split('\n'):
                    if 'vexfs_query_duration_seconds' in line:
                        print(f"Query Duration: {line}")
                    elif 'vexfs_memory_usage_bytes' in line:
                        print(f"Memory Usage: {line}")
                    elif 'vexfs_cache_hit_rate' in line:
                        print(f"Cache Hit Rate: {line}")
            
            time.sleep(10)
            
        except Exception as e:
            print(f"Monitoring error: {e}")
            time.sleep(30)

if __name__ == "__main__":
    monitor_vexfs()
```

## 📞 Getting Help

### Log Collection

```bash
#!/bin/bash
# collect-logs.sh

LOG_DIR="/tmp/vexfs-logs-$(date +%Y%m%d_%H%M%S)"
mkdir -p "$LOG_DIR"

# System info
uname -a > "$LOG_DIR/system-info.txt"
free -h > "$LOG_DIR/memory-info.txt"
df -h > "$LOG_DIR/disk-info.txt"

# VexFS logs
sudo cp /var/log/vexfs/*.log "$LOG_DIR/" 2>/dev/null || true
sudo journalctl -u vexfs --since "24 hours ago" > "$LOG_DIR/systemd-logs.txt"

# Configuration
sudo cp /etc/vexfs/config.toml "$LOG_DIR/" 2>/dev/null || true

# Network info
sudo netstat -tlnp | grep vexfs > "$LOG_DIR/network-info.txt"

# Create archive
tar -czf "vexfs-logs-$(date +%Y%m%d_%H%M%S).tar.gz" -C /tmp "$(basename $LOG_DIR)"

echo "Logs collected in: vexfs-logs-$(date +%Y%m%d_%H%M%S).tar.gz"
```

### Support Channels

- **📖 Documentation**: [VexFS Documentation](https://vexfs.github.io/)
- **🐛 Bug Reports**: [GitHub Issues](https://github.com/lspecian/vexfs/issues)
- **💬 Community**: [GitHub Discussions](https://github.com/lspecian/vexfs/discussions)
- **📧 Email**: support@vexfs.org

### Before Reporting Issues

1. **Check this troubleshooting guide**
2. **Search existing issues** on GitHub
3. **Collect logs** using the script above
4. **Provide system information** (OS, version, hardware)
5. **Include reproduction steps** and expected vs actual behavior

---

**Most issues can be resolved quickly with the solutions above. If you're still experiencing problems, don't hesitate to reach out to our community!** 🚀