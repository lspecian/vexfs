#!/bin/bash
# VexFS .deb Package Build Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PACKAGE_NAME="vexfs"
VERSION="2.0.0"
BUILD_DIR="build"
DIST_DIR="dist"

echo -e "${BLUE}VexFS .deb Package Builder${NC}"
echo -e "${BLUE}============================${NC}"

# Check dependencies
echo -e "${YELLOW}Checking build dependencies...${NC}"
MISSING_DEPS=()

check_command() {
    if ! command -v "$1" &> /dev/null; then
        MISSING_DEPS+=("$1")
    fi
}

check_package() {
    if ! dpkg -l "$1" &> /dev/null; then
        MISSING_DEPS+=("$1")
    fi
}

# Check required commands
check_command "dpkg-buildpackage"
check_command "dh"
check_command "cargo"
check_command "rustc"
check_command "make"
check_command "gcc"

# Check required packages
check_package "debhelper"
check_package "dkms"
check_package "build-essential"
check_package "linux-headers-generic"

if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
    echo -e "${RED}Missing dependencies:${NC}"
    printf '%s\n' "${MISSING_DEPS[@]}"
    echo -e "${YELLOW}Install with:${NC}"
    echo "sudo apt update"
    echo "sudo apt install debhelper dkms build-essential linux-headers-generic rustc cargo pkg-config"
    exit 1
fi

echo -e "${GREEN}All dependencies satisfied${NC}"

# Clean previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
rm -rf "$BUILD_DIR" "$DIST_DIR"
mkdir -p "$BUILD_DIR" "$DIST_DIR"

# Create source package directory
SOURCE_DIR="$BUILD_DIR/${PACKAGE_NAME}-${VERSION}"
mkdir -p "$SOURCE_DIR"

echo -e "${YELLOW}Copying source files...${NC}"

# Copy source files
cp -r kernel "$SOURCE_DIR/"
cp -r rust "$SOURCE_DIR/"
cp -r docs "$SOURCE_DIR/"
cp -r packaging "$SOURCE_DIR/"

# Copy debian packaging files
cp -r packaging/debian "$SOURCE_DIR/"

# Create orig tarball
echo -e "${YELLOW}Creating orig tarball...${NC}"
cd "$BUILD_DIR"
tar -czf "${PACKAGE_NAME}_${VERSION}.orig.tar.gz" "${PACKAGE_NAME}-${VERSION}"
cd ..

# Build Rust utilities first
echo -e "${YELLOW}Building Rust utilities...${NC}"
cd "$SOURCE_DIR/rust"
cargo build --release --bins
cd ../..

# Build packages
echo -e "${YELLOW}Building .deb packages...${NC}"
cd "$SOURCE_DIR"

# Build source package
dpkg-buildpackage -S -us -uc

# Build binary packages
dpkg-buildpackage -b -us -uc

cd ../..

# Move packages to dist directory
echo -e "${YELLOW}Moving packages to dist directory...${NC}"
mv "$BUILD_DIR"/*.deb "$DIST_DIR/"
mv "$BUILD_DIR"/*.dsc "$DIST_DIR/" 2>/dev/null || true
mv "$BUILD_DIR"/*.tar.* "$DIST_DIR/" 2>/dev/null || true
mv "$BUILD_DIR"/*.changes "$DIST_DIR/" 2>/dev/null || true

# Display results
echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "${BLUE}Generated packages:${NC}"
ls -la "$DIST_DIR"

echo ""
echo -e "${BLUE}Installation commands:${NC}"
echo -e "${YELLOW}# Install all packages:${NC}"
echo "sudo dpkg -i $DIST_DIR/*.deb"
echo ""
echo -e "${YELLOW}# Install individual packages:${NC}"
echo "sudo dpkg -i $DIST_DIR/vexfs-dkms_${VERSION}-1_all.deb"
echo "sudo dpkg -i $DIST_DIR/vexfs-utils_${VERSION}-1_amd64.deb"
echo "sudo dpkg -i $DIST_DIR/vexfs-dev_${VERSION}-1_amd64.deb"
echo ""
echo -e "${YELLOW}# Fix dependencies if needed:${NC}"
echo "sudo apt install -f"
echo ""
echo -e "${BLUE}Usage after installation:${NC}"
echo -e "${YELLOW}# Format a device:${NC}"
echo "sudo mkfs.vexfs -V -D 768 -L \"VectorDB\" /dev/sdX1"
echo ""
echo -e "${YELLOW}# Mount filesystem:${NC}"
echo "sudo mount -t vexfs_v2_b62 /dev/sdX1 /mnt/vexfs"
echo ""
echo -e "${GREEN}VexFS is ready for high-performance vector operations!${NC}"