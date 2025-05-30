# VexFS Infrastructure-as-Code Migration Guide

This document provides a comprehensive guide for migrating from the current shell script-based testing approach to the new Infrastructure-as-Code (IaC) framework with Domain-Driven Design (DDD) principles.

## Overview

The new testing infrastructure provides:

- **Infrastructure-as-Code**: Declarative VM lifecycle management with Terraform
- **Configuration Management**: Automated VM setup and test execution with Ansible
- **Domain-Driven Design**: Structured test organization by functional domains
- **Structured Results**: Comprehensive test result collection and analysis
- **Real-time Monitoring**: Prometheus/Grafana integration for observability
- **Trend Analysis**: Automated regression detection and performance tracking

## Migration Strategy

### Phase 1: Infrastructure Setup (Week 1)

#### 1.1 Install Required Tools

```bash
# Install Terraform
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo apt-key add -
sudo apt-add-repository "deb [arch=amd64] https://apt.releases.hashicorp.com $(lsb_release -cs) main"
sudo apt-get update && sudo apt-get install terraform

# Install Ansible
sudo apt-get install ansible

# Install additional tools
sudo apt-get install jq python3-pip
pip3 install aiofiles aiohttp psutil
```

#### 1.2 Prepare Base Infrastructure

```bash
# Clone the repository (if not already done)
cd /path/to/vexfs

# Make deployment script executable
chmod +x infrastructure/scripts/deploy_infrastructure.sh

# Generate SSH keys for VM access
ssh-keygen -t rsa -b 4096 -f ~/.ssh/vexfs_test_key -N ""

# Set environment variables
export VEXFS_SSH_KEY_PATH="$HOME/.ssh/vexfs_test_key"
export VEXFS_BASE_IMAGE_PATH="/var/lib/libvirt/images/vexfs-base.qcow2"
export VEXFS_STORAGE_POOL="default"
export VEXFS_NETWORK_CIDR="192.168.100.0/24"
```

#### 1.3 Build Base VM Image

```bash
# Use existing Packer configuration (enhanced)
cd test_env
packer build vexfs.pkr.hcl

# Or use the new Terraform-managed image building
cd ../infrastructure/terraform
terraform init
terraform apply -target=module.base_images
```

### Phase 2: Domain Migration (Week 2)

#### 2.1 Map Existing Tests to Domains

Current shell scripts map to new domains as follows:

| Current Script | New Domain | Description |
|----------------|------------|-------------|
| `simple_kernel_test.sh` | `kernel_module` | Module loading/unloading tests |
| `vm_comprehensive_test.sh` | `integration_testing` | End-to-end workflow tests |
| `validate_memory_management.sh` | `safety_validation` | Memory leak and hang detection |
| `test_ffi_integration.c` | `kernel_module` | FFI integration tests |
| Vector test runners | `vector_operations` | Vector storage and search tests |
| Performance benchmarks | `performance_metrics` | Throughput and latency tests |
| Basic filesystem ops | `filesystem_operations` | Mount/unmount and I/O tests |

#### 2.2 Migrate Test Logic

For each domain, migrate test logic from shell scripts to Python domain models:

```python
# Example: Migrating simple_kernel_test.sh to kernel_module domain

# Old approach (shell script):
# ./simple_kernel_test.sh

# New approach (domain model):
from tests.domains.kernel_module.domain_model import KernelModuleDomain

domain = KernelModuleDomain(vm_manager, result_collector)
results = await domain.execute_test_suite()
```

#### 2.3 Create Domain-Specific Configurations

```bash
# Create domain configurations
mkdir -p tests/domains/{kernel_module,filesystem_operations,vector_operations}/config

# Example kernel_module config
cat > tests/domains/kernel_module/config.json << EOF
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
EOF
```

### Phase 3: Deployment and Testing (Week 3)

#### 3.1 Deploy Test Infrastructure

```bash
# Deploy full infrastructure
./infrastructure/scripts/deploy_infrastructure.sh --environment test

# Or deploy incrementally
./infrastructure/scripts/deploy_infrastructure.sh --mode terraform-only --environment test
./infrastructure/scripts/deploy_infrastructure.sh --mode ansible-only --environment test
```

#### 3.2 Validate Migration

```bash
# Test VM connectivity
cd infrastructure/ansible
ansible -i inventory/test/hosts.yml test_vms -m ping

# Run domain tests
ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml \
  --extra-vars "domains=['kernel_module']"

# Compare results with old approach
cd ../../test_env
./vm_comprehensive_test.sh  # Old approach
```

