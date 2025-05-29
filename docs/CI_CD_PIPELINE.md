# VexFS CI/CD Pipeline Documentation

This document describes the comprehensive CI/CD pipeline implemented for VexFS v1.0, providing automated testing, security scanning, releases, and deployments.

## Overview

The CI/CD pipeline consists of six main GitHub Actions workflows:

1. **CI (`ci.yml`)** - Continuous Integration with comprehensive testing
2. **Release (`release.yml`)** - Automated release management with semantic versioning
3. **SDK Publishing (`publish-sdks.yml`)** - Automated Python and TypeScript SDK publishing
4. **Docker (`docker.yml`)** - Container image building and publishing
5. **Documentation (`docs.yml`)** - Documentation building and deployment
6. **Security (`security.yml`)** - Security scanning and vulnerability management

## Workflows

### 1. CI Workflow (`ci.yml`)

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop`

**Jobs:**
- **Test Suite**: Multi-Rust version testing (stable, beta, nightly)
- **Security Audit**: Cargo audit and dependency scanning
- **Code Coverage**: Coverage reporting with Codecov
- **Build Matrix**: Cross-platform builds (Ubuntu, macOS)
- **Documentation Check**: Doc generation and testing

**Features:**
- Rust formatting and clippy checks
- FUSE and server binary builds
- Python and TypeScript SDK builds
- Comprehensive test execution
- Caching for faster builds

### 2. Release Workflow (`release.yml`)

**Triggers:**
- Git tags matching `v*.*.*`
- Manual workflow dispatch

**Jobs:**
- **Create Release**: GitHub release with changelog
- **Build Binaries**: Cross-platform binary builds
- **Publish Crate**: Rust crate publishing to crates.io
- **Version Update**: Automatic version bumping for development

**Features:**
- Semantic versioning support
- Automated changelog generation
- Cross-platform binary artifacts
- SHA256 checksums for verification
- Automatic development version bumping

### 3. SDK Publishing Workflow (`publish-sdks.yml`)

**Triggers:**
- Git tags matching `v*.*.*`
- Manual workflow dispatch with selective publishing

**Jobs:**
- **Python SDK**: Multi-architecture wheel building with maturin
- **TypeScript SDK**: npm package publishing
- **Verification**: Post-publication testing

**Features:**
- Multi-architecture Python wheels (x86_64, ARM64, etc.)
- Automatic version synchronization
- Publication verification
- Selective SDK publishing via manual triggers

### 4. Docker Workflow (`docker.yml`)

**Triggers:**
- Push to `main` or `develop`
- Git tags
- Pull requests to `main`

**Jobs:**
- **Development Image**: Full development environment
- **Server Image**: Production-ready VexFS server
- **Testing**: Container functionality verification
- **Security Scanning**: Trivy vulnerability scanning
- **Cleanup**: Old image cleanup

**Features:**
- Multi-architecture builds (AMD64, ARM64)
- GitHub Container Registry publishing
- Automated tagging strategy
- Security scanning with Trivy
- Docker Compose integration

### 5. Documentation Workflow (`docs.yml`)

**Triggers:**
- Push to `main` with doc changes
- Pull requests with doc changes
- Manual workflow dispatch

**Jobs:**
- **Rust Docs**: API documentation generation
- **Python Docs**: Sphinx documentation
- **TypeScript Docs**: TypeDoc documentation
- **Site Building**: Unified documentation site
- **Deployment**: GitHub Pages deployment

**Features:**
- Multi-language documentation
- Unified documentation portal
- Automatic deployment to GitHub Pages
- Link checking for quality assurance

### 6. Security Workflow (`security.yml`)

**Triggers:**
- Push to `main` or `develop`
- Pull requests to `main`
- Daily scheduled runs (2 AM UTC)
- Manual workflow dispatch

**Jobs:**
- **Audit**: Cargo security audit
- **Dependency Review**: PR dependency analysis
- **CodeQL**: Static analysis for multiple languages
- **Semgrep**: Additional security scanning
- **Secrets Scanning**: TruffleHog secret detection
- **Supply Chain**: Dependency analysis
- **Language-Specific**: Python and TypeScript security scans

**Features:**
- Comprehensive security coverage
- Automated vulnerability detection
- Supply chain security analysis
- Dependabot auto-merge for safe updates
- SARIF integration with GitHub Security tab

## Environment Variables and Secrets

The following secrets must be configured in the GitHub repository:

### Required Secrets

| Secret | Purpose | Workflow |
|--------|---------|----------|
| `PYPI_API_TOKEN` | Python package publishing to PyPI | SDK Publishing |
| `NPM_TOKEN` | TypeScript package publishing to npm | SDK Publishing |
| `CARGO_REGISTRY_TOKEN` | Rust crate publishing to crates.io | Release |
| `CODECOV_TOKEN` | Code coverage reporting | CI |

### Automatically Provided

| Secret | Purpose | Workflow |
|--------|---------|----------|
| `GITHUB_TOKEN` | GitHub API access, container registry | All |

## Setup Instructions

### 1. Repository Secrets Configuration

Navigate to your repository's Settings > Secrets and variables > Actions, and add:

```bash
# PyPI API Token (for Python SDK publishing)
PYPI_API_TOKEN=pypi-...

