#!/bin/bash

# VexFS CI/CD Pipeline Setup Script
# This script helps configure the necessary secrets and settings for the CI/CD pipeline

set -e

echo "🚀 VexFS CI/CD Pipeline Setup"
echo "=============================="
echo ""

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

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    print_error "This script must be run from within a git repository"
    exit 1
fi

# Get repository information
REPO_URL=$(git config --get remote.origin.url)
if [[ $REPO_URL == *"github.com"* ]]; then
    # Extract owner/repo from GitHub URL
    REPO_PATH=$(echo $REPO_URL | sed -E 's/.*github\.com[:/]([^/]+\/[^/]+)(\.git)?$/\1/')
    REPO_OWNER=$(echo $REPO_PATH | cut -d'/' -f1)
    REPO_NAME=$(echo $REPO_PATH | cut -d'/' -f2)
    print_status "Detected GitHub repository: $REPO_PATH"
else
    print_error "This script is designed for GitHub repositories"
    exit 1
fi

echo ""
echo "📋 Pre-Setup Checklist"
echo "======================"
echo ""

# Check if required files exist
echo "Checking CI/CD configuration files..."

required_files=(
    ".github/workflows/ci.yml"
    ".github/workflows/release.yml"
    ".github/workflows/publish-sdks.yml"
    ".github/workflows/docker.yml"
    ".github/workflows/docs.yml"
    ".github/workflows/security.yml"
    "deny.toml"
    ".github/changelog-config.json"
)

all_files_exist=true
for file in "${required_files[@]}"; do
    if [[ -f "$file" ]]; then
        print_status "$file exists"
    else
        print_error "$file is missing"
        all_files_exist=false
    fi
done

if [[ "$all_files_exist" == false ]]; then
    print_error "Some required CI/CD files are missing. Please ensure all workflow files are present."
    exit 1
fi

echo ""
echo "🔑 Required Secrets Configuration"
echo "================================="
echo ""

print_info "The following secrets need to be configured in your GitHub repository:"
echo ""

echo "1. PYPI_API_TOKEN (for Python SDK publishing)"
echo "   - Go to https://pypi.org/manage/account/token/"
echo "   - Create a new API token with scope for your project"
echo ""

echo "2. NPM_TOKEN (for TypeScript SDK publishing)"
echo "   - Run: npm login"
echo "   - Run: npm token create --read-only=false"
echo ""

echo "3. CARGO_REGISTRY_TOKEN (for Rust crate publishing)"
echo "   - Go to https://crates.io/settings/tokens"
echo "   - Create a new token"
echo ""

echo "4. CODECOV_TOKEN (optional, for code coverage)"
echo "   - Go to https://codecov.io/ and add your repository"
echo "   - Copy the repository token"
echo ""

print_warning "To add these secrets:"
echo "1. Go to https://github.com/$REPO_PATH/settings/secrets/actions"
echo "2. Click 'New repository secret'"
echo "3. Add each secret with the exact name shown above"
echo ""

read -p "Have you configured all required secrets? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Please configure the secrets before proceeding"
    echo "You can run this script again after setting up the secrets"
    exit 0
fi

echo ""
echo "⚙️  GitHub Repository Settings"
echo "=============================="
echo ""

print_info "Recommended repository settings:"
echo ""

echo "1. Branch Protection for 'main':"
echo "   - Go to https://github.com/$REPO_PATH/settings/branches"
echo "   - Add rule for 'main' branch"
echo "   - Enable: 'Require status checks to pass before merging'"
echo "   - Select: Test Suite, Security Audit, Build Matrix"
echo ""

echo "2. GitHub Pages (for documentation):"
echo "   - Go to https://github.com/$REPO_PATH/settings/pages"
echo "   - Set Source to 'GitHub Actions'"
echo ""

echo "3. Security Settings:"
echo "   - Go to https://github.com/$REPO_PATH/settings/security_analysis"
echo "   - Enable: Dependency graph, Dependabot alerts, Dependabot security updates"
echo "   - Enable: Code scanning alerts"
echo ""

read -p "Have you configured the repository settings? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Please configure the repository settings for optimal CI/CD experience"
fi

echo ""
echo "🧪 Testing the Pipeline"
echo "======================"
echo ""

print_info "To test your CI/CD pipeline:"
echo ""

echo "1. Test CI workflow:"
echo "   git checkout -b test-ci"
echo "   git commit --allow-empty -m 'test: trigger CI workflow'"
echo "   git push origin test-ci"
echo "   # Create a pull request"
echo ""

echo "2. Test release workflow (when ready):"
echo "   # Update version in Cargo.toml"
echo "   git tag v1.0.1"
echo "   git push origin v1.0.1"
echo ""

echo "3. Monitor workflows:"
echo "   https://github.com/$REPO_PATH/actions"
echo ""

echo "📚 Documentation"
echo "================"
echo ""

print_info "Complete documentation available at:"
echo "- CI/CD Pipeline: docs/CI_CD_PIPELINE.md"
echo "- Release Process: .github/ISSUE_TEMPLATE/release.md"
echo ""

echo "🎉 Setup Complete!"
echo "=================="
echo ""

print_status "Your VexFS CI/CD pipeline is ready!"
echo ""

print_info "Next steps:"
echo "1. Push your changes to trigger the first CI run"
echo "2. Monitor the GitHub Actions dashboard"
echo "3. Configure any additional secrets as needed"
echo "4. Review the documentation for detailed usage"
echo ""

print_warning "Remember to:"
echo "- Keep your API tokens secure and rotate them regularly"
echo "- Monitor security scan results"
echo "- Update dependencies regularly via Dependabot"
echo ""

echo "Happy coding! 🚀"