#### 3.3 Parallel Operation Period

Run both old and new systems in parallel for 1-2 weeks:

```bash
# Automated comparison script
#!/bin/bash
echo "Running old test approach..."
cd test_env && ./vm_comprehensive_test.sh > old_results.log 2>&1

echo "Running new test approach..."
cd ../infrastructure/ansible
ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml \
  --extra-vars "domains=['kernel_module','filesystem_operations']" \
  > new_results.log 2>&1

echo "Comparing results..."
python3 scripts/compare_test_results.py old_results.log new_results.log
```

### Phase 4: Full Migration (Week 4)

#### 4.1 Deprecate Old Scripts

```bash
# Move old scripts to deprecated directory
mkdir -p test_env/deprecated
mv test_env/*.sh test_env/deprecated/

# Create migration notices
cat > test_env/README_MIGRATION.md << EOF
# Test Environment Migration Notice

The shell script-based testing approach has been deprecated in favor of the new
Infrastructure-as-Code framework. Please use the new system:

## Old vs New Commands

| Old Command | New Command |
|-------------|-------------|
| \`./simple_kernel_test.sh\` | \`ansible-playbook playbooks/run_domain_tests.yml --extra-vars "domains=['kernel_module']"\` |
| \`./vm_comprehensive_test.sh\` | \`ansible-playbook playbooks/run_domain_tests.yml\` |
| \`./validate_memory_management.sh\` | \`ansible-playbook playbooks/run_domain_tests.yml --extra-vars "domains=['safety_validation']"\` |

## Migration Guide

See: docs/testing/INFRASTRUCTURE_AS_CODE_MIGRATION.md
EOF
```

#### 4.2 Update CI/CD Pipelines

```yaml
# .github/workflows/test.yml (example)
name: VexFS Testing
on: [push, pull_request]

jobs:
  infrastructure-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Infrastructure
        run: |
          # Install tools
          sudo apt-get update
          sudo apt-get install terraform ansible
          
          # Deploy test infrastructure
          ./infrastructure/scripts/deploy_infrastructure.sh \
            --environment ci --mode terraform-only
      
      - name: Run Domain Tests
        run: |
          cd infrastructure/ansible
          ansible-playbook -i inventory/ci/hosts.yml \
            playbooks/run_domain_tests.yml \
            --extra-vars "domains=['kernel_module','filesystem_operations']"
      
      - name: Collect Results
        run: |
          python3 tests/shared/result_analysis/generate_report.py \
            --format junit --output test-results.xml
      
      - name: Cleanup Infrastructure
        if: always()
        run: |
          ./infrastructure/scripts/deploy_infrastructure.sh \
            --environment ci --force-destroy
```

## Key Differences and Benefits

### Old Approach vs New Approach

| Aspect | Old (Shell Scripts) | New (IaC + DDD) |
|--------|-------------------|------------------|
| **Infrastructure** | Manual VM setup | Declarative Terraform |
| **Configuration** | Hardcoded in scripts | Ansible playbooks |
| **Test Organization** | Monolithic scripts | Domain-driven structure |
| **Result Collection** | Basic logging | Structured JSON + analysis |
| **Monitoring** | Manual observation | Prometheus/Grafana |
| **Scalability** | Single VM | Multi-VM, parallel execution |
| **Reproducibility** | Environment-dependent | Fully reproducible |
| **Maintenance** | Script modifications | Configuration changes |

### Benefits of New Approach

1. **Maintainability**: Clear separation of concerns with DDD
2. **Reliability**: Idempotent operations with proper error handling
3. **Scalability**: Easy addition of new test domains and environments
4. **Observability**: Comprehensive metrics and trend analysis
5. **Reproducibility**: Consistent environments across different machines
6. **Automation**: Reduced manual intervention and human error
7. **Insights**: Advanced analytics and regression detection

## Domain Architecture

### Domain Boundaries

```
VexFS Testing Domains:
├── KernelModule
│   ├── Module lifecycle (load/unload)
│   ├── Stability testing
│   ├── Memory leak detection
│   └── System hang prevention
├── FilesystemOperations
│   ├── Mount/unmount operations
│   ├── Basic I/O operations
│   ├── Permission handling
│   └── Error condition testing
├── VectorOperations
│   ├── Vector storage
│   ├── Search operations
│   ├── Indexing performance
│   └── Large dataset handling
├── PerformanceMetrics
│   ├── Throughput testing
│   ├── Latency measurement
│   ├── Resource usage monitoring
│   └── Scalability testing
├── SafetyValidation
│   ├── Hang detection
│   ├── Crash recovery
│   ├── Resource leak detection
│   └── System stability
└── IntegrationTesting
    ├── End-to-end workflows
    ├── Cross-domain interactions
    ├── Real-world scenarios
    └── Regression testing
```

