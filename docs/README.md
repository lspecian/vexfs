# VexFS Documentation

## ⚠️ Documentation Status Warning

This documentation folder contains 300+ files accumulated over the project's development. Many contain:
- Outdated information
- Aspirational features that don't exist
- Conflicting performance claims
- References to deprecated components

## Accurate Documentation

For the current, honest state of the project, see:
- [`../ACTUAL_PROJECT_STATUS.md`](../ACTUAL_PROJECT_STATUS.md) - Real project status
- [`../kernel_module/README.md`](../kernel_module/README.md) - Kernel module details
- [`testing/VEXFS_VM_TEST_REPORT.md`](testing/VEXFS_VM_TEST_REPORT.md) - Latest test results

## Documentation Organization (In Progress)

### `current/` - Verified, accurate docs
- Up-to-date information only
- No aspirational claims

### `archive/` - Historical/outdated docs
- `aspirational/` - Features that were planned but not implemented
- `obsolete/` - Outdated documentation
- `old_implementation/` - Docs for removed code

## Key Technical Documents

### Architecture
- Basic filesystem structure design (if accurate)
- Kernel/userspace separation

### Implementation
- Actual implemented features only
- Known bugs and limitations

### Testing
- VM testing setup and results
- Current test failures

## What to Ignore

Any document claiming:
- 361K+ operations/second performance
- Working Qdrant/ChromaDB API compatibility
- Production-ready status
- Comprehensive test coverage
- Working vector search

These are aspirational goals, not current reality.

---
*Documentation cleanup is ongoing. When in doubt, check ACTUAL_PROJECT_STATUS.md for truth.*