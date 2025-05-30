# VexFS Infrastructure-as-Code Testing Framework

A comprehensive, modern testing infrastructure for VexFS that replaces fragile shell scripts with maintainable Infrastructure-as-Code (IaC) and Domain-Driven Design (DDD) principles.

## 🚀 Quick Start

```bash
# 1. Deploy infrastructure
./scripts/deploy_infrastructure.sh --environment test

# 2. Run domain tests
cd ansible
ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml

# 3. View results
open http://localhost:3000  # Grafana dashboard
```

## 📋 Prerequisites

- **Terraform** >= 1.0
- **Ansible** >= 2.9
- **Python** >= 3.8
- **Libvirt/KVM** (for VM management)
- **Docker** (optional, for containerized testing)

```bash
# Install on Ubuntu/Debian
sudo apt-get update
sudo apt-get install terraform ansible python3-pip libvirt-daemon-system
pip3 install aiofiles aiohttp psutil
```

## 🏗️ Architecture Overview

### Infrastructure Components

```
VexFS Testing Infrastructure
├── Terraform Modules
│   ├── QEMU MicroVMs (fast, lightweight VMs)
│   ├── LXC Containers (optional, for lightweight testing)
│   ├── Network Infrastructure (isolated test networks)
│   ├── Storage Management (VM images, test artifacts)
│   └── Monitoring Stack (Prometheus, Grafana)
├── Ansible Automation
│   ├── VM Configuration (kernel dev tools, Rust, VexFS)
│   ├── Test Execution (domain-specific test runners)
│   ├── Result Collection (structured JSON, trend analysis)
│   └── Monitoring Setup (metrics, alerting)
└── Domain-Driven Tests
    ├── Kernel Module Domain (loading, stability, safety)
    ├── Filesystem Operations (mount, I/O, permissions)
    ├── Vector Operations (storage, search, indexing)
    ├── Performance Metrics (throughput, latency, scaling)
    ├── Safety Validation (hang detection, crash recovery)
    └── Integration Testing (end-to-end workflows)
```

### Domain-Driven Design

Each test domain is a bounded context with:
- **Clear responsibilities** and interfaces
- **Independent test execution** and reporting
- **Domain-specific metrics** and validation
- **Isolated resource management**

## 🎯 Test Domains

### 1. Kernel Module Domain
- Module lifecycle (load/unload)
- Stability and stress testing
- Memory leak detection
- System hang prevention
- FFI integration validation

### 2. Filesystem Operations Domain
- Mount/unmount operations
- Basic I/O operations (read, write, seek)
- Permission and access control
- Error condition handling

### 3. Vector Operations Domain
- Vector storage and retrieval
- Search operations and indexing
- Performance with large datasets
- ANNS (Approximate Nearest Neighbor Search)

### 4. Performance Metrics Domain
- Throughput benchmarking
- Latency measurement
- Resource usage monitoring
- Scalability testing

### 5. Safety Validation Domain
- System hang detection
- Crash recovery mechanisms
- Resource leak detection
- System stability validation

### 6. Integration Testing Domain
- End-to-end workflows
- Cross-domain interactions
- Real-world usage scenarios
- Regression testing

## 🛠️ Usage

### Basic Deployment

```bash
# Deploy test environment
./scripts/deploy_infrastructure.sh --environment test

# Deploy with custom configuration
./scripts/deploy_infrastructure.sh \
  --environment dev \
  --mode terraform-only \
  --no-parallel
```

### Running Tests

```bash
# Run all domain tests
cd ansible
ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml

# Run specific domains
ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml \
  --extra-vars "domains=['kernel_module','filesystem_operations']"

# Run with custom timeout
ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml \
  --extra-vars "timeout=3600 retry_count=3"
```

### Manual Test Execution

```bash
# SSH into test VM
ssh -i ~/.ssh/vexfs_test_key vexfs@<vm_ip>

# Run domain-specific tests
vexfs-test  # Runs tests for the VM's assigned domain

# Monitor system status
vexfs-monitor

# Manual kernel module operations
kbuild    # Build kernel module
kload     # Load kernel module
kunload   # Unload kernel module
ktest     # Run kernel tests
```

### Result Analysis