### Bounded Contexts

Each domain operates independently with:
- **Clear interfaces** between domains
- **Independent test execution** and reporting
- **Domain-specific metrics** and validation
- **Isolated resource management**

## Result Analysis Framework

### Test Result Schema

```json
{
  "metadata": {
    "test_id": "string",
    "domain": "string", 
    "timestamp": "datetime",
    "environment": "object",
    "version": "string"
  },
  "execution": {
    "duration": "duration",
    "status": "enum[PASS, FAIL, SKIP, ERROR]",
    "exit_code": "integer"
  },
  "metrics": {
    "performance": "object",
    "resource_usage": "object", 
    "custom_metrics": "object"
  },
  "artifacts": {
    "logs": "array[string]",
    "screenshots": "array[string]",
    "dumps": "array[string]"
  },
  "analysis": {
    "trends": "object",
    "regressions": "array",
    "recommendations": "array"
  }
}
```

### Trend Analysis

The new system provides:
- **Automated regression detection** with statistical analysis
- **Performance trend tracking** over time
- **Anomaly detection** with configurable thresholds
- **Actionable recommendations** for test failures
- **Historical comparison** and baseline establishment

## Troubleshooting Migration Issues

### Common Issues and Solutions

#### 1. Terraform Provider Issues

```bash
# Error: Failed to install provider
# Solution: Update Terraform and providers
terraform init -upgrade
terraform providers lock -platform=linux_amd64
```

#### 2. Ansible Connection Issues

```bash
# Error: SSH connection failed
# Solution: Check SSH key permissions and VM accessibility
chmod 600 ~/.ssh/vexfs_test_key
ansible -i inventory/test/hosts.yml test_vms -m ping -vvv
```

#### 3. Domain Test Failures

```bash
# Error: Domain tests not executing
# Solution: Check domain configuration and dependencies
python3 -c "
from tests.domains.kernel_module.domain_model import KernelModuleDomain
print('Domain import successful')
"
```

#### 4. Result Collection Issues

```bash
# Error: Results not being stored
# Solution: Check storage backend configuration
python3 tests/shared/result_analysis/collector.py --test-connection
```

### Rollback Procedure

If migration issues occur, rollback to old system:

```bash
# 1. Destroy new infrastructure
./infrastructure/scripts/deploy_infrastructure.sh --force-destroy --environment test

# 2. Restore old scripts
mv test_env/deprecated/*.sh test_env/

# 3. Resume old testing approach
cd test_env && ./vm_comprehensive_test.sh
```

## Performance Comparison

### Expected Improvements

| Metric | Old Approach | New Approach | Improvement |
|--------|-------------|--------------|-------------|
| **Setup Time** | 15-30 minutes | 5-10 minutes | 50-67% faster |
| **Test Execution** | Sequential only | Parallel capable | 2-4x faster |
| **Result Analysis** | Manual | Automated | 90% time savings |
| **Environment Consistency** | Variable | Guaranteed | 100% reproducible |
| **Failure Diagnosis** | Limited logs | Rich metrics | 10x more data |

### Resource Usage

- **Memory**: Comparable per VM, but better utilization with parallel execution
- **CPU**: More efficient with proper scheduling and resource limits
- **Storage**: Structured storage with automatic cleanup and retention policies
- **Network**: Optimized with dedicated test networks and traffic isolation

## Next Steps After Migration

1. **Monitor Performance**: Use Grafana dashboards to track test performance
2. **Expand Domains**: Add new test domains as VexFS features grow
3. **Optimize Resources**: Fine-tune VM specifications based on actual usage
4. **Enhance Analysis**: Implement additional trend analysis and alerting
5. **Scale Testing**: Add more test environments (staging, performance)

## Support and Documentation

- **Architecture Documentation**: `docs/architecture/`
- **Domain Implementation Guide**: `tests/domains/README.md`
- **Terraform Modules**: `infrastructure/terraform/modules/`
- **Ansible Playbooks**: `infrastructure/ansible/playbooks/`
- **Result Analysis**: `tests/shared/result_analysis/`

For questions or issues during migration, refer to the troubleshooting section above or create an issue in the project repository.

---

*This migration guide is part of the VexFS Infrastructure-as-Code implementation. Last updated: $(date)*