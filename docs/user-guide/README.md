# VexFS v1.0 User Documentation

This directory contains the comprehensive user-facing documentation for VexFS v1.0, built with MkDocs Material for a modern, searchable, and mobile-friendly experience.

## 📚 Documentation Structure

```
docs/user-guide/
├── mkdocs.yml              # MkDocs configuration
├── requirements.txt        # Python dependencies
├── README.md              # This file
└── docs/                  # Documentation content
    ├── index.md           # Homepage
    ├── getting-started/   # Installation and quick start
    ├── user-guide/        # Core usage documentation
    ├── sdk/              # SDK documentation
    ├── examples/         # Code examples and tutorials
    ├── deployment/       # Production deployment guides
    ├── migration/        # Migration from other vector DBs
    ├── troubleshooting/  # Common issues and solutions
    └── reference/        # API reference and configuration
```

## 🚀 Quick Start

### Build Documentation Locally

```bash
# From project root
./scripts/build-docs.sh

# Serve with auto-reload for development
./scripts/build-docs.sh --serve

# Serve production-like version
./scripts/build-docs.sh --serve-production
```

### Manual Setup

```bash
cd docs/user-guide

# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install dependencies
pip install -r requirements.txt

# Build documentation
mkdocs build

# Serve locally
mkdocs serve
```

## 📖 Documentation Sections

### Getting Started
- **[Quick Start](docs/getting-started/quick-start.md)** - Get VexFS running in 5 minutes
- **[Installation](docs/getting-started/installation.md)** - Comprehensive installation guide
- **[System Requirements](docs/getting-started/requirements.md)** - Hardware and software requirements
- **[First Steps](docs/getting-started/first-steps.md)** - Your first VexFS operations

### User Guide
- **[Basic Operations](docs/user-guide/basic-operations.md)** - Core VexFS operations
- **[Vector Search](docs/user-guide/vector-search.md)** - Advanced search techniques
- **[Hybrid Queries](docs/user-guide/hybrid-queries.md)** - Combine vector and metadata search
- **[Batch Operations](docs/user-guide/batch-operations.md)** - High-performance bulk operations
- **[Performance Optimization](docs/user-guide/performance.md)** - Tuning for your use case

### SDK Documentation
- **[Python SDK](docs/sdk/python.md)** - Complete Python API reference
- **[TypeScript SDK](docs/sdk/typescript.md)** - TypeScript/JavaScript integration
- **[REST API](docs/sdk/rest-api.md)** - HTTP API documentation
- **[CLI Tool (vexctl)](docs/sdk/vexctl.md)** - Command-line interface

### Examples
- **[Python Examples](docs/examples/python.md)** - Real-world Python examples
- **[TypeScript Examples](docs/examples/typescript.md)** - TypeScript/JavaScript examples
- **[Use Cases](docs/examples/use-cases.md)** - Industry-specific examples
- **[Integration Patterns](docs/examples/integration.md)** - Common integration patterns

### Deployment
- **[Production Setup](docs/deployment/production.md)** - Enterprise deployment guide
- **[Docker Deployment](docs/deployment/docker.md)** - Container-based deployment
- **[Security Configuration](docs/deployment/security.md)** - Security best practices
- **[Monitoring & Logging](docs/deployment/monitoring.md)** - Observability setup
- **[Backup & Recovery](docs/deployment/backup.md)** - Data protection strategies

### Migration
- **[From ChromaDB](docs/migration/chromadb.md)** - 100% compatible migration
- **[From Pinecone](docs/migration/pinecone.md)** - Pinecone to VexFS migration
- **[From Milvus](docs/migration/milvus.md)** - Milvus migration guide
- **[From Weaviate](docs/migration/weaviate.md)** - Weaviate migration
- **[From FAISS](docs/migration/faiss.md)** - FAISS integration migration
- **[Data Conversion](docs/migration/data-conversion.md)** - Format conversion utilities

### Troubleshooting
- **[Common Issues](docs/troubleshooting/common-issues.md)** - Frequently encountered problems
- **[Error Messages](docs/troubleshooting/error-messages.md)** - Error code reference
- **[Performance Issues](docs/troubleshooting/performance.md)** - Performance debugging
- **[Debugging Guide](docs/troubleshooting/debugging.md)** - Systematic debugging approach

