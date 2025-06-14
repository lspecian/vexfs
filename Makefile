# VexFS v2.0 - Unified Build System
# Main project Makefile providing unified interface to all build operations
#
# Usage:
#   make all     - Build everything (kernel module, userspace tools, tests)
#   make kernel  - Build only kernel module
#   make tests   - Build and run tests
#   make clean   - Clean all build artifacts
#   make install - Install components system-wide
#   make help    - Show available targets

.PHONY: all clean kernel userspace tests docs install help
.PHONY: kernel-tests performance-tests integration-tests
.PHONY: syntax-check format lint

# Default target
.DEFAULT_GOAL := all

# Build everything
all: kernel userspace tests

# Kernel module build (primary implementation)
kernel:
	@echo "🔧 Building VexFS v2.0 kernel module..."
	$(MAKE) -C vexfs_v2_build

# Alternative kernel builds
kernel-minimal:
	@echo "🔧 Building minimal kernel module..."
	$(MAKE) -f scripts/build/Makefile.minimal

kernel-fixed:
	@echo "🔧 Building fixed kernel module..."
	$(MAKE) -C vexfs_fixed_build

# Userspace components
userspace:
	@echo "🔧 Building userspace components..."
	@if [ -f "Cargo.toml" ]; then \
		echo "  Building Rust components..."; \
		cargo build --release; \
	fi
	@if [ -d "vexctl" ]; then \
		echo "  Building vexctl..."; \
		$(MAKE) -C vexctl; \
	fi

# Test builds
tests: kernel
	@echo "🧪 Building test suite..."
	$(MAKE) -C tests

# Kernel-specific tests
kernel-tests: kernel
	@echo "🧪 Building kernel tests..."
	$(MAKE) -f scripts/build/Makefile.comparison_tests

# Performance tests
performance-tests: kernel
	@echo "📊 Building performance tests..."
	$(MAKE) -f scripts/build/Makefile.performance

# Integration tests
integration-tests: kernel userspace
	@echo "🔗 Building integration tests..."
	$(MAKE) -C scripts/build -f Makefile.integration

# Documentation
docs:
	@echo "📚 Building documentation..."
	@if [ -f "scripts/build-docs.sh" ]; then \
		./scripts/build-docs.sh; \
	else \
		echo "Documentation build script not found"; \
	fi

# Development tools
syntax-check:
	@echo "✅ Running syntax checks..."
	$(MAKE) -C vexfs_v2_build syntax-check

format:
	@echo "🎨 Formatting code..."
	@if command -v rustfmt >/dev/null 2>&1; then \
		cargo fmt; \
	fi
	@if command -v clang-format >/dev/null 2>&1; then \
		find vexfs_v2_build/ vexfs_fixed_build/ -name "*.c" -o -name "*.h" | xargs clang-format -i; \
	fi

lint:
	@echo "🔍 Running linters..."
	@if command -v cargo >/dev/null 2>&1; then \
		cargo clippy; \
	fi

# Clean targets
clean:
	@echo "🧹 Cleaning build artifacts..."
	$(MAKE) -C vexfs_v2_build clean
	$(MAKE) -C vexfs_fixed_build clean || true
	$(MAKE) -C tests clean || true
	@if [ -f "Cargo.toml" ]; then \
		cargo clean; \
	fi
	@if [ -d "vexctl" ]; then \
		$(MAKE) -C vexctl clean || true; \
	fi
	@echo "✅ Clean completed"

# Installation
install: all
	@echo "📦 Installing VexFS components..."
	@echo "  Installing kernel module..."
	$(MAKE) -C vexfs_v2_build install
	@if [ -f "target/release/vexctl" ]; then \
		echo "  Installing vexctl..."; \
		sudo cp target/release/vexctl /usr/local/bin/; \
	fi
	@echo "✅ Installation completed"

# Development helpers
dev-setup:
	@echo "🛠️  Setting up development environment..."
	@if [ -f "scripts/setup_dev_environment.sh" ]; then \
		./scripts/setup_dev_environment.sh; \
	fi

# Help target
help:
	@echo "VexFS v2.0 Build System"
	@echo "======================="
	@echo ""
	@echo "Main Targets:"
	@echo "  all              Build everything (kernel + userspace + tests)"
	@echo "  kernel           Build VexFS v2.0 kernel module"
	@echo "  userspace        Build userspace components (Rust + vexctl)"
	@echo "  tests            Build and run test suite"
	@echo "  clean            Clean all build artifacts"
	@echo "  install          Install components system-wide"
	@echo ""
	@echo "Kernel Variants:"
	@echo "  kernel-minimal   Build minimal kernel module"
	@echo "  kernel-fixed     Build fixed kernel module"
	@echo ""
	@echo "Test Targets:"
	@echo "  kernel-tests     Build kernel-specific tests"
	@echo "  performance-tests Build performance benchmarks"
	@echo "  integration-tests Build integration tests"
	@echo ""
	@echo "Development:"
	@echo "  syntax-check     Run syntax validation"
	@echo "  format           Format source code"
	@echo "  lint             Run code linters"
	@echo "  dev-setup        Setup development environment"
	@echo "  docs             Build documentation"
	@echo ""
	@echo "Usage Examples:"
	@echo "  make             # Build everything"
	@echo "  make kernel      # Build only kernel module"
	@echo "  make tests       # Build and run tests"
	@echo "  make clean       # Clean all artifacts"
	@echo ""
	@echo "For more information, see docs/build/"