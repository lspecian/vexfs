#!/bin/bash
set -e

# VexFS RPM Package Build Script
# This script builds an .rpm package for VexFS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$SCRIPT_DIR/rpmbuild"
PACKAGE_NAME="vexfs"
VERSION="1.0.0"

echo "Building VexFS RPM package..."
echo "Project root: $PROJECT_ROOT"
echo "Build directory: $BUILD_DIR"

# Clean and create RPM build directory structure
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

# Create source tarball
echo "Creating source tarball..."
cd "$PROJECT_ROOT"
tar --exclude='.git' \
    --exclude='target' \
    --exclude='deployment/packages/build' \
    --exclude='deployment/packages/rpmbuild' \
    --transform "s,^,${PACKAGE_NAME}-${VERSION}/," \
    -czf "$BUILD_DIR/SOURCES/${PACKAGE_NAME}-${VERSION}.tar.gz" .

# Copy spec file
cp "$SCRIPT_DIR/rpm/vexfs.spec" "$BUILD_DIR/SPECS/"

# Build the RPM
echo "Building RPM package..."
rpmbuild --define "_topdir $BUILD_DIR" \
         --define "_builddir $BUILD_DIR/BUILD" \
         --define "_rpmdir $BUILD_DIR/RPMS" \
         --define "_sourcedir $BUILD_DIR/SOURCES" \
         --define "_specdir $BUILD_DIR/SPECS" \
         --define "_srcrpmdir $BUILD_DIR/SRPMS" \
         -ba "$BUILD_DIR/SPECS/vexfs.spec"

# Find the built RPM
RPM_FILE=$(find "$BUILD_DIR/RPMS" -name "*.rpm" -type f | head -1)
SRPM_FILE=$(find "$BUILD_DIR/SRPMS" -name "*.rpm" -type f | head -1)

if [ -n "$RPM_FILE" ]; then
    echo "RPM package built successfully: $RPM_FILE"
    
    # Verify the package
    echo "Verifying RPM package..."
    rpm -qip "$RPM_FILE"
    echo ""
    echo "Package contents:"
    rpm -qlp "$RPM_FILE"
    
    # Run rpmlint if available
    if command -v rpmlint >/dev/null 2>&1; then
        echo ""
        echo "Running rpmlint checks..."
        rpmlint "$RPM_FILE" || true
    fi
    
    echo ""
    echo "To install the package:"
    echo "  sudo rpm -ivh $RPM_FILE"
    echo "  # or"
    echo "  sudo dnf install $RPM_FILE"
    echo "  sudo yum install $RPM_FILE"
    echo ""
    echo "To remove the package:"
    echo "  sudo rpm -e vexfs"
    echo "  # or"
    echo "  sudo dnf remove vexfs"
    echo "  sudo yum remove vexfs"
else
    echo "Error: RPM build failed!"
    exit 1
fi

if [ -n "$SRPM_FILE" ]; then
    echo ""
    echo "Source RPM built: $SRPM_FILE"
fi