```bash
# View real-time results
python3 tests/shared/result_analysis/dashboard.py

# Generate reports
python3 tests/shared/result_analysis/reporter.py \
  --format html --output report.html

# Analyze trends
python3 tests/shared/result_analysis/analyzer.py \
  --domain kernel_module --period 7d
```

## 📊 Monitoring and Observability

### Prometheus Metrics

- **Test execution metrics**: Duration, success rate, failure reasons
- **System metrics**: CPU, memory, disk I/O, network
- **Domain-specific metrics**: Module load time, vector search latency
- **Infrastructure metrics**: VM health, resource utilization

### Grafana Dashboards

- **Test Overview**: High-level test status across all domains
- **Domain Details**: Deep dive into specific domain performance
- **System Health**: VM and infrastructure monitoring
- **Trend Analysis**: Historical performance and regression detection

### Alerting

```yaml
# Example alert configuration
alerts:
  enabled: true
  webhook_url: "https://hooks.slack.com/services/..."
  rules:
    - name: "High Failure Rate"
      condition: "failure_rate > 0.1"
      severity: "warning"
    - name: "System Hang Detected"
      condition: "test_timeout"
      severity: "critical"
```

## 🔧 Configuration

### Environment Variables

```bash
# Required
export VEXFS_SSH_KEY_PATH="$HOME/.ssh/vexfs_test_key"
export VEXFS_BASE_IMAGE_PATH="/var/lib/libvirt/images/vexfs-base.qcow2"

# Optional
export VEXFS_STORAGE_POOL="default"
export VEXFS_NETWORK_CIDR="192.168.100.0/24"
export TERRAFORM_LOG="INFO"
export ANSIBLE_LOG_LEVEL="2"
```

### Terraform Variables

```hcl
# terraform/environments/test.tfvars
environment = "test"

# VM Configuration
kernel_module_vm_count = 2
filesystem_ops_vm_count = 2
vector_ops_vm_count = 1
performance_vm_count = 1
safety_vm_count = 2
integration_vm_count = 1

# Resource Limits
max_total_memory_gb = 16
max_total_cpus = 16
max_total_disk_gb = 100

# Test Configuration
test_timeout_minutes = 30
parallel_test_execution = true
max_parallel_tests = 4
```

### Domain Configuration

```json
// tests/domains/kernel_module/config.json
{
  "module_name": "vexfs",
  "module_path": "/mnt/vexfs_source/vexfs.ko",
  "build_path": "/mnt/vexfs_source",
  "max_load_time": 30.0,
  "max_unload_time": 10.0,
  "memory_leak_threshold": 1048576,
  "hang_detection_timeout": 60.0,
  "stop_on_critical_failure": true
}
```

## 🚀 Advanced Usage

### Custom Domain Implementation

```python
# tests/domains/custom_domain/domain_model.py
from tests.domains.shared.domain_base import DomainBase, TestResult, TestStatus

class CustomDomain(DomainBase):
    def get_domain_description(self) -> str:
        return "Custom domain for specialized testing"
    
    async def setup_domain(self) -> bool:
        # Domain-specific setup
        return True
    
    async def teardown_domain(self) -> bool:
        # Domain-specific cleanup
        return True
    
    async def validate_domain_constraints(self) -> bool:
        # Domain-specific validation
        return True
    
    async def test_custom_functionality(self) -> TestResult:
        # Implement custom test logic
        return TestResult(
            status=TestStatus.PASSED,
            message="Custom test completed successfully"
        )
```

### Multi-Environment Deployment

```bash
# Deploy to multiple environments
for env in dev test staging; do
  ./scripts/deploy_infrastructure.sh --environment $env
done

# Cross-environment comparison
python3 scripts/compare_environments.py dev test staging
```

### CI/CD Integration

```yaml
# .github/workflows/infrastructure-tests.yml
name: VexFS Infrastructure Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Deploy Infrastructure
        run: |
          ./tests/infrastructure/scripts/deploy_infrastructure.sh \
            --environment ci --mode terraform-only
      
      - name: Run Tests
        run: |
          cd tests/infrastructure/ansible
          ansible-playbook -i inventory/ci/hosts.yml \
            playbooks/run_domain_tests.yml
      
      - name: Collect Results
        run: |
          python3 tests/shared/result_analysis/generate_report.py \
            --format junit --output test-results.xml
      
      - name: Cleanup
        if: always()
        run: |
          ./tests/infrastructure/scripts/deploy_infrastructure.sh \
            --environment ci --force-destroy
```

