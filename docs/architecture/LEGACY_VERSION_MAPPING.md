# VexFS Legacy Version Mapping

**Status**: ✅ **REFERENCE DOCUMENT**  
**Date**: June 4, 2025  
**Purpose**: Map legacy version references to standardized VexFS v2.0 naming

## Overview

This document provides a comprehensive mapping of all legacy version references found throughout the VexFS codebase to their standardized VexFS v2.0 equivalents. Use this guide when updating documentation, code, or encountering legacy references.

## Version Reference Mapping

### **Project Version References**

| **Legacy Reference** | **Standard Reference** | **Context** | **Action Required** |
|---------------------|------------------------|-------------|-------------------|
| `VexFS v1.0` | `VexFS v2.0` | All documentation | Replace immediately |
| `vexfs v1` | `vexfs v2` | Technical docs | Replace immediately |
| `VexFS Phase 3` | `VexFS v2.0` | Feature descriptions | Replace with v2.0 |
| `vexfs_v2_phase3` | `VexFS v2.0` | User-facing docs | Replace with v2.0 |
| `VexFS v2.0 Phase 3` | `VexFS v2.0` | Mixed references | Simplify to v2.0 |

### **Kernel Module References**

| **Legacy Name** | **Standard Name** | **File Type** | **Status** |
|-----------------|-------------------|---------------|------------|
| `vexfs_v2_phase3.ko` | `vexfs_v2.ko` | Kernel module | Transition planned |
| `vexfs_v2_phase3` | `vexfs_v2` | Module name | Transition planned |
| `vexfs_v2_b62.ko` | `vexfs_v2.ko` | Alternative name | Deprecated |

### **Filesystem Type References**

| **Legacy Type** | **Standard Type** | **Mount Command** | **Status** |
|-----------------|-------------------|-------------------|------------|
| `vexfs_v2_b62` | `vexfs_v2` | `mount -t vexfs_v2` | Transition planned |
| `vexfs_v2_316` | `vexfs_v2` | `mount -t vexfs_v2` | Deprecated |
| `vexfs_phase3` | `vexfs_v2` | `mount -t vexfs_v2` | Deprecated |

### **Build and Test References**

| **Legacy Reference** | **Standard Reference** | **Context** | **Files Affected** |
|---------------------|------------------------|-------------|-------------------|
| `phase3_test` | `vexfs_v2_test` | Test programs | Multiple test files |
| `vexfs_v2_316_test` | `vexfs_v2_test` | Mount points | Test configurations |
| `vexfs_phase3_test` | `vexfs_v2_test` | Test directories | Test scripts |
| `b62` build tags | `v2` build tags | Build systems | Makefiles, configs |

## File-Specific Mappings

### **Documentation Files**

#### **High-Priority Updates** (User-Facing)
```
README.md
├── "VexFS v1.0" → "VexFS v2.0"
├── "vexfs_v2_phase3" → "VexFS v2.0"
└── "vexfs_v2_b62" → "VexFS v2.0"

docs/README.md
├── "VexFS v2.0 Phase 3" → "VexFS v2.0"
├── "vexfs_v2_phase3" → "VexFS v2.0"
└── Multiple phase3 references → "v2.0"
```

#### **Technical Documentation**
```
docs/implementation/*.md
├── "Phase 3" → "v2.0"
├── "vexfs_v2_phase3" → "vexfs_v2"
└── "b62" → "v2.0"

docs/architecture/*.md
├── "VexFS v2.0 Phase 3" → "VexFS v2.0"
├── "phase3" features → "v2.0" features
└── Legacy version refs → "v2.0"
```

### **Code Files**

#### **Test Files** (Immediate Impact)
```
kernel/vexfs_v2_build/test_*.c
├── "/tmp/vexfs_v2_316_test" → "/tmp/vexfs_v2_test"
├── "/tmp/vexfs_phase3_test" → "/tmp/vexfs_v2_test"
├── "phase3_test" → "vexfs_v2_test"
└── Comments: "Phase 3" → "v2.0"

tests/functional/test_phase3_basic.c
├── "/tmp/vexfs_phase3_test" → "/tmp/vexfs_v2_test"
├── "vexfs_v2_phase3" → "vexfs_v2"
└── "Phase 3" → "v2.0"
```

#### **Build Files**
```
kernel/vexfs_v2_build/Makefile
├── "vexfs_v2_phase3.o" → "vexfs_v2.o"
├── "vexfs_v2_phase3.ko" → "vexfs_v2.ko"
├── "/tmp/vexfs_v2_test" → "/tmp/vexfs_v2_test" ✅
└── Comments: "Phase 3" → "v2.0"
```

#### **Source Files** (Kernel Module)
```
kernel/vexfs_v2_build/vexfs_v2_main.c
├── .name = "vexfs_v2_b62" → "vexfs_v2"
├── Comments: "Phase 3" → "v2.0"
└── Module descriptions → "VexFS v2.0"
```

## Mount Point Standardization

### **Legacy Mount Points** → **Standard Mount Points**

| **Legacy Mount Point** | **Standard Mount Point** | **Usage** |
|------------------------|---------------------------|-----------|
| `/tmp/vexfs_v2_316_test` | `/tmp/vexfs_v2_test` | Development testing |
| `/tmp/vexfs_phase3_test` | `/tmp/vexfs_v2_test` | Phase 3 testing |
| `/tmp/vexfs_v2_monitored` | `/tmp/vexfs_v2_monitored` | ✅ Already standard |
| `/tmp/vexfs_v2_optimized` | `/tmp/vexfs_v2_optimized` | ✅ Already standard |

### **Mount Command Updates**

