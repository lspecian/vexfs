#!/bin/bash
set -e

# VexFS Debian Package Build Script
# This script builds a .deb package for VexFS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
PACKAGE_NAME="vexfs"
VERSION="1.0.0"
ARCH="amd64"

echo "Building VexFS Debian package..."
echo "Project root: $PROJECT_ROOT"
echo "Build directory: $BUILD_DIR"

# Clean and create build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Create package directory structure
PACKAGE_DIR="$BUILD_DIR/${PACKAGE_NAME}_${VERSION}_${ARCH}"
mkdir -p "$PACKAGE_DIR/DEBIAN"
mkdir -p "$PACKAGE_DIR/usr/bin"
mkdir -p "$PACKAGE_DIR/lib/systemd/system"
mkdir -p "$PACKAGE_DIR/etc/vexfs"
mkdir -p "$PACKAGE_DIR/var/lib/vexfs"
mkdir -p "$PACKAGE_DIR/var/log/vexfs"
mkdir -p "$PACKAGE_DIR/usr/share/doc/vexfs"

# Build the VexFS server binary
echo "Building VexFS server binary..."
cd "$PROJECT_ROOT"
cargo build --release --features server --bin vexfs_server

# Copy binary
cp "$PROJECT_ROOT/target/release/vexfs_server" "$PACKAGE_DIR/usr/bin/"
chmod 755 "$PACKAGE_DIR/usr/bin/vexfs_server"

# Copy systemd service file
cp "$SCRIPT_DIR/debian/vexfs.service" "$PACKAGE_DIR/lib/systemd/system/"
chmod 644 "$PACKAGE_DIR/lib/systemd/system/vexfs.service"

# Copy control files
cp "$SCRIPT_DIR/debian/control" "$PACKAGE_DIR/DEBIAN/"
cp "$SCRIPT_DIR/debian/postinst" "$PACKAGE_DIR/DEBIAN/"
cp "$SCRIPT_DIR/debian/prerm" "$PACKAGE_DIR/DEBIAN/"

# Make scripts executable
chmod 755 "$PACKAGE_DIR/DEBIAN/postinst"
chmod 755 "$PACKAGE_DIR/DEBIAN/prerm"

# Copy documentation
cp "$PROJECT_ROOT/README.md" "$PACKAGE_DIR/usr/share/doc/vexfs/"
cp "$PROJECT_ROOT/LICENSE" "$PACKAGE_DIR/usr/share/doc/vexfs/"

# Create changelog
cat > "$PACKAGE_DIR/usr/share/doc/vexfs/changelog.Debian" << EOF
vexfs (1.0.0) stable; urgency=medium

  * Initial release of VexFS v1.0
  * ChromaDB-compatible REST API
  * High-performance vector similarity search
  * Production-ready deployment features
  * Comprehensive monitoring and logging
  * Security hardening and best practices

 -- VexFS Contributors <maintainers@vexfs.org>  $(date -R)
EOF

# Compress changelog
gzip -9 "$PACKAGE_DIR/usr/share/doc/vexfs/changelog.Debian"

# Create copyright file
cat > "$PACKAGE_DIR/usr/share/doc/vexfs/copyright" << EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: vexfs
Upstream-Contact: VexFS Contributors <maintainers@vexfs.org>
Source: https://github.com/vexfs/vexfs

Files: *
Copyright: 2024 VexFS Contributors
License: Apache-2.0

License: Apache-2.0
 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 .
 http://www.apache.org/licenses/LICENSE-2.0
 .
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 .
 On Debian systems, the complete text of the Apache License 2.0 can be
 found in "/usr/share/common-licenses/Apache-2.0".
EOF

# Set proper permissions
find "$PACKAGE_DIR" -type d -exec chmod 755 {} \;
find "$PACKAGE_DIR" -type f -exec chmod 644 {} \;
chmod 755 "$PACKAGE_DIR/usr/bin/vexfs_server"
chmod 755 "$PACKAGE_DIR/DEBIAN/postinst"
chmod 755 "$PACKAGE_DIR/DEBIAN/prerm"

# Calculate installed size
INSTALLED_SIZE=$(du -sk "$PACKAGE_DIR" | cut -f1)
echo "Installed-Size: $INSTALLED_SIZE" >> "$PACKAGE_DIR/DEBIAN/control"

# Build the package
echo "Building .deb package..."
cd "$BUILD_DIR"
dpkg-deb --build "${PACKAGE_NAME}_${VERSION}_${ARCH}"

# Verify the package
echo "Verifying package..."
dpkg-deb --info "${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
dpkg-deb --contents "${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"

# Run lintian if available
if command -v lintian >/dev/null 2>&1; then
    echo "Running lintian checks..."
    lintian "${PACKAGE_NAME}_${VERSION}_${ARCH}.deb" || true
fi

echo "Debian package built successfully: $BUILD_DIR/${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
echo ""
echo "To install the package:"
echo "  sudo dpkg -i $BUILD_DIR/${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
echo "  sudo apt-get install -f  # Fix any dependency issues"
echo ""
echo "To remove the package:"
echo "  sudo apt-get remove vexfs"