## 🔍 Troubleshooting

### Common Issues

#### Terraform Issues
```bash
# Provider installation failed
terraform init -upgrade
terraform providers lock -platform=linux_amd64

# State corruption
terraform refresh
terraform plan -refresh-only
```

#### Ansible Issues
```bash
# SSH connection failed
ansible -i inventory/test/hosts.yml test_vms -m ping -vvv
chmod 600 ~/.ssh/vexfs_test_key

# Playbook execution failed
ansible-playbook playbooks/run_domain_tests.yml --check --diff
```

#### VM Issues
```bash
# VM not responding
virsh list --all
virsh console <vm-name>

# Resource constraints
virsh dominfo <vm-name>
free -h && df -h
```

### Debug Mode

```bash
# Enable debug logging
export TERRAFORM_LOG=DEBUG
export ANSIBLE_LOG_LEVEL=4

# Run with verbose output
./scripts/deploy_infrastructure.sh --environment test -vvv
```

## 📈 Performance Optimization

### Resource Tuning

```hcl
# Optimize for performance testing
performance_vm_count = 2
memory = "4096M"
cpus = 4
disk_cache = "none"
disk_io = "native"
enable_hugepages = true
```

### Parallel Execution

```yaml
# Ansible parallel configuration
strategy: free
serial: 4
forks: 8
```

### Result Storage Optimization

```python
# Use database backend for large-scale testing
config = {
    "storage_backend": "database",
    "database_url": "postgresql://user:pass@host/vexfs_results",
    "enable_real_time_analysis": True,
    "trend_analysis_window": "24h"
}
```

## 🔄 Migration from Shell Scripts

See the comprehensive [Migration Guide](../docs/testing/INFRASTRUCTURE_AS_CODE_MIGRATION.md) for step-by-step instructions on migrating from the old shell script-based approach.

### Quick Migration Commands

```bash
# 1. Deploy new infrastructure
./tests/infrastructure/scripts/deploy_infrastructure.sh --environment test

# 2. Run parallel comparison
./scripts/compare_old_vs_new.sh

# 3. Migrate CI/CD pipelines
./scripts/update_ci_pipelines.sh

# 4. Deprecate old scripts
mv tests/legacy/*.sh tests/legacy/deprecated/
```

## 📚 Documentation

- **[Migration Guide](../docs/testing/INFRASTRUCTURE_AS_CODE_MIGRATION.md)**: Detailed migration instructions
- **[Domain Architecture](../tests/domains/README.md)**: Domain-driven design principles
- **[Terraform Modules](terraform/modules/README.md)**: Infrastructure module documentation
- **[Ansible Playbooks](ansible/playbooks/README.md)**: Automation playbook guide
- **[Result Analysis](../tests/shared/result_analysis/README.md)**: Analytics and reporting

## 🤝 Contributing

1. **Add New Domains**: Implement new test domains following the DDD pattern
2. **Enhance Infrastructure**: Improve Terraform modules and Ansible playbooks
3. **Improve Analytics**: Add new metrics, trend analysis, and reporting features
4. **Optimize Performance**: Tune resource usage and execution efficiency

### Development Workflow

```bash
# 1. Create feature branch
git checkout -b feature/new-domain

# 2. Implement changes
# ... make changes ...

# 3. Test locally
./scripts/deploy_infrastructure.sh --environment dev --dry-run

# 4. Run tests
cd ansible && ansible-playbook playbooks/run_domain_tests.yml

# 5. Submit PR
git push origin feature/new-domain
```

## 📄 License

This infrastructure framework is part of VexFS and follows the same licensing:
- **Infrastructure code**: Apache 2.0 License
- **Kernel module components**: GPL v2 License

## 🆘 Support

- **Issues**: Create GitHub issues for bugs and feature requests
- **Discussions**: Use GitHub Discussions for questions and ideas
- **Documentation**: Refer to the comprehensive docs in `docs/testing/`
- **Examples**: Check `examples/` directory for usage patterns

---

**VexFS Infrastructure-as-Code Testing Framework** - Bringing modern DevOps practices to kernel filesystem testing.