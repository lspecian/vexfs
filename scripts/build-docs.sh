#!/bin/bash

# VexFS Documentation Build Script
# Builds the comprehensive user documentation using MkDocs

set -e

echo "🚀 Building VexFS v1.0 Documentation"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -d "docs/user-guide" ]]; then
    print_error "This script must be run from the VexFS project root directory"
    exit 1
fi

# Check Python installation
if ! command -v python3 &> /dev/null; then
    print_error "Python 3 is required but not installed"
    exit 1
fi

print_info "Python version: $(python3 --version)"

# Navigate to documentation directory
cd docs/user-guide

# Check if virtual environment exists
if [[ ! -d "venv" ]]; then
    print_info "Creating Python virtual environment..."
    python3 -m venv venv
fi

# Activate virtual environment
print_info "Activating virtual environment..."
source venv/bin/activate

# Install/upgrade dependencies
print_info "Installing documentation dependencies..."
pip install --upgrade pip
pip install -r requirements.txt

# Validate MkDocs configuration
print_info "Validating MkDocs configuration..."
if ! mkdocs config-check; then
    print_error "MkDocs configuration is invalid"
    exit 1
fi

print_status "MkDocs configuration is valid"

# Build documentation
print_info "Building documentation..."
if mkdocs build --strict; then
    print_status "Documentation built successfully"
else
    print_error "Documentation build failed"
    exit 1
fi

# Check if site directory was created
if [[ -d "site" ]]; then
    SITE_SIZE=$(du -sh site | cut -f1)
    print_status "Documentation site created (${SITE_SIZE})"
    
    # List main files
    print_info "Generated files:"
    ls -la site/ | head -10
    
    if [[ $(ls site/ | wc -l) -gt 10 ]]; then
        echo "... and $(( $(ls site/ | wc -l) - 10 )) more files"
    fi
else
    print_error "Site directory not created"
    exit 1
fi

# Validate generated HTML
print_info "Validating generated HTML..."
HTML_FILES=$(find site -name "*.html" | wc -l)
print_status "Generated ${HTML_FILES} HTML files"

# Check for broken internal links (basic check)
print_info "Checking for basic issues..."
if grep -r "404" site/ > /dev/null 2>&1; then
    print_warning "Found potential 404 references in generated site"
fi

# Check if index.html exists
if [[ -f "site/index.html" ]]; then
    print_status "Main index.html created"
else
    print_error "Main index.html not found"
    exit 1
fi

# Optional: Serve documentation locally
if [[ "$1" == "--serve" ]]; then
    print_info "Starting local development server..."
    print_info "Documentation will be available at: http://127.0.0.1:8000"
    print_info "Press Ctrl+C to stop the server"
    mkdocs serve
elif [[ "$1" == "--serve-production" ]]; then
    print_info "Starting production-like server..."
    cd site
    python3 -m http.server 8080
else
    print_info "Documentation built successfully!"
    print_info ""
    print_info "To serve locally:"
    print_info "  $0 --serve          # Development server with auto-reload"
    print_info "  $0 --serve-production  # Production-like server"
    print_info ""
    print_info "To deploy:"
    print_info "  - Upload 'site/' directory to your web server"
    print_info "  - Or use GitHub Pages (configured in .github/workflows/docs.yml)"
    print_info ""
    print_info "Documentation site location: $(pwd)/site/"
fi

print_status "Documentation build completed successfully! 🎉"