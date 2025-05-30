# VexFS Documentation

This directory contains all documentation for the VexFS project, organized by category for easy navigation.

## Directory Structure

### 📁 [architecture/](architecture/)
System architecture, design patterns, and strategic documents:
- `C_FFI_ARCHITECTURE.md` - C FFI interface design and implementation
- `FUTURE_BENCHMARKING_STRATEGY.md` - Performance testing strategy
- `HYBRID_DEVELOPMENT_STRATEGY.md` - Development approach combining kernel and userspace
- `KERNEL_DEVELOPMENT_STRATEGY.md` - Kernel module development guidelines

### 📁 [fs/](fs/)
Filesystem-specific documentation and implementation details:
- `ANNS_IMPLEMENTATION.md` - Approximate Nearest Neighbor Search implementation
- `BUILD_SYSTEM.md` - Build system configuration and usage
- `DDD_DOMAIN_ARCHITECTURE.md` - Domain-Driven Design architecture overview
- `DDD_ENTITY_EXTRACTION_PLAN.md` - Plan for extracting domain entities
- `DDD_IMPLEMENTATION_GUIDE.md` - Implementation guide for DDD patterns
- `DDD_REFACTORING_PLAN.md` - **ACTIVE** - Detailed plan for refactoring large files into DDD structure
- `DDD_REFACTORING_SUMMARY.md` - Summary of DDD refactoring progress
- `FILESYSTEM_ARCHITECTURE_ANALYSIS.md` - Analysis of filesystem architecture
- `README.md` - Filesystem module overview
- `VECTOR_SEARCH_IMPLEMENTATION.md` - Vector search implementation details
- `VECTOR_STORAGE.md` - Vector storage system documentation

### 📁 [implementation/](implementation/)
Implementation plans and development progress:
- `IMPLEMENTATION_PLAN.md` - Overall implementation roadmap

### 📁 [status/](status/)
Project status reports and current state assessments:
- `CURRENT_PROJECT_STATUS.md` - Current project state overview (as of Jan 2025)

### 📁 [testing/](testing/)
Testing strategies, VM setup, and testing documentation:
- `README.md` - **UPDATED** - Comprehensive testing infrastructure overview with new consolidated structure
- `QEMU_SETUP_GUIDE.md` - QEMU virtual machine setup guide
- `SIMPLIFIED_VM_SETUP.md` - Simplified VM setup instructions
- `VM_TESTING_STRATEGY.md` - Virtual machine testing strategy
- `COMPREHENSIVE_TESTING_FRAMEWORK.md` - Framework overview
- `INFRASTRUCTURE_AS_CODE_MIGRATION.md` - IaC migration documentation

## Root Level Documentation

- `ACTION_PLAN.md` - High-level action plan for the project
- `DEVELOPMENT_WORKFLOW.md` - Development workflow and processes
- `LICENSING.md` - Licensing information and compliance

## Project Root Documentation

- `../README.md` - Main project README
- `../LICENSE` - Project license
- `../LICENSE.kernel` - Kernel-specific license
- `../NOTICE` - Legal notices

## Current Project State (May 2025)

### ✅ Recently Completed
- **Documentation Organization**: All .md files moved to structured docs/ directories
- **Source Code Flattening**: Moved all source code from `fs/src/` to root `src/` for cleaner structure
- **Build System**: Makefiles and build configuration moved to project root
- **Testing Infrastructure Consolidation**: Complete migration from scattered `test_env/` to unified `tests/` structure
  - **Domain-Driven Design**: Tests organized by business domains with intelligent tagging system
  - **Infrastructure-as-Code**: Terraform + Ansible moved from `infrastructure/` to `tests/infrastructure/`
  - **Legacy Scripts Preserved**: 35+ shell scripts moved from `test_env/` to `tests/legacy/shell_scripts/`
  - **Unified Interface**: Single `make` command system for all testing scenarios

### ⚠️ Current Issues
- **Compilation Errors**: 506 compilation errors need resolution after structure flattening
- **File Size Issues**: Several files exceed 1000+ lines and need DDD refactoring:
  - `src/dir_ops.rs` (1,484 lines)
  - `src/file_ops.rs` (1,388 lines) 
  - `src/ondisk.rs` (1,120 lines)
  - `src/space_alloc.rs` (879 lines)

### 📋 Next Priority Actions
1. **Fix Compilation Errors**: Address the 506 errors introduced during structure flattening
2. **Implement DDD Refactoring**: Follow the plan in `fs/DDD_REFACTORING_PLAN.md` to break down large files
3. **Update Build System**: Ensure Makefiles work with new flattened structure
4. **Test Integration**: Verify kernel module compilation and testing

## Navigation Tips

- **For Architecture Decisions**: See `architecture/` directory
- **For Current Development Status**: Check `status/CURRENT_PROJECT_STATUS.md`
- **For Code Organization**: Review `fs/DDD_REFACTORING_PLAN.md`
- **For Build Issues**: Consult `fs/BUILD_SYSTEM.md`
- **For Testing Setup**: Use guides in `testing/` directory
- **For Modern Testing**: See `../tests/README.md` for the new consolidated testing infrastructure
- **For Legacy Testing**: Check `../tests/legacy/QUICK_START.md` for traditional VM-based testing
- **For Infrastructure-as-Code**: Review `../tests/infrastructure/README.md` for automated provisioning

## Document Maintenance

This documentation index is kept up to date with the current project state. Outdated documents have been removed as of May 2025 cleanup. All remaining documents are relevant to the current flattened project structure.