## 🎨 Documentation Features

### Modern Design
- **Material Design** - Clean, professional appearance
- **Dark/Light Mode** - Automatic theme switching
- **Mobile Responsive** - Optimized for all devices
- **Fast Search** - Instant search across all content

### Developer-Friendly
- **Code Syntax Highlighting** - Multi-language support
- **Copy Code Buttons** - One-click code copying
- **Tabbed Content** - Organize related information
- **Admonitions** - Highlight important information

### Navigation
- **Hierarchical Structure** - Logical content organization
- **Breadcrumbs** - Easy navigation tracking
- **Table of Contents** - Page-level navigation
- **Cross-References** - Linked related content

## 🔧 Customization

### Theme Configuration

The documentation uses MkDocs Material with custom configuration in `mkdocs.yml`:

```yaml
theme:
  name: material
  palette:
    # Light mode
    - scheme: default
      primary: blue
      accent: cyan
      toggle:
        icon: material/brightness-7
        name: Switch to dark mode
    # Dark mode  
    - scheme: slate
      primary: blue
      accent: cyan
      toggle:
        icon: material/brightness-4
        name: Switch to light mode
```

### Adding New Content

1. **Create new markdown file** in appropriate directory
2. **Add to navigation** in `mkdocs.yml`
3. **Use consistent formatting** following existing patterns
4. **Test locally** before committing

### Content Guidelines

- **Use clear headings** with proper hierarchy (H1 → H2 → H3)
- **Include code examples** for all concepts
- **Add cross-references** to related sections
- **Use admonitions** for important notes
- **Test all code examples** to ensure they work

## 🚀 Deployment

### Automatic Deployment

Documentation is automatically built and deployed via GitHub Actions:

- **Trigger**: Push to `main` branch with changes in `docs/` directory
- **Build**: MkDocs builds static site
- **Deploy**: GitHub Pages hosts the documentation
- **URL**: `https://vexfs.github.io/` (when configured)

### Manual Deployment

```bash
# Build documentation
mkdocs build

# Deploy to GitHub Pages
mkdocs gh-deploy

# Or upload site/ directory to your web server
rsync -av site/ user@server:/var/www/vexfs-docs/
```

## 📊 Analytics and Monitoring

### Built-in Features

- **Search Analytics** - Track popular search terms
- **Page Views** - Monitor most accessed content
- **User Feedback** - Collect documentation feedback
- **Performance** - Fast loading and responsive design

### External Integration

Add analytics by configuring in `mkdocs.yml`:

```yaml
extra:
  analytics:
    provider: google
    property: G-XXXXXXXXXX
```

## 🤝 Contributing

### Documentation Updates

1. **Fork the repository**
2. **Create feature branch** for documentation changes
3. **Make changes** following style guidelines
4. **Test locally** using build script
5. **Submit pull request** with clear description

### Style Guidelines

- **Use active voice** and clear, concise language
- **Include practical examples** for all concepts
- **Maintain consistent formatting** across sections
- **Update navigation** when adding new content
- **Test all code examples** before submitting

### Review Process

- **Automated checks** validate MkDocs configuration
- **Manual review** ensures content quality
- **Testing** verifies all examples work correctly
- **Deployment** happens automatically after merge

## 📞 Support

### Documentation Issues

- **Content Errors**: [Report documentation bugs](https://github.com/lspecian/vexfs/issues)
- **Missing Content**: [Request new documentation](https://github.com/lspecian/vexfs/discussions)
- **Improvements**: [Suggest enhancements](https://github.com/lspecian/vexfs/discussions)

### Technical Support

- **VexFS Issues**: [GitHub Issues](https://github.com/lspecian/vexfs/issues)
- **Community**: [GitHub Discussions](https://github.com/lspecian/vexfs/discussions)
- **Email**: support@vexfs.org

---

**VexFS v1.0 Documentation** - Comprehensive, searchable, and always up-to-date documentation for the world's first production-ready vector-extended filesystem! 🚀