# npm Token (for TypeScript SDK publishing)  
NPM_TOKEN=npm_...

# Crates.io Token (for Rust crate publishing)
CARGO_REGISTRY_TOKEN=cio_...

# Codecov Token (for coverage reporting)
CODECOV_TOKEN=...
```

### 2. GitHub Pages Setup

1. Go to Settings > Pages
2. Set Source to "GitHub Actions"
3. Documentation will be automatically deployed on pushes to main

### 3. Container Registry Setup

GitHub Container Registry is automatically configured. Images will be published to:
- `ghcr.io/[owner]/[repo]:latest` (development image)
- `ghcr.io/[owner]/[repo]-server:latest` (server image)

### 4. Branch Protection

Configure branch protection for `main`:
- Require status checks to pass
- Require branches to be up to date
- Include these required checks:
  - `Test Suite`
  - `Security Audit`
  - `Build Matrix`

## Testing the Pipeline

### 1. Test CI Workflow

```bash
# Create a feature branch and push changes
git checkout -b test-ci
git commit --allow-empty -m "test: trigger CI workflow"
git push origin test-ci

# Create a pull request to trigger full CI
```

### 2. Test Release Workflow

```bash
# Ensure version is updated in Cargo.toml
# Create and push a release tag
git tag v1.0.1
git push origin v1.0.1

# Monitor the release workflow in GitHub Actions
```

### 3. Test Docker Workflow

```bash
# Push to main branch triggers Docker builds
git push origin main

# Check GitHub Container Registry for published images
```

## Monitoring and Maintenance

### GitHub Actions Dashboard

Monitor workflow runs at: `https://github.com/[owner]/[repo]/actions`

### Security Dashboard

Review security findings at: `https://github.com/[owner]/[repo]/security`

### Package Registries

- **Rust**: https://crates.io/crates/vexfs
- **Python**: https://pypi.org/project/vexfs/
- **TypeScript**: https://www.npmjs.com/package/vexfs-sdk
- **Docker**: https://github.com/[owner]/[repo]/pkgs/container/[repo]

### Regular Maintenance Tasks

1. **Weekly**: Review security scan results
2. **Monthly**: Update dependencies via Dependabot PRs
3. **Per Release**: Verify all publication channels
4. **Quarterly**: Review and update CI/CD configurations

## Troubleshooting

### Common Issues

1. **Failed Security Scans**: Check the Security tab for detailed findings
2. **Publication Failures**: Verify API tokens are valid and have correct permissions
3. **Docker Build Failures**: Check for dependency issues or platform compatibility
4. **Documentation Deployment**: Ensure GitHub Pages is enabled and configured

### Debug Commands

```bash
# Test local builds
cargo build --all-features
cargo test --all-features

# Test Docker builds locally
docker build -t vexfs-test .
docker build -t vexfs-server-test -f Dockerfile.server .

# Test Python SDK build
cd bindings/python && maturin build

# Test TypeScript SDK build  
cd bindings/typescript && npm run build
```

## Performance Optimization

The pipeline includes several optimizations:

1. **Caching**: Cargo registry and build artifacts
2. **Matrix Builds**: Parallel execution across platforms
3. **Conditional Execution**: Skip unnecessary jobs on PRs
4. **Artifact Reuse**: Share build outputs between jobs
5. **Incremental Builds**: Only rebuild changed components

## Security Best Practices

1. **Secrets Management**: All sensitive data in GitHub Secrets
2. **Least Privilege**: Minimal required permissions for each job
3. **Supply Chain**: Dependency scanning and verification
4. **Container Security**: Multi-stage builds and vulnerability scanning
5. **Code Analysis**: Static analysis with multiple tools

This CI/CD pipeline provides production-ready automation for VexFS v1.0, ensuring quality, security, and reliable releases across all supported platforms and package managers.