```bash
# Legacy commands (still work but deprecated)
sudo mount -t vexfs_v2_b62 /dev/sda1 /mnt/vexfs
sudo mount -t vexfs_v2_316 none /tmp/test

# Standard commands (preferred)
sudo mount -t vexfs_v2 /dev/sda1 /mnt/vexfs
sudo mount -t vexfs_v2 none /tmp/vexfs_v2_test
```

## Symbol and Function Mapping

### **Exported Symbols**
```c
// Legacy symbols (maintain for compatibility)
EXPORT_SYMBOL(vexfs_v2_phase3_ioctl_handler);
EXPORT_SYMBOL(phase3_stats);
EXPORT_SYMBOL(vexfs_phase3_init);

// Standard symbols (preferred)
EXPORT_SYMBOL(vexfs_v2_ioctl_handler);
EXPORT_SYMBOL(vexfs_v2_stats);
EXPORT_SYMBOL(vexfs_v2_init);
```

### **Function Names**
```c
// Legacy function names → Standard function names
vexfs_phase3_init() → vexfs_v2_init()
vexfs_phase3_cleanup() → vexfs_v2_cleanup()
vexfs_phase3_ioctl() → vexfs_v2_ioctl()
phase3_stats → vexfs_v2_stats
```

## Documentation Update Patterns

### **Search and Replace Patterns**

#### **Global Documentation Updates**
```bash
# Replace version references
find docs/ -name "*.md" -exec sed -i 's/VexFS v1\.0/VexFS v2.0/g' {} \;
find docs/ -name "*.md" -exec sed -i 's/vexfs v1/vexfs v2/g' {} \;

# Replace phase references
find docs/ -name "*.md" -exec sed -i 's/VexFS v2\.0 Phase 3/VexFS v2.0/g' {} \;
find docs/ -name "*.md" -exec sed -i 's/Phase 3/v2.0/g' {} \;

# Replace build references
find docs/ -name "*.md" -exec sed -i 's/vexfs_v2_b62/vexfs_v2/g' {} \;
find docs/ -name "*.md" -exec sed -i 's/b62/v2.0/g' {} \;
```

#### **Code Comment Updates**
```bash
# Update code comments
find kernel/ -name "*.c" -exec sed -i 's/Phase 3/v2.0/g' {} \;
find kernel/ -name "*.h" -exec sed -i 's/Phase 3/v2.0/g' {} \;

# Update test mount points
find kernel/ -name "*.c" -exec sed -i 's|/tmp/vexfs_v2_316_test|/tmp/vexfs_v2_test|g' {} \;
find kernel/ -name "*.c" -exec sed -i 's|/tmp/vexfs_phase3_test|/tmp/vexfs_v2_test|g' {} \;
```

## Transition Timeline

### **Phase 1: Documentation Standardization** ✅ **IN PROGRESS**
- **Target**: All documentation uses "VexFS v2.0"
- **Files**: All `.md` files in `docs/`
- **Status**: Active implementation

### **Phase 2: Test Standardization** (Next)
- **Target**: All test files use standard mount points and references
- **Files**: All test programs and scripts
- **Timeline**: After documentation completion

### **Phase 3: Code Standardization** (Future)
- **Target**: Kernel module and filesystem type names
- **Files**: Core kernel module files
- **Timeline**: Next major release

### **Phase 4: Legacy Deprecation** (Future)
- **Target**: Remove legacy compatibility
- **Files**: All legacy references
- **Timeline**: 6 months after Phase 3

## Validation Commands

### **Check for Legacy References**
```bash
# Documentation validation
grep -r "VexFS v1\|vexfs v1\|Phase 3\|b62" docs/ \
  --exclude="*VERSION_STANDARDIZATION.md" \
  --exclude="*LEGACY_VERSION_MAPPING.md"

# Code validation
grep -r "vexfs_v2_316\|phase3_test\|vexfs_phase3_test" kernel/

# Mount point validation
grep -r "/tmp/vexfs_v2_316_test\|/tmp/vexfs_phase3_test" .
```

### **Verify Standard Usage**
```bash
# Count standard references
grep -r "VexFS v2\.0" docs/ | wc -l
grep -r "vexfs_v2_test" kernel/ | wc -l

# Check filesystem type
grep -r "vexfs_v2_b62" kernel/vexfs_v2_build/vexfs_v2_main.c
```

## Common Migration Mistakes

### **❌ Avoid These Patterns**
```markdown
❌ "VexFS v2.0 Phase 3 provides..."
✅ "VexFS v2.0 provides..."

❌ "The vexfs_v2_phase3 module..."
✅ "The VexFS v2.0 module..."

❌ "Mount using vexfs_v2_b62..."
✅ "Mount using vexfs_v2..."

❌ "Phase 3 features include..."
✅ "VexFS v2.0 features include..."
```

### **✅ Correct Patterns**
```markdown
✅ "VexFS v2.0 is a kernel-native vector database filesystem"
✅ "The VexFS v2.0 kernel module provides..."
✅ "Mount VexFS v2.0 using: mount -t vexfs_v2..."
✅ "VexFS v2.0 features include advanced indexing..."
```

## References

- [Version Standardization Guide](VERSION_STANDARDIZATION.md)
- [API Hierarchy Documentation](API_HIERARCHY.md)
- [VexFS v2.0 Architecture](VEXFS_V2_FLOATING_POINT_ARCHITECTURE.md)

---

**This document provides the definitive mapping for all legacy version references. Use it as a reference when updating any VexFS component.**

**Last Updated**: June 4, 2025  
**Next Review**: After each major update  
**Maintained By**: VexFS Development Team