# VexFS v2.0 Ollama Integration Makefile
# 
# This Makefile builds the Ollama integration components for VexFS v2.0
# Phase 1 implementation with real embedding validation.

CC = gcc
CFLAGS = -Wall -Wextra -std=c11 -O2 -g
INCLUDES = -I../kernel/vexfs_v2_build -I.
LIBS = -lcurl -ljson-c -lm -lpthread

# Source files
SOURCES = ollama_client.c
HEADERS = ollama_client.h ../kernel/vexfs_v2_build/vexfs_v2_uapi.h

# Test programs
TEST_PROGRAMS = test_ollama_basic test_real_embeddings test_storage_validation test_sda_full_capacity

# Library
LIBRARY = libvexfs_ollama.a
SHARED_LIBRARY = libvexfs_ollama.so

# Object files
OBJECTS = $(SOURCES:.c=.o)

# Default target
all: check_deps $(LIBRARY) $(SHARED_LIBRARY) $(TEST_PROGRAMS)

# Check dependencies
check_deps:
	@echo "🔍 Checking dependencies..."
	@which curl >/dev/null 2>&1 || (echo "❌ curl not found. Install with: sudo apt-get install curl" && exit 1)
	@pkg-config --exists libcurl || (echo "❌ libcurl-dev not found. Install with: sudo apt-get install libcurl4-openssl-dev" && exit 1)
	@pkg-config --exists json-c || (echo "❌ libjson-c-dev not found. Install with: sudo apt-get install libjson-c-dev" && exit 1)
	@echo "✅ All dependencies found"

# Install dependencies (Ubuntu/Debian)
install_deps:
	@echo "📦 Installing dependencies..."
	sudo apt-get update
	sudo apt-get install -y libcurl4-openssl-dev libjson-c-dev curl
	@echo "✅ Dependencies installed"

# Install Ollama
install_ollama:
	@echo "🦙 Installing Ollama..."
	curl -fsSL https://ollama.ai/install.sh | sh
	@echo "✅ Ollama installed"

# Pull embedding models
pull_models:
	@echo "📥 Pulling embedding models..."
	ollama pull nomic-embed-text
	ollama pull all-minilm
	@echo "✅ Models pulled"

# Setup complete environment
setup: install_deps install_ollama pull_models
	@echo "🎉 Complete setup finished!"

# Compile object files
%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) $(INCLUDES) -fPIC -c $< -o $@

# Build static library
$(LIBRARY): $(OBJECTS)
	ar rcs $@ $^
	@echo "✅ Built static library: $(LIBRARY)"

# Build shared library
$(SHARED_LIBRARY): $(OBJECTS)
	$(CC) -shared -o $@ $^ $(LIBS)
	@echo "✅ Built shared library: $(SHARED_LIBRARY)"

# Test programs
test_ollama_basic: test_ollama_basic.c $(LIBRARY)
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LIBRARY) $(LIBS)

test_real_embeddings: test_real_embeddings.c $(LIBRARY)
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LIBRARY) $(LIBS)

test_storage_validation: test_storage_validation.c $(LIBRARY)
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LIBRARY) $(LIBS)

test_sda_full_capacity: test_sda_full_capacity.c $(LIBRARY)
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LIBRARY) $(LIBS)

# Run basic connectivity test
test: test_ollama_basic
	@echo "🧪 Running Ollama connectivity test..."
	./test_ollama_basic

# Run real embedding tests
test_embeddings: test_real_embeddings
	@echo "🧪 Running real embedding tests..."
	./test_real_embeddings

# Run storage validation tests
test_storage: test_storage_validation
	@echo "🧪 Running storage validation tests..."
	sudo ./test_storage_validation

# Run full /dev/sda capacity test
test_sda: test_sda_full_capacity
	@echo "🧪 Running full /dev/sda capacity test..."
	sudo ./test_sda_full_capacity

# Run all tests
test_all: test test_embeddings test_storage test_sda
	@echo "🎉 All tests completed!"

# Clean build artifacts
clean:
	rm -f $(OBJECTS) $(LIBRARY) $(SHARED_LIBRARY) $(TEST_PROGRAMS)
	@echo "🧹 Cleaned build artifacts"

# Install library system-wide
install: $(LIBRARY) $(SHARED_LIBRARY)
	sudo cp $(LIBRARY) /usr/local/lib/
	sudo cp $(SHARED_LIBRARY) /usr/local/lib/
	sudo cp ollama_client.h /usr/local/include/
	sudo ldconfig
	@echo "✅ Library installed system-wide"

# Uninstall library
uninstall:
	sudo rm -f /usr/local/lib/$(LIBRARY)
	sudo rm -f /usr/local/lib/$(SHARED_LIBRARY)
	sudo rm -f /usr/local/include/ollama_client.h
	sudo ldconfig
	@echo "🗑️ Library uninstalled"

# Performance benchmark
benchmark: test_real_embeddings
	@echo "⚡ Running performance benchmark..."
	./test_real_embeddings --benchmark

# Debug build
debug: CFLAGS += -DDEBUG -g3 -O0
debug: clean all

# Help
help:
	@echo "VexFS v2.0 Ollama Integration Build System"
	@echo "=========================================="
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build library and test programs"
	@echo "  check_deps       - Check if dependencies are installed"
	@echo "  install_deps     - Install system dependencies"
	@echo "  install_ollama   - Install Ollama"
	@echo "  pull_models      - Pull embedding models"
	@echo "  setup            - Complete environment setup"
	@echo "  test             - Run basic connectivity test"
	@echo "  test_embeddings  - Run real embedding tests"
	@echo "  test_storage     - Run storage validation tests"
	@echo "  test_sda         - Run full /dev/sda capacity test"
	@echo "  test_all         - Run all tests"
	@echo "  benchmark        - Run performance benchmark"
	@echo "  install          - Install library system-wide"
	@echo "  uninstall        - Uninstall library"
	@echo "  clean            - Clean build artifacts"
	@echo "  debug            - Build with debug symbols"
	@echo "  help             - Show this help"

.PHONY: all check_deps install_deps install_ollama pull_models setup test test_embeddings test_storage test_sda test_all benchmark clean install uninstall debug help